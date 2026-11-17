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
                    println!(
                        "active_calibration_observability=accepted:{} flatlines:{} affected_conditions:{} missing_conditions:{} incomplete_conditions:{} max_force_setpoint_n:{:.3}",
                        benchmark.response_observability.accepted,
                        benchmark.response_observability.flatline_transition_count,
                        benchmark.response_observability.affected_condition_count,
                        benchmark.response_observability.missing_condition_count,
                        benchmark.response_observability.incomplete_condition_count,
                        benchmark
                            .response_observability
                            .maximum_observed_force_setpoint_n,
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
                        println!(
                            "reference_integrity family={} accepted={} samples={} max_abs_disagreement_n={:.6} rms_disagreement_n={:.6} tolerance_n={:.6} ancestry_attested={} agreement_proves_independence={} shared_common_cause_injected={}",
                            family.family_id,
                            family.reference_integrity.accepted,
                            family.reference_integrity.sample_count,
                            family.reference_integrity.max_abs_disagreement_n,
                            family.reference_integrity.rms_disagreement_n,
                            family.reference_integrity.tolerance_n,
                            family
                                .reference_integrity
                                .independent_reference_ancestry_attested,
                            family.reference_integrity.agreement_proves_independence,
                            family.shared_reference_common_cause_injected,
                        );
                        println!(
                            "response_observability family={} accepted={} conditions={} missing_conditions={} incomplete_conditions={} transitions={} flatlines={} decreasing={} affected_conditions={} first_plateau_start_n={:?} max_force_setpoint_n={:.3} resolution={:.3e}",
                            family.family_id,
                            family.observability.accepted,
                            family.observability.condition_count,
                            family.observability.missing_condition_count,
                            family.observability.incomplete_condition_count,
                            family.observability.checked_transition_count,
                            family.observability.flatline_transition_count,
                            family.observability.decreasing_transition_count,
                            family.observability.affected_condition_count,
                            family.observability.first_flatline_start_force_n,
                            family.observability.maximum_observed_force_setpoint_n,
                            family.observability.raw_response_resolution,
                        );
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
            match phri_tac_transfer_oracle::active_benchmark::model_forms::run_model_form_campaign(
                20261116,
            ) {
                Ok(campaign) => {
                    println!("taxel_model_form_campaign=touchwgnn_style_rational_v0_1");
                    println!("taxel_count={}", campaign.taxel_count);
                    println!(
                        "full_grid_contacts_per_taxel={}",
                        campaign.full_grid_contacts_per_taxel
                    );
                    println!(
                        "heldout_force_count_per_taxel={}",
                        campaign.heldout_force_count_per_taxel
                    );
                    for result in campaign.results {
                        println!(
                            "taxel_model={} contacts_per_taxel={} total_contacts={} pooled_rmse_n={:.6} mean_taxel_rmse_n={:.6} p95_taxel_rmse_n={:.6} worst_taxel_rmse_n={:.6} worst_taxel_id={}",
                            result.model_id,
                            result.reference_contacts_per_taxel,
                            result.total_reference_contacts,
                            result.pooled_rmse_n,
                            result.mean_taxel_rmse_n,
                            result.p95_taxel_rmse_n,
                            result.worst_taxel_rmse_n,
                            result.worst_taxel_id,
                        );
                    }
                }
                Err(error) => {
                    eprintln!("taxel_model_form_campaign=invalid: {error}");
                    return ExitCode::FAILURE;
                }
            }
            match phri_tac_transfer_oracle::active_benchmark::field_campaign::run_field_reconstruction_campaign(
                20261117, 50,
            ) {
                Ok(campaign) => {
                    println!("field_reconstruction_campaign=paired_synthetic_113_taxel_v0_1");
                    match serde_json::to_string(&campaign) {
                        Ok(scorecard_json) => {
                            println!("field_reconstruction_scorecard_json={scorecard_json}");
                        }
                        Err(error) => {
                            eprintln!("field_reconstruction_scorecard_json=serialization_error:{error}");
                            return ExitCode::FAILURE;
                        }
                    }
                    println!("field_taxel_count={}", campaign.taxel_count);
                    println!("field_module_count={}", campaign.module_count);
                    println!("field_replicates={}", campaign.replicate_count);
                    for result in campaign.results {
                        println!(
                            "field_estimator={} rmse_mean={:.8} rmse_p95={:.8} rmse_max={:.8} paired_delta_mean_vs_independent={:.8} paired_delta_p95_vs_independent={:.8} sensitivity_support_mean={:.4} sensitivity_support_min={:.4} data_identifiability_mean={:.4} data_identifiability_min={:.4} underidentified_cells_mean={:.2} mean_native_rank={:.2} full_rank_replicates={} sparsity_prior_enabled={} projected_gradient_residual_mean={:.3e} projected_gradient_residual_max={:.3e} converged_replicates={}/{}",
                            result.estimator_id,
                            result.mean_field_rmse,
                            result.p95_field_rmse,
                            result.maximum_field_rmse,
                            result.paired_rmse_delta_mean_vs_independent,
                            result.paired_rmse_delta_p95_vs_independent,
                            result.mean_sensitivity_support_ratio,
                            result.minimum_sensitivity_support_ratio,
                            result.mean_native_data_identifiability_ratio,
                            result.minimum_native_data_identifiability_ratio,
                            result.mean_underidentified_cells,
                            result.mean_native_data_rank,
                            result.full_rank_replicates,
                            result.sparsity_prior_enabled,
                            result.mean_projected_gradient_residual,
                            result.maximum_projected_gradient_residual,
                            result.converged_replicates,
                            result.replicate_count,
                        );
                    }
                }
                Err(error) => {
                    eprintln!("field_reconstruction_campaign=invalid: {error}");
                    return ExitCode::FAILURE;
                }
            }
            match phri_tac_transfer_oracle::active_benchmark::coupling_campaign::run_coupling_design_campaign(
                20261117, 50,
            ) {
                Ok(campaign) => {
                    println!("coupling_design_campaign=paired_synthetic_v0_1");
                    match serde_json::to_string(&campaign) {
                        Ok(scorecard_json) => {
                            println!("coupling_design_scorecard_json={scorecard_json}");
                        }
                        Err(error) => {
                            eprintln!("coupling_design_scorecard_json=serialization_error:{error}");
                            return ExitCode::FAILURE;
                        }
                    }
                    for result in campaign.results {
                        println!(
                            "coupling_design={:?} patterns={} active_site_excitations_mean={:.2} identified_rows_mean={:.2} underidentified_rows_mean={:.2} coefficient_rmse_mean={:?} heldout_rmse_mean={:?} heldout_rmse_max={:?} paired_heldout_delta_mean_vs_isolated={:?} evaluated_replicates={}/{}",
                            result.design,
                            result.training_pattern_count,
                            result.mean_active_taxel_site_excitations,
                            result.mean_identified_output_rows,
                            result.mean_underidentified_output_rows,
                            result.mean_coefficient_rmse,
                            result.mean_heldout_response_rmse,
                            result.maximum_heldout_response_rmse,
                            result.paired_heldout_rmse_delta_mean_vs_isolated,
                            result.evaluated_replicates,
                            result.replicate_count,
                        );
                    }
                }
                Err(error) => {
                    eprintln!("coupling_design_campaign=invalid: {error}");
                    return ExitCode::FAILURE;
                }
            }
            match phri_tac_transfer_oracle::active_benchmark::wrench_consistency::run_synthetic_wrench_example() {
                Ok(example) => {
                    println!("wrench_consistency_example=synthetic_v0_1");
                    match serde_json::to_string(&example) {
                        Ok(scorecard_json) => {
                            println!("wrench_consistency_scorecard_json={scorecard_json}");
                        }
                        Err(error) => {
                            eprintln!("wrench_consistency_scorecard_json=serialization_error:{error}");
                            return ExitCode::FAILURE;
                        }
                    }
                    println!(
                        "wrench_consistency={:?} native_rank={} wrench_rank={} combined_rank={} incremental_wrench_rank={} native_null={} combined_null={} native_identifiable_scalars={} combined_identifiable_scalars={} fully_identifiable_taxels={}/{}",
                        example.consistency.disposition,
                        example.identifiability.native_observation_rank,
                        example.identifiability.wrench_constraint_rank,
                        example.identifiability.combined_rank,
                        example.identifiability.incremental_rank_from_wrench,
                        example.identifiability.native_null_space_dimension,
                        example.identifiability.combined_null_space_dimension,
                        example.identifiability.native_identifiable_scalar_count,
                        example.identifiability.combined_identifiable_scalar_count,
                        example.identifiability.taxels_fully_identifiable_combined,
                        example.identifiability.per_taxel.len(),
                    );
                }
                Err(error) => {
                    eprintln!("wrench_consistency_example=invalid: {error}");
                    return ExitCode::FAILURE;
                }
            }
            match phri_tac_transfer_oracle::active_benchmark::uncertainty_qualification::run_synthetic_conformal_example() {
                Ok(example) => {
                    println!("tactile_conformal_coverage_example=synthetic_groupwise_v0_1");
                    match serde_json::to_string(&example) {
                        Ok(scorecard_json) => {
                            println!("tactile_conformal_scorecard_json={scorecard_json}");
                        }
                        Err(error) => {
                            eprintln!("tactile_conformal_scorecard_json=serialization_error:{error}");
                            return ExitCode::FAILURE;
                        }
                    }
                    println!(
                        "tactile_conformal nominal={:.3} scored={} covered={} missed={} coverage={:?} wilson95=[{:?},{:?}] abstain_ood={} abstain_reference={} no_stratum_calibrator={} insufficient_stratum_data={}",
                        example.evaluation.nominal_coverage,
                        example.evaluation.scored_count,
                        example.evaluation.covered_count,
                        example.evaluation.missed_count,
                        example.evaluation.overall_empirical_coverage,
                        example.evaluation.overall_wilson_95_lower,
                        example.evaluation.overall_wilson_95_upper,
                        example.evaluation.abstained_out_of_domain_count,
                        example.evaluation.abstained_reference_unresolved_count,
                        example.evaluation.no_calibrator_count,
                        example.evaluation.insufficient_calibration_count,
                    );
                    for stratum in example.evaluation.strata {
                        println!(
                            "tactile_conformal_stratum={} context={} n={} covered={} coverage={:?} wilson95=[{:?},{:?}] mean_width={:?} abstain_ood={} abstain_reference={} no_calibrator={} insufficient={}",
                            stratum.stratum_id,
                            stratum.context_digest,
                            stratum.evaluated_count,
                            stratum.covered_count,
                            stratum.empirical_coverage,
                            stratum.wilson_95_lower,
                            stratum.wilson_95_upper,
                            stratum.mean_interval_width,
                            stratum.abstained_out_of_domain_count,
                            stratum.abstained_reference_unresolved_count,
                            stratum.no_calibrator_count,
                            stratum.insufficient_calibration_count,
                        );
                    }
                }
                Err(error) => {
                    eprintln!("tactile_conformal_example=invalid: {error}");
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
