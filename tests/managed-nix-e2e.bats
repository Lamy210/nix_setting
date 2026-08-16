#!/usr/bin/env bats

# Managed Nix E2E test (issue #14: Linux x86_64 Docker smoke)
#
# Nix 未導入の Ubuntu container (systemd 有効) で schneeforge CLI の
# install → doctor → uninstall の full flow を検証する。
#
# 前提:
#   - Docker が利用可能 (CI: ubuntu-latest runner / ローカル)
#   - 事前に release build された schneeforge binary があること
#   - network access があること (nix-installer の download)
#
# 注意: `--privileged` は systemd (PID 1) 起動と build user 作成のために必要。
# container は毎 test で作成・破棄される (state は残らない)。

set -euo pipefail

# 1 container を複数 test で使い回すため直列実行
# shellcheck disable=SC2034
BATS_NO_PARALLELIZE=1

CT_NAME="schneeforge-e2e-bats"
SCHNEEFORGE_BIN="$BATS_TEST_DIRNAME/../target/release/schneeforge"
REPO_DIR="$(cd "$BATS_TEST_DIRNAME/.." && pwd)"
UBUNTU_IMAGE="${E2E_UBUNTU_IMAGE:-ubuntu:24.04}"

setup_file() {
  if ! command -v docker >/dev/null 2>&1; then
    skip "docker not available"
  fi
  if [ ! -x "$SCHNEEFORGE_BIN" ]; then
    echo "schneeforge binary not found at $SCHNEEFORGE_BIN" >&2
    return 1
  fi
  # systemd を install して PID 1 として起動 (nix-installer の default
  # planner は systemd を要求するため)
  docker rm -f "$CT_NAME" >/dev/null 2>&1 || true
  docker run -d --privileged --name "$CT_NAME" "$UBUNTU_IMAGE" \
    bash -c 'apt-get update -qq >/dev/null 2>&1; apt-get install -y -qq systemd systemd-sysv >/dev/null 2>&1; mkdir -p /opt/repo; exec /sbin/init' >/dev/null
  # systemd が起動するまで待つ (最大 300s)
  for _ in $(seq 1 60); do
    if docker exec "$CT_NAME" bash -c 'systemctl is-system-running 2>/dev/null | grep -qE "running|degraded"'; then
      break
    fi
    sleep 5
  done
  docker exec "$CT_NAME" systemctl is-system-running >/dev/null
  docker cp "$REPO_DIR/bootstrap-manifest.toml" "$CT_NAME:/opt/repo/bootstrap-manifest.toml"
  docker cp "$SCHNEEFORGE_BIN" "$CT_NAME:/usr/local/bin/schneeforge"
  docker exec "$CT_NAME" chmod +x /usr/local/bin/schneeforge
}

teardown_file() {
  docker rm -f "$CT_NAME" >/dev/null 2>&1 || true
}

# container 内で schneeforge を実行する helper
sf() {
  docker exec -e NIX_SETTING_DIR=/opt/repo "$CT_NAME" schneeforge "$@"
}

@test "container is Nix-less before install" {
  run docker exec "$CT_NAME" bash -c 'ls /nix 2>/dev/null; command -v nix 2>/dev/null; true'
  [ -z "$output" ]
}

@test "install: preflight dry-run works without root escalation" {
  run sf nix install --dry-run
  [ "$status" -eq 0 ]
  echo "$output" | grep -q "dry-run"
}

@test "install: full flow succeeds (plan → install → receipt → ownership → post-install gate)" {
  run sf nix install --yes
  [ "$status" -eq 0 ]
  echo "$output" | grep -q "Managed Nix install 完了"
  # artifacts
  docker exec "$CT_NAME" test -f /nix/receipt.json
  docker exec "$CT_NAME" test -f /nix/schneeforge-managed.json
}

@test "install: second install is rejected (ExistingNixDetected)" {
  run sf nix install --yes
  [ "$status" -ne 0 ]
  echo "$output" | grep -q "existing Nix detected"
}

@test "doctor: reports healthy install" {
  run docker exec -e NIX_SETTING_DIR=/opt/repo -e PATH="/nix/var/nix/profiles/default/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin" "$CT_NAME" schneeforge nix doctor
  [ "$status" -eq 0 ]
  echo "$output" | grep -q "receipt"
  echo "$output" | grep -q "version: 2.35.1"
  echo "$output" | grep -q "store accessible: true"
  echo "$output" | grep -q "flakes available: true"
}

@test "uninstall: without ownership record aborts (fail-closed)" {
  docker exec "$CT_NAME" rm -f /nix/schneeforge-managed.json
  run sf nix uninstall
  [ "$status" -ne 0 ]
  echo "$output" | grep -q "ownership record が見つかりません"
  # Nix 自体は残っている (abort は破壊を行わない)
  docker exec "$CT_NAME" test -f /nix/receipt.json
}

@test "uninstall: full flow (--force) succeeds and cleans up /nix" {
  # ownership record は前 test で削除済み。--force で cached installer の
  # SHA 検証 skip 経路を通る (record 無しの fail-closed を明示突破)
  run sf nix uninstall --force
  [ "$status" -eq 0 ]
  echo "$output" | grep -q "upstream uninstall 完了"
  docker exec "$CT_NAME" bash -c '! test -e /nix'
  docker exec "$CT_NAME" bash -c '! test -e /nix/receipt.json'
  docker exec "$CT_NAME" bash -c '! id nixbld1 2>/dev/null'
}

@test "install: re-install after full uninstall succeeds (cached installer)" {
  # /nix は完全除去済み。cache は /var/lib/schneeforge 配下に残っているため
  # 再 download なしで (plan → install → ownership 再記録) が成功する
  run sf nix install --yes
  [ "$status" -eq 0 ]
  echo "$output" | grep -q "Managed Nix install 完了"
  docker exec "$CT_NAME" test -f /nix/schneeforge-managed.json
}

@test "repair: Broken (stale ownership only) is repaired to Missing via doctor" {
  # uninstall 中断を模擬: /nix を手動で消して ownership record のみ残す
  docker exec "$CT_NAME" bash -c 'rm -rf /nix/nix-installer /nix/store /nix/var /nix/receipt.json'
  docker exec "$CT_NAME" test -f /nix/schneeforge-managed.json
  # doctor は Broken を報告し repair を案内する
  run sf nix doctor
  [ "$status" -eq 0 ]
  echo "$output" | grep -q "status:      Broken"
  echo "$output" | grep -q "schneeforge nix repair"
}

@test "repair: dry-run keeps stale ownership record, run removes it" {
  # dry-run は file system を変更しない
  run sf nix repair --dry-run
  [ "$status" -eq 0 ]
  echo "$output" | grep -q "dry-run"
  docker exec "$CT_NAME" test -f /nix/schneeforge-managed.json
  # 実行で stale record を削除し Missing へ復帰
  run sf nix repair
  [ "$status" -eq 0 ]
  echo "$output" | grep -q "stale ownership record を削除しました"
  docker exec "$CT_NAME" bash -c '! test -e /nix/schneeforge-managed.json'
  run sf nix doctor
  [ "$status" -eq 0 ]
  echo "$output" | grep -q "status:      Missing"
}

@test "repair: Missing suggests install" {
  run sf nix repair
  [ "$status" -eq 0 ]
  echo "$output" | grep -q "sudo schneeforge nix install"
}

@test "install: stale receipt.json alone is detected (fail-closed regression)" {
  # issue #14: /nix/receipt.json が残っている状態 (部分的に削除された
  # degraded install) での再 install が ExistingNixDetected で拒否されること。
  # PATH / store / var が無くても receipt marker 単独で検出する回帰保証。
  docker exec "$CT_NAME" bash -c 'mkdir -p /nix && echo "{}" > /nix/receipt.json'
  # dry-run は info 表示で exit 0 (D8: preview は中止を予告するだけ)
  run sf nix install --dry-run
  [ "$status" -eq 0 ]
  echo "$output" | grep -q "既存の Nix が検出されているため install は中止されます"
  # 実 install は ExistingNixDetected で拒否
  run sf nix install --yes
  [ "$status" -ne 0 ]
  echo "$output" | grep -q "existing Nix detected"
  # doctor は Degraded を報告する (marker が残る状態)
  run sf nix doctor
  [ "$status" -eq 0 ]
  echo "$output" | grep -q "Degraded"
}
