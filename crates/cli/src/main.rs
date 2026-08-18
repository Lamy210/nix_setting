mod nix_cmd;

use clap::{Parser, Subcommand};
use nix_cmd::{NixArgs, NixSub};
use schneeforge_core::{
    detect_target, Manifest, SourceKind, SourceResolver, StateStore, ToolInventory,
};
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
    /// profile の選択管理 (list / set / show)
    Profile(ProfileArgs),
    /// 適用内容を dry-run で確認
    Plan,
    /// ホストを検出して設定を適用 (switch)
    Apply,
    /// 前の世代へロールバック
    Rollback,
    /// configuration source を更新 (Release: 次 tag / Git: pull / Pinned・Local: no-op)
    Update,
    /// source 操作 (status / sync / deps update) — Advanced
    Source(SourceArgs),
    /// 依存 (flake.lock) を更新 [非推奨: source deps update]
    Upgrade,
    /// リモート設定を取得 (git pull) [非推奨: source sync]
    Sync,
    /// インストール後の環境を検証
    Verify,
    /// アンインストール手順を表示
    Uninstall,
    /// Managed Nix (nix-installer 統合) の install / doctor / uninstall
    Nix(NixArgs),
}

/// `schneeforge source` の副コマンド
#[derive(Subcommand)]
enum SourceSub {
    /// 現在の source (kind / ref / channel) を表示
    Status,
    /// git tracking source を pull --ff-only (Advanced)
    Sync,
    /// 依存 (flake.lock) を更新 — `nix flake update` 相当 (Advanced)
    DepsUpdate,
}

/// `schneeforge profile` の副コマンド
#[derive(Subcommand)]
enum ProfileSub {
    /// 利用可能な profile と現在の選択を表示
    List,
    /// profile を選択 (state へ保存。manifest の available 検証あり)
    Set { name: String },
    /// 選択を manifest default へ戻す
    Clear,
    /// 現在解決される profile を表示
    Show,
}

#[derive(Parser)]
struct ProfileArgs {
    #[command(subcommand)]
    command: ProfileSub,
}

#[derive(Parser)]
struct SourceArgs {
    #[command(subcommand)]
    command: SourceSub,
}

fn main() {
    let cli = Cli::parse();
    let repo = schneeforge_core::resolve_repo(cli.repo.as_deref());

    // Nix/Git を必要とするコマンドでは起動直後に ToolInventory を1回 discover する。
    // status / uninstall のような info 系コマンドは ToolInventory 無しで動かし、
    // CI の素の Linux runner でもテスト可能にする。
    let result = match cli.command {
        Cmd::Doctor => with_tool_inventory(|tc| doctor(&repo, tc), &repo),
        Cmd::Scan => with_tool_inventory(|tc| scan(&repo, tc), &repo),
        Cmd::Setup => with_tool_inventory(|tc| setup(&repo, tc), &repo),
        Cmd::Status => status(&repo),
        Cmd::Profile(args) => run_profile(args.command, &repo),
        Cmd::Plan => with_tool_inventory(|tc| plan(&repo, tc), &repo),
        Cmd::Apply => with_tool_inventory(|tc| apply(&repo, tc), &repo),
        Cmd::Rollback => with_tool_inventory(|tc| rollback(&repo, tc), &repo),
        Cmd::Update => with_tool_inventory(|tc| update(&repo, tc), &repo),
        Cmd::Source(args) => with_tool_inventory(|tc| run_source(args.command, &repo, tc), &repo),
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

fn doctor(repo: &str, tc: &ToolInventory) -> Result {
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
    if let Some(source) = source_kind_line(repo, tc) {
        println!("  source: {source}");
    }
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
            "  schema: {} / profile: {}",
            m.schema,
            m.profiles.default.as_deref().unwrap_or("(unset)")
        ),
        None => println!("  schneeforge.toml not found"),
    }
    Ok(())
}

fn status(repo: &str) -> Result {
    let target = detect_target();
    let state = StateStore::default().load();
    println!("=== status ===");
    println!();
    println!("  host: {target}");
    // 実効 profile: state 選択 > manifest default
    match schneeforge_core::resolve_profile(repo) {
        Ok((p, from_state)) => {
            let origin = if from_state { "selected" } else { "default" };
            println!("  profile: {p} ({origin})");
        }
        Err(_) if !std::path::Path::new(&format!("{repo}/schneeforge.toml")).exists() => {
            println!("  profile: (manifest not found)");
        }
        Err(e) => println!("  profile: (error: {e})"),
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
    println!("note: `schneeforge upgrade` is deprecated; use `schneeforge source deps update`");
    println!("updating flake.lock...");
    schneeforge_core::upgrade(repo, tc, false).map_err(|e| e.to_string())?;
    Ok(())
}

fn sync(repo: &str, tc: &ToolInventory) -> Result {
    println!("note: `schneeforge sync` is deprecated; use `schneeforge source sync`");
    println!("pulling remote config...");
    schneeforge_core::sync(repo, tc, false).map_err(|e| e.to_string())?;
    Ok(())
}

fn update(repo: &str, tc: &ToolInventory) -> Result {
    let store = StateStore::default();
    let result = schneeforge_core::update(repo, &store, tc, false).map_err(|e| e.to_string())?;
    if let Some(source) = &result.source {
        println!("source: {} ({})", source.kind, source.ref_);
    }
    println!("state saved");
    Ok(())
}

fn run_profile(sub: ProfileSub, repo: &str) -> Result {
    match sub {
        ProfileSub::List => {
            let manifest = Manifest::load(repo).map_err(|e| e.to_string())?;
            let default = manifest.profiles.default.as_deref().unwrap_or("(unset)");
            let selected = StateStore::default().load().and_then(|s| s.profile);
            println!("=== profiles ===");
            println!();
            for name in &manifest.profiles.available {
                let marker = if Some(name.as_str()) == selected.as_deref() {
                    "*"
                } else if name == default {
                    "(default)"
                } else {
                    ""
                };
                println!("  {name} {marker}");
            }
            println!();
            println!("  * = current selection, (default) = manifest default");
            if selected.is_none() {
                println!("  (no selection; manifest default '{default}' is used)");
            }
            Ok(())
        }
        ProfileSub::Set { name } => {
            let manifest = Manifest::load(repo).map_err(|e| e.to_string())?;
            if !manifest.profiles.available.contains(&name) {
                return Err(format!(
                    "profile '{name}' is not in manifest profiles.available: {:?}",
                    manifest.profiles.available
                ));
            }
            schneeforge_core::save_selection(&name).map_err(|e| e.to_string())?;
            println!("profile set to '{name}' (applies from next `schneeforge apply`)");
            Ok(())
        }
        ProfileSub::Clear => {
            schneeforge_core::clear_selection().map_err(|e| e.to_string())?;
            println!("profile selection cleared (manifest default will be used)");
            Ok(())
        }
        ProfileSub::Show => {
            let (name, from_state) =
                schneeforge_core::resolve_profile(repo).map_err(|e| e.to_string())?;
            let origin = if from_state {
                "state"
            } else {
                "manifest default"
            };
            println!("profile: {name} (from {origin})");
            Ok(())
        }
    }
}

fn run_source(sub: SourceSub, repo: &str, tc: &ToolInventory) -> Result {
    match sub {
        SourceSub::Status => source_status(repo, tc),
        SourceSub::Sync => {
            schneeforge_core::source_sync(repo, tc, false).map_err(|e| e.to_string())?;
            Ok(())
        }
        SourceSub::DepsUpdate => {
            schneeforge_core::deps_update(repo, tc, false).map_err(|e| e.to_string())?;
            Ok(())
        }
    }
}

fn source_status(repo: &str, tc: &ToolInventory) -> Result {
    println!("=== source status ===");
    println!();
    let git = match tc.git.as_ref() {
        Some(g) => g,
        None => {
            let state = StateStore::default().load();
            println!("  (git not found; showing state only)");
            print_state_source(state.as_ref());
            return Ok(());
        }
    };
    let detected = SourceResolver::new()
        .detect(repo, git)
        .map_err(|e| e.to_string())?;
    println!("  kind:    {}", detected.kind);
    println!("  ref:     {}", detected.ref_);
    if let Some(channel) = &detected.channel {
        println!("  channel: {channel}");
    }
    let state = StateStore::default().load();
    print_state_source(state.as_ref());
    if let Some(s) = &state {
        if let Some(rev) = &s.applied_revision {
            println!("  applied: {rev}");
        }
    }
    Ok(())
}

fn print_state_source(state: Option<&schneeforge_core::State>) {
    match state.and_then(|s| s.source.as_ref()) {
        Some(src) => println!(
            "  state:   {} ({})",
            src.kind,
            src.channel.as_deref().unwrap_or(&src.ref_)
        ),
        None => println!("  state:   (source not recorded)"),
    }
}

/// doctor 用: source kind を 1 行で返す (検出失敗は None)
fn source_kind_line(repo: &str, tc: &ToolInventory) -> Option<String> {
    let git = tc.git.as_ref()?;
    let state = SourceResolver::new().detect(repo, git).ok()?;
    let _ = SourceKind::Local; // import 確認用 (kind は Display 経由で使用)
    Some(format!("{} ({})", state.kind, state.ref_))
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
