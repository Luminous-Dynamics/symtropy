use std::io::Write;
use symtropy_social_dynamics_reference::{simulate, Metrics, Scenario, Treatment};

const BOOTSTRAP_REPLICATES: usize = 2_000;

struct ScenarioResults {
    id: String,
    config: Scenario,
    treatment_results: Vec<(Treatment, Vec<Metrics>)>,
}

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
    *state = (*state).wrapping_add(0x9E37_79B9_7F4A_7C15);
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

/// One-factor-at-a-time robustness screen. These cases are sensitivity probes,
/// not an exhaustive factorial design and not empirical calibration.
fn scenario_cases(base: &Scenario, sweep: bool) -> Vec<(String, Scenario)> {
    let mut cases = vec![("baseline".to_owned(), base.clone())];
    if !sweep {
        return cases;
    }

    let mut config = base.clone();
    config.observation_noise = 0.0;
    cases.push(("observation_noise_0.00".to_owned(), config));
    let mut config = base.clone();
    config.observation_noise = 0.20;
    cases.push(("observation_noise_0.20".to_owned(), config));

    let mut config = base.clone();
    config.observation_cost = 0.0;
    cases.push(("observation_cost_0.00".to_owned(), config));
    let mut config = base.clone();
    config.observation_cost = 0.35;
    cases.push(("observation_cost_0.35".to_owned(), config));

    let mut config = base.clone();
    config.attestation_tampering_rate = 0.0;
    cases.push(("tampering_rate_0.00".to_owned(), config));
    let mut config = base.clone();
    config.attestation_tampering_rate = 0.20;
    cases.push(("tampering_rate_0.20".to_owned(), config));

    let mut config = base.clone();
    config.shock_capacity = 0.25;
    cases.push(("shock_capacity_0.25".to_owned(), config));
    let mut config = base.clone();
    config.shock_capacity = 0.65;
    cases.push(("shock_capacity_0.65".to_owned(), config));

    cases
}

fn write_run_csv(path: &str, all_results: &[ScenarioResults]) -> Result<(), String> {
    let mut file = std::fs::File::create(path)
        .map_err(|error| format!("cannot create run CSV {path}: {error}"))?;
    writeln!(
        file,
        "scenario_id,treatment,seed,population,ticks,observation_noise,observation_cost,attestation_tampering_rate,shock_capacity,unmet_need_integral,peak_unmet_need,unmet_need_inequality_gini,contribution_count,recovery_tick,observation_overhead,repair_progress_at_shock_end"
    )
    .map_err(|error| format!("cannot write run CSV header: {error}"))?;

    for scenario in all_results {
        for (treatment, results) in &scenario.treatment_results {
            for (seed, metrics) in results.iter().enumerate() {
                let recovery_tick = metrics
                    .recovery_tick
                    .map(|tick| tick.to_string())
                    .unwrap_or_else(|| "NA".to_owned());
                writeln!(
                    file,
                    "{},{},{},{},{},{:.4},{:.4},{:.4},{:.4},{:.6},{:.6},{:.6},{},{},{:.4},{:.6}",
                    scenario.id,
                    treatment.as_str(),
                    seed,
                    scenario.config.population,
                    scenario.config.ticks,
                    scenario.config.observation_noise,
                    scenario.config.observation_cost,
                    scenario.config.attestation_tampering_rate,
                    scenario.config.shock_capacity,
                    metrics.unmet_need_integral,
                    metrics.peak_unmet_need,
                    metrics.unmet_need_inequality_gini,
                    metrics.contribution_count,
                    recovery_tick,
                    metrics.observation_overhead,
                    metrics.repair_progress_at_shock_end
                )
                .map_err(|error| format!("cannot write run CSV row: {error}"))?;
            }
        }
    }
    file.flush()
        .map_err(|error| format!("cannot flush run CSV: {error}"))?;
    Ok(())
}

fn main() {
    let mut runs = 32_u64;
    let mut runs_csv_path: Option<String> = None;
    let mut run_sweep = false;
    let mut args = std::env::args().skip(1);
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--runs" => {
                let value = args.next().unwrap_or_else(|| {
                    eprintln!("--runs requires an integer value");
                    std::process::exit(2);
                });
                runs = value.parse::<u64>().unwrap_or_else(|_| {
                    eprintln!("--runs must be an integer >= 20");
                    std::process::exit(2);
                });
                if runs < 20 {
                    eprintln!("--runs must be at least 20 for the bootstrap report");
                    std::process::exit(2);
                }
            }
            "--runs-csv" => {
                runs_csv_path = Some(args.next().unwrap_or_else(|| {
                    eprintln!("--runs-csv requires an output path");
                    std::process::exit(2);
                }));
            }
            "--sweep" => run_sweep = true,
            "--help" | "-h" => {
                println!(
                    "Usage: social-dynamics-reference [--runs N] [--runs-csv PATH] [--sweep]\n\
                     --runs N        Number of paired seeds (minimum 20; default 32).\n\
                     --runs-csv PATH Write one row per scenario/treatment/seed for audit.\n\
                     --sweep         Add one-factor sensitivity cases (9 scenarios total)."
                );
                return;
            }
            unknown => {
                eprintln!("unknown argument: {unknown}; use --help");
                std::process::exit(2);
            }
        }
    }

    let base = Scenario::default();
    let cases = scenario_cases(&base, run_sweep);
    let mut all_results: Vec<ScenarioResults> = Vec::with_capacity(cases.len());
    for (id, config) in cases {
        if let Err(error) = config.validate() {
            eprintln!("invalid scenario {id}: {error}");
            std::process::exit(2);
        }
        let mut treatment_results: Vec<(Treatment, Vec<Metrics>)> =
            Vec::with_capacity(Treatment::ALL.len());
        for treatment in Treatment::ALL {
            let mut results = Vec::with_capacity(runs as usize);
            for seed in 0..runs {
                match simulate(&config, seed, treatment) {
                    Ok(metrics) => results.push(metrics),
                    Err(error) => {
                        eprintln!("simulation failed for {id}, seed {seed}: {error}");
                        std::process::exit(2);
                    }
                }
            }
            treatment_results.push((treatment, results));
        }
        all_results.push(ScenarioResults {
            id,
            config,
            treatment_results,
        });
    }

    if let Some(path) = runs_csv_path.as_deref() {
        if let Err(error) = write_run_csv(path, &all_results) {
            eprintln!("{error}");
            std::process::exit(2);
        }
        eprintln!("wrote one row per scenario/treatment/seed to {path}");
    }

    println!(
        "scenario_id,treatment,n_runs,mean_unmet_need_integral,mean_peak_unmet_need,paired_reduction_vs_no_commitments_pct,paired_reduction_ci95_low_pct,paired_reduction_ci95_high_pct,mean_unmet_need_inequality_gini,mean_contribution_count,recovery_rate,mean_recovery_tick_when_recovered,mean_observation_overhead,mean_repair_progress_at_shock_end"
    );

    for (scenario_index, scenario) in all_results.iter().enumerate() {
        let baseline = scenario
            .treatment_results
            .iter()
            .find(|(treatment, _)| *treatment == Treatment::NoCommitments)
            .map(|(_, results)| results)
            .expect("Treatment::ALL must contain the declared baseline");

        for (treatment, results) in &scenario.treatment_results {
            let recovery_ticks: Vec<f64> = results
                .iter()
                .filter_map(|metrics| metrics.recovery_tick.map(|tick| tick as f64))
                .collect();
            let mean_unmet =
                mean(results.iter().map(|m| m.unmet_need_integral)).unwrap_or(0.0);
            let mean_peak =
                mean(results.iter().map(|m| m.peak_unmet_need)).unwrap_or(0.0);
            let mean_inequality =
                mean(results.iter().map(|m| m.unmet_need_inequality_gini)).unwrap_or(0.0);
            let mean_contributions =
                mean(results.iter().map(|m| m.contribution_count as f64)).unwrap_or(0.0);
            let mean_overhead =
                mean(results.iter().map(|m| m.observation_overhead)).unwrap_or(0.0);
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
                    0x534F_4349_414C_0001 ^ (results.len() as u64) ^ (scenario_index as u64),
                )
            };

            println!(
                "{},{},{},{:.6},{:.6},{},{},{},{:.6},{:.4},{:.4},{},{:.4},{:.6}",
                scenario.id,
                treatment.as_str(),
                results.len(),
                mean_unmet,
                mean_peak,
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
    }

    eprintln!(
        "note: paired bootstrap CI uses {BOOTSTRAP_REPLICATES} deterministic resamples per treatment/scenario; positive reductions mean lower simulated unmet need. Synthetic outputs are not empirical validity evidence."
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn metric(treatment: Treatment, unmet_need_integral: f64) -> Metrics {
        Metrics {
            treatment,
            unmet_need_integral,
            peak_unmet_need: 1.0,
            unmet_need_inequality_gini: 0.0,
            contribution_count: 0,
            recovery_tick: None,
            observation_overhead: 0.0,
            repair_progress_at_shock_end: 0.0,
        }
    }

    #[test]
    fn paired_bootstrap_recovers_known_reduction() {
        let baseline = [
            metric(Treatment::NoCommitments, 10.0),
            metric(Treatment::NoCommitments, 20.0),
            metric(Treatment::NoCommitments, 30.0),
            metric(Treatment::NoCommitments, 40.0),
        ];
        let treatment = [
            metric(Treatment::Reciprocity, 5.0),
            metric(Treatment::Reciprocity, 10.0),
            metric(Treatment::Reciprocity, 15.0),
            metric(Treatment::Reciprocity, 20.0),
        ];
        let result = paired_reduction_ci95_pct(&treatment, &baseline, 2_000, 12345)
            .expect("valid paired data should produce an interval");
        assert!((result.0 - 50.0).abs() < 1e-10);
        assert!((result.1 - 50.0).abs() < 1e-10);
        assert!((result.2 - 50.0).abs() < 1e-10);
    }

    #[test]
    fn paired_bootstrap_returns_negative_for_a_worse_treatment() {
        let baseline = [
            metric(Treatment::NoCommitments, 10.0),
            metric(Treatment::NoCommitments, 20.0),
            metric(Treatment::NoCommitments, 30.0),
        ];
        let treatment = [
            metric(Treatment::Reciprocity, 15.0),
            metric(Treatment::Reciprocity, 30.0),
            metric(Treatment::Reciprocity, 45.0),
        ];
        let result = paired_reduction_ci95_pct(&treatment, &baseline, 2_000, 9876)
            .expect("valid paired data should produce an interval");
        assert!((result.0 + 50.0).abs() < 1e-10);
        assert!(result.1 <= result.0);
        assert!(result.2 >= result.0);
    }

    #[test]
    fn paired_bootstrap_fails_closed_on_invalid_inputs() {
        let baseline = [metric(Treatment::NoCommitments, 10.0)];
        let treatment = [
            metric(Treatment::Reciprocity, 5.0),
            metric(Treatment::Reciprocity, 5.0),
        ];
        assert!(paired_reduction_ci95_pct(&treatment, &baseline, 2_000, 1).is_none());

        let zero_baseline = [metric(Treatment::NoCommitments, 0.0)];
        let zero_treatment = [metric(Treatment::Reciprocity, 0.0)];
        assert!(paired_reduction_ci95_pct(&zero_treatment, &zero_baseline, 2_000, 1).is_none());
        assert!(paired_reduction_ci95_pct(&baseline, &baseline, 10, 1).is_none());
    }

    #[test]
    fn sensitivity_manifest_has_nine_valid_unique_scenarios() {
        let base = Scenario::default();
        let cases = scenario_cases(&base, true);
        assert_eq!(cases.len(), 9);
        let mut ids: Vec<&str> = cases.iter().map(|(id, _)| id.as_str()).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), cases.len());
        for (_, scenario) in &cases {
            assert!(scenario.validate().is_ok());
        }
        assert_eq!(base.observation_noise, Scenario::default().observation_noise);
        assert_eq!(cases[0].0, "baseline");
    }

    #[test]
    fn single_scenario_mode_keeps_only_the_baseline() {
        let cases = scenario_cases(&Scenario::default(), false);
        assert_eq!(cases.len(), 1);
        assert_eq!(cases[0].0, "baseline");
    }
}
