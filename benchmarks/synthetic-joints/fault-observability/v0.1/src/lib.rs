#![forbid(unsafe_code)]

//! Event-level actuator fault observability metrics.
//!
//! M1 measures event detection, time-to-detect, exact compound-fault isolation,
//! false alarms per verified no-fault hour, and specimen-grouped MDFA.
//! It is an evaluator of externally supplied ground truth and detector alarm
//! episodes; it does not implement or validate a detector.

use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroundTruthEvidenceClass {
    IndependentPhysical,
    SourceAuthorPhysical,
    VendorPhysical,
    SyntheticFixture,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FaultEventTermination {
    FaultCleared,
    ObservationEnded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FaultEventDisposition {
    Detected,
    Missed,
    RightCensored,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlarmDisposition {
    FalseAlarm,
    ExactIsolation,
    MisIsolation,
    UnscoredOutsideExposure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MdfaDisposition {
    Qualified,
    NoVerifiedNoFaultExposure,
    FalseAlarmBudgetExceeded,
    InsufficientAmplitudeLevels,
    InsufficientIndependentSpecimens,
    InsufficientFollowup,
    NoAmplitudeMeetsSensitivityTarget,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CampaignPlan {
    pub campaign_id: String,
    pub detector_digest: String,
    pub dataset_digest: String,
    pub system_boundary_digest: String,
    pub operating_condition_digest: String,
    pub mission_set_digest: String,
    pub expected_fault_classes: Vec<String>,
    /// Required lower endpoint of the two-sided 95% Wilson interval.
    pub minimum_detection_wilson_lower_95: f64,
    /// False alarm events per hour of verified no-fault exposure.
    pub maximum_false_alarms_per_hour: f64,
    pub minimum_independent_specimens_per_amplitude: usize,
    pub evidence_class: GroundTruthEvidenceClass,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FaultEpisode {
    pub episode_id: String,
    pub specimen_id: String,
    pub fault_class: String,
    pub amplitude: f64,
    pub amplitude_unit: String,
    pub onset_s: f64,
    /// Fault-clear time or end of observable follow-up, according to termination.
    pub clear_s: f64,
    pub termination: FaultEventTermination,
    pub evidence_class: GroundTruthEvidenceClass,
    pub ground_truth_reference: String,
    pub raw_trace_digest: String,
    pub configuration_digest: String,
    pub system_boundary_digest: String,
    pub operating_condition_digest: String,
    pub mission_set_digest: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AlarmEpisode {
    pub alarm_id: String,
    pub specimen_id: String,
    /// First timestamp of a deduplicated alarm episode.
    pub alarm_time_s: f64,
    /// A set is required because compound faults can be active simultaneously.
    pub hypothesized_fault_classes: Vec<String>,
    pub detector_digest: String,
    pub channel_set_digest: String,
    pub source_reference: String,
    pub method: String,
    pub configuration_digest: String,
    pub system_boundary_digest: String,
    pub operating_condition_digest: String,
    pub mission_set_digest: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NoFaultExposure {
    pub exposure_id: String,
    pub specimen_id: String,
    pub start_s: f64,
    pub end_s: f64,
    pub evidence_class: GroundTruthEvidenceClass,
    pub source_reference: String,
    pub raw_trace_digest: String,
    pub configuration_digest: String,
    pub system_boundary_digest: String,
    pub operating_condition_digest: String,
    pub mission_set_digest: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FaultCampaign {
    pub plan: CampaignPlan,
    pub fault_episodes: Vec<FaultEpisode>,
    pub alarm_episodes: Vec<AlarmEpisode>,
    pub no_fault_exposure: Vec<NoFaultExposure>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FaultEventAssessment {
    pub episode_id: String,
    pub specimen_id: String,
    pub fault_class: String,
    pub amplitude: f64,
    pub amplitude_unit: String,
    pub disposition: FaultEventDisposition,
    pub time_to_detect_s: Option<f64>,
    pub matching_alarm_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AlarmAssessment {
    pub alarm_id: String,
    pub specimen_id: String,
    pub alarm_time_s: f64,
    pub disposition: AlarmDisposition,
    pub active_ground_truth_faults: Vec<String>,
    pub hypothesized_faults: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AmplitudeSensitivity {
    pub fault_class: String,
    pub amplitude: f64,
    pub amplitude_unit: String,
    /// Each independent specimen contributes exactly one trial in this stratum.
    /// Total unique specimens represented at this class/amplitude.
    pub independent_specimen_count: usize,
    /// Specimens with a completed fault episode (detected or confirmed missed).
    pub evaluable_specimen_count: usize,
    pub detected_specimen_count: usize,
    pub right_censored_specimen_count: usize,
    pub empirical_detection_rate: f64,
    pub wilson_95_lower: Option<f64>,
    pub wilson_95_upper: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MdfaResult {
    pub fault_class: String,
    pub disposition: MdfaDisposition,
    pub minimum_detectable_amplitude: Option<f64>,
    pub amplitude_unit: Option<String>,
    pub qualifying_amplitude_levels: Vec<f64>,
    pub required_wilson_lower_95: f64,
    pub maximum_false_alarms_per_hour: f64,
    pub minimum_independent_specimens_per_amplitude: usize,
    pub observed_false_alarms_per_hour: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FaultObservabilityReport {
    pub campaign_id: String,
    pub detector_digest: String,
    pub dataset_digest: String,
    pub system_boundary_digest: String,
    pub operating_condition_digest: String,
    pub mission_set_digest: String,

    pub fault_episode_count: usize,
    pub detected_fault_episode_count: usize,
    pub missed_fault_episode_count: usize,
    pub right_censored_fault_episode_count: usize,
    pub evaluable_fault_episode_count: usize,
    pub event_detection_rate: Option<f64>,
    pub detected_time_to_detect_median_s: Option<f64>,
    pub detected_time_to_detect_p95_s: Option<f64>,
    pub detected_time_to_detect_max_s: Option<f64>,

    pub alarm_episode_count: usize,
    pub false_alarm_episode_count: usize,
    pub exact_isolation_alarm_count: usize,
    pub mis_isolation_alarm_count: usize,
    pub unscored_alarm_count: usize,
    pub verified_no_fault_exposure_hours: f64,
    pub false_alarms_per_hour: Option<f64>,
    pub false_alarm_budget_pass: Option<bool>,

    pub fault_assessments: Vec<FaultEventAssessment>,
    pub alarm_assessments: Vec<AlarmAssessment>,
    pub amplitude_sensitivity: Vec<AmplitudeSensitivity>,
    pub mdfa: Vec<MdfaResult>,
    pub interpretation: String,
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn wilson_95(successes: usize, trials: usize) -> Option<(f64, f64)> {
    if trials == 0 {
        return None;
    }
    const Z: f64 = 1.959_963_984_540_054;
    let n = trials as f64;
    let p = successes as f64 / n;
    let z2 = Z * Z;
    let denominator = 1.0 + z2 / n;
    let center = (p + z2 / (2.0 * n)) / denominator;
    let half_width =
        Z * ((p * (1.0 - p) / n) + (z2 / (4.0 * n * n))).sqrt() / denominator;
    Some(((center - half_width).max(0.0), (center + half_width).min(1.0)))
}

fn overlaps(start_a: f64, end_a: f64, start_b: f64, end_b: f64) -> bool {
    start_a < end_b && start_b < end_a
}

fn validate_campaign(campaign: &FaultCampaign) -> Result<(), String> {
    let plan = &campaign.plan;
    if !nonempty(&plan.campaign_id)
        || !nonempty(&plan.detector_digest)
        || !nonempty(&plan.dataset_digest)
        || !nonempty(&plan.system_boundary_digest)
        || !nonempty(&plan.operating_condition_digest)
        || !nonempty(&plan.mission_set_digest)
    {
        return Err("campaign and detector/dataset/boundary/condition/mission digests are required".to_owned());
    }
    if !plan.minimum_detection_wilson_lower_95.is_finite()
        || !(0.0..=1.0).contains(&plan.minimum_detection_wilson_lower_95)
        || !plan.maximum_false_alarms_per_hour.is_finite()
        || plan.maximum_false_alarms_per_hour < 0.0
        || plan.minimum_independent_specimens_per_amplitude == 0
    {
        return Err("MDFA sensitivity threshold and false-alarm budget must be finite and in range".to_owned());
    }
    if plan.evidence_class == GroundTruthEvidenceClass::Unknown {
        return Err("campaign ground-truth evidence class cannot be Unknown".to_owned());
    }
    if plan.expected_fault_classes.is_empty() {
        return Err("at least one expected fault class is required".to_owned());
    }
    let mut expected_classes = BTreeSet::new();
    for class in &plan.expected_fault_classes {
        if !nonempty(class) || !expected_classes.insert(class.as_str()) {
            return Err("expected fault classes must be nonempty and unique".to_owned());
        }
    }
    if campaign.fault_episodes.is_empty() {
        return Err("at least one fault episode is required".to_owned());
    }

    let mut episode_ids = BTreeSet::new();
    let mut amplitude_trials: BTreeSet<(String, String, u64)> = BTreeSet::new();
    let mut amplitude_units: BTreeMap<String, String> = BTreeMap::new();
    let mut specimen_configurations: BTreeMap<String, String> = BTreeMap::new();
    let mut specimen_ids = BTreeSet::new();

    for episode in &campaign.fault_episodes {
        if !nonempty(&episode.episode_id)
            || !nonempty(&episode.specimen_id)
            || !nonempty(&episode.fault_class)
            || !nonempty(&episode.amplitude_unit)
            || !nonempty(&episode.ground_truth_reference)
            || !nonempty(&episode.raw_trace_digest)
            || !nonempty(&episode.configuration_digest)
        {
            return Err("fault episode identity, amplitude unit and ground-truth provenance are required".to_owned());
        }
        if !episode_ids.insert(episode.episode_id.as_str()) {
            return Err(format!("duplicate fault episode ID {}", episode.episode_id));
        }
        if !expected_classes.contains(episode.fault_class.as_str()) {
            return Err(format!("fault class {} is not in the frozen campaign taxonomy", episode.fault_class));
        }
        if !episode.amplitude.is_finite() || episode.amplitude < 0.0
            || !episode.onset_s.is_finite()
            || !episode.clear_s.is_finite()
            || episode.onset_s < 0.0
            || episode.clear_s <= episode.onset_s
        {
            return Err(format!("fault episode {} has invalid amplitude or time bounds", episode.episode_id));
        }
        if episode.evidence_class != plan.evidence_class {
            return Err(format!("fault episode {} evidence class differs from the campaign plan", episode.episode_id));
        }
        if episode.system_boundary_digest != plan.system_boundary_digest
            || episode.operating_condition_digest != plan.operating_condition_digest
            || episode.mission_set_digest != plan.mission_set_digest
        {
            return Err(format!("fault episode {} has a mismatched boundary/condition/mission digest", episode.episode_id));
        }
        if !nonempty(&episode.configuration_digest) {
            return Err(format!("fault episode {} is missing configuration digest", episode.episode_id));
        }
        match amplitude_units.get(&episode.fault_class) {
            Some(unit) if unit != &episode.amplitude_unit => {
                return Err(format!(
                    "fault class {} uses inconsistent amplitude units",
                    episode.fault_class
                ));
            }
            None => {
                amplitude_units.insert(episode.fault_class.clone(), episode.amplitude_unit.clone());
            }
            _ => {}
        }
        match specimen_configurations.get(&episode.specimen_id) {
            Some(configuration) if configuration != &episode.configuration_digest => {
                return Err(format!(
                    "specimen {} has inconsistent configuration digests across records",
                    episode.specimen_id
                ));
            }
            None => {
                specimen_configurations.insert(episode.specimen_id.clone(), episode.configuration_digest.clone());
            }
            _ => {}
        }
        let key = (
            episode.specimen_id.clone(),
            episode.fault_class.clone(),
            episode.amplitude.to_bits(),
        );
        if !amplitude_trials.insert(key) {
            return Err(format!(
                "specimen {} has repeated trials for fault {} at amplitude {}",
                episode.specimen_id, episode.fault_class, episode.amplitude
            ));
        }
        specimen_ids.insert(episode.specimen_id.as_str());
    }

    // Repeated episodes of the same fault class on one specimen may not overlap.
    for (index, left) in campaign.fault_episodes.iter().enumerate() {
        for right in campaign.fault_episodes.iter().skip(index + 1) {
            if left.specimen_id == right.specimen_id
                && left.fault_class == right.fault_class
                && overlaps(left.onset_s, left.clear_s, right.onset_s, right.clear_s)
            {
                return Err(format!(
                    "overlapping same-class ground-truth episodes on specimen {}",
                    left.specimen_id
                ));
            }
        }
    }

    let mut exposure_ids = BTreeSet::new();
    for exposure in &campaign.no_fault_exposure {
        if !nonempty(&exposure.exposure_id)
            || !nonempty(&exposure.specimen_id)
            || !nonempty(&exposure.source_reference)
            || !nonempty(&exposure.raw_trace_digest)
            || !nonempty(&exposure.configuration_digest)
        {
            return Err("no-fault exposure identity and trace provenance are required".to_owned());
        }
        if !exposure_ids.insert(exposure.exposure_id.as_str()) {
            return Err(format!("duplicate no-fault exposure ID {}", exposure.exposure_id));
        }
        if !exposure.start_s.is_finite()
            || !exposure.end_s.is_finite()
            || exposure.start_s < 0.0
            || exposure.end_s <= exposure.start_s
        {
            return Err(format!("no-fault exposure {} has invalid time bounds", exposure.exposure_id));
        }
        if exposure.evidence_class != plan.evidence_class {
            return Err(format!("no-fault exposure {} evidence class differs from campaign plan", exposure.exposure_id));
        }
        if exposure.system_boundary_digest != plan.system_boundary_digest
            || exposure.operating_condition_digest != plan.operating_condition_digest
            || exposure.mission_set_digest != plan.mission_set_digest
        {
            return Err(format!("no-fault exposure {} has mismatched boundary/condition/mission identity", exposure.exposure_id));
        }
        specimen_ids.insert(exposure.specimen_id.as_str());
        match specimen_configurations.get(&exposure.specimen_id) {
            Some(configuration) if configuration != &exposure.configuration_digest => {
                return Err(format!(
                    "specimen {} has inconsistent configuration digests across records",
                    exposure.specimen_id
                ));
            }
            None => {
                specimen_configurations.insert(exposure.specimen_id.clone(), exposure.configuration_digest.clone());
            }
            _ => {}
        }

        for episode in campaign.fault_episodes.iter().filter(|episode| episode.specimen_id == exposure.specimen_id) {
            if overlaps(exposure.start_s, exposure.end_s, episode.onset_s, episode.clear_s) {
                return Err(format!(
                    "no-fault exposure {} overlaps fault episode {}",
                    exposure.exposure_id, episode.episode_id
                ));
            }
            if episode.termination == FaultEventTermination::ObservationEnded
                && exposure.end_s > episode.clear_s
            {
                return Err(format!(
                    "no-fault exposure {} extends beyond observation end of right-censored fault {}",
                    exposure.exposure_id, episode.episode_id
                ));
            }
        }
    }

    // No-fault windows for one specimen may not overlap; otherwise exposure hours double-count time.
    for (index, left) in campaign.no_fault_exposure.iter().enumerate() {
        for right in campaign.no_fault_exposure.iter().skip(index + 1) {
            if left.specimen_id == right.specimen_id
                && overlaps(left.start_s, left.end_s, right.start_s, right.end_s)
            {
                return Err(format!("overlapping no-fault exposure windows on specimen {}", left.specimen_id));
            }
        }
    }

    let mut alarm_ids = BTreeSet::new();
    let mut alarm_instants: BTreeSet<(String, u64)> = BTreeSet::new();
    for alarm in &campaign.alarm_episodes {
        if !nonempty(&alarm.alarm_id)
            || !nonempty(&alarm.specimen_id)
            || !nonempty(&alarm.detector_digest)
            || !nonempty(&alarm.channel_set_digest)
            || !nonempty(&alarm.source_reference)
            || !nonempty(&alarm.method)
        {
            return Err("alarm identity, detector and evidence provenance are required".to_owned());
        }
        if !alarm_ids.insert(alarm.alarm_id.as_str()) {
            return Err(format!("duplicate alarm ID {}", alarm.alarm_id));
        }
        if !alarm.alarm_time_s.is_finite() || alarm.alarm_time_s < 0.0 {
            return Err(format!("alarm {} has invalid timestamp", alarm.alarm_id));
        }
        if !specimen_ids.contains(alarm.specimen_id.as_str()) {
            return Err(format!("alarm {} has no matching specimen in the campaign", alarm.alarm_id));
        }
        if alarm.detector_digest != plan.detector_digest
            || alarm.system_boundary_digest != plan.system_boundary_digest
            || alarm.operating_condition_digest != plan.operating_condition_digest
            || alarm.mission_set_digest != plan.mission_set_digest
        {
            return Err(format!("alarm {} has mismatched detector/boundary/condition/mission identity", alarm.alarm_id));
        }
        match specimen_configurations.get(&alarm.specimen_id) {
            Some(configuration) if configuration != &alarm.configuration_digest => {
                return Err(format!(
                    "specimen {} has inconsistent configuration digests across records",
                    alarm.specimen_id
                ));
            }
            None => {
                specimen_configurations.insert(alarm.specimen_id.clone(), alarm.configuration_digest.clone());
            }
            _ => {}
        }
        if alarm.hypothesized_fault_classes.is_empty() {
            return Err(format!("alarm {} has no hypothesized fault class", alarm.alarm_id));
        }
        let mut hypothesized = BTreeSet::new();
        for class in &alarm.hypothesized_fault_classes {
            if !expected_classes.contains(class.as_str()) || !hypothesized.insert(class.as_str()) {
                return Err(format!("alarm {} has unknown or duplicate hypothesized fault class", alarm.alarm_id));
            }
        }
        let instant_key = (alarm.specimen_id.clone(), alarm.alarm_time_s.to_bits());
        if !alarm_instants.insert(instant_key) {
            return Err(format!(
                "specimen {} has multiple alarm episodes at one instant; deduplicate alarm episodes before scoring",
                alarm.specimen_id
            ));
        }
    }

    Ok(())
}

fn active_fault_classes<'a>(
    specimen_id: &str,
    time_s: f64,
    episodes: &'a [FaultEpisode],
) -> Vec<&'a str> {
    let mut active: Vec<&str> = episodes
        .iter()
        .filter(|episode| {
            episode.specimen_id == specimen_id
                && time_s >= episode.onset_s
                && time_s < episode.clear_s
        })
        .map(|episode| episode.fault_class.as_str())
        .collect();
    active.sort_unstable();
    active.dedup();
    active
}

fn in_no_fault_exposure(specimen_id: &str, time_s: f64, exposures: &[NoFaultExposure]) -> bool {
    exposures.iter().any(|exposure| {
        exposure.specimen_id == specimen_id
            && time_s >= exposure.start_s
            && time_s < exposure.end_s
    })
}

fn percentile_nearest_rank(sorted_values: &[f64], percentile: f64) -> Option<f64> {
    if sorted_values.is_empty() || !(0.0..=1.0).contains(&percentile) {
        return None;
    }
    let rank = ((percentile * sorted_values.len() as f64).ceil() as usize)
        .max(1)
        .min(sorted_values.len());
    Some(sorted_values[rank - 1])
}

pub fn evaluate_fault_campaign(campaign: &FaultCampaign) -> Result<FaultObservabilityReport, String> {
    validate_campaign(campaign)?;
    let plan = &campaign.plan;

    let total_no_fault_seconds: f64 = campaign.no_fault_exposure
        .iter()
        .map(|exposure| exposure.end_s - exposure.start_s)
        .sum();
    if !total_no_fault_seconds.is_finite() {
        return Err("no-fault exposure duration overflowed".to_owned());
    }
    let verified_no_fault_exposure_hours = total_no_fault_seconds / 3600.0;

    let mut fault_assessments = Vec::with_capacity(campaign.fault_episodes.len());
    let mut detected_times = Vec::new();

    for episode in &campaign.fault_episodes {
        let matching_alarm = campaign.alarm_episodes
            .iter()
            .filter(|alarm| {
                alarm.specimen_id == episode.specimen_id
                    && alarm.alarm_time_s >= episode.onset_s
                    && alarm.alarm_time_s < episode.clear_s
                    && alarm.hypothesized_fault_classes.iter().any(|class| class == &episode.fault_class)
            })
            .min_by(|left, right| left.alarm_time_s.total_cmp(&right.alarm_time_s));

        let (disposition, time_to_detect_s, matching_alarm_id) = if let Some(alarm) = matching_alarm {
            let ttd = alarm.alarm_time_s - episode.onset_s;
            detected_times.push(ttd);
            (
                FaultEventDisposition::Detected,
                Some(ttd),
                Some(alarm.alarm_id.clone()),
            )
        } else if episode.termination == FaultEventTermination::FaultCleared {
            (FaultEventDisposition::Missed, None, None)
        } else {
            (FaultEventDisposition::RightCensored, None, None)
        };

        fault_assessments.push(FaultEventAssessment {
            episode_id: episode.episode_id.clone(),
            specimen_id: episode.specimen_id.clone(),
            fault_class: episode.fault_class.clone(),
            amplitude: episode.amplitude,
            amplitude_unit: episode.amplitude_unit.clone(),
            disposition,
            time_to_detect_s,
            matching_alarm_id,
        });
    }

    let mut alarm_assessments = Vec::with_capacity(campaign.alarm_episodes.len());
    let mut false_alarm_episode_count = 0usize;
    let mut exact_isolation_alarm_count = 0usize;
    let mut mis_isolation_alarm_count = 0usize;
    let mut unscored_alarm_count = 0usize;

    for alarm in &campaign.alarm_episodes {
        let active = active_fault_classes(
            &alarm.specimen_id,
            alarm.alarm_time_s,
            &campaign.fault_episodes,
        );
        let mut hypothesized = alarm.hypothesized_fault_classes.iter()
            .map(String::as_str)
            .collect::<Vec<_>>();
        hypothesized.sort_unstable();
        let disposition = if active.is_empty() {
            if in_no_fault_exposure(&alarm.specimen_id, alarm.alarm_time_s, &campaign.no_fault_exposure) {
                false_alarm_episode_count += 1;
                AlarmDisposition::FalseAlarm
            } else {
                unscored_alarm_count += 1;
                AlarmDisposition::UnscoredOutsideExposure
            }
        } else if active == hypothesized {
            exact_isolation_alarm_count += 1;
            AlarmDisposition::ExactIsolation
        } else {
            mis_isolation_alarm_count += 1;
            AlarmDisposition::MisIsolation
        };

        alarm_assessments.push(AlarmAssessment {
            alarm_id: alarm.alarm_id.clone(),
            specimen_id: alarm.specimen_id.clone(),
            alarm_time_s: alarm.alarm_time_s,
            disposition,
            active_ground_truth_faults: active.iter().map(|class| (*class).to_owned()).collect(),
            hypothesized_faults: hypothesized.iter().map(|class| (*class).to_owned()).collect(),
        });
    }

    let detected_fault_episode_count = fault_assessments
        .iter()
        .filter(|assessment| assessment.disposition == FaultEventDisposition::Detected)
        .count();
    let missed_fault_episode_count = fault_assessments
        .iter()
        .filter(|assessment| assessment.disposition == FaultEventDisposition::Missed)
        .count();
    let right_censored_fault_episode_count = fault_assessments
        .iter()
        .filter(|assessment| assessment.disposition == FaultEventDisposition::RightCensored)
        .count();
    let evaluable_fault_episode_count =
        detected_fault_episode_count + missed_fault_episode_count;
    let event_detection_rate = if evaluable_fault_episode_count == 0 {
        None
    } else {
        Some(detected_fault_episode_count as f64 / evaluable_fault_episode_count as f64)
    };

    detected_times.sort_by(f64::total_cmp);
    let detected_time_to_detect_median_s = percentile_nearest_rank(&detected_times, 0.5);
    let detected_time_to_detect_p95_s = percentile_nearest_rank(&detected_times, 0.95);
    let detected_time_to_detect_max_s = detected_times.last().copied();

    let false_alarms_per_hour = if verified_no_fault_exposure_hours > 0.0 {
        Some(false_alarm_episode_count as f64 / verified_no_fault_exposure_hours)
    } else {
        None
    };
    let false_alarm_budget_pass = false_alarms_per_hour
        .map(|rate| rate <= plan.maximum_false_alarms_per_hour);

    // One row per (fault class, amplitude, independent specimen) was enforced
    // during validation, so the Wilson denominator never counts repeated windows.
    let mut amplitude_groups: BTreeMap<(String, String, u64), (f64, usize, usize, usize, usize)> = BTreeMap::new();
    for assessment in &fault_assessments {
        let key = (
            assessment.fault_class.clone(),
            assessment.amplitude_unit.clone(),
            assessment.amplitude.to_bits(),
        );
        let entry = amplitude_groups.entry(key).or_insert((assessment.amplitude, 0, 0, 0, 0));
        entry.1 += 1;
        match assessment.disposition {
            FaultEventDisposition::Detected => {
                entry.2 += 1;
                entry.4 += 1;
            }
            FaultEventDisposition::RightCensored => entry.3 += 1,
            FaultEventDisposition::Missed => entry.4 += 1,
        }
    }

    let mut amplitude_sensitivity = Vec::with_capacity(amplitude_groups.len());
    for ((fault_class, amplitude_unit, _), (amplitude, specimen_count, detected_count, censored_count, evaluable_count)) in &amplitude_groups {
        let (wilson_95_lower, wilson_95_upper) =
            wilson_95(*detected_count, *evaluable_count)
                .map_or((None, None), |(lower, upper)| (Some(lower), Some(upper)));
        amplitude_sensitivity.push(AmplitudeSensitivity {
            fault_class: fault_class.clone(),
            amplitude: *amplitude,
            amplitude_unit: amplitude_unit.clone(),
            independent_specimen_count: *specimen_count,
            evaluable_specimen_count: *evaluable_count,
            detected_specimen_count: *detected_count,
            right_censored_specimen_count: *censored_count,
            empirical_detection_rate: *detected_count as f64 / *evaluable_count as f64,
            wilson_95_lower,
            wilson_95_upper,
        });
    }
    amplitude_sensitivity.sort_by(|left, right| {
        left.fault_class.cmp(&right.fault_class)
            .then_with(|| left.amplitude.total_cmp(&right.amplitude))
    });

    let mut mdfa = Vec::with_capacity(plan.expected_fault_classes.len());
    for fault_class in &plan.expected_fault_classes {
        let class_levels: Vec<&AmplitudeSensitivity> = amplitude_sensitivity.iter()
            .filter(|level| &level.fault_class == fault_class)
            .collect();

        let mut result = MdfaResult {
            fault_class: fault_class.clone(),
            disposition: MdfaDisposition::NoAmplitudeMeetsSensitivityTarget,
            minimum_detectable_amplitude: None,
            amplitude_unit: class_levels.first().map(|level| level.amplitude_unit.clone()),
            qualifying_amplitude_levels: Vec::new(),
            required_wilson_lower_95: plan.minimum_detection_wilson_lower_95,
            maximum_false_alarms_per_hour: plan.maximum_false_alarms_per_hour,
            minimum_independent_specimens_per_amplitude: plan.minimum_independent_specimens_per_amplitude,
            observed_false_alarms_per_hour: false_alarms_per_hour,
        };

        if false_alarms_per_hour.is_none() {
            result.disposition = MdfaDisposition::NoVerifiedNoFaultExposure;
        } else if !false_alarm_budget_pass.unwrap_or(false) {
            result.disposition = MdfaDisposition::FalseAlarmBudgetExceeded;
        } else if class_levels.len() < 2 {
            result.disposition = MdfaDisposition::InsufficientAmplitudeLevels;
        } else if class_levels.iter().any(|level| level.right_censored_specimen_count > 0) {
            result.disposition = MdfaDisposition::InsufficientFollowup;
        } else if class_levels.iter().any(|level| {
            level.independent_specimen_count < plan.minimum_independent_specimens_per_amplitude
        }) {
            result.disposition = MdfaDisposition::InsufficientIndependentSpecimens;
        } else {
            let qualifying_indices: Vec<usize> = (0..class_levels.len())
                .filter(|index| {
                    class_levels[*index..].iter().all(|level| {
                        level.wilson_95_lower.is_some_and(|lower| {
                            lower >= plan.minimum_detection_wilson_lower_95
                        })
                    })
                })
                .collect();
            if let Some(index) = qualifying_indices.first().copied() {
                result.disposition = MdfaDisposition::Qualified;
                result.minimum_detectable_amplitude = Some(class_levels[index].amplitude);
                result.qualifying_amplitude_levels = class_levels[index..]
                    .iter()
                    .map(|level| level.amplitude)
                    .collect();
            }
        }
        mdfa.push(result);
    }

    Ok(FaultObservabilityReport {
        campaign_id: plan.campaign_id.clone(),
        detector_digest: plan.detector_digest.clone(),
        dataset_digest: plan.dataset_digest.clone(),
        system_boundary_digest: plan.system_boundary_digest.clone(),
        operating_condition_digest: plan.operating_condition_digest.clone(),
        mission_set_digest: plan.mission_set_digest.clone(),
        fault_episode_count: fault_assessments.len(),
        detected_fault_episode_count,
        missed_fault_episode_count,
        right_censored_fault_episode_count,
        evaluable_fault_episode_count,
        event_detection_rate,
        detected_time_to_detect_median_s,
        detected_time_to_detect_p95_s,
        detected_time_to_detect_max_s,
        alarm_episode_count: campaign.alarm_episodes.len(),
        false_alarm_episode_count,
        exact_isolation_alarm_count,
        mis_isolation_alarm_count,
        unscored_alarm_count,
        verified_no_fault_exposure_hours,
        false_alarms_per_hour,
        false_alarm_budget_pass,
        fault_assessments,
        alarm_assessments,
        amplitude_sensitivity,
        mdfa,
        interpretation: "Event-level diagnostic metrics only. Detection, exact isolation, false alarms and MDFA are computed from supplied ground truth and deduplicated alarm episodes. Synthetic fixtures validate arithmetic only; this report does not establish physical fault-detection performance or a safety function.".to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const DETECTOR: &str = "detector-digest-v1";
    const DATASET: &str = "dataset-digest-v1";
    const BOUNDARY: &str = "installed-joint-boundary-v1";
    const CONDITION: &str = "48V-25C-cooling-A";
    const MISSION: &str = "fault-observability-mission-v1";
    const GEARBOX: &str = "gearbox_backlash";
    const BEARING: &str = "bearing_wear";

    fn plan() -> CampaignPlan {
        CampaignPlan {
            campaign_id: "synthetic-fault-campaign-v1".to_owned(),
            detector_digest: DETECTOR.to_owned(),
            dataset_digest: DATASET.to_owned(),
            system_boundary_digest: BOUNDARY.to_owned(),
            operating_condition_digest: CONDITION.to_owned(),
            mission_set_digest: MISSION.to_owned(),
            expected_fault_classes: vec![GEARBOX.to_owned(), BEARING.to_owned(), "encoder_dropout".to_owned()],
            minimum_detection_wilson_lower_95: 0.8,
            maximum_false_alarms_per_hour: 0.2,
            minimum_independent_specimens_per_amplitude: 16,
            evidence_class: GroundTruthEvidenceClass::SyntheticFixture,
        }
    }

    fn episode(
        specimen: &str,
        fault_class: &str,
        amplitude: f64,
        episode_id: &str,
    ) -> FaultEpisode {
        FaultEpisode {
            episode_id: episode_id.to_owned(),
            specimen_id: specimen.to_owned(),
            fault_class: fault_class.to_owned(),
            amplitude,
            amplitude_unit: "normalized_fault_severity".to_owned(),
            onset_s: 1.0,
            clear_s: 5.0,
            termination: FaultEventTermination::FaultCleared,
            evidence_class: GroundTruthEvidenceClass::SyntheticFixture,
            ground_truth_reference: format!("https://example.invalid/ground-truth/{episode_id}"),
            raw_trace_digest: format!("synthetic-trace-{episode_id}"),
            configuration_digest: format!("config-{specimen}"),
            system_boundary_digest: BOUNDARY.to_owned(),
            operating_condition_digest: CONDITION.to_owned(),
            mission_set_digest: MISSION.to_owned(),
        }
    }

    fn alarm(specimen: &str, time_s: f64, labels: &[&str], id: &str) -> AlarmEpisode {
        AlarmEpisode {
            alarm_id: id.to_owned(),
            specimen_id: specimen.to_owned(),
            alarm_time_s: time_s,
            hypothesized_fault_classes: labels.iter().map(|label| (*label).to_owned()).collect(),
            detector_digest: DETECTOR.to_owned(),
            channel_set_digest: "vibration-servo-metrology-channel-set-v1".to_owned(),
            source_reference: format!("https://example.invalid/detector-output/{id}"),
            method: "Synthetic alarm-episode fixture".to_owned(),
            configuration_digest: format!("config-{specimen}"),
            system_boundary_digest: BOUNDARY.to_owned(),
            operating_condition_digest: CONDITION.to_owned(),
            mission_set_digest: MISSION.to_owned(),
        }
    }

    fn exposure(specimen: &str, id: &str) -> NoFaultExposure {
        NoFaultExposure {
            exposure_id: id.to_owned(),
            specimen_id: specimen.to_owned(),
            start_s: 10.0,
            end_s: 610.0,
            evidence_class: GroundTruthEvidenceClass::SyntheticFixture,
            source_reference: format!("https://example.invalid/no-fault/{id}"),
            raw_trace_digest: format!("synthetic-no-fault-{id}"),
            configuration_digest: format!("config-{specimen}"),
            system_boundary_digest: BOUNDARY.to_owned(),
            operating_condition_digest: CONDITION.to_owned(),
            mission_set_digest: MISSION.to_owned(),
        }
    }

    fn known_campaign() -> FaultCampaign {
        let mut faults = Vec::new();
        let mut alarms = Vec::new();
        let mut no_fault = Vec::new();

        for (level, detected) in [(0.2, false), (0.3, true), (0.4, true)] {
            for index in 0..16 {
                let specimen = format!("spec-{level:.1}-{index:02}");
                let episode_id = format!("fault-{level:.1}-{index:02}");
                faults.push(episode(&specimen, GEARBOX, level, &episode_id));
                no_fault.push(exposure(&specimen, &format!("quiet-{level:.1}-{index:02}")));
                if detected {
                    alarms.push(alarm(&specimen, 2.0, &[GEARBOX], &format!("alarm-{level:.1}-{index:02}")));
                }
            }
        }

        // A single alarm in the known no-fault window: 1 false alarm / 8 h = 0.125/h.
        alarms.push(alarm("spec-0.3-00", 20.0, &[GEARBOX], "false-alarm-01"));

        FaultCampaign {
            plan: plan(),
            fault_episodes: faults,
            alarm_episodes: alarms,
            no_fault_exposure: no_fault,
        }
    }

    #[test]
    fn fault_001_event_detection_ttd_and_false_alarm_denominator_are_exact() {
        let report = evaluate_fault_campaign(&known_campaign()).unwrap();
        assert_eq!(report.fault_episode_count, 48);
        assert_eq!(report.detected_fault_episode_count, 32);
        assert_eq!(report.missed_fault_episode_count, 16);
        assert!((report.event_detection_rate.unwrap() - 32.0 / 48.0).abs() < 1.0e-12);
        assert_eq!(report.detected_time_to_detect_median_s, Some(1.0));
        assert_eq!(report.detected_time_to_detect_p95_s, Some(1.0));
        assert_eq!(report.detected_time_to_detect_max_s, Some(1.0));
        assert_eq!(report.verified_no_fault_exposure_hours, 8.0);
        assert_eq!(report.false_alarm_episode_count, 1);
        assert_eq!(report.false_alarms_per_hour, Some(0.125));
        assert_eq!(report.false_alarm_budget_pass, Some(true));
    }

    #[test]
    fn fault_002_mdfa_uses_independent_specimen_wilson_lower_bound() {
        let report = evaluate_fault_campaign(&known_campaign()).unwrap();
        let mdfa = report.mdfa.iter().find(|result| result.fault_class == GEARBOX).unwrap();
        assert_eq!(mdfa.disposition, MdfaDisposition::Qualified);
        assert_eq!(mdfa.minimum_detectable_amplitude, Some(0.3));
        assert_eq!(mdfa.qualifying_amplitude_levels, vec![0.3, 0.4]);
        let low = report.amplitude_sensitivity.iter().find(|level| level.amplitude == 0.2).unwrap();
        let threshold = report.amplitude_sensitivity.iter().find(|level| level.amplitude == 0.3).unwrap();
        assert_eq!(low.independent_specimen_count, 16);
        assert_eq!(low.detected_specimen_count, 0);
        assert!(low.wilson_95_lower.unwrap() < 0.8);
        assert_eq!(threshold.independent_specimen_count, 16);
        assert_eq!(threshold.detected_specimen_count, 16);
        assert!(threshold.wilson_95_lower.unwrap() >= 0.8);
    }

    #[test]
    fn fault_003_compound_fault_requires_exact_set_isolation() {
        let mut campaign = FaultCampaign {
            plan: plan(),
            fault_episodes: vec![
                episode("compound-specimen", GEARBOX, 0.3, "compound-backlash"),
                episode("compound-specimen", BEARING, 0.4, "compound-bearing"),
            ],
            alarm_episodes: vec![alarm("compound-specimen", 2.0, &[GEARBOX, BEARING], "compound-alarm")],
            no_fault_exposure: vec![exposure("compound-specimen", "compound-quiet")],
        };
        let report = evaluate_fault_campaign(&campaign).unwrap();
        assert_eq!(report.detected_fault_episode_count, 2);
        assert_eq!(report.exact_isolation_alarm_count, 1);
        assert_eq!(report.mis_isolation_alarm_count, 0);
        assert_eq!(
            report.alarm_assessments[0].active_ground_truth_faults,
            vec![BEARING.to_owned(), GEARBOX.to_owned()]
        );
        assert_eq!(
            report.alarm_assessments[0].hypothesized_faults,
            vec![BEARING.to_owned(), GEARBOX.to_owned()]
        );

        campaign.alarm_episodes[0].hypothesized_fault_classes = vec![GEARBOX.to_owned()];
        let partial = evaluate_fault_campaign(&campaign).unwrap();
        assert_eq!(partial.detected_fault_episode_count, 1);
        assert_eq!(partial.missed_fault_episode_count, 1);
        assert_eq!(partial.exact_isolation_alarm_count, 0);
        assert_eq!(partial.mis_isolation_alarm_count, 1);
    }

    #[test]
    fn fault_004_alarm_before_onset_does_not_count_as_detection() {
        let mut campaign = known_campaign();
        campaign.alarm_episodes.retain(|alarm| alarm.specimen_id != "spec-0.2-00");
        campaign.alarm_episodes.push(alarm("spec-0.2-00", 0.5, &[GEARBOX], "pre-onset-alarm"));
        let report = evaluate_fault_campaign(&campaign).unwrap();
        let event = report.fault_assessments.iter().find(|event| event.specimen_id == "spec-0.2-00").unwrap();
        assert_eq!(event.disposition, FaultEventDisposition::Missed);
        assert_eq!(event.time_to_detect_s, None);
        assert_eq!(report.unscored_alarm_count, 1);
    }

    #[test]
    fn fault_005_wrong_label_during_fault_is_mis_isolation_not_true_detection() {
        let mut campaign = known_campaign();
        campaign.alarm_episodes.retain(|alarm| alarm.specimen_id != "spec-0.2-00");
        campaign.alarm_episodes.push(alarm("spec-0.2-00", 2.0, &[BEARING], "wrong-label-alarm"));
        let report = evaluate_fault_campaign(&campaign).unwrap();
        let event = report.fault_assessments.iter().find(|event| event.specimen_id == "spec-0.2-00").unwrap();
        assert_eq!(event.disposition, FaultEventDisposition::Missed);
        assert_eq!(report.mis_isolation_alarm_count, 1);
        assert_eq!(report.false_alarm_episode_count, 1);
    }

    #[test]
    fn fault_006_alarm_after_fault_clear_is_not_detection() {
        let mut campaign = known_campaign();
        campaign.alarm_episodes.retain(|alarm| alarm.specimen_id != "spec-0.2-00");
        campaign.alarm_episodes.push(alarm("spec-0.2-00", 6.0, &[GEARBOX], "post-clear-alarm"));
        let report = evaluate_fault_campaign(&campaign).unwrap();
        let event = report.fault_assessments.iter().find(|event| event.specimen_id == "spec-0.2-00").unwrap();
        assert_eq!(event.disposition, FaultEventDisposition::Missed);
        assert_eq!(report.unscored_alarm_count, 1);
    }

    #[test]
    fn fault_007_no_fault_exposure_is_required_for_far_and_mdfa() {
        let mut campaign = known_campaign();
        campaign.no_fault_exposure.clear();
        let report = evaluate_fault_campaign(&campaign).unwrap();
        assert_eq!(report.verified_no_fault_exposure_hours, 0.0);
        assert_eq!(report.false_alarms_per_hour, None);
        assert_eq!(report.false_alarm_budget_pass, None);
        assert!(report.mdfa.iter().all(|result| result.disposition == MdfaDisposition::NoVerifiedNoFaultExposure));
    }

    #[test]
    fn fault_008_excessive_far_suppresses_mdfa() {
        let mut campaign = known_campaign();
        campaign.plan.maximum_false_alarms_per_hour = 0.1;
        let report = evaluate_fault_campaign(&campaign).unwrap();
        assert_eq!(report.false_alarm_budget_pass, Some(false));
        assert!(report.mdfa.iter().all(|result| result.disposition == MdfaDisposition::FalseAlarmBudgetExceeded));
    }

    #[test]
    fn fault_009_no_fault_windows_may_not_overlap_fault_ground_truth() {
        let mut campaign = known_campaign();
        campaign.no_fault_exposure[0].start_s = 4.0;
        assert!(evaluate_fault_campaign(&campaign).is_err());
    }

    #[test]
    fn fault_010_duplicate_specimen_fault_amplitude_trial_is_rejected() {
        let mut campaign = known_campaign();
        let duplicate = campaign.fault_episodes[0].clone();
        campaign.fault_episodes.push(duplicate);
        assert!(evaluate_fault_campaign(&campaign).is_err());
    }

    #[test]
    fn fault_011_repeated_alarm_instants_must_be_deduplicated() {
        let mut campaign = known_campaign();
        let existing = campaign.alarm_episodes[0].clone();
        let mut duplicate = existing.clone();
        duplicate.alarm_id = "duplicate-alarm-id".to_owned();
        campaign.alarm_episodes.push(duplicate);
        assert!(evaluate_fault_campaign(&campaign).is_err());
    }

    #[test]
    fn fault_012_alarm_outside_declared_fault_and_no_fault_exposure_is_unscored() {
        let mut campaign = known_campaign();
        campaign.alarm_episodes.push(alarm("spec-0.2-00", 700.0, &[GEARBOX], "outside-window-alarm"));
        let report = evaluate_fault_campaign(&campaign).unwrap();
        assert_eq!(report.unscored_alarm_count, 1);
    }

    #[test]
    fn fault_013_overlapping_fault_classes_are_allowed_for_compound_events() {
        let campaign = FaultCampaign {
            plan: plan(),
            fault_episodes: vec![
                episode("compound", GEARBOX, 0.3, "gear-episode"),
                episode("compound", BEARING, 0.4, "bearing-episode"),
            ],
            alarm_episodes: vec![alarm("compound", 2.0, &[GEARBOX, BEARING], "compound-alarm")],
            no_fault_exposure: vec![exposure("compound", "compound-no-fault")],
        };
        assert!(evaluate_fault_campaign(&campaign).is_ok());
    }

    #[test]
    fn fault_014_same_class_overlapping_ground_truth_episodes_reject() {
        let mut campaign = FaultCampaign {
            plan: plan(),
            fault_episodes: vec![
                episode("same-class", GEARBOX, 0.3, "gear-episode-a"),
                episode("same-class", GEARBOX, 0.4, "gear-episode-b"),
            ],
            alarm_episodes: Vec::new(),
            no_fault_exposure: vec![exposure("same-class", "no-fault")],
        };
        campaign.fault_episodes[1].onset_s = 3.0;
        campaign.fault_episodes[1].clear_s = 6.0;
        assert!(evaluate_fault_campaign(&campaign).is_err());
    }

    #[test]
    fn fault_015_unknown_or_unexpected_fault_class_rejects() {
        let mut campaign = known_campaign();
        campaign.fault_episodes[0].fault_class = "unregistered-fault".to_owned();
        assert!(evaluate_fault_campaign(&campaign).is_err());

        let mut campaign = known_campaign();
        campaign.plan.evidence_class = GroundTruthEvidenceClass::Unknown;
        assert!(evaluate_fault_campaign(&campaign).is_err());
    }

    #[test]
    fn fault_016_boundary_condition_and_mission_digest_mismatch_rejects() {
        let mut campaign = known_campaign();
        campaign.fault_episodes[0].operating_condition_digest = "other-condition".to_owned();
        assert!(evaluate_fault_campaign(&campaign).is_err());

        let mut campaign = known_campaign();
        campaign.alarm_episodes[0].mission_set_digest = "other-mission".to_owned();
        assert!(evaluate_fault_campaign(&campaign).is_err());
    }

    #[test]
    fn fault_017_amplitude_levels_must_have_consistent_units() {
        let mut campaign = known_campaign();
        campaign.fault_episodes[0].amplitude_unit = "N m".to_owned();
        // Mixed units for one fault class make the amplitude sweep uninterpretable.
        assert!(evaluate_fault_campaign(&campaign).is_err());
    }

    #[test]
    fn fault_018_configuration_digest_must_match_across_specimen_records() {
        let mut campaign = known_campaign();
        campaign.alarm_episodes[0].configuration_digest = "stale-config".to_owned();
        assert!(evaluate_fault_campaign(&campaign).is_err());

        let mut campaign = known_campaign();
        campaign.no_fault_exposure[0].configuration_digest = "stale-config".to_owned();
        assert!(evaluate_fault_campaign(&campaign).is_err());
    }

    #[test]
    fn fault_019_fault_clear_time_is_exclusive_for_detection() {
        let mut campaign = known_campaign();
        campaign.alarm_episodes.retain(|alarm| alarm.specimen_id != "spec-0.2-00");
        campaign.alarm_episodes.push(alarm("spec-0.2-00", 5.0, &[GEARBOX], "alarm-at-clear"));
        let report = evaluate_fault_campaign(&campaign).unwrap();
        let event = report.fault_assessments.iter()
            .find(|event| event.specimen_id == "spec-0.2-00")
            .unwrap();
        assert_eq!(event.disposition, FaultEventDisposition::Missed);
        assert_eq!(report.unscored_alarm_count, 1);
    }

    #[test]
    fn fault_020_insufficient_amplitude_grid_does_not_claim_mdfa() {
        let mut campaign = known_campaign();
        campaign.fault_episodes.retain(|episode| episode.fault_class != GEARBOX || episode.amplitude == 0.3);
        let report = evaluate_fault_campaign(&campaign).unwrap();
        let mdfa = report.mdfa.iter().find(|result| result.fault_class == GEARBOX).unwrap();
        assert_eq!(mdfa.disposition, MdfaDisposition::InsufficientAmplitudeLevels);
        assert_eq!(mdfa.minimum_detectable_amplitude, None);
    }

    #[test]
    fn fault_021_median_and_nearest_rank_p95_are_deterministic() {
        let mut campaign = known_campaign();
        for (index, alarm) in campaign.alarm_episodes.iter_mut().filter(|alarm| alarm.alarm_time_s == 2.0).enumerate() {
            alarm.alarm_time_s = 1.25 + index as f64 * 0.01;
        }
        let report = evaluate_fault_campaign(&campaign).unwrap();
        assert!(report.detected_time_to_detect_median_s.unwrap() >= 0.25);
        assert!(report.detected_time_to_detect_p95_s.unwrap() >= report.detected_time_to_detect_median_s.unwrap());
        assert!(report.detected_time_to_detect_max_s.unwrap() >= report.detected_time_to_detect_p95_s.unwrap());
    }

    #[test]
    fn fault_022_no_fault_exposure_windows_may_not_overlap_each_other() {
        let mut campaign = known_campaign();
        let mut duplicate_window = campaign.no_fault_exposure[0].clone();
        duplicate_window.exposure_id = "overlapping-window".to_owned();
        duplicate_window.start_s = 20.0;
        duplicate_window.end_s = 40.0;
        campaign.no_fault_exposure.push(duplicate_window);
        assert!(evaluate_fault_campaign(&campaign).is_err());
    }

    #[test]
    fn fault_023_independent_specimen_minimum_is_predeclared_for_mdfa() {
        let mut campaign = known_campaign();
        campaign.fault_episodes.retain(|episode| {
            !(episode.fault_class == GEARBOX
                && episode.amplitude == 0.3
                && episode.specimen_id.ends_with("15"))
        });
        let report = evaluate_fault_campaign(&campaign).unwrap();
        let mdfa = report.mdfa.iter().find(|result| result.fault_class == GEARBOX).unwrap();
        assert_eq!(mdfa.disposition, MdfaDisposition::InsufficientIndependentSpecimens);
        assert_eq!(mdfa.minimum_independent_specimens_per_amplitude, 16);
    }

    #[test]
    fn fault_024_right_censored_fault_is_not_counted_as_a_confirmed_miss() {
        let mut campaign = known_campaign();
        let censored_id = campaign.fault_episodes.iter()
            .find(|episode| episode.specimen_id == "spec-0.2-00")
            .unwrap()
            .episode_id
            .clone();
        let censored = campaign.fault_episodes.iter_mut()
            .find(|episode| episode.episode_id == censored_id)
            .unwrap();
        censored.termination = FaultEventTermination::ObservationEnded;
        campaign.no_fault_exposure.retain(|window| window.specimen_id != "spec-0.2-00");
        let report = evaluate_fault_campaign(&campaign).unwrap();
        assert_eq!(report.right_censored_fault_episode_count, 1);
        assert_eq!(report.missed_fault_episode_count, 15);
        let censored_level = report.amplitude_sensitivity.iter()
            .find(|level| level.fault_class == GEARBOX && level.amplitude == 0.2)
            .unwrap();
        assert_eq!(censored_level.independent_specimen_count, 16);
        assert_eq!(censored_level.evaluable_specimen_count, 15);
        assert_eq!(censored_level.right_censored_specimen_count, 1);
        assert_eq!(report.evaluable_fault_episode_count, 47);
        assert!((report.event_detection_rate.unwrap() - 32.0 / 47.0).abs() < 1.0e-12);
        let mdfa = report.mdfa.iter().find(|result| result.fault_class == GEARBOX).unwrap();
        assert_eq!(mdfa.disposition, MdfaDisposition::InsufficientFollowup);
    }

    #[test]
    fn fault_025_all_censored_amplitude_group_has_no_wilson_interval() {
        let mut campaign = known_campaign();
        for episode in campaign.fault_episodes.iter_mut().filter(|episode| episode.fault_class == GEARBOX && episode.amplitude == 0.2) {
            episode.termination = FaultEventTermination::ObservationEnded;
        }
        campaign.no_fault_exposure.retain(|window| {
            !window.specimen_id.starts_with("spec-0.2-")
        });
        let report = evaluate_fault_campaign(&campaign).unwrap();
        let level = report.amplitude_sensitivity.iter()
            .find(|level| level.fault_class == GEARBOX && level.amplitude == 0.2)
            .unwrap();
        assert_eq!(level.independent_specimen_count, 16);
        assert_eq!(level.evaluable_specimen_count, 0);
        assert_eq!(level.right_censored_specimen_count, 16);
        assert_eq!(level.wilson_95_lower, None);
        assert_eq!(level.wilson_95_upper, None);
        let mdfa = report.mdfa.iter().find(|result| result.fault_class == GEARBOX).unwrap();
        assert_eq!(mdfa.disposition, MdfaDisposition::InsufficientFollowup);
    }

    #[test]
    fn fault_026_no_fault_exposure_cannot_follow_right_censored_observation() {
        let mut campaign = known_campaign();
        let censored = campaign.fault_episodes.iter_mut()
            .find(|episode| episode.specimen_id == "spec-0.2-00")
            .unwrap();
        censored.termination = FaultEventTermination::ObservationEnded;
        assert!(evaluate_fault_campaign(&campaign).is_err());
    }

    #[test]
    fn fault_027_unknown_alarm_hypotheses_reject() {
        let mut campaign = known_campaign();
        campaign.alarm_episodes[0].hypothesized_fault_classes = vec!["mystery-fault".to_owned()];
        assert!(evaluate_fault_campaign(&campaign).is_err());
    }

    #[test]
    fn fault_028_expected_fault_classes_must_be_unique_and_nonempty() {
        let mut campaign = known_campaign();
        campaign.plan.expected_fault_classes.push(GEARBOX.to_owned());
        assert!(evaluate_fault_campaign(&campaign).is_err());
    }

    #[test]
    fn fault_029_zero_alarm_is_not_a_correct_diagnosis() {
        let mut campaign = known_campaign();
        campaign.alarm_episodes.clear();
        let report = evaluate_fault_campaign(&campaign).unwrap();
        assert_eq!(report.detected_fault_episode_count, 0);
        assert_eq!(report.missed_fault_episode_count, 48);
        assert_eq!(report.event_detection_rate, Some(0.0));
    }

    #[test]
    fn fault_030_specimen_amplitude_group_reports_wilson_interval() {
        let report = evaluate_fault_campaign(&known_campaign()).unwrap();
        let perfect = report.amplitude_sensitivity.iter()
            .find(|level| level.fault_class == GEARBOX && level.amplitude == 0.3)
            .unwrap();
        assert!(perfect.wilson_95_lower.unwrap() < 1.0);
        assert!(perfect.wilson_95_upper.unwrap() <= 1.0);
        assert!(perfect.wilson_95_lower.unwrap() >= 0.8);
    }
}
