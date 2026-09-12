include!("manta_forge_reproduction_policy_frontier_v1.rs");

const POLICY_METRICS_FIXTURE: &str =
    include_str!("../fixtures/manta-forge-generation-policy-metrics-v1.txt");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DynamicGenerationPolicyMetrics {
    founded_descendants: u64,
    maturity_completed_descendants: u64,
    descendant_reproduction_transitions: u64,
}

fn simulate_generation_policy_metrics(
    case: &FrontierCase,
    maturity_periods: u64,
) -> Option<DynamicGenerationPolicyMetrics> {
    assert!(maturity_periods > 0);
    let mut state = first_successor(case)?;
    let mut generation = 3u64;
    let mut metrics = DynamicGenerationPolicyMetrics {
        founded_descendants: 1,
        maturity_completed_descendants: 0,
        descendant_reproduction_transitions: 0,
    };

    loop {
        let mut matured = true;
        for expected_tick in 1..=maturity_periods {
            let report = state.step().unwrap();
            assert_eq!(report.tick, expected_tick);
            if !report.essential_capabilities_available {
                matured = false;
                break;
            }
        }
        if !matured {
            break;
        }
        metrics.maturity_completed_descendants += 1;

        let tooling_id = format!("forge-tooling-v{generation}");
        let reactor_id = format!("reactor-service-v{generation}");
        let structural_id = format!("structural-stock-v{generation}");
        let tooling_units = state.dependency(&tooling_id).unwrap().inventory_units;
        let reactor_units = state.dependency(&reactor_id).unwrap().inventory_units;
        let structural_units = state.dependency(&structural_id).unwrap().inventory_units;

        if tooling_units == 0 || reactor_units == 0 {
            break;
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

        metrics.founded_descendants += 1;
        metrics.descendant_reproduction_transitions += 1;
        state = successor;
        generation = next_generation;
    }

    Some(metrics)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ExpectedPolicyMetrics {
    maturity_periods: u64,
    founded_descendants: u64,
    maturity_completed_descendants: u64,
    descendant_reproduction_transitions: u64,
}

fn expected_policy_metrics() -> Vec<ExpectedPolicyMetrics> {
    POLICY_METRICS_FIXTURE
        .lines()
        .filter_map(|line| line.strip_prefix("policy="))
        .map(|line| {
            let mut maturity_periods = None;
            let mut founded_descendants = None;
            let mut maturity_completed_descendants = None;
            let mut descendant_reproduction_transitions = None;
            for field in line.split('|') {
                let (key, value) = field.split_once(':').unwrap();
                match key {
                    "maturity_periods" => maturity_periods = Some(value.parse().unwrap()),
                    "founded_descendants" => founded_descendants = Some(value.parse().unwrap()),
                    "maturity_completed_descendants" => {
                        maturity_completed_descendants = Some(value.parse().unwrap())
                    }
                    "descendant_reproduction_transitions" => {
                        descendant_reproduction_transitions = Some(value.parse().unwrap())
                    }
                    other => panic!("unknown generation policy metric field {other}"),
                }
            }
            ExpectedPolicyMetrics {
                maturity_periods: maturity_periods.unwrap(),
                founded_descendants: founded_descendants.unwrap(),
                maturity_completed_descendants: maturity_completed_descendants.unwrap(),
                descendant_reproduction_transitions: descendant_reproduction_transitions.unwrap(),
            }
        })
        .collect()
}

#[test]
fn dynamic_policy_history_separates_founded_matured_and_reproducing_generations() {
    let cases = cases();
    let expectations = expected_policy_metrics();
    assert_eq!(cases.len(), 42);
    assert_eq!(expectations.len(), 4);

    for expected in expectations {
        let mut founded = 0u64;
        let mut matured = 0u64;
        let mut transitions = 0u64;
        let mut reproductive_cases = 0u64;

        for case in &cases {
            let Some(metrics) =
                simulate_generation_policy_metrics(case, expected.maturity_periods)
            else {
                continue;
            };
            reproductive_cases += 1;
            founded += metrics.founded_descendants;
            matured += metrics.maturity_completed_descendants;
            transitions += metrics.descendant_reproduction_transitions;
            assert_eq!(
                metrics.founded_descendants,
                1 + metrics.descendant_reproduction_transitions
            );
            assert!(metrics.maturity_completed_descendants <= metrics.founded_descendants);
        }

        assert_eq!(reproductive_cases, 18);
        assert_eq!(founded, expected.founded_descendants);
        assert_eq!(matured, expected.maturity_completed_descendants);
        assert_eq!(transitions, expected.descendant_reproduction_transitions);
        assert_eq!(founded, reproductive_cases + transitions);
    }
}

#[test]
fn four_period_policy_has_mature_terminal_generations_without_reproduction() {
    let cases = cases();
    let mut founded = 0u64;
    let mut matured = 0u64;
    let mut transitions = 0u64;
    for case in &cases {
        let Some(metrics) = simulate_generation_policy_metrics(case, 4) else {
            continue;
        };
        founded += metrics.founded_descendants;
        matured += metrics.maturity_completed_descendants;
        transitions += metrics.descendant_reproduction_transitions;
    }

    assert_eq!(founded, 18);
    assert_eq!(matured, 3);
    assert_eq!(transitions, 0);
}
