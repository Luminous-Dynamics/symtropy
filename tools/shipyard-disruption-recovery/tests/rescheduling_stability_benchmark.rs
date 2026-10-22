//! Paired, deterministic smoke benchmark for recovery stability metrics.
//!
//! This is not an industrial scheduling contest: it compares known-answer policies on a
//! deliberately small spatial fixture to verify that the evaluator can distinguish localized
//! repair from a feasible but unnecessarily broad spatial translation or temporal right-shift. Every candidate is independently
//! passed through the recovery evaluator; no runtime or global-optimality claim is made.

use shipyard_disruption_recovery::{Disruption, RecoveryInput, evaluate_recovery};
use shipyard_space_time::{ExclusionZone, Footprint, Placement, Rect, Scenario, TimeRange};

const CASES: usize = 256;

#[derive(Clone, Copy)]
enum CandidatePolicy {
    NoOp,
    LocalRepair,
    GlobalSpatialShift,
    GlobalTemporalRightShift,
}

fn rect(min_x: i64, min_y: i64, max_x: i64, max_y: i64) -> Footprint {
    Footprint::AxisAlignedRect(Rect {
        min_x,
        min_y,
        max_x,
        max_y,
    })
}

fn baseline_for(case_index: usize) -> (Scenario, Disruption) {
    let placement_count = 3 + (case_index % 6);
    let row = (case_index % 3) as i64;
    let start_tick = 5 + (case_index % 7) as u64;
    let end_tick = start_tick + 2 + (case_index % 4) as u64;

    let placements = (0..placement_count)
        .map(|index| {
            let x = (index as i64) * 2;
            Placement {
                reservation_id: format!("placement-{index:02}"),
                block_id: format!("block-{index:02}"),
                footprint: rect(x, row, x + 1, row + 1),
                occupied: TimeRange { start_tick, end_tick },
            }
        })
        .collect();

    let baseline = Scenario {
        yard_bounds: Some(Rect {
            min_x: 0,
            min_y: 0,
            max_x: (placement_count as i64) * 2 + 1,
            max_y: row + 2,
        }),
        placements,
        ..Scenario::default()
    };

    let disruption = Disruption {
        // Observe the closure before these future reservations begin; this keeps the
        // comparison about rescheduling choices rather than rewriting work already in progress.
        observed_at_tick: start_tick - 1,
        new_exclusion_zones: vec![ExclusionZone {
            id: format!("zone-{case_index:03}"),
            footprint: rect(0, row, 1, row + 1),
            active: TimeRange { start_tick, end_tick },
        }],
        vehicle_outages: Vec::new(),
        placement_occupancy_extensions: Vec::new(),
        explicitly_affected_reservation_ids: Vec::new(),
    };

    (baseline, disruption)
}

fn candidate_for(baseline: &Scenario, policy: CandidatePolicy) -> Scenario {
    let mut candidate = baseline.clone();
    for (index, placement) in candidate.placements.iter_mut().enumerate() {
        let (min_x, min_y, max_x, max_y) = match &placement.footprint {
            Footprint::AxisAlignedRect(value) => (
                value.min_x,
                value.min_y,
                value.max_x,
                value.max_y,
            ),
            _ => unreachable!("benchmark only constructs axis-aligned rectangles"),
        };
        let shift = match policy {
            CandidatePolicy::NoOp => 0,
            CandidatePolicy::LocalRepair if index == 0 => 1,
            CandidatePolicy::LocalRepair => 0,
            CandidatePolicy::GlobalSpatialShift => 1,
            CandidatePolicy::GlobalTemporalRightShift => 0,
        };
        placement.footprint = rect(min_x + shift, min_y, max_x + shift, max_y);
        if matches!(policy, CandidatePolicy::GlobalTemporalRightShift) {
            let duration = placement.occupied.end_tick - placement.occupied.start_tick;
            placement.occupied.start_tick += duration;
            placement.occupied.end_tick += duration;
        }
    }
    candidate
}

fn evaluate(
    baseline: &Scenario,
    candidate: Scenario,
    disruption: &Disruption,
) -> shipyard_disruption_recovery::RecoveryReport {
    evaluate_recovery(&RecoveryInput {
        baseline: baseline.clone(),
        candidate,
        disruption: disruption.clone(),
    })
}

#[test]
fn localized_repair_preserves_feasibility_with_less_churn_than_global_right_shift() {
    let mut total_placements = 0usize;
    let mut total_local_changed = 0usize;
    let mut total_global_changed = 0usize;
    let mut total_local_unaffected_churn = 0usize;
    let mut total_global_unaffected_churn = 0usize;
    let mut total_local_edge_shift = 0u64;
    let mut total_global_edge_shift = 0u64;
    let mut total_temporal_right_shift_ticks = 0u64;

    for case_index in 0..CASES {
        let (baseline, disruption) = baseline_for(case_index);
        let placement_count = baseline.placements.len();
        total_placements += placement_count;

        let no_op = evaluate(
            &baseline,
            candidate_for(&baseline, CandidatePolicy::NoOp),
            &disruption,
        );
        assert!(no_op.metrics.baseline_feasible_before_disruption);
        assert!(!no_op.metrics.baseline_feasible_after_disruption);
        assert!(!no_op.accepted, "no-op must fail on case {case_index}");

        let local = evaluate(
            &baseline,
            candidate_for(&baseline, CandidatePolicy::LocalRepair),
            &disruption,
        );
        let global = evaluate(
            &baseline,
            candidate_for(&baseline, CandidatePolicy::GlobalSpatialShift),
            &disruption,
        );

        assert!(local.accepted, "localized repair failed on case {case_index}: {:?}", local.violations);
        assert!(global.accepted, "global right-shift failed on case {case_index}: {:?}", global.violations);
        assert_eq!(local.metrics.changed_placement_reservations, 1);
        assert_eq!(local.metrics.unaffected_placement_churn, 0);
        assert_eq!(global.metrics.changed_placement_reservations, placement_count);
        assert_eq!(
            global.metrics.unaffected_placement_churn,
            placement_count - 1
        );
        assert!(
            local.metrics.rectangle_edge_shift_units < global.metrics.rectangle_edge_shift_units,
            "localized repair should move fewer rectangle edges than a global spatial translation on case {case_index}"
        );

        let temporal = evaluate(
            &baseline,
            candidate_for(&baseline, CandidatePolicy::GlobalTemporalRightShift),
            &disruption,
        );
        assert!(temporal.accepted, "temporal right-shift failed on case {case_index}: {:?}", temporal.violations);
        assert_eq!(temporal.metrics.changed_placement_reservations, placement_count);
        assert_eq!(temporal.metrics.unaffected_placement_churn, placement_count - 1);
        assert_eq!(temporal.metrics.rectangle_edge_shift_units, 0);
        assert_eq!(
            temporal.metrics.time_shift_ticks,
            (placement_count as u64) * 2 * (disruption.new_exclusion_zones[0].active.end_tick - disruption.new_exclusion_zones[0].active.start_tick)
        );
        assert_eq!(local.metrics.time_shift_ticks, 0);
        total_temporal_right_shift_ticks += temporal.metrics.time_shift_ticks;

        total_local_changed += local.metrics.changed_placement_reservations;
        total_global_changed += global.metrics.changed_placement_reservations;
        total_local_unaffected_churn += local.metrics.unaffected_placement_churn;
        total_global_unaffected_churn += global.metrics.unaffected_placement_churn;
        total_local_edge_shift += local.metrics.rectangle_edge_shift_units;
        total_global_edge_shift += global.metrics.rectangle_edge_shift_units;
    }

    // With 256 cases cycling through three to eight placements, the corpus contains 1,404
    // placement records. These aggregate known answers pin both policy behavior and metric totals.
    assert_eq!(total_placements, 1_404);
    assert_eq!(total_local_changed, CASES);
    assert_eq!(total_global_changed, total_placements);
    assert_eq!(total_local_unaffected_churn, 0);
    assert_eq!(total_global_unaffected_churn, 1_148);
    assert_eq!(total_local_edge_shift, 512);
    assert_eq!(total_global_edge_shift, 2_808);
    assert_eq!(total_temporal_right_shift_ticks, 9_964);
}
