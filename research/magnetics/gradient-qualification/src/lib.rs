// SPDX-License-Identifier: MIT

//! Independent directional-derivative evidence contract for magnetic inverse design.
//!
//! This crate does not execute a differentiable kernel. It validates source-bound
//! autodiff, independent reference and central-finite-difference observations.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FloatingPointPrecision {
    F32,
    F64,
    Mixed,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReferenceMethod {
    Analytic,
    HighPrecision,
    ComplexStep,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KernelProvenance {
    pub package_id: String,
    pub package_version: String,
    pub source_revision: String,
    pub artifact_sha256: String,
    pub implementation_id: String,
    pub backend_id: String,
    pub backend_version: String,
    pub device_id: String,
    pub precision: FloatingPointPrecision,
    pub model_class: String,
    pub parameterization_id: String,
    pub parameter_dimension: usize,
    pub objective_id: String,
    pub objective_units: String,
    pub environment_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DerivativeReferenceIdentity {
    pub record_id: String,
    pub implementation_id: String,
    pub source_revision: String,
    pub artifact_sha256: String,
    pub input_sha256: String,
    pub direction_sha256: String,
    pub method: ReferenceMethod,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CentralDifferenceObservation {
    pub plus_observation_id: String,
    pub minus_observation_id: String,
    pub plus_input_sha256: String,
    pub minus_input_sha256: String,
    pub step_size: f64,
    pub plus_objective_value: f64,
    pub minus_objective_value: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DirectionalDerivativeTrial {
    pub trial_id: String,
    pub input_sha256: String,
    pub base_observation_id: String,
    /// Unit vector in the exact parameterization named by KernelProvenance.
    pub direction: Vec<f64>,
    pub direction_sha256: String,
    pub base_objective_value: f64,
    pub autodiff_directional_derivative: f64,
    pub reference_directional_derivative: f64,
    pub reference: DerivativeReferenceIdentity,
    pub forward_value_evaluator_id: String,
    pub forward_value_record_id: String,
    /// Strictly descending, preregistered step sizes.
    pub central_difference: Vec<CentralDifferenceObservation>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GradientQualificationPolicy {
    pub absolute_error_tolerance: f64,
    pub relative_error_tolerance: f64,
    pub step_stability_absolute_tolerance: f64,
    pub step_stability_relative_tolerance: f64,
    pub minimum_stable_step_count: usize,
    pub minimum_step_size: f64,
    pub maximum_step_size: f64,
    pub direction_norm_tolerance: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrialFailureReason {
    NoStableFiniteDifferencePlateau,
    AutodiffDisagreesWithReference,
    FiniteDifferenceDisagreesWithReference,
    AutodiffDisagreesWithFiniteDifference,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DirectionalDerivativeMetrics {
    pub trial_id: String,
    pub accepted: bool,
    pub failure_reasons: Vec<TrialFailureReason>,
    pub autodiff_absolute_error: f64,
    pub autodiff_normalized_error: f64,
    pub finite_difference_plateau_estimate: Option<f64>,
    pub finite_difference_absolute_error: Option<f64>,
    pub finite_difference_normalized_error: Option<f64>,
    pub autodiff_vs_finite_difference_normalized_error: Option<f64>,
    pub stable_step_start_index: Option<usize>,
    pub stable_step_end_index: Option<usize>,
    pub stable_step_count: usize,
    /// Every central-difference estimate is retained in input step order.
    pub central_difference_estimates: Vec<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GradientQualificationReport {
    pub kernel: KernelProvenance,
    pub policy: GradientQualificationPolicy,
    pub trial_count: usize,
    pub accepted_trial_count: usize,
    pub all_trials_accepted: bool,
    pub trials: Vec<DirectionalDerivativeMetrics>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QualificationError {
    EmptyIdentity,
    InvalidSha256 { field: &'static str },
    UnknownNumericalPrecision,
    InvalidParameterDimension,
    InvalidPolicy,
    EmptyTrialSet,
    InvalidTrialIdentity,
    DuplicateTrialId,
    InvalidTrialInputDigest,
    InvalidReferenceIdentity,
    ReferenceImplementationNotIndependent,
    ReferenceInputIdentityMismatch,
    DuplicateObservationId,
    PerturbationInputIdentityMismatch,
    DirectionDimensionMismatch,
    NonFiniteTrialValue,
    InvalidDirection,
    DirectionNotUnitLength,
    InsufficientStepSweep,
    InvalidStepSweep { index: usize },
    NonFiniteFiniteDifference { index: usize },
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn validate_digest(value: &str, field: &'static str) -> Result<(), QualificationError> {
    if valid_sha256(value) {
        Ok(())
    } else {
        Err(QualificationError::InvalidSha256 { field })
    }
}

fn validate_kernel(kernel: &KernelProvenance) -> Result<(), QualificationError> {
    let identities = [
        kernel.package_id.as_str(),
        kernel.package_version.as_str(),
        kernel.source_revision.as_str(),
        kernel.implementation_id.as_str(),
        kernel.backend_id.as_str(),
        kernel.backend_version.as_str(),
        kernel.device_id.as_str(),
        kernel.model_class.as_str(),
        kernel.parameterization_id.as_str(),
        kernel.objective_id.as_str(),
        kernel.objective_units.as_str(),
    ];
    if identities.into_iter().any(|value| !nonempty(value)) {
        return Err(QualificationError::EmptyIdentity);
    }
    if kernel.precision == FloatingPointPrecision::Unknown {
        return Err(QualificationError::UnknownNumericalPrecision);
    }
    if kernel.parameter_dimension == 0 {
        return Err(QualificationError::InvalidParameterDimension);
    }
    validate_digest(&kernel.artifact_sha256, "kernel.artifact_sha256")?;
    validate_digest(&kernel.environment_sha256, "kernel.environment_sha256")?;
    Ok(())
}

fn valid_nonnegative(value: f64) -> bool {
    value.is_finite() && value >= 0.0
}

fn validate_policy(policy: GradientQualificationPolicy) -> Result<(), QualificationError> {
    if !valid_nonnegative(policy.absolute_error_tolerance)
        || !valid_nonnegative(policy.relative_error_tolerance)
        || policy.absolute_error_tolerance + policy.relative_error_tolerance == 0.0
        || !valid_nonnegative(policy.step_stability_absolute_tolerance)
        || !valid_nonnegative(policy.step_stability_relative_tolerance)
        || policy.step_stability_absolute_tolerance
            + policy.step_stability_relative_tolerance
            == 0.0
        || !(policy.absolute_error_tolerance + policy.relative_error_tolerance).is_finite()
        || !(policy.step_stability_absolute_tolerance
            + policy.step_stability_relative_tolerance)
            .is_finite()
        || policy.minimum_stable_step_count < 2
        || !policy.minimum_step_size.is_finite()
        || !policy.maximum_step_size.is_finite()
        || policy.minimum_step_size <= 0.0
        || policy.maximum_step_size < policy.minimum_step_size
        || !valid_nonnegative(policy.direction_norm_tolerance)
        || policy.direction_norm_tolerance >= 0.1
    {
        return Err(QualificationError::InvalidPolicy);
    }
    Ok(())
}

fn validate_reference(
    kernel: &KernelProvenance,
    reference: &DerivativeReferenceIdentity,
) -> Result<(), QualificationError> {
    if !nonempty(&reference.record_id)
        || !nonempty(&reference.implementation_id)
        || !nonempty(&reference.source_revision)
    {
        return Err(QualificationError::InvalidReferenceIdentity);
    }
    if reference.implementation_id == kernel.implementation_id {
        return Err(QualificationError::ReferenceImplementationNotIndependent);
    }
    validate_digest(&reference.artifact_sha256, "reference.artifact_sha256")
}

fn validate_reference_binding(
    reference: &DerivativeReferenceIdentity,
    input_sha256: &str,
    direction_sha256: &str,
) -> Result<(), QualificationError> {
    validate_digest(&reference.input_sha256, "reference.input_sha256")?;
    validate_digest(&reference.direction_sha256, "reference.direction_sha256")?;
    if reference.input_sha256 != input_sha256
        || reference.direction_sha256 != direction_sha256
    {
        return Err(QualificationError::ReferenceInputIdentityMismatch);
    }
    Ok(())
}

fn normalized_error(
    left: f64,
    right: f64,
    absolute_tolerance: f64,
    relative_tolerance: f64,
) -> f64 {
    let scale = left.abs().max(right.abs());
    if scale == 0.0 {
        return 0.0;
    }
    let normalized_difference = (left / scale - right / scale).abs();
    let normalized_tolerance = absolute_tolerance / scale + relative_tolerance;
    if normalized_tolerance == 0.0 {
        if normalized_difference == 0.0 {
            0.0
        } else {
            f64::INFINITY
        }
    } else {
        normalized_difference / normalized_tolerance
    }
}

fn adjacent_steps_are_stable(
    left: f64,
    right: f64,
    policy: GradientQualificationPolicy,
) -> bool {
    normalized_error(
        left,
        right,
        policy.step_stability_absolute_tolerance,
        policy.step_stability_relative_tolerance,
    ) <= 1.0
}

fn entire_run_is_stable(
    estimates: &[f64],
    policy: GradientQualificationPolicy,
) -> bool {
    let center = median(estimates);
    estimates.iter().all(|estimate| {
        normalized_error(
            *estimate,
            center,
            policy.step_stability_absolute_tolerance,
            policy.step_stability_relative_tolerance,
        ) <= 1.0
    })
}

/// Return the longest contiguous run whose adjacent estimates are stable and
/// whose entire span also agrees with its median. Equal-length runs retain the
/// earliest (largest-step) interval. This prevents slow cumulative drift.
fn longest_stable_run(
    estimates: &[f64],
    policy: GradientQualificationPolicy,
) -> Option<(usize, usize)> {
    let mut best: Option<(usize, usize)> = None;
    let mut best_count = 0usize;

    for start in 0..estimates.len() {
        for end in start..estimates.len() {
            if end > start && !adjacent_steps_are_stable(estimates[end - 1], estimates[end], policy) {
                break;
            }

            let count = end - start + 1;
            if count < policy.minimum_stable_step_count {
                continue;
            }

            if entire_run_is_stable(&estimates[start..=end], policy) && count > best_count {
                best = Some((start, end));
                best_count = count;
            }
        }
    }

    best
}

fn median(values: &[f64]) -> f64 {
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let middle = sorted.len() / 2;
    if !sorted.len().is_multiple_of(2) {
        sorted[middle]
    } else {
        sorted[middle - 1] / 2.0 + sorted[middle] / 2.0
    }
}

fn validate_trial(
    kernel: &KernelProvenance,
    trial: &DirectionalDerivativeTrial,
    policy: GradientQualificationPolicy,
) -> Result<Vec<f64>, QualificationError> {
    if !nonempty(&trial.trial_id)
        || !nonempty(&trial.base_observation_id)
        || !nonempty(&trial.forward_value_evaluator_id)
        || !nonempty(&trial.forward_value_record_id)
    {
        return Err(QualificationError::InvalidTrialIdentity);
    }
    validate_digest(&trial.input_sha256, "trial.input_sha256")?;
    validate_digest(&trial.direction_sha256, "trial.direction_sha256")?;
    validate_reference(kernel, &trial.reference)?;
    validate_reference_binding(
        &trial.reference,
        &trial.input_sha256,
        &trial.direction_sha256,
    )?;

    if trial.direction.len() != kernel.parameter_dimension {
        return Err(QualificationError::DirectionDimensionMismatch);
    }
    if trial.direction.iter().any(|value| !value.is_finite())
        || !trial.base_objective_value.is_finite()
        || !trial.autodiff_directional_derivative.is_finite()
        || !trial.reference_directional_derivative.is_finite()
    {
        return Err(QualificationError::NonFiniteTrialValue);
    }

    let norm_squared = trial.direction.iter().map(|value| value * value).sum::<f64>();
    if !norm_squared.is_finite() || norm_squared <= 0.0 {
        return Err(QualificationError::InvalidDirection);
    }
    let norm_error = (norm_squared.sqrt() - 1.0).abs();
    if norm_error > policy.direction_norm_tolerance {
        return Err(QualificationError::DirectionNotUnitLength);
    }

    if trial.central_difference.len() < policy.minimum_stable_step_count {
        return Err(QualificationError::InsufficientStepSweep);
    }

    let mut estimates = Vec::with_capacity(trial.central_difference.len());
    let mut observation_ids = std::collections::HashSet::with_capacity(
        trial.central_difference.len() * 2 + 1,
    );
    let mut perturbation_input_digests =
        std::collections::HashSet::with_capacity(trial.central_difference.len() * 2);
    if !observation_ids.insert(trial.base_observation_id.as_str()) {
        return Err(QualificationError::DuplicateObservationId);
    }
    let mut previous_step = f64::INFINITY;
    for (index, observation) in trial.central_difference.iter().enumerate() {
        if !observation.step_size.is_finite()
            || observation.step_size < policy.minimum_step_size
            || observation.step_size > policy.maximum_step_size
            || observation.step_size >= previous_step
        {
            return Err(QualificationError::InvalidStepSweep { index });
        }
        if !nonempty(&observation.plus_observation_id)
            || !nonempty(&observation.minus_observation_id)
            || !observation_ids.insert(observation.plus_observation_id.as_str())
            || !observation_ids.insert(observation.minus_observation_id.as_str())
        {
            return Err(QualificationError::DuplicateObservationId);
        }
        validate_digest(
            &observation.plus_input_sha256,
            "central_difference.plus_input_sha256",
        )?;
        validate_digest(
            &observation.minus_input_sha256,
            "central_difference.minus_input_sha256",
        )?;
        if observation.plus_input_sha256 == trial.input_sha256
            || observation.minus_input_sha256 == trial.input_sha256
            || observation.plus_input_sha256 == observation.minus_input_sha256
            || !perturbation_input_digests.insert(observation.plus_input_sha256.as_str())
            || !perturbation_input_digests.insert(observation.minus_input_sha256.as_str())
        {
            return Err(QualificationError::PerturbationInputIdentityMismatch);
        }
        if !observation.plus_objective_value.is_finite()
            || !observation.minus_objective_value.is_finite()
        {
            return Err(QualificationError::NonFiniteTrialValue);
        }
        let estimate = (observation.plus_objective_value - observation.minus_objective_value)
            / (2.0 * observation.step_size);
        if !estimate.is_finite() {
            return Err(QualificationError::NonFiniteFiniteDifference { index });
        }
        estimates.push(estimate);
        previous_step = observation.step_size;
    }
    Ok(estimates)
}

fn evaluate_trial(
    trial: &DirectionalDerivativeTrial,
    estimates: &[f64],
    policy: GradientQualificationPolicy,
) -> DirectionalDerivativeMetrics {
    let autodiff_absolute_error =
        (trial.autodiff_directional_derivative - trial.reference_directional_derivative).abs();
    let autodiff_normalized_error = normalized_error(
        trial.autodiff_directional_derivative,
        trial.reference_directional_derivative,
        policy.absolute_error_tolerance,
        policy.relative_error_tolerance,
    );

    let stable_run = longest_stable_run(estimates, policy);
    let (plateau, start, end, stable_count) = if let Some((start, end)) = stable_run {
        (
            Some(median(&estimates[start..=end])),
            Some(start),
            Some(end),
            end - start + 1,
        )
    } else {
        (None, None, None, 0)
    };

    let finite_difference_absolute_error =
        plateau.map(|value| (value - trial.reference_directional_derivative).abs());
    let finite_difference_normalized_error = plateau.map(|value| {
        normalized_error(
            value,
            trial.reference_directional_derivative,
            policy.absolute_error_tolerance,
            policy.relative_error_tolerance,
        )
    });
    let autodiff_vs_finite_difference_normalized_error = plateau.map(|value| {
        normalized_error(
            trial.autodiff_directional_derivative,
            value,
            policy.absolute_error_tolerance,
            policy.relative_error_tolerance,
        )
    });

    let mut failure_reasons = Vec::new();
    if plateau.is_none() {
        failure_reasons.push(TrialFailureReason::NoStableFiniteDifferencePlateau);
    }
    if autodiff_normalized_error > 1.0 {
        failure_reasons.push(TrialFailureReason::AutodiffDisagreesWithReference);
    }
    if finite_difference_normalized_error.is_some_and(|error| error > 1.0) {
        failure_reasons.push(TrialFailureReason::FiniteDifferenceDisagreesWithReference);
    }
    if autodiff_vs_finite_difference_normalized_error.is_some_and(|error| error > 1.0) {
        failure_reasons.push(TrialFailureReason::AutodiffDisagreesWithFiniteDifference);
    }

    DirectionalDerivativeMetrics {
        trial_id: trial.trial_id.clone(),
        accepted: failure_reasons.is_empty(),
        failure_reasons,
        autodiff_absolute_error,
        autodiff_normalized_error,
        finite_difference_plateau_estimate: plateau,
        finite_difference_absolute_error,
        finite_difference_normalized_error,
        autodiff_vs_finite_difference_normalized_error,
        stable_step_start_index: start,
        stable_step_end_index: end,
        stable_step_count: stable_count,
        central_difference_estimates: estimates.to_vec(),
    }
}

/// Qualify a finite set of directional-derivative trials.
///
/// Structural evidence defects return an error. Numerically disagreeing but
/// otherwise well-formed trials remain in the report with accepted=false.
pub fn qualify_directional_derivatives(
    kernel: &KernelProvenance,
    trials: &[DirectionalDerivativeTrial],
    policy: GradientQualificationPolicy,
) -> Result<GradientQualificationReport, QualificationError> {
    validate_kernel(kernel)?;
    validate_policy(policy)?;
    if trials.is_empty() {
        return Err(QualificationError::EmptyTrialSet);
    }

    let mut seen = std::collections::HashSet::with_capacity(trials.len());
    let mut metrics = Vec::with_capacity(trials.len());
    for trial in trials {
        if !seen.insert(trial.trial_id.as_str()) {
            return Err(QualificationError::DuplicateTrialId);
        }
        let estimates = validate_trial(kernel, trial, policy)?;
        metrics.push(evaluate_trial(trial, &estimates, policy));
    }

    let accepted_trial_count = metrics.iter().filter(|trial| trial.accepted).count();
    Ok(GradientQualificationReport {
        kernel: kernel.clone(),
        policy,
        trial_count: metrics.len(),
        accepted_trial_count,
        all_trials_accepted: accepted_trial_count == metrics.len(),
        trials: metrics,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const DIGEST_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const DIGEST_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const DIGEST_C: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
    const DIGEST_D: &str = "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";

    fn kernel() -> KernelProvenance {
        KernelProvenance {
            package_id: "magpylib-material-response".to_owned(),
            package_version: "0.15.0".to_owned(),
            source_revision: "492b37459a47f4220d7e44380668c2d9ab720784".to_owned(),
            artifact_sha256: DIGEST_A.to_owned(),
            implementation_id: "jax-face-charge-autodiff".to_owned(),
            backend_id: "jax".to_owned(),
            backend_version: "pinned-test-version".to_owned(),
            device_id: "cpu-f64".to_owned(),
            precision: FloatingPointPrecision::F64,
            model_class: "face-charge-linear-isotropic".to_owned(),
            parameterization_id: "source-position-m".to_owned(),
            parameter_dimension: 2,
            objective_id: "sensor-plane-phase-margin".to_owned(),
            objective_units: "radian".to_owned(),
            environment_sha256: DIGEST_B.to_owned(),
        }
    }

    fn policy() -> GradientQualificationPolicy {
        GradientQualificationPolicy {
            absolute_error_tolerance: 1.0e-8,
            relative_error_tolerance: 1.0e-6,
            step_stability_absolute_tolerance: 1.0e-8,
            step_stability_relative_tolerance: 1.0e-6,
            minimum_stable_step_count: 3,
            minimum_step_size: 1.0e-5,
            maximum_step_size: 1.0e-1,
            direction_norm_tolerance: 1.0e-8,
        }
    }

    fn trial(trial_id: &str) -> DirectionalDerivativeTrial {
        let reference_value = 1.25;
        let center = 2.0;
        let central_difference = [1.0e-1, 1.0e-2, 1.0e-3, 1.0e-4]
            .into_iter()
            .enumerate()
            .map(|(index, step_size)| {
                let derivative = reference_value + 0.01 * step_size * step_size;
                let delta = derivative * step_size;
                let plus_digit = char::from_digit(index as u32 + 1, 16).expect("hex digit");
                let minus_digit = char::from_digit(index as u32 + 5, 16).expect("hex digit");
                CentralDifferenceObservation {
                    plus_observation_id: format!("plus-{trial_id}-{index}"),
                    minus_observation_id: format!("minus-{trial_id}-{index}"),
                    plus_input_sha256: plus_digit.to_string().repeat(64),
                    minus_input_sha256: minus_digit.to_string().repeat(64),
                    step_size,
                    plus_objective_value: center + delta,
                    minus_objective_value: center - delta,
                }
            })
            .collect();

        DirectionalDerivativeTrial {
            trial_id: trial_id.to_owned(),
            input_sha256: DIGEST_C.to_owned(),
            base_observation_id: format!("base-{trial_id}"),
            direction: vec![1.0, 0.0],
            direction_sha256: DIGEST_D.to_owned(),
            base_objective_value: center,
            autodiff_directional_derivative: reference_value,
            reference_directional_derivative: reference_value,
            reference: DerivativeReferenceIdentity {
                record_id: format!("analytic-reference-{trial_id}"),
                implementation_id: "independent-analytic-oracle".to_owned(),
                source_revision: "analytic-oracle-v1".to_owned(),
                artifact_sha256: DIGEST_B.to_owned(),
                input_sha256: DIGEST_C.to_owned(),
                direction_sha256: DIGEST_D.to_owned(),
                method: ReferenceMethod::Analytic,
            },
            forward_value_evaluator_id: "pinned-forward-kernel".to_owned(),
            forward_value_record_id: format!("forward-values-{trial_id}"),
            central_difference,
        }
    }

    #[test]
    fn known_answer_directional_derivative_is_accepted() {
        let result = qualify_directional_derivatives(&kernel(), &[trial("case-001")], policy())
            .expect("well-formed evidence");
        assert!(result.all_trials_accepted);
        assert_eq!(result.accepted_trial_count, 1);
        assert_eq!(result.trials[0].stable_step_count, 3);
        assert!(
            result.trials[0]
                .finite_difference_plateau_estimate
                .is_some()
        );
    }

    #[test]
    fn incorrect_autodiff_gradient_is_retained_as_a_failure() {
        let mut input = trial("case-wrong-gradient");
        input.autodiff_directional_derivative = 1.5;
        let result = qualify_directional_derivatives(&kernel(), &[input], policy())
            .expect("numerically wrong data is still structurally valid");
        assert!(!result.all_trials_accepted);
        assert!(
            result.trials[0]
                .failure_reasons
                .contains(&TrialFailureReason::AutodiffDisagreesWithReference)
        );
    }

    #[test]
    fn unstable_finite_difference_sweep_fails_closed() {
        let mut input = trial("case-no-plateau");
        for (observation, derivative) in input
            .central_difference
            .iter_mut()
            .zip([1.0_f64, 1.5, 2.0, 2.5])
        {
            let delta = derivative * observation.step_size;
            observation.plus_objective_value = 2.0 + delta;
            observation.minus_objective_value = 2.0 - delta;
        }
        let result = qualify_directional_derivatives(&kernel(), &[input], policy())
            .expect("step instability is a reported qualification failure");
        assert!(!result.trials[0].accepted);
        assert!(
            result.trials[0]
                .failure_reasons
                .contains(&TrialFailureReason::NoStableFiniteDifferencePlateau)
        );
        assert_eq!(result.trials[0].finite_difference_plateau_estimate, None);
    }

    #[test]
    fn finite_difference_disagreement_is_not_hidden_by_a_correct_autodiff_value() {
        let mut input = trial("case-wrong-forward-values");
        for observation in &mut input.central_difference {
            let delta = 1.4 * observation.step_size;
            observation.plus_objective_value = 2.0 + delta;
            observation.minus_objective_value = 2.0 - delta;
        }
        let result = qualify_directional_derivatives(&kernel(), &[input], policy())
            .expect("well-formed finite-difference records");
        assert!(!result.trials[0].accepted);
        assert!(
            result.trials[0]
                .failure_reasons
                .contains(&TrialFailureReason::FiniteDifferenceDisagreesWithReference)
        );
    }

    #[test]
    fn same_implementation_cannot_self_certify_its_gradient() {
        let mut input = trial("case-self-reference");
        input.reference.implementation_id = kernel().implementation_id;
        assert_eq!(
            qualify_directional_derivatives(&kernel(), &[input], policy())
                .expect_err("reference must be independent"),
            QualificationError::ReferenceImplementationNotIndependent
        );
    }

    #[test]
    fn parameter_direction_dimension_must_match_parameterization() {
        let mut input = trial("case-dimension");
        input.direction = vec![1.0];
        assert_eq!(
            qualify_directional_derivatives(&kernel(), &[input], policy())
                .expect_err("dimension mismatch"),
            QualificationError::DirectionDimensionMismatch
        );
    }

    #[test]
    fn direction_must_be_unit_length() {
        let mut input = trial("case-non-unit");
        input.direction = vec![0.8, 0.8];
        let mut bad_policy = policy();
        bad_policy.direction_norm_tolerance = 0.0;
        assert_eq!(
            qualify_directional_derivatives(&kernel(), &[input], bad_policy)
                .expect_err("non-unit direction"),
            QualificationError::DirectionNotUnitLength
        );
    }

    #[test]
    fn step_sizes_must_be_strictly_descending_and_inside_policy_range() {
        let mut input = trial("case-step-order");
        input.central_difference.swap(0, 1);
        assert_eq!(
            qualify_directional_derivatives(&kernel(), &[input], policy())
                .expect_err("unordered step sweep"),
            QualificationError::InvalidStepSweep { index: 1 }
        );
    }

    #[test]
    fn artifact_digests_are_required() {
        let mut invalid_kernel = kernel();
        invalid_kernel.artifact_sha256 = "not-a-sha256".to_owned();
        assert_eq!(
            qualify_directional_derivatives(&invalid_kernel, &[trial("case-digest")], policy())
                .expect_err("artifact digest required"),
            QualificationError::InvalidSha256 {
                field: "kernel.artifact_sha256"
            }
        );
    }

    #[test]
    fn duplicate_trials_are_rejected() {
        let input = trial("case-duplicate");
        assert_eq!(
            qualify_directional_derivatives(&kernel(), &[input.clone(), input], policy(),)
                .expect_err("duplicate trial IDs"),
            QualificationError::DuplicateTrialId
        );
    }

    #[test]
    fn cumulative_step_drift_is_not_accepted_as_a_plateau() {
        let mut input = trial("case-cumulative-drift");
        let mut strict_policy = policy();
        strict_policy.step_stability_absolute_tolerance = 0.0;
        strict_policy.step_stability_relative_tolerance = 0.1;
        strict_policy.minimum_stable_step_count = 4;

        for (observation, derivative) in input
            .central_difference
            .iter_mut()
            .zip([1.0_f64, 1.09, 1.18, 1.27])
        {
            let delta = derivative * observation.step_size;
            observation.plus_objective_value = 2.0 + delta;
            observation.minus_objective_value = 2.0 - delta;
        }

        let result = qualify_directional_derivatives(&kernel(), &[input], strict_policy)
            .expect("well-formed but drifting finite-difference evidence");
        assert!(!result.trials[0].accepted);
        assert!(result.trials[0].failure_reasons.contains(
            &TrialFailureReason::NoStableFiniteDifferencePlateau
        ));
    }

    #[test]
    fn independent_reference_must_bind_the_same_input_and_direction() {
        let mut input = trial("case-reference-binding");
        input.reference.input_sha256 = DIGEST_A.to_owned();
        assert_eq!(
            qualify_directional_derivatives(&kernel(), &[input], policy())
                .expect_err("reference input mismatch"),
            QualificationError::ReferenceInputIdentityMismatch
        );
    }

    #[test]
    fn every_finite_difference_perturbation_requires_a_unique_input_identity() {
        let mut input = trial("case-duplicate-perturbation");
        input.central_difference[1].plus_input_sha256 =
            input.central_difference[0].plus_input_sha256.clone();
        assert_eq!(
            qualify_directional_derivatives(&kernel(), &[input], policy())
                .expect_err("duplicate perturbed input digest"),
            QualificationError::PerturbationInputIdentityMismatch
        );
    }

    #[test]
    fn base_and_perturbation_observation_ids_must_be_unique() {
        let mut input = trial("case-duplicate-observation");
        input.central_difference[0].plus_observation_id = input.base_observation_id.clone();
        assert_eq!(
            qualify_directional_derivatives(&kernel(), &[input], policy())
                .expect_err("duplicate observation id"),
            QualificationError::DuplicateObservationId
        );
    }
}
