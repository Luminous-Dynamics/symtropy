// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Explicit admission of external player-organization evidence.
//!
//! External collaboration systems such as Mycelix may provide identity,
//! governance, treasury, project, or membership records. Those records are
//! evidence from an external provider; they are never fictional civilization
//! truth merely because they exist. One sealed policy mapping and one explicit
//! admission decision are required before a product adapter may use them as
//! inputs to institution/project/authority mutations.

use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
};
use symtropy_game_state::StableId;

/// External collaboration/governance provider identity.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct CollaborationProvider {
    pub id: StableId,
    /// Human-readable implementation family, e.g. `mycelix-holochain`.
    pub namespace: String,
}

/// Stable external organization identity under one provider.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ExternalOrganizationRef {
    pub provider_id: StableId,
    pub organization_id: String,
}

/// Immutable provider record with externally stable provenance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalOrganizationRecord {
    pub id: StableId,
    pub organization: ExternalOrganizationRef,
    /// Provider-specific semantic namespace, such as `governance.proposal`.
    pub record_namespace: String,
    /// Stable provider-native record/action/hash identifier.
    pub provider_record_id: String,
    /// Optional actor identity in provider-native form (DID, agent pubkey, etc.).
    pub external_actor_id: Option<String>,
    /// Optional record subject in provider-native form.
    pub external_subject_id: Option<String>,
    pub observed_tick: u64,
    /// Event that admitted the observation itself into Symtropy history.
    pub source_event_id: StableId,
}

impl ExternalOrganizationRecord {
    pub fn validate(&self) -> Result<(), PlayerOrgError> {
        if self.organization.organization_id.trim().is_empty() {
            return Err(PlayerOrgError::EmptyExternalOrganizationId);
        }
        if self.record_namespace.trim().is_empty() {
            return Err(PlayerOrgError::EmptyRecordNamespace);
        }
        if self.provider_record_id.trim().is_empty() {
            return Err(PlayerOrgError::EmptyProviderRecordId);
        }
        Ok(())
    }
}

/// Civilization-side effect class an external record may be proposed to support.
///
/// These are intents/evidence categories only; this crate exposes no API that
/// mutates the corresponding owning domain.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AdmissionEffectKind {
    InstitutionalMembership,
    InstitutionalOfficeRecord,
    InstitutionalAuthorityGrant,
    ProjectPublication,
    ProjectContributionReview,
    TreasuryAuthorization,
    PublicGovernanceRecord,
    Custom { namespace: String, operation: String },
}

/// Exact proposed civilization target for one admitted external record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdmissionTarget {
    /// Civilization institution/project/etc. that remains authoritative over mutation.
    pub target_id: StableId,
    pub effect: AdmissionEffectKind,
    /// Optional exact internal actor mapped by product identity policy.
    pub internal_actor_id: Option<StableId>,
    /// Optional exact internal subject mapped by product policy.
    pub internal_subject_id: Option<StableId>,
}

/// One allow-list rule binding external provider/organization/record semantics to
/// a bounded class of civilization-side effect proposals.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdmissionRule {
    pub id: StableId,
    pub provider_id: StableId,
    pub external_organization_id: String,
    pub record_namespace: String,
    pub allowed_effects: BTreeSet<AdmissionEffectKind>,
    /// Optional exact institution/project scope. `None` means policy does not
    /// constrain target identity, not that external data becomes authoritative.
    pub allowed_target_id: Option<StableId>,
}

impl AdmissionRule {
    fn validate(&self) -> Result<(), PlayerOrgError> {
        if self.external_organization_id.trim().is_empty() {
            return Err(PlayerOrgError::EmptyExternalOrganizationId);
        }
        if self.record_namespace.trim().is_empty() {
            return Err(PlayerOrgError::EmptyRecordNamespace);
        }
        if self.allowed_effects.is_empty() {
            return Err(PlayerOrgError::EmptyAllowedEffectSet(self.id.clone()));
        }
        Ok(())
    }
}

/// Immutable sealed admission-policy generation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdmissionPolicy {
    pub id: StableId,
    pub generation: u64,
    rules: BTreeMap<StableId, AdmissionRule>,
}

impl AdmissionPolicy {
    pub fn new(
        id: StableId,
        generation: u64,
        rules: impl IntoIterator<Item = AdmissionRule>,
    ) -> Result<Self, PlayerOrgError> {
        let mut map = BTreeMap::new();
        for rule in rules {
            rule.validate()?;
            if map.insert(rule.id.clone(), rule).is_some() {
                return Err(PlayerOrgError::DuplicateRule);
            }
        }
        if map.is_empty() {
            return Err(PlayerOrgError::EmptyPolicy);
        }
        Ok(Self {
            id,
            generation,
            rules: map,
        })
    }

    pub fn rule(&self, rule_id: &StableId) -> Option<&AdmissionRule> {
        self.rules.get(rule_id)
    }
}

/// Request to admit one exact external record as evidence for one bounded target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdmissionRequest {
    pub id: StableId,
    pub policy_id: StableId,
    pub policy_generation: u64,
    pub rule_id: StableId,
    pub external_record_id: StableId,
    pub target: AdmissionTarget,
    pub requested_tick: u64,
    pub source_event_id: StableId,
}

/// Explicit reviewer/admission authority identity. It is not inferred from the
/// external provider or organization.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct AdmissionReviewer {
    pub id: StableId,
}

/// Immutable successful admission receipt.
///
/// This receipt still does not mutate institution/project/treasury state. A
/// domain adapter may consume it together with that domain's own authority checks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdmittedOrganizationAction {
    pub admission_request_id: StableId,
    pub external_record: ExternalOrganizationRecord,
    pub target: AdmissionTarget,
    pub reviewer: AdmissionReviewer,
    pub policy_id: StableId,
    pub policy_generation: u64,
    pub rule_id: StableId,
    pub admitted_tick: u64,
    pub source_event_id: StableId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct CommittedAdmission {
    request: AdmissionRequest,
    result: AdmittedOrganizationAction,
}

/// Provider-neutral external collaboration evidence/admission authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerOrganizationAdmission {
    providers: BTreeMap<StableId, CollaborationProvider>,
    external_records: BTreeMap<StableId, ExternalOrganizationRecord>,
    policy: AdmissionPolicy,
    authorized_reviewers: BTreeSet<AdmissionReviewer>,
    committed: BTreeMap<StableId, CommittedAdmission>,
}

impl PlayerOrganizationAdmission {
    pub fn new(
        providers: impl IntoIterator<Item = CollaborationProvider>,
        policy: AdmissionPolicy,
        authorized_reviewers: impl IntoIterator<Item = AdmissionReviewer>,
    ) -> Result<Self, PlayerOrgError> {
        let mut provider_map = BTreeMap::new();
        for provider in providers {
            if provider.namespace.trim().is_empty() {
                return Err(PlayerOrgError::EmptyProviderNamespace(provider.id));
            }
            if provider_map.insert(provider.id.clone(), provider).is_some() {
                return Err(PlayerOrgError::DuplicateProvider);
            }
        }
        if provider_map.is_empty() {
            return Err(PlayerOrgError::NoProviders);
        }
        let reviewers = authorized_reviewers.into_iter().collect::<BTreeSet<_>>();
        if reviewers.is_empty() {
            return Err(PlayerOrgError::NoAuthorizedReviewers);
        }
        for rule in policy.rules.values() {
            if !provider_map.contains_key(&rule.provider_id) {
                return Err(PlayerOrgError::UnknownProviderInRule(rule.provider_id.clone()));
            }
        }
        Ok(Self {
            providers: provider_map,
            external_records: BTreeMap::new(),
            policy,
            authorized_reviewers: reviewers,
            committed: BTreeMap::new(),
        })
    }

    /// Record external evidence. Observation alone changes no civilization state.
    pub fn observe_external_record(
        &mut self,
        record: ExternalOrganizationRecord,
    ) -> Result<(), PlayerOrgError> {
        record.validate()?;
        if !self.providers.contains_key(&record.organization.provider_id) {
            return Err(PlayerOrgError::UnknownProvider(
                record.organization.provider_id,
            ));
        }
        if let Some(existing) = self.external_records.get(&record.id) {
            if existing == &record {
                return Ok(());
            }
            return Err(PlayerOrgError::ConflictingExternalRecordId(record.id));
        }
        self.external_records.insert(record.id.clone(), record);
        Ok(())
    }

    /// Explicitly admit one external record under a bounded rule.
    pub fn admit(
        &mut self,
        request: AdmissionRequest,
        reviewer: AdmissionReviewer,
        admitted_tick: u64,
        source_event_id: StableId,
    ) -> Result<AdmittedOrganizationAction, PlayerOrgError> {
        if let Some(committed) = self.committed.get(&request.id) {
            if committed.request == request && committed.result.reviewer == reviewer {
                return Ok(committed.result.clone());
            }
            return Err(PlayerOrgError::ConflictingAdmissionRequestId(request.id));
        }
        if !self.authorized_reviewers.contains(&reviewer) {
            return Err(PlayerOrgError::UnauthorizedReviewer(reviewer.id));
        }
        if request.policy_id != self.policy.id
            || request.policy_generation != self.policy.generation
        {
            return Err(PlayerOrgError::StaleAdmissionPolicy {
                expected_id: self.policy.id.clone(),
                expected_generation: self.policy.generation,
                supplied_id: request.policy_id,
                supplied_generation: request.policy_generation,
            });
        }
        let rule = self
            .policy
            .rule(&request.rule_id)
            .ok_or_else(|| PlayerOrgError::UnknownRule(request.rule_id.clone()))?;
        let record = self
            .external_records
            .get(&request.external_record_id)
            .ok_or_else(|| PlayerOrgError::UnknownExternalRecord(request.external_record_id.clone()))?;

        if record.organization.provider_id != rule.provider_id
            || record.organization.organization_id != rule.external_organization_id
            || record.record_namespace != rule.record_namespace
        {
            return Err(PlayerOrgError::RecordDoesNotMatchRule {
                record_id: record.id.clone(),
                rule_id: rule.id.clone(),
            });
        }
        if !rule.allowed_effects.contains(&request.target.effect) {
            return Err(PlayerOrgError::EffectNotAllowed {
                rule_id: rule.id.clone(),
                effect: request.target.effect,
            });
        }
        if let Some(target) = &rule.allowed_target_id
            && target != &request.target.target_id
        {
            return Err(PlayerOrgError::TargetNotAllowed {
                rule_id: rule.id.clone(),
                expected: target.clone(),
                supplied: request.target.target_id,
            });
        }
        if request.requested_tick < record.observed_tick {
            return Err(PlayerOrgError::AdmissionRequestedBeforeObservation {
                record_id: record.id.clone(),
                observed_tick: record.observed_tick,
                requested_tick: request.requested_tick,
            });
        }
        if admitted_tick < request.requested_tick {
            return Err(PlayerOrgError::AdmissionBeforeRequest(request.id));
        }

        let result = AdmittedOrganizationAction {
            admission_request_id: request.id.clone(),
            external_record: record.clone(),
            target: request.target.clone(),
            reviewer: reviewer.clone(),
            policy_id: self.policy.id.clone(),
            policy_generation: self.policy.generation,
            rule_id: rule.id.clone(),
            admitted_tick,
            source_event_id,
        };
        self.committed.insert(
            request.id.clone(),
            CommittedAdmission {
                request,
                result: result.clone(),
            },
        );
        Ok(result)
    }

    pub fn external_record(&self, id: &StableId) -> Option<&ExternalOrganizationRecord> {
        self.external_records.get(id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlayerOrgError {
    EmptyExternalOrganizationId,
    EmptyRecordNamespace,
    EmptyProviderRecordId,
    EmptyAllowedEffectSet(StableId),
    DuplicateRule,
    EmptyPolicy,
    EmptyProviderNamespace(StableId),
    DuplicateProvider,
    NoProviders,
    NoAuthorizedReviewers,
    UnknownProviderInRule(StableId),
    UnknownProvider(StableId),
    ConflictingExternalRecordId(StableId),
    ConflictingAdmissionRequestId(StableId),
    UnauthorizedReviewer(StableId),
    StaleAdmissionPolicy {
        expected_id: StableId,
        expected_generation: u64,
        supplied_id: StableId,
        supplied_generation: u64,
    },
    UnknownRule(StableId),
    UnknownExternalRecord(StableId),
    RecordDoesNotMatchRule { record_id: StableId, rule_id: StableId },
    EffectNotAllowed { rule_id: StableId, effect: AdmissionEffectKind },
    TargetNotAllowed { rule_id: StableId, expected: StableId, supplied: StableId },
    AdmissionRequestedBeforeObservation {
        record_id: StableId,
        observed_tick: u64,
        requested_tick: u64,
    },
    AdmissionBeforeRequest(StableId),
}

impl fmt::Display for PlayerOrgError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyExternalOrganizationId => f.write_str("external organization id is empty"),
            Self::EmptyRecordNamespace => f.write_str("external record namespace is empty"),
            Self::EmptyProviderRecordId => f.write_str("external provider record id is empty"),
            Self::EmptyAllowedEffectSet(id) => write!(f, "admission rule {id} allows no effects"),
            Self::DuplicateRule => f.write_str("duplicate admission rule id"),
            Self::EmptyPolicy => f.write_str("admission policy contains no rules"),
            Self::EmptyProviderNamespace(id) => write!(f, "provider {id} has an empty namespace"),
            Self::DuplicateProvider => f.write_str("duplicate collaboration provider id"),
            Self::NoProviders => f.write_str("no collaboration providers configured"),
            Self::NoAuthorizedReviewers => f.write_str("no external-record admission reviewers configured"),
            Self::UnknownProviderInRule(id) => write!(f, "admission rule references unknown provider {id}"),
            Self::UnknownProvider(id) => write!(f, "unknown collaboration provider {id}"),
            Self::ConflictingExternalRecordId(id) => write!(f, "external record id {id} was reused with different content"),
            Self::ConflictingAdmissionRequestId(id) => write!(f, "admission request id {id} was reused with different content"),
            Self::UnauthorizedReviewer(id) => write!(f, "reviewer {id} is not authorized to admit external records"),
            Self::StaleAdmissionPolicy { .. } => f.write_str("admission request does not match current exact policy generation"),
            Self::UnknownRule(id) => write!(f, "unknown admission rule {id}"),
            Self::UnknownExternalRecord(id) => write!(f, "unknown external organization record {id}"),
            Self::RecordDoesNotMatchRule { record_id, rule_id } => write!(f, "external record {record_id} does not match admission rule {rule_id}"),
            Self::EffectNotAllowed { rule_id, .. } => write!(f, "requested effect is not allowed by admission rule {rule_id}"),
            Self::TargetNotAllowed { rule_id, .. } => write!(f, "requested target is not allowed by admission rule {rule_id}"),
            Self::AdmissionRequestedBeforeObservation { record_id, .. } => write!(f, "admission for {record_id} was requested before the record was observed"),
            Self::AdmissionBeforeRequest(id) => write!(f, "admission {id} completed before its request"),
        }
    }
}

impl Error for PlayerOrgError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(value: &str) -> StableId {
        StableId::parse(value).expect("test id")
    }

    fn provider() -> CollaborationProvider {
        CollaborationProvider {
            id: id("provider:mycelix"),
            namespace: "mycelix-holochain".into(),
        }
    }

    fn policy() -> AdmissionPolicy {
        AdmissionPolicy::new(
            id("policy:mycelix-org"),
            3,
            [AdmissionRule {
                id: id("rule:proposal-public-record"),
                provider_id: id("provider:mycelix"),
                external_organization_id: "org:helix".into(),
                record_namespace: "governance.proposal-submitted".into(),
                allowed_effects: BTreeSet::from([AdmissionEffectKind::PublicGovernanceRecord]),
                allowed_target_id: Some(id("institution:helix")),
            }],
        )
        .expect("policy")
    }

    fn world() -> PlayerOrganizationAdmission {
        PlayerOrganizationAdmission::new(
            [provider()],
            policy(),
            [AdmissionReviewer { id: id("reviewer:player-org") }],
        )
        .expect("world")
    }

    fn record() -> ExternalOrganizationRecord {
        ExternalOrganizationRecord {
            id: id("external:proposal:abc"),
            organization: ExternalOrganizationRef {
                provider_id: id("provider:mycelix"),
                organization_id: "org:helix".into(),
            },
            record_namespace: "governance.proposal-submitted".into(),
            provider_record_id: "uhCkk-action-hash".into(),
            external_actor_id: Some("did:mycelix:alice".into()),
            external_subject_id: Some("proposal:17".into()),
            observed_tick: 100,
            source_event_id: id("event:observe-proposal"),
        }
    }

    #[test]
    fn external_record_alone_creates_no_admitted_action() {
        let mut world = world();
        world.observe_external_record(record()).expect("observe");
        assert!(world.external_record(&id("external:proposal:abc")).is_some());
        // There is deliberately no institution mutation or implicit admission API.
    }

    #[test]
    fn matching_record_can_be_explicitly_admitted_to_bounded_effect() {
        let mut world = world();
        world.observe_external_record(record()).expect("observe");
        let admitted = world
            .admit(
                AdmissionRequest {
                    id: id("admission:proposal:17"),
                    policy_id: id("policy:mycelix-org"),
                    policy_generation: 3,
                    rule_id: id("rule:proposal-public-record"),
                    external_record_id: id("external:proposal:abc"),
                    target: AdmissionTarget {
                        target_id: id("institution:helix"),
                        effect: AdmissionEffectKind::PublicGovernanceRecord,
                        internal_actor_id: None,
                        internal_subject_id: None,
                    },
                    requested_tick: 110,
                    source_event_id: id("event:admission-request"),
                },
                AdmissionReviewer { id: id("reviewer:player-org") },
                120,
                id("event:admission"),
            )
            .expect("admit");
        assert_eq!(admitted.target.target_id, id("institution:helix"));
        assert_eq!(admitted.policy_generation, 3);
    }

    #[test]
    fn provider_record_cannot_escalate_to_unallowed_authority_grant() {
        let mut world = world();
        world.observe_external_record(record()).expect("observe");
        let result = world.admit(
            AdmissionRequest {
                id: id("admission:escalation"),
                policy_id: id("policy:mycelix-org"),
                policy_generation: 3,
                rule_id: id("rule:proposal-public-record"),
                external_record_id: id("external:proposal:abc"),
                target: AdmissionTarget {
                    target_id: id("institution:helix"),
                    effect: AdmissionEffectKind::InstitutionalAuthorityGrant,
                    internal_actor_id: Some(id("resident:alice")),
                    internal_subject_id: None,
                },
                requested_tick: 110,
                source_event_id: id("event:escalation-request"),
            },
            AdmissionReviewer { id: id("reviewer:player-org") },
            120,
            id("event:escalation"),
        );
        assert!(matches!(result, Err(PlayerOrgError::EffectNotAllowed { .. })));
    }

    #[test]
    fn stale_policy_generation_fails_closed() {
        let mut world = world();
        world.observe_external_record(record()).expect("observe");
        let result = world.admit(
            AdmissionRequest {
                id: id("admission:stale"),
                policy_id: id("policy:mycelix-org"),
                policy_generation: 2,
                rule_id: id("rule:proposal-public-record"),
                external_record_id: id("external:proposal:abc"),
                target: AdmissionTarget {
                    target_id: id("institution:helix"),
                    effect: AdmissionEffectKind::PublicGovernanceRecord,
                    internal_actor_id: None,
                    internal_subject_id: None,
                },
                requested_tick: 110,
                source_event_id: id("event:stale-request"),
            },
            AdmissionReviewer { id: id("reviewer:player-org") },
            120,
            id("event:stale"),
        );
        assert!(matches!(result, Err(PlayerOrgError::StaleAdmissionPolicy { .. })));
    }
}
