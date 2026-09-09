// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Provider-attested topology and transit contracts.
//!
//! This crate deliberately does not compute orbital mechanics, route finding,
//! propulsion, gate physics, or vehicle dynamics. It records the bounded result
//! of an external travel provider so economy, projects, communications, and
//! civilization systems can share one causal space-time boundary.

use serde::{Deserialize, Serialize};
use std::{error::Error, fmt};
use symtropy_game_state::StableId;

/// Stable topology-path identity owned by an external routing/travel provider.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TopologyPathRef {
    /// Provider responsible for the path interpretation.
    pub provider_id: StableId,
    /// Provider-local stable path identity.
    pub path_id: StableId,
    /// Exact path origin.
    pub origin_location_id: StableId,
    /// Exact path destination.
    pub destination_location_id: StableId,
}

/// Extensible travel-mode descriptor. No mode has privileged engine semantics.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TransitMode {
    /// Domain such as `orbital`, `surface`, `gate`, `rail`, or scenario-specific text.
    pub namespace: String,
    /// Provider-defined method such as `chemical-transfer`, `torch`, `gate-hop`, etc.
    pub method: String,
}

/// One provider-attested leg of a journey.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransitLeg {
    /// Stable leg identity.
    pub id: StableId,
    /// Topology path traversed by this leg.
    pub path: TopologyPathRef,
    /// Extensible travel mode.
    pub mode: TransitMode,
    /// Earliest canonical tick at which this leg may depart.
    pub depart_not_before_tick: u64,
    /// Latest canonical tick at which this leg may depart, inclusive.
    pub depart_not_after_tick: u64,
    /// Earliest canonical arrival tick attested by the provider.
    pub earliest_arrival_tick: u64,
    /// Optional latest expected arrival tick. This is not a guarantee of success.
    pub latest_arrival_tick: Option<u64>,
    /// Optional exact carrier/vehicle asset for which the attestation was computed.
    pub carrier_asset_id: Option<StableId>,
    /// Provider-native record/solution identity used for audit or recomputation.
    pub provider_record_id: StableId,
    /// Causal-history event that admitted this provider result into authoritative history.
    pub source_event_id: StableId,
}

impl TransitLeg {
    /// Validates only structural/time consistency, never the provider's physics.
    pub fn validate(&self) -> Result<(), TransitError> {
        if self.path.origin_location_id == self.path.destination_location_id {
            return Err(TransitError::SameEndpoint(self.id.clone()));
        }
        if self.depart_not_after_tick < self.depart_not_before_tick {
            return Err(TransitError::InvalidDepartureWindow {
                leg_id: self.id.clone(),
                not_before: self.depart_not_before_tick,
                not_after: self.depart_not_after_tick,
            });
        }
        if self.earliest_arrival_tick < self.depart_not_before_tick {
            return Err(TransitError::ArrivalBeforeDepartureWindow {
                leg_id: self.id.clone(),
                earliest_arrival_tick: self.earliest_arrival_tick,
                depart_not_before_tick: self.depart_not_before_tick,
            });
        }
        if self
            .latest_arrival_tick
            .is_some_and(|latest| latest < self.earliest_arrival_tick)
        {
            return Err(TransitError::InvalidArrivalWindow {
                leg_id: self.id.clone(),
                earliest_arrival_tick: self.earliest_arrival_tick,
                latest_arrival_tick: self.latest_arrival_tick,
            });
        }
        Ok(())
    }

    /// Returns whether one exact departure tick is admitted by this attestation.
    pub fn admits_departure(&self, tick: u64) -> bool {
        tick >= self.depart_not_before_tick && tick <= self.depart_not_after_tick
    }
}

/// Immutable multi-leg travel plan admitted from one provider.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransitPlan {
    /// Stable plan identity.
    pub id: StableId,
    /// Provider responsible for interpreting every path in this plan.
    pub provider_id: StableId,
    /// Ordered journey legs.
    pub legs: Vec<TransitLeg>,
    /// Causal event that admitted the complete plan.
    pub source_event_id: StableId,
}

impl TransitPlan {
    /// Validates provider identity, endpoint continuity, and monotonic time windows.
    pub fn validate(&self) -> Result<(), TransitError> {
        if self.legs.is_empty() {
            return Err(TransitError::EmptyPlan(self.id.clone()));
        }

        for (index, leg) in self.legs.iter().enumerate() {
            leg.validate()?;
            if leg.path.provider_id != self.provider_id {
                return Err(TransitError::ProviderMismatch {
                    plan_id: self.id.clone(),
                    leg_id: leg.id.clone(),
                    expected_provider_id: self.provider_id.clone(),
                    actual_provider_id: leg.path.provider_id.clone(),
                });
            }
            if let Some(previous) = index.checked_sub(1).and_then(|i| self.legs.get(i)) {
                if previous.path.destination_location_id != leg.path.origin_location_id {
                    return Err(TransitError::DisconnectedLegs {
                        plan_id: self.id.clone(),
                        previous_leg_id: previous.id.clone(),
                        next_leg_id: leg.id.clone(),
                    });
                }
                if leg.depart_not_before_tick < previous.earliest_arrival_tick {
                    return Err(TransitError::NonMonotonicLegTime {
                        plan_id: self.id.clone(),
                        previous_leg_id: previous.id.clone(),
                        next_leg_id: leg.id.clone(),
                    });
                }
            }
        }
        Ok(())
    }

    /// Journey origin.
    pub fn origin_location_id(&self) -> &StableId {
        &self.legs[0].path.origin_location_id
    }

    /// Journey destination.
    pub fn destination_location_id(&self) -> &StableId {
        &self.legs[self.legs.len() - 1].path.destination_location_id
    }

    /// First admitted departure tick.
    pub fn depart_not_before_tick(&self) -> u64 {
        self.legs[0].depart_not_before_tick
    }

    /// Final provider-attested arrival floor.
    pub fn earliest_arrival_tick(&self) -> u64 {
        self.legs[self.legs.len() - 1].earliest_arrival_tick
    }

    /// Stable path identities traversed by the journey.
    pub fn topology_paths(&self) -> impl Iterator<Item = &TopologyPathRef> {
        self.legs.iter().map(|leg| &leg.path)
    }
}

/// Shipment/actor/vehicle-facing binding derived from an exact transit plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransitBinding {
    pub plan_id: StableId,
    pub provider_id: StableId,
    pub origin_location_id: StableId,
    pub destination_location_id: StableId,
    pub departed_tick: u64,
    pub earliest_arrival_tick: u64,
    pub carrier_asset_id: Option<StableId>,
}

impl TransitBinding {
    /// Binds one departure to a provider plan without recomputing travel physics.
    pub fn from_plan(
        plan: &TransitPlan,
        origin_location_id: &StableId,
        destination_location_id: &StableId,
        departed_tick: u64,
        carrier_asset_id: Option<&StableId>,
    ) -> Result<Self, TransitError> {
        plan.validate()?;
        if plan.origin_location_id() != origin_location_id
            || plan.destination_location_id() != destination_location_id
        {
            return Err(TransitError::PlanEndpointMismatch {
                plan_id: plan.id.clone(),
                requested_origin: origin_location_id.clone(),
                requested_destination: destination_location_id.clone(),
            });
        }
        let first = &plan.legs[0];
        if !first.admits_departure(departed_tick) {
            return Err(TransitError::DepartureOutsideWindow {
                plan_id: plan.id.clone(),
                attempted_tick: departed_tick,
                not_before: first.depart_not_before_tick,
                not_after: first.depart_not_after_tick,
            });
        }

        for leg in &plan.legs {
            if let Some(required_carrier) = &leg.carrier_asset_id
                && Some(required_carrier) != carrier_asset_id
            {
                return Err(TransitError::CarrierMismatch {
                    plan_id: plan.id.clone(),
                    leg_id: leg.id.clone(),
                    required_carrier_id: required_carrier.clone(),
                    supplied_carrier_id: carrier_asset_id.cloned(),
                });
            }
        }

        Ok(Self {
            plan_id: plan.id.clone(),
            provider_id: plan.provider_id.clone(),
            origin_location_id: origin_location_id.clone(),
            destination_location_id: destination_location_id.clone(),
            departed_tick,
            earliest_arrival_tick: plan.earliest_arrival_tick(),
            carrier_asset_id: carrier_asset_id.cloned(),
        })
    }
}

/// Structural errors in provider-attested transit records.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransitError {
    EmptyPlan(StableId),
    SameEndpoint(StableId),
    InvalidDepartureWindow {
        leg_id: StableId,
        not_before: u64,
        not_after: u64,
    },
    ArrivalBeforeDepartureWindow {
        leg_id: StableId,
        earliest_arrival_tick: u64,
        depart_not_before_tick: u64,
    },
    InvalidArrivalWindow {
        leg_id: StableId,
        earliest_arrival_tick: u64,
        latest_arrival_tick: Option<u64>,
    },
    ProviderMismatch {
        plan_id: StableId,
        leg_id: StableId,
        expected_provider_id: StableId,
        actual_provider_id: StableId,
    },
    DisconnectedLegs {
        plan_id: StableId,
        previous_leg_id: StableId,
        next_leg_id: StableId,
    },
    NonMonotonicLegTime {
        plan_id: StableId,
        previous_leg_id: StableId,
        next_leg_id: StableId,
    },
    PlanEndpointMismatch {
        plan_id: StableId,
        requested_origin: StableId,
        requested_destination: StableId,
    },
    DepartureOutsideWindow {
        plan_id: StableId,
        attempted_tick: u64,
        not_before: u64,
        not_after: u64,
    },
    CarrierMismatch {
        plan_id: StableId,
        leg_id: StableId,
        required_carrier_id: StableId,
        supplied_carrier_id: Option<StableId>,
    },
}

impl fmt::Display for TransitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyPlan(id) => write!(f, "transit plan {id} has no legs"),
            Self::SameEndpoint(id) => write!(f, "transit leg {id} has identical endpoints"),
            Self::InvalidDepartureWindow { leg_id, .. } => write!(f, "transit leg {leg_id} has an invalid departure window"),
            Self::ArrivalBeforeDepartureWindow { leg_id, .. } => write!(f, "transit leg {leg_id} arrives before its departure window begins"),
            Self::InvalidArrivalWindow { leg_id, .. } => write!(f, "transit leg {leg_id} has an invalid arrival window"),
            Self::ProviderMismatch { plan_id, leg_id, .. } => write!(f, "transit plan {plan_id} leg {leg_id} uses another provider"),
            Self::DisconnectedLegs { plan_id, previous_leg_id, next_leg_id } => write!(f, "transit plan {plan_id} disconnects {previous_leg_id} from {next_leg_id}"),
            Self::NonMonotonicLegTime { plan_id, previous_leg_id, next_leg_id } => write!(f, "transit plan {plan_id} schedules {next_leg_id} before {previous_leg_id} can arrive"),
            Self::PlanEndpointMismatch { plan_id, .. } => write!(f, "transit plan {plan_id} does not match requested journey endpoints"),
            Self::DepartureOutsideWindow { plan_id, attempted_tick, .. } => write!(f, "transit plan {plan_id} does not admit departure at tick {attempted_tick}"),
            Self::CarrierMismatch { plan_id, leg_id, .. } => write!(f, "transit plan {plan_id} leg {leg_id} requires another carrier"),
        }
    }
}

impl Error for TransitError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(value: &str) -> StableId {
        StableId::parse(value).expect("test id")
    }

    fn leg(
        id_value: &str,
        origin: &str,
        destination: &str,
        depart: u64,
        arrive: u64,
    ) -> TransitLeg {
        TransitLeg {
            id: id(id_value),
            path: TopologyPathRef {
                provider_id: id("provider:orbital"),
                path_id: id(&format!("path:{id_value}")),
                origin_location_id: id(origin),
                destination_location_id: id(destination),
            },
            mode: TransitMode {
                namespace: "orbital".into(),
                method: "provider-solution".into(),
            },
            depart_not_before_tick: depart,
            depart_not_after_tick: depart + 10,
            earliest_arrival_tick: arrive,
            latest_arrival_tick: Some(arrive + 20),
            carrier_asset_id: Some(id("asset:freighter")),
            provider_record_id: id(&format!("provider-record:{id_value}")),
            source_event_id: id(&format!("event:{id_value}")),
        }
    }

    #[test]
    fn connected_multi_leg_plan_validates_and_binds() {
        let plan = TransitPlan {
            id: id("plan:aster-vesper"),
            provider_id: id("provider:orbital"),
            legs: vec![
                leg("leg:a-hub", "loc:aster", "loc:hub", 100, 180),
                leg("leg:hub-v", "loc:hub", "loc:vesper", 190, 310),
            ],
            source_event_id: id("event:plan"),
        };
        plan.validate().expect("valid plan");
        let binding = TransitBinding::from_plan(
            &plan,
            &id("loc:aster"),
            &id("loc:vesper"),
            105,
            Some(&id("asset:freighter")),
        )
        .expect("bind plan");
        assert_eq!(binding.earliest_arrival_tick, 310);
        assert_eq!(plan.topology_paths().count(), 2);
    }

    #[test]
    fn disconnected_plan_fails_closed() {
        let plan = TransitPlan {
            id: id("plan:broken"),
            provider_id: id("provider:orbital"),
            legs: vec![
                leg("leg:a-b", "loc:a", "loc:b", 10, 20),
                leg("leg:c-d", "loc:c", "loc:d", 30, 40),
            ],
            source_event_id: id("event:broken"),
        };
        assert!(matches!(plan.validate(), Err(TransitError::DisconnectedLegs { .. })));
    }

    #[test]
    fn carrier_bound_plan_rejects_another_ship() {
        let plan = TransitPlan {
            id: id("plan:carrier"),
            provider_id: id("provider:orbital"),
            legs: vec![leg("leg:a-b", "loc:a", "loc:b", 10, 20)],
            source_event_id: id("event:carrier"),
        };
        assert!(matches!(
            TransitBinding::from_plan(
                &plan,
                &id("loc:a"),
                &id("loc:b"),
                12,
                Some(&id("asset:other")),
            ),
            Err(TransitError::CarrierMismatch { .. })
        ));
    }

    #[test]
    fn caller_cannot_depart_outside_provider_window() {
        let plan = TransitPlan {
            id: id("plan:window"),
            provider_id: id("provider:orbital"),
            legs: vec![leg("leg:a-b", "loc:a", "loc:b", 100, 200)],
            source_event_id: id("event:window"),
        };
        assert!(matches!(
            TransitBinding::from_plan(
                &plan,
                &id("loc:a"),
                &id("loc:b"),
                99,
                Some(&id("asset:freighter")),
            ),
            Err(TransitError::DepartureOutsideWindow { .. })
        ));
    }
}
