// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Mission-level conjunction risk conditioned on a shared finite latent state.
//!
//! The latent state can represent common orbit-determination bias, atmospheric
//! density error, tracking bias, catalog error or other shared uncertainty.
//! Each event has a conditional Pc in every state. The mission-level
//! dependence-free union bounds are computed within each state, then averaged
//! over the common latent-state posterior weights.
//!
//! A conditional-independence point result is optional and requires an explicit
//! assumption digest. This module does not prove conditional independence.

use std::collections::{BTreeMap, BTreeSet};

use crate::risk_semantics::{ConditionalCollisionProbability, MissionRiskScope};
use crate::space_time_partition::Rational;

/// One state in a finite discrete shared-uncertainty model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SharedLatentState {
    /// Stable identity of this latent orbit/environment state.
    pub state_id: u64,
    /// Posterior probability mass assigned to this state.
    pub posterior_weight: Rational,
}

/// Conditional Pc for one conjunction under one shared latent state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LatentConjunctionRiskCell {
    /// Common mission, target-measure and conditioning-set scope.
    pub scope: MissionRiskScope,
    /// Shared latent-state identity for this conditional probability.
    pub state_id: u64,
    /// Stable physical conjunction identity, independent of CDM message revision.
    pub conjunction_id: u64,
    /// CDM/orbit-assessment revision used to calculate this cell.
    pub cdm_revision_digest: [u8; 32],
    /// Pc conditional on this common latent state.
    pub conditional_pc: ConditionalCollisionProbability,
}

/// Dependence treatment within each shared latent state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConditionalDependenceMode {
    /// Dependence between conjunction events is unknown within each state.
    UnknownWithinLatentState,
    /// Events are declared mutually independent conditional on the shared
    /// latent state. They can still be dependent after marginalizing over it.
    IndependentGivenLatentState {
        /// Digest of the explicit conditional-independence assumption record.
        assumption_digest: [u8; 32],
    },
}

/// Mission-level risk envelope after integrating over shared latent uncertainty.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LatentMissionRiskReport {
    /// Mission, target-measure, conditioning-set and horizon identity.
    pub scope: MissionRiskScope,
    /// Dependence treatment applied within each latent state.
    pub dependence_mode: ConditionalDependenceMode,
    /// Dependence-free lower bound averaged over latent-state weights.
    pub dependence_free_lower_bound: Rational,
    /// Dependence-free upper bound averaged over latent-state weights.
    pub dependence_free_upper_bound: Rational,
    /// Reported lower bound under the selected conditional-dependence model.
    pub reported_lower_bound: Rational,
    /// Reported upper bound under the selected conditional-dependence model.
    pub reported_upper_bound: Rational,
    /// Conditional-independence point result, absent when dependence is unknown.
    pub conditional_independence_point: Option<Rational>,
    /// Number of shared latent states.
    pub state_count: usize,
    /// Number of unique physical conjunction events in every state.
    pub event_count: usize,
    /// Canonically ordered state identity and posterior weight inventory.
    pub state_inventory: Vec<(u64, Rational)>,
    /// Canonically ordered conjunction ID and CDM revision inventory.
    pub event_inventory: Vec<(u64, [u8; 32])>,
}

/// Errors that prevent valid shared-latent mission aggregation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LatentMissionRiskError {
    /// The latent-state list is empty.
    EmptyLatentStateSet,
    /// A latent state identity appears more than once.
    DuplicateLatentStateIdentity,
    /// A latent posterior weight lies outside [0, 1].
    InvalidLatentStateWeight,
    /// Latent posterior weights do not sum exactly to one.
    LatentWeightsNotNormalized,
    /// A risk cell belongs to another mission/model/horizon.
    MissionScopeMismatch,
    /// A cell references a latent state not in the declared state distribution.
    UnknownLatentState,
    /// A cell uses an all-zero CDM revision digest.
    InvalidEvidenceDigest,
    /// A physical conjunction changes CDM revision across latent states.
    EventRevisionMismatch,
    /// A conjunction cell appears more than once within one latent state.
    DuplicateLatentRiskCell,
    /// Latent states do not contain the same complete conjunction set.
    IncompleteLatentEventGrid,
    /// The product state/event domain size overflowed usize.
    DomainSizeOverflow,
    /// A cell uses a different target probability measure.
    ProbabilityMeasureMismatch,
    /// A cell uses a different common conditioning set.
    ConditioningSetMismatch,
    /// Exact rational aggregation overflowed u128.
    ArithmeticOverflow,
}

impl core::fmt::Display for LatentMissionRiskError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let message = match self {
            Self::EmptyLatentStateSet => "shared latent-state distribution must not be empty",
            Self::DuplicateLatentStateIdentity => "shared latent-state identities must be unique",
            Self::InvalidLatentStateWeight => "latent-state posterior weight must lie in [0, 1]",
            Self::LatentWeightsNotNormalized => {
                "latent-state posterior weights must sum exactly to one"
            }
            Self::MissionScopeMismatch => "latent risk cell belongs to another mission scope",
            Self::UnknownLatentState => "risk cell references an undeclared latent state",
            Self::InvalidEvidenceDigest => "risk cell CDM revision digest must not be all zero",
            Self::EventRevisionMismatch => {
                "a conjunction changes CDM revision across latent states"
            }
            Self::DuplicateLatentRiskCell => "conjunction is duplicated within a latent state",
            Self::IncompleteLatentEventGrid => {
                "latent states do not contain the same complete event set"
            }
            Self::DomainSizeOverflow => "latent state/event domain size overflowed usize",
            Self::ProbabilityMeasureMismatch => "risk cell uses another target probability measure",
            Self::ConditioningSetMismatch => "risk cell uses another common conditioning set",
            Self::ArithmeticOverflow => "exact latent mission-risk arithmetic overflowed u128",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for LatentMissionRiskError {}

/// Aggregate event-conditional Pc values over a shared finite latent-state model.
///
/// Every latent state must contain exactly one conditional Pc for every
/// conjunction in the mission event inventory. Posterior weights must sum
/// exactly to one. Every cell must share the mission scope's target measure and
/// common conditioning-set identity.
///
/// For latent state theta with event marginals p_i(theta), the dependence-free
/// state bounds are max_i p_i(theta) and min(1, sum_i p_i(theta)). The returned
/// mission envelope is the posterior-weighted sum of those state bounds.
///
/// Under an explicit conditional-independence declaration only, the point
/// result is the weighted sum of one minus the product of one-minus-Pc in each
/// latent state. This creates dependence between events after marginalizing
/// over theta, even when conditional independence holds within every state.
pub fn aggregate_latent_mission_collision_risk(
    scope: MissionRiskScope,
    states: &[SharedLatentState],
    cells: &[LatentConjunctionRiskCell],
    dependence_mode: ConditionalDependenceMode,
) -> Result<LatentMissionRiskReport, LatentMissionRiskError> {
    if states.is_empty() {
        return Err(LatentMissionRiskError::EmptyLatentStateSet);
    }
    if let ConditionalDependenceMode::IndependentGivenLatentState { assumption_digest } =
        dependence_mode
        && is_empty_digest(&assumption_digest)
    {
        return Err(LatentMissionRiskError::InvalidEvidenceDigest);
    }

    let mut state_weights = BTreeMap::new();
    let mut weight_sum = Rational::from_integer(0);
    for state in states {
        if state.posterior_weight > Rational::from_integer(1) {
            return Err(LatentMissionRiskError::InvalidLatentStateWeight);
        }
        if state_weights
            .insert(state.state_id, state.posterior_weight)
            .is_some()
        {
            return Err(LatentMissionRiskError::DuplicateLatentStateIdentity);
        }
        weight_sum = weight_sum
            .checked_add(state.posterior_weight)
            .map_err(|_| LatentMissionRiskError::ArithmeticOverflow)?;
    }
    if weight_sum != Rational::from_integer(1) {
        return Err(LatentMissionRiskError::LatentWeightsNotNormalized);
    }

    let mut cells_by_state: BTreeMap<u64, BTreeMap<u64, LatentConjunctionRiskCell>> = state_weights
        .keys()
        .map(|state_id| (*state_id, BTreeMap::new()))
        .collect();
    let mut revision_by_event = BTreeMap::new();

    for cell in cells {
        if cell.scope != scope {
            return Err(LatentMissionRiskError::MissionScopeMismatch);
        }
        if !state_weights.contains_key(&cell.state_id) {
            return Err(LatentMissionRiskError::UnknownLatentState);
        }
        if is_empty_digest(&cell.cdm_revision_digest) {
            return Err(LatentMissionRiskError::InvalidEvidenceDigest);
        }
        if cell.conditional_pc.probability_measure_digest() != scope.probability_measure_digest {
            return Err(LatentMissionRiskError::ProbabilityMeasureMismatch);
        }
        if cell.conditional_pc.conditioning_digest() != scope.conditioning_set_digest {
            return Err(LatentMissionRiskError::ConditioningSetMismatch);
        }

        if let Some(existing_revision) =
            revision_by_event.insert(cell.conjunction_id, cell.cdm_revision_digest)
            && existing_revision != cell.cdm_revision_digest
        {
            return Err(LatentMissionRiskError::EventRevisionMismatch);
        }

        let state_cells = cells_by_state
            .get_mut(&cell.state_id)
            .ok_or(LatentMissionRiskError::UnknownLatentState)?;
        if state_cells.insert(cell.conjunction_id, *cell).is_some() {
            return Err(LatentMissionRiskError::DuplicateLatentRiskCell);
        }
    }

    let reference_events = cells_by_state
        .values()
        .next()
        .map(|state_cells| state_cells.keys().copied().collect::<BTreeSet<_>>())
        .unwrap_or_default();
    for state_cells in cells_by_state.values() {
        if state_cells.keys().copied().collect::<BTreeSet<_>>() != reference_events {
            return Err(LatentMissionRiskError::IncompleteLatentEventGrid);
        }
    }

    let expected_cells = states
        .len()
        .checked_mul(reference_events.len())
        .ok_or(LatentMissionRiskError::DomainSizeOverflow)?;
    if expected_cells != cells.len() {
        return Err(LatentMissionRiskError::IncompleteLatentEventGrid);
    }

    let mut lower = Rational::from_integer(0);
    let mut upper = Rational::from_integer(0);
    let mut independence_point = match dependence_mode {
        ConditionalDependenceMode::UnknownWithinLatentState => None,
        ConditionalDependenceMode::IndependentGivenLatentState { .. } => {
            Some(Rational::from_integer(0))
        }
    };

    for (state_id, state_cells) in &cells_by_state {
        let weight = *state_weights
            .get(state_id)
            .ok_or(LatentMissionRiskError::UnknownLatentState)?;
        let mut state_lower = Rational::from_integer(0);
        let mut state_sum = Rational::from_integer(0);
        let mut state_survival = Rational::from_integer(1);

        for cell in state_cells.values() {
            let pc = cell.conditional_pc.value();
            if pc > Rational::from_integer(1) {
                return Err(LatentMissionRiskError::ArithmeticOverflow);
            }
            if pc > state_lower {
                state_lower = pc;
            }
            state_sum = state_sum
                .checked_add(pc)
                .map_err(|_| LatentMissionRiskError::ArithmeticOverflow)?;

            if independence_point.is_some() {
                let complement = Rational::from_integer(1)
                    .checked_sub(pc)
                    .map_err(|_| LatentMissionRiskError::ArithmeticOverflow)?;
                state_survival = state_survival
                    .checked_mul(complement)
                    .map_err(|_| LatentMissionRiskError::ArithmeticOverflow)?;
            }
        }

        let state_upper = if state_sum > Rational::from_integer(1) {
            Rational::from_integer(1)
        } else {
            state_sum
        };
        let weighted_lower = weight
            .checked_mul(state_lower)
            .map_err(|_| LatentMissionRiskError::ArithmeticOverflow)?;
        let weighted_upper = weight
            .checked_mul(state_upper)
            .map_err(|_| LatentMissionRiskError::ArithmeticOverflow)?;
        lower = lower
            .checked_add(weighted_lower)
            .map_err(|_| LatentMissionRiskError::ArithmeticOverflow)?;
        upper = upper
            .checked_add(weighted_upper)
            .map_err(|_| LatentMissionRiskError::ArithmeticOverflow)?;

        if let Some(current) = independence_point {
            let state_union = Rational::from_integer(1)
                .checked_sub(state_survival)
                .map_err(|_| LatentMissionRiskError::ArithmeticOverflow)?;
            let weighted_state_union = weight
                .checked_mul(state_union)
                .map_err(|_| LatentMissionRiskError::ArithmeticOverflow)?;
            independence_point = Some(
                current
                    .checked_add(weighted_state_union)
                    .map_err(|_| LatentMissionRiskError::ArithmeticOverflow)?,
            );
        }
    }

    let conditional_independence_point = match dependence_mode {
        ConditionalDependenceMode::UnknownWithinLatentState => None,
        ConditionalDependenceMode::IndependentGivenLatentState { .. } => independence_point,
    };
    let (reported_lower_bound, reported_upper_bound) = match conditional_independence_point {
        Some(point) => (point, point),
        None => (lower, upper),
    };

    let mut state_inventory = states
        .iter()
        .map(|state| (state.state_id, state.posterior_weight))
        .collect::<Vec<_>>();
    state_inventory.sort_by_key(|state| state.0);
    let event_inventory = revision_by_event.into_iter().collect();

    Ok(LatentMissionRiskReport {
        scope,
        dependence_mode,
        dependence_free_lower_bound: lower,
        dependence_free_upper_bound: upper,
        reported_lower_bound,
        reported_upper_bound,
        conditional_independence_point,
        state_count: states.len(),
        event_count: reference_events.len(),
        state_inventory,
        event_inventory,
    })
}

fn is_empty_digest(digest: &[u8; 32]) -> bool {
    digest.iter().all(|byte| *byte == 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scope() -> MissionRiskScope {
        MissionRiskScope::new([1; 32], [2; 32], [3; 32], [4; 32], 0, 86_400).unwrap()
    }

    fn rational(numerator: u128, denominator: u128) -> Rational {
        Rational::new(numerator, denominator).unwrap()
    }

    fn state(state_id: u64, numerator: u128, denominator: u128) -> SharedLatentState {
        SharedLatentState {
            state_id,
            posterior_weight: rational(numerator, denominator),
        }
    }

    fn cell(
        state_id: u64,
        conjunction_id: u64,
        numerator: u128,
        denominator: u128,
    ) -> LatentConjunctionRiskCell {
        LatentConjunctionRiskCell {
            scope: scope(),
            state_id,
            conjunction_id,
            cdm_revision_digest: [conjunction_id as u8; 32],
            conditional_pc: ConditionalCollisionProbability::new(
                rational(numerator, denominator),
                [3; 32],
                [4; 32],
            )
            .unwrap(),
        }
    }

    fn anticorrelated_cells() -> Vec<LatentConjunctionRiskCell> {
        vec![
            cell(1, 10, 9, 10),
            cell(1, 20, 1, 10),
            cell(2, 10, 1, 10),
            cell(2, 20, 9, 10),
        ]
    }

    #[test]
    fn unknown_within_state_dependence_returns_latent_averaged_bounds() {
        let report = aggregate_latent_mission_collision_risk(
            scope(),
            &[state(1, 1, 2), state(2, 1, 2)],
            &anticorrelated_cells(),
            ConditionalDependenceMode::UnknownWithinLatentState,
        )
        .unwrap();

        assert_eq!(report.dependence_free_lower_bound, rational(9, 10));
        assert_eq!(report.dependence_free_upper_bound, rational(1, 1));
        assert_eq!(report.reported_lower_bound, rational(9, 10));
        assert_eq!(report.reported_upper_bound, rational(1, 1));
        assert_eq!(report.conditional_independence_point, None);
    }

    #[test]
    fn conditional_independence_given_latent_state_produces_point_inside_envelope() {
        let report = aggregate_latent_mission_collision_risk(
            scope(),
            &[state(1, 1, 2), state(2, 1, 2)],
            &anticorrelated_cells(),
            ConditionalDependenceMode::IndependentGivenLatentState {
                assumption_digest: [9; 32],
            },
        )
        .unwrap();

        assert_eq!(
            report.conditional_independence_point,
            Some(rational(91, 100))
        );
        assert_eq!(report.reported_lower_bound, rational(91, 100));
        assert_eq!(report.reported_upper_bound, rational(91, 100));
        assert!(report.dependence_free_lower_bound <= rational(91, 100));
        assert!(report.dependence_free_upper_bound >= rational(91, 100));
    }

    #[test]
    fn shared_latent_uncertainty_defeats_global_independence_of_marginals() {
        let report = aggregate_latent_mission_collision_risk(
            scope(),
            &[state(1, 1, 2), state(2, 1, 2)],
            &anticorrelated_cells(),
            ConditionalDependenceMode::IndependentGivenLatentState {
                assumption_digest: [9; 32],
            },
        )
        .unwrap();

        // Each event has marginal Pc = 1/2. Multiplying those two marginal
        // survival probabilities would return 3/4, not the latent mixture.
        let naive_global_independence = Rational::from_integer(1)
            .checked_sub(rational(1, 2).checked_mul(rational(1, 2)).unwrap())
            .unwrap();
        assert_eq!(naive_global_independence, rational(3, 4));
        assert_eq!(report.reported_lower_bound, rational(91, 100));
        assert_ne!(report.reported_lower_bound, naive_global_independence);
    }

    #[test]
    fn posterior_weights_must_sum_exactly_to_one() {
        assert_eq!(
            aggregate_latent_mission_collision_risk(
                scope(),
                &[state(1, 1, 3), state(2, 1, 3)],
                &[],
                ConditionalDependenceMode::UnknownWithinLatentState,
            ),
            Err(LatentMissionRiskError::LatentWeightsNotNormalized)
        );
    }

    #[test]
    fn empty_latent_state_set_is_rejected() {
        assert_eq!(
            aggregate_latent_mission_collision_risk(
                scope(),
                &[],
                &[],
                ConditionalDependenceMode::UnknownWithinLatentState,
            ),
            Err(LatentMissionRiskError::EmptyLatentStateSet)
        );
    }

    #[test]
    fn duplicate_latent_state_identity_is_rejected() {
        assert_eq!(
            aggregate_latent_mission_collision_risk(
                scope(),
                &[state(1, 1, 2), state(1, 1, 2)],
                &[],
                ConditionalDependenceMode::UnknownWithinLatentState,
            ),
            Err(LatentMissionRiskError::DuplicateLatentStateIdentity)
        );
    }

    #[test]
    fn latent_state_weight_above_one_is_rejected() {
        assert_eq!(
            aggregate_latent_mission_collision_risk(
                scope(),
                &[state(1, 2, 1)],
                &[],
                ConditionalDependenceMode::UnknownWithinLatentState,
            ),
            Err(LatentMissionRiskError::InvalidLatentStateWeight)
        );
    }

    #[test]
    fn missing_event_cell_in_one_state_is_rejected() {
        let cells = vec![cell(1, 10, 1, 10), cell(1, 20, 1, 5), cell(2, 10, 1, 10)];
        assert_eq!(
            aggregate_latent_mission_collision_risk(
                scope(),
                &[state(1, 1, 2), state(2, 1, 2)],
                &cells,
                ConditionalDependenceMode::UnknownWithinLatentState,
            ),
            Err(LatentMissionRiskError::IncompleteLatentEventGrid)
        );
    }

    #[test]
    fn duplicate_state_event_cell_is_rejected() {
        let mut cells = anticorrelated_cells();
        let duplicate = cells[0];
        cells.push(duplicate);
        assert_eq!(
            aggregate_latent_mission_collision_risk(
                scope(),
                &[state(1, 1, 2), state(2, 1, 2)],
                &cells,
                ConditionalDependenceMode::UnknownWithinLatentState,
            ),
            Err(LatentMissionRiskError::DuplicateLatentRiskCell)
        );
    }

    #[test]
    fn cells_with_an_undeclared_state_are_rejected() {
        let mut cells = anticorrelated_cells();
        cells.push(cell(3, 10, 1, 10));
        assert_eq!(
            aggregate_latent_mission_collision_risk(
                scope(),
                &[state(1, 1, 2), state(2, 1, 2)],
                &cells,
                ConditionalDependenceMode::UnknownWithinLatentState,
            ),
            Err(LatentMissionRiskError::UnknownLatentState)
        );
    }

    #[test]
    fn cdms_must_be_the_same_revision_for_one_event_across_states() {
        let mut cells = anticorrelated_cells();
        cells[2].cdm_revision_digest = [99; 32];
        assert_eq!(
            aggregate_latent_mission_collision_risk(
                scope(),
                &[state(1, 1, 2), state(2, 1, 2)],
                &cells,
                ConditionalDependenceMode::UnknownWithinLatentState,
            ),
            Err(LatentMissionRiskError::EventRevisionMismatch)
        );
    }

    #[test]
    fn zero_independence_assumption_digest_is_rejected() {
        assert_eq!(
            aggregate_latent_mission_collision_risk(
                scope(),
                &[state(1, 1, 2), state(2, 1, 2)],
                &anticorrelated_cells(),
                ConditionalDependenceMode::IndependentGivenLatentState {
                    assumption_digest: [0; 32],
                },
            ),
            Err(LatentMissionRiskError::InvalidEvidenceDigest)
        );
    }

    #[test]
    fn input_permutation_does_not_change_latent_mission_result() {
        let mut states = vec![state(1, 1, 2), state(2, 1, 2)];
        let mut cells = anticorrelated_cells();
        let forward = aggregate_latent_mission_collision_risk(
            scope(),
            &states,
            &cells,
            ConditionalDependenceMode::UnknownWithinLatentState,
        )
        .unwrap();
        states.reverse();
        cells.reverse();
        let reverse = aggregate_latent_mission_collision_risk(
            scope(),
            &states,
            &cells,
            ConditionalDependenceMode::UnknownWithinLatentState,
        )
        .unwrap();
        assert_eq!(forward, reverse);
    }

    #[test]
    fn empty_event_set_has_zero_union_probability() {
        let report = aggregate_latent_mission_collision_risk(
            scope(),
            &[state(1, 1, 2), state(2, 1, 2)],
            &[],
            ConditionalDependenceMode::UnknownWithinLatentState,
        )
        .unwrap();

        assert_eq!(report.event_count, 0);
        assert_eq!(report.dependence_free_lower_bound, rational(0, 1));
        assert_eq!(report.dependence_free_upper_bound, rational(0, 1));
    }

    #[test]
    fn unknown_dependence_does_not_compute_an_unneeded_conditional_product() {
        let tiny = rational(1, u128::MAX);
        let cells = [cell(1, 1, 1, u128::MAX), cell(1, 2, 1, u128::MAX)];
        let report = aggregate_latent_mission_collision_risk(
            scope(),
            &[state(1, 1, 1)],
            &cells,
            ConditionalDependenceMode::UnknownWithinLatentState,
        )
        .unwrap();

        assert_eq!(report.dependence_free_lower_bound, tiny);
        assert_eq!(report.dependence_free_upper_bound, rational(2, u128::MAX));
        assert_eq!(report.conditional_independence_point, None);
    }

    #[test]
    fn conditional_product_overflow_is_reported_only_when_that_model_is_requested() {
        let cells = [cell(1, 1, 1, u128::MAX), cell(1, 2, 1, u128::MAX)];
        assert_eq!(
            aggregate_latent_mission_collision_risk(
                scope(),
                &[state(1, 1, 1)],
                &cells,
                ConditionalDependenceMode::IndependentGivenLatentState {
                    assumption_digest: [9; 32],
                },
            ),
            Err(LatentMissionRiskError::ArithmeticOverflow)
        );
    }

    #[test]
    fn exact_arithmetic_overflow_fails_closed() {
        let cells = [
            cell(1, 1, 1, u128::MAX),
            cell(1, 2, 1, u128::MAX),
            cell(2, 1, 1, u128::MAX),
            cell(2, 2, 1, u128::MAX),
        ];
        assert_eq!(
            aggregate_latent_mission_collision_risk(
                scope(),
                &[state(1, 1, 2), state(2, 1, 2)],
                &cells,
                ConditionalDependenceMode::UnknownWithinLatentState,
            ),
            Err(LatentMissionRiskError::ArithmeticOverflow)
        );
    }
}
