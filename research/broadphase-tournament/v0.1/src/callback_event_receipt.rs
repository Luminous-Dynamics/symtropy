//! Research reference for stable per-step callback event identities.
//!
//! Event receipts are distinct from contact geometry and warm-start freshness.
//! They prove that the supplied event stream exactly matches an independently
//! supplied set of expected events, and make duplicate delivery detectable.
//! The expected event set and payload digests still require an authoritative
//! solver/event observer; this module does not invoke user callbacks or claim
//! exactly-once delivery across a process boundary.

use incremental_aabb_tree_reference::{ProxyId, ProxyPair};
use std::collections::{BTreeMap, BTreeSet};

/// Semantic class of an externally visible physics callback event.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum CallbackEventKind {
    Collision,
    Trauma,
    Dissipation,
    Work,
}

/// Canonical subject for one callback event.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum CallbackEventSubject {
    /// Collision, trauma and contact dissipation are bound to a contact pair.
    Pair {
        pair: ProxyPair,
        generations: [u64; 2],
        manifold_revision: u64,
    },
    /// Mechanical work is attributed to one body/actuator rather than a pair.
    Body {
        proxy: ProxyId,
        generation: u64,
        actuator_revision: u64,
    },
}

/// Stable identity for one event emitted during one world step.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct CallbackEventId {
    pub world_revision: u64,
    pub subject: CallbackEventSubject,
    pub kind: CallbackEventKind,
    /// Deterministic ordinal for multiple same-kind events on one subject/tick.
    pub ordinal: u32,
}

/// Current endpoint state used to bind callback event identities.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CallbackEndpointState {
    pub generation: u64,
    pub response_state: u64,
    pub actuator_revision: u64,
}

/// Immutable event payload identity plus the response state under which it ran.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CallbackEventSnapshot {
    pub id: CallbackEventId,
    pub response_policy_sha256: [u8; 32],
    pub endpoint_response_state: [u64; 2],
    pub payload_sha256: [u8; 32],
}

/// Delivery is explicit. Failed and pending outcomes remain in the receipt.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CallbackDeliveryOutcome {
    Pending,
    Delivered,
    Failed,
}

/// Producer-supplied event and its observed delivery outcome.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CallbackEventReceipt {
    pub snapshot: CallbackEventSnapshot,
    pub outcome: CallbackDeliveryOutcome,
}

/// Counts replay-verified events by delivery state.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CallbackEventVerificationReport {
    pub event_count: usize,
    pub delivered_count: usize,
    pub pending_count: usize,
    pub failed_count: usize,
}

/// Failure from event identity, completeness or payload replay.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CallbackEventVerificationError {
    NonCanonicalPair(ProxyPair),
    SubjectKindMismatch(CallbackEventId),
    WrongWorldRevision(CallbackEventId),
    ResponsePolicyDigestMismatch(CallbackEventId),
    MissingEndpointState(ProxyId),
    EndpointGenerationMismatch(ProxyId),
    EndpointResponseStateMismatch(ProxyId),
    ActuatorRevisionMismatch(ProxyId),
    MissingResponsePolicyDigest(CallbackEventId),
    MissingPayloadDigest(CallbackEventId),
    DuplicateExpectedEvent(CallbackEventId),
    DuplicateSuppliedEvent(CallbackEventId),
    MissingEvent(CallbackEventId),
    UnexpectedEvent(CallbackEventId),
    EventPayloadMismatch(CallbackEventId),
}

/// Verify that a supplied delivery log is a complete, canonical replay of the
/// independently expected event stream for one world step.
///
/// A new world revision creates a distinct event identity for a continuing
/// contact. A replayed duplicate at the same revision and ordinal is rejected.
/// Pending and Failed are valid explicit outcomes in the report, but they are
/// not counted as delivered; callers must not treat them as success.
pub fn verify_callback_event_receipts(
    current_world_revision: u64,
    current_response_policy_sha256: [u8; 32],
    current_endpoints: &BTreeMap<ProxyId, CallbackEndpointState>,
    expected: &[CallbackEventSnapshot],
    supplied: &[CallbackEventReceipt],
) -> Result<CallbackEventVerificationReport, CallbackEventVerificationError> {
    let mut expected_ids = BTreeSet::new();
    let mut previous_expected = None;

    for snapshot in expected {
        validate_snapshot(
            snapshot,
            current_world_revision,
            current_response_policy_sha256,
            current_endpoints,
        )?;
        if previous_expected.is_some_and(|previous| previous >= snapshot.id) {
            if !expected_ids.insert(snapshot.id) {
                return Err(CallbackEventVerificationError::DuplicateExpectedEvent(snapshot.id));
            }
            return Err(CallbackEventVerificationError::UnexpectedEvent(snapshot.id));
        }
        if !expected_ids.insert(snapshot.id) {
            return Err(CallbackEventVerificationError::DuplicateExpectedEvent(snapshot.id));
        }
        previous_expected = Some(snapshot.id);
    }

    let mut previous_supplied = None;
    let mut supplied_ids = BTreeSet::new();
    let mut report = CallbackEventVerificationReport {
        event_count: expected.len(),
        ..CallbackEventVerificationReport::default()
    };

    for receipt in supplied {
        validate_snapshot(
            &receipt.snapshot,
            current_world_revision,
            current_response_policy_sha256,
            current_endpoints,
        )?;
        let id = receipt.snapshot.id;
        if previous_supplied.is_some_and(|previous| previous >= id) {
            if !supplied_ids.insert(id) {
                return Err(CallbackEventVerificationError::DuplicateSuppliedEvent(id));
            }
            return Err(CallbackEventVerificationError::UnexpectedEvent(id));
        }
        if !supplied_ids.insert(id) {
            return Err(CallbackEventVerificationError::DuplicateSuppliedEvent(id));
        }
        previous_supplied = Some(id);
    }

    for (expected_snapshot, supplied_receipt) in expected.iter().zip(supplied) {
        if expected_snapshot.id != supplied_receipt.snapshot.id {
            return Err(CallbackEventVerificationError::MissingEvent(
                expected_snapshot.id,
            ));
        }
        if expected_snapshot != &supplied_receipt.snapshot {
            return Err(CallbackEventVerificationError::EventPayloadMismatch(
                expected_snapshot.id,
            ));
        }
        match supplied_receipt.outcome {
            CallbackDeliveryOutcome::Pending => report.pending_count += 1,
            CallbackDeliveryOutcome::Delivered => report.delivered_count += 1,
            CallbackDeliveryOutcome::Failed => report.failed_count += 1,
        }
    }

    if expected.len() > supplied.len() {
        return Err(CallbackEventVerificationError::MissingEvent(
            expected[supplied.len()].id,
        ));
    }
    if supplied.len() > expected.len() {
        return Err(CallbackEventVerificationError::UnexpectedEvent(
            supplied[expected.len()].snapshot.id,
        ));
    }

    Ok(report)
}

fn validate_snapshot(
    snapshot: &CallbackEventSnapshot,
    current_world_revision: u64,
    current_response_policy_sha256: [u8; 32],
    current_endpoints: &BTreeMap<ProxyId, CallbackEndpointState>,
) -> Result<(), CallbackEventVerificationError> {
    let id = snapshot.id;
    match (id.subject, id.kind) {
        (
            CallbackEventSubject::Pair {
                pair,
                generations,
                ..
            },
            CallbackEventKind::Collision
            | CallbackEventKind::Trauma
            | CallbackEventKind::Dissipation,
        ) => {
            if pair.0 >= pair.1 {
                return Err(CallbackEventVerificationError::NonCanonicalPair(pair));
            }
            let left = current_endpoints
                .get(&pair.0)
                .ok_or(CallbackEventVerificationError::MissingEndpointState(pair.0))?;
            let right = current_endpoints
                .get(&pair.1)
                .ok_or(CallbackEventVerificationError::MissingEndpointState(pair.1))?;
            if generations != [left.generation, right.generation] {
                let proxy = if generations[0] != left.generation {
                    pair.0
                } else {
                    pair.1
                };
                return Err(CallbackEventVerificationError::EndpointGenerationMismatch(
                    proxy,
                ));
            }
            if snapshot.endpoint_response_state != [left.response_state, right.response_state] {
                let proxy = if snapshot.endpoint_response_state[0] != left.response_state {
                    pair.0
                } else {
                    pair.1
                };
                return Err(CallbackEventVerificationError::EndpointResponseStateMismatch(
                    proxy,
                ));
            }
        }
        (
            CallbackEventSubject::Body {
                proxy,
                generation,
                actuator_revision,
            },
            CallbackEventKind::Work,
        ) => {
            let endpoint = current_endpoints
                .get(&proxy)
                .ok_or(CallbackEventVerificationError::MissingEndpointState(proxy))?;
            if generation != endpoint.generation {
                return Err(CallbackEventVerificationError::EndpointGenerationMismatch(
                    proxy,
                ));
            }
            if actuator_revision != endpoint.actuator_revision {
                return Err(CallbackEventVerificationError::ActuatorRevisionMismatch(
                    proxy,
                ));
            }
            if snapshot.endpoint_response_state != [endpoint.response_state, 0] {
                return Err(CallbackEventVerificationError::EndpointResponseStateMismatch(
                    proxy,
                ));
            }
        }
        _ => return Err(CallbackEventVerificationError::SubjectKindMismatch(id)),
    }
    if id.world_revision != current_world_revision {
        return Err(CallbackEventVerificationError::WrongWorldRevision(id));
    }
    if snapshot.response_policy_sha256.iter().all(|byte| *byte == 0) {
        return Err(CallbackEventVerificationError::MissingResponsePolicyDigest(id));
    }
    if snapshot.response_policy_sha256 != current_response_policy_sha256 {
        return Err(CallbackEventVerificationError::ResponsePolicyDigestMismatch(
            id,
        ));
    }
    if snapshot.payload_sha256.iter().all(|byte| *byte == 0) {
        return Err(CallbackEventVerificationError::MissingPayloadDigest(id));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot(world_revision: u64, ordinal: u32) -> CallbackEventSnapshot {
        CallbackEventSnapshot {
            id: CallbackEventId {
                world_revision,
                subject: CallbackEventSubject::Pair {
                    pair: ProxyPair::new(ProxyId(1), ProxyId(2)).expect("pair"),
                    generations: [3, 4],
                    manifold_revision: 8,
                },
                kind: CallbackEventKind::Collision,
                ordinal,
            },
            response_policy_sha256: [11; 32],
            endpoint_response_state: [6, 7],
            payload_sha256: [12; 32],
        }
    }

    fn receipt(snapshot: CallbackEventSnapshot) -> CallbackEventReceipt {
        CallbackEventReceipt {
            snapshot,
            outcome: CallbackDeliveryOutcome::Delivered,
        }
    }

    fn endpoint_states() -> BTreeMap<ProxyId, CallbackEndpointState> {
        BTreeMap::from([
            (
                ProxyId(1),
                CallbackEndpointState {
                    generation: 3,
                    response_state: 6,
                    actuator_revision: 0,
                },
            ),
            (
                ProxyId(2),
                CallbackEndpointState {
                    generation: 4,
                    response_state: 7,
                    actuator_revision: 0,
                },
            ),
            (
                ProxyId(9),
                CallbackEndpointState {
                    generation: 2,
                    response_state: 8,
                    actuator_revision: 5,
                },
            ),
        ])
    }

    fn verify(
        world_revision: u64,
        expected: &[CallbackEventSnapshot],
        supplied: &[CallbackEventReceipt],
    ) -> Result<CallbackEventVerificationReport, CallbackEventVerificationError> {
        super::verify_callback_event_receipts(
            world_revision,
            [11; 32],
            &endpoint_states(),
            expected,
            supplied,
        )
    }

    #[test]
    fn one_event_replays_as_delivered() {
        let expected = [snapshot(10, 0)];
        let supplied = [receipt(expected[0])];
        let report = verify(10, &expected, &supplied)
            .expect("complete event receipt");
        assert_eq!(report.event_count, 1);
        assert_eq!(report.delivered_count, 1);
    }

    #[test]
    fn same_contact_on_next_world_step_has_distinct_event_identity() {
        let previous = snapshot(10, 0);
        let current = snapshot(11, 0);
        assert_ne!(previous.id, current.id);
        assert_eq!(
            verify(11, &[current], &[receipt(current)])
                .expect("next-step event"),
            CallbackEventVerificationReport {
                event_count: 1,
                delivered_count: 1,
                pending_count: 0,
                failed_count: 0,
            }
        );
    }

    #[test]
    fn duplicate_expected_event_is_rejected() {
        let event = snapshot(10, 0);
        assert_eq!(
            verify(10, &[event, event], &[receipt(event), receipt(event)]),
            Err(CallbackEventVerificationError::DuplicateExpectedEvent(event.id))
        );
    }

    #[test]
    fn duplicate_supplied_delivery_is_rejected() {
        let event = snapshot(10, 0);
        let expected = [event];
        assert_eq!(
            verify(10, &expected, &[receipt(event), receipt(event)]),
            Err(CallbackEventVerificationError::DuplicateSuppliedEvent(event.id))
        );
    }

    #[test]
    fn omitted_event_is_rejected() {
        let event = snapshot(10, 0);
        assert_eq!(
            verify(10, &[event], &[]),
            Err(CallbackEventVerificationError::MissingEvent(event.id))
        );
    }

    #[test]
    fn unexpected_event_is_rejected() {
        let expected = snapshot(10, 0);
        let unexpected = snapshot(10, 1);
        assert_eq!(
            verify(10, &[expected], &[receipt(unexpected)]),
            Err(CallbackEventVerificationError::MissingEvent(expected.id))
        );
    }

    #[test]
    fn payload_mutation_is_rejected() {
        let expected = snapshot(10, 0);
        let mut tampered = expected;
        tampered.payload_sha256 = [13; 32];
        assert_eq!(
            verify(10, &[expected], &[receipt(tampered)]),
            Err(CallbackEventVerificationError::EventPayloadMismatch(expected.id))
        );
    }

    #[test]
    fn response_state_binding_is_part_of_event_identity_evidence() {
        let expected = snapshot(10, 0);
        let mut changed = expected;
        changed.endpoint_response_state[0] += 1;
        assert_eq!(
            verify(10, &[expected], &[receipt(changed)]),
            Err(CallbackEventVerificationError::EventPayloadMismatch(expected.id))
        );
    }

    #[test]
    fn pending_and_failed_delivery_remain_visible_not_successful() {
        let delivered = snapshot(10, 0);
        let pending = snapshot(10, 1);
        let failed = snapshot(10, 2);
        let supplied = [
            receipt(delivered),
            CallbackEventReceipt {
                snapshot: pending,
                outcome: CallbackDeliveryOutcome::Pending,
            },
            CallbackEventReceipt {
                snapshot: failed,
                outcome: CallbackDeliveryOutcome::Failed,
            },
        ];
        let report = verify(10, &[delivered, pending, failed], &supplied)
            .expect("all outcomes are explicit");
        assert_eq!(report.delivered_count, 1);
        assert_eq!(report.pending_count, 1);
        assert_eq!(report.failed_count, 1);
    }

    #[test]
    fn stale_world_revision_is_rejected() {
        let event = snapshot(9, 0);
        assert_eq!(
            verify(10, &[event], &[receipt(event)]),
            Err(CallbackEventVerificationError::WrongWorldRevision(event.id))
        );
    }

    #[test]
    fn body_scoped_work_event_has_a_valid_body_subject() {
        let event = CallbackEventSnapshot {
            id: CallbackEventId {
                world_revision: 10,
                subject: CallbackEventSubject::Body {
                    proxy: ProxyId(9),
                    generation: 2,
                    actuator_revision: 5,
                },
                kind: CallbackEventKind::Work,
                ordinal: 0,
            },
            response_policy_sha256: [11; 32],
            endpoint_response_state: [8, 0],
            payload_sha256: [22; 32],
        };
        let report = verify(10, &[event], &[receipt(event)])
            .expect("body-scoped work receipt");
        assert_eq!(report.delivered_count, 1);
    }

    #[test]
    fn pair_scoped_collision_cannot_use_body_subject() {
        let event = CallbackEventSnapshot {
            id: CallbackEventId {
                world_revision: 10,
                subject: CallbackEventSubject::Body {
                    proxy: ProxyId(9),
                    generation: 2,
                    actuator_revision: 5,
                },
                kind: CallbackEventKind::Collision,
                ordinal: 0,
            },
            response_policy_sha256: [21; 32],
            endpoint_response_state: [8, 0],
            payload_sha256: [22; 32],
        };
        assert_eq!(
            verify(10, &[event], &[receipt(event)]),
            Err(CallbackEventVerificationError::SubjectKindMismatch(event.id))
        );
    }

    #[test]
    fn stale_proxy_generation_is_rejected_against_current_endpoint_snapshot() {
        let mut event = snapshot(10, 0);
        let CallbackEventSubject::Pair {
            pair,
            mut generations,
            manifold_revision,
        } = event.id.subject else {
            panic!("pair event expected");
        };
        generations[0] += 1;
        event.id.subject = CallbackEventSubject::Pair {
            pair,
            generations,
            manifold_revision,
        };
        assert_eq!(
            verify(10, &[event], &[receipt(event)]),
            Err(CallbackEventVerificationError::EndpointGenerationMismatch(
                ProxyId(1)
            ))
        );
    }

    #[test]
    fn stale_response_state_is_rejected_against_current_endpoint_snapshot() {
        let mut event = snapshot(10, 0);
        event.endpoint_response_state[0] += 1;
        assert_eq!(
            verify(10, &[event], &[receipt(event)]),
            Err(CallbackEventVerificationError::EndpointResponseStateMismatch(
                ProxyId(1)
            ))
        );
    }

    #[test]
    fn response_policy_digest_must_match_current_global_policy() {
        let mut event = snapshot(10, 0);
        event.response_policy_sha256 = [99; 32];
        assert_eq!(
            verify(10, &[event], &[receipt(event)]),
            Err(CallbackEventVerificationError::ResponsePolicyDigestMismatch(event.id))
        );
    }

    #[test]
    fn body_work_actuator_revision_must_match_current_snapshot() {
        let event = CallbackEventSnapshot {
            id: CallbackEventId {
                world_revision: 10,
                subject: CallbackEventSubject::Body {
                    proxy: ProxyId(9),
                    generation: 2,
                    actuator_revision: 4,
                },
                kind: CallbackEventKind::Work,
                ordinal: 0,
            },
            response_policy_sha256: [11; 32],
            endpoint_response_state: [8, 0],
            payload_sha256: [22; 32],
        };
        assert_eq!(
            verify(10, &[event], &[receipt(event)]),
            Err(CallbackEventVerificationError::ActuatorRevisionMismatch(
                ProxyId(9)
            ))
        );
    }

}
