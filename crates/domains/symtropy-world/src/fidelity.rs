// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Deterministic adaptive-fidelity planning and explicit causal backpressure.
//!
//! This module chooses which requested representations fit a bounded work
//! budget. It never mutates domain state and never decides whether a domain's
//! answer is physically valid. Domains may explicitly return `NeedsRefinement`
//! when their current representation cannot answer a causal question safely.

use std::{collections::BTreeSet, error::Error, fmt};

use symtropy_sim_contracts::{
    AuthorityId, ContractError, RepresentationId, ScopeId, TypedDigest32,
};

/// One request to change the active representation for a scoped authority.
///
/// Priority is causal-first rather than distance-first. The five `u16` signals
/// are packed into a single `u128` score as ordered 16-bit lanes, making the
/// ordering deterministic and preventing high observer interest from outranking
/// any non-zero causal importance.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FidelityDemand {
    pub authority: AuthorityId,
    pub scope: ScopeId,
    pub current: RepresentationId,
    pub requested: RepresentationId,
    pub causal_importance: u16,
    pub instability: u16,
    pub predicted_intersection: u16,
    pub uncertainty: u16,
    pub observer_interest: u16,
    pub estimated_cost: u32,
}

impl FidelityDemand {
    pub fn validate(&self) -> Result<(), FidelityError> {
        if self.current == self.requested {
            return Err(FidelityError::SameRepresentationDemand {
                authority: self.authority.clone(),
                scope: self.scope.clone(),
                representation: self.current.clone(),
            });
        }
        if self.estimated_cost == 0 {
            return Err(FidelityError::ZeroEstimatedCost {
                authority: self.authority.clone(),
                scope: self.scope.clone(),
            });
        }
        Ok(())
    }

    /// Causal-first lexicographic priority encoded as one integer.
    ///
    /// Lane order, most significant first:
    /// causal importance → instability → predicted intersection → uncertainty
    /// → observer interest.
    pub const fn priority_score(&self) -> u128 {
        ((self.causal_importance as u128) << 64)
            | ((self.instability as u128) << 48)
            | ((self.predicted_intersection as u128) << 32)
            | ((self.uncertainty as u128) << 16)
            | self.observer_interest as u128
    }
}

/// Deterministic result of one bounded fidelity-planning cycle.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FidelitySelectionPlan {
    pub budget: u64,
    pub used: u64,
    /// Selected in deterministic priority order.
    pub selected: Vec<FidelityDemand>,
    /// Valid requests that did not fit the remaining budget, also in priority order.
    pub deferred: Vec<FidelityDemand>,
}

impl FidelitySelectionPlan {
    pub fn remaining_budget(&self) -> u64 {
        self.budget.saturating_sub(self.used)
    }
}

/// Stateless deterministic planner. Domain authorities remain responsible for
/// actually performing and proving any representation transfer.
#[derive(Clone, Copy, Debug, Default)]
pub struct FidelityScheduler;

impl FidelityScheduler {
    pub fn select(
        &self,
        demands: impl IntoIterator<Item = FidelityDemand>,
        budget: u64,
    ) -> Result<FidelitySelectionPlan, FidelityError> {
        let mut demands: Vec<_> = demands.into_iter().collect();
        let mut seen = BTreeSet::new();

        for demand in &demands {
            demand.validate()?;
            let key = (demand.authority.clone(), demand.scope.clone());
            if !seen.insert(key) {
                return Err(FidelityError::DuplicateAuthorityScopeDemand {
                    authority: demand.authority.clone(),
                    scope: demand.scope.clone(),
                });
            }
        }

        demands.sort_by(|left, right| {
            right
                .priority_score()
                .cmp(&left.priority_score())
                .then_with(|| left.authority.cmp(&right.authority))
                .then_with(|| left.scope.cmp(&right.scope))
                .then_with(|| left.requested.cmp(&right.requested))
                .then_with(|| left.current.cmp(&right.current))
        });

        let mut used = 0_u64;
        let mut selected = Vec::new();
        let mut deferred = Vec::new();

        for demand in demands {
            let cost = u64::from(demand.estimated_cost);
            if used.saturating_add(cost) <= budget {
                used += cost;
                selected.push(demand);
            } else {
                deferred.push(demand);
            }
        }

        Ok(FidelitySelectionPlan {
            budget,
            used,
            selected,
            deferred,
        })
    }
}

/// Why a domain cannot safely answer from its current representation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RefinementReason {
    InsufficientResolution,
    HighUncertainty,
    Instability,
    CausalBoundary,
    PredictedIntersection,
    DomainSpecific(String),
}

/// Explicit causal backpressure emitted by a domain instead of inventing an
/// answer from insufficiently resolved state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RefinementRequest {
    pub authority: AuthorityId,
    pub scope: ScopeId,
    pub current: RepresentationId,
    pub required: RepresentationId,
    pub reason: RefinementReason,
    pub evidence: TypedDigest32,
}

impl RefinementRequest {
    pub fn new(
        authority: AuthorityId,
        scope: ScopeId,
        current: RepresentationId,
        required: RepresentationId,
        reason: RefinementReason,
        evidence: TypedDigest32,
    ) -> Result<Self, FidelityError> {
        if current == required {
            return Err(FidelityError::SameRefinementRepresentation {
                authority,
                scope,
                representation: current,
            });
        }
        evidence.validate().map_err(FidelityError::Contract)?;
        if matches!(&reason, RefinementReason::DomainSpecific(value) if value.trim().is_empty()) {
            return Err(FidelityError::EmptyDomainSpecificReason);
        }
        Ok(Self {
            authority,
            scope,
            current,
            required,
            reason,
            evidence,
        })
    }
}

/// A domain result that can refuse to answer until the requested scope is
/// represented with enough fidelity. Callers must propagate or satisfy the
/// request; treating it as a successful result would erase causal uncertainty.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResolutionResult<T> {
    Resolved(T),
    NeedsRefinement(RefinementRequest),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FidelityError {
    Contract(ContractError),
    SameRepresentationDemand {
        authority: AuthorityId,
        scope: ScopeId,
        representation: RepresentationId,
    },
    ZeroEstimatedCost {
        authority: AuthorityId,
        scope: ScopeId,
    },
    DuplicateAuthorityScopeDemand {
        authority: AuthorityId,
        scope: ScopeId,
    },
    SameRefinementRepresentation {
        authority: AuthorityId,
        scope: ScopeId,
        representation: RepresentationId,
    },
    EmptyDomainSpecificReason,
}

impl From<ContractError> for FidelityError {
    fn from(value: ContractError) -> Self {
        Self::Contract(value)
    }
}

impl fmt::Display for FidelityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Contract(error) => write!(formatter, "simulation contract rejected fidelity request: {error}"),
            Self::SameRepresentationDemand {
                authority,
                scope,
                representation,
            } => write!(
                formatter,
                "{authority}/{scope} requested its already-active representation {representation}"
            ),
            Self::ZeroEstimatedCost { authority, scope } => write!(
                formatter,
                "{authority}/{scope} fidelity demand must declare non-zero estimated cost"
            ),
            Self::DuplicateAuthorityScopeDemand { authority, scope } => write!(
                formatter,
                "multiple fidelity demands target the same authority/scope {authority}/{scope}"
            ),
            Self::SameRefinementRepresentation {
                authority,
                scope,
                representation,
            } => write!(
                formatter,
                "{authority}/{scope} refinement cannot require the current representation {representation}"
            ),
            Self::EmptyDomainSpecificReason => {
                write!(formatter, "domain-specific refinement reason may not be empty")
            }
        }
    }
}

impl Error for FidelityError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn demand(
        scope: &str,
        causal_importance: u16,
        instability: u16,
        predicted_intersection: u16,
        uncertainty: u16,
        observer_interest: u16,
        estimated_cost: u32,
    ) -> FidelityDemand {
        FidelityDemand {
            authority: AuthorityId::parse("terrain.authority.v1").unwrap(),
            scope: ScopeId::parse(scope).unwrap(),
            current: RepresentationId::parse("terrain.aggregate.v1").unwrap(),
            requested: RepresentationId::parse("terrain.voxel.v1").unwrap(),
            causal_importance,
            instability,
            predicted_intersection,
            uncertainty,
            observer_interest,
            estimated_cost,
        }
    }

    fn selected_scopes(plan: &FidelitySelectionPlan) -> Vec<&str> {
        plan.selected
            .iter()
            .map(|demand| demand.scope.as_str())
            .collect()
    }

    #[test]
    fn insertion_order_does_not_change_selection() {
        let a = demand("sol:earth:a", 10, 0, 0, 0, 0, 4);
        let b = demand("sol:earth:b", 20, 0, 0, 0, 0, 4);
        let c = demand("sol:earth:c", 5, 0, 0, 0, 0, 4);
        let scheduler = FidelityScheduler;

        let forward = scheduler
            .select(vec![a.clone(), b.clone(), c.clone()], 8)
            .unwrap();
        let reverse = scheduler.select(vec![c, b, a], 8).unwrap();

        assert_eq!(forward, reverse);
        assert_eq!(selected_scopes(&forward), vec!["sol:earth:b", "sol:earth:a"]);
    }

    #[test]
    fn causal_importance_outranks_observer_interest_only() {
        let causally_important = demand("sol:mars:reactor-17", 1, 0, 0, 0, 0, 5);
        let visually_near = demand("sol:earth:camera-near", 0, 0, 0, 0, u16::MAX, 5);
        let plan = FidelityScheduler
            .select(vec![visually_near, causally_important], 5)
            .unwrap();

        assert_eq!(selected_scopes(&plan), vec!["sol:mars:reactor-17"]);
    }

    #[test]
    fn budget_skips_expensive_request_and_uses_remaining_capacity() {
        let expensive = demand("sol:earth:expensive", 100, 0, 0, 0, 0, 10);
        let affordable = demand("sol:earth:affordable", 90, 0, 0, 0, 0, 4);
        let plan = FidelityScheduler
            .select(vec![affordable, expensive], 6)
            .unwrap();

        assert_eq!(selected_scopes(&plan), vec!["sol:earth:affordable"]);
        assert_eq!(plan.used, 4);
        assert_eq!(plan.remaining_budget(), 2);
        assert_eq!(plan.deferred[0].scope.as_str(), "sol:earth:expensive");
    }

    #[test]
    fn stable_identity_breaks_equal_priority_ties() {
        let z = demand("sol:earth:z", 1, 2, 3, 4, 5, 1);
        let a = demand("sol:earth:a", 1, 2, 3, 4, 5, 1);
        let plan = FidelityScheduler.select(vec![z, a], 2).unwrap();

        assert_eq!(selected_scopes(&plan), vec!["sol:earth:a", "sol:earth:z"]);
    }

    #[test]
    fn duplicate_authority_scope_is_rejected() {
        let first = demand("sol:earth:cell", 1, 0, 0, 0, 0, 1);
        let mut second = first.clone();
        second.requested = RepresentationId::parse("terrain.mesh.v2").unwrap();

        assert!(matches!(
            FidelityScheduler.select(vec![first, second], 10),
            Err(FidelityError::DuplicateAuthorityScopeDemand { .. })
        ));
    }

    #[test]
    fn domain_can_apply_explicit_causal_backpressure() {
        let request = RefinementRequest::new(
            AuthorityId::parse("hydrology.authority.v1").unwrap(),
            ScopeId::parse("sol:earth:watershed-9").unwrap(),
            RepresentationId::parse("hydrology.watershed.v1").unwrap(),
            RepresentationId::parse("hydrology.local-flow.v1").unwrap(),
            RefinementReason::CausalBoundary,
            TypedDigest32::sha256(
                "symtropy.hydrology.refinement-evidence.v1",
                1,
                b"unresolved dam boundary",
            )
            .unwrap(),
        )
        .unwrap();
        let result: ResolutionResult<u32> = ResolutionResult::NeedsRefinement(request.clone());

        assert_eq!(result, ResolutionResult::NeedsRefinement(request));
    }
}
