#!/usr/bin/env python3
"""UI behavior tests and PR-base screenshot differences with mocked Tauri IPC.

This does not validate a native Tauri window or real Nix installation.
"""
from __future__ import annotations

import argparse
from contextlib import contextmanager
from functools import partial
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from io import BytesIO
import json
from pathlib import Path
from threading import Thread
from urllib.parse import urlsplit

from PIL import Image, ImageChops
from playwright.sync_api import sync_playwright

SCENARIOS = ("dashboard", "first-run", "nix-confirmation", "apply-result", "verify-result")
MOCK = """
(() => {
  const scene = window.__scene;
  window.__calls = [];
  window.__failures = {};
  window.__TAURI__ = {
    core: { invoke: async (command, args) => {
      window.__calls.push({ command, args: args || null });
      if (window.__failures[command]) {
        return { success: false, output: window.__failures[command] };
      }
      const setup = scene === "first-run" || scene === "nix-confirmation";
      const hasNix = scene !== "nix-confirmation";
      switch (command) {
        case "get_status": return {
          host: "ci-test-host", platform: "linux", architecture: "x86_64",
          profile: "minimal", repo_exists: !setup, managed_source: !setup,
          applied_revision: "abc1234", state_error: null,
          nix_status: { status: hasNix ? "Healthy" : "Missing", next_action: null },
          tools: { nix: { available: hasNix }, homebrew: { available: false } }
        };
        case "get_dashboard": return {
          installed: { version: "0.2.0-rc.7", profile: "minimal", channel: "preview" },
          available: null, available_error: "offline fixture", update_available: false
        };
        case "get_profiles": return {
          available: ["minimal", "developer"], default: "minimal", selected: null
        };
        case "get_app_updater_capability": return {
          enabled: false, reason: "test fixture"
        };
        case "run_preflight": return {
          nix_installed: hasNix, git_installed: true, flakes_enabled: true
        };
        case "nix_prepare_plan": return {
          success: true, output: "InstallPlan: verified download fixture"
        };
        case "run_verify": return {
          checks: [{ name: "Nix store", ok: true }, { name: "Profile", ok: true }]
        };
        case "run_apply": return { success: true, output: "設定の適用が完了しました" };
        case "run_plan": return { success: true, output: "変更予定なし" };
        default: return { success: true, output: "fixture: " + command };
      }
    }},
    event: { listen: async () => () => {} }
  };
})();
"""


class QuietHandler(SimpleHTTPRequestHandler):
    def log_message(self, _format, *args):
        pass


@contextmanager
def serve(root: Path):
    if not (root / "index.html").is_file() or not (root / "main.js").is_file():
        raise FileNotFoundError(f"missing static frontend in {root}")
    server = ThreadingHTTPServer(
        ("127.0.0.1", 0), partial(QuietHandler, directory=str(root))
    )
    thread = Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        yield f"http://127.0.0.1:{server.server_address[1]}/index.html"
    finally:
        server.shutdown()
        server.server_close()
        thread.join(timeout=5)


def scene(browser, url: str, name: str):
    context = browser.new_context(
        viewport={"width": 1100, "height": 850}, device_scale_factor=1,
        locale="ja-JP", color_scheme="dark", reduced_motion="reduce"
    )
    context.add_init_script(f"window.__scene={json.dumps(name)};" + MOCK)
    context.route("**/*", lambda route: (
        route.continue_() if urlsplit(route.request.url).hostname == "127.0.0.1"
        else route.abort()
    ))
    page = context.new_page()
    errors = []
    page.on("pageerror", lambda error: errors.append(str(error)))
    try:
        page.goto(url, wait_until="load", timeout=15000)
        if name in ("first-run", "nix-confirmation"):
            page.locator("#setup-view:not(.hidden)").wait_for(timeout=10000)
            page.get_by_text("Nix status").wait_for(timeout=10000)
            if name == "nix-confirmation":
                page.get_by_role("button", name="SchneeForge で導入").click()
                page.get_by_role("button", name="導入する").wait_for(timeout=10000)
        else:
            page.locator("#ready-view:not(.hidden)").wait_for(timeout=10000)
            page.wait_for_function(
                "document.getElementById('dash-installed').textContent === 'v0.2.0-rc.7'",
                timeout=10000
            )
            if name == "apply-result":
                page.locator("#apply").click()
                page.wait_for_function(
                    "document.getElementById('output').textContent.includes('設定の適用が完了しました')",
                    timeout=10000
                )
            if name == "verify-result":
                page.locator("#verify").click()
                page.wait_for_function(
                    "document.getElementById('output').textContent.includes('すべて OK')",
                    timeout=10000
                )
        page.evaluate("document.fonts.ready")
        page.wait_for_timeout(100)
        if errors:
            raise RuntimeError(f"JavaScript errors in {name}: {errors}")
        return context, page
    except Exception:
        context.close()
        raise


def snapshot(browser, url: str, name: str) -> bytes:
    context, page = scene(browser, url, name)
    try:
        return page.screenshot(full_page=True, animations="disabled", caret="hide")
    finally:
        context.close()


def create_diff(before: bytes, after: bytes, dest: Path) -> tuple[int, int]:
    with Image.open(BytesIO(before)) as left, Image.open(BytesIO(after)) as right:
        dimensions = (max(left.width, right.width), max(left.height, right.height))
        first = Image.new("RGB", dimensions, (15, 23, 42))
        second = Image.new("RGB", dimensions, (15, 23, 42))
        first.paste(left.convert("RGB"))
        second.paste(right.convert("RGB"))
        mask = ImageChops.difference(first, second).convert("L").point(
            lambda pixel: 255 if pixel > 15 else 0
        )
        overlay = Image.new("RGBA", dimensions, (255, 48, 48, 0))
        overlay.putalpha(mask.point(lambda pixel: 165 if pixel else 0))
        Image.alpha_composite(second.convert("RGBA"), overlay).save(dest)
        return mask.histogram()[255], dimensions[0] * dimensions[1]


def check_behavior(browser, url: str) -> None:
    context, page = scene(browser, url, "dashboard")
    try:
        assert page.locator("#dash-profile").inner_text() == "minimal"
        page.locator("#apply").click()
        page.wait_for_function(
            "document.getElementById('output').textContent.includes('設定の適用が完了しました')"
        )
        page.locator("#verify").click()
        page.wait_for_function(
            "document.getElementById('output').textContent.includes('すべて OK')"
        )
        page.on("dialog", lambda dialog: dialog.dismiss())
        page.locator("#nix-uninstall").click()
        calls = page.evaluate("window.__calls.map(call => call.command)")
        assert "run_apply" in calls and "run_verify" in calls
        assert "nix_uninstall_escalated" not in calls, "cancel unexpectedly uninstalled Nix"
        page.evaluate("window.__failures.run_apply = 'simulated failure'")
        page.locator("#apply").click()
        page.wait_for_function(
            "document.getElementById('output').textContent.includes('simulated failure')"
        )
    finally:
        context.close()

    context, page = scene(browser, url, "nix-confirmation")
    try:
        assert "nix_prepare_plan" in page.evaluate("window.__calls.map(c => c.command)")
        page.get_by_role("button", name="キャンセル").click()
        assert "nix_install_escalated" not in page.evaluate(
            "window.__calls.map(c => c.command)"
        ), "cancel unexpectedly installed Nix"
    finally:
        context.close()


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--mode", choices=("behavior", "compare"), required=True)
    parser.add_argument("--candidate", type=Path, required=True)
    parser.add_argument("--baseline", type=Path)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)

    with sync_playwright() as playwright:
        browser = playwright.chromium.launch()
        try:
            with serve(args.candidate.resolve()) as candidate:
                if args.mode == "behavior":
                    check_behavior(browser, candidate)
                    print("PASS: UI behavior and cancellation")
                    return
                if not args.baseline:
                    parser.error("--baseline required for comparison")
                with serve(args.baseline.resolve()) as baseline:
                    results = []
                    for name in SCENARIOS:
                        before, after = (
                            snapshot(browser, baseline, name),
                            snapshot(browser, candidate, name)
                        )
                        (args.out / f"{name}-before.png").write_bytes(before)
                        (args.out / f"{name}-after.png").write_bytes(after)
                        changed, total = create_diff(
                            before, after, args.out / f"{name}-diff.png"
                        )
                        results.append({
                            "scenario": name, "changed_pixels": changed,
                            "total_pixels": total, "ratio": round(changed / total, 6)
                        })
                        print(f"{name}: {changed}/{total} pixels changed")
                    (args.out / "results.json").write_text(
                        json.dumps(results, ensure_ascii=False, indent=2) + "\n",
                        encoding="utf-8"
                    )
                    lines = [
                        "### SchneeForge UI画像差分（PRベースとの比較）",
                        "",
                        "| 画面 | 差分ピクセル | 差分率 |",
                        "|---|---:|---:|",
                    ]
                    for result in results:
                        lines.append(
                            f"| {result['scenario']} | {result['changed_pixels']:,} | "
                            f"{result['ratio']:.2%} |"
                        )
                    lines.extend([
                        "",
                        "各画面の変更前・変更後・赤色で強調した差分PNGはArtifactsにあります。",
                        "Tauri IPCは固定fixtureであり、実インストールの保証ではありません。",
                        "",
                    ])
                    (args.out / "summary.md").write_text(
                        "\n".join(lines), encoding="utf-8"
                    )
        finally:
            browser.close()


if __name__ == "__main__":
    main()
