//! Synthetic known-truth causal benchmark for manufacturing interventions.
//!
//! This file deliberately separates:
//! - randomized intervention-effect estimation,
//! - before/after trend confounding,
//! - observational hidden-confounder and placebo diagnostics.
//!
//! No manufacturing data, control-system connection, or production claim is involved.
//!
//! Build and test:
//!   rustc --edition=2021 -D warnings --test tools/manufacturing-assurance/causal.rs -o /tmp/mfg-causal-tests
//!   /tmp/mfg-causal-tests
//!
//! Build and run:
//!   rustc --edition=2021 -D warnings tools/manufacturing-assurance/causal.rs -o /tmp/mfg-causal
//!   /tmp/mfg-causal

#[derive(Clone, Copy, Debug)]
struct PotentialUnit {
    delta_without_treatment: f64,
    treatment_effect: f64,
}

#[derive(Clone, Copy, Debug)]
struct MatchedPair {
    left: PotentialUnit,
    right: PotentialUnit,
}

#[derive(Clone, Copy, Debug)]
struct ObservationalUnit {
    treated: bool,
    hidden_confounder: bool,
    pre_period_change: f64,
    post_period_change_without_treatment: f64,
    treatment_effect: f64,
}

#[derive(Clone, Copy, Debug)]
struct RandomizedResult {
    finite_population_ate: f64,
    observed_assignment_ate: f64,
    exact_randomization_expectation: f64,
    assignment_count: usize,
}

#[derive(Clone, Copy, Debug)]
struct ConfoundingResult {
    true_ate: f64,
    naive_change_difference: f64,
    placebo_pretrend_difference: f64,
    before_after_treated_change: f64,
}

fn mean(values: &[f64]) -> Result<f64, String> {
    if values.is_empty() {
        return Err("mean requires at least one value".to_string());
    }
    let total: f64 = values.iter().sum();
    if !total.is_finite() {
        return Err("mean input produced a non-finite total".to_string());
    }
    Ok(total / values.len() as f64)
}

fn finite_population_ate(pairs: &[MatchedPair]) -> Result<f64, String> {
    if pairs.is_empty() {
        return Err("ATE requires at least one matched pair".to_string());
    }
    let effects: Vec<f64> = pairs
        .iter()
        .flat_map(|pair| [pair.left.treatment_effect, pair.right.treatment_effect])
        .collect();
    mean(&effects)
}

/// Exact paired estimator for one assignment mask.
/// A set bit assigns treatment to the left unit; a clear bit assigns it to right.
fn paired_estimate(pairs: &[MatchedPair], assignment_mask: usize) -> Result<f64, String> {
    if pairs.is_empty() {
        return Err("paired estimate requires at least one pair".to_string());
    }
    if pairs.len() >= usize::BITS as usize {
        return Err("too many pairs for exact assignment enumeration".to_string());
    }
    if assignment_mask >= (1usize << pairs.len()) {
        return Err("assignment mask exceeds the design space".to_string());
    }

    let mut pair_effects = Vec::with_capacity(pairs.len());
    for (index, pair) in pairs.iter().enumerate() {
        let left_treated = ((assignment_mask >> index) & 1) == 1;
        let (treated, control) = if left_treated {
            (pair.left, pair.right)
        } else {
            (pair.right, pair.left)
        };
        let treated_change =
            treated.delta_without_treatment + treated.treatment_effect;
        let control_change = control.delta_without_treatment;
        pair_effects.push(treated_change - control_change);
    }
    mean(&pair_effects)
}

/// Enumerates every within-pair assignment. The mean over all assignments
/// equals the finite-population ATE for this matched-pair randomization design.
fn exact_randomization_expectation(
    pairs: &[MatchedPair],
) -> Result<(f64, usize), String> {
    if pairs.is_empty() {
        return Err("exact randomization requires at least one pair".to_string());
    }
    if pairs.len() >= usize::BITS as usize {
        return Err("too many pairs for exact assignment enumeration".to_string());
    }

    let count = 1usize << pairs.len();
    let mut estimates = Vec::with_capacity(count);
    for mask in 0..count {
        estimates.push(paired_estimate(pairs, mask)?);
    }
    Ok((mean(&estimates)?, count))
}

fn randomized_panel() -> Vec<MatchedPair> {
    // Each pair represents a pre-blocked pair of comparable machine/lot cells.
    // Baseline trends differ within pairs; randomizing treatment within pairs
    // makes the exact estimator's assignment expectation known.
    vec![
        MatchedPair {
            left: PotentialUnit { delta_without_treatment: -3.0, treatment_effect: -8.0 },
            right: PotentialUnit { delta_without_treatment: -1.0, treatment_effect: -6.0 },
        },
        MatchedPair {
            left: PotentialUnit { delta_without_treatment: 2.0, treatment_effect: -5.0 },
            right: PotentialUnit { delta_without_treatment: -2.0, treatment_effect: -7.0 },
        },
        MatchedPair {
            left: PotentialUnit { delta_without_treatment: 1.0, treatment_effect: -9.0 },
            right: PotentialUnit { delta_without_treatment: 4.0, treatment_effect: -4.0 },
        },
        MatchedPair {
            left: PotentialUnit { delta_without_treatment: -4.0, treatment_effect: -6.0 },
            right: PotentialUnit { delta_without_treatment: 3.0, treatment_effect: -8.0 },
        },
    ]
}

fn randomized_result() -> Result<RandomizedResult, String> {
    let pairs = randomized_panel();
    let truth = finite_population_ate(&pairs)?;
    let assignment_mask = 0b1010usize;
    let observed = paired_estimate(&pairs, assignment_mask)?;
    let (expectation, assignment_count) = exact_randomization_expectation(&pairs)?;
    Ok(RandomizedResult {
        finite_population_ate: truth,
        observed_assignment_ate: observed,
        exact_randomization_expectation: expectation,
        assignment_count,
    })
}

fn time_trend_zero_effect_panel() -> Vec<ObservationalUnit> {
    // Intervention effect is exactly zero. All lines improve by -6 over time
    // due to a concurrent maintenance/replenishment trend. Treatment is
    // assigned to half the lines, but before/after alone attributes the trend
    // to the intervention.
    vec![
        ObservationalUnit {
            treated: true, hidden_confounder: false, pre_period_change: -2.0,
            post_period_change_without_treatment: -6.0, treatment_effect: 0.0,
        },
        ObservationalUnit {
            treated: true, hidden_confounder: true, pre_period_change: -2.0,
            post_period_change_without_treatment: -6.0, treatment_effect: 0.0,
        },
        ObservationalUnit {
            treated: false, hidden_confounder: false, pre_period_change: -2.0,
            post_period_change_without_treatment: -6.0, treatment_effect: 0.0,
        },
        ObservationalUnit {
            treated: false, hidden_confounder: true, pre_period_change: -2.0,
            post_period_change_without_treatment: -6.0, treatment_effect: 0.0,
        },
    ]
}

fn hidden_confounding_panel() -> Vec<ObservationalUnit> {
    // Treatment is preferentially assigned to high-hidden-risk lines. Hidden
    // risk produces a pre-existing downward drift, while the true treatment
    // effect remains zero. A placebo pre-period contrast reveals the bias.
    vec![
        ObservationalUnit {
            treated: true, hidden_confounder: true, pre_period_change: -4.0,
            post_period_change_without_treatment: -4.0, treatment_effect: 0.0,
        },
        ObservationalUnit {
            treated: true, hidden_confounder: true, pre_period_change: -4.0,
            post_period_change_without_treatment: -4.0, treatment_effect: 0.0,
        },
        ObservationalUnit {
            treated: false, hidden_confounder: false, pre_period_change: 0.0,
            post_period_change_without_treatment: 0.0, treatment_effect: 0.0,
        },
        ObservationalUnit {
            treated: false, hidden_confounder: false, pre_period_change: 0.0,
            post_period_change_without_treatment: 0.0, treatment_effect: 0.0,
        },
    ]
}

fn mean_group<F>(rows: &[ObservationalUnit], treated: bool, value: F) -> Result<f64, String>
where
    F: Fn(&ObservationalUnit) -> f64,
{
    let values: Vec<f64> = rows
        .iter()
        .filter(|row| row.treated == treated)
        .map(value)
        .collect();
    mean(&values)
}

fn confounding_result(rows: &[ObservationalUnit]) -> Result<ConfoundingResult, String> {
    let true_ate = mean(&rows.iter().map(|row| row.treatment_effect).collect::<Vec<_>>())?;
    let treated_post = mean_group(rows, true, |row| {
        row.post_period_change_without_treatment + row.treatment_effect
    })?;
    let control_post = mean_group(rows, false, |row| row.post_period_change_without_treatment)?;
    let naive_change_difference = treated_post - control_post;

    let treated_placebo = mean_group(rows, true, |row| row.pre_period_change)?;
    let control_placebo = mean_group(rows, false, |row| row.pre_period_change)?;
    let placebo_pretrend_difference = treated_placebo - control_placebo;

    let before_after_treated_change = treated_post;
    Ok(ConfoundingResult {
        true_ate,
        naive_change_difference,
        placebo_pretrend_difference,
        before_after_treated_change,
    })
}

fn run_report() -> Result<String, String> {
    let randomized = randomized_result()?;
    let trend = confounding_result(&time_trend_zero_effect_panel())?;
    let hidden = confounding_result(&hidden_confounding_panel())?;

    Ok(format!(
        concat!(
            "MFG_CAUSAL_SYNTHETIC_KNOWN_TRUTH version=1\n",
            "RANDOMIZED_PAIR_DESIGN units=8 pairs=4 assignments={} finite_population_ate={:.6} observed_assignment_ate={:.6} exact_randomization_expectation={:.6}\n",
            "ZERO_EFFECT_TIME_TREND true_ate={:.6} naive_before_after_treated_change={:.6} naive_treated_minus_control_change={:.6}\n",
            "HIDDEN_CONFOUNDING true_ate={:.6} naive_treated_minus_control_change={:.6} placebo_pretrend_difference={:.6}\n",
            "CLAIM_CEILING synthetic_known_truth=true real_factory_data=false causal_validation_claim=false machine_actuation=false"
        ),
        randomized.assignment_count,
        randomized.finite_population_ate,
        randomized.observed_assignment_ate,
        randomized.exact_randomization_expectation,
        trend.true_ate,
        trend.before_after_treated_change,
        trend.naive_change_difference,
        hidden.true_ate,
        hidden.naive_change_difference,
        hidden.placebo_pretrend_difference
    ))
}

#[cfg(not(test))]
fn main() {
    match run_report() {
        Ok(report) => print!("{report}"),
        Err(error) => {
            eprintln!("mfg-causal-benchmark: {error}");
            std::process::exit(2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(left: f64, right: f64, tolerance: f64) -> bool {
        (left - right).abs() <= tolerance
    }

    #[test]
    fn exact_randomization_expectation_matches_finite_population_ate() {
        let result = randomized_result().unwrap();
        assert_eq!(result.assignment_count, 16);
        assert!(close(
            result.exact_randomization_expectation,
            result.finite_population_ate,
            1e-12
        ));
    }

    #[test]
    fn one_realized_assignment_is_not_mistaken_for_the_exact_ate() {
        let result = randomized_result().unwrap();
        assert!(!close(
            result.observed_assignment_ate,
            result.finite_population_ate,
            1e-12
        ));
        // A realized randomized estimate varies. The exact randomization
        // expectation, not one selected assignment, is the known-answer oracle.
    }

    #[test]
    fn before_after_reports_false_benefit_under_zero_effect_time_trend() {
        let result = confounding_result(&time_trend_zero_effect_panel()).unwrap();
        assert_eq!(result.true_ate, 0.0);
        assert_eq!(result.before_after_treated_change, -6.0);
        assert_eq!(result.naive_change_difference, 0.0);
    }

    #[test]
    fn hidden_confounding_creates_naive_benefit_and_placebo_signal() {
        let result = confounding_result(&hidden_confounding_panel()).unwrap();
        assert_eq!(result.true_ate, 0.0);
        assert_eq!(result.naive_change_difference, -4.0);
        assert_eq!(result.placebo_pretrend_difference, -4.0);
    }

    #[test]
    fn exact_enumeration_rejects_empty_and_oversized_designs() {
        assert!(exact_randomization_expectation(&[]).is_err());
        assert!(paired_estimate(&randomized_panel(), usize::MAX).is_err());
    }

    #[test]
    fn deterministic_report_replays_byte_for_byte() {
        assert_eq!(run_report().unwrap(), run_report().unwrap());
    }
}
