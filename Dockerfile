FROM nixos/nix:latest

RUN mkdir -p /root/.config/nix && \
    echo 'experimental-features = nix-command flakes' >> /root/.config/nix/nix.conf && \
    echo 'accept-flake-config = true' >> /root/.config/nix/nix.conf

WORKDIR /workspace

ENTRYPOINT ["/bin/sh", "-c"]
