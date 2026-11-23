use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::collections::HashSet;

const SCHEMA: &str = "luminous.orbital-reliability-power-law.corpus.v0.1";
const CLAIM_CEILING: &str = "synthetic-power-law-parameter-estimation-only";
const MAX_EVENTS: usize = 100_000;
const MAX_EXPOSURE_SECONDS: u64 = 9_007_199_254_740_991;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModelProfile {
    PowerLawProcessRepairableMinimalRepair,
    HomogeneousPoissonConstantIntensity,
    WeibullFirstFailureLife,
    UnknownOrUnregistered,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RestorationState {
    MinimalRepair,
    AsGoodAsNew,
    ImperfectRepair,
    Unknown,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Censoring {
    AdministrativeEnd,
    RightCensored,
    Intervention,
    InformativeWithdrawal,
    Unknown,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Disposition {
    EstimateProduced,
    Indeterminate,
    ModelNotSupported,
    Rejected,
    Disputed,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FailureEvent {
    pub event_id: String,
    pub operating_time_seconds: u64,
    pub failure_definition_id: String,
    pub restoration_state: RestorationState,
    pub effective_at: i64,
    pub recorded_at: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RepairableSeries {
    pub series_id: String,
    pub article_serial: String,
    pub population_id: String,
    pub lot_id: String,
    pub configuration_id: String,
    pub operating_mode_id: String,
    pub environment_profile_id: String,
    pub failure_definition_id: String,
    pub exposure_end_seconds: u64,
    pub events: Vec<FailureEvent>,
    pub raw_artifact_sha256: String,
    pub evidence_complete: bool,
    pub exposure_clock_verified: bool,
    pub clock_error_bound_seconds: Option<u64>,
    pub event_capture_complete: bool,
    pub data_loss_counter_verified: bool,
    pub maintenance_history_complete: bool,
    pub censoring: Censoring,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Expected {
    pub disposition: Disposition,
    pub unique_events: usize,
    pub duplicate_events: usize,
    pub exposure_end_seconds: u64,
    pub beta_hat: Option<f64>,
    pub log_eta_hat: Option<f64>,
    pub intensity_at_end_per_second: Option<f64>,
    pub cumulative_mean_at_half_exposure: Option<f64>,
    pub descriptive_likelihood_ratio_statistic: Option<f64>,
    pub likelihood_ratio_nonnegative: Option<bool>,
    pub inferential_fit_accepted: bool,
    pub reliability_claim_granted: bool,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Scope {
    pub population_id: String,
    pub lot_id: String,
    pub configuration_id: String,
    pub operating_mode_id: String,
    pub environment_profile_id: String,
    pub failure_definition_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PowerLawCase {
    pub case_id: String,
    pub selected_profile: ModelProfile,
    pub repair_policy_id: String,
    pub preregistration_ref: Option<String>,
    pub sampling_frame_ref: Option<String>,
    pub assumption_assessment_ref: Option<String>,
    pub maintenance_policy_ref: Option<String>,
    pub event_capture_assessment_ref: Option<String>,
    pub clock_calibration_ref: Option<String>,
    pub scope: Scope,
    pub max_clock_error_seconds: u64,
    pub effective_cutoff: i64,
    pub knowledge_cutoff: i64,
    pub diagnostics_predeclared: bool,
    pub thresholds_frozen_before_holdout: bool,
    pub series: RepairableSeries,
    pub expected: Expected,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Corpus {
    pub schema: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub cases: Vec<PowerLawCase>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CaseResult {
    pub case_id: String,
    pub disposition: Disposition,
    pub unique_events: usize,
    pub duplicate_events: usize,
    pub exposure_end_seconds: u64,
    pub beta_hat: Option<f64>,
    pub log_eta_hat: Option<f64>,
    pub intensity_at_end_per_second: Option<f64>,
    pub cumulative_mean_at_half_exposure: Option<f64>,
    pub descriptive_likelihood_ratio_statistic: Option<f64>,
    pub likelihood_ratio_nonnegative: Option<bool>,
    pub inferential_fit_accepted: bool,
    pub reliability_claim_granted: bool,
    pub authority_granted: bool,
    pub blockers: Vec<String>,
    pub expected_result_matched: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Summary {
    pub case_count: usize,
    pub estimate_produced: usize,
    pub indeterminate: usize,
    pub model_not_supported: usize,
    pub rejected: usize,
    pub disputed: usize,
    pub total_unique_events: usize,
    pub total_duplicate_events: usize,
    pub expected_result_mismatches: usize,
    pub inferential_fit_acceptances: usize,
    pub reliability_claims_granted: usize,
    pub authority_grants: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct Report {
    pub result: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub authority_granted: bool,
    pub summary: Summary,
    pub cases: Vec<CaseResult>,
}

#[derive(Debug, Default)]
struct Decision {
    disposition: Option<Disposition>,
    unique_events: usize,
    duplicate_events: usize,
    exposure_end_seconds: u64,
    beta_hat: Option<f64>,
    log_eta_hat: Option<f64>,
    intensity_at_end_per_second: Option<f64>,
    cumulative_mean_at_half_exposure: Option<f64>,
    descriptive_likelihood_ratio_statistic: Option<f64>,
    likelihood_ratio_nonnegative: Option<bool>,
    blockers: Vec<String>,
}

fn add_blocker(decision: &mut Decision, blocker: &str) {
    if !decision.blockers.iter().any(|existing| existing == blocker) {
        decision.blockers.push(blocker.to_owned());
    }
}

fn present(value: &Option<String>) -> bool {
    value.as_deref().is_some_and(|item| !item.trim().is_empty())
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

/// Complete-window NHPP power-law MLEs for M(t) = eta * t^beta.
/// Returns beta, log(eta), end intensity, half-window expected count and
/// the descriptive likelihood-ratio statistic versus beta = 1.
/// No p-value or inferential decision is produced.
pub fn estimate_power_law(
    event_times_seconds: &[u64],
    exposure_end_seconds: u64,
) -> Result<(f64, f64, f64, f64, f64), String> {
    if exposure_end_seconds == 0 || exposure_end_seconds > MAX_EXPOSURE_SECONDS {
        return Err("exposure end must be positive and within the v0.1 limit".to_owned());
    }
    if event_times_seconds.len() < 2 || event_times_seconds.len() > MAX_EVENTS {
        return Err("power-law MLE requires 2..=100000 unique events".to_owned());
    }

    let end = exposure_end_seconds as f64;
    let mut sum_log_ratio = 0.0;
    for (index, event_time) in event_times_seconds.iter().enumerate() {
        if *event_time == 0
            || *event_time >= exposure_end_seconds
            || (index > 0 && event_times_seconds[index - 1] >= *event_time)
        {
            return Err("event times must be strictly increasing in (0, exposure_end)".to_owned());
        }
        sum_log_ratio += (end / *event_time as f64).ln();
    }
    if !sum_log_ratio.is_finite() || sum_log_ratio <= 0.0 {
        return Err("power-law sufficient statistic is invalid".to_owned());
    }

    let count = event_times_seconds.len() as f64;
    let beta = count / sum_log_ratio;
    let log_eta = count.ln() - beta * end.ln();
    let end_intensity = beta * count / end;
    let half_mean = count * 0.5_f64.powf(beta);
    let lr_statistic = 2.0 * count * (beta.ln() + (1.0 / beta) - 1.0);

    if !beta.is_finite()
        || beta <= 0.0
        || !log_eta.is_finite()
        || !end_intensity.is_finite()
        || !half_mean.is_finite()
        || !lr_statistic.is_finite()
    {
        return Err("power-law parameter estimate is outside the v0.1 numeric range".to_owned());
    }

    Ok((
        beta,
        log_eta,
        end_intensity,
        half_mean,
        lr_statistic.max(0.0),
    ))
}

fn evaluate_case(case: &PowerLawCase) -> Result<Decision, String> {
    let mut decision = Decision::default();
    if case.case_id.trim().is_empty() {
        return Err("case ID must be non-empty".to_owned());
    }
    if case.selected_profile != ModelProfile::PowerLawProcessRepairableMinimalRepair {
        add_blocker(
            &mut decision,
            "selected_model_profile_is_not_power_law_repairable_minimal_repair",
        );
        decision.disposition = Some(Disposition::ModelNotSupported);
    }
    if case.repair_policy_id != "minimal_repair_nhpp_v0_1" {
        add_blocker(
            &mut decision,
            "repair_policy_is_not_the_declared_minimal_repair_profile",
        );
        decision.disposition = Some(Disposition::ModelNotSupported);
    }
    if !present(&case.preregistration_ref)
        || !present(&case.sampling_frame_ref)
        || !present(&case.assumption_assessment_ref)
        || !present(&case.maintenance_policy_ref)
        || !present(&case.event_capture_assessment_ref)
        || !present(&case.clock_calibration_ref)
    {
        add_blocker(
            &mut decision,
            "missing_preregistration_or_admissibility_reference",
        );
    }
    if !case.diagnostics_predeclared || !case.thresholds_frozen_before_holdout {
        add_blocker(
            &mut decision,
            "analysis_protocol_or_thresholds_not_predeclared",
        );
    }
    if case.knowledge_cutoff < case.effective_cutoff {
        add_blocker(&mut decision, "knowledge_cutoff_precedes_effective_cutoff");
    }

    let series = &case.series;
    decision.exposure_end_seconds = series.exposure_end_seconds;
    if series.series_id.trim().is_empty() || series.article_serial.trim().is_empty() {
        add_blocker(&mut decision, "missing_series_or_article_identity");
    }
    if series.population_id != case.scope.population_id
        || series.lot_id != case.scope.lot_id
        || series.configuration_id != case.scope.configuration_id
        || series.operating_mode_id != case.scope.operating_mode_id
        || series.environment_profile_id != case.scope.environment_profile_id
        || series.failure_definition_id != case.scope.failure_definition_id
    {
        add_blocker(&mut decision, "series_scope_differs_from_case_scope");
    }
    if series.exposure_end_seconds == 0
        || series.exposure_end_seconds > MAX_EXPOSURE_SECONDS
        || !valid_digest(&series.raw_artifact_sha256)
    {
        add_blocker(&mut decision, "invalid_exposure_end_or_artifact_digest");
    }
    if !series.evidence_complete
        || !series.exposure_clock_verified
        || series.clock_error_bound_seconds.is_none()
        || series
            .clock_error_bound_seconds
            .is_some_and(|error| error > case.max_clock_error_seconds)
        || !series.event_capture_complete
        || !series.data_loss_counter_verified
        || !series.maintenance_history_complete
    {
        add_blocker(
            &mut decision,
            "incomplete_clock_capture_maintenance_or_artifact_evidence",
        );
    }
    if matches!(
        series.censoring,
        Censoring::Intervention | Censoring::InformativeWithdrawal | Censoring::Unknown
    ) {
        add_blocker(
            &mut decision,
            "intervention_or_unresolved_informative_censoring",
        );
    }

    if series.events.len() > MAX_EVENTS {
        return Err("event count exceeds the v0.1 limit".to_owned());
    }
    let mut events_by_id = HashMap::<&str, &FailureEvent>::new();
    let mut unique_events = Vec::<&FailureEvent>::new();
    let mut dispute = false;
    for event in &series.events {
        if event.event_id.trim().is_empty() {
            add_blocker(&mut decision, "empty_event_id");
            continue;
        }
        if let Some(previous) = events_by_id.get(event.event_id.as_str()) {
            if *previous == event {
                decision.duplicate_events += 1;
                add_blocker(&mut decision, "identical_duplicate_event_record");
                continue;
            }
            dispute = true;
            add_blocker(&mut decision, "conflicting_event_id");
            continue;
        }
        events_by_id.insert(&event.event_id, event);
        unique_events.push(event);
    }
    unique_events.sort_by_key(|event| event.operating_time_seconds);
    decision.unique_events = unique_events.len();

    for (index, event) in unique_events.iter().enumerate() {
        if event.failure_definition_id != case.scope.failure_definition_id
            || event.operating_time_seconds == 0
            || event.operating_time_seconds >= series.exposure_end_seconds
            || (index > 0
                && unique_events[index - 1].operating_time_seconds >= event.operating_time_seconds)
        {
            add_blocker(
                &mut decision,
                "event_time_order_window_or_failure_definition_invalid",
            );
        }
        if event.effective_at > case.effective_cutoff || event.recorded_at > case.knowledge_cutoff {
            add_blocker(&mut decision, "event_not_known_at_declared_cutoff");
        }
        if event.restoration_state != RestorationState::MinimalRepair {
            add_blocker(
                &mut decision,
                "restoration_state_is_not_supported_by_v0_1_profile",
            );
            decision.disposition = Some(Disposition::ModelNotSupported);
        }
    }

    if unique_events.len() < 2 {
        add_blocker(
            &mut decision,
            "fewer_than_two_unique_events_for_power_law_mle",
        );
    }

    if series.exposure_end_seconds > 0 && unique_events.len() >= 2 {
        let event_times = unique_events
            .iter()
            .map(|event| event.operating_time_seconds)
            .collect::<Vec<_>>();
        if let Ok((beta, log_eta, intensity_at_end, half_mean, lr)) =
            estimate_power_law(&event_times, series.exposure_end_seconds)
        {
            decision.beta_hat = Some(beta);
            decision.log_eta_hat = Some(log_eta);
            decision.intensity_at_end_per_second = Some(intensity_at_end);
            decision.cumulative_mean_at_half_exposure = Some(half_mean);
            decision.descriptive_likelihood_ratio_statistic = Some(lr);
            decision.likelihood_ratio_nonnegative = Some(lr >= 0.0);
        } else {
            add_blocker(&mut decision, "power_law_estimator_rejected_event_times");
        }
    }

    if dispute {
        decision.disposition = Some(Disposition::Disputed);
    } else if decision.disposition != Some(Disposition::ModelNotSupported) {
        let hard_reject = decision.blockers.iter().any(|blocker| {
            matches!(
                blocker.as_str(),
                "series_scope_differs_from_case_scope"
                    | "invalid_exposure_end_or_artifact_digest"
                    | "event_time_order_window_or_failure_definition_invalid"
                    | "event_not_known_at_declared_cutoff"
                    | "analysis_protocol_or_thresholds_not_predeclared"
            )
        });
        if hard_reject {
            decision.disposition = Some(Disposition::Rejected);
        } else if !decision.blockers.is_empty() {
            decision.disposition = Some(Disposition::Indeterminate);
        } else {
            decision.disposition = Some(Disposition::EstimateProduced);
        }
    }

    decision.blockers.sort();
    decision.blockers.dedup();
    Ok(decision)
}

pub fn evaluate(corpus: &Corpus) -> Result<Report, String> {
    if corpus.schema != SCHEMA {
        return Err("unsupported power-law corpus schema".to_owned());
    }
    if corpus.evidence_class != "synthetic" {
        return Err("v0.1 accepts synthetic power-law fixtures only".to_owned());
    }
    if corpus.claim_ceiling != CLAIM_CEILING {
        return Err("unsupported power-law claim ceiling".to_owned());
    }
    if corpus.cases.is_empty() {
        return Err("power-law corpus must contain cases".to_owned());
    }

    let mut case_ids = HashSet::new();
    let mut summary = Summary {
        case_count: corpus.cases.len(),
        estimate_produced: 0,
        indeterminate: 0,
        model_not_supported: 0,
        rejected: 0,
        disputed: 0,
        total_unique_events: 0,
        total_duplicate_events: 0,
        expected_result_mismatches: 0,
        inferential_fit_acceptances: 0,
        reliability_claims_granted: 0,
        authority_grants: 0,
    };
    let mut results = Vec::with_capacity(corpus.cases.len());

    for case in &corpus.cases {
        if !case_ids.insert(case.case_id.as_str()) {
            return Err("power-law case IDs must be unique".to_owned());
        }
        let decision = evaluate_case(case)?;
        let disposition = decision.disposition.unwrap_or(Disposition::Indeterminate);
        match disposition {
            Disposition::EstimateProduced => summary.estimate_produced += 1,
            Disposition::Indeterminate => summary.indeterminate += 1,
            Disposition::ModelNotSupported => summary.model_not_supported += 1,
            Disposition::Rejected => summary.rejected += 1,
            Disposition::Disputed => summary.disputed += 1,
        }
        summary.total_unique_events += decision.unique_events;
        summary.total_duplicate_events += decision.duplicate_events;

        let close =
            |actual: Option<f64>, expected: Option<f64>, tolerance: f64| match (actual, expected) {
                (Some(left), Some(right)) => {
                    (left - right).abs() <= tolerance * left.abs().max(1.0)
                }
                (None, None) => true,
                _ => false,
            };
        let expected = &case.expected;
        let matched = disposition == expected.disposition
            && decision.unique_events == expected.unique_events
            && decision.duplicate_events == expected.duplicate_events
            && decision.exposure_end_seconds == expected.exposure_end_seconds
            && close(decision.beta_hat, expected.beta_hat, 1e-10)
            && close(decision.log_eta_hat, expected.log_eta_hat, 1e-10)
            && close(
                decision.intensity_at_end_per_second,
                expected.intensity_at_end_per_second,
                1e-10,
            )
            && close(
                decision.cumulative_mean_at_half_exposure,
                expected.cumulative_mean_at_half_exposure,
                1e-10,
            )
            && close(
                decision.descriptive_likelihood_ratio_statistic,
                expected.descriptive_likelihood_ratio_statistic,
                1e-10,
            )
            && decision.likelihood_ratio_nonnegative == expected.likelihood_ratio_nonnegative
            && !expected.inferential_fit_accepted
            && !expected.reliability_claim_granted
            && decision.blockers == expected.blockers;

        summary.expected_result_mismatches += usize::from(!matched);
        summary.inferential_fit_acceptances += usize::from(expected.inferential_fit_accepted);
        summary.reliability_claims_granted += usize::from(expected.reliability_claim_granted);

        results.push(CaseResult {
            case_id: case.case_id.clone(),
            disposition,
            unique_events: decision.unique_events,
            duplicate_events: decision.duplicate_events,
            exposure_end_seconds: decision.exposure_end_seconds,
            beta_hat: decision.beta_hat,
            log_eta_hat: decision.log_eta_hat,
            intensity_at_end_per_second: decision.intensity_at_end_per_second,
            cumulative_mean_at_half_exposure: decision.cumulative_mean_at_half_exposure,
            descriptive_likelihood_ratio_statistic: decision.descriptive_likelihood_ratio_statistic,
            likelihood_ratio_nonnegative: decision.likelihood_ratio_nonnegative,
            inferential_fit_accepted: false,
            reliability_claim_granted: false,
            authority_granted: false,
            blockers: decision.blockers,
            expected_result_matched: matched,
        });
    }

    Ok(Report {
        result: if summary.expected_result_mismatches == 0
            && summary.inferential_fit_acceptances == 0
            && summary.reliability_claims_granted == 0
            && summary.authority_grants == 0
        {
            "pass".to_owned()
        } else {
            "fail".to_owned()
        },
        evidence_class: "synthetic".to_owned(),
        claim_ceiling: CLAIM_CEILING.to_owned(),
        authority_granted: false,
        summary,
        cases: results,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposure_scaling_preserves_beta_and_changes_log_eta() {
        let first = estimate_power_law(&[100, 200, 400], 1000).expect("valid first exposure");
        let second = estimate_power_law(&[200, 400, 800], 2000).expect("scaled exposure");
        assert!((first.0 - second.0).abs() < 1e-12);
        assert!((second.1 - first.1 + first.0 * 2.0_f64.ln()).abs() < 1e-12);
    }

    #[test]
    fn end_intensity_is_beta_times_count_over_exposure() {
        let estimate = estimate_power_law(&[100, 200, 400], 1000).expect("valid history");
        let expected = estimate.0 * 3.0 / 1000.0;
        assert!((estimate.2 - expected).abs() < 1e-12);
    }

    #[test]
    fn likelihood_ratio_statistic_is_nonnegative() {
        for times in [&[100, 200][..], &[400, 700][..], &[100, 300, 800][..]] {
            let estimate = estimate_power_law(times, 1000).expect("valid history");
            assert!(estimate.4 >= 0.0);
        }
    }

    #[test]
    fn fewer_than_two_events_is_rejected() {
        assert!(estimate_power_law(&[100], 1000).is_err());
    }

    #[test]
    fn tied_or_unsorted_event_times_are_rejected() {
        assert!(estimate_power_law(&[100, 100, 200], 1000).is_err());
        assert!(estimate_power_law(&[200, 100, 300], 1000).is_err());
    }

    #[test]
    fn event_at_zero_or_at_end_is_rejected() {
        assert!(estimate_power_law(&[0, 100], 1000).is_err());
        assert!(estimate_power_law(&[100, 1000], 1000).is_err());
    }

    #[test]
    fn invalid_exposure_is_rejected() {
        assert!(estimate_power_law(&[100, 200], 0).is_err());
    }
}
