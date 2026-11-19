// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Frozen adversarial corpus executed against the public geofusion contracts.

use std::collections::BTreeSet;

use symtropy_geofusion_contracts::{
    CircuitKind, EnergyAllocation, EnergyBalance, EnergyCarrier, EnergyStream, EnergyTerm,
    EvidenceClass, ServiceKind, SourceAllocationLedger, SourceKind, ThermalGrade,
    ThermalIsolationBoundary, ValidationError,
    superhot_well::{
        FluidPropertyProvenance, ReservoirBoundaryState, ReservoirWellheadCase,
        WellheadBoundaryState, WellheadPhase, WellheadValidationError,
    },
};

const CASES: &str = include_str!("frozen-cases.tsv");

fn rows() -> Vec<Vec<&'static str>> {
    CASES
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .skip(1)
        .map(|line| line.split('|').collect())
        .collect()
}

fn electric_stream() -> EnergyStream {
    EnergyStream {
        stream_id: "stream-electric-001".into(),
        source_asset_id: "grid-pcc-001".into(),
        destination_asset_id: "facility-load-001".into(),
        source_kind: SourceKind::GridElectricity,
        carrier: EnergyCarrier::Electricity,
        service: ServiceKind::FacilityElectricalLoad,
        power_mw: 8.0,
        duration_hours: 0.25,
        energy_mwh: 2.0,
        thermal_grade: None,
        thermal_isolation: None,
        measurement_boundary_id: "pcc-001".into(),
        evidence_class: EvidenceClass::Synthetic,
    }
}

fn geothermal_heat_stream(source_c: f64, sink_c: f64, approach_k: f64) -> EnergyStream {
    EnergyStream {
        stream_id: "stream-geo-heat-001".into(),
        source_asset_id: "geo-wellfield-001".into(),
        destination_asset_id: "process-loop-001".into(),
        source_kind: SourceKind::GeothermalPrimaryHeat,
        carrier: EnergyCarrier::Heat,
        service: ServiceKind::IndustrialProcessHeat,
        power_mw: 20.0,
        duration_hours: 1.0,
        energy_mwh: 20.0,
        thermal_grade: Some(ThermalGrade {
            source_temperature_c: source_c,
            required_sink_temperature_c: sink_c,
            minimum_approach_k: approach_k,
        }),
        thermal_isolation: Some(ThermalIsolationBoundary {
            interface_id: "hx-001".into(),
            source_circuit_id: "geo-primary-001".into(),
            source_circuit_kind: CircuitKind::GeothermalPrimary,
            receiving_circuit_id: "process-secondary-001".into(),
            receiving_circuit_kind: CircuitKind::ProcessSecondary,
            receiving_loop_isolated: true,
        }),
        measurement_boundary_id: "hx-secondary-outlet-001".into(),
        evidence_class: EvidenceClass::Synthetic,
    }
}

fn heat_allocation(id: &str, destination: &str, energy_mwh: f64) -> EnergyAllocation {
    EnergyAllocation {
        allocation_id: id.into(),
        destination_asset_id: destination.into(),
        carrier: EnergyCarrier::Heat,
        service: ServiceKind::IndustrialProcessHeat,
        energy_mwh,
    }
}

fn allocation_ledger(
    available_energy_mwh: f64,
    allocations: Vec<EnergyAllocation>,
    curtailed_energy_mwh: f64,
) -> SourceAllocationLedger {
    SourceAllocationLedger {
        source_asset_id: "geo-wellfield-001".into(),
        source_boundary_id: "wellhead-hx-outlet-001".into(),
        source_interval_id: "interval-2026-01-01T00:00Z-PT1H".into(),
        carrier: EnergyCarrier::Heat,
        available_energy_mwh,
        allocations,
        curtailed_energy_mwh,
    }
}

fn wellhead_case() -> ReservoirWellheadCase {
    ReservoirWellheadCase {
        case_id: "synthetic-superhot-well-001".into(),
        reservoir: ReservoirBoundaryState {
            well_id: "well-001".into(),
            boundary_id: "feedzone-001".into(),
            pressure_pa: 30.0e6,
            temperature_k: 748.15,
            permeability_thickness_md_m: 1_000.0,
            evidence_class: EvidenceClass::Synthetic,
        },
        wellhead: WellheadBoundaryState {
            well_id: "well-001".into(),
            boundary_id: "wellhead-001".into(),
            pressure_pa: 10.0e6,
            temperature_k: 648.15,
            mass_flow_kg_s: 100.0,
            casing_inner_diameter_m: 0.15,
            density_kg_m3: 100.0,
            specific_enthalpy_j_kg: 2.8e6,
            reference_specific_enthalpy_j_kg: 0.084e6,
            phase: WellheadPhase::SinglePhaseVapor,
            property_provenance: FluidPropertyProvenance {
                fluid_composition_id: "synthetic-water".into(),
                model_id: "synthetic-properties".into(),
                model_version: "0.1".into(),
                formulation_or_database_id: "synthetic-reference".into(),
                reference_state_id: "synthetic-h0".into(),
                output_digest: "synthetic-output-digest".into(),
                state_boundary_id: "wellhead-001".into(),
                evaluated_pressure_pa: 10.0e6,
                evaluated_temperature_k: 648.15,
                evaluated_phase: WellheadPhase::SinglePhaseVapor,
                evidence_class: EvidenceClass::Synthetic,
            },
            evidence_class: EvidenceClass::Synthetic,
        },
    }
}

#[test]
fn frozen_corpus_has_unique_ids_and_declared_dispositions() {
    let cases = rows();
    assert_eq!(cases.len(), 9);
    let mut ids = BTreeSet::new();
    for row in cases {
        assert_eq!(row.len(), 7, "each row must have seven columns: {row:?}");
        assert!(ids.insert(row[0]), "duplicate scenario ID {}", row[0]);
        assert!(matches!(row[1], "reject" | "balanced" | "unbalanced"));
        assert!(!row[2].is_empty(), "missing expectation for {}", row[0]);
    }
}

#[test]
fn every_frozen_case_executes_against_the_public_contracts() {
    for row in rows() {
        let id = row[0];
        match id {
            "stream_energy_mismatch" => {
                let mut stream = electric_stream();
                stream.energy_mwh = row[5].parse().unwrap();
                assert_eq!(
                    stream.validate(1e-9),
                    Err(ValidationError::EnergyPowerDurationMismatch),
                    "{id}"
                );
            }
            "geothermal_primary_heat_to_dt_plasma" => {
                let mut stream = geothermal_heat_stream(
                    row[3].parse().unwrap(),
                    row[4].parse().unwrap(),
                    row[5].parse().unwrap(),
                );
                stream.service = ServiceKind::DtPlasmaHeating;
                assert_eq!(
                    stream.validate(1e-9),
                    Err(ValidationError::PrimaryGeothermalHeatCannotFeedPlasma),
                    "{id}"
                );
            }
            "thermal_approach_below_minimum" => {
                let stream = geothermal_heat_stream(
                    row[3].parse().unwrap(),
                    row[4].parse().unwrap(),
                    row[5].parse().unwrap(),
                );
                assert_eq!(
                    stream.validate(1e-9),
                    Err(ValidationError::InsufficientTemperatureApproach),
                    "{id}"
                );
            }
            "first_law_thermal_to_electric" => {
                let balance = EnergyBalance {
                    inputs: vec![EnergyTerm {
                        carrier: EnergyCarrier::Heat,
                        energy_mwh: row[3].parse().unwrap(),
                    }],
                    useful_outputs: vec![EnergyTerm {
                        carrier: EnergyCarrier::Electricity,
                        energy_mwh: row[4].parse().unwrap(),
                    }],
                    rejected_outputs: vec![EnergyTerm {
                        carrier: EnergyCarrier::Heat,
                        energy_mwh: row[5].parse().unwrap(),
                    }],
                    conversion_losses: vec![EnergyTerm {
                        carrier: EnergyCarrier::Heat,
                        energy_mwh: row[6].parse().unwrap(),
                    }],
                    storage_deltas: vec![],
                };
                let result = balance.evaluate(1e-12).unwrap();
                assert!(result.within_tolerance, "{id}: {result:?}");
                assert_eq!(result.residual_mwh, 0.0, "{id}");
            }
            "first_law_overflow_adversarial" => {
                let balance = EnergyBalance {
                    inputs: vec![EnergyTerm {
                        carrier: EnergyCarrier::Electricity,
                        energy_mwh: f64::MAX,
                    }],
                    useful_outputs: vec![EnergyTerm {
                        carrier: EnergyCarrier::Electricity,
                        energy_mwh: f64::MAX,
                    }],
                    rejected_outputs: vec![EnergyTerm {
                        carrier: EnergyCarrier::Electricity,
                        energy_mwh: f64::MAX,
                    }],
                    conversion_losses: vec![],
                    storage_deltas: vec![],
                };
                let result = balance.evaluate(1e-9).unwrap();
                assert!(!result.within_tolerance, "{id}: {result:?}");
                let expected = row[2].parse::<f64>().unwrap();
                assert!((result.normalized_absolute_residual - expected).abs() < 1e-12);
            }
            "source_allocation_overallocated" => {
                let ledger = allocation_ledger(
                    row[3].parse().unwrap(),
                    vec![
                        heat_allocation("direct-heat", "process-plant", row[4].parse().unwrap()),
                        heat_allocation(
                            "power-cycle-heat",
                            "orc-evaporator",
                            row[5].parse().unwrap(),
                        ),
                    ],
                    row[6].parse().unwrap(),
                );
                let result = ledger.evaluate(1e-3).unwrap();
                assert!(!result.within_tolerance, "{id}: {result:?}");
                let expected = row[2].parse::<f64>().unwrap();
                assert!((result.residual_energy_mwh - expected).abs() < 1e-12);
            }
            "source_allocation_duplicate_edge" => {
                let ledger = allocation_ledger(
                    row[3].parse().unwrap(),
                    vec![
                        heat_allocation("duplicate-edge", "process-plant", row[4].parse().unwrap()),
                        heat_allocation("duplicate-edge", "water-plant", row[5].parse().unwrap()),
                    ],
                    row[6].parse().unwrap(),
                );
                assert_eq!(
                    ledger.evaluate(1e-9),
                    Err(ValidationError::DuplicateAllocationId),
                    "{id}"
                );
            }
            "wellhead_property_state_mismatch" => {
                let mut case = wellhead_case();
                case.wellhead.property_provenance.evaluated_pressure_pa = row[3].parse().unwrap();
                case.wellhead.pressure_pa = row[4].parse().unwrap();
                assert_eq!(
                    case.evaluate(),
                    Err(WellheadValidationError::PropertyStateMismatch),
                    "{id}"
                );
            }
            "wellhead_two_phase_no_model" => {
                let mut case = wellhead_case();
                case.wellhead.phase = WellheadPhase::TwoPhase;
                case.wellhead.property_provenance.evaluated_phase = WellheadPhase::TwoPhase;
                assert_eq!(
                    case.evaluate(),
                    Err(WellheadValidationError::TwoPhaseRequiresQualifiedHydraulics),
                    "{id}"
                );
            }
            other => panic!("unknown frozen geofusion scenario: {other}"),
        }
    }
}
