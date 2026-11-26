//! Phase A: pure, read-only upgrade inspection report contract.
//!
//! This module deliberately does not probe a host or authorize an upgrade.
//! Observations will be supplied by a bounded, local-only collector in a
//! subsequent implementation step. No raw subprocess output or record payload
//! is serialized by this report.

#[cfg(unix)]
pub mod probe_process;
#[cfg(unix)]
pub mod record_fs;

use std::io::{self, Read};

use serde::Serialize;

use crate::discovery::{Architecture, Platform};
use crate::managed_nix::{OwnershipRecord, Receipt};

/// Stable machine-readable categories; never include raw stderr or paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ObservationReason {
    NotProbed,
    ToolMissing,
    UnsupportedFormat,
    UnsupportedImplementation,
    LocalEndpointUnverified,
    TimedOut,
    OutputLimitExceeded,
    ProbeFailed,
    PermissionDenied,
    IoError,
    StableProbeUnverified,
    HostManagerNotVerified,
    InsufficientEvidence,
    RecordMissing,
    RecordTooLarge,
    InstallerVersionMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "kebab-case")]
pub enum Observation<T> {
    Known { value: T },
    Unknown { reason: ObservationReason },
    Error { reason: ObservationReason },
}

/// A stable numeric Nix client version only. No arbitrary version output,
/// prerelease suffix, path, username or stderr is exposed in JSON.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct NixClientVersion(String);

/// Separately typed daemon version. Phase A has no verified probe for it yet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct NixDaemonVersion(String);

/// Interpret successful stdout of a future bounded `nix --version` probe.
/// A strict three-component stable version is supported initially. Unknown
/// implementations and version formats must not be classified as patched.
pub fn parse_client_version(stdout: &[u8]) -> Observation<NixClientVersion> {
    use ObservationReason::{UnsupportedFormat, UnsupportedImplementation};

    if stdout.len() > 128 {
        return Observation::Unknown {
            reason: UnsupportedFormat,
        };
    }
    let Ok(line) = std::str::from_utf8(stdout) else {
        return Observation::Unknown {
            reason: UnsupportedFormat,
        };
    };
    let line = line.trim_end_matches(['\r', '\n']);
    let Some(version) = line.strip_prefix("nix (Nix) ") else {
        return Observation::Unknown {
            reason: if line.starts_with("nix (Lix) ") {
                UnsupportedImplementation
            } else {
                UnsupportedFormat
            },
        };
    };
    if is_stable_version(version) {
        Observation::Known {
            value: NixClientVersion(version.to_owned()),
        }
    } else {
        Observation::Unknown {
            reason: UnsupportedFormat,
        }
    }
}

fn is_stable_version(version: &str) -> bool {
    version.len() <= 32
        && version.split('.').count() == 3
        && version.split('.').all(|part| {
            !part.is_empty()
                && part.bytes().all(|b| b.is_ascii_digit())
                && (part.len() == 1 || !part.starts_with('0'))
                && part.parse::<u32>().is_ok()
        })
}

/// Structural evidence only; a valid record is not authenticated ownership.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RecordState {
    RecordValid,
    Missing,
    Invalid,
    PermissionDenied,
    IoError,
    Unknown,
}

/// Provider/receipt version evidence is intentionally separate from Nix runtime versions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct InstallerVersion(String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordEvidence {
    state: RecordState,
    installer_version: Observation<InstallerVersion>,
}

impl RecordEvidence {
    /// Use when the collector has not obtained any record evidence.
    pub fn unobserved() -> Self {
        Self::without_version(RecordState::Unknown, ObservationReason::NotProbed)
    }

    fn without_version(state: RecordState, reason: ObservationReason) -> Self {
        let installer_version = match state {
            RecordState::PermissionDenied | RecordState::IoError => Observation::Error { reason },
            _ => Observation::Unknown { reason },
        };
        Self {
            state,
            installer_version,
        }
    }

    fn valid(version: Option<&str>) -> Self {
        let installer_version = match version {
            Some(v) if is_stable_version(v) => Observation::Known {
                value: InstallerVersion(v.to_owned()),
            },
            _ => Observation::Unknown {
                reason: ObservationReason::UnsupportedFormat,
            },
        };
        Self {
            state: RecordState::RecordValid,
            installer_version,
        }
    }
}

// Diagnostic limits, not a restriction on the existing installer/repair paths.
// Larger legitimate legacy records remain unknown, not absent or corrupt.
const RECEIPT_LIMIT: u64 = 1024 * 1024;
const OWNERSHIP_LIMIT: u64 = 64 * 1024;

/// An absence observation must be established by the collector using metadata.
/// A failed open/read (even NotFound) can instead mean a broken link or race.
pub enum RecordSource<R> {
    Missing,
    Opened(R),
    Unavailable(io::Error),
}

fn record_io_error(error: io::Error) -> RecordEvidence {
    use ObservationReason as Reason;

    match error.kind() {
        io::ErrorKind::PermissionDenied => {
            RecordEvidence::without_version(RecordState::PermissionDenied, Reason::PermissionDenied)
        }
        _ => RecordEvidence::without_version(RecordState::IoError, Reason::IoError),
    }
}

fn read_record<R: Read>(
    source: RecordSource<R>,
    limit: u64,
    parse: impl FnOnce(&[u8]) -> RecordEvidence,
) -> RecordEvidence {
    let reader = match source {
        RecordSource::Opened(reader) => reader,
        RecordSource::Missing => {
            return RecordEvidence::without_version(
                RecordState::Missing,
                ObservationReason::RecordMissing,
            );
        }
        RecordSource::Unavailable(error) => return record_io_error(error),
    };
    let mut bytes = Vec::new();
    if let Err(error) = reader.take(limit + 1).read_to_end(&mut bytes) {
        return record_io_error(error);
    }
    if bytes.len() as u64 > limit {
        return RecordEvidence::without_version(
            RecordState::Unknown,
            ObservationReason::RecordTooLarge,
        );
    }
    parse(&bytes)
}

fn invalid_record() -> RecordEvidence {
    RecordEvidence::without_version(RecordState::Invalid, ObservationReason::UnsupportedFormat)
}

/// Classify an injected receipt reader with bounded capture. The caller must
/// establish a safe, local, non-blocking/regular-file source before calling;
/// the byte limit is not a wall-clock timeout or a symlink/TOCTOU defense.
/// No host path is opened by this adapter.
pub fn observe_receipt<R: Read>(source: RecordSource<R>) -> RecordEvidence {
    read_record(source, RECEIPT_LIMIT, |bytes| {
        let Ok(receipt) = serde_json::from_slice::<Receipt>(bytes) else {
            return invalid_record();
        };
        // The legacy Receipt view tolerates absent fields. An empty view is
        // insufficient evidence for a structurally meaningful install plan.
        if receipt.version.is_none() && receipt.actions.is_empty() && receipt.planner.is_none() {
            return RecordEvidence::without_version(
                RecordState::Unknown,
                ObservationReason::InsufficientEvidence,
            );
        }
        RecordEvidence::valid(receipt.version.as_deref())
    })
}

/// Reuse the existing ownership schema and validation without exposing record
/// payload, paths or validation error text. Caller has the same source-safety
/// responsibility as for observe_receipt.
pub fn observe_ownership<R: Read>(source: RecordSource<R>) -> RecordEvidence {
    read_record(
        source,
        OWNERSHIP_LIMIT,
        |bytes| match serde_json::from_slice::<OwnershipRecord>(bytes) {
            Ok(record) if record.validate().is_ok() => {
                RecordEvidence::valid(Some(&record.installer_version))
            }
            _ => invalid_record(),
        },
    )
}

/// Combine the three installation marker metadata results. The collector
/// should use symlink_metadata, not exists(): a dangling symlink is present.
/// Any permission/I/O error takes precedence over both present and absent
/// markers so a partial observation cannot silently become a known state.
pub fn observe_installation_markers(markers: [io::Result<()>; 3]) -> Observation<bool> {
    let mut present = false;
    let mut error = None;
    for marker in markers {
        match marker {
            Ok(()) => present = true,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) if e.kind() == io::ErrorKind::PermissionDenied => {
                error = Some(ObservationReason::PermissionDenied);
            }
            Err(_) if error.is_none() => error = Some(ObservationReason::IoError),
            Err(_) => {}
        }
    }
    match error {
        Some(reason) => Observation::Error { reason },
        None => Observation::Known { value: present },
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum UpgradeRoute {
    Manual,
    Unknown,
    NotInstalled,
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum HostManager {
    Unknown,
}

/// Injected observations. A missing tool alone is insufficient evidence of
/// absence: installation markers, receipt and ownership must also be absent.
#[derive(Debug, Clone)]
pub struct UpgradeObservations {
    pub platform: Platform,
    pub architecture: Architecture,
    pub installation_markers: Observation<bool>,
    pub client_version: Observation<NixClientVersion>,
    pub daemon_reachable: Observation<bool>,
    pub receipt: RecordEvidence,
    pub installation_ownership: RecordEvidence,
    pub repo_declares_nix_darwin: Observation<bool>,
}

/// The report has no writable authorization field. The Phase A constructor
/// always sets automation_eligible to false and daemon_version to unknown.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UpgradeInspection {
    schema_version: u8,
    platform: String,
    installation_markers: Observation<bool>,
    client_version: Observation<NixClientVersion>,
    daemon_reachable: Observation<bool>,
    daemon_version: Observation<NixDaemonVersion>,
    receipt: RecordState,
    receipt_installer_version: Observation<InstallerVersion>,
    installation_ownership: RecordState,
    ownership_installer_version: Observation<InstallerVersion>,
    repo_declares_nix_darwin: Observation<bool>,
    host_manager: HostManager,
    upgrade_route: UpgradeRoute,
    automation_eligible: bool,
    reasons: Vec<ObservationReason>,
}

impl UpgradeInspection {
    pub fn from_observations(input: UpgradeObservations) -> Self {
        let missing_client = matches!(
            &input.client_version,
            Observation::Unknown {
                reason: ObservationReason::ToolMissing
            }
        );
        let no_markers = matches!(
            &input.installation_markers,
            Observation::Known { value: false }
        );
        let no_daemon_evidence = matches!(
            &input.daemon_reachable,
            Observation::Unknown {
                reason: ObservationReason::ToolMissing | ObservationReason::NotProbed
            }
        );
        let both_records_missing = input.receipt.state == RecordState::Missing
            && input.installation_ownership.state == RecordState::Missing;
        let known_client = matches!(&input.client_version, Observation::Known { .. });
        let (matching_installer_versions, mismatched_installer_versions) = match (
            &input.receipt.installer_version,
            &input.installation_ownership.installer_version,
        ) {
            (Observation::Known { value: receipt }, Observation::Known { value: owner }) => {
                (receipt == owner, receipt != owner)
            }
            _ => (false, false),
        };

        let mut reasons = vec![
            ObservationReason::HostManagerNotVerified,
            ObservationReason::StableProbeUnverified,
        ];
        if mismatched_installer_versions {
            reasons.push(ObservationReason::InstallerVersionMismatch);
        }
        let route = if !super::is_supported(input.platform, input.architecture) {
            UpgradeRoute::Unsupported
        } else if missing_client && no_markers && both_records_missing && no_daemon_evidence {
            UpgradeRoute::NotInstalled
        } else if known_client
            && matches!(
                &input.installation_markers,
                Observation::Known { value: true }
            )
            && matches!(&input.daemon_reachable, Observation::Known { value: true })
            && input.receipt.state == RecordState::RecordValid
            && input.installation_ownership.state == RecordState::RecordValid
            && matching_installer_versions
        {
            // Even this consistent structural evidence does not establish the
            // effective owner, daemon version or upgrade authorization.
            UpgradeRoute::Manual
        } else {
            reasons.push(ObservationReason::InsufficientEvidence);
            UpgradeRoute::Unknown
        };

        Self {
            schema_version: 1,
            platform: format!("{}-{}", input.platform, input.architecture),
            installation_markers: input.installation_markers,
            client_version: input.client_version,
            daemon_reachable: input.daemon_reachable,
            daemon_version: Observation::Unknown {
                reason: ObservationReason::StableProbeUnverified,
            },
            receipt: input.receipt.state,
            receipt_installer_version: input.receipt.installer_version,
            installation_ownership: input.installation_ownership.state,
            ownership_installer_version: input.installation_ownership.installer_version,
            repo_declares_nix_darwin: input.repo_declares_nix_darwin,
            host_manager: HostManager::Unknown,
            upgrade_route: route,
            automation_eligible: false,
            reasons,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn absent() -> UpgradeObservations {
        UpgradeObservations {
            platform: Platform::Linux,
            architecture: Architecture::X86_64,
            installation_markers: Observation::Known { value: false },
            client_version: Observation::Unknown {
                reason: ObservationReason::ToolMissing,
            },
            daemon_reachable: Observation::Unknown {
                reason: ObservationReason::ToolMissing,
            },
            receipt: observe_receipt(RecordSource::<&[u8]>::Missing),
            installation_ownership: observe_ownership(RecordSource::<&[u8]>::Missing),
            repo_declares_nix_darwin: Observation::Known { value: false },
        }
    }

    fn valid_ownership() -> RecordEvidence {
        let record = OwnershipRecord::new("2.35.4", "a".repeat(64));
        let bytes = serde_json::to_vec(&record).unwrap();
        observe_ownership(RecordSource::Opened(bytes.as_slice()))
    }

    fn consistent() -> UpgradeObservations {
        UpgradeObservations {
            installation_markers: Observation::Known { value: true },
            client_version: parse_client_version(b"nix (Nix) 2.35.2\n"),
            daemon_reachable: Observation::Known { value: true },
            receipt: observe_receipt(RecordSource::Opened(
                &br#"{"version":"2.35.4","actions":[]}"#[..],
            )),
            installation_ownership: valid_ownership(),
            ..absent()
        }
    }

    #[test]
    fn stable_client_version_is_serialized_without_raw_output() {
        let version = parse_client_version(b"nix (Nix) 2.35.2\r\n");
        assert_eq!(
            serde_json::to_value(version).unwrap(),
            serde_json::json!({"state": "known", "value": "2.35.2"})
        );
    }

    #[test]
    fn unsupported_or_untrusted_version_output_never_becomes_a_known_version() {
        for stdout in [
            &b"nix (Lix) 2.35.2\n"[..],
            &b"nix (Nix) 2.35.2-pre"[..],
            &b"nix (Nix) 2.035.2"[..],
            &b"nix (Nix) 2.35"[..],
            &b"nix (Nix) 2.35.4294967296"[..],
            &b"nix (Nix) 2.35.2\n/home/private-token"[..],
            &b"nix (Nix) 2.35.\xff"[..],
            &[b'9'; 129][..],
        ] {
            assert!(matches!(
                parse_client_version(stdout),
                Observation::Unknown { .. }
            ));
        }
        let report = serde_json::to_string(&parse_client_version(
            b"nix (Nix) 2.35.2\n/home/private-token",
        ))
        .unwrap();
        assert!(!report.contains("private-token"));
    }

    #[test]
    fn consistent_records_do_not_establish_daemon_version_or_ownership() {
        let report = UpgradeInspection::from_observations(consistent());
        let json = serde_json::to_value(&report).unwrap();
        assert_eq!(json["schema_version"], 1);
        assert_eq!(json["client_version"]["value"], "2.35.2");
        assert_eq!(json["receipt_installer_version"]["value"], "2.35.4");
        assert_eq!(json["ownership_installer_version"]["value"], "2.35.4");
        assert_eq!(json["daemon_reachable"]["value"], true);
        assert_eq!(json["daemon_version"]["state"], "unknown");
        assert_eq!(json["installation_ownership"], "record-valid");
        assert_eq!(json["host_manager"], "unknown");
        assert_eq!(json["upgrade_route"], "manual");
        assert_eq!(json["automation_eligible"], false);
    }

    #[test]
    fn all_absence_evidence_is_required_for_not_installed() {
        let report = UpgradeInspection::from_observations(absent());
        assert_eq!(report.upgrade_route, UpgradeRoute::NotInstalled);
        for input in [
            UpgradeObservations {
                installation_markers: Observation::Known { value: true },
                ..absent()
            },
            UpgradeObservations {
                installation_markers: Observation::Error {
                    reason: ObservationReason::PermissionDenied,
                },
                ..absent()
            },
            UpgradeObservations {
                installation_ownership: valid_ownership(),
                ..absent()
            },
            UpgradeObservations {
                daemon_reachable: Observation::Known { value: true },
                ..absent()
            },
            UpgradeObservations {
                daemon_reachable: Observation::Error {
                    reason: ObservationReason::TimedOut,
                },
                ..absent()
            },
        ] {
            let report = UpgradeInspection::from_observations(input);
            assert_eq!(report.upgrade_route, UpgradeRoute::Unknown);
            assert!(!report.automation_eligible);
        }
    }

    #[test]
    fn malformed_or_unreadable_receipt_remains_distinct_and_unknown() {
        for receipt in [
            RecordState::Invalid,
            RecordState::PermissionDenied,
            RecordState::IoError,
            RecordState::Unknown,
        ] {
            let report = UpgradeInspection::from_observations(UpgradeObservations {
                receipt: RecordEvidence::without_version(receipt, ObservationReason::NotProbed),
                ..consistent()
            });
            assert_eq!(report.receipt, receipt);
            assert_eq!(report.upgrade_route, UpgradeRoute::Unknown);
            assert!(!report.automation_eligible);
        }
    }

    #[test]
    fn repo_declaration_and_daemon_errors_cannot_authorize_upgrade() {
        let report = UpgradeInspection::from_observations(UpgradeObservations {
            platform: Platform::MacOS,
            architecture: Architecture::Aarch64,
            repo_declares_nix_darwin: Observation::Known { value: true },
            daemon_reachable: Observation::Unknown {
                reason: ObservationReason::LocalEndpointUnverified,
            },
            ..consistent()
        });
        assert_eq!(report.host_manager, HostManager::Unknown);
        assert_eq!(report.upgrade_route, UpgradeRoute::Unknown);
        assert!(!report.automation_eligible);
        let report = UpgradeInspection::from_observations(UpgradeObservations {
            daemon_reachable: Observation::Error {
                reason: ObservationReason::TimedOut,
            },
            ..consistent()
        });
        assert_eq!(report.upgrade_route, UpgradeRoute::Unknown);
    }

    #[test]
    fn legacy_receipt_keeps_version_unknown_and_empty_view_is_not_valid() {
        let legacy = observe_receipt(RecordSource::Opened(
            &br#"{"actions":[{"action":"enable_flakes"}]}"#[..],
        ));
        assert_eq!(legacy.state, RecordState::RecordValid);
        assert!(matches!(
            legacy.installer_version,
            Observation::Unknown { .. }
        ));
        assert_eq!(
            observe_receipt(RecordSource::Opened(&b"{}"[..])).state,
            RecordState::Unknown
        );
        assert_eq!(
            observe_receipt(RecordSource::Opened(&b"{broken"[..])).state,
            RecordState::Invalid
        );
    }

    #[test]
    fn ownership_requires_existing_schema_validation() {
        let mut record = OwnershipRecord::new("2.35.4", "a".repeat(64));
        record.installer_sha256 = "bad".to_owned();
        let bytes = serde_json::to_vec(&record).unwrap();
        assert_eq!(
            observe_ownership(RecordSource::Opened(bytes.as_slice())).state,
            RecordState::Invalid
        );
        let mut legacy =
            serde_json::to_value(OwnershipRecord::new("2.35.4", "a".repeat(64))).unwrap();
        legacy.as_object_mut().unwrap().remove("installer_sha256");
        let bytes = serde_json::to_vec(&legacy).unwrap();
        assert_eq!(
            observe_ownership(RecordSource::Opened(bytes.as_slice())).state,
            RecordState::Invalid
        );
    }

    #[test]
    fn record_io_errors_are_not_missing_without_explicit_absence() {
        assert_eq!(
            observe_receipt(RecordSource::<&[u8]>::Missing).state,
            RecordState::Missing
        );
        assert_eq!(
            observe_ownership(RecordSource::<&[u8]>::Missing).state,
            RecordState::Missing
        );
        for (kind, expected) in [
            (io::ErrorKind::NotFound, RecordState::IoError),
            (
                io::ErrorKind::PermissionDenied,
                RecordState::PermissionDenied,
            ),
            (io::ErrorKind::Other, RecordState::IoError),
        ] {
            assert_eq!(
                observe_receipt(RecordSource::<&[u8]>::Unavailable(kind.into())).state,
                expected
            );
            assert_eq!(
                observe_ownership(RecordSource::<&[u8]>::Unavailable(kind.into())).state,
                expected
            );
        }
        struct FailingReader;
        impl Read for FailingReader {
            fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
                Err(io::ErrorKind::NotFound.into())
            }
        }
        assert_eq!(
            observe_receipt(RecordSource::Opened(FailingReader)).state,
            RecordState::IoError
        );
    }

    #[test]
    fn different_or_unverified_installer_versions_require_manual_review() {
        let record = OwnershipRecord::new("2.35.3", "a".repeat(64));
        let bytes = serde_json::to_vec(&record).unwrap();
        let report = UpgradeInspection::from_observations(UpgradeObservations {
            installation_ownership: observe_ownership(RecordSource::Opened(bytes.as_slice())),
            ..consistent()
        });
        assert_eq!(report.upgrade_route, UpgradeRoute::Unknown);
        assert!(report
            .reasons
            .contains(&ObservationReason::InstallerVersionMismatch));
        assert!(!report.automation_eligible);

        let report = UpgradeInspection::from_observations(UpgradeObservations {
            receipt: observe_receipt(RecordSource::Opened(
                &br#"{"actions":[{"action":"enable_flakes"}]}"#[..],
            )),
            ..consistent()
        });
        assert_eq!(report.upgrade_route, UpgradeRoute::Unknown);
        assert!(!report.automation_eligible);
    }

    #[test]
    fn record_capture_is_bounded_and_limit_boundary_is_not_truncated() {
        let mut infinite = io::repeat(b' ');
        let observed = observe_ownership(RecordSource::Opened(&mut infinite));
        assert_eq!(observed.state, RecordState::Unknown);
        assert!(matches!(
            observed.installer_version,
            Observation::Unknown {
                reason: ObservationReason::RecordTooLarge
            }
        ));
        let mut bytes = br#"{"version":"2.35.4","actions":[]}"#.to_vec();
        bytes.resize(RECEIPT_LIMIT as usize, b' ');
        assert_eq!(
            observe_receipt(RecordSource::Opened(bytes.as_slice())).state,
            RecordState::RecordValid
        );
        bytes.push(b' ');
        assert_eq!(
            observe_receipt(RecordSource::Opened(bytes.as_slice())).state,
            RecordState::Unknown
        );
    }

    #[test]
    fn untrusted_record_version_and_payload_are_never_serialized() {
        let receipt = observe_receipt(RecordSource::Opened(
            &br#"{"version":"/home/private-token","actions":[{"secret":"private-token"}]}"#[..],
        ));
        let report = UpgradeInspection::from_observations(UpgradeObservations {
            receipt,
            ..consistent()
        });
        let json = serde_json::to_string(&report).unwrap();
        assert!(!json.contains("private-token"));
        assert_eq!(
            report.receipt_installer_version,
            Observation::Unknown {
                reason: ObservationReason::UnsupportedFormat
            }
        );
    }

    #[test]
    fn markers_do_not_convert_metadata_errors_to_absence_or_presence() {
        let missing = || Err(io::ErrorKind::NotFound.into());
        assert_eq!(
            observe_installation_markers([missing(), missing(), missing()]),
            Observation::Known { value: false }
        );
        assert_eq!(
            observe_installation_markers([missing(), Ok(()), missing()]),
            Observation::Known { value: true }
        );
        assert_eq!(
            observe_installation_markers([
                Ok(()),
                Err(io::ErrorKind::PermissionDenied.into()),
                Err(io::ErrorKind::Other.into()),
            ]),
            Observation::Error {
                reason: ObservationReason::PermissionDenied
            }
        );
        assert_eq!(
            observe_installation_markers([missing(), Err(io::ErrorKind::Other.into()), missing(),]),
            Observation::Error {
                reason: ObservationReason::IoError
            }
        );
    }

    #[test]
    fn unsupported_platform_is_never_classified_as_upgradeable() {
        let report = UpgradeInspection::from_observations(UpgradeObservations {
            platform: Platform::Unsupported,
            ..consistent()
        });
        assert_eq!(report.upgrade_route, UpgradeRoute::Unsupported);
        assert!(!report.automation_eligible);
    }
}
