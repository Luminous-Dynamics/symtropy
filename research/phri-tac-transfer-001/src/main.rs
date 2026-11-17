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
            match phri_tac_transfer_oracle::active_benchmark::prior_study::run_prior_study(
                20261117, 20261217, 50,
            ) {
                Ok(study) => {
                    println!("field_prior_study=disjoint_development_final_v0_1");
                    println!(
                        "prior_development_seed_first={}",
                        study.development_seed_first
                    );
                    println!("prior_final_seed_first={}", study.final_seed_first);
                    println!(
                        "prior_replicates_per_morphology={}",
                        study.replicate_count_per_morphology
                    );
                    println!("prior_selected_l1_penalty={:?}", study.selected_l1_penalty);
                    println!(
                        "prior_l1_selection_disposition={}",
                        study.l1_selection_disposition
                    );
                    for candidate in &study.development_l1_candidates {
                        println!(
                            "prior_development_candidate_l1={:.6} development_mean_rmse={:.8} development_mean_delta_vs_independent={:.8} maximum_family_relative_regression={:.4} eligible_for_promotion={}",
                            candidate.penalty,
                            candidate.development_mean_field_rmse,
                            candidate.development_mean_delta_vs_independent,
                            candidate.maximum_family_relative_regression,
                            candidate.eligible_for_promotion,
                        );
                    }
                    for morphology in &study.development {
                        println!(
                            "prior_development_morphology={} seed_first={} replicates={}",
                            morphology.morphology,
                            morphology.seed_first,
                            morphology.replicate_count,
                        );
                        for estimator in &morphology.estimators {
                            println!(
                                "prior_development_estimator={} morphology={} rmse_mean={:.8} rmse_p95={:.8} rmse_max={:.8} paired_delta_mean={:.8} paired_delta_p95={:.8} support_mean={:.4} support_min={:.4} native_rank_mean={:.2} full_rank_replicates={} kkt_residual_mean={:.3e} kkt_residual_max={:.3e} converged={}/{}",
                                estimator.estimator_id,
                                morphology.morphology,
                                estimator.mean_field_rmse,
                                estimator.p95_field_rmse,
                                estimator.maximum_field_rmse,
                                estimator.paired_rmse_delta_mean_vs_independent,
                                estimator.paired_rmse_delta_p95_vs_independent,
                                estimator.mean_support_ratio,
                                estimator.minimum_support_ratio,
                                estimator.mean_native_data_rank,
                                estimator.full_rank_replicates,
                                estimator.mean_projected_gradient_residual,
                                estimator.maximum_projected_gradient_residual,
                                estimator.converged_replicates,
                                estimator.replicate_count,
                            );
                        }
                    }
                    for morphology in &study.final_test {
                        println!(
                            "prior_final_morphology={} seed_first={} replicates={} frozen_l1={:?}",
                            morphology.morphology,
                            morphology.seed_first,
                            morphology.replicate_count,
                            study.selected_l1_penalty,
                        );
                        for estimator in &morphology.estimators {
                            println!(
                                "prior_final_estimator={} morphology={} rmse_mean={:.8} rmse_p95={:.8} rmse_max={:.8} paired_delta_mean={:.8} paired_delta_p95={:.8} support_mean={:.4} support_min={:.4} native_rank_mean={:.2} full_rank_replicates={} kkt_residual_mean={:.3e} kkt_residual_max={:.3e} converged={}/{}",
                                estimator.estimator_id,
                                morphology.morphology,
                                estimator.mean_field_rmse,
                                estimator.p95_field_rmse,
                                estimator.maximum_field_rmse,
                                estimator.paired_rmse_delta_mean_vs_independent,
                                estimator.paired_rmse_delta_p95_vs_independent,
                                estimator.mean_support_ratio,
                                estimator.minimum_support_ratio,
                                estimator.mean_native_data_rank,
                                estimator.full_rank_replicates,
                                estimator.mean_projected_gradient_residual,
                                estimator.maximum_projected_gradient_residual,
                                estimator.converged_replicates,
                                estimator.replicate_count,
                            );
                        }
                    }
                    println!("prior_study_claim_class={}", study.claim_class);
                }
                Err(error) => {
                    eprintln!("field_prior_study=invalid: {error}");
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
