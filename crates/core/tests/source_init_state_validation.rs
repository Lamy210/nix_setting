#[cfg(unix)]
mod unix {
    use schneeforge_core::{
        source_init, Error, ResolvedTool, SourceKind, SourceState, State, StateStore, ToolSource,
    };
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn source_init_rejects_invalid_managed_state_before_remote_effects() {
        let dir = std::env::temp_dir().join(format!(
            "schneeforge-source-init-invalid-state-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let marker = dir.join("remote-invoked");
        let fake_git = dir.join("fake-git");
        std::fs::write(
            &fake_git,
            format!(
                "#!/bin/sh\ntouch '{}'\necho 'remote access must not happen' >&2\nexit 23\n",
                marker.display()
            ),
        )
        .unwrap();
        let mut permissions = std::fs::metadata(&fake_git).unwrap().permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&fake_git, permissions).unwrap();

        let store = StateStore::new(dir.join("state.json"));
        let invalid_source = SourceState {
            kind: SourceKind::ReleaseStable,
            ref_: "main".to_string(),
            channel: Some("stable".to_string()),
            managed: true,
            remote: Some("https://github.com/Lamy210/nix_setting.git".to_string()),
            revision: None,
        };
        store
            .save(&State {
                source: Some(invalid_source.clone()),
                ..State::default()
            })
            .unwrap();

        let git = ResolvedTool::new(fake_git, ToolSource::Path);
        let err = source_init(dir.to_str().unwrap(), &store, &git, None, None).unwrap_err();

        assert!(matches!(err, Error::State(_)), "{err}");
        assert!(err.to_string().contains("valid release tag"), "{err}");
        assert!(
            !marker.exists(),
            "source init must validate persisted state before remote tag discovery"
        );
        assert_eq!(
            store.load().unwrap().and_then(|state| state.source),
            Some(invalid_source),
            "source init must preserve invalid persisted state for explicit repair"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }
}
