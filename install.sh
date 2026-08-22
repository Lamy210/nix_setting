#!/usr/bin/env bash
#
# SchneeForge installer. `curl|bash` で実行されるため、リポジトリ内の
# scripts/resolve-tools.sh を source するのではなく、必要最小限の resolver を
# inline で持つ。既存 checkout がある場合のみ bootstrap.sh を使い、fresh
# install は clone せず managed source (flake ref) で導入する。
set -euo pipefail

REPO_DIR="${NIX_SETTING_DIR:-$HOME/nix_setting}"
# bootstrap が download する schneeforge CLI の version。
# latest release 任せにすると rc (壊れた asset を含み得る) が拾われるため
# release 毎に固定する。release 時は RELEASE.md の手順でこの値を bump する。
SCHNEEFORGE_BOOTSTRAP_VERSION="${SCHNEEFORGE_VERSION:-v0.2.0-rc.6}"
# config repository (modules / bootstrap-manifest.toml) の ref。CLI binary と
# 同一 release tag に固定することで bootstrap の対象が release unit として一致する
# (default branch を拾うと過去の installer 実行時に「その時点の develop」が入る)。
# release 時は VERSION と同時に bump する。
SCHNEEFORGE_BOOTSTRAP_REF="${SCHNEEFORGE_REF:-$SCHNEEFORGE_BOOTSTRAP_VERSION}"

# --- inline minimal tool resolver (clone 前に動くよう install.sh 単独で解決可能) ---
# 探索順は scripts/resolve-tools.sh と一致 (Rust tool.rs とも同期)
is_executable() {
  [ -f "$1" ] && [ -x "$1" ]
}

# resolve_tool NAME -- <NAME>_BIN 絶対パスを export。見つからなければ 1
resolve_tool() {
  local name="$1"
  local upper
  upper="$(printf '%s' "$name" | tr '[:lower:]-' '[:upper:]_')"
  local env_var="SCHNEEFORGE_${upper}_BIN"
  local out_var="${upper}_BIN"

  # 1. env override
  if [ -n "${!env_var:-}" ] && is_executable "${!env_var}"; then
    export "${out_var}=${!env_var}"
    return 0
  fi

  # 2. PATH
  if command -v "$name" >/dev/null 2>&1; then
    local p
    p="$(command -v "$name")"
    if is_executable "$p"; then
      export "${out_var}=$p"
      return 0
    fi
  fi

  # 候補ディレクトリ (3-10)
  local candidates=()
  if [ -n "${XDG_STATE_HOME:-}" ]; then
    candidates+=("${XDG_STATE_HOME}/nix/profile/bin")
  fi
  if [ -n "${HOME:-}" ]; then
    candidates+=("${HOME}/.local/state/nix/profile/bin")
  fi
  if [ -n "${NIX_PROFILE:-}" ]; then
    candidates+=("${NIX_PROFILE}/bin")
  fi
  if [ -n "${HOME:-}" ]; then
    candidates+=("${HOME}/.nix-profile/bin")
  fi
  if [ -n "${USER:-}" ]; then
    candidates+=("/etc/profiles/per-user/${USER}/bin")
  fi
  candidates+=("/nix/var/nix/profiles/default/bin")
  candidates+=("/opt/homebrew/bin")
  candidates+=("/usr/local/bin")

  for dir in "${candidates[@]}"; do
    local candidate="${dir}/${name}"
    if is_executable "$candidate"; then
      export "${out_var}=$candidate"
      return 0
    fi
  done

  return 1
}

resolve_nix() { resolve_tool "nix"; }
resolve_git() { resolve_tool "git"; }

# file の SHA256 hex を出力する。Linux (sha256sum) / macOS (shasum) 両対応。
# curl|bash を想定しており coreutils の導入は前提にできない。
sha256_file() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | awk '{print $1}'
  else
    echo "[error] sha256sum / shasum のいずれも見つかりません" >&2
    return 1
  fi
}

# Nix installer が作らないことがある state dir を保証
ensure_nix_state_dir() {
  local state_dir
  if [ -n "${XDG_STATE_HOME:-}" ]; then
    state_dir="${XDG_STATE_HOME}/nix/profiles"
  else
    state_dir="${HOME:?HOME must be set}/.local/state/nix/profiles"
  fi
  [ -d "$state_dir" ] || mkdir -p "$state_dir"
}

# Managed Nix 経路で使う schneeforge CLI binary を GitHub Release から取得し、
# CHECKSUMS.txt の sha256 と突合してから path を標準出力へ出す。
# (version pinning / 検証の詳細は schneeforge nix install 側が持つため、ここでは
#  download 元の integrity のみ確認する)
fetch_schneeforge_binary() {
  local os arch asset_base url_base version tmp_dir expect actual
  case "$(uname -s)" in
  Linux) os="linux" ;;
  Darwin) os="darwin" ;;
  *)
    echo "[error] unsupported OS: $(uname -s)" >&2
    return 1
    ;;
  esac
  case "$(uname -m)" in
  x86_64 | amd64) arch="x86_64" ;;
  aarch64 | arm64) arch="aarch64" ;;
  *)
    echo "[error] unsupported arch: $(uname -m)" >&2
    return 1
    ;;
  esac
  if [ "$os" = "darwin" ]; then
    # darwin release binary は aarch64 のみ。Intel Mac では Rosetta があっても
    # native 動作を保証できないため、download 前に明示 reject する
    if [ "$arch" != "aarch64" ]; then
      echo "[error] macOS x86_64 (Intel Mac) は未提供です。aarch64 (Apple Silicon) のみ対応しています。" >&2
      return 1
    fi
    asset_base="schneeforge-aarch64-darwin"
  else
    # Linux aarch64 の release binary は未提供 (release workflow が x86_64-musl のみ
    # build する)。asset 無しで CHECKSUMS 検証に失敗するより手前で案内する
    if [ "$arch" = "aarch64" ]; then
      echo "[error] Linux aarch64 の one-line bootstrap は未提供です。" >&2
      echo "       Nix を手動導入のうえ bootstrap.sh をお使いください (Nix/Home Manager 設定自体は aarch64 Linux 対応)。" >&2
      return 1
    fi
    asset_base="schneeforge-${arch}-${os}"
  fi

  version="${SCHNEEFORGE_BOOTSTRAP_VERSION:-}"
  url_base="https://github.com/Lamy210/nix_setting/releases/download/${version}"

  tmp_dir="$(mktemp -d "${TMPDIR:-/tmp}/schneeforge-install.XXXXXX")"
  curl -fsSL -o "$tmp_dir/CHECKSUMS.txt" "$url_base/CHECKSUMS.txt" || {
    echo "[error] CHECKSUMS.txt の download に失敗: $version" >&2
    return 1
  }
  # asset 名 (path) は release workflow の配置に依存するため前方一致で取り出す
  expect="$(sed -n "s|^[0-9a-f]\{64\}  .*/${asset_base}\$|&|p" "$tmp_dir/CHECKSUMS.txt" | awk '{print $1}')"
  if [ -z "$expect" ]; then
    echo "[error] CHECKSUMS.txt に ${asset_base} の entry が無い: $version" >&2
    return 1
  fi

  curl -fsSL -o "$tmp_dir/$asset_base" "$url_base/$asset_base" || {
    echo "[error] ${asset_base} の download に失敗: $version" >&2
    return 1
  }
  actual="$(sha256_file "$tmp_dir/$asset_base")"
  if [ "$actual" != "$expect" ]; then
    echo "[error] sha256 mismatch (${asset_base}):" >&2
    echo "  expect: $expect" >&2
    echo "  actual: $actual" >&2
    return 1
  fi
  chmod +x "$tmp_dir/$asset_base"
  echo "$tmp_dir/$asset_base"
}

# root 権限で file の SHA256 hex を出力する (sha256_file の root 版)。
# shell 関数は sudo を通せないため、root の PATH 上で同様に解決する。
sudo_sha256_file() {
  if sudo sh -c 'command -v sha256sum >/dev/null 2>&1'; then
    sudo sha256sum "$1" | awk '{print $1}'
  elif sudo sh -c 'command -v shasum >/dev/null 2>&1'; then
    sudo sh -c "shasum -a 256 '$1'" | awk '{print $1}'
  else
    echo "[error] root 権限で sha256sum / shasum が見つかりません" >&2
    return 1
  fi
}

# Managed Nix (schneeforge nix install) を実行する。
# root 権限が必要なため sudo で再実行する。CLI の D8 最終確認は stdin が
# TTY でないと fail-closed されるため、curl|bash で stdin を奪われている
# 場合は /dev/tty を繋いで確認可能にする。
#
# sf_bin は caller が CHECKSUMS 検証済みの binary path を渡す (fresh 経路は
# source init / apply でも使うため、user 側 binary はここで削除せず EXIT trap の
# cleanup_sf が lifecycle を管理する)。binary は user 権限で download +
# SHA256 検証した後、root-owned の staging dir (Core の privileged_state_dir
# と同じ配置) へ copy し、root 側で hash を再検証してから実行する。sudo
# password 入力待ちの間に user-writable な binary を差し替えられる TOCTOU を
# 潰すため。
#
# repo_dir を渡した場合は repo の bootstrap-manifest.toml を優先させる
# (既存 checkout 経路の現行挙動)。渡さなければ CLI の embedded manifest で
# 動作する (fresh 経路。repo checkout が無くてよい)。
install_managed_nix() {
  local sf_bin="$1"
  local repo_dir="${2:-}"
  local sf_hash stage_dir sf_staged staged_hash
  sf_hash="$(sha256_file "$sf_bin")" || return 1

  # root-owned staging dir (Core の privileged_state_dir と同一配置。
  # macOS は /var → /private/var symlink 問題があるため実 path を使う)
  case "$(uname -s)" in
  Darwin) stage_dir="/private/var/db/schneeforge/bootstrap" ;;
  *) stage_dir="/var/lib/schneeforge/bootstrap" ;;
  esac
  sf_staged="${stage_dir}/schneeforge"

  echo "[nix] Managed Nix install を開始します (NixOS/nix-installer, version pinned)..."
  # fresh machine には staging dir が存在しないため root 権限で作成する
  # (0700: 他 user に binary 置き場を見せない)
  if ! sudo install -d -m 0700 "$stage_dir"; then
    echo "[error] staging dir (${stage_dir}) の作成に失敗" >&2
    return 1
  fi
  if ! sudo install -m 0755 "$sf_bin" "$sf_staged"; then
    echo "[error] staging dir (${stage_dir}) への copy に失敗" >&2
    sudo rm -f "$sf_staged"
    sudo rmdir "$stage_dir" 2>/dev/null || true
    return 1
  fi

  # user 権限で検証した hash と一致するか root 側で再検証する
  # (sudo password 待ち・copy 中の差し替えを検出)
  staged_hash="$(sudo_sha256_file "$sf_staged")" || {
    sudo rm -f "$sf_staged"
    sudo rmdir "$stage_dir" 2>/dev/null || true
    return 1
  }
  if [ "$staged_hash" != "$sf_hash" ]; then
    echo "[error] staged binary の sha256 が検証値と一致しません (TOCTOU 疑い):" >&2
    echo "  expect: $sf_hash" >&2
    echo "  actual: $staged_hash" >&2
    sudo rm -f "$sf_staged"
    sudo rmdir "$stage_dir" 2>/dev/null || true
    return 1
  fi

  # sudo 環境でも repo 位置が分かるように NIX_SETTING_DIR を渡す (既存 checkout
  # 経路のみ。fresh 経路は embedded manifest で動作するため渡さない)
  if [ -n "$repo_dir" ]; then
    if [ -t 0 ]; then
      sudo env NIX_SETTING_DIR="$repo_dir" "$sf_staged" nix install
    else
      # shellcheck disable=SC2024  # redirect 自体が目的 (curl|bash の stdin を外す)
      sudo env NIX_SETTING_DIR="$repo_dir" "$sf_staged" nix install </dev/tty
    fi
  else
    if [ -t 0 ]; then
      sudo "$sf_staged" nix install
    else
      # shellcheck disable=SC2024  # redirect 自体が目的 (curl|bash の stdin を外す)
      sudo "$sf_staged" nix install </dev/tty
    fi
  fi
  local rc=$?
  sudo rm -f "$sf_staged"
  sudo rmdir "$stage_dir" 2>/dev/null || true
  if [ $rc -ne 0 ]; then
    echo "[error] Managed Nix install に失敗 (exit $rc)" >&2
    return 1
  fi
  # install 直後は login shell の PATH に /nix/... が無いことがある
  if [ -e /nix/var/nix/profiles/default/etc/profile.d/nix-daemon.sh ]; then
    # shellcheck disable=SC1091
    . /nix/var/nix/profiles/default/etc/profile.d/nix-daemon.sh 2>/dev/null || true
  fi
  if ! resolve_nix; then
    echo "[error] Nix install は完了したが解決できない。shell を reload して再実行してください。" >&2
    return 1
  fi
}

# home-manager 導入で既存 dotfile が衝突する初回適用の保険として、管理対象の
# 既存 dotfile を timestamp 付き backup dir へ退避する (bootstrap.sh から移植)。
backup_dotfiles() {
  local backup_dir f
  echo "Backing up existing dotfiles..."
  backup_dir="$HOME/hm-bak-$(date +%Y%m%d-%H%M%S)"
  mkdir -p "$backup_dir"
  for f in .zshrc .zprofile .gitconfig .config/starship.toml .config/wezterm/wezterm.lua .config/atuin/config.toml .config/openspec/config.json .config/mise/config.toml; do
    [ -f "$HOME/$f" ] && cp "$HOME/$f" "$backup_dir/$(echo "$f" | tr '/' '_')"
  done
  echo "Backed up to $backup_dir"
}

# CHECKSUMS 検証済み sf binary は fresh 経路で apply 完了まで使うため、
# tmp dir の削除は各使用箇所ではなく EXIT trap で一括して行う
SF_TMP_DIR=""
cleanup_sf() {
  if [ -n "$SF_TMP_DIR" ] && [ -d "$SF_TMP_DIR" ]; then
    rm -rf "$SF_TMP_DIR"
  fi
}
trap cleanup_sf EXIT
# --- end inline resolver ---

echo "=== nix_setting installer ==="
echo

# 1. Git (managed source の tag 解決にも必要なため、経路によらず必須)
if ! resolve_git; then
  echo "[1/4] Git not found. Install Git first via your OS package manager." >&2
  exit 1
fi

SF_BIN=""
if [ -d "$REPO_DIR/.git" ]; then
  # 既存 checkout は user の変更を壊さないようそのまま使う。release tag と
  # 一致する保証は無い (develop 追随中の場合など) ので明示しておく
  FLOW="checkout"
  echo "[1/4] Repository exists: $REPO_DIR"
  echo "[warning] Existing repository detected."
  echo "         Bootstrap will use the current checkout instead of release ${SCHNEEFORGE_BOOTSTRAP_REF}."
else
  # fresh install は clone しない。CLI binary を release から取得し、以降を
  # managed source (flake ref) で進める
  FLOW="managed"
  echo "[1/4] Fetching schneeforge CLI (release: ${SCHNEEFORGE_BOOTSTRAP_VERSION}) ..."
  SF_BIN="$(fetch_schneeforge_binary)" || exit 1
  SF_TMP_DIR="$(dirname "$SF_BIN")"
fi

# 2. Check Nix (resolve_nix で PATH + 既知パス両方を探索)。
#    未検出の場合は curl|sh ではなく SchneeForge Managed Nix で install する
#    (ownership record 付き。ADR-0001 参照)
if resolve_nix; then
  echo "[2/4] Nix found: $NIX_BIN"
else
  echo "[2/4] Nix not found. Installing via SchneeForge Managed Nix..."
  if [ -z "$SF_BIN" ]; then
    SF_BIN="$(fetch_schneeforge_binary)" || exit 1
    SF_TMP_DIR="$(dirname "$SF_BIN")"
  fi
  if [ "$FLOW" = "checkout" ]; then
    install_managed_nix "$SF_BIN" "$REPO_DIR" || exit 1
  else
    install_managed_nix "$SF_BIN" || exit 1
  fi
  echo "[2/4] Nix: $NIX_BIN"
fi

# Nix installer が作らないことがあるフォルダを保証
ensure_nix_state_dir

# 3. Enable flakes
mkdir -p "$HOME/.config/nix"
if ! grep -q "experimental-features" "$HOME/.config/nix/nix.conf" 2>/dev/null; then
  echo "[3/4] Enabling flakes..."
  cat >>"$HOME/.config/nix/nix.conf" <<'NIXCONF'
experimental-features = nix-command flakes
NIXCONF
else
  echo "[3/4] Flakes: enabled"
fi

# 4. Apply
#    checkout 経路: bootstrap.sh (detect host + build + apply)。
#      この時点からリポジトリ内の scripts/resolve-tools.sh が利用可能になる
#    managed 経路: dotfile backup → source init (tag pinned) → apply (flake ref)
if [ "$FLOW" = "checkout" ]; then
  echo "[4/4] Applying configuration..."
  (
    cd "$REPO_DIR"
    ./bootstrap.sh
  )
else
  echo "[4/4] Applying configuration (managed source: ${SCHNEEFORGE_BOOTSTRAP_REF})..."
  echo
  backup_dotfiles
  echo
  echo "Initializing managed source..."
  "$SF_BIN" source init --tag "$SCHNEEFORGE_BOOTSTRAP_REF"
  echo
  echo "Applying configuration..."
  if [ -t 0 ]; then
    "$SF_BIN" apply
  else
    # curl|bash で stdin を奪われている場合に備え /dev/tty を繋ぐ
    # (macOS は darwin-rebuild が内部で sudo password を要求する)
    "$SF_BIN" apply </dev/tty
  fi
fi

echo
echo "Done. Reload your shell or terminal."
