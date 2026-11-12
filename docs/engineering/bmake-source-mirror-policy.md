# bmake 20260313 fixed-output mirror policy

## Scope

Issue #302 tracked a reproducible HTTP 404 while fetching the pinned Nixpkgs `bmake-20260313.tar.gz` on a cold macOS 26 runner. An unmodified pinned Nixpkgs recipe still refers to `https://www.crufty.net/ftp/pub/sjg/bmake-20260313.tar.gz`. The upstream bmake project lists `https://ftp.netbsd.org/pub/NetBSD/misc/sjg/` as its official mirror.

This repository configures Nixpkgs `config.rewriteURL` for exactly that one archival URL, redirecting it to `https://ftp.netbsd.org/pub/NetBSD/misc/sjg/bmake-20260313.tar.gz`. All other URLs are unchanged. The Nixpkgs package's original `fetchurl` SHA-256 `sha256-dsjzzULuBc/7R7zIElbj1edCb00I5zN4i0WYXe30+XU=` remains authoritative. The hash check is **not** disabled; a different archive fails.

The policy is passed to root perSystem package imports, three explicit Home Manager imports, and the nix-darwin system module config. Standalone node/devenv templates embed the identical rewrite so that copying only each template folder remains valid.

## Scope limits and acceptance

- This is a repository-level fetcher policy; it cannot alter an unrelated external `nixpkgs.legacyPackages` instance unless that package set is imported/configured with the same policy.
- The pinned Node security package input in Draft #299 is a separate Nixpkgs package set; it must also use this rewrite after rebase rather than relying on its CI-only store seed. Until then, #299 remains a separate gate.
- Run `nix flake check` and macOS 15/26 Home Manager/nix-darwin CI on a cold runner; require source access without CI prefetch. Verify each standalone template independently and record actual fetch logs on a clean store. CI caching can hide failure, so absence of a 404 on a warm store alone does **not** close #302.
- Prefer an upstream Nixpkgs fix with stable mirror/fallback URLs; retire the local rewrite after the pinned Nixpkgs recipe gains a permanent remedy and clean-host evidence passes.
- No release tag or updater enablement is authorized by this policy.
