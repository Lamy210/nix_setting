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
fn doctor_runs() {
    // doctor は Fresh install 環境でも動く (Nix 無しで nix_installed=no を表示)
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

/// doctor は Fresh install 環境 (Nix 未解決) でも成功し、未インストール状態を表示する。
/// ToolInventory が partial 化されたので、Nix 無し = exit 0 で診断結果を出す。
/// guard は binary と同一の ToolResolver 解決で判定する (release build の checkPhase
/// sandbox では PATH に nix が無くても /nix を読めるため、PATH check では不正確)
#[test]
fn doctor_succeeds_and_reports_missing_nix_without_nix() {
    if nix_available() {
        eprintln!("skipping: nix is resolvable (test is for no-nix env)");
        return;
    }
    let mut cmd = Command::cargo_bin("schneeforge").unwrap();
    cmd.arg("doctor")
        .assert()
        .success()
        .stdout(predicate::str::contains("[nix]").and(predicate::str::contains("installed: no")));
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
