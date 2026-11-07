//! Bind complete aircraft mass properties to the existing steady-level mission screen.
//!
//! The existing `Case` model accepts a separated airframe mass and payload mass.
//! A complete itemized aircraft mass ledger already contains both. This adapter
//! recomputes mass properties from the ledger, reconciles the Payload category,
//! derives the non-payload subtotal, and invokes the mission screen without
//! double-counting or accepting a caller-supplied mass-properties report.
//!
//! This is a screening adapter only. It does not add climb/takeoff/landing,
//! fuel/energy chronology, trim, structural loads, or flightworthiness semantics.

use crate::mass_evidence::{
    AircraftLoadingState, LoadingStateKind, MassEvidenceLedger, MassPropertiesStateReport,
    StateEvidenceStatus,
};
use crate::mass_properties::{
    MASS_PROPERTIES_SCHEMA_VERSION, MassItemCategory, MassLedger, MassPropertiesReport,
    MassPropertiesStatus,
};
use crate::{Case, ResultRow};

const PAYLOAD_MASS_RELATIVE_TOLERANCE: f64 = 1.0e-9;
const MASS_CLOSURE_RELATIVE_TOLERANCE: f64 = 1.0e-12;

/// Result of evaluating a mission case with an exact-configuration mass report.
///
/// The wrapper preserves the configuration identity and full mass-properties
/// report beside the existing mission-energy result. The underlying mission
/// metrics remain `screening_only`.
#[derive(Debug, Clone, PartialEq)]
pub struct MassBoundMissionResult {
    pub schema_version: &'static str,
    pub evidence_level: &'static str,
    pub configuration_sha256: String,
    pub mission_case_id: String,
    pub mass_properties: MassPropertiesReport,
    pub result: ResultRow,
}

/// Result of evaluating the steady-level cruise screen using an evidence-qualified
/// takeoff loading state. The loading-state report and digest remain attached.
#[derive(Debug, Clone, PartialEq)]
pub struct TakeoffStateBoundMissionResult {
    pub schema_version: &'static str,
    pub evidence_level: &'static str,
    pub mission_case_id: String,
    pub configuration_sha256: String,
    pub loading_state_sha256: String,
    pub loading_state_report: MassPropertiesStateReport,
    pub mission_result: MassBoundMissionResult,
}

/// Evaluate the existing cruise screen using mass from an itemized, complete
/// mass-properties report.
///
/// Preconditions:
/// - the requested aircraft configuration matches the mass report identity;
/// - the mass report is complete and contains finite CG/inertia outputs;
/// - payload mass in the mass ledger's Payload category matches the mission case;
/// - the remaining mass after subtracting payload is positive.
///
/// Unknown or mismatched inputs return an error. This function never substitutes
/// a partial known-mass subtotal for total aircraft mass.
pub fn evaluate_case_with_mass_ledger(
    case: &Case,
    mass_ledger: &MassLedger,
    expected_configuration_sha256: &str,
) -> Result<MassBoundMissionResult, String> {
    let report = mass_ledger.evaluate_for_configuration(expected_configuration_sha256)?;
    evaluate_case_with_mass_properties(case, report, expected_configuration_sha256)
}

/// Evaluate only from an exact Takeoff loading state. The existing mission
/// screen has no time/phase chronology, so it must not silently use a landing
/// state or an intermediate fuel state as a whole-flight mass assumption.
pub fn evaluate_takeoff_case_with_loading_state(
    case: &Case,
    evidence_ledger: &MassEvidenceLedger,
    loading_state: &AircraftLoadingState,
    expected_configuration_sha256: &str,
) -> Result<TakeoffStateBoundMissionResult, String> {
    if loading_state.state_kind != LoadingStateKind::Takeoff {
        return Err("mission cruise screen requires an explicit takeoff loading state".to_owned());
    }
    let state_report =
        evidence_ledger.evaluate_state(loading_state, expected_configuration_sha256)?;
    if state_report.status != StateEvidenceStatus::CompleteForScreening {
        return Err(
            "loading-state evidence is not complete for screening; mission evaluation withheld"
                .to_owned(),
        );
    }
    let mission_result = evaluate_case_with_mass_properties(
        case,
        state_report.mass_properties.clone(),
        expected_configuration_sha256,
    )?;
    Ok(TakeoffStateBoundMissionResult {
        schema_version: "takeoff-state-bound-mission-screen/0.1.0",
        evidence_level: "screening_only",
        mission_case_id: case.case_id.clone(),
        configuration_sha256: expected_configuration_sha256.to_owned(),
        loading_state_sha256: state_report.state_sha256.clone(),
        loading_state_report: state_report,
        mission_result,
    })
}

fn evaluate_case_with_mass_properties(
    case: &Case,
    mass_properties: MassPropertiesReport,
    expected_configuration_sha256: &str,
) -> Result<MassBoundMissionResult, String> {
    validate_sha256(
        expected_configuration_sha256,
        "expected_configuration_sha256",
    )?;
    if mass_properties.configuration_sha256 != expected_configuration_sha256 {
        return Err("stale mass properties: configuration SHA-256 mismatch".to_owned());
    }
    if mass_properties.status != MassPropertiesStatus::Complete {
        return Err("mass properties are not complete; mission evaluation withheld".to_owned());
    }
    validate_mass_properties_report(&mass_properties)?;
    if !mass_properties.unknown_mass_item_ids.is_empty()
        || !mass_properties.unknown_geometry_item_ids.is_empty()
    {
        return Err("mass properties contain unresolved mass or geometry items".to_owned());
    }

    let total_mass_kg = mass_properties
        .total_mass_kg
        .ok_or_else(|| "complete mass-properties report has no total mass".to_owned())?;
    let payload_mass_kg = mass_properties
        .known_mass_by_category_kg
        .iter()
        .filter(|(category, _)| *category == MassItemCategory::Payload)
        .map(|(_, mass)| *mass)
        .sum::<f64>();

    if !payload_mass_kg.is_finite() || payload_mass_kg <= 0.0 {
        return Err("mass ledger must contain a finite positive payload subtotal".to_owned());
    }

    let payload_tolerance = PAYLOAD_MASS_RELATIVE_TOLERANCE
        * case
            .payload_mass_kg
            .abs()
            .max(payload_mass_kg.abs())
            .max(1.0);
    if (case.payload_mass_kg - payload_mass_kg).abs() > payload_tolerance {
        return Err(format!(
            "mission payload mismatch: Case has {:.12} kg but mass ledger Payload category has {:.12} kg",
            case.payload_mass_kg, payload_mass_kg
        ));
    }

    let mass_residual = (total_mass_kg - mass_properties.known_mass_subtotal_kg).abs();
    let mass_tolerance = MASS_CLOSURE_RELATIVE_TOLERANCE
        * total_mass_kg
            .abs()
            .max(mass_properties.known_mass_subtotal_kg.abs())
            .max(1.0);
    if mass_residual > mass_tolerance {
        return Err("complete mass report total does not match known mass subtotal".to_owned());
    }

    let airframe_mass_kg = total_mass_kg - payload_mass_kg;
    if !airframe_mass_kg.is_finite() || airframe_mass_kg <= 0.0 {
        return Err("derived non-payload aircraft mass must be finite and positive".to_owned());
    }

    let mut bound_case = case.clone();
    bound_case.airframe_mass_kg = airframe_mass_kg;
    bound_case.validate()?;
    let result = bound_case.evaluate()?;

    Ok(MassBoundMissionResult {
        schema_version: "mass-bound-mission-screen/0.1.0",
        evidence_level: "screening_only",
        configuration_sha256: expected_configuration_sha256.to_owned(),
        mission_case_id: case.case_id.clone(),
        mass_properties,
        result,
    })
}

fn validate_mass_properties_report(report: &MassPropertiesReport) -> Result<(), String> {
    if report.schema_version != MASS_PROPERTIES_SCHEMA_VERSION {
        return Err("unsupported mass-properties schema version".to_owned());
    }
    if report.evidence_level != "screening_only" {
        return Err("mass-properties report has an unsupported evidence level".to_owned());
    }
    validate_sha256(
        &report.configuration_sha256,
        "mass_properties.configuration_sha256",
    )?;

    let total = report
        .total_mass_kg
        .ok_or_else(|| "mass-properties report has no complete total mass".to_owned())?;
    if !total.is_finite() || total <= 0.0 || !report.known_mass_subtotal_kg.is_finite() {
        return Err("mass-properties total/subtotal must be finite and positive".to_owned());
    }

    let cg = report
        .center_of_gravity_m
        .ok_or_else(|| "mass-properties report has no complete centre of gravity".to_owned())?;
    if cg.iter().any(|component| !component.is_finite()) {
        return Err("mass-properties centre of gravity must be finite".to_owned());
    }

    let inertia = report
        .inertia_tensor_about_cg_kg_m2
        .ok_or_else(|| "mass-properties report has no complete inertia tensor".to_owned())?;
    if inertia.iter().any(|component| !component.is_finite()) {
        return Err("mass-properties inertia tensor must be finite".to_owned());
    }

    Ok(())
}

fn validate_sha256(value: &str, field: &str) -> Result<(), String> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(format!("{field} must be 64 hexadecimal characters"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mass_evidence::{
        AccountingRole, AircraftEffectivity, EvidenceAdmission, InstallationState, LengthUnit,
        LoadingItemState, MassEvidenceClass, MassItemEvidence, MassUnit, SourceMassValue,
        SourcePosition, SourceRecord, UncertaintyRelationship,
    };
    use crate::mass_properties::{MassGeometry, MassItem, MassLedger};

    const CONFIG: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    fn point(
        id: &str,
        category: MassItemCategory,
        mass_kg: f64,
        x: f64,
        y: f64,
        z: f64,
    ) -> MassItem {
        MassItem {
            id: id.to_owned(),
            category,
            mass_kg: Some(mass_kg),
            geometry: MassGeometry::PointMass {
                position_m: [x, y, z],
            },
        }
    }

    fn complete_ledger() -> MassLedger {
        MassLedger {
            configuration_sha256: CONFIG.to_owned(),
            items: vec![
                point(
                    "left_wing",
                    MassItemCategory::PrimaryStructure,
                    50.0,
                    1.3,
                    -1.8,
                    0.4,
                ),
                point(
                    "right_wing",
                    MassItemCategory::PrimaryStructure,
                    50.0,
                    1.3,
                    1.8,
                    0.4,
                ),
                point(
                    "fuselage",
                    MassItemCategory::PrimaryStructure,
                    260.0,
                    1.6,
                    0.0,
                    0.35,
                ),
                point(
                    "propulsion",
                    MassItemCategory::Propulsion,
                    120.0,
                    2.2,
                    0.0,
                    0.6,
                ),
                point(
                    "fuel",
                    MassItemCategory::FuelAndFluids,
                    85.0,
                    2.1,
                    0.0,
                    0.25,
                ),
                point("systems", MassItemCategory::Avionics, 95.0, 1.2, 0.0, 0.45),
                point("pilot", MassItemCategory::Payload, 80.0, 2.7, -0.35, 0.55),
                point(
                    "passenger",
                    MassItemCategory::Payload,
                    80.0,
                    2.7,
                    0.35,
                    0.55,
                ),
                point("baggage", MassItemCategory::Payload, 25.0, 3.1, 0.0, 0.35),
            ],
        }
    }

    fn mission_case() -> Case {
        Case {
            case_id: "wren-2-synthetic-mission".to_owned(),
            parameter_basis: "synthetic".to_owned(),
            source_ref: "synthetic-mass-bound-mission-fixture".to_owned(),
            source_sha256: "-".to_owned(),
            airframe_mass_kg: 660.0,
            payload_mass_kg: 185.0,
            wing_area_m2: 12.0,
            aspect_ratio: 8.0,
            oswald_efficiency: 0.8,
            cd0: 0.035,
            cruise_speed_m_s: 50.0,
            air_density_kg_m3: 1.0,
            propulsive_efficiency: 0.7,
            battery_energy_wh: 1_000_000.0,
            reserve_fraction: 0.2,
            route_distance_km: 100.0,
            assumed_max_lift_coefficient: 1.8,
        }
    }

    #[test]
    fn complete_mass_ledger_flows_into_mission_screen_without_payload_double_count() {
        let ledger = complete_ledger();
        let case = mission_case();
        let bound = evaluate_case_with_mass_ledger(&case, &ledger, CONFIG).expect("bound mission");

        assert_eq!(bound.evidence_level, "screening_only");
        assert_eq!(bound.configuration_sha256, CONFIG);
        assert_eq!(bound.mission_case_id, case.case_id);
        assert!((bound.result.total_mass_kg - 845.0).abs() < 1.0e-12);
        assert!((bound.result.case.airframe_mass_kg - 660.0).abs() < 1.0e-12);
        assert!((bound.result.case.payload_mass_kg - 185.0).abs() < 1.0e-12);
        assert!(
            (bound.result.total_mass_kg - bound.mass_properties.total_mass_kg.unwrap()).abs()
                < 1.0e-12
        );
    }

    #[test]
    fn incomplete_mass_properties_withhold_mission_evaluation() {
        let mut ledger = MassLedger {
            configuration_sha256: CONFIG.to_owned(),
            items: complete_report_items(),
        };
        ledger.items.push(MassItem {
            id: "unknown_cooling".to_owned(),
            category: MassItemCategory::ThermalManagement,
            mass_kg: None,
            geometry: MassGeometry::Unknown,
        });
        let error = evaluate_case_with_mass_ledger(&mission_case(), &ledger, CONFIG)
            .expect_err("unknown mass blocks mission");
        assert!(error.contains("not complete"));
    }

    #[test]
    fn payload_mismatch_is_rejected() {
        let mut case = mission_case();
        case.payload_mass_kg = 180.0;
        let error = evaluate_case_with_mass_ledger(&case, &complete_ledger(), CONFIG)
            .expect_err("payload mismatch");
        assert!(error.contains("payload mismatch"));
    }

    #[test]
    fn stale_configuration_identity_is_rejected() {
        let other = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
        let error = evaluate_case_with_mass_ledger(&mission_case(), &complete_ledger(), other)
            .expect_err("stale configuration");
        assert!(error.contains("stale mass ledger"));
    }

    #[test]
    fn mass_ledger_missing_payload_category_is_rejected() {
        let ledger = MassLedger {
            configuration_sha256: CONFIG.to_owned(),
            items: complete_report_items()
                .into_iter()
                .filter(|item| item.category != MassItemCategory::Payload)
                .collect(),
        };
        let error = evaluate_case_with_mass_ledger(&mission_case(), &ledger, CONFIG)
            .expect_err("the mission has a payload");
        assert!(error.contains("payload subtotal"));
    }

    #[test]
    fn additional_installed_mass_changes_configuration_and_mission_demand() {
        let baseline = evaluate_case_with_mass_ledger(&mission_case(), &complete_ledger(), CONFIG)
            .expect("baseline configuration");

        let heavier_configuration =
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
        let mut heavier_ledger = complete_ledger();
        heavier_ledger.configuration_sha256 = heavier_configuration.to_owned();
        heavier_ledger.items.push(point(
            "additional_cooling_system",
            MassItemCategory::ThermalManagement,
            20.0,
            1.0,
            0.0,
            0.5,
        ));

        let heavier =
            evaluate_case_with_mass_ledger(&mission_case(), &heavier_ledger, heavier_configuration)
                .expect("new configuration with additional installed mass");

        assert!((baseline.result.total_mass_kg - 845.0).abs() < 1.0e-12);
        assert!((heavier.result.total_mass_kg - 865.0).abs() < 1.0e-12);
        assert!(
            heavier.result.required_lift_coefficient > baseline.result.required_lift_coefficient
        );
        assert!(heavier.result.route_energy_wh > baseline.result.route_energy_wh);
        assert_ne!(
            heavier.mass_properties.center_of_gravity_m,
            baseline.mass_properties.center_of_gravity_m
        );
    }

    #[test]
    fn known_mass_with_unknown_location_withholds_mission_evaluation() {
        let mut ledger = complete_ledger();
        ledger.items.push(MassItem {
            id: "cooling_location_unknown".to_owned(),
            category: MassItemCategory::ThermalManagement,
            mass_kg: Some(20.0),
            geometry: MassGeometry::Unknown,
        });
        let error = evaluate_case_with_mass_ledger(&mission_case(), &ledger, CONFIG)
            .expect_err("unknown location blocks complete mass properties");
        assert!(error.contains("not complete"));
    }

    const STATE_CONFIG: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
    const STATE_FRAME: &str = "wren2-synthetic-datum-v1";
    const STATE_SOURCE: &str = "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";

    fn state_source(reference: &str) -> SourceRecord {
        SourceRecord {
            reference: reference.to_owned(),
            revision: "fixture-rev-1".to_owned(),
            locator: "fixture/row-1".to_owned(),
            sha256: STATE_SOURCE.to_owned(),
            captured_at_utc: "2026-11-07T00:00:00Z".to_owned(),
            measurement: None,
        }
    }

    fn takeoff_evidence_ledger() -> MassEvidenceLedger {
        let entries = vec![
            (
                "airframe",
                MassItemCategory::PrimaryStructure,
                500.0,
                [1.0, 0.0, 0.0],
            ),
            (
                "fuel",
                MassItemCategory::FuelAndFluids,
                100.0,
                [4.0, 0.0, 0.0],
            ),
            (
                "module",
                MassItemCategory::Propulsion,
                40.0,
                [0.5, 0.0, 0.0],
            ),
            ("occupant", MassItemCategory::Payload, 80.0, [2.0, 0.0, 0.5]),
        ];
        let mut records = entries
            .into_iter()
            .map(|(id, category, mass_kg, position_m)| MassItemEvidence {
                item: MassItem {
                    id: id.to_owned(),
                    category,
                    mass_kg: Some(mass_kg),
                    geometry: MassGeometry::PointMass { position_m },
                },
                parent_id: None,
                accounting_role: AccountingRole::Leaf,
                included_child_ids: Vec::new(),
                evidence_class: MassEvidenceClass::SyntheticFixture,
                admission: EvidenceAdmission::AdmittedForScreening,
                source: state_source(id),
                effectivity: AircraftEffectivity {
                    aircraft_family: "Wren 2".to_owned(),
                    aircraft_model: "Wren 2 synthetic mission fixture".to_owned(),
                    configuration_sha256: STATE_CONFIG.to_owned(),
                    serial_numbers: Some(vec!["SYNTH-001".to_owned()]),
                },
                source_mass: Some(SourceMassValue {
                    value: mass_kg,
                    unit: MassUnit::Kilogram,
                }),
                source_position: Some(SourcePosition {
                    values: position_m,
                    unit: LengthUnit::Metre,
                    coordinate_frame_id: STATE_FRAME.to_owned(),
                }),
                source_inertia: None,
                standard_uncertainty_kg: Some(0.1),
                uncertainty_relationship: UncertaintyRelationship::IndependentByDeclaredModel,
            })
            .collect::<Vec<_>>();
        records.sort_by(|left, right| left.item.id.cmp(&right.item.id));
        MassEvidenceLedger {
            schema_version: crate::mass_evidence::MASS_EVIDENCE_SCHEMA_VERSION,
            configuration_sha256: STATE_CONFIG.to_owned(),
            aircraft_family: "Wren 2".to_owned(),
            aircraft_model: "Wren 2 synthetic mission fixture".to_owned(),
            serial_number: "SYNTH-001".to_owned(),
            reference_frame_id: STATE_FRAME.to_owned(),
            records,
        }
    }

    fn takeoff_state(ledger: &MassEvidenceLedger) -> AircraftLoadingState {
        let items = ledger
            .records
            .iter()
            .map(|record| LoadingItemState {
                item_id: record.item.id.clone(),
                installation: InstallationState::Installed,
                mass_kg: record.item.mass_kg,
                source_mass: record.source_mass,
                geometry: record.item.geometry.clone(),
                source_position: record.source_position.clone(),
                source_inertia: record.source_inertia,
                value_source: Some(record.source.clone()),
                installation_source: None,
            })
            .collect();
        AircraftLoadingState {
            schema_version: crate::mass_evidence::LOADING_STATE_SCHEMA_VERSION,
            state_id: "wren2-synthetic-takeoff".to_owned(),
            state_kind: LoadingStateKind::Takeoff,
            configuration_sha256: ledger.configuration_sha256.clone(),
            source_generation_sha256: ledger
                .source_generation_sha256()
                .expect("source generation"),
            aircraft_family: ledger.aircraft_family.clone(),
            aircraft_model: ledger.aircraft_model.clone(),
            serial_number: ledger.serial_number.clone(),
            reference_frame_id: ledger.reference_frame_id.clone(),
            items,
        }
    }

    fn state_mission_case() -> Case {
        let mut case = mission_case();
        case.payload_mass_kg = 80.0;
        case.airframe_mass_kg = 640.0;
        case.case_id = "wren-2-state-bound-synthetic-mission".to_owned();
        case
    }

    #[test]
    fn complete_takeoff_state_flows_into_mission_and_preserves_state_digest() {
        let ledger = takeoff_evidence_ledger();
        let state = takeoff_state(&ledger);
        let bound = evaluate_takeoff_case_with_loading_state(
            &state_mission_case(),
            &ledger,
            &state,
            STATE_CONFIG,
        )
        .expect("complete takeoff state mission");
        assert_eq!(bound.evidence_level, "screening_only");
        assert_eq!(
            bound.loading_state_report.status,
            StateEvidenceStatus::CompleteForScreening
        );
        assert_eq!(
            bound.loading_state_sha256,
            bound.loading_state_report.state_sha256
        );
        assert!((bound.mission_result.result.total_mass_kg - 720.0).abs() < 1.0e-12);
        assert!((bound.mission_result.result.case.airframe_mass_kg - 640.0).abs() < 1.0e-12);
        assert!((bound.mission_result.result.case.payload_mass_kg - 80.0).abs() < 1.0e-12);
    }

    #[test]
    fn incomplete_state_evidence_withholds_mission_evaluation() {
        let ledger = takeoff_evidence_ledger();
        let mut state = takeoff_state(&ledger);
        let index = state
            .items
            .iter()
            .position(|item| item.item_id == "fuel")
            .expect("fuel item");
        state.items[index].installation = InstallationState::Unknown;
        state.items[index].mass_kg = None;
        state.items[index].source_mass = None;
        state.items[index].geometry = MassGeometry::Unknown;
        state.items[index].source_position = None;
        state.items[index].source_inertia = None;
        state.items[index].value_source = None;
        let error = evaluate_takeoff_case_with_loading_state(
            &state_mission_case(),
            &ledger,
            &state,
            STATE_CONFIG,
        )
        .expect_err("unknown installed item blocks mission");
        assert!(error.contains("loading-state evidence is not complete"));
    }

    #[test]
    fn landing_state_is_not_silently_used_as_takeoff_cruise_mass() {
        let ledger = takeoff_evidence_ledger();
        let mut state = takeoff_state(&ledger);
        state.state_kind = LoadingStateKind::Landing;
        let error = evaluate_takeoff_case_with_loading_state(
            &state_mission_case(),
            &ledger,
            &state,
            STATE_CONFIG,
        )
        .expect_err("landing state cannot be used by steady cruise screen");
        assert!(error.contains("requires an explicit takeoff loading state"));
    }

    #[test]
    fn takeoff_loading_state_payload_mismatch_is_rejected() {
        let ledger = takeoff_evidence_ledger();
        let state = takeoff_state(&ledger);
        let mut case = state_mission_case();
        case.payload_mass_kg = 81.0;
        let error = evaluate_takeoff_case_with_loading_state(&case, &ledger, &state, STATE_CONFIG)
            .expect_err("payload mismatch");
        assert!(error.contains("mission payload mismatch"));
    }

    fn complete_report_items() -> Vec<MassItem> {
        vec![
            point(
                "left_wing",
                MassItemCategory::PrimaryStructure,
                50.0,
                1.3,
                -1.8,
                0.4,
            ),
            point(
                "right_wing",
                MassItemCategory::PrimaryStructure,
                50.0,
                1.3,
                1.8,
                0.4,
            ),
            point(
                "fuselage",
                MassItemCategory::PrimaryStructure,
                260.0,
                1.6,
                0.0,
                0.35,
            ),
            point(
                "propulsion",
                MassItemCategory::Propulsion,
                120.0,
                2.2,
                0.0,
                0.6,
            ),
            point(
                "fuel",
                MassItemCategory::FuelAndFluids,
                85.0,
                2.1,
                0.0,
                0.25,
            ),
            point("systems", MassItemCategory::Avionics, 95.0, 1.2, 0.0, 0.45),
            point("pilot", MassItemCategory::Payload, 80.0, 2.7, -0.35, 0.55),
            point(
                "passenger",
                MassItemCategory::Payload,
                80.0,
                2.7,
                0.35,
                0.55,
            ),
            point("baggage", MassItemCategory::Payload, 25.0, 3.1, 0.0, 0.35),
        ]
    }
}
