// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Evidence-bounded construction work-package capability matching.
//!
//! A positive result is only a synthetic capability match. It is not a
//! dispatcher, safety controller, construction order or physical qualification.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceLevel { VendorClaim, Simulation, SyntheticFixture, Measured, IndependentlyQualified }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityEvidence {
    pub evidence_id: String,
    pub subject_profile_revision: String,
    pub level: EvidenceLevel,
    pub valid_through_tick: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstructionCapabilityProfile {
    pub profile_id: String,
    pub profile_revision: String,
    pub platform_id: String,
    pub as_built_revision: String,
    pub work_type: String,
    pub tool_id: String,
    pub coordinate_frame: String,
    pub environment: String,
    pub max_payload_grams: u64,
    pub max_reach_mm: u64,
    /// Maximum admitted position error; smaller is more precise.
    pub max_position_error_micrometres: u64,
    pub available_energy_wh: u64,
    pub minimum_reserve_wh: u64,
    pub calibration_id: String,
    pub calibration_for_as_built_revision: String,
    pub calibration_valid_through_tick: u64,
    pub evidence: CapabilityEvidence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperatorApproval { NotRequired, Pending, Approved, Declined }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InspectionState { NotRequired, Pending, Accepted, Rejected }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnknownConditionState { None, Bounded, Unbounded }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntheticWorkPackage {
    pub work_package_id: String,
    pub synthetic_case_id: String,
    pub expected_profile_revision: String,
    pub expected_as_built_revision: String,
    pub site_snapshot_revision: String,
    pub observed_site_snapshot_revision: String,
    pub work_type: String,
    pub required_tool_id: String,
    pub coordinate_frame: String,
    pub environment: String,
    pub required_payload_grams: u64,
    pub required_reach_mm: u64,
    pub maximum_position_error_micrometres: u64,
    pub estimated_energy_wh: u64,
    pub required_reserve_after_task_wh: u64,
    pub evaluation_tick: u64,
    pub unknown_conditions: UnknownConditionState,
    pub operator_approval_required: bool,
    pub operator_approval: OperatorApproval,
    pub inspection_required: bool,
    pub inspection_state: InspectionState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchDisposition {
    CompatibleForSyntheticCase, Incompatible, UnknownReviewRequired, EvidenceInconsistent, Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchReason {
    InvalidIdentity, InvalidEnergyEnvelope, ProfileRevisionMismatch, AsBuiltRevisionMismatch,
    SiteSnapshotStale, EvidenceSubjectMismatch, CalibrationSubjectMismatch,
    CalibrationExpired, EvidenceCurrentnessUnknown, EvidenceExpired, EvidenceNotQualified,
    UnsupportedWorkType, WrongTool, CoordinateFrameMismatch, EnvironmentMismatch,
    PayloadExceeded, ReachExceeded, InsufficientPrecision, InsufficientEnergyOrReserve,
    UnknownSiteConditions, OperatorApprovalPending, OperatorDeclined,
    InspectionPending, InspectionRejected,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityMatchResult {
    pub disposition: MatchDisposition,
    /// Stable order: checks append reasons in a fixed sequence.
    pub reasons: Vec<MatchReason>,
}

/// Independent, deterministic match of one exact profile to one frozen task.
#[must_use]
pub fn evaluate_synthetic_match(
    profile: &ConstructionCapabilityProfile,
    task: &SyntheticWorkPackage,
) -> CapabilityMatchResult {
    let mut reasons = Vec::new();
    let mut inconsistent = false;
    let mut unsupported = false;
    let mut incompatible = false;
    let mut unknown = false;

    let ids = [
        profile.profile_id.as_str(), profile.profile_revision.as_str(), profile.platform_id.as_str(),
        profile.as_built_revision.as_str(), profile.work_type.as_str(), profile.tool_id.as_str(),
        profile.coordinate_frame.as_str(), profile.environment.as_str(), profile.calibration_id.as_str(),
        profile.evidence.evidence_id.as_str(), profile.evidence.subject_profile_revision.as_str(),
        task.work_package_id.as_str(), task.synthetic_case_id.as_str(),
        task.expected_profile_revision.as_str(), task.expected_as_built_revision.as_str(),
        task.site_snapshot_revision.as_str(), task.observed_site_snapshot_revision.as_str(),
        task.work_type.as_str(), task.required_tool_id.as_str(), task.coordinate_frame.as_str(),
        task.environment.as_str(),
    ];
    if ids.iter().any(|id| id.trim().is_empty()) {
        reasons.push(MatchReason::InvalidIdentity);
        inconsistent = true;
    }
    if profile.available_energy_wh == 0 || profile.minimum_reserve_wh > profile.available_energy_wh {
        reasons.push(MatchReason::InvalidEnergyEnvelope);
        inconsistent = true;
    }
    if profile.profile_revision != task.expected_profile_revision {
        reasons.push(MatchReason::ProfileRevisionMismatch);
        inconsistent = true;
    }
    if profile.as_built_revision != task.expected_as_built_revision {
        reasons.push(MatchReason::AsBuiltRevisionMismatch);
        inconsistent = true;
    }
    if task.site_snapshot_revision != task.observed_site_snapshot_revision {
        reasons.push(MatchReason::SiteSnapshotStale);
        inconsistent = true;
    }
    if profile.evidence.subject_profile_revision != profile.profile_revision {
        reasons.push(MatchReason::EvidenceSubjectMismatch);
        inconsistent = true;
    }
    if profile.calibration_for_as_built_revision != profile.as_built_revision {
        reasons.push(MatchReason::CalibrationSubjectMismatch);
        inconsistent = true;
    }

    if profile.work_type != task.work_type {
        reasons.push(MatchReason::UnsupportedWorkType);
        unsupported = true;
    }
    if profile.tool_id != task.required_tool_id {
        reasons.push(MatchReason::WrongTool);
        incompatible = true;
    }
    if profile.coordinate_frame != task.coordinate_frame {
        reasons.push(MatchReason::CoordinateFrameMismatch);
        incompatible = true;
    }
    if profile.environment != task.environment {
        reasons.push(MatchReason::EnvironmentMismatch);
        incompatible = true;
    }
    if task.required_payload_grams > profile.max_payload_grams {
        reasons.push(MatchReason::PayloadExceeded);
        incompatible = true;
    }
    if task.required_reach_mm > profile.max_reach_mm {
        reasons.push(MatchReason::ReachExceeded);
        incompatible = true;
    }
    if profile.max_position_error_micrometres > task.maximum_position_error_micrometres {
        reasons.push(MatchReason::InsufficientPrecision);
        incompatible = true;
    }
    let reserve = profile.minimum_reserve_wh.max(task.required_reserve_after_task_wh);
    if task.estimated_energy_wh > profile.available_energy_wh
        || profile.available_energy_wh - task.estimated_energy_wh < reserve
    {
        reasons.push(MatchReason::InsufficientEnergyOrReserve);
        incompatible = true;
    }

    if profile.calibration_valid_through_tick < task.evaluation_tick {
        reasons.push(MatchReason::CalibrationExpired);
        unknown = true;
    }
    match profile.evidence.valid_through_tick {
        None => { reasons.push(MatchReason::EvidenceCurrentnessUnknown); unknown = true; }
        Some(expiry) if expiry < task.evaluation_tick => {
            reasons.push(MatchReason::EvidenceExpired); unknown = true;
        }
        Some(_) => {}
    }
    match profile.evidence.level {
        EvidenceLevel::SyntheticFixture | EvidenceLevel::IndependentlyQualified => {}
        EvidenceLevel::VendorClaim | EvidenceLevel::Simulation | EvidenceLevel::Measured => {
            reasons.push(MatchReason::EvidenceNotQualified);
            unknown = true;
        }
    }
    if task.unknown_conditions != UnknownConditionState::None {
        reasons.push(MatchReason::UnknownSiteConditions);
        unknown = true;
    }

    if task.operator_approval_required {
        match task.operator_approval {
            OperatorApproval::Approved => {}
            OperatorApproval::Declined => { reasons.push(MatchReason::OperatorDeclined); incompatible = true; }
            OperatorApproval::Pending | OperatorApproval::NotRequired => {
                reasons.push(MatchReason::OperatorApprovalPending); unknown = true;
            }
        }
    } else if task.operator_approval == OperatorApproval::Declined {
        reasons.push(MatchReason::OperatorDeclined);
        incompatible = true;
    }
    if task.inspection_required {
        match task.inspection_state {
            InspectionState::Accepted => {}
            InspectionState::Rejected => { reasons.push(MatchReason::InspectionRejected); incompatible = true; }
            InspectionState::Pending | InspectionState::NotRequired => {
                reasons.push(MatchReason::InspectionPending); unknown = true;
            }
        }
    }

    let disposition = if inconsistent { MatchDisposition::EvidenceInconsistent }
        else if unsupported { MatchDisposition::Unsupported }
        else if incompatible { MatchDisposition::Incompatible }
        else if unknown { MatchDisposition::UnknownReviewRequired }
        else { MatchDisposition::CompatibleForSyntheticCase };
    CapabilityMatchResult { disposition, reasons }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn profile() -> ConstructionCapabilityProfile {
        ConstructionCapabilityProfile {
            profile_id: "roadheader-profile".into(), profile_revision: "r3".into(),
            platform_id: "roadheader-01".into(), as_built_revision: "asbuilt-7".into(),
            work_type: "rock-cutting".into(), tool_id: "pick-head-02".into(),
            coordinate_frame: "site-local-metric".into(), environment: "subterranean".into(),
            max_payload_grams: 500_000, max_reach_mm: 2_500,
            max_position_error_micrometres: 1_000, available_energy_wh: 800,
            minimum_reserve_wh: 200, calibration_id: "cal-22".into(),
            calibration_for_as_built_revision: "asbuilt-7".into(),
            calibration_valid_through_tick: 500,
            evidence: CapabilityEvidence { evidence_id: "fixture-evidence-9".into(),
                subject_profile_revision: "r3".into(), level: EvidenceLevel::SyntheticFixture,
                valid_through_tick: Some(500) },
        }
    }
    fn task() -> SyntheticWorkPackage {
        SyntheticWorkPackage {
            work_package_id: "wp-17".into(), synthetic_case_id: "baseline-01".into(),
            expected_profile_revision: "r3".into(), expected_as_built_revision: "asbuilt-7".into(),
            site_snapshot_revision: "site-v12".into(), observed_site_snapshot_revision: "site-v12".into(),
            work_type: "rock-cutting".into(), required_tool_id: "pick-head-02".into(),
            coordinate_frame: "site-local-metric".into(), environment: "subterranean".into(),
            required_payload_grams: 100_000, required_reach_mm: 1_200,
            maximum_position_error_micrometres: 2_000, estimated_energy_wh: 300,
            required_reserve_after_task_wh: 200, evaluation_tick: 100,
            unknown_conditions: UnknownConditionState::None, operator_approval_required: false,
            operator_approval: OperatorApproval::NotRequired, inspection_required: false,
            inspection_state: InspectionState::NotRequired,
        }
    }

    #[test]
    fn baseline_matches_only_the_declared_synthetic_case() {
        let r = evaluate_synthetic_match(&profile(), &task());
        assert_eq!(r.disposition, MatchDisposition::CompatibleForSyntheticCase);
        assert!(r.reasons.is_empty());
    }
    #[test]
    fn wrong_tool_and_environment_are_incompatible() {
        let mut t = task(); t.required_tool_id = "gripper".into(); t.environment = "open-water".into();
        let r = evaluate_synthetic_match(&profile(), &t);
        assert_eq!(r.disposition, MatchDisposition::Incompatible);
        assert!(r.reasons.contains(&MatchReason::WrongTool));
        assert!(r.reasons.contains(&MatchReason::EnvironmentMismatch));
    }
    #[test]
    fn task_outside_machine_family_is_unsupported() {
        let mut t = task(); t.work_type = "concrete-lifting".into();
        assert_eq!(evaluate_synthetic_match(&profile(), &t).disposition, MatchDisposition::Unsupported);
    }
    #[test]
    fn stale_profile_asbuilt_or_site_is_inconsistent() {
        let mut t = task(); t.expected_profile_revision = "r2".into();
        t.expected_as_built_revision = "asbuilt-6".into(); t.observed_site_snapshot_revision = "site-v11".into();
        let r = evaluate_synthetic_match(&profile(), &t);
        assert_eq!(r.disposition, MatchDisposition::EvidenceInconsistent);
        assert!(r.reasons.contains(&MatchReason::SiteSnapshotStale));
        assert!(r.reasons.contains(&MatchReason::AsBuiltRevisionMismatch));
    }
    #[test]
    fn payload_reach_precision_and_power_limits_are_checked() {
        let mut t = task(); t.required_payload_grams = 500_001; t.required_reach_mm = 2_501;
        t.maximum_position_error_micrometres = 500; t.estimated_energy_wh = 700;
        t.required_reserve_after_task_wh = 250;
        let r = evaluate_synthetic_match(&profile(), &t);
        assert_eq!(r.disposition, MatchDisposition::Incompatible);
        assert!(r.reasons.contains(&MatchReason::PayloadExceeded));
        assert!(r.reasons.contains(&MatchReason::ReachExceeded));
        assert!(r.reasons.contains(&MatchReason::InsufficientPrecision));
        assert!(r.reasons.contains(&MatchReason::InsufficientEnergyOrReserve));
    }
    #[test]
    fn expired_calibration_and_unknown_geology_require_review() {
        let mut p = profile(); p.calibration_valid_through_tick = 99;
        let mut t = task(); t.unknown_conditions = UnknownConditionState::Unbounded;
        let r = evaluate_synthetic_match(&p, &t);
        assert_eq!(r.disposition, MatchDisposition::UnknownReviewRequired);
        assert!(r.reasons.contains(&MatchReason::CalibrationExpired));
        assert!(r.reasons.contains(&MatchReason::UnknownSiteConditions));
    }
    #[test]
    fn vendor_assertion_is_not_independent_qualification() {
        let mut p = profile(); p.evidence.level = EvidenceLevel::VendorClaim;
        assert_eq!(evaluate_synthetic_match(&p, &task()).disposition, MatchDisposition::UnknownReviewRequired);
    }
    #[test]
    fn evidence_bound_to_another_profile_revision_is_inconsistent() {
        let mut p = profile(); p.evidence.subject_profile_revision = "r2".into();
        assert_eq!(evaluate_synthetic_match(&p, &task()).disposition, MatchDisposition::EvidenceInconsistent);
    }
    #[test]
    fn pending_and_declined_operator_approval_never_pass() {
        let mut t = task(); t.operator_approval_required = true; t.operator_approval = OperatorApproval::Pending;
        assert_eq!(evaluate_synthetic_match(&profile(), &t).disposition, MatchDisposition::UnknownReviewRequired);
        t.operator_approval = OperatorApproval::Declined;
        assert_eq!(evaluate_synthetic_match(&profile(), &t).disposition, MatchDisposition::Incompatible);
    }
    #[test]
    fn missing_or_rejected_inspection_never_passes() {
        let mut t = task(); t.inspection_required = true; t.inspection_state = InspectionState::Pending;
        assert_eq!(evaluate_synthetic_match(&profile(), &t).disposition, MatchDisposition::UnknownReviewRequired);
        t.inspection_state = InspectionState::Rejected;
        assert_eq!(evaluate_synthetic_match(&profile(), &t).disposition, MatchDisposition::Incompatible);
    }
    #[test]
    fn invalid_energy_profile_is_evidence_inconsistent() {
        let mut p = profile(); p.minimum_reserve_wh = p.available_energy_wh + 1;
        assert_eq!(evaluate_synthetic_match(&p, &task()).disposition, MatchDisposition::EvidenceInconsistent);
    }
}
