// SPDX-License-Identifier: AGPL-3.0-or-later
//! Research-only gate for worksite observation sufficiency.
//! No sensor I/O, cryptographic verification, robot actuation or physical safety claim.

use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourcePolicy {
    pub source_id: String,
    /// Trusted source checkpoint installed by the upstream re-baseline process.
    pub baseline_sequence: u64,
    pub baseline_digest: String,
    pub required_cells: Vec<String>,
    pub max_age_ns: u64,
    pub max_clock_uncertainty_ns: u64,
    pub max_measurement_uncertainty_um: u64,
    pub requires_calibration: bool,
    pub requires_independent_reference: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GatePolicy {
    pub task_scope: String,
    pub expected_timebase_id: String,
    pub sources: Vec<SourcePolicy>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SensorHealth {
    Qualified,
    Degraded,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReferenceIndependence {
    NotRequired,
    IndependentUnderProfile,
    NotAssessed,
    CommonMode,
    Indeterminate,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Observation {
    pub source_id: String,
    pub task_scope: String,
    pub sequence: u64,
    /// SHA-256 identity asserted by the owning evidence pipeline.
    pub evidence_digest: String,
    /// Must match the previous accepted observation digest or policy baseline.
    pub predecessor_digest: String,
    pub timebase_id: String,
    pub observed_at_monotonic_ns: u64,
    pub clock_uncertainty_ns: u64,
    pub measurement_uncertainty_um: u64,
    pub health: SensorHealth,
    pub calibration_valid: bool,
    /// Upstream assertion only; this crate does not verify a signature.
    pub authenticated_by_upstream: bool,
    pub reference_independence: ReferenceIndependence,
    pub independence_assessment_ref: Option<String>,
    /// Cells explicitly observed clear. Absence from every list is unobserved.
    pub clear_cells: Vec<String>,
    /// Positive observations of occupancy, obstruction or another critical change.
    pub changed_cells: Vec<String>,
    /// Cells whose visibility/measurement coverage is occluded or unavailable.
    pub occluded_cells: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GateState {
    VerifiedClear,
    OccupiedOrChanged,
    UnknownOrUnobserved,
    Conflicting,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Evaluation {
    pub state: GateState,
    pub must_hold: bool,
    pub invalidate_epoch: bool,
    pub changed_cells: Vec<String>,
    pub conflicting_cells: Vec<String>,
    pub reason_codes: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IngestOutcome {
    Accepted,
    DuplicateIdempotent,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IngestError {
    InvalidPolicy(String),
    UnknownSource,
    MalformedEvidenceDigest,
    InitialSequenceGap,
    SequenceGap,
    SequenceRollback,
    SequenceDigestConflict,
    PredecessorDigestMismatch,
    SourceQuarantined,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ChannelState {
    Current(Box<Observation>),
    Unknown(String),
    Conflicting(String),
}

pub struct ObservationGate {
    policy: GatePolicy,
    channels: BTreeMap<String, ChannelState>,
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn unique_nonempty(values: &[String]) -> bool {
    let mut set = BTreeSet::new();
    values
        .iter()
        .all(|value| !value.trim().is_empty() && set.insert(value))
}

impl ObservationGate {
    pub fn new(policy: GatePolicy) -> Result<Self, IngestError> {
        if policy.task_scope.trim().is_empty() || policy.expected_timebase_id.trim().is_empty() {
            return Err(IngestError::InvalidPolicy(
                "task scope and expected timebase must be non-empty".to_owned(),
            ));
        }
        if policy.sources.is_empty() {
            return Err(IngestError::InvalidPolicy(
                "at least one required observation source is necessary".to_owned(),
            ));
        }
        let mut source_ids = BTreeSet::new();
        for source in &policy.sources {
            if source.source_id.trim().is_empty() || !source_ids.insert(source.source_id.as_str()) {
                return Err(IngestError::InvalidPolicy(
                    "source identifiers must be unique and non-empty".to_owned(),
                ));
            }
            if !valid_sha256(&source.baseline_digest) {
                return Err(IngestError::InvalidPolicy(format!(
                    "source {} has a malformed baseline digest",
                    source.source_id
                )));
            }
            if source.required_cells.is_empty() || !unique_nonempty(&source.required_cells) {
                return Err(IngestError::InvalidPolicy(format!(
                    "source {} needs unique, non-empty required cells",
                    source.source_id
                )));
            }
            if source.max_age_ns == 0 {
                return Err(IngestError::InvalidPolicy(format!(
                    "source {} cannot use a zero freshness interval",
                    source.source_id
                )));
            }
        }
        Ok(Self {
            policy,
            channels: BTreeMap::new(),
        })
    }

    /// Ingest one source observation. Duplicate identical sequence/digest/content is
    /// idempotent. Rollback and same-sequence divergence latch conflict. Sequence
    /// gaps latch unknown because an intermediate critical change may have been lost.
    pub fn ingest(&mut self, observation: Observation) -> Result<IngestOutcome, IngestError> {
        let Some(source_policy) = self
            .policy
            .sources
            .iter()
            .find(|policy| policy.source_id == observation.source_id)
        else {
            return Err(IngestError::UnknownSource);
        };
        let source_id = source_policy.source_id.clone();

        if !valid_sha256(&observation.evidence_digest)
            || !valid_sha256(&observation.predecessor_digest)
        {
            self.channels.insert(
                source_id,
                ChannelState::Unknown("source_evidence_or_predecessor_digest_malformed".to_owned()),
            );
            return Err(IngestError::MalformedEvidenceDigest);
        }

        match self.channels.get(&source_id) {
            Some(ChannelState::Unknown(_)) | Some(ChannelState::Conflicting(_)) => {
                return Err(IngestError::SourceQuarantined);
            }
            Some(ChannelState::Current(previous)) if observation.sequence == previous.sequence => {
                if observation.evidence_digest == previous.evidence_digest
                    && observation == **previous
                {
                    return Ok(IngestOutcome::DuplicateIdempotent);
                }
                self.channels.insert(
                    source_id,
                    ChannelState::Conflicting("same_sequence_different_content".to_owned()),
                );
                return Err(IngestError::SequenceDigestConflict);
            }
            Some(ChannelState::Current(previous)) if observation.sequence < previous.sequence => {
                self.channels.insert(
                    source_id,
                    ChannelState::Conflicting("observation_sequence_rollback".to_owned()),
                );
                return Err(IngestError::SequenceRollback);
            }
            Some(ChannelState::Current(previous)) => {
                let Some(expected) = previous.sequence.checked_add(1) else {
                    self.channels.insert(
                        source_id,
                        ChannelState::Conflicting("observation_sequence_exhausted".to_owned()),
                    );
                    return Err(IngestError::SequenceRollback);
                };
                if observation.sequence != expected {
                    self.channels.insert(
                        source_id,
                        ChannelState::Unknown("observation_sequence_gap".to_owned()),
                    );
                    return Err(IngestError::SequenceGap);
                }
                if observation.predecessor_digest != previous.evidence_digest {
                    self.channels.insert(
                        source_id,
                        ChannelState::Conflicting(
                            "observation_predecessor_digest_mismatch".to_owned(),
                        ),
                    );
                    return Err(IngestError::PredecessorDigestMismatch);
                }
            }
            None => {
                let expected = source_policy.baseline_sequence.checked_add(1);
                if Some(observation.sequence) != expected {
                    self.channels.insert(
                        source_id,
                        ChannelState::Unknown("initial_observation_sequence_gap".to_owned()),
                    );
                    return Err(IngestError::InitialSequenceGap);
                }
                if observation.predecessor_digest != source_policy.baseline_digest {
                    self.channels.insert(
                        source_id,
                        ChannelState::Conflicting(
                            "initial_observation_predecessor_mismatch".to_owned(),
                        ),
                    );
                    return Err(IngestError::PredecessorDigestMismatch);
                }
            }
        }

        self.channels
            .insert(source_id, ChannelState::Current(Box::new(observation)));
        Ok(IngestOutcome::Accepted)
    }

    /// Evaluate a full worksite observation snapshot. Any non-clear state requires
    /// a hold and epoch invalidation; this method does not execute that action.
    pub fn evaluate(&self, current_timebase_id: &str, now_monotonic_ns: u64) -> Evaluation {
        let mut reasons = BTreeSet::new();
        let mut changed_cells = BTreeSet::new();
        let mut clear_cells = BTreeSet::new();
        let mut conflicting_cells = BTreeSet::new();
        let mut latched_conflict = false;

        for source_policy in &self.policy.sources {
            let Some(channel) = self.channels.get(&source_policy.source_id) else {
                reasons.insert(format!(
                    "required_source_missing:{}",
                    source_policy.source_id
                ));
                continue;
            };

            let observation = match channel {
                ChannelState::Unknown(reason) => {
                    reasons.insert(reason.clone());
                    continue;
                }
                ChannelState::Conflicting(reason) => {
                    latched_conflict = true;
                    reasons.insert(reason.clone());
                    continue;
                }
                ChannelState::Current(observation) => observation,
            };

            let mut source_trustworthy = true;
            let mut source_clear_cells = BTreeSet::new();
            let mut source_changed_cells = BTreeSet::new();
            let mut source_conflicting_cells = BTreeSet::new();

            if observation.task_scope != self.policy.task_scope {
                source_trustworthy = false;
                reasons.insert(format!("task_scope_mismatch:{}", source_policy.source_id));
            }
            if !observation.authenticated_by_upstream {
                source_trustworthy = false;
                reasons.insert(format!(
                    "source_not_authenticated:{}",
                    source_policy.source_id
                ));
            }
            if observation.timebase_id != self.policy.expected_timebase_id
                || current_timebase_id != self.policy.expected_timebase_id
            {
                source_trustworthy = false;
                reasons.insert(format!("timebase_mismatch:{}", source_policy.source_id));
            }
            if now_monotonic_ns < observation.observed_at_monotonic_ns {
                source_trustworthy = false;
                reasons.insert(format!(
                    "future_observation_timestamp:{}",
                    source_policy.source_id
                ));
            } else if now_monotonic_ns - observation.observed_at_monotonic_ns
                > source_policy.max_age_ns
            {
                source_trustworthy = false;
                reasons.insert(format!("observation_stale:{}", source_policy.source_id));
            }
            if observation.clock_uncertainty_ns > source_policy.max_clock_uncertainty_ns {
                source_trustworthy = false;
                reasons.insert(format!(
                    "clock_uncertainty_exceeds_policy:{}",
                    source_policy.source_id
                ));
            }
            if observation.measurement_uncertainty_um > source_policy.max_measurement_uncertainty_um
            {
                source_trustworthy = false;
                reasons.insert(format!(
                    "measurement_uncertainty_exceeds_policy:{}",
                    source_policy.source_id
                ));
            }
            if observation.health != SensorHealth::Qualified {
                source_trustworthy = false;
                reasons.insert(format!("sensor_not_qualified:{}", source_policy.source_id));
            }
            if source_policy.requires_calibration && !observation.calibration_valid {
                source_trustworthy = false;
                reasons.insert(format!("calibration_not_valid:{}", source_policy.source_id));
            }
            if source_policy.requires_independent_reference
                && (observation.reference_independence
                    != ReferenceIndependence::IndependentUnderProfile
                    || observation
                        .independence_assessment_ref
                        .as_deref()
                        .is_none_or(str::is_empty))
            {
                source_trustworthy = false;
                reasons.insert(format!(
                    "independent_reference_not_established:{}",
                    source_policy.source_id
                ));
            }

            let clear: BTreeSet<&str> =
                observation.clear_cells.iter().map(String::as_str).collect();
            let changed: BTreeSet<&str> = observation
                .changed_cells
                .iter()
                .map(String::as_str)
                .collect();
            let occluded: BTreeSet<&str> = observation
                .occluded_cells
                .iter()
                .map(String::as_str)
                .collect();

            let unknown_cells = observation
                .clear_cells
                .iter()
                .chain(&observation.changed_cells)
                .chain(&observation.occluded_cells)
                .any(|cell| !source_policy.required_cells.contains(cell));
            if unknown_cells {
                source_trustworthy = false;
                reasons.insert(format!(
                    "observation_contains_out_of_scope_cells:{}",
                    source_policy.source_id
                ));
            }

            for cell in &source_policy.required_cells {
                let is_clear = clear.contains(cell.as_str());
                let is_changed = changed.contains(cell.as_str());
                let is_occluded = occluded.contains(cell.as_str());
                let classifications = [is_clear, is_changed, is_occluded]
                    .into_iter()
                    .filter(|classified| *classified)
                    .count();
                if classifications > 1 {
                    source_conflicting_cells.insert(cell.clone());
                    reasons.insert(format!(
                        "source_cell_classification_conflict:{}:{}",
                        source_policy.source_id, cell
                    ));
                } else if !is_clear && !is_changed && !is_occluded {
                    // Incomplete coverage blocks VERIFIED_CLEAR, but must not erase a
                    // separate positive change observed in another required cell.
                    reasons.insert(format!(
                        "required_cell_unobserved:{}:{}",
                        source_policy.source_id, cell
                    ));
                } else if is_occluded {
                    reasons.insert(format!(
                        "required_cell_occluded:{}:{}",
                        source_policy.source_id, cell
                    ));
                } else if is_changed {
                    source_changed_cells.insert(cell.clone());
                } else if is_clear {
                    source_clear_cells.insert(cell.clone());
                }
            }

            // Source-level integrity failures invalidate all of that source's claims.
            // Coverage incompleteness only prevents "verified clear"; it does not hide
            // a qualified positive change in a different cell.
            if source_trustworthy {
                clear_cells.extend(source_clear_cells);
                changed_cells.extend(source_changed_cells);
                conflicting_cells.extend(source_conflicting_cells);
            }
        }

        for cell in changed_cells.intersection(&clear_cells) {
            conflicting_cells.insert(cell.clone());
            reasons.insert(format!("independent_sources_disagree:{}", cell));
        }

        let state = if latched_conflict || !conflicting_cells.is_empty() {
            GateState::Conflicting
        } else if !changed_cells.is_empty() {
            GateState::OccupiedOrChanged
        } else if !reasons.is_empty() {
            GateState::UnknownOrUnobserved
        } else {
            GateState::VerifiedClear
        };
        let must_hold = state != GateState::VerifiedClear;
        Evaluation {
            state,
            must_hold,
            invalidate_epoch: must_hold,
            changed_cells: changed_cells.into_iter().collect(),
            conflicting_cells: conflicting_cells.into_iter().collect(),
            reason_codes: reasons.into_iter().collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(ch: char) -> String {
        std::iter::repeat_n(ch, 64).collect()
    }

    fn source(source_id: &str, required_cells: &[&str]) -> SourcePolicy {
        SourcePolicy {
            source_id: source_id.to_owned(),
            baseline_sequence: 0,
            baseline_digest: digest('0'),
            required_cells: required_cells.iter().map(|v| (*v).to_owned()).collect(),
            max_age_ns: 100,
            max_clock_uncertainty_ns: 10,
            max_measurement_uncertainty_um: 250,
            requires_calibration: true,
            requires_independent_reference: true,
        }
    }

    fn policy(sources: Vec<SourcePolicy>) -> GatePolicy {
        GatePolicy {
            task_scope: "floor-3-layout".to_owned(),
            expected_timebase_id: "boot-001".to_owned(),
            sources,
        }
    }

    fn observation(
        source_id: &str,
        sequence: u64,
        clear_cells: &[&str],
        changed_cells: &[&str],
        occluded_cells: &[&str],
    ) -> Observation {
        Observation {
            source_id: source_id.to_owned(),
            task_scope: "floor-3-layout".to_owned(),
            sequence,
            evidence_digest: format!(
                "{:064x}",
                (u64::from(source_id.as_bytes()[0]) << 32) | sequence
            ),
            predecessor_digest: if sequence == 1 {
                digest('0')
            } else {
                format!(
                    "{:064x}",
                    (u64::from(source_id.as_bytes()[0]) << 32) | (sequence - 1)
                )
            },
            timebase_id: "boot-001".to_owned(),
            observed_at_monotonic_ns: 100,
            clock_uncertainty_ns: 2,
            measurement_uncertainty_um: 100,
            health: SensorHealth::Qualified,
            calibration_valid: true,
            authenticated_by_upstream: true,
            reference_independence: ReferenceIndependence::IndependentUnderProfile,
            independence_assessment_ref: Some("fixture-independence-assessment".to_owned()),
            clear_cells: clear_cells.iter().map(|v| (*v).to_owned()).collect(),
            changed_cells: changed_cells.iter().map(|v| (*v).to_owned()).collect(),
            occluded_cells: occluded_cells.iter().map(|v| (*v).to_owned()).collect(),
        }
    }

    #[test]
    fn missing_required_source_is_unknown_not_clear() {
        let gate =
            ObservationGate::new(policy(vec![source("tracker", &["z1"])])).expect("valid policy");
        let result = gate.evaluate("boot-001", 150);
        assert_eq!(result.state, GateState::UnknownOrUnobserved);
        assert!(result.must_hold);
        assert!(
            result
                .reason_codes
                .iter()
                .any(|v| v.starts_with("required_source_missing"))
        );
    }

    #[test]
    fn unknown_source_cannot_extend_the_policy() {
        let mut gate =
            ObservationGate::new(policy(vec![source("tracker", &["z1"])])).expect("valid policy");
        assert_eq!(
            gate.ingest(observation("unconfigured", 1, &["z1"], &[], &[])),
            Err(IngestError::UnknownSource)
        );
        assert_eq!(
            gate.evaluate("boot-001", 150).state,
            GateState::UnknownOrUnobserved
        );
    }

    #[test]
    fn malformed_digest_quarantines_source() {
        let mut gate =
            ObservationGate::new(policy(vec![source("tracker", &["z1"])])).expect("valid policy");
        let mut malformed = observation("tracker", 1, &["z1"], &[], &[]);
        malformed.evidence_digest = "not-a-digest".to_owned();
        assert_eq!(
            gate.ingest(malformed),
            Err(IngestError::MalformedEvidenceDigest)
        );
        assert_eq!(
            gate.evaluate("boot-001", 150).state,
            GateState::UnknownOrUnobserved
        );
        assert_eq!(
            gate.ingest(observation("tracker", 1, &["z1"], &[], &[])),
            Err(IngestError::SourceQuarantined)
        );
    }

    #[test]
    fn all_fresh_qualified_cells_must_be_observed_clear() {
        let mut gate = ObservationGate::new(policy(vec![source("tracker", &["z1", "z2"])]))
            .expect("valid policy");
        gate.ingest(observation("tracker", 1, &["z1", "z2"], &[], &[]))
            .expect("first observation");
        let result = gate.evaluate("boot-001", 150);
        assert_eq!(result.state, GateState::VerifiedClear);
        assert!(!result.must_hold);
        assert!(!result.invalidate_epoch);
    }

    #[test]
    fn empty_detection_is_not_verified_clear() {
        let mut gate = ObservationGate::new(policy(vec![source("tracker", &["z1", "z2"])]))
            .expect("valid policy");
        gate.ingest(observation("tracker", 1, &[], &[], &[]))
            .expect("observation");
        let result = gate.evaluate("boot-001", 150);
        assert_eq!(result.state, GateState::UnknownOrUnobserved);
        assert!(result.must_hold);
        assert!(
            result
                .reason_codes
                .iter()
                .any(|v| v.starts_with("required_cell_unobserved"))
        );
    }

    #[test]
    fn out_of_scope_cell_cannot_be_promoted_to_clear_or_change() {
        let mut gate =
            ObservationGate::new(policy(vec![source("tracker", &["z1"])])).expect("valid policy");
        gate.ingest(observation(
            "tracker",
            1,
            &["z1", "unregistered-zone"],
            &[],
            &[],
        ))
        .expect("observation");
        let result = gate.evaluate("boot-001", 150);
        assert_eq!(result.state, GateState::UnknownOrUnobserved);
        assert!(result.must_hold);
        assert!(
            result
                .reason_codes
                .iter()
                .any(|v| v.starts_with("observation_contains_out_of_scope_cells"))
        );
    }

    #[test]
    fn occluded_zone_remains_unknown_even_with_no_positive_detection() {
        let mut gate = ObservationGate::new(policy(vec![source("tracker", &["z1", "z2"])]))
            .expect("valid policy");
        gate.ingest(observation("tracker", 1, &["z1"], &[], &["z2"]))
            .expect("observation");
        assert_eq!(
            gate.evaluate("boot-001", 150).state,
            GateState::UnknownOrUnobserved
        );
    }

    #[test]
    fn stale_source_forces_unknown() {
        let mut gate =
            ObservationGate::new(policy(vec![source("tracker", &["z1"])])).expect("valid policy");
        let mut stale = observation("tracker", 1, &["z1"], &[], &[]);
        stale.observed_at_monotonic_ns = 0;
        gate.ingest(stale).expect("observation");
        assert_eq!(
            gate.evaluate("boot-001", 101).state,
            GateState::UnknownOrUnobserved
        );
    }

    #[test]
    fn future_observation_timestamp_forces_unknown() {
        let mut gate =
            ObservationGate::new(policy(vec![source("tracker", &["z1"])])).expect("valid policy");
        let mut future = observation("tracker", 1, &["z1"], &[], &[]);
        future.observed_at_monotonic_ns = 151;
        gate.ingest(future).expect("observation");
        assert_eq!(
            gate.evaluate("boot-001", 150).state,
            GateState::UnknownOrUnobserved
        );
    }

    #[test]
    fn changed_timebase_forces_unknown() {
        let mut gate =
            ObservationGate::new(policy(vec![source("tracker", &["z1"])])).expect("valid policy");
        gate.ingest(observation("tracker", 1, &["z1"], &[], &[]))
            .expect("observation");
        assert_eq!(
            gate.evaluate("boot-002", 150).state,
            GateState::UnknownOrUnobserved
        );
    }

    #[test]
    fn bad_clock_uncertainty_forces_unknown() {
        let mut gate =
            ObservationGate::new(policy(vec![source("tracker", &["z1"])])).expect("valid policy");
        let mut bad_clock = observation("tracker", 1, &["z1"], &[], &[]);
        bad_clock.clock_uncertainty_ns = 11;
        gate.ingest(bad_clock).expect("observation");
        assert_eq!(
            gate.evaluate("boot-001", 150).state,
            GateState::UnknownOrUnobserved
        );
    }

    #[test]
    fn measurement_uncertainty_outside_policy_forces_unknown() {
        let mut gate =
            ObservationGate::new(policy(vec![source("tracker", &["z1"])])).expect("valid policy");
        let mut uncertain = observation("tracker", 1, &["z1"], &[], &[]);
        uncertain.measurement_uncertainty_um = 251;
        gate.ingest(uncertain).expect("observation");
        let result = gate.evaluate("boot-001", 150);
        assert_eq!(result.state, GateState::UnknownOrUnobserved);
        assert!(
            result
                .reason_codes
                .iter()
                .any(|v| v.starts_with("measurement_uncertainty_exceeds_policy"))
        );
    }

    #[test]
    fn degraded_sensor_forces_unknown() {
        let mut gate =
            ObservationGate::new(policy(vec![source("tracker", &["z1"])])).expect("valid policy");
        let mut degraded = observation("tracker", 1, &["z1"], &[], &[]);
        degraded.health = SensorHealth::Degraded;
        gate.ingest(degraded).expect("observation");
        assert_eq!(
            gate.evaluate("boot-001", 150).state,
            GateState::UnknownOrUnobserved
        );
    }

    #[test]
    fn invalid_calibration_forces_unknown() {
        let mut gate =
            ObservationGate::new(policy(vec![source("tracker", &["z1"])])).expect("valid policy");
        let mut uncalibrated = observation("tracker", 1, &["z1"], &[], &[]);
        uncalibrated.calibration_valid = false;
        gate.ingest(uncalibrated).expect("observation");
        assert_eq!(
            gate.evaluate("boot-001", 150).state,
            GateState::UnknownOrUnobserved
        );
    }

    #[test]
    fn non_independent_reference_forces_unknown_when_required() {
        let mut gate =
            ObservationGate::new(policy(vec![source("tracker", &["z1"])])).expect("valid policy");
        let mut observation = observation("tracker", 1, &["z1"], &[], &[]);
        observation.reference_independence = ReferenceIndependence::CommonMode;
        gate.ingest(observation).expect("observation");
        assert_eq!(
            gate.evaluate("boot-001", 150).state,
            GateState::UnknownOrUnobserved
        );
    }

    #[test]
    fn unauthenticated_observation_forces_unknown() {
        let mut gate =
            ObservationGate::new(policy(vec![source("tracker", &["z1"])])).expect("valid policy");
        let mut observation = observation("tracker", 1, &["z1"], &[], &[]);
        observation.authenticated_by_upstream = false;
        gate.ingest(observation).expect("observation");
        assert_eq!(
            gate.evaluate("boot-001", 150).state,
            GateState::UnknownOrUnobserved
        );
    }

    #[test]
    fn positive_change_is_not_erased_by_incomplete_coverage_elsewhere() {
        let mut gate = ObservationGate::new(policy(vec![source("tracker", &["z1", "z2", "z3"])]))
            .expect("valid policy");
        gate.ingest(observation("tracker", 1, &["z1"], &["z2"], &[]))
            .expect("observation");
        let result = gate.evaluate("boot-001", 150);
        assert_eq!(result.state, GateState::OccupiedOrChanged);
        assert!(result.must_hold);
        assert!(result.changed_cells.contains(&"z2".to_owned()));
        assert!(
            result
                .reason_codes
                .iter()
                .any(|v| v.starts_with("required_cell_unobserved"))
        );
    }

    #[test]
    fn positive_change_forces_hold_and_epoch_invalidation() {
        let mut gate = ObservationGate::new(policy(vec![source("tracker", &["z1", "z2"])]))
            .expect("valid policy");
        gate.ingest(observation("tracker", 1, &["z1"], &["z2"], &[]))
            .expect("observation");
        let result = gate.evaluate("boot-001", 150);
        assert_eq!(result.state, GateState::OccupiedOrChanged);
        assert!(result.must_hold);
        assert!(result.invalidate_epoch);
        assert_eq!(result.changed_cells, vec!["z2".to_owned()]);
    }

    #[test]
    fn conflicting_qualified_sources_fail_closed() {
        let mut gate = ObservationGate::new(policy(vec![
            source("tracker", &["z1"]),
            source("occupancy", &["z1"]),
        ]))
        .expect("valid policy");
        gate.ingest(observation("tracker", 1, &["z1"], &[], &[]))
            .expect("clear source");
        gate.ingest(observation("occupancy", 1, &[], &["z1"], &[]))
            .expect("changed source");
        let result = gate.evaluate("boot-001", 150);
        assert_eq!(result.state, GateState::Conflicting);
        assert!(result.must_hold);
        assert!(result.conflicting_cells.contains(&"z1".to_owned()));
    }

    #[test]
    fn duplicate_observation_is_idempotent_only_when_content_matches() {
        let mut gate =
            ObservationGate::new(policy(vec![source("tracker", &["z1"])])).expect("valid policy");
        let observation = observation("tracker", 1, &["z1"], &[], &[]);
        assert_eq!(
            gate.ingest(observation.clone()),
            Ok(IngestOutcome::Accepted)
        );
        assert_eq!(
            gate.ingest(observation),
            Ok(IngestOutcome::DuplicateIdempotent)
        );
    }

    #[test]
    fn same_sequence_divergent_digest_latches_conflict() {
        let mut gate =
            ObservationGate::new(policy(vec![source("tracker", &["z1"])])).expect("valid policy");
        gate.ingest(observation("tracker", 1, &["z1"], &[], &[]))
            .expect("observation");
        let mut divergent = observation("tracker", 1, &["z1"], &[], &[]);
        divergent.evidence_digest = digest('f');
        assert_eq!(
            gate.ingest(divergent),
            Err(IngestError::SequenceDigestConflict)
        );
        assert_eq!(gate.evaluate("boot-001", 150).state, GateState::Conflicting);
    }

    #[test]
    fn sequence_rollback_latches_conflict() {
        let mut gate =
            ObservationGate::new(policy(vec![source("tracker", &["z1"])])).expect("valid policy");
        gate.ingest(observation("tracker", 1, &["z1"], &[], &[]))
            .expect("sequence 1");
        gate.ingest(observation("tracker", 2, &["z1"], &[], &[]))
            .expect("sequence 2");
        assert_eq!(
            gate.ingest(observation("tracker", 1, &["z1"], &[], &[])),
            Err(IngestError::SequenceRollback)
        );
        assert_eq!(gate.evaluate("boot-001", 150).state, GateState::Conflicting);
    }

    #[test]
    fn wrong_predecessor_digest_latches_conflict() {
        let mut gate =
            ObservationGate::new(policy(vec![source("tracker", &["z1"])])).expect("valid policy");
        gate.ingest(observation("tracker", 1, &["z1"], &[], &[]))
            .expect("sequence 1");
        let mut tampered = observation("tracker", 2, &["z1"], &[], &[]);
        tampered.predecessor_digest = digest('f');
        assert_eq!(
            gate.ingest(tampered),
            Err(IngestError::PredecessorDigestMismatch)
        );
        assert_eq!(gate.evaluate("boot-001", 150).state, GateState::Conflicting);
    }

    #[test]
    fn trusted_midstream_baseline_supports_gate_restart_without_accepting_a_gap() {
        let mut p = policy(vec![source("tracker", &["z1"])]);
        p.sources[0].baseline_sequence = 7;
        p.sources[0].baseline_digest = digest('7');
        let mut gate = ObservationGate::new(p).expect("valid baseline policy");
        let mut first_after_baseline = observation("tracker", 8, &["z1"], &[], &[]);
        first_after_baseline.predecessor_digest = digest('7');
        gate.ingest(first_after_baseline)
            .expect("sequence after trusted baseline");
        assert_eq!(
            gate.evaluate("boot-001", 150).state,
            GateState::VerifiedClear
        );
    }

    #[test]
    fn sequence_gap_latches_unknown_until_external_rebaseline() {
        let mut gate =
            ObservationGate::new(policy(vec![source("tracker", &["z1"])])).expect("valid policy");
        assert_eq!(
            gate.ingest(observation("tracker", 2, &["z1"], &[], &[])),
            Err(IngestError::InitialSequenceGap)
        );
        assert_eq!(
            gate.evaluate("boot-001", 150).state,
            GateState::UnknownOrUnobserved
        );
        assert_eq!(
            gate.ingest(observation("tracker", 3, &["z1"], &[], &[])),
            Err(IngestError::SourceQuarantined)
        );
    }
}
