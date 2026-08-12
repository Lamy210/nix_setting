_: {
  perSystem =
    { pkgs, ... }:
    let
      detectHost = pkgs.writeShellApplication {
        name = "detect-host";
        text = ''
          case "$(uname -s)" in
            Darwin)
              case "$(uname -m)" in
                arm64 | aarch64) echo "macbook-air" ;;
                *) echo "unsupported" ;;
              esac
              ;;
            Linux)
              case "$(uname -m)" in
                aarch64 | arm64) echo "linux-arm" ;;
                x86_64 | amd64) echo "linux" ;;
                *) echo "unsupported" ;;
              esac
              ;;
            *)
              echo "unsupported"
              ;;
          esac
        '';
      };

      doctor = pkgs.writeShellApplication {
        name = "doctor";
        runtimeInputs = [
          pkgs.git
          detectHost
        ];
        text = ''
          set -u
          echo "=== doctor ==="
          echo

          echo "[system]"
          echo "  OS:   $(uname -s)"
          echo "  arch: $(uname -m)"
          echo

          echo "[nix]"
          if command -v nix >/dev/null 2>&1; then
            echo "  installed: yes"
            echo "  version:   $(nix --version 2>/dev/null || echo 'unknown')"
          else
            echo "  installed: no"
            echo "  install:   curl -L https://nixos.org/nix/install | sh"
          fi
          echo

          echo "[flakes]"
          if nix flake --version >/dev/null 2>&1; then
            echo "  supported: yes"
          else
            echo "  supported: no (enable experimental-features = nix-command flakes)"
          fi
          echo

          echo "[git]"
          if command -v git >/dev/null 2>&1; then
            echo "  installed: yes"
          else
            echo "  installed: no"
          fi
          echo

          echo "[host detection]"
          echo "  host: $(detect-host)"
        '';
      };

      apply = pkgs.writeShellApplication {
        name = "apply";
        runtimeInputs = [
          pkgs.nix
          detectHost
        ];
        text = ''
          set -eu
          host="$(detect-host)"
          if [ "$host" = "unsupported" ]; then
            echo "unsupported platform: $(uname -s) $(uname -m)"
            exit 1
          fi

          echo "applying host: $host"
          if [ "$host" = "macbook-air" ]; then
            nix run nix-darwin -- switch --flake ".#$host"
          else
            nix run nixpkgs#home-manager -- switch --flake ".#$host"
          fi
        '';
      };

      status = pkgs.writeShellApplication {
        name = "status";
        runtimeInputs = [
          pkgs.nix
          detectHost
        ];
        text = ''
          set -u
          host="$(detect-host)"
          echo "=== status ==="
          echo
          echo "  host: $host"
          echo "  nix:  $(nix --version 2>/dev/null || echo 'not installed')"
          echo "  flake: $(nix flake metadata --json 2>/dev/null | ${pkgs.jq}/bin/jq -r '.locks.nodes.root.locked.rev // "local"' 2>/dev/null || echo 'unknown')"
        '';
      };

      rollback = pkgs.writeShellApplication {
        name = "rollback";
        runtimeInputs = [
          pkgs.nix
          detectHost
        ];
        text = ''
          set -eu
          host="$(detect-host)"
          if [ "$host" = "unsupported" ]; then
            echo "unsupported platform: $(uname -s) $(uname -m)"
            exit 1
          fi

          echo "rolling back host: $host"
          if [ "$host" = "macbook-air" ]; then
            darwin-rebuild --rollback
          else
            nix run nixpkgs#home-manager -- switch --rollback
          fi
        '';
      };
    in
    {
      apps = {
        doctor = {
          type = "app";
          program = "${doctor}/bin/doctor";
          meta.description = "Diagnose system / Nix / host compatibility";
        };
        apply = {
          type = "app";
          program = "${apply}/bin/apply";
          meta.description = "Detect host and apply configuration (switch)";
        };
        status = {
          type = "app";
          program = "${status}/bin/status";
          meta.description = "Show current host and configuration status";
        };
        rollback = {
          type = "app";
          program = "${rollback}/bin/rollback";
          meta.description = "Rollback to previous generation";
        };
      };
    };
}
