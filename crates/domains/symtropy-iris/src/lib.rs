// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Bounded IRIS assistance over observer-scoped evidence and explicit permissions.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use symtropy_game_state::StableId;
use symtropy_residents::{DisclosureContext, KnowledgeBase, KnowledgeClaim};

/// Confidence language shown to players instead of false precision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConfidenceBand {
    /// Evidence is absent or too contradictory to support a useful claim.
    Unknown,
    /// A possibility worth investigating, not acting on as fact.
    Possible,
    /// Leading explanation with meaningful unresolved alternatives.
    Likely,
    /// Multiple independent observations support the claim.
    Strong,
    /// Direct verified observation within its validity window.
    Confirmed,
}

impl ConfidenceBand {
    fn from_score(score: u16) -> Self {
        match score {
            0..=1_999 => Self::Unknown,
            2_000..=4_499 => Self::Possible,
            4_500..=7_499 => Self::Likely,
            7_500..=9_499 => Self::Strong,
            _ => Self::Confirmed,
        }
    }
}

/// Authority explicitly granted to IRIS.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum IrisPermission {
    /// Read observations from one instrument or observer.
    ReadSource(StableId),
    /// Operate a specific device within its local safety interlocks.
    ControlDevice(StableId),
    /// Release one protected claim.
    DiscloseClaim(StableId),
    /// Issue an emergency warning without ordinary publication delay.
    EmergencyBroadcast,
}

/// Candidate explanation maintained separately from raw observations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hypothesis {
    /// Stable hypothesis identity.
    pub id: StableId,
    /// Subject being explained.
    pub subject_id: StableId,
    /// Concise proposition.
    pub proposition: String,
    /// Confidence from 0 to 10,000.
    pub confidence: u16,
    /// Claims supporting the proposition.
    pub supporting_claim_ids: BTreeSet<StableId>,
    /// Claims that conflict with the proposition.
    pub contradicting_claim_ids: BTreeSet<StableId>,
    /// Next observation that would most reduce uncertainty.
    pub recommended_observation: Option<String>,
}

/// Bounded answer produced from disclosed knowledge and explicit uncertainty.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IrisAssessment {
    /// Best supported statement IRIS can currently make.
    pub statement: String,
    /// Human-legible confidence band.
    pub confidence: ConfidenceBand,
    /// Evidence claims actually used.
    pub evidence_claim_ids: Vec<StableId>,
    /// Important contradiction or missing information.
    pub uncertainty: Vec<String>,
    /// Whether IRIS refused an action or disclosure.
    pub refused: bool,
}

/// Requested physical action rather than an informational question.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceActionRequest {
    /// Device to control.
    pub device_id: StableId,
    /// Requested action.
    pub action: String,
    /// Whether a wrong action could immediately threaten life or cause cascading damage.
    pub life_safety_critical: bool,
    /// Minimum confidence required by the local operating procedure.
    pub minimum_confidence: u16,
}

/// Observer-bound IRIS state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IrisState {
    /// Stable assistant identity.
    pub id: StableId,
    /// Knowledge available to this IRIS instance.
    pub knowledge: KnowledgeBase,
    /// Explicit permissions; absence means no authority.
    pub permissions: BTreeSet<IrisPermission>,
    /// Current candidate explanations.
    pub hypotheses: BTreeMap<StableId, Hypothesis>,
}

impl IrisState {
    /// Assesses a subject using only claims disclosable in the supplied context.
    pub fn assess_subject(
        &self,
        subject_id: &StableId,
        current_tick: u64,
        disclosure: &DisclosureContext,
    ) -> IrisAssessment {
        let mut claims: Vec<&KnowledgeClaim> = self
            .knowledge
            .disclose(disclosure)
            .filter(|claim| &claim.subject_id == subject_id)
            .collect();
        claims.sort_by_key(|claim| {
            (
                std::cmp::Reverse(effective_confidence(claim, current_tick)),
                claim.id.clone(),
            )
        });

        let Some(primary) = claims.first() else {
            return IrisAssessment {
                statement: "I do not have a disclosed observation that supports a conclusion."
                    .into(),
                confidence: ConfidenceBand::Unknown,
                evidence_claim_ids: Vec::new(),
                uncertainty: vec![
                    "Inspect locally or request access to a relevant instrument or witness.".into(),
                ],
                refused: false,
            };
        };

        let primary_score = effective_confidence(primary, current_tick);
        let mut uncertainty = Vec::new();
        let contradictions: Vec<&KnowledgeClaim> = claims
            .iter()
            .copied()
            .filter(|claim| claim.id != primary.id && claim.proposition != primary.proposition)
            .collect();
        if !contradictions.is_empty() {
            uncertainty.push(format!(
                "{} disclosed observation(s) support a different account.",
                contradictions.len()
            ));
        }
        if primary.is_stale(current_tick) {
            uncertainty.push("The leading observation is stale and should be rechecked.".into());
        }
        if let Some(hypothesis) = self
            .hypotheses
            .values()
            .filter(|hypothesis| &hypothesis.subject_id == subject_id)
            .max_by_key(|hypothesis| hypothesis.confidence)
            && let Some(next) = &hypothesis.recommended_observation
        {
            uncertainty.push(format!("Highest-value next observation: {next}"));
        }

        IrisAssessment {
            statement: primary.proposition.clone(),
            confidence: ConfidenceBand::from_score(primary_score),
            evidence_claim_ids: claims
                .iter()
                .take(4)
                .map(|claim| claim.id.clone())
                .collect(),
            uncertainty,
            refused: false,
        }
    }

    /// Evaluates whether IRIS may issue a physical control command.
    pub fn evaluate_device_action(
        &self,
        request: &DeviceActionRequest,
        assessed_confidence: u16,
    ) -> IrisAssessment {
        if !self
            .permissions
            .contains(&IrisPermission::ControlDevice(request.device_id.clone()))
        {
            return IrisAssessment {
                statement: format!("I do not have control authority for {}.", request.device_id),
                confidence: ConfidenceBand::Confirmed,
                evidence_claim_ids: Vec::new(),
                uncertainty: vec![
                    "A local operator may inspect or grant bounded authority.".into(),
                ],
                refused: true,
            };
        }
        if assessed_confidence < request.minimum_confidence {
            return IrisAssessment {
                statement: format!(
                    "I will not issue '{}' with the current evidence.",
                    request.action
                ),
                confidence: ConfidenceBand::from_score(assessed_confidence),
                evidence_claim_ids: Vec::new(),
                uncertainty: vec![format!(
                    "Procedure requires confidence {} but the current assessment is {}.",
                    request.minimum_confidence, assessed_confidence
                )],
                refused: true,
            };
        }
        if request.life_safety_critical && assessed_confidence < 9_000 {
            return IrisAssessment {
                statement: "I will not automate a life-safety-critical action below strong independent confirmation.".into(),
                confidence: ConfidenceBand::from_score(assessed_confidence),
                evidence_claim_ids: Vec::new(),
                uncertainty: vec!["A human operator must verify local state and accept responsibility.".into()],
                refused: true,
            };
        }
        IrisAssessment {
            statement: format!(
                "The bounded command '{}' may be issued to {}.",
                request.action, request.device_id
            ),
            confidence: ConfidenceBand::from_score(assessed_confidence),
            evidence_claim_ids: Vec::new(),
            uncertainty: vec!["Local interlocks and operator stop authority remain active.".into()],
            refused: false,
        }
    }
}

fn effective_confidence(claim: &KnowledgeClaim, current_tick: u64) -> u16 {
    if claim.is_stale(current_tick) {
        claim.confidence / 2
    } else {
        claim.confidence
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use symtropy_residents::{ClaimPrivacy, KnowledgeClaim};

    fn id(value: &str) -> StableId {
        StableId::parse(value).expect("test identifier is valid")
    }

    fn disclosure() -> DisclosureContext {
        DisclosureContext {
            requester_id: id("resident:technician"),
            requester_household_id: None,
            consented_claim_ids: BTreeSet::new(),
            life_safety_emergency: false,
        }
    }

    #[test]
    fn iris_refuses_device_without_authority() {
        let iris = IrisState {
            id: id("iris:firstlight"),
            knowledge: KnowledgeBase::new(id("iris:firstlight")),
            permissions: BTreeSet::new(),
            hypotheses: BTreeMap::new(),
        };
        let assessment = iris.evaluate_device_action(
            &DeviceActionRequest {
                device_id: id("device:bent-feeder-breaker"),
                action: "open breaker".into(),
                life_safety_critical: true,
                minimum_confidence: 9_000,
            },
            9_900,
        );
        assert!(assessment.refused);
        assert!(
            assessment
                .statement
                .contains("do not have control authority")
        );
    }

    #[test]
    fn stale_claim_is_downgraded_and_explained() {
        let owner = id("iris:firstlight");
        let subject = id("device:bent-feeder");
        let mut knowledge = KnowledgeBase::new(owner.clone());
        knowledge.remember(KnowledgeClaim {
            id: id("claim:temperature"),
            subject_id: subject.clone(),
            proposition: "outlet temperature is 42.3 C".into(),
            confidence: 9_600,
            source_id: id("sensor:outlet-temperature"),
            observed_tick: 10,
            stale_after_tick: Some(20),
            privacy: ClaimPrivacy::Public,
        });
        let iris = IrisState {
            id: owner,
            knowledge,
            permissions: BTreeSet::new(),
            hypotheses: BTreeMap::new(),
        };
        let assessment = iris.assess_subject(&subject, 50, &disclosure());
        assert_eq!(assessment.confidence, ConfidenceBand::Likely);
        assert!(
            assessment
                .uncertainty
                .iter()
                .any(|line| line.contains("stale"))
        );
    }

    #[test]
    fn contradictory_claims_remain_visible() {
        let owner = id("iris:firstlight");
        let subject = id("fault:bent-feeder");
        let mut knowledge = KnowledgeBase::new(owner.clone());
        for (claim, proposition, confidence) in [
            (
                "claim:thermal",
                "support movement caused insulation damage",
                8_400,
            ),
            (
                "claim:firmware",
                "converter firmware caused current oscillation",
                6_800,
            ),
        ] {
            knowledge.remember(KnowledgeClaim {
                id: id(claim),
                subject_id: subject.clone(),
                proposition: proposition.into(),
                confidence,
                source_id: id("observer:joint-team"),
                observed_tick: 10,
                stale_after_tick: None,
                privacy: ClaimPrivacy::Public,
            });
        }
        let iris = IrisState {
            id: owner,
            knowledge,
            permissions: BTreeSet::new(),
            hypotheses: BTreeMap::new(),
        };
        let assessment = iris.assess_subject(&subject, 12, &disclosure());
        assert!(
            assessment
                .uncertainty
                .iter()
                .any(|line| line.contains("different account"))
        );
    }
}
