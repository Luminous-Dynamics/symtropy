// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::collections::BTreeSet;
use symtropy_core::industrial_ecology::{
    IndustrialCapability, IndustrialDependencyState, IndustrialEcology, IndustrialGovernance,
    IndustrialShock,
};

fn dependency(
    id: &str,
    governance: IndustrialGovernance,
    demand: u64,
    production: u64,
    recycling: u64,
    inventory: u64,
) -> IndustrialDependencyState {
    IndustrialDependencyState {
        dependency_id: id.into(),
        governance,
        demand_units_per_tick: demand,
        local_production_units_per_tick: production,
        recycling_units_per_tick: recycling,
        inventory_units: inventory,
    }
}

fn candidate(electronics_production: u64, electronics_inventory: u64) -> IndustrialEcology {
    IndustrialEcology::new(
        [
            dependency(
                "structural-material",
                IndustrialGovernance::Ordinary,
                100,
                80,
                20,
                0,
            ),
            dependency(
                "control-electronics",
                IndustrialGovernance::Ordinary,
                10,
                electronics_production,
                0,
                electronics_inventory,
            ),
            dependency(
                "qualified-reactor-fuel-service",
                IndustrialGovernance::SafeguardedExternal,
                1,
                0,
                0,
                30,
            ),
        ],
        vec![IndustrialCapability {
            capability_id: "persistent-ocean-infrastructure".into(),
            essential: true,
            dependency_ids: BTreeSet::from([
                "structural-material".into(),
                "control-electronics".into(),
                "qualified-reactor-fuel-service".into(),
            ]),
        }],
    )
    .unwrap()
}

#[test]
fn qualified_local_alternative_extends_successor_viability_without_erasing_safeguarded_limit() {
    // Legacy successor: electronics are externally buffered but not locally regenerated.
    let mut legacy = candidate(0, 50);
    let legacy_outcome = legacy.run_until_essential_failure(100).unwrap();
    assert_eq!(legacy_outcome.survived_ticks, 5);
    let legacy_terminal = legacy_outcome.terminal_report.unwrap();
    assert_eq!(legacy_terminal.tick, 6);
    assert_eq!(legacy_terminal.shortages.len(), 1);
    assert_eq!(legacy_terminal.shortages[0].dependency_id, "control-electronics");

    // Qualified regenerative successor: local electronics production closes that dependency.
    // The separately safeguarded reactor-service inventory remains the hard external limit.
    let mut regenerative = candidate(10, 0);
    let regenerative_outcome = regenerative.run_until_essential_failure(100).unwrap();
    assert_eq!(regenerative_outcome.survived_ticks, 30);
    let regenerative_terminal = regenerative_outcome.terminal_report.unwrap();
    assert_eq!(regenerative_terminal.tick, 31);
    assert_eq!(regenerative_terminal.shortages.len(), 1);
    assert_eq!(
        regenerative_terminal.shortages[0].dependency_id,
        "qualified-reactor-fuel-service"
    );
}

#[test]
fn losing_local_successor_production_reduces_future_viability_without_mutating_the_baseline_candidate() {
    let baseline = candidate(10, 20);
    let mut intact = baseline.clone();
    let mut shocked = baseline;

    shocked
        .apply_shock(IndustrialShock::SetLocalProduction {
            dependency_id: "control-electronics".into(),
            units_per_tick: 0,
        })
        .unwrap();

    let intact_outcome = intact.run_until_essential_failure(100).unwrap();
    let shocked_outcome = shocked.run_until_essential_failure(100).unwrap();

    assert_eq!(intact_outcome.survived_ticks, 30);
    assert_eq!(shocked_outcome.survived_ticks, 2);
    assert_eq!(
        shocked_outcome
            .terminal_report
            .unwrap()
            .shortages[0]
            .dependency_id,
        "control-electronics"
    );
}

#[test]
fn no_candidate_can_gain_local_supply_for_the_safeguarded_dependency() {
    let mut regenerative = candidate(10, 0);
    let before = regenerative
        .dependency("qualified-reactor-fuel-service")
        .unwrap()
        .clone();

    assert!(
        regenerative
            .apply_shock(IndustrialShock::SetLocalProduction {
                dependency_id: "qualified-reactor-fuel-service".into(),
                units_per_tick: 1,
            })
            .is_err()
    );
    assert_eq!(
        regenerative.dependency("qualified-reactor-fuel-service"),
        Some(&before)
    );
}
