use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn check_workflow_groups_pr_runs_by_pull_request_number() {
    let path = repo_root().join(".github/workflows/check.yml");
    let workflow = fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("read {}: {err}", path.display()));

    assert!(
        workflow.contains(
            r#"group: check-pr-${{ github.event.pull_request.number || github.ref }}"#,
        ),
        "check workflow must use a PR-number-stable concurrency group so a close event can cancel the same group"
    );
}

#[test]
fn closed_pr_workflow_cancels_the_shared_group_without_expensive_work() {
    let path = repo_root().join(".github/workflows/pr-close-cancel.yml");
    let workflow = fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("read {}: {err}", path.display()));

    for required in [
        "types: [closed]",
        "branches: [main, develop]",
        r#"group: check-pr-${{ github.event.pull_request.number }}"#,
        "cancel-in-progress: true",
        "runs-on: ubuntu-latest",
    ] {
        assert!(
            workflow.contains(required),
            "closed-PR cancellation workflow must contain {required:?}"
        );
    }

    for forbidden in ["macos-", "cachix/", "cargo ", "nix "] {
        assert!(
            !workflow.contains(forbidden),
            "closed-PR cancellation workflow must stay lightweight; found {forbidden:?}"
        );
    }
}
