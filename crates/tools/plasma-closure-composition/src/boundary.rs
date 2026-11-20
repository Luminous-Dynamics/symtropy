//! Explicit external-boundary flux accounting.
//!
//! Internal shared faces cancel between adjacent cells. External physical
//! boundaries do not cancel inside the domain: their outward flux must remain
//! visible in the global balance. This module accounts for one already-computed
//! outward-oriented boundary flux; it does not implement boundary physics.

use super::{
    BlendError, CandidateQualification, FaceCompatibility, validate_compatibility,
};

#[derive(Debug, Clone, PartialEq)]
pub struct BoundaryFluxRequest {
    pub schema_version: String,
    pub request_id: String,
    pub boundary_face_id: String,
    pub adjacent_cell_id: String,
    pub boundary_condition_id: String,
    pub outward_normal_id: String,
    pub cell_state_sha256: String,
    pub boundary_state_sha256: String,
    pub boundary_profile_sha256: String,
    pub boundary_model_sha256: String,
    pub qualification: CandidateQualification,
    /// Positive means material/energy leaves the domain; negative means enters.
    pub outward_flux: Vec<f64>,
    pub face_measure: f64,
    pub compatibility: FaceCompatibility,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BoundaryFluxReport {
    pub schema_version: String,
    pub request_id: String,
    pub boundary_face_id: String,
    pub adjacent_cell_id: String,
    pub boundary_condition_id: String,
    pub outward_normal_id: String,
    pub cell_state_sha256: String,
    pub boundary_state_sha256: String,
    pub boundary_profile_sha256: String,
    pub boundary_model_sha256: String,
    pub compatibility: FaceCompatibility,
    pub outward_flux: Vec<f64>,
    pub outward_integrated_flux: Vec<f64>,
    /// Finite-volume update contributed to the adjacent cell: -A * F_out.
    pub cell_integrated_update: Vec<f64>,
    /// Cell update plus outward boundary export; must cancel componentwise.
    pub boundary_balance_residual: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BoundaryFluxError {
    UnsupportedSchemaVersion,
    EmptyIdentity(&'static str),
    InvalidDigest(&'static str),
    InvalidFaceMeasure,
    CandidateUnknown,
    CandidateRejected,
    FluxComponentCountMismatch,
    NonFiniteFlux,
    NonFiniteIntegratedFlux,
    Compatibility(BlendError),
}

/// Audit and account for one already-computed external boundary flux.
///
/// outward_flux is oriented using the declared outward normal. The cell
/// contribution is its negative integrated flux, matching the finite-volume
/// convention dU/dt = -sum_faces(A_f * F_f_out).
pub fn account_external_boundary_flux(
    request: &BoundaryFluxRequest,
) -> Result<BoundaryFluxReport, BoundaryFluxError> {
    if request.schema_version != super::SCHEMA_VERSION {
        return Err(BoundaryFluxError::UnsupportedSchemaVersion);
    }
    for (name, value) in [
        ("request_id", request.request_id.as_str()),
        ("boundary_face_id", request.boundary_face_id.as_str()),
        ("adjacent_cell_id", request.adjacent_cell_id.as_str()),
        ("boundary_condition_id", request.boundary_condition_id.as_str()),
        ("outward_normal_id", request.outward_normal_id.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(BoundaryFluxError::EmptyIdentity(name));
        }
    }
    for (name, value) in [
        ("cell_state_sha256", request.cell_state_sha256.as_str()),
        ("boundary_state_sha256", request.boundary_state_sha256.as_str()),
        ("boundary_profile_sha256", request.boundary_profile_sha256.as_str()),
        ("boundary_model_sha256", request.boundary_model_sha256.as_str()),
    ] {
        if !is_sha256(value) {
            return Err(BoundaryFluxError::InvalidDigest(name));
        }
    }
    match request.qualification {
        CandidateQualification::Qualified => {}
        CandidateQualification::Unknown => return Err(BoundaryFluxError::CandidateUnknown),
        CandidateQualification::Rejected => return Err(BoundaryFluxError::CandidateRejected),
    }
    validate_compatibility(&request.compatibility)
        .map_err(BoundaryFluxError::Compatibility)?;

    if !request.face_measure.is_finite() || request.face_measure <= 0.0 {
        return Err(BoundaryFluxError::InvalidFaceMeasure);
    }
    if request.outward_flux.len() != request.compatibility.component_count {
        return Err(BoundaryFluxError::FluxComponentCountMismatch);
    }
    if request.outward_flux.iter().any(|value| !value.is_finite()) {
        return Err(BoundaryFluxError::NonFiniteFlux);
    }

    let outward_integrated_flux: Vec<f64> = request.outward_flux
        .iter()
        .map(|value| value * request.face_measure)
        .collect();
    if outward_integrated_flux.iter().any(|value| !value.is_finite()) {
        return Err(BoundaryFluxError::NonFiniteIntegratedFlux);
    }
    let cell_integrated_update: Vec<f64> =
        outward_integrated_flux.iter().map(|value| -*value).collect();
    let boundary_balance_residual: Vec<f64> = cell_integrated_update
        .iter()
        .zip(outward_integrated_flux.iter())
        .map(|(cell, outward)| cell + outward)
        .collect();

    Ok(BoundaryFluxReport {
        schema_version: super::SCHEMA_VERSION.to_owned(),
        request_id: request.request_id.clone(),
        boundary_face_id: request.boundary_face_id.clone(),
        adjacent_cell_id: request.adjacent_cell_id.clone(),
        boundary_condition_id: request.boundary_condition_id.clone(),
        outward_normal_id: request.outward_normal_id.clone(),
        cell_state_sha256: request.cell_state_sha256.clone(),
        boundary_state_sha256: request.boundary_state_sha256.clone(),
        boundary_profile_sha256: request.boundary_profile_sha256.clone(),
        boundary_model_sha256: request.boundary_model_sha256.clone(),
        compatibility: request.compatibility.clone(),
        outward_flux: request.outward_flux.clone(),
        outward_integrated_flux,
        cell_integrated_update,
        boundary_balance_residual,
    })
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CandidateQualification;

    fn digest(character: char) -> String {
        character.to_string().repeat(64)
    }

    fn compatibility() -> FaceCompatibility {
        FaceCompatibility {
            physics_model_id: "synthetic-conservation-law".into(),
            conserved_state_id: "conserved-state-u".into(),
            closure_quantity_id: "boundary-normal-flux".into(),
            species_scope_id: "single-species".into(),
            normalization_id: "dimensionless".into(),
            units_id: "normalized-flux".into(),
            coordinate_frame_id: "cartesian-lab".into(),
            geometry_id: "1d-uniform".into(),
            mesh_sha256: digest('c'),
            staggering_id: "cell-state-boundary-face".into(),
            boundary_semantics_id: "physical-external-boundary-outward".into(),
            operator_id: "synthetic-fv-v1".into(),
            solver_build_sha256: digest('d'),
            spatial_dimension: 1,
            component_count: 2,
        }
    }

    fn request() -> BoundaryFluxRequest {
        BoundaryFluxRequest {
            schema_version: crate::SCHEMA_VERSION.into(),
            request_id: "boundary-request-1".into(),
            boundary_face_id: "right-boundary".into(),
            adjacent_cell_id: "cell-last".into(),
            boundary_condition_id: "declared-open-boundary-v1".into(),
            outward_normal_id: "positive-x-outward".into(),
            cell_state_sha256: digest('1'),
            boundary_state_sha256: digest('2'),
            boundary_profile_sha256: digest('a'),
            boundary_model_sha256: digest('b'),
            qualification: CandidateQualification::Qualified,
            outward_flux: vec![2.0, -0.5],
            face_measure: 3.0,
            compatibility: compatibility(),
        }
    }

    #[test]
    fn outward_and_inward_components_remain_explicit_in_global_balance() {
        let report = account_external_boundary_flux(&request()).expect("boundary accounting");
        assert_eq!(report.outward_integrated_flux, vec![6.0, -1.5]);
        assert_eq!(report.cell_integrated_update, vec![-6.0, 1.5]);
        assert_eq!(report.boundary_balance_residual, vec![0.0, 0.0]);
    }

    #[test]
    fn unqualified_or_unknown_boundary_flux_is_rejected() {
        let mut unknown = request();
        unknown.qualification = CandidateQualification::Unknown;
        assert_eq!(
            account_external_boundary_flux(&unknown),
            Err(BoundaryFluxError::CandidateUnknown)
        );
        let mut rejected = request();
        rejected.qualification = CandidateQualification::Rejected;
        assert_eq!(
            account_external_boundary_flux(&rejected),
            Err(BoundaryFluxError::CandidateRejected)
        );
    }

    #[test]
    fn malformed_boundary_state_or_model_identity_is_rejected() {
        let mut invalid = request();
        invalid.boundary_state_sha256 = "unknown".into();
        assert_eq!(
            account_external_boundary_flux(&invalid),
            Err(BoundaryFluxError::InvalidDigest("boundary_state_sha256"))
        );
        let mut invalid_model = request();
        invalid_model.boundary_model_sha256.clear();
        assert_eq!(
            account_external_boundary_flux(&invalid_model),
            Err(BoundaryFluxError::InvalidDigest("boundary_model_sha256"))
        );
    }

    #[test]
    fn boundary_flux_component_mismatch_or_non_finite_data_is_rejected() {
        let mut wrong_arity = request();
        wrong_arity.outward_flux.pop();
        assert_eq!(
            account_external_boundary_flux(&wrong_arity),
            Err(BoundaryFluxError::FluxComponentCountMismatch)
        );
        let mut non_finite = request();
        non_finite.outward_flux[0] = f64::NAN;
        assert_eq!(
            account_external_boundary_flux(&non_finite),
            Err(BoundaryFluxError::NonFiniteFlux)
        );
    }

    #[test]
    fn invalid_boundary_measure_and_compatibility_fail_closed() {
        let mut zero_measure = request();
        zero_measure.face_measure = 0.0;
        assert_eq!(
            account_external_boundary_flux(&zero_measure),
            Err(BoundaryFluxError::InvalidFaceMeasure)
        );
        let mut incompatible = request();
        incompatible.compatibility.operator_id.clear();
        assert!(matches!(
            account_external_boundary_flux(&incompatible),
            Err(BoundaryFluxError::Compatibility(BlendError::EmptyIdentity("operator_id")))
        ));
    }

    #[test]
    fn outward_orientation_is_not_inferred_or_reversed() {
        let mut outward = request();
        let mut inward = request();
        outward.outward_flux = vec![1.0, 2.0];
        inward.outward_flux = vec![-1.0, -2.0];
        let outward_report = account_external_boundary_flux(&outward).unwrap();
        let inward_report = account_external_boundary_flux(&inward).unwrap();
        assert_eq!(outward_report.cell_integrated_update, vec![-3.0, -6.0]);
        assert_eq!(inward_report.cell_integrated_update, vec![3.0, 6.0]);
    }
}
