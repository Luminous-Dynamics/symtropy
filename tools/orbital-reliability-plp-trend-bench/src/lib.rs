use serde::{Deserialize, Serialize};
use std::collections::HashSet;

const CORPUS_SCHEMA: &str = "luminous.orbital-reliability-plp-trend.corpus.v0.1";
const CLAIM_CEILING: &str = "synthetic-constant-vs-plp-trend-diagnostic-only";
const MAX_EVENTS: usize = 10_000;
const MAX_EXPOSURE_SECONDS: u64 = 9_007_199_254_740_991;
const EPSILON: f64 = 1.0e-13;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Disposition {
    TrendSignalDetected,
    NoTrendSignalDetected,
    Indeterminate,
    ModelNotSupported,
    Rejected,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TrendCase {
    pub case_id: String,
    pub model_profile: String,
    pub admissibility_receipt_sha256: String,
    pub admissibility_source_commit: String,
    pub exposure_end_seconds: u64,
    pub event_times_seconds: Vec<u64>,
    pub significance_level: f64,
    pub expected: Expected,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Expected {
    pub disposition: Disposition,
    pub event_count: usize,
    pub beta_hat: Option<f64>,
    pub likelihood_ratio_statistic: Option<f64>,
    pub exact_conditional_p_value: Option<f64>,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Corpus {
    pub schema: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub cases: Vec<TrendCase>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CaseResult {
    pub case_id: String,
    pub disposition: Disposition,
    pub event_count: usize,
    pub beta_hat: Option<f64>,
    pub likelihood_ratio_statistic: Option<f64>,
    pub exact_conditional_p_value: Option<f64>,
    pub trend_signal_detected: bool,
    pub fit_acceptance_granted: bool,
    pub reliability_claim_granted: bool,
    pub authority_granted: bool,
    pub blockers: Vec<String>,
    pub expected_result_matched: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Summary {
    pub case_count: usize,
    pub trend_signal_detected: usize,
    pub no_trend_signal_detected: usize,
    pub indeterminate: usize,
    pub model_not_supported: usize,
    pub rejected: usize,
    pub expected_result_mismatches: usize,
    pub fit_acceptances: usize,
    pub reliability_claims_granted: usize,
    pub authority_grants: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct Report {
    pub result: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub fit_acceptance_granted: bool,
    pub reliability_claim_granted: bool,
    pub authority_granted: bool,
    pub summary: Summary,
    pub cases: Vec<CaseResult>,
}

#[derive(Debug, Clone)]
struct Decision {
    disposition: Disposition,
    event_count: usize,
    beta_hat: Option<f64>,
    likelihood_ratio_statistic: Option<f64>,
    exact_conditional_p_value: Option<f64>,
    blockers: Vec<String>,
}

fn push_blocker(blockers: &mut Vec<String>, code: &str) {
    if !blockers.iter().any(|existing| existing == code) {
        blockers.push(code.to_owned());
    }
}

fn valid_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn log_gamma_integer(shape: usize) -> f64 {
    (1..shape).map(|value| (value as f64).ln()).sum()
}

/// Regularized lower incomplete gamma P(a, x), using series / continued fraction.
fn regularized_gamma_p(a: f64, x: f64) -> Result<f64, String> {
    if !a.is_finite() || a <= 0.0 || !x.is_finite() || x < 0.0 {
        return Err("invalid regularized-gamma arguments".to_owned());
    }
    if x == 0.0 {
        return Ok(0.0);
    }

    let log_factor = -x + a * x.ln() - log_gamma_integer(a as usize);
    if x < a + 1.0 {
        let mut ap = a;
        let mut term = 1.0 / a;
        let mut sum = term;
        for _ in 0..100_000 {
            ap += 1.0;
            term *= x / ap;
            sum += term;
            if term.abs() <= sum.abs() * EPSILON {
                break;
            }
        }
        return Ok((sum * log_factor.exp()).clamp(0.0, 1.0));
    }

    let tiny = 1.0e-300;
    let mut b = x + 1.0 - a;
    let mut c = 1.0 / tiny;
    let mut d = 1.0 / b;
    let mut h = d;
    for index in 1..=100_000 {
        let i = index as f64;
        let an = -i * (i - a);
        b += 2.0;
        d = an * d + b;
        if d.abs() < tiny {
            d = tiny;
        }
        c = b + an / c;
        if c.abs() < tiny {
            c = tiny;
        }
        d = 1.0 / d;
        let delta = d * c;
        h *= delta;
        if (delta - 1.0).abs() <= EPSILON {
            break;
        }
    }
    let q = (log_factor.exp() * h).clamp(0.0, 1.0);
    Ok((1.0 - q).clamp(0.0, 1.0))
}

fn trend_deviance(ratio: f64) -> f64 {
    ratio - 1.0 - ratio.ln()
}

fn lower_ratio_root(deviance: f64) -> f64 {
    let mut lower = 0.0;
    let mut upper = 1.0;
    for _ in 0..120 {
        let midpoint = lower + (upper - lower) / 2.0;
        if midpoint == 0.0 || trend_deviance(midpoint) > deviance {
            lower = midpoint;
        } else {
            upper = midpoint;
        }
    }
    lower + (upper - lower) / 2.0
}

fn upper_ratio_root(deviance: f64) -> Result<f64, String> {
    let mut lower = 1.0;
    let mut upper = 2.0;
    for _ in 0..128 {
        if trend_deviance(upper) >= deviance {
            break;
        }
        upper *= 2.0;
        if !upper.is_finite() {
            return Err("could not bracket upper trend root".to_owned());
        }
    }
    if trend_deviance(upper) < deviance {
        return Err("could not bracket upper trend root".to_owned());
    }
    for _ in 0..120 {
        let midpoint = lower + (upper - lower) / 2.0;
        if trend_deviance(midpoint) < deviance {
            lower = midpoint;
        } else {
            upper = midpoint;
        }
    }
    Ok(lower + (upper - lower) / 2.0)
}

/// Exact conditional two-sided likelihood-ratio p-value for H0: beta = 1.
///
/// Conditional on n events under a homogeneous Poisson process, the event
/// locations are iid uniform on (0, T). Therefore S = sum(log(T / t_i))
/// follows Gamma(n, 1), and beta_hat = n / S.
pub fn exact_conditional_trend_p_value(event_count: usize, ratio: f64) -> Result<f64, String> {
    if !(2..=MAX_EVENTS).contains(&event_count) {
        return Err("event count is outside the supported v0.1 range".to_owned());
    }
    if !ratio.is_finite() || ratio <= 0.0 {
        return Err("exposure ratio must be finite and positive".to_owned());
    }
    let deviance = trend_deviance(ratio).max(0.0);
    if deviance <= EPSILON {
        return Ok(1.0);
    }

    let lower_root = lower_ratio_root(deviance);
    let upper_root = upper_ratio_root(deviance)?;
    let shape = event_count as f64;
    let lower_tail = regularized_gamma_p(shape, shape * lower_root)?;
    let upper_cdf = regularized_gamma_p(shape, shape * upper_root)?;
    Ok((lower_tail + (1.0 - upper_cdf)).clamp(0.0, 1.0))
}

/// Estimates beta and the exact conditional LRT against beta=1.
pub fn estimate_trend(
    event_times_seconds: &[u64],
    exposure_end_seconds: u64,
) -> Result<(f64, f64, f64), String> {
    if exposure_end_seconds == 0 || exposure_end_seconds > MAX_EXPOSURE_SECONDS {
        return Err("exposure endpoint is outside the supported v0.1 range".to_owned());
    }
    if event_times_seconds.len() < 2 || event_times_seconds.len() > MAX_EVENTS {
        return Err("event count is outside the supported v0.1 range".to_owned());
    }
    let exposure = exposure_end_seconds as f64;
    let mut sum_log_ratio = 0.0;
    let mut previous = 0u64;
    for time in event_times_seconds {
        if *time == 0 || *time >= exposure_end_seconds || *time <= previous {
            return Err(
                "event times must be strictly increasing inside the observation window".to_owned(),
            );
        }
        sum_log_ratio += (exposure / *time as f64).ln();
        previous = *time;
    }
    if !sum_log_ratio.is_finite() || sum_log_ratio <= 0.0 {
        return Err("event-time log exposure sum is invalid".to_owned());
    }
    let count = event_times_seconds.len() as f64;
    let beta_hat = count / sum_log_ratio;
    let ratio = sum_log_ratio / count;
    let likelihood_ratio = (2.0 * count * trend_deviance(ratio)).max(0.0);
    let p_value = exact_conditional_trend_p_value(event_times_seconds.len(), ratio)?;
    Ok((beta_hat, likelihood_ratio, p_value))
}

fn evaluate_case(case: &TrendCase) -> Decision {
    let mut blockers = Vec::new();
    if case.model_profile != "power_law_process_repairable_minimal_repair" {
        push_blocker(
            &mut blockers,
            "selected_model_profile_is_not_power_law_minimal_repair",
        );
        return Decision {
            disposition: Disposition::ModelNotSupported,
            event_count: case.event_times_seconds.len(),
            beta_hat: None,
            likelihood_ratio_statistic: None,
            exact_conditional_p_value: None,
            blockers,
        };
    }
    if !valid_hex(&case.admissibility_receipt_sha256, 64)
        || !valid_hex(&case.admissibility_source_commit, 40)
    {
        push_blocker(
            &mut blockers,
            "missing_or_malformed_admissibility_receipt_pin",
        );
        return Decision {
            disposition: Disposition::Indeterminate,
            event_count: case.event_times_seconds.len(),
            beta_hat: None,
            likelihood_ratio_statistic: None,
            exact_conditional_p_value: None,
            blockers,
        };
    }
    if !(case.significance_level.is_finite()
        && 0.0 < case.significance_level
        && case.significance_level < 1.0)
    {
        push_blocker(&mut blockers, "invalid_significance_level");
        return Decision {
            disposition: Disposition::Rejected,
            event_count: case.event_times_seconds.len(),
            beta_hat: None,
            likelihood_ratio_statistic: None,
            exact_conditional_p_value: None,
            blockers,
        };
    }

    match estimate_trend(&case.event_times_seconds, case.exposure_end_seconds) {
        Ok((beta_hat, likelihood_ratio, p_value)) => Decision {
            disposition: if p_value <= case.significance_level {
                Disposition::TrendSignalDetected
            } else {
                Disposition::NoTrendSignalDetected
            },
            event_count: case.event_times_seconds.len(),
            beta_hat: Some(beta_hat),
            likelihood_ratio_statistic: Some(likelihood_ratio),
            exact_conditional_p_value: Some(p_value),
            blockers,
        },
        Err(error) => {
            let invalid_window = case.exposure_end_seconds == 0
                || case.exposure_end_seconds > MAX_EXPOSURE_SECONDS
                || case
                    .event_times_seconds
                    .iter()
                    .any(|time| *time == 0 || *time >= case.exposure_end_seconds)
                || case
                    .event_times_seconds
                    .windows(2)
                    .any(|pair| pair[0] >= pair[1]);
            let insufficient = case.event_times_seconds.len() < 2;
            let code = if invalid_window {
                "event_history_or_exposure_window_invalid"
            } else if insufficient {
                "fewer_than_two_events_for_trend_test"
            } else {
                "insufficient_or_numerically_invalid_event_history"
            };
            let _ = error;
            push_blocker(&mut blockers, code);
            Decision {
                disposition: if insufficient && !invalid_window {
                    Disposition::Indeterminate
                } else {
                    Disposition::Rejected
                },
                event_count: case.event_times_seconds.len(),
                beta_hat: None,
                likelihood_ratio_statistic: None,
                exact_conditional_p_value: None,
                blockers,
            }
        }
    }
}

pub fn evaluate(corpus: &Corpus) -> Result<Report, String> {
    if corpus.schema != CORPUS_SCHEMA
        || corpus.evidence_class != "synthetic"
        || corpus.claim_ceiling != CLAIM_CEILING
        || corpus.cases.is_empty()
    {
        return Err("unsupported or empty PLP trend corpus contract".to_owned());
    }

    let mut case_ids = HashSet::new();
    let mut summary = Summary {
        case_count: corpus.cases.len(),
        trend_signal_detected: 0,
        no_trend_signal_detected: 0,
        indeterminate: 0,
        model_not_supported: 0,
        rejected: 0,
        expected_result_mismatches: 0,
        fit_acceptances: 0,
        reliability_claims_granted: 0,
        authority_grants: 0,
    };
    let mut results = Vec::with_capacity(corpus.cases.len());
    for case in &corpus.cases {
        if case.case_id.trim().is_empty() || !case_ids.insert(case.case_id.as_str()) {
            return Err("PLP trend case IDs must be unique and non-empty".to_owned());
        }
        let decision = evaluate_case(case);
        match decision.disposition {
            Disposition::TrendSignalDetected => summary.trend_signal_detected += 1,
            Disposition::NoTrendSignalDetected => summary.no_trend_signal_detected += 1,
            Disposition::Indeterminate => summary.indeterminate += 1,
            Disposition::ModelNotSupported => summary.model_not_supported += 1,
            Disposition::Rejected => summary.rejected += 1,
        }

        let expected = &case.expected;
        let close = |actual: Option<f64>, expected: Option<f64>| match (actual, expected) {
            (Some(left), Some(right)) => (left - right).abs() <= 1.0e-10 * left.abs().max(1.0),
            (None, None) => true,
            _ => false,
        };
        let matched = decision.disposition == expected.disposition
            && decision.event_count == expected.event_count
            && close(decision.beta_hat, expected.beta_hat)
            && close(
                decision.likelihood_ratio_statistic,
                expected.likelihood_ratio_statistic,
            )
            && close(
                decision.exact_conditional_p_value,
                expected.exact_conditional_p_value,
            )
            && decision.blockers == expected.blockers;
        summary.expected_result_mismatches += usize::from(!matched);

        results.push(CaseResult {
            case_id: case.case_id.clone(),
            disposition: decision.disposition,
            event_count: decision.event_count,
            beta_hat: decision.beta_hat,
            likelihood_ratio_statistic: decision.likelihood_ratio_statistic,
            exact_conditional_p_value: decision.exact_conditional_p_value,
            trend_signal_detected: decision.disposition == Disposition::TrendSignalDetected,
            fit_acceptance_granted: false,
            reliability_claim_granted: false,
            authority_granted: false,
            blockers: decision.blockers,
            expected_result_matched: matched,
        });
    }

    Ok(Report {
        result: if summary.expected_result_mismatches == 0
            && summary.fit_acceptances == 0
            && summary.reliability_claims_granted == 0
            && summary.authority_grants == 0
        {
            "pass".to_owned()
        } else {
            "fail".to_owned()
        },
        evidence_class: "synthetic".to_owned(),
        claim_ceiling: CLAIM_CEILING.to_owned(),
        fit_acceptance_granted: false,
        reliability_claim_granted: false,
        authority_granted: false,
        summary,
        cases: results,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gamma_cdf_known_values() {
        let value = regularized_gamma_p(2.0, 2.0).expect("valid gamma arguments");
        let expected = 1.0 - 3.0 * (-2.0_f64).exp();
        assert!((value - expected).abs() < 1.0e-12);
    }

    #[test]
    fn beta_one_boundary_has_unit_p_value() {
        let event_times = [
            (10_000.0 * (-1.2_f64).exp()) as u64,
            (10_000.0 * (-0.8_f64).exp()) as u64,
        ];
        let (beta_hat, likelihood_ratio, p_value) =
            estimate_trend(&event_times, 10_000).expect("valid beta-one history");
        assert!((beta_hat - 1.0).abs() < 1.0e-3);
        assert!(likelihood_ratio < 1.0e-5);
        assert!(p_value > 0.99);
    }

    #[test]
    fn scale_invariance_of_trend_statistic() {
        let first =
            estimate_trend(&[1_000, 2_000, 4_000, 5_000], 10_000).expect("valid first history");
        let second =
            estimate_trend(&[10_000, 20_000, 40_000, 50_000], 100_000).expect("scaled history");
        assert!((first.0 - second.0).abs() < 1.0e-12);
        assert!((first.1 - second.1).abs() < 1.0e-12);
        assert!((first.2 - second.2).abs() < 1.0e-12);
    }

    #[test]
    fn conditional_p_value_is_bounded() {
        let p = exact_conditional_trend_p_value(5, 0.4).expect("valid ratio");
        assert!((0.0..=1.0).contains(&p));
    }

    #[test]
    fn lower_and_upper_roots_return_same_tail_probability() {
        let deviance = trend_deviance(0.5);
        let upper = upper_ratio_root(deviance).expect("upper root");
        let lower_p = exact_conditional_trend_p_value(5, 0.5).expect("lower-side ratio");
        let upper_p = exact_conditional_trend_p_value(5, upper).expect("upper-side ratio");
        assert!((lower_p - upper_p).abs() < 1.0e-10);
    }

    #[test]
    fn duplicate_and_out_of_window_events_are_rejected() {
        assert!(estimate_trend(&[100, 100, 200], 1_000).is_err());
        assert!(estimate_trend(&[100, 1_000], 1_000).is_err());
    }

    #[test]
    fn model_profile_mismatch_is_not_supported() {
        let case = TrendCase {
            case_id: "wrong".to_owned(),
            model_profile: "weibull_first_failure_life".to_owned(),
            admissibility_receipt_sha256: "a".repeat(64),
            admissibility_source_commit: "b".repeat(40),
            exposure_end_seconds: 1_000,
            event_times_seconds: vec![100, 200],
            significance_level: 0.05,
            expected: Expected {
                disposition: Disposition::ModelNotSupported,
                event_count: 2,
                beta_hat: None,
                likelihood_ratio_statistic: None,
                exact_conditional_p_value: None,
                blockers: vec!["selected_model_profile_is_not_power_law_minimal_repair".to_owned()],
            },
        };
        assert_eq!(
            evaluate_case(&case).disposition,
            Disposition::ModelNotSupported
        );
    }
}
