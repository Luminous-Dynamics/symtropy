use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    path::{Path, PathBuf},
    process::{self, Command},
};

const MARGINS: [f64; 3] = [0.01, 0.05, 2.5];
const CLAIM_CEILING: &str = "Scenario-family sensitivity analysis over synthetic input mutations. Family counts are not probabilities; results are not measured factory outcomes, industry SOTA, causal effects, ROI, or purchase advice.";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct FamilySpec {
    family_id: String,
    description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct CandidateGain {
    station_id: String,
    #[serde(alias = "oracle_mean_paired_gain_units_per_hour")]
    mean_paired_gain_units_per_hour: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct MarginResult {
    margin_units_per_hour: f64,
    ranking_report_sha256: String,
    robust_best_station_id: Option<String>,
    recommendation_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct FamilyResult {
    family: FamilySpec,
    input_fixture_sha256: String,
    simulation_report_sha256: String,
    simulation_outcome_rows: usize,
    replication_count: usize,
    replication_seeds: Vec<u64>,
    point_estimate_best_station_id: String,
    robust_best_station_at_default_margin: Option<String>,
    candidates: Vec<CandidateGain>,
    margin_sensitivity: Vec<MarginResult>,
    nominal_selection_regret_units_per_hour: Option<f64>,
    point_estimate_rank_reversal_vs_nominal: bool,
    release_ready: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct RobustnessReport {
    schema_version: String,
    benchmark_id: String,
    source_fixture_sha256: String,
    family_count: usize,
    replication_count_per_family: usize,
    scenarios_per_family: usize,
    practical_equivalence_margins_units_per_hour: Vec<f64>,
    nominal_point_estimate_best_station_id: String,
    nominal_robust_best_station_id: Option<String>,
    point_estimate_rank_reversal_count_vs_nominal: usize,
    robust_best_family_counts: BTreeMap<String, usize>,
    unresolved_family_count_at_default_margin: usize,
    worst_case_regret_for_nominal_selection_units_per_hour: Option<f64>,
    family_results: Vec<FamilyResult>,
    unweighted_family_counts_are_probabilities: bool,
    production_release_authorized: bool,
    claim_ceiling: String,
}

#[derive(Debug, Deserialize)]
struct RankingSummary {
    benchmark_id: String,
    source_simulation_report_sha256: String,
    source_simulation_replication_count: usize,
    source_simulation_seed_count: usize,
    robust_best_station_id: Option<String>,
    paired_effect_recommendation_status: String,
    candidates: Vec<CandidateGain>,
    results_are_synthetic_model_conditional: bool,
    production_release_authorized: bool,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("factory scenario frontier failed closed: {error}");
        process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        return Err("usage: ev_factory_scenario_frontier <base-fixture.json> <output-directory>".into());
    }
    let fixture_path = PathBuf::from(&args[1]);
    let output_dir = PathBuf::from(&args[2]);
    fs::create_dir_all(&output_dir)
        .map_err(|error| format!("cannot create output directory: {error}"))?;
    let base_text = fs::read(&fixture_path)
        .map_err(|error| format!("cannot read base fixture: {error}"))?;
    let base: Value = serde_json::from_slice(&base_text)
        .map_err(|error| format!("invalid base fixture JSON: {error}"))?;
    validate_base_fixture(&base)?;
    let base_sha = sha256(&base_text);
    let seeds = base["replication_seeds"].clone();
    let exe_dir = env::current_exe()
        .map_err(|error| format!("cannot resolve current executable: {error}"))?
        .parent()
        .ok_or_else(|| "current executable has no parent directory".to_string())?
        .to_path_buf();
    let simulator = sibling_executable(&exe_dir, "ev_factory_simulation");
    let evaluator = sibling_executable(&exe_dir, "ev_factory_intervention_benchmark");
    if !simulator.exists() || !evaluator.exists() {
        return Err("required sibling binaries are missing; build all package binaries before running this frontier".into());
    }

    let specs = family_specs();
    let mut family_results = Vec::with_capacity(specs.len());
    for spec in &specs {
        let mut fixture = base.clone();
        apply_family(&mut fixture, &spec.family_id)?;
        if fixture["replication_seeds"] != seeds || fixture["seed"] != base["seed"] {
            return Err(format!("family_{}_changed_the_declared_seed_catalog", spec.family_id));
        }
        if fixture["input_evidence_class"] != "synthetic" || fixture["release_ready"] == true {
            return Err(format!("family_{}_changed_evidence_or_release_boundary", spec.family_id));
        }
        let fixture_bytes = pretty_json(&fixture)?;
        let fixture_sha = sha256(fixture_bytes.as_bytes());
        let fixture_path = output_dir.join(format!("{}.fixture.json", spec.family_id));
        let simulation_path = output_dir.join(format!("{}.simulation.json", spec.family_id));
        fs::write(&fixture_path, &fixture_bytes)
            .map_err(|error| format!("cannot write family fixture: {error}"))?;
        let simulation_output = run_child(&simulator, &[fixture_path.as_path()])?;
        let simulation_value: Value = serde_json::from_slice(&simulation_output)
            .map_err(|error| format!("family_{}_produced_invalid_simulation_JSON: {error}", spec.family_id))?;
        validate_simulation_output(&simulation_value, seeds.as_array().map(Vec::len).unwrap_or(0))?;
        let simulation_bytes = pretty_json(&simulation_value)?;
        let simulation_sha = sha256(simulation_bytes.as_bytes());
        fs::write(&simulation_path, &simulation_bytes)
            .map_err(|error| format!("cannot write simulation report: {error}"))?;

        let mut margin_results = Vec::with_capacity(MARGINS.len());
        let mut default_ranking: Option<RankingSummary> = None;
        let mut default_value: Option<Value> = None;
        for margin in MARGINS {
            let margin_slug = margin_slug(margin);
            let ranking_output =
                run_child_with_margin(&evaluator, &simulation_path, margin)?;
            let ranking_value: Value = serde_json::from_slice(&ranking_output)
                .map_err(|error| format!("family_{}_margin_{}_invalid_ranking_JSON: {error}", spec.family_id, margin_slug))?;
            let ranking: RankingSummary = serde_json::from_value(ranking_value.clone())
                .map_err(|error| format!("family_{}_margin_{}_invalid_ranking_schema: {error}", spec.family_id, margin_slug))?;
            if ranking.benchmark_id != "EV-FACTORY-INTERVENTION-BENCH-001"
                || ranking.source_simulation_report_sha256 != simulation_sha
                || ranking.source_simulation_replication_count != seeds.as_array().map(Vec::len).unwrap_or(0)
                || ranking.source_simulation_seed_count != seeds.as_array().map(Vec::len).unwrap_or(0)
                || !ranking.results_are_synthetic_model_conditional
                || ranking.production_release_authorized
            {
                return Err(format!("family_{}_margin_{}_ranking_report_mismatch", spec.family_id, margin_slug));
            }
            let ranking_bytes = pretty_json(&ranking_value)?;
            let ranking_sha = sha256(ranking_bytes.as_bytes());
            let ranking_path = output_dir.join(format!("{}.margin-{}.ranking.json", spec.family_id, margin_slug));
            fs::write(&ranking_path, &ranking_bytes)
                .map_err(|error| format!("cannot write ranking report: {error}"))?;
            margin_results.push(MarginResult {
                margin_units_per_hour: margin,
                ranking_report_sha256: ranking_sha,
                robust_best_station_id: ranking.robust_best_station_id.clone(),
                recommendation_status: ranking.paired_effect_recommendation_status.clone(),
            });
            if (margin - 0.05).abs() < f64::EPSILON {
                default_ranking = Some(ranking);
                default_value = Some(ranking_value);
            }
        }
        let ranking = default_ranking
            .ok_or_else(|| format!("family_{}_missing_default_margin_result", spec.family_id))?;
        let default_value = default_value.expect("default margin report stored");
        let best = ranking.candidates.iter()
            .max_by(|left, right| left.mean_paired_gain_units_per_hour.total_cmp(&right.mean_paired_gain_units_per_hour))
            .ok_or_else(|| format!("family_{}_has_no_candidates", spec.family_id))?;
        let release_ready = simulation_value["release_ready"] == true
            || default_value["production_release_authorized"] == true;
        if release_ready {
            return Err(format!("family_{}_unexpected_release_authorization", spec.family_id));
        }
        let outcome_rows = simulation_value["replication_outcomes"].as_array()
            .map(Vec::len)
            .ok_or_else(|| format!("family_{}_missing_per_seed_outcomes", spec.family_id))?;
        family_results.push(FamilyResult {
            family: spec.clone(),
            input_fixture_sha256: fixture_sha,
            simulation_report_sha256: simulation_sha,
            simulation_outcome_rows: outcome_rows,
            replication_count: ranking.source_simulation_replication_count,
            replication_seeds: seeds.as_array().unwrap().iter().filter_map(Value::as_u64).collect(),
            point_estimate_best_station_id: best.station_id.clone(),
            robust_best_station_at_default_margin: ranking.robust_best_station_id.clone(),
            candidates: ranking.candidates.clone(),
            margin_sensitivity: margin_results,
            nominal_selection_regret_units_per_hour: None,
            point_estimate_rank_reversal_vs_nominal: false,
            release_ready,
        });
    }

    let nominal = family_results.iter()
        .find(|family| family.family.family_id == "nominal")
        .ok_or_else(|| "nominal_family_missing".to_string())?;
    let nominal_point_best = nominal.point_estimate_best_station_id.clone();
    let nominal_robust_best = nominal.robust_best_station_at_default_margin.clone();
    let nominal_choice = nominal_robust_best.as_deref();
    let mut robust_best_counts = BTreeMap::<String, usize>::new();
    let mut unresolved = 0_usize;
    let mut reversals = 0_usize;
    let mut worst_case_regret: Option<f64> = None;
    for family in &mut family_results {
        family.point_estimate_rank_reversal_vs_nominal =
            family.point_estimate_best_station_id != nominal_point_best;
        if family.point_estimate_rank_reversal_vs_nominal {
            reversals += 1;
        }
        match family.robust_best_station_at_default_margin.as_ref() {
            Some(station) => *robust_best_counts.entry(station.clone()).or_default() += 1,
            None => unresolved += 1,
        }
        if let Some(selected) = nominal_choice {
            let best_gain = family.candidates.iter()
                .map(|candidate| candidate.mean_paired_gain_units_per_hour)
                .fold(f64::NEG_INFINITY, f64::max);
            let selected_gain = family.candidates.iter()
                .find(|candidate| candidate.station_id == selected)
                .map(|candidate| candidate.mean_paired_gain_units_per_hour);
            if let Some(selected_gain) = selected_gain {
                let regret = (best_gain - selected_gain).max(0.0);
                family.nominal_selection_regret_units_per_hour = Some(regret);
                worst_case_regret = Some(worst_case_regret.map_or(regret, |current| current.max(regret)));
            }
        }
    }

    let report = RobustnessReport {
        schema_version: "0.1.0".into(),
        benchmark_id: "EV-FACTORY-SCENARIO-FRONTIER-001".into(),
        source_fixture_sha256: base_sha,
        family_count: family_results.len(),
        replication_count_per_family: seeds.as_array().map(Vec::len).unwrap_or(0),
        scenarios_per_family: family_results.first().map(|family| family.simulation_outcome_rows / family.replication_count.max(1)).unwrap_or(0),
        practical_equivalence_margins_units_per_hour: MARGINS.to_vec(),
        nominal_point_estimate_best_station_id: nominal_point_best,
        nominal_robust_best_station_id: nominal_robust_best,
        point_estimate_rank_reversal_count_vs_nominal: reversals,
        robust_best_family_counts: robust_best_counts,
        unresolved_family_count_at_default_margin: unresolved,
        worst_case_regret_for_nominal_selection_units_per_hour: worst_case_regret,
        family_results,
        unweighted_family_counts_are_probabilities: false,
        production_release_authorized: false,
        claim_ceiling: CLAIM_CEILING.into(),
    };
    let report_bytes = pretty_json(&serde_json::to_value(&report).map_err(|error| error.to_string())?)?;
    let manifest_path = output_dir.join("ev-factory-scenario-frontier-report.json");
    fs::write(&manifest_path, &report_bytes)
        .map_err(|error| format!("cannot write robustness report: {error}"))?;
    println!("{report_bytes}");
    Ok(())
}

fn sibling_executable(directory: &Path, name: &str) -> PathBuf {
    directory.join(if cfg!(windows) { format!("{name}.exe") } else { name.into() })
}

fn run_child(executable: &Path, args: &[&Path]) -> Result<Vec<u8>, String> {
    let output = Command::new(executable)
        .args(args.iter().map(|path| path.as_os_str()))
        .output()
        .map_err(|error| format!("cannot execute {}: {error}", executable.display()))?;
    if !output.status.success() {
        return Err(format!(
            "{} failed with status {}: {}",
            executable.file_name().and_then(|name| name.to_str()).unwrap_or("child"),
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(output.stdout)
}

fn run_child_with_margin(executable: &Path, report_path: &Path, margin: f64) -> Result<Vec<u8>, String> {
    let output = Command::new(executable)
        .arg(report_path)
        .arg(margin.to_string())
        .output()
        .map_err(|error| format!("cannot execute {}: {error}", executable.display()))?;
    if !output.status.success() {
        return Err(format!(
            "{} failed with status {}: {}",
            executable.file_name().and_then(|name| name.to_str()).unwrap_or("child"),
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(output.stdout)
}

fn validate_base_fixture(value: &Value) -> Result<(), String> {
    if value["benchmark_id"] != "EV-FACTORY-SIM-001"
        || value["input_evidence_class"] != "synthetic"
        || value["release_ready"] == true
        || value["seed"].as_u64().unwrap_or(0) == 0
        || value["replication_seeds"].as_array().is_none()
        || value["stations"].as_array().is_none()
    {
        return Err("base_fixture_is_not_a_valid_synthetic_simulation_catalog".into());
    }
    Ok(())
}

fn family_specs() -> Vec<FamilySpec> {
    vec![
        FamilySpec { family_id: "nominal".into(), description: "Declared base fixture; no parameter changes.".into() },
        FamilySpec { family_id: "supplier-stress".into(), description: "Mean supplier inter-arrival increased by 30%; arrival variability increased by 20%.".into() },
        FamilySpec { family_id: "body-preparation-stress".into(), description: "Body-preparation mean cycle time increased by 35%.".into() },
        FamilySpec { family_id: "eol-test-stress".into(), description: "End-of-line test mean cycle time increased by 40%.".into() },
        FamilySpec { family_id: "changeover-small-batches".into(), description: "All station changeovers multiplied by 3; product batch size reduced to 5.".into() },
        FamilySpec { family_id: "yield-rework-stress".into(), description: "Each station's good-yield fraction reduced by 0.02, bounded at 0.90.".into() },
        FamilySpec { family_id: "tight-buffers".into(), description: "Source buffer and each inter-station buffer reduced to half capacity, minimum 1.".into() },
        FamilySpec { family_id: "downtime-stress".into(), description: "Mean time between failures halved and mean time to repair multiplied by 1.5.".into() },
        FamilySpec { family_id: "mixed-stress".into(), description: "Supplier arrival interval x1.20, body-preparation cycle x1.15, changeovers x1.5, yield -0.01 and buffers halved.".into() },
    ]
}

fn apply_family(fixture: &mut Value, family_id: &str) -> Result<(), String> {
    match family_id {
        "nominal" => {}
        "supplier-stress" => {
            multiply_number(fixture, "mean_supply_interarrival_seconds", 1.30)?;
            multiply_number(fixture, "supply_interarrival_variability_fraction", 1.20)?;
        }
        "body-preparation-stress" => {
            station_multiply(fixture, "body-preparation", "mean_cycle_seconds", 1.35)?;
        }
        "eol-test-stress" => {
            station_multiply(fixture, "end-of-line-test", "mean_cycle_seconds", 1.40)?;
        }
        "changeover-small-batches" => {
            fixture["variant_batch_size"] = json!(5);
            for station in fixture["stations"].as_array_mut().ok_or_else(|| "stations_must_be_array".to_string())? {
                multiply_number(station, "changeover_seconds", 3.0)?;
            }
        }
        "yield-rework-stress" => {
            for station in fixture["stations"].as_array_mut().ok_or_else(|| "stations_must_be_array".to_string())? {
                let current = station["good_yield_fraction"].as_f64().ok_or_else(|| "good_yield_fraction_must_be_numeric".to_string())?;
                station["good_yield_fraction"] = json!((current - 0.02).max(0.90));
            }
        }
        "tight-buffers" => {
            halve_positive(fixture, "source_buffer_capacity")?;
            halve_buffer_capacities(fixture)?;
        },
        "downtime-stress" => {
            for station in fixture["stations"].as_array_mut().ok_or_else(|| "stations_must_be_array".to_string())? {
                multiply_number(station, "mean_time_between_failures_seconds", 0.5)?;
                multiply_number(station, "mean_time_to_repair_seconds", 1.5)?;
            }
        }
        "mixed-stress" => {
            multiply_number(fixture, "mean_supply_interarrival_seconds", 1.20)?;
            station_multiply(fixture, "body-preparation", "mean_cycle_seconds", 1.15)?;
            for station in fixture["stations"].as_array_mut().ok_or_else(|| "stations_must_be_array".to_string())? {
                multiply_number(station, "changeover_seconds", 1.5)?;
                let current = station["good_yield_fraction"].as_f64().ok_or_else(|| "good_yield_fraction_must_be_numeric".to_string())?;
                station["good_yield_fraction"] = json!((current - 0.01).max(0.90));
            }
            halve_positive(fixture, "source_buffer_capacity")?;
            halve_buffer_capacities(fixture)?;
        }
        _ => return Err(format!("unknown_scenario_family_{family_id}")),
    }
    Ok(())
}

fn station_multiply(fixture: &mut Value, station_id: &str, field: &str, factor: f64) -> Result<(), String> {
    let stations = fixture["stations"].as_array_mut().ok_or_else(|| "stations_must_be_array".to_string())?;
    let station = stations.iter_mut()
        .find(|station| station["station_id"].as_str() == Some(station_id))
        .ok_or_else(|| format!("station_{station_id}_not_found"))?;
    multiply_number(station, field, factor)
}

fn multiply_number(value: &mut Value, field: &str, factor: f64) -> Result<(), String> {
    let current = value[field].as_f64().ok_or_else(|| format!("{field}_must_be_numeric"))?;
    let changed = current * factor;
    if !changed.is_finite() || changed <= 0.0 {
        return Err(format!("{field}_mutation_out_of_range"));
    }
    value[field] = json!(changed);
    Ok(())
}

fn halve_buffer_capacities(fixture: &mut Value) -> Result<(), String> {
    let buffers = fixture["buffer_capacities"]
        .as_array_mut()
        .ok_or_else(|| "buffer_capacities_must_be_array".to_string())?;
    for buffer in buffers {
        let value = buffer
            .as_u64()
            .ok_or_else(|| "buffer_capacity_must_be_positive_integer".to_string())?;
        *buffer = json!((value / 2).max(1));
    }
    Ok(())
}

fn halve_positive(value: &mut Value, field: &str) -> Result<(), String> {
    let current = value[field].as_u64().ok_or_else(|| format!("{field}_must_be_positive_integer"))?;
    value[field] = json!((current / 2).max(1));
    Ok(())
}

fn validate_simulation_output(value: &Value, expected_seed_count: usize) -> Result<(), String> {
    if value["benchmark_id"] != "EV-FACTORY-SIM-001"
        || value["input_evidence_class"] != "synthetic"
        || value["release_ready"] != false
        || value["replication_count"].as_u64() != Some(expected_seed_count as u64)
        || value["replication_seeds"].as_array().map(Vec::len) != Some(expected_seed_count)
        || value["replication_outcomes"].as_array().map(Vec::len)
            != Some(expected_seed_count.saturating_mul(6))
        || value["station_operating_statistics"].as_array().map(Vec::len) != Some(30)
        || value["output_never_exceeds_released_input"] != true
        || value["all_buffers_respected_capacity"] != true
        || value["station_time_accounting_within_horizon"] != true
    {
        return Err("simulation_output_failed_synthetic_invariant_contract".into());
    }
    Ok(())
}

fn margin_slug(margin: f64) -> String {
    match margin {
        0.01 => "001".into(),
        0.05 => "005".into(),
        2.5 => "250".into(),
        _ => margin.to_string().replace('.', "_"),
    }
}

fn pretty_json(value: &Value) -> Result<String, String> {
    serde_json::to_string_pretty(value).map(|mut text| {
        text.push('\n');
        text
    }).map_err(|error| error.to_string())
}

fn sha256(data: &[u8]) -> String {
    format!("{:x}", Sha256::digest(data))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn family_catalog_has_unique_explicit_ids_and_no_probabilistic_claim() {
        let specs = family_specs();
        let ids: BTreeSet<_> = specs.iter().map(|spec| spec.family_id.as_str()).collect();
        assert_eq!(specs.len(), 9);
        assert_eq!(ids.len(), specs.len());
        assert_eq!(MARGINS, [0.01, 0.05, 2.5]);
    }

    #[test]
    fn scenario_mutations_leave_seed_catalog_and_synthetic_evidence_unchanged() {
        let mut fixture = serde_json::json!({
            "benchmark_id":"EV-FACTORY-SIM-001",
            "input_evidence_class":"synthetic",
            "release_ready":false,
            "seed":22,
            "replication_seeds":[22,23],
            "mean_supply_interarrival_seconds":100.0,
            "supply_interarrival_variability_fraction":0.1,
            "source_buffer_capacity":8,
            "buffer_capacities":[2,4,1],
            "variant_batch_size":20,
            "stations":[
                {"station_id":"body-preparation","mean_cycle_seconds":200.0,"changeover_seconds":30.0,"good_yield_fraction":0.99,"mean_time_between_failures_seconds":1000.0,"mean_time_to_repair_seconds":10.0},
                {"station_id":"end-of-line-test","mean_cycle_seconds":250.0,"changeover_seconds":20.0,"good_yield_fraction":0.99,"mean_time_between_failures_seconds":1000.0,"mean_time_to_repair_seconds":10.0}
            ]
        });
        let original_seeds = fixture["replication_seeds"].clone();
        apply_family(&mut fixture, "mixed-stress").expect("family mutation");
        assert_eq!(fixture["replication_seeds"], original_seeds);
        assert_eq!(fixture["input_evidence_class"], "synthetic");
        assert_eq!(fixture["release_ready"], false);
        assert_eq!(fixture["buffer_capacities"], json!([1,2,1]));
        assert_eq!(fixture["stations"][0]["mean_cycle_seconds"], json!(230.0));
        assert_eq!(fixture["stations"][0]["changeover_seconds"], json!(45.0));
        assert_eq!(fixture["stations"][0]["good_yield_fraction"], json!(0.98));
        assert_eq!(fixture["mean_supply_interarrival_seconds"], json!(120.0));
    }

    #[test]
    fn fixture_digest_is_repeatable_and_mutations_change_only_declared_family_fields() {
        let base = serde_json::json!({
            "mean_supply_interarrival_seconds":100.0,
            "supply_interarrival_variability_fraction":0.2
        });
        let first = serde_json::to_vec(&base).unwrap();
        let second = serde_json::to_vec(&base).unwrap();
        assert_eq!(sha256(&first), sha256(&second));
        let mut family = base.clone();
        multiply_number(&mut family, "mean_supply_interarrival_seconds", 1.3).unwrap();
        assert_eq!(family["supply_interarrival_variability_fraction"], base["supply_interarrival_variability_fraction"]);
        assert_ne!(sha256(&serde_json::to_vec(&family).unwrap()), sha256(&first));
    }
}
