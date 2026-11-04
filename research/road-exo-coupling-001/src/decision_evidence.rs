//! Evidence-bound decision outcomes.
//!
//! This adapter composes the minimax-regret oracle with the evidence provenance
//! verifier. A raw evidence digest is insufficient: every supported
//! action/hypothesis cell must point to a replay-verified qualification receipt.

use std::collections::{BTreeMap, BTreeSet};

use crate::StableId;
use crate::decision_closure::{
    DecisionCertificate, DecisionError, DecisionManifest, DecisionStatus, OutcomeSupport,
};
use crate::evidence_provenance::{
    EvidenceError, EvidencePacket, GroundTruthEligibility, QualificationReceipt,
    QualificationTarget, SubjectLineage,
};

const DOMAIN: &[u8] = b"symtropy.road-decision-evidence-binding.v0.1\0";

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum DecisionEvidencePurpose {
    /// Measured factual/controlled/natural-experiment outcomes for an action.
    EmpiricalDecision,
    /// Predeclared human-judged behavior or preference constructs.
    HumanAdjudication,
    /// Exact analytic or formal conformance only.
    FormalConformance,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecisionEvidenceBinding {
    pub hypothesis_id: StableId,
    pub action_id: StableId,
    pub purpose: DecisionEvidencePurpose,
    pub packet: EvidencePacket,
    pub subject_lineage: SubjectLineage,
    pub target: QualificationTarget,
    pub qualification: QualificationReceipt,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvidenceBoundDecisionManifest {
    pub decision: DecisionManifest,
    pub bindings: Vec<DecisionEvidenceBinding>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BoundEvidenceReference {
    pub hypothesis_id: StableId,
    pub action_id: StableId,
    pub purpose: DecisionEvidencePurpose,
    pub evidence_id: StableId,
    pub content_digest: [u8; 32],
    pub metadata_digest: [u8; 32],
    pub subject_lineage_digest: [u8; 32],
    pub qualification_receipt_digest: [u8; 32],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvidenceBoundDecisionCertificate {
    pub schema_id: &'static str,
    pub decision_certificate: DecisionCertificate,
    pub evidence_bundle_digest: [u8; 32],
    pub evidence_references: Vec<BoundEvidenceReference>,
    pub receipt_digest: [u8; 32],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DecisionEvidenceError {
    DecisionInvalid(DecisionError),
    DuplicateBinding,
    MissingBinding,
    OrphanBinding,
    QualificationReplayMismatch,
    QualificationReceiptIntegrityFailure,
    OutcomeEvidenceDigestMismatch,
    EvidencePurposeMismatch,
    EvidenceIneligible,
    EvidenceTargetMismatch,
    CertificateMismatch,
    ArithmeticOverflow,
}

impl EvidenceBoundDecisionManifest {
    /// Check the one-to-one relationship between supported outcomes and
    /// purpose-eligible, replay-verified evidence qualification receipts.
    pub fn validate(&self) -> Result<(), DecisionEvidenceError> {
        self.decision
            .validate()
            .map_err(DecisionEvidenceError::DecisionInvalid)?;

        let mut binding_map = BTreeMap::<(StableId, StableId), &DecisionEvidenceBinding>::new();
        for binding in &self.bindings {
            let key = (binding.hypothesis_id.clone(), binding.action_id.clone());
            if binding_map.insert(key, binding).is_some() {
                return Err(DecisionEvidenceError::DuplicateBinding);
            }
        }

        let mut expected_bindings = BTreeSet::<(StableId, StableId)>::new();
        for hypothesis in &self.decision.hypotheses {
            for outcome in &hypothesis.outcomes {
                let key = (hypothesis.hypothesis_id.clone(), outcome.action_id.clone());
                let OutcomeSupport::Supported {
                    evidence_digest, ..
                } = &outcome.support
                else {
                    continue;
                };

                let binding = binding_map
                    .get(&key)
                    .ok_or(DecisionEvidenceError::MissingBinding)?;
                expected_bindings.insert(key);

                if !binding.qualification.integrity_check() {
                    return Err(DecisionEvidenceError::QualificationReceiptIntegrityFailure);
                }

                binding
                    .qualification
                    .verify_against(&binding.packet, &binding.subject_lineage)
                    .map_err(|_error: EvidenceError| {
                        DecisionEvidenceError::QualificationReplayMismatch
                    })?;

                if binding.qualification
                    != binding
                        .packet
                        .qualify_against(&binding.subject_lineage, binding.target.clone())
                {
                    return Err(DecisionEvidenceError::QualificationReplayMismatch);
                }

                if binding.qualification.evidence_digest != binding.packet.content_digest
                    || binding.qualification.evidence_id != binding.packet.evidence_id
                    || binding.qualification.metadata_digest != binding.packet.metadata_digest()
                {
                    return Err(DecisionEvidenceError::QualificationReplayMismatch);
                }

                if *evidence_digest != binding.qualification.receipt_digest {
                    return Err(DecisionEvidenceError::OutcomeEvidenceDigestMismatch);
                }

                if binding.hypothesis_id != hypothesis.hypothesis_id
                    || binding.action_id != outcome.action_id
                {
                    return Err(DecisionEvidenceError::OrphanBinding);
                }

                if !target_matches_purpose_and_action(
                    binding.purpose,
                    &binding.target,
                    &binding.action_id,
                ) {
                    return Err(DecisionEvidenceError::EvidenceTargetMismatch);
                }

                if !eligibility_allows(binding.purpose, binding.qualification.eligibility) {
                    return Err(DecisionEvidenceError::EvidenceIneligible);
                }
            }
        }

        let actual_bindings = binding_map.keys().cloned().collect::<BTreeSet<_>>();
        if actual_bindings != expected_bindings {
            return Err(DecisionEvidenceError::OrphanBinding);
        }

        Ok(())
    }

    pub fn canonical_bytes(&self) -> Result<Vec<u8>, DecisionEvidenceError> {
        self.validate()?;
        let decision_digest = self
            .decision
            .digest()
            .map_err(DecisionEvidenceError::DecisionInvalid)?;

        let mut references = self
            .bindings
            .iter()
            .map(binding_reference)
            .collect::<Vec<_>>();
        references.sort_by(|a, b| {
            (&a.hypothesis_id, &a.action_id).cmp(&(&b.hypothesis_id, &b.action_id))
        });

        let mut out = Vec::new();
        out.extend_from_slice(DOMAIN);
        out.extend_from_slice(&decision_digest);
        push_len(&mut out, references.len());
        for reference in &references {
            push_id(&mut out, &reference.hypothesis_id);
            push_id(&mut out, &reference.action_id);
            out.push(reference.purpose as u8);
            push_id(&mut out, &reference.evidence_id);
            out.extend_from_slice(&reference.content_digest);
            out.extend_from_slice(&reference.metadata_digest);
            out.extend_from_slice(&reference.subject_lineage_digest);
            out.extend_from_slice(&reference.qualification_receipt_digest);
        }
        Ok(out)
    }

    pub fn evidence_bundle_digest(&self) -> Result<[u8; 32], DecisionEvidenceError> {
        Ok(*blake3::hash(&self.canonical_bytes()?).as_bytes())
    }

    pub fn evaluate(&self) -> Result<EvidenceBoundDecisionCertificate, DecisionEvidenceError> {
        self.validate()?;
        let decision_certificate = self
            .decision
            .evaluate()
            .map_err(DecisionEvidenceError::DecisionInvalid)?;
        let evidence_bundle_digest = self.evidence_bundle_digest()?;

        let mut evidence_references = self
            .bindings
            .iter()
            .map(binding_reference)
            .collect::<Vec<_>>();
        evidence_references.sort_by(|a, b| {
            (&a.hypothesis_id, &a.action_id).cmp(&(&b.hypothesis_id, &b.action_id))
        });

        let mut certificate = EvidenceBoundDecisionCertificate {
            schema_id: "symtropy.road-decision-evidence-binding.certificate.v0.1",
            decision_certificate,
            evidence_bundle_digest,
            evidence_references,
            receipt_digest: [0; 32],
        };
        certificate.receipt_digest =
            *blake3::hash(&evidence_bound_certificate_bytes(&certificate)).as_bytes();
        Ok(certificate)
    }
}

impl EvidenceBoundDecisionCertificate {
    /// Unkeyed checksum only; the exact replay method below is the stronger
    /// verification path. This checksum is not authentication.
    pub fn integrity_check(&self) -> bool {
        *blake3::hash(&evidence_bound_certificate_bytes(self)).as_bytes() == self.receipt_digest
    }

    pub fn verify_against(
        &self,
        manifest: &EvidenceBoundDecisionManifest,
    ) -> Result<(), DecisionEvidenceError> {
        let recomputed = manifest.evaluate()?;
        if recomputed == *self {
            Ok(())
        } else {
            Err(DecisionEvidenceError::CertificateMismatch)
        }
    }

    pub fn decision_status(&self) -> DecisionStatus {
        self.decision_certificate.status
    }
}

fn binding_reference(binding: &DecisionEvidenceBinding) -> BoundEvidenceReference {
    BoundEvidenceReference {
        hypothesis_id: binding.hypothesis_id.clone(),
        action_id: binding.action_id.clone(),
        purpose: binding.purpose,
        evidence_id: binding.packet.evidence_id.clone(),
        content_digest: binding.packet.content_digest,
        metadata_digest: binding.packet.metadata_digest(),
        subject_lineage_digest: binding.subject_lineage.digest(),
        qualification_receipt_digest: binding.qualification.receipt_digest,
    }
}

fn target_matches_purpose_and_action(
    purpose: DecisionEvidencePurpose,
    target: &QualificationTarget,
    action_id: &StableId,
) -> bool {
    match (purpose, target) {
        (
            DecisionEvidencePurpose::EmpiricalDecision,
            QualificationTarget::FactualOutcome {
                action_id: target_action,
            },
        )
        | (
            DecisionEvidencePurpose::EmpiricalDecision,
            QualificationTarget::ExecutedInterventionOutcome {
                action_id: target_action,
            },
        )
        | (
            DecisionEvidencePurpose::EmpiricalDecision,
            QualificationTarget::UnexecutedActionOutcome {
                action_id: target_action,
            },
        ) => target_action == action_id,
        (DecisionEvidencePurpose::HumanAdjudication, QualificationTarget::HumanLabel { .. }) => {
            true
        }
        (
            DecisionEvidencePurpose::FormalConformance,
            QualificationTarget::FormalConformance { .. },
        ) => true,
        _ => false,
    }
}

fn eligibility_allows(
    purpose: DecisionEvidencePurpose,
    eligibility: GroundTruthEligibility,
) -> bool {
    match purpose {
        DecisionEvidencePurpose::EmpiricalDecision => matches!(
            eligibility,
            GroundTruthEligibility::IndependentObservedAnchor
                | GroundTruthEligibility::ControlledInterventionOutcome
                | GroundTruthEligibility::QualifiedNaturalExperiment
        ),
        DecisionEvidencePurpose::HumanAdjudication => {
            eligibility == GroundTruthEligibility::HumanAdjudicationReference
        }
        DecisionEvidencePurpose::FormalConformance => {
            eligibility == GroundTruthEligibility::AnalyticReference
        }
    }
}

fn evidence_bound_certificate_bytes(certificate: &EvidenceBoundDecisionCertificate) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"symtropy.road-decision-evidence-binding.certificate.v0.1\0");
    push_bytes(&mut out, certificate.schema_id.as_bytes());
    out.extend_from_slice(&certificate.decision_certificate.receipt_digest);
    out.extend_from_slice(&certificate.evidence_bundle_digest);
    push_len(&mut out, certificate.evidence_references.len());
    for reference in &certificate.evidence_references {
        push_id(&mut out, &reference.hypothesis_id);
        push_id(&mut out, &reference.action_id);
        out.push(reference.purpose as u8);
        push_id(&mut out, &reference.evidence_id);
        out.extend_from_slice(&reference.content_digest);
        out.extend_from_slice(&reference.metadata_digest);
        out.extend_from_slice(&reference.subject_lineage_digest);
        out.extend_from_slice(&reference.qualification_receipt_digest);
    }
    out
}

fn push_id(out: &mut Vec<u8>, id: &StableId) {
    let bytes = id.as_str().as_bytes();
    let length = u32::try_from(bytes.len()).expect("StableId length is bounded");
    out.extend_from_slice(&length.to_le_bytes());
    out.extend_from_slice(bytes);
}

fn push_bytes(out: &mut Vec<u8>, bytes: &[u8]) {
    let length = u64::try_from(bytes.len()).expect("schema byte length must fit u64");
    out.extend_from_slice(&length.to_le_bytes());
    out.extend_from_slice(bytes);
}

fn push_len(out: &mut Vec<u8>, length: usize) {
    let length = u64::try_from(length).expect("collection length must fit u64");
    out.extend_from_slice(&length.to_le_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decision_closure::{DecisionHypothesis, ExactRational, HypothesisActionOutcome};
    use crate::evidence_provenance::{
        ActionRealization, EvidenceClaim, EvidenceLineage, EvidenceOrigin, EvidenceUse,
    };
    use std::collections::BTreeSet;

    fn id(value: &str) -> StableId {
        StableId::new(value).expect("fixture ID")
    }

    fn subject() -> SubjectLineage {
        SubjectLineage {
            complete: true,
            generator_ancestry: BTreeSet::from([id("decision-model-v1")]),
            upstream_source_digests: BTreeSet::from([[7; 32]]),
            parent_content_digests: BTreeSet::from([[6; 32]]),
        }
    }

    fn packet(action: &str, index: u8) -> EvidencePacket {
        EvidencePacket {
            evidence_id: id(format!("observed-record-{action}")),
            content_digest: [index; 32],
            origin: EvidenceOrigin::ObservedFactual,
            claim: EvidenceClaim::FactualObservation,
            use_role: EvidenceUse::SealedTest,
            dataset_id: id("sealed-road-cohort"),
            dataset_version: id("v1"),
            source_system_id: id("instrumented-road-vehicle"),
            action_id: Some(id(action)),
            action_realization: ActionRealization::Observed,
            assignment: None,
            measurement_calibration_id: Some(id("calibration-v1")),
            identification: None,
            formal_domain_id: None,
            annotation: None,
            lineage: EvidenceLineage {
                complete: true,
                generator_ancestry: BTreeSet::from([id("independent-capture-v1")]),
                upstream_source_digests: BTreeSet::from([[8; 32]]),
                parent_content_digests: BTreeSet::from([[9; 32]]),
            },
        }
    }

    fn target(action: &str) -> QualificationTarget {
        QualificationTarget::FactualOutcome {
            action_id: id(action),
        }
    }

    fn binding(hypothesis: &str, action: &str, index: u8) -> DecisionEvidenceBinding {
        let packet = packet(action, index);
        let subject = subject();
        let target = target(action);
        let qualification = packet.qualify_against(&subject, target.clone());
        DecisionEvidenceBinding {
            hypothesis_id: id(hypothesis),
            action_id: id(action),
            purpose: DecisionEvidencePurpose::EmpiricalDecision,
            packet,
            subject_lineage: subject,
            target,
            qualification,
        }
    }

    fn decision_and_bindings() -> EvidenceBoundDecisionManifest {
        let hypothesis_id = id("traffic-dependence-hypothesis-v1");
        let binding_a = binding("traffic-dependence-hypothesis-v1", "action-a", 1);
        let binding_b = binding("traffic-dependence-hypothesis-v1", "action-b", 2);
        let decision = DecisionManifest {
            decision_id: id("crossing-decision-v1"),
            version: id("v0.1"),
            utility_scale_id: id("common-risk-adjusted-utility-v1"),
            utility_semantics_id: id("higher-is-better-net-benefit-v1"),
            actions: vec![id("action-a"), id("action-b")],
            hypotheses: vec![DecisionHypothesis {
                hypothesis_id,
                reference_manifest_digest: [11; 32],
                outcomes: vec![
                    HypothesisActionOutcome {
                        action_id: id("action-a"),
                        support: OutcomeSupport::Supported {
                            utility: ExactRational::new(10, 1).unwrap(),
                            evidence_digest: binding_a.qualification.receipt_digest,
                        },
                    },
                    HypothesisActionOutcome {
                        action_id: id("action-b"),
                        support: OutcomeSupport::Supported {
                            utility: ExactRational::new(4, 1).unwrap(),
                            evidence_digest: binding_b.qualification.receipt_digest,
                        },
                    },
                ],
            }],
        };
        EvidenceBoundDecisionManifest {
            decision,
            bindings: vec![binding_a, binding_b],
        }
    }

    #[test]
    fn empirical_decision_requires_exact_observed_action_receipts() {
        let manifest = decision_and_bindings();
        manifest.validate().unwrap();
        let certificate = manifest.evaluate().unwrap();
        assert!(certificate.integrity_check());
        assert_eq!(
            certificate.decision_status(),
            DecisionStatus::DecisionStable
        );
        assert_eq!(certificate.evidence_references.len(), 2);
        assert_eq!(certificate.verify_against(&manifest), Ok(()));
    }

    #[test]
    fn missing_supported_outcome_binding_rejects() {
        let mut manifest = decision_and_bindings();
        manifest.bindings.pop();
        assert_eq!(
            manifest.validate(),
            Err(DecisionEvidenceError::MissingBinding)
        );
    }

    #[test]
    fn duplicate_binding_rejects() {
        let mut manifest = decision_and_bindings();
        manifest.bindings.push(manifest.bindings[0].clone());
        assert_eq!(
            manifest.validate(),
            Err(DecisionEvidenceError::DuplicateBinding)
        );
    }

    #[test]
    fn orphan_binding_rejects() {
        let mut manifest = decision_and_bindings();
        let mut extra = binding("unknown-hypothesis", "action-c", 3);
        extra.action_id = id("action-c");
        manifest.bindings.push(extra);
        assert_eq!(
            manifest.validate(),
            Err(DecisionEvidenceError::OrphanBinding)
        );
    }

    #[test]
    fn outcome_digest_must_equal_qualification_receipt_digest() {
        let mut manifest = decision_and_bindings();
        if let OutcomeSupport::Supported {
            evidence_digest, ..
        } = &mut manifest.decision.hypotheses[0].outcomes[0].support
        {
            *evidence_digest = [99; 32];
        }
        assert_eq!(
            manifest.validate(),
            Err(DecisionEvidenceError::OutcomeEvidenceDigestMismatch)
        );
    }

    #[test]
    fn simulator_reference_cannot_support_empirical_decision() {
        let mut manifest = decision_and_bindings();
        let binding = &mut manifest.bindings[0];
        binding.packet.origin = EvidenceOrigin::SimulatorGenerated;
        binding.packet.claim = EvidenceClaim::SimulatorOutput;
        binding.qualification = binding
            .packet
            .qualify_against(&binding.subject_lineage, binding.target.clone());
        if let OutcomeSupport::Supported {
            evidence_digest, ..
        } = &mut manifest.decision.hypotheses[0].outcomes[0].support
        {
            *evidence_digest = binding.qualification.receipt_digest;
        }
        assert_eq!(
            manifest.validate(),
            Err(DecisionEvidenceError::EvidenceIneligible)
        );
    }

    #[test]
    fn model_dependent_lineage_cannot_support_empirical_decision() {
        let mut manifest = decision_and_bindings();
        let binding = &mut manifest.bindings[0];
        binding.packet.lineage.generator_ancestry = BTreeSet::from([id("decision-model-v1")]);
        binding.qualification = binding
            .packet
            .qualify_against(&binding.subject_lineage, binding.target.clone());
        if let OutcomeSupport::Supported {
            evidence_digest, ..
        } = &mut manifest.decision.hypotheses[0].outcomes[0].support
        {
            *evidence_digest = binding.qualification.receipt_digest;
        }
        assert_eq!(
            manifest.validate(),
            Err(DecisionEvidenceError::EvidenceIneligible)
        );
    }

    #[test]
    fn factual_reference_must_match_the_selected_action() {
        let mut manifest = decision_and_bindings();
        let binding = &mut manifest.bindings[0];
        binding.target = target("action-b");
        binding.qualification = binding
            .packet
            .qualify_against(&binding.subject_lineage, binding.target.clone());
        if let OutcomeSupport::Supported {
            evidence_digest, ..
        } = &mut manifest.decision.hypotheses[0].outcomes[0].support
        {
            *evidence_digest = binding.qualification.receipt_digest;
        }
        assert_eq!(
            manifest.validate(),
            Err(DecisionEvidenceError::EvidenceTargetMismatch)
        );
    }

    #[test]
    fn fit_data_cannot_support_a_sealed_empirical_decision() {
        let mut manifest = decision_and_bindings();
        let binding = &mut manifest.bindings[0];
        binding.packet.use_role = EvidenceUse::Fit;
        binding.qualification = binding
            .packet
            .qualify_against(&binding.subject_lineage, binding.target.clone());
        if let OutcomeSupport::Supported {
            evidence_digest, ..
        } = &mut manifest.decision.hypotheses[0].outcomes[0].support
        {
            *evidence_digest = binding.qualification.receipt_digest;
        }
        assert_eq!(
            manifest.validate(),
            Err(DecisionEvidenceError::EvidenceIneligible)
        );
    }

    #[test]
    fn human_adjudication_requires_exact_human_reference_type() {
        let mut manifest = decision_and_bindings();
        let binding = &mut manifest.bindings[0];
        binding.purpose = DecisionEvidencePurpose::HumanAdjudication;
        assert_eq!(
            manifest.validate(),
            Err(DecisionEvidenceError::EvidenceTargetMismatch)
        );
    }

    #[test]
    fn formal_reference_cannot_be_used_for_empirical_decision() {
        let mut manifest = decision_and_bindings();
        let binding = &mut manifest.bindings[0];
        binding.packet.origin = EvidenceOrigin::FormalReference;
        binding.packet.claim = EvidenceClaim::FormalResult;
        binding.packet.formal_domain_id = Some(id("finite-fixture-v1"));
        binding.target = QualificationTarget::FormalConformance {
            domain_id: id("finite-fixture-v1"),
        };
        binding.qualification = binding
            .packet
            .qualify_against(&binding.subject_lineage, binding.target.clone());
        if let OutcomeSupport::Supported {
            evidence_digest, ..
        } = &mut manifest.decision.hypotheses[0].outcomes[0].support
        {
            *evidence_digest = binding.qualification.receipt_digest;
        }
        assert_eq!(
            manifest.validate(),
            Err(DecisionEvidenceError::EvidenceTargetMismatch)
        );
    }

    #[test]
    fn evidence_binding_order_does_not_change_bundle_digest() {
        let first = decision_and_bindings();
        let mut second = first.clone();
        second.bindings.reverse();
        assert_eq!(
            first.evidence_bundle_digest().unwrap(),
            second.evidence_bundle_digest().unwrap()
        );
        assert_eq!(first.evaluate().unwrap(), second.evaluate().unwrap());
    }

    #[test]
    fn packet_metadata_tampering_fails_qualification_replay() {
        let mut manifest = decision_and_bindings();
        manifest.bindings[0].packet.dataset_version = id("v2");
        assert_eq!(
            manifest.validate(),
            Err(DecisionEvidenceError::QualificationReplayMismatch)
        );
    }

    #[test]
    fn qualification_receipt_tampering_fails_integrity_check() {
        let mut manifest = decision_and_bindings();
        manifest.bindings[0].qualification.metadata_digest[0] ^= 1;
        assert_eq!(
            manifest.validate(),
            Err(DecisionEvidenceError::QualificationReceiptIntegrityFailure)
        );
    }

    #[test]
    fn human_adjudication_is_accepted_only_for_human_reference_purpose() {
        let mut manifest = decision_and_bindings();
        let binding = &mut manifest.bindings[0];
        binding.purpose = DecisionEvidencePurpose::HumanAdjudication;
        binding.packet.origin = EvidenceOrigin::HumanAdjudication;
        binding.packet.claim = EvidenceClaim::HumanJudgement;
        binding.packet.annotation = Some(crate::evidence_provenance::AnnotationQuality {
            rubric_id: id("crossing-behavior-rubric-v1"),
            assessor_protocol_id: id("double-blind-panel-v1"),
            disagreement_digest: [12; 32],
            assessor_count: 3,
        });
        binding.target = QualificationTarget::HumanLabel {
            rubric_id: id("crossing-behavior-rubric-v1"),
        };
        binding.qualification = binding
            .packet
            .qualify_against(&binding.subject_lineage, binding.target.clone());
        if let OutcomeSupport::Supported {
            evidence_digest, ..
        } = &mut manifest.decision.hypotheses[0].outcomes[0].support
        {
            *evidence_digest = binding.qualification.receipt_digest;
        }
        assert_eq!(manifest.validate(), Ok(()));
    }

    #[test]
    fn formal_reference_is_accepted_only_for_formal_conformance_purpose() {
        let mut manifest = decision_and_bindings();
        let binding = &mut manifest.bindings[0];
        binding.purpose = DecisionEvidencePurpose::FormalConformance;
        binding.packet.origin = EvidenceOrigin::FormalReference;
        binding.packet.claim = EvidenceClaim::FormalResult;
        binding.packet.formal_domain_id = Some(id("finite-decision-fixture-v1"));
        binding.target = QualificationTarget::FormalConformance {
            domain_id: id("finite-decision-fixture-v1"),
        };
        binding.qualification = binding
            .packet
            .qualify_against(&binding.subject_lineage, binding.target.clone());
        if let OutcomeSupport::Supported {
            evidence_digest, ..
        } = &mut manifest.decision.hypotheses[0].outcomes[0].support
        {
            *evidence_digest = binding.qualification.receipt_digest;
        }
        assert_eq!(manifest.validate(), Ok(()));
    }

    #[test]
    fn unsupported_outcome_needs_no_evidence_binding_and_stays_unsupported() {
        let mut manifest = decision_and_bindings();
        manifest.bindings.pop();
        manifest.decision.hypotheses[0].outcomes[1].support = OutcomeSupport::Unsupported {
            reason_id: id("counterfactual-action-outside-support"),
        };
        let certificate = manifest.evaluate().unwrap();
        assert_eq!(certificate.decision_status(), DecisionStatus::Unsupported);
        assert_eq!(certificate.evidence_references.len(), 1);
        assert_eq!(certificate.decision_certificate.minimax_regret_action, None);
    }

    #[test]
    fn evidence_receipt_digest_is_stable_when_only_utility_changes() {
        let mut manifest = decision_and_bindings();
        let digest = manifest.bindings[0].qualification.receipt_digest;
        if let OutcomeSupport::Supported { utility, .. } =
            &mut manifest.decision.hypotheses[0].outcomes[0].support
        {
            *utility = ExactRational::new(9, 1).unwrap();
        }
        assert_eq!(manifest.bindings[0].qualification.receipt_digest, digest);
        assert_ne!(
            manifest
                .evaluate()
                .unwrap()
                .decision_certificate
                .receipt_digest,
            decision_and_bindings()
                .evaluate()
                .unwrap()
                .decision_certificate
                .receipt_digest
        );
    }
}
