//! Managed Nix bootstrap (design.md / ADR-0001)
//!
//! SchneeForge の Core から NixOS/nix-installer を外部プロセスとして実行し、
//! `/nix/receipt.json` を source of truth として扱う。

pub mod download;
pub mod error;
pub mod installer;
pub mod manifest;
pub mod ownership;
pub mod provider;
pub mod receipt;
pub mod verify;

pub use download::{cache_path, download, download_text};
pub use error::ManagedNixError;
pub use installer::{
    install_args, installed_binary_path, parse_json_line, plan_args, planner_name,
    run_with_json_logs, uninstall_args, InstallPhase, JsonLogLine,
};
pub use manifest::{BootstrapManifest, ManagedNixSection, Sha256ByArch};
pub use ownership::{default_ownership_path, OwnershipRecord};
pub use provider::Provider;
pub use receipt::{default_receipt_path, Receipt};
pub use verify::{parse_sha256_sums, sha256_hex, verify_file, verify_sha256};

use std::path::{Path, PathBuf};

use crate::discovery::{detect_arch, detect_platform, has_nix, Architecture, Platform};

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
            existing_nix: has_nix(),
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

/// plan ファイルを保存する directory。
///
/// `/tmp` は world-writable で symlink attack の危険があるため、
/// `$XDG_STATE_HOME/schneeforge/managed-nix/plans/` を使う。
/// 以下を明示的に保証する:
///   - directory 自体が symlink なら拒否 (metadata ではなく symlink_metadata で判定)
///   - owner が現在の uid と一致しなければ拒否
///   - permission を 0700 (owner のみ access) に設定
pub fn secure_plan_dir() -> Result<PathBuf, ManagedNixError> {
    let base = dirs::state_dir()
        .or_else(dirs::data_dir)
        .ok_or_else(|| ManagedNixError::Io {
            context: "resolve XDG state/data dir".to_string(),
            source: "XDG state/data dir unavailable".to_string(),
        })?;
    let dir = base.join("schneeforge").join("managed-nix").join("plans");
    std::fs::create_dir_all(&dir).map_err(|e| ManagedNixError::Io {
        context: format!("create plan dir {}", dir.display()),
        source: e.to_string(),
    })?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};

        // symlink_metadata は symlink 自体の情報を返す (follow しない)
        let meta = std::fs::symlink_metadata(&dir).map_err(|e| ManagedNixError::Io {
            context: format!("stat plan dir {}", dir.display()),
            source: e.to_string(),
        })?;
        if meta.file_type().is_symlink() {
            return Err(ManagedNixError::Io {
                context: format!("plan dir {} is a symlink; refusing to write", dir.display()),
                source: String::new(),
            });
        }
        let uid = current_uid();
        if meta.uid() != uid {
            return Err(ManagedNixError::Io {
                context: format!(
                    "plan dir {} is owned by uid {} (expected {uid}); refusing to write",
                    dir.display(),
                    meta.uid()
                ),
                source: String::new(),
            });
        }
        // 0700 を強制 (作成時の umask に依存しない)
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).map_err(|e| {
            ManagedNixError::Io {
                context: format!("chmod 0700 {}", dir.display()),
                source: e.to_string(),
            }
        })?;
    }

    Ok(dir)
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

    /// binary を download し (キャッシュ優先)、SHA256 を検証して path を返す
    pub fn fetch_binary(
        &self,
        platform: Platform,
        arch: Architecture,
    ) -> Result<PathBuf, ManagedNixError> {
        let (url, expected) = self.resolve_asset(platform, arch)?;
        let cache = cache_path(self.version())?;

        if cache.exists() {
            // キャッシュがあっても SHA256 を再検証する (manifest 更新で古い版が残るのを防ぐ)
            match verify_file(&cache, &expected) {
                Ok(()) => return Ok(cache),
                Err(ManagedNixError::ChecksumMismatch { .. }) => {
                    // キャッシュが壊れているので削除して再取得
                    let _ = std::fs::remove_file(&cache);
                }
                Err(e) => return Err(e),
            }
        }

        download(&url, &cache)?;
        verify_file(&cache, &expected)?;
        Ok(cache)
    }

    /// preflight を返す (design.md D8)。install 開始前に呼ぶ。
    pub fn preflight(&self) -> PreflightSummary {
        PreflightSummary::detect()
    }

    /// plan ファイルを生成する (`nix-installer plan <planner> --out-file <plan.json>`)
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
        let args = plan_args(planner, out_file, extra_conf);
        let mut noop = NoProgress;
        let sink = progress.unwrap_or(&mut noop);
        run_with_json_logs(binary, &args, |line| sink.on_log(line))
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

    /// download → verify → plan → install の全 phase を progress へ通知しながら実行する。
    /// Phase 2 (GUI) はこの 1 メソッドを呼ぶだけで全 phase の進捗を受け取れる。
    pub fn install_with_progress(
        &self,
        platform: Platform,
        arch: Architecture,
        plan_dir: &Path,
        extra_conf: &[String],
        progress: &mut dyn ProgressSink,
    ) -> Result<PathBuf, ManagedNixError> {
        progress.on_phase(InstallPhase::Download);
        let binary = self.fetch_binary(platform, arch)?;
        progress.on_phase(InstallPhase::Verify);

        let planner = planner_name(platform, arch)?;
        std::fs::create_dir_all(plan_dir).map_err(|e| ManagedNixError::Io {
            context: format!("create plan dir {}", plan_dir.display()),
            source: e.to_string(),
        })?;
        // predictable name なので、既存 file が symlink 等で置き換えられていないか
        // 確認してから上書きする (0700 dir 内だが defense in depth)
        let plan_file = plan_dir.join(format!("plan-{}.json", self.version()));
        if plan_file.symlink_metadata().is_ok() {
            let meta = plan_file
                .symlink_metadata()
                .map_err(|e| ManagedNixError::Io {
                    context: format!("stat plan file {}", plan_file.display()),
                    source: e.to_string(),
                })?;
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
        self.run_install(&binary, &plan_file, progress)?;
        progress.on_phase(InstallPhase::PostInstall);
        Ok(plan_file)
    }

    /// `/nix/nix-installer uninstall --no-confirm` を呼ぶ。receipt は default。
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
x86_64-linux = "3b49a0b9deadbeef"
aarch64-linux = "cafebabe"
aarch64-darwin = "feedface"
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
        assert_eq!(sha, "3b49a0b9deadbeef");
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

    #[test]
    fn is_supported_matrix() {
        assert!(is_supported(Platform::Linux, Architecture::X86_64));
        assert!(is_supported(Platform::Linux, Architecture::Aarch64));
        assert!(is_supported(Platform::MacOS, Architecture::Aarch64));
        assert!(!is_supported(Platform::MacOS, Architecture::X86_64));
        assert!(!is_supported(Platform::Unsupported, Architecture::X86_64));
    }
}
