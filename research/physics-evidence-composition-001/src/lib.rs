// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Fail-closed composition of independently verified evidence receipts.
//!
//! This module checks applicability, identity binding, claim scope and required
//! evidence coverage. It does not repeat the owning mathematics or physics
//! calculations, hash external artifacts, fetch source commits, or verify
//! signatures. Upstream receipts must already have been independently verified.

use std::collections::BTreeMap;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EvidenceKindV1 {
    MathematicalSymmetry,
    CoordinateCovariance,
    OperatorSpectrum,
    ConstraintDelassus,
    KktResidual,
    NonlinearSensitivity,
    DiscreteStep,
    /// Independently reconstructed discrete energy/work and impulse-work balance.
    EnergyWorkLedger,
    TaskSemantics,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerificationStatusV1 {
    Pass,
    Fail,
    Inconclusive,
    NotApplicable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MathematicalEvidenceClassV1 {
    LearnedProposal,
    SampledNumeric,
    NumericLieBasis,
    ExactPolynomialIdentity,
    ExactFiniteGroup,
    FormalProof,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompositionClaimV1 {
    PhysicalStepEquivalence,
    TaskEquivalence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompositionStatusV1 {
    Pass,
    Rejected,
    Inconclusive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClaimAuthorityV1 {
    None,
    BoundedPhysicalStepOnly,
    FullTaskEquivalence,
}

/// Immutable identity tuple that every receipt in a composition must share.
///
/// The strings are already-canonical identifiers/digests created by the
/// owning fixture/verifier. The composer checks equality and syntax only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProblemBindingV1 {
    pub problem_id: String,
    pub model_revision_sha256: String,
    pub fixture_sha256: String,
    pub initial_state_sha256: String,
    pub units_profile_id: String,
    pub integration_method_id: String,
    pub timestep_nanoseconds: u64,
    pub solver_profile_id: String,
    pub constraint_model_sha256: String,
    pub observable_map_sha256: String,
    /// Absent when the requested claim is only physical-step equivalence.
    pub task_semantics_sha256: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameBindingV1 {
    pub coordinate_frame_id: String,
    pub coordinate_frame_sha256: String,
    pub metric_sha256: String,
}

/// The covariance witness explicitly relates source and target frames.
/// It does not make arbitrary representations equivalent: the owning
/// coordinate-covariance checker must already have verified this artifact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoordinateRelationV1 {
    pub source_frame_sha256: String,
    pub source_metric_sha256: String,
    pub target_frame_sha256: String,
    pub target_metric_sha256: String,
    pub basis_transform_sha256: String,
    pub covariance_receipt_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrameApplicabilityV1 {
    Source(FrameBindingV1),
    Target(FrameBindingV1),
    RepresentationInvariant,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceReceiptV1 {
    pub kind: EvidenceKindV1,
    pub verification_status: VerificationStatusV1,
    pub artifact_sha256: String,
    pub verifier_id: String,
    pub verifier_version: String,
    pub verifier_commit_sha: String,
    pub binding: ProblemBindingV1,
    /// Declares which coordinate representation the owning checker used.
    /// Task semantics are representation-independent in this v0.1 contract.
    pub frame_applicability: FrameApplicabilityV1,
    /// Present only for CoordinateCovariance evidence.
    pub coordinate_relation: Option<CoordinateRelationV1>,
    /// Present for nonlinear sensitivity evidence.
    pub valid_perturbation_radius_nanometers: Option<u64>,
    /// Present for MathematicalSymmetry evidence.
    pub mathematical_evidence_class: Option<MathematicalEvidenceClassV1>,
    /// Stable claim-scope identifier, e.g. one-step-dumbbell-v1.
    pub claim_scope_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpectedVerifierIdentityV1 {
    pub kind: EvidenceKindV1,
    pub verifier_id: String,
    pub verifier_version: String,
    pub verifier_commit_sha: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompositionRequestV1 {
    pub claim: CompositionClaimV1,
    pub binding: ProblemBindingV1,
    pub source_frame: FrameBindingV1,
    pub target_frame: FrameBindingV1,
    pub basis_transform_sha256: String,
    pub minimum_mathematical_evidence_class: MathematicalEvidenceClassV1,
    pub required_perturbation_radius_nanometers: u64,
    pub accepted_claim_scope_id: String,
    /// Exact, pre-approved verifier identities for every required evidence kind.
    pub expected_verifiers: Vec<ExpectedVerifierIdentityV1>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompositionErrorV1 {
    InvalidIdentifier(&'static str),
    InvalidSha256(&'static str),
    InvalidGitSha(&'static str),
    InvalidTimestep,
    MissingTaskSemanticsBinding,
    DuplicateEvidenceKind(EvidenceKindV1),
    DuplicateExpectedVerifier(EvidenceKindV1),
    MissingExpectedVerifier(EvidenceKindV1),
    UnexpectedExpectedVerifier(EvidenceKindV1),
    VerifierIdentityMismatch(EvidenceKindV1),
    UnexpectedCoordinateRelation(EvidenceKindV1),
    UnexpectedMathematicalEvidenceClass(EvidenceKindV1),
    UnexpectedPerturbationRadius(EvidenceKindV1),
    MissingCoordinateRelation,
    MissingEvidence(EvidenceKindV1),
    BindingMismatch(EvidenceKindV1),
    FrameMismatch(EvidenceKindV1),
    CoordinateRelationMismatch,
    ClaimScopeMismatch(EvidenceKindV1),
    MathematicalEvidenceTooWeak,
    MathematicalEvidenceClassMissing,
    PerturbationRadiusInsufficient,
    ReceiptFailed(EvidenceKindV1),
}

impl fmt::Display for CompositionErrorV1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for CompositionErrorV1 {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceUseV1 {
    pub kind: EvidenceKindV1,
    pub artifact_sha256: String,
    pub verifier_id: String,
    pub verifier_version: String,
    pub verifier_commit_sha: String,
    pub frame_applicability: FrameApplicabilityV1,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceCompositionReceiptV1 {
    pub schema_version: u32,
    pub claim: CompositionClaimV1,
    pub physical_step_status: CompositionStatusV1,
    pub task_equivalence_status: CompositionStatusV1,
    pub physical_authority: ClaimAuthorityV1,
    pub task_authority: ClaimAuthorityV1,
    /// Hard pruning is deliberately unavailable in this composition layer.
    pub scientific_search_pruning_authorized: bool,
    pub binding: ProblemBindingV1,
    pub source_frame: FrameBindingV1,
    pub target_frame: FrameBindingV1,
    pub evidence: Vec<EvidenceUseV1>,
    pub composition_scope_id: String,
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._:-".contains(&byte))
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn valid_git_sha(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn validate_problem_binding(binding: &ProblemBindingV1) -> Result<(), CompositionErrorV1> {
    if binding.timestep_nanoseconds == 0 {
        return Err(CompositionErrorV1::InvalidTimestep);
    }
    for (name, value) in [
        ("problem_id", binding.problem_id.as_str()),
        ("units_profile_id", binding.units_profile_id.as_str()),
        (
            "integration_method_id",
            binding.integration_method_id.as_str(),
        ),
        ("solver_profile_id", binding.solver_profile_id.as_str()),
    ] {
        if !valid_identifier(value) {
            return Err(CompositionErrorV1::InvalidIdentifier(name));
        }
    }
    for (name, value) in [
        (
            "model_revision_sha256",
            binding.model_revision_sha256.as_str(),
        ),
        ("fixture_sha256", binding.fixture_sha256.as_str()),
        (
            "initial_state_sha256",
            binding.initial_state_sha256.as_str(),
        ),
        (
            "constraint_model_sha256",
            binding.constraint_model_sha256.as_str(),
        ),
        (
            "observable_map_sha256",
            binding.observable_map_sha256.as_str(),
        ),
    ] {
        if !valid_sha256(value) {
            return Err(CompositionErrorV1::InvalidSha256(name));
        }
    }
    if let Some(task_digest) = &binding.task_semantics_sha256 {
        if !valid_sha256(task_digest) {
            return Err(CompositionErrorV1::InvalidSha256("task_semantics_sha256"));
        }
    }
    Ok(())
}

fn validate_frame(frame: &FrameBindingV1) -> Result<(), CompositionErrorV1> {
    if !valid_identifier(&frame.coordinate_frame_id) {
        return Err(CompositionErrorV1::InvalidIdentifier("coordinate_frame_id"));
    }
    for (name, value) in [
        (
            "coordinate_frame_sha256",
            frame.coordinate_frame_sha256.as_str(),
        ),
        ("metric_sha256", frame.metric_sha256.as_str()),
    ] {
        if !valid_sha256(value) {
            return Err(CompositionErrorV1::InvalidSha256(name));
        }
    }
    Ok(())
}

fn required_kinds(claim: CompositionClaimV1) -> &'static [EvidenceKindV1] {
    const PHYSICAL_STEP: &[EvidenceKindV1] = &[
        EvidenceKindV1::MathematicalSymmetry,
        EvidenceKindV1::CoordinateCovariance,
        EvidenceKindV1::OperatorSpectrum,
        EvidenceKindV1::ConstraintDelassus,
        EvidenceKindV1::KktResidual,
        EvidenceKindV1::NonlinearSensitivity,
        EvidenceKindV1::DiscreteStep,
        EvidenceKindV1::EnergyWorkLedger,
    ];
    const TASK: &[EvidenceKindV1] = &[
        EvidenceKindV1::MathematicalSymmetry,
        EvidenceKindV1::CoordinateCovariance,
        EvidenceKindV1::OperatorSpectrum,
        EvidenceKindV1::ConstraintDelassus,
        EvidenceKindV1::KktResidual,
        EvidenceKindV1::NonlinearSensitivity,
        EvidenceKindV1::DiscreteStep,
        EvidenceKindV1::EnergyWorkLedger,
        EvidenceKindV1::TaskSemantics,
    ];
    match claim {
        CompositionClaimV1::PhysicalStepEquivalence => PHYSICAL_STEP,
        CompositionClaimV1::TaskEquivalence => TASK,
    }
}

fn validate_receipt_structure(receipt: &EvidenceReceiptV1) -> Result<(), CompositionErrorV1> {
    if !valid_sha256(&receipt.artifact_sha256) {
        return Err(CompositionErrorV1::InvalidSha256("artifact_sha256"));
    }
    if !valid_identifier(&receipt.verifier_id) {
        return Err(CompositionErrorV1::InvalidIdentifier("verifier_id"));
    }
    if !valid_identifier(&receipt.verifier_version) {
        return Err(CompositionErrorV1::InvalidIdentifier("verifier_version"));
    }
    if !valid_git_sha(&receipt.verifier_commit_sha) {
        return Err(CompositionErrorV1::InvalidGitSha("verifier_commit_sha"));
    }
    if !valid_identifier(&receipt.claim_scope_id) {
        return Err(CompositionErrorV1::InvalidIdentifier("claim_scope_id"));
    }
    validate_problem_binding(&receipt.binding)?;
    match &receipt.frame_applicability {
        FrameApplicabilityV1::Source(frame) | FrameApplicabilityV1::Target(frame) => {
            validate_frame(frame)?;
        }
        FrameApplicabilityV1::RepresentationInvariant => {}
    }

    match (receipt.kind, &receipt.coordinate_relation) {
        (EvidenceKindV1::CoordinateCovariance, Some(relation)) => {
            for (name, value) in [
                ("source_frame_sha256", relation.source_frame_sha256.as_str()),
                (
                    "source_metric_sha256",
                    relation.source_metric_sha256.as_str(),
                ),
                ("target_frame_sha256", relation.target_frame_sha256.as_str()),
                (
                    "target_metric_sha256",
                    relation.target_metric_sha256.as_str(),
                ),
                (
                    "basis_transform_sha256",
                    relation.basis_transform_sha256.as_str(),
                ),
                (
                    "covariance_receipt_sha256",
                    relation.covariance_receipt_sha256.as_str(),
                ),
            ] {
                if !valid_sha256(value) {
                    return Err(CompositionErrorV1::InvalidSha256(name));
                }
            }
        }
        (EvidenceKindV1::CoordinateCovariance, None) => {
            return Err(CompositionErrorV1::MissingCoordinateRelation);
        }
        (_, Some(_)) => {
            return Err(CompositionErrorV1::UnexpectedCoordinateRelation(
                receipt.kind,
            ));
        }
        (_, None) => {}
    }

    if receipt.kind == EvidenceKindV1::NonlinearSensitivity {
        if receipt.valid_perturbation_radius_nanometers.is_none() {
            return Err(CompositionErrorV1::PerturbationRadiusInsufficient);
        }
    } else if receipt.valid_perturbation_radius_nanometers.is_some() {
        return Err(CompositionErrorV1::UnexpectedPerturbationRadius(
            receipt.kind,
        ));
    }
    if receipt.kind == EvidenceKindV1::TaskSemantics
        && receipt.binding.task_semantics_sha256.is_none()
    {
        return Err(CompositionErrorV1::MissingTaskSemanticsBinding);
    }
    if receipt.kind == EvidenceKindV1::MathematicalSymmetry
        && receipt.mathematical_evidence_class.is_none()
    {
        return Err(CompositionErrorV1::MathematicalEvidenceClassMissing);
    }
    if receipt.kind != EvidenceKindV1::MathematicalSymmetry
        && receipt.mathematical_evidence_class.is_some()
    {
        return Err(CompositionErrorV1::UnexpectedMathematicalEvidenceClass(
            receipt.kind,
        ));
    }
    Ok(())
}

/// Compose a required evidence set. A mismatched/failed receipt rejects the
/// bundle; missing evidence is INCONCLUSIVE; an inconclusive or N/A receipt
/// lowers the composition outcome. No upstream status is promoted in scope.
pub fn compose_evidence(
    request: &CompositionRequestV1,
    receipts: &[EvidenceReceiptV1],
) -> Result<EvidenceCompositionReceiptV1, CompositionErrorV1> {
    validate_problem_binding(&request.binding)?;
    validate_frame(&request.source_frame)?;
    validate_frame(&request.target_frame)?;
    if !valid_sha256(&request.basis_transform_sha256) {
        return Err(CompositionErrorV1::InvalidSha256("basis_transform_sha256"));
    }
    if !valid_identifier(&request.accepted_claim_scope_id) {
        return Err(CompositionErrorV1::InvalidIdentifier(
            "accepted_claim_scope_id",
        ));
    }
    if request.required_perturbation_radius_nanometers == 0 {
        return Err(CompositionErrorV1::PerturbationRadiusInsufficient);
    }

    if request.claim == CompositionClaimV1::TaskEquivalence
        && request.binding.task_semantics_sha256.is_none()
    {
        return Err(CompositionErrorV1::MissingTaskSemanticsBinding);
    }

    let required = required_kinds(request.claim);
    let mut expected_by_kind = BTreeMap::new();
    for expected in &request.expected_verifiers {
        if !valid_identifier(&expected.verifier_id) {
            return Err(CompositionErrorV1::InvalidIdentifier(
                "expected_verifier_id",
            ));
        }
        if !valid_identifier(&expected.verifier_version) {
            return Err(CompositionErrorV1::InvalidIdentifier(
                "expected_verifier_version",
            ));
        }
        if !valid_git_sha(&expected.verifier_commit_sha) {
            return Err(CompositionErrorV1::InvalidGitSha(
                "expected_verifier_commit_sha",
            ));
        }
        if expected_by_kind.insert(expected.kind, expected).is_some() {
            return Err(CompositionErrorV1::DuplicateExpectedVerifier(expected.kind));
        }
    }
    for kind in required {
        if !expected_by_kind.contains_key(kind) {
            return Err(CompositionErrorV1::MissingExpectedVerifier(*kind));
        }
    }
    for kind in expected_by_kind.keys() {
        if !required.contains(kind) {
            return Err(CompositionErrorV1::UnexpectedExpectedVerifier(*kind));
        }
    }

    let mut by_kind = BTreeMap::new();
    for receipt in receipts {
        validate_receipt_structure(receipt)?;
        if by_kind.insert(receipt.kind, receipt).is_some() {
            return Err(CompositionErrorV1::DuplicateEvidenceKind(receipt.kind));
        }
    }

    let mut uses = Vec::new();
    let mut inconclusive = false;
    for kind in required {
        let receipt = by_kind
            .get(kind)
            .copied()
            .ok_or(CompositionErrorV1::MissingEvidence(*kind))?;

        if receipt.binding != request.binding {
            return Err(CompositionErrorV1::BindingMismatch(*kind));
        }
        if receipt.claim_scope_id != request.accepted_claim_scope_id {
            return Err(CompositionErrorV1::ClaimScopeMismatch(*kind));
        }

        let expected_verifier = expected_by_kind
            .get(kind)
            .copied()
            .ok_or(CompositionErrorV1::MissingExpectedVerifier(*kind))?;
        if receipt.verifier_id != expected_verifier.verifier_id
            || receipt.verifier_version != expected_verifier.verifier_version
            || receipt.verifier_commit_sha != expected_verifier.verifier_commit_sha
        {
            return Err(CompositionErrorV1::VerifierIdentityMismatch(*kind));
        }

        let frame_is_valid = match kind {
            EvidenceKindV1::MathematicalSymmetry => {
                receipt.frame_applicability
                    == FrameApplicabilityV1::Source(request.source_frame.clone())
            }
            EvidenceKindV1::CoordinateCovariance => {
                receipt.frame_applicability
                    == FrameApplicabilityV1::Target(request.target_frame.clone())
            }
            EvidenceKindV1::TaskSemantics => {
                receipt.frame_applicability == FrameApplicabilityV1::RepresentationInvariant
            }
            _ => {
                receipt.frame_applicability
                    == FrameApplicabilityV1::Target(request.target_frame.clone())
            }
        };
        if !frame_is_valid {
            return Err(CompositionErrorV1::FrameMismatch(*kind));
        }

        if *kind == EvidenceKindV1::CoordinateCovariance {
            let relation = receipt
                .coordinate_relation
                .as_ref()
                .ok_or(CompositionErrorV1::MissingCoordinateRelation)?;
            if relation.source_frame_sha256 != request.source_frame.coordinate_frame_sha256
                || relation.source_metric_sha256 != request.source_frame.metric_sha256
                || relation.target_frame_sha256 != request.target_frame.coordinate_frame_sha256
                || relation.target_metric_sha256 != request.target_frame.metric_sha256
                || relation.basis_transform_sha256 != request.basis_transform_sha256
                || relation.covariance_receipt_sha256 != receipt.artifact_sha256
            {
                return Err(CompositionErrorV1::CoordinateRelationMismatch);
            }
        }

        if *kind == EvidenceKindV1::MathematicalSymmetry {
            let class = receipt
                .mathematical_evidence_class
                .ok_or(CompositionErrorV1::MathematicalEvidenceClassMissing)?;
            // Evidence classes are proposition-specific, not a total
            // strength ordering. Require an exact class match; a formal proof
            // of a different proposition must not be substituted silently.
            if class != request.minimum_mathematical_evidence_class {
                return Err(CompositionErrorV1::MathematicalEvidenceTooWeak);
            }
        }

        if *kind == EvidenceKindV1::NonlinearSensitivity {
            let valid_radius = receipt
                .valid_perturbation_radius_nanometers
                .ok_or(CompositionErrorV1::PerturbationRadiusInsufficient)?;
            if valid_radius < request.required_perturbation_radius_nanometers {
                return Err(CompositionErrorV1::PerturbationRadiusInsufficient);
            }
        }

        match receipt.verification_status {
            VerificationStatusV1::Pass => {}
            VerificationStatusV1::Fail => {
                return Err(CompositionErrorV1::ReceiptFailed(*kind));
            }
            VerificationStatusV1::Inconclusive | VerificationStatusV1::NotApplicable => {
                inconclusive = true;
            }
        }

        uses.push(EvidenceUseV1 {
            kind: *kind,
            artifact_sha256: receipt.artifact_sha256.clone(),
            verifier_id: receipt.verifier_id.clone(),
            verifier_version: receipt.verifier_version.clone(),
            verifier_commit_sha: receipt.verifier_commit_sha.clone(),
            frame_applicability: receipt.frame_applicability.clone(),
        });
    }

    let (physical_step_status, task_equivalence_status, physical_authority, task_authority) =
        if inconclusive {
            (
                CompositionStatusV1::Inconclusive,
                CompositionStatusV1::Inconclusive,
                ClaimAuthorityV1::None,
                ClaimAuthorityV1::None,
            )
        } else {
            match request.claim {
                CompositionClaimV1::PhysicalStepEquivalence => (
                    CompositionStatusV1::Pass,
                    CompositionStatusV1::Inconclusive,
                    ClaimAuthorityV1::BoundedPhysicalStepOnly,
                    ClaimAuthorityV1::None,
                ),
                CompositionClaimV1::TaskEquivalence => (
                    CompositionStatusV1::Pass,
                    CompositionStatusV1::Pass,
                    ClaimAuthorityV1::BoundedPhysicalStepOnly,
                    ClaimAuthorityV1::FullTaskEquivalence,
                ),
            }
        };

    Ok(EvidenceCompositionReceiptV1 {
        schema_version: 1,
        claim: request.claim,
        physical_step_status,
        task_equivalence_status,
        physical_authority,
        task_authority,
        scientific_search_pruning_authorized: false,
        binding: request.binding.clone(),
        source_frame: request.source_frame.clone(),
        target_frame: request.target_frame.clone(),
        evidence: uses,
        composition_scope_id: request.accepted_claim_scope_id.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(byte: char) -> String {
        std::iter::repeat_n(byte, 64).collect()
    }

    fn binding() -> ProblemBindingV1 {
        ProblemBindingV1 {
            problem_id: "dumbbell-step-v1".to_owned(),
            model_revision_sha256: digest('1'),
            fixture_sha256: digest('2'),
            initial_state_sha256: digest('3'),
            units_profile_id: "SI-v1".to_owned(),
            integration_method_id: "midpoint-constrained-v1".to_owned(),
            timestep_nanoseconds: 20_000_000,
            solver_profile_id: "midpoint-newton-profile-v1".to_owned(),
            constraint_model_sha256: digest('4'),
            observable_map_sha256: digest('5'),
            task_semantics_sha256: None,
        }
    }

    fn task_binding() -> ProblemBindingV1 {
        let mut value = binding();
        value.task_semantics_sha256 = Some(digest('6'));
        value
    }

    fn source_frame() -> FrameBindingV1 {
        FrameBindingV1 {
            coordinate_frame_id: "cartesian-q".to_owned(),
            coordinate_frame_sha256: digest('7'),
            metric_sha256: digest('8'),
        }
    }

    fn target_frame() -> FrameBindingV1 {
        FrameBindingV1 {
            coordinate_frame_id: "dense-y".to_owned(),
            coordinate_frame_sha256: digest('9'),
            metric_sha256: digest('a'),
        }
    }

    fn request(claim: CompositionClaimV1) -> CompositionRequestV1 {
        CompositionRequestV1 {
            claim,
            binding: binding(),
            source_frame: source_frame(),
            target_frame: target_frame(),
            basis_transform_sha256: digest('b'),
            minimum_mathematical_evidence_class: MathematicalEvidenceClassV1::NumericLieBasis,
            required_perturbation_radius_nanometers: 50_000,
            accepted_claim_scope_id: "one-step-dumbbell-v1".to_owned(),
            expected_verifiers: required_kinds(claim)
                .iter()
                .map(|kind| ExpectedVerifierIdentityV1 {
                    kind: *kind,
                    verifier_id: format!("checker-{:?}", kind).to_ascii_lowercase(),
                    verifier_version: "1.0.0".to_owned(),
                    verifier_commit_sha: "a".repeat(40),
                })
                .collect(),
        }
    }

    fn receipt(kind: EvidenceKindV1) -> EvidenceReceiptV1 {
        let coordinate_relation =
            (kind == EvidenceKindV1::CoordinateCovariance).then(|| CoordinateRelationV1 {
                source_frame_sha256: source_frame().coordinate_frame_sha256,
                source_metric_sha256: source_frame().metric_sha256,
                target_frame_sha256: target_frame().coordinate_frame_sha256,
                target_metric_sha256: target_frame().metric_sha256,
                basis_transform_sha256: digest('b'),
                covariance_receipt_sha256: digest('c'),
            });
        let artifact_sha256 = if kind == EvidenceKindV1::CoordinateCovariance {
            digest('c')
        } else {
            digest(match kind {
                EvidenceKindV1::MathematicalSymmetry => 'd',
                EvidenceKindV1::OperatorSpectrum => 'e',
                EvidenceKindV1::ConstraintDelassus => 'f',
                EvidenceKindV1::KktResidual => '1',
                EvidenceKindV1::NonlinearSensitivity => '2',
                EvidenceKindV1::DiscreteStep => '3',
                EvidenceKindV1::EnergyWorkLedger => '5',
                EvidenceKindV1::TaskSemantics => '4',
                EvidenceKindV1::CoordinateCovariance => unreachable!(),
            })
        };
        EvidenceReceiptV1 {
            kind,
            verification_status: VerificationStatusV1::Pass,
            artifact_sha256,
            verifier_id: format!("checker-{:?}", kind).to_ascii_lowercase(),
            verifier_version: "1.0.0".to_owned(),
            verifier_commit_sha: "a".repeat(40),
            binding: binding(),
            frame_applicability: match kind {
                EvidenceKindV1::MathematicalSymmetry => {
                    FrameApplicabilityV1::Source(source_frame())
                }
                EvidenceKindV1::TaskSemantics => FrameApplicabilityV1::RepresentationInvariant,
                _ => FrameApplicabilityV1::Target(target_frame()),
            },
            coordinate_relation,
            valid_perturbation_radius_nanometers: if kind == EvidenceKindV1::NonlinearSensitivity {
                Some(100_000)
            } else {
                None
            },
            mathematical_evidence_class: if kind == EvidenceKindV1::MathematicalSymmetry {
                Some(MathematicalEvidenceClassV1::NumericLieBasis)
            } else {
                None
            },
            claim_scope_id: "one-step-dumbbell-v1".to_owned(),
        }
    }

    fn physical_receipts() -> Vec<EvidenceReceiptV1> {
        [
            EvidenceKindV1::MathematicalSymmetry,
            EvidenceKindV1::CoordinateCovariance,
            EvidenceKindV1::OperatorSpectrum,
            EvidenceKindV1::ConstraintDelassus,
            EvidenceKindV1::KktResidual,
            EvidenceKindV1::NonlinearSensitivity,
            EvidenceKindV1::DiscreteStep,
            EvidenceKindV1::EnergyWorkLedger,
        ]
        .into_iter()
        .map(receipt)
        .collect()
    }

    #[test]
    fn aligned_physical_chain_passes_but_task_equivalence_remains_inconclusive() {
        let output = compose_evidence(
            &request(CompositionClaimV1::PhysicalStepEquivalence),
            &physical_receipts(),
        )
        .unwrap();
        assert_eq!(output.physical_step_status, CompositionStatusV1::Pass);
        assert_eq!(
            output.task_equivalence_status,
            CompositionStatusV1::Inconclusive
        );
        assert_eq!(
            output.physical_authority,
            ClaimAuthorityV1::BoundedPhysicalStepOnly
        );
        assert_eq!(output.task_authority, ClaimAuthorityV1::None);
        assert!(!output.scientific_search_pruning_authorized);
        assert_eq!(output.evidence.len(), 7);
    }

    #[test]
    fn physical_step_chain_does_not_require_task_semantics_digest() {
        let request = request(CompositionClaimV1::PhysicalStepEquivalence);
        assert!(request.binding.task_semantics_sha256.is_none());
        let output = compose_evidence(&request, &physical_receipts()).unwrap();
        assert_eq!(output.physical_step_status, CompositionStatusV1::Pass);
        assert_eq!(
            output.task_equivalence_status,
            CompositionStatusV1::Inconclusive
        );
    }

    #[test]
    fn full_task_chain_requires_separate_task_semantics_receipt() {
        let mut missing_task_request = request(CompositionClaimV1::TaskEquivalence);
        let missing_task_evidence = physical_receipts();
        assert_eq!(
            compose_evidence(&missing_task_request, &missing_task_evidence),
            Err(CompositionErrorV1::MissingTaskSemanticsBinding)
        );

        let mut task_request = request(CompositionClaimV1::TaskEquivalence);
        task_request.binding = task_binding();
        let mut evidence = physical_receipts();
        for item in &mut evidence {
            item.binding = task_binding();
        }
        let mut semantics = receipt(EvidenceKindV1::TaskSemantics);
        semantics.binding = task_binding();
        evidence.push(semantics);
        let output = compose_evidence(&task_request, &evidence).unwrap();
        assert_eq!(output.physical_step_status, CompositionStatusV1::Pass);
        assert_eq!(output.task_equivalence_status, CompositionStatusV1::Pass);
        assert_eq!(output.task_authority, ClaimAuthorityV1::FullTaskEquivalence);
        assert!(!output.scientific_search_pruning_authorized);
    }

    #[test]
    fn a_single_initial_state_mismatch_rejects_the_entire_composition() {
        let mut evidence = physical_receipts();
        evidence[3].binding.initial_state_sha256 = digest('f');
        assert_eq!(
            compose_evidence(
                &request(CompositionClaimV1::PhysicalStepEquivalence),
                &evidence
            ),
            Err(CompositionErrorV1::BindingMismatch(
                EvidenceKindV1::ConstraintDelassus
            ))
        );
    }

    #[test]
    fn timestep_and_solver_profile_mismatches_reject_composition() {
        let mut evidence = physical_receipts();
        evidence[4].binding.timestep_nanoseconds += 1;
        assert_eq!(
            compose_evidence(
                &request(CompositionClaimV1::PhysicalStepEquivalence),
                &evidence
            ),
            Err(CompositionErrorV1::BindingMismatch(
                EvidenceKindV1::KktResidual
            ))
        );

        let mut evidence = physical_receipts();
        evidence[5].binding.solver_profile_id = "different-solver-profile-v1".to_owned();
        assert_eq!(
            compose_evidence(
                &request(CompositionClaimV1::PhysicalStepEquivalence),
                &evidence
            ),
            Err(CompositionErrorV1::BindingMismatch(
                EvidenceKindV1::NonlinearSensitivity
            ))
        );
    }

    #[test]
    fn coordinate_frame_and_basis_witness_mismatches_reject_composition() {
        let mut evidence = physical_receipts();
        evidence[2].frame = source_frame();
        assert_eq!(
            compose_evidence(
                &request(CompositionClaimV1::PhysicalStepEquivalence),
                &evidence
            ),
            Err(CompositionErrorV1::FrameMismatch(
                EvidenceKindV1::OperatorSpectrum
            ))
        );

        let mut evidence = physical_receipts();
        evidence[1]
            .coordinate_relation
            .as_mut()
            .unwrap()
            .basis_transform_sha256 = digest('c');
        assert_eq!(
            compose_evidence(
                &request(CompositionClaimV1::PhysicalStepEquivalence),
                &evidence
            ),
            Err(CompositionErrorV1::CoordinateRelationMismatch)
        );
    }

    #[test]
    fn nonlinear_curvature_radius_must_cover_the_requested_ball() {
        let mut evidence = physical_receipts();
        evidence[5].valid_perturbation_radius_nanometers = Some(49_999);
        assert_eq!(
            compose_evidence(
                &request(CompositionClaimV1::PhysicalStepEquivalence),
                &evidence
            ),
            Err(CompositionErrorV1::PerturbationRadiusInsufficient)
        );
    }

    #[test]
    fn a_failed_or_duplicate_receipt_never_gets_aggregated_to_pass() {
        let mut evidence = physical_receipts();
        evidence[6].verification_status = VerificationStatusV1::Fail;
        assert_eq!(
            compose_evidence(
                &request(CompositionClaimV1::PhysicalStepEquivalence),
                &evidence
            ),
            Err(CompositionErrorV1::ReceiptFailed(
                EvidenceKindV1::DiscreteStep
            ))
        );

        let mut evidence = physical_receipts();
        evidence.push(receipt(EvidenceKindV1::DiscreteStep));
        assert_eq!(
            compose_evidence(
                &request(CompositionClaimV1::PhysicalStepEquivalence),
                &evidence
            ),
            Err(CompositionErrorV1::DuplicateEvidenceKind(
                EvidenceKindV1::DiscreteStep
            ))
        );
    }

    #[test]
    fn inconclusive_or_not_applicable_receipts_degrade_the_composed_claim() {
        let mut evidence = physical_receipts();
        evidence[4].verification_status = VerificationStatusV1::Inconclusive;
        let output = compose_evidence(
            &request(CompositionClaimV1::PhysicalStepEquivalence),
            &evidence,
        )
        .unwrap();
        assert_eq!(
            output.physical_step_status,
            CompositionStatusV1::Inconclusive
        );
        assert_eq!(output.physical_authority, ClaimAuthorityV1::None);

        let mut evidence = physical_receipts();
        evidence[2].verification_status = VerificationStatusV1::NotApplicable;
        let output = compose_evidence(
            &request(CompositionClaimV1::PhysicalStepEquivalence),
            &evidence,
        )
        .unwrap();
        assert_eq!(
            output.physical_step_status,
            CompositionStatusV1::Inconclusive
        );
    }

    #[test]
    fn numeric_lie_receipt_cannot_satisfy_an_exact_group_requirement() {
        let mut request = request(CompositionClaimV1::PhysicalStepEquivalence);
        request.minimum_mathematical_evidence_class = MathematicalEvidenceClassV1::ExactFiniteGroup;
        assert_eq!(
            compose_evidence(&request, &physical_receipts()),
            Err(CompositionErrorV1::MathematicalEvidenceTooWeak)
        );
    }

    #[test]
    fn different_exact_mathematical_evidence_kind_is_not_interchangeable() {
        let mut evidence = physical_receipts();
        evidence[0].mathematical_evidence_class =
            Some(MathematicalEvidenceClassV1::ExactFiniteGroup);
        assert_eq!(
            compose_evidence(
                &request(CompositionClaimV1::PhysicalStepEquivalence),
                &evidence
            ),
            Err(CompositionErrorV1::MathematicalEvidenceTooWeak)
        );
    }

    #[test]
    fn source_metric_digest_is_part_of_the_covariance_witness() {
        let mut evidence = physical_receipts();
        evidence[1]
            .coordinate_relation
            .as_mut()
            .unwrap()
            .source_metric_sha256 = digest('c');
        assert_eq!(
            compose_evidence(
                &request(CompositionClaimV1::PhysicalStepEquivalence),
                &evidence
            ),
            Err(CompositionErrorV1::CoordinateRelationMismatch)
        );
    }

    #[test]
    fn malformed_verifier_identity_is_rejected() {
        let mut evidence = physical_receipts();
        evidence[0].verifier_commit_sha = "not-a-git-sha".to_owned();
        assert_eq!(
            compose_evidence(
                &request(CompositionClaimV1::PhysicalStepEquivalence),
                &evidence
            ),
            Err(CompositionErrorV1::InvalidGitSha("verifier_commit_sha"))
        );
    }

    #[test]
    fn well_formed_but_stale_verifier_identity_is_rejected() {
        let mut evidence = physical_receipts();
        evidence[0].verifier_commit_sha = "b".repeat(40);
        assert_eq!(
            compose_evidence(
                &request(CompositionClaimV1::PhysicalStepEquivalence),
                &evidence
            ),
            Err(CompositionErrorV1::VerifierIdentityMismatch(
                EvidenceKindV1::MathematicalSymmetry
            ))
        );
    }

    #[test]
    fn mathematical_receipt_is_bound_to_source_and_task_semantics_are_frame_invariant() {
        let mut evidence = physical_receipts();
        evidence[0].frame_applicability = FrameApplicabilityV1::Target(target_frame());
        assert_eq!(
            compose_evidence(
                &request(CompositionClaimV1::PhysicalStepEquivalence),
                &evidence
            ),
            Err(CompositionErrorV1::FrameMismatch(
                EvidenceKindV1::MathematicalSymmetry
            ))
        );
    }

    #[test]
    fn task_semantics_receipt_is_representation_independent() {
        let mut task_request = request(CompositionClaimV1::TaskEquivalence);
        task_request.binding = task_binding();
        let mut evidence = physical_receipts();
        for item in &mut evidence {
            item.binding = task_binding();
        }
        let mut semantics = receipt(EvidenceKindV1::TaskSemantics);
        semantics.binding = task_binding();
        semantics.frame_applicability = FrameApplicabilityV1::Target(target_frame());
        evidence.push(semantics);
        assert_eq!(
            compose_evidence(&task_request, &evidence),
            Err(CompositionErrorV1::FrameMismatch(
                EvidenceKindV1::TaskSemantics
            ))
        );
    }

    #[test]
    fn missing_energy_work_receipt_never_becomes_a_physical_pass() {
        let mut evidence = physical_receipts();
        evidence.retain(|receipt| receipt.kind != EvidenceKindV1::EnergyWorkLedger);
        assert_eq!(
            compose_evidence(
                &request(CompositionClaimV1::PhysicalStepEquivalence),
                &evidence
            ),
            Err(CompositionErrorV1::MissingEvidence(
                EvidenceKindV1::EnergyWorkLedger
            ))
        );
    }

    #[test]
    fn energy_work_receipt_must_bind_the_same_exact_physical_problem() {
        let mut evidence = physical_receipts();
        let energy = evidence
            .iter_mut()
            .find(|receipt| receipt.kind == EvidenceKindV1::EnergyWorkLedger)
            .unwrap();
        energy.binding.initial_state_sha256 = digest('f');
        assert_eq!(
            compose_evidence(
                &request(CompositionClaimV1::PhysicalStepEquivalence),
                &evidence
            ),
            Err(CompositionErrorV1::BindingMismatch(
                EvidenceKindV1::EnergyWorkLedger
            ))
        );
    }

    #[test]
    fn failed_energy_work_verification_rejects_the_composition() {
        let mut evidence = physical_receipts();
        let energy = evidence
            .iter_mut()
            .find(|receipt| receipt.kind == EvidenceKindV1::EnergyWorkLedger)
            .unwrap();
        energy.verification_status = VerificationStatusV1::Fail;
        assert_eq!(
            compose_evidence(
                &request(CompositionClaimV1::PhysicalStepEquivalence),
                &evidence
            ),
            Err(CompositionErrorV1::ReceiptFailed(
                EvidenceKindV1::EnergyWorkLedger
            ))
        );
    }

    #[test]
    fn energy_work_verifier_identity_is_pinned_separately() {
        let mut request = request(CompositionClaimV1::PhysicalStepEquivalence);
        let expected = request
            .expected_verifiers
            .iter_mut()
            .find(|expected| expected.kind == EvidenceKindV1::EnergyWorkLedger)
            .unwrap();
        expected.verifier_commit_sha = "b".repeat(40);
        assert_eq!(
            compose_evidence(&request, &physical_receipts()),
            Err(CompositionErrorV1::VerifierIdentityMismatch(
                EvidenceKindV1::EnergyWorkLedger
            ))
        );
    }

    #[test]
    fn missing_evidence_never_becomes_a_pass() {
        let mut evidence = physical_receipts();
        evidence.pop();
        assert_eq!(
            compose_evidence(
                &request(CompositionClaimV1::PhysicalStepEquivalence),
                &evidence
            ),
            Err(CompositionErrorV1::MissingEvidence(
                EvidenceKindV1::EnergyWorkLedger
            ))
        );
    }
}
