#![cfg(target_os = "linux")]

use std::ffi::OsString;
use std::os::unix::ffi::{OsStrExt, OsStringExt};

use schneeforge_core::discovery::detect_target_for;
use schneeforge_core::repo::RepositoryPath;
use schneeforge_core::source::EffectiveSourceRef;

fn non_utf8_repo_path() -> std::path::PathBuf {
    std::path::PathBuf::from(OsString::from_vec(b"/tmp/schneeforge-repo-\xff".to_vec()))
}

#[test]
fn checkout_build_target_preserves_repository_bytes_before_fragment() {
    let repo = non_utf8_repo_path();
    let source = EffectiveSourceRef::Checkout(RepositoryPath::new(repo.clone()));
    let target = detect_target_for("linux", "x86_64");

    let fragment = target.build_fragment();
    assert_eq!(fragment, "#homeConfigurations.linux.activationPackage");

    let arg = source.with_fragment(&fragment);
    let mut expected = repo.as_os_str().as_bytes().to_vec();
    expected.extend_from_slice(fragment.as_bytes());
    assert_eq!(arg.as_os_str().as_bytes(), expected.as_slice());
}

#[test]
fn checkout_switch_target_preserves_repository_bytes_before_fragment() {
    let repo = non_utf8_repo_path();
    let source = EffectiveSourceRef::Checkout(RepositoryPath::new(repo.clone()));
    let target = detect_target_for("macos", "aarch64");

    let fragment = target.switch_fragment();
    assert_eq!(fragment, "#darwinConfigurations.darwin-aarch64");

    let arg = source.with_fragment(&fragment);
    let mut expected = repo.as_os_str().as_bytes().to_vec();
    expected.extend_from_slice(fragment.as_bytes());
    assert_eq!(arg.as_os_str().as_bytes(), expected.as_slice());
}

#[test]
fn managed_flake_target_stays_textual() {
    let source = EffectiveSourceRef::ManagedFlake("github:Lamy210/nix_setting/v0.2.0".to_string());
    let target = detect_target_for("linux", "x86_64");

    assert_eq!(
        source.with_fragment(&target.build_fragment()),
        OsString::from(
            "github:Lamy210/nix_setting/v0.2.0#homeConfigurations.linux.activationPackage"
        )
    );
}
