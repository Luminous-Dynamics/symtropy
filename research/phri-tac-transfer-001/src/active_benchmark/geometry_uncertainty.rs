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
    TaxelGeometrySet, Vec3, WrenchVector, build_wrench_operator, integrate_tactile_field,
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
        0 => Vec3 { x: 1.0, y: 0.0, z: 0.0 },
        1 => Vec3 { x: 0.0, y: 1.0, z: 0.0 },
        _ => Vec3 { x: 0.0, y: 0.0, z: 1.0 },
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
        return Err("common rigid-pose covariance must contain 36 finite row-major values".to_owned());
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
                lower[row][column] =
                    (matrix[row][column] - correction) / lower[column][column];
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
        ("reference_frame_digest", model.reference_frame_digest.as_str()),
        ("contact_boundary_digest", model.contact_boundary_digest.as_str()),
        ("transform_digest", model.transform_digest.as_str()),
        ("sample_identity", model.sample_identity.as_str()),
        ("field_estimate_digest", model.field_estimate_digest.as_str()),
        ("uncertainty_method_digest", model.uncertainty_method_digest.as_str()),
        ("reference_ancestry_receipt_id", model.reference_ancestry_receipt_id.as_str()),
        ("common_cause_analysis_receipt_id", model.common_cause_analysis_receipt_id.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(format!("geometry uncertainty field {name} is missing"));
        }
    }
    if model.geometry_digest != geometry.geometry_digest
        || model.reference_frame_digest != geometry.reference_frame_digest
        || model.contact_boundary_digest != geometry.contact_boundary_digest
    {
        return Err("geometry uncertainty digests do not match the exact geometry/frame/contact boundary".to_owned());
    }
    if model.local_taxels.len() != geometry.taxels.len() {
        return Err("local geometry uncertainty must contain exactly one entry per taxel".to_owned());
    }
    for (taxel, uncertainty) in geometry.taxels.iter().zip(&model.local_taxels) {
        if taxel.taxel_id != uncertainty.taxel_id {
            return Err("local geometry uncertainty taxel identity/order does not match geometry".to_owned());
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
            return Err(format!("taxel {} geometry standard uncertainties must be finite and nonnegative", taxel.taxel_id));
        }
    }
    validate_spd_6(&model.common_rigid_pose_covariance_row_major)
}

/// Propagate first-order local taxel geometry and common rigid-pose uncertainty
/// to the integrated wrench. Field and geometry means are unchanged.
pub fn propagate_geometry_wrench_uncertainty(
    geometry: &TaxelGeometrySet,
    local_traction_pa: &[f64],
    model: &GeometryUncertaintyModel,
) -> Result<GeometryWrenchUncertaintyReport, String> {
    let pose_covariance = validate_model(geometry, model)?;
    let operator = build_wrench_operator(geometry)?;
    let nominal = integrate_tactile_field(&operator, local_traction_pa)?;

    let mut local_covariance = [[0.0; 6]; 6];
    for (taxel, uncertainty) in geometry.taxels.iter().zip(&model.local_taxels) {
        let parameter = geometry.taxels.iter().position(|candidate| candidate.taxel_id == taxel.taxel_id)
            .ok_or_else(|| "taxel identity could not be resolved".to_owned())?;
        let offset = parameter * 3;
        let normal_pressure = local_traction_pa[offset];
        let tangent_u = local_traction_pa[offset + 1];
        let tangent_v = local_traction_pa[offset + 2];
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
        outer_add(&mut local_covariance, &area_jacobian, uncertainty.area_m2.powi(2));

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
        && model.local_to_global_relationship == GeometrySourceRelationship::IndependentlyCharacterized;
    let total = total_geometry_covariance_available
        .then(|| matrix_add(&local_covariance, &global_covariance));
    let combined_independence = total_geometry_covariance_available
        && model.geometry_to_field_relationship == GeometrySourceRelationship::IndependentlyCharacterized
        && model.geometry_to_wrench_reference_relationship == GeometrySourceRelationship::IndependentlyCharacterized;

    let mut reasons = Vec::new();
    if !local_covariance_available {
        reasons.push("per-taxel local geometry errors are correlated or unknown; independent local covariance sum is not admissible".to_owned());
    }
    if model.local_to_global_relationship != GeometrySourceRelationship::IndependentlyCharacterized {
        reasons.push("local and global geometry errors have unresolved cross-covariance".to_owned());
    }
    if model.geometry_to_field_relationship != GeometrySourceRelationship::IndependentlyCharacterized {
        reasons.push("geometry and tactile field uncertainty share or may share evidence ancestry".to_owned());
    }
    if model.geometry_to_wrench_reference_relationship != GeometrySourceRelationship::IndependentlyCharacterized {
        reasons.push("geometry and wrench-reference uncertainty share or may share evidence ancestry".to_owned());
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
        reasons,
        claim_class: "synthetic_geometry_uncertainty_propagation_only",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn geometry() -> TaxelGeometrySet {
        let tangent_u = Vec3 { x: 1.0, y: 0.0, z: 0.0 };
        let tangent_v = Vec3 { x: 0.0, y: 1.0, z: 0.0 };
        let normal = Vec3 { x: 0.0, y: 0.0, z: 1.0 };
        TaxelGeometrySet {
            geometry_digest: "sha256:geometry-1".to_owned(),
            reference_frame_digest: "sha256:reference-frame".to_owned(),
            contact_boundary_digest: "sha256:boundary".to_owned(),
            wrench_origin_m: Vec3::default(),
            taxels: vec![super::super::wrench_consistency::TaxelGeometry {
                taxel_id: "taxel-0".to_owned(),
                area_m2: 1.0e-4,
                application_point_m: Vec3 { x: 0.1, y: 0.0, z: 0.0 },
                normal,
                tangent_u,
                tangent_v,
            }],
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
            field_estimate_digest: "sha256:field-estimate".to_owned(),
            uncertainty_method_digest: "sha256:geometry-uncertainty-method".to_owned(),
            reference_ancestry_receipt_id: "receipt:geometry-reference-ancestry".to_owned(),
            common_cause_analysis_receipt_id: "receipt:geometry-cca".to_owned(),
            local_taxels: vec![TaxelGeometryStandardUncertainty {
                taxel_id: "taxel-0".to_owned(),
                area_m2: 1.0e-6,
                application_point_m: Vec3 { x: 1.0e-3, y: 1.0e-3, z: 1.0e-3 },
                orientation_rad: Vec3 { x: 1.0e-3, y: 1.0e-3, z: 1.0e-3 },
            }],
            local_taxel_relationship: GeometrySourceRelationship::IndependentlyCharacterized,
            common_rigid_pose_covariance_row_major: pose_covariance,
            local_to_global_relationship: GeometrySourceRelationship::IndependentlyCharacterized,
            geometry_to_field_relationship: GeometrySourceRelationship::IndependentlyCharacterized,
            geometry_to_wrench_reference_relationship: GeometrySourceRelationship::IndependentlyCharacterized,
            wrench_origin_attachment: WrenchOriginAttachment::FixedInReferenceFrame,
        }
    }

    #[test]
    fn geometry_uncertainty_produces_local_and_common_pose_covariance() {
        let geometry = geometry();
        let field = [10_000.0, 1_000.0, 500.0];
        let report = propagate_geometry_wrench_uncertainty(&geometry, &field, &model(&geometry))
            .expect("geometry uncertainty propagation");
        assert_eq!(report.disposition, GeometryUqDisposition::IndependentGeometryCovarianceAvailable);
        assert!(report.admissible_for_combined_wrench_uncertainty);
        assert_eq!(report.local_geometry_covariance_row_major.as_ref().unwrap().len(), 36);
        assert_eq!(report.global_pose_covariance_row_major.as_ref().unwrap().len(), 36);
        assert_eq!(report.total_geometry_covariance_row_major.as_ref().unwrap().len(), 36);
        assert!(report.local_geometry_covariance_row_major.unwrap().iter().any(|value| *value > 0.0));
        assert!(report.global_pose_covariance_row_major.unwrap().iter().any(|value| *value > 0.0));
    }

    #[test]
    fn common_translation_affects_fixed_reference_moment_but_not_body_attached_moment() {
        let geometry = geometry();
        let field = [10_000.0, 0.0, 0.0];
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
        attached.common_rigid_pose_covariance_row_major = fixed.common_rigid_pose_covariance_row_major.clone();

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
        let field = [10_000.0, 1_000.0, 500.0];
        let mut evidence = model(&geometry);
        evidence.geometry_to_field_relationship = GeometrySourceRelationship::SharedOrUnknown;
        evidence.geometry_to_wrench_reference_relationship = GeometrySourceRelationship::SharedOrUnknown;
        let report = propagate_geometry_wrench_uncertainty(&geometry, &field, &evidence).unwrap();
        assert_eq!(report.disposition, GeometryUqDisposition::GeometryCovarianceAvailableButCrossSourcesUnresolved);
        assert!(!report.admissible_for_combined_wrench_uncertainty);
        assert!(report.total_geometry_covariance_row_major.is_some());
    }

    #[test]
    fn local_taxel_correlations_and_local_global_cross_terms_fail_closed() {
        let geometry = geometry();
        let field = [10_000.0, 1_000.0, 500.0];
        let mut local_correlated = model(&geometry);
        local_correlated.local_taxel_relationship = GeometrySourceRelationship::SharedOrUnknown;
        let report = propagate_geometry_wrench_uncertainty(&geometry, &field, &local_correlated).unwrap();
        assert_eq!(report.disposition, GeometryUqDisposition::LocalGeometryCorrelationUnresolved);
        assert!(report.local_geometry_covariance_row_major.is_none());
        assert!(report.total_geometry_covariance_row_major.is_none());

        let mut local_global = model(&geometry);
        local_global.local_to_global_relationship = GeometrySourceRelationship::SharedOrUnknown;
        let report = propagate_geometry_wrench_uncertainty(&geometry, &field, &local_global).unwrap();
        assert_eq!(report.disposition, GeometryUqDisposition::LocalGlobalGeometryCorrelationUnresolved);
        assert!(report.total_geometry_covariance_row_major.is_none());
    }

    #[test]
    fn geometry_context_and_parameter_validation_fail_closed() {
        let geometry = geometry();
        let field = [10_000.0, 1_000.0, 500.0];
        let mut wrong_digest = model(&geometry);
        wrong_digest.geometry_digest = "sha256:stale-geometry".to_owned();
        assert!(propagate_geometry_wrench_uncertainty(&geometry, &field, &wrong_digest).is_err());

        let mut negative_sigma = model(&geometry);
        negative_sigma.local_taxels[0].orientation_rad.z = -1.0;
        assert!(propagate_geometry_wrench_uncertainty(&geometry, &field, &negative_sigma).is_err());

        let mut bad_pose = model(&geometry);
        bad_pose.common_rigid_pose_covariance_row_major[1] = 1.0;
        assert!(propagate_geometry_wrench_uncertainty(&geometry, &field, &bad_pose).is_err());

        assert!(propagate_geometry_wrench_uncertainty(&geometry, &[1.0], &model(&geometry)).is_err());
    }
}
