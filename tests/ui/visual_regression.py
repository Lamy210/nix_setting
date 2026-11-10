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

SCENARIOS = ("dashboard", "first-run", "nix-confirmation", "apply-result", "verify-result", "app-updater")
VIEWPORTS = {"native-800x600": (800, 600), "wide-1100x850": (1100, 850)}
MOCK = """
(() => {
  const scene = window.__scene;
  window.__calls = [];
  window.__failures = {};
  window.__rejections = {};
  window.__listeners = {};
  window.__emit = (name, payload) => window.__listeners[name]?.({ payload });
  window.__TAURI__ = {
    core: { invoke: async (command, args) => {
      window.__calls.push({ command, args: args || null });
      if (window.__rejections[command]) throw new Error(window.__rejections[command]);
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
          enabled: scene === "app-updater", reason: "test fixture"
        };
        case "fetch_app_update": return {
          version: "0.2.0-rc.8", currentVersion: "0.2.0-rc.7", notes: "CI update fixture"
        };
        case "install_app_update":
          window.__emit("app-update-progress", { phase: "downloading", downloaded: 100, contentLength: 100 });
          window.__emit("app-update-progress", { phase: "downloaded" });
          window.__emit("app-update-progress", { phase: "installed" });
          return { success: true, output: "signed update installed" };
        case "run_preflight": return {
          nix_installed: hasNix, git_installed: true, flakes_enabled: true
        };
        case "nix_prepare_plan": return {
          success: true, output: "InstallPlan: verified download fixture"
        };
        case "run_verify": return window.__reportOverride || {
          checks: [{ name: "Nix store", ok: true }, { name: "Profile", ok: true }]
        };
        case "run_apply": return { success: true, output: "設定の適用が完了しました" };
        case "run_plan": return { success: true, output: "変更予定なし" };
        case "nix_install_escalated":
          window.__emit("nix-install-progress", { phase: "download", message: "download complete" });
          window.__emit("nix-install-progress", { phase: "verify", message: "SHA256 verified" });
          window.__emit("nix-install-progress", { phase: "install", message: "completed" });
          return { success: true, output: "Managed Nix installed" };
        default: throw new Error("Unmocked Tauri IPC command: " + command);
      }
    }},
    event: { listen: async (name, handler) => {
      window.__listeners[name] = handler;
      return () => { delete window.__listeners[name]; };
    } }
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


def scene(browser, url: str, name: str, viewport: tuple[int, int] = (800, 600)):
    width, height = viewport
    context = browser.new_context(
        viewport={"width": width, "height": height}, device_scale_factor=1,
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


def snapshot(browser, url: str, name: str, viewport: tuple[int, int]) -> bytes:
    context, page = scene(browser, url, name, viewport)
    try:
        return page.screenshot(full_page=True, animations="disabled", caret="hide")
    finally:
        context.close()


def create_diff(before: bytes, after: bytes, dest: Path, pixel_threshold: int = 0) -> tuple[int, int]:
    if not 0 <= pixel_threshold <= 255:
        raise ValueError("pixel threshold must be 0..255")
    with Image.open(BytesIO(before)) as left, Image.open(BytesIO(after)) as right:
        dimensions = (max(left.width, right.width), max(left.height, right.height))
        first = Image.new("RGB", dimensions, (15, 23, 42))
        second = Image.new("RGB", dimensions, (15, 23, 42))
        first.paste(left.convert("RGB"))
        second.paste(right.convert("RGB"))
        red, green, blue = ImageChops.difference(first, second).split()
        mask = ImageChops.lighter(ImageChops.lighter(red, green), blue).point(
            lambda pixel: 255 if pixel > pixel_threshold else 0
        )
        # A taller/wider screenshot must be reported even when the added
        # pixels happen to match the padding color.
        if left.width != right.width:
            mask.paste(255, (min(left.width, right.width), 0, dimensions[0], dimensions[1]))
        if left.height != right.height:
            mask.paste(255, (0, min(left.height, right.height), dimensions[0], dimensions[1]))
        overlay = Image.new("RGBA", dimensions, (255, 48, 48, 0))
        overlay.putalpha(mask.point(lambda pixel: 165 if pixel else 0))
        Image.alpha_composite(second.convert("RGBA"), overlay).save(dest)
        return mask.histogram()[255], dimensions[0] * dimensions[1]


def check_diff_algorithm(out: Path) -> None:
    """Prove that a one-pixel visual change produces a nonempty red diff."""
    unchanged = Image.new("RGB", (8, 8), (15, 23, 42))
    modified = unchanged.copy()
    modified.putpixel((3, 4), (255, 255, 255))
    before, after = BytesIO(), BytesIO()
    unchanged.save(before, format="PNG")
    modified.save(after, format="PNG")
    result_file = out / "selftest-diff.png"
    changed, total = create_diff(before.getvalue(), after.getvalue(), result_file)
    assert (changed, total) == (1, 64), f"visual diff calculation failed: {changed}/{total}"
    with Image.open(result_file) as visual:
        red, green, blue, _ = visual.getpixel((3, 4))
        assert red > green and red > blue, "changed pixel must be highlighted red"
    # Regression: grayscale conversion previously missed a 50-level blue change.
    blue_changed = unchanged.copy()
    blue_changed.putpixel((3, 4), (15, 23, 92))
    blue_bytes = BytesIO()
    blue_changed.save(blue_bytes, format="PNG")
    changed, _ = create_diff(before.getvalue(), blue_bytes.getvalue(), out / "selftest-blue.png")
    assert changed == 1, "blue-only changes must not be ignored"
    # A single-level channel change must be visible with strict pixel comparison.
    tiny_changed = unchanged.copy()
    tiny_changed.putpixel((3, 4), (15, 23, 43))
    tiny_bytes = BytesIO()
    tiny_changed.save(tiny_bytes, format="PNG")
    changed, _ = create_diff(before.getvalue(), tiny_bytes.getvalue(), out / "selftest-tiny.png")
    assert changed == 1, "one-channel delta of 1 must be detected at threshold 0"
    changed, _ = create_diff(before.getvalue(), tiny_bytes.getvalue(), out / "selftest-threshold.png", 1)
    assert changed == 0, "explicit threshold of 1 should ignore a channel delta of 1"
    # Image dimension changes are meaningful even if the added pixels match background.
    larger = Image.new("RGB", (9, 8), (15, 23, 42))
    large_bytes = BytesIO()
    larger.save(large_bytes, format="PNG")
    changed, _ = create_diff(before.getvalue(), large_bytes.getvalue(), out / "selftest-size.png")
    assert changed >= 8, "screenshot size differences must be detected"
    unchanged_count, _ = create_diff(before.getvalue(), before.getvalue(), out / "selftest-equal.png")
    assert unchanged_count == 0, "identical screenshots must have no differences"


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
        # A missing report or truthy but non-boolean ok must never display success.
        for report, expected in (
            ({"checks": []}, "検証エラー"),
            ({"checks": [{"name": "Profile", "ok": "false"}]}, "1 件 NG"),
            ({"checks": [{"name": "Profile", "ok": 1}]}, "1 件 NG"),
            ({"checks": [None]}, "1 件 NG"),
        ):
            page.evaluate("(r) => { window.__reportOverride = r; }", report)
            page.locator("#verify").click()
            page.wait_for_function(
                "(message) => document.getElementById('output').textContent.includes(message)",
                arg=expected, timeout=10000
            )
            assert "すべて OK" not in page.locator("#output").inner_text()
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


def check_failure_scenarios(browser, url: str) -> None:
    # Wizard apply/plan/verify must remain usable after rejected IPC promises.
    for step, command, message in (
        (3, "run_plan", "プラン失敗"),
        (5, "run_apply", "適用に失敗"),
        (6, "run_verify", "検証に失敗"),
    ):
        context, page = scene(browser, url, "first-run")
        try:
            page.evaluate(
                "(cmd) => { window.__rejections[cmd] = 'simulated network error'; }", command
            )
            page.evaluate("(step) => { setupStep = step; renderStep(); }", step)
            if step == 3:
                page.get_by_role("button", name="プラン実行").click()
            page.get_by_text(message, exact=False).wait_for(timeout=10000)
            page.get_by_role("button", name="再試行").wait_for(timeout=10000)
            assert not page.locator("#setup-step").inner_text().startswith("適用が完了")
        finally:
            context.close()

    # Invalid and empty verification reports must also block wizard completion.
    for report, expected in (
        ({"checks": []}, "検証に失敗しました"),
        ({"checks": [{"name": "Profile", "ok": "false"}]}, "1 件 NG"),
    ):
        context, page = scene(browser, url, "first-run")
        try:
            page.evaluate("""(report) => {
                window.__reportOverride = report;
                setupStep = 6; renderStep();
            }""", report)
            page.get_by_text(expected, exact=False).wait_for(timeout=10000)
            assert page.get_by_role("button", name="完了").count() == 0
        finally:
            context.close()

    # Negative verification reports must block completion.
    context, page = scene(browser, url, "first-run")
    try:
        page.evaluate("""() => {
            window.__reportOverride = { checks: [{ name: 'Profile', ok: false }] };
            setupStep = 6; renderStep();
        }""")
        page.get_by_text("1 件 NG").wait_for()
        assert page.get_by_role("button", name="完了").count() == 0
    finally:
        context.close()


def check_download_and_install_ui(browser, url: str) -> None:
    # No installation is allowed before the explicit confirmation.
    context, page = scene(browser, url, "nix-confirmation")
    try:
        assert "nix_install_escalated" not in page.evaluate(
            "window.__calls.map(call => call.command)"
        )
        page.get_by_role("button", name="導入する").click()
        page.get_by_text("Managed Nix の導入が完了しました。").wait_for(timeout=10000)
        commands = page.evaluate("window.__calls.map(call => call.command)")
        assert commands.index("nix_prepare_plan") < commands.index("nix_install_escalated")
        page.get_by_role("button", name="前提確認へ戻る").wait_for()
    finally:
        context.close()

    for failure_type in ("result", "reject"):
        context, page = scene(browser, url, "nix-confirmation")
        try:
            if failure_type == "result":
                page.evaluate("window.__failures.nix_install_escalated = 'HTTP 500 / download failed'")
            else:
                page.evaluate("window.__rejections.nix_install_escalated = 'connection reset'")
            page.get_by_role("button", name="導入する").click()
            expected_error = (
                "HTTP 500 / download failed" if failure_type == "result"
                else "導入に失敗しました: Error: connection reset"
            )
            page.locator("#setup-step").get_by_text(expected_error, exact=False).wait_for(timeout=10000)
            page.get_by_role("button", name="再試行").wait_for()
            assert "導入が完了しました" not in page.locator("#setup-step").inner_text()
        finally:
            context.close()

    # App updater progress / successful install is UI-only; no real bytes fetched.
    context, page = scene(browser, url, "app-updater")
    try:
        page.on("dialog", lambda d: (
            d.accept() if "更新しますか" in d.message else d.dismiss()
        ))
        page.locator("#app-update-check").click()
        page.get_by_text("signed update installed", exact=False).first.wait_for(timeout=10000)
        commands = page.evaluate("window.__calls.map(call => call.command)")
        assert commands.index("fetch_app_update") < commands.index("install_app_update")
        assert "restart_app" not in commands, "cancelled restart must not be invoked"
    finally:
        context.close()


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--mode", choices=("behavior", "compare"), required=True)
    parser.add_argument("--candidate", type=Path, required=True)
    parser.add_argument("--baseline", type=Path)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--pixel-threshold", type=int, default=0,
                        help="max per-channel delta ignored (0=strict, 0..255)")
    args = parser.parse_args()
    if not 0 <= args.pixel_threshold <= 255:
        parser.error("--pixel-threshold must be between 0 and 255")
    args.out.mkdir(parents=True, exist_ok=True)

    with sync_playwright() as playwright:
        browser = playwright.chromium.launch()
        try:
            with serve(args.candidate.resolve()) as candidate:
                if args.mode == "behavior":
                    check_diff_algorithm(args.out)
                    check_behavior(browser, candidate)
                    check_failure_scenarios(browser, candidate)
                    check_download_and_install_ui(browser, candidate)
                    print("PASS: UI behavior, failure recovery, update display, and cancellation")
                    return
                if not args.baseline:
                    parser.error("--baseline required for comparison")
                with serve(args.baseline.resolve()) as baseline:
                    results = []
                    for viewport_name, viewport in VIEWPORTS.items():
                        for name in SCENARIOS:
                            label = f"{viewport_name}-{name}"
                            before = snapshot(browser, baseline, name, viewport)
                            after = snapshot(browser, candidate, name, viewport)
                            (args.out / f"{label}-before.png").write_bytes(before)
                            (args.out / f"{label}-after.png").write_bytes(after)
                            changed, total = create_diff(
                                before, after, args.out / f"{label}-diff.png",
                                args.pixel_threshold
                            )
                            results.append({
                                "scenario": label, "changed_pixels": changed,
                                "total_pixels": total, "ratio": round(changed / total, 6),
                                "pixel_threshold": args.pixel_threshold
                            })
                            print(f"{label}: {changed}/{total} pixels changed")
                    (args.out / "results.json").write_text(
                        json.dumps(results, ensure_ascii=False, indent=2) + "\n",
                        encoding="utf-8"
                    )
                    lines = [
                        "### SchneeForge UI画像差分（PRベースとの比較）",
                        "",
                        f"RGB各チャンネルの許容差: {args.pixel_threshold}（0=厳密比較）",
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
