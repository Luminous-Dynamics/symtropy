// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Bounded VCG-style procurement of consented schedule-flexibility options.
//!
//! This is a finite research/reference mechanism, not a live market or schedule
//! executor. Its restricted model assumes one candidate offer per participant,
//! additive externally declared gross benefits, one private reported cost per
//! participant, quasilinear utility, and a complete finite offer set. It does
//! not guarantee strong budget balance or fairness. All results are proposals;
//! Mycelix must separately authorize any offer activation or schedule change.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

/// A time bucket backed by one canonical physical resource.
///
/// For this v0.1 mechanism, time buckets for the same physical resource must
/// not overlap. Overlapping-window scheduling belongs to the existing
/// schedule-aware capacity contract and is rejected here rather than guessed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FlexMechanismSlot {
    pub id: String,
    pub physical_resource_id: String,
    pub start_minute: u64,
    pub end_minute: u64,
    pub capacity: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FlexMechanismDemand {
    pub id: String,
    pub participant_id: String,
    pub baseline_slot_id: String,
    pub quantity: u64,
    pub hard_latest_start_minute: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlexOfferState {
    Offered,
    AcceptedConditional,
    Revoked,
}

/// One participant's single-parameter bid for moving one demand to one slot.
///
/// Gross operational benefit is supplied by the caller's declared evaluation
/// model and is not self-reported by the participant. The evidence ID is only a
/// reference: this pure module does not authenticate or dereference it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FlexMechanismOffer {
    pub id: String,
    pub participant_id: String,
    pub demand_id: String,
    pub target_slot_id: String,
    pub state: FlexOfferState,
    /// Time the participant-authorized conditional acceptance became known.
    /// Required for AcceptedConditional offers and must not exceed decision time.
    pub accepted_at_minute: Option<u64>,
    pub expires_at_minute: u64,
    pub max_deviation_minutes: u64,
    pub reported_cost_minor_units: u64,
    pub gross_benefit_minor_units: u64,
    pub benefit_evidence_id: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FlexMechanismLimits {
    /// Total recursive search nodes across allocation and all counterfactuals.
    pub max_search_steps: usize,
    /// Hard guard against exponential search and recursion depth.
    pub max_eligible_offers: usize,
}

impl Default for FlexMechanismLimits {
    fn default() -> Self {
        Self {
            max_search_steps: 1_000_000,
            max_eligible_offers: 16,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FlexMechanismInput {
    pub time_profile_id: String,
    pub currency_id: String,
    pub decision_time_minute: u64,
    pub slots: Vec<FlexMechanismSlot>,
    pub demands: Vec<FlexMechanismDemand>,
    pub offers: Vec<FlexMechanismOffer>,
    /// An affordability diagnostic only. It does not silently change the
    /// welfare-maximizing allocation, because such a post-hoc rejection can
    /// invalidate the mechanism's incentive properties.
    pub buyer_budget_limit_minor_units: Option<u64>,
    pub limits: FlexMechanismLimits,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlexOfferExclusionReason {
    NotAcceptedConditional,
    Revoked,
    AcceptanceReceiptMissing,
    AcceptanceAfterDecision,
    ExpiredAtDecision,
    SameSlotAsBaseline,
    ShiftExceedsDeclaredLimit,
    HardDeadlineWouldBeMissed,
    BenefitEvidenceReferenceMissing,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FlexExcludedOffer {
    pub offer_id: String,
    pub reason: FlexOfferExclusionReason,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FlexMechanismSlotUsage {
    pub slot_id: String,
    pub physical_resource_id: String,
    pub baseline_quantity: u64,
    pub final_quantity: u64,
    pub capacity: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FlexMechanismPayment {
    pub offer_id: String,
    pub participant_id: String,
    pub reported_cost_minor_units: u64,
    /// VCG/pivot compensation recommended for the selected offer.
    pub payment_minor_units: u64,
    /// Best declared welfare when this participant/offer is removed.
    pub counterfactual_welfare_minor_units: i128,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlexOptimalityScope {
    ExactOverEligibleOfferSet,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FlexMechanismReport {
    pub time_profile_id: String,
    pub currency_id: String,
    pub selected_offer_ids: Vec<String>,
    pub gross_benefit_minor_units: u64,
    pub reported_cost_minor_units: u64,
    pub total_surplus_minor_units: i128,
    pub total_recommended_compensation_minor_units: u64,
    /// None means no budget was supplied; Some(0) means within budget;
    /// Some(n > 0) is the amount by which pivot compensation exceeds budget.
    pub buyer_budget_shortfall_minor_units: Option<u64>,
    pub payments: Vec<FlexMechanismPayment>,
    pub excluded_offers: Vec<FlexExcludedOffer>,
    pub slot_usage: Vec<FlexMechanismSlotUsage>,
    pub search_steps: usize,
    pub optimality_scope: FlexOptimalityScope,
    /// True only for the supplied finite eligible-offer set. It says nothing
    /// about real-world offer completeness, physical availability or authority.
    pub search_complete: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FlexMechanismError {
    InvalidSearchLimits,
    MissingTimeProfile,
    MissingCurrency,
    EmptySlots,
    EmptyDemands,
    DuplicateSlotId(String),
    EmptyPhysicalResourceId(String),
    InvalidSlotWindow(String),
    OverlappingSlotsOnPhysicalResource {
        physical_resource_id: String,
        first_slot_id: String,
        second_slot_id: String,
    },
    DuplicateDemandId(String),
    InvalidDemand(String),
    UnknownDemandSlot {
        demand_id: String,
        slot_id: String,
    },
    BaselineCapacityExceeded {
        slot_id: String,
        quantity: u64,
        capacity: u64,
    },
    DuplicateOfferId(String),
    DuplicateParticipantOffer(String),
    EmptyOfferIdentity(String),
    UnknownOfferDemand {
        offer_id: String,
        demand_id: String,
    },
    OfferParticipantMismatch(String),
    UnknownOfferTarget {
        offer_id: String,
        slot_id: String,
    },
    TooManyEligibleOffers {
        count: usize,
        limit: usize,
    },
    SearchBudgetExceeded {
        limit: usize,
    },
    NoFeasibleAllocation,
    ArithmeticOverflow,
    NegativeOrUnrepresentablePivotPayment(String),
}

impl fmt::Display for FlexMechanismError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSearchLimits => write!(f, "flex mechanism search limits must be positive"),
            Self::MissingTimeProfile => write!(f, "flex mechanism time profile identity is empty"),
            Self::MissingCurrency => write!(f, "flex mechanism currency identity is empty"),
            Self::EmptySlots => write!(f, "flex mechanism requires at least one slot"),
            Self::EmptyDemands => write!(f, "flex mechanism requires at least one demand"),
            Self::DuplicateSlotId(id) => write!(f, "duplicate flex slot identity {id}"),
            Self::EmptyPhysicalResourceId(id) => {
                write!(f, "flex slot {id} has an empty physical resource identity")
            }
            Self::InvalidSlotWindow(id) => write!(f, "flex slot {id} has an invalid time window"),
            Self::OverlappingSlotsOnPhysicalResource {
                physical_resource_id,
                first_slot_id,
                second_slot_id,
            } => write!(
                f,
                "slots {first_slot_id}/{second_slot_id} overlap on resource {physical_resource_id}"
            ),
            Self::DuplicateDemandId(id) => write!(f, "duplicate flex demand identity {id}"),
            Self::InvalidDemand(id) => write!(f, "demand {id} has invalid identity or quantity"),
            Self::UnknownDemandSlot { demand_id, slot_id } => {
                write!(f, "demand {demand_id} references unknown slot {slot_id}")
            }
            Self::BaselineCapacityExceeded { slot_id, quantity, capacity } => write!(
                f,
                "baseline slot {slot_id} uses {quantity} units but has capacity {capacity}"
            ),
            Self::DuplicateOfferId(id) => write!(f, "duplicate flex offer identity {id}"),
            Self::DuplicateParticipantOffer(id) => write!(
                f,
                "participant {id} has multiple offers; v0.1 allows one"
            ),
            Self::EmptyOfferIdentity(id) => write!(f, "offer {id} has invalid identity"),
            Self::UnknownOfferDemand { offer_id, demand_id } => {
                write!(f, "offer {offer_id} references unknown demand {demand_id}")
            }
            Self::OfferParticipantMismatch(id) => {
                write!(f, "offer {id} participant does not match its demand")
            }
            Self::UnknownOfferTarget { offer_id, slot_id } => {
                write!(f, "offer {offer_id} references unknown target slot {slot_id}")
            }
            Self::TooManyEligibleOffers { count, limit } => write!(
                f,
                "eligible offer count {count} exceeds the declared limit {limit}"
            ),
            Self::SearchBudgetExceeded { limit } => {
                write!(f, "flex mechanism exceeded its {limit}-node search budget")
            }
            Self::NoFeasibleAllocation => write!(f, "no feasible flexibility allocation exists"),
            Self::ArithmeticOverflow => write!(f, "flex mechanism arithmetic overflowed"),
            Self::NegativeOrUnrepresentablePivotPayment(id) => write!(
                f,
                "pivot payment for selected offer {id} is negative or unrepresentable"
            ),
        }
    }
}

impl std::error::Error for FlexMechanismError {}

#[derive(Clone)]
struct PreparedInput {
    time_profile_id: String,
    currency_id: String,
    slots: BTreeMap<String, FlexMechanismSlot>,
    demands: BTreeMap<String, FlexMechanismDemand>,
    eligible_offers: Vec<FlexMechanismOffer>,
    excluded_offers: Vec<FlexExcludedOffer>,
    buyer_budget_limit_minor_units: Option<u64>,
    limits: FlexMechanismLimits,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct EvaluatedPlan {
    selected_offer_ids: Vec<String>,
    gross_benefit_minor_units: u64,
    reported_cost_minor_units: u64,
    surplus_minor_units: i128,
    slot_usage: Vec<FlexMechanismSlotUsage>,
}

/// Selects a welfare-maximizing feasible subset of consented flexibility offers
/// and calculates pivot payments over the same finite eligible offer set.
///
/// The returned allocation and payments are advisory. The function does not
/// activate an offer, change a schedule, reserve a resource or settle a payment.
pub fn select_consent_based_flexibility(
    input: &FlexMechanismInput,
) -> Result<FlexMechanismReport, FlexMechanismError> {
    let prepared = prepare_input(input)?;
    if prepared.eligible_offers.len() > prepared.limits.max_eligible_offers {
        return Err(FlexMechanismError::TooManyEligibleOffers {
            count: prepared.eligible_offers.len(),
            limit: prepared.limits.max_eligible_offers,
        });
    }

    let mut search_steps = 0usize;
    let chosen = search_best_plan(
        &prepared,
        &prepared.eligible_offers,
        &mut search_steps,
    )?;

    let selected = chosen
        .selected_offer_ids
        .iter()
        .map(|id| {
            prepared
                .eligible_offers
                .iter()
                .find(|offer| &offer.id == id)
                .cloned()
                .ok_or(FlexMechanismError::ArithmeticOverflow)
        })
        .collect::<Result<Vec<_>, _>>()?;

    let mut payments = Vec::with_capacity(selected.len());
    let mut total_payment = 0u64;
    for selected_offer in &selected {
        let alternatives = prepared
            .eligible_offers
            .iter()
            .filter(|offer| offer.id != selected_offer.id)
            .cloned()
            .collect::<Vec<_>>();
        let without = search_best_plan(&prepared, &alternatives, &mut search_steps)?;
        let other_reported_cost = chosen
            .reported_cost_minor_units
            .checked_sub(selected_offer.reported_cost_minor_units)
            .ok_or(FlexMechanismError::ArithmeticOverflow)?;
        let payment_i128 = i128::from(chosen.gross_benefit_minor_units)
            .checked_sub(i128::from(other_reported_cost))
            .and_then(|value| value.checked_sub(without.surplus_minor_units))
            .ok_or(FlexMechanismError::ArithmeticOverflow)?;
        let payment = u64::try_from(payment_i128).map_err(|_| {
            FlexMechanismError::NegativeOrUnrepresentablePivotPayment(
                selected_offer.id.clone(),
            )
        })?;
        if payment < selected_offer.reported_cost_minor_units {
            return Err(FlexMechanismError::NegativeOrUnrepresentablePivotPayment(
                selected_offer.id.clone(),
            ));
        }
        total_payment = total_payment
            .checked_add(payment)
            .ok_or(FlexMechanismError::ArithmeticOverflow)?;
        payments.push(FlexMechanismPayment {
            offer_id: selected_offer.id.clone(),
            participant_id: selected_offer.participant_id.clone(),
            reported_cost_minor_units: selected_offer.reported_cost_minor_units,
            payment_minor_units: payment,
            counterfactual_welfare_minor_units: without.surplus_minor_units,
        });
    }
    payments.sort_by(|a, b| a.participant_id.cmp(&b.participant_id));

    let budget_shortfall = prepared
        .buyer_budget_limit_minor_units
        .map(|budget| total_payment.saturating_sub(budget));

    Ok(FlexMechanismReport {
        time_profile_id: prepared.time_profile_id,
        currency_id: prepared.currency_id,
        selected_offer_ids: chosen.selected_offer_ids,
        gross_benefit_minor_units: chosen.gross_benefit_minor_units,
        reported_cost_minor_units: chosen.reported_cost_minor_units,
        total_surplus_minor_units: chosen.surplus_minor_units,
        total_recommended_compensation_minor_units: total_payment,
        buyer_budget_shortfall_minor_units: budget_shortfall,
        payments,
        excluded_offers: prepared.excluded_offers,
        slot_usage: chosen.slot_usage,
        search_steps,
        optimality_scope: FlexOptimalityScope::ExactOverEligibleOfferSet,
        search_complete: true,
    })
}

fn prepare_input(input: &FlexMechanismInput) -> Result<PreparedInput, FlexMechanismError> {
    if input.time_profile_id.trim().is_empty() {
        return Err(FlexMechanismError::MissingTimeProfile);
    }
    if input.currency_id.trim().is_empty() {
        return Err(FlexMechanismError::MissingCurrency);
    }
    if input.slots.is_empty() {
        return Err(FlexMechanismError::EmptySlots);
    }
    if input.demands.is_empty() {
        return Err(FlexMechanismError::EmptyDemands);
    }
    if input.limits.max_search_steps == 0 || input.limits.max_eligible_offers == 0 {
        return Err(FlexMechanismError::InvalidSearchLimits);
    }

    let mut slots = BTreeMap::new();
    for slot in &input.slots {
        if slot.id.trim().is_empty() {
            return Err(FlexMechanismError::DuplicateSlotId(slot.id.clone()));
        }
        if slot.physical_resource_id.trim().is_empty() {
            return Err(FlexMechanismError::EmptyPhysicalResourceId(slot.id.clone()));
        }
        if slot.start_minute >= slot.end_minute {
            return Err(FlexMechanismError::InvalidSlotWindow(slot.id.clone()));
        }
        if slots.insert(slot.id.clone(), slot.clone()).is_some() {
            return Err(FlexMechanismError::DuplicateSlotId(slot.id.clone()));
        }
    }

    let mut slots_by_resource = input.slots.clone();
    slots_by_resource.sort_by(|a, b| {
        a.physical_resource_id
            .cmp(&b.physical_resource_id)
            .then(a.start_minute.cmp(&b.start_minute))
            .then(a.end_minute.cmp(&b.end_minute))
            .then(a.id.cmp(&b.id))
    });
    for pair in slots_by_resource.windows(2) {
        if pair[0].physical_resource_id == pair[1].physical_resource_id
            && pair[1].start_minute < pair[0].end_minute
        {
            return Err(FlexMechanismError::OverlappingSlotsOnPhysicalResource {
                physical_resource_id: pair[0].physical_resource_id.clone(),
                first_slot_id: pair[0].id.clone(),
                second_slot_id: pair[1].id.clone(),
            });
        }
    }

    let mut demands = BTreeMap::new();
    let mut baseline_usage = BTreeMap::<String, u64>::new();
    for demand in &input.demands {
        if demand.id.trim().is_empty()
            || demand.participant_id.trim().is_empty()
            || demand.quantity == 0
        {
            return Err(FlexMechanismError::InvalidDemand(demand.id.clone()));
        }
        let Some(slot) = slots.get(&demand.baseline_slot_id) else {
            return Err(FlexMechanismError::UnknownDemandSlot {
                demand_id: demand.id.clone(),
                slot_id: demand.baseline_slot_id.clone(),
            });
        };
        if slot.start_minute > demand.hard_latest_start_minute {
            return Err(FlexMechanismError::InvalidDemand(demand.id.clone()));
        }
        if demands.insert(demand.id.clone(), demand.clone()).is_some() {
            return Err(FlexMechanismError::DuplicateDemandId(demand.id.clone()));
        }
        let previous = baseline_usage
            .get(&demand.baseline_slot_id)
            .copied()
            .unwrap_or(0);
        let next = previous
            .checked_add(demand.quantity)
            .ok_or(FlexMechanismError::ArithmeticOverflow)?;
        baseline_usage.insert(demand.baseline_slot_id.clone(), next);
    }
    for (slot_id, quantity) in &baseline_usage {
        let capacity = slots
            .get(slot_id)
            .ok_or(FlexMechanismError::UnknownDemandSlot {
                demand_id: String::new(),
                slot_id: slot_id.clone(),
            })?
            .capacity;
        if *quantity > capacity {
            return Err(FlexMechanismError::BaselineCapacityExceeded {
                slot_id: slot_id.clone(),
                quantity: *quantity,
                capacity,
            });
        }
    }

    let mut seen_offer_ids = BTreeSet::new();
    let mut seen_participants = BTreeSet::new();
    for offer in &input.offers {
        if offer.id.trim().is_empty() || offer.participant_id.trim().is_empty() {
            return Err(FlexMechanismError::EmptyOfferIdentity(offer.id.clone()));
        }
        if !seen_offer_ids.insert(offer.id.clone()) {
            return Err(FlexMechanismError::DuplicateOfferId(offer.id.clone()));
        }
        if !seen_participants.insert(offer.participant_id.clone()) {
            return Err(FlexMechanismError::DuplicateParticipantOffer(
                offer.participant_id.clone(),
            ));
        }
        let Some(demand) = demands.get(&offer.demand_id) else {
            return Err(FlexMechanismError::UnknownOfferDemand {
                offer_id: offer.id.clone(),
                demand_id: offer.demand_id.clone(),
            });
        };
        if demand.participant_id != offer.participant_id {
            return Err(FlexMechanismError::OfferParticipantMismatch(
                offer.id.clone(),
            ));
        }
        if !slots.contains_key(&offer.target_slot_id) {
            return Err(FlexMechanismError::UnknownOfferTarget {
                offer_id: offer.id.clone(),
                slot_id: offer.target_slot_id.clone(),
            });
        }
    }

    let mut eligible_offers = Vec::new();
    let mut excluded_offers = Vec::new();
    for offer in &input.offers {
        let demand = demands
            .get(&offer.demand_id)
            .ok_or_else(|| FlexMechanismError::UnknownOfferDemand {
                offer_id: offer.id.clone(),
                demand_id: offer.demand_id.clone(),
            })?;
        let baseline_slot = slots
            .get(&demand.baseline_slot_id)
            .ok_or_else(|| FlexMechanismError::UnknownDemandSlot {
                demand_id: demand.id.clone(),
                slot_id: demand.baseline_slot_id.clone(),
            })?;
        let target_slot = slots
            .get(&offer.target_slot_id)
            .ok_or_else(|| FlexMechanismError::UnknownOfferTarget {
                offer_id: offer.id.clone(),
                slot_id: offer.target_slot_id.clone(),
            })?;

        let exclusion = match offer.state {
            FlexOfferState::Revoked => Some(FlexOfferExclusionReason::Revoked),
            FlexOfferState::Offered => Some(FlexOfferExclusionReason::NotAcceptedConditional),
            FlexOfferState::AcceptedConditional if offer.accepted_at_minute.is_none() => {
                Some(FlexOfferExclusionReason::AcceptanceReceiptMissing)
            }
            FlexOfferState::AcceptedConditional
                if offer
                    .accepted_at_minute
                    .is_some_and(|accepted_at| accepted_at > input.decision_time_minute) =>
            {
                Some(FlexOfferExclusionReason::AcceptanceAfterDecision)
            }
            FlexOfferState::AcceptedConditional
                if offer.expires_at_minute <= input.decision_time_minute =>
            {
                Some(FlexOfferExclusionReason::ExpiredAtDecision)
            }
            FlexOfferState::AcceptedConditional
                if offer.target_slot_id == demand.baseline_slot_id =>
            {
                Some(FlexOfferExclusionReason::SameSlotAsBaseline)
            }
            FlexOfferState::AcceptedConditional
                if baseline_slot.start_minute.abs_diff(target_slot.start_minute)
                    > offer.max_deviation_minutes =>
            {
                Some(FlexOfferExclusionReason::ShiftExceedsDeclaredLimit)
            }
            FlexOfferState::AcceptedConditional
                if target_slot.start_minute > demand.hard_latest_start_minute =>
            {
                Some(FlexOfferExclusionReason::HardDeadlineWouldBeMissed)
            }
            FlexOfferState::AcceptedConditional if offer.benefit_evidence_id.trim().is_empty() => {
                Some(FlexOfferExclusionReason::BenefitEvidenceReferenceMissing)
            }
            FlexOfferState::AcceptedConditional => None,
        };
        if let Some(reason) = exclusion {
            excluded_offers.push(FlexExcludedOffer {
                offer_id: offer.id.clone(),
                reason,
            });
        } else {
            eligible_offers.push(offer.clone());
        }
    }
    eligible_offers.sort_by(|a, b| a.id.cmp(&b.id));
    excluded_offers.sort_by(|a, b| a.offer_id.cmp(&b.offer_id));

    Ok(PreparedInput {
        time_profile_id: input.time_profile_id.clone(),
        currency_id: input.currency_id.clone(),
        slots,
        demands,
        eligible_offers,
        excluded_offers,
        buyer_budget_limit_minor_units: input.buyer_budget_limit_minor_units,
        limits: input.limits,
    })
}

fn search_best_plan(
    prepared: &PreparedInput,
    offers: &[FlexMechanismOffer],
    search_steps: &mut usize,
) -> Result<EvaluatedPlan, FlexMechanismError> {
    let mut best = None;
    let mut chosen_indices = Vec::new();
    enumerate_offer_subsets(
        prepared,
        offers,
        0,
        &mut chosen_indices,
        &mut best,
        search_steps,
    )?;
    best.ok_or(FlexMechanismError::NoFeasibleAllocation)
}

fn enumerate_offer_subsets(
    prepared: &PreparedInput,
    offers: &[FlexMechanismOffer],
    index: usize,
    chosen_indices: &mut Vec<usize>,
    best: &mut Option<EvaluatedPlan>,
    search_steps: &mut usize,
) -> Result<(), FlexMechanismError> {
    if *search_steps >= prepared.limits.max_search_steps {
        return Err(FlexMechanismError::SearchBudgetExceeded {
            limit: prepared.limits.max_search_steps,
        });
    }
    *search_steps += 1;

    if index == offers.len() {
        let chosen = chosen_indices
            .iter()
            .map(|chosen| &offers[*chosen])
            .collect::<Vec<_>>();
        let Some(plan) = evaluate_subset(prepared, &chosen)? else {
            return Ok(());
        };
        let replace = match best {
            None => true,
            Some(current) => {
                plan.surplus_minor_units > current.surplus_minor_units
                    || (plan.surplus_minor_units == current.surplus_minor_units
                        && plan.selected_offer_ids < current.selected_offer_ids)
            }
        };
        if replace {
            *best = Some(plan);
        }
        return Ok(());
    }

    // Exclusion branch first makes the empty offer set the deterministic
    // winner when it ties any zero-surplus non-empty set.
    enumerate_offer_subsets(
        prepared,
        offers,
        index + 1,
        chosen_indices,
        best,
        search_steps,
    )?;
    chosen_indices.push(index);
    let included = enumerate_offer_subsets(
        prepared,
        offers,
        index + 1,
        chosen_indices,
        best,
        search_steps,
    );
    chosen_indices.pop();
    included
}

fn evaluate_subset(
    prepared: &PreparedInput,
    chosen: &[&FlexMechanismOffer],
) -> Result<Option<EvaluatedPlan>, FlexMechanismError> {
    let mut occupancy = prepared
        .slots
        .keys()
        .map(|id| (id.clone(), 0u64))
        .collect::<BTreeMap<_, _>>();
    for demand in prepared.demands.values() {
        let previous = occupancy
            .get(&demand.baseline_slot_id)
            .copied()
            .unwrap_or(0);
        let next = previous
            .checked_add(demand.quantity)
            .ok_or(FlexMechanismError::ArithmeticOverflow)?;
        occupancy.insert(demand.baseline_slot_id.clone(), next);
    }

    let mut gross_benefit = 0u64;
    let mut reported_cost = 0u64;
    let mut selected_offer_ids = Vec::with_capacity(chosen.len());
    let mut moved_demands = BTreeSet::new();
    for offer in chosen {
        if !moved_demands.insert(offer.demand_id.clone()) {
            return Ok(None);
        }
        let demand = prepared
            .demands
            .get(&offer.demand_id)
            .ok_or_else(|| FlexMechanismError::UnknownOfferDemand {
                offer_id: offer.id.clone(),
                demand_id: offer.demand_id.clone(),
            })?;
        let from_previous = occupancy
            .get(&demand.baseline_slot_id)
            .copied()
            .unwrap_or(0);
        let from_next = from_previous
            .checked_sub(demand.quantity)
            .ok_or(FlexMechanismError::ArithmeticOverflow)?;
        occupancy.insert(demand.baseline_slot_id.clone(), from_next);

        let to_previous = occupancy
            .get(&offer.target_slot_id)
            .copied()
            .unwrap_or(0);
        let to_next = to_previous
            .checked_add(demand.quantity)
            .ok_or(FlexMechanismError::ArithmeticOverflow)?;
        occupancy.insert(offer.target_slot_id.clone(), to_next);

        gross_benefit = gross_benefit
            .checked_add(offer.gross_benefit_minor_units)
            .ok_or(FlexMechanismError::ArithmeticOverflow)?;
        reported_cost = reported_cost
            .checked_add(offer.reported_cost_minor_units)
            .ok_or(FlexMechanismError::ArithmeticOverflow)?;
        selected_offer_ids.push(offer.id.clone());
    }

    for (slot_id, quantity) in &occupancy {
        let slot = prepared
            .slots
            .get(slot_id)
            .ok_or_else(|| FlexMechanismError::UnknownOfferTarget {
                offer_id: String::new(),
                slot_id: slot_id.clone(),
            })?;
        if *quantity > slot.capacity {
            return Ok(None);
        }
    }

    selected_offer_ids.sort();
    let surplus = i128::from(gross_benefit)
        .checked_sub(i128::from(reported_cost))
        .ok_or(FlexMechanismError::ArithmeticOverflow)?;
    let slot_usage = prepared
        .slots
        .values()
        .map(|slot| {
            let baseline_quantity = prepared
                .demands
                .values()
                .filter(|demand| demand.baseline_slot_id == slot.id)
                .try_fold(0u64, |total, demand| total.checked_add(demand.quantity))
                .ok_or(FlexMechanismError::ArithmeticOverflow)?;
            let final_quantity = occupancy.get(&slot.id).copied().unwrap_or(0);
            Ok(FlexMechanismSlotUsage {
                slot_id: slot.id.clone(),
                physical_resource_id: slot.physical_resource_id.clone(),
                baseline_quantity,
                final_quantity,
                capacity: slot.capacity,
            })
        })
        .collect::<Result<Vec<_>, FlexMechanismError>>()?;

    Ok(Some(EvaluatedPlan {
        selected_offer_ids,
        gross_benefit_minor_units: gross_benefit,
        reported_cost_minor_units: reported_cost,
        surplus_minor_units: surplus,
        slot_usage,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_input() -> FlexMechanismInput {
        FlexMechanismInput {
            time_profile_id: "synthetic-UTC-minute-v1".to_owned(),
            currency_id: "SYNTH-MINOR-UNIT".to_owned(),
            decision_time_minute: 80,
            slots: vec![
                FlexMechanismSlot {
                    id: "EARLY".to_owned(),
                    physical_resource_id: "BERTH-A".to_owned(),
                    start_minute: 100,
                    end_minute: 160,
                    capacity: 2,
                },
                FlexMechanismSlot {
                    id: "LATE".to_owned(),
                    physical_resource_id: "BERTH-B".to_owned(),
                    start_minute: 160,
                    end_minute: 220,
                    capacity: 1,
                },
            ],
            demands: vec![
                FlexMechanismDemand {
                    id: "CALL-A".to_owned(),
                    participant_id: "SHIPPER-A".to_owned(),
                    baseline_slot_id: "EARLY".to_owned(),
                    quantity: 1,
                    hard_latest_start_minute: 220,
                },
                FlexMechanismDemand {
                    id: "CALL-B".to_owned(),
                    participant_id: "SHIPPER-B".to_owned(),
                    baseline_slot_id: "EARLY".to_owned(),
                    quantity: 1,
                    hard_latest_start_minute: 220,
                },
            ],
            offers: vec![
                FlexMechanismOffer {
                    id: "OFFER-A".to_owned(),
                    participant_id: "SHIPPER-A".to_owned(),
                    demand_id: "CALL-A".to_owned(),
                    target_slot_id: "LATE".to_owned(),
                    state: FlexOfferState::AcceptedConditional,
                    accepted_at_minute: Some(70),
                    expires_at_minute: 100,
                    max_deviation_minutes: 60,
                    reported_cost_minor_units: 2,
                    gross_benefit_minor_units: 10,
                    benefit_evidence_id: "synthetic-benefit-A".to_owned(),
                },
                FlexMechanismOffer {
                    id: "OFFER-B".to_owned(),
                    participant_id: "SHIPPER-B".to_owned(),
                    demand_id: "CALL-B".to_owned(),
                    target_slot_id: "LATE".to_owned(),
                    state: FlexOfferState::AcceptedConditional,
                    expires_at_minute: 100,
                    max_deviation_minutes: 60,
                    reported_cost_minor_units: 4,
                    gross_benefit_minor_units: 10,
                    benefit_evidence_id: "synthetic-benefit-B".to_owned(),
                },
            ],
            buyer_budget_limit_minor_units: Some(3),
            limits: FlexMechanismLimits::default(),
        }
    }

    /// Independent bit-mask oracle for tiny instances. It intentionally does
    /// not call the production recursive selector or subset evaluator.
    fn independent_oracle(
        input: &FlexMechanismInput,
        excluded_offer_id: Option<&str>,
    ) -> (Vec<String>, i128) {
        let slots = input
            .slots
            .iter()
            .map(|slot| (slot.id.as_str(), slot))
            .collect::<BTreeMap<_, _>>();
        let demands = input
            .demands
            .iter()
            .map(|demand| (demand.id.as_str(), demand))
            .collect::<BTreeMap<_, _>>();
        let mut offers = input
            .offers
            .iter()
            .filter(|offer| {
                Some(offer.id.as_str()) != excluded_offer_id
                    && offer.state == FlexOfferState::AcceptedConditional
                    && offer
                        .accepted_at_minute
                        .is_some_and(|accepted_at| accepted_at <= input.decision_time_minute)
                    && offer.expires_at_minute > input.decision_time_minute
                    && !offer.benefit_evidence_id.trim().is_empty()
                    && offers_match_oracle(input, offer, &slots, &demands)
            })
            .collect::<Vec<_>>();
        offers.sort_by(|a, b| a.id.cmp(&b.id));
        assert!(offers.len() < usize::BITS as usize);
        let mut best: Option<(Vec<String>, i128)> = None;
        for mask in 0usize..(1usize << offers.len()) {
            let mut chosen = Vec::new();
            let mut moved_demands = BTreeSet::new();
            let mut valid = true;
            let mut benefit = 0i128;
            let mut cost = 0i128;
            for (index, offer) in offers.iter().enumerate() {
                if (mask & (1usize << index)) == 0 {
                    continue;
                }
                if !moved_demands.insert(offer.demand_id.as_str()) {
                    valid = false;
                    break;
                }
                let demand = demands[offer.demand_id.as_str()];
                let baseline = slots[demand.baseline_slot_id.as_str()];
                let target = slots[offer.target_slot_id.as_str()];
                if baseline.start_minute.abs_diff(target.start_minute)
                    > offer.max_deviation_minutes
                    || target.start_minute > demand.hard_latest_start_minute
                {
                    valid = false;
                    break;
                }
                chosen.push(offer.id.clone());
                benefit += i128::from(offer.gross_benefit_minor_units);
                cost += i128::from(offer.reported_cost_minor_units);
            }
            if !valid {
                continue;
            }
            let mut final_quantities = BTreeMap::<&str, u64>::new();
            for demand in &input.demands {
                *final_quantities.entry(demand.baseline_slot_id.as_str()).or_default() +=
                    demand.quantity;
            }
            for offer in offers.iter().filter(|offer| chosen.contains(&offer.id)) {
                let demand = demands[offer.demand_id.as_str()];
                *final_quantities.entry(demand.baseline_slot_id.as_str()).or_default() -=
                    demand.quantity;
                *final_quantities.entry(offer.target_slot_id.as_str()).or_default() +=
                    demand.quantity;
            }
            for (slot_id, quantity) in &final_quantities {
                if *quantity > slots[*slot_id].capacity {
                    valid = false;
                }
            }
            if !valid {
                continue;
            }
            chosen.sort();
            let welfare = benefit - cost;
            let replace = match &best {
                None => true,
                Some((best_ids, best_welfare)) => {
                    welfare > *best_welfare || (welfare == *best_welfare && chosen < *best_ids)
                }
            };
            if replace {
                best = Some((chosen, welfare));
            }
        }
        best.expect("baseline fixture is feasible").clone()
    }

    fn offers_match_oracle(
        input: &FlexMechanismInput,
        offer: &FlexMechanismOffer,
        slots: &BTreeMap<&str, &FlexMechanismSlot>,
        demands: &BTreeMap<&str, &FlexMechanismDemand>,
    ) -> bool {
        let Some(demand) = demands.get(offer.demand_id.as_str()) else {
            return false;
        };
        if demand.participant_id != offer.participant_id {
            return false;
        }
        let Some(baseline) = slots.get(demand.baseline_slot_id.as_str()) else {
            return false;
        };
        let Some(target) = slots.get(offer.target_slot_id.as_str()) else {
            return false;
        };
        offer.target_slot_id != demand.baseline_slot_id
            && baseline
                .start_minute
                .abs_diff(target.start_minute)
                <= offer.max_deviation_minutes
            && target.start_minute <= demand.hard_latest_start_minute
            && offer
                .accepted_at_minute
                .is_some_and(|accepted_at| accepted_at <= input.decision_time_minute)
            && input.decision_time_minute < offer.expires_at_minute
    }

    fn utility_for(
        report: &FlexMechanismReport,
        offer_id: &str,
        true_cost: u64,
    ) -> i128 {
        report
            .payments
            .iter()
            .find(|payment| payment.offer_id == offer_id)
            .map(|payment| {
                i128::from(payment.payment_minor_units) - i128::from(true_cost)
            })
            .unwrap_or(0)
    }

    #[test]
    fn selects_highest_surplus_offer_and_reports_pivot_payment() {
        let input = base_input();
        let report = select_consent_based_flexibility(&input).unwrap();
        assert_eq!(report.selected_offer_ids, vec!["OFFER-A".to_owned()]);
        assert_eq!(report.gross_benefit_minor_units, 10);
        assert_eq!(report.reported_cost_minor_units, 2);
        assert_eq!(report.total_surplus_minor_units, 8);
        assert_eq!(report.payments.len(), 1);
        assert_eq!(report.payments[0].payment_minor_units, 4);
        assert_eq!(report.payments[0].counterfactual_welfare_minor_units, 6);
        assert_eq!(report.buyer_budget_shortfall_minor_units, Some(1));
        assert_eq!(
            report
                .slot_usage
                .iter()
                .find(|slot| slot.slot_id == "LATE")
                .unwrap()
                .final_quantity,
            1
        );
    }

    #[test]
    fn allocation_matches_independent_subset_oracle() {
        let input = base_input();
        let expected = independent_oracle(&input, None);
        let report = select_consent_based_flexibility(&input).unwrap();
        assert_eq!(report.selected_offer_ids, expected.0);
        assert_eq!(report.total_surplus_minor_units, expected.1);
        for payment in &report.payments {
            let (without_ids, welfare_without) =
                independent_oracle(&input, Some(payment.offer_id.as_str()));
            assert_eq!(payment.counterfactual_welfare_minor_units, welfare_without);
            let selected_other_cost = report
                .reported_cost_minor_units
                .checked_sub(payment.reported_cost_minor_units)
                .unwrap();
            let gross = i128::from(report.gross_benefit_minor_units);
            let expected_payment = gross - i128::from(selected_other_cost) - welfare_without;
            assert_eq!(i128::from(payment.payment_minor_units), expected_payment);
            assert!(!without_ids.contains(&payment.offer_id));
        }
    }

    #[test]
    fn unaccepted_revoked_and_expired_offers_never_enter_the_allocation() {
        let mut input = base_input();
        input.offers[0].state = FlexOfferState::Offered;
        input.offers[1].state = FlexOfferState::Revoked;
        let report = select_consent_based_flexibility(&input).unwrap();
        assert!(report.selected_offer_ids.is_empty());
        assert_eq!(report.excluded_offers.len(), 2);
        assert_eq!(report.total_recommended_compensation_minor_units, 0);

        let mut input = base_input();
        input.offers[0].expires_at_minute = input.decision_time_minute;
        let report = select_consent_based_flexibility(&input).unwrap();
        assert!(!report.selected_offer_ids.contains(&"OFFER-A".to_owned()));
        assert!(report.excluded_offers.iter().any(|offer| {
            offer.offer_id == "OFFER-A"
                && offer.reason == FlexOfferExclusionReason::ExpiredAtDecision
        }));
    }

    #[test]
    fn future_or_missing_acceptance_receipts_are_not_admissible_at_decision_time() {
        let mut input = base_input();
        input.offers[0].accepted_at_minute = Some(input.decision_time_minute + 1);
        let report = select_consent_based_flexibility(&input).unwrap();
        assert!(!report.selected_offer_ids.contains(&"OFFER-A".to_owned()));
        assert!(report.excluded_offers.iter().any(|offer| {
            offer.offer_id == "OFFER-A"
                && offer.reason == FlexOfferExclusionReason::AcceptanceAfterDecision
        }));

        input.offers[0].accepted_at_minute = None;
        let report = select_consent_based_flexibility(&input).unwrap();
        assert!(!report.selected_offer_ids.contains(&"OFFER-A".to_owned()));
        assert!(report.excluded_offers.iter().any(|offer| {
            offer.offer_id == "OFFER-A"
                && offer.reason == FlexOfferExclusionReason::AcceptanceReceiptMissing
        }));
    }

    #[test]
    fn offer_cannot_exceed_shift_bound_or_hard_deadline() {
        let mut input = base_input();
        input.demands[0].hard_latest_start_minute = 150;
        input.offers[0].max_deviation_minutes = 30;
        let report = select_consent_based_flexibility(&input).unwrap();
        assert!(report.excluded_offers.iter().any(|offer| {
            offer.offer_id == "OFFER-A"
                && offer.reason == FlexOfferExclusionReason::ShiftExceedsDeclaredLimit
        }));
        assert!(!report.selected_offer_ids.contains(&"OFFER-A".to_owned()));

        input.offers[0].max_deviation_minutes = 60;
        let report = select_consent_based_flexibility(&input).unwrap();
        assert!(report.excluded_offers.iter().any(|offer| {
            offer.offer_id == "OFFER-A"
                && offer.reason == FlexOfferExclusionReason::HardDeadlineWouldBeMissed
        }));
    }

    #[test]
    fn buyer_budget_is_diagnostic_and_does_not_silently_change_the_allocation() {
        let budgeted = select_consent_based_flexibility(&base_input()).unwrap();
        let mut no_budget_input = base_input();
        no_budget_input.buyer_budget_limit_minor_units = None;
        let no_budget = select_consent_based_flexibility(&no_budget_input).unwrap();
        assert_eq!(budgeted.selected_offer_ids, no_budget.selected_offer_ids);
        assert_eq!(budgeted.payments, no_budget.payments);
        assert_eq!(budgeted.buyer_budget_shortfall_minor_units, Some(1));
        assert_eq!(no_budget.buyer_budget_shortfall_minor_units, None);
    }

    #[test]
    fn baseline_capacity_overrun_fails_closed_before_mechanism_selection() {
        let mut input = base_input();
        input.slots[0].capacity = 1;
        assert_eq!(
            select_consent_based_flexibility(&input),
            Err(FlexMechanismError::BaselineCapacityExceeded {
                slot_id: "EARLY".to_owned(),
                quantity: 2,
                capacity: 1,
            })
        );
    }

    #[test]
    fn same_physical_resource_overlapping_slot_definitions_fail_closed() {
        let mut input = base_input();
        input.slots[1].physical_resource_id = "BERTH-A".to_owned();
        input.slots[1].start_minute = 150;
        let result = select_consent_based_flexibility(&input);
        assert!(matches!(
            result,
            Err(FlexMechanismError::OverlappingSlotsOnPhysicalResource { .. })
        ));
    }

    #[test]
    fn pivot_mechanism_is_truthful_for_exhaustive_two_participant_cost_grid() {
        for true_cost_a in 0..=12u64 {
            for true_cost_b in 0..=12u64 {
                let mut truthful = base_input();
                truthful.buyer_budget_limit_minor_units = None;
                truthful.offers[0].reported_cost_minor_units = true_cost_a;
                truthful.offers[1].reported_cost_minor_units = true_cost_b;
                let truthful_report = select_consent_based_flexibility(&truthful).unwrap();
                let truthful_utility = utility_for(&truthful_report, "OFFER-A", true_cost_a);

                for reported_cost_a in 0..=12u64 {
                    let mut deviated = base_input();
                    deviated.buyer_budget_limit_minor_units = None;
                    deviated.offers[0].reported_cost_minor_units = reported_cost_a;
                    deviated.offers[1].reported_cost_minor_units = true_cost_b;
                    let deviated_report = select_consent_based_flexibility(&deviated).unwrap();
                    let deviated_utility =
                        utility_for(&deviated_report, "OFFER-A", true_cost_a);
                    assert!(
                        truthful_utility >= deviated_utility,
                        "A gains by misreport {true_cost_a}/{true_cost_b}/{reported_cost_a}"
                    );
                }

                for reported_cost_b in 0..=12u64 {
                    let mut deviated = base_input();
                    deviated.buyer_budget_limit_minor_units = None;
                    deviated.offers[0].reported_cost_minor_units = true_cost_a;
                    deviated.offers[1].reported_cost_minor_units = reported_cost_b;
                    let deviated_report = select_consent_based_flexibility(&deviated).unwrap();
                    let deviated_utility =
                        utility_for(&deviated_report, "OFFER-B", true_cost_b);
                    let truthful_b_utility =
                        utility_for(&truthful_report, "OFFER-B", true_cost_b);
                    assert!(
                        truthful_b_utility >= deviated_utility,
                        "B gains by misreport {true_cost_a}/{true_cost_b}/{reported_cost_b}"
                    );
                }
            }
        }
    }

    #[test]
    fn canonical_input_permutations_preserve_allocation_payments_and_digest_fields() {
        let input = base_input();
        let report = select_consent_based_flexibility(&input).unwrap();
        let mut permuted = input.clone();
        permuted.slots.reverse();
        permuted.demands.reverse();
        permuted.offers.reverse();
        let permuted_report = select_consent_based_flexibility(&permuted).unwrap();
        assert_eq!(report, permuted_report);
    }

    #[test]
    fn search_budget_exhaustion_returns_no_partial_payment_report() {
        let mut input = base_input();
        input.limits.max_search_steps = 2;
        assert_eq!(
            select_consent_based_flexibility(&input),
            Err(FlexMechanismError::SearchBudgetExceeded { limit: 2 })
        );
    }

    #[test]
    fn multiple_candidate_offers_from_one_participant_are_rejected_in_v01() {
        let mut input = base_input();
        let mut duplicate = input.offers[0].clone();
        duplicate.id = "OFFER-A-ALTERNATE".to_owned();
        duplicate.target_slot_id = "LATE".to_owned();
        input.offers.push(duplicate);
        assert_eq!(
            select_consent_based_flexibility(&input),
            Err(FlexMechanismError::DuplicateParticipantOffer(
                "SHIPPER-A".to_owned()
            ))
        );
    }
}
