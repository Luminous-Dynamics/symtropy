// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

#![forbid(unsafe_code)]

//! Deterministic incident- and alarm-episode evaluation primitives for CivBench.
//!
//! This crate is intentionally dependency-free. It scores a supplied set of
//! evaluator-owned events, unresolved intervals and candidate alarms. It does
//! not ingest SCADA, run a detector, model hydraulics, attribute malicious
//! intent, authorize control actions, or claim service restoration.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

/// A confirmed incident class supported by the evaluator's truth source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EventClass {
    /// Physical leak or hydraulic event.
    Leak,
    /// Instrument, sensor or calibration failure.
    SensorFault,
    /// Communication, gateway or telemetry failure.
    CommunicationFault,
    /// Ordinary process drift or a non-malicious operating transition.
    ProcessDrift,
    /// Independently supported cyber compromise or unauthorized action.
    CyberCompromise,
}

/// A candidate alarm's asserted class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AlarmClass {
    /// An asserted event class.
    Classified(EventClass),
    /// Alarm detected an event but did not diagnose its cause.
    Unclassified,
    /// Constituent alarms in one episode assert conflicting classes.
    Conflicting,
}

/// A confirmed event interval held by the evaluator, not the candidate policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TruthEvent {
    /// Immutable event identifier.
    pub event_id: String,
    /// Scenario/block identity.
    pub scenario_id: String,
    /// Identity of the source establishing event truth.
    pub source_id: String,
    /// Independently supported event class.
    pub class: EventClass,
    /// Event onset on the declared relative timeline.
    pub onset_ms: u64,
    /// Event end on the declared relative timeline.
    pub end_ms: u64,
}

/// An interval with unresolved truth. It is excluded from false-positive scoring.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownInterval {
    /// Immutable unresolved-interval identifier.
    pub interval_id: String,
    /// Scenario/block identity.
    pub scenario_id: String,
    /// Inclusive interval start.
    pub start_ms: u64,
    /// Inclusive interval end.
    pub end_ms: u64,
}

/// A detector-produced alarm. A missing clear time means it remained open
/// through the evaluation horizon.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Alarm {
    /// Immutable alarm identifier.
    pub alarm_id: String,
    /// Scenario/block identity.
    pub scenario_id: String,
    /// Detector/source identity. Episodes never merge across sources.
    pub source_id: String,
    /// The alarm's asserted cause, if any.
    pub class: AlarmClass,
    /// Alarm onset on the declared relative timeline.
    pub start_ms: u64,
    /// Clear time, or None if still open at the evaluation horizon.
    pub clear_ms: Option<u64>,
}

/// Frozen event-matching and alarm-episode parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EvaluationConfig {
    /// Monitored duration for each scenario, in milliseconds.
    pub monitored_duration_ms: u64,
    /// Merge overlapping alarms or alarms separated by no more than this gap.
    pub merge_gap_ms: u64,
    /// An alarm may be early by at most this amount and still be timely.
    pub allowed_lead_ms: u64,
    /// Maximum delay after event onset for a timely detection.
    pub maximum_detection_delay_ms: u64,
    /// Additional post-event interval in which a late alarm can be associated.
    pub late_association_grace_ms: u64,
}

/// Whether an event was detected in time or only associated after the deadline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchKind {
    /// Alarm episode started within the timely onset window.
    Timely,
    /// Alarm episode was associated after the timely onset window.
    Late,
}

/// A deterministic one-to-one event/episode association.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchRecord {
    /// Evaluator-owned event identifier.
    pub event_id: String,
    /// Candidate alarm episode identifier.
    pub episode_id: String,
    /// Ground-truth event class.
    pub truth_class: EventClass,
    /// Episode diagnosis after constituent-alarm aggregation.
    pub reported_class: AlarmClass,
    /// Timely or late association.
    pub kind: MatchKind,
    /// Episode onset minus event onset; negative values are early warnings.
    pub onset_offset_ms: i64,
    /// None for unclassified/conflicting alarm episodes.
    pub class_correct: Option<bool>,
}

/// One cell in the event-level diagnosis confusion matrix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassConfusionCell {
    /// Ground-truth event class.
    pub truth_class: EventClass,
    /// Reported class (including unclassified/conflicting).
    pub reported_class: AlarmClass,
    /// Number of timely matched events in this cell.
    pub count: usize,
}

/// Computed event-level metrics and the full deterministic association record.
#[derive(Debug, Clone, PartialEq)]
pub struct EvaluationReport {
    /// Number of confirmed truth events in the scored corpus.
    pub confirmed_event_count: usize,
    /// Number of confirmed events detected within the timely onset window.
    pub timely_event_count: usize,
    /// Timely detections divided by all scorable alarm episodes; None if no episodes.
    pub event_precision: Option<f64>,
    /// Timely detections divided by confirmed events; None if no eligible events.
    pub event_onset_recall: Option<f64>,
    /// Event-level diagnosis confusion for timely matched events.
    pub class_confusion: Vec<ClassConfusionCell>,
    /// Number of unmatched-timely events with a late associated episode.
    pub late_associated_event_count: usize,
    /// Event identifiers with an associated but late alarm episode.
    pub late_associated_event_ids: Vec<String>,
    /// Confirmed events with neither a timely nor late-associated episode.
    pub missed_event_ids: Vec<String>,
    /// Timely and late one-to-one associations, sorted by event identifier.
    pub matches: Vec<MatchRecord>,
    /// Additional alarm episodes associated with already matched events.
    pub fragmentation_episode_ids: Vec<String>,
    /// Episodes overlapping an unresolved truth interval.
    pub unscorable_episode_ids: Vec<String>,
    /// Unmatched episodes outside unresolved truth intervals and event windows.
    pub false_alarm_episode_ids: Vec<String>,
    /// Total monitored system-days across the scenario blocks.
    pub monitored_system_days: f64,
    /// False alarm episodes divided by monitored system-days.
    pub false_alarm_episodes_per_system_day: f64,
    /// Union duration of false alarm episodes, avoiding cross-source double count.
    pub false_alarm_duration_ms: u64,
    /// Union duration of all alarm-active intervals divided by monitoring exposure.
    pub alarm_duty_cycle_ppm: u64,
    /// Nearest-rank median of non-negative timely detection delays.
    pub p50_detection_delay_ms: Option<i64>,
    /// Nearest-rank 95th percentile of non-negative timely detection delays.
    pub p95_detection_delay_ms: Option<i64>,
    /// Nearest-rank median of early-warning lead times.
    pub p50_early_lead_ms: Option<i64>,
    /// Nearest-rank 95th percentile of early-warning lead times.
    pub p95_early_lead_ms: Option<i64>,
}

impl EvaluationReport {
    /// Render a stable, line-oriented receipt with fixed field order.
    ///
    /// This is a deterministic report representation, not a cryptographic
    /// signature and not a proof that source observations are true.
    pub fn canonical_text(&self) -> String {
        let mut out = String::new();
        let _ = writeln!(&mut out, "CIVBENCH_EVENT_EVALUATION_V0.1");
        let _ = writeln!(&mut out, "confirmed_events={}", self.confirmed_event_count);
        let _ = writeln!(&mut out, "timely_events={}", self.timely_event_count);
        let _ = writeln!(
            &mut out,
            "event_precision={}",
            option_f64(self.event_precision)
        );
        let _ = writeln!(
            &mut out,
            "event_onset_recall={}",
            option_f64(self.event_onset_recall)
        );
        for cell in &self.class_confusion {
            let _ = writeln!(
                &mut out,
                "class_confusion={},{},{}",
                event_class_name(cell.truth_class),
                alarm_class_name(cell.reported_class),
                cell.count
            );
        }
        let _ = writeln!(
            &mut out,
            "late_associated_events={}",
            self.late_associated_event_count
        );
        let _ = writeln!(
            &mut out,
            "late_associated_event_ids={}",
            self.late_associated_event_ids.join(",")
        );
        let _ = writeln!(
            &mut out,
            "missed_event_ids={}",
            self.missed_event_ids.join(",")
        );
        let _ = writeln!(
            &mut out,
            "fragmentation_episode_ids={}",
            self.fragmentation_episode_ids.join(",")
        );
        let _ = writeln!(
            &mut out,
            "unscorable_episode_ids={}",
            self.unscorable_episode_ids.join(",")
        );
        let _ = writeln!(
            &mut out,
            "false_alarm_episode_ids={}",
            self.false_alarm_episode_ids.join(",")
        );
        let _ = writeln!(
            &mut out,
            "monitored_system_days={:.9}",
            self.monitored_system_days
        );
        let _ = writeln!(
            &mut out,
            "false_alarm_episodes_per_system_day={:.9}",
            self.false_alarm_episodes_per_system_day
        );
        let _ = writeln!(
            &mut out,
            "false_alarm_duration_ms={}",
            self.false_alarm_duration_ms
        );
        let _ = writeln!(
            &mut out,
            "alarm_duty_cycle_ppm={}",
            self.alarm_duty_cycle_ppm
        );
        let _ = writeln!(
            &mut out,
            "p50_detection_delay_ms={}",
            option_i64(self.p50_detection_delay_ms)
        );
        let _ = writeln!(
            &mut out,
            "p95_detection_delay_ms={}",
            option_i64(self.p95_detection_delay_ms)
        );
        let _ = writeln!(
            &mut out,
            "p50_early_lead_ms={}",
            option_i64(self.p50_early_lead_ms)
        );
        let _ = writeln!(
            &mut out,
            "p95_early_lead_ms={}",
            option_i64(self.p95_early_lead_ms)
        );
        for record in &self.matches {
            let kind = match record.kind {
                MatchKind::Timely => "timely",
                MatchKind::Late => "late",
            };
            let class = match record.class_correct {
                Some(true) => "correct",
                Some(false) => "incorrect",
                None => "unclassified_or_conflicting",
            };
            let _ = writeln!(
                &mut out,
                "match={},{},{},{},{},{},{}",
                record.event_id,
                record.episode_id,
                kind,
                record.onset_offset_ms,
                event_class_name(record.truth_class),
                alarm_class_name(record.reported_class),
                class
            );
        }
        out
    }
}

fn match_kind_rank(kind: MatchKind) -> u8 {
    match kind {
        MatchKind::Timely => 0,
        MatchKind::Late => 1,
    }
}

fn option_i64(value: Option<i64>) -> String {
    value.map_or_else(|| "none".to_owned(), |v| v.to_string())
}

fn option_f64(value: Option<f64>) -> String {
    value.map_or_else(|| "none".to_owned(), |v| format!("{v:.9}"))
}

fn event_class_name(class: EventClass) -> &'static str {
    match class {
        EventClass::Leak => "leak",
        EventClass::SensorFault => "sensor_fault",
        EventClass::CommunicationFault => "communication_fault",
        EventClass::ProcessDrift => "process_drift",
        EventClass::CyberCompromise => "cyber_compromise",
    }
}

fn alarm_class_name(class: AlarmClass) -> &'static str {
    match class {
        AlarmClass::Classified(value) => event_class_name(value),
        AlarmClass::Unclassified => "unclassified",
        AlarmClass::Conflicting => "conflicting",
    }
}

/// Input validation or matching failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvaluationError {
    /// Stable machine-readable error category.
    pub code: &'static str,
    /// Field or offending identifier.
    pub subject: String,
}

impl EvaluationError {
    fn new(code: &'static str, subject: impl Into<String>) -> Self {
        Self {
            code,
            subject: subject.into(),
        }
    }
}

impl std::fmt::Display for EvaluationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.subject)
    }
}

impl std::error::Error for EvaluationError {}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Episode {
    episode_id: String,
    scenario_id: String,
    source_id: String,
    class: AlarmClass,
    start_ms: u64,
    end_ms: u64,
    constituent_alarm_ids: Vec<String>,
}

fn valid_id(id: &str) -> bool {
    !id.trim().is_empty()
}

fn insert_unique(
    set: &mut BTreeSet<String>,
    id: &str,
    kind: &'static str,
) -> Result<(), EvaluationError> {
    if !valid_id(id) {
        return Err(EvaluationError::new("empty_identifier", kind));
    }
    if !set.insert(id.to_owned()) {
        return Err(EvaluationError::new("duplicate_identifier", id));
    }
    Ok(())
}

fn validate_inputs(
    events: &[TruthEvent],
    unknown: &[UnknownInterval],
    alarms: &[Alarm],
    config: EvaluationConfig,
) -> Result<BTreeSet<String>, EvaluationError> {
    if config.monitored_duration_ms == 0 {
        return Err(EvaluationError::new(
            "invalid_monitored_duration",
            "must be positive",
        ));
    }

    let mut scenario_ids = BTreeSet::new();
    let mut event_ids = BTreeSet::new();
    for event in events {
        insert_unique(&mut event_ids, &event.event_id, "event_id")?;
        if !valid_id(&event.scenario_id) || !valid_id(&event.source_id) {
            return Err(EvaluationError::new(
                "empty_event_source_or_scenario",
                event.event_id.as_str(),
            ));
        }
        if event.onset_ms > event.end_ms || event.end_ms > config.monitored_duration_ms {
            return Err(EvaluationError::new(
                "invalid_event_interval",
                event.event_id.as_str(),
            ));
        }
        scenario_ids.insert(event.scenario_id.clone());
    }

    let mut unknown_ids = BTreeSet::new();
    for interval in unknown {
        insert_unique(&mut unknown_ids, &interval.interval_id, "interval_id")?;
        if !valid_id(&interval.scenario_id)
            || interval.start_ms > interval.end_ms
            || interval.end_ms > config.monitored_duration_ms
        {
            return Err(EvaluationError::new(
                "invalid_unknown_interval",
                interval.interval_id.as_str(),
            ));
        }
        scenario_ids.insert(interval.scenario_id.clone());
    }

    let mut alarm_ids = BTreeSet::new();
    for alarm in alarms {
        insert_unique(&mut alarm_ids, &alarm.alarm_id, "alarm_id")?;
        if !valid_id(&alarm.scenario_id) || !valid_id(&alarm.source_id) {
            return Err(EvaluationError::new(
                "empty_alarm_source_or_scenario",
                alarm.alarm_id.as_str(),
            ));
        }
        if alarm.start_ms > config.monitored_duration_ms
            || alarm
                .clear_ms
                .is_some_and(|end| end < alarm.start_ms || end > config.monitored_duration_ms)
        {
            return Err(EvaluationError::new(
                "invalid_alarm_interval",
                alarm.alarm_id.as_str(),
            ));
        }
        scenario_ids.insert(alarm.scenario_id.clone());
    }

    if scenario_ids.is_empty() {
        return Err(EvaluationError::new("empty_evaluation", "no scenarios"));
    }
    Ok(scenario_ids)
}

fn combine_class(left: AlarmClass, right: AlarmClass) -> AlarmClass {
    use AlarmClass::{Classified, Conflicting, Unclassified};
    match (left, right) {
        (Conflicting, _) | (_, Conflicting) => Conflicting,
        (Unclassified, _) | (_, Unclassified) => Unclassified,
        (Classified(a), Classified(b)) if a == b => Classified(a),
        (Classified(_), Classified(_)) => Conflicting,
    }
}

fn build_episodes(alarms: &[Alarm], config: EvaluationConfig) -> Vec<Episode> {
    let mut groups: BTreeMap<(String, String), Vec<&Alarm>> = BTreeMap::new();
    for alarm in alarms {
        groups
            .entry((alarm.scenario_id.clone(), alarm.source_id.clone()))
            .or_default()
            .push(alarm);
    }

    let mut episodes = Vec::new();
    for ((scenario_id, source_id), mut group) in groups {
        group.sort_by(|a, b| {
            a.start_ms
                .cmp(&b.start_ms)
                .then_with(|| a.alarm_id.cmp(&b.alarm_id))
        });

        let mut current: Option<Episode> = None;
        for alarm in group {
            let alarm_end = alarm.clear_ms.unwrap_or(config.monitored_duration_ms);
            let should_merge = current.as_ref().is_some_and(|episode| {
                alarm.start_ms <= episode.end_ms.saturating_add(config.merge_gap_ms)
            });

            if should_merge {
                if let Some(episode) = current.as_mut() {
                    episode.end_ms = episode.end_ms.max(alarm_end);
                    episode.class = combine_class(episode.class, alarm.class);
                    episode.constituent_alarm_ids.push(alarm.alarm_id.clone());
                }
                continue;
            }

            if let Some(mut finished) = current.take() {
                finished.constituent_alarm_ids.sort();
                finished.episode_id = format!("episode:{}", finished.constituent_alarm_ids[0]);
                episodes.push(finished);
            }
            current = Some(Episode {
                episode_id: String::new(),
                scenario_id: scenario_id.clone(),
                source_id: source_id.clone(),
                class: alarm.class,
                start_ms: alarm.start_ms,
                end_ms: alarm_end,
                constituent_alarm_ids: vec![alarm.alarm_id.clone()],
            });
        }

        if let Some(mut finished) = current {
            finished.constituent_alarm_ids.sort();
            finished.episode_id = format!("episode:{}", finished.constituent_alarm_ids[0]);
            episodes.push(finished);
        }
    }

    episodes.sort_by(|a, b| {
        a.scenario_id
            .cmp(&b.scenario_id)
            .then_with(|| a.start_ms.cmp(&b.start_ms))
            .then_with(|| a.source_id.cmp(&b.source_id))
            .then_with(|| a.episode_id.cmp(&b.episode_id))
    });
    episodes
}

fn overlaps_unknown(episode: &Episode, unknown: &[UnknownInterval]) -> bool {
    unknown.iter().any(|interval| {
        episode.scenario_id == interval.scenario_id
            && episode.start_ms <= interval.end_ms
            && interval.start_ms <= episode.end_ms
    })
}

fn onset_offset(episode_start: u64, event_onset: u64) -> i64 {
    if episode_start >= event_onset {
        (episode_start - event_onset).min(i64::MAX as u64) as i64
    } else {
        -((event_onset - episode_start).min(i64::MAX as u64) as i64)
    }
}

fn onset_distance(episode_start: u64, event_onset: u64) -> u64 {
    episode_start.abs_diff(event_onset)
}

fn candidate_order(event: &TruthEvent, episodes: &[Episode], indexes: &mut [usize]) {
    indexes.sort_by(|a, b| {
        onset_distance(episodes[*a].start_ms, event.onset_ms)
            .cmp(&onset_distance(episodes[*b].start_ms, event.onset_ms))
            .then_with(|| episodes[*a].episode_id.cmp(&episodes[*b].episode_id))
    });
}

fn augment(
    event_index: usize,
    candidates: &[Vec<usize>],
    episode_owner: &mut [Option<usize>],
    seen: &mut [bool],
) -> bool {
    for &episode_index in &candidates[event_index] {
        if seen[episode_index] {
            continue;
        }
        seen[episode_index] = true;
        let can_reassign = match episode_owner[episode_index] {
            None => true,
            Some(previous_owner) => augment(previous_owner, candidates, episode_owner, seen),
        };
        if can_reassign {
            episode_owner[episode_index] = Some(event_index);
            return true;
        }
    }
    false
}

fn maximum_cardinality_matching(
    event_indexes: &[usize],
    candidates: &[Vec<usize>],
    events: &[TruthEvent],
    episode_count: usize,
) -> Vec<Option<usize>> {
    let mut order = event_indexes.to_vec();
    order.sort_by(|a, b| {
        candidates[*a]
            .len()
            .cmp(&candidates[*b].len())
            .then_with(|| events[*a].event_id.cmp(&events[*b].event_id))
    });

    let mut episode_owner = vec![None; episode_count];
    for event_index in order {
        let mut seen = vec![false; episode_count];
        let _ = augment(event_index, candidates, &mut episode_owner, &mut seen);
    }

    let mut event_episode = vec![None; events.len()];
    for (episode_index, event_index) in episode_owner.into_iter().enumerate() {
        if let Some(event_index) = event_index {
            event_episode[event_index] = Some(episode_index);
        }
    }
    event_episode
}

fn percentile_nearest_rank(values: &[i64], percentile: usize) -> Option<i64> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let rank = (sorted.len() * percentile).div_ceil(100).max(1);
    sorted.get(rank - 1).copied()
}

fn union_duration(mut intervals: Vec<(u64, u64)>) -> u64 {
    if intervals.is_empty() {
        return 0;
    }
    intervals.sort_unstable();
    let mut total = 0_u128;
    let mut current_start = intervals[0].0;
    let mut current_end = intervals[0].1;
    for (start, end) in intervals.into_iter().skip(1) {
        if start <= current_end {
            current_end = current_end.max(end);
        } else {
            total += u128::from(current_end - current_start);
            current_start = start;
            current_end = end;
        }
    }
    total += u128::from(current_end - current_start);
    total.min(u128::from(u64::MAX)) as u64
}

/// Evaluate incident detection and alarm burden using frozen episode parameters.
///
/// Matching is class-agnostic for event detection; class correctness is scored
/// afterward. Unknown intervals are removed from matching and false-alarm
/// exposure. Maximum-cardinality matching is deterministic: process events by
/// candidate-count ascending then event ID; visit candidate episodes by
/// absolute onset difference then episode ID; use augmenting paths.
pub fn evaluate(
    events: &[TruthEvent],
    unknown: &[UnknownInterval],
    alarms: &[Alarm],
    config: EvaluationConfig,
) -> Result<EvaluationReport, EvaluationError> {
    let scenarios = validate_inputs(events, unknown, alarms, config)?;
    let episodes = build_episodes(alarms, config);
    let unscorable: BTreeSet<usize> = episodes
        .iter()
        .enumerate()
        .filter_map(|(index, episode)| overlaps_unknown(episode, unknown).then_some(index))
        .collect();

    let event_indexes: Vec<usize> = (0..events.len()).collect();
    let mut timely_candidates = vec![Vec::new(); events.len()];
    for (event_index, event) in events.iter().enumerate() {
        let lower = event.onset_ms.saturating_sub(config.allowed_lead_ms);
        let upper = event
            .onset_ms
            .saturating_add(config.maximum_detection_delay_ms);
        for (episode_index, episode) in episodes.iter().enumerate() {
            if !unscorable.contains(&episode_index)
                && episode.scenario_id == event.scenario_id
                && episode.start_ms >= lower
                && episode.start_ms <= upper
            {
                timely_candidates[event_index].push(episode_index);
            }
        }
        candidate_order(event, &episodes, &mut timely_candidates[event_index]);
    }

    let timely_assignment =
        maximum_cardinality_matching(&event_indexes, &timely_candidates, events, episodes.len());
    let mut used_episodes = BTreeSet::new();
    let mut matched_events = BTreeSet::new();
    let mut matches = Vec::new();
    let mut detection_delays = Vec::new();
    let mut early_leads = Vec::new();

    for (event_index, episode_index) in timely_assignment.iter().enumerate() {
        if let Some(episode_index) = episode_index {
            used_episodes.insert(*episode_index);
            matched_events.insert(event_index);
            let event = &events[event_index];
            let episode = &episodes[*episode_index];
            let correct = match episode.class {
                AlarmClass::Classified(class) => Some(class == event.class),
                AlarmClass::Unclassified | AlarmClass::Conflicting => None,
            };
            let offset = onset_offset(episode.start_ms, event.onset_ms);
            if offset >= 0 {
                detection_delays.push(offset);
            } else {
                early_leads.push(-offset);
            }
            matches.push(MatchRecord {
                event_id: event.event_id.clone(),
                episode_id: episode.episode_id.clone(),
                truth_class: event.class,
                reported_class: episode.class,
                kind: MatchKind::Timely,
                onset_offset_ms: offset,
                class_correct: correct,
            });
        }
    }

    let unmatched_event_indexes: Vec<usize> = event_indexes
        .iter()
        .copied()
        .filter(|index| !matched_events.contains(index))
        .collect();
    let mut late_candidates = vec![Vec::new(); events.len()];
    for event_index in &unmatched_event_indexes {
        let event = &events[*event_index];
        let lower = event.onset_ms.saturating_sub(config.allowed_lead_ms);
        let timely_upper = event
            .onset_ms
            .saturating_add(config.maximum_detection_delay_ms);
        let late_upper = event
            .end_ms
            .saturating_add(config.late_association_grace_ms);
        for (episode_index, episode) in episodes.iter().enumerate() {
            if !used_episodes.contains(&episode_index)
                && !unscorable.contains(&episode_index)
                && episode.scenario_id == event.scenario_id
                && episode.start_ms >= lower
                && episode.start_ms > timely_upper
                && episode.start_ms <= late_upper
            {
                late_candidates[*event_index].push(episode_index);
            }
        }
        candidate_order(event, &episodes, &mut late_candidates[*event_index]);
    }

    let late_assignment = maximum_cardinality_matching(
        &unmatched_event_indexes,
        &late_candidates,
        events,
        episodes.len(),
    );
    for event_index in unmatched_event_indexes {
        if let Some(episode_index) = late_assignment[event_index]
            && used_episodes.insert(episode_index)
        {
            let event = &events[event_index];
            let episode = &episodes[episode_index];
            let correct = match episode.class {
                AlarmClass::Classified(class) => Some(class == event.class),
                AlarmClass::Unclassified | AlarmClass::Conflicting => None,
            };
            matches.push(MatchRecord {
                event_id: event.event_id.clone(),
                episode_id: episode.episode_id.clone(),
                truth_class: event.class,
                reported_class: episode.class,
                kind: MatchKind::Late,
                onset_offset_ms: onset_offset(episode.start_ms, event.onset_ms),
                class_correct: correct,
            });
        }
    }

    let mut fragmentation = BTreeSet::new();
    for record in &matches {
        let Some(event) = events
            .iter()
            .find(|event| event.event_id == record.event_id)
        else {
            continue;
        };
        let lower = event.onset_ms.saturating_sub(config.allowed_lead_ms);
        let upper = event
            .end_ms
            .saturating_add(config.late_association_grace_ms);
        for (episode_index, episode) in episodes.iter().enumerate() {
            if !used_episodes.contains(&episode_index)
                && !unscorable.contains(&episode_index)
                && episode.scenario_id == event.scenario_id
                && episode.start_ms >= lower
                && episode.start_ms <= upper
            {
                fragmentation.insert(episode_index);
            }
        }
    }

    let mut false_alarm_indexes = Vec::new();
    let mut missed_event_ids = Vec::new();
    let mut late_event_count = 0;
    let mut late_associated_event_ids = Vec::new();
    let timely_event_count = timely_assignment
        .iter()
        .filter(|value| value.is_some())
        .count();
    let timely_event_ids: BTreeSet<String> = matches
        .iter()
        .filter(|record| record.kind == MatchKind::Timely)
        .map(|record| record.event_id.clone())
        .collect();
    let late_event_ids: BTreeSet<String> = matches
        .iter()
        .filter(|record| record.kind == MatchKind::Late)
        .map(|record| record.event_id.clone())
        .collect();
    for event in events {
        if late_event_ids.contains(event.event_id.as_str()) {
            late_event_count += 1;
            late_associated_event_ids.push(event.event_id.clone());
        } else if !timely_event_ids.contains(event.event_id.as_str()) {
            missed_event_ids.push(event.event_id.clone());
        }
    }

    for episode_index in 0..episodes.len() {
        if !used_episodes.contains(&episode_index)
            && !unscorable.contains(&episode_index)
            && !fragmentation.contains(&episode_index)
        {
            false_alarm_indexes.push(episode_index);
        }
    }

    let mut all_intervals_by_scenario: BTreeMap<String, Vec<(u64, u64)>> = BTreeMap::new();
    let mut false_intervals_by_scenario: BTreeMap<String, Vec<(u64, u64)>> = BTreeMap::new();
    for episode in &episodes {
        all_intervals_by_scenario
            .entry(episode.scenario_id.clone())
            .or_default()
            .push((episode.start_ms, episode.end_ms));
    }
    for index in &false_alarm_indexes {
        let episode = &episodes[*index];
        false_intervals_by_scenario
            .entry(episode.scenario_id.clone())
            .or_default()
            .push((episode.start_ms, episode.end_ms));
    }

    let all_active_ms: u128 = all_intervals_by_scenario
        .into_values()
        .map(|intervals| u128::from(union_duration(intervals)))
        .sum();
    let false_duration_ms: u128 = false_intervals_by_scenario
        .into_values()
        .map(|intervals| u128::from(union_duration(intervals)))
        .sum();
    let exposure_ms = u128::from(config.monitored_duration_ms) * scenarios.len() as u128;
    let duty_ppm = (all_active_ms * 1_000_000)
        .checked_div(exposure_ms)
        .unwrap_or(0)
        .min(u128::from(u64::MAX)) as u64;
    let monitored_system_days = exposure_ms as f64 / 86_400_000.0_f64;
    let false_alarm_count = false_alarm_indexes.len();
    let false_rate = if monitored_system_days == 0.0 {
        0.0
    } else {
        false_alarm_count as f64 / monitored_system_days
    };
    let scorable_episode_count = episodes.len().saturating_sub(unscorable.len());
    let event_precision = if scorable_episode_count == 0 {
        None
    } else {
        Some(timely_event_count as f64 / scorable_episode_count as f64)
    };
    let event_onset_recall = if events.is_empty() {
        None
    } else {
        Some(timely_event_count as f64 / events.len() as f64)
    };
    let mut confusion_counts: BTreeMap<(EventClass, AlarmClass), usize> = BTreeMap::new();
    for record in matches
        .iter()
        .filter(|record| record.kind == MatchKind::Timely)
    {
        *confusion_counts
            .entry((record.truth_class, record.reported_class))
            .or_default() += 1;
    }
    let class_confusion = confusion_counts
        .into_iter()
        .map(
            |((truth_class, reported_class), count)| ClassConfusionCell {
                truth_class,
                reported_class,
                count,
            },
        )
        .collect();

    matches.sort_by(|a, b| {
        a.event_id
            .cmp(&b.event_id)
            .then_with(|| match_kind_rank(a.kind).cmp(&match_kind_rank(b.kind)))
    });
    missed_event_ids.sort();
    late_associated_event_ids.sort();

    let mut fragmentation_episode_ids: Vec<String> = fragmentation
        .into_iter()
        .map(|index| episodes[index].episode_id.clone())
        .collect();
    fragmentation_episode_ids.sort();
    let mut unscorable_episode_ids: Vec<String> = unscorable
        .into_iter()
        .map(|index| episodes[index].episode_id.clone())
        .collect();
    unscorable_episode_ids.sort();
    let mut false_alarm_episode_ids: Vec<String> = false_alarm_indexes
        .into_iter()
        .map(|index| episodes[index].episode_id.clone())
        .collect();
    false_alarm_episode_ids.sort();

    Ok(EvaluationReport {
        confirmed_event_count: events.len(),
        timely_event_count,
        event_precision,
        event_onset_recall,
        class_confusion,
        late_associated_event_count: late_event_count,
        late_associated_event_ids,
        missed_event_ids,
        matches,
        fragmentation_episode_ids,
        unscorable_episode_ids,
        false_alarm_episode_ids,
        monitored_system_days,
        false_alarm_episodes_per_system_day: false_rate,
        false_alarm_duration_ms: false_duration_ms.min(u128::from(u64::MAX)) as u64,
        alarm_duty_cycle_ppm: duty_ppm,
        p50_detection_delay_ms: percentile_nearest_rank(&detection_delays, 50),
        p95_detection_delay_ms: percentile_nearest_rank(&detection_delays, 95),
        p50_early_lead_ms: percentile_nearest_rank(&early_leads, 50),
        p95_early_lead_ms: percentile_nearest_rank(&early_leads, 95),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(id: &str, class: EventClass, onset_ms: u64, end_ms: u64) -> TruthEvent {
        TruthEvent {
            event_id: id.into(),
            scenario_id: "scenario-1".into(),
            source_id: "truth-oracle".into(),
            class,
            onset_ms,
            end_ms,
        }
    }

    fn alarm(id: &str, source: &str, class: AlarmClass, start_ms: u64, clear_ms: u64) -> Alarm {
        Alarm {
            alarm_id: id.into(),
            scenario_id: "scenario-1".into(),
            source_id: source.into(),
            class,
            start_ms,
            clear_ms: Some(clear_ms),
        }
    }

    fn config() -> EvaluationConfig {
        EvaluationConfig {
            monitored_duration_ms: 86_400_000,
            merge_gap_ms: 60_000,
            allowed_lead_ms: 300_000,
            maximum_detection_delay_ms: 600_000,
            late_association_grace_ms: 300_000,
        }
    }

    #[test]
    fn undefined_event_rates_remain_none_instead_of_zero() {
        let unknown = vec![UnknownInterval {
            interval_id: "unknown-only".into(),
            scenario_id: "scenario-1".into(),
            start_ms: 0,
            end_ms: 1,
        }];
        let empty = evaluate(&[], &unknown, &[], config()).expect("scenario exists");
        assert_eq!(empty.event_precision, None);
        assert_eq!(empty.event_onset_recall, None);
        assert!(empty.canonical_text().contains("event_precision=none"));
        assert!(empty.canonical_text().contains("event_onset_recall=none"));

        let no_alarms = evaluate(
            &[event("event-no-alarm", EventClass::Leak, 1_000, 2_000)],
            &[],
            &[],
            config(),
        )
        .expect("event scenario exists");
        assert_eq!(no_alarms.event_precision, None);
        assert_eq!(no_alarms.event_onset_recall, Some(0.0));

        let no_events = evaluate(
            &[],
            &[],
            &[alarm(
                "false-alarm",
                "sensor-a",
                AlarmClass::Unclassified,
                1_000,
                2_000,
            )],
            config(),
        )
        .expect("alarm scenario exists");
        assert_eq!(no_events.event_precision, Some(0.0));
        assert_eq!(no_events.event_onset_recall, None);
    }

    #[test]
    fn merges_repeated_alarms_into_one_episode() {
        let events = vec![event("event-1", EventClass::Leak, 1_000_000, 1_300_000)];
        let alarms = vec![
            alarm(
                "alarm-a",
                "sensor-a",
                AlarmClass::Classified(EventClass::Leak),
                900_000,
                1_050_000,
            ),
            alarm(
                "alarm-b",
                "sensor-a",
                AlarmClass::Classified(EventClass::Leak),
                1_100_000,
                1_200_000,
            ),
        ];
        let report = evaluate(&events, &[], &alarms, config()).expect("valid fixture");
        assert_eq!(report.timely_event_count, 1);
        assert_eq!(report.matches[0].episode_id, "episode:alarm-a");
        assert_eq!(report.p50_early_lead_ms, Some(100_000));
        assert_eq!(report.p50_detection_delay_ms, None);
        assert!(report.fragmentation_episode_ids.is_empty());
    }

    #[test]
    fn detection_is_class_agnostic_and_diagnosis_is_scored_separately() {
        let events = vec![event("event-1", EventClass::Leak, 1_000, 2_000)];
        let alarms = vec![alarm(
            "alarm-1",
            "ot-monitor",
            AlarmClass::Classified(EventClass::CyberCompromise),
            1_100,
            1_200,
        )];
        let report = evaluate(&events, &[], &alarms, config()).expect("valid fixture");
        assert_eq!(report.timely_event_count, 1);
        assert_eq!(report.event_precision, Some(1.0));
        assert_eq!(report.event_onset_recall, Some(1.0));
        assert_eq!(report.matches[0].class_correct, Some(false));
        assert_eq!(
            report.class_confusion,
            vec![ClassConfusionCell {
                truth_class: EventClass::Leak,
                reported_class: AlarmClass::Classified(EventClass::CyberCompromise),
                count: 1,
            }]
        );
    }

    #[test]
    fn unclassified_alarm_detects_but_does_not_claim_correct_cause() {
        let events = vec![event("event-1", EventClass::SensorFault, 5_000, 6_000)];
        let alarms = vec![alarm(
            "alarm-1",
            "sensor-a",
            AlarmClass::Unclassified,
            5_200,
            5_400,
        )];
        let report = evaluate(&events, &[], &alarms, config()).expect("valid fixture");
        assert_eq!(report.timely_event_count, 1);
        assert_eq!(report.matches[0].class_correct, None);
        assert_eq!(report.matches[0].onset_offset_ms, 200);
        assert_eq!(report.p50_detection_delay_ms, Some(200));
        assert_eq!(report.p50_early_lead_ms, None);
    }

    #[test]
    fn late_association_is_not_a_timely_detection_or_false_alarm() {
        let events = vec![event(
            "event-1",
            EventClass::CyberCompromise,
            9_000_000,
            9_600_000,
        )];
        let alarms = vec![alarm(
            "alarm-late",
            "ot-monitor",
            AlarmClass::Classified(EventClass::CyberCompromise),
            9_700_000,
            10_000_000,
        )];
        let report = evaluate(&events, &[], &alarms, config()).expect("valid fixture");
        assert_eq!(report.timely_event_count, 0);
        assert_eq!(report.late_associated_event_count, 1);
        assert_eq!(report.late_associated_event_ids, vec!["event-1"]);
        assert!(report.false_alarm_episode_ids.is_empty());
        assert!(report.missed_event_ids.is_empty());
    }

    #[test]
    fn unknown_truth_interval_is_unscorable_not_false_positive() {
        let unknown = vec![UnknownInterval {
            interval_id: "unknown-1".into(),
            scenario_id: "scenario-1".into(),
            start_ms: 12_000,
            end_ms: 14_000,
        }];
        let alarms = vec![alarm(
            "alarm-unknown",
            "sensor-c",
            AlarmClass::Unclassified,
            12_500,
            12_700,
        )];
        let report = evaluate(&[], &unknown, &alarms, config()).expect("valid fixture");
        assert!(report.false_alarm_episode_ids.is_empty());
        assert_eq!(report.unscorable_episode_ids, vec!["episode:alarm-unknown"]);
    }

    #[test]
    fn one_event_cannot_claim_two_timely_alarm_episodes() {
        let events = vec![event("event-1", EventClass::Leak, 1_000, 2_000)];
        let alarms = vec![
            alarm(
                "alarm-a",
                "sensor-a",
                AlarmClass::Unclassified,
                1_100,
                1_200,
            ),
            alarm(
                "alarm-b",
                "sensor-b",
                AlarmClass::Unclassified,
                1_150,
                1_250,
            ),
        ];
        let report = evaluate(&events, &[], &alarms, config()).expect("valid fixture");
        assert_eq!(report.timely_event_count, 1);
        assert_eq!(report.fragmentation_episode_ids.len(), 1);
        assert!(report.false_alarm_episode_ids.is_empty());
    }

    #[test]
    fn one_alarm_episode_cannot_detect_two_nearby_events() {
        let events = vec![
            event("event-a", EventClass::Leak, 1_000, 1_500),
            event("event-b", EventClass::SensorFault, 1_200, 1_600),
        ];
        let alarms = vec![alarm(
            "alarm-only",
            "shared-source",
            AlarmClass::Unclassified,
            1_100,
            1_300,
        )];
        let report = evaluate(&events, &[], &alarms, config()).expect("valid fixture");
        assert_eq!(report.timely_event_count, 1);
        assert_eq!(report.missed_event_ids.len(), 1);
    }

    #[test]
    fn augmenting_paths_reach_maximum_cardinality() {
        let events = vec![
            event("event-a", EventClass::Leak, 1_000, 2_000),
            event("event-b", EventClass::Leak, 900, 1_900),
        ];
        let alarms = vec![
            alarm(
                "alarm-near-a",
                "source-a",
                AlarmClass::Unclassified,
                950,
                1_000,
            ),
            alarm(
                "alarm-only-a",
                "source-b",
                AlarmClass::Unclassified,
                1_050,
                1_100,
            ),
        ];
        let report = evaluate(&events, &[], &alarms, config()).expect("valid fixture");
        assert_eq!(report.timely_event_count, 2);
        assert_eq!(report.matches.len(), 2);
    }

    #[test]
    fn alarm_after_association_grace_is_false_and_event_missed() {
        let events = vec![event("event-1", EventClass::Leak, 1_000, 900_000)];
        let alarms = vec![alarm(
            "alarm-after-association",
            "source-a",
            AlarmClass::Unclassified,
            1_200_001,
            1_200_100,
        )];
        let report = evaluate(&events, &[], &alarms, config()).expect("valid fixture");
        assert_eq!(report.timely_event_count, 0);
        assert_eq!(report.missed_event_ids, vec!["event-1"]);
        assert_eq!(report.false_alarm_episode_ids.len(), 1);
    }

    #[test]
    fn open_alarm_runs_to_the_declared_horizon() {
        let alarms = vec![Alarm {
            alarm_id: "alarm-open".into(),
            scenario_id: "scenario-1".into(),
            source_id: "source-a".into(),
            class: AlarmClass::Unclassified,
            start_ms: 10,
            clear_ms: None,
        }];
        let report = evaluate(&[], &[], &alarms, config()).expect("valid fixture");
        assert_eq!(report.alarm_duty_cycle_ppm, 999_999);
    }

    #[test]
    fn duplicate_ids_and_invalid_intervals_fail_closed() {
        let duplicate = vec![
            alarm("same", "source-a", AlarmClass::Unclassified, 10, 20),
            alarm("same", "source-b", AlarmClass::Unclassified, 30, 40),
        ];
        assert_eq!(
            evaluate(&[], &[], &duplicate, config()).unwrap_err().code,
            "duplicate_identifier"
        );

        let invalid = vec![alarm("bad", "source-a", AlarmClass::Unclassified, 100, 99)];
        assert_eq!(
            evaluate(&[], &[], &invalid, config()).unwrap_err().code,
            "invalid_alarm_interval"
        );
    }

    #[test]
    fn input_permutations_produce_identical_canonical_receipts() {
        let events = vec![
            event("event-a", EventClass::Leak, 1_000, 1_500),
            event("event-b", EventClass::CommunicationFault, 5_000, 5_500),
        ];
        let alarms = vec![
            alarm(
                "alarm-b",
                "source-b",
                AlarmClass::Unclassified,
                5_100,
                5_300,
            ),
            alarm(
                "alarm-a",
                "source-a",
                AlarmClass::Classified(EventClass::Leak),
                1_100,
                1_300,
            ),
        ];
        let first = evaluate(&events, &[], &alarms, config()).expect("valid fixture");
        let mut reversed_events = events.clone();
        reversed_events.reverse();
        let mut reversed_alarms = alarms.clone();
        reversed_alarms.reverse();
        let second =
            evaluate(&reversed_events, &[], &reversed_alarms, config()).expect("permuted fixture");
        assert_eq!(first.canonical_text(), second.canonical_text());
    }

    #[test]
    fn different_alarm_sources_do_not_merge() {
        let events = vec![event("event-1", EventClass::Leak, 1_000, 2_000)];
        let alarms = vec![
            alarm(
                "alarm-a",
                "source-a",
                AlarmClass::Unclassified,
                1_000,
                1_100,
            ),
            alarm(
                "alarm-b",
                "source-b",
                AlarmClass::Unclassified,
                1_120,
                1_200,
            ),
        ];
        let report = evaluate(&events, &[], &alarms, config()).expect("valid fixture");
        assert_eq!(report.timely_event_count, 1);
        assert_eq!(report.fragmentation_episode_ids.len(), 1);
    }
}
