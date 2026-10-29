use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::{env, error::Error, fs};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    schema_version: String,
    artifact_kind: String,
    evidence_class: String,
    status: String,
    reference_controller_id: String,
    candidate_controller_id: String,
    controllers: Vec<String>,
    axes: Vec<Axis>,
    gate_ids: Vec<String>,
    gate_margin_contracts: BTreeMap<String, GateMarginContract>,
    trajectory_artifacts: Vec<TrajectoryArtifact>,
    controller_artifacts: Vec<ControllerArtifact>,
    cases: Vec<Case>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct Axis {
    id: String,
    unit: String,
    higher_is_more_severe: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct GateMarginContract {
    unit: String,
    signed_margin_definition: String,
    hard_gate: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct TrajectoryArtifact {
    trajectory_id: String,
    initial_state_id: String,
    trajectory_payload: String,
    trajectory_sha256: String,
    initial_state_payload: String,
    initial_state_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct ControllerArtifact {
    controller_id: String,
    controller_payload: String,
    controller_sha256: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum GateResult {
    Pass,
    Breach,
    Invalid,
    NotEvaluated,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    case_id: String,
    controller_id: String,
    trajectory_id: String,
    comparison_group_id: String,
    hazard_family: String,
    initial_state_id: String,
    replicate_id: String,
    severity: BTreeMap<String, f64>,
    trajectory_sha256: String,
    initial_state_sha256: String,
    controller_artifact_sha256: String,
    gate_results: BTreeMap<String, GateResult>,
    signed_gate_margins: BTreeMap<String, Option<f64>>,
}

#[derive(Clone, Debug, PartialEq)]
struct TrajectoryMeta {
    comparison_group_id: String,
    hazard_family: String,
    initial_state_id: String,
    replicate_id: String,
    severity: BTreeMap<String, f64>,
    trajectory_sha256: String,
    initial_state_sha256: String,
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn unique(values: &[String]) -> bool {
    let set: BTreeSet<&String> = values.iter().collect();
    set.len() == values.len()
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

fn validate(input: &Input) -> Result<(), String> {
    if input.schema_version != "0.4.0"
        || input.artifact_kind != "paired-resilience-comparison-input"
        || input.evidence_class != "synthetic_fixture"
        || input.status != "synthetic_only"
    {
        return Err("unsupported header or evidence claim; this executable accepts synthetic fixtures only".into());
    }
    if input.controllers.len() < 2 || !unique(&input.controllers)
        || input.controllers.iter().any(|v| !nonempty(v))
    {
        return Err("controllers must contain at least two unique non-empty IDs".into());
    }
    if input.reference_controller_id == input.candidate_controller_id
        || !input.controllers.contains(&input.reference_controller_id)
        || !input.controllers.contains(&input.candidate_controller_id)
    {
        return Err("reference and candidate must be distinct declared controllers".into());
    }
    if input.axes.is_empty() || input.axes.iter().any(|a| !nonempty(&a.id) || !nonempty(&a.unit)) {
        return Err("at least one severity axis with ID and unit is required".into());
    }
    let axis_ids: Vec<String> = input.axes.iter().map(|a| a.id.clone()).collect();
    if !unique(&axis_ids) || input.gate_ids.is_empty() || !unique(&input.gate_ids)
        || input.gate_ids.iter().any(|g| !nonempty(g))
    {
        return Err("axis and gate IDs must be unique and non-empty".into());
    }
    if input.cases.is_empty() {
        return Err("cases cannot be empty".into());
    }

    let expected_axes: BTreeSet<String> = axis_ids.into_iter().collect();
    let expected_gates: BTreeSet<String> = input.gate_ids.iter().cloned().collect();
    if input.gate_margin_contracts.keys().cloned().collect::<BTreeSet<_>>() != expected_gates {
        return Err("gate margin contracts must match the declared gate IDs exactly".into());
    }
    for (gate, contract) in &input.gate_margin_contracts {
        if !nonempty(&contract.unit) || !nonempty(&contract.signed_margin_definition) {
            return Err(format!("gate {gate} must declare margin units and signed-margin semantics"));
        }
        if !contract.hard_gate {
            return Err(format!("gate {gate} is not declared as a hard gate; soft metrics cannot be scored as service gates"));
        }
    }
    let declared_controllers: BTreeSet<String> = input.controllers.iter().cloned().collect();

    let mut trajectory_artifact_ids = BTreeSet::new();
    for artifact in &input.trajectory_artifacts {
        if !nonempty(&artifact.trajectory_id) || !nonempty(&artifact.initial_state_id)
            || artifact.trajectory_payload.is_empty() || artifact.initial_state_payload.is_empty()
            || !valid_sha256(&artifact.trajectory_sha256) || !valid_sha256(&artifact.initial_state_sha256)
        {
            return Err("trajectory and initial-state artifacts need non-empty payloads and lowercase SHA-256 digests".into());
        }
        if !trajectory_artifact_ids.insert(artifact.trajectory_id.clone()) {
            return Err(format!("duplicate trajectory artifact: {}", artifact.trajectory_id));
        }
    }
    let mut controller_artifact_ids = BTreeSet::new();
    for artifact in &input.controller_artifacts {
        if !nonempty(&artifact.controller_id) || artifact.controller_payload.is_empty()
            || !valid_sha256(&artifact.controller_sha256)
        {
            return Err("controller artifacts need a non-empty payload and lowercase SHA-256 digest".into());
        }
        if !controller_artifact_ids.insert(artifact.controller_id.clone()) {
            return Err(format!("duplicate controller artifact: {}", artifact.controller_id));
        }
    }
    if trajectory_artifact_ids != input.cases.iter().map(|c| c.trajectory_id.clone()).collect()
        || controller_artifact_ids != declared_controllers
    {
        return Err("artifact inventories must exactly match the declared trajectories and controllers".into());
    }
    let trajectory_artifacts: BTreeMap<&str, &TrajectoryArtifact> = input.trajectory_artifacts.iter()
        .map(|a| (a.trajectory_id.as_str(), a)).collect();
    let controller_artifacts: BTreeMap<&str, &ControllerArtifact> = input.controller_artifacts.iter()
        .map(|a| (a.controller_id.as_str(), a)).collect();

    let mut case_ids = BTreeSet::new();
    let mut trajectory_meta: BTreeMap<String, TrajectoryMeta> = BTreeMap::new();
    let mut rows: BTreeMap<(String, String), &Case> = BTreeMap::new();
    let mut trajectories = BTreeSet::new();

    for case in &input.cases {
        for (field, value) in [
            ("case_id", case.case_id.as_str()),
            ("controller_id", case.controller_id.as_str()),
            ("trajectory_id", case.trajectory_id.as_str()),
            ("comparison_group_id", case.comparison_group_id.as_str()),
            ("hazard_family", case.hazard_family.as_str()),
            ("initial_state_id", case.initial_state_id.as_str()),
            ("replicate_id", case.replicate_id.as_str()),
        ] {
            if !nonempty(value) {
                return Err(format!("{field} cannot be empty"));
            }
        }
        if !case_ids.insert(case.case_id.clone()) {
            return Err(format!("duplicate case_id: {}", case.case_id));
        }
        if !declared_controllers.contains(&case.controller_id) {
            return Err(format!("undeclared controller: {}", case.controller_id));
        }
        if !valid_sha256(&case.trajectory_sha256) || !valid_sha256(&case.initial_state_sha256)
            || !valid_sha256(&case.controller_artifact_sha256)
        {
            return Err(format!("case {} contains an invalid SHA-256 digest", case.case_id));
        }
        let trajectory_artifact = trajectory_artifacts.get(case.trajectory_id.as_str()).expect("inventory checked");
        let controller_artifact = controller_artifacts.get(case.controller_id.as_str()).expect("inventory checked");
        if case.trajectory_sha256 != trajectory_artifact.trajectory_sha256
            || case.initial_state_id != trajectory_artifact.initial_state_id
            || case.initial_state_sha256 != trajectory_artifact.initial_state_sha256
            || case.controller_artifact_sha256 != controller_artifact.controller_sha256
        {
            return Err(format!("case {} digest/ID binding does not match the artifact registry", case.case_id));
        }
        if case.severity.keys().cloned().collect::<BTreeSet<_>>() != expected_axes {
            return Err(format!("severity axes do not match declarations for {}", case.case_id));
        }
        if case.severity.values().any(|v| !v.is_finite() || *v < 0.0) {
            return Err(format!("severity must be finite and non-negative for {}", case.case_id));
        }
        if case.gate_results.keys().cloned().collect::<BTreeSet<_>>() != expected_gates
            || case.signed_gate_margins.keys().cloned().collect::<BTreeSet<_>>() != expected_gates
        {
            return Err(format!("gate result/margin keys do not match declarations for {}", case.case_id));
        }
        for gate in &input.gate_ids {
            let result = case.gate_results.get(gate).expect("keys checked");
            let margin = case.signed_gate_margins.get(gate).expect("keys checked");
            match (result, margin) {
                (GateResult::Pass, Some(v)) if v.is_finite() && *v >= 0.0 => {}
                (GateResult::Breach, Some(v)) if v.is_finite() && *v < 0.0 => {}
                (GateResult::Invalid | GateResult::NotEvaluated, None) => {}
                _ => return Err(format!("status/margin mismatch for case {} gate {}", case.case_id, gate)),
            }
        }

        let meta = TrajectoryMeta {
            comparison_group_id: case.comparison_group_id.clone(),
            hazard_family: case.hazard_family.clone(),
            initial_state_id: case.initial_state_id.clone(),
            replicate_id: case.replicate_id.clone(),
            severity: case.severity.clone(),
            trajectory_sha256: case.trajectory_sha256.clone(),
            initial_state_sha256: case.initial_state_sha256.clone(),
        };
        if let Some(previous) = trajectory_meta.get(&case.trajectory_id) {
            if previous != &meta {
                return Err(format!("paired rows disagree on trajectory metadata: {}", case.trajectory_id));
            }
        } else {
            trajectory_meta.insert(case.trajectory_id.clone(), meta);
        }
        trajectories.insert(case.trajectory_id.clone());
        let pair_key = (case.controller_id.clone(), case.trajectory_id.clone());
        if rows.insert(pair_key, case).is_some() {
            return Err(format!("duplicate controller/trajectory row for {}", case.case_id));
        }
    }

    let expected_rows = input.controllers.len() * trajectories.len();
    if rows.len() != expected_rows || input.cases.len() != expected_rows {
        return Err("controller/trajectory matrix is incomplete: missing or duplicate executions are forbidden; encode them as invalid or not_evaluated gate results".into());
    }
    for controller in &input.controllers {
        for trajectory in &trajectories {
            if !rows.contains_key(&(controller.clone(), trajectory.clone())) {
                return Err(format!("missing row: controller {} trajectory {}", controller, trajectory));
            }
        }
    }
    Ok(())
}

fn analyze(input: &Input) -> Value {
    let mut coverage = Vec::new();
    let mut comparisons = Vec::new();

    for controller in &input.controllers {
        for gate in &input.gate_ids {
            let mut pass = 0usize;
            let mut breach = 0usize;
            let mut invalid = 0usize;
            let mut not_evaluated = 0usize;
            for case in input.cases.iter().filter(|c| &c.controller_id == controller) {
                match case.gate_results[gate] {
                    GateResult::Pass => pass += 1,
                    GateResult::Breach => breach += 1,
                    GateResult::Invalid => invalid += 1,
                    GateResult::NotEvaluated => not_evaluated += 1,
                }
            }
            coverage.push(json!({
                "controller_id": controller,
                "gate_id": gate,
                "pass_count": pass,
                "breach_count": breach,
                "invalid_count": invalid,
                "not_evaluated_count": not_evaluated,
                "valid_evaluated_count": pass + breach
            }));
        }
    }

    let trajectories: BTreeSet<String> = input.cases.iter().map(|c| c.trajectory_id.clone()).collect();
    for gate in &input.gate_ids {
        let mut both_pass = 0usize;
        let mut both_breach = 0usize;
        let mut reference_breach_candidate_pass = 0usize;
        let mut reference_pass_candidate_breach = 0usize;
        let mut incomplete = 0usize;
        let mut deltas = Vec::new();

        for trajectory in &trajectories {
            let reference = input.cases.iter().find(|c|
                c.controller_id == input.reference_controller_id && &c.trajectory_id == trajectory
            ).expect("validated matrix");
            let candidate = input.cases.iter().find(|c|
                c.controller_id == input.candidate_controller_id && &c.trajectory_id == trajectory
            ).expect("validated matrix");
            let r = reference.gate_results[gate];
            let c = candidate.gate_results[gate];
            match (r, c) {
                (GateResult::Pass, GateResult::Pass) => both_pass += 1,
                (GateResult::Breach, GateResult::Breach) => both_breach += 1,
                (GateResult::Breach, GateResult::Pass) => reference_breach_candidate_pass += 1,
                (GateResult::Pass, GateResult::Breach) => reference_pass_candidate_breach += 1,
                _ => incomplete += 1,
            }
            if let (Some(rm), Some(cm)) = (
                reference.signed_gate_margins[gate],
                candidate.signed_gate_margins[gate],
            ) {
                deltas.push(cm - rm);
            }
        }
        let mean_delta = if deltas.is_empty() {
            Value::Null
        } else {
            json!(deltas.iter().sum::<f64>() / deltas.len() as f64)
        };
        let minimum_delta = deltas.iter().copied().reduce(f64::min);
        comparisons.push(json!({
            "gate_id": gate,
            "reference_controller_id": input.reference_controller_id,
            "candidate_controller_id": input.candidate_controller_id,
            "trajectory_pair_count": trajectories.len(),
            "both_pass_count": both_pass,
            "both_breach_count": both_breach,
            "reference_breach_candidate_pass_count": reference_breach_candidate_pass,
            "reference_pass_candidate_breach_count": reference_pass_candidate_breach,
            "incomplete_pair_count": incomplete,
            "margin_delta_count": deltas.len(),
            "mean_candidate_minus_reference_margin": mean_delta,
            "minimum_candidate_minus_reference_margin": minimum_delta
        }));
    }

    json!({
        "schema_version": "0.3.0",
        "report_kind": "paired-resilience-comparison-report",
        "evidence_class": "synthetic_fixture",
        "status": "synthetic_only",
        "controller_count": input.controllers.len(),
        "trajectory_count": trajectories.len(),
        "case_count": input.cases.len(),
        "gate_margin_contracts": input.gate_margin_contracts,
        "coverage": coverage,
        "paired_gate_comparisons": comparisons,
        "performance_claim": false,
        "result_scope": "finite synthetic contract fixture only; no physical model, field validation, standards conformity, or controller superiority established"
    })
}

fn main() -> Result<(), Box<dyn Error>> {
    let path = env::args().nth(1).ok_or("usage: paired_comparison <input.json>")?;
    let raw = fs::read_to_string(path)?;
    let input: Input = serde_json::from_str(&raw)?;
    validate(&input).map_err(|e| format!("paired comparison rejected: {e}"))?;
    println!("{}", analyze(&input));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load() -> Input {
        serde_json::from_str(include_str!("../../fixtures/paired-comparison-v0.4.json")).unwrap()
    }

    #[test]
    fn synthetic_fixture_is_valid() {
        validate(&load()).unwrap();
    }

    #[test]
    fn rejects_missing_controller_trajectory_row() {
        let mut input = load();
        input.cases.pop();
        assert!(validate(&input).unwrap_err().contains("incomplete"));
    }

    #[test]
    fn rejects_paired_severity_mismatch() {
        let mut input = load();
        input.cases.last_mut().unwrap().severity.insert("storm_duration_h".into(), 99.0);
        assert!(validate(&input).unwrap_err().contains("disagree on trajectory metadata"));
    }

    #[test]
    fn rejects_margin_status_contradiction() {
        let mut input = load();
        input.cases[0].signed_gate_margins.insert("critical_energy".into(), Some(-1.0));
        assert!(validate(&input).unwrap_err().contains("status/margin mismatch"));
    }

    #[test]
    fn rejects_unadmitted_evidence_escalation() {
        let mut input = load();
        input.evidence_class = "field_validated".into();
        assert!(validate(&input).unwrap_err().contains("synthetic fixtures only"));
    }

    #[test]
    fn rejects_invalid_digest_format() {
        let mut input = load();
        input.cases[0].trajectory_sha256 = "NOT_A_SHA256".into();
        assert!(validate(&input).unwrap_err().contains("invalid SHA-256 digest"));
    }

    #[test]
    fn rejects_digest_reference_mismatch() {
        let mut input = load();
        input.cases[1].trajectory_sha256 = "a".repeat(64);
        assert!(validate(&input).unwrap_err().contains("digest/ID binding"));
    }

    #[test]
    fn rejects_missing_trajectory_artifact() {
        let mut input = load();
        input.trajectory_artifacts.pop();
        assert!(validate(&input).unwrap_err().contains("artifact inventories"));
    }

    #[test]
    fn rejects_controller_artifact_drift() {
        let mut input = load();
        input.cases[1].controller_artifact_sha256 = "a".repeat(64);
        assert!(validate(&input).unwrap_err().contains("digest/ID binding"));
    }
}
