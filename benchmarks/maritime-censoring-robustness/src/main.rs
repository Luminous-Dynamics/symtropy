use maritime_censoring_robustness::evaluate_reference_scenarios;

fn main() {
    let report = evaluate_reference_scenarios();
    println!("Maritime censoring robustness benchmark v0.1");
    println!(
        "{} calls per seeded scenario; horizon={} synthetic ticks",
        maritime_censoring_robustness::CALLS_PER_SCENARIO,
        maritime_censoring_robustness::HORIZON
    );

    for scenario in &report.scenarios {
        println!();
        println!(
            "{} qualification={}",
            scenario.scenario,
            scenario.qualification.label()
        );
        println!(
            "  latent truth: berth={:.6} diversion={:.6} cancellation={:.6}",
            scenario.true_empirical.berth,
            scenario.true_empirical.diversion,
            scenario.true_empirical.cancellation
        );
        println!(
            "  analytic truth: berth={:.6} diversion={:.6} cancellation={:.6}",
            scenario.analytic_truth.berth,
            scenario.analytic_truth.diversion,
            scenario.analytic_truth.cancellation
        );
        println!(
            "  pooled A-J: berth={:.6} diversion={:.6} cancellation={:.6} max_abs_error={:.6}",
            scenario.pooled_aalen_johansen.berth,
            scenario.pooled_aalen_johansen.diversion,
            scenario.pooled_aalen_johansen.cancellation,
            scenario.pooled_aj_error
        );
        println!(
            "  standardized A-J: berth={:.6} diversion={:.6} \
             cancellation={:.6} max_abs_error={:.6}",
            scenario.standardized_stratum_aalen_johansen.berth,
            scenario.standardized_stratum_aalen_johansen.diversion,
            scenario.standardized_stratum_aalen_johansen.cancellation,
            scenario.standardized_aj_error
        );

        match scenario.known_g_conditional_ipcw {
            Some(incidence) => println!(
                "  known-G conditional IPCW: berth={:.6} diversion={:.6} \
                 cancellation={:.6} max_abs_error={:.6}",
                incidence.berth,
                incidence.diversion,
                incidence.cancellation,
                scenario.conditional_ipcw_error.unwrap_or(f64::NAN)
            ),
            None => println!("  known-G conditional IPCW: NOT_EVALUABLE_FOR_THIS_MECHANISM"),
        }

        println!(
            "  unresolved={:.4} administrative_censors={} reporting_lag_censors={}",
            scenario.unresolved_fraction,
            scenario.administrative_censored_calls,
            scenario.reporting_lag_censored_calls
        );
        println!(
            "  observed-only berth bound=[{:.6}, {:.6}]",
            scenario.partial_identification_bounds.lower.berth,
            scenario.partial_identification_bounds.upper.berth
        );
        if let Some(minimum_g) = scenario.minimum_censoring_survival {
            println!(
                "  minimum horizon G(t|X)={:.6} maximum observed IPCW weight={:.6}",
                minimum_g,
                scenario.maximum_ipcw_weight.unwrap_or(f64::NAN)
            );
        } else {
            println!("  censoring support: ZERO / NOT_EVALUABLE");
        }
        println!(
            "  stratum-1 berth count: latent={} observed={}",
            scenario.stratum_1_true_berths, scenario.stratum_1_observed_berths
        );
    }

    println!();
    println!("Same-latent-cohort reporting-delay sensitivity:");
    for step in &report.reporting_delay_sweep {
        println!(
            "  delay={} ticks true_berth_CIF={:.6} observed_AJ={:.6} \
             underestimation={:.6} unresolved={:.4}",
            step.reporting_delay_ticks,
            step.true_berth_cif,
            step.observed_berth_cif,
            step.berth_underestimation,
            step.unresolved_fraction
        );
    }

    println!();
    println!("This is a seeded synthetic mechanism stress test, not a port-data estimate.");
}
