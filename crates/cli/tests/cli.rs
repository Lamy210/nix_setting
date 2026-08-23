use assert_cmd::Command;
use predicates::prelude::*;

/// このテストファイル内の "nix 必須" テストは環境に nix が無い場合は skip する。
/// binary 側の ToolResolver と同一の解決 (PATH 以外に Nix profile 群 / Homebrew も
/// 探索する) で判定する。PATH check だけだと「PATH に nix 無いが /nix はある」環境
/// (例: nix build の checkPhase sandbox) で guard を抜けて assertion が崩れる
fn nix_available() -> bool {
    schneeforge_core::ToolInventory::discover().nix.is_some()
}

#[test]
fn help_lists_commands() {
    let mut cmd = Command::cargo_bin("schneeforge").unwrap();
    cmd.arg("--help").assert().success().stdout(
        predicate::str::contains("Declarative Developer Workstation Manager")
            .and(predicate::str::contains("doctor"))
            .and(predicate::str::contains("apply"))
            .and(predicate::str::contains("rollback")),
    );
}

#[test]
fn version_prints_semver() {
    let mut cmd = Command::cargo_bin("schneeforge").unwrap();
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"schneeforge \d+\.\d+\.\d+").unwrap());
}

#[test]
fn doctor_prints_system_section() {
    // doctor の基本出力 (system/host)。Nix 有無の断言は core hermetic test が担う
    let state = state_dir("doctor-system");
    let mut cmd = Command::cargo_bin("schneeforge").unwrap();
    cmd.arg("doctor")
        .env("XDG_STATE_HOME", &state)
        .assert()
        .success()
        .stdout(predicate::str::contains("[system]").and(predicate::str::contains("host")));
}

/// status は ToolInventory 解決を必要としないので nix 無しでも動く
#[test]
fn status_runs_without_nix() {
    let state = state_dir("status-no-nix");
    let mut cmd = Command::cargo_bin("schneeforge").unwrap();
    cmd.arg("status")
        .env("XDG_STATE_HOME", &state)
        .assert()
        .success()
        .stdout(predicate::str::contains("host:"));
}

#[test]
fn scan_runs() {
    if !nix_available() {
        eprintln!("skipping: nix not installed");
        return;
    }
    let mut cmd = Command::cargo_bin("schneeforge").unwrap();
    cmd.arg("scan")
        .env("XDG_STATE_HOME", state_dir("scan"))
        .assert()
        .success()
        .stdout(predicate::str::contains("OS:").and(predicate::str::contains("arch:")));
}

#[test]
fn status_respects_repo_flag() {
    let mut cmd = Command::cargo_bin("schneeforge").unwrap();
    cmd.arg("--repo")
        .arg("/nonexistent")
        .arg("status")
        .env("XDG_STATE_HOME", state_dir("status-repo-flag"))
        .assert()
        .success();
}

#[test]
fn plan_shows_target_with_repo() {
    if !nix_available() {
        eprintln!("skipping: nix not installed");
        return;
    }
    // --repo を明示すると target 表示までは進む (nix build は未実行)
    let mut cmd = Command::cargo_bin("schneeforge").unwrap();
    cmd.arg("--repo")
        .arg(env!("CARGO_MANIFEST_DIR"))
        .arg("plan")
        .env("XDG_STATE_HOME", state_dir("plan-target"))
        .assert()
        .stdout(predicate::str::contains("target:"));
}

/// uninstall は info 系なので ToolInventory 無しで動く
#[test]
fn uninstall_runs_without_nix() {
    let mut cmd = Command::cargo_bin("schneeforge").unwrap();
    cmd.arg("uninstall")
        .assert()
        .success()
        .stdout(predicate::str::contains("=== uninstall ==="));
}

/// doctor は Fresh install 環境でもクラッシュしないことの実機検証。
/// 「Nix 無し = installed: no と表示する」の決定論的な検証は core 側の
/// `nix_health_returns_not_installed_when_unresolved` (ToolInventory を
/// injection した hermetic test) が担う。CLI integration 側で環境の Nix
/// 有無を前提にした assertion を書くと、nix build の checkPhase sandbox
/// (PATH に nix 無いが /nix は読める) で壊れるため行わない
#[test]
fn doctor_runs() {
    let state = state_dir("doctor-runs");
    let mut cmd = Command::cargo_bin("schneeforge").unwrap();
    cmd.arg("doctor")
        .env("XDG_STATE_HOME", &state)
        .assert()
        .success()
        .stdout(predicate::str::contains("[system]").and(predicate::str::contains("host")));
}

/// `schneeforge nix` サブコマンド一覧が help へ出る
#[test]
fn nix_subcommand_help_lists_actions() {
    let mut cmd = Command::cargo_bin("schneeforge").unwrap();
    cmd.arg("nix").arg("--help").assert().success().stdout(
        predicate::str::contains("install")
            .and(predicate::str::contains("doctor"))
            .and(predicate::str::contains("uninstall")),
    );
}

/// `schneeforge nix doctor` は Nix/receipt 無しでも動き、receipt not found を表示する (D7)
#[test]
fn nix_doctor_runs_without_receipt() {
    let state = state_dir("nix-doctor");
    let mut cmd = Command::cargo_bin("schneeforge").unwrap();
    cmd.arg("nix")
        .arg("doctor")
        .env("XDG_STATE_HOME", &state)
        .assert()
        .success()
        .stdout(
            predicate::str::contains("=== schneeforge nix doctor ===")
                .and(predicate::str::contains("[environment]"))
                .and(predicate::str::contains("[receipt]")),
        );
}

/// `schneeforge nix install --dry-run` は preflight を表示して終了する (D8)
#[test]
fn nix_install_dry_run_shows_preflight() {
    // bootstrap-manifest.toml は workspace root にある。crates/cli から見て ../../
    let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut cmd = Command::cargo_bin("schneeforge").unwrap();
    cmd.arg("--repo")
        .arg(repo_root.canonicalize().unwrap())
        .arg("nix")
        .arg("install")
        .arg("--dry-run")
        .env("XDG_STATE_HOME", state_dir("install-dry-run"))
        .assert()
        .success()
        .stderr(
            predicate::str::contains("=== Managed Nix install ===")
                .and(predicate::str::contains("[dry-run]")),
        );
}

/// `schneeforge nix install --yes` で最終確認 skip が parse される (D8 automation mode)
#[test]
fn nix_install_parses_yes_flag() {
    let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut cmd = Command::cargo_bin("schneeforge").unwrap();
    cmd.arg("--repo")
        .arg(repo_root.canonicalize().unwrap())
        .arg("nix")
        .arg("install")
        .arg("--dry-run")
        .arg("--yes")
        .env("XDG_STATE_HOME", state_dir("install-yes"))
        .assert()
        .success()
        .stderr(predicate::str::contains("[dry-run]"));
}

/// `schneeforge nix install` (root 以外) は preflight 後に root 再実行を促して終了する (D4)
#[test]
fn nix_install_without_root_prompts_sudo() {
    // CI / テスト環境では通常 root ではない。root の場合はこのテストの意味が無いので skip。
    if unsafe { libc::geteuid() } == 0 {
        eprintln!("skipping: running as root");
        return;
    }
    let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut cmd = Command::cargo_bin("schneeforge").unwrap();
    cmd.arg("--repo")
        .arg(repo_root.canonicalize().unwrap())
        .arg("nix")
        .arg("install")
        .env("XDG_STATE_HOME", state_dir("install-no-root"))
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("root 権限が必要です")
                .or(predicate::str::contains("existing Nix detected"))
                .or(predicate::str::contains("unsupported platform/arch")),
        );
}

/// `schneeforge nix uninstall` は receipt が無ければ ReceiptNotFound 相当の message で終了する (D6)
#[test]
fn nix_uninstall_without_receipt_errors() {
    let mut cmd = Command::cargo_bin("schneeforge").unwrap();
    cmd.arg("nix")
        .arg("uninstall")
        .arg("--receipt")
        .arg("/tmp/__definitely_no_receipt__.json")
        .assert()
        .failure()
        .stderr(predicate::str::contains("receipt not found"));
}

/// v2 P1: update / source subcommand が定義されている (task 5.1 / 5.2)
#[test]
fn help_lists_update_and_source_commands() {
    let mut cmd = Command::cargo_bin("schneeforge").unwrap();
    cmd.arg("--help").assert().success().stdout(
        predicate::str::contains("update")
            .and(predicate::str::contains("source"))
            .and(predicate::str::contains("deps update")),
    );
}

/// v2 P1: `source status` は kind / ref を表示する。repo (git 管理外) は local
#[test]
fn source_status_reports_local_for_non_git_repo() {
    let dir = std::env::temp_dir().join(format!("sf-cli-local-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut cmd = Command::cargo_bin("schneeforge").unwrap();
    cmd.arg("--repo")
        .arg(&dir)
        .arg("source")
        .arg("status")
        .env("XDG_STATE_HOME", &dir)
        .assert()
        .success()
        .stdout(predicate::str::contains("kind:").and(predicate::str::contains("local")));
    let _ = std::fs::remove_dir_all(&dir);
}

/// v2 P1: `update` は Local source で no-op 案内を出して成功する
#[test]
fn update_on_local_source_is_noop() {
    let dir = std::env::temp_dir().join(format!("sf-cli-update-local-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut cmd = Command::cargo_bin("schneeforge").unwrap();
    cmd.arg("--repo")
        .arg(&dir)
        .arg("update")
        .env("XDG_STATE_HOME", &dir)
        .assert()
        .success()
        .stdout(predicate::str::contains("local"));
    let _ = std::fs::remove_dir_all(&dir);
}

/// v2 P1: 旧 sync command は deprecation note を出す (task 5.3)
#[test]
fn sync_prints_deprecation_note() {
    let dir = std::env::temp_dir().join(format!("sf-cli-sync-dep-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut cmd = Command::cargo_bin("schneeforge").unwrap();
    // Local source では note を出した上で pinned no-op note で成功する
    cmd.arg("--repo")
        .arg(&dir)
        .arg("sync")
        .env("XDG_STATE_HOME", &dir)
        .assert()
        .stdout(predicate::str::contains("deprecated"));
    let _ = std::fs::remove_dir_all(&dir);
}

/// v2 §17: profile list は manifest の available と default を表示する
#[test]
fn profile_list_shows_manifest_profiles() {
    let dir = std::env::temp_dir().join(format!("sf-cli-profile-list-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("schneeforge.toml"),
        "schema = 1\n[profiles]\ndefault = \"developer\"\navailable = [\"minimal\", \"developer\"]\n",
    )
    .unwrap();
    let mut cmd = Command::cargo_bin("schneeforge").unwrap();
    cmd.arg("--repo")
        .arg(&dir)
        .arg("profile")
        .arg("list")
        .env("XDG_STATE_HOME", &dir)
        .assert()
        .success()
        .stdout(
            predicate::str::contains("minimal")
                .and(predicate::str::contains("developer"))
                .and(predicate::str::contains("(default)")),
        );
    let _ = std::fs::remove_dir_all(&dir);
}

/// v2 §17: profile set は manifest available 内なら state へ保存する
#[test]
fn profile_set_saves_selection_to_state() {
    let dir = std::env::temp_dir().join(format!("sf-cli-profile-set-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("schneeforge.toml"),
        "schema = 1\n[profiles]\ndefault = \"developer\"\navailable = [\"minimal\", \"developer\"]\n",
    )
    .unwrap();
    let mut cmd = Command::cargo_bin("schneeforge").unwrap();
    cmd.arg("--repo")
        .arg(&dir)
        .arg("profile")
        .arg("set")
        .arg("minimal")
        .env("XDG_STATE_HOME", &dir)
        .assert()
        .success()
        .stdout(predicate::str::contains("minimal"));

    // state.json に保存されている
    let state = std::fs::read_to_string(dir.join("schneeforge/state.json")).unwrap();
    assert!(state.contains("\"profile\": \"minimal\""), "state: {state}");

    let _ = std::fs::remove_dir_all(&dir);
}

/// v2 §17: profile set は available 外の name を fail-closed で拒否
#[test]
fn profile_set_rejects_unknown_profile() {
    let dir = std::env::temp_dir().join(format!("sf-cli-profile-bad-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("schneeforge.toml"),
        "schema = 1\n[profiles]\ndefault = \"developer\"\navailable = [\"minimal\", \"developer\"]\n",
    )
    .unwrap();
    let mut cmd = Command::cargo_bin("schneeforge").unwrap();
    cmd.arg("--repo")
        .arg(&dir)
        .arg("profile")
        .arg("set")
        .arg("unknown-profile")
        .env("XDG_STATE_HOME", &dir)
        .assert()
        .failure()
        .stderr(predicate::str::contains("not in manifest"));

    // state は作られない
    assert!(!dir.join("schneeforge/state.json").exists());
    let _ = std::fs::remove_dir_all(&dir);
}

/// v2 §17: profile show は解決結果と出典を表示する (未選択なら manifest default)
#[test]
fn profile_show_reports_resolved_profile() {
    let dir = std::env::temp_dir().join(format!("sf-cli-profile-show-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("schneeforge.toml"),
        "schema = 1\n[profiles]\ndefault = \"developer\"\navailable = [\"minimal\", \"developer\"]\n",
    )
    .unwrap();
    let mut cmd = Command::cargo_bin("schneeforge").unwrap();
    cmd.arg("--repo")
        .arg(&dir)
        .arg("profile")
        .arg("show")
        .env("XDG_STATE_HOME", &dir)
        .assert()
        .success()
        .stdout(
            predicate::str::contains("developer").and(predicate::str::contains("manifest default")),
        );
    let _ = std::fs::remove_dir_all(&dir);
}

/// v2 §27: 存在しない tag の metadata は fail-closed に error。
/// network 依存は error path のみ (成功 path の fetch は release 前提のため CI では検証しない)
#[test]
fn source_metadata_unknown_tag_fails_closed() {
    let mut cmd = Command::cargo_bin("schneeforge").unwrap();
    cmd.arg("source")
        .arg("metadata")
        .arg("v0.0.0-does-not-exist.999999")
        .assert()
        .failure()
        .stderr(predicate::str::contains("error:"));
}

/// v2 §27: v prefix 無しの tag は asset 取得前に検証 error になる
#[test]
fn source_metadata_rejects_tag_without_v_prefix() {
    let mut cmd = Command::cargo_bin("schneeforge").unwrap();
    cmd.arg("source")
        .arg("metadata")
        .arg("0.2.0")
        .assert()
        .failure()
        .stderr(predicate::str::contains("must start with 'v'"));
}

// -------------------------------------------------------------------------
// v2 §7: managed release source (working tree-less)
// -------------------------------------------------------------------------

fn git_available() -> bool {
    schneeforge_core::ToolInventory::discover().git.is_some()
}

/// test 内で直接 git を実行する (crates/cli/tests は raw spawn 許可対象)
fn git(dir: &std::path::Path, args: &[&str]) {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@t")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@t")
        .output()
        .expect("run git");
    assert!(
        out.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

fn cli_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "sf-cli-managed-{name}-{}-{}",
        std::process::id(),
        line!()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// state 隔離用の XDG_STATE_HOME temp dir。CLI に dev machine の実 state
/// (~/.local/state/schneeforge) を読ませると、手動実行 (source init 等) の
/// state 汚染が無関係の test まで崩す (2026-08-20 に実際に発生:
/// source status / profile list が実 state の managed source で誤 fail)。
/// state を読み得る起動は全てこの dir へ隔離する
fn state_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("sf-cli-state-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// tag を持つ local の origin repo (network 不要の ls-remote 先)
fn origin_repo(dir: &std::path::Path, tags: &[&str]) -> std::path::PathBuf {
    let origin = dir.join("origin");
    std::fs::create_dir_all(&origin).unwrap();
    git(&origin, &["init", "-q", "-b", "main"]);
    std::fs::write(origin.join("README.md"), "# test\n").unwrap();
    git(&origin, &["add", "."]);
    git(&origin, &["commit", "-q", "-m", "init"]);
    for tag in tags {
        git(&origin, &["tag", tag]);
    }
    origin
}

/// state.json に source を直接書き込む (XDG_STATE_HOME 配下)
fn write_source_state(dir: &std::path::Path, source_json: &str) {
    let state_dir = dir.join("schneeforge");
    std::fs::create_dir_all(&state_dir).unwrap();
    std::fs::write(
        state_dir.join("state.json"),
        format!("{{\"source\": {source_json}}}"),
    )
    .unwrap();
}

/// v2 §7: `source init --tag` は managed source を state に設定する。
/// tag 解決の ls-remote は SCHNEEFORGE_REPO_URL を local origin へ向けて
/// network 無しで実行する。metadata asset が無い tag は警告付き skip のため
/// offline でも成功する
#[test]
fn source_init_with_tag_sets_managed_state() {
    if !git_available() {
        eprintln!("skipping: git not available");
        return;
    }
    let dir = cli_dir("init-tag");
    let origin = origin_repo(&dir, &["v0.1.0"]);
    let mut cmd = Command::cargo_bin("schneeforge").unwrap();
    cmd.arg("source")
        .arg("init")
        .arg("--tag")
        .arg("v0.1.0")
        .env("XDG_STATE_HOME", &dir)
        .env("SCHNEEFORGE_REPO_URL", &origin)
        .assert()
        .success()
        .stdout(predicate::str::contains("managed source set"));
    let state = std::fs::read_to_string(dir.join("schneeforge/state.json")).unwrap();
    assert!(state.contains("\"managed\": true"), "state: {state}");
    assert!(state.contains("\"ref\": \"v0.1.0\""), "state: {state}");
    assert!(state.contains("\"channel\": \"stable\""), "state: {state}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// v2 §7: `source init` の tag と channel の不整合は fail-closed。
/// tag 解決の ls-remote は local origin を向けるため network 不要
#[test]
fn source_init_rejects_tag_channel_mismatch() {
    if !git_available() {
        eprintln!("skipping: git not available");
        return;
    }
    let dir = cli_dir("init-mismatch");
    let origin = origin_repo(&dir, &["v0.2.0"]);
    let mut cmd = Command::cargo_bin("schneeforge").unwrap();
    cmd.arg("source")
        .arg("init")
        .arg("--channel")
        .arg("stable")
        .arg("--tag")
        .arg("v0.3.0-rc.1")
        .env("XDG_STATE_HOME", &dir)
        .env("SCHNEEFORGE_REPO_URL", &origin)
        .assert()
        .failure()
        .stderr(predicate::str::contains("is preview"));
    let _ = std::fs::remove_dir_all(&dir);
}

/// v2 §7: managed state で `source status` は state 由来の情報を表示する
/// (表現 / channel / rev 検証 / cache 有無)
#[test]
fn source_status_shows_managed_state() {
    let dir = cli_dir("status-managed");
    write_source_state(
        &dir,
        r#"{"kind":"release-stable","ref":"v0.2.0","channel":"stable","managed":true,"remote":"https://github.com/Lamy210/nix_setting.git","revision":"0123456789abcdef0123456789abcdef01234567"}"#,
    );
    let mut cmd = Command::cargo_bin("schneeforge").unwrap();
    cmd.arg("source")
        .arg("status")
        .env("XDG_STATE_HOME", &dir)
        .assert()
        .success()
        .stdout(
            predicate::str::contains("(managed)")
                .and(predicate::str::contains(
                    "github:Lamy210/nix_setting/v0.2.0",
                ))
                .and(predicate::str::contains("channel:   stable"))
                .and(predicate::str::contains("(verified)"))
                .and(predicate::str::contains("file cache:")),
        );
    let _ = std::fs::remove_dir_all(&dir);
}

/// v2 §7: managed field を持たない旧 state.json は checkout 表現として
/// 扱われる (managed short-circuit しない)
#[test]
fn legacy_state_without_managed_is_checkout_representation() {
    if !git_available() {
        eprintln!("skipping: git not available");
        return;
    }
    let dir = cli_dir("legacy-state");
    // 旧形式: managed / remote / revision 無し
    write_source_state(
        &dir,
        r#"{"kind":"release-stable","ref":"v0.2.0","channel":"stable"}"#,
    );
    // repo は git 管理外 → detect が走り local になる (state の kind を使わない)
    std::fs::create_dir_all(dir.join("repo")).unwrap();
    let mut cmd = Command::cargo_bin("schneeforge").unwrap();
    cmd.arg("--repo")
        .arg(dir.join("repo"))
        .arg("source")
        .arg("status")
        .env("XDG_STATE_HOME", &dir)
        .assert()
        .success()
        .stdout(predicate::str::contains("kind:    local"));
    let _ = std::fs::remove_dir_all(&dir);
}

/// v2 §7: managed source の update は state の tag 更新のみ行う
/// (checkout は操作しない)。tag 解決は local origin の ls-remote
/// (network 不要)。metadata は警告付き skip。
#[test]
fn managed_update_moves_state_without_touching_checkout() {
    if !git_available() {
        eprintln!("skipping: git not available");
        return;
    }
    let dir = cli_dir("update-managed");
    let origin = origin_repo(&dir, &["v0.2.0", "v0.3.0"]);

    // install.sh 相当の pinned checkout (v0.2.0) を用意する
    let checkout = dir.join("nix_setting");
    let out = std::process::Command::new("git")
        .arg("clone")
        .arg("--branch")
        .arg("v0.2.0")
        .arg("--depth")
        .arg("1")
        .arg(&origin)
        .arg(&checkout)
        .output()
        .expect("git clone");
    assert!(
        out.status.success(),
        "clone failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // state は managed v0.2.0 (remote = local origin)
    write_source_state(
        &dir,
        &format!(
            r#"{{"kind":"release-stable","ref":"v0.2.0","channel":"stable","managed":true,"remote":"{}"}}"#,
            origin.display()
        ),
    );

    let mut cmd = Command::cargo_bin("schneeforge").unwrap();
    cmd.arg("--repo")
        .arg(&checkout)
        .arg("update")
        .env("XDG_STATE_HOME", &dir)
        .assert()
        .success()
        .stdout(
            predicate::str::contains("updated managed source to v0.3.0")
                .and(predicate::str::contains("release-stable")),
        );

    // state は新 tag へ更新されている
    let state = std::fs::read_to_string(dir.join("schneeforge/state.json")).unwrap();
    assert!(state.contains("\"ref\": \"v0.3.0\""), "state: {state}");

    // checkout は v0.2.0 のまま (managed update は checkout を操作しない)
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(&checkout)
        .arg("describe")
        .arg("--tags")
        .arg("--exact-match")
        .output()
        .expect("git describe");
    assert_eq!(
        String::from_utf8_lossy(&out.stdout).trim(),
        "v0.2.0",
        "checkout must stay pinned to v0.2.0"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// v2 §7: managed source の sync は git 実態が無い旨を案内して
/// error にならない
#[test]
fn managed_sync_is_guidance_not_error() {
    let dir = cli_dir("sync-managed");
    write_source_state(
        &dir,
        r#"{"kind":"release-stable","ref":"v0.2.0","channel":"stable","managed":true}"#,
    );
    let mut cmd = Command::cargo_bin("schneeforge").unwrap();
    cmd.arg("--repo")
        .arg(dir.join("repo"))
        .arg("source")
        .arg("sync")
        .env("XDG_STATE_HOME", &dir)
        .assert()
        .success()
        .stdout(predicate::str::contains("no git working tree"));
    let _ = std::fs::remove_dir_all(&dir);
}

/// v2 §7: managed source の deps update は fail-closed に拒否される
/// (flake ref が実体のため lock を書き換えられない)
#[test]
fn managed_deps_update_is_rejected() {
    let dir = cli_dir("deps-managed");
    write_source_state(
        &dir,
        r#"{"kind":"release-stable","ref":"v0.2.0","channel":"stable","managed":true}"#,
    );
    let mut cmd = Command::cargo_bin("schneeforge").unwrap();
    cmd.arg("--repo")
        .arg(dir.join("repo"))
        .arg("source")
        .arg("deps-update")
        .env("XDG_STATE_HOME", &dir)
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be updated locally"));
    let _ = std::fs::remove_dir_all(&dir);
}

/// `self-update` が help に列挙されていること
#[test]
fn help_lists_self_update() {
    let mut cmd = Command::cargo_bin("schneeforge").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("self-update"));
}

fn self_update_platform_supported() -> bool {
    schneeforge_core::current_platform_asset().is_ok()
}

/// self-update は release tag が channel に無いと fail-closed。
/// ls-remote 先は tag 無し local origin を向けるため network 不要
#[test]
fn self_update_fails_closed_without_release_tags() {
    if !git_available() {
        eprintln!("skipping: git not available");
        return;
    }
    if !self_update_platform_supported() {
        eprintln!("skipping: no release binary for this platform");
        return;
    }
    let dir = cli_dir("self-update-no-tags");
    let origin = origin_repo(&dir, &[]);
    let state = state_dir("self-update-no-tags");
    let mut cmd = Command::cargo_bin("schneeforge").unwrap();
    cmd.arg("self-update")
        .env("XDG_STATE_HOME", &state)
        .env("SCHNEEFORGE_REPO_URL", &origin)
        .assert()
        .failure()
        .stderr(predicate::str::contains("release tag"));
    let _ = std::fs::remove_dir_all(&dir);
}

/// tag 解決後の asset URL 構築は github slug を要求する (fail-closed)。
/// tag あり local origin (github 形式ではない path) を向けるため network 不要
#[test]
fn self_update_fails_closed_for_non_github_origin() {
    if !git_available() {
        eprintln!("skipping: git not available");
        return;
    }
    if !self_update_platform_supported() {
        eprintln!("skipping: no release binary for this platform");
        return;
    }
    let dir = cli_dir("self-update-non-github");
    // 現行 version より新しい stable tag を用意する (plan が Update まで進む)
    let origin = origin_repo(&dir, &["v9.9.9"]);
    let state = state_dir("self-update-non-github");
    let mut cmd = Command::cargo_bin("schneeforge").unwrap();
    cmd.arg("self-update")
        .env("XDG_STATE_HOME", &state)
        .env("SCHNEEFORGE_REPO_URL", &origin)
        .assert()
        .failure()
        .stderr(predicate::str::contains("owner/repo"));
    let _ = std::fs::remove_dir_all(&dir);
}

/// channel の最新が利用中の版より古い場合 (state 未初期化で channel が
/// stable default、実行版が rc の場合等) は downgrade せず、その旨を
/// 正しく表示する。channel 最新 tag (v0.0.1) は local origin で用意
/// (UpToDate で終わるため network 不要)
#[test]
fn self_update_reports_newer_than_channel_latest() {
    if !git_available() {
        eprintln!("skipping: git not available");
        return;
    }
    if !self_update_platform_supported() {
        eprintln!("skipping: no release binary for this platform");
        return;
    }
    let dir = cli_dir("self-update-older-latest");
    let origin = origin_repo(&dir, &["v0.0.1"]);
    let state = state_dir("self-update-older-latest");
    let mut cmd = Command::cargo_bin("schneeforge").unwrap();
    cmd.arg("self-update")
        .env("XDG_STATE_HOME", &state)
        .env("SCHNEEFORGE_REPO_URL", &origin)
        .assert()
        .success()
        // 「利用中の版 (channel 最新) が最新です」の誤表示が出ないこと
        .stdout(predicate::str::contains("更新しません"))
        .stdout(predicate::str::contains("channel の最新は 0.0.1"))
        .stdout(predicate::str::contains("利用中の版 (0."));
    let _ = std::fs::remove_dir_all(&dir);
}
