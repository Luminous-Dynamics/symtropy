//! Adversarial synthetic response families for tactile calibration policy stress.
//!
//! These scenarios make the inverse-response model wrong in controlled ways.
//! They are not physical sensor models or metrological uncertainty calculations.
//! Candidate selection sees only `Candidate` records; training
//! reference observations and held-out truth remain separate data planes.

#![forbid(unsafe_code)]

use super::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ResponseFamily {
    Nominal,
    CubicForce,
    ForceTemperatureHistoryInteraction,
    Saturation,
    ReferenceCommonModeBias,
    SharedReferenceCommonModeBias,
}

impl ResponseFamily {
    pub fn id(self) -> &'static str {
        match self {
            Self::Nominal => "nominal",
            Self::CubicForce => "cubic_force_misspecification",
            Self::ForceTemperatureHistoryInteraction => "force_temperature_history_interaction",
            Self::Saturation => "sensor_response_saturation",
            Self::ReferenceCommonModeBias => "reference_common_mode_bias",
            Self::SharedReferenceCommonModeBias => "shared_reference_common_mode_bias",
        }
    }

    fn sensor_response_shift(self, force_n: f64, temperature: f64, phase: f64) -> f64 {
        match self {
            Self::Nominal | Self::ReferenceCommonModeBias | Self::SharedReferenceCommonModeBias => {
                0.0
            }
            Self::CubicForce => 0.0009 * force_n.powi(3),
            Self::ForceTemperatureHistoryInteraction => 0.035 * force_n * temperature * phase,
            Self::Saturation => 0.0,
        }
    }

    fn reference_bias_n(self) -> f64 {
        match self {
            Self::ReferenceCommonModeBias | Self::SharedReferenceCommonModeBias => 0.25,
            _ => 0.0,
        }
    }

    fn independent_check_bias_n(self) -> f64 {
        match self {
            Self::SharedReferenceCommonModeBias => 0.25,
            _ => 0.0,
        }
    }

    fn has_shared_reference_common_cause(self) -> bool {
        self == Self::SharedReferenceCommonModeBias
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FamilyPolicySummary {
    pub policy_id: &'static str,
    pub source_reference_contacts: usize,
    pub target_reference_contacts: usize,
    pub rmse_n: MetricSummary,
    pub worst_absolute_error_n: MetricSummary,
    pub prediction_interval_coverage_95: MetricSummary,
    pub paired_rmse_delta_vs_full_grid_n: MetricSummary,
    pub paired_rmse_delta_bootstrap_95: BootstrapMeanInterval,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FamilyCampaignResult {
    pub family_id: &'static str,
    pub reference_integrity: ReferenceIntegrityReport,
    pub observability: ResponseObservabilityReport,
    /// This is known from the synthetic fault injection. Pairwise channel
    /// agreement cannot discover an undisclosed shared systematic cause.
    pub shared_reference_common_cause_injected: bool,
    pub policies: Vec<FamilyPolicySummary>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AdversarialCampaignReport {
    pub first_seed: u64,
    pub replicate_count: usize,
    pub source_candidate_pool_size: usize,
    pub target_candidate_pool_size: usize,
    pub heldout_case_count: usize,
    pub target_contact_budget: usize,
    pub families: Vec<FamilyCampaignResult>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct FamilyStimulus {
    profile: SensorProfile,
    family: ResponseFamily,
    force_n: f64,
    position: f64,
    temperature: f64,
    phase: f64,
    seed: u64,
    is_heldout: bool,
}

fn family_candidate(id: usize, stimulus: FamilyStimulus) -> (Candidate, f64) {
    let FamilyStimulus {
        profile,
        family,
        force_n,
        position,
        temperature,
        phase,
        seed,
        is_heldout,
    } = stimulus;
    let stratum = usize::from(temperature > 0.0) * 2 + usize::from(phase > 0.0);
    let noise = deterministic_noise(seed, id, 0.006);
    let mut raw = polynomial_response(profile, force_n, position, temperature, phase, noise)
        + family.sensor_response_shift(force_n, temperature, phase);

    if family == ResponseFamily::Saturation {
        // Deliberately clips the sensor response; no inverse model can recover
        // force information that is absent from the saturated readout.
        raw = raw.min(7.0);
    }

    let cost_ms = 500
        + (force_n * 5.0) as u64
        + if phase > 0.0 { 100 } else { 0 }
        + if position.abs() > 0.9 { 80 } else { 0 };

    let candidate = Candidate {
        id,
        force_setpoint_n: force_n,
        position,
        temperature,
        phase,
        stratum,
        cost_ms,
        features: design_features(raw, position, temperature, phase),
    };

    // The target/reference channel receives its own noise and a separately
    // injected common-mode fault. Held-out truth is not exposed here.
    let reference_noise_seed = if is_heldout {
        seed ^ 0x2468_ACE0_1357_9BDF
    } else {
        seed ^ 0xD1B5_4A32_D192_ED03
    };
    let reference_observation_n =
        force_n + deterministic_noise(reference_noise_seed, id, 0.003) + family.reference_bias_n();

    (candidate, reference_observation_n)
}

fn family_training_domain(
    seed: u64,
    profile: SensorProfile,
    family: ResponseFamily,
) -> SyntheticDomain {
    let mut candidates = Vec::with_capacity(72);
    let mut reference_force_observations_n = Vec::with_capacity(72);
    let mut independent_check_reference_force_observations_n = Vec::with_capacity(72);
    let mut id = 0;

    for force_index in 0..6 {
        let force = force_index as f64 * 2.0;
        for position in [-1.0, 0.0, 1.0] {
            for temperature in [-1.0, 1.0] {
                for phase in [-1.0, 1.0] {
                    let (candidate, reference) = family_candidate(
                        id,
                        FamilyStimulus {
                            profile,
                            family,
                            force_n: force,
                            position,
                            temperature,
                            phase,
                            seed,
                            is_heldout: false,
                        },
                    );
                    candidates.push(candidate);
                    reference_force_observations_n.push(reference);
                    independent_check_reference_force_observations_n.push(
                        force
                            + deterministic_noise(seed ^ 0xA076_1D64_78BD_642F, id, 0.003)
                            + family.independent_check_bias_n(),
                    );
                    id += 1;
                }
            }
        }
    }

    SyntheticDomain {
        candidates,
        reference_force_observations_n,
        independent_check_reference_force_observations_n,
        independent_reference_ancestry_attested: !family.has_shared_reference_common_cause(),
    }
}

fn family_heldout_domain(seed: u64, family: ResponseFamily) -> HeldoutDomain {
    let mut candidates = Vec::with_capacity(60);
    let mut evaluator_truth_force_n = Vec::with_capacity(60);
    let mut id = 0;

    for force in [1.0, 3.0, 5.0, 7.0, 9.0] {
        for position in [-0.7, 0.25, 0.8] {
            for temperature in [-0.6, 0.6] {
                for phase in [-0.5, 0.5] {
                    let (candidate, _heldout_reference_observation) = family_candidate(
                        id,
                        FamilyStimulus {
                            profile: TARGET_PROFILE,
                            family,
                            force_n: force,
                            position,
                            temperature,
                            phase,
                            seed: seed ^ 0x1357_9BDF_2468_ACE0,
                            is_heldout: true,
                        },
                    );
                    candidates.push(candidate);
                    evaluator_truth_force_n.push(force);
                    id += 1;
                }
            }
        }
    }

    HeldoutDomain {
        candidates,
        evaluator_truth_force_n,
    }
}

fn source_prior_diagnostic_only(source: &SyntheticDomain) -> Result<[f64; PARAMS], String> {
    // Deliberately bypasses qualification only for counterfactual diagnostics.
    let selected: Vec<_> = (0..source.candidates.len()).collect();
    let fit = fit_model(
        &source.candidates,
        &source.reference_force_observations_n,
        &selected,
        None,
        0.0,
    )?;
    Ok(fit.coefficients)
}

fn evaluate_source_transfer_diagnostic_only(
    source: &SyntheticDomain,
    heldout: &HeldoutDomain,
) -> Result<PolicyResult, String> {
    // Deliberately bypasses qualification only to quantify the consequence of
    // ignoring an invalid reference chain. It cannot qualify any measurement.
    let selected: Vec<_> = (0..source.candidates.len()).collect();
    let fit = fit_model(
        &source.candidates,
        &source.reference_force_observations_n,
        &selected,
        None,
        0.0,
    )?;
    evaluate(
        "transfer_only_source_model",
        &[],
        &[],
        source.candidates.len(),
        candidate_cost_sum(&source.candidates)?,
        &fit,
        heldout,
    )
}

fn evaluate_family_policy_arms(
    seed: u64,
    family: ResponseFamily,
) -> Result<Vec<PolicyResult>, String> {
    let target = family_training_domain(seed, TARGET_PROFILE, family);
    let source = family_training_domain(seed, SOURCE_PROFILE, family);
    let heldout = family_heldout_domain(seed, family);
    let candidates = &target.candidates;

    let full_grid: Vec<_> = (0..candidates.len()).collect();
    let uniform = select_uniform(candidates)?;
    let random = select_random_stratified(candidates, seed)?;
    let maximin = select_maximin(candidates)?;
    let d_optimal = select_d_optimal(candidates)?;

    let mut results = Vec::with_capacity(7);

    // Diagnostic-only counterfactual: these deliberately bypass the
    // reference-integrity gate so the benchmark can quantify the damage from
    // fitting against a biased or unqualified reference. These results never
    // become qualified measurements.
    let full_fit = fit_model(
        candidates,
        &target.reference_force_observations_n,
        &full_grid,
        None,
        0.0,
    )?;
    results.push(evaluate(
        "target_full_grid_reference",
        candidates,
        &full_grid,
        0,
        0,
        &full_fit,
        &heldout,
    )?);

    let uniform_fit = fit_model(
        candidates,
        &target.reference_force_observations_n,
        &uniform,
        None,
        0.0,
    )?;
    results.push(evaluate(
        "uniform_stratified",
        candidates,
        &uniform,
        0,
        0,
        &uniform_fit,
        &heldout,
    )?);

    let random_fit = fit_model(
        candidates,
        &target.reference_force_observations_n,
        &random,
        None,
        0.0,
    )?;
    results.push(evaluate(
        "random_stratified",
        candidates,
        &random,
        0,
        0,
        &random_fit,
        &heldout,
    )?);

    let maximin_fit = fit_model(
        candidates,
        &target.reference_force_observations_n,
        &maximin,
        None,
        0.0,
    )?;
    results.push(evaluate(
        "maximin_stratified",
        candidates,
        &maximin,
        0,
        0,
        &maximin_fit,
        &heldout,
    )?);

    let d_optimal_fit = fit_model(
        candidates,
        &target.reference_force_observations_n,
        &d_optimal,
        None,
        0.0,
    )?;
    results.push(evaluate(
        "d_optimal_stratified",
        candidates,
        &d_optimal,
        0,
        0,
        &d_optimal_fit,
        &heldout,
    )?);

    results.push(evaluate_source_transfer_diagnostic_only(&source, &heldout)?);

    let prior = source_prior_diagnostic_only(&source)?;
    let transfer_fit = fit_model(
        candidates,
        &target.reference_force_observations_n,
        &d_optimal,
        Some(&prior),
        0.25,
    )?;
    results.push(evaluate(
        "transfer_plus_d_optimal_target_adaptation",
        candidates,
        &d_optimal,
        source.candidates.len(),
        candidate_cost_sum(&source.candidates)?,
        &transfer_fit,
        &heldout,
    )?);

    Ok(results)
}

/// Run a paired multi-seed campaign across five known-answer response families.
/// The five families intentionally keep the original inverse-model basis fixed
/// while altering the data-generating response or reference channel.
pub fn run_adversarial_campaign(
    first_seed: u64,
    replicate_count: usize,
) -> Result<AdversarialCampaignReport, String> {
    if first_seed == 0 {
        return Err("adversarial campaign first seed must be nonzero".to_owned());
    }
    if replicate_count < 50 {
        return Err("adversarial campaign requires at least 50 paired seeds".to_owned());
    }
    first_seed
        .checked_add((replicate_count - 1) as u64)
        .ok_or_else(|| "adversarial campaign seed range overflow".to_owned())?;

    let families = [
        ResponseFamily::Nominal,
        ResponseFamily::CubicForce,
        ResponseFamily::ForceTemperatureHistoryInteraction,
        ResponseFamily::Saturation,
        ResponseFamily::ReferenceCommonModeBias,
        ResponseFamily::SharedReferenceCommonModeBias,
    ];

    let mut family_summaries = Vec::with_capacity(families.len());

    for family in families {
        let first_run = evaluate_family_policy_arms(first_seed, family)?;
        if first_run.len() != 7 {
            return Err(format!(
                "unexpected policy count for response family {}",
                family.id()
            ));
        }

        let mut policy_runs =
            vec![Vec::<PolicyResult>::with_capacity(replicate_count); first_run.len()];
        let mut paired_deltas = vec![Vec::<f64>::with_capacity(replicate_count); first_run.len()];

        let first_grid_rmse = first_run[0].rmse_n;
        for (index, result) in first_run.iter().enumerate() {
            policy_runs[index].push(result.clone());
            paired_deltas[index].push(result.rmse_n - first_grid_rmse);
        }

        for offset in 1..replicate_count {
            let seed = first_seed + offset as u64;
            let run = evaluate_family_policy_arms(seed, family)?;
            if run.len() != first_run.len() {
                return Err(format!("policy count drift in family {}", family.id()));
            }

            let grid_rmse = run[0].rmse_n;
            for (index, result) in run.into_iter().enumerate() {
                if result.policy_id != first_run[index].policy_id
                    || result.source_reference_contacts
                        != first_run[index].source_reference_contacts
                    || result.target_reference_contacts
                        != first_run[index].target_reference_contacts
                {
                    return Err(format!(
                        "policy identity/budget drift in family {}",
                        family.id()
                    ));
                }
                paired_deltas[index].push(result.rmse_n - grid_rmse);
                policy_runs[index].push(result);
            }
        }

        let mut policy_summaries = Vec::with_capacity(first_run.len());
        for (policy_index, (runs, paired)) in policy_runs.into_iter().zip(paired_deltas).enumerate()
        {
            let first = runs
                .first()
                .ok_or_else(|| "empty adversarial policy run".to_owned())?;
            let collect = |extract: fn(&PolicyResult) -> f64| {
                let values: Vec<_> = runs.iter().map(extract).collect();
                MetricSummary::from_samples(&values)
            };
            let mut bootstrap_seed =
                first_seed ^ (policy_index as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
            if bootstrap_seed == 0 {
                bootstrap_seed = 1;
            }
            policy_summaries.push(FamilyPolicySummary {
                policy_id: first.policy_id,
                source_reference_contacts: first.source_reference_contacts,
                target_reference_contacts: first.target_reference_contacts,
                rmse_n: collect(|result| result.rmse_n)?,
                worst_absolute_error_n: collect(|result| result.worst_absolute_error_n)?,
                prediction_interval_coverage_95: collect(|result| result.interval_coverage_95)?,
                paired_rmse_delta_vs_full_grid_n: MetricSummary::from_samples(&paired)?,
                paired_rmse_delta_bootstrap_95: bootstrap_mean_interval(
                    &paired,
                    bootstrap_seed,
                    2_000,
                )?,
            });
        }

        let target_reference_domain = family_training_domain(first_seed, TARGET_PROFILE, family);
        let reference_integrity = reference_integrity_check(
            &target_reference_domain.reference_force_observations_n,
            &target_reference_domain.independent_check_reference_force_observations_n,
            SYNTHETIC_REFERENCE_AGREEMENT_TOLERANCE_N,
            target_reference_domain.independent_reference_ancestry_attested,
        )?;
        let observability = response_observability_check(
            &target_reference_domain.candidates,
            SYNTHETIC_RESPONSE_RESOLUTION,
        )?;

        family_summaries.push(FamilyCampaignResult {
            family_id: family.id(),
            reference_integrity,
            observability,
            shared_reference_common_cause_injected: family.has_shared_reference_common_cause(),
            policies: policy_summaries,
        });
    }

    Ok(AdversarialCampaignReport {
        first_seed,
        replicate_count,
        source_candidate_pool_size: 72,
        target_candidate_pool_size: 72,
        heldout_case_count: 60,
        target_contact_budget: 16,
        families: family_summaries,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primary_reference_common_mode_bias_is_rejected_before_qualification() {
        let domain = family_training_domain(
            20261116,
            TARGET_PROFILE,
            ResponseFamily::ReferenceCommonModeBias,
        );
        let integrity = reference_integrity_check(
            &domain.reference_force_observations_n,
            &domain.independent_check_reference_force_observations_n,
            SYNTHETIC_REFERENCE_AGREEMENT_TOLERANCE_N,
            domain.independent_reference_ancestry_attested,
        )
        .expect("reference integrity report");
        assert!(!integrity.accepted);
        assert!(integrity.max_abs_disagreement_n > 0.2);

        // Diagnostic-only metrics still show the consequence of ignoring the
        // failed gate; these metrics must never qualify a measurement.
        let runs = evaluate_family_policy_arms(20261116, ResponseFamily::ReferenceCommonModeBias)
            .expect("reference-bias diagnostic family");
        let sparse = runs
            .iter()
            .find(|result| result.policy_id == "d_optimal_stratified")
            .expect("D-optimal diagnostic result");
        assert!(sparse.rmse_n > 0.1);
        assert!(sparse.interval_coverage_95 < 0.5);
    }

    #[test]
    fn failed_reference_integrity_blocks_qualified_model_fitting() {
        let biased = family_training_domain(
            20261116,
            TARGET_PROFILE,
            ResponseFamily::ReferenceCommonModeBias,
        );
        let selected: Vec<_> = (0..biased.candidates.len()).collect();
        assert!(fit_qualified_domain(&biased, &selected, None, 0.0).is_err());

        let shared = family_training_domain(
            20261116,
            TARGET_PROFILE,
            ResponseFamily::SharedReferenceCommonModeBias,
        );
        let selected: Vec<_> = (0..shared.candidates.len()).collect();
        assert!(fit_qualified_domain(&shared, &selected, None, 0.0).is_err());
    }

    #[test]
    fn shared_reference_bias_passes_pairwise_agreement_but_remains_unresolved() {
        let domain = family_training_domain(
            20261116,
            TARGET_PROFILE,
            ResponseFamily::SharedReferenceCommonModeBias,
        );
        let integrity = reference_integrity_check(
            &domain.reference_force_observations_n,
            &domain.independent_check_reference_force_observations_n,
            SYNTHETIC_REFERENCE_AGREEMENT_TOLERANCE_N,
            domain.independent_reference_ancestry_attested,
        )
        .expect("pairwise reference check");
        assert!(integrity.max_abs_disagreement_n <= integrity.tolerance_n);
        assert!(!integrity.accepted);
        assert!(!integrity.independent_reference_ancestry_attested);
        assert!(!integrity.agreement_proves_independence);
        assert!(ResponseFamily::SharedReferenceCommonModeBias.has_shared_reference_common_cause());
    }

    #[test]
    fn saturation_family_has_irrecoverable_high_force_error() {
        let nominal =
            evaluate_family_policy_arms(20261116, ResponseFamily::Nominal).expect("nominal family");
        let saturated = evaluate_family_policy_arms(20261116, ResponseFamily::Saturation)
            .expect("saturation family");
        assert!(saturated[0].worst_absolute_error_n > nominal[0].worst_absolute_error_n);
        let saturated_domain =
            family_training_domain(20261116, TARGET_PROFILE, ResponseFamily::Saturation);
        let observability = response_observability_check(
            &saturated_domain.candidates,
            SYNTHETIC_RESPONSE_RESOLUTION,
        )
        .expect("saturation observability diagnostic");
        assert!(!observability.accepted);
        assert!(observability.flatline_transition_count > 0);
    }

    #[test]
    fn adversarial_campaign_requires_fifty_seeds() {
        assert!(run_adversarial_campaign(20261116, 49).is_err());
        assert!(run_adversarial_campaign(0, 50).is_err());
    }
}
