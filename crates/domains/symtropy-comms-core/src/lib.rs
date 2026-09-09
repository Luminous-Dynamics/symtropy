// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Delayed communication propagation over provider-owned topology paths.
//!
//! Transmission is not delivery. Delivery is not belief, recognition,
//! obedience, authority, or truth. This crate only owns message propagation
//! lifecycle and timing.

use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    error::Error,
    fmt,
};
use symtropy_game_state::StableId;
use symtropy_transit_core::TopologyPathRef;

/// Extensible signal/channel descriptor.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct CommunicationChannel {
    /// Domain such as `radio`, `laser`, `courier`, `relay`, or scenario-specific text.
    pub namespace: String,
    /// Provider-defined method.
    pub method: String,
}

/// Generic reference carried by a message without importing every possible domain.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct CommunicationPayloadRef {
    /// Owning namespace, e.g. `epistemic-assertion`, `authority-order`, `treaty`, `project`.
    pub namespace: String,
    /// Stable referenced record identity.
    pub record_id: StableId,
}

/// Provider-attested propagation timing across one ordered topology path chain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignalPropagationPlan {
    /// Stable plan identity.
    pub id: StableId,
    /// Signal/routing provider responsible for the propagation timing claim.
    pub provider_id: StableId,
    /// Ordered path identities reused from `symtropy-transit-core`.
    pub topology_paths: Vec<TopologyPathRef>,
    /// Communication channel/method.
    pub channel: CommunicationChannel,
    /// Earliest canonical send tick admitted by the provider.
    pub send_not_before_tick: u64,
    /// Latest canonical send tick admitted by the provider, inclusive.
    pub send_not_after_tick: u64,
    /// Earliest canonical delivery tick for an admitted send.
    pub earliest_delivery_tick: u64,
    /// Optional latest expected delivery tick. It is not a guarantee of success.
    pub latest_delivery_tick: Option<u64>,
    /// Provider-native result identity.
    pub provider_record_id: StableId,
    /// Causal event admitting the plan into authoritative history.
    pub source_event_id: StableId,
}

impl SignalPropagationPlan {
    /// Validates only structural topology/timing consistency, not signal physics.
    pub fn validate(&self) -> Result<(), CommunicationError> {
        if self.topology_paths.is_empty() {
            return Err(CommunicationError::EmptyPropagationPath(self.id.clone()));
        }
        if self.send_not_after_tick < self.send_not_before_tick {
            return Err(CommunicationError::InvalidSendWindow {
                plan_id: self.id.clone(),
                not_before: self.send_not_before_tick,
                not_after: self.send_not_after_tick,
            });
        }
        if self.earliest_delivery_tick < self.send_not_before_tick {
            return Err(CommunicationError::DeliveryBeforeSendWindow {
                plan_id: self.id.clone(),
                earliest_delivery_tick: self.earliest_delivery_tick,
                send_not_before_tick: self.send_not_before_tick,
            });
        }
        if self
            .latest_delivery_tick
            .is_some_and(|latest| latest < self.earliest_delivery_tick)
        {
            return Err(CommunicationError::InvalidDeliveryWindow {
                plan_id: self.id.clone(),
                earliest_delivery_tick: self.earliest_delivery_tick,
                latest_delivery_tick: self.latest_delivery_tick,
            });
        }
        for pair in self.topology_paths.windows(2) {
            if pair[0].destination_location_id != pair[1].origin_location_id {
                return Err(CommunicationError::DisconnectedTopologyPath {
                    plan_id: self.id.clone(),
                    left_path_id: pair[0].path_id.clone(),
                    right_path_id: pair[1].path_id.clone(),
                });
            }
        }
        Ok(())
    }

    pub fn origin_location_id(&self) -> &StableId {
        &self.topology_paths[0].origin_location_id
    }

    pub fn destination_location_id(&self) -> &StableId {
        &self.topology_paths[self.topology_paths.len() - 1].destination_location_id
    }

    pub fn admits_send(&self, tick: u64) -> bool {
        tick >= self.send_not_before_tick && tick <= self.send_not_after_tick
    }
}

/// Immutable authored message. Authorship does not mean transmission or delivery.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageEnvelope {
    pub id: StableId,
    pub sender_id: StableId,
    pub origin_location_id: StableId,
    pub destination_location_id: StableId,
    pub authored_tick: u64,
    pub payload_refs: Vec<CommunicationPayloadRef>,
    pub source_event_id: StableId,
}

/// One concrete transmission attempt of one message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageTransmission {
    pub id: StableId,
    pub message_id: StableId,
    pub propagation_plan_id: StableId,
    pub sent_tick: u64,
    pub source_event_id: StableId,
}

/// Successful destination availability record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageDelivery {
    pub id: StableId,
    pub transmission_id: StableId,
    pub delivered_tick: u64,
    /// Optional exact receiving actor/institution/device. Destination location still comes from the message.
    pub receiver_id: Option<StableId>,
    pub source_event_id: StableId,
}

/// Explicit failed/lost/intercepted transmission settlement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageLoss {
    pub id: StableId,
    pub transmission_id: StableId,
    pub lost_tick: u64,
    pub cause: String,
    pub source_event_id: StableId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransmissionState {
    InFlight,
    Delivered,
    Lost,
}

/// Headless propagation state. It does not write beliefs, orders, or institutional records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CommunicationWorld {
    plans: BTreeMap<StableId, SignalPropagationPlan>,
    messages: BTreeMap<StableId, MessageEnvelope>,
    transmissions: BTreeMap<StableId, MessageTransmission>,
    deliveries: BTreeMap<StableId, MessageDelivery>,
    delivery_by_transmission: BTreeMap<StableId, StableId>,
    losses: BTreeMap<StableId, MessageLoss>,
    loss_by_transmission: BTreeMap<StableId, StableId>,
}

impl CommunicationWorld {
    pub fn register_plan(&mut self, plan: SignalPropagationPlan) -> Result<(), CommunicationError> {
        if self.plans.contains_key(&plan.id) {
            return Err(CommunicationError::DuplicatePlan(plan.id));
        }
        plan.validate()?;
        self.plans.insert(plan.id.clone(), plan);
        Ok(())
    }

    pub fn author_message(&mut self, message: MessageEnvelope) -> Result<(), CommunicationError> {
        if self.messages.contains_key(&message.id) {
            return Err(CommunicationError::DuplicateMessage(message.id));
        }
        if message.origin_location_id == message.destination_location_id {
            return Err(CommunicationError::SameMessageEndpoint(message.id));
        }
        self.messages.insert(message.id.clone(), message);
        Ok(())
    }

    /// Starts a propagation attempt. No destination availability is created here.
    pub fn transmit(&mut self, transmission: MessageTransmission) -> Result<(), CommunicationError> {
        if self.transmissions.contains_key(&transmission.id) {
            return Err(CommunicationError::DuplicateTransmission(transmission.id));
        }
        let message = self
            .messages
            .get(&transmission.message_id)
            .ok_or_else(|| CommunicationError::UnknownMessage(transmission.message_id.clone()))?;
        let plan = self
            .plans
            .get(&transmission.propagation_plan_id)
            .ok_or_else(|| CommunicationError::UnknownPlan(transmission.propagation_plan_id.clone()))?;
        if message.origin_location_id != *plan.origin_location_id()
            || message.destination_location_id != *plan.destination_location_id()
        {
            return Err(CommunicationError::MessagePlanEndpointMismatch {
                message_id: message.id.clone(),
                plan_id: plan.id.clone(),
            });
        }
        if transmission.sent_tick < message.authored_tick {
            return Err(CommunicationError::SentBeforeAuthored {
                message_id: message.id.clone(),
                authored_tick: message.authored_tick,
                sent_tick: transmission.sent_tick,
            });
        }
        if !plan.admits_send(transmission.sent_tick) {
            return Err(CommunicationError::SendOutsideWindow {
                plan_id: plan.id.clone(),
                attempted_tick: transmission.sent_tick,
                not_before: plan.send_not_before_tick,
                not_after: plan.send_not_after_tick,
            });
        }
        self.transmissions.insert(transmission.id.clone(), transmission);
        Ok(())
    }

    /// Makes a message available at the destination only after the provider floor.
    pub fn deliver(&mut self, delivery: MessageDelivery) -> Result<(), CommunicationError> {
        if self.deliveries.contains_key(&delivery.id) {
            return Err(CommunicationError::DuplicateDelivery(delivery.id));
        }
        let transmission = self
            .transmissions
            .get(&delivery.transmission_id)
            .ok_or_else(|| CommunicationError::UnknownTransmission(delivery.transmission_id.clone()))?;
        if self.delivery_by_transmission.contains_key(&delivery.transmission_id)
            || self.loss_by_transmission.contains_key(&delivery.transmission_id)
        {
            return Err(CommunicationError::TransmissionAlreadySettled(delivery.transmission_id));
        }
        let plan = self
            .plans
            .get(&transmission.propagation_plan_id)
            .expect("registered transmission retains its plan");
        if delivery.delivered_tick < plan.earliest_delivery_tick {
            return Err(CommunicationError::DeliveredTooEarly {
                transmission_id: delivery.transmission_id,
                earliest_tick: plan.earliest_delivery_tick,
                attempted_tick: delivery.delivered_tick,
            });
        }
        self.delivery_by_transmission
            .insert(delivery.transmission_id.clone(), delivery.id.clone());
        self.deliveries.insert(delivery.id.clone(), delivery);
        Ok(())
    }

    pub fn lose(&mut self, loss: MessageLoss) -> Result<(), CommunicationError> {
        if self.losses.contains_key(&loss.id) {
            return Err(CommunicationError::DuplicateLoss(loss.id));
        }
        let transmission = self
            .transmissions
            .get(&loss.transmission_id)
            .ok_or_else(|| CommunicationError::UnknownTransmission(loss.transmission_id.clone()))?;
        if self.delivery_by_transmission.contains_key(&loss.transmission_id)
            || self.loss_by_transmission.contains_key(&loss.transmission_id)
        {
            return Err(CommunicationError::TransmissionAlreadySettled(loss.transmission_id));
        }
        if loss.lost_tick < transmission.sent_tick {
            return Err(CommunicationError::LossBeforeSend {
                transmission_id: transmission.id.clone(),
                sent_tick: transmission.sent_tick,
                lost_tick: loss.lost_tick,
            });
        }
        self.loss_by_transmission
            .insert(loss.transmission_id.clone(), loss.id.clone());
        self.losses.insert(loss.id.clone(), loss);
        Ok(())
    }

    pub fn transmission_state(&self, transmission_id: &StableId) -> Option<TransmissionState> {
        self.transmissions.get(transmission_id)?;
        if self.delivery_by_transmission.contains_key(transmission_id) {
            Some(TransmissionState::Delivered)
        } else if self.loss_by_transmission.contains_key(transmission_id) {
            Some(TransmissionState::Lost)
        } else {
            Some(TransmissionState::InFlight)
        }
    }

    /// Returns the message only after at least one concrete transmission has delivered.
    pub fn delivered_message(&self, message_id: &StableId) -> Option<&MessageEnvelope> {
        let message = self.messages.get(message_id)?;
        let delivered = self.transmissions.values().any(|transmission| {
            transmission.message_id == *message_id
                && self.delivery_by_transmission.contains_key(&transmission.id)
        });
        delivered.then_some(message)
    }

    pub fn plan(&self, plan_id: &StableId) -> Option<&SignalPropagationPlan> {
        self.plans.get(plan_id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommunicationError {
    EmptyPropagationPath(StableId),
    InvalidSendWindow { plan_id: StableId, not_before: u64, not_after: u64 },
    DeliveryBeforeSendWindow { plan_id: StableId, earliest_delivery_tick: u64, send_not_before_tick: u64 },
    InvalidDeliveryWindow { plan_id: StableId, earliest_delivery_tick: u64, latest_delivery_tick: Option<u64> },
    DisconnectedTopologyPath { plan_id: StableId, left_path_id: StableId, right_path_id: StableId },
    DuplicatePlan(StableId),
    DuplicateMessage(StableId),
    SameMessageEndpoint(StableId),
    DuplicateTransmission(StableId),
    UnknownMessage(StableId),
    UnknownPlan(StableId),
    MessagePlanEndpointMismatch { message_id: StableId, plan_id: StableId },
    SentBeforeAuthored { message_id: StableId, authored_tick: u64, sent_tick: u64 },
    SendOutsideWindow { plan_id: StableId, attempted_tick: u64, not_before: u64, not_after: u64 },
    DuplicateDelivery(StableId),
    UnknownTransmission(StableId),
    TransmissionAlreadySettled(StableId),
    DeliveredTooEarly { transmission_id: StableId, earliest_tick: u64, attempted_tick: u64 },
    DuplicateLoss(StableId),
    LossBeforeSend { transmission_id: StableId, sent_tick: u64, lost_tick: u64 },
}

impl fmt::Display for CommunicationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyPropagationPath(id) => write!(f, "communication plan {id} has no topology path"),
            Self::InvalidSendWindow { plan_id, .. } => write!(f, "communication plan {plan_id} has invalid send window"),
            Self::DeliveryBeforeSendWindow { plan_id, .. } => write!(f, "communication plan {plan_id} delivers before sending can begin"),
            Self::InvalidDeliveryWindow { plan_id, .. } => write!(f, "communication plan {plan_id} has invalid delivery window"),
            Self::DisconnectedTopologyPath { plan_id, .. } => write!(f, "communication plan {plan_id} has disconnected topology paths"),
            Self::DuplicatePlan(id) => write!(f, "communication plan {id} already exists"),
            Self::DuplicateMessage(id) => write!(f, "message {id} already exists"),
            Self::SameMessageEndpoint(id) => write!(f, "message {id} has identical endpoints"),
            Self::DuplicateTransmission(id) => write!(f, "transmission {id} already exists"),
            Self::UnknownMessage(id) => write!(f, "unknown message {id}"),
            Self::UnknownPlan(id) => write!(f, "unknown communication plan {id}"),
            Self::MessagePlanEndpointMismatch { message_id, plan_id } => write!(f, "message {message_id} does not match communication plan {plan_id} endpoints"),
            Self::SentBeforeAuthored { message_id, .. } => write!(f, "message {message_id} was sent before it was authored"),
            Self::SendOutsideWindow { plan_id, attempted_tick, .. } => write!(f, "communication plan {plan_id} does not admit send tick {attempted_tick}"),
            Self::DuplicateDelivery(id) => write!(f, "delivery {id} already exists"),
            Self::UnknownTransmission(id) => write!(f, "unknown transmission {id}"),
            Self::TransmissionAlreadySettled(id) => write!(f, "transmission {id} is already settled"),
            Self::DeliveredTooEarly { transmission_id, .. } => write!(f, "transmission {transmission_id} was delivered too early"),
            Self::DuplicateLoss(id) => write!(f, "communication loss {id} already exists"),
            Self::LossBeforeSend { transmission_id, .. } => write!(f, "transmission {transmission_id} was lost before it was sent"),
        }
    }
}

impl Error for CommunicationError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(value: &str) -> StableId {
        StableId::parse(value).expect("test id")
    }

    fn path(id_value: &str, origin: &str, destination: &str) -> TopologyPathRef {
        TopologyPathRef {
            provider_id: id("provider:topology"),
            path_id: id(id_value),
            origin_location_id: id(origin),
            destination_location_id: id(destination),
        }
    }

    fn plan() -> SignalPropagationPlan {
        SignalPropagationPlan {
            id: id("signal-plan:aster-vesper"),
            provider_id: id("provider:signal"),
            topology_paths: vec![
                path("path:a-hub", "loc:aster", "loc:hub"),
                path("path:hub-v", "loc:hub", "loc:vesper"),
            ],
            channel: CommunicationChannel {
                namespace: "laser-relay".into(),
                method: "provider-solution".into(),
            },
            send_not_before_tick: 100,
            send_not_after_tick: 120,
            earliest_delivery_tick: 130,
            latest_delivery_tick: Some(150),
            provider_record_id: id("provider-record:signal"),
            source_event_id: id("event:signal-plan"),
        }
    }

    fn message() -> MessageEnvelope {
        MessageEnvelope {
            id: id("message:order"),
            sender_id: id("institution:central"),
            origin_location_id: id("loc:aster"),
            destination_location_id: id("loc:vesper"),
            authored_tick: 98,
            payload_refs: vec![CommunicationPayloadRef {
                namespace: "authority-order".into(),
                record_id: id("order:17"),
            }],
            source_event_id: id("event:order-authored"),
        }
    }

    #[test]
    fn message_is_unavailable_until_delivery() {
        let mut world = CommunicationWorld::default();
        world.register_plan(plan()).expect("plan");
        world.author_message(message()).expect("message");
        world
            .transmit(MessageTransmission {
                id: id("tx:1"),
                message_id: id("message:order"),
                propagation_plan_id: id("signal-plan:aster-vesper"),
                sent_tick: 105,
                source_event_id: id("event:tx"),
            })
            .expect("transmit");
        assert!(world.delivered_message(&id("message:order")).is_none());
        world
            .deliver(MessageDelivery {
                id: id("delivery:1"),
                transmission_id: id("tx:1"),
                delivered_tick: 130,
                receiver_id: Some(id("governor:vesper")),
                source_event_id: id("event:delivery"),
            })
            .expect("deliver");
        assert!(world.delivered_message(&id("message:order")).is_some());
    }

    #[test]
    fn early_delivery_fails_closed() {
        let mut world = CommunicationWorld::default();
        world.register_plan(plan()).expect("plan");
        world.author_message(message()).expect("message");
        world
            .transmit(MessageTransmission {
                id: id("tx:1"),
                message_id: id("message:order"),
                propagation_plan_id: id("signal-plan:aster-vesper"),
                sent_tick: 105,
                source_event_id: id("event:tx"),
            })
            .expect("transmit");
        assert!(matches!(
            world.deliver(MessageDelivery {
                id: id("delivery:early"),
                transmission_id: id("tx:1"),
                delivered_tick: 129,
                receiver_id: None,
                source_event_id: id("event:early"),
            }),
            Err(CommunicationError::DeliveredTooEarly { .. })
        ));
    }

    #[test]
    fn lost_transmission_cannot_later_deliver() {
        let mut world = CommunicationWorld::default();
        world.register_plan(plan()).expect("plan");
        world.author_message(message()).expect("message");
        world
            .transmit(MessageTransmission {
                id: id("tx:lost"),
                message_id: id("message:order"),
                propagation_plan_id: id("signal-plan:aster-vesper"),
                sent_tick: 105,
                source_event_id: id("event:tx-lost"),
            })
            .expect("transmit");
        world
            .lose(MessageLoss {
                id: id("loss:1"),
                transmission_id: id("tx:lost"),
                lost_tick: 120,
                cause: "relay outage".into(),
                source_event_id: id("event:loss"),
            })
            .expect("loss");
        assert!(matches!(
            world.deliver(MessageDelivery {
                id: id("delivery:impossible"),
                transmission_id: id("tx:lost"),
                delivered_tick: 140,
                receiver_id: None,
                source_event_id: id("event:impossible"),
            }),
            Err(CommunicationError::TransmissionAlreadySettled(_))
        ));
    }

    #[test]
    fn signal_can_share_freight_geography_but_arrive_much_sooner() {
        let plan = plan();
        assert_eq!(plan.origin_location_id(), &id("loc:aster"));
        assert_eq!(plan.destination_location_id(), &id("loc:vesper"));
        assert_eq!(plan.earliest_delivery_tick, 130);
        // A transit provider may attest a much later freight arrival over these same path ids.
        assert_eq!(plan.topology_paths[0].path_id, id("path:a-hub"));
        assert_eq!(plan.topology_paths[1].path_id, id("path:hub-v"));
    }
}
