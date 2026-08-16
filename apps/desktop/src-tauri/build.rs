fn main() {
    stage_cli_sidecar();
    tauri_build::build();
}

// CLI sidecar (externalBin) を build 前に staging する。
// tauri.conf.json の externalBin は `binaries/schneeforge-cli-$TARGET_TRIPLE`
// を要求するため、cargo で build 済みの workspace CLI binary を
// target triple suffix 付きで copy する (tauri CLI が要する形式)。
fn stage_cli_sidecar() {
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR set by cargo");
    // OUT_DIR = <target>/<profile>/build/<pkg>-<hash>/out
    // 3 つ上ると <target>/<profile>、その親が target dir
    let mut build_pkg_dir = std::path::PathBuf::from(&out_dir);
    for _ in 0..3 {
        build_pkg_dir.pop();
    }
    let profile_dir = build_pkg_dir;
    let target_dir = profile_dir.parent().expect("target dir").to_path_buf();
    let profile = profile_dir
        .file_name()
        .and_then(|s| s.to_str())
        .expect("profile dir name");

    let cli_src = target_dir.join(profile).join("schneeforge");
    if !cli_src.exists() {
        // CLI が未 build の環境 (初回 cargo check 等) では warning のみで抜ける。
        // この場合 tauri CLI の bundle は externalBin 解決に失敗するため、
        // build script の段階では致命扱いにしない (cargo test --lib は通す)
        println!("cargo:warning=CLI sidecar source not found at {} (run cargo build -p schneeforge first)", cli_src.display());
        println!("cargo:rerun-if-changed={}", cli_src.display());
        return;
    }

    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    let dest_dir = std::path::Path::new(&manifest_dir).join("binaries");
    std::fs::create_dir_all(&dest_dir).expect("create binaries dir");

    let triple = std::env::var("TARGET").expect("TARGET set by cargo");
    let dest = dest_dir.join(format!("schneeforge-cli-{triple}"));
    std::fs::copy(&cli_src, &dest).unwrap_or_else(|e| {
        panic!(
            "copy CLI sidecar {} -> {}: {e}",
            cli_src.display(),
            dest.display()
        )
    });
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&dest)
            .expect("sidecar metadata")
            .permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&dest, perms).expect("chmod sidecar");
    }
    println!("cargo:rerun-if-changed={}", cli_src.display());
}
