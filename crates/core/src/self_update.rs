//! 本体自己更新 (Phase E self-update) — CLI `schneeforge self-update` の芯。
//!
//! tag 解決は Dashboard の available 解決と同じ規則 (`git ls-remote --tags`
//! → `latest_tag_for_channel`)、binary の検証は release asset `CHECKSUMS.txt`
//! との sha256 突合 (install.sh と同一の供給網モデル)。置換は実行 binary と
//! 同一 directory の temp file → rename で atomic に行い、検証失敗時は
//! 実行 binary を一切変更しない (fail-closed)。
//!
//! network / fs の effect は `run` に集約し、tag 選択・asset 選択・
//! checksums parse は純関数として分離している (hermetic test 可能)。

use std::path::{Path, PathBuf};

use crate::dashboard::{remote_tags, version_is_newer};
use crate::discovery::{detect_arch, detect_platform, Architecture, Platform};
use crate::error::{Error, Result};
use crate::managed_nix::{download, download_text, verify_file};
use crate::source::{github_slug, latest_tag_for_channel, repo_url};
use crate::tool::ResolvedTool;

const CHECKSUMS_ASSET: &str = "CHECKSUMS.txt";

/// 自己更新の実行結果
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelfUpdateStatus {
    /// 実行 version が channel の最新以上のため何もしなかった
    UpToDate { version: String },
    /// 実行 binary の置換が完了した (次回起動から新 binary)
    Updated {
        from: String,
        to: String,
        exe: PathBuf,
    },
}

/// platform 毎の release binary asset 名 (install.sh と同一の提供条件)。
/// darwin は aarch64 のみ、linux は x86_64 のみ。それ以外は binary asset が
/// 存在しないため download 手前で fail-closed にする。
pub fn platform_asset(platform: Platform, arch: Architecture) -> Result<&'static str> {
    match (platform, arch) {
        (Platform::MacOS, Architecture::Aarch64) => Ok("schneeforge-aarch64-darwin"),
        (Platform::Linux, Architecture::X86_64) => Ok("schneeforge-x86_64-linux"),
        (platform, arch) => Err(Error::UnsupportedPlatform {
            os: platform.to_string(),
            arch: arch.to_string(),
        }),
    }
}

/// 実行環境の asset 名 (`std::env::consts` の検出結果から)
pub fn current_platform_asset() -> Result<&'static str> {
    platform_asset(detect_platform(), detect_arch())
}

/// `CHECKSUMS.txt` (sha256sum 形式) から asset の sha256 を取り出す。
/// entry は `<64hex>  <path>/<asset>` (release.yml は `sha256sum dist/*/*`
/// で path 付きで出力する)。install.sh の `^[0-9a-f]\{64\}  .*/<asset>$`
/// 検証と同じ規則で、平坦 (`<64hex>  <asset>`) な形式も受け付ける。
pub fn expected_sha256(checksums: &str, asset: &str) -> Result<String> {
    for line in checksums.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((sha, name)) = line.split_once("  ") else {
            continue;
        };
        let sha = sha.trim();
        let name = name.trim_start_matches('*');
        let matched = name == asset || name.ends_with(&format!("/{asset}"));
        if !matched || !is_sha256_hex(sha) {
            continue;
        }
        return Ok(sha.to_lowercase());
    }
    Err(Error::SelfUpdate(format!(
        "CHECKSUMS.txt に {asset} の entry がありません (release asset が不正か形式が変わりました)"
    )))
}

fn is_sha256_hex(s: &str) -> bool {
    s.len() == 64 && s.chars().all(|c| c.is_ascii_hexdigit())
}

/// repo_url 規約 (`SCHNEEFORGE_REPO_URL` > `DEFAULT_REPO_URL`) から
/// release asset URL を組み立てる (install.sh の fork 規約と同じ)。
pub fn release_asset_url(tag: &str, asset: &str) -> Result<String> {
    let base = repo_url();
    let Some((owner, repo)) = github_slug(&base) else {
        return Err(Error::SelfUpdate(format!(
            "repository URL から github owner/repo を解決できません: {base}"
        )));
    };
    Ok(format!(
        "https://github.com/{owner}/{repo}/releases/download/{tag}/{asset}"
    ))
}

/// 自己更新の計画 (network 前に決まる純粋な決定)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelfUpdatePlan {
    UpToDate { version: String },
    Update(SelfUpdateAction),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelfUpdateAction {
    pub tag: String,
    /// tag から `v` を除いた version
    pub version: String,
    pub asset: &'static str,
    pub checksums_url: String,
    pub asset_url: String,
}

/// tag 列と channel から自己更新の計画を組み立てる (純関数)。
/// 実行 version が最新以上なら `UpToDate`。
pub fn plan(
    tags: &[String],
    channel: &str,
    current_version: &str,
    asset: &'static str,
) -> Result<SelfUpdatePlan> {
    let tag = latest_tag_for_channel(tags, channel)
        .cloned()
        .ok_or_else(|| {
            Error::SelfUpdate(format!("channel {channel} に release tag が見つかりません"))
        })?;
    let version = tag.strip_prefix('v').unwrap_or(&tag).to_string();
    if !version_is_newer(&version, current_version) {
        return Ok(SelfUpdatePlan::UpToDate { version });
    }
    Ok(SelfUpdatePlan::Update(SelfUpdateAction {
        checksums_url: release_asset_url(&tag, CHECKSUMS_ASSET)?,
        asset_url: release_asset_url(&tag, asset)?,
        tag,
        version,
        asset,
    }))
}

/// 最新 release を解決して自己更新を実行する (network + fs)。
/// channel は state の source channel (未初期化なら stable) を呼び出し元が
/// 渡す (`channel_of`)。
pub fn run(git: &ResolvedTool, current_version: &str, channel: &str) -> Result<SelfUpdateStatus> {
    let asset = current_platform_asset()?;
    let tags = remote_tags(&repo_url(), git)?;
    match plan(&tags, channel, current_version, asset)? {
        SelfUpdatePlan::UpToDate { version } => Ok(SelfUpdateStatus::UpToDate { version }),
        SelfUpdatePlan::Update(action) => {
            let status = execute(&action, current_version);
            if status.is_err() {
                // temp だけ掃除 (実行 binary は触れていない)
                if let Ok(exe) = std::env::current_exe() {
                    if let Some(dir) = exe.parent() {
                        let _ = std::fs::remove_file(dir.join(".schneeforge-self-update.tmp"));
                    }
                }
            }
            status
        }
    }
}

/// 計画に従って download → 検証 → atomic 置換を行う。
fn execute(action: &SelfUpdateAction, current_version: &str) -> Result<SelfUpdateStatus> {
    let checksums = download_text(&action.checksums_url)?;
    let expected = expected_sha256(&checksums, action.asset)?;

    let exe = std::env::current_exe()
        .map_err(|e| Error::SelfUpdate(format!("実行 binary の path を解決できません: {e}")))?;
    let dir = exe.parent().ok_or_else(|| {
        Error::SelfUpdate(format!(
            "実行 binary に親 directory がありません: {}",
            exe.display()
        ))
    })?;
    // 同一 filesystem 上の固定名 temp (rename 可能にするため)
    let temp = dir.join(".schneeforge-self-update.tmp");

    download(&action.asset_url, &temp)?;
    let result = verify_and_replace(&temp, &exe, &expected);
    if result.is_err() {
        let _ = std::fs::remove_file(&temp);
    }
    result?;

    Ok(SelfUpdateStatus::Updated {
        from: current_version.to_string(),
        to: action.version.clone(),
        exe,
    })
}

/// download 済み temp file を検証してから `exe` へ atomic 置換する。
/// 検証失敗・権限不足では `exe` を変更しない。
pub fn verify_and_replace(temp: &Path, exe: &Path, expected_sha256: &str) -> Result<()> {
    verify_file(temp, expected_sha256)?;

    // fsync してから rename (download 直後の page cache のみの状態を避ける)
    std::fs::File::open(temp)
        .and_then(|f| f.sync_all())
        .map_err(|e| Error::SelfUpdate(format!("temp file の fsync に失敗: {e}")))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(exe)
            .map_err(|e| Error::SelfUpdate(format!("実行 binary の metadata 取得に失敗: {e}")))?
            .permissions()
            .mode();
        // 元 binary の mode を踏襲しつつ owner 実行 bit は保証する
        std::fs::set_permissions(temp, std::fs::Permissions::from_mode(mode | 0o100))
            .map_err(|e| Error::SelfUpdate(format!("temp file の権限設定に失敗: {e}")))?;
    }

    match std::fs::rename(temp, exe) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => Err(Error::SelfUpdate(format!(
            "実行 binary ({}) の置換に書き込み権限がありません。`sudo schneeforge self-update` または install.sh で更新してください",
            exe.display()
        ))),
        Err(e) => Err(Error::SelfUpdate(format!(
            "実行 binary ({}) の置換に失敗: {e}",
            exe.display()
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tags(list: &[&str]) -> Vec<String> {
        list.iter().map(|t| t.to_string()).collect()
    }

    #[test]
    fn platform_asset_matches_installer_support_matrix() {
        assert_eq!(
            platform_asset(Platform::MacOS, Architecture::Aarch64).unwrap(),
            "schneeforge-aarch64-darwin"
        );
        assert_eq!(
            platform_asset(Platform::Linux, Architecture::X86_64).unwrap(),
            "schneeforge-x86_64-linux"
        );
    }

    #[test]
    fn platform_asset_rejects_unsupported_combinations() {
        for (p, a) in [
            (Platform::MacOS, Architecture::X86_64),
            (Platform::Linux, Architecture::Aarch64),
            (Platform::Unsupported, Architecture::X86_64),
        ] {
            assert!(
                matches!(platform_asset(p, a), Err(Error::UnsupportedPlatform { .. })),
                "{p} {a} should be rejected"
            );
        }
    }

    #[test]
    fn expected_sha256_parses_path_prefixed_entry() {
        let checksums = "# nix_setting v0.2.0-rc.6\n\n\
             ## Binaries\n\
             AAAAaaaa0000111122223333444455556666777788889999aaaabbbbccccdddd  dist/schneeforge-aarch64-darwin/schneeforge-aarch64-darwin\n\
             BBBBbbbb0000111122223333444455556666777788889999aaaabbbbccccdddd  dist/schneeforge-x86_64-linux/schneeforge-x86_64-linux\n";
        let sha = expected_sha256(checksums, "schneeforge-x86_64-linux").unwrap();
        assert_eq!(
            sha,
            "bbbbbbbb0000111122223333444455556666777788889999aaaabbbbccccdddd"
        );
    }

    #[test]
    fn expected_sha256_accepts_flat_entry() {
        let checksums =
            "aaaa0000111122223333444455556666777788889999aaaabbbbccccdddd0000  schneeforge-x86_64-linux\n";
        let sha = expected_sha256(checksums, "schneeforge-x86_64-linux").unwrap();
        assert_eq!(
            sha,
            "aaaa0000111122223333444455556666777788889999aaaabbbbccccdddd0000"
        );
    }

    #[test]
    fn expected_sha256_fails_closed_without_entry() {
        let checksums =
            "aaaa0000111122223333444455556666777788889999aaaabbbbccccdddd0000  other-asset\n";
        assert!(matches!(
            expected_sha256(checksums, "schneeforge-x86_64-linux"),
            Err(Error::SelfUpdate(_))
        ));
    }

    #[test]
    fn expected_sha256_ignores_malformed_hash() {
        let checksums = "short  dist/schneeforge-x86_64-linux/schneeforge-x86_64-linux\n";
        assert!(expected_sha256(checksums, "schneeforge-x86_64-linux").is_err());
    }

    #[test]
    fn plan_returns_update_for_newer_release() {
        let plan = plan(
            &tags(&["v0.2.0-rc.5", "v0.2.0-rc.6", "v0.3.0"]),
            "preview",
            "0.2.0-rc.5",
            "schneeforge-x86_64-linux",
        )
        .unwrap();
        match plan {
            SelfUpdatePlan::Update(action) => {
                assert_eq!(action.tag, "v0.2.0-rc.6");
                assert_eq!(action.version, "0.2.0-rc.6");
                assert_eq!(
                    action.checksums_url,
                    "https://github.com/Lamy210/nix_setting/releases/download/v0.2.0-rc.6/CHECKSUMS.txt"
                );
                assert_eq!(
                    action.asset_url,
                    "https://github.com/Lamy210/nix_setting/releases/download/v0.2.0-rc.6/schneeforge-x86_64-linux"
                );
            }
            other => panic!("expected Update, got {other:?}"),
        }
    }

    #[test]
    fn plan_is_noop_when_current_is_latest() {
        let plan = plan(
            &tags(&["v0.2.0-rc.5", "v0.2.0-rc.6"]),
            "preview",
            "0.2.0-rc.6",
            "schneeforge-x86_64-linux",
        )
        .unwrap();
        assert_eq!(
            plan,
            SelfUpdatePlan::UpToDate {
                version: "0.2.0-rc.6".to_string()
            }
        );
    }

    #[test]
    fn plan_fails_closed_without_channel_tags() {
        let result = plan(
            &tags(&["v0.3.0"]),
            "preview",
            "0.2.0-rc.5",
            "schneeforge-x86_64-linux",
        );
        assert!(matches!(result, Err(Error::SelfUpdate(_))));
    }

    #[test]
    fn verify_and_replace_swaps_binary_and_preserves_mode() {
        let dir = std::env::temp_dir().join(format!("sf-selfupdate-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let exe = dir.join("schneeforge");
        let temp = dir.join(".schneeforge-self-update.tmp");

        // 既存 binary (0o755) と新 binary を作る
        std::fs::write(&exe, b"old-binary").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&exe, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        let new_bytes = b"new-binary";
        std::fs::write(&temp, new_bytes).unwrap();
        // sha256 of b"new-binary"
        let expected = {
            use sha2::{Digest, Sha256};
            let d = Sha256::digest(new_bytes);
            d.iter().map(|b| format!("{b:02x}")).collect::<String>()
        };

        verify_and_replace(&temp, &exe, &expected).unwrap();

        assert_eq!(std::fs::read(&exe).unwrap(), new_bytes);
        assert!(!temp.exists(), "temp file should be renamed away");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&exe).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o755, "mode should be preserved");
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn verify_and_replace_keeps_exe_on_checksum_mismatch() {
        let dir =
            std::env::temp_dir().join(format!("sf-selfupdate-mismatch-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let exe = dir.join("schneeforge");
        let temp = dir.join(".schneeforge-self-update.tmp");
        std::fs::write(&exe, b"old-binary").unwrap();
        std::fs::write(&temp, b"tampered-binary").unwrap();

        let result = verify_and_replace(&temp, &exe, &"0".repeat(64));

        assert!(matches!(result, Err(Error::ManagedNix(_))));
        assert_eq!(std::fs::read(&exe).unwrap(), b"old-binary");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
