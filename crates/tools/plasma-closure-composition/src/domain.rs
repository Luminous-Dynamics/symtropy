//! Deterministic assembly of qualified internal and external face receipts.
//!
//! This module aggregates integrated face contributions into per-cell updates
//! and checks global conservation against signed physical-boundary export. It
//! is a receipt assembler, not a discretization, PDE solver or physics oracle.

use std::collections::{BTreeMap, BTreeSet};

use super::{
    BlendError, BoundaryFluxReport, CandidateQualification, FaceCompatibility,
    FaceFluxBlendReport, SCHEMA_VERSION, validate_compatibility,
};

#[derive(Debug, Clone, PartialEq)]
pub struct DomainAssemblyRequest {
    pub schema_version: String,
    pub request_id: String,
    pub domain_id: String,
    pub compatibility: FaceCompatibility,
    pub cell_ids: Vec<String>,
    pub internal_faces: Vec<FaceFluxBlendReport>,
    pub boundary_faces: Vec<BoundaryFluxReport>,
    /// Dimensionless absolute tolerance for candidate weight sums.
    pub weight_sum_tolerance: f64,
    /// Absolute tolerance in the declared conserved-state units.
    pub balance_tolerance: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CellIntegratedUpdate {
    pub cell_id: String,
    pub integrated_update: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DomainBalanceReport {
    pub schema_version: String,
    pub request_id: String,
    pub domain_id: String,
    pub compatibility: FaceCompatibility,
    pub cells: Vec<CellIntegratedUpdate>,
    pub internal_face_ids: Vec<String>,
    pub boundary_face_ids: Vec<String>,
    /// Sum of all per-cell integrated changes.
    pub global_cell_change: Vec<f64>,
    /// Signed sum of outward boundary fluxes (negative is net inflow).
    pub total_boundary_export: Vec<f64>,
    /// global_cell_change + total_boundary_export.
    pub global_balance_residual: Vec<f64>,
    pub balance_tolerance: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DomainBalanceError {
    UnsupportedSchemaVersion,
    EmptyIdentity(&'static str),
    EmptyCellInventory,
    DuplicateCell(String),
    InvalidBalanceTolerance,
    InvalidWeightSumTolerance,
    Compatibility(BlendError),
    DuplicateFace(String),
    UnknownCell { face_id: String, cell_id: String },
    InvalidInternalFaceSchema(String),
    InternalFaceCompatibilityMismatch(String),
    InvalidInternalFaceReceipt(String),
    InvalidBoundarySchema(String),
    BoundaryCompatibilityMismatch(String),
    InvalidBoundaryReceipt(String),
    InvalidCandidateReceipt(String),
    NonFiniteContribution(String),
    GlobalBalanceExceeded { component: usize, residual: f64 },
}

/// Assemble already-computed face receipts in a stable identity order.
///
/// Internal shared-face contributions are added once to each adjacent cell
/// with opposite sign. Boundary contributions are added only to their adjacent
/// cell; the outward export remains a separate global term. Identical input
/// receipts in any caller order produce the same ordered report.
pub fn assemble_domain_balance(
    request: &DomainAssemblyRequest,
) -> Result<DomainBalanceReport, DomainBalanceError> {
    if request.schema_version != SCHEMA_VERSION {
        return Err(DomainBalanceError::UnsupportedSchemaVersion);
    }
    for (name, value) in [
        ("request_id", request.request_id.as_str()),
        ("domain_id", request.domain_id.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(DomainBalanceError::EmptyIdentity(name));
        }
    }
    if !request.balance_tolerance.is_finite() || request.balance_tolerance < 0.0 {
        return Err(DomainBalanceError::InvalidBalanceTolerance);
    }
    if !request.weight_sum_tolerance.is_finite()
        || request.weight_sum_tolerance < 0.0
        || request.weight_sum_tolerance >= 1.0
    {
        return Err(DomainBalanceError::InvalidWeightSumTolerance);
    }
    validate_compatibility(&request.compatibility)
        .map_err(DomainBalanceError::Compatibility)?;

    if request.cell_ids.is_empty() {
        return Err(DomainBalanceError::EmptyCellInventory);
    }
    let component_count = request.compatibility.component_count;
    let mut cell_set = BTreeSet::new();
    for cell_id in &request.cell_ids {
        if cell_id.trim().is_empty() {
            return Err(DomainBalanceError::EmptyIdentity("cell_id"));
        }
        if !cell_set.insert(cell_id.clone()) {
            return Err(DomainBalanceError::DuplicateCell(cell_id.clone()));
        }
    }

    let mut face_ids = BTreeSet::new();
    let mut internal_faces: Vec<&FaceFluxBlendReport> = request.internal_faces.iter().collect();
    internal_faces.sort_by(|left, right| left.face_id.cmp(&right.face_id));

    let mut contributions: BTreeMap<String, Vec<(String, Vec<f64>)>> = cell_set
        .iter()
        .map(|cell_id| (cell_id.clone(), Vec::new()))
        .collect();

    for face in &internal_faces {
        if face.schema_version != SCHEMA_VERSION {
            return Err(DomainBalanceError::InvalidInternalFaceSchema(
                face.face_id.clone(),
            ));
        }
        if face.compatibility != request.compatibility {
            return Err(DomainBalanceError::InternalFaceCompatibilityMismatch(
                face.face_id.clone(),
            ));
        }
        if face.face_id.trim().is_empty()
            || face.left_cell_id.trim().is_empty()
            || face.right_cell_id.trim().is_empty()
            || face.left_cell_id == face.right_cell_id
        {
            return Err(DomainBalanceError::InvalidInternalFaceReceipt(
                face.face_id.clone(),
            ));
        }
        if !face_ids.insert(face.face_id.clone()) {
            return Err(DomainBalanceError::DuplicateFace(face.face_id.clone()));
        }
        if !cell_set.contains(&face.left_cell_id) {
            return Err(DomainBalanceError::UnknownCell {
                face_id: face.face_id.clone(),
                cell_id: face.left_cell_id.clone(),
            });
        }
        if !cell_set.contains(&face.right_cell_id) {
            return Err(DomainBalanceError::UnknownCell {
                face_id: face.face_id.clone(),
                cell_id: face.right_cell_id.clone(),
            });
        }
        validate_internal_face(
            face,
            component_count,
            request.weight_sum_tolerance,
            request.balance_tolerance,
        )?;

        contributions
            .get_mut(&face.left_cell_id)
            .expect("cell inventory checked")
            .push((
                format!("internal:{}:left", face.face_id),
                face.left_integrated_update.clone(),
            ));
        contributions
            .get_mut(&face.right_cell_id)
            .expect("cell inventory checked")
            .push((
                format!("internal:{}:right", face.face_id),
                face.right_integrated_update.clone(),
            ));
    }

    let mut boundary_faces: Vec<&BoundaryFluxReport> =
        request.boundary_faces.iter().collect();
    boundary_faces.sort_by(|left, right| left.boundary_face_id.cmp(&right.boundary_face_id));
    let mut boundary_exports: Vec<(String, Vec<f64>)> = Vec::new();

    for boundary in &boundary_faces {
        if boundary.schema_version != SCHEMA_VERSION {
            return Err(DomainBalanceError::InvalidBoundarySchema(
                boundary.boundary_face_id.clone(),
            ));
        }
        if boundary.compatibility != request.compatibility {
            return Err(DomainBalanceError::BoundaryCompatibilityMismatch(
                boundary.boundary_face_id.clone(),
            ));
        }
        if boundary.boundary_face_id.trim().is_empty()
            || boundary.adjacent_cell_id.trim().is_empty()
        {
            return Err(DomainBalanceError::InvalidBoundaryReceipt(
                boundary.boundary_face_id.clone(),
            ));
        }
        if !face_ids.insert(boundary.boundary_face_id.clone()) {
            return Err(DomainBalanceError::DuplicateFace(
                boundary.boundary_face_id.clone(),
            ));
        }
        if !cell_set.contains(&boundary.adjacent_cell_id) {
            return Err(DomainBalanceError::UnknownCell {
                face_id: boundary.boundary_face_id.clone(),
                cell_id: boundary.adjacent_cell_id.clone(),
            });
        }
        validate_boundary_face(boundary, component_count, request.balance_tolerance)?;

        contributions
            .get_mut(&boundary.adjacent_cell_id)
            .expect("cell inventory checked")
            .push((
                format!("boundary:{}", boundary.boundary_face_id),
                boundary.cell_integrated_update.clone(),
            ));
        boundary_exports.push((
            boundary.boundary_face_id.clone(),
            boundary.outward_integrated_flux.clone(),
        ));
    }

    let mut cells = Vec::with_capacity(cell_set.len());
    for (cell_id, mut cell_contributions) in contributions {
        cell_contributions.sort_by(|left, right| left.0.cmp(&right.0));
        let mut integrated_update = vec![0.0; component_count];
        for component in 0..component_count {
            integrated_update[component] = compensated_sum(
                cell_contributions
                    .iter()
                    .map(|(_, update)| update[component]),
            );
            if !integrated_update[component].is_finite() {
                return Err(DomainBalanceError::NonFiniteContribution(cell_id.clone()));
            }
        }
        cells.push(CellIntegratedUpdate {
            cell_id,
            integrated_update,
        });
    }

    boundary_exports.sort_by(|left, right| left.0.cmp(&right.0));
    let mut total_boundary_export = vec![0.0; component_count];
    for component in 0..component_count {
        total_boundary_export[component] = compensated_sum(
            boundary_exports
                .iter()
                .map(|(_, export)| export[component]),
        );
    }

    let mut global_cell_change = vec![0.0; component_count];
    for component in 0..component_count {
        global_cell_change[component] =
            compensated_sum(cells.iter().map(|cell| cell.integrated_update[component]));
    }

    let global_balance_residual: Vec<f64> = global_cell_change
        .iter()
        .zip(total_boundary_export.iter())
        .map(|(change, export)| change + export)
        .collect();

    for (component, residual) in global_balance_residual.iter().copied().enumerate() {
        if !residual.is_finite() {
            return Err(DomainBalanceError::NonFiniteContribution(
                "global-balance".into(),
            ));
        }
        if residual.abs() > request.balance_tolerance {
            return Err(DomainBalanceError::GlobalBalanceExceeded {
                component,
                residual,
            });
        }
    }

    Ok(DomainBalanceReport {
        schema_version: SCHEMA_VERSION.to_owned(),
        request_id: request.request_id.clone(),
        domain_id: request.domain_id.clone(),
        compatibility: request.compatibility.clone(),
        cells,
        internal_face_ids: internal_faces
            .iter()
            .map(|face| face.face_id.clone())
            .collect(),
        boundary_face_ids: boundary_faces
            .iter()
            .map(|face| face.boundary_face_id.clone())
            .collect(),
        global_cell_change,
        total_boundary_export,
        global_balance_residual,
        balance_tolerance: request.balance_tolerance,
    })
}

fn validate_internal_face(
    face: &FaceFluxBlendReport,
    component_count: usize,
    weight_sum_tolerance: f64,
    balance_tolerance: f64,
) -> Result<(), DomainBalanceError> {
    if face.candidate_receipts.is_empty()
        || face.candidate_receipts.len() != face.candidate_weights.len()
    {
        return Err(DomainBalanceError::InvalidCandidateReceipt(
            face.face_id.clone(),
        ));
    }
    let mut expert_ids = BTreeSet::new();
    for (receipt, weight) in face
        .candidate_receipts
        .iter()
        .zip(face.candidate_weights.iter())
    {
        if receipt.expert_id.trim().is_empty()
            || receipt.qualification != CandidateQualification::Qualified
            || !is_sha256(&receipt.profile_sha256)
            || !is_sha256(&receipt.model_sha256)
            || !expert_ids.insert(receipt.expert_id.clone())
            || !weight.is_finite()
            || *weight < 0.0
        {
            return Err(DomainBalanceError::InvalidCandidateReceipt(
                face.face_id.clone(),
            ));
        }
    }
    let weight_sum = compensated_sum(face.candidate_weights.iter().copied());
    if !weight_sum.is_finite() || (weight_sum - 1.0).abs() > weight_sum_tolerance {
        return Err(DomainBalanceError::InvalidCandidateReceipt(
            face.face_id.clone(),
        ));
    }
    if !face.face_measure.is_finite() || face.face_measure <= 0.0 {
        return Err(DomainBalanceError::InvalidInternalFaceReceipt(
            face.face_id.clone(),
        ));
    }

    let vectors = [
        &face.blended_flux,
        &face.integrated_flux,
        &face.left_integrated_update,
        &face.right_integrated_update,
        &face.internal_balance_residual,
    ];
    if vectors.iter().any(|vector| vector.len() != component_count) {
        return Err(DomainBalanceError::InvalidInternalFaceReceipt(
            face.face_id.clone(),
        ));
    }
    for component in 0..component_count {
        let values = [
            face.blended_flux[component],
            face.integrated_flux[component],
            face.left_integrated_update[component],
            face.right_integrated_update[component],
            face.internal_balance_residual[component],
        ];
        if values.iter().any(|value| !value.is_finite()) {
            return Err(DomainBalanceError::NonFiniteContribution(
                face.face_id.clone(),
            ));
        }
        let expected_residual =
            face.left_integrated_update[component] + face.right_integrated_update[component];
        let expected_integrated_flux = face.blended_flux[component] * face.face_measure;
        if !expected_integrated_flux.is_finite()
            || (face.integrated_flux[component] - expected_integrated_flux).abs()
                > balance_tolerance
            || (face.left_integrated_update[component] + face.integrated_flux[component]).abs()
                > balance_tolerance
            || (face.right_integrated_update[component] - face.integrated_flux[component]).abs()
                > balance_tolerance
            || (face.internal_balance_residual[component] - expected_residual).abs()
                > balance_tolerance
            || expected_residual.abs() > balance_tolerance
        {
            return Err(DomainBalanceError::InvalidInternalFaceReceipt(
                face.face_id.clone(),
            ));
        }
    }
    Ok(())
}

fn validate_boundary_face(
    boundary: &BoundaryFluxReport,
    component_count: usize,
    tolerance: f64,
) -> Result<(), DomainBalanceError> {
    if boundary.qualification != CandidateQualification::Qualified
        || boundary.boundary_condition_id.trim().is_empty()
        || boundary.outward_normal_id.trim().is_empty()
    {
        return Err(DomainBalanceError::InvalidBoundaryReceipt(
            boundary.boundary_face_id.clone(),
        ));
    }
    if !boundary.face_measure.is_finite() || boundary.face_measure <= 0.0 {
        return Err(DomainBalanceError::InvalidBoundaryReceipt(
            boundary.boundary_face_id.clone(),
        ));
    }
    for (name, value) in [
        ("cell_state_sha256", boundary.cell_state_sha256.as_str()),
        ("boundary_state_sha256", boundary.boundary_state_sha256.as_str()),
        ("boundary_profile_sha256", boundary.boundary_profile_sha256.as_str()),
        ("boundary_model_sha256", boundary.boundary_model_sha256.as_str()),
    ] {
        if !is_sha256(value) {
            return Err(DomainBalanceError::InvalidBoundaryReceipt(format!(
                "{}:{name}",
                boundary.boundary_face_id
            )));
        }
    }
    let vectors = [
        &boundary.outward_flux,
        &boundary.outward_integrated_flux,
        &boundary.cell_integrated_update,
        &boundary.boundary_balance_residual,
    ];
    if vectors.iter().any(|vector| vector.len() != component_count) {
        return Err(DomainBalanceError::InvalidBoundaryReceipt(
            boundary.boundary_face_id.clone(),
        ));
    }
    for component in 0..component_count {
        let outward = boundary.outward_integrated_flux[component];
        let cell = boundary.cell_integrated_update[component];
        let residual = boundary.boundary_balance_residual[component];
        if !boundary.outward_flux[component].is_finite()
            || !outward.is_finite()
            || !cell.is_finite()
            || !residual.is_finite()
        {
            return Err(DomainBalanceError::NonFiniteContribution(
                boundary.boundary_face_id.clone(),
            ));
        }
        let expected_outward = boundary.outward_flux[component] * boundary.face_measure;
        if !expected_outward.is_finite()
            || (outward - expected_outward).abs() > tolerance
            || (cell + outward).abs() > tolerance
            || (residual - (cell + outward)).abs() > tolerance
            || residual.abs() > tolerance
        {
            return Err(DomainBalanceError::InvalidBoundaryReceipt(
                boundary.boundary_face_id.clone(),
            ));
        }
    }
    Ok(())
}

fn compensated_sum(values: impl IntoIterator<Item = f64>) -> f64 {
    let mut sum = 0.0;
    let mut correction = 0.0;
    for value in values {
        let adjusted = value - correction;
        let next = sum + adjusted;
        correction = (next - sum) - adjusted;
        sum = next;
    }
    sum
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        CandidateQualification, FaceFluxBlendRequest, FaceFluxCandidate,
        account_external_boundary_flux, compose_shared_face_flux,
    };

    fn digest(character: char) -> String {
        character.to_string().repeat(64)
    }

    fn compatibility() -> FaceCompatibility {
        FaceCompatibility {
            physics_model_id: "synthetic-conservation-law".into(),
            conserved_state_id: "density-momentum-energy".into(),
            closure_quantity_id: "closure-flux".into(),
            species_scope_id: "single-species".into(),
            normalization_id: "dimensionless-v1".into(),
            units_id: "normalized-flux-v1".into(),
            coordinate_frame_id: "cartesian-lab".into(),
            geometry_id: "uniform-1d".into(),
            mesh_sha256: digest('c'),
            staggering_id: "cell-state-face-flux".into(),
            boundary_semantics_id: "explicit-external-boundaries".into(),
            operator_id: "synthetic-fv-v1".into(),
            solver_build_sha256: digest('d'),
            spatial_dimension: 1,
            component_count: 2,
        }
    }

    fn internal_face(
        face_id: &str,
        left: &str,
        right: &str,
        flux: Vec<f64>,
    ) -> FaceFluxBlendReport {
        compose_shared_face_flux(&FaceFluxBlendRequest {
            schema_version: SCHEMA_VERSION.into(),
            request_id: format!("request-{face_id}"),
            face_id: face_id.into(),
            left_cell_id: left.into(),
            right_cell_id: right.into(),
            face_measure: 1.0,
            weight_sum_tolerance: 1.0e-12,
            compatibility: compatibility(),
            candidates: vec![FaceFluxCandidate {
                expert_id: "expert-a".into(),
                profile_sha256: digest('a'),
                model_sha256: digest('b'),
                qualification: CandidateQualification::Qualified,
                compatibility: compatibility(),
                weight: 1.0,
                flux,
            }],
        })
        .expect("synthetic internal face")
    }

    fn boundary_face(face_id: &str, cell_id: &str, outward: Vec<f64>) -> BoundaryFluxReport {
        boundary_face_with_measure(face_id, cell_id, outward, 1.0)
    }

    fn boundary_face_with_measure(
        face_id: &str,
        cell_id: &str,
        outward: Vec<f64>,
        face_measure: f64,
    ) -> BoundaryFluxReport {
        account_external_boundary_flux(&crate::boundary::BoundaryFluxRequest {
            schema_version: SCHEMA_VERSION.into(),
            request_id: format!("request-{face_id}"),
            boundary_face_id: face_id.into(),
            adjacent_cell_id: cell_id.into(),
            boundary_condition_id: "declared-boundary-v1".into(),
            outward_normal_id: format!("normal-{face_id}"),
            cell_state_sha256: digest('1'),
            boundary_state_sha256: digest('2'),
            boundary_profile_sha256: digest('a'),
            boundary_model_sha256: digest('b'),
            qualification: CandidateQualification::Qualified,
            outward_flux: outward,
            face_measure,
            compatibility: compatibility(),
        })
        .expect("synthetic boundary face")
    }

    fn request(
        cell_ids: Vec<&str>,
        internal_faces: Vec<FaceFluxBlendReport>,
        boundary_faces: Vec<BoundaryFluxReport>,
    ) -> DomainAssemblyRequest {
        DomainAssemblyRequest {
            schema_version: SCHEMA_VERSION.into(),
            request_id: "domain-request-1".into(),
            domain_id: "synthetic-domain-1".into(),
            compatibility: compatibility(),
            cell_ids: cell_ids.into_iter().map(str::to_owned).collect(),
            internal_faces,
            boundary_faces,
            weight_sum_tolerance: 1.0e-12,
            balance_tolerance: 1.0e-12,
        }
    }

    #[test]
    fn two_cell_mixed_domain_matches_global_boundary_balance() {
        let report = assemble_domain_balance(&request(
            vec!["cell-0", "cell-1"],
            vec![internal_face(
                "internal-0-1",
                "cell-0",
                "cell-1",
                vec![2.0, -1.0],
            )],
            vec![
                boundary_face_with_measure("left", "cell-0", vec![1.0, -1.0], 0.5),
                boundary_face_with_measure("right", "cell-1", vec![-0.5, 2.0], 0.25),
            ],
        ))
        .expect("balanced two-cell domain");

        assert_eq!(
            report.cells.iter().map(|cell| cell.cell_id.as_str()).collect::<Vec<_>>(),
            vec!["cell-0", "cell-1"]
        );
        assert_eq!(
            report.cells[0].integrated_update,
            vec![-2.5, 1.5]
        );
        assert_eq!(
            report.cells[1].integrated_update,
            vec![2.125, -1.5]
        );
        assert_eq!(report.global_cell_change, vec![-0.375, 0.0]);
        assert_eq!(report.total_boundary_export, vec![0.375, 0.0]);
        assert_eq!(report.global_balance_residual, vec![0.0, 0.0]);
    }

    #[test]
    fn periodic_internal_faces_cancel_without_boundary_export() {
        let report = assemble_domain_balance(&request(
            vec!["cell-0", "cell-1"],
            vec![
                internal_face("face-0-1", "cell-0", "cell-1", vec![2.0, 1.0]),
                internal_face("face-1-0", "cell-1", "cell-0", vec![2.0, 1.0]),
            ],
            vec![],
        ))
        .expect("internal periodic-like ring");
        assert_eq!(report.global_cell_change, vec![0.0, 0.0]);
        assert_eq!(report.total_boundary_export, vec![0.0, 0.0]);
        assert_eq!(report.global_balance_residual, vec![0.0, 0.0]);
    }

    #[test]
    fn caller_order_does_not_change_domain_report() {
        let first = request(
            vec!["cell-0", "cell-1"],
            vec![
                internal_face("face-a", "cell-0", "cell-1", vec![1.0, 0.0]),
                internal_face("face-b", "cell-1", "cell-0", vec![0.25, 0.5]),
            ],
            vec![],
        );
        let second = request(
            vec!["cell-1", "cell-0"],
            vec![
                internal_face("face-b", "cell-1", "cell-0", vec![0.25, 0.5]),
                internal_face("face-a", "cell-0", "cell-1", vec![1.0, 0.0]),
            ],
            vec![],
        );
        assert_eq!(
            assemble_domain_balance(&first).unwrap(),
            assemble_domain_balance(&second).unwrap()
        );
    }

    #[test]
    fn duplicate_cells_and_faces_are_rejected() {
        assert_eq!(
            assemble_domain_balance(&request(
                vec!["cell-0", "cell-0"],
                vec![],
                vec![],
            )),
            Err(DomainBalanceError::DuplicateCell("cell-0".into()))
        );

        let duplicate = internal_face("same-face", "cell-0", "cell-1", vec![1.0, 1.0]);
        assert_eq!(
            assemble_domain_balance(&request(
                vec!["cell-0", "cell-1"],
                vec![duplicate.clone(), duplicate],
                vec![],
            )),
            Err(DomainBalanceError::DuplicateFace("same-face".into()))
        );
    }

    #[test]
    fn unknown_cells_and_mismatched_compatibility_are_rejected() {
        assert!(matches!(
            assemble_domain_balance(&request(
                vec!["cell-0"],
                vec![internal_face("face", "cell-0", "missing", vec![1.0, 1.0])],
                vec![],
            )),
            Err(DomainBalanceError::UnknownCell { .. })
        ));

        let mut incompatible = internal_face("face", "cell-0", "cell-1", vec![1.0, 1.0]);
        incompatible.compatibility.operator_id = "other-operator".into();
        assert!(matches!(
            assemble_domain_balance(&request(
                vec!["cell-0", "cell-1"],
                vec![incompatible],
                vec![],
            )),
            Err(DomainBalanceError::InternalFaceCompatibilityMismatch(_))
        ));
    }

    #[test]
    fn malformed_nonconservative_face_receipt_is_rejected() {
        let mut nonconservative = internal_face("face", "cell-0", "cell-1", vec![1.0, 1.0]);
        nonconservative.left_integrated_update[0] += 0.1;
        assert!(matches!(
            assemble_domain_balance(&request(
                vec!["cell-0", "cell-1"],
                vec![nonconservative],
                vec![],
            )),
            Err(DomainBalanceError::InvalidInternalFaceReceipt(_))
        ));

        let mut wrong_area = internal_face("wrong-area", "cell-0", "cell-1", vec![1.0, 1.0]);
        wrong_area.face_measure = 2.0;
        assert!(matches!(
            assemble_domain_balance(&request(
                vec!["cell-0", "cell-1"],
                vec![wrong_area],
                vec![],
            )),
            Err(DomainBalanceError::InvalidInternalFaceReceipt(_))
        ));

        let mut unqualified = internal_face("unqualified", "cell-0", "cell-1", vec![1.0, 1.0]);
        unqualified.candidate_receipts[0].qualification = CandidateQualification::Unknown;
        assert!(matches!(
            assemble_domain_balance(&request(
                vec!["cell-0", "cell-1"],
                vec![unqualified],
                vec![],
            )),
            Err(DomainBalanceError::InvalidCandidateReceipt(_))
        ));
    }

    #[test]
    fn invalid_boundary_receipt_is_rejected() {
        let mut boundary = boundary_face("wall", "cell-0", vec![1.0, 1.0]);
        boundary.cell_integrated_update[1] = 0.0;
        assert!(matches!(
            assemble_domain_balance(&request(
                vec!["cell-0"],
                vec![],
                vec![boundary],
            )),
            Err(DomainBalanceError::InvalidBoundaryReceipt(_))
        ));

        let mut unknown_authority = boundary_face("unknown-wall", "cell-0", vec![1.0, 1.0]);
        unknown_authority.qualification = CandidateQualification::Unknown;
        assert!(matches!(
            assemble_domain_balance(&request(
                vec!["cell-0"],
                vec![],
                vec![unknown_authority],
            )),
            Err(DomainBalanceError::InvalidBoundaryReceipt(_))
        ));
    }

    #[test]
    fn internal_face_cancellation_is_preserved_in_domain_assembly() {
        let report = assemble_domain_balance(&request(
            vec!["cell-0", "cell-1"],
            vec![internal_face("face", "cell-0", "cell-1", vec![1.0, 0.0])],
            vec![boundary_face("wall", "cell-0", vec![0.0, 0.0])],
        ));
        assert_eq!(
            report,
            Ok(DomainBalanceReport {
                schema_version: SCHEMA_VERSION.into(),
                request_id: "domain-request-1".into(),
                domain_id: "synthetic-domain-1".into(),
                compatibility: compatibility(),
                cells: vec![
                    CellIntegratedUpdate {
                        cell_id: "cell-0".into(),
                        integrated_update: vec![-1.0, 0.0],
                    },
                    CellIntegratedUpdate {
                        cell_id: "cell-1".into(),
                        integrated_update: vec![1.0, 0.0],
                    },
                ],
                internal_face_ids: vec!["face".into()],
                boundary_face_ids: vec!["wall".into()],
                global_cell_change: vec![0.0, 0.0],
                total_boundary_export: vec![0.0, 0.0],
                global_balance_residual: vec![0.0, 0.0],
                balance_tolerance: 1.0e-12,
            })
        );
    }
}
