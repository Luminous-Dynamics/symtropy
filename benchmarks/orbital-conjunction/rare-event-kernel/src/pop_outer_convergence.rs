// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Final outer temporal convergence gate for NASA CARA Pc3D_Hall.
//!
//! The module consumes an already-computed, ordered per-epoch POP ledger. It
//! reproduces the final source decision only; it does not execute POP, compute
//! Ncdot, refine an ephemeris, or qualify the physical/statistical model.
//!
//! Pinned source: nasa/CARA_Analysis_Tools commit
//! 43f0160f94d76a91805d86521c3e8fa0d09ecd68 (November 2026).

use std::collections::BTreeSet;

/// One ephemeris epoch and its final PeakOverlapPos outputs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PopOuterEpoch {
    /// Stable identity of this epoch within the assessment.
    pub epoch_id: u64,
    /// Epoch time in seconds in the assessment's declared time coordinate.
    pub time_seconds: f64,
    /// Whether PeakOverlapPos converged at this epoch.
    pub pop_converged: bool,
    /// NASA PeakOverlapPos failure code for this epoch.
    pub pop_failure_code: u32,
    /// Ncdot in inverse seconds when the POP solution converged.
    ///
    /// Absent means the epoch was not evaluated to a usable Ncdot result.
    /// Positive infinity is retained as a non-finite peak diagnostic and
    /// forces nonconvergence. NaN and negative values are invalid.
    pub ncdot_per_second: Option<f64>,
    /// Effective first-order surface-MD2 diagnostic when POP converged.
    ///
    /// This is NASA's MS2eff approximation, not a true squared norm; it may
    /// therefore be negative. The source requires a finite value but does not
    /// clamp it to zero.
    pub effective_surface_md2: Option<f64>,
}

/// Inputs controlling NASA's final outer temporal convergence decision.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PopOuterConvergenceConfig {
    /// Ncdot reduction factor defining the significant peak region.
    pub ncdot_reduction_factor: f64,
    /// Surface-MD2 cutoff used by the partial zero-rate exception.
    pub surface_md2_cutoff: f64,
    /// Whether either conjunction time limit was explicitly supplied.
    pub explicit_time_bounds_supplied: bool,
    /// Whether the ephemeris refinement loop was enabled.
    pub refine_ephemeris: bool,
    /// Number of ephemeris refinements performed.
    pub refinement_count: usize,
    /// Maximum refinement count; source success requires count strictly less.
    pub maximum_refinements: usize,
}

/// Machine-readable reason for the outer convergence result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopOuterConvergenceReason {
    /// Every POP epoch converged and the maximum Ncdot was finite.
    AllPopEpochsConverged,
    /// No POP epoch converged.
    NoPopEpochConverged,
    /// A POP failure code belongs to NASA's undefined-orbit class (>= 100).
    UndefinedEquinoctialOrbitFailure,
    /// The maximum Ncdot was non-finite.
    NonFiniteNcdotPeak,
    /// Partial zero-rate result has all surface-MD2 values strictly above cutoff.
    PartialZeroRateSurfaceClearance,
    /// Partial zero-rate result is accepted because a time bound was supplied.
    PartialZeroRateExplicitTimeBounds,
    /// Partial zero-rate result satisfies neither source exception.
    PartialZeroRateUnresolved,
    /// A high-Ncdot epoch is adjacent to a nonconverged POP epoch.
    HighRateRegionAdjacentToNonconvergedPop,
    /// Partial convergence has a finite peak and a complete high-rate region.
    PartialHighRateRegionComplete,
    /// The ephemeris refinement limit was exhausted.
    RefinementLimitExhausted,
}

/// Final outer convergence decision, separate from any probability value.
#[derive(Debug, Clone, PartialEq)]
pub struct PopOuterConvergenceReport {
    /// Whether the pinned NASA outer convergence gate accepts the assessment.
    pub converged: bool,
    /// Final reason, including an overriding refinement-limit failure.
    pub reason: PopOuterConvergenceReason,
    /// Number of converged POP epochs.
    pub converged_epoch_count: usize,
    /// Total number of epochs in the supplied ledger.
    pub total_epoch_count: usize,
    /// Maximum Ncdot over converged epochs, including positive infinity if present.
    pub maximum_ncdot_per_second: Option<f64>,
    /// Ncdot cutoff used by the partial-convergence neighborhood test.
    pub ncdot_cutoff_per_second: Option<f64>,
    /// Ordered-ledger indices at or above the Ncdot cutoff.
    pub high_rate_epoch_indices: Vec<usize>,
    /// Minimum available effective surface MD2 across converged epochs.
    pub minimum_surface_md2: Option<f64>,
    /// Whether the strict source refinement condition is satisfied.
    pub refinement_limit_satisfied: bool,
}

/// Invalid input that prevents a trustworthy outer convergence decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopOuterConvergenceError {
    /// No ephemeris epochs were supplied.
    EmptyEpochLedger,
    /// Epoch identifiers are duplicated.
    DuplicateEpochIdentity,
    /// Epoch times are non-finite or not strictly increasing.
    InvalidEpochTimeOrder,
    /// A converged POP epoch is missing Ncdot.
    MissingConvergedEpochNcdot,
    /// A converged POP epoch is missing its surface-MD2 diagnostic.
    MissingConvergedEpochSurfaceMd2,
    /// An unconverged POP epoch carries derived values that could be stale.
    UnconvergedEpochCarriesDerivedValues,
    /// Ncdot is NaN or negative.
    InvalidNcdotValue,
    /// Surface MD2 is non-finite.
    InvalidSurfaceMd2,
    /// Ncdot reduction factor is not finite and in (0, 1].
    InvalidNcdotReductionFactor,
    /// Surface-MD2 cutoff is negative or non-finite.
    InvalidSurfaceMd2Cutoff,
    /// Refinement mode has a zero maximum refinement count.
    InvalidRefinementConfiguration,
}

impl core::fmt::Display for PopOuterConvergenceError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let message = match self {
            Self::EmptyEpochLedger => "outer convergence requires at least one epoch",
            Self::DuplicateEpochIdentity => "outer convergence epoch identities must be unique",
            Self::InvalidEpochTimeOrder => "epoch times must be finite and strictly increasing",
            Self::MissingConvergedEpochNcdot => "a converged POP epoch is missing Ncdot",
            Self::MissingConvergedEpochSurfaceMd2 => {
                "a converged POP epoch is missing its surface MD2 diagnostic"
            }
            Self::UnconvergedEpochCarriesDerivedValues => {
                "an unconverged POP epoch carries stale derived values"
            }
            Self::InvalidNcdotValue => "Ncdot must be nonnegative and not NaN",
            Self::InvalidSurfaceMd2 => "surface MD2 must be finite",
            Self::InvalidNcdotReductionFactor => {
                "Ncdot reduction factor must be finite and lie in (0, 1]"
            }
            Self::InvalidSurfaceMd2Cutoff => {
                "surface MD2 cutoff must be finite and nonnegative"
            }
            Self::InvalidRefinementConfiguration => {
                "refinement mode requires a positive maximum refinement count"
            }
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for PopOuterConvergenceError {}

/// Apply the final outer Pc3D_Hall convergence gate to an ordered epoch ledger.
///
/// The source decision order is preserved:
/// - all POP epochs converged: require finite maximum Ncdot;
/// - no POP epochs converged: reject;
/// - partial convergence: reject non-finite peak or failure code >= 100;
/// - partial zero-rate: apply the strict surface-MD2 or explicit-time exception;
/// - partial positive-rate: reject high-rate epochs adjacent to nonconverged POP.
///
/// Finally, when ephemeris refinement was enabled, the strict source condition
/// refinement count < maximum refinements is applied. Failure means unresolved,
/// never a zero collision probability.
pub fn decide_pop_outer_convergence(
    epochs: &[PopOuterEpoch],
    config: PopOuterConvergenceConfig,
) -> Result<PopOuterConvergenceReport, PopOuterConvergenceError> {
    validate_inputs(epochs, config)?;

    let total_epoch_count = epochs.len();
    let converged_epoch_count = epochs.iter().filter(|epoch| epoch.pop_converged).count();
    let rates = epochs
        .iter()
        .filter(|epoch| epoch.pop_converged)
        .map(|epoch| epoch.ncdot_per_second.expect("validated converged Ncdot"))
        .collect::<Vec<_>>();
    let surfaces = epochs
        .iter()
        .filter(|epoch| epoch.pop_converged)
        .map(|epoch| {
            epoch
                .effective_surface_md2
                .expect("validated converged surface MD2")
        })
        .collect::<Vec<_>>();
    let maximum_ncdot_per_second = rates.iter().copied().reduce(f64::max);
    let minimum_surface_md2 = surfaces.iter().copied().reduce(f64::min);

    let mut ncdot_cutoff_per_second = None;
    let mut high_rate_epoch_indices = Vec::new();

    let (mut converged, mut reason) = if converged_epoch_count == total_epoch_count {
        match maximum_ncdot_per_second {
            Some(maximum) if maximum.is_finite() => {
                ncdot_cutoff_per_second = Some(maximum * config.ncdot_reduction_factor);
                (true, PopOuterConvergenceReason::AllPopEpochsConverged)
            }
            _ => (false, PopOuterConvergenceReason::NonFiniteNcdotPeak),
        }
    } else if converged_epoch_count == 0 {
        (false, PopOuterConvergenceReason::NoPopEpochConverged)
    } else if epochs.iter().any(|epoch| epoch.pop_failure_code >= 100) {
        (
            false,
            PopOuterConvergenceReason::UndefinedEquinoctialOrbitFailure,
        )
    } else {
        let maximum = maximum_ncdot_per_second
            .expect("partial convergence has at least one converged epoch");
        if !maximum.is_finite() {
            (false, PopOuterConvergenceReason::NonFiniteNcdotPeak)
        } else {
            ncdot_cutoff_per_second = Some(maximum * config.ncdot_reduction_factor);
            if maximum == 0.0 {
                if minimum_surface_md2
                    .is_some_and(|minimum| minimum > config.surface_md2_cutoff)
                {
                    (
                        true,
                        PopOuterConvergenceReason::PartialZeroRateSurfaceClearance,
                    )
                } else if config.explicit_time_bounds_supplied {
                    (
                        true,
                        PopOuterConvergenceReason::PartialZeroRateExplicitTimeBounds,
                    )
                } else {
                    (false, PopOuterConvergenceReason::PartialZeroRateUnresolved)
                }
            } else {
                let cutoff = ncdot_cutoff_per_second.expect("cutoff set above");
                high_rate_epoch_indices = epochs
                    .iter()
                    .enumerate()
                    .filter_map(|(index, epoch)| {
                        epoch
                            .ncdot_per_second
                            .filter(|rate| *rate >= cutoff)
                            .map(|_| index)
                    })
                    .collect();

                let adjacent_unconverged = high_rate_epoch_indices.iter().any(|index| {
                    (*index > 0 && !epochs[*index - 1].pop_converged)
                        || (*index + 1 < total_epoch_count
                            && !epochs[*index + 1].pop_converged)
                });
                if adjacent_unconverged {
                    (
                        false,
                        PopOuterConvergenceReason::HighRateRegionAdjacentToNonconvergedPop,
                    )
                } else {
                    (
                        true,
                        PopOuterConvergenceReason::PartialHighRateRegionComplete,
                    )
                }
            }
        }
    };

    let refinement_limit_satisfied =
        !config.refine_ephemeris || config.refinement_count < config.maximum_refinements;
    if !refinement_limit_satisfied {
        converged = false;
        reason = PopOuterConvergenceReason::RefinementLimitExhausted;
    }

    Ok(PopOuterConvergenceReport {
        converged,
        reason,
        converged_epoch_count,
        total_epoch_count,
        maximum_ncdot_per_second,
        ncdot_cutoff_per_second,
        high_rate_epoch_indices,
        minimum_surface_md2,
        refinement_limit_satisfied,
    })
}

fn validate_inputs(
    epochs: &[PopOuterEpoch],
    config: PopOuterConvergenceConfig,
) -> Result<(), PopOuterConvergenceError> {
    if epochs.is_empty() {
        return Err(PopOuterConvergenceError::EmptyEpochLedger);
    }
    if !config.ncdot_reduction_factor.is_finite()
        || config.ncdot_reduction_factor <= 0.0
        || config.ncdot_reduction_factor > 1.0
    {
        return Err(PopOuterConvergenceError::InvalidNcdotReductionFactor);
    }
    if !config.surface_md2_cutoff.is_finite() || config.surface_md2_cutoff < 0.0 {
        return Err(PopOuterConvergenceError::InvalidSurfaceMd2Cutoff);
    }
    if config.refine_ephemeris && config.maximum_refinements == 0 {
        return Err(PopOuterConvergenceError::InvalidRefinementConfiguration);
    }

    let mut identities = BTreeSet::new();
    let mut previous_time = None;
    for epoch in epochs {
        if !identities.insert(epoch.epoch_id) {
            return Err(PopOuterConvergenceError::DuplicateEpochIdentity);
        }
        if !epoch.time_seconds.is_finite()
            || previous_time.is_some_and(|previous| epoch.time_seconds <= previous)
        {
            return Err(PopOuterConvergenceError::InvalidEpochTimeOrder);
        }
        previous_time = Some(epoch.time_seconds);

        match (epoch.pop_converged, epoch.ncdot_per_second) {
            (true, None) => return Err(PopOuterConvergenceError::MissingConvergedEpochNcdot),
            (true, Some(rate)) if rate.is_nan() || rate < 0.0 => {
                return Err(PopOuterConvergenceError::InvalidNcdotValue);
            }
            (false, Some(_)) => {
                return Err(PopOuterConvergenceError::UnconvergedEpochCarriesDerivedValues);
            }
            _ => {}
        }
        match (epoch.pop_converged, epoch.effective_surface_md2) {
            (true, None) => {
                return Err(PopOuterConvergenceError::MissingConvergedEpochSurfaceMd2);
            }
            (true, Some(value)) if !value.is_finite() => {
                return Err(PopOuterConvergenceError::InvalidSurfaceMd2);
            }
            (false, Some(_)) => {
                return Err(PopOuterConvergenceError::UnconvergedEpochCarriesDerivedValues);
            }
            _ => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> PopOuterConvergenceConfig {
        PopOuterConvergenceConfig {
            ncdot_reduction_factor: 0.1,
            surface_md2_cutoff: 10.0,
            explicit_time_bounds_supplied: false,
            refine_ephemeris: false,
            refinement_count: 0,
            maximum_refinements: 250,
        }
    }

    fn epoch(
        id: u64,
        time: f64,
        converged: bool,
        failure: u32,
        ncdot: Option<f64>,
        surface_md2: Option<f64>,
    ) -> PopOuterEpoch {
        PopOuterEpoch {
            epoch_id: id,
            time_seconds: time,
            pop_converged: converged,
            pop_failure_code: failure,
            ncdot_per_second: ncdot,
            effective_surface_md2: surface_md2,
        }
    }

    #[test]
    fn all_pop_epochs_converged_with_finite_peak_is_accepted() {
        let result = decide_pop_outer_convergence(
            &[
                epoch(1, 0.0, true, 0, Some(2.0), Some(3.0)),
                epoch(2, 1.0, true, 0, Some(5.0), Some(4.0)),
                epoch(3, 2.0, true, 0, Some(1.0), Some(5.0)),
            ],
            config(),
        )
        .unwrap();

        assert!(result.converged);
        assert_eq!(result.reason, PopOuterConvergenceReason::AllPopEpochsConverged);
        assert_eq!(result.maximum_ncdot_per_second, Some(5.0));
        assert_eq!(result.ncdot_cutoff_per_second, Some(0.5));
    }

    #[test]
    fn no_pop_epoch_converged_is_unqualified() {
        let result = decide_pop_outer_convergence(
            &[
                epoch(1, 0.0, false, 1, None, None),
                epoch(2, 1.0, false, 1, None, None),
            ],
            config(),
        )
        .unwrap();

        assert!(!result.converged);
        assert_eq!(result.reason, PopOuterConvergenceReason::NoPopEpochConverged);
    }

    #[test]
    fn partial_high_rate_region_without_adjacent_failure_is_accepted() {
        let result = decide_pop_outer_convergence(
            &[
                epoch(1, 0.0, false, 1, None, None),
                epoch(2, 1.0, true, 0, Some(0.1), Some(2.0)),
                epoch(3, 2.0, true, 0, Some(10.0), Some(2.0)),
                epoch(4, 3.0, true, 0, Some(0.2), Some(2.0)),
                epoch(5, 4.0, false, 1, None, None),
            ],
            config(),
        )
        .unwrap();

        assert!(result.converged);
        assert_eq!(result.reason, PopOuterConvergenceReason::PartialHighRateRegionComplete);
        assert_eq!(result.high_rate_epoch_indices, [2]);
    }

    #[test]
    fn high_rate_epoch_adjacent_to_unconverged_epoch_is_rejected() {
        let result = decide_pop_outer_convergence(
            &[
                epoch(1, 0.0, false, 1, None, None),
                epoch(2, 1.0, true, 0, Some(10.0), Some(2.0)),
                epoch(3, 2.0, true, 0, Some(0.1), Some(2.0)),
            ],
            config(),
        )
        .unwrap();

        assert!(!result.converged);
        assert_eq!(
            result.reason,
            PopOuterConvergenceReason::HighRateRegionAdjacentToNonconvergedPop
        );
    }

    #[test]
    fn either_neighbor_of_a_high_rate_epoch_can_disqualify_partial_result() {
        let left = [
            epoch(1, 0.0, true, 0, Some(0.1), Some(2.0)),
            epoch(2, 1.0, true, 0, Some(10.0), Some(2.0)),
            epoch(3, 2.0, false, 1, None, None),
        ];
        let right = [
            epoch(1, 0.0, false, 1, None, None),
            epoch(2, 1.0, true, 0, Some(10.0), Some(2.0)),
            epoch(3, 2.0, true, 0, Some(0.1), Some(2.0)),
        ];

        for epochs in [&left[..], &right[..]] {
            let result = decide_pop_outer_convergence(epochs, config()).unwrap();
            assert!(!result.converged);
        }
    }

    #[test]
    fn partial_failure_code_one_hundred_or_more_is_disqualifying() {
        let result = decide_pop_outer_convergence(
            &[
                epoch(1, 0.0, true, 0, Some(1.0), Some(1.0)),
                epoch(2, 1.0, false, 100, None, None),
                epoch(3, 2.0, true, 0, Some(1.0), Some(1.0)),
            ],
            config(),
        )
        .unwrap();

        assert!(!result.converged);
        assert_eq!(
            result.reason,
            PopOuterConvergenceReason::UndefinedEquinoctialOrbitFailure
        );
    }

    #[test]
    fn partial_zero_rate_requires_strict_surface_md2_clearance() {
        let mut policy = config();
        policy.surface_md2_cutoff = 10.0;
        let accepted = decide_pop_outer_convergence(
            &[
                epoch(1, 0.0, true, 0, Some(0.0), Some(10.1)),
                epoch(2, 1.0, false, 1, None, None),
            ],
            policy,
        )
        .unwrap();
        let rejected = decide_pop_outer_convergence(
            &[
                epoch(1, 0.0, true, 0, Some(0.0), Some(10.0)),
                epoch(2, 1.0, false, 1, None, None),
            ],
            policy,
        )
        .unwrap();

        assert!(accepted.converged);
        assert_eq!(
            accepted.reason,
            PopOuterConvergenceReason::PartialZeroRateSurfaceClearance
        );
        assert!(!rejected.converged);
        assert_eq!(rejected.reason, PopOuterConvergenceReason::PartialZeroRateUnresolved);
    }

    #[test]
    fn partial_zero_rate_explicit_time_bounds_is_source_exception() {
        let mut policy = config();
        policy.explicit_time_bounds_supplied = true;
        let result = decide_pop_outer_convergence(
            &[
                epoch(1, 0.0, true, 0, Some(0.0), Some(1.0)),
                epoch(2, 1.0, false, 1, None, None),
            ],
            policy,
        )
        .unwrap();

        assert!(result.converged);
        assert_eq!(
            result.reason,
            PopOuterConvergenceReason::PartialZeroRateExplicitTimeBounds
        );
    }

    #[test]
    fn all_zero_ncdot_with_all_pop_converged_does_not_need_partial_exception() {
        let result = decide_pop_outer_convergence(
            &[
                epoch(1, 0.0, true, 0, Some(0.0), Some(0.0)),
                epoch(2, 1.0, true, 0, Some(0.0), Some(0.0)),
            ],
            config(),
        )
        .unwrap();

        assert!(result.converged);
        assert_eq!(result.reason, PopOuterConvergenceReason::AllPopEpochsConverged);
    }

    #[test]
    fn nonfinite_peak_is_rejected() {
        let result = decide_pop_outer_convergence(
            &[
                epoch(1, 0.0, true, 0, Some(f64::INFINITY), Some(1.0)),
                epoch(2, 1.0, true, 0, Some(1.0), Some(1.0)),
            ],
            config(),
        )
        .unwrap();

        assert!(!result.converged);
        assert_eq!(result.reason, PopOuterConvergenceReason::NonFiniteNcdotPeak);
    }

    #[test]
    fn refinement_limit_uses_strict_less_than_comparison() {
        let mut policy = config();
        policy.refine_ephemeris = true;
        policy.refinement_count = 250;
        policy.maximum_refinements = 250;
        let result = decide_pop_outer_convergence(
            &[epoch(1, 0.0, true, 0, Some(1.0), Some(1.0))],
            policy,
        )
        .unwrap();

        assert!(!result.converged);
        assert_eq!(result.reason, PopOuterConvergenceReason::RefinementLimitExhausted);
        assert!(!result.refinement_limit_satisfied);
    }

    #[test]
    fn refinement_one_before_limit_preserves_decision() {
        let mut policy = config();
        policy.refine_ephemeris = true;
        policy.refinement_count = 249;
        policy.maximum_refinements = 250;
        let result = decide_pop_outer_convergence(
            &[epoch(1, 0.0, true, 0, Some(1.0), Some(1.0))],
            policy,
        )
        .unwrap();

        assert!(result.converged);
        assert!(result.refinement_limit_satisfied);
    }

    #[test]
    fn missing_ncdot_for_converged_epoch_is_rejected() {
        assert_eq!(
            decide_pop_outer_convergence(
                &[epoch(1, 0.0, true, 0, None, Some(1.0))],
                config()
            ),
            Err(PopOuterConvergenceError::MissingConvergedEpochNcdot)
        );
    }

    #[test]
    fn duplicate_ids_and_nonincreasing_times_are_rejected() {
        assert_eq!(
            decide_pop_outer_convergence(
                &[
                    epoch(1, 0.0, true, 0, Some(1.0), Some(1.0)),
                    epoch(1, 1.0, true, 0, Some(1.0), Some(1.0)),
                ],
                config()
            ),
            Err(PopOuterConvergenceError::DuplicateEpochIdentity)
        );
        assert_eq!(
            decide_pop_outer_convergence(
                &[
                    epoch(1, 0.0, true, 0, Some(1.0), Some(1.0)),
                    epoch(2, 0.0, true, 0, Some(1.0), Some(1.0)),
                ],
                config()
            ),
            Err(PopOuterConvergenceError::InvalidEpochTimeOrder)
        );
    }

    #[test]
    fn negative_first_order_surface_md2_is_preserved_without_clamping() {
        let result = decide_pop_outer_convergence(
            &[
                epoch(1, 0.0, true, 0, Some(0.0), Some(-0.25)),
                epoch(2, 1.0, false, 1, None, None),
            ],
            config(),
        )
        .unwrap();

        assert!(!result.converged);
        assert_eq!(result.minimum_surface_md2, Some(-0.25));
        assert_eq!(result.reason, PopOuterConvergenceReason::PartialZeroRateUnresolved);
    }

    #[test]
    fn nonfinite_or_negative_surface_diagnostic_is_rejected() {
        assert_eq!(
            decide_pop_outer_convergence(
                &[epoch(1, 0.0, true, 0, Some(1.0), Some(f64::NAN))],
                config()
            ),
            Err(PopOuterConvergenceError::InvalidSurfaceMd2)
        );
        assert_eq!(
            decide_pop_outer_convergence(
                &[epoch(1, 0.0, true, 0, Some(-1.0), Some(1.0))],
                config()
            ),
            Err(PopOuterConvergenceError::InvalidNcdotValue)
        );
    }
}
