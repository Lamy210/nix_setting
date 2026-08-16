//! GUI 用 privilege escalation helper — design.md D4 Phase 2
//!
//! GUI process 自身は root にならず、SchneeForge 自身の CLI を管理者権限で
//! 再実行する command を構築する。macOS は osascript、Linux は pkexec。
//! 実行対象は SchneeForge binary + `nix install` 固定とし、shell 文字列の
//! 組み立ては本 module が escape を担う (任意 command の実行を許さない)。

use std::path::{Path, PathBuf};

use crate::managed_nix::error::ManagedNixError;

/// escalation 先で実行する SchneeForge 操作。
/// `--yes` を付ける確認責任は caller (GUI の確認 UI) 側にある (D8)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EscalatedOp {
    /// `schneeforge nix install --yes` (GUI の最終確認済みを前提)
    NixInstall,
}

impl EscalatedOp {
    /// SchneeForge CLI の引数列
    fn cli_args(&self) -> Vec<String> {
        match self {
            EscalatedOp::NixInstall => vec![
                "nix".to_string(),
                "install".to_string(),
                "--yes".to_string(),
            ],
        }
    }
}

/// Linux: pkexec 経由で GUI 表示に必要な環境変数を引き継ぐ対象。
/// X 越えの昇格先 process で認証 dialog を出すために必要。
const PKEXEC_PASSTHROUGH_ENV: &[&str] = &[
    "DISPLAY",
    "XAUTHORITY",
    "WAYLAND_DISPLAY",
    "DBUS_SESSION_BUS_ADDRESS",
];

/// POSIX shell (osascript の `do shell script` は /bin/sh) 向けの
/// single-quote escape。`'` → `'\''`。
fn sh_quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('\'');
    for c in s.chars() {
        if c == '\'' {
            out.push_str("'\\''");
        } else {
            out.push(c);
        }
    }
    out.push('\'');
    out
}

/// GUI の昇格に使う環境変数を現在の process から収集する (Linux pkexec 用)。
/// 値はそのまま env(1) の引数に渡すため shell は介さない (escape 不要)。
fn collect_gui_env() -> Vec<(String, String)> {
    PKEXEC_PASSTHROUGH_ENV
        .iter()
        .filter_map(|k| {
            std::env::var(k)
                .ok()
                .filter(|v| !v.is_empty())
                .map(|v| (k.to_string(), v))
        })
        .collect()
}

/// macOS: `osascript -e 'do shell script "<cmd>" with administrator privileges'`
/// の引数列を構築する。
///
/// `do shell script` は /bin/sh -c で文字列を実行するため、埋め込む command
/// 全体を sh_quote した上で AppleScript の文字列 literal へ escape する
/// (AppleScript では `"` と `\` を `\` escape)。
fn osascript_args(schneeforge_bin: &Path, op: EscalatedOp) -> Vec<String> {
    let mut cmd = sh_quote(&schneeforge_bin.to_string_lossy());
    for a in op.cli_args() {
        cmd.push(' ');
        cmd.push_str(&sh_quote(&a));
    }
    let applescript = format!(
        "do shell script {} with administrator privileges",
        applescript_quote(&cmd)
    );
    vec!["-e".to_string(), applescript]
}

/// AppleScript の string literal 向け escape (`"` と `\`)
fn applescript_quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        if c == '"' || c == '\\' {
            out.push('\\');
        }
        out.push(c);
    }
    out.push('"');
    out
}

/// Linux: `pkexec env KEY=VALUE… <schneeforge> nix install --yes` の引数列を
/// 構築する。env(1) を挟むことで pkexec の sanitize で落ちる環境変数を
/// 明示的に引き継ぐ。値は引数として渡すため shell escape は不要。
fn pkexec_args(schneeforge_bin: &Path, op: EscalatedOp, env: &[(String, String)]) -> Vec<String> {
    let mut args = vec!["env".to_string()];
    for (k, v) in env {
        args.push(format!("{k}={v}"));
    }
    args.push(schneeforge_bin.to_string_lossy().into_owned());
    args.extend(op.cli_args());
    args
}

/// platform 毎の昇格実行 command (program + args) を構築する。
///
/// 戻り値は `(program, args)`。caller はこれをそのまま `Command::new(program)`
/// へ渡す (shell を介さないため、この引数列以外の注入面は無い)。
pub fn escalate_command(
    schneeforge_bin: &Path,
    op: EscalatedOp,
) -> Result<(PathBuf, Vec<String>), ManagedNixError> {
    if schneeforge_bin.as_os_str().is_empty() {
        return Err(ManagedNixError::Io {
            context: "resolve schneeforge binary for escalation".to_string(),
            source: "empty binary path".to_string(),
        });
    }
    if cfg!(target_os = "macos") {
        Ok((
            PathBuf::from("/usr/bin/osascript"),
            osascript_args(schneeforge_bin, op),
        ))
    } else if cfg!(target_os = "linux") {
        Ok((
            PathBuf::from("pkexec"),
            pkexec_args(schneeforge_bin, op, &collect_gui_env()),
        ))
    } else {
        Err(ManagedNixError::UnsupportedArch {
            arch: format!("escalation on {}", std::env::consts::OS),
        })
    }
}

/// 自分自身の binary path (escalation 再実行の対象)。
/// `current_exe` は symlink を resolve するため .app bundle 内の実体を指す。
pub fn self_binary_path() -> Result<PathBuf, ManagedNixError> {
    std::env::current_exe().map_err(|e| ManagedNixError::Io {
        context: "resolve current executable".to_string(),
        source: e.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sh_quote_wraps_in_single_quotes() {
        assert_eq!(sh_quote("nix install"), "'nix install'");
        assert_eq!(sh_quote("it's"), "'it'\\''s'");
    }

    #[test]
    fn applescript_quote_escapes_backslash_and_quote() {
        assert_eq!(applescript_quote("a\"b"), "\"a\\\"b\"");
        assert_eq!(applescript_quote("a\\b"), "\"a\\\\b\"");
    }

    #[test]
    fn osascript_args_shape() {
        let args = osascript_args(
            Path::new("/usr/local/bin/schneeforge"),
            EscalatedOp::NixInstall,
        );
        assert_eq!(args[0], "-e");
        let script = &args[1];
        // AppleScript string literal ("…") で全体が wrap される
        assert!(script.starts_with("do shell script \""), "got: {script}");
        assert!(script.ends_with("\" with administrator privileges"));
        // SchneeForge binary と引数が sh_quote されて埋め込まれている
        assert!(
            script.contains("'/usr/local/bin/schneeforge'"),
            "got: {script}"
        );
        assert!(script.contains("'nix' 'install' '--yes'"), "got: {script}");
    }

    #[test]
    fn osascript_args_escape_binary_path_with_quote() {
        // path に single quote が入っていても shell injection にならない。
        // sh_quote の `'\''` に含まれる `\` が AppleScript で `\\` に二重 escape される
        let args = osascript_args(Path::new("/opt/sch'neeforge"), EscalatedOp::NixInstall);
        let script = &args[1];
        assert!(
            script.contains("'/opt/sch'\\\\''neeforge'"),
            "quote must be escaped: {script}"
        );
    }

    #[test]
    fn pkexec_args_shape_with_env() {
        let env = vec![
            ("DISPLAY".to_string(), ":0".to_string()),
            (
                "XAUTHORITY".to_string(),
                "/run/user/1000/gdm/Xauthority".to_string(),
            ),
        ];
        let args = pkexec_args(
            Path::new("/usr/bin/schneeforge"),
            EscalatedOp::NixInstall,
            &env,
        );
        assert_eq!(args[0], "env");
        assert_eq!(args[1], "DISPLAY=:0");
        assert_eq!(args[2], "XAUTHORITY=/run/user/1000/gdm/Xauthority");
        assert_eq!(args[3], "/usr/bin/schneeforge");
        assert_eq!(&args[4..], &["nix", "install", "--yes"]);
    }

    #[test]
    fn pkexec_args_without_env_omits_env_block() {
        let args = pkexec_args(
            Path::new("/usr/bin/schneeforge"),
            EscalatedOp::NixInstall,
            &[],
        );
        assert_eq!(args[0], "env");
        assert_eq!(args[1], "/usr/bin/schneeforge");
        assert_eq!(&args[2..], &["nix", "install", "--yes"]);
    }

    #[test]
    fn escalated_op_is_fixed_to_nix_install_yes() {
        assert_eq!(
            EscalatedOp::NixInstall.cli_args(),
            vec!["nix", "install", "--yes"]
        );
    }

    #[test]
    fn escalate_command_rejects_empty_binary() {
        assert!(escalate_command(Path::new(""), EscalatedOp::NixInstall).is_err());
    }

    #[test]
    fn pkexec_passthrough_env_are_known_good() {
        // X / Wayland / D-BS のみ。PATH 等の汎用 env は pkexec 側の既定に任せる
        for k in PKEXEC_PASSTHROUGH_ENV {
            assert!(!k.contains('='), "env key must be a bare name: {k}");
        }
    }
}
