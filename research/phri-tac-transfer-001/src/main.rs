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

            match phri_tac_transfer_oracle::active_benchmark::run_active_calibration_campaign(
                20261116, 50,
            ) {
                Ok(campaign) => {
                    println!("active_calibration_campaign=paired_seed_v0_1");
                    println!("campaign_first_seed={}", campaign.first_seed);
                    println!("campaign_replicates={}", campaign.replicate_count);
                    for result in campaign.results {
                        println!(
                            "campaign_policy={} source_contacts={} target_contacts={} rmse_mean={:.6} rmse_median={:.6} rmse_p95={:.6} rmse_max={:.6} coverage95_mean={:.4} paired_rmse_delta_mean={:.6} paired_rmse_delta_p95={:.6} paired_rmse_delta_bootstrap95_low={:.6} paired_rmse_delta_bootstrap95_high={:.6} cost_ms_mean={:.2}",
                            result.policy_id,
                            result.source_reference_contacts,
                            result.target_reference_contacts,
                            result.rmse_n.mean,
                            result.rmse_n.median,
                            result.rmse_n.p95,
                            result.rmse_n.maximum,
                            result.prediction_interval_coverage_95.mean,
                            result.paired_rmse_delta_vs_full_grid_n.mean,
                            result.paired_rmse_delta_vs_full_grid_n.p95,
                            result.paired_rmse_delta_vs_full_grid_bootstrap_95.lower_95,
                            result.paired_rmse_delta_vs_full_grid_bootstrap_95.upper_95,
                            result.total_estimated_fixture_cost_ms.mean,
                        );
                    }
                }
                Err(error) => {
                    eprintln!("active_calibration_campaign=invalid: {error}");
                    return ExitCode::FAILURE;
                }
            }
            match phri_tac_transfer_oracle::active_benchmark::adversarial::run_adversarial_campaign(
                20261116, 50,
            ) {
                Ok(campaign) => {
                    println!("adversarial_response_campaign=paired_seed_v0_1");
                    println!("adversarial_family_count={}", campaign.families.len());
                    println!("adversarial_replicates={}", campaign.replicate_count);
                    for family in campaign.families {
                        for result in family.policies {
                            println!(
                                "adversarial_family={} policy={} source_contacts={} target_contacts={} rmse_mean={:.6} rmse_p95={:.6} worst_error_mean={:.6} interval_coverage_mean={:.4} paired_rmse_delta_mean={:.6} bootstrap95_low={:.6} bootstrap95_high={:.6}",
                                family.family_id,
                                result.policy_id,
                                result.source_reference_contacts,
                                result.target_reference_contacts,
                                result.rmse_n.mean,
                                result.rmse_n.p95,
                                result.worst_absolute_error_n.mean,
                                result.prediction_interval_coverage_95.mean,
                                result.paired_rmse_delta_vs_full_grid_n.mean,
                                result.paired_rmse_delta_bootstrap_95.lower_95,
                                result.paired_rmse_delta_bootstrap_95.upper_95,
                            );
                        }
                    }
                }
                Err(error) => {
                    eprintln!("adversarial_response_campaign=invalid: {error}");
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
