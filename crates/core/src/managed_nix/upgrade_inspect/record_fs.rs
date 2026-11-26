//! Read-only record acquisition from a caller-verified local Unix directory.
//!
//! This is not yet invoked by a CLI or a host collector. The caller must
//! establish that the directory is on a trusted local filesystem. Ownership
//! and mode checks do not prove mount locality, absence of ACLs, or record
//! authenticity. No privilege escalation or writes are performed.

use std::fs::{File, OpenOptions};
use std::io;
use std::mem::MaybeUninit;
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::Path;

use super::{observe_ownership, observe_receipt, RecordEvidence, RecordSource};

/// An opened directory descriptor keeps subsequent lookups relative to the
/// same directory even if its pathname changes.
pub struct RecordDirectory {
    directory: File,
    owner_uid: u32,
}

impl RecordDirectory {
    /// Open a caller-verified local directory, normally /nix.
    /// Reject symlinks and directories writable by group or other users.
    pub fn open(root: &Path) -> io::Result<Self> {
        Self::open_for_uid(root, 0)
    }

    fn open_for_uid(root: &Path, owner_uid: u32) -> io::Result<Self> {
        let directory = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(root)?;
        let metadata = directory.metadata()?;
        if !metadata.is_dir() || metadata.uid() != owner_uid || metadata.mode() & 0o022 != 0 {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        Ok(Self {
            directory,
            owner_uid,
        })
    }

    pub fn receipt(&self) -> RecordEvidence {
        observe_receipt(self.open_record(c"receipt.json"))
    }

    pub fn ownership(&self) -> RecordEvidence {
        observe_ownership(self.open_record(c"schneeforge-managed.json"))
    }

    // Only fixed basenames are passed by the two public observation methods.
    fn open_record(&self, name: &std::ffi::CStr) -> RecordSource<File> {
        let before = match self.stat_record(name) {
            Ok(stat) => stat,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return RecordSource::Missing;
            }
            Err(error) => return RecordSource::Unavailable(error),
        };
        if let Err(error) = self.check_regular(&before) {
            return RecordSource::Unavailable(error);
        }
        // SAFETY: directory is a live descriptor; name is a NUL-terminated
        // fixed basename. No create flag is supplied, and no mode is needed.
        let fd = unsafe {
            libc::openat(
                self.directory.as_raw_fd(),
                name.as_ptr(),
                libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC,
            )
        };
        if fd < 0 {
            return RecordSource::Unavailable(io::Error::last_os_error());
        }
        // SAFETY: openat returned a new owned descriptor. File closes it once.
        let file = unsafe { File::from_raw_fd(fd) };
        let after = match stat_file(&file) {
            Ok(stat) => stat,
            Err(error) => return RecordSource::Unavailable(error),
        };
        if let Err(error) = self.check_regular(&after) {
            return RecordSource::Unavailable(error);
        }
        // Reject replacement between observation and open; this is not an
        // authentication proof or a defense against privileged concurrent writes.
        if before.st_dev != after.st_dev || before.st_ino != after.st_ino {
            return RecordSource::Unavailable(io::ErrorKind::InvalidData.into());
        }
        RecordSource::Opened(file)
    }

    fn check_regular(&self, stat: &libc::stat) -> io::Result<()> {
        if stat.st_mode & libc::S_IFMT != libc::S_IFREG {
            return Err(io::ErrorKind::InvalidData.into());
        }
        if stat.st_uid != self.owner_uid || stat.st_mode & 0o022 != 0 {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        Ok(())
    }

    fn stat_record(&self, name: &std::ffi::CStr) -> io::Result<libc::stat> {
        let mut stat = MaybeUninit::<libc::stat>::uninit();
        // SAFETY: the descriptor and C string are live, and the output buffer
        // is writable. Only a successful fstatat initializes it.
        let result = unsafe {
            libc::fstatat(
                self.directory.as_raw_fd(),
                name.as_ptr(),
                stat.as_mut_ptr(),
                libc::AT_SYMLINK_NOFOLLOW,
            )
        };
        if result != 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: fstatat succeeded and initialized the stat buffer.
        Ok(unsafe { stat.assume_init() })
    }
}

fn stat_file(file: &File) -> io::Result<libc::stat> {
    let mut stat = MaybeUninit::<libc::stat>::uninit();
    // SAFETY: live file descriptor and writable output buffer.
    if unsafe { libc::fstat(file.as_raw_fd(), stat.as_mut_ptr()) } != 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: fstat succeeded.
    Ok(unsafe { stat.assume_init() })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::managed_nix::upgrade_inspect::RecordState;
    use std::fs;
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::fs::{symlink, PermissionsExt};
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    struct Fixture(PathBuf);

    impl Fixture {
        fn new() -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let path = std::env::temp_dir().join(format!(
                "sf-record-fs-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
            Self(path)
        }

        fn open(&self) -> io::Result<RecordDirectory> {
            RecordDirectory::open_for_uid(&self.0, fs::metadata(&self.0)?.uid())
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn regular_records_are_read_without_mutating_contents() {
        let fixture = Fixture::new();
        let receipt = fixture.0.join("receipt.json");
        let owner = fixture.0.join("schneeforge-managed.json");
        let body = br#"{"version":"2.35.4","actions":[]}"#;
        let owner_body = serde_json::to_vec(&crate::managed_nix::OwnershipRecord::new(
            "2.35.4",
            "a".repeat(64),
        ))
        .unwrap();
        fs::write(&receipt, body).unwrap();
        fs::write(&owner, &owner_body).unwrap();
        for path in [&receipt, &owner] {
            fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
        }
        let dir = fixture.open().unwrap();
        assert_eq!(dir.receipt().state, RecordState::RecordValid);
        assert_eq!(dir.ownership().state, RecordState::RecordValid);
        assert_eq!(fs::read(receipt).unwrap(), body);
        assert_eq!(fs::read(owner).unwrap(), owner_body);
    }

    #[test]
    fn missing_entry_is_distinct_from_dangling_or_live_symlink() {
        let fixture = Fixture::new();
        let dir = fixture.open().unwrap();
        assert_eq!(dir.receipt().state, RecordState::Missing);
        let path = fixture.0.join("receipt.json");
        let target = fixture.0.join("target");
        symlink(&target, &path).unwrap();
        assert_eq!(dir.receipt().state, RecordState::IoError);
        fs::write(target, br#"{"version":"2.35.4"}"#).unwrap();
        assert_eq!(dir.receipt().state, RecordState::IoError);
    }

    #[test]
    fn directory_and_fifo_are_rejected_before_read() {
        let fixture = Fixture::new();
        fs::create_dir(fixture.0.join("receipt.json")).unwrap();
        let fifo = fixture.0.join("schneeforge-managed.json");
        let name = std::ffi::CString::new(fifo.as_os_str().as_bytes()).unwrap();
        // SAFETY: name is a live NUL-terminated path inside the test directory.
        assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
        let dir = fixture.open().unwrap();
        assert_eq!(dir.receipt().state, RecordState::IoError);
        assert_eq!(dir.ownership().state, RecordState::IoError);
    }

    #[test]
    fn writable_or_wrong_owner_is_rejected() {
        let fixture = Fixture::new();
        let receipt = fixture.0.join("receipt.json");
        fs::write(&receipt, br#"{"version":"2.35.4"}"#).unwrap();
        fs::set_permissions(&receipt, fs::Permissions::from_mode(0o666)).unwrap();
        assert_eq!(
            fixture.open().unwrap().receipt().state,
            RecordState::PermissionDenied
        );
        let uid = fs::metadata(&fixture.0).unwrap().uid();
        assert!(RecordDirectory::open_for_uid(&fixture.0, uid ^ 1).is_err());
        fs::set_permissions(&fixture.0, fs::Permissions::from_mode(0o777)).unwrap();
        assert!(fixture.open().is_err());
    }

    #[test]
    fn directory_descriptor_keeps_lookup_bound_to_opened_directory() {
        let fixture = Fixture::new();
        let original = fixture.0.join("original");
        let moved = fixture.0.join("moved");
        fs::create_dir(&original).unwrap();
        fs::set_permissions(&original, fs::Permissions::from_mode(0o700)).unwrap();
        let uid = fs::metadata(&original).unwrap().uid();
        let dir = RecordDirectory::open_for_uid(&original, uid).unwrap();
        fs::rename(&original, &moved).unwrap();
        fs::create_dir(&original).unwrap();
        let record = original.join("receipt.json");
        fs::write(&record, br#"{"version":"2.35.4"}"#).unwrap();
        assert_eq!(dir.receipt().state, RecordState::Missing);
        let record = moved.join("receipt.json");
        fs::write(&record, br#"{"version":"2.35.4"}"#).unwrap();
        fs::set_permissions(&record, fs::Permissions::from_mode(0o600)).unwrap();
        assert_eq!(dir.receipt().state, RecordState::RecordValid);
    }

    #[test]
    fn directory_symlink_is_not_followed() {
        let fixture = Fixture::new();
        let path = fixture.0.join("alias");
        symlink(&fixture.0, &path).unwrap();
        let uid = fs::metadata(&fixture.0).unwrap().uid();
        assert!(RecordDirectory::open_for_uid(&path, uid).is_err());
    }
}
