//! Typed references to admissible causal evidence.

use crate::{ActionId, EmissionId, ExperienceId, PerceptId};

/// Reference to evidence that may support a later canonical inference or decision.
///
/// There is intentionally no `WorldTruth`, Bevy entity, raw pointer, or free-form
/// string variant. Higher layers may add semantic interpretation only through
/// qualified transformations that retain these causal references.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum EvidenceRef {
    /// Physical/communicative emission occurrence.
    Emission(EmissionId),
    /// Receptor-legitimate percept occurrence.
    Percept(PerceptId),
    /// Future-bearing retained experience.
    Experience(ExperienceId),
    /// Settled bodily/action outcome.
    BodilyOutcome(ActionId),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evidence_refs_preserve_domain_identity() {
        let bytes = [3; 32];
        let percept = EvidenceRef::Percept(PerceptId::from_bytes(bytes));
        let experience = EvidenceRef::Experience(ExperienceId::from_bytes(bytes));
        assert_ne!(percept, experience);
    }
}
