use phri_tac_transfer_oracle::validate_tracked;
use std::process::ExitCode;

fn main() -> ExitCode {
    match validate_tracked() {
        Ok(report) => {
            println!("suite=phri_tac_transfer_001_v0_1");
            println!("cases={}", report.case_count);
            println!("valid_splits={}", report.valid_splits);
            println!("leaked_splits={}", report.leaked_splits);
            println!("transfer_eligible={}", report.transfer_eligible);
            println!(
                "target_qualified_estimates={}",
                report.target_qualified_estimates
            );
            println!("model_estimate_only={}", report.model_estimate_only);
            println!(
                "invalid_measurement_claims={}",
                report.invalid_measurement_claims
            );
            println!("result=all_known_answers_matched");
            println!("claim_ceiling=synthetic_transfer_contract_only");
            match phri_tac_transfer_oracle::active_benchmark::run_active_calibration_benchmark(
                20261116,
            ) {
                Ok(benchmark) => {
                    println!("active_calibration_suite=synthetic_policy_comparison_v0_1");
                    println!(
                        "active_calibration_source_candidate_pool={}",
                        benchmark.source_candidate_pool_size
                    );
                    println!(
                        "active_calibration_target_candidate_pool={}",
                        benchmark.target_candidate_pool_size
                    );
                    println!(
                        "active_calibration_heldout_cases={}",
                        benchmark.heldout_case_count
                    );
                    println!(
                        "active_calibration_target_contact_budget={}",
                        benchmark.target_contact_budget
                    );
                    for result in benchmark.results {
                        println!(
                            "policy={} source_contacts={} target_contacts={} source_cost_ms={} target_cost_ms={} total_synthetic_cost_ms={} rmse_n={:.6} p95_abs_n={:.6} worst_abs_n={:.6} prediction_interval_coverage95={:.4} mean_interval_width_n={:.6} claim={}",
                            result.policy_id,
                            result.source_reference_contacts,
                            result.target_reference_contacts,
                            result.source_reference_cost_ms,
                            result.target_reference_cost_ms,
                            result.total_estimated_fixture_cost_ms,
                            result.rmse_n,
                            result.p95_absolute_error_n,
                            result.worst_absolute_error_n,
                            result.interval_coverage_95,
                            result.mean_interval_width_95_n,
                            result.claim_class
                        );
                    }
                }
                Err(error) => {
                    eprintln!("active_calibration_result=invalid: {error}");
                    return ExitCode::FAILURE;
                }
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("result=invalid");
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
