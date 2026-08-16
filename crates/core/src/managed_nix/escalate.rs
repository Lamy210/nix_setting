//! GUI 用 privilege escalation helper — design.md D4 Phase 2
//!
//! GUI process 自身は root にならず、SchneeForge CLI binary を管理者権限で
//! 再実行する command を構築する。macOS は osascript、Linux は pkexec。
//! 実行対象は SchneeForge の CLI binary + `nix install` 固定とし、shell 文字列の
//! 組み立ては本 module が escape を担う (任意 command の実行を許さない)。
//! なお昇格対象は呼び出し側が解決した **CLI** binary であること — GUI 自身
//! (`current_exe`) は CLI 引数を解釈しないため昇格先として使えない。

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

/// escalation 先に引き継ぐ環境変数。`NIX_SETTING_DIR` は CLI が
/// `bootstrap-manifest.toml` を解決するために必須 (root 環境では HOME が
/// 変わるため user の repo 位置が見えなくなる)。
fn base_env(repo_dir: &Path) -> Vec<(String, String)> {
    vec![(
        "NIX_SETTING_DIR".to_string(),
        repo_dir.to_string_lossy().into_owned(),
    )]
}

/// Linux: pkexec 経由で GUI 表示に必要な環境変数を引き継ぐ対象。
/// X 越えの昇格先 process で認証 dialog を出すために必要。
const PKEXEC_GUI_ENV: &[&str] = &[
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
fn collect_gui_env() -> Vec<(String, String)> {
    PKEXEC_GUI_ENV
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
/// (AppleScript では `"` と `\` を `\` escape)。環境変数は `export K=V;` prefix
/// で先頭に置く (値も sh_quote する)。
fn osascript_args(cli_bin: &Path, op: EscalatedOp, env: &[(String, String)]) -> Vec<String> {
    let mut cmd = String::new();
    for (k, v) in env {
        cmd.push_str(&format!("export {}={}; ", sh_quote(k), sh_quote(v)));
    }
    cmd.push_str(&sh_quote(&cli_bin.to_string_lossy()));
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

/// Linux: `pkexec env KEY=VALUE… <cli> nix install --yes` の引数列を構築する。
/// env(1) を挟むことで pkexec の sanitize で落ちる環境変数を明示的に引き継ぐ。
/// 値は引数として渡すため shell escape は不要。
fn pkexec_args(cli_bin: &Path, op: EscalatedOp, env: &[(String, String)]) -> Vec<String> {
    let mut args = vec!["env".to_string()];
    for (k, v) in env {
        args.push(format!("{k}={v}"));
    }
    args.push(cli_bin.to_string_lossy().into_owned());
    args.extend(op.cli_args());
    args
}

/// platform 毎の昇格実行 command (program + args) を構築する。
///
/// `cli_bin` は SchneeForge の **CLI** binary (GUI ではない)。caller はこれを
/// そのまま `Command::new(program)` へ渡す (shell を介さないため、この引数列
/// 以外の注入面は無い)。
pub fn escalate_command(
    cli_bin: &Path,
    op: EscalatedOp,
    repo_dir: &Path,
) -> Result<(PathBuf, Vec<String>), ManagedNixError> {
    if cli_bin.as_os_str().is_empty() {
        return Err(ManagedNixError::Io {
            context: "resolve schneeforge CLI binary for escalation".to_string(),
            source: "empty binary path".to_string(),
        });
    }
    if repo_dir.as_os_str().is_empty() {
        return Err(ManagedNixError::Io {
            context: "resolve repo dir for escalation".to_string(),
            source: "empty repo dir".to_string(),
        });
    }
    let mut env = base_env(repo_dir);
    if cfg!(target_os = "linux") {
        env.extend(collect_gui_env());
    }

    if cfg!(target_os = "macos") {
        Ok((
            PathBuf::from("/usr/bin/osascript"),
            osascript_args(cli_bin, op, &env),
        ))
    } else if cfg!(target_os = "linux") {
        // pkexec は絶対 path で呼ぶ (GUI の PATH 汚染の影響を受けない)
        Ok((
            PathBuf::from("/usr/bin/pkexec"),
            pkexec_args(cli_bin, op, &env),
        ))
    } else {
        Err(ManagedNixError::UnsupportedArch {
            arch: format!("escalation on {}", std::env::consts::OS),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const REPO: &str = "/Users/foo/nix_setting";

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
            &base_env(Path::new(REPO)),
        );
        assert_eq!(args[0], "-e");
        let script = &args[1];
        // AppleScript string literal ("…") で全体が wrap される
        assert!(script.starts_with("do shell script \""), "got: {script}");
        assert!(script.ends_with("\" with administrator privileges"));
        // CLI binary と引数が sh_quote されて埋め込まれている
        assert!(
            script.contains("'/usr/local/bin/schneeforge'"),
            "got: {script}"
        );
        assert!(script.contains("'nix' 'install' '--yes'"), "got: {script}");
    }

    #[test]
    fn osascript_args_pass_repo_dir_via_export() {
        let args = osascript_args(
            Path::new("/usr/local/bin/schneeforge"),
            EscalatedOp::NixInstall,
            &base_env(Path::new(REPO)),
        );
        let script = &args[1];
        assert!(
            script.contains("export 'NIX_SETTING_DIR'='/Users/foo/nix_setting';"),
            "repo dir must be exported before the command: {script}"
        );
        // export は command より前に来る
        let export_at = script.find("export ").unwrap();
        let cmd_at = script.find("'/usr/local/bin/schneeforge'").unwrap();
        assert!(export_at < cmd_at);
    }

    #[test]
    fn osascript_args_escape_binary_path_with_quote() {
        // path に single quote が入っていても shell injection にならない。
        // sh_quote の `'\''` に含まれる `\` が AppleScript で `\\` に二重 escape される
        let args = osascript_args(
            Path::new("/opt/sch'neeforge"),
            EscalatedOp::NixInstall,
            &base_env(Path::new(REPO)),
        );
        let script = &args[1];
        assert!(
            script.contains("'/opt/sch'\\\\''neeforge'"),
            "quote must be escaped: {script}"
        );
    }

    #[test]
    fn pkexec_args_shape_with_env() {
        let mut env = base_env(Path::new(REPO));
        env.push(("DISPLAY".to_string(), ":0".to_string()));
        env.push((
            "XAUTHORITY".to_string(),
            "/run/user/1000/gdm/Xauthority".to_string(),
        ));
        let args = pkexec_args(
            Path::new("/usr/bin/schneeforge"),
            EscalatedOp::NixInstall,
            &env,
        );
        assert_eq!(args[0], "env");
        assert_eq!(args[1], "NIX_SETTING_DIR=/Users/foo/nix_setting");
        assert_eq!(args[2], "DISPLAY=:0");
        assert_eq!(args[3], "XAUTHORITY=/run/user/1000/gdm/Xauthority");
        assert_eq!(args[4], "/usr/bin/schneeforge");
        assert_eq!(&args[5..], &["nix", "install", "--yes"]);
    }

    #[test]
    fn pkexec_args_without_gui_env_still_has_repo_dir() {
        let args = pkexec_args(
            Path::new("/usr/bin/schneeforge"),
            EscalatedOp::NixInstall,
            &base_env(Path::new(REPO)),
        );
        assert_eq!(args[0], "env");
        assert_eq!(args[1], "NIX_SETTING_DIR=/Users/foo/nix_setting");
        assert_eq!(args[2], "/usr/bin/schneeforge");
        assert_eq!(&args[3..], &["nix", "install", "--yes"]);
    }

    #[test]
    fn escalated_op_is_fixed_to_nix_install_yes() {
        assert_eq!(
            EscalatedOp::NixInstall.cli_args(),
            vec!["nix", "install", "--yes"]
        );
    }

    #[test]
    fn escalate_command_rejects_empty_binary_or_repo() {
        assert!(escalate_command(Path::new(""), EscalatedOp::NixInstall, Path::new(REPO)).is_err());
        assert!(escalate_command(
            Path::new("/usr/bin/schneeforge"),
            EscalatedOp::NixInstall,
            Path::new("")
        )
        .is_err());
    }

    #[test]
    fn pkexec_gui_env_are_known_good() {
        // X / Wayland / D-BS のみ。PATH 等の汎用 env は pkexec 側の既定に任せる
        for k in PKEXEC_GUI_ENV {
            assert!(!k.contains('='), "env key must be a bare name: {k}");
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn escalate_command_uses_absolute_pkexec_path() {
        let (program, _) = escalate_command(
            Path::new("/usr/bin/schneeforge"),
            EscalatedOp::NixInstall,
            Path::new(REPO),
        )
        .expect("linux escalation");
        assert_eq!(program, PathBuf::from("/usr/bin/pkexec"));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn escalate_command_uses_absolute_osascript_path() {
        let (program, _) = escalate_command(
            Path::new("/usr/local/bin/schneeforge"),
            EscalatedOp::NixInstall,
            Path::new(REPO),
        )
        .expect("macos escalation");
        assert_eq!(program, PathBuf::from("/usr/bin/osascript"));
    }
}
