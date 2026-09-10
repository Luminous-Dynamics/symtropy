// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Persistent person-level condition -> capability semantics.
//!
//! This module intentionally avoids a single hit-point scalar. Conditions remain
//! explicit persistent facts, while gameplay consumers ask what the person can
//! currently do. The model is fixed-point and deterministic so save/replay and
//! multiscale simulation do not depend on floating-point health accumulation.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use symtropy_game_state::StableId;

pub const FULL_CAPABILITY: u16 = 10_000;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum CapabilityKind {
    Locomotion,
    Manipulation,
    Stability,
    Endurance,
    Sensing,
    Communication,
    Cognition,
    Carrying,
    Custom(String),
}

/// One persistent condition that may impair one or more capabilities.
///
/// `severity` and every capability effect use basis points in [0, 10_000]. An
/// effect of 10_000 means a maximally severe instance of this condition can fully
/// remove that capability; lower severities scale that maximum effect linearly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Condition {
    pub id: StableId,
    pub kind: String,
    pub severity: u16,
    pub onset_tick: u64,
    pub source_event_id: Option<StableId>,
    pub last_revision_tick: u64,
    pub last_revision_event_id: Option<StableId>,
    pub capability_effects: BTreeMap<CapabilityKind, u16>,
}

impl Condition {
    pub fn new(
        id: StableId,
        kind: impl Into<String>,
        severity: u16,
        onset_tick: u64,
        source_event_id: Option<StableId>,
        capability_effects: BTreeMap<CapabilityKind, u16>,
    ) -> Result<Self, HealthError> {
        let kind = kind.into();
        if kind.trim().is_empty() {
            return Err(HealthError::EmptyConditionKind);
        }
        validate_score("severity", severity)?;
        for effect in capability_effects.values().copied() {
            validate_score("capability_effect", effect)?;
        }
        Ok(Self {
            id,
            kind,
            severity,
            onset_tick,
            last_revision_tick: onset_tick,
            last_revision_event_id: source_event_id.clone(),
            source_event_id,
            capability_effects,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ConditionSet {
    conditions: BTreeMap<StableId, Condition>,
}

impl ConditionSet {
    pub fn insert(&mut self, condition: Condition) -> Option<Condition> {
        self.conditions.insert(condition.id.clone(), condition)
    }

    pub fn get(&self, id: &StableId) -> Option<&Condition> {
        self.conditions.get(id)
    }

    pub fn remove(&mut self, id: &StableId) -> Option<Condition> {
        self.conditions.remove(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Condition> {
        self.conditions.values()
    }

    /// Revise one persistent condition with explicit causal provenance.
    ///
    /// This method does not decide *why* severity changed. Treatment, recovery,
    /// deterioration, surgery, rest, or environmental exposure are owned by higher
    /// layers; they must provide the causal event and monotonic simulation tick.
    pub fn revise_severity(
        &mut self,
        id: &StableId,
        new_severity: u16,
        revision_tick: u64,
        revision_event_id: StableId,
    ) -> Result<u16, HealthError> {
        validate_score("severity", new_severity)?;
        let condition = self
            .conditions
            .get_mut(id)
            .ok_or_else(|| HealthError::UnknownCondition(id.clone()))?;
        if revision_tick < condition.last_revision_tick {
            return Err(HealthError::NonMonotonicRevision {
                condition_id: id.clone(),
                previous_tick: condition.last_revision_tick,
                attempted_tick: revision_tick,
            });
        }
        condition.severity = new_severity;
        condition.last_revision_tick = revision_tick;
        condition.last_revision_event_id = Some(revision_event_id);
        Ok(condition.severity)
    }
}

/// Baseline person capability before current conditions are applied.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CapabilityProfile {
    baseline: BTreeMap<CapabilityKind, u16>,
}

impl CapabilityProfile {
    pub fn set(&mut self, capability: CapabilityKind, score: u16) -> Result<(), HealthError> {
        validate_score("baseline_capability", score)?;
        self.baseline.insert(capability, score);
        Ok(())
    }

    pub fn baseline(&self, capability: &CapabilityKind) -> u16 {
        self.baseline.get(capability).copied().unwrap_or(0)
    }

    /// Assess one capability under all current conditions.
    ///
    /// Independent condition impacts compose multiplicatively. This prevents two
    /// moderate conditions from producing impossible negative capability and avoids
    /// a hidden global "health" scalar. Integer arithmetic keeps the result stable.
    pub fn assess(
        &self,
        capability: CapabilityKind,
        conditions: &ConditionSet,
    ) -> CapabilityAssessment {
        let baseline = self.baseline(&capability);
        let mut effective = u32::from(baseline);
        let mut limiting_conditions = BTreeSet::new();

        for condition in conditions.iter() {
            let Some(max_effect) = condition.capability_effects.get(&capability).copied() else {
                continue;
            };
            if condition.severity == 0 || max_effect == 0 {
                continue;
            }

            // severity * max_effect / 10_000 gives the condition's current
            // impairment fraction in basis points. Round to nearest basis point.
            let impairment_bp =
                (u32::from(condition.severity) * u32::from(max_effect) + 5_000) / 10_000;
            let retained_bp = 10_000u32.saturating_sub(impairment_bp.min(10_000));
            effective = (effective * retained_bp + 5_000) / 10_000;
            limiting_conditions.insert(condition.id.clone());
        }

        let effective = effective.min(u32::from(FULL_CAPABILITY)) as u16;
        CapabilityAssessment {
            capability,
            baseline,
            effective,
            deficit: baseline.saturating_sub(effective),
            limiting_conditions,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityAssessment {
    pub capability: CapabilityKind,
    pub baseline: u16,
    pub effective: u16,
    pub deficit: u16,
    pub limiting_conditions: BTreeSet<StableId>,
}

impl CapabilityAssessment {
    /// Fraction of baseline capability in basis points; undefined if baseline is zero.
    pub fn fraction_of_baseline_bp(&self) -> Option<u16> {
        if self.baseline == 0 {
            return None;
        }
        Some(
            ((u32::from(self.effective) * 10_000 + u32::from(self.baseline) / 2)
                / u32::from(self.baseline))
                .min(10_000) as u16,
        )
    }
}

fn validate_score(field: &'static str, value: u16) -> Result<(), HealthError> {
    if value > FULL_CAPABILITY {
        return Err(HealthError::ScoreOutOfRange { field, value });
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HealthError {
    EmptyConditionKind,
    ScoreOutOfRange {
        field: &'static str,
        value: u16,
    },
    UnknownCondition(StableId),
    NonMonotonicRevision {
        condition_id: StableId,
        previous_tick: u64,
        attempted_tick: u64,
    },
}

impl std::fmt::Display for HealthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyConditionKind => write!(f, "condition kind must not be empty"),
            Self::ScoreOutOfRange { field, value } => {
                write!(f, "{field} score must be in 0..=10000, got {value}")
            }
            Self::UnknownCondition(id) => write!(f, "unknown condition {id}"),
            Self::NonMonotonicRevision {
                condition_id,
                previous_tick,
                attempted_tick,
            } => write!(
                f,
                "condition {condition_id} revision tick {attempted_tick} precedes prior revision tick {previous_tick}"
            ),
        }
    }
}

impl std::error::Error for HealthError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(value: &str) -> StableId {
        StableId::parse(value).unwrap()
    }

    fn leg_condition(id_value: &str, severity: u16) -> Condition {
        Condition::new(
            id(id_value),
            "left-leg-trauma",
            severity,
            100,
            Some(id("event:impact")),
            BTreeMap::from([
                (CapabilityKind::Locomotion, 8_000),
                (CapabilityKind::Stability, 5_000),
            ]),
        )
        .unwrap()
    }

    #[test]
    fn one_condition_reduces_only_declared_capabilities() {
        let mut profile = CapabilityProfile::default();
        profile.set(CapabilityKind::Locomotion, 10_000).unwrap();
        profile.set(CapabilityKind::Cognition, 9_000).unwrap();
        let mut conditions = ConditionSet::default();
        conditions.insert(leg_condition("condition:leg", 5_000));

        let locomotion = profile.assess(CapabilityKind::Locomotion, &conditions);
        let cognition = profile.assess(CapabilityKind::Cognition, &conditions);
        assert_eq!(locomotion.effective, 6_000);
        assert_eq!(cognition.effective, 9_000);
    }

    #[test]
    fn multiple_conditions_compose_without_negative_capability() {
        let mut profile = CapabilityProfile::default();
        profile.set(CapabilityKind::Locomotion, 10_000).unwrap();
        let mut conditions = ConditionSet::default();
        conditions.insert(leg_condition("condition:a", 7_500));
        conditions.insert(leg_condition("condition:b", 7_500));
        let result = profile.assess(CapabilityKind::Locomotion, &conditions);
        assert!(result.effective <= result.baseline);
        assert_eq!(result.limiting_conditions.len(), 2);
    }

    #[test]
    fn condition_revision_requires_monotonic_causal_provenance() {
        let mut profile = CapabilityProfile::default();
        profile.set(CapabilityKind::Locomotion, 10_000).unwrap();
        let mut conditions = ConditionSet::default();
        let condition_id = id("condition:leg");
        conditions.insert(leg_condition("condition:leg", 8_000));
        let before = profile.assess(CapabilityKind::Locomotion, &conditions);
        conditions
            .revise_severity(&condition_id, 4_000, 200, id("event:treatment"))
            .unwrap();
        let after = profile.assess(CapabilityKind::Locomotion, &conditions);
        assert_eq!(profile.baseline(&CapabilityKind::Locomotion), 10_000);
        assert!(after.effective > before.effective);
        let revised = conditions.get(&condition_id).unwrap();
        assert_eq!(revised.severity, 4_000);
        assert_eq!(revised.last_revision_tick, 200);
        assert_eq!(
            revised.last_revision_event_id.as_ref(),
            Some(&id("event:treatment"))
        );
        assert!(matches!(
            conditions.revise_severity(&condition_id, 3_000, 199, id("event:stale")),
            Err(HealthError::NonMonotonicRevision { .. })
        ));
    }

    #[test]
    fn zero_baseline_fraction_is_explicitly_undefined() {
        let profile = CapabilityProfile::default();
        let assessment = profile.assess(CapabilityKind::Locomotion, &ConditionSet::default());
        assert_eq!(assessment.fraction_of_baseline_bp(), None);
    }

    #[test]
    fn deterministic_condition_order_does_not_change_result() {
        let mut profile = CapabilityProfile::default();
        profile.set(CapabilityKind::Locomotion, 9_000).unwrap();
        let a = leg_condition("condition:a", 4_000);
        let b = leg_condition("condition:b", 6_000);
        let mut first = ConditionSet::default();
        first.insert(a.clone());
        first.insert(b.clone());
        let mut second = ConditionSet::default();
        second.insert(b);
        second.insert(a);
        assert_eq!(
            profile.assess(CapabilityKind::Locomotion, &first),
            profile.assess(CapabilityKind::Locomotion, &second)
        );
    }
}
