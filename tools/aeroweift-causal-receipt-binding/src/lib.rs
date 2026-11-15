//! Structural verifier for subject-scoped AeroWeft evidence receipt graphs.
//!
//! A passing result proves structural binding only. It does not authenticate bytes,
//! verify signatures or issuer authority, evaluate revocation, or establish physical truth.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::collections::{BTreeMap, BTreeSet};

/// A required traffic population; classes are never substituted for one another.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum TrafficClass {
    /// Cooperative crewed traffic.
    CooperativeCrewed,
    /// Non-cooperative crewed traffic.
    NonCooperativeCrewed,
    /// Uncrewed traffic admitted by an explicit profile.
    ProfileQualifiedUncrewed,
}

/// Half-open evaluation window shared by every receipt in one graph.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TimeWindow {
    /// Inclusive start tick.
    pub start: i64,
    /// Exclusive end tick.
    pub end: i64,
}

impl TimeWindow {
    fn is_valid(self) -> bool {
        self.start < self.end
    }

    fn contains(self, tick: i64) -> bool {
        self.start <= tick && tick < self.end
    }
}

/// Exact operation, aircraft, authorization, profile, scenario and time scope.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubjectScope {
    /// Frozen scenario identifier.
    pub scenario_id: String,
    /// SHA-256-shaped frozen scenario manifest digest; syntax only.
    pub scenario_manifest_sha256: String,
    /// Operation/mission identifier.
    pub operation_id: String,
    /// Pseudonymous aircraft identifier.
    pub aircraft_id: String,
    /// Individual flight identifier.
    pub flight_id: String,
    /// Operational-intent identifier.
    pub intent_id: String,
    /// Operational-intent revision.
    pub intent_revision: u64,
    /// Operational-intent digest; syntax only.
    pub intent_sha256: String,
    /// Authorization identifier.
    pub authorization_id: String,
    /// Authorization revision.
    pub authorization_revision: u64,
    /// Authorization digest; syntax only.
    pub authorization_sha256: String,
    /// Aircraft/operation profile identifier.
    pub profile_id: String,
    /// Aircraft/operation profile revision.
    pub profile_revision: String,
    /// Normalized comparison time-scale identity.
    pub time_scale_id: String,
    /// Horizontal coordinate frame identity.
    pub horizontal_frame_id: String,
    /// Vertical datum identity.
    pub vertical_datum_id: String,
    /// Exact evaluation horizon.
    pub evaluation_window: TimeWindow,
}

impl SubjectScope {
    fn is_complete(&self) -> bool {
        !self.scenario_id.trim().is_empty()
            && is_sha256(&self.scenario_manifest_sha256)
            && !self.operation_id.trim().is_empty()
            && !self.aircraft_id.trim().is_empty()
            && !self.flight_id.trim().is_empty()
            && !self.intent_id.trim().is_empty()
            && is_sha256(&self.intent_sha256)
            && !self.authorization_id.trim().is_empty()
            && is_sha256(&self.authorization_sha256)
            && !self.profile_id.trim().is_empty()
            && !self.profile_revision.trim().is_empty()
            && !self.time_scale_id.trim().is_empty()
            && !self.horizontal_frame_id.trim().is_empty()
            && !self.vertical_datum_id.trim().is_empty()
            && self.evaluation_window.is_valid()
    }
}

/// Stage that produced one receipt.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Stage {
    /// Original position/track/measurement observation.
    SourceObservation,
    /// Qualified clock-correlation artifact.
    ClockCorrelation,
    /// Profile-owned operation and traffic-class policy.
    ProfilePolicy,
    /// PNT/time qualification result.
    PntQualification,
    /// Surveillance coverage/track qualification result.
    SurveillanceAssessment,
    /// Aircraft reachable occupancy result.
    Reachability,
    /// Dynamic restriction snapshot.
    RestrictionSnapshot,
    /// Current flight authorization.
    Authorization,
    /// Operational-intent conformance result.
    IntentConformance,
    /// Four-dimensional conflict-screen result.
    ConflictScreening,
    /// Candidate schedule/resource allocation.
    Schedule,
}

/// Typed claim state; freshness and physical validity remain owner responsibilities.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ClaimDisposition {
    /// Claim is qualified under its owner-declared profile.
    Qualified,
    /// Qualified coverage exists but no report arrived; occupancy remains possible.
    NoReportQualifiedCoverage,
    /// Claim is stale.
    Stale,
    /// Claim is not assessable.
    NotAssessable,
    /// Claim is revoked or withdrawn.
    Revoked,
    /// Claim is definitely nonconforming.
    Nonconforming,
    /// Claim is potentially nonconforming.
    PotentiallyNonconforming,
    /// Claim is contingent and requires a separate handoff.
    Contingent,
    /// Authorization/intent requires reauthorization.
    ReauthorizationRequired,
}

/// Semantic meaning of one consumed-parent edge.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ParentRole {
    /// Source position observation used by PNT.
    PositionObservation,
    /// Sample-time clock correlation used by PNT.
    SampleClockCorrelation,
    /// Evaluation-time clock correlation used by PNT.
    AssessmentClockCorrelation,
    /// Applicable surveillance coverage profile.
    CoverageProfile,
    /// Optional observed surveillance report.
    SurveillanceObservation,
    /// Qualified initial position used by reachability.
    PntInitialPosition,
    /// Qualified reachability result consumed by conflict screening.
    ReachableOccupancy,
    /// Profile-required traffic occupancy for one traffic class.
    TrafficOccupancy(TrafficClass),
    /// Profile-owned required traffic-class set.
    RequiredTrafficClasses,
    /// Active dynamic restriction snapshot.
    ActiveRestrictions,
    /// Current authorization consumed by conformance/scheduling.
    CurrentAuthorization,
    /// Subject position consumed by conformance.
    SubjectPosition,
    /// Qualified subject track consumed by conformance.
    SubjectTrack,
    /// Conflict result consumed by scheduling.
    ConflictAssessment,
    /// Execution conformance result consumed by scheduling.
    ExecutionConformance,
    /// Current operational-intent conformance required before conflict screening.
    OperationalIntentConformance,
    /// Prior conflict-screen receipt used as non-authoritative replay context.
    PriorConflictAssessment,
}

/// Exact reference from a child receipt to one parent receipt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParentEdge {
    /// Exact parent receipt identifier.
    pub parent_receipt_id: String,
    /// Parent payload digest copied from the parent receipt; syntax/value match only.
    pub parent_payload_sha256: String,
    /// Semantic role this exact parent is claimed to satisfy.
    pub role: ParentRole,
}

/// Receipt temporal fields. These clocks are compared, never converted here.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReceiptTime {
    /// Owner evaluation time.
    pub evaluated_at: i64,
    /// Time the owner emitted this receipt.
    pub produced_at: i64,
    /// Start of claim validity, inclusive.
    pub valid_from: i64,
    /// End of claim validity, exclusive.
    pub valid_until: i64,
    /// Original observation time, when this is an observation receipt.
    pub observed_at: Option<i64>,
    /// Arrival/receipt time of the original observation, when applicable.
    pub received_at: Option<i64>,
}

/// One immutable receipt header and its consumed-parent edges.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Receipt {
    /// Globally unique receipt identifier within this graph.
    pub receipt_id: String,
    /// Stage/owner class that emitted this receipt.
    pub stage: Stage,
    /// Schema identifier.
    pub schema_id: String,
    /// Schema revision.
    pub schema_revision: String,
    /// Producer identity.
    pub producer_id: String,
    /// Producer revision.
    pub producer_revision: String,
    /// Producer stream identifier.
    pub producer_stream_id: String,
    /// Monotonic sequence number within the producer stream.
    pub producer_sequence: u64,
    /// Payload digest string; format is checked, bytes are not available here.
    pub payload_sha256: String,
    /// Exact subject scope.
    pub subject: SubjectScope,
    /// Owner-produced claim disposition.
    pub disposition: ClaimDisposition,
    /// Temporal lineage.
    pub time: ReceiptTime,
    /// Parent edges consumed to produce this receipt.
    pub parents: Vec<ParentEdge>,
    /// Required traffic classes, present only on a profile-policy receipt.
    pub required_traffic_classes: Option<Vec<TrafficClass>>,
    /// Traffic class represented by a surveillance-assessment receipt.
    pub traffic_class: Option<TrafficClass>,
    /// Traffic class attributed to a source-observation receipt when used as a surveillance report.
    pub observation_traffic_class: Option<TrafficClass>,
}

/// Trust facets intentionally outside this structural verifier.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrustFacet {
    /// Canonical source-byte digest recomputation.
    ByteIntegrity,
    /// Signature cryptographic verification and signer identity.
    SignatureVerification,
    /// Issuer identity, role and authority-scope evaluation.
    IssuerAuthority,
    /// Key lifecycle, revocation and compromise-time evaluation.
    RevocationEvaluation,
}

/// Structural disposition of the receipt graph.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StructuralDisposition {
    /// Exact subject and causal-parent graph is structurally coherent only.
    CausallyBoundOnly,
}

/// Output is deliberately not an operational readiness or safety decision.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationResult {
    /// Strongest positive structural result.
    pub disposition: StructuralDisposition,
    /// Canonical receipt IDs in the admitted rooted graph.
    pub receipt_ids: Vec<String>,
    /// Trust facets never evaluated by this crate.
    pub trust_not_evaluated: [TrustFacet; 4],
}

/// Typed reason why a receipt graph is structurally inadmissible.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum VerificationError {
    /// Declared root receipt is absent.
    RootMissing,
    /// Expected subject scope is incomplete or malformed.
    ExpectedScopeInvalid,
    /// A receipt has incomplete metadata, digest syntax or invalid time fields.
    ReceiptMalformed,
    /// A receipt's own validity interval does not include its evaluation instant.
    ReceiptOutsideOwnValidity,
    /// Original observation arrival occurred after the owning receipt's evaluation.
    ObservationReceivedAfterEvaluation,
    /// A receipt subject differs from the exact expected subject.
    SubjectMismatch,
    /// A receipt identifier occurs more than once.
    DuplicateReceiptId,
    /// One producer stream/sequence identifies divergent or duplicate receipts.
    ForkedSequence,
    /// A parent identifier is not present in the supplied graph.
    ParentMissing,
    /// Parent digest does not equal the parent's recorded payload digest.
    ParentDigestMismatch,
    /// Parent stage is not allowed for the declared semantic role.
    RoleMismatch,
    /// A child repeats the same parent/role edge.
    DuplicateParentEdge,
    /// A required parent role is absent.
    MissingRequiredParent,
    /// Child stage declares a role that is not allowed for that stage.
    UnexpectedParentRole,
    /// A parent claim state is not eligible for the edge role.
    ParentNotQualified,
    /// Profile class obligations are absent, duplicated or changed.
    ProfileClassMismatch,
    /// A required or optional semantic parent role occurs with invalid cardinality.
    DuplicateParentRole,
    /// A surveillance receipt's class or report/no-report evidence does not match its claim.
    SurveillanceEvidenceMismatch,
    /// An external traffic observation was presented as focal ownship position evidence.
    ObservationRoleMismatch,
    /// Receipt graph contains a directed cycle.
    Cycle,
    /// A supplied receipt is not an ancestor of the declared root.
    OrphanReceipt,
    /// Parent receipt was produced after the child evaluation time.
    CausalTimeViolation,
    /// Parent claim validity does not cover the child evaluation instant.
    OutsideParentValidity,
    /// Observation time is absent, future, or after its recorded arrival.
    ObservationTimeInvalid,
    /// Observation arrival is after the receipt that records it.
    ReceiptTimeInversion,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Verification policy frozen independently of the submitted receipt graph.
pub struct VerificationPolicy {
    /// Exact scope expected for every receipt.
    pub expected_subject: SubjectScope,
    /// Required traffic classes from the independently frozen benchmark profile.
    pub required_traffic_classes: Vec<TrafficClass>,
    /// Traffic class of the subject aircraft's own surveillance track.
    pub subject_traffic_class: TrafficClass,
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn stage_for_role(role: ParentRole) -> Stage {
    match role {
        ParentRole::PositionObservation | ParentRole::SurveillanceObservation => {
            Stage::SourceObservation
        }
        ParentRole::SampleClockCorrelation | ParentRole::AssessmentClockCorrelation => {
            Stage::ClockCorrelation
        }
        ParentRole::CoverageProfile | ParentRole::RequiredTrafficClasses => Stage::ProfilePolicy,
        ParentRole::PntInitialPosition | ParentRole::SubjectPosition => Stage::PntQualification,
        ParentRole::ReachableOccupancy => Stage::Reachability,
        ParentRole::TrafficOccupancy(_) | ParentRole::SubjectTrack => Stage::SurveillanceAssessment,
        ParentRole::ActiveRestrictions => Stage::RestrictionSnapshot,
        ParentRole::CurrentAuthorization => Stage::Authorization,
        ParentRole::ConflictAssessment | ParentRole::PriorConflictAssessment => {
            Stage::ConflictScreening
        }
        ParentRole::ExecutionConformance | ParentRole::OperationalIntentConformance => {
            Stage::IntentConformance
        }
    }
}

fn required_roles(stage: Stage) -> Vec<ParentRole> {
    match stage {
        Stage::SourceObservation
        | Stage::ClockCorrelation
        | Stage::ProfilePolicy
        | Stage::RestrictionSnapshot
        | Stage::Authorization => Vec::new(),
        Stage::PntQualification => vec![
            ParentRole::PositionObservation,
            ParentRole::SampleClockCorrelation,
            ParentRole::AssessmentClockCorrelation,
        ],
        Stage::SurveillanceAssessment => vec![ParentRole::CoverageProfile],
        Stage::Reachability => vec![ParentRole::PntInitialPosition],
        Stage::IntentConformance => vec![
            ParentRole::CurrentAuthorization,
            ParentRole::SubjectPosition,
            ParentRole::SubjectTrack,
            ParentRole::ActiveRestrictions,
        ],
        Stage::ConflictScreening => vec![
            ParentRole::OperationalIntentConformance,
            ParentRole::ReachableOccupancy,
            ParentRole::RequiredTrafficClasses,
            ParentRole::ActiveRestrictions,
        ],
        Stage::Schedule => vec![
            ParentRole::ConflictAssessment,
            ParentRole::ExecutionConformance,
            ParentRole::CurrentAuthorization,
        ],
    }
}

fn optional_roles(stage: Stage) -> Vec<ParentRole> {
    match stage {
        Stage::SurveillanceAssessment => vec![ParentRole::SurveillanceObservation],
        Stage::ConflictScreening => vec![ParentRole::PriorConflictAssessment],
        _ => Vec::new(),
    }
}

fn has_role(receipt: &Receipt, role: ParentRole) -> bool {
    receipt.parents.iter().any(|edge| edge.role == role)
}

fn validate_disposition(parent: &Receipt, role: ParentRole) -> bool {
    match role {
        ParentRole::TrafficOccupancy(_) => matches!(
            parent.disposition,
            ClaimDisposition::Qualified | ClaimDisposition::NoReportQualifiedCoverage
        ),
        _ => parent.disposition == ClaimDisposition::Qualified,
    }
}

fn validate_observation_time(receipt: &Receipt) -> Result<(), VerificationError> {
    if receipt.stage != Stage::SourceObservation {
        if receipt.time.observed_at.is_some() || receipt.time.received_at.is_some() {
            return Err(VerificationError::ObservationTimeInvalid);
        }
        return Ok(());
    }
    let (Some(observed), Some(received)) = (receipt.time.observed_at, receipt.time.received_at)
    else {
        return Err(VerificationError::ObservationTimeInvalid);
    };
    if !receipt.subject.evaluation_window.contains(observed) || observed > received {
        return Err(VerificationError::ObservationTimeInvalid);
    }
    if received > receipt.time.produced_at {
        return Err(VerificationError::ReceiptTimeInversion);
    }
    if received > receipt.time.evaluated_at {
        return Err(VerificationError::ObservationReceivedAfterEvaluation);
    }
    Ok(())
}

/// Verify one receipt DAG against a separately frozen subject and traffic policy.
///
/// The root must be the final stage to qualify (normally ConflictScreening or Schedule).
/// The verifier checks identity, parent links, role, validity, causal ordering and graph closure.
pub fn verify(
    receipts: &[Receipt],
    root_receipt_id: &str,
    policy: &VerificationPolicy,
) -> Result<VerificationResult, VerificationError> {
    if !policy.expected_subject.is_complete() || policy.required_traffic_classes.is_empty() {
        return Err(VerificationError::ExpectedScopeInvalid);
    }
    let mut required = policy.required_traffic_classes.clone();
    required.sort_unstable();
    if required.windows(2).any(|pair| pair[0] == pair[1])
        || !required.contains(&policy.subject_traffic_class)
    {
        return Err(VerificationError::ProfileClassMismatch);
    }

    let mut by_id: BTreeMap<String, &Receipt> = BTreeMap::new();
    let mut streams: BTreeMap<(String, String, u64), (String, String)> = BTreeMap::new();

    for receipt in receipts {
        if receipt.receipt_id.trim().is_empty()
            || receipt.schema_id.trim().is_empty()
            || receipt.schema_revision.trim().is_empty()
            || receipt.producer_id.trim().is_empty()
            || receipt.producer_revision.trim().is_empty()
            || receipt.producer_stream_id.trim().is_empty()
            || !is_sha256(&receipt.payload_sha256)
            || !receipt.subject.is_complete()
            || receipt.time.evaluated_at > receipt.time.produced_at
            || receipt.time.valid_from >= receipt.time.valid_until
            || !receipt
                .subject
                .evaluation_window
                .contains(receipt.time.evaluated_at)
        {
            return Err(VerificationError::ReceiptMalformed);
        }
        if receipt.subject != policy.expected_subject {
            return Err(VerificationError::SubjectMismatch);
        }
        if receipt.time.evaluated_at < receipt.time.valid_from
            || receipt.time.evaluated_at >= receipt.time.valid_until
        {
            return Err(VerificationError::ReceiptOutsideOwnValidity);
        }
        validate_observation_time(receipt)?;

        if by_id.insert(receipt.receipt_id.clone(), receipt).is_some() {
            return Err(VerificationError::DuplicateReceiptId);
        }

        let stream_key = (
            receipt.producer_id.clone(),
            receipt.producer_stream_id.clone(),
            receipt.producer_sequence,
        );
        if streams
            .insert(
                stream_key,
                (receipt.receipt_id.clone(), receipt.payload_sha256.clone()),
            )
            .is_some()
        {
            return Err(VerificationError::ForkedSequence);
        }

        if receipt.stage == Stage::SurveillanceAssessment {
            if receipt.observation_traffic_class.is_some() {
                return Err(VerificationError::SurveillanceEvidenceMismatch);
            }
            if receipt.traffic_class.is_none() {
                return Err(VerificationError::SurveillanceEvidenceMismatch);
            }
        } else if receipt.traffic_class.is_some() {
            return Err(VerificationError::SurveillanceEvidenceMismatch);
        }
        if receipt.stage != Stage::SourceObservation && receipt.observation_traffic_class.is_some()
        {
            return Err(VerificationError::SurveillanceEvidenceMismatch);
        }
        if receipt.stage == Stage::ProfilePolicy {
            let Some(classes) = receipt.required_traffic_classes.as_ref() else {
                return Err(VerificationError::ProfileClassMismatch);
            };
            let mut classes = classes.clone();
            classes.sort_unstable();
            if classes != required || classes.windows(2).any(|pair| pair[0] == pair[1]) {
                return Err(VerificationError::ProfileClassMismatch);
            }
        } else if receipt.required_traffic_classes.is_some() {
            return Err(VerificationError::ProfileClassMismatch);
        }
    }

    let Some(root) = by_id.get(root_receipt_id).copied() else {
        return Err(VerificationError::RootMissing);
    };
    if !matches!(root.stage, Stage::ConflictScreening | Stage::Schedule) {
        return Err(VerificationError::RoleMismatch);
    }

    for receipt in receipts {
        let mut seen_edges = BTreeSet::new();
        for edge in &receipt.parents {
            if !seen_edges.insert((edge.parent_receipt_id.as_str(), edge.role)) {
                return Err(VerificationError::DuplicateParentEdge);
            }
            if !required_roles(receipt.stage).contains(&edge.role)
                && !optional_roles(receipt.stage).contains(&edge.role)
                && !matches!(
                    (receipt.stage, edge.role),
                    (Stage::ConflictScreening, ParentRole::TrafficOccupancy(_))
                )
            {
                return Err(VerificationError::UnexpectedParentRole);
            }

            let Some(parent) = by_id.get(&edge.parent_receipt_id).copied() else {
                return Err(VerificationError::ParentMissing);
            };
            if !is_sha256(&edge.parent_payload_sha256)
                || edge.parent_payload_sha256 != parent.payload_sha256
            {
                return Err(VerificationError::ParentDigestMismatch);
            }
            if parent.stage != stage_for_role(edge.role) {
                return Err(VerificationError::RoleMismatch);
            }
            if !validate_disposition(parent, edge.role) {
                return Err(VerificationError::ParentNotQualified);
            }
            match edge.role {
                ParentRole::PositionObservation
                    if parent.observation_traffic_class.is_some() =>
                {
                    return Err(VerificationError::ObservationRoleMismatch);
                }
                ParentRole::SurveillanceObservation
                    if receipt.traffic_class.is_none()
                        || parent.observation_traffic_class != receipt.traffic_class =>
                {
                    return Err(VerificationError::SurveillanceEvidenceMismatch);
                }
                ParentRole::TrafficOccupancy(class) if parent.traffic_class != Some(class) => {
                    return Err(VerificationError::SurveillanceEvidenceMismatch);
                }
                ParentRole::SubjectTrack
                    if parent.traffic_class != Some(policy.subject_traffic_class) =>
                {
                    return Err(VerificationError::SurveillanceEvidenceMismatch);
                }
                _ => {}
            }
            if parent.time.produced_at > receipt.time.evaluated_at {
                return Err(VerificationError::CausalTimeViolation);
            }
            if parent.time.valid_from > receipt.time.evaluated_at
                || receipt.time.evaluated_at >= parent.time.valid_until
            {
                return Err(VerificationError::OutsideParentValidity);
            }
        }

        for role in required_roles(receipt.stage) {
            let count = receipt
                .parents
                .iter()
                .filter(|edge| edge.role == role)
                .count();
            if count == 0 {
                return Err(VerificationError::MissingRequiredParent);
            }
            if count > 1 {
                return Err(VerificationError::DuplicateParentRole);
            }
        }
        for role in optional_roles(receipt.stage) {
            let count = receipt
                .parents
                .iter()
                .filter(|edge| edge.role == role)
                .count();
            if count > 1 {
                return Err(VerificationError::DuplicateParentRole);
            }
        }
        if receipt.stage == Stage::SurveillanceAssessment {
            let has_observation = has_role(receipt, ParentRole::SurveillanceObservation);
            match receipt.disposition {
                ClaimDisposition::Qualified if !has_observation => {
                    return Err(VerificationError::SurveillanceEvidenceMismatch);
                }
                ClaimDisposition::NoReportQualifiedCoverage if has_observation => {
                    return Err(VerificationError::SurveillanceEvidenceMismatch);
                }
                _ => {}
            }
        }

        if receipt.stage == Stage::ConflictScreening {
            for edge in &receipt.parents {
                if let ParentRole::TrafficOccupancy(class) = edge.role
                    && !required.contains(&class)
                {
                    return Err(VerificationError::ProfileClassMismatch);
                }
            }
            for class in &required {
                let count = receipt
                    .parents
                    .iter()
                    .filter(|edge| edge.role == ParentRole::TrafficOccupancy(*class))
                    .count();
                if count == 0 {
                    return Err(VerificationError::MissingRequiredParent);
                }
                if count > 1 {
                    return Err(VerificationError::DuplicateParentRole);
                }
            }
            let profile = receipt
                .parents
                .iter()
                .find(|edge| edge.role == ParentRole::RequiredTrafficClasses)
                .and_then(|edge| by_id.get(&edge.parent_receipt_id).copied())
                .ok_or(VerificationError::MissingRequiredParent)?;
            let Some(profile_classes) = profile.required_traffic_classes.as_ref() else {
                return Err(VerificationError::ProfileClassMismatch);
            };
            let mut canonical_profile_classes = profile_classes.clone();
            canonical_profile_classes.sort_unstable();
            if canonical_profile_classes != required {
                return Err(VerificationError::ProfileClassMismatch);
            }
        }
    }

    // Iterative depth-first traversal avoids recursion-depth dependence on graph size.
    let mut colors: BTreeMap<String, u8> = BTreeMap::new();
    let mut reached = BTreeSet::new();
    let mut stack: Vec<(String, bool)> = vec![(root_receipt_id.to_owned(), false)];
    while let Some((id, exiting)) = stack.pop() {
        if exiting {
            colors.insert(id, 2);
            continue;
        }
        match colors.get(&id).copied().unwrap_or(0) {
            1 => return Err(VerificationError::Cycle),
            2 => continue,
            _ => {}
        }
        colors.insert(id.clone(), 1);
        reached.insert(id.clone());
        stack.push((id.clone(), true));

        let receipt = by_id
            .get(&id)
            .copied()
            .ok_or(VerificationError::ParentMissing)?;
        let mut parents: Vec<String> = receipt
            .parents
            .iter()
            .map(|edge| edge.parent_receipt_id.clone())
            .collect();
        parents.sort_unstable();
        for parent in parents.into_iter().rev() {
            if colors.get(&parent).copied() == Some(1) {
                return Err(VerificationError::Cycle);
            }
            if colors.get(&parent).copied() != Some(2) {
                stack.push((parent, false));
            }
        }
    }

    if reached.len() != receipts.len() {
        return Err(VerificationError::OrphanReceipt);
    }

    let mut receipt_ids: Vec<String> = reached.into_iter().collect();
    receipt_ids.sort_unstable();
    Ok(VerificationResult {
        disposition: StructuralDisposition::CausallyBoundOnly,
        receipt_ids,
        trust_not_evaluated: [
            TrustFacet::ByteIntegrity,
            TrustFacet::SignatureVerification,
            TrustFacet::IssuerAuthority,
            TrustFacet::RevocationEvaluation,
        ],
    })
}
