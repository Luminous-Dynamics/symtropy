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

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct SeverityAxis {
    id: String,
    unit: String,
    /// True means larger values represent more severe disturbances; false means smaller values do.
    higher_is_more_severe: bool,
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
    axes: Vec<SeverityAxis>,
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
    observed_pass_count: usize,
    observed_breach_count: usize,
    invalid_case_count: usize,
    not_evaluated_case_count: usize,
    valid_coverage_fraction: f64,
    status: &'static str,
    /// Pareto-maximal observed pass points; not a proof of a safe region.
    maximal_observed_passing_points: Vec<BoundaryPoint>,
    /// Pareto-minimal observed failures for this gate.
    minimal_observed_failing_points: Vec<BoundaryPoint>,
    /// Passes strictly more severe than observed failures on every declared axis.
    /// These expose monotonicity conflicts; they are not automatically invalid results.
    monotonicity_conflict_witnesses: Vec<MonotonicityConflictWitness>,
}

#[derive(Debug, Serialize)]
struct MonotonicityConflictWitness {
    passing_case_id: String,
    failing_case_id: String,
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

fn dominates(a: &BoundaryCase, b: &BoundaryCase, axes: &[SeverityAxis]) -> bool {
    // Only compare cases explicitly placed in the same comparison group. The
    // input validator guarantees that a group has one hazard family and initial state.
    if a.comparison_group_id != b.comparison_group_id {
        return false;
    }
    let no_more_severe = axes.iter().all(|axis| {
        if axis.higher_is_more_severe {
            a.severity[&axis.id] <= b.severity[&axis.id]
        } else {
            a.severity[&axis.id] >= b.severity[&axis.id]
        }
    });
    let strictly_less_severe = axes.iter().any(|axis| {
        if axis.higher_is_more_severe {
            a.severity[&axis.id] < b.severity[&axis.id]
        } else {
            a.severity[&axis.id] > b.severity[&axis.id]
        }
    });
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

    let mut axis_set = BTreeSet::new();
    for axis in &input.axes {
        if !nonempty(&axis.id) || !nonempty(&axis.unit) {
            return Err("severity axes require non-empty IDs and units".to_string());
        }
        if !axis_set.insert(axis.id.clone()) {
            return Err(format!("duplicate severity axis ID {}", axis.id));
        }
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

    // Produce a report for every gate × comparison group, including groups with
    // no observed breach. This gives a sampled pass frontier as well as failures.
    // Different hazard families or initial states are deliberately incomparable.
    let mut comparison_groups: BTreeMap<String, Vec<&BoundaryCase>> = BTreeMap::new();
    for case in &input.cases {
        comparison_groups
            .entry(case.comparison_group_id.clone())
            .or_default()
            .push(case);
    }

    let mut frontiers = Vec::with_capacity(input.gate_ids.len() * comparison_groups.len());
    for gate_id in &input.gate_ids {
        for (comparison_group_id, cases) in &comparison_groups {
            let passing: Vec<&BoundaryCase> = cases
                .iter()
                .copied()
                .filter(|case| {
                    !matches!(case.outcome, Outcome::Invalid | Outcome::NotEvaluated)
                        && !case.failed_gate_ids.contains(gate_id)
                })
                .collect();
            let failing: Vec<&BoundaryCase> = cases
                .iter()
                .copied()
                .filter(|case| {
                    case.outcome == Outcome::GateBreach
                        && case.failed_gate_ids.contains(gate_id)
                })
                .collect();

            // A pass point is maximal if no strictly more severe observed pass
            // dominates it. An observed pass frontier is not a certified safe set.
            let maximal_passing: Vec<&BoundaryCase> = passing
                .iter()
                .copied()
                .filter(|candidate| {
                    !passing.iter().copied().any(|other| {
                        other.case_id != candidate.case_id
                            && dominates(candidate, other, &input.axes)
                    })
                })
                .collect();

            // For failures, retain the Pareto-minimal observed points.
            let minimal_failing: Vec<&BoundaryCase> = failing
                .iter()
                .copied()
                .filter(|candidate| {
                    !failing.iter().copied().any(|other| {
                        other.case_id != candidate.case_id
                            && dominates(other, candidate, &input.axes)
                    })
                })
                .collect();

            // Report rather than assume monotonicity: a pass at a strictly more
            // severe vector than a failing point is a witness against a simple
            // componentwise monotone failure boundary for this observed sample.
            let mut conflicts = Vec::new();
            for pass_case in &passing {
                for fail_case in &failing {
                    if dominates(fail_case, pass_case, &input.axes) {
                        conflicts.push(MonotonicityConflictWitness {
                            passing_case_id: pass_case.case_id.clone(),
                            failing_case_id: fail_case.case_id.clone(),
                        });
                    }
                }
            }
            conflicts.sort_by(|a, b| {
                a.passing_case_id.cmp(&b.passing_case_id)
                    .then_with(|| a.failing_case_id.cmp(&b.failing_case_id))
            });

            let mut maximal_observed_passing_points: Vec<BoundaryPoint> = maximal_passing
                .iter()
                .map(|case| BoundaryPoint {
                    case_id: case.case_id.clone(),
                    replicate_id: case.replicate_id.clone(),
                    severity: case.severity.clone(),
                })
                .collect();
            maximal_observed_passing_points.sort_by(|a, b| a.case_id.cmp(&b.case_id));

            let mut minimal_observed_failing_points: Vec<BoundaryPoint> = minimal_failing
                .iter()
                .map(|case| BoundaryPoint {
                    case_id: case.case_id.clone(),
                    replicate_id: case.replicate_id.clone(),
                    severity: case.severity.clone(),
                })
                .collect();
            minimal_observed_failing_points.sort_by(|a, b| a.case_id.cmp(&b.case_id));

            let observed_pass_count = passing.len();
            let observed_breach_count = failing.len();
            let invalid_case_count = cases.iter().filter(|case| case.outcome == Outcome::Invalid).count();
            let not_evaluated_case_count = cases.iter().filter(|case| case.outcome == Outcome::NotEvaluated).count();
            let valid_coverage_fraction =
                (observed_pass_count + observed_breach_count) as f64 / cases.len() as f64;
            let status = if invalid_case_count > 0 || not_evaluated_case_count > 0 {
                "incomplete_due_to_invalid_or_not_evaluated_cases"
            } else if observed_breach_count > 0 {
                "observed_breach_boundary_in_supplied_cases"
            } else {
                "no_breach_observed_in_supplied_finite_sample"
            };
            let first = cases.first().ok_or_else(|| {
                format!("internal error: comparison group {comparison_group_id} is empty")
            })?;

            frontiers.push(GateFrontier {
                gate_id: gate_id.clone(),
                comparison_group_id: comparison_group_id.clone(),
                hazard_family: first.hazard_family.clone(),
                initial_state_id: first.initial_state_id.clone(),
                observed_pass_count,
                observed_breach_count,
                invalid_case_count,
                not_evaluated_case_count,
                valid_coverage_fraction,
                status,
                maximal_observed_passing_points,
                minimal_observed_failing_points,
                monotonicity_conflict_witnesses: conflicts,
            });
        }
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
        schema_version: "0.2.0",
        report_kind: "polar-resilience-boundary-search-report",
        evidence_class: "synthetic_only",
        search_partition_id: input.search_partition_id.clone(),
        axes: input.axes.iter().map(|axis| SeverityAxis {
            id: axis.id.clone(),
            unit: axis.unit.clone(),
            higher_is_more_severe: axis.higher_is_more_severe,
        }).collect(),
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
        assert_eq!(report.schema_version, "0.2.0");
        assert_eq!(report.evidence_class, "synthetic_only");
        assert_eq!(report.case_count, 9);
        assert_eq!(report.frontiers.len(), 8);
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
        assert_eq!(heat.observed_pass_count, 2);
        assert_eq!(heat.observed_breach_count, 4);
        assert_eq!(heat.invalid_case_count, 1);
        assert_eq!(heat.not_evaluated_case_count, 1);
        assert_eq!(heat.valid_coverage_fraction, 0.75);
        assert_eq!(heat.maximal_observed_passing_points.len(), 1);
        assert_eq!(heat.maximal_observed_passing_points[0].case_id, "water_short_low");

        let water = report.frontiers.iter()
            .find(|f| f.gate_id == "water_floor" && f.comparison_group_id == "supply_cold_low_reserve")
            .expect("water frontier should exist");
        assert_eq!(water.minimal_observed_failing_points.len(), 1);
        assert_eq!(water.minimal_observed_failing_points[0].case_id, "water_short_low");
        assert!(water.monotonicity_conflict_witnesses.iter().any(|w| {
            w.passing_case_id == "heat_short_low_b" && w.failing_case_id == "water_short_low"
        }));

        let sanitation = report.frontiers.iter()
            .find(|f| f.gate_id == "sanitation_floor" && f.comparison_group_id == "supply_cold_low_reserve")
            .expect("sanitation group should be reported even with no failure");
        assert!(sanitation.minimal_observed_failing_points.is_empty());
        assert_eq!(sanitation.status, "incomplete_due_to_invalid_or_not_evaluated_cases");
        assert_eq!(sanitation.observed_pass_count, 6);

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

    #[test]
    fn supports_axes_where_smaller_values_are_more_severe() {
        let axis = SeverityAxis {
            id: "temperature_margin".to_string(),
            unit: "normalized_margin".to_string(),
            higher_is_more_severe: false,
        };
        let make_case = |case_id: &str, value: f64| BoundaryCase {
            case_id: case_id.to_string(),
            comparison_group_id: "same-envelope".to_string(),
            hazard_family: "cold-start".to_string(),
            initial_state_id: "same-reserve".to_string(),
            replicate_id: "replicate-1".to_string(),
            severity: BTreeMap::from([("temperature_margin".to_string(), value)]),
            outcome: Outcome::GateBreach,
            failed_gate_ids: vec!["heat_floor".to_string()],
        };
        let milder = make_case("milder", 8.0);
        let harsher = make_case("harsher", 2.0);
        assert!(dominates(&milder, &harsher, &[axis]));
    }

}
