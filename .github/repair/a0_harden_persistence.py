#!/usr/bin/env python3
from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected one exact anchor, found {count}")
    return text.replace(old, new, 1)


lib = Path("crates/domains/symtropy-analysis-contracts/src/lib.rs")
text = lib.read_text()

text = replace_once(
    text,
    "use symtropy_game_state::StableId;\n",
    "use symtropy_game_state::StableId;\n\nmod persistence;\n",
    "persistence module",
)

for name in [
    "ExactSemanticRef",
    "AnalysisArtifactRef",
    "ObservableRequest",
    "AnalysisProfileRef",
    "AnalysisRequestRef",
    "SolverIdentity",
    "AnalysisObservation",
    "AnalysisEvidenceRef",
]:
    old = f"#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]\npub struct {name}" if name in {
        "ExactSemanticRef", "AnalysisArtifactRef", "ObservableRequest", "AnalysisProfileRef", "AnalysisRequestRef", "AnalysisEvidenceRef"
    } else f"#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]\npub struct {name}"
    new = old.replace(", Deserialize", "")
    text = replace_once(text, old, new, f"remove raw Deserialize from {name}")

text = replace_once(
    text,
    "#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]\npub struct AnalysisRequest {",
    "#[derive(Debug, Clone, PartialEq, Eq, Serialize)]\npub struct AnalysisRequest {",
    "remove raw Deserialize from AnalysisRequest",
)
text = replace_once(
    text,
    "#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]\n#[serde(tag = \"value_kind\", rename_all = \"snake_case\")]\npub enum AnalysisValue {",
    "#[derive(Debug, Clone, PartialEq, Eq, Serialize)]\n#[serde(tag = \"value_kind\", rename_all = \"snake_case\")]\npub enum AnalysisValue {",
    "remove raw Deserialize from AnalysisValue",
)
text = replace_once(
    text,
    "#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]\npub struct AnalysisEvidence {",
    "#[derive(Debug, Clone, PartialEq, Eq, Serialize)]\npub struct AnalysisEvidence {",
    "remove raw Deserialize from AnalysisEvidence",
)

old_eligibility = '''    /// Structural eligibility only: downstream model/applicability/evidence\n    /// qualification is still required before any engineering fact exists.\n    pub fn may_enter_result_qualification(&self) -> bool {\n        self.run_disposition == RunDisposition::Completed\n            && matches!(\n                self.numerical_disposition,\n                NumericalDisposition::Converged | NumericalDisposition::NotApplicable\n            )\n    }\n'''
new_eligibility = '''    fn result_disposition_eligible(&self) -> bool {\n        self.run_disposition == RunDisposition::Completed\n            && matches!(\n                self.numerical_disposition,\n                NumericalDisposition::Converged | NumericalDisposition::NotApplicable\n            )\n    }\n\n    /// Replay every request-dependent invariant and return the only A0 type\n    /// allowed to expose affirmative downstream result-qualification eligibility.\n    pub fn rebind<'a>(\n        &'a self,\n        request: &'a AnalysisRequest,\n    ) -> Result<ValidatedAnalysisEvidence<'a>, AnalysisError> {\n        self.validate_against(request)?;\n        Ok(ValidatedAnalysisEvidence {\n            evidence: self,\n            request,\n        })\n    }\n'''
text = replace_once(text, old_eligibility, new_eligibility, "eligibility type-state boundary")

old_validate = '''    pub fn validate_against(&self, request: &AnalysisRequest) -> Result<(), AnalysisError> {\n        if self.schema_version != ANALYSIS_EVIDENCE_SCHEMA_VERSION {\n            return Err(AnalysisError::UnsupportedEvidenceSchema(\n                self.schema_version,\n            ));\n        }\n        validate_stable_id(self.id.stable_id())?;\n        request.validate()?;\n        self.request.validate()?;\n        if self.request != request.exact_ref()? {\n            return Err(AnalysisError::RequestIdentityMismatch);\n        }\n        if self.subject != *request.subject() {\n            return Err(AnalysisError::DesignSubjectMismatch);\n        }\n        self.subject.validate().map_err(AnalysisError::Design)?;\n        self.solver.validate()?;\n        self.environment.validate()?;\n        self.model_profile.validate()?;\n        self.applicability_profile.validate()?;\n        validate_unique_artifacts(&self.output_artifacts)?;\n        validate_unique_observation_ids(&self.observations)?;\n\n        if self.run_disposition != RunDisposition::Completed\n            && self.numerical_disposition == NumericalDisposition::Converged\n        {\n            return Err(AnalysisError::ConvergenceOnIncompleteRun);\n        }\n\n        let result_eligible = self.may_enter_result_qualification();\n        for observation in &self.observations {\n            observation.validate()?;\n            if observation.class == ObservationClass::RequestedResult {\n                if !result_eligible {\n                    return Err(AnalysisError::AffirmativeResultFromUnqualifiedRun {\n                        observation_id: observation.id.clone(),\n                        run_disposition: self.run_disposition,\n                        numerical_disposition: self.numerical_disposition,\n                    });\n                }\n                let Some(expected) = request\n                    .requested_observables()\n                    .iter()\n                    .find(|candidate| candidate.observable_id == observation.observable_id)\n                else {\n                    return Err(AnalysisError::UnrequestedResultObservable(\n                        observation.observable_id.clone(),\n                    ));\n                };\n                if expected.dimension_id != observation.dimension_id\n                    || expected.unit_id != observation.unit_id\n                {\n                    return Err(AnalysisError::ObservableContractMismatch {\n                        observable_id: observation.observable_id.clone(),\n                    });\n                }\n            }\n        }\n        Ok(())\n    }\n'''
new_validate = '''    fn validate_structure(&self) -> Result<(), AnalysisError> {\n        if self.schema_version != ANALYSIS_EVIDENCE_SCHEMA_VERSION {\n            return Err(AnalysisError::UnsupportedEvidenceSchema(\n                self.schema_version,\n            ));\n        }\n        validate_stable_id(self.id.stable_id())?;\n        self.request.validate()?;\n        self.subject.validate().map_err(AnalysisError::Design)?;\n        self.solver.validate()?;\n        self.environment.validate()?;\n        self.model_profile.validate()?;\n        self.applicability_profile.validate()?;\n        validate_unique_artifacts(&self.output_artifacts)?;\n        validate_unique_observation_ids(&self.observations)?;\n\n        if self.run_disposition != RunDisposition::Completed\n            && self.numerical_disposition == NumericalDisposition::Converged\n        {\n            return Err(AnalysisError::ConvergenceOnIncompleteRun);\n        }\n\n        let result_eligible = self.result_disposition_eligible();\n        for observation in &self.observations {\n            observation.validate()?;\n            if observation.class == ObservationClass::RequestedResult && !result_eligible {\n                return Err(AnalysisError::AffirmativeResultFromUnqualifiedRun {\n                    observation_id: observation.id.clone(),\n                    run_disposition: self.run_disposition,\n                    numerical_disposition: self.numerical_disposition,\n                });\n            }\n        }\n        Ok(())\n    }\n\n    pub fn validate_against(&self, request: &AnalysisRequest) -> Result<(), AnalysisError> {\n        self.validate_structure()?;\n        request.validate()?;\n        if self.request != request.exact_ref()? {\n            return Err(AnalysisError::RequestIdentityMismatch);\n        }\n        if self.subject != *request.subject() {\n            return Err(AnalysisError::DesignSubjectMismatch);\n        }\n\n        for observation in &self.observations {\n            if observation.class == ObservationClass::RequestedResult {\n                let Some(expected) = request\n                    .requested_observables()\n                    .iter()\n                    .find(|candidate| candidate.observable_id == observation.observable_id)\n                else {\n                    return Err(AnalysisError::UnrequestedResultObservable(\n                        observation.observable_id.clone(),\n                    ));\n                };\n                if expected.dimension_id != observation.dimension_id\n                    || expected.unit_id != observation.unit_id\n                {\n                    return Err(AnalysisError::ObservableContractMismatch {\n                        observable_id: observation.observable_id.clone(),\n                    });\n                }\n            }\n        }\n        Ok(())\n    }\n'''
text = replace_once(text, old_validate, new_validate, "split structural and request replay validation")

wrapper_anchor = '''}\n\n#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]\npub struct AnalysisEvidenceRef {\n'''
wrapper = '''}\n\n/// Request-replayed A0 execution evidence.\n///\n/// This wrapper deliberately does not implement `Serialize` or `Deserialize`.\n/// Persistence restores raw `AnalysisEvidence`; downstream affirmative eligibility\n/// must be reacquired by calling `AnalysisEvidence::rebind` with the exact request.\n#[derive(Debug, Clone, Copy, PartialEq, Eq)]\npub struct ValidatedAnalysisEvidence<'a> {\n    evidence: &'a AnalysisEvidence,\n    request: &'a AnalysisRequest,\n}\n\nimpl<'a> ValidatedAnalysisEvidence<'a> {\n    pub const fn evidence(&self) -> &'a AnalysisEvidence {\n        self.evidence\n    }\n\n    pub const fn request(&self) -> &'a AnalysisRequest {\n        self.request\n    }\n\n    /// Structural eligibility only. Model qualification, applicability,\n    /// replication, engineering truth, safety and commissioning remain external.\n    pub fn may_enter_result_qualification(&self) -> bool {\n        self.evidence.result_disposition_eligible()\n    }\n\n    pub fn exact_ref(&self) -> Result<AnalysisEvidenceRef, AnalysisError> {\n        self.evidence.exact_ref(self.request)\n    }\n\n    pub fn content_digest(&self) -> Result<ContentDigest, AnalysisError> {\n        self.evidence.content_digest(self.request)\n    }\n}\n\n#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]\npub struct AnalysisEvidenceRef {\n'''
text = replace_once(text, wrapper_anchor, wrapper, "validated evidence wrapper")

ref_anchor = '''pub struct AnalysisEvidenceRef {\n    pub id: AnalysisEvidenceId,\n    pub content_digest: ContentDigest,\n}\n\nfn validate_unique_artifacts'''
ref_impl = '''pub struct AnalysisEvidenceRef {\n    pub id: AnalysisEvidenceId,\n    pub content_digest: ContentDigest,\n}\n\nimpl AnalysisEvidenceRef {\n    pub fn validate(&self) -> Result<(), AnalysisError> {\n        validate_stable_id(self.id.stable_id())?;\n        self.content_digest\n            .validate()\n            .map_err(AnalysisError::Design)\n    }\n}\n\nfn validate_unique_artifacts'''
text = replace_once(text, ref_anchor, ref_impl, "AnalysisEvidenceRef validation")

text = replace_once(
    text,
    "        assert!(evidence.may_enter_result_qualification());",
    "        assert!(evidence\n            .rebind(&request)\n            .unwrap()\n            .may_enter_result_qualification());",
    "validated positive eligibility test",
)
text = replace_once(
    text,
    "        assert!(!evidence.may_enter_result_qualification());",
    "        assert!(!evidence\n            .rebind(&request)\n            .unwrap()\n            .may_enter_result_qualification());",
    "validated negative eligibility test",
)

lib.write_text(text)

manifest = Path("crates/domains/symtropy-analysis-contracts/Cargo.toml")
manifest_text = manifest.read_text()
if "[dev-dependencies]" not in manifest_text:
    manifest_text = manifest_text.rstrip() + '\n\n[dev-dependencies]\nserde_json = "1.0"\n'
else:
    raise SystemExit("unexpected existing dev-dependencies section")
manifest.write_text(manifest_text)

persistence = Path("crates/domains/symtropy-analysis-contracts/src/persistence.rs")
persistence.write_text(r'''use super::*;
use serde::{de::Error as _, Deserialize, Deserializer};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExactSemanticRefWire {
    authority_id: StableId,
    subject_id: StableId,
    revision: u64,
    content_digest: ContentDigest,
}

impl<'de> Deserialize<'de> for ExactSemanticRef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = ExactSemanticRefWire::deserialize(deserializer)?;
        Self::new(
            wire.authority_id,
            wire.subject_id,
            wire.revision,
            wire.content_digest,
        )
        .map_err(D::Error::custom)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AnalysisArtifactRefWire {
    id: AnalysisArtifactId,
    role_id: StableId,
    content_digest: ContentDigest,
}

impl<'de> Deserialize<'de> for AnalysisArtifactRef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = AnalysisArtifactRefWire::deserialize(deserializer)?;
        Self::new(wire.id, wire.role_id, wire.content_digest).map_err(D::Error::custom)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ObservableRequestWire {
    observable_id: StableId,
    dimension_id: StableId,
    unit_id: Option<StableId>,
}

impl<'de> Deserialize<'de> for ObservableRequest {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = ObservableRequestWire::deserialize(deserializer)?;
        Self::new(wire.observable_id, wire.dimension_id, wire.unit_id).map_err(D::Error::custom)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AnalysisProfileRefWire {
    id: AnalysisProfileId,
    revision: u64,
    content_digest: ContentDigest,
}

impl<'de> Deserialize<'de> for AnalysisProfileRef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = AnalysisProfileRefWire::deserialize(deserializer)?;
        Self::new(wire.id, wire.revision, wire.content_digest).map_err(D::Error::custom)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AnalysisRequestRefWire {
    id: AnalysisRequestId,
    content_digest: ContentDigest,
}

impl<'de> Deserialize<'de> for AnalysisRequestRef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = AnalysisRequestRefWire::deserialize(deserializer)?;
        let value = Self {
            id: wire.id,
            content_digest: wire.content_digest,
        };
        value.validate().map_err(D::Error::custom)?;
        Ok(value)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AnalysisRequestWire {
    schema_version: u32,
    id: AnalysisRequestId,
    subject: DesignRevisionRef,
    profile: AnalysisProfileRef,
    input_artifacts: Vec<AnalysisArtifactRef>,
    requested_observables: Vec<ObservableRequest>,
    assumptions: Vec<ExactSemanticRef>,
    boundary_conditions: Vec<ExactSemanticRef>,
}

impl<'de> Deserialize<'de> for AnalysisRequest {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = AnalysisRequestWire::deserialize(deserializer)?;
        if wire.schema_version != ANALYSIS_REQUEST_SCHEMA_VERSION {
            return Err(D::Error::custom(AnalysisError::UnsupportedRequestSchema(
                wire.schema_version,
            )));
        }
        Self::new(
            wire.id,
            wire.subject,
            wire.profile,
            wire.input_artifacts,
            wire.requested_observables,
            wire.assumptions,
            wire.boundary_conditions,
        )
        .map_err(D::Error::custom)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SolverIdentityWire {
    provider_id: StableId,
    implementation_id: StableId,
    version: String,
    binary_digest: ContentDigest,
}

impl<'de> Deserialize<'de> for SolverIdentity {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = SolverIdentityWire::deserialize(deserializer)?;
        Self::new(
            wire.provider_id,
            wire.implementation_id,
            wire.version,
            wire.binary_digest,
        )
        .map_err(D::Error::custom)
    }
}

#[derive(Deserialize)]
#[serde(tag = "value_kind", rename_all = "snake_case")]
enum AnalysisValueWire {
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

impl<'de> Deserialize<'de> for AnalysisValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = AnalysisValueWire::deserialize(deserializer)?;
        let value = match wire {
            AnalysisValueWire::Measurement {
                lower,
                upper,
                resolution,
            } => Self::Measurement {
                lower,
                upper,
                resolution,
            },
            AnalysisValueWire::Category { value_id } => Self::Category { value_id },
            AnalysisValueWire::Predicate { state } => Self::Predicate { state },
        };
        value.validate().map_err(D::Error::custom)?;
        Ok(value)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AnalysisObservationWire {
    id: AnalysisObservationId,
    class: ObservationClass,
    observable_id: StableId,
    dimension_id: StableId,
    unit_id: Option<StableId>,
    value: AnalysisValue,
}

impl<'de> Deserialize<'de> for AnalysisObservation {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = AnalysisObservationWire::deserialize(deserializer)?;
        Self::new(
            wire.id,
            wire.class,
            wire.observable_id,
            wire.dimension_id,
            wire.unit_id,
            wire.value,
        )
        .map_err(D::Error::custom)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AnalysisEvidenceRefWire {
    id: AnalysisEvidenceId,
    content_digest: ContentDigest,
}

impl<'de> Deserialize<'de> for AnalysisEvidenceRef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = AnalysisEvidenceRefWire::deserialize(deserializer)?;
        let value = Self {
            id: wire.id,
            content_digest: wire.content_digest,
        };
        value.validate().map_err(D::Error::custom)?;
        Ok(value)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AnalysisEvidenceWire {
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

impl<'de> Deserialize<'de> for AnalysisEvidence {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = AnalysisEvidenceWire::deserialize(deserializer)?;
        let evidence = Self {
            schema_version: wire.schema_version,
            id: wire.id,
            request: wire.request,
            subject: wire.subject,
            solver: wire.solver,
            environment: wire.environment,
            model_profile: wire.model_profile,
            applicability_profile: wire.applicability_profile,
            run_disposition: wire.run_disposition,
            numerical_disposition: wire.numerical_disposition,
            output_artifacts: wire.output_artifacts,
            observations: wire.observations,
        };
        evidence.validate_structure().map_err(D::Error::custom)?;
        Ok(evidence)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};
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

    fn observable(name: &str, dimension: &str, unit: Option<&str>) -> ObservableRequest {
        ObservableRequest::new(
            id(name),
            id(dimension),
            unit.map(id),
        )
        .unwrap()
    }

    fn request_with(observables: Vec<ObservableRequest>) -> AnalysisRequest {
        AnalysisRequest::new(
            AnalysisRequestId::new(id("analysis-request:bracket-static")),
            subject("design"),
            profile(),
            Vec::new(),
            observables,
            vec![exact("authority:materials", "assumption:6061-t6", "materials")],
            vec![exact("authority:analysis", "boundary:fixed-face", "boundary")],
        )
        .unwrap()
    }

    fn request() -> AnalysisRequest {
        request_with(vec![observable(
            "observable:deflection",
            "dimension:length-um",
            Some("unit:um"),
        )])
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
    ) -> AnalysisEvidence {
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
        .unwrap()
    }

    fn result_evidence(request: &AnalysisRequest) -> AnalysisEvidence {
        evidence(
            request,
            RunDisposition::Completed,
            NumericalDisposition::Converged,
            vec![result_observation()],
            "solver-a",
        )
    }

    fn evidence_json(request: &AnalysisRequest) -> Value {
        serde_json::to_value(result_evidence(request)).unwrap()
    }

    #[test]
    fn valid_request_round_trip_replays_constructor_and_digest() {
        let request = request();
        let restored: AnalysisRequest =
            serde_json::from_str(&serde_json::to_string(&request).unwrap()).unwrap();
        assert_eq!(restored, request);
        assert_eq!(
            restored.content_digest().unwrap().value,
            "3b0ce5c820d1dd550b164c7691ba9804ecc1f4a672d8615fa7f732e372829037"
        );
    }

    #[test]
    fn reordered_reusable_request_collections_restore_canonically() {
        let request = request_with(vec![
            observable("observable:a", "dimension:a", None),
            observable("observable:b", "dimension:b", None),
        ]);
        let mut wire = serde_json::to_value(&request).unwrap();
        wire["requested_observables"].as_array_mut().unwrap().reverse();
        let restored: AnalysisRequest = serde_json::from_value(wire).unwrap();
        assert_eq!(restored, request);
        assert_eq!(restored.content_digest().unwrap(), request.content_digest().unwrap());
    }

    #[test]
    fn duplicate_request_observable_rejects_on_restore() {
        let request = request();
        let mut wire = serde_json::to_value(&request).unwrap();
        let duplicate = wire["requested_observables"][0].clone();
        wire["requested_observables"].as_array_mut().unwrap().push(duplicate);
        assert!(serde_json::from_value::<AnalysisRequest>(wire).is_err());
    }

    #[test]
    fn unsupported_request_schema_rejects_on_restore() {
        let mut wire = serde_json::to_value(request()).unwrap();
        wire["schema_version"] = json!(ANALYSIS_REQUEST_SCHEMA_VERSION + 1);
        assert!(serde_json::from_value::<AnalysisRequest>(wire).is_err());
    }

    #[test]
    fn reversed_measurement_interval_rejects_on_restore() {
        let request = request();
        let mut wire = evidence_json(&request);
        wire["observations"][0]["value"]["lower"] = json!(500);
        wire["observations"][0]["value"]["upper"] = json!(400);
        assert!(serde_json::from_value::<AnalysisEvidence>(wire).is_err());
    }

    #[test]
    fn zero_measurement_resolution_rejects_on_restore() {
        let request = request();
        let mut wire = evidence_json(&request);
        wire["observations"][0]["value"]["resolution"] = json!(0);
        assert!(serde_json::from_value::<AnalysisEvidence>(wire).is_err());
    }

    #[test]
    fn invalid_solver_version_rejects_on_restore() {
        let request = request();
        let mut wire = evidence_json(&request);
        wire["solver"]["version"] = json!("");
        assert!(serde_json::from_value::<AnalysisEvidence>(wire).is_err());
    }

    #[test]
    fn valid_raw_evidence_requires_explicit_rebind_for_affirmative_eligibility() {
        let request = request();
        let restored: AnalysisEvidence = serde_json::from_value(evidence_json(&request)).unwrap();
        let validated = restored.rebind(&request).unwrap();
        assert!(validated.may_enter_result_qualification());
        assert_eq!(
            validated.content_digest().unwrap().value,
            "501e1ec4aabe3ebcbc49b0db12ae9a0963415df32dfa040e05e45ae3cf514dc5"
        );
    }

    #[test]
    fn wrong_exact_request_ref_rejects_rebind() {
        let request = request();
        let mut wire = evidence_json(&request);
        wire["request"]["content_digest"]["value"] = json!("forged-request-digest");
        let restored: AnalysisEvidence = serde_json::from_value(wire).unwrap();
        assert!(matches!(
            restored.rebind(&request),
            Err(AnalysisError::RequestIdentityMismatch)
        ));
    }

    #[test]
    fn wrong_design_subject_rejects_rebind() {
        let request = request();
        let mut wire = evidence_json(&request);
        wire["subject"]["content_digest"]["value"] = json!("other-design-digest");
        let restored: AnalysisEvidence = serde_json::from_value(wire).unwrap();
        assert!(matches!(
            restored.rebind(&request),
            Err(AnalysisError::DesignSubjectMismatch)
        ));
    }

    #[test]
    fn failed_converged_wire_state_rejects_before_rebind() {
        let request = request();
        let failed = evidence(
            &request,
            RunDisposition::Failed,
            NumericalDisposition::NotConverged,
            vec![diagnostic_observation()],
            "solver-a",
        );
        let mut wire = serde_json::to_value(failed).unwrap();
        wire["numerical_disposition"] = json!("converged");
        assert!(serde_json::from_value::<AnalysisEvidence>(wire).is_err());
    }

    #[test]
    fn nonconverged_requested_result_rejects_before_rebind() {
        let request = request();
        let mut wire = evidence_json(&request);
        wire["numerical_disposition"] = json!("not_converged");
        assert!(serde_json::from_value::<AnalysisEvidence>(wire).is_err());
    }

    #[test]
    fn unrequested_result_is_raw_retainable_but_rebind_rejects() {
        let request = request();
        let mut wire = evidence_json(&request);
        wire["observations"][0]["observable_id"] = json!("observable:strain");
        let restored: AnalysisEvidence = serde_json::from_value(wire).unwrap();
        assert!(matches!(
            restored.rebind(&request),
            Err(AnalysisError::UnrequestedResultObservable(_))
        ));
    }

    #[test]
    fn dimension_or_unit_substitution_rejects_rebind() {
        let request = request();
        let mut wire = evidence_json(&request);
        wire["observations"][0]["unit_id"] = json!("unit:mm");
        let restored: AnalysisEvidence = serde_json::from_value(wire).unwrap();
        assert!(matches!(
            restored.rebind(&request),
            Err(AnalysisError::ObservableContractMismatch { .. })
        ));
    }

    #[test]
    fn noncanonical_observation_order_rejects_exact_evidence_restore() {
        let request = request();
        let evidence = evidence(
            &request,
            RunDisposition::Completed,
            NumericalDisposition::Converged,
            vec![result_observation(), diagnostic_observation()],
            "solver-a",
        );
        let mut wire = serde_json::to_value(evidence).unwrap();
        wire["observations"].as_array_mut().unwrap().reverse();
        assert!(serde_json::from_value::<AnalysisEvidence>(wire).is_err());
    }

    #[test]
    fn duplicate_observation_rejects_exact_evidence_restore() {
        let request = request();
        let mut wire = evidence_json(&request);
        let duplicate = wire["observations"][0].clone();
        wire["observations"].as_array_mut().unwrap().push(duplicate);
        assert!(serde_json::from_value::<AnalysisEvidence>(wire).is_err());
    }

    #[test]
    fn failed_diagnostic_evidence_round_trips_without_gaining_eligibility() {
        let request = request();
        let failed = evidence(
            &request,
            RunDisposition::Failed,
            NumericalDisposition::NotConverged,
            vec![diagnostic_observation()],
            "solver-a",
        );
        let restored: AnalysisEvidence =
            serde_json::from_str(&serde_json::to_string(&failed).unwrap()).unwrap();
        let validated = restored.rebind(&request).unwrap();
        assert!(!validated.may_enter_result_qualification());
        assert_eq!(validated.evidence().observations().len(), 1);
    }
}
''')
