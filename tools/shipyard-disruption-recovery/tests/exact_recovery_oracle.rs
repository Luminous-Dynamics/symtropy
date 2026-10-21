//! Independent exhaustive recovery oracle for one disturbed placement.
//!
//! Candidates are enumerated on an integer lattice and checked using explicit cell sets rather
//! than the production rectangle-overlap helper or the evaluator's derived affected-set logic.

use std::collections::BTreeSet;

use shipyard_disruption_recovery::{
    Disruption, RecoveryInput, evaluate_recovery,
};
use shipyard_space_time::{ExclusionZone, Footprint, Placement, Rect, Scenario, TimeRange};

fn rect(min_x: i64, min_y: i64, max_x: i64, max_y: i64) -> Footprint {
    Footprint::AxisAlignedRect(Rect {
        min_x,
        min_y,
        max_x,
        max_y,
    })
}

fn cells(rect: Rect) -> BTreeSet<(i64, i64)> {
    let mut result = BTreeSet::new();
    for x in rect.min_x..rect.max_x {
        for y in rect.min_y..rect.max_y {
            result.insert((x, y));
        }
    }
    result
}

fn placement(id: &str, block: &str, footprint: Footprint, start: u64, end: u64) -> Placement {
    Placement {
        reservation_id: id.to_owned(),
        block_id: block.to_owned(),
        footprint,
        occupied: TimeRange {
            start_tick: start,
            end_tick: end,
        },
    }
}

#[test]
fn exact_recovery_oracle_finds_minimum_churn_feasible_relocation() {
    let yard = Rect {
        min_x: 0,
        min_y: 0,
        max_x: 6,
        max_y: 4,
    };
    let horizon = TimeRange {
        start_tick: 5,
        end_tick: 10,
    };
    let baseline = Scenario {
        yard_bounds: Some(yard),
        placements: vec![
            placement("a-past", "block-a", rect(0, 0, 2, 2), 0, 5),
            placement("a-future", "block-a", rect(0, 0, 2, 2), 5, 10),
            placement("b-future", "block-b", rect(2, 0, 4, 2), 0, 10),
            placement("c-future", "block-c", rect(4, 0, 6, 2), 0, 10),
        ],
        ..Scenario::default()
    };
    let disturbance_zone = ExclusionZone {
        id: "zone-a".to_owned(),
        footprint: rect(0, 0, 2, 2),
        active: horizon,
    };
    let disruption = Disruption {
        observed_at_tick: 5,
        new_exclusion_zones: vec![disturbance_zone.clone()],
        vehicle_outages: Vec::new(),
        placement_occupancy_extensions: Vec::new(),
        explicitly_affected_reservation_ids: Vec::new(),
    };

    let fixed: Vec<(&Placement, BTreeSet<(i64, i64)>)> = baseline
        .placements
        .iter()
        .filter(|placement| placement.reservation_id != "a-future")
        .filter_map(|placement| match &placement.footprint {
            Footprint::AxisAlignedRect(r) if placement.occupied.overlaps(horizon) => {
                Some((placement, cells(*r)))
            }
            _ => None,
        })
        .collect();
    let zone_cells = match disturbance_zone.footprint {
        Footprint::AxisAlignedRect(r) => cells(r),
        Footprint::Unsupported { .. } => panic!("fixed fixture uses supported rectangle"),
    };

    let mut exact_feasible = Vec::new();
    for min_y in yard.min_y..yard.max_y {
        for min_x in yard.min_x..yard.max_x {
            let candidate_rect = Rect {
                min_x,
                min_y,
                max_x: min_x + 2,
                max_y: min_y + 2,
            };
            if !yard.contains(candidate_rect) {
                continue;
            }
            let candidate_cells = cells(candidate_rect);
            if !candidate_cells.is_disjoint(&zone_cells) {
                continue;
            }
            if fixed
                .iter()
                .any(|(_, occupied)| !candidate_cells.is_disjoint(occupied))
            {
                continue;
            }

            let mut candidate = baseline.clone();
            let future = candidate
                .placements
                .iter_mut()
                .find(|placement| placement.reservation_id == "a-future")
                .expect("frozen disturbed reservation exists");
            future.footprint = Footprint::AxisAlignedRect(candidate_rect);

            let report = evaluate_recovery(&RecoveryInput {
                baseline: baseline.clone(),
                candidate,
                disruption: disruption.clone(),
            });
            assert!(report.accepted, "oracle candidate rejected: {:?}", report.violations);
            assert_eq!(report.metrics.unaffected_placement_churn, 0);
            assert_eq!(report.metrics.changed_placement_reservations, 1);
            exact_feasible.push((candidate_rect, report));
        }
    }

    assert_eq!(exact_feasible.len(), 5, "tiny recovery feasible set changed");

    let (best_rect, best_report) = exact_feasible
        .into_iter()
        .min_by_key(|(rect, report)| {
            (
                report.metrics.unaffected_placement_churn,
                report.metrics.changed_placement_reservations,
                rect.min_x.abs_diff(0) + rect.min_y.abs_diff(0) * 2,
                report.metrics.rectangle_edge_shift_units,
            )
        })
        .expect("at least one recovery exists");

    assert_eq!(
        best_rect,
        Rect {
            min_x: 0,
            min_y: 2,
            max_x: 2,
            max_y: 4,
        }
    );
    assert_eq!(best_report.metrics.rectangle_edge_shift_units, 4);
    assert_eq!(best_report.metrics.recovery_horizon_ticks, Some(5));
}
