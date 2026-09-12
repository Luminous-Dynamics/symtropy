include!("manta_forge_multigeneration_depth_v1.rs");

const POLICY_FIXTURE: &str =
    include_str!("../fixtures/manta-forge-reproduction-policy-frontier-v1.txt");

#[derive(Debug, Clone, PartialEq, Eq)]
struct ReproductionPolicyExpectation {
    maturity_periods: u64,
    depth_counts: [usize; 5],
    expected_total_handoffs: u64,
}

fn policy_expectations() -> Vec<ReproductionPolicyExpectation> {
    POLICY_FIXTURE
        .lines()
        .filter_map(|line| line.strip_prefix("policy="))
        .map(|line| {
            let mut maturity_periods = None;
            let mut depth_counts = [0usize; 5];
            let mut expected_total_handoffs = None;
            for field in line.split('|') {
                let (key, value) = field.split_once(':').unwrap();
                match key {
                    "maturity_periods" => maturity_periods = Some(value.parse().unwrap()),
                    "depth1_cases" => depth_counts[1] = value.parse().unwrap(),
                    "depth2_cases" => depth_counts[2] = value.parse().unwrap(),
                    "depth3_cases" => depth_counts[3] = value.parse().unwrap(),
                    "depth4_cases" => depth_counts[4] = value.parse().unwrap(),
                    "expected_total_handoffs" => {
                        expected_total_handoffs = Some(value.parse().unwrap())
                    }
                    other => panic!("unknown reproduction-policy field {other}"),
                }
            }
            ReproductionPolicyExpectation {
                maturity_periods: maturity_periods.unwrap(),
                depth_counts,
                expected_total_handoffs: expected_total_handoffs.unwrap(),
            }
        })
        .collect()
}

fn simulate_generation_depth_with_maturity(
    case: &FrontierCase,
    maturity_periods: u64,
) -> (Option<u64>, u64) {
    assert!(maturity_periods > 0);
    let Some(mut state) = first_successor(case) else {
        return (None, 0);
    };

    let mut generation = 3u64;
    let mut descendant_generations = 1u64;
    let mut successful_handoffs = 1u64; // v2 -> v3

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

        state = successor;
        generation = next_generation;
        descendant_generations += 1;
        successful_handoffs += 1;
    }

    (Some(descendant_generations), successful_handoffs)
}

#[test]
fn longer_descendant_maturity_strictly_reduces_generational_depth() {
    let cases = cases();
    let expectations = policy_expectations();
    assert_eq!(cases.len(), 42);
    assert_eq!(expectations.len(), 4);

    let mut prior_total_handoffs = None;
    for expectation in expectations {
        let mut observed_depth_counts = [0usize; 5];
        let mut total_handoffs = 0u64;
        let mut reproductive_cases = 0usize;

        for case in &cases {
            let (depth, handoffs) =
                simulate_generation_depth_with_maturity(case, expectation.maturity_periods);
            total_handoffs += handoffs;
            if let Some(depth) = depth {
                reproductive_cases += 1;
                observed_depth_counts[depth as usize] += 1;
                assert_eq!(handoffs, depth);
            } else {
                assert_eq!(handoffs, 0);
            }
        }

        assert_eq!(reproductive_cases, 18);
        assert_eq!(observed_depth_counts, expectation.depth_counts);
        assert_eq!(total_handoffs, expectation.expected_total_handoffs);
        if let Some(previous) = prior_total_handoffs {
            assert!(total_handoffs < previous);
        }
        prior_total_handoffs = Some(total_handoffs);
    }
}

#[test]
fn one_period_depth_equivalence_is_policy_specific_not_universal() {
    let cases = cases();
    let depth = depth_cases();

    for (case, temporal) in cases.iter().zip(&depth) {
        let (one_period_depth, _) = simulate_generation_depth_with_maturity(case, 1);
        assert_eq!(one_period_depth, temporal.expected_successor_horizon);

        let (two_period_depth, _) = simulate_generation_depth_with_maturity(case, 2);
        if temporal.expected_successor_horizon.is_some_and(|horizon| horizon >= 2) {
            assert!(two_period_depth.unwrap() <= temporal.expected_successor_horizon.unwrap());
        }
    }

    // The parent temporal runway remains 6/5/4/3 across depths 1..4, while the
    // two-period policy collapses the same 18 reproductive cases to 11 at depth 1
    // and 7 at depth 2. Equality therefore belongs to the bound one-period policy,
    // not to regenerative-lineage semantics in general.
}
