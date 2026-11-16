# Git 2.56.1 fixes the security issues disclosed on 2026-11-10.
# Keep the root nixpkgs lock unchanged until its independent update is accepted.
# Archive SHA-256: https://www.kernel.org/pub/software/scm/git/sha256sums.asc
# The 2.56.0 nixpkgs recipe no longer carries darwin-unicode-filename-fix.patch
# (already integrated upstream); retain all other existing packaging patches.
{ pkgs }:
let
  version = "2.56.1";
in
if pkgs.lib.versionAtLeast pkgs.git.version version then
  pkgs.git
else
  pkgs.git.overrideAttrs (
    final: previous: {
      inherit version;
      src = pkgs.fetchurl {
        url = "https://www.kernel.org/pub/software/scm/git/git-${version}.tar.xz";
        hash = "sha256-8LC/iSZLm9LhxkA2PnUpZDmJ2nYoFgBHPFB0XNNKYH8=";
      };
      patches = builtins.filter (
        patch: !(pkgs.lib.hasSuffix "-darwin-unicode-filename-fix.patch" (toString patch))
      ) previous.patches;
      meta = previous.meta // {
        changelog = "https://github.com/git/git/blob/v${final.version}/Documentation/RelNotes/${final.version}.adoc";
      };
    }
  )
