use std::collections::{BTreeMap, BTreeSet};
use symtropy_core::industrial_ecology::{
    IndustrialCapability, IndustrialDependencyState, IndustrialEcology, IndustrialGovernance,
};
use symtropy_core::industrial_watch::{
    IndustrialCapabilityWatch, assess_industrial_capability_watches,
};

const FIXTURE: &str = include_str!("../fixtures/manta-forge-lineage-viability-v1.txt");

fn scalar(key: &str) -> &str {
    FIXTURE
        .lines()
        .find_map(|line| line.strip_prefix(&format!("{key}=")))
        .unwrap_or_else(|| panic!("missing fixture key {key}"))
}

fn role_capabilities() -> BTreeMap<&'static str, BTreeSet<String>> {
    FIXTURE
        .lines()
        .filter_map(|line| line.strip_prefix("role="))
        .map(|record| {
            let fields: Vec<_> = record.split('|').collect();
            assert_eq!(fields.len(), 2);
            let role = match fields[0] {
                "operation" => "operation",
                "successor_construction" => "successor_construction",
                "successor_qualification" => "successor_qualification",
                other => panic!("unexpected role {other}"),
            };
            (
                role,
                fields[1]
                    .split(',')
                    .map(str::to_owned)
                    .collect::<BTreeSet<_>>(),
            )
        })
        .collect()
}

#[test]
fn manta_forge_fixture_pins_dynamic_role_failure_ticks() {
    assert_eq!(scalar("schema"), "manta-forge-lineage-viability-v1");

    let dependencies = FIXTURE
        .lines()
        .filter_map(|line| line.strip_prefix("dependency="))
        .map(|record| {
            let fields: Vec<_> = record.split('|').collect();
            assert_eq!(fields.len(), 6);
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

    let capabilities = FIXTURE
        .lines()
        .filter_map(|line| line.strip_prefix("capability="))
        .map(|record| {
            let fields: Vec<_> = record.split('|').collect();
            assert_eq!(fields.len(), 3);
            IndustrialCapability {
                capability_id: fields[0].into(),
                essential: match fields[1] {
                    "essential" => true,
                    "optional" => false,
                    other => panic!("unexpected capability class {other}"),
                },
                dependency_ids: fields[2]
                    .split(',')
                    .map(str::to_owned)
                    .collect::<BTreeSet<_>>(),
            }
        })
        .collect::<Vec<_>>();

    let roles = role_capabilities();
    let watches = vec![
        IndustrialCapabilityWatch::new(
            "operation",
            roles["operation"].clone(),
            &capabilities,
        )
        .unwrap(),
        IndustrialCapabilityWatch::new(
            "successor_construction",
            roles["successor_construction"].clone(),
            &capabilities,
        )
        .unwrap(),
        IndustrialCapabilityWatch::new(
            "successor_qualification",
            roles["successor_qualification"].clone(),
            &capabilities,
        )
        .unwrap(),
    ];

    let mut ecology = IndustrialEcology::new(dependencies, capabilities).unwrap();
    let mut first_failure_tick = BTreeMap::<String, u64>::new();
    let mut qualification_failure_report = None;

    while first_failure_tick.len() < watches.len() {
        let tick = ecology.step().unwrap();
        for report in assess_industrial_capability_watches(&tick, &watches) {
            if !report.available {
                first_failure_tick
                    .entry(report.watch_id.clone())
                    .or_insert(report.tick);
                if report.watch_id == "successor_qualification"
                    && qualification_failure_report.is_none()
                {
                    qualification_failure_report = Some(tick.clone());
                }
            }
        }
        assert!(tick.tick <= 101, "fixture watch did not fail within bounded horizon");
    }

    assert_eq!(
        first_failure_tick["operation"],
        scalar("expected_operation_failure_tick").parse().unwrap()
    );
    assert_eq!(
        first_failure_tick["successor_construction"],
        scalar("expected_construction_failure_tick").parse().unwrap()
    );
    assert_eq!(
        first_failure_tick["successor_qualification"],
        scalar("expected_qualification_failure_tick")
            .parse()
            .unwrap()
    );

    let operation_horizon = first_failure_tick["operation"] - 1;
    let construction_horizon = first_failure_tick["successor_construction"] - 1;
    let qualification_horizon = first_failure_tick["successor_qualification"] - 1;
    assert_eq!(
        operation_horizon,
        scalar("expected_operation_horizon_periods").parse().unwrap()
    );
    assert_eq!(
        construction_horizon,
        scalar("expected_construction_horizon_periods")
            .parse()
            .unwrap()
    );
    assert_eq!(
        qualification_horizon,
        scalar("expected_qualification_horizon_periods")
            .parse()
            .unwrap()
    );
    assert_eq!(
        operation_horizon
            .min(construction_horizon)
            .min(qualification_horizon),
        scalar("expected_regenerative_viability_horizon_periods")
            .parse()
            .unwrap()
    );

    let terminal = qualification_failure_report.expect("expected qualification failure");
    assert!(terminal.shortages.iter().any(|shortage| {
        shortage.dependency_id == scalar("expected_limiting_dependency")
    }));
    assert_eq!(scalar("expected_limiting_role"), "successor_qualification");

    // Symthaea consumes these exact fixture bytes and independently derives the
    // static role-separated horizons rather than trusting this dynamic simulator.
    assert_eq!(scalar("expected_qualification_horizon_periods"), "30");
}
