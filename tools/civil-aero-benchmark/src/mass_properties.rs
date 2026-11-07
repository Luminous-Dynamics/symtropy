//! Exact-configuration mass properties for conceptual civil-aircraft screening.
//!
//! This module computes mass, centre of gravity and a symmetric inertia tensor from
//! itemized installed-mass records. It is deliberately narrower than an aircraft
//! loads or weight-and-balance approval system:
//!
//! - unknown mass or location is retained as unresolved, never replaced with zero;
//! - the caller must bind the ledger to a 64-character configuration SHA-256;
//! - a configuration mismatch rejects reuse of stale mass properties;
//! - point-mass and rigid-body entries are supported, with the parallel-axis theorem;
//! - the output is always `screening_only` and is not an approved weight-and-balance
//!   schedule, structural inertia model, flight envelope or airworthiness finding.
//!
//! Tensor tuple order is `[Ixx, Iyy, Izz, Ixy, Ixz, Iyz]`. The off-diagonal
//! entries are the actual symmetric matrix entries (normally negative products
//! of inertia for positive coordinate products), not unsigned products.

use std::collections::{BTreeMap, HashSet};

pub const MASS_PROPERTIES_SCHEMA_VERSION: &str = "aircraft-mass-properties/0.1.0";

/// Installed mass accounting category. These labels aid reconciliation; they
/// do not prescribe a regulatory empty-weight convention.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MassItemCategory {
    PrimaryStructure,
    SecondaryStructure,
    Propulsion,
    EnergyStorage,
    ElectricalDistribution,
    ThermalManagement,
    FlightControls,
    Avionics,
    LandingGear,
    InteriorAndFurnishings,
    SafetyEquipment,
    FuelAndFluids,
    Payload,
    Other,
}

/// Geometric representation of one installed mass item.
#[derive(Debug, Clone, PartialEq)]
pub enum MassGeometry {
    /// Item represented as a point mass at the declared aircraft-frame position.
    PointMass { position_m: [f64; 3] },
    /// Rigid body represented by its own centre of mass and inertia tensor,
    /// both expressed in the aircraft axes. The tensor is about the item's CG.
    RigidBody {
        center_of_mass_m: [f64; 3],
        inertia_tensor_about_center_kg_m2: [f64; 6],
    },
    /// Location/distribution is not yet known. This prevents CG/inertia closure.
    Unknown,
}

/// One row in the installed-mass ledger.
///
/// `mass_kg = None` means the mass is unknown. It is not a zero-mass item.
/// Geometry can independently be unknown even when the mass itself is known.
#[derive(Debug, Clone, PartialEq)]
pub struct MassItem {
    pub id: String,
    pub category: MassItemCategory,
    pub mass_kg: Option<f64>,
    pub geometry: MassGeometry,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MassLedger {
    /// Exact configuration identity supplied by the upstream configuration manifest.
    /// This module checks syntax and equality; it does not authenticate the manifest.
    pub configuration_sha256: String,
    pub items: Vec<MassItem>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MassPropertiesStatus {
    Complete,
    Indeterminate,
}

/// Deterministic report. Partial subtotal is diagnostic only; complete aircraft
/// mass/CG/inertia values are present only when all required inputs are known.
#[derive(Debug, Clone, PartialEq)]
pub struct MassPropertiesReport {
    pub schema_version: &'static str,
    pub evidence_level: &'static str,
    pub configuration_sha256: String,
    pub status: MassPropertiesStatus,
    /// Complete installed mass; None when at least one item mass is unknown.
    pub total_mass_kg: Option<f64>,
    /// Sum of known item masses. Never substitute this for total mass when
    /// `unknown_mass_item_ids` is non-empty.
    pub known_mass_subtotal_kg: f64,
    /// Complete aircraft-frame CG; None when any mass or location is unknown.
    pub center_of_gravity_m: Option<[f64; 3]>,
    /// Complete inertia tensor about total CG; None when closure is incomplete.
    pub inertia_tensor_about_cg_kg_m2: Option<[f64; 6]>,
    /// Known subtotal by category, in enum order.
    pub known_mass_by_category_kg: Vec<(MassItemCategory, f64)>,
    pub unknown_mass_item_ids: Vec<String>,
    pub unknown_geometry_item_ids: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
struct ResolvedMassItem {
    mass_kg: f64,
    position_m: [f64; 3],
    local_inertia_kg_m2: [f64; 6],
}

impl MassLedger {
    /// Recompute mass properties from the ledger, using no caller-supplied CG
    /// or inertia values. Missing material inputs return an Indeterminate report.
    pub fn evaluate(&self) -> Result<MassPropertiesReport, String> {
        validate_sha256(&self.configuration_sha256, "configuration_sha256")?;
        if self.items.is_empty() {
            return Err("mass ledger must contain at least one item".to_owned());
        }

        let mut seen = HashSet::new();
        let mut unknown_mass_item_ids = Vec::new();
        let mut unknown_geometry_item_ids = Vec::new();
        let mut known_mass_subtotal_kg = 0.0;
        let mut weighted_position = [0.0_f64; 3];
        let mut category_mass = BTreeMap::<MassItemCategory, f64>::new();
        let mut known = Vec::<ResolvedMassItem>::new();

        for item in &self.items {
            if item.id.trim().is_empty() {
                return Err("mass item id must not be empty".to_owned());
            }
            if !seen.insert(item.id.as_str()) {
                return Err(format!("duplicate mass item id: {}", item.id));
            }

            let Some(mass_kg) = item.mass_kg else {
                unknown_mass_item_ids.push(item.id.clone());
                if matches!(&item.geometry, MassGeometry::Unknown) {
                    unknown_geometry_item_ids.push(item.id.clone());
                }
                continue;
            };
            if !mass_kg.is_finite() || mass_kg <= 0.0 {
                return Err(format!(
                    "mass item {}: mass_kg must be finite and > 0",
                    item.id
                ));
            }

            known_mass_subtotal_kg += mass_kg;
            *category_mass.entry(item.category).or_insert(0.0) += mass_kg;

            let (position_m, local_inertia) = match &item.geometry {
                MassGeometry::PointMass { position_m } => {
                    validate_vector(position_m, &item.id)?;
                    (*position_m, [0.0; 6])
                }
                MassGeometry::RigidBody {
                    center_of_mass_m,
                    inertia_tensor_about_center_kg_m2,
                } => {
                    validate_vector(center_of_mass_m, &item.id)?;
                    validate_inertia_tensor(inertia_tensor_about_center_kg_m2, &item.id)?;
                    (*center_of_mass_m, *inertia_tensor_about_center_kg_m2)
                }
                MassGeometry::Unknown => {
                    unknown_geometry_item_ids.push(item.id.clone());
                    continue;
                }
            };

            for (weighted, coordinate) in weighted_position.iter_mut().zip(position_m) {
                *weighted += mass_kg * coordinate;
            }
            known.push(ResolvedMassItem { mass_kg, position_m, local_inertia_kg_m2: local_inertia });
        }

        unknown_mass_item_ids.sort();
        unknown_geometry_item_ids.sort();

        if !known_mass_subtotal_kg.is_finite() {
            return Err("mass ledger subtotal overflowed to a non-finite value".to_owned());
        }

        let total_mass_kg = if unknown_mass_item_ids.is_empty() {
            Some(known_mass_subtotal_kg)
        } else {
            None
        };

        let complete = unknown_mass_item_ids.is_empty()
            && unknown_geometry_item_ids.is_empty()
            && known.len() == self.items.len();

        let (center_of_gravity_m, inertia_tensor_about_cg_kg_m2) = if complete {
            let total = total_mass_kg.expect("complete ledger has total mass");
            let center = [
                weighted_position[0] / total,
                weighted_position[1] / total,
                weighted_position[2] / total,
            ];

            let mut inertia = [0.0_f64; 6];
            for item in &known {
                let dx = item.position_m[0] - center[0];
                let dy = item.position_m[1] - center[1];
                let dz = item.position_m[2] - center[2];
                let mass_kg = item.mass_kg;
                let local_inertia = item.local_inertia_kg_m2;

                inertia[0] += local_inertia[0] + mass_kg * (dy * dy + dz * dz);
                inertia[1] += local_inertia[1] + mass_kg * (dx * dx + dz * dz);
                inertia[2] += local_inertia[2] + mass_kg * (dx * dx + dy * dy);
                inertia[3] += local_inertia[3] - mass_kg * dx * dy;
                inertia[4] += local_inertia[4] - mass_kg * dx * dz;
                inertia[5] += local_inertia[5] - mass_kg * dy * dz;
            }

            if center.iter().any(|value| !value.is_finite())
                || inertia.iter().any(|value| !value.is_finite())
            {
                return Err("mass-property calculation produced non-finite output".to_owned());
            }
            (Some(center), Some(inertia))
        } else {
            (None, None)
        };

        Ok(MassPropertiesReport {
            schema_version: MASS_PROPERTIES_SCHEMA_VERSION,
            evidence_level: "screening_only",
            configuration_sha256: self.configuration_sha256.clone(),
            status: if complete {
                MassPropertiesStatus::Complete
            } else {
                MassPropertiesStatus::Indeterminate
            },
            total_mass_kg,
            known_mass_subtotal_kg,
            center_of_gravity_m,
            inertia_tensor_about_cg_kg_m2,
            known_mass_by_category_kg: category_mass.into_iter().collect(),
            unknown_mass_item_ids,
            unknown_geometry_item_ids,
        })
    }

    /// Refuse to reuse a ledger against a different requested configuration.
    /// This is an identity-consistency check, not signature or source authentication.
    pub fn evaluate_for_configuration(
        &self,
        expected_configuration_sha256: &str,
    ) -> Result<MassPropertiesReport, String> {
        validate_sha256(expected_configuration_sha256, "expected_configuration_sha256")?;
        if self.configuration_sha256 != expected_configuration_sha256 {
            return Err(
                "stale mass ledger: configuration SHA-256 does not match requested aircraft"
                    .to_owned(),
            );
        }
        self.evaluate()
    }
}

fn validate_sha256(value: &str, field: &str) -> Result<(), String> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(format!("{field} must be 64 hexadecimal characters"));
    }
    Ok(())
}

fn validate_vector(value: &[f64; 3], item_id: &str) -> Result<(), String> {
    if value.iter().any(|component| !component.is_finite()) {
        return Err(format!(
            "mass item {item_id}: geometry coordinates must be finite"
        ));
    }
    Ok(())
}

/// Validate a symmetric inertia tensor as positive semidefinite within a
/// scale-relative numerical tolerance. This is a basic input sanity check,
/// not a structural or physical validation of the modeled component.
fn validate_inertia_tensor(value: &[f64; 6], item_id: &str) -> Result<(), String> {
    if value.iter().any(|component| !component.is_finite()) {
        return Err(format!(
            "mass item {item_id}: inertia tensor entries must be finite"
        ));
    }

    let [xx, yy, zz, xy, xz, yz] = *value;
    let scale = value
        .iter()
        .fold(0.0_f64, |largest, component| largest.max(component.abs()));
    let scale = scale.max(1.0e-30);
    let tol2 = 1.0e-12 * scale.powi(2);
    let tol3 = 1.0e-12 * scale.powi(3);

    let minor_xy = xx * yy - xy * xy;
    let minor_xz = xx * zz - xz * xz;
    let minor_yz = yy * zz - yz * yz;
    let determinant = xx * yy * zz + 2.0 * xy * xz * yz
        - xx * yz * yz
        - yy * xz * xz
        - zz * xy * xy;

    if xx < -1.0e-12 * scale
        || yy < -1.0e-12 * scale
        || zz < -1.0e-12 * scale
        || minor_xy < -tol2
        || minor_xz < -tol2
        || minor_yz < -tol2
        || determinant < -tol3
    {
        return Err(format!(
            "mass item {item_id}: inertia tensor is not positive semidefinite"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONFIG: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    fn point(id: &str, category: MassItemCategory, mass_kg: f64, x: f64, y: f64, z: f64) -> MassItem {
        MassItem {
            id: id.to_owned(),
            category,
            mass_kg: Some(mass_kg),
            geometry: MassGeometry::PointMass {
                position_m: [x, y, z],
            },
        }
    }

    fn known_ledger() -> MassLedger {
        MassLedger {
            configuration_sha256: CONFIG.to_owned(),
            items: vec![
                point("left_wing_lump", MassItemCategory::PrimaryStructure, 50.0, 1.30, -1.80, 0.40),
                point("right_wing_lump", MassItemCategory::PrimaryStructure, 50.0, 1.30, 1.80, 0.40),
                point("fuselage_structure_lump", MassItemCategory::PrimaryStructure, 260.0, 1.60, 0.0, 0.35),
                point("propulsion_lump", MassItemCategory::Propulsion, 120.0, 2.20, 0.0, 0.60),
                point("fuel_lump", MassItemCategory::FuelAndFluids, 85.0, 2.10, 0.0, 0.25),
                point("systems_lump", MassItemCategory::Avionics, 95.0, 1.20, 0.0, 0.45),
                point("pilot_point_mass", MassItemCategory::Payload, 80.0, 2.70, -0.35, 0.55),
                point("passenger_point_mass", MassItemCategory::Payload, 80.0, 2.70, 0.35, 0.55),
                point("baggage_point_mass", MassItemCategory::Payload, 25.0, 3.10, 0.0, 0.35),
            ],
        }
    }

    #[test]
    fn independently_hand_calculable_point_mass_fixture_closes() {
        let report = known_ledger().evaluate().expect("valid ledger");
        assert_eq!(report.status, MassPropertiesStatus::Complete);
        assert_eq!(report.evidence_level, "screening_only");
        assert!((report.total_mass_kg.expect("mass") - 845.0).abs() < 1.0e-12);

        let cg = report.center_of_gravity_m.expect("complete CG");
        assert!((cg[0] - 1.907_692_307_692_307_8).abs() < 1.0e-12);
        assert!(cg[1].abs() < 1.0e-12);
        assert!((cg[2] - 0.430_473_372_781_065_07).abs() < 1.0e-12);

        let inertia = report
            .inertia_tensor_about_cg_kg_m2
            .expect("complete inertia");
        let expected = [
            354.077_810_650_887_6,
            268.977_810_650_887_6,
            602.1,
            0.0,
            -22.726_923_076_923_086,
            0.0,
        ];
        for (actual, expected) in inertia.iter().zip(expected) {
            assert!((actual - expected).abs() < 1.0e-9);
        }
    }

    #[test]
    fn missing_mass_is_indeterminate_not_zero() {
        let mut ledger = known_ledger();
        ledger.items.push(MassItem {
            id: "cooling_system".to_owned(),
            category: MassItemCategory::ThermalManagement,
            mass_kg: None,
            geometry: MassGeometry::Unknown,
        });
        let report = ledger.evaluate().expect("unknown remains representable");
        assert_eq!(report.status, MassPropertiesStatus::Indeterminate);
        assert_eq!(report.total_mass_kg, None);
        assert_eq!(report.center_of_gravity_m, None);
        assert_eq!(report.inertia_tensor_about_cg_kg_m2, None);
        assert_eq!(report.unknown_mass_item_ids, vec!["cooling_system"]);
        assert_eq!(report.unknown_geometry_item_ids, vec!["cooling_system"]);
        assert!((report.known_mass_subtotal_kg - 845.0).abs() < 1.0e-12);
    }

    #[test]
    fn unknown_geometry_with_known_mass_preserves_mass_but_withholds_balance() {
        let mut ledger = known_ledger();
        ledger.items.push(MassItem {
            id: "distributed_cooling_pack".to_owned(),
            category: MassItemCategory::ThermalManagement,
            mass_kg: Some(18.0),
            geometry: MassGeometry::Unknown,
        });
        let report = ledger.evaluate().expect("unknown geometry");
        assert_eq!(report.status, MassPropertiesStatus::Indeterminate);
        assert!((report.total_mass_kg.expect("mass is known") - 863.0).abs() < 1.0e-12);
        assert_eq!(report.center_of_gravity_m, None);
        assert_eq!(
            report.unknown_geometry_item_ids,
            vec!["distributed_cooling_pack"]
        );
    }

    #[test]
    fn stale_configuration_identity_is_rejected() {
        let ledger = known_ledger();
        let other = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
        let error = ledger
            .evaluate_for_configuration(other)
            .expect_err("stale configuration must not reuse mass properties");
        assert!(error.contains("stale mass ledger"));
    }

    #[test]
    fn duplicate_mass_item_ids_are_rejected() {
        let mut ledger = known_ledger();
        ledger.items.push(ledger.items[0].clone());
        let error = ledger.evaluate().expect_err("duplicate ID");
        assert!(error.contains("duplicate mass item id"));
    }

    #[test]
    fn rigid_body_inertia_is_shifted_to_total_aircraft_cg() {
        let ledger = MassLedger {
            configuration_sha256: CONFIG.to_owned(),
            items: vec![
                MassItem {
                    id: "rigid_engine".to_owned(),
                    category: MassItemCategory::Propulsion,
                    mass_kg: Some(2.0),
                    geometry: MassGeometry::RigidBody {
                        center_of_mass_m: [0.0, 0.0, 0.0],
                        inertia_tensor_about_center_kg_m2: [1.0, 2.0, 3.0, 0.0, 0.0, 0.0],
                    },
                },
                point("ballast", MassItemCategory::Other, 2.0, 2.0, 0.0, 0.0),
            ],
        };
        let report = ledger.evaluate().expect("valid rigid body");
        assert_eq!(report.center_of_gravity_m, Some([1.0, 0.0, 0.0]));
        assert_eq!(
            report.inertia_tensor_about_cg_kg_m2,
            Some([1.0, 6.0, 7.0, 0.0, 0.0, 0.0])
        );
    }

    #[test]
    fn non_physical_inertia_tensor_is_rejected() {
        let ledger = MassLedger {
            configuration_sha256: CONFIG.to_owned(),
            items: vec![MassItem {
                id: "invalid_body".to_owned(),
                category: MassItemCategory::Other,
                mass_kg: Some(1.0),
                geometry: MassGeometry::RigidBody {
                    center_of_mass_m: [0.0, 0.0, 0.0],
                    inertia_tensor_about_center_kg_m2: [1.0, 1.0, 1.0, 2.0, 0.0, 0.0],
                },
            }],
        };
        let error = ledger.evaluate().expect_err("indefinite tensor");
        assert!(error.contains("not positive semidefinite"));
    }

    #[test]
    fn malformed_configuration_digest_is_rejected() {
        let mut ledger = known_ledger();
        ledger.configuration_sha256 = "not-a-digest".to_owned();
        let error = ledger.evaluate().expect_err("invalid config digest");
        assert!(error.contains("64 hexadecimal"));
    }
}
