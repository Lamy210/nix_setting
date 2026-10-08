#![cfg(unix)]

use std::ffi::OsString;
use std::fs;
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use schneeforge_core::{BootstrapManifest, ManagedNix, NoProgress};

fn managed_nix() -> ManagedNix {
    let manifest = BootstrapManifest::parse(
        r#"
[managed_nix]
version = "2.35.1"

[managed_nix.sha256_by_arch]
x86_64-linux = "1111111111111111111111111111111111111111111111111111111111111111"
aarch64-linux = "2222222222222222222222222222222222222222222222222222222222222222"
aarch64-darwin = "3333333333333333333333333333333333333333333333333333333333333333"
"#,
    )
    .unwrap();
    ManagedNix::from_manifest(manifest)
}

fn non_utf8_path(prefix: &Path, name: &[u8]) -> PathBuf {
    let mut bytes = prefix.as_os_str().as_bytes().to_vec();
    bytes.push(b'/');
    bytes.extend_from_slice(name);
    PathBuf::from(OsString::from_vec(bytes))
}

fn probe_script(dir: &Path, output: &Path) -> PathBuf {
    let script = dir.join("argv-probe.sh");
    fs::write(
        &script,
        format!(
            "#!/bin/sh\ncase \"$1\" in\n  install) printf '%s' \"$2\" > '{}' ;;\n  uninstall) printf '%s' \"$3\" > '{}' ;;\nesac\n",
            output.display(),
            output.display()
        ),
    )
    .unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o700)).unwrap();
    script
}

#[test]
fn run_install_preserves_non_utf8_plan_path_bytes() {
    let root = std::env::temp_dir().join(format!(
        "schneeforge-installer-non-utf8-install-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    let captured = root.join("captured-install-arg");
    let binary = probe_script(&root, &captured);
    let plan = non_utf8_path(&root, b"plan-\xff.json");

    managed_nix()
        .run_install(&binary, &plan, &mut NoProgress)
        .unwrap();

    assert_eq!(
        fs::read(&captured).unwrap(),
        plan.as_os_str().as_bytes(),
        "installer must receive the exact filesystem bytes for the plan path"
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn run_uninstall_preserves_non_utf8_receipt_path_bytes() {
    let root = std::env::temp_dir().join(format!(
        "schneeforge-installer-non-utf8-uninstall-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    let captured = root.join("captured-uninstall-arg");
    let binary = probe_script(&root, &captured);
    let receipt = non_utf8_path(&root, b"receipt-\xff.json");

    managed_nix()
        .run_uninstall(&binary, Some(&receipt))
        .unwrap();

    assert_eq!(
        fs::read(&captured).unwrap(),
        receipt.as_os_str().as_bytes(),
        "installer must receive the exact filesystem bytes for the receipt path"
    );
    let _ = fs::remove_dir_all(&root);
}
