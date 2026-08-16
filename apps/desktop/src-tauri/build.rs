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
    let profile = profile_dir
        .file_name()
        .and_then(|s| s.to_str())
        .expect("profile dir name")
        .to_string();

    let cli_src = match find_cli_binary(&profile_dir, &profile) {
        Some(p) => p,
        None => {
            // CLI が未 build の環境 (初回 cargo check 等) では warning のみで抜ける。
            // この場合 tauri CLI の bundle は externalBin 解決に失敗するため、
            // build script の段階では致命扱いにしない (cargo test --lib は通す)
            println!(
                "cargo:warning=CLI sidecar source not found in {} or <repo>/target/{profile} (run `cargo build -p schneeforge` first)",
                profile_dir.display()
            );
            return;
        }
    };

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

// build 済み CLI binary の所在解決。
// desktop は root workspace とは分離した workspace のため、CLI は通常
// root 側の `cargo build -p schneeforge` で <repo>/target/<profile>/ に
// 置かれる (DMG build script もこの形式)。同一 target dir を使う運用
// (CARGO_TARGET_DIR 共有 / --target-dir 指定) も候補に含める。
// CARGO_MANIFEST_DIR = <repo>/apps/desktop/src-tauri なので root は 3 つ上。
fn find_cli_binary(profile_dir: &std::path::Path, profile: &str) -> Option<std::path::PathBuf> {
    let own = profile_dir.join("schneeforge");
    if own.is_file() {
        return Some(own);
    }
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let root_target = std::path::Path::new(&manifest_dir)
            .join("../../../target")
            .join(profile)
            .join("schneeforge");
        if root_target.is_file() {
            return Some(root_target);
        }
    }
    None
}
