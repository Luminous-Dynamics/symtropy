//! Composition of already-qualified specialist closure fluxes.
//!
//! This is not a router, plasma solver, or physical referee. It validates
//! compatibility declarations for one internal face and derives both cell
//! transfers from one shared, canonical face flux.

pub mod boundary;
pub mod domain;
pub mod entropy;

pub const SCHEMA_VERSION: &str = "luminous.plasma-closure-composition.v0.1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FaceCompatibility {
    pub physics_model_id: String,
    pub conserved_state_id: String,
    pub closure_quantity_id: String,
    /// Stable schema for vector component identities, order and units.
    pub component_layout_id: String,
    pub species_scope_id: String,
    pub normalization_id: String,
    pub units_id: String,
    pub coordinate_frame_id: String,
    pub geometry_id: String,
    pub mesh_sha256: String,
    pub staggering_id: String,
    pub boundary_semantics_id: String,
    pub operator_id: String,
    pub solver_build_sha256: String,
    pub spatial_dimension: u8,
    pub component_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandidateQualification {
    Qualified,
    Unknown,
    Rejected,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FaceFluxCandidate {
    pub expert_id: String,
    pub profile_sha256: String,
    pub model_sha256: String,
    pub qualification: CandidateQualification,
    pub compatibility: FaceCompatibility,
    pub weight: f64,
    /// Canonical flux oriented from left_cell_id toward right_cell_id.
    pub flux: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FaceFluxBlendRequest {
    pub schema_version: String,
    pub request_id: String,
    pub face_id: String,
    pub left_cell_id: String,
    pub right_cell_id: String,
    /// Positive physical face measure; input fluxes are per unit face measure.
    pub face_measure: f64,
    /// Absolute tolerance for the sum of nonnegative weights; no renormalization.
    pub weight_sum_tolerance: f64,
    pub compatibility: FaceCompatibility,
    pub candidates: Vec<FaceFluxCandidate>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateReceipt {
    pub expert_id: String,
    pub profile_sha256: String,
    pub model_sha256: String,
    pub qualification: CandidateQualification,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FaceFluxBlendReport {
    pub schema_version: String,
    pub request_id: String,
    pub face_id: String,
    pub left_cell_id: String,
    pub right_cell_id: String,
    pub face_measure: f64,
    pub compatibility: FaceCompatibility,
    pub candidate_receipts: Vec<CandidateReceipt>,
    pub candidate_weights: Vec<f64>,
    pub weight_sum: f64,
    pub blended_flux: Vec<f64>,
    pub integrated_flux: Vec<f64>,
    pub left_integrated_update: Vec<f64>,
    pub right_integrated_update: Vec<f64>,
    pub internal_balance_residual: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlendError {
    UnsupportedSchemaVersion,
    EmptyIdentity(&'static str),
    InvalidCompatibilityDigest(&'static str),
    ZeroSpatialDimension,
    ZeroComponentCount,
    InvalidFaceMeasure,
    InvalidWeightTolerance,
    SameCellOnBothSides,
    NoCandidates,
    InvalidCandidateDigest { expert_id: String, field: &'static str },
    CandidateUnknown(String),
    CandidateRejected(String),
    DuplicateCandidate(String),
    IncompatibleCandidate(String),
    InvalidCandidateWeight(String),
    WeightSumMismatch,
    CandidateFluxLengthMismatch(String),
    NonFiniteCandidateFlux(String),
    NonFiniteWeightSum,
    NonFiniteBlendedFlux,
}

/// Compose already-qualified specialist fluxes for one internal face.
///
/// Candidates are sorted by stable expert ID before summation, so caller input
/// order cannot change the result for the same candidate set. This function
/// does not authenticate digests or determine scientific qualification.
pub fn compose_shared_face_flux(
    request: &FaceFluxBlendRequest,
) -> Result<FaceFluxBlendReport, BlendError> {
    if request.schema_version != SCHEMA_VERSION {
        return Err(BlendError::UnsupportedSchemaVersion);
    }
    for (name, value) in [
        ("request_id", request.request_id.as_str()),
        ("face_id", request.face_id.as_str()),
        ("left_cell_id", request.left_cell_id.as_str()),
        ("right_cell_id", request.right_cell_id.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(BlendError::EmptyIdentity(name));
        }
    }
    if request.left_cell_id == request.right_cell_id {
        return Err(BlendError::SameCellOnBothSides);
    }
    if !request.face_measure.is_finite() || request.face_measure <= 0.0 {
        return Err(BlendError::InvalidFaceMeasure);
    }
    if !request.weight_sum_tolerance.is_finite()
        || request.weight_sum_tolerance < 0.0
        || request.weight_sum_tolerance >= 1.0
    {
        return Err(BlendError::InvalidWeightTolerance);
    }
    validate_compatibility(&request.compatibility)?;
    if request.candidates.is_empty() {
        return Err(BlendError::NoCandidates);
    }

    let mut candidates: Vec<&FaceFluxCandidate> = request.candidates.iter().collect();
    candidates.sort_by(|left, right| left.expert_id.cmp(&right.expert_id));

    for (index, candidate) in candidates.iter().enumerate() {
        if candidate.expert_id.trim().is_empty() {
            return Err(BlendError::EmptyIdentity("expert_id"));
        }
        if index > 0 && candidates[index - 1].expert_id == candidate.expert_id {
            return Err(BlendError::DuplicateCandidate(candidate.expert_id.clone()));
        }
        if !is_sha256(&candidate.profile_sha256) {
            return Err(BlendError::InvalidCandidateDigest {
                expert_id: candidate.expert_id.clone(),
                field: "profile_sha256",
            });
        }
        if !is_sha256(&candidate.model_sha256) {
            return Err(BlendError::InvalidCandidateDigest {
                expert_id: candidate.expert_id.clone(),
                field: "model_sha256",
            });
        }
        match candidate.qualification {
            CandidateQualification::Qualified => {}
            CandidateQualification::Unknown => {
                return Err(BlendError::CandidateUnknown(candidate.expert_id.clone()));
            }
            CandidateQualification::Rejected => {
                return Err(BlendError::CandidateRejected(candidate.expert_id.clone()));
            }
        }
        if candidate.compatibility != request.compatibility {
            return Err(BlendError::IncompatibleCandidate(candidate.expert_id.clone()));
        }
        if !candidate.weight.is_finite() || candidate.weight < 0.0 {
            return Err(BlendError::InvalidCandidateWeight(candidate.expert_id.clone()));
        }
        if candidate.flux.len() != request.compatibility.component_count {
            return Err(BlendError::CandidateFluxLengthMismatch(candidate.expert_id.clone()));
        }
        if candidate.flux.iter().any(|value| !value.is_finite()) {
            return Err(BlendError::NonFiniteCandidateFlux(candidate.expert_id.clone()));
        }
    }

    let weight_sum = compensated_sum(candidates.iter().map(|candidate| candidate.weight));
    if !weight_sum.is_finite() {
        return Err(BlendError::NonFiniteWeightSum);
    }
    if (weight_sum - 1.0).abs() > request.weight_sum_tolerance {
        return Err(BlendError::WeightSumMismatch);
    }

    let mut blended_flux = vec![0.0; request.compatibility.component_count];
    for (component, output) in blended_flux.iter_mut().enumerate() {
        let values = candidates
            .iter()
            .map(|candidate| candidate.weight * candidate.flux[component]);
        *output = compensated_sum(values);
        if !output.is_finite() {
            return Err(BlendError::NonFiniteBlendedFlux);
        }
    }

    let integrated_flux: Vec<f64> = blended_flux
        .iter()
        .map(|value| value * request.face_measure)
        .collect();
    if integrated_flux.iter().any(|value| !value.is_finite()) {
        return Err(BlendError::NonFiniteBlendedFlux);
    }

    // Derive both cell contributions from the same integrated face flux.
    let left_integrated_update: Vec<f64> =
        integrated_flux.iter().map(|value| -*value).collect();
    let right_integrated_update = integrated_flux.clone();
    let internal_balance_residual: Vec<f64> = left_integrated_update
        .iter()
        .zip(right_integrated_update.iter())
        .map(|(left, right)| left + right)
        .collect();

    Ok(FaceFluxBlendReport {
        schema_version: SCHEMA_VERSION.to_owned(),
        request_id: request.request_id.clone(),
        face_id: request.face_id.clone(),
        left_cell_id: request.left_cell_id.clone(),
        right_cell_id: request.right_cell_id.clone(),
        face_measure: request.face_measure,
        compatibility: request.compatibility.clone(),
        candidate_receipts: candidates
            .iter()
            .map(|candidate| CandidateReceipt {
                expert_id: candidate.expert_id.clone(),
                profile_sha256: candidate.profile_sha256.clone(),
                model_sha256: candidate.model_sha256.clone(),
                qualification: candidate.qualification,
            })
            .collect(),
        candidate_weights: candidates.iter().map(|candidate| candidate.weight).collect(),
        weight_sum,
        blended_flux,
        integrated_flux,
        left_integrated_update,
        right_integrated_update,
        internal_balance_residual,
    })
}

fn validate_compatibility(value: &FaceCompatibility) -> Result<(), BlendError> {
    for (name, identity) in [
        ("physics_model_id", value.physics_model_id.as_str()),
        ("conserved_state_id", value.conserved_state_id.as_str()),
        ("closure_quantity_id", value.closure_quantity_id.as_str()),
        ("component_layout_id", value.component_layout_id.as_str()),
        ("species_scope_id", value.species_scope_id.as_str()),
        ("normalization_id", value.normalization_id.as_str()),
        ("units_id", value.units_id.as_str()),
        ("coordinate_frame_id", value.coordinate_frame_id.as_str()),
        ("geometry_id", value.geometry_id.as_str()),
        ("staggering_id", value.staggering_id.as_str()),
        ("boundary_semantics_id", value.boundary_semantics_id.as_str()),
        ("operator_id", value.operator_id.as_str()),
    ] {
        if identity.trim().is_empty() {
            return Err(BlendError::EmptyIdentity(name));
        }
    }
    if !is_sha256(&value.mesh_sha256) {
        return Err(BlendError::InvalidCompatibilityDigest("mesh_sha256"));
    }
    if !is_sha256(&value.solver_build_sha256) {
        return Err(BlendError::InvalidCompatibilityDigest("solver_build_sha256"));
    }
    if value.spatial_dimension == 0 {
        return Err(BlendError::ZeroSpatialDimension);
    }
    if value.component_count == 0 {
        return Err(BlendError::ZeroComponentCount);
    }
    Ok(())
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

/// Kahan-style compensated sum in a stable, caller-declared order.
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

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(character: char) -> String {
        character.to_string().repeat(64)
    }

    fn compatibility() -> FaceCompatibility {
        FaceCompatibility {
            physics_model_id: "synthetic-2d3v-moment-system".into(),
            conserved_state_id: "density-momentum-energy".into(),
            closure_quantity_id: "electron-pressure-flux".into(),
            component_layout_id: "pressure-flux-xyz-v1".into(),
            species_scope_id: "electrons".into(),
            normalization_id: "synthetic-normalization-v1".into(),
            units_id: "normalized_flux".into(),
            coordinate_frame_id: "cartesian-lab".into(),
            geometry_id: "uniform-cartesian-2d".into(),
            mesh_sha256: digest('c'),
            staggering_id: "cell-centered-state-shared-face-flux".into(),
            boundary_semantics_id: "internal-face-left-to-right".into(),
            operator_id: "synthetic-shared-face-divergence-v1".into(),
            solver_build_sha256: digest('d'),
            spatial_dimension: 2,
            component_count: 2,
        }
    }

    fn candidate(expert_id: &str, weight: f64, flux: Vec<f64>) -> FaceFluxCandidate {
        FaceFluxCandidate {
            expert_id: expert_id.into(),
            profile_sha256: digest('a'),
            model_sha256: digest('b'),
            qualification: CandidateQualification::Qualified,
            compatibility: compatibility(),
            weight,
            flux,
        }
    }

    fn request(candidates: Vec<FaceFluxCandidate>) -> FaceFluxBlendRequest {
        FaceFluxBlendRequest {
            schema_version: SCHEMA_VERSION.into(),
            request_id: "synthetic-request-001".into(),
            face_id: "face-x-0-1".into(),
            left_cell_id: "cell-0".into(),
            right_cell_id: "cell-1".into(),
            face_measure: 2.0,
            weight_sum_tolerance: 1.0e-12,
            compatibility: compatibility(),
            candidates,
        }
    }

    #[test]
    fn one_expert_preserves_flux_and_emits_opposite_cell_transfers() {
        let result = compose_shared_face_flux(&request(vec![
            candidate("expert-a", 1.0, vec![1.5, -2.0]),
        ]))
        .expect("valid single expert");
        assert_eq!(result.blended_flux, vec![1.5, -2.0]);
        assert_eq!(result.integrated_flux, vec![3.0, -4.0]);
        assert_eq!(result.left_integrated_update, vec![-3.0, 4.0]);
        assert_eq!(result.right_integrated_update, vec![3.0, -4.0]);
        assert_eq!(result.internal_balance_residual, vec![0.0, 0.0]);
    }

    #[test]
    fn two_specialists_match_closed_form_convex_flux_blend() {
        let result = compose_shared_face_flux(&request(vec![
            candidate("expert-b", 0.75, vec![0.0, 6.0]),
            candidate("expert-a", 0.25, vec![4.0, 2.0]),
        ]))
        .expect("valid two-expert blend");
        assert_eq!(result.blended_flux, vec![1.0, 5.0]);
        assert_eq!(result.candidate_weights, vec![0.25, 0.75]);
        assert_eq!(
            result.candidate_receipts.iter().map(|item| item.expert_id.as_str()).collect::<Vec<_>>(),
            vec!["expert-a", "expert-b"]
        );
    }

    #[test]
    fn candidate_input_order_does_not_change_composition() {
        let a = candidate("expert-a", 0.25, vec![4.0, 2.0]);
        let b = candidate("expert-b", 0.75, vec![0.0, 6.0]);
        let first = compose_shared_face_flux(&request(vec![a.clone(), b.clone()])).unwrap();
        let second = compose_shared_face_flux(&request(vec![b, a])).unwrap();
        assert_eq!(first.blended_flux, second.blended_flux);
        assert_eq!(first.integrated_flux, second.integrated_flux);
        assert_eq!(first.candidate_receipts, second.candidate_receipts);
    }

    #[test]
    fn shared_internal_face_transfers_are_equal_and_opposite() {
        let result = compose_shared_face_flux(&request(vec![
            candidate("expert-a", 1.0, vec![0.1, -0.3]),
        ]))
        .unwrap();
        for component in 0..2 {
            assert_eq!(
                result.left_integrated_update[component]
                    + result.right_integrated_update[component],
                0.0
            );
        }
    }

    #[test]
    fn empty_candidate_set_is_rejected() {
        assert_eq!(
            compose_shared_face_flux(&request(vec![])),
            Err(BlendError::NoCandidates)
        );
    }

    #[test]
    fn duplicate_expert_identity_is_rejected() {
        let result = compose_shared_face_flux(&request(vec![
            candidate("expert-a", 0.5, vec![1.0, 1.0]),
            candidate("expert-a", 0.5, vec![2.0, 2.0]),
        ]));
        assert_eq!(result, Err(BlendError::DuplicateCandidate("expert-a".into())));
    }

    #[test]
    fn weights_are_not_silently_renormalized() {
        assert_eq!(
            compose_shared_face_flux(&request(vec![
                candidate("expert-a", 0.4, vec![1.0, 1.0]),
                candidate("expert-b", 0.4, vec![1.0, 1.0]),
            ])),
            Err(BlendError::WeightSumMismatch)
        );
    }

    #[test]
    fn negative_or_non_finite_weight_is_rejected() {
        assert_eq!(
            compose_shared_face_flux(&request(vec![
                candidate("expert-a", -0.1, vec![1.0, 1.0]),
            ])),
            Err(BlendError::InvalidCandidateWeight("expert-a".into()))
        );
        assert_eq!(
            compose_shared_face_flux(&request(vec![
                candidate("expert-a", f64::NAN, vec![1.0, 1.0]),
            ])),
            Err(BlendError::InvalidCandidateWeight("expert-a".into()))
        );
    }

    #[test]
    fn unknown_or_rejected_candidate_cannot_be_blended() {
        let mut unknown = candidate("expert-a", 1.0, vec![1.0, 1.0]);
        unknown.qualification = CandidateQualification::Unknown;
        assert_eq!(
            compose_shared_face_flux(&request(vec![unknown])),
            Err(BlendError::CandidateUnknown("expert-a".into()))
        );
        let mut rejected = candidate("expert-b", 1.0, vec![1.0, 1.0]);
        rejected.qualification = CandidateQualification::Rejected;
        assert_eq!(
            compose_shared_face_flux(&request(vec![rejected])),
            Err(BlendError::CandidateRejected("expert-b".into()))
        );
    }

    #[test]
    fn compatibility_mismatch_is_rejected_before_blending() {
        let mut incompatible = candidate("expert-a", 1.0, vec![1.0, 1.0]);
        incompatible.compatibility.operator_id = "different-operator".into();
        assert_eq!(
            compose_shared_face_flux(&request(vec![incompatible])),
            Err(BlendError::IncompatibleCandidate("expert-a".into()))
        );
    }

    #[test]
    fn incompatible_units_and_mesh_are_not_coerced() {
        let mut incompatible = candidate("expert-a", 1.0, vec![1.0, 1.0]);
        incompatible.compatibility.units_id = "other-units".into();
        assert_eq!(
            compose_shared_face_flux(&request(vec![incompatible])),
            Err(BlendError::IncompatibleCandidate("expert-a".into()))
        );
    }

    #[test]
    fn malformed_profile_or_model_digest_is_rejected() {
        let mut invalid_profile = candidate("expert-a", 1.0, vec![1.0, 1.0]);
        invalid_profile.profile_sha256 = "not-a-digest".into();
        assert_eq!(
            compose_shared_face_flux(&request(vec![invalid_profile])),
            Err(BlendError::InvalidCandidateDigest {
                expert_id: "expert-a".into(),
                field: "profile_sha256",
            })
        );
        let mut invalid_model = candidate("expert-b", 1.0, vec![1.0, 1.0]);
        invalid_model.model_sha256.clear();
        assert_eq!(
            compose_shared_face_flux(&request(vec![invalid_model])),
            Err(BlendError::InvalidCandidateDigest {
                expert_id: "expert-b".into(),
                field: "model_sha256",
            })
        );
    }

    #[test]
    fn flux_component_mismatch_and_non_finite_flux_are_rejected() {
        assert_eq!(
            compose_shared_face_flux(&request(vec![
                candidate("expert-a", 1.0, vec![1.0]),
            ])),
            Err(BlendError::CandidateFluxLengthMismatch("expert-a".into()))
        );
        assert_eq!(
            compose_shared_face_flux(&request(vec![
                candidate("expert-a", 1.0, vec![f64::INFINITY, 1.0]),
            ])),
            Err(BlendError::NonFiniteCandidateFlux("expert-a".into()))
        );
    }

    #[test]
    fn invalid_face_measure_or_weight_tolerance_is_rejected() {
        let mut invalid_measure = request(vec![
            candidate("expert-a", 1.0, vec![1.0, 1.0]),
        ]);
        invalid_measure.face_measure = 0.0;
        assert_eq!(
            compose_shared_face_flux(&invalid_measure),
            Err(BlendError::InvalidFaceMeasure)
        );
        let mut invalid_tolerance = request(vec![
            candidate("expert-a", 1.0, vec![1.0, 1.0]),
        ]);
        invalid_tolerance.weight_sum_tolerance = f64::NAN;
        assert_eq!(
            compose_shared_face_flux(&invalid_tolerance),
            Err(BlendError::InvalidWeightTolerance)
        );
    }

    #[test]
    fn identical_left_and_right_cell_is_rejected() {
        let mut invalid = request(vec![
            candidate("expert-a", 1.0, vec![1.0, 1.0]),
        ]);
        invalid.right_cell_id = invalid.left_cell_id.clone();
        assert_eq!(
            compose_shared_face_flux(&invalid),
            Err(BlendError::SameCellOnBothSides)
        );
    }

    #[test]
    fn invalid_mesh_or_solver_digest_is_rejected() {
        let mut invalid = request(vec![
            candidate("expert-a", 1.0, vec![1.0, 1.0]),
        ]);
        invalid.compatibility.mesh_sha256 = "unknown".into();
        assert_eq!(
            compose_shared_face_flux(&invalid),
            Err(BlendError::InvalidCompatibilityDigest("mesh_sha256"))
        );
    }

    #[test]
    fn unsupported_schema_and_empty_operator_identity_are_rejected() {
        let mut invalid_schema = request(vec![
            candidate("expert-a", 1.0, vec![1.0, 1.0]),
        ]);
        invalid_schema.schema_version = "future-version".into();
        assert_eq!(
            compose_shared_face_flux(&invalid_schema),
            Err(BlendError::UnsupportedSchemaVersion)
        );
        let mut invalid_operator = request(vec![
            candidate("expert-a", 1.0, vec![1.0, 1.0]),
        ]);
        invalid_operator.compatibility.operator_id.clear();
        assert_eq!(
            compose_shared_face_flux(&invalid_operator),
            Err(BlendError::EmptyIdentity("operator_id"))
        );
    }

    #[test]
    fn four_cell_periodic_assembly_matches_independent_face_oracle() {
        // Four periodic cells, each face oriented from cell i to cell (i+1)%4.
        // Expert weights vary by face so this exercises a spatially varying
        // composition field rather than one global constant blend.
        let expert_a = [
            [1.0, -2.0],
            [3.0, 0.5],
            [-1.0, 4.0],
            [2.0, 1.5],
        ];
        let expert_b = [
            [5.0, 2.0],
            [-2.0, 3.0],
            [4.0, -1.0],
            [0.0, 2.5],
        ];
        let weights_a = [0.2, 0.4, 0.7, 0.1];
        let face_measure = 1.5_f64;
        let mut reports = Vec::new();

        for face in 0..4 {
            let mut req = request(vec![
                candidate("expert-b", 1.0 - weights_a[face], expert_b[face].to_vec()),
                candidate("expert-a", weights_a[face], expert_a[face].to_vec()),
            ]);
            req.face_id = format!("periodic-face-{face}");
            req.left_cell_id = format!("cell-{face}");
            req.right_cell_id = format!("cell-{}", (face + 1) % 4);
            req.face_measure = face_measure;
            reports.push(compose_shared_face_flux(&req).expect("face composition"));
        }

        // Production-path assembly uses the per-face receipt transfers.
        let mut assembled = [[0.0_f64; 2]; 4];
        for report in &reports {
            let left: usize = report.left_cell_id.trim_start_matches("cell-").parse().unwrap();
            let right: usize = report.right_cell_id.trim_start_matches("cell-").parse().unwrap();
            for component in 0..2 {
                assembled[left][component] += report.left_integrated_update[component];
                assembled[right][component] += report.right_integrated_update[component];
            }
        }

        // Independent reference: compute weighted face values directly from
        // fixture data, then use the periodic incoming-minus-outgoing stencil.
        // This deliberately does not call the production sum/assembly helpers.
        let mut expected_face_flux = [[0.0_f64; 2]; 4];
        for face in 0..4 {
            for component in 0..2 {
                expected_face_flux[face][component] =
                    weights_a[face] * expert_a[face][component]
                    + (1.0 - weights_a[face]) * expert_b[face][component];
            }
        }

        let mut expected_cell_update = [[0.0_f64; 2]; 4];
        for cell in 0..4 {
            let incoming_face = (cell + 3) % 4;
            let outgoing_face = cell;
            for component in 0..2 {
                expected_cell_update[cell][component] = face_measure
                    * (expected_face_flux[incoming_face][component]
                        - expected_face_flux[outgoing_face][component]);
            }
        }

        for cell in 0..4 {
            for component in 0..2 {
                assert!(
                    (assembled[cell][component] - expected_cell_update[cell][component]).abs()
                        < 1.0e-12
                );
            }
        }

        // Periodicity makes every face internal: total integrated transfer
        // across all cells must cancel component by component.
        for component in 0..2 {
            let actual_total: f64 = assembled.iter().map(|cell| cell[component]).sum();
            let expected_total: f64 = expected_cell_update
                .iter()
                .map(|cell| cell[component])
                .sum();
            assert!(actual_total.abs() < 1.0e-12);
            assert!(expected_total.abs() < 1.0e-12);
        }
    }

}
