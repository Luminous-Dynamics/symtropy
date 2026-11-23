// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! NASA-compatible center-of-linearity (CoL) energy-change adjustment.
//!
//! This module ports the policy in the pinned CARA
//! `PeakOverlapPos.m::AdjustPOPCoL` routine. It keeps the physical energy
//! calculation in `pop_col_energy` and adds the source's fraction search:
//! 201 initial samples, a 10% default relative-energy-change ceiling, and
//! endpoint-aware bisection of local minima with an absolute fraction
//! tolerance of 5e-4.
//!
//! The local-minimum detector follows the pinned CARA `extrema.m` behavior,
//! including the middle element of flat plateaus and optional endpoint
//! extrema. The refinement loop follows `refine_bounded_extrema.m`'s
//! midpoint-bisection semantics. This remains a fixed-time numerical policy;
//! it is not a propagator or the full iterative PeakOverlapPos state machine.

use crate::conditional_velocity::Matrix3;
use crate::pop_col_energy::{
    PopColEnergyInput, PopColEnergyReport, calculate_pop_col_energy,
};
use crate::KernelError;

/// Pinned NASA CARA center-of-linearity adjustment configuration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PopColAdjustmentConfig {
    /// Maximum allowed relative change in specific orbital energy.
    ///
    /// CARA default: 0.10. Positive infinity explicitly disables the cap,
    /// matching the source's documented `MaxRelEnergyChange = Inf` mode.
    pub maximum_relative_energy_change: f64,
}

impl Default for PopColAdjustmentConfig {
    fn default() -> Self {
        Self {
            maximum_relative_energy_change: 0.10,
        }
    }
}

/// Why the CoL adjustment selected its returned fraction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopColAdjustmentStatus {
    /// The original CoL has nonnegative specific energy; CARA returns the
    /// proposed overlap point without applying the fractional energy test.
    OriginalCenterNonnegativeEnergy,
    /// The proposed overlap center already satisfies the energy-change cap.
    CandidateWithinEnergyLimit,
    /// The candidate exceeded the cap and the source-compatible fraction
    /// search selected a point on the original-to-candidate segment.
    EnergyLimitedCandidate,
}

/// Replay diagnostics for the source-compatible CoL fraction minimization.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PopColFractionSearchReport {
    /// True when the source's near-zero-grid-minimum condition triggered refinement.
    pub refinement_attempted: bool,
    /// True when the bisection loop terminated because no additional midpoint
    /// was required or no local minimum remained.
    pub refinement_converged: Option<bool>,
    /// Number of bisection rounds completed by the local-minimum refiner.
    pub bisection_rounds: usize,
    /// Number of local minima found in the final refined sample series.
    pub final_minimum_count: usize,
    /// Zero-based initial 201-point grid index selected by the first objective scan.
    pub initial_minimum_grid_index: Option<usize>,
    /// Number of source initial-grid samples; zero when the source bypasses the search.
    pub initial_sample_count: usize,
    /// Objective value at the final selected fraction, absent when the source bypasses the search.
    pub selected_objective: Option<f64>,
}

/// Input to the NASA CoL energy-change limiter.
///
/// The expansion-center state and conditional-velocity sensitivity are the
/// same values used by `CalcCoLEnergy`. The original CoL position and
/// proposed overlap position are distinct: CARA moves between the previous
/// CoL and the new product-Gaussian overlap point while evaluating velocity
/// relative to the current offset state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PopColAdjustmentInput {
    /// Current offset expansion-center position (r_u), kilometres.
    pub expansion_center_position_km: [f64; 3],
    /// Current offset expansion-center velocity (v_u), kilometres/second.
    pub expansion_center_velocity_km_per_s: [f64; 3],
    /// Previous center-of-linearity position (r_A), kilometres.
    pub original_col_position_km: [f64; 3],
    /// New product-Gaussian overlap position (r_B), kilometres.
    pub proposed_overlap_position_km: [f64; 3],
    /// Conditional-velocity sensitivity (eta = B A^{-1}), inverse seconds.
    pub beta_per_second: Matrix3,
    /// Central-body gravitational parameter, km³/s².
    pub gravitational_parameter_km3_per_s2: f64,
}

/// Result of one NASA-compatible center-of-linearity energy adjustment.
#[derive(Debug, Clone, PartialEq)]
pub struct PopColAdjustmentReport {
    /// Selected source-compatible outcome.
    pub status: PopColAdjustmentStatus,
    /// Initial CoL energy at fraction zero.
    pub original_energy: PopColEnergyReport,
    /// Unadjusted proposed-overlap energy at fraction one.
    pub proposed_energy: PopColEnergyReport,
    /// Energy at the returned adjusted CoL position.
    pub adjusted_energy: PopColEnergyReport,
    /// Returned position along (r(x)=r_A+x(r_B-r_A)), always in [0,1].
    pub adjustment_fraction: f64,
    /// Returned CoL position, kilometres.
    pub adjusted_col_position_km: [f64; 3],
    /// Relative energy change of the unadjusted candidate; absent only when
    /// the original specific energy is nonnegative.
    pub proposed_relative_energy_change: Option<f64>,
    /// Search trace sufficient to audit the fraction-selection path.
    pub search: PopColFractionSearchReport,
}

/// Apply the pinned CARA `AdjustPOPCoL` energy-change policy.
///
/// Source behavior:
/// 1. Compute the original and proposed CoL energies using the conditional
///    velocity relation.
/// 2. If original energy is nonnegative, return the proposed overlap center.
/// 3. If the candidate relative energy change is within the cap, return it.
/// 4. Otherwise scan 201 fractions on [0,1] and minimize
///    `(abs(E(x)/E_A - 1) - MaxRelEnergyChange)^2`.
/// 5. Invoke bounded-extrema bisection only when the first grid minimum is
///    among the first nine entries. Include endpoints only when it is in the
///    first two entries.
/// 6. If the selected fraction is zero, use the second initial grid point
///    (0.005) so that the returned point is not the original CoL itself.
///
/// The search's convergence flag is evidence about this scalar objective
/// minimization only. It does not establish orbital-model validity.
pub fn adjust_pop_col_energy_limited_center(
    input: PopColAdjustmentInput,
    config: PopColAdjustmentConfig,
) -> Result<PopColAdjustmentReport, KernelError> {
    validate_config(config)?;

    let original_energy = evaluate_fraction(input, 0.0)?;
    let proposed_energy = evaluate_fraction(input, 1.0)?;

    if original_energy.specific_orbital_energy_km2_per_s2 >= 0.0 {
        let search = empty_search();
        return Ok(PopColAdjustmentReport {
            status: PopColAdjustmentStatus::OriginalCenterNonnegativeEnergy,
            original_energy,
            proposed_energy,
            adjusted_energy: proposed_energy,
            adjustment_fraction: 1.0,
            adjusted_col_position_km: input.proposed_overlap_position_km,
            proposed_relative_energy_change: None,
            search,
        });
    }

    let relative_energy_change = relative_energy_change(
        original_energy.specific_orbital_energy_km2_per_s2,
        proposed_energy.specific_orbital_energy_km2_per_s2,
    )?;

    if relative_energy_change <= config.maximum_relative_energy_change {
        let search = empty_search();
        return Ok(PopColAdjustmentReport {
            status: PopColAdjustmentStatus::CandidateWithinEnergyLimit,
            original_energy,
            proposed_energy,
            adjusted_energy: proposed_energy,
            adjustment_fraction: 1.0,
            adjusted_col_position_km: input.proposed_overlap_position_km,
            proposed_relative_energy_change: Some(relative_energy_change),
            search,
        });
    }

    let mut fractions = (0..INITIAL_FRACTION_COUNT)
        .map(|index| index as f64 / (INITIAL_FRACTION_COUNT - 1) as f64)
        .collect::<Vec<_>>();
    let mut objective = Vec::with_capacity(INITIAL_FRACTION_COUNT);
    for fraction in &fractions {
        objective.push(energy_change_objective(
            evaluate_fraction(input, *fraction)?.specific_orbital_energy_km2_per_s2,
            original_energy.specific_orbital_energy_km2_per_s2,
            config.maximum_relative_energy_change,
        )?);
    }

    let initial_minimum_index = argmin_first(&objective)?;
    let mut selected_fraction = fractions[initial_minimum_index];
    let mut refinement_attempted = false;
    let mut refinement_converged = None;
    let mut bisection_rounds = 0;
    let mut final_minimum_count = 0;
    let mut selected_objective = objective[initial_minimum_index];

    // MATLAB source: Ninitcut = max(5, round(0.05 * 201)) = 10;
    // the 1-based condition imin < 10 means zero-based indices 0 through 8.
    if initial_minimum_index < SOURCE_INITIAL_MINIMUM_CUTOFF {
        refinement_attempted = true;
        let include_endpoints = initial_minimum_index < 2;
        let refined = refine_fraction_minima(
            input,
            original_energy.specific_orbital_energy_km2_per_s2,
            config.maximum_relative_energy_change,
            &mut fractions,
            &mut objective,
            include_endpoints,
        )?;
        refinement_converged = Some(refined.converged);
        bisection_rounds = refined.bisection_rounds;
        final_minimum_count = refined.minimum_indices.len();

        if let Some((index, value)) = refined
            .minimum_indices
            .iter()
            .map(|index| (*index, objective[*index]))
            .min_by(|left, right| left.1.total_cmp(&right.1))
        {
            selected_fraction = fractions[index];
            selected_objective = value;
        }
    }

    // CARA explicitly refuses to return the original CoL fraction.
    if selected_fraction <= 0.0 {
        selected_fraction = 1.0 / (INITIAL_FRACTION_COUNT - 1) as f64;
        selected_objective = energy_change_objective(
            evaluate_fraction(input, selected_fraction)?
                .specific_orbital_energy_km2_per_s2,
            original_energy.specific_orbital_energy_km2_per_s2,
            config.maximum_relative_energy_change,
        )?;
    }

    let adjusted_energy = evaluate_fraction(input, selected_fraction)?;
    let adjusted_col_position_km = adjusted_energy.candidate_col_position_km;

    Ok(PopColAdjustmentReport {
        status: PopColAdjustmentStatus::EnergyLimitedCandidate,
        original_energy,
        proposed_energy,
        adjusted_energy,
        adjustment_fraction: selected_fraction,
        adjusted_col_position_km,
        proposed_relative_energy_change: Some(relative_energy_change),
        search: PopColFractionSearchReport {
            refinement_attempted,
            refinement_converged,
            bisection_rounds,
            final_minimum_count,
            initial_minimum_grid_index: Some(initial_minimum_index),
            initial_sample_count: INITIAL_FRACTION_COUNT,
            selected_objective: Some(selected_objective),
        },
    })
}

const INITIAL_FRACTION_COUNT: usize = 201;
const SOURCE_INITIAL_MINIMUM_CUTOFF: usize = 9;
const SOURCE_MAX_BISECTION_ROUNDS: usize = 100;
const SOURCE_ABSOLUTE_FRACTION_TOLERANCE: f64 = 5.0e-4;

#[derive(Debug, Clone, PartialEq)]
struct RefinedMinima {
    minimum_indices: Vec<usize>,
    converged: bool,
    bisection_rounds: usize,
}

fn refine_fraction_minima(
    input: PopColAdjustmentInput,
    original_energy: f64,
    maximum_relative_energy_change: f64,
    fractions: &mut Vec<f64>,
    objective: &mut Vec<f64>,
    include_endpoints: bool,
) -> Result<RefinedMinima, KernelError> {
    let mut converged = false;
    let mut bisection_rounds = 0_usize;

    loop {
        let (_, minimum_indices) = extrema_indices(objective, include_endpoints)?;
        if minimum_indices.is_empty() {
            converged = true;
            break;
        }

        let count = fractions.len();
        let mut new_fractions = Vec::new();
        let mut delta_x_at_epsilon = false;

        for minimum_index in minimum_indices {
            // MATLAB is 1-based and clamps to [2, N-1].
            let middle = minimum_index.clamp(1, count - 2);
            let lower = middle - 1;
            let upper = middle + 1;

            let lower_gap = fractions[middle] - fractions[lower];
            if lower_gap <= ulp(fractions[middle]).max(ulp(fractions[lower])) {
                delta_x_at_epsilon = true;
            }
            if lower_gap > SOURCE_ABSOLUTE_FRACTION_TOLERANCE {
                new_fractions.push(midpoint(fractions[lower], fractions[middle])?);
            }

            let upper_gap = fractions[upper] - fractions[middle];
            if upper_gap <= ulp(fractions[middle]).max(ulp(fractions[upper])) {
                delta_x_at_epsilon = true;
            }
            if upper_gap > SOURCE_ABSOLUTE_FRACTION_TOLERANCE {
                new_fractions.push(midpoint(fractions[middle], fractions[upper])?);
            }
        }

        if new_fractions.is_empty() {
            converged = true;
            break;
        }
        if delta_x_at_epsilon {
            converged = false;
            break;
        }

        new_fractions.sort_by(f64::total_cmp);
        new_fractions.dedup_by(|left, right| left.to_bits() == right.to_bits());
        for fraction in new_fractions {
            if fractions
                .binary_search_by(|value| value.total_cmp(&fraction))
                .is_ok()
            {
                continue;
            }
            let value = energy_change_objective(
                evaluate_fraction(input, fraction)?.specific_orbital_energy_km2_per_s2,
                original_energy,
                maximum_relative_energy_change,
            )?;
            let index = fractions.partition_point(|existing| *existing < fraction);
            fractions.insert(index, fraction);
            objective.insert(index, value);
        }

        bisection_rounds += 1;
        // The CARA helper stops when nbisect > Nbisectmax.
        if bisection_rounds > SOURCE_MAX_BISECTION_ROUNDS {
            converged = false;
            break;
        }
    }

    let (_, minimum_indices) = extrema_indices(objective, include_endpoints)?;
    Ok(RefinedMinima {
        minimum_indices,
        converged,
        bisection_rounds,
    })
}

/// Reproduce the pinned CARA `extrema.m` local extrema selection.
///
/// Flat plateaus are represented by their middle element. Optional endpoint
/// handling follows the source's alternating endpoint assignment based on
/// the first and last interior extrema. The input is expected to be finite
/// and sorted in the same order as the fraction grid.
fn extrema_indices(
    values: &[f64],
    include_endpoints: bool,
) -> Result<(Vec<usize>, Vec<usize>), KernelError> {
    if values.len() < 2 || values.iter().any(|value| !value.is_finite()) {
        return Err(KernelError::InvalidPopColAdjustmentInput);
    }

    let mut nonzero_slope_indices = values
        .windows(2)
        .enumerate()
        .filter_map(|(index, pair)| (pair[1] - pair[0] != 0.0).then_some(index))
        .collect::<Vec<_>>();

    if nonzero_slope_indices.is_empty() {
        return Ok((Vec::new(), Vec::new()));
    }

    let original = nonzero_slope_indices.clone();
    for index in 1..original.len() {
        let gap = original[index] - original[index - 1];
        if gap != 1 {
            nonzero_slope_indices[index] = original[index] - gap / 2;
        }
    }
    nonzero_slope_indices.push(values.len() - 1);

    let collapsed = nonzero_slope_indices
        .iter()
        .map(|index| values[*index])
        .collect::<Vec<_>>();
    let rising = collapsed
        .windows(2)
        .map(|pair| pair[1] - pair[0] > 0.0)
        .collect::<Vec<_>>();

    let mut maxima = Vec::new();
    let mut minima = Vec::new();
    for index in 0..rising.len().saturating_sub(1) {
        match (rising[index], rising[index + 1]) {
            (true, false) => maxima.push(nonzero_slope_indices[index + 1]),
            (false, true) => minima.push(nonzero_slope_indices[index + 1]),
            _ => {}
        }
    }

    if include_endpoints {
        let last = values.len() - 1;
        if maxima.is_empty() && minima.is_empty() {
            if values[0] > values[last] {
                maxima.push(0);
                minima.push(last);
            } else if values[0] < values[last] {
                maxima.push(last);
                minima.push(0);
            }
            return Ok((maxima, minima));
        }

        if maxima.is_empty() {
            maxima.extend([0, last]);
        } else if minima.is_empty() {
            minima.extend([0, last]);
        } else {
            if maxima[0] < minima[0] {
                minima.insert(0, 0);
            } else {
                maxima.insert(0, 0);
            }
            if maxima[maxima.len() - 1] > minima[minima.len() - 1] {
                minima.push(last);
            } else {
                maxima.push(last);
            }
        }
    }

    Ok((maxima, minima))
}

fn evaluate_fraction(
    input: PopColAdjustmentInput,
    fraction: f64,
) -> Result<PopColEnergyReport, KernelError> {
    if !fraction.is_finite() || !(0.0..=1.0).contains(&fraction) {
        return Err(KernelError::InvalidPopColAdjustmentInput);
    }
    let candidate = interpolate(
        input.original_col_position_km,
        input.proposed_overlap_position_km,
        fraction,
    );
    calculate_pop_col_energy(PopColEnergyInput {
        expansion_center_position_km: input.expansion_center_position_km,
        expansion_center_velocity_km_per_s: input.expansion_center_velocity_km_per_s,
        candidate_col_position_km: candidate,
        beta_per_second: input.beta_per_second,
        gravitational_parameter_km3_per_s2: input.gravitational_parameter_km3_per_s2,
    })
}

fn energy_change_objective(
    energy: f64,
    original_energy: f64,
    maximum_relative_energy_change: f64,
) -> Result<f64, KernelError> {
    let relative = (energy / original_energy - 1.0).abs();
    let residual = relative - maximum_relative_energy_change;
    let objective = residual * residual;
    if !relative.is_finite() || !objective.is_finite() {
        return Err(KernelError::NumericalOverflow);
    }
    Ok(objective)
}

fn relative_energy_change(original_energy: f64, proposed_energy: f64) -> Result<f64, KernelError> {
    let result = (proposed_energy - original_energy).abs() / original_energy.abs();
    if result.is_finite() {
        Ok(result)
    } else {
        Err(KernelError::NumericalOverflow)
    }
}

fn validate_config(config: PopColAdjustmentConfig) -> Result<(), KernelError> {
    if config.maximum_relative_energy_change.is_nan()
        || config.maximum_relative_energy_change < 0.0
    {
        return Err(KernelError::InvalidPopColAdjustmentPolicy);
    }
    Ok(())
}

fn interpolate(start: [f64; 3], end: [f64; 3], fraction: f64) -> [f64; 3] {
    [
        start[0] + fraction * (end[0] - start[0]),
        start[1] + fraction * (end[1] - start[1]),
        start[2] + fraction * (end[2] - start[2]),
    ]
}

fn midpoint(first: f64, second: f64) -> Result<f64, KernelError> {
    let lower = first.min(second);
    let upper = first.max(second);
    let value = 0.5 * lower + 0.5 * upper;
    if !value.is_finite() {
        return Err(KernelError::NumericalOverflow);
    }
    if value <= lower || value >= upper {
        return Err(KernelError::PopColAdjustmentResolutionLimit);
    }
    Ok(value)
}

fn ulp(value: f64) -> f64 {
    let absolute = value.abs();
    if absolute == 0.0 {
        return f64::from_bits(1);
    }
    let bits = absolute.to_bits();
    if bits == f64::MAX.to_bits() {
        return f64::INFINITY;
    }
    f64::from_bits(bits + 1) - absolute
}

fn argmin_first(values: &[f64]) -> Result<usize, KernelError> {
    if values.is_empty() || values.iter().any(|value| !value.is_finite()) {
        return Err(KernelError::InvalidPopColAdjustmentInput);
    }
    let mut best = 0;
    for index in 1..values.len() {
        if values[index] < values[best] {
            best = index;
        }
    }
    Ok(best)
}

fn empty_search() -> PopColFractionSearchReport {
    PopColFractionSearchReport {
        refinement_attempted: false,
        refinement_converged: None,
        bisection_rounds: 0,
        final_minimum_count: 0,
        initial_minimum_grid_index: None,
        initial_sample_count: 0,
        selected_objective: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GM: f64 = 3.986_004_418e5;

    fn input(
        original_col_position_km: [f64; 3],
        proposed_overlap_position_km: [f64; 3],
        beta_per_second: Matrix3,
        expansion_center_velocity_km_per_s: [f64; 3],
        maximum_relative_energy_change: f64,
    ) -> (PopColAdjustmentInput, PopColAdjustmentConfig) {
        (
            PopColAdjustmentInput {
                expansion_center_position_km: [7000.0, 0.0, 0.0],
                expansion_center_velocity_km_per_s,
                original_col_position_km,
                proposed_overlap_position_km,
                beta_per_second,
                gravitational_parameter_km3_per_s2: GM,
            },
            PopColAdjustmentConfig {
                maximum_relative_energy_change,
            },
        )
    }

    fn zero_beta() -> Matrix3 {
        [[0.0; 3]; 3]
    }

    #[test]
    fn candidate_within_energy_limit_returns_fraction_one_without_search() {
        let (input, config) = input(
            [7000.0, 0.0, 0.0],
            [7000.0, 10.0, 0.0],
            zero_beta(),
            [0.0, (GM / 7000.0).sqrt(), 0.0],
            0.10,
        );
        let result = adjust_pop_col_energy_limited_center(input, config).unwrap();
        assert_eq!(result.status, PopColAdjustmentStatus::CandidateWithinEnergyLimit);
        assert_eq!(result.adjustment_fraction, 1.0);
        assert_eq!(result.adjusted_col_position_km, input.proposed_overlap_position_km);
        assert!(!result.search.refinement_attempted);
        assert_eq!(result.search.initial_sample_count, 0);
        assert_eq!(result.search.initial_minimum_grid_index, None);
        assert_eq!(result.search.selected_objective, None);
    }

    #[test]
    fn nonnegative_original_energy_preserves_cara_bypass_behavior() {
        let (input, config) = input(
            [7000.0, 0.0, 0.0],
            [7100.0, 0.0, 0.0],
            zero_beta(),
            [0.0, 20.0, 0.0],
            0.10,
        );
        let result = adjust_pop_col_energy_limited_center(input, config).unwrap();
        assert_eq!(result.status, PopColAdjustmentStatus::OriginalCenterNonnegativeEnergy);
        assert_eq!(result.adjustment_fraction, 1.0);
        assert_eq!(result.adjusted_col_position_km, input.proposed_overlap_position_km);
        assert_eq!(result.proposed_relative_energy_change, None);
    }

    #[test]
    fn energy_limiter_selects_an_interior_fraction_for_a_large_conditional_velocity_change() {
        let beta = [
            [0.05, 0.0, 0.0],
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0],
        ];
        let (input, config) = input(
            [7000.0, 0.0, 0.0],
            [7100.0, 0.0, 0.0],
            beta,
            [0.0, (GM / 7000.0).sqrt(), 0.0],
            0.10,
        );
        let result = adjust_pop_col_energy_limited_center(input, config).unwrap();
        assert_eq!(result.status, PopColAdjustmentStatus::EnergyLimitedCandidate);
        assert!(result.adjustment_fraction > 0.0 && result.adjustment_fraction < 1.0);
        assert!(result.search.initial_sample_count == 201);
        assert!(result.adjusted_energy.specific_orbital_energy_km2_per_s2 < 0.0);
    }

    #[test]
    fn source_endpoint_refinement_is_triggered_when_grid_minimum_is_in_first_two_points() {
        let beta = [
            [1.0, 0.0, 0.0],
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0],
        ];
        let (input, config) = input(
            [7000.0, 0.0, 0.0],
            [7100.0, 0.0, 0.0],
            beta,
            [0.0, (GM / 7000.0).sqrt(), 0.0],
            0.10,
        );
        let result = adjust_pop_col_energy_limited_center(input, config).unwrap();
        assert!(result.search.refinement_attempted);
        assert!(result.search.refinement_converged.is_some());
        assert!(result.adjustment_fraction > 0.0);
    }

    #[test]
    fn zero_selected_fraction_uses_the_second_initial_grid_point() {
        let (input, config) = input(
            [7000.0, 0.0, 0.0],
            [7000.0, 100.0, 0.0],
            zero_beta(),
            [0.0, (GM / 7000.0).sqrt(), 0.0],
            0.0,
        );
        let result = adjust_pop_col_energy_limited_center(input, config).unwrap();
        assert_eq!(result.adjustment_fraction, 0.005);
    }

    #[test]
    fn flat_plateau_extrema_uses_middle_element() {
        let values = [4.0, 3.0, 2.0, 2.0, 2.0, 3.0, 4.0];
        let (_, minima) = extrema_indices(&values, false).unwrap();
        assert_eq!(minima, vec![3]);
    }

    #[test]
    fn monotonic_objective_with_endpoints_returns_lower_endpoint_minimum() {
        let (_, minima) = extrema_indices(&[0.0, 1.0, 2.0, 3.0], true).unwrap();
        assert_eq!(minima, vec![0]);
    }

    #[test]
    fn horizontal_objective_has_no_extrema_even_with_endpoint_mode() {
        let (maxima, minima) = extrema_indices(&[1.0, 1.0, 1.0], true).unwrap();
        assert!(maxima.is_empty());
        assert!(minima.is_empty());
    }

    #[test]
    fn invalid_energy_change_policy_is_rejected_but_infinity_disables_the_cap() {
        assert_eq!(
            adjust_pop_col_energy_limited_center(
                input(
                    [7000.0, 0.0, 0.0],
                    [7000.0, 1.0, 0.0],
                    zero_beta(),
                    [0.0, (GM / 7000.0).sqrt(), 0.0],
                    -0.1,
                )
                .0,
                PopColAdjustmentConfig {
                    maximum_relative_energy_change: -0.1,
                },
            ),
            Err(KernelError::InvalidPopColAdjustmentPolicy)
        );
        let (input, config) = input(
            [7000.0, 0.0, 0.0],
            [7100.0, 0.0, 0.0],
            [[1.0, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0, 0.0]],
            [0.0, (GM / 7000.0).sqrt(), 0.0],
            f64::INFINITY,
        );
        let result = adjust_pop_col_energy_limited_center(
            input,
            PopColAdjustmentConfig {
                maximum_relative_energy_change: config.maximum_relative_energy_change,
            },
        )
        .unwrap();
        assert_eq!(result.adjustment_fraction, 1.0);
    }

    #[test]
    fn invalid_energy_evaluator_input_propagates_fail_closed() {
        let (mut input, config) = input(
            [7000.0, 0.0, 0.0],
            [7100.0, 0.0, 0.0],
            zero_beta(),
            [0.0, (GM / 7000.0).sqrt(), 0.0],
            0.10,
        );
        input.gravitational_parameter_km3_per_s2 = 0.0;
        assert_eq!(
            adjust_pop_col_energy_limited_center(input, config),
            Err(KernelError::InvalidGravitationalParameter)
        );
    }
}

