use maritime_wait_cohort::evaluate_reference_cohorts;

fn quantile(value: Option<u64>) -> String {
    value.map_or_else(|| "NOT_ESTIMABLE".to_string(), |ticks| ticks.to_string())
}

fn main() {
    let report = evaluate_reference_cohorts().unwrap_or_else(|error| {
        eprintln!("wait cohort evaluation failed: {error}");
        std::process::exit(2);
    });

    println!("Maritime competing-risk wait cohort benchmark v0.1");
    println!("Synthetic ticks only; do not interpret ticks as real hours.");
    for cohort in &report.cohorts {
        println!(
            "{} as_of={} calls={} berth={} diverted={} cancelled={} censored={} (admin:{},reporting_lag:{}) vessel_ticks={} integrated_queue_ticks={} mean_exposure_ticks={:.4}",
            cohort.scenario_id,
            cohort.as_of_tick,
            cohort.call_count,
            cohort.berth_count,
            cohort.diversion_count,
            cohort.cancellation_count,
            cohort.censored_count,
            cohort.administrative_censored_count,
            cohort.reporting_lag_censored_count,
            cohort.total_vessel_ticks,
            cohort.integrated_queue_vessel_ticks,
            cohort.mean_observed_queue_ticks
        );
        println!(
            "  berth-CIF quantile ticks: p50={} p90={} p95={}",
            quantile(cohort.p50_berth_incidence_tick),
            quantile(cohort.p90_berth_incidence_tick),
            quantile(cohort.p95_berth_incidence_tick)
        );
        for step in &cohort.risk_steps {
            println!(
                "  t={} risk={} outcomes=(berth:{},divert:{},cancel:{},censored:{}) survival={:.6} CIF=(berth:{:.6},divert:{:.6},cancel:{:.6})",
                step.duration_ticks,
                step.at_risk,
                step.berth_events,
                step.diverted_events,
                step.cancelled_events,
                step.censored,
                step.event_free_survival,
                step.berth_cumulative_incidence,
                step.diversion_cumulative_incidence,
                step.cancellation_cumulative_incidence
            );
        }
    }

    println!();
    println!("All values are computed from synthetic cohort fixtures, not observed vessel calls.");
}
