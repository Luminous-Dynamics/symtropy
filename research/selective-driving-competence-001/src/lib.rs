//! Selective-driving risk/coverage evaluator.
//!
//! This crate evaluates frozen offline decision records. It does not authorize
//! actions, run a vehicle, or make a safety claim.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OutcomeStatus {
    Observed,
    Missing,
    Censored,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceRole {
    PolicyVisible,
    EvaluatorOnly,
    Future,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyInputField {
    pub field_id: String,
    pub source_role: SourceRole,
    pub available_timestamp_us: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DecisionRecord {
    pub decision_id: String,
    pub episode_id: String,
    pub scenario_family: String,
    pub prediction_timestamp_us: u64,
    pub outcome_timestamp_us: u64,
    pub outcome_status: OutcomeStatus,
    pub predicted_failure_probability: f64,
    pub observed_failure: bool,
    pub observed_loss: f64,
    pub policy_input_fields: Vec<PolicyInputField>,
    pub abstained: bool,
    pub abstention_timestamp_us: Option<u64>,
    pub decision_deadline_us: Option<u64>,
    pub fallback_succeeded: Option<bool>,
    pub fallback_duration_ms: Option<f64>,
    pub human_interventions: u64,
    pub inference_latency_ms: Option<f64>,
    pub inference_energy_mj: Option<f64>,
    pub peak_memory_bytes: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EvaluationInput {
    pub schema_version: u32,
    pub protocol_id: String,
    pub expected_episode_ids: Vec<String>,
    pub expected_decision_ids: Vec<String>,
    pub calibration_bin_count: usize,
    pub fixed_coverage_target: f64,
    pub fixed_risk_limit: f64,
    pub bootstrap_replicates: usize,
    pub bootstrap_seed: u64,
    pub decisions: Vec<DecisionRecord>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RiskCoveragePoint {
    pub threshold_probability_inclusive: f64,
    pub accepted_decisions: usize,
    pub coverage: f64,
    pub observed_failures: usize,
    pub failure_rate: f64,
    pub mean_observed_loss: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct CalibrationBin {
    pub bin_index: usize,
    pub lower_probability_inclusive: f64,
    pub upper_probability_bound: f64,
    pub upper_bound_inclusive: bool,
    pub count: usize,
    pub mean_predicted_failure_probability: Option<f64>,
    pub observed_failure_rate: Option<f64>,
    pub absolute_gap: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CalibrationMetrics {
    pub brier_score: f64,
    pub log_loss: f64,
    pub expected_calibration_error: f64,
    pub maximum_calibration_gap: f64,
    pub bins: Vec<CalibrationBin>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FixedCoverageResult {
    pub target_coverage: f64,
    pub achieved_coverage: f64,
    pub threshold_probability_inclusive: f64,
    pub accepted_decisions: usize,
    pub failure_rate: f64,
    pub mean_observed_loss: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct FixedRiskResult {
    pub maximum_failure_rate: f64,
    pub achieved_coverage: f64,
    pub threshold_probability_inclusive: Option<f64>,
    pub accepted_decisions: usize,
    pub failure_rate: Option<f64>,
    pub mean_observed_loss: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FamilyMetrics {
    pub scenario_family: String,
    pub decisions: usize,
    pub accepted_decisions: usize,
    pub abstained_decisions: usize,
    pub observed_failures: usize,
    pub accepted_failure_rate: Option<f64>,
    pub accepted_mean_loss: Option<f64>,
    pub coverage: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct FallbackMetrics {
    pub abstained_decisions: usize,
    pub fallback_successes: usize,
    pub fallback_failures: usize,
    pub fallback_success_rate: Option<f64>,
    pub mean_fallback_duration_ms: Option<f64>,
    pub late_abstentions: usize,
    pub total_human_interventions: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResourceMetrics {
    pub latency_observation_count: usize,
    pub mean_inference_latency_ms: Option<f64>,
    pub energy_observation_count: usize,
    pub mean_inference_energy_mj: Option<f64>,
    pub memory_observation_count: usize,
    pub mean_peak_memory_bytes: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BootstrapInterval {
    pub lower_95_percent: f64,
    pub upper_95_percent: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct BootstrapMetrics {
    pub episode_cluster_count: usize,
    pub replicates: usize,
    pub seed: u64,
    pub risk_coverage_area_failure_rate: Option<BootstrapInterval>,
    pub actual_autonomy_coverage: Option<BootstrapInterval>,
    pub actual_accepted_failure_rate: Option<BootstrapInterval>,
    pub accepted_risk_valid_replicates: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct EvaluationReport {
    pub schema_version: u32,
    pub protocol_id: String,
    pub expected_episode_count: usize,
    pub observed_episode_count: usize,
    pub expected_decision_count: usize,
    pub observed_decision_count: usize,
    pub observed_failure_count: usize,
    pub overall_failure_rate: f64,
    pub overall_mean_loss: f64,
    pub actual_autonomy_coverage: f64,
    pub actual_accepted_failure_rate: Option<f64>,
    pub actual_accepted_mean_loss: Option<f64>,
    pub risk_coverage_area_failure_rate: f64,
    pub risk_coverage_area_mean_loss: f64,
    pub risk_coverage_curve: Vec<RiskCoveragePoint>,
    pub fixed_coverage: FixedCoverageResult,
    pub fixed_risk: FixedRiskResult,
    pub calibration: CalibrationMetrics,
    pub families: Vec<FamilyMetrics>,
    pub fallback: FallbackMetrics,
    pub resources: ResourceMetrics,
    pub bootstrap: BootstrapMetrics,
    pub claim_ceiling: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationError {
    pub findings: Vec<String>,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.findings.join("; "))
    }
}

impl std::error::Error for ValidationError {}

impl EvaluationInput {
    pub fn from_json(input: &str) -> Result<Self, String> {
        serde_json::from_str(input).map_err(|error| format!("evaluation input JSON: {error}"))
    }

    pub fn validate(&self) -> Result<(), ValidationError> {
        let mut findings = Vec::new();
        if self.schema_version != 1 {
            findings.push("schema_version must equal 1".to_string());
        }
        if self.protocol_id.trim().is_empty() {
            findings.push("protocol_id must be non-empty".to_string());
        }
        if self.expected_episode_ids.is_empty() {
            findings.push("expected_episode_ids must be non-empty".to_string());
        }
        if self.expected_decision_ids.is_empty() {
            findings.push("expected_decision_ids must be non-empty".to_string());
        }
        if self.calibration_bin_count == 0 || self.calibration_bin_count > 100 {
            findings.push("calibration_bin_count must be in 1..=100".to_string());
        }
        if !unit_interval(self.fixed_coverage_target) || self.fixed_coverage_target == 0.0 {
            findings.push("fixed_coverage_target must be in (0, 1]".to_string());
        }
        if !unit_interval(self.fixed_risk_limit) {
            findings.push("fixed_risk_limit must be in [0, 1]".to_string());
        }
        if self.decisions.is_empty() {
            findings.push("decisions must be non-empty".to_string());
        }
        if !(100..=10_000).contains(&self.bootstrap_replicates) {
            findings.push("bootstrap_replicates must be in 100..=10000".to_string());
        }
        if self.expected_episode_ids.len() > 1_000_000
            || self.expected_decision_ids.len() > 10_000_000
            || self.decisions.len() > 10_000_000
        {
            findings.push("episode or decision bound exceeded".to_string());
        }
        if has_duplicates(&self.expected_episode_ids) {
            findings.push("expected_episode_ids contains duplicates".to_string());
        }
        if has_duplicates(&self.expected_decision_ids) {
            findings.push("expected_decision_ids contains duplicates".to_string());
        }
        if self
            .expected_episode_ids
            .iter()
            .any(|id| id.trim().is_empty())
        {
            findings.push("expected_episode_ids contains an empty identity".to_string());
        }
        if self
            .expected_decision_ids
            .iter()
            .any(|id| id.trim().is_empty())
        {
            findings.push("expected_decision_ids contains an empty identity".to_string());
        }

        let expected_episodes = self
            .expected_episode_ids
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        let expected_decisions = self
            .expected_decision_ids
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        let mut observed_episodes = BTreeSet::new();
        let mut observed_decisions = BTreeSet::new();

        for decision in &self.decisions {
            if decision.decision_id.trim().is_empty()
                || decision.episode_id.trim().is_empty()
                || decision.scenario_family.trim().is_empty()
            {
                findings.push(format!(
                    "decision {:?} has an empty decision, episode or scenario-family identity",
                    decision.decision_id
                ));
            }
            if !observed_decisions.insert(decision.decision_id.as_str()) {
                findings.push(format!("duplicate decision_id {:?}", decision.decision_id));
            }
            observed_episodes.insert(decision.episode_id.as_str());
            if !expected_decisions.contains(decision.decision_id.as_str()) {
                findings.push(format!("unexpected decision_id {:?}", decision.decision_id));
            }
            if !expected_episodes.contains(decision.episode_id.as_str()) {
                findings.push(format!(
                    "decision {:?} references an unexpected episode",
                    decision.decision_id
                ));
            }
            if decision.outcome_status != OutcomeStatus::Observed {
                findings.push(format!(
                    "decision {:?} has {:?} outcome; missing/censored outcomes are not silently excluded",
                    decision.decision_id, decision.outcome_status
                ));
            }
            if decision.outcome_timestamp_us < decision.prediction_timestamp_us {
                findings.push(format!(
                    "decision {:?} has outcome timestamp before prediction",
                    decision.decision_id
                ));
            }
            if !unit_interval(decision.predicted_failure_probability) {
                findings.push(format!(
                    "decision {:?} predicted_failure_probability is outside [0, 1]",
                    decision.decision_id
                ));
            }
            if !unit_interval(decision.observed_loss) {
                findings.push(format!(
                    "decision {:?} observed_loss is outside [0, 1]",
                    decision.decision_id
                ));
            }
            if decision.policy_input_fields.is_empty() {
                findings.push(format!(
                    "decision {:?} has no declared policy input fields",
                    decision.decision_id
                ));
            }
            if decision.policy_input_fields.len() > 100_000 {
                findings.push(format!(
                    "decision {:?} exceeds policy input field bound",
                    decision.decision_id
                ));
            }
            let mut field_ids = BTreeSet::new();
            for field in &decision.policy_input_fields {
                if field.field_id.trim().is_empty() {
                    findings.push(format!(
                        "decision {:?} contains an empty policy field identity",
                        decision.decision_id
                    ));
                }
                if !field_ids.insert(field.field_id.as_str()) {
                    findings.push(format!(
                        "decision {:?} contains duplicate policy field {:?}",
                        decision.decision_id, field.field_id
                    ));
                }
                if field.source_role != SourceRole::PolicyVisible {
                    findings.push(format!(
                        "decision {:?} contains non-policy-visible field {:?}",
                        decision.decision_id, field.field_id
                    ));
                }
                if field.available_timestamp_us > decision.prediction_timestamp_us {
                    findings.push(format!(
                        "decision {:?} contains future policy input {:?}",
                        decision.decision_id, field.field_id
                    ));
                }
            }

            for (name, value) in [
                ("inference_latency_ms", decision.inference_latency_ms),
                ("inference_energy_mj", decision.inference_energy_mj),
                ("fallback_duration_ms", decision.fallback_duration_ms),
            ] {
                if value.is_some_and(|number| !number.is_finite() || number < 0.0) {
                    findings.push(format!(
                        "decision {:?} has invalid {name}",
                        decision.decision_id
                    ));
                }
            }
            if decision.abstained {
                if decision.abstention_timestamp_us.is_none()
                    || decision.decision_deadline_us.is_none()
                    || decision.fallback_succeeded.is_none()
                    || decision.fallback_duration_ms.is_none()
                {
                    findings.push(format!(
                        "abstained decision {:?} lacks timestamp, deadline or fallback outcome/duration",
                        decision.decision_id
                    ));
                }
                if decision
                    .abstention_timestamp_us
                    .is_some_and(|timestamp| timestamp < decision.prediction_timestamp_us)
                {
                    findings.push(format!(
                        "abstained decision {:?} has abstention timestamp before prediction",
                        decision.decision_id
                    ));
                }
                if decision
                    .decision_deadline_us
                    .is_some_and(|deadline| deadline < decision.prediction_timestamp_us)
                {
                    findings.push(format!(
                        "abstained decision {:?} has decision deadline before prediction",
                        decision.decision_id
                    ));
                }
            } else if decision.abstention_timestamp_us.is_some()
                || decision.decision_deadline_us.is_some()
                || decision.fallback_succeeded.is_some()
                || decision.fallback_duration_ms.is_some()
                || decision.human_interventions > 0
            {
                findings.push(format!(
                    "non-abstained decision {:?} contains fallback/abstention-only fields",
                    decision.decision_id
                ));
            }
        }

        for missing in expected_decisions.difference(&observed_decisions) {
            findings.push(format!("expected decision {missing:?} is missing"));
        }
        for missing in expected_episodes.difference(&observed_episodes) {
            findings.push(format!(
                "expected episode {missing:?} has no decision records"
            ));
        }

        findings.sort();
        findings.dedup();
        if findings.is_empty() {
            Ok(())
        } else {
            Err(ValidationError { findings })
        }
    }

    pub fn evaluate(&self) -> Result<EvaluationReport, ValidationError> {
        self.validate()?;
        Ok(evaluate_validated(self))
    }
}

fn evaluate_validated(input: &EvaluationInput) -> EvaluationReport {
    let total = input.decisions.len();
    let failures = input
        .decisions
        .iter()
        .filter(|decision| decision.observed_failure)
        .count();
    let overall_failure_rate = failures as f64 / total as f64;
    let overall_mean_loss = mean(
        input
            .decisions
            .iter()
            .map(|decision| decision.observed_loss),
    );

    // Stable deterministic tie semantics: equal probabilities enter coverage together.
    let mut ranked = input.decisions.iter().collect::<Vec<_>>();
    ranked.sort_by(|left, right| {
        left.predicted_failure_probability
            .total_cmp(&right.predicted_failure_probability)
            .then_with(|| left.decision_id.cmp(&right.decision_id))
    });

    let mut curve = Vec::new();
    let mut accepted = 0_usize;
    let mut accepted_failures = 0_usize;
    let mut accepted_loss = 0.0_f64;
    let mut index = 0_usize;
    let mut previous_coverage = 0.0_f64;
    let mut aurc_failure = 0.0_f64;
    let mut aurc_loss = 0.0_f64;
    while index < ranked.len() {
        let threshold = ranked[index].predicted_failure_probability;
        let mut end = index;
        while end < ranked.len() && ranked[end].predicted_failure_probability == threshold {
            accepted += 1;
            accepted_failures += if ranked[end].observed_failure { 1 } else { 0 };
            accepted_loss += ranked[end].observed_loss;
            end += 1;
        }
        let coverage = accepted as f64 / total as f64;
        let failure_rate = accepted_failures as f64 / accepted as f64;
        let mean_loss = accepted_loss / accepted as f64;
        let delta_coverage = coverage - previous_coverage;
        // Right-continuous step integral. Equal-score groups are never split.
        aurc_failure += delta_coverage * failure_rate;
        aurc_loss += delta_coverage * mean_loss;
        curve.push(RiskCoveragePoint {
            threshold_probability_inclusive: threshold,
            accepted_decisions: accepted,
            coverage,
            observed_failures: accepted_failures,
            failure_rate,
            mean_observed_loss: mean_loss,
        });
        previous_coverage = coverage;
        index = end;
    }

    let fixed_coverage_point = curve
        .iter()
        .find(|point| point.coverage + f64::EPSILON >= input.fixed_coverage_target)
        .expect("validated non-empty decisions yield a risk-coverage point");
    let fixed_coverage = FixedCoverageResult {
        target_coverage: input.fixed_coverage_target,
        achieved_coverage: fixed_coverage_point.coverage,
        threshold_probability_inclusive: fixed_coverage_point.threshold_probability_inclusive,
        accepted_decisions: fixed_coverage_point.accepted_decisions,
        failure_rate: fixed_coverage_point.failure_rate,
        mean_observed_loss: fixed_coverage_point.mean_observed_loss,
    };

    let fixed_risk_point = curve
        .iter()
        .filter(|point| point.failure_rate <= input.fixed_risk_limit)
        .max_by(|left, right| left.coverage.total_cmp(&right.coverage));
    let fixed_risk = match fixed_risk_point {
        Some(point) => FixedRiskResult {
            maximum_failure_rate: input.fixed_risk_limit,
            achieved_coverage: point.coverage,
            threshold_probability_inclusive: Some(point.threshold_probability_inclusive),
            accepted_decisions: point.accepted_decisions,
            failure_rate: Some(point.failure_rate),
            mean_observed_loss: Some(point.mean_observed_loss),
        },
        None => FixedRiskResult {
            maximum_failure_rate: input.fixed_risk_limit,
            achieved_coverage: 0.0,
            threshold_probability_inclusive: None,
            accepted_decisions: 0,
            failure_rate: None,
            mean_observed_loss: None,
        },
    };

    let calibration = calibration_metrics(&input.decisions, input.calibration_bin_count);
    let families = family_metrics(&input.decisions);
    let fallback = fallback_metrics(&input.decisions);
    let resources = resource_metrics(&input.decisions);
    let bootstrap = bootstrap_metrics(input);
    let accepted_records = input
        .decisions
        .iter()
        .filter(|decision| !decision.abstained)
        .collect::<Vec<_>>();
    let actual_autonomy_coverage = accepted_records.len() as f64 / total as f64;
    let actual_accepted_failure_rate = if accepted_records.is_empty() {
        None
    } else {
        Some(
            accepted_records
                .iter()
                .filter(|decision| decision.observed_failure)
                .count() as f64
                / accepted_records.len() as f64,
        )
    };
    let actual_accepted_mean_loss = if accepted_records.is_empty() {
        None
    } else {
        Some(mean(
            accepted_records
                .iter()
                .map(|decision| decision.observed_loss),
        ))
    };

    EvaluationReport {
        schema_version: 1,
        protocol_id: input.protocol_id.clone(),
        expected_episode_count: input.expected_episode_ids.len(),
        observed_episode_count: input.expected_episode_ids.len(),
        expected_decision_count: input.expected_decision_ids.len(),
        observed_decision_count: total,
        observed_failure_count: failures,
        overall_failure_rate,
        overall_mean_loss,
        actual_autonomy_coverage,
        actual_accepted_failure_rate,
        actual_accepted_mean_loss,
        risk_coverage_area_failure_rate: aurc_failure,
        risk_coverage_area_mean_loss: aurc_loss,
        risk_coverage_curve: curve,
        fixed_coverage,
        fixed_risk,
        calibration,
        families,
        fallback,
        resources,
        bootstrap,
        claim_ceiling: "Offline statistics for the exact declared frozen records only. Risk-coverage and calibration metrics are not a safe-action certificate, a distribution-free safety guarantee, a runtime controller or evidence of roadworthiness.".to_string(),
    }
}

#[derive(Debug, Clone, Copy)]
struct SelectiveSummary {
    risk_coverage_area_failure_rate: f64,
    actual_autonomy_coverage: f64,
    actual_accepted_failure_rate: Option<f64>,
}

fn selective_summary(decisions: &[&DecisionRecord]) -> SelectiveSummary {
    let total = decisions.len();
    if total == 0 {
        return SelectiveSummary {
            risk_coverage_area_failure_rate: 0.0,
            actual_autonomy_coverage: 0.0,
            actual_accepted_failure_rate: None,
            accepted_risk_valid_replicates: 0,
        };
    }
    let mut ranked = decisions.to_vec();
    ranked.sort_by(|left, right| {
        left.predicted_failure_probability
            .total_cmp(&right.predicted_failure_probability)
            .then_with(|| left.decision_id.cmp(&right.decision_id))
    });
    let mut accepted = 0_usize;
    let mut failures = 0_usize;
    let mut area = 0.0_f64;
    let mut previous_coverage = 0.0_f64;
    let mut index = 0_usize;
    while index < ranked.len() {
        let score = ranked[index].predicted_failure_probability;
        let mut end = index;
        while end < ranked.len() && ranked[end].predicted_failure_probability == score {
            accepted += 1;
            failures += if ranked[end].observed_failure { 1 } else { 0 };
            end += 1;
        }
        let coverage = accepted as f64 / total as f64;
        let risk = failures as f64 / accepted as f64;
        area += (coverage - previous_coverage) * risk;
        previous_coverage = coverage;
        index = end;
    }
    let accepted_records = decisions
        .iter()
        .filter(|decision| !decision.abstained)
        .collect::<Vec<_>>();
    let actual_autonomy_coverage = accepted_records.len() as f64 / total as f64;
    let actual_accepted_failure_rate = if accepted_records.is_empty() {
        None
    } else {
        Some(
            accepted_records
                .iter()
                .filter(|decision| decision.observed_failure)
                .count() as f64
                / accepted_records.len() as f64,
        )
    };
    SelectiveSummary {
        risk_coverage_area_failure_rate: area,
        actual_autonomy_coverage,
        actual_accepted_failure_rate,
    }
}

#[derive(Debug, Clone, Copy)]
struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        value ^ (value >> 31)
    }

    fn index(&mut self, upper_exclusive: usize) -> usize {
        let bound = upper_exclusive as u64;
        let rejection_threshold = bound.wrapping_neg() % bound;
        loop {
            let value = self.next_u64();
            if value >= rejection_threshold {
                return (value % bound) as usize;
            }
        }
    }
}

fn percentile_interval(mut values: Vec<f64>) -> BootstrapInterval {
    values.sort_by(f64::total_cmp);
    let lower = ((values.len() - 1) as f64 * 0.025).floor() as usize;
    let upper = ((values.len() - 1) as f64 * 0.975).ceil() as usize;
    BootstrapInterval {
        lower_95_percent: values[lower],
        upper_95_percent: values[upper],
    }
}

fn bootstrap_metrics(input: &EvaluationInput) -> BootstrapMetrics {
    let mut episodes: BTreeMap<&str, Vec<&DecisionRecord>> = BTreeMap::new();
    for decision in &input.decisions {
        episodes
            .entry(decision.episode_id.as_str())
            .or_default()
            .push(decision);
    }
    let episode_ids = episodes.keys().copied().collect::<Vec<_>>();
    if episode_ids.len() < 2 {
        return BootstrapMetrics {
            episode_cluster_count: episode_ids.len(),
            replicates: input.bootstrap_replicates,
            seed: input.bootstrap_seed,
            risk_coverage_area_failure_rate: None,
            actual_autonomy_coverage: None,
            actual_accepted_failure_rate: None,
        };
    }

    let mut rng = SplitMix64::new(input.bootstrap_seed);
    let mut aurc_samples = Vec::with_capacity(input.bootstrap_replicates);
    let mut coverage_samples = Vec::with_capacity(input.bootstrap_replicates);
    let mut accepted_risk_samples = Vec::with_capacity(input.bootstrap_replicates);
    for _ in 0..input.bootstrap_replicates {
        let mut sample = Vec::new();
        for _ in 0..episode_ids.len() {
            let selected_episode = episode_ids[rng.index(episode_ids.len())];
            if let Some(records) = episodes.get(selected_episode) {
                sample.extend(records.iter().copied());
            }
        }
        let summary = selective_summary(&sample);
        aurc_samples.push(summary.risk_coverage_area_failure_rate);
        coverage_samples.push(summary.actual_autonomy_coverage);
        if let Some(rate) = summary.actual_accepted_failure_rate {
            accepted_risk_samples.push(rate);
        }
    }

    BootstrapMetrics {
        episode_cluster_count: episode_ids.len(),
        replicates: input.bootstrap_replicates,
        seed: input.bootstrap_seed,
        risk_coverage_area_failure_rate: Some(percentile_interval(aurc_samples)),
        actual_autonomy_coverage: Some(percentile_interval(coverage_samples)),
        actual_accepted_failure_rate: if accepted_risk_samples.len() == input.bootstrap_replicates {
            Some(percentile_interval(accepted_risk_samples))
        } else {
            None
        },
        accepted_risk_valid_replicates: accepted_risk_samples.len(),
    }
}

fn calibration_metrics(decisions: &[DecisionRecord], bin_count: usize) -> CalibrationMetrics {
    let count = decisions.len() as f64;
    let brier_score = mean(decisions.iter().map(|decision| {
        let actual = if decision.observed_failure { 1.0 } else { 0.0 };
        (decision.predicted_failure_probability - actual).powi(2)
    }));
    let epsilon = 1e-15_f64;
    let log_loss = mean(decisions.iter().map(|decision| {
        let p = decision
            .predicted_failure_probability
            .clamp(epsilon, 1.0 - epsilon);
        if decision.observed_failure {
            -p.ln()
        } else {
            -(1.0 - p).ln()
        }
    }));
    let mut bins = (0..bin_count)
        .map(|bin_index| {
            let lower = bin_index as f64 / bin_count as f64;
            let upper = if bin_index + 1 == bin_count {
                1.0
            } else {
                (bin_index + 1) as f64 / bin_count as f64
            };
            let members = decisions
                .iter()
                .filter(|decision| {
                    decision.predicted_failure_probability >= lower
                        && (decision.predicted_failure_probability < upper
                            || (bin_index + 1 == bin_count
                                && decision.predicted_failure_probability <= upper))
                })
                .collect::<Vec<_>>();
            if members.is_empty() {
                CalibrationBin {
                    bin_index,
                    lower_probability_inclusive: lower,
                    upper_probability_bound: upper,
                    upper_bound_inclusive: bin_index + 1 == bin_count,
                    count: 0,
                    mean_predicted_failure_probability: None,
                    observed_failure_rate: None,
                    absolute_gap: None,
                }
            } else {
                let predicted = mean(
                    members
                        .iter()
                        .map(|decision| decision.predicted_failure_probability),
                );
                let observed = members
                    .iter()
                    .filter(|decision| decision.observed_failure)
                    .count() as f64
                    / members.len() as f64;
                CalibrationBin {
                    bin_index,
                    lower_probability_inclusive: lower,
                    upper_probability_bound: upper,
                    upper_bound_inclusive: bin_index + 1 == bin_count,
                    count: members.len(),
                    mean_predicted_failure_probability: Some(predicted),
                    observed_failure_rate: Some(observed),
                    absolute_gap: Some((predicted - observed).abs()),
                }
            }
        })
        .collect::<Vec<_>>();
    bins.sort_by_key(|bin| bin.bin_index);
    let expected_calibration_error = bins
        .iter()
        .filter_map(|bin| bin.absolute_gap.map(|gap| bin.count as f64 / count * gap))
        .sum();
    let maximum_calibration_gap = bins
        .iter()
        .filter_map(|bin| bin.absolute_gap)
        .fold(0.0_f64, f64::max);
    CalibrationMetrics {
        brier_score,
        log_loss,
        expected_calibration_error,
        maximum_calibration_gap,
        bins,
    }
}

fn family_metrics(decisions: &[DecisionRecord]) -> Vec<FamilyMetrics> {
    let mut by_family: BTreeMap<&str, Vec<&DecisionRecord>> = BTreeMap::new();
    for decision in decisions {
        by_family
            .entry(decision.scenario_family.as_str())
            .or_default()
            .push(decision);
    }
    by_family
        .into_iter()
        .map(|(family, records)| {
            let accepted = records
                .iter()
                .filter(|decision| !decision.abstained)
                .collect::<Vec<_>>();
            let failures = accepted
                .iter()
                .filter(|decision| decision.observed_failure)
                .count();
            FamilyMetrics {
                scenario_family: family.to_string(),
                decisions: records.len(),
                accepted_decisions: accepted.len(),
                abstained_decisions: records.len() - accepted.len(),
                observed_failures: records
                    .iter()
                    .filter(|decision| decision.observed_failure)
                    .count(),
                accepted_failure_rate: if accepted.is_empty() {
                    None
                } else {
                    Some(failures as f64 / accepted.len() as f64)
                },
                accepted_mean_loss: if accepted.is_empty() {
                    None
                } else {
                    Some(mean(accepted.iter().map(|decision| decision.observed_loss)))
                },
                coverage: accepted.len() as f64 / records.len() as f64,
            }
        })
        .collect()
}

fn fallback_metrics(decisions: &[DecisionRecord]) -> FallbackMetrics {
    let abstained = decisions
        .iter()
        .filter(|decision| decision.abstained)
        .collect::<Vec<_>>();
    let successes = abstained
        .iter()
        .filter(|decision| decision.fallback_succeeded == Some(true))
        .count();
    let failures = abstained
        .iter()
        .filter(|decision| decision.fallback_succeeded == Some(false))
        .count();
    let durations = abstained
        .iter()
        .filter_map(|decision| decision.fallback_duration_ms);
    let late = abstained
        .iter()
        .filter(|decision| {
            matches!(
                (decision.abstention_timestamp_us, decision.decision_deadline_us),
                (Some(timestamp), Some(deadline)) if timestamp > deadline
            )
        })
        .count();
    FallbackMetrics {
        abstained_decisions: abstained.len(),
        fallback_successes: successes,
        fallback_failures: failures,
        fallback_success_rate: if abstained.is_empty() {
            None
        } else {
            Some(successes as f64 / abstained.len() as f64)
        },
        mean_fallback_duration_ms: {
            let values = durations.collect::<Vec<_>>();
            if values.is_empty() {
                None
            } else {
                Some(mean(values.into_iter()))
            }
        },
        late_abstentions: late,
        total_human_interventions: abstained
            .iter()
            .map(|decision| decision.human_interventions)
            .sum(),
    }
}

fn resource_metrics(decisions: &[DecisionRecord]) -> ResourceMetrics {
    let latencies = decisions
        .iter()
        .filter_map(|decision| decision.inference_latency_ms)
        .collect::<Vec<_>>();
    let energies = decisions
        .iter()
        .filter_map(|decision| decision.inference_energy_mj)
        .collect::<Vec<_>>();
    let memories = decisions
        .iter()
        .filter_map(|decision| decision.peak_memory_bytes)
        .collect::<Vec<_>>();
    ResourceMetrics {
        latency_observation_count: latencies.len(),
        mean_inference_latency_ms: if latencies.is_empty() {
            None
        } else {
            Some(mean(latencies.iter().copied()))
        },
        energy_observation_count: energies.len(),
        mean_inference_energy_mj: if energies.is_empty() {
            None
        } else {
            Some(mean(energies.iter().copied()))
        },
        memory_observation_count: memories.len(),
        mean_peak_memory_bytes: if memories.is_empty() {
            None
        } else {
            Some(memories.iter().map(|value| *value as f64).sum::<f64>() / memories.len() as f64)
        },
    }
}

fn unit_interval(value: f64) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}

fn has_duplicates(values: &[String]) -> bool {
    let mut seen = BTreeSet::new();
    values.iter().any(|value| !seen.insert(value.as_str()))
}

fn mean(values: impl Iterator<Item = f64>) -> f64 {
    let collected = values.collect::<Vec<_>>();
    if collected.is_empty() {
        0.0
    } else {
        collected.iter().sum::<f64>() / collected.len() as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(id: &str, family: &str, probability: f64, failed: bool, loss: f64) -> DecisionRecord {
        DecisionRecord {
            decision_id: id.to_string(),
            episode_id: format!("episode-{id}"),
            scenario_family: family.to_string(),
            prediction_timestamp_us: 1_000,
            outcome_timestamp_us: 2_000,
            outcome_status: OutcomeStatus::Observed,
            predicted_failure_probability: probability,
            observed_failure: failed,
            observed_loss: loss,
            policy_input_fields: vec![PolicyInputField {
                field_id: "front-camera".to_string(),
                source_role: SourceRole::PolicyVisible,
                available_timestamp_us: 900,
            }],
            abstained: false,
            abstention_timestamp_us: None,
            decision_deadline_us: None,
            fallback_succeeded: None,
            fallback_duration_ms: None,
            human_interventions: 0,
            inference_latency_ms: Some(12.0),
            inference_energy_mj: Some(4.0),
            peak_memory_bytes: Some(1024),
        }
    }

    fn input(records: Vec<DecisionRecord>) -> EvaluationInput {
        let ids = records
            .iter()
            .map(|record| record.decision_id.clone())
            .collect::<Vec<_>>();
        let episodes = records
            .iter()
            .map(|record| record.episode_id.clone())
            .collect::<Vec<_>>();
        EvaluationInput {
            schema_version: 1,
            protocol_id: "selective-driving-v0.1".to_string(),
            expected_episode_ids: episodes,
            expected_decision_ids: ids,
            calibration_bin_count: 5,
            fixed_coverage_target: 0.5,
            fixed_risk_limit: 0.25,
            bootstrap_replicates: 200,
            bootstrap_seed: 7,
            decisions: records,
        }
    }

    #[test]
    fn low_risk_first_ranking_has_lower_area_than_random_order() {
        let result = input(vec![
            record("a", "urban", 0.01, false, 0.0),
            record("b", "urban", 0.02, false, 0.0),
            record("c", "urban", 0.8, true, 1.0),
            record("d", "urban", 0.9, true, 1.0),
        ])
        .evaluate()
        .unwrap();
        assert!((result.risk_coverage_area_failure_rate - 0.20833333333333331).abs() < 1e-12);
        assert_eq!(result.risk_coverage_curve[0].failure_rate, 0.0);
        assert_eq!(result.risk_coverage_curve.last().unwrap().failure_rate, 0.5);
    }

    #[test]
    fn inverted_risk_ranking_has_worse_area_than_perfect_ranking() {
        let good = input(vec![
            record("a", "urban", 0.01, false, 0.0),
            record("b", "urban", 0.02, false, 0.0),
            record("c", "urban", 0.8, true, 1.0),
            record("d", "urban", 0.9, true, 1.0),
        ])
        .evaluate()
        .unwrap();
        let bad = input(vec![
            record("a", "urban", 0.9, false, 0.0),
            record("b", "urban", 0.8, false, 0.0),
            record("c", "urban", 0.02, true, 1.0),
            record("d", "urban", 0.01, true, 1.0),
        ])
        .evaluate()
        .unwrap();
        assert!(bad.risk_coverage_area_failure_rate > good.risk_coverage_area_failure_rate);
    }

    #[test]
    fn equal_score_ties_enter_coverage_as_one_group() {
        let result = input(vec![
            record("a", "urban", 0.1, false, 0.0),
            record("b", "urban", 0.1, true, 1.0),
            record("c", "urban", 0.9, false, 0.0),
        ])
        .evaluate()
        .unwrap();
        assert_eq!(result.risk_coverage_curve.len(), 2);
        assert_eq!(result.risk_coverage_curve[0].accepted_decisions, 2);
        assert!((result.risk_coverage_curve[0].coverage - 2.0 / 3.0).abs() < 1e-12);
    }

    #[test]
    fn rejects_missing_decision_from_expected_denominator() {
        let mut value = input(vec![record("a", "urban", 0.1, false, 0.0)]);
        value.expected_decision_ids.push("missing".to_string());
        assert!(
            value
                .validate()
                .unwrap_err()
                .findings
                .iter()
                .any(|item| item.contains("missing"))
        );
    }

    #[test]
    fn rejects_duplicate_decision_identity() {
        let first = record("a", "urban", 0.1, false, 0.0);
        let mut second = record("a", "urban", 0.2, false, 0.0);
        second.episode_id = first.episode_id.clone();
        let mut value = input(vec![first.clone(), second]);
        value.expected_decision_ids = vec!["a".to_string()];
        value.expected_episode_ids = vec![first.episode_id];
        assert!(value.validate().is_err());
    }

    #[test]
    fn rejects_missing_or_censored_outcome_instead_of_zero_imputation() {
        let mut value = input(vec![record("a", "urban", 0.1, false, 0.0)]);
        value.decisions[0].outcome_status = OutcomeStatus::Missing;
        assert!(
            value
                .validate()
                .unwrap_err()
                .findings
                .iter()
                .any(|item| item.contains("missing/censored"))
        );
        value.decisions[0].outcome_status = OutcomeStatus::Censored;
        assert!(value.validate().is_err());
    }

    #[test]
    fn rejects_future_policy_input_and_evaluator_only_fields() {
        let mut value = input(vec![record("a", "urban", 0.1, false, 0.0)]);
        value.decisions[0].policy_input_fields[0].available_timestamp_us = 1_001;
        assert!(
            value
                .validate()
                .unwrap_err()
                .findings
                .iter()
                .any(|item| item.contains("future policy input"))
        );
        value.decisions[0].policy_input_fields[0].available_timestamp_us = 900;
        value.decisions[0].policy_input_fields[0].source_role = SourceRole::EvaluatorOnly;
        assert!(value.validate().is_err());
    }

    #[test]
    fn rejects_non_monotonic_prediction_outcome_timestamps() {
        let mut value = input(vec![record("a", "urban", 0.1, false, 0.0)]);
        value.decisions[0].outcome_timestamp_us = 999;
        assert!(
            value
                .validate()
                .unwrap_err()
                .findings
                .iter()
                .any(|item| item.contains("outcome timestamp"))
        );
    }

    #[test]
    fn computes_brier_score_for_binary_outcomes() {
        let result = input(vec![
            record("a", "urban", 0.1, false, 0.0),
            record("b", "urban", 0.9, true, 1.0),
        ])
        .evaluate()
        .unwrap();
        assert!((result.calibration.brier_score - 0.01).abs() < 1e-12);
    }

    #[test]
    fn rejects_abstention_without_explicit_fallback_evidence() {
        let mut value = input(vec![record("a", "urban", 0.4, false, 0.0)]);
        value.decisions[0].abstained = true;
        assert!(
            value
                .validate()
                .unwrap_err()
                .findings
                .iter()
                .any(|item| item.contains("lacks timestamp"))
        );
    }

    #[test]
    fn rejects_abstention_timestamp_or_deadline_before_prediction() {
        let mut value = input(vec![record("a", "urban", 0.4, false, 0.0)]);
        value.decisions[0].abstained = true;
        value.decisions[0].abstention_timestamp_us = Some(900);
        value.decisions[0].decision_deadline_us = Some(1_200);
        value.decisions[0].fallback_succeeded = Some(true);
        value.decisions[0].fallback_duration_ms = Some(100.0);
        assert!(
            value
                .validate()
                .unwrap_err()
                .findings
                .iter()
                .any(|item| item.contains("abstention timestamp before prediction"))
        );

        value.decisions[0].abstention_timestamp_us = Some(1_100);
        value.decisions[0].decision_deadline_us = Some(999);
        assert!(
            value
                .validate()
                .unwrap_err()
                .findings
                .iter()
                .any(|item| item.contains("decision deadline before prediction"))
        );
    }

    #[test]
    fn reports_late_abstention_and_fallback_failures() {
        let mut item = record("a", "urban", 0.4, true, 1.0);
        item.abstained = true;
        item.abstention_timestamp_us = Some(1_500);
        item.decision_deadline_us = Some(1_400);
        item.fallback_succeeded = Some(false);
        item.fallback_duration_ms = Some(90.0);
        item.human_interventions = 2;
        let result = input(vec![item]).evaluate().unwrap();
        assert_eq!(result.fallback.late_abstentions, 1);
        assert_eq!(result.fallback.fallback_failures, 1);
        assert_eq!(result.fallback.total_human_interventions, 2);
        assert_eq!(result.actual_autonomy_coverage, 0.0);
    }

    #[test]
    fn fixed_coverage_respects_tie_overshoot() {
        let mut value = input(vec![
            record("a", "urban", 0.1, false, 0.0),
            record("b", "urban", 0.1, false, 0.0),
            record("c", "urban", 0.9, true, 1.0),
        ]);
        value.fixed_coverage_target = 0.5;
        let result = value.evaluate().unwrap();
        assert_eq!(result.fixed_coverage.accepted_decisions, 2);
        assert!((result.fixed_coverage.achieved_coverage - 2.0 / 3.0).abs() < 1e-12);
    }

    #[test]
    fn fixed_risk_reports_maximum_attainable_coverage() {
        let mut value = input(vec![
            record("a", "urban", 0.1, false, 0.0),
            record("b", "urban", 0.2, true, 1.0),
            record("c", "urban", 0.9, true, 1.0),
        ]);
        value.fixed_risk_limit = 0.0;
        let result = value.evaluate().unwrap();
        assert_eq!(result.fixed_risk.accepted_decisions, 1);
        assert!((result.fixed_risk.achieved_coverage - 1.0 / 3.0).abs() < 1e-12);
    }

    #[test]
    fn stratifies_worst_family_instead_of_only_reporting_aggregate() {
        let result = input(vec![
            record("a", "daylight", 0.1, false, 0.0),
            record("b", "daylight", 0.1, false, 0.0),
            record("c", "night-rain", 0.1, true, 1.0),
        ])
        .evaluate()
        .unwrap();
        assert_eq!(result.families.len(), 2);
        assert_eq!(result.families[1].scenario_family, "night-rain");
        assert_eq!(result.families[1].accepted_failure_rate, Some(1.0));
    }

    #[test]
    fn missing_resource_measurements_are_not_imputed_as_zero() {
        let mut value = input(vec![record("a", "urban", 0.1, false, 0.0)]);
        value.decisions[0].inference_energy_mj = None;
        let result = value.evaluate().unwrap();
        assert_eq!(result.resources.energy_observation_count, 0);
        assert_eq!(result.resources.mean_inference_energy_mj, None);
    }

    #[test]
    fn no_abstention_has_no_fallback_rate() {
        let result = input(vec![record("a", "urban", 0.1, false, 0.0)])
            .evaluate()
            .unwrap();
        assert_eq!(result.fallback.fallback_success_rate, None);
        assert_eq!(result.fallback.abstained_decisions, 0);
    }

    #[test]
    fn deterministic_report_is_independent_of_input_order() {
        let first = record("a", "urban", 0.1, false, 0.0);
        let second = record("b", "urban", 0.8, true, 1.0);
        let left = input(vec![first.clone(), second.clone()])
            .evaluate()
            .unwrap();
        let right = input(vec![second, first]).evaluate().unwrap();
        assert_eq!(
            serde_json::to_string(&left).unwrap(),
            serde_json::to_string(&right).unwrap()
        );
    }

    #[test]
    fn episode_cluster_bootstrap_is_deterministic_and_reports_coverage_intervals() {
        let value = input(vec![
            record("a", "urban", 0.01, false, 0.0),
            record("b", "urban", 0.1, false, 0.0),
            record("c", "night", 0.8, true, 1.0),
            record("d", "night", 0.9, true, 1.0),
        ]);
        let first = value.evaluate().unwrap().bootstrap;
        let second = value.evaluate().unwrap().bootstrap;
        assert_eq!(first.episode_cluster_count, 4);
        assert_eq!(first.replicates, 200);
        assert_eq!(
            first
                .risk_coverage_area_failure_rate
                .as_ref()
                .unwrap()
                .lower_95_percent,
            second
                .risk_coverage_area_failure_rate
                .as_ref()
                .unwrap()
                .lower_95_percent
        );
        let coverage = first.actual_autonomy_coverage.unwrap();
        assert!(coverage.lower_95_percent <= coverage.upper_95_percent);
        assert!((0.0..=1.0).contains(&coverage.lower_95_percent));
    }

    #[test]
    fn cluster_bootstrap_resampling_is_reproducible_for_the_same_seed() {
        let mut value = input(vec![
            record("a", "urban", 0.01, false, 0.0),
            record("b", "urban", 0.1, false, 0.0),
            record("c", "night", 0.8, true, 1.0),
            record("d", "night", 0.9, true, 1.0),
        ]);
        value.bootstrap_replicates = 400;
        value.bootstrap_seed = 0x5eed_cafe;
        let first = value.evaluate().unwrap().bootstrap;
        let second = value.evaluate().unwrap().bootstrap;
        assert_eq!(
            serde_json::to_string(&first).unwrap(),
            serde_json::to_string(&second).unwrap()
        );
    }

    #[test]
    fn accepted_risk_interval_is_unavailable_when_any_replicate_has_no_accepted_decisions() {
        let mut first = record("a", "all-abstain", 0.1, false, 0.0);
        first.abstained = true;
        first.abstention_timestamp_us = Some(1_100);
        first.decision_deadline_us = Some(1_200);
        first.fallback_succeeded = Some(true);
        first.fallback_duration_ms = Some(100.0);
        let second = record("b", "accepted", 0.2, false, 0.0);
        let mut value = input(vec![first, second]);
        value.bootstrap_replicates = 100;
        value.bootstrap_seed = 17;
        let result = value.evaluate().unwrap();
        assert!(result.bootstrap.accepted_risk_valid_replicates <= 100);
        if result.bootstrap.accepted_risk_valid_replicates < 100 {
            assert_eq!(result.bootstrap.actual_accepted_failure_rate, None);
        }
    }

    #[test]
    fn single_episode_bootstrap_is_reported_unavailable_not_as_false_precision() {
        let report = input(vec![record("a", "urban", 0.1, false, 0.0)])
            .evaluate()
            .unwrap();
        assert_eq!(report.bootstrap.episode_cluster_count, 1);
        assert_eq!(report.bootstrap.actual_autonomy_coverage, None);
        assert_eq!(report.bootstrap.risk_coverage_area_failure_rate, None);
    }

    #[test]
    fn published_demo_evaluates_with_complete_fallback_record() {
        let value = EvaluationInput::from_json(include_str!(
            "../examples/selective-driving-demo-v0_1.json"
        ))
        .unwrap();
        let report = value.evaluate().unwrap();
        assert_eq!(report.observed_decision_count, 6);
        assert_eq!(report.expected_episode_count, 3);
        assert_eq!(report.fallback.abstained_decisions, 1);
        assert_eq!(report.fallback.fallback_successes, 1);
        assert_eq!(report.fallback.late_abstentions, 0);
    }

    #[test]
    fn protocol_documents_are_parseable_and_mark_targets_not_measured() {
        let manifest: serde_json::Value =
            serde_json::from_str(include_str!("../manifest-v0_1.json")).unwrap();
        let schema: serde_json::Value = serde_json::from_str(include_str!(
            "../selective-driving-evaluation-v0_1.schema.json"
        ))
        .unwrap();
        assert_eq!(
            manifest["status"],
            "reference_evaluator_and_protocol_in_progress"
        );
        assert_eq!(manifest["prospective_targets"]["status"], "not_measured");
        assert_eq!(
            schema["$schema"],
            "https://json-schema.org/draft/2020-12/schema"
        );
    }

    #[test]
    fn strict_json_rejects_unknown_fields() {
        let mut raw =
            serde_json::to_value(input(vec![record("a", "urban", 0.1, false, 0.0)])).unwrap();
        raw["untrusted_shortcut"] = serde_json::json!(true);
        assert!(EvaluationInput::from_json(&raw.to_string()).is_err());
    }
}
