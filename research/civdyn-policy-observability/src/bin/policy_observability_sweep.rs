// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use civdyn_policy_observability::{
    CandidateRecord, EvaluatorRecord, MissingReason, MissingnessMechanism, ObservationAssumption,
    PolicyEvaluation, evaluate_policy, known_mar_propensity_mean, robust_comparison,
    worst_case_regret_upper_bound,
};

const SUPPORT_LOWER: u64 = 0;
const SUPPORT_UPPER: u64 = 100;

#[derive(Debug)]
struct RowSpec<'a> {
    policy_id: &'a str,
    unit_id: &'a str,
    stratum_id: &'a str,
    outcome: u64,
    observed: bool,
    reason: Option<MissingReason>,
    propensity_bps: Option<u32>,
    mechanism: MissingnessMechanism,
}

fn row(spec: RowSpec<'_>) -> EvaluatorRecord {
    let RowSpec {
        policy_id,
        unit_id,
        stratum_id,
        outcome,
        observed,
        reason,
        propensity_bps,
        mechanism,
    } = spec;
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
        row(RowSpec {
            policy_id: "masked_failures",
            unit_id: "u1",
            stratum_id: "high_need",
            outcome: 100,
            observed: true,
            reason: None,
            propensity_bps: None,
            mechanism: MissingnessMechanism::MnarAdverseOutcomesHidden,
        }),
        row(RowSpec {
            policy_id: "masked_failures",
            unit_id: "u2",
            stratum_id: "high_need",
            outcome: 0,
            observed: false,
            reason: Some(MissingReason::PolicySuppressed),
            propensity_bps: None,
            mechanism: MissingnessMechanism::MnarAdverseOutcomesHidden,
        }),
        row(RowSpec {
            policy_id: "masked_failures",
            unit_id: "u3",
            stratum_id: "general",
            outcome: 0,
            observed: false,
            reason: Some(MissingReason::TelemetryUnavailable),
            propensity_bps: None,
            mechanism: MissingnessMechanism::MnarAdverseOutcomesHidden,
        }),
        row(RowSpec {
            policy_id: "masked_failures",
            unit_id: "u4",
            stratum_id: "general",
            outcome: 0,
            observed: false,
            reason: Some(MissingReason::PolicySuppressed),
            propensity_bps: None,
            mechanism: MissingnessMechanism::MnarAdverseOutcomesHidden,
        }),
    ];
    let transparent = vec![
        row(RowSpec {
            policy_id: "transparent_service",
            unit_id: "u1",
            stratum_id: "high_need",
            outcome: 60,
            observed: true,
            reason: None,
            propensity_bps: None,
            mechanism: MissingnessMechanism::Complete,
        }),
        row(RowSpec {
            policy_id: "transparent_service",
            unit_id: "u2",
            stratum_id: "high_need",
            outcome: 60,
            observed: true,
            reason: None,
            propensity_bps: None,
            mechanism: MissingnessMechanism::Complete,
        }),
        row(RowSpec {
            policy_id: "transparent_service",
            unit_id: "u3",
            stratum_id: "general",
            outcome: 60,
            observed: true,
            reason: None,
            propensity_bps: None,
            mechanism: MissingnessMechanism::Complete,
        }),
        row(RowSpec {
            policy_id: "transparent_service",
            unit_id: "u4",
            stratum_id: "general",
            outcome: 60,
            observed: true,
            reason: None,
            propensity_bps: None,
            mechanism: MissingnessMechanism::Complete,
        }),
    ];
    (masked, transparent)
}

fn selective_benefit() -> (Vec<EvaluatorRecord>, Vec<EvaluatorRecord>) {
    let hidden = vec![
        row(RowSpec {
            policy_id: "hidden_successes",
            unit_id: "u1",
            stratum_id: "high_need",
            outcome: 100,
            observed: true,
            reason: None,
            propensity_bps: None,
            mechanism: MissingnessMechanism::MnarBeneficialOutcomesHidden,
        }),
        row(RowSpec {
            policy_id: "hidden_successes",
            unit_id: "u2",
            stratum_id: "high_need",
            outcome: 100,
            observed: false,
            reason: Some(MissingReason::TelemetryUnavailable),
            propensity_bps: None,
            mechanism: MissingnessMechanism::MnarBeneficialOutcomesHidden,
        }),
        row(RowSpec {
            policy_id: "hidden_successes",
            unit_id: "u3",
            stratum_id: "general",
            outcome: 100,
            observed: false,
            reason: Some(MissingReason::PolicySuppressed),
            propensity_bps: None,
            mechanism: MissingnessMechanism::MnarBeneficialOutcomesHidden,
        }),
        row(RowSpec {
            policy_id: "hidden_successes",
            unit_id: "u4",
            stratum_id: "general",
            outcome: 100,
            observed: false,
            reason: Some(MissingReason::TelemetryUnavailable),
            propensity_bps: None,
            mechanism: MissingnessMechanism::MnarBeneficialOutcomesHidden,
        }),
    ];
    let steady = vec![
        row(RowSpec {
            policy_id: "steady_service",
            unit_id: "u1",
            stratum_id: "high_need",
            outcome: 60,
            observed: true,
            reason: None,
            propensity_bps: None,
            mechanism: MissingnessMechanism::Complete,
        }),
        row(RowSpec {
            policy_id: "steady_service",
            unit_id: "u2",
            stratum_id: "high_need",
            outcome: 60,
            observed: true,
            reason: None,
            propensity_bps: None,
            mechanism: MissingnessMechanism::Complete,
        }),
        row(RowSpec {
            policy_id: "steady_service",
            unit_id: "u3",
            stratum_id: "general",
            outcome: 60,
            observed: true,
            reason: None,
            propensity_bps: None,
            mechanism: MissingnessMechanism::Complete,
        }),
        row(RowSpec {
            policy_id: "steady_service",
            unit_id: "u4",
            stratum_id: "general",
            outcome: 60,
            observed: true,
            reason: None,
            propensity_bps: None,
            mechanism: MissingnessMechanism::Complete,
        }),
    ];
    (hidden, steady)
}

fn mar_positive_control() -> Vec<EvaluatorRecord> {
    [
        ("u1", "high_need", 20, true),
        ("u2", "high_need", 20, false),
        ("u3", "general", 80, true),
        ("u4", "general", 80, false),
    ]
    .into_iter()
    .map(|(unit, stratum, outcome, observed)| {
        row(RowSpec {
            policy_id: "mar_known_propensity",
            unit_id: unit,
            stratum_id: stratum,
            outcome,
            observed,
            reason: Some(MissingReason::Nonresponse),
            propensity_bps: Some(5_000),
            mechanism: MissingnessMechanism::Mar,
        })
    })
    .collect()
}

fn mixed_mechanism() -> Vec<EvaluatorRecord> {
    vec![
        row(RowSpec {
            policy_id: "mixed_observation",
            unit_id: "u1",
            stratum_id: "high_need",
            outcome: 90,
            observed: true,
            reason: None,
            propensity_bps: Some(5_000),
            mechanism: MissingnessMechanism::Mar,
        }),
        row(RowSpec {
            policy_id: "mixed_observation",
            unit_id: "u2",
            stratum_id: "high_need",
            outcome: 10,
            observed: false,
            reason: Some(MissingReason::Nonresponse),
            propensity_bps: Some(5_000),
            mechanism: MissingnessMechanism::Mar,
        }),
        row(RowSpec {
            policy_id: "mixed_observation",
            unit_id: "u3",
            stratum_id: "general",
            outcome: 0,
            observed: false,
            reason: Some(MissingReason::PolicySuppressed),
            propensity_bps: None,
            mechanism: MissingnessMechanism::MnarAdverseOutcomesHidden,
        }),
        row(RowSpec {
            policy_id: "mixed_observation",
            unit_id: "u4",
            stratum_id: "general",
            outcome: 80,
            observed: true,
            reason: None,
            propensity_bps: None,
            mechanism: MissingnessMechanism::MnarAdverseOutcomesHidden,
        }),
    ]
}

fn find_stratum_mean(evaluation: &PolicyEvaluation, stratum_id: &str) -> String {
    evaluation
        .strata
        .iter()
        .find(|stratum| stratum.stratum_id == stratum_id)
        .map_or_else(String::new, |stratum| format!("{:.3}", stratum.true_mean))
}

fn find_stratum_interval(
    evaluation: &PolicyEvaluation,
    stratum_id: &str,
) -> (String, String, String) {
    evaluation
        .strata
        .iter()
        .find(|stratum| stratum.stratum_id == stratum_id)
        .map_or_else(
            || (String::new(), String::new(), String::new()),
            |stratum| {
                (
                    format!("{:.3}", stratum.estimates.identified_interval.lower),
                    format!("{:.3}", stratum.estimates.identified_interval.upper),
                    format!("{:.3}", stratum.estimates.observation_coverage),
                )
            },
        )
}

fn fmt_u32_option(value: Option<u32>) -> String {
    value.map_or_else(String::new, |number| number.to_string())
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
    let (high_lower, high_upper, high_coverage) = find_stratum_interval(evaluation, "high_need");
    let (general_lower, general_upper, general_coverage) =
        find_stratum_interval(evaluation, "general");
    println!(
        "{scenario_id},{},{},{:.3},{},{},{:.3},{:.3},{truth_contained},{:.3},{},{},{},{},{},{},{},{},{},{},{:.3},{},{},{},{},{},{},{},{},{comparison},{regret},{}",
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
        high_lower,
        high_upper,
        high_coverage,
        find_stratum_mean(evaluation, "general"),
        general_lower,
        general_upper,
        general_coverage,
        evaluation.true_stratum_gap,
        evaluation
            .estimates
            .missing_by_reason
            .get(&MissingReason::TelemetryUnavailable)
            .copied()
            .unwrap_or(0),
        evaluation
            .estimates
            .missing_by_reason
            .get(&MissingReason::Nonresponse)
            .copied()
            .unwrap_or(0),
        evaluation
            .estimates
            .missing_by_reason
            .get(&MissingReason::PolicySuppressed)
            .copied()
            .unwrap_or(0),
        evaluation
            .estimates
            .missing_by_reason
            .get(&MissingReason::AdministrativeExclusion)
            .copied()
            .unwrap_or(0),
        evaluation.estimates.propensity_units,
        evaluation.estimates.missing_propensity_units,
        fmt_u32_option(evaluation.estimates.minimum_propensity_bps),
        fmt_u32_option(evaluation.estimates.maximum_propensity_bps),
        fmt_option(mar_estimate),
    );
}

fn main() {
    println!(
        "scenario_id,policy_id,true_mechanism,true_mean,complete_case_mean,missing_as_zero_mean,identified_lower,identified_upper,truth_contained,observation_coverage,observed_units,missing_units,high_need_true_mean,high_need_identified_lower,high_need_identified_upper,high_need_observation_coverage,general_true_mean,general_identified_lower,general_identified_upper,general_observation_coverage,true_stratum_gap,missing_telemetry_unavailable_units,missing_nonresponse_units,missing_policy_suppressed_units,missing_administrative_exclusion_units,propensity_units,missing_propensity_units,minimum_propensity_bps,maximum_propensity_bps,robust_comparison_vs_reference,worst_case_regret_upper_bound,known_mar_ht_mean"
    );

    let support = civdyn_policy_observability::OutcomeSupport {
        lower: SUPPORT_LOWER,
        upper: SUPPORT_UPPER,
    };

    let (masked, transparent) = selective_adverse();
    let masked_eval = evaluate_policy(&masked, support).expect("masked adverse fixture");
    let transparent_eval =
        evaluate_policy(&transparent, support).expect("transparent adverse fixture");
    emit(
        "selective_adverse",
        &masked_eval,
        Some(&transparent_eval),
        None,
    );
    emit(
        "selective_adverse",
        &transparent_eval,
        Some(&masked_eval),
        None,
    );

    let (hidden, steady) = selective_benefit();
    let hidden_eval = evaluate_policy(&hidden, support).expect("hidden benefit fixture");
    let steady_eval = evaluate_policy(&steady, support).expect("steady benefit fixture");
    emit("selective_benefit", &hidden_eval, Some(&steady_eval), None);
    emit("selective_benefit", &steady_eval, Some(&hidden_eval), None);

    let mixed_rows = mixed_mechanism();
    let mixed_eval = evaluate_policy(&mixed_rows, support).expect("mixed mechanism fixture");
    emit("mixed_mechanism", &mixed_eval, None, None);

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
