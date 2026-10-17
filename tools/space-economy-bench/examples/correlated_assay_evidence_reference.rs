use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::env;
use std::process::ExitCode;

const BASIS_POINTS: i128 = 10_000;
const PROBABILITY_DENOMINATOR: i128 = BASIS_POINTS * BASIS_POINTS;
const PRIOR_RICH_BPS: i128 = 5_000;
const BUILD_PAYOFF_RICH: i128 = 200;
const BUILD_PAYOFF_POOR: i128 = -100;
const IMPORT_PAYOFF: i128 = 0;
const ASSAY_A_COST: i128 = 4;
const ASSAY_B_COST: i128 = 5;
const ASSAY_C_COST: i128 = 5;

const OUTCOMES_SINGLE: &[&str] = &["positive", "negative"];
const OUTCOMES_PAIR: &[&str] = &[
    "positive_positive",
    "positive_negative",
    "negative_positive",
    "negative_negative",
];

#[derive(Clone, Copy, Debug)]
struct LikelihoodModel {
    model_id: &'static str,
    outcomes: &'static [&'static str],
    rich_likelihood_bps: &'static [i128],
    poor_likelihood_bps: &'static [i128],
    description: &'static str,
}

const SINGLE_ASSAY: LikelihoodModel = LikelihoodModel {
    model_id: "assay_a_single",
    outcomes: OUTCOMES_SINGLE,
    rich_likelihood_bps: &[8_000, 2_000],
    poor_likelihood_bps: &[2_000, 8_000],
    description: "One assay with 80% sensitivity and 80% specificity.",
};

// Exact mixture: with probability 1/2, a shared calibration mode forces both
// observations to use one common latent result; with probability 1/2, their
// results are conditionally independent given the resource state. Both
// marginals still have 80% sensitivity and 80% specificity.
const CORRELATED_REPEAT: LikelihoodModel = LikelihoodModel {
    model_id: "assay_a_plus_shared_calibration_assay_b",
    outcomes: OUTCOMES_PAIR,
    rich_likelihood_bps: &[7_200, 800, 800, 1_200],
    poor_likelihood_bps: &[1_200, 800, 800, 7_200],
    description: "Two nominal 80% assays with an explicit 50% shared-calibration mode; joint likelihoods replace a false independence assumption.",
};

// Deliberately wrong shadow model: it multiplies marginal likelihoods as if the
// two measurements were independent despite sharing calibration and sampling
// domains. This model is diagnostic-only and must never enter policy selection.
const NAIVE_INDEPENDENCE: LikelihoodModel = LikelihoodModel {
    model_id: "negative_control_naive_independence",
    outcomes: OUTCOMES_PAIR,
    rich_likelihood_bps: &[6_400, 1_600, 1_600, 400],
    poor_likelihood_bps: &[400, 1_600, 1_600, 6_400],
    description: "Intentionally misspecified model: multiplies marginal probabilities despite shared calibration and sample domain.",
};

// Assay C has 90% sensitivity/specificity in this synthetic oracle. Its
// likelihood is conditionally independent of assay A only because this fixture
// explicitly supplies a separate calibration and sampling domain plus verified
// dependency metadata; domain labels alone never prove independence in reality.
const COMPLEMENTARY_ASSAY: LikelihoodModel = LikelihoodModel {
    model_id: "assay_a_plus_complementary_assay_c",
    outcomes: OUTCOMES_PAIR,
    rich_likelihood_bps: &[7_200, 800, 1_800, 200],
    poor_likelihood_bps: &[200, 1_800, 800, 7_200],
    description: "80% assay A combined with a synthetic 90% assay C and an explicitly verified distinct failure-domain contract.",
};

const NO_MEASUREMENT: LikelihoodModel = LikelihoodModel {
    model_id: "no_measurement_baseline",
    outcomes: &["no_observation"],
    rich_likelihood_bps: &[10_000],
    poor_likelihood_bps: &[10_000],
    description: "The decision baseline before purchasing a measurement.",
};

#[derive(Clone, Copy, Debug)]
struct OutcomeEvaluation {
    name: &'static str,
    rich_joint_weight: i128,
    poor_joint_weight: i128,
    posterior_rich_bps_floor: i128,
    build_value_numerator: i128,
    selected_reward_numerator: i128,
}

#[derive(Clone, Debug)]
struct ModelEvaluation {
    model_id: &'static str,
    outcomes: Vec<OutcomeEvaluation>,
    gross_value_numerator: i128,
    baseline_value_numerator: i128,
    evsi_numerator: i128,
    evpi_numerator: i128,
}

#[derive(Clone, Copy, Debug)]
struct EvidenceProvenance {
    calibration_domain_a: &'static str,
    calibration_domain_b: &'static str,
    spatial_domain_a: &'static str,
    spatial_domain_b: &'static str,
    calibration_verified: bool,
    sample_representativeness_verified: bool,
    dependence_metadata_verified: bool,
    result_observed: bool,
    assumes_conditional_independence: bool,
}

fn provenance_for(
    calibration_domain_a: &'static str,
    calibration_domain_b: &'static str,
    spatial_domain_a: &'static str,
    spatial_domain_b: &'static str,
    calibration_verified: bool,
    sample_representativeness_verified: bool,
    dependence_metadata_verified: bool,
    result_observed: bool,
    assumes_conditional_independence: bool,
) -> EvidenceProvenance {
    EvidenceProvenance {
        calibration_domain_a,
        calibration_domain_b,
        spatial_domain_a,
        spatial_domain_b,
        calibration_verified,
        sample_representativeness_verified,
        dependence_metadata_verified,
        result_observed,
        assumes_conditional_independence,
    }
}

const BASELINE_PROVENANCE: EvidenceProvenance = EvidenceProvenance {
    calibration_domain_a: "none",
    calibration_domain_b: "none",
    spatial_domain_a: "none",
    spatial_domain_b: "none",
    calibration_verified: true,
    sample_representativeness_verified: true,
    dependence_metadata_verified: true,
    result_observed: true,
    assumes_conditional_independence: false,
};

const SINGLE_ASSAY_PROVENANCE: EvidenceProvenance = EvidenceProvenance {
    calibration_domain_a: "cal_standard_a",
    calibration_domain_b: "cal_standard_a",
    spatial_domain_a: "site_patch_a",
    spatial_domain_b: "site_patch_a",
    calibration_verified: true,
    sample_representativeness_verified: true,
    dependence_metadata_verified: true,
    result_observed: true,
    assumes_conditional_independence: false,
};

const CORRELATED_PROVENANCE: EvidenceProvenance = EvidenceProvenance {
    calibration_domain_a: "cal_standard_shared",
    calibration_domain_b: "cal_standard_shared",
    spatial_domain_a: "site_patch_shared",
    spatial_domain_b: "site_patch_shared",
    calibration_verified: true,
    sample_representativeness_verified: true,
    dependence_metadata_verified: true,
    result_observed: true,
    assumes_conditional_independence: false,
};

const NAIVE_PROVENANCE: EvidenceProvenance = EvidenceProvenance {
    calibration_domain_a: "cal_standard_shared",
    calibration_domain_b: "cal_standard_shared",
    spatial_domain_a: "site_patch_shared",
    spatial_domain_b: "site_patch_shared",
    calibration_verified: true,
    sample_representativeness_verified: true,
    dependence_metadata_verified: true,
    result_observed: true,
    assumes_conditional_independence: true,
};

const COMPLEMENTARY_PROVENANCE: EvidenceProvenance = EvidenceProvenance {
    calibration_domain_a: "cal_standard_a",
    calibration_domain_b: "cal_standard_c",
    spatial_domain_a: "site_patch_a",
    spatial_domain_b: "site_patch_c",
    calibration_verified: true,
    sample_representativeness_verified: true,
    dependence_metadata_verified: true,
    result_observed: true,
    assumes_conditional_independence: true,
};

#[derive(Clone, Copy, Debug)]
struct ResourceUse {
    assay_count: i128,
    sample_units: i128,
    power_units: i128,
    elapsed_hours: i128,
}

#[derive(Clone, Copy, Debug)]
struct ResourceBudget {
    max_assays: i128,
    max_sample_units: i128,
    max_power_units: i128,
    max_elapsed_hours: i128,
}

const BUDGET: ResourceBudget = ResourceBudget {
    max_assays: 2,
    max_sample_units: 2,
    max_power_units: 4,
    max_elapsed_hours: 2,
};

const ZERO_USE: ResourceUse = ResourceUse {
    assay_count: 0,
    sample_units: 0,
    power_units: 0,
    elapsed_hours: 0,
};

const ONE_ASSAY_USE: ResourceUse = ResourceUse {
    assay_count: 1,
    sample_units: 1,
    power_units: 2,
    elapsed_hours: 1,
};

const TWO_ASSAY_USE: ResourceUse = ResourceUse {
    assay_count: 2,
    sample_units: 2,
    power_units: 4,
    elapsed_hours: 2,
};

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

fn validate_model(model: &LikelihoodModel) -> Result<(), String> {
    if model.model_id.is_empty()
        || model.outcomes.is_empty()
        || model.outcomes.len() != model.rich_likelihood_bps.len()
        || model.outcomes.len() != model.poor_likelihood_bps.len()
    {
        return Err(format!("{}: outcome arrays have invalid shape", model.model_id));
    }
    for value in model
        .rich_likelihood_bps
        .iter()
        .chain(model.poor_likelihood_bps.iter())
    {
        if !(0..=BASIS_POINTS).contains(value) {
            return Err(format!("{}: likelihood outside 0..=10000 bps", model.model_id));
        }
    }
    if model.rich_likelihood_bps.iter().sum::<i128>() != BASIS_POINTS
        || model.poor_likelihood_bps.iter().sum::<i128>() != BASIS_POINTS
    {
        return Err(format!("{}: conditional probability mass does not sum to 10000 bps", model.model_id));
    }
    Ok(())
}

fn validate_provenance(provenance: &EvidenceProvenance) -> Result<(), &'static str> {
    if !provenance.result_observed {
        return Err("RESULT_MISSING");
    }
    if !provenance.calibration_verified {
        return Err("CALIBRATION_UNVERIFIED");
    }
    if !provenance.sample_representativeness_verified {
        return Err("SAMPLE_NOT_REPRESENTATIVE");
    }
    if !provenance.dependence_metadata_verified {
        return Err("DEPENDENCE_METADATA_MISSING");
    }
    if provenance.assumes_conditional_independence
        && (provenance.calibration_domain_a == provenance.calibration_domain_b
            || provenance.spatial_domain_a == provenance.spatial_domain_b)
    {
        return Err("INDEPENDENCE_CLAIM_CONFLICTS_WITH_SHARED_FAILURE_DOMAIN");
    }
    Ok(())
}

fn resource_use_fits(use_: ResourceUse, budget: ResourceBudget) -> bool {
    use_.assay_count >= 0
        && use_.sample_units >= 0
        && use_.power_units >= 0
        && use_.elapsed_hours >= 0
        && use_.assay_count <= budget.max_assays
        && use_.sample_units <= budget.max_sample_units
        && use_.power_units <= budget.max_power_units
        && use_.elapsed_hours <= budget.max_elapsed_hours
}

fn prior_rich_joint_weight() -> i128 {
    PRIOR_RICH_BPS * BASIS_POINTS
}

fn prior_poor_joint_weight() -> i128 {
    (BASIS_POINTS - PRIOR_RICH_BPS) * BASIS_POINTS
}

fn baseline_value_numerator() -> i128 {
    let rich = prior_rich_joint_weight();
    let poor = prior_poor_joint_weight();
    let build = rich * BUILD_PAYOFF_RICH + poor * BUILD_PAYOFF_POOR;
    let import = (rich + poor) * IMPORT_PAYOFF;
    build.max(import)
}

fn perfect_information_value_numerator() -> i128 {
    prior_rich_joint_weight() * BUILD_PAYOFF_RICH
        + prior_poor_joint_weight() * IMPORT_PAYOFF
}

fn evaluate_model(model: &LikelihoodModel) -> Result<ModelEvaluation, String> {
    validate_model(model)?;
    let baseline = baseline_value_numerator();
    let evpi = perfect_information_value_numerator() - baseline;
    let mut outcomes = Vec::with_capacity(model.outcomes.len());
    let mut gross_value = 0_i128;

    for index in 0..model.outcomes.len() {
        let rich_weight = PRIOR_RICH_BPS * model.rich_likelihood_bps[index];
        let poor_weight = (BASIS_POINTS - PRIOR_RICH_BPS) * model.poor_likelihood_bps[index];
        let total_weight = rich_weight + poor_weight;
        let build = rich_weight * BUILD_PAYOFF_RICH + poor_weight * BUILD_PAYOFF_POOR;
        let import = total_weight * IMPORT_PAYOFF;
        let reward = build.max(import);
        gross_value += reward;
        outcomes.push(OutcomeEvaluation {
            name: model.outcomes[index],
            rich_joint_weight: rich_weight,
            poor_joint_weight: poor_weight,
            posterior_rich_bps_floor: if total_weight == 0 {
                0
            } else {
                rich_weight * BASIS_POINTS / total_weight
            },
            build_value_numerator: build,
            selected_reward_numerator: reward,
        });
    }

    let total_joint_weight = outcomes
        .iter()
        .map(|outcome| outcome.rich_joint_weight + outcome.poor_joint_weight)
        .sum::<i128>();
    if total_joint_weight != PROBABILITY_DENOMINATOR {
        return Err(format!(
            "{}: joint outcome mass {} does not equal denominator {}",
            model.model_id, total_joint_weight, PROBABILITY_DENOMINATOR
        ));
    }

    let evsi = gross_value - baseline;
    if evsi < 0 || evsi > evpi {
        return Err(format!("{}: EVSI violates 0 <= EVSI <= EVPI", model.model_id));
    }

    Ok(ModelEvaluation {
        model_id: model.model_id,
        outcomes,
        gross_value_numerator: gross_value,
        baseline_value_numerator: baseline,
        evsi_numerator: evsi,
        evpi_numerator: evpi,
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

fn model_evaluation_json(evaluation: &ModelEvaluation) -> Value {
    json!({
        "model_id": evaluation.model_id,
        "gross_expected_value": rational_json(evaluation.gross_value_numerator, PROBABILITY_DENOMINATOR),
        "expected_value_without_measurement": rational_json(evaluation.baseline_value_numerator, PROBABILITY_DENOMINATOR),
        "evsi": rational_json(evaluation.evsi_numerator, PROBABILITY_DENOMINATOR),
        "evpi": rational_json(evaluation.evpi_numerator, PROBABILITY_DENOMINATOR),
        "evsi_leq_evpi": evaluation.evsi_numerator <= evaluation.evpi_numerator,
        "outcomes": evaluation.outcomes.iter().map(|outcome| {
            json!({
                "outcome": outcome.name,
                "rich_joint_weight": outcome.rich_joint_weight,
                "poor_joint_weight": outcome.poor_joint_weight,
                "joint_weight_total": outcome.rich_joint_weight + outcome.poor_joint_weight,
                "posterior_rich_probability_bps_floor": outcome.posterior_rich_bps_floor,
                "build_action_value_numerator": outcome.build_value_numerator,
                "selected_reward_numerator": outcome.selected_reward_numerator,
                "selected_action": if outcome.build_value_numerator > IMPORT_PAYOFF {
                    "build_local_oxygen"
                } else {
                    "import_oxygen"
                }
            })
        }).collect::<Vec<_>>()
    })
}

fn provenance_json(provenance: &EvidenceProvenance) -> Value {
    let validation = validate_provenance(provenance);
    json!({
        "calibration_domain_a": provenance.calibration_domain_a,
        "calibration_domain_b": provenance.calibration_domain_b,
        "spatial_domain_a": provenance.spatial_domain_a,
        "spatial_domain_b": provenance.spatial_domain_b,
        "calibration_verified": provenance.calibration_verified,
        "sample_representativeness_verified": provenance.sample_representativeness_verified,
        "dependence_metadata_verified": provenance.dependence_metadata_verified,
        "result_observed": provenance.result_observed,
        "assumes_conditional_independence": provenance.assumes_conditional_independence,
        "accepted": validation.is_ok(),
        "rejection_reason": validation.err()
    })
}

fn portfolio_json(
    portfolio_id: &'static str,
    model: &ModelEvaluation,
    measurement_cost_units: i128,
    use_: ResourceUse,
    budget: ResourceBudget,
    evidence_valid: bool,
    rejection_reason: Option<&'static str>,
) -> Value {
    let budget_pass = resource_use_fits(use_, budget);
    let net_value = model.gross_value_numerator - measurement_cost_units * PROBABILITY_DENOMINATOR;
    let eligible = budget_pass && evidence_valid;

    json!({
        "portfolio_id": portfolio_id,
        "model_id": model.model_id,
        "gross_expected_value": rational_json(model.gross_value_numerator, PROBABILITY_DENOMINATOR),
        "measurement_cost_units": measurement_cost_units,
        "net_expected_value": rational_json(net_value, PROBABILITY_DENOMINATOR),
        "net_incremental_value_over_no_measurement": rational_json(net_value - model.baseline_value_numerator, PROBABILITY_DENOMINATOR),
        "evsi": rational_json(model.evsi_numerator, PROBABILITY_DENOMINATOR),
        "evpi": rational_json(model.evpi_numerator, PROBABILITY_DENOMINATOR),
        "resource_use": {
            "assay_count": use_.assay_count,
            "sample_units": use_.sample_units,
            "power_units": use_.power_units,
            "elapsed_hours": use_.elapsed_hours
        },
        "budget_pass": budget_pass,
        "evidence_valid": evidence_valid,
        "eligible_for_selection": eligible,
        "rejection_reason": rejection_reason,
        "selection_credit": if eligible {
            rational_json(net_value, PROBABILITY_DENOMINATOR)
        } else {
            Value::Null
        }
    })
}

fn control_json(
    control_id: &'static str,
    provenance: EvidenceProvenance,
    claimed_incremental_value_units: i128,
) -> Value {
    let validation = validate_provenance(&provenance);
    let accepted = validation.is_ok();
    json!({
        "control_id": control_id,
        "provenance": provenance_json(&provenance),
        "claimed_incremental_value_units": claimed_incremental_value_units,
        "credited_incremental_value_units": if accepted { claimed_incremental_value_units } else { 0 },
        "evidence_credit_rejected": !accepted,
        "rejection_reason": validation.err()
    })
}

fn execute(source_commit: &str) -> Result<Value, String> {
    validate_source_commit(source_commit)?;

    let single = evaluate_model(&SINGLE_ASSAY)?;
    let correlated = evaluate_model(&CORRELATED_REPEAT)?;
    let naive = evaluate_model(&NAIVE_INDEPENDENCE)?;
    let complementary = evaluate_model(&COMPLEMENTARY_ASSAY)?;
    let baseline = evaluate_model(&NO_MEASUREMENT)?;

    if baseline.gross_value_numerator != 5_000_000_000
        || perfect_information_value_numerator() != 10_000_000_000
    {
        return Err("baseline / EVPI exact arithmetic oracle mismatch".to_owned());
    }
    if single.gross_value_numerator != 7_000_000_000
        || single.evsi_numerator != 2_000_000_000
    {
        return Err("single assay expected value oracle mismatch".to_owned());
    }
    if correlated.gross_value_numerator != 7_400_000_000
        || correlated.evsi_numerator != 2_400_000_000
        || correlated.outcomes[0].posterior_rich_bps_floor != 8_571
    {
        return Err("correlated joint-likelihood oracle mismatch".to_owned());
    }
    if naive.gross_value_numerator != 7_800_000_000
        || naive.evsi_numerator != 2_800_000_000
        || naive.outcomes[0].posterior_rich_bps_floor != 9_411
    {
        return Err("naive independence negative-control oracle mismatch".to_owned());
    }
    if complementary.gross_value_numerator != 8_500_000_000
        || complementary.evsi_numerator != 3_500_000_000
    {
        return Err("complementary assay oracle mismatch".to_owned());
    }

    let baseline_net = baseline.gross_value_numerator;
    let single_net = single.gross_value_numerator - ASSAY_A_COST * PROBABILITY_DENOMINATOR;
    let correlated_net = correlated.gross_value_numerator
        - (ASSAY_A_COST + ASSAY_B_COST) * PROBABILITY_DENOMINATOR;
    let complementary_net = complementary.gross_value_numerator
        - (ASSAY_A_COST + ASSAY_C_COST) * PROBABILITY_DENOMINATOR;
    let naive_net = naive.gross_value_numerator
        - (ASSAY_A_COST + ASSAY_B_COST) * PROBABILITY_DENOMINATOR;

    if baseline_net != 5_000_000_000
        || single_net != 6_600_000_000
        || correlated_net != 6_500_000_000
        || complementary_net != 7_600_000_000
        || naive_net != 6_900_000_000
    {
        return Err("net portfolio value oracle mismatch".to_owned());
    }

    let correlated_provenance = validate_provenance(&CORRELATED_PROVENANCE);
    let single_provenance = validate_provenance(&SINGLE_ASSAY_PROVENANCE);
    let complementary_provenance = validate_provenance(&COMPLEMENTARY_PROVENANCE);
    let naive_provenance = validate_provenance(&NAIVE_PROVENANCE);
    if correlated_provenance.is_err() || single_provenance.is_err() || complementary_provenance.is_err() {
        return Err("a model with complete synthetic contract metadata was rejected".to_owned());
    }
    if naive_provenance.is_ok() {
        return Err("same-domain conditional-independence claim was accepted".to_owned());
    }

    // The finite candidate list is exhaustively enumerated after hard evidence
    // and resource gates. Net values use the exact probability denominator.
    let eligible_portfolios = [
        ("no_measurement", baseline_net, true),
        ("single_assay_a", single_net, single_provenance.is_ok()),
        ("correlated_repeat_a_b", correlated_net, correlated_provenance.is_ok()),
        ("complementary_assay_a_c", complementary_net, complementary_provenance.is_ok()),
    ];
    let budget_fits = [
        ("no_measurement", ZERO_USE),
        ("single_assay_a", ONE_ASSAY_USE),
        ("correlated_repeat_a_b", TWO_ASSAY_USE),
        ("complementary_assay_a_c", TWO_ASSAY_USE),
    ];
    for (candidate, use_) in budget_fits {
        if !resource_use_fits(use_, BUDGET) {
            return Err(format!("{candidate}: a declared eligible portfolio exceeded its resource budget"));
        }
    }
    let selected = eligible_portfolios
        .iter()
        .filter(|(_, _, evidence_ok)| *evidence_ok)
        .max_by_key(|(_, net, _)| *net)
        .ok_or_else(|| "no eligible portfolio survived evidence and resource gates".to_owned())?;
    if selected.0 != "complementary_assay_a_c" || selected.1 != 7_600_000_000 {
        return Err("finite exhaustive policy oracle did not choose the complementary assay".to_owned());
    }

    let over_budget_use = ResourceUse {
        assay_count: 3,
        sample_units: 3,
        power_units: 6,
        elapsed_hours: 3,
    };
    if resource_use_fits(over_budget_use, BUDGET) {
        return Err("budget negative control should have failed at least one hard limit".to_owned());
    }

    let unknown_dependence = provenance_for(
        "cal_standard_shared", "cal_standard_shared",
        "site_patch_shared", "site_patch_shared",
        true, true, false, true, true,
    );
    let uncalibrated = provenance_for(
        "cal_standard_a", "cal_standard_c",
        "site_patch_a", "site_patch_c",
        false, true, true, true, true,
    );
    let unrepresentative = provenance_for(
        "cal_standard_a", "cal_standard_c",
        "site_patch_same", "site_patch_same",
        true, false, true, true, true,
    );
    let missing_result = provenance_for(
        "cal_standard_a", "cal_standard_c",
        "site_patch_a", "site_patch_c",
        true, true, true, false, true,
    );

    let controls = vec![
        control_json("unknown_dependence", unknown_dependence, 4),
        control_json("uncalibrated_instrument", uncalibrated, 15),
        control_json("spatially_unrepresentative_sample", unrepresentative, 15),
        control_json("missing_measurement_result", missing_result, 15),
        control_json("same_standard_mislabeled_independent", NAIVE_PROVENANCE, 4),
    ];
    if controls.iter().any(|control| control["credited_incremental_value_units"] != 0) {
        return Err("an evidence-provenance negative control received information credit".to_owned());
    }

    let source_digest = format!(
        "{:x}",
        Sha256::digest(include_bytes!("correlated_assay_evidence_reference.rs"))
    );

    Ok(json!({
        "receipt_schema_version": "0.1.0",
        "reference_version": "correlated-assay-evidence-v0.1",
        "source_commit": source_commit,
        "reference_source_sha256": source_digest,
        "result": "pass",
        "evidence_class": "synthetic_exact_joint_likelihood_decision_model",
        "prior_rich_site_probability_bps": PRIOR_RICH_BPS,
        "probability_denominator": PROBABILITY_DENOMINATOR,
        "decision_payoffs_synthetic_units": {
            "build_local_oxygen_if_rich": BUILD_PAYOFF_RICH,
            "build_local_oxygen_if_poor": BUILD_PAYOFF_POOR,
            "import_oxygen_either_state": IMPORT_PAYOFF
        },
        "assay_costs_synthetic_units": {
            "assay_a": ASSAY_A_COST,
            "assay_b_shared_calibration": ASSAY_B_COST,
            "assay_c_complementary": ASSAY_C_COST
        },
        "resource_budget": {
            "max_assays": BUDGET.max_assays,
            "max_sample_units": BUDGET.max_sample_units,
            "max_power_units": BUDGET.max_power_units,
            "max_elapsed_hours": BUDGET.max_elapsed_hours
        },
        "latent_shared_calibration_mode": {
            "probability_bps": 5_000,
            "description": "In the shared mode both A/B observations use one common latent binary result; in the remaining half they are conditionally independent given the rich/poor site state. This creates a verified joint likelihood rather than two independent evidence claims."
        },
        "baseline": {
            "model": model_evaluation_json(&baseline),
            "expected_value_without_measurement": rational_json(baseline.gross_value_numerator, PROBABILITY_DENOMINATOR),
            "expected_value_with_perfect_information": rational_json(perfect_information_value_numerator(), PROBABILITY_DENOMINATOR),
            "evpi": rational_json(perfect_information_value_numerator() - baseline.gross_value_numerator, PROBABILITY_DENOMINATOR)
        },
        "model_comparisons": {
            "single_assay_a": {
                "model": model_evaluation_json(&single),
                "portfolio": portfolio_json("single_assay_a", &single, ASSAY_A_COST, ONE_ASSAY_USE, BUDGET, single_provenance.is_ok(), single_provenance.err())
            },
            "shared_calibration_joint_model_a_b": {
                "model": model_evaluation_json(&correlated),
                "provenance": provenance_json(&CORRELATED_PROVENANCE),
                "portfolio": portfolio_json("correlated_repeat_a_b", &correlated, ASSAY_A_COST + ASSAY_B_COST, TWO_ASSAY_USE, BUDGET, correlated_provenance.is_ok(), correlated_provenance.err())
            },
            "naive_independence_negative_control": {
                "model": model_evaluation_json(&naive),
                "provenance": provenance_json(&NAIVE_PROVENANCE),
                "model_implied_net_expected_value_if_wrongly_credited": rational_json(naive_net, PROBABILITY_DENOMINATOR),
                "accepted_for_policy_selection": false,
                "credited_incremental_value_units": 0,
                "reason": "The same calibration standard and sample domain contradict the asserted conditional independence. The naive posterior/value is diagnostic only."
            },
            "complementary_assay_a_c": {
                "model": model_evaluation_json(&complementary),
                "provenance": provenance_json(&COMPLEMENTARY_PROVENANCE),
                "portfolio": portfolio_json("complementary_assay_a_c", &complementary, ASSAY_A_COST + ASSAY_C_COST, TWO_ASSAY_USE, BUDGET, complementary_provenance.is_ok(), complementary_provenance.err())
            }
        },
        "portfolio_exhaustive_selection": {
            "enumerated_feasible_evidence_accepted_portfolios": eligible_portfolios.iter().filter(|(_, _, evidence_ok)| *evidence_ok).map(|(id, net, _)| json!({
                "portfolio_id": id,
                "net_expected_value": rational_json(*net, PROBABILITY_DENOMINATOR)
            })).collect::<Vec<_>>(),
            "selected_portfolio": selected.0,
            "selected_net_expected_value": rational_json(selected.1, PROBABILITY_DENOMINATOR),
            "zero_regret_vs_finite_enumeration": true
        },
        "negative_controls": {
            "provenance_controls": controls,
            "three_assay_budget_overrun": {
                "resource_use": {
                    "assay_count": over_budget_use.assay_count,
                    "sample_units": over_budget_use.sample_units,
                    "power_units": over_budget_use.power_units,
                    "elapsed_hours": over_budget_use.elapsed_hours
                },
                "budget_pass": false,
                "selection_credit": null
            }
        },
        "claim_ceiling": "Synthetic contract only. It does not establish actual survey accuracy, geological priors, resource reserves, sample representativeness in a field setting, mission economics or physical feasibility. Domain labels alone do not prove independence. Empirical use requires calibrated likelihoods, measured joint/dependence data, representative site/epoch samples, and independent held-out validation."
    }))
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 || args[1] != "--source-commit" {
        return Err(
            "usage: cargo run --example correlated_assay_evidence_reference -- --source-commit <40-char-lowercase-git-sha>"
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
            eprintln!("correlated_assay_evidence_reference: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finite_probability_tables_conserve_mass() {
        for model in [SINGLE_ASSAY, CORRELATED_REPEAT, NAIVE_INDEPENDENCE, COMPLEMENTARY_ASSAY, NO_MEASUREMENT] {
            validate_model(&model).unwrap();
            let evaluated = evaluate_model(&model).unwrap();
            let mass = evaluated.outcomes.iter()
                .map(|outcome| outcome.rich_joint_weight + outcome.poor_joint_weight)
                .sum::<i128>();
            assert_eq!(mass, PROBABILITY_DENOMINATOR);
            assert!(evaluated.evsi_numerator >= 0);
            assert!(evaluated.evsi_numerator <= evaluated.evpi_numerator);
        }
    }

    #[test]
    fn single_assay_matches_expected_value_oracle() {
        let result = evaluate_model(&SINGLE_ASSAY).unwrap();
        assert_eq!(result.gross_value_numerator, 7_000_000_000);
        assert_eq!(result.evsi_numerator, 2_000_000_000);
        assert_eq!(result.outcomes[0].posterior_rich_bps_floor, 8_000);
        assert_eq!(result.outcomes[1].posterior_rich_bps_floor, 2_000);
    }

    #[test]
    fn shared_calibration_joint_model_prevents_naive_posterior_inflation() {
        let correlated = evaluate_model(&CORRELATED_REPEAT).unwrap();
        let naive = evaluate_model(&NAIVE_INDEPENDENCE).unwrap();
        assert_eq!(correlated.gross_value_numerator, 7_400_000_000);
        assert_eq!(correlated.evsi_numerator, 2_400_000_000);
        assert_eq!(correlated.outcomes[0].posterior_rich_bps_floor, 8_571);
        assert_eq!(naive.gross_value_numerator, 7_800_000_000);
        assert_eq!(naive.evsi_numerator, 2_800_000_000);
        assert_eq!(naive.outcomes[0].posterior_rich_bps_floor, 9_411);
        assert_eq!(naive.gross_value_numerator - correlated.gross_value_numerator, 400_000_000);
    }

    #[test]
    fn complementary_assay_has_higher_fixture_value_than_shared_repeat() {
        let repeated = evaluate_model(&CORRELATED_REPEAT).unwrap();
        let complementary = evaluate_model(&COMPLEMENTARY_ASSAY).unwrap();
        assert_eq!(repeated.evsi_numerator, 2_400_000_000);
        assert_eq!(complementary.gross_value_numerator, 8_500_000_000);
        assert_eq!(complementary.evsi_numerator, 3_500_000_000);
        assert_eq!(complementary.outcomes[1].selected_reward_numerator, 0);
        assert_eq!(complementary.outcomes[2].selected_reward_numerator, 1_400_000_000);
    }

    #[test]
    fn same_domain_independence_claim_fails_closed() {
        assert_eq!(
            validate_provenance(&NAIVE_PROVENANCE),
            Err("INDEPENDENCE_CLAIM_CONFLICTS_WITH_SHARED_FAILURE_DOMAIN")
        );
    }

    #[test]
    fn distinct_domain_independence_requires_complete_metadata() {
        assert!(validate_provenance(&COMPLEMENTARY_PROVENANCE).is_ok());
        let missing = provenance_for(
            "cal_a", "cal_b", "site_a", "site_b",
            true, true, false, true, true,
        );
        assert_eq!(validate_provenance(&missing), Err("DEPENDENCE_METADATA_MISSING"));
    }

    #[test]
    fn uncalibrated_unrepresentative_and_missing_results_receive_no_evidence_credit() {
        let controls = [
            provenance_for("a", "b", "x", "y", false, true, true, true, true),
            provenance_for("a", "b", "x", "x", true, false, true, true, true),
            provenance_for("a", "b", "x", "y", true, true, true, false, true),
        ];
        for control in controls {
            assert!(validate_provenance(&control).is_err());
            assert_eq!(if validate_provenance(&control).is_ok() { 15 } else { 0 }, 0);
        }
    }

    #[test]
    fn sample_power_and_elapsed_time_budgets_are_hard_gates() {
        assert!(resource_use_fits(ONE_ASSAY_USE, BUDGET));
        assert!(resource_use_fits(TWO_ASSAY_USE, BUDGET));
        let overrun = ResourceUse {
            assay_count: 3, sample_units: 3, power_units: 6, elapsed_hours: 3,
        };
        assert!(!resource_use_fits(overrun, BUDGET));
    }

    #[test]
    fn exhaustive_policy_selects_complementary_assay_with_zero_finite_fixture_regret() {
        let baseline = evaluate_model(&NO_MEASUREMENT).unwrap().gross_value_numerator;
        let single = evaluate_model(&SINGLE_ASSAY).unwrap().gross_value_numerator
            - ASSAY_A_COST * PROBABILITY_DENOMINATOR;
        let correlated = evaluate_model(&CORRELATED_REPEAT).unwrap().gross_value_numerator
            - (ASSAY_A_COST + ASSAY_B_COST) * PROBABILITY_DENOMINATOR;
        let complementary = evaluate_model(&COMPLEMENTARY_ASSAY).unwrap().gross_value_numerator
            - (ASSAY_A_COST + ASSAY_C_COST) * PROBABILITY_DENOMINATOR;

        let candidates = [
            ("no_measurement", baseline),
            ("single_assay_a", single),
            ("correlated_repeat_a_b", correlated),
            ("complementary_assay_a_c", complementary),
        ];
        let selected = candidates.iter().max_by_key(|candidate| candidate.1).unwrap();
        assert_eq!(selected.0, "complementary_assay_a_c");
        assert_eq!(selected.1, 7_600_000_000);
        assert_eq!(single, 6_600_000_000);
        assert_eq!(correlated, 6_500_000_000);
    }

    #[test]
    fn malformed_likelihood_mass_and_invalid_commit_fail_closed() {
        let malformed = LikelihoodModel {
            rich_likelihood_bps: &[7_000, 2_000],
            ..SINGLE_ASSAY
        };
        assert!(validate_model(&malformed).is_err());
        assert!(validate_source_commit("not-a-git-sha").is_err());
    }

    #[test]
    fn receipt_binds_source_and_exposes_claim_ceiling() {
        let receipt = execute(&"a".repeat(40)).unwrap();
        assert_eq!(receipt["result"], "pass");
        assert_eq!(receipt["source_commit"], "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        assert_eq!(receipt["reference_source_sha256"].as_str().unwrap().len(), 64);
        assert_eq!(receipt["portfolio_exhaustive_selection"]["selected_portfolio"], "complementary_assay_a_c");
        assert_eq!(receipt["negative_controls"]["provenance_controls"].as_array().unwrap().len(), 5);
        assert_eq!(receipt["model_comparisons"]["shared_calibration_joint_model_a_b"]["model"]["gross_expected_value"]["numerator"], 7_400_000_000_i64);
    }
}
