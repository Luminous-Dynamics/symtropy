use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs, process,
};

const CLAIM_CEILING: &str = "Synthetic set-based value-of-information benchmark. Scenario probabilities are not assumed; reported regret reduction is gross and is not ROI, a measurement purchase recommendation, field performance or production authorization.";

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Fixture {
    schema_version: String,
    benchmark_id: String,
    evidence_class: String,
    currency: String,
    scenarios: Vec<Scenario>,
    actions: Vec<Action>,
    measurements: Vec<Measurement>,
    claim_ceiling: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Scenario {
    scenario_id: String,
    action_effects: Vec<ActionEffect>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ActionEffect {
    action_id: String,
    incremental_good_units_per_hour: f64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Action {
    action_id: String,
    label: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Measurement {
    measurement_id: String,
    label: String,
    evidence_class: String,
    cost_amount: f64,
    currency: String,
    duration_hours: f64,
    observations: Vec<Observation>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Observation {
    scenario_id: String,
    outcome: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct Report {
    schema_version: String,
    benchmark_id: String,
    source_fixture_sha256: String,
    scenario_count: usize,
    action_count: usize,
    measurement_count: usize,
    objective: String,
    no_information_policy: PolicySummary,
    measurements: Vec<MeasurementResult>,
    all_inputs_are_synthetic: bool,
    scenario_probabilities_used: bool,
    expected_value_reported: bool,
    roi_computed: bool,
    production_release_authorized: bool,
    claim_ceiling: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct PolicySummary {
    selected_action_id: String,
    worst_case_regret_units_per_hour: f64,
    action_worst_case_regret: Vec<ActionRisk>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct ActionRisk {
    action_id: String,
    worst_case_regret_units_per_hour: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct MeasurementResult {
    measurement_id: String,
    label: String,
    cost_amount: f64,
    currency: String,
    duration_hours: f64,
    current_worst_case_regret_units_per_hour: f64,
    post_measurement_worst_case_regret_units_per_hour: f64,
    gross_worst_case_regret_reduction_units_per_hour: f64,
    outcome_decisions: Vec<OutcomeDecision>,
    frontier_member: bool,
    evidence_class: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct OutcomeDecision {
    outcome: String,
    scenario_ids: Vec<String>,
    selected_action_id: String,
    worst_case_regret_units_per_hour: f64,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("factory value-of-information benchmark rejected input: {error}");
        process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        return Err("usage: ev_factory_value_of_information <synthetic-voi-fixture.json>".into());
    }

    let fixture_bytes = fs::read(&args[1])
        .map_err(|error| format!("cannot read value-of-information fixture: {error}"))?;
    let fixture: Fixture = serde_json::from_slice(&fixture_bytes)
        .map_err(|error| format!("value-of-information fixture JSON/schema invalid: {error}"))?;
    validate_fixture(&fixture)?;

    let digest = format!("{:x}", Sha256::digest(&fixture_bytes));
    let report = evaluate(&fixture, digest)?;
    let output = serde_json::to_string_pretty(&report)
        .map_err(|error| format!("cannot serialize report: {error}"))?;
    println!("{output}");
    Ok(())
}

fn validate_fixture(fixture: &Fixture) -> Result<(), String> {
    if fixture.schema_version != "0.1.0"
        || fixture.benchmark_id != "EV-FACTORY-VOI-001"
        || fixture.evidence_class != "synthetic"
        || fixture.currency.trim().is_empty()
        || fixture.claim_ceiling.trim().is_empty()
        || fixture.scenarios.len() < 2
        || fixture.actions.len() < 2
        || fixture.measurements.is_empty()
    {
        return Err("fixture_failed_synthetic_schema_contract".into());
    }

    let mut scenario_ids = BTreeSet::new();
    for scenario in &fixture.scenarios {
        if scenario.scenario_id.trim().is_empty()
            || !scenario_ids.insert(scenario.scenario_id.clone())
        {
            return Err("scenario_ids_must_be_nonempty_and_unique".into());
        }
    }

    let mut action_ids = BTreeSet::new();
    for action in &fixture.actions {
        if action.action_id.trim().is_empty()
            || action.label.trim().is_empty()
            || !action_ids.insert(action.action_id.clone())
        {
            return Err("action_ids_and_labels_must_be_nonempty_and_action_ids_unique".into());
        }
    }

    for scenario in &fixture.scenarios {
        let mut seen_actions = BTreeSet::new();
        for effect in &scenario.action_effects {
            if !action_ids.contains(&effect.action_id)
                || !seen_actions.insert(effect.action_id.clone())
                || !effect.incremental_good_units_per_hour.is_finite()
            {
                return Err(format!(
                    "action_effects_have_unknown_duplicate_or_nonfinite_values_for_{}",
                    scenario.scenario_id
                ));
            }
        }
        if seen_actions != action_ids {
            return Err(format!(
                "action_effects_must_cover_each_action_exactly_once_for_{}",
                scenario.scenario_id
            ));
        }
    }

    let mut measurement_ids = BTreeSet::new();
    for measurement in &fixture.measurements {
        if measurement.measurement_id.trim().is_empty()
            || measurement.label.trim().is_empty()
            || measurement.evidence_class != "synthetic"
            || measurement.currency != fixture.currency
            || !measurement_ids.insert(measurement.measurement_id.clone())
            || !measurement.cost_amount.is_finite()
            || measurement.cost_amount < 0.0
            || !measurement.duration_hours.is_finite()
            || measurement.duration_hours < 0.0
        {
            return Err(format!(
                "measurement_{}_identity_cost_time_or_evidence_invalid",
                measurement.measurement_id
            ));
        }

        let mut seen_scenarios = BTreeSet::new();
        for observation in &measurement.observations {
            if !scenario_ids.contains(&observation.scenario_id)
                || observation.outcome.trim().is_empty()
                || !seen_scenarios.insert(observation.scenario_id.clone())
            {
                return Err(format!(
                    "measurement_{}_has_unknown_duplicate_or_empty_observation",
                    measurement.measurement_id
                ));
            }
        }
        if seen_scenarios != scenario_ids {
            return Err(format!(
                "measurement_{}_must_map_every_scenario_exactly_once",
                measurement.measurement_id
            ));
        }
    }
    Ok(())
}

fn evaluate(fixture: &Fixture, source_fixture_sha256: String) -> Result<Report, String> {
    let all_scenarios: BTreeSet<String> = fixture
        .scenarios
        .iter()
        .map(|scenario| scenario.scenario_id.clone())
        .collect();
    let no_information_policy = evaluate_policy(fixture, &all_scenarios)?;
    let mut measurement_results = Vec::with_capacity(fixture.measurements.len());

    let mut measurements: Vec<&Measurement> = fixture.measurements.iter().collect();
    measurements.sort_by(|a, b| a.measurement_id.cmp(&b.measurement_id));

    for measurement in measurements {
        let mut groups = BTreeMap::<String, Vec<String>>::new();
        for observation in &measurement.observations {
            groups
                .entry(observation.outcome.clone())
                .or_default()
                .push(observation.scenario_id.clone());
        }

        let mut outcome_decisions = Vec::with_capacity(groups.len());
        let mut post_measurement_regret: f64 = 0.0;
        for (outcome, mut scenario_ids) in groups {
            scenario_ids.sort();
            let scenario_set: BTreeSet<String> = scenario_ids.iter().cloned().collect();
            let policy = evaluate_policy(fixture, &scenario_set)?;
            post_measurement_regret =
                post_measurement_regret.max(policy.worst_case_regret_units_per_hour);
            outcome_decisions.push(OutcomeDecision {
                outcome,
                scenario_ids,
                selected_action_id: policy.selected_action_id,
                worst_case_regret_units_per_hour: policy.worst_case_regret_units_per_hour,
            });
        }

        let before = no_information_policy.worst_case_regret_units_per_hour;
        if post_measurement_regret > before + 1e-9 {
            return Err(format!(
                "measurement_{}_increased_worst_case_regret_after_reoptimization",
                measurement.measurement_id
            ));
        }
        let reduction = (before - post_measurement_regret).max(0.0);
        measurement_results.push(MeasurementResult {
            measurement_id: measurement.measurement_id.clone(),
            label: measurement.label.clone(),
            cost_amount: measurement.cost_amount,
            currency: measurement.currency.clone(),
            duration_hours: measurement.duration_hours,
            current_worst_case_regret_units_per_hour: before,
            post_measurement_worst_case_regret_units_per_hour: post_measurement_regret,
            gross_worst_case_regret_reduction_units_per_hour: reduction,
            outcome_decisions,
            frontier_member: false,
            evidence_class: measurement.evidence_class.clone(),
        });
    }

    for i in 0..measurement_results.len() {
        let dominated = (0..measurement_results.len())
            .any(|j| i != j && dominates(&measurement_results[j], &measurement_results[i]));
        measurement_results[i].frontier_member = !dominated;
    }

    measurement_results.sort_by(|a, b| a.measurement_id.cmp(&b.measurement_id));
    Ok(Report {
        schema_version: "0.1.0".into(),
        benchmark_id: fixture.benchmark_id.clone(),
        source_fixture_sha256,
        scenario_count: fixture.scenarios.len(),
        action_count: fixture.actions.len(),
        measurement_count: fixture.measurements.len(),
        objective: "minimize worst-case regret in incremental good units/hour; keep synthetic measurement cost and duration as separate Pareto objectives; use no scenario probabilities".into(),
        no_information_policy,
        measurements: measurement_results,
        all_inputs_are_synthetic: true,
        scenario_probabilities_used: false,
        expected_value_reported: false,
        roi_computed: false,
        production_release_authorized: false,
        claim_ceiling: CLAIM_CEILING.into(),
    })
}

fn evaluate_policy(
    fixture: &Fixture,
    scenario_ids: &BTreeSet<String>,
) -> Result<PolicySummary, String> {
    if scenario_ids.is_empty() {
        return Err("cannot_evaluate_policy_for_empty_scenario_set".into());
    }

    let scenarios: Vec<&Scenario> = fixture
        .scenarios
        .iter()
        .filter(|scenario| scenario_ids.contains(&scenario.scenario_id))
        .collect();
    if scenarios.len() != scenario_ids.len() {
        return Err("policy_scenario_set_contains_unknown_scenario".into());
    }

    let mut risks = Vec::with_capacity(fixture.actions.len());
    for action in &fixture.actions {
        let mut worst_regret: f64 = 0.0;
        for scenario in &scenarios {
            let best_effect = scenario
                .action_effects
                .iter()
                .map(|effect| effect.incremental_good_units_per_hour)
                .fold(f64::NEG_INFINITY, f64::max);
            let action_effect = scenario
                .action_effects
                .iter()
                .find(|effect| effect.action_id == action.action_id)
                .ok_or_else(|| {
                    format!(
                        "scenario_{}_missing_action_{}",
                        scenario.scenario_id, action.action_id
                    )
                })?
                .incremental_good_units_per_hour;
            worst_regret = worst_regret.max(best_effect - action_effect);
        }
        risks.push(ActionRisk {
            action_id: action.action_id.clone(),
            worst_case_regret_units_per_hour: worst_regret,
        });
    }

    risks.sort_by(|a, b| {
        a.worst_case_regret_units_per_hour
            .total_cmp(&b.worst_case_regret_units_per_hour)
            .then_with(|| a.action_id.cmp(&b.action_id))
    });
    let selected = risks
        .first()
        .ok_or_else(|| "no_actions_for_policy".to_string())?;
    Ok(PolicySummary {
        selected_action_id: selected.action_id.clone(),
        worst_case_regret_units_per_hour: selected.worst_case_regret_units_per_hour,
        action_worst_case_regret: risks,
    })
}

fn dominates(a: &MeasurementResult, b: &MeasurementResult) -> bool {
    let epsilon = 1e-9;
    a.gross_worst_case_regret_reduction_units_per_hour + epsilon
        >= b.gross_worst_case_regret_reduction_units_per_hour
        && a.cost_amount <= b.cost_amount + epsilon
        && a.duration_hours <= b.duration_hours + epsilon
        && (a.gross_worst_case_regret_reduction_units_per_hour
            > b.gross_worst_case_regret_reduction_units_per_hour + epsilon
            || a.cost_amount + epsilon < b.cost_amount
            || a.duration_hours + epsilon < b.duration_hours)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Fixture {
        serde_json::from_str(include_str!(
            "../../fixtures/ev_factory_value_of_information_v0_1.json"
        ))
        .expect("VOI fixture must parse")
    }

    #[test]
    fn fixed_inputs_are_deterministic_and_claims_stay_bounded() {
        let fixture = fixture();
        validate_fixture(&fixture).expect("fixture valid");
        let first = evaluate(&fixture, "fixed-input-digest".into()).expect("first report");
        let second = evaluate(&fixture, "fixed-input-digest".into()).expect("second report");
        assert_eq!(first, second);
        assert!(first.all_inputs_are_synthetic);
        assert!(!first.scenario_probabilities_used);
        assert!(!first.expected_value_reported);
        assert!(!first.roi_computed);
        assert!(!first.production_release_authorized);
    }

    #[test]
    fn measurement_never_increases_worst_case_regret_after_reoptimization() {
        let fixture = fixture();
        let report = evaluate(&fixture, "fixture-digest".into()).expect("report");
        for measurement in report.measurements {
            assert!(
                measurement.post_measurement_worst_case_regret_units_per_hour
                    <= measurement.current_worst_case_regret_units_per_hour + 1e-9
            );
            assert!(measurement.gross_worst_case_regret_reduction_units_per_hour >= 0.0);
        }
    }

    #[test]
    fn minimax_regret_and_measurement_reductions_match_hand_calculated_fixture() {
        let fixture = fixture();
        let report = evaluate(&fixture, "fixture-digest".into()).expect("report");
        assert!(
            (report
                .no_information_policy
                .worst_case_regret_units_per_hour
                - 1.5)
                .abs()
                < 1e-9
        );

        let by_id: BTreeMap<String, &MeasurementResult> = report
            .measurements
            .iter()
            .map(|measurement| (measurement.measurement_id.clone(), measurement))
            .collect();
        assert!(
            (by_id["cycle-time-study"].gross_worst_case_regret_reduction_units_per_hour - 1.4)
                .abs()
                < 1e-9
        );
        assert!(
            (by_id["quality-rework-audit"].gross_worst_case_regret_reduction_units_per_hour - 0.5)
                .abs()
                < 1e-9
        );
        assert!(
            by_id["supplier-delivery-audit"]
                .gross_worst_case_regret_reduction_units_per_hour
                .abs()
                < 1e-9
        );
        assert!(
            by_id["line-side-visual-audit"]
                .gross_worst_case_regret_reduction_units_per_hour
                .abs()
                < 1e-9
        );
        assert!(by_id["cycle-time-study"].frontier_member);
        assert!(by_id["quality-rework-audit"].frontier_member);
        assert!(by_id["supplier-delivery-audit"].frontier_member);
        assert!(!by_id["line-side-visual-audit"].frontier_member);
    }

    #[test]
    fn cycle_time_study_can_change_the_selected_action_by_observation() {
        let fixture = fixture();
        let report = evaluate(&fixture, "fixture-digest".into()).expect("report");
        let cycle = report
            .measurements
            .iter()
            .find(|measurement| measurement.measurement_id == "cycle-time-study")
            .expect("cycle-time measurement");
        let distinct_actions: BTreeSet<String> = cycle
            .outcome_decisions
            .iter()
            .map(|decision| decision.selected_action_id.clone())
            .collect();
        assert!(distinct_actions.len() >= 2);
        assert!(cycle.gross_worst_case_regret_reduction_units_per_hour > 0.0);
    }

    #[test]
    fn duplicate_action_effect_is_rejected() {
        let mut fixture = fixture();
        let duplicate = fixture.scenarios[0].action_effects[0].clone();
        fixture.scenarios[0].action_effects.push(duplicate);
        assert!(
            validate_fixture(&fixture)
                .expect_err("duplicate action effect rejected")
                .contains("duplicate")
        );
    }

    #[test]
    fn incomplete_measurement_observations_are_rejected() {
        let mut fixture = fixture();
        fixture.measurements[0].observations.pop();
        assert!(
            validate_fixture(&fixture)
                .expect_err("missing measurement observation rejected")
                .contains("must_map_every_scenario")
        );
    }

    #[test]
    fn non_synthetic_measurements_and_negative_costs_are_rejected() {
        let mut fixture = fixture();
        fixture.measurements[0].evidence_class = "measured".into();
        assert!(
            validate_fixture(&fixture)
                .expect_err("evidence label rejected")
                .contains("evidence_invalid")
        );

        let mut negative_cost_fixture = fixture();
        negative_cost_fixture.measurements[0].cost_amount = -1.0;
        assert!(
            validate_fixture(&negative_cost_fixture)
                .expect_err("negative cost rejected")
                .contains("cost_time_or_evidence_invalid")
        );
    }

    #[test]
    fn pareto_dominance_requires_all_objectives_no_worse() {
        let mut fixture = fixture();
        let mut report = evaluate(&fixture, "fixture-digest".into()).expect("report");
        let best = report
            .measurements
            .iter()
            .find(|measurement| measurement.measurement_id == "cycle-time-study")
            .expect("cycle-time result");
        let supplier_audit = report
            .measurements
            .iter()
            .find(|measurement| measurement.measurement_id == "supplier-delivery-audit")
            .expect("supplier audit result");
        let redundant_audit = report
            .measurements
            .iter()
            .find(|measurement| measurement.measurement_id == "line-side-visual-audit")
            .expect("line-side audit result");
        assert!(best.frontier_member);
        assert!(supplier_audit.frontier_member);
        assert_eq!(
            supplier_audit.gross_worst_case_regret_reduction_units_per_hour,
            redundant_audit.gross_worst_case_regret_reduction_units_per_hour
        );
        assert!(!redundant_audit.frontier_member);
    }
}
