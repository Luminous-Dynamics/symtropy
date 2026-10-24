use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::error::Error;
use std::fs;
use std::io;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct BoundaryInput {
    schema_version: String,
    artifact_kind: String,
    evidence_class: String,
    status: String,
    search_partition_id: String,
    axes: Vec<SeverityAxis>,
    gate_ids: Vec<String>,
    cases: Vec<BoundaryCase>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SeverityAxis {
    id: String,
    unit: String,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Outcome {
    Pass,
    GateBreach,
    Invalid,
    NotEvaluated,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct BoundaryCase {
    case_id: String,
    comparison_group_id: String,
    hazard_family: String,
    initial_state_id: String,
    replicate_id: String,
    severity: BTreeMap<String, f64>,
    outcome: Outcome,
    failed_gate_ids: Vec<String>,
}

#[derive(Debug, Serialize)]
struct BoundaryReport {
    schema_version: &'static str,
    report_kind: &'static str,
    evidence_class: &'static str,
    search_partition_id: String,
    case_count: usize,
    pass_count: usize,
    gate_breach_case_count: usize,
    invalid_case_count: usize,
    not_evaluated_case_count: usize,
    frontier_completeness_status: &'static str,
    result_scope: &'static str,
    frontiers: Vec<GateFrontier>,
    gates_without_observed_breach: Vec<String>,
}

#[derive(Debug, Serialize)]
struct GateFrontier {
    gate_id: String,
    comparison_group_id: String,
    hazard_family: String,
    initial_state_id: String,
    minimal_observed_failing_points: Vec<BoundaryPoint>,
}

#[derive(Debug, Serialize)]
struct BoundaryPoint {
    case_id: String,
    replicate_id: String,
    severity: BTreeMap<String, f64>,
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn dominates(a: &BoundaryCase, b: &BoundaryCase, axis_ids: &[String]) -> bool {
    // Only compare cases explicitly placed in the same comparison group. The
    // input validator guarantees that a group has one hazard family and initial state.
    if a.comparison_group_id != b.comparison_group_id {
        return false;
    }
    let no_more_severe = axis_ids
        .iter()
        .all(|axis| a.severity[axis] <= b.severity[axis]);
    let strictly_less_severe = axis_ids
        .iter()
        .any(|axis| a.severity[axis] < b.severity[axis]);
    no_more_severe && strictly_less_severe
}

fn aggregate(input: &BoundaryInput) -> Result<BoundaryReport, String> {
    if input.schema_version != "0.1.0"
        || input.artifact_kind != "polar-resilience-boundary-search-input"
        || input.evidence_class != "synthetic_only"
        || input.status != "synthetic_fixture"
    {
        return Err(
            "only schema 0.1.0 synthetic_fixture inputs with synthetic_only evidence are accepted"
                .to_string(),
        );
    }
    if !nonempty(&input.search_partition_id) {
        return Err("search_partition_id must be non-empty".to_string());
    }
    if input.axes.is_empty() || input.gate_ids.is_empty() || input.cases.is_empty() {
        return Err("at least one severity axis, gate ID, and case are required".to_string());
    }

    let mut axis_ids = Vec::with_capacity(input.axes.len());
    let mut axis_set = BTreeSet::new();
    for axis in &input.axes {
        if !nonempty(&axis.id) || !nonempty(&axis.unit) {
            return Err("severity axes require non-empty IDs and units".to_string());
        }
        if !axis_set.insert(axis.id.clone()) {
            return Err(format!("duplicate severity axis ID {}", axis.id));
        }
        axis_ids.push(axis.id.clone());
    }

    let mut gate_set = BTreeSet::new();
    for gate_id in &input.gate_ids {
        if !nonempty(gate_id) || !gate_set.insert(gate_id.clone()) {
            return Err("gate IDs must be non-empty and unique".to_string());
        }
    }

    let mut case_ids = BTreeSet::new();
    let mut group_basis: BTreeMap<String, (String, String)> = BTreeMap::new();
    for case in &input.cases {
        if !nonempty(&case.case_id)
            || !nonempty(&case.comparison_group_id)
            || !nonempty(&case.hazard_family)
            || !nonempty(&case.initial_state_id)
            || !nonempty(&case.replicate_id)
        {
            return Err("case, group, hazard, initial-state, and replicate IDs must be non-empty".to_string());
        }
        if !case_ids.insert(case.case_id.clone()) {
            return Err(format!("duplicate case ID {}", case.case_id));
        }

        let basis = (case.hazard_family.clone(), case.initial_state_id.clone());
        if let Some(previous) = group_basis.get(&case.comparison_group_id) {
            if previous != &basis {
                return Err(format!(
                    "comparison group {} changes hazard family or initial state",
                    case.comparison_group_id
                ));
            }
        } else {
            group_basis.insert(case.comparison_group_id.clone(), basis);
        }

        if case.severity.len() != axis_set.len()
            || case.severity.keys().any(|axis_id| !axis_set.contains(axis_id))
        {
            return Err(format!(
                "case {} severity keys must exactly match declared axes",
                case.case_id
            ));
        }
        for (axis_id, value) in &case.severity {
            if !value.is_finite() || *value < 0.0 {
                return Err(format!(
                    "case {}/{} severity must be finite and nonnegative",
                    case.case_id, axis_id
                ));
            }
        }

        let mut seen_gates = BTreeSet::new();
        for gate_id in &case.failed_gate_ids {
            if !gate_set.contains(gate_id) {
                return Err(format!("case {} references undeclared gate {}", case.case_id, gate_id));
            }
            if !seen_gates.insert(gate_id) {
                return Err(format!("case {} repeats failed gate {}", case.case_id, gate_id));
            }
        }
        match case.outcome {
            Outcome::GateBreach if case.failed_gate_ids.is_empty() => {
                return Err(format!(
                    "gate_breach case {} must name at least one failed gate",
                    case.case_id
                ));
            }
            Outcome::Pass | Outcome::Invalid | Outcome::NotEvaluated
                if !case.failed_gate_ids.is_empty() =>
            {
                return Err(format!(
                    "non-breach case {} cannot claim failed gates",
                    case.case_id
                ));
            }
            _ => {}
        }
    }

    let pass_count = input.cases.iter().filter(|c| c.outcome == Outcome::Pass).count();
    let gate_breach_case_count = input.cases.iter().filter(|c| c.outcome == Outcome::GateBreach).count();
    let invalid_case_count = input.cases.iter().filter(|c| c.outcome == Outcome::Invalid).count();
    let not_evaluated_case_count = input.cases.iter().filter(|c| c.outcome == Outcome::NotEvaluated).count();

    // Compute one frontier per gate and explicit comparison group. Different
    // hazard families or initial states are deliberately incomparable.
    let mut groups: BTreeMap<(String, String), Vec<&BoundaryCase>> = BTreeMap::new();
    for case in &input.cases {
        if case.outcome != Outcome::GateBreach {
            continue;
        }
        for gate_id in &case.failed_gate_ids {
            groups
                .entry((gate_id.clone(), case.comparison_group_id.clone()))
                .or_default()
                .push(case);
        }
    }

    let mut frontiers = Vec::with_capacity(groups.len());
    for ((gate_id, comparison_group_id), cases) in groups {
        let nondominated: Vec<&BoundaryCase> = cases
            .iter()
            .copied()
            .filter(|candidate| {
                !cases.iter().copied().any(|other| {
                    other.case_id != candidate.case_id
                        && other.outcome == Outcome::GateBreach
                        && other.failed_gate_ids.contains(&gate_id)
                        && dominates(other, candidate, &axis_ids)
                })
            })
            .collect();

        let first = nondominated
            .first()
            .ok_or_else(|| "internal error: non-empty failure group has empty frontier".to_string())?;
        let mut points: Vec<BoundaryPoint> = nondominated
            .iter()
            .map(|case| BoundaryPoint {
                case_id: case.case_id.clone(),
                replicate_id: case.replicate_id.clone(),
                severity: case.severity.clone(),
            })
            .collect();
        points.sort_by(|a, b| a.case_id.cmp(&b.case_id));

        frontiers.push(GateFrontier {
            gate_id,
            comparison_group_id,
            hazard_family: first.hazard_family.clone(),
            initial_state_id: first.initial_state_id.clone(),
            minimal_observed_failing_points: points,
        });
    }

    let observed_gates: BTreeSet<String> = input
        .cases
        .iter()
        .filter(|c| c.outcome == Outcome::GateBreach)
        .flat_map(|c| c.failed_gate_ids.iter().cloned())
        .collect();
    let gates_without_observed_breach = input
        .gate_ids
        .iter()
        .filter(|gate_id| !observed_gates.contains(*gate_id))
        .cloned()
        .collect();

    Ok(BoundaryReport {
        schema_version: "0.1.0",
        report_kind: "polar-resilience-boundary-search-report",
        evidence_class: "synthetic_only",
        search_partition_id: input.search_partition_id.clone(),
        case_count: input.cases.len(),
        pass_count,
        gate_breach_case_count,
        invalid_case_count,
        not_evaluated_case_count,
        frontier_completeness_status: if invalid_case_count > 0 || not_evaluated_case_count > 0 {
            "incomplete_invalid_or_not_evaluated_cases"
        } else {
            "complete_for_supplied_cases_only"
        },
        result_scope: "observed_enumerated_cases_only_not_a_safety_certificate_or_probability_estimate",
        frontiers,
        gates_without_observed_breach,
    })
}

fn run() -> Result<(), Box<dyn Error>> {
    let path = env::args().nth(1).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "usage: resilience_boundary <synthetic-boundary-search-input.json>",
        )
    })?;
    let raw = fs::read_to_string(path)?;
    let input: BoundaryInput = serde_json::from_str(&raw)?;
    let report = aggregate(&input)
        .map_err(|message| io::Error::new(io::ErrorKind::InvalidData, message))?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("resilience_boundary: {error}");
        std::process::exit(2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> BoundaryInput {
        serde_json::from_str(include_str!("../../fixtures/toy-boundary-search.json"))
            .expect("boundary fixture must be valid JSON")
    }

    #[test]
    fn emits_gate_specific_pareto_minimal_observed_failures() {
        let report = aggregate(&fixture()).expect("fixture should aggregate");
        assert_eq!(report.evidence_class, "synthetic_only");
        assert_eq!(report.case_count, 9);
        assert_eq!(report.pass_count, 1);
        assert_eq!(report.gate_breach_case_count, 6);
        assert_eq!(report.invalid_case_count, 1);
        assert_eq!(report.not_evaluated_case_count, 1);
        assert_eq!(report.frontier_completeness_status, "incomplete_invalid_or_not_evaluated_cases");

        let heat = report.frontiers.iter()
            .find(|f| f.gate_id == "heat_floor" && f.comparison_group_id == "supply_cold_low_reserve")
            .expect("heat frontier should exist");
        let heat_ids: Vec<&str> = heat.minimal_observed_failing_points.iter()
            .map(|p| p.case_id.as_str()).collect();
        assert_eq!(heat_ids, vec!["heat_short_low_a", "heat_short_low_b"]);

        let water = report.frontiers.iter()
            .find(|f| f.gate_id == "water_floor" && f.comparison_group_id == "supply_cold_low_reserve")
            .expect("water frontier should exist");
        assert_eq!(water.minimal_observed_failing_points.len(), 1);
        assert_eq!(water.minimal_observed_failing_points[0].case_id, "water_short_low");

        // A gate with no observed failure is reported explicitly; this is not
        // interpreted as proof that the gate cannot fail outside the sample.
        assert_eq!(report.gates_without_observed_breach, vec!["sanitation_floor"]);
        assert!(report.result_scope.contains("not_a_safety_certificate"));
    }

    #[test]
    fn does_not_compare_different_initial_states_or_hazard_families() {
        let report = aggregate(&fixture()).expect("fixture should aggregate");
        let comms = report.frontiers.iter()
            .find(|f| f.gate_id == "communications_floor")
            .expect("communications frontier should exist");
        assert_eq!(comms.comparison_group_id, "telemetry_restart_high_reserve");
        assert_eq!(comms.initial_state_id, "reserve_high");
    }

    #[test]
    fn rejects_severity_axis_drift_and_unknown_json_fields() {
        let mut input = fixture();
        input.cases[0].severity.remove("fuel_delay_hours");
        assert!(aggregate(&input).unwrap_err().contains("severity keys"));

        let mut value: serde_json::Value =
            serde_json::from_str(include_str!("../../fixtures/toy-boundary-search.json")).unwrap();
        value["unexpected"] = serde_json::json!("must be rejected");
        assert!(serde_json::from_value::<BoundaryInput>(value).is_err());
    }

    #[test]
    fn rejects_gate_breach_without_explicit_failed_gate() {
        let mut input = fixture();
        let case = input.cases.iter_mut().find(|c| c.case_id == "heat_short_low_a").unwrap();
        case.failed_gate_ids.clear();
        assert!(aggregate(&input).unwrap_err().contains("must name at least one failed gate"));
    }

    #[test]
    fn rejects_inconsistent_comparison_group_basis() {
        let mut input = fixture();
        let case = input.cases.iter_mut().find(|c| c.case_id == "heat_short_low_a").unwrap();
        case.initial_state_id = "different_initial_state".to_string();
        assert!(aggregate(&input).unwrap_err().contains("changes hazard family or initial state"));
    }

    #[test]
    fn rejects_evidence_claim_escalation() {
        let mut input = fixture();
        input.evidence_class = "domain_validated".to_string();
        assert!(aggregate(&input).unwrap_err().contains("synthetic_only"));
    }
}
