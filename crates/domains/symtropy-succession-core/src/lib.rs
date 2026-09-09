// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Succession policies and competing office claims.
//!
//! This crate deliberately does **not** install office holders, recognize a
//! claimant, measure legitimacy, or declare civil war. A succession/selection
//! procedure may produce a claim; the institution's current office record remains
//! owned by `symtropy-civilization-core`, and external recognition remains a
//! separate authority surface.

use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
};
use symtropy_civilization_core::Institution;
use symtropy_epistemics_core::EpistemicRef;
use symtropy_game_state::StableId;

/// Extensible selection mechanism rather than a privileged House/Kingdom enum.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SelectionMechanism {
    /// Broad domain such as `succession`, `election`, `appointment`, or scenario-specific vocabulary.
    pub namespace: String,
    /// Exact mechanism such as `eldest-eligible`, `member-ballot-v2`, or `board-appointment`.
    pub method: String,
}

/// Immutable institution-local succession/selection policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SuccessionPolicy {
    /// Stable policy identity.
    pub id: StableId,
    /// Institution whose office the policy concerns.
    pub institution_id: StableId,
    /// Exact office governed by the policy.
    pub office_id: StableId,
    /// Extensible selection mechanism.
    pub mechanism: SelectionMechanism,
    /// First canonical tick at which this policy may ground a claim.
    pub valid_from_tick: u64,
    /// Optional final canonical tick, inclusive.
    pub valid_until_tick: Option<u64>,
    /// Causal-history event that adopted/recorded this policy.
    pub source_event_id: StableId,
}

impl SuccessionPolicy {
    /// Returns whether the policy is active at `tick`.
    pub fn is_active(&self, tick: u64) -> bool {
        tick >= self.valid_from_tick
            && self
                .valid_until_tick
                .is_none_or(|until| tick <= until)
    }
}

/// Explicit grounds under which an actor asserts an office claim.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClaimGrounds {
    /// Claim cites one registered institution-local succession policy.
    Policy(StableId),
    /// Claim is asserted outside the currently registered policy corpus.
    ///
    /// This is not automatically invalid or illegitimate; it is simply not
    /// promoted into policy-backed status by the engine.
    ExtraInstitutional {
        /// Broad claim family such as `customary`, `revolutionary`, `external-charter`.
        namespace: String,
        /// Scenario/content-defined exact basis label.
        basis: String,
    },
}

/// Immutable claim by one actor to one institution office.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OfficeClaim {
    /// Stable claim identity.
    pub id: StableId,
    /// Institution whose office is claimed.
    pub institution_id: StableId,
    /// Claimed office.
    pub office_id: StableId,
    /// Actor asserting the claim.
    pub claimant_id: StableId,
    /// Explicit policy-backed or extra-institutional grounds.
    pub grounds: ClaimGrounds,
    /// Canonical tick at which the claim was asserted.
    pub asserted_tick: u64,
    /// Optional final tick after which the claim is no longer active.
    pub valid_until_tick: Option<u64>,
    /// Optional evidence/belief/assertion/record references supporting the claim.
    ///
    /// These typed references are provenance only here; the owning epistemic
    /// authority decides whether they exist/currently apply.
    pub epistemic_basis: BTreeSet<EpistemicRef>,
    /// Causal-history event that introduced the claim.
    pub source_event_id: StableId,
}

impl OfficeClaim {
    /// Returns whether the claim's own temporal window includes `tick`.
    pub fn temporally_active(&self, tick: u64) -> bool {
        tick >= self.asserted_tick
            && self
                .valid_until_tick
                .is_none_or(|until| tick <= until)
    }
}

/// Immutable withdrawal of a prior office claim.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaimWithdrawal {
    /// Stable withdrawal-record identity.
    pub id: StableId,
    /// Claim being withdrawn.
    pub claim_id: StableId,
    /// Canonical tick of withdrawal.
    pub withdrawn_tick: u64,
    /// Causal-history event that records withdrawal.
    pub source_event_id: StableId,
}

/// Dependency-light succession policy/claim registry.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SuccessionLedger {
    policies: BTreeMap<StableId, SuccessionPolicy>,
    claims: BTreeMap<StableId, OfficeClaim>,
    withdrawals: BTreeMap<StableId, ClaimWithdrawal>,
    withdrawal_by_claim: BTreeMap<StableId, StableId>,
}

impl SuccessionLedger {
    /// Creates an empty succession ledger.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers one immutable institution-local succession policy.
    pub fn register_policy(
        &mut self,
        institution: &Institution,
        policy: SuccessionPolicy,
    ) -> Result<(), SuccessionError> {
        validate_institution_office(institution, &policy.institution_id, &policy.office_id)?;
        validate_mechanism(&policy.mechanism)?;
        validate_window(
            policy.valid_from_tick,
            policy.valid_until_tick,
            &policy.id,
        )?;
        if self.policies.contains_key(&policy.id) {
            return Err(SuccessionError::DuplicatePolicy(policy.id));
        }
        self.policies.insert(policy.id.clone(), policy);
        Ok(())
    }

    /// Returns one registered policy.
    pub fn policy(&self, policy_id: &StableId) -> Option<&SuccessionPolicy> {
        self.policies.get(policy_id)
    }

    /// Records one immutable office claim.
    ///
    /// This method does not mutate `Institution::office_holders`.
    pub fn submit_claim(
        &mut self,
        institution: &Institution,
        claim: OfficeClaim,
    ) -> Result<(), SuccessionError> {
        validate_institution_office(institution, &claim.institution_id, &claim.office_id)?;
        validate_window(claim.asserted_tick, claim.valid_until_tick, &claim.id)?;
        if self.claims.contains_key(&claim.id) {
            return Err(SuccessionError::DuplicateClaim(claim.id));
        }

        match &claim.grounds {
            ClaimGrounds::Policy(policy_id) => {
                let policy = self
                    .policies
                    .get(policy_id)
                    .ok_or_else(|| SuccessionError::UnknownPolicy(policy_id.clone()))?;
                if policy.institution_id != claim.institution_id || policy.office_id != claim.office_id {
                    return Err(SuccessionError::PolicyScopeMismatch {
                        policy_id: policy_id.clone(),
                        claim_id: claim.id,
                    });
                }
                if !policy.is_active(claim.asserted_tick) {
                    return Err(SuccessionError::InactivePolicy {
                        policy_id: policy_id.clone(),
                        claim_tick: claim.asserted_tick,
                    });
                }
            }
            ClaimGrounds::ExtraInstitutional { namespace, basis } => {
                if namespace.trim().is_empty() || basis.trim().is_empty() {
                    return Err(SuccessionError::EmptyExtraInstitutionalGrounds(claim.id));
                }
            }
        }

        self.claims.insert(claim.id.clone(), claim);
        Ok(())
    }

    /// Returns one immutable claim whether active or withdrawn/expired.
    pub fn claim(&self, claim_id: &StableId) -> Option<&OfficeClaim> {
        self.claims.get(claim_id)
    }

    /// Records an immutable withdrawal. A claim may be withdrawn at most once in V0.
    pub fn withdraw_claim(&mut self, withdrawal: ClaimWithdrawal) -> Result<(), SuccessionError> {
        if self.withdrawals.contains_key(&withdrawal.id) {
            return Err(SuccessionError::DuplicateWithdrawal(withdrawal.id));
        }
        let claim = self
            .claims
            .get(&withdrawal.claim_id)
            .ok_or_else(|| SuccessionError::UnknownClaim(withdrawal.claim_id.clone()))?;
        if self.withdrawal_by_claim.contains_key(&withdrawal.claim_id) {
            return Err(SuccessionError::ClaimAlreadyWithdrawn(
                withdrawal.claim_id,
            ));
        }
        if withdrawal.withdrawn_tick < claim.asserted_tick {
            return Err(SuccessionError::WithdrawalBeforeClaim {
                claim_id: withdrawal.claim_id,
                asserted_tick: claim.asserted_tick,
                withdrawn_tick: withdrawal.withdrawn_tick,
            });
        }
        self.withdrawal_by_claim
            .insert(withdrawal.claim_id.clone(), withdrawal.id.clone());
        self.withdrawals
            .insert(withdrawal.id.clone(), withdrawal);
        Ok(())
    }

    /// Returns the withdrawal record for a claim, when one exists.
    pub fn withdrawal_for_claim(&self, claim_id: &StableId) -> Option<&ClaimWithdrawal> {
        self.withdrawal_by_claim
            .get(claim_id)
            .and_then(|withdrawal_id| self.withdrawals.get(withdrawal_id))
    }

    /// Returns whether one exact claim is active at `tick`.
    pub fn claim_is_active(&self, claim_id: &StableId, tick: u64) -> bool {
        self.claims.get(claim_id).is_some_and(|claim| {
            claim.temporally_active(tick)
                && self
                    .withdrawal_for_claim(claim_id)
                    .is_none_or(|withdrawal| tick < withdrawal.withdrawn_tick)
        })
    }

    /// Returns all simultaneously active claims to one exact office.
    ///
    /// Multiple results are a normal state. This helper does not label that state
    /// a civil war, legitimacy crisis, or invalidity condition.
    pub fn active_claims_for_office<'a>(
        &'a self,
        institution_id: &'a StableId,
        office_id: &'a StableId,
        tick: u64,
    ) -> impl Iterator<Item = &'a OfficeClaim> {
        self.claims.values().filter(move |claim| {
            claim.institution_id == *institution_id
                && claim.office_id == *office_id
                && self.claim_is_active(&claim.id, tick)
        })
    }
}

fn validate_institution_office(
    institution: &Institution,
    institution_id: &StableId,
    office_id: &StableId,
) -> Result<(), SuccessionError> {
    if institution.id != *institution_id {
        return Err(SuccessionError::InstitutionMismatch {
            expected: institution.id.clone(),
            actual: institution_id.clone(),
        });
    }
    if institution.offices().any(|office| office.id == *office_id) {
        Ok(())
    } else {
        Err(SuccessionError::UnknownOffice(office_id.clone()))
    }
}

fn validate_mechanism(mechanism: &SelectionMechanism) -> Result<(), SuccessionError> {
    if mechanism.namespace.trim().is_empty() || mechanism.method.trim().is_empty() {
        Err(SuccessionError::EmptySelectionMechanism)
    } else {
        Ok(())
    }
}

fn validate_window(
    valid_from_tick: u64,
    valid_until_tick: Option<u64>,
    id: &StableId,
) -> Result<(), SuccessionError> {
    if valid_until_tick.is_some_and(|until| until < valid_from_tick) {
        Err(SuccessionError::InvalidTemporalWindow {
            id: id.clone(),
            valid_from_tick,
            valid_until_tick,
        })
    } else {
        Ok(())
    }
}

/// Structural failures in succession-policy/claim state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SuccessionError {
    DuplicatePolicy(StableId),
    UnknownPolicy(StableId),
    DuplicateClaim(StableId),
    UnknownClaim(StableId),
    DuplicateWithdrawal(StableId),
    ClaimAlreadyWithdrawn(StableId),
    InstitutionMismatch {
        expected: StableId,
        actual: StableId,
    },
    UnknownOffice(StableId),
    EmptySelectionMechanism,
    EmptyExtraInstitutionalGrounds(StableId),
    InvalidTemporalWindow {
        id: StableId,
        valid_from_tick: u64,
        valid_until_tick: Option<u64>,
    },
    PolicyScopeMismatch {
        policy_id: StableId,
        claim_id: StableId,
    },
    InactivePolicy {
        policy_id: StableId,
        claim_tick: u64,
    },
    WithdrawalBeforeClaim {
        claim_id: StableId,
        asserted_tick: u64,
        withdrawn_tick: u64,
    },
}

impl fmt::Display for SuccessionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicatePolicy(id) => write!(formatter, "succession policy {id} already exists"),
            Self::UnknownPolicy(id) => write!(formatter, "unknown succession policy {id}"),
            Self::DuplicateClaim(id) => write!(formatter, "office claim {id} already exists"),
            Self::UnknownClaim(id) => write!(formatter, "unknown office claim {id}"),
            Self::DuplicateWithdrawal(id) => write!(formatter, "claim withdrawal {id} already exists"),
            Self::ClaimAlreadyWithdrawn(id) => write!(formatter, "office claim {id} is already withdrawn"),
            Self::InstitutionMismatch { expected, actual } => write!(
                formatter,
                "institution mismatch: expected {expected}, got {actual}"
            ),
            Self::UnknownOffice(id) => write!(formatter, "unknown office {id}"),
            Self::EmptySelectionMechanism => formatter.write_str("selection mechanism namespace/method must be non-empty"),
            Self::EmptyExtraInstitutionalGrounds(id) => {
                write!(formatter, "office claim {id} has empty extra-institutional grounds")
            }
            Self::InvalidTemporalWindow {
                id,
                valid_from_tick,
                valid_until_tick,
            } => write!(
                formatter,
                "{id} has invalid temporal window {valid_from_tick}..={valid_until_tick:?}"
            ),
            Self::PolicyScopeMismatch { policy_id, claim_id } => write!(
                formatter,
                "policy {policy_id} does not govern the institution/office claimed by {claim_id}"
            ),
            Self::InactivePolicy {
                policy_id,
                claim_tick,
            } => write!(
                formatter,
                "policy {policy_id} is inactive at claim tick {claim_tick}"
            ),
            Self::WithdrawalBeforeClaim {
                claim_id,
                asserted_tick,
                withdrawn_tick,
            } => write!(
                formatter,
                "claim {claim_id} asserted at {asserted_tick} cannot be withdrawn at earlier tick {withdrawn_tick}"
            ),
        }
    }
}

impl Error for SuccessionError {}

#[cfg(test)]
mod tests {
    use super::*;
    use symtropy_civilization_core::{OfficeDefinition, OfficeHolderRecord};

    fn id(value: &str) -> StableId {
        StableId::parse(value).expect("test id is valid")
    }

    fn institution() -> Institution {
        let mut institution = Institution::new(id("institution:helion"), "Helion Compact");
        institution
            .define_office(OfficeDefinition {
                id: id("office:chancellor"),
                name: "Chancellor".into(),
                tags: BTreeSet::new(),
            })
            .expect("define office");
        institution
            .record_office_holder(OfficeHolderRecord {
                office_id: id("office:chancellor"),
                holder_id: id("actor:old-chancellor"),
                recorded_tick: 5,
                source_event_id: id("event:old-appointment"),
            })
            .expect("record incumbent");
        institution
    }

    fn policy(id_value: &str, method: &str) -> SuccessionPolicy {
        SuccessionPolicy {
            id: id(id_value),
            institution_id: id("institution:helion"),
            office_id: id("office:chancellor"),
            mechanism: SelectionMechanism {
                namespace: "succession".into(),
                method: method.into(),
            },
            valid_from_tick: 1,
            valid_until_tick: None,
            source_event_id: id("event:policy"),
        }
    }

    #[test]
    fn competing_claims_coexist_without_installing_a_holder() {
        let institution = institution();
        let mut ledger = SuccessionLedger::new();
        ledger
            .register_policy(&institution, policy("policy:charter", "charter-selection-v1"))
            .expect("policy");

        ledger
            .submit_claim(
                &institution,
                OfficeClaim {
                    id: id("claim:mara"),
                    institution_id: id("institution:helion"),
                    office_id: id("office:chancellor"),
                    claimant_id: id("actor:mara"),
                    grounds: ClaimGrounds::Policy(id("policy:charter")),
                    asserted_tick: 10,
                    valid_until_tick: None,
                    epistemic_basis: BTreeSet::new(),
                    source_event_id: id("event:mara-claims"),
                },
            )
            .expect("policy claim");
        ledger
            .submit_claim(
                &institution,
                OfficeClaim {
                    id: id("claim:kael"),
                    institution_id: id("institution:helion"),
                    office_id: id("office:chancellor"),
                    claimant_id: id("actor:kael"),
                    grounds: ClaimGrounds::ExtraInstitutional {
                        namespace: "fleet-acclamation".into(),
                        basis: "command-council-declaration".into(),
                    },
                    asserted_tick: 11,
                    valid_until_tick: None,
                    epistemic_basis: BTreeSet::new(),
                    source_event_id: id("event:kael-claims"),
                },
            )
            .expect("extra-institutional claim");

        assert_eq!(
            ledger
                .active_claims_for_office(
                    &id("institution:helion"),
                    &id("office:chancellor"),
                    12,
                )
                .count(),
            2
        );
        let holder = institution
            .office_holders()
            .find(|holder| holder.office_id == id("office:chancellor"))
            .expect("incumbent record remains");
        assert_eq!(holder.holder_id, id("actor:old-chancellor"));
    }

    #[test]
    fn policy_claim_must_match_policy_scope() {
        let institution = institution();
        let mut ledger = SuccessionLedger::new();
        let mut wrong = policy("policy:wrong", "appointment");
        wrong.office_id = id("office:other");
        assert!(matches!(
            ledger.register_policy(&institution, wrong),
            Err(SuccessionError::UnknownOffice(_))
        ));
    }

    #[test]
    fn withdrawal_ends_future_activity_without_erasing_claim() {
        let institution = institution();
        let mut ledger = SuccessionLedger::new();
        ledger
            .register_policy(&institution, policy("policy:charter", "charter-selection-v1"))
            .expect("policy");
        ledger
            .submit_claim(
                &institution,
                OfficeClaim {
                    id: id("claim:mara"),
                    institution_id: id("institution:helion"),
                    office_id: id("office:chancellor"),
                    claimant_id: id("actor:mara"),
                    grounds: ClaimGrounds::Policy(id("policy:charter")),
                    asserted_tick: 10,
                    valid_until_tick: None,
                    epistemic_basis: BTreeSet::new(),
                    source_event_id: id("event:mara-claims"),
                },
            )
            .expect("claim");
        ledger
            .withdraw_claim(ClaimWithdrawal {
                id: id("withdrawal:mara"),
                claim_id: id("claim:mara"),
                withdrawn_tick: 20,
                source_event_id: id("event:mara-withdraws"),
            })
            .expect("withdraw");

        assert!(ledger.claim_is_active(&id("claim:mara"), 19));
        assert!(!ledger.claim_is_active(&id("claim:mara"), 20));
        assert!(ledger.claim(&id("claim:mara")).is_some());
        assert!(ledger.withdrawal_for_claim(&id("claim:mara")).is_some());
    }

    #[test]
    fn claim_is_not_active_before_it_is_asserted() {
        let institution = institution();
        let mut ledger = SuccessionLedger::new();
        ledger
            .submit_claim(
                &institution,
                OfficeClaim {
                    id: id("claim:custom"),
                    institution_id: id("institution:helion"),
                    office_id: id("office:chancellor"),
                    claimant_id: id("actor:custom"),
                    grounds: ClaimGrounds::ExtraInstitutional {
                        namespace: "customary".into(),
                        basis: "frontier-charter".into(),
                    },
                    asserted_tick: 50,
                    valid_until_tick: Some(100),
                    epistemic_basis: BTreeSet::new(),
                    source_event_id: id("event:custom-claim"),
                },
            )
            .expect("claim");
        assert!(!ledger.claim_is_active(&id("claim:custom"), 49));
        assert!(ledger.claim_is_active(&id("claim:custom"), 50));
        assert!(!ledger.claim_is_active(&id("claim:custom"), 101));
    }

    #[test]
    fn empty_extra_institutional_ground_rejects() {
        let institution = institution();
        let mut ledger = SuccessionLedger::new();
        let result = ledger.submit_claim(
            &institution,
            OfficeClaim {
                id: id("claim:bad"),
                institution_id: id("institution:helion"),
                office_id: id("office:chancellor"),
                claimant_id: id("actor:bad"),
                grounds: ClaimGrounds::ExtraInstitutional {
                    namespace: String::new(),
                    basis: "some basis".into(),
                },
                asserted_tick: 1,
                valid_until_tick: None,
                epistemic_basis: BTreeSet::new(),
                source_event_id: id("event:bad"),
            },
        );
        assert!(matches!(
            result,
            Err(SuccessionError::EmptyExtraInstitutionalGrounds(_))
        ));
    }
}
