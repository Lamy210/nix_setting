#!/usr/bin/env bash
set -euo pipefail

original='https://www.crufty.net/ftp/pub/sjg/bmake-20260313.tar.gz'
mirror='https://ftp.netbsd.org/pub/NetBSD/misc/sjg/bmake-20260313.tar.gz'
untouched='https://example.invalid/unrelated-source.tar.gz'

for policy in \
  modules/fetchurl-rewrite.nix \
  templates/node/fetchurl-rewrite.nix \
  templates/devenv/fetchurl-rewrite.nix; do
  actual="$(nix eval --raw --impure --expr "(import ./$policy) \"$original\"")"
  test "$actual" = "$mirror" || {
    echo "bmake URL rewrite failure: $policy" >&2
    exit 1
  }
  unchanged="$(nix eval --raw --impure --expr "(import ./$policy) \"$untouched\"")"
  test "$unchanged" = "$untouched" || {
    echo "rewrite must not alter other sources: $policy" >&2
    exit 1
  }
done

# Verify the fixed-output fetcher actually receives the mirror URL without
# downloading or building the archive (warm CI stores cannot mask bad eval).
urls="$(nix eval --impure --json --expr '
  let
    flake = builtins.getFlake (toString ./.);
    pkgs = import flake.inputs.nixpkgs {
      system = "x86_64-linux";
      config.rewriteURL = import ./modules/fetchurl-rewrite.nix;
    };
  in
  pkgs.bmake.src.urls
')"
printf '%s\n' "$urls" | jq -e --arg url "$mirror" 'index($url) != null' >/dev/null || {
  echo 'bmake fetchurl derivation does not point at the checksum-verified upstream mirror' >&2
  exit 1
}
echo 'Exact bmake rewrite, unrelated URL passthrough and real fetchurl inputs verified'
