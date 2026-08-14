use std::fs;
use std::io::Read;
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::managed_nix::error::ManagedNixError;

/// ファイルの SHA256 を hex 文字列 (lowercase, 64 chars) で計算する
pub fn sha256_hex(path: &Path) -> Result<String, ManagedNixError> {
    let mut file = fs::File::open(path).map_err(|e| ManagedNixError::Io {
        context: format!("open {} for sha256", path.display()),
        source: e.to_string(),
    })?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf).map_err(|e| ManagedNixError::Io {
            context: format!("read {}", path.display()),
            source: e.to_string(),
        })?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    let digest = hasher.finalize();
    Ok(hex::encode(&digest[..]))
}

/// 期待値と実測値を比較し、不一致は `ChecksumMismatch` を返す
pub fn verify_sha256(actual: &str, expected: &str) -> Result<(), ManagedNixError> {
    if actual.eq_ignore_ascii_case(expected.trim()) {
        Ok(())
    } else {
        Err(ManagedNixError::ChecksumMismatch {
            expected: expected.to_string(),
            actual: actual.to_string(),
        })
    }
}

/// ファイルを実際に読んで expected と比較する
pub fn verify_file(path: &Path, expected_sha256: &str) -> Result<(), ManagedNixError> {
    let actual = sha256_hex(path)?;
    verify_sha256(&actual, expected_sha256)
}

/// SHA256SUMS 形式 (`<sha256>  <filename>`) を parse して、対象 asset 名の sha256 を取り出す
pub fn parse_sha256_sums(sums: &str, asset_basename: &str) -> Option<String> {
    for line in sums.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        // "<sha256>  <name>" 形式 (二箇所の space or asterisk)
        let mut parts = line.splitn(2, |c: char| c.is_whitespace());
        let sha = parts.next()?.trim();
        let name = parts.next()?.trim_start_matches(|c: char| c == '*' || c.is_whitespace());
        if name == asset_basename {
            return Some(sha.to_lowercase());
        }
    }
    None
}

/// SHA256 hex を表現するためだけの minimal encoder (依存を増やさない)
mod hex {
    pub fn encode(bytes: &[u8]) -> String {
        let mut s = String::with_capacity(bytes.len() * 2);
        for b in bytes {
            s.push_str(&format!("{b:02x}"));
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn sha256_of_known_content() {
        // echo -n "hello" | sha256sum
        // 2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824
        let tmp = std::env::temp_dir().join("schneeforge_verify_test_hello");
        let mut f = fs::File::create(&tmp).unwrap();
        f.write_all(b"hello").unwrap();
        drop(f);

        let sha = sha256_hex(&tmp).unwrap();
        assert_eq!(
            sha,
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn verify_match_case_insensitive() {
        assert!(verify_sha256(
            "ABCDEF0123456789",
            "abcdef0123456789"
        )
        .is_ok());
    }

    #[test]
    fn verify_mismatch_returns_error() {
        let e = verify_sha256("abc", "def").unwrap_err();
        assert!(matches!(e, ManagedNixError::ChecksumMismatch { .. }));
    }

    #[test]
    fn parse_sha256_sums_basic() {
        let sums = "\
abcdef0123456789  nix-installer-x86_64-linux
deadbeef         nix-installer-aarch64-darwin
";
        assert_eq!(
            parse_sha256_sums(sums, "nix-installer-x86_64-linux"),
            Some("abcdef0123456789".to_string())
        );
        assert_eq!(
            parse_sha256_sums(sums, "nix-installer-aarch64-darwin"),
            Some("deadbeef".to_string())
        );
        assert_eq!(parse_sha256_sums(sums, "nix-installer-foo"), None);
    }

    #[test]
    fn parse_sha256_sums_binary_mode_asterisk() {
        // SHA256SUMS は稀に binary mode (`*`) を含む
        let sums = "feedface  *nix-installer-x86_64-linux\n";
        assert_eq!(
            parse_sha256_sums(sums, "nix-installer-x86_64-linux"),
            Some("feedface".to_string())
        );
    }

    #[test]
    fn parse_sha256_sums_lowercases() {
        let sums = "ABCDEFFEDCBA  nix-installer-x86_64-linux\n";
        assert_eq!(
            parse_sha256_sums(sums, "nix-installer-x86_64-linux"),
            Some("abcdeffedcba".to_string())
        );
    }

    #[test]
    fn hex_encode_zero_and_max() {
        assert_eq!(hex::encode(&[0x00]), "00");
        assert_eq!(hex::encode(&[0xff]), "ff");
        assert_eq!(hex::encode(&[]), "");
    }
}
