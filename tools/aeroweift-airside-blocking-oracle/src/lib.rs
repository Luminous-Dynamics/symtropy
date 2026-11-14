//! Tiny finite-resource oracle for AeroWeft's synthetic blocking cascade.
//!
//! The crate checks a frozen aircraft/resource event trace for finite capacity,
//! acquisition/release correspondence, causal predecessors, and the expected
//! blocking/holding intervals. It does not implement a flight scheduler,
//! aircraft model, general discrete-event simulator, queueing product, or
//! operational vertiport system.
//!
//! All aircraft, resource identifiers, ticks, and energy quantities are
//! fictional fixture values.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::collections::{BTreeMap, BTreeSet};

/// A finite, half-open resource occupancy claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reservation {
    /// Synthetic aircraft identifier.
    pub aircraft_id: &'static str,
    /// Synthetic finite resource identifier.
    pub resource_id: &'static str,
    /// Inclusive acquisition tick.
    pub start_tick: u16,
    /// Exclusive release tick.
    pub end_tick: u16,
}

/// Capacity of one resource in the frozen fixture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceCapacity {
    /// Synthetic resource identifier.
    pub resource_id: &'static str,
    /// Maximum simultaneous aircraft/resource claims.
    pub capacity: u8,
}

/// Resource event kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventKind {
    /// Aircraft acquired the named resource.
    Acquire,
    /// Aircraft released the named resource.
    Release,
    /// Declared service completed while the aircraft still occupies a resource.
    ServiceComplete,
    /// Downstream resource request was blocked by another aircraft.
    DownstreamAcquireBlocked,
}

/// One immutable event in the synthetic causal trace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Event {
    /// Unique synthetic event identifier.
    pub event_id: &'static str,
    /// Aircraft associated with the event.
    pub aircraft_id: &'static str,
    /// Resource associated with the event.
    pub resource_id: &'static str,
    /// Event kind.
    pub kind: EventKind,
    /// Integer event tick.
    pub tick: u16,
    /// Optional causal predecessor event.
    pub predecessor: Option<&'static str>,
    /// Aircraft occupying the downstream resource when blocked, if applicable.
    pub blocker_aircraft_id: Option<&'static str>,
}

/// Error from a finite-resource event trace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditError {
    /// A reservation has an empty or reversed interval.
    InvalidInterval,
    /// A reservation references no declared resource.
    UnknownResource,
    /// A resource capacity is declared more than once.
    DuplicateResourceCapacity,
    /// A capacity limit is exceeded in an elementary time interval.
    CapacityExceeded,
    /// Event identifiers are not unique.
    DuplicateEventId,
    /// A predecessor event is absent from the manifest.
    MissingPredecessor,
    /// A causal predecessor occurs after its child event.
    CausalityViolation,
    /// An acquisition or release has no matching reservation interval.
    ReservationEventMismatch,
    /// A blocking event does not identify its blocker.
    MissingBlockingWitness,
    /// The same aircraft/resource occupancy interval is declared more than once.
    DuplicateReservation,
}

/// Peak simultaneous occupancy for one declared resource.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourcePeak {
    /// Synthetic resource identifier.
    pub resource_id: &'static str,
    /// Maximum simultaneous occupancy observed.
    pub maximum_occupancy: usize,
}

/// Validate every reservation against its declared finite resource capacity.
///
/// Half-open intervals are used: a claim ending at tick t does not overlap a
/// different claim beginning at tick t.
pub fn audit_capacities(
    reservations: &[Reservation],
    capacities: &[ResourceCapacity],
) -> Result<Vec<ResourcePeak>, AuditError> {
    let mut capacity_by_resource = BTreeMap::new();
    for capacity in capacities {
        if capacity_by_resource
            .insert(capacity.resource_id, capacity.capacity)
            .is_some()
        {
            return Err(AuditError::DuplicateResourceCapacity);
        }
    }

    for reservation in reservations {
        if reservation.start_tick >= reservation.end_tick {
            return Err(AuditError::InvalidInterval);
        }
        if !capacity_by_resource.contains_key(reservation.resource_id) {
            return Err(AuditError::UnknownResource);
        }
    }

    let mut peaks = Vec::new();
    for capacity in capacities {
        let claims: Vec<_> = reservations
            .iter()
            .filter(|claim| claim.resource_id == capacity.resource_id)
            .collect();

        let mut boundaries = Vec::with_capacity(claims.len() * 2);
        for claim in &claims {
            boundaries.push(claim.start_tick);
            boundaries.push(claim.end_tick);
        }
        boundaries.sort_unstable();
        boundaries.dedup();

        let mut maximum_occupancy = 0usize;
        for interval in boundaries.windows(2) {
            let start = interval[0];
            let end = interval[1];
            if start >= end {
                continue;
            }

            let active = claims
                .iter()
                .filter(|claim| {
                    claim.start_tick <= start && claim.end_tick >= end
                })
                .count();

            maximum_occupancy = maximum_occupancy.max(active);
            if active > usize::from(capacity.capacity) {
                return Err(AuditError::CapacityExceeded);
            }
        }

        peaks.push(ResourcePeak {
            resource_id: capacity.resource_id,
            maximum_occupancy,
        });
    }

    peaks.sort_by_key(|peak| peak.resource_id);
    Ok(peaks)
}

/// Check that each reservation has exactly one matching acquire and release.
pub fn audit_reservation_events(
    reservations: &[Reservation],
    events: &[Event],
) -> Result<(), AuditError> {
    let mut reservation_ids = BTreeSet::new();
    for reservation in reservations {
        let key = (
            reservation.aircraft_id,
            reservation.resource_id,
            reservation.start_tick,
            reservation.end_tick,
        );
        if !reservation_ids.insert(key) {
            return Err(AuditError::DuplicateReservation);
        }
    }

    let mut event_ids = BTreeSet::new();
    for event in events {
        if !event_ids.insert(event.event_id) {
            return Err(AuditError::DuplicateEventId);
        }
    }

    for reservation in reservations {
        let acquire_count = events
            .iter()
            .filter(|event| {
                event.kind == EventKind::Acquire
                    && event.aircraft_id == reservation.aircraft_id
                    && event.resource_id == reservation.resource_id
                    && event.tick == reservation.start_tick
            })
            .count();
        let release_count = events
            .iter()
            .filter(|event| {
                event.kind == EventKind::Release
                    && event.aircraft_id == reservation.aircraft_id
                    && event.resource_id == reservation.resource_id
                    && event.tick == reservation.end_tick
            })
            .count();

        if acquire_count != 1 || release_count != 1 {
            return Err(AuditError::ReservationEventMismatch);
        }
    }

    for event in events {
        if matches!(event.kind, EventKind::Acquire | EventKind::Release) {
            let matching_claim = reservations.iter().any(|reservation| {
                reservation.aircraft_id == event.aircraft_id
                    && reservation.resource_id == event.resource_id
                    && match event.kind {
                        EventKind::Acquire => reservation.start_tick == event.tick,
                        EventKind::Release => reservation.end_tick == event.tick,
                        _ => false,
                    }
            });
            if !matching_claim {
                return Err(AuditError::ReservationEventMismatch);
            }
        }

        if event.kind == EventKind::DownstreamAcquireBlocked
            && event.blocker_aircraft_id.is_none()
        {
            return Err(AuditError::MissingBlockingWitness);
        }
    }

    Ok(())
}

/// Validate event-ID uniqueness and all declared causal predecessor edges.
pub fn audit_causality(events: &[Event]) -> Result<(), AuditError> {
    let mut by_id = BTreeMap::new();
    for event in events {
        if by_id.insert(event.event_id, event).is_some() {
            return Err(AuditError::DuplicateEventId);
        }
    }

    for event in events {
        if let Some(predecessor_id) = event.predecessor {
            let Some(predecessor) = by_id.get(predecessor_id) else {
                return Err(AuditError::MissingPredecessor);
            };
            if predecessor.tick > event.tick {
                return Err(AuditError::CausalityViolation);
            }
        }
    }

    // A predecessor chain is single-parent, so an iterative walk detects
    // same-tick cycles without depending on input ordering.
    let mut fully_checked = BTreeSet::new();
    for event in events {
        let mut path = BTreeSet::new();
        let mut cursor: Option<&Event> = Some(event);

        while let Some(current) = cursor {
            if fully_checked.contains(current.event_id) {
                break;
            }
            if !path.insert(current.event_id) {
                return Err(AuditError::CausalityViolation);
            }
            cursor = match current.predecessor {
                Some(predecessor_id) => by_id.get(predecessor_id).copied(),
                None => None,
            };
        }

        fully_checked.extend(path);
    }

    Ok(())
}

/// Return an event tick by identifier.
pub fn event_tick(events: &[Event], event_id: &str) -> Option<u16> {
    events
        .iter()
        .find(|event| event.event_id == event_id)
        .map(|event| event.tick)
}

/// Derive a blocking-after-service duration from service completion and resource release.
pub fn blocking_ticks(
    events: &[Event],
    service_complete_event: &str,
    release_event: &str,
) -> Option<u16> {
    let service_complete = event_tick(events, service_complete_event)?;
    let release = event_tick(events, release_event)?;
    release.checked_sub(service_complete)
}

/// Derive occupancy duration for one aircraft/resource pair from reservations.
pub fn occupancy_ticks(
    reservations: &[Reservation],
    aircraft_id: &str,
    resource_id: &str,
) -> Option<u16> {
    let reservation = reservations.iter().find(|reservation| {
        reservation.aircraft_id == aircraft_id
            && reservation.resource_id == resource_id
    })?;
    reservation.end_tick.checked_sub(reservation.start_tick)
}

/// Derive the synthetic energy delta across the declared hold interval.
pub fn holding_energy_delta(
    holding_ticks: u16,
    energy_units_per_tick: i32,
) -> Option<i32> {
    i32::from(holding_ticks).checked_mul(energy_units_per_tick).map(|cost| -cost)
}
