const DESKTOP_BUILD_SCRIPT: &str = include_str!("../../../apps/desktop/src-tauri/build.rs");

#[test]
fn desktop_build_script_preserves_filesystem_path_env_identity() {
    assert!(
        DESKTOP_BUILD_SCRIPT.contains(r#"std::env::var_os("OUT_DIR")"#),
        "OUT_DIR is a filesystem path and must be read as OsString"
    );
    assert!(
        DESKTOP_BUILD_SCRIPT.contains(r#"std::env::var_os("CARGO_MANIFEST_DIR")"#),
        "CARGO_MANIFEST_DIR is a filesystem path and must be read as OsString"
    );
    assert!(
        !DESKTOP_BUILD_SCRIPT.contains(r#"std::env::var("OUT_DIR")"#),
        "OUT_DIR must not be forced through UTF-8"
    );
    assert!(
        !DESKTOP_BUILD_SCRIPT.contains(r#"std::env::var("CARGO_MANIFEST_DIR")"#),
        "CARGO_MANIFEST_DIR must not be forced through UTF-8"
    );
    assert!(
        DESKTOP_BUILD_SCRIPT.contains(r#"std::env::var("TARGET")"#),
        "TARGET is textual target-triple data and should remain a String"
    );
}
