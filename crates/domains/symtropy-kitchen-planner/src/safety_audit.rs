// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Deterministic post-hoc safety assessment for simulated kitchen episodes.
//!
//! This module audits submitted task-aware safety-spec results. It does not
//! control a robot, define appliance interlocks, certify physical safety, or
//! convert missing telemetry into a safe result.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const KITCHEN_SAFETY_AUDIT_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SafetyAuditStatusV1 {
    NotRun,
    Complete,
    Incomplete,
    Invalid,
    Error,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SafetyClaimStateV1 {
    NotClaimed,
    SafeUnderDeclaredSpecs,
    Unsafe,
    Indeterminate,
    Invalid,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SafetySpecApplicabilityV1 {
    Applicable,
    NotApplicable,
    Unknown,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SafetySpecStatusV1 {
    Satisfied,
    Violated,
    Unknown,
    NotApplicable,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SafetySpecFamilyV1 {
    ArmContactForce,
    TargetContactForce,
    AggregateContactForce,
    BystanderDisplacement,
    HeldObjectStability,
    GraspSlip,
    JointTorque,
    SelfCollision,
    ApplianceBoundary,
    ThermalExposure,
    PressureBoundary,
    Other,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct SafetySpecResultV1 {
    pub spec_id: String,
    pub family: SafetySpecFamilyV1,
    pub applicability: SafetySpecApplicabilityV1,
    pub applicability_reason: Option<String>,
    /// Signals needed to evaluate this specification.
    pub required_signal_refs: Vec<String>,
    /// Signals actually available and accepted by the evaluator.
    pub observed_signal_refs: Vec<String>,
    pub status: SafetySpecStatusV1,
    /// Quantitative robustness; positive is satisfied, negative is violated.
    pub robustness_score: Option<f64>,
    /// Normalized worst-violation depth in [0, 1], when violated.
    pub violation_severity: Option<f64>,
    pub method_ref: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct SafetyEpisodeInputV1 {
    pub audit_status: SafetyAuditStatusV1,
    pub spec_registry_ref: Option<String>,
    pub spec_results: Vec<SafetySpecResultV1>,
    pub native_task_success: Option<bool>,
    pub hazard_triggered: bool,
    pub safe_stop_bound_ms: Option<u64>,
    pub safe_stop_success: Option<bool>,
    pub safe_stop_latency_ms: Option<u64>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SafetyAuditIssueCodeV1 {
    UnexpectedEvidenceWhenNotRun,
    MissingSpecRegistry,
    DuplicateSpecId,
    MissingSpecId,
    MissingMethodRef,
    MissingApplicabilityReason,
    ApplicabilityStatusMismatch,
    InvalidRequiredSignalSet,
    InvalidObservedSignalSet,
    MissingRequiredSignal,
    UnexpectedObservedSignal,
    MissingRobustnessScore,
    InvalidRobustnessScore,
    UnexpectedViolationSeverity,
    MissingViolationSeverity,
    InvalidViolationSeverity,
    InvalidSafeStopState,
    MissingSafeStopBound,
    MissingSafeStopResult,
    SafeStopFailed,
    SafeStopExceededBound,
    NoApplicableSpecifications,
    AuditNotComplete,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SafetyAuditIssueV1 {
    pub code: SafetyAuditIssueCodeV1,
    pub spec_id: Option<String>,
    pub detail: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct SafetyEpisodeReportV1 {
    pub schema_version: u32,
    pub claim_state: SafetyClaimStateV1,
    pub applicable_spec_count: u32,
    pub satisfied_spec_count: u32,
    pub violated_spec_count: u32,
    pub unknown_spec_count: u32,
    pub signal_coverage_rate: Option<f64>,
    pub success_but_unsafe: Option<bool>,
    pub worst_violation_severity_index: Option<f64>,
    pub safe_stop_checked: bool,
    pub issues: Vec<SafetyAuditIssueV1>,
}

impl SafetyEpisodeReportV1 {
    pub fn is_safe_under_declared_specs(&self) -> bool {
        self.claim_state == SafetyClaimStateV1::SafeUnderDeclaredSpecs
    }

    pub fn is_unsafe(&self) -> bool {
        self.claim_state == SafetyClaimStateV1::Unsafe
    }
}

fn add_issue(
    issues: &mut Vec<SafetyAuditIssueV1>,
    code: SafetyAuditIssueCodeV1,
    spec_id: Option<&str>,
    detail: impl Into<String>,
) {
    issues.push(SafetyAuditIssueV1 {
        code,
        spec_id: spec_id.map(str::to_owned),
        detail: detail.into(),
    });
}

fn sort_issues(issues: &mut [SafetyAuditIssueV1]) {
    issues.sort_by(|left, right| {
        (
            left.code,
            left.spec_id.as_deref().unwrap_or(""),
            left.detail.as_str(),
        )
            .cmp(&(
                right.code,
                right.spec_id.as_deref().unwrap_or(""),
                right.detail.as_str(),
            ))
    });
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn finite(value: f64) -> bool {
    value.is_finite()
}

/// Audit one simulated episode. The output is derived from per-spec evidence;
/// summary counts and a producer-supplied "safe" boolean are never trusted.
pub fn evaluate_safety_episode(input: &SafetyEpisodeInputV1) -> SafetyEpisodeReportV1 {
    let mut issues = Vec::new();

    if input.audit_status == SafetyAuditStatusV1::NotRun {
        if !input.spec_results.is_empty()
            || input.spec_registry_ref.is_some()
            || input.hazard_triggered
            || input.safe_stop_bound_ms.is_some()
            || input.safe_stop_success.is_some()
            || input.safe_stop_latency_ms.is_some()
        {
            add_issue(
                &mut issues,
                SafetyAuditIssueCodeV1::UnexpectedEvidenceWhenNotRun,
                None,
                "not_run audit must not contain spec or safe-stop evidence",
            );
        }
        sort_issues(&mut issues);
        return SafetyEpisodeReportV1 {
            schema_version: KITCHEN_SAFETY_AUDIT_SCHEMA_VERSION,
            claim_state: if issues.is_empty() {
                SafetyClaimStateV1::NotClaimed
            } else {
                SafetyClaimStateV1::Invalid
            },
            applicable_spec_count: 0,
            satisfied_spec_count: 0,
            violated_spec_count: 0,
            unknown_spec_count: 0,
            signal_coverage_rate: None,
            success_but_unsafe: None,
            worst_violation_severity_index: None,
            safe_stop_checked: false,
            issues,
        };
    }

    if input.spec_registry_ref.as_deref().is_none_or(|value| !nonempty(value)) {
        add_issue(
            &mut issues,
            SafetyAuditIssueCodeV1::MissingSpecRegistry,
            None,
            "a non-empty exact safety-spec registry reference is required",
        );
    }

    let mut seen_spec_ids = BTreeSet::new();
    let mut applicable_spec_count = 0_u32;
    let mut satisfied_spec_count = 0_u32;
    let mut violated_spec_count = 0_u32;
    let mut unknown_spec_count = 0_u32;
    let mut total_required_signals = 0_u64;
    let mut observed_required_signals = 0_u64;
    let mut worst_violation: Option<f64> = None;

    for result in &input.spec_results {
        let spec_id = result.spec_id.as_str();

        if !nonempty(spec_id) {
            add_issue(
                &mut issues,
                SafetyAuditIssueCodeV1::MissingSpecId,
                None,
                "spec_id must be non-empty",
            );
        } else if !seen_spec_ids.insert(spec_id) {
            add_issue(
                &mut issues,
                SafetyAuditIssueCodeV1::DuplicateSpecId,
                Some(spec_id),
                "spec_id must be unique within an episode",
            );
        }

        if !nonempty(&result.method_ref) {
            add_issue(
                &mut issues,
                SafetyAuditIssueCodeV1::MissingMethodRef,
                Some(spec_id),
                "each result must identify its evaluation method",
            );
        }

        match result.applicability {
            SafetySpecApplicabilityV1::NotApplicable => {
                if result
                    .applicability_reason
                    .as_deref()
                    .is_none_or(|reason| !nonempty(reason))
                    || result.status != SafetySpecStatusV1::NotApplicable
                    || !result.required_signal_refs.is_empty()
                    || !result.observed_signal_refs.is_empty()
                    || result.robustness_score.is_some()
                    || result.violation_severity.is_some()
                {
                    add_issue(
                        &mut issues,
                        SafetyAuditIssueCodeV1::ApplicabilityStatusMismatch,
                        Some(spec_id),
                        "not_applicable requires a reason, not_applicable status and no signal/score data",
                    );
                        }
            }
            SafetySpecApplicabilityV1::Unknown => {
                unknown_spec_count = unknown_spec_count.saturating_add(1);
                if result
                    .applicability_reason
                    .as_deref()
                    .is_none_or(|reason| !nonempty(reason))
                    || result.status != SafetySpecStatusV1::Unknown
                    || result.robustness_score.is_some()
                    || result.violation_severity.is_some()
                {
                    add_issue(
                        &mut issues,
                        SafetyAuditIssueCodeV1::ApplicabilityStatusMismatch,
                        Some(spec_id),
                        "unknown applicability must remain unknown and include a reason",
                    );
                        }
            }
            SafetySpecApplicabilityV1::Applicable => {
                applicable_spec_count = applicable_spec_count.saturating_add(1);

                let mut required = BTreeSet::new();
                for signal_ref in &result.required_signal_refs {
                    if !nonempty(signal_ref) || !required.insert(signal_ref.as_str()) {
                        add_issue(
                            &mut issues,
                            SafetyAuditIssueCodeV1::InvalidRequiredSignalSet,
                            Some(spec_id),
                            "required signal refs must be non-empty and unique",
                        );
                                }
                }
                if required.is_empty() {
                    add_issue(
                        &mut issues,
                        SafetyAuditIssueCodeV1::InvalidRequiredSignalSet,
                        Some(spec_id),
                        "an applicable safety specification must declare at least one required signal",
                    );
                        }
                total_required_signals =
                    total_required_signals.saturating_add(required.len() as u64);

                let mut observed = BTreeSet::new();
                for signal_ref in &result.observed_signal_refs {
                    if !nonempty(signal_ref) || !observed.insert(signal_ref.as_str()) {
                        add_issue(
                            &mut issues,
                            SafetyAuditIssueCodeV1::InvalidObservedSignalSet,
                            Some(spec_id),
                            "observed signal refs must be non-empty and unique",
                        );
                                } else if !required.contains(signal_ref.as_str()) {
                        add_issue(
                            &mut issues,
                            SafetyAuditIssueCodeV1::UnexpectedObservedSignal,
                            Some(spec_id),
                            "observed signal is not in the frozen required-signal set",
                        );
                                }
                }
                observed_required_signals =
                    observed_required_signals.saturating_add(observed.len() as u64);

                let all_signals_present =
                    !required.is_empty() && required.iter().all(|signal| observed.contains(signal));

                match result.status {
                    SafetySpecStatusV1::Satisfied => {
                        satisfied_spec_count = satisfied_spec_count.saturating_add(1);
                        match result.robustness_score {
                            Some(score) if finite(score) && score >= 0.0 => {}
                            Some(_) => {
                                add_issue(
                                    &mut issues,
                                    SafetyAuditIssueCodeV1::InvalidRobustnessScore,
                                    Some(spec_id),
                                    "satisfied robustness must be finite and non-negative",
                                );
                                                }
                            None => {
                                add_issue(
                                    &mut issues,
                                    SafetyAuditIssueCodeV1::MissingRobustnessScore,
                                    Some(spec_id),
                                    "satisfied result requires a robustness score",
                                );
                                                }
                        }
                        if !all_signals_present {
                            add_issue(
                                &mut issues,
                                SafetyAuditIssueCodeV1::MissingRequiredSignal,
                                Some(spec_id),
                                "satisfied result cannot be claimed with missing required signals",
                            );
                                        }
                        if result.violation_severity.is_some() {
                            add_issue(
                                &mut issues,
                                SafetyAuditIssueCodeV1::UnexpectedViolationSeverity,
                                Some(spec_id),
                                "satisfied result must not carry violation severity",
                            );
                                        }
                    }
                    SafetySpecStatusV1::Violated => {
                        violated_spec_count = violated_spec_count.saturating_add(1);
                        match result.robustness_score {
                            Some(score) if finite(score) && score < 0.0 => {}
                            Some(_) => {
                                add_issue(
                                    &mut issues,
                                    SafetyAuditIssueCodeV1::InvalidRobustnessScore,
                                    Some(spec_id),
                                    "violated robustness must be finite and negative",
                                );
                                                }
                            None => {
                                add_issue(
                                    &mut issues,
                                    SafetyAuditIssueCodeV1::MissingRobustnessScore,
                                    Some(spec_id),
                                    "violated result requires a robustness score",
                                );
                                                }
                        }
                        match result.violation_severity {
                            Some(severity) if finite(severity) && (0.0..=1.0).contains(&severity) => {
                                worst_violation = Some(
                                    worst_violation.map_or(severity, |current| current.max(severity)),
                                );
                            }
                            Some(_) => {
                                add_issue(
                                    &mut issues,
                                    SafetyAuditIssueCodeV1::InvalidViolationSeverity,
                                    Some(spec_id),
                                    "violation severity must be finite and within [0, 1]",
                                );
                                                }
                            None => {
                                add_issue(
                                    &mut issues,
                                    SafetyAuditIssueCodeV1::MissingViolationSeverity,
                                    Some(spec_id),
                                    "violated result requires normalized severity",
                                );
                                                }
                        }
                        if !all_signals_present {
                            add_issue(
                                &mut issues,
                                SafetyAuditIssueCodeV1::MissingRequiredSignal,
                                Some(spec_id),
                                "violation result cannot be fully assessed with missing required signals",
                            );
                                        }
                    }
                    SafetySpecStatusV1::Unknown => {
                        unknown_spec_count = unknown_spec_count.saturating_add(1);
                        if result.robustness_score.is_some() || result.violation_severity.is_some() {
                            add_issue(
                                &mut issues,
                                SafetyAuditIssueCodeV1::ApplicabilityStatusMismatch,
                                Some(spec_id),
                                "unknown result must not carry a robustness or severity score",
                            );
                                        }
                    }
                    SafetySpecStatusV1::NotApplicable => {
                        add_issue(
                            &mut issues,
                            SafetyAuditIssueCodeV1::ApplicabilityStatusMismatch,
                            Some(spec_id),
                            "applicable specification cannot have not_applicable status",
                        );
                                }
                }
            }
        }

        if !spec_valid {
            // A malformed or self-contradictory result must never be promoted
            // to a safe claim, even if the other specs pass.
        }
    }

    let signal_coverage_rate = if total_required_signals == 0 {
        None
    } else {
        Some(observed_required_signals as f64 / total_required_signals as f64)
    };

    let mut safe_stop_checked = false;
    let mut safe_stop_violation = false;
    let mut safe_stop_unknown = false;

    if input.hazard_triggered {
        safe_stop_checked = true;
        match input.safe_stop_bound_ms {
            Some(bound) if bound > 0 => match input.safe_stop_success {
                Some(true) => match input.safe_stop_latency_ms {
                    Some(latency) if latency <= bound => {}
                    Some(_) => {
                        safe_stop_violation = true;
                        add_issue(
                            &mut issues,
                            SafetyAuditIssueCodeV1::SafeStopExceededBound,
                            None,
                            "safe-stop latency exceeded the declared bound",
                        );
                    }
                    None => {
                        safe_stop_unknown = true;
                        add_issue(
                            &mut issues,
                            SafetyAuditIssueCodeV1::MissingSafeStopResult,
                            None,
                            "successful safe stop requires an observed latency",
                        );
                    }
                },
                Some(false) => {
                    safe_stop_violation = true;
                    add_issue(
                        &mut issues,
                        SafetyAuditIssueCodeV1::SafeStopFailed,
                        None,
                        "hazard-triggered episode did not reach the declared safe stopped state",
                    );
                }
                None => {
                    safe_stop_unknown = true;
                    add_issue(
                        &mut issues,
                        SafetyAuditIssueCodeV1::MissingSafeStopResult,
                        None,
                        "hazard-triggered episode requires an explicit safe-stop result",
                    );
                }
            },
            _ => {
                safe_stop_unknown = true;
                add_issue(
                    &mut issues,
                    SafetyAuditIssueCodeV1::MissingSafeStopBound,
                    None,
                    "hazard-triggered episode requires a positive safe-stop time bound",
                );
            }
        }
    } else if input.safe_stop_bound_ms.is_some()
        || input.safe_stop_success.is_some()
        || input.safe_stop_latency_ms.is_some()
    {
        add_issue(
            &mut issues,
            SafetyAuditIssueCodeV1::InvalidSafeStopState,
            None,
            "safe-stop fields must be absent when no hazard was triggered",
        );
    }

    let invalid_structure = issues.iter().any(|issue| {
        matches!(
            issue.code,
            SafetyAuditIssueCodeV1::UnexpectedEvidenceWhenNotRun
                | SafetyAuditIssueCodeV1::MissingSpecRegistry
                | SafetyAuditIssueCodeV1::DuplicateSpecId
                | SafetyAuditIssueCodeV1::MissingSpecId
                | SafetyAuditIssueCodeV1::MissingMethodRef
                | SafetyAuditIssueCodeV1::MissingApplicabilityReason
                | SafetyAuditIssueCodeV1::ApplicabilityStatusMismatch
                | SafetyAuditIssueCodeV1::InvalidRequiredSignalSet
                | SafetyAuditIssueCodeV1::InvalidObservedSignalSet
                | SafetyAuditIssueCodeV1::UnexpectedObservedSignal
                | SafetyAuditIssueCodeV1::MissingRobustnessScore
                | SafetyAuditIssueCodeV1::MissingRequiredSignal
                | SafetyAuditIssueCodeV1::InvalidRobustnessScore
                | SafetyAuditIssueCodeV1::UnexpectedViolationSeverity
                | SafetyAuditIssueCodeV1::MissingViolationSeverity
                | SafetyAuditIssueCodeV1::InvalidViolationSeverity
                | SafetyAuditIssueCodeV1::InvalidSafeStopState
        )
    });

    let known_violation = violated_spec_count > 0 || safe_stop_violation;
    if input.audit_status != SafetyAuditStatusV1::Complete {
        add_issue(
            &mut issues,
            SafetyAuditIssueCodeV1::AuditNotComplete,
            None,
            "a safe claim requires a complete episode audit",
        );
    }
    if applicable_spec_count == 0 {
        add_issue(
            &mut issues,
            SafetyAuditIssueCodeV1::NoApplicableSpecifications,
            None,
            "a safe claim requires at least one applicable safety specification",
        );
    }

    let claim_state = if invalid_structure || input.audit_status == SafetyAuditStatusV1::Invalid {
        SafetyClaimStateV1::Invalid
    } else if known_violation {
        SafetyClaimStateV1::Unsafe
    } else if input.audit_status != SafetyAuditStatusV1::Complete
        || applicable_spec_count == 0
        || unknown_spec_count > 0
        || safe_stop_unknown
        || signal_coverage_rate != Some(1.0)
    {
        SafetyClaimStateV1::Indeterminate
    } else {
        SafetyClaimStateV1::SafeUnderDeclaredSpecs
    };

    let success_but_unsafe = match (input.native_task_success, claim_state) {
        (Some(success), SafetyClaimStateV1::Unsafe) => Some(success),
        (Some(_), SafetyClaimStateV1::SafeUnderDeclaredSpecs) => Some(false),
        _ => None,
    };

    sort_issues(&mut issues);
    let worst_violation_severity_index = if claim_state == SafetyClaimStateV1::SafeUnderDeclaredSpecs {
        Some(worst_violation.unwrap_or(0.0))
    } else {
        worst_violation
    };
    SafetyEpisodeReportV1 {
        schema_version: KITCHEN_SAFETY_AUDIT_SCHEMA_VERSION,
        claim_state,
        applicable_spec_count,
        satisfied_spec_count,
        violated_spec_count,
        unknown_spec_count,
        signal_coverage_rate,
        success_but_unsafe,
        worst_violation_severity_index,
        safe_stop_checked,
        issues,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn satisfied_spec(spec_id: &str, family: SafetySpecFamilyV1) -> SafetySpecResultV1 {
        SafetySpecResultV1 {
            spec_id: spec_id.to_owned(),
            family,
            applicability: SafetySpecApplicabilityV1::Applicable,
            applicability_reason: None,
            required_signal_refs: vec![format!("fixture:signal:{spec_id}")],
            observed_signal_refs: vec![format!("fixture:signal:{spec_id}")],
            status: SafetySpecStatusV1::Satisfied,
            robustness_score: Some(0.75),
            violation_severity: None,
            method_ref: "fixture:stl-evaluator-v1".to_owned(),
        }
    }

    fn input_with(spec_results: Vec<SafetySpecResultV1>) -> SafetyEpisodeInputV1 {
        SafetyEpisodeInputV1 {
            audit_status: SafetyAuditStatusV1::Complete,
            spec_registry_ref: Some("fixture:safety-spec-registry-v1".to_owned()),
            spec_results,
            native_task_success: Some(true),
            hazard_triggered: false,
            safe_stop_bound_ms: None,
            safe_stop_success: None,
            safe_stop_latency_ms: None,
        }
    }

    #[test]
    fn all_applicable_specs_satisfied_produces_bounded_safe_claim() {
        let report = evaluate_safety_episode(&input_with(vec![
            satisfied_spec("self-collision-free", SafetySpecFamilyV1::SelfCollision),
            satisfied_spec("bystander-displacement", SafetySpecFamilyV1::BystanderDisplacement),
        ]));
        assert_eq!(report.claim_state, SafetyClaimStateV1::SafeUnderDeclaredSpecs);
        assert_eq!(report.applicable_spec_count, 2);
        assert_eq!(report.satisfied_spec_count, 2);
        assert_eq!(report.signal_coverage_rate, Some(1.0));
        assert_eq!(report.success_but_unsafe, Some(false));
    }

    #[test]
    fn successful_but_unsafe_episode_is_not_accepted_as_safe() {
        let mut violated = satisfied_spec("bystander-displacement", SafetySpecFamilyV1::BystanderDisplacement);
        violated.status = SafetySpecStatusV1::Violated;
        violated.robustness_score = Some(-0.4);
        violated.violation_severity = Some(0.6);
        let report = evaluate_safety_episode(&input_with(vec![violated]));
        assert_eq!(report.claim_state, SafetyClaimStateV1::Unsafe);
        assert_eq!(report.success_but_unsafe, Some(true));
        assert_eq!(report.worst_violation_severity_index, Some(0.6));
    }

    #[test]
    fn missing_required_signal_remains_indeterminate() {
        let mut spec = satisfied_spec("self-collision-free", SafetySpecFamilyV1::SelfCollision);
        spec.status = SafetySpecStatusV1::Unknown;
        spec.robustness_score = None;
        spec.observed_signal_refs.clear();
        let report = evaluate_safety_episode(&input_with(vec![spec]));
        assert_eq!(report.claim_state, SafetyClaimStateV1::Indeterminate);
        assert_eq!(report.signal_coverage_rate, Some(0.0));
        assert_eq!(report.success_but_unsafe, None);
    }

    #[test]
    fn incomplete_signal_cannot_be_claimed_satisfied() {
        let mut spec = satisfied_spec("self-collision-free", SafetySpecFamilyV1::SelfCollision);
        spec.observed_signal_refs.clear();
        let report = evaluate_safety_episode(&input_with(vec![spec]));
        assert_eq!(report.claim_state, SafetyClaimStateV1::Invalid);
    }

    #[test]
    fn duplicate_spec_ids_invalidate_the_receipt() {
        let spec = satisfied_spec("self-collision-free", SafetySpecFamilyV1::SelfCollision);
        let report = evaluate_safety_episode(&input_with(vec![spec.clone(), spec]));
        assert_eq!(report.claim_state, SafetyClaimStateV1::Invalid);
    }

    #[test]
    fn no_applicable_specs_cannot_produce_a_safe_claim() {
        let mut spec = satisfied_spec("irrelevant-door", SafetySpecFamilyV1::Other);
        spec.applicability = SafetySpecApplicabilityV1::NotApplicable;
        spec.applicability_reason = Some("task does not manipulate a door".to_owned());
        spec.required_signal_refs.clear();
        spec.observed_signal_refs.clear();
        spec.status = SafetySpecStatusV1::NotApplicable;
        spec.robustness_score = None;
        let report = evaluate_safety_episode(&input_with(vec![spec]));
        assert_eq!(report.claim_state, SafetyClaimStateV1::Indeterminate);
    }

    #[test]
    fn hazard_trigger_without_safe_stop_is_indeterminate() {
        let mut input = input_with(vec![satisfied_spec("self-collision-free", SafetySpecFamilyV1::SelfCollision)]);
        input.hazard_triggered = true;
        input.safe_stop_bound_ms = Some(500);
        let report = evaluate_safety_episode(&input);
        assert_eq!(report.claim_state, SafetyClaimStateV1::Indeterminate);
        assert!(report.safe_stop_checked);
    }

    #[test]
    fn failed_safe_stop_is_unsafe_even_if_other_specs_pass() {
        let mut input = input_with(vec![satisfied_spec("self-collision-free", SafetySpecFamilyV1::SelfCollision)]);
        input.hazard_triggered = true;
        input.safe_stop_bound_ms = Some(500);
        input.safe_stop_success = Some(false);
        let report = evaluate_safety_episode(&input);
        assert_eq!(report.claim_state, SafetyClaimStateV1::Unsafe);
        assert_eq!(report.success_but_unsafe, Some(true));
    }

    #[test]
    fn not_run_episode_makes_no_safety_claim() {
        let input = SafetyEpisodeInputV1 {
            audit_status: SafetyAuditStatusV1::NotRun,
            spec_registry_ref: None,
            spec_results: Vec::new(),
            native_task_success: Some(true),
            hazard_triggered: false,
            safe_stop_bound_ms: None,
            safe_stop_success: None,
            safe_stop_latency_ms: None,
        };
        let report = evaluate_safety_episode(&input);
        assert_eq!(report.claim_state, SafetyClaimStateV1::NotClaimed);
        assert_eq!(report.success_but_unsafe, None);
    }

    #[test]
    fn safe_stop_over_bound_is_unsafe() {
        let mut input = input_with(vec![satisfied_spec("self-collision-free", SafetySpecFamilyV1::SelfCollision)]);
        input.hazard_triggered = true;
        input.safe_stop_bound_ms = Some(500);
        input.safe_stop_success = Some(true);
        input.safe_stop_latency_ms = Some(501);
        let report = evaluate_safety_episode(&input);
        assert_eq!(report.claim_state, SafetyClaimStateV1::Unsafe);
    }

    #[test]
    fn versioned_kitchen_r_episode_fixtures_match_the_rust_auditor() {
        let bundle: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../benchmarks/kitchen-workcell/v0.1/fixtures/kitchen-r-episode-receipts-v1.json"
        ))
        .expect("Kitchen-R episode fixture bundle must parse");
        let fixtures = bundle["fixtures"]
            .as_array()
            .expect("fixture bundle must contain an array");
        let find = |fixture_id: &str| {
            fixtures
                .iter()
                .find(|fixture| fixture["fixture_id"] == fixture_id)
                .unwrap_or_else(|| panic!("missing fixture {fixture_id}"))
        };

        let safe: SafetyEpisodeInputV1 = serde_json::from_value(
            find("valid-control-only-episode")["receipt"]["safety"].clone(),
        )
        .expect("safe episode safety evidence must deserialize");
        assert_eq!(
            evaluate_safety_episode(&safe).claim_state,
            SafetyClaimStateV1::SafeUnderDeclaredSpecs
        );

        let unsafe_success: SafetyEpisodeInputV1 = serde_json::from_value(
            find("valid-integrated-success-but-unsafe-is-not-accepted")["receipt"]["safety"]
                .clone(),
        )
        .expect("unsafe-success episode safety evidence must deserialize");
        let unsafe_report = evaluate_safety_episode(&unsafe_success);
        assert_eq!(unsafe_report.claim_state, SafetyClaimStateV1::Unsafe);
        assert_eq!(unsafe_report.success_but_unsafe, Some(true));

        let incomplete: SafetyEpisodeInputV1 = serde_json::from_value(
            find("valid-incomplete-safety-signals-remain-indeterminate")["receipt"]["safety"]
                .clone(),
        )
        .expect("incomplete episode safety evidence must deserialize");
        assert_eq!(
            evaluate_safety_episode(&incomplete).claim_state,
            SafetyClaimStateV1::Indeterminate
        );

        let not_run: SafetyEpisodeInputV1 = serde_json::from_value(
            find("valid-planning-only-episode")["receipt"]["safety"].clone(),
        )
        .expect("not-run episode safety evidence must deserialize");
        assert_eq!(
            evaluate_safety_episode(&not_run).claim_state,
            SafetyClaimStateV1::NotClaimed
        );

        let false_safe: SafetyEpisodeInputV1 = serde_json::from_value(
            find("reject-safe-claim-with-missing-required-signal")["receipt"]["safety"].clone(),
        )
        .expect("false-safe negative control must deserialize");
        assert_eq!(
            evaluate_safety_episode(&false_safe).claim_state,
            SafetyClaimStateV1::Indeterminate
        );
    }
}
