//! Oracle-first verifier for a deliberately bounded shipyard space-time model.
//!
//! V0 supports axis-aligned rectangular footprints on an integer coordinate lattice and
//! half-open integer-tick time intervals. It is a *feasibility verifier*, not a scheduler,
//! route planner, crane simulator, lift plan, or physical safety case.
//!
//! Unsupported geometry is represented explicitly and makes a scenario non-feasible;
//! it is never silently interpreted as free space. Polygonal footprints, turning envelopes,
//! vertical clearance, stacking/support, floor bearing, crane/rigging physics and dynamic
//! loads are outside V0 and require separately qualified models.

use std::collections::{BTreeMap, HashSet};

/// A half-open interval [start_tick, end_tick), with discrete integer ticks.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct TimeRange {
    pub start_tick: u64,
    pub end_tick: u64,
}

impl TimeRange {
    pub const fn is_valid(self) -> bool {
        self.start_tick < self.end_tick
    }

    /// Half-open intervals that only touch at an endpoint do not overlap.
    pub const fn overlaps(self, other: Self) -> bool {
        self.start_tick < other.end_tick && other.start_tick < self.end_tick
    }
}

/// Axis-aligned rectangle [min_x, max_x) × [min_y, max_y) on an integer lattice.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Rect {
    pub min_x: i64,
    pub min_y: i64,
    pub max_x: i64,
    pub max_y: i64,
}

impl Rect {
    pub const fn is_valid(self) -> bool {
        self.min_x < self.max_x && self.min_y < self.max_y
    }

    /// Strict-area intersection. Shared edges alone are legal and do not count as overlap.
    pub const fn overlaps(self, other: Self) -> bool {
        self.min_x < other.max_x
            && other.min_x < self.max_x
            && self.min_y < other.max_y
            && other.min_y < self.max_y
    }

    pub const fn contains(self, other: Self) -> bool {
        self.min_x <= other.min_x
            && self.min_y <= other.min_y
            && other.max_x <= self.max_x
            && other.max_y <= self.max_y
    }
}

/// Geometry is either represented in the supported V0 model or explicitly unsupported.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Footprint {
    AxisAlignedRect(Rect),
    Unsupported { profile: String },
}

impl Footprint {
    fn is_supported_and_valid(&self) -> bool {
        matches!(self, Self::AxisAlignedRect(rect) if rect.is_valid())
    }

    fn overlaps_supported(&self, other: &Self) -> Option<bool> {
        match (self, other) {
            (Self::AxisAlignedRect(left), Self::AxisAlignedRect(right))
                if left.is_valid() && right.is_valid() =>
            {
                Some(left.overlaps(*right))
            }
            _ => None,
        }
    }
}

/// A block's reserved staging/work-cell footprint for a specified time window.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Placement {
    /// Stable identity for this placement reservation, not merely the block ID.
    pub reservation_id: String,
    pub block_id: String,
    pub footprint: Footprint,
    pub occupied: TimeRange,
}

/// A declared transfer/access corridor occupied while a block move is underway.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Transfer {
    pub id: String,
    pub block_id: String,
    /// V0 models a declared axis-aligned corridor envelope, not an automatically planned path.
    pub corridor: Footprint,
    pub interval: TimeRange,
    pub vehicle_id: String,
    pub predecessor_move_ids: Vec<String>,
}

/// A spatial region unavailable to block placement or transfer during its active interval.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExclusionZone {
    pub id: String,
    pub footprint: Footprint,
    pub active: TimeRange,
}

/// A complete finite scenario. This structure contains no planner-supplied feasibility flag.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Scenario {
    /// Finite yard boundary in the same integer coordinate lattice as every footprint.
    /// Missing or invalid bounds make the scenario unqualified.
    pub yard_bounds: Option<Rect>,
    pub placements: Vec<Placement>,
    pub transfers: Vec<Transfer>,
    pub exclusion_zones: Vec<ExclusionZone>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ViolationCode {
    EmptyPlacementId,
    EmptyBlockId,
    DuplicatePlacementId,
    EmptyTransferId,
    DuplicateTransferId,
    EmptyVehicleId,
    EmptyExclusionId,
    DuplicateExclusionId,
    EmptyPredecessorId,
    DuplicatePredecessor,
    SelfPredecessor,
    MissingPredecessor,
    InvalidFootprint,
    UnsupportedFootprint,
    MissingYardBounds,
    InvalidYardBounds,
    OutOfBoundsPlacement,
    OutOfBoundsTransferCorridor,
    OutOfBoundsExclusionZone,
    NoOperationalReservations,
    InvalidTimeRange,
    PrecedenceViolation,
    BlockDoubleBooked,
    BlockFootprintOverlap,
    BlockInsideExclusion,
    MoveBlockedByBlock,
    MoveIntersectsExclusion,
    VehicleOverlap,
}

impl ViolationCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::EmptyPlacementId => "EmptyPlacementId",
            Self::EmptyBlockId => "EmptyBlockId",
            Self::DuplicatePlacementId => "DuplicatePlacementId",
            Self::EmptyTransferId => "EmptyTransferId",
            Self::DuplicateTransferId => "DuplicateTransferId",
            Self::EmptyVehicleId => "EmptyVehicleId",
            Self::EmptyExclusionId => "EmptyExclusionId",
            Self::DuplicateExclusionId => "DuplicateExclusionId",
            Self::EmptyPredecessorId => "EmptyPredecessorId",
            Self::DuplicatePredecessor => "DuplicatePredecessor",
            Self::SelfPredecessor => "SelfPredecessor",
            Self::MissingPredecessor => "MissingPredecessor",
            Self::InvalidFootprint => "InvalidFootprint",
            Self::UnsupportedFootprint => "UnsupportedFootprint",
            Self::MissingYardBounds => "MissingYardBounds",
            Self::InvalidYardBounds => "InvalidYardBounds",
            Self::OutOfBoundsPlacement => "OutOfBoundsPlacement",
            Self::OutOfBoundsTransferCorridor => "OutOfBoundsTransferCorridor",
            Self::OutOfBoundsExclusionZone => "OutOfBoundsExclusionZone",
            Self::NoOperationalReservations => "NoOperationalReservations",
            Self::InvalidTimeRange => "InvalidTimeRange",
            Self::PrecedenceViolation => "PrecedenceViolation",
            Self::BlockDoubleBooked => "BlockDoubleBooked",
            Self::BlockFootprintOverlap => "BlockFootprintOverlap",
            Self::BlockInsideExclusion => "BlockInsideExclusion",
            Self::MoveBlockedByBlock => "MoveBlockedByBlock",
            Self::MoveIntersectsExclusion => "MoveIntersectsExclusion",
            Self::VehicleOverlap => "VehicleOverlap",
        }
    }
}

/// A machine-readable invariant violation. Subject IDs are sorted for stable output.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Violation {
    pub code: ViolationCode,
    pub subject_ids: Vec<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct VerificationReport {
    pub violations: Vec<Violation>,
}

impl VerificationReport {
    pub fn is_feasible(&self) -> bool {
        self.violations.is_empty()
    }

    pub fn count(&self, code: ViolationCode) -> usize {
        self.violations
            .iter()
            .filter(|violation| violation.code == code)
            .count()
    }
}

fn add_violation(
    violations: &mut Vec<Violation>,
    code: ViolationCode,
    mut subject_ids: Vec<String>,
) {
    subject_ids.sort();
    violations.push(Violation { code, subject_ids });
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn validate_footprint(footprint: &Footprint, subject_id: &str, violations: &mut Vec<Violation>) {
    match footprint {
        Footprint::AxisAlignedRect(rect) if !rect.is_valid() => add_violation(
            violations,
            ViolationCode::InvalidFootprint,
            vec![subject_id.to_owned()],
        ),
        Footprint::Unsupported { .. } => add_violation(
            violations,
            ViolationCode::UnsupportedFootprint,
            vec![subject_id.to_owned()],
        ),
        Footprint::AxisAlignedRect(_) => {}
    }
}

fn validate_time_range(range: TimeRange, subject_id: &str, violations: &mut Vec<Violation>) {
    if !range.is_valid() {
        add_violation(
            violations,
            ViolationCode::InvalidTimeRange,
            vec![subject_id.to_owned()],
        );
    }
}

/// Independently verify geometry, temporal occupancy, access and exclusive vehicle use.
///
/// Reservations use half-open intervals: one task may start exactly when another finishes.
/// This verifier is intentionally conservative about missing/unsupported facts and returns
/// every discoverable violation in deterministic order. It does not attempt to repair a plan.
pub fn verify(scenario: &Scenario) -> VerificationReport {
    let mut violations = Vec::new();

    // Missing/invalid spatial boundaries cannot qualify a finite yard plan.
    match scenario.yard_bounds {
        None => add_violation(
            &mut violations,
            ViolationCode::MissingYardBounds,
            Vec::new(),
        ),
        Some(bounds) if !bounds.is_valid() => add_violation(
            &mut violations,
            ViolationCode::InvalidYardBounds,
            Vec::new(),
        ),
        Some(_) => {}
    }

    // A vacuously consistent empty input must not be promoted to a qualified yard plan.
    if scenario.placements.is_empty() && scenario.transfers.is_empty() {
        add_violation(
            &mut violations,
            ViolationCode::NoOperationalReservations,
            Vec::new(),
        );
    }
    let mut placement_ids = HashSet::new();
    let mut transfer_ids = HashSet::new();
    let mut exclusion_ids = HashSet::new();

    for placement in &scenario.placements {
        if !nonempty(&placement.reservation_id) {
            add_violation(
                &mut violations,
                ViolationCode::EmptyPlacementId,
                vec![placement.reservation_id.clone()],
            );
        } else if !placement_ids.insert(placement.reservation_id.as_str()) {
            add_violation(
                &mut violations,
                ViolationCode::DuplicatePlacementId,
                vec![placement.reservation_id.clone()],
            );
        }
        if !nonempty(&placement.block_id) {
            add_violation(
                &mut violations,
                ViolationCode::EmptyBlockId,
                vec![placement.reservation_id.clone()],
            );
        }
        validate_footprint(
            &placement.footprint,
            &placement.reservation_id,
            &mut violations,
        );
        validate_time_range(
            placement.occupied,
            &placement.reservation_id,
            &mut violations,
        );
        if let Some(bounds) = scenario.yard_bounds.filter(|bounds| bounds.is_valid())
            && let Footprint::AxisAlignedRect(rect) = &placement.footprint
            && rect.is_valid()
            && !bounds.contains(*rect)
        {
            add_violation(
                &mut violations,
                ViolationCode::OutOfBoundsPlacement,
                vec![placement.reservation_id.clone()],
            );
        }
    }

    for transfer in &scenario.transfers {
        if !nonempty(&transfer.id) {
            add_violation(
                &mut violations,
                ViolationCode::EmptyTransferId,
                vec![transfer.id.clone()],
            );
        } else if !transfer_ids.insert(transfer.id.as_str()) {
            add_violation(
                &mut violations,
                ViolationCode::DuplicateTransferId,
                vec![transfer.id.clone()],
            );
        }
        if !nonempty(&transfer.block_id) {
            add_violation(
                &mut violations,
                ViolationCode::EmptyBlockId,
                vec![transfer.id.clone()],
            );
        }
        if !nonempty(&transfer.vehicle_id) {
            add_violation(
                &mut violations,
                ViolationCode::EmptyVehicleId,
                vec![transfer.id.clone()],
            );
        }
        validate_footprint(&transfer.corridor, &transfer.id, &mut violations);
        validate_time_range(transfer.interval, &transfer.id, &mut violations);
        if let Some(bounds) = scenario.yard_bounds.filter(|bounds| bounds.is_valid())
            && let Footprint::AxisAlignedRect(rect) = &transfer.corridor
            && rect.is_valid()
            && !bounds.contains(*rect)
        {
            add_violation(
                &mut violations,
                ViolationCode::OutOfBoundsTransferCorridor,
                vec![transfer.id.clone()],
            );
        }

        let mut seen_predecessors = HashSet::new();
        for predecessor_id in &transfer.predecessor_move_ids {
            if !nonempty(predecessor_id) {
                add_violation(
                    &mut violations,
                    ViolationCode::EmptyPredecessorId,
                    vec![transfer.id.clone()],
                );
            } else if !seen_predecessors.insert(predecessor_id.as_str()) {
                add_violation(
                    &mut violations,
                    ViolationCode::DuplicatePredecessor,
                    vec![transfer.id.clone(), predecessor_id.clone()],
                );
            }
            if predecessor_id == &transfer.id {
                add_violation(
                    &mut violations,
                    ViolationCode::SelfPredecessor,
                    vec![transfer.id.clone()],
                );
            }
        }
    }

    for zone in &scenario.exclusion_zones {
        if !nonempty(&zone.id) {
            add_violation(
                &mut violations,
                ViolationCode::EmptyExclusionId,
                vec![zone.id.clone()],
            );
        } else if !exclusion_ids.insert(zone.id.as_str()) {
            add_violation(
                &mut violations,
                ViolationCode::DuplicateExclusionId,
                vec![zone.id.clone()],
            );
        }
        validate_footprint(&zone.footprint, &zone.id, &mut violations);
        validate_time_range(zone.active, &zone.id, &mut violations);
        if let Some(bounds) = scenario.yard_bounds.filter(|bounds| bounds.is_valid())
            && let Footprint::AxisAlignedRect(rect) = &zone.footprint
            && rect.is_valid()
            && !bounds.contains(*rect)
        {
            add_violation(
                &mut violations,
                ViolationCode::OutOfBoundsExclusionZone,
                vec![zone.id.clone()],
            );
        }
    }

    // Only uniquely identified transfers can be used to establish precedence.
    let mut transfers_by_id: BTreeMap<&str, Vec<&Transfer>> = BTreeMap::new();
    for transfer in &scenario.transfers {
        if nonempty(&transfer.id) {
            transfers_by_id
                .entry(transfer.id.as_str())
                .or_default()
                .push(transfer);
        }
    }

    for transfer in &scenario.transfers {
        if !transfer.interval.is_valid() {
            continue;
        }
        for predecessor_id in &transfer.predecessor_move_ids {
            if !nonempty(predecessor_id) || predecessor_id == &transfer.id {
                continue;
            }
            match transfers_by_id.get(predecessor_id.as_str()) {
                None => add_violation(
                    &mut violations,
                    ViolationCode::MissingPredecessor,
                    vec![transfer.id.clone(), predecessor_id.clone()],
                ),
                Some(matches) if matches.len() == 1 => {
                    let predecessor = matches[0];
                    if predecessor.interval.is_valid()
                        && predecessor.interval.end_tick > transfer.interval.start_tick
                    {
                        add_violation(
                            &mut violations,
                            ViolationCode::PrecedenceViolation,
                            vec![transfer.id.clone(), predecessor.id.clone()],
                        );
                    }
                }
                Some(_) => {
                    // Duplicate predecessor transfer IDs are already a violation. Do not trust
                    // an arbitrary matching record to establish precedence.
                }
            }
        }
    }

    for (left_index, left) in scenario.placements.iter().enumerate() {
        for right in scenario.placements.iter().skip(left_index + 1) {
            if !left.occupied.is_valid()
                || !right.occupied.is_valid()
                || !left.occupied.overlaps(right.occupied)
                || !left.footprint.is_supported_and_valid()
                || !right.footprint.is_supported_and_valid()
            {
                continue;
            }

            if left.block_id == right.block_id && nonempty(&left.block_id) {
                add_violation(
                    &mut violations,
                    ViolationCode::BlockDoubleBooked,
                    vec![left.reservation_id.clone(), right.reservation_id.clone()],
                );
            } else if left.block_id != right.block_id
                && left.footprint.overlaps_supported(&right.footprint) == Some(true)
            {
                add_violation(
                    &mut violations,
                    ViolationCode::BlockFootprintOverlap,
                    vec![left.reservation_id.clone(), right.reservation_id.clone()],
                );
            }
        }
    }

    for placement in &scenario.placements {
        for zone in &scenario.exclusion_zones {
            if placement.occupied.is_valid()
                && zone.active.is_valid()
                && placement.occupied.overlaps(zone.active)
                && placement.footprint.is_supported_and_valid()
                && zone.footprint.is_supported_and_valid()
                && placement.footprint.overlaps_supported(&zone.footprint) == Some(true)
            {
                add_violation(
                    &mut violations,
                    ViolationCode::BlockInsideExclusion,
                    vec![placement.reservation_id.clone(), zone.id.clone()],
                );
            }
        }
    }

    for transfer in &scenario.transfers {
        for placement in &scenario.placements {
            if transfer.block_id == placement.block_id
                || !nonempty(&transfer.block_id)
                || !transfer.interval.is_valid()
                || !placement.occupied.is_valid()
                || !transfer.interval.overlaps(placement.occupied)
                || !transfer.corridor.is_supported_and_valid()
                || !placement.footprint.is_supported_and_valid()
            {
                continue;
            }
            if transfer.corridor.overlaps_supported(&placement.footprint) == Some(true) {
                add_violation(
                    &mut violations,
                    ViolationCode::MoveBlockedByBlock,
                    vec![transfer.id.clone(), placement.reservation_id.clone()],
                );
            }
        }

        for zone in &scenario.exclusion_zones {
            if transfer.interval.is_valid()
                && zone.active.is_valid()
                && transfer.interval.overlaps(zone.active)
                && transfer.corridor.is_supported_and_valid()
                && zone.footprint.is_supported_and_valid()
                && transfer.corridor.overlaps_supported(&zone.footprint) == Some(true)
            {
                add_violation(
                    &mut violations,
                    ViolationCode::MoveIntersectsExclusion,
                    vec![transfer.id.clone(), zone.id.clone()],
                );
            }
        }
    }

    for (left_index, left) in scenario.transfers.iter().enumerate() {
        for right in scenario.transfers.iter().skip(left_index + 1) {
            if nonempty(&left.vehicle_id)
                && left.vehicle_id == right.vehicle_id
                && left.interval.is_valid()
                && right.interval.is_valid()
                && left.interval.overlaps(right.interval)
            {
                add_violation(
                    &mut violations,
                    ViolationCode::VehicleOverlap,
                    vec![left.id.clone(), right.id.clone()],
                );
            }
        }
    }

    violations.sort_by(|left, right| {
        left.code
            .cmp(&right.code)
            .then_with(|| left.subject_ids.cmp(&right.subject_ids))
    });

    VerificationReport { violations }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn rect(min_x: i64, min_y: i64, max_x: i64, max_y: i64) -> Footprint {
        Footprint::AxisAlignedRect(Rect {
            min_x,
            min_y,
            max_x,
            max_y,
        })
    }

    fn placement(
        reservation_id: &str,
        block_id: &str,
        footprint: Footprint,
        start_tick: u64,
        end_tick: u64,
    ) -> Placement {
        Placement {
            reservation_id: reservation_id.to_owned(),
            block_id: block_id.to_owned(),
            footprint,
            occupied: TimeRange {
                start_tick,
                end_tick,
            },
        }
    }

    fn transfer(
        id: &str,
        block_id: &str,
        corridor: Footprint,
        start_tick: u64,
        end_tick: u64,
        vehicle_id: &str,
    ) -> Transfer {
        Transfer {
            id: id.to_owned(),
            block_id: block_id.to_owned(),
            corridor,
            interval: TimeRange {
                start_tick,
                end_tick,
            },
            vehicle_id: vehicle_id.to_owned(),
            predecessor_move_ids: Vec::new(),
        }
    }

    fn has_code(report: &VerificationReport, code: ViolationCode) -> bool {
        report
            .violations
            .iter()
            .any(|violation| violation.code == code)
    }

    #[test]
    fn missing_yard_bounds_fail_closed() {
        let scenario = Scenario {
            placements: vec![placement("p-1", "block-1", rect(0, 0, 2, 2), 0, 2)],
            ..Scenario::default()
        };
        assert!(has_code(
            &verify(&scenario),
            ViolationCode::MissingYardBounds
        ));
    }

    #[test]
    fn out_of_bounds_blocks_and_transfer_corridors_are_rejected() {
        let yard_bounds = Some(Rect {
            min_x: 0,
            min_y: 0,
            max_x: 10,
            max_y: 10,
        });
        let block = placement("p-1", "block-1", rect(9, 0, 11, 2), 0, 2);
        let move_outside = transfer("move-1", "block-2", rect(9, 1, 11, 2), 0, 2, "tug-1");
        let report = verify(&Scenario {
            yard_bounds,
            placements: vec![block],
            transfers: vec![move_outside],
            ..Scenario::default()
        });
        assert!(has_code(&report, ViolationCode::OutOfBoundsPlacement));
        assert!(has_code(
            &report,
            ViolationCode::OutOfBoundsTransferCorridor
        ));
    }

    #[test]
    fn exclusion_zones_outside_yard_bounds_fail_closed() {
        let scenario = Scenario {
            yard_bounds: Some(Rect {
                min_x: 0,
                min_y: 0,
                max_x: 10,
                max_y: 10,
            }),
            placements: vec![placement("p-1", "block-1", rect(0, 0, 1, 1), 0, 2)],
            exclusion_zones: vec![ExclusionZone {
                id: "zone-outside".to_owned(),
                footprint: rect(9, 1, 11, 2),
                active: TimeRange {
                    start_tick: 0,
                    end_tick: 2,
                },
            }],
            ..Scenario::default()
        };
        assert!(has_code(
            &verify(&scenario),
            ViolationCode::OutOfBoundsExclusionZone
        ));
    }

    #[test]
    fn empty_scenario_is_not_mistaken_for_a_qualified_yard_plan() {
        let report = verify(&Scenario::default());
        assert!(!report.is_feasible());
        assert_eq!(report.count(ViolationCode::NoOperationalReservations), 1);
    }

    #[test]
    fn rectangles_agree_with_independent_integer_cell_oracle() {
        fn rasterized_cells(rect: Rect) -> BTreeSet<(i64, i64)> {
            let mut cells = BTreeSet::new();
            for x in rect.min_x..rect.max_x {
                for y in rect.min_y..rect.max_y {
                    cells.insert((x, y));
                }
            }
            cells
        }

        let mut rectangles = Vec::new();
        for min_x in -2..=2 {
            for max_x in (min_x + 1)..=3 {
                for min_y in -2..=2 {
                    for max_y in (min_y + 1)..=3 {
                        rectangles.push(Rect {
                            min_x,
                            min_y,
                            max_x,
                            max_y,
                        });
                    }
                }
            }
        }

        for left in &rectangles {
            let left_cells = rasterized_cells(*left);
            for right in &rectangles {
                let right_cells = rasterized_cells(*right);
                let oracle_overlap = !left_cells.is_disjoint(&right_cells);
                assert_eq!(
                    left.overlaps(*right),
                    oracle_overlap,
                    "rectangles differ: {left:?} and {right:?}"
                );
            }
        }
    }

    #[test]
    fn intervals_agree_with_independent_tick_oracle() {
        fn tick_oracle(left: TimeRange, right: TimeRange) -> bool {
            (0..8).any(|tick| {
                left.start_tick <= tick
                    && tick < left.end_tick
                    && right.start_tick <= tick
                    && tick < right.end_tick
            })
        }

        for a_start in 0..6 {
            for a_end in (a_start + 1)..=7 {
                for b_start in 0..6 {
                    for b_end in (b_start + 1)..=7 {
                        let left = TimeRange {
                            start_tick: a_start,
                            end_tick: a_end,
                        };
                        let right = TimeRange {
                            start_tick: b_start,
                            end_tick: b_end,
                        };
                        assert_eq!(
                            left.overlaps(right),
                            tick_oracle(left, right),
                            "intervals differ: {left:?} and {right:?}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn half_open_rectangles_may_touch_edges_without_overlap() {
        let left = Rect {
            min_x: 0,
            min_y: 0,
            max_x: 2,
            max_y: 2,
        };
        let right = Rect {
            min_x: 2,
            min_y: 0,
            max_x: 4,
            max_y: 2,
        };
        assert!(!left.overlaps(right));
    }

    #[test]
    fn unsupported_footprints_never_become_free_space() {
        let scenario = Scenario {
            yard_bounds: Some(Rect {
                min_x: 0,
                min_y: 0,
                max_x: 10,
                max_y: 10,
            }),
            placements: vec![placement(
                "p-1",
                "block-1",
                Footprint::Unsupported {
                    profile: "polygon-v1".to_owned(),
                },
                0,
                2,
            )],
            ..Scenario::default()
        };
        let report = verify(&scenario);
        assert!(!report.is_feasible());
        assert_eq!(report.count(ViolationCode::UnsupportedFootprint), 1);
    }

    #[test]
    fn verifier_output_is_stable_under_input_permutation() {
        let a = placement("p-a", "block-a", rect(0, 0, 3, 3), 0, 4);
        let b = placement("p-b", "block-b", rect(2, 2, 4, 4), 1, 5);
        let yard_bounds = Some(Rect {
            min_x: 0,
            min_y: 0,
            max_x: 10,
            max_y: 10,
        });
        let forward = verify(&Scenario {
            yard_bounds,
            placements: vec![a.clone(), b.clone()],
            ..Scenario::default()
        });
        let reverse = verify(&Scenario {
            yard_bounds,
            placements: vec![b, a],
            ..Scenario::default()
        });
        assert_eq!(forward, reverse);
    }

    #[test]
    fn vehicle_overlap_is_detected_but_endpoint_contact_is_allowed() {
        let a = transfer("m-a", "block-a", rect(0, 0, 1, 1), 0, 3, "tug-1");
        let b = transfer("m-b", "block-b", rect(4, 0, 5, 1), 2, 4, "tug-1");
        let yard_bounds = Some(Rect {
            min_x: 0,
            min_y: 0,
            max_x: 10,
            max_y: 10,
        });
        let report = verify(&Scenario {
            yard_bounds,
            transfers: vec![a.clone(), b],
            ..Scenario::default()
        });
        assert!(has_code(&report, ViolationCode::VehicleOverlap));

        let endpoint = transfer("m-c", "block-c", rect(7, 0, 8, 1), 3, 5, "tug-1");
        let report = verify(&Scenario {
            yard_bounds,
            transfers: vec![a, endpoint],
            ..Scenario::default()
        });
        assert!(!has_code(&report, ViolationCode::VehicleOverlap));
    }
}
