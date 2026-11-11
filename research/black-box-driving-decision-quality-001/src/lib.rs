//! Four-tier black-box decision-quality evaluation.
//!
//! This crate evaluates finite declared records. It does not run a simulator,
//! vehicle policy, controller or actuator, and does not authenticate physical
//! evidence or establish causal/vehicle safety.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const SCHEMA_VERSION: u32 = 1;
pub const DEFAULT_CALIBRATION_BINS: usize = 10;
const MAX_EPISODES: usize = 100_000;
const MAX_DECISIONS: usize = 1_000_000;
const MAX_SURROGATE_EVENTS: usize = 2_000_000;
const MAX_PREDICTIONS: usize = 2_000_000;
const MAX_ACTIONS_PER_DECISION: usize = 64;
const MAX_DIAGNOSTIC_TAGS: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EpisodeDisposition { Completed, Failed, Incomplete, Censored }

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OutcomeSource { ForwardSimulation, CounterfactualSimulation, ObservedPhysical }

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EpisodeRecord {
    pub episode_id: String,
    pub disposition: EpisodeDisposition,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConflictOccupancy {
    pub actor_id: String,
    pub enter_timestamp_us: u64,
    pub exit_timestamp_us: u64,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConflictOccupancyPair {
    pub actor_a: ConflictOccupancy,
    pub actor_b: ConflictOccupancy,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SurrogateEvent {
    pub event_id: String,
    pub episode_id: String,
    pub decision_id: String,
    pub actor_a_id: String,
    pub actor_b_id: String,
    pub gap_m: f64,
    pub closing_speed_mps: f64,
    pub analysis_horizon_s: f64,
    pub conflict_occupancy: Option<ConflictOccupancyPair>,
    pub measurement_source_revision: String,
    pub evidence_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OutcomePrediction {
    pub prediction_id: String,
    pub episode_id: String,
    pub decision_id: String,
    pub action_id: String,
    pub predicted_collision_probability: f64,
    pub realized_collision_within_horizon: bool,
    pub prediction_horizon_s: f64,
    pub outcome_source: OutcomeSource,
    pub prediction_model_revision: String,
    pub outcome_evidence_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DiagnosticContext {
    pub scenario_family: String,
    pub actor_class: String,
    pub road_context: String,
    pub weather: String,
    pub visibility: String,
    pub sensor_fault_family: String,
    pub decision_mode: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ActionOutcome {
    pub action_id: String,
    pub objective_cost: f64,
    pub hard_safety_violation: bool,
    pub counterfactual_evidence_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CounterfactualDecision {
    pub decision_id: String,
    pub episode_id: String,
    pub decision_tick: u64,
    pub selected_action_id: String,
    pub objective_id: String,
    pub objective_revision: String,
    pub counterfactual_evaluator_revision: String,
    pub simulator_revision: String,
    pub objective_cost_unit: String,
    pub context: DiagnosticContext,
    pub diagnostic_tags: Vec<String>,
    pub actions: Vec<ActionOutcome>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CampaignRecord {
    pub schema_version: u32,
    pub expected_episode_ids: Vec<String>,
    pub expected_decision_ids: Vec<String>,
    pub expected_surrogate_event_ids: Vec<String>,
    pub expected_prediction_ids: Vec<String>,
    pub episodes: Vec<EpisodeRecord>,
    pub decisions: Vec<CounterfactualDecision>,
    pub surrogate_events: Vec<SurrogateEvent>,
    pub outcome_predictions: Vec<OutcomePrediction>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditDisposition { Valid, Rejected }

#[derive(Debug, Clone, Serialize)]
pub struct CampaignSummary {
    pub expected_episodes: usize,
    pub observed_episodes: usize,
    pub completed_episodes: usize,
    pub failed_episodes: usize,
    pub incomplete_episodes: usize,
    pub censored_episodes: usize,
    pub expected_decisions: usize,
    pub observed_decisions: usize,
    pub surrogate_events: usize,
    pub outcome_predictions: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TtcStatus { AlreadyOverlapping, NotClosing, WithinHorizon, BeyondHorizon }

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct TtcResult {
    pub status: TtcStatus,
    pub time_to_collision_s: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DracStatus { AlreadyOverlapping, NoPositiveClosingSpeed, Finite }

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct DracResult {
    pub status: DracStatus,
    pub required_deceleration_mps2: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PetResult {
    pub post_encroachment_time_s: f64,
    pub first_actor_id: String,
    pub second_actor_id: String,
    pub conflict_zone_occupancy_overlapped: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Tier1Result {
    pub event_id: String,
    pub episode_id: String,
    pub decision_id: String,
    pub actor_a_id: String,
    pub actor_b_id: String,
    pub ttc: TtcResult,
    pub drac: DracResult,
    pub pet: Option<PetResult>,
    pub measurement_source_revision: String,
    pub evidence_sha256: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CalibrationBin {
    pub bin_index: usize,
    pub lower_probability_inclusive: f64,
    pub upper_probability_exclusive: f64,
    pub sample_count: usize,
    pub mean_predicted_probability: f64,
    pub observed_collision_rate: f64,
    pub absolute_calibration_error: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Tier2SourceMetrics {
    pub outcome_source: OutcomeSource,
    pub prediction_model_revision: String,
    pub prediction_horizon_s: f64,
    pub sample_count: usize,
    pub brier_score: f64,
    pub log_loss: f64,
    pub expected_calibration_error: f64,
    pub calibration_bins: Vec<CalibrationBin>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Tier3Result {
    pub decision_id: String,
    pub episode_id: String,
    pub selected_action_id: String,
    pub best_feasible_action_id: Option<String>,
    pub selected_action_hard_safety_violation: bool,
    pub avoidable_hard_safety_violation: bool,
    pub no_feasible_action_available: bool,
    pub feasible_regret: Option<f64>,
    pub selected_cost_minus_best_feasible_cost: Option<f64>,
    pub selected_objective_cost: f64,
    pub best_feasible_objective_cost: Option<f64>,
    pub objective_id: String,
    pub objective_revision: String,
    pub counterfactual_evaluator_revision: String,
    pub simulator_revision: String,
    pub objective_cost_unit: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct DiagnosticKey {
    scenario_family: String,
    actor_class: String,
    road_context: String,
    weather: String,
    visibility: String,
    sensor_fault_family: String,
    decision_mode: String,
    objective_id: String,
    objective_revision: String,
    counterfactual_evaluator_revision: String,
    simulator_revision: String,
    objective_cost_unit: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Tier4Result {
    pub scenario_family: String,
    pub actor_class: String,
    pub road_context: String,
    pub weather: String,
    pub visibility: String,
    pub sensor_fault_family: String,
    pub decision_mode: String,
    pub objective_id: String,
    pub objective_revision: String,
    pub counterfactual_evaluator_revision: String,
    pub simulator_revision: String,
    pub objective_cost_unit: String,
    pub decision_count: usize,
    pub avoidable_hard_safety_violation_count: usize,
    pub no_feasible_action_count: usize,
    pub mean_feasible_regret: Option<f64>,
    pub max_feasible_regret: Option<f64>,
    pub mean_selected_cost_minus_best_feasible_cost: Option<f64>,
    pub surrogate_event_count: usize,
    pub min_ttc_s: Option<f64>,
    pub max_drac_mps2: Option<f64>,
    pub min_pet_s: Option<f64>,
    pub prediction_count: usize,
    pub mean_brier_score: Option<f64>,
    pub diagnostic_tag_counts: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DecisionQualityReport {
    pub schema_version: u32,
    pub disposition: AuditDisposition,
    pub summary: CampaignSummary,
    pub tier1_surrogate_safety: Vec<Tier1Result>,
    pub tier2_outcome_prediction: Vec<Tier2SourceMetrics>,
    pub tier3_counterfactual_decision_quality: Vec<Tier3Result>,
    pub tier4_systematic_weakness_diagnosis: Vec<Tier4Result>,
    pub findings: Vec<String>,
    pub claim_ceiling: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PetError { SameActor, EmptyActorId, InvalidOccupancyInterval }

impl std::fmt::Display for PetError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SameActor => formatter.write_str("PET requires two distinct actors"),
            Self::EmptyActorId => formatter.write_str("PET actor IDs must be non-empty"),
            Self::InvalidOccupancyInterval => formatter.write_str("conflict occupancy exit must be after entry"),
        }
    }
}

impl std::error::Error for PetError {}

/// Calculate TTC for a same-conflict-path gap and positive closing speed.
/// gap_m and closing_speed_mps must be measured along the same axis.
pub fn compute_ttc(gap_m: f64, closing_speed_mps: f64, horizon_s: f64) -> Result<TtcResult, String> {
    if !gap_m.is_finite() || gap_m < 0.0 { return Err("gap_m must be finite and non-negative".into()); }
    if !closing_speed_mps.is_finite() { return Err("closing_speed_mps must be finite".into()); }
    if !horizon_s.is_finite() || horizon_s <= 0.0 { return Err("horizon_s must be finite and positive".into()); }
    if gap_m <= f64::EPSILON {
        return Ok(TtcResult { status: TtcStatus::AlreadyOverlapping, time_to_collision_s: Some(0.0) });
    }
    if closing_speed_mps <= 0.0 {
        return Ok(TtcResult { status: TtcStatus::NotClosing, time_to_collision_s: None });
    }
    let ttc = gap_m / closing_speed_mps;
    if !ttc.is_finite() { return Err("TTC computation overflowed".into()); }
    Ok(TtcResult {
        status: if ttc <= horizon_s { TtcStatus::WithinHorizon } else { TtcStatus::BeyondHorizon },
        time_to_collision_s: Some(ttc),
    })
}

/// Required longitudinal deceleration under constant relative closing speed:
/// DRAC = closing_speed^2 / (2 * gap). This is not a full vehicle-dynamics model.
pub fn compute_drac(gap_m: f64, closing_speed_mps: f64) -> Result<DracResult, String> {
    if !gap_m.is_finite() || gap_m < 0.0 { return Err("gap_m must be finite and non-negative".into()); }
    if !closing_speed_mps.is_finite() { return Err("closing_speed_mps must be finite".into()); }
    if gap_m <= f64::EPSILON {
        return Ok(DracResult { status: DracStatus::AlreadyOverlapping, required_deceleration_mps2: None });
    }
    if closing_speed_mps <= 0.0 {
        return Ok(DracResult { status: DracStatus::NoPositiveClosingSpeed, required_deceleration_mps2: Some(0.0) });
    }
    let drac = closing_speed_mps * closing_speed_mps / (2.0 * gap_m);
    if !drac.is_finite() { return Err("DRAC computation overflowed".into()); }
    Ok(DracResult { status: DracStatus::Finite, required_deceleration_mps2: Some(drac) })
}

/// Calculate PET from two conflict-zone occupancy intervals. Overlap is
/// represented as PET=0 and reported separately.
pub fn compute_pet(pair: &ConflictOccupancyPair) -> Result<PetResult, PetError> {
    let a = &pair.actor_a;
    let b = &pair.actor_b;
    if a.actor_id.trim().is_empty() || b.actor_id.trim().is_empty() { return Err(PetError::EmptyActorId); }
    if a.actor_id == b.actor_id { return Err(PetError::SameActor); }
    if a.exit_timestamp_us <= a.enter_timestamp_us || b.exit_timestamp_us <= b.enter_timestamp_us {
        return Err(PetError::InvalidOccupancyInterval);
    }
    if a.exit_timestamp_us <= b.enter_timestamp_us {
        Ok(PetResult {
            post_encroachment_time_s: (b.enter_timestamp_us - a.exit_timestamp_us) as f64 / 1_000_000.0,
            first_actor_id: a.actor_id.clone(),
            second_actor_id: b.actor_id.clone(),
            conflict_zone_occupancy_overlapped: false,
        })
    } else if b.exit_timestamp_us <= a.enter_timestamp_us {
        Ok(PetResult {
            post_encroachment_time_s: (a.enter_timestamp_us - b.exit_timestamp_us) as f64 / 1_000_000.0,
            first_actor_id: b.actor_id.clone(),
            second_actor_id: a.actor_id.clone(),
            conflict_zone_occupancy_overlapped: false,
        })
    } else {
        let (first_actor_id, second_actor_id) = if a.enter_timestamp_us <= b.enter_timestamp_us {
            (a.actor_id.clone(), b.actor_id.clone())
        } else {
            (b.actor_id.clone(), a.actor_id.clone())
        };
        Ok(PetResult {
            post_encroachment_time_s: 0.0,
            first_actor_id,
            second_actor_id,
            conflict_zone_occupancy_overlapped: true,
        })
    }
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
fn nonempty(value: &str) -> bool { !value.trim().is_empty() }
fn has_duplicates(values: &[String]) -> bool {
    let mut sorted = values.to_vec();
    sorted.sort();
    sorted.windows(2).any(|pair| pair[0] == pair[1])
}
fn set_difference(expected: &[String], observed: &[String], label: &str, errors: &mut Vec<String>) {
    let expected_set = expected.iter().cloned().collect::<BTreeSet<_>>();
    let observed_set = observed.iter().cloned().collect::<BTreeSet<_>>();
    for missing in expected_set.difference(&observed_set) { errors.push(format!("missing {label}: {missing}")); }
    for unexpected in observed_set.difference(&expected_set) { errors.push(format!("unexpected {label}: {unexpected}")); }
}

fn validate_campaign(campaign: &CampaignRecord) -> Vec<String> {
    let mut errors = Vec::new();
    if campaign.schema_version != SCHEMA_VERSION {
        errors.push(format!("schema_version must be {SCHEMA_VERSION}, got {}", campaign.schema_version));
    }
    for (label, ids) in [
        ("expected_episode_ids", &campaign.expected_episode_ids),
        ("expected_decision_ids", &campaign.expected_decision_ids),
        ("expected_surrogate_event_ids", &campaign.expected_surrogate_event_ids),
        ("expected_prediction_ids", &campaign.expected_prediction_ids),
    ] {
        if ids.iter().any(|id| !nonempty(id)) { errors.push(format!("{label} contains an empty ID")); }
        if has_duplicates(ids) { errors.push(format!("{label} contains duplicate IDs")); }
    }
    for (label, count, limit) in [
        ("episodes", campaign.episodes.len(), MAX_EPISODES),
        ("decisions", campaign.decisions.len(), MAX_DECISIONS),
        ("surrogate_events", campaign.surrogate_events.len(), MAX_SURROGATE_EVENTS),
        ("outcome_predictions", campaign.outcome_predictions.len(), MAX_PREDICTIONS),
    ] {
        if count > limit { errors.push(format!("{label} exceeds bound {limit}")); }
    }

    let episode_ids = campaign.episodes.iter().map(|x| x.episode_id.clone()).collect::<Vec<_>>();
    let decision_ids = campaign.decisions.iter().map(|x| x.decision_id.clone()).collect::<Vec<_>>();
    let event_ids = campaign.surrogate_events.iter().map(|x| x.event_id.clone()).collect::<Vec<_>>();
    let prediction_ids = campaign.outcome_predictions.iter().map(|x| x.prediction_id.clone()).collect::<Vec<_>>();
    for (label, ids) in [
        ("episode_id", &episode_ids), ("decision_id", &decision_ids),
        ("surrogate_event_id", &event_ids), ("prediction_id", &prediction_ids),
    ] {
        if ids.iter().any(|id| !nonempty(id)) { errors.push(format!("observed {label} contains an empty ID")); }
        if has_duplicates(ids) { errors.push(format!("observed {label} contains duplicates")); }
    }
    set_difference(&campaign.expected_episode_ids, &episode_ids, "episodes", &mut errors);
    set_difference(&campaign.expected_decision_ids, &decision_ids, "decisions", &mut errors);
    set_difference(&campaign.expected_surrogate_event_ids, &event_ids, "surrogate events", &mut errors);
    set_difference(&campaign.expected_prediction_ids, &prediction_ids, "predictions", &mut errors);

    let episodes = episode_ids.iter().cloned().collect::<BTreeSet<_>>();
    let decisions = campaign.decisions.iter().map(|d| (d.decision_id.as_str(), d)).collect::<BTreeMap<_, _>>();

    for decision in &campaign.decisions {
        if !episodes.contains(&decision.episode_id) {
            errors.push(format!("decision {} references missing episode {}", decision.decision_id, decision.episode_id));
        }
        for (label, value) in [
            ("objective_id", decision.objective_id.as_str()),
            ("objective_revision", decision.objective_revision.as_str()),
            ("counterfactual_evaluator_revision", decision.counterfactual_evaluator_revision.as_str()),
            ("simulator_revision", decision.simulator_revision.as_str()),
            ("objective_cost_unit", decision.objective_cost_unit.as_str()),
            ("scenario_family", decision.context.scenario_family.as_str()),
            ("actor_class", decision.context.actor_class.as_str()),
            ("road_context", decision.context.road_context.as_str()),
            ("weather", decision.context.weather.as_str()),
            ("visibility", decision.context.visibility.as_str()),
            ("sensor_fault_family", decision.context.sensor_fault_family.as_str()),
            ("decision_mode", decision.context.decision_mode.as_str()),
        ] {
            if !nonempty(value) { errors.push(format!("decision {} has empty {label}", decision.decision_id)); }
        }
        if decision.actions.is_empty() || decision.actions.len() > MAX_ACTIONS_PER_DECISION {
            errors.push(format!("decision {} must contain 1..={} action outcomes", decision.decision_id, MAX_ACTIONS_PER_DECISION));
        }
        let action_ids = decision.actions.iter().map(|a| a.action_id.clone()).collect::<Vec<_>>();
        if action_ids.iter().any(|id| !nonempty(id)) || has_duplicates(&action_ids) {
            errors.push(format!("decision {} has empty or duplicate action IDs", decision.decision_id));
        }
        if !action_ids.contains(&decision.selected_action_id) {
            errors.push(format!("decision {} selected action {} is absent from candidate outcomes", decision.decision_id, decision.selected_action_id));
        }
        if decision.diagnostic_tags.len() > MAX_DIAGNOSTIC_TAGS
            || decision.diagnostic_tags.iter().any(|tag| !nonempty(tag))
            || has_duplicates(&decision.diagnostic_tags)
        {
            errors.push(format!("decision {} has invalid diagnostic tags", decision.decision_id));
        }
        for action in &decision.actions {
            if !action.objective_cost.is_finite() {
                errors.push(format!("decision {} action {} has non-finite objective cost", decision.decision_id, action.action_id));
            }
            if !valid_sha256(&action.counterfactual_evidence_sha256) {
                errors.push(format!("decision {} action {} has invalid counterfactual evidence SHA-256", decision.decision_id, action.action_id));
            }
        }
    }

    for event in &campaign.surrogate_events {
        if !episodes.contains(&event.episode_id) {
            errors.push(format!("surrogate event {} references missing episode {}", event.event_id, event.episode_id));
        }
        match decisions.get(event.decision_id.as_str()) {
            Some(decision) if decision.episode_id == event.episode_id => {}
            Some(_) => errors.push(format!("surrogate event {} decision/episode identity mismatch", event.event_id)),
            None => errors.push(format!("surrogate event {} references missing decision {}", event.event_id, event.decision_id)),
        }
        if !nonempty(&event.actor_a_id) || !nonempty(&event.actor_b_id) || event.actor_a_id == event.actor_b_id {
            errors.push(format!("surrogate event {} has invalid actor IDs", event.event_id));
        }
        if !event.gap_m.is_finite() || event.gap_m < 0.0
            || !event.closing_speed_mps.is_finite()
            || !event.analysis_horizon_s.is_finite() || event.analysis_horizon_s <= 0.0
        {
            errors.push(format!("surrogate event {} has invalid metric inputs", event.event_id));
        }
        if !nonempty(&event.measurement_source_revision) || !valid_sha256(&event.evidence_sha256) {
            errors.push(format!("surrogate event {} has incomplete measurement provenance", event.event_id));
        }
        if let Some(pair) = &event.conflict_occupancy {
            if pair.actor_a.actor_id != event.actor_a_id || pair.actor_b.actor_id != event.actor_b_id {
                errors.push(format!("surrogate event {} PET actor identity mismatch", event.event_id));
            }
            if pair.actor_a.exit_timestamp_us <= pair.actor_a.enter_timestamp_us
                || pair.actor_b.exit_timestamp_us <= pair.actor_b.enter_timestamp_us
            {
                errors.push(format!("surrogate event {} PET occupancy interval is invalid", event.event_id));
            }
        }
    }

    for prediction in &campaign.outcome_predictions {
        if !episodes.contains(&prediction.episode_id) {
            errors.push(format!("prediction {} references missing episode {}", prediction.prediction_id, prediction.episode_id));
        }
        match decisions.get(prediction.decision_id.as_str()) {
            Some(decision) if decision.episode_id == prediction.episode_id => {
                if !decision.actions.iter().any(|a| a.action_id == prediction.action_id) {
                    errors.push(format!("prediction {} references action {} absent from decision", prediction.prediction_id, prediction.action_id));
                }
            }
            Some(_) => errors.push(format!("prediction {} decision/episode identity mismatch", prediction.prediction_id)),
            None => errors.push(format!("prediction {} references missing decision {}", prediction.prediction_id, prediction.decision_id)),
        }
        if !prediction.predicted_collision_probability.is_finite()
            || !(0.0..=1.0).contains(&prediction.predicted_collision_probability)
            || !prediction.prediction_horizon_s.is_finite()
            || prediction.prediction_horizon_s <= 0.0
        {
            errors.push(format!("prediction {} has invalid probability or horizon", prediction.prediction_id));
        }
        if !nonempty(&prediction.prediction_model_revision) || !valid_sha256(&prediction.outcome_evidence_sha256) {
            errors.push(format!("prediction {} has incomplete outcome provenance", prediction.prediction_id));
        }
    }
    errors.sort();
    errors.dedup();
    errors
}

fn calculate_tier1(campaign: &CampaignRecord) -> Vec<Tier1Result> {
    let mut results = Vec::with_capacity(campaign.surrogate_events.len());
    for event in &campaign.surrogate_events {
        let ttc = compute_ttc(event.gap_m, event.closing_speed_mps, event.analysis_horizon_s)
            .expect("validated TTC inputs must be calculable");
        let drac = compute_drac(event.gap_m, event.closing_speed_mps)
            .expect("validated DRAC inputs must be calculable");
        let pet = event.conflict_occupancy.as_ref().map(compute_pet).transpose()
            .expect("validated PET inputs must be calculable");
        results.push(Tier1Result {
            event_id: event.event_id.clone(),
            episode_id: event.episode_id.clone(),
            decision_id: event.decision_id.clone(),
            actor_a_id: event.actor_a_id.clone(),
            actor_b_id: event.actor_b_id.clone(),
            ttc, drac, pet,
            measurement_source_revision: event.measurement_source_revision.clone(),
            evidence_sha256: event.evidence_sha256.clone(),
        });
    }
    results.sort_by(|a, b| a.event_id.cmp(&b.event_id));
    results
}

fn calculate_tier2(campaign: &CampaignRecord) -> Vec<Tier2SourceMetrics> {
    let mut grouped = BTreeMap::<(OutcomeSource, String, u64), Vec<&OutcomePrediction>>::new();
    for prediction in &campaign.outcome_predictions {
        grouped
            .entry((
                prediction.outcome_source,
                prediction.prediction_model_revision.clone(),
                prediction.prediction_horizon_s.to_bits(),
            ))
            .or_default()
            .push(prediction);
    }
    let mut results = Vec::with_capacity(grouped.len());
    for ((source, model_revision, horizon_bits), predictions) in grouped {
        let count = predictions.len();
        let mut brier_sum = 0.0;
        let mut log_loss_sum = 0.0;
        let mut bins = vec![Vec::<&OutcomePrediction>::new(); DEFAULT_CALIBRATION_BINS];
        for prediction in &predictions {
            let p = prediction.predicted_collision_probability;
            let actual = if prediction.realized_collision_within_horizon { 1.0 } else { 0.0 };
            let error = p - actual;
            brier_sum += error * error;
            let clipped = p.clamp(1e-15, 1.0 - 1e-15);
            log_loss_sum -= actual * clipped.ln() + (1.0 - actual) * (1.0 - clipped).ln();
            let index = ((p * DEFAULT_CALIBRATION_BINS as f64).floor() as usize)
                .min(DEFAULT_CALIBRATION_BINS - 1);
            bins[index].push(*prediction);
        }
        let mut calibration_bins = Vec::new();
        let mut ece = 0.0;
        for (index, samples) in bins.into_iter().enumerate() {
            if samples.is_empty() { continue; }
            let n = samples.len();
            let mean_predicted = samples.iter()
                .map(|sample| sample.predicted_collision_probability)
                .sum::<f64>() / n as f64;
            let observed_rate = samples.iter()
                .filter(|sample| sample.realized_collision_within_horizon)
                .count() as f64 / n as f64;
            let absolute_error = (mean_predicted - observed_rate).abs();
            ece += n as f64 / count as f64 * absolute_error;
            calibration_bins.push(CalibrationBin {
                bin_index: index,
                lower_probability_inclusive: index as f64 / DEFAULT_CALIBRATION_BINS as f64,
                upper_probability_exclusive: (index + 1) as f64 / DEFAULT_CALIBRATION_BINS as f64,
                sample_count: n,
                mean_predicted_probability: mean_predicted,
                observed_collision_rate: observed_rate,
                absolute_calibration_error: absolute_error,
            });
        }
        results.push(Tier2SourceMetrics {
            outcome_source: source,
            prediction_model_revision: model_revision,
            prediction_horizon_s: f64::from_bits(horizon_bits),
            sample_count: count,
            brier_score: brier_sum / count as f64,
            log_loss: log_loss_sum / count as f64,
            expected_calibration_error: ece,
            calibration_bins,
        });
    }
    results
}

fn calculate_tier3(campaign: &CampaignRecord) -> Vec<Tier3Result> {
    let mut results = Vec::with_capacity(campaign.decisions.len());
    for decision in &campaign.decisions {
        let selected = decision.actions.iter().find(|a| a.action_id == decision.selected_action_id)
            .expect("validated selected action must exist");
        let best = decision.actions.iter().filter(|a| !a.hard_safety_violation)
            .min_by(|a, b| a.objective_cost.total_cmp(&b.objective_cost));
        let selected_feasible = !selected.hard_safety_violation;
        let selected_gap = best.map(|b| selected.objective_cost - b.objective_cost);
        let feasible_regret = if selected_feasible {
            selected_gap.map(|gap| if gap.abs() <= 1e-12 { 0.0 } else { gap })
        } else { None };
        results.push(Tier3Result {
            decision_id: decision.decision_id.clone(),
            episode_id: decision.episode_id.clone(),
            selected_action_id: decision.selected_action_id.clone(),
            best_feasible_action_id: best.map(|a| a.action_id.clone()),
            selected_action_hard_safety_violation: !selected_feasible,
            avoidable_hard_safety_violation: !selected_feasible && best.is_some(),
            no_feasible_action_available: best.is_none(),
            feasible_regret,
            selected_cost_minus_best_feasible_cost: selected_gap,
            selected_objective_cost: selected.objective_cost,
            best_feasible_objective_cost: best.map(|a| a.objective_cost),
            objective_id: decision.objective_id.clone(),
            objective_revision: decision.objective_revision.clone(),
            counterfactual_evaluator_revision: decision.counterfactual_evaluator_revision.clone(),
            simulator_revision: decision.simulator_revision.clone(),
            objective_cost_unit: decision.objective_cost_unit.clone(),
        });
    }
    results.sort_by(|a, b| a.decision_id.cmp(&b.decision_id));
    results
}

#[derive(Default)]
struct DiagnosticAccumulator {
    decision_count: usize,
    avoidable_count: usize,
    no_feasible_count: usize,
    feasible_regrets: Vec<f64>,
    selected_cost_gaps: Vec<f64>,
    surrogate_event_count: usize,
    min_ttc: Option<f64>,
    max_drac: Option<f64>,
    min_pet: Option<f64>,
    prediction_briers: Vec<f64>,
    diagnostic_tag_counts: BTreeMap<String, usize>,
}

fn calculate_tier4(campaign: &CampaignRecord, tier1: &[Tier1Result], tier3: &[Tier3Result]) -> Vec<Tier4Result> {
    let decisions = campaign.decisions.iter().map(|d| (d.decision_id.as_str(), d)).collect::<BTreeMap<_, _>>();
    let tier3_by_id = tier3.iter().map(|r| (r.decision_id.as_str(), r)).collect::<BTreeMap<_, _>>();
    let key_for = |decision: &CounterfactualDecision| DiagnosticKey {
        scenario_family: decision.context.scenario_family.clone(),
        actor_class: decision.context.actor_class.clone(),
        road_context: decision.context.road_context.clone(),
        weather: decision.context.weather.clone(),
        visibility: decision.context.visibility.clone(),
        sensor_fault_family: decision.context.sensor_fault_family.clone(),
        decision_mode: decision.context.decision_mode.clone(),
        objective_id: decision.objective_id.clone(),
        objective_revision: decision.objective_revision.clone(),
        counterfactual_evaluator_revision: decision.counterfactual_evaluator_revision.clone(),
        simulator_revision: decision.simulator_revision.clone(),
        objective_cost_unit: decision.objective_cost_unit.clone(),
    };
    let mut grouped = BTreeMap::<DiagnosticKey, DiagnosticAccumulator>::new();
    for (id, decision) in &decisions {
        let aggregate = grouped.entry(key_for(decision)).or_default();
        aggregate.decision_count += 1;
        let result = tier3_by_id[id];
        if result.avoidable_hard_safety_violation { aggregate.avoidable_count += 1; }
        if result.no_feasible_action_available { aggregate.no_feasible_count += 1; }
        if let Some(regret) = result.feasible_regret { aggregate.feasible_regrets.push(regret); }
        if let Some(gap) = result.selected_cost_minus_best_feasible_cost { aggregate.selected_cost_gaps.push(gap); }
        for tag in &decision.diagnostic_tags {
            *aggregate.diagnostic_tag_counts.entry(tag.clone()).or_default() += 1;
        }
    }
    for event in tier1 {
        let Some(decision) = decisions.get(event.decision_id.as_str()) else { continue; };
        let aggregate = grouped.entry(key_for(decision)).or_default();
        aggregate.surrogate_event_count += 1;
        if let Some(ttc) = event.ttc.time_to_collision_s {
            aggregate.min_ttc = Some(aggregate.min_ttc.map_or(ttc, |v| v.min(ttc)));
        }
        if let Some(drac) = event.drac.required_deceleration_mps2 {
            aggregate.max_drac = Some(aggregate.max_drac.map_or(drac, |v| v.max(drac)));
        }
        if let Some(pet) = &event.pet {
            aggregate.min_pet = Some(aggregate.min_pet.map_or(pet.post_encroachment_time_s, |v| v.min(pet.post_encroachment_time_s)));
        }
    }
    for prediction in &campaign.outcome_predictions {
        let Some(decision) = decisions.get(prediction.decision_id.as_str()) else { continue; };
        let aggregate = grouped.entry(key_for(&decision.context)).or_default();
        let actual = if prediction.realized_collision_within_horizon { 1.0 } else { 0.0 };
        let error = prediction.predicted_collision_probability - actual;
        aggregate.prediction_briers.push(error * error);
    }
    grouped.into_iter().map(|(key, a)| Tier4Result {
        scenario_family: key.scenario_family,
        actor_class: key.actor_class,
        road_context: key.road_context,
        weather: key.weather,
        visibility: key.visibility,
        sensor_fault_family: key.sensor_fault_family,
        decision_mode: key.decision_mode,
        objective_id: key.objective_id,
        objective_revision: key.objective_revision,
        counterfactual_evaluator_revision: key.counterfactual_evaluator_revision,
        simulator_revision: key.simulator_revision,
        objective_cost_unit: key.objective_cost_unit,
        decision_count: a.decision_count,
        avoidable_hard_safety_violation_count: a.avoidable_count,
        no_feasible_action_count: a.no_feasible_count,
        mean_feasible_regret: mean(&a.feasible_regrets),
        max_feasible_regret: a.feasible_regrets.iter().copied().reduce(f64::max),
        mean_selected_cost_minus_best_feasible_cost: mean(&a.selected_cost_gaps),
        surrogate_event_count: a.surrogate_event_count,
        min_ttc_s: a.min_ttc,
        max_drac_mps2: a.max_drac,
        min_pet_s: a.min_pet,
        prediction_count: a.prediction_briers.len(),
        mean_brier_score: mean(&a.prediction_briers),
        diagnostic_tag_counts: a.diagnostic_tag_counts,
    }).collect()
}

fn mean(values: &[f64]) -> Option<f64> {
    if values.is_empty() { None } else { Some(values.iter().sum::<f64>() / values.len() as f64) }
}

fn summary(campaign: &CampaignRecord) -> CampaignSummary {
    CampaignSummary {
        expected_episodes: campaign.expected_episode_ids.len(),
        observed_episodes: campaign.episodes.len(),
        completed_episodes: campaign.episodes.iter().filter(|e| e.disposition == EpisodeDisposition::Completed).count(),
        failed_episodes: campaign.episodes.iter().filter(|e| e.disposition == EpisodeDisposition::Failed).count(),
        incomplete_episodes: campaign.episodes.iter().filter(|e| e.disposition == EpisodeDisposition::Incomplete).count(),
        censored_episodes: campaign.episodes.iter().filter(|e| e.disposition == EpisodeDisposition::Censored).count(),
        expected_decisions: campaign.expected_decision_ids.len(),
        observed_decisions: campaign.decisions.len(),
        surrogate_events: campaign.surrogate_events.len(),
        outcome_predictions: campaign.outcome_predictions.len(),
    }
}

pub fn evaluate_campaign(campaign: &CampaignRecord) -> DecisionQualityReport {
    let summary = summary(campaign);
    let findings = validate_campaign(campaign);
    if !findings.is_empty() {
        return DecisionQualityReport {
            schema_version: SCHEMA_VERSION,
            disposition: AuditDisposition::Rejected,
            summary,
            tier1_surrogate_safety: Vec::new(),
            tier2_outcome_prediction: Vec::new(),
            tier3_counterfactual_decision_quality: Vec::new(),
            tier4_systematic_weakness_diagnosis: Vec::new(),
            findings,
            claim_ceiling: claim_ceiling(),
        };
    }
    let tier1 = calculate_tier1(campaign);
    let tier2 = calculate_tier2(campaign);
    let tier3 = calculate_tier3(campaign);
    let tier4 = calculate_tier4(campaign, &tier1, &tier3);
    DecisionQualityReport {
        schema_version: SCHEMA_VERSION,
        disposition: AuditDisposition::Valid,
        summary,
        tier1_surrogate_safety: tier1,
        tier2_outcome_prediction: tier2,
        tier3_counterfactual_decision_quality: tier3,
        tier4_systematic_weakness_diagnosis: tier4,
        findings: Vec::new(),
        claim_ceiling: claim_ceiling(),
    }
}

fn claim_ceiling() -> String {
    "Finite black-box decision-quality records only. This evaluator does not run a simulator or ADS, authenticate physical evidence, prove counterfactual truth, establish causal correctness, certify an automated driving system, or establish roadworthiness.".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(ch: char) -> String { std::iter::repeat_n(ch, 64).collect() }

    fn fixture() -> CampaignRecord {
        CampaignRecord {
            schema_version: 1,
            expected_episode_ids: vec!["episode-1".into()],
            expected_decision_ids: vec!["decision-1".into()],
            expected_surrogate_event_ids: vec!["event-1".into()],
            expected_prediction_ids: vec!["prediction-1".into(), "prediction-2".into()],
            episodes: vec![EpisodeRecord { episode_id: "episode-1".into(), disposition: EpisodeDisposition::Completed }],
            decisions: vec![CounterfactualDecision {
                decision_id: "decision-1".into(),
                episode_id: "episode-1".into(),
                decision_tick: 10,
                selected_action_id: "accelerate".into(),
                objective_id: "safety-progress-v1".into(),
                objective_revision: "objective-rev-1".into(),
                counterfactual_evaluator_revision: "counterfactual-evaluator-v1".into(),
                simulator_revision: "sim-snapshot-v1".into(),
                objective_cost_unit: "declared_cost_units".into(),
                context: DiagnosticContext {
                    scenario_family: "pedestrian_crossing".into(),
                    actor_class: "pedestrian".into(),
                    road_context: "urban_intersection".into(),
                    weather: "clear".into(),
                    visibility: "daylight".into(),
                    sensor_fault_family: "none".into(),
                    decision_mode: "yield_or_proceed".into(),
                },
                diagnostic_tags: vec!["yielding_gap_misjudgment".into()],
                actions: vec![
                    ActionOutcome { action_id: "accelerate".into(), objective_cost: 0.5, hard_safety_violation: true, counterfactual_evidence_sha256: digest('a') },
                    ActionOutcome { action_id: "yield".into(), objective_cost: 2.0, hard_safety_violation: false, counterfactual_evidence_sha256: digest('b') },
                ],
            }],
            surrogate_events: vec![SurrogateEvent {
                event_id: "event-1".into(),
                episode_id: "episode-1".into(),
                decision_id: "decision-1".into(),
                actor_a_id: "ego".into(),
                actor_b_id: "pedestrian".into(),
                gap_m: 8.0,
                closing_speed_mps: 4.0,
                analysis_horizon_s: 5.0,
                conflict_occupancy: Some(ConflictOccupancyPair {
                    actor_a: ConflictOccupancy { actor_id: "ego".into(), enter_timestamp_us: 1_000_000, exit_timestamp_us: 1_200_000 },
                    actor_b: ConflictOccupancy { actor_id: "pedestrian".into(), enter_timestamp_us: 1_500_000, exit_timestamp_us: 1_800_000 },
                }),
                measurement_source_revision: "surrogate-measurement-v1".into(),
                evidence_sha256: digest('c'),
            }],
            outcome_predictions: vec![
                OutcomePrediction {
                    prediction_id: "prediction-1".into(), episode_id: "episode-1".into(),
                    decision_id: "decision-1".into(), action_id: "yield".into(),
                    predicted_collision_probability: 0.1, realized_collision_within_horizon: false,
                    prediction_horizon_s: 3.0, outcome_source: OutcomeSource::ForwardSimulation,
                    prediction_model_revision: "risk-model-v1".into(), outcome_evidence_sha256: digest('d'),
                },
                OutcomePrediction {
                    prediction_id: "prediction-2".into(), episode_id: "episode-1".into(),
                    decision_id: "decision-1".into(), action_id: "accelerate".into(),
                    predicted_collision_probability: 0.8, realized_collision_within_horizon: true,
                    prediction_horizon_s: 3.0, outcome_source: OutcomeSource::ForwardSimulation,
                    prediction_model_revision: "risk-model-v1".into(), outcome_evidence_sha256: digest('e'),
                },
            ],
        }
    }

    #[test]
    fn ttc_handles_closing_non_closing_overlap_and_horizon() {
        let within = compute_ttc(8.0, 4.0, 3.0).unwrap();
        assert_eq!(within.status, TtcStatus::WithinHorizon);
        assert_eq!(within.time_to_collision_s, Some(2.0));
        let beyond = compute_ttc(20.0, 2.0, 5.0).unwrap();
        assert_eq!(beyond.status, TtcStatus::BeyondHorizon);
        assert_eq!(beyond.time_to_collision_s, Some(10.0));
        assert_eq!(compute_ttc(8.0, -1.0, 3.0).unwrap().status, TtcStatus::NotClosing);
        assert_eq!(compute_ttc(0.0, 2.0, 3.0).unwrap().status, TtcStatus::AlreadyOverlapping);
        assert!(compute_ttc(-1.0, 2.0, 3.0).is_err());
    }

    #[test]
    fn drac_matches_constant_speed_formula_and_edge_cases() {
        let result = compute_drac(8.0, 4.0).unwrap();
        assert_eq!(result.status, DracStatus::Finite);
        assert_eq!(result.required_deceleration_mps2, Some(1.0));
        assert_eq!(compute_drac(8.0, 0.0).unwrap().required_deceleration_mps2, Some(0.0));
        assert_eq!(compute_drac(0.0, 2.0).unwrap().status, DracStatus::AlreadyOverlapping);
    }

    #[test]
    fn pet_uses_conflict_zone_exit_to_next_entry_and_detects_overlap() {
        let pair = ConflictOccupancyPair {
            actor_a: ConflictOccupancy { actor_id: "a".into(), enter_timestamp_us: 1_000_000, exit_timestamp_us: 1_200_000 },
            actor_b: ConflictOccupancy { actor_id: "b".into(), enter_timestamp_us: 1_500_000, exit_timestamp_us: 1_800_000 },
        };
        let pet = compute_pet(&pair).unwrap();
        assert!((pet.post_encroachment_time_s - 0.3).abs() < 1e-12);
        assert!(!pet.conflict_zone_occupancy_overlapped);
        let mut overlap = pair;
        overlap.actor_b.enter_timestamp_us = 1_100_000;
        let pet = compute_pet(&overlap).unwrap();
        assert_eq!(pet.post_encroachment_time_s, 0.0);
        assert!(pet.conflict_zone_occupancy_overlapped);
    }

    #[test]
    fn pet_rejects_same_actor_and_invalid_interval() {
        let mut pair = ConflictOccupancyPair {
            actor_a: ConflictOccupancy { actor_id: "a".into(), enter_timestamp_us: 1, exit_timestamp_us: 2 },
            actor_b: ConflictOccupancy { actor_id: "a".into(), enter_timestamp_us: 3, exit_timestamp_us: 4 },
        };
        assert_eq!(compute_pet(&pair).unwrap_err(), PetError::SameActor);
        pair.actor_b.actor_id = "b".into();
        pair.actor_b.exit_timestamp_us = 3;
        assert_eq!(compute_pet(&pair).unwrap_err(), PetError::InvalidOccupancyInterval);
    }

    #[test]
    fn brier_log_loss_and_calibration_are_computed_by_source() {
        let report = evaluate_campaign(&fixture());
        assert_eq!(report.disposition, AuditDisposition::Valid);
        let metrics = &report.tier2_outcome_prediction[0];
        assert_eq!(metrics.outcome_source, OutcomeSource::ForwardSimulation);
        assert_eq!(metrics.prediction_model_revision, "risk-model-v1");
        assert_eq!(metrics.prediction_horizon_s, 3.0);
        assert_eq!(metrics.sample_count, 2);
        assert!((metrics.brier_score - 0.025).abs() < 1e-12);
        assert_eq!(metrics.calibration_bins.len(), 2);
        assert!(metrics.log_loss.is_finite());
        assert!((metrics.expected_calibration_error - 0.15).abs() < 1e-12);
    }

    #[test]
    fn unsafe_selected_action_is_avoidable_not_given_feasible_regret() {
        let report = evaluate_campaign(&fixture());
        let decision = &report.tier3_counterfactual_decision_quality[0];
        assert!(decision.selected_action_hard_safety_violation);
        assert!(decision.avoidable_hard_safety_violation);
        assert_eq!(decision.best_feasible_action_id.as_deref(), Some("yield"));
        assert_eq!(decision.feasible_regret, None);
        assert_eq!(decision.selected_cost_minus_best_feasible_cost, Some(-1.5));
    }

    #[test]
    fn selected_best_feasible_action_has_zero_regret() {
        let mut campaign = fixture();
        campaign.decisions[0].selected_action_id = "yield".into();
        let report = evaluate_campaign(&campaign);
        let decision = &report.tier3_counterfactual_decision_quality[0];
        assert!(!decision.avoidable_hard_safety_violation);
        assert_eq!(decision.feasible_regret, Some(0.0));
    }

    #[test]
    fn reports_when_no_feasible_action_exists_without_inventing_regret() {
        let mut campaign = fixture();
        for action in &mut campaign.decisions[0].actions { action.hard_safety_violation = true; }
        let report = evaluate_campaign(&campaign);
        let decision = &report.tier3_counterfactual_decision_quality[0];
        assert!(decision.no_feasible_action_available);
        assert_eq!(decision.best_feasible_action_id, None);
        assert_eq!(decision.feasible_regret, None);
    }

    #[test]
    fn tier_four_groups_repeated_weaknesses_without_hiding_denominators() {
        let report = evaluate_campaign(&fixture());
        let group = &report.tier4_systematic_weakness_diagnosis[0];
        assert_eq!(group.decision_count, 1);
        assert_eq!(group.avoidable_hard_safety_violation_count, 1);
        assert_eq!(group.surrogate_event_count, 1);
        assert_eq!(group.prediction_count, 2);
        assert_eq!(group.diagnostic_tag_counts.get("yielding_gap_misjudgment"), Some(&1));
        assert_eq!(group.min_ttc_s, Some(2.0));
        assert_eq!(group.max_drac_mps2, Some(1.0));
        assert!((group.min_pet_s.unwrap() - 0.3).abs() < 1e-12);
    }

    #[test]
    fn rejects_missing_decision_and_event_denominators() {
        let mut campaign = fixture();
        campaign.expected_decision_ids.push("decision-missing".into());
        campaign.expected_surrogate_event_ids.push("event-missing".into());
        let report = evaluate_campaign(&campaign);
        assert_eq!(report.disposition, AuditDisposition::Rejected);
        assert!(report.findings.iter().any(|f| f.contains("missing decisions")));
        assert!(report.findings.iter().any(|f| f.contains("missing surrogate events")));
        assert!(report.tier3_counterfactual_decision_quality.is_empty());
    }

    #[test]
    fn rejects_invalid_digest_and_mismatched_action_reference() {
        let mut campaign = fixture();
        campaign.decisions[0].actions[0].counterfactual_evidence_sha256 = "not-a-digest".into();
        campaign.outcome_predictions[0].action_id = "ghost-action".into();
        let report = evaluate_campaign(&campaign);
        assert_eq!(report.disposition, AuditDisposition::Rejected);
        assert!(report.findings.iter().any(|f| f.contains("counterfactual evidence SHA-256")));
        assert!(report.findings.iter().any(|f| f.contains("absent from decision")));
    }

    #[test]
    fn schema_and_example_are_parseable_and_evaluable() {
        let example = include_str!("../examples/analytic-fixture-v1.json");
        let campaign: CampaignRecord = serde_json::from_str(example).unwrap();
        let report = evaluate_campaign(&campaign);
        assert_eq!(report.disposition, AuditDisposition::Valid);
        let schema: serde_json::Value = serde_json::from_str(include_str!("../schemas/decision-quality-campaign-v1.schema.json")).unwrap();
        assert_eq!(schema["properties"]["schema_version"]["const"], 1);
        assert!(report.claim_ceiling.contains("does not run a simulator"));
    }
}
