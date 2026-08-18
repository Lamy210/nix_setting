#!/usr/bin/env bats

# v2 P1 (add-configuration-sources): schneeforge update の source kind dispatch の
# bats smoke。cargo test が source.rs の分類を detail に検証するのに対し、ここは
# 実 binary が git repo の状態を正しく分類して no-op / pull を使い分けるかを見る。

SCHNEEFORGE_BIN="${BATS_TEST_DIRNAME}/../target/release/schneeforge"

make_repo() {
  local dir="$1"
  rm -rf "$dir"
  mkdir -p "$dir"
  git -C "$dir" init -q -b main
  git -C "$dir" -c user.email=t@t -c user.name=t commit -q --allow-empty -m init
  echo "$dir"
}

@test "schneeforge binary exists" {
  [ -x "$SCHNEEFORGE_BIN" ]
}

@test "source status reports git-tracking for branch checkout" {
  dir="$(make_repo "$BATS_TEST_TMPDIR/tracking")"
  run "$SCHNEEFORGE_BIN" --repo "$dir" source status
  [ "$status" -eq 0 ]
  grep -q "kind:    git-tracking" <<<"$output"
  grep -q "ref:     main" <<<"$output"
}

@test "source status reports release-stable for tag checkout" {
  dir="$(make_repo "$BATS_TEST_TMPDIR/stable")"
  git -C "$dir" tag v0.2.0
  git -C "$dir" checkout -q v0.2.0
  run "$SCHNEEFORGE_BIN" --repo "$dir" source status
  [ "$status" -eq 0 ]
  grep -q "kind:    release-stable" <<<"$output"
  grep -q "channel: stable" <<<"$output"
}

@test "update on git-pinned is a no-op with guidance" {
  dir="$(make_repo "$BATS_TEST_TMPDIR/pinned")"
  rev="$(git -C "$dir" rev-parse HEAD)"
  git -C "$dir" checkout -q "$rev"
  run "$SCHNEEFORGE_BIN" --repo "$dir" update
  [ "$status" -eq 0 ]
  grep -q "pinned" <<<"$output"
  # HEAD は移動しない
  [ "$(git -C "$dir" rev-parse HEAD)" = "$rev" ]
}

@test "update on local (non-git) dir is a no-op" {
  dir="$BATS_TEST_TMPDIR/local"
  rm -rf "$dir"
  mkdir -p "$dir"
  run "$SCHNEEFORGE_BIN" --repo "$dir" update
  [ "$status" -eq 0 ]
  grep -q "local" <<<"$output"
}
