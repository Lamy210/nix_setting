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
# Exercise stable C Node-API by compiling and loading a native addon.
# The fixture uses the published opaque-pointer C ABI (no external npm install)
# and deliberately avoids conflating a basic N-API load with C++ ABI validation.
if ! command -v cc >/dev/null 2>&1; then
  echo 'C compiler missing: native Node-API load test cannot run' >&2
  exit 1
fi
addon_dir="$(mktemp -d)"
trap 'rm -rf "$addon_dir"' EXIT
cat >"$addon_dir/abi.c" <<'NODE_API_C'
#include <stdint.h>
typedef struct napi_env__ *napi_env;
typedef struct napi_value__ *napi_value;
typedef int32_t napi_status;
extern napi_status napi_create_int32(napi_env env, int32_t value, napi_value *result);
extern napi_status napi_set_named_property(napi_env env, napi_value object, const char *name, napi_value value);
__attribute__((visibility("default")))
napi_value napi_register_module_v1(napi_env env, napi_value exports) {
  napi_value answer;
  if (napi_create_int32(env, 42, &answer) != 0) return (napi_value)0;
  if (napi_set_named_property(env, exports, "answer", answer) != 0) return (napi_value)0;
  return exports;
}
NODE_API_C
case "$(uname -s)" in
Darwin) cc -bundle -undefined dynamic_lookup -o "$addon_dir/abi.node" "$addon_dir/abi.c" ;;
Linux) cc -shared -fPIC -o "$addon_dir/abi.node" "$addon_dir/abi.c" ;;
*)
  echo 'Unsupported platform for Node-API smoke test' >&2
  exit 1
  ;;
esac
node -e 'const assert = require("node:assert/strict"); assert.equal(require(process.argv[1]).answer, 42); console.log("Native Node-API compiled addon load: OK");' "$addon_dir/abi.node"

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
# Do not interpret ldd's own loader mapping as the Node ELF PT_INTERP.
# Directly inspect the actual loader embedded in the binary and compare its
# glibc store output with the libc.so.6 path exposed by the dynamic linker.
if [ "$(uname -s)" = Linux ]; then
  if ! command -v readelf >/dev/null 2>&1; then
    echo 'readelf missing: cannot establish actual ELF interpreter' >&2
    exit 1
  fi
  interpreter="$(LC_ALL=C readelf -l "$node_path" | sed -n 's/.*Requesting program interpreter: \([^]]*\)].*/\1/p')"
  case "$interpreter" in
  /nix/store/*/lib/ld-*.so* | /nix/store/*/lib64/ld-*.so*) ;;
  *)
    echo "Invalid or missing Nix ELF PT_INTERP: $interpreter" >&2
    exit 1
    ;;
  esac
  if [ ! -f "$interpreter" ]; then
    echo "ELF interpreter is absent: $interpreter" >&2
    exit 1
  fi
  interpreter_store="$(printf '%s\n' "$interpreter" | cut -d/ -f1-4)"
  libc_path="$(printf '%s\n' "$linkage" | awk '$1 == "libc.so.6" && $2 == "=>" { print $3; exit }')"
  case "$libc_path" in
  /nix/store/*/lib/libc.so.6) ;;
  *)
    echo "Unexpected glibc shared object: $libc_path" >&2
    exit 1
    ;;
  esac
  libc_store="$(printf '%s\n' "$libc_path" | cut -d/ -f1-4)"
  if [ "$interpreter_store" != "$libc_store" ]; then
    echo "Node ELF interpreter/libc Nix outputs differ: $interpreter_store vs $libc_store" >&2
    exit 1
  fi
  if ! printf '%s\n' "$closure" | grep -Fx "$interpreter_store" >/dev/null; then
    echo "ELF interpreter glibc output is absent from Node closure: $interpreter_store" >&2
    exit 1
  fi
  echo "Direct ELF PT_INTERP and libc store output agree: $interpreter"
fi

echo 'Runtime version and Nix store dependency names match candidate; source / ABI / binary provenance still require review.'
