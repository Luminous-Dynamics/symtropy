//! Generated oracle for a shared-resource portfolio spanning newbuild, repair and recycling.
//!
//! The reference computes sequential operation finishes, per-class tardiness and reserved-resource
//! time from its own generated durations and due ticks. It does not call the production audit's
//! operation-window or tardiness helpers.

use shipyard_disruption_recovery::resources::{
    ActivityOperationAssignment, ShipyardActivityClass, ResourceReservation, ResourceSchedule,
    audit_multipurpose_shipyard_portfolio,
};
use shipyard_space_time::TimeRange;

const CASES: usize = 512;
const REQUIRED: [ShipyardActivityClass; 3] = [
    ShipyardActivityClass::Newbuild,
    ShipyardActivityClass::Repair,
    ShipyardActivityClass::Recycling,
];

fn next_u32(state: &mut u32) -> u32 {
    *state ^= *state << 13;
    *state ^= *state >> 17;
    *state ^= *state << 5;
    *state
}

fn pick(state: &mut u32, bound: u32) -> u32 {
    next_u32(state) % bound
}

#[derive(Clone, Copy, Debug)]
struct ExpectedActivity {
    class: ShipyardActivityClass,
    due_tick: u64,
    start_tick: u64,
    finish_tick: u64,
    duration_ticks: u64,
}

#[test]
fn generated_multipurpose_portfolio_matches_independent_deadline_oracle() {
    let mut state = 0x5070_2026_u32;
    let mut cases = 0usize;
    let mut operations = 0usize;
    let mut on_time_operations = 0usize;
    let mut late_operations = 0usize;
    let mut total_tardiness_ticks = 0u128;
    let mut worst_tardiness_ticks = 0u64;
    let mut reserved_resource_ticks = 0u128;

    for case_index in 0..CASES {
        let ids = [
            ("build-a", "station-build"),
            ("repair-a", "station-repair"),
            ("recycle-a", "station-recycle"),
        ];
        let mut schedule = ResourceSchedule {
            precedence_constraints: Vec::new(),
            reservations: Vec::new(),
        };
        let mut assignments = Vec::new();
        let mut expected = Vec::new();
        let mut cursor = 0u64;

        for (index, (operation_id, station_id)) in ids.iter().enumerate() {
            let activity = REQUIRED[index];
            let duration_ticks = 1 + u64::from(pick(&mut state, 4));
            let due_tick = u64::from(pick(&mut state, 14));
            let start_tick = cursor;
            let finish_tick = start_tick + duration_ticks;
            let interval = TimeRange {
                start_tick,
                end_tick: finish_tick,
            };

            schedule.reservations.push(ResourceReservation {
                reservation_id: format!("{operation_id}-crane"),
                operation_id: (*operation_id).to_owned(),
                resource_id: "shared-crane-1".to_owned(),
                resource_kind: "gantry-crane".to_owned(),
                interval,
            });
            schedule.reservations.push(ResourceReservation {
                reservation_id: format!("{operation_id}-station"),
                operation_id: (*operation_id).to_owned(),
                resource_id: (*station_id).to_owned(),
                resource_kind: "workstation".to_owned(),
                interval,
            });
            assignments.push(ActivityOperationAssignment {
                operation_id: (*operation_id).to_owned(),
                activity,
                due_tick,
            });
            expected.push(ExpectedActivity {
                class: activity,
                due_tick,
                start_tick,
                finish_tick,
                duration_ticks,
            });
            cursor = finish_tick;
        }

        let report =
            audit_multipurpose_shipyard_portfolio(&schedule, &assignments, &REQUIRED);
        assert!(report.accepted, "case {case_index}: {report:?}");
        assert!(report.resource_schedule_feasible);
        assert_eq!(report.scheduled_operation_count, 3);
        assert_eq!(report.activity_metrics.len(), 3);

        for (metric, reference) in report.activity_metrics.iter().zip(expected.iter()) {
            let tardiness = reference.finish_tick.saturating_sub(reference.due_tick);
            assert_eq!(metric.activity, Some(reference.class));
            assert_eq!(metric.operation_count, 1);
            assert_eq!(metric.on_time_operations, if tardiness == 0 { 1 } else { 0 });
            assert_eq!(metric.late_operations, if tardiness > 0 { 1 } else { 0 });
            assert_eq!(metric.total_tardiness_ticks, u128::from(tardiness));
            assert_eq!(metric.worst_tardiness_ticks, tardiness);
            assert_eq!(
                metric.reserved_resource_ticks,
                u128::from(reference.duration_ticks) * 2
            );

            operations += 1;
            if tardiness == 0 {
                on_time_operations += 1;
            } else {
                late_operations += 1;
                total_tardiness_ticks =
                    total_tardiness_ticks.saturating_add(u128::from(tardiness));
            }
            worst_tardiness_ticks = worst_tardiness_ticks.max(tardiness);
            reserved_resource_ticks = reserved_resource_ticks
                .saturating_add(u128::from(reference.duration_ticks) * 2);
            assert_eq!(reference.finish_tick, reference.start_tick + reference.duration_ticks);
        }
        cases += 1;
    }

    assert_eq!(cases, CASES);
    assert_eq!(operations, 1_536);
    assert_eq!(on_time_operations, 982);
    assert_eq!(late_operations, 554);
    assert_eq!(total_tardiness_ticks, 2_054);
    assert_eq!(worst_tardiness_ticks, 11);
    assert_eq!(reserved_resource_ticks, 7_740);
}
