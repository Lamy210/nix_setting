//! Download contract checks against a local HTTP server (no public network).
//! The real cross-platform release assets are verified in distribution CI.
use schneeforge_core::managed_nix::{download, download_text, sha256_hex, verify_file};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;

static TEST_ID: AtomicU64 = AtomicU64::new(0);

struct Scratch {
    dir: PathBuf,
}

impl Scratch {
    fn new(label: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "schneeforge-download-integration-{}-{label}-{}",
            std::process::id(),
            TEST_ID.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&dir).expect("temporary directory");
        Self { dir }
    }

    fn file(&self) -> PathBuf {
        self.dir.join("cache/nix-installer")
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.dir).ok();
    }
}

fn mock_http(response: Vec<u8>, test: impl FnOnce(String)) {
    let server = TcpListener::bind(("127.0.0.1", 0)).expect("loopback listener");
    let addr = server.local_addr().unwrap();
    let worker = thread::spawn(move || {
        let (mut conn, _) = server.accept().expect("single HTTP client");
        let mut buf = [0_u8; 4096];
        let _ = conn.read(&mut buf);
        conn.write_all(&response).expect("send fixture response");
        conn.flush().unwrap();
    });
    test(format!("http://{addr}/nix-installer"));
    worker.join().expect("mock server thread");
}

fn http_response(status: &str, announced_size: usize, body: &[u8]) -> Vec<u8> {
    let mut response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/octet-stream\r\nContent-Length: {announced_size}\r\nConnection: close\r\n\r\n"
    )
    .into_bytes();
    response.extend_from_slice(body);
    response
}

#[test]
fn full_download_has_exact_size_digest_and_executable_permissions() {
    let scratch = Scratch::new("complete");
    let body = b"managed-nix-fixture-complete";
    mock_http(http_response("200 OK", body.len(), body), |url| {
        download(&url, &scratch.file()).expect("download must complete");
        assert_eq!(std::fs::read(scratch.file()).unwrap(), body);
        // A file digest must be computed from the bytes really downloaded.
        let actual = sha256_hex(&scratch.file()).expect("sha256");
        assert_eq!(actual.len(), 64);
        assert!(verify_file(&scratch.file(), &actual).is_ok());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_ne!(
                std::fs::metadata(scratch.file())
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o100,
                0
            );
        }
    });
}

#[test]
fn http_not_found_never_creates_a_finished_file() {
    let scratch = Scratch::new("404");
    mock_http(http_response("404 Not Found", 0, b""), |url| {
        assert!(download(&url, &scratch.file()).is_err());
        assert!(!scratch.file().exists());
        assert_eq!(
            std::fs::read_dir(scratch.file().parent().unwrap())
                .unwrap()
                .count(),
            0,
            "failed transfer must not leave partial files"
        );
    });
}

#[test]
fn interrupted_transfer_fails_without_partial_cache() {
    let scratch = Scratch::new("truncated");
    mock_http(http_response("200 OK", 400, b"partial"), |url| {
        assert!(
            download(&url, &scratch.file()).is_err(),
            "incomplete body must fail"
        );
        assert!(
            !scratch.file().exists(),
            "incomplete download must not be published"
        );
        assert_eq!(
            std::fs::read_dir(scratch.file().parent().unwrap())
                .unwrap()
                .count(),
            0,
            "temporary .part file must be cleaned"
        );
    });
}

#[test]
fn http_error_body_is_not_exposed_as_successful_metadata() {
    mock_http(http_response("500 Server Error", 5, b"error"), |url| {
        assert!(download_text(&url).is_err());
    });
}
