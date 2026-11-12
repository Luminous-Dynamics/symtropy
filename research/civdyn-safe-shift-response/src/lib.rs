// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Finite-known-truth safe-shift response reference.
//!
//! This crate deliberately separates:
//! 1. an anytime-valid monitor under an explicit IID Bernoulli null contract;
//! 2. observation-cell robust action feasibility;
//! 3. evaluator-only admission of independent requalification evidence.
//!
//! An alarm is not cause attribution. A non-alarm is not proof of stationarity.
//! Neither changes the plausible model set without a separately admitted receipt.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

const TOLERANCE: f64 = 1.0e-9;
pub const NULL_EVENT_PROBABILITY: f64 = 0.1;
pub const BET_EVENT_PROBABILITY: f64 = 0.5;
pub const FALSE_ALARM_LEVEL: f64 = 0.01;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonitorValidity {
    Valid,
    Invalid,
}

impl MonitorValidity {
    pub const fn id(self) -> &'static str {
        match self {
            Self::Valid => "valid",
            Self::Invalid => "invalid",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndependenceValidation {
    SourceContractValidated,
    Invalid,
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MonitorContract {
    pub contract_id: String,
    pub reference_digest: String,
    pub stream_id: String,
    pub independence_contract_id: String,
    pub null_event_probability: f64,
    pub bet_event_probability: f64,
    pub alpha: f64,
    pub maximum_lateness_ticks: u64,
    /// This is a registered design/collection assumption, not something
    /// established merely by inspecting the observed residual sequence.
    pub iid_null_contract_registered: bool,
}

impl Default for MonitorContract {
    fn default() -> Self {
        Self {
            contract_id: "iid-bernoulli-residual-p0-0.1-v1".to_owned(),
            reference_digest: "sha256-iid-bernoulli-reference-v1".to_owned(),
            stream_id: "observation-residual-stream-v1".to_owned(),
            independence_contract_id: "registered-independent-residual-sampling-v1".to_owned(),
            null_event_probability: NULL_EVENT_PROBABILITY,
            bet_event_probability: BET_EVENT_PROBABILITY,
            alpha: FALSE_ALARM_LEVEL,
            maximum_lateness_ticks: 1,
            iid_null_contract_registered: true,
        }
    }
}

impl MonitorContract {
    pub fn validate(&self) -> Result<(), String> {
        if self.contract_id.trim().is_empty()
            || self.reference_digest.trim().is_empty()
            || self.stream_id.trim().is_empty()
            || self.independence_contract_id.trim().is_empty()
        {
            return Err("monitor contract IDs and digests must be non-empty".to_owned());
        }
        for (name, value) in [
            ("null probability", self.null_event_probability),
            ("bet probability", self.bet_event_probability),
            ("alpha", self.alpha),
        ] {
            if !value.is_finite() || value <= 0.0 || value >= 1.0 {
                return Err(format!("{name} must be finite and strictly between zero and one"));
            }
        }
        if !self.iid_null_contract_registered {
            return Err("the IID null contract is not registered".to_owned());
        }
        Ok(())
    }

    pub fn log_alarm_threshold(&self) -> f64 {
        (1.0 / self.alpha).ln()
    }

    pub fn log_likelihood_increment(&self, event: bool) -> f64 {
        if event {
            (self.bet_event_probability / self.null_event_probability).ln()
        } else {
            ((1.0 - self.bet_event_probability) / (1.0 - self.null_event_probability)).ln()
        }
    }

    pub fn null_expected_one_step_factor(&self) -> f64 {
        self.null_event_probability
            * (self.bet_event_probability / self.null_event_probability)
            + (1.0 - self.null_event_probability)
                * ((1.0 - self.bet_event_probability)
                    / (1.0 - self.null_event_probability))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResidualEvent {
    pub event_id: String,
    pub stream_id: String,
    pub contract_digest: String,
    pub event_time: u64,
    pub arrival_time: u64,
    pub residual_event: bool,
    pub independence_contract_id: String,
    pub independence_validation: IndependenceValidation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MonitorRejectReason {
    EmptyEventId,
    WrongStream,
    WrongContractDigest,
    DuplicateEventId,
    EventTimeNotIncreasing,
    ArrivalTimeNotIncreasing,
    EventArrivedBeforeItOccurred,
    EventOutsideLatenessBudget,
    IndependenceContractMismatch,
    IndependenceNotValidated,
    InvalidLikelihoodUpdate,
    MonitorAlreadyInvalid,
}

impl MonitorRejectReason {
    pub const fn id(&self) -> &'static str {
        match self {
            Self::EmptyEventId => "empty_event_id",
            Self::WrongStream => "wrong_stream",
            Self::WrongContractDigest => "wrong_contract_digest",
            Self::DuplicateEventId => "duplicate_event_id",
            Self::EventTimeNotIncreasing => "event_time_not_increasing",
            Self::ArrivalTimeNotIncreasing => "arrival_time_not_increasing",
            Self::EventArrivedBeforeItOccurred => "arrival_before_event",
            Self::EventOutsideLatenessBudget => "event_outside_lateness_budget",
            Self::IndependenceContractMismatch => "independence_contract_mismatch",
            Self::IndependenceNotValidated => "independence_not_validated",
            Self::InvalidLikelihoodUpdate => "invalid_likelihood_update",
            Self::MonitorAlreadyInvalid => "monitor_already_invalid",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MonitorSnapshot {
    pub contract_id: String,
    pub reference_digest: String,
    pub stream_id: String,
    pub validity: MonitorValidity,
    pub alarm_crossed: bool,
    pub log_evidence: f64,
    pub log_alarm_threshold: f64,
    pub processed_events: usize,
    pub event_time_watermark: Option<u64>,
    pub arrival_time_watermark: Option<u64>,
    pub anytime_iid_bound_applicable: bool,
    pub invalid_reason: Option<MonitorRejectReason>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MonitorStep {
    pub accepted: bool,
    pub rejected_reason: Option<MonitorRejectReason>,
    pub snapshot: MonitorSnapshot,
}

#[derive(Debug, Clone)]
pub struct AnytimeBernoulliMonitor {
    contract: MonitorContract,
    validity: MonitorValidity,
    alarm_crossed: bool,
    log_evidence: f64,
    processed_events: usize,
    event_time_watermark: Option<u64>,
    arrival_time_watermark: Option<u64>,
    seen_event_ids: BTreeSet<String>,
    invalid_reason: Option<MonitorRejectReason>,
}

impl AnytimeBernoulliMonitor {
    pub fn new(contract: MonitorContract) -> Result<Self, String> {
        contract.validate()?;
        Ok(Self {
            contract,
            validity: MonitorValidity::Valid,
            alarm_crossed: false,
            log_evidence: 0.0,
            processed_events: 0,
            event_time_watermark: None,
            arrival_time_watermark: None,
            seen_event_ids: BTreeSet::new(),
            invalid_reason: None,
        })
    }

    pub fn snapshot(&self) -> MonitorSnapshot {
        MonitorSnapshot {
            contract_id: self.contract.contract_id.clone(),
            reference_digest: self.contract.reference_digest.clone(),
            stream_id: self.contract.stream_id.clone(),
            validity: self.validity,
            alarm_crossed: self.alarm_crossed,
            log_evidence: self.log_evidence,
            log_alarm_threshold: self.contract.log_alarm_threshold(),
            processed_events: self.processed_events,
            event_time_watermark: self.event_time_watermark,
            arrival_time_watermark: self.arrival_time_watermark,
            anytime_iid_bound_applicable: self.validity == MonitorValidity::Valid
                && self.contract.iid_null_contract_registered,
            invalid_reason: self.invalid_reason.clone(),
        }
    }

    pub fn process(&mut self, event: &ResidualEvent) -> MonitorStep {
        if self.validity == MonitorValidity::Invalid {
            return MonitorStep {
                accepted: false,
                rejected_reason: Some(MonitorRejectReason::MonitorAlreadyInvalid),
                snapshot: self.snapshot(),
            };
        }

        if let Some(reason) = self.validate_event(event) {
            self.validity = MonitorValidity::Invalid;
            self.invalid_reason = Some(reason.clone());
            return MonitorStep {
                accepted: false,
                rejected_reason: Some(reason),
                snapshot: self.snapshot(),
            };
        }

        let next_log_evidence =
            self.log_evidence + self.contract.log_likelihood_increment(event.residual_event);
        if !next_log_evidence.is_finite() {
            self.validity = MonitorValidity::Invalid;
            self.invalid_reason = Some(MonitorRejectReason::InvalidLikelihoodUpdate);
            return MonitorStep {
                accepted: false,
                rejected_reason: self.invalid_reason.clone(),
                snapshot: self.snapshot(),
            };
        }

        self.log_evidence = next_log_evidence;
        self.alarm_crossed |= self.log_evidence + TOLERANCE
            >= self.contract.log_alarm_threshold();
        self.processed_events += 1;
        self.event_time_watermark = Some(event.event_time);
        self.arrival_time_watermark = Some(event.arrival_time);
        self.seen_event_ids.insert(event.event_id.clone());

        MonitorStep {
            accepted: true,
            rejected_reason: None,
            snapshot: self.snapshot(),
        }
    }

    fn validate_event(&self, event: &ResidualEvent) -> Option<MonitorRejectReason> {
        if event.event_id.trim().is_empty() {
            return Some(MonitorRejectReason::EmptyEventId);
        }
        if event.stream_id != self.contract.stream_id {
            return Some(MonitorRejectReason::WrongStream);
        }
        if event.contract_digest != self.contract.reference_digest {
            return Some(MonitorRejectReason::WrongContractDigest);
        }
        if self.seen_event_ids.contains(&event.event_id) {
            return Some(MonitorRejectReason::DuplicateEventId);
        }
        if self.event_time_watermark.is_some_and(|last| event.event_time <= last) {
            return Some(MonitorRejectReason::EventTimeNotIncreasing);
        }
        if self.arrival_time_watermark.is_some_and(|last| event.arrival_time < last) {
            return Some(MonitorRejectReason::ArrivalTimeNotIncreasing);
        }
        if event.arrival_time < event.event_time {
            return Some(MonitorRejectReason::EventArrivedBeforeItOccurred);
        }
        if event.arrival_time - event.event_time > self.contract.maximum_lateness_ticks {
            return Some(MonitorRejectReason::EventOutsideLatenessBudget);
        }
        if event.independence_contract_id != self.contract.independence_contract_id {
            return Some(MonitorRejectReason::IndependenceContractMismatch);
        }
        if event.independence_validation != IndependenceValidation::SourceContractValidated {
            return Some(MonitorRejectReason::IndependenceNotValidated);
        }
        None
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ActionOutcome {
    pub action_id: String,
    pub service_units: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ObservationModel {
    /// Opaque, candidate-visible structural model-family identifier. It is
    /// not a label for the evaluator's true physical cause.
    pub model_id: String,
    pub observation_id: String,
    pub actions: Vec<ActionOutcome>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeploymentRegime {
    AutomaticallyDeployable,
    Remediable,
    HardNonDeployable,
}

impl DeploymentRegime {
    pub const fn id(&self) -> &'static str {
        match self {
            Self::AutomaticallyDeployable => "automatically_deployable",
            Self::Remediable => "remediable",
            Self::HardNonDeployable => "hard_non_deployable",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CandidateObservationCell {
    pub observation_id: String,
    pub common_safe_action_ids: Vec<String>,
    pub selected_action_id: Option<String>,
    pub guaranteed_service_lower_bound: Option<f64>,
    pub disposition: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CandidateGateReport {
    pub service_floor: f64,
    pub all_observation_cells_feasible: bool,
    pub cells: Vec<CandidateObservationCell>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EvaluatorWorldActionAudit {
    pub model_id: String,
    pub action_id: String,
    pub service_units: f64,
    pub meets_service_floor: bool,
    pub shortfall_service_units: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EvaluatorObservationCellAudit {
    pub observation_id: String,
    pub model_ids: Vec<String>,
    pub actions: Vec<EvaluatorWorldActionAudit>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EvaluatorGateAudit {
    pub service_floor: f64,
    pub cells: Vec<EvaluatorObservationCellAudit>,
}

fn validate_models(models: &[ObservationModel], service_floor: f64) -> Result<(), String> {
    if models.is_empty() {
        return Err("at least one observation model is required".to_owned());
    }
    if !service_floor.is_finite() {
        return Err("service floor must be finite".to_owned());
    }

    let mut model_ids = BTreeSet::new();
    let expected_actions: BTreeSet<String> = models[0]
        .actions
        .iter()
        .map(|action| action.action_id.clone())
        .collect();
    if expected_actions.is_empty() {
        return Err("at least one physical action is required".to_owned());
    }

    for model in models {
        if model.model_id.trim().is_empty() || model.observation_id.trim().is_empty() {
            return Err("model and observation identities must be non-empty".to_owned());
        }
        if !model_ids.insert(model.model_id.as_str()) {
            return Err(format!("duplicate observation model {}", model.model_id));
        }
        let mut action_ids = BTreeSet::new();
        for action in &model.actions {
            if action.action_id.trim().is_empty() || !action.service_units.is_finite() {
                return Err(format!("invalid action outcome in {}", model.model_id));
            }
            if !action_ids.insert(action.action_id.clone()) {
                return Err(format!("duplicate action in {}", model.model_id));
            }
        }
        if action_ids != expected_actions {
            return Err("every model must declare the same complete action set".to_owned());
        }
    }
    Ok(())
}

fn grouped_models<'a>(
    models: &'a [ObservationModel],
) -> BTreeMap<String, Vec<&'a ObservationModel>> {
    let mut groups: BTreeMap<String, Vec<&ObservationModel>> = BTreeMap::new();
    for model in models {
        groups
            .entry(model.observation_id.clone())
            .or_default()
            .push(model);
    }
    groups
}

pub fn candidate_gate_report(
    models: &[ObservationModel],
    service_floor: f64,
) -> Result<CandidateGateReport, String> {
    validate_models(models, service_floor)?;
    let groups = grouped_models(models);
    let mut cells = Vec::with_capacity(groups.len());

    for (observation_id, group) in groups {
        let mut candidates = Vec::new();
        let mut action_ids: Vec<String> = group[0]
            .actions
            .iter()
            .map(|action| action.action_id.clone())
            .collect();
        action_ids.sort();

        for action_id in action_ids {
            let lower_bound = group
                .iter()
                .map(|model| {
                    model
                        .actions
                        .iter()
                        .find(|action| action.action_id == action_id)
                        .expect("common action set")
                        .service_units
                })
                .fold(f64::INFINITY, f64::min);

            if lower_bound + TOLERANCE >= service_floor {
                candidates.push((action_id, lower_bound));
            }
        }

        candidates.sort_by(|left, right| {
            right
                .1
                .total_cmp(&left.1)
                .then_with(|| left.0.cmp(&right.0))
        });
        let selected = candidates.first();

        cells.push(CandidateObservationCell {
            observation_id,
            common_safe_action_ids: candidates.iter().map(|(id, _)| id.clone()).collect(),
            selected_action_id: selected.map(|(id, _)| id.clone()),
            guaranteed_service_lower_bound: selected.map(|(_, lower)| *lower),
            disposition: if selected.is_some() {
                "action_admitted".to_owned()
            } else {
                "no_common_safe_action_abstain".to_owned()
            },
        });
    }

    cells.sort_by(|left, right| left.observation_id.cmp(&right.observation_id));
    let all_observation_cells_feasible = cells.iter().all(|cell| cell.selected_action_id.is_some());

    Ok(CandidateGateReport {
        service_floor,
        all_observation_cells_feasible,
        cells,
    })
}

/// Evaluator-only audit. Do not pass this structure to the deployed controller.
pub fn evaluator_gate_audit(
    models: &[ObservationModel],
    service_floor: f64,
) -> Result<EvaluatorGateAudit, String> {
    validate_models(models, service_floor)?;
    let groups = grouped_models(models);
    let mut cells = Vec::with_capacity(groups.len());

    for (observation_id, group) in groups {
        let mut actions = Vec::new();
        for model in &group {
            for action in &model.actions {
                actions.push(EvaluatorWorldActionAudit {
                    model_id: model.model_id.clone(),
                    action_id: action.action_id.clone(),
                    service_units: action.service_units,
                    meets_service_floor: action.service_units + TOLERANCE >= service_floor,
                    shortfall_service_units: (service_floor - action.service_units).max(0.0),
                });
            }
        }
        actions.sort_by(|left, right| {
            left.model_id
                .cmp(&right.model_id)
                .then_with(|| left.action_id.cmp(&right.action_id))
        });
        let mut model_ids: Vec<String> = group.iter().map(|model| model.model_id.clone()).collect();
        model_ids.sort();
        cells.push(EvaluatorObservationCellAudit {
            observation_id,
            model_ids,
            actions,
        });
    }
    cells.sort_by(|left, right| left.observation_id.cmp(&right.observation_id));

    Ok(EvaluatorGateAudit {
        service_floor,
        cells,
    })
}

#[derive(Debug, Clone, PartialEq)]
pub struct RefinementPrediction {
    pub model_id: String,
    pub observation_id: String,
    pub latency_service_loss_per_tick: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ObservationRefinement {
    pub refinement_id: String,
    pub protocol_digest: String,
    pub energy_cost_service_units: f64,
    pub latency_ticks: u32,
    pub predictions: Vec<RefinementPrediction>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeploymentClassification {
    pub regime: DeploymentRegime,
    pub current_candidate_report: CandidateGateReport,
    pub refined_candidate_report: Option<CandidateGateReport>,
    pub refinement_id: Option<String>,
    pub worst_case_refinement_penalty_service_units: Option<f64>,
}

fn apply_refinement(
    models: &[ObservationModel],
    refinement: &ObservationRefinement,
) -> Result<(Vec<ObservationModel>, f64), String> {
    if refinement.refinement_id.trim().is_empty() || refinement.protocol_digest.trim().is_empty() {
        return Err("refinement identity and protocol digest must be non-empty".to_owned());
    }
    if !refinement.energy_cost_service_units.is_finite()
        || refinement.energy_cost_service_units < 0.0
    {
        return Err("refinement energy cost must be finite and non-negative".to_owned());
    }
    if refinement.predictions.len() != models.len() {
        return Err("refinement must provide exactly one prediction per model".to_owned());
    }

    let mut predictions = BTreeMap::new();
    let mut maximum_penalty = refinement.energy_cost_service_units;
    for prediction in &refinement.predictions {
        if prediction.model_id.trim().is_empty()
            || prediction.observation_id.trim().is_empty()
            || !prediction.latency_service_loss_per_tick.is_finite()
            || prediction.latency_service_loss_per_tick < 0.0
        {
            return Err("invalid refinement prediction".to_owned());
        }
        let penalty = refinement.energy_cost_service_units
            + f64::from(refinement.latency_ticks) * prediction.latency_service_loss_per_tick;
        if !penalty.is_finite() {
            return Err("non-finite refinement penalty".to_owned());
        }
        maximum_penalty = maximum_penalty.max(penalty);
        if predictions.insert(prediction.model_id.clone(), (prediction, penalty)).is_some() {
            return Err(format!("duplicate refinement model {}", prediction.model_id));
        }
    }

    let mut refined_models = Vec::with_capacity(models.len());
    for model in models {
        let (prediction, penalty) = predictions
            .get(&model.model_id)
            .ok_or_else(|| format!("missing refinement for {}", model.model_id))?;
        let actions = model
            .actions
            .iter()
            .map(|action| ActionOutcome {
                action_id: action.action_id.clone(),
                service_units: action.service_units - penalty,
            })
            .collect();
        refined_models.push(ObservationModel {
            model_id: model.model_id.clone(),
            observation_id: prediction.observation_id.clone(),
            actions,
        });
    }

    Ok((refined_models, maximum_penalty))
}

pub fn classify_deployment(
    models: &[ObservationModel],
    service_floor: f64,
    refinement: Option<&ObservationRefinement>,
    maximum_refinement_penalty_service_units: f64,
) -> Result<DeploymentClassification, String> {
    if !maximum_refinement_penalty_service_units.is_finite()
        || maximum_refinement_penalty_service_units < 0.0
    {
        return Err("maximum refinement budget must be finite and non-negative".to_owned());
    }
    let current = candidate_gate_report(models, service_floor)?;
    if current.all_observation_cells_feasible {
        return Ok(DeploymentClassification {
            regime: DeploymentRegime::AutomaticallyDeployable,
            current_candidate_report: current,
            refined_candidate_report: None,
            refinement_id: None,
            worst_case_refinement_penalty_service_units: None,
        });
    }

    let Some(refinement) = refinement else {
        return Ok(DeploymentClassification {
            regime: DeploymentRegime::HardNonDeployable,
            current_candidate_report: current,
            refined_candidate_report: None,
            refinement_id: None,
            worst_case_refinement_penalty_service_units: None,
        });
    };

    let (refined_models, maximum_penalty) = apply_refinement(models, refinement)?;
    if maximum_penalty > maximum_refinement_penalty_service_units + TOLERANCE {
        return Ok(DeploymentClassification {
            regime: DeploymentRegime::HardNonDeployable,
            current_candidate_report: current,
            refined_candidate_report: None,
            refinement_id: Some(refinement.refinement_id.clone()),
            worst_case_refinement_penalty_service_units: Some(maximum_penalty),
        });
    }
    let refined_report = candidate_gate_report(&refined_models, service_floor)?;
    let regime = if refined_report.all_observation_cells_feasible {
        DeploymentRegime::Remediable
    } else {
        DeploymentRegime::HardNonDeployable
    };

    Ok(DeploymentClassification {
        regime,
        current_candidate_report: current,
        refined_candidate_report: Some(refined_report),
        refinement_id: Some(refinement.refinement_id.clone()),
        worst_case_refinement_penalty_service_units: Some(maximum_penalty),
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActualCalibrationWorld {
    ReferenceValid,
    ObservationShifted,
    CommonCauseDependent,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RequalificationRequirements {
    pub reference_digest: String,
    pub sensor_id: String,
    pub monitored_path_id: String,
    pub allowed_independent_paths: BTreeSet<String>,
    pub configuration_digest: String,
    pub unit: String,
    pub calibration_revision: String,
    pub allowed_attesters: BTreeSet<String>,
    pub maximum_measurement_age: u64,
    pub now: u64,
    pub reference_model_scope_id: String,
    pub expected_reference_check_value: f64,
    pub check_tolerance: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RawRequalificationReceipt {
    pub receipt_id: String,
    pub reference_digest: String,
    pub sensor_id: String,
    pub source_path_id: String,
    pub configuration_digest: String,
    pub unit: String,
    pub calibration_revision: String,
    pub attester_id: String,
    pub attestation_digest: String,
    pub event_time: u64,
    pub arrival_time: u64,
    pub valid_until: u64,
    pub physical_cross_check_value: f64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RequalificationRejectReason {
    EmptyIdentity,
    ReferenceMismatch,
    SensorMismatch,
    SharedOrUnregisteredPath,
    ConfigurationMismatch,
    UnitMismatch,
    CalibrationMismatch,
    UnknownAttester,
    MissingAttestationDigest,
    InvalidClockOrder,
    StaleEvidence,
    ExpiredScope,
    NonFiniteCrossCheck,
    CrossCheckOutsideReferenceTolerance,
    CurrentWorldNotReferenceValid,
}

impl RequalificationRejectReason {
    pub const fn id(&self) -> &'static str {
        match self {
            Self::EmptyIdentity => "empty_identity",
            Self::ReferenceMismatch => "reference_mismatch",
            Self::SensorMismatch => "sensor_mismatch",
            Self::SharedOrUnregisteredPath => "shared_or_unregistered_path",
            Self::ConfigurationMismatch => "configuration_mismatch",
            Self::UnitMismatch => "unit_mismatch",
            Self::CalibrationMismatch => "calibration_mismatch",
            Self::UnknownAttester => "unknown_attester",
            Self::MissingAttestationDigest => "missing_attestation_digest",
            Self::InvalidClockOrder => "invalid_clock_order",
            Self::StaleEvidence => "stale_evidence",
            Self::ExpiredScope => "expired_scope",
            Self::NonFiniteCrossCheck => "non_finite_cross_check",
            Self::CrossCheckOutsideReferenceTolerance => "cross_check_outside_reference_tolerance",
            Self::CurrentWorldNotReferenceValid => "current_world_not_reference_valid",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AdmittedRequalificationReceipt {
    pub receipt_id: String,
    pub reference_digest: String,
    pub sensor_id: String,
    pub source_path_id: String,
    pub configuration_digest: String,
    pub unit: String,
    pub calibration_revision: String,
    pub model_scope_id: String,
    pub valid_until: u64,
}

impl AdmittedRequalificationReceipt {
    pub fn is_current(&self, now: u64) -> bool {
        now <= self.valid_until
    }
}

/// Evaluator-side validation. ActualCalibrationWorld is hidden truth and must
/// never be included in the admitted receipt or the candidate policy export.
/// The attestation digest/allowlist check is a synthetic provenance contract,
/// not cryptographic signature verification.
pub fn admit_requalification_receipt(
    requirements: &RequalificationRequirements,
    receipt: &RawRequalificationReceipt,
    actual_world: ActualCalibrationWorld,
) -> Result<AdmittedRequalificationReceipt, RequalificationRejectReason> {
    if receipt.receipt_id.trim().is_empty()
        || requirements.reference_digest.trim().is_empty()
        || requirements.configuration_digest.trim().is_empty()
        || requirements.sensor_id.trim().is_empty()
        || requirements.unit.trim().is_empty()
    {
        return Err(RequalificationRejectReason::EmptyIdentity);
    }
    if receipt.reference_digest != requirements.reference_digest {
        return Err(RequalificationRejectReason::ReferenceMismatch);
    }
    if receipt.sensor_id != requirements.sensor_id {
        return Err(RequalificationRejectReason::SensorMismatch);
    }
    if receipt.source_path_id == requirements.monitored_path_id
        || !requirements.allowed_independent_paths.contains(&receipt.source_path_id)
    {
        return Err(RequalificationRejectReason::SharedOrUnregisteredPath);
    }
    if receipt.configuration_digest != requirements.configuration_digest {
        return Err(RequalificationRejectReason::ConfigurationMismatch);
    }
    if receipt.unit != requirements.unit {
        return Err(RequalificationRejectReason::UnitMismatch);
    }
    if receipt.calibration_revision != requirements.calibration_revision {
        return Err(RequalificationRejectReason::CalibrationMismatch);
    }
    if !requirements.allowed_attesters.contains(&receipt.attester_id) {
        return Err(RequalificationRejectReason::UnknownAttester);
    }
    if receipt.attestation_digest.trim().is_empty() {
        return Err(RequalificationRejectReason::MissingAttestationDigest);
    }
    if receipt.event_time > receipt.arrival_time || receipt.arrival_time > requirements.now {
        return Err(RequalificationRejectReason::InvalidClockOrder);
    }
    if requirements.now - receipt.event_time > requirements.maximum_measurement_age {
        return Err(RequalificationRejectReason::StaleEvidence);
    }
    if receipt.valid_until < requirements.now {
        return Err(RequalificationRejectReason::ExpiredScope);
    }
    if !receipt.physical_cross_check_value.is_finite()
        || !requirements.expected_reference_check_value.is_finite()
        || !requirements.check_tolerance.is_finite()
        || requirements.check_tolerance < 0.0
    {
        return Err(RequalificationRejectReason::NonFiniteCrossCheck);
    }
    if actual_world != ActualCalibrationWorld::ReferenceValid {
        return Err(RequalificationRejectReason::CurrentWorldNotReferenceValid);
    }
    if (receipt.physical_cross_check_value - requirements.expected_reference_check_value).abs()
        > requirements.check_tolerance
    {
        return Err(RequalificationRejectReason::CrossCheckOutsideReferenceTolerance);
    }

    Ok(AdmittedRequalificationReceipt {
        receipt_id: receipt.receipt_id.clone(),
        reference_digest: receipt.reference_digest.clone(),
        sensor_id: receipt.sensor_id.clone(),
        source_path_id: receipt.source_path_id.clone(),
        configuration_digest: receipt.configuration_digest.clone(),
        unit: receipt.unit.clone(),
        calibration_revision: receipt.calibration_revision.clone(),
        model_scope_id: requirements.reference_model_scope_id.clone(),
        valid_until: receipt.valid_until,
    })
}

/// A detector alarm/non-alarm never removes plausible worlds. Only a current,
/// admitted receipt can narrow the set to its registered calibration family.
pub fn candidate_models_after_evidence(
    models: &[ObservationModel],
    monitor: &MonitorSnapshot,
    receipt: Option<&AdmittedRequalificationReceipt>,
    now: u64,
) -> Result<Vec<ObservationModel>, String> {
    let _monitor_evidence_is_not_a_model_exclusion = (
        monitor.validity,
        monitor.alarm_crossed,
        monitor.log_evidence,
    );
    let Some(receipt) = receipt.filter(|item| item.is_current(now)) else {
        return Ok(models.to_vec());
    };

    let scoped: Vec<ObservationModel> = models
        .iter()
        .filter(|model| model.model_id == receipt.model_scope_id)
        .cloned()
        .collect();
    if scoped.is_empty() {
        return Err("admitted receipt scope does not match any declared model family".to_owned());
    }
    Ok(scoped)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model(id: &str, observation: &str, nominal: f64, fallback: f64) -> ObservationModel {
        ObservationModel {
            model_id: id.to_owned(),
            observation_id: observation.to_owned(),
            actions: vec![
                ActionOutcome { action_id: "nominal_action".to_owned(), service_units: nominal },
                ActionOutcome { action_id: "robust_fallback".to_owned(), service_units: fallback },
                ActionOutcome { action_id: "hold_or_abstain".to_owned(), service_units: 0.0 },
            ],
        }
    }

    fn ambiguous_models() -> Vec<ObservationModel> {
        vec![
            model("model_reference", "residual_unresolved", 100.0, 80.0),
            model("model_observation_shift", "residual_unresolved", 20.0, 80.0),
            model("model_common_cause", "residual_unresolved", 30.0, 80.0),
        ]
    }

    fn contract() -> MonitorContract {
        MonitorContract::default()
    }

    fn event(id: &str, time: u64, value: bool) -> ResidualEvent {
        ResidualEvent {
            event_id: id.to_owned(),
            stream_id: contract().stream_id,
            contract_digest: contract().reference_digest,
            event_time: time,
            arrival_time: time,
            residual_event: value,
            independence_contract_id: contract().independence_contract_id,
            independence_validation: IndependenceValidation::SourceContractValidated,
        }
    }

    fn requirements() -> RequalificationRequirements {
        RequalificationRequirements {
            reference_digest: "sha256-calibration-family-v1".to_owned(),
            sensor_id: "independent-field-calibrator-v1".to_owned(),
            monitored_path_id: "telemetry-gateway-v1".to_owned(),
            allowed_independent_paths: ["field-inspector-direct-v1".to_owned()].into(),
            configuration_digest: "sha256-plant-config-v1".to_owned(),
            unit: "service_units".to_owned(),
            calibration_revision: "calibration-r7".to_owned(),
            allowed_attesters: ["metrology-authority-v1".to_owned()].into(),
            maximum_measurement_age: 3,
            now: 10,
            reference_model_scope_id: "model_reference".to_owned(),
            expected_reference_check_value: 1.0,
            check_tolerance: 0.01,
        }
    }

    fn raw_receipt() -> RawRequalificationReceipt {
        RawRequalificationReceipt {
            receipt_id: "receipt-001".to_owned(),
            reference_digest: "sha256-calibration-family-v1".to_owned(),
            sensor_id: "independent-field-calibrator-v1".to_owned(),
            source_path_id: "field-inspector-direct-v1".to_owned(),
            configuration_digest: "sha256-plant-config-v1".to_owned(),
            unit: "service_units".to_owned(),
            calibration_revision: "calibration-r7".to_owned(),
            attester_id: "metrology-authority-v1".to_owned(),
            attestation_digest: "sha256-attestation-001".to_owned(),
            event_time: 9,
            arrival_time: 9,
            valid_until: 12,
            physical_cross_check_value: 1.0,
        }
    }

    #[test]
    fn bernoulli_likelihood_ratio_has_unit_expectation_under_declared_null() {
        let contract = contract();
        let expectation = contract.null_expected_one_step_factor();
        assert!((expectation - 1.0).abs() < 1.0e-12);
        assert!((contract.log_likelihood_increment(true).exp() - 5.0).abs() < 1.0e-12);
        assert!((contract.log_likelihood_increment(false).exp() - (5.0 / 9.0)).abs() < 1.0e-12);
    }

    #[test]
    fn three_successive_residual_events_cross_the_one_percent_anytime_boundary() {
        let mut monitor = AnytimeBernoulliMonitor::new(contract()).unwrap();
        let first = monitor.process(&event("e1", 1, true));
        let second = monitor.process(&event("e2", 2, true));
        let third = monitor.process(&event("e3", 3, true));
        assert!(first.accepted && second.accepted && third.accepted);
        assert!(!first.snapshot.alarm_crossed && !second.snapshot.alarm_crossed);
        assert!(third.snapshot.alarm_crossed);
        assert!((third.snapshot.log_evidence.exp() - 125.0).abs() < 1.0e-9);
        assert!(third.snapshot.anytime_iid_bound_applicable);
    }

    #[test]
    fn valid_non_alarm_and_alarm_both_retain_the_same_plausible_model_set() {
        let mut monitor = AnytimeBernoulliMonitor::new(contract()).unwrap();
        let non_alarm = monitor.process(&event("n1", 1, false)).snapshot;
        let alarm = {
            let mut separate = AnytimeBernoulliMonitor::new(contract()).unwrap();
            separate.process(&event("a1", 1, true));
            separate.process(&event("a2", 2, true));
            separate.process(&event("a3", 3, true)).snapshot
        };
        let before = candidate_models_after_evidence(&ambiguous_models(), &non_alarm, None, 3).unwrap();
        let after = candidate_models_after_evidence(&ambiguous_models(), &alarm, None, 3).unwrap();
        assert_eq!(before.len(), 3);
        assert_eq!(after.len(), 3);
        assert_eq!(before, after);
    }

    #[test]
    fn invalid_dependence_contract_revokes_anytime_guarantee_and_freezes_watermark() {
        let mut monitor = AnytimeBernoulliMonitor::new(contract()).unwrap();
        let mut bad = event("dependent", 1, true);
        bad.independence_validation = IndependenceValidation::Invalid;
        let step = monitor.process(&bad);
        assert!(!step.accepted);
        assert_eq!(step.rejected_reason, Some(MonitorRejectReason::IndependenceNotValidated));
        assert_eq!(step.snapshot.validity, MonitorValidity::Invalid);
        assert!(!step.snapshot.anytime_iid_bound_applicable);
        assert_eq!(step.snapshot.event_time_watermark, None);
        assert_eq!(step.snapshot.processed_events, 0);
    }

    #[test]
    fn delayed_out_of_order_or_duplicate_events_fail_closed_without_regressing_watermark() {
        let mut monitor = AnytimeBernoulliMonitor::new(contract()).unwrap();
        monitor.process(&event("e1", 1, false));
        monitor.process(&event("e3", 3, false));
        let bad = monitor.process(&event("e2", 2, true));
        assert_eq!(bad.rejected_reason, Some(MonitorRejectReason::EventTimeNotIncreasing));
        assert_eq!(bad.snapshot.event_time_watermark, Some(3));
        assert!(!bad.snapshot.anytime_iid_bound_applicable);

        let mut duplicate = AnytimeBernoulliMonitor::new(contract()).unwrap();
        duplicate.process(&event("same", 1, false));
        let repeated = duplicate.process(&event("same", 2, true));
        assert_eq!(repeated.rejected_reason, Some(MonitorRejectReason::DuplicateEventId));
    }

    #[test]
    fn late_event_invalidates_the_reference_contract() {
        let mut monitor = AnytimeBernoulliMonitor::new(contract()).unwrap();
        let mut late = event("late", 1, true);
        late.arrival_time = 4;
        let step = monitor.process(&late);
        assert_eq!(step.rejected_reason, Some(MonitorRejectReason::EventOutsideLatenessBudget));
        assert_eq!(step.snapshot.validity, MonitorValidity::Invalid);
    }

    #[test]
    fn non_alarm_does_not_allow_nominal_action_when_shifted_worlds_remain_plausible() {
        let report = candidate_gate_report(&ambiguous_models(), 70.0).unwrap();
        assert!(report.all_observation_cells_feasible);
        assert_eq!(report.cells[0].selected_action_id.as_deref(), Some("robust_fallback"));
        assert_eq!(report.cells[0].common_safe_action_ids, vec!["robust_fallback"]);
    }

    #[test]
    fn candidate_gate_output_does_not_contain_evaluator_model_ids() {
        let report = candidate_gate_report(&ambiguous_models(), 70.0).unwrap();
        assert_eq!(report.cells.len(), 1);
        assert_eq!(report.cells[0].observation_id, "residual_unresolved");
        assert!(!report.cells[0].disposition.contains("model_"));
        let audit = evaluator_gate_audit(&ambiguous_models(), 70.0).unwrap();
        assert_eq!(audit.cells[0].model_ids.len(), 3);
    }

    #[test]
    fn observation_structure_classifies_automatic_remediable_and_hard_cases() {
        let automatic = classify_deployment(&ambiguous_models(), 70.0, None, 10.0).unwrap();
        assert_eq!(automatic.regime, DeploymentRegime::AutomaticallyDeployable);

        let needs_measurement = vec![
            model("model_reference", "unresolved", 100.0, 60.0),
            model("model_shift", "unresolved", 20.0, 100.0),
        ];
        let refinement = ObservationRefinement {
            refinement_id: "independent_inspection-v1".to_owned(),
            protocol_digest: "sha256-independent-inspection-protocol".to_owned(),
            energy_cost_service_units: 2.0,
            latency_ticks: 1,
            predictions: vec![
                RefinementPrediction {
                    model_id: "model_reference".to_owned(),
                    observation_id: "nominal_observation".to_owned(),
                    latency_service_loss_per_tick: 1.0,
                },
                RefinementPrediction {
                    model_id: "model_shift".to_owned(),
                    observation_id: "shifted_observation".to_owned(),
                    latency_service_loss_per_tick: 1.0,
                },
            ],
        };
        let remediable = classify_deployment(&needs_measurement, 70.0, Some(&refinement), 5.0).unwrap();
        assert_eq!(remediable.regime, DeploymentRegime::Remediable);
        assert!(remediable.refined_candidate_report.unwrap().all_observation_cells_feasible);

        let no_refinement = classify_deployment(&needs_measurement, 70.0, None, 5.0).unwrap();
        assert_eq!(no_refinement.regime, DeploymentRegime::HardNonDeployable);

        let same_observation = ObservationRefinement {
            predictions: vec![
                RefinementPrediction {
                    model_id: "model_reference".to_owned(),
                    observation_id: "still_unresolved".to_owned(),
                    latency_service_loss_per_tick: 1.0,
                },
                RefinementPrediction {
                    model_id: "model_shift".to_owned(),
                    observation_id: "still_unresolved".to_owned(),
                    latency_service_loss_per_tick: 1.0,
                },
            ],
            ..refinement.clone()
        };
        let still_hard = classify_deployment(&needs_measurement, 70.0, Some(&same_observation), 5.0).unwrap();
        assert_eq!(still_hard.regime, DeploymentRegime::HardNonDeployable);

        let over_budget = classify_deployment(&needs_measurement, 70.0, Some(&refinement), 1.0).unwrap();
        assert_eq!(over_budget.regime, DeploymentRegime::HardNonDeployable);
    }

    #[test]
    fn a_valid_independent_receipt_is_admitted_only_in_the_reference_world() {
        let requirements = requirements();
        let raw = raw_receipt();
        let admitted = admit_requalification_receipt(
            &requirements,
            &raw,
            ActualCalibrationWorld::ReferenceValid,
        ).unwrap();
        assert_eq!(admitted.model_scope_id, "model_reference");
        assert!(admitted.is_current(requirements.now));

        let rejected = admit_requalification_receipt(
            &requirements,
            &raw,
            ActualCalibrationWorld::ObservationShifted,
        );
        assert_eq!(rejected, Err(RequalificationRejectReason::CurrentWorldNotReferenceValid));

        let rejected_common = admit_requalification_receipt(
            &requirements,
            &raw,
            ActualCalibrationWorld::CommonCauseDependent,
        );
        assert_eq!(rejected_common, Err(RequalificationRejectReason::CurrentWorldNotReferenceValid));
    }

    #[test]
    fn stale_shared_path_untrusted_attester_and_mismatched_scope_receipts_are_rejected() {
        let requirements = requirements();

        let mut stale = raw_receipt();
        stale.event_time = 1;
        assert_eq!(
            admit_requalification_receipt(&requirements, &stale, ActualCalibrationWorld::ReferenceValid),
            Err(RequalificationRejectReason::StaleEvidence)
        );

        let mut shared = raw_receipt();
        shared.source_path_id = requirements.monitored_path_id.clone();
        assert_eq!(
            admit_requalification_receipt(&requirements, &shared, ActualCalibrationWorld::ReferenceValid),
            Err(RequalificationRejectReason::SharedOrUnregisteredPath)
        );

        let mut unknown = raw_receipt();
        unknown.attester_id = "self-attested-agent".to_owned();
        assert_eq!(
            admit_requalification_receipt(&requirements, &unknown, ActualCalibrationWorld::ReferenceValid),
            Err(RequalificationRejectReason::UnknownAttester)
        );

        let mut config_mismatch = raw_receipt();
        config_mismatch.configuration_digest = "wrong-config".to_owned();
        assert_eq!(
            admit_requalification_receipt(&requirements, &config_mismatch, ActualCalibrationWorld::ReferenceValid),
            Err(RequalificationRejectReason::ConfigurationMismatch)
        );
    }

    #[test]
    fn only_a_current_admitted_receipt_can_narrow_the_candidate_model_set() {
        let models = ambiguous_models();
        let mut monitor = AnytimeBernoulliMonitor::new(contract()).unwrap();
        let snapshot = monitor.process(&event("e1", 1, false)).snapshot;
        let raw = raw_receipt();
        let requirements = requirements();
        let admitted = admit_requalification_receipt(
            &requirements,
            &raw,
            ActualCalibrationWorld::ReferenceValid,
        ).unwrap();

        let narrowed = candidate_models_after_evidence(&models, &snapshot, Some(&admitted), 10).unwrap();
        assert_eq!(narrowed.len(), 1);
        assert_eq!(narrowed[0].model_id, "model_reference");
        let stale = candidate_models_after_evidence(&models, &snapshot, Some(&admitted), 13).unwrap();
        assert_eq!(stale.len(), 3);
    }

    #[test]
    fn no_safe_fallback_is_explicit_abstention_not_a_safe_claim() {
        let models = vec![
            model("m1", "same", 100.0, 40.0),
            model("m2", "same", 20.0, 40.0),
        ];
        let report = candidate_gate_report(&models, 70.0).unwrap();
        assert!(!report.all_observation_cells_feasible);
        assert!(report.cells[0].selected_action_id.is_none());
        assert_eq!(report.cells[0].disposition, "no_common_safe_action_abstain");
    }

    #[test]
    fn detector_non_alarm_and_alarm_do_not_self_authorize_requalification() {
        let mut monitor = AnytimeBernoulliMonitor::new(contract()).unwrap();
        let non_alarm = monitor.process(&event("n", 1, false)).snapshot;
        let mut alarm_monitor = AnytimeBernoulliMonitor::new(contract()).unwrap();
        alarm_monitor.process(&event("a1", 1, true));
        alarm_monitor.process(&event("a2", 2, true));
        let alarm = alarm_monitor.process(&event("a3", 3, true)).snapshot;

        let models = ambiguous_models();
        let no_receipt_before = candidate_models_after_evidence(&models, &non_alarm, None, 3).unwrap();
        let no_receipt_after = candidate_models_after_evidence(&models, &alarm, None, 3).unwrap();
        assert_eq!(no_receipt_before, no_receipt_after);
        assert_eq!(no_receipt_after.len(), 3);
    }
}
