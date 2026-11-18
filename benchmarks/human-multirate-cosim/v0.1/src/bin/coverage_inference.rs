use human_multirate_cosim::coverage_inference::{
    CoverageOutcomeV1, CoverageSeriesV1, DelayedAciConfigV1, DelayedAciSchedulerV1,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let indicators: Vec<bool> = (0..120)
        .map(|index| (index % 16) < 8)
        .collect();
    let outcomes = indicators
        .iter()
        .enumerate()
        .map(|(index, covered)| CoverageOutcomeV1 {
            forecast_id: index as u64,
            issue_step: index as u64,
            availability_step: index as u64 + 3,
            issue_time: index as f64,
            event_time: index as f64 + 0.5,
            availability_time: index as f64 + 3.0,
            covered: *covered,
        })
        .collect();
    let series = CoverageSeriesV1::new(
        "frozen-block-pattern-v1".to_owned(),
        outcomes,
    )?;
    let hac = series.hac_report(0.9, 8)?;
    let bootstrap = series.moving_block_bootstrap(0.9, 8, 5_000, 2026_1118)?;

    println!(
        "coverage,hac,series,{},samples,{},covered,{},realized,{:.6},target,{:.6},bandwidth,{},lrv,{:.12e},se,{:.12e},normal_95,[{:.6},{:.6}],iid_wilson_95,[{:.6},{:.6}],ess_diagnostic,{}",
        hac.series_id,
        hac.samples,
        hac.covered_count,
        hac.realized_coverage,
        hac.target_coverage,
        hac.bandwidth,
        hac.long_run_variance,
        hac.hac_standard_error,
        hac.hac_normal_95[0],
        hac.hac_normal_95[1],
        hac.iid_wilson_95[0],
        hac.iid_wilson_95[1],
        hac.effective_sample_size_diagnostic
            .map(|value| format!("{value:.6}"))
            .unwrap_or_else(|| "not_available".to_owned()),
    );
    println!(
        "coverage,block_bootstrap,block_length,{},replicates,{},seed,{},percentile_95,[{:.6},{:.6}]",
        bootstrap.block_length,
        bootstrap.replicates,
        bootstrap.seed,
        bootstrap.percentile_95[0],
        bootstrap.percentile_95[1],
    );

    let config = DelayedAciConfigV1 {
        target_miscoverage: 0.1,
        initial_miscoverage: 0.1,
        learning_rate: 0.05,
        minimum_miscoverage: 0.01,
        maximum_miscoverage: 0.5,
        delay_steps: 3,
    };
    let mut scheduler = DelayedAciSchedulerV1::new(config)?;
    for step in 0_u64..30 {
        if step >= config.delay_steps {
            let forecast_index = step - config.delay_steps;
            let forecast_id = 10_000 + forecast_index;
            let covered = indicators[forecast_index as usize];
            let receipt = scheduler.resolve(forecast_id, step, covered)?;
            println!(
                "aci,update,forecast_id,{},issue_step,{},available_step,{},lane,{},covered,{},alpha_at_issue,{:.6},alpha_after,{:.6},clipped,{}",
                receipt.forecast_id,
                receipt.issue_step,
                receipt.availability_step,
                receipt.lane,
                receipt.covered,
                receipt.miscoverage_at_issue,
                receipt.miscoverage_after_feedback,
                receipt.clipping_applied,
            );
        }
        let ticket = scheduler.issue(10_000 + step, step)?;
        println!(
            "aci,issue,forecast_id,{},issue_step,{},due_step,{},lane,{},alpha,{:.6}",
            ticket.forecast_id,
            ticket.issue_step,
            ticket.due_step,
            ticket.lane,
            ticket.miscoverage_at_issue,
        );
    }
    Ok(())
}
