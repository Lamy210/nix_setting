#!/usr/bin/env bash
# release workflow と PR CI で同一の macOS DMG artifact build を実行する。
# この script を修正した場合は .github/workflows/{check,release}.yml 両方の
# 呼び出し側も同一のまま保つこと。
#
# host toolchain (cargo) + sha256 pin 済み Tauri CLI で build する。
# v0.2.0-rc.4 まで DMG は `nix develop --command cargo tauri build` で、nix 産
# desktop binary が /nix/store の libiconv に link したまま release された
# (CLI と同じ欠陥。Final Acceptance の packaging preflight で静的検出)。
# devShell の cargo-tauri は nixpkgs 供給で version が channel と共に動くため、
# release 再現性のために exact version を sha256 pin した prebuilt を使う。
#
# gate は build 済み DMG を mount して .app 内の完成品 binary に適用する
# (raw binary や .app ではなく DMG が最終配布物であるため)。
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Tauri CLI の exact pin。update 時は SHA256 を GitHub release の digest と
# 照合して更新すること (gh api repos/tauri-apps/tauri/releases/tags/tauri-cli-v${VERSION})
TAURI_CLI_VERSION="2.11.4"
TAURI_CLI_SHA256="82bdcb9ae7f407882321680ae50750f11623fae22445f8b00b096e10f815d604"
TAURI_CLI_URL="https://github.com/tauri-apps/tauri/releases/download/tauri-cli-v${TAURI_CLI_VERSION}/cargo-tauri-aarch64-apple-darwin.zip"

# GUI updater は production activation が明示された tag release でのみ有効にする。
# PR / develop の通常 artifact gate は signing secret を要求せず updater-disabled。
SCHNEEFORGE_UPDATER_ACTIVATED="${SCHNEEFORGE_UPDATER_ACTIVATED:-false}"
case "$SCHNEEFORGE_UPDATER_ACTIVATED" in
true | false) ;;
*)
  echo "ERROR: SCHNEEFORGE_UPDATER_ACTIVATED must be true or false" >&2
  exit 1
  ;;
esac

WORKDIR="$(mktemp -d)"
MOUNT_POINT=""
cleanup() {
  if [ -n "$MOUNT_POINT" ] && mount | grep -q "$MOUNT_POINT"; then
    hdiutil detach "$MOUNT_POINT" -force >/dev/null 2>&1 || true
  fi
  rm -rf "$WORKDIR"
}
trap cleanup EXIT

# --- Tauri CLI を pin して install ---
curl -fsSL -o "$WORKDIR/cargo-tauri.zip" "$TAURI_CLI_URL"
echo "${TAURI_CLI_SHA256}  $WORKDIR/cargo-tauri.zip" | shasum -a 256 --check --status

unzip -o "$WORKDIR/cargo-tauri.zip" -d "$WORKDIR/tauri-cli" >/dev/null
TAURI_BIN="$WORKDIR/tauri-cli/cargo-tauri"
chmod +x "$TAURI_BIN"

# --- build (host cargo + pinned CLI) ---
# CLI sidecar (externalBin) の source を先に build する。desktop は root
# workspace と分離しているため root 側で build し、build script は
# <repo>/target/release/schneeforge を stage 元として解決する
cd "$REPO_ROOT"
cargo build --release -p schneeforge

cd "$REPO_ROOT/apps/desktop/src-tauri"
if [ "$SCHNEEFORGE_UPDATER_ACTIVATED" = "true" ]; then
  : "${SCHNEEFORGE_UPDATER_PUBKEY:?SCHNEEFORGE_UPDATER_PUBKEY is required when updater is activated}"
  : "${TAURI_SIGNING_PRIVATE_KEY:?TAURI_SIGNING_PRIVATE_KEY is required when updater is activated}"

  export SCHNEEFORGE_UPDATER_ENABLED=1
  export SCHNEEFORGE_UPDATER_PUBKEY
  # Tauri CLI consumes TAURI_SIGNING_PRIVATE_KEY and optional
  # TAURI_SIGNING_PRIVATE_KEY_PASSWORD from the environment.
  cat >"$WORKDIR/updater-config.json" <<'JSON'
{
  "bundle": {
    "createUpdaterArtifacts": true
  }
}
JSON
  "$TAURI_BIN" build --config "$WORKDIR/updater-config.json"

  UPDATER_ARTIFACT="$(find target/release/bundle/macos -type f -name '*.app.tar.gz' | head -1)"
  if [ -z "$UPDATER_ARTIFACT" ] || [ ! -s "$UPDATER_ARTIFACT" ]; then
    echo "ERROR: updater activation produced no .app.tar.gz artifact" >&2
    exit 1
  fi
  if [ ! -s "${UPDATER_ARTIFACT}.sig" ]; then
    echo "ERROR: updater activation produced no signature for $UPDATER_ARTIFACT" >&2
    exit 1
  fi
  echo "OK: signed updater artifact produced: $UPDATER_ARTIFACT"
else
  # Fail closed: ambient developer/runner variables must not accidentally
  # activate an updater trust root in ordinary PR/develop builds.
  unset SCHNEEFORGE_UPDATER_ENABLED
  unset SCHNEEFORGE_UPDATER_PUBKEY
  unset TAURI_SIGNING_PRIVATE_KEY
  unset TAURI_SIGNING_PRIVATE_KEY_PASSWORD
  "$TAURI_BIN" build
fi

# --- gate: build 済み DMG を mount して完成品を検査 ---
DMG="$(find target/release/bundle/dmg -name '*.dmg' | head -1)"
if [ -z "$DMG" ]; then
  echo "ERROR: no DMG produced" >&2
  exit 1
fi

hdiutil verify "$DMG"

MOUNT_POINT="$WORKDIR/mount"
hdiutil attach "$DMG" -mountpoint "$MOUNT_POINT" -nobrowse -readonly

APP="$MOUNT_POINT/SchneeForge.app"
DESKTOP_BIN="$APP/Contents/MacOS/schneeforge-desktop"

"$SCRIPT_DIR/check-macos-portability.sh" "$DESKTOP_BIN"

# GUI の escalation 先は bundle 内 CLI sidecar でなければならない
# (GUI binary は CLI 引数を解釈しない。externalBin が外れると install が
#  実行されないまま昇格だけ走る)。tauri 2.x は triple suffix を除去して
#  `schneeforge-cli` の名で配置する
SIDECAR_BIN="$APP/Contents/MacOS/schneeforge-cli"
if [ ! -f "$SIDECAR_BIN" ]; then
  echo "ERROR: CLI sidecar missing from .app bundle: $SIDECAR_BIN" >&2
  ls "$APP/Contents/MacOS/" >&2
  exit 1
fi
"$SCRIPT_DIR/check-macos-portability.sh" "$SIDECAR_BIN"

# DMG 内 app の version が tauri.conf.json と一致すること
EXPECTED_VERSION="$(plutil -extract version raw "$REPO_ROOT/apps/desktop/src-tauri/tauri.conf.json")"
ACTUAL_VERSION="$(plutil -extract CFBundleShortVersionString raw "$APP/Contents/Info.plist")"
if [ "$EXPECTED_VERSION" != "$ACTUAL_VERSION" ]; then
  echo "ERROR: DMG app version mismatch: expected $EXPECTED_VERSION, got $ACTUAL_VERSION" >&2
  exit 1
fi

hdiutil detach "$MOUNT_POINT" >/dev/null
MOUNT_POINT=""

echo "OK: macOS DMG built and verified (version $ACTUAL_VERSION)"
