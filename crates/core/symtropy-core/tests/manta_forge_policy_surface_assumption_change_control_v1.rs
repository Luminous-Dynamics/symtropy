include!("manta_forge_policy_surface_history_invariance_v1.rs");

fn simulate_policy_surface_after_post_handoff_reserve_loss(
    case: &FrontierCase,
    maturity_periods: u64,
    lost_units: u64,
) -> DynamicPolicySurfacePoint {
    assert!(maturity_periods > 0);
    assert!(lost_units > 0);

    let mut state = first_successor(case).expect("selected control case must found v3");
    let tooling_before = state
        .dependency("forge-tooling-v3")
        .unwrap()
        .inventory_units;
    let reactor_before = state
        .dependency("reactor-service-v3")
        .unwrap()
        .inventory_units;
    assert!(tooling_before >= lost_units);
    assert!(reactor_before >= lost_units);

    // Change the post-handoff physical assumptions without changing the already
    // established v2 -> v3 lineage history. Losing one qualified unit from both
    // finite bootstrap reserves should reduce the carried conservative runway by
    // one complete period in this controlled ecology.
    state
        .apply_shock(IndustrialShock::LoseInventory {
            dependency_id: "forge-tooling-v3".into(),
            units: lost_units,
        })
        .unwrap();
    state
        .apply_shock(IndustrialShock::LoseInventory {
            dependency_id: "reactor-service-v3".into(),
            units: lost_units,
        })
        .unwrap();

    assert_eq!(
        state.dependency("forge-tooling-v3").unwrap().inventory_units,
        tooling_before - lost_units
    );
    assert_eq!(
        state
            .dependency("reactor-service-v3")
            .unwrap()
            .inventory_units,
        reactor_before - lost_units
    );
    // Successful shocks invalidate the pre-shock completed-tick observation.
    assert!(state.last_report().is_none());

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

#[test]
fn post_handoff_reserve_loss_moves_h4_surface_to_the_h3_equivalence_class() {
    let cases = cases();
    let depth = depth_cases();
    let maturities = [1u64, 2, 3, 4];

    let h4_index = depth
        .iter()
        .position(|entry| entry.expected_successor_horizon == Some(4))
        .expect("frontier must contain an H=4 case");
    let h3_index = depth
        .iter()
        .position(|entry| entry.expected_successor_horizon == Some(3))
        .expect("frontier must contain an H=3 case");

    let baseline_h4: Vec<_> = maturities
        .iter()
        .map(|maturity| simulate_fixed_case_policy_surface_point(&cases[h4_index], *maturity))
        .collect();
    let baseline_h3: Vec<_> = maturities
        .iter()
        .map(|maturity| simulate_fixed_case_policy_surface_point(&cases[h3_index], *maturity))
        .collect();
    let shocked_h4: Vec<_> = maturities
        .iter()
        .map(|maturity| {
            simulate_policy_surface_after_post_handoff_reserve_loss(
                &cases[h4_index],
                *maturity,
                1,
            )
        })
        .collect();

    assert_ne!(baseline_h4, baseline_h3);
    assert_ne!(shocked_h4, baseline_h4);
    assert_eq!(shocked_h4, baseline_h3);
}

#[test]
fn changed_assumptions_must_not_be_reported_as_unchanged_h4_policy_evidence() {
    let cases = cases();
    let depth = depth_cases();
    let h4_index = depth
        .iter()
        .position(|entry| entry.expected_successor_horizon == Some(4))
        .unwrap();

    for maturity in [1u64, 2, 3, 4] {
        let baseline = simulate_fixed_case_policy_surface_point(&cases[h4_index], maturity);
        let shocked = simulate_policy_surface_after_post_handoff_reserve_loss(
            &cases[h4_index],
            maturity,
            1,
        );
        assert!(shocked.founded_descendants <= baseline.founded_descendants);
        assert!(
            shocked.maturity_completed_descendants
                <= baseline.maturity_completed_descendants
        );
        assert!(
            shocked.descendant_reproduction_transitions
                <= baseline.descendant_reproduction_transitions
        );
    }
}
