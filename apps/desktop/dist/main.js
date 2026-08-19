const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

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
    $("profile").textContent = s.profile ?? "-";
    $("nix").textContent = s.tools.nix.available ? "yes" : "no";
    $("homebrew").textContent = s.tools.homebrew.available ? "yes" : "no";
    $("applied").textContent = s.applied_revision ?? "(never)";
    return s;
  } catch (e) {
    $("output").textContent = `get_status エラー: ${e}`;
    return null;
  }
}

// ---------- dashboard (v2 §28: Installed / Available) ----------
async function refreshDashboard() {
  try {
    const d = await invoke("get_dashboard");
    $("dash-installed").textContent = `v${d.installed.version}`;
    $("dash-profile").textContent = d.installed.profile ?? "-";
    $("dash-channel").textContent = d.installed.channel;
    if (d.available) {
      $("dash-available").textContent =
        `v${d.available.version} (${d.available.channel}) [${d.available.systems.join(", ")}]`;
    } else {
      $("dash-available").textContent =
        `取得できません (${d.available_error ?? "理由不明"})`;
    }
    const note = $("dash-update");
    note.textContent = d.update_available
      ? `新しいリリース v${d.available.version} があります — GitHub Releases / install.sh で更新できます`
      : "利用中の版は最新です";
    note.classList.toggle("update-available", d.update_available);
  } catch (e) {
    $("dash-installed").textContent = "-";
    $("dash-channel").textContent = "-";
    $("dash-available").textContent = `取得エラー: ${e}`;
  }
  await refreshProfileSelect();
}

// ---------- profile 切替 (v2 §17 follow-up) ----------
function setProfileNote(msg) {
  $("profile-note").textContent = msg;
}

async function refreshProfileSelect() {
  const sel = $("dash-profile-select");
  try {
    const p = await invoke("get_profiles");
    sel.innerHTML = "";
    for (const name of p.available) {
      const opt = document.createElement("option");
      opt.value = name;
      opt.textContent = name === p.default ? `${name} (既定)` : name;
      sel.appendChild(opt);
    }
    sel.value = p.selected ?? p.default ?? "";
    $("profile-set").disabled = false;
    $("profile-clear").disabled = !p.selected;
    setProfileNote(p.selected ? "選択は次回の「適用」から反映されます" : "");
  } catch (e) {
    sel.innerHTML = "";
    $("profile-set").disabled = true;
    $("profile-clear").disabled = true;
    setProfileNote(`切替は使用できません (${e})`);
  }
}

async function switchProfile() {
  const name = $("dash-profile-select").value;
  if (!name) return;
  try {
    const r = await invoke("set_profile", { name });
    if (r.success) {
      setProfileNote(`profile を ${name} へ変更しました — 次回の「適用」から反映されます`);
      await refresh();
      await refreshDashboard();
    } else {
      setProfileNote(`error: ${r.output}`);
    }
  } catch (e) {
    setProfileNote(`error: ${e}`);
  }
}

async function clearProfile() {
  try {
    const r = await invoke("clear_profile");
    if (r.success) {
      setProfileNote("manifest の既定へ戻しました — 次回の「適用」から反映されます");
      await refresh();
      await refreshDashboard();
    } else {
      setProfileNote(`error: ${r.output}`);
    }
  } catch (e) {
    setProfileNote(`error: ${e}`);
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
  refreshDashboard();
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
      // wizard は install 案内だけで済ませず、修復 / 手動調査の案内を出す。
      // repair は非破壊の状態を含むため確認なしで実行できる (破壊操作は
      // stale ownership record の削除のみで、内容は CLI 側が案内する)
      box.innerHTML += errorBlock(
        `Nix の状態は ${nixStatus} です: ${nixNextAction}`
      );
      actions.appendChild(
        actionBtn("修復を試みる", () => stepNixRepair(box, actions))
      );
    }
    actions.appendChild(actionBtn("次へ", next));
  } else if (!nixOk) {
    // Managed Nix を SchneeForge 自身で install する (ownership 管理下)。
    // legacy な curl | sh は ownership record が残らず uninstall 対称性が
    // 崩れるため案内しない
    const statusNote =
      nixStatus && nixStatus !== "Missing"
        ? `<br>現在の状態は ${nixStatus} です — まず修復が必要です: ${nixNextAction}`
        : "";
    if (nixStatus && nixStatus !== "Missing") {
      // Missing 以外 (Broken 等) は GUI install を offering しない
      box.innerHTML += errorBlock(
        "Nix が未導入です。" + statusNote
      );
      actions.appendChild(actionBtn("再確認", () => renderStep()));
    } else {
      box.innerHTML += errorBlock(
        "Nix が未導入です。SchneeForge の Managed Nix で導入できます。" + statusNote
      );
      // Managed Nix 導入は repo の bootstrap-manifest.toml を必要とする。
      // 未 clone のまま install しても backend が fail-closed で拒否するため、
      // frontend で先に repo step へ誘導する (D: fresh machine での空振り防止)
      if (status && !status.repo_exists) {
        box.innerHTML +=
          '<p class="note">まず repository の clone が必要です。次へ進んでください。</p>';
        actions.appendChild(actionBtn("次へ (repository 設定)", next));
      } else {
        actions.appendChild(actionBtn("SchneeForge で導入", () => stepNixInstall(box, actions)));
        // escalation helper が使えない環境向けの fallback 案内
        box.innerHTML +=
          '<p class="note">ターミナルから <code>sudo schneeforge nix install</code> でも導入できます</p>';
      }
      actions.appendChild(actionBtn("再確認", () => renderStep()));
    }
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

// Managed Nix install flow (D8 の GUI 版: plan preview → 最終確認 → install)
async function stepNixInstall(box, actions) {
  box.textContent = "Managed Nix の plan を生成しています... (download / verify を含みます)";
  actions.innerHTML = "";
  let plan;
  try {
    plan = await invoke("nix_prepare_plan");
  } catch (e) {
    box.innerHTML = errorBlock(`plan 生成に失敗しました: ${e}`) +
      cliFallbackNote();
    actions.appendChild(actionBtn("再試行", () => stepNixInstall(box, actions)));
    return;
  }
  if (!plan.success) {
    box.innerHTML = errorBlock(`plan 生成に失敗しました: ${plan.output}`) +
      cliFallbackNote();
    actions.appendChild(actionBtn("再試行", () => stepNixInstall(box, actions)));
    return;
  }

  // detailed plan 表示 (D8: この確認が GUI 側の確認 gate)
  box.innerHTML =
    "<p>以下の内容で Nix を導入します (detailed plan):</p>" +
    "<pre>" + escapeHtml(plan.output) + "</pre>" +
    "<p>続行しますか? 管理者権限の確認が表示される場合があります。</p>";
  actions.innerHTML = "";
  actions.appendChild(
    actionBtn("導入する", async () => {
      box.innerHTML =
        '<p id="nix-install-phase">Managed Nix を導入しています...</p>' +
        '<pre id="nix-install-log" class="nix-log"></pre>';
      actions.innerHTML = "";
      // backend の stderr JSON Lines を event で受け、phase + 直近 log を随時表示
      let unlisten = null;
      try {
        unlisten = await listen("nix-install-progress", (e) => {
          const phaseEl = document.getElementById("nix-install-phase");
          const logEl = document.getElementById("nix-install-log");
          const { phase, message } = e.payload || {};
          if (phaseEl && phase) {
            phaseEl.textContent = `Managed Nix を導入しています... (${phase})`;
          }
          if (logEl && message) {
            logEl.textContent = tailLines(logEl.textContent + message + "\n", 10);
          }
        });
      } catch (_) {
        // event listener が取得できない環境でも install 自体は続行 (表示は完了後一括)
      }
      let r;
      try {
        r = await invoke("nix_install_escalated");
      } catch (e) {
        if (unlisten) unlisten();
        box.innerHTML = errorBlock(`導入に失敗しました: ${e}`) + cliFallbackNote();
        actions.appendChild(actionBtn("再試行", () => stepNixInstall(box, actions)));
        return;
      }
      if (unlisten) unlisten();
      if (r.success) {
        box.innerHTML =
          "<p>Managed Nix の導入が完了しました。</p><pre>" +
          escapeHtml(tailLines(r.output, 30)) +
          "</pre>";
        actions.innerHTML = "";
        actions.appendChild(actionBtn("前提確認へ戻る", () => renderStep()));
      } else {
        box.innerHTML = errorBlock(r.output) + cliFallbackNote();
        actions.innerHTML = "";
        actions.appendChild(actionBtn("再試行", () => stepNixInstall(box, actions)));
      }
    })
  );
  actions.appendChild(actionBtn("キャンセル", () => renderStep()));
}

function cliFallbackNote() {
  return '<p class="note">ターミナルから <code>sudo schneeforge nix install</code> でも導入できます</p>';
}

// Nix repair flow: NixStatus 分類に基づく修復 (Broken は stale record 削除、
// Degraded は uninstall / 手動手順の案内)。実行は昇格された CLI sidecar。
async function stepNixRepair(box, actions) {
  box.textContent = "Nix の状態を修復しています... (schneeforge nix repair)";
  actions.innerHTML = "";
  let r;
  try {
    r = await invoke("nix_repair_escalated");
  } catch (e) {
    box.innerHTML = errorBlock(`修復の実行に失敗しました: ${e}`) +
      '<p class="note">ターミナルから <code>sudo schneeforge nix repair</code> でも実行できます</p>';
    actions.appendChild(actionBtn("再試行", () => stepNixRepair(box, actions)));
    actions.appendChild(actionBtn("前提確認へ戻る", () => renderStep()));
    return;
  }
  if (r.success) {
    box.innerHTML =
      "<p>修復コマンドが完了しました。結果:</p><pre>" +
      escapeHtml(tailLines(r.output, 30)) +
      "</pre>";
  } else {
    box.innerHTML = errorBlock(r.output) +
      '<p class="note">ターミナルから <code>sudo schneeforge nix repair</code> でも実行できます</p>';
  }
  actions.innerHTML = "";
  actions.appendChild(actionBtn("前提確認へ戻る (状態を再確認)", () => renderStep()));
}

function escapeHtml(s) {
  return String(s)
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

function tailLines(s, n) {
  const lines = String(s).split("\n");
  return lines.slice(Math.max(0, lines.length - n)).join("\n");
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
  box.textContent = "machine 情報を検出中...";
  let r = null;
  try {
    r = await invoke("machine_facts");
  } catch (e) {
    r = { success: false, output: String(e) };
  }
  if (!r.success) {
    box.innerHTML = errorBlock(
      `machine 情報を検出できませんでした: ${r.output}`
    );
    actions.innerHTML = "";
    actions.appendChild(actionBtn("再検出", () => renderStep()));
    return;
  }
  const display = r.output
    .replace(/&/g, "&amp;")
    .replace(/"/g, "&quot;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
  // v2: machine 情報は自動検出のみ。repo へ書き込まない (machine input は
  // apply 時に state dir へ生成される)
  box.innerHTML =
    "<p>machine 情報を検出しました:</p>" +
    `<pre>${display}</pre>` +
    "<p>この内容で proceed します (repo は書き換えられません)。</p>";
  actions.appendChild(actionBtn("次へ", next));
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

$("refresh").addEventListener("click", () => {
  refresh();
  refreshDashboard();
});
$("scan").addEventListener("click", () => run(() => invoke("run_scan"), "scan", "スキャン"));
$("plan").addEventListener("click", () => run(() => invoke("run_plan"), "plan", "プラン"));
$("apply").addEventListener("click", () => run(() => invoke("run_apply"), "apply", "適用"));
$("rollback").addEventListener("click", () => run(() => invoke("run_rollback"), "rollback", "ロールバック"));
$("upgrade").addEventListener("click", () => run(() => invoke("run_upgrade"), "upgrade", "アップグレード"));
$("verify").addEventListener("click", verify);
$("nix-uninstall").addEventListener("click", nixUninstall);
$("profile-set").addEventListener("click", switchProfile);
$("profile-clear").addEventListener("click", clearProfile);

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

// Managed Nix の削除 (破壊的操作)。確認 dialog を経てのみ実行する。
// ownership record が無い環境では CLI 側が fail-closed で拒否する
// (--force は GUI からは渡さない)。
async function nixUninstall() {
  const ok = confirm(
    "SchneeForge の管理する Nix (/nix 配下) を削除します。\n" +
      "適用済みの環境も失われます。本当に削除しますか?"
  );
  if (!ok) return;
  await run(
    () => invoke("nix_uninstall_escalated"),
    "nix-uninstall",
    "Nix 削除"
  );
}

boot();
