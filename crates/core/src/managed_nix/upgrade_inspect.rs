//! Phase A: pure, read-only upgrade inspection report contract.
//!
//! This module deliberately does not probe a host or authorize an upgrade.
//! Observations will be supplied by a bounded, local-only collector in a
//! subsequent implementation step. No raw subprocess output or record payload
//! is serialized by this report.

use serde::Serialize;

use crate::discovery::{Architecture, Platform};

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
    ProbeFailed,
    PermissionDenied,
    IoError,
    StableProbeUnverified,
    HostManagerNotVerified,
    InsufficientEvidence,
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
    let parts: Vec<_> = version.split('.').collect();
    let valid = parts.len() == 3
        && parts.iter().all(|part| {
            !part.is_empty()
                && part.bytes().all(|b| b.is_ascii_digit())
                && (part.len() == 1 || !part.starts_with('0'))
                && part.parse::<u32>().is_ok()
        });
    if valid {
        Observation::Known {
            value: NixClientVersion(version.to_owned()),
        }
    } else {
        Observation::Unknown {
            reason: UnsupportedFormat,
        }
    }
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
    pub receipt: RecordState,
    pub installation_ownership: RecordState,
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
    installation_ownership: RecordState,
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
        let both_records_missing = input.receipt == RecordState::Missing
            && input.installation_ownership == RecordState::Missing;
        let known_client = matches!(&input.client_version, Observation::Known { .. });

        let mut reasons = vec![
            ObservationReason::HostManagerNotVerified,
            ObservationReason::StableProbeUnverified,
        ];
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
            && input.receipt == RecordState::RecordValid
            && input.installation_ownership == RecordState::RecordValid
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
            receipt: input.receipt,
            installation_ownership: input.installation_ownership,
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
            receipt: RecordState::Missing,
            installation_ownership: RecordState::Missing,
            repo_declares_nix_darwin: Observation::Known { value: false },
        }
    }

    fn consistent() -> UpgradeObservations {
        UpgradeObservations {
            installation_markers: Observation::Known { value: true },
            client_version: parse_client_version(b"nix (Nix) 2.35.2\n"),
            daemon_reachable: Observation::Known { value: true },
            receipt: RecordState::RecordValid,
            installation_ownership: RecordState::RecordValid,
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
                installation_ownership: RecordState::RecordValid,
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
                receipt,
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
    fn unsupported_platform_is_never_classified_as_upgradeable() {
        let report = UpgradeInspection::from_observations(UpgradeObservations {
            platform: Platform::Unsupported,
            ..consistent()
        });
        assert_eq!(report.upgrade_route, UpgradeRoute::Unsupported);
        assert!(!report.automation_eligible);
    }
}
