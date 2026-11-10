#!/usr/bin/env python3
"""Check full Nix installer downloads against the pinned manifest across target architectures.

This verifies transfer completion, hash, and object format. It does not execute
cross-platform binaries or prove installation on non-host platforms.
"""
from __future__ import annotations

import argparse
import hashlib
import hmac
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
from pathlib import Path
import re
import subprocess
import tempfile
from threading import Thread
import tomllib
from urllib.parse import urlsplit
from urllib.request import urlopen

TARGETS = ("x86_64-linux", "aarch64-linux", "aarch64-darwin")
MAX_ASSET_BYTES = 160 * 1024 * 1024
MACHO_MAGIC = {
    bytes.fromhex(value) for value in ("cffaedfe", "feedfacf", "cafebabe", "bebafeca")
}


def download_checked(
    url: str, expected: str, dest: Path, target: str, *,
    local_test_only: bool = False
) -> dict:
    parsed = urlsplit(url)
    if parsed.scheme != "https":
        if not (local_test_only and parsed.scheme == "http" and parsed.hostname == "127.0.0.1"):
            raise ValueError("non-HTTPS download refused")
    if target not in TARGETS:
        raise ValueError(f"unsupported target {target}")
    if not re.fullmatch(r"[0-9a-fA-F]{64}", expected):
        raise ValueError("expected digest is not a SHA256 hex digest")
    sha = hashlib.sha256()
    count = 0
    # Verify the entire downloaded object before publishing its final path.
    staging = dest.with_name(dest.name + ".part")
    try:
        with urlopen(url, timeout=65) as response:
            if response.status != 200:
                raise ValueError(f"unexpected HTTP status {response.status}")
            if urlsplit(response.url).scheme != parsed.scheme:
                raise ValueError("unexpected redirect protocol change")
            advertised = response.headers.get("Content-Length")
            expected_bytes = int(advertised) if advertised is not None else None
            if expected_bytes is not None and (expected_bytes <= 0 or expected_bytes > MAX_ASSET_BYTES):
                raise ValueError("invalid Content-Length")
            with staging.open("xb") as file:
                while True:
                    block = response.read(128 * 1024)
                    if not block:
                        break
                    count += len(block)
                    if count > MAX_ASSET_BYTES:
                        raise ValueError("download exceeds maximum allowed size")
                    sha.update(block)
                    file.write(block)
        if count == 0 or (expected_bytes is not None and count != expected_bytes):
            raise ValueError(f"incomplete download: {count} bytes, advertised {expected_bytes}")
        digest = sha.hexdigest()
        if not hmac.compare_digest(digest.lower(), expected.lower()):
            raise ValueError(f"SHA256 mismatch for {target}: {digest}")
        with staging.open("rb") as file:
            magic = file.read(4)
        if target.endswith("-linux") and magic != b"\x7fELF":
            raise ValueError(f"Linux asset is not ELF: {magic!r}")
        if target.endswith("-darwin") and magic not in MACHO_MAGIC:
            raise ValueError(f"Darwin asset is not Mach-O: {magic!r}")
        staging.replace(dest)
        return {"target": target, "bytes": count, "sha256": digest, "status": "verified"}
    except Exception:
        staging.unlink(missing_ok=True)
        raise


class FixtureHandler(BaseHTTPRequestHandler):
    BODY = b"\x7fELF" + b"download-fixture" * 3
    WRONG_FORMAT = b"NOT-AN-ELF" + b"download-fixture" * 3

    def log_message(self, _format, *args):
        pass

    def do_GET(self):
        if self.path == "/404":
            self.send_error(404)
            return
        if self.path == "/no-length":
            self.send_response(200)
            self.end_headers()
            self.wfile.write(self.BODY)
            self.wfile.flush()
            self.close_connection = True
            return
        if self.path == "/oversized":
            self.send_response(200)
            self.send_header("Content-Length", str(MAX_ASSET_BYTES + 1))
            self.end_headers()
            return
        if self.path == "/bad-magic":
            self.send_response(200)
            self.send_header("Content-Length", str(len(self.WRONG_FORMAT)))
            self.end_headers()
            self.wfile.write(self.WRONG_FORMAT)
            return
        if self.path == "/truncated":
            self.send_response(200)
            self.send_header("Content-Length", str(len(self.BODY) + 5))
            self.end_headers()
            self.wfile.write(self.BODY)
            self.wfile.flush()
            self.close_connection = True
            return
        self.send_response(200)
        self.send_header("Content-Length", str(len(self.BODY)))
        self.end_headers()
        self.wfile.write(self.BODY)


def self_test() -> None:
    server = ThreadingHTTPServer(("127.0.0.1", 0), FixtureHandler)
    thread = Thread(target=server.serve_forever, daemon=True)
    thread.start()
    url = f"http://127.0.0.1:{server.server_address[1]}"
    digest = hashlib.sha256(FixtureHandler.BODY).hexdigest()
    try:
        with tempfile.TemporaryDirectory() as tmp:
            for name, suffix, sha, success in (
                ("complete", "/complete", digest, True),
                ("without-length", "/no-length", digest, True),
                ("bad-magic", "/bad-magic", hashlib.sha256(FixtureHandler.WRONG_FORMAT).hexdigest(), False),
                ("oversized", "/oversized", digest, False),
                ("badsha", "/complete", "0" * 64, False),
                ("truncated", "/truncated", digest, False),
                ("notfound", "/404", digest, False),
            ):
                try:
                    result = download_checked(
                        url + suffix, sha, Path(tmp) / name, "x86_64-linux",
                        local_test_only=True
                    )
                except Exception:
                    if success:
                        raise
                else:
                    if not success:
                        raise AssertionError(f"{name}: an invalid transfer was accepted")
                    assert result["bytes"] == len(FixtureHandler.BODY)
                if not success:
                    assert not (Path(tmp) / name).exists(), "invalid download was published"
                    assert not (Path(tmp) / (name + ".part")).exists(), "partial cache leaked"
            # The scheme is always enforced outside explicit localhost tests.
            try:
                download_checked(url + "/complete", digest, Path(tmp) / "denied", "x86_64-linux")
            except ValueError:
                pass
            else:
                raise AssertionError("HTTP download must be rejected")
    finally:
        server.shutdown()
        server.server_close()
        thread.join(timeout=5)
    print("PASS: completed and no-length downloads; rejected oversized, malformed binary, HTTP error, truncation, SHA mismatch, and non-TLS")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument("--native-help-smoke", action="store_true", help="execute verified Linux x64 --help on ephemeral CI")
    parser.add_argument("--manifest", type=Path)
    parser.add_argument("--out", type=Path)
    args = parser.parse_args()
    if args.self_test:
        self_test()
        return
    if not args.manifest or not args.out:
        parser.error("--manifest and --out are required")
    manifest = tomllib.loads(args.manifest.read_text(encoding="utf-8"))["managed_nix"]
    version = manifest["version"]
    if not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", version):
        raise ValueError("invalid installer release version")
    pins = manifest["sha256_by_arch"]
    args.out.mkdir(parents=True, exist_ok=True)
    verified = []
    with tempfile.TemporaryDirectory(prefix="schneeforge-assets-") as temp:
        for target in TARGETS:
            url = f"https://github.com/NixOS/nix-installer/releases/download/{version}/nix-installer-{target}"
            executable = Path(temp) / target
            result = download_checked(url, pins[target], executable, target)
            if args.native_help_smoke and target == "x86_64-linux":
                executable.chmod(0o500)
                subprocess.run(
                    [str(executable), "--help"], check=True, timeout=25,
                    stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.PIPE
                )
                result["execution"] = "help_success"
            else:
                result["execution"] = "not_executed"
            verified.append(result)
            print(f"PASS: {target}: {result['bytes']} bytes, SHA256 {result['sha256']}")
    (args.out / "results.json").write_text(
        json.dumps({"version": version, "assets": verified}, indent=2) + "\n",
        encoding="utf-8"
    )
    (args.out / "summary.md").write_text(
        "### Managed Nix: 全配布対象アセットの実ダウンロード確認\n\n"
        + "| OS/CPU | バイト数 | SHA256 | 取得検証 | 実行確認 |\n|---|---:|---|---|---|\n"
        + "".join(
            f"| {item['target']} | {item['bytes']:,} | \`{item['sha256']}\` | OK | {item['execution']} |\n"
            for item in verified
        )
        + "\n完全取得・ハッシュ・ELF/Mach-O形式の検査です。Linux x64は指定時に--helpの実起動も検査します。"
          "macOS/ARM64で実際に導入・適用できる証明ではありません。\n",
        encoding="utf-8"
    )


if __name__ == "__main__":
    main()
