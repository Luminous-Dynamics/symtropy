// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Persistent residents, households, obligations, custody, and scoped knowledge.

use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
};
use symtropy_game_state::StableId;

/// Persistent person in the simulated world.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Resident {
    /// Stable generated or authored identity.
    pub id: StableId,
    /// Display name. Names are not identifiers.
    pub name: String,
    /// Household membership, when any.
    pub household_id: Option<StableId>,
    /// Current world location.
    pub location_id: StableId,
    /// Skills and embodied capabilities available to this person.
    pub capabilities: BTreeSet<String>,
    /// Needs whose urgency may independently drive action.
    pub needs: Vec<ResidentNeed>,
    /// Commitments this resident may prioritize over player requests.
    pub obligations: Vec<Obligation>,
    /// Observer-scoped remembered claims.
    pub knowledge: KnowledgeBase,
    /// Whether this person can currently take independent action.
    pub conscious_and_mobile: bool,
}

impl Resident {
    /// Chooses one deterministic high-priority intent from current needs and obligations.
    pub fn choose_intent(&self, context: &ResidentContext) -> ResidentIntent {
        if !self.conscious_and_mobile {
            return ResidentIntent::Wait {
                reason: "resident cannot currently act independently".into(),
            };
        }
        if let Some(need) = self
            .needs
            .iter()
            .filter(|need| need.urgency >= context.urgent_need_threshold)
            .max_by_key(|need| (need.urgency, std::cmp::Reverse(need.kind.clone())))
        {
            return ResidentIntent::MeetNeed {
                need: need.kind.clone(),
                preferred_location: need.preferred_location.clone(),
            };
        }
        if let Some(obligation) = self
            .obligations
            .iter()
            .filter(|obligation| {
                obligation.due_tick <= context.current_tick + context.lookahead_ticks
            })
            .min_by_key(|obligation| (obligation.due_tick, obligation.id.clone()))
        {
            return ResidentIntent::FulfilObligation {
                obligation_id: obligation.id.clone(),
                target_id: obligation.target_id.clone(),
            };
        }
        ResidentIntent::ContinueRoutine {
            location_id: self.location_id.clone(),
        }
    }
}

/// Resident need expressed without reducing the person to a single meter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResidentNeed {
    /// Need category such as powered-respiratory-support or reunification.
    pub kind: String,
    /// Urgency from 0 to 10,000.
    pub urgency: u16,
    /// Location or service that can plausibly meet the need.
    pub preferred_location: Option<StableId>,
    /// Consequence if unmet, phrased for explanation and playtest review.
    pub consequence: String,
}

/// Commitment that may create independent action, advocacy, delay, or refusal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Obligation {
    /// Stable obligation identity.
    pub id: StableId,
    /// Person, place, animal, item, or institution owed the commitment.
    pub target_id: StableId,
    /// Authoritative due tick.
    pub due_tick: u64,
    /// Priority from 0 to 10,000.
    pub priority: u16,
    /// Human-readable reason.
    pub reason: String,
}

/// Result of a resident making an autonomous local choice.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResidentIntent {
    /// Move or act to meet an urgent need.
    MeetNeed {
        /// Need category being pursued.
        need: String,
        /// Preferred location when known.
        preferred_location: Option<StableId>,
    },
    /// Honour a commitment without waiting for the player.
    FulfilObligation {
        /// Obligation being pursued.
        obligation_id: StableId,
        /// Target of the commitment.
        target_id: StableId,
    },
    /// Continue ordinary life at the current location.
    ContinueRoutine {
        /// Current routine location.
        location_id: StableId,
    },
    /// Remain in place because action is impossible or unsafe.
    Wait {
        /// Reason the resident is waiting.
        reason: String,
    },
}

/// Decision context shared by deterministic resident intent selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResidentContext {
    /// Current authoritative simulation tick.
    pub current_tick: u64,
    /// Window in which obligations are considered imminent.
    pub lookahead_ticks: u64,
    /// Need urgency that preempts routine and ordinary obligations.
    pub urgent_need_threshold: u16,
}

/// Household with continuity constraints and shared custody.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Household {
    /// Stable household identity.
    pub id: StableId,
    /// Resident members.
    pub member_ids: BTreeSet<StableId>,
    /// People who must remain together unless an explicit emergency exception is recorded.
    pub keep_together_groups: Vec<BTreeSet<StableId>>,
    /// Shared continuity items and their custody.
    pub continuity_items: Vec<CustodyItem>,
}

impl Household {
    /// Checks that every keep-together constraint references a member.
    pub fn validate(&self) -> Result<(), ResidentError> {
        for group in &self.keep_together_groups {
            for resident_id in group {
                if !self.member_ids.contains(resident_id) {
                    return Err(ResidentError::UnknownHouseholdMember {
                        household_id: self.id.clone(),
                        resident_id: resident_id.clone(),
                    });
                }
            }
        }
        Ok(())
    }
}

/// Item whose physical location and lawful custody must persist.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustodyItem {
    /// Stable item identity.
    pub id: StableId,
    /// Current responsible person, household, or institution.
    pub custodian_id: StableId,
    /// Current physical location.
    pub location_id: StableId,
    /// Whether transfer requires explicit consent outside a life-safety emergency.
    pub consent_required: bool,
}

/// Observer-owned collection of claims rather than omniscient world truth.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeBase {
    /// Person, device, or institution that knows these claims.
    pub owner_id: StableId,
    claims: BTreeMap<StableId, KnowledgeClaim>,
}

impl KnowledgeBase {
    /// Creates an empty observer-owned knowledge base.
    pub fn new(owner_id: StableId) -> Self {
        Self {
            owner_id,
            claims: BTreeMap::new(),
        }
    }

    /// Adds or replaces a claim with the same stable identity.
    pub fn remember(&mut self, claim: KnowledgeClaim) {
        self.claims.insert(claim.id.clone(), claim);
    }

    /// Returns a claim by identity.
    pub fn claim(&self, claim_id: &StableId) -> Option<&KnowledgeClaim> {
        self.claims.get(claim_id)
    }

    /// Returns claims that may be disclosed in the supplied context.
    pub fn disclose<'a>(
        &'a self,
        context: &'a DisclosureContext,
    ) -> impl Iterator<Item = &'a KnowledgeClaim> {
        self.claims
            .values()
            .filter(move |claim| claim.may_disclose(context))
    }
}

/// Bounded proposition with provenance, confidence, and privacy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeClaim {
    /// Stable claim identity.
    pub id: StableId,
    /// Thing or system the claim concerns.
    pub subject_id: StableId,
    /// Proposition in concise human-readable form.
    pub proposition: String,
    /// Confidence from 0 to 10,000.
    pub confidence: u16,
    /// Source person, sensor, document, or derived track.
    pub source_id: StableId,
    /// Tick at which the claim was observed or received.
    pub observed_tick: u64,
    /// Tick after which the claim should be treated as stale.
    pub stale_after_tick: Option<u64>,
    /// Disclosure boundary.
    pub privacy: ClaimPrivacy,
}

impl KnowledgeClaim {
    /// Returns whether this claim is stale at the supplied simulation tick.
    pub fn is_stale(&self, current_tick: u64) -> bool {
        self.stale_after_tick
            .is_some_and(|expiry| current_tick > expiry)
    }

    /// Applies privacy, consent, household, and emergency disclosure rules.
    pub fn may_disclose(&self, context: &DisclosureContext) -> bool {
        match &self.privacy {
            ClaimPrivacy::Public => true,
            ClaimPrivacy::Household(household_id) => {
                context.requester_household_id.as_ref() == Some(household_id)
            }
            ClaimPrivacy::ConsentRequired => context.consented_claim_ids.contains(&self.id),
            ClaimPrivacy::LifeSafetyRestricted => context.life_safety_emergency,
            ClaimPrivacy::Private => context.requester_id == self.source_id,
        }
    }
}

/// Disclosure class for observer knowledge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClaimPrivacy {
    /// Public operational or civic information.
    Public,
    /// Shared within one household.
    Household(StableId),
    /// Requires claim-specific consent.
    ConsentRequired,
    /// Disclosed only when a declared life-safety emergency makes it necessary.
    LifeSafetyRestricted,
    /// Retained by the source unless they are the requester.
    Private,
}

/// Information needed to evaluate disclosure without granting global knowledge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DisclosureContext {
    /// Person or system requesting disclosure.
    pub requester_id: StableId,
    /// Requester's household, when relevant.
    pub requester_household_id: Option<StableId>,
    /// Explicit consent for specific claims.
    pub consented_claim_ids: BTreeSet<StableId>,
    /// Whether a declared life-safety emergency is active.
    pub life_safety_emergency: bool,
}

/// Resident and household consistency failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResidentError {
    /// A keep-together group referenced a person outside the household.
    UnknownHouseholdMember {
        /// Household containing the invalid constraint.
        household_id: StableId,
        /// Unknown member identity.
        resident_id: StableId,
    },
}

impl fmt::Display for ResidentError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownHouseholdMember {
                household_id,
                resident_id,
            } => {
                write!(
                    formatter,
                    "household {household_id} references unknown member {resident_id}"
                )
            }
        }
    }
}

impl Error for ResidentError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(value: &str) -> StableId {
        StableId::parse(value).expect("test identifier is valid")
    }

    #[test]
    fn urgent_need_drives_independent_action() {
        let resident = Resident {
            id: id("resident:mina"),
            name: "Mina".into(),
            household_id: None,
            location_id: id("place:bent-feeder"),
            capabilities: BTreeSet::new(),
            needs: vec![ResidentNeed {
                kind: "powered-respiratory-support".into(),
                urgency: 9_400,
                preferred_location: Some(id("service:clinic-power")),
                consequence: "respiratory support battery will deplete".into(),
            }],
            obligations: Vec::new(),
            knowledge: KnowledgeBase::new(id("resident:mina")),
            conscious_and_mobile: true,
        };
        let intent = resident.choose_intent(&ResidentContext {
            current_tick: 100,
            lookahead_ticks: 200,
            urgent_need_threshold: 8_000,
        });
        assert!(matches!(intent, ResidentIntent::MeetNeed { .. }));
    }

    #[test]
    fn consent_required_claim_stays_private_without_consent() {
        let claim_id = id("claim:medical-power");
        let mut knowledge = KnowledgeBase::new(id("resident:mina"));
        knowledge.remember(KnowledgeClaim {
            id: claim_id.clone(),
            subject_id: id("resident:mina"),
            proposition: "uses powered respiratory support".into(),
            confidence: 10_000,
            source_id: id("resident:mina"),
            observed_tick: 1,
            stale_after_tick: None,
            privacy: ClaimPrivacy::ConsentRequired,
        });
        let mut context = DisclosureContext {
            requester_id: id("resident:technician"),
            requester_household_id: None,
            consented_claim_ids: BTreeSet::new(),
            life_safety_emergency: false,
        };
        assert_eq!(knowledge.disclose(&context).count(), 0);
        context.consented_claim_ids.insert(claim_id);
        assert_eq!(knowledge.disclose(&context).count(), 1);
    }

    #[test]
    fn household_constraints_reject_unknown_members() {
        let household = Household {
            id: id("household:44"),
            member_ids: BTreeSet::from([id("resident:mina")]),
            keep_together_groups: vec![BTreeSet::from([id("resident:mina"), id("resident:ivo")])],
            continuity_items: Vec::new(),
        };
        assert!(matches!(
            household.validate(),
            Err(ResidentError::UnknownHouseholdMember { .. })
        ));
    }
}
