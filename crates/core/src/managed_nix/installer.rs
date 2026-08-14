use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde::Deserialize;

use crate::discovery::{Architecture, Platform};
use crate::managed_nix::error::ManagedNixError;

/// SchneeForge 側で管理する install の大きな phase (design.md D4)
///
/// installer 内部の `Step: CreateUsers` 等のメッセージに深く依存せず、
/// progress 表示をこの phase 単位で行う。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallPhase {
    Download,
    Verify,
    Privilege,
    Plan,
    Install,
    PostInstall,
}

/// `--logger json` が出す JSON Lines の best-effort parse 結果
#[derive(Debug, Clone, Deserialize)]
pub struct JsonLogLine {
    /// tracing level (INFO/WARN/ERROR 等)
    #[serde(default)]
    pub level: Option<String>,
    /// メッセージ本文 (`fields.message` に入ることが多い)
    #[serde(default)]
    pub fields: Option<JsonLogFields>,
    /// span 名 (Step 等)
    #[serde(default)]
    pub spans: Option<Vec<JsonLogSpan>>,
    /// target
    #[serde(default)]
    pub target: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct JsonLogFields {
    #[serde(default)]
    pub message: Option<String>,
    #[serde(flatten)]
    pub extra: std::collections::BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct JsonLogSpan {
    pub name: String,
    #[serde(flatten)]
    pub extra: std::collections::BTreeMap<String, serde_json::Value>,
}

/// JSON Lines の 1 行を parse する。妥協 parse (壊れた行は skip)
pub fn parse_json_line(line: &str) -> Option<JsonLogLine> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }
    serde_json::from_str(trimmed).ok()
}

/// `nix-installer plan <planner>` の CLI args を構築する
pub fn plan_args(planner: &str, out_file: &Path, extra_conf: &[String]) -> Vec<String> {
    let mut args = vec![
        "plan".to_string(),
        planner.to_string(),
        "--out-file".to_string(),
        out_file.to_string_lossy().into_owned(),
        "--enable-flakes".to_string(),
    ];
    if !extra_conf.is_empty() {
        args.push("--extra-conf".to_string());
        args.push(extra_conf.join("\n"));
    }
    args
}

/// `nix-installer install --plan <plan.json>` の CLI args を構築する
///
/// `--plan` と planner-subcommand は排他なので、planner は渡さない。
pub fn install_args(plan_file: &Path) -> Vec<String> {
    vec![
        "install".to_string(),
        "--plan".to_string(),
        plan_file.to_string_lossy().into_owned(),
        "--logger".to_string(),
        "json".to_string(),
        "--enable-flakes".to_string(),
        "--no-confirm".to_string(),
    ]
}

/// `nix-installer uninstall --no-confirm` の CLI args
pub fn uninstall_args(receipt: Option<&Path>) -> Vec<String> {
    let mut args = vec!["uninstall".to_string(), "--no-confirm".to_string()];
    if let Some(r) = receipt {
        args.push(r.to_string_lossy().into_owned());
    }
    args
}

/// subprocess を spawn し、stderr を JSON Lines で best-effort parse しながら callback へ渡す。
/// exit code 0 以外は `Subprocess` エラー (stderr の最後の N 行を保持)。
pub fn run_with_json_logs<F>(
    binary: &Path,
    args: &[String],
    mut on_line: F,
) -> Result<(), ManagedNixError>
where
    F: FnMut(&JsonLogLine),
{
    let mut child = Command::new(binary)
        .args(args)
        .stdout(Stdio::inherit())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| ManagedNixError::Io {
            context: format!("spawn {}", binary.display()),
            source: e.to_string(),
        })?;

    // 失敗時の診断用に stderr の最後の N 行を保持 (ring buffer)
    const TAIL_LINES: usize = 20;
    let mut tail: std::collections::VecDeque<String> = std::collections::VecDeque::new();

    if let Some(stderr) = child.stderr.take() {
        let reader = BufReader::new(stderr);
        for line in reader.lines() {
            match line {
                Ok(s) => {
                    tail.push_back(s.clone());
                    if tail.len() > TAIL_LINES {
                        tail.pop_front();
                    }
                    if let Some(parsed) = parse_json_line(&s) {
                        on_line(&parsed);
                    } else {
                        // JSON parse できなくても行自体は stderr へ見えるように流す
                        eprintln!("{s}");
                    }
                }
                Err(e) => {
                    return Err(ManagedNixError::Io {
                        context: "read installer stderr".to_string(),
                        source: e.to_string(),
                    });
                }
            }
        }
    }

    let status = child.wait().map_err(|e| ManagedNixError::Io {
        context: format!("wait {}", binary.display()),
        source: e.to_string(),
    })?;

    if status.success() {
        Ok(())
    } else {
        Err(ManagedNixError::Subprocess {
            exit_status: status.code(),
            stderr_tail: tail.into_iter().collect::<Vec<_>>().join("\n"),
        })
    }
}

/// `/nix/nix-installer` のパスを返す (uninstall 時に使われる既定位置)
pub fn installed_binary_path() -> PathBuf {
    PathBuf::from("/nix/nix-installer")
}

/// install / uninstall に渡す planner 名を `(platform, arch)` から確定する
pub fn planner_name(
    platform: Platform,
    arch: Architecture,
) -> Result<&'static str, ManagedNixError> {
    match (platform, arch) {
        (Platform::Linux, _) => Ok("linux"),
        (Platform::MacOS, _) => Ok("macos"),
        _ => Err(ManagedNixError::UnsupportedArch {
            arch: format!("{platform}-{arch}"),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn parse_minimal_json_line() {
        let line = r#"{"level":"INFO","fields":{"message":"hello"}}"#;
        let parsed = parse_json_line(line).unwrap();
        assert_eq!(parsed.level.as_deref(), Some("INFO"));
        assert_eq!(
            parsed.fields.as_ref().and_then(|f| f.message.as_deref()),
            Some("hello")
        );
    }

    #[test]
    fn parse_line_with_spans() {
        let line = r#"{"fields":{"message":"Step"},"spans":[{"name":"create_directory"}]}"#;
        let parsed = parse_json_line(line).unwrap();
        assert_eq!(parsed.spans.as_ref().unwrap()[0].name, "create_directory");
    }

    #[test]
    fn parse_blank_line_returns_none() {
        assert!(parse_json_line("").is_none());
        assert!(parse_json_line("   ").is_none());
    }

    #[test]
    fn parse_invalid_json_returns_none() {
        assert!(parse_json_line("not json").is_none());
        assert!(parse_json_line("{ broken").is_none());
    }

    #[test]
    fn plan_args_basic() {
        let args = plan_args("linux", Path::new("/tmp/plan.json"), &[]);
        assert_eq!(args[0], "plan");
        assert_eq!(args[1], "linux");
        assert!(args.contains(&"--enable-flakes".to_string()));
        assert!(args.contains(&"--out-file".to_string()));
    }

    #[test]
    fn plan_args_with_extra_conf() {
        let args = plan_args(
            "macos",
            Path::new("/tmp/p.json"),
            &["experimental-features = flakes nix-command".to_string()],
        );
        let joined = args.join(" ");
        assert!(joined.contains("--extra-conf"));
        assert!(joined.contains("flakes nix-command"));
    }

    #[test]
    fn install_args_uses_plan_flag() {
        let args = install_args(Path::new("/tmp/plan.json"));
        // --plan と planner-subcommand は排他なので planner は無い
        assert!(args.contains(&"--plan".to_string()));
        assert!(args.contains(&"--logger".to_string()));
        assert!(args.contains(&"json".to_string()));
        assert!(args.contains(&"--enable-flakes".to_string()));
        assert!(args.contains(&"--no-confirm".to_string()));
        assert!(!args.iter().any(|a| a == "linux" || a == "macos"));
    }

    #[test]
    fn uninstall_args_no_receipt() {
        let args = uninstall_args(None);
        assert_eq!(args, vec!["uninstall", "--no-confirm"]);
    }

    #[test]
    fn uninstall_args_with_receipt() {
        let args = uninstall_args(Some(Path::new("/nix/receipt.json")));
        assert_eq!(args, vec!["uninstall", "--no-confirm", "/nix/receipt.json"]);
    }

    #[test]
    fn planner_name_linux() {
        assert_eq!(
            planner_name(Platform::Linux, Architecture::X86_64).unwrap(),
            "linux"
        );
    }

    #[test]
    fn planner_name_macos() {
        assert_eq!(
            planner_name(Platform::MacOS, Architecture::Aarch64).unwrap(),
            "macos"
        );
    }
}
