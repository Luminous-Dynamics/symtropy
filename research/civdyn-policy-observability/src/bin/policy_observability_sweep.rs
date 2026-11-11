// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use civdyn_policy_observability::{
    evaluate_policy, known_mar_propensity_mean, robust_comparison, worst_case_regret_upper_bound,
    CandidateRecord, EvaluatorRecord, MissingReason, MissingnessMechanism,
    ObservationAssumption, PolicyEvaluation,
};

const SUPPORT_LOWER: u64 = 0;
const SUPPORT_UPPER: u64 = 100;

fn row(
    policy_id: &str,
    unit_id: &str,
    stratum_id: &str,
    outcome: u64,
    observed: bool,
    reason: Option<MissingReason>,
    propensity_bps: Option<u32>,
    mechanism: MissingnessMechanism,
) -> EvaluatorRecord {
    EvaluatorRecord {
        observation: CandidateRecord {
            policy_id: policy_id.to_owned(),
            unit_id: unit_id.to_owned(),
            stratum_id: stratum_id.to_owned(),
            population_weight: 1,
            observed_outcome: observed.then_some(outcome),
            missing_reason: if observed { None } else { reason },
            observation_propensity_bps: propensity_bps,
        },
        latent_outcome: outcome,
        true_mechanism: mechanism,
    }
}

fn selective_adverse() -> (Vec<EvaluatorRecord>, Vec<EvaluatorRecord>) {
    let masked = vec![
        row("masked_failures", "u1", "high_need", 100, true, None, None, MissingnessMechanism::MnarAdverseOutcomesHidden),
        row("masked_failures", "u2", "high_need", 0, false, Some(MissingReason::PolicySuppressed), None, MissingnessMechanism::MnarAdverseOutcomesHidden),
        row("masked_failures", "u3", "general", 0, false, Some(MissingReason::TelemetryUnavailable), None, MissingnessMechanism::MnarAdverseOutcomesHidden),
        row("masked_failures", "u4", "general", 0, false, Some(MissingReason::PolicySuppressed), None, MissingnessMechanism::MnarAdverseOutcomesHidden),
    ];
    let transparent = vec![
        row("transparent_service", "u1", "high_need", 60, true, None, None, MissingnessMechanism::Complete),
        row("transparent_service", "u2", "high_need", 60, true, None, None, MissingnessMechanism::Complete),
        row("transparent_service", "u3", "general", 60, true, None, None, MissingnessMechanism::Complete),
        row("transparent_service", "u4", "general", 60, true, None, None, MissingnessMechanism::Complete),
    ];
    (masked, transparent)
}

fn selective_benefit() -> (Vec<EvaluatorRecord>, Vec<EvaluatorRecord>) {
    let hidden = vec![
        row("hidden_successes", "u1", "high_need", 100, true, None, None, MissingnessMechanism::MnarBeneficialOutcomesHidden),
        row("hidden_successes", "u2", "high_need", 100, false, Some(MissingReason::TelemetryUnavailable), None, MissingnessMechanism::MnarBeneficialOutcomesHidden),
        row("hidden_successes", "u3", "general", 100, false, Some(MissingReason::PolicySuppressed), None, MissingnessMechanism::MnarBeneficialOutcomesHidden),
        row("hidden_successes", "u4", "general", 100, false, Some(MissingReason::TelemetryUnavailable), None, MissingnessMechanism::MnarBeneficialOutcomesHidden),
    ];
    let steady = vec![
        row("steady_service", "u1", "high_need", 60, true, None, None, MissingnessMechanism::Complete),
        row("steady_service", "u2", "high_need", 60, true, None, None, MissingnessMechanism::Complete),
        row("steady_service", "u3", "general", 60, true, None, None, MissingnessMechanism::Complete),
        row("steady_service", "u4", "general", 60, true, None, None, MissingnessMechanism::Complete),
    ];
    (hidden, steady)
}

fn mar_positive_control() -> Vec<EvaluatorRecord> {
    [
        ("u1", "lower", 20, true),
        ("u2", "lower", 20, false),
        ("u3", "upper", 80, true),
        ("u4", "upper", 80, false),
    ]
    .into_iter()
    .map(|(unit, stratum, outcome, observed)| {
        row(
            "mar_known_propensity",
            unit,
            stratum,
            outcome,
            observed,
            Some(MissingReason::Nonresponse),
            Some(5_000),
            MissingnessMechanism::Mar,
        )
    })
    .collect()
}

fn find_stratum_mean(evaluation: &PolicyEvaluation, stratum_id: &str) -> String {
    evaluation
        .strata
        .iter()
        .find(|stratum| stratum.stratum_id == stratum_id)
        .map_or_else(String::new, |stratum| format!("{:.3}", stratum.true_mean))
}

fn fmt_option(value: Option<f64>) -> String {
    value.map_or_else(String::new, |number| format!("{number:.3}"))
}

fn emit(
    scenario_id: &str,
    evaluation: &PolicyEvaluation,
    reference: Option<&PolicyEvaluation>,
    mar_estimate: Option<f64>,
) {
    let (comparison, regret) = reference.map_or_else(
        || ("not_compared".to_owned(), String::new()),
        |other| {
            let comparison = robust_comparison(
                &evaluation.estimates.identified_interval,
                &other.estimates.identified_interval,
            );
            let regret = worst_case_regret_upper_bound(
                &evaluation.estimates.identified_interval,
                &other.estimates.identified_interval,
            );
            (comparison.id().to_owned(), format!("{regret:.3}"))
        },
    );
    let interval = &evaluation.estimates.identified_interval;
    let truth_contained = interval.contains(evaluation.true_mean);
    println!(
        "{scenario_id},{},{},{:.3},{},{},{:.3},{:.3},{truth_contained},{:.3},{},{},{},{},{:.3},{comparison},{regret},{}",
        evaluation.policy_id,
        evaluation.true_mechanism.id(),
        evaluation.true_mean,
        fmt_option(evaluation.estimates.complete_case_mean),
        fmt_option(evaluation.estimates.missing_as_zero_mean),
        interval.lower,
        interval.upper,
        evaluation.estimates.observation_coverage,
        evaluation.estimates.observed_units,
        evaluation.estimates.missing_units,
        find_stratum_mean(evaluation, "high_need"),
        find_stratum_mean(evaluation, "general"),
        evaluation.true_stratum_gap,
        fmt_option(mar_estimate),
    );
}

fn main() {
    println!(
        "scenario_id,policy_id,true_mechanism,true_mean,complete_case_mean,missing_as_zero_mean,identified_lower,identified_upper,truth_contained,observation_coverage,observed_units,missing_units,high_need_true_mean,general_true_mean,missing_telemetry_unavailable_units,missing_nonresponse_units,missing_policy_suppressed_units,missing_administrative_exclusion_units,robust_comparison_vs_reference,worst_case_regret_upper_bound,known_mar_ht_mean"
    );

    let support = civdyn_policy_observability::OutcomeSupport {
        lower: SUPPORT_LOWER,
        upper: SUPPORT_UPPER,
    };

    let (masked, transparent) = selective_adverse();
    let masked_eval = evaluate_policy(&masked, support).expect("masked adverse fixture");
    let transparent_eval =
        evaluate_policy(&transparent, support).expect("transparent adverse fixture");
    emit("selective_adverse", &masked_eval, Some(&transparent_eval), None);
    emit("selective_adverse", &transparent_eval, Some(&masked_eval), None);

    let (hidden, steady) = selective_benefit();
    let hidden_eval = evaluate_policy(&hidden, support).expect("hidden benefit fixture");
    let steady_eval = evaluate_policy(&steady, support).expect("steady benefit fixture");
    emit("selective_benefit", &hidden_eval, Some(&steady_eval), None);
    emit("selective_benefit", &steady_eval, Some(&hidden_eval), None);

    let mar_rows = mar_positive_control();
    let mar_eval = evaluate_policy(&mar_rows, support).expect("MAR positive control");
    let candidate_rows: Vec<CandidateRecord> = mar_rows
        .iter()
        .map(|record| record.observation.clone())
        .collect();
    let mar_estimate = known_mar_propensity_mean(
        &candidate_rows,
        support,
        ObservationAssumption::KnownMarPropensity,
    )
    .expect("known propensity estimate");
    emit("mar_positive_control", &mar_eval, None, Some(mar_estimate));

}
