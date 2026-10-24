//! Digest-bound deterministic robustness report for the frozen candidate × scenario matrix.
//!
//! This derives per-scenario axis-wise attainable bounds and worst-case-of-best
//! envelopes. These coordinate-wise values are not necessarily jointly attainable
//! from a single schedule. The full per-scenario frontiers are retained to make
//! that limitation inspectable and to support later search-regret analysis.
use std::{
    collections::{BTreeMap, BTreeSet},
    env,
    error::Error,
    fs,
    io,
};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use symtropy_energy_dispatch_audit::DispatchObjective;

const MATRIX_SCHEMA: &str = "energy-candidate-scenario-matrix-report-v0.2";
const OUTPUT_SCHEMA: &str = "energy-candidate-robustness-report-v0.1";
const MATRIX_ID: &str = "energy-candidate-scenario-matrix-v0.1";
const EXPECTED_CANDIDATES: usize = 39;
const EXPECTED_SCENARIOS: usize = 5;
const EXPECTED_ROWS: usize = EXPECTED_CANDIDATES * EXPECTED_SCENARIOS;

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn invalid(message: impl Into<String>) -> Box<dyn Error> {
    io::Error::new(io::ErrorKind::InvalidData, message.into()).into()
}

fn required_str<'a>(value: &'a Value, key: &str) -> Result<&'a str, Box<dyn Error>> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| invalid(format!("missing string field {key}")))
}

fn required_u64(value: &Value, key: &str) -> Result<u64, Box<dyn Error>> {
    value
        .get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| invalid(format!("missing unsigned field {key}")))
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn expected_candidate_ids() -> Vec<String> {
    let mut ids = vec![
        "synthetic-grid-only".to_owned(),
        "synthetic-pv-grid".to_owned(),
        "synthetic-pv-battery-grid".to_owned(),
    ];
    for pv in [1, 2, 3] {
        for capacity in [1, 2, 3] {
            for battery_power in [1, 2] {
                for grid_power in [1, 2] {
                    ids.push(format!(
                        "synthetic-parametric-pv-{pv}kw-battery-{capacity}kwh-battery-power-{battery_power}kw-grid-{grid_power}kw"
                    ));
                }
            }
        }
    }
    ids.sort();
    ids
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct AxisBest {
    essential_unserved_quanta: u64,
    flexible_unserved_quanta: u64,
    operating_cost_units: u64,
    grid_carbon_grams: u64,
    curtailed_pv_quanta: u64,
    terminal_soc_quanta: u16,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct ScenarioProfile {
    scenario_id: String,
    frontier_count: usize,
    frontier_objectives: Vec<DispatchObjective>,
    axis_best: AxisBest,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct CandidateProfile {
    candidate_id: String,
    scenario_profiles: Vec<ScenarioProfile>,
    /// Worst scenario's best attainable value computed independently per axis.
    /// Coordinates may originate from different frontier points and are not a schedule.
    worst_case_of_per_scenario_best_axis_value: AxisBest,
    envelope_dominates: Vec<String>,
    envelope_dominated_by: Vec<String>,
    envelope_equal_to: Vec<String>,
    envelope_incomparable_with: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
struct RobustnessReport {
    schema_version: &'static str,
    status: &'static str,
    matrix_id: String,
    source_matrix_sha256: String,
    source_scenario_profile_sha256: String,
    source_design_space_manifest_sha256: String,
    candidate_count: usize,
    scenario_count: usize,
    expected_row_count: usize,
    observed_row_count: usize,
    passed_row_count: usize,
    failed_or_unknown_row_count: usize,
    deterministic_scenario_ids: Vec<String>,
    candidates: Vec<CandidateProfile>,
    claim_ceiling: &'static str,
}

fn axis_best(frontier: &[DispatchObjective]) -> Result<AxisBest, Box<dyn Error>> {
    if frontier.is_empty() {
        return Err(invalid("a qualified scenario frontier must not be empty"));
    }
    Ok(AxisBest {
        essential_unserved_quanta: frontier.iter()
            .map(|item| item.essential_unserved_quanta).min().unwrap(),
        flexible_unserved_quanta: frontier.iter()
            .map(|item| item.flexible_unserved_quanta).min().unwrap(),
        operating_cost_units: frontier.iter()
            .map(|item| item.operating_cost_units).min().unwrap(),
        grid_carbon_grams: frontier.iter()
            .map(|item| item.grid_carbon_grams).min().unwrap(),
        curtailed_pv_quanta: frontier.iter()
            .map(|item| item.curtailed_pv_quanta).min().unwrap(),
        terminal_soc_quanta: frontier.iter()
            .map(|item| item.terminal_soc_quanta).max().unwrap(),
    })
}

/// This is envelope dominance, not proof that one joint dispatch schedule
/// dominates another in every scenario.
fn dominates(left: &AxisBest, right: &AxisBest) -> bool {
    let no_worse = left.essential_unserved_quanta <= right.essential_unserved_quanta
        && left.flexible_unserved_quanta <= right.flexible_unserved_quanta
        && left.operating_cost_units <= right.operating_cost_units
        && left.grid_carbon_grams <= right.grid_carbon_grams
        && left.curtailed_pv_quanta <= right.curtailed_pv_quanta
        && left.terminal_soc_quanta >= right.terminal_soc_quanta;
    let strictly_better = left.essential_unserved_quanta < right.essential_unserved_quanta
        || left.flexible_unserved_quanta < right.flexible_unserved_quanta
        || left.operating_cost_units < right.operating_cost_units
        || left.grid_carbon_grams < right.grid_carbon_grams
        || left.curtailed_pv_quanta < right.curtailed_pv_quanta
        || left.terminal_soc_quanta > right.terminal_soc_quanta;
    no_worse && strictly_better
}

fn derive_worst_case_axis_best(
    profiles: &[ScenarioProfile],
) -> Result<AxisBest, Box<dyn Error>> {
    if profiles.len() != EXPECTED_SCENARIOS {
        return Err(invalid("candidate does not have the exact required scenario set"));
    }
    let first = profiles.first()
        .ok_or_else(|| invalid("candidate has no scenario profiles"))?;
    let mut result = first.axis_best.clone();
    for profile in &profiles[1..] {
        // All dimensions except terminal SoC are minimized objectives.
        result.essential_unserved_quanta =
            result.essential_unserved_quanta.max(profile.axis_best.essential_unserved_quanta);
        result.flexible_unserved_quanta =
            result.flexible_unserved_quanta.max(profile.axis_best.flexible_unserved_quanta);
        result.operating_cost_units =
            result.operating_cost_units.max(profile.axis_best.operating_cost_units);
        result.grid_carbon_grams =
            result.grid_carbon_grams.max(profile.axis_best.grid_carbon_grams);
        result.curtailed_pv_quanta =
            result.curtailed_pv_quanta.max(profile.axis_best.curtailed_pv_quanta);
        result.terminal_soc_quanta =
            result.terminal_soc_quanta.min(profile.axis_best.terminal_soc_quanta);
    }
    Ok(result)
}

fn validate_and_derive(
    matrix: &Value,
    matrix_bytes: &[u8],
) -> Result<RobustnessReport, Box<dyn Error>> {
    let embedded_profile_bytes =
        include_bytes!("../../fixtures/energy-candidate-scenario-panel-v0.1.json");
    let profile: Value = serde_json::from_slice(embedded_profile_bytes)?;
    let embedded_profile_digest = sha256_hex(embedded_profile_bytes);

    if required_str(matrix, "schema_version")? != MATRIX_SCHEMA
        || required_str(matrix, "matrix_id")? != MATRIX_ID
        || required_str(&profile, "schema_version")? != "energy-candidate-scenario-panel-v0.1"
        || required_str(&profile, "panel_id")? != MATRIX_ID
        || required_str(matrix, "scenario_profile_sha256")? != embedded_profile_digest
        || required_u64(matrix, "expected_candidate_count")? != EXPECTED_CANDIDATES as u64
        || required_u64(matrix, "scenario_count")? != EXPECTED_SCENARIOS as u64
        || required_u64(matrix, "expected_row_count")? != EXPECTED_ROWS as u64
        || required_u64(matrix, "observed_row_count")? != EXPECTED_ROWS as u64
        || required_u64(matrix, "passing_row_count")? != EXPECTED_ROWS as u64
        || required_u64(matrix, "failed_or_unknown_row_count")? != 0
        || matrix["all_passed"].as_bool() != Some(true)
    {
        return Err(invalid("matrix schema, pinned profile digest, complete coverage, or all-pass gate mismatch"));
    }

    let scenario_ids = profile["required_scenarios"].as_array()
        .ok_or_else(|| invalid("embedded scenario profile has no required_scenarios array"))?
        .iter()
        .map(|item| required_str(item, "scenario_id").map(str::to_owned))
        .collect::<Result<Vec<_>, _>>()?;
    if scenario_ids.len() != EXPECTED_SCENARIOS
        || scenario_ids.iter().collect::<BTreeSet<_>>().len() != EXPECTED_SCENARIOS
    {
        return Err(invalid("embedded scenario profile is not a five-scenario unique set"));
    }
    let allowed_scenarios: BTreeSet<String> = scenario_ids.iter().cloned().collect();

    let rows = matrix["rows"].as_array()
        .ok_or_else(|| invalid("matrix report has no rows array"))?;
    if rows.len() != EXPECTED_ROWS {
        return Err(invalid("matrix report must contain exactly 195 row objects"));
    }

    let expected_ids = expected_candidate_ids();
    let allowed_candidates: BTreeSet<String> = expected_ids.iter().cloned().collect();
    let mut pairs = BTreeSet::<(String, String)>::new();
    let mut observed_candidates = BTreeSet::<String>::new();
    let mut observed_scenarios = BTreeSet::<String>::new();
    let mut source_manifest_digest: Option<String> = None;
    let mut grouped = BTreeMap::<String, BTreeMap<String, Vec<DispatchObjective>>>::new();

    for row in rows {
        let candidate_id = required_str(row, "candidate_id")?.to_owned();
        let scenario_id = required_str(row, "scenario_id")?.to_owned();
        if !allowed_candidates.contains(&candidate_id)
            || !allowed_scenarios.contains(&scenario_id)
            || !pairs.insert((candidate_id.clone(), scenario_id.clone()))
        {
            return Err(invalid(format!(
                "unexpected or duplicate candidate/scenario pair: {candidate_id}/{scenario_id}"
            )));
        }
        if required_str(row, "status")? != "pass"
            || required_str(row, "independent_audit_status")? != "pass"
            || required_str(row, "reference_frontier_status")? != "pass"
            || required_str(row, "objective_claim_status")? != "verified_for_bounded_model_case"
        {
            return Err(invalid(format!("row is not independently qualified: {candidate_id}/{scenario_id}")));
        }

        let mut solver_frontier: Vec<DispatchObjective> =
            serde_json::from_value(row["solver_frontier_objectives"].clone())?;
        let mut reference_frontier: Vec<DispatchObjective> =
            serde_json::from_value(row["independent_reference_frontier_objectives"].clone())?;
        solver_frontier.sort();
        reference_frontier.sort();
        if solver_frontier.is_empty()
            || solver_frontier != reference_frontier
            || solver_frontier.len() as u64 != required_u64(row, "solver_frontier_count")?
            || reference_frontier.len() as u64 != required_u64(row, "independent_reference_frontier_count")?
            || required_u64(row, "solver_feasible_complete_schedules")?
                != required_u64(row, "independent_reference_feasible_complete_schedules")?
        {
            return Err(invalid(format!(
                "complete solver/reference objective vectors or counts disagree: {candidate_id}/{scenario_id}"
            )));
        }

        let manifest_digest = required_str(matrix, "design_space_manifest_sha256")?.to_owned();
        if !valid_sha256(&manifest_digest) {
            return Err(invalid("design-space manifest digest is not a SHA-256 hex digest"));
        }
        if let Some(existing) = &source_manifest_digest {
            if existing != &manifest_digest {
                return Err(invalid("matrix contains inconsistent design-manifest identity"));
            }
        } else {
            source_manifest_digest = Some(manifest_digest);
        }

        observed_candidates.insert(candidate_id.clone());
        observed_scenarios.insert(scenario_id.clone());
        grouped.entry(candidate_id)
            .or_default()
            .insert(scenario_id, solver_frontier);
    }

    let expected_pairs = EXPECTED_ROWS;
    if pairs.len() != expected_pairs
        || observed_candidates.len() != EXPECTED_CANDIDATES
        || observed_scenarios != allowed_scenarios
    {
        return Err(invalid("candidate/scenario Cartesian coverage is incomplete"));
    }
    for candidate_id in &expected_ids {
        let scenario_map = grouped.get(candidate_id)
            .ok_or_else(|| invalid(format!("candidate missing from matrix: {candidate_id}")))?;
        if scenario_map.len() != EXPECTED_SCENARIOS
            || scenario_ids.iter().any(|id| !scenario_map.contains_key(id))
        {
            return Err(invalid(format!("candidate missing one or more scenario rows: {candidate_id}")));
        }
    }

    let mut profiles = Vec::with_capacity(EXPECTED_CANDIDATES);
    for candidate_id in expected_ids {
        let scenario_map = grouped.remove(&candidate_id)
            .ok_or_else(|| invalid(format!("candidate disappeared during derivation: {candidate_id}")))?;
        let mut scenario_profiles = Vec::with_capacity(EXPECTED_SCENARIOS);
        for scenario_id in &scenario_ids {
            let frontier = scenario_map.get(scenario_id)
                .ok_or_else(|| invalid(format!("missing scenario frontier: {candidate_id}/{scenario_id}")))?
                .clone();
            scenario_profiles.push(ScenarioProfile {
                scenario_id: scenario_id.clone(),
                frontier_count: frontier.len(),
                axis_best: axis_best(&frontier)?,
                frontier_objectives: frontier,
            });
        }
        let envelope = derive_worst_case_axis_best(&scenario_profiles)?;
        profiles.push(CandidateProfile {
            candidate_id,
            scenario_profiles,
            worst_case_of_per_scenario_best_axis_value: envelope,
            envelope_dominates: Vec::new(),
            envelope_dominated_by: Vec::new(),
            envelope_equal_to: Vec::new(),
            envelope_incomparable_with: Vec::new(),
        });
    }

    // Compare all candidates without selecting weights. An equal envelope is
    // reported separately from strict dominance and incomparability.
    for left_index in 0..profiles.len() {
        for right_index in (left_index + 1)..profiles.len() {
            let left_id = profiles[left_index].candidate_id.clone();
            let right_id = profiles[right_index].candidate_id.clone();
            let left = profiles[left_index].worst_case_of_per_scenario_best_axis_value.clone();
            let right = profiles[right_index].worst_case_of_per_scenario_best_axis_value.clone();
            if dominates(&left, &right) {
                profiles[left_index].envelope_dominates.push(right_id.clone());
                profiles[right_index].envelope_dominated_by.push(left_id);
            } else if dominates(&right, &left) {
                profiles[right_index].envelope_dominates.push(left_id.clone());
                profiles[left_index].envelope_dominated_by.push(right_id);
            } else if left == right {
                profiles[left_index].envelope_equal_to.push(right_id.clone());
                profiles[right_index].envelope_equal_to.push(left_id);
            } else {
                profiles[left_index].envelope_incomparable_with.push(right_id.clone());
                profiles[right_index].envelope_incomparable_with.push(left_id);
            }
        }
    }

    let manifest_digest = source_manifest_digest
        .ok_or_else(|| invalid("matrix source manifest digest was not present"))?;
    Ok(RobustnessReport {
        schema_version: OUTPUT_SCHEMA,
        status: "pass",
        matrix_id: MATRIX_ID.to_owned(),
        source_matrix_sha256: sha256_hex(matrix_bytes),
        source_scenario_profile_sha256: embedded_profile_digest,
        source_design_space_manifest_sha256: manifest_digest,
        candidate_count: EXPECTED_CANDIDATES,
        scenario_count: EXPECTED_SCENARIOS,
        expected_row_count: EXPECTED_ROWS,
        observed_row_count: pairs.len(),
        passed_row_count: EXPECTED_ROWS,
        failed_or_unknown_row_count: 0,
        deterministic_scenario_ids: scenario_ids,
        candidates: profiles,
        claim_ceiling: "Derived only from the complete v0.2 matrix for the declared synthetic finite model. Per-scenario axis-best and worst-case-of-best values are coordinate-wise envelopes and may come from different schedules; they are not jointly attainable dispatch plans, calibrated site resilience, real economics, or an industry-outperformance claim.",
    })
}

fn run() -> Result<(), Box<dyn Error>> {
    let args = env::args().collect::<Vec<_>>();
    if args.len() != 3 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "usage: energy_candidate_robustness_report <matrix-report-v0.2.json> <robustness-report.json>",
        ).into());
    }
    let matrix_bytes = fs::read(&args[1])?;
    let matrix: Value = serde_json::from_slice(&matrix_bytes)?;
    let report = validate_and_derive(&matrix, &matrix_bytes)?;
    let output_bytes = serde_json::to_vec_pretty(&report)?;
    fs::write(&args[2], &output_bytes)?;
    println!("PASS: derived robustness report for {} candidates × {} scenarios",
        report.candidate_count, report.scenario_count);
    println!("source_matrix_sha256={}", report.source_matrix_sha256);
    println!("report_sha256={}", sha256_hex(&output_bytes));
    println!("claim_ceiling={}", report.claim_ceiling);
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("candidate robustness report failed: {error}");
        std::process::exit(2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_objective(essential: u64, flexible: u64, cost: u64, carbon: u64, curt: u64, soc: u16) -> DispatchObjective {
        DispatchObjective {
            essential_unserved_quanta: essential,
            flexible_unserved_quanta: flexible,
            operating_cost_units: cost,
            grid_carbon_grams: carbon,
            curtailed_pv_quanta: curt,
            terminal_soc_quanta: soc,
        }
    }

    fn synthetic_matrix() -> Value {
        let profile_bytes = include_bytes!("../../fixtures/energy-candidate-scenario-panel-v0.1.json");
        let profile: Value = serde_json::from_slice(profile_bytes).unwrap();
        let scenario_ids = profile["required_scenarios"].as_array().unwrap()
            .iter().map(|item| item["scenario_id"].as_str().unwrap().to_owned())
            .collect::<Vec<_>>();
        let mut rows = Vec::new();
        for (index, candidate_id) in expected_candidate_ids().iter().enumerate() {
            for scenario_id in &scenario_ids {
                let objective = test_objective(
                    (index % 4) as u64,
                    (index % 3) as u64,
                    (39 - index) as u64,
                    (index * 2) as u64,
                    (index % 5) as u64,
                    (index % 7) as u16,
                );
                let obj = serde_json::to_value(&objective).unwrap();
                rows.push(serde_json::json!({
                    "candidate_id": candidate_id,
                    "scenario_id": scenario_id,
                    "status": "pass",
                    "independent_audit_status": "pass",
                    "reference_frontier_status": "pass",
                    "objective_claim_status": "verified_for_bounded_model_case",
                    "solver_feasible_complete_schedules": 1,
                    "independent_reference_feasible_complete_schedules": 1,
                    "solver_frontier_count": 1,
                    "independent_reference_frontier_count": 1,
                    "solver_frontier_objectives": [obj.clone()],
                    "independent_reference_frontier_objectives": [obj]
                }));
            }
        }
        serde_json::json!({
            "schema_version": MATRIX_SCHEMA,
            "matrix_id": MATRIX_ID,
            "scenario_profile_sha256": sha256_hex(profile_bytes),
            "design_space_manifest_sha256": "a".repeat(64),
            "expected_candidate_count": EXPECTED_CANDIDATES,
            "scenario_count": EXPECTED_SCENARIOS,
            "expected_row_count": EXPECTED_ROWS,
            "observed_row_count": EXPECTED_ROWS,
            "passing_row_count": EXPECTED_ROWS,
            "failed_or_unknown_row_count": 0,
            "all_passed": true,
            "rows": rows
        })
    }

    #[test]
    fn complete_matrix_derives_deterministic_robust_profiles_and_relations() {
        let matrix = synthetic_matrix();
        let bytes = serde_json::to_vec(&matrix).unwrap();
        let report = validate_and_derive(&matrix, &bytes).unwrap();
        assert_eq!(report.candidate_count, 39);
        assert_eq!(report.scenario_count, 5);
        assert_eq!(report.observed_row_count, 195);
        assert_eq!(report.candidates.len(), 39);
        assert!(report.candidates.iter().all(|candidate| candidate.scenario_profiles.len() == 5));
        assert!(report.candidates.iter().all(|candidate| {
            candidate.scenario_profiles.iter().all(|scenario| !scenario.frontier_objectives.is_empty())
        }));
        let second = validate_and_derive(&matrix, &bytes).unwrap();
        assert_eq!(
            serde_json::to_vec(&report).unwrap(),
            serde_json::to_vec(&second).unwrap(),
            "same input bytes should produce deterministic report bytes"
        );
    }

    #[test]
    fn matrix_gate_rejects_missing_unknown_and_duplicate_pairs() {
        let mut missing = synthetic_matrix();
        missing["rows"].as_array_mut().unwrap().pop();
        assert!(validate_and_derive(&missing, &serde_json::to_vec(&missing).unwrap()).is_err());

        let mut unknown = synthetic_matrix();
        unknown["rows"][0]["scenario_id"] = serde_json::json!("unregistered-scenario");
        assert!(validate_and_derive(&unknown, &serde_json::to_vec(&unknown).unwrap()).is_err());

        let mut duplicate = synthetic_matrix();
        duplicate["rows"][1]["candidate_id"] = duplicate["rows"][0]["candidate_id"].clone();
        duplicate["rows"][1]["scenario_id"] = duplicate["rows"][0]["scenario_id"].clone();
        assert!(validate_and_derive(&duplicate, &serde_json::to_vec(&duplicate).unwrap()).is_err());
    }

    #[test]
    fn matrix_gate_rejects_solver_reference_mismatch_and_failed_audit() {
        let mut mismatch = synthetic_matrix();
        mismatch["rows"][0]["independent_reference_frontier_objectives"][0]["operating_cost_units"] =
            serde_json::json!(999);
        assert!(validate_and_derive(&mismatch, &serde_json::to_vec(&mismatch).unwrap()).is_err());

        let mut failed = synthetic_matrix();
        failed["rows"][0]["independent_audit_status"] = serde_json::json!("fail");
        assert!(validate_and_derive(&failed, &serde_json::to_vec(&failed).unwrap()).is_err());
    }

    #[test]
    fn axis_best_is_explicitly_not_a_jointly_attainable_schedule() {
        let frontier = vec![
            test_objective(0, 0, 10, 10, 4, 2),
            test_objective(2, 1, 2, 3, 1, 8),
        ];
        let best = axis_best(&frontier).unwrap();
        assert_eq!(best.essential_unserved_quanta, 0);
        assert_eq!(best.operating_cost_units, 2);
        assert_eq!(best.terminal_soc_quanta, 8);
        assert!(!frontier.contains(&test_objective(
            best.essential_unserved_quanta,
            best.flexible_unserved_quanta,
            best.operating_cost_units,
            best.grid_carbon_grams,
            best.curtailed_pv_quanta,
            best.terminal_soc_quanta
        )), "axis-wise best values must not be misrepresented as one feasible schedule");
    }

    #[test]
    fn envelope_dominance_respects_every_axis_and_terminal_soc_direction() {
        let better = AxisBest {
            essential_unserved_quanta: 0,
            flexible_unserved_quanta: 0,
            operating_cost_units: 3,
            grid_carbon_grams: 4,
            curtailed_pv_quanta: 2,
            terminal_soc_quanta: 8,
        };
        let worse = AxisBest {
            essential_unserved_quanta: 1,
            flexible_unserved_quanta: 2,
            operating_cost_units: 5,
            grid_carbon_grams: 6,
            curtailed_pv_quanta: 3,
            terminal_soc_quanta: 5,
        };
        assert!(dominates(&better, &worse));
        assert!(!dominates(&worse, &better));
        assert!(!dominates(&better, &better));

        let traded_off = AxisBest {
            essential_unserved_quanta: 0,
            flexible_unserved_quanta: 0,
            operating_cost_units: 9,
            grid_carbon_grams: 10,
            curtailed_pv_quanta: 1,
            terminal_soc_quanta: 9,
        };
        assert!(!dominates(&better, &traded_off));
        assert!(!dominates(&traded_off, &better));
    }

    #[test]
    fn empty_frontier_is_unknown_not_a_zero_objective_pass() {
        assert!(axis_best(&[]).is_err());
    }
}
