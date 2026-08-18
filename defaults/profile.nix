# profile input の placeholder。
# clone 直後の `nix flake check` 等が通るように同梱している。
# SchneeForge の apply / plan は選択 profile ({ profile = "<name>"; })
# を生成し --override-input profile <path> で差し替える。
# null は「未選択」を表し、modules/profile-input.nix が manifest の
# default (developer) へ fallback する。
{ profile = null; }
