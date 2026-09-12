include!("manta_forge_generation_policy_metrics_v1.rs");

const POLICY_SURFACE_FIXTURE: &str =
    include_str!("../fixtures/manta-forge-policy-sensitivity-surface-v1.txt");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DynamicPolicySurfacePoint {
    maturity_periods: u64,
    founded_descendants: u64,
    maturity_completed_descendants: u64,
    descendant_reproduction_transitions: u64,
    terminal_residual_periods: u64,
}

fn simulate_fixed_case_policy_surface_point(
    case: &FrontierCase,
    maturity_periods: u64,
) -> DynamicPolicySurfacePoint {
    assert!(maturity_periods > 0);
    let mut state = first_successor(case).expect("selected fixed case must found v3");
    let mut generation = 3u64;
    let mut founded_descendants = 1u64;
    let mut maturity_completed_descendants = 0u64;
    let mut descendant_reproduction_transitions = 0u64;

    loop {
        let mut terminal_residual_periods = 0u64;
        let mut matured = true;
        for expected_tick in 1..=maturity_periods {
            let report = state.step().unwrap();
            assert_eq!(report.tick, expected_tick);
            if !report.essential_capabilities_available {
                matured = false;
                break;
            }
            terminal_residual_periods += 1;
        }

        if !matured {
            return DynamicPolicySurfacePoint {
                maturity_periods,
                founded_descendants,
                maturity_completed_descendants,
                descendant_reproduction_transitions,
                terminal_residual_periods,
            };
        }
        maturity_completed_descendants += 1;

        let tooling_id = format!("forge-tooling-v{generation}");
        let reactor_id = format!("reactor-service-v{generation}");
        let structural_id = format!("structural-stock-v{generation}");
        let tooling_units = state.dependency(&tooling_id).unwrap().inventory_units;
        let reactor_units = state.dependency(&reactor_id).unwrap().inventory_units;
        let structural_units = state.dependency(&structural_id).unwrap().inventory_units;

        if tooling_units == 0 || reactor_units == 0 {
            return DynamicPolicySurfacePoint {
                maturity_periods,
                founded_descendants,
                maturity_completed_descendants,
                descendant_reproduction_transitions,
                terminal_residual_periods,
            };
        }

        let next_generation = generation + 1;
        let (successor, receipt) = state
            .handoff_to(
                descendant_spec(case, next_generation),
                descendant_handoff_plan(
                    case,
                    generation,
                    next_generation,
                    tooling_units,
                    reactor_units,
                    structural_units,
                ),
            )
            .unwrap();
        assert_eq!(receipt.source_final_tick, maturity_periods);
        assert_eq!(receipt.source_final_observation_tick, Some(maturity_periods));
        assert!(receipt.source_final_shortage_ids.is_empty());
        assert!(receipt.source_final_unavailable_capability_ids.is_empty());

        founded_descendants += 1;
        descendant_reproduction_transitions += 1;
        state = successor;
        generation = next_generation;
    }
}

fn expected_surface_points() -> Vec<DynamicPolicySurfacePoint> {
    POLICY_SURFACE_FIXTURE
        .lines()
        .filter_map(|line| line.strip_prefix("policy="))
        .map(|line| {
            let mut point = DynamicPolicySurfacePoint {
                maturity_periods: 0,
                founded_descendants: 0,
                maturity_completed_descendants: 0,
                descendant_reproduction_transitions: 0,
                terminal_residual_periods: 0,
            };
            for field in line.split('|') {
                let (key, value) = field.split_once(':').unwrap();
                let value: u64 = value.parse().unwrap();
                match key {
                    "maturity_periods" => point.maturity_periods = value,
                    "founded_descendants" => point.founded_descendants = value,
                    "maturity_completed_descendants" => {
                        point.maturity_completed_descendants = value
                    }
                    "descendant_reproduction_transitions" => {
                        point.descendant_reproduction_transitions = value
                    }
                    "terminal_residual_periods" => point.terminal_residual_periods = value,
                    other => panic!("unknown policy-surface field {other}"),
                }
            }
            point
        })
        .collect()
}

#[test]
fn one_fixed_v3_physical_state_matches_the_full_policy_sensitivity_surface() {
    let cases = cases();
    let depth = depth_cases();
    let index = depth
        .iter()
        .position(|entry| entry.expected_successor_horizon == Some(4))
        .expect("frontier must contain a four-period successor case");
    let case = &cases[index];
    assert_eq!(depth[index].expected_successor_horizon, Some(4));

    let expected = expected_surface_points();
    assert_eq!(expected.len(), 4);
    let observed: Vec<_> = expected
        .iter()
        .map(|point| simulate_fixed_case_policy_surface_point(case, point.maturity_periods))
        .collect();
    assert_eq!(observed, expected);

    for pair in observed.windows(2) {
        assert!(pair[1].maturity_periods > pair[0].maturity_periods);
        assert!(pair[1].founded_descendants <= pair[0].founded_descendants);
        assert!(
            pair[1].maturity_completed_descendants <= pair[0].maturity_completed_descendants
        );
        assert!(
            pair[1].descendant_reproduction_transitions
                <= pair[0].descendant_reproduction_transitions
        );
    }
}

#[test]
fn terminal_residual_distinguishes_exact_maturity_from_partial_terminal_survival() {
    let cases = cases();
    let depth = depth_cases();
    let index = depth
        .iter()
        .position(|entry| entry.expected_successor_horizon == Some(4))
        .unwrap();
    let case = &cases[index];

    let maturity_three = simulate_fixed_case_policy_surface_point(case, 3);
    assert_eq!(maturity_three.founded_descendants, 2);
    assert_eq!(maturity_three.maturity_completed_descendants, 1);
    assert_eq!(maturity_three.terminal_residual_periods, 1);

    let maturity_four = simulate_fixed_case_policy_surface_point(case, 4);
    assert_eq!(maturity_four.founded_descendants, 1);
    assert_eq!(maturity_four.maturity_completed_descendants, 1);
    assert_eq!(maturity_four.descendant_reproduction_transitions, 0);
    assert_eq!(maturity_four.terminal_residual_periods, 4);
}
