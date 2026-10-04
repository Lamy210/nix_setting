use schneeforge_core::{profile, Error, StateStore};

#[test]
fn resolve_rejects_manifest_default_outside_available_profiles() {
    let root = std::env::temp_dir().join(format!(
        "schneeforge-profile-default-validation-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join("schneeforge.toml"),
        r#"
schema = 1
[profiles]
default = "developer"
available = ["minimal"]
[systems]
x86_64-linux = true
"#,
    )
    .unwrap();

    let store = StateStore::new(root.join("state.json"));
    let err = profile::resolve_with(root.to_string_lossy().as_ref(), &store).unwrap_err();

    assert!(matches!(err, Error::Manifest(_)), "{err}");
    assert!(err.to_string().contains("default profile"), "{err}");
    assert!(err.to_string().contains("available"), "{err}");

    let _ = std::fs::remove_dir_all(&root);
}
