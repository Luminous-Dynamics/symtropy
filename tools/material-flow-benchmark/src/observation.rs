// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Fail-closed receiver-presence observation consistency.
//!
//! This module evaluates declared observations from distinct common-cause
//! groups. It does not establish sensor truth, verify signatures, prove that
//! common-cause declarations are honest, grant motion authority, or complete
//! custody. Simulator ground truth must remain outside this input contract.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

pub const OBSERVATION_SCHEMA_VERSION: u16 = 1;
pub const MINIMUM_INDEPENDENT_GROUPS_V1: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PresenceReading {
    Present,
    Absent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PresenceVerdict {
    ConfirmedPresent,
    ConfirmedAbsent,
    Unknown,
    Contradictory,
    Stale,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReceiverPresenceObservation {
    pub event_id: String,
    pub source_id: String,
    /// Declared shared cause boundary (e.g. controller, power rail, firmware,
    /// calibration chain or clock). Missing means independence is unknown.
    pub common_cause_group: Option<String>,
    pub scenario_id: String,
    pub subject_id: String,
    pub reading: PresenceReading,
    pub observed_at_unix_ms: u64,
    pub expires_at_unix_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PresenceAdjudicationPolicy {
    pub scenario_id: String,
    pub subject_id: String,
    pub earliest_decision_time_unix_ms: u64,
    pub latest_decision_time_unix_ms: u64,
    pub maximum_observation_age_ms: u64,
    pub minimum_independent_groups: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PresenceAdjudicationInput {
    pub policy: PresenceAdjudicationPolicy,
    pub observations: Vec<ReceiverPresenceObservation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PresenceAdjudicationReport {
    pub schema_version: u16,
    pub verdict: PresenceVerdict,
    pub observation_count: usize,
    pub fresh_observation_count: usize,
    pub unique_source_count: usize,
    pub independent_common_cause_group_count: usize,
    pub stale_observation_count: usize,
    pub duplicate_event_count: usize,
    pub duplicate_event_conflict_count: usize,
    pub conflicting_source_count: usize,
    pub positive_presence_confirmed: bool,
    pub sensor_truth_established: bool,
    pub cryptographic_authentication_verified: bool,
    pub issuer_authorization_verified: bool,
    pub trusted_time_verified: bool,
    pub motion_authority_granted: bool,
    pub physical_command_emitted: bool,
    pub custody_completion_permitted: bool,
    pub limitations: Vec<String>,
}

/// Adjudicate receiver presence using fresh observations from distinct
/// declared common-cause groups.
///
/// Unknown common-cause membership does not count toward the v1 independent
/// source quorum. Two source IDs in one common-cause group count as one group.
/// A confirmed presence result is only an observation-consistency result: it
/// is not a delivery receipt, authorization or custody completion.
pub fn adjudicate_receiver_presence(
    observations: &[ReceiverPresenceObservation],
    policy: &PresenceAdjudicationPolicy,
) -> Result<PresenceAdjudicationReport, Vec<String>> {
    let mut errors = Vec::new();

    if policy.scenario_id.trim().is_empty() {
        errors.push("policy.scenario_id: must be non-empty".into());
    }
    if policy.subject_id.trim().is_empty() {
        errors.push("policy.subject_id: must be non-empty".into());
    }
    if policy.earliest_decision_time_unix_ms > policy.latest_decision_time_unix_ms {
        errors.push("policy: decision-time interval is inverted".into());
    }
    if policy.maximum_observation_age_ms == 0 {
        errors.push("policy.maximum_observation_age_ms: must be non-zero".into());
    }
    if policy.minimum_independent_groups < MINIMUM_INDEPENDENT_GROUPS_V1 {
        errors.push(format!(
            "policy.minimum_independent_groups: v1 requires at least {MINIMUM_INDEPENDENT_GROUPS_V1}"
        ));
    }
    if observations.len() > 256 {
        errors.push("observations: maximum of 256 records exceeded".into());
    }

    for (index, observation) in observations.iter().enumerate() {
        let prefix = format!("observations[{index}]");
        if observation.event_id.trim().is_empty()
            || observation.source_id.trim().is_empty()
            || observation
                .common_cause_group
                .as_ref()
                .is_some_and(|group| group.trim().is_empty())
        {
            errors.push(format!(
                "{prefix}: event/source/common-cause identifiers must not be empty"
            ));
        }
        if observation.scenario_id != policy.scenario_id {
            errors.push(format!("{prefix}.scenario_id: exact scenario mismatch"));
        }
        if observation.subject_id != policy.subject_id {
            errors.push(format!("{prefix}.subject_id: exact subject mismatch"));
        }
        if observation.expires_at_unix_ms <= observation.observed_at_unix_ms {
            errors.push(format!("{prefix}: expiry must be later than observation"));
        }
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    let mut events_by_id: BTreeMap<&str, &ReceiverPresenceObservation> = BTreeMap::new();
    let mut duplicate_event_count = 0_usize;
    let mut duplicate_event_conflict_count = 0_usize;
    let mut fresh_observations = Vec::new();
    let mut stale_observation_count = 0_usize;

    for observation in observations {
        if let Some(previous) = events_by_id.get(observation.event_id.as_str()) {
            duplicate_event_count += 1;
            if *previous != observation {
                duplicate_event_conflict_count += 1;
            }
            // An identical replay is idempotently ignored. Reuse of an event
            // ID for different content is retained as a contradiction below.
            continue;
        }
        events_by_id.insert(observation.event_id.as_str(), observation);

        let fresh = observation.observed_at_unix_ms < policy.earliest_decision_time_unix_ms
            && observation.expires_at_unix_ms > policy.latest_decision_time_unix_ms
            && policy
                .latest_decision_time_unix_ms
                .saturating_sub(observation.observed_at_unix_ms)
                <= policy.maximum_observation_age_ms;

        if fresh {
            fresh_observations.push(observation);
        } else {
            stale_observation_count += 1;
        }
    }

    let mut source_readings: BTreeMap<&str, (Option<&str>, PresenceReading)> = BTreeMap::new();
    let mut group_readings: BTreeMap<&str, BTreeSet<PresenceReading>> = BTreeMap::new();
    let mut conflicting_source_count = 0_usize;

    for observation in &fresh_observations {
        match source_readings.get(observation.source_id.as_str()) {
            Some((existing_group, existing_reading))
                if *existing_group != observation.common_cause_group.as_deref()
                    || *existing_reading != observation.reading =>
            {
                conflicting_source_count += 1;
            }
            Some(_) => {}
            None => {
                source_readings.insert(
                    observation.source_id.as_str(),
                    (
                        observation.common_cause_group.as_deref(),
                        observation.reading,
                    ),
                );
            }
        }

        if let Some(group) = observation.common_cause_group.as_deref() {
            group_readings
                .entry(group)
                .or_default()
                .insert(observation.reading);
        }
    }

    let group_conflict = group_readings.values().any(|readings| readings.len() > 1);
    let all_fresh_readings = fresh_observations
        .iter()
        .map(|observation| observation.reading)
        .collect::<BTreeSet<_>>();
    let cross_group_conflict = all_fresh_readings.len() > 1;
    let conflicting_source = conflicting_source_count > 0;
    let contradictory = group_conflict || cross_group_conflict || conflicting_source;

    let independent_group_count = group_readings.len();
    let verdict = if duplicate_event_conflict_count > 0 || contradictory {
        PresenceVerdict::Contradictory
    } else if independent_group_count >= policy.minimum_independent_groups
        && !fresh_observations.is_empty()
        && all_fresh_readings == BTreeSet::from([PresenceReading::Present])
    {
        PresenceVerdict::ConfirmedPresent
    } else if independent_group_count >= policy.minimum_independent_groups
        && !fresh_observations.is_empty()
        && all_fresh_readings == BTreeSet::from([PresenceReading::Absent])
    {
        PresenceVerdict::ConfirmedAbsent
    } else if stale_observation_count > 0
        && (fresh_observations.is_empty()
            || independent_group_count < policy.minimum_independent_groups)
    {
        PresenceVerdict::Stale
    } else {
        PresenceVerdict::Unknown
    };

    Ok(PresenceAdjudicationReport {
        schema_version: OBSERVATION_SCHEMA_VERSION,
        verdict,
        observation_count: observations.len(),
        fresh_observation_count: fresh_observations.len(),
        unique_source_count: source_readings.len(),
        independent_common_cause_group_count: independent_group_count,
        stale_observation_count,
        duplicate_event_count,
        duplicate_event_conflict_count,
        conflicting_source_count,
        positive_presence_confirmed: verdict == PresenceVerdict::ConfirmedPresent,
        sensor_truth_established: false,
        cryptographic_authentication_verified: false,
        issuer_authorization_verified: false,
        trusted_time_verified: false,
        motion_authority_granted: false,
        physical_command_emitted: false,
        custody_completion_permitted: false,
        limitations: vec![
            "Adjudicates declared readings and common-cause labels only; it does not establish sensor truth".into(),
            "Common-cause groups are caller-declared and are not authenticated or independently proven".into(),
            "Declared timestamps are checked against a policy window; no trusted clock or timestamp authority is verified".into(),
            "Signatures, content commitments, issuer authorization and hardware attestation are not verified".into(),
            "A confirmed presence observation grants no motion authority and cannot complete custody".into(),
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> PresenceAdjudicationPolicy {
        PresenceAdjudicationPolicy {
            scenario_id: "mfb-homeops-stationary-carrier-nominal-v1".into(),
            subject_id: "package-01".into(),
            earliest_decision_time_unix_ms: 1_000,
            latest_decision_time_unix_ms: 1_050,
            maximum_observation_age_ms: 2_000,
            minimum_independent_groups: 2,
        }
    }

    fn observation(
        event_id: &str,
        source_id: &str,
        group: Option<&str>,
        reading: PresenceReading,
    ) -> ReceiverPresenceObservation {
        ReceiverPresenceObservation {
            event_id: event_id.into(),
            source_id: source_id.into(),
            common_cause_group: group.map(str::to_string),
            scenario_id: "mfb-homeops-stationary-carrier-nominal-v1".into(),
            subject_id: "package-01".into(),
            reading,
            observed_at_unix_ms: 900,
            expires_at_unix_ms: 1_100,
        }
    }

    #[test]
    fn one_positive_sensor_is_unknown_and_cannot_complete_custody() {
        let report = adjudicate_receiver_presence(
            &[observation(
                "evt-1",
                "sensor-1",
                Some("controller-a"),
                PresenceReading::Present,
            )],
            &policy(),
        )
        .expect("well-formed observation");
        assert_eq!(report.verdict, PresenceVerdict::Unknown);
        assert!(!report.positive_presence_confirmed);
        assert!(!report.custody_completion_permitted);
    }

    #[test]
    fn two_fresh_positive_independent_groups_confirm_observation_not_delivery() {
        let report = adjudicate_receiver_presence(
            &[
                observation(
                    "evt-1",
                    "scale-1",
                    Some("scale-chain"),
                    PresenceReading::Present,
                ),
                observation(
                    "evt-2",
                    "beam-1",
                    Some("optical-chain"),
                    PresenceReading::Present,
                ),
            ],
            &policy(),
        )
        .expect("well-formed observations");
        assert_eq!(report.verdict, PresenceVerdict::ConfirmedPresent);
        assert!(report.positive_presence_confirmed);
        assert!(!report.sensor_truth_established);
        assert!(!report.motion_authority_granted);
        assert!(!report.custody_completion_permitted);
    }

    #[test]
    fn two_sensors_sharing_one_common_cause_group_are_not_independent() {
        let report = adjudicate_receiver_presence(
            &[
                observation(
                    "evt-1",
                    "sensor-1",
                    Some("shared-controller"),
                    PresenceReading::Present,
                ),
                observation(
                    "evt-2",
                    "sensor-2",
                    Some("shared-controller"),
                    PresenceReading::Present,
                ),
            ],
            &policy(),
        )
        .expect("well-formed observations");
        assert_eq!(report.verdict, PresenceVerdict::Unknown);
        assert_eq!(report.independent_common_cause_group_count, 1);
    }

    #[test]
    fn false_positive_conflicting_with_independent_absence_is_contradictory() {
        let report = adjudicate_receiver_presence(
            &[
                observation(
                    "evt-1",
                    "receiver-presence",
                    Some("receiver-device"),
                    PresenceReading::Present,
                ),
                observation(
                    "evt-2",
                    "gate-contact",
                    Some("gate-controller"),
                    PresenceReading::Absent,
                ),
            ],
            &policy(),
        )
        .expect("well-formed observations");
        assert_eq!(report.verdict, PresenceVerdict::Contradictory);
        assert!(!report.positive_presence_confirmed);
        assert!(!report.custody_completion_permitted);
    }

    #[test]
    fn stale_negative_observation_cannot_be_used_to_erase_a_fresh_single_positive() {
        let mut stale_negative = observation(
            "evt-2",
            "gate-contact",
            Some("gate-controller"),
            PresenceReading::Absent,
        );
        stale_negative.observed_at_unix_ms = 100;
        let mut stale_policy = policy();
        stale_policy.maximum_observation_age_ms = 500;
        let report = adjudicate_receiver_presence(
            &[
                observation(
                    "evt-1",
                    "receiver-presence",
                    Some("receiver-device"),
                    PresenceReading::Present,
                ),
                stale_negative,
            ],
            &stale_policy,
        )
        .expect("well-formed observations");
        assert_eq!(report.verdict, PresenceVerdict::Stale);
        assert!(!report.positive_presence_confirmed);
    }

    #[test]
    fn stale_positive_observation_cannot_contribute_to_quorum() {
        let mut stale = observation(
            "evt-1",
            "sensor-1",
            Some("sensor-a"),
            PresenceReading::Present,
        );
        stale.observed_at_unix_ms = 100;
        let mut stale_policy = policy();
        stale_policy.maximum_observation_age_ms = 500;
        let report = adjudicate_receiver_presence(&[stale], &stale_policy)
            .expect("well-formed stale observation");
        assert_eq!(report.verdict, PresenceVerdict::Stale);
        assert_eq!(report.stale_observation_count, 1);
    }

    #[test]
    fn identical_event_replay_is_suppressed_without_counting_a_second_vote() {
        let first = observation(
            "evt-1",
            "sensor-1",
            Some("sensor-a"),
            PresenceReading::Present,
        );
        let second = observation(
            "evt-2",
            "sensor-2",
            Some("sensor-b"),
            PresenceReading::Present,
        );
        let duplicate = first.clone();
        let report = adjudicate_receiver_presence(&[first, second, duplicate], &policy())
            .expect("well-formed observations");
        assert_eq!(report.verdict, PresenceVerdict::ConfirmedPresent);
        assert_eq!(report.duplicate_event_count, 1);
        assert_eq!(report.duplicate_event_conflict_count, 0);
        assert_eq!(report.fresh_observation_count, 2);
    }

    #[test]
    fn reused_event_id_with_different_payload_is_contradictory() {
        let first = observation(
            "evt-1",
            "sensor-1",
            Some("sensor-a"),
            PresenceReading::Present,
        );
        let mut conflicting = first.clone();
        conflicting.reading = PresenceReading::Absent;
        let report = adjudicate_receiver_presence(&[first, conflicting], &policy())
            .expect("well-formed observations");
        assert_eq!(report.verdict, PresenceVerdict::Contradictory);
        assert_eq!(report.duplicate_event_count, 1);
        assert_eq!(report.duplicate_event_conflict_count, 1);
    }

    #[test]
    fn the_same_source_cannot_change_value_or_common_cause_identity() {
        let report = adjudicate_receiver_presence(
            &[
                observation(
                    "evt-1",
                    "sensor-1",
                    Some("sensor-a"),
                    PresenceReading::Present,
                ),
                observation(
                    "evt-2",
                    "sensor-1",
                    Some("sensor-b"),
                    PresenceReading::Present,
                ),
            ],
            &policy(),
        )
        .expect("well-formed observations");
        assert_eq!(report.verdict, PresenceVerdict::Contradictory);
        assert_eq!(report.conflicting_source_count, 1);
    }

    #[test]
    fn observations_touching_clock_uncertainty_boundary_are_stale() {
        let mut boundary = observation(
            "evt-1",
            "sensor-1",
            Some("sensor-a"),
            PresenceReading::Present,
        );
        boundary.observed_at_unix_ms = 1_000;
        let report = adjudicate_receiver_presence(&[boundary], &policy())
            .expect("well-formed boundary observation");
        assert_eq!(report.verdict, PresenceVerdict::Stale);
    }

    #[test]
    fn wrong_subject_or_scenario_rejects_the_observation_bundle() {
        let mut wrong = observation(
            "evt-1",
            "sensor-1",
            Some("sensor-a"),
            PresenceReading::Present,
        );
        wrong.subject_id = "package-02".into();
        assert!(adjudicate_receiver_presence(&[wrong], &policy()).is_err());
    }

    #[test]
    fn unknown_common_cause_is_not_assumed_independent() {
        let report = adjudicate_receiver_presence(
            &[
                observation("evt-1", "sensor-1", None, PresenceReading::Present),
                observation("evt-2", "sensor-2", None, PresenceReading::Present),
            ],
            &policy(),
        )
        .expect("well-formed observations");
        assert_eq!(report.verdict, PresenceVerdict::Unknown);
        assert_eq!(report.independent_common_cause_group_count, 0);
    }
}
