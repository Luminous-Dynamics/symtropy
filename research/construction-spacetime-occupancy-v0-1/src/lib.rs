//! Conservative spatial-temporal occupancy overlap oracle.
//!
//! This module composes exact site/design/frame identity with the time interval
//! and clock uncertainty declared by the existing construction interop contract.
//! It detects potential overlap between conservative axis-aligned outer bounds.
//! It is not a collision-avoidance system, safety controller, clearance authority,
//! trajectory planner, BIM parser, coordinate transform engine, or motion authority.

use std::collections::BTreeMap;

use construction_robot_interop_core::ReservationRequest;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SiteBinding {
    pub floor_id: String,
    pub site_epoch: String,
    pub design_revision_digest: String,
    pub target_frame_id: String,
}

impl SiteBinding {
    fn validate(&self) -> Result<(), SpatialError> {
        require_non_empty("floor_id", &self.floor_id)?;
        require_non_empty("site_epoch", &self.site_epoch)?;
        require_non_empty("design_revision_digest", &self.design_revision_digest)?;
        require_non_empty("target_frame_id", &self.target_frame_id)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OccupancyKind {
    Workface,
    RobotOperatingEnvelope,
    ToolAndPayloadEnvelope,
    TransitCorridor,
    CraneOrPlantSweptOuterBound,
    LiftingPlanningEnvelope,
    MaterialStaging,
    InspectionAccess,
    TemporaryWorksReconfiguration,
    ExclusionEnvelope,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bounds3Mm {
    pub min: [i64; 3],
    pub max: [i64; 3],
}

impl Bounds3Mm {
    fn validate(&self) -> Result<(), SpatialError> {
        for (axis, (minimum, maximum)) in
            self.min.iter().zip(self.max.iter()).enumerate()
        {
            if minimum >= maximum {
                return Err(SpatialError::InvalidBounds { axis });
            }
        }
        Ok(())
    }

    fn expanded(
        &self,
        uncertainty_mm: [u64; 3],
        clearance_mm: [u64; 3],
    ) -> Result<Self, SpatialError> {
        self.validate()?;
        let mut min = self.min;
        let mut max = self.max;

        for (axis, ((minimum, maximum), (uncertainty, clearance))) in self
            .min
            .iter()
            .zip(self.max.iter())
            .zip(uncertainty_mm.iter().zip(clearance_mm.iter()))
            .enumerate()
        {
            let total_margin = uncertainty
                .checked_add(*clearance)
                .ok_or(SpatialError::ExpansionOverflow { axis })?;
            let margin = i64::try_from(total_margin)
                .map_err(|_| SpatialError::ExpansionOverflow { axis })?;
            min[axis] = minimum
                .checked_sub(margin)
                .ok_or(SpatialError::ExpansionOverflow { axis })?;
            max[axis] = maximum
                .checked_add(margin)
                .ok_or(SpatialError::ExpansionOverflow { axis })?;
        }

        Ok(Self { min, max })
    }

    /// Closed spatial intersection: touching expanded boundaries are a potential
    /// conflict. This conservative policy is not a physical collision verdict.
    fn intersection_closed(&self, other: &Self) -> Option<Self> {
        let separated = self
            .min
            .iter()
            .zip(self.max.iter())
            .zip(other.min.iter().zip(other.max.iter()))
            .any(|((left_min, left_max), (right_min, right_max))| {
                left_min > right_max || right_min > left_max
            });
        if separated {
            return None;
        }

        let min = std::array::from_fn(|axis| self.min[axis].max(other.min[axis]));
        let max = std::array::from_fn(|axis| self.max[axis].min(other.max[axis]));
        Some(Self { min, max })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpatialOccupancy {
    pub occupancy_id: String,
    pub task_id: String,
    pub kind: OccupancyKind,
    pub binding: SiteBinding,
    /// Conservative axis-aligned outer bounds in the declared target frame.
    pub bounds: Bounds3Mm,
    /// Measurement / geometry uncertainty, separate from policy clearance.
    pub uncertainty_margin_mm: [u64; 3],
    /// Project-declared planning clearance, not a safety authorization.
    pub clearance_margin_mm: [u64; 3],
    /// The exact time/resource request used by the existing #3644 contract.
    /// The caller is responsible for admitting this request to its ReservationBook.
    pub reservation: ReservationRequest,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConflictRecord {
    pub existing_occupancy_id: String,
    pub candidate_occupancy_id: String,
    pub existing_task_id: String,
    pub candidate_task_id: String,
    pub existing_kind: OccupancyKind,
    pub candidate_kind: OccupancyKind,
    /// Intersection of the two uncertainty/clearance-expanded outer bounds.
    pub overlap_bounds_mm: Bounds3Mm,
    /// Intersection of the two uncertainty-expanded half-open time intervals.
    pub overlap_start_ms: u64,
    pub overlap_end_ms: u64,
    pub clock_domain_id: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InsertOutcome {
    AdmittedToSpatialIndex,
    Duplicate,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SpatialError {
    EmptyField(&'static str),
    InvalidBounds { axis: usize },
    ExpansionOverflow { axis: usize },
    TimeExpansionOverflow,
    StaleFloor,
    StaleSiteEpoch,
    StaleDesignRevision,
    FrameMismatch,
    ClockDomainMismatch,
    ReservationTaskMismatch,
    InvalidReservationInterval,
    OccupancyIdConflict(String),
    PotentialConflict(ConflictRecord),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpatialOccupancyBook {
    binding: SiteBinding,
    clock_domain_id: String,
    occupancies: BTreeMap<String, SpatialOccupancy>,
}

impl SpatialOccupancyBook {
    pub fn new(
        binding: SiteBinding,
        clock_domain_id: impl Into<String>,
    ) -> Result<Self, SpatialError> {
        binding.validate()?;
        let clock_domain_id = clock_domain_id.into();
        require_non_empty("clock_domain_id", &clock_domain_id)?;

        Ok(Self {
            binding,
            clock_domain_id,
            occupancies: BTreeMap::new(),
        })
    }

    pub fn binding(&self) -> &SiteBinding {
        &self.binding
    }

    pub fn occupancy_count(&self) -> usize {
        self.occupancies.len()
    }

    /// Insert a planned occupancy after its corresponding time/resource request has
    /// been admitted by the existing reservation owner. This method checks the exact
    /// declared reservation fields for task, site epoch, clock domain and interval.
    /// It cannot prove that a caller actually obtained a grant from ReservationBook.
    pub fn insert(
        &mut self,
        occupancy: SpatialOccupancy,
    ) -> Result<InsertOutcome, SpatialError> {
        require_non_empty("occupancy_id", &occupancy.occupancy_id)?;
        require_non_empty("task_id", &occupancy.task_id)?;
        require_non_empty("reservation_id", &occupancy.reservation.reservation_id)?;
        require_non_empty("reservation_resource_id", &occupancy.reservation.resource_id)?;
        require_non_empty("reservation_owner_id", &occupancy.reservation.owner_id)?;
        require_non_empty("reservation_task_id", &occupancy.reservation.task_id)?;
        require_non_empty("reservation_site_epoch", &occupancy.reservation.site_epoch)?;
        require_non_empty("reservation_clock_domain_id", &occupancy.reservation.clock_domain_id)?;

        occupancy.bounds.validate()?;
        let expanded_candidate = occupancy.bounds.expanded(
            occupancy.uncertainty_margin_mm,
            occupancy.clearance_margin_mm,
        )?;

        if occupancy.binding.floor_id != self.binding.floor_id {
            return Err(SpatialError::StaleFloor);
        }
        if occupancy.binding.site_epoch != self.binding.site_epoch
            || occupancy.reservation.site_epoch != self.binding.site_epoch
        {
            return Err(SpatialError::StaleSiteEpoch);
        }
        if occupancy.binding.design_revision_digest != self.binding.design_revision_digest {
            return Err(SpatialError::StaleDesignRevision);
        }
        if occupancy.binding.target_frame_id != self.binding.target_frame_id {
            return Err(SpatialError::FrameMismatch);
        }
        if occupancy.reservation.clock_domain_id != self.clock_domain_id {
            return Err(SpatialError::ClockDomainMismatch);
        }
        if occupancy.reservation.task_id != occupancy.task_id {
            return Err(SpatialError::ReservationTaskMismatch);
        }
        if occupancy.reservation.interval_start_ms >= occupancy.reservation.interval_end_ms {
            return Err(SpatialError::InvalidReservationInterval);
        }

        if let Some(previous) = self.occupancies.get(&occupancy.occupancy_id) {
            return if previous == &occupancy {
                Ok(InsertOutcome::Duplicate)
            } else {
                Err(SpatialError::OccupancyIdConflict(occupancy.occupancy_id))
            };
        }

        let (candidate_start, candidate_end) = expanded_time_interval(&occupancy.reservation)?;

        for existing in self.occupancies.values() {
            let (existing_start, existing_end) = expanded_time_interval(&existing.reservation)?;
            let overlap_start = candidate_start.max(existing_start);
            let overlap_end = candidate_end.min(existing_end);
            if overlap_start >= overlap_end {
                continue;
            }

            let expanded_existing = existing.bounds.expanded(
                existing.uncertainty_margin_mm,
                existing.clearance_margin_mm,
            )?;
            if let Some(overlap_bounds_mm) =
                expanded_candidate.intersection_closed(&expanded_existing)
            {
                return Err(SpatialError::PotentialConflict(ConflictRecord {
                    existing_occupancy_id: existing.occupancy_id.clone(),
                    candidate_occupancy_id: occupancy.occupancy_id.clone(),
                    existing_task_id: existing.task_id.clone(),
                    candidate_task_id: occupancy.task_id.clone(),
                    existing_kind: existing.kind,
                    candidate_kind: occupancy.kind,
                    overlap_bounds_mm,
                    overlap_start_ms: overlap_start,
                    overlap_end_ms: overlap_end,
                    clock_domain_id: self.clock_domain_id.clone(),
                }));
            }
        }

        self.occupancies
            .insert(occupancy.occupancy_id.clone(), occupancy);
        Ok(InsertOutcome::AdmittedToSpatialIndex)
    }
}

/// Expand the existing reservation interval conservatively using its clock bound.
/// Half-open intervals remain half-open; no independent calendar or clock policy
/// is created here. Underflow/overflow is a typed failure, not a wrap or clamp.
fn expanded_time_interval(
    reservation: &ReservationRequest,
) -> Result<(u64, u64), SpatialError> {
    let start = reservation
        .interval_start_ms
        .checked_sub(reservation.clock_uncertainty_ms)
        .ok_or(SpatialError::TimeExpansionOverflow)?;
    let end = reservation
        .interval_end_ms
        .checked_add(reservation.clock_uncertainty_ms)
        .ok_or(SpatialError::TimeExpansionOverflow)?;
    if start >= end {
        return Err(SpatialError::InvalidReservationInterval);
    }
    Ok((start, end))
}

fn require_non_empty(name: &'static str, value: &str) -> Result<(), SpatialError> {
    if value.trim().is_empty() {
        return Err(SpatialError::EmptyField(name));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn binding() -> SiteBinding {
        SiteBinding {
            floor_id: "reference-floor-01".to_owned(),
            site_epoch: "site-epoch-1".to_owned(),
            design_revision_digest: "design-sha256-1".to_owned(),
            target_frame_id: "project-grid-1".to_owned(),
        }
    }

    fn reservation(
        reservation_id: &str,
        task_id: &str,
        start: u64,
        end: u64,
        clock_uncertainty_ms: u64,
    ) -> ReservationRequest {
        ReservationRequest {
            reservation_id: reservation_id.to_owned(),
            resource_id: "workface-resource".to_owned(),
            task_id: task_id.to_owned(),
            owner_id: format!("owner:{task_id}"),
            site_epoch: "site-epoch-1".to_owned(),
            clock_domain_id: "site-clock-1".to_owned(),
            clock_uncertainty_ms,
            interval_start_ms: start,
            interval_end_ms: end,
            expires_at_ms: end.saturating_add(clock_uncertainty_ms).saturating_add(100),
            units: 1,
        }
    }

    fn occupancy(
        id: &str,
        task: &str,
        kind: OccupancyKind,
        bounds: Bounds3Mm,
        start: u64,
        end: u64,
    ) -> SpatialOccupancy {
        SpatialOccupancy {
            occupancy_id: id.to_owned(),
            task_id: task.to_owned(),
            kind,
            binding: binding(),
            bounds,
            uncertainty_margin_mm: [0, 0, 0],
            clearance_margin_mm: [0, 0, 0],
            reservation: reservation(&format!("reservation-{id}"), task, start, end, 0),
        }
    }

    fn bounds(min: [i64; 3], max: [i64; 3]) -> Bounds3Mm {
        Bounds3Mm { min, max }
    }

    fn book() -> SpatialOccupancyBook {
        SpatialOccupancyBook::new(binding(), "site-clock-1").expect("valid book")
    }

    fn lattice_oracle_intersection(
        left: Bounds3Mm,
        right: Bounds3Mm,
    ) -> Option<Bounds3Mm> {
        let mut points = Vec::new();
        for x in 0_i64..=4 {
            for y in 0_i64..=4 {
                for z in 0_i64..=4 {
                    let point = [x, y, z];
                    let in_left = (0..3).all(|axis| {
                        left.min[axis] <= point[axis] && point[axis] <= left.max[axis]
                    });
                    let in_right = (0..3).all(|axis| {
                        right.min[axis] <= point[axis] && point[axis] <= right.max[axis]
                    });
                    if in_left && in_right {
                        points.push(point);
                    }
                }
            }
        }

        if points.is_empty() {
            return None;
        }

        let mut minimum = points[0];
        let mut maximum = points[0];
        for point in points.iter().skip(1) {
            for axis in 0..3 {
                minimum[axis] = minimum[axis].min(point[axis]);
                maximum[axis] = maximum[axis].max(point[axis]);
            }
        }
        Some(Bounds3Mm {
            min: minimum,
            max: maximum,
        })
    }

    #[test]
    fn closed_aabb_intersection_matches_exhaustive_lattice_oracle() {
        let mut boxes = Vec::new();
        for x in 0_i64..=2 {
            for y in 0_i64..=2 {
                for z in 0_i64..=2 {
                    for dx in 1_i64..=2 {
                        for dy in 1_i64..=2 {
                            for dz in 1_i64..=2 {
                                boxes.push(bounds(
                                    [x, y, z],
                                    [x + dx, y + dy, z + dz],
                                ));
                            }
                        }
                    }
                }
            }
        }

        for left in &boxes {
            for right in &boxes {
                assert_eq!(
                    left.intersection_closed(right),
                    lattice_oracle_intersection(*left, *right),
                    "left={left:?}, right={right:?}"
                );
            }
        }
    }

    #[test]
    fn same_volume_and_overlapping_time_are_a_potential_conflict() {
        let mut book = book();
        let first = occupancy(
            "workface-a",
            "task-a",
            OccupancyKind::Workface,
            bounds([0, 0, 0], [1_000, 1_000, 2_000]),
            0,
            100,
        );
        let second = occupancy(
            "robot-b",
            "task-b",
            OccupancyKind::RobotOperatingEnvelope,
            bounds([500, 500, 0], [1_500, 1_500, 2_000]),
            50,
            150,
        );

        assert_eq!(
            book.insert(first),
            Ok(InsertOutcome::AdmittedToSpatialIndex)
        );
        match book.insert(second) {
            Err(SpatialError::PotentialConflict(conflict)) => {
                assert_eq!(conflict.existing_occupancy_id, "workface-a");
                assert_eq!(conflict.candidate_occupancy_id, "robot-b");
                assert_eq!(conflict.overlap_start_ms, 50);
                assert_eq!(conflict.overlap_end_ms, 100);
                assert_eq!(
                    conflict.overlap_bounds_mm,
                    bounds([500, 500, 0], [1_000, 1_000, 2_000])
                );
            }
            other => panic!("expected potential conflict, got {other:?}"),
        }
        assert_eq!(book.occupancy_count(), 1);
    }

    #[test]
    fn adjacent_half_open_time_windows_can_reuse_the_same_space() {
        let mut book = book();
        let first = occupancy(
            "zone-a",
            "task-a",
            OccupancyKind::Workface,
            bounds([0, 0, 0], [1_000, 1_000, 2_000]),
            0,
            100,
        );
        let second = occupancy(
            "zone-b",
            "task-b",
            OccupancyKind::MaterialStaging,
            bounds([0, 0, 0], [1_000, 1_000, 2_000]),
            100,
            200,
        );

        book.insert(first).expect("first occupancy");
        assert_eq!(
            book.insert(second),
            Ok(InsertOutcome::AdmittedToSpatialIndex)
        );
    }

    #[test]
    fn disjoint_spatial_envelopes_can_overlap_in_time() {
        let mut book = book();
        let first = occupancy(
            "left",
            "task-left",
            OccupancyKind::TransitCorridor,
            bounds([0, 0, 0], [1_000, 1_000, 1_000]),
            0,
            100,
        );
        let second = occupancy(
            "right",
            "task-right",
            OccupancyKind::InspectionAccess,
            bounds([2_000, 0, 0], [3_000, 1_000, 1_000]),
            0,
            100,
        );

        book.insert(first).expect("first occupancy");
        assert_eq!(
            book.insert(second),
            Ok(InsertOutcome::AdmittedToSpatialIndex)
        );
    }

    #[test]
    fn vertical_separation_is_respected() {
        let mut book = book();
        let first = occupancy(
            "lower",
            "task-lower",
            OccupancyKind::Workface,
            bounds([0, 0, 0], [1_000, 1_000, 1_000]),
            0,
            100,
        );
        let second = occupancy(
            "upper",
            "task-upper",
            OccupancyKind::RobotOperatingEnvelope,
            bounds([0, 0, 1_100], [1_000, 1_000, 2_000]),
            0,
            100,
        );

        book.insert(first).expect("lower occupancy");
        assert_eq!(
            book.insert(second),
            Ok(InsertOutcome::AdmittedToSpatialIndex)
        );
    }

    #[test]
    fn boundary_touch_after_expansion_is_a_potential_conflict() {
        let mut book = book();
        let first = occupancy(
            "first",
            "task-first",
            OccupancyKind::Workface,
            bounds([0, 0, 0], [1_000, 1_000, 1_000]),
            0,
            100,
        );
        let second = occupancy(
            "second",
            "task-second",
            OccupancyKind::ToolAndPayloadEnvelope,
            bounds([1_000, 0, 0], [2_000, 1_000, 1_000]),
            0,
            100,
        );

        book.insert(first).expect("first occupancy");
        assert!(matches!(
            book.insert(second),
            Err(SpatialError::PotentialConflict(_))
        ));
    }

    #[test]
    fn geometry_uncertainty_can_create_a_potential_conflict() {
        let mut book = book();
        let first = occupancy(
            "base",
            "task-base",
            OccupancyKind::Workface,
            bounds([0, 0, 0], [1_000, 1_000, 1_000]),
            0,
            100,
        );
        let mut second = occupancy(
            "uncertain",
            "task-uncertain",
            OccupancyKind::TransitCorridor,
            bounds([1_100, 0, 0], [2_000, 1_000, 1_000]),
            0,
            100,
        );
        second.uncertainty_margin_mm = [100, 0, 0];

        book.insert(first).expect("base occupancy");
        assert!(matches!(
            book.insert(second),
            Err(SpatialError::PotentialConflict(_))
        ));
    }

    #[test]
    fn clearance_and_measurement_uncertainty_are_distinct_inputs() {
        let mut book = book();
        let first = occupancy(
            "base",
            "task-base",
            OccupancyKind::Workface,
            bounds([0, 0, 0], [1_000, 1_000, 1_000]),
            0,
            100,
        );
        let mut second = occupancy(
            "clearance",
            "task-clearance",
            OccupancyKind::InspectionAccess,
            bounds([1_150, 0, 0], [2_000, 1_000, 1_000]),
            0,
            100,
        );
        second.uncertainty_margin_mm = [50, 0, 0];
        second.clearance_margin_mm = [100, 0, 0];

        book.insert(first).expect("base occupancy");
        assert!(matches!(
            book.insert(second),
            Err(SpatialError::PotentialConflict(_))
        ));
    }

    #[test]
    fn time_uncertainty_expands_intervals_conservatively() {
        let mut book = book();
        let first = occupancy(
            "first",
            "task-first",
            OccupancyKind::Workface,
            bounds([0, 0, 0], [1_000, 1_000, 1_000]),
            0,
            100,
        );
        let mut second = occupancy(
            "second",
            "task-second",
            OccupancyKind::MaterialStaging,
            bounds([0, 0, 0], [1_000, 1_000, 1_000]),
            109,
            200,
        );
        second.reservation.clock_uncertainty_ms = 10;

        book.insert(first).expect("first occupancy");
        assert!(matches!(
            book.insert(second),
            Err(SpatialError::PotentialConflict(_))
        ));
    }

    #[test]
    fn stale_site_design_and_frame_bindings_are_rejected() {
        let mut book = book();

        let mut stale_epoch = occupancy(
            "stale-epoch",
            "task-a",
            OccupancyKind::Workface,
            bounds([0, 0, 0], [1_000, 1_000, 1_000]),
            0,
            100,
        );
        stale_epoch.binding.site_epoch = "site-epoch-old".to_owned();
        stale_epoch.reservation.site_epoch = "site-epoch-old".to_owned();
        assert_eq!(
            book.insert(stale_epoch),
            Err(SpatialError::StaleSiteEpoch)
        );

        let mut stale_design = occupancy(
            "stale-design",
            "task-b",
            OccupancyKind::Workface,
            bounds([0, 0, 0], [1_000, 1_000, 1_000]),
            0,
            100,
        );
        stale_design.binding.design_revision_digest = "old-design".to_owned();
        assert_eq!(
            book.insert(stale_design),
            Err(SpatialError::StaleDesignRevision)
        );

        let mut wrong_frame = occupancy(
            "wrong-frame",
            "task-c",
            OccupancyKind::Workface,
            bounds([0, 0, 0], [1_000, 1_000, 1_000]),
            0,
            100,
        );
        wrong_frame.binding.target_frame_id = "local-frame".to_owned();
        assert_eq!(book.insert(wrong_frame), Err(SpatialError::FrameMismatch));
    }

    #[test]
    fn wrong_floor_clock_domain_and_task_binding_are_rejected() {
        let mut book = book();

        let mut wrong_floor = occupancy(
            "wrong-floor",
            "task-a",
            OccupancyKind::Workface,
            bounds([0, 0, 0], [1_000, 1_000, 1_000]),
            0,
            100,
        );
        wrong_floor.binding.floor_id = "other-floor".to_owned();
        assert_eq!(book.insert(wrong_floor), Err(SpatialError::StaleFloor));

        let mut wrong_clock = occupancy(
            "wrong-clock",
            "task-b",
            OccupancyKind::Workface,
            bounds([0, 0, 0], [1_000, 1_000, 1_000]),
            0,
            100,
        );
        wrong_clock.reservation.clock_domain_id = "another-clock".to_owned();
        assert_eq!(
            book.insert(wrong_clock),
            Err(SpatialError::ClockDomainMismatch)
        );

        let mut wrong_task = occupancy(
            "wrong-task",
            "task-c",
            OccupancyKind::Workface,
            bounds([0, 0, 0], [1_000, 1_000, 1_000]),
            0,
            100,
        );
        wrong_task.reservation.task_id = "different-task".to_owned();
        assert_eq!(
            book.insert(wrong_task),
            Err(SpatialError::ReservationTaskMismatch)
        );
    }

    #[test]
    fn malformed_bounds_and_expansion_overflow_are_rejected() {
        let mut book = book();
        let mut reversed = occupancy(
            "reversed",
            "task-a",
            OccupancyKind::Workface,
            bounds([100, 0, 0], [100, 1_000, 1_000]),
            0,
            100,
        );
        assert_eq!(
            book.insert(reversed.clone()),
            Err(SpatialError::InvalidBounds { axis: 0 })
        );

        reversed.bounds = bounds([0, 0, 0], [1_000, 1_000, 1_000]);
        reversed.clearance_margin_mm = [u64::MAX, 0, 0];
        assert_eq!(
            book.insert(reversed),
            Err(SpatialError::ExpansionOverflow { axis: 0 })
        );
    }

    #[test]
    fn reservation_time_uncertainty_underflow_is_rejected() {
        let mut book = book();
        let mut item = occupancy(
            "underflow",
            "task-a",
            OccupancyKind::Workface,
            bounds([0, 0, 0], [1_000, 1_000, 1_000]),
            0,
            100,
        );
        item.reservation.clock_uncertainty_ms = 1;
        assert_eq!(
            book.insert(item),
            Err(SpatialError::TimeExpansionOverflow)
        );
    }

    #[test]
    fn identical_occupancy_is_idempotent_but_identity_reuse_is_not() {
        let mut book = book();
        let first = occupancy(
            "stable-id",
            "task-a",
            OccupancyKind::Workface,
            bounds([0, 0, 0], [1_000, 1_000, 1_000]),
            0,
            100,
        );
        assert_eq!(
            book.insert(first.clone()),
            Ok(InsertOutcome::AdmittedToSpatialIndex)
        );
        assert_eq!(book.insert(first.clone()), Ok(InsertOutcome::Duplicate));

        let mut changed = first;
        changed.bounds = bounds([2_000, 0, 0], [3_000, 1_000, 1_000]);
        assert_eq!(
            book.insert(changed),
            Err(SpatialError::OccupancyIdConflict("stable-id".to_owned()))
        );
    }
}
