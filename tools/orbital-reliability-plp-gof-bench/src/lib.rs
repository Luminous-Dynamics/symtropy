use serde::{Deserialize, Serialize};
use std::collections::HashSet;

const CORPUS_SCHEMA: &str = "luminous.orbital-plp-conditional-gof.corpus.v0.1";
const CLAIM_CEILING: &str = "synthetic-conditional-plp-goodness-of-fit-only";
const MODEL_PROFILE: &str = "power_law_process_repairable_minimal_repair";
const TERMINATION_PROFILE: &str = "administrative_time_terminated_complete_window";
const PRNG_ID: &str = "splitmix64-open-uniform-exp-normalize-v1";
const STATISTIC_ID: &str = "fitted-plp-uniform-order-statistics-cramer-von-mises-v1";
const MIN_EVENTS: usize = 5;
const MAX_EVENTS: usize = 10_000;
const MIN_REPLICATES: usize = 999;
const MAX_REPLICATES: usize = 100_000;
const MAX_EXPOSURE_SECONDS: u64 = 9_007_199_254_740_991;
const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;
const WILSON_Z_95: f64 = 1.959_963_984_540_054;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Disposition {
    FitRejected,
    FitNotRejectedAtDeclaredLevel,
    Indeterminate,
    ModelNotSupported,
    Rejected,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GofCase {
    pub case_id: String,
    pub model_profile: String,
    pub termination_profile: String,
    pub subject_group_id: String,
    pub configuration_sha256: String,
    pub environment_id: String,
    pub exposure_clock_id: String,
    pub observation_start_seconds: u64,
    pub exposure_end_seconds: u64,
    pub event_times_seconds: Vec<u64>,
    pub complete_event_history: bool,
    pub repair_policy: String,
    pub admissibility_receipt_sha256: String,
    pub admissibility_source_commit: String,
    pub capture_receipt_sha256: String,
    pub capture_source_commit: String,
    pub analysis_protocol_id: String,
    pub multiplicity_policy_id: String,
    pub significance_level: f64,
    pub monte_carlo_replicates: usize,
    pub random_seed: u64,
    pub expected: Expected,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Expected {
    pub disposition: Disposition,
    pub event_count: usize,
    pub beta_hat: Option<f64>,
    pub observed_cvm: Option<f64>,
    pub tail_exceedances: Option<usize>,
    pub monte_carlo_p_value: Option<f64>,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Corpus {
    pub schema: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub cases: Vec<GofCase>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MonteCarloResult {
    pub prng_id: String,
    pub statistic_id: String,
    pub seed: u64,
    pub replicate_count: usize,
    pub tail_exceedances: usize,
    pub monte_carlo_minimum_p_value: f64,
    pub monte_carlo_p_value: f64,
    pub tail_probability_estimate: f64,
    pub monte_carlo_standard_error: f64,
    pub wilson_95_lower: f64,
    pub wilson_95_upper: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct CaseResult {
    pub case_id: String,
    pub disposition: Disposition,
    pub event_count: usize,
    pub beta_hat: Option<f64>,
    pub observed_cvm: Option<f64>,
    pub monte_carlo: Option<MonteCarloResult>,
    pub fit_acceptance_granted: bool,
    pub reliability_claim_granted: bool,
    pub physical_qualification_granted: bool,
    pub authority_granted: bool,
    pub expected_result_matched: bool,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Summary {
    pub case_count: usize,
    pub fit_rejected: usize,
    pub fit_not_rejected: usize,
    pub indeterminate: usize,
    pub model_not_supported: usize,
    pub rejected: usize,
    pub expected_result_mismatches: usize,
    pub fit_acceptances: usize,
    pub reliability_claims: usize,
    pub physical_qualifications: usize,
    pub authority_grants: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct Report {
    pub result: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub fit_acceptance_granted: bool,
    pub reliability_claim_granted: bool,
    pub physical_qualification_granted: bool,
    pub authority_granted: bool,
    pub summary: Summary,
    pub cases: Vec<CaseResult>,
}

#[derive(Debug, Clone)]
struct Decision {
    disposition: Disposition,
    event_count: usize,
    beta_hat: Option<f64>,
    observed_cvm: Option<f64>,
    monte_carlo: Option<MonteCarloResult>,
    blockers: Vec<String>,
}

struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        value ^ (value >> 31)
    }

    fn uniform_open(&mut self) -> f64 {
        let mantissa = (self.next_u64() >> 11) as f64;
        ((mantissa + 0.5) / 9_007_199_254_740_992.0)
            .clamp(f64::EPSILON, 1.0 - f64::EPSILON)
    }

    fn exponential_unit(&mut self) -> f64 {
        -self.uniform_open().ln()
    }
}

fn blocker(code: &str) -> Vec<String> {
    vec![code.to_owned()]
}

pub fn estimate_beta_and_cvm(
    event_times_seconds: &[u64],
    exposure_end_seconds: u64,
) -> Result<(f64, f64), String> {
    if exposure_end_seconds == 0 || exposure_end_seconds > MAX_EXPOSURE_SECONDS {
        return Err("exposure endpoint is outside the supported v0.1 range".to_owned());
    }
    if !(MIN_EVENTS..=MAX_EVENTS).contains(&event_times_seconds.len()) {
        return Err("event count is outside the supported v0.1 range".to_owned());
    }

    let exposure = exposure_end_seconds as f64;
    let mut previous = 0u64;
    let mut sum_log_ratio = 0.0;
    for time in event_times_seconds {
        if *time == 0 || *time >= exposure_end_seconds || *time <= previous {
            return Err(
                "event times must be strictly increasing inside the fixed observation window"
                    .to_owned(),
            );
        }
        sum_log_ratio += (exposure / *time as f64).ln();
        previous = *time;
    }
    if !sum_log_ratio.is_finite() || sum_log_ratio <= 0.0 {
        return Err("event-time log exposure sum is invalid".to_owned());
    }

    let count = event_times_seconds.len();
    let beta_hat = count as f64 / sum_log_ratio;
    if !beta_hat.is_finite() || beta_hat <= 0.0 {
        return Err("estimated PLP shape is not finite and positive".to_owned());
    }

    let mut fitted_uniform = Vec::with_capacity(count);
    for time in event_times_seconds {
        fitted_uniform.push((*time as f64 / exposure).powf(beta_hat));
    }
    let statistic = cramer_von_mises(&fitted_uniform)?;
    Ok((beta_hat, statistic))
}

/// One-sample Cramer-von Mises statistic for already sorted fitted transforms.
pub fn cramer_von_mises(sorted_values: &[f64]) -> Result<f64, String> {
    let count = sorted_values.len();
    if count == 0 {
        return Err("Cramer-von Mises input must be non-empty".to_owned());
    }
    let n = count as f64;
    let mut statistic = 1.0 / (12.0 * n);
    let mut previous = 0.0;
    for (index, value) in sorted_values.iter().enumerate() {
        if !value.is_finite()
            || !(0.0..=1.0).contains(value)
            || *value < previous
        {
            return Err("fitted transforms must be finite, ordered and in [0,1]".to_owned());
        }
        let expected = (2 * index + 1) as f64 / (2.0 * n);
        statistic += (*value - expected).powi(2);
        previous = *value;
    }
    if !statistic.is_finite() || statistic < 0.0 {
        return Err("Cramer-von Mises statistic is invalid".to_owned());
    }
    Ok(statistic)
}

/// Generate one fitted-transform reference sample under the exact conditional
/// PLP null. Normalized iid Exp(1) values have Dirichlet(1,...,1) proportions.
fn conditional_reference_cvm(count: usize, rng: &mut SplitMix64) -> Result<f64, String> {
    let mut proportions = Vec::with_capacity(count);
    let mut total = 0.0;
    for _ in 0..count {
        let value = rng.exponential_unit();
        total += value;
        proportions.push(value);
    }
    if !total.is_finite() || total <= 0.0 {
        return Err("conditional reference normalization failed".to_owned());
    }

    proportions.sort_by(|left, right| right.total_cmp(left));
    let fitted: Vec<f64> = proportions
        .iter()
        .map(|proportion| (-count as f64 * proportion / total).exp())
        .collect();
    cramer_von_mises(&fitted)
}

fn wilson_interval(successes: usize, trials: usize) -> Result<(f64, f64, f64, f64), String> {
    if trials == 0 || successes > trials {
        return Err("invalid Monte Carlo exceedance count".to_owned());
    }
    let n = trials as f64;
    let estimate = successes as f64 / n;
    let standard_error = (estimate * (1.0 - estimate) / n).sqrt();
    let z2 = WILSON_Z_95 * WILSON_Z_95;
    let denominator = 1.0 + z2 / n;
    let center = (estimate + z2 / (2.0 * n)) / denominator;
    let half_width = WILSON_Z_95
        * (estimate * (1.0 - estimate) / n + z2 / (4.0 * n * n)).sqrt()
        / denominator;
    Ok((
        estimate,
        standard_error,
        (center - half_width).max(0.0),
        (center + half_width).min(1.0),
    ))
}

pub fn conditional_gof(
    event_times_seconds: &[u64],
    exposure_end_seconds: u64,
    replicates: usize,
    seed: u64,
) -> Result<(f64, f64, MonteCarloResult), String> {
    if !(MIN_REPLICATES..=MAX_REPLICATES).contains(&replicates) {
        return Err("Monte Carlo replicate count is outside the supported v0.1 range".to_owned());
    }
    if seed > MAX_SAFE_INTEGER {
        return Err("random seed exceeds the interoperable safe-integer range".to_owned());
    }

    let (beta_hat, observed_cvm) =
        estimate_beta_and_cvm(event_times_seconds, exposure_end_seconds)?;
    let count = event_times_seconds.len();
    let mut rng = SplitMix64::new(seed);
    let mut tail_exceedances = 0usize;

    for _ in 0..replicates {
        let simulated = conditional_reference_cvm(count, &mut rng)?;
        if simulated >= observed_cvm {
            tail_exceedances += 1;
        }
    }

    let monte_carlo_p_value =
        (tail_exceedances as f64 + 1.0) / (replicates as f64 + 1.0);
    let (tail_probability_estimate, monte_carlo_standard_error, wilson_95_lower, wilson_95_upper) =
        wilson_interval(tail_exceedances, replicates)?;
    let monte_carlo = MonteCarloResult {
        prng_id: PRNG_ID.to_owned(),
        statistic_id: STATISTIC_ID.to_owned(),
        seed,
        replicate_count: replicates,
        tail_exceedances,
        monte_carlo_minimum_p_value: 1.0 / (replicates as f64 + 1.0),
        monte_carlo_p_value,
        tail_probability_estimate,
        monte_carlo_standard_error,
        wilson_95_lower,
        wilson_95_upper,
    };
    Ok((beta_hat, observed_cvm, monte_carlo))
}

fn evaluate_case(case: &GofCase) -> Decision {
    if case.model_profile != MODEL_PROFILE {
        return Decision {
            disposition: Disposition::ModelNotSupported,
            event_count: case.event_times_seconds.len(),
            beta_hat: None,
            observed_cvm: None,
            monte_carlo: None,
            blockers: blocker("selected_model_profile_is_not_power_law_minimal_repair"),
        };
    }
    if case.termination_profile != TERMINATION_PROFILE || case.observation_start_seconds != 0 {
        return Decision {
            disposition: Disposition::ModelNotSupported,
            event_count: case.event_times_seconds.len(),
            beta_hat: None,
            observed_cvm: None,
            monte_carlo: None,
            blockers: blocker("only_complete_fixed_time_terminated_windows_from_zero_are_supported"),
        };
    }
    if !case.complete_event_history {
        return Decision {
            disposition: Disposition::Indeterminate,
            event_count: case.event_times_seconds.len(),
            beta_hat: None,
            observed_cvm: None,
            monte_carlo: None,
            blockers: blocker("complete_event_history_not_established"),
        };
    }
    if case.repair_policy != "minimal_repair" {
        return Decision {
            disposition: Disposition::ModelNotSupported,
            event_count: case.event_times_seconds.len(),
            beta_hat: None,
            observed_cvm: None,
            monte_carlo: None,
            blockers: blocker("repair_policy_is_not_minimal_repair"),
        };
    }
    if case.subject_group_id.trim().is_empty()
        || !valid_hex(&case.configuration_sha256, 64)
        || case.environment_id.trim().is_empty()
        || case.exposure_clock_id.trim().is_empty()
        || case.analysis_protocol_id.trim().is_empty()
        || case.multiplicity_policy_id.trim().is_empty()
    {
        return Decision {
            disposition: Disposition::Rejected,
            event_count: case.event_times_seconds.len(),
            beta_hat: None,
            observed_cvm: None,
            monte_carlo: None,
            blockers: blocker("subject_configuration_clock_or_protocol_binding_invalid"),
        };
    }
    if !valid_hex(&case.admissibility_receipt_sha256, 64)
        || !valid_hex(&case.admissibility_source_commit, 40)
        || !valid_hex(&case.capture_receipt_sha256, 64)
        || !valid_hex(&case.capture_source_commit, 40)
    {
        return Decision {
            disposition: Disposition::Indeterminate,
            event_count: case.event_times_seconds.len(),
            beta_hat: None,
            observed_cvm: None,
            monte_carlo: None,
            blockers: blocker("admissibility_or_capture_receipt_pin_missing_or_malformed"),
        };
    }
    if !(case.significance_level.is_finite()
        && 0.0 < case.significance_level
        && case.significance_level < 1.0)
    {
        return Decision {
            disposition: Disposition::Rejected,
            event_count: case.event_times_seconds.len(),
            beta_hat: None,
            observed_cvm: None,
            monte_carlo: None,
            blockers: blocker("invalid_significance_level"),
        };
    }
    if case.monte_carlo_replicates < MIN_REPLICATES
        || case.monte_carlo_replicates > MAX_REPLICATES
        || case.monte_carlo_replicates < (1.0 / case.significance_level).ceil() as usize - 1
        || case.random_seed > MAX_SAFE_INTEGER
    {
        return Decision {
            disposition: Disposition::Rejected,
            event_count: case.event_times_seconds.len(),
            beta_hat: None,
            observed_cvm: None,
            monte_carlo: None,
            blockers: blocker("monte_carlo_resolution_or_seed_outside_predeclared_contract"),
        };
    }
    if case.event_times_seconds.len() < MIN_EVENTS {
        return Decision {
            disposition: Disposition::Indeterminate,
            event_count: case.event_times_seconds.len(),
            beta_hat: None,
            observed_cvm: None,
            monte_carlo: None,
            blockers: blocker("insufficient_event_count_for_v0_1_diagnostic"),
        };
    }

    match conditional_gof(
        &case.event_times_seconds,
        case.exposure_end_seconds,
        case.monte_carlo_replicates,
        case.random_seed,
    ) {
        Ok((beta_hat, observed_cvm, monte_carlo)) => Decision {
            disposition: if monte_carlo.monte_carlo_p_value <= case.significance_level {
                Disposition::FitRejected
            } else {
                Disposition::FitNotRejectedAtDeclaredLevel
            },
            event_count: case.event_times_seconds.len(),
            beta_hat: Some(beta_hat),
            observed_cvm: Some(observed_cvm),
            monte_carlo: Some(monte_carlo),
            blockers: Vec::new(),
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
            let code = if invalid_window {
                "event_history_or_exposure_window_invalid"
            } else {
                "insufficient_or_numerically_invalid_event_history"
            };
            let _ = error;
            Decision {
                disposition: Disposition::Rejected,
                event_count: case.event_times_seconds.len(),
                beta_hat: None,
                observed_cvm: None,
                monte_carlo: None,
                blockers: blocker(code),
            }
        }
    }
}

fn valid_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

pub fn evaluate(corpus: &Corpus) -> Result<Report, String> {
    if corpus.schema != CORPUS_SCHEMA
        || corpus.evidence_class != "synthetic"
        || corpus.claim_ceiling != CLAIM_CEILING
        || corpus.cases.is_empty()
    {
        return Err("unsupported or empty conditional PLP goodness-of-fit corpus".to_owned());
    }

    let mut ids = HashSet::with_capacity(corpus.cases.len());
    let mut summary = Summary {
        case_count: corpus.cases.len(),
        fit_rejected: 0,
        fit_not_rejected: 0,
        indeterminate: 0,
        model_not_supported: 0,
        rejected: 0,
        expected_result_mismatches: 0,
        fit_acceptances: 0,
        reliability_claims: 0,
        physical_qualifications: 0,
        authority_grants: 0,
    };
    let mut results = Vec::with_capacity(corpus.cases.len());

    for case in &corpus.cases {
        if case.case_id.trim().is_empty() || !ids.insert(case.case_id.as_str()) {
            return Err("PLP GOF case IDs must be unique and non-empty".to_owned());
        }
        let decision = evaluate_case(case);
        match decision.disposition {
            Disposition::FitRejected => summary.fit_rejected += 1,
            Disposition::FitNotRejectedAtDeclaredLevel => summary.fit_not_rejected += 1,
            Disposition::Indeterminate => summary.indeterminate += 1,
            Disposition::ModelNotSupported => summary.model_not_supported += 1,
            Disposition::Rejected => summary.rejected += 1,
        }

        let expected = &case.expected;
        let close = |actual: Option<f64>, expected: Option<f64>| match (actual, expected) {
            (Some(left), Some(right)) => {
                (left - right).abs() <= 1.0e-10 * left.abs().max(1.0)
            }
            (None, None) => true,
            _ => false,
        };
        let matched = decision.disposition == expected.disposition
            && decision.event_count == expected.event_count
            && close(decision.beta_hat, expected.beta_hat)
            && close(decision.observed_cvm, expected.observed_cvm)
            && decision
                .monte_carlo
                .as_ref()
                .map(|m| m.tail_exceedances)
                == expected.tail_exceedances
            && decision
                .monte_carlo
                .as_ref()
                .map(|m| m.monte_carlo_p_value)
                .zip(expected.monte_carlo_p_value)
                .map(|(actual, expected)| close(Some(actual), Some(expected)))
                .unwrap_or(expected.monte_carlo_p_value.is_none() && decision.monte_carlo.is_none())
            && decision.blockers == expected.blockers;
        summary.expected_result_mismatches += usize::from(!matched);

        results.push(CaseResult {
            case_id: case.case_id.clone(),
            disposition: decision.disposition,
            event_count: decision.event_count,
            beta_hat: decision.beta_hat,
            observed_cvm: decision.observed_cvm,
            monte_carlo: decision.monte_carlo,
            fit_acceptance_granted: false,
            reliability_claim_granted: false,
            physical_qualification_granted: false,
            authority_granted: false,
            expected_result_matched: matched,
            blockers: decision.blockers,
        });
    }

    let result = if summary.expected_result_mismatches == 0
        && summary.fit_acceptances == 0
        && summary.reliability_claims == 0
        && summary.physical_qualifications == 0
        && summary.authority_grants == 0
    {
        "pass"
    } else {
        "fail"
    };

    Ok(Report {
        result: result.to_owned(),
        evidence_class: "synthetic".to_owned(),
        claim_ceiling: CLAIM_CEILING.to_owned(),
        fit_acceptance_granted: false,
        reliability_claim_granted: false,
        physical_qualification_granted: false,
        authority_granted: false,
        summary,
        cases: results,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cvm_known_value() {
        let statistic = cramer_von_mises(&[0.1, 0.5, 0.9]).expect("valid ordered values");
        assert!((statistic - 0.045_555_555_555_555_56).abs() < 1.0e-14);
    }

    #[test]
    fn conditional_reference_replays_for_fixed_seed() {
        let mut first = SplitMix64::new(42);
        let mut second = SplitMix64::new(42);
        let left = conditional_reference_cvm(12, &mut first).expect("valid first replay");
        let right = conditional_reference_cvm(12, &mut second).expect("valid second replay");
        assert_eq!(left, right);
    }

    #[test]
    fn conditional_cvm_is_invariant_to_exposure_unit_scaling() {
        let first = estimate_beta_and_cvm(&[1_000, 2_000, 3_000, 5_000, 8_000, 9_000], 10_000)
            .expect("first history");
        let second =
            estimate_beta_and_cvm(&[10_000, 20_000, 30_000, 50_000, 80_000, 90_000], 100_000)
                .expect("scaled history");
        assert!((first.0 - second.0).abs() < 1.0e-12);
        assert!((first.1 - second.1).abs() < 1.0e-12);
    }

    #[test]
    fn monte_carlo_p_value_has_plus_one_resolution() {
        let (_, _, result) =
            conditional_gof(&[10, 20, 30, 40, 50, 60], 100, 999, 123).expect("valid GOF");
        assert!(result.monte_carlo_p_value >= 1.0 / 1000.0);
        assert!(result.monte_carlo_p_value <= 1.0);
        assert!(result.wilson_95_lower <= result.tail_probability_estimate);
        assert!(result.tail_probability_estimate <= result.wilson_95_upper);
    }

    #[test]
    fn beta_not_equal_to_one_is_not_a_trend_test() {
        let times = [3_000, 7_000, 12_000, 22_000, 45_000, 78_000];
        let (beta_hat, _) = estimate_beta_and_cvm(&times, 100_000).expect("valid PLP history");
        assert!(beta_hat > 0.0);
        assert!((beta_hat - 1.0).abs() > 0.1);
    }
}
