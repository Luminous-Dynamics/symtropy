//! Dissonance and paradox handling without forced consensus.
//!
//! The model separates empirical disagreements, normative conflicts, resource
//! trade-offs, temporal tensions, structural paradoxes and interpretive
//! differences. It never assigns a moral score to a person or labels a
//! disagreement constructive/corrosive from language alone.
//!
//! Constructive conditions and corrosive hazards are represented as explicit
//! procedural evidence. Reported hazards are not silently discarded. A
//! provisional decision may be made while preserving minority positions,
//! but the host must provide the relevant appeal, review and rights protections.

use std::collections::{BTreeMap, BTreeSet};

/// The kind of tension being recorded.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DissonanceKind {
    /// Disagreement about claims that can in principle be tested against evidence.
    EmpiricalDisagreement,
    /// Conflict between values, rights, or legitimate normative commitments.
    NormativeConflict,
    /// Competing claims on scarce physical, financial, or human capacity.
    ResourceTradeoff,
    /// Tension between short-term and long-term obligations or effects.
    TemporalTension,
    /// Persistent, interdependent demands that cannot be permanently collapsed.
    StructuralParadox,
    /// Different interpretations, frames, experiences, or meanings.
    InterpretiveDifference,
}

/// Stable identifier for a position in a recorded tension.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub struct PositionId(String);

impl PositionId {
    /// Construct a bounded, non-empty position identifier.
    pub fn new(value: impl Into<String>) -> Result<Self, DissonanceError> {
        let value = value.into();
        let value = value.trim();
        if value.is_empty() || value.len() > 128 {
            return Err(DissonanceError::InvalidIdentifier);
        }
        Ok(Self(value.to_owned()))
    }

    /// Return the canonical identifier.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A recorded position, including its rationale and traceable evidence references.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Position {
    /// Stable position identifier.
    pub id: PositionId,
    /// Stakeholder, constituency, or source identifier; not a moral rank.
    pub source: String,
    /// Faithful statement of the position.
    pub claim: String,
    /// Rationale supplied by the position's source.
    pub rationale: String,
    /// References to evidence, testimony, or relevant records.
    pub evidence_refs: Vec<String>,
}

impl Position {
    /// Construct a bounded position record.
    pub fn new(
        id: PositionId,
        source: impl Into<String>,
        claim: impl Into<String>,
        rationale: impl Into<String>,
        evidence_refs: Vec<String>,
    ) -> Result<Self, DissonanceError> {
        let source = source.into();
        let claim = claim.into();
        let rationale = rationale.into();
        if source.trim().is_empty() || source.len() > 256
            || claim.trim().is_empty() || claim.len() > 4_096
            || rationale.len() > 8_192
            || evidence_refs.len() > 128
            || evidence_refs.iter().any(|item| item.trim().is_empty() || item.len() > 512)
        {
            return Err(DissonanceError::InvalidPosition);
        }
        Ok(Self { id, source, claim, rationale, evidence_refs })
    }
}

/// Safeguards required before a consequential joint decision.
///
/// These are explicit procedural attestations. They must be supported by
/// external evidence and contestable by affected participants; setting a bool
/// in this in-memory model is not proof that a real process was fair.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ProcessSafeguards {
    /// Each position has been checked for faithful representation by its source.
    pub position_fidelity_confirmed: bool,
    /// Affected people have an accessible way to contribute, including offline routes.
    pub affected_voice_accessible: bool,
    /// Dissenting views and reasons are retained in the decision record.
    pub dissent_preserved: bool,
    /// Evidence and uncertainty are made available for review.
    pub evidence_reviewable: bool,
    /// A real, independent appeal route exists.
    pub appeal_available: bool,
    /// Decision-makers disclose material power and conflict-of-interest relationships.
    pub power_context_disclosed: bool,
}

/// A reported or assessed hazard that may make a disagreement corrosive.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum CorrosiveHazard {
    /// Retaliation or threat for dissent, refusal, or complaint.
    Retaliation,
    /// Coercion, dependency pressure, or threat to essential access.
    Coercion,
    /// Essential services conditioned on agreement, participation, or loyalty.
    EssentialAccessConditioning,
    /// Suppression, destruction, or selective withholding of relevant evidence.
    EvidenceSuppression,
    /// A position is materially misrepresented after correction was requested.
    Misrepresentation,
    /// Intimidation, harassment, or status-based silencing.
    Intimidation,
    /// Disproportionate collection or exposure of sensitive personal information.
    Surveillance,
}

/// The evidentiary state of a corrosive-hazard report.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HazardFinding {
    /// A participant has reported the hazard; no finding has been made.
    Reported,
    /// Independent corroboration exists, but assessment is incomplete.
    Corroborated,
    /// The hazard has been substantiated under the applicable review process.
    Substantiated,
    /// The claim was reviewed and not substantiated; retain the audit trail.
    Refuted,
    /// The review is inconclusive and the hazard remains unresolved.
    Unresolved,
}

/// A traceable hazard report.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HazardReport {
    /// Stable report identifier.
    pub id: String,
    /// Hazard category.
    pub hazard: CorrosiveHazard,
    /// Current finding status.
    pub finding: HazardFinding,
    /// Evidence or case reference; avoid public raw sensitive data.
    pub evidence_ref: Option<String>,
}

/// A position's explicitly bounded context in a contextualized policy.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContextAssignment {
    /// Position being applied in this context.
    pub position_id: PositionId,
    /// Publicly explainable context boundary for this position.
    pub context_rule: String,
}

/// Permitted ways to act on a tension without erasing its history.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DissonanceDisposition {
    /// Keep the disagreement open, with a defined review point.
    HoldOpen {
        /// Time at which the tension must be revisited.
        review_at: u64,
    },
    /// Preserve multiple positions in an explicitly integrated arrangement.
    Integrate {
        /// Position identifiers intentionally represented in the arrangement.
        position_ids: Vec<PositionId>,
        /// Reason the arrangement can honor more than one pole.
        rationale: String,
        /// Mandatory review point.
        review_at: u64,
    },
    /// Prioritize one pole temporarily while retaining the excluded positions.
    PrioritizeProvisionally {
        /// Position selected for the current decision period.
        selected: PositionId,
        /// Every non-selected position that remains visible and contestable.
        retained: Vec<PositionId>,
        /// Reason for the temporary priority.
        rationale: String,
        /// Mandatory review point.
        review_at: u64,
        /// Whether rollback is operationally possible.
        reversible: bool,
    },
    /// Sequence two demands over time rather than pretending to satisfy both at once.
    Sequence {
        /// First position to be prioritized.
        first: PositionId,
        /// Second position to receive a defined later phase.
        second: PositionId,
        /// Every additional recorded position retained outside the two-phase sequence.
        retained: Vec<PositionId>,
        /// Reason and sequence assumptions.
        rationale: String,
        /// Mandatory review point.
        review_at: u64,
        /// Condition that triggers rollback or phase change.
        rollback_trigger: String,
    },
    /// Test a bounded intervention while preserving the rival positions.
    Experiment {
        /// Competing positions tested by the protocol.
        position_ids: Vec<PositionId>,
        /// Pre-registered protocol identifier.
        protocol_id: String,
        /// Mandatory review point.
        review_at: u64,
        /// Observable condition that stops or reverses the experiment.
        stop_condition: String,
    },
    /// Use different rules in explicitly described contexts.
    Contextualize {
        /// Explicit position-to-context assignments; every recorded position is required.
        assignments: Vec<ContextAssignment>,
        /// Mandatory review point.
        review_at: u64,
    },
    /// Resolve an empirical disagreement through a traceable evidence process.
    ///
    /// All original positions remain in the record; resolution does not erase
    /// the history or imply that normative disagreements are empirical.
    EvidenceResolution {
        /// Position supported by the reviewed evidence.
        supported: PositionId,
        /// Identifier of the reviewed evidence bundle.
        evidence_bundle: String,
        /// Reasoning, limitations, and residual uncertainty.
        rationale: String,
        /// Date of the evidence review.
        reviewed_at: u64,
    },
}

/// Current procedural state of a dissonance record.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DissonanceStatus {
    /// Positions are recorded and have not entered structured inquiry.
    Open,
    /// Structured inquiry is active.
    UnderInquiry,
    /// A time-bounded action is being taken while the disagreement is preserved.
    Provisional,
    /// Positions are being tested in a bounded experiment.
    Experimenting,
    /// Context-specific approaches have been defined.
    Contextualized,
    /// Multiple positions have been intentionally integrated.
    Integrated,
    /// An empirical issue has a recorded evidence-based resolution.
    EvidenceResolved,
    /// The record is retained without a current active process.
    HeldOpen,
}

/// Host-supplied event context. The host must authenticate actor identity and
/// verify that receipt_ref commits to the canonical event and prior receipt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventContext {
    actor_id: String,
    occurred_at: u64,
    previous_receipt_ref: Option<String>,
    receipt_ref: String,
}

impl EventContext {
    /// Construct a bounded event context. Receipt verification remains a host responsibility.
    pub fn new(
        actor_id: impl Into<String>,
        occurred_at: u64,
        previous_receipt_ref: Option<String>,
        receipt_ref: impl Into<String>,
    ) -> Result<Self, DissonanceError> {
        let actor_id = actor_id.into();
        let receipt_ref = receipt_ref.into();
        if actor_id.trim().is_empty() || actor_id.len() > 256
            || receipt_ref.trim().is_empty() || receipt_ref.len() > 512
            || previous_receipt_ref.as_ref().is_some_and(|value| {
                value.trim().is_empty() || value.len() > 512 || value == &receipt_ref
            })
        {
            return Err(DissonanceError::InvalidEventContext);
        }
        Ok(Self { actor_id, occurred_at, previous_receipt_ref, receipt_ref })
    }
}

/// Immutable payload of a process event.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DissonanceEventKind {
    /// A new explicit safeguard snapshot was recorded.
    SafeguardsRecorded {
        /// Safeguards asserted for the state after this event.
        safeguards: ProcessSafeguards,
    },
    /// A hazard report was added without making an automatic finding about intent.
    HazardReported {
        /// The new hazard report.
        report: HazardReport,
    },
    /// A hazard finding changed with a new supporting evidence receipt.
    HazardFindingChanged {
        /// Stable hazard report identifier.
        report_id: String,
        /// Finding before this event.
        previous_finding: HazardFinding,
        /// Finding after this event.
        finding: HazardFinding,
        /// Evidence reference supporting the change.
        evidence_ref: String,
    },
    /// A disposition was appended without erasing previous positions.
    DispositionRecorded {
        /// The newly recorded disposition.
        disposition: DissonanceDisposition,
    },
}

/// One append-only process event. Sequence and receipt links allow deterministic
/// replay; they are not a substitute for host-side signature or digest verification.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DissonanceEvent {
    sequence: u64,
    actor_id: String,
    occurred_at: u64,
    previous_receipt_ref: Option<String>,
    receipt_ref: String,
    kind: DissonanceEventKind,
}

impl DissonanceEvent {
    /// Monotonic sequence number within this record.
    pub const fn sequence(&self) -> u64 { self.sequence }

    /// Authenticated actor identifier asserted by the host adapter.
    pub fn actor_id(&self) -> &str { &self.actor_id }

    /// Host-supplied event time in the record's declared time basis.
    pub const fn occurred_at(&self) -> u64 { self.occurred_at }

    /// Prior receipt reference in the append-only chain.
    pub fn previous_receipt_ref(&self) -> Option<&str> {
        self.previous_receipt_ref.as_deref()
    }

    /// Receipt reference that binds this event and its predecessor.
    pub fn receipt_ref(&self) -> &str { &self.receipt_ref }

    /// Event payload.
    pub fn kind(&self) -> &DissonanceEventKind { &self.kind }
}

/// A multi-perspective record that does not force a single truth or consensus.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DissonanceRecord {
    kind: DissonanceKind,
    positions: BTreeMap<PositionId, Position>,
    safeguards: ProcessSafeguards,
    hazards: BTreeMap<String, HazardReport>,
    disposition_history: Vec<DissonanceDisposition>,
    event_history: Vec<DissonanceEvent>,
    status: DissonanceStatus,
}

/// Explicit assessment of process conditions. This is a vector of findings,
/// not a scalar conflict-quality or moral score.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DissonanceAssessment {
    /// Constructive process conditions that are currently evidenced.
    pub constructive_conditions: Vec<&'static str>,
    /// Corrosive hazards that remain active or unresolved.
    pub corrosive_hazards: Vec<CorrosiveHazard>,
    /// Required safeguards that are still absent.
    pub missing_safeguards: Vec<&'static str>,
    /// Whether consequential joint action may proceed under this model.
    pub ready_for_consequential_action: bool,
}

/// Errors from the dissonance and paradox model.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DissonanceError {
    /// An identifier is empty or exceeds its bound.
    InvalidIdentifier,
    /// A position has missing or overlong content.
    InvalidPosition,
    /// The position identifier is already recorded.
    DuplicatePosition,
    /// A referenced position or hazard report does not exist.
    UnknownReference,
    /// A hazard report identifier is already recorded.
    DuplicateHazardReport,
    /// A review time must be later than the current time.
    InvalidReviewTime,
    /// A consequential decision is blocked by missing safeguards or active hazards.
    UnsafeProcess,
    /// Integrating poles must include at least two distinct recorded positions.
    InvalidIntegration,
    /// A decision must preserve every non-selected recorded position.
    DissentNotPreserved,
    /// Empirical evidence resolution is only valid for an empirical disagreement.
    NotEmpiricalDisagreement,
    /// A required disposition field is empty or malformed.
    InvalidDisposition,
    /// Event actor or receipt metadata is invalid.
    InvalidEventContext,
    /// Event does not link to the prior receipt in the current chain.
    BrokenEventChain,
    /// Event timestamp precedes the prior event timestamp.
    NonMonotonicEventTime,
    /// A receipt reference has already appeared in this record.
    DuplicateEventReceipt,
    /// Event sequence space has been exhausted.
    EventSequenceExhausted,
}

/// Create and maintain a dissonance record with a durable history of positions.
impl DissonanceRecord {
    /// Create a record with two or more faithfully represented positions.
    pub fn new(kind: DissonanceKind, positions: Vec<Position>) -> Result<Self, DissonanceError> {
        if positions.len() < 2 {
            return Err(DissonanceError::InvalidPosition);
        }
        let mut stored = BTreeMap::new();
        for position in positions {
            let id = position.id.clone();
            if stored.insert(id, position).is_some() {
                return Err(DissonanceError::DuplicatePosition);
            }
        }
        Ok(Self {
            kind,
            positions: stored,
            safeguards: ProcessSafeguards::default(),
            hazards: BTreeMap::new(),
            disposition_history: Vec::new(),
            event_history: Vec::new(),
            status: DissonanceStatus::Open,
        })
    }

    /// Return the tension kind.
    pub const fn kind(&self) -> DissonanceKind {
        self.kind
    }

    /// Return the current status.
    pub const fn status(&self) -> DissonanceStatus {
        self.status
    }

    /// Return the current disposition, if any.
    pub fn disposition(&self) -> Option<&DissonanceDisposition> {
        self.disposition_history.last()
    }

    /// Return the next mandatory review time for the current disposition.
    /// Empirical evidence resolutions have no automatic review date; hosts may
    /// reopen them when new evidence or changed conditions warrant it.
    pub fn next_review_at(&self) -> Option<u64> {
        match self.disposition()? {
            DissonanceDisposition::HoldOpen { review_at }
            | DissonanceDisposition::Integrate { review_at, .. }
            | DissonanceDisposition::PrioritizeProvisionally { review_at, .. }
            | DissonanceDisposition::Sequence { review_at, .. }
            | DissonanceDisposition::Experiment { review_at, .. }
            | DissonanceDisposition::Contextualize { review_at, .. } => Some(*review_at),
            DissonanceDisposition::EvidenceResolution { .. } => None,
        }
    }

    /// Whether the current disposition has reached or passed its mandatory review time.
    pub fn review_due(&self, now: u64) -> bool {
        self.next_review_at().is_some_and(|review_at| now >= review_at)
    }

    /// Iterate over every disposition in append-only decision history.
    pub fn disposition_history(&self) -> impl Iterator<Item = &DissonanceDisposition> {
        self.disposition_history.iter()
    }

    /// Iterate over every append-only process event in sequence order.
    pub fn event_history(&self) -> impl Iterator<Item = &DissonanceEvent> {
        self.event_history.iter()
    }

    /// Rebuild a record by replaying a complete event stream from its genesis positions.
    /// The host must independently verify the genesis receipt, actor signatures and
    /// receipt digests before treating replayed state as authoritative.
    pub fn replay(
        kind: DissonanceKind,
        positions: Vec<Position>,
        events: &[DissonanceEvent],
    ) -> Result<Self, DissonanceError> {
        let mut record = Self::new(kind, positions)?;
        for event in events {
            let context = EventContext {
                actor_id: event.actor_id.clone(),
                occurred_at: event.occurred_at,
                previous_receipt_ref: event.previous_receipt_ref.clone(),
                receipt_ref: event.receipt_ref.clone(),
            };
            match &event.kind {
                DissonanceEventKind::SafeguardsRecorded { safeguards } => {
                    record.record_safeguards(*safeguards, context)?;
                }
                DissonanceEventKind::HazardReported { report } => {
                    record.report_hazard_with_context(report.clone(), context)?;
                }
                DissonanceEventKind::HazardFindingChanged {
                    report_id, finding, evidence_ref, ..
                } => {
                    record.update_hazard_finding_with_context(
                        report_id, *finding, evidence_ref.clone(), context
                    )?;
                }
                DissonanceEventKind::DispositionRecorded { disposition } => {
                    record.set_disposition_with_context(disposition.clone(), context)?;
                }
            }
            if record.event_history.last() != Some(event) {
                return Err(DissonanceError::BrokenEventChain);
            }
        }
        Ok(record)
    }

    /// Iterate over every preserved position in canonical ID order.
    pub fn positions(&self) -> impl Iterator<Item = &Position> {
        self.positions.values()
    }

    fn append_event(
        &mut self,
        context: EventContext,
        kind: DissonanceEventKind,
    ) -> Result<(), DissonanceError> {
        let expected_previous = self.event_history.last().map(|event| event.receipt_ref.clone());
        if context.previous_receipt_ref != expected_previous {
            return Err(DissonanceError::BrokenEventChain);
        }
        if self.event_history.last().is_some_and(|event| context.occurred_at < event.occurred_at) {
            return Err(DissonanceError::NonMonotonicEventTime);
        }
        if self.event_history.iter().any(|event| event.receipt_ref == context.receipt_ref) {
            return Err(DissonanceError::DuplicateEventReceipt);
        }
        let sequence = u64::try_from(self.event_history.len())
            .ok()
            .and_then(|length| length.checked_add(1))
            .ok_or(DissonanceError::EventSequenceExhausted)?;
        self.event_history.push(DissonanceEvent {
            sequence,
            actor_id: context.actor_id,
            occurred_at: context.occurred_at,
            previous_receipt_ref: context.previous_receipt_ref,
            receipt_ref: context.receipt_ref,
            kind,
        });
        Ok(())
    }

    /// Record a new safeguard snapshot as an attributed, receipt-linked event.
    pub fn record_safeguards(
        &mut self,
        safeguards: ProcessSafeguards,
        context: EventContext,
    ) -> Result<(), DissonanceError> {
        self.append_event(context, DissonanceEventKind::SafeguardsRecorded { safeguards })?;
        self.safeguards = safeguards;
        if self.status == DissonanceStatus::Open {
            self.status = DissonanceStatus::UnderInquiry;
        }
        Ok(())
    }

    /// Add an attributed hazard report without making an automatic finding about intent.
    pub fn report_hazard_with_context(
        &mut self,
        report: HazardReport,
        context: EventContext,
    ) -> Result<(), DissonanceError> {
        if report.id.trim().is_empty() || report.id.len() > 128
            || report.evidence_ref.as_ref().is_some_and(|value| {
                value.trim().is_empty() || value.len() > 512
            })
        {
            return Err(DissonanceError::InvalidIdentifier);
        }
        if self.hazards.contains_key(&report.id) {
            return Err(DissonanceError::DuplicateHazardReport);
        }
        self.append_event(
            context,
            DissonanceEventKind::HazardReported { report: report.clone() },
        )?;
        self.hazards.insert(report.id.clone(), report);
        Ok(())
    }

    /// Append a hazard finding transition. New evidence is mandatory; previous
    /// findings remain available in event history and are never overwritten there.
    pub fn update_hazard_finding_with_context(
        &mut self,
        report_id: &str,
        finding: HazardFinding,
        evidence_ref: String,
        context: EventContext,
    ) -> Result<(), DissonanceError> {
        if evidence_ref.trim().is_empty() || evidence_ref.len() > 512 {
            return Err(DissonanceError::InvalidIdentifier);
        }
        let report = self.hazards.get(report_id).ok_or(DissonanceError::UnknownReference)?;
        let previous_finding = report.finding;
        self.append_event(
            context,
            DissonanceEventKind::HazardFindingChanged {
                report_id: report_id.to_owned(),
                previous_finding,
                finding,
                evidence_ref: evidence_ref.clone(),
            },
        )?;
        let report = self.hazards.get_mut(report_id).ok_or(DissonanceError::UnknownReference)?;
        report.finding = finding;
        report.evidence_ref = Some(evidence_ref);
        Ok(())
    }

    /// Iterate over retained hazard reports.
    pub fn hazard_reports(&self) -> impl Iterator<Item = &HazardReport> {
        self.hazards.values()
    }

    /// Produce a non-scalar assessment of constructive conditions and corrosive risks.
    pub fn assess(&self) -> DissonanceAssessment {
        let mut constructive = Vec::new();
        let mut missing = Vec::new();

        macro_rules! condition {
            ($field:ident, $name:literal) => {
                if self.safeguards.$field {
                    constructive.push($name);
                } else {
                    missing.push($name);
                }
            };
        }

        condition!(position_fidelity_confirmed, "position fidelity confirmed");
        condition!(affected_voice_accessible, "affected-party voice accessible");
        condition!(dissent_preserved, "dissent preserved");
        condition!(evidence_reviewable, "evidence reviewable");
        condition!(appeal_available, "independent appeal available");
        condition!(power_context_disclosed, "power context disclosed");

        let hazards: BTreeSet<CorrosiveHazard> = self.hazards.values()
            .filter(|report| report.finding != HazardFinding::Refuted)
            .map(|report| report.hazard)
            .collect();
        let ready = missing.is_empty() && hazards.is_empty();

        DissonanceAssessment {
            constructive_conditions: constructive,
            corrosive_hazards: hazards.into_iter().collect(),
            missing_safeguards: missing,
            ready_for_consequential_action: ready,
        }
    }

    /// Record a disposition. Every original position remains in the record.
    ///
    /// Holding a tension open does not require procedural readiness. Any
    /// consequential action requires all safeguards and no active/unresolved
    /// corrosive hazard. The host must separately enforce legal rights and
    /// emergency authority; this model is not an authorization engine.
    pub fn set_disposition_with_context(
        &mut self,
        disposition: DissonanceDisposition,
        context: EventContext,
    ) -> Result<(), DissonanceError> {
        let now = context.occurred_at;
        validate_disposition(&disposition, now, &self.positions, self.kind)?;
        if !matches!(&disposition, DissonanceDisposition::HoldOpen { .. })
            && !self.assess().ready_for_consequential_action
        {
            return Err(DissonanceError::UnsafeProcess);
        }

        let next_status = match &disposition {
            DissonanceDisposition::HoldOpen { .. } => DissonanceStatus::HeldOpen,
            DissonanceDisposition::Integrate { .. } => DissonanceStatus::Integrated,
            DissonanceDisposition::PrioritizeProvisionally { .. }
            | DissonanceDisposition::Sequence { .. } => DissonanceStatus::Provisional,
            DissonanceDisposition::Experiment { .. } => DissonanceStatus::Experimenting,
            DissonanceDisposition::Contextualize { .. } => DissonanceStatus::Contextualized,
            DissonanceDisposition::EvidenceResolution { .. } => DissonanceStatus::EvidenceResolved,
        };
        self.append_event(
            context,
            DissonanceEventKind::DispositionRecorded { disposition: disposition.clone() },
        )?;
        self.status = next_status;
        self.disposition_history.push(disposition);
        Ok(())
    }

    // Test-only conveniences preserve old unit-test readability. Production callers
    // cannot bypass actor attribution, timestamp ordering or receipt-chain checks.
    #[cfg(test)]
    fn test_context(&self, at: u64) -> EventContext {
        let previous = self.event_history.last().map(|event| event.receipt_ref.clone());
        EventContext::new(
            "test:legacy-adapter",
            self.event_history.last().map_or(at, |event| at.max(event.occurred_at)),
            previous,
            format!("test:receipt:{}", self.event_history.len() + 1),
        ).unwrap()
    }

    #[cfg(test)]
    fn set_safeguards(&mut self, safeguards: ProcessSafeguards) {
        let next_time = self.event_history.last().map_or(1, |event| event.occurred_at + 1);
        let context = self.test_context(next_time);
        self.record_safeguards(safeguards, context).unwrap();
    }

    #[cfg(test)]
    fn report_hazard(&mut self, report: HazardReport) -> Result<(), DissonanceError> {
        let next_time = self.event_history.last().map_or(1, |event| event.occurred_at + 1);
        let context = self.test_context(next_time);
        self.report_hazard_with_context(report, context)
    }

    #[cfg(test)]
    fn update_hazard_finding(
        &mut self,
        report_id: &str,
        finding: HazardFinding,
        evidence_ref: Option<String>,
    ) -> Result<(), DissonanceError> {
        let evidence_ref = evidence_ref.ok_or(DissonanceError::InvalidIdentifier)?;
        let next_time = self.event_history.last().map_or(1, |event| event.occurred_at + 1);
        let context = self.test_context(next_time);
        self.update_hazard_finding_with_context(report_id, finding, evidence_ref, context)
    }

    #[cfg(test)]
    fn set_disposition(
        &mut self,
        disposition: DissonanceDisposition,
        now: u64,
    ) -> Result<(), DissonanceError> {
        let context = self.test_context(now);
        self.set_disposition_with_context(disposition, context)
    }
}

fn validate_disposition(
    disposition: &DissonanceDisposition,
    now: u64,
    positions: &BTreeMap<PositionId, Position>,
    kind: DissonanceKind,
) -> Result<(), DissonanceError> {
    let has_position = |id: &PositionId| positions.contains_key(id);
    let future = |time: u64| {
        if time > now { Ok(()) } else { Err(DissonanceError::InvalidReviewTime) }
    };
    match disposition {
        DissonanceDisposition::HoldOpen { review_at } => future(*review_at),
        DissonanceDisposition::Integrate { position_ids, rationale, review_at } => {
            future(*review_at)?;
            let ids: BTreeSet<&PositionId> = position_ids.iter().collect();
            if ids.len() < 2
                || ids.len() != position_ids.len()
                || ids.iter().any(|id| !has_position(id))
            {
                return Err(DissonanceError::InvalidIntegration);
            }
            if ids.len() != positions.len()
                || positions.keys().any(|id| !position_ids.iter().any(|candidate| candidate == id))
            {
                return Err(DissonanceError::DissentNotPreserved);
            }
            if rationale.trim().is_empty() || rationale.len() > 8_192 {
                return Err(DissonanceError::InvalidDisposition);
            }
            Ok(())
        }
        DissonanceDisposition::PrioritizeProvisionally {
            selected,
            retained,
            rationale,
            review_at,
            ..
        } => {
            future(*review_at)?;
            if !has_position(selected) || rationale.trim().is_empty() || rationale.len() > 8_192 {
                return Err(DissonanceError::InvalidDisposition);
            }
            let retained_set: BTreeSet<&PositionId> = retained.iter().collect();
            if retained_set.len() != retained.len() || retained.iter().any(|id| id == selected)
                || positions
                    .keys()
                    .any(|id| id != selected && !retained.iter().any(|other| other == id))
                || retained.iter().any(|id| !has_position(id))
            {
                return Err(DissonanceError::DissentNotPreserved);
            }
            Ok(())
        }
        DissonanceDisposition::Sequence {
            first,
            second,
            retained,
            rationale,
            review_at,
            rollback_trigger,
        } => {
            future(*review_at)?;
            let retained_set: BTreeSet<&PositionId> = retained.iter().collect();
            if first == second || !has_position(first) || !has_position(second)
                || retained_set.len() != retained.len()
                || retained.iter().any(|id| !has_position(id) || id == first || id == second)
                || positions
                    .keys()
                    .any(|id| {
                        id != first && id != second && !retained.iter().any(|other| other == id)
                    })
                || rationale.trim().is_empty() || rollback_trigger.trim().is_empty()
                || rationale.len() > 8_192 || rollback_trigger.len() > 2_048
            {
                return Err(DissonanceError::DissentNotPreserved);
            }
            Ok(())
        }
        DissonanceDisposition::Experiment {
            position_ids,
            protocol_id,
            review_at,
            stop_condition,
        } => {
            future(*review_at)?;
            let ids: BTreeSet<&PositionId> = position_ids.iter().collect();
            if ids.len() < 2 || ids.len() != position_ids.len()
                || ids.iter().any(|id| !has_position(id))
                || protocol_id.trim().is_empty() || protocol_id.len() > 256
                || stop_condition.trim().is_empty() || stop_condition.len() > 2_048
            {
                return Err(DissonanceError::InvalidDisposition);
            }
            Ok(())
        }
        DissonanceDisposition::Contextualize { assignments, review_at } => {
            future(*review_at)?;
            let ids: BTreeSet<&PositionId> = assignments
                .iter()
                .map(|assignment| &assignment.position_id)
                .collect();
            if ids.len() != assignments.len()
                || ids.len() != positions.len()
                || positions
                    .keys()
                    .any(|id| !assignments.iter().any(|item| &item.position_id == id))
                || assignments.iter().any(|item| {
                    !has_position(&item.position_id)
                        || item.context_rule.trim().is_empty()
                        || item.context_rule.len() > 4_096
                })
            {
                return Err(DissonanceError::DissentNotPreserved);
            }
            Ok(())
        }
        DissonanceDisposition::EvidenceResolution {
            supported,
            evidence_bundle,
            rationale,
            reviewed_at,
        } => {
            if kind != DissonanceKind::EmpiricalDisagreement {
                return Err(DissonanceError::NotEmpiricalDisagreement);
            }
            if !has_position(supported) || evidence_bundle.trim().is_empty()
                || rationale.trim().is_empty() || evidence_bundle.len() > 256
                || rationale.len() > 8_192 || *reviewed_at > now
            {
                return Err(DissonanceError::InvalidDisposition);
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn position(id: &str, source: &str, claim: &str) -> Position {
        Position::new(
            PositionId::new(id).unwrap(),
            source,
            claim,
            "A reasoned statement that remains attributable to its source.",
            vec!["evidence:1".to_owned()],
        ).unwrap()
    }

    fn record(kind: DissonanceKind) -> DissonanceRecord {
        DissonanceRecord::new(
            kind,
            vec![
                position("p1", "workers", "Protect worker autonomy."),
                position("p2", "community", "Maintain reliable shared capacity."),
            ],
        ).unwrap()
    }

    fn safe_safeguards() -> ProcessSafeguards {
        ProcessSafeguards {
            position_fidelity_confirmed: true,
            affected_voice_accessible: true,
            dissent_preserved: true,
            evidence_reviewable: true,
            appeal_available: true,
            power_context_disclosed: true,
        }
    }

    fn event_context(record: &DissonanceRecord, at: u64, receipt: &str) -> EventContext {
        let previous = record.event_history().last().map(|event| event.receipt_ref().to_owned());
        EventContext::new(
            "test:reviewer",
            at.max(record.event_history().last().map_or(0, |event| event.occurred_at())),
            previous,
            format!("receipt:{receipt}"),
        ).unwrap()
    }

    fn record_safe(record: &mut DissonanceRecord, at: u64, receipt: &str) {
        let context = event_context(record, at, receipt);
        record.record_safeguards(safe_safeguards(), context).unwrap();
    }

    fn apply_disposition(
        record: &mut DissonanceRecord,
        disposition: DissonanceDisposition,
        at: u64,
    ) -> Result<(), DissonanceError> {
        let receipt = format!("decision-{}", record.event_history().count() + 1);
        let context = event_context(record, at, &receipt);
        record.set_disposition_with_context(disposition, context)
    }

    #[test]
    fn record_preserves_both_poles_without_a_composite_score() {
        let record = record(DissonanceKind::StructuralParadox);
        assert_eq!(record.positions().count(), 2);
        assert_eq!(record.status(), DissonanceStatus::Open);
        let assessment = record.assess();
        assert!(!assessment.ready_for_consequential_action);
        assert_eq!(assessment.constructive_conditions.len(), 0);
        assert_eq!(assessment.missing_safeguards.len(), 6);
    }

    #[test]
    fn hold_open_is_a_valid_outcome_not_a_failed_resolution() {
        let mut record = record(DissonanceKind::StructuralParadox);
        record.set_disposition(DissonanceDisposition::HoldOpen { review_at: 100 }, 10).unwrap();
        assert_eq!(record.status(), DissonanceStatus::HeldOpen);
        assert_eq!(record.positions().count(), 2);
    }

    #[test]
    fn consequential_action_requires_process_safeguards() {
        let mut record = record(DissonanceKind::NormativeConflict);
        let decision = DissonanceDisposition::Integrate {
            position_ids: vec![PositionId::new("p1").unwrap(), PositionId::new("p2").unwrap()],
            rationale: "A scoped joint design.".to_owned(),
            review_at: 100,
        };
        assert_eq!(
            record.set_disposition(decision.clone(), 10),
            Err(DissonanceError::UnsafeProcess)
        );
        record.set_safeguards(safe_safeguards());
        assert_eq!(record.set_disposition(decision, 10), Ok(()));
        assert_eq!(record.status(), DissonanceStatus::Integrated);
    }

    #[test]
    fn reported_corrosive_hazard_blocks_consequential_action_without_discarding_dissent() {
        let mut record = record(DissonanceKind::NormativeConflict);
        record.set_safeguards(safe_safeguards());
        record.report_hazard(HazardReport {
            id: "hazard-1".to_owned(),
            hazard: CorrosiveHazard::Retaliation,
            finding: HazardFinding::Reported,
            evidence_ref: Some("case:private-1".to_owned()),
        }).unwrap();
        let assessment = record.assess();
        assert!(assessment.corrosive_hazards.contains(&CorrosiveHazard::Retaliation));
        assert!(!assessment.ready_for_consequential_action);
        assert_eq!(record.positions().count(), 2);
    }

    #[test]
    fn a_refuted_hazard_is_retained_in_audit_but_no_longer_blocks() {
        let mut record = record(DissonanceKind::NormativeConflict);
        record.set_safeguards(safe_safeguards());
        record.report_hazard(HazardReport {
            id: "hazard-1".to_owned(),
            hazard: CorrosiveHazard::Coercion,
            finding: HazardFinding::Reported,
            evidence_ref: None,
        }).unwrap();
        record
            .update_hazard_finding("hazard-1", HazardFinding::Refuted, Some("review:1".to_owned()))
            .unwrap();
        assert_eq!(record.hazard_reports().count(), 1);
        assert!(record.assess().ready_for_consequential_action);
    }

    #[test]
    fn provisional_priority_must_preserve_all_other_positions() {
        let mut record = record(DissonanceKind::ResourceTradeoff);
        record.set_safeguards(safe_safeguards());
        let invalid = DissonanceDisposition::PrioritizeProvisionally {
            selected: PositionId::new("p1").unwrap(),
            retained: vec![],
            rationale: "Urgent service continuity.".to_owned(),
            review_at: 100,
            reversible: true,
        };
        assert_eq!(record.set_disposition(invalid, 10), Err(DissonanceError::DissentNotPreserved));
        let valid = DissonanceDisposition::PrioritizeProvisionally {
            selected: PositionId::new("p1").unwrap(),
            retained: vec![PositionId::new("p2").unwrap()],
            rationale: "Urgent service continuity, subject to review.".to_owned(),
            review_at: 100,
            reversible: true,
        };
        assert_eq!(record.set_disposition(valid, 10), Ok(()));
    }

    #[test]
    fn integration_cannot_silently_drop_a_third_position() {
        let mut record = DissonanceRecord::new(
            DissonanceKind::StructuralParadox,
            vec![
                position("p1", "workers", "Protect worker autonomy."),
                position("p2", "community", "Maintain shared capacity."),
                position("p3", "caregivers", "Protect care time and respite."),
            ],
        ).unwrap();
        record.set_safeguards(safe_safeguards());
        let decision = DissonanceDisposition::Integrate {
            position_ids: vec![PositionId::new("p1").unwrap(), PositionId::new("p2").unwrap()],
            rationale: "Integrate the first two positions.".to_owned(),
            review_at: 100,
        };
        assert_eq!(record.set_disposition(decision, 10), Err(DissonanceError::DissentNotPreserved));
    }

    #[test]
    fn contextualization_must_assign_every_position_to_a_context() {
        let mut record = DissonanceRecord::new(
            DissonanceKind::StructuralParadox,
            vec![
                position("p1", "workers", "Protect worker autonomy."),
                position("p2", "community", "Maintain shared capacity."),
                position("p3", "caregivers", "Protect care time and respite."),
            ],
        ).unwrap();
        record.set_safeguards(safe_safeguards());
        let incomplete = DissonanceDisposition::Contextualize {
            assignments: vec![
                ContextAssignment {
                    position_id: PositionId::new("p1").unwrap(),
                    context_rule: "context A".to_owned(),
                },
                ContextAssignment {
                    position_id: PositionId::new("p2").unwrap(),
                    context_rule: "context B".to_owned(),
                },
            ],
            review_at: 100,
        };
        assert_eq!(
            record.set_disposition(incomplete, 10),
            Err(DissonanceError::DissentNotPreserved)
        );
        let complete = DissonanceDisposition::Contextualize {
            assignments: vec![
                ContextAssignment {
                    position_id: PositionId::new("p1").unwrap(),
                    context_rule: "context A".to_owned(),
                },
                ContextAssignment {
                    position_id: PositionId::new("p2").unwrap(),
                    context_rule: "context B".to_owned(),
                },
                ContextAssignment {
                    position_id: PositionId::new("p3").unwrap(),
                    context_rule: "context C".to_owned(),
                },
            ],
            review_at: 100,
        };
        assert_eq!(record.set_disposition(complete, 10), Ok(()));
    }

    #[test]
    fn evidence_resolution_is_not_allowed_for_a_normative_conflict() {
        let mut record = record(DissonanceKind::NormativeConflict);
        record.set_safeguards(safe_safeguards());
        let decision = DissonanceDisposition::EvidenceResolution {
            supported: PositionId::new("p1").unwrap(),
            evidence_bundle: "bundle:1".to_owned(),
            rationale: "Evidence review.".to_owned(),
            reviewed_at: 10,
        };
        assert_eq!(
            record.set_disposition(decision, 10),
            Err(DissonanceError::NotEmpiricalDisagreement)
        );
    }

    #[test]
    fn empirical_resolution_keeps_the_original_position_history() {
        let mut record = record(DissonanceKind::EmpiricalDisagreement);
        record.set_safeguards(safe_safeguards());
        record.set_disposition(DissonanceDisposition::EvidenceResolution {
            supported: PositionId::new("p1").unwrap(),
            evidence_bundle: "bundle:1".to_owned(),
            rationale: "Pre-registered test favoured this claim within stated limits.".to_owned(),
            reviewed_at: 10,
        }, 10).unwrap();
        assert_eq!(record.status(), DissonanceStatus::EvidenceResolved);
        assert_eq!(record.positions().count(), 2);
    }

    #[test]
    fn sequence_requires_two_distinct_recorded_poles_and_a_rollback_trigger() {
        let mut record = record(DissonanceKind::TemporalTension);
        record.set_safeguards(safe_safeguards());
        let decision = DissonanceDisposition::Sequence {
            first: PositionId::new("p1").unwrap(),
            second: PositionId::new("p2").unwrap(),
            rationale: "Stabilize first, then expand autonomy.".to_owned(),
            review_at: 100,
            retained: vec![],
            rollback_trigger: "Service reliability falls below the declared floor.".to_owned(),
        };
        assert_eq!(record.set_disposition(decision, 10), Ok(()));
        assert_eq!(record.status(), DissonanceStatus::Provisional);
    }

    #[test]
    fn evidence_cannot_be_closed_by_premature_or_backdated_review() {
        let mut record = record(DissonanceKind::EmpiricalDisagreement);
        record.set_safeguards(safe_safeguards());
        let decision = DissonanceDisposition::EvidenceResolution {
            supported: PositionId::new("p1").unwrap(),
            evidence_bundle: "bundle:1".to_owned(),
            rationale: "Evidence reviewed.".to_owned(),
            reviewed_at: 11,
        };
        assert_eq!(record.set_disposition(decision, 10), Err(DissonanceError::InvalidDisposition));
    }

    #[test]
    fn changing_the_disposition_appends_history_instead_of_rewriting_it() {
        let mut record = record(DissonanceKind::StructuralParadox);
        record.set_disposition(DissonanceDisposition::HoldOpen { review_at: 100 }, 10).unwrap();
        record.set_safeguards(safe_safeguards());
        record.set_disposition(DissonanceDisposition::PrioritizeProvisionally {
            selected: PositionId::new("p1").unwrap(),
            retained: vec![PositionId::new("p2").unwrap()],
            rationale: "Temporary service continuity measure.".to_owned(),
            review_at: 200,
            reversible: true,
        }, 20).unwrap();
        assert_eq!(record.disposition_history().count(), 2);
        assert!(matches!(
            record.disposition(),
            Some(DissonanceDisposition::PrioritizeProvisionally { .. })
        ));
    }

    #[test]
    fn experiment_must_name_at_least_two_recorded_positions() {
        let mut record = record(DissonanceKind::EmpiricalDisagreement);
        record.set_safeguards(safe_safeguards());
        let decision = DissonanceDisposition::Experiment {
            position_ids: vec![PositionId::new("p1").unwrap()],
            protocol_id: "protocol:one".to_owned(),
            review_at: 100,
            stop_condition: "Stop on material harm.".to_owned(),
        };
        assert_eq!(record.set_disposition(decision, 10), Err(DissonanceError::InvalidDisposition));
    }


    #[test]
    fn process_events_are_attributed_sequenced_and_receipt_linked() {
        let mut record = record(DissonanceKind::StructuralParadox);
        record_safe(&mut record, 10, "safeguards");
        let ctx = event_context(&record, 11, "hazard-report");
        record.report_hazard_with_context(HazardReport {
            id: "hazard-1".to_owned(),
            hazard: CorrosiveHazard::Retaliation,
            finding: HazardFinding::Reported,
            evidence_ref: Some("case:1".to_owned()),
        }, ctx).unwrap();
        let ctx = event_context(&record, 12, "hazard-review");
        record.update_hazard_finding_with_context(
            "hazard-1", HazardFinding::Substantiated, "review:1".to_owned(), ctx
        ).unwrap();
        let events: Vec<&DissonanceEvent> = record.event_history().collect();
        assert_eq!(events.len(), 3);
        assert_eq!(events.iter().map(|event| event.sequence()).collect::<Vec<_>>(), vec![1, 2, 3]);
        assert_eq!(events[0].actor_id(), "test:reviewer");
        assert_eq!(events[1].previous_receipt_ref(), Some("receipt:safeguards"));
        assert_eq!(events[2].previous_receipt_ref(), Some("receipt:hazard-report"));
        assert!(matches!(
            events[2].kind(),
            DissonanceEventKind::HazardFindingChanged {
                previous_finding: HazardFinding::Reported,
                finding: HazardFinding::Substantiated,
                ..
            }
        ));
    }

    #[test]
    fn event_chain_rejects_wrong_predecessor_and_backdated_updates_atomically() {
        let mut record = record(DissonanceKind::StructuralParadox);
        let bad_context = EventContext::new(
            "test:reviewer", 10, Some("receipt:forged-predecessor".to_owned()),
            "receipt:bad-event",
        ).unwrap();
        assert_eq!(
            record.record_safeguards(safe_safeguards(), bad_context),
            Err(DissonanceError::BrokenEventChain)
        );
        assert_eq!(record.event_history().count(), 0);

        record_safe(&mut record, 20, "first");
        let backdated = EventContext::new(
            "test:reviewer", 19, Some("receipt:first".to_owned()), "receipt:backdated"
        ).unwrap();
        assert_eq!(
            record.record_safeguards(ProcessSafeguards::default(), backdated),
            Err(DissonanceError::NonMonotonicEventTime)
        );
        assert_eq!(record.event_history().count(), 1);
        assert_eq!(record.assess().missing_safeguards.len(), 0);
    }

    #[test]
    fn event_replay_reconstructs_the_same_record_projection_and_history() {
        let mut original = record(DissonanceKind::StructuralParadox);
        let genesis: Vec<Position> = original.positions().cloned().collect();
        record_safe(&mut original, 10, "safe");
        let ctx = event_context(&original, 11, "report");
        original.report_hazard_with_context(HazardReport {
            id: "hazard-1".to_owned(),
            hazard: CorrosiveHazard::Coercion,
            finding: HazardFinding::Reported,
            evidence_ref: Some("case:coercion".to_owned()),
        }, ctx).unwrap();
        let ctx = event_context(&original, 12, "refuted");
        original.update_hazard_finding_with_context(
            "hazard-1", HazardFinding::Refuted, "review:refuted".to_owned(), ctx
        ).unwrap();
        let events: Vec<DissonanceEvent> = original.event_history().cloned().collect();
        let replayed = DissonanceRecord::replay(
            DissonanceKind::StructuralParadox, genesis, &events
        ).unwrap();
        assert_eq!(replayed, original);
    }


    #[test]
    fn mandatory_review_becomes_due_without_silently_closing_the_record() {
        let mut record = record(DissonanceKind::StructuralParadox);
        apply_disposition(
            &mut record,
            DissonanceDisposition::HoldOpen { review_at: 100 },
            10,
        ).unwrap();
        assert_eq!(record.next_review_at(), Some(100));
        assert!(!record.review_due(99));
        assert!(record.review_due(100));
        assert_eq!(record.status(), DissonanceStatus::HeldOpen);
    }

    #[test]
    fn safeguard_regression_is_logged_and_blocks_later_consequential_action() {
        let mut record = record(DissonanceKind::StructuralParadox);
        record_safe(&mut record, 10, "initial-safeguards");
        let context = event_context(&record, 11, "safeguard-regression");
        record.record_safeguards(ProcessSafeguards::default(), context).unwrap();
        assert_eq!(record.event_history().count(), 2);
        assert_eq!(record.assess().missing_safeguards.len(), 6);
        let decision = DissonanceDisposition::Integrate {
            position_ids: vec![PositionId::new("p1").unwrap(), PositionId::new("p2").unwrap()],
            rationale: "An integrated arrangement.".to_owned(),
            review_at: 100,
        };
        let context = event_context(&record, 12, "blocked-after-regression");
        assert_eq!(
            record.set_disposition_with_context(decision, context),
            Err(DissonanceError::UnsafeProcess)
        );
        assert_eq!(record.event_history().count(), 2);
    }

    #[test]
    fn a_refuted_hazard_can_reopen_only_as_a_new_evidence_linked_event() {
        let mut record = record(DissonanceKind::StructuralParadox);
        let ctx = event_context(&record, 10, "hazard-report");
        record.report_hazard_with_context(HazardReport {
            id: "hazard-reopen".to_owned(),
            hazard: CorrosiveHazard::Coercion,
            finding: HazardFinding::Reported,
            evidence_ref: Some("case:initial".to_owned()),
        }, ctx).unwrap();
        let ctx = event_context(&record, 11, "hazard-refuted");
        record.update_hazard_finding_with_context(
            "hazard-reopen", HazardFinding::Refuted, "review:refutation".to_owned(), ctx
        ).unwrap();
        let ctx = event_context(&record, 12, "hazard-reopened");
        record.update_hazard_finding_with_context(
            "hazard-reopen", HazardFinding::Corroborated, "evidence:new".to_owned(), ctx
        ).unwrap();
        let events: Vec<&DissonanceEvent> = record.event_history().collect();
        assert_eq!(events.len(), 3);
        assert!(matches!(
            events[2].kind(),
            DissonanceEventKind::HazardFindingChanged {
                previous_finding: HazardFinding::Refuted,
                finding: HazardFinding::Corroborated,
                evidence_ref,
                ..
            } if evidence_ref == "evidence:new"
        ));
        assert!(record.assess().corrosive_hazards.contains(&CorrosiveHazard::Coercion));
    }

    #[test]
    fn receipt_context_rejects_reusing_the_current_receipt() {
        assert_eq!(
            EventContext::new(
                "test:reviewer", 1, Some("receipt:same".to_owned()), "receipt:same"
            ),
            Err(DissonanceError::InvalidEventContext)
        );
    }

    #[test]
    fn hazard_can_be_flagged_as_unresolved_without_inventing_a_finding() {
        let mut record = record(DissonanceKind::StructuralParadox);
        record.report_hazard(HazardReport {
            id: "hazard-u".to_owned(),
            hazard: CorrosiveHazard::EssentialAccessConditioning,
            finding: HazardFinding::Unresolved,
            evidence_ref: None,
        }).unwrap();
        assert!(record
            .assess()
            .corrosive_hazards
            .contains(&CorrosiveHazard::EssentialAccessConditioning));
    }
}
