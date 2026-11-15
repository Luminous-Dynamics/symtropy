//! Aviation-specific bitemporal model-evidence change-impact oracle.
//!
//! This crate is a narrow consumer of owner-qualified change events and coverage receipts.
//! It does not implement a generic event store, trusted clock, EvidenceLease,
//! cryptographic trust root, physical model validation, or operational authority.
//! It evaluates synthetic dependency rules at separate effective-time and knowledge-time
//! cutoffs, preserving the historical result while computing later reliance status.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::collections::{BTreeMap, BTreeSet};

/// Model-evidence dimension whose exact identity supports a claim.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum DependencyKind {
    /// Aircraft type, configuration, mass, centre of gravity, payload and propulsion.
    AircraftConfiguration,
    /// Exact aircraft/model dynamics artifact.
    ModelArtifact,
    /// Flight-control, guidance and control-parameter configuration.
    ControlStack,
    /// Sensor, receiver, antenna placement, firmware and calibration profile.
    SensorStack,
    /// Numerical solver, integration and precision configuration.
    SolverConfiguration,
    /// Independent reference instrument and measurement calibration profile.
    ReferenceMeasurement,
    /// Declared weather, airspace, terrain, traffic and environmental domain.
    OperatingDomain,
    /// PNT, clock, time-scale and coordinate reference profile.
    NavigationTimeBasis,
    /// Frozen validation evaluator and metric policy.
    EvaluationPolicy,
}

/// Exact dependency identity embedded in a model-validation claim.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DependencyBinding {
    /// Dimension whose identity is bound.
    pub kind: DependencyKind,
    /// Exact dependency digest string; bytes are not authenticated here.
    pub digest: String,
    /// Owner/source namespace expected to report changes for this dimension.
    pub change_source_id: String,
}

/// A historical model-evidence claim and the dependency identities it used.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelEvidenceClaim {
    /// Stable identifier for this exact qualification claim.
    pub claim_id: String,
    /// Exact model-validation package digest.
    pub validation_package_sha256: String,
    /// Time at which the evidence was evaluated, on its declared time scale.
    pub validated_at_effective: i64,
    /// Time at which the qualification result became recorded/known.
    pub recorded_at: i64,
    /// Exact dependency set consumed by this claim.
    pub dependencies: Vec<DependencyBinding>,
}

/// Semantic class of an owner-declared dependency change.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChangeClass {
    /// Material successor: dependent evidence needs owner-defined requalification.
    MaterialChange,
    /// A prior source/reference statement was corrected; previous dependent claim is contradicted.
    CorrectiveChange,
    /// Component/profile was retired or superseded without asserting old historical evidence false.
    Retirement,
    /// Change semantic is unsupported by the frozen aviation policy.
    Unsupported,
}

/// Owner-reported epistemic state of a change event.
/// This crate does not independently authenticate it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChangeEvidenceState {
    /// Owner says the event is qualified under its own source contract.
    OwnerQualified,
    /// Owner says the event is invalid or not admitted.
    OwnerUnqualified,
    /// Evidence state is unknown or not assessable.
    Unknown,
}

/// One bitemporal dependency-change event.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChangeEvent {
    /// Unique event identity.
    pub event_id: String,
    /// Owner/source namespace.
    pub source_id: String,
    /// Monotonic sequence number within the source stream.
    pub source_sequence: u64,
    /// Dependency dimension changed.
    pub dependency: DependencyKind,
    /// Exact prior dependency digest.
    pub prior_digest: String,
    /// Exact successor dependency digest.
    pub successor_digest: String,
    /// Effective time: when the change took effect.
    pub effective_at: i64,
    /// Recorded time: when this event entered the evidence system.
    pub recorded_at: i64,
    /// Owner-declared semantic change class.
    pub change_class: ChangeClass,
    /// Owner-declared source qualification.
    pub evidence_state: ChangeEvidenceState,
    /// Event commitment string; syntax only, not cryptographic verification.
    pub event_sha256: String,
}

/// Completeness claim for one dependency-change feed and time frontier.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChangeFeedCoverage {
    /// Dependency dimension covered.
    pub dependency: DependencyKind,
    /// Exact source namespace.
    pub source_id: String,
    /// Inclusive start of the covered effective-time interval.
    pub effective_from: i64,
    /// Inclusive end of the covered effective-time interval.
    pub effective_through: i64,
    /// Earliest recorded/knowledge time for which the feed retains complete history.
    pub knowledge_from: i64,
    /// Latest recorded/knowledge time for which the feed asserts completeness.
    pub knowledge_through: i64,
    /// First source sequence included in this coverage statement.
    pub first_sequence: u64,
    /// Last source sequence included in this coverage statement.
    pub last_sequence: u64,
    /// Knowledge/record time at which this coverage manifest became available.
    pub recorded_at: i64,
    /// Source's completeness assertion.
    pub completeness: CoverageCompleteness,
    /// Owner-declared evidence state for the coverage receipt.
    pub evidence_state: ChangeEvidenceState,
    /// Coverage manifest commitment; syntax only.
    pub manifest_sha256: String,
}

/// Declared completeness of one change feed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CoverageCompleteness {
    /// Complete for the stated effective-time and knowledge-time frontiers.
    Complete,
    /// Known partial coverage; omitted changes cannot be ruled out.
    Partial,
    /// Coverage completeness is unknown.
    Unknown,
}

/// Two independent cutoffs for reproducible historical/current evaluation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BitemporalCutoff {
    /// Target effective-time frontier being assessed.
    pub effective_at: i64,
    /// Knowledge-time cutoff: only events recorded by this time may be considered.
    pub known_at: i64,
}

/// Per-claim reliance status at the requested bitemporal cutoff.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClaimStatus {
    /// Complete change coverage and no applicable material change was found.
    StillApplicable,
    /// Evidence remains a valid historical record but a retirement superseded current reliance.
    HistoricalOnly,
    /// A relevant material change occurred after the claim and requalification is needed.
    RequalificationRequired,
    /// A corrective source event contradicts a dependency used by the claim.
    Contradicted,
    /// Feed coverage, event qualification or dependency lineage is incomplete.
    Indeterminate,
    /// A relevant event class is not defined by this frozen profile.
    UnsupportedChangeType,
}

/// Typed structural failure in an event/coverage input set.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvaluationError {
    /// Claim identity, digest, dependency set or time metadata is malformed.
    InvalidClaim,
    /// The claim did not yet exist at one or both requested historical cutoffs.
    ClaimNotAvailableAtCutoff,
    /// Duplicate dependency dimension or empty change-source identity.
    InvalidDependencySet,
    /// Duplicate event identity.
    DuplicateEventId,
    /// A source sequence is duplicated or forked.
    ForkedSourceSequence,
    /// Event identity, digest or time metadata is malformed.
    InvalidEvent,
    /// Duplicate coverage for a dependency/source pair.
    DuplicateCoverage,
    /// Coverage interval, sequence range or digest metadata is malformed.
    InvalidCoverage,
    /// Two eligible events for one dependency form a broken digest chain.
    DependencyChainMismatch,
    /// A change event's source does not match the claim's declared source.
    ChangeSourceMismatch,
}

/// Per-claim result; historical evidence is never deleted or rewritten.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClaimAssessment {
    /// Claim being assessed.
    pub claim_id: String,
    /// Exact model-validation package digest for the claim.
    pub validation_package_sha256: String,
    /// Effective-time cutoff used for this assessment.
    pub effective_cutoff: i64,
    /// Knowledge-time cutoff used for this assessment.
    pub knowledge_cutoff: i64,
    /// Exact change-impact policy digest used for this assessment.
    pub policy_sha256: String,
    /// Status at this effective-time and knowledge-time cutoff.
    pub status: ClaimStatus,
    /// Dependency dimensions with known relevant change events.
    pub affected_dimensions: Vec<DependencyKind>,
    /// Exact event IDs relied on to explain the result.
    pub event_ids: Vec<String>,
    /// Exact dimensions requiring owner-defined requalification.
    pub requalification_dimensions: Vec<DependencyKind>,
    /// Dimensions with a known corrective event contradicting the historical claim.
    pub contradicted_dimensions: Vec<DependencyKind>,
    /// Dimensions retired/superseded for current reliance while historical evidence is retained.
    pub historical_only_dimensions: Vec<DependencyKind>,
    /// Dimensions affected by an unsupported change semantic.
    pub unsupported_dimensions: Vec<DependencyKind>,
    /// Dependency dimensions whose feed coverage is incomplete/unknown.
    pub indeterminate_dimensions: Vec<DependencyKind>,
    /// Exact current dependency digest snapshot after eligible material changes.
    pub evaluated_dependencies: Vec<DependencyBinding>,
}

/// Frozen policy for the aviation-specific change-impact oracle.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RequalificationPolicy {
    /// Digest of the versioned dependency and change-class mapping.
    pub policy_sha256: String,
}

/// Check only digest syntax. Hashing/authentication is owned by the custody layer.
fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn dependency_is_complete(dependency: &DependencyBinding) -> bool {
    is_sha256(&dependency.digest) && !dependency.change_source_id.trim().is_empty()
}

fn validate_claim(claim: &ModelEvidenceClaim) -> Result<(), EvaluationError> {
    if claim.claim_id.trim().is_empty()
        || !is_sha256(&claim.validation_package_sha256)
        || claim.dependencies.is_empty()
    {
        return Err(EvaluationError::InvalidClaim);
    }
    let mut dimensions = BTreeSet::new();
    for dependency in &claim.dependencies {
        if !dependency_is_complete(dependency) || !dimensions.insert(dependency.kind) {
            return Err(EvaluationError::InvalidDependencySet);
        }
    }
    Ok(())
}

fn validate_events(events: &[ChangeEvent]) -> Result<(), EvaluationError> {
    let mut ids = BTreeSet::new();
    let mut sequences: BTreeMap<(&str, u64), (&str, &str)> = BTreeMap::new();
    for event in events {
        if event.event_id.trim().is_empty()
            || event.source_id.trim().is_empty()
            || !is_sha256(&event.prior_digest)
            || !is_sha256(&event.successor_digest)
            || !is_sha256(&event.event_sha256)
            || event.prior_digest == event.successor_digest
            || event.recorded_at < 0
        {
            return Err(EvaluationError::InvalidEvent);
        }
        if !ids.insert(event.event_id.as_str()) {
            return Err(EvaluationError::DuplicateEventId);
        }
        let key = (event.source_id.as_str(), event.source_sequence);
        if let Some((_, previous_digest)) =
            sequences.insert(key, (event.event_id.as_str(), event.event_sha256.as_str()))
        {
            // Sequence reuse is treated as equivocation even when payload digests match.
            let _same_digest = previous_digest == event.event_sha256;
            return Err(EvaluationError::ForkedSourceSequence);
        }
    }
    Ok(())
}

fn validate_coverage(coverage: &[ChangeFeedCoverage]) -> Result<(), EvaluationError> {
    let mut keys = BTreeSet::new();
    for item in coverage {
        if item.source_id.trim().is_empty()
            || item.recorded_at < 0
            || item.knowledge_from > item.knowledge_through
            || !is_sha256(&item.manifest_sha256)
            || item.effective_from > item.effective_through
            || item.first_sequence > item.last_sequence
        {
            return Err(EvaluationError::InvalidCoverage);
        }
        if !keys.insert((item.dependency, item.source_id.as_str())) {
            return Err(EvaluationError::DuplicateCoverage);
        }
    }
    Ok(())
}

fn coverage_is_complete(
    coverage: &[ChangeFeedCoverage],
    dependency: &DependencyBinding,
    cutoff: BitemporalCutoff,
    claim_validated_at: i64,
    claim_recorded_at: i64,
) -> bool {
    coverage.iter().any(|item| {
        item.dependency == dependency.kind
            && item.source_id == dependency.change_source_id
            && item.completeness == CoverageCompleteness::Complete
            && item.evidence_state == ChangeEvidenceState::OwnerQualified
            && item.effective_from <= claim_validated_at
            && item.effective_through >= cutoff.effective_at
            && item.knowledge_from <= claim_recorded_at
            && item.knowledge_through >= cutoff.known_at
    })
}

/// Evaluate one claim under separate effective-time and knowledge-time cutoffs.
///
/// A late-recorded event affects a later knowledge cutoff but does not rewrite an
/// earlier "as-known-at" assessment. No generic currentness or lease algebra is used.
pub fn evaluate_claim(
    claim: &ModelEvidenceClaim,
    events: &[ChangeEvent],
    coverage: &[ChangeFeedCoverage],
    cutoff: BitemporalCutoff,
    policy: &RequalificationPolicy,
) -> Result<ClaimAssessment, EvaluationError> {
    validate_claim(claim)?;
    if claim.validated_at_effective > cutoff.effective_at || claim.recorded_at > cutoff.known_at {
        return Err(EvaluationError::ClaimNotAvailableAtCutoff);
    }
    let eligible_events: Vec<ChangeEvent> = events
        .iter()
        .filter(|event| {
            event.effective_at <= cutoff.effective_at && event.recorded_at <= cutoff.known_at
        })
        .cloned()
        .collect();
    // Events outside either requested cutoff were not part of this historical view.
    // They must not poison an earlier as-known assessment.
    validate_events(&eligible_events)?;
    let eligible_coverage: Vec<ChangeFeedCoverage> = coverage
        .iter()
        .filter(|item| item.recorded_at <= cutoff.known_at)
        .cloned()
        .collect();
    // A future-recorded coverage correction cannot change an earlier as-known view.
    validate_coverage(&eligible_coverage)?;
    if !is_sha256(&policy.policy_sha256) {
        return Err(EvaluationError::InvalidClaim);
    }

    let mut affected = BTreeSet::new();
    let mut requalification = BTreeSet::new();
    let mut contradicted = BTreeSet::new();
    let mut historical_only = BTreeSet::new();
    let mut unsupported = BTreeSet::new();
    let mut indeterminate = BTreeSet::new();
    let mut event_ids = BTreeSet::new();
    let mut evaluated_dependencies = claim.dependencies.clone();

    // Source sequence controls causal digest succession; effective and recorded times
    // only control whether an event is visible at the requested bitemporal cutoff.
    let mut ordered_events: Vec<&ChangeEvent> = eligible_events.iter().collect();
    ordered_events.sort_by(|left, right| {
        (
            left.source_id.as_str(),
            left.source_sequence,
            left.event_id.as_str(),
        )
            .cmp(&(
                right.source_id.as_str(),
                right.source_sequence,
                right.event_id.as_str(),
            ))
    });

    for dependency in &claim.dependencies {
        if !coverage_is_complete(
            &eligible_coverage,
            dependency,
            cutoff,
            claim.validated_at_effective,
            claim.recorded_at,
        ) {
            indeterminate.insert(dependency.kind);
        }

        let mut active_digest = dependency.digest.clone();
        let mut dimension_events: Vec<&ChangeEvent> = ordered_events
            .iter()
            .copied()
            .filter(|event| event.dependency == dependency.kind)
            .collect();
        let mut previous_baseline_successor: Option<String> = None;

        for event in dimension_events.drain(..) {
            if event.source_id != dependency.change_source_id {
                return Err(EvaluationError::ChangeSourceMismatch);
            }
            let event_is_covered = eligible_coverage.iter().any(|item| {
                item.dependency == dependency.kind
                    && item.source_id == dependency.change_source_id
                    && item.first_sequence <= event.source_sequence
                    && item.last_sequence >= event.source_sequence
            });
            if !event_is_covered {
                indeterminate.insert(dependency.kind);
                affected.insert(dependency.kind);
                event_ids.insert(event.event_id.clone());
            }
            if event.evidence_state != ChangeEvidenceState::OwnerQualified {
                indeterminate.insert(dependency.kind);
                event_ids.insert(event.event_id.clone());
                affected.insert(dependency.kind);
                continue;
            }

            // Events already both effective and recorded when the original claim
            // was created belong to the baseline. The claim must bind their latest
            // successor digest or its own starting state is already contradicted.
            let part_of_claim_baseline = event.effective_at <= claim.validated_at_effective
                && event.recorded_at <= claim.recorded_at;
            if part_of_claim_baseline {
                if let Some(previous) = previous_baseline_successor.as_ref()
                    && event.prior_digest != *previous
                {
                    indeterminate.insert(dependency.kind);
                    affected.insert(dependency.kind);
                    event_ids.insert(event.event_id.clone());
                }
                previous_baseline_successor = Some(event.successor_digest.clone());
                active_digest = event.successor_digest.clone();
                continue;
            }

            if event.prior_digest != active_digest {
                indeterminate.insert(dependency.kind);
                affected.insert(dependency.kind);
                event_ids.insert(event.event_id.clone());
                continue;
            }

            active_digest = event.successor_digest.clone();
            affected.insert(dependency.kind);
            event_ids.insert(event.event_id.clone());
            match event.change_class {
                ChangeClass::MaterialChange => {
                    requalification.insert(dependency.kind);
                }
                ChangeClass::CorrectiveChange => {
                    contradicted.insert(dependency.kind);
                }
                ChangeClass::Retirement => {
                    historical_only.insert(dependency.kind);
                }
                ChangeClass::Unsupported => {
                    unsupported.insert(dependency.kind);
                }
            }
        }

        // If a relevant change was already known and effective before claim issue,
        // its digest must match the dependency snapshot the claim says it used.
        let latest_baseline = ordered_events
            .iter()
            .copied()
            .filter(|event| {
                event.dependency == dependency.kind
                    && event.source_id == dependency.change_source_id
                    && event.effective_at <= claim.validated_at_effective
                    && event.recorded_at <= claim.recorded_at
                    && event.evidence_state == ChangeEvidenceState::OwnerQualified
            })
            .max_by_key(|event| event.source_sequence);
        if let Some(event) = latest_baseline
            && event.successor_digest != dependency.digest
        {
            contradicted.insert(dependency.kind);
            affected.insert(dependency.kind);
            event_ids.insert(event.event_id.clone());
        }

        if let Some(current) = evaluated_dependencies
            .iter_mut()
            .find(|item| item.kind == dependency.kind)
        {
            current.digest = active_digest;
        }
    }

    let status = if !contradicted.is_empty() {
        ClaimStatus::Contradicted
    } else if !indeterminate.is_empty() {
        ClaimStatus::Indeterminate
    } else if !unsupported.is_empty() {
        ClaimStatus::UnsupportedChangeType
    } else if !historical_only.is_empty() {
        ClaimStatus::HistoricalOnly
    } else if !requalification.is_empty() {
        ClaimStatus::RequalificationRequired
    } else {
        ClaimStatus::StillApplicable
    };

    evaluated_dependencies.sort_by_key(|dependency| dependency.kind);
    Ok(ClaimAssessment {
        claim_id: claim.claim_id.clone(),
        validation_package_sha256: claim.validation_package_sha256.clone(),
        effective_cutoff: cutoff.effective_at,
        knowledge_cutoff: cutoff.known_at,
        policy_sha256: policy.policy_sha256.clone(),
        status,
        affected_dimensions: affected.into_iter().collect(),
        event_ids: event_ids.into_iter().collect(),
        requalification_dimensions: requalification.into_iter().collect(),
        contradicted_dimensions: contradicted.into_iter().collect(),
        historical_only_dimensions: historical_only.into_iter().collect(),
        unsupported_dimensions: unsupported.into_iter().collect(),
        indeterminate_dimensions: indeterminate.into_iter().collect(),
        evaluated_dependencies,
    })
}
