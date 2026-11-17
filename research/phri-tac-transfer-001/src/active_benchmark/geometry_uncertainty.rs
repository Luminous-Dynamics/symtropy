//! Propagate taxel-geometry and rigid-pose uncertainty into tactile wrench.
//!
//! The nominal wrench operator is built from as-characterized taxel area,
//! application point, normal and tangent basis. This module computes first-order
//! covariance contributions from (a) independent local per-taxel geometry
//! perturbations and (b) one common six-parameter rigid-pose perturbation.
//!
//! Local parameter order per taxel is:
//! [area_m2, point_dx_m, point_dy_m, point_dz_m,
//!  small_angle_rx_rad, small_angle_ry_rad, small_angle_rz_rad].
//!
//! Global parameter order is:
//! [translation_dx_m, translation_dy_m, translation_dz_m,
//!  left_rotation_rx_rad, left_rotation_ry_rad, left_rotation_rz_rad].
//!
//! A global transform is shared across taxels, so its contribution is
//! propagated once through the aggregate wrench Jacobian. It is not copied as
//! independent noise onto each taxel. When the wrench origin is attached to the
//! sensor body, common translation has no effect on the wrench about that
//! co-moving origin; for a fixed reference-frame origin, translation changes
//! moment by delta_t × F.
//!
//! V0.1 admits independent diagonal local geometry errors and a full 6×6
//! positive-definite common-pose covariance. Module-correlated local errors,
//! cross-taxel manufacturing covariance, geometry/field cross-covariance and
//! geometry/reference cross-covariance are explicit dispositions, not silently
//! approximated as independent. This is a first-order small-uncertainty model;
//! the uncertainty domain must be validated against Monte Carlo/physical data.

#![forbid(unsafe_code)]

use super::wrench_consistency::{
    FieldCovarianceStructure, FieldWrenchReferenceRelationship, TaxelGeometry, TaxelGeometrySet,
    TractionFieldEstimate, Vec3, WrenchVector, build_wrench_operator, integrate_tactile_field,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum GeometrySourceRelationship {
    IndependentlyCharacterized,
    SharedOrUnknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum WrenchOriginAttachment {
    /// The wrench origin moves with the sensorized body. Common translation
    /// changes coordinates but not the wrench about this co-moving origin.
    SensorBodyAttached,
    /// The wrench origin is fixed in the declared reference frame. Common
    /// sensor translation changes the moment arm relative to this origin.
    FixedInReferenceFrame,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct TaxelGeometryStandardUncertainty {
    pub taxel_id: String,
    /// One-standard-deviation uncertainty in effective contact area, m².
    pub area_m2: f64,
    /// Per-axis one-standard-deviation application-point uncertainty, m.
    pub application_point_m: Vec3,
    /// Per-axis small-angle one-standard-deviation local-basis uncertainty, rad.
    pub orientation_rad: Vec3,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct GeometryUncertaintyModel {
    pub geometry_digest: String,
    pub reference_frame_digest: String,
    pub contact_boundary_digest: String,
    pub transform_digest: String,
    pub sample_identity: String,
    pub native_observation_set_digest: String,
    pub field_estimate_digest: String,
    pub uncertainty_method_digest: String,
    pub reference_ancestry_receipt_id: String,
    pub common_cause_analysis_receipt_id: String,
    /// Ordered exactly as geometry.taxels. Cross-taxel local covariance is not
    /// supported in v0.1 and must be declared IndependentByTaxel.
    pub local_taxels: Vec<TaxelGeometryStandardUncertainty>,
    pub local_taxel_relationship: GeometrySourceRelationship,
    /// Full 6×6 covariance, row-major, in [m,m,m,rad,rad,rad] parameter order.
    pub common_rigid_pose_covariance_row_major: Vec<f64>,
    pub local_to_global_relationship: GeometrySourceRelationship,
    pub geometry_to_field_relationship: GeometrySourceRelationship,
    pub geometry_to_wrench_reference_relationship: GeometrySourceRelationship,
    pub wrench_origin_attachment: WrenchOriginAttachment,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum GeometryUqDisposition {
    IndependentGeometryCovarianceAvailable,
    GeometryCovarianceAvailableButCrossSourcesUnresolved,
    LocalGeometryCorrelationUnresolved,
    LocalGlobalGeometryCorrelationUnresolved,
    InvalidGeometryUncertainty,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct GeometryWrenchUncertaintyReport {
    pub disposition: GeometryUqDisposition,
    pub nominal_wrench: Option<WrenchVector>,
    /// Six-axis covariance due to local per-taxel geometry uncertainty alone.
    pub local_geometry_covariance_row_major: Option<Vec<f64>>,
    /// Six-axis covariance due to common rigid-pose uncertainty alone.
    pub global_pose_covariance_row_major: Option<Vec<f64>>,
    /// Sum of local and global geometry covariance only when their independence
    /// is explicitly characterized.
    pub total_geometry_covariance_row_major: Option<Vec<f64>>,
    /// A separate total-geometry covariance is not automatically admissible
    /// alongside field/reference covariance. All cross-source independence
    /// relationships must also be independently characterized.
    pub admissible_for_combined_wrench_uncertainty: bool,
    pub local_taxel_parameter_count: usize,
    pub global_pose_parameter_count: usize,
    pub geometry_digest: String,
    pub reference_frame_digest: String,
    pub transform_digest: String,
    pub sample_identity: String,
    pub field_estimate_digest: String,
    pub native_observation_set_digest: String,
    pub reasons: Vec<String>,
    pub claim_class: &'static str,
}

fn vec_add(left: Vec3, right: Vec3) -> Vec3 {
    Vec3 {
        x: left.x + right.x,
        y: left.y + right.y,
        z: left.z + right.z,
    }
}

fn vec_subtract(left: Vec3, right: Vec3) -> Vec3 {
    Vec3 {
        x: left.x - right.x,
        y: left.y - right.y,
        z: left.z - right.z,
    }
}

fn vec_scale(value: Vec3, scale: f64) -> Vec3 {
    Vec3 {
        x: value.x * scale,
        y: value.y * scale,
        z: value.z * scale,
    }
}

fn vec_cross(left: Vec3, right: Vec3) -> Vec3 {
    Vec3 {
        x: left.y * right.z - left.z * right.y,
        y: left.z * right.x - left.x * right.z,
        z: left.x * right.y - left.y * right.x,
    }
}

fn unit_axis(index: usize) -> Vec3 {
    match index {
        0 => Vec3 {
            x: 1.0,
            y: 0.0,
            z: 0.0,
        },
        1 => Vec3 {
            x: 0.0,
            y: 1.0,
            z: 0.0,
        },
        _ => Vec3 {
            x: 0.0,
            y: 0.0,
            z: 1.0,
        },
    }
}

fn finite_vec(value: Vec3) -> bool {
    value.x.is_finite() && value.y.is_finite() && value.z.is_finite()
}

fn outer_add(covariance: &mut [[f64; 6]; 6], jacobian: &[f64; 6], variance: f64) {
    for row in 0..6 {
        for column in 0..6 {
            covariance[row][column] += jacobian[row] * jacobian[column] * variance;
        }
    }
}

fn matrix_add(left: &[[f64; 6]; 6], right: &[[f64; 6]; 6]) -> [[f64; 6]; 6] {
    let mut result = [[0.0; 6]; 6];
    for row in 0..6 {
        for column in 0..6 {
            result[row][column] = left[row][column] + right[row][column];
        }
    }
    result
}

fn matrix_to_row_major(matrix: &[[f64; 6]; 6]) -> Vec<f64> {
    matrix.iter().flat_map(|row| row.iter().copied()).collect()
}

fn validate_spd_6(covariance: &[f64]) -> Result<[[f64; 6]; 6], String> {
    if covariance.len() != 36 || covariance.iter().any(|value| !value.is_finite()) {
        return Err(
            "common rigid-pose covariance must contain 36 finite row-major values".to_owned(),
        );
    }
    let mut matrix = [[0.0; 6]; 6];
    for (row, matrix_row) in matrix.iter_mut().enumerate() {
        for (column, value) in matrix_row.iter_mut().enumerate() {
            *value = covariance[row * 6 + column];
            let transpose = covariance[column * 6 + row];
            let scale = value.abs().max(transpose.abs()).max(f64::MIN_POSITIVE);
            if (*value - transpose).abs() > 1.0e-12 * scale {
                return Err("common rigid-pose covariance must be symmetric".to_owned());
            }
        }
    }
    let mut lower = [[0.0; 6]; 6];
    for row in 0..6 {
        for column in 0..=row {
            let correction = (0..column)
                .map(|index| lower[row][index] * lower[column][index])
                .sum::<f64>();
            if row == column {
                let diagonal = matrix[row][row] - correction;
                if !diagonal.is_finite() || diagonal <= 0.0 {
                    return Err("common rigid-pose covariance must be positive definite".to_owned());
                }
                lower[row][column] = diagonal.sqrt();
            } else {
                lower[row][column] = (matrix[row][column] - correction) / lower[column][column];
            }
        }
    }
    Ok(matrix)
}

fn validate_model(
    geometry: &TaxelGeometrySet,
    model: &GeometryUncertaintyModel,
) -> Result<[[f64; 6]; 6], String> {
    for (name, value) in [
        ("geometry_digest", model.geometry_digest.as_str()),
        (
            "reference_frame_digest",
            model.reference_frame_digest.as_str(),
        ),
        (
            "contact_boundary_digest",
            model.contact_boundary_digest.as_str(),
        ),
        ("transform_digest", model.transform_digest.as_str()),
        ("sample_identity", model.sample_identity.as_str()),
        (
            "native_observation_set_digest",
            model.native_observation_set_digest.as_str(),
        ),
        (
            "field_estimate_digest",
            model.field_estimate_digest.as_str(),
        ),
        (
            "uncertainty_method_digest",
            model.uncertainty_method_digest.as_str(),
        ),
        (
            "reference_ancestry_receipt_id",
            model.reference_ancestry_receipt_id.as_str(),
        ),
        (
            "common_cause_analysis_receipt_id",
            model.common_cause_analysis_receipt_id.as_str(),
        ),
    ] {
        if value.trim().is_empty() {
            return Err(format!("geometry uncertainty field {name} is missing"));
        }
    }
    if model.geometry_digest != geometry.geometry_digest
        || model.reference_frame_digest != geometry.reference_frame_digest
        || model.contact_boundary_digest != geometry.contact_boundary_digest
    {
        return Err(
            "geometry uncertainty digests do not match the exact geometry/frame/contact boundary"
                .to_owned(),
        );
    }
    if model.local_taxels.len() != geometry.taxels.len() {
        return Err(
            "local geometry uncertainty must contain exactly one entry per taxel".to_owned(),
        );
    }
    for (taxel, uncertainty) in geometry.taxels.iter().zip(&model.local_taxels) {
        if taxel.taxel_id != uncertainty.taxel_id {
            return Err(
                "local geometry uncertainty taxel identity/order does not match geometry"
                    .to_owned(),
            );
        }
        if !uncertainty.area_m2.is_finite()
            || uncertainty.area_m2 < 0.0
            || !finite_vec(uncertainty.application_point_m)
            || !finite_vec(uncertainty.orientation_rad)
            || uncertainty.application_point_m.x < 0.0
            || uncertainty.application_point_m.y < 0.0
            || uncertainty.application_point_m.z < 0.0
            || uncertainty.orientation_rad.x < 0.0
            || uncertainty.orientation_rad.y < 0.0
            || uncertainty.orientation_rad.z < 0.0
        {
            return Err(format!(
                "taxel {} geometry standard uncertainties must be finite and nonnegative",
                taxel.taxel_id
            ));
        }
    }
    validate_spd_6(&model.common_rigid_pose_covariance_row_major)
}

/// Propagate first-order local taxel geometry and common rigid-pose uncertainty
/// to the integrated wrench. Field and geometry means are unchanged.
pub fn propagate_geometry_wrench_uncertainty(
    geometry: &TaxelGeometrySet,
    field: &TractionFieldEstimate,
    model: &GeometryUncertaintyModel,
) -> Result<GeometryWrenchUncertaintyReport, String> {
    let pose_covariance = validate_model(geometry, model)?;
    if field.estimate_digest.trim().is_empty()
        || field.sample_identity.trim().is_empty()
        || field.native_observation_set_digest.trim().is_empty()
        || field.uncertainty_method_digest.trim().is_empty()
        || field
            .uncertainty_reference_ancestry_receipt_id
            .trim()
            .is_empty()
        || field
            .uncertainty_common_cause_analysis_receipt_id
            .trim()
            .is_empty()
        || field.standard_uncertainties_pa.len() != field.local_traction_pa.len()
        || field
            .standard_uncertainties_pa
            .iter()
            .any(|value| !value.is_finite() || *value < 0.0)
    {
        return Err("traction field estimate or uncertainty provenance is incomplete".to_owned());
    }
    if field.estimate_digest != model.field_estimate_digest
        || field.sample_identity != model.sample_identity
        || field.native_observation_set_digest != model.native_observation_set_digest
    {
        return Err("geometry uncertainty model does not bind the exact field estimate, sample and native observation set".to_owned());
    }
    let operator = build_wrench_operator(geometry)?;
    let nominal = integrate_tactile_field(&operator, &field.local_traction_pa)?;

    let mut local_covariance = [[0.0; 6]; 6];
    for (parameter, (taxel, uncertainty)) in
        geometry.taxels.iter().zip(&model.local_taxels).enumerate()
    {
        let offset = parameter * 3;
        let normal_pressure = field.local_traction_pa[offset];
        let tangent_u = field.local_traction_pa[offset + 1];
        let tangent_v = field.local_traction_pa[offset + 2];
        let traction_density = vec_add(
            vec_add(
                vec_scale(taxel.normal, normal_pressure),
                vec_scale(taxel.tangent_u, tangent_u),
            ),
            vec_scale(taxel.tangent_v, tangent_v),
        );
        let force_contribution = vec_scale(traction_density, taxel.area_m2);
        let arm = vec_subtract(taxel.application_point_m, geometry.wrench_origin_m);

        // dW/d(area) = [q, arm × q].
        let area_moment = vec_cross(arm, traction_density);
        let area_jacobian = [
            traction_density.x,
            traction_density.y,
            traction_density.z,
            area_moment.x,
            area_moment.y,
            area_moment.z,
        ];
        outer_add(
            &mut local_covariance,
            &area_jacobian,
            uncertainty.area_m2.powi(2),
        );

        // dM/d(application_point_k) = e_k × force_contribution.
        for axis in 0..3 {
            let derivative_moment = vec_cross(unit_axis(axis), force_contribution);
            let jacobian = [
                0.0,
                0.0,
                0.0,
                derivative_moment.x,
                derivative_moment.y,
                derivative_moment.z,
            ];
            let sigma = [
                uncertainty.application_point_m.x,
                uncertainty.application_point_m.y,
                uncertainty.application_point_m.z,
            ][axis];
            outer_add(&mut local_covariance, &jacobian, sigma.powi(2));
        }

        // A small local basis rotation changes q by delta_theta × q.
        // dW/d(theta_k) = [A(e_k × q), arm × A(e_k × q)].
        for axis in 0..3 {
            let delta_density = vec_cross(unit_axis(axis), traction_density);
            let delta_force = vec_scale(delta_density, taxel.area_m2);
            let delta_moment = vec_cross(arm, delta_force);
            let jacobian = [
                delta_force.x,
                delta_force.y,
                delta_force.z,
                delta_moment.x,
                delta_moment.y,
                delta_moment.z,
            ];
            let sigma = [
                uncertainty.orientation_rad.x,
                uncertainty.orientation_rad.y,
                uncertainty.orientation_rad.z,
            ][axis];
            outer_add(&mut local_covariance, &jacobian, sigma.powi(2));
        }
    }

    let force = nominal.force_n;
    let moment = nominal.moment_nm;
    let mut global_jacobian = [[0.0; 6]; 6];
    // Global parameter order: translation xyz, then left-rotation xyz.
    for axis in 0..3 {
        if model.wrench_origin_attachment == WrenchOriginAttachment::FixedInReferenceFrame {
            let translation_moment = vec_cross(unit_axis(axis), force);
            global_jacobian[3][axis] = translation_moment.x;
            global_jacobian[4][axis] = translation_moment.y;
            global_jacobian[5][axis] = translation_moment.z;
        }
        let rotation_force = vec_cross(unit_axis(axis), force);
        let rotation_moment = vec_cross(unit_axis(axis), moment);
        global_jacobian[0][axis + 3] = rotation_force.x;
        global_jacobian[1][axis + 3] = rotation_force.y;
        global_jacobian[2][axis + 3] = rotation_force.z;
        global_jacobian[3][axis + 3] = rotation_moment.x;
        global_jacobian[4][axis + 3] = rotation_moment.y;
        global_jacobian[5][axis + 3] = rotation_moment.z;
    }

    let mut global_covariance = [[0.0; 6]; 6];
    for row in 0..6 {
        for column in 0..6 {
            let mut value = 0.0;
            for left_parameter in 0..6 {
                for right_parameter in 0..6 {
                    value += global_jacobian[row][left_parameter]
                        * pose_covariance[left_parameter][right_parameter]
                        * global_jacobian[column][right_parameter];
                }
            }
            global_covariance[row][column] = value;
        }
    }

    let local_covariance_available =
        model.local_taxel_relationship == GeometrySourceRelationship::IndependentlyCharacterized;
    let total_geometry_covariance_available = local_covariance_available
        && model.local_to_global_relationship
            == GeometrySourceRelationship::IndependentlyCharacterized;
    let total = total_geometry_covariance_available
        .then(|| matrix_add(&local_covariance, &global_covariance));
    let combined_independence = total_geometry_covariance_available
        && model.geometry_to_field_relationship
            == GeometrySourceRelationship::IndependentlyCharacterized
        && model.geometry_to_wrench_reference_relationship
            == GeometrySourceRelationship::IndependentlyCharacterized
        && field.relationship_to_wrench_reference
            == FieldWrenchReferenceRelationship::IndependentFromWrenchReference
        && field.covariance_structure == FieldCovarianceStructure::IndependentDiagonalComponents;

    let mut reasons = Vec::new();
    if !local_covariance_available {
        reasons.push("per-taxel local geometry errors are correlated or unknown; independent local covariance sum is not admissible".to_owned());
    }
    if model.local_to_global_relationship != GeometrySourceRelationship::IndependentlyCharacterized
    {
        reasons
            .push("local and global geometry errors have unresolved cross-covariance".to_owned());
    }
    if model.geometry_to_field_relationship
        != GeometrySourceRelationship::IndependentlyCharacterized
    {
        reasons.push(
            "geometry and tactile field uncertainty share or may share evidence ancestry"
                .to_owned(),
        );
    }
    if model.geometry_to_wrench_reference_relationship
        != GeometrySourceRelationship::IndependentlyCharacterized
    {
        reasons.push(
            "geometry and wrench-reference uncertainty share or may share evidence ancestry"
                .to_owned(),
        );
    }
    if field.relationship_to_wrench_reference
        != FieldWrenchReferenceRelationship::IndependentFromWrenchReference
    {
        reasons.push("the exact traction-field estimate shares or may share wrench-reference/calibration ancestry".to_owned());
    }
    if field.covariance_structure != FieldCovarianceStructure::IndependentDiagonalComponents {
        reasons.push("traction-field covariance is correlated or unknown; combined independence is unresolved".to_owned());
    }

    let disposition = if combined_independence {
        GeometryUqDisposition::IndependentGeometryCovarianceAvailable
    } else if !local_covariance_available {
        GeometryUqDisposition::LocalGeometryCorrelationUnresolved
    } else if !total_geometry_covariance_available {
        GeometryUqDisposition::LocalGlobalGeometryCorrelationUnresolved
    } else {
        GeometryUqDisposition::GeometryCovarianceAvailableButCrossSourcesUnresolved
    };

    Ok(GeometryWrenchUncertaintyReport {
        disposition,
        nominal_wrench: Some(nominal),
        local_geometry_covariance_row_major: local_covariance_available
            .then(|| matrix_to_row_major(&local_covariance)),
        global_pose_covariance_row_major: Some(matrix_to_row_major(&global_covariance)),
        total_geometry_covariance_row_major: total.map(|matrix| matrix_to_row_major(&matrix)),
        admissible_for_combined_wrench_uncertainty: combined_independence,
        local_taxel_parameter_count: geometry.taxels.len() * 7,
        global_pose_parameter_count: 6,
        geometry_digest: geometry.geometry_digest.clone(),
        reference_frame_digest: geometry.reference_frame_digest.clone(),
        transform_digest: model.transform_digest.clone(),
        sample_identity: model.sample_identity.clone(),
        field_estimate_digest: model.field_estimate_digest.clone(),
        native_observation_set_digest: model.native_observation_set_digest.clone(),
        reasons,
        claim_class: "synthetic_geometry_uncertainty_propagation_only",
    })
}

/// Deterministic small synthetic CLI example for the geometry uncertainty
/// path. This is a contract smoke-test, not physical sensor evidence.
pub fn run_synthetic_geometry_uncertainty_example()
-> Result<GeometryWrenchUncertaintyReport, String> {
    let tangent_u = Vec3 {
        x: 1.0,
        y: 0.0,
        z: 0.0,
    };
    let tangent_v = Vec3 {
        x: 0.0,
        y: 1.0,
        z: 0.0,
    };
    let normal = Vec3 {
        x: 0.0,
        y: 0.0,
        z: 1.0,
    };
    let geometry = TaxelGeometrySet {
        geometry_digest: "sha256:synthetic-geometry-uq-grid".to_owned(),
        reference_frame_digest: "sha256:synthetic-geometry-uq-frame".to_owned(),
        contact_boundary_digest: "sha256:synthetic-geometry-uq-boundary".to_owned(),
        wrench_origin_m: Vec3::default(),
        taxels: vec![
            TaxelGeometry {
                taxel_id: "taxel-0".to_owned(),
                area_m2: 1.0e-4,
                application_point_m: Vec3 {
                    x: 0.02,
                    y: 0.0,
                    z: 0.0,
                },
                normal,
                tangent_u,
                tangent_v,
            },
            TaxelGeometry {
                taxel_id: "taxel-1".to_owned(),
                area_m2: 1.0e-4,
                application_point_m: Vec3 {
                    x: -0.02,
                    y: 0.0,
                    z: 0.0,
                },
                normal,
                tangent_u,
                tangent_v,
            },
        ],
    };
    let mut pose_covariance = vec![0.0; 36];
    for axis in 0..6 {
        pose_covariance[axis * 6 + axis] = if axis < 3 { 1.0e-8 } else { 1.0e-6 };
    }
    let model = GeometryUncertaintyModel {
        geometry_digest: geometry.geometry_digest.clone(),
        reference_frame_digest: geometry.reference_frame_digest.clone(),
        contact_boundary_digest: geometry.contact_boundary_digest.clone(),
        transform_digest: "sha256:synthetic-geometry-uq-transform".to_owned(),
        sample_identity: "sample:synthetic-geometry-uq".to_owned(),
        native_observation_set_digest: "sha256:synthetic-geometry-uq-native".to_owned(),
        field_estimate_digest: "sha256:synthetic-geometry-uq-field".to_owned(),
        uncertainty_method_digest: "sha256:synthetic-geometry-uq-method".to_owned(),
        reference_ancestry_receipt_id: "receipt:geometry-ancestry".to_owned(),
        common_cause_analysis_receipt_id: "receipt:geometry-cca".to_owned(),
        local_taxels: geometry
            .taxels
            .iter()
            .map(|taxel| TaxelGeometryStandardUncertainty {
                taxel_id: taxel.taxel_id.clone(),
                area_m2: 1.0e-6,
                application_point_m: Vec3 {
                    x: 1.0e-3,
                    y: 1.0e-3,
                    z: 1.0e-3,
                },
                orientation_rad: Vec3 {
                    x: 1.0e-3,
                    y: 1.0e-3,
                    z: 1.0e-3,
                },
            })
            .collect(),
        local_taxel_relationship: GeometrySourceRelationship::IndependentlyCharacterized,
        common_rigid_pose_covariance_row_major: pose_covariance,
        local_to_global_relationship: GeometrySourceRelationship::IndependentlyCharacterized,
        geometry_to_field_relationship: GeometrySourceRelationship::IndependentlyCharacterized,
        geometry_to_wrench_reference_relationship:
            GeometrySourceRelationship::IndependentlyCharacterized,
        wrench_origin_attachment: WrenchOriginAttachment::FixedInReferenceFrame,
    };
    let field = TractionFieldEstimate {
        estimate_digest: "sha256:synthetic-geometry-uq-field".to_owned(),
        sample_identity: "sample:synthetic-geometry-uq".to_owned(),
        native_observation_set_digest: "sha256:synthetic-geometry-uq-native".to_owned(),
        local_traction_pa: vec![10_000.0, 1_000.0, 500.0, 8_000.0, 500.0, 250.0],
        standard_uncertainties_pa: vec![100.0; 6],
        uncertainty_method_digest: "sha256:synthetic-field-uq-method".to_owned(),
        uncertainty_reference_ancestry_receipt_id: "receipt:field-ancestry".to_owned(),
        uncertainty_common_cause_analysis_receipt_id: "receipt:field-cca".to_owned(),
        covariance_structure: FieldCovarianceStructure::IndependentDiagonalComponents,
        relationship_to_wrench_reference:
            FieldWrenchReferenceRelationship::IndependentFromWrenchReference,
    };
    propagate_geometry_wrench_uncertainty(&geometry, &field, &model)
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct GeometryUqMonteCarloReport {
    pub replicate_count: usize,
    pub first_seed: u64,
    pub linearized_covariance_row_major: Vec<f64>,
    pub empirical_covariance_row_major: Vec<f64>,
    pub empirical_mean_bias_from_nominal: [f64; 6],
    pub relative_covariance_frobenius_error: f64,
    pub claim_class: &'static str,
}

fn next_random_u64(state: &mut u64) -> u64 {
    let mut value = *state;
    value ^= value << 13;
    value ^= value >> 7;
    value ^= value << 17;
    *state = value.max(1);
    *state
}

fn uniform_open01(state: &mut u64) -> f64 {
    ((next_random_u64(state) >> 11) as f64 + 0.5) / ((1_u64 << 53) as f64)
}

fn standard_normal(state: &mut u64) -> f64 {
    let u1 = uniform_open01(state);
    let u2 = uniform_open01(state);
    (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos()
}

fn cholesky_lower_6(covariance: &[[f64; 6]; 6]) -> Result<[[f64; 6]; 6], String> {
    let mut lower = [[0.0; 6]; 6];
    for row in 0..6 {
        for column in 0..=row {
            let correction = (0..column)
                .map(|index| lower[row][index] * lower[column][index])
                .sum::<f64>();
            if row == column {
                let diagonal = covariance[row][row] - correction;
                if !diagonal.is_finite() || diagonal <= 0.0 {
                    return Err("Monte Carlo rigid-pose covariance is not positive definite".to_owned());
                }
                lower[row][column] = diagonal.sqrt();
            } else {
                lower[row][column] =
                    (covariance[row][column] - correction) / lower[column][column];
            }
        }
    }
    Ok(lower)
}

fn rotate_small_angle(vector: Vec3, rotation_vector: Vec3) -> Vec3 {
    let angle = (rotation_vector.x * rotation_vector.x
        + rotation_vector.y * rotation_vector.y
        + rotation_vector.z * rotation_vector.z)
        .sqrt();
    if angle <= 1.0e-15 {
        return vector;
    }
    let axis = vec_scale(rotation_vector, 1.0 / angle);
    let cosine = angle.cos();
    let sine = angle.sin();
    vec_add(
        vec_add(
            vec_scale(vector, cosine),
            vec_scale(vec_cross(axis, vector), sine),
        ),
        vec_scale(axis, axis.x.mul_add(vector.x, axis.y.mul_add(vector.y, axis.z * vector.z)) * (1.0 - cosine)),
    )
}

fn perturb_geometry_once(
    geometry: &TaxelGeometrySet,
    model: &GeometryUncertaintyModel,
    pose_lower: &[[f64; 6]; 6],
    state: &mut u64,
) -> Result<TaxelGeometrySet, String> {
    let mut sampled = geometry.clone();
    for (taxel, uncertainty) in sampled.taxels.iter_mut().zip(&model.local_taxels) {
        let area = taxel.area_m2 + uncertainty.area_m2 * standard_normal(state);
        if !area.is_finite() || area <= 0.0 {
            return Err("sampled taxel area became nonpositive; uncertainty exceeds the valid small-error domain".to_owned());
        }
        taxel.area_m2 = area;
        taxel.application_point_m = vec_add(
            taxel.application_point_m,
            Vec3 {
                x: uncertainty.application_point_m.x * standard_normal(state),
                y: uncertainty.application_point_m.y * standard_normal(state),
                z: uncertainty.application_point_m.z * standard_normal(state),
            },
        );
        let local_rotation = Vec3 {
            x: uncertainty.orientation_rad.x * standard_normal(state),
            y: uncertainty.orientation_rad.y * standard_normal(state),
            z: uncertainty.orientation_rad.z * standard_normal(state),
        };
        taxel.normal = rotate_small_angle(taxel.normal, local_rotation);
        taxel.tangent_u = rotate_small_angle(taxel.tangent_u, local_rotation);
        taxel.tangent_v = rotate_small_angle(taxel.tangent_v, local_rotation);
    }

    let independent = std::array::from_fn::<_, 6, _>(|_| standard_normal(state));
    let mut pose = [0.0; 6];
    for row in 0..6 {
        pose[row] = (0..=row)
            .map(|column| pose_lower[row][column] * independent[column])
            .sum();
    }
    let translation = Vec3 {
        x: pose[0],
        y: pose[1],
        z: pose[2],
    };
    let rotation = Vec3 {
        x: pose[3],
        y: pose[4],
        z: pose[5],
    };
    for taxel in &mut sampled.taxels {
        taxel.application_point_m = vec_add(
            rotate_small_angle(taxel.application_point_m, rotation),
            translation,
        );
        taxel.normal = rotate_small_angle(taxel.normal, rotation);
        taxel.tangent_u = rotate_small_angle(taxel.tangent_u, rotation);
        taxel.tangent_v = rotate_small_angle(taxel.tangent_v, rotation);
    }
    if model.wrench_origin_attachment == WrenchOriginAttachment::SensorBodyAttached {
        sampled.wrench_origin_m = vec_add(
            rotate_small_angle(sampled.wrench_origin_m, rotation),
            translation,
        );
    }
    Ok(sampled)
}

/// Compare the first-order geometry covariance with an empirical covariance
/// from seeded Monte Carlo perturbations of independent local geometry and a
/// common correlated rigid pose. This is an evaluator oracle for the stated
/// small-uncertainty model, not a physical calibration result.
pub fn validate_geometry_uq_against_monte_carlo(
    geometry: &TaxelGeometrySet,
    field: &TractionFieldEstimate,
    model: &GeometryUncertaintyModel,
    first_seed: u64,
    replicate_count: usize,
) -> Result<GeometryUqMonteCarloReport, String> {
    if first_seed == 0 {
        return Err("geometry Monte Carlo seed must be nonzero".to_owned());
    }
    if replicate_count < 2_000 {
        return Err("geometry Monte Carlo validation requires at least 2,000 replicates".to_owned());
    }
    let analytic = propagate_geometry_wrench_uncertainty(geometry, field, model)?;
    if !analytic.admissible_for_combined_wrench_uncertainty {
        return Err("geometry Monte Carlo comparison requires independently admissible covariance sources".to_owned());
    }
    let linearized_row_major = analytic
        .total_geometry_covariance_row_major
        .ok_or_else(|| "linearized total geometry covariance is unavailable".to_owned())?;
    let linearized: [[f64; 6]; 6] = std::array::from_fn(|row| {
        std::array::from_fn(|column| linearized_row_major[row * 6 + column])
    });
    let pose_covariance = validate_spd_6(&model.common_rigid_pose_covariance_row_major)?;
    let pose_lower = cholesky_lower_6(&pose_covariance)?;
    let nominal = analytic
        .nominal_wrench
        .ok_or_else(|| "nominal wrench missing from geometry uncertainty report".to_owned())?;
    let nominal_values = [
        nominal.force_n.x,
        nominal.force_n.y,
        nominal.force_n.z,
        nominal.moment_nm.x,
        nominal.moment_nm.y,
        nominal.moment_nm.z,
    ];

    let mut state = first_seed;
    let mut mean = [0.0; 6];
    let mut sum_outer = [[0.0; 6]; 6];
    for sample_index in 0..replicate_count {
        let perturbed = perturb_geometry_once(geometry, model, &pose_lower, &mut state)?;
        let operator = build_wrench_operator(&perturbed)?;
        let wrench = integrate_tactile_field(&operator, &field.local_traction_pa)?;
        let values = [
            wrench.force_n.x,
            wrench.force_n.y,
            wrench.force_n.z,
            wrench.moment_nm.x,
            wrench.moment_nm.y,
            wrench.moment_nm.z,
        ];
        let count = (sample_index + 1) as f64;
        let mut delta = [0.0; 6];
        let mut delta_after = [0.0; 6];
        for component in 0..6 {
            delta[component] = values[component] - mean[component];
            mean[component] += delta[component] / count;
            delta_after[component] = values[component] - mean[component];
        }
        for row in 0..6 {
            for column in 0..6 {
                sum_outer[row][column] += delta[row] * delta_after[column];
            }
        }
    }
    let empirical: [[f64; 6]; 6] = std::array::from_fn(|row| {
        std::array::from_fn(|column| {
            sum_outer[row][column] / (replicate_count - 1) as f64
        })
    });
    let mut squared_difference = 0.0;
    let mut squared_linearized = 0.0;
    for row in 0..6 {
        for column in 0..6 {
            squared_difference += (empirical[row][column] - linearized[row][column]).powi(2);
            squared_linearized += linearized[row][column].powi(2);
        }
    }
    let denominator = squared_linearized.sqrt();
    if !denominator.is_finite() || denominator <= f64::MIN_POSITIVE {
        return Err("linearized geometry covariance has zero or non-finite Frobenius norm".to_owned());
    }
    let mean_bias_from_nominal =
        std::array::from_fn(|index| mean[index] - nominal_values[index]);
    Ok(GeometryUqMonteCarloReport {
        replicate_count,
        first_seed,
        linearized_covariance_row_major: matrix_to_row_major(&linearized),
        empirical_covariance_row_major: matrix_to_row_major(&empirical),
        empirical_mean_bias_from_nominal: mean_bias_from_nominal,
        relative_covariance_frobenius_error: squared_difference.sqrt() / denominator,
        claim_class: "synthetic_geometry_uq_monte_carlo_validation_only",
    })
}

/// Deterministic 20,000-draw Monte Carlo comparison for the CLI. The output
/// measures covariance agreement for this synthetic small-error scenario only.
pub fn run_synthetic_geometry_uq_monte_carlo_example(
) -> Result<GeometryUqMonteCarloReport, String> {
    let tangent_u = Vec3 { x: 1.0, y: 0.0, z: 0.0 };
    let tangent_v = Vec3 { x: 0.0, y: 1.0, z: 0.0 };
    let normal = Vec3 { x: 0.0, y: 0.0, z: 1.0 };
    let geometry = TaxelGeometrySet {
        geometry_digest: "sha256:synthetic-geometry-mc-grid".to_owned(),
        reference_frame_digest: "sha256:synthetic-geometry-mc-frame".to_owned(),
        contact_boundary_digest: "sha256:synthetic-geometry-mc-boundary".to_owned(),
        wrench_origin_m: Vec3::default(),
        taxels: vec![
            TaxelGeometry {
                taxel_id: "taxel-0".to_owned(),
                area_m2: 1.0e-4,
                application_point_m: Vec3 { x: 0.02, y: 0.0, z: 0.0 },
                normal,
                tangent_u,
                tangent_v,
            },
            TaxelGeometry {
                taxel_id: "taxel-1".to_owned(),
                area_m2: 1.0e-4,
                application_point_m: Vec3 { x: -0.02, y: 0.0, z: 0.0 },
                normal,
                tangent_u,
                tangent_v,
            },
        ],
    };
    let field = TractionFieldEstimate {
        estimate_digest: "sha256:synthetic-geometry-mc-field".to_owned(),
        sample_identity: "sample:synthetic-geometry-mc".to_owned(),
        native_observation_set_digest: "sha256:synthetic-geometry-mc-native".to_owned(),
        local_traction_pa: vec![10_000.0, 1_000.0, 500.0, 8_000.0, 500.0, 250.0],
        standard_uncertainties_pa: vec![100.0; 6],
        uncertainty_method_digest: "sha256:synthetic-geometry-mc-field-uq".to_owned(),
        uncertainty_reference_ancestry_receipt_id: "receipt:field-ancestry".to_owned(),
        uncertainty_common_cause_analysis_receipt_id: "receipt:field-cca".to_owned(),
        covariance_structure: FieldCovarianceStructure::IndependentDiagonalComponents,
        relationship_to_wrench_reference: FieldWrenchReferenceRelationship::IndependentFromWrenchReference,
    };
    let mut pose_covariance = vec![0.0; 36];
    for axis in 0..6 {
        pose_covariance[axis * 6 + axis] = if axis < 3 { 1.0e-8 } else { 1.0e-6 };
    }
    let model = GeometryUncertaintyModel {
        geometry_digest: geometry.geometry_digest.clone(),
        reference_frame_digest: geometry.reference_frame_digest.clone(),
        contact_boundary_digest: geometry.contact_boundary_digest.clone(),
        transform_digest: "sha256:synthetic-geometry-mc-transform".to_owned(),
        sample_identity: field.sample_identity.clone(),
        native_observation_set_digest: field.native_observation_set_digest.clone(),
        field_estimate_digest: field.estimate_digest.clone(),
        uncertainty_method_digest: "sha256:synthetic-geometry-mc-method".to_owned(),
        reference_ancestry_receipt_id: "receipt:geometry-ancestry".to_owned(),
        common_cause_analysis_receipt_id: "receipt:geometry-cca".to_owned(),
        local_taxels: geometry.taxels
            .iter()
            .map(|taxel| TaxelGeometryStandardUncertainty {
                taxel_id: taxel.taxel_id.clone(),
                area_m2: 1.0e-6,
                application_point_m: Vec3 { x: 1.0e-3, y: 1.0e-3, z: 1.0e-3 },
                orientation_rad: Vec3 { x: 1.0e-3, y: 1.0e-3, z: 1.0e-3 },
            })
            .collect(),
        local_taxel_relationship: GeometrySourceRelationship::IndependentlyCharacterized,
        common_rigid_pose_covariance_row_major: pose_covariance,
        local_to_global_relationship: GeometrySourceRelationship::IndependentlyCharacterized,
        geometry_to_field_relationship: GeometrySourceRelationship::IndependentlyCharacterized,
        geometry_to_wrench_reference_relationship: GeometrySourceRelationship::IndependentlyCharacterized,
        wrench_origin_attachment: WrenchOriginAttachment::FixedInReferenceFrame,
    };
    validate_geometry_uq_against_monte_carlo(&geometry, &field, &model, 20261117, 20_000)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn geometry() -> TaxelGeometrySet {
        let tangent_u = Vec3 {
            x: 1.0,
            y: 0.0,
            z: 0.0,
        };
        let tangent_v = Vec3 {
            x: 0.0,
            y: 1.0,
            z: 0.0,
        };
        let normal = Vec3 {
            x: 0.0,
            y: 0.0,
            z: 1.0,
        };
        TaxelGeometrySet {
            geometry_digest: "sha256:geometry-1".to_owned(),
            reference_frame_digest: "sha256:reference-frame".to_owned(),
            contact_boundary_digest: "sha256:boundary".to_owned(),
            wrench_origin_m: Vec3::default(),
            taxels: vec![super::super::wrench_consistency::TaxelGeometry {
                taxel_id: "taxel-0".to_owned(),
                area_m2: 1.0e-4,
                application_point_m: Vec3 {
                    x: 0.1,
                    y: 0.0,
                    z: 0.0,
                },
                normal,
                tangent_u,
                tangent_v,
            }],
        }
    }

    fn field_estimate(local_traction_pa: Vec<f64>) -> TractionFieldEstimate {
        TractionFieldEstimate {
            estimate_digest: "sha256:field-estimate".to_owned(),
            sample_identity: "sample:1".to_owned(),
            native_observation_set_digest: "sha256:native-observations".to_owned(),
            standard_uncertainties_pa: vec![1.0; local_traction_pa.len()],
            local_traction_pa,
            uncertainty_method_digest: "sha256:field-method".to_owned(),
            uncertainty_reference_ancestry_receipt_id: "receipt:field-ancestry".to_owned(),
            uncertainty_common_cause_analysis_receipt_id: "receipt:field-cca".to_owned(),
            covariance_structure: super::super::wrench_consistency::FieldCovarianceStructure::IndependentDiagonalComponents,
            relationship_to_wrench_reference: FieldWrenchReferenceRelationship::IndependentFromWrenchReference,
        }
    }

    fn model(geometry: &TaxelGeometrySet) -> GeometryUncertaintyModel {
        let mut pose_covariance = vec![0.0; 36];
        for axis in 0..6 {
            pose_covariance[axis * 6 + axis] = if axis < 3 { 1.0e-8 } else { 1.0e-6 };
        }
        GeometryUncertaintyModel {
            geometry_digest: geometry.geometry_digest.clone(),
            reference_frame_digest: geometry.reference_frame_digest.clone(),
            contact_boundary_digest: geometry.contact_boundary_digest.clone(),
            transform_digest: "sha256:transform".to_owned(),
            sample_identity: "sample:1".to_owned(),
            native_observation_set_digest: "sha256:native-observations".to_owned(),
            field_estimate_digest: "sha256:field-estimate".to_owned(),
            uncertainty_method_digest: "sha256:geometry-uncertainty-method".to_owned(),
            reference_ancestry_receipt_id: "receipt:geometry-reference-ancestry".to_owned(),
            common_cause_analysis_receipt_id: "receipt:geometry-cca".to_owned(),
            local_taxels: vec![TaxelGeometryStandardUncertainty {
                taxel_id: "taxel-0".to_owned(),
                area_m2: 1.0e-6,
                application_point_m: Vec3 {
                    x: 1.0e-3,
                    y: 1.0e-3,
                    z: 1.0e-3,
                },
                orientation_rad: Vec3 {
                    x: 1.0e-3,
                    y: 1.0e-3,
                    z: 1.0e-3,
                },
            }],
            local_taxel_relationship: GeometrySourceRelationship::IndependentlyCharacterized,
            common_rigid_pose_covariance_row_major: pose_covariance,
            local_to_global_relationship: GeometrySourceRelationship::IndependentlyCharacterized,
            geometry_to_field_relationship: GeometrySourceRelationship::IndependentlyCharacterized,
            geometry_to_wrench_reference_relationship:
                GeometrySourceRelationship::IndependentlyCharacterized,
            wrench_origin_attachment: WrenchOriginAttachment::FixedInReferenceFrame,
        }
    }

    #[test]
    fn geometry_uncertainty_produces_local_and_common_pose_covariance() {
        let geometry = geometry();
        let field = field_estimate(vec![10_000.0, 1_000.0, 500.0]);
        let report = propagate_geometry_wrench_uncertainty(&geometry, &field, &model(&geometry))
            .expect("geometry uncertainty propagation");
        assert_eq!(
            report.disposition,
            GeometryUqDisposition::IndependentGeometryCovarianceAvailable
        );
        assert!(report.admissible_for_combined_wrench_uncertainty);
        assert_eq!(
            report
                .local_geometry_covariance_row_major
                .as_ref()
                .unwrap()
                .len(),
            36
        );
        assert_eq!(
            report
                .global_pose_covariance_row_major
                .as_ref()
                .unwrap()
                .len(),
            36
        );
        assert_eq!(
            report
                .total_geometry_covariance_row_major
                .as_ref()
                .unwrap()
                .len(),
            36
        );
        assert!(
            report
                .local_geometry_covariance_row_major
                .unwrap()
                .iter()
                .any(|value| *value > 0.0)
        );
        assert!(
            report
                .global_pose_covariance_row_major
                .unwrap()
                .iter()
                .any(|value| *value > 0.0)
        );
    }

    #[test]
    fn common_translation_affects_fixed_reference_moment_but_not_body_attached_moment() {
        let geometry = geometry();
        let field = field_estimate(vec![10_000.0, 0.0, 0.0]);
        let mut fixed = model(&geometry);
        let mut attached = fixed.clone();
        attached.wrench_origin_attachment = WrenchOriginAttachment::SensorBodyAttached;
        // Keep only translation covariance, with small positive rotation
        // variances to satisfy the positive-definite covariance contract.
        for row in 0..6 {
            for column in 0..6 {
                fixed.common_rigid_pose_covariance_row_major[row * 6 + column] = 0.0;
            }
        }
        for axis in 0..3 {
            fixed.common_rigid_pose_covariance_row_major[axis * 6 + axis] = 1.0e-6;
            fixed.common_rigid_pose_covariance_row_major[(axis + 3) * 6 + axis + 3] = 1.0e-12;
        }
        attached.common_rigid_pose_covariance_row_major =
            fixed.common_rigid_pose_covariance_row_major.clone();

        let fixed_report =
            propagate_geometry_wrench_uncertainty(&geometry, &field, &fixed).unwrap();
        let attached_report =
            propagate_geometry_wrench_uncertainty(&geometry, &field, &attached).unwrap();
        let fixed_cov = fixed_report.global_pose_covariance_row_major.unwrap();
        let attached_cov = attached_report.global_pose_covariance_row_major.unwrap();
        assert!(fixed_cov[4 * 6 + 4] > 0.0);
        assert_eq!(attached_cov[4 * 6 + 4], 0.0);
    }

    #[test]
    fn unresolved_cross_source_relationships_do_not_admit_combined_covariance() {
        let geometry = geometry();
        let field = field_estimate(vec![10_000.0, 1_000.0, 500.0]);
        let mut evidence = model(&geometry);
        evidence.geometry_to_field_relationship = GeometrySourceRelationship::SharedOrUnknown;
        evidence.geometry_to_wrench_reference_relationship =
            GeometrySourceRelationship::SharedOrUnknown;
        let report = propagate_geometry_wrench_uncertainty(&geometry, &field, &evidence).unwrap();
        assert_eq!(
            report.disposition,
            GeometryUqDisposition::GeometryCovarianceAvailableButCrossSourcesUnresolved
        );
        assert!(!report.admissible_for_combined_wrench_uncertainty);
        assert!(report.total_geometry_covariance_row_major.is_some());
    }

    #[test]
    fn local_taxel_correlations_and_local_global_cross_terms_fail_closed() {
        let geometry = geometry();
        let field = field_estimate(vec![10_000.0, 1_000.0, 500.0]);
        let mut local_correlated = model(&geometry);
        local_correlated.local_taxel_relationship = GeometrySourceRelationship::SharedOrUnknown;
        let report =
            propagate_geometry_wrench_uncertainty(&geometry, &field, &local_correlated).unwrap();
        assert_eq!(
            report.disposition,
            GeometryUqDisposition::LocalGeometryCorrelationUnresolved
        );
        assert!(report.local_geometry_covariance_row_major.is_none());
        assert!(report.total_geometry_covariance_row_major.is_none());

        let mut local_global = model(&geometry);
        local_global.local_to_global_relationship = GeometrySourceRelationship::SharedOrUnknown;
        let report =
            propagate_geometry_wrench_uncertainty(&geometry, &field, &local_global).unwrap();
        assert_eq!(
            report.disposition,
            GeometryUqDisposition::LocalGlobalGeometryCorrelationUnresolved
        );
        assert!(report.total_geometry_covariance_row_major.is_none());
    }

    fn wrench_components(geometry: &TaxelGeometrySet, field: &[f64]) -> [f64; 6] {
        let operator = build_wrench_operator(geometry).unwrap();
        let wrench = integrate_tactile_field(&operator, field).unwrap();
        [
            wrench.force_n.x,
            wrench.force_n.y,
            wrench.force_n.z,
            wrench.moment_nm.x,
            wrench.moment_nm.y,
            wrench.moment_nm.z,
        ]
    }

    fn finite_difference(
        geometry: &TaxelGeometrySet,
        field: &[f64],
        perturb: impl Fn(&mut TaxelGeometrySet, f64),
        epsilon: f64,
    ) -> [f64; 6] {
        let mut plus = geometry.clone();
        let mut minus = geometry.clone();
        perturb(&mut plus, epsilon);
        perturb(&mut minus, -epsilon);
        let plus_wrench = wrench_components(&plus, field);
        let minus_wrench = wrench_components(&minus, field);
        std::array::from_fn(|index| (plus_wrench[index] - minus_wrench[index]) / (2.0 * epsilon))
    }

    fn covariance_from_single_local_parameter(
        geometry: &TaxelGeometrySet,
        field: &TractionFieldEstimate,
        set_sigma: impl Fn(&mut GeometryUncertaintyModel, f64),
        perturb: impl Fn(&mut TaxelGeometrySet, f64),
        epsilon: f64,
        sigma: f64,
    ) -> [[f64; 6]; 6] {
        let mut uncertainty = model(geometry);
        for value in &mut uncertainty.common_rigid_pose_covariance_row_major {
            *value = 0.0;
        }
        for axis in 0..6 {
            uncertainty.common_rigid_pose_covariance_row_major[axis * 6 + axis] = 1.0e-24;
        }
        uncertainty.local_taxels[0].area_m2 = 0.0;
        uncertainty.local_taxels[0].application_point_m = Vec3::default();
        uncertainty.local_taxels[0].orientation_rad = Vec3::default();
        set_sigma(&mut uncertainty, sigma);
        let report = propagate_geometry_wrench_uncertainty(geometry, field, &uncertainty).unwrap();
        let row_major = report.local_geometry_covariance_row_major.unwrap();
        let analytic: [[f64; 6]; 6] =
            std::array::from_fn(|row| std::array::from_fn(|column| row_major[row * 6 + column]));
        let derivative = finite_difference(geometry, &field.local_traction_pa, perturb, epsilon);
        for row in 0..6 {
            for column in 0..6 {
                let expected = derivative[row] * derivative[column] * sigma * sigma;
                assert!(
                    (analytic[row][column] - expected).abs()
                        < expected.abs().max(1.0e-15) * 1.0e-5 + 1.0e-15,
                    "local geometry Jacobian mismatch at ({row}, {column}): analytic={}, finite-difference={expected}",
                    analytic[row][column]
                );
            }
        }
        analytic
    }

    #[test]
    fn local_area_position_and_basis_jacobians_match_central_finite_difference() {
        let geometry = geometry();
        let field = field_estimate(vec![10_000.0, 1_000.0, 500.0]);

        let _area = covariance_from_single_local_parameter(
            &geometry,
            &field,
            |model, sigma| model.local_taxels[0].area_m2 = sigma,
            |geometry, delta| geometry.taxels[0].area_m2 += delta,
            1.0e-8,
            1.0e-6,
        );

        let position = covariance_from_single_local_parameter(
            &geometry,
            &field,
            |model, sigma| model.local_taxels[0].application_point_m.x = sigma,
            |geometry, delta| geometry.taxels[0].application_point_m.x += delta,
            1.0e-6,
            1.0e-3,
        );
        assert!(position[4][4] > 0.0);

        let orientation = covariance_from_single_local_parameter(
            &geometry,
            &field,
            |model, sigma| model.local_taxels[0].orientation_rad.z = sigma,
            |geometry, delta| {
                let cosine = delta.cos();
                let sine = delta.sin();
                let tangent_u = geometry.taxels[0].tangent_u;
                let tangent_v = geometry.taxels[0].tangent_v;
                geometry.taxels[0].tangent_u = Vec3 {
                    x: cosine * tangent_u.x - sine * tangent_u.y,
                    y: sine * tangent_u.x + cosine * tangent_u.y,
                    z: tangent_u.z,
                };
                geometry.taxels[0].tangent_v = Vec3 {
                    x: cosine * tangent_v.x - sine * tangent_v.y,
                    y: sine * tangent_v.x + cosine * tangent_v.y,
                    z: tangent_v.z,
                };
            },
            1.0e-6,
            1.0e-3,
        );
        assert!(orientation[0][0] > 0.0);
    }

    #[test]
    fn shared_global_pose_covariance_produces_correlated_wrench_components() {
        let geometry = geometry();
        let field = field_estimate(vec![10_000.0, 0.0, 0.0]);
        let mut uncertainty = model(&geometry);
        // Correlate x/y translation errors. For a normal force, these produce
        // coupled My/Mx errors about a fixed external origin.
        uncertainty.common_rigid_pose_covariance_row_major[0] = 1.0e-6;
        uncertainty.common_rigid_pose_covariance_row_major[7] = 1.0e-6;
        uncertainty.common_rigid_pose_covariance_row_major[1] = 0.5e-6;
        uncertainty.common_rigid_pose_covariance_row_major[6] = 0.5e-6;
        let report = propagate_geometry_wrench_uncertainty(&geometry, &field, &uncertainty)
            .expect("correlated global pose covariance");
        let covariance = report.global_pose_covariance_row_major.unwrap();
        assert!(covariance[3 * 6 + 4].abs() > 0.0);
        assert!((covariance[3 * 6 + 4] - covariance[4 * 6 + 3]).abs() < 1.0e-15);
    }

    #[test]
    fn field_sample_and_native_observation_binding_mismatches_fail_closed() {
        let geometry = geometry();
        let model = model(&geometry);
        let valid_field = field_estimate(vec![10_000.0, 1_000.0, 500.0]);

        let mut wrong_estimate = valid_field.clone();
        wrong_estimate.estimate_digest = "sha256:other-field".to_owned();
        assert!(propagate_geometry_wrench_uncertainty(&geometry, &wrong_estimate, &model).is_err());

        let mut wrong_sample = valid_field.clone();
        wrong_sample.sample_identity = "sample:other".to_owned();
        assert!(propagate_geometry_wrench_uncertainty(&geometry, &wrong_sample, &model).is_err());

        let mut wrong_native = valid_field;
        wrong_native.native_observation_set_digest = "sha256:other-native".to_owned();
        assert!(propagate_geometry_wrench_uncertainty(&geometry, &wrong_native, &model).is_err());
    }

    #[test]
    fn correlated_field_covariance_blocks_combined_independence_claim() {
        let geometry = geometry();
        let mut field = field_estimate(vec![10_000.0, 1_000.0, 500.0]);
        field.covariance_structure = FieldCovarianceStructure::CorrelatedOrUnknown;
        let report =
            propagate_geometry_wrench_uncertainty(&geometry, &field, &model(&geometry)).unwrap();
        assert!(!report.admissible_for_combined_wrench_uncertainty);
        assert!(report.total_geometry_covariance_row_major.is_some());
        assert!(report
            .reasons
            .iter()
            .any(|reason| reason.contains("traction-field covariance is correlated or unknown")));
    }

    #[test]
    fn seeded_monte_carlo_matches_first_order_geometry_covariance_in_small_error_regime() {
        let geometry = geometry();
        let field = field_estimate(vec![10_000.0, 1_000.0, 500.0]);
        let model = model(&geometry);
        let report = validate_geometry_uq_against_monte_carlo(
            &geometry,
            &field,
            &model,
            20261117,
            20_000,
        )
        .expect("Monte Carlo geometry covariance validation");
        assert_eq!(report.replicate_count, 20_000);
        assert!(report.relative_covariance_frobenius_error < 0.12);
        assert_eq!(report.linearized_covariance_row_major.len(), 36);
        assert_eq!(report.empirical_covariance_row_major.len(), 36);
        assert!(report
            .empirical_mean_bias_from_nominal
            .iter()
            .all(|value| value.is_finite()));
    }

    #[test]
    fn geometry_monte_carlo_rejects_insufficient_replicates_and_invalid_seed() {
        let geometry = geometry();
        let field = field_estimate(vec![10_000.0, 1_000.0, 500.0]);
        let model = model(&geometry);
        assert!(validate_geometry_uq_against_monte_carlo(
            &geometry,
            &field,
            &model,
            1,
            1_999,
        )
        .is_err());
        assert!(validate_geometry_uq_against_monte_carlo(
            &geometry,
            &field,
            &model,
            0,
            2_000,
        )
        .is_err());
    }

    #[test]
    fn geometry_context_and_parameter_validation_fail_closed() {
        let geometry = geometry();
        let field = field_estimate(vec![10_000.0, 1_000.0, 500.0]);
        let mut wrong_digest = model(&geometry);
        wrong_digest.geometry_digest = "sha256:stale-geometry".to_owned();
        assert!(propagate_geometry_wrench_uncertainty(&geometry, &field, &wrong_digest).is_err());

        let mut negative_sigma = model(&geometry);
        negative_sigma.local_taxels[0].orientation_rad.z = -1.0;
        assert!(propagate_geometry_wrench_uncertainty(&geometry, &field, &negative_sigma).is_err());

        let mut bad_pose = model(&geometry);
        bad_pose.common_rigid_pose_covariance_row_major[1] = 1.0;
        assert!(propagate_geometry_wrench_uncertainty(&geometry, &field, &bad_pose).is_err());

        let short_field = field_estimate(vec![1.0]);
        assert!(
            propagate_geometry_wrench_uncertainty(&geometry, &short_field, &model(&geometry))
                .is_err()
        );
    }
}
