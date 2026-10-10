// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Executable oracle for resource-paid plant structural growth.
//!
//! This freezes transaction semantics independently of a future plant module:
//! planning is read-only; canonical structure changes only when the exact stock
//! transfer and structural append commit atomically.

use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct GrowthRequestId(u128);

/// Stable commitment to the complete intended growth action.
///
/// A future product API should derive this from canonical target element,
/// operation kind, developmental program/schema, requested structural change,
/// exact cost basis, and any other fields that distinguish one growth action
/// from another. It is deliberately not a render/ECS identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct GrowthIntentFingerprint(u128);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct GrowthPlan {
    request_id: GrowthRequestId,
    intent: GrowthIntentFingerprint,
    expected_revision: u64,
    cost_mg: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct GrowthCommit {
    element_id: u64,
    intent: GrowthIntentFingerprint,
    cost_mg: u64,
    committed_revision: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GrowthError {
    ZeroCost,
    InsufficientStock { available_mg: u64, required_mg: u64 },
    StalePlan { expected_revision: u64, actual_revision: u64 },
    RevisionOverflow,
    ElementIdOverflow,
    StockOverflow,
    RequestConflict,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PlantGrowthState {
    free_stock_mg: u64,
    structural_stock_mg: u64,
    revision: u64,
    next_element_id: u64,
    committed: BTreeMap<GrowthRequestId, GrowthCommit>,
}

impl PlantGrowthState {
    fn with_free_stock(free_stock_mg: u64) -> Self {
        Self {
            free_stock_mg,
            structural_stock_mg: 0,
            revision: 0,
            next_element_id: 1,
            committed: BTreeMap::new(),
        }
    }

    fn total_owned_stock(&self) -> Result<u64, GrowthError> {
        self.free_stock_mg
            .checked_add(self.structural_stock_mg)
            .ok_or(GrowthError::StockOverflow)
    }

    fn prepare_growth(
        &self,
        request_id: GrowthRequestId,
        intent: GrowthIntentFingerprint,
        cost_mg: u64,
    ) -> Result<GrowthPlan, GrowthError> {
        if cost_mg == 0 {
            return Err(GrowthError::ZeroCost);
        }
        if self.free_stock_mg < cost_mg {
            return Err(GrowthError::InsufficientStock {
                available_mg: self.free_stock_mg,
                required_mg: cost_mg,
            });
        }
        Ok(GrowthPlan {
            request_id,
            intent,
            expected_revision: self.revision,
            cost_mg,
        })
    }

    fn commit_growth(&mut self, plan: GrowthPlan) -> Result<GrowthCommit, GrowthError> {
        if let Some(existing) = self.committed.get(&plan.request_id).copied() {
            if existing.intent == plan.intent && existing.cost_mg == plan.cost_mg {
                return Ok(existing);
            }
            return Err(GrowthError::RequestConflict);
        }

        if plan.expected_revision != self.revision {
            return Err(GrowthError::StalePlan {
                expected_revision: plan.expected_revision,
                actual_revision: self.revision,
            });
        }
        if self.free_stock_mg < plan.cost_mg {
            return Err(GrowthError::InsufficientStock {
                available_mg: self.free_stock_mg,
                required_mg: plan.cost_mg,
            });
        }

        let next_revision = self
            .revision
            .checked_add(1)
            .ok_or(GrowthError::RevisionOverflow)?;
        let next_element_id = self
            .next_element_id
            .checked_add(1)
            .ok_or(GrowthError::ElementIdOverflow)?;
        let next_structural_stock = self
            .structural_stock_mg
            .checked_add(plan.cost_mg)
            .ok_or(GrowthError::StockOverflow)?;
        let next_free_stock = self.free_stock_mg - plan.cost_mg;

        let commit = GrowthCommit {
            element_id: self.next_element_id,
            intent: plan.intent,
            cost_mg: plan.cost_mg,
            committed_revision: next_revision,
        };

        self.free_stock_mg = next_free_stock;
        self.structural_stock_mg = next_structural_stock;
        self.revision = next_revision;
        self.next_element_id = next_element_id;
        self.committed.insert(plan.request_id, commit);
        Ok(commit)
    }
}

fn intent(value: u128) -> GrowthIntentFingerprint {
    GrowthIntentFingerprint(value)
}

#[test]
fn preparing_growth_is_read_only() {
    let state = PlantGrowthState::with_free_stock(1_000);
    let before = state.clone();

    let plan = state
        .prepare_growth(GrowthRequestId(1), intent(101), 125)
        .unwrap();

    assert_eq!(plan.expected_revision, 0);
    assert_eq!(state, before);
}

#[test]
fn successful_growth_transfers_exact_stock_and_conserves_total_ownership() {
    let mut state = PlantGrowthState::with_free_stock(1_000);
    let initial_total = state.total_owned_stock().unwrap();
    let plan = state
        .prepare_growth(GrowthRequestId(7), intent(707), 275)
        .unwrap();

    let commit = state.commit_growth(plan).unwrap();

    assert_eq!(commit.element_id, 1);
    assert_eq!(commit.intent, intent(707));
    assert_eq!(commit.cost_mg, 275);
    assert_eq!(commit.committed_revision, 1);
    assert_eq!(state.free_stock_mg, 725);
    assert_eq!(state.structural_stock_mg, 275);
    assert_eq!(state.total_owned_stock().unwrap(), initial_total);
}

#[test]
fn insufficient_stock_cannot_create_structure() {
    let state = PlantGrowthState::with_free_stock(40);
    let before = state.clone();

    assert_eq!(
        state.prepare_growth(GrowthRequestId(9), intent(909), 41),
        Err(GrowthError::InsufficientStock {
            available_mg: 40,
            required_mg: 41,
        })
    );
    assert_eq!(state, before);
}

#[test]
fn stale_plan_fails_with_zero_mutation() {
    let mut state = PlantGrowthState::with_free_stock(1_000);
    let stale = state
        .prepare_growth(GrowthRequestId(1), intent(11), 100)
        .unwrap();
    let winner = state
        .prepare_growth(GrowthRequestId(2), intent(22), 200)
        .unwrap();

    state.commit_growth(winner).unwrap();
    let before_stale_commit = state.clone();

    assert_eq!(
        state.commit_growth(stale),
        Err(GrowthError::StalePlan {
            expected_revision: 0,
            actual_revision: 1,
        })
    );
    assert_eq!(state, before_stale_commit);
}

#[test]
fn duplicate_retry_is_idempotent_and_cannot_grow_twice() {
    let mut state = PlantGrowthState::with_free_stock(500);
    let plan = state
        .prepare_growth(GrowthRequestId(42), intent(4_242), 125)
        .unwrap();

    let first = state.commit_growth(plan).unwrap();
    let after_first = state.clone();
    let retry = state.commit_growth(plan).unwrap();

    assert_eq!(retry, first);
    assert_eq!(state, after_first);
    assert_eq!(state.structural_stock_mg, 125);
    assert_eq!(state.next_element_id, 2);
}

#[test]
fn reused_request_identity_with_different_intent_fails_closed_even_at_same_cost() {
    let mut state = PlantGrowthState::with_free_stock(500);
    let original = state
        .prepare_growth(GrowthRequestId(5), intent(500), 100)
        .unwrap();
    state.commit_growth(original).unwrap();
    let before_conflict = state.clone();

    let conflicting = GrowthPlan {
        request_id: GrowthRequestId(5),
        intent: intent(501),
        expected_revision: state.revision,
        cost_mg: 100,
    };

    assert_eq!(state.commit_growth(conflicting), Err(GrowthError::RequestConflict));
    assert_eq!(state, before_conflict);
}

#[test]
fn reused_request_identity_with_different_cost_fails_closed() {
    let mut state = PlantGrowthState::with_free_stock(500);
    let original = state
        .prepare_growth(GrowthRequestId(6), intent(600), 100)
        .unwrap();
    state.commit_growth(original).unwrap();
    let before_conflict = state.clone();

    let conflicting = GrowthPlan {
        request_id: GrowthRequestId(6),
        intent: intent(600),
        expected_revision: state.revision,
        cost_mg: 101,
    };

    assert_eq!(state.commit_growth(conflicting), Err(GrowthError::RequestConflict));
    assert_eq!(state, before_conflict);
}

#[test]
fn zero_cost_growth_is_rejected_as_unpaid_structure() {
    let state = PlantGrowthState::with_free_stock(500);
    assert_eq!(
        state.prepare_growth(GrowthRequestId(3), intent(303), 0),
        Err(GrowthError::ZeroCost)
    );
}

#[test]
fn repeated_commits_preserve_total_stock_and_monotonic_element_ids() {
    let mut state = PlantGrowthState::with_free_stock(1_000);
    let initial_total = state.total_owned_stock().unwrap();
    let mut ids = Vec::new();

    for (request, fingerprint, cost) in [
        (1_u128, 101_u128, 100_u64),
        (2, 202, 250),
        (3, 303, 50),
        (4, 404, 300),
    ] {
        let plan = state
            .prepare_growth(GrowthRequestId(request), intent(fingerprint), cost)
            .unwrap();
        ids.push(state.commit_growth(plan).unwrap().element_id);
        assert_eq!(state.total_owned_stock().unwrap(), initial_total);
    }

    assert_eq!(ids, vec![1, 2, 3, 4]);
    assert_eq!(state.free_stock_mg, 300);
    assert_eq!(state.structural_stock_mg, 700);
}
