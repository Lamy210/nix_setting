FROM nixos/nix:latest

RUN mkdir -p /root/.config/nix && \
    echo 'experimental-features = nix-command flakes' >> /root/.config/nix/nix.conf && \
    echo 'accept-flake-config = true' >> /root/.config/nix/nix.conf && \
    git config --global --add safe.directory /workspace

WORKDIR /workspace

ENTRYPOINT ["/bin/sh", "-c"]
