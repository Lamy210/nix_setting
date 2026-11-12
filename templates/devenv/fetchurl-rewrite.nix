# This exact bmake source URL is 404 on cold Nix/macOS builds (issue #302).
# Repoint only this fixed-version archive to the upstream documented NetBSD
# mirror. Nixpkgs fetchurl still validates the original SHA-256, and all
# unrelated source URLs pass through unchanged.
url:
if url == "https://www.crufty.net/ftp/pub/sjg/bmake-20260313.tar.gz" then
  "https://ftp.netbsd.org/pub/NetBSD/misc/sjg/bmake-20260313.tar.gz"
else
  url
