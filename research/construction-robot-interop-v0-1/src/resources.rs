//! Shared-resource reservation reference evaluator.
//!
//! This is scheduling semantics only. A granted reservation is not a safety permit,
//! clearance, motion authorization, or proof that a physical work zone is safe.

use std::collections::BTreeMap;
use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReservationRequest {
    pub reservation_id: String,
    pub resource_id: String,
    pub task_id: String,
    pub owner_id: String,
    pub site_epoch: String,
    pub clock_domain_id: String,
    pub interval_start_ms: u64,
    pub interval_end_ms: u64,
    pub expires_at_ms: u64,
    pub units: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReservationState {
    Active,
    Released,
    Expired,
    Invalidated,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReservationOutcome {
    Granted,
    Duplicate,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReleaseOutcome {
    Released,
    AlreadyReleased,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReservationError {
    EmptyField(&'static str),
    ZeroCapacity,
    ZeroUnits,
    InvalidInterval,
    ExpiryBeforeEnd,
    RequestAlreadyExpired,
    UnknownResource,
    ResourceAlreadyDefined,
    ReservationIdConflict,
    ReservationIdConsumed,
    UnknownReservation,
    WrongOwner,
    StaleSiteEpoch,
    ClockDomainMismatch,
    CapacityExceeded { capacity: u32, requested_total: u32 },
    CapacityOverflow,
    ReservationNotActive,
}

impl fmt::Display for ReservationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ReservationError {}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ReservationRecord {
    request: ReservationRequest,
    state: ReservationState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReservationBook {
    current_site_epoch: String,
    clock_domain_id: String,
    resources: BTreeMap<String, u32>,
    reservations: BTreeMap<String, ReservationRecord>,
}

impl ReservationBook {
    pub fn new(
        current_site_epoch: impl Into<String>,
        clock_domain_id: impl Into<String>,
    ) -> Result<Self, ReservationError> {
        let current_site_epoch = current_site_epoch.into();
        let clock_domain_id = clock_domain_id.into();
        require_non_empty("current_site_epoch", &current_site_epoch)?;
        require_non_empty("clock_domain_id", &clock_domain_id)?;

        Ok(Self {
            current_site_epoch,
            clock_domain_id,
            resources: BTreeMap::new(),
            reservations: BTreeMap::new(),
        })
    }

    pub fn register_resource(
        &mut self,
        resource_id: impl Into<String>,
        capacity: u32,
    ) -> Result<(), ReservationError> {
        let resource_id = resource_id.into();
        require_non_empty("resource_id", &resource_id)?;
        if capacity == 0 {
            return Err(ReservationError::ZeroCapacity);
        }

        if let Some(existing_capacity) = self.resources.get(&resource_id) {
            return if *existing_capacity == capacity {
                Ok(())
            } else {
                Err(ReservationError::ResourceAlreadyDefined)
            };
        }

        self.resources.insert(resource_id, capacity);
        Ok(())
    }

    /// Reserve capacity in a declared clock domain and current site epoch.
    /// Overlap is evaluated on half-open intervals [start, end).
    pub fn reserve(
        &mut self,
        request: ReservationRequest,
        now_ms: u64,
    ) -> Result<ReservationOutcome, ReservationError> {
        validate_request(&request)?;
        self.expire(now_ms);

        if request.site_epoch != self.current_site_epoch {
            return Err(ReservationError::StaleSiteEpoch);
        }
        if request.clock_domain_id != self.clock_domain_id {
            return Err(ReservationError::ClockDomainMismatch);
        }

        let capacity = self
            .resources
            .get(&request.resource_id)
            .copied()
            .ok_or(ReservationError::UnknownResource)?;

        if let Some(existing) = self.reservations.get(&request.reservation_id) {
            if existing.state != ReservationState::Active {
                return Err(ReservationError::ReservationIdConsumed);
            }
            return if existing.request == request {
                Ok(ReservationOutcome::Duplicate)
            } else {
                Err(ReservationError::ReservationIdConflict)
            };
        }

        if request.interval_start_ms < now_ms {
            return Err(ReservationError::InvalidInterval);
        }
        if request.expires_at_ms <= now_ms {
            return Err(ReservationError::RequestAlreadyExpired);
        }

        let mut used_capacity = 0_u32;
        for existing in self.reservations.values() {
            if existing.state == ReservationState::Active
                && existing.request.resource_id == request.resource_id
                && overlaps(
                    request.interval_start_ms,
                    request.interval_end_ms,
                    existing.request.interval_start_ms,
                    existing.request.interval_end_ms,
                )
            {
                used_capacity = used_capacity
                    .checked_add(existing.request.units)
                    .ok_or(ReservationError::CapacityOverflow)?;
            }
        }

        let requested_total = used_capacity
            .checked_add(request.units)
            .ok_or(ReservationError::CapacityOverflow)?;
        if requested_total > capacity {
            return Err(ReservationError::CapacityExceeded {
                capacity,
                requested_total,
            });
        }

        self.reservations.insert(
            request.reservation_id.clone(),
            ReservationRecord {
                request,
                state: ReservationState::Active,
            },
        );
        Ok(ReservationOutcome::Granted)
    }

    pub fn release(
        &mut self,
        reservation_id: &str,
        owner_id: &str,
    ) -> Result<ReleaseOutcome, ReservationError> {
        require_non_empty("reservation_id", reservation_id)?;
        require_non_empty("owner_id", owner_id)?;

        let record = self
            .reservations
            .get_mut(reservation_id)
            .ok_or(ReservationError::UnknownReservation)?;

        if record.request.owner_id != owner_id {
            return Err(ReservationError::WrongOwner);
        }

        match record.state {
            ReservationState::Active => {
                record.state = ReservationState::Released;
                Ok(ReleaseOutcome::Released)
            }
            ReservationState::Released => Ok(ReleaseOutcome::AlreadyReleased),
            ReservationState::Expired | ReservationState::Invalidated => {
                Err(ReservationError::ReservationNotActive)
            }
        }
    }

    /// Invalidate every active reservation when the authoritative site epoch changes.
    /// The method does not decide whether a robot may move or whether a zone is safe.
    pub fn advance_site_epoch(
        &mut self,
        new_site_epoch: impl Into<String>,
    ) -> Result<(), ReservationError> {
        let new_site_epoch = new_site_epoch.into();
        require_non_empty("new_site_epoch", &new_site_epoch)?;
        if new_site_epoch == self.current_site_epoch {
            return Ok(());
        }

        self.current_site_epoch = new_site_epoch;
        for record in self.reservations.values_mut() {
            if record.state == ReservationState::Active {
                record.state = ReservationState::Invalidated;
            }
        }
        Ok(())
    }

    /// Mark expired reservations as historical records. Their IDs remain consumed.
    pub fn expire(&mut self, now_ms: u64) -> usize {
        let mut expired = 0;
        for record in self.reservations.values_mut() {
            if record.state == ReservationState::Active
                && record.request.expires_at_ms <= now_ms
            {
                record.state = ReservationState::Expired;
                expired += 1;
            }
        }
        expired
    }

    pub fn state(&self, reservation_id: &str) -> Option<ReservationState> {
        self.reservations
            .get(reservation_id)
            .map(|record| record.state)
    }

    pub fn current_site_epoch(&self) -> &str {
        &self.current_site_epoch
    }
}

fn validate_request(request: &ReservationRequest) -> Result<(), ReservationError> {
    require_non_empty("reservation_id", &request.reservation_id)?;
    require_non_empty("resource_id", &request.resource_id)?;
    require_non_empty("task_id", &request.task_id)?;
    require_non_empty("owner_id", &request.owner_id)?;
    require_non_empty("site_epoch", &request.site_epoch)?;
    require_non_empty("clock_domain_id", &request.clock_domain_id)?;

    if request.units == 0 {
        return Err(ReservationError::ZeroUnits);
    }
    if request.interval_start_ms >= request.interval_end_ms {
        return Err(ReservationError::InvalidInterval);
    }
    if request.expires_at_ms < request.interval_end_ms {
        return Err(ReservationError::ExpiryBeforeEnd);
    }
    Ok(())
}

fn overlaps(first_start: u64, first_end: u64, second_start: u64, second_end: u64) -> bool {
    first_start < second_end && second_start < first_end
}

fn require_non_empty(name: &'static str, value: &str) -> Result<(), ReservationError> {
    if value.trim().is_empty() {
        return Err(ReservationError::EmptyField(name));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn book(capacity: u32) -> ReservationBook {
        let mut book = ReservationBook::new("site-epoch-1", "site-monotonic-ms")
            .expect("valid reservation book");
        book.register_resource("work-zone-A", capacity)
            .expect("valid resource");
        book
    }

    fn request(
        id: &str,
        owner: &str,
        start: u64,
        end: u64,
        expiry: u64,
        units: u32,
    ) -> ReservationRequest {
        ReservationRequest {
            reservation_id: id.to_owned(),
            resource_id: "work-zone-A".to_owned(),
            task_id: format!("task-{id}"),
            owner_id: owner.to_owned(),
            site_epoch: "site-epoch-1".to_owned(),
            clock_domain_id: "site-monotonic-ms".to_owned(),
            interval_start_ms: start,
            interval_end_ms: end,
            expires_at_ms: expiry,
            units,
        }
    }

    #[test]
    fn exclusive_resource_rejects_overlapping_reservation() {
        let mut book = book(1);
        assert_eq!(
            book.reserve(request("r1", "robot-1", 100, 200, 250, 1), 50),
            Ok(ReservationOutcome::Granted)
        );
        assert_eq!(
            book.reserve(request("r2", "robot-2", 150, 250, 300, 1), 50),
            Err(ReservationError::CapacityExceeded {
                capacity: 1,
                requested_total: 2
            })
        );
    }

    #[test]
    fn half_open_non_overlapping_intervals_can_share_exclusive_resource() {
        let mut book = book(1);
        book.reserve(request("r1", "robot-1", 100, 200, 250, 1), 50)
            .expect("first reservation");
        assert_eq!(
            book.reserve(request("r2", "robot-2", 200, 300, 350, 1), 50),
            Ok(ReservationOutcome::Granted)
        );
    }

    #[test]
    fn capacity_limited_resource_enforces_aggregate_units() {
        let mut book = book(3);
        book.reserve(request("r1", "robot-1", 100, 200, 250, 2), 50)
            .expect("two units");
        book.reserve(request("r2", "robot-2", 150, 250, 300, 1), 50)
            .expect("one remaining unit");
        assert_eq!(
            book.reserve(request("r3", "robot-3", 160, 240, 300, 1), 50),
            Err(ReservationError::CapacityExceeded {
                capacity: 3,
                requested_total: 4
            })
        );
    }

    #[test]
    fn identical_reservation_is_idempotent_but_id_reuse_is_not() {
        let mut book = book(1);
        let first = request("r1", "robot-1", 100, 200, 250, 1);
        assert_eq!(
            book.reserve(first.clone(), 50),
            Ok(ReservationOutcome::Granted)
        );
        assert_eq!(
            book.reserve(first, 50),
            Ok(ReservationOutcome::Duplicate)
        );
        assert_eq!(
            book.reserve(request("r1", "robot-2", 100, 200, 250, 1), 50),
            Err(ReservationError::ReservationIdConflict)
        );
    }

    #[test]
    fn expired_reservation_no_longer_consumes_capacity_but_id_stays_consumed() {
        let mut book = book(1);
        book.reserve(request("r1", "robot-1", 100, 200, 200, 1), 50)
            .expect("first reservation");
        assert_eq!(book.expire(200), 1);
        assert_eq!(book.state("r1"), Some(ReservationState::Expired));
        assert_eq!(
            book.reserve(request("r2", "robot-2", 200, 300, 350, 1), 200),
            Ok(ReservationOutcome::Granted)
        );
        assert_eq!(
            book.reserve(request("r1", "robot-1", 200, 300, 350, 1), 200),
            Err(ReservationError::ReservationIdConsumed)
        );
    }

    #[test]
    fn site_epoch_change_invalidates_every_active_reservation() {
        let mut book = book(2);
        book.reserve(request("r1", "robot-1", 100, 200, 250, 1), 50)
            .expect("reservation");
        book.advance_site_epoch("site-epoch-2")
            .expect("new epoch");
        assert_eq!(book.state("r1"), Some(ReservationState::Invalidated));
        assert_eq!(book.current_site_epoch(), "site-epoch-2");
        assert_eq!(
            book.reserve(request("r2", "robot-2", 200, 300, 350, 1), 100),
            Err(ReservationError::StaleSiteEpoch)
        );
    }

    #[test]
    fn reservations_from_another_clock_domain_are_rejected() {
        let mut book = book(1);
        let mut wrong_clock = request("r1", "robot-1", 100, 200, 250, 1);
        wrong_clock.clock_domain_id = "robot-local-clock".to_owned();
        assert_eq!(
            book.reserve(wrong_clock, 50),
            Err(ReservationError::ClockDomainMismatch)
        );
    }

    #[test]
    fn only_owner_can_release_and_released_capacity_is_available() {
        let mut book = book(1);
        book.reserve(request("r1", "robot-1", 100, 200, 250, 1), 50)
            .expect("reservation");
        assert_eq!(
            book.release("r1", "robot-2"),
            Err(ReservationError::WrongOwner)
        );
        assert_eq!(
            book.release("r1", "robot-1"),
            Ok(ReleaseOutcome::Released)
        );
        assert_eq!(
            book.release("r1", "robot-1"),
            Ok(ReleaseOutcome::AlreadyReleased)
        );
        assert_eq!(
            book.reserve(request("r2", "robot-2", 100, 200, 250, 1), 50),
            Ok(ReservationOutcome::Granted)
        );
    }

    #[test]
    fn zero_units_and_expired_or_retroactive_requests_are_rejected() {
        let mut book = book(2);
        assert_eq!(
            book.reserve(request("r1", "robot-1", 100, 200, 250, 0), 50),
            Err(ReservationError::ZeroUnits)
        );
        assert_eq!(
            book.reserve(request("r2", "robot-1", 100, 200, 200, 1), 200),
            Err(ReservationError::RequestAlreadyExpired)
        );
        assert_eq!(
            book.reserve(request("r3", "robot-1", 40, 60, 80, 1), 50),
            Err(ReservationError::InvalidInterval)
        );
    }

    #[test]
    fn resource_capacity_must_be_positive() {
        let mut book = ReservationBook::new("site-epoch-1", "site-monotonic-ms")
            .expect("valid book");
        assert_eq!(
            book.register_resource("work-zone-A", 0),
            Err(ReservationError::ZeroCapacity)
        );
    }
}
