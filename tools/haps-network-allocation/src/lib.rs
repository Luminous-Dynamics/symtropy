//! Deterministic, radio-agnostic HAPS demand/beam allocation.
//!
//! This crate allocates a declared finite number of abstract beams against
//! cell-level critical-excess demand. It is not a propagation, SINR,
//! interference, antenna, spectrum, backhaul, flight-dynamics, or safety model.
//! A caller must supply evidence-qualified demand and resource boundaries.

use std::collections::{HashMap, HashSet};
use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::Value;

const TIME_TOLERANCE_S: f64 = 1.0e-6;
const INTEGER_TOLERANCE: f64 = 1.0e-9;
const SECONDS_PER_DAY: f64 = 86_400.0;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AllocationError {
    Json(String),
    MissingField(String),
    InvalidTrace(String),
    InvalidWindow(String),
    InvalidBudget(String),
    InvalidPolicy(String),
}

impl fmt::Display for AllocationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(message)
            | Self::MissingField(message)
            | Self::InvalidTrace(message)
            | Self::InvalidWindow(message)
            | Self::InvalidBudget(message)
            | Self::InvalidPolicy(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for AllocationError {}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrafficTrace {
    pub schema_version: String,
    pub trace_id: String,
    pub revision: u64,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub region_ref: String,
    pub source: TrafficSource,
    pub time_basis: String,
    pub horizon_s: f64,
    pub interval_duration_s: f64,
    pub cells: Vec<Cell>,
    pub intervals: Vec<TrafficInterval>,
    pub assumptions: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrafficSource {
    pub source_kind: String,
    pub source_uri: Option<String>,
    pub source_digest: Option<String>,
    pub rights_or_license_ref: Option<String>,
    pub initialization_time_utc: Option<String>,
    pub available_at_decision_time: bool,
    pub reanalysis_not_used_as_live_forecast: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cell {
    pub cell_id: String,
    pub latitude_deg: f64,
    pub longitude_deg: f64,
    pub priority_weight: f64,
    pub service_class: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrafficInterval {
    pub offset_s: f64,
    pub cell_loads: Vec<CellLoad>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CellLoad {
    pub cell_id: String,
    pub offered_load_mbps: f64,
    pub terrestrial_available_mbps: f64,
}

#[derive(Debug, Clone)]
pub enum AllocationPolicy {
    CriticalExcessWeightedGreedy,
    StaticFixedAssignment(Vec<String>),
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct IntervalAllocation {
    /// Zero-based day index inside the requested evaluation window.
    pub day_index: usize,
    /// Zero-based day index inside the original traffic trace.
    pub source_day_index: usize,
    /// Offset relative to this evaluation window.
    pub offset_s: f64,
    /// Original offset in the source traffic trace.
    pub source_offset_s: f64,
    pub selected_cell_ids: Vec<String>,
    pub critical_excess_offered_mbps: f64,
    pub critical_excess_served_mbps: f64,
    pub critical_excess_unserved_mbps: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct AllocationTotals {
    /// Sum of per-interval Mbps values (Mbps-intervals), not bit volume.
    pub critical_excess_offered_mbps_intervals: f64,
    pub critical_excess_served_mbps_intervals: f64,
    pub critical_excess_unserved_mbps_intervals: f64,
    pub critical_excess_offload_fraction: f64,
    pub beam_utilization_fraction: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct AllocationReport {
    pub trace_id: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub policy: String,
    pub start_day_index: usize,
    pub evaluated_days: usize,
    pub beam_budget: usize,
    pub per_beam_capacity_mbps: f64,
    pub static_assignment_cell_ids: Vec<String>,
    pub intervals: Vec<IntervalAllocation>,
    pub totals: AllocationTotals,
}

#[derive(Debug, Clone)]
struct RankedCell {
    cell_id: String,
    critical_excess_mbps: f64,
    weighted_critical_excess: f64,
}

/// Parse the versioned trace and check required JSON keys before deserializing.
pub fn parse_trace_json(bytes: &[u8]) -> Result<TrafficTrace, AllocationError> {
    let value: Value = serde_json::from_slice(bytes)
        .map_err(|error| AllocationError::Json(error.to_string()))?;

    require_keys(
        &value,
        "traffic trace",
        &[
            "schema_version",
            "trace_id",
            "revision",
            "evidence_class",
            "claim_ceiling",
            "region_ref",
            "source",
            "time_basis",
            "horizon_s",
            "interval_duration_s",
            "cells",
            "intervals",
            "assumptions",
        ],
    )?;

    let source = value
        .get("source")
        .ok_or_else(|| AllocationError::MissingField("source".to_owned()))?;
    require_keys(
        source,
        "source",
        &[
            "source_kind",
            "source_uri",
            "source_digest",
            "rights_or_license_ref",
            "initialization_time_utc",
            "available_at_decision_time",
            "reanalysis_not_used_as_live_forecast",
        ],
    )?;

    let cells = value
        .get("cells")
        .and_then(Value::as_array)
        .ok_or_else(|| AllocationError::InvalidTrace("cells must be an array".to_owned()))?;
    for (index, cell) in cells.iter().enumerate() {
        require_keys(
            cell,
            &format!("cells[{index}]"),
            &[
                "cell_id",
                "latitude_deg",
                "longitude_deg",
                "priority_weight",
                "service_class",
            ],
        )?;
    }

    let intervals = value
        .get("intervals")
        .and_then(Value::as_array)
        .ok_or_else(|| AllocationError::InvalidTrace("intervals must be an array".to_owned()))?;
    for (index, interval) in intervals.iter().enumerate() {
        require_keys(
            interval,
            &format!("intervals[{index}]"),
            &["offset_s", "cell_loads"],
        )?;
        let loads = interval
            .get("cell_loads")
            .and_then(Value::as_array)
            .ok_or_else(|| {
                AllocationError::InvalidTrace(format!(
                    "intervals[{index}].cell_loads must be an array"
                ))
            })?;
        for (load_index, load) in loads.iter().enumerate() {
            require_keys(
                load,
                &format!("intervals[{index}].cell_loads[{load_index}]"),
                &[
                    "cell_id",
                    "offered_load_mbps",
                    "terrestrial_available_mbps",
                ],
            )?;
        }
    }

    let trace: TrafficTrace = serde_json::from_value(value)
        .map_err(|error| AllocationError::Json(error.to_string()))?;
    validate_trace(&trace)?;
    Ok(trace)
}

fn require_keys(
    value: &Value,
    context: &str,
    keys: &[&str],
) -> Result<(), AllocationError> {
    let object = value.as_object().ok_or_else(|| {
        AllocationError::InvalidTrace(format!("{context} must be a JSON object"))
    })?;
    for key in keys {
        if !object.contains_key(*key) {
            return Err(AllocationError::MissingField(format!(
                "{context}.{key} is required (null is allowed where declared)"
            )));
        }
    }
    Ok(())
}

pub fn validate_trace(trace: &TrafficTrace) -> Result<(), AllocationError> {
    if trace.schema_version != "haps-traffic-demand-trace/v1" {
        return Err(AllocationError::InvalidTrace(
            "unsupported traffic trace schema_version".to_owned(),
        ));
    }
    if trace.trace_id.trim().is_empty() || trace.region_ref.trim().is_empty() {
        return Err(AllocationError::InvalidTrace(
            "trace_id and region_ref must be non-empty".to_owned(),
        ));
    }
    if !matches!(
        trace.evidence_class.as_str(),
        "synthetic" | "operator_measurement" | "replayed_telemetry" | "traffic_forecast"
    ) {
        return Err(AllocationError::InvalidTrace(
            "unsupported evidence_class".to_owned(),
        ));
    }
    if !matches!(
        trace.claim_ceiling.as_str(),
        "synthetic_demand_input" | "offline_traffic_replay"
    ) {
        return Err(AllocationError::InvalidTrace(
            "unsupported traffic claim_ceiling".to_owned(),
        ));
    }
    if trace.evidence_class == "synthetic"
        && trace.claim_ceiling != "synthetic_demand_input"
    {
        return Err(AllocationError::InvalidTrace(
            "synthetic traces must use synthetic_demand_input claim ceiling".to_owned(),
        ));
    }
    if !matches!(
        trace.time_basis.as_str(),
        "valid_time" | "observation_time" | "issue_time"
    ) {
        return Err(AllocationError::InvalidTrace(
            "unsupported time_basis".to_owned(),
        ));
    }
    if !trace.source.reanalysis_not_used_as_live_forecast {
        return Err(AllocationError::InvalidTrace(
            "reanalysis/live-forecast leakage guard must be true".to_owned(),
        ));
    }
    if trace.source.source_kind == "reanalysis" && trace.source.available_at_decision_time {
        return Err(AllocationError::InvalidTrace(
            "reanalysis cannot be marked available as an operational live forecast".to_owned(),
        ));
    }
    if trace.source.source_kind == "synthetic_generator"
        && trace.evidence_class != "synthetic"
    {
        return Err(AllocationError::InvalidTrace(
            "synthetic_generator source must use synthetic evidence_class".to_owned(),
        ));
    }
    if !trace.horizon_s.is_finite()
        || !trace.interval_duration_s.is_finite()
        || trace.horizon_s <= 0.0
        || trace.interval_duration_s <= 0.0
    {
        return Err(AllocationError::InvalidTrace(
            "horizon_s and interval_duration_s must be finite and positive".to_owned(),
        ));
    }
    if trace.cells.is_empty() || trace.intervals.is_empty() {
        return Err(AllocationError::InvalidTrace(
            "cells and intervals must be non-empty".to_owned(),
        ));
    }

    if (trace.interval_duration_s - SECONDS_PER_DAY).abs() > TIME_TOLERANCE_S {
        return Err(AllocationError::InvalidTrace(
            "this initial allocator expects one 24-hour demand sample per interval".to_owned(),
        ));
    }
    let ratio = trace.horizon_s / trace.interval_duration_s;
    let expected_intervals = ratio.round();
    if !ratio.is_finite()
        || (ratio - expected_intervals).abs() > INTEGER_TOLERANCE
        || expected_intervals < 1.0
        || expected_intervals > usize::MAX as f64
        || expected_intervals as usize != trace.intervals.len()
    {
        return Err(AllocationError::InvalidTrace(
            "horizon_s must equal interval_count × interval_duration_s".to_owned(),
        ));
    }

    let mut cell_weights = HashMap::with_capacity(trace.cells.len());
    for cell in &trace.cells {
        if cell.cell_id.trim().is_empty()
            || !cell.latitude_deg.is_finite()
            || !cell.longitude_deg.is_finite()
            || !(-90.0..=90.0).contains(&cell.latitude_deg)
            || !(-180.0..=180.0).contains(&cell.longitude_deg)
            || !cell.priority_weight.is_finite()
            || cell.priority_weight <= 0.0
        {
            return Err(AllocationError::InvalidTrace(format!(
                "cell {} has invalid identity, coordinates or priority",
                cell.cell_id
            )));
        }
        if cell_weights
            .insert(cell.cell_id.as_str(), cell.priority_weight)
            .is_some()
        {
            return Err(AllocationError::InvalidTrace(format!(
                "duplicate cell_id {}",
                cell.cell_id
            )));
        }
    }

    for (index, interval) in trace.intervals.iter().enumerate() {
        let expected_offset = index as f64 * trace.interval_duration_s;
        if !interval.offset_s.is_finite()
            || (interval.offset_s - expected_offset).abs() > TIME_TOLERANCE_S
        {
            return Err(AllocationError::InvalidTrace(format!(
                "interval {index} offset is not contiguous and aligned to the trace cadence"
            )));
        }
        if interval.cell_loads.len() != trace.cells.len() {
            return Err(AllocationError::InvalidTrace(format!(
                "interval {index} must contain exactly one load row per declared cell"
            )));
        }
        let mut seen = HashSet::with_capacity(interval.cell_loads.len());
        for load in &interval.cell_loads {
            if !cell_weights.contains_key(load.cell_id.as_str()) {
                return Err(AllocationError::InvalidTrace(format!(
                    "interval {index} references undeclared cell {}",
                    load.cell_id
                )));
            }
            if !seen.insert(load.cell_id.as_str()) {
                return Err(AllocationError::InvalidTrace(format!(
                    "interval {index} contains duplicate cell {}",
                    load.cell_id
                )));
            }
            if !load.offered_load_mbps.is_finite()
                || !load.terrestrial_available_mbps.is_finite()
                || load.offered_load_mbps < 0.0
                || load.terrestrial_available_mbps < 0.0
            {
                return Err(AllocationError::InvalidTrace(format!(
                    "interval {index} has negative or non-finite cell load"
                )));
            }
        }
    }

    Ok(())
}

/// Select a fixed cell assignment using only the declared training window.
/// Scores are summed as priority_weight × critical_excess.
pub fn select_static_assignment(
    trace: &TrafficTrace,
    training_days: usize,
    beam_budget: usize,
) -> Result<Vec<String>, AllocationError> {
    validate_trace(trace)?;
    if training_days == 0 || training_days > trace.intervals.len() {
        return Err(AllocationError::InvalidWindow(
            "training_days must be within the available trace".to_owned(),
        ));
    }
    if beam_budget == 0 || beam_budget > trace.cells.len() {
        return Err(AllocationError::InvalidBudget(
            "beam_budget must be positive and no greater than the number of cells".to_owned(),
        ));
    }

    let weights: HashMap<&str, f64> = trace
        .cells
        .iter()
        .map(|cell| (cell.cell_id.as_str(), cell.priority_weight))
        .collect();
    let mut scores: HashMap<&str, f64> = trace
        .cells
        .iter()
        .map(|cell| (cell.cell_id.as_str(), 0.0))
        .collect();

    for interval in trace.intervals.iter().take(training_days) {
        for load in &interval.cell_loads {
            let excess =
                (load.offered_load_mbps - load.terrestrial_available_mbps).max(0.0);
            let score = excess * weights[load.cell_id.as_str()];
            let current = scores.get_mut(load.cell_id.as_str()).ok_or_else(|| {
                AllocationError::InvalidTrace(format!(
                    "training interval references undeclared cell {}",
                    load.cell_id
                ))
            })?;
            *current += score;
        }
    }

    let mut ranked: Vec<(String, f64)> = scores
        .into_iter()
        .map(|(cell_id, score)| (cell_id.to_owned(), score))
        .collect();
    ranked.sort_by(|left, right| {
        right
            .1
            .total_cmp(&left.1)
            .then_with(|| left.0.cmp(&right.0))
    });
    Ok(ranked
        .into_iter()
        .take(beam_budget)
        .map(|(cell_id, _)| cell_id)
        .collect())
}

/// Evaluate a window of daily samples from the source trace.
pub fn evaluate_window(
    trace: &TrafficTrace,
    start_day: usize,
    days: usize,
    beam_budget: usize,
    per_beam_capacity_mbps: f64,
    policy: AllocationPolicy,
) -> Result<AllocationReport, AllocationError> {
    validate_trace(trace)?;
    if days == 0
        || start_day >= trace.intervals.len()
        || start_day.saturating_add(days) > trace.intervals.len()
    {
        return Err(AllocationError::InvalidWindow(
            "requested start_day/days window is outside the source trace".to_owned(),
        ));
    }
    if beam_budget == 0
        || beam_budget > trace.cells.len()
        || !per_beam_capacity_mbps.is_finite()
        || per_beam_capacity_mbps <= 0.0
    {
        return Err(AllocationError::InvalidBudget(
            "beam budget and per-beam capacity must be finite and positive".to_owned(),
        ));
    }

    let weights: HashMap<&str, f64> = trace
        .cells
        .iter()
        .map(|cell| (cell.cell_id.as_str(), cell.priority_weight))
        .collect();

    let (policy_name, static_assignment_cell_ids) = match &policy {
        AllocationPolicy::CriticalExcessWeightedGreedy => {
            ("critical_excess_weighted_greedy".to_owned(), Vec::new())
        }
        AllocationPolicy::StaticFixedAssignment(cell_ids) => {
            if cell_ids.is_empty() || cell_ids.len() > beam_budget {
                return Err(AllocationError::InvalidPolicy(
                    "static assignment must contain between one and beam_budget cells"
                        .to_owned(),
                ));
            }
            let mut seen = HashSet::with_capacity(cell_ids.len());
            for cell_id in cell_ids {
                if !weights.contains_key(cell_id.as_str()) || !seen.insert(cell_id.as_str()) {
                    return Err(AllocationError::InvalidPolicy(format!(
                        "static assignment has unknown or duplicate cell_id {cell_id}"
                    )));
                }
            }
            ("static_fixed_assignment".to_owned(), cell_ids.clone())
        }
    };

    let mut report_intervals = Vec::with_capacity(days);
    let mut total_offered = 0.0;
    let mut total_served = 0.0;
    let mut total_unserved = 0.0;

    for relative_day in 0..days {
        let source_day = start_day + relative_day;
        let interval = &trace.intervals[source_day];
        let mut ranked: Vec<RankedCell> = interval
            .cell_loads
            .iter()
            .map(|load| {
                let excess = (load.offered_load_mbps - load.terrestrial_available_mbps).max(0.0);
                RankedCell {
                    cell_id: load.cell_id.clone(),
                    critical_excess_mbps: excess,
                    weighted_critical_excess: excess * weights[load.cell_id.as_str()],
                }
            })
            .collect();

        let selected_cell_ids = match &policy {
            AllocationPolicy::CriticalExcessWeightedGreedy => {
                ranked.retain(|cell| cell.critical_excess_mbps > 0.0);
                ranked.sort_by(|left, right| {
                    right
                        .weighted_critical_excess
                        .total_cmp(&left.weighted_critical_excess)
                        .then_with(|| left.cell_id.cmp(&right.cell_id))
                });
                ranked
                    .iter()
                    .take(beam_budget)
                    .map(|cell| cell.cell_id.clone())
                    .collect::<Vec<_>>()
            }
            AllocationPolicy::StaticFixedAssignment(cell_ids) => cell_ids.clone(),
        };

        let offered = ranked
            .iter()
            .map(|cell| cell.critical_excess_mbps)
            .sum::<f64>();
        let mut served = 0.0;
        for cell_id in &selected_cell_ids {
            let row = interval
                .cell_loads
                .iter()
                .find(|load| load.cell_id == *cell_id)
                .ok_or_else(|| {
                    AllocationError::InvalidTrace(format!(
                        "interval {source_day} is missing assigned cell {cell_id}"
                    ))
                })?;
            let excess = (row.offered_load_mbps - row.terrestrial_available_mbps).max(0.0);
            served += excess.min(per_beam_capacity_mbps);
        }
        let unserved = (offered - served).max(0.0);

        total_offered += offered;
        total_served += served;
        total_unserved += unserved;
        report_intervals.push(IntervalAllocation {
            day_index: relative_day,
            source_day_index: source_day,
            offset_s: relative_day as f64 * trace.interval_duration_s,
            source_offset_s: interval.offset_s,
            selected_cell_ids,
            critical_excess_offered_mbps: offered,
            critical_excess_served_mbps: served,
            critical_excess_unserved_mbps: unserved,
        });
    }

    let denominator = beam_budget as f64 * per_beam_capacity_mbps * days as f64;
    let offload_fraction = if total_offered > 0.0 {
        total_served / total_offered
    } else {
        0.0
    };

    Ok(AllocationReport {
        trace_id: trace.trace_id.clone(),
        evidence_class: trace.evidence_class.clone(),
        claim_ceiling: if trace.evidence_class == "synthetic" {
            "synthetic_allocation_oracle".to_owned()
        } else {
            "offline_traffic_replay".to_owned()
        },
        policy: policy_name,
        start_day_index: start_day,
        evaluated_days: days,
        beam_budget,
        per_beam_capacity_mbps,
        static_assignment_cell_ids,
        intervals: report_intervals,
        totals: AllocationTotals {
            critical_excess_offered_mbps_intervals: total_offered,
            critical_excess_served_mbps_intervals: total_served,
            critical_excess_unserved_mbps_intervals: total_unserved,
            critical_excess_offload_fraction: offload_fraction,
            beam_utilization_fraction: if denominator > 0.0 {
                total_served / denominator
            } else {
                0.0
            },
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const TRAFFIC_TRACE: &str =
        include_str!("../../../docs/research/aviation/examples/synthetic_haps_disaster_traffic_trace_v1.json");
    const FULL_ALLOCATION_CASE: &str = include_str!(
        "../../../docs/research/aviation/examples/synthetic_haps_disaster_30day_allocation_v1.json"
    );
    const DYNAMIC_HOLDOUT_CASE: &str = include_str!(
        "../../../docs/research/aviation/examples/synthetic_haps_15day_holdout_dynamic_allocation_v1.json"
    );
    const STATIC_HOLDOUT_CASE: &str = include_str!(
        "../../../docs/research/aviation/examples/synthetic_haps_15day_holdout_static_allocation_v1.json"
    );

    fn fixture_trace() -> TrafficTrace {
        parse_trace_json(TRAFFIC_TRACE.as_bytes()).expect("traffic fixture parses and validates")
    }

    fn assert_matches_case(report: &AllocationReport, case_json: &str) {
        let case: Value = serde_json::from_str(case_json).expect("allocation case JSON parses");
        let expected = &case["expected_oracle"];
        let close = |actual: f64, path: &str| {
            let expected_value = expected[path]
                .as_f64()
                .unwrap_or_else(|| panic!("expected_oracle.{path} must be numeric"));
            assert!(
                (actual - expected_value).abs() < 1.0e-9,
                "expected_oracle.{path}: actual {actual}, expected {expected_value}"
            );
        };

        close(
            report.totals.critical_excess_offered_mbps_intervals,
            "total_critical_excess_offered_mbps_intervals",
        );
        close(
            report.totals.critical_excess_served_mbps_intervals,
            "total_critical_excess_served_mbps_intervals",
        );
        close(
            report.totals.critical_excess_unserved_mbps_intervals,
            "total_critical_excess_unserved_mbps_intervals",
        );
        close(
            report.totals.critical_excess_offload_fraction,
            "critical_excess_offload_fraction",
        );
        close(
            report.totals.beam_utilization_fraction,
            "beam_utilization_fraction",
        );

        let expected_intervals = expected["intervals"]
            .as_array()
            .expect("expected_oracle.intervals must be an array");
        assert_eq!(report.intervals.len(), expected_intervals.len());
        for (actual, expected_interval) in report.intervals.iter().zip(expected_intervals) {
            let expected_offset = expected_interval["offset_s"]
                .as_f64()
                .expect("expected offset_s must be numeric");
            assert!((actual.offset_s - expected_offset).abs() < 1.0e-9);
            let expected_source_offset = expected_interval
                .get("source_offset_s")
                .and_then(Value::as_f64)
                .unwrap_or(expected_offset);
            assert!((actual.source_offset_s - expected_source_offset).abs() < 1.0e-9);

            let mut actual_ids = actual.selected_cell_ids.clone();
            actual_ids.sort();
            let mut expected_ids: Vec<String> = expected_interval["selected_cell_ids"]
                .as_array()
                .expect("selected_cell_ids must be an array")
                .iter()
                .map(|value| value.as_str().expect("cell ID must be a string").to_owned())
                .collect();
            expected_ids.sort();
            assert_eq!(actual_ids, expected_ids);

            for (actual_value, key) in [
                (
                    actual.critical_excess_offered_mbps,
                    "critical_excess_offered_mbps",
                ),
                (
                    actual.critical_excess_served_mbps,
                    "critical_excess_served_mbps",
                ),
                (
                    actual.critical_excess_unserved_mbps,
                    "critical_excess_unserved_mbps",
                ),
            ] {
                let expected_value = expected_interval[key]
                    .as_f64()
                    .expect("interval oracle value must be numeric");
                assert!((actual_value - expected_value).abs() < 1.0e-9);
            }
        }
    }

    #[test]
    fn full_thirty_day_greedy_allocation_matches_independent_fixture() {
        let trace = fixture_trace();
        let report = evaluate_window(
            &trace,
            0,
            30,
            2,
            10.0,
            AllocationPolicy::CriticalExcessWeightedGreedy,
        )
        .unwrap();

        assert_matches_case(&report, FULL_ALLOCATION_CASE);
    }

    #[test]
    fn dynamic_holdout_matches_15_day_oracle() {
        let trace = fixture_trace();
        let report = evaluate_window(
            &trace,
            15,
            15,
            2,
            10.0,
            AllocationPolicy::CriticalExcessWeightedGreedy,
        )
        .unwrap();

        assert_matches_case(&report, DYNAMIC_HOLDOUT_CASE);
    }

    #[test]
    fn static_assignment_is_selected_only_from_training_days() {
        let trace = fixture_trace();
        let assignment = select_static_assignment(&trace, 15, 2).unwrap();
        assert_eq!(assignment, vec!["cell-east", "cell-north"]);
        let report = evaluate_window(
            &trace,
            15,
            15,
            2,
            10.0,
            AllocationPolicy::StaticFixedAssignment(assignment),
        )
        .unwrap();

        assert_matches_case(&report, STATIC_HOLDOUT_CASE);
    }

    #[test]
    fn greedy_dynamic_improves_synthetic_holdout_over_training_selected_fixed_cells() {
        let trace = fixture_trace();
        let fixed = select_static_assignment(&trace, 15, 2).unwrap();
        let dynamic_report = evaluate_window(
            &trace,
            15,
            15,
            2,
            10.0,
            AllocationPolicy::CriticalExcessWeightedGreedy,
        )
        .unwrap();
        let static_report = evaluate_window(
            &trace,
            15,
            15,
            2,
            10.0,
            AllocationPolicy::StaticFixedAssignment(fixed),
        )
        .unwrap();

        let relative_gain = (dynamic_report.totals.critical_excess_served_mbps_intervals
            - static_report.totals.critical_excess_served_mbps_intervals)
            / static_report.totals.critical_excess_served_mbps_intervals;
        assert!((relative_gain - 40.0 / 234.0).abs() < 1e-12);
        assert!(relative_gain > 0.17);
    }

    #[test]
    fn deterministic_tie_break_uses_cell_id_ascending() {
        let mut trace = fixture_trace();
        trace.cells = vec![
            Cell {
                cell_id: "cell-z".to_owned(),
                latitude_deg: 0.0,
                longitude_deg: 0.0,
                priority_weight: 1.0,
                service_class: "other".to_owned(),
            },
            Cell {
                cell_id: "cell-a".to_owned(),
                latitude_deg: 0.0,
                longitude_deg: 0.0,
                priority_weight: 1.0,
                service_class: "other".to_owned(),
            },
        ];
        trace.intervals = vec![TrafficInterval {
            offset_s: 0.0,
            cell_loads: vec![
                CellLoad {
                    cell_id: "cell-z".to_owned(),
                    offered_load_mbps: 10.0,
                    terrestrial_available_mbps: 5.0,
                },
                CellLoad {
                    cell_id: "cell-a".to_owned(),
                    offered_load_mbps: 10.0,
                    terrestrial_available_mbps: 5.0,
                },
            ],
        }];
        trace.horizon_s = trace.interval_duration_s;

        let report = evaluate_window(
            &trace,
            0,
            1,
            1,
            10.0,
            AllocationPolicy::CriticalExcessWeightedGreedy,
        )
        .unwrap();
        assert_eq!(report.intervals[0].selected_cell_ids, vec!["cell-a"]);
    }

    #[test]
    fn rejects_duplicate_cell_load_rows() {
        let mut trace = fixture_trace();
        let duplicate_id = trace.intervals[0].cell_loads[0].cell_id.clone();
        trace.intervals[0].cell_loads[1].cell_id = duplicate_id;
        assert!(validate_trace(&trace).is_err());
    }

    #[test]
    fn rejects_negative_traffic_loads() {
        let mut trace = fixture_trace();
        trace.intervals[0].cell_loads[0].offered_load_mbps = -1.0;
        assert!(validate_trace(&trace).is_err());
    }

    #[test]
    fn rejects_noncontiguous_time_offsets() {
        let mut trace = fixture_trace();
        trace.intervals[1].offset_s += 1.0;
        assert!(validate_trace(&trace).is_err());
    }

    #[test]
    fn rejects_reanalysis_marked_as_available_live_forecast() {
        let mut trace = fixture_trace();
        trace.source.source_kind = "reanalysis".to_owned();
        trace.source.available_at_decision_time = true;
        assert!(validate_trace(&trace).is_err());
    }

    #[test]
    fn rejects_static_assignments_larger_than_the_beam_budget() {
        let trace = fixture_trace();
        let result = evaluate_window(
            &trace,
            15,
            15,
            1,
            10.0,
            AllocationPolicy::StaticFixedAssignment(vec![
                "cell-east".to_owned(),
                "cell-north".to_owned(),
            ]),
        );
        assert!(matches!(result, Err(AllocationError::InvalidPolicy(_))));
    }

    #[test]
    fn rejects_windows_that_extend_beyond_the_trace() {
        let trace = fixture_trace();
        let result = evaluate_window(
            &trace,
            29,
            2,
            2,
            10.0,
            AllocationPolicy::CriticalExcessWeightedGreedy,
        );
        assert!(matches!(result, Err(AllocationError::InvalidWindow(_))));
    }
}
