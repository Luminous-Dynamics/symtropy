//! Versioned bridge between SAM #10514 path-support assessments and
//! evidence-bound minimax-regret decisions.
//!
//! A supported trajectory is necessary for a supported action/hypothesis cell,
//! but it is not sufficient evidence of utility. The utility must still pass
//! #3635/#3638 evidence qualification. Simulator-generated outcomes remain
//! SimulationGenerated / FormalConformanceOnly and cannot be upgraded here.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

use crate::StableId;
use crate::decision_closure::{DecisionStatus, OutcomeSupport};
use crate::decision_evidence::{
    DecisionEvidenceError, EvidenceBoundDecisionCertificate, EvidenceBoundDecisionManifest,
};

const BUNDLE_DOMAIN: &[u8] = b"symtropy.road-sam-trajectory-evidence-binding.v0.1\0";
const RECEIPT_DOMAIN: &[u8] = b"symtropy.road-sam-support-assessment.v0.1\0";
pub const ROAD_SAM_ASSESSMENT_ENVELOPE_SCHEMA_V0_1: &str =
    "symtropy.road-sam-support-assessment-envelope.v0.1";

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum SamTrajectoryDisposition {
    TrajectorySupported = 1,
    UnsupportedSequence = 2,
    SupportUnknown = 3,
    Incomplete = 4,
    OutOfScope = 5,
}

/// Exact SAM support result plus the identities of the input artifacts that
/// produced it. Digests are content commitments supplied by the adapter; this
/// module does not authenticate the SAM build or the caller's authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SamTrajectoryAssessmentBinding {
    pub schema_id: String,
    pub hypothesis_id: StableId,
    pub action_id: StableId,
    pub sam_build_digest: [u8; 32],
    /// The SAM SupportManifest evidence-manifest identifier, retained verbatim.
    pub evidence_manifest_id: String,
    pub support_manifest_digest: [u8; 32],
    pub query_digest: [u8; 32],
    pub target_policy_digest: String,
    pub context_digest: String,
    pub path_universe_exhaustive: bool,
    pub query_complete: bool,
    pub decision_points: u64,
    pub matched_finite_path: bool,
    pub disposition: SamTrajectoryDisposition,
    /// BLAKE3 digest of SAM AssessmentReceipt::canonical_bytes().
    pub sam_assessment_receipt_digest: [u8; 32],
    /// Unkeyed digest over every preceding field; not a signature.
    pub assessment_receipt_digest: [u8; 32],
}

impl SamTrajectoryAssessmentBinding {
    /// Seal after checking the exact source-generated SAM AssessmentReceipt,
    /// which binds the canonical support manifest, query and assessment bytes.
    /// The caller still supplies the external build commitment; this method
    /// does not authenticate that build.
    pub fn seal_from_sam_artifacts(
        mut self,
        support_manifest_bytes: &[u8],
        query_bytes: &[u8],
        assessment_receipt_bytes: &[u8],
    ) -> Result<Self, TrajectoryEvidenceError> {
        if *blake3::hash(support_manifest_bytes).as_bytes() != self.support_manifest_digest {
            return Err(TrajectoryEvidenceError::SupportManifestArtifactMismatch);
        }
        if *blake3::hash(query_bytes).as_bytes() != self.query_digest {
            return Err(TrajectoryEvidenceError::QueryArtifactMismatch);
        }
        if self.canonical_assessment_receipt_bytes(support_manifest_bytes, query_bytes)
            != assessment_receipt_bytes
        {
            return Err(TrajectoryEvidenceError::AssessmentArtifactMismatch);
        }
        self.sam_assessment_receipt_digest = *blake3::hash(assessment_receipt_bytes).as_bytes();
        self.seal()
    }

    /// Verify the retained envelope against the exact SAM source artifacts,
    /// including the source-generated receipt that binds the result to those
    /// exact inputs, rather than trusting digest fields alone.
    pub fn verify_against_sam_artifacts(
        &self,
        support_manifest_bytes: &[u8],
        query_bytes: &[u8],
        assessment_receipt_bytes: &[u8],
    ) -> Result<(), TrajectoryEvidenceError> {
        self.verify_receipt()?;
        if *blake3::hash(support_manifest_bytes).as_bytes() != self.support_manifest_digest {
            return Err(TrajectoryEvidenceError::SupportManifestArtifactMismatch);
        }
        if *blake3::hash(query_bytes).as_bytes() != self.query_digest {
            return Err(TrajectoryEvidenceError::QueryArtifactMismatch);
        }
        if self.canonical_assessment_receipt_bytes(support_manifest_bytes, query_bytes)
            != assessment_receipt_bytes
            || *blake3::hash(assessment_receipt_bytes).as_bytes() != self.sam_assessment_receipt_digest
        {
            return Err(TrajectoryEvidenceError::AssessmentArtifactMismatch);
        }
        Ok(())
    }

    fn canonical_assessment_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(b"sam.policy-trajectory-support.assessment.v0.1\0");
        out.push(self.disposition as u8);
        out.extend_from_slice(&self.decision_points.to_le_bytes());
        out.push(u8::from(self.matched_finite_path));
        out
    }

    fn canonical_assessment_receipt_bytes(
        &self,
        support_manifest_bytes: &[u8],
        query_bytes: &[u8],
    ) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(b"sam.policy-trajectory-support.assessment-receipt.v0.1\0");
        push_bytes(&mut out, support_manifest_bytes);
        push_bytes(&mut out, query_bytes);
        push_bytes(&mut out, &self.canonical_assessment_bytes());
        out
    }

    /// Seal a typed SAM assessment after validating disposition invariants.
    fn seal(mut self) -> Result<Self, TrajectoryEvidenceError> {
        self.validate_fields()?;
        self.assessment_receipt_digest = *blake3::hash(&self.receipt_bytes()).as_bytes();
        Ok(self)
    }

    pub fn verify_receipt(&self) -> Result<(), TrajectoryEvidenceError> {
        self.validate_fields()?;
        if self.assessment_receipt_digest != *blake3::hash(&self.receipt_bytes()).as_bytes() {
            return Err(TrajectoryEvidenceError::AssessmentReceiptMismatch);
        }
        Ok(())
    }

    fn validate_fields(&self) -> Result<(), TrajectoryEvidenceError> {
        if self.schema_id != ROAD_SAM_ASSESSMENT_ENVELOPE_SCHEMA_V0_1 {
            return Err(TrajectoryEvidenceError::WrongSamAssessmentSchema);
        }
        require_digest(&self.sam_build_digest, "sam_build_digest")?;
        require_identity(&self.evidence_manifest_id, "evidence_manifest_id")?;
        require_digest(&self.support_manifest_digest, "support_manifest_digest")?;
        require_digest(&self.query_digest, "query_digest")?;
        require_digest(&self.sam_assessment_receipt_digest, "sam_assessment_receipt_digest")?;
        require_identity(&self.target_policy_digest, "target_policy_digest")?;
        require_identity(&self.context_digest, "context_digest")?;

        match self.disposition {
            SamTrajectoryDisposition::TrajectorySupported => {
                if !self.query_complete || !self.matched_finite_path || self.decision_points == 0 {
                    return Err(TrajectoryEvidenceError::InvalidSamDisposition);
                }
            }
            SamTrajectoryDisposition::UnsupportedSequence => {
                if !self.query_complete
                    || !self.path_universe_exhaustive
                    || self.matched_finite_path
                    || self.decision_points == 0
                {
                    return Err(TrajectoryEvidenceError::InvalidSamDisposition);
                }
            }
            SamTrajectoryDisposition::SupportUnknown => {
                if !self.query_complete
                    || self.path_universe_exhaustive
                    || self.matched_finite_path
                    || self.decision_points == 0
                {
                    return Err(TrajectoryEvidenceError::InvalidSamDisposition);
                }
            }
            SamTrajectoryDisposition::Incomplete => {
                if self.query_complete || self.matched_finite_path {
                    return Err(TrajectoryEvidenceError::InvalidSamDisposition);
                }
            }
            SamTrajectoryDisposition::OutOfScope => {
                if !self.query_complete || self.matched_finite_path || self.decision_points == 0 {
                    return Err(TrajectoryEvidenceError::InvalidSamDisposition);
                }
            }
        }
        Ok(())
    }

    fn receipt_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(256);
        out.extend_from_slice(RECEIPT_DOMAIN);
        push_string(&mut out, &self.schema_id);
        push_id(&mut out, &self.hypothesis_id);
        push_id(&mut out, &self.action_id);
        out.extend_from_slice(&self.sam_build_digest);
        push_string(&mut out, &self.evidence_manifest_id);
        out.extend_from_slice(&self.support_manifest_digest);
        out.extend_from_slice(&self.query_digest);
        push_string(&mut out, &self.target_policy_digest);
        push_string(&mut out, &self.context_digest);
        out.push(u8::from(self.path_universe_exhaustive));
        out.push(u8::from(self.query_complete));
        out.extend_from_slice(&self.decision_points.to_le_bytes());
        out.push(u8::from(self.matched_finite_path));
        out.push(self.disposition as u8);
        out.extend_from_slice(&self.sam_assessment_receipt_digest);
        out
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrajectoryEvidenceBoundDecisionManifest {
    /// The existing #3638 bundle remains the sole owner of utility evidence.
    pub evidence_bound_decision: EvidenceBoundDecisionManifest,
    /// Exactly one SAM path-support assessment for every action/hypothesis cell.
    pub trajectory_support: Vec<SamTrajectoryAssessmentBinding>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrajectorySupportReference {
    pub hypothesis_id: StableId,
    pub action_id: StableId,
    pub disposition: SamTrajectoryDisposition,
    pub sam_build_digest: [u8; 32],
    pub evidence_manifest_id: String,
    pub support_manifest_digest: [u8; 32],
    pub query_digest: [u8; 32],
    pub sam_assessment_receipt_digest: [u8; 32],
    pub assessment_receipt_digest: [u8; 32],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrajectoryEvidenceBoundDecisionCertificate {
    pub schema_id: &'static str,
    pub evidence_bound_decision: EvidenceBoundDecisionCertificate,
    pub trajectory_support_bundle_digest: [u8; 32],
    pub trajectory_support: Vec<TrajectorySupportReference>,
    /// Unkeyed content checksum; not authentication.
    pub receipt_digest: [u8; 32],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TrajectoryEvidenceError {
    DecisionEvidence(DecisionEvidenceError),
    WrongSamAssessmentSchema,
    EmptyIdentity(&'static str),
    InvalidDigest(&'static str),
    InvalidSamDisposition,
    DuplicateSupportBinding,
    MissingSupportBinding,
    OrphanSupportBinding,
    AssessmentReceiptMismatch,
    SupportManifestArtifactMismatch,
    QueryArtifactMismatch,
    AssessmentArtifactMismatch,
    SupportOutcomeMismatch,
    ArithmeticOverflow,
    CertificateMismatch,
}

impl TrajectoryEvidenceBoundDecisionManifest {
    /// Validate one-to-one action×hypothesis path support and its consistency
    /// with #3637 outcome support, then invoke #3638 evidence qualification.
    pub fn validate(&self) -> Result<(), TrajectoryEvidenceError> {
        self.evidence_bound_decision
            .validate()
            .map_err(TrajectoryEvidenceError::DecisionEvidence)?;

        let decision = &self.evidence_bound_decision.decision;
        let expected_count = decision
            .actions
            .len()
            .checked_mul(decision.hypotheses.len())
            .ok_or(TrajectoryEvidenceError::ArithmeticOverflow)?;
        let mut support_map =
            BTreeMap::<(StableId, StableId), &SamTrajectoryAssessmentBinding>::new();
        for binding in &self.trajectory_support {
            binding.verify_receipt()?;
            if support_map
                .insert(
                    (binding.hypothesis_id.clone(), binding.action_id.clone()),
                    binding,
                )
                .is_some()
            {
                return Err(TrajectoryEvidenceError::DuplicateSupportBinding);
            }
        }

        let hypothesis_ids = decision
            .hypotheses
            .iter()
            .map(|hypothesis| hypothesis.hypothesis_id.clone())
            .collect::<BTreeSet<_>>();
        let action_ids = decision.actions.iter().cloned().collect::<BTreeSet<_>>();

        for binding in &self.trajectory_support {
            if !hypothesis_ids.contains(&binding.hypothesis_id)
                || !action_ids.contains(&binding.action_id)
            {
                return Err(TrajectoryEvidenceError::OrphanSupportBinding);
            }
        }

        for hypothesis in &decision.hypotheses {
            for outcome in &hypothesis.outcomes {
                let key = (hypothesis.hypothesis_id.clone(), outcome.action_id.clone());
                let binding = support_map
                    .get(&key)
                    .ok_or(TrajectoryEvidenceError::MissingSupportBinding)?;
                if !support_matches_outcome(binding.disposition, &outcome.support) {
                    return Err(TrajectoryEvidenceError::SupportOutcomeMismatch);
                }
            }
        }

        if support_map.len() < expected_count {
            return Err(TrajectoryEvidenceError::MissingSupportBinding);
        }
        if support_map.len() > expected_count {
            return Err(TrajectoryEvidenceError::OrphanSupportBinding);
        }
        Ok(())
    }

    pub fn trajectory_support_bundle_digest(&self) -> Result<[u8; 32], TrajectoryEvidenceError> {
        self.validate()?;
        let mut bindings = self.trajectory_support.iter().collect::<Vec<_>>();
        bindings.sort_by(|left, right| {
            (&left.hypothesis_id, &left.action_id).cmp(&(&right.hypothesis_id, &right.action_id))
        });
        let mut hasher = blake3::Hasher::new();
        hasher.update(BUNDLE_DOMAIN);
        let evidence_digest = self
            .evidence_bound_decision
            .evidence_bundle_digest()
            .map_err(TrajectoryEvidenceError::DecisionEvidence)?;
        hasher.update(&evidence_digest);
        push_len_hasher(&mut hasher, bindings.len());
        for binding in bindings {
            hasher.update(&binding.receipt_bytes());
            hasher.update(&binding.assessment_receipt_digest);
        }
        Ok(*hasher.finalize().as_bytes())
    }

    pub fn evaluate(
        &self,
    ) -> Result<TrajectoryEvidenceBoundDecisionCertificate, TrajectoryEvidenceError> {
        self.validate()?;
        let evidence_bound_decision = self
            .evidence_bound_decision
            .evaluate()
            .map_err(TrajectoryEvidenceError::DecisionEvidence)?;
        let trajectory_support_bundle_digest = self.trajectory_support_bundle_digest()?;

        let mut trajectory_support = self
            .trajectory_support
            .iter()
            .map(support_reference)
            .collect::<Vec<_>>();
        trajectory_support.sort_by(|left, right| {
            (&left.hypothesis_id, &left.action_id).cmp(&(&right.hypothesis_id, &right.action_id))
        });

        let mut certificate = TrajectoryEvidenceBoundDecisionCertificate {
            schema_id: "symtropy.road-trajectory-evidence-bound-decision.v0.1",
            evidence_bound_decision,
            trajectory_support_bundle_digest,
            trajectory_support,
            receipt_digest: [0; 32],
        };
        certificate.receipt_digest = *blake3::hash(&certificate_bytes(&certificate)).as_bytes();
        Ok(certificate)
    }
}

impl TrajectoryEvidenceBoundDecisionCertificate {
    pub fn integrity_check(&self) -> bool {
        *blake3::hash(&certificate_bytes(self)).as_bytes() == self.receipt_digest
    }

    pub fn verify_against(
        &self,
        manifest: &TrajectoryEvidenceBoundDecisionManifest,
    ) -> Result<(), TrajectoryEvidenceError> {
        let expected = manifest.evaluate()?;
        if expected == *self {
            Ok(())
        } else {
            Err(TrajectoryEvidenceError::CertificateMismatch)
        }
    }

    pub fn decision_status(&self) -> DecisionStatus {
        self.evidence_bound_decision.decision_status()
    }
}

fn support_matches_outcome(
    disposition: SamTrajectoryDisposition,
    outcome: &OutcomeSupport,
) -> bool {
    match disposition {
        SamTrajectoryDisposition::TrajectorySupported => true,
        SamTrajectoryDisposition::UnsupportedSequence => {
            matches!(outcome, OutcomeSupport::Unsupported { .. })
        }
        SamTrajectoryDisposition::SupportUnknown => {
            matches!(outcome, OutcomeSupport::NotIdentified { .. })
        }
        SamTrajectoryDisposition::Incomplete | SamTrajectoryDisposition::OutOfScope => {
            matches!(outcome, OutcomeSupport::Incomplete { .. })
        }
    }
}

fn support_reference(binding: &SamTrajectoryAssessmentBinding) -> TrajectorySupportReference {
    TrajectorySupportReference {
        hypothesis_id: binding.hypothesis_id.clone(),
        action_id: binding.action_id.clone(),
        disposition: binding.disposition,
        sam_build_digest: binding.sam_build_digest,
        evidence_manifest_id: binding.evidence_manifest_id.clone(),
        support_manifest_digest: binding.support_manifest_digest,
        query_digest: binding.query_digest,
        sam_assessment_receipt_digest: binding.sam_assessment_receipt_digest,
        assessment_receipt_digest: binding.assessment_receipt_digest,
    }
}

fn certificate_bytes(certificate: &TrajectoryEvidenceBoundDecisionCertificate) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"symtropy.road-trajectory-evidence-bound-decision.v0.1\0");
    push_string(&mut out, certificate.schema_id);
    out.extend_from_slice(&certificate.evidence_bound_decision.receipt_digest);
    out.extend_from_slice(&certificate.trajectory_support_bundle_digest);
    push_len(&mut out, certificate.trajectory_support.len());
    for reference in &certificate.trajectory_support {
        push_id(&mut out, &reference.hypothesis_id);
        push_id(&mut out, &reference.action_id);
        out.push(reference.disposition as u8);
        out.extend_from_slice(&reference.sam_build_digest);
        push_string(&mut out, &reference.evidence_manifest_id);
        out.extend_from_slice(&reference.support_manifest_digest);
        out.extend_from_slice(&reference.query_digest);
        out.extend_from_slice(&reference.sam_assessment_receipt_digest);
        out.extend_from_slice(&reference.assessment_receipt_digest);
    }
    out
}

fn require_identity(value: &str, field: &'static str) -> Result<(), TrajectoryEvidenceError> {
    if value.trim().is_empty() {
        Err(TrajectoryEvidenceError::EmptyIdentity(field))
    } else {
        Ok(())
    }
}

fn require_digest(value: &[u8; 32], field: &'static str) -> Result<(), TrajectoryEvidenceError> {
    if *value == [0; 32] {
        Err(TrajectoryEvidenceError::InvalidDigest(field))
    } else {
        Ok(())
    }
}

fn push_id(out: &mut Vec<u8>, id: &StableId) {
    push_string(out, id.as_str());
}

fn push_bytes(out: &mut Vec<u8>, value: &[u8]) {
    let len = u64::try_from(value.len()).expect("byte length fits u64");
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(value);
}

fn push_string(out: &mut Vec<u8>, value: &str) {
    let len = u64::try_from(value.len()).expect("string length fits u64");
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(value.as_bytes());
}

fn push_len(out: &mut Vec<u8>, length: usize) {
    let len = u64::try_from(length).expect("collection length fits u64");
    out.extend_from_slice(&len.to_le_bytes());
}

fn push_len_hasher(hasher: &mut blake3::Hasher, length: usize) {
    let len = u64::try_from(length).expect("collection length fits u64");
    hasher.update(&len.to_le_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decision_closure::{
        DecisionHypothesis, DecisionManifest, ExactRational, HypothesisActionOutcome,
    };
    use crate::decision_evidence::{DecisionEvidenceBinding, DecisionEvidencePurpose};
    use crate::evidence_provenance::{
        ActionRealization, EvidenceClaim, EvidenceLineage, EvidenceOrigin, EvidencePacket,
        EvidenceUse, QualificationTarget, SubjectLineage,
    };
    use std::collections::BTreeSet;

    fn id(value: &str) -> StableId {
        StableId::new(value).unwrap()
    }

    fn canonical_vector(name: &str) -> Vec<u8> {
        let hex = include_str!("../fixtures/sam-canonical-artifact-vectors.v0.1.txt")
            .lines()
            .find_map(|line| line.strip_prefix(&format!("{name}=")))
            .expect("shared SAM golden vector is present");
        hex.as_bytes()
            .chunks_exact(2)
            .map(|pair| {
                let text = std::str::from_utf8(pair).expect("hex is UTF-8");
                u8::from_str_radix(text, 16).expect("golden vector is hexadecimal")
            })
            .collect()
    }

    fn subject() -> SubjectLineage {
        SubjectLineage {
            complete: true,
            generator_ancestry: BTreeSet::from([id("decision-source-model")]),
            upstream_source_digests: BTreeSet::from([[7; 32]]),
            parent_content_digests: BTreeSet::from([[6; 32]]),
        }
    }

    fn packet(action: &str, index: u8) -> EvidencePacket {
        EvidencePacket {
            evidence_id: id(&format!("observed-record-{action}")),
            content_digest: [index; 32],
            origin: EvidenceOrigin::ObservedFactual,
            claim: EvidenceClaim::FactualObservation,
            use_role: EvidenceUse::SealedTest,
            dataset_id: id("sealed-trajectory-cohort"),
            dataset_version: id("v1"),
            source_system_id: id("instrumented-capture-system"),
            action_id: Some(id(action)),
            action_realization: ActionRealization::Observed,
            assignment: None,
            measurement_calibration_id: Some(id("calibration-v1")),
            identification: None,
            formal_domain_id: None,
            annotation: None,
            lineage: EvidenceLineage {
                complete: true,
                generator_ancestry: BTreeSet::from([id("independent-measurement-system")]),
                upstream_source_digests: BTreeSet::from([[8; 32]]),
                parent_content_digests: BTreeSet::from([[9; 32]]),
            },
        }
    }

    fn empirical_binding(
        hypothesis_id: &str,
        action_id: &str,
        index: u8,
    ) -> DecisionEvidenceBinding {
        let packet = packet(action_id, index);
        let subject = subject();
        let target = QualificationTarget::FactualOutcome {
            action_id: id(action_id),
        };
        let qualification = packet.qualify_against(&subject, target.clone());
        DecisionEvidenceBinding {
            hypothesis_id: id(hypothesis_id),
            action_id: id(action_id),
            purpose: DecisionEvidencePurpose::EmpiricalDecision,
            packet,
            subject_lineage: subject,
            target,
            qualification,
        }
    }

    fn evidence_manifest(
        outcome: OutcomeSupport,
        bindings: Vec<DecisionEvidenceBinding>,
    ) -> EvidenceBoundDecisionManifest {
        EvidenceBoundDecisionManifest {
            decision: DecisionManifest {
                decision_id: id("road-crossing-choice"),
                version: id("v0.1"),
                utility_scale_id: id("calibrated-net-benefit-v1"),
                utility_semantics_id: id("higher-is-better-v1"),
                actions: vec![id("action-a")],
                hypotheses: vec![DecisionHypothesis {
                    hypothesis_id: id("traffic-model-h1"),
                    reference_manifest_digest: [11; 32],
                    outcomes: vec![HypothesisActionOutcome {
                        action_id: id("action-a"),
                        support: outcome,
                    }],
                }],
            },
            bindings,
        }
    }

    fn sam_binding(disposition: SamTrajectoryDisposition) -> SamTrajectoryAssessmentBinding {
        let (path_universe_exhaustive, query_complete, decision_points, matched_finite_path) =
            match disposition {
                SamTrajectoryDisposition::TrajectorySupported => (false, true, 2, true),
                SamTrajectoryDisposition::UnsupportedSequence => (true, true, 2, false),
                SamTrajectoryDisposition::SupportUnknown => (false, true, 2, false),
                SamTrajectoryDisposition::Incomplete => (false, false, 1, false),
                SamTrajectoryDisposition::OutOfScope => (true, true, 2, false),
            };
        let mut binding = SamTrajectoryAssessmentBinding {
            schema_id: ROAD_SAM_ASSESSMENT_ENVELOPE_SCHEMA_V0_1.to_owned(),
            hypothesis_id: id("traffic-model-h1"),
            action_id: id("action-a"),
            sam_build_digest: [1; 32],
            evidence_manifest_id: "evidence-v1".to_owned(),
            support_manifest_digest: [3; 32],
            query_digest: [4; 32],
            sam_assessment_receipt_digest: [0; 32],
            target_policy_digest: "policy-digest".to_owned(),
            context_digest: "context-digest".to_owned(),
            path_universe_exhaustive,
            query_complete,
            decision_points,
            matched_finite_path,
            disposition,
            assessment_receipt_digest: [0; 32],
        };
        binding.sam_assessment_receipt_digest = *blake3::hash(&binding.canonical_assessment_bytes()).as_bytes();
        binding.seal().unwrap()
    }

    fn bundle(
        outcome: OutcomeSupport,
        disposition: SamTrajectoryDisposition,
        with_evidence: bool,
    ) -> TrajectoryEvidenceBoundDecisionManifest {
        let bindings = if with_evidence {
            vec![empirical_binding("traffic-model-h1", "action-a", 1)]
        } else {
            Vec::new()
        };
        TrajectoryEvidenceBoundDecisionManifest {
            evidence_bound_decision: evidence_manifest(outcome, bindings),
            trajectory_support: vec![sam_binding(disposition)],
        }
    }

    #[test]
    fn supported_path_does_not_replace_action_utility_evidence() {
        let no_utility_evidence = bundle(
            OutcomeSupport::Supported {
                utility: ExactRational::new(10, 1).unwrap(),
                evidence_digest: [9; 32],
            },
            SamTrajectoryDisposition::TrajectorySupported,
            false,
        );
        assert!(matches!(
            no_utility_evidence.validate(),
            Err(TrajectoryEvidenceError::DecisionEvidence(
                DecisionEvidenceError::MissingBinding
            ))
        ));

        let valid = bundle(
            OutcomeSupport::Supported {
                utility: ExactRational::new(10, 1).unwrap(),
                evidence_digest: empirical_binding("traffic-model-h1", "action-a", 1)
                    .qualification
                    .receipt_digest,
            },
            SamTrajectoryDisposition::TrajectorySupported,
            true,
        );
        let certificate = valid.evaluate().unwrap();
        assert_eq!(
            certificate.decision_status(),
            DecisionStatus::DecisionStable
        );
        assert!(certificate.integrity_check());
        assert_eq!(certificate.verify_against(&valid), Ok(()));
    }

    #[test]
    fn support_unknown_requires_not_identified_utility_cell() {
        let valid = bundle(
            OutcomeSupport::NotIdentified {
                evidence_ref: id("trajectory-support-incomplete"),
            },
            SamTrajectoryDisposition::SupportUnknown,
            false,
        );
        let certificate = valid.evaluate().unwrap();
        assert_eq!(certificate.decision_status(), DecisionStatus::Incomplete);
    }

    #[test]
    fn incomplete_or_out_of_scope_path_cannot_be_marked_empirically_supported() {
        for disposition in [
            SamTrajectoryDisposition::Incomplete,
            SamTrajectoryDisposition::OutOfScope,
        ] {
            let manifest = bundle(
                OutcomeSupport::Incomplete {
                    reason_id: id("trajectory-query-incomplete"),
                },
                disposition,
                false,
            );
            let certificate = manifest.evaluate().unwrap();
            assert_eq!(certificate.decision_status(), DecisionStatus::Incomplete);
        }
    }

    #[test]
    fn exhaustive_missing_path_maps_to_unsupported_not_zero_utility() {
        let manifest = bundle(
            OutcomeSupport::Unsupported {
                reason_id: id("exhaustive-finite-ledger-absence"),
            },
            SamTrajectoryDisposition::UnsupportedSequence,
            false,
        );
        let certificate = manifest.evaluate().unwrap();
        assert_eq!(certificate.decision_status(), DecisionStatus::Unsupported);
    }

    #[test]
    fn unsupported_or_unknown_sam_path_cannot_be_labelled_supported_utility() {
        let evidence = empirical_binding("traffic-model-h1", "action-a", 1);
        let supported_utility = OutcomeSupport::Supported {
            utility: ExactRational::new(10, 1).unwrap(),
            evidence_digest: evidence.qualification.receipt_digest,
        };
        for disposition in [
            SamTrajectoryDisposition::UnsupportedSequence,
            SamTrajectoryDisposition::SupportUnknown,
            SamTrajectoryDisposition::Incomplete,
            SamTrajectoryDisposition::OutOfScope,
        ] {
            let manifest = TrajectoryEvidenceBoundDecisionManifest {
                evidence_bound_decision: evidence_manifest(
                    supported_utility.clone(),
                    vec![evidence.clone()],
                ),
                trajectory_support: vec![sam_binding(disposition)],
            };
            assert_eq!(
                manifest.validate(),
                Err(TrajectoryEvidenceError::SupportOutcomeMismatch)
            );
        }
    }
    #[test]
    fn supported_path_may_still_have_unidentified_utility() {
        // Trajectory membership is necessary for a supported utility cell,
        // but is not evidence that the utility itself has been identified.
        let manifest = bundle(
            OutcomeSupport::NotIdentified {
                evidence_ref: id("utility-not-identified"),
            },
            SamTrajectoryDisposition::TrajectorySupported,
            false,
        );
        let certificate = manifest.evaluate().unwrap();
        assert_eq!(certificate.decision_status(), DecisionStatus::Incomplete);
    }

    #[test]
    fn all_action_hypothesis_cells_require_exactly_one_support_assessment() {
        let actions = vec![id("action-a"), id("action-b")];
        let hypotheses = vec![
            DecisionHypothesis {
                hypothesis_id: id("traffic-model-h1"),
                reference_manifest_digest: [11; 32],
                outcomes: vec![
                    HypothesisActionOutcome {
                        action_id: id("action-a"),
                        support: OutcomeSupport::NotIdentified {
                            evidence_ref: id("not-identified-a"),
                        },
                    },
                    HypothesisActionOutcome {
                        action_id: id("action-b"),
                        support: OutcomeSupport::NotIdentified {
                            evidence_ref: id("not-identified-b"),
                        },
                    },
                ],
            },
            DecisionHypothesis {
                hypothesis_id: id("traffic-model-h2"),
                reference_manifest_digest: [12; 32],
                outcomes: vec![
                    HypothesisActionOutcome {
                        action_id: id("action-a"),
                        support: OutcomeSupport::NotIdentified {
                            evidence_ref: id("not-identified-c"),
                        },
                    },
                    HypothesisActionOutcome {
                        action_id: id("action-b"),
                        support: OutcomeSupport::NotIdentified {
                            evidence_ref: id("not-identified-d"),
                        },
                    },
                ],
            },
        ];
        let decision = DecisionManifest {
            decision_id: id("two-action-two-hypothesis"),
            version: id("v0.1"),
            utility_scale_id: id("common-utility-v1"),
            utility_semantics_id: id("higher-is-better-v1"),
            actions: actions.clone(),
            hypotheses,
        };
        let mut support = Vec::new();
        for hypothesis in ["traffic-model-h1", "traffic-model-h2"] {
            for action in ["action-a", "action-b"] {
                let mut binding = sam_binding(SamTrajectoryDisposition::SupportUnknown);
                binding.hypothesis_id = id(hypothesis);
                binding.action_id = id(action);
                support.push(binding.seal().unwrap());
            }
        }
        let evidence_bound_decision = EvidenceBoundDecisionManifest {
            decision,
            bindings: Vec::new(),
        };
        let complete = TrajectoryEvidenceBoundDecisionManifest {
            evidence_bound_decision: evidence_bound_decision.clone(),
            trajectory_support: support.clone(),
        };
        assert_eq!(complete.validate(), Ok(()));
        let mut missing = complete.clone();
        missing.trajectory_support.pop();
        assert_eq!(
            missing.validate(),
            Err(TrajectoryEvidenceError::MissingSupportBinding)
        );
        let mut duplicate = complete.clone();
        duplicate.trajectory_support.push(support[0].clone());
        assert_eq!(
            duplicate.validate(),
            Err(TrajectoryEvidenceError::DuplicateSupportBinding)
        );
        let mut orphan = complete;
        orphan.trajectory_support[0].action_id = id("action-orphan");
        orphan.trajectory_support[0].assessment_receipt_digest = [0; 32];
        orphan.trajectory_support[0] = orphan.trajectory_support[0].clone().seal().unwrap();
        assert_eq!(
            orphan.validate(),
            Err(TrajectoryEvidenceError::OrphanSupportBinding)
        );
    }

    #[test]
    fn exact_sam_canonical_artifacts_are_verified_before_envelope_acceptance() {
        let support_bytes = canonical_vector("manifest");
        let query_bytes = canonical_vector("query");
        let assessment_bytes = canonical_vector("assessment");
        let assessment_receipt_bytes = canonical_vector("assessment_receipt");
        let mut binding = sam_binding(SamTrajectoryDisposition::SupportUnknown);
        binding.decision_points = 1;
        binding.support_manifest_digest = *blake3::hash(&support_bytes).as_bytes();
        binding.query_digest = *blake3::hash(&query_bytes).as_bytes();
        assert_eq!(binding.canonical_assessment_bytes(), assessment_bytes);
        assert_eq!(
            binding.canonical_assessment_receipt_bytes(&support_bytes, &query_bytes),
            assessment_receipt_bytes
        );

        let sealed = binding
            .clone()
            .seal_from_sam_artifacts(&support_bytes, &query_bytes, &assessment_receipt_bytes)
            .unwrap();
        assert_eq!(
            sealed.verify_against_sam_artifacts(
                &support_bytes,
                &query_bytes,
                &assessment_receipt_bytes
            ),
            Ok(())
        );
        assert_eq!(
            binding.clone().seal_from_sam_artifacts(
                &support_bytes,
                &query_bytes,
                &assessment_bytes
            ),
            Err(TrajectoryEvidenceError::AssessmentArtifactMismatch)
        );
        assert_eq!(
            sealed.verify_against_sam_artifacts(
                b"mutated-support-manifest",
                &query_bytes,
                &assessment_bytes
            ),
            Err(TrajectoryEvidenceError::SupportManifestArtifactMismatch)
        );
        assert_eq!(
            sealed.verify_against_sam_artifacts(
                &support_bytes,
                b"mutated-query",
                &assessment_bytes
            ),
            Err(TrajectoryEvidenceError::QueryArtifactMismatch)
        );
        let mut mutated_assessment_receipt = assessment_receipt_bytes.clone();
        mutated_assessment_receipt.push(0);
        assert_eq!(
            sealed.verify_against_sam_artifacts(
                &support_bytes,
                &query_bytes,
                &mutated_assessment_receipt
            ),
            Err(TrajectoryEvidenceError::AssessmentArtifactMismatch)
        );
    }

    #[test]
    fn out_of_scope_requires_an_actual_support_query_scope_mismatch() {
        let mut binding = sam_binding(SamTrajectoryDisposition::OutOfScope);
        assert_eq!(binding.verify_receipt(), Ok(()));

        binding.query_target_policy_digest = binding.support_target_policy_digest.clone();
        assert_eq!(
            binding.seal(),
            Err(TrajectoryEvidenceError::InvalidSamDisposition)
        );
    }

    #[test]
    fn in_scope_disposition_rejects_mismatched_policy_or_context() {
        let mut binding = sam_binding(SamTrajectoryDisposition::SupportUnknown);
        binding.query_context_digest = "different-context".to_owned();
        assert_eq!(
            binding.seal(),
            Err(TrajectoryEvidenceError::InvalidSamDisposition)
        );
    }

    #[test]
    fn repeated_scope_metadata_is_bound_by_source_assessment_receipt() {
        let support_bytes = canonical_vector("manifest");
        let query_bytes = canonical_vector("query");
        let assessment_receipt_bytes = canonical_vector("assessment_receipt");
        let mut binding = sam_binding(SamTrajectoryDisposition::SupportUnknown);
        binding.decision_points = 1;
        binding.support_target_policy_digest = "policy-v1".to_owned();
        binding.support_context_digest = "context-v1".to_owned();
        binding.query_target_policy_digest = "policy-v1".to_owned();
        binding.query_context_digest = "context-v1".to_owned();
        binding.support_manifest_digest = *blake3::hash(&support_bytes).as_bytes();
        binding.query_digest = *blake3::hash(&query_bytes).as_bytes();

        let sealed = binding
            .seal_from_sam_artifacts(&support_bytes, &query_bytes, &assessment_receipt_bytes)
            .unwrap();
        let mut tampered = sealed.clone();
        tampered.support_target_policy_digest = "other-policy".to_owned();
        assert_eq!(
            tampered.verify_against_sam_artifacts(
                &support_bytes,
                &query_bytes,
                &assessment_receipt_bytes
            ),
            Err(TrajectoryEvidenceError::AssessmentReceiptMismatch)
        );
    }

    #[test]
    fn wrong_envelope_schema_and_empty_provenance_digests_reject() {
        let mut binding = sam_binding(SamTrajectoryDisposition::SupportUnknown);
        binding.schema_id = "sam.policy-trajectory-support.assessment.v0.1".to_owned();
        assert_eq!(
            binding.seal(),
            Err(TrajectoryEvidenceError::WrongSamAssessmentSchema)
        );

        let mut binding = sam_binding(SamTrajectoryDisposition::SupportUnknown);
        binding.query_digest = [0; 32];
        assert_eq!(
            binding.seal(),
            Err(TrajectoryEvidenceError::InvalidDigest("query_digest"))
        );
    }

    #[test]
    fn receipt_mutation_is_rejected_and_binding_order_is_irrelevant() {
        let base = bundle(
            OutcomeSupport::NotIdentified {
                evidence_ref: id("support-not-exhaustive"),
            },
            SamTrajectoryDisposition::SupportUnknown,
            false,
        );
        let certificate = base.evaluate().unwrap();
        let mut tampered = base.clone();
        tampered.trajectory_support[0].query_digest = [44; 32];
        assert_eq!(
            tampered.validate(),
            Err(TrajectoryEvidenceError::AssessmentReceiptMismatch)
        );
        assert_eq!(
            certificate.verify_against(&tampered),
            Err(TrajectoryEvidenceError::AssessmentReceiptMismatch)
        );

        let mut reordered = base.clone();
        reordered.trajectory_support.reverse();
        assert_eq!(
            base.trajectory_support_bundle_digest().unwrap(),
            reordered.trajectory_support_bundle_digest().unwrap()
        );
    }
}
