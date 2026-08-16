const { invoke } = window.__TAURI__.core;

const $ = (id) => document.getElementById(id);

let setupStep = 0;

function row(key, val) {
  return `<div class="row"><span class="key">${key}</span><span class="val">${val}</span></div>`;
}

function actionBtn(label, onClick) {
  const b = document.createElement("button");
  b.textContent = label;
  b.addEventListener("click", onClick);
  return b;
}

function errorBlock(msg) {
  return `<p class="err">${msg}</p>`;
}

// ---------- ready view ----------
async function refresh() {
  try {
    const s = await invoke("get_status");
    $("host").textContent = s.host;
    $("user").textContent = s.username ?? "-";
    $("nix").textContent = s.tools.nix.available ? "yes" : "no";
    $("homebrew").textContent = s.tools.homebrew.available ? "yes" : "no";
    $("applied").textContent = s.applied_revision ?? "(never)";
    return s;
  } catch (e) {
    $("output").textContent = `get_status エラー: ${e}`;
    return null;
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

// ---------- setup wizard ----------
function showSetup() {
  setupStep = 0;
  $("ready-view").classList.add("hidden");
  $("setup-view").classList.remove("hidden");
  renderStep();
}

function showReady() {
  $("setup-view").classList.add("hidden");
  $("ready-view").classList.remove("hidden");
  refresh();
}

function next() {
  setupStep++;
  renderStep();
}

function renderStep() {
  const box = $("setup-step");
  const actions = $("setup-actions");
  box.innerHTML = "";
  actions.innerHTML = "";
  switch (setupStep) {
    case 0: stepPrereq(box, actions); break;
    case 1: stepRepo(box, actions); break;
    case 2: stepUser(box, actions); break;
    case 3: stepPlan(box, actions); break;
    case 4: stepConfirm(box, actions); break;
    case 5: stepApply(box, actions); break;
    case 6: stepVerify(box, actions); break;
  }
}

async function stepPrereq(box, actions) {
  box.textContent = "前提条件を確認しています...";
  let status = null;
  try {
    status = await invoke("get_status");
  } catch (_) {}
  let pre;
  try {
    pre = await invoke("run_preflight");
  } catch (e) {
    box.innerHTML = errorBlock(`前提条件の確認に失敗しました: ${e}`);
    actions.innerHTML = "";
    actions.appendChild(actionBtn("再確認", () => renderStep()));
    return;
  }
  const nixOk = pre.nix_installed;
  const gitOk = pre.git_installed;
  const flakesOk = pre.flakes_enabled;
  // NixStatus 分類 (Missing / Healthy / Degraded / Broken)。nix_health が
  // binary 単位の検知なのに対し、marker / receipt / ownership を組合せた
  // install 状態。get_status 失敗時は未取得として表示しない
  const nixStatus = status && status.nix_status ? status.nix_status.status : null;
  const nixNextAction = status && status.nix_status ? status.nix_status.next_action : null;
  box.innerHTML =
    row("host", status ? status.host : "-") +
    row("platform", status ? `${status.platform}/${status.architecture}` : "-") +
    row("Nix", nixOk ? "OK" : "NG") +
    row("Nix status", nixStatus ?? "-") +
    row("Git", gitOk ? "OK" : "NG") +
    row("flakes", flakesOk ? "OK" : "NG");
  if (nixOk && gitOk && flakesOk) {
    if (nixStatus === "Degraded" || nixStatus === "Broken") {
      // wizard は install 案内だけで済ませず、修復 / 手動調査の案内を出す
      box.innerHTML += errorBlock(
        `Nix の状態は ${nixStatus} です: ${nixNextAction}`
      );
    }
    actions.appendChild(actionBtn("次へ", next));
  } else if (!nixOk) {
    // Managed Nix を SchneeForge 自身で導入する案内 (ownership 管理下)。
    // legacy な curl | sh は ownership record が残らず uninstall 対称性が
    // 崩れるため案内しない
    box.innerHTML += errorBlock(
      "Nix が未導入です。SchneeForge の Managed Nix で導入してください:<br>" +
        "<code>sudo schneeforge nix install</code>" +
        (nixStatus && nixStatus !== "Missing"
          ? `<br>現在の状態は ${nixStatus} です — まず修復が必要です: ${nixNextAction}`
          : "")
    );
    actions.appendChild(actionBtn("再確認", () => renderStep()));
  } else {
    box.innerHTML += errorBlock(
      !gitOk
        ? "Git が見つかりません。Xcode Command Line Tools をインストールしてください:<br>" +
            "<code>xcode-select --install</code>"
        : "flakes が無効です。Nix 設定に追加してください:<br>" +
            "<code>experimental-features = nix-command flakes</code>"
    );
    actions.appendChild(actionBtn("再確認", () => renderStep()));
  }
}

function stepRepo(box, actions) {
  const DEFAULT_REPO_URL = "https://github.com/Lamy210/nix_setting.git";
  const escapedDefault = DEFAULT_REPO_URL
    .replace(/&/g, "&amp;")
    .replace(/"/g, "&quot;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
  box.innerHTML =
    '<p>repository の URL を確認してください。空のままなら既定 (upstream) を使います。</p>' +
    '<input id="repo-url" type="text" value="' + escapedDefault + '" />';
  actions.appendChild(
    actionBtn("clone", async () => {
      // 空 (または既定値のまま) なら backend の既定解決に任せる。
      // fork を使うユーザーはここを自分の URL に書き換える
      const url = $("repo-url").value.trim();
      box.textContent = url ? `cloning ${url}...` : "cloning (default repository)...";
      const r = await invoke("run_clone_repo", { url });
      if (r.success) {
        next();
      } else {
        box.innerHTML = errorBlock(`clone 失敗: ${r.output}`);
        actions.innerHTML = "";
        actions.appendChild(actionBtn("再試行", () => renderStep()));
      }
    })
  );
}

async function stepUser(box, actions) {
  let status = null;
  try {
    status = await invoke("get_status");
  } catch (_) {}
  const detected = (status && status.system_user) || "";
  const escaped = detected
    .replace(/&/g, "&amp;")
    .replace(/"/g, "&quot;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
  box.innerHTML =
    '<p>ユーザー名を確認してください。</p>' +
    `<input id="username" type="text" value="${escaped}" />`;
  actions.appendChild(
    actionBtn("config.toml を生成", async () => {
      const username = $("username").value.trim();
      if (!username) {
        box.innerHTML += errorBlock("ユーザー名を入力してください。");
        return;
      }
      box.textContent = "config.toml を生成中...";
      const r = await invoke("run_generate_config", { username });
      if (r.success) {
        next();
      } else {
        box.innerHTML = errorBlock(`生成失敗: ${r.output}`);
        actions.innerHTML = "";
        actions.appendChild(actionBtn("再試行", () => renderStep()));
      }
    })
  );
}

function stepPlan(box, actions) {
  box.textContent = "dry-run でプランを確認できます。";
  actions.appendChild(
    actionBtn("プラン実行", async () => {
      box.textContent = "プラン実行中...";
      const r = await invoke("run_plan");
      if (r.success) {
        box.innerHTML = "<p>プラン結果:</p><pre>" + (r.output || "(空)") + "</pre>";
        actions.innerHTML = "";
        actions.appendChild(actionBtn("次へ", next));
      } else {
        box.innerHTML = errorBlock(`プラン失敗: ${r.output}`);
        actions.innerHTML = "";
        actions.appendChild(actionBtn("再試行", () => renderStep()));
      }
    })
  );
}

function stepConfirm(box, actions) {
  box.innerHTML =
    "<p>適用の準備ができました。適用はマシンの設定を変更します。</p>" +
    "<p>続行しますか？</p>";
  actions.appendChild(actionBtn("適用する", next));
  actions.appendChild(actionBtn("キャンセル", showReady));
}

function stepApply(box, actions) {
  box.textContent = "適用中... (nix ビルドには数分かかります)";
  invoke("run_apply").then((r) => {
    if (r.success) {
      box.innerHTML = "<p>適用が完了しました。</p>";
      actions.innerHTML = "";
      actions.appendChild(actionBtn("検証へ", next));
    } else {
      box.innerHTML = errorBlock(`適用に失敗しました: ${r.output}`);
      actions.innerHTML = "";
      actions.appendChild(actionBtn("再試行", () => renderStep()));
    }
  });
}

async function stepVerify(box, actions) {
  box.textContent = "検証中...";
  const report = await invoke("run_verify");
  const rows = report.checks.map((c) => row(c.name, c.ok ? "OK" : "NG")).join("");
  const failed = report.checks.filter((c) => !c.ok).length;
  box.innerHTML = "<p>検証結果:</p>" + rows + `<p>${failed === 0 ? "すべて OK" : `${failed} 件 NG`}</p>`;
  actions.innerHTML = "";
  if (failed === 0) {
    actions.appendChild(actionBtn("完了", showReady));
  } else {
    actions.appendChild(actionBtn("再試行", () => renderStep()));
  }
}

// ---------- boot ----------
async function boot() {
  const s = await refresh();
  if (s && !s.repo_exists) {
    showSetup();
  } else {
    showReady();
  }
}

$("refresh").addEventListener("click", refresh);
$("scan").addEventListener("click", () => run(() => invoke("run_scan"), "scan", "スキャン"));
$("plan").addEventListener("click", () => run(() => invoke("run_plan"), "plan", "プラン"));
$("apply").addEventListener("click", () => run(() => invoke("run_apply"), "apply", "適用"));
$("rollback").addEventListener("click", () => run(() => invoke("run_rollback"), "rollback", "ロールバック"));
$("upgrade").addEventListener("click", () => run(() => invoke("run_upgrade"), "upgrade", "アップグレード"));
$("verify").addEventListener("click", verify);

async function verify() {
  const btn = $("verify");
  if (!btn) {
    $("output").textContent = "検証ボタンが見つかりません";
    return;
  }
  btn.disabled = true;
  $("output").textContent = "検証中...";
  try {
    const report = await invoke("run_verify");
    const rows = report.checks
      .map((c) => `${c.ok ? "✅" : "❌"} ${c.name}`)
      .join("\n");
    const failed = report.checks.filter((c) => !c.ok).length;
    $("output").textContent = `検証結果:\n${rows}\n\n${
      failed === 0 ? "すべて OK" : `${failed} 件 NG`
    }`;
  } catch (e) {
    $("output").textContent = `検証エラー: ${e}`;
  } finally {
    btn.disabled = false;
  }
}

boot();
