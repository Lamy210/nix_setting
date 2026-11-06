#!/usr/bin/env bash
set -euo pipefail

# Experimental acceptance probe for the Node 24 security candidate.
# A version-only check is never sufficient evidence for a patched closure.
node -e '
const v = process.versions;
const [major, minor, patch] = v.node.split(".").map(Number);
if (major !== 24 || minor < 22 || (minor === 22 && patch < 2)) {
  console.error("Node 24 >=24.22.2 required, got " + v.node);
  process.exit(1);
}
console.log(JSON.stringify({node:v.node, brotli:v.brotli, ares:v.ares, openssl:v.openssl, execPath:process.execPath}));
if (v.brotli !== "1.2.1" || v.ares !== "1.34.9") {
  console.error("Patched Brotli 1.2.1 / c-ares 1.34.9 required, got " + v.brotli + " / " + v.ares);
  process.exit(1);
}
'
# Exercise patched native codepaths, not just version metadata and loader paths.
# DNS uses local OS getaddrinfo only: no public network or DNS server required.
node <<'NODE'
const assert = require("node:assert/strict");
const zlib = require("node:zlib");
const dns = require("node:dns");
const dgram = require("node:dgram");
const tls = require("node:tls");
const input = Buffer.from("SchneeForge Node 24 native Brotli ABI check. ".repeat(32));
for (const quality of [1, 5, 11]) {
  const compressed = zlib.brotliCompressSync(input, {
    params: { [zlib.constants.BROTLI_PARAM_QUALITY]: quality },
  });
  assert.deepEqual(zlib.brotliDecompressSync(compressed), input);
}
assert.ok(tls.createSecureContext().context, "OpenSSL secure context unavailable");
async function verifyDns() {
  const hosts = await new Promise((resolve, reject) => {
    dns.lookup("localhost", { all: true }, (err, records) => err ? reject(err) : resolve(records));
  });
  assert.ok(hosts.length > 0 && hosts.every((entry) => entry.address && entry.family));

  // Local UDP authoritative reply exercises the c-ares resolve4 path,
  // unlike dns.lookup which uses the OS getaddrinfo resolver.
  const server = dgram.createSocket("udp4");
  try {
    await new Promise((resolve, reject) => {
      server.once("error", reject);
      server.bind(0, "127.0.0.1", resolve);
    });
    server.on("message", (query, remote) => {
      if (query.length < 17) return;
      let offset = 12;
      while (offset < query.length && query[offset] !== 0) {
        const labelLength = query[offset];
        if (labelLength > 63 || offset + 1 + labelLength >= query.length) return;
        offset += 1 + labelLength;
      }
      if (offset + 5 > query.length) return;
      const question = query.subarray(12, offset + 5);
      const header = Buffer.from(query.subarray(0, 12));
      header.writeUInt16BE(0x8180, 2); // standard response, no error
      header.writeUInt16BE(1, 4); // one question
      header.writeUInt16BE(1, 6); // one answer
      header.writeUInt16BE(0, 8); // no authorities
      header.writeUInt16BE(0, 10); // no additional records
      const answer = Buffer.from([0xc0, 0x0c, 0, 1, 0, 1, 0, 0, 0, 0, 0, 4, 127, 0, 0, 1]);
      server.send(Buffer.concat([header, question, answer]), remote.port, remote.address);
    });
    const resolver = new dns.Resolver();
    resolver.setServers([`127.0.0.1:${server.address().port}`]);
    const records = await new Promise((resolve, reject) => {
      const timeout = setTimeout(() => {
        resolver.cancel();
        reject(new Error("local c-ares DNS smoke timed out"));
      }, 5000);
      resolver.resolve4("schneeforge-probe.invalid", (err, values) => {
        clearTimeout(timeout);
        err ? reject(err) : resolve(values);
      });
    });
    assert.deepEqual(records, ["127.0.0.1"]);
  } finally {
    server.close();
  }
}
verifyDns().then(
  () => console.log("Brotli, TLS, OS resolver and local UDP c-ares resolver smoke: OK"),
  (err) => { console.error(err); process.exitCode = 1; },
);
NODE
node_path="$(node -p 'process.execPath')"
if ! command -v nix-store >/dev/null 2>&1; then
  echo 'Missing nix-store; cannot inspect dependency closure' >&2
  exit 1
fi
case "$node_path" in
/nix/store/*) ;;
*)
  echo 'Node binary does not originate from the Nix store; refusing unverifiable host binary' >&2
  exit 1
  ;;
esac
echo "Node executable: $node_path"
case "$(uname -s)" in
Darwin)
  if ! command -v otool >/dev/null 2>&1; then
    echo 'otool missing: cannot record Mach-O dependency linkage' >&2
    exit 1
  fi
  echo 'Mach-O linked dynamic libraries:'
  linkage="$(otool -L "$node_path")"
  ;;
Linux)
  if ! command -v ldd >/dev/null 2>&1; then
    echo 'ldd missing: cannot record ELF dependency linkage' >&2
    exit 1
  fi
  echo 'ELF linked dynamic libraries:'
  linkage="$(ldd "$node_path")"
  ;;
*)
  echo "Unsupported host OS: $(uname -s)" >&2
  exit 1
  ;;
esac
printf '%s\n' "$linkage"
for lib in libbrotlidec libbrotlienc; do
  if ! printf '%s\n' "$linkage" | grep -E "/nix/store/[^/]+-brotli-1\\.2\\.1-lib/[^[:space:]]*${lib}" >/dev/null; then
    echo "Node dynamic link to patched Brotli 1.2.1 $lib not verified" >&2
    exit 1
  fi
done
if ! printf '%s\n' "$linkage" | grep -E '/nix/store/[^/]+-c-ares-1\.34\.9/[^[:space:]]*libcares' >/dev/null; then
  echo 'Node dynamic link to patched c-ares 1.34.9 not verified' >&2
  exit 1
fi
if command -v shasum >/dev/null 2>&1; then
  shasum -a 256 "$node_path"
elif command -v sha256sum >/dev/null 2>&1; then
  sha256sum "$node_path"
else
  echo 'No SHA-256 digest tool available for executable fingerprint' >&2
  exit 1
fi
closure="$(nix-store --query --requisites "$node_path")"
printf '%s\n' "$closure" | grep -E '/nix/store/[^/]+-brotli-1\.2\.1(-|$)' >/dev/null || {
  echo 'Verified Brotli 1.2.1 store closure reference not found' >&2
  exit 1
}
printf '%s\n' "$closure" | grep -E '/nix/store/[^/]+-c-ares-1\.34\.9(-|$)' >/dev/null || {
  echo 'Verified c-ares 1.34.9 store closure reference not found' >&2
  exit 1
}
echo 'Runtime version and Nix store dependency names match candidate; source / ABI / binary provenance still require review.'
