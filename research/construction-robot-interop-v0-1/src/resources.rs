//! Shared-resource reservation reference evaluator.
//!
//! This is scheduling semantics only. A granted reservation is not a safety permit,
//! clearance, motion authorization, or proof that a physical work zone is safe.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

/// Multi-resolution temporal broad-phase cells. Each active interval is indexed at
/// the finest level that can represent it within the bounded bucket budget.
const RESERVATION_TIME_BUCKET_WIDTHS_MS: [u64; 4] = [1_000, 10_000, 60_000, 600_000];
/// Very long reservations use a per-resource fallback rather than allocating thousands of buckets.
const MAX_RESERVATION_TIME_BUCKETS: u64 = 4_096;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReservationRequest {
    pub reservation_id: String,
    pub resource_id: String,
    pub task_id: String,
    pub owner_id: String,
    pub site_epoch: String,
    pub clock_domain_id: String,
    /// Maximum absolute timestamp error for the request, in the declared clock domain.
    pub clock_uncertainty_ms: u64,
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
    ClockUncertaintyExceeded { allowed_ms: u64, actual_ms: u64 },
    CapacityExceeded { capacity: u32, requested_total: u32 },
    CapacityOverflow,
    ReservationNotActive,
    IndexInconsistent(String),
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
    max_clock_uncertainty_ms: u64,
    resources: BTreeMap<String, u32>,
    reservations: BTreeMap<String, ReservationRecord>,
    active_by_resource: BTreeMap<String, BTreeSet<String>>,
    time_buckets: BTreeMap<(String, usize, u64), BTreeSet<String>>,
    global_by_resource: BTreeMap<String, BTreeSet<String>>,
    expiry_index: BTreeMap<u64, BTreeSet<String>>,
}

impl ReservationBook {
    pub fn new(
        current_site_epoch: impl Into<String>,
        clock_domain_id: impl Into<String>,
        max_clock_uncertainty_ms: u64,
    ) -> Result<Self, ReservationError> {
        let current_site_epoch = current_site_epoch.into();
        let clock_domain_id = clock_domain_id.into();
        require_non_empty("current_site_epoch", &current_site_epoch)?;
        require_non_empty("clock_domain_id", &clock_domain_id)?;

        Ok(Self {
            current_site_epoch,
            clock_domain_id,
            max_clock_uncertainty_ms,
            resources: BTreeMap::new(),
            reservations: BTreeMap::new(),
            active_by_resource: BTreeMap::new(),
            time_buckets: BTreeMap::new(),
            global_by_resource: BTreeMap::new(),
            expiry_index: BTreeMap::new(),
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

    fn index_active_reservation(&mut self, request: &ReservationRequest) {
        let id = request.reservation_id.clone();
        let resource_id = request.resource_id.clone();
        self.active_by_resource
            .entry(resource_id.clone())
            .or_default()
            .insert(id.clone());

        let start = request
            .interval_start_ms
            .saturating_sub(request.clock_uncertainty_ms);
        let end = request
            .interval_end_ms
            .saturating_add(request.clock_uncertainty_ms);
        let earliest_expiry = request
            .expires_at_ms
            .saturating_sub(request.clock_uncertainty_ms);
        self.expiry_index
            .entry(earliest_expiry)
            .or_default()
            .insert(id.clone());

        if let Some(level) = reservation_time_bucket_level(start, end) {
            let width = RESERVATION_TIME_BUCKET_WIDTHS_MS[level];
            let (first, last) = reservation_time_bucket_bounds(start, end, width)
                .expect("selected temporal bucket level fits");
            for bucket in first..=last {
                self.time_buckets
                    .entry((resource_id.clone(), level, bucket))
                    .or_default()
                    .insert(id.clone());
            }
        } else {
            self.global_by_resource
                .entry(resource_id)
                .or_default()
                .insert(id);
        }
    }

    fn unindex_active_reservation(&mut self, request: &ReservationRequest) {
        let id = request.reservation_id.as_str();
        let resource_id = request.resource_id.as_str();

        if let Some(ids) = self.active_by_resource.get_mut(resource_id) {
            ids.remove(id);
            if ids.is_empty() {
                self.active_by_resource.remove(resource_id);
            }
        }

        let earliest_expiry = request
            .expires_at_ms
            .saturating_sub(request.clock_uncertainty_ms);
        if let Some(ids) = self.expiry_index.get_mut(&earliest_expiry) {
            ids.remove(id);
            if ids.is_empty() {
                self.expiry_index.remove(&earliest_expiry);
            }
        }

        let start = request
            .interval_start_ms
            .saturating_sub(request.clock_uncertainty_ms);
        let end = request
            .interval_end_ms
            .saturating_add(request.clock_uncertainty_ms);

        if let Some(level) = reservation_time_bucket_level(start, end) {
            let width = RESERVATION_TIME_BUCKET_WIDTHS_MS[level];
            let (first, last) = reservation_time_bucket_bounds(start, end, width)
                .expect("selected temporal bucket level fits");
            for bucket in first..=last {
                let key = (resource_id.to_owned(), level, bucket);
                if let Some(ids) = self.time_buckets.get_mut(&key) {
                    ids.remove(id);
                    if ids.is_empty() {
                        self.time_buckets.remove(&key);
                    }
                }
            }
        } else if let Some(ids) = self.global_by_resource.get_mut(resource_id) {
            ids.remove(id);
            if ids.is_empty() {
                self.global_by_resource.remove(resource_id);
            }
        }
    }

    fn candidate_reservation_ids(
        &self,
        resource_id: &str,
        start_ms: u64,
        end_ms: u64,
    ) -> BTreeSet<String> {
        let mut candidates = self
            .global_by_resource
            .get(resource_id)
            .cloned()
            .unwrap_or_default();
        for (level, width) in RESERVATION_TIME_BUCKET_WIDTHS_MS
            .iter()
            .copied()
            .enumerate()
        {
            let Some((first, last)) = reservation_time_bucket_bounds(start_ms, end_ms, width)
            else {
                return self
                    .active_by_resource
                    .get(resource_id)
                    .cloned()
                    .unwrap_or_default();
            };
            for bucket in first..=last {
                if let Some(ids) = self
                    .time_buckets
                    .get(&(resource_id.to_owned(), level, bucket))
                {
                    candidates.extend(ids.iter().cloned());
                }
            }
        }
        candidates
    }

    /// Diagnostic broad-phase candidate count for one resource and uncertainty-expanded
    /// half-open interval. Exact capacity semantics are still checked by `reserve`.
    pub fn candidate_count_for_interval(
        &self,
        resource_id: &str,
        start_ms: u64,
        end_ms: u64,
    ) -> Result<usize, ReservationError> {
        require_non_empty("resource_id", resource_id)?;
        if start_ms >= end_ms {
            return Err(ReservationError::InvalidInterval);
        }
        if !self.resources.contains_key(resource_id) {
            return Err(ReservationError::UnknownResource);
        }
        Ok(self
            .candidate_reservation_ids(resource_id, start_ms, end_ms)
            .len())
    }

    /// Independent full-scan reference count for differential tests and benchmarks.
    /// Endpoints must already include the caller's conservative clock uncertainty.
    /// This intentionally walks reservation history and is not used for admission.
    pub fn reference_candidate_count_for_interval(
        &self,
        resource_id: &str,
        start_ms: u64,
        end_ms: u64,
    ) -> Result<usize, ReservationError> {
        require_non_empty("resource_id", resource_id)?;
        if start_ms >= end_ms {
            return Err(ReservationError::InvalidInterval);
        }
        if !self.resources.contains_key(resource_id) {
            return Err(ReservationError::UnknownResource);
        }

        Ok(self
            .reservations
            .values()
            .filter(|record| {
                if record.state != ReservationState::Active
                    || record.request.resource_id != resource_id
                {
                    return false;
                }
                let existing_start = record
                    .request
                    .interval_start_ms
                    .saturating_sub(record.request.clock_uncertainty_ms);
                let existing_end = record
                    .request
                    .interval_end_ms
                    .saturating_add(record.request.clock_uncertainty_ms);
                overlaps(start_ms, end_ms, existing_start, existing_end)
            })
            .count())
    }

    /// Number of active reservations for one declared resource. Historical IDs
    /// remain in the ledger but do not consume active capacity.
    pub fn active_reservation_count_for_resource(
        &self,
        resource_id: &str,
    ) -> Result<usize, ReservationError> {
        require_non_empty("resource_id", resource_id)?;
        if !self.resources.contains_key(resource_id) {
            return Err(ReservationError::UnknownResource);
        }
        Ok(self
            .active_by_resource
            .get(resource_id)
            .map_or(0, BTreeSet::len))
    }

    /// Independent full-history expiry count for differential tests and benchmarks.
    /// The production `expire` path uses the ordered expiry index instead.
    pub fn reference_due_count_at(&self, now_ms: u64) -> usize {
        self.reservations
            .values()
            .filter(|record| {
                record.state == ReservationState::Active
                    && record
                        .request
                        .expires_at_ms
                        .saturating_sub(record.request.clock_uncertainty_ms)
                        <= now_ms
            })
            .count()
    }

    /// Reserve capacity in a declared clock domain and current site epoch.
    /// Overlap is evaluated on half-open intervals [start, end).
    /// Read-only admission preview using the same exact interval and peak-capacity
    /// semantics as `reserve`. This method does not expire, insert, release or otherwise
    /// mutate records or indexes. An active record whose earliest possible expiry is
    /// already due at `now_ms` is treated exactly as `reserve` would treat it after
    /// lazy expiry (historical ID consumed, no longer capacity-consuming).
    pub fn preview_reservation(
        &self,
        request: &ReservationRequest,
        now_ms: u64,
    ) -> Result<ReservationOutcome, ReservationError> {
        validate_request(request)?;

        if request.site_epoch != self.current_site_epoch {
            return Err(ReservationError::StaleSiteEpoch);
        }
        if request.clock_domain_id != self.clock_domain_id {
            return Err(ReservationError::ClockDomainMismatch);
        }
        if request.clock_uncertainty_ms > self.max_clock_uncertainty_ms {
            return Err(ReservationError::ClockUncertaintyExceeded {
                allowed_ms: self.max_clock_uncertainty_ms,
                actual_ms: request.clock_uncertainty_ms,
            });
        }

        let capacity = self
            .resources
            .get(&request.resource_id)
            .copied()
            .ok_or(ReservationError::UnknownResource)?;

        if let Some(existing) = self.reservations.get(&request.reservation_id) {
            if existing.state != ReservationState::Active
                || existing
                    .request
                    .expires_at_ms
                    .saturating_sub(existing.request.clock_uncertainty_ms)
                    <= now_ms
            {
                return Err(ReservationError::ReservationIdConsumed);
            }
            return if existing.request == *request {
                Ok(ReservationOutcome::Duplicate)
            } else {
                Err(ReservationError::ReservationIdConflict)
            };
        }

        let uncertainty = request.clock_uncertainty_ms;
        let earliest_start = request.interval_start_ms.saturating_sub(uncertainty);
        let latest_end = request.interval_end_ms.saturating_add(uncertainty);
        let earliest_expiry = request.expires_at_ms.saturating_sub(uncertainty);
        if earliest_expiry <= now_ms {
            return Err(ReservationError::RequestAlreadyExpired);
        }
        if earliest_start < now_ms {
            return Err(ReservationError::InvalidInterval);
        }
        if earliest_expiry < latest_end {
            return Err(ReservationError::ExpiryBeforeEnd);
        }

        let mut occupancy_events: Vec<(u64, bool, u64)> = Vec::new();
        let candidate_ids =
            self.candidate_reservation_ids(&request.resource_id, earliest_start, latest_end);
        for existing_id in candidate_ids {
            let existing = self
                .reservations
                .get(&existing_id)
                .ok_or_else(|| ReservationError::IndexInconsistent(existing_id.clone()))?;
            if existing.request.resource_id != request.resource_id {
                return Err(ReservationError::IndexInconsistent(existing_id));
            }
            if existing.state != ReservationState::Active {
                return Err(ReservationError::IndexInconsistent(existing_id));
            }
            let existing_earliest_expiry = existing
                .request
                .expires_at_ms
                .saturating_sub(existing.request.clock_uncertainty_ms);
            if existing_earliest_expiry <= now_ms {
                // The real mutating path would expire and unindex this record before
                // capacity admission. Preview preserves state and simply ignores it.
                continue;
            }

            let existing_start = existing
                .request
                .interval_start_ms
                .saturating_sub(existing.request.clock_uncertainty_ms);
            let existing_end = existing
                .request
                .interval_end_ms
                .saturating_add(existing.request.clock_uncertainty_ms);
            if overlaps(earliest_start, latest_end, existing_start, existing_end) {
                occupancy_events.push((existing_start, true, u64::from(existing.request.units)));
                occupancy_events.push((existing_end, false, u64::from(existing.request.units)));
            }
        }
        occupancy_events.push((earliest_start, true, u64::from(request.units)));
        occupancy_events.push((latest_end, false, u64::from(request.units)));

        // For half-open intervals, process end events before start events at the same
        // timestamp. The peak concurrent occupancy, not the sum of all intersecting
        // reservations, determines whether the candidate can be admitted.
        occupancy_events.sort_unstable_by_key(|(timestamp, is_start, _)| (*timestamp, *is_start));
        let mut current_units = 0_u64;
        let mut peak_units = 0_u64;
        for (_, is_start, units) in occupancy_events {
            if is_start {
                current_units = current_units
                    .checked_add(units)
                    .ok_or(ReservationError::CapacityOverflow)?;
                peak_units = peak_units.max(current_units);
            } else {
                current_units = current_units
                    .checked_sub(units)
                    .ok_or(ReservationError::CapacityOverflow)?;
            }
        }

        if peak_units > u64::from(capacity) {
            return Err(ReservationError::CapacityExceeded {
                capacity,
                requested_total: u32::try_from(peak_units)
                    .map_err(|_| ReservationError::CapacityOverflow)?,
            });
        }

        Ok(ReservationOutcome::Granted)
    }

    /// Reserve capacity in a declared clock domain and current site epoch.
    /// Overlap is evaluated on half-open intervals [start, end).
    pub fn reserve(
        &mut self,
        request: ReservationRequest,
        now_ms: u64,
    ) -> Result<ReservationOutcome, ReservationError> {
        // Preserve the prior side-effect order: malformed requests do not trigger
        // lazy expiry of unrelated records.
        validate_request(&request)?;
        self.expire(now_ms);
        let outcome = self.preview_reservation(&request, now_ms)?;
        if outcome == ReservationOutcome::Duplicate {
            return Ok(outcome);
        }

        let indexed_request = request.clone();
        self.reservations.insert(
            request.reservation_id.clone(),
            ReservationRecord {
                request,
                state: ReservationState::Active,
            },
        );
        self.index_active_reservation(&indexed_request);
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

        let (outcome, request_to_unindex) = match record.state {
            ReservationState::Active => {
                record.state = ReservationState::Released;
                (Ok(ReleaseOutcome::Released), Some(record.request.clone()))
            }
            ReservationState::Released => (Ok(ReleaseOutcome::AlreadyReleased), None),
            ReservationState::Expired | ReservationState::Invalidated => {
                (Err(ReservationError::ReservationNotActive), None)
            }
        };
        if let Some(request) = request_to_unindex {
            self.unindex_active_reservation(&request);
        }
        outcome
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
        self.active_by_resource.clear();
        self.time_buckets.clear();
        self.global_by_resource.clear();
        self.expiry_index.clear();
        Ok(())
    }

    /// Mark expired reservations as historical records. Their IDs remain consumed.
    pub fn expire(&mut self, now_ms: u64) -> usize {
        let due_reservation_ids: Vec<String> = self
            .expiry_index
            .range(..=now_ms)
            .flat_map(|(_, reservation_ids)| reservation_ids.iter().cloned())
            .collect();

        let expired_requests: Vec<ReservationRequest> = due_reservation_ids
            .iter()
            .filter_map(|reservation_id| {
                self.reservations
                    .get(reservation_id)
                    .filter(|record| record.state == ReservationState::Active)
                    .map(|record| record.request.clone())
            })
            .collect();

        for request in &expired_requests {
            self.unindex_active_reservation(request);
            if let Some(record) = self.reservations.get_mut(&request.reservation_id) {
                record.state = ReservationState::Expired;
            }
        }
        expired_requests.len()
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

/// Return conservative inclusive broad-phase bucket bounds for an uncertainty-expanded
/// half-open interval. Including the end bucket may add candidates at exact boundaries;
/// the final `overlaps` predicate still enforces half-open semantics.
fn reservation_time_bucket_bounds(
    start_ms: u64,
    end_ms: u64,
    bucket_width_ms: u64,
) -> Option<(u64, u64)> {
    if start_ms >= end_ms || bucket_width_ms == 0 {
        return None;
    }
    let first = start_ms / bucket_width_ms;
    let last = end_ms / bucket_width_ms;
    let count = last.checked_sub(first)?.checked_add(1)?;
    if count > MAX_RESERVATION_TIME_BUCKETS || usize::try_from(count).is_err() {
        return None;
    }
    Some((first, last))
}

/// Select the finest temporal bucket level that can represent an interval within
/// the fixed index-entry budget. Very long intervals use the resource-wide fallback.
fn reservation_time_bucket_level(start_ms: u64, end_ms: u64) -> Option<usize> {
    RESERVATION_TIME_BUCKET_WIDTHS_MS
        .iter()
        .copied()
        .enumerate()
        .find_map(|(level, width)| {
            reservation_time_bucket_bounds(start_ms, end_ms, width).map(|_| level)
        })
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
        book_with_uncertainty(capacity, 0)
    }

    fn book_with_uncertainty(capacity: u32, max_uncertainty_ms: u64) -> ReservationBook {
        let mut book =
            ReservationBook::new("site-epoch-1", "site-monotonic-ms", max_uncertainty_ms)
                .expect("valid reservation book");
        book.register_resource("work-zone-A", capacity)
            .expect("valid resource");
        book
    }

    fn assert_index_contains_every_exact_overlap(
        book: &ReservationBook,
        resource_id: &str,
        start_ms: u64,
        end_ms: u64,
    ) {
        let indexed = book.candidate_reservation_ids(resource_id, start_ms, end_ms);
        let exact_overlaps: BTreeSet<String> = book
            .reservations
            .iter()
            .filter_map(|(reservation_id, record)| {
                if record.state != ReservationState::Active
                    || record.request.resource_id != resource_id
                {
                    return None;
                }
                let existing_start = record
                    .request
                    .interval_start_ms
                    .saturating_sub(record.request.clock_uncertainty_ms);
                let existing_end = record
                    .request
                    .interval_end_ms
                    .saturating_add(record.request.clock_uncertainty_ms);
                (start_ms < existing_end && existing_start < end_ms).then(|| reservation_id.clone())
            })
            .collect();

        assert!(
            exact_overlaps.is_subset(&indexed),
            "temporal index omitted exact overlaps: exact={exact_overlaps:?}, indexed={indexed:?}, query=[{start_ms},{end_ms})"
        );
        for reservation_id in indexed {
            let record = book
                .reservations
                .get(&reservation_id)
                .expect("candidate ID resolves to a reservation");
            assert_eq!(record.state, ReservationState::Active);
            assert_eq!(record.request.resource_id, resource_id);
        }
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
            clock_uncertainty_ms: 0,
            interval_start_ms: start,
            interval_end_ms: end,
            expires_at_ms: expiry,
            units,
        }
    }

    #[test]
    fn reference_full_scan_and_indexed_broad_phase_agree_on_overlap_completeness() {
        let mut book = book_with_uncertainty(100, 500);
        let records = [
            (100, 200, 0),
            (59_900, 60_100, 50),
            (60_100, 60_200, 100),
            (119_900, 120_100, 200),
            (180_000, 240_000, 500),
        ];
        for (index, (start, end, uncertainty)) in records.into_iter().enumerate() {
            let mut item = request(
                &format!("reference-{index}"),
                &format!("robot-{index}"),
                start,
                end,
                end + 2_000,
                1,
            );
            item.clock_uncertainty_ms = uncertainty;
            book.reserve(item, 0).expect("reference reservation");
        }

        for start in [
            0_u64, 50, 100, 59_800, 59_999, 60_000, 60_200, 119_999, 180_000,
        ] {
            for end in [start + 1, start + 50, start + 500, start + 2_000] {
                let indexed = book.candidate_reservation_ids("work-zone-A", start, end);
                let reference = book
                    .reference_candidate_count_for_interval("work-zone-A", start, end)
                    .expect("full scan reference");
                let exact_ids: BTreeSet<String> = book
                    .reservations
                    .iter()
                    .filter_map(|(id, record)| {
                        if record.state != ReservationState::Active
                            || record.request.resource_id != "work-zone-A"
                        {
                            return None;
                        }
                        let existing_start = record
                            .request
                            .interval_start_ms
                            .saturating_sub(record.request.clock_uncertainty_ms);
                        let existing_end = record
                            .request
                            .interval_end_ms
                            .saturating_add(record.request.clock_uncertainty_ms);
                        overlaps(start, end, existing_start, existing_end).then(|| id.clone())
                    })
                    .collect();
                assert_eq!(reference, exact_ids.len());
                assert!(exact_ids.is_subset(&indexed));
            }
        }
    }

    #[test]
    fn temporal_index_has_no_false_negatives_across_bucket_boundaries() {
        let mut book = book_with_uncertainty(100, 1_000);
        let cases = [
            (59_900, 60_100, 0),
            (60_000, 60_500, 10),
            (119_900, 120_100, 0),
            (200_000, 250_000, 1_000),
            (300_000, 300_010, 0),
        ];

        for (index, (start, end, uncertainty)) in cases.into_iter().enumerate() {
            let mut item = request(
                &format!("indexed-{index}"),
                &format!("robot-{index}"),
                start,
                end,
                end + 5_000,
                1,
            );
            item.clock_uncertainty_ms = uncertainty;
            book.reserve(item, 0).expect("frozen reservation");
        }

        let starts = [
            0_u64, 1, 59_899, 59_990, 60_000, 60_100, 119_999, 120_000, 200_000, 249_999, 299_999,
            300_000,
        ];
        let durations = [1_u64, 10, 100, 1_000, 60_000, 120_000];
        for start in starts {
            for duration in durations {
                let end = start.saturating_add(duration);
                if start < end {
                    assert_index_contains_every_exact_overlap(&book, "work-zone-A", start, end);
                }
            }
        }
    }

    #[test]
    fn multi_resolution_index_selects_the_finest_bounded_level() {
        assert_eq!(reservation_time_bucket_level(10, 20), Some(0));
        assert_eq!(reservation_time_bucket_level(10, 5_000_010), Some(1));
        assert_eq!(reservation_time_bucket_level(10, 100_000_010), Some(2));
        assert_eq!(reservation_time_bucket_level(10, 300_000_010), Some(3));
        assert_eq!(reservation_time_bucket_level(10, 3_000_000_010), None);

        let mut book = book(100);
        let requests = [
            request("level-0", "robot-0", 10, 20, 100, 1),
            request("level-1", "robot-1", 10, 5_000_010, 5_001_000, 1),
            request("level-2", "robot-2", 10, 100_000_010, 100_001_000, 1),
            request("level-3", "robot-3", 10, 300_000_010, 300_001_000, 1),
        ];
        for item in requests {
            book.reserve(item, 0).expect("multi-resolution reservation");
        }

        for reservation_id in ["level-0", "level-1", "level-2", "level-3"] {
            let record = book
                .reservations
                .get(reservation_id)
                .expect("indexed reservation exists");
            let start = record.request.interval_start_ms;
            let end = record.request.interval_end_ms;
            assert_index_contains_every_exact_overlap(&book, "work-zone-A", start, end);
        }
    }

    #[test]
    fn long_reservations_use_per_resource_global_fallback() {
        let mut book = book(10);
        let long_end = (MAX_RESERVATION_TIME_BUCKETS + 100) * RESERVATION_TIME_BUCKET_WIDTHS_MS[3];
        let long = request(
            "long-global",
            "robot-long",
            1_000,
            long_end,
            long_end + 10_000,
            1,
        );
        book.reserve(long, 0).expect("long reservation");
        assert_eq!(
            book.candidate_count_for_interval("work-zone-A", long_end - 5, long_end + 5),
            Ok(1)
        );
        assert_index_contains_every_exact_overlap(&book, "work-zone-A", long_end - 5, long_end + 5);
    }

    #[test]
    fn temporal_index_is_partitioned_by_resource() {
        let mut book = book(10);
        book.register_resource("work-zone-B", 10)
            .expect("second resource");
        let first = request("resource-a", "robot-a", 100, 200, 300, 1);
        book.reserve(first, 0).expect("resource A reservation");
        let mut second = request("resource-b", "robot-b", 100, 200, 300, 1);
        second.resource_id = "work-zone-B".to_owned();
        book.reserve(second, 0).expect("resource B reservation");

        assert_eq!(
            book.candidate_count_for_interval("work-zone-A", 100, 200),
            Ok(1)
        );
        assert_eq!(
            book.candidate_count_for_interval("work-zone-B", 100, 200),
            Ok(1)
        );
        // A one-second bucket separates this remote interval; exact and indexed
        // queries both return no candidate.
        assert_eq!(
            book.candidate_count_for_interval("work-zone-A", 10_000, 20_000),
            Ok(0)
        );
        assert_eq!(
            book.reference_candidate_count_for_interval("work-zone-A", 10_000, 20_000),
            Ok(0)
        );
    }

    #[test]
    fn release_expiry_and_site_epoch_remove_active_index_entries() {
        let mut released = book(2);
        released
            .reserve(request("release-me", "robot-a", 100, 200, 300, 1), 0)
            .expect("reservation");
        assert_eq!(
            released.candidate_count_for_interval("work-zone-A", 100, 200),
            Ok(1)
        );
        released
            .release("release-me", "robot-a")
            .expect("owner releases reservation");
        assert_eq!(
            released.candidate_count_for_interval("work-zone-A", 100, 200),
            Ok(0)
        );
        assert_eq!(
            released.state("release-me"),
            Some(ReservationState::Released)
        );
        assert!(released.expiry_index.is_empty());

        let mut expired = book(2);
        expired
            .reserve(request("expire-me", "robot-b", 100, 200, 200, 1), 0)
            .expect("reservation");
        assert_eq!(expired.expire(200), 1);
        assert_eq!(
            expired.candidate_count_for_interval("work-zone-A", 100, 200),
            Ok(0)
        );
        assert_eq!(expired.state("expire-me"), Some(ReservationState::Expired));
        assert!(expired.expiry_index.is_empty());

        let mut invalidated = book(2);
        invalidated
            .reserve(request("invalidate-me", "robot-c", 100, 200, 300, 1), 0)
            .expect("reservation");
        invalidated
            .advance_site_epoch("site-epoch-2")
            .expect("site epoch advances");
        assert_eq!(
            invalidated.candidate_count_for_interval("work-zone-A", 100, 200),
            Ok(0)
        );
        assert_eq!(
            invalidated.state("invalidate-me"),
            Some(ReservationState::Invalidated)
        );
        assert!(invalidated.expiry_index.is_empty());
    }

    #[test]
    fn expiry_index_and_full_history_oracle_agree_on_due_count() {
        let mut book = book(10);
        for index in 0..100_u64 {
            let mut item = request(
                &format!("expiry-{index}"),
                &format!("robot-{index}"),
                10_000 + index * 100,
                10_050 + index * 100,
                10_150 + index * 100,
                1,
            );
            item.clock_uncertainty_ms = 0;
            book.reserve(item, 0).expect("due reservation");
        }
        for index in 100..200_u64 {
            let item = request(
                &format!("future-{index}"),
                &format!("robot-{index}"),
                1_000_000 + index * 100,
                1_000_050 + index * 100,
                10_000_000_000,
                1,
            );
            book.reserve(item, 0).expect("future reservation");
        }

        assert_eq!(book.reference_due_count_at(20_100), 100);
        assert_eq!(book.expire(20_100), 100);
        assert_eq!(book.reference_due_count_at(20_100), 0);
        assert_eq!(
            book.active_reservation_count_for_resource("work-zone-A"),
            Ok(100)
        );
    }

    #[test]
    fn duplicate_reservation_does_not_duplicate_index_membership() {
        let mut book = book(1);
        let item = request("stable-id", "robot-a", 100, 200, 300, 1);
        assert_eq!(
            book.reserve(item.clone(), 0),
            Ok(ReservationOutcome::Granted)
        );
        assert_eq!(book.reserve(item, 0), Ok(ReservationOutcome::Duplicate));
        assert_eq!(
            book.candidate_count_for_interval("work-zone-A", 100, 200),
            Ok(1)
        );
        assert_eq!(
            book.expiry_index.values().map(BTreeSet::len).sum::<usize>(),
            1
        );
    }

    #[test]
    fn preview_matches_reserve_without_mutating_the_book() {
        let mut book = book(1);
        book.reserve(request("existing", "robot-a", 100, 200, 300, 1), 0)
            .expect("existing reservation");
        let before = book.clone();
        let candidate = request("candidate", "robot-b", 150, 250, 350, 1);

        assert_eq!(
            book.preview_reservation(&candidate, 0),
            Err(ReservationError::CapacityExceeded {
                capacity: 1,
                requested_total: 2
            })
        );
        assert_eq!(book, before);

        let mut reference = before;
        assert_eq!(
            reference.reserve(candidate, 0),
            Err(ReservationError::CapacityExceeded {
                capacity: 1,
                requested_total: 2
            })
        );
        assert_eq!(book, reference);
    }

    #[test]
    fn preview_handles_duplicate_stale_expired_and_future_requests_like_reserve() {
        let mut book = book_with_uncertainty(2, 10);
        let mut active = request("active", "robot-a", 100, 200, 240, 1);
        active.clock_uncertainty_ms = 10;
        book.reserve(active.clone(), 0).expect("active reservation");
        let before = book.clone();

        assert_eq!(
            book.preview_reservation(&active, 0),
            Ok(ReservationOutcome::Duplicate)
        );
        assert_eq!(book, before);

        let mut stale = active.clone();
        stale.site_epoch = "old-site-epoch".to_owned();
        assert_eq!(
            book.preview_reservation(&stale, 0),
            Err(ReservationError::StaleSiteEpoch)
        );
        assert_eq!(book, before);

        assert_eq!(
            book.preview_reservation(&active, 230),
            Err(ReservationError::ReservationIdConsumed)
        );
        assert_eq!(book, before);

        let future = request("future", "robot-b", 300, 400, 500, 1);
        assert_eq!(
            book.preview_reservation(&future, 0),
            Ok(ReservationOutcome::Granted)
        );
        // Preview did not create the new booking or mutate expiry/index state.
        assert_eq!(book, before);
        assert_eq!(book.state("future"), None);
    }

    #[test]
    fn preview_ignores_due_records_without_mutating_expiry_or_time_indexes() {
        let mut book = book(1);
        book.reserve(request("due", "robot-a", 100, 200, 200, 1), 0)
            .expect("due reservation");
        let before = book.clone();
        let candidate = request("replacement", "robot-b", 300, 400, 500, 1);

        assert_eq!(
            book.preview_reservation(&candidate, 200),
            Ok(ReservationOutcome::Granted)
        );
        assert_eq!(book, before);
        assert_eq!(book.state("due"), Some(ReservationState::Active));
        assert_eq!(
            book.candidate_count_for_interval("work-zone-A", 100, 200),
            Ok(1)
        );
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
        assert_eq!(book.reserve(first, 50), Ok(ReservationOutcome::Duplicate));
        assert_eq!(
            book.reserve(request("r1", "robot-2", 100, 200, 250, 1), 50),
            Err(ReservationError::ReservationIdConflict)
        );
    }

    #[test]
    fn capacity_sweep_accepts_candidate_across_disjoint_existing_reservations() {
        let mut book = book(2);
        book.reserve(request("r1", "robot-1", 100, 200, 250, 1), 50)
            .expect("first interval");
        book.reserve(request("r2", "robot-2", 200, 300, 350, 1), 50)
            .expect("second disjoint interval");

        assert_eq!(
            book.reserve(request("r3", "robot-3", 100, 300, 350, 1), 50),
            Ok(ReservationOutcome::Granted)
        );
    }

    #[test]
    fn capacity_sweep_rejects_peak_concurrent_overbooking() {
        let mut book = book(2);
        book.reserve(request("r1", "robot-1", 100, 250, 300, 1), 50)
            .expect("first interval");
        book.reserve(request("r2", "robot-2", 200, 300, 350, 1), 50)
            .expect("second interval");

        assert_eq!(
            book.reserve(request("r3", "robot-3", 100, 300, 350, 1), 50),
            Err(ReservationError::CapacityExceeded {
                capacity: 2,
                requested_total: 3
            })
        );
    }

    #[test]
    fn uncertainty_expanded_boundaries_use_half_open_peak_occupancy() {
        let mut book = book_with_uncertainty(1, 5);
        let mut first = request("r1", "robot-1", 100, 200, 230, 1);
        first.clock_uncertainty_ms = 5;
        book.reserve(first, 50).expect("first reservation");

        let mut touching = request("r2", "robot-2", 209, 300, 330, 1);
        touching.clock_uncertainty_ms = 5;
        assert_eq!(
            book.reserve(touching, 50),
            Err(ReservationError::CapacityExceeded {
                capacity: 1,
                requested_total: 2
            })
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
        book.advance_site_epoch("site-epoch-2").expect("new epoch");
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
        assert_eq!(book.release("r1", "robot-1"), Ok(ReleaseOutcome::Released));
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
    fn clock_uncertainty_over_the_declared_policy_is_rejected() {
        let mut book = book_with_uncertainty(1, 5);
        let mut uncertain = request("r1", "robot-1", 100, 200, 240, 1);
        uncertain.clock_uncertainty_ms = 6;
        assert_eq!(
            book.reserve(uncertain, 50),
            Err(ReservationError::ClockUncertaintyExceeded {
                allowed_ms: 5,
                actual_ms: 6
            })
        );
    }

    #[test]
    fn uncertainty_expands_intervals_and_detects_near_boundary_conflicts() {
        let mut book = book_with_uncertainty(1, 10);
        let mut first = request("r1", "robot-1", 100, 200, 240, 1);
        first.clock_uncertainty_ms = 10;
        book.reserve(first, 50).expect("first reservation");

        let mut second = request("r2", "robot-2", 215, 300, 340, 1);
        second.clock_uncertainty_ms = 10;
        assert_eq!(
            book.reserve(second, 50),
            Err(ReservationError::CapacityExceeded {
                capacity: 1,
                requested_total: 2
            })
        );
    }

    #[test]
    fn uncertainty_requires_expiry_after_the_latest_possible_interval_end() {
        let mut book = book_with_uncertainty(1, 10);
        let mut request = request("r1", "robot-1", 100, 200, 210, 1);
        request.clock_uncertainty_ms = 10;
        assert_eq!(
            book.reserve(request, 50),
            Err(ReservationError::ExpiryBeforeEnd)
        );
    }

    #[test]
    fn uncertainty_cannot_make_a_past_start_appear_future() {
        let mut book = book_with_uncertainty(1, 10);
        let mut request = request("r1", "robot-1", 55, 100, 130, 1);
        request.clock_uncertainty_ms = 10;
        assert_eq!(
            book.reserve(request, 50),
            Err(ReservationError::InvalidInterval)
        );
    }

    #[test]
    fn resource_capacity_must_be_positive() {
        let mut book =
            ReservationBook::new("site-epoch-1", "site-monotonic-ms", 0).expect("valid book");
        assert_eq!(
            book.register_resource("work-zone-A", 0),
            Err(ReservationError::ZeroCapacity)
        );
    }
}
