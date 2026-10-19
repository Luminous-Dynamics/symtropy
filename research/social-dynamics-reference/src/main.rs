use symtropy_social_dynamics_reference::{simulate, Scenario, Treatment};

fn mean(values: impl Iterator<Item = f64>) -> Option<f64> {
    let values: Vec<f64> = values.collect();
    if values.is_empty() {
        None
    } else {
        Some(values.iter().sum::<f64>() / values.len() as f64)
    }
}

fn main() {
    let scenario = Scenario::default();
    if let Err(error) = scenario.validate() {
        eprintln!("invalid scenario: {error}");
        std::process::exit(2);
    }

    let runs = 32_u64;
    println!(
        "treatment,n_runs,mean_unmet_need_integral,mean_unmet_need_inequality_gini,mean_contribution_count,recovery_rate,mean_recovery_tick_when_recovered,verification_overhead,mean_repair_progress_at_shock_end"
    );

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
        let mean_recovery = mean(recovery_ticks.iter().copied())
            .map(|x| format!("{x:.4}"))
            .unwrap_or_else(|| "NA".to_owned());

        println!(
            "{},{},{:.6},{:.6},{:.4},{:.4},{},{:.4},{:.6}",
            treatment.as_str(),
            results.len(),
            mean_unmet,
            mean_inequality,
            mean_contributions,
            recovery_rate,
            mean_recovery,
            mean_overhead,
            mean_repair
        );
    }
}
