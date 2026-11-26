# nix_setting tasks — run: just <recipe>

default:
    @just --list

# ---- apply (advanced: configure the machine input before applying; normal users use schneeforge plan/apply) ----
apply:
    nh darwin switch . -H darwin-aarch64

apply-linux:
    nh home switch . -c linux

# ---- build / check ----
build:
    nix build .#homeConfigurations.darwin-aarch64.activationPackage

build-darwin:
    nix build .#darwinConfigurations.darwin-aarch64.system

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
