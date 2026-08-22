use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::managed_nix::error::ManagedNixError;

/// download / text 取得に使う HTTP client。
/// network hang で永久待ちしないよう timeout を設定する。
fn http_client() -> Result<reqwest::blocking::Client, ManagedNixError> {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(120))
        .connect_timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| ManagedNixError::Download {
            source: format!("build http client: {e}"),
        })
}

/// installer binary のキャッシュパスを返す。
///
/// - root 実行時: `privileged_state_dir()/managed-nix/cache/{version}/nix-installer`
///   (sudo で user の HOME/XDG が持ち込まれても user-writable path を
///   root 実行 binary の cache に使わない。macOS は `/private/var/db/schneeforge`)
/// - 非 root: `$XDG_DATA_HOME/schneeforge/managed-nix/{version}/nix-installer`
pub fn cache_path(version: &str) -> Result<PathBuf, ManagedNixError> {
    let dir = if crate::managed_nix::is_root() {
        crate::managed_nix::privileged_state_dir()
            .join("managed-nix")
            .join("cache")
    } else {
        dirs::data_dir()
            .ok_or_else(|| ManagedNixError::Io {
                context: "resolve XDG data dir".to_string(),
                source: std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "XDG data dir unavailable",
                )
                .to_string(),
            })?
            .join("schneeforge")
            .join("managed-nix")
    };
    Ok(dir.join(version).join("nix-installer"))
}

/// 指定 URL から `dest` へ download。online でない場合は `ManagedNixError::NetworkRequired`。
///
/// temp file は random suffix + `O_CREAT|O_EXCL` で作成する (既存 file や symlink を
/// 絶対に open しない)。download → verify は caller 責務。
pub fn download(url: &str, dest: &Path) -> Result<(), ManagedNixError> {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|e| ManagedNixError::Io {
            context: format!("create dir {}", parent.display()),
            source: e.to_string(),
        })?;
    }

    // predictable な `<dest>.part` は事前に symlink を置かれる危険があるため、
    // random suffix を使い create_new (O_EXCL) で排他作成する
    let rnd = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    let tmp = dest.with_extension(format!("part-{rnd:08x}-{}", std::process::id()));
    let resp = http_client()?
        .get(url)
        .send()
        .map_err(|e| classify_reqwest_error(&NetworkClassifier::from(&e), dest.exists()));

    // error 時にも temp file を残さない
    let mut resp = match resp {
        Ok(r) => r,
        Err(e) => {
            let _ = fs::remove_file(&tmp);
            return Err(e);
        }
    };

    if !resp.status().is_success() {
        let _ = fs::remove_file(&tmp);
        return Err(ManagedNixError::Download {
            source: format!("HTTP {} for {url}", resp.status()),
        });
    }

    let write_result = (|| -> Result<(), ManagedNixError> {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp)
            .map_err(|e| ManagedNixError::Io {
                context: format!("create {}", tmp.display()),
                source: e.to_string(),
            })?;
        resp.copy_to(&mut file)
            .map_err(|e| ManagedNixError::Download {
                source: format!("body read: {e}"),
            })?;
        file.flush().map_err(|e| ManagedNixError::Io {
            context: format!("flush {}", tmp.display()),
            source: e.to_string(),
        })?;
        Ok(())
    })();

    if let Err(e) = write_result {
        let _ = fs::remove_file(&tmp);
        return Err(e);
    }

    fs::rename(&tmp, dest).map_err(|e| ManagedNixError::Io {
        context: format!("rename {} -> {}", tmp.display(), dest.display()),
        source: e.to_string(),
    })?;

    set_executable(dest)?;
    Ok(())
}

fn set_executable(path: &Path) -> Result<(), ManagedNixError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(path)
            .map_err(|e| ManagedNixError::Io {
                context: format!("stat {}", path.display()),
                source: e.to_string(),
            })?
            .permissions();
        perms.set_mode(0o500); // r-x------ (root only; SchneeForge は root で実行)
        fs::set_permissions(path, perms).map_err(|e| ManagedNixError::Io {
            context: format!("chmod {}", path.display()),
            source: e.to_string(),
        })?;
    }
    let _ = path;
    Ok(())
}

/// reqwest::Error の connection / timeout / request 系フラグの抽出 (テスト容易化)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NetworkClassifier {
    pub is_connect: bool,
    pub is_timeout: bool,
    pub is_request: bool,
}

impl From<&reqwest::Error> for NetworkClassifier {
    fn from(e: &reqwest::Error) -> Self {
        Self {
            is_connect: e.is_connect(),
            is_timeout: e.is_timeout(),
            is_request: e.is_request(),
        }
    }
}

impl NetworkClassifier {
    pub fn is_network(&self) -> bool {
        self.is_connect || self.is_timeout || self.is_request
    }
}

fn classify_reqwest_error(cls: &NetworkClassifier, dest_exists: bool) -> ManagedNixError {
    if cls.is_network() && !dest_exists {
        return ManagedNixError::NetworkRequired;
    }
    ManagedNixError::Download {
        source: "network error".to_string(),
    }
}

/// URL から文字列を GET する (SHA256SUMS / release metadata の取得等)。
/// HTTP error status は error page body を返さず fail-closed にする。
pub fn download_text(url: &str) -> Result<String, ManagedNixError> {
    let client = http_client()?;
    let response = client
        .get(url)
        .send()
        .map_err(|e| ManagedNixError::Download {
            source: format!("{url}: {e}"),
        })?;
    let status = response.status();
    if !status.is_success() {
        return Err(ManagedNixError::Download {
            source: format!("{url}: HTTP {status}"),
        });
    }
    response.text().map_err(|e| ManagedNixError::Download {
        source: format!("{url}: {e}"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_path_contains_version() {
        let p = cache_path("2.35.1").unwrap();
        let s = p.to_string_lossy();
        assert!(s.contains("schneeforge"));
        assert!(s.contains("managed-nix"));
        assert!(s.contains("2.35.1"));
        assert!(s.ends_with("nix-installer"));
    }

    #[test]
    fn classify_network_error_when_dest_missing() {
        let cls = NetworkClassifier {
            is_connect: true,
            is_timeout: false,
            is_request: false,
        };
        assert!(matches!(
            classify_reqwest_error(&cls, false),
            ManagedNixError::NetworkRequired
        ));
    }

    #[test]
    fn classify_network_error_falls_back_when_dest_exists() {
        let cls = NetworkClassifier {
            is_connect: true,
            is_timeout: false,
            is_request: false,
        };
        assert!(matches!(
            classify_reqwest_error(&cls, true),
            ManagedNixError::Download { .. }
        ));
    }

    #[test]
    fn network_classifier_is_network_or() {
        let net = NetworkClassifier {
            is_connect: false,
            is_timeout: true,
            is_request: false,
        };
        assert!(net.is_network());
        let non_net = NetworkClassifier {
            is_connect: false,
            is_timeout: false,
            is_request: false,
        };
        assert!(!non_net.is_network());
    }
}
