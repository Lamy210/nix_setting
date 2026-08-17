mod nix_cmd;

use clap::{Parser, Subcommand};
use nix_cmd::{NixArgs, NixSub};
use schneeforge_core::{detect_target, Manifest, StateStore, ToolInventory};
/// Declarative Developer Workstation Manager
#[derive(Parser)]
#[command(name = "schneeforge", version, about)]
struct Cli {
    /// リポジトリのパス (default: $NIX_SETTING_DIR or ~/nix_setting)
    #[arg(long, global = true)]
    repo: Option<String>,

    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// システム / Nix / ホスト互換性を診断
    Doctor,
    /// 環境をスキャンして検出結果を表示
    Scan,
    /// 初回セットアップ (Nix/flakes 確認 → apply)
    Setup,
    /// 現在の状態を表示
    Status,
    /// 適用内容を dry-run で確認
    Plan,
    /// ホストを検出して設定を適用 (switch)
    Apply,
    /// 前の世代へロールバック
    Rollback,
    /// 依存 (flake.lock) を更新
    Upgrade,
    /// リモート設定を取得 (git pull)
    Sync,
    /// インストール後の環境を検証
    Verify,
    /// アンインストール手順を表示
    Uninstall,
    /// Managed Nix (nix-installer 統合) の install / doctor / uninstall
    Nix(NixArgs),
}

fn main() {
    let cli = Cli::parse();
    let repo = schneeforge_core::resolve_repo(cli.repo.as_deref());

    // Nix/Git を必要とするコマンドでは起動直後に ToolInventory を1回 discover する。
    // status / uninstall のような info 系コマンドは ToolInventory 無しで動かし、
    // CI の素の Linux runner でもテスト可能にする。
    let result = match cli.command {
        Cmd::Doctor => with_tool_inventory(doctor, &repo),
        Cmd::Scan => with_tool_inventory(|tc| scan(&repo, tc), &repo),
        Cmd::Setup => with_tool_inventory(|tc| setup(&repo, tc), &repo),
        Cmd::Status => status(&repo),
        Cmd::Plan => with_tool_inventory(|tc| plan(&repo, tc), &repo),
        Cmd::Apply => with_tool_inventory(|tc| apply(&repo, tc), &repo),
        Cmd::Rollback => with_tool_inventory(|tc| rollback(&repo, tc), &repo),
        Cmd::Upgrade => with_tool_inventory(|tc| upgrade(&repo, tc), &repo),
        Cmd::Sync => with_tool_inventory(|tc| sync(&repo, tc), &repo),
        Cmd::Verify => with_tool_inventory(|tc| verify(&repo, tc), &repo),
        Cmd::Uninstall => uninstall(),
        Cmd::Nix(nix_args) => run_nix(nix_args.command, &repo),
    };
    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

type Result = std::result::Result<(), String>;

/// ToolInventory を必要とするコマンドのラッパ。
///
/// `ToolInventory::discover` は infallible (未検出ツールは None になる) なので、
/// ここでは1回 discover した inventory を渡すだけ。Nix が無くても doctor は動く。
fn with_tool_inventory<F>(f: F, _repo: &str) -> Result
where
    F: FnOnce(&ToolInventory) -> Result,
{
    let tc = ToolInventory::discover();
    f(&tc)
}

fn doctor(tc: &ToolInventory) -> Result {
    let r = schneeforge_core::doctor(tc);
    println!("=== doctor ===");
    println!();
    println!("[system]");
    println!("  OS:   {}", r.os);
    println!("  arch: {}", r.arch);
    println!();
    println!("[nix]");
    match tc.nix.as_ref() {
        Some(nix) => {
            println!("  path:   {}", nix.path.display());
            println!("  source: {}", nix.source);
            if let Some(v) = &nix.version {
                println!("  version: {v}");
            }
            println!("  installed: yes");
        }
        None => {
            println!("  installed: no");
            println!("  install:   curl -L https://nixos.org/nix/install | sh");
        }
    }
    println!();
    println!("[homebrew]");
    println!("  installed: {}", if r.homebrew { "yes" } else { "no" });
    println!();
    println!("[git]");
    match tc.git.as_ref() {
        Some(git) => {
            println!("  path:   {}", git.path.display());
            println!("  source: {}", git.source);
            println!("  installed: yes");
        }
        None => {
            println!("  installed: no");
        }
    }
    println!();
    println!("[host detection]");
    println!("  host: {}", r.host);
    // v2: machine 情報は repo でなく MachineFacts 検出で管理する
    match schneeforge_core::MachineFacts::detect() {
        Ok(f) => {
            println!("  user: {}", f.username);
            println!("  home: {}", f.home_directory.display());
            println!("  system: {}", f.nix_system_string());
            println!("  hostname: {}", f.hostname);
        }
        Err(e) => println!("  machine facts: (detection failed: {e})"),
    }
    if r.host == "darwin-aarch64" {
        println!();
        println!("  note: host name was renamed from 'macbook-air' to");
        println!("        'darwin-aarch64' (machine model と platform の分離)");
    }
    println!();
    println!("[managed nix]");
    // D7: schneeforge doctor から schneeforge nix doctor を呼び出して nix 関連 section を埋める
    // 失敗しても全体の doctor は継続する (nix 未 install 環境を考慮)
    if let Err(e) = nix_cmd::run_doctor(Some(tc)) {
        println!("  (managed nix doctor failed: {e})");
    }
    Ok(())
}

fn scan(repo: &str, tc: &ToolInventory) -> Result {
    let target = detect_target();
    let manifest = load_manifest(repo);
    println!("=== scan ===");
    println!();
    print!("{}", schneeforge_core::scan(&target, tc));
    println!();
    println!("[manifest]");
    match manifest {
        Some(m) => println!(
            "  schema: {} / user: {}",
            m.schema,
            m.user
                .as_ref()
                .map(|u| u.username.as_str())
                .unwrap_or("(machine input)")
        ),
        None => println!("  config.toml not found"),
    }
    Ok(())
}

fn status(repo: &str) -> Result {
    let target = detect_target();
    let manifest = load_manifest(repo);
    let state = StateStore::default().load();
    println!("=== status ===");
    println!();
    println!("  host: {target}");
    match manifest.and_then(|m| m.user.map(|u| u.username)) {
        Some(u) => println!("  user: {u}"),
        None => println!("  user: (machine input)"),
    }
    match &state {
        Some(s) => {
            if let Some(rev) = &s.applied_revision {
                println!("  applied: {rev}");
            }
            if let Some(at) = &s.applied_at {
                println!("  applied at: {at}");
            }
        }
        None => println!("  applied: (never)"),
    }
    Ok(())
}

fn apply(repo: &str, tc: &ToolInventory) -> Result {
    let target = detect_target();
    println!("applying host: {target}");
    schneeforge_core::apply(&target, repo, &StateStore::default(), tc, false)
        .map_err(|e| e.to_string())?;
    println!("state saved");
    Ok(())
}

fn setup(repo: &str, tc: &ToolInventory) -> Result {
    println!("=== setup ===");
    schneeforge_core::setup(repo, &StateStore::default(), tc).map_err(|e| e.to_string())?;
    println!("state saved");
    Ok(())
}

fn plan(repo: &str, tc: &ToolInventory) -> Result {
    let t = schneeforge_core::plan_target(repo).map_err(|e| e.to_string())?;
    println!("=== plan ===");
    println!();
    println!("  host: {}", t.host);
    println!("  target: {}", t.flake_target);
    println!();
    println!("dry-run build...");
    schneeforge_core::plan(repo, tc, false).map_err(|e| e.to_string())?;
    Ok(())
}

fn rollback(repo: &str, tc: &ToolInventory) -> Result {
    let target = detect_target();
    println!("rolling back host: {target}");
    schneeforge_core::rollback(&target, repo, &StateStore::default(), tc, false)
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn upgrade(repo: &str, tc: &ToolInventory) -> Result {
    println!("updating flake.lock...");
    schneeforge_core::upgrade(repo, tc, false).map_err(|e| e.to_string())?;
    Ok(())
}

fn sync(repo: &str, tc: &ToolInventory) -> Result {
    println!("pulling remote config...");
    schneeforge_core::sync(repo, tc, false).map_err(|e| e.to_string())?;
    Ok(())
}

fn verify(repo: &str, tc: &ToolInventory) -> Result {
    let report = schneeforge_core::verify(repo, tc);
    println!("=== verify ===");
    println!();
    println!("[checks]");
    for c in &report.checks {
        println!("  {} {}", if c.ok { "✅" } else { "❌" }, c.name);
    }
    println!();
    println!(
        "=== result: {} passed, {} failed ===",
        report.passed(),
        report.failed()
    );
    if report.is_ok() {
        Ok(())
    } else {
        Err(format!("{} checks failed", report.failed()))
    }
}

fn uninstall() -> Result {
    let target = detect_target();
    println!("=== uninstall ===");
    println!();
    println!("削除レベル:");
    println!("  1. 状態ファイルのみ削除 (安全)");
    println!("  2. Home Manager / nix-darwin の managed config を解除");
    println!("  3. Nix 自体も削除 (既存 Nix は削除禁止)");
    println!();
    println!("ホスト: {target}");
    println!();
    println!("設定の完全な解除は手動で:");
    println!("  # Home Manager (Linux)");
    println!("  home-manager uninstall");
    println!();
    println!("  # nix-darwin (macOS)");
    println!("  sudo nix --extra-experimental-features \"nix-command flakes\" \\");
    println!("    run nix-darwin#darwin-uninstaller");
    Ok(())
}

fn load_manifest(repo: &str) -> Option<Manifest> {
    Manifest::load(repo).ok()
}

fn run_nix(sub: NixSub, repo: &str) -> Result {
    let tc = ToolInventory::discover();
    match sub {
        NixSub::Install(args) => nix_cmd::run_install(repo, args),
        NixSub::Doctor => nix_cmd::run_doctor(Some(&tc)),
        NixSub::Uninstall(args) => nix_cmd::run_uninstall(args),
        NixSub::Repair(args) => nix_cmd::run_repair(args, Some(&tc)),
    }
}
