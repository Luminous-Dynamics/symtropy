use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::env;
use std::process::ExitCode;

const BASIS_POINTS: i128 = 10_000;
const JOINT_DENOMINATOR: i128 = BASIS_POINTS * BASIS_POINTS;

const PRIOR_RICH_BPS: i128 = 5_000;
const BUILD_PAYOFF_RICH: i128 = 200;
const BUILD_PAYOFF_POOR: i128 = -100;
const IMPORT_PAYOFF: i128 = 0;

#[derive(Clone, Copy, Debug)]
struct Survey {
    survey_id: &'static str,
    claimed_sensitivity_bps: i128,
    claimed_specificity_bps: i128,
    cost_units: i128,
    evidence_verified: bool,
}

const SURVEYS: [Survey; 4] = [
    Survey {
        survey_id: "direct_sample_assay",
        claimed_sensitivity_bps: 9_000,
        claimed_specificity_bps: 9_000,
        cost_units: 10,
        evidence_verified: true,
    },
    Survey {
        survey_id: "low_resolution_remote_assay",
        claimed_sensitivity_bps: 6_000,
        claimed_specificity_bps: 6_000,
        cost_units: 2,
        evidence_verified: true,
    },
    Survey {
        survey_id: "uninformative_assay",
        claimed_sensitivity_bps: 5_000,
        claimed_specificity_bps: 5_000,
        cost_units: 1,
        evidence_verified: true,
    },
    Survey {
        survey_id: "unverified_high_accuracy_claim",
        claimed_sensitivity_bps: 9_000,
        claimed_specificity_bps: 9_000,
        cost_units: 1,
        evidence_verified: false,
    },
];

#[derive(Clone, Debug)]
struct Evaluation {
    survey_id: &'static str,
    evidence_verified: bool,
    effective_sensitivity_bps: i128,
    effective_specificity_bps: i128,
    cost_units: i128,
    positive_event_weight: i128,
    negative_event_weight: i128,
    posterior_rich_after_positive_bps: i128,
    posterior_rich_after_negative_bps: i128,
    choose_build_after_positive: bool,
    choose_build_after_negative: bool,
    gross_value_after_survey_numerator: i128,
    baseline_value_numerator: i128,
    evsi_numerator: i128,
    net_incremental_value_numerator: i128,
    evsi_leq_evpi: bool,
    purchase: bool,
}

fn validate_survey(survey: &Survey) -> Result<(), String> {
    if survey.survey_id.is_empty() || survey.cost_units <= 0 {
        return Err("survey identity and positive cost are required".to_owned());
    }
    if !(0..=BASIS_POINTS).contains(&survey.claimed_sensitivity_bps)
        || !(0..=BASIS_POINTS).contains(&survey.claimed_specificity_bps)
    {
        return Err(format!("{}: claimed sensitivity/specificity must be within 0..=10000 bps", survey.survey_id));
    }
    Ok(())
}

fn baseline_value_numerator() -> i128 {
    let rich_weight = PRIOR_RICH_BPS * BASIS_POINTS;
    let poor_weight = (BASIS_POINTS - PRIOR_RICH_BPS) * BASIS_POINTS;
    let build_value =
        rich_weight * BUILD_PAYOFF_RICH + poor_weight * BUILD_PAYOFF_POOR;
    build_value.max(IMPORT_PAYOFF * JOINT_DENOMINATOR)
}

fn perfect_information_value_numerator() -> i128 {
    // With perfect information, build for a rich site and choose import for a
    // poor site. Since importing has zero fixture payoff, the poor-state term
    // contributes zero.
    let rich_weight = PRIOR_RICH_BPS * BASIS_POINTS;
    rich_weight * BUILD_PAYOFF_RICH
        + (BASIS_POINTS - PRIOR_RICH_BPS) * BASIS_POINTS * IMPORT_PAYOFF
}

fn evaluate_survey(survey: &Survey) -> Result<Evaluation, String> {
    validate_survey(survey)?;

    // Unverified assay performance is deliberately replaced by an uninformative
    // likelihood model. Claimed accuracy remains visible in the receipt but
    // does not affect the accepted decision calculation.
    let effective_sensitivity = if survey.evidence_verified {
        survey.claimed_sensitivity_bps
    } else {
        BASIS_POINTS / 2
    };
    let effective_specificity = if survey.evidence_verified {
        survey.claimed_specificity_bps
    } else {
        BASIS_POINTS / 2
    };

    let rich_prior = PRIOR_RICH_BPS;
    let poor_prior = BASIS_POINTS - PRIOR_RICH_BPS;

    let positive_rich_weight = rich_prior * effective_sensitivity;
    let positive_poor_weight = poor_prior * (BASIS_POINTS - effective_specificity);
    let negative_rich_weight = rich_prior * (BASIS_POINTS - effective_sensitivity);
    let negative_poor_weight = poor_prior * effective_specificity;

    let positive_event_weight = positive_rich_weight + positive_poor_weight;
    let negative_event_weight = negative_rich_weight + negative_poor_weight;
    if positive_event_weight == 0 || negative_event_weight == 0 {
        return Err(format!("{}: this fixture requires both assay outcomes to have positive probability", survey.survey_id));
    }

    let positive_build_value =
        positive_rich_weight * BUILD_PAYOFF_RICH
            + positive_poor_weight * BUILD_PAYOFF_POOR;
    let negative_build_value =
        negative_rich_weight * BUILD_PAYOFF_RICH
            + negative_poor_weight * BUILD_PAYOFF_POOR;

    let positive_action_value =
        positive_build_value.max(IMPORT_PAYOFF * positive_event_weight);
    let negative_action_value =
        negative_build_value.max(IMPORT_PAYOFF * negative_event_weight);

    let gross_value_after_survey_numerator =
        positive_action_value + negative_action_value;
    let baseline = baseline_value_numerator();
    let evsi = gross_value_after_survey_numerator - baseline;
    let net = evsi - survey.cost_units * JOINT_DENOMINATOR;
    let evpi = perfect_information_value_numerator() - baseline;

    if evsi < 0 {
        return Err(format!("{}: value of information below the no-survey option; decision rule may not be optimal", survey.survey_id));
    }
    if evsi > evpi {
        return Err(format!("{}: EVSI exceeds the perfect-information bound", survey.survey_id));
    }

    Ok(Evaluation {
        survey_id: survey.survey_id,
        evidence_verified: survey.evidence_verified,
        effective_sensitivity_bps: effective_sensitivity,
        effective_specificity_bps: effective_specificity,
        cost_units: survey.cost_units,
        positive_event_weight,
        negative_event_weight,
        posterior_rich_after_positive_bps: positive_rich_weight * BASIS_POINTS / positive_event_weight,
        posterior_rich_after_negative_bps: negative_rich_weight * BASIS_POINTS / negative_event_weight,
        choose_build_after_positive: positive_build_value > IMPORT_PAYOFF * positive_event_weight,
        choose_build_after_negative: negative_build_value > IMPORT_PAYOFF * negative_event_weight,
        gross_value_after_survey_numerator,
        baseline_value_numerator: baseline,
        evsi_numerator: evsi,
        net_incremental_value_numerator: net,
        evsi_leq_evpi: evsi <= evpi,
        purchase: net > 0,
    })
}

fn rational_json(numerator: i128, denominator: i128) -> Value {
    json!({
        "numerator": numerator,
        "denominator": denominator,
        "value_if_exactly_divisible": if numerator % denominator == 0 {
            Some(numerator / denominator)
        } else {
            None
        }
    })
}

fn evaluation_json(
    survey: &Survey,
    result: &Evaluation,
    evpi_numerator: i128,
) -> Value {
    json!({
        "survey_id": result.survey_id,
        "claimed_sensitivity_bps": survey.claimed_sensitivity_bps,
        "claimed_specificity_bps": survey.claimed_specificity_bps,
        "evidence_verified": result.evidence_verified,
        "effective_sensitivity_used_bps": result.effective_sensitivity_bps,
        "effective_specificity_used_bps": result.effective_specificity_bps,
        "positive_outcome_joint_weight": result.positive_event_weight,
        "negative_outcome_joint_weight": result.negative_event_weight,
        "posterior_rich_site_probability_after_positive_bps": result.posterior_rich_after_positive_bps,
        "posterior_rich_site_probability_after_negative_bps": result.posterior_rich_after_negative_bps,
        "decision_after_positive": if result.choose_build_after_positive { "build_local_oxygen" } else { "import_oxygen" },
        "decision_after_negative": if result.choose_build_after_negative { "build_local_oxygen" } else { "import_oxygen" },
        "expected_value_with_survey_before_cost": rational_json(result.gross_value_after_survey_numerator, JOINT_DENOMINATOR),
        "expected_value_without_survey": rational_json(result.baseline_value_numerator, JOINT_DENOMINATOR),
        "evsi": rational_json(result.evsi_numerator, JOINT_DENOMINATOR),
        "survey_cost_units": result.cost_units,
        "net_incremental_value": rational_json(result.net_incremental_value_numerator, JOINT_DENOMINATOR),
        "evpi": rational_json(evpi_numerator, JOINT_DENOMINATOR),
        "evsi_leq_evpi": result.evsi_leq_evpi,
        "purchase_survey": result.purchase
    })
}

fn find_evaluation<'a>(
    evaluations: &'a [(Survey, Evaluation)],
    survey_id: &str,
) -> Result<&'a Evaluation, String> {
    evaluations.iter()
        .find(|(survey, _)| survey.survey_id == survey_id)
        .map(|(_, evaluation)| evaluation)
        .ok_or_else(|| format!("missing expected survey {survey_id}"))
}

fn execute(source_commit: &str) -> Result<Value, String> {
    if source_commit.len() != 40
        || !source_commit.bytes().all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err("source commit must be a 40-character lowercase Git SHA-1".to_owned());
    }

    let mut ids = BTreeSet::new();
    let mut evaluations = Vec::new();
    for survey in SURVEYS {
        if !ids.insert(survey.survey_id) {
            return Err(format!("duplicate survey ID {}", survey.survey_id));
        }
        evaluations.push((survey, evaluate_survey(&survey)?));
    }

    let baseline = baseline_value_numerator();
    let evpi = perfect_information_value_numerator() - baseline;
    if baseline != 5_000_000_000 || evpi != 5_000_000_000 {
        return Err("baseline/EVPI analytical oracle mismatch".to_owned());
    }

    let direct = find_evaluation(&evaluations, "direct_sample_assay")?;
    let low_resolution = find_evaluation(&evaluations, "low_resolution_remote_assay")?;
    let uninformative = find_evaluation(&evaluations, "uninformative_assay")?;
    let unverified = find_evaluation(&evaluations, "unverified_high_accuracy_claim")?;

    if direct.evsi_numerator != 3_500_000_000
        || direct.net_incremental_value_numerator != 2_500_000_000
        || !direct.purchase
        || !direct.evsi_leq_evpi
    {
        return Err("direct sample EVSI / net-value oracle mismatch".to_owned());
    }
    if low_resolution.evsi_numerator != 0
        || low_resolution.net_incremental_value_numerator != -200_000_000
        || low_resolution.purchase
    {
        return Err("low-resolution survey should have zero EVSI and negative net value".to_owned());
    }
    if uninformative.evsi_numerator != 0
        || uninformative.net_incremental_value_numerator != -100_000_000
        || uninformative.purchase
    {
        return Err("uninformative survey should not be purchased at positive cost".to_owned());
    }
    if unverified.evidence_verified
        || unverified.effective_sensitivity_bps != 5_000
        || unverified.effective_specificity_bps != 5_000
        || unverified.evsi_numerator != 0
        || unverified.purchase
    {
        return Err("unverified assay claim improperly received information value".to_owned());
    }

    let selected_survey = evaluations.iter()
        .filter(|(_, evaluation)| evaluation.purchase)
        .max_by_key(|(_, evaluation)| evaluation.net_incremental_value_numerator)
        .map(|(survey, _)| survey.survey_id)
        .unwrap_or("no_survey");
    if selected_survey != "direct_sample_assay" {
        return Err("selected measurement oracle mismatch".to_owned());
    }

    let source_digest =
        format!("{:x}", Sha256::digest(include_bytes!("resource_assay_selection_reference.rs")));
    Ok(json!({
        "receipt_schema_version": "0.1.0",
        "reference_version": "resource-assay-decision-value-v0.1",
        "source_commit": source_commit,
        "reference_source_sha256": source_digest,
        "result": "pass",
        "evidence_class": "synthetic_exact_rational_decision_model",
        "prior_rich_site_bps": PRIOR_RICH_BPS,
        "action_payoffs_synthetic_units": {
            "build_local_oxygen_if_rich": BUILD_PAYOFF_RICH,
            "build_local_oxygen_if_poor": BUILD_PAYOFF_POOR,
            "import_oxygen_either_state": IMPORT_PAYOFF
        },
        "expected_value_without_survey": rational_json(baseline, JOINT_DENOMINATOR),
        "evpi": rational_json(evpi, JOINT_DENOMINATOR),
        "selected_measurement": selected_survey,
        "decision_rule": "Choose the survey with the highest positive net incremental value; otherwise choose no survey. All likelihood calculations use exact integer weights and one fixed denominator.",
        "surveys": evaluations.iter().map(|(survey, evaluation)| {
            evaluation_json(survey, evaluation, evpi)
        }).collect::<Vec<_>>(),
        "claim_ceiling": "Synthetic decision-theory fixture only; no empirical lunar site prior, survey performance, recoverable resource estimate or actual project investment value."
    }))
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 || args[1] != "--source-commit" {
        return Err("usage: cargo run --example resource_assay_selection_reference -- --source-commit <40-char-lowercase-git-sha>".to_owned());
    }
    println!("{}", serde_json::to_string_pretty(&execute(&args[2])?).map_err(|e| e.to_string())?);
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("resource_assay_selection_reference: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn baseline_and_evpi_match_exact_synthetic_oracle() {
        assert_eq!(baseline_value_numerator(), 5_000_000_000);
        assert_eq!(perfect_information_value_numerator() - baseline_value_numerator(), 5_000_000_000);
    }

    #[test]
    fn direct_sample_has_positive_net_value_and_is_selected() {
        let result = evaluate_survey(&SURVEYS[0]).unwrap();
        assert_eq!(result.evsi_numerator, 3_500_000_000);
        assert_eq!(result.net_incremental_value_numerator, 2_500_000_000);
        assert_eq!(result.posterior_rich_after_positive_bps, 9_000);
        assert_eq!(result.posterior_rich_after_negative_bps, 1_000);
        assert!(result.purchase);
        let receipt = execute(&"a".repeat(40)).unwrap();
        assert_eq!(receipt["selected_measurement"], "direct_sample_assay");
    }

    #[test]
    fn low_resolution_assay_does_not_change_optimal_action() {
        let result = evaluate_survey(&SURVEYS[1]).unwrap();
        assert_eq!(result.evsi_numerator, 0);
        assert_eq!(result.net_incremental_value_numerator, -200_000_000);
        assert_eq!(result.posterior_rich_after_positive_bps, 6_000);
        assert_eq!(result.posterior_rich_after_negative_bps, 4_000);
        assert_eq!(result.choose_build_after_positive, result.choose_build_after_negative);
        assert!(!result.purchase);
    }

    #[test]
    fn uninformative_survey_is_not_bought_at_positive_cost() {
        let result = evaluate_survey(&SURVEYS[2]).unwrap();
        assert_eq!(result.evsi_numerator, 0);
        assert_eq!(result.net_incremental_value_numerator, -100_000_000);
        assert!(!result.purchase);
    }

    #[test]
    fn unverified_assay_accuracy_claim_is_replaced_by_uninformative_model() {
        let result = evaluate_survey(&SURVEYS[3]).unwrap();
        assert_eq!(result.effective_sensitivity_bps, 5_000);
        assert_eq!(result.effective_specificity_bps, 5_000);
        assert_eq!(result.evsi_numerator, 0);
        assert!(!result.purchase);
    }

    #[test]
    fn sample_value_never_exceeds_perfect_information_value() {
        let evpi = perfect_information_value_numerator() - baseline_value_numerator();
        for survey in SURVEYS {
            let result = evaluate_survey(&survey).unwrap();
            assert!(result.evsi_numerator <= evpi);
            assert!(result.evsi_leq_evpi);
        }
    }

    #[test]
    fn increasing_assay_cost_cannot_increase_net_value() {
        let base = evaluate_survey(&SURVEYS[0]).unwrap();
        let more_expensive = Survey { cost_units: SURVEYS[0].cost_units + 1, ..SURVEYS[0] };
        let raised = evaluate_survey(&more_expensive).unwrap();
        assert!(raised.net_incremental_value_numerator < base.net_incremental_value_numerator);
    }

    #[test]
    fn invalid_likelihoods_and_commit_fail_closed() {
        let invalid = Survey { claimed_sensitivity_bps: 10_001, ..SURVEYS[0] };
        assert!(evaluate_survey(&invalid).is_err());
        assert!(execute("not-a-commit-sha").is_err());
    }

    #[test]
    fn receipt_is_source_bound_and_has_four_assay_options() {
        let report = execute(&"a".repeat(40)).unwrap();
        assert_eq!(report["result"], "pass");
        assert_eq!(report["surveys"].as_array().unwrap().len(), 4);
        assert_eq!(report["reference_source_sha256"].as_str().unwrap().len(), 64);
    }
}
