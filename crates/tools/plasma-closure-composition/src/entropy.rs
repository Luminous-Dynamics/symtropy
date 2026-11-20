//! Sampled-interface Tadmor entropy audit for convex flux composition.
//!
//! The module evaluates a declared entropy inequality for candidate fluxes
//! sharing the same entropy pair and left/right state identities. It proves no
//! global-in-state theorem and does not establish that the selected entropy is
//! physically valid for any particular plasma moment system.

use super::{
    BlendError, FaceFluxBlendRequest, compose_shared_face_flux,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateEntropyBinding {
    pub expert_id: String,
    pub profile_sha256: String,
    pub model_sha256: String,
    pub entropy_pair_sha256: String,
    pub left_state_sha256: String,
    pub right_state_sha256: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EntropyInterfaceContext {
    pub entropy_pair_sha256: String,
    pub left_state_sha256: String,
    pub right_state_sha256: String,
    /// Delta v = v(right) - v(left), in the declared entropy-variable units.
    pub entropy_variable_jump: Vec<f64>,
    /// Delta psi = psi(right) - psi(left), with the same orientation convention.
    pub entropy_potential_jump: f64,
    pub entropy_variable_units_id: String,
    pub entropy_potential_units_id: String,
    /// Absolute tolerance in the declared entropy-potential units.
    pub residual_tolerance: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EntropyBlendRequest {
    pub face: FaceFluxBlendRequest,
    pub context: EntropyInterfaceContext,
    /// One source-bound context per closure candidate. All must name the same
    /// entropy pair and exact left/right state fingerprints.
    pub candidate_bindings: Vec<CandidateEntropyBinding>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CandidateEntropyResidual {
    pub expert_id: String,
    /// Delta v dot candidate_flux - Delta psi.
    pub residual: f64,
    pub within_tolerance: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntropyDisposition {
    /// All candidates and their convex blend satisfy the sampled interface
    /// inequality for the declared common entropy pair and state fingerprints.
    AdmissibleAtSampledInterface,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EntropyBlendReport {
    pub schema_version: String,
    pub request_id: String,
    pub face_id: String,
    pub entropy_pair_sha256: String,
    pub left_state_sha256: String,
    pub right_state_sha256: String,
    pub entropy_variable_units_id: String,
    pub entropy_potential_units_id: String,
    pub candidate_residuals: Vec<CandidateEntropyResidual>,
    pub weighted_candidate_residual: f64,
    pub blended_flux_residual: f64,
    /// Difference between direct blended-flux evaluation and weighted
    /// candidate residuals. It should remain within roundoff-scale tolerance.
    pub convexity_identity_residual: f64,
    pub disposition: EntropyDisposition,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EntropyAuditError {
    Blend(BlendError),
    InvalidDigest(&'static str),
    EmptyIdentity(&'static str),
    InvalidResidualTolerance,
    NonFiniteEntropyData,
    EntropyVariableArityMismatch,
    CandidateBindingCountMismatch,
    DuplicateCandidateBinding(String),
    MissingCandidateBinding(String),
    ExtraCandidateBinding(String),
    CandidateBindingMismatch(String),
    WeightSumNotExactlyOne,
    CandidateEntropyViolation { expert_id: String, residual: f64 },
    BlendedEntropyViolation(f64),
    ConvexityIdentityMismatch(f64),
}

/// Evaluate Tadmor's interface condition:
///
///     (v_R - v_L) . F_hat <= psi_R - psi_L
///
/// Equivalently, residual = Delta v . F_hat - Delta psi <= 0.
///
/// For a fixed interface state pair and one common entropy pair, a convex
/// combination of candidate fluxes that each satisfies this inequality also
/// satisfies it algebraically. This function checks one supplied interface
/// sample numerically; it does not establish the condition for all states.
pub fn audit_entropy_stable_blend(
    request: &EntropyBlendRequest,
) -> Result<EntropyBlendReport, EntropyAuditError> {
    validate_context(&request.context)?;

    if request.context.entropy_variable_jump.len()
        != request.face.compatibility.component_count
    {
        return Err(EntropyAuditError::EntropyVariableArityMismatch);
    }

    let composition = compose_shared_face_flux(&request.face)
        .map_err(EntropyAuditError::Blend)?;

    // The convexity argument assumes weights sum to one. Unlike the generic
    // composition routine's configurable tolerance, this audit requires the
    // computed deterministic sum to be exactly 1.0.
    if composition.weight_sum != 1.0 {
        return Err(EntropyAuditError::WeightSumNotExactlyOne);
    }

    let bindings = validated_bindings(request)?;
    let mut candidate_residuals = Vec::with_capacity(request.face.candidates.len());
    let mut weighted_residual_terms = Vec::with_capacity(request.face.candidates.len());

    let mut candidates: Vec<_> = request.face.candidates.iter().collect();
    candidates.sort_by(|left, right| left.expert_id.cmp(&right.expert_id));

    for candidate in candidates {
        let binding = bindings
            .iter()
            .find(|binding| binding.expert_id == candidate.expert_id)
            .ok_or_else(|| EntropyAuditError::MissingCandidateBinding(candidate.expert_id.clone()))?;

        // The binding lookup above has already verified the source/profile/model
        // identities and common entropy/state context.
        let _ = binding;
        let residual = entropy_residual(
            &request.context.entropy_variable_jump,
            &candidate.flux,
            request.context.entropy_potential_jump,
        );
        if !residual.is_finite() {
            return Err(EntropyAuditError::NonFiniteEntropyData);
        }
        let within_tolerance = residual <= request.context.residual_tolerance;
        if !within_tolerance {
            return Err(EntropyAuditError::CandidateEntropyViolation {
                expert_id: candidate.expert_id.clone(),
                residual,
            });
        }
        weighted_residual_terms.push(candidate.weight * residual);
        candidate_residuals.push(CandidateEntropyResidual {
            expert_id: candidate.expert_id.clone(),
            residual,
            within_tolerance,
        });
    }

    let weighted_candidate_residual = compensated_sum(weighted_residual_terms);
    let blended_flux_residual = entropy_residual(
        &request.context.entropy_variable_jump,
        &composition.blended_flux,
        request.context.entropy_potential_jump,
    );
    let convexity_identity_residual =
        (blended_flux_residual - weighted_candidate_residual).abs();

    if !weighted_candidate_residual.is_finite()
        || !blended_flux_residual.is_finite()
        || !convexity_identity_residual.is_finite()
    {
        return Err(EntropyAuditError::NonFiniteEntropyData);
    }
    if blended_flux_residual > request.context.residual_tolerance {
        return Err(EntropyAuditError::BlendedEntropyViolation(blended_flux_residual));
    }
    if convexity_identity_residual > request.context.residual_tolerance {
        return Err(EntropyAuditError::ConvexityIdentityMismatch(
            convexity_identity_residual,
        ));
    }

    Ok(EntropyBlendReport {
        schema_version: super::SCHEMA_VERSION.to_owned(),
        request_id: request.face.request_id.clone(),
        face_id: request.face.face_id.clone(),
        entropy_pair_sha256: request.context.entropy_pair_sha256.clone(),
        left_state_sha256: request.context.left_state_sha256.clone(),
        right_state_sha256: request.context.right_state_sha256.clone(),
        entropy_variable_units_id: request.context.entropy_variable_units_id.clone(),
        entropy_potential_units_id: request.context.entropy_potential_units_id.clone(),
        candidate_residuals,
        weighted_candidate_residual,
        blended_flux_residual,
        convexity_identity_residual,
        disposition: EntropyDisposition::AdmissibleAtSampledInterface,
    })
}

fn validate_context(context: &EntropyInterfaceContext) -> Result<(), EntropyAuditError> {
    for (name, value) in [
        ("entropy_pair_sha256", context.entropy_pair_sha256.as_str()),
        ("left_state_sha256", context.left_state_sha256.as_str()),
        ("right_state_sha256", context.right_state_sha256.as_str()),
    ] {
        if !is_sha256(value) {
            return Err(EntropyAuditError::InvalidDigest(name));
        }
    }
    for (name, value) in [
        ("entropy_variable_units_id", context.entropy_variable_units_id.as_str()),
        ("entropy_potential_units_id", context.entropy_potential_units_id.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(EntropyAuditError::EmptyIdentity(name));
        }
    }
    if !context.residual_tolerance.is_finite()
        || context.residual_tolerance < 0.0
    {
        return Err(EntropyAuditError::InvalidResidualTolerance);
    }
    if !context.entropy_potential_jump.is_finite()
        || context.entropy_variable_jump.iter().any(|value| !value.is_finite())
    {
        return Err(EntropyAuditError::NonFiniteEntropyData);
    }
    Ok(())
}

fn validated_bindings<'a>(
    request: &'a EntropyBlendRequest,
) -> Result<Vec<&'a CandidateEntropyBinding>, EntropyAuditError> {
    if request.candidate_bindings.len() != request.face.candidates.len() {
        return Err(EntropyAuditError::CandidateBindingCountMismatch);
    }

    let mut bindings: Vec<&CandidateEntropyBinding> =
        request.candidate_bindings.iter().collect();
    bindings.sort_by(|left, right| left.expert_id.cmp(&right.expert_id));

    for (index, binding) in bindings.iter().enumerate() {
        if index > 0 && bindings[index - 1].expert_id == binding.expert_id {
            return Err(EntropyAuditError::DuplicateCandidateBinding(
                binding.expert_id.clone(),
            ));
        }
        for (field, value) in [
            ("profile_sha256", binding.profile_sha256.as_str()),
            ("model_sha256", binding.model_sha256.as_str()),
            ("entropy_pair_sha256", binding.entropy_pair_sha256.as_str()),
            ("left_state_sha256", binding.left_state_sha256.as_str()),
            ("right_state_sha256", binding.right_state_sha256.as_str()),
        ] {
            if !is_sha256(value) {
                return Err(EntropyAuditError::InvalidDigest(field));
            }
        }
        if binding.entropy_pair_sha256 != request.context.entropy_pair_sha256
            || binding.left_state_sha256 != request.context.left_state_sha256
            || binding.right_state_sha256 != request.context.right_state_sha256
        {
            return Err(EntropyAuditError::CandidateBindingMismatch(
                binding.expert_id.clone(),
            ));
        }
        let candidate = request
            .face
            .candidates
            .iter()
            .find(|candidate| candidate.expert_id == binding.expert_id)
            .ok_or_else(|| {
                EntropyAuditError::ExtraCandidateBinding(binding.expert_id.clone())
            })?;
        if candidate.profile_sha256 != binding.profile_sha256
            || candidate.model_sha256 != binding.model_sha256
        {
            return Err(EntropyAuditError::CandidateBindingMismatch(
                binding.expert_id.clone(),
            ));
        }
    }

    Ok(bindings)
}

fn entropy_residual(
    entropy_variable_jump: &[f64],
    flux: &[f64],
    entropy_potential_jump: f64,
) -> f64 {
    compensated_sum(
        entropy_variable_jump
            .iter()
            .zip(flux.iter())
            .map(|(jump, component)| jump * component),
    ) - entropy_potential_jump
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
        CandidateQualification, FaceCompatibility, FaceFluxCandidate,
        FaceFluxBlendRequest,
    };

    fn digest(character: char) -> String {
        character.to_string().repeat(64)
    }

    fn compatibility() -> FaceCompatibility {
        FaceCompatibility {
            physics_model_id: "synthetic-conservation-law".into(),
            conserved_state_id: "u".into(),
            closure_quantity_id: "fhat".into(),
            species_scope_id: "single-species".into(),
            normalization_id: "dimensionless".into(),
            units_id: "flux-units".into(),
            coordinate_frame_id: "lab".into(),
            geometry_id: "1d-uniform".into(),
            mesh_sha256: digest('c'),
            staggering_id: "cell-state-face-flux".into(),
            boundary_semantics_id: "internal-face-left-to-right".into(),
            operator_id: "synthetic-fv-v1".into(),
            solver_build_sha256: digest('d'),
            spatial_dimension: 1,
            component_count: 1,
        }
    }

    fn candidate(expert: &str, weight: f64, flux: f64, suffix: char) -> FaceFluxCandidate {
        FaceFluxCandidate {
            expert_id: expert.into(),
            profile_sha256: digest(suffix),
            model_sha256: digest('e'),
            qualification: CandidateQualification::Qualified,
            compatibility: compatibility(),
            weight,
            flux: vec![flux],
        }
    }

    fn request() -> EntropyBlendRequest {
        let face = FaceFluxBlendRequest {
            schema_version: crate::SCHEMA_VERSION.into(),
            request_id: "entropy-test-request".into(),
            face_id: "face-0-1".into(),
            left_cell_id: "cell-0".into(),
            right_cell_id: "cell-1".into(),
            face_measure: 1.0,
            weight_sum_tolerance: 1.0e-12,
            compatibility: compatibility(),
            candidates: vec![
                candidate("expert-a", 0.25, 0.5, 'a'),
                candidate("expert-b", 0.75, 0.75, 'b'),
            ],
        };
        let context = EntropyInterfaceContext {
            entropy_pair_sha256: digest('f'),
            left_state_sha256: digest('1'),
            right_state_sha256: digest('2'),
            entropy_variable_jump: vec![1.0],
            entropy_potential_jump: 1.0,
            entropy_variable_units_id: "dimensionless_entropy_variable".into(),
            entropy_potential_units_id: "dimensionless_entropy_potential".into(),
            residual_tolerance: 1.0e-12,
        };
        let candidate_bindings = face.candidates.iter().map(|candidate| {
            CandidateEntropyBinding {
                expert_id: candidate.expert_id.clone(),
                profile_sha256: candidate.profile_sha256.clone(),
                model_sha256: candidate.model_sha256.clone(),
                entropy_pair_sha256: context.entropy_pair_sha256.clone(),
                left_state_sha256: context.left_state_sha256.clone(),
                right_state_sha256: context.right_state_sha256.clone(),
            }
        }).collect();

        EntropyBlendRequest {
            face,
            context,
            candidate_bindings,
        }
    }

    #[test]
    fn common_entropy_pair_convex_blend_passes_sampled_interface() {
        let report = audit_entropy_stable_blend(&request()).expect("entropy audit");
        assert_eq!(report.disposition, EntropyDisposition::AdmissibleAtSampledInterface);
        assert!(report.candidate_residuals.iter().all(|item| item.within_tolerance));
        assert!(report.blended_flux_residual <= 0.0);
        assert!(report.convexity_identity_residual <= 1.0e-12);
    }

    #[test]
    fn candidate_violation_is_rejected_even_when_the_average_could_pass() {
        let mut req = request();
        // expert-b violates: 1.1 - 1.0 = +0.1.
        // The weighted average remains below zero, but the convex guarantee
        // requires every candidate itself to satisfy the common inequality.
        req.face.candidates[1].flux = vec![1.1];
        assert!(matches!(
            audit_entropy_stable_blend(&req),
            Err(EntropyAuditError::CandidateEntropyViolation { expert_id, .. })
                if expert_id == "expert-b"
        ));
    }

    #[test]
    fn different_entropy_pair_fails_closed() {
        let mut req = request();
        req.candidate_bindings[0].entropy_pair_sha256 = digest('9');
        assert!(matches!(
            audit_entropy_stable_blend(&req),
            Err(EntropyAuditError::CandidateBindingMismatch(_))
        ));
    }

    #[test]
    fn different_interface_state_fails_closed() {
        let mut req = request();
        req.candidate_bindings[1].right_state_sha256 = digest('8');
        assert!(matches!(
            audit_entropy_stable_blend(&req),
            Err(EntropyAuditError::CandidateBindingMismatch(_))
        ));
    }

    #[test]
    fn missing_or_duplicate_candidate_binding_is_rejected() {
        let mut missing = request();
        missing.candidate_bindings.pop();
        assert_eq!(
            audit_entropy_stable_blend(&missing),
            Err(EntropyAuditError::CandidateBindingCountMismatch)
        );

        let mut duplicate = request();
        duplicate.candidate_bindings[1].expert_id =
            duplicate.candidate_bindings[0].expert_id.clone();
        assert!(matches!(
            audit_entropy_stable_blend(&duplicate),
            Err(EntropyAuditError::DuplicateCandidateBinding(_))
        ));
    }

    #[test]
    fn malformed_entropy_identity_is_rejected() {
        let mut req = request();
        req.context.entropy_pair_sha256 = "not-a-digest".into();
        assert_eq!(
            audit_entropy_stable_blend(&req),
            Err(EntropyAuditError::InvalidDigest("entropy_pair_sha256"))
        );
    }

    #[test]
    fn exact_unit_weight_sum_is_required_for_convexity_audit() {
        let mut req = request();
        req.face.candidates[0].weight = 0.25;
        req.face.candidates[1].weight = 0.749999999999;
        assert_eq!(
            audit_entropy_stable_blend(&req),
            Err(EntropyAuditError::WeightSumNotExactlyOne)
        );
    }

    #[test]
    fn incompatible_entropy_vector_arity_is_rejected() {
        let mut req = request();
        req.context.entropy_variable_jump = vec![1.0, 2.0];
        assert_eq!(
            audit_entropy_stable_blend(&req),
            Err(EntropyAuditError::EntropyVariableArityMismatch)
        );
    }

    #[test]
    fn non_finite_entropy_context_is_rejected() {
        let mut req = request();
        req.context.entropy_potential_jump = f64::NAN;
        assert_eq!(
            audit_entropy_stable_blend(&req),
            Err(EntropyAuditError::NonFiniteEntropyData)
        );
    }


    #[test]
    fn convexity_holds_over_frozen_sweep_when_each_candidate_is_admissible() {
        // For Delta_v = 1 and Delta_psi = 0, every non-positive scalar
        // candidate flux satisfies the interface inequality. A convex blend
        // must also satisfy it. This is a finite sampled property sweep, not
        // a proof over all states or fluxes.
        for left_flux in -5..=0 {
            for right_flux in -5..=0 {
                let mut req = request();
                req.context.entropy_potential_jump = 0.0;
                req.face.candidates[0].flux = vec![f64::from(left_flux)];
                req.face.candidates[1].flux = vec![f64::from(right_flux)];

                let expected = 0.25 * f64::from(left_flux)
                    + 0.75 * f64::from(right_flux);
                let report = audit_entropy_stable_blend(&req).expect("stable convex blend");
                assert!(report.blended_flux_residual <= 0.0);
                assert!((report.blended_flux_residual - expected).abs() < 1.0e-12);
            }
        }
    }

    #[test]
    fn entropy_audit_refuses_zero_or_negative_tolerance() {
        let mut zero = request();
        zero.context.residual_tolerance = f64::NAN;
        assert_eq!(
            audit_entropy_stable_blend(&zero),
            Err(EntropyAuditError::InvalidResidualTolerance)
        );

        let mut negative = request();
        negative.context.residual_tolerance = -1.0;
        assert_eq!(
            audit_entropy_stable_blend(&negative),
            Err(EntropyAuditError::InvalidResidualTolerance)
        );
    }
}
