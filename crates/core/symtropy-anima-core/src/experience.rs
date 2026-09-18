//! Provenance-closed future-bearing experience records.
//!
//! This module records that an agent retained an experience. It deliberately does
//! not implement memory storage, retrieval, consolidation, belief revision, or
//! species-specific learning policy.

use crate::{
    ActionId, AgentId, ConfidenceQ, EvidenceRef, ExperienceId, PerceptId, PolicyId, ProfileId,
    SalienceQ, Tick,
};

/// Admissible causal source for a retained experience.
///
/// There is intentionally no world-truth or exact-hidden-state variant.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ExperienceSource {
    DirectPercept {
        percept_id: PerceptId,
        observed_tick: Tick,
    },
    BodilyOutcome {
        action_id: ActionId,
        settled_tick: Tick,
    },
    Communication {
        percept_id: PerceptId,
        observed_tick: Tick,
    },
    SocialObservation {
        percept_id: PerceptId,
        observed_tick: Tick,
    },
    Inference {
        prior_experience_id: ExperienceId,
        derived_from_tick: Tick,
    },
}

/// Input parts for checked experience construction.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ExperienceTraceParts {
    pub id: ExperienceId,
    pub subject: AgentId,
    pub source: ExperienceSource,
    pub recorded_tick: Tick,
    pub salience: SalienceQ,
    pub confidence: ConfidenceQ,
    pub policy_id: PolicyId,
    pub profile_id: ProfileId,
}

/// Invalid construction of a future-bearing experience trace.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExperienceTraceError {
    /// The causal source lies in the future relative to the trace record tick.
    FutureSource,
    /// An inferred experience attempted to cite itself as its own causal predecessor.
    SelfInference,
}

/// Minimal future-bearing autobiographical record.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ExperienceTrace {
    id: ExperienceId,
    subject: AgentId,
    source: ExperienceSource,
    recorded_tick: Tick,
    salience: SalienceQ,
    confidence: ConfidenceQ,
    policy_id: PolicyId,
    profile_id: ProfileId,
}

impl ExperienceSource {
    /// Canonical tick of the source evidence used to form the experience.
    #[must_use]
    pub const fn source_tick(self) -> Tick {
        match self {
            Self::DirectPercept { observed_tick, .. }
            | Self::Communication { observed_tick, .. }
            | Self::SocialObservation { observed_tick, .. } => observed_tick,
            Self::BodilyOutcome { settled_tick, .. } => settled_tick,
            Self::Inference {
                derived_from_tick, ..
            } => derived_from_tick,
        }
    }

    /// Typed admissible evidence reference retained by this source.
    #[must_use]
    pub const fn evidence_ref(self) -> EvidenceRef {
        match self {
            Self::DirectPercept { percept_id, .. }
            | Self::Communication { percept_id, .. }
            | Self::SocialObservation { percept_id, .. } => EvidenceRef::Percept(percept_id),
            Self::BodilyOutcome { action_id, .. } => EvidenceRef::BodilyOutcome(action_id),
            Self::Inference {
                prior_experience_id,
                ..
            } => EvidenceRef::Experience(prior_experience_id),
        }
    }
}

impl ExperienceTrace {
    pub fn new(parts: ExperienceTraceParts) -> Result<Self, ExperienceTraceError> {
        if parts.source.source_tick() > parts.recorded_tick {
            return Err(ExperienceTraceError::FutureSource);
        }

        if let ExperienceSource::Inference {
            prior_experience_id,
            ..
        } = parts.source
        {
            if prior_experience_id == parts.id {
                return Err(ExperienceTraceError::SelfInference);
            }
        }

        Ok(Self {
            id: parts.id,
            subject: parts.subject,
            source: parts.source,
            recorded_tick: parts.recorded_tick,
            salience: parts.salience,
            confidence: parts.confidence,
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
    pub const fn source(&self) -> ExperienceSource {
        self.source
    }

    #[must_use]
    pub const fn recorded_tick(&self) -> Tick {
        self.recorded_tick
    }

    #[must_use]
    pub const fn salience(&self) -> SalienceQ {
        self.salience
    }

    #[must_use]
    pub const fn confidence(&self) -> ConfidenceQ {
        self.confidence
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

    fn ids() -> (
        ExperienceId,
        AgentId,
        PerceptId,
        ActionId,
        PolicyId,
        ProfileId,
    ) {
        (
            ExperienceId::from_bytes([1; 32]),
            AgentId::from_bytes([2; 32]),
            PerceptId::from_bytes([3; 32]),
            ActionId::from_bytes([4; 32]),
            PolicyId::from_bytes([5; 32]),
            ProfileId::from_bytes([6; 32]),
        )
    }

    fn parts(source: ExperienceSource, recorded_tick: Tick) -> ExperienceTraceParts {
        let (id, subject, _, _, policy_id, profile_id) = ids();
        ExperienceTraceParts {
            id,
            subject,
            source,
            recorded_tick,
            salience: SalienceQ::ZERO,
            confidence: ConfidenceQ::ZERO,
            policy_id,
            profile_id,
        }
    }

    #[test]
    fn every_source_maps_to_admissible_evidence() {
        let (experience_id, _, percept_id, action_id, _, _) = ids();
        let prior = ExperienceId::from_bytes([9; 32]);

        let sources = [
            ExperienceSource::DirectPercept {
                percept_id,
                observed_tick: Tick::new(1),
            },
            ExperienceSource::BodilyOutcome {
                action_id,
                settled_tick: Tick::new(1),
            },
            ExperienceSource::Communication {
                percept_id,
                observed_tick: Tick::new(1),
            },
            ExperienceSource::SocialObservation {
                percept_id,
                observed_tick: Tick::new(1),
            },
            ExperienceSource::Inference {
                prior_experience_id: prior,
                derived_from_tick: Tick::new(1),
            },
        ];

        assert_eq!(sources[0].evidence_ref(), EvidenceRef::Percept(percept_id));
        assert_eq!(sources[1].evidence_ref(), EvidenceRef::BodilyOutcome(action_id));
        assert_eq!(sources[2].evidence_ref(), EvidenceRef::Percept(percept_id));
        assert_eq!(sources[3].evidence_ref(), EvidenceRef::Percept(percept_id));
        assert_eq!(sources[4].evidence_ref(), EvidenceRef::Experience(prior));
        assert_ne!(sources[4].evidence_ref(), EvidenceRef::Experience(experience_id));
    }

    #[test]
    fn future_dated_sources_fail_closed() {
        let (_, _, percept_id, _, _, _) = ids();
        let source = ExperienceSource::DirectPercept {
            percept_id,
            observed_tick: Tick::new(11),
        };

        assert_eq!(
            ExperienceTrace::new(parts(source, Tick::new(10))),
            Err(ExperienceTraceError::FutureSource)
        );
    }

    #[test]
    fn self_referential_inference_fails_closed() {
        let (id, _, _, _, _, _) = ids();
        let source = ExperienceSource::Inference {
            prior_experience_id: id,
            derived_from_tick: Tick::new(10),
        };

        assert_eq!(
            ExperienceTrace::new(parts(source, Tick::new(10))),
            Err(ExperienceTraceError::SelfInference)
        );
    }

    #[test]
    fn trace_retains_exact_lineage_and_allows_measured_zero_scores() {
        let (id, subject, percept_id, _, policy_id, profile_id) = ids();
        let source = ExperienceSource::DirectPercept {
            percept_id,
            observed_tick: Tick::new(7),
        };

        let trace = ExperienceTrace::new(parts(source, Tick::new(7))).unwrap();

        assert_eq!(trace.id(), id);
        assert_eq!(trace.subject(), subject);
        assert_eq!(trace.source(), source);
        assert_eq!(trace.recorded_tick(), Tick::new(7));
        assert_eq!(trace.salience(), SalienceQ::ZERO);
        assert_eq!(trace.confidence(), ConfidenceQ::ZERO);
        assert_eq!(trace.policy_id(), policy_id);
        assert_eq!(trace.profile_id(), profile_id);
    }
}
