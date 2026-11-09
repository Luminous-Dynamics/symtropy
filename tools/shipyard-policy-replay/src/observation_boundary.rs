//! Capability-limited observation snapshots for online shipyard recovery policies.
//!
//! The candidate policy entry point accepts only an immutable ObservedSnapshot
//! and declared action bounds. Evaluator-only truth is kept on the evaluator
//! side of the call boundary. This is an API/type boundary, not an OS security
//! sandbox: untrusted policies require process isolation and a narrow IPC channel.

use std::collections::BTreeSet;

use crate::Tick;

/// Whether a source record is admissible to the policy's observation history.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    PolicyVisible,
    EvaluatorOnly,
}

/// Source-owned event record. Effective time describes when the underlying
/// event applies; observed time describes when the policy could first know it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceObservation {
    pub source_id: String,
    pub event_id: String,
    pub effective_at_tick: Tick,
    pub observed_at_tick: Tick,
    /// Lowercase SHA-256 content digest supplied by the source adapter.
    pub payload_sha256: String,
    pub visibility: Visibility,
}

/// Immutable source observation inventory for one run/scenario.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservationTape {
    records: Vec<SourceObservation>,
}

/// One policy-visible record in a decision snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotObservation {
    pub source_id: String,
    pub event_id: String,
    pub effective_at_tick: Tick,
    pub observed_at_tick: Tick,
    pub payload_sha256: String,
}

/// Immutable view of observations admitted by one decision epoch.
///
/// Fields are private: callers can inspect the snapshot but cannot append
/// observations or rewrite its decision time after construction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservedSnapshot {
    decision_tick: Tick,
    observations: Vec<SnapshotObservation>,
    canonical_key: String,
}

impl ObservedSnapshot {
    pub fn decision_tick(&self) -> Tick {
        self.decision_tick
    }

    pub fn observations(&self) -> &[SnapshotObservation] {
        &self.observations
    }

    /// Deterministic length-prefixed canonical key, not a cryptographic digest.
    /// A caller may hash these bytes with its approved digest implementation.
    pub fn canonical_key(&self) -> &str {
        &self.canonical_key
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObservationError {
    EmptySourceId,
    EmptyEventId,
    DuplicateEventId(String),
    InvalidPayloadDigest(String),
}

impl ObservationTape {
    /// Validate and freeze the source-owned observation inventory.
    pub fn new(records: Vec<SourceObservation>) -> Result<Self, ObservationError> {
        let mut ids = BTreeSet::new();
        for record in &records {
            if record.source_id.trim().is_empty() {
                return Err(ObservationError::EmptySourceId);
            }
            if record.event_id.trim().is_empty() {
                return Err(ObservationError::EmptyEventId);
            }
            if !ids.insert((record.source_id.as_str(), record.event_id.as_str())) {
                return Err(ObservationError::DuplicateEventId(record.event_id.clone()));
            }
            if !is_lowercase_sha256(&record.payload_sha256) {
                return Err(ObservationError::InvalidPayloadDigest(
                    record.event_id.clone(),
                ));
            }
        }
        Ok(Self { records })
    }

    /// Materialize only policy-visible records actually observed by decision_tick.
    ///
    /// An event can have an earlier effective time and still be excluded when
    /// it arrived late. Evaluator-only records never enter the returned object.
    pub fn snapshot_at(&self, decision_tick: Tick) -> ObservedSnapshot {
        let mut observations: Vec<SnapshotObservation> = self
            .records
            .iter()
            .filter(|record| {
                record.visibility == Visibility::PolicyVisible
                    && record.observed_at_tick <= decision_tick
            })
            .map(|record| SnapshotObservation {
                source_id: record.source_id.clone(),
                event_id: record.event_id.clone(),
                effective_at_tick: record.effective_at_tick,
                observed_at_tick: record.observed_at_tick,
                payload_sha256: record.payload_sha256.clone(),
            })
            .collect();

        observations.sort_by(|left, right| {
            left.source_id
                .cmp(&right.source_id)
                .then_with(|| left.event_id.cmp(&right.event_id))
                .then_with(|| left.effective_at_tick.cmp(&right.effective_at_tick))
                .then_with(|| left.observed_at_tick.cmp(&right.observed_at_tick))
                .then_with(|| left.payload_sha256.cmp(&right.payload_sha256))
        });

        let canonical_key = canonical_snapshot_key(decision_tick, &observations);
        ObservedSnapshot {
            decision_tick,
            observations,
            canonical_key,
        }
    }
}

fn is_lowercase_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

/// Length-prefix every string field to avoid delimiter ambiguity.
fn canonical_snapshot_key(decision_tick: Tick, observations: &[SnapshotObservation]) -> String {
    let mut key = format!(
        "observed-snapshot-v1|tick={decision_tick}|count={}|",
        observations.len()
    );
    for observation in observations {
        let effective = observation.effective_at_tick.to_string();
        let observed = observation.observed_at_tick.to_string();
        for field in [
            observation.source_id.as_str(),
            observation.event_id.as_str(),
            effective.as_str(),
            observed.as_str(),
            observation.payload_sha256.as_str(),
        ] {
            key.push_str(&format!("{}:", field.len()));
            key.push_str(field);
            key.push('|');
        }
    }
    key
}

/// Evaluator-only ground truth. This type is intentionally absent from the
/// RecoveryPolicy interface and the candidate decision function.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvaluatorTruth {
    pub scenario_id: String,
    pub future_outcome_keys: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionBounds {
    pub permitted_action_kinds: Vec<String>,
}

/// Candidate proposal only. There is deliberately no planner-supplied
/// feasible/verified/success flag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateAction {
    pub decision_tick: Tick,
    pub action_kind: String,
    pub canonical_action_key: String,
}

/// Minimal policy interface: no scenario ID, future tape or evaluator truth.
pub trait RecoveryPolicy {
    fn propose(&self, snapshot: &ObservedSnapshot, bounds: &ActionBounds) -> CandidateAction;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyDecisionReceipt {
    pub decision_tick: Tick,
    pub snapshot_key: String,
    pub policy_revision: String,
    pub action_kind: String,
    pub action_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecisionError {
    EmptyPolicyRevision,
    EmptyActionKind,
    EmptyActionKey,
    ActionTickMismatch,
    ActionKindOutsideBounds(String),
}

/// Invoke a policy using only its admitted snapshot and declared action bounds.
/// Feasibility and authority remain the responsibility of independent owners.
pub fn decide<P: RecoveryPolicy>(
    policy: &P,
    policy_revision: &str,
    snapshot: &ObservedSnapshot,
    bounds: &ActionBounds,
) -> Result<PolicyDecisionReceipt, DecisionError> {
    if policy_revision.trim().is_empty() {
        return Err(DecisionError::EmptyPolicyRevision);
    }
    let action = policy.propose(snapshot, bounds);
    if action.decision_tick != snapshot.decision_tick {
        return Err(DecisionError::ActionTickMismatch);
    }
    if action.action_kind.trim().is_empty() {
        return Err(DecisionError::EmptyActionKind);
    }
    if action.canonical_action_key.trim().is_empty() {
        return Err(DecisionError::EmptyActionKey);
    }
    if !bounds
        .permitted_action_kinds
        .iter()
        .any(|kind| kind == &action.action_kind)
    {
        return Err(DecisionError::ActionKindOutsideBounds(action.action_kind));
    }
    Ok(PolicyDecisionReceipt {
        decision_tick: snapshot.decision_tick,
        snapshot_key: snapshot.canonical_key.clone(),
        policy_revision: policy_revision.to_owned(),
        action_kind: action.action_kind,
        action_key: action.canonical_action_key,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const DIGEST_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const DIGEST_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

    fn record(
        event_id: &str,
        effective_at_tick: Tick,
        observed_at_tick: Tick,
        payload_sha256: &str,
        visibility: Visibility,
    ) -> SourceObservation {
        SourceObservation {
            source_id: "yard-event-feed".into(),
            event_id: event_id.into(),
            effective_at_tick,
            observed_at_tick,
            payload_sha256: payload_sha256.into(),
            visibility,
        }
    }

    struct WaitPolicy;

    impl RecoveryPolicy for WaitPolicy {
        fn propose(&self, snapshot: &ObservedSnapshot, _bounds: &ActionBounds) -> CandidateAction {
            CandidateAction {
                decision_tick: snapshot.decision_tick(),
                action_kind: "wait".into(),
                canonical_action_key: format!("wait-at-{}", snapshot.decision_tick()),
            }
        }
    }

    fn bounds() -> ActionBounds {
        ActionBounds {
            permitted_action_kinds: vec!["wait".into(), "dispatch".into()],
        }
    }

    #[test]
    fn future_observation_does_not_change_earlier_snapshot() {
        let before = ObservationTape::new(vec![record(
            "known",
            2,
            2,
            DIGEST_A,
            Visibility::PolicyVisible,
        )])
        .unwrap();
        let after = ObservationTape::new(vec![
            record("known", 2, 2, DIGEST_A, Visibility::PolicyVisible),
            record("future", 11, 11, DIGEST_B, Visibility::PolicyVisible),
        ])
        .unwrap();
        assert_eq!(
            before.snapshot_at(10).canonical_key(),
            after.snapshot_at(10).canonical_key()
        );
    }

    #[test]
    fn late_arriving_event_is_excluded_until_observed() {
        let tape = ObservationTape::new(vec![record(
            "late-correction",
            4,
            9,
            DIGEST_A,
            Visibility::PolicyVisible,
        )])
        .unwrap();
        assert!(tape.snapshot_at(8).observations().is_empty());
        let admitted = tape.snapshot_at(9);
        assert_eq!(admitted.observations().len(), 1);
        assert_eq!(admitted.observations()[0].effective_at_tick, 4);
        assert_eq!(admitted.observations()[0].observed_at_tick, 9);
    }

    #[test]
    fn evaluator_only_future_truth_never_enters_policy_snapshot() {
        let left = ObservationTape::new(vec![
            record("shared", 0, 0, DIGEST_A, Visibility::PolicyVisible),
            record("hidden-a", 15, 5, DIGEST_B, Visibility::EvaluatorOnly),
        ])
        .unwrap();
        let right = ObservationTape::new(vec![
            record("shared", 0, 0, DIGEST_A, Visibility::PolicyVisible),
            record("hidden-b", 15, 5, DIGEST_A, Visibility::EvaluatorOnly),
        ])
        .unwrap();
        assert_eq!(
            left.snapshot_at(10).canonical_key(),
            right.snapshot_at(10).canonical_key()
        );
        assert_eq!(left.snapshot_at(10).observations().len(), 1);
    }

    #[test]
    fn new_observation_changes_snapshot_only_after_admission_tick() {
        let base = ObservationTape::new(vec![record(
            "base",
            0,
            0,
            DIGEST_A,
            Visibility::PolicyVisible,
        )])
        .unwrap();
        let with_new = ObservationTape::new(vec![
            record("base", 0, 0, DIGEST_A, Visibility::PolicyVisible),
            record("new", 5, 7, DIGEST_B, Visibility::PolicyVisible),
        ])
        .unwrap();
        assert_eq!(
            base.snapshot_at(6).canonical_key(),
            with_new.snapshot_at(6).canonical_key()
        );
        assert_ne!(
            base.snapshot_at(7).canonical_key(),
            with_new.snapshot_at(7).canonical_key()
        );
    }

    #[test]
    fn snapshot_is_order_independent() {
        let first = ObservationTape::new(vec![
            record("a", 1, 1, DIGEST_A, Visibility::PolicyVisible),
            record("b", 2, 3, DIGEST_B, Visibility::PolicyVisible),
        ])
        .unwrap();
        let second = ObservationTape::new(vec![
            record("b", 2, 3, DIGEST_B, Visibility::PolicyVisible),
            record("a", 1, 1, DIGEST_A, Visibility::PolicyVisible),
        ])
        .unwrap();
        assert_eq!(
            first.snapshot_at(5).canonical_key(),
            second.snapshot_at(5).canonical_key()
        );
    }

    #[test]
    fn event_identity_is_qualified_by_source() {
        let mut first = record("shared-id", 1, 1, DIGEST_A, Visibility::PolicyVisible);
        let mut second = record("shared-id", 2, 2, DIGEST_B, Visibility::PolicyVisible);
        second.source_id = "crane-feed".into();
        first.source_id = "yard-feed".into();
        let tape = ObservationTape::new(vec![first, second]).unwrap();
        assert_eq!(tape.snapshot_at(3).observations().len(), 2);
    }

    #[test]
    fn policy_receipt_binds_exact_snapshot_and_policy_revision() {
        let tape = ObservationTape::new(vec![record(
            "known",
            1,
            1,
            DIGEST_A,
            Visibility::PolicyVisible,
        )])
        .unwrap();
        let snapshot = tape.snapshot_at(5);
        let receipt = decide(&WaitPolicy, "policy:abc123", &snapshot, &bounds()).unwrap();
        assert_eq!(receipt.decision_tick, 5);
        assert_eq!(receipt.snapshot_key, snapshot.canonical_key());
        assert_eq!(receipt.policy_revision, "policy:abc123");
        assert_eq!(receipt.action_kind, "wait");
    }

    #[test]
    fn out_of_bounds_action_is_rejected() {
        struct BadPolicy;
        impl RecoveryPolicy for BadPolicy {
            fn propose(
                &self,
                snapshot: &ObservedSnapshot,
                _bounds: &ActionBounds,
            ) -> CandidateAction {
                CandidateAction {
                    decision_tick: snapshot.decision_tick(),
                    action_kind: "unbounded-action".into(),
                    canonical_action_key: "action-1".into(),
                }
            }
        }
        let tape = ObservationTape::new(vec![]).unwrap();
        let result = decide(&BadPolicy, "rev-1", &tape.snapshot_at(0), &bounds());
        assert_eq!(
            result.unwrap_err(),
            DecisionError::ActionKindOutsideBounds("unbounded-action".into())
        );
    }

    #[test]
    fn invalid_observation_inventory_fails_closed() {
        let duplicate = ObservationTape::new(vec![
            record("same", 0, 0, DIGEST_A, Visibility::PolicyVisible),
            record("same", 1, 1, DIGEST_B, Visibility::PolicyVisible),
        ]);
        assert_eq!(
            duplicate.unwrap_err(),
            ObservationError::DuplicateEventId("same".into())
        );
        let bad_digest = ObservationTape::new(vec![record(
            "bad",
            0,
            0,
            "not-a-digest",
            Visibility::PolicyVisible,
        )]);
        assert_eq!(
            bad_digest.unwrap_err(),
            ObservationError::InvalidPayloadDigest("bad".into())
        );
    }

    #[test]
    fn mismatched_action_epoch_is_rejected() {
        struct WrongTickPolicy;
        impl RecoveryPolicy for WrongTickPolicy {
            fn propose(
                &self,
                _snapshot: &ObservedSnapshot,
                _bounds: &ActionBounds,
            ) -> CandidateAction {
                CandidateAction {
                    decision_tick: 99,
                    action_kind: "wait".into(),
                    canonical_action_key: "wait-99".into(),
                }
            }
        }
        let tape = ObservationTape::new(vec![]).unwrap();
        assert_eq!(
            decide(&WrongTickPolicy, "rev-1", &tape.snapshot_at(5), &bounds()).unwrap_err(),
            DecisionError::ActionTickMismatch
        );
    }
}
