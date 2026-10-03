use schneeforge_core::{repo_url, DEFAULT_REPO_URL};

#[test]
fn empty_repository_url_override_falls_back_to_default() {
    // This integration test is its own test process, so mutating the process
    // environment cannot race with unit tests in other test binaries.
    unsafe {
        std::env::set_var("SCHNEEFORGE_REPO_URL", "");
    }

    assert_eq!(repo_url(), DEFAULT_REPO_URL);
}
