use std::collections::{BTreeMap, BTreeSet};
use symtropy_core::industrial_ecology::{
    IndustrialCapability, IndustrialDependencyState, IndustrialGovernance,
};
use symtropy_core::industrial_epoch::{
    IndustrialEpochFlowModel, IndustrialEpochHandoffPlan, IndustrialEpochSpec,
    IndustrialEpochState, IndustrialExternalInventoryAdmission,
    IndustrialSourceInventoryDisposition,
};

const FIXTURE: &str = include_str!("../fixtures/manta-forge-epoch-handoff-v1.txt");

fn scalar(key: &str) -> &str {
    FIXTURE
        .lines()
        .find_map(|line| line.strip_prefix(&format!("{key}=")))
        .unwrap_or_else(|| panic!("missing fixture key {key}"))
}

fn governance(value: &str) -> IndustrialGovernance {
    match value {
        "ordinary" => IndustrialGovernance::Ordinary,
        "safeguarded_external" => IndustrialGovernance::SafeguardedExternal,
        other => panic!("unexpected governance {other}"),
    }
}

fn epoch_record(key: &str) -> (&str, &str) {
    let fields: Vec<_> = scalar(key).split('|').collect();
    assert_eq!(fields.len(), 2);
    (fields[0], fields[1])
}

fn capability_record(key: &str) -> IndustrialCapability {
    let fields: Vec<_> = scalar(key).split('|').collect();
    assert_eq!(fields.len(), 2);
    IndustrialCapability {
        capability_id: fields[0].into(),
        essential: true,
        dependency_ids: fields[1].split(',').map(str::to_owned).collect(),
    }
}

#[test]
fn shared_epoch_handoff_fixture_pins_conservation_and_successor_inventory() {
    assert_eq!(scalar("schema"), "manta-forge-epoch-handoff-v1");
    let (source_epoch_id, source_epoch_binding) = epoch_record("source_epoch");
    let (successor_epoch_id, successor_epoch_binding) = epoch_record("successor_epoch");

    let source_dependencies = FIXTURE
        .lines()
        .filter_map(|line| line.strip_prefix("source_dependency="))
        .map(|record| {
            let fields: Vec<_> = record.split('|').collect();
            assert_eq!(fields.len(), 4);
            IndustrialDependencyState {
                dependency_id: fields[0].into(),
                governance: governance(fields[1]),
                demand_units_per_tick: 1,
                local_production_units_per_tick: 0,
                recycling_units_per_tick: 0,
                inventory_units: fields[3].parse().unwrap(),
            }
        })
        .collect::<Vec<_>>();

    let successor_dependencies = FIXTURE
        .lines()
        .filter_map(|line| line.strip_prefix("successor_dependency="))
        .map(|record| {
            let fields: Vec<_> = record.split('|').collect();
            assert_eq!(fields.len(), 3);
            IndustrialDependencyState {
                dependency_id: fields[0].into(),
                governance: governance(fields[1]),
                demand_units_per_tick: 1,
                local_production_units_per_tick: 0,
                recycling_units_per_tick: 0,
                inventory_units: 0,
            }
        })
        .collect::<Vec<_>>();

    let source_spec = IndustrialEpochSpec {
        epoch_id: source_epoch_id.into(),
        evidence_binding: source_epoch_binding.into(),
        dependencies: source_dependencies,
        capabilities: vec![capability_record("source_capability")],
        flow_model: IndustrialEpochFlowModel::LegacyUngated,
    };
    let successor_spec = IndustrialEpochSpec {
        epoch_id: successor_epoch_id.into(),
        evidence_binding: successor_epoch_binding.into(),
        dependencies: successor_dependencies,
        capabilities: vec![capability_record("successor_capability")],
        flow_model: IndustrialEpochFlowModel::LegacyUngated,
    };

    let handoff_fields: Vec<_> = scalar("handoff").split('|').collect();
    assert_eq!(handoff_fields.len(), 3);
    let source_inventory_dispositions = FIXTURE
        .lines()
        .filter_map(|line| line.strip_prefix("transfer="))
        .map(|record| {
            let fields: Vec<_> = record.split('|').collect();
            assert_eq!(fields.len(), 6);
            IndustrialSourceInventoryDisposition {
                source_dependency_id: fields[0].into(),
                successor_dependency_id: Some(fields[1].into()),
                transferred_units: fields[2].parse().unwrap(),
                retired_units: fields[3].parse().unwrap(),
            }
        })
        .collect();
    let external_inventory_admissions = FIXTURE
        .lines()
        .filter_map(|line| line.strip_prefix("external="))
        .map(|record| {
            let fields: Vec<_> = record.split('|').collect();
            assert_eq!(fields.len(), 5);
            IndustrialExternalInventoryAdmission {
                successor_dependency_id: fields[0].into(),
                units: fields[1].parse().unwrap(),
                evidence_binding: fields[2].into(),
            }
        })
        .collect();

    let plan = IndustrialEpochHandoffPlan {
        handoff_id: handoff_fields[0].into(),
        evidence_binding: handoff_fields[1].into(),
        source_epoch_id: source_epoch_id.into(),
        source_epoch_evidence_binding: source_epoch_binding.into(),
        successor_epoch_id: successor_epoch_id.into(),
        successor_epoch_evidence_binding: successor_epoch_binding.into(),
        source_inventory_dispositions,
        external_inventory_admissions,
    };

    let mut source = IndustrialEpochState::from_spec(source_spec).unwrap();
    let ticks: u64 = scalar("handoff_after_ticks").parse().unwrap();
    for _ in 0..ticks {
        source.step().unwrap();
    }
    let (successor, receipt) = source.handoff_to(successor_spec, plan).unwrap();

    assert_eq!(
        receipt.source_final_tick,
        scalar("expected_source_final_tick").parse().unwrap()
    );
    let expected: BTreeMap<String, u64> = FIXTURE
        .lines()
        .filter_map(|line| line.strip_prefix("expected_successor_inventory="))
        .map(|record| {
            let fields: Vec<_> = record.split('|').collect();
            assert_eq!(fields.len(), 2);
            (fields[0].to_owned(), fields[1].parse().unwrap())
        })
        .collect();
    let actual: BTreeMap<String, u64> = expected
        .keys()
        .map(|dependency_id| {
            (
                dependency_id.clone(),
                successor.dependency(dependency_id).unwrap().inventory_units,
            )
        })
        .collect();
    assert_eq!(actual, expected);

    // Symthaea independently consumes the exact same fixture bytes and validates
    // semantic transfer qualification rather than trusting this accounting engine.
    assert_eq!(scalar("expected_qualified_transfer_count"), "3");
    assert_eq!(scalar("expected_safeguarded_transfer_count"), "1");
    assert_eq!(scalar("expected_external_admission_count"), "1");
}
