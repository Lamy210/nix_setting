{ ... }:
{
  perSystem =
    { pkgs, ... }:
    let
      doctor = pkgs.writeShellApplication {
        name = "doctor";
        runtimeInputs = [ pkgs.git ];
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
          case "$(uname -s)" in
            Darwin)
              case "$(uname -m)" in
                arm64 | aarch64) echo "  host: macbook-air" ;;
                *) echo "  host: unsupported" ;;
              esac
              ;;
            Linux)
              case "$(uname -m)" in
                aarch64 | arm64) echo "  host: linux-arm" ;;
                x86_64 | amd64) echo "  host: linux" ;;
                *) echo "  host: unsupported" ;;
              esac
              ;;
            *)
              echo "  host: unsupported"
              ;;
          esac
        '';
      };

      apply = pkgs.writeShellApplication {
        name = "apply";
        runtimeInputs = [ pkgs.nix ];
        text = ''
          set -eu
          host=""
          case "$(uname -s)" in
            Darwin)
              case "$(uname -m)" in
                arm64 | aarch64) host="macbook-air" ;;
                *) echo "unsupported Darwin arch: $(uname -m)"; exit 1 ;;
              esac
              ;;
            Linux)
              case "$(uname -m)" in
                aarch64 | arm64) host="linux-arm" ;;
                x86_64 | amd64) host="linux" ;;
                *) echo "unsupported Linux arch: $(uname -m)"; exit 1 ;;
              esac
              ;;
            *)
              echo "unsupported OS: $(uname -s)"; exit 1 ;;
          esac

          echo "applying host: $host"
          if [ "$host" = "macbook-air" ]; then
            nix run nix-darwin -- switch --flake ".#$host"
          else
            nix run nixpkgs#home-manager -- switch --flake ".#$host"
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
      };
    };
}
