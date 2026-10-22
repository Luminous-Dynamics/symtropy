//! Generated oracle for adaptive shipyard recourse with a non-anticipativity constraint.
//!
//! The reference computes one-cell feasibility and information-set action equality directly. It
//! does not call the production spatial verifier or the production policy evaluator to decide the
//! expected result.

use shipyard_disruption_recovery::resources::{
    JointDisruptionScenario, JointRecourseScenario, JointRecourseViolationCode,
    ResourceReservation, ResourceSchedule, evaluate_joint_recourse_policy,
};
use shipyard_disruption_recovery::Disruption;
use shipyard_space_time::{ExclusionZone, Footprint, Placement, Rect, Scenario, TimeRange};

const CASES: usize = 512;

fn next_u32(state: &mut u32) -> u32 {
    *state ^= *state << 13;
    *state ^= *state >> 17;
    *state ^= *state << 5;
    *state
}

fn pick(state: &mut u32, bound: u32) -> u32 {
    next_u32(state) % bound
}

fn one_block_plan(x: i64) -> Scenario {
    Scenario {
        yard_bounds: Some(Rect {
            min_x: 0,
            min_y: 0,
            max_x: 4,
            max_y: 1,
        }),
        placements: vec![Placement {
            reservation_id: "block-a-placement".to_owned(),
            block_id: "block-a".to_owned(),
            footprint: Footprint::AxisAlignedRect(Rect {
                min_x: x,
                min_y: 0,
                max_x: x + 1,
                max_y: 1,
            }),
            occupied: TimeRange {
                start_tick: 5,
                end_tick: 10,
            },
        }],
        transfers: Vec::new(),
        exclusion_zones: Vec::new(),
    }
}

fn resource_schedule() -> ResourceSchedule {
    ResourceSchedule {
        precedence_constraints: Vec::new(),
        reservations: vec![ResourceReservation {
            reservation_id: "block-a-workstation".to_owned(),
            operation_id: "block-a-operation".to_owned(),
            resource_id: "station-1".to_owned(),
            resource_kind: "workstation".to_owned(),
            interval: TimeRange {
                start_tick: 5,
                end_tick: 10,
            },
        }],
    }
}

fn case_scenario(
    scenario_id: &str,
    zone_x: i64,
    evidence_id: &str,
    candidate_x: i64,
) -> JointRecourseScenario {
    JointRecourseScenario {
        scenario: JointDisruptionScenario {
            scenario_id: scenario_id.to_owned(),
            spatial_disruption: Disruption {
                observed_at_tick: 4,
                new_exclusion_zones: vec![ExclusionZone {
                    id: format!("closure-{scenario_id}"),
                    footprint: Footprint::AxisAlignedRect(Rect {
                        min_x: zone_x,
                        min_y: 0,
                        max_x: zone_x + 1,
                        max_y: 1,
                    }),
                    active: TimeRange {
                        start_tick: 5,
                        end_tick: 10,
                    },
                }],
                vehicle_outages: Vec::new(),
                placement_occupancy_extensions: Vec::new(),
                explicitly_affected_reservation_ids: Vec::new(),
            },
            resource_outages: Vec::new(),
        },
        observed_evidence_ids: vec![evidence_id.to_owned()],
        candidate_spatial: one_block_plan(candidate_x),
        candidate_resources: resource_schedule(),
        candidate_bindings: Vec::new(),
    }
}

fn multi_zone_scenario(scenario_id: &str, closed_cells: &[i64]) -> JointDisruptionScenario {
    JointDisruptionScenario {
        scenario_id: scenario_id.to_owned(),
        spatial_disruption: Disruption {
            observed_at_tick: 4,
            new_exclusion_zones: closed_cells
                .iter()
                .enumerate()
                .map(|(index, x)| ExclusionZone {
                    id: format!("closure-{scenario_id}-{index}"),
                    footprint: Footprint::AxisAlignedRect(Rect {
                        min_x: *x,
                        min_y: 0,
                        max_x: *x + 1,
                        max_y: 1,
                    }),
                    active: TimeRange {
                        start_tick: 5,
                        end_tick: 10,
                    },
                })
                .collect(),
            vehicle_outages: Vec::new(),
            placement_occupancy_extensions: Vec::new(),
            explicitly_affected_reservation_ids: Vec::new(),
        },
        resource_outages: Vec::new(),
    }
}

/// Independent oracle for the single-cell specialization of this generated family.
fn exact_case(
    zone_x_a: i64,
    candidate_x_a: i64,
    zone_x_b: i64,
    candidate_x_b: i64,
    same_information_set: bool,
) -> (bool, usize, bool) {
    let feasible_a = zone_x_a != candidate_x_a;
    let feasible_b = zone_x_b != candidate_x_b;
    let anticipative = same_information_set && candidate_x_a != candidate_x_b;
    let accepted = feasible_a && feasible_b && !anticipative;
    let mut accepted_scenarios = 0usize;
    if feasible_a {
        accepted_scenarios += 1;
    }
    if feasible_b {
        accepted_scenarios += 1;
    }
    (accepted, accepted_scenarios, anticipative)
}

#[test]
fn evidence_triggered_recourse_succeeds_where_no_static_placement_can() {
    let baseline_spatial = one_block_plan(0);
    let baseline_resources = resource_schedule();
    let scenarios = [
        multi_zone_scenario("only-left-cell-open", &[1, 2, 3]),
        multi_zone_scenario("only-right-cell-open", &[0, 1, 2]),
    ];

    let mut static_candidates_accepted = 0usize;
    for x in 0..4 {
        let report = evaluate_joint_recovery_envelope(
            &baseline_spatial,
            &one_block_plan(x),
            &baseline_resources,
            &baseline_resources,
            &[],
            &[],
            &scenarios,
        );
        if report.robustly_accepted {
            static_candidates_accepted += 1;
        }
    }
    assert_eq!(
        static_candidates_accepted, 0,
        "no single cell is open in both disturbance scenarios"
    );

    let adaptive_policy = [
        JointRecourseScenario {
            scenario: scenarios[0].clone(),
            observed_evidence_ids: vec!["closure-left-evidence".to_owned()],
            candidate_spatial: one_block_plan(0),
            candidate_resources: resource_schedule(),
            candidate_bindings: Vec::new(),
        },
        JointRecourseScenario {
            scenario: scenarios[1].clone(),
            observed_evidence_ids: vec!["closure-right-evidence".to_owned()],
            candidate_spatial: one_block_plan(3),
            candidate_resources: resource_schedule(),
            candidate_bindings: Vec::new(),
        },
    ];

    let report = evaluate_joint_recourse_policy(
        &baseline_spatial,
        &baseline_resources,
        &[],
        &adaptive_policy,
    );

    assert!(report.robustly_accepted, "{:?}", report.violations);
    assert_eq!(report.metrics.total_scenarios, 2);
    assert_eq!(report.metrics.accepted_scenarios, 2);
    assert_eq!(report.metrics.observation_information_sets, 2);
    assert_eq!(report.metrics.distinct_actions, 2);
}

#[test]
fn generated_recourse_policy_matches_independent_non_anticipativity_oracle() {
    let mut state = 0xada0_2026_u32;
    let baseline_spatial = one_block_plan(0);
    let baseline_resources = resource_schedule();

    let mut cases = 0usize;
    let mut expected_policy_acceptances = 0usize;
    let mut actual_policy_acceptances = 0usize;
    let mut expected_non_anticipativity_rejections = 0usize;
    let mut actual_non_anticipativity_rejections = 0usize;
    let mut expected_accepted_scenarios = 0usize;
    let mut actual_accepted_scenarios = 0usize;

    for case_index in 0..CASES {
        let zone_x_a = i64::from(pick(&mut state, 4));
        let candidate_x_a = i64::from(pick(&mut state, 4));
        let zone_x_b = i64::from(pick(&mut state, 4));
        let candidate_x_b = i64::from(pick(&mut state, 4));
        let same_information_set = pick(&mut state, 2) == 0;
        let evidence_a = if same_information_set {
            "known-event-set"
        } else {
            "known-event-set-a"
        };
        let evidence_b = if same_information_set {
            "known-event-set"
        } else {
            "known-event-set-b"
        };

        let candidates = [
            case_scenario("future-a", zone_x_a, evidence_a, candidate_x_a),
            case_scenario("future-b", zone_x_b, evidence_b, candidate_x_b),
        ];
        let expected = exact_case(
            zone_x_a,
            candidate_x_a,
            zone_x_b,
            candidate_x_b,
            same_information_set,
        );
        let report = evaluate_joint_recourse_policy(
            &baseline_spatial,
            &baseline_resources,
            &[],
            &candidates,
        );

        assert_eq!(
            report.metrics.accepted_scenarios,
            expected.1,
            "scenario-feasibility mismatch in case {case_index}: {report:?}"
        );
        assert_eq!(
            report.robustly_accepted,
            expected.0,
            "policy eligibility mismatch in case {case_index}: {report:?}"
        );
        assert_eq!(
            report.violations.iter().any(|violation| {
                violation.code == JointRecourseViolationCode::NonAnticipativeActionMismatch
            }),
            expected.2,
            "non-anticipativity mismatch in case {case_index}: {report:?}"
        );

        let reversed = [candidates[1].clone(), candidates[0].clone()];
        let reversed_report = evaluate_joint_recourse_policy(
            &baseline_spatial,
            &baseline_resources,
            &[],
            &reversed,
        );
        assert_eq!(
            report, reversed_report,
            "policy report must not depend on scenario input order in case {case_index}"
        );

        cases += 1;
        if expected.0 {
            expected_policy_acceptances += 1;
        }
        if report.robustly_accepted {
            actual_policy_acceptances += 1;
        }
        if expected.2 {
            expected_non_anticipativity_rejections += 1;
        }
        if report.violations.iter().any(|violation| {
            violation.code == JointRecourseViolationCode::NonAnticipativeActionMismatch
        }) {
            actual_non_anticipativity_rejections += 1;
        }
        expected_accepted_scenarios += expected.1;
        actual_accepted_scenarios += report.metrics.accepted_scenarios;
    }

    assert_eq!(cases, CASES);
    assert_eq!(expected_policy_acceptances, 204);
    assert_eq!(expected_non_anticipativity_rejections, 190);
    assert_eq!(expected_accepted_scenarios, 810);
    assert_eq!(actual_policy_acceptances, expected_policy_acceptances);
    assert_eq!(
        actual_non_anticipativity_rejections,
        expected_non_anticipativity_rejections
    );
    assert_eq!(actual_accepted_scenarios, expected_accepted_scenarios);
}
