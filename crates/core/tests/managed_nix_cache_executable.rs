#![cfg(target_os = "linux")]

use schneeforge_core::managed_nix::{
    cache_path, is_root, sha256_hex, BootstrapManifest, ManagedNix,
};
use schneeforge_core::{Architecture, Platform};
use std::os::unix::fs::PermissionsExt;
use std::process::Command;

const CHILD_CASE: &str = "SCHNEEFORGE_MANAGED_NIX_CACHE_EXEC_CHILD";

#[test]
fn cached_installer_restores_owner_execute_permission() {
    if is_root() {
        eprintln!("skipping non-root cache regression while running as root");
        return;
    }

    if std::env::var(CHILD_CASE).as_deref() == Ok("non-executable-cache") {
        let version = "9.9.9";
        let cache = cache_path(version).expect("resolve isolated cache path");
        std::fs::create_dir_all(cache.parent().expect("cache parent"))
            .expect("create isolated cache directory");
        std::fs::write(&cache, b"cached installer body").expect("write cached installer");
        std::fs::set_permissions(&cache, std::fs::Permissions::from_mode(0o400))
            .expect("make cached installer non-executable");

        let sha = sha256_hex(&cache).expect("hash cached installer");
        let manifest = BootstrapManifest::parse(&format!(
            r#"
[managed_nix]
version = "{version}"

[managed_nix.sha256_by_arch]
x86_64-linux = "{sha}"
aarch64-linux = "{sha}"
aarch64-darwin = "{sha}"
"#
        ))
        .expect("parse test bootstrap manifest");
        let managed = ManagedNix::from_manifest(manifest);

        let (resolved, _) = managed
            .fetch_binary(Platform::Linux, Architecture::X86_64)
            .expect("reuse checksum-valid cached installer");
        let mode = std::fs::metadata(&resolved)
            .expect("stat reused cached installer")
            .permissions()
            .mode();
        assert_ne!(
            mode & 0o100,
            0,
            "checksum-valid cached installer must be executable before reuse"
        );
        return;
    }

    let data_root = std::env::temp_dir().join(format!(
        "schneeforge-managed-nix-cache-exec-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&data_root);
    std::fs::create_dir_all(&data_root).expect("create isolated XDG data root");

    let status = Command::new(std::env::current_exe().expect("current test executable"))
        .args([
            "--exact",
            "cached_installer_restores_owner_execute_permission",
            "--nocapture",
        ])
        .env(CHILD_CASE, "non-executable-cache")
        .env("XDG_DATA_HOME", &data_root)
        .status()
        .expect("spawn isolated Managed Nix cache regression test");

    let _ = std::fs::remove_dir_all(&data_root);
    assert!(
        status.success(),
        "cache reuse must restore owner execute permission"
    );
}
