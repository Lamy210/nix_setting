const { invoke } = window.__TAURI__.core;

const $ = (id) => document.getElementById(id);

async function refresh() {
  try {
    const s = await invoke("get_status");
    $("host").textContent = s.host;
    $("user").textContent = s.username ?? "-";
    $("nix").textContent = s.tools.nix.available ? "yes" : "no";
    $("homebrew").textContent = s.tools.homebrew.available ? "yes" : "no";
    $("applied").textContent = s.applied_revision ?? "(never)";
  } catch (e) {
    $("output").textContent = `get_status エラー: ${e}`;
  }
}

function setRunning(label, active) {
  const bar = $("running");
  bar.classList.toggle("active", active);
  if (active) {
    $("running-text").textContent = `${label} を実行中... (nix ビルドには数分かかります)`;
  }
}

async function run(fn, buttonId, label) {
  const btn = $(buttonId);
  if (!btn) {
    $("output").textContent = `ボタンが見つかりません: ${buttonId}`;
    return;
  }
  btn.disabled = true;
  setRunning(label, true);
  $("output").textContent = `${label} を実行中...`;
  try {
    const r = await fn();
    if (r.success) {
      $("output").textContent = r.output || "(完了)";
    } else {
      $("output").textContent = `${label} 失敗: ${r.output}`;
    }
  } catch (e) {
    $("output").textContent = `${label} エラー: ${e}`;
  } finally {
    btn.disabled = false;
    setRunning(label, false);
    refresh();
  }
}

$("refresh").addEventListener("click", refresh);
$("scan").addEventListener("click", () => run(() => invoke("run_scan"), "scan", "スキャン"));
$("apply").addEventListener("click", () => run(() => invoke("run_apply"), "apply", "適用"));
$("rollback").addEventListener("click", () => run(() => invoke("run_rollback"), "rollback", "ロールバック"));
$("upgrade").addEventListener("click", () => run(() => invoke("run_upgrade"), "upgrade", "アップグレード"));

refresh();
