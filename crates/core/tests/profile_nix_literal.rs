use std::path::PathBuf;

#[test]
fn profile_input_escapes_nix_string_metacharacters() {
    let state_root = std::env::temp_dir().join(format!(
        "schneeforge-profile-nix-literal-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&state_root);
    std::fs::create_dir_all(&state_root).unwrap();
    std::env::set_var("XDG_STATE_HOME", &state_root);

    let profile = "dev\"${builtins.abort \"boom\"}\\ops";
    let path = schneeforge_core::write_profile_input(profile).unwrap();
    let content = std::fs::read_to_string(&path).unwrap();

    assert_eq!(
        content,
        "{ profile = \"dev\\\"\\${builtins.abort \\\"boom\\\"}\\\\ops\"; }\n",
        "generated profile.nix must encode the profile name as a literal Nix string"
    );
    assert_eq!(
        path,
        PathBuf::from(&state_root).join("schneeforge/profile.nix")
    );

    let _ = std::fs::remove_dir_all(&state_root);
}
