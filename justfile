# nix_setting tasks — run: just <recipe>

default:
    @just --list

# ---- apply ----
apply:
    nh darwin switch .#darwinConfigurations.macbook-air

apply-linux:
    nh home switch .#homeConfigurations.linux

# ---- build / check ----
build:
    nix build .#homeConfigurations.macbook-air.activationPackage

build-darwin:
    nix build .#darwinConfigurations.macbook-air.system

check:
    nix flake check --allow-import-from-derivation

fmt:
    nix fmt

update:
    nix flake update

# ---- lint / test ----
lint:
    statix check .
    deadnix --fail .
    shellcheck bootstrap.sh bench-shell.sh
    actionlint

test:
    bats tests/bootstrap.bats
    nix-unit tests/nix-unit.nix

bench:
    ./bench-shell.sh

# ---- docker ----
docker-check:
    docker compose run check

# ---- clean ----
clean:
    nix-store --gc
