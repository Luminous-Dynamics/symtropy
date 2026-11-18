//! Small, dependency-free verifier for synthetic shipyard space-time fixtures.
//!
//! Geometry is axis-aligned rectangles on an integer coordinate lattice; intervals
//! are half-open [start, end). This is a deliberately limited verification profile,
//! not a crane, lift, structural, soil-bearing, or real-yard safety model.

use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Disposition {
    Feasible,
    Infeasible,
    Indeterminate,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Interval {
    pub start: u32,
    pub end: u32,
}

impl Interval {
    pub const fn new(start: u32, end: u32) -> Self {
        Self { start, end }
    }

    pub const fn is_valid(self) -> bool {
        self.start < self.end
    }

    pub const fn overlaps(self, other: Self) -> bool {
        self.start < other.end && other.start < self.end
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Rect {
    pub min_x: i64,
    pub min_y: i64,
    pub max_x: i64,
    pub max_y: i64,
}

impl Rect {
    pub const fn new(min_x: i64, min_y: i64, max_x: i64, max_y: i64) -> Self {
        Self { min_x, min_y, max_x, max_y }
    }

    pub const fn is_valid(self) -> bool {
        self.min_x < self.max_x && self.min_y < self.max_y
    }

    /// Half-open boundaries: sharing an edge is not an overlap.
    pub const fn intersects(self, other: Self) -> bool {
        self.min_x < other.max_x
            && other.min_x < self.max_x
            && self.min_y < other.max_y
            && other.min_y < self.max_y
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Placement {
    pub block_id: String,
    pub area_id: String,
    /// Missing footprint is unknown, never free space.
    pub footprint: Option<Rect>,
    pub occupied: Interval,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AccessWindow {
    pub access_id: String,
    pub area_id: String,
    pub corridor: Rect,
    pub required: Interval,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceSpec {
    pub resource_id: String,
    /// Missing capacity is unknown, not unlimited.
    pub capacity: Option<u32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Reservation {
    pub reservation_id: String,
    pub resource_id: String,
    pub interval: Interval,
    pub units: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Operation {
    pub operation_id: String,
    pub interval: Interval,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Precedence {
    pub before: String,
    pub after: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Plan {
    pub placements: Vec<Placement>,
    pub access_windows: Vec<AccessWindow>,
    pub resources: Vec<ResourceSpec>,
    pub reservations: Vec<Reservation>,
    pub operations: Vec<Operation>,
    pub precedences: Vec<Precedence>,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Violation {
    InvalidInterval { entity_id: String },
    InvalidFootprint { entity_id: String },
    InvalidIdentity { entity_kind: String, entity_id: String },
    DuplicateAccessWindow { access_id: String },
    DuplicateResourceSpec { resource_id: String },
    DuplicateOperation { operation_id: String },
    InvalidReservation { reservation_id: String },
    DuplicateReservation { reservation_id: String },
    BlockOverlap { first_block: String, second_block: String, area_id: String },
    BlockDoubleBooked { block_id: String, first_area_id: String, second_area_id: String },
    AccessBlocked { block_id: String, access_id: String, area_id: String },
    ResourceOverCapacity { resource_id: String, at: u32, capacity: u32, active_units: u64 },
    PrecedenceViolation { before: String, after: String },
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Unknown {
    MissingFootprint { block_id: String },
    MissingResourceSpec { reservation_id: String, resource_id: String },
    UnknownResourceCapacity { resource_id: String },
    MissingOperation { operation_id: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Verification {
    pub disposition: Disposition,
    pub violations: Vec<Violation>,
    pub unknowns: Vec<Unknown>,
}

/// Independently checks geometry occupancy, access windows, resource capacities,
/// and precedence for this limited synthetic profile.
///
/// Known hard violations dominate the disposition. Unknown evidence is still
/// returned alongside violations so it cannot disappear from the result.
pub fn verify(plan: &Plan) -> Verification {
    let mut violations = Vec::new();
    let mut unknowns = Vec::new();

    for placement in &plan.placements {
        if placement.block_id.trim().is_empty() {
            violations.push(Violation::InvalidIdentity {
                entity_kind: "block".to_owned(),
                entity_id: placement.block_id.clone(),
            });
        }
        if placement.area_id.trim().is_empty() {
            violations.push(Violation::InvalidIdentity {
                entity_kind: "area".to_owned(),
                entity_id: placement.area_id.clone(),
            });
        }
        if !placement.occupied.is_valid() {
            violations.push(Violation::InvalidInterval {
                entity_id: format!("placement:{}", placement.block_id),
            });
        }
        match placement.footprint {
            None => unknowns.push(Unknown::MissingFootprint {
                block_id: placement.block_id.clone(),
            }),
            Some(rect) if !rect.is_valid() => violations.push(Violation::InvalidFootprint {
                entity_id: format!("placement:{}", placement.block_id),
            }),
            Some(_) => {}
        }
    }

    let mut access_ids = BTreeSet::new();
    for access in &plan.access_windows {
        if access.access_id.trim().is_empty() {
            violations.push(Violation::InvalidIdentity {
                entity_kind: "access_window".to_owned(),
                entity_id: access.access_id.clone(),
            });
        }
        if !access_ids.insert(access.access_id.clone()) {
            violations.push(Violation::DuplicateAccessWindow {
                access_id: access.access_id.clone(),
            });
        }
        if access.area_id.trim().is_empty() {
            violations.push(Violation::InvalidIdentity {
                entity_kind: "area".to_owned(),
                entity_id: access.area_id.clone(),
            });
        }
        if !access.required.is_valid() {
            violations.push(Violation::InvalidInterval {
                entity_id: format!("access:{}", access.access_id),
            });
        }
        if !access.corridor.is_valid() {
            violations.push(Violation::InvalidFootprint {
                entity_id: format!("access:{}", access.access_id),
            });
        }
    }

    for i in 0..plan.placements.len() {
        for j in (i + 1)..plan.placements.len() {
            let first = &plan.placements[i];
            let second = &plan.placements[j];

            // A physical block may have successive placement records, but it
            // cannot occupy two places at the same time, even across areas.
            if first.block_id == second.block_id
                && first.occupied.is_valid()
                && second.occupied.is_valid()
                && first.occupied.overlaps(second.occupied)
            {
                violations.push(Violation::BlockDoubleBooked {
                    block_id: first.block_id.clone(),
                    first_area_id: first.area_id.clone(),
                    second_area_id: second.area_id.clone(),
                });
            }

            if first.area_id != second.area_id
                || !first.occupied.is_valid()
                || !second.occupied.is_valid()
                || !first.occupied.overlaps(second.occupied)
            {
                continue;
            }

            if let (Some(a), Some(b)) = (first.footprint, second.footprint) {
                if a.is_valid() && b.is_valid() && a.intersects(b) {
                    violations.push(Violation::BlockOverlap {
                        first_block: first.block_id.clone(),
                        second_block: second.block_id.clone(),
                        area_id: first.area_id.clone(),
                    });
                }
            }
        }
    }

    for placement in &plan.placements {
        let Some(footprint) = placement.footprint else { continue };
        if !footprint.is_valid() || !placement.occupied.is_valid() {
            continue;
        }
        for access in &plan.access_windows {
            if placement.area_id == access.area_id
                && access.corridor.is_valid()
                && access.required.is_valid()
                && placement.occupied.overlaps(access.required)
                && footprint.intersects(access.corridor)
            {
                violations.push(Violation::AccessBlocked {
                    block_id: placement.block_id.clone(),
                    access_id: access.access_id.clone(),
                    area_id: access.area_id.clone(),
                });
            }
        }
    }

    let mut capacities: BTreeMap<String, Option<u32>> = BTreeMap::new();
    for resource in &plan.resources {
        if resource.resource_id.trim().is_empty() {
            violations.push(Violation::InvalidIdentity {
                entity_kind: "resource".to_owned(),
                entity_id: resource.resource_id.clone(),
            });
            continue;
        }
        if capacities.contains_key(&resource.resource_id) {
            violations.push(Violation::DuplicateResourceSpec {
                resource_id: resource.resource_id.clone(),
            });
        } else {
            capacities.insert(resource.resource_id.clone(), resource.capacity);
        }
    }

    let mut events: BTreeMap<String, Vec<(u32, i64)>> = BTreeMap::new();
    let mut reservation_ids = BTreeSet::new();
    for reservation in &plan.reservations {
        if reservation.reservation_id.trim().is_empty() {
            violations.push(Violation::InvalidIdentity {
                entity_kind: "reservation".to_owned(),
                entity_id: reservation.reservation_id.clone(),
            });
            continue;
        }
        if !reservation_ids.insert(reservation.reservation_id.clone()) {
            violations.push(Violation::DuplicateReservation {
                reservation_id: reservation.reservation_id.clone(),
            });
            continue;
        }
        if reservation.resource_id.trim().is_empty() {
            violations.push(Violation::InvalidIdentity {
                entity_kind: "resource_reference".to_owned(),
                entity_id: reservation.resource_id.clone(),
            });
            continue;
        }
        if !reservation.interval.is_valid() || reservation.units == 0 {
            violations.push(Violation::InvalidReservation {
                reservation_id: reservation.reservation_id.clone(),
            });
            continue;
        }

        match capacities.get(&reservation.resource_id) {
            None => unknowns.push(Unknown::MissingResourceSpec {
                reservation_id: reservation.reservation_id.clone(),
                resource_id: reservation.resource_id.clone(),
            }),
            Some(None) => unknowns.push(Unknown::UnknownResourceCapacity {
                resource_id: reservation.resource_id.clone(),
            }),
            Some(Some(_)) => {
                let resource_events = events.entry(reservation.resource_id.clone()).or_default();
                resource_events.push((reservation.interval.start, i64::from(reservation.units)));
                resource_events.push((reservation.interval.end, -i64::from(reservation.units)));
            }
        }
    }

    for (resource_id, mut resource_events) in events {
        let capacity = capacities.get(&resource_id).copied().flatten().unwrap_or_default();
        // Release events sort before acquisitions at the same tick for [start, end).
        resource_events.sort_unstable();
        let mut active_units = 0_i64;
        for (at, delta) in resource_events {
            active_units += delta;
            if delta > 0 && active_units > i64::from(capacity) {
                violations.push(Violation::ResourceOverCapacity {
                    resource_id: resource_id.clone(),
                    at,
                    capacity,
                    active_units: u64::try_from(active_units).unwrap_or(u64::MAX),
                });
            }
        }
    }

    let mut operations: BTreeMap<String, Interval> = BTreeMap::new();
    for operation in &plan.operations {
        if operation.operation_id.trim().is_empty() {
            violations.push(Violation::InvalidIdentity {
                entity_kind: "operation".to_owned(),
                entity_id: operation.operation_id.clone(),
            });
            continue;
        }
        if !operation.interval.is_valid() {
            violations.push(Violation::InvalidInterval {
                entity_id: format!("operation:{}", operation.operation_id),
            });
            continue;
        }
        if operations.contains_key(&operation.operation_id) {
            violations.push(Violation::DuplicateOperation {
                operation_id: operation.operation_id.clone(),
            });
        } else {
            operations.insert(operation.operation_id.clone(), operation.interval);
        }
    }

    for edge in &plan.precedences {
        if edge.before.trim().is_empty() {
            violations.push(Violation::InvalidIdentity {
                entity_kind: "precedence_before_reference".to_owned(),
                entity_id: edge.before.clone(),
            });
        }
        if edge.after.trim().is_empty() {
            violations.push(Violation::InvalidIdentity {
                entity_kind: "precedence_after_reference".to_owned(),
                entity_id: edge.after.clone(),
            });
        }
        let before = if edge.before.trim().is_empty() {
            None
        } else {
            operations.get(&edge.before)
        };
        let after = if edge.after.trim().is_empty() {
            None
        } else {
            operations.get(&edge.after)
        };
        if before.is_none() && !edge.before.trim().is_empty() {
            unknowns.push(Unknown::MissingOperation { operation_id: edge.before.clone() });
        }
        if after.is_none() && !edge.after.trim().is_empty() {
            unknowns.push(Unknown::MissingOperation { operation_id: edge.after.clone() });
        }
        if let (Some(before), Some(after)) = (before, after) {
            if before.end > after.start {
                violations.push(Violation::PrecedenceViolation {
                    before: edge.before.clone(),
                    after: edge.after.clone(),
                });
            }
        }
    }

    violations.sort();
    violations.dedup();
    unknowns.sort();
    unknowns.dedup();

    let disposition = if !violations.is_empty() {
        Disposition::Infeasible
    } else if !unknowns.is_empty() {
        Disposition::Indeterminate
    } else {
        Disposition::Feasible
    };

    Verification { disposition, violations, unknowns }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn placement(block_id: &str, footprint: Option<Rect>, occupied: Interval) -> Placement {
        Placement {
            block_id: block_id.to_owned(),
            area_id: "yard-a".to_owned(),
            footprint,
            occupied,
        }
    }

    fn baseline() -> Plan {
        Plan {
            placements: vec![
                placement("a", Some(Rect::new(0, 0, 2, 2)), Interval::new(0, 2)),
                placement("b", Some(Rect::new(3, 0, 5, 2)), Interval::new(0, 2)),
            ],
            access_windows: vec![AccessWindow {
                access_id: "north-corridor".to_owned(),
                area_id: "yard-a".to_owned(),
                corridor: Rect::new(0, 5, 10, 6),
                required: Interval::new(0, 2),
            }],
            resources: vec![ResourceSpec {
                resource_id: "carrier-1".to_owned(),
                capacity: Some(1),
            }],
            reservations: vec![
                Reservation {
                    reservation_id: "move-a".to_owned(),
                    resource_id: "carrier-1".to_owned(),
                    interval: Interval::new(0, 1),
                    units: 1,
                },
                Reservation {
                    reservation_id: "move-b".to_owned(),
                    resource_id: "carrier-1".to_owned(),
                    interval: Interval::new(1, 2),
                    units: 1,
                },
            ],
            operations: vec![
                Operation { operation_id: "fabricate".to_owned(), interval: Interval::new(0, 1) },
                Operation { operation_id: "assemble".to_owned(), interval: Interval::new(1, 2) },
            ],
            precedences: vec![Precedence {
                before: "fabricate".to_owned(),
                after: "assemble".to_owned(),
            }],
        }
    }

    // Test-only oracle: enumerate discrete unit cells and integer time ticks.
    // It deliberately does not call Rect::intersects or Interval::overlaps.
    fn tiny_grid_oracle_overlap(first: &Placement, second: &Placement) -> bool {
        let (Some(a), Some(b)) = (first.footprint, second.footprint) else {
            return false;
        };

        for tick in 0..3_u32 {
            let first_active = first.occupied.start <= tick && tick < first.occupied.end;
            let second_active = second.occupied.start <= tick && tick < second.occupied.end;
            if !first_active || !second_active {
                continue;
            }

            for x in 0..3_i64 {
                for y in 0..3_i64 {
                    let first_covers_cell =
                        a.min_x <= x && x < a.max_x && a.min_y <= y && y < a.max_y;
                    let second_covers_cell =
                        b.min_x <= x && x < b.max_x && b.min_y <= y && y < b.max_y;
                    if first_covers_cell && second_covers_cell {
                        return true;
                    }
                }
            }
        }
        false
    }

    // Test-only capacity oracle: count active units at each integer tick.
    // It does not use the production event sweep or its sort ordering.
    fn tiny_tick_oracle_over_capacity(
        first: &Reservation,
        second: &Reservation,
        capacity: u32,
    ) -> bool {
        for tick in 0..3_u32 {
            let mut active = 0_u64;
            if first.interval.start <= tick && tick < first.interval.end {
                active += u64::from(first.units);
            }
            if second.interval.start <= tick && tick < second.interval.end {
                active += u64::from(second.units);
            }
            if active > u64::from(capacity) {
                return true;
            }
        }
        false
    }

    #[test]
    fn exhaustive_tiny_grid_oracle_matches_rectangle_time_overlap() {
        let mut rectangles = Vec::new();
        for min_x in 0..3_i64 {
            for max_x in (min_x + 1)..=3_i64 {
                for min_y in 0..3_i64 {
                    for max_y in (min_y + 1)..=3_i64 {
                        rectangles.push(Rect::new(min_x, min_y, max_x, max_y));
                    }
                }
            }
        }

        let mut intervals = Vec::new();
        for start in 0..3_u32 {
            for end in (start + 1)..=3_u32 {
                intervals.push(Interval::new(start, end));
            }
        }

        for first_rect in &rectangles {
            for first_interval in &intervals {
                let first = placement("first", Some(*first_rect), *first_interval);
                for second_rect in &rectangles {
                    for second_interval in &intervals {
                        let second = placement("second", Some(*second_rect), *second_interval);
                        let expected = tiny_grid_oracle_overlap(&first, &second);
                        let plan = Plan {
                            placements: vec![first.clone(), second.clone()],
                            ..Plan::default()
                        };
                        let result = verify(&plan);
                        let observed = result.violations.iter().any(|violation| matches!(
                            violation,
                            Violation::BlockOverlap { .. }
                        ));
                        assert_eq!(
                            observed,
                            expected,
                            "oracle mismatch: first={first:?}, second={second:?}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn exhaustive_tiny_tick_oracle_matches_resource_capacity_sweep() {
        let mut intervals = Vec::new();
        for start in 0..3_u32 {
            for end in (start + 1)..=3_u32 {
                intervals.push(Interval::new(start, end));
            }
        }

        for first_interval in &intervals {
            for second_interval in &intervals {
                for first_units in 1..=2_u32 {
                    for second_units in 1..=2_u32 {
                        for capacity in 1..=3_u32 {
                            let first = Reservation {
                                reservation_id: "first".to_owned(),
                                resource_id: "vehicle".to_owned(),
                                interval: *first_interval,
                                units: first_units,
                            };
                            let second = Reservation {
                                reservation_id: "second".to_owned(),
                                resource_id: "vehicle".to_owned(),
                                interval: *second_interval,
                                units: second_units,
                            };
                            let expected = tiny_tick_oracle_over_capacity(
                                &first,
                                &second,
                                capacity,
                            );
                            let plan = Plan {
                                resources: vec![ResourceSpec {
                                    resource_id: "vehicle".to_owned(),
                                    capacity: Some(capacity),
                                }],
                                reservations: vec![first, second],
                                ..Plan::default()
                            };
                            let result = verify(&plan);
                            let observed = result.violations.iter().any(|violation| matches!(
                                violation,
                                Violation::ResourceOverCapacity { .. }
                            ));
                            assert_eq!(
                                observed,
                                expected,
                                "capacity oracle mismatch: capacity={capacity}, plan={plan:?}"
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn blank_block_identity_is_rejected() {
        let mut plan = baseline();
        plan.placements[0].block_id = "   ".to_owned();
        let result = verify(&plan);
        assert_eq!(result.disposition, Disposition::Infeasible);
        assert!(result.violations.iter().any(|violation| matches!(
            violation,
            Violation::InvalidIdentity { entity_kind, .. } if entity_kind == "block"
        )));
    }

    #[test]
    fn blank_area_identity_is_rejected() {
        let mut plan = baseline();
        plan.placements[0].area_id.clear();
        let result = verify(&plan);
        assert_eq!(result.disposition, Disposition::Infeasible);
        assert!(result.violations.iter().any(|violation| matches!(
            violation,
            Violation::InvalidIdentity { entity_kind, .. } if entity_kind == "area"
        )));
    }

    #[test]
    fn blank_and_duplicate_access_window_identities_are_rejected() {
        let mut plan = baseline();
        plan.access_windows.push(plan.access_windows[0].clone());
        plan.access_windows[0].access_id = " ".to_owned();

        let result = verify(&plan);
        assert_eq!(result.disposition, Disposition::Infeasible);
        assert!(result.violations.iter().any(|violation| matches!(
            violation,
            Violation::InvalidIdentity { entity_kind, .. } if entity_kind == "access_window"
        )));
        assert!(result.violations.iter().any(|violation| matches!(
            violation,
            Violation::DuplicateAccessWindow { .. }
        )));
    }

    #[test]
    fn blank_resource_identity_is_rejected() {
        let mut plan = baseline();
        plan.resources[0].resource_id.clear();
        let result = verify(&plan);
        assert_eq!(result.disposition, Disposition::Infeasible);
        assert!(result.violations.iter().any(|violation| matches!(
            violation,
            Violation::InvalidIdentity { entity_kind, .. } if entity_kind == "resource"
        )));
    }

    #[test]
    fn blank_reservation_resource_reference_is_rejected() {
        let mut plan = baseline();
        plan.reservations[0].resource_id = " ".to_owned();
        let result = verify(&plan);
        assert_eq!(result.disposition, Disposition::Infeasible);
        assert!(result.violations.iter().any(|violation| matches!(
            violation,
            Violation::InvalidIdentity { entity_kind, .. } if entity_kind == "resource_reference"
        )));
    }

    #[test]
    fn blank_operation_identity_is_rejected() {
        let mut plan = baseline();
        plan.operations[0].operation_id.clear();
        let result = verify(&plan);
        assert_eq!(result.disposition, Disposition::Infeasible);
        assert!(result.violations.iter().any(|violation| matches!(
            violation,
            Violation::InvalidIdentity { entity_kind, .. } if entity_kind == "operation"
        )));
    }

    #[test]
    fn blank_precedence_reference_is_rejected_not_reported_as_missing_only() {
        let mut plan = baseline();
        plan.precedences[0].before = " ".to_owned();
        let result = verify(&plan);
        assert_eq!(result.disposition, Disposition::Infeasible);
        assert!(result.violations.iter().any(|violation| matches!(
            violation,
            Violation::InvalidIdentity { entity_kind, .. } if entity_kind == "precedence_before_reference"
        )));
        assert!(!result.unknowns.contains(&Unknown::MissingOperation {
            operation_id: " ".to_owned()
        }));
    }

    #[test]
    fn back_to_back_plan_is_feasible() {
        assert_eq!(verify(&baseline()).disposition, Disposition::Feasible);
    }

    #[test]
    fn overlapping_footprints_in_overlapping_time_are_rejected() {
        let mut plan = baseline();
        plan.placements.push(placement("c", Some(Rect::new(1, 1, 3, 3)), Interval::new(1, 3)));
        let result = verify(&plan);
        assert_eq!(result.disposition, Disposition::Infeasible);
        assert!(result.violations.iter().any(|v| matches!(v, Violation::BlockOverlap { .. })));
    }

    #[test]
    fn edge_touching_footprints_do_not_overlap() {
        let mut plan = baseline();
        plan.placements[1].footprint = Some(Rect::new(2, 0, 4, 2));
        assert_eq!(verify(&plan).disposition, Disposition::Feasible);
    }

    #[test]
    fn same_footprint_can_be_reused_in_non_overlapping_intervals() {
        let mut plan = baseline();
        plan.placements[1].footprint = plan.placements[0].footprint;
        plan.placements[1].occupied = Interval::new(2, 4);
        assert_eq!(verify(&plan).disposition, Disposition::Feasible);
    }

    #[test]
    fn same_block_cannot_be_double_booked_across_areas() {
        let mut plan = baseline();
        let mut second = placement(
            "a",
            Some(Rect::new(10, 0, 12, 2)),
            Interval::new(1, 3),
        );
        second.area_id = "yard-b".to_owned();
        plan.placements.push(second);

        let result = verify(&plan);
        assert_eq!(result.disposition, Disposition::Infeasible);
        assert!(result.violations.iter().any(|violation| matches!(
            violation,
            Violation::BlockDoubleBooked { block_id, .. } if block_id == "a"
        )));
    }

    #[test]
    fn same_block_can_have_successive_non_overlapping_placements() {
        let mut plan = baseline();
        let mut next = placement(
            "a",
            Some(Rect::new(10, 0, 12, 2)),
            Interval::new(2, 4),
        );
        next.area_id = "yard-b".to_owned();
        plan.placements.push(next);

        assert_eq!(verify(&plan).disposition, Disposition::Feasible);
    }

    #[test]
    fn required_access_corridor_cannot_be_occupied() {
        let mut plan = baseline();
        plan.placements[0].footprint = Some(Rect::new(2, 5, 4, 7));
        let result = verify(&plan);
        assert_eq!(result.disposition, Disposition::Infeasible);
        assert!(result.violations.iter().any(|v| matches!(v, Violation::AccessBlocked { .. })));
    }

    #[test]
    fn shared_capacity_one_resource_rejects_concurrent_reservations() {
        let mut plan = baseline();
        plan.reservations[1].interval = Interval::new(0, 2);
        let result = verify(&plan);
        assert_eq!(result.disposition, Disposition::Infeasible);
        assert!(result.violations.iter().any(|v| matches!(v, Violation::ResourceOverCapacity { .. })));
    }

    #[test]
    fn capacity_two_allows_two_concurrent_units() {
        let mut plan = baseline();
        plan.resources[0].capacity = Some(2);
        for reservation in &mut plan.reservations {
            reservation.interval = Interval::new(0, 2);
        }
        assert_eq!(verify(&plan).disposition, Disposition::Feasible);
    }

    #[test]
    fn duplicate_reservation_identity_is_rejected_below_resource_capacity() {
        let mut plan = baseline();
        plan.resources[0].capacity = Some(2);
        plan.reservations[1] = plan.reservations[0].clone();

        let result = verify(&plan);
        assert_eq!(result.disposition, Disposition::Infeasible);
        assert!(result.violations.iter().any(|violation| matches!(
            violation,
            Violation::DuplicateReservation { reservation_id } if reservation_id == "move-a"
        )));
    }

    #[test]
    fn missing_footprint_is_indeterminate_not_free_space() {
        let mut plan = baseline();
        plan.placements[0].footprint = None;
        let result = verify(&plan);
        assert_eq!(result.disposition, Disposition::Indeterminate);
        assert!(result.unknowns.contains(&Unknown::MissingFootprint { block_id: "a".to_owned() }));
    }

    #[test]
    fn missing_resource_capacity_is_indeterminate_not_unlimited() {
        let mut plan = baseline();
        plan.resources[0].capacity = None;
        let result = verify(&plan);
        assert_eq!(result.disposition, Disposition::Indeterminate);
        assert!(result.unknowns.iter().any(|u| matches!(u, Unknown::UnknownResourceCapacity { .. })));
    }

    #[test]
    fn absent_resource_spec_is_indeterminate() {
        let mut plan = baseline();
        plan.resources.clear();
        let result = verify(&plan);
        assert_eq!(result.disposition, Disposition::Indeterminate);
        assert!(result.unknowns.iter().any(|u| matches!(u, Unknown::MissingResourceSpec { .. })));
    }

    #[test]
    fn precedence_violation_is_rejected() {
        let mut plan = baseline();
        plan.operations[0].interval = Interval::new(0, 2);
        plan.operations[1].interval = Interval::new(1, 3);
        let result = verify(&plan);
        assert_eq!(result.disposition, Disposition::Infeasible);
        assert!(result.violations.iter().any(|v| matches!(v, Violation::PrecedenceViolation { .. })));
    }

    #[test]
    fn invalid_interval_is_rejected() {
        let mut plan = baseline();
        plan.placements[0].occupied = Interval::new(2, 2);
        let result = verify(&plan);
        assert_eq!(result.disposition, Disposition::Infeasible);
        assert!(result.violations.iter().any(|v| matches!(v, Violation::InvalidInterval { .. })));
    }

    #[test]
    fn violations_do_not_hide_unknown_geometry() {
        let mut plan = baseline();
        plan.placements.push(placement("c", Some(Rect::new(1, 1, 3, 3)), Interval::new(1, 3)));
        plan.placements.push(placement("unknown", None, Interval::new(0, 2)));
        let result = verify(&plan);
        assert_eq!(result.disposition, Disposition::Infeasible);
        assert!(result.violations.iter().any(|v| matches!(v, Violation::BlockOverlap { .. })));
        assert!(result.unknowns.contains(&Unknown::MissingFootprint {
            block_id: "unknown".to_owned()
        }));
    }
}
