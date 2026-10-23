use std::collections::BTreeSet;
use symtropy_core::industrial_ecology::{
    DependencyShortage, IndustrialCapability, IndustrialDependencyState, IndustrialEcology,
    IndustrialGovernance,
};

const FIXTURE: &str = include_str!("../fixtures/manta-forge-regenerative-v1.txt");

fn scalar(key: &str) -> &str {
    FIXTURE
        .lines()
        .find_map(|line| line.strip_prefix(&format!("{key}=")))
        .unwrap_or_else(|| panic!("missing fixture key {key}"))
}

#[test]
fn manta_forge_fixture_pins_dynamic_viability_theorem() {
    assert_eq!(scalar("schema"), "manta-forge-regenerative-v1");

    let dependencies = FIXTURE
        .lines()
        .filter_map(|line| line.strip_prefix("dependency="))
        .map(|record| {
            let fields: Vec<_> = record.split('|').collect();
            assert_eq!(fields.len(), 7);
            IndustrialDependencyState {
                dependency_id: fields[0].into(),
                governance: match fields[1] {
                    "ordinary" => IndustrialGovernance::Ordinary,
                    "safeguarded_external" => IndustrialGovernance::SafeguardedExternal,
                    other => panic!("unexpected governance {other}"),
                },
                demand_units_per_tick: fields[2].parse().unwrap(),
                local_production_units_per_tick: fields[3].parse().unwrap(),
                recycling_units_per_tick: fields[4].parse().unwrap(),
                inventory_units: fields[5].parse().unwrap(),
            }
        })
        .collect::<Vec<_>>();

    let capability_record = scalar("capability");
    let capability_fields: Vec<_> = capability_record.split('|').collect();
    assert_eq!(capability_fields.len(), 3);
    let capability = IndustrialCapability {
        capability_id: capability_fields[0].into(),
        essential: match capability_fields[1] {
            "essential" => true,
            "optional" => false,
            other => panic!("unexpected capability class {other}"),
        },
        dependency_ids: capability_fields[2]
            .split(',')
            .map(str::to_owned)
            .collect::<BTreeSet<_>>(),
    };

    let mut ecology = IndustrialEcology::new(dependencies, vec![capability]).unwrap();
    let outcome = ecology.run_until_essential_failure(100).unwrap();

    assert_eq!(
        outcome.survived_ticks,
        scalar("expected_dynamic_survived_ticks").parse().unwrap()
    );
    let terminal = outcome.terminal_report.expect("expected essential failure");
    assert_eq!(terminal.tick, scalar("expected_dynamic_failure_tick").parse().unwrap());
    assert_eq!(
        terminal.shortages,
        vec![DependencyShortage {
            dependency_id: scalar("expected_limiting_dependency").into(),
            missing_units: 1,
        }]
    );
    assert!(!terminal.essential_capabilities_available);

    // Symthaea independently consumes these same fixture bytes and derives the
    // static closure metrics rather than trusting this dynamic simulator.
    assert_eq!(scalar("expected_mass_closure_basis_points"), "9999");
    assert_eq!(scalar("expected_static_horizon_periods"), "30");
}
