use serde::{Deserialize, Serialize};

const MAX_TRIALS: usize = 100_000;
const MAX_SAFE_INTEGER: usize = 9_007_199_254_740_991;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PodTrendStatus {
    Bounded,
    Unbounded,
    ModelIncompatible,
    Indeterminate,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BinaryPodAnchor {
    pub normalized_time_fraction: f64,
    pub hits: usize,
    pub trials: usize,
    pub event_class: String,
    pub covariate_stratum_id: String,
    pub configuration_sha256: String,
    pub sampling_unit: String,
    pub independent_trials_asserted: bool,
    pub reference_evidence_sha256: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PodTrendCase {
    pub case_id: String,
    pub early_anchor: BinaryPodAnchor,
    pub late_anchor: BinaryPodAnchor,
    pub familywise_alpha: f64,
    pub expected_status: PodTrendStatus,
    pub expected_gamma_min: f64,
    pub expected_gamma_max: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PodProbabilityInterval {
    pub point_estimate: f64,
    pub lower: f64,
    pub upper: f64,
    pub confidence_level: f64,
    pub successes: usize,
    pub trials: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct PodTrendResult {
    pub case_id: String,
    pub status: PodTrendStatus,
    pub early_pod: Option<PodProbabilityInterval>,
    pub late_pod: Option<PodProbabilityInterval>,
    pub gamma_min: Option<f64>,
    pub gamma_max: Option<f64>,
    pub physical_beta_lower_open_if_used: bool,
    pub familywise_alpha: f64,
    pub anchor_alpha_allocation: f64,
    pub expected_result_matched: bool,
    pub blockers: Vec<String>,
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

fn log_binomial_cdf_at_most(k: usize, n: usize, probability: f64) -> f64 {
    if k >= n {
        return 0.0;
    }
    if probability <= 0.0 {
        return 0.0;
    }
    if probability >= 1.0 {
        return f64::NEG_INFINITY;
    }
    let log_p = probability.ln();
    let log_q = (-probability).ln_1p();
    let mut log_mass = n as f64 * log_q;
    let mut log_sum = log_mass;
    for j in 1..=k {
        log_mass += ((n - j + 1) as f64).ln() - (j as f64).ln() + log_p - log_q;
        log_sum = log_add_exp(log_sum, log_mass);
    }
    log_sum.min(0.0)
}

fn log_binomial_tail_at_least(k: usize, n: usize, probability: f64) -> f64 {
    if k == 0 {
        return 0.0;
    }
    if k > n {
        return f64::NEG_INFINITY;
    }
    if probability <= 0.0 {
        return f64::NEG_INFINITY;
    }
    if probability >= 1.0 {
        return 0.0;
    }
    let log_p = probability.ln();
    let log_q = (-probability).ln_1p();
    let mut log_mass = n as f64 * log_q;
    for j in 1..=k {
        log_mass += ((n - j + 1) as f64).ln() - (j as f64).ln() + log_p - log_q;
    }
    let mut log_sum = log_mass;
    for j in (k + 1)..=n {
        log_mass += ((n - j + 1) as f64).ln() - (j as f64).ln() + log_p - log_q;
        log_sum = log_add_exp(log_sum, log_mass);
    }
    log_sum.min(0.0)
}

/// Two-sided exact Clopper-Pearson interval with each tail allocated alpha.
/// The caller performs family-wise allocation across independent POD anchors.
pub fn clopper_pearson_interval(
    successes: usize,
    trials: usize,
    tail_alpha: f64,
) -> Result<(f64, f64), String> {
    if trials == 0
        || trials > MAX_TRIALS
        || successes > trials
        || !tail_alpha.is_finite()
        || tail_alpha <= 0.0
        || tail_alpha >= 0.5
    {
        return Err("POD binomial counts or tail alpha are outside the supported range".to_owned());
    }

    let log_alpha = tail_alpha.ln();

    let lower = if successes == 0 {
        0.0
    } else {
        let mut lo = 0.0;
        let mut hi = 1.0;
        for _ in 0..100 {
            let midpoint = lo + (hi - lo) / 2.0;
            if log_binomial_tail_at_least(successes, trials, midpoint) < log_alpha {
                lo = midpoint;
            } else {
                hi = midpoint;
            }
        }
        lo + (hi - lo) / 2.0
    };

    let upper = if successes == trials {
        1.0
    } else {
        let mut lo = 0.0;
        let mut hi = 1.0;
        for _ in 0..100 {
            let midpoint = lo + (hi - lo) / 2.0;
            if log_binomial_cdf_at_most(successes, trials, midpoint) > log_alpha {
                lo = midpoint;
            } else {
                hi = midpoint;
            }
        }
        lo + (hi - lo) / 2.0
    };

    Ok((lower, upper))
}

fn validate_anchor(anchor: &BinaryPodAnchor) -> Result<(), String> {
    if !anchor.normalized_time_fraction.is_finite()
        || anchor.normalized_time_fraction <= 0.0
        || anchor.normalized_time_fraction > 1.0
        || anchor.trials == 0
        || anchor.trials > MAX_TRIALS
        || anchor.hits > anchor.trials
        || anchor.hits > MAX_SAFE_INTEGER
        || anchor.event_class.trim().is_empty()
        || anchor.covariate_stratum_id.trim().is_empty()
        || anchor.sampling_unit.trim().is_empty()
        || anchor.configuration_sha256.len() != 64
        || !anchor.configuration_sha256.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        || anchor.reference_evidence_sha256.len() != 64
        || !anchor.reference_evidence_sha256.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    {
        return Err("POD anchor scope, count or evidence digest is invalid".to_owned());
    }
    Ok(())
}

fn indeterminate(case: &PodTrendCase, code: &str, allocation: f64) -> PodTrendResult {
    PodTrendResult {
        case_id: case.case_id.clone(),
        status: PodTrendStatus::Indeterminate,
        early_pod: None,
        late_pod: None,
        gamma_min: None,
        gamma_max: None,
        physical_beta_lower_open_if_used: true,
        familywise_alpha: case.familywise_alpha,
        anchor_alpha_allocation: allocation,
        expected_result_matched: case.expected_status == PodTrendStatus::Indeterminate,
        blockers: vec![code.to_owned()],
    }
}

/// Bounds gamma under the explicit model p(x)=p0*x^gamma between two
/// independently measured POD anchors with exactly matched non-time strata.
/// The two anchor intervals receive a Bonferroni family-wise alpha allocation.
pub fn analyze_pod_trend(case: &PodTrendCase) -> Result<PodTrendResult, String> {
    if case.case_id.trim().is_empty()
        || !case.familywise_alpha.is_finite()
        || case.familywise_alpha <= 0.0
        || case.familywise_alpha >= 0.2
    {
        return Err("POD trend case or family-wise alpha is invalid".to_owned());
    }
    let anchor_alpha_allocation = case.familywise_alpha / 2.0;
    let tail_alpha = anchor_alpha_allocation / 2.0;

    if validate_anchor(&case.early_anchor).is_err()
        || validate_anchor(&case.late_anchor).is_err()
    {
        return Ok(indeterminate(
            case,
            "POD anchor evidence, counts or configuration digest invalid",
            anchor_alpha_allocation,
        ));
    }
    if !case.early_anchor.independent_trials_asserted
        || !case.late_anchor.independent_trials_asserted
    {
        return Ok(indeterminate(
            case,
            "independence of POD sampling units not established",
            anchor_alpha_allocation,
        ));
    }
    if case.early_anchor.event_class != case.late_anchor.event_class
        || case.early_anchor.covariate_stratum_id != case.late_anchor.covariate_stratum_id
        || case.early_anchor.configuration_sha256 != case.late_anchor.configuration_sha256
        || case.early_anchor.sampling_unit != case.late_anchor.sampling_unit
    {
        return Ok(indeterminate(
            case,
            "POD anchors do not share the exact non-time event/configuration/sampling stratum",
            anchor_alpha_allocation,
        ));
    }
    if case.late_anchor.normalized_time_fraction
        <= case.early_anchor.normalized_time_fraction
    {
        return Ok(indeterminate(
            case,
            "POD anchor times are not strictly increasing",
            anchor_alpha_allocation,
        ));
    }

    let (early_lower, early_upper) = clopper_pearson_interval(
        case.early_anchor.hits,
        case.early_anchor.trials,
        tail_alpha,
    )?;
    let (late_lower, late_upper) = clopper_pearson_interval(
        case.late_anchor.hits,
        case.late_anchor.trials,
        tail_alpha,
    )?;
    let confidence_level = 1.0 - anchor_alpha_allocation;
    let early_pod = PodProbabilityInterval {
        point_estimate: case.early_anchor.hits as f64 / case.early_anchor.trials as f64,
        lower: early_lower,
        upper: early_upper,
        confidence_level,
        successes: case.early_anchor.hits,
        trials: case.early_anchor.trials,
    };
    let late_pod = PodProbabilityInterval {
        point_estimate: case.late_anchor.hits as f64 / case.late_anchor.trials as f64,
        lower: late_lower,
        upper: late_upper,
        confidence_level,
        successes: case.late_anchor.hits,
        trials: case.late_anchor.trials,
    };

    let log_time_ratio = (case.late_anchor.normalized_time_fraction
        / case.early_anchor.normalized_time_fraction)
        .ln();
    let gamma_min = if late_lower > 0.0 {
        ((late_lower / early_upper).ln() / log_time_ratio).max(0.0)
    } else {
        0.0
    };

    // p0 must remain <= 1. For each anchor with a positive lower POD
    // confidence limit and normalized time x < 1, feasibility requires
    // lower_POD <= x^gamma, giving gamma <= ln(lower_POD)/ln(x).
    let p0_gamma_caps = [
        (early_lower, case.early_anchor.normalized_time_fraction),
        (late_lower, case.late_anchor.normalized_time_fraction),
    ]
    .into_iter()
    .filter_map(|(lower, time)| {
        (lower > 0.0 && time < 1.0).then_some(lower.ln() / time.ln())
    })
    .collect::<Vec<_>>();
    let p0_gamma_cap = p0_gamma_caps.into_iter().reduce(f64::min);

    let ratio_gamma_max = if early_lower > 0.0 && late_upper > 0.0 {
        Some((late_upper / early_lower).ln() / log_time_ratio)
    } else {
        None
    };
    let gamma_max = match (ratio_gamma_max, p0_gamma_cap) {
        (Some(ratio), Some(cap)) => Some(ratio.min(cap)),
        (Some(ratio), None) => Some(ratio),
        (None, Some(cap)) => Some(cap),
        (None, None) => None,
    };

    if gamma_max.is_some_and(|upper| upper < gamma_min || upper < 0.0) {
        return Ok(PodTrendResult {
            case_id: case.case_id.clone(),
            status: PodTrendStatus::ModelIncompatible,
            early_pod: Some(early_pod),
            late_pod: Some(late_pod),
            gamma_min: Some(gamma_min),
            gamma_max,
            physical_beta_lower_open_if_used: true,
            familywise_alpha: case.familywise_alpha,
            anchor_alpha_allocation,
            expected_result_matched: case.expected_status == PodTrendStatus::ModelIncompatible,
            blockers: vec![
                "POD confidence regions are incompatible with p0<=1 and a nondecreasing power-law detection model".to_owned(),
            ],
        });
    }

    match gamma_max {
        Some(upper) => {
            let expected_match = case.expected_status == PodTrendStatus::Bounded
                && (gamma_min - case.expected_gamma_min).abs() <= 1.0e-8
                && case.expected_gamma_max.is_some_and(|expected| {
                    (upper - expected).abs() <= 1.0e-8
                });
            Ok(PodTrendResult {
                case_id: case.case_id.clone(),
                status: PodTrendStatus::Bounded,
                early_pod: Some(early_pod),
                late_pod: Some(late_pod),
                gamma_min: Some(gamma_min),
                gamma_max: Some(upper),
                physical_beta_lower_open_if_used: false,
                familywise_alpha: case.familywise_alpha,
                anchor_alpha_allocation,
                expected_result_matched: expected_match,
                blockers: vec![
                    "gamma bounds are conditional on the two-anchor power-law POD model and p0<=1".to_owned(),
                    "POD bounds apply only to the exact matched event/configuration stratum".to_owned(),
                ],
            })
        }
        None => {
            let expected_match = case.expected_status == PodTrendStatus::Unbounded
                && (gamma_min - case.expected_gamma_min).abs() <= 1.0e-8
                && case.expected_gamma_max.is_none();
            Ok(PodTrendResult {
                case_id: case.case_id.clone(),
                status: PodTrendStatus::Unbounded,
                early_pod: Some(early_pod),
                late_pod: Some(late_pod),
                gamma_min: Some(gamma_min),
                gamma_max: None,
                physical_beta_lower_open_if_used: true,
                familywise_alpha: case.familywise_alpha,
                anchor_alpha_allocation,
                expected_result_matched: expected_match,
                blockers: vec![
                    "POD confidence evidence does not impose a finite gamma upper bound".to_owned(),
                    "physical beta lower endpoint remains open without a finite gamma upper bound".to_owned(),
                ],
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn anchor(time: f64, hits: usize, trials: usize) -> BinaryPodAnchor {
        BinaryPodAnchor {
            normalized_time_fraction: time,
            hits,
            trials,
            event_class: "impact-pulse".to_owned(),
            covariate_stratum_id: "amplitude-medium-width-short-rate-isolated-ambient".to_owned(),
            configuration_sha256: "a".repeat(64),
            sampling_unit: "pulse".to_owned(),
            independent_trials_asserted: true,
            reference_evidence_sha256: "b".repeat(64),
        }
    }

    #[test]
    fn clopper_pearson_zero_and_all_hit_edges_are_valid() {
        let (_, upper_zero) = clopper_pearson_interval(0, 1000, 0.0125).unwrap();
        let (lower_all, _) = clopper_pearson_interval(1000, 1000, 0.0125).unwrap();
        assert!(upper_zero > 0.0 && upper_zero < 0.02);
        assert!(lower_all < 1.0 && lower_all > 0.98);
    }

    #[test]
    fn pod_power_trend_is_bounded_with_matched_independent_anchors() {
        let case = PodTrendCase {
            case_id: "bounded".to_owned(),
            early_anchor: anchor(0.25, 400, 1000),
            late_anchor: anchor(1.0, 800, 1000),
            familywise_alpha: 0.05,
            expected_status: PodTrendStatus::Bounded,
            expected_gamma_min: 0.0,
            expected_gamma_max: Some(1.0),
        };
        let result = analyze_pod_trend(&case).unwrap();
        assert_eq!(result.status, PodTrendStatus::Bounded);
        assert!(result.gamma_min.unwrap() >= 0.0);
        assert!(result.gamma_max.unwrap() > result.gamma_min.unwrap());
    }

    #[test]
    fn unverified_independence_never_produces_pod_trend_bound() {
        let mut early = anchor(0.25, 400, 1000);
        early.independent_trials_asserted = false;
        let case = PodTrendCase {
            case_id: "dependent".to_owned(),
            early_anchor: early,
            late_anchor: anchor(1.0, 800, 1000),
            familywise_alpha: 0.05,
            expected_status: PodTrendStatus::Indeterminate,
            expected_gamma_min: 0.0,
            expected_gamma_max: None,
        };
        assert_eq!(analyze_pod_trend(&case).unwrap().status, PodTrendStatus::Indeterminate);
    }

    #[test]
    fn unmatched_event_covariates_do_not_produce_gamma_bound() {
        let mut late = anchor(1.0, 800, 1000);
        late.covariate_stratum_id = "amplitude-high-width-short-rate-isolated-ambient".to_owned();
        let case = PodTrendCase {
            case_id: "unmatched".to_owned(),
            early_anchor: anchor(0.25, 400, 1000),
            late_anchor: late,
            familywise_alpha: 0.05,
            expected_status: PodTrendStatus::Indeterminate,
            expected_gamma_min: 0.0,
            expected_gamma_max: None,
        };
        assert_eq!(analyze_pod_trend(&case).unwrap().status, PodTrendStatus::Indeterminate);
    }
}
