use serde::{Deserialize, Serialize};
use std::collections::HashSet;

const FLOAT_TOLERANCE: f64 = 1.0e-8;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema_version: String,
    pub benchmark_id: String,
    pub frozen_at: String,
    pub evidence_class: String,
    pub physical_measurements: bool,
    pub plasma_solver_outputs: bool,
    pub service_unit_policy: String,
    pub scaleout_reference_scenario_id: String,
    pub research_targets: ResearchTargets,
    pub scenarios: Vec<Scenario>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResearchTargets {
    pub minimum_scaleout_efficiency: f64,
    pub maximum_active_cell_cv: f64,
    pub maximum_neighbor_cross_talk_fraction: f64,
    pub minimum_faulted_service_floor: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scenario {
    pub id: String,
    pub evidence_class: String,
    pub configuration_digest: String,
    pub reference_configuration_digest: String,
    pub service: Service,
    pub baseline: Baseline,
    pub cells: Vec<Cell>,
    pub energy: Energy,
    pub gas: Gas,
    pub cross_talk: CrossTalk,
    pub attempts: Attempts,
    pub expected: Expected,
    pub expected_metrics: Option<ExpectedMetrics>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExpectedMetrics {
    pub scaleout_efficiency: Option<serde_json::Value>,
    pub active_cell_cv: Option<serde_json::Value>,
    pub service_retention: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Service {
    pub unit: String,
    pub reference_unit: String,
    pub quality_gate_id: String,
    pub accepted_total: f64,
    pub minimum_total: Option<f64>,
    pub healthy_reference_total: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Baseline {
    pub scenario_id: String,
    pub service_per_cell: f64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cell {
    pub id: String,
    pub state: String,
    pub observed_state: String,
    pub accepted_service: f64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Energy {
    pub grid_energy_wh: f64,
    pub meter_ids: Vec<String>,
    pub boundary_complete: bool,
    pub includes_supply_losses: bool,
    pub includes_gas_supply: bool,
    pub includes_cooling_pumps_controls_exhaust: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Gas {
    pub consumption_normal_l: f64,
    pub unit: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CrossTalk {
    pub max_allowed_fraction: f64,
    pub coverage_status: String,
    pub sampling_plan_id: Option<String>,
    pub required_pair_ids: Vec<String>,
    pub observed_pair_ids: Vec<String>,
    pub edges: Vec<CrossTalkEdge>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CrossTalkEdge {
    pub pair_id: String,
    pub from: String,
    pub to: String,
    pub intervention: String,
    pub neighbor_output_delta_fraction: f64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Attempts {
    pub attempted: u64,
    pub succeeded: u64,
    pub failed: u64,
    pub failed_included_in_denominator: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Expected {
    pub disposition: String,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScenarioResult {
    pub id: String,
    pub disposition: String,
    pub expected_disposition: String,
    pub reasons: Vec<String>,
    pub scaleout_efficiency: Option<f64>,
    pub active_cell_cv: Option<f64>,
    pub grid_energy_per_service: Option<f64>,
    pub faulted_service_retention: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AuditReport {
    pub schema_version: String,
    pub benchmark_id: String,
    pub scenario_count: usize,
    pub matched_expectations: usize,
    pub mismatched_expectations: usize,
    pub results: Vec<ScenarioResult>,
}

fn close_enough(a: f64, b: f64) -> bool {
    (a - b).abs() <= FLOAT_TOLERANCE * a.abs().max(b.abs()).max(1.0)
}

fn sample_cv(values: &[f64]) -> Option<f64> {
    if values.len() < 2 {
        return None;
    }
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    if !mean.is_finite() || mean <= 0.0 {
        return None;
    }
    let squared_deviation = values
        .iter()
        .map(|value| {
            let delta = value - mean;
            delta * delta
        })
        .sum::<f64>();
    let sample_variance = squared_deviation / (values.len() - 1) as f64;
    let cv = sample_variance.sqrt() / mean;
    cv.is_finite().then_some(cv)
}

fn evaluate_scenario(
    scenario: &Scenario,
    manifest: &Manifest,
    reference_scenario_id: &str,
    reference_service_per_cell: f64,
    reference_service_unit: &str,
    reference_quality_gate_id: &str,
) -> ScenarioResult {
    let mut rejected = Vec::new();
    let mut blocked = Vec::new();
    let mut scaleout_efficiency = None;
    let mut active_cell_cv = None;
    let mut grid_energy_per_service = None;
    let mut faulted_service_retention = None;

    let reject = |reason: &str, reasons: &mut Vec<String>| reasons.push(reason.to_owned());
    let block = |reason: &str, reasons: &mut Vec<String>| reasons.push(reason.to_owned());

    if scenario.evidence_class != "synthetic" {
        reject("scenario evidence class is not synthetic", &mut rejected);
    }
    if scenario.configuration_digest.is_empty()
        || scenario.configuration_digest != scenario.reference_configuration_digest
    {
        reject("configuration identity is missing or stale", &mut rejected);
    }
    if scenario.baseline.scenario_id != reference_scenario_id {
        reject("baseline scenario identity does not match the manifest reference", &mut rejected);
    }
    if !scenario.baseline.service_per_cell.is_finite()
        || scenario.baseline.service_per_cell <= 0.0
        || !close_enough(scenario.baseline.service_per_cell, reference_service_per_cell)
    {
        reject("declared one-cell baseline does not match the frozen reference scenario", &mut rejected);
    }
    if scenario.service.unit.is_empty()
        || scenario.service.unit != scenario.service.reference_unit
        || scenario.service.unit != reference_service_unit
        || scenario.service.quality_gate_id.is_empty()
        || scenario.service.quality_gate_id != reference_quality_gate_id
    {
        reject("service unit or quality-gate identity is missing or mismatched", &mut rejected);
    }
    if !scenario.service.accepted_total.is_finite() || scenario.service.accepted_total < 0.0 {
        reject("accepted service total is non-finite or negative", &mut rejected);
    }
    if scenario.cells.is_empty() {
        reject("scenario has no cells", &mut rejected);
    }

    let mut cell_ids = HashSet::new();
    let mut active_outputs = Vec::new();
    let mut failed_count = 0usize;
    let mut observed_sum = 0.0;
    for cell in &scenario.cells {
        if cell.id.is_empty() || !cell_ids.insert(cell.id.as_str()) {
            reject("cell identity is empty or duplicated", &mut rejected);
        }
        if !cell.accepted_service.is_finite() || cell.accepted_service < 0.0 {
            reject("cell service is non-finite or negative", &mut rejected);
            continue;
        }
        observed_sum += cell.accepted_service;
        match (cell.state.as_str(), cell.observed_state.as_str()) {
            ("active", "active") => active_outputs.push(cell.accepted_service),
            ("failed", "isolated") => {
                failed_count += 1;
                if cell.accepted_service > FLOAT_TOLERANCE {
                    reject("isolated failed cell reports positive accepted service", &mut rejected);
                }
            }
            ("failed", "active") => {
                failed_count += 1;
                reject("failed cell is still observed as active", &mut rejected);
            }
            _ => reject("cell state or observed state is unsupported", &mut rejected),
        }
    }

    if !close_enough(observed_sum, scenario.service.accepted_total) {
        reject("per-cell service does not reconcile to accepted service total", &mut rejected);
    }
    if active_outputs.is_empty() {
        reject("scenario has no active cells", &mut rejected);
    } else if active_outputs.len() < 2 && scenario.cells.len() > 1 {
        block("fewer than two active cells remain for an array uniformity estimate", &mut blocked);
    } else {
        let active_mean = active_outputs.iter().sum::<f64>() / active_outputs.len() as f64;
        if !active_mean.is_finite() || active_mean <= 0.0 {
            reject("active-cell mean service is zero or invalid", &mut rejected);
        } else {
            active_cell_cv = sample_cv(&active_outputs);
            if let Some(cv) = active_cell_cv {
                if cv > manifest.research_targets.maximum_active_cell_cv {
                    reject("active-cell output CV exceeds the research target", &mut rejected);
                }
            }
        }
    }

    let cell_count = scenario.cells.len() as f64;
    let scaleout_denominator = cell_count * reference_service_per_cell;
    if scaleout_denominator.is_finite()
        && scaleout_denominator > 0.0
        && scenario.service.accepted_total.is_finite()
    {
        scaleout_efficiency = Some(scenario.service.accepted_total / scaleout_denominator);
        if let Some(value) = scaleout_efficiency {
            if value < manifest.research_targets.minimum_scaleout_efficiency {
                reject("scale-out efficiency is below the research target", &mut rejected);
            }
        }
    }

    if scenario.energy.meter_ids.is_empty() {
        block("system energy boundary has no meter identities", &mut blocked);
    } else {
        let mut meter_ids = HashSet::new();
        if scenario.energy.meter_ids.iter().any(|id| id.is_empty() || !meter_ids.insert(id)) {
            block("system energy meter identity is empty or duplicated", &mut blocked);
        }
    }
    if !scenario.energy.boundary_complete
        || !scenario.energy.includes_supply_losses
        || !scenario.energy.includes_gas_supply
        || !scenario.energy.includes_cooling_pumps_controls_exhaust
    {
        block("complete grid-to-service energy boundary is not accounted", &mut blocked);
    }

    if !scenario.gas.consumption_normal_l.is_finite()
        || scenario.gas.consumption_normal_l < 0.0
        || scenario.gas.unit.is_empty()
    {
        block("gas resource accounting is missing or invalid", &mut blocked);
    }

    if scenario.attempts.attempted == 0
        || scenario.attempts.succeeded.checked_add(scenario.attempts.failed)
            != Some(scenario.attempts.attempted)
        || !scenario.attempts.failed_included_in_denominator
    {
        block("attempt, success and failure denominators do not reconcile", &mut blocked);
    }

    if !scenario.cross_talk.max_allowed_fraction.is_finite()
        || scenario.cross_talk.max_allowed_fraction < 0.0
    {
        reject("cross-talk limit is invalid", &mut rejected);
    } else if scenario.cells.len() == 1 {
        if scenario.cross_talk.coverage_status != "NOT_APPLICABLE"
            || scenario.cross_talk.sampling_plan_id.is_some()
            || !scenario.cross_talk.required_pair_ids.is_empty()
            || !scenario.cross_talk.observed_pair_ids.is_empty()
            || !scenario.cross_talk.edges.is_empty()
        {
            block("single-cell cross-talk coverage must be explicitly NOT_APPLICABLE", &mut blocked);
        }
    } else {
        let required_pairs: HashSet<_> =
            scenario.cross_talk.required_pair_ids.iter().map(String::as_str).collect();
        let observed_pairs: HashSet<_> =
            scenario.cross_talk.observed_pair_ids.iter().map(String::as_str).collect();
        if required_pairs.len() != scenario.cross_talk.required_pair_ids.len()
            || observed_pairs.len() != scenario.cross_talk.observed_pair_ids.len()
        {
            reject("cross-talk pair identities are duplicated", &mut rejected);
        }
        if scenario.cross_talk.coverage_status != "COMPLETE" {
            block("multi-cell cross-talk is NOT_MEASURED or incomplete; it cannot be treated as zero", &mut blocked);
        } else if scenario.cross_talk.sampling_plan_id.as_deref().is_none_or(str::is_empty)
            || required_pairs.is_empty()
            || required_pairs != observed_pairs
        {
            block("cross-talk sampling plan does not fully cover its declared neighbor-pair set", &mut blocked);
        }
        let declared_limit = scenario
            .cross_talk
            .max_allowed_fraction
            .min(manifest.research_targets.maximum_neighbor_cross_talk_fraction);
        let mut edge_pairs = HashSet::new();
        for edge in &scenario.cross_talk.edges {
            if edge.pair_id.trim().is_empty()
                || !edge_pairs.insert(edge.pair_id.as_str())
                || !required_pairs.contains(edge.pair_id.as_str())
                || edge.from.is_empty()
                || edge.to.is_empty()
                || !cell_ids.contains(edge.from.as_str())
                || !cell_ids.contains(edge.to.as_str())
                || edge.from == edge.to
                || edge.intervention.is_empty()
                || !edge.neighbor_output_delta_fraction.is_finite()
            {
                reject("cross-talk edge identity, pair coverage or signed delta is invalid", &mut rejected);
            } else if edge.neighbor_output_delta_fraction.abs() > declared_limit {
                reject("neighbor cross-talk magnitude exceeds the declared limit", &mut rejected);
            }
        }
        if scenario.cross_talk.coverage_status == "COMPLETE"
            && edge_pairs != required_pairs
        {
            block("cross-talk edge records do not match the complete declared pair set", &mut blocked);
        }
    }


    let energy_boundary_complete = scenario.energy.boundary_complete
        && !scenario.energy.meter_ids.is_empty()
        && scenario.energy.includes_supply_losses
        && scenario.energy.includes_gas_supply
        && scenario.energy.includes_cooling_pumps_controls_exhaust
        && scenario.gas.consumption_normal_l.is_finite()
        && scenario.gas.consumption_normal_l >= 0.0
        && !scenario.gas.unit.is_empty()
        && {
            let mut ids = HashSet::new();
            scenario.energy.meter_ids.iter().all(|id| !id.is_empty() && ids.insert(id))
        };
    if energy_boundary_complete
        && scenario.energy.grid_energy_wh.is_finite()
        && scenario.energy.grid_energy_wh > 0.0
        && scenario.service.accepted_total.is_finite()
        && scenario.service.accepted_total > 0.0
    {
        grid_energy_per_service =
            Some(scenario.energy.grid_energy_wh / scenario.service.accepted_total);
    } else if !scenario.energy.grid_energy_wh.is_finite() || scenario.energy.grid_energy_wh <= 0.0 {
        reject("grid energy must be finite and positive", &mut rejected);
    } else {
        grid_energy_per_service = None;
    }

    if failed_count > 0 {
        match (
            scenario.service.minimum_total,
            scenario.service.healthy_reference_total,
        ) {
            (Some(minimum), Some(healthy))
                if minimum.is_finite() && minimum >= 0.0 && healthy.is_finite() && healthy > 0.0 =>
            {
                if scenario.service.accepted_total < minimum {
                    reject("faulted array falls below the declared service floor", &mut rejected);
                }
                faulted_service_retention = Some(scenario.service.accepted_total / healthy);
            }
            _ => block("failed-cell scenario lacks a valid service floor/reference", &mut blocked),
        }
    }

    if let Some(expected) = &scenario.expected_metrics {
        match expected.scaleout_efficiency.as_ref() {
            Some(value) => match value.as_f64() {
                Some(expected_value) if scaleout_efficiency.is_some_and(|actual| close_enough(actual, expected_value)) => {}
                _ => reject("scale-out efficiency does not match the frozen independent expected metric", &mut rejected),
            },
            None if scaleout_efficiency.is_none() => {}
            None => reject("frozen expected metrics omit a computed scale-out efficiency", &mut rejected),
        }

        match expected.active_cell_cv.as_ref() {
            None if active_cell_cv.is_none() => {}
            None => reject("expected null active-cell CV but evaluator computed a value", &mut rejected),
            Some(value) => {
                if let Some(expected_value) = value.as_f64() {
                    if !active_cell_cv.is_some_and(|actual| close_enough(actual, expected_value)) {
                        reject("active-cell CV does not match the frozen independent expected metric", &mut rejected);
                    }
                } else if value.as_str() == Some("computed_from_cells") {
                    if active_cell_cv.is_none() {
                        reject("active-cell CV was expected to be computed from cells but is undefined", &mut rejected);
                    }
                } else {
                    reject("unsupported expected active-cell CV representation", &mut rejected);
                }
            }
        }

        if let Some(expected_retention) = expected.service_retention {
            if !faulted_service_retention
                .is_some_and(|actual| close_enough(actual, expected_retention))
            {
                reject("faulted service retention does not match the frozen expected metric", &mut rejected);
            }
        }
    }


    let (disposition, reasons) = if !rejected.is_empty() {
        ("REJECTED".to_owned(), rejected)
    } else if !blocked.is_empty() {
        ("BLOCKED".to_owned(), blocked)
    } else {
        ("SYNTHETIC_PASS".to_owned(), Vec::new())
    };

    ScenarioResult {
        id: scenario.id.clone(),
        disposition,
        expected_disposition: scenario.expected.disposition.clone(),
        reasons,
        scaleout_efficiency,
        active_cell_cv,
        grid_energy_per_service,
        faulted_service_retention,
    }
}

pub fn audit_manifest_json(input: &str) -> Result<AuditReport, String> {
    let manifest: Manifest =
        serde_json::from_str(input).map_err(|error| format!("invalid manifest JSON: {error}"))?;

    if manifest.schema_version != "luminous.plasma-array-scaleout.v0.1" {
        return Err("unsupported schema_version".to_owned());
    }
    if manifest.benchmark_id != "PLASMA-ARRAY-SCALEOUT-001" {
        return Err("unexpected benchmark_id".to_owned());
    }
    if manifest.evidence_class != "synthetic"
        || manifest.physical_measurements
        || manifest.plasma_solver_outputs
    {
        return Err("manifest must remain synthetic with no physical or plasma-solver evidence".to_owned());
    }
    if manifest.frozen_at.trim().is_empty()
        || manifest.service_unit_policy.trim().is_empty()
        || manifest.research_targets.minimum_faulted_service_floor != "scenario-specific"
    {
        return Err("manifest metadata or service-floor policy is invalid".to_owned());
    }
    if manifest.scenarios.is_empty() {
        return Err("manifest contains no scenarios".to_owned());
    }

    let mut scenario_ids = HashSet::new();
    for scenario in &manifest.scenarios {
        if scenario.id.is_empty() || !scenario_ids.insert(scenario.id.as_str()) {
            return Err("scenario identity is empty or duplicated".to_owned());
        }
    }
    if !scenario_ids.contains(manifest.scaleout_reference_scenario_id.as_str()) {
        return Err("scale-out reference scenario is missing".to_owned());
    }

    if !manifest.research_targets.minimum_scaleout_efficiency.is_finite()
        || manifest.research_targets.minimum_scaleout_efficiency < 0.0
        || !manifest.research_targets.maximum_active_cell_cv.is_finite()
        || manifest.research_targets.maximum_active_cell_cv < 0.0
        || !manifest.research_targets.maximum_neighbor_cross_talk_fraction.is_finite()
        || manifest.research_targets.maximum_neighbor_cross_talk_fraction < 0.0
    {
        return Err("research target bounds are invalid".to_owned());
    }

    let reference = manifest
        .scenarios
        .iter()
        .find(|scenario| scenario.id == manifest.scaleout_reference_scenario_id)
        .expect("reference identity checked above");
    if reference.cells.len() != 1
        || reference.cells[0].state != "active"
        || reference.cells[0].observed_state != "active"
        || !reference.cells[0].accepted_service.is_finite()
        || reference.cells[0].accepted_service <= 0.0
        || reference.service.unit != reference.service.reference_unit
        || reference.service.quality_gate_id.is_empty()
        || !close_enough(reference.cells[0].accepted_service, reference.service.accepted_total)
    {
        return Err("reference scenario must contain one valid active cell with reconciled service".to_owned());
    }
    let reference_service_per_cell = reference.cells[0].accepted_service;
    let reference_result = evaluate_scenario(
        reference,
        &manifest,
        &manifest.scaleout_reference_scenario_id,
        reference_service_per_cell,
        &reference.service.unit,
        &reference.service.quality_gate_id,
    );
    if reference_result.disposition != "SYNTHETIC_PASS" {
        return Err(format!(
            "one-cell reference scenario is not independently qualified by the evaluator: {}",
            reference_result.reasons.join("; ")
        ));
    }

    let results: Vec<ScenarioResult> = manifest
        .scenarios
        .iter()
        .map(|scenario| {
            evaluate_scenario(
                scenario,
                &manifest,
                &manifest.scaleout_reference_scenario_id,
                reference_service_per_cell,
                &reference.service.unit,
                &reference.service.quality_gate_id,
            )
        })
        .collect();
    let matched_expectations = results
        .iter()
        .filter(|result| result.disposition == result.expected_disposition)
        .count();
    let scenario_count = results.len();

    Ok(AuditReport {
        schema_version: manifest.schema_version,
        benchmark_id: manifest.benchmark_id,
        scenario_count,
        matched_expectations,
        mismatched_expectations: scenario_count - matched_expectations,
        results,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str =
        include_str!("../../../../benchmarks/plasma/plasma-array-scaleout-v0.json");

    fn fixture_value() -> serde_json::Value {
        serde_json::from_str(FIXTURE).expect("frozen fixture must be valid JSON")
    }

    fn run(value: &serde_json::Value) -> AuditReport {
        audit_manifest_json(&value.to_string()).expect("mutated fixture remains valid JSON")
    }

    fn set_scenario_field(
        value: &mut serde_json::Value,
        scenario_id: &str,
        field: &str,
        replacement: serde_json::Value,
    ) {
        let scenarios = value["scenarios"].as_array_mut().expect("scenarios array");
        let scenario = scenarios
            .iter_mut()
            .find(|scenario| scenario["id"].as_str() == Some(scenario_id))
            .expect("scenario exists");
        scenario[field] = replacement;
    }

    #[test]
    fn frozen_scenarios_match_expected_dispositions() {
        let report = audit_manifest_json(FIXTURE).expect("fixture parses");
        assert_eq!(report.scenario_count, 12);
        assert_eq!(report.mismatched_expectations, 0);
        assert_eq!(report.matched_expectations, 12);
    }

    #[test]
    fn single_cell_reference_has_undefined_cv() {
        let report = audit_manifest_json(FIXTURE).expect("fixture parses");
        let reference = report
            .results
            .iter()
            .find(|result| result.id == "scale-n1-reference")
            .expect("reference result");
        assert_eq!(reference.active_cell_cv, None);
    }

    #[test]
    fn stale_configuration_is_rejected() {
        let mut value = fixture_value();
        let mut config = value["scenarios"][1]["configuration_digest"]
            .as_str()
            .expect("digest")
            .to_owned();
        config.push_str("-stale");
        set_scenario_field(
            &mut value,
            "scale-n5-linear",
            "configuration_digest",
            serde_json::Value::String(config),
        );
        assert_eq!(
            run(&value).results[1].disposition,
            "REJECTED"
        );
    }


    #[test]
    fn candidate_cannot_change_its_one_cell_reference() {
        let mut value = fixture_value();
        value["scenarios"][1]["baseline"]["service_per_cell"] = serde_json::json!(0.1);
        assert_eq!(run(&value).results[1].disposition, "REJECTED");
    }

    #[test]
    fn duplicate_cell_identity_is_rejected() {
        let mut value = fixture_value();
        let first_id = value["scenarios"][1]["cells"][0]["id"].clone();
        value["scenarios"][1]["cells"][1]["id"] = first_id;
        assert_eq!(run(&value).results[1].disposition, "REJECTED");
    }


    #[test]
    fn candidate_must_match_reference_service_and_quality_gate() {
        let mut value = fixture_value();
        value["scenarios"][1]["service"]["quality_gate_id"] =
            serde_json::json!("different-quality-gate");
        assert_eq!(run(&value).results[1].disposition, "REJECTED");
    }

    #[test]
    fn service_total_must_reconcile_with_cells() {
        let mut value = fixture_value();
        value["scenarios"][1]["service"]["accepted_total"] = serde_json::json!(99.0);
        assert_eq!(run(&value).results[1].disposition, "REJECTED");
    }

    #[test]
    fn cross_talk_above_limit_is_rejected() {
        let mut value = fixture_value();
        value["scenarios"][5]["cross_talk"]["edges"][0]["neighbor_output_delta_fraction"] =
            serde_json::json!(0.08);
        assert_eq!(run(&value).results[5].disposition, "REJECTED");
    }

    #[test]
    fn incomplete_energy_boundary_is_blocked() {
        let report = audit_manifest_json(FIXTURE).expect("fixture parses");
        assert_eq!(report.results[7].disposition, "BLOCKED");
    }

    #[test]
    fn stale_telemetry_is_rejected() {
        let report = audit_manifest_json(FIXTURE).expect("fixture parses");
        assert_eq!(report.results[6].disposition, "REJECTED");
    }

    #[test]
    fn failed_attempt_denominator_is_blocked() {
        let report = audit_manifest_json(FIXTURE).expect("fixture parses");
        assert_eq!(report.results[11].disposition, "BLOCKED");
    }

    #[test]
    fn isolated_failed_cell_preserves_service_floor() {
        let report = audit_manifest_json(FIXTURE).expect("fixture parses");
        let result = &report.results[4];
        assert_eq!(result.disposition, "SYNTHETIC_PASS");
        assert!(result.faulted_service_retention.unwrap() >= 0.90);
    }

    #[test]
    fn unmeasured_multi_cell_cross_talk_blocks_otherwise_passing_array() {
        let report = audit_manifest_json(FIXTURE).expect("fixture parses");
        assert_eq!(report.results[1].disposition, "BLOCKED");
        assert_eq!(report.results[4].disposition, "BLOCKED");
    }

    #[test]
    fn single_cell_cross_talk_is_explicitly_not_applicable() {
        let report = audit_manifest_json(FIXTURE).expect("fixture parses");
        assert_eq!(report.results[0].disposition, "SYNTHETIC_PASS");
    }

    #[test]
    fn expected_scaleout_efficiency_is_checked_for_n5_and_n100() {
        let report = audit_manifest_json(FIXTURE).expect("fixture parses");
        assert!(close_enough(report.results[1].scaleout_efficiency.unwrap(), 0.96));
        assert!(close_enough(report.results[4].scaleout_efficiency.unwrap(), 0.9405));
        assert_eq!(report.results[1].disposition, "BLOCKED");
    }

    #[test]
    fn incomplete_energy_boundary_has_no_scoreable_energy_intensity() {
        let report = audit_manifest_json(FIXTURE).expect("fixture parses");
        assert_eq!(report.results[7].disposition, "BLOCKED");
        assert_eq!(report.results[7].grid_energy_per_service, None);
    }

    #[test]
    fn negative_neighbor_output_delta_is_checked_by_magnitude() {
        let mut value = fixture_value();
        value["scenarios"][5]["cross_talk"]["edges"][0]["neighbor_output_delta_fraction"] =
            serde_json::json!(-0.12);
        assert_eq!(run(&value).results[5].disposition, "REJECTED");
    }

    #[test]
    fn reference_scenario_must_pass_its_complete_boundary() {
        let mut value = fixture_value();
        value["scenarios"][0]["energy"]["boundary_complete"] = serde_json::json!(false);
        let error = audit_manifest_json(&value.to_string()).unwrap_err();
        assert!(error.contains("one-cell reference scenario"));
    }

    #[test]
    fn unknown_scenario_fields_are_rejected() {
        let mut value = fixture_value();
        value["scenarios"][1]["unrecognized_typo_field"] = serde_json::json!(true);
        assert!(audit_manifest_json(&value.to_string()).is_err());
    }

    #[test]
    fn independent_sample_cv_oracle_detects_synthetic_outliers() {
        let values = [0.9, 0.9, 0.9, 0.9, 1.4];
        let expected_cv = {
            let mean = values.iter().sum::<f64>() / values.len() as f64;
            let variance = values
                .iter()
                .map(|value| {
                    let delta = *value - mean;
                    delta * delta
                })
                .sum::<f64>()
                / (values.len() - 1) as f64;
            variance.sqrt() / mean
        };
        assert!(expected_cv > 0.05);
        assert!((sample_cv(&values).unwrap() - expected_cv).abs() < 1.0e-12);
    }
}
