// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Dry-run request admission for Symthaea structural analysis proposals.
//!
//! This module is intentionally not a solver runner. It has no process,
//! filesystem, network, authority-token or backend-execution interface.
//! It compares supplied exact references, model applicability declarations,
//! evidence states and allowlists, then returns a restricted disposition.
//!
//! Digest strings are opaque caller-supplied identities. Syntax/equality
//! checking does not authenticate remote artifact bytes or prove that a
//! registry reference resolves.

use std::collections::BTreeSet;

/// Version of the receiver-side request contract.
pub const ADMISSION_SCHEMA_VERSION: &str = "space-struct-admission/0.1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdmissionDisposition {
    AdmissibleRestricted,
    NeedsEvidence,
    OutOfDomain,
    Reject,
    NotComparable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReasonCode {
    RequestIdentityMalformed,
    MissionIdentityMismatch,
    CandidateIdentityMismatch,
    GeometryIdentityMismatch,
    MaterialIdentityMismatch,
    ProcessBatchIdentityMismatch,
    LoadCaseIdentityMismatch,
    FrameIdentityMismatch,
    ModelProfileIdentityMismatch,
    BackendIdentityMismatch,
    BackendNotAllowlisted,
    MaterialEvidenceInsufficient,
    MaterialBatchUnresolved,
    MaterialProfileMismatch,
    MeshRequired,
    MeshIdentityMismatch,
    PhysicsOutOfDomain,
    ObservableOutOfDomain,
    IntendedUseExceedsClaimCeiling,
    DraftProfileRestricted,
    ReferenceComparisonNotComparable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntendedUse {
    SyntheticReferenceCheck,
    BoundedEngineeringScreen,
    PhysicalQualification,
}

impl IntendedUse {
    const fn rank(self) -> u8 {
        match self {
            Self::SyntheticReferenceCheck => 0,
            Self::BoundedEngineeringScreen => 1,
            Self::PhysicalQualification => 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelMaturity {
    DraftAnalyticalReference,
    SoftwareQualified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaterialEvidenceState {
    AnalyticalFixtureOnly,
    BatchQualifiedForProfile,
    ReferenceOnly,
    Unresolved,
    Rejected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhysicalValidationState {
    Unresolved,
}

/// Immutable subject reference supplied by a canonical owner.
///
/// A matching ID, generation and digest establish only equality with the
/// current admission context. The caller must separately resolve/authenticate
/// the referenced artifact at the integration boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExactRef {
    pub id: String,
    pub generation: String,
    pub digest: String,
}

impl ExactRef {
    pub fn new(id: &str, generation: &str, digest: &str) -> Self {
        Self {
            id: id.to_owned(),
            generation: generation.to_owned(),
            digest: digest.to_owned(),
        }
    }

    pub fn is_well_formed(&self) -> bool {
        !self.id.trim().is_empty()
            && !self.generation.trim().is_empty()
            && self.digest.len() == 64
            && self
                .digest
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PhysicsTerm {
    SteadyUniformRotation,
    ThinRingMembrane,
    SelfCentrifugalLoad,
    RadialAxialElasticity,
    RingSpokeCoupling,
    JointCompliance,
    FlexibleModes,
    GyroscopicDynamics,
    ShellMechanics,
    PressureBoundary,
    Fatigue,
    Contact,
    ThermalStress,
    NonlinearMaterial,
    Buckling,
    LinearElasticSolid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Observable {
    HoopStress,
    HoopStrain,
    RadialExtension,
    AxialForceDistribution,
    AxialStressDistribution,
    TipExtension,
    Displacement,
    Reaction,
    NaturalFrequency,
    ModeShape,
    JointLoad,
    PressureIntegrity,
    FatigueLife,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelProfile {
    pub reference: ExactRef,
    pub backend: ExactRef,
    pub maturity: ModelMaturity,
    pub max_intended_use: IntendedUse,
    pub supported_physics: Vec<PhysicsTerm>,
    pub supported_observables: Vec<Observable>,
    pub requires_mesh: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaterialEvidence {
    pub subject: ExactRef,
    pub state: MaterialEvidenceState,
    /// Exact profile ID for which a batch-property set was qualified.
    pub qualified_model_profile_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructuralAnalysisRequest {
    pub schema_version: String,
    pub request: ExactRef,
    pub mission: ExactRef,
    pub candidate: ExactRef,
    pub geometry: ExactRef,
    pub material: ExactRef,
    pub process_batch: Option<ExactRef>,
    pub load_case: ExactRef,
    pub frame: ExactRef,
    pub model_profile: ExactRef,
    pub backend: ExactRef,
    pub requested_physics: Vec<PhysicsTerm>,
    pub requested_observables: Vec<Observable>,
    pub intended_use: IntendedUse,
    pub mesh: Option<ExactRef>,
    pub comparison_reference: Option<ExactRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmissionContext {
    pub current_mission: ExactRef,
    pub current_candidate: ExactRef,
    pub current_geometry: ExactRef,
    pub current_load_case: ExactRef,
    pub current_frame: ExactRef,
    pub material_evidence: MaterialEvidence,
    pub accepted_process_batch: Option<ExactRef>,
    pub model_profile: ModelProfile,
    pub backend_allowlisted: bool,
    pub expected_mesh: Option<ExactRef>,
    pub compatible_comparison_reference: Option<ExactRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmissionResult {
    pub disposition: AdmissionDisposition,
    pub reason_codes: Vec<ReasonCode>,
    pub model_maturity: ModelMaturity,
    pub claim_ceiling: IntendedUse,
    pub solver_dispatch_started: bool,
    pub execution_authorized: bool,
    pub decision_eligible: bool,
    pub physical_validation: PhysicalValidationState,
}

impl AdmissionResult {
    fn new(
        disposition: AdmissionDisposition,
        reason_codes: Vec<ReasonCode>,
        model_maturity: ModelMaturity,
        claim_ceiling: IntendedUse,
    ) -> Self {
        Self {
            disposition,
            reason_codes,
            model_maturity,
            claim_ceiling,
            solver_dispatch_started: false,
            execution_authorized: false,
            decision_eligible: false,
            physical_validation: PhysicalValidationState::Unresolved,
        }
    }
}

/// Pure, dry-run admission.
///
/// No code path in this function can launch a solver. The most favorable
/// disposition is ADMISSIBLE_RESTRICTED; execution and design authority remain
/// separate system boundaries.
pub fn admit(request: &StructuralAnalysisRequest, context: &AdmissionContext) -> AdmissionResult {
    let mut reject = Vec::new();
    let mut out_of_domain = Vec::new();
    let mut needs_evidence = Vec::new();
    let mut not_comparable = Vec::new();
    let mut informational = Vec::new();

    if request.schema_version != ADMISSION_SCHEMA_VERSION {
        reject.push(ReasonCode::RequestIdentityMalformed);
    }

    let mut references = vec![
        &request.request,
        &request.mission,
        &request.candidate,
        &request.geometry,
        &request.material,
        &request.load_case,
        &request.frame,
        &request.model_profile,
        &request.backend,
        &context.current_mission,
        &context.current_candidate,
        &context.current_geometry,
        &context.current_load_case,
        &context.current_frame,
        &context.material_evidence.subject,
        &context.model_profile.reference,
        &context.model_profile.backend,
    ];
    if let Some(process_batch) = request.process_batch.as_ref() {
        references.push(process_batch);
    }
    if let Some(process_batch) = context.accepted_process_batch.as_ref() {
        references.push(process_batch);
    }
    if let Some(mesh) = request.mesh.as_ref() {
        references.push(mesh);
    }
    if let Some(mesh) = context.expected_mesh.as_ref() {
        references.push(mesh);
    }
    if let Some(reference) = request.comparison_reference.as_ref() {
        references.push(reference);
    }
    if let Some(reference) = context.compatible_comparison_reference.as_ref() {
        references.push(reference);
    }

    if references
        .iter()
        .any(|reference| !reference.is_well_formed())
    {
        reject.push(ReasonCode::RequestIdentityMalformed);
    }

    if request.requested_physics.is_empty() || request.requested_observables.is_empty() {
        reject.push(ReasonCode::RequestIdentityMalformed);
    }
    let unique_physics: BTreeSet<_> = request.requested_physics.iter().collect();
    if unique_physics.len() != request.requested_physics.len() {
        reject.push(ReasonCode::RequestIdentityMalformed);
    }
    let unique_observables: BTreeSet<_> = request.requested_observables.iter().collect();
    if unique_observables.len() != request.requested_observables.len() {
        reject.push(ReasonCode::RequestIdentityMalformed);
    }

    if request.mission != context.current_mission {
        reject.push(ReasonCode::MissionIdentityMismatch);
    }
    if request.candidate != context.current_candidate {
        reject.push(ReasonCode::CandidateIdentityMismatch);
    }
    if request.geometry != context.current_geometry {
        reject.push(ReasonCode::GeometryIdentityMismatch);
    }
    if request.material != context.material_evidence.subject {
        reject.push(ReasonCode::MaterialIdentityMismatch);
    }
    match (&request.process_batch, &context.accepted_process_batch) {
        (Some(requested), Some(accepted)) if requested != accepted => {
            reject.push(ReasonCode::ProcessBatchIdentityMismatch);
        }
        (Some(_), None) | (None, Some(_)) => {
            needs_evidence.push(ReasonCode::MaterialBatchUnresolved);
        }
        _ => {}
    }
    if context.material_evidence.state == MaterialEvidenceState::BatchQualifiedForProfile
        && request.process_batch.is_none()
    {
        needs_evidence.push(ReasonCode::MaterialBatchUnresolved);
    }
    if request.load_case != context.current_load_case {
        reject.push(ReasonCode::LoadCaseIdentityMismatch);
    }
    if request.frame != context.current_frame {
        reject.push(ReasonCode::FrameIdentityMismatch);
    }
    if request.model_profile != context.model_profile.reference {
        reject.push(ReasonCode::ModelProfileIdentityMismatch);
    }
    if request.backend != context.model_profile.backend {
        reject.push(ReasonCode::BackendIdentityMismatch);
    }
    if !context.backend_allowlisted {
        reject.push(ReasonCode::BackendNotAllowlisted);
    }

    if request.intended_use.rank() > context.model_profile.max_intended_use.rank() {
        out_of_domain.push(ReasonCode::IntendedUseExceedsClaimCeiling);
    }
    if context.model_profile.maturity == ModelMaturity::DraftAnalyticalReference {
        informational.push(ReasonCode::DraftProfileRestricted);
        if request.intended_use != IntendedUse::SyntheticReferenceCheck {
            out_of_domain.push(ReasonCode::DraftProfileRestricted);
        }
    }

    if request
        .requested_physics
        .iter()
        .any(|term| !context.model_profile.supported_physics.contains(term))
    {
        out_of_domain.push(ReasonCode::PhysicsOutOfDomain);
    }
    if request.requested_observables.iter().any(|observable| {
        !context
            .model_profile
            .supported_observables
            .contains(observable)
    }) {
        out_of_domain.push(ReasonCode::ObservableOutOfDomain);
    }

    match context.material_evidence.state {
        MaterialEvidenceState::AnalyticalFixtureOnly => {
            if request.intended_use != IntendedUse::SyntheticReferenceCheck {
                needs_evidence.push(ReasonCode::MaterialEvidenceInsufficient);
            }
        }
        MaterialEvidenceState::BatchQualifiedForProfile => {
            if context
                .material_evidence
                .qualified_model_profile_id
                .as_deref()
                != Some(context.model_profile.reference.id.as_str())
            {
                needs_evidence.push(ReasonCode::MaterialProfileMismatch);
            }
        }
        MaterialEvidenceState::ReferenceOnly | MaterialEvidenceState::Unresolved => {
            needs_evidence.push(ReasonCode::MaterialEvidenceInsufficient);
            needs_evidence.push(ReasonCode::MaterialBatchUnresolved);
        }
        MaterialEvidenceState::Rejected => {
            reject.push(ReasonCode::MaterialEvidenceInsufficient);
        }
    }

    if context.model_profile.requires_mesh {
        match (&request.mesh, &context.expected_mesh) {
            (None, _) | (Some(_), None) => needs_evidence.push(ReasonCode::MeshRequired),
            (Some(requested), Some(expected)) if requested != expected => {
                reject.push(ReasonCode::MeshIdentityMismatch);
            }
            (Some(_), Some(_)) => {}
        }
    } else if request.mesh.is_some() {
        match (&request.mesh, &context.expected_mesh) {
            (Some(requested), Some(expected)) if requested == expected => {}
            _ => reject.push(ReasonCode::MeshIdentityMismatch),
        }
    }

    if let Some(requested_reference) = request.comparison_reference.as_ref() {
        match context.compatible_comparison_reference.as_ref() {
            Some(compatible) if requested_reference == compatible => {}
            _ => not_comparable.push(ReasonCode::ReferenceComparisonNotComparable),
        }
    }

    let (disposition, mut reasons) = if !reject.is_empty() {
        (AdmissionDisposition::Reject, reject)
    } else if !out_of_domain.is_empty() {
        (AdmissionDisposition::OutOfDomain, out_of_domain)
    } else if !needs_evidence.is_empty() {
        (AdmissionDisposition::NeedsEvidence, needs_evidence)
    } else if !not_comparable.is_empty() {
        (AdmissionDisposition::NotComparable, not_comparable)
    } else {
        (AdmissionDisposition::AdmissibleRestricted, informational)
    };

    reasons.sort();
    reasons.dedup();

    AdmissionResult::new(
        disposition,
        reasons,
        context.model_profile.maturity,
        context.model_profile.max_intended_use,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reference(id: &str, generation: &str, digit: char) -> ExactRef {
        ExactRef::new(id, generation, &digit.to_string().repeat(64))
    }

    fn ring_profile() -> ModelProfile {
        ModelProfile {
            reference: reference("ARTGRAV-THIN-RING-REF-V0_1", "p1", 'a'),
            backend: reference("artgrav-prestress-reference", "b1", 'b'),
            maturity: ModelMaturity::DraftAnalyticalReference,
            max_intended_use: IntendedUse::SyntheticReferenceCheck,
            supported_physics: vec![
                PhysicsTerm::SteadyUniformRotation,
                PhysicsTerm::ThinRingMembrane,
            ],
            supported_observables: vec![
                Observable::HoopStress,
                Observable::HoopStrain,
                Observable::RadialExtension,
            ],
            requires_mesh: false,
        }
    }

    fn base_request(profile: &ModelProfile) -> StructuralAnalysisRequest {
        StructuralAnalysisRequest {
            schema_version: ADMISSION_SCHEMA_VERSION.to_owned(),
            request: reference("REQUEST-RING-001", "r1", '1'),
            mission: reference("MISSION-SPACE-STRUCT-001", "m1", '9'),
            candidate: reference("AG-RING-REF-001", "c1", '2'),
            geometry: reference("RING-GEOM-A", "g1", '3'),
            material: reference("MAT-ANALYTIC-A", "mat1", '4'),
            process_batch: None,
            load_case: reference("LOAD-ROT-STEADY", "l1", '5'),
            frame: reference("FRAME-RING-LOCAL", "f1", '6'),
            model_profile: profile.reference.clone(),
            backend: profile.backend.clone(),
            requested_physics: vec![
                PhysicsTerm::SteadyUniformRotation,
                PhysicsTerm::ThinRingMembrane,
            ],
            requested_observables: vec![
                Observable::HoopStress,
                Observable::HoopStrain,
                Observable::RadialExtension,
            ],
            intended_use: IntendedUse::SyntheticReferenceCheck,
            mesh: None,
            comparison_reference: None,
        }
    }

    fn base_context(profile: ModelProfile) -> AdmissionContext {
        AdmissionContext {
            current_mission: reference("MISSION-SPACE-STRUCT-001", "m1", '9'),
            current_candidate: reference("AG-RING-REF-001", "c1", '2'),
            current_geometry: reference("RING-GEOM-A", "g1", '3'),
            current_load_case: reference("LOAD-ROT-STEADY", "l1", '5'),
            current_frame: reference("FRAME-RING-LOCAL", "f1", '6'),
            material_evidence: MaterialEvidence {
                subject: reference("MAT-ANALYTIC-A", "m1", '4'),
                state: MaterialEvidenceState::AnalyticalFixtureOnly,
                qualified_model_profile_id: None,
            },
            accepted_process_batch: None,
            backend_allowlisted: true,
            model_profile: profile,
            expected_mesh: None,
            compatible_comparison_reference: None,
        }
    }

    fn assert_reason(result: &AdmissionResult, expected: ReasonCode) {
        assert!(
            result.reason_codes.contains(&expected),
            "missing {expected:?} in {:?}",
            result.reason_codes
        );
    }

    #[test]
    fn valid_draft_ring_request_is_admissible_only_for_synthetic_reference() {
        let profile = ring_profile();
        let request = base_request(&profile);
        let context = base_context(profile);
        let result = admit(&request, &context);

        assert_eq!(
            result.disposition,
            AdmissionDisposition::AdmissibleRestricted
        );
        assert_reason(&result, ReasonCode::DraftProfileRestricted);
        assert!(!result.solver_dispatch_started);
        assert!(!result.execution_authorized);
        assert!(!result.decision_eligible);
        assert_eq!(
            result.physical_validation,
            PhysicalValidationState::Unresolved
        );
    }

    #[test]
    fn stale_geometry_is_rejected() {
        let profile = ring_profile();
        let mut request = base_request(&profile);
        request.geometry = reference("RING-GEOM-A", "g0", '3');
        let result = admit(&request, &base_context(profile));

        assert_eq!(result.disposition, AdmissionDisposition::Reject);
        assert_reason(&result, ReasonCode::GeometryIdentityMismatch);
    }

    #[test]
    fn stale_mission_identity_is_rejected() {
        let profile = ring_profile();
        let mut request = base_request(&profile);
        request.mission = reference("MISSION-SPACE-STRUCT-001", "m0", '9');
        let result = admit(&request, &base_context(profile));

        assert_eq!(result.disposition, AdmissionDisposition::Reject);
        assert_reason(&result, ReasonCode::MissionIdentityMismatch);
    }

    #[test]
    fn missing_process_batch_reference_needs_evidence_for_batch_material() {
        let profile = ring_profile();
        let request = base_request(&profile);
        let mut context = base_context(profile);
        context.material_evidence.state = MaterialEvidenceState::BatchQualifiedForProfile;
        context.material_evidence.qualified_model_profile_id =
            Some("ARTGRAV-THIN-RING-REF-V0_1".to_owned());
        let result = admit(&request, &context);

        assert_eq!(result.disposition, AdmissionDisposition::NeedsEvidence);
        assert_reason(&result, ReasonCode::MaterialBatchUnresolved);
    }

    #[test]
    fn mismatched_process_batch_identity_is_rejected() {
        let profile = ring_profile();
        let mut request = base_request(&profile);
        request.process_batch = Some(reference("LUNAR-BATCH-A", "b1", '7'));
        let mut context = base_context(profile);
        context.accepted_process_batch = Some(reference("LUNAR-BATCH-B", "b1", '8'));
        let result = admit(&request, &context);

        assert_eq!(result.disposition, AdmissionDisposition::Reject);
        assert_reason(&result, ReasonCode::ProcessBatchIdentityMismatch);
    }

    #[test]
    fn stale_candidate_generation_is_rejected() {
        let profile = ring_profile();
        let mut request = base_request(&profile);
        request.candidate = reference("AG-RING-REF-001", "c0", '2');
        let result = admit(&request, &base_context(profile));

        assert_eq!(result.disposition, AdmissionDisposition::Reject);
        assert_reason(&result, ReasonCode::CandidateIdentityMismatch);
    }

    #[test]
    fn reference_only_material_cannot_support_engineering_screen() {
        let mut profile = ring_profile();
        profile.maturity = ModelMaturity::SoftwareQualified;
        profile.max_intended_use = IntendedUse::BoundedEngineeringScreen;
        let mut request = base_request(&profile);
        request.intended_use = IntendedUse::BoundedEngineeringScreen;
        let mut context = base_context(profile);
        context.material_evidence.state = MaterialEvidenceState::ReferenceOnly;
        let result = admit(&request, &context);

        assert_eq!(result.disposition, AdmissionDisposition::NeedsEvidence);
        assert_reason(&result, ReasonCode::MaterialEvidenceInsufficient);
        assert_reason(&result, ReasonCode::MaterialBatchUnresolved);
    }

    #[test]
    fn draft_profile_blocks_bounded_engineering_screen() {
        let profile = ring_profile();
        let mut request = base_request(&profile);
        request.intended_use = IntendedUse::BoundedEngineeringScreen;
        let mut context = base_context(profile);
        context.material_evidence.state = MaterialEvidenceState::BatchQualifiedForProfile;
        context.material_evidence.qualified_model_profile_id =
            Some("ARTGRAV-THIN-RING-REF-V0_1".to_owned());
        let process_batch = reference("LUNAR-BATCH-001", "batch1", '8');
        request.process_batch = Some(process_batch.clone());
        context.accepted_process_batch = Some(process_batch);
        let result = admit(&request, &context);

        assert_eq!(result.disposition, AdmissionDisposition::OutOfDomain);
        assert_reason(&result, ReasonCode::IntendedUseExceedsClaimCeiling);
        assert_reason(&result, ReasonCode::DraftProfileRestricted);
    }

    #[test]
    fn unallowlisted_backend_is_rejected() {
        let profile = ring_profile();
        let request = base_request(&profile);
        let mut context = base_context(profile);
        context.backend_allowlisted = false;
        let result = admit(&request, &context);

        assert_eq!(result.disposition, AdmissionDisposition::Reject);
        assert_reason(&result, ReasonCode::BackendNotAllowlisted);
    }

    #[test]
    fn unsupported_gyroscopic_physics_and_frequency_are_out_of_domain() {
        let profile = ring_profile();
        let mut request = base_request(&profile);
        request
            .requested_physics
            .push(PhysicsTerm::GyroscopicDynamics);
        request
            .requested_observables
            .push(Observable::NaturalFrequency);
        let result = admit(&request, &base_context(profile));

        assert_eq!(result.disposition, AdmissionDisposition::OutOfDomain);
        assert_reason(&result, ReasonCode::PhysicsOutOfDomain);
        assert_reason(&result, ReasonCode::ObservableOutOfDomain);
    }

    #[test]
    fn physical_qualification_is_never_granted_by_a_draft_analytical_profile() {
        let profile = ring_profile();
        let mut request = base_request(&profile);
        request.intended_use = IntendedUse::PhysicalQualification;
        let result = admit(&request, &base_context(profile));

        assert_eq!(result.disposition, AdmissionDisposition::OutOfDomain);
        assert_reason(&result, ReasonCode::IntendedUseExceedsClaimCeiling);
        assert!(!result.decision_eligible);
        assert_eq!(
            result.physical_validation,
            PhysicalValidationState::Unresolved
        );
    }

    #[test]
    fn required_mesh_missing_needs_evidence() {
        let mut profile = ring_profile();
        profile.requires_mesh = true;
        let request = base_request(&profile);
        let mut context = base_context(profile);
        context.expected_mesh = Some(reference("MESH-001", "mesh1", '7'));
        let result = admit(&request, &context);

        assert_eq!(result.disposition, AdmissionDisposition::NeedsEvidence);
        assert_reason(&result, ReasonCode::MeshRequired);
    }

    #[test]
    fn hidden_or_changed_mesh_is_rejected() {
        let mut profile = ring_profile();
        profile.requires_mesh = true;
        let mut request = base_request(&profile);
        request.mesh = Some(reference("MESH-OLD", "mesh0", '7'));
        let mut context = base_context(profile);
        context.expected_mesh = Some(reference("MESH-001", "mesh1", '8'));
        let result = admit(&request, &context);

        assert_eq!(result.disposition, AdmissionDisposition::Reject);
        assert_reason(&result, ReasonCode::MeshIdentityMismatch);
    }

    #[test]
    fn comparison_with_wrong_reference_is_not_comparable() {
        let profile = ring_profile();
        let mut request = base_request(&profile);
        request.comparison_reference = Some(reference("REFERENCE-OTHER-FRAME", "v1", '9'));
        let mut context = base_context(profile);
        context.compatible_comparison_reference =
            Some(reference("REFERENCE-CURRENT-FRAME", "v1", 'a'));
        let result = admit(&request, &context);

        assert_eq!(result.disposition, AdmissionDisposition::NotComparable);
        assert_reason(&result, ReasonCode::ReferenceComparisonNotComparable);
    }

    #[test]
    fn malformed_digest_is_rejected_without_attempting_resolution() {
        let profile = ring_profile();
        let mut request = base_request(&profile);
        request.candidate.digest = "not-a-digest".to_owned();
        let result = admit(&request, &base_context(profile));

        assert_eq!(result.disposition, AdmissionDisposition::Reject);
        assert_reason(&result, ReasonCode::RequestIdentityMalformed);
    }

    #[test]
    fn duplicate_requested_observable_is_rejected() {
        let profile = ring_profile();
        let mut request = base_request(&profile);
        request.requested_observables.push(Observable::HoopStress);
        let result = admit(&request, &base_context(profile));

        assert_eq!(result.disposition, AdmissionDisposition::Reject);
        assert_reason(&result, ReasonCode::RequestIdentityMalformed);
    }

    #[test]
    fn batch_properties_must_be_qualified_for_the_selected_profile() {
        let profile = ring_profile();
        let mut request = base_request(&profile);
        request.intended_use = IntendedUse::BoundedEngineeringScreen;
        let mut context = base_context(profile);
        context.model_profile.maturity = ModelMaturity::SoftwareQualified;
        context.model_profile.max_intended_use = IntendedUse::BoundedEngineeringScreen;
        context.material_evidence.state = MaterialEvidenceState::BatchQualifiedForProfile;
        context.material_evidence.qualified_model_profile_id =
            Some("DIFFERENT-MODEL-PROFILE".to_owned());
        let result = admit(&request, &context);

        assert_eq!(result.disposition, AdmissionDisposition::NeedsEvidence);
        assert_reason(&result, ReasonCode::MaterialProfileMismatch);
    }

    #[test]
    fn empty_physics_or_observable_set_is_rejected() {
        let profile = ring_profile();
        let mut request = base_request(&profile);
        request.requested_observables.clear();
        let result = admit(&request, &base_context(profile));

        assert_eq!(result.disposition, AdmissionDisposition::Reject);
        assert_reason(&result, ReasonCode::RequestIdentityMalformed);
    }

    #[test]
    fn wrong_model_profile_reference_is_rejected() {
        let profile = ring_profile();
        let mut request = base_request(&profile);
        request.model_profile = reference("OTHER-PROFILE", "p1", 'a');
        let result = admit(&request, &base_context(profile));

        assert_eq!(result.disposition, AdmissionDisposition::Reject);
        assert_reason(&result, ReasonCode::ModelProfileIdentityMismatch);
    }

    #[test]
    fn rejected_material_evidence_is_rejected() {
        let profile = ring_profile();
        let request = base_request(&profile);
        let mut context = base_context(profile);
        context.material_evidence.state = MaterialEvidenceState::Rejected;
        let result = admit(&request, &context);

        assert_eq!(result.disposition, AdmissionDisposition::Reject);
        assert_reason(&result, ReasonCode::MaterialEvidenceInsufficient);
    }
}
