//! Nix 状態分類 (NixStatus) — issue #15
//!
//! `existing_nix_detected()` は marker の存在で fail-closed に「存在する」ことしか
//! 判定できない。本 module は install 済み環境を 4 状態に分類し、次のアクション
//! (何もしない / repair / 手動対応) を機械的に決定する。install 拒否の policy は
//! `existing_nix_detected()` 側に残したまま (NixStatus はその上位概念)。

use std::path::Path;

use serde::Serialize;

use crate::managed_nix::ownership::default_ownership_path;
use crate::managed_nix::receipt::{default_receipt_path, Receipt};

/// install 済み Nix 環境の分類
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum NixStatus {
    /// installation marker が一切存在しない
    Missing,
    /// marker + receipt + runtime 検証が全て揃っている
    Healthy,
    /// marker は残存するが receipt が読めない / store ping が失敗する
    Degraded,
    /// ownership record と /nix 配下の実態が不一致 (片方だけ残っている)
    Broken,
}

impl NixStatus {
    /// 分類 label (doctor の [status] 欄そのもの)
    pub fn label(&self) -> &'static str {
        match self {
            NixStatus::Missing => "Missing",
            NixStatus::Healthy => "Healthy",
            NixStatus::Degraded => "Degraded",
            NixStatus::Broken => "Broken",
        }
    }

    /// 各状態の次アクション文案
    pub fn next_action(&self) -> &'static str {
        match self {
            NixStatus::Missing => "schneeforge nix install で Managed Nix を導入してください",
            NixStatus::Healthy => "対応不要です",
            NixStatus::Degraded => {
                "修復が必要です: schneeforge nix repair で次の 手順 を確認してください"
            }
            NixStatus::Broken => {
                "ownership record と /nix の実態が不一致です: schneeforge nix repair で\
                 stale な ownership record を削除できます"
            }
        }
    }
}

impl std::fmt::Display for NixStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

/// `schneeforge nix repair` が状態ごとに取る修復 action。
/// repair が自動実行するのは stale record 削除のみ。破壊的な uninstall /
/// 再 install は案内表示に留める (spec: state-driven 修復)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum RepairAction {
    /// marker が一切無いのに ownership record が残っている (Broken)。
    /// Nix 実態が無いため record を削除するだけで Missing へ復帰する
    RemoveStaleOwnership,
    /// marker + receipt はあるが runtime 検証に失敗 (Degraded)。
    /// uninstall + 再 install を案内する (自動実行しない)
    SuggestUninstall,
    /// marker のみで receipt が読めない (Degraded)。upstream も revert
    /// できないため `uninstall --force` と手動手順を案内する
    SuggestManualCleanup,
    /// Healthy — 対応不要
    NoActionNeeded,
    /// Missing — install を案内
    SuggestInstall,
}

/// NixStatus 分類から repair の action を決定する。
/// `NixStatus` 単体では Degraded の receipt 有無が区別できないため、
/// 観測結果 (`StatusProbe`) を直接入力にする。
pub fn repair_action(probe: &StatusProbe) -> RepairAction {
    if !probe.any_marker() {
        return if probe.ownership_exists {
            RepairAction::RemoveStaleOwnership
        } else {
            RepairAction::SuggestInstall
        };
    }
    if !probe.receipt_readable {
        return RepairAction::SuggestManualCleanup;
    }
    if !probe.store_ping_ok {
        return RepairAction::SuggestUninstall;
    }
    RepairAction::NoActionNeeded
}

/// 分類に必要な実環境の観測結果。実 path 群を差し替え可能にすることで
/// unit test は実 `/nix` に依存しない (spec: 分類 input は injectable)。
#[derive(Debug, Clone)]
pub struct StatusProbe {
    /// installation marker (`/nix/store`) の実在
    pub store_marker: bool,
    /// installation marker (`/nix/var/nix`) の実在
    pub var_marker: bool,
    /// receipt (`/nix/receipt.json`) が存在し parse 可能か
    pub receipt_readable: bool,
    /// ownership record (`/nix/schneeforge-managed.json`) の存在
    pub ownership_exists: bool,
    /// `nix store ping` の成否
    pub store_ping_ok: bool,
}

impl StatusProbe {
    /// marker 群 (receipt 含む) のいずれかが存在するか
    fn any_marker(&self) -> bool {
        self.store_marker || self.var_marker || self.receipt_readable
    }

    /// 実環境の既定 path から観測する。`store_ping_ok` は caller 側で解決済み
    /// nix binary の実行結果を渡す (path の spawn を本 module に持ち込まない)。
    pub fn detect(store_ping_ok: bool) -> Self {
        Self {
            store_marker: Path::new("/nix/store").exists(),
            var_marker: Path::new("/nix/var/nix").exists(),
            receipt_readable: Receipt::load(&default_receipt_path()).is_ok(),
            ownership_exists: default_ownership_path().exists(),
            store_ping_ok,
        }
    }

    /// test 用: `root` を `/nix` に見立てて観測する
    pub fn detect_at(root: &Path, store_ping_ok: bool) -> Self {
        Self {
            store_marker: root.join("store").exists(),
            var_marker: root.join("var/nix").exists(),
            receipt_readable: Receipt::load(&root.join("receipt.json")).is_ok(),
            ownership_exists: root.join("schneeforge-managed.json").exists(),
            store_ping_ok,
        }
    }
}

/// `NixStatus` への分類結果と、分類根拠の要約 (doctor 表示用)
#[derive(Debug, Clone, Serialize)]
pub struct StatusReport {
    pub status: NixStatus,
    /// Broken 判定時の不一致内容 (どちら側が残っているか)
    pub mismatch: Option<String>,
}

/// 観測結果を 4 状態へ分類する。
///
/// 判定順序:
/// 1. marker も ownership も一切存在しない → `Missing`
/// 2. ownership のみ残存し marker が無い → `Broken` (uninstall 中断の跡)
/// 3. marker はあるが receipt が読めない / ping が失敗 → `Degraded`
/// 4. 全て揃っている → `Healthy`
///
/// marker のみで ownership が無い場合は `Broken` にしない。SchneeForge 経由で
/// ない install (nix-installer 直接等) は ownership 無しで正常に動くため、
/// receipt / runtime 検証の結果に委ねる。
pub fn classify(probe: &StatusProbe) -> StatusReport {
    if !probe.any_marker() {
        return if probe.ownership_exists {
            StatusReport {
                status: NixStatus::Broken,
                mismatch: Some(
                    "ownership record は存在しますが /nix 配下の installation marker が\
                     ありません (uninstall が途中で失敗した可能性があります)"
                        .to_string(),
                ),
            }
        } else {
            StatusReport {
                status: NixStatus::Missing,
                mismatch: None,
            }
        };
    }

    if !probe.receipt_readable || !probe.store_ping_ok {
        return StatusReport {
            status: NixStatus::Degraded,
            mismatch: None,
        };
    }

    StatusReport {
        status: NixStatus::Healthy,
        mismatch: None,
    }
}

/// 実環境を観測して分類する (doctor 用 helper)
pub fn classify_current(store_ping_ok: bool) -> StatusReport {
    classify(&StatusProbe::detect(store_ping_ok))
}

/// 実環境を観測して repair action を決定する (CLI repair 用 helper)
pub fn repair_action_current(store_ping_ok: bool) -> RepairAction {
    repair_action(&StatusProbe::detect(store_ping_ok))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::managed_nix::ownership::OwnershipRecord;

    fn write_receipt(root: &Path) {
        std::fs::write(
            root.join("receipt.json"),
            r#"{"version":"0.1.0","actions":[],"planner":null}"#,
        )
        .unwrap();
    }

    fn write_ownership(root: &Path) {
        let rec = OwnershipRecord::new("2.35.1", "a".repeat(64));
        std::fs::write(
            root.join("schneeforge-managed.json"),
            serde_json::to_string(&rec).unwrap(),
        )
        .unwrap();
    }

    fn temp_root(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("sf_status_{tag}_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn classify_missing_when_no_markers() {
        let tmp = temp_root("missing");
        let report = classify(&StatusProbe::detect_at(&tmp, false));
        assert_eq!(report.status, NixStatus::Missing);
        assert!(report
            .status
            .next_action()
            .contains("schneeforge nix install"));
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn classify_healthy_when_complete() {
        let tmp = temp_root("healthy");
        std::fs::create_dir_all(tmp.join("store")).unwrap();
        std::fs::create_dir_all(tmp.join("var/nix")).unwrap();
        write_receipt(&tmp);
        write_ownership(&tmp);
        let report = classify(&StatusProbe::detect_at(&tmp, true));
        assert_eq!(report.status, NixStatus::Healthy);
        assert!(report.status.next_action().contains("対応不要"));
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn classify_degraded_when_receipt_missing() {
        let tmp = temp_root("deg_receipt");
        std::fs::create_dir_all(tmp.join("store")).unwrap();
        let report = classify(&StatusProbe::detect_at(&tmp, true));
        assert_eq!(report.status, NixStatus::Degraded);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn classify_degraded_when_store_ping_fails() {
        let tmp = temp_root("deg_ping");
        std::fs::create_dir_all(tmp.join("store")).unwrap();
        write_receipt(&tmp);
        let report = classify(&StatusProbe::detect_at(&tmp, false));
        assert_eq!(report.status, NixStatus::Degraded);
        assert!(report
            .status
            .next_action()
            .contains("schneeforge nix repair"));
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn classify_broken_when_ownership_without_markers() {
        let tmp = temp_root("broken");
        write_ownership(&tmp);
        let report = classify(&StatusProbe::detect_at(&tmp, false));
        assert_eq!(report.status, NixStatus::Broken);
        let mismatch = report.mismatch.expect("broken must explain the mismatch");
        assert!(mismatch.contains("ownership"));
        assert!(report
            .status
            .next_action()
            .contains("schneeforge nix repair"));
        let _ = std::fs::remove_dir_all(&tmp);
    }

    /// SchneeForge 経由でない install (nix-installer 直接等) は ownership が
    /// 無くて正常。marker のみ + receipt + ping が揃っていれば Broken にしない。
    #[test]
    fn classify_marker_only_without_ownership_is_not_broken() {
        let tmp = temp_root("external");
        std::fs::create_dir_all(tmp.join("store")).unwrap();
        write_receipt(&tmp);
        let report = classify(&StatusProbe::detect_at(&tmp, true));
        assert_eq!(report.status, NixStatus::Healthy);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn next_action_covers_all_states() {
        assert_eq!(NixStatus::Missing.label(), "Missing");
        assert_eq!(NixStatus::Healthy.label(), "Healthy");
        assert_eq!(NixStatus::Degraded.label(), "Degraded");
        assert_eq!(NixStatus::Broken.label(), "Broken");
        assert_eq!(NixStatus::Missing.to_string(), "Missing");
        for status in [
            NixStatus::Missing,
            NixStatus::Healthy,
            NixStatus::Degraded,
            NixStatus::Broken,
        ] {
            assert!(!status.next_action().is_empty());
        }
    }

    /// doctor の案内 (next_action) は Degraded / Broken で実行可能な
    /// `nix repair` を指すこと (repair command が実装されたため)
    #[test]
    fn next_action_points_to_repair_for_degraded_and_broken() {
        assert!(NixStatus::Degraded
            .next_action()
            .contains("schneeforge nix repair"));
        assert!(NixStatus::Broken
            .next_action()
            .contains("schneeforge nix repair"));
    }

    #[test]
    fn repair_action_broken_removes_stale_ownership() {
        let tmp = temp_root("repair_broken");
        write_ownership(&tmp);
        let action = repair_action(&StatusProbe::detect_at(&tmp, false));
        assert_eq!(action, RepairAction::RemoveStaleOwnership);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn repair_action_missing_suggests_install() {
        let tmp = temp_root("repair_missing");
        let action = repair_action(&StatusProbe::detect_at(&tmp, false));
        assert_eq!(action, RepairAction::SuggestInstall);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    /// Degraded は receipt の有無で案内が分岐する
    #[test]
    fn repair_action_degraded_with_receipt_suggests_uninstall() {
        let tmp = temp_root("repair_deg_receipt");
        std::fs::create_dir_all(tmp.join("store")).unwrap();
        write_receipt(&tmp);
        let action = repair_action(&StatusProbe::detect_at(&tmp, false));
        assert_eq!(action, RepairAction::SuggestUninstall);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn repair_action_degraded_without_receipt_suggests_manual() {
        let tmp = temp_root("repair_deg_manual");
        std::fs::create_dir_all(tmp.join("store")).unwrap();
        let action = repair_action(&StatusProbe::detect_at(&tmp, true));
        assert_eq!(action, RepairAction::SuggestManualCleanup);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn repair_action_healthy_is_noop() {
        let tmp = temp_root("repair_healthy");
        std::fs::create_dir_all(tmp.join("store")).unwrap();
        write_receipt(&tmp);
        let action = repair_action(&StatusProbe::detect_at(&tmp, true));
        assert_eq!(action, RepairAction::NoActionNeeded);
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
