// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Policy-visible observation frames, intentionally separate from latent world
//! state. This module contains no simulator state, future event list or outcome
//! object in the frame API.

use super::{ComponentId, ResourceKind, SettlementError, sha256_hex, valid_identifier};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const OBSERVATION_SCHEMA_VERSION: u32 = 1;
pub const OBSERVATION_CLAIM_CEILING: &str =
    "synthetic_policy_visible_observations_only_not_sensor_or_physical_validation";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnownComponentCondition {
    Available,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ObservedValue {
    ComponentCondition {
        condition: KnownComponentCondition,
    },
    ResourceQuantity {
        resource_kind: ResourceKind,
        quantity_units: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum ObservationDisposition {
    /// A current, validated value available at the frame's decision tick.
    Current {
        value: ObservedValue,
    },
    /// A prior value is available but must not be interpreted as current.
    Stale {
        last_value: ObservedValue,
    },
    /// An expected reading is explicitly absent.
    Missing {
        reason_ref: String,
    },
    /// Conflicting candidate records are preserved by identity, not collapsed.
    Conflicting {
        candidate_observation_ids: Vec<String>,
    },
    /// A value exists but has not passed its declared validation requirement.
    Unvalidated {
        value: Option<ObservedValue>,
    },
    /// The evidence was explicitly revoked and cannot be used as current proof.
    Revoked {
        reason_ref: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationRecord {
    pub observation_id: String,
    pub source_id: String,
    pub source_generation: u64,
    pub target_component: ComponentId,
    pub target_generation: u64,
    /// Synthetic tick when the represented sample/event was taken.
    pub sample_tick: u64,
    /// Synthetic tick when this record became available to a decision-maker.
    pub available_tick: u64,
    /// Maximum sample age (in synthetic ticks) permitted to remain Current.
    pub freshness_window_ticks: u64,
    pub disposition: ObservationDisposition,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationSchedule {
    pub schema_version: u32,
    pub records: Vec<ObservationRecord>,
}

impl ObservationSchedule {
    /// Canonicalizes schedule order without removing duplicate observation IDs.
    pub fn canonicalized(&self) -> Self {
        let mut canonical = self.clone();
        canonical.records.sort_by(|a, b| {
            a.available_tick
                .cmp(&b.available_tick)
                .then_with(|| a.sample_tick.cmp(&b.sample_tick))
                .then_with(|| a.observation_id.cmp(&b.observation_id))
        });
        for record in &mut canonical.records {
            if let ObservationDisposition::Conflicting {
                candidate_observation_ids,
            } = &mut record.disposition
            {
                candidate_observation_ids.sort();
            }
        }
        canonical
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyInformationFramePayload {
    pub schema_version: u32,
    pub decision_tick: u64,
    pub claim_ceiling: String,
    /// Contains only observation records available no later than decision_tick.
    /// Latent component state and future scenario events have no field here.
    pub observations: Vec<ObservationRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyInformationFrame {
    pub payload: PolicyInformationFramePayload,
    /// SHA-256 over the canonical compact payload serialization. This is an
    /// integrity link, not a signature, sensor attestation or source of truth.
    pub payload_sha256: String,
}

pub fn build_policy_information_frame(
    schedule: &ObservationSchedule,
    decision_tick: u64,
) -> Result<PolicyInformationFrame, SettlementError> {
    let canonical = schedule.canonicalized();
    validate_schedule(&canonical)?;
    let observations: Vec<ObservationRecord> = canonical
        .records
        .into_iter()
        .filter(|record| record.available_tick <= decision_tick)
        .map(|mut record| {
            let expired_value = if decision_tick.saturating_sub(record.sample_tick)
                > record.freshness_window_ticks
            {
                match &record.disposition {
                    ObservationDisposition::Current { value } => Some(value.clone()),
                    _ => None,
                }
            } else {
                None
            };
            if let Some(last_value) = expired_value {
                record.disposition = ObservationDisposition::Stale { last_value };
            }
            record
        })
        .collect();
    let payload = PolicyInformationFramePayload {
        schema_version: OBSERVATION_SCHEMA_VERSION,
        decision_tick,
        claim_ceiling: OBSERVATION_CLAIM_CEILING.into(),
        observations,
    };
    let encoded = serde_json::to_vec(&payload)
        .map_err(|error| SettlementError(format!("serialize observation frame: {error}")))?;
    let payload_sha256 = sha256_hex(&encoded);
    let frame = PolicyInformationFrame {
        payload,
        payload_sha256,
    };
    verify_policy_information_frame(&frame)?;
    Ok(frame)
}

pub fn verify_policy_information_frame(
    frame: &PolicyInformationFrame,
) -> Result<(), SettlementError> {
    let payload = &frame.payload;
    if payload.schema_version != OBSERVATION_SCHEMA_VERSION {
        return Err(SettlementError(
            "unsupported policy-information frame schema version".into(),
        ));
    }
    if payload.claim_ceiling != OBSERVATION_CLAIM_CEILING {
        return Err(SettlementError(
            "policy-information claim ceiling is missing or widened".into(),
        ));
    }
    let encoded = serde_json::to_vec(payload)
        .map_err(|error| SettlementError(format!("serialize observation frame: {error}")))?;
    if sha256_hex(&encoded) != frame.payload_sha256 {
        return Err(SettlementError(
            "policy-information frame digest mismatch".into(),
        ));
    }

    let mut ids = BTreeSet::new();
    let mut prior_key: Option<(u64, u64, String)> = None;
    for record in &payload.observations {
        validate_record(record)?;
        if record.available_tick > payload.decision_tick
            || record.sample_tick > payload.decision_tick
        {
            return Err(SettlementError(format!(
                "future observation {} leaked into decision tick {}",
                record.observation_id, payload.decision_tick
            )));
        }
        let sample_age = payload.decision_tick - record.sample_tick;
        match &record.disposition {
            ObservationDisposition::Current { .. }
                if sample_age > record.freshness_window_ticks =>
            {
                return Err(SettlementError(format!(
                    "over-age observation {} is incorrectly marked Current",
                    record.observation_id
                )));
            }
            ObservationDisposition::Stale { .. }
                if sample_age <= record.freshness_window_ticks =>
            {
                return Err(SettlementError(format!(
                    "observation {} is marked Stale inside its freshness window",
                    record.observation_id
                )));
            }
            _ => {}
        }
        if !ids.insert(record.observation_id.clone()) {
            return Err(SettlementError(format!(
                "duplicate observation ID {} in policy frame",
                record.observation_id
            )));
        }
        let key = (
            record.available_tick,
            record.sample_tick,
            record.observation_id.clone(),
        );
        if prior_key.as_ref().is_some_and(|prior| prior > &key) {
            return Err(SettlementError(
                "policy-information frame observations are not canonically ordered".into(),
            ));
        }
        prior_key = Some(key);
    }
    Ok(())
}

fn validate_schedule(schedule: &ObservationSchedule) -> Result<(), SettlementError> {
    if schedule.schema_version != OBSERVATION_SCHEMA_VERSION {
        return Err(SettlementError(format!(
            "unsupported observation-schedule schema version {}",
            schedule.schema_version
        )));
    }
    let mut ids = BTreeSet::new();
    for record in &schedule.records {
        validate_record(record)?;
        if matches!(&record.disposition, ObservationDisposition::Stale { .. })
            && record.available_tick.saturating_sub(record.sample_tick)
                <= record.freshness_window_ticks
        {
            return Err(SettlementError(format!(
                "source marks observation {} stale inside its freshness window",
                record.observation_id
            )));
        }
        if !ids.insert(record.observation_id.clone()) {
            return Err(SettlementError(format!(
                "duplicate observation ID {} in schedule",
                record.observation_id
            )));
        }
    }
    Ok(())
}

fn validate_record(record: &ObservationRecord) -> Result<(), SettlementError> {
    valid_identifier(&record.observation_id, "observation ID")?;
    valid_identifier(&record.source_id, "observation source ID")?;
    valid_identifier(record.target_component.as_str(), "observation target component ID")?;
    if record.source_generation == 0 || record.target_generation == 0 {
        return Err(SettlementError(format!(
            "observation {} has generation zero",
            record.observation_id
        )));
    }
    if record.sample_tick > record.available_tick {
        return Err(SettlementError(format!(
            "observation {} is available before its sample tick",
            record.observation_id
        )));
    }

    match &record.disposition {
        ObservationDisposition::Current { .. } => {}
        ObservationDisposition::Stale { .. } => {}
        ObservationDisposition::Missing { reason_ref }
        | ObservationDisposition::Revoked { reason_ref } => {
            valid_identifier(reason_ref, "observation reason reference")?;
        }
        ObservationDisposition::Conflicting {
            candidate_observation_ids,
        } => {
            if candidate_observation_ids.len() < 2 {
                return Err(SettlementError(format!(
                    "conflicting observation {} has fewer than two candidates",
                    record.observation_id
                )));
            }
            let mut candidates = BTreeSet::new();
            for candidate in candidate_observation_ids {
                valid_identifier(candidate, "conflicting candidate observation ID")?;
                if !candidates.insert(candidate) {
                    return Err(SettlementError(format!(
                        "conflicting observation {} repeats a candidate",
                        record.observation_id
                    )));
                }
                if candidate == &record.observation_id {
                    return Err(SettlementError(format!(
                        "observation {} lists itself as a conflicting candidate",
                        record.observation_id
                    )));
                }
            }
        }
        ObservationDisposition::Unvalidated { .. } => {}
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(
        observation_id: &str,
        sample_tick: u64,
        available_tick: u64,
        disposition: ObservationDisposition,
    ) -> ObservationRecord {
        ObservationRecord {
            observation_id: observation_id.into(),
            source_id: "grid-sensor".into(),
            source_generation: 1,
            target_component: ComponentId::parse("grid").unwrap(),
            target_generation: 1,
            sample_tick,
            available_tick,
            freshness_window_ticks: 1,
            disposition,
        }
    }

    fn schedule(records: Vec<ObservationRecord>) -> ObservationSchedule {
        ObservationSchedule {
            schema_version: OBSERVATION_SCHEMA_VERSION,
            records,
        }
    }

    #[test]
    fn future_observations_are_not_visible_before_their_available_tick() {
        let common = record(
            "grid-now",
            0,
            0,
            ObservationDisposition::Current {
                value: ObservedValue::ComponentCondition {
                    condition: KnownComponentCondition::Available,
                },
            },
        );
        let future_failed = record(
            "grid-future",
            5,
            5,
            ObservationDisposition::Current {
                value: ObservedValue::ComponentCondition {
                    condition: KnownComponentCondition::Failed,
                },
            },
        );
        let future_available = record(
            "grid-future",
            5,
            5,
            ObservationDisposition::Current {
                value: ObservedValue::ComponentCondition {
                    condition: KnownComponentCondition::Available,
                },
            },
        );

        let first = build_policy_information_frame(
            &schedule(vec![common.clone(), future_failed]),
            2,
        )
        .unwrap();
        let second = build_policy_information_frame(
            &schedule(vec![common, future_available]),
            2,
        )
        .unwrap();

        assert_eq!(first, second);
        assert_eq!(first.payload.observations.len(), 1);
        assert_eq!(first.payload.observations[0].observation_id, "grid-now");
    }

    #[test]
    fn current_observation_ages_into_stale_at_decision_time() {
        let frame = build_policy_information_frame(
            &schedule(vec![record(
                "old-but-once-current",
                0,
                0,
                ObservationDisposition::Current {
                    value: ObservedValue::ComponentCondition {
                        condition: KnownComponentCondition::Available,
                    },
                },
            )]),
            3,
        )
        .unwrap();
        assert!(matches!(
            &frame.payload.observations[0].disposition,
            ObservationDisposition::Stale { .. }
        ));
        assert_eq!(frame.payload.observations[0].sample_tick, 0);
        assert_eq!(frame.payload.observations[0].available_tick, 0);
    }

    #[test]
    fn stale_values_keep_their_stale_disposition_in_the_policy_frame() {
        let frame = build_policy_information_frame(
            &schedule(vec![record(
                "grid-old",
                1,
                3,
                ObservationDisposition::Stale {
                    last_value: ObservedValue::ComponentCondition {
                        condition: KnownComponentCondition::Available,
                    },
                },
            )]),
            3,
        )
        .unwrap();

        assert!(matches!(
            &frame.payload.observations[0].disposition,
            ObservationDisposition::Stale { .. }
        ));
        assert_eq!(frame.payload.observations[0].sample_tick, 1);
        assert_eq!(frame.payload.observations[0].available_tick, 3);
    }

    #[test]
    fn missing_and_conflicting_observations_are_not_collapsed_into_a_value() {
        let missing = record(
            "grid-missing",
            2,
            2,
            ObservationDisposition::Missing {
                reason_ref: "dropout-02".into(),
            },
        );
        let conflicting = record(
            "grid-conflict",
            3,
            3,
            ObservationDisposition::Conflicting {
                candidate_observation_ids: vec!["reading-a".into(), "reading-b".into()],
            },
        );
        let frame = build_policy_information_frame(&schedule(vec![conflicting, missing]), 3)
            .unwrap();
        assert!(matches!(
            &frame.payload.observations[0].disposition,
            ObservationDisposition::Missing { .. }
        ));
        assert!(matches!(
            &frame.payload.observations[1].disposition,
            ObservationDisposition::Conflicting { .. }
        ));
    }

    #[test]
    fn observation_frame_is_canonical_and_order_independent() {
        let a = record(
            "reading-a",
            0,
            1,
            ObservationDisposition::Current {
                value: ObservedValue::ResourceQuantity {
                    resource_kind: ResourceKind::PotableWater,
                    quantity_units: 12,
                },
            },
        );
        let b = record(
            "reading-b",
            1,
            1,
            ObservationDisposition::Unvalidated {
                value: Some(ObservedValue::ResourceQuantity {
                    resource_kind: ResourceKind::PotableWater,
                    quantity_units: 15,
                }),
            },
        );
        let first = build_policy_information_frame(&schedule(vec![a.clone(), b.clone()]), 1)
            .unwrap();
        let second = build_policy_information_frame(&schedule(vec![b, a]), 1).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn future_sample_cannot_be_available_before_it_occurs() {
        let input = schedule(vec![record(
            "impossible-reading",
            4,
            3,
            ObservationDisposition::Current {
                value: ObservedValue::ComponentCondition {
                    condition: KnownComponentCondition::Available,
                },
            },
        )]);
        assert!(build_policy_information_frame(&input, 4).is_err());
    }

    #[test]
    fn duplicate_ids_and_unknown_fields_are_rejected() {
        let duplicate = record(
            "reading-a",
            0,
            0,
            ObservationDisposition::Missing {
                reason_ref: "dropout".into(),
            },
        );
        assert!(
            build_policy_information_frame(&schedule(vec![duplicate.clone(), duplicate]), 0)
                .is_err()
        );

        let mut json = serde_json::to_value(schedule(vec![record(
            "reading-b",
            0,
            0,
            ObservationDisposition::Missing {
                reason_ref: "dropout".into(),
            },
        )]))
        .unwrap();
        json["records"][0]["unreviewed_value"] = serde_json::json!(true);
        assert!(serde_json::from_value::<ObservationSchedule>(json).is_err());
    }

    #[test]
    fn tampered_policy_frame_is_rejected() {
        let mut frame = build_policy_information_frame(
            &schedule(vec![record(
                "reading",
                0,
                0,
                ObservationDisposition::Missing {
                    reason_ref: "dropout".into(),
                },
            )]),
            0,
        )
        .unwrap();
        frame.payload.observations[0].available_tick = 1;
        assert!(verify_policy_information_frame(&frame).is_err());
    }
}
