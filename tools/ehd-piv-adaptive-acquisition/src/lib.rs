// SPDX-License-Identifier: AGPL-3.0-or-later

//! Deterministic risk-triggered scheduling for multi-fidelity diagnostics.
//!
//! This controller schedules measurement actions only. It does not actuate
//! electrodes, change discharge conditions, or authorize physical experiments.
//! Hard evidence and observer-state requirements take precedence over cost.
//!
//! The action selector solves a bounded set-cover problem over available
//! diagnostic capabilities, minimizing cost and then latency. It is a
//! transparent deterministic baseline, not a globally optimal Bayesian
//! expected-value-of-information policy.

#![forbid(unsafe_code)]

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

pub const MAX_DIAGNOSTIC_OPTIONS: usize = 16;
const METRIC_NONNEGATIVE: bool = true;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum FlowRegion {
    Bulk,
    NearWall,
    ShearLayer,
    Wake,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceTarget {
    SentinelMonitoring,
    FieldCovariance,
    ThreeDimensionalVelocity,
    IntegratedForce,
    ThrustClaim,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DiagnosticCapability {
    SentinelReacquisition,
    HighResolutionPiv,
    VolumetricLpt,
    IndependentVelocityReference,
    DirectForceMeasurement,
    CompleteControlVolume,
    ElectricalPowerAccounting,
    SpatialRegionCoverage,
    ConservationAudit,
    ReferenceQualification,
    ObserverRequalification,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiagnosticKind {
    SentinelPiv,
    HighResolutionPiv,
    StereoOrVolumetricPiv,
    LptShakeTheBox,
    IndependentVelocityReference,
    DirectForceBalance,
    ControlVolumeReconstruction,
    ElectricalPowerMeasurement,
    ReferenceRequalification,
    ObserverWashoutAndRequalification,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObserverImpact {
    AssessedNonPerturbing,
    PotentiallyPerturbing,
    Unknown,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DiagnosticOption {
    pub action_id: String,
    pub kind: DiagnosticKind,
    pub cost_units: u32,
    pub latency_seconds: f64,
    pub capabilities: BTreeSet<DiagnosticCapability>,
    pub available: bool,
    pub observer_impact: ObserverImpact,
    /// Every selected diagnostic action must generate a bound, replayable
    /// evidence receipt. A capability without a receipt cannot close a gate.
    pub evidence_receipt_required: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SentinelSnapshot {
    pub acquisition_id: String,
    pub dependence_cluster_id: String,
    pub protocol_fingerprint: String,
    pub observer_state_id: String,
    pub observer_state_qualified: bool,
    pub reference_evidence_qualified: bool,
    pub diagnostic_quality_qualified: bool,
    pub uncertainty_ratio: Option<f64>,
    pub out_of_domain_score: Option<f64>,
    pub model_disagreement_sigma: Option<f64>,
    pub conservation_residual_sigma: Option<f64>,
    pub seconds_since_last_high_fidelity: Option<f64>,
    pub observed_regions: BTreeSet<FlowRegion>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TriggerThresholds {
    /// Field- or functional-uncertainty bound divided by the predeclared limit.
    pub maximum_uncertainty_ratio: f64,
    pub maximum_out_of_domain_score: f64,
    pub maximum_model_disagreement_sigma: f64,
    pub maximum_conservation_residual_sigma: f64,
    pub maximum_high_fidelity_age_seconds: f64,
    pub required_regions: BTreeSet<FlowRegion>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ScheduleConfig {
    pub measurement_budget_units: u32,
    pub maximum_total_latency_seconds: f64,
    pub maximum_selected_actions: usize,
    pub thresholds: TriggerThresholds,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum TriggerReason {
    LocalUncertaintyExceeded,
    OutOfDomain,
    IndependentDiagnosticsDisagree,
    ConservationResidualExceeded,
    HighFidelityAcquisitionStale,
    SpatialCoverageMissing(FlowRegion),
    DiagnosticQualityFailed,
    ReferenceEvidenceUnqualified,
    ObserverStateUnqualified,
    SentinelTelemetryIncomplete(&'static str),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScheduleDisposition {
    ContinueSentinel,
    AcquireDiagnostics,
    Abstain,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AbstentionReason {
    NoAvailableActionCoversRequiredCapabilities,
    MeasurementBudgetInsufficient,
    LatencyLimitExceeded,
    SelectedActionLimitExceeded,
    ObserverRequalificationUnavailable,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ScheduleError {
    EmptyIdentity { field: &'static str },
    InvalidThreshold { field: &'static str },
    InvalidSnapshotMetric { field: &'static str },
    InvalidBudget,
    InvalidLatencyLimit,
    InvalidActionLimit,
    TooManyDiagnosticOptions { actual: usize, maximum: usize },
    EmptyActionId { index: usize },
    DuplicateActionId(String),
    InvalidActionCost { action_id: String },
    InvalidActionLatency { action_id: String },
    EmptyActionCapabilities { action_id: String },
    EvidenceReceiptNotRequired { action_id: String },
    NonFiniteAccumulation,
}

impl fmt::Display for ScheduleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyIdentity { field } => write!(f, "{field} must be non-empty"),
            Self::InvalidThreshold { field } => {
                write!(f, "schedule threshold {field} must be finite and positive")
            }
            Self::InvalidSnapshotMetric { field } => write!(
                f,
                "sentinel metric {field} must be finite and non-negative when present"
            ),
            Self::InvalidBudget => write!(f, "measurement budget must be positive"),
            Self::InvalidLatencyLimit => {
                write!(f, "maximum total diagnostic latency must be finite and positive")
            }
            Self::InvalidActionLimit => write!(f, "maximum selected-action count is invalid"),
            Self::TooManyDiagnosticOptions { actual, maximum } => {
                write!(f, "diagnostic option count {actual} exceeds limit {maximum}")
            }
            Self::EmptyActionId { index } => write!(f, "diagnostic action {index} has an empty ID"),
            Self::DuplicateActionId(id) => write!(f, "duplicate diagnostic action ID: {id}"),
            Self::InvalidActionCost { action_id } => {
                write!(f, "diagnostic action {action_id} must have positive cost")
            }
            Self::InvalidActionLatency { action_id } => {
                write!(f, "diagnostic action {action_id} has invalid latency")
            }
            Self::EmptyActionCapabilities { action_id } => {
                write!(f, "diagnostic action {action_id} has no capability declaration")
            }
            Self::EvidenceReceiptNotRequired { action_id } => write!(
                f,
                "diagnostic action {action_id} cannot be selected without a required evidence receipt"
            ),
            Self::NonFiniteAccumulation => {
                write!(f, "diagnostic schedule accumulation is non-finite or overflowed")
            }
        }
    }
}

impl Error for ScheduleError {}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RejectedSubsetCounts {
    pub over_budget: usize,
    pub over_latency: usize,
    pub observer_requalification_missing: usize,
    pub missing_capability_coverage: usize,
    pub over_action_limit: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ScheduleReport {
    pub disposition: ScheduleDisposition,
    pub target: EvidenceTarget,
    pub trigger_reasons: Vec<TriggerReason>,
    pub required_capabilities: BTreeSet<DiagnosticCapability>,
    pub selected_actions: Vec<DiagnosticOption>,
    pub total_cost_units: u32,
    pub total_latency_seconds: f64,
    pub observer_requalification_required: bool,
    pub rejected_subset_counts: RejectedSubsetCounts,
    pub abstention_reason: Option<AbstentionReason>,
    pub claim_boundary: &'static str,
}

#[derive(Clone, Debug)]
struct CandidatePlan {
    option_indices: Vec<usize>,
    action_ids: Vec<String>,
    cost: u32,
    latency: f64,
    observer_requalification_required: bool,
}

pub fn schedule_diagnostics(
    target: EvidenceTarget,
    snapshot: &SentinelSnapshot,
    config: &ScheduleConfig,
    options: &[DiagnosticOption],
) -> Result<ScheduleReport, ScheduleError> {
    validate_config(config)?;
    validate_snapshot(snapshot)?;
    validate_options(options)?;

    let mut reasons = BTreeSet::new();
    let mut required = target_capabilities(target);

    evaluate_metric(
        snapshot.uncertainty_ratio,
        "uncertainty_ratio",
        config.thresholds.maximum_uncertainty_ratio,
        TriggerReason::LocalUncertaintyExceeded,
        DiagnosticCapability::HighResolutionPiv,
        &mut reasons,
        &mut required,
    );
    evaluate_metric(
        snapshot.out_of_domain_score,
        "out_of_domain_score",
        config.thresholds.maximum_out_of_domain_score,
        TriggerReason::OutOfDomain,
        DiagnosticCapability::VolumetricLpt,
        &mut reasons,
        &mut required,
    );
    evaluate_metric(
        snapshot.model_disagreement_sigma,
        "model_disagreement_sigma",
        config.thresholds.maximum_model_disagreement_sigma,
        TriggerReason::IndependentDiagnosticsDisagree,
        DiagnosticCapability::IndependentVelocityReference,
        &mut reasons,
        &mut required,
    );
    evaluate_metric(
        snapshot.conservation_residual_sigma,
        "conservation_residual_sigma",
        config.thresholds.maximum_conservation_residual_sigma,
        TriggerReason::ConservationResidualExceeded,
        DiagnosticCapability::ConservationAudit,
        &mut reasons,
        &mut required,
    );

    match snapshot.seconds_since_last_high_fidelity {
        Some(age) if age >= config.thresholds.maximum_high_fidelity_age_seconds => {
            reasons.insert(TriggerReason::HighFidelityAcquisitionStale);
            required.insert(DiagnosticCapability::HighResolutionPiv);
        }
        Some(_) => {}
        None => {
            reasons.insert(TriggerReason::SentinelTelemetryIncomplete(
                "seconds_since_last_high_fidelity",
            ));
            required.insert(DiagnosticCapability::SentinelReacquisition);
        }
    }

    for region in &config.thresholds.required_regions {
        if !snapshot.observed_regions.contains(region) {
            reasons.insert(TriggerReason::SpatialCoverageMissing(*region));
            required.insert(DiagnosticCapability::SpatialRegionCoverage);
        }
    }

    if !snapshot.diagnostic_quality_qualified {
        reasons.insert(TriggerReason::DiagnosticQualityFailed);
        required.insert(DiagnosticCapability::SentinelReacquisition);
    }
    if !snapshot.reference_evidence_qualified {
        reasons.insert(TriggerReason::ReferenceEvidenceUnqualified);
        required.insert(DiagnosticCapability::ReferenceQualification);
    }
    if !snapshot.observer_state_qualified {
        reasons.insert(TriggerReason::ObserverStateUnqualified);
        required.insert(DiagnosticCapability::ObserverRequalification);
    }

    let trigger_reasons: Vec<TriggerReason> = reasons.into_iter().collect();
    if trigger_reasons.is_empty() && required.is_empty() {
        return Ok(ScheduleReport {
            disposition: ScheduleDisposition::ContinueSentinel,
            target,
            trigger_reasons,
            required_capabilities: required,
            selected_actions: Vec::new(),
            total_cost_units: 0,
            total_latency_seconds: 0.0,
            observer_requalification_required: false,
            rejected_subset_counts: RejectedSubsetCounts::default(),
            abstention_reason: None,
            claim_boundary: "sentinel continuation only; not a physical validation or thrust authorization",
        });
    }

    let (best, rejected) = select_capability_cover(&required, options, config)?;
    match best {
        Some(plan) => {
            let selected_actions = plan
                .option_indices
                .iter()
                .map(|index| options[*index].clone())
                .collect();
            Ok(ScheduleReport {
                disposition: ScheduleDisposition::AcquireDiagnostics,
                target,
                trigger_reasons,
                required_capabilities: required,
                selected_actions,
                total_cost_units: plan.cost,
                total_latency_seconds: plan.latency,
                observer_requalification_required: plan.observer_requalification_required,
                rejected_subset_counts: rejected,
                abstention_reason: None,
                claim_boundary: "schedules diagnostics only; claims remain blocked until required evidence receipts are reconciled",
            })
        }
        None => {
            let abstention_reason = determine_abstention(&required, options, config, &rejected);
            Ok(ScheduleReport {
                disposition: ScheduleDisposition::Abstain,
                target,
                trigger_reasons,
                required_capabilities: required,
                selected_actions: Vec::new(),
                total_cost_units: 0,
                total_latency_seconds: 0.0,
                observer_requalification_required: false,
                rejected_subset_counts: rejected,
                abstention_reason: Some(abstention_reason),
                claim_boundary: "no measurement action authorized by this scheduler; maintain current safe state and retain unresolved evidence status",
            })
        }
    }
}

fn validate_config(config: &ScheduleConfig) -> Result<(), ScheduleError> {
    if config.measurement_budget_units == 0 {
        return Err(ScheduleError::InvalidBudget);
    }
    if !config.maximum_total_latency_seconds.is_finite()
        || config.maximum_total_latency_seconds <= 0.0
    {
        return Err(ScheduleError::InvalidLatencyLimit);
    }
    if config.maximum_selected_actions == 0 || config.maximum_selected_actions > MAX_DIAGNOSTIC_OPTIONS {
        return Err(ScheduleError::InvalidActionLimit);
    }
    for (field, value) in [
        ("maximum_uncertainty_ratio", config.thresholds.maximum_uncertainty_ratio),
        ("maximum_out_of_domain_score", config.thresholds.maximum_out_of_domain_score),
        (
            "maximum_model_disagreement_sigma",
            config.thresholds.maximum_model_disagreement_sigma,
        ),
        (
            "maximum_conservation_residual_sigma",
            config.thresholds.maximum_conservation_residual_sigma,
        ),
        (
            "maximum_high_fidelity_age_seconds",
            config.thresholds.maximum_high_fidelity_age_seconds,
        ),
    ] {
        if !value.is_finite() || value <= 0.0 {
            return Err(ScheduleError::InvalidThreshold { field });
        }
    }
    if config.thresholds.required_regions.is_empty() {
        return Err(ScheduleError::InvalidThreshold {
            field: "required_regions",
        });
    }
    Ok(())
}

fn validate_snapshot(snapshot: &SentinelSnapshot) -> Result<(), ScheduleError> {
    for (field, value) in [
        ("acquisition_id", snapshot.acquisition_id.as_str()),
        ("dependence_cluster_id", snapshot.dependence_cluster_id.as_str()),
        ("protocol_fingerprint", snapshot.protocol_fingerprint.as_str()),
        ("observer_state_id", snapshot.observer_state_id.as_str()),
    ] {
        if value.trim().is_empty() || value.chars().any(char::is_control) {
            return Err(ScheduleError::EmptyIdentity { field });
        }
    }
    for (field, value) in [
        ("uncertainty_ratio", snapshot.uncertainty_ratio),
        ("out_of_domain_score", snapshot.out_of_domain_score),
        ("model_disagreement_sigma", snapshot.model_disagreement_sigma),
        ("conservation_residual_sigma", snapshot.conservation_residual_sigma),
        (
            "seconds_since_last_high_fidelity",
            snapshot.seconds_since_last_high_fidelity,
        ),
    ] {
        if value.is_some_and(|value| {
            !value.is_finite() || (METRIC_NONNEGATIVE && value < 0.0)
        }) {
            return Err(ScheduleError::InvalidSnapshotMetric { field });
        }
    }
    Ok(())
}

fn validate_options(options: &[DiagnosticOption]) -> Result<(), ScheduleError> {
    if options.len() > MAX_DIAGNOSTIC_OPTIONS {
        return Err(ScheduleError::TooManyDiagnosticOptions {
            actual: options.len(),
            maximum: MAX_DIAGNOSTIC_OPTIONS,
        });
    }
    let mut ids = BTreeSet::new();
    for (index, option) in options.iter().enumerate() {
        if option.action_id.trim().is_empty()
            || option.action_id.chars().any(char::is_control)
        {
            return Err(ScheduleError::EmptyActionId { index });
        }
        if !ids.insert(option.action_id.as_str()) {
            return Err(ScheduleError::DuplicateActionId(
                option.action_id.clone(),
            ));
        }
        if option.cost_units == 0 {
            return Err(ScheduleError::InvalidActionCost {
                action_id: option.action_id.clone(),
            });
        }
        if !option.latency_seconds.is_finite() || option.latency_seconds <= 0.0 {
            return Err(ScheduleError::InvalidActionLatency {
                action_id: option.action_id.clone(),
            });
        }
        if option.capabilities.is_empty() {
            return Err(ScheduleError::EmptyActionCapabilities {
                action_id: option.action_id.clone(),
            });
        }
        if !option.evidence_receipt_required {
            return Err(ScheduleError::EvidenceReceiptNotRequired {
                action_id: option.action_id.clone(),
            });
        }
    }
    Ok(())
}

fn evaluate_metric(
    value: Option<f64>,
    field: &'static str,
    threshold: f64,
    reason: TriggerReason,
    capability: DiagnosticCapability,
    reasons: &mut BTreeSet<TriggerReason>,
    required: &mut BTreeSet<DiagnosticCapability>,
) {
    match value {
        Some(value) if value >= threshold => {
            reasons.insert(reason);
            required.insert(capability);
        }
        Some(_) => {}
        None => {
            reasons.insert(TriggerReason::SentinelTelemetryIncomplete(field));
            required.insert(DiagnosticCapability::SentinelReacquisition);
        }
    }
}

fn target_capabilities(target: EvidenceTarget) -> BTreeSet<DiagnosticCapability> {
    match target {
        EvidenceTarget::SentinelMonitoring => BTreeSet::new(),
        EvidenceTarget::FieldCovariance => BTreeSet::new(),
        EvidenceTarget::ThreeDimensionalVelocity => BTreeSet::from([
            DiagnosticCapability::VolumetricLpt,
            DiagnosticCapability::IndependentVelocityReference,
        ]),
        EvidenceTarget::IntegratedForce => BTreeSet::from([
            DiagnosticCapability::DirectForceMeasurement,
            DiagnosticCapability::CompleteControlVolume,
        ]),
        EvidenceTarget::ThrustClaim => BTreeSet::from([
            DiagnosticCapability::DirectForceMeasurement,
            DiagnosticCapability::CompleteControlVolume,
            DiagnosticCapability::IndependentVelocityReference,
            DiagnosticCapability::ElectricalPowerAccounting,
        ]),
    }
}

fn select_capability_cover(
    required: &BTreeSet<DiagnosticCapability>,
    options: &[DiagnosticOption],
    config: &ScheduleConfig,
) -> Result<(Option<CandidatePlan>, RejectedSubsetCounts), ScheduleError> {
    let usable_indices: Vec<usize> = options
        .iter()
        .enumerate()
        .filter_map(|(index, option)| option.available.then_some(index))
        .collect();
    let subset_limit = 1_u32
        .checked_shl(usable_indices.len() as u32)
        .ok_or(ScheduleError::NonFiniteAccumulation)?;

    let mut rejected = RejectedSubsetCounts::default();
    let mut best: Option<CandidatePlan> = None;

    for mask in 1_u32..subset_limit {
        let selected: Vec<usize> = usable_indices
            .iter()
            .enumerate()
            .filter_map(|(bit, option_index)| {
                ((mask & (1_u32 << bit)) != 0).then_some(*option_index)
            })
            .collect();

        if selected.len() > config.maximum_selected_actions {
            rejected.over_action_limit += 1;
            continue;
        }

        let mut cost = 0_u32;
        let mut latency = 0.0_f64;
        let mut capabilities = BTreeSet::new();
        let mut observer_requalification_required = false;
        let mut action_ids = Vec::with_capacity(selected.len());

        for index in &selected {
            let option = &options[*index];
            cost = cost
                .checked_add(option.cost_units)
                .ok_or(ScheduleError::NonFiniteAccumulation)?;
            latency += option.latency_seconds;
            capabilities.extend(option.capabilities.iter().copied());
            action_ids.push(option.action_id.clone());
            observer_requalification_required |=
                option.observer_impact != ObserverImpact::AssessedNonPerturbing;
        }
        if observer_requalification_required
            && !capabilities.contains(&DiagnosticCapability::ObserverRequalification)
        {
            rejected.observer_requalification_missing += 1;
            continue;
        }
        if !required.is_subset(&capabilities) {
            rejected.missing_capability_coverage += 1;
            continue;
        }
        if cost > config.measurement_budget_units {
            rejected.over_budget += 1;
            continue;
        }
        if !latency.is_finite() {
            return Err(ScheduleError::NonFiniteAccumulation);
        }
        if latency > config.maximum_total_latency_seconds {
            rejected.over_latency += 1;
            continue;
        }

        action_ids.sort();
        let candidate = CandidatePlan {
            option_indices: selected,
            action_ids,
            cost,
            latency,
            observer_requalification_required,
        };
        let replace = best.as_ref().is_none_or(|current| candidate_better(&candidate, current));
        if replace {
            best = Some(candidate);
        }
    }

    Ok((best, rejected))
}

fn candidate_better(candidate: &CandidatePlan, current: &CandidatePlan) -> bool {
    candidate
        .cost
        .cmp(&current.cost)
        .then_with(|| candidate.latency.total_cmp(&current.latency))
        .then_with(|| candidate.action_ids.cmp(&current.action_ids))
        .is_lt()
}

fn determine_abstention(
    required: &BTreeSet<DiagnosticCapability>,
    options: &[DiagnosticOption],
    config: &ScheduleConfig,
    rejected: &RejectedSubsetCounts,
) -> AbstentionReason {
    let mut all_capabilities = BTreeSet::new();
    let mut all_available_capabilities = BTreeSet::new();
    let mut any_observer_requalification = false;
    for option in options {
        all_capabilities.extend(option.capabilities.iter().copied());
        if option.available {
            all_available_capabilities.extend(option.capabilities.iter().copied());
            any_observer_requalification |= option
                .capabilities
                .contains(&DiagnosticCapability::ObserverRequalification);
        }
    }
    if !required.is_subset(&all_available_capabilities) {
        return AbstentionReason::NoAvailableActionCoversRequiredCapabilities;
    }
    if rejected.observer_requalification_missing > 0
        && !any_observer_requalification
    {
        return AbstentionReason::ObserverRequalificationUnavailable;
    }
    if rejected.over_action_limit > 0
        && rejected.missing_capability_coverage == 0
        && rejected.over_budget == 0
        && rejected.over_latency == 0
    {
        return AbstentionReason::SelectedActionLimitExceeded;
    }
    if rejected.over_latency > 0
        && rejected.over_budget == 0
    {
        return AbstentionReason::LatencyLimitExceeded;
    }
    if rejected.over_budget > 0 || config.measurement_budget_units == 0 {
        return AbstentionReason::MeasurementBudgetInsufficient;
    }
    if all_capabilities.is_empty() {
        return AbstentionReason::NoAvailableActionCoversRequiredCapabilities;
    }
    AbstentionReason::NoAvailableActionCoversRequiredCapabilities
}

#[cfg(test)]
mod tests {
    use super::*;

    fn regions() -> BTreeSet<FlowRegion> {
        BTreeSet::from([
            FlowRegion::Bulk,
            FlowRegion::NearWall,
            FlowRegion::ShearLayer,
            FlowRegion::Wake,
        ])
    }

    fn config() -> ScheduleConfig {
        ScheduleConfig {
            measurement_budget_units: 12,
            maximum_total_latency_seconds: 30.0,
            maximum_selected_actions: 6,
            thresholds: TriggerThresholds {
                maximum_uncertainty_ratio: 1.0,
                maximum_out_of_domain_score: 0.8,
                maximum_model_disagreement_sigma: 3.0,
                maximum_conservation_residual_sigma: 3.0,
                maximum_high_fidelity_age_seconds: 120.0,
                required_regions: regions(),
            },
        }
    }

    fn snapshot() -> SentinelSnapshot {
        SentinelSnapshot {
            acquisition_id: "acq-001".to_owned(),
            dependence_cluster_id: "cluster-01".to_owned(),
            protocol_fingerprint: "protocol-01".to_owned(),
            observer_state_id: "observer-01".to_owned(),
            observer_state_qualified: true,
            reference_evidence_qualified: true,
            diagnostic_quality_qualified: true,
            uncertainty_ratio: Some(0.4),
            out_of_domain_score: Some(0.2),
            model_disagreement_sigma: Some(0.5),
            conservation_residual_sigma: Some(0.7),
            seconds_since_last_high_fidelity: Some(30.0),
            observed_regions: regions(),
        }
    }

    fn action(
        id: &str,
        kind: DiagnosticKind,
        cost: u32,
        latency: f64,
        capabilities: &[DiagnosticCapability],
        available: bool,
        observer_impact: ObserverImpact,
    ) -> DiagnosticOption {
        DiagnosticOption {
            action_id: id.to_owned(),
            kind,
            cost_units: cost,
            latency_seconds: latency,
            capabilities: capabilities.iter().copied().collect(),
            available,
            observer_impact,
            evidence_receipt_required: true,
        }
    }

    fn catalog() -> Vec<DiagnosticOption> {
        vec![
            action(
                "piv-hires",
                DiagnosticKind::HighResolutionPiv,
                2,
                5.0,
                &[
                    DiagnosticCapability::HighResolutionPiv,
                    DiagnosticCapability::SpatialRegionCoverage,
                ],
                true,
                ObserverImpact::AssessedNonPerturbing,
            ),
            action(
                "lpt-3d",
                DiagnosticKind::LptShakeTheBox,
                4,
                10.0,
                &[
                    DiagnosticCapability::VolumetricLpt,
                    DiagnosticCapability::IndependentVelocityReference,
                    DiagnosticCapability::SpatialRegionCoverage,
                ],
                true,
                ObserverImpact::PotentiallyPerturbing,
            ),
            action(
                "velocity-reference",
                DiagnosticKind::IndependentVelocityReference,
                3,
                6.0,
                &[DiagnosticCapability::IndependentVelocityReference],
                true,
                ObserverImpact::AssessedNonPerturbing,
            ),
            action(
                "force-balance",
                DiagnosticKind::DirectForceBalance,
                3,
                4.0,
                &[DiagnosticCapability::DirectForceMeasurement],
                true,
                ObserverImpact::AssessedNonPerturbing,
            ),
            action(
                "control-volume",
                DiagnosticKind::ControlVolumeReconstruction,
                2,
                6.0,
                &[
                    DiagnosticCapability::CompleteControlVolume,
                    DiagnosticCapability::ConservationAudit,
                ],
                true,
                ObserverImpact::AssessedNonPerturbing,
            ),
            action(
                "power-meter",
                DiagnosticKind::ElectricalPowerMeasurement,
                1,
                2.0,
                &[DiagnosticCapability::ElectricalPowerAccounting],
                true,
                ObserverImpact::AssessedNonPerturbing,
            ),
            action(
                "reference-qual",
                DiagnosticKind::ReferenceRequalification,
                2,
                4.0,
                &[DiagnosticCapability::ReferenceQualification],
                true,
                ObserverImpact::AssessedNonPerturbing,
            ),
            action(
                "observer-washout",
                DiagnosticKind::ObserverWashoutAndRequalification,
                1,
                3.0,
                &[DiagnosticCapability::ObserverRequalification],
                true,
                ObserverImpact::AssessedNonPerturbing,
            ),
            action(
                "sentinel-repeat",
                DiagnosticKind::SentinelPiv,
                1,
                1.0,
                &[DiagnosticCapability::SentinelReacquisition],
                true,
                ObserverImpact::AssessedNonPerturbing,
            ),
        ]
    }

    #[test]
    fn benign_sentinel_window_continues_without_unnecessary_high_fidelity() {
        let report = schedule_diagnostics(
            EvidenceTarget::SentinelMonitoring,
            &snapshot(),
            &config(),
            &catalog(),
        )
        .unwrap();
        assert_eq!(report.disposition, ScheduleDisposition::ContinueSentinel);
        assert!(report.selected_actions.is_empty());
        assert!(report.trigger_reasons.is_empty());
    }

    #[test]
    fn high_uncertainty_triggers_high_resolution_piv() {
        let mut sample = snapshot();
        sample.uncertainty_ratio = Some(1.2);
        let report = schedule_diagnostics(
            EvidenceTarget::SentinelMonitoring,
            &sample,
            &config(),
            &catalog(),
        )
        .unwrap();
        assert_eq!(report.disposition, ScheduleDisposition::AcquireDiagnostics);
        assert!(
            report
                .selected_actions
                .iter()
                .any(|action| action.kind == DiagnosticKind::HighResolutionPiv)
        );
        assert!(
            report
                .trigger_reasons
                .contains(&TriggerReason::LocalUncertaintyExceeded)
        );
    }

    #[test]
    fn out_of_domain_trigger_selects_volumetric_lpt_and_observer_requalification() {
        let mut sample = snapshot();
        sample.out_of_domain_score = Some(0.95);
        let report = schedule_diagnostics(
            EvidenceTarget::ThreeDimensionalVelocity,
            &sample,
            &config(),
            &catalog(),
        )
        .unwrap();
        assert_eq!(report.disposition, ScheduleDisposition::AcquireDiagnostics);
        assert!(report.observer_requalification_required);
        assert!(report
            .selected_actions
            .iter()
            .any(|action| action.kind == DiagnosticKind::LptShakeTheBox));
        assert!(report
            .selected_actions
            .iter()
            .any(|action| action.kind == DiagnosticKind::ObserverWashoutAndRequalification));
    }

    #[test]
    fn unqualified_reference_triggers_reference_requalification() {
        let mut sample = snapshot();
        sample.reference_evidence_qualified = false;
        let report = schedule_diagnostics(
            EvidenceTarget::FieldCovariance,
            &sample,
            &config(),
            &catalog(),
        )
        .unwrap();
        assert_eq!(report.disposition, ScheduleDisposition::AcquireDiagnostics);
        assert!(report
            .selected_actions
            .iter()
            .any(|action| action.kind == DiagnosticKind::ReferenceRequalification));
    }

    #[test]
    fn unqualified_observer_state_requires_requalification() {
        let mut sample = snapshot();
        sample.observer_state_qualified = false;
        let report = schedule_diagnostics(
            EvidenceTarget::SentinelMonitoring,
            &sample,
            &config(),
            &catalog(),
        )
        .unwrap();
        assert_eq!(report.disposition, ScheduleDisposition::AcquireDiagnostics);
        assert!(report
            .selected_actions
            .iter()
            .any(|action| action.kind == DiagnosticKind::ObserverWashoutAndRequalification));
    }

    #[test]
    fn thrust_target_requires_force_velocity_control_volume_and_power_evidence() {
        let report = schedule_diagnostics(
            EvidenceTarget::ThrustClaim,
            &snapshot(),
            &config(),
            &catalog(),
        )
        .unwrap();
        assert_eq!(report.disposition, ScheduleDisposition::AcquireDiagnostics);
        let capabilities: BTreeSet<_> = report
            .selected_actions
            .iter()
            .flat_map(|action| action.capabilities.iter().copied())
            .collect();
        for required in [
            DiagnosticCapability::DirectForceMeasurement,
            DiagnosticCapability::CompleteControlVolume,
            DiagnosticCapability::IndependentVelocityReference,
            DiagnosticCapability::ElectricalPowerAccounting,
        ] {
            assert!(capabilities.contains(&required));
        }
    }

    #[test]
    fn force_claim_abstains_when_direct_force_action_is_unavailable() {
        let mut options = catalog();
        for option in &mut options {
            if option.kind == DiagnosticKind::DirectForceBalance {
                option.available = false;
            }
        }
        let report = schedule_diagnostics(
            EvidenceTarget::IntegratedForce,
            &snapshot(),
            &config(),
            &options,
        )
        .unwrap();
        assert_eq!(report.disposition, ScheduleDisposition::Abstain);
        assert_eq!(
            report.abstention_reason,
            Some(AbstentionReason::NoAvailableActionCoversRequiredCapabilities)
        );
        assert!(report.selected_actions.is_empty());
    }

    #[test]
    fn budget_exhaustion_abstains_without_downgrading_evidence_requirements() {
        let mut limited = config();
        limited.measurement_budget_units = 2;
        let report = schedule_diagnostics(
            EvidenceTarget::ThrustClaim,
            &snapshot(),
            &limited,
            &catalog(),
        )
        .unwrap();
        assert_eq!(report.disposition, ScheduleDisposition::Abstain);
        assert!(report.abstention_reason.is_some());
        assert!(report.selected_actions.is_empty());
    }

    #[test]
    fn unavailable_independent_reference_does_not_get_replaced_by_piv() {
        let mut options = catalog();
        for option in &mut options {
            if option.kind == DiagnosticKind::IndependentVelocityReference
                || option.kind == DiagnosticKind::LptShakeTheBox
            {
                option.available = false;
            }
        }
        let report = schedule_diagnostics(
            EvidenceTarget::ThrustClaim,
            &snapshot(),
            &config(),
            &options,
        )
        .unwrap();
        assert_eq!(report.disposition, ScheduleDisposition::Abstain);
    }

    #[test]
    fn action_with_unknown_observer_impact_requires_requalification() {
        let mut options = catalog();
        let highres = options
            .iter_mut()
            .find(|option| option.kind == DiagnosticKind::HighResolutionPiv)
            .unwrap();
        highres.observer_impact = ObserverImpact::Unknown;

        let mut sample = snapshot();
        sample.uncertainty_ratio = Some(1.2);
        let report = schedule_diagnostics(
            EvidenceTarget::SentinelMonitoring,
            &sample,
            &config(),
            &options,
        )
        .unwrap();
        assert_eq!(report.disposition, ScheduleDisposition::AcquireDiagnostics);
        assert!(report.observer_requalification_required);
        assert!(report
            .selected_actions
            .iter()
            .any(|action| action.kind == DiagnosticKind::ObserverWashoutAndRequalification));
    }

    #[test]
    fn missing_sentinel_metric_triggers_reacquisition() {
        let mut sample = snapshot();
        sample.out_of_domain_score = None;
        let report = schedule_diagnostics(
            EvidenceTarget::SentinelMonitoring,
            &sample,
            &config(),
            &catalog(),
        )
        .unwrap();
        assert_eq!(report.disposition, ScheduleDisposition::AcquireDiagnostics);
        assert!(report
            .selected_actions
            .iter()
            .any(|action| action.kind == DiagnosticKind::SentinelPiv));
        assert!(report.trigger_reasons.iter().any(|reason| matches!(
            reason,
            TriggerReason::SentinelTelemetryIncomplete("out_of_domain_score")
        )));
    }

    #[test]
    fn stale_high_fidelity_data_triggers_reacquisition() {
        let mut sample = snapshot();
        sample.seconds_since_last_high_fidelity = Some(600.0);
        let report = schedule_diagnostics(
            EvidenceTarget::SentinelMonitoring,
            &sample,
            &config(),
            &catalog(),
        )
        .unwrap();
        assert!(report
            .trigger_reasons
            .contains(&TriggerReason::HighFidelityAcquisitionStale));
        assert!(report.selected_actions.iter().any(|action| {
            action.kind == DiagnosticKind::HighResolutionPiv
        }));
    }

    #[test]
    fn cheapest_capability_cover_is_selected_deterministically() {
        let mut options = catalog();
        options.push(action(
            "piv-hires-expensive",
            DiagnosticKind::HighResolutionPiv,
            5,
            2.0,
            &[DiagnosticCapability::HighResolutionPiv],
            true,
            ObserverImpact::AssessedNonPerturbing,
        ));
        let mut sample = snapshot();
        sample.uncertainty_ratio = Some(1.5);
        let report = schedule_diagnostics(
            EvidenceTarget::SentinelMonitoring,
            &sample,
            &config(),
            &options,
        )
        .unwrap();
        assert_eq!(report.total_cost_units, 2);
        assert_eq!(report.selected_actions[0].action_id, "piv-hires");
    }

    #[test]
    fn missing_region_triggers_spatial_coverage_action() {
        let mut sample = snapshot();
        sample.observed_regions.remove(&FlowRegion::NearWall);
        let report = schedule_diagnostics(
            EvidenceTarget::SentinelMonitoring,
            &sample,
            &config(),
            &catalog(),
        )
        .unwrap();
        assert!(report
            .trigger_reasons
            .contains(&TriggerReason::SpatialCoverageMissing(FlowRegion::NearWall)));
        assert!(report.selected_actions.iter().any(|action| {
            action
                .capabilities
                .contains(&DiagnosticCapability::SpatialRegionCoverage)
        }));
    }

    #[test]
    fn duplicate_action_ids_are_rejected() {
        let mut options = catalog();
        options.push(options[0].clone());
        assert!(matches!(
            schedule_diagnostics(
                EvidenceTarget::SentinelMonitoring,
                &snapshot(),
                &config(),
                &options
            ),
            Err(ScheduleError::DuplicateActionId(_))
        ));
    }

    #[test]
    fn diagnostic_without_replayable_receipt_is_rejected() {
        let mut options = catalog();
        options[0].evidence_receipt_required = false;
        assert!(matches!(
            schedule_diagnostics(
                EvidenceTarget::SentinelMonitoring,
                &snapshot(),
                &config(),
                &options
            ),
            Err(ScheduleError::EvidenceReceiptNotRequired { .. })
        ));
    }

    #[test]
    fn nonfinite_sentinel_metric_is_rejected() {
        let mut sample = snapshot();
        sample.uncertainty_ratio = Some(f64::NAN);
        assert!(matches!(
            schedule_diagnostics(
                EvidenceTarget::SentinelMonitoring,
                &sample,
                &config(),
                &catalog()
            ),
            Err(ScheduleError::InvalidSnapshotMetric { .. })
        ));
    }

    #[test]
    fn options_above_exhaustive_search_limit_are_rejected() {
        let mut options = catalog();
        while options.len() <= MAX_DIAGNOSTIC_OPTIONS {
            let mut extra = options[0].clone();
            extra.action_id = format!("extra-{}", options.len());
            options.push(extra);
        }
        assert!(matches!(
            schedule_diagnostics(
                EvidenceTarget::SentinelMonitoring,
                &snapshot(),
                &config(),
                &options
            ),
            Err(ScheduleError::TooManyDiagnosticOptions { .. })
        ));
    }
}
