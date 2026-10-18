//! Small, dependency-free verifier for synthetic shipyard space-time fixtures.
//!
//! Geometry is axis-aligned rectangles on an integer coordinate lattice; intervals
//! are half-open [start, end). This is a deliberately limited verification profile,
//! not a crane, lift, structural, soil-bearing, or real-yard safety model.

use std::collections::BTreeMap;

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
    DuplicateResourceSpec { resource_id: String },
    DuplicateOperation { operation_id: String },
    InvalidReservation { reservation_id: String },
    BlockOverlap { first_block: String, second_block: String, area_id: String },
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

    for access in &plan.access_windows {
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
        if capacities.contains_key(&resource.resource_id) {
            violations.push(Violation::DuplicateResourceSpec {
                resource_id: resource.resource_id.clone(),
            });
        } else {
            capacities.insert(resource.resource_id.clone(), resource.capacity);
        }
    }

    let mut events: BTreeMap<String, Vec<(u32, i64)>> = BTreeMap::new();
    for reservation in &plan.reservations {
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
        let before = operations.get(&edge.before);
        let after = operations.get(&edge.after);
        if before.is_none() {
            unknowns.push(Unknown::MissingOperation { operation_id: edge.before.clone() });
        }
        if after.is_none() {
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
