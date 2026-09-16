use schneeforge_core::execution::{
    build_wsl_argv, validate_backend_info, validate_wsl_repo_path, BackendInfo,
    BACKEND_PROTOCOL_VERSION,
};

fn valid_backend() -> BackendInfo {
    BackendInfo {
        protocol_version: BACKEND_PROTOCOL_VERSION,
        app_version: "0.2.0-rc.7".into(),
        os: "linux".into(),
        arch: "x86_64".into(),
    }
}

#[test]
fn helper_requires_protocol_and_app_version_match() {
    let valid = valid_backend();
    assert!(validate_backend_info(&valid, "0.2.0-rc.7").is_ok());

    let mut protocol_mismatch = valid.clone();
    protocol_mismatch.protocol_version += 1;
    assert!(validate_backend_info(&protocol_mismatch, "0.2.0-rc.7").is_err());

    let mut app_mismatch = valid;
    app_mismatch.app_version = "0.2.0-rc.6".into();
    assert!(validate_backend_info(&app_mismatch, "0.2.0-rc.7").is_err());
}

#[test]
fn helper_requires_linux_and_supported_architecture() {
    let mut windows = valid_backend();
    windows.os = "windows".into();
    assert!(validate_backend_info(&windows, "0.2.0-rc.7").is_err());

    let mut unsupported_arch = valid_backend();
    unsupported_arch.arch = "riscv64".into();
    assert!(validate_backend_info(&unsupported_arch, "0.2.0-rc.7").is_err());

    let mut arm = valid_backend();
    arm.arch = "aarch64".into();
    assert!(validate_backend_info(&arm, "0.2.0-rc.7").is_ok());
}

#[test]
fn backend_info_is_machine_readable_json() {
    let json = serde_json::to_string(&valid_backend()).unwrap();
    let parsed: BackendInfo = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, valid_backend());
    assert!(serde_json::from_str::<BackendInfo>(r#"{"protocol_version":1}"#).is_err());
}

#[test]
fn repo_accepts_absolute_linux_path_only() {
    assert!(validate_wsl_repo_path("/home/alice/nix_setting").is_ok());
    assert!(validate_wsl_repo_path(r"C:\src\nix_setting").is_err());
    assert!(validate_wsl_repo_path(r"\\server\share").is_err());
    assert!(validate_wsl_repo_path("relative/path").is_err());
    assert!(validate_wsl_repo_path("").is_err());
}

#[test]
fn wsl_argv_preserves_metacharacters_as_distinct_arguments() {
    let forwarded = vec!["apply".into(), "a b;$(x)".into()];
    assert_eq!(
        build_wsl_argv("Ubuntu Dev", &forwarded),
        vec![
            "-d",
            "Ubuntu Dev",
            "--",
            "schneeforge",
            "apply",
            "a b;$(x)",
        ]
    );
}
