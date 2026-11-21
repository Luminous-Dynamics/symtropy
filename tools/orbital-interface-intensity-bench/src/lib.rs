use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

const CORPUS_SCHEMA: &str = "luminous.orbital-interface-intensity.corpus.v0.1";
const CLAIM_CEILING: &str = "synthetic-constant-failure-intensity-only";
const SAMPLING_DESIGN: &str = "simple_random_independent_articles_common_intensity";
const MAX_ARTICLES: usize = 100_000;
const MAX_FAILURE_EVENTS: u64 = 100_000;
const MAX_EXPOSURE_SECONDS: u64 = 9_007_199_254_740_991;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CensoringDisposition {
    AdministrativeEnd,
    FailureTerminated,
    Intervention,
    InformativeWithdrawal,
    Unknown,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Disposition {
    RateDemonstrated,
    RateNotDemonstrated,
    Indeterminate,
    ModelNotSupported,
    Rejected,
    Disputed,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ArticleExposure {
    pub record_id: String,
    pub article_serial: String,
    pub sampling_cluster_id: String,
    pub population_id: String,
    pub lot_id: String,
    pub configuration_id: String,
    pub operating_mode_id: String,
    pub restoration_policy_id: String,
    pub failure_definition_id: String,
    pub exposure_seconds: u64,
    pub failure_events: u64,
    pub raw_artifact_sha256: String,
    pub evidence_complete: bool,
    pub operating_time_verified: bool,
    pub event_capture_complete: bool,
    pub intervention_during_exposure: bool,
    pub censoring: CensoringDisposition,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Expected {
    pub disposition: Disposition,
    pub unique_articles: usize,
    pub total_exposure_seconds: u64,
    pub failure_events: u64,
    pub duplicate_records: usize,
    pub upper_bound_per_million_hours: Option<f64>,
    pub meets_target: Option<bool>,
    pub observed_rate_per_million_hours: Option<f64>,
    pub naive_false_assurance: bool,
    pub minimum_zero_failure_exposure_hours: Option<f64>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct IntensityCase {
    pub case_id: String,
    pub preregistration_ref: Option<String>,
    pub sampling_frame_ref: Option<String>,
    pub dependence_assessment_ref: Option<String>,
    pub assumption_assessment_ref: Option<String>,
    pub population_id: String,
    pub lot_id: String,
    pub configuration_id: String,
    pub operating_mode_id: String,
    pub restoration_policy_id: String,
    pub failure_definition_id: String,
    pub sampling_design: String,
    pub target_failure_intensity_per_million_hours: f64,
    pub confidence: f64,
    pub articles: Vec<ArticleExposure>,
    pub expected: Expected,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Corpus {
    pub schema: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub cases: Vec<IntensityCase>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CaseResult {
    pub case_id: String,
    pub disposition: Disposition,
    pub unique_articles: usize,
    pub total_exposure_seconds: u64,
    pub failure_events: u64,
    pub duplicate_records: usize,
    pub upper_bound_per_million_hours: Option<f64>,
    pub meets_target: Option<bool>,
    pub observed_rate_per_million_hours: Option<f64>,
    pub naive_false_assurance: bool,
    pub minimum_zero_failure_exposure_hours: Option<f64>,
    pub expected_result_matched: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Summary {
    pub case_count: usize,
    pub rate_demonstrated: usize,
    pub rate_not_demonstrated: usize,
    pub indeterminate: usize,
    pub model_not_supported: usize,
    pub rejected: usize,
    pub disputed: usize,
    pub unique_articles: usize,
    pub total_exposure_seconds: u64,
    pub failure_events: u64,
    pub duplicate_records: usize,
    pub naive_false_assurance_cases: usize,
    pub expected_result_mismatches: usize,
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

#[derive(Debug, Clone)]
struct Decision {
    disposition: Disposition,
    unique_articles: usize,
    total_exposure_seconds: u64,
    failure_events: u64,
    duplicate_records: usize,
    upper_bound_per_million_hours: Option<f64>,
    meets_target: Option<bool>,
    observed_rate_per_million_hours: Option<f64>,
    naive_false_assurance: bool,
    minimum_zero_failure_exposure_hours: Option<f64>,
}

fn empty_decision(disposition: Disposition, duplicate_records: usize) -> Decision {
    Decision {
        disposition,
        unique_articles: 0,
        total_exposure_seconds: 0,
        failure_events: 0,
        duplicate_records,
        upper_bound_per_million_hours: None,
        meets_target: None,
        observed_rate_per_million_hours: None,
        naive_false_assurance: false,
        minimum_zero_failure_exposure_hours: None,
    }
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn log_add_exp(left: f64, right: f64) -> f64 {
    if left == f64::NEG_INFINITY {
        return right;
    }
    if right == f64::NEG_INFINITY {
        return left;
    }
    let maximum = left.max(right);
    maximum + ((left - maximum).exp() + (right - maximum).exp()).ln()
}

fn log_poisson_cdf_at_most(events: u64, mean: f64) -> f64 {
    if events == 0 {
        return -mean;
    }
    if mean == 0.0 {
        return 0.0;
    }
    if !mean.is_finite() {
        return f64::NEG_INFINITY;
    }

    let log_mean = mean.ln();
    let mut log_mass = -mean;
    let mut log_sum = log_mass;
    for index in 1..=events {
        log_mass += log_mean - (index as f64).ln();
        log_sum = log_add_exp(log_sum, log_mass);
    }
    log_sum.min(0.0)
}

/// Exact one-sided upper confidence limit for the mean of a Poisson count.
pub fn poisson_mean_upper_bound(events: u64, confidence: f64) -> Result<f64, String> {
    if events > MAX_FAILURE_EVENTS {
        return Err("failure-event count exceeds the v0.1 limit".to_owned());
    }
    if !(confidence.is_finite() && 0.0 < confidence && confidence < 1.0) {
        return Err("confidence must be finite and strictly between zero and one".to_owned());
    }

    let alpha = 1.0 - confidence;
    if events == 0 {
        return Ok(-alpha.ln());
    }

    let log_alpha = alpha.ln();
    let event_count = events as f64;
    let mut lower = 0.0;
    let mut upper = (event_count + 12.0 * (event_count + 1.0).sqrt() + 10.0).max(-alpha.ln() + 1.0);

    let mut bracketed = false;
    for _ in 0..128 {
        if log_poisson_cdf_at_most(events, upper) <= log_alpha {
            bracketed = true;
            break;
        }
        upper *= 2.0;
        if !upper.is_finite() {
            break;
        }
    }
    if !bracketed {
        return Err("could not bracket the Poisson upper confidence limit".to_owned());
    }

    for _ in 0..120 {
        let midpoint = lower + (upper - lower) / 2.0;
        if log_poisson_cdf_at_most(events, midpoint) > log_alpha {
            lower = midpoint;
        } else {
            upper = midpoint;
        }
    }
    Ok(upper)
}

/// Exact one-sided upper confidence limit for failure intensity, in events per million hours.
pub fn failure_intensity_upper_bound(
    events: u64,
    exposure_seconds: u64,
    confidence: f64,
) -> Result<f64, String> {
    if exposure_seconds == 0 || exposure_seconds > MAX_EXPOSURE_SECONDS {
        return Err("exposure seconds must be positive and within the v0.1 limit".to_owned());
    }
    let exposure_hours = exposure_seconds as f64 / 3_600.0;
    let mean_upper = poisson_mean_upper_bound(events, confidence)?;
    Ok(mean_upper / exposure_hours * 1_000_000.0)
}

/// Required exposure for an upper bound to meet a target intensity when up to
/// maximum_failures events are observed.
pub fn minimum_exposure_hours_for_maximum_failures(
    maximum_failures: u64,
    target_per_million_hours: f64,
    confidence: f64,
) -> Result<f64, String> {
    if !target_per_million_hours.is_finite() || target_per_million_hours <= 0.0 {
        return Err("target intensity must be finite and positive".to_owned());
    }
    let mean_upper = poisson_mean_upper_bound(maximum_failures, confidence)?;
    let exposure = mean_upper * 1_000_000.0 / target_per_million_hours;
    if !exposure.is_finite() || exposure <= 0.0 {
        return Err("required exposure is outside the v0.1 numeric range".to_owned());
    }
    Ok(exposure)
}

/// Zero-failure special case retained as an explicit convenience function.
pub fn minimum_zero_failure_exposure_hours(
    target_per_million_hours: f64,
    confidence: f64,
) -> Result<f64, String> {
    minimum_exposure_hours_for_maximum_failures(0, target_per_million_hours, confidence)
}

fn evaluate_case(case: &IntensityCase) -> Result<Decision, String> {
    if case.case_id.trim().is_empty() {
        return Err("case ID must be non-empty".to_owned());
    }
    if !(case.target_failure_intensity_per_million_hours.is_finite()
        && case.target_failure_intensity_per_million_hours > 0.0
        && case.confidence.is_finite()
        && 0.0 < case.confidence
        && case.confidence < 1.0)
    {
        return Ok(empty_decision(Disposition::Rejected, 0));
    }
    if case.articles.is_empty() {
        return Ok(empty_decision(Disposition::Indeterminate, 0));
    }
    if case.articles.len() > MAX_ARTICLES {
        return Err("article record count exceeds the v0.1 limit".to_owned());
    }
    if case.sampling_design != SAMPLING_DESIGN {
        return Ok(empty_decision(Disposition::ModelNotSupported, 0));
    }
    if case
        .preregistration_ref
        .as_deref()
        .is_none_or(|value| value.trim().is_empty())
        || case
            .sampling_frame_ref
            .as_deref()
            .is_none_or(|value| value.trim().is_empty())
        || case
            .dependence_assessment_ref
            .as_deref()
            .is_none_or(|value| value.trim().is_empty())
        || case
            .assumption_assessment_ref
            .as_deref()
            .is_none_or(|value| value.trim().is_empty())
    {
        return Ok(empty_decision(Disposition::Indeterminate, 0));
    }

    let mut by_record_id: HashMap<&str, &ArticleExposure> = HashMap::new();
    let mut by_article: HashMap<&str, &ArticleExposure> = HashMap::new();
    let mut by_cluster: HashMap<&str, &ArticleExposure> = HashMap::new();
    let mut unique_records: Vec<&ArticleExposure> = Vec::with_capacity(case.articles.len());
    let mut duplicate_records = 0usize;

    for article in &case.articles {
        if article.record_id.trim().is_empty()
            || article.article_serial.trim().is_empty()
            || article.sampling_cluster_id.trim().is_empty()
        {
            return Ok(empty_decision(
                Disposition::Indeterminate,
                duplicate_records,
            ));
        }

        if let Some(previous) = by_record_id.get(article.record_id.as_str()) {
            if *previous == article {
                duplicate_records += 1;
                continue;
            }
            return Ok(empty_decision(Disposition::Disputed, duplicate_records));
        }

        if let Some(previous) = by_article.get(article.article_serial.as_str()) {
            let changed_result = previous.exposure_seconds != article.exposure_seconds
                || previous.failure_events != article.failure_events
                || previous.raw_artifact_sha256 != article.raw_artifact_sha256;
            return Ok(empty_decision(
                if changed_result {
                    Disposition::Disputed
                } else {
                    Disposition::Indeterminate
                },
                duplicate_records,
            ));
        }

        if by_cluster.contains_key(article.sampling_cluster_id.as_str()) {
            return Ok(empty_decision(
                Disposition::Indeterminate,
                duplicate_records,
            ));
        }

        by_record_id.insert(&article.record_id, article);
        by_article.insert(&article.article_serial, article);
        by_cluster.insert(&article.sampling_cluster_id, article);
        unique_records.push(article);
    }

    if unique_records.len() < 2 {
        return Ok(empty_decision(
            Disposition::Indeterminate,
            duplicate_records,
        ));
    }

    for article in &unique_records {
        if article.population_id != case.population_id
            || article.lot_id != case.lot_id
            || article.configuration_id != case.configuration_id
            || article.operating_mode_id != case.operating_mode_id
            || article.restoration_policy_id != case.restoration_policy_id
            || article.failure_definition_id != case.failure_definition_id
        {
            return Ok(empty_decision(Disposition::Rejected, duplicate_records));
        }
        if article.exposure_seconds == 0
            || article.exposure_seconds > MAX_EXPOSURE_SECONDS
            || article.failure_events > MAX_FAILURE_EVENTS
            || !valid_digest(&article.raw_artifact_sha256)
            || !article.evidence_complete
            || !article.operating_time_verified
            || !article.event_capture_complete
            || article.intervention_during_exposure
            || (article.censoring == CensoringDisposition::FailureTerminated
                && article.failure_events == 0)
            || matches!(
                article.censoring,
                CensoringDisposition::Intervention
                    | CensoringDisposition::InformativeWithdrawal
                    | CensoringDisposition::Unknown
            )
        {
            return Ok(empty_decision(
                Disposition::Indeterminate,
                duplicate_records,
            ));
        }
    }

    let mut total_exposure_seconds = 0u64;
    let mut failure_events = 0u64;
    for article in &unique_records {
        total_exposure_seconds = total_exposure_seconds
            .checked_add(article.exposure_seconds)
            .ok_or_else(|| "total exposure seconds overflow".to_owned())?;
        failure_events = failure_events
            .checked_add(article.failure_events)
            .ok_or_else(|| "total failure-event count overflow".to_owned())?;
    }
    if total_exposure_seconds == 0 || total_exposure_seconds > MAX_EXPOSURE_SECONDS {
        return Ok(empty_decision(
            Disposition::Indeterminate,
            duplicate_records,
        ));
    }
    if failure_events > MAX_FAILURE_EVENTS {
        return Err("total failure-event count exceeds the v0.1 limit".to_owned());
    }

    let upper_bound =
        failure_intensity_upper_bound(failure_events, total_exposure_seconds, case.confidence)?;
    let exposure_hours = total_exposure_seconds as f64 / 3_600.0;
    let observed_rate = failure_events as f64 / exposure_hours * 1_000_000.0;
    let target = case.target_failure_intensity_per_million_hours;
    let meets_target = upper_bound <= target;
    let naive_false_assurance = observed_rate <= target && !meets_target;
    let required_exposure = minimum_zero_failure_exposure_hours(target, case.confidence)?;
    let disposition = if meets_target {
        Disposition::RateDemonstrated
    } else {
        Disposition::RateNotDemonstrated
    };

    Ok(Decision {
        disposition,
        unique_articles: unique_records.len(),
        total_exposure_seconds,
        failure_events,
        duplicate_records,
        upper_bound_per_million_hours: Some(upper_bound),
        meets_target: Some(meets_target),
        observed_rate_per_million_hours: Some(observed_rate),
        naive_false_assurance,
        minimum_zero_failure_exposure_hours: Some(required_exposure),
    })
}

pub fn evaluate(corpus: &Corpus) -> Result<Report, String> {
    if corpus.schema != CORPUS_SCHEMA {
        return Err("unsupported intensity corpus schema".to_owned());
    }
    if corpus.evidence_class != "synthetic" {
        return Err("v0.1 accepts synthetic intensity fixtures only".to_owned());
    }
    if corpus.claim_ceiling != CLAIM_CEILING {
        return Err("unsupported intensity claim ceiling".to_owned());
    }
    if corpus.cases.is_empty() {
        return Err("intensity corpus must contain cases".to_owned());
    }

    let mut case_ids = HashSet::with_capacity(corpus.cases.len());
    let mut summary = Summary {
        case_count: corpus.cases.len(),
        rate_demonstrated: 0,
        rate_not_demonstrated: 0,
        indeterminate: 0,
        model_not_supported: 0,
        rejected: 0,
        disputed: 0,
        unique_articles: 0,
        total_exposure_seconds: 0,
        failure_events: 0,
        duplicate_records: 0,
        naive_false_assurance_cases: 0,
        expected_result_mismatches: 0,
    };
    let mut results = Vec::with_capacity(corpus.cases.len());

    for case in &corpus.cases {
        if !case_ids.insert(case.case_id.as_str()) {
            return Err("intensity case IDs must be unique".to_owned());
        }
        let decision = evaluate_case(case)?;
        match decision.disposition {
            Disposition::RateDemonstrated => summary.rate_demonstrated += 1,
            Disposition::RateNotDemonstrated => summary.rate_not_demonstrated += 1,
            Disposition::Indeterminate => summary.indeterminate += 1,
            Disposition::ModelNotSupported => summary.model_not_supported += 1,
            Disposition::Rejected => summary.rejected += 1,
            Disposition::Disputed => summary.disputed += 1,
        }
        summary.unique_articles += decision.unique_articles;
        summary.total_exposure_seconds = summary
            .total_exposure_seconds
            .checked_add(decision.total_exposure_seconds)
            .ok_or_else(|| "summary exposure seconds overflow".to_owned())?;
        summary.failure_events = summary
            .failure_events
            .checked_add(decision.failure_events)
            .ok_or_else(|| "summary failure-event count overflow".to_owned())?;
        summary.duplicate_records += decision.duplicate_records;
        summary.naive_false_assurance_cases += usize::from(decision.naive_false_assurance);

        let expected = &case.expected;
        let close =
            |actual: Option<f64>, expected: Option<f64>, tolerance: f64| match (actual, expected) {
                (Some(left), Some(right)) => {
                    (left - right).abs() <= tolerance * left.abs().max(1.0)
                }
                (None, None) => true,
                _ => false,
            };
        let matched = decision.disposition == expected.disposition
            && decision.unique_articles == expected.unique_articles
            && decision.total_exposure_seconds == expected.total_exposure_seconds
            && decision.failure_events == expected.failure_events
            && decision.duplicate_records == expected.duplicate_records
            && close(
                decision.upper_bound_per_million_hours,
                expected.upper_bound_per_million_hours,
                1e-10,
            )
            && decision.meets_target == expected.meets_target
            && close(
                decision.observed_rate_per_million_hours,
                expected.observed_rate_per_million_hours,
                1e-12,
            )
            && decision.naive_false_assurance == expected.naive_false_assurance
            && close(
                decision.minimum_zero_failure_exposure_hours,
                expected.minimum_zero_failure_exposure_hours,
                1e-10,
            );
        if !matched {
            summary.expected_result_mismatches += 1;
        }

        results.push(CaseResult {
            case_id: case.case_id.clone(),
            disposition: decision.disposition,
            unique_articles: decision.unique_articles,
            total_exposure_seconds: decision.total_exposure_seconds,
            failure_events: decision.failure_events,
            duplicate_records: decision.duplicate_records,
            upper_bound_per_million_hours: decision.upper_bound_per_million_hours,
            meets_target: decision.meets_target,
            observed_rate_per_million_hours: decision.observed_rate_per_million_hours,
            naive_false_assurance: decision.naive_false_assurance,
            minimum_zero_failure_exposure_hours: decision.minimum_zero_failure_exposure_hours,
            expected_result_matched: matched,
        });
    }

    Ok(Report {
        result: if summary.expected_result_mismatches == 0 {
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
    fn zero_event_mean_bound_is_exact() {
        let bound = poisson_mean_upper_bound(0, 0.90).expect("valid input");
        assert!((bound - std::f64::consts::LN_10).abs() < 1e-12);
    }

    #[test]
    fn one_event_mean_bound_matches_known_value() {
        let bound = poisson_mean_upper_bound(1, 0.90).expect("valid input");
        assert!((bound - 3.889_720_169_867_429).abs() < 1e-9);
    }

    #[test]
    fn two_event_mean_bound_is_above_one_event_bound() {
        let one = poisson_mean_upper_bound(1, 0.90).expect("valid input");
        let two = poisson_mean_upper_bound(2, 0.90).expect("valid input");
        assert!(two > one);
    }

    #[test]
    fn higher_confidence_requires_a_larger_upper_bound() {
        let ninety = poisson_mean_upper_bound(2, 0.90).expect("valid input");
        let ninety_five = poisson_mean_upper_bound(2, 0.95).expect("valid input");
        assert!(ninety_five > ninety);
    }

    #[test]
    fn upper_rate_scales_inverse_linearly_with_exposure() {
        let short = failure_intensity_upper_bound(0, 3_600_000, 0.90).expect("valid input");
        let long = failure_intensity_upper_bound(0, 7_200_000, 0.90).expect("valid input");
        assert!((short / long - 2.0).abs() < 1e-12);
    }

    #[test]
    fn one_event_has_positive_upper_rate() {
        let bound = failure_intensity_upper_bound(1, 36_000_000, 0.90).expect("valid input");
        assert!(bound > 0.0);
    }

    #[test]
    fn zero_failure_exposure_known_answer() {
        let hours = minimum_zero_failure_exposure_hours(100.0, 0.90).expect("valid input");
        assert!((hours - 23_025.850_929_940_46).abs() < 1e-8);
    }

    #[test]
    fn one_allowed_failure_requires_more_exposure_than_zero_failures() {
        let zero = minimum_exposure_hours_for_maximum_failures(0, 100.0, 0.90)
            .expect("valid zero-failure design");
        let one = minimum_exposure_hours_for_maximum_failures(1, 100.0, 0.90)
            .expect("valid one-failure design");
        assert!((one - 38_897.201_698_674_29).abs() < 1e-7);
        assert!(one > zero);
    }

    #[test]
    fn exposure_sizing_rejects_excessive_failure_count() {
        assert!(
            minimum_exposure_hours_for_maximum_failures(MAX_FAILURE_EVENTS + 1, 100.0, 0.90)
                .is_err()
        );
    }

    #[test]
    fn zero_target_is_rejected() {
        assert!(minimum_zero_failure_exposure_hours(0.0, 0.90).is_err());
    }

    #[test]
    fn invalid_confidence_is_rejected() {
        assert!(poisson_mean_upper_bound(0, 1.0).is_err());
    }

    #[test]
    fn excessive_event_count_is_rejected() {
        assert!(poisson_mean_upper_bound(MAX_FAILURE_EVENTS + 1, 0.90).is_err());
    }
}
