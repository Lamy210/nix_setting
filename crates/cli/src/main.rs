use clap::{Parser, Subcommand};
use schneeforge_core::{detect_host, has_git, has_homebrew, has_nix, Host, Manifest, State};
use std::process::Command;

/// Declarative Developer Workstation Manager
#[derive(Parser)]
#[command(name = "schneeforge", version, about)]
struct Cli {
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
    /// アンインストール手順を表示
    Uninstall,
}

fn main() {
    let cli = Cli::parse();
    let result = match cli.command {
        Cmd::Doctor => doctor(),
        Cmd::Scan => scan(),
        Cmd::Setup => setup(),
        Cmd::Status => status(),
        Cmd::Plan => plan(),
        Cmd::Apply => apply(),
        Cmd::Rollback => rollback(),
        Cmd::Upgrade => upgrade(),
        Cmd::Sync => sync(),
        Cmd::Uninstall => uninstall(),
    };
    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

type Result = std::result::Result<(), String>;

fn doctor() -> Result {
    println!("=== doctor ===");
    println!();
    println!("[system]");
    println!("  OS:   {}", std::env::consts::OS);
    println!("  arch: {}", std::env::consts::ARCH);
    println!();
    println!("[nix]");
    if has_nix() {
        println!("  installed: yes");
    } else {
        println!("  installed: no");
        println!("  install:   curl -L https://nixos.org/nix/install | sh");
    }
    println!();
    println!("[homebrew]");
    println!("  installed: {}", if has_homebrew() { "yes" } else { "no" });
    println!();
    println!("[git]");
    println!("  installed: {}", if has_git() { "yes" } else { "no" });
    println!();
    println!("[host detection]");
    println!("  host: {}", detect_host());
    Ok(())
}

fn scan() -> Result {
    let host = detect_host();
    let manifest = load_manifest();
    println!("=== scan ===");
    println!();
    println!("[system]");
    println!("  OS:   {}", std::env::consts::OS);
    println!("  arch: {}", std::env::consts::ARCH);
    println!("  host: {host}");
    println!();
    println!("[nix]");
    println!("  installed: {}", if has_nix() { "yes" } else { "no" });
    println!();
    println!("[homebrew]");
    println!("  installed: {}", if has_homebrew() { "yes" } else { "no" });
    println!();
    println!("[manifest]");
    match manifest {
        Some(m) => println!("  schema: {} / user: {}", m.schema, m.user.username),
        None => println!("  config.toml not found"),
    }
    Ok(())
}

fn status() -> Result {
    let host = detect_host();
    let manifest = load_manifest();
    let state = State::load(&State::default_path());
    println!("=== status ===");
    println!();
    println!("  host: {host}");
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

fn apply() -> Result {
    let host = detect_host();
    if host == Host::Unsupported {
        return Err(format!(
            "unsupported platform: {} {}",
            std::env::consts::OS,
            std::env::consts::ARCH
        ));
    }
    println!("applying host: {host}");

    let result = if host == Host::MacbookAir {
        run_nix([
            "run",
            "nix-darwin",
            "--",
            "switch",
            "--flake",
            &format!(".#{host}"),
        ])
    } else {
        run_nix([
            "run",
            "nixpkgs#home-manager",
            "--",
            "switch",
            "--flake",
            &format!(".#{host}"),
        ])
    };

    result?;

    // 適用後に状態を記録
    let revision = current_git_revision();
    let state = State {
        host: Some(host.name().to_string()),
        applied_revision: revision,
        applied_at: Some(now_string()),
        product_version: Some(env!("CARGO_PKG_VERSION").to_string()),
    };
    let _ = state.save(&State::default_path());
    println!("state saved");
    Ok(())
}

fn setup() -> Result {
    println!("=== setup ===");
    println!();

    if !has_nix() {
        println!("Nix not installed.");
        println!("  curl -L https://nixos.org/nix/install | sh");
        return Ok(());
    }
    println!("[nix] installed");

    enable_flakes();
    println!("[flakes] enabled");

    let host = detect_host();
    if host == Host::Unsupported {
        return Err(format!(
            "unsupported platform: {} {}",
            std::env::consts::OS,
            std::env::consts::ARCH
        ));
    }
    println!("[host] {host}");

    println!();
    apply()
}

fn enable_flakes() {
    let base = std::env::var("XDG_CONFIG_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|_| std::env::var("HOME").map(|h| std::path::PathBuf::from(h).join(".config")))
        .unwrap_or_else(|_| std::path::PathBuf::from("."));
    let conf = base.join("nix").join("nix.conf");
    if let Ok(content) = std::fs::read_to_string(&conf) {
        if content.contains("experimental-features") {
            return;
        }
    }
    let _ = std::fs::create_dir_all(conf.parent().unwrap());
    let line = "experimental-features = nix-command flakes\n";
    match std::fs::OpenOptions::new().append(true).open(&conf) {
        Ok(mut f) => {
            use std::io::Write;
            let _ = f.write_all(line.as_bytes());
        }
        Err(_) => {
            let _ = std::fs::write(&conf, line);
        }
    }
}

fn plan() -> Result {
    let host = detect_host();
    if host == Host::Unsupported {
        return Err(format!(
            "unsupported platform: {} {}",
            std::env::consts::OS,
            std::env::consts::ARCH
        ));
    }
    println!("=== plan ===");
    println!();
    println!("  host: {host}");

    let target = if host == Host::MacbookAir {
        format!(".#darwinConfigurations.{host}.system")
    } else {
        format!(".#homeConfigurations.{host}.activationPackage")
    };
    println!("  target: {target}");
    println!();
    println!("dry-run build...");
    run_nix(["build", "--dry-run", &target])
}

fn rollback() -> Result {
    let host = detect_host();
    if host == Host::Unsupported {
        return Err("unsupported platform".to_string());
    }
    println!("rolling back host: {host}");
    if host == Host::MacbookAir {
        run_command("darwin-rebuild", ["--rollback"])
    } else {
        run_nix(["run", "nixpkgs#home-manager", "--", "switch", "--rollback"])
    }
}

fn upgrade() -> Result {
    println!("updating flake.lock...");
    run_nix(["flake", "update"])
}

fn sync() -> Result {
    if !has_git() {
        return Err("git not found".to_string());
    }
    println!("pulling remote config...");
    run_command("git", ["pull"])
}

fn uninstall() -> Result {
    let host = detect_host();
    println!("=== uninstall ===");
    println!();
    println!("削除レベル:");
    println!("  1. 状態ファイルのみ削除 (安全)");
    println!("  2. Home Manager / nix-darwin の managed config を解除");
    println!("  3. Nix 自体も削除 (既存 Nix は削除禁止)");
    println!();
    println!("ホスト: {host}");
    println!();

    // 状態ファイルを削除
    let state_path = State::default_path();
    if state_path.exists() {
        match std::fs::remove_file(&state_path) {
            Ok(()) => println!("removed state: {}", state_path.display()),
            Err(e) => println!("failed to remove state: {e}"),
        }
    } else {
        println!("no state file found");
    }

    println!();
    println!("設定の完全な解除は手動で:");
    println!("  # Home Manager (Linux)");
    println!("  home-manager uninstall");
    println!();
    println!("  # nix-darwin (macOS)");
    println!("  nix run nix-darwin -- uninstall");
    Ok(())
}

fn load_manifest() -> Option<Manifest> {
    let content = std::fs::read_to_string("config.toml").ok()?;
    Manifest::parse(&content).ok()
}

fn run_nix<I, S>(args: I) -> Result
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    run_command("nix", args)
}

fn run_command<I, S>(cmd: &str, args: I) -> Result
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    let status = Command::new(cmd)
        .args(args)
        .status()
        .map_err(|e| format!("failed to run {cmd}: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{cmd} exited with {}", status.code().unwrap_or(1)))
    }
}

fn current_git_revision() -> Option<String> {
    let out = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()?;
    if out.status.success() {
        String::from_utf8(out.stdout)
            .ok()
            .map(|s| s.trim().to_string())
    } else {
        None
    }
}

fn now_string() -> String {
    // 依存を増やさないための簡易タイムスタンプ (UNIX 秒)
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    secs.to_string()
}
