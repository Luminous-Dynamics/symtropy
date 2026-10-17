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

const DIRECT_ASSAY_COST: i128 = 20;
const CALIBRATION_GATE_COST: i128 = 3;

#[derive(Clone, Copy, Debug)]
struct LikelihoodModel {
    model_id: &'static str,
    sensitivity_bps: i128,
    specificity_bps: i128,
    performance_evidence_verified: bool,
}

const MODEL_SET: [LikelihoodModel; 3] = [
    LikelihoodModel {
        model_id: "nominal_high_accuracy",
        sensitivity_bps: 9_000,
        specificity_bps: 9_000,
        performance_evidence_verified: true,
    },
    LikelihoodModel {
        model_id: "moderate_accuracy",
        sensitivity_bps: 7_000,
        specificity_bps: 7_000,
        performance_evidence_verified: true,
    },
    LikelihoodModel {
        model_id: "uninformative_limit",
        sensitivity_bps: 5_000,
        specificity_bps: 5_000,
        performance_evidence_verified: true,
    },
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Policy {
    NoMeasurement,
    AlwaysBuyAssay,
    CalibrateThenAdapt,
}

impl Policy {
    const ALL: [Policy; 3] = [
        Policy::NoMeasurement,
        Policy::AlwaysBuyAssay,
        Policy::CalibrateThenAdapt,
    ];

    fn id(self) -> &'static str {
        match self {
            Policy::NoMeasurement => "no_measurement",
            Policy::AlwaysBuyAssay => "always_buy_assay",
            Policy::CalibrateThenAdapt => "calibrate_then_adapt",
        }
    }

    fn description(self) -> &'static str {
        match self {
            Policy::NoMeasurement => "Keep the baseline decision; no assay or calibration spend.",
            Policy::AlwaysBuyAssay => "Buy the assay under every model without validating which performance regime applies.",
            Policy::CalibrateThenAdapt => "Spend 3 units on a synthetic perfect model-regime check; buy the 20-unit assay only if it identifies the 90%/90% regime, otherwise stop.",
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct ResourceUse {
    assays: i128,
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

fn validate_model(model: &LikelihoodModel) -> Result<(), String> {
    if model.model_id.is_empty() {
        return Err("model ID must not be empty".to_owned());
    }
    if !(0..=BASIS_POINTS).contains(&model.sensitivity_bps)
        || !(0..=BASIS_POINTS).contains(&model.specificity_bps)
    {
        return Err(format!("{}: likelihood outside 0..=10000 bps", model.model_id));
    }
    Ok(())
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

fn baseline_value_numerator() -> i128 {
    let rich_weight = PRIOR_RICH_BPS * BASIS_POINTS;
    let poor_weight = (BASIS_POINTS - PRIOR_RICH_BPS) * BASIS_POINTS;
    let build = rich_weight * BUILD_PAYOFF_RICH + poor_weight * BUILD_PAYOFF_POOR;
    let import = (rich_weight + poor_weight) * IMPORT_PAYOFF;
    build.max(import)
}

fn perfect_information_value_numerator() -> i128 {
    PRIOR_RICH_BPS * BASIS_POINTS * BUILD_PAYOFF_RICH
        + (BASIS_POINTS - PRIOR_RICH_BPS) * BASIS_POINTS * IMPORT_PAYOFF
}

fn outcome_weights(model: &LikelihoodModel) -> Result<[(i128, i128); 2], String> {
    validate_model(model)?;
    let rich_prior = PRIOR_RICH_BPS;
    let poor_prior = BASIS_POINTS - PRIOR_RICH_BPS;

    let positive = (
        rich_prior * model.sensitivity_bps,
        poor_prior * (BASIS_POINTS - model.specificity_bps),
    );
    let negative = (
        rich_prior * (BASIS_POINTS - model.sensitivity_bps),
        poor_prior * model.specificity_bps,
    );

    let joint_mass = positive.0 + positive.1 + negative.0 + negative.1;
    if joint_mass != PROBABILITY_DENOMINATOR {
        return Err(format!("{}: outcome mass does not conserve probability", model.model_id));
    }
    Ok([positive, negative])
}

fn best_action_reward_numerator(rich_weight: i128, poor_weight: i128) -> i128 {
    let build = rich_weight * BUILD_PAYOFF_RICH + poor_weight * BUILD_PAYOFF_POOR;
    let import = (rich_weight + poor_weight) * IMPORT_PAYOFF;
    build.max(import)
}

fn expected_value_with_assay_numerator(model: &LikelihoodModel) -> Result<i128, String> {
    let outcomes = outcome_weights(model)?;
    Ok(outcomes
        .iter()
        .map(|(rich, poor)| best_action_reward_numerator(*rich, *poor))
        .sum())
}

fn evsi_numerator(model: &LikelihoodModel) -> Result<i128, String> {
    Ok(expected_value_with_assay_numerator(model)? - baseline_value_numerator())
}

fn policy_value_numerator(policy: Policy, model: &LikelihoodModel) -> Result<i128, String> {
    validate_model(model)?;
    match policy {
        Policy::NoMeasurement => Ok(baseline_value_numerator()),
        Policy::AlwaysBuyAssay => Ok(
            expected_value_with_assay_numerator(model)? - DIRECT_ASSAY_COST * PROBABILITY_DENOMINATOR
        ),
        Policy::CalibrateThenAdapt => {
            // The perfect regime-identifying check is a deliberately idealized
            // benchmark control. Its 3-unit cost is always paid. Only the high
            // regime purchases the 20-unit assay; the others stop at baseline.
            let selected_value = if model.sensitivity_bps == 9_000
                && model.specificity_bps == 9_000
            {
                expected_value_with_assay_numerator(model)?
                    - DIRECT_ASSAY_COST * PROBABILITY_DENOMINATOR
            } else {
                baseline_value_numerator()
            };
            Ok(selected_value - CALIBRATION_GATE_COST * PROBABILITY_DENOMINATOR)
        }
    }
}

fn resource_use(policy: Policy) -> ResourceUse {
    match policy {
        Policy::NoMeasurement => ResourceUse {
            assays: 0,
            sample_units: 0,
            power_units: 0,
            elapsed_hours: 0,
        },
        Policy::AlwaysBuyAssay => ResourceUse {
            assays: 1,
            sample_units: 1,
            power_units: 2,
            elapsed_hours: 1,
        },
        Policy::CalibrateThenAdapt => ResourceUse {
            assays: 2,
            sample_units: 2,
            power_units: 4,
            elapsed_hours: 2,
        },
    }
}

fn resource_use_fits(use_: ResourceUse) -> bool {
    use_.assays >= 0
        && use_.sample_units >= 0
        && use_.power_units >= 0
        && use_.elapsed_hours >= 0
        && use_.assays <= BUDGET.max_assays
        && use_.sample_units <= BUDGET.max_sample_units
        && use_.power_units <= BUDGET.max_power_units
        && use_.elapsed_hours <= BUDGET.max_elapsed_hours
}

fn maximizer_for_model(model: &LikelihoodModel) -> Result<(Policy, i128), String> {
    let mut best: Option<(Policy, i128)> = None;
    for policy in Policy::ALL {
        let use_ = resource_use(policy);
        if !resource_use_fits(use_) {
            continue;
        }
        let value = policy_value_numerator(policy, model)?;
        if best.map(|(_, incumbent)| value > incumbent).unwrap_or(true) {
            best = Some((policy, value));
        }
    }
    best.ok_or_else(|| format!("{}: no policy passes resource budgets", model.model_id))
}

#[derive(Clone, Debug)]
struct PolicyScores {
    policy: Policy,
    value_by_model: Vec<i128>,
    worst_case_value_numerator: i128,
    max_regret_numerator: i128,
    worst_case_model_id: &'static str,
    resource_use: ResourceUse,
}

fn score_policy(policy: Policy, models: &[LikelihoodModel]) -> Result<PolicyScores, String> {
    let usage = resource_use(policy);
    if !resource_use_fits(usage) {
        return Err(format!("{}: declared policy exceeds resource budget", policy.id()));
    }
    let mut values = Vec::with_capacity(models.len());
    let mut worst_value = i128::MAX;
    let mut worst_model_id = "none";
    let mut max_regret = i128::MIN;

    for model in models {
        let value = policy_value_numerator(policy, model)?;
        values.push(value);
        if value < worst_value {
            worst_value = value;
            worst_model_id = model.model_id;
        }
        let (_, best_value) = maximizer_for_model(model)?;
        let regret = best_value - value;
        if regret < 0 {
            return Err("regret cannot be negative against the exhaustive per-model oracle".to_owned());
        }
        max_regret = max_regret.max(regret);
    }

    Ok(PolicyScores {
        policy,
        value_by_model: values,
        worst_case_value_numerator: worst_value,
        max_regret_numerator: max_regret,
        worst_case_model_id: worst_model_id,
        resource_use: usage,
    })
}

fn exact_policy_selection(models: &[LikelihoodModel]) -> Result<Value, String> {
    if models.is_empty() {
        return Err("uncertainty set must contain at least one model".to_owned());
    }
    for model in models {
        validate_model(model)?;
        if !model.performance_evidence_verified {
            return Err(format!("{}: an unverified model cannot enter the accepted uncertainty set", model.model_id));
        }
    }

    let nominal_model = models
        .iter()
        .find(|model| model.model_id == "nominal_high_accuracy")
        .ok_or_else(|| "nominal model missing from uncertainty set".to_owned())?;

    let mut scores = Vec::with_capacity(Policy::ALL.len());
    for policy in Policy::ALL {
        scores.push(score_policy(policy, models)?);
    }

    let nominal_selected = scores
        .iter()
        .max_by_key(|score| policy_value_numerator(score.policy, nominal_model).unwrap_or(i128::MIN))
        .ok_or_else(|| "no candidate nominal policy".to_owned())?;
    let robust_selected = scores
        .iter()
        .max_by_key(|score| score.worst_case_value_numerator)
        .ok_or_else(|| "no candidate robust policy".to_owned())?;
    let regret_selected = scores
        .iter()
        .min_by_key(|score| score.max_regret_numerator)
        .ok_or_else(|| "no candidate minimax-regret policy".to_owned())?;

    if nominal_selected.policy != Policy::AlwaysBuyAssay
        || robust_selected.policy != Policy::NoMeasurement
        || regret_selected.policy != Policy::CalibrateThenAdapt
    {
        return Err("nominal/maximin/minimax-regret analytical policy oracle mismatch".to_owned());
    }

    let no_measurement = scores.iter().find(|score| score.policy == Policy::NoMeasurement)
        .ok_or_else(|| "no-measurement candidate missing".to_owned())?;
    let always_assay = scores.iter().find(|score| score.policy == Policy::AlwaysBuyAssay)
        .ok_or_else(|| "direct-assay candidate missing".to_owned())?;
    let calibrate_then_adapt = scores.iter().find(|score| score.policy == Policy::CalibrateThenAdapt)
        .ok_or_else(|| "calibration-gated candidate missing".to_owned())?;
    if no_measurement.worst_case_value_numerator != 5_000_000_000
        || no_measurement.max_regret_numerator != 1_500_000_000
        || always_assay.worst_case_value_numerator != 3_000_000_000
        || always_assay.max_regret_numerator != 2_000_000_000
        || calibrate_then_adapt.worst_case_value_numerator != 4_700_000_000
        || calibrate_then_adapt.max_regret_numerator != 300_000_000
    {
        return Err("maximin / minimax-regret numerical oracle mismatch".to_owned());
    }

    let baseline = baseline_value_numerator();
    let evpi = perfect_information_value_numerator() - baseline;
    let model_reports = models.iter().map(|model| {
        let gross = expected_value_with_assay_numerator(model).expect("validated assay");
        let evsi = gross - baseline;
        let evpi_bound = evsi >= 0 && evsi <= evpi;
        let outcomes = outcome_weights(model).expect("validated outcome weights");
        json!({
            "model_id": model.model_id,
            "sensitivity_bps": model.sensitivity_bps,
            "specificity_bps": model.specificity_bps,
            "performance_evidence_verified": model.performance_evidence_verified,
            "gross_expected_value_if_assay_bought": rational_json(gross, PROBABILITY_DENOMINATOR),
            "evsi_before_assay_cost": rational_json(evsi, PROBABILITY_DENOMINATOR),
            "evpi": rational_json(evpi, PROBABILITY_DENOMINATOR),
            "evsi_leq_evpi": evpi_bound,
            "posterior_rich_probability_after_positive_bps_floor": (outcomes[0].0 * BASIS_POINTS / (outcomes[0].0 + outcomes[0].1)),
            "posterior_rich_probability_after_negative_bps_floor": (outcomes[1].0 * BASIS_POINTS / (outcomes[1].0 + outcomes[1].1)),
            "net_value_if_always_buy_assay": rational_json(
                policy_value_numerator(Policy::AlwaysBuyAssay, model).expect("validated policy"),
                PROBABILITY_DENOMINATOR
            ),
            "net_value_if_calibrate_then_adapt": rational_json(
                policy_value_numerator(Policy::CalibrateThenAdapt, model).expect("validated policy"),
                PROBABILITY_DENOMINATOR
            )
        })
    }).collect::<Vec<_>>();

    let score_reports = scores.iter().map(|score| {
        json!({
            "policy_id": score.policy.id(),
            "description": score.policy.description(),
            "values_by_model": models.iter().zip(score.value_by_model.iter()).map(|(model, value)| {
                json!({
                    "model_id": model.model_id,
                    "net_expected_value": rational_json(*value, PROBABILITY_DENOMINATOR)
                })
            }).collect::<Vec<_>>(),
            "nominal_value": rational_json(
                policy_value_numerator(score.policy, nominal_model).expect("validated policy"),
                PROBABILITY_DENOMINATOR
            ),
            "worst_case_net_expected_value": rational_json(score.worst_case_value_numerator, PROBABILITY_DENOMINATOR),
            "worst_case_model_id": score.worst_case_model_id,
            "max_regret": rational_json(score.max_regret_numerator, PROBABILITY_DENOMINATOR),
            "resource_use": {
                "assays": score.resource_use.assays,
                "sample_units": score.resource_use.sample_units,
                "power_units": score.resource_use.power_units,
                "elapsed_hours": score.resource_use.elapsed_hours,
                "budget_pass": resource_use_fits(score.resource_use)
            },
            "eligible": true
        })
    }).collect::<Vec<_>>();

    Ok(json!({
        "receipt_schema_version": "0.1.0",
        "reference_version": "robust-exploration-model-uncertainty-v0.1",
        "result": "pass",
        "evidence_class": "synthetic_exact_finite_model_uncertainty_decision",
        "uncertainty_set": {
            "model_count": models.len(),
            "model_selection_rule": "No probability distribution over models is assumed; nominal, maximin and minimax-regret objectives are reported separately.",
            "nominal_model_id": nominal_model.model_id,
            "models": model_reports
        },
        "decision_payoffs_synthetic_units": {
            "build_if_rich": BUILD_PAYOFF_RICH,
            "build_if_poor": BUILD_PAYOFF_POOR,
            "import": IMPORT_PAYOFF
        },
        "expected_value_without_measurement": rational_json(baseline, PROBABILITY_DENOMINATOR),
        "expected_value_with_perfect_information": rational_json(perfect_information_value_numerator(), PROBABILITY_DENOMINATOR),
        "evpi": rational_json(evpi, PROBABILITY_DENOMINATOR),
        "calibration_gate": {
            "cost_units": CALIBRATION_GATE_COST,
            "assumption": "The calibration gate perfectly identifies which of the three likelihood regimes applies; this idealized assumption exists only to test policy trade-offs.",
            "buy_expensive_assay_only_for_model_id": "nominal_high_accuracy",
            "real_world_evidence": false
        },
        "resource_budget": {
            "max_assays": BUDGET.max_assays,
            "max_sample_units": BUDGET.max_sample_units,
            "max_power_units": BUDGET.max_power_units,
            "max_elapsed_hours": BUDGET.max_elapsed_hours
        },
        "policy_scores": score_reports,
        "selected_policies": {
            "nominal_optimal": nominal_selected.policy.id(),
            "maximin_expected_value": robust_selected.policy.id(),
            "minimax_max_regret": regret_selected.policy.id(),
            "nominal_value_numerator": policy_value_numerator(nominal_selected.policy, nominal_model)?,
            "maximin_value_numerator": robust_selected.worst_case_value_numerator,
            "minimax_regret_numerator": regret_selected.max_regret_numerator,
            "objective_choice_changes_policy": nominal_selected.policy != robust_selected.policy
                && robust_selected.policy != regret_selected.policy
        },
        "unverified_model_negative_control": {
            "model_id": "unverified_accuracy_claim",
            "claimed_sensitivity_bps": 9_900,
            "claimed_specificity_bps": 9_900,
            "performance_evidence_verified": false,
            "eligible_for_robust_selection": false,
            "credited_incremental_value": 0,
            "rejection_reason": "LIKELIHOOD_BOUNDS_UNVERIFIED"
        },
        "claim_ceiling": "Synthetic finite decision contract only. The uncertainty set, test accuracy values, model-regime calibration gate, payoffs and resource units are deliberately invented. The perfect regime-identifying calibration gate is an oracle control, not a proposed real instrument or empirically supported strategy. No geological prior, real survey accuracy, reserve estimate, or mission investment claim is made."
    }))
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

fn execute(source_commit: &str) -> Result<Value, String> {
    validate_source_commit(source_commit)?;
    let selection = exact_policy_selection(&MODEL_SET)?;
    let mut receipt = selection;
    let digest = format!(
        "{:x}",
        Sha256::digest(include_bytes!("robust_exploration_model_uncertainty_reference.rs"))
    );
    receipt["source_commit"] = json!(source_commit);
    receipt["reference_source_sha256"] = json!(digest);
    let model_specifications = json!({
        "schema_version": "0.1.0",
        "nominal_model_id": "nominal_high_accuracy",
        "models": MODEL_SET.iter().map(|model| json!({
            "model_id": model.model_id,
            "sensitivity_bps": model.sensitivity_bps,
            "specificity_bps": model.specificity_bps,
            "performance_evidence_verified": model.performance_evidence_verified
        })).collect::<Vec<_>>()
    });
    let model_set_bytes = serde_json::to_vec(&model_specifications)
        .map_err(|error| error.to_string())?;
    receipt["model_specifications"] = model_specifications;
    receipt["model_set_sha256"] = json!(format!("{:x}", Sha256::digest(model_set_bytes)));
    Ok(receipt)
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 || args[1] != "--source-commit" {
        return Err(
            "usage: cargo run --example robust_exploration_model_uncertainty_reference -- --source-commit <40-char-lowercase-git-sha>"
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
            eprintln!("robust_exploration_model_uncertainty_reference: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_model_likelihoods_conserve_joint_probability_mass() {
        for model in MODEL_SET {
            let outcomes = outcome_weights(&model).unwrap();
            assert_eq!(outcomes[0].0 + outcomes[0].1 + outcomes[1].0 + outcomes[1].1, PROBABILITY_DENOMINATOR);
        }
    }

    #[test]
    fn value_of_sample_information_never_exceeds_perfect_information_bound() {
        let evpi = perfect_information_value_numerator() - baseline_value_numerator();
        assert_eq!(evpi, 5_000_000_000);
        for model in MODEL_SET {
            let evsi = evsi_numerator(&model).unwrap();
            assert!(evsi >= 0);
            assert!(evsi <= evpi);
        }
    }

    #[test]
    fn nominal_policy_buys_assay_under_high_accuracy_model() {
        let high = &MODEL_SET[0];
        assert_eq!(expected_value_with_assay_numerator(high).unwrap(), 8_500_000_000);
        assert_eq!(policy_value_numerator(Policy::AlwaysBuyAssay, high).unwrap(), 6_500_000_000);
        assert_eq!(policy_value_numerator(Policy::CalibrateThenAdapt, high).unwrap(), 6_200_000_000);
        assert_eq!(policy_value_numerator(Policy::NoMeasurement, high).unwrap(), 5_000_000_000);
    }

    #[test]
    fn assay_positive_nominal_value_but_negative_worst_case_value() {
        let high_net = policy_value_numerator(Policy::AlwaysBuyAssay, &MODEL_SET[0]).unwrap();
        let low_net = policy_value_numerator(Policy::AlwaysBuyAssay, &MODEL_SET[2]).unwrap();
        let baseline = baseline_value_numerator();
        assert!(high_net > baseline);
        assert!(low_net < baseline);
        assert_eq!(high_net, 6_500_000_000);
        assert_eq!(low_net, 3_000_000_000);
    }

    #[test]
    fn calibration_then_adapt_wins_minimax_regret_objective() {
        let scores: Vec<_> = Policy::ALL.iter().map(|policy| score_policy(*policy, &MODEL_SET).unwrap()).collect();
        let chosen = scores.iter().min_by_key(|score| score.max_regret_numerator).unwrap();
        assert_eq!(chosen.policy, Policy::CalibrateThenAdapt);
        assert_eq!(chosen.max_regret_numerator, 300_000_000);
    }

    #[test]
    fn no_measurement_wins_maximin_expected_value_objective() {
        let scores: Vec<_> = Policy::ALL.iter().map(|policy| score_policy(*policy, &MODEL_SET).unwrap()).collect();
        let chosen = scores.iter().max_by_key(|score| score.worst_case_value_numerator).unwrap();
        assert_eq!(chosen.policy, Policy::NoMeasurement);
        assert_eq!(chosen.worst_case_value_numerator, 5_000_000_000);
    }

    #[test]
    fn nominal_optimizer_maximin_and_minimax_regret_choose_different_policies() {
        let receipt = exact_policy_selection(&MODEL_SET).unwrap();
        assert_eq!(receipt["selected_policies"]["nominal_optimal"], "always_buy_assay");
        assert_eq!(receipt["selected_policies"]["maximin_expected_value"], "no_measurement");
        assert_eq!(receipt["selected_policies"]["minimax_max_regret"], "calibrate_then_adapt");
        assert_eq!(receipt["selected_policies"]["objective_choice_changes_policy"], true);
    }

    #[test]
    fn unverified_likelihood_cannot_enter_uncertainty_set() {
        let mut models = MODEL_SET;
        models[0].performance_evidence_verified = false;
        assert!(exact_policy_selection(&models).is_err());
    }

    #[test]
    fn invalid_likelihood_or_commit_fails_closed() {
        let invalid = LikelihoodModel { sensitivity_bps: 10_001, ..MODEL_SET[0] };
        assert!(validate_model(&invalid).is_err());
        assert!(execute("bad-sha").is_err());
    }

    #[test]
    fn all_policy_resource_uses_pass_declared_hard_budget() {
        for policy in Policy::ALL {
            assert!(resource_use_fits(resource_use(policy)));
        }
        let over_budget = ResourceUse {
            assays: 3,
            sample_units: 3,
            power_units: 5,
            elapsed_hours: 3,
        };
        assert!(!resource_use_fits(over_budget));
    }

    #[test]
    fn receipt_is_source_bound_and_reports_uncertainty_set_digest() {
        let receipt = execute(&"a".repeat(40)).unwrap();
        assert_eq!(receipt["result"], "pass");
        assert_eq!(receipt["source_commit"], "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        assert_eq!(receipt["reference_source_sha256"].as_str().unwrap().len(), 64);
        assert_eq!(receipt["model_set_sha256"].as_str().unwrap().len(), 64);
        assert_eq!(receipt["uncertainty_set"]["model_count"], 3);
    }
}
