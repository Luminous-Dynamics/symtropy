//! Typed paired clean/perturbed deployment-fault campaign contract.
//!
//! The validator checks finite-record consistency and matched experimental
//! context. It does not establish that a fault distribution is representative,
//! authenticate sensor truth, prove safety or replace an independent safety case.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const MAX_PAIRS: usize = 100_000;
pub const MAX_METRICS: usize = 256;
pub const MAX_FAULT_EVENTS_PER_RUN: usize = 1_000_000;
pub const SHA256_HEX_LENGTH: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RunDisposition {
    Completed,
    Failed,
    Incomplete,
    Censored,
    TimedOut,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MetricDirection {
    HigherIsBetter,
    LowerIsBetter,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MetricDefinition {
    pub metric_id: String,
    pub direction: MetricDirection,
    pub unit: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MatchedContext {
    pub benchmark_id: String,
    pub evaluator_revision: String,
    pub scenario_id: String,
    pub route_id: String,
    pub map_revision: String,
    pub environment_revision: String,
    pub simulator_revision: String,
    pub model_code_revision: String,
    pub checkpoint_sha256: String,
    pub recurrent_state_policy_sha256: String,
    pub actor_policy_set_sha256: String,
    pub action_schema_sha256: String,
    pub observation_schema_sha256: String,
    pub dataset_split_sha256: String,
    pub environment_seed_manifest_sha256: String,
    pub actor_random_stream_manifest_sha256: String,
    pub exogenous_randomness_manifest_sha256: String,
    pub action_space_id: String,
    pub inference_hardware_id: String,
    pub inference_power_profile: String,
    pub inference_precision: String,
    pub runtime_config_sha256: String,
    pub input_privileges: Vec<String>,
    pub teacher_privileges: Vec<String>,
    pub trainable_parameters: u64,
    pub training_simulator_transitions: u64,
    pub training_gpu_seconds: u64,
    pub training_energy_milliwatt_hours: u64,
    pub human_intervention_seconds: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MaskMode {
    FixedPerEpisode,
    ResampledPerFrame,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum QueueMode {
    Fifo,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(tag = "family", rename_all = "snake_case", deny_unknown_fields)]
pub enum FaultSpec {
    CameraBurstDrop {
        duration_ticks: u64,
        onset_tick: u64,
        burst_probability: f64,
        schedule_seed_sha256: String,
    },
    CameraPartialOcclusion {
        occlusion_fraction: f64,
        mask_mode: MaskMode,
        onset_tick: u64,
        duration_ticks: u64,
        schedule_seed_sha256: String,
    },
    GpsLocalizationNoise {
        sigma_m: f64,
        onset_tick: u64,
        duration_ticks: u64,
        noise_seed_sha256: String,
    },
    SpeedEstimateMultiplicativeNoise {
        multiplier_mean: f64,
        multiplier_std_dev: f64,
        onset_tick: u64,
        duration_ticks: u64,
        noise_seed_sha256: String,
    },
    InferenceControlDelay {
        delay_ms: u64,
        onset_tick: u64,
        duration_ticks: u64,
        queue_mode: QueueMode,
        schedule_seed_sha256: String,
    },
}

impl FaultSpec {
    pub fn family_id(&self) -> &'static str {
        match self {
            Self::CameraBurstDrop { .. } => "camera_burst_drop",
            Self::CameraPartialOcclusion { .. } => "camera_partial_occlusion",
            Self::GpsLocalizationNoise { .. } => "gps_localization_noise",
            Self::SpeedEstimateMultiplicativeNoise { .. } => {
                "speed_estimate_multiplicative_noise"
            }
            Self::InferenceControlDelay { .. } => "inference_control_delay",
        }
    }

    fn onset_tick(&self) -> u64 {
        match self {
            Self::CameraBurstDrop { onset_tick, .. }
            | Self::CameraPartialOcclusion { onset_tick, .. }
            | Self::GpsLocalizationNoise { onset_tick, .. }
            | Self::SpeedEstimateMultiplicativeNoise { onset_tick, .. }
            | Self::InferenceControlDelay { onset_tick, .. } => *onset_tick,
        }
    }

    fn duration_ticks(&self) -> u64 {
        match self {
            Self::CameraBurstDrop { duration_ticks, .. }
            | Self::CameraPartialOcclusion { duration_ticks, .. }
            | Self::GpsLocalizationNoise { duration_ticks, .. }
            | Self::SpeedEstimateMultiplicativeNoise { duration_ticks, .. }
            | Self::InferenceControlDelay { duration_ticks, .. } => *duration_ticks,
        }
    }

    fn schedule_digest(&self) -> &str {
        match self {
            Self::CameraBurstDrop {
                schedule_seed_sha256,
                ..
            }
            | Self::CameraPartialOcclusion {
                schedule_seed_sha256,
                ..
            }
            | Self::InferenceControlDelay {
                schedule_seed_sha256,
                ..
            } => schedule_seed_sha256,
            Self::GpsLocalizationNoise {
                noise_seed_sha256, ..
            }
            | Self::SpeedEstimateMultiplicativeNoise {
                noise_seed_sha256, ..
            } => noise_seed_sha256,
        }
    }

    fn validate(&self) -> Vec<Finding> {
        let mut findings = Vec::new();
        if self.onset_tick() == 0 || self.duration_ticks() == 0 {
            findings.push(Finding::InvalidFaultWindow);
        }
        if !is_sha256_hex(self.schedule_digest()) {
            findings.push(Finding::InvalidFaultScheduleDigest);
        }
        match self {
            Self::CameraBurstDrop {
                duration_ticks,
                burst_probability,
                ..
            } => {
                if !matches!(*duration_ticks, 20 | 60) {
                    findings.push(Finding::UnsupportedFaultSeverity);
                }
                if !burst_probability.is_finite()
                    || !(0.0..=1.0).contains(burst_probability)
                {
                    findings.push(Finding::InvalidFaultParameter);
                }
            }
            Self::CameraPartialOcclusion {
                occlusion_fraction,
                ..
            } => {
                if !occlusion_fraction.is_finite()
                    || !matches!(*occlusion_fraction, 0.5 | 0.8)
                {
                    findings.push(Finding::UnsupportedFaultSeverity);
                }
            }
            Self::GpsLocalizationNoise { sigma_m, .. } => {
                if !sigma_m.is_finite() || !matches!(*sigma_m, 5.0 | 15.0) {
                    findings.push(Finding::UnsupportedFaultSeverity);
                }
            }
            Self::SpeedEstimateMultiplicativeNoise {
                multiplier_mean,
                multiplier_std_dev,
                ..
            } => {
                if !multiplier_mean.is_finite()
                    || !matches!(*multiplier_mean, 0.2 | 0.5)
                    || !multiplier_std_dev.is_finite()
                    || (*multiplier_std_dev - 0.2).abs() > f64::EPSILON
                {
                    findings.push(Finding::UnsupportedFaultSeverity);
                }
            }
            Self::InferenceControlDelay { delay_ms, .. } => {
                if !matches!(*delay_ms, 100 | 200 | 500) {
                    findings.push(Finding::UnsupportedFaultSeverity);
                }
            }
        }
        findings
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FaultEventKind {
    Injected,
    ObservedMissingFrame,
    ActionDelayed,
    FaultRestored,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FaultEvent {
    pub event_id: String,
    pub fault_family: String,
    pub kind: FaultEventKind,
    pub tick: u64,
    pub affected_channel: String,
    pub sampled_value_sha256: String,
    pub evidence_sha256: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryDisposition {
    Recovered,
    NotRecovered,
    NotApplicable,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RecoveryAssessment {
    pub disposition: RecoveryDisposition,
    pub fault_onset_tick: u64,
    pub fault_restored_tick: Option<u64>,
    pub stable_from_tick: Option<u64>,
    pub stability_window_ticks: u64,
    pub stability_envelope_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MetricEstimate {
    pub value: Option<f64>,
    pub lower_95: Option<f64>,
    pub upper_95: Option<f64>,
    pub samples: u64,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RunRecord {
    pub run_id: String,
    pub context: MatchedContext,
    pub disposition: RunDisposition,
    pub expected_episodes: u64,
    pub completed_episodes: u64,
    pub failed_episodes: u64,
    pub incomplete_episodes: u64,
    pub censored_episodes: u64,
    pub timed_out_episodes: u64,
    pub observation_trace_sha256: String,
    pub metrics: BTreeMap<String, MetricEstimate>,
    pub fault_events: Vec<FaultEvent>,
    pub recovery: Option<RecoveryAssessment>,
    pub fault: Option<FaultSpec>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PairedRun {
    pub pair_id: String,
    pub clean: RunRecord,
    pub perturbed: RunRecord,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FaultCampaign {
    pub schema_version: u32,
    pub campaign_id: String,
    pub benchmark_id: String,
    pub evaluator_revision: String,
    pub expected_pair_ids: Vec<String>,
    pub metric_definitions: Vec<MetricDefinition>,
    pub pairs: Vec<PairedRun>,
}

impl FaultCampaign {
    pub fn from_json(input: &str) -> Result<Self, String> {
        serde_json::from_str(input).map_err(|error| format!("fault campaign JSON: {error}"))
    }

    pub fn audit(&self) -> AuditReport {
        let mut findings = Vec::new();
        if self.schema_version != 1 {
            findings.push(Finding::UnsupportedSchemaVersion);
        }
        if !nonempty(&self.campaign_id)
            || !nonempty(&self.benchmark_id)
            || !nonempty(&self.evaluator_revision)
        {
            findings.push(Finding::EmptyCampaignIdentity);
        }
        if self.expected_pair_ids.len() > MAX_PAIRS || self.pairs.len() > MAX_PAIRS {
            findings.push(Finding::BoundExceeded("pairs"));
        }
        if self.metric_definitions.is_empty() || self.metric_definitions.len() > MAX_METRICS {
            findings.push(Finding::InvalidMetricDefinitionCount);
        }

        let expected = id_set(self.expected_pair_ids.iter().cloned());
        let observed = id_set(self.pairs.iter().map(|pair| pair.pair_id.clone()));
        if duplicate(&self.expected_pair_ids)
            || self.expected_pair_ids.iter().any(|id| !nonempty(id))
        {
            findings.push(Finding::InvalidExpectedPairManifest);
        }
        if duplicate(&self.pairs.iter().map(|pair| pair.pair_id.clone()).collect::<Vec<_>>()) {
            findings.push(Finding::DuplicatePair);
        }
        if expected != observed {
            findings.push(Finding::PairDenominatorMismatch);
        }

        let mut metric_ids = Vec::new();
        for definition in &self.metric_definitions {
            metric_ids.push(definition.metric_id.clone());
            if !nonempty(&definition.metric_id) || !nonempty(&definition.unit) {
                findings.push(Finding::InvalidMetricDefinition);
            }
        }
        if duplicate(&metric_ids) {
            findings.push(Finding::DuplicateMetricDefinition);
        }
        metric_ids.sort();

        for pair in &self.pairs {
            self.audit_pair(pair, &metric_ids, &mut findings);
        }
        findings.sort();
        findings.dedup();

        let disposition = if findings.is_empty() {
            AuditDisposition::Valid
        } else {
            AuditDisposition::Rejected
        };
        let report_pairs = self
            .pairs
            .iter()
            .map(|pair| self.build_pair_report(pair))
            .collect();
        AuditReport {
            campaign_id: self.campaign_id.clone(),
            disposition,
            expected_pair_count: self.expected_pair_ids.len(),
            observed_pair_count: self.pairs.len(),
            metric_count: self.metric_definitions.len(),
            findings,
            pair_results: report_pairs,
            claim_ceiling: "Finite typed-record consistency and paired metric arithmetic only. This audit does not prove fault prevalence, physical causality, sensor authenticity, safe fallback, public-road safety or roadworthiness.".to_string(),
        }
    }

    fn audit_pair(
        &self,
        pair: &PairedRun,
        metric_ids: &[String],
        findings: &mut Vec<Finding>,
    ) {
        if !nonempty(&pair.pair_id)
            || !nonempty(&pair.clean.run_id)
            || !nonempty(&pair.perturbed.run_id)
            || pair.clean.run_id == pair.perturbed.run_id
        {
            findings.push(Finding::InvalidPairIdentity);
        }
        if pair.clean.context != pair.perturbed.context {
            findings.push(Finding::MatchedContextMismatch);
        }
        if pair.clean.context.benchmark_id != self.benchmark_id
            || pair.perturbed.context.benchmark_id != self.benchmark_id
            || pair.clean.context.evaluator_revision != self.evaluator_revision
            || pair.perturbed.context.evaluator_revision != self.evaluator_revision
        {
            findings.push(Finding::CampaignContextMismatch);
        }
        validate_context(&pair.clean.context, findings);
        validate_context(&pair.perturbed.context, findings);

        if pair.clean.fault.is_some() || !pair.clean.fault_events.is_empty() {
            findings.push(Finding::CleanRunContainsFault);
        }
        let Some(fault) = pair.perturbed.fault.as_ref() else {
            findings.push(Finding::PerturbedRunMissingFaultSpec);
            self.audit_run(&pair.clean, metric_ids, false, findings);
            self.audit_run(&pair.perturbed, metric_ids, false, findings);
            return;
        };
        findings.extend(fault.validate());
        self.audit_run(&pair.clean, metric_ids, false, findings);
        self.audit_run(&pair.perturbed, metric_ids, true, findings);

        if pair.perturbed.fault_events.len() > MAX_FAULT_EVENTS_PER_RUN {
            findings.push(Finding::BoundExceeded("fault_events_per_run"));
        }
        let family = fault.family_id();
        let matching_events = pair
            .perturbed
            .fault_events
            .iter()
            .filter(|event| event.fault_family == family)
            .collect::<Vec<_>>();
        if matching_events.is_empty() {
            findings.push(Finding::MissingFaultEvent);
        }
        let mut event_ids = Vec::new();
        for event in &pair.perturbed.fault_events {
            event_ids.push(event.event_id.clone());
            if !nonempty(&event.event_id) || !nonempty(&event.affected_channel) {
                findings.push(Finding::InvalidFaultEventIdentity);
            }
            let schedule_end = fault.onset_tick().saturating_add(fault.duration_ticks());
            let event_is_in_window = if event.kind == FaultEventKind::FaultRestored {
                event.tick >= fault.onset_tick() && event.tick <= schedule_end
            } else {
                event.tick >= fault.onset_tick() && event.tick < schedule_end
            };
            if event.fault_family != family
                || !event_is_in_window
                || !is_sha256_hex(&event.sampled_value_sha256)
                || !is_sha256_hex(&event.evidence_sha256)
            {
                findings.push(Finding::FaultEventOutsideDeclaredSchedule);
            }
        }
        if duplicate(&event_ids) {
            findings.push(Finding::DuplicateFaultEvent);
        }
        if let FaultSpec::CameraBurstDrop { duration_ticks, .. } = fault {
            let missing_ticks = pair
                .perturbed
                .fault_events
                .iter()
                .filter(|event| {
                    event.fault_family == family
                        && event.kind == FaultEventKind::ObservedMissingFrame
                })
                .map(|event| event.tick)
                .collect::<BTreeSet<_>>();
            if missing_ticks.len() < *duration_ticks as usize {
                findings.push(Finding::CameraDropDenominatorMismatch);
            }
        }
        self.audit_recovery(fault, &pair.perturbed, findings);
    }

    fn audit_run(
        &self,
        run: &RunRecord,
        metric_ids: &[String],
        is_perturbed: bool,
        findings: &mut Vec<Finding>,
    ) {
        if !nonempty(&run.run_id) || !is_sha256_hex(&run.observation_trace_sha256) {
            findings.push(Finding::InvalidRunIdentity);
        }
        let dispositioned = run
            .completed_episodes
            .checked_add(run.failed_episodes)
            .and_then(|value| value.checked_add(run.incomplete_episodes))
            .and_then(|value| value.checked_add(run.censored_episodes))
            .and_then(|value| value.checked_add(run.timed_out_episodes));
        match dispositioned {
            Some(value) if value == run.expected_episodes => {}
            _ => findings.push(Finding::EpisodeDenominatorMismatch),
        }
        if run.expected_episodes == 0 {
            findings.push(Finding::ZeroEpisodeDenominator);
        }
        if run.disposition == RunDisposition::Completed && run.completed_episodes == 0 {
            findings.push(Finding::CompletedRunHasNoCompletedEpisodes);
        }
        if run.disposition != RunDisposition::Completed && run.completed_episodes == run.expected_episodes {
            findings.push(Finding::RunDispositionMismatch);
        }
        let keys = run.metrics.keys().cloned().collect::<Vec<_>>();
        if keys != metric_ids {
            findings.push(Finding::MetricSetMismatch);
        }
        for (metric_id, metric) in &run.metrics {
            if let Some(value) = metric.value {
                if !value.is_finite() {
                    findings.push(Finding::InvalidMetricValue);
                }
            }
            if let (Some(lower), Some(value), Some(upper)) =
                (metric.lower_95, metric.value, metric.upper_95)
            {
                if !lower.is_finite()
                    || !upper.is_finite()
                    || lower > value
                    || value > upper
                {
                    findings.push(Finding::InvalidMetricInterval);
                }
            } else if metric.lower_95.is_some() || metric.upper_95.is_some() {
                findings.push(Finding::IncompleteMetricInterval);
            }
            if run.disposition == RunDisposition::Completed
                && (metric.value.is_none() || metric.samples == 0)
            {
                findings.push(Finding::CompletedRunMetricMissing);
            }
        }
        if is_perturbed && run.fault.is_none() {
            findings.push(Finding::PerturbedRunMissingFaultSpec);
        }
    }

    fn audit_recovery(
        &self,
        fault: &FaultSpec,
        run: &RunRecord,
        findings: &mut Vec<Finding>,
    ) {
        let Some(recovery) = run.recovery.as_ref() else {
            findings.push(Finding::MissingRecoveryAssessment);
            return;
        };
        if recovery.fault_onset_tick != fault.onset_tick()
            || !is_sha256_hex(&recovery.stability_envelope_sha256)
        {
            findings.push(Finding::InvalidRecoveryAssessment);
        }
        match recovery.disposition {
            RecoveryDisposition::Recovered => {
                let (Some(restored), Some(stable)) =
                    (recovery.fault_restored_tick, recovery.stable_from_tick)
                else {
                    findings.push(Finding::InvalidRecoveryAssessment);
                    return;
                };
                if restored < recovery.fault_onset_tick
                    || stable < restored
                    || recovery.stability_window_ticks == 0
                {
                    findings.push(Finding::InvalidRecoveryAssessment);
                }
            }
            RecoveryDisposition::NotRecovered => {
                if recovery.stable_from_tick.is_some() {
                    findings.push(Finding::InvalidRecoveryAssessment);
                }
            }
            RecoveryDisposition::NotApplicable => {
                if recovery.stable_from_tick.is_some()
                    || recovery.fault_restored_tick.is_some()
                {
                    findings.push(Finding::InvalidRecoveryAssessment);
                }
            }
        }
    }

    fn build_pair_report(&self, pair: &PairedRun) -> PairReport {
        let fault_family = pair
            .perturbed
            .fault
            .as_ref()
            .map(|fault| fault.family_id().to_string())
            .unwrap_or_else(|| "missing_fault_spec".to_string());
        let metric_results = self
            .metric_definitions
            .iter()
            .map(|definition| {
                let clean = pair
                    .clean
                    .metrics
                    .get(&definition.metric_id)
                    .and_then(|estimate| estimate.value);
                let perturbed = pair
                    .perturbed
                    .metrics
                    .get(&definition.metric_id)
                    .and_then(|estimate| estimate.value);
                let relative_degradation = match (clean, perturbed) {
                    (Some(baseline), Some(value)) if baseline.abs() > f64::EPSILON => {
                        Some(match definition.direction {
                            MetricDirection::HigherIsBetter => (baseline - value) / baseline.abs(),
                            MetricDirection::LowerIsBetter => (value - baseline) / baseline.abs(),
                        })
                    }
                    _ => None,
                };
                MetricPairResult {
                    metric_id: definition.metric_id.clone(),
                    direction: definition.direction,
                    unit: definition.unit.clone(),
                    clean_value: clean,
                    perturbed_value: perturbed,
                    relative_degradation,
                    relative_degradation_unavailable_reason: if clean.is_none() || perturbed.is_none() {
                        Some("missing_clean_or_perturbed_metric".to_string())
                    } else if clean.is_some_and(|value| value.abs() <= f64::EPSILON) {
                        Some("clean_baseline_is_zero".to_string())
                    } else {
                        None
                    },
                }
            })
            .collect();
        PairReport {
            pair_id: pair.pair_id.clone(),
            fault_family,
            clean_disposition: pair.clean.disposition,
            perturbed_disposition: pair.perturbed.disposition,
            metric_results,
            recovery: pair.perturbed.recovery.clone(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditDisposition {
    Valid,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Finding {
    UnsupportedSchemaVersion,
    EmptyCampaignIdentity,
    BoundExceeded(&'static str),
    InvalidMetricDefinitionCount,
    InvalidExpectedPairManifest,
    DuplicatePair,
    PairDenominatorMismatch,
    InvalidMetricDefinition,
    DuplicateMetricDefinition,
    InvalidPairIdentity,
    MatchedContextMismatch,
    CampaignContextMismatch,
    InvalidContextIdentity,
    InvalidContextDigest,
    InvalidPrivilegeManifest,
    CleanRunContainsFault,
    PerturbedRunMissingFaultSpec,
    InvalidFaultWindow,
    InvalidFaultScheduleDigest,
    InvalidFaultParameter,
    UnsupportedFaultSeverity,
    InvalidRunIdentity,
    EpisodeDenominatorMismatch,
    ZeroEpisodeDenominator,
    CompletedRunHasNoCompletedEpisodes,
    RunDispositionMismatch,
    MetricSetMismatch,
    InvalidMetricValue,
    InvalidMetricInterval,
    IncompleteMetricInterval,
    CompletedRunMetricMissing,
    InvalidFaultEventIdentity,
    FaultEventOutsideDeclaredSchedule,
    DuplicateFaultEvent,
    MissingFaultEvent,
    CameraDropDenominatorMismatch,
    MissingRecoveryAssessment,
    InvalidRecoveryAssessment,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MetricPairResult {
    pub metric_id: String,
    pub direction: MetricDirection,
    pub unit: String,
    pub clean_value: Option<f64>,
    pub perturbed_value: Option<f64>,
    pub relative_degradation: Option<f64>,
    pub relative_degradation_unavailable_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PairReport {
    pub pair_id: String,
    pub fault_family: String,
    pub clean_disposition: RunDisposition,
    pub perturbed_disposition: RunDisposition,
    pub metric_results: Vec<MetricPairResult>,
    pub recovery: Option<RecoveryAssessment>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AuditReport {
    pub campaign_id: String,
    pub disposition: AuditDisposition,
    pub expected_pair_count: usize,
    pub observed_pair_count: usize,
    pub metric_count: usize,
    pub findings: Vec<Finding>,
    pub pair_results: Vec<PairReport>,
    pub claim_ceiling: String,
}

fn validate_context(context: &MatchedContext, findings: &mut Vec<Finding>) {
    let identities = [
        context.benchmark_id.as_str(),
        context.evaluator_revision.as_str(),
        context.scenario_id.as_str(),
        context.route_id.as_str(),
        context.map_revision.as_str(),
        context.environment_revision.as_str(),
        context.simulator_revision.as_str(),
        context.model_code_revision.as_str(),
        context.action_space_id.as_str(),
        context.inference_hardware_id.as_str(),
        context.inference_power_profile.as_str(),
        context.inference_precision.as_str(),
    ];
    if identities.iter().any(|value| !nonempty(value)) {
        findings.push(Finding::InvalidContextIdentity);
    }
    let digests = [
        context.checkpoint_sha256.as_str(),
        context.recurrent_state_policy_sha256.as_str(),
        context.actor_policy_set_sha256.as_str(),
        context.action_schema_sha256.as_str(),
        context.observation_schema_sha256.as_str(),
        context.dataset_split_sha256.as_str(),
        context.environment_seed_manifest_sha256.as_str(),
        context.actor_random_stream_manifest_sha256.as_str(),
        context.exogenous_randomness_manifest_sha256.as_str(),
        context.runtime_config_sha256.as_str(),
    ];
    if digests.iter().any(|digest| !is_sha256_hex(digest)) {
        findings.push(Finding::InvalidContextDigest);
    }
    if duplicate(&context.input_privileges)
        || duplicate(&context.teacher_privileges)
        || !strictly_sorted(&context.input_privileges)
        || !strictly_sorted(&context.teacher_privileges)
    {
        findings.push(Finding::InvalidPrivilegeManifest);
    }
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn duplicate(values: &[String]) -> bool {
    let mut copy = values.to_vec();
    copy.sort();
    copy.windows(2).any(|pair| pair[0] == pair[1])
}

fn strictly_sorted(values: &[String]) -> bool {
    values.windows(2).all(|pair| pair[0] < pair[1])
}

fn id_set(values: impl IntoIterator<Item = String>) -> BTreeSet<String> {
    values.into_iter().collect()
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == SHA256_HEX_LENGTH
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

pub fn relative_degradation(
    clean: f64,
    perturbed: f64,
    direction: MetricDirection,
) -> Option<f64> {
    if !clean.is_finite() || !perturbed.is_finite() || clean.abs() <= f64::EPSILON {
        return None;
    }
    Some(match direction {
        MetricDirection::HigherIsBetter => (clean - perturbed) / clean.abs(),
        MetricDirection::LowerIsBetter => (perturbed - clean) / clean.abs(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(byte: char) -> String {
        std::iter::repeat_n(byte, SHA256_HEX_LENGTH).collect()
    }

    fn context() -> MatchedContext {
        MatchedContext {
            benchmark_id: "bench2drive-robust-v1".into(),
            evaluator_revision: "robust-eval-v1".into(),
            scenario_id: "scenario-crossing-pedestrian".into(),
            route_id: "route-001".into(),
            map_revision: "map-v1".into(),
            environment_revision: "env-v1".into(),
            simulator_revision: "sim-v1".into(),
            model_code_revision: "model-commit-1".into(),
            checkpoint_sha256: digest('a'),
            recurrent_state_policy_sha256: digest('b'),
            actor_policy_set_sha256: digest('c'),
            action_schema_sha256: digest('d'),
            observation_schema_sha256: digest('e'),
            dataset_split_sha256: digest('f'),
            environment_seed_manifest_sha256: digest('1'),
            actor_random_stream_manifest_sha256: digest('2'),
            exogenous_randomness_manifest_sha256: digest('3'),
            action_space_id: "vehicle-control-v1".into(),
            inference_hardware_id: "same-gpu-rig".into(),
            inference_power_profile: "fixed-tdp-profile".into(),
            inference_precision: "fp16".into(),
            runtime_config_sha256: digest('4'),
            input_privileges: vec!["camera_rgb".into(), "vehicle_state".into()],
            teacher_privileges: vec![],
            trainable_parameters: 2_000_000,
            training_simulator_transitions: 1_000_000,
            training_gpu_seconds: 7_200,
            training_energy_milliwatt_hours: 500_000,
            human_intervention_seconds: 3_600,
        }
    }

    fn metrics(score: f64, latency: f64) -> BTreeMap<String, MetricEstimate> {
        BTreeMap::from([
            (
                "driving_score".into(),
                MetricEstimate {
                    value: Some(score),
                    lower_95: Some(score - 0.5),
                    upper_95: Some(score + 0.5),
                    samples: 5,
                },
            ),
            (
                "p99_latency_ms".into(),
                MetricEstimate {
                    value: Some(latency),
                    lower_95: None,
                    upper_95: None,
                    samples: 5,
                },
            ),
        ])
    }

    fn clean_run() -> RunRecord {
        RunRecord {
            run_id: "clean-1".into(),
            context: context(),
            disposition: RunDisposition::Completed,
            expected_episodes: 5,
            completed_episodes: 5,
            failed_episodes: 0,
            incomplete_episodes: 0,
            censored_episodes: 0,
            timed_out_episodes: 0,
            observation_trace_sha256: digest('5'),
            metrics: metrics(90.0, 30.0),
            fault_events: vec![],
            recovery: None,
            fault: None,
        }
    }

    fn fault_spec() -> FaultSpec {
        FaultSpec::CameraBurstDrop {
            duration_ticks: 20,
            onset_tick: 100,
            burst_probability: 0.01,
            schedule_seed_sha256: digest('6'),
        }
    }

    fn perturbed_run() -> RunRecord {
        let fault = fault_spec();
        let fault_events = (100..120)
            .map(|tick| FaultEvent {
                event_id: format!("missing-frame-{tick}"),
                fault_family: "camera_burst_drop".into(),
                kind: FaultEventKind::ObservedMissingFrame,
                tick,
                affected_channel: "front_camera".into(),
                sampled_value_sha256: digest('7'),
                evidence_sha256: digest('8'),
            })
            .collect();
        RunRecord {
            run_id: "perturbed-1".into(),
            context: context(),
            disposition: RunDisposition::Completed,
            expected_episodes: 5,
            completed_episodes: 5,
            failed_episodes: 0,
            incomplete_episodes: 0,
            censored_episodes: 0,
            timed_out_episodes: 0,
            observation_trace_sha256: digest('9'),
            metrics: metrics(72.0, 55.0),
            fault_events,
            recovery: Some(RecoveryAssessment {
                disposition: RecoveryDisposition::Recovered,
                fault_onset_tick: 100,
                fault_restored_tick: Some(120),
                stable_from_tick: Some(125),
                stability_window_ticks: 20,
                stability_envelope_sha256: digest('a'),
            }),
            fault: Some(fault),
        }
    }

    fn campaign() -> FaultCampaign {
        FaultCampaign {
            schema_version: 1,
            campaign_id: "paired-fault-campaign-v1".into(),
            benchmark_id: "bench2drive-robust-v1".into(),
            evaluator_revision: "robust-eval-v1".into(),
            expected_pair_ids: vec!["pair-001".into()],
            metric_definitions: vec![
                MetricDefinition {
                    metric_id: "driving_score".into(),
                    direction: MetricDirection::HigherIsBetter,
                    unit: "points".into(),
                },
                MetricDefinition {
                    metric_id: "p99_latency_ms".into(),
                    direction: MetricDirection::LowerIsBetter,
                    unit: "ms".into(),
                },
            ],
            pairs: vec![PairedRun {
                pair_id: "pair-001".into(),
                clean: clean_run(),
                perturbed: perturbed_run(),
            }],
        }
    }

    #[test]
    fn checked_in_valid_campaign_fixture_passes_the_typed_auditor() {
        let input = include_str!("../examples/paired-camera-burst-valid.json");
        let campaign = FaultCampaign::from_json(input).unwrap();
        let report = campaign.audit();
        assert_eq!(report.disposition, AuditDisposition::Valid);
        assert_eq!(report.expected_pair_count, 1);
        assert_eq!(report.observed_pair_count, 1);
        assert_eq!(
            report.pair_results[0].metric_results[0].relative_degradation,
            Some(0.2)
        );
    }

    #[test]
    fn checked_in_mismatched_checkpoint_negative_control_is_rejected() {
        let input = include_str!("../examples/mismatched-checkpoint-negative-control.json");
        let campaign = FaultCampaign::from_json(input).unwrap();
        let report = campaign.audit();
        assert_eq!(report.disposition, AuditDisposition::Rejected);
        assert!(report.findings.contains(&Finding::MatchedContextMismatch));
    }

    #[test]
    fn campaign_schema_is_parseable_and_pins_protocol_bounds() {
        let schema: serde_json::Value =
            serde_json::from_str(include_str!("../schemas/fault-campaign-v1.schema.json"))
                .unwrap();
        assert_eq!(
            schema["$schema"],
            "https://json-schema.org/draft/2020-12/schema"
        );
        assert_eq!(
            schema["properties"]["schema_version"]["const"],
            1
        );
        assert!(schema["$defs"]["fault_spec"]["oneOf"].as_array().unwrap().len() >= 5);
    }

    #[test]
    fn accepts_complete_matched_clean_and_camera_drop_pair() {
        let report = campaign().audit();
        assert_eq!(report.disposition, AuditDisposition::Valid);
        assert!(report.findings.is_empty());
        assert_eq!(report.pair_results[0].metric_results[0].relative_degradation, Some(0.2));
        assert_eq!(report.pair_results[0].metric_results[1].relative_degradation, Some(25.0 / 30.0));
    }

    #[test]
    fn rejects_clean_perturbed_context_mismatch() {
        let mut value = campaign();
        value.pairs[0].perturbed.context.checkpoint_sha256 = digest('f');
        assert!(value.audit().findings.contains(&Finding::MatchedContextMismatch));
    }

    #[test]
    fn rejects_lost_pair_denominator() {
        let mut value = campaign();
        value.pairs.clear();
        assert!(value.audit().findings.contains(&Finding::PairDenominatorMismatch));
    }

    #[test]
    fn rejects_baseline_fault_contamination() {
        let mut value = campaign();
        value.pairs[0].clean.fault_events.push(FaultEvent {
            event_id: "unexpected".into(),
            fault_family: "camera_burst_drop".into(),
            kind: FaultEventKind::ObservedMissingFrame,
            tick: 100,
            affected_channel: "front_camera".into(),
            sampled_value_sha256: digest('7'),
            evidence_sha256: digest('8'),
        });
        assert!(value.audit().findings.contains(&Finding::CleanRunContainsFault));
    }

    #[test]
    fn rejects_unsupported_fault_severity() {
        let mut value = campaign();
        value.pairs[0].perturbed.fault = Some(FaultSpec::CameraBurstDrop {
            duration_ticks: 30,
            onset_tick: 100,
            burst_probability: 0.01,
            schedule_seed_sha256: digest('6'),
        });
        assert!(value.audit().findings.contains(&Finding::UnsupportedFaultSeverity));
    }

    #[test]
    fn rejects_missing_dropped_frame_events() {
        let mut value = campaign();
        value.pairs[0].perturbed.fault_events.pop();
        assert!(value.audit().findings.contains(&Finding::CameraDropDenominatorMismatch));
    }

    #[test]
    fn rejects_event_from_wrong_fault_family() {
        let mut value = campaign();
        value.pairs[0].perturbed.fault_events[0].fault_family = "gps_localization_noise".into();
        let report = value.audit();
        assert!(report.findings.contains(&Finding::FaultEventOutsideDeclaredSchedule));
    }

    #[test]
    fn rejects_episode_disposition_denominator_loss() {
        let mut value = campaign();
        value.pairs[0].perturbed.completed_episodes = 4;
        let report = value.audit();
        assert!(report.findings.contains(&Finding::EpisodeDenominatorMismatch));
    }

    #[test]
    fn baseline_zero_has_no_relative_degradation_value() {
        assert_eq!(
            relative_degradation(0.0, 10.0, MetricDirection::HigherIsBetter),
            None
        );
    }

    #[test]
    fn lower_is_better_relative_degradation_sign_is_correct() {
        assert_eq!(
            relative_degradation(100.0, 125.0, MetricDirection::LowerIsBetter),
            Some(0.25)
        );
        assert_eq!(
            relative_degradation(100.0, 75.0, MetricDirection::LowerIsBetter),
            Some(-0.25)
        );
    }

    #[test]
    fn rejects_invalid_metric_confidence_interval() {
        let mut value = campaign();
        value.pairs[0].perturbed.metrics.get_mut("driving_score").unwrap().lower_95 = Some(80.0);
        let report = value.audit();
        assert!(report.findings.contains(&Finding::InvalidMetricInterval));
    }

    #[test]
    fn rejects_missing_recovery_assessment() {
        let mut value = campaign();
        value.pairs[0].perturbed.recovery = None;
        let report = value.audit();
        assert!(report.findings.contains(&Finding::MissingRecoveryAssessment));
    }

    #[test]
    fn rejects_unsorted_or_duplicate_privilege_sets() {
        let mut value = campaign();
        value.pairs[0].clean.context.input_privileges = vec![
            "vehicle_state".into(),
            "camera_rgb".into(),
        ];
        let report = value.audit();
        assert!(report.findings.contains(&Finding::InvalidPrivilegeManifest));
    }
}
