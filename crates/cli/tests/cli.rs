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
    let mut cmd = Command::cargo_bin("schneeforge").unwrap();
    cmd.arg("doctor")
        .assert()
        .success()
        .stdout(predicate::str::contains("[system]").and(predicate::str::contains("host")));
}

/// status は ToolInventory 解決を必要としないので nix 無しでも動く
#[test]
fn status_runs_without_nix() {
    let mut cmd = Command::cargo_bin("schneeforge").unwrap();
    cmd.arg("status")
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
    let mut cmd = Command::cargo_bin("schneeforge").unwrap();
    cmd.arg("doctor")
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
    let mut cmd = Command::cargo_bin("schneeforge").unwrap();
    cmd.arg("nix").arg("doctor").assert().success().stdout(
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
