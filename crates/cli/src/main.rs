use clap::{Parser, Subcommand};
use schneeforge_core::{detect_target, Manifest, StateStore, Toolchain};
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
}

fn main() {
    let cli = Cli::parse();
    let repo = schneeforge_core::resolve_repo(cli.repo.as_deref());

    // Nix/Git を必要とするコマンドでは起動直後に Toolchain を1回解決する。
    // status / uninstall のような info 系コマンドは Toolchain 無しで動かし、
    // CI の素の Linux runner でもテスト可能にする。
    let result = match cli.command {
        Cmd::Doctor => with_toolchain(doctor, &repo),
        Cmd::Scan => with_toolchain(|tc| scan(&repo, tc), &repo),
        Cmd::Setup => with_toolchain(|tc| setup(&repo, tc), &repo),
        Cmd::Status => status(&repo),
        Cmd::Plan => with_toolchain(|tc| plan(&repo, tc), &repo),
        Cmd::Apply => with_toolchain(|tc| apply(&repo, tc), &repo),
        Cmd::Rollback => with_toolchain(|tc| rollback(&repo, tc), &repo),
        Cmd::Upgrade => with_toolchain(|tc| upgrade(&repo, tc), &repo),
        Cmd::Sync => with_toolchain(|tc| sync(&repo, tc), &repo),
        Cmd::Verify => with_toolchain(|tc| verify(&repo, tc), &repo),
        Cmd::Uninstall => uninstall(),
    };
    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

type Result = std::result::Result<(), String>;

/// Toolchain 必須コマンドのラッパ。解決失敗時はユーザーフレンドリなエラーを返す
fn with_toolchain<F>(f: F, _repo: &str) -> Result
where
    F: FnOnce(&Toolchain) -> Result,
{
    let tc = Toolchain::resolve().map_err(|e| e.to_string())?;
    f(&tc)
}

fn doctor(tc: &Toolchain) -> Result {
    let r = schneeforge_core::doctor(tc);
    println!("=== doctor ===");
    println!();
    println!("[system]");
    println!("  OS:   {}", r.os);
    println!("  arch: {}", r.arch);
    println!();
    println!("[nix]");
    println!("  path:   {}", tc.nix.path.display());
    println!("  source: {}", tc.nix.source);
    if let Some(v) = &tc.nix.version {
        println!("  version: {v}");
    }
    if r.nix {
        println!("  installed: yes");
    } else {
        println!("  installed: no");
        println!("  install:   curl -L https://nixos.org/nix/install | sh");
    }
    println!();
    println!("[homebrew]");
    println!("  installed: {}", if r.homebrew { "yes" } else { "no" });
    println!();
    println!("[git]");
    println!("  path:   {}", tc.git.path.display());
    println!("  source: {}", tc.git.source);
    println!("  installed: {}", if r.git { "yes" } else { "no" });
    println!();
    println!("[host detection]");
    println!("  host: {}", r.host);
    Ok(())
}

fn scan(repo: &str, tc: &Toolchain) -> Result {
    let target = detect_target();
    let manifest = load_manifest(repo);
    println!("=== scan ===");
    println!();
    print!("{}", schneeforge_core::scan(&target, tc));
    println!();
    println!("[manifest]");
    match manifest {
        Some(m) => println!("  schema: {} / user: {}", m.schema, m.user.username),
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
    match manifest {
        Some(m) => println!("  user: {}", m.user.username),
        None => println!("  user: (config.toml not found)"),
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

fn apply(repo: &str, tc: &Toolchain) -> Result {
    let target = detect_target();
    println!("applying host: {target}");
    schneeforge_core::apply(&target, repo, &StateStore::default(), tc, false)
        .map_err(|e| e.to_string())?;
    println!("state saved");
    Ok(())
}

fn setup(repo: &str, tc: &Toolchain) -> Result {
    println!("=== setup ===");
    schneeforge_core::setup(repo, &StateStore::default(), tc).map_err(|e| e.to_string())?;
    println!("state saved");
    Ok(())
}

fn plan(repo: &str, tc: &Toolchain) -> Result {
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

fn rollback(repo: &str, tc: &Toolchain) -> Result {
    let target = detect_target();
    println!("rolling back host: {target}");
    schneeforge_core::rollback(&target, repo, &StateStore::default(), tc, false)
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn upgrade(repo: &str, tc: &Toolchain) -> Result {
    println!("updating flake.lock...");
    schneeforge_core::upgrade(repo, tc, false).map_err(|e| e.to_string())?;
    Ok(())
}

fn sync(repo: &str, tc: &Toolchain) -> Result {
    println!("pulling remote config...");
    schneeforge_core::sync(repo, tc, false).map_err(|e| e.to_string())?;
    Ok(())
}

fn verify(repo: &str, tc: &Toolchain) -> Result {
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
    println!("  nix run nix-darwin -- uninstall");
    Ok(())
}

fn load_manifest(repo: &str) -> Option<Manifest> {
    Manifest::load(repo).ok()
}
