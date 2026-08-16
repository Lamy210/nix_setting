//! Managed Nix bootstrap (design.md / ADR-0001)
//!
//! SchneeForge の Core から NixOS/nix-installer を外部プロセスとして実行し、
//! `/nix/receipt.json` を source of truth として扱う。

pub mod download;
pub mod error;
pub mod escalate;
pub mod installer;
pub mod manifest;
pub mod ownership;
pub mod provider;
pub mod receipt;
pub mod status;
pub mod verify;

pub use download::{cache_path, download, download_text};
pub use error::ManagedNixError;
pub use escalate::{escalate_command, self_binary_path, EscalatedOp};
pub use installer::{
    install_args, installed_binary_path, parse_json_line, plan_args, planner_name, repair_args,
    run_with_json_logs, run_with_json_logs_capture_stdout, uninstall_args, InstallPhase,
    JsonLogLine, UpstreamRepair,
};
pub use manifest::{BootstrapManifest, ManagedNixSection, Sha256ByArch};
pub use ownership::{default_ownership_path, OwnershipRecord};
pub use provider::Provider;
pub use receipt::{default_receipt_path, Receipt};
pub use status::{
    classify, classify_current, repair_action, repair_action_current, NixStatus, RepairAction,
    StatusProbe, StatusReport,
};
pub use verify::{parse_sha256_sums, sha256_hex, verify_file, verify_sha256};

use std::path::{Path, PathBuf};

use crate::discovery::{detect_arch, detect_platform, Architecture, Platform};
use crate::tool::ToolResolver;

/// Managed Nix install の進捗 callback。phase 切替と JSON line の両方を処理する
pub trait ProgressSink {
    fn on_phase(&mut self, phase: InstallPhase);
    fn on_log(&mut self, line: &JsonLogLine);
}

/// デフォルトの progress sink (何もしない)
pub struct NoProgress;
impl ProgressSink for NoProgress {
    fn on_phase(&mut self, _phase: InstallPhase) {}
    fn on_log(&mut self, _line: &JsonLogLine) {}
}

/// 既存 Nix の検出。PATH のみでなく `/nix/var/nix/profiles/default/bin` 等の
/// known locations も含める (sudo の minimal PATH で PATH-only 検出が
/// false negative になる回帰 — #11 と同一の問題 — を防ぐ)。
/// さらに `sudo` 実行時に root 環境からは元 user の profile が見えない
/// ケースに備え、installation marker (`/nix/store`, `/nix/var/nix`,
/// `/nix/receipt.json`) の存在でも検出する (fail-closed)。
/// `discovery::has_nix` (which のみ) は新規コードで使わないこと。
pub fn existing_nix_detected() -> bool {
    if ToolResolver::new().resolve_tool("nix").is_some() {
        return true;
    }
    // /nix 直下の marker。部分削除された degraded install も「存在する」と扱う
    Path::new("/nix/store").exists()
        || Path::new("/nix/var/nix").exists()
        || Path::new("/nix/receipt.json").exists()
}

/// preflight の結果 (root 不要、design.md D8)
#[derive(Debug, Clone)]
pub struct PreflightSummary {
    pub platform: Platform,
    pub arch: Architecture,
    /// 既に Nix が検出されている場合、Managed Nix を上書きで入れない
    pub existing_nix: bool,
    /// 現在 root かどうか
    pub is_root: bool,
    /// SchneeForge がサポートする arch か
    pub supported: bool,
}

impl PreflightSummary {
    pub fn detect() -> Self {
        let platform = detect_platform();
        let arch = detect_arch();
        let supported = is_supported(platform, arch);
        Self {
            platform,
            arch,
            existing_nix: existing_nix_detected(),
            is_root: is_root(),
            supported,
        }
    }

    /// preflight の内容を人間可読で返す (CLI / GUI 共通)
    pub fn summary_lines(&self) -> Vec<String> {
        let mut lines = Vec::new();
        lines.push(format!("platform: {} / arch: {}", self.platform, self.arch));
        if !self.supported {
            lines.push(format!(
                "  (SchneeForge はこの arch で Managed Nix をサポートしていません: {} {})",
                self.platform, self.arch
            ));
            return lines;
        }
        lines.push("Nix をインストールすると以下を変更します:".to_string());
        lines.push("  - /nix".to_string());
        match self.platform {
            Platform::Linux => lines.push("  - nix-daemon (systemd)".to_string()),
            Platform::MacOS => lines.push("  - nix-daemon (launchd)".to_string()),
            Platform::Unsupported => {}
        }
        lines.push("  - build users".to_string());
        lines.push("  - shell profiles".to_string());
        lines.push("  - flakes (experimental-features)".to_string());
        if self.existing_nix {
            lines.push(
                "  ⚠ 既存の Nix が検出されています。Managed Nix を上書きで入れません。".to_string(),
            );
        }
        lines
    }
}

pub fn is_supported(platform: Platform, arch: Architecture) -> bool {
    matches!(
        (platform, arch),
        (Platform::Linux, Architecture::X86_64)
            | (Platform::Linux, Architecture::Aarch64)
            | (Platform::MacOS, Architecture::Aarch64)
    )
}

pub fn is_root() -> bool {
    current_uid() == 0
}

#[cfg(unix)]
pub(crate) fn current_uid() -> u32 {
    unsafe extern "C" {
        fn geteuid() -> u32;
    }
    // SAFETY: geteuid は失敗しない read-only syscall
    unsafe { geteuid() }
}

#[cfg(not(unix))]
pub(crate) fn current_uid() -> u32 {
    u32::MAX
}

/// root 実行時の SchneeForge privileged state の base (platform 別)。
/// sudo で user の HOME / XDG 変数が持ち込まれることを想定せず、
/// root-managed の固定 path を使う。
///
/// macOS は `/var` が `/private/var` への symlink であるため `/var/...` を
/// 使うと component 毎 symlink 検査に引っかかる。実 path を指定する。
pub fn privileged_state_dir() -> PathBuf {
    if cfg!(target_os = "macos") {
        PathBuf::from("/private/var/db/schneeforge")
    } else {
        PathBuf::from("/var/lib/schneeforge")
    }
}

/// plan ファイルを保存する directory。
///
/// `/tmp` は world-writable で symlink attack の危険があるため、
/// root 実行時は privileged_state_dir 配下、非 root 時は
/// `$XDG_STATE_HOME/schneeforge/managed-nix/plans/` を使う。
/// 以下を明示的に保証する:
///   - 作成した path の全 component が symlink でない (作成後、各 component を検査)
///   - 最終 directory の owner が現在の uid と一致
///   - permission 0700 (owner のみ access)
pub fn secure_plan_dir() -> Result<PathBuf, ManagedNixError> {
    let dir = if is_root() {
        privileged_state_dir()
    } else {
        dirs::state_dir()
            .or_else(dirs::data_dir)
            .ok_or_else(|| ManagedNixError::Io {
                context: "resolve XDG state/data dir".to_string(),
                source: "XDG state/data dir unavailable".to_string(),
            })?
            .join("schneeforge")
    }
    .join("managed-nix")
    .join("plans");

    create_secure_dir(&dir)?;
    verify_no_symlink_components(&dir)?;
    Ok(dir)
}

/// world-writable な parent 配下を避けた 0700 directory を作る。
fn create_secure_dir(dir: &Path) -> Result<(), ManagedNixError> {
    std::fs::create_dir_all(dir).map_err(|e| ManagedNixError::Io {
        context: format!("create dir {}", dir.display()),
        source: e.to_string(),
    })?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700)).map_err(|e| {
            ManagedNixError::Io {
                context: format!("chmod 0700 {}", dir.display()),
                source: e.to_string(),
            }
        })?;
    }
    Ok(())
}

/// `root` から `dir` までの既存 component に symlink が無いこと、
/// および最終 directory の owner が現在の uid であることを検証する。
/// (component 間で symlink を差し替える TOCTOU を完全に防ぐことはできないが、
/// 「user 制御可能な中間 path を root が信頼する」事故を防ぐ)
fn verify_no_symlink_components(dir: &Path) -> Result<(), ManagedNixError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;

        let mut current = PathBuf::from("/");
        for comp in dir.components().skip(1) {
            current.push(comp);
            let meta = std::fs::symlink_metadata(&current).map_err(|e| ManagedNixError::Io {
                context: format!("stat {}", current.display()),
                source: e.to_string(),
            })?;
            if meta.file_type().is_symlink() {
                return Err(ManagedNixError::Io {
                    context: format!(
                        "path component {} is a symlink; refusing",
                        current.display()
                    ),
                    source: String::new(),
                });
            }
        }
        // 最終 directory は所有者も検証
        if dir.symlink_metadata().map(|m| m.uid()).unwrap_or(u32::MAX) != current_uid() {
            return Err(ManagedNixError::Io {
                context: format!(
                    "dir {} is not owned by the current uid; refusing",
                    dir.display()
                ),
                source: String::new(),
            });
        }
    }
    #[cfg(not(unix))]
    {
        // symlink_metadata で最終 component のみ検証 (best effort)
        if let Ok(meta) = dir.symlink_metadata() {
            if meta.file_type().is_symlink() {
                return Err(ManagedNixError::Io {
                    context: format!("dir {} is a symlink; refusing", dir.display()),
                    source: String::new(),
                });
            }
        }
    }
    let _ = dir;
    Ok(())
}

/// plan JSON から人間可読な summary 行を返す (D8: Detailed Plan 表示用)。
/// upstream `InstallPlan` の内部構造に依存しない best-effort な抽出。
pub fn summarize_plan(plan_file: &Path) -> Result<Vec<String>, ManagedNixError> {
    let body = std::fs::read_to_string(plan_file).map_err(|e| ManagedNixError::Io {
        context: format!("read plan {}", plan_file.display()),
        source: e.to_string(),
    })?;
    let parsed: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| ManagedNixError::ReceiptParse {
            source: format!("plan json {}: {e}", plan_file.display()),
        })?;

    let mut lines = Vec::new();
    match parsed.get("actions").and_then(|a| a.as_array()) {
        Some(actions) => {
            lines.push(format!("actions ({}):", actions.len()));
            for a in actions.iter().take(50) {
                // upstream の StatefulAction は { action: { action_name: ... }, state: ... }
                // として直列化される (typetag::serde(tag = "action_name"))
                let name = a
                    .get("action")
                    .and_then(|v| v.get("action_name"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("(unknown)");
                lines.push(format!("  - {name}"));
            }
            if actions.len() > 50 {
                lines.push(format!("  ... and {} more", actions.len() - 50));
            }
        }
        None => lines.push("(actions を読めませんでした)".to_string()),
    }
    if let Some(planner) = parsed
        .get("planner")
        .and_then(|p| p.get("planner"))
        .and_then(|v| v.as_str())
    {
        lines.push(format!("planner: {planner}"));
    }
    Ok(lines)
}

/// bootstrap-manifest.toml の内容と環境から install の準備を行う
pub struct ManagedNix {
    manifest: BootstrapManifest,
    provider: Provider,
}

impl ManagedNix {
    pub fn from_manifest(manifest: BootstrapManifest) -> Self {
        Self {
            manifest,
            provider: Provider::new(),
        }
    }

    /// repo root に配置された `bootstrap-manifest.toml` を読む
    pub fn load_from_repo(repo_root: &Path) -> Result<Self, ManagedNixError> {
        let path = repo_root.join("bootstrap-manifest.toml");
        let body = std::fs::read_to_string(&path).map_err(|e| ManagedNixError::Io {
            context: format!("read {}", path.display()),
            source: e.to_string(),
        })?;
        let manifest = BootstrapManifest::parse(&body)?;
        Ok(Self::from_manifest(manifest))
    }

    pub fn version(&self) -> &str {
        &self.manifest.managed_nix.version
    }

    pub fn manifest(&self) -> &BootstrapManifest {
        &self.manifest
    }

    /// 指定 platform/arch の asset URL と expected sha256 を返す
    pub fn resolve_asset(
        &self,
        platform: Platform,
        arch: Architecture,
    ) -> Result<(String, String), ManagedNixError> {
        let asset = self.provider.asset(self.version(), platform, arch)?;
        let expected = self
            .manifest
            .expected_sha256(&asset.arch_name)
            .ok_or_else(|| ManagedNixError::UnsupportedArch {
                arch: asset.arch_name.clone(),
            })?
            .to_string();
        Ok((asset.url, expected))
    }

    /// binary を download し (キャッシュ優先)、SHA256 を検証して path を返す。
    /// 戻り値の `String` は検証済みの expected SHA256 (ownership record へ保存する)。
    pub fn fetch_binary(
        &self,
        platform: Platform,
        arch: Architecture,
    ) -> Result<(PathBuf, String), ManagedNixError> {
        let (url, expected) = self.resolve_asset(platform, arch)?;
        let cache = cache_path(self.version())?;

        if cache.exists() {
            // キャッシュがあっても SHA256 を再検証する (manifest 更新で古い版が残るのを防ぐ)
            match verify_file(&cache, &expected) {
                Ok(()) => return Ok((cache, expected)),
                Err(ManagedNixError::ChecksumMismatch { .. }) => {
                    // キャッシュが壊れているので削除して再取得
                    let _ = std::fs::remove_file(&cache);
                }
                Err(e) => return Err(e),
            }
        }

        download(&url, &cache)?;
        if let Err(e @ ManagedNixError::ChecksumMismatch { .. }) = verify_file(&cache, &expected) {
            // 不一致 file を次回に持ち越さない (spec: download 済み file は削除)
            let _ = std::fs::remove_file(&cache);
            return Err(e);
        }
        Ok((cache, expected))
    }

    /// preflight を返す (design.md D8)。install 開始前に呼ぶ。
    pub fn preflight(&self) -> PreflightSummary {
        PreflightSummary::detect()
    }

    /// plan ファイルを生成する (`nix-installer plan <planner>` → stdout を書き込み)
    pub fn generate_plan(
        &self,
        binary: &Path,
        planner: &str,
        out_file: &Path,
        extra_conf: &[String],
        mut progress: Option<&mut dyn ProgressSink>,
    ) -> Result<(), ManagedNixError> {
        if let Some(p) = progress.as_deref_mut() {
            p.on_phase(InstallPhase::Plan);
        }
        let args = plan_args(planner, extra_conf);
        let mut noop = NoProgress;
        let sink = progress.unwrap_or(&mut noop);
        let stdout = run_with_json_logs_capture_stdout(binary, &args, |line| sink.on_log(line))?;
        // plan JSON が空なら upstream 契約変更を疑う (Docker E2E 実測では
        // 約 34KB の JSON が出力される)
        if stdout.is_empty() {
            return Err(ManagedNixError::Subprocess {
                exit_status: Some(0),
                stderr_tail: "plan subcommand produced empty stdout (upstream contract change?)"
                    .to_string(),
            });
        }
        std::fs::write(out_file, &stdout).map_err(|e| ManagedNixError::Io {
            context: format!("write plan file {}", out_file.display()),
            source: e.to_string(),
        })?;
        Ok(())
    }

    /// plan ファイルを元に install を実行する
    pub fn run_install(
        &self,
        binary: &Path,
        plan_file: &Path,
        progress: &mut dyn ProgressSink,
    ) -> Result<(), ManagedNixError> {
        progress.on_phase(InstallPhase::Install);
        let args = install_args(plan_file);
        run_with_json_logs(binary, &args, |line| progress.on_log(line))
    }

    /// download → verify → plan 生成までを実行し、plan file path を返す。
    /// design.md D8 の 2 段階 Plan UX: caller はこの結果を表示して
    /// ユーザーの最終確認を取った上で `execute_plan` を呼ぶ。
    /// policy (supported / existing Nix) もここで強制する (Phase 2 GUI も同一 API を通る)。
    pub fn prepare_plan(
        &self,
        platform: Platform,
        arch: Architecture,
        plan_dir: &Path,
        extra_conf: &[String],
        progress: &mut dyn ProgressSink,
    ) -> Result<PathBuf, ManagedNixError> {
        if !is_supported(platform, arch) {
            return Err(ManagedNixError::UnsupportedArch {
                arch: format!("{platform}-{arch}"),
            });
        }
        if existing_nix_detected() {
            return Err(ManagedNixError::ExistingNixDetected {
                path: PathBuf::from("/nix"),
            });
        }

        progress.on_phase(InstallPhase::Download);
        let (binary, _expected_sha) = self.fetch_binary(platform, arch)?;
        progress.on_phase(InstallPhase::Verify);

        let planner = planner_name(platform, arch)?;
        create_secure_dir(plan_dir)?;
        verify_no_symlink_components(plan_dir)?;
        let plan_file = plan_dir.join(format!("plan-{}.json", self.version()));
        // predictable name なので、symlink に差し替えられていないか確認してから上書き
        if let Ok(meta) = plan_file.symlink_metadata() {
            if meta.file_type().is_symlink() {
                return Err(ManagedNixError::Io {
                    context: format!(
                        "plan file {} is a symlink; refusing to overwrite",
                        plan_file.display()
                    ),
                    source: String::new(),
                });
            }
        }

        self.generate_plan(&binary, planner, &plan_file, extra_conf, Some(progress))?;
        Ok(plan_file)
    }

    /// ユーザー確認済みの plan file で install を実行する (D8 の [Install] step)。
    /// upstream は `--no-confirm` で呼ぶため、確認責任は caller 側にある。
    /// `binary` は `fetch_binary` が返した検証済み installer (再取得しない)。
    pub fn execute_plan(
        &self,
        platform: Platform,
        arch: Architecture,
        plan_file: &Path,
        binary: &Path,
        progress: &mut dyn ProgressSink,
    ) -> Result<(), ManagedNixError> {
        if !is_supported(platform, arch) {
            return Err(ManagedNixError::UnsupportedArch {
                arch: format!("{platform}-{arch}"),
            });
        }
        if existing_nix_detected() {
            return Err(ManagedNixError::ExistingNixDetected {
                path: PathBuf::from("/nix"),
            });
        }
        if !plan_file.exists() {
            return Err(ManagedNixError::PlanFileNotFound {
                path: plan_file.to_path_buf(),
            });
        }

        progress.on_phase(InstallPhase::Install);
        let result = self.run_install(binary, plan_file, progress);
        if result.is_ok() {
            progress.on_phase(InstallPhase::PostInstall);
        }
        result
    }
    pub fn run_uninstall(
        &self,
        binary: &Path,
        receipt: Option<&Path>,
    ) -> Result<(), ManagedNixError> {
        let args = uninstall_args(receipt);
        let mut noop = NoProgress;
        run_with_json_logs(binary, &args, |line| noop.on_log(line))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_manifest() -> BootstrapManifest {
        let toml_str = r#"
[managed_nix]
version = "2.35.1"

[managed_nix.sha256_by_arch]
x86_64-linux = "1111111111111111111111111111111111111111111111111111111111111111"
aarch64-linux = "2222222222222222222222222222222222222222222222222222222222222222"
aarch64-darwin = "3333333333333333333333333333333333333333333333333333333333333333"
"#;
        BootstrapManifest::parse(toml_str).unwrap()
    }

    #[test]
    fn resolve_asset_x86_64_linux() {
        let mn = ManagedNix::from_manifest(sample_manifest());
        let (url, sha) = mn
            .resolve_asset(Platform::Linux, Architecture::X86_64)
            .unwrap();
        assert!(url.ends_with("/2.35.1/nix-installer-x86_64-linux"));
        assert_eq!(
            sha,
            "1111111111111111111111111111111111111111111111111111111111111111"
        );
    }

    #[test]
    fn resolve_asset_unsupported_arch() {
        let mn = ManagedNix::from_manifest(sample_manifest());
        // x86_64-darwin は provider 側で弾かれる
        let res = mn.resolve_asset(Platform::MacOS, Architecture::X86_64);
        assert!(matches!(res, Err(ManagedNixError::UnsupportedArch { .. })));
    }

    #[test]
    fn preflight_summary_contains_nix_path() {
        let s = PreflightSummary {
            platform: Platform::Linux,
            arch: Architecture::X86_64,
            existing_nix: false,
            is_root: false,
            supported: true,
        };
        let joined = s.summary_lines().join("\n");
        assert!(joined.contains("/nix"));
        assert!(joined.contains("flakes"));
    }

    #[test]
    fn preflight_unsupported_arch_warns() {
        let s = PreflightSummary {
            platform: Platform::MacOS,
            arch: Architecture::X86_64,
            existing_nix: false,
            is_root: false,
            supported: false,
        };
        let joined = s.summary_lines().join("\n");
        assert!(joined.contains("サポートしていません"));
    }

    #[test]
    fn preflight_existing_nix_warns() {
        let s = PreflightSummary {
            platform: Platform::Linux,
            arch: Architecture::X86_64,
            existing_nix: true,
            is_root: true,
            supported: true,
        };
        let joined = s.summary_lines().join("\n");
        assert!(joined.contains("既存の Nix"));
    }

    /// macOS は `/var` が `/private/var` への symlink のため、privileged state dir
    /// に `/var/...` を使うと component 毎 symlink 検査で self-abort する。
    /// platform 別の実 path であることを検証する。
    #[test]
    fn privileged_state_dir_uses_real_path_per_platform() {
        let dir = privileged_state_dir();
        let s = dir.to_string_lossy();
        if cfg!(target_os = "macos") {
            assert!(s.starts_with("/private/var/"), "got: {s}");
        } else {
            assert!(s.starts_with("/var/lib/"), "got: {s}");
        }
        // 実在する全 component が symlink でないこと
        // (macOS で /var 問題が再発しない保証。未作成の末端は skip)
        let mut current = PathBuf::from("/");
        for comp in dir.components().skip(1) {
            current.push(comp);
            if let Ok(meta) = std::fs::symlink_metadata(&current) {
                assert!(
                    !meta.file_type().is_symlink(),
                    "{} is a symlink",
                    current.display()
                );
            }
        }
    }

    #[test]
    fn is_supported_matrix() {
        assert!(is_supported(Platform::Linux, Architecture::X86_64));
        assert!(is_supported(Platform::Linux, Architecture::Aarch64));
        assert!(is_supported(Platform::MacOS, Architecture::Aarch64));
        assert!(!is_supported(Platform::MacOS, Architecture::X86_64));
        assert!(!is_supported(Platform::Unsupported, Architecture::X86_64));
    }

    /// upstream 2.35.1 の `InstallPlan` 直列化 shape (src/plan.rs + src/action/mod.rs
    /// `#[typetag::serde(tag = "action_name")]` + src/action/stateful.rs
    /// `StatefulAction { action, state }`) に基づく fixture。
    /// この shape が変わったら summarize_plan が壊れるので、この test が検知する。
    #[test]
    fn summarize_plan_reads_upstream_shape() {
        let plan = serde_json::json!({
            "version": { "major": 0, "minor": 1, "patch": 1 },
            "actions": [
                {
                    "action": {
                        "action_name": "create_directory",
                        "path": "/nix",
                        "user": null,
                        "group": null,
                        "mode": 493
                    },
                    "state": "Uncompleted"
                },
                {
                    "action": {
                        "action_name": "create_user",
                        "name": "nixbld1",
                        "uid": 30001
                    },
                    "state": "Uncompleted"
                }
            ],
            "planner": {
                "planner": "linux",
                "settings": { "enable_flakes": true },
                "init": {
                    "init": "systemd",
                    "is_remote": false
                }
            }
        });
        let dir = std::env::temp_dir().join(format!(
            "schneeforge_plan_summary_{}.json",
            std::process::id()
        ));
        std::fs::write(&dir, plan.to_string()).unwrap();
        let lines = summarize_plan(&dir).unwrap();
        let _ = std::fs::remove_file(&dir);

        let joined = lines.join("\n");
        assert!(joined.contains("actions (2):"), "got: {joined}");
        assert!(joined.contains("create_directory"), "got: {joined}");
        assert!(joined.contains("create_user"), "got: {joined}");
        assert!(!joined.contains("(unknown)"), "got: {joined}");
        assert!(joined.contains("planner: linux"), "got: {joined}");
    }

    #[test]
    fn summarize_plan_unknown_action_falls_back() {
        let plan = serde_json::json!({
            "actions": [{ "action": {}, "state": "Uncompleted" }],
        });
        let dir = std::env::temp_dir().join(format!(
            "schneeforge_plan_summary_fallback_{}.json",
            std::process::id()
        ));
        std::fs::write(&dir, plan.to_string()).unwrap();
        let lines = summarize_plan(&dir).unwrap();
        let _ = std::fs::remove_file(&dir);
        assert!(lines.join("\n").contains("(unknown)"));
    }
}
