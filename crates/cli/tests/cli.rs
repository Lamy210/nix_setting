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
