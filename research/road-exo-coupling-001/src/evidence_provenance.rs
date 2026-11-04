//! Evidence provenance, ground-truth eligibility and split-leakage conformance.
//!
//! This module does not certify the accuracy of an empirical model. It prevents
//! evidence classes and their lineage from being silently conflated.

use std::collections::{BTreeMap, BTreeSet};

use crate::StableId;

const DOMAIN: &[u8] = b"symtropy.road-reality-gap.evidence.v0.1\0";

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum EvidenceOrigin {
    ObservedFactual,
    ControlledIntervention,
    NaturalExperiment,
    SimulatorGenerated,
    ModelGenerated,
    FormalReference,
    HumanAdjudication,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum EvidenceClaim {
    FactualObservation,
    ControlledOutcome,
    NaturalExperimentEstimate,
    SimulatorOutput,
    ModelCounterfactual,
    FormalResult,
    HumanJudgement,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum EvidenceUse {
    Fit,
    Calibration,
    DevelopmentValidation,
    SealedTest,
    SafetyChallenge,
    IllustrativeOnly,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum ActionRealization {
    Observed,
    AssignedAndExecuted,
    NotExecuted,
    Unknown,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum AssignmentMethod {
    Randomized,
    Counterbalanced,
    ControlledPerturbation,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssignmentProtocol {
    pub protocol_id: StableId,
    pub method: AssignmentMethod,
    pub protocol_digest: [u8; 32],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IdentificationPackage {
    pub estimand_id: StableId,
    pub assumptions_digest: [u8; 32],
    pub overlap_evidence_id: StableId,
    pub sensitivity_plan_id: StableId,
    pub preregistration_id: StableId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnnotationQuality {
    pub rubric_id: StableId,
    pub assessor_protocol_id: StableId,
    pub disagreement_digest: [u8; 32],
    pub assessor_count: u32,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct EvidenceLineage {
    pub complete: bool,
    pub generator_ancestry: BTreeSet<StableId>,
    pub upstream_source_digests: BTreeSet<[u8; 32]>,
    pub parent_content_digests: BTreeSet<[u8; 32]>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvidencePacket {
    pub evidence_id: StableId,
    pub content_digest: [u8; 32],
    pub origin: EvidenceOrigin,
    pub claim: EvidenceClaim,
    pub use_role: EvidenceUse,
    pub dataset_id: StableId,
    pub dataset_version: StableId,
    pub source_system_id: StableId,
    pub action_id: Option<StableId>,
    pub action_realization: ActionRealization,
    pub assignment: Option<AssignmentProtocol>,
    pub measurement_calibration_id: Option<StableId>,
    pub identification: Option<IdentificationPackage>,
    pub formal_domain_id: Option<StableId>,
    pub annotation: Option<AnnotationQuality>,
    pub lineage: EvidenceLineage,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubjectLineage {
    pub complete: bool,
    pub generator_ancestry: BTreeSet<StableId>,
    pub upstream_source_digests: BTreeSet<[u8; 32]>,
    pub parent_content_digests: BTreeSet<[u8; 32]>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum QualificationTarget {
    FactualOutcome { action_id: StableId },
    ExecutedInterventionOutcome { action_id: StableId },
    UnexecutedActionOutcome { action_id: StableId },
    FormalConformance { domain_id: StableId },
    HumanLabel { rubric_id: StableId },
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum GroundTruthEligibility {
    IndependentObservedAnchor,
    ControlledInterventionOutcome,
    QualifiedNaturalExperiment,
    AnalyticReference,
    HumanAdjudicationReference,
    ModelDependentReference,
    SimulationOnly,
    NotIdentified,
    Incomplete,
    Contaminated,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum QualificationReason {
    FactualMeasurementMatchesExecutedAction,
    ControlledAssignmentAndMeasurement,
    IdentificationPackagePresent,
    FormalDomainMatches,
    AnnotationProtocolComplete,
    SharedGeneratorAncestry,
    SharedUpstreamSource,
    ModelGeneratedReference,
    SimulatorGeneratedReference,
    FactualRecordCannotIdentifyUnexecutedAction,
    EvidenceTypeClaimMismatch,
    ActionMismatch,
    MissingCalibration,
    MissingAssignmentProtocol,
    MissingIdentificationPackage,
    MissingAnnotationQuality,
    MissingLineage,
    MissingFormalDomain,
    UnsupportedEvidenceClass,
    IllustrativeOnlyCannotBeReference,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QualificationReceipt {
    pub schema_id: &'static str,
    pub evidence_id: StableId,
    pub evidence_digest: [u8; 32],
    pub metadata_digest: [u8; 32],
    pub target: QualificationTarget,
    pub eligibility: GroundTruthEligibility,
    pub reason: QualificationReason,
    pub subject_lineage_digest: [u8; 32],
    pub receipt_digest: [u8; 32],
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum DatasetSplit {
    Fit,
    Calibration,
    DevelopmentValidation,
    SealedTest,
    SafetyChallenge,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct SplitEntry {
    pub record_id: StableId,
    pub split: DatasetSplit,
    pub lineage_complete: bool,
    pub scenario_family_id: StableId,
    pub drive_id: StableId,
    pub capture_session_id: StableId,
    pub upstream_parent_digests: BTreeSet<[u8; 32]>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SplitManifest {
    pub manifest_id: StableId,
    pub dataset_version: StableId,
    pub entries: Vec<SplitEntry>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EvidenceError {
    EmptyEntries,
    DuplicateRecord,
    IncompleteSplitLineage,
    LeakageDetected,
    DuplicateParentDigest,
    ArithmeticOverflow,
    ReceiptMismatch,
}

impl EvidenceLineage {
    pub fn digest(&self) -> [u8; 32] {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"symtropy.road-reality-gap.lineage.v0.1\0");
        bytes.push(u8::from(self.complete));
        push_len(&mut bytes, self.generator_ancestry.len());
        for item in &self.generator_ancestry {
            push_id(&mut bytes, item);
        }
        push_len(&mut bytes, self.upstream_source_digests.len());
        for digest in &self.upstream_source_digests {
            bytes.extend_from_slice(digest);
        }
        push_len(&mut bytes, self.parent_content_digests.len());
        for digest in &self.parent_content_digests {
            bytes.extend_from_slice(digest);
        }
        *blake3::hash(&bytes).as_bytes()
    }
}

impl SubjectLineage {
    pub fn digest(&self) -> [u8; 32] {
        let lineage = EvidenceLineage {
            complete: self.complete,
            generator_ancestry: self.generator_ancestry.clone(),
            upstream_source_digests: self.upstream_source_digests.clone(),
            parent_content_digests: self.parent_content_digests.clone(),
        };
        lineage.digest()
    }
}

impl EvidencePacket {
    pub fn metadata_digest(&self) -> [u8; 32] {
        *blake3::hash(&self.metadata_bytes()).as_bytes()
    }

    pub fn metadata_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(b"symtropy.road-reality-gap.evidence-packet.v0.1\0");
        push_id(&mut out, &self.evidence_id);
        out.extend_from_slice(&self.content_digest);
        out.push(self.origin as u8);
        out.push(self.claim as u8);
        out.push(self.use_role as u8);
        push_id(&mut out, &self.dataset_id);
        push_id(&mut out, &self.dataset_version);
        push_id(&mut out, &self.source_system_id);
        push_option_id(&mut out, self.action_id.as_ref());
        out.push(self.action_realization as u8);
        match &self.assignment {
            Some(assignment) => {
                out.push(1);
                push_id(&mut out, &assignment.protocol_id);
                out.push(assignment.method as u8);
                out.extend_from_slice(&assignment.protocol_digest);
            }
            None => out.push(0),
        }
        push_option_id(&mut out, self.measurement_calibration_id.as_ref());
        match &self.identification {
            Some(identification) => {
                out.push(1);
                push_id(&mut out, &identification.estimand_id);
                out.extend_from_slice(&identification.assumptions_digest);
                push_id(&mut out, &identification.overlap_evidence_id);
                push_id(&mut out, &identification.sensitivity_plan_id);
                push_id(&mut out, &identification.preregistration_id);
            }
            None => out.push(0),
        }
        push_option_id(&mut out, self.formal_domain_id.as_ref());
        match &self.annotation {
            Some(annotation) => {
                out.push(1);
                push_id(&mut out, &annotation.rubric_id);
                push_id(&mut out, &annotation.assessor_protocol_id);
                out.extend_from_slice(&annotation.disagreement_digest);
                out.extend_from_slice(&annotation.assessor_count.to_le_bytes());
            }
            None => out.push(0),
        }
        out.extend_from_slice(&self.lineage.digest());
        out
    }

    pub fn validate_shape(&self) -> Result<(), QualificationReason> {
        let valid_pair = matches!(
            (self.origin, self.claim),
            (
                EvidenceOrigin::ObservedFactual,
                EvidenceClaim::FactualObservation
            ) | (
                EvidenceOrigin::ControlledIntervention,
                EvidenceClaim::ControlledOutcome
            ) | (
                EvidenceOrigin::NaturalExperiment,
                EvidenceClaim::NaturalExperimentEstimate
            ) | (
                EvidenceOrigin::SimulatorGenerated,
                EvidenceClaim::SimulatorOutput
            ) | (
                EvidenceOrigin::ModelGenerated,
                EvidenceClaim::ModelCounterfactual
            ) | (EvidenceOrigin::FormalReference, EvidenceClaim::FormalResult)
                | (
                    EvidenceOrigin::HumanAdjudication,
                    EvidenceClaim::HumanJudgement
                )
        );
        if !valid_pair {
            return Err(QualificationReason::EvidenceTypeClaimMismatch);
        }
        if !self.lineage.complete {
            return Err(QualificationReason::MissingLineage);
        }
        Ok(())
    }

    pub fn qualify_against(
        &self,
        subject: &SubjectLineage,
        target: QualificationTarget,
    ) -> QualificationReceipt {
        let (eligibility, reason) = self.qualify_inner(subject, &target);
        let mut receipt = QualificationReceipt {
            schema_id: "symtropy.road-reality-gap.qualification.v0.1",
            evidence_id: self.evidence_id.clone(),
            evidence_digest: self.content_digest,
            metadata_digest: self.metadata_digest(),
            target,
            eligibility,
            reason,
            subject_lineage_digest: subject.digest(),
            receipt_digest: [0; 32],
        };
        receipt.receipt_digest = *blake3::hash(&qualification_bytes(&receipt)).as_bytes();
        receipt
    }

    fn qualify_inner(
        &self,
        subject: &SubjectLineage,
        target: &QualificationTarget,
    ) -> (GroundTruthEligibility, QualificationReason) {
        if !subject.complete || !self.lineage.complete {
            return (
                GroundTruthEligibility::Incomplete,
                QualificationReason::MissingLineage,
            );
        }

        if !self
            .lineage
            .generator_ancestry
            .is_disjoint(&subject.generator_ancestry)
        {
            return (
                GroundTruthEligibility::ModelDependentReference,
                QualificationReason::SharedGeneratorAncestry,
            );
        }
        let evidence_sources = self
            .lineage
            .upstream_source_digests
            .union(&self.lineage.parent_content_digests)
            .copied()
            .chain(std::iter::once(self.content_digest))
            .collect::<BTreeSet<_>>();
        let subject_sources = subject
            .upstream_source_digests
            .union(&subject.parent_content_digests)
            .copied()
            .collect::<BTreeSet<_>>();
        if !evidence_sources.is_disjoint(&subject_sources) {
            return (
                GroundTruthEligibility::ModelDependentReference,
                QualificationReason::SharedUpstreamSource,
            );
        }

        if self.use_role == EvidenceUse::IllustrativeOnly {
            return (
                GroundTruthEligibility::NotIdentified,
                QualificationReason::IllustrativeOnlyCannotBeReference,
            );
        }

        if let Err(reason) = self.validate_shape() {
            let status = if reason == QualificationReason::MissingLineage {
                GroundTruthEligibility::Incomplete
            } else {
                GroundTruthEligibility::Contaminated
            };
            return (status, reason);
        }

        match (target, self.origin) {
            (
                QualificationTarget::FactualOutcome { action_id },
                EvidenceOrigin::ObservedFactual,
            ) => {
                if self.action_realization != ActionRealization::Observed
                    || self.action_id.as_ref() != Some(action_id)
                {
                    return (
                        GroundTruthEligibility::NotIdentified,
                        QualificationReason::ActionMismatch,
                    );
                }
                if self.measurement_calibration_id.is_none() {
                    return (
                        GroundTruthEligibility::Incomplete,
                        QualificationReason::MissingCalibration,
                    );
                }
                (
                    GroundTruthEligibility::IndependentObservedAnchor,
                    QualificationReason::FactualMeasurementMatchesExecutedAction,
                )
            }
            (
                QualificationTarget::ExecutedInterventionOutcome { action_id },
                EvidenceOrigin::ControlledIntervention,
            )
            | (
                QualificationTarget::UnexecutedActionOutcome { action_id },
                EvidenceOrigin::ControlledIntervention,
            ) => {
                if self.action_realization != ActionRealization::AssignedAndExecuted
                    || self.action_id.as_ref() != Some(action_id)
                {
                    return (
                        GroundTruthEligibility::NotIdentified,
                        QualificationReason::ActionMismatch,
                    );
                }
                if self.assignment.is_none() {
                    return (
                        GroundTruthEligibility::Incomplete,
                        QualificationReason::MissingAssignmentProtocol,
                    );
                }
                if self.measurement_calibration_id.is_none() {
                    return (
                        GroundTruthEligibility::Incomplete,
                        QualificationReason::MissingCalibration,
                    );
                }
                (
                    GroundTruthEligibility::ControlledInterventionOutcome,
                    QualificationReason::ControlledAssignmentAndMeasurement,
                )
            }
            (
                QualificationTarget::UnexecutedActionOutcome { action_id },
                EvidenceOrigin::NaturalExperiment,
            ) => {
                if self.action_realization != ActionRealization::AssignedAndExecuted
                    || self.action_id.as_ref() != Some(action_id)
                {
                    return (
                        GroundTruthEligibility::NotIdentified,
                        QualificationReason::ActionMismatch,
                    );
                }
                if self.measurement_calibration_id.is_none() {
                    return (
                        GroundTruthEligibility::Incomplete,
                        QualificationReason::MissingCalibration,
                    );
                }
                if self.identification.is_none() {
                    return (
                        GroundTruthEligibility::NotIdentified,
                        QualificationReason::MissingIdentificationPackage,
                    );
                }
                (
                    GroundTruthEligibility::QualifiedNaturalExperiment,
                    QualificationReason::IdentificationPackagePresent,
                )
            }
            (
                QualificationTarget::FormalConformance { domain_id },
                EvidenceOrigin::FormalReference,
            ) => {
                if self.formal_domain_id.as_ref() != Some(domain_id) {
                    return (
                        GroundTruthEligibility::NotIdentified,
                        QualificationReason::MissingFormalDomain,
                    );
                }
                (
                    GroundTruthEligibility::AnalyticReference,
                    QualificationReason::FormalDomainMatches,
                )
            }
            (QualificationTarget::HumanLabel { rubric_id }, EvidenceOrigin::HumanAdjudication) => {
                let annotation = match &self.annotation {
                    Some(annotation) => annotation,
                    None => {
                        return (
                            GroundTruthEligibility::Incomplete,
                            QualificationReason::MissingAnnotationQuality,
                        );
                    }
                };
                if &annotation.rubric_id != rubric_id || annotation.assessor_count == 0 {
                    return (
                        GroundTruthEligibility::Incomplete,
                        QualificationReason::MissingAnnotationQuality,
                    );
                }
                (
                    GroundTruthEligibility::HumanAdjudicationReference,
                    QualificationReason::AnnotationProtocolComplete,
                )
            }
            (_, EvidenceOrigin::SimulatorGenerated) => (
                GroundTruthEligibility::SimulationOnly,
                QualificationReason::SimulatorGeneratedReference,
            ),
            (_, EvidenceOrigin::ModelGenerated) => (
                GroundTruthEligibility::ModelDependentReference,
                QualificationReason::ModelGeneratedReference,
            ),
            (
                QualificationTarget::UnexecutedActionOutcome { .. },
                EvidenceOrigin::ObservedFactual,
            ) => (
                GroundTruthEligibility::NotIdentified,
                QualificationReason::FactualRecordCannotIdentifyUnexecutedAction,
            ),
            _ => (
                GroundTruthEligibility::NotIdentified,
                QualificationReason::UnsupportedEvidenceClass,
            ),
        }
    }
}

impl QualificationReceipt {
    /// Unkeyed checksum only; this is not authentication or independent evidence.
    pub fn integrity_check(&self) -> bool {
        *blake3::hash(&qualification_bytes(self)).as_bytes() == self.receipt_digest
    }

    /// Recompute the eligibility decision and every bound evidence/lineage digest.
    pub fn verify_against(
        &self,
        packet: &EvidencePacket,
        subject: &SubjectLineage,
    ) -> Result<(), EvidenceError> {
        let recomputed = packet.qualify_against(subject, self.target.clone());
        if recomputed == *self {
            Ok(())
        } else {
            Err(EvidenceError::ReceiptMismatch)
        }
    }
}

impl SplitManifest {
    pub fn validate(&self) -> Result<(), EvidenceError> {
        if self.entries.is_empty() {
            return Err(EvidenceError::EmptyEntries);
        }

        let mut records = BTreeSet::new();
        let mut scenario_splits = BTreeMap::<StableId, DatasetSplit>::new();
        let mut drive_splits = BTreeMap::<StableId, DatasetSplit>::new();
        let mut session_splits = BTreeMap::<StableId, DatasetSplit>::new();
        let mut parent_splits = BTreeMap::<[u8; 32], DatasetSplit>::new();

        for entry in &self.entries {
            if !records.insert(entry.record_id.clone()) {
                return Err(EvidenceError::DuplicateRecord);
            }
            if !entry.lineage_complete
                || entry.scenario_family_id.as_str().is_empty()
                || entry.drive_id.as_str().is_empty()
                || entry.capture_session_id.as_str().is_empty()
            {
                return Err(EvidenceError::IncompleteSplitLineage);
            }

            register_split(
                &mut scenario_splits,
                entry.scenario_family_id.clone(),
                entry.split,
            )?;
            register_split(&mut drive_splits, entry.drive_id.clone(), entry.split)?;
            register_split(
                &mut session_splits,
                entry.capture_session_id.clone(),
                entry.split,
            )?;
            for digest in &entry.upstream_parent_digests {
                register_split(&mut parent_splits, *digest, entry.split)?;
            }
        }
        Ok(())
    }

    pub fn canonical_bytes(&self) -> Result<Vec<u8>, EvidenceError> {
        self.validate()?;
        let mut out = Vec::new();
        out.extend_from_slice(DOMAIN);
        push_id(&mut out, &self.manifest_id);
        push_id(&mut out, &self.dataset_version);

        let mut entries = self.entries.iter().collect::<Vec<_>>();
        entries.sort_by(|a, b| a.record_id.cmp(&b.record_id));
        push_len(&mut out, entries.len());
        for entry in entries {
            push_id(&mut out, &entry.record_id);
            out.push(match entry.split {
                DatasetSplit::Fit => 1,
                DatasetSplit::Calibration => 2,
                DatasetSplit::DevelopmentValidation => 3,
                DatasetSplit::SealedTest => 4,
                DatasetSplit::SafetyChallenge => 5,
            });
            push_id(&mut out, &entry.scenario_family_id);
            push_id(&mut out, &entry.drive_id);
            push_id(&mut out, &entry.capture_session_id);
            push_len(&mut out, entry.upstream_parent_digests.len());
            for digest in &entry.upstream_parent_digests {
                out.extend_from_slice(digest);
            }
        }
        Ok(out)
    }

    pub fn digest(&self) -> Result<[u8; 32], EvidenceError> {
        Ok(*blake3::hash(&self.canonical_bytes()?).as_bytes())
    }

    pub fn verify_frozen(&self, expected_digest: [u8; 32]) -> Result<(), EvidenceError> {
        if self.digest()? == expected_digest {
            Ok(())
        } else {
            Err(EvidenceError::LeakageDetected)
        }
    }
}

fn register_split<K: Ord>(
    registry: &mut BTreeMap<K, DatasetSplit>,
    identity: K,
    split: DatasetSplit,
) -> Result<(), EvidenceError> {
    match registry.get(&identity) {
        Some(previous) if *previous != split => Err(EvidenceError::LeakageDetected),
        Some(_) => Ok(()),
        None => {
            registry.insert(identity, split);
            Ok(())
        }
    }
}

fn qualification_bytes(receipt: &QualificationReceipt) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"symtropy.road-reality-gap.qualification.v0.1\0");
    push_id(&mut out, &receipt.evidence_id);
    out.extend_from_slice(&receipt.evidence_digest);
    out.extend_from_slice(&receipt.metadata_digest);
    match &receipt.target {
        QualificationTarget::FactualOutcome { action_id } => {
            out.push(1);
            push_id(&mut out, action_id);
        }
        QualificationTarget::ExecutedInterventionOutcome { action_id } => {
            out.push(2);
            push_id(&mut out, action_id);
        }
        QualificationTarget::UnexecutedActionOutcome { action_id } => {
            out.push(3);
            push_id(&mut out, action_id);
        }
        QualificationTarget::FormalConformance { domain_id } => {
            out.push(4);
            push_id(&mut out, domain_id);
        }
        QualificationTarget::HumanLabel { rubric_id } => {
            out.push(5);
            push_id(&mut out, rubric_id);
        }
    }
    out.push(receipt.eligibility as u8);
    out.push(receipt.reason as u8);
    out.extend_from_slice(&receipt.subject_lineage_digest);
    out
}

fn push_option_id(out: &mut Vec<u8>, id: Option<&StableId>) {
    match id {
        Some(value) => {
            out.push(1);
            push_id(out, value);
        }
        None => out.push(0),
    }
}

fn push_id(out: &mut Vec<u8>, id: &StableId) {
    let bytes = id.as_str().as_bytes();
    let length = u32::try_from(bytes.len()).expect("StableId length is bounded");
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

    fn id(value: &str) -> StableId {
        StableId::new(value).expect("fixture ID")
    }

    fn subject() -> SubjectLineage {
        SubjectLineage {
            complete: true,
            generator_ancestry: BTreeSet::from([id("model-under-test-v3")]),
            upstream_source_digests: BTreeSet::from([[7; 32]]),
            parent_content_digests: BTreeSet::from([[6; 32]]),
        }
    }

    fn lineage() -> EvidenceLineage {
        EvidenceLineage {
            complete: true,
            generator_ancestry: BTreeSet::from([id("independent-capture-pipeline-v1")]),
            upstream_source_digests: BTreeSet::from([[8; 32]]),
            parent_content_digests: BTreeSet::from([[9; 32]]),
        }
    }

    fn factual_packet() -> EvidencePacket {
        EvidencePacket {
            evidence_id: id("record-factual-01"),
            content_digest: [1; 32],
            origin: EvidenceOrigin::ObservedFactual,
            claim: EvidenceClaim::FactualObservation,
            use_role: EvidenceUse::SealedTest,
            dataset_id: id("naturalistic-cohort"),
            dataset_version: id("v1"),
            source_system_id: id("instrumented-vehicle"),
            action_id: Some(id("continue")),
            action_realization: ActionRealization::Observed,
            assignment: None,
            measurement_calibration_id: Some(id("sensor-calibration-2026-01")),
            identification: None,
            formal_domain_id: None,
            annotation: None,
            lineage: lineage(),
        }
    }

    fn split_entry(
        record: &str,
        split: DatasetSplit,
        family: &str,
        drive: &str,
        session: &str,
        parent: [u8; 32],
    ) -> SplitEntry {
        SplitEntry {
            record_id: id(record),
            split,
            lineage_complete: true,
            scenario_family_id: id(family),
            drive_id: id(drive),
            capture_session_id: id(session),
            upstream_parent_digests: BTreeSet::from([parent]),
        }
    }

    #[test]
    fn observed_factual_record_qualifies_only_for_its_executed_action() {
        let packet = factual_packet();
        let receipt = packet.qualify_against(
            &subject(),
            QualificationTarget::FactualOutcome {
                action_id: id("continue"),
            },
        );
        assert_eq!(
            receipt.eligibility,
            GroundTruthEligibility::IndependentObservedAnchor
        );
        assert_eq!(
            packet
                .qualify_against(
                    &subject(),
                    QualificationTarget::UnexecutedActionOutcome {
                        action_id: id("yield"),
                    }
                )
                .eligibility,
            GroundTruthEligibility::NotIdentified
        );
        assert!(receipt.integrity_check());
    }

    #[test]
    fn evidence_content_digest_intersection_is_not_independent() {
        let mut packet = factual_packet();
        packet.content_digest = [6; 32];
        let mut model = subject();
        model.parent_content_digests = BTreeSet::from([[6; 32]]);
        let receipt = packet.qualify_against(
            &model,
            QualificationTarget::FactualOutcome {
                action_id: id("continue"),
            },
        );
        assert_eq!(
            receipt.eligibility,
            GroundTruthEligibility::ModelDependentReference
        );
    }

    #[test]
    fn illustrative_record_cannot_be_used_as_ground_truth() {
        let mut packet = factual_packet();
        packet.use_role = EvidenceUse::IllustrativeOnly;
        let receipt = packet.qualify_against(
            &subject(),
            QualificationTarget::FactualOutcome {
                action_id: id("continue"),
            },
        );
        assert_eq!(receipt.eligibility, GroundTruthEligibility::NotIdentified);
        assert_eq!(
            receipt.reason,
            QualificationReason::IllustrativeOnlyCannotBeReference
        );
    }

    #[test]
    fn qualification_receipt_binds_evidence_metadata() {
        let packet = factual_packet();
        let model = subject();
        let receipt = packet.qualify_against(
            &model,
            QualificationTarget::FactualOutcome {
                action_id: id("continue"),
            },
        );
        assert_eq!(receipt.verify_against(&packet, &model), Ok(()));

        let mut mutated = packet;
        mutated.dataset_version = id("v2");
        assert_eq!(
            receipt.verify_against(&mutated, &model),
            Err(EvidenceError::ReceiptMismatch)
        );
    }

    #[test]
    fn same_model_ancestry_rejects_self_validation() {
        let mut packet = factual_packet();
        packet.lineage.generator_ancestry = BTreeSet::from([id("model-under-test-v3")]);
        let receipt = packet.qualify_against(
            &subject(),
            QualificationTarget::FactualOutcome {
                action_id: id("continue"),
            },
        );
        assert_eq!(
            receipt.eligibility,
            GroundTruthEligibility::ModelDependentReference
        );
    }

    #[test]
    fn shared_upstream_source_rejects_nominally_different_pipeline() {
        let mut packet = factual_packet();
        packet.lineage.upstream_source_digests = BTreeSet::from([[7; 32]]);
        let receipt = packet.qualify_against(
            &subject(),
            QualificationTarget::FactualOutcome {
                action_id: id("continue"),
            },
        );
        assert_eq!(
            receipt.eligibility,
            GroundTruthEligibility::ModelDependentReference
        );
        assert_eq!(receipt.reason, QualificationReason::SharedUpstreamSource);
    }

    #[test]
    fn simulator_output_is_never_independent_real_world_ground_truth() {
        let mut packet = factual_packet();
        packet.origin = EvidenceOrigin::SimulatorGenerated;
        packet.claim = EvidenceClaim::SimulatorOutput;
        let receipt = packet.qualify_against(
            &subject(),
            QualificationTarget::FactualOutcome {
                action_id: id("continue"),
            },
        );
        assert_eq!(receipt.eligibility, GroundTruthEligibility::SimulationOnly);
    }

    #[test]
    fn factual_replay_cannot_identify_an_unexecuted_action() {
        let packet = factual_packet();
        let receipt = packet.qualify_against(
            &subject(),
            QualificationTarget::UnexecutedActionOutcome {
                action_id: id("yield"),
            },
        );
        assert_eq!(receipt.eligibility, GroundTruthEligibility::NotIdentified);
        assert_eq!(
            receipt.reason,
            QualificationReason::FactualRecordCannotIdentifyUnexecutedAction
        );
    }

    #[test]
    fn controlled_intervention_requires_action_protocol_and_calibration() {
        let mut packet = factual_packet();
        packet.origin = EvidenceOrigin::ControlledIntervention;
        packet.claim = EvidenceClaim::ControlledOutcome;
        packet.action_realization = ActionRealization::AssignedAndExecuted;
        packet.assignment = Some(AssignmentProtocol {
            protocol_id: id("randomized-closed-course"),
            method: AssignmentMethod::Randomized,
            protocol_digest: [4; 32],
        });
        let receipt = packet.qualify_against(
            &subject(),
            QualificationTarget::UnexecutedActionOutcome {
                action_id: id("continue"),
            },
        );
        assert_eq!(
            receipt.eligibility,
            GroundTruthEligibility::ControlledInterventionOutcome
        );

        packet.assignment = None;
        assert_eq!(
            packet
                .qualify_against(
                    &subject(),
                    QualificationTarget::ExecutedInterventionOutcome {
                        action_id: id("continue"),
                    }
                )
                .eligibility,
            GroundTruthEligibility::Incomplete
        );
    }

    #[test]
    fn natural_experiment_requires_explicit_identification_package() {
        let mut packet = factual_packet();
        packet.origin = EvidenceOrigin::NaturalExperiment;
        packet.claim = EvidenceClaim::NaturalExperimentEstimate;
        packet.action_realization = ActionRealization::AssignedAndExecuted;
        assert_eq!(
            packet
                .qualify_against(
                    &subject(),
                    QualificationTarget::UnexecutedActionOutcome {
                        action_id: id("continue"),
                    }
                )
                .eligibility,
            GroundTruthEligibility::NotIdentified
        );

        packet.identification = Some(IdentificationPackage {
            estimand_id: id("average-treatment-effect"),
            assumptions_digest: [2; 32],
            overlap_evidence_id: id("overlap-check"),
            sensitivity_plan_id: id("sensitivity-v1"),
            preregistration_id: id("preregistered-protocol"),
        });
        assert_eq!(
            packet
                .qualify_against(
                    &subject(),
                    QualificationTarget::UnexecutedActionOutcome {
                        action_id: id("continue"),
                    }
                )
                .eligibility,
            GroundTruthEligibility::QualifiedNaturalExperiment
        );
    }

    #[test]
    fn incomplete_lineage_or_measurement_calibration_never_passes() {
        let mut packet = factual_packet();
        packet.lineage.complete = false;
        assert_eq!(
            packet
                .qualify_against(
                    &subject(),
                    QualificationTarget::FactualOutcome {
                        action_id: id("continue"),
                    }
                )
                .eligibility,
            GroundTruthEligibility::Incomplete
        );

        let mut packet = factual_packet();
        packet.measurement_calibration_id = None;
        assert_eq!(
            packet
                .qualify_against(
                    &subject(),
                    QualificationTarget::FactualOutcome {
                        action_id: id("continue"),
                    }
                )
                .eligibility,
            GroundTruthEligibility::Incomplete
        );
    }

    #[test]
    fn formal_reference_is_scoped_to_matching_domain() {
        let mut packet = factual_packet();
        packet.origin = EvidenceOrigin::FormalReference;
        packet.claim = EvidenceClaim::FormalResult;
        packet.formal_domain_id = Some(id("binary-markov-fixture-v1"));
        assert_eq!(
            packet
                .qualify_against(
                    &subject(),
                    QualificationTarget::FormalConformance {
                        domain_id: id("binary-markov-fixture-v1"),
                    }
                )
                .eligibility,
            GroundTruthEligibility::AnalyticReference
        );
        assert_eq!(
            packet
                .qualify_against(
                    &subject(),
                    QualificationTarget::FormalConformance {
                        domain_id: id("different-domain"),
                    }
                )
                .eligibility,
            GroundTruthEligibility::NotIdentified
        );
    }

    #[test]
    fn human_judgement_requires_rubric_assessor_count_and_disagreement() {
        let mut packet = factual_packet();
        packet.origin = EvidenceOrigin::HumanAdjudication;
        packet.claim = EvidenceClaim::HumanJudgement;
        packet.annotation = Some(AnnotationQuality {
            rubric_id: id("near-miss-rubric-v1"),
            assessor_protocol_id: id("double-blind-review-v1"),
            disagreement_digest: [5; 32],
            assessor_count: 2,
        });
        assert_eq!(
            packet
                .qualify_against(
                    &subject(),
                    QualificationTarget::HumanLabel {
                        rubric_id: id("near-miss-rubric-v1"),
                    }
                )
                .eligibility,
            GroundTruthEligibility::HumanAdjudicationReference
        );
        packet.annotation.as_mut().unwrap().assessor_count = 0;
        assert_eq!(
            packet
                .qualify_against(
                    &subject(),
                    QualificationTarget::HumanLabel {
                        rubric_id: id("near-miss-rubric-v1"),
                    }
                )
                .eligibility,
            GroundTruthEligibility::Incomplete
        );
    }

    #[test]
    fn incompatible_origin_and_claim_pair_is_contaminated() {
        let mut packet = factual_packet();
        packet.origin = EvidenceOrigin::ModelGenerated;
        let receipt = packet.qualify_against(
            &subject(),
            QualificationTarget::FactualOutcome {
                action_id: id("continue"),
            },
        );
        assert_eq!(receipt.eligibility, GroundTruthEligibility::Contaminated);
    }

    #[test]
    fn split_manifest_rejects_scenario_drive_session_and_parent_leakage() {
        for mutate in 0..4 {
            let first = split_entry(
                "record-a",
                DatasetSplit::Fit,
                "family-a",
                "drive-a",
                "session-a",
                [10; 32],
            );
            let mut second = split_entry(
                "record-b",
                DatasetSplit::SealedTest,
                "family-b",
                "drive-b",
                "session-b",
                [11; 32],
            );
            match mutate {
                0 => second.scenario_family_id = first.scenario_family_id.clone(),
                1 => second.drive_id = first.drive_id.clone(),
                2 => second.capture_session_id = first.capture_session_id.clone(),
                3 => second.upstream_parent_digests = first.upstream_parent_digests.clone(),
                _ => unreachable!(),
            }
            let manifest = SplitManifest {
                manifest_id: id("split-manifest"),
                dataset_version: id("v1"),
                entries: vec![first.clone(), second],
            };
            assert_eq!(manifest.validate(), Err(EvidenceError::LeakageDetected));
        }
    }

    #[test]
    fn split_manifest_is_order_independent_and_frozen() {
        let first = SplitManifest {
            manifest_id: id("sealed-v1"),
            dataset_version: id("dataset-v1"),
            entries: vec![
                split_entry(
                    "record-a",
                    DatasetSplit::Fit,
                    "family-a",
                    "drive-a",
                    "session-a",
                    [10; 32],
                ),
                split_entry(
                    "record-b",
                    DatasetSplit::SealedTest,
                    "family-b",
                    "drive-b",
                    "session-b",
                    [11; 32],
                ),
            ],
        };
        let digest = first.digest().unwrap();
        let mut reordered = first.clone();
        reordered.entries.reverse();
        assert_eq!(digest, reordered.digest().unwrap());
        assert_eq!(reordered.verify_frozen(digest), Ok(()));

        reordered.entries[0].split = DatasetSplit::Fit;
        assert_eq!(
            reordered.verify_frozen(digest),
            Err(EvidenceError::LeakageDetected)
        );
    }

    #[test]
    fn duplicate_record_and_incomplete_split_lineage_reject() {
        let entry = split_entry(
            "record-a",
            DatasetSplit::Fit,
            "family-a",
            "drive-a",
            "session-a",
            [10; 32],
        );
        let duplicate = SplitManifest {
            manifest_id: id("split"),
            dataset_version: id("v1"),
            entries: vec![entry.clone(), entry],
        };
        assert_eq!(duplicate.validate(), Err(EvidenceError::DuplicateRecord));

        let mut incomplete_entry = split_entry(
            "record-b",
            DatasetSplit::Fit,
            "family-b",
            "drive-b",
            "session-b",
            [11; 32],
        );
        incomplete_entry.lineage_complete = false;
        let incomplete = SplitManifest {
            manifest_id: id("split"),
            dataset_version: id("v1"),
            entries: vec![incomplete_entry],
        };
        assert_eq!(
            incomplete.validate(),
            Err(EvidenceError::IncompleteSplitLineage)
        );
    }
}
