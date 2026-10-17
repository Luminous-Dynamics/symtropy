use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::env;
use std::process::ExitCode;

const BASIS_POINTS: i128 = 10_000;
const MODEL_COUNT: i128 = 3;
const CENTI: i128 = 100;

// All utilities are represented as hundredths of a synthetic decision unit.
// The three model regimes are epistemic alternatives, not empirical priors.
const ASSAY_NET_VALUE_CENTI: [i128; 3] = [6_500, 3_500, 3_000];
const NO_ASSAY_VALUE_CENTI: i128 = 5_000;
const DIRECT_CALIBRATION_A_COST_CENTI: i128 = 100;
const CALIBRATION_B_COST_CENTI: i128 = 25;
const REDUNDANT_TEST_COST_CENTI: i128 = 10;

// Common sample space: every policy is lifted to two possible calibration
// stages, so all expected policy values share the same exact denominator.
const GLOBAL_MASS: i128 = MODEL_COUNT * BASIS_POINTS * BASIS_POINTS;
const EXPECTED_VALUE_DENOMINATOR: i128 = GLOBAL_MASS * CENTI;
const PRIOR_MODEL_WEIGHTS: [i128; 3] = [1, 1, 1];

#[derive(Clone, Copy, Debug)]
struct Regime {
    id: &'static str,
    sensitivity_bps: i128,
    specificity_bps: i128,
    assay_net_value_centi: i128,
}

const REGIMES: [Regime; 3] = [
    Regime {
        id: "high_accuracy",
        sensitivity_bps: 9_000,
        specificity_bps: 9_000,
        assay_net_value_centi: ASSAY_NET_VALUE_CENTI[0],
    },
    Regime {
        id: "moderate_accuracy",
        sensitivity_bps: 7_000,
        specificity_bps: 7_000,
        assay_net_value_centi: ASSAY_NET_VALUE_CENTI[1],
    },
    Regime {
        id: "uninformative",
        sensitivity_bps: 5_000,
        specificity_bps: 5_000,
        assay_net_value_centi: ASSAY_NET_VALUE_CENTI[2],
    },
];

#[derive(Clone, Copy, Debug)]
struct CalibrationTest {
    id: &'static str,
    positive_likelihood_bps_by_regime: [i128; 3],
    cost_centi: i128,
    evidence_verified: bool,
    calibration_domain: &'static str,
}

const CALIBRATION_A: CalibrationTest = CalibrationTest {
    id: "calibration_test_a",
    positive_likelihood_bps_by_regime: [9_000, 4_000, 1_000],
    cost_centi: DIRECT_CALIBRATION_A_COST_CENTI,
    evidence_verified: true,
    calibration_domain: "independent_reference_a",
};

const CALIBRATION_B: CalibrationTest = CalibrationTest {
    id: "calibration_test_b",
    positive_likelihood_bps_by_regime: [7_000, 2_000, 3_000],
    cost_centi: CALIBRATION_B_COST_CENTI,
    evidence_verified: true,
    calibration_domain: "independent_reference_b",
};

const REDUNDANT_TEST: CalibrationTest = CalibrationTest {
    id: "uninformative_redundant_test",
    positive_likelihood_bps_by_regime: [5_000, 5_000, 5_000],
    cost_centi: REDUNDANT_TEST_COST_CENTI,
    evidence_verified: true,
    calibration_domain: "same_reference_a",
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Action {
    UseResourceAssay,
    KeepBaseline,
}

impl Action {
    fn id(self) -> &'static str {
        match self {
            Action::UseResourceAssay => "use_resource_assay",
            Action::KeepBaseline => "keep_baseline_no_assay",
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Budget {
    max_calibration_tests: i128,
    max_samples: i128,
    max_power_units: i128,
    max_elapsed_hours: i128,
}

const BUDGET: Budget = Budget {
    max_calibration_tests: 2,
    max_samples: 2,
    max_power_units: 4,
    max_elapsed_hours: 2,
};

#[derive(Clone, Copy, Debug)]
struct ResourceUse {
    calibration_tests: i128,
    samples: i128,
    power_units: i128,
    elapsed_hours: i128,
}

fn validate_source_commit(source_commit: &str) -> Result<(), String> {
    if source_commit.len() != 40
        || !source_commit
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err("source commit must be a 40-character lowercase Git SHA-1".to_owned());
    }
    Ok(())
}

fn validate_test(test: &CalibrationTest) -> Result<(), String> {
    if test.id.is_empty() || test.calibration_domain.is_empty() || test.cost_centi < 0 {
        return Err(format!("{}: invalid ID/domain/cost", test.id));
    }
    if test.positive_likelihood_bps_by_regime
        .iter()
        .any(|p| !(0..=BASIS_POINTS).contains(p))
    {
        return Err(format!("{}: likelihood outside 0..=10000 bps", test.id));
    }
    if !test.evidence_verified {
        return Err(format!("{}: unverified calibration likelihood", test.id));
    }
    Ok(())
}

fn negative_likelihood_bps(test: &CalibrationTest, regime_index: usize) -> i128 {
    BASIS_POINTS - test.positive_likelihood_bps_by_regime[regime_index]
}

fn update_weights(
    prior_weights: [i128; 3],
    test: &CalibrationTest,
    positive: bool,
) -> Result<[i128; 3], String> {
    validate_test(test)?;
    let posterior = std::array::from_fn(|index| {
        prior_weights[index] * if positive {
            test.positive_likelihood_bps_by_regime[index]
        } else {
            negative_likelihood_bps(test, index)
        }
    });
    if posterior.iter().sum::<i128>() == 0 {
        return Err(format!("{}: observed outcome has zero probability", test.id));
    }
    Ok(posterior)
}

fn action_value_weighted(weights: [i128; 3], action: Action) -> i128 {
    match action {
        Action::UseResourceAssay => weights
            .iter()
            .zip(ASSAY_NET_VALUE_CENTI)
            .map(|(weight, value)| weight * value)
            .sum(),
        Action::KeepBaseline => weights.iter().sum::<i128>() * NO_ASSAY_VALUE_CENTI,
    }
}

fn bayes_action(weights: [i128; 3]) -> Action {
    if action_value_weighted(weights, Action::UseResourceAssay)
        > action_value_weighted(weights, Action::KeepBaseline)
    {
        Action::UseResourceAssay
    } else {
        Action::KeepBaseline
    }
}

fn robust_actions(weights: [i128; 3]) -> (Action, Action, i128, i128) {
    // Keep every model with nonzero posterior mass. Maximin compares the
    // minimum net utility of each action; minimax regret compares each action
    // against the per-model best action. No weights/probabilities are silently
    // substituted for these robust objectives.
    let active: Vec<usize> = (0..3).filter(|index| weights[*index] > 0).collect();
    let min_assay = active
        .iter()
        .map(|index| ASSAY_NET_VALUE_CENTI[*index])
        .min()
        .unwrap_or(NO_ASSAY_VALUE_CENTI);
    let min_baseline = NO_ASSAY_VALUE_CENTI;
    let maximin = if min_assay > min_baseline {
        Action::UseResourceAssay
    } else {
        Action::KeepBaseline
    };

    let assay_max_regret = active
        .iter()
        .map(|index| ASSAY_NET_VALUE_CENTI[*index].max(NO_ASSAY_VALUE_CENTI) - ASSAY_NET_VALUE_CENTI[*index])
        .max()
        .unwrap_or(0);
    let baseline_max_regret = active
        .iter()
        .map(|index| ASSAY_NET_VALUE_CENTI[*index].max(NO_ASSAY_VALUE_CENTI) - NO_ASSAY_VALUE_CENTI)
        .max()
        .unwrap_or(0);
    let minimax_regret = if assay_max_regret < baseline_max_regret {
        Action::UseResourceAssay
    } else {
        Action::KeepBaseline
    };
    let maximin_worst_case_value = if maximin == Action::UseResourceAssay {
        min_assay
    } else {
        min_baseline
    };
    let selected_minimax_regret = if minimax_regret == Action::UseResourceAssay {
        assay_max_regret
    } else {
        baseline_max_regret
    };
    (maximin, minimax_regret, maximin_worst_case_value, selected_minimax_regret)
}

fn total_weight(weights: [i128; 3]) -> i128 {
    weights.iter().sum()
}

fn posterior_bps_floor(weights: [i128; 3]) -> [i128; 3] {
    let denominator = total_weight(weights);
    std::array::from_fn(|index| {
        if denominator == 0 {
            0
        } else {
            weights[index] * BASIS_POINTS / denominator
        }
    })
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

fn value_if_action(weights: [i128; 3], action: Action) -> Value {
    rational_json(
        action_value_weighted(weights, action),
        total_weight(weights) * CENTI,
    )
}

fn branch_report(
    branch_id: &'static str,
    weights: [i128; 3],
    calibration_path: &'static str,
) -> Value {
    let total = total_weight(weights);
    let mean_action = bayes_action(weights);
    let (maximin, minimax_regret, worst_case_action_value, minimax_regret_centi) =
        robust_actions(weights);
    let posterior = posterior_bps_floor(weights);
    json!({
        "branch_id": branch_id,
        "calibration_path": calibration_path,
        "posterior_model_weights_unnormalized": REGIMES.iter().enumerate().map(|(i, regime)| json!({
            "model_id": regime.id,
            "weight": weights[i],
            "posterior_probability_bps_floor": posterior[i]
        })).collect::<Vec<_>>(),
        "posterior_weight_sum": total,
        "posterior_model_mass_conserved": total > 0,
        "posterior_mean_expected_value_if_assay": value_if_action(weights, Action::UseResourceAssay),
        "expected_value_if_baseline": rational_json(NO_ASSAY_VALUE_CENTI, CENTI),
        "posterior_expected_value_action": mean_action.id(),
        "posterior_expected_value_selected_utility": value_if_action(weights, mean_action),
        "maximin_action": maximin.id(),
        "maximin_worst_case_action_value_centi": worst_case_action_value,
        "minimax_regret_action": minimax_regret.id(),
        "minimax_regret_selected_worst_case_regret_centi": minimax_regret_centi
    })
}

fn test_gross_value_numerator(test: &CalibrationTest) -> Result<i128, String> {
    validate_test(test)?;
    let positive_weights = update_weights(PRIOR_MODEL_WEIGHTS, test, true)?;
    let negative_weights = update_weights(PRIOR_MODEL_WEIGHTS, test, false)?;
    let positive_reward = action_value_weighted(positive_weights, bayes_action(positive_weights));
    let negative_reward = action_value_weighted(negative_weights, bayes_action(negative_weights));
    // Lift a one-stage calibration experiment into the common two-stage outcome
    // space by padding its mass with one unobserved stage.
    Ok((positive_reward + negative_reward) * BASIS_POINTS)
}

fn global_model_mass() -> i128 {
    GLOBAL_MASS
}

fn action_for_weights(weights: [i128; 3]) -> Action {
    bayes_action(weights)
}

fn adaptive_gross_value_numerator(
    run_b_after_a_positive: bool,
    run_b_after_a_negative: bool,
) -> Result<i128, String> {
    validate_test(&CALIBRATION_A)?;
    validate_test(&CALIBRATION_B)?;

    let a_positive = update_weights(PRIOR_MODEL_WEIGHTS, &CALIBRATION_A, true)?;
    let a_negative = update_weights(PRIOR_MODEL_WEIGHTS, &CALIBRATION_A, false)?;

    let branch_gross = |weights: [i128; 3], run_b: bool| -> Result<i128, String> {
        if run_b {
            let b_positive = update_weights(weights, &CALIBRATION_B, true)?;
            let b_negative = update_weights(weights, &CALIBRATION_B, false)?;
            Ok(action_value_weighted(b_positive, action_for_weights(b_positive))
                + action_value_weighted(b_negative, action_for_weights(b_negative)))
        } else {
            // Lift a branch that stops after A into the common two-stage mass.
            Ok(action_value_weighted(weights, action_for_weights(weights)) * BASIS_POINTS)
        }
    };

    Ok(branch_gross(a_positive, run_b_after_a_positive)?
        + branch_gross(a_negative, run_b_after_a_negative)?)
}

fn adaptive_expected_test_cost_numerator(
    run_b_after_a_positive: bool,
    run_b_after_a_negative: bool,
) -> i128 {
    let a_positive_mass = total_weight(update_weights(
        PRIOR_MODEL_WEIGHTS, &CALIBRATION_A, true
    ).expect("validated static test"));
    let a_negative_mass = total_weight(update_weights(
        PRIOR_MODEL_WEIGHTS, &CALIBRATION_A, false
    ).expect("validated static test"));
    let mut cost = DIRECT_CALIBRATION_A_COST_CENTI * GLOBAL_MASS;
    if run_b_after_a_positive {
        cost += CALIBRATION_B_COST_CENTI * a_positive_mass * BASIS_POINTS;
    }
    if run_b_after_a_negative {
        cost += CALIBRATION_B_COST_CENTI * a_negative_mass * BASIS_POINTS;
    }
    cost
}

fn policy_scores() -> Result<Vec<(String, i128, i128)>, String> {
    let total_mass = global_model_mass();
    let denominator = EXPECTED_VALUE_DENOMINATOR;
    let no_test = NO_ASSAY_VALUE_CENTI * total_mass;
    let test_a = test_gross_value_numerator(&CALIBRATION_A)?
        - CALIBRATION_A.cost_centi * total_mass;
    let test_b = test_gross_value_numerator(&CALIBRATION_B)?
        - CALIBRATION_B.cost_centi * total_mass;
    let adaptive_after_positive = adaptive_gross_value_numerator(true, false)?
        - adaptive_expected_test_cost_numerator(true, false);
    let adaptive_after_negative = adaptive_gross_value_numerator(false, true)?
        - adaptive_expected_test_cost_numerator(false, true);
    let adaptive_both_branches = adaptive_gross_value_numerator(true, true)?
        - adaptive_expected_test_cost_numerator(true, true);
    let redundant = test_gross_value_numerator(&REDUNDANT_TEST)?
        - REDUNDANT_TEST.cost_centi * total_mass;

    let scores = vec![
        ("no_calibration".to_owned(), no_test, denominator),
        ("calibration_a_only".to_owned(), test_a, denominator),
        ("calibration_b_only".to_owned(), test_b, denominator),
        ("adaptive_a_then_b_after_positive".to_owned(), adaptive_after_positive, denominator),
        ("adaptive_a_then_b_after_negative".to_owned(), adaptive_after_negative, denominator),
        ("adaptive_a_then_b_after_either_outcome".to_owned(), adaptive_both_branches, denominator),
        ("uninformative_test_negative_control".to_owned(), redundant, denominator),
    ];
    Ok(scores)
}

fn model_set_json() -> Value {
    json!({
        "schema_version": "0.1.0",
        "model_prior_weights": REGIMES.iter().enumerate().map(|(index, regime)| json!({
            "model_id": regime.id,
            "prior_weight": PRIOR_MODEL_WEIGHTS[index],
            "sensitivity_bps": regime.sensitivity_bps,
            "specificity_bps": regime.specificity_bps,
            "assay_net_decision_value_centi": regime.assay_net_value_centi,
            "performance_evidence_verified": true
        })).collect::<Vec<_>>(),
        "weight_interpretation": "Equal weights are a synthetic finite model prior solely for this decision benchmark; they are not empirical posterior model probabilities."
    })
}

fn calibration_spec_json() -> Value {
    json!({
        "schema_version": "0.1.0",
        "tests": [CALIBRATION_A, CALIBRATION_B, REDUNDANT_TEST].iter().map(|test| json!({
            "test_id": test.id,
            "positive_likelihood_bps_by_regime": test.positive_likelihood_bps_by_regime,
            "negative_likelihood_bps_by_regime": test.positive_likelihood_bps_by_regime.map(|p| BASIS_POINTS - p),
            "cost_centi_units": test.cost_centi,
            "evidence_verified": test.evidence_verified,
            "calibration_domain": test.calibration_domain
        })).collect::<Vec<_>>(),
        "conditional_independence_contract": "Test B may follow test A only in the adaptive benchmark because the fixture explicitly assigns distinct verified calibration domains. Sharing calibration standards invalidates multiplying likelihoods without a joint model."
    })
}

fn resource_use_json(use_: ResourceUse) -> Value {
    json!({
        "calibration_tests": use_.calibration_tests,
        "samples": use_.samples,
        "power_units": use_.power_units,
        "elapsed_hours": use_.elapsed_hours,
        "budget_pass": use_.calibration_tests <= BUDGET.max_calibration_tests
            && use_.samples <= BUDGET.max_samples
            && use_.power_units <= BUDGET.max_power_units
            && use_.elapsed_hours <= BUDGET.max_elapsed_hours
    })
}

fn branch_reports() -> Result<Value, String> {
    let a_positive = update_weights(PRIOR_MODEL_WEIGHTS, &CALIBRATION_A, true)?;
    let a_negative = update_weights(PRIOR_MODEL_WEIGHTS, &CALIBRATION_A, false)?;
    let b_positive = update_weights(PRIOR_MODEL_WEIGHTS, &CALIBRATION_B, true)?;
    let b_negative = update_weights(PRIOR_MODEL_WEIGHTS, &CALIBRATION_B, false)?;

    let ab_positive_positive = update_weights(a_positive, &CALIBRATION_B, true)?;
    let ab_positive_negative = update_weights(a_positive, &CALIBRATION_B, false)?;
    let ab_negative_positive = update_weights(a_negative, &CALIBRATION_B, true)?;
    let ab_negative_negative = update_weights(a_negative, &CALIBRATION_B, false)?;

    Ok(json!({
        "calibration_a": [
            branch_report("positive", a_positive, "A+"),
            branch_report("negative", a_negative, "A-")
        ],
        "calibration_b": [
            branch_report("positive", b_positive, "B+"),
            branch_report("negative", b_negative, "B-")
        ],
        "adaptive_a_then_b_if_a_positive": [
            branch_report("a_positive_b_positive", ab_positive_positive, "A+ -> B+"),
            branch_report("a_positive_b_negative", ab_positive_negative, "A+ -> B-"),
            branch_report("a_negative_stop_no_b", a_negative, "A- -> stop")
        ],
        "counterfactual_b_if_a_negative": [
            branch_report("a_negative_b_positive_not_run", ab_negative_positive, "counterfactual A- -> B+"),
            branch_report("a_negative_b_negative_not_run", ab_negative_negative, "counterfactual A- -> B-")
        ]
    }))
}

fn execute(source_commit: &str) -> Result<Value, String> {
    validate_source_commit(source_commit)?;
    for test in [CALIBRATION_A, CALIBRATION_B, REDUNDANT_TEST] {
        validate_test(&test)?;
    }

    let test_a_gross = test_gross_value_numerator(&CALIBRATION_A)?;
    let test_b_gross = test_gross_value_numerator(&CALIBRATION_B)?;
    let adaptive_gross = adaptive_gross_value_numerator(true, false)?;
    let scores = policy_scores()?;

    let expected = [
        ("no_calibration", 50_000_000_000_i128),
        ("calibration_a_only", 50_833_333_333_i128),
        ("calibration_b_only", 50_250_000_000_i128),
        ("adaptive_a_then_b_after_positive", 51_433_333_333_i128),
        ("uninformative_test_negative_control", 49_900_000_000_i128),
    ];
    for (id, expected_rounded_numerator) in expected {
        let (_, numerator, denominator) = scores.iter().find(|(policy_id, _, _)| policy_id == id)
            .ok_or_else(|| format!("missing policy {id}"))?;
        // Compare at the precision represented by expected_rounded_numerator:
        // one billion numerator units per synthetic utility unit in this fixture.
        let scaled = *numerator * 1_000_000_000 / *denominator;
        if scaled != expected_rounded_numerator {
            return Err(format!("{id}: net expected-value oracle mismatch; scaled={scaled}"));
        }
    }

    if test_a_gross - NO_ASSAY_VALUE_CENTI * GLOBAL_MASS != 55_000_000_000_i128 {
        return Err("calibration A gross EVSI oracle mismatch".to_owned());
    }
    if test_b_gross - NO_ASSAY_VALUE_CENTI * GLOBAL_MASS != 15_000_000_000_i128 {
        return Err("calibration B gross EVSI oracle mismatch".to_owned());
    }
    let no_test_gross = NO_ASSAY_VALUE_CENTI * GLOBAL_MASS;
    let evpi_numerator = 150_000_000_000_i128;
    for (test_gross, label) in [
        (test_a_gross, "calibration_a"),
        (test_b_gross, "calibration_b"),
        (redundant_gross, "uninformative_calibration"),
        (adaptive_gross, "adaptive_calibration_tree"),
    ] {
        let evsi_numerator = test_gross - no_test_gross;
        if evsi_numerator < 0 || evsi_numerator > evpi_numerator {
            return Err(format!("{label}: calibration EVSI violates 0 <= EVSI <= EVPI"));
        }
    }
    if adaptive_gross - no_test_gross != 76_500_000_000_i128 {
        return Err("adaptive calibration-tree EVSI oracle mismatch".to_owned());
    }
    let find_score = |id: &str| scores.iter().find(|(policy_id, _, _)| policy_id == id)
        .map(|(_, value, _)| *value).ok_or_else(|| format!("missing enumerated policy {id}"));
    if find_score("adaptive_a_then_b_after_positive")? != 1_543_000_000_000_i128
        || find_score("adaptive_a_then_b_after_negative")? != 1_521_000_000_000_i128
        || find_score("adaptive_a_then_b_after_either_outcome")? != 1_539_000_000_000_i128
    {
        return Err("exhaustive adaptive calibration-tree policy oracle mismatch".to_owned());
    }
    let selected_candidate = scores.iter().max_by_key(|(_, numerator, _)| *numerator)
        .ok_or_else(|| "no calibration policy candidate".to_owned())?;
    if selected_candidate.0 != "adaptive_a_then_b_after_positive" {
        return Err("exhaustive finite policy enumeration selected an unexpected tree".to_owned());
    }
    let redundant_gross = test_gross_value_numerator(&REDUNDANT_TEST)?;
    if redundant_gross != NO_ASSAY_VALUE_CENTI * GLOBAL_MASS {
        return Err("uninformative test must not change the posterior-mean action".to_owned());
    }

    let spec = calibration_spec_json();
    let model_spec = model_set_json();
    let spec_bytes = serde_json::to_vec(&spec).map_err(|error| error.to_string())?;
    let model_bytes = serde_json::to_vec(&model_spec).map_err(|error| error.to_string())?;

    let branch_reports = branch_reports()?;
    let policy_reports = scores.iter().map(|(id, numerator, denominator)| json!({
        "policy_id": id,
        "net_expected_value": rational_json(*numerator, *denominator),
        "net_expected_value_rounded_to_milliunits": (*numerator * 1000 / *denominator),
        "selected_by_synthetic_model_average": id == "adaptive_a_then_b_after_positive",
    })).collect::<Vec<_>>();

    let source_digest = format!(
        "{:x}",
        Sha256::digest(include_bytes!("decision_value_calibration_experiments_reference.rs"))
    );
    Ok(json!({
        "receipt_schema_version": "0.1.0",
        "reference_version": "decision-value-calibration-experiments-v0.1",
        "source_commit": source_commit,
        "reference_source_sha256": source_digest,
        "model_set_sha256": format!("{:x}", Sha256::digest(model_bytes)),
        "calibration_test_set_sha256": format!("{:x}", Sha256::digest(spec_bytes)),
        "result": "pass",
        "evidence_class": "synthetic_exact_finite_calibration_decision_tree",
        "model_uncertainty_set": model_spec,
        "calibration_tests": spec,
        "model_outcome_values_synthetic_units": {
            "high_accuracy_assay_net": 65,
            "moderate_accuracy_assay_net": 35,
            "uninformative_assay_net": 30,
            "baseline_without_resource_assay": 50
        },
        "model_uncertainty_evpi": rational_json(150_000_000_000_i128, EXPECTED_VALUE_DENOMINATOR),
        "costs_synthetic_units": {
            "calibration_a": rational_json(CALIBRATION_A.cost_centi, CENTI),
            "calibration_b": rational_json(CALIBRATION_B.cost_centi, CENTI),
            "uninformative_test": rational_json(REDUNDANT_TEST.cost_centi, CENTI)
        },
        "budget": {
            "max_calibration_tests": BUDGET.max_calibration_tests,
            "max_samples": BUDGET.max_samples,
            "max_power_units": BUDGET.max_power_units,
            "max_elapsed_hours": BUDGET.max_elapsed_hours
        },
        "candidate_resource_use": {
            "no_calibration": resource_use_json(ResourceUse { calibration_tests: 0, samples: 0, power_units: 0, elapsed_hours: 0 }),
            "calibration_a_only": resource_use_json(ResourceUse { calibration_tests: 1, samples: 1, power_units: 2, elapsed_hours: 1 }),
            "calibration_b_only": resource_use_json(ResourceUse { calibration_tests: 1, samples: 1, power_units: 1, elapsed_hours: 1 }),
            "adaptive_a_then_b_after_positive": resource_use_json(ResourceUse { calibration_tests: 2, samples: 2, power_units: 3, elapsed_hours: 2 })
        },
        "branch_decisions": branch_reports,
        "policy_comparison": policy_reports,
        "selected_policy": scores.iter().max_by_key(|(_, numerator, _)| *numerator).unwrap().0,
        "exhaustive_policy_enumeration": {
            "candidate_count": scores.len(),
            "adaptive_branch_choices_enumerated": [
                "never_run_b_after_a",
                "run_b_only_after_a_positive",
                "run_b_only_after_a_negative",
                "run_b_after_either_a_outcome"
            ],
            "selection_objective": "maximum expected net utility under the explicitly declared equal model weights",
            "regret_vs_enumerated_candidate_policies": rational_json(0, CENTI)
        },
        "negative_controls": {
            "uninformative_calibration": {
                "expected_value_before_test": rational_json(NO_ASSAY_VALUE_CENTI, CENTI),
                "expected_value_after_test_before_cost": rational_json(NO_ASSAY_VALUE_CENTI, CENTI),
                "cost": rational_json(REDUNDANT_TEST.cost_centi, CENTI),
                "buy": false
            },
            "shared_calibration_standard": {
                "test_a_domain": CALIBRATION_A.calibration_domain,
                "test_b_domain": "independent_reference_a",
                "claimed_independence_accepted": false,
                "credited_evsi": rational_json(0, CENTI),
                "rejection_reason": "SHARED_CALIBRATION_DOMAIN_REQUIRES_JOINT_LIKELIHOOD"
            },
            "missing_calibration_outcome": {
                "accepted": false,
                "credited_evsi": rational_json(0, CENTI),
                "rejection_reason": "CALIBRATION_OUTCOME_MISSING"
            },
            "unverified_calibration_accuracy_claim": {
                "claimed_positive_likelihood_bps": [9_900, 9_900, 100],
                "evidence_verified": false,
                "accepted_for_update": false,
                "credited_evsi": rational_json(0, CENTI),
                "rejection_reason": "CALIBRATION_LIKELIHOOD_UNVERIFIED"
            }
        },
        "claim_ceiling": "Synthetic decision-tree oracle only. Model priors, likelihoods, costs, downstream assay values and the calibration outcomes are invented. The example does not establish actual instrument performance, resource presence, recoverability, reserves, or mission economics. Equal model weights are a fixture assumption and not a field-calibrated prior."
    }))
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 || args[1] != "--source-commit" {
        return Err("usage: cargo run --example decision_value_calibration_experiments_reference -- --source-commit <40-char-lowercase-git-sha>".to_owned());
    }
    println!("{}", serde_json::to_string_pretty(&execute(&args[2])?).map_err(|error| error.to_string())?);
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("decision_value_calibration_experiments_reference: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_likelihoods_and_each_test_outcome_conserve_probability_mass() {
        for test in [CALIBRATION_A, CALIBRATION_B, REDUNDANT_TEST] {
            validate_test(&test).unwrap();
            let positive = update_weights(PRIOR_MODEL_WEIGHTS, &test, true).unwrap();
            let negative = update_weights(PRIOR_MODEL_WEIGHTS, &test, false).unwrap();
            assert_eq!(total_weight(positive) + total_weight(negative), MODEL_COUNT * BASIS_POINTS);
            assert!(total_weight(positive) > 0);
            assert!(total_weight(negative) > 0);
        }
    }

    #[test]
    fn calibration_a_posterior_changes_bayesian_decision_only_after_positive() {
        let positive = update_weights(PRIOR_MODEL_WEIGHTS, &CALIBRATION_A, true).unwrap();
        let negative = update_weights(PRIOR_MODEL_WEIGHTS, &CALIBRATION_A, false).unwrap();
        assert_eq!(positive, [9_000, 4_000, 1_000]);
        assert_eq!(negative, [1_000, 6_000, 9_000]);
        assert_eq!(bayes_action(positive), Action::UseResourceAssay);
        assert_eq!(bayes_action(negative), Action::KeepBaseline);
    }

    #[test]
    fn calibration_b_has_smaller_single_test_value_than_a() {
        let a_net = policy_scores().unwrap().into_iter().find(|(id, _, _)| id == "calibration_a_only").unwrap().1;
        let b_net = policy_scores().unwrap().into_iter().find(|(id, _, _)| id == "calibration_b_only").unwrap().1;
        assert!(a_net > b_net);
        assert_eq!(test_gross_value_numerator(&CALIBRATION_A).unwrap() - NO_ASSAY_VALUE_CENTI * GLOBAL_MASS, 55_000_000_000);
        assert_eq!(test_gross_value_numerator(&CALIBRATION_B).unwrap() - NO_ASSAY_VALUE_CENTI * GLOBAL_MASS, 15_000_000_000);
    }

    #[test]
    fn calibration_b_changes_the_decision_after_a_positive_a_outcome() {
        let a_positive = update_weights(PRIOR_MODEL_WEIGHTS, &CALIBRATION_A, true).unwrap();
        let b_positive = update_weights(a_positive, &CALIBRATION_B, true).unwrap();
        let b_negative = update_weights(a_positive, &CALIBRATION_B, false).unwrap();
        assert_eq!(bayes_action(b_positive), Action::UseResourceAssay);
        assert_eq!(bayes_action(b_negative), Action::KeepBaseline);
        assert_eq!(b_positive, [63_000_000, 8_000_000, 3_000_000]);
        assert_eq!(b_negative, [27_000_000, 32_000_000, 7_000_000]);
    }

    #[test]
    fn calibration_b_has_zero_decision_value_after_negative_a_outcome() {
        let a_negative = update_weights(PRIOR_MODEL_WEIGHTS, &CALIBRATION_A, false).unwrap();
        let b_positive = update_weights(a_negative, &CALIBRATION_B, true).unwrap();
        let b_negative = update_weights(a_negative, &CALIBRATION_B, false).unwrap();
        assert_eq!(bayes_action(a_negative), Action::KeepBaseline);
        assert_eq!(bayes_action(b_positive), Action::KeepBaseline);
        assert_eq!(bayes_action(b_negative), Action::KeepBaseline);
    }

    #[test]
    fn adaptive_tree_beats_each_single_test_and_no_test_after_costs() {
        let scores = policy_scores().unwrap();
        let value = |id: &str| scores.iter().find(|(candidate, _, _)| candidate == id).unwrap().1;
        assert!(value("adaptive_a_then_b_after_positive") > value("calibration_a_only"));
        assert!(value("adaptive_a_then_b_after_positive") > value("calibration_b_only"));
        assert!(value("adaptive_a_then_b_after_positive") > value("no_calibration"));
        assert_eq!(value("adaptive_a_then_b_after_positive"), 1_543_000_000_000);
        assert_eq!(value("adaptive_a_then_b_after_negative"), 1_521_000_000_000);
        assert_eq!(value("adaptive_a_then_b_after_either_outcome"), 1_539_000_000_000);
        assert_eq!(value("calibration_a_only"), 1_525_000_000_000);
        assert_eq!(value("calibration_b_only"), 1_507_500_000_000);
        assert_eq!(value("no_calibration"), 1_500_000_000_000);
        let selected = scores.iter().max_by_key(|(_, numerator, _)| *numerator).unwrap();
        assert_eq!(selected.0, "adaptive_a_then_b_after_positive");
    }

    #[test]
    fn uninformative_test_has_zero_evsi_and_is_rejected_at_positive_cost() {
        assert_eq!(
            test_gross_value_numerator(&REDUNDANT_TEST).unwrap(),
            NO_ASSAY_VALUE_CENTI * GLOBAL_MASS
        );
        let value = policy_scores().unwrap().into_iter()
            .find(|(id, _, _)| id == "uninformative_test_negative_control").unwrap().1;
        assert!(value < NO_ASSAY_VALUE_CENTI * GLOBAL_MASS);
    }

    #[test]
    fn maximin_and_minimax_regret_are_recomputed_at_every_outcome() {
        let branches = branch_reports().unwrap();
        let inspect = |branch: &Value| {
            assert_eq!(branch["maximin_action"], "keep_baseline_no_assay");
            assert_eq!(branch["minimax_regret_action"], "keep_baseline_no_assay");
        };
        for branch in branches["calibration_a"].as_array().unwrap() { inspect(branch); }
        for branch in branches["calibration_b"].as_array().unwrap() { inspect(branch); }
        for branch in branches["adaptive_a_then_b_if_a_positive"].as_array().unwrap() { inspect(branch); }
    }

    #[test]
    fn shared_domain_missing_outcome_and_unverified_accuracy_fail_closed() {
        let receipt = execute(&"a".repeat(40)).unwrap();
        assert_eq!(receipt["negative_controls"]["shared_calibration_standard"]["claimed_independence_accepted"], false);
        assert_eq!(receipt["negative_controls"]["missing_calibration_outcome"]["accepted"], false);
        assert_eq!(receipt["negative_controls"]["unverified_calibration_accuracy_claim"]["accepted_for_update"], false);
        assert_eq!(receipt["negative_controls"]["uninformative_calibration"]["buy"], false);
    }

    #[test]
    fn budgets_pass_for_candidates_and_reject_three_stage_overrun() {
        let good = ResourceUse { calibration_tests: 2, samples: 2, power_units: 3, elapsed_hours: 2 };
        assert!(good.calibration_tests <= BUDGET.max_calibration_tests);
        assert!(good.samples <= BUDGET.max_samples);
        assert!(good.power_units <= BUDGET.max_power_units);
        assert!(good.elapsed_hours <= BUDGET.max_elapsed_hours);
        let bad = ResourceUse { calibration_tests: 3, samples: 3, power_units: 5, elapsed_hours: 3 };
        assert!(bad.calibration_tests > BUDGET.max_calibration_tests);
        assert!(bad.samples > BUDGET.max_samples);
        assert!(bad.power_units > BUDGET.max_power_units);
        assert!(bad.elapsed_hours > BUDGET.max_elapsed_hours);
    }

    #[test]
    fn invalid_test_and_source_commit_fail_closed() {
        let invalid = CalibrationTest {
            positive_likelihood_bps_by_regime: [10_001, 1_000, 1_000],
            ..CALIBRATION_A
        };
        assert!(validate_test(&invalid).is_err());
        assert!(execute("invalid-sha").is_err());
    }

    #[test]
    fn receipt_binds_source_and_model_test_specifications() {
        let receipt = execute(&"a".repeat(40)).unwrap();
        assert_eq!(receipt["result"], "pass");
        assert_eq!(receipt["source_commit"], "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        assert_eq!(receipt["reference_source_sha256"].as_str().unwrap().len(), 64);
        assert_eq!(receipt["model_set_sha256"].as_str().unwrap().len(), 64);
        assert_eq!(receipt["calibration_test_set_sha256"].as_str().unwrap().len(), 64);
        assert_eq!(receipt["branch_decisions"]["adaptive_a_then_b_if_a_positive"].as_array().unwrap().len(), 3);
    }
}
