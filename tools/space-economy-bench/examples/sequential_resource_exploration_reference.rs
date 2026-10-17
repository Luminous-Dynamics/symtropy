use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::env;
use std::process::ExitCode;

const BASIS_POINTS: i128 = 10_000;
const PROBABILITY_DENOMINATOR: i128 = BASIS_POINTS * BASIS_POINTS * BASIS_POINTS;

const PRIOR_RICH_BPS: i128 = 5_000;
const BUILD_PAYOFF_RICH: i128 = 200;
const BUILD_PAYOFF_POOR: i128 = -100;
const IMPORT_PAYOFF: i128 = 0;

#[derive(Clone, Copy, Debug)]
struct Assay {
    assay_id: &'static str,
    sensitivity_bps: i128,
    specificity_bps: i128,
    cost_units: i128,
}

const REMOTE_SCREEN: Assay = Assay {
    assay_id: "remote_screen",
    sensitivity_bps: 7_000,
    specificity_bps: 7_000,
    cost_units: 1,
};

const HIGH_RESOLUTION_ASSAY: Assay = Assay {
    assay_id: "high_resolution_assay",
    sensitivity_bps: 9_000,
    specificity_bps: 9_000,
    cost_units: 20,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Mass {
    rich: i128,
    poor: i128,
}

impl Mass {
    fn total(self) -> i128 {
        self.rich + self.poor
    }
}

#[derive(Clone, Debug)]
struct BranchEvaluation {
    first_outcome: &'static str,
    first_stage_mass: Mass,
    first_stage_posterior_rich_bps: i128,
    gross_without_followup_numerator: i128,
    gross_with_followup_numerator: i128,
    conditional_evsi_numerator: i128,
    followup_cost_if_purchased_numerator: i128,
    purchase_followup: bool,
    positive_followup_posterior_rich_bps: i128,
    negative_followup_posterior_rich_bps: i128,
}

#[derive(Clone, Debug)]
struct PolicyEvaluation {
    branches: Vec<BranchEvaluation>,
    gross_value_numerator: i128,
    expected_survey_cost_numerator: i128,
    net_value_numerator: i128,
    baseline_value_numerator: i128,
    incremental_net_value_numerator: i128,
}

fn validate_assay(assay: &Assay) -> Result<(), String> {
    if assay.assay_id.is_empty() || assay.cost_units <= 0 {
        return Err("assay identity and positive cost are required".to_owned());
    }
    if !(0..=BASIS_POINTS).contains(&assay.sensitivity_bps)
        || !(0..=BASIS_POINTS).contains(&assay.specificity_bps)
    {
        return Err(format!(
            "{}: sensitivity/specificity must be within 0..=10000 bps",
            assay.assay_id
        ));
    }
    Ok(())
}

fn prior_mass() -> Mass {
    Mass {
        rich: PRIOR_RICH_BPS * PROBABILITY_DENOMINATOR / BASIS_POINTS,
        poor: (BASIS_POINTS - PRIOR_RICH_BPS) * PROBABILITY_DENOMINATOR / BASIS_POINTS,
    }
}

fn split_mass(mass: Mass, assay: &Assay) -> Result<(Mass, Mass), String> {
    validate_assay(assay)?;

    let rich_positive_numerator = mass.rich * assay.sensitivity_bps;
    let poor_positive_numerator = mass.poor * (BASIS_POINTS - assay.specificity_bps);
    if rich_positive_numerator % BASIS_POINTS != 0
        || poor_positive_numerator % BASIS_POINTS != 0
    {
        return Err(format!(
            "{}: fixture weights cannot be represented exactly at the fixed denominator",
            assay.assay_id
        ));
    }

    let positive = Mass {
        rich: rich_positive_numerator / BASIS_POINTS,
        poor: poor_positive_numerator / BASIS_POINTS,
    };
    let negative = Mass {
        rich: mass.rich - positive.rich,
        poor: mass.poor - positive.poor,
    };
    if positive.total() == 0 || negative.total() == 0 {
        return Err(format!(
            "{}: both outcomes must have positive probability in this reference fixture",
            assay.assay_id
        ));
    }
    Ok((positive, negative))
}

fn best_expected_reward_numerator(mass: Mass) -> i128 {
    let build_value =
        mass.rich * BUILD_PAYOFF_RICH + mass.poor * BUILD_PAYOFF_POOR;
    let import_value = mass.total() * IMPORT_PAYOFF;
    build_value.max(import_value)
}

fn posterior_rich_bps(mass: Mass) -> i128 {
    if mass.total() == 0 {
        return 0;
    }
    mass.rich * BASIS_POINTS / mass.total()
}

fn baseline_value_numerator() -> i128 {
    best_expected_reward_numerator(prior_mass())
}

fn perfect_information_value_numerator() -> i128 {
    let mass = prior_mass();
    mass.rich * BUILD_PAYOFF_RICH + mass.poor * IMPORT_PAYOFF
}

fn evaluate_branch(
    outcome: &'static str,
    first_stage_mass: Mass,
    followup: &Assay,
) -> Result<BranchEvaluation, String> {
    let gross_without_followup = best_expected_reward_numerator(first_stage_mass);
    let (positive_followup_mass, negative_followup_mass) =
        split_mass(first_stage_mass, followup)?;
    let gross_with_followup = best_expected_reward_numerator(positive_followup_mass)
        + best_expected_reward_numerator(negative_followup_mass);
    let conditional_evsi = gross_with_followup - gross_without_followup;
    let cost_if_purchased = followup.cost_units * first_stage_mass.total();

    // Compare joint expected value with joint cost. This avoids floating-point
    // conversion or rounded posterior probabilities changing the policy.
    let purchase_followup = conditional_evsi > cost_if_purchased;

    Ok(BranchEvaluation {
        first_outcome: outcome,
        first_stage_mass,
        first_stage_posterior_rich_bps: posterior_rich_bps(first_stage_mass),
        gross_without_followup_numerator: gross_without_followup,
        gross_with_followup_numerator: gross_with_followup,
        conditional_evsi_numerator: conditional_evsi,
        followup_cost_if_purchased_numerator: cost_if_purchased,
        purchase_followup,
        positive_followup_posterior_rich_bps: posterior_rich_bps(positive_followup_mass),
        negative_followup_posterior_rich_bps: posterior_rich_bps(negative_followup_mass),
    })
}

fn evaluate_adaptive_policy(
    screen: &Assay,
    followup: &Assay,
) -> Result<PolicyEvaluation, String> {
    validate_assay(screen)?;
    validate_assay(followup)?;

    let baseline = baseline_value_numerator();
    let (first_positive, first_negative) = split_mass(prior_mass(), screen)?;
    let mut branches = vec![
        evaluate_branch("positive", first_positive, followup)?,
        evaluate_branch("negative", first_negative, followup)?,
    ];

    let mut gross_value = 0_i128;
    let mut expected_survey_cost = screen.cost_units * PROBABILITY_DENOMINATOR;
    for branch in &branches {
        if branch.purchase_followup {
            gross_value += branch.gross_with_followup_numerator;
            expected_survey_cost += branch.followup_cost_if_purchased_numerator;
        } else {
            gross_value += branch.gross_without_followup_numerator;
        }
    }

    let net_value = gross_value - expected_survey_cost;
    Ok(PolicyEvaluation {
        branches: std::mem::take(&mut branches),
        gross_value_numerator: gross_value,
        expected_survey_cost_numerator: expected_survey_cost,
        net_value_numerator: net_value,
        baseline_value_numerator: baseline,
        incremental_net_value_numerator: net_value - baseline,
    })
}

fn evaluate_direct_assay(assay: &Assay) -> Result<(i128, i128, i128), String> {
    validate_assay(assay)?;
    let (positive, negative) = split_mass(prior_mass(), assay)?;
    let gross_value = best_expected_reward_numerator(positive)
        + best_expected_reward_numerator(negative);
    let net_value = gross_value - assay.cost_units * PROBABILITY_DENOMINATOR;
    let incremental_value = net_value - baseline_value_numerator();
    Ok((gross_value, net_value, incremental_value))
}

fn rational_json(numerator: i128, denominator: i128) -> Value {
    json!({
        "numerator": numerator,
        "denominator": denominator,
        "value_if_exactly_divisible": if denominator != 0 && numerator % denominator == 0 {
            Some(numerator / denominator)
        } else {
            None
        }
    })
}

fn assay_json(assay: &Assay) -> Value {
    json!({
        "assay_id": assay.assay_id,
        "sensitivity_bps": assay.sensitivity_bps,
        "specificity_bps": assay.specificity_bps,
        "cost_units": assay.cost_units
    })
}

fn branch_json(branch: &BranchEvaluation, followup: &Assay) -> Value {
    json!({
        "first_stage_outcome": branch.first_outcome,
        "first_stage_joint_mass": {
            "rich": branch.first_stage_mass.rich,
            "poor": branch.first_stage_mass.poor,
            "total": branch.first_stage_mass.total()
        },
        "posterior_rich_probability_after_first_stage_bps_floor": branch.first_stage_posterior_rich_bps,
        "expected_value_if_stopping_after_first_stage": rational_json(
            branch.gross_without_followup_numerator,
            PROBABILITY_DENOMINATOR
        ),
        "expected_value_if_followup_is_run": rational_json(
            branch.gross_with_followup_numerator,
            PROBABILITY_DENOMINATOR
        ),
        "conditional_evsi": rational_json(
            branch.conditional_evsi_numerator,
            branch.first_stage_mass.total()
        ),
        "followup_assay_cost_if_purchased": followup.cost_units,
        "expected_followup_cost_contribution_if_purchased": rational_json(
            branch.followup_cost_if_purchased_numerator,
            PROBABILITY_DENOMINATOR
        ),
        "posterior_rich_bps_floor_after_followup_positive": branch.positive_followup_posterior_rich_bps,
        "posterior_rich_bps_floor_after_followup_negative": branch.negative_followup_posterior_rich_bps,
        "purchase_followup": branch.purchase_followup
    })
}

fn execute(source_commit: &str) -> Result<Value, String> {
    if source_commit.len() != 40
        || !source_commit
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err("source commit must be a 40-character lowercase Git SHA-1".to_owned());
    }

    let baseline = baseline_value_numerator();
    let perfect_information = perfect_information_value_numerator();
    let evpi = perfect_information - baseline;
    if baseline != 50 * PROBABILITY_DENOMINATOR
        || perfect_information != 100 * PROBABILITY_DENOMINATOR
        || evpi != 50 * PROBABILITY_DENOMINATOR
    {
        return Err("baseline / perfect-information analytical oracle mismatch".to_owned());
    }

    let adaptive = evaluate_adaptive_policy(&REMOTE_SCREEN, &HIGH_RESOLUTION_ASSAY)?;
    let (direct_gross, direct_net, direct_incremental) =
        evaluate_direct_assay(&HIGH_RESOLUTION_ASSAY)?;

    let expected_selected_outcomes: Vec<&str> = adaptive
        .branches
        .iter()
        .filter(|branch| branch.purchase_followup)
        .map(|branch| branch.first_outcome)
        .collect();
    if expected_selected_outcomes != vec!["negative"] {
        return Err("adaptive policy must purchase follow-up only after a negative screen".to_owned());
    }
    if adaptive.gross_value_numerator != 78_500_000_000_000
        || adaptive.expected_survey_cost_numerator != 11_000_000_000_000
        || adaptive.net_value_numerator != 67_500_000_000_000
        || adaptive.incremental_net_value_numerator != 17_500_000_000_000
    {
        return Err("adaptive policy expected-value/cost oracle mismatch".to_owned());
    }
    if direct_gross != 85_000_000_000_000
        || direct_net != 65_000_000_000_000
        || direct_incremental != 15_000_000_000_000
        || adaptive.net_value_numerator <= direct_net
        || adaptive.net_value_numerator - direct_net != 2_500_000_000_000
    {
        return Err("adaptive-vs-direct comparison oracle mismatch".to_owned());
    }
    if adaptive.branches[0].conditional_evsi_numerator
        != 6_500_000_000_000
        || adaptive.branches[1].conditional_evsi_numerator
            != 23_500_000_000_000
    {
        return Err("conditional EVSI branch oracle mismatch".to_owned());
    }
    if adaptive.branches.iter().any(|branch| {
        branch.gross_with_followup_numerator < branch.gross_without_followup_numerator
    }) {
        return Err("follow-up information unexpectedly reduced optimized gross value".to_owned());
    }
    if adaptive
        .branches
        .iter()
        .filter(|branch| branch.purchase_followup)
        .any(|branch| {
            branch.conditional_evsi_numerator
                <= branch.followup_cost_if_purchased_numerator
        })
    {
        return Err("selected follow-up did not exceed its branch-weighted cost".to_owned());
    }

    let high_cost_followup = Assay {
        cost_units: 60,
        ..HIGH_RESOLUTION_ASSAY
    };
    let high_cost_policy = evaluate_adaptive_policy(&REMOTE_SCREEN, &high_cost_followup)?;
    if high_cost_policy.branches.iter().any(|branch| branch.purchase_followup)
        || high_cost_policy.net_value_numerator
            != 54 * PROBABILITY_DENOMINATOR
    {
        return Err("high-cost negative control should stop after the first screen".to_owned());
    }

    let source_digest = format!(
        "{:x}",
        Sha256::digest(include_bytes!("sequential_resource_exploration_reference.rs"))
    );
    Ok(json!({
        "receipt_schema_version": "0.1.0",
        "reference_version": "sequential-resource-exploration-v0.1",
        "source_commit": source_commit,
        "reference_source_sha256": source_digest,
        "result": "pass",
        "evidence_class": "synthetic_exact_rational_sequential_decision_model",
        "probability_weight_denominator": PROBABILITY_DENOMINATOR,
        "prior_rich_site_probability_bps": PRIOR_RICH_BPS,
        "decision_payoffs_synthetic_units": {
            "build_local_oxygen_if_rich": BUILD_PAYOFF_RICH,
            "build_local_oxygen_if_poor": BUILD_PAYOFF_POOR,
            "import_oxygen_either_state": IMPORT_PAYOFF
        },
        "screen_assay": assay_json(&REMOTE_SCREEN),
        "followup_assay": assay_json(&HIGH_RESOLUTION_ASSAY),
        "baseline": {
            "expected_value_without_measurement": rational_json(baseline, PROBABILITY_DENOMINATOR),
            "expected_value_with_perfect_information": rational_json(perfect_information, PROBABILITY_DENOMINATOR),
            "evpi": rational_json(evpi, PROBABILITY_DENOMINATOR)
        },
        "direct_high_resolution_assay": {
            "gross_expected_value": rational_json(direct_gross, PROBABILITY_DENOMINATOR),
            "expected_assay_cost": rational_json(HIGH_RESOLUTION_ASSAY.cost_units * PROBABILITY_DENOMINATOR, PROBABILITY_DENOMINATOR),
            "net_expected_value": rational_json(direct_net, PROBABILITY_DENOMINATOR),
            "net_incremental_value_over_no_measurement": rational_json(direct_incremental, PROBABILITY_DENOMINATOR)
        },
        "adaptive_policy": {
            "branches": adaptive.branches.iter().map(|branch| branch_json(branch, &HIGH_RESOLUTION_ASSAY)).collect::<Vec<_>>(),
            "gross_expected_value": rational_json(adaptive.gross_value_numerator, PROBABILITY_DENOMINATOR),
            "expected_survey_cost": rational_json(adaptive.expected_survey_cost_numerator, PROBABILITY_DENOMINATOR),
            "net_expected_value": rational_json(adaptive.net_value_numerator, PROBABILITY_DENOMINATOR),
            "net_incremental_value_over_no_measurement": rational_json(adaptive.incremental_net_value_numerator, PROBABILITY_DENOMINATOR),
            "net_advantage_over_direct_high_resolution_assay": rational_json(adaptive.net_value_numerator - direct_net, PROBABILITY_DENOMINATOR),
            "selected_followup_after_screen_outcomes": expected_selected_outcomes,
            "decision_rule": "Run follow-up in a branch iff branch-conditional EVSI strictly exceeds follow-up cost; select import on nonpositive build expectation."
        },
        "high_cost_negative_control": {
            "followup_cost_units": high_cost_followup.cost_units,
            "selected_followup_after_screen_outcomes": high_cost_policy.branches.iter().filter(|branch| branch.purchase_followup).map(|branch| branch.first_outcome).collect::<Vec<_>>(),
            "net_expected_value": rational_json(high_cost_policy.net_value_numerator, PROBABILITY_DENOMINATOR),
            "stops_after_screen": true
        },
        "claim_ceiling": "Synthetic decision-theory fixture only. No empirical lunar prior, assay accuracy, resource reserve, engineering yield, mission investment value, or field survey performance is established. Real use requires calibrated site-specific priors, validation of conditional likelihoods, and explicit sample/time/power/access constraints."
    }))
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 || args[1] != "--source-commit" {
        return Err(
            "usage: cargo run --example sequential_resource_exploration_reference -- --source-commit <40-char-lowercase-git-sha>"
                .to_owned(),
        );
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&execute(&args[2])?).map_err(|error| error.to_string())?
    );
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("sequential_resource_exploration_reference: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn baseline_and_perfect_information_bound_are_exact() {
        assert_eq!(baseline_value_numerator(), 50 * PROBABILITY_DENOMINATOR);
        assert_eq!(
            perfect_information_value_numerator(),
            100 * PROBABILITY_DENOMINATOR
        );
        assert_eq!(
            perfect_information_value_numerator() - baseline_value_numerator(),
            50 * PROBABILITY_DENOMINATOR
        );
    }

    #[test]
    fn screen_posteriors_are_exactly_seventy_and_thirty_percent() {
        let (positive, negative) = split_mass(prior_mass(), &REMOTE_SCREEN).unwrap();
        assert_eq!(posterior_rich_bps(positive), 7_000);
        assert_eq!(posterior_rich_bps(negative), 3_000);
    }

    #[test]
    fn followup_is_selected_only_after_negative_screen() {
        let policy = evaluate_adaptive_policy(&REMOTE_SCREEN, &HIGH_RESOLUTION_ASSAY).unwrap();
        assert!(policy.branches[0].conditional_evsi_numerator
            < policy.branches[0].followup_cost_if_purchased_numerator);
        assert!(policy.branches[1].conditional_evsi_numerator
            > policy.branches[1].followup_cost_if_purchased_numerator);
        assert!(!policy.branches[0].purchase_followup);
        assert!(policy.branches[1].purchase_followup);
    }

    #[test]
    fn branch_conditional_evsi_matches_exact_fixture() {
        let policy = evaluate_adaptive_policy(&REMOTE_SCREEN, &HIGH_RESOLUTION_ASSAY).unwrap();
        assert_eq!(policy.branches[0].conditional_evsi_numerator, 6_500_000_000_000);
        assert_eq!(policy.branches[1].conditional_evsi_numerator, 23_500_000_000_000);
        assert_eq!(policy.branches[0].first_stage_posterior_rich_bps, 7_000);
        assert_eq!(policy.branches[1].first_stage_posterior_rich_bps, 3_000);
    }

    #[test]
    fn adaptive_policy_beats_direct_high_resolution_measurement_after_cost() {
        let adaptive = evaluate_adaptive_policy(&REMOTE_SCREEN, &HIGH_RESOLUTION_ASSAY).unwrap();
        let (_, direct_net, direct_incremental) =
            evaluate_direct_assay(&HIGH_RESOLUTION_ASSAY).unwrap();
        assert_eq!(adaptive.net_value_numerator, 67_500_000_000_000);
        assert_eq!(adaptive.incremental_net_value_numerator, 17_500_000_000_000);
        assert_eq!(direct_net, 65_000_000_000_000);
        assert_eq!(direct_incremental, 15_000_000_000_000);
        assert_eq!(adaptive.net_value_numerator - direct_net, 2_500_000_000_000);
    }

    #[test]
    fn expected_followup_cost_is_probability_weighted() {
        let policy = evaluate_adaptive_policy(&REMOTE_SCREEN, &HIGH_RESOLUTION_ASSAY).unwrap();
        assert_eq!(policy.expected_survey_cost_numerator, 11_000_000_000_000);
        assert_eq!(
            policy.expected_survey_cost_numerator / PROBABILITY_DENOMINATOR,
            11
        );
    }

    #[test]
    fn expensive_followup_is_rejected_on_both_branches() {
        let expensive = Assay { cost_units: 60, ..HIGH_RESOLUTION_ASSAY };
        let policy = evaluate_adaptive_policy(&REMOTE_SCREEN, &expensive).unwrap();
        assert!(policy.branches.iter().all(|branch| !branch.purchase_followup));
        assert_eq!(policy.net_value_numerator, 54 * PROBABILITY_DENOMINATOR);
    }

    #[test]
    fn optimized_sample_value_does_not_exceed_perfect_information() {
        let evpi = perfect_information_value_numerator() - baseline_value_numerator();
        let (positive, negative) = split_mass(prior_mass(), &HIGH_RESOLUTION_ASSAY).unwrap();
        let direct_evsi = best_expected_reward_numerator(positive)
            + best_expected_reward_numerator(negative)
            - baseline_value_numerator();
        let policy = evaluate_adaptive_policy(&REMOTE_SCREEN, &HIGH_RESOLUTION_ASSAY).unwrap();
        assert!(direct_evsi <= evpi);
        assert!(policy.incremental_net_value_numerator <= evpi);
    }

    #[test]
    fn invalid_assay_and_malformed_commit_fail_closed() {
        let invalid = Assay { sensitivity_bps: 10_001, ..REMOTE_SCREEN };
        assert!(evaluate_adaptive_policy(&invalid, &HIGH_RESOLUTION_ASSAY).is_err());
        assert!(execute("not-a-commit-sha").is_err());
    }

    #[test]
    fn receipt_binds_source_and_reports_policy_selection() {
        let report = execute(&"a".repeat(40)).unwrap();
        assert_eq!(report["result"], "pass");
        assert_eq!(report["source_commit"], "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        assert_eq!(report["reference_source_sha256"].as_str().unwrap().len(), 64);
        assert_eq!(report["adaptive_policy"]["selected_followup_after_screen_outcomes"][0], "negative");
        assert_eq!(report["adaptive_policy"]["branches"][0]["purchase_followup"], false);
        assert_eq!(report["adaptive_policy"]["branches"][1]["purchase_followup"], true);
        assert_eq!(report["adaptive_policy"]["net_expected_value"]["numerator"], 67_500_000_000_000_i64);
    }
}
