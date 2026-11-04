// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Conservative firm-capacity qualification over repeated grid-response trials.
//!
//! The event-level verifier owns response latency, sustained delivery,
//! under-delivery, overshoot and ramp. This module aggregates *all scheduled
//! opportunities*, including explicit unavailable events, against one frozen
//! product/site/baseline/configuration/condition stratum.
//!
//! Reliability is bounded with a one-sided Hoeffding lower confidence bound:
//! max(0, p_hat - sqrt(ln(1/delta)/(2n))). The bound assumes independent
//! Bernoulli event trials. The caller pins an independence-basis identity;
//! that identity is a declaration, not proof of independence.
//!
//! This module is not an ISO/TSO capacity-accreditation engine, does not prove
//! future representativeness, and does not infer a higher capacity than the
//! single committed reduction level tested.

use core::fmt;
use std::collections::BTreeSet;

use serde::Serialize;

use crate::GridResponseReport;

/// Frozen identity and tested commitment level for one firm-flexibility stratum.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GridFirmCapacityIdentity {
    portfolio_id: String,
    opportunity_set_digest: String,
    service_product_id: String,
    measurement_boundary_id: String,
    baseline_method_id: String,
    response_configuration_digest: String,
    condition_stratum_id: String,
    independence_basis_id: String,
    committed_reduction_kw: f64,
}

impl GridFirmCapacityIdentity {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        portfolio_id: impl Into<String>,
        opportunity_set_digest: impl Into<String>,
        service_product_id: impl Into<String>,
        measurement_boundary_id: impl Into<String>,
        baseline_method_id: impl Into<String>,
        response_configuration_digest: impl Into<String>,
        condition_stratum_id: impl Into<String>,
        independence_basis_id: impl Into<String>,
        committed_reduction_kw: f64,
    ) -> Result<Self, GridFirmCapacityError> {
        let identity = Self {
            portfolio_id: portfolio_id.into(),
            opportunity_set_digest: opportunity_set_digest.into(),
            service_product_id: service_product_id.into(),
            measurement_boundary_id: measurement_boundary_id.into(),
            baseline_method_id: baseline_method_id.into(),
            response_configuration_digest: response_configuration_digest.into(),
            condition_stratum_id: condition_stratum_id.into(),
            independence_basis_id: independence_basis_id.into(),
            committed_reduction_kw,
        };
        for (name, value) in [
            ("portfolio ID", identity.portfolio_id.as_str()),
            ("opportunity-set digest", identity.opportunity_set_digest.as_str()),
            ("service product ID", identity.service_product_id.as_str()),
            ("measurement boundary ID", identity.measurement_boundary_id.as_str()),
            ("baseline method ID", identity.baseline_method_id.as_str()),
            ("response configuration digest", identity.response_configuration_digest.as_str()),
            ("condition stratum ID", identity.condition_stratum_id.as_str()),
            ("independence-basis ID", identity.independence_basis_id.as_str()),
        ] {
            require_id(name, value)?;
        }
        if !identity.committed_reduction_kw.is_finite()
            || identity.committed_reduction_kw <= 0.0
        {
            return Err(GridFirmCapacityError::InvalidQuantity {
                name: "committed reduction",
                value: identity.committed_reduction_kw,
            });
        }
        Ok(identity)
    }

    pub fn portfolio_id(&self) -> &str { &self.portfolio_id }
    pub fn opportunity_set_digest(&self) -> &str { &self.opportunity_set_digest }
    pub fn service_product_id(&self) -> &str { &self.service_product_id }
    pub fn measurement_boundary_id(&self) -> &str { &self.measurement_boundary_id }
    pub fn baseline_method_id(&self) -> &str { &self.baseline_method_id }
    pub fn response_configuration_digest(&self) -> &str {
        &self.response_configuration_digest
    }
    pub fn condition_stratum_id(&self) -> &str { &self.condition_stratum_id }
    pub fn independence_basis_id(&self) -> &str { &self.independence_basis_id }
    pub const fn committed_reduction_kw(&self) -> f64 { self.committed_reduction_kw }
}

/// Confidence and event-count requirements for the tested commitment level.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GridFirmCapacityContract {
    identity: GridFirmCapacityIdentity,
    /// Exact opportunity manifest. Its digest is pinned in the identity and
    /// must be computed independently by the caller from this canonical list.
    scheduled_event_ids: Vec<String>,
    minimum_trial_count: usize,
    minimum_success_probability: f64,
    confidence_failure_probability: f64,
}

impl GridFirmCapacityContract {
    pub fn new(
        identity: GridFirmCapacityIdentity,
        mut scheduled_event_ids: Vec<String>,
        minimum_trial_count: usize,
        minimum_success_probability: f64,
        confidence_failure_probability: f64,
    ) -> Result<Self, GridFirmCapacityError> {
        if minimum_trial_count == 0 {
            return Err(GridFirmCapacityError::InvalidMinimumTrialCount);
        }
        if scheduled_event_ids.is_empty() {
            return Err(GridFirmCapacityError::EmptyScheduledOpportunitySet);
        }
        scheduled_event_ids.sort();
        for event_id in &scheduled_event_ids {
            require_id("scheduled event ID", event_id)?;
        }
        if scheduled_event_ids.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(GridFirmCapacityError::DuplicateScheduledEventId);
        }
        if scheduled_event_ids.len() < minimum_trial_count {
            return Err(GridFirmCapacityError::ScheduledOpportunityCountBelowMinimum {
                observed: scheduled_event_ids.len(),
                required: minimum_trial_count,
            });
        }
        for (name, value) in [
            ("minimum success probability", minimum_success_probability),
            ("confidence failure probability", confidence_failure_probability),
        ] {
            if !value.is_finite() || !(0.0..1.0).contains(&value) {
                return Err(GridFirmCapacityError::InvalidProbability { name, value });
            }
        }
        Ok(Self {
            identity,
            scheduled_event_ids,
            minimum_trial_count,
            minimum_success_probability,
            confidence_failure_probability,
        })
    }

    pub fn identity(&self) -> &GridFirmCapacityIdentity { &self.identity }
    pub fn scheduled_event_ids(&self) -> &[String] { &self.scheduled_event_ids }
    pub const fn minimum_trial_count(&self) -> usize { self.minimum_trial_count }
    pub const fn minimum_success_probability(&self) -> f64 {
        self.minimum_success_probability
    }
    pub const fn confidence_failure_probability(&self) -> f64 {
        self.confidence_failure_probability
    }
}

/// One scheduled opportunity. Unavailable opportunities are explicit failures.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GridFirmCapacityTrial {
    event_id: String,
    condition_stratum_id: String,
    outcome: GridFirmCapacityTrialOutcome,
}

/// Evaluated response or explicit no-response/unavailable record.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum GridFirmCapacityTrialOutcome {
    Evaluated(GridResponseReport),
    Unavailable {
        reason: String,
        evidence_id: String,
    },
}

impl GridFirmCapacityTrial {
    pub fn evaluated(
        report: GridResponseReport,
        condition_stratum_id: impl Into<String>,
    ) -> Result<Self, GridFirmCapacityError> {
        let event_id = report.identity.event_id().to_string();
        let condition_stratum_id = condition_stratum_id.into();
        require_id("event ID", &event_id)?;
        require_id("condition stratum ID", &condition_stratum_id)?;
        Ok(Self {
            event_id,
            condition_stratum_id,
            outcome: GridFirmCapacityTrialOutcome::Evaluated(report),
        })
    }

    pub fn unavailable(
        event_id: impl Into<String>,
        condition_stratum_id: impl Into<String>,
        reason: impl Into<String>,
        evidence_id: impl Into<String>,
    ) -> Result<Self, GridFirmCapacityError> {
        let event_id = event_id.into();
        let condition_stratum_id = condition_stratum_id.into();
        let reason = reason.into();
        let evidence_id = evidence_id.into();
        require_id("event ID", &event_id)?;
        require_id("condition stratum ID", &condition_stratum_id)?;
        require_id("unavailability reason", &reason)?;
        require_id("unavailability evidence ID", &evidence_id)?;
        Ok(Self {
            event_id,
            condition_stratum_id,
            outcome: GridFirmCapacityTrialOutcome::Unavailable {
                reason,
                evidence_id,
            },
        })
    }

    pub fn event_id(&self) -> &str { &self.event_id }
}

/// One trial in the accreditation receipt.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GridFirmCapacityTrialReceipt {
    pub event_id: String,
    pub condition_stratum_id: String,
    pub available: bool,
    pub successful: bool,
    pub response_latency_ns: Option<u64>,
    pub delivered_energy_fraction: Option<f64>,
    pub underdelivery_energy_fraction: Option<f64>,
    pub overshoot_energy_fraction: Option<f64>,
    pub response_sample_evidence_ids: Vec<String>,
    pub unavailable_reason: Option<String>,
    pub availability_evidence_id: Option<String>,
}

/// Conservative repeated-event firm-capacity report.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GridFirmCapacityReport {
    pub identity: GridFirmCapacityIdentity,
    /// Complete manifest compared against the supplied trial receipts.
    pub scheduled_event_ids: Vec<String>,
    pub metric_convention_id: String,
    pub confidence_method_id: String,
    pub confidence_assumption: String,
    pub claim_ceiling: String,
    pub trial_count: usize,
    pub successful_trial_count: usize,
    pub failed_trial_count: usize,
    pub unavailable_trial_count: usize,
    pub empirical_success_probability: f64,
    pub confidence_failure_probability: f64,
    pub hoeffding_radius: f64,
    pub lower_confidence_success_probability: f64,
    pub required_success_probability: f64,
    pub minimum_trial_count: usize,
    pub event_response_reliability_qualified: bool,
    pub qualified_committed_reduction_kw: Option<f64>,
    pub trial_receipts: Vec<GridFirmCapacityTrialReceipt>,
}

/// Invalid firm-capacity contracts, event sets or confidence arithmetic.
#[derive(Debug, Clone, PartialEq)]
pub enum GridFirmCapacityError {
    MissingIdentity { name: &'static str },
    InvalidQuantity { name: &'static str, value: f64 },
    InvalidProbability { name: &'static str, value: f64 },
    InvalidMinimumTrialCount,
    EmptyScheduledOpportunitySet,
    DuplicateScheduledEventId,
    ScheduledOpportunityCountBelowMinimum { observed: usize, required: usize },
    ScheduledOpportunityCountMismatch { observed: usize, expected: usize },
    UnexpectedScheduledOpportunity(String),
    EmptyTrialSet,
    InsufficientTrials { observed: usize, required: usize },
    DuplicateEventId(String),
    ConditionStratumMismatch {
        event_id: String,
        expected: String,
        observed: String,
    },
    ServiceProductMismatch { event_id: String },
    MeasurementBoundaryMismatch { event_id: String },
    BaselineMethodMismatch { event_id: String },
    ResponseConfigurationMismatch { event_id: String },
    CommittedCapacityMismatch {
        event_id: String,
        expected_kw: f64,
        observed_kw: f64,
    },
    ResponseReportEventIdMismatch { event_id: String },
    InconsistentResponseReport { event_id: String },
    NumericalOverflow(&'static str),
}

impl fmt::Display for GridFirmCapacityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingIdentity { name } => write!(f, "{name} must not be empty"),
            Self::InvalidQuantity { name, value } => {
                write!(f, "{name} must be finite and positive; got {value}")
            }
            Self::InvalidProbability { name, value } => {
                write!(f, "{name} must be in (0, 1); got {value}")
            }
            Self::InvalidMinimumTrialCount => {
                write!(f, "minimum event trial count must be positive")
            }
            Self::EmptyScheduledOpportunitySet => {
                write!(f, "scheduled opportunity set must not be empty")
            }
            Self::DuplicateScheduledEventId => {
                write!(f, "scheduled opportunity IDs must be unique")
            }
            Self::ScheduledOpportunityCountBelowMinimum { observed, required } => write!(
                f, "scheduled opportunity set has {observed} events; {required} minimum trials required"
            ),
            Self::ScheduledOpportunityCountMismatch { observed, expected } => write!(
                f, "trial set has {observed} events but opportunity manifest pins {expected}"
            ),
            Self::UnexpectedScheduledOpportunity(id) => {
                write!(f, "trial event {id} is not in the frozen opportunity manifest")
            }
            Self::EmptyTrialSet => write!(f, "firm-capacity evaluation requires event trials"),
            Self::InsufficientTrials { observed, required } => write!(
                f, "firm-capacity evaluation has {observed} trials; {required} required"
            ),
            Self::DuplicateEventId(id) => write!(f, "duplicate grid-response event ID {id}"),
            Self::ConditionStratumMismatch { event_id, expected, observed } => write!(
                f, "event {event_id} belongs to stratum {observed}, expected {expected}"
            ),
            Self::ServiceProductMismatch { event_id } => {
                write!(f, "event {event_id} uses a different service product")
            }
            Self::MeasurementBoundaryMismatch { event_id } => {
                write!(f, "event {event_id} uses a different PCC measurement boundary")
            }
            Self::BaselineMethodMismatch { event_id } => {
                write!(f, "event {event_id} uses a different baseline method")
            }
            Self::ResponseConfigurationMismatch { event_id } => {
                write!(f, "event {event_id} uses a different response configuration")
            }
            Self::CommittedCapacityMismatch { event_id, expected_kw, observed_kw } => write!(
                f, "event {event_id} tested {observed_kw} kW, expected {expected_kw} kW"
            ),
            Self::ResponseReportEventIdMismatch { event_id } => {
                write!(f, "response report event identity mismatch for {event_id}")
            }
            Self::InconsistentResponseReport { event_id } => {
                write!(f, "response report gates and pass flag disagree for event {event_id}")
            }
            Self::NumericalOverflow(operation) => {
                write!(f, "non-finite arithmetic during {operation}")
            }
        }
    }
}

impl std::error::Error for GridFirmCapacityError {}

/// Complete, frozen set of scheduled grid-service opportunities.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GridFirmCapacityEvaluation {
    contract: GridFirmCapacityContract,
    trials: Vec<GridFirmCapacityTrial>,
}

impl GridFirmCapacityEvaluation {
    pub fn new(
        contract: GridFirmCapacityContract,
        mut trials: Vec<GridFirmCapacityTrial>,
    ) -> Result<Self, GridFirmCapacityError> {
        if trials.is_empty() {
            return Err(GridFirmCapacityError::EmptyTrialSet);
        }
        if trials.len() < contract.minimum_trial_count {
            return Err(GridFirmCapacityError::InsufficientTrials {
                observed: trials.len(),
                required: contract.minimum_trial_count,
            });
        }
        if trials.len() != contract.scheduled_event_ids.len() {
            return Err(GridFirmCapacityError::ScheduledOpportunityCountMismatch {
                observed: trials.len(),
                expected: contract.scheduled_event_ids.len(),
            });
        }
        trials.sort_by(|left, right| left.event_id.cmp(&right.event_id));
        let mut event_ids = BTreeSet::new();
        let scheduled_ids = contract.scheduled_event_ids.iter().cloned().collect::<BTreeSet<_>>();
        for trial in &trials {
            if !event_ids.insert(trial.event_id.clone()) {
                return Err(GridFirmCapacityError::DuplicateEventId(
                    trial.event_id.clone(),
                ));
            }
            if !scheduled_ids.contains(&trial.event_id) {
                return Err(GridFirmCapacityError::UnexpectedScheduledOpportunity(
                    trial.event_id.clone(),
                ));
            }
            if trial.condition_stratum_id != contract.identity.condition_stratum_id {
                return Err(GridFirmCapacityError::ConditionStratumMismatch {
                    event_id: trial.event_id.clone(),
                    expected: contract.identity.condition_stratum_id.clone(),
                    observed: trial.condition_stratum_id.clone(),
                });
            }
            if let GridFirmCapacityTrialOutcome::Evaluated(report) = &trial.outcome {
                validate_response_report(&contract.identity, trial, report)?;
            }
        }
        Ok(Self { contract, trials })
    }

    pub fn contract(&self) -> &GridFirmCapacityContract { &self.contract }
    pub fn trials(&self) -> &[GridFirmCapacityTrial] { &self.trials }
}

/// Accredit the one committed capacity if its conservative success bound passes.
pub fn evaluate_grid_firm_capacity(
    evaluation: &GridFirmCapacityEvaluation,
) -> Result<GridFirmCapacityReport, GridFirmCapacityError> {
    let trial_count = evaluation.trials.len();
    if trial_count == 0 {
        return Err(GridFirmCapacityError::EmptyTrialSet);
    }

    let mut successful_trial_count = 0_usize;
    let mut unavailable_trial_count = 0_usize;
    let mut trial_receipts = Vec::with_capacity(trial_count);

    for trial in &evaluation.trials {
        match &trial.outcome {
            GridFirmCapacityTrialOutcome::Unavailable { reason, evidence_id } => {
                unavailable_trial_count += 1;
                trial_receipts.push(GridFirmCapacityTrialReceipt {
                    event_id: trial.event_id.clone(),
                    condition_stratum_id: trial.condition_stratum_id.clone(),
                    available: false,
                    successful: false,
                    response_latency_ns: None,
                    delivered_energy_fraction: None,
                    underdelivery_energy_fraction: None,
                    overshoot_energy_fraction: None,
                    response_sample_evidence_ids: Vec::new(),
                    unavailable_reason: Some(reason.clone()),
                    availability_evidence_id: Some(evidence_id.clone()),
                });
            }
            GridFirmCapacityTrialOutcome::Evaluated(report) => {
                let gate_success = response_gates_pass(report);
                if gate_success {
                    successful_trial_count += 1;
                }
                trial_receipts.push(GridFirmCapacityTrialReceipt {
                    event_id: trial.event_id.clone(),
                    condition_stratum_id: trial.condition_stratum_id.clone(),
                    available: true,
                    successful: gate_success,
                    response_latency_ns: report.response_latency_ns,
                    delivered_energy_fraction: Some(report.delivered_energy_fraction),
                    underdelivery_energy_fraction: Some(report.underdelivery_energy_fraction),
                    overshoot_energy_fraction: Some(report.overshoot_energy_fraction),
                    response_sample_evidence_ids: report.sample_receipts
                        .iter()
                        .flat_map(|sample| [
                            sample.baseline_evidence_id.clone(),
                            sample.response_evidence_id.clone(),
                        ])
                        .collect(),
                    unavailable_reason: None,
                    availability_evidence_id: None,
                });
            }
        }
    }

    let failed_trial_count = trial_count - successful_trial_count;
    let empirical_success_probability =
        successful_trial_count as f64 / trial_count as f64;
    let delta = evaluation.contract.confidence_failure_probability;
    let hoeffding_radius =
        ((-delta.ln()) / (2.0 * trial_count as f64)).sqrt();
    let lower_confidence_success_probability =
        (empirical_success_probability - hoeffding_radius).max(0.0);
    if !empirical_success_probability.is_finite()
        || !hoeffding_radius.is_finite()
        || !lower_confidence_success_probability.is_finite()
    {
        return Err(GridFirmCapacityError::NumericalOverflow(
            "Hoeffding lower confidence bound",
        ));
    }

    let event_response_reliability_qualified = lower_confidence_success_probability
        >= evaluation.contract.minimum_success_probability;
    let qualified_committed_reduction_kw = event_response_reliability_qualified
        .then_some(evaluation.contract.identity.committed_reduction_kw);

    Ok(GridFirmCapacityReport {
        identity: evaluation.contract.identity.clone(),
        scheduled_event_ids: evaluation.contract.scheduled_event_ids.clone(),
        metric_convention_id: "SYMTROPY-GRID-RESPONSE-RELIABILITY-HOEFFDING-INDEPENDENT-TRIALS-V0.1".to_string(),
        confidence_method_id: "ONE_SIDED_HOEFFDING_BERNOUlli_LOWER_BOUND".to_string(),
        confidence_assumption: "INDEPENDENT_BERNOULLI_EVENT_TRIALS_DECLARED_BY_CALLER_NOT_PROVEN".to_string(),
        claim_ceiling: "EVENT_RESPONSE_RELIABILITY_LOWER_BOUND_WITHIN_ONE_FROZEN_CONDITION_STRATUM_NOT_SYSTEM_RESOURCE_ADEQUACY_OR_ISO_TSO_ACCREDITATION".to_string(),
        trial_count,
        successful_trial_count,
        failed_trial_count,
        unavailable_trial_count,
        empirical_success_probability,
        confidence_failure_probability: delta,
        hoeffding_radius,
        lower_confidence_success_probability,
        required_success_probability: evaluation.contract.minimum_success_probability,
        minimum_trial_count: evaluation.contract.minimum_trial_count,
        event_response_reliability_qualified,
        qualified_committed_reduction_kw,
        trial_receipts,
    })
}

fn validate_response_report(
    identity: &GridFirmCapacityIdentity,
    trial: &GridFirmCapacityTrial,
    report: &GridResponseReport,
) -> Result<(), GridFirmCapacityError> {
    let event_id = trial.event_id.as_str();
    if report.identity.event_id() != event_id {
        return Err(GridFirmCapacityError::ResponseReportEventIdMismatch {
            event_id: trial.event_id.clone(),
        });
    }
    if report.identity.service_product_id() != identity.service_product_id {
        return Err(GridFirmCapacityError::ServiceProductMismatch {
            event_id: trial.event_id.clone(),
        });
    }
    if report.identity.measurement_boundary_id() != identity.measurement_boundary_id {
        return Err(GridFirmCapacityError::MeasurementBoundaryMismatch {
            event_id: trial.event_id.clone(),
        });
    }
    if report.identity.baseline_method_id() != identity.baseline_method_id {
        return Err(GridFirmCapacityError::BaselineMethodMismatch {
            event_id: trial.event_id.clone(),
        });
    }
    if report.identity.response_configuration_digest()
        != identity.response_configuration_digest
    {
        return Err(GridFirmCapacityError::ResponseConfigurationMismatch {
            event_id: trial.event_id.clone(),
        });
    }
    if report.target_reduction_kw != identity.committed_reduction_kw {
        return Err(GridFirmCapacityError::CommittedCapacityMismatch {
            event_id: trial.event_id.clone(),
            expected_kw: identity.committed_reduction_kw,
            observed_kw: report.target_reduction_kw,
        });
    }
    if report.passed != response_gates_pass(report) {
        return Err(GridFirmCapacityError::InconsistentResponseReport {
            event_id: trial.event_id.clone(),
        });
    }
    Ok(())
}

fn response_gates_pass(report: &GridResponseReport) -> bool {
    report.response_latency_pass
        && report.sustained_delivery_pass
        && report.underdelivery_energy_pass
        && report.overshoot_energy_pass
        && report.ramp_limit_pass
}

fn require_id(name: &'static str, value: &str) -> Result<(), GridFirmCapacityError> {
    if value.trim().is_empty() {
        return Err(GridFirmCapacityError::MissingIdentity { name });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::GridResponseIdentity;

    fn identity() -> GridFirmCapacityIdentity {
        GridFirmCapacityIdentity::new(
            "portfolio-north",
            "scheduled-opportunity-manifest-v1",
            "frequency-response-class-a",
            "pcc-site-1",
            "baseline-method-v1",
            "response-config-v1",
            "hot-ambient-high-workload",
            "independence-blocking-plan-v1",
            100.0,
        ).unwrap()
    }

    fn contract_for(
        event_ids: Vec<String>,
        minimum_trial_count: usize,
        minimum_success_probability: f64,
        confidence_failure_probability: f64,
    ) -> GridFirmCapacityContract {
        GridFirmCapacityContract::new(
            identity(),
            event_ids,
            minimum_trial_count,
            minimum_success_probability,
            confidence_failure_probability,
        ).unwrap()
    }

    fn contract() -> GridFirmCapacityContract {
        contract_for(
            (0..5).map(|i| format!("event-{i}")).collect(),
            5,
            0.60,
            0.05,
        )
    }

    fn response_report(event_id: &str, passed: bool) -> GridResponseReport {
        let response_identity = GridResponseIdentity::new(
            event_id,
            "frequency-response-class-a",
            "pcc-site-1",
            "baseline-method-v1",
            "response-config-v1",
        ).unwrap();
        GridResponseReport {
            identity: response_identity,
            instruction_issued_at_ns: 0,
            service_end_ns: 4_000_000_000,
            sample_interval_ns: 1_000_000_000,
            sample_count: 4,
            target_reduction_kw: 100.0,
            requested_reduction_energy_kwh: 100.0 / 900.0,
            delivered_reduction_energy_kwh: if passed { 100.0 / 900.0 } else { 50.0 / 900.0 },
            underdelivery_energy_kwh: if passed { 0.0 } else { 50.0 / 900.0 },
            overshoot_energy_kwh: 0.0,
            delivered_energy_fraction: if passed { 1.0 } else { 0.5 },
            underdelivery_energy_fraction: if passed { 0.0 } else { 0.5 },
            overshoot_energy_fraction: 0.0,
            response_latency_ns: Some(1_000_000_000),
            maximum_response_latency_ns: 2_000_000_000,
            compliant_sample_count: if passed { 4 } else { 2 },
            compliant_sample_fraction: if passed { 1.0 } else { 0.5 },
            maximum_underdelivery_kw: if passed { 0.0 } else { 50.0 },
            maximum_overshoot_kw: 0.0,
            maximum_measured_ramp_kw_per_second: 100.0,
            response_latency_pass: passed,
            sustained_delivery_pass: passed,
            underdelivery_energy_pass: passed,
            overshoot_energy_pass: true,
            ramp_limit_pass: true,
            passed,
            sub_sample_transients_unverified: true,
            sample_receipts: Vec::new(),
        }
    }

    fn passing_trials(count: usize) -> Vec<GridFirmCapacityTrial> {
        (0..count).map(|i| {
            GridFirmCapacityTrial::evaluated(
                response_report(&format!("event-{i}"), true),
                "hot-ambient-high-workload",
            ).unwrap()
        }).collect()
    }

    #[test]
    fn all_passes_accredit_only_the_committed_capacity_when_bound_is_sufficient() {
        let trials = passing_trials(100);
        let expanded = contract_for(
            (0..100).map(|i| format!("event-{i}")).collect(),
            5,
            0.60,
            0.05,
        );
        let input = GridFirmCapacityEvaluation::new(expanded, trials).unwrap();
        let report = evaluate_grid_firm_capacity(&input).unwrap();
        assert_eq!(report.successful_trial_count, 100);
        assert_eq!(report.failed_trial_count, 0);
        assert!(report.empirical_success_probability > 0.999);
        assert!(report.event_response_reliability_qualified);
        assert_eq!(report.qualified_committed_reduction_kw, Some(100.0));
        assert_eq!(report.scheduled_event_ids.len(), 100);
        assert_eq!(report.scheduled_event_ids[0], "event-0");
        assert!(report.lower_confidence_success_probability >= 0.60);
    }

    #[test]
    fn pins_one_sided_hoeffding_lower_bound_for_twenty_perfect_trials() {
        let input = GridFirmCapacityEvaluation::new(
            contract_for(
                (0..20).map(|i| format!("event-{i}")).collect(),
                20,
                0.60,
                0.05,
            ),
            passing_trials(20),
        ).unwrap();
        let report = evaluate_grid_firm_capacity(&input).unwrap();
        let expected_radius = (20.0_f64.ln() / 40.0).sqrt();
        assert!((report.hoeffding_radius - expected_radius).abs() < 1e-12);
        assert!((report.lower_confidence_success_probability - (1.0 - expected_radius)).abs() < 1e-12);
        assert!(!report.firm_capacity_qualified || report.lower_confidence_success_probability >= 0.60);
    }

    #[test]
    fn insufficient_trial_count_cannot_accredit_best_case_capacity() {
        assert!(matches!(
            GridFirmCapacityEvaluation::new(contract(), passing_trials(4)),
            Err(GridFirmCapacityError::InsufficientTrials { .. })
        ));
    }

    #[test]
    fn one_failed_dispatch_reduces_conservative_capacity_confidence() {
        let mut trials = passing_trials(100);
        trials.push(GridFirmCapacityTrial::evaluated(
            response_report("event-failed", false),
            "hot-ambient-high-workload",
        ).unwrap());
        let input = GridFirmCapacityEvaluation::new(
            contract_for(
                (0..100).map(|i| format!("event-{i}"))
                    .chain(std::iter::once("event-failed".to_string()))
                    .collect(),
                5,
                0.60,
                0.05,
            ),
            trials,
        ).unwrap();
        let report = evaluate_grid_firm_capacity(&input).unwrap();
        assert_eq!(report.failed_trial_count, 1);
        assert_eq!(report.successful_trial_count, 100);
        assert!(report.lower_confidence_success_probability < report.empirical_success_probability);
    }

    #[test]
    fn unavailable_dispatch_opportunity_counts_as_failure() {
        let mut trials = passing_trials(100);
        trials.push(GridFirmCapacityTrial::unavailable(
            "event-unavailable",
            "hot-ambient-high-workload",
            "controller unreachable",
            "availability-log-001",
        ).unwrap());
        let input = GridFirmCapacityEvaluation::new(
            contract_for(
                (0..100).map(|i| format!("event-{i}"))
                    .chain(std::iter::once("event-unavailable".to_string()))
                    .collect(),
                5,
                0.60,
                0.05,
            ),
            trials,
        ).unwrap();
        let report = evaluate_grid_firm_capacity(&input).unwrap();
        assert_eq!(report.unavailable_trial_count, 1);
        assert_eq!(report.failed_trial_count, 1);
        assert_eq!(report.successful_trial_count, 100);
        let unavailable = report.trial_receipts.iter()
            .find(|r| r.event_id == "event-unavailable").unwrap();
        assert!(!unavailable.available);
        assert!(!unavailable.successful);
        assert_eq!(unavailable.availability_evidence_id.as_deref(), Some("availability-log-001"));
    }

    #[test]
    fn mismatched_product_boundary_baseline_or_configuration_is_rejected() {
        let cases = [
            (
                GridResponseIdentity::new(
                    "event-bad", "different-product", "pcc-site-1", "baseline-method-v1",
                    "response-config-v1",
                ).unwrap(),
                0_u8,
            ),
            (
                GridResponseIdentity::new(
                    "event-bad", "frequency-response-class-a", "different-pcc",
                    "baseline-method-v1", "response-config-v1",
                ).unwrap(),
                1_u8,
            ),
            (
                GridResponseIdentity::new(
                    "event-bad", "frequency-response-class-a", "pcc-site-1",
                    "different-baseline", "response-config-v1",
                ).unwrap(),
                2_u8,
            ),
            (
                GridResponseIdentity::new(
                    "event-bad", "frequency-response-class-a", "pcc-site-1",
                    "baseline-method-v1", "different-response-config",
                ).unwrap(),
                3_u8,
            ),
        ];
        for (response_identity, expected_error) in cases {
            let mut report = response_report("event-bad", true);
            report.identity = response_identity;
            let evaluation = GridFirmCapacityEvaluation::new(
                contract_for(vec!["event-bad".to_string()], 1, 0.60, 0.05),
                vec![GridFirmCapacityTrial::evaluated(
                    report, "hot-ambient-high-workload"
                ).unwrap()],
            );
            match expected_error {
                0 => assert!(matches!(evaluation, Err(GridFirmCapacityError::ServiceProductMismatch { .. }))),
                1 => assert!(matches!(evaluation, Err(GridFirmCapacityError::MeasurementBoundaryMismatch { .. }))),
                2 => assert!(matches!(evaluation, Err(GridFirmCapacityError::BaselineMethodMismatch { .. }))),
                3 => assert!(matches!(evaluation, Err(GridFirmCapacityError::ResponseConfigurationMismatch { .. }))),
                _ => unreachable!(),
            }
        }
    }

    #[test]
    fn mismatched_committed_capacity_and_condition_stratum_are_rejected() {
        let mut bad = response_report("event-bad", true);
        bad.target_reduction_kw = 110.0;
        assert!(matches!(
            GridFirmCapacityEvaluation::new(
                contract_for(vec!["event-bad".to_string()], 1, 0.60, 0.05),
                vec![GridFirmCapacityTrial::evaluated(
                    bad, "hot-ambient-high-workload"
                ).unwrap()]
            ),
            Err(GridFirmCapacityError::CommittedCapacityMismatch { .. })
        ));
        assert!(matches!(
            GridFirmCapacityEvaluation::new(
                contract_for(vec!["event-stratum".to_string()], 1, 0.60, 0.05),
                vec![GridFirmCapacityTrial::unavailable(
                    "event-stratum", "cold-low-workload", "not requested", "reason-log"
                ).unwrap()]
            ),
            Err(GridFirmCapacityError::ConditionStratumMismatch { .. })
        ));
    }

    #[test]
    fn missing_scheduled_opportunity_is_rejected_instead_of_silently_dropped() {
        let mut trials = passing_trials(5);
        trials.pop();
        let manifest = contract_for(
            (0..5).map(|i| format!("event-{i}")).collect(),
            4,
            0.60,
            0.05,
        );
        assert!(matches!(
            GridFirmCapacityEvaluation::new(manifest, trials),
            Err(GridFirmCapacityError::ScheduledOpportunityCountMismatch { .. })
        ));
    }

    #[test]
    fn an_unexpected_event_cannot_replace_a_scheduled_opportunity() {
        let mut trials = passing_trials(5);
        trials[4] = GridFirmCapacityTrial::evaluated(
            response_report("unscheduled-event", true),
            "hot-ambient-high-workload",
        ).unwrap();
        assert!(matches!(
            GridFirmCapacityEvaluation::new(contract(), trials),
            Err(GridFirmCapacityError::UnexpectedScheduledOpportunity(_))
        ));
    }

    #[test]
    fn duplicate_event_ids_are_rejected() {
        let first = GridFirmCapacityTrial::evaluated(
            response_report("same-event", true),
            "hot-ambient-high-workload",
        ).unwrap();
        let second = GridFirmCapacityTrial::unavailable(
            "same-event",
            "hot-ambient-high-workload",
            "duplicate opportunity",
            "availability-log",
        ).unwrap();
        assert!(matches!(
            GridFirmCapacityEvaluation::new(
                contract_for(
                    vec!["same-event".to_string(), "other-event".to_string()],
                    2,
                    0.60,
                    0.05,
                ),
                vec![first, second],
            ),
            Err(GridFirmCapacityError::DuplicateEventId(_))
        ));
    }

    #[test]
    fn hoeffding_lower_bound_can_fail_despite_a_high_empirical_pass_rate() {
        let weak_contract = contract_for(
            (0..20).map(|i| format!("event-{i}")).collect(),
            5,
            0.99,
            0.05,
        );
        let input = GridFirmCapacityEvaluation::new(weak_contract, passing_trials(20)).unwrap();
        let report = evaluate_grid_firm_capacity(&input).unwrap();
        assert_eq!(report.empirical_success_probability, 1.0);
        assert!(!report.firm_capacity_qualified);
        assert_eq!(report.accredited_reduction_kw, None);
    }

    #[test]
    fn input_order_does_not_change_the_firm_capacity_report() {
        let trials = passing_trials(100);
        let many_event_contract = contract_for(
            (0..100).map(|i| format!("event-{i}")).collect(),
            5,
            0.60,
            0.05,
        );
        let first = GridFirmCapacityEvaluation::new(many_event_contract.clone(), trials.clone()).unwrap();
        let mut reversed = trials;
        reversed.reverse();
        let second = GridFirmCapacityEvaluation::new(many_event_contract, reversed).unwrap();
        assert_eq!(
            evaluate_grid_firm_capacity(&first).unwrap(),
            evaluate_grid_firm_capacity(&second).unwrap()
        );
    }

    #[test]
    fn rejects_invalid_probabilities_and_missing_independence_identity() {
        assert!(GridFirmCapacityIdentity::new(
            "portfolio", "opportunities", "product", "pcc", "baseline", "config", "stratum", "", 100.0
        ).is_err());
        let ids = (0..5).map(|i| format!("event-{i}")).collect::<Vec<_>>();
        assert!(GridFirmCapacityContract::new(identity(), ids.clone(), 0, 0.9, 0.05).is_err());
        assert!(GridFirmCapacityContract::new(identity(), ids.clone(), 5, 1.0, 0.05).is_err());
        assert!(GridFirmCapacityContract::new(identity(), ids.clone(), 5, 0.9, 0.0).is_err());
        assert!(GridFirmCapacityContract::new(identity(), ids.clone(), 5, 0.9, 1.0).is_err());
        assert!(GridFirmCapacityContract::new(
            identity(), vec!["event-0".to_string()], 5, 0.9, 0.05
        ).is_err());
        assert!(GridFirmCapacityContract::new(
            identity(), vec!["same".to_string(), "same".to_string()], 1, 0.9, 0.05
        ).is_err());
    }

    #[test]
    fn rejects_inconsistent_response_pass_flag() {
        let mut bad = response_report("event-inconsistent", true);
        bad.ramp_limit_pass = false;
        assert!(matches!(
            GridFirmCapacityEvaluation::new(
                contract_for(vec!["event-inconsistent".to_string()], 1, 0.60, 0.05),
                vec![GridFirmCapacityTrial::evaluated(
                    bad, "hot-ambient-high-workload"
                ).unwrap()]
            ),
            Err(GridFirmCapacityError::InconsistentResponseReport { .. })
        ));
    }
}
