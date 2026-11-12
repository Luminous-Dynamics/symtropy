//! Small, dependency-free reference for evidence-bound component condition transitions.
//!
//! This crate verifies identity, revision, event, evidence-class, unit, and time
//! continuity. It is not a damage solver, material ledger, structural assessor,
//! or maintenance authority. A SHA-256 string is syntax-checked only: this crate
//! does not authenticate the referenced bytes or establish their physical truth.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

/// Exact physical subject to which condition evidence applies.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ComponentIdentity {
    /// Physical article / assembly identity.
    pub physical_article_id: String,
    /// Unique physical component instance; replacement creates a new instance ID.
    pub component_instance_id: String,
    /// Monotonically increasing configuration generation for this component lineage.
    pub configuration_revision: u64,
    /// Material lot / heat / batch identity. Unknown is not an empty substitute.
    pub material_lot_id: String,
}

impl ComponentIdentity {
    fn validate(&self) -> Result<(), TransitionError> {
        if self.physical_article_id.trim().is_empty()
            || self.component_instance_id.trim().is_empty()
            || self.material_lot_id.trim().is_empty()
        {
            return Err(TransitionError::EmptyIdentity);
        }
        Ok(())
    }
}

/// Evidence class is deliberately distinct from model confidence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceClass {
    Synthetic,
    ModelEstimate,
    Measured,
}

/// A reference to an upstream input or output artifact.
///
/// The digest is checked for 64 hexadecimal characters. Authenticity, access
/// rights, calibration validity, and truth of the source remain external gates.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceRef {
    pub evidence_id: String,
    pub sha256: String,
    pub class: EvidenceClass,
}

impl EvidenceRef {
    fn validate(&self) -> Result<(), TransitionError> {
        if self.evidence_id.trim().is_empty() {
            return Err(TransitionError::EmptyEvidenceId);
        }
        if self.sha256.len() != 64 || !self.sha256.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(TransitionError::InvalidDigest);
        }
        Ok(())
    }
}

/// Mechanism-specific condition axes. These are not interchangeable health units.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DamageMechanism {
    /// Dimensionless linear Miner accumulation; not a physical failure verdict.
    FatigueAccumulation,
    /// Crack length in metres.
    CrackGrowth,
    /// Material loss depth in metres.
    Corrosion,
    /// Removed / worn material volume in cubic metres.
    Wear,
}

/// Typed quantity prevents comparing unrelated damage axes as one scalar.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ConditionQuantity {
    MinerDamage(f64),
    CrackLengthMetres(f64),
    CorrosionLossMetres(f64),
    WearVolumeCubicMetres(f64),
}

impl ConditionQuantity {
    fn value(self) -> f64 {
        match self {
            Self::MinerDamage(v)
            | Self::CrackLengthMetres(v)
            | Self::CorrosionLossMetres(v)
            | Self::WearVolumeCubicMetres(v) => v,
        }
    }

    fn matches(self, mechanism: DamageMechanism) -> bool {
        matches!(
            (self, mechanism),
            (Self::MinerDamage(_), DamageMechanism::FatigueAccumulation)
                | (Self::CrackLengthMetres(_), DamageMechanism::CrackGrowth)
                | (Self::CorrosionLossMetres(_), DamageMechanism::Corrosion)
                | (Self::WearVolumeCubicMetres(_), DamageMechanism::Wear)
        )
    }
}

/// Whether the claim is usable inside its declared model/evidence envelope.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConditionDisposition {
    InDomain,
    OutOfDomain,
    Incomplete,
    Inconclusive,
}

/// One append-only condition claim. Missing uncertainty remains explicitly None.
#[derive(Clone, Debug, PartialEq)]
pub struct ConditionClaim {
    pub claim_id: String,
    pub mechanism: DamageMechanism,
    pub disposition: ConditionDisposition,
    /// A value exists only for an in-domain claim. Non-success dispositions do
    /// not turn an unknown value into zero.
    pub quantity: Option<ConditionQuantity>,
    /// Absolute uncertainty in the same unit as quantity, if established.
    pub uncertainty: Option<f64>,
    pub evidence_id: String,
    pub evidence_class: EvidenceClass,
    /// Required for a model estimate; never inferred from the number itself.
    pub model_profile_id: Option<String>,
}

impl ConditionClaim {
    fn validate(&self, evidence: &BTreeMap<&str, &EvidenceRef>) -> Result<(), TransitionError> {
        if self.claim_id.trim().is_empty() || self.evidence_id.trim().is_empty() {
            return Err(TransitionError::EmptyClaimIdentity);
        }

        let source = evidence
            .get(self.evidence_id.as_str())
            .ok_or(TransitionError::EvidenceReferenceMissing)?;

        if source.class != self.evidence_class {
            return Err(TransitionError::EvidenceClassMismatch);
        }

        if self.evidence_class == EvidenceClass::ModelEstimate
            && self
                .model_profile_id
                .as_deref()
                .is_none_or(|profile| profile.trim().is_empty())
        {
            return Err(TransitionError::MissingModelProfile);
        }

        match (self.disposition, self.quantity) {
            (ConditionDisposition::InDomain, Some(quantity)) => {
                if !quantity.value().is_finite() || quantity.value() < 0.0 {
                    return Err(TransitionError::InvalidQuantity);
                }
                if !quantity.matches(self.mechanism) {
                    return Err(TransitionError::MechanismQuantityMismatch);
                }
            }
            (ConditionDisposition::InDomain, None) => {
                return Err(TransitionError::MissingQuantity);
            }
            (_, Some(_)) => return Err(TransitionError::QuantityOnNonSuccess),
            (_, None) => {}
        }

        if self
            .uncertainty
            .is_some_and(|value| !value.is_finite() || value < 0.0)
        {
            return Err(TransitionError::InvalidUncertainty);
        }

        Ok(())
    }
}

/// Two time roles on one declared clock. Cross-clock conversion requires an
/// upstream transform receipt and is intentionally not guessed here.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventTime {
    pub clock_id: String,
    pub event_time_ns: u64,
    pub recorded_time_ns: u64,
}

impl EventTime {
    fn validate(&self, expected_clock: &str) -> Result<(), TransitionError> {
        if self.clock_id.trim().is_empty() {
            return Err(TransitionError::EmptyClockId);
        }
        if self.clock_id != expected_clock {
            return Err(TransitionError::ClockMismatch);
        }
        if self.event_time_ns > self.recorded_time_ns {
            return Err(TransitionError::InvalidTimeOrder);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MaintenanceAction {
    Inspect,
    Repair,
    Reinforce,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TransitionKind {
    Assessment {
        claims: Vec<ConditionClaim>,
    },
    Maintenance {
        action: MaintenanceAction,
        /// Post-action observations are appended; prior damage/usage is not erased.
        postcondition_claims: Vec<ConditionClaim>,
    },
    Replacement {
        successor: ComponentIdentity,
        /// Initial condition evidence belongs to the replacement instance.
        initial_claims: Vec<ConditionClaim>,
    },
}

/// Revision-bound input event. It is consumed only after every check succeeds.
#[derive(Clone, Debug, PartialEq)]
pub struct ConditionEvent {
    pub event_id: String,
    pub expected_revision: u64,
    pub subject: ComponentIdentity,
    pub time: EventTime,
    pub sources: Vec<EvidenceRef>,
    pub kind: TransitionKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransitionKindLabel {
    Assessment,
    Inspect,
    Repair,
    Reinforce,
    Replacement,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ConditionRecord {
    pub subject: ComponentIdentity,
    pub event_id: String,
    pub claim: ConditionClaim,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TransitionRecord {
    pub event_id: String,
    pub revision: u64,
    pub subject_before: ComponentIdentity,
    pub subject_after: ComponentIdentity,
    pub clock_id: String,
    pub event_time_ns: u64,
    pub recorded_time_ns: u64,
    pub kind: TransitionKindLabel,
}

/// Append-only reference ledger. It stores claims and lineage; it does not
/// calculate structural capacity, damage evolution, or reuse fitness.
#[derive(Clone, Debug, PartialEq)]
pub struct ConditionLedger {
    current_subject: ComponentIdentity,
    clock_id: String,
    revision: u64,
    last_recorded_time_ns: Option<u64>,
    seen_event_ids: BTreeSet<String>,
    seen_claim_ids: BTreeSet<String>,
    history: Vec<ConditionRecord>,
    transitions: Vec<TransitionRecord>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TransitionReceipt {
    pub revision: u64,
    pub event_id: String,
    pub current_subject: ComponentIdentity,
    pub appended_claims: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TransitionError {
    EmptyIdentity,
    EmptyClockId,
    ClockMismatch,
    ClockRegression,
    InvalidTimeOrder,
    EmptyEventId,
    DuplicateEventId,
    EmptyEvidenceId,
    InvalidDigest,
    EmptySources,
    DuplicateSourceId,
    EmptyClaimIdentity,
    DuplicateClaimId,
    EvidenceReferenceMissing,
    EvidenceClassMismatch,
    MissingModelProfile,
    MissingQuantity,
    QuantityOnNonSuccess,
    InvalidQuantity,
    MechanismQuantityMismatch,
    InvalidUncertainty,
    EmptyClaims,
    StaleRevision,
    SubjectMismatch,
    InvalidSuccessor,
    MissingMeasuredPostcondition,
    RevisionOverflow,
}

impl fmt::Display for TransitionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for TransitionError {}

impl ConditionLedger {
    pub fn new(
        subject: ComponentIdentity,
        clock_id: impl Into<String>,
    ) -> Result<Self, TransitionError> {
        subject.validate()?;
        let clock_id = clock_id.into();
        if clock_id.trim().is_empty() {
            return Err(TransitionError::EmptyClockId);
        }
        Ok(Self {
            current_subject: subject,
            clock_id,
            revision: 0,
            last_recorded_time_ns: None,
            seen_event_ids: BTreeSet::new(),
            seen_claim_ids: BTreeSet::new(),
            history: Vec::new(),
            transitions: Vec::new(),
        })
    }

    pub fn current_subject(&self) -> &ComponentIdentity {
        &self.current_subject
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn history(&self) -> &[ConditionRecord] {
        &self.history
    }

    pub fn transitions(&self) -> &[TransitionRecord] {
        &self.transitions
    }

    /// Validate completely before mutation. Any error leaves the ledger
    /// byte-for-byte equivalent at the Rust value level to its prior state.
    pub fn apply(&mut self, event: ConditionEvent) -> Result<TransitionReceipt, TransitionError> {
        if event.event_id.trim().is_empty() {
            return Err(TransitionError::EmptyEventId);
        }
        if self.seen_event_ids.contains(&event.event_id) {
            return Err(TransitionError::DuplicateEventId);
        }
        if event.expected_revision != self.revision {
            return Err(TransitionError::StaleRevision);
        }
        if event.subject != self.current_subject {
            return Err(TransitionError::SubjectMismatch);
        }

        event.time.validate(&self.clock_id)?;
        if self
            .last_recorded_time_ns
            .is_some_and(|last| event.time.recorded_time_ns < last)
        {
            return Err(TransitionError::ClockRegression);
        }

        let evidence = validate_sources(&event.sources)?;
        let (subject_after, claims, kind_label) =
            validate_kind(&event.kind, &self.current_subject, &evidence)?;

        for claim in &claims {
            if self.seen_claim_ids.contains(&claim.claim_id) {
                return Err(TransitionError::DuplicateClaimId);
            }
        }

        let next_revision = self
            .revision
            .checked_add(1)
            .ok_or(TransitionError::RevisionOverflow)?;

        let appended_claims = claims.len();
        for claim in claims {
            self.seen_claim_ids.insert(claim.claim_id.clone());
            self.history.push(ConditionRecord {
                subject: subject_for_claim(&event.kind, &event.subject, &subject_after, &claim),
                event_id: event.event_id.clone(),
                claim,
            });
        }

        self.seen_event_ids.insert(event.event_id.clone());
        self.transitions.push(TransitionRecord {
            event_id: event.event_id.clone(),
            revision: next_revision,
            subject_before: self.current_subject.clone(),
            subject_after: subject_after.clone(),
            clock_id: event.time.clock_id,
            event_time_ns: event.time.event_time_ns,
            recorded_time_ns: event.time.recorded_time_ns,
            kind: kind_label,
        });
        self.current_subject = subject_after.clone();
        self.revision = next_revision;
        self.last_recorded_time_ns = Some(event.time.recorded_time_ns);

        Ok(TransitionReceipt {
            revision: self.revision,
            event_id: event.event_id,
            current_subject: subject_after,
            appended_claims,
        })
    }
}

fn validate_sources(
    sources: &[EvidenceRef],
) -> Result<BTreeMap<&str, &EvidenceRef>, TransitionError> {
    if sources.is_empty() {
        return Err(TransitionError::EmptySources);
    }
    let mut evidence = BTreeMap::new();
    for source in sources {
        source.validate()?;
        if evidence
            .insert(source.evidence_id.as_str(), source)
            .is_some()
        {
            return Err(TransitionError::DuplicateSourceId);
        }
    }
    Ok(evidence)
}

fn validate_claims(
    claims: &[ConditionClaim],
    evidence: &BTreeMap<&str, &EvidenceRef>,
) -> Result<(), TransitionError> {
    if claims.is_empty() {
        return Err(TransitionError::EmptyClaims);
    }
    let mut ids = BTreeSet::new();
    for claim in claims {
        if !ids.insert(claim.claim_id.as_str()) {
            return Err(TransitionError::DuplicateClaimId);
        }
        claim.validate(evidence)?;
    }
    Ok(())
}

fn validate_kind(
    kind: &TransitionKind,
    current: &ComponentIdentity,
    evidence: &BTreeMap<&str, &EvidenceRef>,
) -> Result<(ComponentIdentity, Vec<ConditionClaim>, TransitionKindLabel), TransitionError> {
    match kind {
        TransitionKind::Assessment { claims } => {
            validate_claims(claims, evidence)?;
            Ok((
                current.clone(),
                claims.clone(),
                TransitionKindLabel::Assessment,
            ))
        }
        TransitionKind::Maintenance {
            action,
            postcondition_claims,
        } => {
            validate_claims(postcondition_claims, evidence)?;
            if !postcondition_claims
                .iter()
                .any(|claim| claim.evidence_class == EvidenceClass::Measured)
            {
                return Err(TransitionError::MissingMeasuredPostcondition);
            }
            let label = match action {
                MaintenanceAction::Inspect => TransitionKindLabel::Inspect,
                MaintenanceAction::Repair => TransitionKindLabel::Repair,
                MaintenanceAction::Reinforce => TransitionKindLabel::Reinforce,
            };
            Ok((current.clone(), postcondition_claims.clone(), label))
        }
        TransitionKind::Replacement {
            successor,
            initial_claims,
        } => {
            successor.validate()?;
            if successor.physical_article_id != current.physical_article_id
                || successor.component_instance_id == current.component_instance_id
                || successor.configuration_revision <= current.configuration_revision
            {
                return Err(TransitionError::InvalidSuccessor);
            }
            validate_claims(initial_claims, evidence)?;
            if !initial_claims
                .iter()
                .any(|claim| claim.evidence_class == EvidenceClass::Measured)
            {
                return Err(TransitionError::MissingMeasuredPostcondition);
            }
            Ok((
                successor.clone(),
                initial_claims.clone(),
                TransitionKindLabel::Replacement,
            ))
        }
    }
}

fn subject_for_claim(
    kind: &TransitionKind,
    before: &ComponentIdentity,
    after: &ComponentIdentity,
    _claim: &ConditionClaim,
) -> ComponentIdentity {
    match kind {
        TransitionKind::Replacement { .. } => after.clone(),
        TransitionKind::Assessment { .. } | TransitionKind::Maintenance { .. } => before.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn subject() -> ComponentIdentity {
        ComponentIdentity {
            physical_article_id: "article-001".into(),
            component_instance_id: "beam-001-generation-1".into(),
            configuration_revision: 1,
            material_lot_id: "heat-42".into(),
        }
    }

    fn source(id: &str, class: EvidenceClass) -> EvidenceRef {
        EvidenceRef {
            evidence_id: id.into(),
            sha256: "a".repeat(64),
            class,
        }
    }

    fn model_claim(id: &str, evidence_id: &str) -> ConditionClaim {
        ConditionClaim {
            claim_id: id.into(),
            mechanism: DamageMechanism::FatigueAccumulation,
            disposition: ConditionDisposition::InDomain,
            quantity: Some(ConditionQuantity::MinerDamage(0.25)),
            uncertainty: Some(0.02),
            evidence_id: evidence_id.into(),
            evidence_class: EvidenceClass::ModelEstimate,
            model_profile_id: Some("sn-miner-profile-v1".into()),
        }
    }

    fn event(
        event_id: &str,
        revision: u64,
        subject: ComponentIdentity,
        recorded_time_ns: u64,
        kind: TransitionKind,
        sources: Vec<EvidenceRef>,
    ) -> ConditionEvent {
        ConditionEvent {
            event_id: event_id.into(),
            expected_revision: revision,
            subject,
            time: EventTime {
                clock_id: "test-monotonic-clock".into(),
                event_time_ns: recorded_time_ns.saturating_sub(1),
                recorded_time_ns,
            },
            sources,
            kind,
        }
    }

    fn ledger() -> ConditionLedger {
        ConditionLedger::new(subject(), "test-monotonic-clock").unwrap()
    }

    #[test]
    fn accepts_in_domain_damage_claim_with_exact_subject_and_evidence() {
        let mut ledger = ledger();
        let receipt = ledger
            .apply(event(
                "evt-1",
                0,
                subject(),
                10,
                TransitionKind::Assessment {
                    claims: vec![model_claim("claim-1", "damage-output")],
                },
                vec![
                    source("load-history", EvidenceClass::Measured),
                    source("damage-output", EvidenceClass::ModelEstimate),
                ],
            ))
            .unwrap();

        assert_eq!(receipt.revision, 1);
        assert_eq!(receipt.appended_claims, 1);
        assert_eq!(ledger.history().len(), 1);
        assert_eq!(ledger.history()[0].subject, subject());
    }

    #[test]
    fn duplicate_event_is_rejected_without_mutation() {
        let mut ledger = ledger();
        let e = event(
            "evt-1",
            0,
            subject(),
            10,
            TransitionKind::Assessment {
                claims: vec![model_claim("claim-1", "damage-output")],
            },
            vec![source("damage-output", EvidenceClass::ModelEstimate)],
        );
        ledger.apply(e.clone()).unwrap();
        let before = ledger.clone();
        assert_eq!(ledger.apply(e), Err(TransitionError::DuplicateEventId));
        assert_eq!(ledger, before);
    }

    #[test]
    fn stale_revision_is_rejected_without_mutation() {
        let mut ledger = ledger();
        ledger
            .apply(event(
                "evt-1",
                0,
                subject(),
                10,
                TransitionKind::Assessment {
                    claims: vec![model_claim("claim-1", "damage-output-1")],
                },
                vec![source("damage-output-1", EvidenceClass::ModelEstimate)],
            ))
            .unwrap();
        let before = ledger.clone();
        let result = ledger.apply(event(
            "evt-2",
            0,
            subject(),
            11,
            TransitionKind::Assessment {
                claims: vec![model_claim("claim-2", "damage-output-2")],
            },
            vec![source("damage-output-2", EvidenceClass::ModelEstimate)],
        ));
        assert_eq!(result, Err(TransitionError::StaleRevision));
        assert_eq!(ledger, before);
    }

    #[test]
    fn wrong_component_subject_is_rejected_without_mutation() {
        let mut ledger = ledger();
        let mut wrong = subject();
        wrong.component_instance_id = "different-component".into();
        let before = ledger.clone();
        let result = ledger.apply(event(
            "evt-1",
            0,
            wrong,
            10,
            TransitionKind::Assessment {
                claims: vec![model_claim("claim-1", "damage-output")],
            },
            vec![source("damage-output", EvidenceClass::ModelEstimate)],
        ));
        assert_eq!(result, Err(TransitionError::SubjectMismatch));
        assert_eq!(ledger, before);
    }

    #[test]
    fn measured_claim_cannot_reference_model_estimate_evidence() {
        let mut ledger = ledger();
        let mut claim = model_claim("claim-1", "damage-output");
        claim.evidence_class = EvidenceClass::Measured;
        claim.model_profile_id = None;
        let before = ledger.clone();
        let result = ledger.apply(event(
            "evt-1",
            0,
            subject(),
            10,
            TransitionKind::Assessment {
                claims: vec![claim],
            },
            vec![source("damage-output", EvidenceClass::ModelEstimate)],
        ));
        assert_eq!(result, Err(TransitionError::EvidenceClassMismatch));
        assert_eq!(ledger, before);
    }

    #[test]
    fn model_estimate_requires_declared_profile() {
        let mut ledger = ledger();
        let mut claim = model_claim("claim-1", "damage-output");
        claim.model_profile_id = None;
        let result = ledger.apply(event(
            "evt-1",
            0,
            subject(),
            10,
            TransitionKind::Assessment {
                claims: vec![claim],
            },
            vec![source("damage-output", EvidenceClass::ModelEstimate)],
        ));
        assert_eq!(result, Err(TransitionError::MissingModelProfile));
        assert_eq!(ledger.revision(), 0);
    }

    #[test]
    fn non_domain_claim_remains_unquantified_not_zero() {
        let mut ledger = ledger();
        let claim = ConditionClaim {
            claim_id: "claim-1".into(),
            mechanism: DamageMechanism::CrackGrowth,
            disposition: ConditionDisposition::OutOfDomain,
            quantity: None,
            uncertainty: None,
            evidence_id: "crack-evaluation".into(),
            evidence_class: EvidenceClass::ModelEstimate,
            model_profile_id: Some("paris-profile-v1".into()),
        };
        ledger
            .apply(event(
                "evt-1",
                0,
                subject(),
                10,
                TransitionKind::Assessment {
                    claims: vec![claim],
                },
                vec![source("crack-evaluation", EvidenceClass::ModelEstimate)],
            ))
            .unwrap();
        assert_eq!(
            ledger.history()[0].claim.disposition,
            ConditionDisposition::OutOfDomain
        );
        assert_eq!(ledger.history()[0].claim.quantity, None);
    }

    #[test]
    fn repair_appends_postcondition_and_preserves_damage_history() {
        let mut ledger = ledger();
        ledger
            .apply(event(
                "evt-1",
                0,
                subject(),
                10,
                TransitionKind::Assessment {
                    claims: vec![model_claim("before-repair", "damage-before")],
                },
                vec![source("damage-before", EvidenceClass::ModelEstimate)],
            ))
            .unwrap();

        let post = ConditionClaim {
            claim_id: "after-repair".into(),
            mechanism: DamageMechanism::CrackGrowth,
            disposition: ConditionDisposition::InDomain,
            quantity: Some(ConditionQuantity::CrackLengthMetres(0.001)),
            uncertainty: Some(0.0001),
            evidence_id: "post-repair-inspection".into(),
            evidence_class: EvidenceClass::Measured,
            model_profile_id: None,
        };
        ledger
            .apply(event(
                "evt-2",
                1,
                subject(),
                20,
                TransitionKind::Maintenance {
                    action: MaintenanceAction::Repair,
                    postcondition_claims: vec![post],
                },
                vec![source("post-repair-inspection", EvidenceClass::Measured)],
            ))
            .unwrap();

        assert_eq!(ledger.history().len(), 2);
        assert_eq!(ledger.history()[0].claim.claim_id, "before-repair");
        assert_eq!(ledger.history()[1].claim.claim_id, "after-repair");
        assert_eq!(ledger.transitions()[1].kind, TransitionKindLabel::Repair);
    }

    #[test]
    fn model_only_repair_cannot_become_an_executed_transition() {
        let mut ledger = ledger();
        let before = ledger.clone();
        let result = ledger.apply(event(
            "evt-repair",
            0,
            subject(),
            10,
            TransitionKind::Maintenance {
                action: MaintenanceAction::Repair,
                postcondition_claims: vec![model_claim("repair-model-only", "repair-model-output")],
            },
            vec![source("repair-model-output", EvidenceClass::ModelEstimate)],
        ));
        assert_eq!(result, Err(TransitionError::MissingMeasuredPostcondition));
        assert_eq!(ledger, before);
    }

    #[test]
    fn model_only_replacement_cannot_create_a_verified_new_generation() {
        let mut ledger = ledger();
        let successor = ComponentIdentity {
            physical_article_id: "article-001".into(),
            component_instance_id: "beam-001-generation-2".into(),
            configuration_revision: 2,
            material_lot_id: "heat-99".into(),
        };
        let before = ledger.clone();
        let result = ledger.apply(event(
            "evt-replace",
            0,
            subject(),
            10,
            TransitionKind::Replacement {
                successor,
                initial_claims: vec![model_claim(
                    "replacement-model-only",
                    "replacement-model-output",
                )],
            },
            vec![source(
                "replacement-model-output",
                EvidenceClass::ModelEstimate,
            )],
        ));
        assert_eq!(result, Err(TransitionError::MissingMeasuredPostcondition));
        assert_eq!(ledger, before);
    }

    #[test]
    fn replacement_creates_new_instance_and_keeps_predecessor_history() {
        let mut ledger = ledger();
        ledger
            .apply(event(
                "evt-1",
                0,
                subject(),
                10,
                TransitionKind::Assessment {
                    claims: vec![model_claim("old-claim", "old-damage-output")],
                },
                vec![source("old-damage-output", EvidenceClass::ModelEstimate)],
            ))
            .unwrap();

        let successor = ComponentIdentity {
            physical_article_id: "article-001".into(),
            component_instance_id: "beam-001-generation-2".into(),
            configuration_revision: 2,
            material_lot_id: "heat-99".into(),
        };
        let initial = ConditionClaim {
            claim_id: "new-instance-inspection".into(),
            mechanism: DamageMechanism::CrackGrowth,
            disposition: ConditionDisposition::InDomain,
            quantity: Some(ConditionQuantity::CrackLengthMetres(0.0)),
            uncertainty: Some(0.00001),
            evidence_id: "incoming-inspection".into(),
            evidence_class: EvidenceClass::Measured,
            model_profile_id: None,
        };

        ledger
            .apply(event(
                "evt-2",
                1,
                subject(),
                20,
                TransitionKind::Replacement {
                    successor: successor.clone(),
                    initial_claims: vec![initial],
                },
                vec![source("incoming-inspection", EvidenceClass::Measured)],
            ))
            .unwrap();

        assert_eq!(ledger.current_subject(), &successor);
        assert_eq!(ledger.history().len(), 2);
        assert_eq!(ledger.history()[0].subject, subject());
        assert_eq!(ledger.history()[1].subject, successor);
    }

    #[test]
    fn invalid_quantity_unit_pair_is_rejected() {
        let mut ledger = ledger();
        let mut claim = model_claim("claim-1", "damage-output");
        claim.mechanism = DamageMechanism::CrackGrowth;
        let before = ledger.clone();
        let result = ledger.apply(event(
            "evt-1",
            0,
            subject(),
            10,
            TransitionKind::Assessment {
                claims: vec![claim],
            },
            vec![source("damage-output", EvidenceClass::ModelEstimate)],
        ));
        assert_eq!(result, Err(TransitionError::MechanismQuantityMismatch));
        assert_eq!(ledger, before);
    }

    #[test]
    fn invalid_digest_is_rejected_without_mutation() {
        let mut ledger = ledger();
        let mut bad = source("damage-output", EvidenceClass::ModelEstimate);
        bad.sha256 = "not-a-digest".into();
        let before = ledger.clone();
        let result = ledger.apply(event(
            "evt-1",
            0,
            subject(),
            10,
            TransitionKind::Assessment {
                claims: vec![model_claim("claim-1", "damage-output")],
            },
            vec![bad],
        ));
        assert_eq!(result, Err(TransitionError::InvalidDigest));
        assert_eq!(ledger, before);
    }

    #[test]
    fn recorded_time_regression_is_rejected_without_mutation() {
        let mut ledger = ledger();
        ledger
            .apply(event(
                "evt-1",
                0,
                subject(),
                20,
                TransitionKind::Assessment {
                    claims: vec![model_claim("claim-1", "damage-output-1")],
                },
                vec![source("damage-output-1", EvidenceClass::ModelEstimate)],
            ))
            .unwrap();
        let before = ledger.clone();
        let result = ledger.apply(event(
            "evt-2",
            1,
            subject(),
            19,
            TransitionKind::Assessment {
                claims: vec![model_claim("claim-2", "damage-output-2")],
            },
            vec![source("damage-output-2", EvidenceClass::ModelEstimate)],
        ));
        assert_eq!(result, Err(TransitionError::ClockRegression));
        assert_eq!(ledger, before);
    }

    #[test]
    fn cross_clock_event_is_rejected_without_mutation() {
        let mut ledger = ledger();
        let mut e = event(
            "evt-1",
            0,
            subject(),
            10,
            TransitionKind::Assessment {
                claims: vec![model_claim("claim-1", "damage-output")],
            },
            vec![source("damage-output", EvidenceClass::ModelEstimate)],
        );
        e.time.clock_id = "unmapped-clock".into();
        let before = ledger.clone();
        assert_eq!(ledger.apply(e), Err(TransitionError::ClockMismatch));
        assert_eq!(ledger, before);
    }

    #[test]
    fn identical_ordered_event_streams_produce_identical_reference_ledgers() {
        let make_stream = || {
            vec![
                event(
                    "evt-1",
                    0,
                    subject(),
                    10,
                    TransitionKind::Assessment {
                        claims: vec![model_claim("claim-1", "damage-output-1")],
                    },
                    vec![source("damage-output-1", EvidenceClass::ModelEstimate)],
                ),
                event(
                    "evt-2",
                    1,
                    subject(),
                    20,
                    TransitionKind::Maintenance {
                        action: MaintenanceAction::Inspect,
                        postcondition_claims: vec![ConditionClaim {
                            claim_id: "inspection-2".into(),
                            mechanism: DamageMechanism::CrackGrowth,
                            disposition: ConditionDisposition::Inconclusive,
                            quantity: None,
                            uncertainty: None,
                            evidence_id: "inspection-evidence".into(),
                            evidence_class: EvidenceClass::Measured,
                            model_profile_id: None,
                        }],
                    },
                    vec![source("inspection-evidence", EvidenceClass::Measured)],
                ),
            ]
        };

        let mut a = ledger();
        let mut b = ledger();
        for e in make_stream() {
            a.apply(e).unwrap();
        }
        for e in make_stream() {
            b.apply(e).unwrap();
        }
        assert_eq!(a, b);
    }
}
