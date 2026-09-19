//! Provenance-closed future-bearing experience records.
//!
//! This module records that an agent retained an experience. It deliberately does
//! not implement memory storage, retrieval, consolidation, belief revision, or
//! species-specific learning policy.

use crate::{
    AgentId, CausalStamp, ConfidenceQ, EvidenceRef, ExperienceId, PolicyId, ProfileId, SalienceQ,
};

/// Process by which an experience was formed.
///
/// This classifies formation, not semantic world content. The evidence domain is
/// carried separately by [`ExperienceProvenance`].
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ExperienceFormation {
    Perceptual,
    Bodily,
    Communicative,
    Social,
    Inferred,
}

/// Exact admissible evidence and causal coordinate used to form an experience.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ExperienceProvenance {
    evidence: EvidenceRef,
    observed_at: CausalStamp,
}

/// Input parts for checked experience construction.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ExperienceTraceParts {
    pub id: ExperienceId,
    pub subject: AgentId,
    pub provenance: ExperienceProvenance,
    pub formation: ExperienceFormation,
    pub recorded_at: CausalStamp,
    pub salience_at_record: SalienceQ,
    pub confidence_at_record: ConfidenceQ,
    pub policy_id: PolicyId,
    pub profile_id: ProfileId,
}

/// Invalid construction of a future-bearing experience trace.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExperienceTraceError {
    /// Source evidence does not strictly precede the experience record coordinate.
    SourceNotBeforeRecord,
    /// The declared formation class is incompatible with the cited evidence domain.
    IncompatibleFormationEvidence,
    /// An inferred experience attempted to cite itself as its own causal predecessor.
    SelfInference,
}

/// Minimal future-bearing autobiographical record.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ExperienceTrace {
    id: ExperienceId,
    subject: AgentId,
    provenance: ExperienceProvenance,
    formation: ExperienceFormation,
    recorded_at: CausalStamp,
    salience_at_record: SalienceQ,
    confidence_at_record: ConfidenceQ,
    policy_id: PolicyId,
    profile_id: ProfileId,
}

impl ExperienceProvenance {
    #[must_use]
    pub const fn new(evidence: EvidenceRef, observed_at: CausalStamp) -> Self {
        Self {
            evidence,
            observed_at,
        }
    }

    #[must_use]
    pub const fn evidence(self) -> EvidenceRef {
        self.evidence
    }

    #[must_use]
    pub const fn observed_at(self) -> CausalStamp {
        self.observed_at
    }
}

impl ExperienceFormation {
    /// Whether this formation class may directly cite the supplied evidence domain.
    #[must_use]
    pub const fn accepts(self, evidence: EvidenceRef) -> bool {
        matches!(
            (self, evidence),
            (
                Self::Perceptual | Self::Communicative | Self::Social,
                EvidenceRef::Percept(_)
            ) | (Self::Bodily, EvidenceRef::BodilyOutcome(_))
                | (Self::Inferred, EvidenceRef::Experience(_))
        )
    }
}

impl ExperienceTrace {
    pub fn new(parts: ExperienceTraceParts) -> Result<Self, ExperienceTraceError> {
        if !parts
            .provenance
            .observed_at()
            .strictly_precedes(parts.recorded_at)
        {
            return Err(ExperienceTraceError::SourceNotBeforeRecord);
        }

        if !parts.formation.accepts(parts.provenance.evidence()) {
            return Err(ExperienceTraceError::IncompatibleFormationEvidence);
        }

        if parts.formation == ExperienceFormation::Inferred
            && matches!(
                parts.provenance.evidence(),
                EvidenceRef::Experience(prior_experience_id) if prior_experience_id == parts.id
            )
        {
            return Err(ExperienceTraceError::SelfInference);
        }

        Ok(Self {
            id: parts.id,
            subject: parts.subject,
            provenance: parts.provenance,
            formation: parts.formation,
            recorded_at: parts.recorded_at,
            salience_at_record: parts.salience_at_record,
            confidence_at_record: parts.confidence_at_record,
            policy_id: parts.policy_id,
            profile_id: parts.profile_id,
        })
    }

    #[must_use]
    pub const fn id(&self) -> ExperienceId {
        self.id
    }

    #[must_use]
    pub const fn subject(&self) -> AgentId {
        self.subject
    }

    #[must_use]
    pub const fn provenance(&self) -> ExperienceProvenance {
        self.provenance
    }

    #[must_use]
    pub const fn formation(&self) -> ExperienceFormation {
        self.formation
    }

    #[must_use]
    pub const fn recorded_at(&self) -> CausalStamp {
        self.recorded_at
    }

    #[must_use]
    pub const fn salience_at_record(&self) -> SalienceQ {
        self.salience_at_record
    }

    #[must_use]
    pub const fn confidence_at_record(&self) -> ConfidenceQ {
        self.confidence_at_record
    }

    #[must_use]
    pub const fn policy_id(&self) -> PolicyId {
        self.policy_id
    }

    #[must_use]
    pub const fn profile_id(&self) -> ProfileId {
        self.profile_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ActionId, CausalPhase, EmissionId, PerceptId, Tick};

    struct FixtureIds {
        experience: ExperienceId,
        subject: AgentId,
        percept: PerceptId,
        action: ActionId,
        policy: PolicyId,
        profile: ProfileId,
    }

    fn ids() -> FixtureIds {
        FixtureIds {
            experience: ExperienceId::from_bytes([1; 32]),
            subject: AgentId::from_bytes([2; 32]),
            percept: PerceptId::from_bytes([3; 32]),
            action: ActionId::from_bytes([4; 32]),
            policy: PolicyId::from_bytes([5; 32]),
            profile: ProfileId::from_bytes([6; 32]),
        }
    }

    fn stamp(tick: u64, ordinal: u32, phase: CausalPhase) -> CausalStamp {
        CausalStamp::new(Tick::new(tick), ordinal, phase)
    }

    fn parts(
        provenance: ExperienceProvenance,
        formation: ExperienceFormation,
        recorded_at: CausalStamp,
    ) -> ExperienceTraceParts {
        let ids = ids();
        ExperienceTraceParts {
            id: ids.experience,
            subject: ids.subject,
            provenance,
            formation,
            recorded_at,
            salience_at_record: SalienceQ::ZERO,
            confidence_at_record: ConfidenceQ::ZERO,
            policy_id: ids.policy,
            profile_id: ids.profile,
        }
    }

    #[test]
    fn compatible_evidence_domains_are_accepted() {
        let ids = ids();
        let observed = stamp(7, 2, CausalPhase::Sense);
        let recorded = stamp(7, 3, CausalPhase::Experience);
        let prior = ExperienceId::from_bytes([9; 32]);

        let cases = [
            (
                ExperienceFormation::Perceptual,
                EvidenceRef::Percept(ids.percept),
            ),
            (
                ExperienceFormation::Communicative,
                EvidenceRef::Percept(ids.percept),
            ),
            (
                ExperienceFormation::Social,
                EvidenceRef::Percept(ids.percept),
            ),
            (
                ExperienceFormation::Bodily,
                EvidenceRef::BodilyOutcome(ids.action),
            ),
            (
                ExperienceFormation::Inferred,
                EvidenceRef::Experience(prior),
            ),
        ];

        for (formation, evidence) in cases {
            let provenance = ExperienceProvenance::new(evidence, observed);
            let trace = ExperienceTrace::new(parts(provenance, formation, recorded)).unwrap();
            assert_eq!(trace.provenance().evidence(), evidence);
            assert_eq!(trace.formation(), formation);
        }
    }

    #[test]
    fn direct_emission_and_mismatched_domains_fail_closed() {
        let ids = ids();
        let observed = stamp(7, 2, CausalPhase::Sense);
        let recorded = stamp(7, 3, CausalPhase::Experience);
        let emission = EvidenceRef::Emission(EmissionId::from_bytes([8; 32]));
        let bodily_as_percept =
            ExperienceProvenance::new(EvidenceRef::BodilyOutcome(ids.action), observed);

        assert_eq!(
            ExperienceTrace::new(parts(
                ExperienceProvenance::new(emission, observed),
                ExperienceFormation::Perceptual,
                recorded,
            )),
            Err(ExperienceTraceError::IncompatibleFormationEvidence)
        );
        assert_eq!(
            ExperienceTrace::new(parts(
                bodily_as_percept,
                ExperienceFormation::Perceptual,
                recorded,
            )),
            Err(ExperienceTraceError::IncompatibleFormationEvidence)
        );
    }

    #[test]
    fn evidence_must_strictly_precede_the_record_coordinate() {
        let ids = ids();
        let record = stamp(10, 4, CausalPhase::Experience);
        let evidence = EvidenceRef::Percept(ids.percept);

        for observed in [
            stamp(10, 4, CausalPhase::Sense),
            stamp(10, 5, CausalPhase::Sense),
            stamp(11, 0, CausalPhase::Sense),
        ] {
            let provenance = ExperienceProvenance::new(evidence, observed);
            assert_eq!(
                ExperienceTrace::new(parts(provenance, ExperienceFormation::Perceptual, record,)),
                Err(ExperienceTraceError::SourceNotBeforeRecord)
            );
        }

        let earlier_same_tick =
            ExperienceProvenance::new(evidence, stamp(10, 3, CausalPhase::Outcome));
        assert!(
            ExperienceTrace::new(parts(
                earlier_same_tick,
                ExperienceFormation::Perceptual,
                record,
            ))
            .is_ok()
        );
    }

    #[test]
    fn inferred_self_reference_fails_closed() {
        let ids = ids();
        let provenance = ExperienceProvenance::new(
            EvidenceRef::Experience(ids.experience),
            stamp(10, 2, CausalPhase::Interpret),
        );

        assert_eq!(
            ExperienceTrace::new(parts(
                provenance,
                ExperienceFormation::Inferred,
                stamp(10, 3, CausalPhase::Experience),
            )),
            Err(ExperienceTraceError::SelfInference)
        );
    }

    #[test]
    fn trace_retains_exact_lineage_and_record_time_scores() {
        let ids = ids();
        let provenance = ExperienceProvenance::new(
            EvidenceRef::Percept(ids.percept),
            stamp(7, 2, CausalPhase::Sense),
        );
        let recorded = stamp(7, 3, CausalPhase::Experience);
        let trace =
            ExperienceTrace::new(parts(provenance, ExperienceFormation::Perceptual, recorded))
                .unwrap();

        assert_eq!(trace.id(), ids.experience);
        assert_eq!(trace.subject(), ids.subject);
        assert_eq!(trace.provenance(), provenance);
        assert_eq!(trace.recorded_at(), recorded);
        assert_eq!(trace.salience_at_record(), SalienceQ::ZERO);
        assert_eq!(trace.confidence_at_record(), ConfidenceQ::ZERO);
        assert_eq!(trace.policy_id(), ids.policy);
        assert_eq!(trace.profile_id(), ids.profile);
    }
}
