//! Conservative local-daemon preflight. This does not open a socket, execute
//! Nix, verify a peer, or authorize a subsequent connection.
//!
//! Nix 2.35 documents that daemon resolves through NIX_DAEMON_SOCKET_PATH
//! or the state directory. Configuration can also introduce side effects:
//! https://nix.dev/manual/nix/2.35/store/types/local-daemon-store.html
//! https://nix.dev/manual/nix/2.35/command-ref/nix-store/query
//!
//! A candidate is only pathname/metadata evidence. Mount locality, ACLs,
//! executable/config provenance, replacement races and actual peer identity
//! remain separate prerequisites. Never serialize paths or environment values.

use std::ffi::OsStr;
use std::fs;
use std::io;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{FileTypeExt, MetadataExt};
use std::path::Path;

use super::ObservationReason;

const DEFAULT_SOCKET: &str = "/nix/var/nix/daemon-socket/socket";
const DEFAULT_DIRECTORIES: [&str; 5] =
    ["/", "/nix", "/nix/var", "/nix/var/nix", "/nix/var/nix/daemon-socket"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DaemonPreflight {
    /// Default socket is structurally plausible, not known reachable or safe.
    Candidate,
    Blocked(ObservationReason),
}

/// Use only environment names, including non-UTF8 Unix names. Initially all
/// NIX_* names are rejected (even benign ones), to avoid silently accepting
/// a new override. XDG configuration overrides are also rejected. The
/// absence of overrides does not establish that default config is safe.
fn has_override<'a>(names: impl IntoIterator<Item = &'a OsStr>) -> bool {
    names.into_iter().any(|name| {
        let name = name.as_bytes();
        name.starts_with(b"NIX_")
            || name == b"XDG_CONFIG_HOME"
            || name == b"XDG_CONFIG_DIRS"
    })
}

/// No subprocess or connection: inspect only fixed, default local paths.
/// This intentionally cannot return a positive daemon reachability result.
pub fn inspect_default_endpoint<'a>(
    names: impl IntoIterator<Item = &'a OsStr>,
) -> DaemonPreflight {
    if has_override(names) {
        return DaemonPreflight::Blocked(ObservationReason::LocalEndpointUnverified);
    }

    inspect_metadata(
        DEFAULT_DIRECTORIES.iter().map(Path::new),
        Path::new(DEFAULT_SOCKET),
        0,
    )
}

fn io_reason(error: io::Error) -> ObservationReason {
    match error.kind() {
        io::ErrorKind::NotFound => ObservationReason::LocalEndpointUnverified,
        io::ErrorKind::PermissionDenied => ObservationReason::PermissionDenied,
        _ => ObservationReason::IoError,
    }
}

fn inspect_metadata<'a>(
    directories: impl IntoIterator<Item = &'a Path>,
    socket: &Path,
    expected_uid: u32,
) -> DaemonPreflight {
    for path in directories {
        let metadata = match fs::symlink_metadata(path) {
            Ok(metadata) => metadata,
            Err(error) => return DaemonPreflight::Blocked(io_reason(error)),
        };
        // symlink_metadata rejects symlinks at every enumerated component.
        // These are independent snapshots, not an atomic path walk.
        if !metadata.is_dir() || metadata.uid() != expected_uid || metadata.mode() & 0o022 != 0 {
            return DaemonPreflight::Blocked(ObservationReason::LocalEndpointUnverified);
        }
    }
    let metadata = match fs::symlink_metadata(socket) {
        Ok(metadata) => metadata,
        Err(error) => return DaemonPreflight::Blocked(io_reason(error)),
    };
    // Socket mode may legitimately be 0666; unlike directory permissions,
    // write access on the socket itself does not permit replacing its name.
    if !metadata.file_type().is_socket() || metadata.uid() != expected_uid {
        return DaemonPreflight::Blocked(ObservationReason::LocalEndpointUnverified);
    }
    DaemonPreflight::Candidate
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::{symlink, PermissionsExt};
    use std::os::unix::net::UnixListener;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    struct Fixture(PathBuf);

    impl Fixture {
        fn new() -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            // Use a short pathname for macOS's Unix socket length limit.
            let path = PathBuf::from("/tmp").join(format!(
                "sf-dp-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
            Self(path)
        }

        fn inspect(&self) -> DaemonPreflight {
            let uid = fs::metadata(&self.0).unwrap().uid();
            inspect_metadata(
                [self.0.as_path()],
                &self.0.join("socket"),
                uid,
            )
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn override_names_are_rejected_without_reading_values() {
        for name in [
            "NIX_DAEMON_SOCKET_PATH",
            "NIX_STATE_DIR",
            "NIX_CONF_DIR",
            "NIX_USER_CONF_FILES",
            "NIX_CONFIG",
            "NIX_STORE_DIR",
            "NIX_FUTURE_OVERRIDE",
            "XDG_CONFIG_HOME",
            "XDG_CONFIG_DIRS",
        ] {
            assert_eq!(
                inspect_default_endpoint([OsStr::new(name)]),
                DaemonPreflight::Blocked(ObservationReason::LocalEndpointUnverified)
            );
        }
        assert!(has_override([OsStr::from_bytes(b"NIX_\xff")]));
        assert!(!has_override([OsStr::new("HOME"), OsStr::new("PATH")]));
    }

    #[test]
    fn owned_socket_is_only_a_candidate() {
        let fixture = Fixture::new();
        let _listener = UnixListener::bind(fixture.0.join("socket")).unwrap();
        assert_eq!(fixture.inspect(), DaemonPreflight::Candidate);
    }

    #[test]
    fn absent_and_regular_file_never_become_candidate() {
        let fixture = Fixture::new();
        let blocked = DaemonPreflight::Blocked(ObservationReason::LocalEndpointUnverified);
        assert_eq!(fixture.inspect(), blocked);
        fs::write(fixture.0.join("socket"), b"not a socket").unwrap();
        assert_eq!(fixture.inspect(), blocked);
    }

    #[test]
    fn live_and_dangling_socket_symlink_are_rejected() {
        let fixture = Fixture::new();
        let target = fixture.0.join("target");
        symlink(&target, fixture.0.join("socket")).unwrap();
        let blocked = DaemonPreflight::Blocked(ObservationReason::LocalEndpointUnverified);
        assert_eq!(fixture.inspect(), blocked);
        let _listener = UnixListener::bind(target).unwrap();
        assert_eq!(fixture.inspect(), blocked);
    }

    #[test]
    fn writable_wrong_owner_and_symlink_directory_are_rejected() {
        let fixture = Fixture::new();
        let socket = fixture.0.join("socket");
        let _listener = UnixListener::bind(&socket).unwrap();
        let uid = fs::metadata(&fixture.0).unwrap().uid();
        let blocked = DaemonPreflight::Blocked(ObservationReason::LocalEndpointUnverified);
        assert_eq!(
            inspect_metadata([fixture.0.as_path()], &socket, uid ^ 1),
            blocked
        );
        let alias = fixture.0.join("alias");
        symlink(&fixture.0, &alias).unwrap();
        assert_eq!(inspect_metadata([alias.as_path()], &socket, uid), blocked);
        fs::set_permissions(&fixture.0, fs::Permissions::from_mode(0o777)).unwrap();
        assert_eq!(fixture.inspect(), blocked);
    }

    #[test]
    fn filesystem_errors_remain_distinct() {
        assert_eq!(
            io_reason(io::ErrorKind::PermissionDenied.into()),
            ObservationReason::PermissionDenied
        );
        assert_eq!(
            io_reason(io::ErrorKind::Other.into()),
            ObservationReason::IoError
        );
    }
}
