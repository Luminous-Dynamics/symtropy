use human_multirate_cosim::noise_robustness::run_ar1_negative_control;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let report = run_ar1_negative_control(200_000, 20_261_118, 0.8)?;
    println!(
        "noise_robustness,process,AR1,rho,{:.3},trials,{},seed,{}",
        report.rho, report.trials, report.seed
    );
    println!(
        "noise_robustness,iid_negative_control,false_rejection_count,{},rate,{:.6},wilson_95,[{:.6},{:.6}],threshold,{:.12e}",
        report.iid_assumption_false_rejection_count,
        report.iid_assumption_false_rejection_rate,
        report.iid_assumption_false_rejection_wilson_95[0],
        report.iid_assumption_false_rejection_wilson_95[1],
        report.nominal_iid_threshold,
    );
    println!(
        "noise_robustness,full_covariance,false_rejection_count,{},rate,{:.6},wilson_95,[{:.6},{:.6}],threshold,{:.12e}",
        report.full_covariance_false_rejection_count,
        report.full_covariance_false_rejection_rate,
        report.full_covariance_false_rejection_wilson_95[0],
        report.full_covariance_false_rejection_wilson_95[1],
        report.nominal_iid_threshold,
    );
    Ok(())
}
