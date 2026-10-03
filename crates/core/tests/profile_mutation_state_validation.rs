use schneeforge_core::{profile, Error, SourceKind, SourceState, State, StateStore};

fn invalid_managed_state() -> State {
    State {
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
    }
}

#[test]
fn save_selection_rejects_invalid_managed_state_without_overwriting_it() {
    let dir = std::env::temp_dir().join(format!(
        "schneeforge-profile-invalid-state-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let store = StateStore::new(dir.join("state.json"));
    store.save(&invalid_managed_state()).unwrap();
    let before = std::fs::read_to_string(store.path()).unwrap();

    let err = profile::save_selection_with(&store, "minimal").unwrap_err();

    assert!(matches!(err, Error::State(_)), "{err}");
    assert!(err.to_string().contains("valid release tag"), "{err}");
    assert_eq!(
        std::fs::read_to_string(store.path()).unwrap(),
        before,
        "profile mutation must not overwrite semantically invalid state"
    );

    let _ = std::fs::remove_dir_all(&dir);
}
