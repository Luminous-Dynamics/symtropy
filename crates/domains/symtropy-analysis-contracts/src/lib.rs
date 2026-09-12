// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Solver-independent analysis request and evidence contracts for Symtropy.
//!
//! Analysis records what an exact tool/model execution reported for an exact
//! design revision. It does not evaluate Fabrication constraints, declare a
//! design safe, or promote simulation output into physical evidence.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{error::Error, fmt};
use symtropy_design::{ContentDigest, DesignRevisionRef};
use symtropy_game_state::StableId;

mod persistence;

pub const ANALYSIS_REQUEST_SCHEMA_VERSION: u32 = 1;
pub const ANALYSIS_EVIDENCE_SCHEMA_VERSION: u32 = 1;
const REQUEST_DIGEST_DOMAIN: &[u8] = b"symtropy.analysis.request.v1\0";
const EVIDENCE_DIGEST_DOMAIN: &[u8] = b"symtropy.analysis.evidence.v1\0";
const SHA256_ALGORITHM_ID: &str = "sha256";

macro_rules! stable_id_type {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(StableId);

        impl $name {
            pub const fn new(id: StableId) -> Self {
                Self(id)
            }

            pub const fn stable_id(&self) -> &StableId {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(formatter)
            }
        }
    };
}

stable_id_type!(AnalysisRequestId);
stable_id_type!(AnalysisEvidenceId);
stable_id_type!(AnalysisArtifactId);
stable_id_type!(AnalysisProfileId);
stable_id_type!(AnalysisObservationId);

/// Exact immutable reference to semantics owned by an external authority.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct ExactSemanticRef {
    pub authority_id: StableId,
    pub subject_id: StableId,
    pub revision: u64,
    pub content_digest: ContentDigest,
}

impl ExactSemanticRef {
    pub fn new(
        authority_id: StableId,
        subject_id: StableId,
        revision: u64,
        content_digest: ContentDigest,
    ) -> Result<Self, AnalysisError> {
        let reference = Self {
            authority_id,
            subject_id,
            revision,
            content_digest,
        };
        reference.validate()?;
        Ok(reference)
    }

    pub fn validate(&self) -> Result<(), AnalysisError> {
        validate_stable_id(&self.authority_id)?;
        validate_stable_id(&self.subject_id)?;
        self.content_digest
            .validate()
            .map_err(AnalysisError::Design)
    }
}

/// Exact artifact used or produced by analysis. Storage location is not part
/// of artifact identity.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct AnalysisArtifactRef {
    pub id: AnalysisArtifactId,
    pub role_id: StableId,
    pub content_digest: ContentDigest,
}

impl AnalysisArtifactRef {
    pub fn new(
        id: AnalysisArtifactId,
        role_id: StableId,
        content_digest: ContentDigest,
    ) -> Result<Self, AnalysisError> {
        let reference = Self {
            id,
            role_id,
            content_digest,
        };
        reference.validate()?;
        Ok(reference)
    }

    pub fn validate(&self) -> Result<(), AnalysisError> {
        validate_stable_id(self.id.stable_id())?;
        validate_stable_id(&self.role_id)?;
        self.content_digest
            .validate()
            .map_err(AnalysisError::Design)
    }
}

/// One observable requested by an analysis consumer.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct ObservableRequest {
    pub observable_id: StableId,
    pub dimension_id: StableId,
    pub unit_id: Option<StableId>,
}

impl ObservableRequest {
    pub fn new(
        observable_id: StableId,
        dimension_id: StableId,
        unit_id: Option<StableId>,
    ) -> Result<Self, AnalysisError> {
        let request = Self {
            observable_id,
            dimension_id,
            unit_id,
        };
        request.validate()?;
        Ok(request)
    }

    fn validate(&self) -> Result<(), AnalysisError> {
        validate_stable_id(&self.observable_id)?;
        validate_stable_id(&self.dimension_id)?;
        if let Some(unit_id) = &self.unit_id {
            validate_stable_id(unit_id)?;
        }
        Ok(())
    }
}

/// Exact analysis profile. Profile contents define the discipline/model family,
/// numerical policy and other semantics; the request binds its exact digest.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct AnalysisProfileRef {
    pub id: AnalysisProfileId,
    pub revision: u64,
    pub content_digest: ContentDigest,
}

impl AnalysisProfileRef {
    pub fn new(
        id: AnalysisProfileId,
        revision: u64,
        content_digest: ContentDigest,
    ) -> Result<Self, AnalysisError> {
        let reference = Self {
            id,
            revision,
            content_digest,
        };
        reference.validate()?;
        Ok(reference)
    }

    pub fn validate(&self) -> Result<(), AnalysisError> {
        validate_stable_id(self.id.stable_id())?;
        self.content_digest
            .validate()
            .map_err(AnalysisError::Design)
    }
}

/// Immutable solver-independent request for analysis of one exact design.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AnalysisRequest {
    schema_version: u32,
    id: AnalysisRequestId,
    subject: DesignRevisionRef,
    profile: AnalysisProfileRef,
    input_artifacts: Vec<AnalysisArtifactRef>,
    requested_observables: Vec<ObservableRequest>,
    assumptions: Vec<ExactSemanticRef>,
    boundary_conditions: Vec<ExactSemanticRef>,
}

impl AnalysisRequest {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: AnalysisRequestId,
        subject: DesignRevisionRef,
        profile: AnalysisProfileRef,
        mut input_artifacts: Vec<AnalysisArtifactRef>,
        mut requested_observables: Vec<ObservableRequest>,
        mut assumptions: Vec<ExactSemanticRef>,
        mut boundary_conditions: Vec<ExactSemanticRef>,
    ) -> Result<Self, AnalysisError> {
        input_artifacts.sort_by(|left, right| left.id.cmp(&right.id));
        requested_observables.sort_by(|left, right| left.observable_id.cmp(&right.observable_id));
        assumptions.sort();
        boundary_conditions.sort();

        let request = Self {
            schema_version: ANALYSIS_REQUEST_SCHEMA_VERSION,
            id,
            subject,
            profile,
            input_artifacts,
            requested_observables,
            assumptions,
            boundary_conditions,
        };
        request.validate()?;
        Ok(request)
    }

    pub fn id(&self) -> &AnalysisRequestId {
        &self.id
    }

    pub fn subject(&self) -> &DesignRevisionRef {
        &self.subject
    }

    pub fn profile(&self) -> &AnalysisProfileRef {
        &self.profile
    }

    pub fn requested_observables(&self) -> &[ObservableRequest] {
        &self.requested_observables
    }

    pub fn exact_ref(&self) -> Result<AnalysisRequestRef, AnalysisError> {
        Ok(AnalysisRequestRef {
            id: self.id.clone(),
            content_digest: self.content_digest()?,
        })
    }

    pub fn content_digest(&self) -> Result<ContentDigest, AnalysisError> {
        self.validate()?;
        sha256_digest(&self.canonical_preimage()?)
    }

    pub fn validate(&self) -> Result<(), AnalysisError> {
        if self.schema_version != ANALYSIS_REQUEST_SCHEMA_VERSION {
            return Err(AnalysisError::UnsupportedRequestSchema(self.schema_version));
        }
        validate_stable_id(self.id.stable_id())?;
        self.subject.validate().map_err(AnalysisError::Design)?;
        self.profile.validate()?;
        validate_unique_artifacts(&self.input_artifacts)?;
        validate_unique_observables(&self.requested_observables)?;
        validate_unique_exact_refs("assumptions", &self.assumptions)?;
        validate_unique_exact_refs("boundary_conditions", &self.boundary_conditions)?;
        Ok(())
    }

    fn canonical_preimage(&self) -> Result<Vec<u8>, AnalysisError> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(REQUEST_DIGEST_DOMAIN);
        bytes.extend_from_slice(&self.schema_version.to_le_bytes());
        encode_stable_id(&mut bytes, self.id.stable_id());
        encode_design_ref(&mut bytes, &self.subject)?;
        encode_stable_id(&mut bytes, self.profile.id.stable_id());
        bytes.extend_from_slice(&self.profile.revision.to_le_bytes());
        encode_digest(&mut bytes, &self.profile.content_digest)?;
        encode_artifacts(&mut bytes, &self.input_artifacts)?;

        encode_len(&mut bytes, self.requested_observables.len())?;
        for observable in &self.requested_observables {
            encode_stable_id(&mut bytes, &observable.observable_id);
            encode_stable_id(&mut bytes, &observable.dimension_id);
            encode_optional_id(&mut bytes, observable.unit_id.as_ref());
        }
        encode_exact_refs(&mut bytes, &self.assumptions)?;
        encode_exact_refs(&mut bytes, &self.boundary_conditions)?;
        Ok(bytes)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct AnalysisRequestRef {
    pub id: AnalysisRequestId,
    pub content_digest: ContentDigest,
}

impl AnalysisRequestRef {
    pub fn validate(&self) -> Result<(), AnalysisError> {
        validate_stable_id(self.id.stable_id())?;
        self.content_digest
            .validate()
            .map_err(AnalysisError::Design)
    }
}

/// Exact identity of the program that produced an analysis execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SolverIdentity {
    pub provider_id: StableId,
    pub implementation_id: StableId,
    pub version: String,
    pub binary_digest: ContentDigest,
}

impl SolverIdentity {
    pub fn new(
        provider_id: StableId,
        implementation_id: StableId,
        version: impl Into<String>,
        binary_digest: ContentDigest,
    ) -> Result<Self, AnalysisError> {
        let identity = Self {
            provider_id,
            implementation_id,
            version: version.into(),
            binary_digest,
        };
        identity.validate()?;
        Ok(identity)
    }

    fn validate(&self) -> Result<(), AnalysisError> {
        validate_stable_id(&self.provider_id)?;
        validate_stable_id(&self.implementation_id)?;
        if self.version.is_empty()
            || self.version.len() > 128
            || self.version.chars().any(char::is_control)
        {
            return Err(AnalysisError::InvalidSolverVersion(self.version.clone()));
        }
        self.binary_digest.validate().map_err(AnalysisError::Design)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunDisposition {
    Completed,
    Failed,
    Cancelled,
}

/// Numerical state is deliberately separate from process completion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NumericalDisposition {
    /// The analysis profile has no iterative/convergence theorem.
    NotApplicable,
    Converged,
    NotConverged,
    Indeterminate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationClass {
    /// Value corresponding to a requested analysis observable.
    RequestedResult,
    /// Diagnostic value retained for troubleshooting/qualification only.
    Diagnostic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationState {
    Present,
    Absent,
    Unknown,
}

/// Analysis value. Integer intervals preserve uncertainty/resolution without
/// importing floating-point NaN/Infinity semantics into authority records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "value_kind", rename_all = "snake_case")]
pub enum AnalysisValue {
    Measurement {
        lower: i64,
        upper: i64,
        resolution: u64,
    },
    Category {
        value_id: StableId,
    },
    Predicate {
        state: ObservationState,
    },
}

impl AnalysisValue {
    fn validate(&self) -> Result<(), AnalysisError> {
        match self {
            Self::Measurement {
                lower,
                upper,
                resolution,
            } => {
                if lower > upper {
                    return Err(AnalysisError::InvalidMeasurementInterval {
                        lower: *lower,
                        upper: *upper,
                    });
                }
                if *resolution == 0 {
                    return Err(AnalysisError::ZeroMeasurementResolution);
                }
                Ok(())
            }
            Self::Category { value_id } => validate_stable_id(value_id),
            Self::Predicate { .. } => Ok(()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AnalysisObservation {
    pub id: AnalysisObservationId,
    pub class: ObservationClass,
    pub observable_id: StableId,
    pub dimension_id: StableId,
    pub unit_id: Option<StableId>,
    pub value: AnalysisValue,
}

impl AnalysisObservation {
    pub fn new(
        id: AnalysisObservationId,
        class: ObservationClass,
        observable_id: StableId,
        dimension_id: StableId,
        unit_id: Option<StableId>,
        value: AnalysisValue,
    ) -> Result<Self, AnalysisError> {
        let observation = Self {
            id,
            class,
            observable_id,
            dimension_id,
            unit_id,
            value,
        };
        observation.validate()?;
        Ok(observation)
    }

    fn validate(&self) -> Result<(), AnalysisError> {
        validate_stable_id(self.id.stable_id())?;
        validate_stable_id(&self.observable_id)?;
        validate_stable_id(&self.dimension_id)?;
        if let Some(unit_id) = &self.unit_id {
            validate_stable_id(unit_id)?;
        }
        self.value.validate()
    }
}

/// Immutable execution evidence for one exact request.
///
/// Raw/restored evidence deliberately has no affirmative result-qualification API.
/// Consumers must replay it against the exact request and retain the validated
/// wrapper before asking whether requested results may enter downstream qualification.
///
/// ```compile_fail
/// use symtropy_analysis_contracts::AnalysisEvidence;
/// fn raw_evidence_cannot_authorize(evidence: &AnalysisEvidence) {
///     let _ = evidence.may_enter_result_qualification();
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AnalysisEvidence {
    schema_version: u32,
    id: AnalysisEvidenceId,
    request: AnalysisRequestRef,
    subject: DesignRevisionRef,
    solver: SolverIdentity,
    environment: ExactSemanticRef,
    model_profile: ExactSemanticRef,
    applicability_profile: ExactSemanticRef,
    run_disposition: RunDisposition,
    numerical_disposition: NumericalDisposition,
    output_artifacts: Vec<AnalysisArtifactRef>,
    observations: Vec<AnalysisObservation>,
}

impl AnalysisEvidence {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: AnalysisEvidenceId,
        request: &AnalysisRequest,
        solver: SolverIdentity,
        environment: ExactSemanticRef,
        model_profile: ExactSemanticRef,
        applicability_profile: ExactSemanticRef,
        run_disposition: RunDisposition,
        numerical_disposition: NumericalDisposition,
        mut output_artifacts: Vec<AnalysisArtifactRef>,
        mut observations: Vec<AnalysisObservation>,
    ) -> Result<Self, AnalysisError> {
        output_artifacts.sort_by(|left, right| left.id.cmp(&right.id));
        observations.sort_by(|left, right| left.id.cmp(&right.id));
        let evidence = Self {
            schema_version: ANALYSIS_EVIDENCE_SCHEMA_VERSION,
            id,
            request: request.exact_ref()?,
            subject: request.subject().clone(),
            solver,
            environment,
            model_profile,
            applicability_profile,
            run_disposition,
            numerical_disposition,
            output_artifacts,
            observations,
        };
        evidence.validate_against(request)?;
        Ok(evidence)
    }

    pub fn id(&self) -> &AnalysisEvidenceId {
        &self.id
    }

    pub fn request(&self) -> &AnalysisRequestRef {
        &self.request
    }

    pub fn subject(&self) -> &DesignRevisionRef {
        &self.subject
    }

    pub fn observations(&self) -> &[AnalysisObservation] {
        &self.observations
    }

    pub fn run_disposition(&self) -> RunDisposition {
        self.run_disposition
    }

    pub fn numerical_disposition(&self) -> NumericalDisposition {
        self.numerical_disposition
    }

    fn result_disposition_eligible(&self) -> bool {
        self.run_disposition == RunDisposition::Completed
            && matches!(
                self.numerical_disposition,
                NumericalDisposition::Converged | NumericalDisposition::NotApplicable
            )
    }

    /// Replay every request-dependent invariant and return the only A0 type
    /// allowed to expose affirmative downstream result-qualification eligibility.
    pub fn rebind<'a>(
        &'a self,
        request: &'a AnalysisRequest,
    ) -> Result<ValidatedAnalysisEvidence<'a>, AnalysisError> {
        self.validate_against(request)?;
        Ok(ValidatedAnalysisEvidence {
            evidence: self,
            request,
        })
    }

    pub fn exact_ref(
        &self,
        request: &AnalysisRequest,
    ) -> Result<AnalysisEvidenceRef, AnalysisError> {
        Ok(AnalysisEvidenceRef {
            id: self.id.clone(),
            content_digest: self.content_digest(request)?,
        })
    }

    pub fn content_digest(
        &self,
        request: &AnalysisRequest,
    ) -> Result<ContentDigest, AnalysisError> {
        self.validate_against(request)?;
        sha256_digest(&self.canonical_preimage()?)
    }

    fn validate_structure(&self) -> Result<(), AnalysisError> {
        if self.schema_version != ANALYSIS_EVIDENCE_SCHEMA_VERSION {
            return Err(AnalysisError::UnsupportedEvidenceSchema(
                self.schema_version,
            ));
        }
        validate_stable_id(self.id.stable_id())?;
        self.request.validate()?;
        self.subject.validate().map_err(AnalysisError::Design)?;
        self.solver.validate()?;
        self.environment.validate()?;
        self.model_profile.validate()?;
        self.applicability_profile.validate()?;
        validate_unique_artifacts(&self.output_artifacts)?;
        validate_unique_observation_ids(&self.observations)?;

        if self.run_disposition != RunDisposition::Completed
            && self.numerical_disposition == NumericalDisposition::Converged
        {
            return Err(AnalysisError::ConvergenceOnIncompleteRun);
        }

        let result_eligible = self.result_disposition_eligible();
        for observation in &self.observations {
            observation.validate()?;
            if observation.class == ObservationClass::RequestedResult && !result_eligible {
                return Err(AnalysisError::AffirmativeResultFromUnqualifiedRun {
                    observation_id: observation.id.clone(),
                    run_disposition: self.run_disposition,
                    numerical_disposition: self.numerical_disposition,
                });
            }
        }
        Ok(())
    }

    pub fn validate_against(&self, request: &AnalysisRequest) -> Result<(), AnalysisError> {
        self.validate_structure()?;
        request.validate()?;
        if self.request != request.exact_ref()? {
            return Err(AnalysisError::RequestIdentityMismatch);
        }
        if self.subject != *request.subject() {
            return Err(AnalysisError::DesignSubjectMismatch);
        }

        for observation in &self.observations {
            if observation.class == ObservationClass::RequestedResult {
                let Some(expected) = request
                    .requested_observables()
                    .iter()
                    .find(|candidate| candidate.observable_id == observation.observable_id)
                else {
                    return Err(AnalysisError::UnrequestedResultObservable(
                        observation.observable_id.clone(),
                    ));
                };
                if expected.dimension_id != observation.dimension_id
                    || expected.unit_id != observation.unit_id
                {
                    return Err(AnalysisError::ObservableContractMismatch {
                        observable_id: observation.observable_id.clone(),
                    });
                }
            }
        }
        Ok(())
    }

    fn canonical_preimage(&self) -> Result<Vec<u8>, AnalysisError> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(EVIDENCE_DIGEST_DOMAIN);
        bytes.extend_from_slice(&self.schema_version.to_le_bytes());
        encode_stable_id(&mut bytes, self.id.stable_id());
        encode_stable_id(&mut bytes, self.request.id.stable_id());
        encode_digest(&mut bytes, &self.request.content_digest)?;
        encode_design_ref(&mut bytes, &self.subject)?;
        encode_stable_id(&mut bytes, &self.solver.provider_id);
        encode_stable_id(&mut bytes, &self.solver.implementation_id);
        encode_string(&mut bytes, &self.solver.version)?;
        encode_digest(&mut bytes, &self.solver.binary_digest)?;
        encode_exact_ref(&mut bytes, &self.environment)?;
        encode_exact_ref(&mut bytes, &self.model_profile)?;
        encode_exact_ref(&mut bytes, &self.applicability_profile)?;
        bytes.push(run_tag(self.run_disposition));
        bytes.push(numerical_tag(self.numerical_disposition));
        encode_artifacts(&mut bytes, &self.output_artifacts)?;
        encode_len(&mut bytes, self.observations.len())?;
        for observation in &self.observations {
            encode_stable_id(&mut bytes, observation.id.stable_id());
            bytes.push(match observation.class {
                ObservationClass::RequestedResult => 0,
                ObservationClass::Diagnostic => 1,
            });
            encode_stable_id(&mut bytes, &observation.observable_id);
            encode_stable_id(&mut bytes, &observation.dimension_id);
            encode_optional_id(&mut bytes, observation.unit_id.as_ref());
            encode_value(&mut bytes, &observation.value);
        }
        Ok(bytes)
    }
}

/// Request-replayed A0 execution evidence.
///
/// This wrapper deliberately does not implement `Serialize` or `Deserialize`.
/// Persistence restores raw `AnalysisEvidence`; downstream affirmative eligibility
/// must be reacquired by calling `AnalysisEvidence::rebind` with the exact request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValidatedAnalysisEvidence<'a> {
    evidence: &'a AnalysisEvidence,
    request: &'a AnalysisRequest,
}

impl<'a> ValidatedAnalysisEvidence<'a> {
    pub const fn evidence(&self) -> &'a AnalysisEvidence {
        self.evidence
    }

    pub const fn request(&self) -> &'a AnalysisRequest {
        self.request
    }

    /// Structural eligibility only. Model qualification, applicability,
    /// replication, engineering truth, safety and commissioning remain external.
    pub fn may_enter_result_qualification(&self) -> bool {
        self.evidence.result_disposition_eligible()
    }

    pub fn exact_ref(&self) -> Result<AnalysisEvidenceRef, AnalysisError> {
        self.evidence.exact_ref(self.request)
    }

    pub fn content_digest(&self) -> Result<ContentDigest, AnalysisError> {
        self.evidence.content_digest(self.request)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct AnalysisEvidenceRef {
    pub id: AnalysisEvidenceId,
    pub content_digest: ContentDigest,
}

impl AnalysisEvidenceRef {
    pub fn validate(&self) -> Result<(), AnalysisError> {
        validate_stable_id(self.id.stable_id())?;
        self.content_digest
            .validate()
            .map_err(AnalysisError::Design)
    }
}

fn validate_unique_artifacts(artifacts: &[AnalysisArtifactRef]) -> Result<(), AnalysisError> {
    let mut previous: Option<&AnalysisArtifactId> = None;
    for artifact in artifacts {
        artifact.validate()?;
        if let Some(previous) = previous
            && previous >= &artifact.id
        {
            return if previous == &artifact.id {
                Err(AnalysisError::DuplicateArtifact(artifact.id.clone()))
            } else {
                Err(AnalysisError::NonCanonicalOrder("artifacts"))
            };
        }
        previous = Some(&artifact.id);
    }
    Ok(())
}

fn validate_unique_observables(observables: &[ObservableRequest]) -> Result<(), AnalysisError> {
    let mut previous: Option<&StableId> = None;
    for observable in observables {
        observable.validate()?;
        if let Some(previous) = previous
            && previous >= &observable.observable_id
        {
            return if previous == &observable.observable_id {
                Err(AnalysisError::DuplicateObservable(
                    observable.observable_id.clone(),
                ))
            } else {
                Err(AnalysisError::NonCanonicalOrder("requested_observables"))
            };
        }
        previous = Some(&observable.observable_id);
    }
    Ok(())
}

fn validate_unique_exact_refs(
    field: &'static str,
    refs: &[ExactSemanticRef],
) -> Result<(), AnalysisError> {
    let mut previous: Option<&ExactSemanticRef> = None;
    for reference in refs {
        reference.validate()?;
        if let Some(previous) = previous
            && previous >= reference
        {
            return if previous == reference {
                Err(AnalysisError::DuplicateExactSemanticRef(reference.clone()))
            } else {
                Err(AnalysisError::NonCanonicalOrder(field))
            };
        }
        previous = Some(reference);
    }
    Ok(())
}

fn validate_unique_observation_ids(
    observations: &[AnalysisObservation],
) -> Result<(), AnalysisError> {
    let mut previous: Option<&AnalysisObservationId> = None;
    for observation in observations {
        if let Some(previous) = previous
            && previous >= &observation.id
        {
            return if previous == &observation.id {
                Err(AnalysisError::DuplicateObservation(observation.id.clone()))
            } else {
                Err(AnalysisError::NonCanonicalOrder("observations"))
            };
        }
        previous = Some(&observation.id);
    }
    Ok(())
}

fn encode_design_ref(
    bytes: &mut Vec<u8>,
    reference: &DesignRevisionRef,
) -> Result<(), AnalysisError> {
    reference.validate().map_err(AnalysisError::Design)?;
    encode_stable_id(bytes, reference.design_id.stable_id());
    bytes.extend_from_slice(&reference.revision.to_le_bytes());
    encode_digest(bytes, &reference.content_digest)
}

fn encode_exact_ref(
    bytes: &mut Vec<u8>,
    reference: &ExactSemanticRef,
) -> Result<(), AnalysisError> {
    encode_stable_id(bytes, &reference.authority_id);
    encode_stable_id(bytes, &reference.subject_id);
    bytes.extend_from_slice(&reference.revision.to_le_bytes());
    encode_digest(bytes, &reference.content_digest)
}

fn encode_exact_refs(bytes: &mut Vec<u8>, refs: &[ExactSemanticRef]) -> Result<(), AnalysisError> {
    encode_len(bytes, refs.len())?;
    for reference in refs {
        encode_exact_ref(bytes, reference)?;
    }
    Ok(())
}

fn encode_artifacts(
    bytes: &mut Vec<u8>,
    artifacts: &[AnalysisArtifactRef],
) -> Result<(), AnalysisError> {
    encode_len(bytes, artifacts.len())?;
    for artifact in artifacts {
        encode_stable_id(bytes, artifact.id.stable_id());
        encode_stable_id(bytes, &artifact.role_id);
        encode_digest(bytes, &artifact.content_digest)?;
    }
    Ok(())
}

fn encode_digest(bytes: &mut Vec<u8>, digest: &ContentDigest) -> Result<(), AnalysisError> {
    digest.validate().map_err(AnalysisError::Design)?;
    encode_stable_id(bytes, &digest.algorithm);
    encode_string(bytes, &digest.value)
}

fn encode_value(bytes: &mut Vec<u8>, value: &AnalysisValue) {
    match value {
        AnalysisValue::Measurement {
            lower,
            upper,
            resolution,
        } => {
            bytes.push(0);
            bytes.extend_from_slice(&lower.to_le_bytes());
            bytes.extend_from_slice(&upper.to_le_bytes());
            bytes.extend_from_slice(&resolution.to_le_bytes());
        }
        AnalysisValue::Category { value_id } => {
            bytes.push(1);
            encode_stable_id(bytes, value_id);
        }
        AnalysisValue::Predicate { state } => {
            bytes.push(2);
            bytes.push(match state {
                ObservationState::Present => 0,
                ObservationState::Absent => 1,
                ObservationState::Unknown => 2,
            });
        }
    }
}

fn encode_optional_id(bytes: &mut Vec<u8>, value: Option<&StableId>) {
    match value {
        Some(value) => {
            bytes.push(1);
            encode_stable_id(bytes, value);
        }
        None => bytes.push(0),
    }
}

fn run_tag(value: RunDisposition) -> u8 {
    match value {
        RunDisposition::Completed => 0,
        RunDisposition::Failed => 1,
        RunDisposition::Cancelled => 2,
    }
}

fn numerical_tag(value: NumericalDisposition) -> u8 {
    match value {
        NumericalDisposition::NotApplicable => 0,
        NumericalDisposition::Converged => 1,
        NumericalDisposition::NotConverged => 2,
        NumericalDisposition::Indeterminate => 3,
    }
}

fn sha256_digest(bytes: &[u8]) -> Result<ContentDigest, AnalysisError> {
    ContentDigest::new(
        StableId::parse(SHA256_ALGORITHM_ID).expect("sha256 is a valid stable identifier literal"),
        hex(&Sha256::digest(bytes)),
    )
    .map_err(AnalysisError::Design)
}

fn encode_stable_id(bytes: &mut Vec<u8>, id: &StableId) {
    encode_string(bytes, id.as_str()).expect("StableId length is bounded well below u64");
}

fn encode_string(bytes: &mut Vec<u8>, value: &str) -> Result<(), AnalysisError> {
    encode_len(bytes, value.len())?;
    bytes.extend_from_slice(value.as_bytes());
    Ok(())
}

fn encode_len(bytes: &mut Vec<u8>, len: usize) -> Result<(), AnalysisError> {
    let len = u64::try_from(len).map_err(|_| AnalysisError::LengthOverflow)?;
    bytes.extend_from_slice(&len.to_le_bytes());
    Ok(())
}

fn validate_stable_id(id: &StableId) -> Result<(), AnalysisError> {
    StableId::parse(id.as_str())
        .map(|_| ())
        .map_err(|_| AnalysisError::InvalidStableId(id.as_str().to_string()))
}

fn hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

#[derive(Debug)]
pub enum AnalysisError {
    InvalidStableId(String),
    InvalidSolverVersion(String),
    UnsupportedRequestSchema(u32),
    UnsupportedEvidenceSchema(u32),
    DuplicateArtifact(AnalysisArtifactId),
    DuplicateObservable(StableId),
    DuplicateExactSemanticRef(ExactSemanticRef),
    DuplicateObservation(AnalysisObservationId),
    NonCanonicalOrder(&'static str),
    InvalidMeasurementInterval {
        lower: i64,
        upper: i64,
    },
    ZeroMeasurementResolution,
    RequestIdentityMismatch,
    DesignSubjectMismatch,
    ConvergenceOnIncompleteRun,
    AffirmativeResultFromUnqualifiedRun {
        observation_id: AnalysisObservationId,
        run_disposition: RunDisposition,
        numerical_disposition: NumericalDisposition,
    },
    UnrequestedResultObservable(StableId),
    ObservableContractMismatch {
        observable_id: StableId,
    },
    LengthOverflow,
    Design(symtropy_design::DesignError),
}

impl fmt::Display for AnalysisError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidStableId(id) => write!(formatter, "invalid stable identifier: {id}"),
            Self::InvalidSolverVersion(version) => {
                write!(formatter, "invalid solver version: {version:?}")
            }
            Self::UnsupportedRequestSchema(version) => {
                write!(formatter, "unsupported analysis request schema {version}")
            }
            Self::UnsupportedEvidenceSchema(version) => {
                write!(formatter, "unsupported analysis evidence schema {version}")
            }
            Self::DuplicateArtifact(id) => write!(formatter, "duplicate analysis artifact {id}"),
            Self::DuplicateObservable(id) => {
                write!(formatter, "duplicate requested observable {id}")
            }
            Self::DuplicateExactSemanticRef(reference) => write!(
                formatter,
                "duplicate exact semantic reference {reference:?}"
            ),
            Self::DuplicateObservation(id) => {
                write!(formatter, "duplicate analysis observation {id}")
            }
            Self::NonCanonicalOrder(field) => write!(
                formatter,
                "analysis field {field} is not canonically ordered"
            ),
            Self::InvalidMeasurementInterval { lower, upper } => write!(
                formatter,
                "analysis measurement interval is invalid: {lower}..{upper}"
            ),
            Self::ZeroMeasurementResolution => write!(
                formatter,
                "analysis measurement resolution must be non-zero"
            ),
            Self::RequestIdentityMismatch => write!(
                formatter,
                "analysis evidence does not bind the supplied exact request"
            ),
            Self::DesignSubjectMismatch => write!(
                formatter,
                "analysis evidence design subject differs from the request subject"
            ),
            Self::ConvergenceOnIncompleteRun => write!(
                formatter,
                "an incomplete analysis run cannot claim numerical convergence"
            ),
            Self::AffirmativeResultFromUnqualifiedRun {
                observation_id,
                run_disposition,
                numerical_disposition,
            } => write!(
                formatter,
                "requested-result observation {observation_id} cannot be emitted from {run_disposition:?}/{numerical_disposition:?} execution"
            ),
            Self::UnrequestedResultObservable(id) => write!(
                formatter,
                "analysis emitted unrequested result observable {id}"
            ),
            Self::ObservableContractMismatch { observable_id } => write!(
                formatter,
                "analysis result {observable_id} changed requested dimension/unit semantics"
            ),
            Self::LengthOverflow => write!(formatter, "canonical analysis length exceeds u64"),
            Self::Design(error) => error.fmt(formatter),
        }
    }
}

impl Error for AnalysisError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Design(error) => Some(error),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use symtropy_design::{DesignId, DesignRevisionRef};

    fn id(value: &str) -> StableId {
        StableId::parse(value).unwrap()
    }

    fn digest(value: &str) -> ContentDigest {
        ContentDigest::new(id("blake3"), value).unwrap()
    }

    fn subject(content: &str) -> DesignRevisionRef {
        DesignRevisionRef::new(DesignId::new(id("design:bracket")), 1, digest(content)).unwrap()
    }

    fn exact(authority: &str, subject: &str, content: &str) -> ExactSemanticRef {
        ExactSemanticRef::new(id(authority), id(subject), 1, digest(content)).unwrap()
    }

    fn profile() -> AnalysisProfileRef {
        AnalysisProfileRef::new(
            AnalysisProfileId::new(id("analysis-profile:structural-static-v1")),
            1,
            digest("profile-digest"),
        )
        .unwrap()
    }

    fn request_for(
        subject: DesignRevisionRef,
        observables: Vec<ObservableRequest>,
    ) -> AnalysisRequest {
        AnalysisRequest::new(
            AnalysisRequestId::new(id("analysis-request:bracket-static")),
            subject,
            profile(),
            Vec::new(),
            observables,
            vec![exact(
                "authority:materials",
                "assumption:6061-t6",
                "materials",
            )],
            vec![exact(
                "authority:analysis",
                "boundary:fixed-face",
                "boundary",
            )],
        )
        .unwrap()
    }

    fn deflection_request(subject: DesignRevisionRef) -> AnalysisRequest {
        request_for(
            subject,
            vec![
                ObservableRequest::new(
                    id("observable:deflection"),
                    id("dimension:length-um"),
                    Some(id("unit:um")),
                )
                .unwrap(),
            ],
        )
    }

    fn solver(binary: &str) -> SolverIdentity {
        SolverIdentity::new(
            id("provider:solver-lab"),
            id("solver:fem-reference"),
            "1.0.0",
            digest(binary),
        )
        .unwrap()
    }

    fn result_observation() -> AnalysisObservation {
        AnalysisObservation::new(
            AnalysisObservationId::new(id("analysis-observation:deflection")),
            ObservationClass::RequestedResult,
            id("observable:deflection"),
            id("dimension:length-um"),
            Some(id("unit:um")),
            AnalysisValue::Measurement {
                lower: 390,
                upper: 430,
                resolution: 10,
            },
        )
        .unwrap()
    }

    fn diagnostic_observation() -> AnalysisObservation {
        AnalysisObservation::new(
            AnalysisObservationId::new(id("analysis-observation:residual")),
            ObservationClass::Diagnostic,
            id("diagnostic:residual"),
            id("dimension:dimensionless"),
            None,
            AnalysisValue::Measurement {
                lower: 100,
                upper: 100,
                resolution: 1,
            },
        )
        .unwrap()
    }

    fn evidence(
        request: &AnalysisRequest,
        run: RunDisposition,
        numerical: NumericalDisposition,
        observations: Vec<AnalysisObservation>,
        binary: &str,
    ) -> Result<AnalysisEvidence, AnalysisError> {
        AnalysisEvidence::new(
            AnalysisEvidenceId::new(id("analysis-evidence:bracket-static")),
            request,
            solver(binary),
            exact("authority:nix", "environment:solver-capsule", "env"),
            exact("authority:model", "model:linear-elasticity", "model"),
            exact(
                "authority:qualification",
                "applicability:bracket-static",
                "applicability",
            ),
            run,
            numerical,
            Vec::new(),
            observations,
        )
    }

    #[test]
    fn request_insertion_order_is_canonical() {
        let a = ObservableRequest::new(id("observable:a"), id("dimension:a"), None).unwrap();
        let b = ObservableRequest::new(id("observable:b"), id("dimension:b"), None).unwrap();
        let left = request_for(subject("design"), vec![a.clone(), b.clone()]);
        let right = request_for(subject("design"), vec![b, a]);
        assert_eq!(left, right);
        assert_eq!(
            left.content_digest().unwrap(),
            right.content_digest().unwrap()
        );
    }

    #[test]
    fn canonical_analysis_request_digest_has_frozen_golden_vector() {
        let request = deflection_request(subject("design"));
        let digest = request.content_digest().unwrap();
        assert_eq!(digest.algorithm, id("sha256"));
        assert_eq!(
            digest.value,
            "3b0ce5c820d1dd550b164c7691ba9804ecc1f4a672d8615fa7f732e372829037"
        );
    }

    #[test]
    fn canonical_analysis_evidence_digest_has_frozen_golden_vector() {
        let request = deflection_request(subject("design"));
        let evidence = evidence(
            &request,
            RunDisposition::Completed,
            NumericalDisposition::Converged,
            vec![result_observation()],
            "solver-a",
        )
        .unwrap();
        let digest = evidence.content_digest(&request).unwrap();
        assert_eq!(digest.algorithm, id("sha256"));
        assert_eq!(
            digest.value,
            "501e1ec4aabe3ebcbc49b0db12ae9a0963415df32dfa040e05e45ae3cf514dc5"
        );
    }

    #[test]
    fn changing_exact_design_changes_request_identity() {
        let left = deflection_request(subject("design-a"));
        let right = deflection_request(subject("design-b"));
        assert_ne!(left.exact_ref().unwrap(), right.exact_ref().unwrap());
    }

    #[test]
    fn converged_completed_run_can_retain_requested_result() {
        let request = deflection_request(subject("design"));
        let evidence = evidence(
            &request,
            RunDisposition::Completed,
            NumericalDisposition::Converged,
            vec![result_observation()],
            "solver-a",
        )
        .unwrap();
        assert!(
            evidence
                .rebind(&request)
                .unwrap()
                .may_enter_result_qualification()
        );
        assert_eq!(evidence.observations().len(), 1);
    }

    #[test]
    fn non_converged_run_cannot_emit_requested_result() {
        let request = deflection_request(subject("design"));
        let result = evidence(
            &request,
            RunDisposition::Completed,
            NumericalDisposition::NotConverged,
            vec![result_observation()],
            "solver-a",
        );
        assert!(matches!(
            result,
            Err(AnalysisError::AffirmativeResultFromUnqualifiedRun { .. })
        ));
    }

    #[test]
    fn failed_run_can_retain_diagnostics_without_result_authority() {
        let request = deflection_request(subject("design"));
        let evidence = evidence(
            &request,
            RunDisposition::Failed,
            NumericalDisposition::NotConverged,
            vec![diagnostic_observation()],
            "solver-a",
        )
        .unwrap();
        assert!(
            !evidence
                .rebind(&request)
                .unwrap()
                .may_enter_result_qualification()
        );
        assert_eq!(evidence.observations().len(), 1);
    }

    #[test]
    fn failed_run_cannot_claim_convergence() {
        let request = deflection_request(subject("design"));
        let result = evidence(
            &request,
            RunDisposition::Failed,
            NumericalDisposition::Converged,
            Vec::new(),
            "solver-a",
        );
        assert!(matches!(
            result,
            Err(AnalysisError::ConvergenceOnIncompleteRun)
        ));
    }

    #[test]
    fn unrequested_result_observable_is_rejected() {
        let request = deflection_request(subject("design"));
        let observation = AnalysisObservation::new(
            AnalysisObservationId::new(id("analysis-observation:strain")),
            ObservationClass::RequestedResult,
            id("observable:strain"),
            id("dimension:strain"),
            None,
            AnalysisValue::Measurement {
                lower: 1,
                upper: 2,
                resolution: 1,
            },
        )
        .unwrap();
        let result = evidence(
            &request,
            RunDisposition::Completed,
            NumericalDisposition::Converged,
            vec![observation],
            "solver-a",
        );
        assert!(matches!(
            result,
            Err(AnalysisError::UnrequestedResultObservable(_))
        ));
    }

    #[test]
    fn result_cannot_change_requested_unit_semantics() {
        let request = deflection_request(subject("design"));
        let observation = AnalysisObservation::new(
            AnalysisObservationId::new(id("analysis-observation:deflection")),
            ObservationClass::RequestedResult,
            id("observable:deflection"),
            id("dimension:length-um"),
            Some(id("unit:mm")),
            AnalysisValue::Measurement {
                lower: 1,
                upper: 1,
                resolution: 1,
            },
        )
        .unwrap();
        let result = evidence(
            &request,
            RunDisposition::Completed,
            NumericalDisposition::Converged,
            vec![observation],
            "solver-a",
        );
        assert!(matches!(
            result,
            Err(AnalysisError::ObservableContractMismatch { .. })
        ));
    }

    #[test]
    fn exact_solver_binary_is_bound_into_evidence_identity() {
        let request = deflection_request(subject("design"));
        let left = evidence(
            &request,
            RunDisposition::Completed,
            NumericalDisposition::Converged,
            vec![result_observation()],
            "solver-a",
        )
        .unwrap();
        let right = evidence(
            &request,
            RunDisposition::Completed,
            NumericalDisposition::Converged,
            vec![result_observation()],
            "solver-b",
        )
        .unwrap();
        assert_ne!(
            left.exact_ref(&request).unwrap(),
            right.exact_ref(&request).unwrap()
        );
    }

    #[test]
    fn uncertainty_interval_is_not_collapsed_to_point_score() {
        let observation = result_observation();
        assert!(matches!(
            observation.value,
            AnalysisValue::Measurement {
                lower: 390,
                upper: 430,
                resolution: 10,
            }
        ));
    }
}
