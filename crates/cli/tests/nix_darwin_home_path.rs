const NIX_CMD_SOURCE: &str = include_str!("../src/nix_cmd.rs");

#[test]
fn nix_darwin_home_is_read_without_utf8_coercion() {
    assert!(
        NIX_CMD_SOURCE.contains(r#"std::env::var_os("HOME")"#),
        "HOME is a filesystem path and must be read as OsString"
    );
    assert!(
        !NIX_CMD_SOURCE.contains(r#"std::env::var("HOME")"#),
        "nix-darwin HOME marker lookup must not force HOME through UTF-8"
    );
}

#[cfg(unix)]
#[test]
fn non_utf8_home_path_can_address_nix_darwin_marker() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    let mut leaf = format!("sf-nix-darwin-home-{}-", std::process::id()).into_bytes();
    leaf.push(0xff);

    let home = std::env::temp_dir().join(OsString::from_vec(leaf));
    let marker = home.join(".nix-darwin");
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&marker).expect("create non-UTF-8 nix-darwin marker path");

    assert!(marker.exists());

    let _ = std::fs::remove_dir_all(&home);
}
