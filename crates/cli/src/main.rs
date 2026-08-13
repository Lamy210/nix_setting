use clap::{Parser, Subcommand};
use schneeforge_core::{detect_host, has_git, has_homebrew, has_nix, Host, Manifest, State};
use std::process::Command;
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
    let repo = resolve_repo(cli.repo.as_deref());
    let result = match cli.command {
        Cmd::Doctor => doctor(),
        Cmd::Scan => scan(&repo),
        Cmd::Setup => setup(&repo),
        Cmd::Status => status(&repo),
        Cmd::Plan => plan(&repo),
        Cmd::Apply => apply(&repo),
        Cmd::Rollback => rollback(),
        Cmd::Upgrade => upgrade(),
        Cmd::Sync => sync(),
        Cmd::Verify => verify(),
        Cmd::Uninstall => uninstall(),
    };
    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

fn resolve_repo(cli_repo: Option<&str>) -> String {
    if let Some(r) = cli_repo {
        return r.to_string();
    }
    if let Ok(r) = std::env::var("NIX_SETTING_DIR") {
        return r;
    }
    if let Ok(home) = std::env::var("HOME") {
        return format!("{home}/nix_setting");
    }
    ".".to_string()
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

fn scan(repo: &str) -> Result {
    let host = detect_host();
    let manifest = load_manifest(repo);
    println!("=== scan ===");
    println!();
    print!("{}", schneeforge_core::scan(host));
    println!();
    println!("[manifest]");
    match manifest {
        Some(m) => println!("  schema: {} / user: {}", m.schema, m.user.username),
        None => println!("  config.toml not found"),
    }
    Ok(())
}

fn status(repo: &str) -> Result {
    let host = detect_host();
    let manifest = load_manifest(repo);
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

fn apply(repo: &str) -> Result {
    let host = detect_host();
    if host == Host::Unsupported {
        return Err(format!(
            "unsupported platform: {} {}",
            std::env::consts::OS,
            std::env::consts::ARCH
        ));
    }
    println!("applying host: {host}");
    print!("{}", schneeforge_core::apply(host, repo)?);

    // 適用後に状態を記録
    let revision = current_git_revision(repo);
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

fn setup(repo: &str) -> Result {
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
    apply(repo)
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

fn plan(repo: &str) -> Result {
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
        format!("{repo}#darwinConfigurations.{host}.system")
    } else {
        format!("{repo}#homeConfigurations.{host}.activationPackage")
    };
    println!("  target: {target}");
    println!();
    println!("dry-run build...");
    run_nix(["build", "--dry-run", &target])
}

fn rollback() -> Result {
    let host = detect_host();
    println!("rolling back host: {host}");
    print!("{}", schneeforge_core::rollback(host)?);
    Ok(())
}

fn upgrade() -> Result {
    println!("updating flake.lock...");
    print!("{}", schneeforge_core::upgrade()?);
    Ok(())
}

fn sync() -> Result {
    if !has_git() {
        return Err("git not found".to_string());
    }
    println!("pulling remote config...");
    run_command("git", ["pull"])
}

fn verify() -> Result {
    let mut pass = 0;
    let mut fail = 0;
    println!("=== verify ===");
    println!();

    // 必須コマンド
    let commands = ["nix", "zsh", "git"];
    println!("[commands]");
    for cmd in commands {
        let ok = schneeforge_core::which(cmd).is_some();
        println!("  {} {}", if ok { "✅" } else { "❌" }, cmd);
        if ok {
            pass += 1;
        } else {
            fail += 1;
        }
    }
    println!();

    // 設定ファイル
    let home = std::env::var("HOME").unwrap_or_default();
    let files = [
        (".zshrc", format!("{home}/.zshrc")),
        (".gitconfig", format!("{home}/.gitconfig")),
        ("starship.toml", format!("{home}/.config/starship.toml")),
    ];
    println!("[config files]");
    for (name, path) in files {
        let ok = std::path::Path::new(&path).exists();
        println!("  {} {}", if ok { "✅" } else { "❌" }, name);
        if ok {
            pass += 1;
        } else {
            fail += 1;
        }
    }
    println!();

    let state = State::load(&State::default_path());
    println!("[state]");
    match &state {
        Some(s) => {
            println!(
                "  ✅ applied: {}",
                s.applied_revision.as_deref().unwrap_or("(none)")
            );
            pass += 1;
        }
        None => {
            println!("  ❌ state not found (apply not run yet)");
            fail += 1;
        }
    }
    println!();

    println!("=== result: {pass} passed, {fail} failed ===");
    if fail > 0 {
        Err(format!("{fail} checks failed"))
    } else {
        Ok(())
    }
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

fn load_manifest(repo: &str) -> Option<Manifest> {
    let path = format!("{repo}/config.toml");
    let content = std::fs::read_to_string(path).ok()?;
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

fn current_git_revision(repo: &str) -> Option<String> {
    let out = Command::new("git")
        .current_dir(repo)
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
    // 人間が読める ISO 8601 (UTC)
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let days = secs / 86400;
    let rem = secs % 86400;
    let hours = rem / 3600;
    let mins = (rem % 3600) / 60;
    let sec = rem % 60;

    // 1970-01-01 からの日数 → 年/月/日 変換 (簡易)
    let (year, month, day) = days_to_ymd(days);
    format!("{year:04}-{month:02}-{day:02}T{hours:02}:{mins:02}:{sec:02}Z")
}

fn days_to_ymd(days: u64) -> (u64, u64, u64) {
    // Howard Hinnant の civil_from_days アルゴリズム
    let z = days as i64 + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as u64, m, d)
}
