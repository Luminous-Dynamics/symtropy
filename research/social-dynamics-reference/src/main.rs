use symtropy_social_dynamics_reference::{simulate, Metrics, Scenario, Treatment};

const BOOTSTRAP_REPLICATES: usize = 2_000;

fn mean(values: impl Iterator<Item = f64>) -> Option<f64> {
    let values: Vec<f64> = values.collect();
    if values.is_empty() {
        None
    } else {
        Some(values.iter().sum::<f64>() / values.len() as f64)
    }
}

/// SplitMix64 stream used only to make bootstrap resampling reproducible.
fn next_random(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Paired bootstrap interval for percentage reduction in mean unmet need.
/// Positive means the treatment reduces the loss relative to the baseline.
/// Pairing requires each slice to be ordered by the same declared seed list.
fn paired_reduction_ci95_pct(
    treatment: &[Metrics],
    baseline: &[Metrics],
    replicates: usize,
    seed: u64,
) -> Option<(f64, f64, f64)> {
    if treatment.is_empty() || treatment.len() != baseline.len() || replicates < 100 {
        return None;
    }
    let baseline_mean = mean(baseline.iter().map(|m| m.unmet_need_integral))?;
    let treatment_mean = mean(treatment.iter().map(|m| m.unmet_need_integral))?;
    if !baseline_mean.is_finite() || baseline_mean <= f64::EPSILON {
        return None;
    }
    let observed = 100.0 * (baseline_mean - treatment_mean) / baseline_mean;

    let mut rng_state = seed;
    let n = treatment.len();
    let mut distribution = Vec::with_capacity(replicates);
    for _ in 0..replicates {
        let mut baseline_sum = 0.0;
        let mut treatment_sum = 0.0;
        for _ in 0..n {
            let index = (next_random(&mut rng_state) % n as u64) as usize;
            baseline_sum += baseline[index].unmet_need_integral;
            treatment_sum += treatment[index].unmet_need_integral;
        }
        let sampled_baseline_mean = baseline_sum / n as f64;
        let sampled_treatment_mean = treatment_sum / n as f64;
        if sampled_baseline_mean > f64::EPSILON {
            distribution.push(
                100.0 * (sampled_baseline_mean - sampled_treatment_mean)
                    / sampled_baseline_mean,
            );
        }
    }
    if distribution.len() < replicates * 9 / 10 {
        return None;
    }
    distribution.sort_by(f64::total_cmp);
    let low = ((distribution.len() - 1) as f64 * 0.025).floor() as usize;
    let high = ((distribution.len() - 1) as f64 * 0.975).ceil() as usize;
    Some((observed, distribution[low], distribution[high]))
}

fn format_optional(value: Option<f64>) -> String {
    value.map(|v| format!("{v:.4}")).unwrap_or_else(|| "NA".to_owned())
}

fn main() {
    let scenario = Scenario::default();
    if let Err(error) = scenario.validate() {
        eprintln!("invalid scenario: {error}");
        std::process::exit(2);
    }

    let runs = 32_u64;
    let mut all_results: Vec<(Treatment, Vec<Metrics>)> =
        Vec::with_capacity(Treatment::ALL.len());
    for treatment in Treatment::ALL {
        let mut results = Vec::with_capacity(runs as usize);
        for seed in 0..runs {
            match simulate(&scenario, seed, treatment) {
                Ok(metrics) => results.push(metrics),
                Err(error) => {
                    eprintln!("simulation failed for seed {seed}: {error}");
                    std::process::exit(2);
                }
            }
        }
        all_results.push((treatment, results));
    }

    let baseline = all_results
        .iter()
        .find(|(treatment, _)| *treatment == Treatment::NoCommitments)
        .map(|(_, results)| results)
        .expect("Treatment::ALL must contain the declared baseline");

    println!(
        "treatment,n_runs,mean_unmet_need_integral,paired_reduction_vs_no_commitments_pct,paired_reduction_ci95_low_pct,paired_reduction_ci95_high_pct,mean_unmet_need_inequality_gini,mean_contribution_count,recovery_rate,mean_recovery_tick_when_recovered,verification_overhead,mean_repair_progress_at_shock_end"
    );

    for (treatment, results) in &all_results {
        let recovery_ticks: Vec<f64> = results
            .iter()
            .filter_map(|metrics| metrics.recovery_tick.map(|tick| tick as f64))
            .collect();
        let mean_unmet = mean(results.iter().map(|m| m.unmet_need_integral)).unwrap_or(0.0);
        let mean_inequality =
            mean(results.iter().map(|m| m.unmet_need_inequality_gini)).unwrap_or(0.0);
        let mean_contributions =
            mean(results.iter().map(|m| m.contribution_count as f64)).unwrap_or(0.0);
        let mean_overhead =
            mean(results.iter().map(|m| m.verification_overhead)).unwrap_or(0.0);
        let mean_repair =
            mean(results.iter().map(|m| m.repair_progress_at_shock_end)).unwrap_or(0.0);
        let recovery_rate = results
            .iter()
            .filter(|metrics| metrics.recovery_tick.is_some())
            .count() as f64
            / results.len() as f64;
        let mean_recovery = mean(recovery_ticks.iter().copied());
        let reduction = if *treatment == Treatment::NoCommitments {
            Some((0.0, 0.0, 0.0))
        } else {
            paired_reduction_ci95_pct(
                results,
                baseline,
                BOOTSTRAP_REPLICATES,
                0x534F_4349_414C_0001 ^ (results.len() as u64),
            )
        };

        println!(
            "{},{},{:.6},{},{},{},{:.6},{:.4},{:.4},{},{:.4},{:.6}",
            treatment.as_str(),
            results.len(),
            mean_unmet,
            format_optional(reduction.map(|r| r.0)),
            format_optional(reduction.map(|r| r.1)),
            format_optional(reduction.map(|r| r.2)),
            mean_inequality,
            mean_contributions,
            recovery_rate,
            format_optional(mean_recovery),
            mean_overhead,
            mean_repair
        );
    }

    eprintln!(
        "note: paired bootstrap CI uses {BOOTSTRAP_REPLICATES} deterministic resamples; positive reductions mean lower unmet need. Synthetic outputs are not empirical validity evidence."
    );
}
