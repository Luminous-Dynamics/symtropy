// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Deterministic, semantics-preserving level-of-detail (LOD) budget allocation.
//!
//! This module is a pure policy kernel, not a Bevy asset/mesh LOD integration.
//! The host supplies ordered representation candidates with measured or
//! calibrated relative cost units and conservative screen-space error estimates.
//! It never reads or mutates canonical simulation state.
//!
//! Candidates are ordered from coarsest to finest. Within one demand, increasing
//! quality rank must strictly increase cost and must not increase screen-space
//! error. Cost units are comparable only inside one allocation call unless the
//! caller has calibrated them to a common machine/backend/workload profile.
//!
//! Selected and critical entities are pinned to their finest candidate. If the
//! total budget cannot satisfy those pins and the remaining minimum costs, the
//! allocator returns an error rather than silently dropping protected detail.

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashSet};

/// Failure from malformed LOD data or an infeasible hard budget.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LodError {
    /// Candidate lists must contain at least one representation.
    EmptyCandidates,
    /// Entity IDs must be unique within an allocation.
    DuplicateEntityId,
    /// Representation IDs must be unique within one entity's candidate list.
    DuplicateRepresentationId,
    /// Importance weights must be positive.
    InvalidImportance,
    /// Costs must be positive and strictly increase with quality rank.
    InvalidCostOrCostOrder,
    /// Screen-space errors must be finite, non-negative, and non-increasing
    /// as quality rank increases.
    InvalidScreenSpaceErrorOrOrder,
    /// Candidate quality ranks must be strictly increasing.
    InvalidQualityRankOrder,
    /// The maximum permitted screen-space error is invalid.
    InvalidScreenErrorBudget,
    /// Even the finest supplied representation exceeds the screen-error ceiling.
    ScreenErrorBudgetInfeasible,
    /// The cost budget cannot be met without violating protected-detail or
    /// screen-error constraints.
    BudgetInfeasible,
}

/// One visual representation of an entity.
///
/// Candidates are supplied in ascending quality-rank order: coarse first,
/// finest last. The error estimate is in pixels and must be computed for the
/// current view by the host from a stated geometric error bound. Cost units
/// are calibrated relative units, not milliseconds or GPU timestamps.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LodCandidate {
    /// Stable quality ordering. Higher means more detail.
    pub quality_rank: u16,
    /// Host-defined stable ID for this representation (for example, a mesh LOD).
    pub representation_id: u32,
    /// Estimated maximum screen-space deviation in pixels.
    pub screen_error_px: f32,
    /// Positive, calibrated relative cost units for this representation.
    pub cost_units: u32,
}

/// Rendering demand for one semantic entity.
///
/// A demand marked selected or critical is pinned to its finest candidate.
#[derive(Debug, Clone, PartialEq)]
pub struct LodDemand {
    /// Stable identity independent of rendering order or ECS iteration order.
    pub entity_id: u64,
    /// Positive relative importance weight for visual-error trade-offs.
    pub importance: u32,
    /// User- or tool-selected entities preserve their finest available detail.
    pub selected: bool,
    /// Safety-, engineering-, or task-critical entities preserve their detail.
    pub critical: bool,
    /// Validated candidates ordered from coarse to fine.
    pub candidates: Vec<LodCandidate>,
}

impl LodDemand {
    fn is_protected(&self) -> bool {
        self.selected || self.critical
    }
}

/// Chosen representation for one entity.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LodChoice {
    pub entity_id: u64,
    pub representation_id: u32,
    pub quality_rank: u16,
    pub screen_error_px: f32,
    pub cost_units: u32,
    pub protected: bool,
}

/// Deterministic allocation result, sorted by entity ID.
#[derive(Debug, Clone, PartialEq)]
pub struct LodAllocation {
    pub choices: Vec<LodChoice>,
    pub budget_units: u64,
    pub total_cost_units: u64,
    /// Sum of importance * screen_error_px^2 for the selected representations.
    /// This is a policy objective, not a perceptual-quality score.
    pub total_weighted_error_squared: f64,
    /// Number of entities assigned below their finest candidate.
    pub degraded_entity_count: usize,
}

#[derive(Debug, Clone, Copy)]
struct Demotion {
    loss_per_saved_unit: f64,
    entity_id: u64,
    demand_index: usize,
    saved_units: u64,
}

impl PartialEq for Demotion {
    fn eq(&self, other: &Self) -> bool {
        self.loss_per_saved_unit
            .total_cmp(&other.loss_per_saved_unit)
            .is_eq()
            && self.entity_id == other.entity_id
            && self.demand_index == other.demand_index
            && self.saved_units == other.saved_units
    }
}

impl Eq for Demotion {}

impl PartialOrd for Demotion {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Demotion {
    fn cmp(&self, other: &Self) -> Ordering {
        // BinaryHeap is a max-heap: reverse the score so the smallest error
        // loss per saved unit is selected first. Stable IDs break score ties.
        other
            .loss_per_saved_unit
            .total_cmp(&self.loss_per_saved_unit)
            .then_with(|| other.entity_id.cmp(&self.entity_id))
            .then_with(|| other.demand_index.cmp(&self.demand_index))
    }
}

fn next_demotion(
    demand: &LodDemand,
    demand_index: usize,
    current_index: usize,
    max_screen_error_px: f32,
) -> Option<Demotion> {
    if demand.is_protected() || current_index == 0 {
        return None;
    }

    let current = demand.candidates[current_index];
    let next = demand.candidates[current_index - 1];
    if next.screen_error_px > max_screen_error_px {
        return None;
    }

    let saved_units = u64::from(current.cost_units - next.cost_units);
    let current_error = f64::from(current.screen_error_px);
    let next_error = f64::from(next.screen_error_px);
    let added_error =
        f64::from(demand.importance) * (next_error * next_error - current_error * current_error);

    Some(Demotion {
        loss_per_saved_unit: added_error / saved_units as f64,
        entity_id: demand.entity_id,
        demand_index,
        saved_units,
    })
}

/// Allocate representations under a shared cost budget.
///
/// The starting point is the finest available representation for every entity.
/// When over budget, the allocator repeatedly selects the downgrade with the
/// smallest incremental weighted squared screen-error per saved cost unit.
/// Ties are broken by stable entity ID, so input iteration order does not affect
/// the output. The policy is deterministic but greedy; it does not claim a
/// globally optimal knapsack solution.
///
/// Protected entities are never downgraded. If no legal downgrade can fit the
/// budget, this function fails closed with BudgetInfeasible.
pub fn allocate_lod(
    demands: &[LodDemand],
    budget_units: u64,
    max_screen_error_px: f32,
) -> Result<LodAllocation, LodError> {
    validate_demands(demands)?;

    if !max_screen_error_px.is_finite() || max_screen_error_px < 0.0 {
        return Err(LodError::InvalidScreenErrorBudget);
    }
    if demands.iter().any(|demand| {
        demand
            .candidates
            .last()
            .is_some_and(|candidate| candidate.screen_error_px > max_screen_error_px)
    }) {
        return Err(LodError::ScreenErrorBudgetInfeasible);
    }

    let mut selected_indices: Vec<usize> = demands
        .iter()
        .map(|demand| demand.candidates.len() - 1)
        .collect();

    let mut total_cost_units = 0_u64;
    for demand in demands {
        let cost = u64::from(
            demand
                .candidates
                .last()
                .expect("validated non-empty candidate list")
                .cost_units,
        );
        total_cost_units = total_cost_units
            .checked_add(cost)
            .ok_or(LodError::InvalidCostOrCostOrder)?;
    }

    let mut demotions = BinaryHeap::new();
    for (demand_index, demand) in demands.iter().enumerate() {
        let finest_index = selected_indices[demand_index];
        if let Some(demotion) =
            next_demotion(demand, demand_index, finest_index, max_screen_error_px)
        {
            demotions.push(demotion);
        }
    }

    while total_cost_units > budget_units {
        let Some(demotion) = demotions.pop() else {
            return Err(LodError::BudgetInfeasible);
        };

        selected_indices[demotion.demand_index] -= 1;
        total_cost_units = total_cost_units
            .checked_sub(demotion.saved_units)
            .ok_or(LodError::InvalidCostOrCostOrder)?;

        let next_index = selected_indices[demotion.demand_index];
        if let Some(next) = next_demotion(
            &demands[demotion.demand_index],
            demotion.demand_index,
            next_index,
            max_screen_error_px,
        ) {
            demotions.push(next);
        }
    }

    let mut choices = Vec::with_capacity(demands.len());
    let mut weighted_error_contributions = Vec::with_capacity(demands.len());
    let mut degraded_entity_count = 0_usize;

    for (demand, selected_index) in demands.iter().zip(selected_indices.iter().copied()) {
        let candidate = demand.candidates[selected_index];
        let weighted_error_squared = f64::from(demand.importance)
            * f64::from(candidate.screen_error_px)
            * f64::from(candidate.screen_error_px);
        weighted_error_contributions.push((demand.entity_id, weighted_error_squared));

        if selected_index + 1 < demand.candidates.len() {
            degraded_entity_count += 1;
        }

        choices.push(LodChoice {
            entity_id: demand.entity_id,
            representation_id: candidate.representation_id,
            quality_rank: candidate.quality_rank,
            screen_error_px: candidate.screen_error_px,
            cost_units: candidate.cost_units,
            protected: demand.is_protected(),
        });
    }

    choices.sort_by_key(|choice| choice.entity_id);
    weighted_error_contributions.sort_by_key(|(entity_id, _)| *entity_id);
    let total_weighted_error_squared = weighted_error_contributions
        .iter()
        .map(|(_, contribution)| *contribution)
        .sum();

    Ok(LodAllocation {
        choices,
        budget_units,
        total_cost_units,
        total_weighted_error_squared,
        degraded_entity_count,
    })
}

fn validate_demands(demands: &[LodDemand]) -> Result<(), LodError> {
    let mut entity_ids = HashSet::with_capacity(demands.len());

    for demand in demands {
        if !entity_ids.insert(demand.entity_id) {
            return Err(LodError::DuplicateEntityId);
        }
        if demand.importance == 0 {
            return Err(LodError::InvalidImportance);
        }
        if demand.candidates.is_empty() {
            return Err(LodError::EmptyCandidates);
        }

        let mut representation_ids = HashSet::with_capacity(demand.candidates.len());
        let mut previous: Option<LodCandidate> = None;

        for candidate in &demand.candidates {
            if !representation_ids.insert(candidate.representation_id) {
                return Err(LodError::DuplicateRepresentationId);
            }
            if candidate.cost_units == 0 {
                return Err(LodError::InvalidCostOrCostOrder);
            }
            if !candidate.screen_error_px.is_finite() || candidate.screen_error_px < 0.0 {
                return Err(LodError::InvalidScreenSpaceErrorOrOrder);
            }

            if let Some(previous) = previous {
                if candidate.quality_rank <= previous.quality_rank {
                    return Err(LodError::InvalidQualityRankOrder);
                }
                if candidate.cost_units <= previous.cost_units {
                    return Err(LodError::InvalidCostOrCostOrder);
                }
                if candidate.screen_error_px > previous.screen_error_px {
                    return Err(LodError::InvalidScreenSpaceErrorOrOrder);
                }
            }
            previous = Some(*candidate);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(rank: u16, representation_id: u32, error: f32, cost: u32) -> LodCandidate {
        LodCandidate {
            quality_rank: rank,
            representation_id,
            screen_error_px: error,
            cost_units: cost,
        }
    }

    fn demand(entity_id: u64, importance: u32, selected: bool, critical: bool) -> LodDemand {
        LodDemand {
            entity_id,
            importance,
            selected,
            critical,
            candidates: vec![candidate(0, 100, 4.0, 1), candidate(1, 101, 2.0, 3)],
        }
    }

    #[test]
    fn keeps_finest_representations_when_budget_allows() {
        let demands = vec![demand(8, 1, false, false), demand(2, 1, false, false)];

        let allocation = allocate_lod(&demands, 6, 4.0).expect("budget is sufficient");

        assert_eq!(allocation.total_cost_units, 6);
        assert_eq!(allocation.degraded_entity_count, 0);
        assert_eq!(
            allocation
                .choices
                .iter()
                .map(|choice| choice.entity_id)
                .collect::<Vec<_>>(),
            vec![2, 8]
        );
        assert!(
            allocation
                .choices
                .iter()
                .all(|choice| choice.quality_rank == 1)
        );
    }

    #[test]
    fn spends_detail_on_high_importance_entities_first() {
        let mut low_importance = demand(10, 1, false, false);
        low_importance.candidates = vec![candidate(0, 200, 4.0, 1), candidate(1, 201, 1.0, 3)];

        let mut high_importance = demand(20, 10, false, false);
        high_importance.candidates = vec![candidate(0, 300, 4.0, 1), candidate(1, 301, 1.0, 3)];

        let allocation = allocate_lod(&[high_importance, low_importance], 5, 4.0)
            .expect("one downgrade should satisfy the budget");
        let low = allocation
            .choices
            .iter()
            .find(|choice| choice.entity_id == 10)
            .expect("low importance choice exists");
        let high = allocation
            .choices
            .iter()
            .find(|choice| choice.entity_id == 20)
            .expect("high importance choice exists");

        assert_eq!(low.quality_rank, 0);
        assert_eq!(high.quality_rank, 1);
        assert_eq!(allocation.total_cost_units, 4);
    }

    #[test]
    fn selected_and_critical_entities_are_pinned_to_finest_detail() {
        let selected = demand(1, 1, true, false);
        let critical = demand(2, 1, false, true);
        let ordinary = demand(3, 1, false, false);

        let allocation = allocate_lod(&[ordinary, critical, selected], 7, 4.0)
            .expect("ordinary entity can degrade");

        assert!(
            allocation
                .choices
                .iter()
                .filter(|choice| choice.entity_id == 1 || choice.entity_id == 2)
                .all(|choice| choice.quality_rank == 1 && choice.protected)
        );
        assert_eq!(
            allocation
                .choices
                .iter()
                .find(|choice| choice.entity_id == 3)
                .expect("ordinary choice exists")
                .quality_rank,
            0
        );
        assert_eq!(allocation.total_cost_units, 7);
    }

    #[test]
    fn fails_closed_when_protected_detail_cannot_fit() {
        let protected = demand(1, 1, true, false);
        let ordinary = demand(2, 1, false, false);

        assert_eq!(
            allocate_lod(&[protected, ordinary], 3, 4.0),
            Err(LodError::BudgetInfeasible)
        );
    }

    #[test]
    fn allocation_is_independent_of_input_order() {
        let a = demand(40, 2, false, false);
        let b = demand(10, 3, false, false);
        let c = demand(30, 1, false, false);

        let forward = allocate_lod(&[a.clone(), b.clone(), c.clone()], 7, 4.0)
            .expect("budget permits deterministic degradation");
        let reverse = allocate_lod(&[c, b, a], 7, 4.0).expect("reordered input remains valid");

        assert_eq!(forward, reverse);
    }

    #[test]
    fn rejects_duplicate_entity_ids() {
        assert_eq!(
            allocate_lod(
                &[demand(1, 1, false, false), demand(1, 2, false, false)],
                10,
                4.0
            ),
            Err(LodError::DuplicateEntityId)
        );
    }

    #[test]
    fn rejects_empty_candidate_sets_and_invalid_errors() {
        let mut empty = demand(1, 1, false, false);
        empty.candidates.clear();
        assert_eq!(
            allocate_lod(&[empty], 10, 4.0),
            Err(LodError::EmptyCandidates)
        );

        let mut invalid = demand(2, 1, false, false);
        invalid.candidates[1].screen_error_px = f32::NAN;
        assert_eq!(
            allocate_lod(&[invalid], 10, 4.0),
            Err(LodError::InvalidScreenSpaceErrorOrOrder)
        );
    }

    #[test]
    fn rejects_non_monotonic_candidate_cost_and_quality() {
        let mut invalid_cost = demand(1, 1, false, false);
        invalid_cost.candidates[1].cost_units = 1;
        assert_eq!(
            allocate_lod(&[invalid_cost], 10, 4.0),
            Err(LodError::InvalidCostOrCostOrder)
        );

        let mut invalid_rank = demand(2, 1, false, false);
        invalid_rank.candidates[1].quality_rank = 0;
        assert_eq!(
            allocate_lod(&[invalid_rank], 10, 4.0),
            Err(LodError::InvalidQualityRankOrder)
        );
    }

    #[test]
    fn rejects_zero_importance_and_duplicate_representations() {
        let mut no_importance = demand(1, 1, false, false);
        no_importance.importance = 0;
        assert_eq!(
            allocate_lod(&[no_importance], 10, 4.0),
            Err(LodError::InvalidImportance)
        );

        let mut duplicate = demand(2, 1, false, false);
        duplicate.candidates[1].representation_id = duplicate.candidates[0].representation_id;
        assert_eq!(
            allocate_lod(&[duplicate], 10, 4.0),
            Err(LodError::DuplicateRepresentationId)
        );
    }

    #[test]
    fn enforces_the_screen_error_ceiling() {
        assert_eq!(
            allocate_lod(&[demand(1, 1, false, false)], 10, 1.5),
            Err(LodError::ScreenErrorBudgetInfeasible)
        );
        assert_eq!(
            allocate_lod(&[], 0, f32::INFINITY),
            Err(LodError::InvalidScreenErrorBudget)
        );
    }

    #[test]
    fn empty_scene_has_zero_cost() {
        let allocation = allocate_lod(&[], 0, 4.0).expect("empty scene fits every budget");
        assert!(allocation.choices.is_empty());
        assert_eq!(allocation.total_cost_units, 0);
        assert_eq!(allocation.degraded_entity_count, 0);
    }
}
