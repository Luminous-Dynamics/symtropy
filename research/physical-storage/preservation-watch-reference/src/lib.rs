//! Deterministic preservation-watch semantics and non-executing action proposals.
//!
//! This crate does not collect physical observations, verify their signatures,
//! measure media, execute remediation, or predict lifetime. It evaluates
//! externally produced, provenance-labelled observations and emits proposals
//! only. No proposal grants authority to mutate an archive or operate hardware.

use std::collections::{BTreeMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WatchSignal {
    Fixity,
    MediaRead,
    Calibration,
    ReaderAvailability,
    ReaderIndependence,
    TrustAnchor,
    SignatureValidity,
    RevocationFreshness,
    PreservationObjective,
    RepresentationInformation,
    IndependentCopies,
    CommonCause,
    TechnologySupport,
    EnvironmentEnvelope,
    ReadMargin,
    DependencyPolicy,
    SandboxPolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObservationState {
    Alert,
    Healthy,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceProvenance {
    Verified,
    Unverified,
    Invalid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatchDisposition {
    Stable,
    Watch,
    InterventionRequired,
    Blocked,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ProposedAction {
    VerifyFixity,
    IsolateSuspectCopy,
    RecoverFromIndependentCopy,
    RecalibrateReader,
    ReplaceReader,
    ObtainTrustAnchor,
    RefreshRevocationEvidence,
    RestoreRepresentationInformation,
    RunPreservationObjective,
    AssessFailureDomains,
    CreateIndependentCopy,
    InspectMedia,
    RunControlledReadTrial,
    PlanMigration,
    VerifyRuntimeDependencies,
    HumanReview,
    GatherFreshEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WatchObservation {
    pub observation_id: String,
    pub target_id: String,
    pub signal: WatchSignal,
    pub sequence: u64,
    pub state: ObservationState,
    pub observed_at_unix: u64,
    pub valid_until_unix: u64,
    pub evidence_sha256: String,
    pub observer_id: String,
    pub provenance: EvidenceProvenance,
    /// A Healthy observation that clears an earlier Alert must identify the
    /// alert it supersedes. Mere disappearance or elapsed time cannot clear it.
    pub supersedes_observation_id: Option<String>,
    /// Must be supplied for IndependentCopies observations; measured by the
    /// independent dependency/failure-domain authority, not inferred here.
    pub observed_independent_copies: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WatchPolicy {
    pub required_signals: Vec<WatchSignal>,
    /// This is supplied by the independently assessed copy/dependency authority.
    /// The watch kernel does not infer independence from replica count alone.
    pub minimum_independent_copies: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionProposal {
    pub target_id: String,
    pub signal: WatchSignal,
    pub action: ProposedAction,
    pub trigger_observation_id: String,
    pub required_approvals: Vec<String>,
    pub required_evidence: Vec<String>,
    pub auto_execute: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignalResult {
    pub signal: WatchSignal,
    pub state: ObservationState,
    pub evidence_fresh: bool,
    pub source_verified: bool,
    pub latest_observation_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WatchReceipt {
    pub target_id: String,
    pub evaluated_at_unix: u64,
    pub disposition: WatchDisposition,
    pub signal_results: Vec<SignalResult>,
    pub action_proposals: Vec<ActionProposal>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatchContractError {
    EmptyTargetId,
    EmptyRequiredSignals,
    DuplicateRequiredSignal,
    ZeroMinimumIndependentCopies,
    CopyThresholdWithoutSignal,
    MissingIndependentCopyCount,
    CopyCountOnWrongSignal,
    InconsistentIndependentCopyEvidence,
    EmptyObservationId,
    DuplicateObservationId,
    EmptyObserverId,
    EmptyEvidenceDigest,
    InvalidEvidenceDigest,
    InvalidObservationTime,
    InvalidValidityWindow,
    DuplicateSignalSequence,
    NonMonotonicSignalHistory,
    UnsubstantiatedClear,
    SupersedesWrongTargetOrSignal,
    HealthyStateWithAlertOnlyProvenance,
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn action_plan(signal: WatchSignal) -> (WatchDisposition, Vec<ProposedAction>, Vec<&'static str>) {
    use ProposedAction as A;
    use WatchDisposition as D;
    use WatchSignal as S;
    match signal {
        S::Fixity => (
            D::Blocked,
            vec![A::IsolateSuspectCopy, A::VerifyFixity, A::RecoverFromIndependentCopy, A::HumanReview],
            vec!["scorer-held expected digest", "independent copy provenance", "recovery receipt"],
        ),
        S::MediaRead => (
            D::Blocked,
            vec![A::InspectMedia, A::RunControlledReadTrial, A::RecoverFromIndependentCopy],
            vec!["raw reader observations", "instrument calibration", "specimen identity"],
        ),
        S::Calibration => (
            D::Blocked,
            vec![A::RecalibrateReader, A::ReplaceReader, A::HumanReview],
            vec!["current calibration record", "reader and firmware identity", "calibration reference provenance"],
        ),
        S::ReaderAvailability => (
            D::Blocked,
            vec![A::ReplaceReader, A::RunControlledReadTrial],
            vec!["qualified reader package", "independent output comparison"],
        ),
        S::ReaderIndependence => (
            D::InterventionRequired,
            vec![A::AssessFailureDomains, A::ReplaceReader, A::HumanReview],
            vec!["transitive dependency graph", "common-cause cut sets", "two-reader comparison"],
        ),
        S::TrustAnchor => (
            D::Unknown,
            vec![A::ObtainTrustAnchor, A::HumanReview],
            vec!["out-of-band trust-root provenance", "trust policy version", "custody evidence"],
        ),
        S::SignatureValidity => (
            D::Blocked,
            vec![A::IsolateSuspectCopy, A::HumanReview],
            vec!["signature validation material", "independent trust root", "provenance chain"],
        ),
        S::RevocationFreshness => (
            D::Unknown,
            vec![A::RefreshRevocationEvidence, A::HumanReview],
            vec!["fresh revocation evidence", "historical validation record", "trust policy"],
        ),
        S::PreservationObjective => (
            D::InterventionRequired,
            vec![A::RestoreRepresentationInformation, A::RunPreservationObjective],
            vec!["failed objective definition", "representation information", "independent objective result"],
        ),
        S::RepresentationInformation => (
            D::InterventionRequired,
            vec![A::RestoreRepresentationInformation, A::PlanMigration],
            vec!["format specification", "schema and units", "independent render/interpretation result"],
        ),
        S::IndependentCopies => (
            D::InterventionRequired,
            vec![A::CreateIndependentCopy, A::AssessFailureDomains],
            vec!["copy fixity receipt", "failure-domain assessment", "custody and placement evidence"],
        ),
        S::CommonCause => (
            D::InterventionRequired,
            vec![A::AssessFailureDomains, A::CreateIndependentCopy],
            vec!["dependency graph", "common-cause analysis", "independent-copy receipt"],
        ),
        S::TechnologySupport => (
            D::Watch,
            vec![A::PlanMigration, A::RunControlledReadTrial],
            vec!["reader/vendor support evidence", "migration rehearsal", "format portability assessment"],
        ),
        S::EnvironmentEnvelope => (
            D::InterventionRequired,
            vec![A::InspectMedia, A::RunControlledReadTrial, A::HumanReview],
            vec!["calibrated environmental measurement", "specimen and batch identity", "measurement uncertainty"],
        ),
        S::ReadMargin => (
            D::InterventionRequired,
            vec![A::RunControlledReadTrial, A::InspectMedia, A::PlanMigration],
            vec!["raw read measurements", "calibration provenance", "independent decode result"],
        ),
        S::DependencyPolicy => (
            D::Blocked,
            vec![A::VerifyRuntimeDependencies, A::HumanReview],
            vec!["observed runtime dependency trace", "declared dependency manifest", "offline run receipt"],
        ),
        S::SandboxPolicy => (
            D::Blocked,
            vec![A::IsolateSuspectCopy, A::VerifyRuntimeDependencies, A::HumanReview],
            vec!["sandbox event log", "host-boundary policy", "independent incident review"],
        ),
    }
}

fn make_proposals(
    target_id: &str,
    observation: &WatchObservation,
    actions: &[ProposedAction],
    requirements: &[&str],
) -> Vec<ActionProposal> {
    actions.iter().map(|action| ActionProposal {
        target_id: target_id.to_string(),
        signal: observation.signal,
        action: *action,
        trigger_observation_id: observation.observation_id.clone(),
        required_approvals: vec!["archive_owner".into(), "independent_reviewer".into()],
        required_evidence: requirements.iter().map(|value| (*value).to_string()).collect(),
        auto_execute: false,
    }).collect()
}

/// Evaluate a time-bounded evidence snapshot.
///
/// Multiple observations for the same target/signal are accepted as a short
/// ordered history. Sequence and observation time must increase. A Healthy
/// observation can clear an earlier Alert only when it points to that alert
/// and is backed by current, verified evidence.
pub fn evaluate_watch(
    target_id: &str,
    evaluated_at_unix: u64,
    policy: &WatchPolicy,
    observations: &[WatchObservation],
) -> Result<WatchReceipt, WatchContractError> {
    if target_id.trim().is_empty() {
        return Err(WatchContractError::EmptyTargetId);
    }
    if policy.required_signals.is_empty() {
        return Err(WatchContractError::EmptyRequiredSignals);
    }
    let copies_required = policy.required_signals.contains(&WatchSignal::IndependentCopies);
    if copies_required && policy.minimum_independent_copies == 0 {
        return Err(WatchContractError::ZeroMinimumIndependentCopies);
    }
    if !copies_required && policy.minimum_independent_copies > 0 {
        return Err(WatchContractError::CopyThresholdWithoutSignal);
    }
    if policy.required_signals.iter().enumerate().any(
        |(index, signal)| policy.required_signals[..index].contains(signal)
    ) {
        return Err(WatchContractError::DuplicateRequiredSignal);
    }

    let mut observation_ids = HashSet::with_capacity(observations.len());
    let mut grouped: BTreeMap<WatchSignal, Vec<&WatchObservation>> = BTreeMap::new();

    for observation in observations {
        if observation.observation_id.trim().is_empty() {
            return Err(WatchContractError::EmptyObservationId);
        }
        if !observation_ids.insert(observation.observation_id.as_str()) {
            return Err(WatchContractError::DuplicateObservationId);
        }
        if observation.target_id.trim().is_empty() {
            return Err(WatchContractError::EmptyTargetId);
        }
        if observation.observer_id.trim().is_empty() {
            return Err(WatchContractError::EmptyObserverId);
        }
        if observation.evidence_sha256.is_empty() {
            return Err(WatchContractError::EmptyEvidenceDigest);
        }
        if !is_sha256_hex(&observation.evidence_sha256) {
            return Err(WatchContractError::InvalidEvidenceDigest);
        }
        if observation.observed_at_unix > evaluated_at_unix {
            return Err(WatchContractError::InvalidObservationTime);
        }
        if observation.valid_until_unix < observation.observed_at_unix {
            return Err(WatchContractError::InvalidValidityWindow);
        }
        if observation.state == ObservationState::Healthy
            && observation.provenance != EvidenceProvenance::Verified
        {
            return Err(WatchContractError::HealthyStateWithAlertOnlyProvenance);
        }
        if observation.signal == WatchSignal::IndependentCopies {
            if !copies_required {
                return Err(WatchContractError::CopyThresholdWithoutSignal);
            }
            if observation.observed_independent_copies.is_none() {
                return Err(WatchContractError::MissingIndependentCopyCount);
            }
        } else if observation.observed_independent_copies.is_some() {
            return Err(WatchContractError::CopyCountOnWrongSignal);
        }
        if observation.target_id == target_id {
            grouped.entry(observation.signal).or_default().push(observation);
        }
    }

    let mut signal_results = Vec::new();
    let mut proposals = Vec::new();
    let mut dispositions = Vec::new();

    for signal in &policy.required_signals {
        let Some(history) = grouped.get(signal) else {
            signal_results.push(SignalResult {
                signal: *signal,
                state: ObservationState::Unknown,
                evidence_fresh: false,
                source_verified: false,
                latest_observation_id: None,
            });
            dispositions.push(WatchDisposition::Unknown);
            let (_, actions, requirements) = action_plan(*signal);
            let placeholder = WatchObservation {
                observation_id: "MISSING-OBSERVATION".into(),
                target_id: target_id.into(),
                signal: *signal,
                sequence: 0,
                state: ObservationState::Unknown,
                observed_at_unix: 0,
                valid_until_unix: 0,
                evidence_sha256: "0".repeat(64),
                observer_id: "watch-engine".into(),
                provenance: EvidenceProvenance::Unverified,
                supersedes_observation_id: None,
            };
            let mut missing_actions = actions;
            if !missing_actions.contains(&ProposedAction::GatherFreshEvidence) {
                missing_actions.push(ProposedAction::GatherFreshEvidence);
            }
            proposals.extend(make_proposals(target_id, &placeholder, &missing_actions, &requirements));
            continue;
        };

        let mut ordered = history.clone();
        ordered.sort_by_key(|observation| (observation.sequence, observation.observed_at_unix));
        for pair in ordered.windows(2) {
            if pair[0].sequence == pair[1].sequence {
                return Err(WatchContractError::DuplicateSignalSequence);
            }
            if pair[0].observed_at_unix > pair[1].observed_at_unix {
                return Err(WatchContractError::NonMonotonicSignalHistory);
            }
        }

        let mut last_alert: Option<&WatchObservation> = None;
        for observation in &ordered {
            if observation.state == ObservationState::Alert {
                last_alert = Some(observation);
            } else if observation.state == ObservationState::Healthy {
                if let Some(alert) = last_alert {
                    if observation.supersedes_observation_id.as_deref()
                        != Some(alert.observation_id.as_str())
                    {
                        return Err(WatchContractError::UnsubstantiatedClear);
                    }
                    if observation.provenance != EvidenceProvenance::Verified
                        || observation.valid_until_unix < evaluated_at_unix
                    {
                        return Err(WatchContractError::UnsubstantiatedClear);
                    }
                    if alert.target_id != observation.target_id || alert.signal != observation.signal {
                        return Err(WatchContractError::SupersedesWrongTargetOrSignal);
                    }
                    last_alert = None;
                }
            }
        }

        let latest = ordered.last().copied().expect("history is non-empty");
        let fresh = latest.valid_until_unix >= evaluated_at_unix;
        let source_verified = latest.provenance == EvidenceProvenance::Verified;
        let state = if latest.provenance == EvidenceProvenance::Invalid {
            ObservationState::Alert
        } else if !fresh || !source_verified {
            ObservationState::Unknown
        } else if *signal == WatchSignal::IndependentCopies {
            let observed = latest.observed_independent_copies
                .ok_or(WatchContractError::MissingIndependentCopyCount)?;
            let below_policy = observed < policy.minimum_independent_copies;
            if below_policy && latest.state == ObservationState::Healthy {
                return Err(WatchContractError::InconsistentIndependentCopyEvidence);
            }
            if !below_policy && latest.state == ObservationState::Alert {
                return Err(WatchContractError::InconsistentIndependentCopyEvidence);
            }
            if below_policy { ObservationState::Alert } else { ObservationState::Healthy }
        } else {
            latest.state
        };

        signal_results.push(SignalResult {
            signal: *signal,
            state,
            evidence_fresh: fresh,
            source_verified,
            latest_observation_id: Some(latest.observation_id.clone()),
        });

        if state == ObservationState::Alert {
            let (disposition, actions, requirements) = if latest.provenance == EvidenceProvenance::Invalid {
                (WatchDisposition::Blocked, vec![ProposedAction::IsolateSuspectCopy, ProposedAction::HumanReview], vec!["invalid evidence provenance", "independent investigation"])
            } else {
                action_plan(*signal)
            };
            dispositions.push(disposition);
            proposals.extend(make_proposals(target_id, latest, &actions, &requirements));
        } else if state == ObservationState::Unknown {
            dispositions.push(WatchDisposition::Unknown);
            let (_, mut actions, requirements) = action_plan(*signal);
            actions.push(ProposedAction::GatherFreshEvidence);
            proposals.extend(make_proposals(target_id, latest, &actions, &requirements));
        }
    }

    // Non-required active signals are still surfaced; they are never hidden
    // merely because the current policy omitted them from the mandatory set.
    for (signal, history) in &grouped {
        if policy.required_signals.contains(signal) {
            continue;
        }
        let mut ordered = history.clone();
        ordered.sort_by_key(|observation| (observation.sequence, observation.observed_at_unix));
        for pair in ordered.windows(2) {
            if pair[0].sequence == pair[1].sequence {
                return Err(WatchContractError::DuplicateSignalSequence);
            }
            if pair[0].observed_at_unix > pair[1].observed_at_unix {
                return Err(WatchContractError::NonMonotonicSignalHistory);
            }
        }
        let mut last_alert: Option<&WatchObservation> = None;
        for observation in &ordered {
            if observation.state == ObservationState::Alert {
                last_alert = Some(observation);
            } else if observation.state == ObservationState::Healthy {
                if let Some(alert) = last_alert {
                    if observation.supersedes_observation_id.as_deref()
                        != Some(alert.observation_id.as_str())
                        || observation.provenance != EvidenceProvenance::Verified
                        || observation.valid_until_unix < evaluated_at_unix
                    {
                        return Err(WatchContractError::UnsubstantiatedClear);
                    }
                    last_alert = None;
                }
            }
        }
        if let Some(latest) = ordered.last().copied() {
            let state = if latest.provenance == EvidenceProvenance::Invalid {
                ObservationState::Alert
            } else if latest.valid_until_unix < evaluated_at_unix
                || latest.provenance == EvidenceProvenance::Unverified
            {
                ObservationState::Unknown
            } else {
                latest.state
            };
            match state {
                ObservationState::Alert if latest.provenance == EvidenceProvenance::Invalid => {
                    dispositions.push(WatchDisposition::Blocked);
                    proposals.extend(make_proposals(
                        target_id,
                        latest,
                        &[ProposedAction::IsolateSuspectCopy, ProposedAction::HumanReview],
                        &["invalid evidence provenance", "independent investigation"],
                    ));
                }
                ObservationState::Alert => {
                    let (disposition, actions, requirements) = action_plan(*signal);
                    dispositions.push(disposition);
                    proposals.extend(make_proposals(target_id, latest, &actions, &requirements));
                }
                ObservationState::Unknown => {
                    dispositions.push(WatchDisposition::Unknown);
                    let (_, mut actions, requirements) = action_plan(*signal);
                    actions.push(ProposedAction::GatherFreshEvidence);
                    proposals.extend(make_proposals(target_id, latest, &actions, &requirements));
                }
                ObservationState::Healthy => {}
            }
        }
    }

    // Highest-severity disposition wins; Unknown cannot be silently treated
    // as stable but a confirmed blocker remains visible above it.
    let disposition = if dispositions.contains(&WatchDisposition::Blocked) {
        WatchDisposition::Blocked
    } else if dispositions.contains(&WatchDisposition::InterventionRequired) {
        WatchDisposition::InterventionRequired
    } else if dispositions.contains(&WatchDisposition::Unknown) {
        WatchDisposition::Unknown
    } else if dispositions.contains(&WatchDisposition::Watch) {
        WatchDisposition::Watch
    } else {
        WatchDisposition::Stable
    };

    proposals.sort_by_key(|proposal| (
        proposal.target_id.clone(),
        proposal.signal,
        proposal.action,
        proposal.trigger_observation_id.clone(),
    ));
    proposals.dedup_by(|left, right| {
        left.target_id == right.target_id
            && left.signal == right.signal
            && left.action == right.action
            && left.trigger_observation_id == right.trigger_observation_id
    });

    Ok(WatchReceipt {
        target_id: target_id.to_string(),
        evaluated_at_unix,
        disposition,
        signal_results,
        action_proposals: proposals,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> WatchPolicy {
        WatchPolicy {
            required_signals: vec![
                WatchSignal::Fixity,
                WatchSignal::Calibration,
                WatchSignal::TrustAnchor,
            ],
            minimum_independent_copies: 0,
        }
    }

    fn observation(
        id: &str,
        signal: WatchSignal,
        state: ObservationState,
        sequence: u64,
    ) -> WatchObservation {
        WatchObservation {
            observation_id: id.into(),
            target_id: "archive-A".into(),
            signal,
            sequence,
            state,
            observed_at_unix: 100 + sequence,
            valid_until_unix: 200,
            evidence_sha256: "a".repeat(64),
            observer_id: "independent-watcher".into(),
            provenance: EvidenceProvenance::Verified,
            supersedes_observation_id: None,
            observed_independent_copies: if signal == WatchSignal::IndependentCopies { Some(2) } else { None },
        }
    }

    fn healthy_snapshot() -> Vec<WatchObservation> {
        vec![
            observation("FIX-1", WatchSignal::Fixity, ObservationState::Healthy, 1),
            observation("CAL-1", WatchSignal::Calibration, ObservationState::Healthy, 1),
            observation("TRUST-1", WatchSignal::TrustAnchor, ObservationState::Healthy, 1),
        ]
    }

    #[test]
    fn current_verified_required_signals_are_stable() {
        let receipt = evaluate_watch("archive-A", 150, &policy(), &healthy_snapshot()).unwrap();
        assert_eq!(receipt.disposition, WatchDisposition::Stable);
        assert!(receipt.action_proposals.is_empty());
    }

    #[test]
    fn missing_required_observation_is_unknown_never_stable() {
        let mut observations = healthy_snapshot();
        observations.pop();
        let receipt = evaluate_watch("archive-A", 150, &policy(), &observations).unwrap();
        assert_eq!(receipt.disposition, WatchDisposition::Unknown);
        assert!(receipt.action_proposals.iter().any(
            |proposal| proposal.action == ProposedAction::GatherFreshEvidence
        ));
    }

    #[test]
    fn expired_observation_becomes_unknown() {
        let mut observations = healthy_snapshot();
        observations[0].valid_until_unix = 120;
        let receipt = evaluate_watch("archive-A", 150, &policy(), &observations).unwrap();
        assert_eq!(receipt.disposition, WatchDisposition::Unknown);
    }

    #[test]
    fn confirmed_fixity_mismatch_blocks_and_proposes_non_executing_recovery() {
        let mut observations = healthy_snapshot();
        observations[0].state = ObservationState::Alert;
        let receipt = evaluate_watch("archive-A", 150, &policy(), &observations).unwrap();
        assert_eq!(receipt.disposition, WatchDisposition::Blocked);
        assert!(receipt.action_proposals.iter().any(
            |proposal| proposal.action == ProposedAction::RecoverFromIndependentCopy
        ));
        assert!(receipt.action_proposals.iter().all(|proposal| !proposal.auto_execute));
    }

    #[test]
    fn technology_support_risk_maps_to_watch_not_blocked() {
        let mut observations = healthy_snapshot();
        observations.push(observation(
            "TECH-1",
            WatchSignal::TechnologySupport,
            ObservationState::Alert,
            1,
        ));
        let receipt = evaluate_watch("archive-A", 150, &policy(), &observations).unwrap();
        assert_eq!(receipt.disposition, WatchDisposition::Watch);
        assert!(receipt.action_proposals.iter().any(
            |proposal| proposal.action == ProposedAction::PlanMigration
        ));
    }

    #[test]
    fn stale_trust_evidence_is_unknown_and_requests_refresh() {
        let mut observations = healthy_snapshot();
        observations[2].valid_until_unix = 120;
        let receipt = evaluate_watch("archive-A", 150, &policy(), &observations).unwrap();
        assert_eq!(receipt.disposition, WatchDisposition::Unknown);
        assert!(receipt.action_proposals.iter().any(
            |proposal| proposal.action == ProposedAction::RefreshRevocationEvidence
        ));
    }

    #[test]
    fn verified_remediation_can_clear_a_prior_alert() {
        let alert = observation("FIX-ALERT", WatchSignal::Fixity, ObservationState::Alert, 1);
        let mut clear = observation("FIX-CLEAR", WatchSignal::Fixity, ObservationState::Healthy, 2);
        clear.supersedes_observation_id = Some("FIX-ALERT".into());
        let observations = vec![
            alert,
            clear,
            observation("CAL-1", WatchSignal::Calibration, ObservationState::Healthy, 1),
            observation("TRUST-1", WatchSignal::TrustAnchor, ObservationState::Healthy, 1),
        ];
        let receipt = evaluate_watch("archive-A", 150, &policy(), &observations).unwrap();
        assert_eq!(receipt.disposition, WatchDisposition::Stable);
    }

    #[test]
    fn prior_alert_cannot_clear_without_explicit_supersession() {
        let alert = observation("FIX-ALERT", WatchSignal::Fixity, ObservationState::Alert, 1);
        let clear = observation("FIX-CLEAR", WatchSignal::Fixity, ObservationState::Healthy, 2);
        let observations = vec![
            alert, clear,
            observation("CAL-1", WatchSignal::Calibration, ObservationState::Healthy, 1),
            observation("TRUST-1", WatchSignal::TrustAnchor, ObservationState::Healthy, 1),
        ];
        assert_eq!(
            evaluate_watch("archive-A", 150, &policy(), &observations),
            Err(WatchContractError::UnsubstantiatedClear)
        );
    }

    #[test]
    fn wrong_or_unverified_provenance_never_yields_stable() {
        let mut observations = healthy_snapshot();
        observations[1].provenance = EvidenceProvenance::Unverified;
        let receipt = evaluate_watch("archive-A", 150, &policy(), &observations).unwrap();
        assert_eq!(receipt.disposition, WatchDisposition::Unknown);
    }

    #[test]
    fn invalid_provenance_is_a_blocker() {
        let mut observations = healthy_snapshot();
        observations[0].provenance = EvidenceProvenance::Invalid;
        let receipt = evaluate_watch("archive-A", 150, &policy(), &observations).unwrap();
        assert_eq!(receipt.disposition, WatchDisposition::Blocked);
    }

    #[test]
    fn observation_histories_must_have_monotonic_sequence_and_time() {
        let mut first = observation("FIX-1", WatchSignal::Fixity, ObservationState::Alert, 2);
        first.observed_at_unix = 110;
        let mut second = observation("FIX-2", WatchSignal::Fixity, ObservationState::Unknown, 1);
        second.observed_at_unix = 120;
        let observations = vec![
            first, second,
            observation("CAL-1", WatchSignal::Calibration, ObservationState::Healthy, 1),
            observation("TRUST-1", WatchSignal::TrustAnchor, ObservationState::Healthy, 1),
        ];
        assert_eq!(
            evaluate_watch("archive-A", 150, &policy(), &observations),
            Err(WatchContractError::NonMonotonicSignalHistory)
        );
    }

    #[test]
    fn malformed_and_duplicate_evidence_is_rejected() {
        let mut empty_id = healthy_snapshot();
        empty_id[0].evidence_sha256 = "x".into();
        assert_eq!(
            evaluate_watch("archive-A", 150, &policy(), &empty_id),
            Err(WatchContractError::InvalidEvidenceDigest)
        );

        let mut duplicate = healthy_snapshot();
        duplicate[1].observation_id = duplicate[0].observation_id.clone();
        assert_eq!(
            evaluate_watch("archive-A", 150, &policy(), &duplicate),
            Err(WatchContractError::DuplicateObservationId)
        );
    }

    #[test]
    fn future_and_invalidity_windows_are_rejected() {
        let mut future = healthy_snapshot();
        future[0].observed_at_unix = 999;
        assert_eq!(
            evaluate_watch("archive-A", 150, &policy(), &future),
            Err(WatchContractError::InvalidObservationTime)
        );

        let mut invalid_window = healthy_snapshot();
        invalid_window[0].valid_until_unix = 90;
        assert_eq!(
            evaluate_watch("archive-A", 150, &policy(), &invalid_window),
            Err(WatchContractError::InvalidValidityWindow)
        );
    }

    #[test]
    fn independent_copy_threshold_is_enforced_from_measured_count() {
        let mut copy_policy = policy();
        copy_policy.required_signals.push(WatchSignal::IndependentCopies);
        copy_policy.minimum_independent_copies = 2;
        let mut observations = healthy_snapshot();
        let mut copies = observation(
            "COPIES-1",
            WatchSignal::IndependentCopies,
            ObservationState::Alert,
            1,
        );
        copies.observed_independent_copies = Some(1);
        observations.push(copies);
        let receipt = evaluate_watch("archive-A", 150, &copy_policy, &observations).unwrap();
        assert_eq!(receipt.disposition, WatchDisposition::InterventionRequired);
        assert!(receipt.action_proposals.iter().any(
            |proposal| proposal.action == ProposedAction::CreateIndependentCopy
        ));
    }

    #[test]
    fn copy_count_and_reported_state_must_not_contradict() {
        let mut copy_policy = policy();
        copy_policy.required_signals.push(WatchSignal::IndependentCopies);
        copy_policy.minimum_independent_copies = 2;
        let mut observations = healthy_snapshot();
        let mut copies = observation(
            "COPIES-1",
            WatchSignal::IndependentCopies,
            ObservationState::Healthy,
            1,
        );
        copies.observed_independent_copies = Some(1);
        observations.push(copies);
        assert_eq!(
            evaluate_watch("archive-A", 150, &copy_policy, &observations),
            Err(WatchContractError::InconsistentIndependentCopyEvidence)
        );
    }

    #[test]
    fn copy_threshold_requires_an_explicit_required_signal() {
        let mut invalid_policy = policy();
        invalid_policy.minimum_independent_copies = 2;
        assert_eq!(
            evaluate_watch("archive-A", 150, &invalid_policy, &healthy_snapshot()),
            Err(WatchContractError::CopyThresholdWithoutSignal)
        );

        let mut zero_policy = policy();
        zero_policy.required_signals.push(WatchSignal::IndependentCopies);
        assert_eq!(
            evaluate_watch("archive-A", 150, &zero_policy, &healthy_snapshot()),
            Err(WatchContractError::ZeroMinimumIndependentCopies)
        );
    }

    #[test]
    fn duplicate_required_signals_are_rejected() {
        let mut duplicate_policy = policy();
        duplicate_policy.required_signals.push(WatchSignal::Fixity);
        assert_eq!(
            evaluate_watch("archive-A", 150, &duplicate_policy, &healthy_snapshot()),
            Err(WatchContractError::DuplicateRequiredSignal)
        );
    }
}
