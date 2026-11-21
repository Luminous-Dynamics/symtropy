// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Deterministic 3D cargo-bay loading contract for MFB.
//!
//! This checks axis-aligned bounding-box placement, declared extraction
//! corridors, mass and supplied vehicle CG envelopes. It is not a packing
//! optimizer, restraint solver, axle-load model, crash model or vehicle
//! roadworthiness assessment.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const CARGO_LOADING_SCHEMA_VERSION: u16 = 1;
pub const MAX_PACKAGES_V1: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CargoEvidenceStatus {
    FreshInDomain,
    Missing,
    Unknown,
    Stale,
    Conflicted,
    Revoked,
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BoxOrientation {
    LengthWidthHeight,
    LengthHeightWidth,
    WidthLengthHeight,
    WidthHeightLength,
    HeightLengthWidth,
    HeightWidthLength,
}

impl BoxOrientation {
    fn dimensions(self, length_mm: u32, width_mm: u32, height_mm: u32) -> [u32; 3] {
        match self {
            Self::LengthWidthHeight => [length_mm, width_mm, height_mm],
            Self::LengthHeightWidth => [length_mm, height_mm, width_mm],
            Self::WidthLengthHeight => [width_mm, length_mm, height_mm],
            Self::WidthHeightLength => [width_mm, height_mm, length_mm],
            Self::HeightLengthWidth => [height_mm, length_mm, width_mm],
            Self::HeightWidthLength => [height_mm, width_mm, length_mm],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccessFace {
    XMin,
    XMax,
    YMin,
    YMax,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DimensionsMm {
    pub length_mm: u32,
    pub width_mm: u32,
    pub height_mm: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CargoCgBoundsMm {
    pub x_min_mm: i64,
    pub x_max_mm: i64,
    pub y_min_mm: i64,
    pub y_max_mm: i64,
    pub z_min_mm: i64,
    pub z_max_mm: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CargoBayEnvelope {
    pub bay_id: String,
    pub internal_dimensions_mm: DimensionsMm,
    pub vehicle_tare_mass_g: u64,
    /// Tare vehicle centre of gravity in the same coordinate frame as the bay.
    pub vehicle_tare_cg_mm: [i64; 3],
    pub maximum_cargo_payload_g: u64,
    pub maximum_gross_vehicle_mass_g: u64,
    /// Limits supplied by the owning vehicle engineering / road-dynamics profile.
    pub combined_cg_bounds_mm: CargoCgBoundsMm,
    pub bay_geometry_evidence: CargoEvidenceStatus,
    pub vehicle_mass_cg_evidence: CargoEvidenceStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CargoPlacementMm {
    pub x_mm: u32,
    pub y_mm: u32,
    pub z_mm: u32,
    pub orientation: BoxOrientation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CargoPackage {
    pub package_id: String,
    pub dimensions_mm: DimensionsMm,
    pub mass_g: u64,
    pub allowed_orientations: Vec<BoxOrientation>,
    pub placement: CargoPlacementMm,
    pub access_face: AccessFace,
    pub geometry_evidence: CargoEvidenceStatus,
    pub restraint_evidence: CargoEvidenceStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CargoLoadingInput {
    pub schema_version: u16,
    pub scenario_id: String,
    pub bay: CargoBayEnvelope,
    pub packages: Vec<CargoPackage>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CargoFeasibilityVerdict {
    Feasible,
    Infeasible,
    Indeterminate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CargoLoadingReport {
    pub schema_version: u16,
    pub scenario_id: String,
    pub bay_id: String,
    pub input_digest_sha256: String,
    pub verdict: CargoFeasibilityVerdict,
    pub package_count: usize,
    pub total_cargo_mass_g: Option<u64>,
    pub gross_vehicle_mass_g: Option<u64>,
    pub combined_cg_mm: Option<[i64; 3]>,
    pub cargo_payload_margin_g: Option<u64>,
    pub gross_vehicle_mass_margin_g: Option<u64>,
    pub findings: Vec<String>,
    pub geometry_model: String,
    pub claim_ceiling: String,
    pub motion_authority_granted: bool,
    pub physical_load_authorized: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Aabb {
    minimum: [u32; 3],
    maximum: [u32; 3],
}

impl Aabb {
    fn overlaps(self, other: Self) -> bool {
        self.minimum
            .iter()
            .zip(self.maximum.iter())
            .zip(other.minimum.iter())
            .zip(other.maximum.iter())
            .all(|(((left_min, left_max), right_min), right_max)| {
                *left_min < *right_max && *right_min < *left_max
            })
    }

    fn intersects_access_corridor(self, corridor: Self) -> bool {
        self.overlaps(corridor)
    }
}

#[derive(Debug, Clone)]
struct PlacedPackage {
    package_id: String,
    bounds: Aabb,
    mass_g: u64,
    access_face: AccessFace,
}

fn package_bounds(package: &CargoPackage) -> Result<Aabb, String> {
    let dimensions = package.placement.orientation.dimensions(
        package.dimensions_mm.length_mm,
        package.dimensions_mm.width_mm,
        package.dimensions_mm.height_mm,
    );
    let minimum = [
        package.placement.x_mm,
        package.placement.y_mm,
        package.placement.z_mm,
    ];
    let maximum = [
        minimum[0]
            .checked_add(dimensions[0])
            .ok_or_else(|| format!("package {} x extent overflow", package.package_id))?,
        minimum[1]
            .checked_add(dimensions[1])
            .ok_or_else(|| format!("package {} y extent overflow", package.package_id))?,
        minimum[2]
            .checked_add(dimensions[2])
            .ok_or_else(|| format!("package {} z extent overflow", package.package_id))?,
    ];
    Ok(Aabb { minimum, maximum })
}

fn access_corridor(package: Aabb, bay: [u32; 3], face: AccessFace) -> Aabb {
    let mut minimum = package.minimum;
    let mut maximum = package.maximum;
    match face {
        AccessFace::XMin => minimum[0] = 0,
        AccessFace::XMax => maximum[0] = bay[0],
        AccessFace::YMin => minimum[1] = 0,
        AccessFace::YMax => maximum[1] = bay[1],
    }
    Aabb { minimum, maximum }
}

fn canonical_input_digest(input: &CargoLoadingInput) -> Result<String, String> {
    let mut canonical = input.clone();
    canonical
        .packages
        .sort_by(|left, right| left.package_id.cmp(&right.package_id));
    for package in &mut canonical.packages {
        package.allowed_orientations.sort();
    }
    let encoded = serde_json::to_vec(&canonical)
        .map_err(|error| format!("cannot serialize canonical cargo input: {error}"))?;
    Ok(format!("{:x}", Sha256::digest(encoded)))
}

fn evidence_is_current(status: CargoEvidenceStatus) -> bool {
    status == CargoEvidenceStatus::FreshInDomain
}

fn structural_errors(input: &CargoLoadingInput) -> Vec<String> {
    let mut errors = Vec::new();
    if input.schema_version != CARGO_LOADING_SCHEMA_VERSION {
        errors.push(format!(
            "schema_version: expected {}, got {}",
            CARGO_LOADING_SCHEMA_VERSION, input.schema_version
        ));
    }
    if input.scenario_id.trim().is_empty() || input.bay.bay_id.trim().is_empty() {
        errors.push("scenario_id and bay_id must be non-empty".into());
    }
    let bay = &input.bay;
    if bay.internal_dimensions_mm.length_mm == 0
        || bay.internal_dimensions_mm.width_mm == 0
        || bay.internal_dimensions_mm.height_mm == 0
    {
        errors.push("cargo bay dimensions must be non-zero".into());
    }
    if bay.vehicle_tare_mass_g == 0
        || bay.maximum_cargo_payload_g == 0
        || bay.maximum_gross_vehicle_mass_g <= bay.vehicle_tare_mass_g
    {
        errors.push("vehicle tare and cargo/gross mass limits are invalid".into());
    }
    let cg = &bay.combined_cg_bounds_mm;
    if cg.x_min_mm > cg.x_max_mm || cg.y_min_mm > cg.y_max_mm || cg.z_min_mm > cg.z_max_mm {
        errors.push("combined CG bounds are inverted".into());
    }
    if input.packages.is_empty() || input.packages.len() > MAX_PACKAGES_V1 {
        errors.push(format!("packages: expected 1..={MAX_PACKAGES_V1} packages"));
    }
    let mut package_ids = BTreeSet::new();
    for (index, package) in input.packages.iter().enumerate() {
        if package.package_id.trim().is_empty() {
            errors.push(format!("packages[{index}].package_id: must be non-empty"));
        }
        if !package_ids.insert(package.package_id.as_str()) {
            errors.push(format!("packages[{index}].package_id: duplicate identity"));
        }
        if package.dimensions_mm.length_mm == 0
            || package.dimensions_mm.width_mm == 0
            || package.dimensions_mm.height_mm == 0
            || package.mass_g == 0
        {
            errors.push(format!(
                "packages[{index}]: dimensions and mass must be non-zero"
            ));
        }
        if package.allowed_orientations.is_empty() {
            errors.push(format!(
                "packages[{index}].allowed_orientations: must not be empty"
            ));
        }
        let mut orientations = BTreeSet::new();
        for orientation in &package.allowed_orientations {
            if !orientations.insert(*orientation) {
                errors.push(format!(
                    "packages[{index}].allowed_orientations: duplicate orientation"
                ));
            }
        }
        if !package
            .allowed_orientations
            .contains(&package.placement.orientation)
        {
            errors.push(format!(
                "packages[{index}].placement.orientation: selected orientation is not permitted"
            ));
        }
    }
    errors
}

fn weighted_cg(
    bay: &CargoBayEnvelope,
    placed: &[PlacedPackage],
    total_mass_g: u64,
) -> Result<([i64; 3], [i128; 3], i128), String> {
    // Doubled coordinates preserve half-millimetre package centres while
    // signed arithmetic permits the vehicle tare CG below the cargo floor.
    let mut weighted = [0_i128; 3];
    for (weighted_axis, tare_cg_mm) in weighted.iter_mut().zip(bay.vehicle_tare_cg_mm.iter()) {
        let doubled_tare_cg = i128::from(*tare_cg_mm)
            .checked_mul(2)
            .ok_or_else(|| "tare CG doubled-coordinate overflow".to_string())?;
        *weighted_axis = i128::from(bay.vehicle_tare_mass_g)
            .checked_mul(doubled_tare_cg)
            .ok_or_else(|| "tare CG weighted-mass overflow".to_string())?;
    }

    for package in placed {
        for (weighted_axis, (minimum, maximum)) in weighted.iter_mut().zip(
            package
                .bounds
                .minimum
                .iter()
                .zip(package.bounds.maximum.iter()),
        ) {
            let doubled_center = i128::from(*minimum)
                .checked_mul(2)
                .and_then(|value| value.checked_add(i128::from(*maximum - *minimum)))
                .ok_or_else(|| "package CG coordinate overflow".to_string())?;
            let contribution = i128::from(package.mass_g)
                .checked_mul(doubled_center)
                .ok_or_else(|| "package weighted-mass overflow".to_string())?;
            *weighted_axis = weighted_axis
                .checked_add(contribution)
                .ok_or_else(|| "combined weighted-mass overflow".to_string())?;
        }
    }

    let denominator = i128::from(total_mass_g)
        .checked_mul(2)
        .ok_or_else(|| "combined mass denominator overflow".to_string())?;
    let mut result = [0_i64; 3];
    for (result_axis, weighted_axis) in result.iter_mut().zip(weighted.iter()) {
        let coordinate = *weighted_axis / denominator;
        *result_axis = i64::try_from(coordinate)
            .map_err(|_| "combined CG exceeds supported coordinate range".to_string())?;
    }
    Ok((result, weighted, denominator))
}

fn check_accessibility(
    placed: &[PlacedPackage],
    bay: [u32; 3],
    findings: &mut Vec<String>,
) -> bool {
    let mut blocked = false;
    for target in placed {
        let corridor = access_corridor(target.bounds, bay, target.access_face);
        for blocker in placed {
            if blocker.package_id == target.package_id {
                continue;
            }
            if blocker.bounds.intersects_access_corridor(corridor) {
                blocked = true;
                findings.push(format!(
                    "INFEASIBLE: package {} is blocked from {:?} by package {}",
                    target.package_id, target.access_face, blocker.package_id
                ));
            }
        }
    }
    blocked
}

/// Validate one declared multi-package cargo arrangement.
///
/// A feasible result is limited to the supplied axis-aligned box, mass and CG
/// envelopes. It is not a physical restraint, axle-load or crash-safety result.
pub fn evaluate_cargo_loading(
    input: &CargoLoadingInput,
) -> Result<CargoLoadingReport, Vec<String>> {
    let errors = structural_errors(input);
    if !errors.is_empty() {
        return Err(errors);
    }
    let input_digest_sha256 = canonical_input_digest(input).map_err(|error| vec![error])?;

    let mut findings = Vec::new();
    let mut known_infeasible = false;
    let mut indeterminate = false;

    for (label, status) in [
        ("cargo bay geometry", input.bay.bay_geometry_evidence),
        (
            "vehicle mass / CG envelope",
            input.bay.vehicle_mass_cg_evidence,
        ),
    ] {
        if !evidence_is_current(status) {
            indeterminate = true;
            findings.push(format!("INDETERMINATE: {label} evidence is {status:?}"));
        }
    }

    let bay_dimensions = [
        input.bay.internal_dimensions_mm.length_mm,
        input.bay.internal_dimensions_mm.width_mm,
        input.bay.internal_dimensions_mm.height_mm,
    ];
    let mut placed = Vec::with_capacity(input.packages.len());
    let mut total_cargo_mass_g = 0_u64;
    let mut placement_complete = true;

    for package in &input.packages {
        if !evidence_is_current(package.geometry_evidence) {
            indeterminate = true;
            findings.push(format!(
                "INDETERMINATE: package {} geometry evidence is {:?}",
                package.package_id, package.geometry_evidence
            ));
        }
        match package.restraint_evidence {
            CargoEvidenceStatus::FreshInDomain => {}
            CargoEvidenceStatus::Missing
            | CargoEvidenceStatus::Unknown
            | CargoEvidenceStatus::Stale
            | CargoEvidenceStatus::Conflicted
            | CargoEvidenceStatus::Revoked
            | CargoEvidenceStatus::Unsupported => {
                indeterminate = true;
                findings.push(format!(
                    "INDETERMINATE: package {} restraint evidence is {:?}",
                    package.package_id, package.restraint_evidence
                ));
            }
        }

        match package_bounds(package) {
            Ok(bounds) => {
                if bounds
                    .maximum
                    .iter()
                    .zip(bay_dimensions.iter())
                    .any(|(maximum, limit)| maximum > limit)
                {
                    known_infeasible = true;
                    findings.push(format!(
                        "INFEASIBLE: package {} is outside the cargo bay",
                        package.package_id
                    ));
                }
                placed.push(PlacedPackage {
                    package_id: package.package_id.clone(),
                    bounds,
                    mass_g: package.mass_g,
                    access_face: package.access_face,
                });
            }
            Err(error) => {
                known_infeasible = true;
                placement_complete = false;
                findings.push(format!("INFEASIBLE: {error}"));
            }
        }

        match total_cargo_mass_g.checked_add(package.mass_g) {
            Some(total) => total_cargo_mass_g = total,
            None => {
                known_infeasible = true;
                findings.push("INFEASIBLE: total cargo mass overflows".into());
            }
        }
    }

    for (left_index, left) in placed.iter().enumerate() {
        for right in placed.iter().skip(left_index + 1) {
            if left.bounds.overlaps(right.bounds) {
                known_infeasible = true;
                let (first_id, second_id) = if left.package_id <= right.package_id {
                    (&left.package_id, &right.package_id)
                } else {
                    (&right.package_id, &left.package_id)
                };
                findings.push(format!(
                    "INFEASIBLE: packages {first_id} and {second_id} overlap"
                ));
            }
        }
    }

    known_infeasible |= check_accessibility(&placed, bay_dimensions, &mut findings);

    if total_cargo_mass_g > input.bay.maximum_cargo_payload_g {
        known_infeasible = true;
        findings.push("INFEASIBLE: cargo payload exceeds vehicle limit".into());
    }
    let gross_vehicle_mass_g = input
        .bay
        .vehicle_tare_mass_g
        .checked_add(total_cargo_mass_g);
    let gross_vehicle_mass_g = match gross_vehicle_mass_g {
        Some(gross) => Some(gross),
        None => {
            known_infeasible = true;
            findings.push("INFEASIBLE: gross vehicle mass overflows".into());
            None
        }
    };
    let gross_vehicle_mass_margin_g = gross_vehicle_mass_g
        .and_then(|gross| input.bay.maximum_gross_vehicle_mass_g.checked_sub(gross));
    if gross_vehicle_mass_margin_g.is_none() {
        known_infeasible = true;
        findings.push("INFEASIBLE: gross vehicle mass exceeds vehicle limit".into());
    }

    let total_mass = gross_vehicle_mass_g.unwrap_or(0);
    let weighted_cg_result = if total_mass > 0 && placement_complete {
        match weighted_cg(&input.bay, &placed, total_mass) {
            Ok(result) => Some(result),
            Err(error) => {
                known_infeasible = true;
                findings.push(format!("INFEASIBLE: {error}"));
                None
            }
        }
    } else {
        None
    };
    let combined_cg_mm = weighted_cg_result.map(|(cg, _, _)| cg);

    if let Some((_, weighted_twice_mm_g, denominator)) = weighted_cg_result {
        let limits = &input.bay.combined_cg_bounds_mm;
        let lower = [limits.x_min_mm, limits.y_min_mm, limits.z_min_mm];
        let upper = [limits.x_max_mm, limits.y_max_mm, limits.z_max_mm];
        let outside = weighted_twice_mm_g
            .iter()
            .zip(lower.iter())
            .zip(upper.iter())
            .any(|((weighted, minimum), maximum)| {
                i128::from(*minimum)
                    .checked_mul(denominator)
                    .is_none_or(|bound| *weighted < bound)
                    || i128::from(*maximum)
                        .checked_mul(denominator)
                        .is_none_or(|bound| *weighted > bound)
            });
        if outside {
            known_infeasible = true;
            findings.push(
                "INFEASIBLE: combined vehicle centre of gravity is outside supplied envelope"
                    .into(),
            );
        }
    }

    let cargo_payload_margin_g = input
        .bay
        .maximum_cargo_payload_g
        .checked_sub(total_cargo_mass_g);
    if findings.is_empty() {
        findings
            .push("FEASIBLE: declared 3D placements, access, mass and CG envelopes pass".into());
    }
    findings.sort();
    let verdict = if known_infeasible {
        CargoFeasibilityVerdict::Infeasible
    } else if indeterminate {
        CargoFeasibilityVerdict::Indeterminate
    } else {
        CargoFeasibilityVerdict::Feasible
    };

    Ok(CargoLoadingReport {
        schema_version: CARGO_LOADING_SCHEMA_VERSION,
        scenario_id: input.scenario_id.clone(),
        bay_id: input.bay.bay_id.clone(),
        input_digest_sha256,
        verdict,
        package_count: input.packages.len(),
        total_cargo_mass_g: Some(total_cargo_mass_g),
        gross_vehicle_mass_g,
        combined_cg_mm,
        cargo_payload_margin_g,
        gross_vehicle_mass_margin_g,
        findings,
        geometry_model: "axis-aligned cuboid placement with straight-line extraction corridors".into(),
        claim_ceiling: "software geometry, gross-mass and supplied CG-envelope checks only; no restraint, axle, roadworthiness or physical loading qualification".into(),
        motion_authority_granted: false,
        physical_load_authorized: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input() -> CargoLoadingInput {
        CargoLoadingInput {
            schema_version: 1,
            scenario_id: "synthetic-three-package-cargo-v1".into(),
            bay: CargoBayEnvelope {
                bay_id: "synthetic-bay-01".into(),
                internal_dimensions_mm: DimensionsMm {
                    length_mm: 3_000,
                    width_mm: 1_800,
                    height_mm: 1_800,
                },
                vehicle_tare_mass_g: 2_000_000,
                vehicle_tare_cg_mm: [1_500, 900, 700],
                maximum_cargo_payload_g: 1_500_000,
                maximum_gross_vehicle_mass_g: 3_800_000,
                combined_cg_bounds_mm: CargoCgBoundsMm {
                    x_min_mm: 1_000,
                    x_max_mm: 2_000,
                    y_min_mm: 500,
                    y_max_mm: 1_300,
                    z_min_mm: 300,
                    z_max_mm: 1_200,
                },
                bay_geometry_evidence: CargoEvidenceStatus::FreshInDomain,
                vehicle_mass_cg_evidence: CargoEvidenceStatus::FreshInDomain,
            },
            packages: vec![
                CargoPackage {
                    package_id: "parcel-a".into(),
                    dimensions_mm: DimensionsMm {
                        length_mm: 600,
                        width_mm: 500,
                        height_mm: 400,
                    },
                    mass_g: 100_000,
                    allowed_orientations: vec![BoxOrientation::LengthWidthHeight],
                    placement: CargoPlacementMm {
                        x_mm: 0,
                        y_mm: 0,
                        z_mm: 0,
                        orientation: BoxOrientation::LengthWidthHeight,
                    },
                    access_face: AccessFace::XMin,
                    geometry_evidence: CargoEvidenceStatus::FreshInDomain,
                    restraint_evidence: CargoEvidenceStatus::FreshInDomain,
                },
                CargoPackage {
                    package_id: "parcel-b".into(),
                    dimensions_mm: DimensionsMm {
                        length_mm: 500,
                        width_mm: 500,
                        height_mm: 400,
                    },
                    mass_g: 150_000,
                    allowed_orientations: vec![BoxOrientation::LengthWidthHeight],
                    placement: CargoPlacementMm {
                        x_mm: 700,
                        y_mm: 600,
                        z_mm: 0,
                        orientation: BoxOrientation::LengthWidthHeight,
                    },
                    access_face: AccessFace::XMin,
                    geometry_evidence: CargoEvidenceStatus::FreshInDomain,
                    restraint_evidence: CargoEvidenceStatus::FreshInDomain,
                },
                CargoPackage {
                    package_id: "parcel-c".into(),
                    dimensions_mm: DimensionsMm {
                        length_mm: 400,
                        width_mm: 400,
                        height_mm: 300,
                    },
                    mass_g: 80_000,
                    allowed_orientations: vec![BoxOrientation::LengthWidthHeight],
                    placement: CargoPlacementMm {
                        x_mm: 1_300,
                        y_mm: 1_200,
                        z_mm: 0,
                        orientation: BoxOrientation::LengthWidthHeight,
                    },
                    access_face: AccessFace::XMin,
                    geometry_evidence: CargoEvidenceStatus::FreshInDomain,
                    restraint_evidence: CargoEvidenceStatus::FreshInDomain,
                },
            ],
        }
    }

    #[test]
    fn accepts_synthetic_three_package_layout() {
        let report = evaluate_cargo_loading(&input()).expect("valid input");
        assert_eq!(report.verdict, CargoFeasibilityVerdict::Feasible);
        assert_eq!(report.total_cargo_mass_g, Some(330_000));
        assert_eq!(report.gross_vehicle_mass_g, Some(2_330_000));
        assert_eq!(report.cargo_payload_margin_g, Some(1_170_000));
        assert!(!report.motion_authority_granted);
        assert!(!report.physical_load_authorized);
    }

    #[test]
    fn rejects_overlapping_packages() {
        let mut test = input();
        test.packages[1].placement.x_mm = 500;
        test.packages[1].placement.y_mm = 400;
        let report = evaluate_cargo_loading(&test).expect("well-formed input");
        assert_eq!(report.verdict, CargoFeasibilityVerdict::Infeasible);
        assert!(report.findings.iter().any(|f| f.contains("overlap")));
    }

    #[test]
    fn omits_combined_cg_when_package_extent_arithmetic_overflows() {
        let mut test = input();
        test.packages[2].placement.x_mm = u32::MAX;
        let report = evaluate_cargo_loading(&test).expect("well-formed input");
        assert_eq!(report.verdict, CargoFeasibilityVerdict::Infeasible);
        assert_eq!(report.combined_cg_mm, None);
    }

    #[test]
    fn rejects_package_outside_bay() {
        let mut test = input();
        test.packages[2].placement.x_mm = 2_800;
        let report = evaluate_cargo_loading(&test).expect("well-formed input");
        assert_eq!(report.verdict, CargoFeasibilityVerdict::Infeasible);
    }

    #[test]
    fn accepts_declared_axis_rotation_within_bay() {
        let mut test = input();
        test.packages[0]
            .allowed_orientations
            .push(BoxOrientation::LengthHeightWidth);
        test.packages[0].placement.orientation = BoxOrientation::LengthHeightWidth;
        let report = evaluate_cargo_loading(&test).expect("valid rotated placement");
        assert_eq!(report.verdict, CargoFeasibilityVerdict::Feasible);
    }

    #[test]
    fn rejects_package_orientation_not_allowed() {
        let mut test = input();
        test.packages[0].placement.orientation = BoxOrientation::WidthLengthHeight;
        let errors = evaluate_cargo_loading(&test).expect_err("unsupported orientation");
        assert!(errors.iter().any(|e| e.contains("not permitted")));
    }

    #[test]
    fn rejects_package_blocked_from_required_access_face() {
        let mut test = input();
        test.packages[0].access_face = AccessFace::XMax;
        test.packages[1].placement.y_mm = 0;
        let report = evaluate_cargo_loading(&test).expect("well-formed input");
        assert_eq!(report.verdict, CargoFeasibilityVerdict::Infeasible);
        assert!(report.findings.iter().any(|f| f.contains("is blocked")));
    }

    #[test]
    fn rejects_cargo_payload_over_limit() {
        let mut test = input();
        test.bay.maximum_cargo_payload_g = 300_000;
        let report = evaluate_cargo_loading(&test).expect("well-formed input");
        assert_eq!(report.verdict, CargoFeasibilityVerdict::Infeasible);
    }

    #[test]
    fn rejects_gross_vehicle_mass_over_limit() {
        let mut test = input();
        test.bay.maximum_gross_vehicle_mass_g = 2_300_000;
        let report = evaluate_cargo_loading(&test).expect("well-formed input");
        assert_eq!(report.verdict, CargoFeasibilityVerdict::Infeasible);
    }

    #[test]
    fn rejects_combined_cg_outside_supplied_envelope() {
        let mut test = input();
        test.bay.combined_cg_bounds_mm.x_min_mm = 1_800;
        test.bay.combined_cg_bounds_mm.x_max_mm = 1_900;
        let report = evaluate_cargo_loading(&test).expect("well-formed input");
        assert_eq!(report.verdict, CargoFeasibilityVerdict::Infeasible);
    }

    #[test]
    fn stale_package_geometry_is_indeterminate() {
        let mut test = input();
        test.packages[0].geometry_evidence = CargoEvidenceStatus::Stale;
        let report = evaluate_cargo_loading(&test).expect("well-formed input");
        assert_eq!(report.verdict, CargoFeasibilityVerdict::Indeterminate);
    }

    #[test]
    fn missing_restraint_evidence_is_indeterminate() {
        let mut test = input();
        test.packages[0].restraint_evidence = CargoEvidenceStatus::Missing;
        let report = evaluate_cargo_loading(&test).expect("well-formed input");
        assert_eq!(report.verdict, CargoFeasibilityVerdict::Indeterminate);
    }

    #[test]
    fn canonical_digest_ignores_package_and_orientation_list_order() {
        let mut first = input();
        first.packages[1].placement.x_mm = 500;
        first.packages[1].placement.y_mm = 400;
        first.packages[0].access_face = AccessFace::XMax;
        let mut second = first.clone();
        second.packages.reverse();
        for package in &mut second.packages {
            package.allowed_orientations.reverse();
        }
        let first_report = evaluate_cargo_loading(&first).expect("first input");
        let second_report = evaluate_cargo_loading(&second).expect("permuted input");
        assert_eq!(
            first_report.input_digest_sha256,
            second_report.input_digest_sha256
        );
        assert_eq!(first_report, second_report);
        assert_eq!(first_report.verdict, CargoFeasibilityVerdict::Infeasible);
    }

    #[test]
    fn rejects_fractional_cg_just_outside_integer_limit() {
        let mut test = input();
        test.bay.combined_cg_bounds_mm.x_max_mm = 1_413;
        let report = evaluate_cargo_loading(&test).expect("valid input");
        assert_eq!(report.verdict, CargoFeasibilityVerdict::Infeasible);
    }

    #[test]
    fn supports_tare_centre_of_gravity_below_the_cargo_floor() {
        let mut test = input();
        test.bay.vehicle_tare_cg_mm[2] = -100;
        test.bay.combined_cg_bounds_mm.z_min_mm = -200;
        let report = evaluate_cargo_loading(&test).expect("signed vehicle CG");
        assert_eq!(report.verdict, CargoFeasibilityVerdict::Feasible);
        assert!(report.combined_cg_mm.expect("CG")[2] < 0);
    }

    #[test]
    fn rejects_duplicate_package_identity() {
        let mut test = input();
        test.packages[1].package_id = test.packages[0].package_id.clone();
        assert!(evaluate_cargo_loading(&test).is_err());
    }

    #[test]
    fn rejects_unknown_input_fields() {
        let value = serde_json::to_value(input()).expect("serialize input");
        let mut object = value.as_object().expect("object").clone();
        object.insert("optimistic_loading_pass".into(), serde_json::json!(true));
        assert!(serde_json::from_value::<CargoLoadingInput>(object.into()).is_err());
    }
}
