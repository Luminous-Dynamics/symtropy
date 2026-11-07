//! Bind complete aircraft mass properties to the existing steady-level mission screen.
//!
//! The existing `Case` model accepts a separated airframe mass and payload mass.
//! A complete itemized aircraft mass ledger already contains both. This adapter
//! reconciles the payload category exactly, derives the corresponding airframe
//! subtotal, and then invokes the existing mission screen without double-counting.
//!
//! This is a screening adapter only. It does not add climb/takeoff/landing,
//! fuel/energy chronology, trim, structural loads, or flightworthiness semantics.

use crate::mass_properties::{
    MASS_PROPERTIES_SCHEMA_VERSION, MassItemCategory, MassPropertiesReport, MassPropertiesStatus,
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
pub fn evaluate_case_with_mass_properties(
    case: &Case,
    mass_properties: &MassPropertiesReport,
    expected_configuration_sha256: &str,
) -> Result<MassBoundMissionResult, String> {
    validate_sha256(
        expected_configuration_sha256,
        "expected_configuration_sha256",
    )?;
    validate_mass_properties_report(mass_properties)?;

    if mass_properties.configuration_sha256 != expected_configuration_sha256 {
        return Err(
            "stale mass properties: configuration SHA-256 does not match requested aircraft"
                .to_owned(),
        );
    }

    if mass_properties.status != MassPropertiesStatus::Complete {
        return Err("mass properties are not complete; mission evaluation withheld".to_owned());
    }
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
        * total_mass_kg.abs().max(mass_properties.known_mass_subtotal_kg.abs()).max(1.0);
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
        mass_properties: mass_properties.clone(),
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
    validate_sha256(&report.configuration_sha256, "mass_properties.configuration_sha256")?;

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

    fn complete_report() -> MassPropertiesReport {
        MassLedger {
            configuration_sha256: CONFIG.to_owned(),
            items: vec![
                point("left_wing", MassItemCategory::PrimaryStructure, 50.0, 1.3, -1.8, 0.4),
                point("right_wing", MassItemCategory::PrimaryStructure, 50.0, 1.3, 1.8, 0.4),
                point("fuselage", MassItemCategory::PrimaryStructure, 260.0, 1.6, 0.0, 0.35),
                point("propulsion", MassItemCategory::Propulsion, 120.0, 2.2, 0.0, 0.6),
                point("fuel", MassItemCategory::FuelAndFluids, 85.0, 2.1, 0.0, 0.25),
                point("systems", MassItemCategory::Avionics, 95.0, 1.2, 0.0, 0.45),
                point("pilot", MassItemCategory::Payload, 80.0, 2.7, -0.35, 0.55),
                point("passenger", MassItemCategory::Payload, 80.0, 2.7, 0.35, 0.55),
                point("baggage", MassItemCategory::Payload, 25.0, 3.1, 0.0, 0.35),
            ],
        }
        .evaluate()
        .expect("complete synthetic fixture")
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
        let report = complete_report();
        let case = mission_case();
        let bound =
            evaluate_case_with_mass_properties(&case, &report, CONFIG).expect("bound mission");

        assert_eq!(bound.evidence_level, "screening_only");
        assert_eq!(bound.configuration_sha256, CONFIG);
        assert_eq!(bound.mission_case_id, case.case_id);
        assert!((bound.result.total_mass_kg - 845.0).abs() < 1.0e-12);
        assert!((bound.result.case.airframe_mass_kg - 660.0).abs() < 1.0e-12);
        assert!((bound.result.case.payload_mass_kg - 185.0).abs() < 1.0e-12);
        assert!((bound.result.total_mass_kg - report.total_mass_kg.unwrap()).abs() < 1.0e-12);
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
        let report = ledger.evaluate().expect("indeterminate report");
        let error = evaluate_case_with_mass_properties(&mission_case(), &report, CONFIG)
            .expect_err("unknown mass blocks mission");
        assert!(error.contains("not complete"));
    }

    #[test]
    fn payload_mismatch_is_rejected() {
        let mut case = mission_case();
        case.payload_mass_kg = 180.0;
        let error = evaluate_case_with_mass_properties(&case, &complete_report(), CONFIG)
            .expect_err("payload mismatch");
        assert!(error.contains("payload mismatch"));
    }

    #[test]
    fn stale_configuration_identity_is_rejected() {
        let other = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
        let error = evaluate_case_with_mass_properties(&mission_case(), &complete_report(), other)
            .expect_err("stale configuration");
        assert!(error.contains("stale mass properties"));
    }

    #[test]
    fn mass_ledger_missing_payload_category_is_rejected() {
        let mut ledger = MassLedger {
            configuration_sha256: CONFIG.to_owned(),
            items: complete_report_items()
                .into_iter()
                .filter(|item| item.category != MassItemCategory::Payload)
                .collect(),
        };
        let report = ledger.evaluate().expect("complete non-payload ledger");
        let error = evaluate_case_with_mass_properties(&mission_case(), &report, CONFIG)
            .expect_err("the mission has a payload");
        assert!(error.contains("payload subtotal"));
    }

    fn complete_report_items() -> Vec<MassItem> {
        vec![
            point("left_wing", MassItemCategory::PrimaryStructure, 50.0, 1.3, -1.8, 0.4),
            point("right_wing", MassItemCategory::PrimaryStructure, 50.0, 1.3, 1.8, 0.4),
            point("fuselage", MassItemCategory::PrimaryStructure, 260.0, 1.6, 0.0, 0.35),
            point("propulsion", MassItemCategory::Propulsion, 120.0, 2.2, 0.0, 0.6),
            point("fuel", MassItemCategory::FuelAndFluids, 85.0, 2.1, 0.0, 0.25),
            point("systems", MassItemCategory::Avionics, 95.0, 1.2, 0.0, 0.45),
            point("pilot", MassItemCategory::Payload, 80.0, 2.7, -0.35, 0.55),
            point("passenger", MassItemCategory::Payload, 80.0, 2.7, 0.35, 0.55),
            point("baggage", MassItemCategory::Payload, 25.0, 3.1, 0.0, 0.35),
        ]
    }
}
