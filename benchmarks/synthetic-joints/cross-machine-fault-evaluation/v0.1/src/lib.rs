#![forbid(unsafe_code)]

//! Episode-level cross-machine and compound-fault benchmark evaluator.
//!
//! This M1 crate validates frozen evaluation records and computes event,
//! multi-label, false-alarm, detection-latency, abstention and calibration
//! metrics. It does not train a model or inspect raw vibration/servo signals.
//!
//! Key units:
//! - independent physical specimen / complete drive identity;
//! - operating episode with a labelled event onset (if present);
//! - prediction window nested inside the episode;
//! - healthy exposure explicitly recorded in verified seconds.
//!
//! Sliding windows are never counted as independent physical specimens.

use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ChannelState {
    Healthy,
    Missing,
    PartialCorruption,
    Stuck,
    Drifting,
    Saturated,
    IntermittentLoss,
    Delayed,
    Invalid,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimebaseStatus {
    Synchronized,
    Unmapped,
    Unknown,
    Invalid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EpisodeDisposition {
    Scored,
    OutOfDomain,
    UnsupportedTimebase,
    UnknownSensorState,
    UnknownFaultLabel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectionStatus {
    Detected,
    Missed,
    NotApplicable,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FaultThreshold {
    pub fault_label: String,
    pub probability_threshold: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EvaluationPlan {
    pub model_digest: String,
    pub feature_schema_digest: String,
    pub training_split_digest: String,
    pub calibration_split_digest: String,
    pub test_split_digest: String,
    pub domain_digest: String,
    pub fault_taxonomy: Vec<String>,
    pub thresholds: Vec<FaultThreshold>,
    pub training_specimen_ids: Vec<String>,
    pub calibration_specimen_ids: Vec<String>,
    pub training_machine_ids: Vec<String>,
    pub maximum_time_mapping_skew_ns: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SensorChannel {
    pub channel_id: String,
    pub native_sample_rate_hz: f64,
    pub state: ChannelState,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TimebaseEvidence {
    pub status: TimebaseStatus,
    pub reference_clock_id: String,
    pub mapping_digest: String,
    /// Worst-case residual inter-channel timing skew.
    pub maximum_residual_skew_ns: Option<u64>,
    /// Native sampling rates are retained; rows are not assumed synchronous.
    pub synchronization_method: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FaultProbability {
    pub fault_label: String,
    pub probability: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PredictionWindow {
    pub window_id: String,
    pub timestamp_ns: u64,
    pub abstained: bool,
    /// One probability per frozen fault-taxonomy label, including healthy
    /// episodes. An abstention preserves scores for audit but they are not
    /// interpreted as a decision.
    pub probabilities: Vec<FaultProbability>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EpisodeRecord {
    pub episode_id: String,
    pub specimen_id: String,
    pub machine_id: String,
    pub axis_id: String,
    pub session_id: String,
    pub operating_condition_digest: String,
    pub source_record_digest: String,
    pub start_time_ns: u64,
    pub end_time_ns: u64,
    /// Empty for a verified healthy episode; multi-label for compound faults.
    pub true_faults: Vec<String>,
    /// True when the source identifies a fault but cannot map it to the frozen
    /// taxonomy. Such an episode must abstain rather than be relabelled healthy.
    pub unknown_fault_class: bool,
    /// First independently established onset of the labelled event.
    pub fault_onset_time_ns: Option<u64>,
    /// Verified healthy exposure within this episode, before any labelled
    /// onset. This is the only exposure admitted to the FAR denominator.
    pub verified_healthy_exposure_seconds: f64,
    pub in_domain: bool,
    pub timebase: TimebaseEvidence,
    pub channels: Vec<SensorChannel>,
    pub predictions: Vec<PredictionWindow>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EvaluationDataset {
    pub split_digest: String,
    pub model_digest: String,
    pub feature_schema_digest: String,
    pub domain_digest: String,
    pub source_dataset_id: String,
    pub source_dataset_license: String,
    pub attribution_reference: String,
    pub episodes: Vec<EpisodeRecord>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EpisodeAssessment {
    pub episode_id: String,
    pub specimen_id: String,
    pub machine_id: String,
    pub axis_id: String,
    pub disposition: EpisodeDisposition,
    pub detection_status: DetectionStatus,
    pub event_detected_time_to_detect_seconds: Option<f64>,
    pub complete_compound_detected_time_to_detect_seconds: Option<f64>,
    pub true_faults: Vec<String>,
    pub union_predicted_faults: Vec<String>,
    pub exact_set_match: Option<bool>,
    pub verified_healthy_exposure_seconds: f64,
    pub false_alarm_transition_count: usize,
    pub abstained_window_count: usize,
    /// Abstentions among supported/scored episodes only; unsupported-domain
    /// abstentions are not mixed into this selective-risk denominator.
    pub scored_episode_abstained_window_count: usize,
    pub scored_window_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FaultClassMetrics {
    pub fault_label: String,
    pub true_positive_episodes: usize,
    pub false_positive_episodes: usize,
    pub false_negative_episodes: usize,
    pub precision: Option<f64>,
    pub recall: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MachineMetrics {
    pub machine_id: String,
    pub held_out_machine: bool,
    pub scored_episode_count: usize,
    pub fault_episode_count: usize,
    pub detected_fault_episode_count: usize,
    pub compound_episode_count: usize,
    pub complete_compound_detected_count: usize,
    pub false_alarm_transition_count: usize,
    pub verified_healthy_exposure_hours: f64,
    pub false_alarms_per_healthy_hour: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChannelStateMetrics {
    pub channel_state: ChannelState,
    pub episode_count: usize,
    pub scored_episode_count: usize,
    pub fault_episode_count: usize,
    pub detected_fault_episode_count: usize,
    pub abstained_window_count: usize,
    pub scored_window_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DetectionDelay {
    pub episode_id: String,
    pub specimen_id: String,
    pub machine_id: String,
    /// Earliest correct fault label after onset.
    pub first_fault_detection_seconds: Option<f64>,
    /// Earliest single prediction containing every fault in the compound event.
    pub complete_compound_detection_seconds: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EvaluationReport {
    pub model_digest: String,
    pub feature_schema_digest: String,
    pub training_split_digest: String,
    pub calibration_split_digest: String,
    pub test_split_digest: String,
    pub domain_digest: String,
    pub source_dataset_id: String,
    pub source_dataset_license: String,
    pub attribution_reference: String,
    pub independent_specimen_count: usize,
    pub machine_count: usize,
    pub total_episode_count: usize,
    pub scored_episode_count: usize,
    pub out_of_domain_episode_count: usize,
    pub unsupported_timebase_episode_count: usize,
    pub unknown_sensor_state_episode_count: usize,
    pub unknown_fault_label_episode_count: usize,
    pub fault_episode_count: usize,
    pub detected_fault_episode_count: usize,
    pub event_recall: Option<f64>,
    pub compound_fault_episode_count: usize,
    pub complete_compound_detected_count: usize,
    pub compound_fault_retention: Option<f64>,
    pub exact_set_match_count: usize,
    pub exact_set_accuracy: Option<f64>,
    pub false_alarm_transition_count: usize,
    pub verified_healthy_exposure_hours: f64,
    pub false_alarms_per_healthy_hour: Option<f64>,
    pub mean_time_to_detect_seconds: Option<f64>,
    pub median_time_to_detect_seconds: Option<f64>,
    pub p95_time_to_detect_seconds: Option<f64>,
    pub mean_complete_compound_detection_seconds: Option<f64>,
    pub abstained_window_count: usize,
    pub scored_window_count: usize,
    pub window_abstention_rate: Option<f64>,
    pub unsupported_nonabstention_prediction_count: usize,
    pub brier_score: Option<f64>,
    pub per_fault_class: Vec<FaultClassMetrics>,
    pub per_machine: Vec<MachineMetrics>,
    pub per_channel_state: Vec<ChannelStateMetrics>,
    pub detection_delays: Vec<DetectionDelay>,
    pub episodes: Vec<EpisodeAssessment>,
    pub claim_ceiling: String,
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn rate(numerator: usize, denominator: usize) -> Option<f64> {
    (denominator > 0).then_some(numerator as f64 / denominator as f64)
}

fn sorted_unique(values: &mut Vec<String>, label: &str) -> Result<(), String> {
    if values.iter().any(|value| !nonempty(value)) {
        return Err(format!("{label} contains an empty identity"));
    }
    values.sort();
    if values.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(format!("{label} contains duplicate identities"));
    }
    Ok(())
}

fn validate_plan(plan: &EvaluationPlan) -> Result<(), String> {
    for (label, value) in [
        ("model digest", plan.model_digest.as_str()),
        ("feature schema digest", plan.feature_schema_digest.as_str()),
        ("training split digest", plan.training_split_digest.as_str()),
        ("calibration split digest", plan.calibration_split_digest.as_str()),
        ("test split digest", plan.test_split_digest.as_str()),
        ("domain digest", plan.domain_digest.as_str()),
    ] {
        if !nonempty(value) {
            return Err(format!("{label} is required"));
        }
    }

    let splits = [
        plan.training_split_digest.as_str(),
        plan.calibration_split_digest.as_str(),
        plan.test_split_digest.as_str(),
    ];
    if splits[0] == splits[1] || splits[0] == splits[2] || splits[1] == splits[2] {
        return Err("training, calibration and test split digests must be distinct".to_owned());
    }

    let mut taxonomy = plan.fault_taxonomy.clone();
    sorted_unique(&mut taxonomy, "fault taxonomy")?;
    if taxonomy.is_empty() {
        return Err("fault taxonomy must contain at least one fault class".to_owned());
    }
    if taxonomy != plan.fault_taxonomy {
        return Err("fault taxonomy must be sorted to produce canonical multi-label sets".to_owned());
    }

    if plan.thresholds.len() != taxonomy.len() {
        return Err("there must be exactly one probability threshold per fault class".to_owned());
    }
    let mut threshold_names: Vec<String> =
        plan.thresholds.iter().map(|threshold| threshold.fault_label.clone()).collect();
    sorted_unique(&mut threshold_names, "fault thresholds")?;
    if threshold_names != taxonomy {
        return Err("threshold labels must match the frozen fault taxonomy exactly".to_owned());
    }
    for threshold in &plan.thresholds {
        if !threshold.probability_threshold.is_finite()
            || !(0.0..=1.0).contains(&threshold.probability_threshold)
        {
            return Err(format!("invalid threshold for fault {:?}", threshold.fault_label));
        }
    }

    let mut training_specimens = plan.training_specimen_ids.clone();
    let mut calibration_specimens = plan.calibration_specimen_ids.clone();
    let mut training_machines = plan.training_machine_ids.clone();
    sorted_unique(&mut training_specimens, "training specimen IDs")?;
    sorted_unique(&mut calibration_specimens, "calibration specimen IDs")?;
    sorted_unique(&mut training_machines, "training machine IDs")?;
    if training_specimens.is_empty() || calibration_specimens.is_empty() || training_machines.is_empty() {
        return Err("training, calibration and training-machine inventories must be nonempty".to_owned());
    }
    if training_specimens.iter().any(|id| calibration_specimens.binary_search(id).is_ok()) {
        return Err("specimen ID appears in both training and calibration inventories".to_owned());
    }
    Ok(())
}

fn prediction_labels(
    prediction: &PredictionWindow,
    plan: &EvaluationPlan,
) -> Result<Vec<String>, String> {
    if prediction.probabilities.len() != plan.fault_taxonomy.len() {
        return Err(format!("window {} does not contain one score per taxonomy label", prediction.window_id));
    }
    let mut scores = BTreeMap::new();
    for score in &prediction.probabilities {
        if !score.probability.is_finite() || !(0.0..=1.0).contains(&score.probability) {
            return Err(format!("window {} contains an invalid probability", prediction.window_id));
        }
        if !plan.fault_taxonomy.contains(&score.fault_label) {
            return Err(format!("window {} contains unknown fault label {:?}", prediction.window_id, score.fault_label));
        }
        if scores.insert(score.fault_label.as_str(), score.probability).is_some() {
            return Err(format!("window {} repeats a fault probability label", prediction.window_id));
        }
    }
    if scores.len() != plan.fault_taxonomy.len() {
        return Err(format!("window {} omits one or more taxonomy probability labels", prediction.window_id));
    }

    if prediction.abstained {
        return Ok(Vec::new());
    }

    let labels = plan
        .thresholds
        .iter()
        .filter_map(|threshold| {
            scores
                .get(threshold.fault_label.as_str())
                .filter(|probability| **probability >= threshold.probability_threshold)
                .map(|_| threshold.fault_label.clone())
        })
        .collect();
    Ok(labels)
}

fn episode_disposition(episode: &EpisodeRecord, plan: &EvaluationPlan) -> EpisodeDisposition {
    if !episode.in_domain {
        return EpisodeDisposition::OutOfDomain;
    }
    if episode.timebase.status != TimebaseStatus::Synchronized
        || !nonempty(&episode.timebase.reference_clock_id)
        || !nonempty(&episode.timebase.mapping_digest)
        || episode.timebase.maximum_residual_skew_ns.is_none_or(|skew| {
            skew > plan.maximum_time_mapping_skew_ns
        })
        || !nonempty(&episode.timebase.synchronization_method)
    {
        return EpisodeDisposition::UnsupportedTimebase;
    }
    if episode.channels.iter().any(|channel| channel.state == ChannelState::Unknown) {
        return EpisodeDisposition::UnknownSensorState;
    }
    if episode.unknown_fault_class {
        return EpisodeDisposition::UnknownFaultLabel;
    }
    EpisodeDisposition::Scored
}

fn validate_episode(
    episode: &EpisodeRecord,
    plan: &EvaluationPlan,
    episode_ids: &mut BTreeSet<String>,
    window_ids: &mut BTreeSet<String>,
    specimen_machine: &mut BTreeMap<String, String>,
) -> Result<(), String> {
    for (label, value) in [
        ("episode ID", episode.episode_id.as_str()),
        ("specimen ID", episode.specimen_id.as_str()),
        ("machine ID", episode.machine_id.as_str()),
        ("axis ID", episode.axis_id.as_str()),
        ("session ID", episode.session_id.as_str()),
        ("operating-condition digest", episode.operating_condition_digest.as_str()),
        ("source-record digest", episode.source_record_digest.as_str()),
    ] {
        if !nonempty(value) {
            return Err(format!("{label} is required"));
        }
    }
    if !episode_ids.insert(episode.episode_id.clone()) {
        return Err(format!("duplicate episode ID {}", episode.episode_id));
    }
    if episode.start_time_ns >= episode.end_time_ns {
        return Err(format!("episode {} has an invalid time interval", episode.episode_id));
    }
    if !episode.verified_healthy_exposure_seconds.is_finite()
        || episode.verified_healthy_exposure_seconds < 0.0
    {
        return Err(format!("episode {} has invalid verified healthy exposure", episode.episode_id));
    }

    let duration_s = (episode.end_time_ns - episode.start_time_ns) as f64 / 1.0e9;
    let available_healthy_s = if let Some(onset) = episode.fault_onset_time_ns {
        if onset < episode.start_time_ns || onset > episode.end_time_ns {
            return Err(format!("episode {} fault onset is outside its time interval", episode.episode_id));
        }
        if episode.true_faults.is_empty() && !episode.unknown_fault_class {
            return Err(format!("episode {} has an onset but no fault label", episode.episode_id));
        }
        (onset - episode.start_time_ns) as f64 / 1.0e9
    } else {
        if !episode.true_faults.is_empty() && !episode.unknown_fault_class {
            return Err(format!("episode {} has fault labels but no onset", episode.episode_id));
        }
        duration_s
    };
    if episode.verified_healthy_exposure_seconds > available_healthy_s + 1.0e-9 {
        return Err(format!("episode {} claims more verified healthy exposure than its known healthy interval", episode.episode_id));
    }

    if episode.unknown_fault_class && !episode.true_faults.is_empty() {
        return Err(format!("episode {} must not map an unknown fault label to known classes", episode.episode_id));
    }
    if episode.unknown_fault_class && episode.fault_onset_time_ns.is_none() {
        return Err(format!("episode {} has an unknown fault class but no independently established event onset", episode.episode_id));
    }
    let mut truth = episode.true_faults.clone();
    sorted_unique(&mut truth, "episode truth fault set")?;
    if truth != episode.true_faults {
        return Err(format!("episode {} truth fault set must use canonical sorted order", episode.episode_id));
    }
    for label in &episode.true_faults {
        if !plan.fault_taxonomy.contains(label) {
            return Err(format!("episode {} includes a fault outside the frozen taxonomy: {label}", episode.episode_id));
        }
    }

    if episode.channels.is_empty() {
        return Err(format!("episode {} must declare at least one sensor channel", episode.episode_id));
    }
    let mut channel_ids = BTreeSet::new();
    for channel in &episode.channels {
        if !nonempty(&channel.channel_id) || !channel_ids.insert(channel.channel_id.as_str()) {
            return Err(format!("episode {} has an empty or duplicate sensor channel ID", episode.episode_id));
        }
        if !channel.native_sample_rate_hz.is_finite() || channel.native_sample_rate_hz <= 0.0 {
            return Err(format!("episode {} channel {} has invalid native sample rate", episode.episode_id, channel.channel_id));
        }
    }

    if episode.timebase.status == TimebaseStatus::Synchronized {
        if !nonempty(&episode.timebase.reference_clock_id)
            || !nonempty(&episode.timebase.mapping_digest)
            || episode.timebase.maximum_residual_skew_ns.is_none()
            || !nonempty(&episode.timebase.synchronization_method)
        {
            return Err(format!("episode {} synchronized timebase has incomplete mapping evidence", episode.episode_id));
        }
    }

    if episode.predictions.is_empty() {
        return Err(format!("episode {} has no model prediction windows", episode.episode_id));
    }
    let mut previous_time = None;
    for prediction in &episode.predictions {
        if !nonempty(&prediction.window_id) || !window_ids.insert(prediction.window_id.clone()) {
            return Err(format!("episode {} has an empty or duplicate prediction window ID", episode.episode_id));
        }
        if prediction.timestamp_ns < episode.start_time_ns || prediction.timestamp_ns > episode.end_time_ns {
            return Err(format!("episode {} prediction lies outside its episode interval", episode.episode_id));
        }
        if previous_time.is_some_and(|time| prediction.timestamp_ns <= time) {
            return Err(format!("episode {} prediction times are not strictly increasing", episode.episode_id));
        }
        previous_time = Some(prediction.timestamp_ns);
        let _ = prediction_labels(prediction, plan)?;
    }

    match specimen_machine.get(&episode.specimen_id) {
        Some(machine) if machine != &episode.machine_id => {
            return Err(format!("specimen {} is assigned to multiple machine IDs", episode.specimen_id));
        }
        None => {
            specimen_machine.insert(episode.specimen_id.clone(), episode.machine_id.clone());
        }
        _ => {}
    }
    Ok(())
}

fn event_truth_at(episode: &EpisodeRecord, timestamp_ns: u64) -> BTreeSet<&str> {
    match episode.fault_onset_time_ns {
        Some(onset) if timestamp_ns >= onset => episode.true_faults.iter().map(String::as_str).collect(),
        _ => BTreeSet::new(),
    }
}

fn sorted_percentile(values: &mut [f64], fraction: f64) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(f64::total_cmp);
    let index = ((values.len() - 1) as f64 * fraction).round() as usize;
    values.get(index).copied()
}

#[derive(Debug, Clone, Default)]
struct Counts {
    scored_episodes: usize,
    fault_episodes: usize,
    detected_fault_episodes: usize,
    compound_episodes: usize,
    complete_compound_detected: usize,
    false_alarm_transitions: usize,
    verified_healthy_seconds: f64,
}

impl Counts {
    fn freeze_machine(self, machine_id: String, held_out: bool) -> MachineMetrics {
        let healthy_hours = self.verified_healthy_seconds / 3600.0;
        MachineMetrics {
            machine_id,
            held_out_machine: held_out,
            scored_episode_count: self.scored_episodes,
            fault_episode_count: self.fault_episodes,
            detected_fault_episode_count: self.detected_fault_episodes,
            compound_episode_count: self.compound_episodes,
            complete_compound_detected_count: self.complete_compound_detected,
            false_alarm_transition_count: self.false_alarm_transitions,
            verified_healthy_exposure_hours: healthy_hours,
            false_alarms_per_healthy_hour: (healthy_hours > 0.0)
                .then_some(self.false_alarm_transitions as f64 / healthy_hours),
        }
    }
}

#[derive(Debug, Clone, Default)]
struct ChannelCounts {
    episodes: usize,
    scored_episodes: usize,
    fault_episodes: usize,
    detected_fault_episodes: usize,
    abstained_windows: usize,
    scored_windows: usize,
}

pub fn evaluate_fault_benchmark(
    plan: &EvaluationPlan,
    dataset: &EvaluationDataset,
) -> Result<EvaluationReport, String> {
    validate_plan(plan)?;
    if dataset.split_digest != plan.test_split_digest {
        return Err("dataset is not the frozen held-out test split".to_owned());
    }
    if dataset.model_digest != plan.model_digest
        || dataset.feature_schema_digest != plan.feature_schema_digest
        || dataset.domain_digest != plan.domain_digest
    {
        return Err("dataset model, feature-schema or domain digest differs from plan".to_owned());
    }
    if !nonempty(&dataset.source_dataset_id)
        || !nonempty(&dataset.source_dataset_license)
        || !nonempty(&dataset.attribution_reference)
    {
        return Err("source dataset ID, license and attribution reference are required".to_owned());
    }
    if dataset.episodes.is_empty() {
        return Err("evaluation dataset must contain episodes".to_owned());
    }

    let training_specimens: BTreeSet<&str> =
        plan.training_specimen_ids.iter().map(String::as_str).collect();
    let calibration_specimens: BTreeSet<&str> =
        plan.calibration_specimen_ids.iter().map(String::as_str).collect();

    let mut episode_ids = BTreeSet::new();
    let mut window_ids = BTreeSet::new();
    let mut specimen_machine = BTreeMap::new();
    for episode in &dataset.episodes {
        if training_specimens.contains(episode.specimen_id.as_str())
            || calibration_specimens.contains(episode.specimen_id.as_str())
        {
            return Err(format!("test episode {} leaks a training/calibration specimen", episode.episode_id));
        }
        validate_episode(episode, plan, &mut episode_ids, &mut window_ids, &mut specimen_machine)?;
    }

    let mut counts_by_machine: BTreeMap<String, Counts> = BTreeMap::new();
    let mut counts_by_channel_state: BTreeMap<ChannelState, ChannelCounts> = BTreeMap::new();
    let mut class_counts: BTreeMap<String, (usize, usize, usize)> = plan
        .fault_taxonomy
        .iter()
        .map(|label| (label.clone(), (0, 0, 0)))
        .collect();

    let mut episode_assessments = Vec::with_capacity(dataset.episodes.len());
    let mut detection_delays = Vec::new();
    let mut all_ttd = Vec::new();
    let mut all_compound_ttd = Vec::new();
    let mut total_false_alarms = 0usize;
    let mut total_healthy_seconds = 0.0_f64;
    let mut total_scored_windows = 0usize;
    let mut total_abstained_windows = 0usize;
    let mut scored_episode_abstained_windows = 0usize;
    let mut unsupported_nonabstention = 0usize;
    let mut unsupported_counts = [0usize; 4]; // OOD, timebase, sensor-state, unknown-label
    let mut fault_episode_count = 0usize;
    let mut detected_fault_episode_count = 0usize;
    let mut compound_episode_count = 0usize;
    let mut complete_compound_count = 0usize;
    let mut exact_set_match_count = 0usize;
    let mut exact_set_denominator = 0usize;
    let mut brier_sum = 0.0_f64;
    let mut brier_terms = 0usize;

    for episode in &dataset.episodes {
        let disposition = episode_disposition(episode, plan);
        let machine_counts = counts_by_machine.entry(episode.machine_id.clone()).or_default();
        let episode_channel_states: BTreeSet<ChannelState> =
            episode.channels.iter().map(|channel| channel.state).collect();

        let mut abstained_windows = 0usize;
        let mut scored_windows = 0usize;
        let mut false_alarm_transitions = 0usize;
        let mut union_predicted = BTreeSet::new();
        let mut first_detection_ns: Option<u64> = None;
        let mut complete_compound_detection_ns: Option<u64> = None;
        let mut previous_alarm = false;

        if disposition == EpisodeDisposition::Scored {
            machine_counts.scored_episodes += 1;
            machine_counts.verified_healthy_seconds += episode.verified_healthy_exposure_seconds;
            total_healthy_seconds += episode.verified_healthy_exposure_seconds;

            if !episode.true_faults.is_empty() {
                machine_counts.fault_episodes += 1;
                fault_episode_count += 1;
                if episode.true_faults.len() > 1 {
                    machine_counts.compound_episodes += 1;
                    compound_episode_count += 1;
                }
            }

            for channel in &episode.channels {
                let counts = counts_by_channel_state.entry(channel.state).or_default();
                counts.episodes += 1;
                counts.scored_episodes += 1;
                if !episode.true_faults.is_empty() {
                    counts.fault_episodes += 1;
                }
            }

            for prediction in &episode.predictions {
                if prediction.abstained {
                    abstained_windows += 1;
                    total_abstained_windows += 1;
                    scored_episode_abstained_windows += 1;
                    previous_alarm = false;
                    for state in &episode_channel_states {
                        counts_by_channel_state.entry(*state).or_default().abstained_windows += 1;
                    }
                    continue;
                }

                scored_windows += 1;
                total_scored_windows += 1;
                for state in &episode_channel_states {
                    counts_by_channel_state.entry(*state).or_default().scored_windows += 1;
                }

                let predicted_labels = prediction_labels(prediction, plan)?;
                let active_faults = event_truth_at(episode, prediction.timestamp_ns);
                // Only post-onset decisions contribute to fault localization.
                // A pre-onset alarm remains a false alarm but cannot be
                // credited as a prediction of an event that has not happened.
                if episode.fault_onset_time_ns.is_none() || !active_faults.is_empty() {
                    for label in &predicted_labels {
                        union_predicted.insert(label.clone());
                    }
                }
                let is_alarm = !predicted_labels.is_empty();
                if is_alarm && !previous_alarm {
                    if active_faults.is_empty() {
                        false_alarm_transitions += 1;
                    }
                }
                previous_alarm = is_alarm;

                if !active_faults.is_empty() {
                    if first_detection_ns.is_none()
                        && predicted_labels.iter().any(|label| active_faults.contains(label.as_str()))
                    {
                        first_detection_ns = Some(prediction.timestamp_ns);
                    }
                    if episode.true_faults.len() > 1
                        && complete_compound_detection_ns.is_none()
                        && episode.true_faults.iter().all(|label| predicted_labels.contains(label))
                    {
                        complete_compound_detection_ns = Some(prediction.timestamp_ns);
                    }
                }

                for threshold in &plan.thresholds {
                    let target = if active_faults.contains(threshold.fault_label.as_str()) {
                        1.0
                    } else {
                        0.0
                    };
                    let probability = prediction
                        .probabilities
                        .iter()
                        .find(|score| score.fault_label == threshold.fault_label)
                        .unwrap()
                        .probability;
                    brier_sum += (probability - target).powi(2);
                    brier_terms += 1;
                }
            }

            machine_counts.false_alarm_transitions += false_alarm_transitions;
            total_false_alarms += false_alarm_transitions;

            let mut union_predicted_vec: Vec<String> = union_predicted.iter().cloned().collect();
            let truth_set: BTreeSet<String> = episode.true_faults.iter().cloned().collect();
            let exact_match = union_predicted == truth_set;
            exact_set_denominator += 1;
            if exact_match {
                exact_set_match_count += 1;
            }

            if !episode.true_faults.is_empty() {
                if let Some(detection_time) = first_detection_ns {
                    detected_fault_episode_count += 1;
                    machine_counts.detected_fault_episodes += 1;
                    for state in &episode_channel_states {
                        counts_by_channel_state.entry(*state).or_default().detected_fault_episodes += 1;
                    }
                    let delay = (detection_time - episode.fault_onset_time_ns.unwrap()) as f64 / 1.0e9;
                    all_ttd.push(delay);
                }
                let complete_delay = complete_compound_detection_ns.map(|time| {
                    (time - episode.fault_onset_time_ns.unwrap()) as f64 / 1.0e9
                });
                if episode.true_faults.len() > 1 {
                    if let Some(delay) = complete_delay {
                        complete_compound_count += 1;
                        machine_counts.complete_compound_detected += 1;
                        all_compound_ttd.push(delay);
                    }
                }
                detection_delays.push(DetectionDelay {
                    episode_id: episode.episode_id.clone(),
                    specimen_id: episode.specimen_id.clone(),
                    machine_id: episode.machine_id.clone(),
                    first_fault_detection_seconds: first_detection_ns.map(|time| {
                        (time - episode.fault_onset_time_ns.unwrap()) as f64 / 1.0e9
                    }),
                    complete_compound_detection_seconds: complete_delay,
                });
            }

            // Per-class episode-level counts use the union of post-onset
            // predictions; windows are not treated as independent episodes.
            for label in &episode.true_faults {
                let (tp, _, fn_count) = class_counts.get_mut(label).unwrap();
                if union_predicted.contains(label) {
                    *tp += 1;
                } else {
                    *fn_count += 1;
                }
            }
            for label in &union_predicted_vec {
                if !episode.true_faults.contains(label) {
                    if let Some((_, fp, _)) = class_counts.get_mut(label) {
                        *fp += 1;
                    }
                }
            }

            if episode.true_faults.len() > 1 && complete_compound_detection_ns.is_some() {
                // The complete compound count is recorded at the first
                // prediction window where the full true set was present.
            }

            episode_assessments.push(EpisodeAssessment {
                episode_id: episode.episode_id.clone(),
                specimen_id: episode.specimen_id.clone(),
                machine_id: episode.machine_id.clone(),
                axis_id: episode.axis_id.clone(),
                disposition,
                detection_status: if episode.true_faults.is_empty() {
                    DetectionStatus::NotApplicable
                } else if first_detection_ns.is_some() {
                    DetectionStatus::Detected
                } else {
                    DetectionStatus::Missed
                },
                event_detected_time_to_detect_seconds: first_detection_ns.map(|time| {
                    (time - episode.fault_onset_time_ns.unwrap()) as f64 / 1.0e9
                }),
                complete_compound_detected_time_to_detect_seconds: complete_compound_detection_ns.map(|time| {
                    (time - episode.fault_onset_time_ns.unwrap()) as f64 / 1.0e9
                }),
                true_faults: episode.true_faults.clone(),
                union_predicted_faults: union_predicted_vec,
                exact_set_match: Some(exact_match),
                verified_healthy_exposure_seconds: episode.verified_healthy_exposure_seconds,
                false_alarm_transition_count: false_alarm_transitions,
                abstained_window_count: abstained_windows,
                scored_window_count: scored_windows,
            });
        } else {
            let index = match disposition {
                EpisodeDisposition::OutOfDomain => 0,
                EpisodeDisposition::UnsupportedTimebase => 1,
                EpisodeDisposition::UnknownSensorState => 2,
                EpisodeDisposition::UnknownFaultLabel => 3,
                EpisodeDisposition::Scored => unreachable!(),
            };
            unsupported_counts[index] += 1;
            for prediction in &episode.predictions {
                if prediction.abstained {
                    abstained_windows += 1;
                    total_abstained_windows += 1;
                } else {
                    unsupported_nonabstention += 1;
                }
            }
            episode_assessments.push(EpisodeAssessment {
                episode_id: episode.episode_id.clone(),
                specimen_id: episode.specimen_id.clone(),
                machine_id: episode.machine_id.clone(),
                axis_id: episode.axis_id.clone(),
                disposition,
                detection_status: DetectionStatus::NotApplicable,
                event_detected_time_to_detect_seconds: None,
                complete_compound_detected_time_to_detect_seconds: None,
                true_faults: episode.true_faults.clone(),
                union_predicted_faults: Vec::new(),
                exact_set_match: None,
                verified_healthy_exposure_seconds: 0.0,
                false_alarm_transition_count: 0,
                abstained_window_count: abstained_windows,
                scored_window_count: 0,
            });
        }
    }

    for episode in dataset.episodes.iter().filter(|episode| episode_disposition(episode, plan) == EpisodeDisposition::Scored) {
        for state in episode.channels.iter().map(|channel| channel.state).collect::<BTreeSet<_>>() {
            let _ = counts_by_channel_state.entry(state).or_default();
        }
    }

    let per_fault_class = class_counts
        .into_iter()
        .map(|(fault_label, (tp, fp, fn_count))| FaultClassMetrics {
            fault_label,
            true_positive_episodes: tp,
            false_positive_episodes: fp,
            false_negative_episodes: fn_count,
            precision: rate(tp, tp + fp),
            recall: rate(tp, tp + fn_count),
        })
        .collect();

    let per_machine = {
        let mut machine_ids: Vec<String> = counts_by_machine.keys().cloned().collect();
        machine_ids.sort();
        machine_ids
            .into_iter()
            .map(|machine_id| {
                let counts = counts_by_machine.remove(&machine_id).unwrap();
                let held_out = !plan.training_machine_ids.contains(&machine_id);
                counts.freeze_machine(machine_id, held_out)
            })
            .collect()
    };

    let per_channel_state = counts_by_channel_state
        .into_iter()
        .map(|(channel_state, counts)| ChannelStateMetrics {
            channel_state,
            episode_count: counts.episodes,
            scored_episode_count: counts.scored_episodes,
            fault_episode_count: counts.fault_episodes,
            detected_fault_episode_count: counts.detected_fault_episodes,
            abstained_window_count: counts.abstained_windows,
            scored_window_count: counts.scored_windows,
        })
        .collect();

    let healthy_hours = total_healthy_seconds / 3600.0;
    let mean_ttd = if all_ttd.is_empty() {
        None
    } else {
        Some(all_ttd.iter().sum::<f64>() / all_ttd.len() as f64)
    };
    let median_ttd = sorted_percentile(&mut all_ttd, 0.5);
    let p95_ttd = sorted_percentile(&mut all_ttd, 0.95);

    Ok(EvaluationReport {
        model_digest: plan.model_digest.clone(),
        feature_schema_digest: plan.feature_schema_digest.clone(),
        training_split_digest: plan.training_split_digest.clone(),
        calibration_split_digest: plan.calibration_split_digest.clone(),
        test_split_digest: plan.test_split_digest.clone(),
        domain_digest: plan.domain_digest.clone(),
        source_dataset_id: dataset.source_dataset_id.clone(),
        source_dataset_license: dataset.source_dataset_license.clone(),
        attribution_reference: dataset.attribution_reference.clone(),
        independent_specimen_count: specimen_machine.len(),
        machine_count: specimen_machine.values().collect::<BTreeSet<_>>().len(),
        total_episode_count: dataset.episodes.len(),
        scored_episode_count: episode_assessments.iter().filter(|episode| episode.disposition == EpisodeDisposition::Scored).count(),
        out_of_domain_episode_count: unsupported_counts[0],
        unsupported_timebase_episode_count: unsupported_counts[1],
        unknown_sensor_state_episode_count: unsupported_counts[2],
        unknown_fault_label_episode_count: unsupported_counts[3],
        fault_episode_count,
        detected_fault_episode_count,
        event_recall: rate(detected_fault_episode_count, fault_episode_count),
        compound_fault_episode_count: compound_episode_count,
        complete_compound_detected_count: complete_compound_count,
        compound_fault_retention: rate(complete_compound_count, compound_episode_count),
        exact_set_match_count,
        exact_set_accuracy: rate(exact_set_match_count, exact_set_denominator),
        false_alarm_transition_count: total_false_alarms,
        verified_healthy_exposure_hours: healthy_hours,
        false_alarms_per_healthy_hour: (healthy_hours > 0.0).then_some(total_false_alarms as f64 / healthy_hours),
        mean_time_to_detect_seconds: mean_ttd,
        median_time_to_detect_seconds: median_ttd,
        p95_time_to_detect_seconds: p95_ttd,
        mean_complete_compound_detection_seconds: if all_compound_ttd.is_empty() {
            None
        } else {
            Some(all_compound_ttd.iter().sum::<f64>() / all_compound_ttd.len() as f64)
        },
        abstained_window_count: total_abstained_windows,
        scored_episode_abstained_window_count: scored_episode_abstained_windows,
        scored_window_count: total_scored_windows,
        window_abstention_rate: rate(
            scored_episode_abstained_windows,
            scored_episode_abstained_windows + total_scored_windows,
        ),
        unsupported_nonabstention_prediction_count: unsupported_nonabstention,
        brier_score: (brier_terms > 0).then_some(brier_sum / brier_terms as f64),
        per_fault_class,
        per_machine,
        per_channel_state,
        detection_delays,
        episodes: episode_assessments,
        claim_ceiling: "Episode-level benchmark metrics only. No physical fault diagnosis, probability calibration, cross-machine generalization, prognostic life, actuator derating, or safety action is established by this evaluator. It does not validate raw signal synchronization or source labels.".to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const MODEL: &str = "fault-model-v1";
    const FEATURE: &str = "multimodal-feature-schema-v1";
    const TRAIN: &str = "train-split-v1";
    const CAL: &str = "calibration-split-v1";
    const TEST: &str = "test-split-v1";
    const DOMAIN: &str = "robot-harmonic-drive-domain-v1";
    const LABELS: [&str; 3] = ["bearing_wear", "gear_tooth_wear", "lubrication_loss"];

    fn plan() -> EvaluationPlan {
        EvaluationPlan {
            model_digest: MODEL.to_owned(),
            feature_schema_digest: FEATURE.to_owned(),
            training_split_digest: TRAIN.to_owned(),
            calibration_split_digest: CAL.to_owned(),
            test_split_digest: TEST.to_owned(),
            domain_digest: DOMAIN.to_owned(),
            fault_taxonomy: LABELS.iter().map(|label| (*label).to_owned()).collect(),
            thresholds: LABELS.iter().map(|label| FaultThreshold {
                fault_label: (*label).to_owned(),
                probability_threshold: 0.5,
            }).collect(),
            training_specimen_ids: vec!["train-specimen-1".to_owned()],
            calibration_specimen_ids: vec!["cal-specimen-1".to_owned()],
            training_machine_ids: vec!["robot-train".to_owned()],
            maximum_time_mapping_skew_ns: 1_000_000,
        }
    }

    fn channel(id: &str, state: ChannelState) -> SensorChannel {
        SensorChannel {
            channel_id: id.to_owned(),
            native_sample_rate_hz: if id.starts_with("vib") { 2560.0 } else { 800.0 },
            state,
        }
    }

    fn prediction(
        id: &str,
        time: u64,
        probabilities: [f64; 3],
        abstained: bool,
    ) -> PredictionWindow {
        PredictionWindow {
            window_id: id.to_owned(),
            timestamp_ns: time,
            abstained,
            probabilities: LABELS.iter().zip(probabilities).map(|(label, probability)| {
                FaultProbability { fault_label: (*label).to_owned(), probability }
            }).collect(),
        }
    }

    fn episode(
        id: &str,
        specimen: &str,
        machine: &str,
        truth: &[&str],
        onset: Option<u64>,
        healthy_seconds: f64,
        predictions: Vec<PredictionWindow>,
    ) -> EpisodeRecord {
        EpisodeRecord {
            episode_id: id.to_owned(),
            specimen_id: specimen.to_owned(),
            machine_id: machine.to_owned(),
            axis_id: "axis-5".to_owned(),
            session_id: format!("session-{id}"),
            operating_condition_digest: "speed-medium-load-3kg".to_owned(),
            source_record_digest: format!("record-{id}"),
            start_time_ns: 0,
            end_time_ns: 10_000_000_000,
            true_faults: truth.iter().map(|fault| (*fault).to_owned()).collect(),
            unknown_fault_class: false,
            fault_onset_time_ns: onset,
            verified_healthy_exposure_seconds: healthy_seconds,
            in_domain: true,
            timebase: TimebaseEvidence {
                status: TimebaseStatus::Synchronized,
                reference_clock_id: "clock-ptp-1".to_owned(),
                mapping_digest: "mapping-1".to_owned(),
                maximum_residual_skew_ns: Some(100_000),
                synchronization_method: "PTP synchronized; synthetic fixture".to_owned(),
            },
            channels: vec![
                channel("vib-x", ChannelState::Healthy),
                channel("servo-torque", ChannelState::Healthy),
            ],
            predictions,
        }
    }

    fn dataset(episodes: Vec<EpisodeRecord>) -> EvaluationDataset {
        EvaluationDataset {
            split_digest: TEST.to_owned(),
            model_digest: MODEL.to_owned(),
            feature_schema_digest: FEATURE.to_owned(),
            domain_digest: DOMAIN.to_owned(),
            source_dataset_id: "synthetic-gdhdd-like".to_owned(),
            source_dataset_license: "synthetic-fixture-only".to_owned(),
            attribution_reference: "https://example.invalid/synthetic".to_owned(),
            episodes,
        }
    }

    fn find_episode<'a>(report: &'a EvaluationReport, id: &str) -> &'a EpisodeAssessment {
        report.episodes.iter().find(|episode| episode.episode_id == id).unwrap()
    }

    #[test]
    fn fault_001_detects_event_and_measures_time_to_first_correct_label() {
        let mut episode = episode(
            "fault-1", "test-specimen-1", "robot-heldout",
            &["gear_tooth_wear"], Some(5_000_000_000), 5.0,
            vec![
                prediction("w1", 4_000_000_000, [0.1, 0.1, 0.1], false),
                prediction("w2", 5_500_000_000, [0.1, 0.9, 0.1], false),
            ],
        );
        episode.axis_id = "axis-5".to_owned();
        let report = evaluate_fault_benchmark(&plan(), &dataset(vec![episode])).unwrap();
        assert_eq!(report.event_recall, Some(1.0));
        assert_eq!(find_episode(&report, "fault-1").detection_status, DetectionStatus::Detected);
        assert_eq!(find_episode(&report, "fault-1").event_detected_time_to_detect_seconds, Some(0.5));
    }

    #[test]
    fn fault_002_compound_event_requires_all_labels_in_one_prediction() {
        let episode = episode(
            "compound", "test-specimen-2", "robot-heldout",
            &["bearing_wear", "gear_tooth_wear"], Some(4_000_000_000), 4.0,
            vec![
                prediction("w1", 5_000_000_000, [0.9, 0.1, 0.1], false),
                prediction("w2", 6_000_000_000, [0.1, 0.9, 0.1], false),
            ],
        );
        let report = evaluate_fault_benchmark(&plan(), &dataset(vec![episode])).unwrap();
        assert_eq!(report.event_recall, Some(1.0));
        assert_eq!(report.compound_fault_retention, Some(0.0));
        assert_eq!(find_episode(&report, "compound").complete_compound_detected_time_to_detect_seconds, None);
    }

    #[test]
    fn fault_003_complete_compound_detection_is_recorded() {
        let episode = episode(
            "compound", "test-specimen-2", "robot-heldout",
            &["bearing_wear", "gear_tooth_wear"], Some(4_000_000_000), 4.0,
            vec![prediction("w1", 6_000_000_000, [0.9, 0.8, 0.1], false)],
        );
        let report = evaluate_fault_benchmark(&plan(), &dataset(vec![episode])).unwrap();
        assert_eq!(report.compound_fault_retention, Some(1.0));
        assert_eq!(report.mean_complete_compound_detection_seconds, Some(2.0));
    }

    #[test]
    fn fault_004_healthy_hours_are_explicit_false_alarm_denominator() {
        let episode = episode(
            "healthy", "test-specimen-3", "robot-heldout",
            &[], None, 10.0,
            vec![
                prediction("w1", 1_000_000_000, [0.1, 0.1, 0.1], false),
                prediction("w2", 2_000_000_000, [0.9, 0.1, 0.1], false),
                prediction("w3", 3_000_000_000, [0.9, 0.1, 0.1], false),
                prediction("w4", 4_000_000_000, [0.1, 0.1, 0.1], false),
                prediction("w5", 5_000_000_000, [0.9, 0.1, 0.1], false),
            ],
        );
        let report = evaluate_fault_benchmark(&plan(), &dataset(vec![episode])).unwrap();
        assert_eq!(report.false_alarm_transition_count, 2);
        assert_eq!(report.verified_healthy_exposure_hours, 10.0 / 3600.0);
        assert!((report.false_alarms_per_healthy_hour.unwrap() - 720.0).abs() < 1.0e-9);
    }

    #[test]
    fn fault_005_pre_onset_alarm_is_false_alarm_not_fault_detection() {
        let episode = episode(
            "pre-onset", "test-specimen-4", "robot-heldout",
            &["lubrication_loss"], Some(8_000_000_000), 8.0,
            vec![
                prediction("w1", 2_000_000_000, [0.1, 0.1, 0.9], false),
                prediction("w2", 4_000_000_000, [0.1, 0.1, 0.1], false),
                prediction("w3", 8_500_000_000, [0.1, 0.1, 0.9], false),
            ],
        );
        let report = evaluate_fault_benchmark(&plan(), &dataset(vec![episode])).unwrap();
        assert_eq!(report.false_alarm_transition_count, 1);
        assert_eq!(report.event_recall, Some(1.0));
        assert_eq!(find_episode(&report, "pre-onset").event_detected_time_to_detect_seconds, Some(0.5));
    }

    #[test]
    fn fault_006_abstention_is_not_healthy_prediction() {
        let episode = episode(
            "abstain", "test-specimen-5", "robot-heldout",
            &["gear_tooth_wear"], Some(2_000_000_000), 2.0,
            vec![
                prediction("w1", 2_500_000_000, [0.1, 0.1, 0.1], true),
                prediction("w2", 3_000_000_000, [0.1, 0.1, 0.1], true),
            ],
        );
        let report = evaluate_fault_benchmark(&plan(), &dataset(vec![episode])).unwrap();
        assert_eq!(report.event_recall, Some(0.0));
        assert_eq!(report.window_abstention_rate, Some(1.0));
        assert_eq!(report.false_alarm_transition_count, 0);
    }

    #[test]
    fn fault_007_pre_onset_false_alarm_does_not_pollute_post_onset_fault_localization() {
        let episode = episode(
            "pre-onset-union", "test-specimen-pre-union", "robot-heldout",
            &["bearing_wear"], Some(5_000_000_000), 5.0,
            vec![
                prediction("w1", 2_000_000_000, [0.9, 0.1, 0.1], false),
                prediction("w2", 6_000_000_000, [0.9, 0.1, 0.1], false),
            ],
        );
        let report = evaluate_fault_benchmark(&plan(), &dataset(vec![episode])).unwrap();
        assert_eq!(report.false_alarm_transition_count, 1);
        assert_eq!(report.event_recall, Some(1.0));
        assert_eq!(find_episode(&report, "pre-onset-union").union_predicted_faults, vec!["bearing_wear"]);
        assert_eq!(find_episode(&report, "pre-onset-union").exact_set_match, Some(true));
    }

    #[test]
    fn fault_008_out_of_domain_nonabstention_is_explicitly_counted() {
        let mut episode = episode(
            "ood", "test-specimen-6", "robot-heldout",
            &[], None, 10.0,
            vec![prediction("w1", 1_000_000_000, [0.9, 0.1, 0.1], false)],
        );
        episode.in_domain = false;
        let report = evaluate_fault_benchmark(&plan(), &dataset(vec![episode])).unwrap();
        assert_eq!(report.out_of_domain_episode_count, 1);
        assert_eq!(report.unsupported_nonabstention_prediction_count, 1);
        assert_eq!(report.scored_episode_count, 0);
    }

    #[test]
    fn fault_009_unmapped_or_excessively_skewed_timebase_is_unsupported() {
        let mut episode = episode(
            "async", "test-specimen-7", "robot-heldout",
            &["bearing_wear"], Some(2_000_000_000), 2.0,
            vec![prediction("w1", 3_000_000_000, [0.9, 0.1, 0.1], false)],
        );
        episode.timebase.maximum_residual_skew_ns = Some(2_000_000);
        let report = evaluate_fault_benchmark(&plan(), &dataset(vec![episode])).unwrap();
        assert_eq!(report.unsupported_timebase_episode_count, 1);
        assert_eq!(report.scored_episode_count, 0);
    }

    #[test]
    fn fault_010_unknown_sensor_state_makes_episode_unsupported() {
        let mut episode = episode(
            "unknown-channel", "test-specimen-8", "robot-heldout",
            &["bearing_wear"], Some(2_000_000_000), 2.0,
            vec![prediction("w1", 3_000_000_000, [0.9, 0.1, 0.1], false)],
        );
        episode.channels[0].state = ChannelState::Unknown;
        let report = evaluate_fault_benchmark(&plan(), &dataset(vec![episode])).unwrap();
        assert_eq!(report.unknown_sensor_state_episode_count, 1);
        assert_eq!(report.scored_episode_count, 0);
    }

    #[test]
    fn fault_011_unknown_fault_label_requires_abstention() {
        let mut episode = episode(
            "unknown-fault", "test-specimen-9", "robot-heldout",
            &[], Some(2_000_000_000), 2.0,
            vec![prediction("w1", 3_000_000_000, [0.1, 0.1, 0.1], true)],
        );
        episode.unknown_fault_class = true;
        let report = evaluate_fault_benchmark(&plan(), &dataset(vec![episode])).unwrap();
        assert_eq!(report.unknown_fault_label_episode_count, 1);
        assert_eq!(report.unsupported_nonabstention_prediction_count, 0);
    }

    #[test]
    fn fault_012_channel_dropout_strata_are_reported_separately() {
        let mut episode = episode(
            "missing-vibration", "test-specimen-10", "robot-heldout",
            &["bearing_wear"], Some(2_000_000_000), 2.0,
            vec![prediction("w1", 3_000_000_000, [0.9, 0.1, 0.1], false)],
        );
        episode.channels[0].state = ChannelState::Missing;
        let report = evaluate_fault_benchmark(&plan(), &dataset(vec![episode])).unwrap();
        let missing = report.per_channel_state.iter().find(|metric| metric.channel_state == ChannelState::Missing).unwrap();
        assert_eq!(missing.episode_count, 1);
        assert_eq!(missing.detected_fault_episode_count, 1);
    }

    #[test]
    fn fault_013_cross_machine_metrics_identify_heldout_machines() {
        let episodes = vec![
            episode(
                "seen", "test-specimen-11", "robot-train",
                &["gear_tooth_wear"], Some(2_000_000_000), 2.0,
                vec![prediction("w1", 3_000_000_000, [0.1, 0.9, 0.1], false)],
            ),
            episode(
                "heldout", "test-specimen-12", "robot-new",
                &["gear_tooth_wear"], Some(2_000_000_000), 2.0,
                vec![prediction("w2", 3_000_000_000, [0.1, 0.9, 0.1], false)],
            ),
        ];
        let report = evaluate_fault_benchmark(&plan(), &dataset(episodes)).unwrap();
        assert_eq!(report.per_machine.len(), 2);
        assert!(report.per_machine.iter().any(|m| m.machine_id == "robot-new" && m.held_out_machine));
        assert!(report.per_machine.iter().any(|m| m.machine_id == "robot-train" && !m.held_out_machine));
    }

    #[test]
    fn fault_014_probability_scores_contribute_brier_score() {
        let episode = episode(
            "brier", "test-specimen-13", "robot-heldout",
            &["bearing_wear"], Some(0), 0.0,
            vec![prediction("w1", 1_000_000_000, [0.8, 0.1, 0.1], false)],
        );
        let report = evaluate_fault_benchmark(&plan(), &dataset(vec![episode])).unwrap();
        // (0.8-1)^2 + (0.1-0)^2 + (0.1-0)^2, divided by 3.
        assert!((report.brier_score.unwrap() - 0.02).abs() < 1.0e-12);
    }

    #[test]
    fn fault_015_training_and_calibration_specimen_leakage_rejects() {
        let episode = episode(
            "leak", "train-specimen-1", "robot-heldout",
            &["bearing_wear"], Some(2_000_000_000), 2.0,
            vec![prediction("w1", 3_000_000_000, [0.9, 0.1, 0.1], false)],
        );
        assert!(evaluate_fault_benchmark(&plan(), &dataset(vec![episode])).is_err());

        let plan = plan();
        let episode = episode(
            "leak-cal", "cal-specimen-1", "robot-heldout",
            &["bearing_wear"], Some(2_000_000_000), 2.0,
            vec![prediction("w1", 3_000_000_000, [0.9, 0.1, 0.1], false)],
        );
        assert!(evaluate_fault_benchmark(&plan, &dataset(vec![episode])).is_err());
    }

    #[test]
    fn fault_016_duplicate_episode_and_window_ids_reject() {
        let a = episode(
            "same", "specimen-a", "robot-heldout",
            &[], None, 10.0,
            vec![prediction("window", 1_000_000_000, [0.1, 0.1, 0.1], false)],
        );
        let b = episode(
            "same", "specimen-b", "robot-heldout",
            &[], None, 10.0,
            vec![prediction("window-2", 1_000_000_000, [0.1, 0.1, 0.1], false)],
        );
        assert!(evaluate_fault_benchmark(&plan(), &dataset(vec![a.clone(), b])).is_err());

        let mut duplicate_window = a.clone();
        duplicate_window.episode_id = "another".to_owned();
        let b = episode(
            "third", "specimen-c", "robot-heldout",
            &[], None, 10.0,
            vec![prediction("window", 2_000_000_000, [0.1, 0.1, 0.1], false)],
        );
        assert!(evaluate_fault_benchmark(&plan(), &dataset(vec![duplicate_window, b])).is_err());
    }

    #[test]
    fn fault_017_prediction_timestamps_must_be_ordered_and_within_episode() {
        let episode = episode(
            "bad-time", "test-specimen-16", "robot-heldout",
            &[], None, 10.0,
            vec![
                prediction("w1", 2_000_000_000, [0.1, 0.1, 0.1], false),
                prediction("w2", 1_000_000_000, [0.1, 0.1, 0.1], false),
            ],
        );
        assert!(evaluate_fault_benchmark(&plan(), &dataset(vec![episode])).is_err());
    }

    #[test]
    fn fault_018_fault_truth_and_onset_must_be_consistent() {
        let missing_onset = episode(
            "no-onset", "test-specimen-17", "robot-heldout",
            &["bearing_wear"], None, 0.0,
            vec![prediction("w1", 1_000_000_000, [0.9, 0.1, 0.1], false)],
        );
        assert!(evaluate_fault_benchmark(&plan(), &dataset(vec![missing_onset])).is_err());

        let no_fault_with_onset = episode(
            "false-onset", "test-specimen-18", "robot-heldout",
            &[], Some(1_000_000_000), 1.0,
            vec![prediction("w1", 2_000_000_000, [0.1, 0.1, 0.1], false)],
        );
        assert!(evaluate_fault_benchmark(&plan(), &dataset(vec![no_fault_with_onset])).is_err());
    }

    #[test]
    fn fault_019_healthy_exposure_cannot_exceed_verified_interval() {
        let episode = episode(
            "overcount", "test-specimen-19", "robot-heldout",
            &[], None, 11.0,
            vec![prediction("w1", 1_000_000_000, [0.1, 0.1, 0.1], false)],
        );
        assert!(evaluate_fault_benchmark(&plan(), &dataset(vec![episode])).is_err());
    }

    #[test]
    fn fault_020_native_sample_rates_are_not_assumed_equal() {
        let mut episode = episode(
            "rates", "test-specimen-20", "robot-heldout",
            &["bearing_wear"], Some(2_000_000_000), 2.0,
            vec![prediction("w1", 3_000_000_000, [0.9, 0.1, 0.1], false)],
        );
        episode.channels[0].native_sample_rate_hz = 512.0;
        episode.channels[1].native_sample_rate_hz = 800.0;
        let report = evaluate_fault_benchmark(&plan(), &dataset(vec![episode])).unwrap();
        assert_eq!(report.event_recall, Some(1.0));
    }

    #[test]
    fn fault_021_unsupported_predictions_are_not_scored_as_healthy() {
        let mut episode = episode(
            "bad-clock", "test-specimen-21", "robot-heldout",
            &[], None, 10.0,
            vec![prediction("w1", 1_000_000_000, [0.1, 0.1, 0.1], false)],
        );
        episode.timebase.status = TimebaseStatus::Unknown;
        let report = evaluate_fault_benchmark(&plan(), &dataset(vec![episode])).unwrap();
        assert_eq!(report.unsupported_timebase_episode_count, 1);
        assert_eq!(report.unsupported_nonabstention_prediction_count, 1);
        assert_eq!(report.verified_healthy_exposure_hours, 0.0);
        assert_eq!(report.false_alarms_per_healthy_hour, None);
    }

    #[test]
    fn fault_022_prediction_probability_schema_must_be_complete() {
        let mut episode = episode(
            "missing-score", "test-specimen-22", "robot-heldout",
            &[], None, 10.0,
            vec![prediction("w1", 1_000_000_000, [0.1, 0.1, 0.1], false)],
        );
        episode.predictions[0].probabilities.pop();
        assert!(evaluate_fault_benchmark(&plan(), &dataset(vec![episode])).is_err());
    }

    #[test]
    fn fault_023_unknown_prediction_label_and_nonfinite_probability_reject() {
        let mut episode = episode(
            "unknown-score", "test-specimen-23", "robot-heldout",
            &[], None, 10.0,
            vec![prediction("w1", 1_000_000_000, [0.1, 0.1, 0.1], false)],
        );
        episode.predictions[0].probabilities[0].fault_label = "unregistered_fault".to_owned();
        assert!(evaluate_fault_benchmark(&plan(), &dataset(vec![episode])).is_err());

        let mut episode = episode(
            "nan-score", "test-specimen-24", "robot-heldout",
            &[], None, 10.0,
            vec![prediction("w1", 1_000_000_000, [0.1, 0.1, 0.1], false)],
        );
        episode.predictions[0].probabilities[0].probability = f64::NAN;
        assert!(evaluate_fault_benchmark(&plan(), &dataset(vec![episode])).is_err());
    }

    #[test]
    fn fault_024_dataset_and_model_digests_must_match() {
        let mut dataset = dataset(vec![episode(
            "valid", "test-specimen-25", "robot-heldout",
            &[], None, 10.0,
            vec![prediction("w1", 1_000_000_000, [0.1, 0.1, 0.1], false)],
        )]);
        dataset.model_digest = "other-model".to_owned();
        assert!(evaluate_fault_benchmark(&plan(), &dataset).is_err());
    }

    #[test]
    fn fault_025_unknown_fault_episode_does_not_get_healthy_credit() {
        let mut episode = episode(
            "unknown-class", "test-specimen-26", "robot-heldout",
            &[], Some(2_000_000_000), 0.0,
            vec![prediction("w1", 3_000_000_000, [0.1, 0.1, 0.1], true)],
        );
        episode.unknown_fault_class = true;
        let report = evaluate_fault_benchmark(&plan(), &dataset(vec![episode])).unwrap();
        assert_eq!(report.unknown_fault_label_episode_count, 1);
        assert_eq!(report.verified_healthy_exposure_hours, 0.0);
        assert_eq!(report.fault_episode_count, 0);
    }
}
