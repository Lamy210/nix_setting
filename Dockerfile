FROM nixos/nix@sha256:d78540374f6a886653cba47d5c3f61c5a41d42e2a8db2607b8d68cb226fd463e

RUN mkdir -p /root/.config/nix && \
    echo 'experimental-features = nix-command flakes' >> /root/.config/nix/nix.conf && \
    echo 'accept-flake-config = true' >> /root/.config/nix/nix.conf && \
    git config --global --add safe.directory /workspace

WORKDIR /workspace

ENTRYPOINT ["/bin/sh", "-c"]
