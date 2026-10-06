use schneeforge_core::{verify, ToolInventory};
use std::process::Command;

const CHILD_CASE: &str = "SCHNEEFORGE_VERIFY_RELATIVE_HOME_CHILD";

#[test]
fn verify_ignores_relative_home_for_dotfile_checks() {
    if std::env::var_os(CHILD_CASE).is_some() {
        let report = verify(".", &ToolInventory::default());
        for name in [".zshrc", ".gitconfig", "starship.toml"] {
            let check = report
                .checks
                .iter()
                .find(|check| check.name == name)
                .unwrap_or_else(|| panic!("missing verify check: {name}"));
            assert!(
                !check.ok,
                "verify must not resolve {name} through a relative HOME"
            );
        }
        return;
    }

    let root = std::env::temp_dir().join(format!(
        "schneeforge-verify-relative-home-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    let work = root.join("work");
    let relative_home = work.join("relative-home");
    let state_root = root.join("state");
    std::fs::create_dir_all(relative_home.join(".config")).expect("create fake relative HOME");
    std::fs::create_dir_all(&state_root).expect("create isolated state root");
    std::fs::write(relative_home.join(".zshrc"), "# cwd-relative decoy\n")
        .expect("write .zshrc decoy");
    std::fs::write(relative_home.join(".gitconfig"), "[user]\n\tname = decoy\n")
        .expect("write .gitconfig decoy");
    std::fs::write(
        relative_home.join(".config/starship.toml"),
        "# cwd-relative decoy\n",
    )
    .expect("write starship decoy");

    let status = Command::new(std::env::current_exe().expect("current test executable"))
        .args([
            "--exact",
            "verify_ignores_relative_home_for_dotfile_checks",
            "--nocapture",
        ])
        .current_dir(&work)
        .env(CHILD_CASE, "1")
        .env("HOME", "relative-home")
        .env("XDG_STATE_HOME", &state_root)
        .status()
        .expect("run isolated verify child");

    let _ = std::fs::remove_dir_all(&root);
    assert!(
        status.success(),
        "relative HOME decoys must not satisfy verify dotfile checks"
    );
}
