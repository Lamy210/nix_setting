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
      # nixpkgs 945c71b: Git 2.56 t1017 requires Bash invoked as POSIX sh.
      # Preserve the cross-compilation default and all other make flags.
      makeFlags = map (
        flag:
        if pkgs.lib.hasPrefix "SHELL_PATH=" flag then
          "SHELL_PATH=${pkgs.lib.getExe' pkgs.runtimeShellPackage "sh"}"
        else
          flag
      ) previous.makeFlags;

      # nixpkgs 885841e: Git 2.56 removed RUST_TARGET_DIR from its Makefile.
      # Remove only the exact old cross-build substitution, never other hooks.
      postPatch =
        let
          obsolete = ''
            substituteInPlace Makefile \
              --replace-fail "RUST_TARGET_DIR = target/" \
                             "RUST_TARGET_DIR = target/${pkgs.stdenv.hostPlatform.rust.cargoShortTarget}/"
          '';
          cleaned = builtins.replaceStrings [ obsolete ] [ "" ] previous.postPatch;
        in
        assert !(pkgs.lib.hasInfix "RUST_TARGET_DIR = target/" cleaned);
        cleaned;

      patches = builtins.filter (
        patch: !(pkgs.lib.hasSuffix "-darwin-unicode-filename-fix.patch" (toString patch))
      ) previous.patches;
      meta = previous.meta // {
        changelog = "https://github.com/git/git/blob/v${final.version}/Documentation/RelNotes/${final.version}.adoc";
      };
    }
  )
