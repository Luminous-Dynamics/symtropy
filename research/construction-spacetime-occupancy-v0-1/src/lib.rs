//! Conservative spatial-temporal occupancy overlap oracle.
//!
//! This module composes exact site/design/frame identity with the time interval
//! and clock uncertainty declared by the existing construction interop contract.
//! It detects potential overlap between conservative axis-aligned outer bounds.
//! It is not a collision-avoidance system, safety controller, clearance authority,
//! trajectory planner, BIM parser, coordinate transform engine, or motion authority.

use std::collections::{BTreeMap, BTreeSet};

/// Broad-phase cell edge. This is an acceleration parameter, not a clearance rule.
const SPATIAL_CELL_SIZE_MM: i64 = 1_000;
/// Oversized envelopes use a conservative global candidate bucket.
const MAX_GRID_CELLS_PER_OCCUPANCY: u128 = 4_096;

type CellKey = (i64, i64, i64);

use construction_robot_interop_core::resources::{
    ReservationBook, ReservationOutcome, ReservationRequest,
};

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
        for (axis, (minimum, maximum)) in self.min.iter().zip(self.max.iter()).enumerate() {
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CombinedAdmission {
    pub reservation_outcome: ReservationOutcome,
    pub occupancy_outcome: InsertOutcome,
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
    IndexInconsistent(String),
    PotentialConflict(Box<ConflictRecord>),
    Reservation(construction_robot_interop_core::resources::ReservationError),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpatialOccupancyBook {
    binding: SiteBinding,
    clock_domain_id: String,
    occupancies: BTreeMap<String, SpatialOccupancy>,
    expanded_bounds: BTreeMap<String, Bounds3Mm>,
    expanded_intervals: BTreeMap<String, (u64, u64)>,
    spatial_cells: BTreeMap<CellKey, BTreeSet<String>>,
    global_occupancies: BTreeSet<String>,
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
            expanded_bounds: BTreeMap::new(),
            expanded_intervals: BTreeMap::new(),
            spatial_cells: BTreeMap::new(),
            global_occupancies: BTreeSet::new(),
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
    /// Atomically admit the time/resource reservation and its spatial occupancy
    /// against cloned in-memory owners. No partial mutation is committed when either
    /// owner rejects the candidate. This is a single-process composition primitive,
    /// not a distributed transaction, durable commit or safety authorization.
    pub fn reserve_and_insert(
        &mut self,
        reservation_book: &mut ReservationBook,
        occupancy: SpatialOccupancy,
        now_ms: u64,
    ) -> Result<CombinedAdmission, SpatialError> {
        let mut candidate_reservations = reservation_book.clone();
        let mut candidate_occupancies = self.clone();

        let reservation_outcome = candidate_reservations
            .reserve(occupancy.reservation.clone(), now_ms)
            .map_err(SpatialError::Reservation)?;
        let occupancy_outcome = candidate_occupancies.insert(occupancy)?;

        *reservation_book = candidate_reservations;
        *self = candidate_occupancies;

        Ok(CombinedAdmission {
            reservation_outcome,
            occupancy_outcome,
        })
    }

    fn validate_occupancy(&self, occupancy: &SpatialOccupancy) -> Result<Bounds3Mm, SpatialError> {
        require_non_empty("occupancy_id", &occupancy.occupancy_id)?;
        require_non_empty("task_id", &occupancy.task_id)?;
        require_non_empty("reservation_id", &occupancy.reservation.reservation_id)?;
        require_non_empty(
            "reservation_resource_id",
            &occupancy.reservation.resource_id,
        )?;
        require_non_empty("reservation_owner_id", &occupancy.reservation.owner_id)?;
        require_non_empty("reservation_task_id", &occupancy.reservation.task_id)?;
        require_non_empty("reservation_site_epoch", &occupancy.reservation.site_epoch)?;
        require_non_empty(
            "reservation_clock_domain_id",
            &occupancy.reservation.clock_domain_id,
        )?;

        occupancy.bounds.validate()?;
        let expanded = occupancy.bounds.expanded(
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

        Ok(expanded)
    }

    fn spatial_candidate_ids(&self, bounds: &Bounds3Mm) -> BTreeSet<String> {
        let Some(cells) = grid_cells(bounds) else {
            return self.occupancies.keys().cloned().collect();
        };

        let mut candidate_ids = self.global_occupancies.clone();
        for cell in cells {
            if let Some(ids) = self.spatial_cells.get(&cell) {
                candidate_ids.extend(ids.iter().cloned());
            }
        }
        candidate_ids
    }

    fn index_occupancy(&mut self, occupancy_id: &str, expanded_bounds: &Bounds3Mm) {
        match grid_cells(expanded_bounds) {
            Some(cells) => {
                for cell in cells {
                    self.spatial_cells
                        .entry(cell)
                        .or_default()
                        .insert(occupancy_id.to_owned());
                }
            }
            None => {
                self.global_occupancies.insert(occupancy_id.to_owned());
            }
        }
    }

    fn find_conflicts_validated(
        &self,
        occupancy: &SpatialOccupancy,
        expanded_candidate: Bounds3Mm,
        (candidate_start, candidate_end): (u64, u64),
    ) -> Result<Vec<ConflictRecord>, SpatialError> {
        let mut conflicts = Vec::new();

        for existing_id in self.spatial_candidate_ids(&expanded_candidate) {
            let existing = self
                .occupancies
                .get(&existing_id)
                .ok_or_else(|| SpatialError::IndexInconsistent(existing_id.clone()))?;
            let (existing_start, existing_end) = self
                .expanded_intervals
                .get(&existing_id)
                .copied()
                .ok_or_else(|| SpatialError::IndexInconsistent(existing_id.clone()))?;
            let overlap_start = candidate_start.max(existing_start);
            let overlap_end = candidate_end.min(existing_end);
            if overlap_start >= overlap_end {
                continue;
            }

            let expanded_existing = self
                .expanded_bounds
                .get(&existing_id)
                .copied()
                .ok_or_else(|| SpatialError::IndexInconsistent(existing_id.clone()))?;
            if let Some(overlap_bounds_mm) =
                expanded_candidate.intersection_closed(&expanded_existing)
            {
                conflicts.push(ConflictRecord {
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
                });
            }
        }

        Ok(conflicts)
    }

    /// Return every conservative space-time conflict against the current index
    /// without mutating it. This is the planning query; callers can compare a full
    /// conflict set before choosing a different sequence or resource assignment.
    pub fn conflicts_for(
        &self,
        occupancy: &SpatialOccupancy,
    ) -> Result<Vec<ConflictRecord>, SpatialError> {
        let expanded_candidate = self.validate_occupancy(occupancy)?;
        if let Some(previous) = self.occupancies.get(&occupancy.occupancy_id) {
            return if previous == occupancy {
                Ok(Vec::new())
            } else {
                Err(SpatialError::OccupancyIdConflict(
                    occupancy.occupancy_id.clone(),
                ))
            };
        }

        let candidate_interval = expanded_time_interval(&occupancy.reservation)?;
        self.find_conflicts_validated(occupancy, expanded_candidate, candidate_interval)
    }

    /// Number of broad-phase candidates considered by the spatial index. This is
    /// a diagnostic for benchmark/reporting only; exact conflict results come from
    /// `conflicts_for`, which applies interval and AABB overlap tests.
    pub fn candidate_count_for(&self, occupancy: &SpatialOccupancy) -> Result<usize, SpatialError> {
        let expanded_candidate = self.validate_occupancy(occupancy)?;
        if let Some(previous) = self.occupancies.get(&occupancy.occupancy_id) {
            return if previous == occupancy {
                Ok(0)
            } else {
                Err(SpatialError::OccupancyIdConflict(
                    occupancy.occupancy_id.clone(),
                ))
            };
        }
        Ok(self.spatial_candidate_ids(&expanded_candidate).len())
    }

    /// Insert a planned occupancy after its corresponding time/resource request has
    /// been admitted by the existing reservation owner. This method checks the exact
    /// declared reservation fields for task, site epoch, clock domain and interval.
    /// It cannot prove that a caller actually obtained a grant from ReservationBook.
    pub fn insert(&mut self, occupancy: SpatialOccupancy) -> Result<InsertOutcome, SpatialError> {
        let expanded_candidate = self.validate_occupancy(&occupancy)?;

        if let Some(previous) = self.occupancies.get(&occupancy.occupancy_id) {
            return if previous == &occupancy {
                Ok(InsertOutcome::Duplicate)
            } else {
                Err(SpatialError::OccupancyIdConflict(occupancy.occupancy_id))
            };
        }

        let candidate_interval = expanded_time_interval(&occupancy.reservation)?;
        if let Some(conflict) = self
            .find_conflicts_validated(&occupancy, expanded_candidate, candidate_interval)?
            .into_iter()
            .next()
        {
            return Err(SpatialError::PotentialConflict(Box::new(conflict)));
        }

        let occupancy_id = occupancy.occupancy_id.clone();
        self.expanded_bounds
            .insert(occupancy_id.clone(), expanded_candidate);
        self.expanded_intervals
            .insert(occupancy_id.clone(), candidate_interval);
        self.index_occupancy(&occupancy_id, &expanded_candidate);
        self.occupancies.insert(occupancy_id, occupancy);
        Ok(InsertOutcome::AdmittedToSpatialIndex)
    }
}

/// Return the conservative broad-phase cells for an already expanded AABB.
/// An envelope spanning too many cells returns None and is tracked in a global
/// fallback bucket. The exact closed AABB test remains authoritative.
fn grid_cells(bounds: &Bounds3Mm) -> Option<Vec<CellKey>> {
    let minimum = bounds
        .min
        .map(|coordinate| coordinate.div_euclid(SPATIAL_CELL_SIZE_MM));
    let maximum = bounds
        .max
        .map(|coordinate| coordinate.div_euclid(SPATIAL_CELL_SIZE_MM));

    let counts: [u128; 3] = std::array::from_fn(|axis| {
        (i128::from(maximum[axis]) - i128::from(minimum[axis]) + 1) as u128
    });
    let cell_count = counts
        .into_iter()
        .try_fold(1_u128, |product, count| product.checked_mul(count))?;
    if cell_count > MAX_GRID_CELLS_PER_OCCUPANCY {
        return None;
    }

    let mut cells = Vec::with_capacity(usize::try_from(cell_count).ok()?);
    for x in minimum[0]..=maximum[0] {
        for y in minimum[1]..=maximum[1] {
            for z in minimum[2]..=maximum[2] {
                cells.push((x, y, z));
            }
        }
    }
    Some(cells)
}

/// Expand the existing reservation interval conservatively using its clock bound.
/// Half-open intervals remain half-open; no independent calendar or clock policy
/// is created here. Underflow/overflow is a typed failure, not a wrap or clamp.
fn expanded_time_interval(reservation: &ReservationRequest) -> Result<(u64, u64), SpatialError> {
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
    use construction_floor_production_core::{
        CostCategory, CostEntry, FloorBinding, FloorLedger, LabourSlice, WorkUnitSpec,
    };
    use construction_robot_interop_core::{
        AcceptanceReceipt, ContextBinding, EventEnvelope, TaskMachine, TransitionKind,
    };

    const BENCHMARK_PROFILE_BLOB_ID: &str =
        "git-blob-sha1:25862337341f4c5ceb216e24bc49f6926082c1c6";
    const REFERENCE_FIXTURE_BLOB_ID: &str =
        "git-blob-sha1:9db3b46b8eb81d302f44b9bac9a515d4206dae23";

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

    fn lattice_oracle_intersection(left: Bounds3Mm, right: Bounds3Mm) -> Option<Bounds3Mm> {
        let mut points = Vec::new();
        for x in 0_i64..=4 {
            for y in 0_i64..=4 {
                for z in 0_i64..=4 {
                    let point = [x, y, z];
                    let in_left = (0..3)
                        .all(|axis| left.min[axis] <= point[axis] && point[axis] <= left.max[axis]);
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
                                boxes.push(bounds([x, y, z], [x + dx, y + dy, z + dz]));
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
    fn conflict_query_returns_all_overlaps_without_mutating_the_index() {
        let mut book = book();
        let first = occupancy(
            "zone-left",
            "task-left",
            OccupancyKind::Workface,
            bounds([0, 0, 0], [1_000, 1_000, 1_000]),
            0,
            100,
        );
        let second = occupancy(
            "zone-right",
            "task-right",
            OccupancyKind::MaterialStaging,
            bounds([2_000, 0, 0], [3_000, 1_000, 1_000]),
            0,
            100,
        );
        let candidate = occupancy(
            "wide-workface",
            "task-candidate",
            OccupancyKind::RobotOperatingEnvelope,
            bounds([500, 0, 0], [2_500, 1_000, 1_000]),
            10,
            90,
        );

        book.insert(first).expect("left occupancy");
        book.insert(second).expect("right occupancy");
        let conflicts = book
            .conflicts_for(&candidate)
            .expect("query candidate conflicts");

        assert_eq!(conflicts.len(), 2);
        assert_eq!(
            conflicts
                .iter()
                .map(|conflict| conflict.existing_occupancy_id.as_str())
                .collect::<Vec<_>>(),
            vec!["zone-left", "zone-right"]
        );
        assert_eq!(book.occupancy_count(), 2);
    }

    #[test]
    fn spatial_grid_preserves_closed_boundary_overlap_across_negative_and_zero_cells() {
        let mut book = book();
        let first = occupancy(
            "negative-side",
            "task-negative",
            OccupancyKind::Workface,
            bounds([-10_000, 0, 0], [0, 1_000, 1_000]),
            0,
            100,
        );
        let touching = occupancy(
            "zero-side",
            "task-zero",
            OccupancyKind::InspectionAccess,
            bounds([0, 0, 0], [1_000, 1_000, 1_000]),
            0,
            100,
        );
        book.insert(first).expect("negative-side occupancy");

        assert_eq!(book.candidate_count_for(&touching), Ok(1));
        assert_eq!(
            book.conflicts_for(&touching).expect("conflict query").len(),
            1
        );
    }

    #[test]
    fn oversized_envelope_uses_global_fallback_without_false_negatives() {
        let mut book = book();
        let oversized = occupancy(
            "large-platform-envelope",
            "task-platform",
            OccupancyKind::CraneOrPlantSweptOuterBound,
            bounds([0, 0, 0], [1_000_000, 1_000_000, 1_000_000]),
            0,
            100,
        );
        let small = occupancy(
            "robot-cell",
            "task-robot",
            OccupancyKind::RobotOperatingEnvelope,
            bounds([500_000, 500_000, 500_000], [501_000, 501_000, 501_000]),
            10,
            90,
        );
        book.insert(oversized).expect("global fallback occupancy");

        assert_eq!(book.candidate_count_for(&small), Ok(1));
        assert_eq!(
            book.conflicts_for(&small)
                .expect("global fallback query")
                .len(),
            1
        );
    }

    #[test]
    fn grid_broad_phase_has_no_false_negatives_on_frozen_boundary_pair_corpus() {
        let coordinates = [-10_001_i64, -10_000, -1, 0, 1, 9_999, 10_000, 10_001];
        let mut boxes = Vec::new();

        for (index, x) in coordinates.iter().enumerate() {
            for y in coordinates {
                for z in coordinates {
                    let width = if (index + (y as usize & 1)) % 2 == 0 {
                        1_000
                    } else {
                        10_001
                    };
                    boxes.push(bounds(
                        [*x, y, z],
                        [*x + width, y + width, z + width],
                    ));
                }
            }
        }

        for left in &boxes {
            let left_cells = grid_cells(left);
            for right in &boxes {
                if left.intersection_closed(right).is_none() {
                    continue;
                }

                let right_cells = grid_cells(right);
                match (&left_cells, &right_cells) {
                    (Some(left_cells), Some(right_cells)) => {
                        let right_set: BTreeSet<_> = right_cells.iter().copied().collect();
                        assert!(
                            left_cells.iter().any(|cell| right_set.contains(cell)),
                            "grid false negative for left={left:?}, right={right:?}"
                        );
                    }
                    // An oversized box uses the global fallback and is compared
                    // against every candidate; it cannot be pruned by the grid.
                    _ => {}
                }
            }
        }
    }

    #[test]
    fn extreme_coordinate_span_uses_global_fallback() {
        let giant = bounds([i64::MIN, i64::MIN, i64::MIN], [i64::MAX, i64::MAX, i64::MAX]);
        assert!(grid_cells(&giant).is_none());
    }

    #[test]
    fn sparse_grid_filters_unrelated_candidate_pairs() {
        let mut book = book();
        for index in 0..128_i64 {
            let x = index * 20_000;
            let item = occupancy(
                &format!("sparse-{index}"),
                &format!("task-{index}"),
                OccupancyKind::Workface,
                bounds([x, 0, 0], [x + 1_000, 1_000, 1_000]),
                0,
                100,
            );
            book.insert(item).expect("spatially separated insertion");
        }

        let candidate = occupancy(
            "empty-gap",
            "task-gap",
            OccupancyKind::MaterialStaging,
            bounds([10_000, 0, 0], [11_000, 1_000, 1_000]),
            0,
            100,
        );
        assert_eq!(book.candidate_count_for(&candidate), Ok(0));
        assert!(
            book.conflicts_for(&candidate)
                .expect("empty-gap query")
                .is_empty()
        );
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
        assert_eq!(book.insert(stale_epoch), Err(SpatialError::StaleSiteEpoch));

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
        assert_eq!(book.insert(item), Err(SpatialError::TimeExpansionOverflow));
    }

    #[test]
    fn combined_admission_rolls_back_resource_reservation_on_spatial_conflict() {
        let mut spatial = book();
        let mut reservations =
            ReservationBook::new("site-epoch-1", "site-clock-1", 0).expect("reservation book");
        reservations
            .register_resource("workface-resource", 1)
            .expect("workface resource");
        reservations
            .register_resource("independent-robot-resource", 1)
            .expect("independent robot resource");

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
            OccupancyKind::RobotOperatingEnvelope,
            bounds([500, 500, 0], [1_500, 1_500, 1_000]),
            50,
            150,
        );
        second.reservation.resource_id = "independent-robot-resource".to_owned();

        let admitted = spatial
            .reserve_and_insert(&mut reservations, first, 0)
            .expect("first admission");
        assert_eq!(
            admitted,
            CombinedAdmission {
                reservation_outcome: ReservationOutcome::Granted,
                occupancy_outcome: InsertOutcome::AdmittedToSpatialIndex,
            }
        );

        assert!(matches!(
            spatial.reserve_and_insert(&mut reservations, second, 0),
            Err(SpatialError::PotentialConflict(_))
        ));
        assert_eq!(spatial.occupancy_count(), 1);
        assert_eq!(reservations.state("reservation-second"), None);
        assert_eq!(
            reservations.state("reservation-first"),
            Some(construction_robot_interop_core::resources::ReservationState::Active)
        );
    }

    #[test]
    fn combined_admission_rolls_back_occupancy_when_resource_capacity_rejects() {
        let mut spatial = book();
        let mut reservations =
            ReservationBook::new("site-epoch-1", "site-clock-1", 0).expect("reservation book");
        reservations
            .register_resource("workface-resource", 1)
            .expect("resource");

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
            OccupancyKind::TransitCorridor,
            bounds([2_000, 0, 0], [3_000, 1_000, 1_000]),
            50,
            150,
        );

        spatial
            .reserve_and_insert(&mut reservations, first, 0)
            .expect("first admission");
        assert!(matches!(
            spatial.reserve_and_insert(&mut reservations, second, 0),
            Err(SpatialError::Reservation(
                construction_robot_interop_core::resources::ReservationError::CapacityExceeded { .. }
            ))
        ));
        assert_eq!(spatial.occupancy_count(), 1);
        assert_eq!(reservations.state("reservation-second"), None);
    }

    #[test]
    fn accepted_floor_ledger_and_space_time_admission_integrate_end_to_end() {
        let context = ContextBinding {
            site_epoch: "site-epoch-1".to_owned(),
            design_revision_digest: "design-sha256-1".to_owned(),
            target_frame_id: "project-grid-1".to_owned(),
            robot_config_digest: "robot-config-sha256-1".to_owned(),
        };
        let floor_binding = FloorBinding {
            floor_id: "reference-floor-01".to_owned(),
            site_epoch: context.site_epoch.clone(),
            design_revision_digest: context.design_revision_digest.clone(),
            target_frame_id: context.target_frame_id.clone(),
            benchmark_profile_id: "HIGHRISE-REFERENCE-FLOOR-ADAPTER-001".to_owned(),
            benchmark_profile_digest: BENCHMARK_PROFILE_BLOB_ID.to_owned(),
            reference_fixture_id: "highrise-reference-floor-micro-oracle-001".to_owned(),
            reference_fixture_digest: REFERENCE_FIXTURE_BLOB_ID.to_owned(),
            claim_ceiling: "synthetic_integrated_production_experiment_only".to_owned(),
            labour_time_axis_id: "site-labour-clock-1".to_owned(),
        };
        let mut ledger = FloorLedger::new(
            floor_binding,
            "USD",
            vec![WorkUnitSpec {
                work_unit_id: "rebar-functional-unit-001".to_owned(),
                acceptance_contract_ref: "acceptance-contract:rebar-install".to_owned(),
            }],
        )
        .expect("floor ledger");

        let mut machine = TaskMachine::new("task-1", context.clone()).expect("valid task machine");
        let actor = "robot-1";
        let session = "session-1";
        let mut send = |sequence: u64, transition: TransitionKind| {
            machine.apply(EventEnvelope {
                message_id: format!("{actor}-message-{sequence}"),
                payload_digest: format!("{actor}-payload-{sequence}"),
                actor_id: actor.to_owned(),
                actor_session_id: session.to_owned(),
                task_sequence: sequence,
                task_id: "task-1".to_owned(),
                binding: context.clone(),
                transition,
            })
        };
        send(1, TransitionKind::Accept).expect("task accepted for execution");
        send(2, TransitionKind::Start).expect("task started");
        send(
            3,
            TransitionKind::ExecutorComplete {
                output_evidence_ref: Some("output-evidence:rebar-001".to_owned()),
            },
        )
        .expect("executor completed");
        send(4, TransitionKind::SubmitForMeasurement).expect("submitted for inspection");
        send(
            5,
            TransitionKind::InspectionAccepted {
                receipt: AcceptanceReceipt {
                    receipt_ref: "inspection-receipt:rebar-001".to_owned(),
                    task_id: "task-1".to_owned(),
                    binding: context,
                    output_evidence_ref: "output-evidence:rebar-001".to_owned(),
                },
            },
        )
        .expect("independent inspection acceptance");

        assert_eq!(
            ledger.observe_attempt("rebar-functional-unit-001", &machine, None),
            Ok(construction_floor_production_core::ObserveOutcome::Recorded)
        );
        ledger
            .add_cost(CostEntry {
                entry_id: "robot-task-cost-001".to_owned(),
                task_id: Some("task-1".to_owned()),
                category: CostCategory::RobotOperation,
                amount_minor_units: 1_000,
                currency_code: "USD".to_owned(),
            })
            .expect("task-attributed cost");
        ledger
            .add_cost(CostEntry {
                entry_id: "shared-survey-cost-001".to_owned(),
                task_id: None,
                category: CostCategory::SurveyAndControl,
                amount_minor_units: 300,
                currency_code: "USD".to_owned(),
            })
            .expect("unallocated floor-wide cost");
        ledger
            .add_labour_slice(LabourSlice {
                slice_id: "operator-slice-001".to_owned(),
                person_id: "operator-1".to_owned(),
                role: "robot-operator".to_owned(),
                task_id: Some("task-1".to_owned()),
                start_minute: 0,
                end_minute: 30,
            })
            .expect("task-attributed operator time");
        ledger
            .add_labour_slice(LabourSlice {
                slice_id: "supervisor-slice-001".to_owned(),
                person_id: "supervisor-1".to_owned(),
                role: "site-supervisor".to_owned(),
                task_id: None,
                start_minute: 0,
                end_minute: 15,
            })
            .expect("shared supervisor time");

        let mut reservations = ReservationBook::new("site-epoch-1", "site-clock-1", 0)
            .expect("existing interop resource book");
        reservations
            .register_resource("HOIST-1", 1)
            .expect("hoist resource");
        reservations
            .register_resource("ROBOT-1", 1)
            .expect("robot resource");
        let mut spatial = book();

        let mut accepted_workface = occupancy(
            "accepted-rebar-workface",
            "task-1",
            OccupancyKind::Workface,
            bounds([0, 0, 0], [1_000, 1_000, 2_000]),
            0,
            100,
        );
        accepted_workface.reservation.resource_id = "HOIST-1".to_owned();
        let first = spatial
            .reserve_and_insert(&mut reservations, accepted_workface, 0)
            .expect("reserve and admit accepted workface");
        assert_eq!(first.reservation_outcome, ReservationOutcome::Granted);
        assert_eq!(
            first.occupancy_outcome,
            InsertOutcome::AdmittedToSpatialIndex
        );

        let mut overlapping_candidate = occupancy(
            "overlapping-robot-envelope",
            "task-2",
            OccupancyKind::RobotOperatingEnvelope,
            bounds([500, 500, 0], [1_500, 1_500, 2_000]),
            50,
            150,
        );
        overlapping_candidate.reservation.resource_id = "ROBOT-1".to_owned();
        assert!(matches!(
            spatial.reserve_and_insert(&mut reservations, overlapping_candidate, 0),
            Err(SpatialError::PotentialConflict(_))
        ));
        assert_eq!(spatial.occupancy_count(), 1);
        assert_eq!(
            reservations.state("reservation-overlapping-robot-envelope"),
            None
        );
        assert_eq!(
            reservations.state("reservation-accepted-rebar-workface"),
            Some(construction_robot_interop_core::resources::ReservationState::Active)
        );

        let report = ledger.report().expect("whole-floor production report");
        assert!(report.floor_accepted());
        assert_eq!(report.required_units, 1);
        assert_eq!(report.finally_accepted_units, 1);
        assert_eq!(report.total_cost_minor_units, 1_300);
        assert_eq!(report.costs_by_task_minor_units.get("task-1"), Some(&1_000));
        assert_eq!(report.unallocated_floor_cost_minor_units, 300);
        assert_eq!(report.accepted_floor_cost_minor_units, Some(1_300));
        assert_eq!(report.total_workflow_crew_minutes, 45);
        assert_eq!(report.crew_minutes_by_role.get("robot-operator"), Some(&30));
        assert_eq!(
            report.crew_minutes_by_role.get("site-supervisor"),
            Some(&15)
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
