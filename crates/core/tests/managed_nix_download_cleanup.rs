use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;

use schneeforge_core::managed_nix::download;

static NEXT_TEST_DIR: AtomicU64 = AtomicU64::new(0);

fn test_dir() -> PathBuf {
    std::env::temp_dir().join(format!(
        "schneeforge-managed-nix-download-cleanup-{}-{}",
        std::process::id(),
        NEXT_TEST_DIR.fetch_add(1, Ordering::Relaxed)
    ))
}

fn serve_once(body: &'static [u8]) -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind test HTTP listener");
    let addr = listener.local_addr().expect("resolve test HTTP address");

    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept test HTTP request");
        let mut request = [0_u8; 1024];
        let _ = stream.read(&mut request);

        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        )
        .expect("write test HTTP headers");
        stream.write_all(body).expect("write test HTTP body");
    });

    (format!("http://{addr}/nix-installer"), handle)
}

#[test]
fn download_removes_temp_file_when_rename_fails() {
    let root = test_dir();
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("create test root");

    let dest = root.join("nix-installer");
    fs::create_dir(&dest).expect("create destination directory that forces rename failure");

    let (url, server) = serve_once(b"installer");
    let result = download(&url, &dest);
    server.join().expect("test HTTP server should complete");

    assert!(result.is_err(), "rename into an existing directory must fail");

    let leftovers: Vec<_> = fs::read_dir(&root)
        .expect("list test root")
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with("nix-installer.part-")
        })
        .map(|entry| entry.path())
        .collect();

    let _ = fs::remove_dir_all(&root);

    assert!(
        leftovers.is_empty(),
        "download must remove temp files after rename failure, found: {leftovers:?}"
    );
}
