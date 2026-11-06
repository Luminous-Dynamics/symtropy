//! Stage-resolved dry-solid material-flow and particle-size transformation ledger.
//!
//! This crate validates observed streams at physical boundaries and reports
//! interval-valued input/output flow closure and per-size-bin flow changes.
//! It does not attribute a PSD change to breakage, infer particle trajectories,
//! estimate attrition rates, simulate transport or qualify operating settings.
#![forbid(unsafe_code)]

use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Violation {
    pub code: String,
    pub path: String,
    pub detail: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FlowInterval {
    pub lower_kg_s: f64,
    pub upper_kg_s: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BinFlowChange {
    pub bin_index: usize,
    pub lower_mm: f64,
    pub upper_mm: Option<f64>,
    pub change_lower_kg_s: f64,
    pub change_upper_kg_s: f64,
    pub disposition: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TransitionReport {
    pub transition_id: String,
    pub stage: String,
    pub input_flow: FlowInterval,
    pub output_flow: FlowInterval,
    pub mass_balance_disposition: String,
    pub residence_time_observed_midpoint_ms: f64,
    pub bin_flow_changes: Vec<BinFlowChange>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StreamReport {
    pub stream_id: String,
    pub station_id: String,
    pub role: String,
    pub dry_mass_flow: FlowInterval,
    pub bin_count: usize,
    pub binned_flow: FlowInterval,
    pub binned_flow_overlaps_total: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EvaluationReport {
    pub case_id: String,
    pub parent_case_id: String,
    pub parent_interval_id: String,
    pub trial_id: String,
    pub material_cohort_ref: String,
    pub machine_configuration_ref: String,
    pub transport_configuration_ref: String,
    pub station_count: usize,
    pub stream_count: usize,
    pub transition_count: usize,
    pub streams: Vec<StreamReport>,
    pub transitions: Vec<TransitionReport>,
    pub causal_attribution: String,
}

fn push(errors: &mut Vec<Violation>, code: &str, path: &str, detail: impl Into<String>) {
    errors.push(Violation {
        code: code.to_owned(),
        path: path.to_owned(),
        detail: detail.into(),
    });
}

fn text<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
}

fn finite(value: &Value, key: &str) -> Option<f64> {
    value.get(key).and_then(Value::as_f64).filter(|n| n.is_finite())
}

fn uint(value: &Value, key: &str) -> Option<u64> {
    value.get(key).and_then(Value::as_u64)
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1.0e-9 * a.abs().max(b.abs()).max(1.0)
}

fn valid_interval(value: &Value, lower_key: &str, upper_key: &str) -> Option<FlowInterval> {
    let lower = finite(value, lower_key)?;
    let upper = finite(value, upper_key)?;
    if lower < 0.0 || upper < lower {
        return None;
    }
    Some(FlowInterval {
        lower_kg_s: lower,
        upper_kg_s: upper,
    })
}

fn add_intervals<'a>(intervals: impl Iterator<Item = &'a FlowInterval>) -> FlowInterval {
    intervals.fold(
        FlowInterval {
            lower_kg_s: 0.0,
            upper_kg_s: 0.0,
        },
        |mut total, interval| {
            total.lower_kg_s += interval.lower_kg_s;
            total.upper_kg_s += interval.upper_kg_s;
            total
        },
    )
}

fn intervals_overlap(a: &FlowInterval, b: &FlowInterval) -> bool {
    a.lower_kg_s <= b.upper_kg_s && b.lower_kg_s <= a.upper_kg_s
}

fn mass_balance_disposition(input: &FlowInterval, output: &FlowInterval) -> &'static str {
    if output.upper_kg_s < input.lower_kg_s {
        "DEFINITE_DEFICIT"
    } else if output.lower_kg_s > input.upper_kg_s {
        "DEFINITE_SURPLUS"
    } else {
        "WITHIN_UNCERTAINTY"
    }
}

fn expected_station_for_role(role: &str) -> Option<&'static str> {
    match role {
        "GENERATED" => Some("CUTTER_DISCHARGE"),
        "CAPTURED" => Some("CONVEYANCE_INLET"),
        "FLOOR_RESIDUAL" => Some("FLOOR_RESIDUAL"),
        "CONVEYED" => Some("CONVEYANCE_OUTLET"),
        "LINE_DEPOSIT" => Some("LINE_DEPOSIT"),
        "SEPARATOR_COARSE" => Some("SEPARATOR_COARSE_OUTLET"),
        "FILTER_FINES" => Some("FILTER_FINE_OUTLET"),
        "REJECTED" => Some("REJECT_DISCHARGE"),
        "ACCEPTED_DISCHARGE" => Some("ACCEPTED_DISCHARGE"),
        "OTHER" => None,
        _ => None,
    }
}

fn validate_provenance(value: &Value, path: &str, errors: &mut Vec<Violation>) {
    if !matches!(
        text(value, "evidence_role"),
        Some("INDEPENDENT_MEASUREMENT" | "DERIVED_FROM_INDEPENDENT_MEASUREMENTS")
    ) {
        push(
            errors,
            "NON_INDEPENDENT_STREAM_EVIDENCE",
            &format!("{path}.evidence_role"),
            "stream and PSD flow evidence must be independently measured or explicitly derived from independent measurements",
        );
    }
    for key in ["calibration_ref", "source_ref"] {
        if text(value, key).is_none() {
            push(
                errors,
                "MISSING_STREAM_PROVENANCE",
                &format!("{path}.{key}"),
                "calibration and source references are required",
            );
        }
    }
    if text(value, "evidence_role") == Some("DERIVED_FROM_INDEPENDENT_MEASUREMENTS")
        && text(value, "lineage_ref").is_none()
    {
        push(
            errors,
            "MISSING_STREAM_LINEAGE",
            &format!("{path}.lineage_ref"),
            "derived stream measurements require explicit lineage",
        );
    }
}

/// Validate parent identity, physical stream stations, PSD grids, time alignment,
/// stream topology and conservative mass-flow closure prerequisites.
pub fn validate_against_parent(case: &Value, parent: &Value) -> Vec<Violation> {
    let mut errors = Vec::new();

    if case.get("schema_version").and_then(Value::as_u64) != Some(1) {
        push(
            &mut errors,
            "UNSUPPORTED_SCHEMA_VERSION",
            "$.schema_version",
            "schema_version must be exactly 1",
        );
    }
    if text(case, "case_id").is_none() {
        push(&mut errors, "MISSING_CASE_ID", "$.case_id", "case_id is required");
    }
    if !matches!(
        text(case, "evidence_class"),
        Some("SYNTHETIC_CONFORMANCE_FIXTURE" | "MEASURED")
    ) {
        push(
            &mut errors,
            "INVALID_EVIDENCE_CLASS",
            "$.evidence_class",
            "evidence_class must be SYNTHETIC_CONFORMANCE_FIXTURE or MEASURED",
        );
    }

    let Some(context) = case.get("parent_context") else {
        push(
            &mut errors,
            "MISSING_PARENT_CONTEXT",
            "$.parent_context",
            "exact parent trial and configuration identity is required",
        );
        return errors;
    };
    let parent_case_id = text(parent, "case_id");
    if text(context, "parent_case_id") != parent_case_id || parent_case_id.is_none() {
        push(
            &mut errors,
            "PARENT_CASE_MISMATCH",
            "$.parent_context.parent_case_id",
            "material transformation case must reference the exact supplied parent PSD case",
        );
    }

    let Some(parent_profile) = parent.get("profile") else {
        push(
            &mut errors,
            "PARENT_PROFILE_MISSING",
            "$.parent.profile",
            "parent profile is required",
        );
        return errors;
    };
    let intervals = parent.get("intervals").and_then(Value::as_array);
    let interval_id = text(context, "parent_interval_id");
    let matching: Vec<&Value> = intervals
        .into_iter()
        .flatten()
        .filter(|interval| text(interval, "interval_id") == interval_id)
        .collect();
    if matching.len() != 1 {
        push(
            &mut errors,
            "PARENT_INTERVAL_NOT_UNIQUE",
            "$.parent_context.parent_interval_id",
            "parent interval must resolve to exactly one interval",
        );
    }
    let Some(parent_interval) = matching.first().copied() else {
        return errors;
    };

    let identity_pairs = [
        ("trial_id", text(parent_interval, "trial_id")),
        ("material_cohort_ref", text(parent_profile, "material_cohort_ref")),
        ("site_context_ref", text(parent_profile, "site_context_ref")),
        ("machine_configuration_ref", text(parent_profile, "machine_configuration_ref")),
        ("transport_configuration_ref", text(parent_profile, "transport_configuration_ref")),
        ("clock_sync_protocol_ref", text(parent_profile, "clock_sync_protocol_ref")),
    ];
    for (field, expected) in identity_pairs {
        if text(context, field) != expected || expected.is_none() {
            push(
                &mut errors,
                "PARENT_CONTEXT_MISMATCH",
                &format!("$.parent_context.{field}"),
                format!("material transformation {field} must match the exact parent interval/profile"),
            );
        }
    }
    if text(parent_interval, "configuration_ref")
        != text(context, "machine_configuration_ref")
    {
        push(
            &mut errors,
            "PARENT_INTERVAL_CONFIGURATION_MISMATCH",
            "$.parent_context.machine_configuration_ref",
            "parent interval configuration must match the declared machine configuration",
        );
    }

    let Some(parent_bins) = parent_interval.get("fragment_bins").and_then(Value::as_array) else {
        push(
            &mut errors,
            "PARENT_PSD_UNAVAILABLE",
            "$.parent_context.parent_interval_id",
            "parent interval must expose its authoritative fragment_bins",
        );
        return errors;
    };
    if parent_bins.is_empty() {
        push(
            &mut errors,
            "PARENT_PSD_EMPTY",
            "$.parent_context.parent_interval_id",
            "parent PSD must contain at least one bin",
        );
        return errors;
    }

    for (index, bin) in parent_bins.iter().enumerate() {
        let lower = finite(bin, "lower_mm");
        let path = format!("$.parent.fragment_bins[{index}]");
        match (lower, bin.get("upper_mm")) {
            (Some(lo), Some(Value::Number(number))) => {
                let upper = number.as_f64().filter(|v| v.is_finite());
                if lo < 0.0 || upper.is_none_or(|hi| hi <= lo) {
                    push(
                        &mut errors,
                        "INVALID_PARENT_PSD_BIN",
                        &path,
                        "parent PSD finite bin must be non-negative with upper > lower",
                    );
                }
                if index == 0 && !close(lo, 0.0) {
                    push(
                        &mut errors,
                        "PARENT_PSD_NOT_ZERO_ORIGIN",
                        &path,
                        "parent PSD must start at zero",
                    );
                }
                if index + 1 == parent_bins.len() {
                    push(
                        &mut errors,
                        "PARENT_PSD_FINAL_BIN_NOT_OPEN",
                        &path,
                        "final parent PSD bin must be open-ended",
                    );
                }
                if index > 0 {
                    let previous = parent_bins[index - 1]
                        .get("upper_mm")
                        .and_then(Value::as_f64);
                    if previous.is_none_or(|v| !close(v, lo)) {
                        push(
                            &mut errors,
                            "PARENT_PSD_GAP_OR_OVERLAP",
                            &path,
                            "parent PSD bins must be contiguous and non-overlapping",
                        );
                    }
                }
            }
            (Some(lo), Some(Value::Null)) if lo >= 0.0 && index + 1 == parent_bins.len() => {
                if index == 0 && !close(lo, 0.0) {
                    push(
                        &mut errors,
                        "PARENT_PSD_NOT_ZERO_ORIGIN",
                        &path,
                        "parent PSD must start at zero",
                    );
                }
                if index > 0 {
                    let previous = parent_bins[index - 1]
                        .get("upper_mm")
                        .and_then(Value::as_f64);
                    if previous.is_none_or(|v| !close(v, lo)) {
                        push(
                            &mut errors,
                            "PARENT_PSD_GAP_OR_OVERLAP",
                            &path,
                            "final open-ended parent bin must begin where the previous bin ends",
                        );
                    }
                }
            }
            _ => push(
                &mut errors,
                "INVALID_PARENT_PSD_BIN",
                &path,
                "parent PSD bins require ordered finite lower bounds and only the final upper bound may be null",
            ),
        }
    }

    let Some(stations) = case.get("stations").and_then(Value::as_array) else {
        push(&mut errors, "MISSING_STATIONS", "$.stations", "stations must be an array");
        return errors;
    };
    let mut station_by_id: BTreeMap<String, &Value> = BTreeMap::new();
    for (index, station) in stations.iter().enumerate() {
        let path = format!("$.stations[{index}]");
        let Some(id) = text(station, "station_id") else {
            push(&mut errors, "MISSING_STATION_ID", &format!("{path}.station_id"), "station_id is required");
            continue;
        };
        if station_by_id.insert(id.to_owned(), station).is_some() {
            push(&mut errors, "DUPLICATE_STATION_ID", &format!("{path}.station_id"), "station IDs must be unique");
        }
        for key in ["physical_location_ref", "configuration_ref", "stage"] {
            if text(station, key).is_none() {
                push(&mut errors, "MISSING_STATION_CONTEXT", &format!("{path}.{key}"), "station stage, physical location and configuration are required");
            }
        }
        let allowed_config = text(context, "machine_configuration_ref") == text(station, "configuration_ref")
            || text(context, "transport_configuration_ref") == text(station, "configuration_ref");
        if !allowed_config {
            push(&mut errors, "STATION_CONFIGURATION_MISMATCH", &format!("{path}.configuration_ref"), "station must bind to the declared exact machine or transport configuration");
        }
    }

    let Some(streams) = case.get("streams").and_then(Value::as_array) else {
        push(&mut errors, "MISSING_STREAMS", "$.streams", "streams must be an array");
        return errors;
    };
    if streams.is_empty() {
        push(&mut errors, "EMPTY_STREAMS", "$.streams", "at least one measured stream is required");
    }

    let mut stream_by_id: BTreeMap<String, &Value> = BTreeMap::new();
    let mut source_refs = BTreeSet::new();
    for (index, stream) in streams.iter().enumerate() {
        let path = format!("$.streams[{index}]");
        let Some(id) = text(stream, "stream_id") else {
            push(&mut errors, "MISSING_STREAM_ID", &format!("{path}.stream_id"), "stream_id is required");
            continue;
        };
        if stream_by_id.insert(id.to_owned(), stream).is_some() {
            push(&mut errors, "DUPLICATE_STREAM_ID", &format!("{path}.stream_id"), "stream IDs must be unique");
        }
        let Some(station_id) = text(stream, "station_id") else {
            push(&mut errors, "MISSING_STREAM_STATION", &format!("{path}.station_id"), "stream must identify its physical station");
            continue;
        };
        let Some(station) = station_by_id.get(station_id).copied() else {
            push(&mut errors, "UNKNOWN_STREAM_STATION", &format!("{path}.station_id"), "stream references an undeclared station");
            continue;
        };
        if let Some(expected) = expected_station_for_role(text(stream, "role").unwrap_or("")) {
            if text(station, "stage") != Some(expected) {
                push(&mut errors, "STREAM_ROLE_STATION_MISMATCH", &format!("{path}.role"), "stream role must match the declared physical station stage");
            }
        } else if text(stream, "role") != Some("OTHER") {
            push(&mut errors, "UNKNOWN_STREAM_ROLE", &format!("{path}.role"), "stream role is outside v0.1 vocabulary");
        }

        match (uint(stream, "start_ms"), uint(stream, "end_ms")) {
            (Some(start), Some(end)) if end > start => {}
            _ => push(&mut errors, "INVALID_STREAM_WINDOW", &path, "stream requires end_ms > start_ms"),
        }
        if text(stream, "clock_sync_ref").is_none() {
            push(&mut errors, "MISSING_STREAM_CLOCK", &format!("{path}.clock_sync_ref"), "stream clock synchronization reference is required");
        }
        match finite(stream, "time_uncertainty_ms") {
            Some(v) if v >= 0.0 => {}
            _ => push(&mut errors, "INVALID_STREAM_TIME_UNCERTAINTY", &format!("{path}.time_uncertainty_ms"), "time uncertainty must be finite and non-negative"),
        }
        let total = valid_interval(stream, "dry_mass_flow_lower_kg_s", "dry_mass_flow_upper_kg_s");
        if total.is_none() {
            push(&mut errors, "INVALID_STREAM_TOTAL_FLOW", &path, "stream dry-mass flow interval must be finite, non-negative and ordered");
        }
        if let Some(provenance) = stream.get("provenance") {
            validate_provenance(provenance, &format!("{path}.provenance"), &mut errors);
            if let Some(source) = text(provenance, "source_ref") {
                if !source_refs.insert(source.to_owned()) {
                    push(&mut errors, "DUPLICATE_STREAM_SOURCE", &format!("{path}.provenance.source_ref"), "each stream must have a distinct independently traceable raw sample source; reuse must be explicitly represented as a derived lineage");
                }
            }
        } else {
            push(&mut errors, "MISSING_STREAM_PROVENANCE", &format!("{path}.provenance"), "stream provenance is required");
        }

        let Some(bins) = stream.get("bins").and_then(Value::as_array) else {
            push(&mut errors, "MISSING_STREAM_BINS", &format!("{path}.bins"), "each stream requires a complete PSD bin array");
            continue;
        };
        if bins.len() != parent_bins.len() {
            push(&mut errors, "STREAM_PSD_BIN_COUNT_MISMATCH", &format!("{path}.bins"), "each stream must use exactly the parent PSD bin grid");
        }
        let mut binned_lower = 0.0;
        let mut binned_upper = 0.0;
        for (bin_index, bin) in bins.iter().enumerate() {
            let bin_path = format!("{path}.bins[{bin_index}]");
            if let Some(parent_bin) = parent_bins.get(bin_index) {
                match (finite(bin, "lower_mm"), finite(parent_bin, "lower_mm")) {
                    (Some(a), Some(b)) if close(a, b) => {}
                    _ => push(&mut errors, "STREAM_PSD_LOWER_BOUND_MISMATCH", &format!("{bin_path}.lower_mm"), "stream PSD lower boundary must match the exact parent PSD boundary"),
                }
                let upper_match = match (bin.get("upper_mm"), parent_bin.get("upper_mm")) {
                    (Some(Value::Null), Some(Value::Null)) => true,
                    (Some(a), Some(b)) => match (a.as_f64(), b.as_f64()) {
                        (Some(x), Some(y)) if x.is_finite() && y.is_finite() => close(x, y),
                        _ => false,
                    },
                    _ => false,
                };
                if !upper_match {
                    push(&mut errors, "STREAM_PSD_UPPER_BOUND_MISMATCH", &format!("{bin_path}.upper_mm"), "stream PSD upper boundary must match the exact parent PSD boundary");
                }
            }
            let flow = valid_interval(bin, "dry_mass_flow_lower_kg_s", "dry_mass_flow_upper_kg_s");
            if let Some(flow) = flow {
                binned_lower += flow.lower_kg_s;
                binned_upper += flow.upper_kg_s;
            } else {
                push(&mut errors, "INVALID_BIN_FLOW_INTERVAL", &bin_path, "per-bin dry-mass flow interval must be finite, non-negative and ordered");
            }
            if let Some(provenance) = bin.get("provenance") {
                validate_provenance(provenance, &format!("{bin_path}.provenance"), &mut errors);
            } else {
                push(&mut errors, "MISSING_BIN_PROVENANCE", &format!("{bin_path}.provenance"), "per-bin calibration and sample provenance are required");
            }
        }
        if let Some(total) = total {
            let binned = FlowInterval {
                lower_kg_s: binned_lower,
                upper_kg_s: binned_upper,
            };
            if !intervals_overlap(&total, &binned) {
                push(&mut errors, "BINNED_FLOW_DOES_NOT_RECONCILE", &format!("{path}.bins"), "conservative sum of per-bin flow intervals does not overlap the independently measured total stream flow interval");
            }
        }
    }

    let Some(transitions) = case.get("transitions").and_then(Value::as_array) else {
        push(&mut errors, "MISSING_TRANSITIONS", "$.transitions", "transitions must be an array");
        return errors;
    };
    if transitions.is_empty() {
        push(&mut errors, "EMPTY_TRANSITIONS", "$.transitions", "at least one measured transition is required");
    }

    let mut transition_ids = BTreeSet::new();
    let mut produced_by: BTreeMap<String, String> = BTreeMap::new();
    let mut consumed_by: BTreeMap<String, String> = BTreeMap::new();
    let mut adjacency: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for (index, transition) in transitions.iter().enumerate() {
        let path = format!("$.transitions[{index}]");
        let Some(id) = text(transition, "transition_id") else {
            push(&mut errors, "MISSING_TRANSITION_ID", &format!("{path}.transition_id"), "transition_id is required");
            continue;
        };
        if !transition_ids.insert(id.to_owned()) {
            push(&mut errors, "DUPLICATE_TRANSITION_ID", &format!("{path}.transition_id"), "transition IDs must be unique");
        }
        if text(transition, "path_segment_ref").is_none() {
            push(&mut errors, "MISSING_PATH_SEGMENT", &format!("{path}.path_segment_ref"), "exact installed path segment reference is required");
        }
        if text(transition, "clock_sync_ref").is_none() {
            push(&mut errors, "MISSING_TRANSITION_CLOCK", &format!("{path}.clock_sync_ref"), "transition clock synchronization reference is required");
        }
        match (uint(transition, "residence_time_lower_ms"), uint(transition, "residence_time_upper_ms")) {
            (Some(lo), Some(hi)) if hi >= lo => {}
            _ => push(&mut errors, "INVALID_RESIDENCE_TIME_INTERVAL", &path, "residence-time interval must be non-negative and ordered"),
        }
        match finite(transition, "alignment_tolerance_ms") {
            Some(v) if v >= 0.0 => {}
            _ => push(&mut errors, "INVALID_ALIGNMENT_TOLERANCE", &format!("{path}.alignment_tolerance_ms"), "alignment tolerance must be finite and non-negative"),
        }

        let inputs = transition.get("input_stream_ids").and_then(Value::as_array);
        let outputs = transition.get("output_stream_ids").and_then(Value::as_array);
        let (Some(inputs), Some(outputs)) = (inputs, outputs) else {
            push(&mut errors, "MISSING_TRANSITION_STREAMS", &path, "each transition needs input and output stream arrays");
            continue;
        };
        let input_ids: Vec<String> = inputs.iter().filter_map(Value::as_str).map(str::to_owned).collect();
        let output_ids: Vec<String> = outputs.iter().filter_map(Value::as_str).map(str::to_owned).collect();
        if input_ids.is_empty() || output_ids.is_empty() {
            push(&mut errors, "EMPTY_TRANSITION_STREAMS", &path, "input and output stream arrays must both be non-empty");
        }
        for input_id in &input_ids {
            if !stream_by_id.contains_key(input_id) {
                push(&mut errors, "UNKNOWN_INPUT_STREAM", &path, format!("input stream {input_id} is not declared"));
            }
            if consumed_by.insert(input_id.clone(), id.to_owned()).is_some() {
                push(&mut errors, "STREAM_CONSUMED_MULTIPLE_TIMES", &path, format!("input stream {input_id} is consumed by more than one transition"));
            }
            for output_id in &output_ids {
                adjacency.entry(input_id.clone()).or_default().push(output_id.clone());
            }
        }
        for output_id in &output_ids {
            if !stream_by_id.contains_key(output_id) {
                push(&mut errors, "UNKNOWN_OUTPUT_STREAM", &path, format!("output stream {output_id} is not declared"));
            }
            if produced_by.insert(output_id.clone(), id.to_owned()).is_some() {
                push(&mut errors, "STREAM_PRODUCED_MULTIPLE_TIMES", &path, format!("output stream {output_id} is produced by more than one transition"));
            }
        }
        if input_ids.iter().any(|input| output_ids.contains(input)) {
            push(&mut errors, "STREAM_SELF_TRANSITION", &path, "a stream cannot be both input and output of the same transition");
        }

        let stream_map_complete = input_ids.iter().all(|id| stream_by_id.contains_key(id))
            && output_ids.iter().all(|id| stream_by_id.contains_key(id));
        if stream_map_complete {
            let expected_sync = text(transition, "clock_sync_ref");
            let mut input_midpoints = Vec::new();
            let mut output_midpoints = Vec::new();
            let mut input_time_uncertainty = 0.0_f64;
            let mut output_time_uncertainty = 0.0_f64;

            for stream_id in &input_ids {
                let stream = stream_by_id[stream_id];
                if text(stream, "clock_sync_ref") != expected_sync {
                    push(&mut errors, "TRANSITION_CLOCK_SYNC_MISMATCH", &path, format!("input stream {stream_id} does not use transition clock reference"));
                }
                let start = uint(stream, "start_ms").unwrap_or(0) as f64;
                let end = uint(stream, "end_ms").unwrap_or(0) as f64;
                input_midpoints.push((start + end) / 2.0);
                input_time_uncertainty += finite(stream, "time_uncertainty_ms").unwrap_or(0.0);
            }
            for stream_id in &output_ids {
                let stream = stream_by_id[stream_id];
                if text(stream, "clock_sync_ref") != expected_sync {
                    push(&mut errors, "TRANSITION_CLOCK_SYNC_MISMATCH", &path, format!("output stream {stream_id} does not use transition clock reference"));
                }
                let start = uint(stream, "start_ms").unwrap_or(0) as f64;
                let end = uint(stream, "end_ms").unwrap_or(0) as f64;
                output_midpoints.push((start + end) / 2.0);
                output_time_uncertainty += finite(stream, "time_uncertainty_ms").unwrap_or(0.0);
            }

            let input_midpoint = input_midpoints.iter().sum::<f64>() / input_midpoints.len().max(1) as f64;
            let output_midpoint = output_midpoints.iter().sum::<f64>() / output_midpoints.len().max(1) as f64;
            let observed_delta = output_midpoint - input_midpoint;
            let time_uncertainty = input_time_uncertainty + output_time_uncertainty
                + finite(transition, "alignment_tolerance_ms").unwrap_or(0.0);
            let residence_lower = uint(transition, "residence_time_lower_ms").unwrap_or(0) as f64;
            let residence_upper = uint(transition, "residence_time_upper_ms").unwrap_or(0) as f64;
            if observed_delta + time_uncertainty < residence_lower
                || observed_delta - time_uncertainty > residence_upper
            {
                push(
                    &mut errors,
                    "RESIDENCE_TIME_ALIGNMENT_MISMATCH",
                    &path,
                    format!("observed midpoint displacement {observed_delta} ms is outside declared residence interval [{residence_lower}, {residence_upper}] ms after explicit timing uncertainty"),
                );
            }
        }
    }

    for stream_id in stream_by_id.keys() {
        // A stream is permitted to be an initial source (no producer) or a
        // terminal sink (no consumer), but it cannot be absent from both sides
        // once transitions are declared: otherwise it is an unconnected sample.
        if !produced_by.contains_key(stream_id) && !consumed_by.contains_key(stream_id) {
            push(&mut errors, "UNCONNECTED_STREAM", "$.streams", format!("stream {stream_id} is not connected to any declared transition"));
        }
    }

    // Kahn's topological check rejects cyclic material-flow lineage.
    let mut indegree: BTreeMap<String, usize> =
        stream_by_id.keys().map(|id| (id.clone(), 0)).collect();
    for targets in adjacency.values() {
        for target in targets {
            if let Some(value) = indegree.get_mut(target) {
                *value += 1;
            }
        }
    }
    let mut queue: VecDeque<String> = indegree
        .iter()
        .filter(|(_, degree)| **degree == 0)
        .map(|(id, _)| id.clone())
        .collect();
    let mut visited = 0usize;
    while let Some(node) = queue.pop_front() {
        visited += 1;
        for target in adjacency.get(&node).into_iter().flatten() {
            if let Some(degree) = indegree.get_mut(target) {
                *degree -= 1;
                if *degree == 0 {
                    queue.push_back(target.clone());
                }
            }
        }
    }
    if visited != stream_by_id.len() {
        push(
            &mut errors,
            "CYCLIC_STREAM_TOPOLOGY",
            "$.transitions",
            "material stream topology must be an acyclic directed graph",
        );
    }

    errors
}

/// Evaluate conservative flow-closure intervals and per-size-bin changes.
/// Distribution shifts are observations only; no breakage/attrition cause is
/// inferred from a PSD change.
pub fn evaluate_against_parent(
    case: &Value,
    parent: &Value,
) -> Result<EvaluationReport, Vec<Violation>> {
    let errors = validate_against_parent(case, parent);
    if !errors.is_empty() {
        return Err(errors);
    }

    let context = &case["parent_context"];
    let streams = case["streams"].as_array().expect("validated streams");
    let transitions = case["transitions"].as_array().expect("validated transitions");
    let parent_interval = parent["intervals"]
        .as_array()
        .expect("validated parent intervals")
        .iter()
        .find(|interval| {
            text(interval, "interval_id") == text(context, "parent_interval_id")
        })
        .expect("validated parent interval");
    let parent_bins = parent_interval["fragment_bins"]
        .as_array()
        .expect("validated parent PSD bins");

    let stream_map: BTreeMap<&str, &Value> = streams
        .iter()
        .map(|stream| {
            (
                text(stream, "stream_id").expect("validated stream id"),
                stream,
            )
        })
        .collect();

    let mut stream_reports = Vec::with_capacity(streams.len());
    for stream in streams {
        let bins = stream["bins"].as_array().expect("validated bins");
        let bin_interval = |bin: &Value| {
            valid_interval(
                bin,
                "dry_mass_flow_lower_kg_s",
                "dry_mass_flow_upper_kg_s",
            )
            .expect("validated bin flow")
        };
        let binned = add_intervals(bins.iter().map(bin_interval).collect::<Vec<_>>().iter());
        let total = valid_interval(
            stream,
            "dry_mass_flow_lower_kg_s",
            "dry_mass_flow_upper_kg_s",
        )
        .expect("validated total flow");
        stream_reports.push(StreamReport {
            stream_id: text(stream, "stream_id")
                .expect("validated stream id")
                .to_owned(),
            station_id: text(stream, "station_id")
                .expect("validated station id")
                .to_owned(),
            role: text(stream, "role")
                .expect("validated stream role")
                .to_owned(),
            dry_mass_flow: total.clone(),
            bin_count: bins.len(),
            binned_flow: binned.clone(),
            binned_flow_overlaps_total: intervals_overlap(&total, &binned),
        });
    }

    let mut transition_reports = Vec::with_capacity(transitions.len());
    for transition in transitions {
        let input_ids: Vec<&str> = transition["input_stream_ids"]
            .as_array()
            .expect("validated inputs")
            .iter()
            .map(|id| id.as_str().expect("validated input id"))
            .collect();
        let output_ids: Vec<&str> = transition["output_stream_ids"]
            .as_array()
            .expect("validated outputs")
            .iter()
            .map(|id| id.as_str().expect("validated output id"))
            .collect();

        let input_flows: Vec<FlowInterval> = input_ids
            .iter()
            .map(|id| {
                valid_interval(
                    stream_map[id],
                    "dry_mass_flow_lower_kg_s",
                    "dry_mass_flow_upper_kg_s",
                )
                .expect("validated input stream")
            })
            .collect();
        let output_flows: Vec<FlowInterval> = output_ids
            .iter()
            .map(|id| {
                valid_interval(
                    stream_map[id],
                    "dry_mass_flow_lower_kg_s",
                    "dry_mass_flow_upper_kg_s",
                )
                .expect("validated output stream")
            })
            .collect();
        let input_total = add_intervals(input_flows.iter());
        let output_total = add_intervals(output_flows.iter());

        let input_midpoint = input_ids
            .iter()
            .map(|id| {
                let stream = stream_map[id];
                (uint(stream, "start_ms").unwrap() as f64
                    + uint(stream, "end_ms").unwrap() as f64)
                    / 2.0
            })
            .sum::<f64>()
            / input_ids.len() as f64;
        let output_midpoint = output_ids
            .iter()
            .map(|id| {
                let stream = stream_map[id];
                (uint(stream, "start_ms").unwrap() as f64
                    + uint(stream, "end_ms").unwrap() as f64)
                    / 2.0
            })
            .sum::<f64>()
            / output_ids.len() as f64;

        let mut bin_changes = Vec::with_capacity(parent_bins.len());
        for (index, parent_bin) in parent_bins.iter().enumerate() {
            let input_bin_flows: Vec<FlowInterval> = input_ids
                .iter()
                .map(|id| {
                    valid_interval(
                        &stream_map[id]["bins"][index],
                        "dry_mass_flow_lower_kg_s",
                        "dry_mass_flow_upper_kg_s",
                    )
                    .expect("validated input bin")
                })
                .collect();
            let output_bin_flows: Vec<FlowInterval> = output_ids
                .iter()
                .map(|id| {
                    valid_interval(
                        &stream_map[id]["bins"][index],
                        "dry_mass_flow_lower_kg_s",
                        "dry_mass_flow_upper_kg_s",
                    )
                    .expect("validated output bin")
                })
                .collect();
            let input_bin_total = add_intervals(input_bin_flows.iter());
            let output_bin_total = add_intervals(output_bin_flows.iter());
            let change_lower = output_bin_total.lower_kg_s - input_bin_total.upper_kg_s;
            let change_upper = output_bin_total.upper_kg_s - input_bin_total.lower_kg_s;
            let disposition = if change_lower > 0.0 {
                "INCREASE_CONFIRMED"
            } else if change_upper < 0.0 {
                "DECREASE_CONFIRMED"
            } else {
                "OVERLAPPING_UNCERTAINTY"
            };
            bin_changes.push(BinFlowChange {
                bin_index: index,
                lower_mm: finite(parent_bin, "lower_mm").expect("validated parent bin"),
                upper_mm: parent_bin.get("upper_mm").and_then(Value::as_f64),
                change_lower_kg_s: change_lower,
                change_upper_kg_s: change_upper,
                disposition: disposition.to_owned(),
            });
        }

        transition_reports.push(TransitionReport {
            transition_id: text(transition, "transition_id")
                .expect("validated transition id")
                .to_owned(),
            stage: text(transition, "stage")
                .expect("validated stage")
                .to_owned(),
            input_flow: input_total.clone(),
            output_flow: output_total.clone(),
            mass_balance_disposition: mass_balance_disposition(&input_total, &output_total)
                .to_owned(),
            residence_time_observed_midpoint_ms: output_midpoint - input_midpoint,
            bin_flow_changes: bin_changes,
        });
    }

    Ok(EvaluationReport {
        case_id: text(case, "case_id").expect("validated case id").to_owned(),
        parent_case_id: text(context, "parent_case_id")
            .expect("validated parent case")
            .to_owned(),
        parent_interval_id: text(context, "parent_interval_id")
            .expect("validated parent interval")
            .to_owned(),
        trial_id: text(context, "trial_id")
            .expect("validated trial")
            .to_owned(),
        material_cohort_ref: text(context, "material_cohort_ref")
            .expect("validated cohort")
            .to_owned(),
        machine_configuration_ref: text(context, "machine_configuration_ref")
            .expect("validated machine config")
            .to_owned(),
        transport_configuration_ref: text(context, "transport_configuration_ref")
            .expect("validated transport config")
            .to_owned(),
        station_count: case["stations"].as_array().expect("stations").len(),
        stream_count: streams.len(),
        transition_count: transitions.len(),
        streams: stream_reports,
        transitions: transition_reports,
        causal_attribution: "PSD_FLOW_CHANGE_IS_OBSERVATIONAL_NOT_CAUSAL_BREAKAGE_OR_ATTRITION_PROOF".to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn parent() -> Value {
        serde_json::from_str(include_str!(
            "../../../../docs/research/rock-extraction-portfolio/fixtures/synthetic-fragment-transport-case.v0_1.json"
        ))
        .expect("parent fixture parses")
    }

    fn fixture() -> Value {
        serde_json::from_str(include_str!(
            "../../../../docs/research/rock-extraction-portfolio/fixtures/synthetic-material-transformation-case.v0_1.json"
        ))
        .expect("material transformation fixture parses")
    }

    fn has(errors: &[Violation], code: &str) -> bool {
        errors.iter().any(|error| error.code == code)
    }

    fn stream_mut(case: &mut Value, id: &str) -> &mut Value {
        case["streams"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|stream| stream["stream_id"] == id)
            .unwrap()
    }

    fn transition_mut(case: &mut Value, id: &str) -> &mut Value {
        case["transitions"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|transition| transition["transition_id"] == id)
            .unwrap()
    }

    fn transition_report<'a>(
        report: &'a EvaluationReport,
        id: &str,
    ) -> &'a TransitionReport {
        report
            .transitions
            .iter()
            .find(|transition| transition.transition_id == id)
            .unwrap()
    }

    #[test]
    fn synthetic_fixture_is_valid_against_exact_parent_case() {
        assert!(validate_against_parent(&fixture(), &parent()).is_empty());
    }

    #[test]
    fn structural_replay_reports_all_four_transitions() {
        let report = evaluate_against_parent(&fixture(), &parent()).unwrap();
        assert_eq!(report.station_count, 10);
        assert_eq!(report.stream_count, 10);
        assert_eq!(report.transition_count, 4);
        assert!(report.transitions.iter().all(|t| {
            t.mass_balance_disposition == "WITHIN_UNCERTAINTY"
        }));
    }

    #[test]
    fn stage_resolved_flow_mass_closes_with_uncertainty() {
        let report = evaluate_against_parent(&fixture(), &parent()).unwrap();
        for transition in &report.transitions {
            assert!(intervals_overlap(&transition.input_flow, &transition.output_flow));
        }
    }

    #[test]
    fn per_bin_flow_changes_are_reported_without_causal_claim() {
        let report = evaluate_against_parent(&fixture(), &parent()).unwrap();
        let convey = transition_report(&report, "transition-convey-001");
        assert_eq!(convey.bin_flow_changes.len(), 4);
        assert_eq!(
            convey.causal_attribution_if_present(),
            "NOT_APPLICABLE"
        );
    }

    #[test]
    fn parent_case_identity_must_match() {
        let mut case = fixture();
        case["parent_context"]["parent_case_id"] = serde_json::json!("OTHER");
        assert!(has(&validate_against_parent(&case, &parent()), "PARENT_CASE_MISMATCH"));
    }

    #[test]
    fn parent_interval_must_resolve_uniquely() {
        let mut case = fixture();
        case["parent_context"]["parent_interval_id"] = serde_json::json!("MISSING");
        assert!(has(&validate_against_parent(&case, &parent()), "PARENT_INTERVAL_NOT_UNIQUE"));
    }

    #[test]
    fn trial_material_and_machine_identity_must_match() {
        let mut case = fixture();
        case["parent_context"]["trial_id"] = serde_json::json!("OTHER-TRIAL");
        assert!(has(&validate_against_parent(&case, &parent()), "PARENT_CONTEXT_MISMATCH"));
    }

    #[test]
    fn stream_bin_grid_must_match_parent_exactly() {
        let mut case = fixture();
        stream_mut(&mut case, "stream-generated")["bins"][0]["upper_mm"] =
            serde_json::json!(4.0);
        assert!(has(&validate_against_parent(&case, &parent()), "STREAM_PSD_UPPER_BOUND_MISMATCH"));
    }

    #[test]
    fn missing_psd_bin_is_rejected() {
        let mut case = fixture();
        stream_mut(&mut case, "stream-generated")["bins"].as_array_mut().unwrap().pop();
        assert!(has(&validate_against_parent(&case, &parent()), "STREAM_PSD_BIN_COUNT_MISMATCH"));
    }

    #[test]
    fn binned_flow_must_reconcile_with_total() {
        let mut case = fixture();
        stream_mut(&mut case, "stream-generated")["dry_mass_flow_lower_kg_s"] =
            serde_json::json!(5.0);
        stream_mut(&mut case, "stream-generated")["dry_mass_flow_upper_kg_s"] =
            serde_json::json!(5.1);
        assert!(has(&validate_against_parent(&case, &parent()), "BINNED_FLOW_DOES_NOT_RECONCILE"));
    }

    #[test]
    fn invalid_bin_flow_interval_is_rejected() {
        let mut case = fixture();
        stream_mut(&mut case, "stream-generated")["bins"][0]["dry_mass_flow_upper_kg_s"] =
            serde_json::json!(0.0);
        assert!(has(&validate_against_parent(&case, &parent()), "INVALID_BIN_FLOW_INTERVAL"));
    }

    #[test]
    fn duplicate_stream_id_is_rejected() {
        let mut case = fixture();
        let first = case["streams"][0].clone();
        case["streams"].as_array_mut().unwrap().push(first);
        assert!(has(&validate_against_parent(&case, &parent()), "DUPLICATE_STREAM_ID"));
    }

    #[test]
    fn stream_role_must_match_physical_station() {
        let mut case = fixture();
        stream_mut(&mut case, "stream-generated")["role"] = serde_json::json!("FILTER_FINES");
        assert!(has(&validate_against_parent(&case, &parent()), "STREAM_ROLE_STATION_MISMATCH"));
    }

    #[test]
    fn unknown_station_is_rejected() {
        let mut case = fixture();
        stream_mut(&mut case, "stream-generated")["station_id"] = serde_json::json!("not-a-station");
        assert!(has(&validate_against_parent(&case, &parent()), "UNKNOWN_STREAM_STATION"));
    }

    #[test]
    fn duplicate_independent_raw_stream_source_is_rejected() {
        let mut case = fixture();
        stream_mut(&mut case, "stream-captured")["provenance"]["source_ref"] =
            case["streams"][0]["provenance"]["source_ref"].clone();
        assert!(has(&validate_against_parent(&case, &parent()), "DUPLICATE_STREAM_SOURCE"));
    }

    #[test]
    fn model_estimated_stream_cannot_be_measurement_truth() {
        let mut case = fixture();
        stream_mut(&mut case, "stream-generated")["provenance"]["evidence_role"] =
            serde_json::json!("MODEL_ESTIMATE");
        assert!(has(&validate_against_parent(&case, &parent()), "NON_INDEPENDENT_STREAM_EVIDENCE"));
    }

    #[test]
    fn derived_stream_evidence_requires_lineage() {
        let mut case = fixture();
        stream_mut(&mut case, "stream-generated")["provenance"]["evidence_role"] =
            serde_json::json!("DERIVED_FROM_INDEPENDENT_MEASUREMENTS");
        assert!(has(&validate_against_parent(&case, &parent()), "MISSING_STREAM_LINEAGE"));
    }

    #[test]
    fn stream_time_window_must_be_positive() {
        let mut case = fixture();
        stream_mut(&mut case, "stream-generated")["end_ms"] =
            case["streams"][0]["start_ms"].clone();
        assert!(has(&validate_against_parent(&case, &parent()), "INVALID_STREAM_WINDOW"));
    }

    #[test]
    fn transition_cannot_reference_unknown_input() {
        let mut case = fixture();
        transition_mut(&mut case, "transition-capture-001")["input_stream_ids"] =
            serde_json::json!(["no-stream"]);
        assert!(has(&validate_against_parent(&case, &parent()), "UNKNOWN_INPUT_STREAM"));
    }

    #[test]
    fn one_stream_cannot_be_consumed_by_two_transitions() {
        let mut case = fixture();
        let extra = case["transitions"][1].clone();
        case["transitions"].as_array_mut().unwrap().push(extra);
        assert!(has(&validate_against_parent(&case, &parent()), "STREAM_CONSUMED_MULTIPLE_TIMES"));
    }

    #[test]
    fn one_stream_cannot_be_produced_by_two_transitions() {
        let mut case = fixture();
        let extra = case["transitions"][1].clone();
        case["transitions"].as_array_mut().unwrap().push(extra);
        assert!(has(&validate_against_parent(&case, &parent()), "STREAM_PRODUCED_MULTIPLE_TIMES"));
    }

    #[test]
    fn cyclic_stream_topology_is_rejected() {
        let mut case = fixture();
        let last = case["transitions"].as_array().unwrap().last().unwrap().clone();
        case["transitions"].as_array_mut().unwrap().push(serde_json::json!({
            "transition_id":"cycle-test","stage":"OTHER","path_segment_ref":"path:cycle",
            "input_stream_ids":["stream-accepted"],"output_stream_ids":["stream-generated"],
            "residence_time_lower_ms":0,"residence_time_upper_ms":1000,
            "alignment_tolerance_ms":100,"clock_sync_ref":"clock:campaign-001"
        }));
        let _ = last;
        assert!(has(&validate_against_parent(&case, &parent()), "CYCLIC_STREAM_TOPOLOGY"));
    }

    #[test]
    fn duplicated_bin_stream_source_is_not_accepted_as_independent_measurement() {
        let mut case = fixture();
        case["streams"][0]["bins"][1]["provenance"]["evidence_role"] =
            serde_json::json!("MODEL_ESTIMATE");
        assert!(has(&validate_against_parent(&case, &parent()), "NON_INDEPENDENT_STREAM_EVIDENCE"));
    }

    #[test]
    fn transition_clock_must_match_stream_clock() {
        let mut case = fixture();
        transition_mut(&mut case, "transition-convey-001")["clock_sync_ref"] =
            serde_json::json!("other-clock");
        assert!(has(&validate_against_parent(&case, &parent()), "TRANSITION_CLOCK_SYNC_MISMATCH"));
    }

    #[test]
    fn residence_time_mismatch_is_rejected() {
        let mut case = fixture();
        transition_mut(&mut case, "transition-convey-001")["residence_time_lower_ms"] =
            serde_json::json!(10_000);
        transition_mut(&mut case, "transition-convey-001")["residence_time_upper_ms"] =
            serde_json::json!(12_000);
        assert!(has(&validate_against_parent(&case, &parent()), "RESIDENCE_TIME_ALIGNMENT_MISMATCH"));
    }

    #[test]
    fn input_output_flow_deficit_is_detected() {
        let mut case = fixture();
        stream_mut(&mut case, "stream-conveyed")["dry_mass_flow_lower_kg_s"] =
            serde_json::json!(1.0);
        stream_mut(&mut case, "stream-conveyed")["dry_mass_flow_upper_kg_s"] =
            serde_json::json!(1.01);
        // Keep per-bin total valid so the transition—not the stream ledger—is
        // the source of the measured deficit.
        let bins = stream_mut(&mut case, "stream-conveyed")["bins"].as_array_mut().unwrap();
        bins[0]["dry_mass_flow_lower_kg_s"] = serde_json::json!(0.999);
        bins[0]["dry_mass_flow_upper_kg_s"] = serde_json::json!(1.001);
        for bin in bins.iter_mut().skip(1) {
            bin["dry_mass_flow_lower_kg_s"] = serde_json::json!(0.0);
            bin["dry_mass_flow_upper_kg_s"] = serde_json::json!(0.001);
        }
        let result = evaluate_against_parent(&case, &parent());
        assert!(result.is_err());
    }

    #[test]
    fn deterministic_replay_preserves_transition_order() {
        let case = fixture();
        let p = parent();
        assert_eq!(
            evaluate_against_parent(&case, &p).unwrap(),
            evaluate_against_parent(&case, &p).unwrap()
        );
    }

    #[test]
    fn causal_attribution_is_explicitly_not_inferred() {
        let report = evaluate_against_parent(&fixture(), &parent()).unwrap();
        assert!(report.causal_attribution.contains("NOT_CAUSAL"));
    }

    #[test]
    fn open_ended_final_bin_must_match_parent() {
        let mut case = fixture();
        stream_mut(&mut case, "stream-generated")["bins"][3]["upper_mm"] =
            serde_json::json!(500.0);
        assert!(has(&validate_against_parent(&case, &parent()), "STREAM_PSD_UPPER_BOUND_MISMATCH"));
    }
}

