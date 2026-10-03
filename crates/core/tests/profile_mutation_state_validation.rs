use schneeforge_core::{profile, Error, SourceKind, SourceState, State, StateStore};

fn setup_invalid_store(name: &str) -> (StateStore, std::path::PathBuf) {
    let dir = std::env::temp_dir().join(format!(
        "schneeforge-profile-invalid-state-{name}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let store = StateStore::new(dir.join("state.json"));
    store
        .save(&State {
            source: Some(SourceState {
                kind: SourceKind::ReleaseStable,
                ref_: "main".to_string(),
                channel: Some("stable".to_string()),
                managed: true,
                remote: Some("https://github.com/Lamy210/nix_setting.git".to_string()),
                revision: None,
            }),
            profile: Some("developer".to_string()),
            ..State::default()
        })
        .unwrap();
    (store, dir)
}

fn assert_state_error_preserves_file(
    store: &StateStore,
    before: &str,
    result: schneeforge_core::Result<()>,
) {
    let err = result.unwrap_err();
    assert!(matches!(err, Error::State(_)), "{err}");
    assert!(err.to_string().contains("valid release tag"), "{err}");
    assert_eq!(
        std::fs::read_to_string(store.path()).unwrap(),
        before,
        "profile mutation must not overwrite semantically invalid state"
    );
}

#[test]
fn save_selection_rejects_invalid_managed_state_without_overwriting_it() {
    let (store, dir) = setup_invalid_store("save");
    let before = std::fs::read_to_string(store.path()).unwrap();

    assert_state_error_preserves_file(
        &store,
        &before,
        profile::save_selection_with(&store, "minimal"),
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn clear_selection_rejects_invalid_managed_state_without_overwriting_it() {
    let (store, dir) = setup_invalid_store("clear");
    let before = std::fs::read_to_string(store.path()).unwrap();

    assert_state_error_preserves_file(&store, &before, profile::clear_selection_with(&store));

    let _ = std::fs::remove_dir_all(&dir);
}
