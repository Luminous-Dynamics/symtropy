use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, env, fs, process};

const CLAIM_CEILING: &str = "Declared-path accounting screen only; not a reactor model, grid study, thermal-hydraulic qualification, electromagnetic safety assessment, or physical fleet result.";

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Carrier {
    Electrical,
    Thermal,
}

#[derive(Debug, Clone, Deserialize)]
struct Benchmark {
    schema_version: String,
    scenarios: Vec<EnergyPath>,
}

#[derive(Debug, Clone, Deserialize)]
struct EnergyPath {
    scenario_id: String,
    label: String,
    evidence_class: String,
    carrier: Carrier,
    source_available_kw: f64,
    requested_load_kw: f64,
    operation_hours: f64,
    source_temperature_c: Option<f64>,
    required_delivery_temperature_c: Option<f64>,
    minimum_approach_temperature_c: Option<f64>,
    stages: Vec<Stage>,
}

#[derive(Debug, Clone, Deserialize)]
struct Stage {
    stage_id: String,
    carrier: Carrier,
    efficiency_fraction: f64,
    max_output_kw: f64,
    temperature_drop_c: Option<f64>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Status {
    FeasibleUnderDeclaredScreen,
    NotFeasibleUnderDeclaredScreen,
    Indeterminate,
    InvalidInput,
}

#[derive(Debug, Clone, Serialize)]
struct StageResult {
    stage_id: String,
    required_input_power_kw: f64,
    required_output_power_kw: f64,
    conversion_loss_kw: f64,
    capacity_headroom_kw: f64,
}

#[derive(Debug, Clone, Serialize)]
struct PathResult {
    scenario_id: String,
    label: String,
    evidence_class: String,
    carrier: Carrier,
    status: Status,
    source_energy_available_kwh: Option<f64>,
    requested_load_energy_kwh: Option<f64>,
    maximum_deliverable_power_kw: Option<f64>,
    required_source_power_kw: Option<f64>,
    required_source_energy_kwh: Option<f64>,
    source_power_headroom_kw: Option<f64>,
    source_power_shortfall_kw: Option<f64>,
    source_to_load_conversion_loss_kw: Option<f64>,
    source_to_load_conversion_loss_kwh: Option<f64>,
    delivery_temperature_at_load_c: Option<f64>,
    stages: Vec<StageResult>,
    findings: Vec<String>,
    claim_ceiling: &'static str,
}

#[derive(Debug, Serialize)]
struct Report {
    schema_version: String,
    method: &'static str,
    results: Vec<PathResult>,
    claim_ceiling: &'static str,
}

fn main() {
    let path = env::args().nth(1).unwrap_or_else(|| {
        "tools/rural-industrial-electrification-evaluator/fixtures/energy_paths_v0_1.json"
            .to_string()
    });

    let input = match fs::read_to_string(&path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("cannot read {path}: {error}");
            process::exit(2);
        }
    };

    let benchmark: Benchmark = match serde_json::from_str(&input) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("invalid energy-path JSON: {error}");
            process::exit(2);
        }
    };

    if benchmark.schema_version != "0.1.0" {
        eprintln!("unsupported schema_version: {}", benchmark.schema_version);
        process::exit(2);
    }

    let mut seen_ids = BTreeSet::new();
    for scenario in &benchmark.scenarios {
        if !seen_ids.insert(scenario.scenario_id.clone()) {
            eprintln!("duplicate scenario_id: {}", scenario.scenario_id);
            process::exit(2);
        }
    }

    let mut results: Vec<PathResult> = benchmark.scenarios.iter().map(evaluate).collect();
    results.sort_by(|a, b| a.scenario_id.cmp(&b.scenario_id));

    let report = Report {
        schema_version: benchmark.schema_version,
        method: "stage-by-stage source-to-load power and energy accounting",
        results,
        claim_ceiling: CLAIM_CEILING,
    };

    match serde_json::to_string_pretty(&report) {
        Ok(value) => println!("{value}"),
        Err(error) => {
            eprintln!("cannot serialize report: {error}");
            process::exit(2);
        }
    }
}

fn evaluate(path: &EnergyPath) -> PathResult {
    let mut findings = Vec::new();

    if let Some(error) = validate(path) {
        findings.push(error);
        return empty_result(path, Status::InvalidInput, findings, None);
    }

    let mut delivery_temperature_at_load_c = None;
    if path.carrier == Carrier::Thermal {
        match (
            path.source_temperature_c,
            path.required_delivery_temperature_c,
            path.minimum_approach_temperature_c,
        ) {
            (Some(source_temp), Some(required_temp), Some(approach)) => {
                let drops: Option<Vec<f64>> = path
                    .stages
                    .iter()
                    .map(|stage| stage.temperature_drop_c)
                    .collect();
                let Some(drops) = drops else {
                    findings.push(
                        "thermal_route_missing_one_or_more_stage_temperature_drops".to_string(),
                    );
                    return empty_result(path, Status::Indeterminate, findings, None);
                };
                let delivered_temp = source_temp - drops.iter().sum::<f64>();
                delivery_temperature_at_load_c = Some(delivered_temp);
                if delivered_temp - approach < required_temp {
                    findings.push("source_temperature_cannot_meet_required_delivery_temperature_after_declared_stage_drops_and_approach".to_string());
                    return empty_result(
                        path,
                        Status::NotFeasibleUnderDeclaredScreen,
                        findings,
                        Some(delivered_temp),
                    );
                }
            }
            _ => {
                findings.push(
                    "thermal_route_missing_source_delivery_or_approach_temperature".to_string(),
                );
                return empty_result(path, Status::Indeterminate, findings, None);
            }
        }
    }

    // Forward pass: maximum power that can traverse the declared chain from available source power.
    let mut available_power = path.source_available_kw;
    for stage in &path.stages {
        available_power = (available_power * stage.efficiency_fraction).min(stage.max_output_kw);
    }

    // Backward pass: source power required to deliver exactly the requested load.
    // This avoids claiming unused route capacity as delivered energy.
    let mut required_output_power = path.requested_load_kw;
    let mut stage_results_reversed = Vec::with_capacity(path.stages.len());
    for stage in path.stages.iter().rev() {
        let capacity_headroom = stage.max_output_kw - required_output_power;
        if capacity_headroom < -1e-9 {
            findings.push(format!(
                "stage_output_capacity_below_required_load:{}",
                stage.stage_id
            ));
        }
        let required_input_power = required_output_power / stage.efficiency_fraction;
        stage_results_reversed.push(StageResult {
            stage_id: stage.stage_id.clone(),
            required_input_power_kw: required_input_power,
            required_output_power_kw: required_output_power,
            conversion_loss_kw: required_input_power - required_output_power,
            capacity_headroom_kw: capacity_headroom,
        });
        required_output_power = required_input_power;
    }
    stage_results_reversed.reverse();

    let required_source_power = required_output_power;
    if required_source_power > path.source_available_kw + 1e-9 {
        findings.push("source_power_insufficient_for_requested_load".to_string());
    }

    let requested_energy = path.requested_load_kw * path.operation_hours;
    let required_source_energy = required_source_power * path.operation_hours;
    let source_energy_available = path.source_available_kw * path.operation_hours;
    let conversion_loss_power = (required_source_power - path.requested_load_kw).max(0.0);
    let conversion_loss_energy = conversion_loss_power * path.operation_hours;
    let source_headroom = (path.source_available_kw - required_source_power).max(0.0);
    let source_shortfall = (required_source_power - path.source_available_kw).max(0.0);

    let status = if findings.is_empty() && available_power + 1e-9 >= path.requested_load_kw {
        Status::FeasibleUnderDeclaredScreen
    } else {
        Status::NotFeasibleUnderDeclaredScreen
    };

    PathResult {
        scenario_id: path.scenario_id.clone(),
        label: path.label.clone(),
        evidence_class: path.evidence_class.clone(),
        carrier: path.carrier,
        status,
        source_energy_available_kwh: Some(source_energy_available),
        requested_load_energy_kwh: Some(requested_energy),
        maximum_deliverable_power_kw: Some(available_power),
        required_source_power_kw: Some(required_source_power),
        required_source_energy_kwh: Some(required_source_energy),
        source_power_headroom_kw: Some(source_headroom),
        source_power_shortfall_kw: Some(source_shortfall),
        source_to_load_conversion_loss_kw: Some(conversion_loss_power),
        source_to_load_conversion_loss_kwh: Some(conversion_loss_energy),
        delivery_temperature_at_load_c,
        stages: stage_results_reversed,
        findings,
        claim_ceiling: CLAIM_CEILING,
    }
}

fn validate(path: &EnergyPath) -> Option<String> {
    for (field, value) in [
        ("source_available_kw", path.source_available_kw),
        ("requested_load_kw", path.requested_load_kw),
        ("operation_hours", path.operation_hours),
    ] {
        if !value.is_finite() || value <= 0.0 {
            return Some(format!("{field}_must_be_finite_and_positive"));
        }
    }

    if path.scenario_id.trim().is_empty() || path.label.trim().is_empty() {
        return Some("scenario_id_and_label_must_be_nonempty".to_string());
    }

    if ![
        "synthetic",
        "measured",
        "manufacturer",
        "literature",
        "estimate",
    ]
    .contains(&path.evidence_class.as_str())
    {
        return Some("unsupported_evidence_class".to_string());
    }

    if path.stages.is_empty() {
        return Some("energy_path_must_contain_at_least_one_stage".to_string());
    }

    for stage in &path.stages {
        if stage.stage_id.trim().is_empty() {
            return Some("stage_id_must_be_nonempty".to_string());
        }
        if stage.carrier != path.carrier {
            return Some("stage_carrier_must_match_path_carrier_in_v0_1".to_string());
        }
        if !stage.efficiency_fraction.is_finite()
            || stage.efficiency_fraction <= 0.0
            || stage.efficiency_fraction > 1.0
        {
            return Some("stage_efficiency_must_be_in_(0,1]".to_string());
        }
        if !stage.max_output_kw.is_finite() || stage.max_output_kw <= 0.0 {
            return Some("stage_max_output_kw_must_be_finite_and_positive".to_string());
        }
        if let Some(drop) = stage.temperature_drop_c {
            if !drop.is_finite() || drop < 0.0 {
                return Some(
                    "stage_temperature_drop_c_must_be_finite_and_nonnegative_when_present"
                        .to_string(),
                );
            }
        }
    }

    for value in [
        path.source_temperature_c,
        path.required_delivery_temperature_c,
        path.minimum_approach_temperature_c,
    ]
    .into_iter()
    .flatten()
    {
        if !value.is_finite() {
            return Some("temperature_values_must_be_finite".to_string());
        }
    }
    if let Some(approach) = path.minimum_approach_temperature_c {
        if approach < 0.0 {
            return Some("minimum_approach_temperature_c_must_be_nonnegative".to_string());
        }
    }

    None
}

fn empty_result(
    path: &EnergyPath,
    status: Status,
    findings: Vec<String>,
    delivery_temperature_at_load_c: Option<f64>,
) -> PathResult {
    let source_energy_available = (path.source_available_kw.is_finite()
        && path.source_available_kw > 0.0
        && path.operation_hours.is_finite()
        && path.operation_hours > 0.0)
        .then_some(path.source_available_kw * path.operation_hours);
    let requested_energy = (path.requested_load_kw.is_finite()
        && path.requested_load_kw > 0.0
        && path.operation_hours.is_finite()
        && path.operation_hours > 0.0)
        .then_some(path.requested_load_kw * path.operation_hours);

    PathResult {
        scenario_id: path.scenario_id.clone(),
        label: path.label.clone(),
        evidence_class: path.evidence_class.clone(),
        carrier: path.carrier,
        status,
        source_energy_available_kwh: source_energy_available,
        requested_load_energy_kwh: requested_energy,
        maximum_deliverable_power_kw: None,
        required_source_power_kw: None,
        required_source_energy_kwh: None,
        source_power_headroom_kw: None,
        source_power_shortfall_kw: None,
        source_to_load_conversion_loss_kw: None,
        source_to_load_conversion_loss_kwh: None,
        delivery_temperature_at_load_c,
        stages: Vec::new(),
        findings,
        claim_ceiling: CLAIM_CEILING,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn electrical_path(id: &str) -> EnergyPath {
        EnergyPath {
            scenario_id: id.into(),
            label: "synthetic electrical route".into(),
            evidence_class: "synthetic".into(),
            carrier: Carrier::Electrical,
            source_available_kw: 1_000.0,
            requested_load_kw: 850.0,
            operation_hours: 2.0,
            source_temperature_c: None,
            required_delivery_temperature_c: None,
            minimum_approach_temperature_c: None,
            stages: vec![
                Stage {
                    stage_id: "feeder".into(),
                    carrier: Carrier::Electrical,
                    efficiency_fraction: 0.98,
                    max_output_kw: 950.0,
                    temperature_drop_c: None,
                },
                Stage {
                    stage_id: "converter".into(),
                    carrier: Carrier::Electrical,
                    efficiency_fraction: 0.95,
                    max_output_kw: 950.0,
                    temperature_drop_c: None,
                },
            ],
        }
    }

    #[test]
    fn stage_losses_match_backward_hand_oracle() {
        let result = evaluate(&electrical_path("oracle"));
        let required = 850.0 / (0.98 * 0.95);
        assert_eq!(result.status, Status::FeasibleUnderDeclaredScreen);
        assert!((result.required_source_power_kw.unwrap() - required).abs() < 1e-9);
        assert!((result.maximum_deliverable_power_kw.unwrap() - 902.5).abs() < 1e-9);
        assert!(
            (result.source_to_load_conversion_loss_kw.unwrap() - (required - 850.0)).abs() < 1e-9
        );
        assert!((result.requested_load_energy_kwh.unwrap() - 1_700.0).abs() < 1e-9);
        assert!(
            result
                .stages
                .iter()
                .all(|stage| stage.capacity_headroom_kw >= 0.0)
        );
    }

    #[test]
    fn receiver_overload_is_not_accepted() {
        let mut path = electrical_path("overload");
        path.requested_load_kw = 950.0;
        let result = evaluate(&path);
        assert_eq!(result.status, Status::NotFeasibleUnderDeclaredScreen);
    }

    #[test]
    fn source_power_shortfall_is_not_accepted() {
        let mut path = electrical_path("source_shortfall");
        path.source_available_kw = 800.0;
        let result = evaluate(&path);
        assert_eq!(result.status, Status::NotFeasibleUnderDeclaredScreen);
        assert!(
            result
                .findings
                .iter()
                .any(|x| x == "source_power_insufficient_for_requested_load")
        );
        assert!(result.source_power_shortfall_kw.unwrap() > 0.0);
    }

    #[test]
    fn thermal_temperature_mismatch_fails_closed() {
        let mut path = electrical_path("heat");
        path.carrier = Carrier::Thermal;
        path.source_temperature_c = Some(300.0);
        path.required_delivery_temperature_c = Some(320.0);
        path.minimum_approach_temperature_c = Some(15.0);
        for stage in &mut path.stages {
            stage.carrier = Carrier::Thermal;
            stage.temperature_drop_c = Some(0.0);
        }
        let result = evaluate(&path);
        assert_eq!(result.status, Status::NotFeasibleUnderDeclaredScreen);
        assert!(result.findings[0].contains("source_temperature"));
    }

    #[test]
    fn missing_thermal_grade_is_indeterminate() {
        let mut path = electrical_path("unknown_heat");
        path.carrier = Carrier::Thermal;
        for stage in &mut path.stages {
            stage.carrier = Carrier::Thermal;
        }
        let result = evaluate(&path);
        assert_eq!(result.status, Status::Indeterminate);
        assert!(result.maximum_deliverable_power_kw.is_none());
    }

    #[test]
    fn mixed_carrier_stage_is_invalid_in_v0_1() {
        let mut path = electrical_path("mixed");
        path.stages[0].carrier = Carrier::Thermal;
        assert_eq!(evaluate(&path).status, Status::InvalidInput);
    }

    #[test]
    fn unknown_evidence_class_is_invalid() {
        let mut path = electrical_path("bad_evidence");
        path.evidence_class = "handwave".into();
        assert_eq!(evaluate(&path).status, Status::InvalidInput);
    }

    #[test]
    fn zero_stage_capacity_is_invalid() {
        let mut path = electrical_path("zero_cap");
        path.stages[0].max_output_kw = 0.0;
        assert_eq!(evaluate(&path).status, Status::InvalidInput);
    }

    #[test]
    fn input_order_is_canonicalized() {
        let a = vec![electrical_path("z"), electrical_path("a")];
        let b = vec![electrical_path("a"), electrical_path("z")];
        let mut ar: Vec<PathResult> = a.iter().map(evaluate).collect();
        let mut br: Vec<PathResult> = b.iter().map(evaluate).collect();
        ar.sort_by(|x, y| x.scenario_id.cmp(&y.scenario_id));
        br.sort_by(|x, y| x.scenario_id.cmp(&y.scenario_id));
        let aa = serde_json::to_string(&ar).unwrap();
        let bb = serde_json::to_string(&br).unwrap();
        assert_eq!(aa, bb);
    }
}
