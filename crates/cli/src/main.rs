use clap::{Parser, Subcommand};
use schneeforge_core::{detect_host, Manifest};
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
    /// 現在の状態を表示
    Status,
    /// ホストを検出して設定を適用 (switch)
    Apply,
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Cmd::Doctor => doctor(),
        Cmd::Status => status(),
        Cmd::Apply => apply(),
    }
}

fn doctor() {
    println!("=== doctor ===");
    println!();
    println!("[system]");
    println!("  OS:   {}", std::env::consts::OS);
    println!("  arch: {}", std::env::consts::ARCH);
    println!();
    println!("[nix]");
    match which("nix") {
        Some(_) => println!("  installed: yes"),
        None => {
            println!("  installed: no");
            println!("  install:   curl -L https://nixos.org/nix/install | sh");
        }
    }
    println!();
    println!("[host detection]");
    println!("  host: {}", detect_host());
}

fn status() {
    let host = detect_host();
    let manifest = load_manifest();
    println!("=== status ===");
    println!();
    println!("  host: {host}");
    match manifest {
        Some(m) => println!("  user: {}", m.user.username),
        None => println!("  user: (config.toml not found)"),
    }
}

fn apply() {
    let host = detect_host();
    if host == schneeforge_core::Host::Unsupported {
        eprintln!(
            "unsupported platform: {} {}",
            std::env::consts::OS,
            std::env::consts::ARCH
        );
        std::process::exit(1);
    }
    println!("applying host: {host}");

    let status = if host == schneeforge_core::Host::MacbookAir {
        Command::new("nix")
            .args([
                "run",
                "nix-darwin",
                "--",
                "switch",
                "--flake",
                &format!(".#{host}"),
            ])
            .status()
    } else {
        Command::new("nix")
            .args([
                "run",
                "nixpkgs#home-manager",
                "--",
                "switch",
                "--flake",
                &format!(".#{host}"),
            ])
            .status()
    };

    match status {
        Ok(s) if s.success() => {}
        Ok(s) => std::process::exit(s.code().unwrap_or(1)),
        Err(e) => {
            eprintln!("failed to run nix: {e}");
            std::process::exit(1);
        }
    }
}

fn load_manifest() -> Option<Manifest> {
    let content = std::fs::read_to_string("config.toml").ok()?;
    Manifest::parse(&content).ok()
}

fn which(cmd: &str) -> Option<String> {
    let path = std::env::var("PATH").ok()?;
    for dir in path.split(':') {
        let candidate = format!("{dir}/{cmd}");
        if std::path::Path::new(&candidate).is_file() {
            return Some(candidate);
        }
    }
    None
}
