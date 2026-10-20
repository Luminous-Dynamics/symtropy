use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    env, fs, process,
};

const DEFAULT_FIXTURE: &str =
    "tools/ev-passenger-cost-frontier/fixtures/ev_factory_trace_v0_1.json";
const CLAIM_CEILING: &str = "Trace structure and declared provenance validated only. This report is not an event-log completeness guarantee, calibrated digital twin, audited production result, or factory/quality/safety/legal release.";

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct Trace {
    schema_version: String,
    benchmark_id: String,
    trace_id: String,
    collection_window_seconds: u64,
    collection_started_at_utc: String,
    collection_ended_at_utc: String,
    site_timezone: String,
    clock_basis: String,
    time_unit: String,
    shift_calendar_id: String,
    source_evidence_class: EvidenceClass,
    source_sha256: String,
    complete_cohort: bool,
    calibration_window: TimeWindow,
    holdout_window: TimeWindow,
    missingness: Vec<MissingnessDeclaration>,
    stations: Vec<StationSpec>,
    buffers: Vec<BufferSpec>,
    events: Vec<TraceEvent>,
    claim_ceiling: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct TimeWindow {
    start_seconds: u64,
    end_seconds: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct MissingnessDeclaration {
    field: String,
    status: MissingnessStatus,
    note: String,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum MissingnessStatus {
    NoneKnown,
    KnownPartial,
    Unobserved,
    Estimated,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct StationSpec {
    station_id: String,
    resource_ids: Vec<String>,
    max_rework_attempts: u32,
    note: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct BufferSpec {
    buffer_id: String,
    after_station_id: String,
    capacity_units: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct TraceEvent {
    event_id: String,
    timestamp_seconds: u64,
    kind: EventKind,
    job_id: Option<String>,
    station_id: Option<String>,
    buffer_id: Option<String>,
    resource_id: Option<String>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum EventKind {
    JobRelease,
    StationStart,
    StationEnd,
    QualityPass,
    QualityFail,
    BufferEnter,
    BufferExit,
    JobScrapped,
    BreakdownStart,
    RepairComplete,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum EvidenceClass {
    Synthetic,
    PublicMarketData,
    ManufacturerSpec,
    OfficialRecord,
    Measured,
    IndependentTest,
    AuditedCost,
    Unknown,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct TraceReport {
    schema_version: String,
    benchmark_id: String,
    trace_id: String,
    event_count: usize,
    source_evidence_class: EvidenceClass,
    source_digest_is_well_formed: bool,
    collection_window_seconds: u64,
    calibration_window: TimeWindow,
    holdout_window: TimeWindow,
    calibration_and_holdout_nonoverlap: bool,
    job_releases: u64,
    accepted_jobs: u64,
    scrapped_jobs: u64,
    incomplete_jobs_at_trace_end: u64,
    station_operations_completed: BTreeMap<String, u64>,
    station_rework_starts: BTreeMap<String, u64>,
    buffer_peak_occupancy: BTreeMap<String, usize>,
    buffer_final_occupancy: BTreeMap<String, usize>,
    equipment_breakdowns: u64,
    equipment_repairs: u64,
    structural_validation_passed: bool,
    calibration_review_candidate: bool,
    production_release_authorized: bool,
    missingness: Vec<MissingnessDeclaration>,
    claim_ceiling: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum TerminalState {
    Accepted,
    Scrapped,
}

#[derive(Debug, Clone, Default)]
struct JobState {
    station_index: usize,
    in_process: Option<(String, String)>,
    quality_pending: bool,
    attempts_at_station: u32,
    ready_for_rework: bool,
    awaiting_buffer_enter: bool,
    in_buffer: Option<String>,
    awaiting_scrap: bool,
    terminal: Option<TerminalState>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ActiveOperation {
    job_id: String,
    station_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DownResource {
    station_id: String,
}

fn main() {
    let path = env::args()
        .nth(1)
        .unwrap_or_else(|| DEFAULT_FIXTURE.to_string());
    let input = match fs::read_to_string(&path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("cannot read {path}: {error}");
            process::exit(2);
        }
    };
    let trace: Trace = match serde_json::from_str(&input) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("invalid factory trace: {error}");
            process::exit(2);
        }
    };
    let report = match validate_trace(&trace) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("factory trace rejected: {error}");
            process::exit(2);
        }
    };
    match serde_json::to_string_pretty(&report) {
        Ok(value) => println!("{value}"),
        Err(error) => {
            eprintln!("cannot serialize factory trace report: {error}");
            process::exit(2);
        }
    }
}

fn nonempty(name: &str, value: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("{name}_must_be_nonempty"));
    }
    Ok(())
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn validate_window(name: &str, window: &TimeWindow, limit: u64) -> Result<(), String> {
    if window.start_seconds >= window.end_seconds || window.end_seconds > limit {
        return Err(format!(
            "{name}_must_be_nonempty_and_inside_collection_window"
        ));
    }
    Ok(())
}

fn validate_event_shape(event: &TraceEvent) -> Result<(), String> {
    let shape = (
        event.job_id.is_some(),
        event.station_id.is_some(),
        event.buffer_id.is_some(),
        event.resource_id.is_some(),
    );
    let expected = match event.kind {
        EventKind::JobRelease | EventKind::JobScrapped => (true, false, false, false),
        EventKind::StationStart | EventKind::StationEnd => (true, true, false, true),
        EventKind::QualityPass | EventKind::QualityFail => (true, true, false, false),
        EventKind::BufferEnter | EventKind::BufferExit => (true, false, true, false),
        EventKind::BreakdownStart | EventKind::RepairComplete => (false, true, false, true),
    };
    if shape != expected {
        return Err(format!("event_{}_has_invalid_field_shape", event.event_id));
    }
    Ok(())
}

fn validate_trace(trace: &Trace) -> Result<TraceReport, String> {
    if trace.schema_version != "0.1.0" || trace.benchmark_id != "EV-FACTORY-TRACE-001" {
        return Err("unsupported_trace_schema_or_benchmark_id".into());
    }
    for (name, value) in [
        ("trace_id", trace.trace_id.as_str()),
        (
            "collection_started_at_utc",
            trace.collection_started_at_utc.as_str(),
        ),
        (
            "collection_ended_at_utc",
            trace.collection_ended_at_utc.as_str(),
        ),
        ("site_timezone", trace.site_timezone.as_str()),
        ("clock_basis", trace.clock_basis.as_str()),
        ("time_unit", trace.time_unit.as_str()),
        ("shift_calendar_id", trace.shift_calendar_id.as_str()),
        ("claim_ceiling", trace.claim_ceiling.as_str()),
    ] {
        nonempty(name, value)?;
    }
    if trace.collection_window_seconds == 0 {
        return Err("collection_window_seconds_must_be_positive".into());
    }
    if trace.clock_basis != "monotonic_seconds_from_collection_start"
        || trace.time_unit != "seconds"
    {
        return Err("unsupported_clock_basis_or_time_unit".into());
    }
    if !valid_sha256(&trace.source_sha256) {
        return Err("source_sha256_must_be_64_hex_characters".into());
    }
    validate_window(
        "calibration_window",
        &trace.calibration_window,
        trace.collection_window_seconds,
    )?;
    validate_window(
        "holdout_window",
        &trace.holdout_window,
        trace.collection_window_seconds,
    )?;
    let windows_nonoverlap = trace.calibration_window.end_seconds
        <= trace.holdout_window.start_seconds
        || trace.holdout_window.end_seconds <= trace.calibration_window.start_seconds;
    if !windows_nonoverlap {
        return Err("calibration_and_holdout_windows_overlap".into());
    }
    if trace.stations.is_empty() || trace.events.is_empty() {
        return Err("trace_requires_station_definitions_and_events".into());
    }
    let mut station_index = HashMap::<String, usize>::new();
    let mut resource_station = HashMap::<String, String>::new();
    for (index, station) in trace.stations.iter().enumerate() {
        nonempty("station_id", &station.station_id)?;
        nonempty("station_note", &station.note)?;
        if station.max_rework_attempts > 10 || station.resource_ids.is_empty() {
            return Err(format!(
                "station_{}_resource_or_rework_count_invalid",
                station.station_id
            ));
        }
        if station_index
            .insert(station.station_id.clone(), index)
            .is_some()
        {
            return Err("station_ids_must_be_unique".into());
        }
        let mut local_resources = HashSet::new();
        for resource_id in &station.resource_ids {
            nonempty("resource_id", resource_id)?;
            if !local_resources.insert(resource_id.clone())
                || resource_station
                    .insert(resource_id.clone(), station.station_id.clone())
                    .is_some()
            {
                return Err("resource_ids_must_be_unique_across_the_trace".into());
            }
        }
    }
    let mut buffer_index = HashMap::<String, usize>::new();
    for (index, buffer) in trace.buffers.iter().enumerate() {
        nonempty("buffer_id", &buffer.buffer_id)?;
        if buffer.capacity_units == 0 {
            return Err(format!(
                "buffer_{}_capacity_must_be_positive",
                buffer.buffer_id
            ));
        }
        if !station_index.contains_key(&buffer.after_station_id) {
            return Err(format!(
                "buffer_{}_references_unknown_station",
                buffer.buffer_id
            ));
        }
        if buffer_index
            .insert(buffer.buffer_id.clone(), index)
            .is_some()
        {
            return Err("buffer_ids_must_be_unique".into());
        }
    }
    if trace.buffers.len() != trace.stations.len().saturating_sub(1) {
        return Err("one_buffer_definition_required_between_each_adjacent_station".into());
    }
    for (index, buffer) in trace.buffers.iter().enumerate() {
        if station_index.get(&buffer.after_station_id) != Some(&index) {
            return Err("buffers_must_follow_stations_in_route_order".into());
        }
    }

    let mut missing_fields = HashSet::new();
    for declaration in &trace.missingness {
        nonempty("missingness_field", &declaration.field)?;
        nonempty("missingness_note", &declaration.note)?;
        if !missing_fields.insert(declaration.field.clone()) {
            return Err("missingness_fields_must_be_unique".into());
        }
    }
    if trace.missingness.is_empty() {
        return Err("missingness_declarations_required_even_when_none_known".into());
    }

    let mut job_states = HashMap::<String, JobState>::new();
    let mut event_ids = HashSet::<String>::new();
    let mut last_timestamp = 0_u64;
    let mut first_event = true;
    let mut resource_active = HashMap::<String, ActiveOperation>::new();
    let mut resource_down = HashMap::<String, DownResource>::new();
    let mut buffer_occupancy = vec![0_usize; trace.buffers.len()];
    let mut buffer_peak = vec![0_usize; trace.buffers.len()];
    let mut station_completed = vec![0_u64; trace.stations.len()];
    let mut station_rework_starts = vec![0_u64; trace.stations.len()];
    let mut job_releases = 0_u64;
    let mut accepted_jobs = 0_u64;
    let mut scrapped_jobs = 0_u64;
    let mut equipment_breakdowns = 0_u64;
    let mut equipment_repairs = 0_u64;

    for event in &trace.events {
        nonempty("event_id", &event.event_id)?;
        if !event_ids.insert(event.event_id.clone()) {
            return Err(format!("duplicate_event_id_{}", event.event_id));
        }
        validate_event_shape(event)?;
        if !first_event && event.timestamp_seconds < last_timestamp {
            return Err(format!("event_clock_regression_at_{}", event.event_id));
        }
        first_event = false;
        last_timestamp = event.timestamp_seconds;
        if event.timestamp_seconds >= trace.collection_window_seconds {
            return Err(format!(
                "event_{}_outside_collection_window",
                event.event_id
            ));
        }

        match event.kind {
            EventKind::JobRelease => {
                let job_id = event.job_id.as_ref().expect("shape validated");
                nonempty("job_id", job_id)?;
                if job_states
                    .insert(job_id.clone(), JobState::default())
                    .is_some()
                {
                    return Err(format!("job_{}_released_more_than_once", job_id));
                }
                job_releases += 1;
            }
            EventKind::StationStart => {
                let job_id = event.job_id.as_ref().expect("shape validated");
                let station_id = event.station_id.as_ref().expect("shape validated");
                let resource_id = event.resource_id.as_ref().expect("shape validated");
                let index = *station_index.get(station_id).ok_or_else(|| {
                    format!("event_{}_references_unknown_station", event.event_id)
                })?;
                if resource_station.get(resource_id) != Some(station_id) {
                    return Err(format!(
                        "event_{}_resource_station_mismatch",
                        event.event_id
                    ));
                }
                if resource_down.contains_key(resource_id) {
                    return Err(format!("event_{}_uses_resource_while_down", event.event_id));
                }
                if resource_active.contains_key(resource_id) {
                    return Err(format!("event_{}_resource_already_busy", event.event_id));
                }
                let job = job_states
                    .get_mut(job_id)
                    .ok_or_else(|| format!("event_{}_job_not_released", event.event_id))?;
                if job.terminal.is_some()
                    || job.in_process.is_some()
                    || job.quality_pending
                    || job.in_buffer.is_some()
                    || job.awaiting_buffer_enter
                    || job.awaiting_scrap
                {
                    return Err(format!(
                        "event_{}_job_not_ready_for_station_start",
                        event.event_id
                    ));
                }
                if job.station_index != index {
                    return Err(format!(
                        "event_{}_violates_station_route_order",
                        event.event_id
                    ));
                }
                if job.attempts_at_station > 0 && !job.ready_for_rework {
                    return Err(format!(
                        "event_{}_unexpected_repeat_station_start",
                        event.event_id
                    ));
                }
                if job.attempts_at_station == 0 && job.ready_for_rework {
                    return Err(format!("event_{}_invalid_rework_state", event.event_id));
                }
                if job.ready_for_rework {
                    station_rework_starts[index] += 1;
                }
                job.ready_for_rework = false;
                job.attempts_at_station += 1;
                job.in_process = Some((station_id.clone(), resource_id.clone()));
                resource_active.insert(
                    resource_id.clone(),
                    ActiveOperation {
                        job_id: job_id.clone(),
                        station_id: station_id.clone(),
                    },
                );
            }
            EventKind::StationEnd => {
                let job_id = event.job_id.as_ref().expect("shape validated");
                let station_id = event.station_id.as_ref().expect("shape validated");
                let resource_id = event.resource_id.as_ref().expect("shape validated");
                let job = job_states
                    .get_mut(job_id)
                    .ok_or_else(|| format!("event_{}_job_not_released", event.event_id))?;
                if job.in_process.as_ref() != Some(&(station_id.clone(), resource_id.clone())) {
                    return Err(format!(
                        "event_{}_does_not_match_station_start",
                        event.event_id
                    ));
                }
                let active = resource_active
                    .remove(resource_id)
                    .ok_or_else(|| format!("event_{}_resource_not_active", event.event_id))?;
                if active.job_id != *job_id || active.station_id != *station_id {
                    return Err(format!(
                        "event_{}_resource_operation_mismatch",
                        event.event_id
                    ));
                }
                let index = *station_index
                    .get(station_id)
                    .ok_or_else(|| format!("event_{}_references_unknown_station", event.event_id))?;
                job.in_process = None;
                job.quality_pending = true;
                station_completed[index] += 1;
            }
            EventKind::QualityPass => {
                let job_id = event.job_id.as_ref().expect("shape validated");
                let station_id = event.station_id.as_ref().expect("shape validated");
                let index = *station_index
                    .get(station_id)
                    .ok_or_else(|| format!("event_{}_references_unknown_station", event.event_id))?;
                let job = job_states
                    .get_mut(job_id)
                    .ok_or_else(|| format!("event_{}_job_not_released", event.event_id))?;
                if !job.quality_pending || job.station_index != index {
                    return Err(format!(
                        "event_{}_quality_without_matching_station_end",
                        event.event_id
                    ));
                }
                job.quality_pending = false;
                job.attempts_at_station = 0;
                if index + 1 == trace.stations.len() {
                    job.terminal = Some(TerminalState::Accepted);
                    accepted_jobs += 1;
                } else {
                    job.awaiting_buffer_enter = true;
                }
            }
            EventKind::QualityFail => {
                let job_id = event.job_id.as_ref().expect("shape validated");
                let station_id = event.station_id.as_ref().expect("shape validated");
                let index = *station_index
                    .get(station_id)
                    .ok_or_else(|| format!("event_{}_references_unknown_station", event.event_id))?;
                let job = job_states
                    .get_mut(job_id)
                    .ok_or_else(|| format!("event_{}_job_not_released", event.event_id))?;
                if !job.quality_pending || job.station_index != index {
                    return Err(format!("event_{}_quality_without_matching_station_end", event.event_id));
                }
                job.quality_pending = false;
                if job.attempts_at_station <= trace.stations[index].max_rework_attempts {
                    job.ready_for_rework = true;
                } else {
                    job.awaiting_scrap = true;
                }
            }
            EventKind::BufferEnter => {
                let job_id = event.job_id.as_ref().expect("shape validated");
                let buffer_id = event.buffer_id.as_ref().expect("shape validated");
                let bidx = *buffer_index
                    .get(buffer_id)
                    .ok_or_else(|| format!("event_{}_references_unknown_buffer", event.event_id))?;
                let bdef = &trace.buffers[bidx];
                let expected_station_index = station_index[&bdef.after_station_id];
                let job = job_states
                    .get_mut(job_id)
                    .ok_or_else(|| format!("event_{}_job_not_released", event.event_id))?;
                if !job.awaiting_buffer_enter
                    || job.station_index != expected_station_index
                    || job.in_buffer.is_some()
                {
                    return Err(format!("event_{}_unexpected_buffer_enter", event.event_id));
                }
                if buffer_occupancy[bidx] >= bdef.capacity_units {
                    return Err(format!("buffer_{}_capacity_exceeded", buffer_id));
                }
                buffer_occupancy[bidx] += 1;
                buffer_peak[bidx] = buffer_peak[bidx].max(buffer_occupancy[bidx]);
                job.awaiting_buffer_enter = false;
                job.in_buffer = Some(buffer_id.clone());
            }
            EventKind::BufferExit => {
                let job_id = event.job_id.as_ref().expect("shape validated");
                let buffer_id = event.buffer_id.as_ref().expect("shape validated");
                let bidx = *buffer_index
                    .get(buffer_id)
                    .ok_or_else(|| format!("event_{}_references_unknown_buffer", event.event_id))?;
                let job = job_states
                    .get_mut(job_id)
                    .ok_or_else(|| format!("event_{}_job_not_released", event.event_id))?;
                if job.in_buffer.as_deref() != Some(buffer_id.as_str())
                    || job.station_index != station_index[&trace.buffers[bidx].after_station_id]
                {
                    return Err(format!("event_{}_unexpected_buffer_exit", event.event_id));
                }
                if buffer_occupancy[bidx] == 0 {
                    return Err(format!("buffer_{}_occupancy_underflow", buffer_id));
                }
                buffer_occupancy[bidx] -= 1;
                job.in_buffer = None;
                job.station_index += 1;
                job.attempts_at_station = 0;
            }
            EventKind::JobScrapped => {
                let job_id = event.job_id.as_ref().expect("shape validated");
                let job = job_states
                    .get_mut(job_id)
                    .ok_or_else(|| format!("event_{}_job_not_released", event.event_id))?;
                if !job.awaiting_scrap
                    || job.in_process.is_some()
                    || job.quality_pending
                    || job.in_buffer.is_some()
                {
                    return Err(format!(
                        "event_{}_scrap_without_exhausted_quality_rework",
                        event.event_id
                    ));
                }
                job.awaiting_scrap = false;
                job.terminal = Some(TerminalState::Scrapped);
                scrapped_jobs += 1;
            }
            EventKind::BreakdownStart => {
                let station_id = event.station_id.as_ref().expect("shape validated");
                let resource_id = event.resource_id.as_ref().expect("shape validated");
                if resource_station.get(resource_id) != Some(station_id) {
                    return Err(format!("event_{}_resource_station_mismatch", event.event_id));
                }
                if resource_active.contains_key(resource_id) {
                    return Err(format!(
                        "event_{}_breakdown_during_active_operation_not_supported",
                        event.event_id
                    ));
                }
                if resource_down
                    .insert(
                        resource_id.clone(),
                        DownResource {
                            station_id: station_id.clone(),
                        },
                    )
                    .is_some()
                {
                    return Err(format!("event_{}_resource_already_down", event.event_id));
                }
                equipment_breakdowns += 1;
            }
            EventKind::RepairComplete => {
                let station_id = event.station_id.as_ref().expect("shape validated");
                let resource_id = event.resource_id.as_ref().expect("shape validated");
                let down = resource_down
                    .remove(resource_id)
                    .ok_or_else(|| format!("event_{}_repair_without_breakdown", event.event_id))?;
                if down.station_id != *station_id {
                    return Err(format!("event_{}_repair_station_mismatch", event.event_id));
                }
                equipment_repairs += 1;
            }
        }
    }

    if !resource_active.is_empty() {
        return Err("trace_ends_with_open_station_operations".into());
    }
    if !resource_down.is_empty() {
        return Err("trace_ends_with_unpaired_equipment_breakdowns".into());
    }
    let incomplete_jobs = job_states
        .values()
        .filter(|state| state.terminal.is_none())
        .count() as u64;
    if trace.complete_cohort && incomplete_jobs > 0 {
        return Err("complete_cohort_has_nonterminal_jobs".into());
    }
    let buffer_final: Vec<usize> = buffer_occupancy;
    if trace.complete_cohort && buffer_final.iter().any(|value| *value != 0) {
        return Err("complete_cohort_has_nonempty_buffers".into());
    }
    if accepted_jobs + scrapped_jobs > job_releases {
        return Err("terminal_job_count_exceeds_releases".into());
    }
    let critical_unobserved = trace.missingness.iter().any(|item| {
        item.status == MissingnessStatus::Unobserved
            && [
                "job_id",
                "timestamp_seconds",
                "station_id",
                "resource_id",
                "quality_outcome",
            ]
            .contains(&item.field.as_str())
    });
    let review_candidate = trace.source_evidence_class != EvidenceClass::Synthetic
        && trace.source_evidence_class != EvidenceClass::Unknown
        && !critical_unobserved
        && trace.calibration_window.end_seconds <= trace.holdout_window.start_seconds;

    Ok(TraceReport {
        schema_version: trace.schema_version.clone(),
        benchmark_id: trace.benchmark_id.clone(),
        trace_id: trace.trace_id.clone(),
        event_count: trace.events.len(),
        source_evidence_class: trace.source_evidence_class,
        source_digest_is_well_formed: true,
        collection_window_seconds: trace.collection_window_seconds,
        calibration_window: trace.calibration_window.clone(),
        holdout_window: trace.holdout_window.clone(),
        calibration_and_holdout_nonoverlap: true,
        job_releases,
        accepted_jobs,
        scrapped_jobs,
        incomplete_jobs_at_trace_end: incomplete_jobs,
        station_operations_completed: trace
            .stations
            .iter()
            .enumerate()
            .map(|(index, station)| (station.station_id.clone(), station_completed[index]))
            .collect(),
        station_rework_starts: trace
            .stations
            .iter()
            .enumerate()
            .map(|(index, station)| (station.station_id.clone(), station_rework_starts[index]))
            .collect(),
        buffer_peak_occupancy: trace
            .buffers
            .iter()
            .enumerate()
            .map(|(index, buffer)| (buffer.buffer_id.clone(), buffer_peak[index]))
            .collect(),
        buffer_final_occupancy: trace
            .buffers
            .iter()
            .enumerate()
            .map(|(index, buffer)| (buffer.buffer_id.clone(), buffer_final[index]))
            .collect(),
        equipment_breakdowns,
        equipment_repairs,
        structural_validation_passed: true,
        calibration_review_candidate: review_candidate,
        production_release_authorized: false,
        missingness: trace.missingness.clone(),
        claim_ceiling: CLAIM_CEILING.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn fixture() -> Trace {
        serde_json::from_str(include_str!("../../fixtures/ev_factory_trace_v0_1.json"))
            .expect("fixture parses")
    }

    fn fixture_value() -> Value {
        serde_json::from_str(include_str!("../../fixtures/ev_factory_trace_v0_1.json"))
            .expect("fixture JSON parses")
    }

    #[test]
    fn synthetic_fixture_validates_without_granting_release() {
        let report = validate_trace(&fixture()).expect("valid trace");
        assert!(report.structural_validation_passed);
        assert_eq!(report.event_count, 33);
        assert_eq!(report.job_releases, 2);
        assert_eq!(report.accepted_jobs, 2);
        assert_eq!(report.scrapped_jobs, 0);
        assert_eq!(report.incomplete_jobs_at_trace_end, 0);
        assert_eq!(report.equipment_breakdowns, 1);
        assert_eq!(report.equipment_repairs, 1);
        assert!(!report.calibration_review_candidate);
        assert!(!report.production_release_authorized);
    }

    #[test]
    fn duplicate_event_ids_are_rejected() {
        let mut trace = fixture();
        trace.events[1].event_id = trace.events[0].event_id.clone();
        assert!(
            validate_trace(&trace)
                .unwrap_err()
                .contains("duplicate_event_id")
        );
    }

    #[test]
    fn clock_regression_is_rejected() {
        let mut trace = fixture();
        trace.events[0].timestamp_seconds = 4;
        assert!(
            validate_trace(&trace)
                .unwrap_err()
                .contains("event_clock_regression")
        );
    }

    #[test]
    fn calibration_holdout_overlap_is_rejected() {
        let mut trace = fixture();
        trace.holdout_window.start_seconds = 25;
        assert!(
            validate_trace(&trace)
                .unwrap_err()
                .contains("windows_overlap")
        );
    }

    #[test]
    fn route_transition_to_wrong_station_is_rejected() {
        let mut trace = fixture();
        let first_start = trace
            .events
            .iter_mut()
            .find(|event| event.kind == EventKind::StationStart)
            .expect("station start exists");
        first_start.station_id = Some("battery-module".into());
        first_start.resource_id = Some("battery-1".into());
        assert!(validate_trace(&trace).unwrap_err().contains("route_order"));
    }

    #[test]
    fn buffer_over_capacity_is_rejected() {
        let mut trace = fixture();
        trace.buffers[0].capacity_units = 1;
        assert!(
            validate_trace(&trace)
                .unwrap_err()
                .contains("capacity_exceeded")
        );
    }

    #[test]
    fn repair_without_breakdown_is_rejected() {
        let mut trace = fixture();
        trace
            .events
            .retain(|event| event.kind != EventKind::BreakdownStart);
        assert!(
            validate_trace(&trace)
                .unwrap_err()
                .contains("repair_without_breakdown")
        );
    }

    #[test]
    fn unknown_json_fields_fail_closed() {
        let mut value = fixture_value();
        value["unreviewed_override"] = Value::Bool(true);
        assert!(serde_json::from_value::<Trace>(value).is_err());
    }

    #[test]
    fn malformed_digest_is_rejected() {
        let mut trace = fixture();
        trace.source_sha256 = "not-a-digest".into();
        assert!(
            validate_trace(&trace)
                .unwrap_err()
                .contains("source_sha256")
        );
    }

    #[test]
    fn deterministic_report_serializes_identically() {
        let trace = fixture();
        let a = serde_json::to_vec(&validate_trace(&trace).unwrap()).unwrap();
        let b = serde_json::to_vec(&validate_trace(&trace).unwrap()).unwrap();
        assert_eq!(a, b);
    }
}
