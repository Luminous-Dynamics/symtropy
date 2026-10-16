use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::env;
use std::process::ExitCode;

const PROBABILITY_SCALE: i128 = 10_000;
const PROBABILITY_DENOMINATOR: i128 = PROBABILITY_SCALE * PROBABILITY_SCALE;

#[derive(Clone, Copy, Debug)]
struct SurveyCase {
    prior_rich_bp: i128,
    sensitivity_bp: i128,
    specificity_bp: i128,
    rich_build_payoff_scu: i128,
    poor_build_payoff_scu: i128,
    import_payoff_scu: i128,
    survey_cost_scu: i128,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Evaluation {
    immediate_build_expected_numerator: i128,
    no_survey_value_numerator: i128,
    with_survey_gross_value_numerator: i128,
    sample_information_value_numerator: i128,
    survey_net_incremental_value_numerator: i128,
    perfect_information_value_numerator: i128,
    purchase_survey: bool,
}

fn validate(case: SurveyCase) -> Result<(), String> {
    for (name, value) in [
        ("prior_rich_bp", case.prior_rich_bp),
        ("sensitivity_bp", case.sensitivity_bp),
        ("specificity_bp", case.specificity_bp),
    ] {
        if !(0..=PROBABILITY_SCALE).contains(&value) {
            return Err(format!("{name} must be in 0..=10000 basis points"));
        }
    }
    if case.survey_cost_scu < 0 {
        return Err("survey cost cannot be negative".to_owned());
    }
    Ok(())
}

fn max_action_value_numerator(
    rich_weight: i128,
    poor_weight: i128,
    case: SurveyCase,
) -> i128 {
    let build_value =
        rich_weight * case.rich_build_payoff_scu + poor_weight * case.poor_build_payoff_scu;
    let import_value = (rich_weight + poor_weight) * case.import_payoff_scu;
    build_value.max(import_value)
}

fn evaluate(case: SurveyCase) -> Result<Evaluation, String> {
    validate(case)?;

    let rich_prior = case.prior_rich_bp;
    let poor_prior = PROBABILITY_SCALE - rich_prior;
    let rich_weight_without_survey = rich_prior * PROBABILITY_SCALE;
    let poor_weight_without_survey = poor_prior * PROBABILITY_SCALE;

    let immediate_build_expected_numerator = rich_weight_without_survey
        * case.rich_build_payoff_scu
        + poor_weight_without_survey * case.poor_build_payoff_scu;
    let no_survey_value_numerator = immediate_build_expected_numerator
        .max(PROBABILITY_DENOMINATOR * case.import_payoff_scu);

    // Joint state/observation weights are exact products of basis-point probabilities.
    let rich_positive = rich_prior * case.sensitivity_bp;
    let poor_positive = poor_prior * (PROBABILITY_SCALE - case.specificity_bp);
    let rich_negative = rich_prior * (PROBABILITY_SCALE - case.sensitivity_bp);
    let poor_negative = poor_prior * case.specificity_bp;

    let positive_branch_value =
        max_action_value_numerator(rich_positive, poor_positive, case);
    let negative_branch_value =
        max_action_value_numerator(rich_negative, poor_negative, case);
    let with_survey_gross_value_numerator = positive_branch_value + negative_branch_value;

    let sample_information_value_numerator =
        with_survey_gross_value_numerator - no_survey_value_numerator;
    let survey_cost_numerator = case.survey_cost_scu * PROBABILITY_DENOMINATOR;
    let survey_net_incremental_value_numerator =
        sample_information_value_numerator - survey_cost_numerator;

    // Perfect information permits a different action in each true state.
    let perfect_information_value_numerator = rich_weight_without_survey
        * case.rich_build_payoff_scu.max(case.import_payoff_scu)
        + poor_weight_without_survey
            * case.poor_build_payoff_scu.max(case.import_payoff_scu);
    if sample_information_value_numerator > perfect_information_value_numerator {
        return Err("sample information value exceeds the perfect-information upper bound".to_owned());
    }

    Ok(Evaluation {
        immediate_build_expected_numerator,
        no_survey_value_numerator,
        with_survey_gross_value_numerator,
        sample_information_value_numerator,
        survey_net_incremental_value_numerator,
        perfect_information_value_numerator,
        purchase_survey: survey_net_incremental_value_numerator > 0,
    })
}

fn rational(numerator: i128) -> Value {
    json!({
        "numerator": numerator,
        "denominator": PROBABILITY_DENOMINATOR,
        "whole_scu_if_integral": if numerator % PROBABILITY_DENOMINATOR == 0 {
            Some(numerator / PROBABILITY_DENOMINATOR)
        } else {
            None
        }
    })
}

fn is_commit_sha(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn report(case_id: &str, case: SurveyCase, result: Evaluation) -> Value {
    json!({
        "scenario_id": case_id,
        "evidence_class": "synthetic_analytical_reference",
        "input": {
            "prior_rich_bp": case.prior_rich_bp,
            "prior_poor_bp": PROBABILITY_SCALE - case.prior_rich_bp,
            "sensitivity_bp": case.sensitivity_bp,
            "specificity_bp": case.specificity_bp,
            "rich_build_payoff_scu": case.rich_build_payoff_scu,
            "poor_build_payoff_scu": case.poor_build_payoff_scu,
            "import_payoff_scu": case.import_payoff_scu,
            "survey_cost_scu": case.survey_cost_scu
        },
        "metrics": {
            "immediate_build_expected_value_scu": rational(result.immediate_build_expected_numerator),
            "best_no_survey_value_scu": rational(result.no_survey_value_numerator),
            "with_survey_gross_value_scu": rational(result.with_survey_gross_value_numerator),
            "sample_information_value_before_cost_scu": rational(result.sample_information_value_numerator),
            "survey_net_incremental_value_scu": rational(result.survey_net_incremental_value_numerator),
            "perfect_information_value_scu": rational(result.perfect_information_value_numerator)
        },
        "decision": {
            "purchase_survey": result.purchase_survey,
            "claim_ceiling": "Exact arithmetic on a synthetic binary decision fixture; not a real lunar-resource prior, yield, survey performance, or investment valuation."
        }
    })
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 || args[1] != "--source-commit" {
        return Err("usage: cargo run --example voi_reference -- --source-commit <40-char-lowercase-git-sha>".to_owned());
    }
    let source_commit = &args[2];
    if !is_commit_sha(source_commit) {
        return Err("source commit must be a 40-character lowercase SHA-1".to_owned());
    }

    let informative = SurveyCase {
        prior_rich_bp: 4_000,
        sensitivity_bp: 9_000,
        specificity_bp: 9_000,
        rich_build_payoff_scu: 100,
        poor_build_payoff_scu: -150,
        import_payoff_scu: 0,
        survey_cost_scu: 10,
    };
    let uninformative = SurveyCase {
        sensitivity_bp: 5_000,
        specificity_bp: 5_000,
        ..informative
    };
    let informative_result = evaluate(informative)?;
    let uninformative_result = evaluate(uninformative)?;
    let source_digest = format!("{:x}", Sha256::digest(include_bytes!("voi_reference.rs")));

    let receipt = json!({
        "receipt_schema_version": "0.1.0",
        "reference_version": "voi-binary-decision-v0.1",
        "source_commit": source_commit,
        "reference_source_sha256": source_digest,
        "result": "pass",
        "scenario_count": 2,
        "claim_ceiling": "Synthetic decision-theory fixture only; no real-world space-economy or engineering claim.",
        "scenarios": [
            report("INFORMATIVE_SURVEY_EVSI", informative, informative_result),
            report("UNINFORMATIVE_SURVEY_NEGATIVE_CONTROL", uninformative, uninformative_result)
        ]
    });
    println!("{}", serde_json::to_string_pretty(&receipt).map_err(|error| error.to_string())?);
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("voi_reference: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_case() -> SurveyCase {
        SurveyCase {
            prior_rich_bp: 4_000,
            sensitivity_bp: 9_000,
            specificity_bp: 9_000,
            rich_build_payoff_scu: 100,
            poor_build_payoff_scu: -150,
            import_payoff_scu: 0,
            survey_cost_scu: 10,
        }
    }

    #[test]
    fn informative_survey_has_exact_net_value_of_seventeen() {
        let result = evaluate(base_case()).expect("fixture is valid");
        assert_eq!(result.immediate_build_expected_numerator, -5_000_000_000);
        assert_eq!(result.no_survey_value_numerator, 0);
        assert_eq!(result.sample_information_value_numerator, 2_700_000_000);
        assert_eq!(result.survey_net_incremental_value_numerator, 1_700_000_000);
        assert_eq!(result.perfect_information_value_numerator, 4_000_000_000);
        assert!(result.purchase_survey);
    }

    #[test]
    fn uninformative_survey_is_not_purchased_at_positive_cost() {
        let case = SurveyCase {
            sensitivity_bp: 5_000,
            specificity_bp: 5_000,
            ..base_case()
        };
        let result = evaluate(case).expect("fixture is valid");
        assert_eq!(result.sample_information_value_numerator, 0);
        assert_eq!(result.survey_net_incremental_value_numerator, -1_000_000_000);
        assert!(!result.purchase_survey);
    }

    #[test]
    fn survey_value_cannot_exceed_perfect_information() {
        let result = evaluate(base_case()).expect("fixture is valid");
        assert!(result.sample_information_value_numerator <= result.perfect_information_value_numerator);
    }

    #[test]
    fn invalid_probability_and_negative_cost_fail_closed() {
        let invalid_probability = SurveyCase { prior_rich_bp: 10_001, ..base_case() };
        assert!(evaluate(invalid_probability).is_err());

        let negative_cost = SurveyCase { survey_cost_scu: -1, ..base_case() };
        assert!(evaluate(negative_cost).is_err());
    }
}
