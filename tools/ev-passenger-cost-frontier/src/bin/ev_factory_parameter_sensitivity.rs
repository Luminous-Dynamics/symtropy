use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs, process,
};

const BENCHMARK_ID: &str = "EV-FACTORY-EPISTEMIC-001";
const PRACTICAL_MARGIN_UNITS_PER_HOUR: f64 = 0.05;
const CLAIM_CEILING: &str = "Controlled one-factor-at-a-time sensitivity analysis across synthetic model-input envelopes. Parameter settings are not probability samples or empirical plant estimates; results do not validate a factory, establish SOTA, or authorize capital, safety, quality or production release.";

#[derive(Debug, Clone)]
struct Case {
    case_id: String,
    factor_id: String,
    level: String,
    catalog_path: String,
    report_path: String,
    catalog: Value,
    report: Value,
    catalog_sha256: String,
    report_sha256: String,
    seed_order: Vec<u64>,
    station_ids: Vec<String>,
    baseline_by_seed: BTreeMap<u64, f64>,
    gains_by_station: BTreeMap<String, BTreeMap<u64, f64>>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct DistributionSummary {
    sample_count: usize,
    mean: f64,
    sample_standard_deviation: Option<f64>,
    minimum: f64,
    p05: f64,
    median: f64,
    p95: f64,
    maximum: f64,
    mean_confidence_95_lower: Option<f64>,
    mean_confidence_95_upper: Option<f64>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct ParameterValue {
    path: String,
    value: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct StationLevelResult {
    station_id: String,
    point_estimate_rank: usize,
    mean_paired_gain_units_per_hour: f64,
    paired_gain: DistributionSummary,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct PairwiseLevelComparison {
    station_a_id: String,
    station_b_id: String,
    mean_gain_difference_a_minus_b_units_per_hour: f64,
    paired_gain_difference: DistributionSummary,
    practical_equivalence_margin_units_per_hour: f64,
    decision: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct LevelResult {
    level: String,
    catalog_sha256: String,
    report_sha256: String,
    parameter_values: Vec<ParameterValue>,
    baseline_good_units_per_hour: DistributionSummary,
    station_intervention_gains: Vec<StationLevelResult>,
    pairwise_intervention_comparisons: Vec<PairwiseLevelComparison>,
    point_estimate_best_station_id: Option<String>,
    robust_best_station_id: Option<String>,
    robust_best_rule: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct StationEndpointSensitivity {
    station_id: String,
    low_mean_paired_gain_units_per_hour: f64,
    nominal_mean_paired_gain_units_per_hour: f64,
    high_mean_paired_gain_units_per_hour: f64,
    high_minus_low_paired_gain_units_per_hour: DistributionSummary,
    relative_endpoint_swing_fraction_of_nominal_magnitude: Option<f64>,
    ranking_changed_across_levels: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct FactorSensitivity {
    factor_id: String,
    changed_parameter_paths: Vec<String>,
    low_parameter_values: Vec<ParameterValue>,
    nominal_parameter_values: Vec<ParameterValue>,
    high_parameter_values: Vec<ParameterValue>,
    low_minus_high_baseline_output_effect: DistributionSummary,
    factor_has_point_estimate_winner_switch: bool,
    station_sensitivities: Vec<StationEndpointSensitivity>,
    low: LevelResult,
    nominal: LevelResult,
    high: LevelResult,
    robust_winner_across_low_nominal_high: Option<String>,
    measurement_priority_proxy: f64,
    measurement_priority_note: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct CaseDigest {
    catalog_sha256: String,
    simulation_report_sha256: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct Report {
    schema_version: String,
    benchmark_id: String,
    factor_count: usize,
    parameter_setting_count: usize,
    shared_replication_count: usize,
    shared_replication_seeds: Vec<u64>,
    station_ids: Vec<String>,
    practical_equivalence_margin_units_per_hour: f64,
    case_digests: BTreeMap<String, CaseDigest>,
    factors: Vec<FactorSensitivity>,
    measurement_priority_order: Vec<String>,
    robust_best_across_declared_parameter_envelope: Option<String>,
    recommendation_status: String,
    aleatory_and_epistemic_are_separate: bool,
    factor_levels_are_probability_weighted: bool,
    results_are_synthetic_model_conditional: bool,
    production_release_authorized: bool,
    claim_ceiling: String,
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() != 13 {
        eprintln!("usage: ev_factory_parameter_sensitivity nominal=nominal:catalog.json:report.json factor_low=factor:low:catalog.json:report.json ... (nominal plus 6 low/high pairs)");
        process::exit(2);
    }
    let mut cases = BTreeMap::<String, Case>::new();
    for arg in &args {
        let Some((case_id, specification)) = arg.split_once('=') else {
            eprintln!("invalid case argument {arg:?}");
            process::exit(2);
        };
        match parse_case(case_id, specification) {
            Ok(case) => {
                if cases.insert(case.case_id.clone(), case).is_some() {
                    eprintln!("duplicate case id {case_id}");
                    process::exit(2);
                }
            }
            Err(error) => {
                eprintln!("invalid sensitivity input {case_id}: {error}");
                process::exit(2);
            }
        }
    }
    let report = match evaluate(cases) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("parameter sensitivity rejected inputs: {error}");
            process::exit(2);
        }
    };
    match serde_json::to_string_pretty(&report) {
        Ok(value) => println!("{value}"),
        Err(error) => {
            eprintln!("cannot serialize sensitivity report: {error}");
            process::exit(2);
        }
    }
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn parse_case(case_id: &str, specification: &str) -> Result<Case, String> {
    let parts: Vec<&str> = specification.split(':').collect();
    if parts.len() != 4 {
        return Err("expected factor-id:level:catalog-path:report-path".into());
    }
    let factor_id = parts[0];
    let level = parts[1];
    if case_id.trim().is_empty()
        || factor_id.trim().is_empty()
        || !["nominal", "low", "high"].contains(&level)
    {
        return Err("case_id_factor_id_or_level_invalid".into());
    }
    let catalog_path = parts[2].to_string();
    let report_path = parts[3].to_string();
    let catalog_bytes = fs::read(&catalog_path)
        .map_err(|error| format!("cannot_read_catalog_{catalog_path}:{error}"))?;
    let report_bytes = fs::read(&report_path)
        .map_err(|error| format!("cannot_read_report_{report_path}:{error}"))?;
    let catalog: Value = serde_json::from_slice(&catalog_bytes)
        .map_err(|error| format!("invalid_catalog_json:{error}"))?;
    let report: Value = serde_json::from_slice(&report_bytes)
        .map_err(|error| format!("invalid_report_json:{error}"))?;
    let catalog_sha256 = sha256(&catalog_bytes);
    let report_sha256 = sha256(&report_bytes);
    let (seed_order, station_ids, baseline_by_seed, gains_by_station) =
        validate_and_extract(&catalog, &report)?;
    Ok(Case {
        case_id: case_id.to_string(),
        factor_id: factor_id.to_string(),
        level: level.to_string(),
        catalog_path,
        report_path,
        catalog,
        report,
        catalog_sha256,
        report_sha256,
        seed_order,
        station_ids,
        baseline_by_seed,
        gains_by_station,
    })
}

fn value_str<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    value.get(key).and_then(Value::as_str)
        .ok_or_else(|| format!("{key}_missing_or_not_string"))
}

fn value_bool(value: &Value, key: &str) -> Result<bool, String> {
    value.get(key).and_then(Value::as_bool)
        .ok_or_else(|| format!("{key}_missing_or_not_bool"))
}

fn value_u64(value: &Value, key: &str) -> Result<u64, String> {
    value.get(key).and_then(Value::as_u64)
        .ok_or_else(|| format!("{key}_missing_or_not_u64"))
}

fn value_usize(value: &Value, key: &str) -> Result<usize, String> {
    usize::try_from(value_u64(value, key)?)
        .map_err(|_| format!("{key}_out_of_range"))
}

fn value_f64(value: &Value, key: &str) -> Result<f64, String> {
    let number = value.get(key).and_then(Value::as_f64)
        .ok_or_else(|| format!("{key}_missing_or_not_number"))?;
    if !number.is_finite() {
        return Err(format!("{key}_must_be_finite"));
    }
    Ok(number)
}

fn validate_and_extract(
    catalog: &Value,
    report: &Value,
) -> Result<(
    Vec<u64>,
    Vec<String>,
    BTreeMap<u64, f64>,
    BTreeMap<String, BTreeMap<u64, f64>>,
), String> {
    if value_str(catalog, "schema_version")? != "0.1.0"
        || value_str(catalog, "benchmark_id")? != "EV-FACTORY-SIM-001"
    {
        return Err("unsupported_catalog_schema_or_benchmark".into());
    }
    if value_str(catalog, "input_evidence_class")? != "synthetic" {
        return Err("catalog_must_be_explicitly_synthetic".into());
    }
    if value_str(report, "schema_version")? != "0.1.0"
        || value_str(report, "benchmark_id")? != "EV-FACTORY-SIM-001"
    {
        return Err("unsupported_simulation_report_schema_or_benchmark".into());
    }
    if value_str(report, "input_evidence_class")? != "synthetic"
        || value_bool(report, "release_ready")?
        || !value_bool(report, "output_never_exceeds_released_input")?
        || !value_bool(report, "all_buffers_respected_capacity")?
        || !value_bool(report, "station_time_accounting_within_horizon")?
    {
        return Err("simulation_report_not_synthetic_or_invariants_failed".into());
    }
    if value_str(report, "dispatch_policy")? != value_str(catalog, "dispatch_policy")? {
        return Err("catalog_report_dispatch_policy_mismatch".into());
    }
    let horizon = value_u64(catalog, "horizon_seconds")?;
    let warmup = value_u64(catalog, "warmup_seconds")?;
    if warmup >= horizon || value_u64(report, "measured_horizon_seconds")? != horizon - warmup {
        return Err("catalog_report_measurement_horizon_mismatch".into());
    }
    if value_u64(report, "productive_seconds_per_year")?
        != value_u64(catalog, "productive_seconds_per_year")?
    {
        return Err("catalog_report_annualization_hours_mismatch".into());
    }
    let catalog_seeds: Vec<u64> = catalog.get("replication_seeds")
        .and_then(Value::as_array)
        .ok_or_else(|| "catalog_replication_seeds_missing".to_string())?
        .iter()
        .map(|value| value.as_u64().ok_or_else(|| "catalog_seed_not_u64".to_string()))
        .collect::<Result<_, _>>()?;
    if catalog_seeds.is_empty() || catalog_seeds.iter().any(|seed| *seed == 0) {
        return Err("catalog_seed_list_must_be_nonempty_and_positive".into());
    }
    let unique_seeds: BTreeSet<u64> = catalog_seeds.iter().copied().collect();
    if unique_seeds.len() != catalog_seeds.len() {
        return Err("catalog_seeds_must_be_unique".into());
    }
    let report_seeds: Vec<u64> = report.get("replication_seeds")
        .and_then(Value::as_array)
        .ok_or_else(|| "report_replication_seeds_missing".to_string())?
        .iter()
        .map(|value| value.as_u64().ok_or_else(|| "report_seed_not_u64".to_string()))
        .collect::<Result<_, _>>()?;
    if report_seeds != catalog_seeds
        || value_usize(report, "replication_count")? != catalog_seeds.len()
    {
        return Err("catalog_report_ordered_seed_list_mismatch".into());
    }
    let stations_value = catalog.get("stations").and_then(Value::as_array)
        .ok_or_else(|| "catalog_stations_missing".to_string())?;
    if stations_value.len() < 2 {
        return Err("catalog_requires_at_least_two_stations".into());
    }
    let mut station_ids = Vec::with_capacity(stations_value.len());
    for station in stations_value {
        let id = value_str(station, "station_id")?.to_string();
        if id.trim().is_empty() || station_ids.contains(&id) {
            return Err("catalog_station_ids_must_be_nonempty_and_unique".into());
        }
        station_ids.push(id);
    }
    let expected_scenario_count = station_ids.len() + 1;
    if value_usize(report, "scenario_count")? != expected_scenario_count {
        return Err("report_scenario_count_mismatch".into());
    }
    let outcomes = report.get("replication_outcomes").and_then(Value::as_array)
        .ok_or_else(|| "replication_outcomes_missing".to_string())?;
    if outcomes.len() != catalog_seeds.len() * expected_scenario_count {
        return Err("replication_outcome_count_mismatch".into());
    }
    let seed_set: BTreeSet<u64> = catalog_seeds.iter().copied().collect();
    let mut seen = BTreeSet::<(u64, String)>::new();
    let mut baseline_by_seed = BTreeMap::new();
    let mut gains_by_station = BTreeMap::<String, BTreeMap<u64, f64>>::new();
    for station_id in &station_ids {
        gains_by_station.insert(station_id.clone(), BTreeMap::new());
    }
    for row in outcomes {
        let seed = value_u64(row, "seed")?;
        let scenario_id = value_str(row, "scenario_id")?.to_string();
        let good_rate = value_f64(row, "good_units_per_hour")?;
        let base_rate = value_f64(row, "baseline_good_units_per_hour")?;
        if !seed_set.contains(&seed) || good_rate < 0.0 || base_rate < 0.0
            || !seen.insert((seed, scenario_id.clone()))
        {
            return Err("replication_outcome_invalid_seed_rate_or_duplicate".into());
        }
        if scenario_id == "baseline" {
            if (good_rate - base_rate).abs() > 1e-9
                || row.get("paired_good_units_per_hour_delta_vs_baseline")
                    .is_some_and(|value| !value.is_null())
            {
                return Err("baseline_outcome_arithmetic_invalid".into());
            }
            baseline_by_seed.insert(seed, good_rate);
        } else {
            let station_id = row.get("intervention_station_id").and_then(Value::as_str)
                .ok_or_else(|| "intervention_outcome_station_id_missing".to_string())?;
            if !station_ids.iter().any(|id| id == station_id)
                || scenario_id != format!("add-one-server-{station_id}")
            {
                return Err("intervention_outcome_scenario_station_mismatch".into());
            }
            let delta = row.get("paired_good_units_per_hour_delta_vs_baseline")
                .and_then(Value::as_f64)
                .ok_or_else(|| "intervention_delta_missing_or_not_number".to_string())?;
            if !delta.is_finite() || (delta - (good_rate - base_rate)).abs() > 1e-9 {
                return Err("intervention_delta_arithmetic_mismatch".into());
            }
            gains_by_station.get_mut(station_id).expect("known station")
                .insert(seed, delta);
        }
    }
    if baseline_by_seed.len() != seed_set.len()
        || gains_by_station.values().any(|rows| rows.len() != seed_set.len())
    {
        return Err("per_seed_baseline_or_intervention_coverage_incomplete".into());
    }
    for seed in &catalog_seeds {
        if let Some(base) = baseline_by_seed.get(seed) {
            for rows in gains_by_station.values() {
                if !rows.contains_key(seed) {
                    return Err("per_seed_intervention_pair_missing".into());
                }
            }
            let baseline_rows: Vec<_> = outcomes.iter()
                .filter(|row| row.get("seed").and_then(Value::as_u64) == Some(*seed)
                    && row.get("scenario_id").and_then(Value::as_str) == Some("baseline"))
                .collect();
            if baseline_rows.len() != 1 || (value_f64(baseline_rows[0], "good_units_per_hour")? - *base).abs() > 1e-9 {
                return Err("per_seed_baseline_duplicate_or_inconsistent".into());
            }
        }
    }
    Ok((catalog_seeds, station_ids, baseline_by_seed, gains_by_station))
}

fn factor_paths(factor_id: &str, nominal_catalog: &Value) -> Result<Vec<String>, String> {
    let stations = nominal_catalog.get("stations").and_then(Value::as_array)
        .ok_or_else(|| "nominal_station_array_missing".to_string())?;
    let paths = match factor_id {
        "supplier_interarrival" => vec!["/mean_supply_interarrival_seconds".to_string()],
        "cycle_time_scale" => stations.iter().enumerate()
            .map(|(index, _)| format!("/stations/{index}/mean_cycle_seconds")).collect(),
        "yield_envelope" => stations.iter().enumerate()
            .map(|(index, _)| format!("/stations/{index}/good_yield_fraction")).collect(),
        "changeover_scale" => stations.iter().enumerate()
            .map(|(index, _)| format!("/stations/{index}/changeover_seconds")).collect(),
        "mtbf_scale" => stations.iter().enumerate()
            .map(|(index, _)| format!("/stations/{index}/mean_time_between_failures_seconds")).collect(),
        "mttr_scale" => stations.iter().enumerate()
            .map(|(index, _)| format!("/stations/{index}/mean_time_to_repair_seconds")).collect(),
        _ => return Err(format!("unknown_factor_{factor_id}")),
    };
    Ok(paths)
}

fn path_segments(pointer: &str) -> Result<Vec<String>, String> {
    if !pointer.starts_with('/') {
        return Err("json_pointer_must_start_with_slash".into());
    }
    Ok(pointer[1..].split('/')
        .map(|segment| segment.replace("~1", "/").replace("~0", "~"))
        .collect())
}

fn value_at_pointer<'a>(root: &'a Value, pointer: &str) -> Result<&'a Value, String> {
    let mut current = root;
    for segment in path_segments(pointer)? {
        current = match current {
            Value::Object(map) => map.get(&segment)
                .ok_or_else(|| format!("json_pointer_object_path_missing_{pointer}"))?,
            Value::Array(items) => {
                let index = segment.parse::<usize>()
                    .map_err(|_| format!("json_pointer_array_index_invalid_{pointer}"))?;
                items.get(index)
                    .ok_or_else(|| format!("json_pointer_array_path_missing_{pointer}"))?
            }
            _ => return Err(format!("json_pointer_crosses_scalar_{pointer}")),
        };
    }
    Ok(current)
}

fn number_at_pointer(root: &Value, pointer: &str) -> Result<f64, String> {
    value_at_pointer(root, pointer)?.as_f64()
        .filter(|value| value.is_finite())
        .ok_or_else(|| format!("json_pointer_value_not_finite_number_{pointer}"))
}

fn set_pointer(root: &mut Value, pointer: &str, replacement: Value) -> Result<(), String> {
    let segments = path_segments(pointer)?;
    let (last, parents) = segments.split_last()
        .ok_or_else(|| "json_pointer_empty".to_string())?;
    let mut current = root;
    for segment in parents {
        current = match current {
            Value::Object(map) => map.get_mut(segment)
                .ok_or_else(|| format!("json_pointer_object_path_missing_{pointer}"))?,
            Value::Array(items) => {
                let index = segment.parse::<usize>()
                    .map_err(|_| format!("json_pointer_array_index_invalid_{pointer}"))?;
                items.get_mut(index)
                    .ok_or_else(|| format!("json_pointer_array_path_missing_{pointer}"))?
            }
            _ => return Err(format!("json_pointer_crosses_scalar_{pointer}")),
        };
    }
    match current {
        Value::Object(map) => {
            if !map.contains_key(last) {
                return Err(format!("json_pointer_leaf_missing_{pointer}"));
            }
            map.insert(last.clone(), replacement);
        }
        Value::Array(items) => {
            let index = last.parse::<usize>()
                .map_err(|_| format!("json_pointer_array_index_invalid_{pointer}"))?;
            let slot = items.get_mut(index)
                .ok_or_else(|| format!("json_pointer_array_leaf_missing_{pointer}"))?;
            *slot = replacement;
        }
        _ => return Err(format!("json_pointer_parent_scalar_{pointer}")),
    }
    Ok(())
}

fn mask_paths(catalog: &Value, paths: &[String]) -> Result<Value, String> {
    let mut masked = catalog.clone();
    for path in paths {
        set_pointer(&mut masked, path, Value::String("__DECLARED_FACTOR_PATH__".into()))?;
    }
    Ok(masked)
}

fn validate_factor_design(nominal: &Case, low: &Case, high: &Case) -> Result<Vec<String>, String> {
    if low.factor_id != high.factor_id || low.factor_id == "nominal"
        || low.level != "low" || high.level != "high"
    {
        return Err("factor_low_high_labels_do_not_match".into());
    }
    let paths = factor_paths(&low.factor_id, &nominal.catalog)?;
    if mask_paths(&nominal.catalog, &paths)? != mask_paths(&low.catalog, &paths)?
        || mask_paths(&nominal.catalog, &paths)? != mask_paths(&high.catalog, &paths)?
    {
        return Err(format!("factor_{}_changed_undeclared_catalog_paths", low.factor_id));
    }
    let mut changed_values = false;
    for path in &paths {
        let nominal_value = number_at_pointer(&nominal.catalog, path)?;
        let low_value = number_at_pointer(&low.catalog, path)?;
        let high_value = number_at_pointer(&high.catalog, path)?;
        if !(low_value < nominal_value && nominal_value < high_value) {
            return Err(format!("factor_{}_low_nominal_high_values_not_strictly_ordered_at_{}", low.factor_id, path));
        }
        if (low_value - nominal_value).abs() > 1e-12 || (high_value - nominal_value).abs() > 1e-12 {
            changed_values = true;
        }
    }
    if !changed_values {
        return Err(format!("factor_{}_endpoints_do_not_change_any_value", low.factor_id));
    }
    for case in [low, high] {
        if case.seed_order != nominal.seed_order || case.station_ids != nominal.station_ids {
            return Err(format!("factor_{}_report_does_not_share_nominal_seed_or_station_order", low.factor_id));
        }
    }
    Ok(paths)
}

fn parameter_values(catalog: &Value, paths: &[String]) -> Result<Vec<ParameterValue>, String> {
    paths.iter().map(|path| Ok(ParameterValue {
        path: path.clone(),
        value: number_at_pointer(catalog, path)?,
    })).collect()
}

fn summarize(values: &[f64]) -> Result<DistributionSummary, String> {
    if values.is_empty() || values.iter().any(|value| !value.is_finite()) {
        return Err("summary_requires_nonempty_finite_values".into());
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let count = sorted.len();
    let mean = sorted.iter().sum::<f64>() / count as f64;
    let sd = if count > 1 {
        Some((sorted.iter().map(|value| (value - mean).powi(2)).sum::<f64>() / (count - 1) as f64).sqrt())
    } else {
        None
    };
    let ci = sd.map(|sample_sd| {
        let margin = student_t_critical_95((count - 1) as u32) * sample_sd / (count as f64).sqrt();
        (mean - margin, mean + margin)
    });
    Ok(DistributionSummary {
        sample_count: count,
        mean,
        sample_standard_deviation: sd,
        minimum: sorted[0],
        p05: quantile(&sorted, 0.05),
        median: quantile(&sorted, 0.5),
        p95: quantile(&sorted, 0.95),
        maximum: sorted[count - 1],
        mean_confidence_95_lower: ci.map(|bounds| bounds.0),
        mean_confidence_95_upper: ci.map(|bounds| bounds.1),
    })
}

fn quantile(sorted: &[f64], p: f64) -> f64 {
    let pos = p.clamp(0.0, 1.0) * (sorted.len() - 1) as f64;
    let lo = pos.floor() as usize;
    let hi = pos.ceil() as usize;
    sorted[lo] * (1.0 - (pos - lo as f64)) + sorted[hi] * (pos - lo as f64)
}

fn student_t_critical_95(df: u32) -> f64 {
    match df {
        0 => f64::NAN,
        1 => 12.706,
        2 => 4.303,
        3 => 3.182,
        4 => 2.776,
        5 => 2.571,
        6 => 2.447,
        7 => 2.365,
        8 => 2.306,
        9 => 2.262,
        10 => 2.228,
        11 => 2.201,
        12 => 2.179,
        13 => 2.160,
        14 => 2.145,
        15 => 2.131,
        16 => 2.120,
        17 => 2.110,
        18 => 2.101,
        19 => 2.093,
        20 => 2.086,
        21 => 2.080,
        22 => 2.074,
        23 => 2.069,
        24 => 2.064,
        25 => 2.060,
        26 => 2.056,
        27 => 2.052,
        28 => 2.048,
        29 => 2.045,
        _ => 1.960,
    }
}

fn pairwise(
    left_station: &str,
    right_station: &str,
    gains: &BTreeMap<String, BTreeMap<u64, f64>>,
    seed_order: &[u64],
) -> Result<PairwiseLevelComparison, String> {
    let left = gains.get(left_station).ok_or_else(|| format!("gains_missing_{left_station}"))?;
    let right = gains.get(right_station).ok_or_else(|| format!("gains_missing_{right_station}"))?;
    let mut difference = Vec::with_capacity(seed_order.len());
    for seed in seed_order {
        let a = left.get(seed).ok_or_else(|| format!("station_{left_station}_seed_{seed}_missing"))?;
        let b = right.get(seed).ok_or_else(|| format!("station_{right_station}_seed_{seed}_missing"))?;
        difference.push(a - b);
    }
    let summary = summarize(&difference)?;
    let decision = paired_decision(&summary, PRACTICAL_MARGIN_UNITS_PER_HOUR);
    Ok(PairwiseLevelComparison {
        station_a_id: left_station.to_string(),
        station_b_id: right_station.to_string(),
        mean_gain_difference_a_minus_b_units_per_hour: summary.mean,
        paired_gain_difference: summary,
        practical_equivalence_margin_units_per_hour: PRACTICAL_MARGIN_UNITS_PER_HOUR,
        decision: decision.into(),
    })
}

fn paired_decision(summary: &DistributionSummary, margin: f64) -> &'static str {
    match (summary.mean_confidence_95_lower, summary.mean_confidence_95_upper) {
        (Some(lower), Some(upper)) if lower > margin => "station_a_meaningfully_better_under_paired_model",
        (Some(lower), Some(upper)) if upper < -margin => "station_b_meaningfully_better_under_paired_model",
        (Some(lower), Some(upper)) if lower >= -margin && upper <= margin => "practically_equivalent_within_declared_margin",
        _ => "inconclusive_under_paired_interval",
    }
}

fn level_result(case: &Case, nominal: &Case, label: &str, paths: &[String]) -> Result<LevelResult, String> {
    let baseline_values: Vec<f64> = case.seed_order.iter()
        .map(|seed| *case.baseline_by_seed.get(seed).expect("validated baseline seed"))
        .collect();
    let baseline_summary = summarize(&baseline_values)?;

    let mut gains_summary = Vec::new();
    for station_id in &case.station_ids {
        let rows = case.gains_by_station.get(station_id).expect("validated station");
        let values: Vec<f64> = case.seed_order.iter()
            .map(|seed| *rows.get(seed).expect("validated station seed"))
            .collect();
        gains_summary.push(StationLevelResult {
            station_id: station_id.clone(),
            point_estimate_rank: 0,
            mean_paired_gain_units_per_hour: values.iter().sum::<f64>() / values.len() as f64,
            paired_gain: summarize(&values)?,
        });
    }
    gains_summary.sort_by(|a, b| {
        b.mean_paired_gain_units_per_hour.total_cmp(&a.mean_paired_gain_units_per_hour)
            .then_with(|| a.station_id.cmp(&b.station_id))
    });
    for (rank, station) in gains_summary.iter_mut().enumerate() {
        station.point_estimate_rank = rank + 1;
    }
    let point_best = if gains_summary.len() > 1
        && (gains_summary[0].mean_paired_gain_units_per_hour - gains_summary[1].mean_paired_gain_units_per_hour).abs() <= 1e-9
    {
        None
    } else {
        gains_summary.first().map(|row| row.station_id.clone())
    };

    let mut gain_map = BTreeMap::new();
    for station in &case.station_ids {
        gain_map.insert(station.clone(), case.gains_by_station[station].clone());
    }
    let mut comparisons = Vec::new();
    for i in 0..case.station_ids.len() {
        for j in i + 1..case.station_ids.len() {
            comparisons.push(pairwise(&case.station_ids[i], &case.station_ids[j], &gain_map, &case.seed_order)?);
        }
    }
    let robust_best = case.station_ids.iter().find(|candidate| {
        case.station_ids.iter().filter(|other| *other != *candidate).all(|other| {
            comparisons.iter().find(|comparison| {
                (comparison.station_a_id == **candidate && comparison.station_b_id == **other)
                    || (comparison.station_a_id == **other && comparison.station_b_id == **candidate)
            }).is_some_and(|comparison| {
                if comparison.station_a_id == **candidate {
                    comparison.decision == "station_a_meaningfully_better_under_paired_model"
                } else {
                    comparison.decision == "station_b_meaningfully_better_under_paired_model"
                }
            })
        })
    }).cloned();

    Ok(LevelResult {
        level: label.to_string(),
        catalog_sha256: case.catalog_sha256.clone(),
        report_sha256: case.report_sha256.clone(),
        parameter_values: parameter_values(&case.catalog, paths)?,
        baseline_good_units_per_hour: baseline_summary,
        station_intervention_gains: gains_summary,
        pairwise_intervention_comparisons: comparisons,
        point_estimate_best_station_id: point_best,
        robust_best_station_id: robust_best,
        robust_best_rule: "one station must beat every other station by more than the predeclared practical margin using paired per-seed gain differences; this describes only this one synthetic model specification".into(),
    })
}

fn evaluate(cases: BTreeMap<String, Case>) -> Result<Report, String> {
    let nominal = cases.get("nominal").ok_or_else(|| "nominal_case_missing".to_string())?;
    if nominal.factor_id != "nominal" || nominal.level != "nominal" {
        return Err("nominal_case_label_invalid".into());
    }
    const FACTORS: [&str; 6] = [
        "supplier_interarrival",
        "cycle_time_scale",
        "yield_envelope",
        "changeover_scale",
        "mtbf_scale",
        "mttr_scale",
    ];
    if cases.len() != 13 {
        return Err("requires_exactly_nominal_plus_six_low_high_factor_pairs".into());
    }
    let mut expected_ids = BTreeSet::from(["nominal".to_string()]);
    for factor in FACTORS {
        expected_ids.insert(format!("{factor}_low"));
        expected_ids.insert(format!("{factor}_high"));
    }
    if cases.keys().cloned().collect::<BTreeSet<_>>() != expected_ids {
        return Err("sensitivity_case_ids_do_not_match_versioned_design".into());
    }
    let mut case_digests = BTreeMap::new();
    case_digests.insert("nominal".into(), CaseDigest {
        catalog_sha256: nominal.catalog_sha256.clone(),
        simulation_report_sha256: nominal.report_sha256.clone(),
    });
    let mut factors = Vec::new();

    for factor in FACTORS {
        let low = cases.get(&format!("{factor}_low")).expect("validated low case");
        let high = cases.get(&format!("{factor}_high")).expect("validated high case");
        let paths = validate_factor_design(nominal, low, high)?;
        if low.factor_id != factor || high.factor_id != factor {
            return Err(format!("factor_case_id_mismatch_{factor}"));
        }
        let nominal_paths = factor_paths(factor, &nominal.catalog)?;
        for case in [low, high] {
            case_digests.insert(case.case_id.clone(), CaseDigest {
                catalog_sha256: case.catalog_sha256.clone(),
                simulation_report_sha256: case.report_sha256.clone(),
            });
        }
        let low_result = level_result(low, nominal, "low", &paths)?;
        let nominal_result = level_result(nominal, nominal, "nominal", &paths)?;
        let high_result = level_result(high, nominal, "high", &paths)?;
        let baseline_differences: Vec<f64> = nominal.seed_order.iter().map(|seed| {
            low.baseline_by_seed[seed] - high.baseline_by_seed[seed]
        }).collect();
        let baseline_low_minus_high = summarize(&baseline_differences)?;
        let low_point = low_result.point_estimate_best_station_id.clone();
        let nominal_point = nominal_result.point_estimate_best_station_id.clone();
        let high_point = high_result.point_estimate_best_station_id.clone();
        let ranking_changed = low_point != nominal_point || nominal_point != high_point;

        let mut station_sensitivities = Vec::new();
        for station_id in &nominal.station_ids {
            let low_gain = &low.gains_by_station[station_id];
            let nominal_gain = &nominal.gains_by_station[station_id];
            let high_gain = &high.gains_by_station[station_id];
            let differences: Vec<f64> = nominal.seed_order.iter().map(|seed| {
                high_gain[seed] - low_gain[seed]
            }).collect();
            let swing = summarize(&differences)?;
            let low_mean = low_result.station_intervention_gains.iter()
                .find(|row| row.station_id == *station_id).expect("low candidate").mean_paired_gain_units_per_hour;
            let nominal_mean = nominal_result.station_intervention_gains.iter()
                .find(|row| row.station_id == *station_id).expect("nominal candidate").mean_paired_gain_units_per_hour;
            let high_mean = high_result.station_intervention_gains.iter()
                .find(|row| row.station_id == *station_id).expect("high candidate").mean_paired_gain_units_per_hour;
            let relative = if nominal_mean.abs() > 1e-9 {
                Some((high_mean - low_mean) / nominal_mean.abs())
            } else {
                None
            };
            station_sensitivities.push(StationEndpointSensitivity {
                station_id: station_id.clone(),
                low_mean_paired_gain_units_per_hour: low_mean,
                nominal_mean_paired_gain_units_per_hour: nominal_mean,
                high_mean_paired_gain_units_per_hour: high_mean,
                high_minus_low_paired_gain_units_per_hour: swing,
                relative_endpoint_swing_fraction_of_nominal_magnitude: relative,
                ranking_changed_across_levels: ranking_changed,
            });
        }
        let robust_winner = [ &low_result, &nominal_result, &high_result ]
            .iter()
            .map(|result| result.robust_best_station_id.clone())
            .reduce(|left, right| if left == right { left } else { None })
            .flatten();
        let maximum_gain_swing = station_sensitivities.iter()
            .map(|row| row.high_minus_low_paired_gain_units_per_hour.mean.abs())
            .fold(0.0_f64, f64::max);
        let max_relative = station_sensitivities.iter()
            .filter_map(|row| row.relative_endpoint_swing_fraction_of_nominal_magnitude)
            .map(f64::abs)
            .fold(0.0_f64, f64::max);
        let priority = if ranking_changed { maximum_gain_swing + 1.0 } else { maximum_gain_swing };
        factors.push(FactorSensitivity {
            factor_id: factor.into(),
            changed_parameter_paths: paths.clone(),
            low_parameter_values: parameter_values(&low.catalog, &paths)?,
            nominal_parameter_values: parameter_values(&nominal.catalog, &nominal_paths)?,
            high_parameter_values: parameter_values(&high.catalog, &paths)?,
            low_minus_high_baseline_output_effect: baseline_low_minus_high,
            factor_has_point_estimate_winner_switch: ranking_changed,
            station_sensitivities,
            low: low_result,
            nominal: nominal_result,
            high: high_result,
            robust_winner_across_low_nominal_high: robust_winner,
            measurement_priority_proxy: priority.max(max_relative),
            measurement_priority_note: "heuristic only: higher when a point-estimate winner changes or candidate gains move materially across the declared endpoint settings; not formal value-of-information and not weighted by real-world factor likelihood or measurement cost".into(),
        });
    }
    factors.sort_by(|a, b| b.measurement_priority_proxy.total_cmp(&a.measurement_priority_proxy)
        .then_with(|| a.factor_id.cmp(&b.factor_id)));
    let measurement_priority_order = factors.iter().map(|factor| factor.factor_id.clone()).collect();
    let candidate_ids = nominal.station_ids.clone();
    let global_robust_best = candidate_ids.iter().find(|candidate| {
        factors.iter().all(|factor| {
            [&factor.low, &factor.nominal, &factor.high].iter().all(|level| {
                level.robust_best_station_id.as_deref() == Some(candidate.as_str())
            })
        })
    }).cloned();
    let status = if global_robust_best.is_some() {
        "unique_station_intervention_robust_across_all_declared_one_factor_envelopes"
    } else {
        "no_single_intervention_robust_across_entire_declared_envelope_abstain"
    };
    Ok(Report {
        schema_version: "0.1.0".into(),
        benchmark_id: BENCHMARK_ID.into(),
        factor_count: factors.len(),
        parameter_setting_count: 1 + 2 * factors.len(),
        shared_replication_count: nominal.seed_order.len(),
        shared_replication_seeds: nominal.seed_order.clone(),
        station_ids: nominal.station_ids.clone(),
        practical_equivalence_margin_units_per_hour: PRACTICAL_MARGIN_UNITS_PER_HOUR,
        case_digests,
        factors,
        measurement_priority_order,
        robust_best_across_declared_parameter_envelope: global_robust_best,
        recommendation_status: status.into(),
        aleatory_and_epistemic_are_separate: true,
        factor_levels_are_probability_weighted: false,
        results_are_synthetic_model_conditional: true,
        production_release_authorized: false,
        claim_ceiling: CLAIM_CEILING.into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empirical_summary_is_hand_computable() {
        let summary = summarize(&[1.0, 2.0, 3.0]).unwrap();
        assert!((summary.mean - 2.0).abs() < 1e-12);
        assert!((summary.p05 - 1.1).abs() < 1e-12);
        assert_eq!(summary.sample_standard_deviation, Some(1.0));
    }

    #[test]
    fn paired_decision_has_advantage_equivalence_and_inconclusive_states() {
        let clear = summarize(&[0.8, 0.8, 0.8, 0.8, 0.8]).unwrap();
        assert_eq!(paired_decision(&clear, 0.05), "station_a_meaningfully_better_under_paired_model");
        let equal = summarize(&[0.01, 0.02, 0.0, 0.01, 0.02]).unwrap();
        assert_eq!(paired_decision(&equal, 0.05), "practically_equivalent_within_declared_margin");
        let variable = summarize(&[-0.2, 0.1, 0.3, -0.1, 0.0]).unwrap();
        assert_eq!(paired_decision(&variable, 0.05), "inconclusive_under_paired_interval");
    }

    #[test]
    fn masked_catalog_preserves_all_undeclared_fields() {
        let nominal: Value = serde_json::from_str(r#"{"x":1,"nested":{"target":2,"stay":3}}"#).unwrap();
        let mut low = nominal.clone();
        low["nested"]["target"] = Value::from(1);
        let mut high = nominal.clone();
        high["nested"]["target"] = Value::from(4);
        let paths = vec!["/nested/target".to_string()];
        assert_eq!(mask_paths(&nominal, &paths).unwrap(), mask_paths(&low, &paths).unwrap());
        assert_eq!(mask_paths(&nominal, &paths).unwrap(), mask_paths(&high, &paths).unwrap());
        assert_ne!(nominal, low);
    }

    #[test]
    fn factor_catalog_changes_outside_declared_paths_are_rejected() {
        let nominal: Value = serde_json::from_str(r#"{"mean_supply_interarrival_seconds":235,"horizon_seconds":100,"stations":[]}"#).unwrap();
        let mut low = nominal.clone();
        low["mean_supply_interarrival_seconds"] = Value::from(200);
        let mut high = nominal.clone();
        high["mean_supply_interarrival_seconds"] = Value::from(260);
        high["horizon_seconds"] = Value::from(90);
        let paths = vec!["/mean_supply_interarrival_seconds".to_string()];
        assert_eq!(mask_paths(&nominal, &paths).unwrap(), mask_paths(&low, &paths).unwrap());
        assert_ne!(mask_paths(&nominal, &paths).unwrap(), mask_paths(&high, &paths).unwrap());
    }
}
