// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Provider-owned mapping from residual world effects to person condition proposals.
//!
//! Protection answers how much of an already-resolved effect remains. Health owns
//! persistent conditions and capabilities. This bridge owns neither authority: it
//! applies explicit, versionable body-model rules and returns a proposal that an
//! authoritative caller may choose to commit to a person's `ConditionSet`.

use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, error::Error, fmt};
use symtropy_game_state::StableId;
use symtropy_protection_core::{
    Effect, EffectKind,
    coverage::{ProtectionContact, ProtectionRegion},
};
use symtropy_residents::condition::{
    CapabilityKind, Condition, HealthError, FULL_CAPABILITY,
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct EffectConditionRuleKey {
    pub effect_kind: EffectKind,
    /// `None` is a deliberate wildcard fallback for this effect kind.
    pub region: Option<ProtectionRegion>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EffectConditionRule {
    pub id: StableId,
    pub key: EffectConditionRuleKey,
    /// Residual magnitude that maps to the rule's configured `max_severity`.
    pub reference_magnitude: f64,
    /// Upper bound for severity produced by this rule, in 0..=10_000 basis points.
    pub max_severity: u16,
    pub condition_kind: String,
    pub capability_effects: BTreeMap<CapabilityKind, u16>,
}

impl EffectConditionRule {
    pub fn validate(&self) -> Result<(), MappingError> {
        if !self.reference_magnitude.is_finite() || self.reference_magnitude <= 0.0 {
            return Err(MappingError::InvalidRule {
                rule_id: self.id.clone(),
                field: "reference_magnitude",
            });
        }
        if self.max_severity > FULL_CAPABILITY {
            return Err(MappingError::InvalidRule {
                rule_id: self.id.clone(),
                field: "max_severity",
            });
        }
        if self.condition_kind.trim().is_empty() {
            return Err(MappingError::InvalidRule {
                rule_id: self.id.clone(),
                field: "condition_kind",
            });
        }
        if self
            .capability_effects
            .values()
            .any(|value| *value > FULL_CAPABILITY)
        {
            return Err(MappingError::InvalidRule {
                rule_id: self.id.clone(),
                field: "capability_effects",
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EffectConditionRequest {
    pub subject_id: StableId,
    pub condition_id: StableId,
    pub contact: ProtectionContact,
    pub residual: Effect,
    pub onset_tick: u64,
    pub source_event_id: StableId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConditionProposal {
    pub subject_id: StableId,
    pub rule_id: StableId,
    pub condition_id: StableId,
    pub condition_kind: String,
    pub severity: u16,
    pub onset_tick: u64,
    pub source_event_id: StableId,
    pub capability_effects: BTreeMap<CapabilityKind, u16>,
}

impl ConditionProposal {
    /// Materialize a validated health-domain value without inserting it anywhere.
    /// The owning caller still decides whether this proposal becomes authoritative.
    pub fn into_condition(self) -> Result<Condition, HealthError> {
        Condition::new(
            self.condition_id,
            self.condition_kind,
            self.severity,
            self.onset_tick,
            Some(self.source_event_id),
            self.capability_effects,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct RuleBasedConditionMapper {
    rules: BTreeMap<EffectConditionRuleKey, EffectConditionRule>,
}

impl RuleBasedConditionMapper {
    pub fn new(rules: impl IntoIterator<Item = EffectConditionRule>) -> Result<Self, MappingError> {
        let mut mapper = Self::default();
        for rule in rules {
            mapper.insert_rule(rule)?;
        }
        Ok(mapper)
    }

    pub fn insert_rule(&mut self, rule: EffectConditionRule) -> Result<(), MappingError> {
        rule.validate()?;
        if let Some(existing) = self.rules.get(&rule.key) {
            return Err(MappingError::DuplicateRuleKey {
                existing_rule_id: existing.id.clone(),
                attempted_rule_id: rule.id,
            });
        }
        self.rules.insert(rule.key.clone(), rule);
        Ok(())
    }

    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }

    /// Map one already-resolved residual effect to a condition proposal.
    ///
    /// Exact region rules win over wildcard rules. Unknown effect/region pairs fail
    /// closed rather than silently applying a generic injury. Zero residual produces
    /// no condition proposal.
    pub fn propose(
        &self,
        request: EffectConditionRequest,
    ) -> Result<Option<ConditionProposal>, MappingError> {
        request
            .residual
            .validate()
            .map_err(|_| MappingError::InvalidResidual)?;
        if request.residual.magnitude == 0.0 {
            return Ok(None);
        }

        let exact = EffectConditionRuleKey {
            effect_kind: request.residual.kind,
            region: Some(request.contact.region.clone()),
        };
        let wildcard = EffectConditionRuleKey {
            effect_kind: request.residual.kind,
            region: None,
        };
        let rule = self
            .rules
            .get(&exact)
            .or_else(|| self.rules.get(&wildcard))
            .ok_or_else(|| MappingError::NoMatchingRule {
                effect_kind: request.residual.kind,
                region: request.contact.region.clone(),
            })?;

        // Explicit quantization boundary from protection's abstract f64 magnitude into
        // health's fixed-point 0..10_000 severity space. The body-model rule supplies
        // the reference scale; protection itself never decides injury severity.
        let normalized = (request.residual.magnitude / rule.reference_magnitude).clamp(0.0, 1.0);
        let severity = (normalized * f64::from(rule.max_severity))
            .round()
            .clamp(0.0, f64::from(FULL_CAPABILITY)) as u16;

        if severity == 0 {
            return Ok(None);
        }

        Ok(Some(ConditionProposal {
            subject_id: request.subject_id,
            rule_id: rule.id.clone(),
            condition_id: request.condition_id,
            condition_kind: rule.condition_kind.clone(),
            severity,
            onset_tick: request.onset_tick,
            source_event_id: request.source_event_id,
            capability_effects: rule.capability_effects.clone(),
        }))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum MappingError {
    InvalidRule {
        rule_id: StableId,
        field: &'static str,
    },
    DuplicateRuleKey {
        existing_rule_id: StableId,
        attempted_rule_id: StableId,
    },
    InvalidResidual,
    NoMatchingRule {
        effect_kind: EffectKind,
        region: ProtectionRegion,
    },
}

impl fmt::Display for MappingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRule { rule_id, field } => {
                write!(f, "invalid condition mapping rule {rule_id}: {field}")
            }
            Self::DuplicateRuleKey {
                existing_rule_id,
                attempted_rule_id,
            } => write!(
                f,
                "condition mapping rule {attempted_rule_id} duplicates key owned by {existing_rule_id}"
            ),
            Self::InvalidResidual => write!(f, "residual effect is invalid"),
            Self::NoMatchingRule {
                effect_kind,
                region,
            } => write!(
                f,
                "no body-model mapping rule for {effect_kind:?} at {}",
                region.as_str()
            ),
        }
    }
}

impl Error for MappingError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    use symtropy_protection_core::coverage::ApproachSector;
    use symtropy_residents::condition::{CapabilityProfile, ConditionSet};

    fn id(value: &str) -> StableId {
        StableId::parse(value).unwrap()
    }

    fn region(value: &str) -> ProtectionRegion {
        ProtectionRegion::parse(value).unwrap()
    }

    fn rule(
        id_value: &str,
        target_region: Option<&str>,
        kind: &str,
        capability: CapabilityKind,
    ) -> EffectConditionRule {
        EffectConditionRule {
            id: id(id_value),
            key: EffectConditionRuleKey {
                effect_kind: EffectKind::Impact,
                region: target_region.map(region),
            },
            reference_magnitude: 100.0,
            max_severity: 10_000,
            condition_kind: kind.into(),
            capability_effects: BTreeMap::from([(capability, 8_000)]),
        }
    }

    fn request(target_region: &str, magnitude: f64) -> EffectConditionRequest {
        EffectConditionRequest {
            subject_id: id("resident:arin"),
            condition_id: id("condition:impact-1"),
            contact: ProtectionContact {
                region: region(target_region),
                approach: ApproachSector::Front,
            },
            residual: Effect {
                magnitude,
                kind: EffectKind::Impact,
            },
            onset_tick: 42,
            source_event_id: id("event:impact-residual"),
        }
    }

    #[test]
    fn zero_residual_creates_no_condition() {
        let mapper = RuleBasedConditionMapper::new([rule(
            "rule:any-impact",
            None,
            "impact-trauma",
            CapabilityKind::Stability,
        )])
        .unwrap();
        assert_eq!(mapper.propose(request("body:chest", 0.0)).unwrap(), None);
    }

    #[test]
    fn exact_region_rule_wins_over_wildcard() {
        let mapper = RuleBasedConditionMapper::new([
            rule(
                "rule:any-impact",
                None,
                "general-impact",
                CapabilityKind::Stability,
            ),
            rule(
                "rule:left-leg-impact",
                Some("body:left-leg"),
                "left-leg-impact",
                CapabilityKind::Locomotion,
            ),
        ])
        .unwrap();
        let proposal = mapper
            .propose(request("body:left-leg", 50.0))
            .unwrap()
            .unwrap();
        assert_eq!(proposal.rule_id, id("rule:left-leg-impact"));
        assert_eq!(proposal.condition_kind, "left-leg-impact");
        assert_eq!(proposal.severity, 5_000);
        assert!(proposal
            .capability_effects
            .contains_key(&CapabilityKind::Locomotion));
    }

    #[test]
    fn same_residual_can_mean_different_things_for_different_regions() {
        let mapper = RuleBasedConditionMapper::new([
            rule(
                "rule:leg",
                Some("body:left-leg"),
                "leg-trauma",
                CapabilityKind::Locomotion,
            ),
            rule(
                "rule:hand",
                Some("body:left-hand"),
                "hand-trauma",
                CapabilityKind::Manipulation,
            ),
        ])
        .unwrap();
        let leg = mapper
            .propose(request("body:left-leg", 40.0))
            .unwrap()
            .unwrap();
        let hand = mapper
            .propose(request("body:left-hand", 40.0))
            .unwrap()
            .unwrap();
        assert_ne!(leg.condition_kind, hand.condition_kind);
        assert_ne!(leg.capability_effects, hand.capability_effects);
    }

    #[test]
    fn unknown_region_without_wildcard_fails_closed() {
        let mapper = RuleBasedConditionMapper::new([rule(
            "rule:leg",
            Some("body:left-leg"),
            "leg-trauma",
            CapabilityKind::Locomotion,
        )])
        .unwrap();
        assert!(matches!(
            mapper.propose(request("body:chest", 20.0)),
            Err(MappingError::NoMatchingRule { .. })
        ));
    }

    #[test]
    fn mapping_does_not_mutate_health_until_caller_commits() {
        let mapper = RuleBasedConditionMapper::new([rule(
            "rule:leg",
            Some("body:left-leg"),
            "leg-trauma",
            CapabilityKind::Locomotion,
        )])
        .unwrap();
        let mut conditions = ConditionSet::default();
        let proposal = mapper
            .propose(request("body:left-leg", 50.0))
            .unwrap()
            .unwrap();
        assert_eq!(conditions.iter().count(), 0);
        assert_eq!(proposal.subject_id, id("resident:arin"));
        assert_eq!(proposal.source_event_id, id("event:impact-residual"));
        assert_eq!(proposal.onset_tick, 42);

        let condition = proposal.into_condition().unwrap();
        conditions.insert(condition);
        assert_eq!(conditions.iter().count(), 1);
    }

    #[test]
    fn committed_condition_affects_only_declared_capability() {
        let mapper = RuleBasedConditionMapper::new([rule(
            "rule:leg",
            Some("body:left-leg"),
            "leg-trauma",
            CapabilityKind::Locomotion,
        )])
        .unwrap();
        let condition = mapper
            .propose(request("body:left-leg", 50.0))
            .unwrap()
            .unwrap()
            .into_condition()
            .unwrap();
        let mut conditions = ConditionSet::default();
        conditions.insert(condition);
        let mut profile = CapabilityProfile::default();
        profile.set(CapabilityKind::Locomotion, 10_000).unwrap();
        profile.set(CapabilityKind::Cognition, 9_000).unwrap();
        assert!(profile
            .assess(CapabilityKind::Locomotion, &conditions)
            .effective
            < 10_000);
        assert_eq!(
            profile
                .assess(CapabilityKind::Cognition, &conditions)
                .effective,
            9_000
        );
    }

    #[test]
    fn duplicate_rule_key_is_rejected() {
        let key_region = Some("body:left-leg");
        assert!(matches!(
            RuleBasedConditionMapper::new([
                rule(
                    "rule:first",
                    key_region,
                    "first",
                    CapabilityKind::Locomotion,
                ),
                rule(
                    "rule:second",
                    key_region,
                    "second",
                    CapabilityKind::Stability,
                ),
            ]),
            Err(MappingError::DuplicateRuleKey { .. })
        ));
    }

    #[test]
    fn deterministic_mapping_is_input_stable() {
        let rules = vec![
            rule(
                "rule:leg",
                Some("body:left-leg"),
                "leg-trauma",
                CapabilityKind::Locomotion,
            ),
            rule(
                "rule:any",
                None,
                "general-impact",
                CapabilityKind::Stability,
            ),
        ];
        let a = RuleBasedConditionMapper::new(rules.clone()).unwrap();
        let b = RuleBasedConditionMapper::new(rules.into_iter().rev()).unwrap();
        assert_eq!(
            a.propose(request("body:left-leg", 33.0)).unwrap(),
            b.propose(request("body:left-leg", 33.0)).unwrap()
        );
    }

    #[test]
    fn rule_region_type_remains_generic_not_anatomy_locked() {
        let mapper = RuleBasedConditionMapper::new([rule(
            "rule:generic",
            Some("wearer:mobility-interface"),
            "interface-overload",
            CapabilityKind::Locomotion,
        )])
        .unwrap();
        let proposal = mapper
            .propose(request("wearer:mobility-interface", 25.0))
            .unwrap()
            .unwrap();
        let regions = BTreeSet::from([region("wearer:mobility-interface")]);
        assert!(regions.contains(&ProtectionRegion::parse("wearer:mobility-interface").unwrap()));
        assert_eq!(proposal.condition_kind, "interface-overload");
    }
}
