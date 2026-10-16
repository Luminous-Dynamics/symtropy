// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Fail-closed, multi-temporal selection of transport-policy rule versions.
//!
//! A rule is selectable only when it was publicly published and present in the
//! replay corpus by the query's knowledge cutoff, and when it was effective at
//! the event time. Scope and explicit supersession are evaluated independently.
//! This module resolves rule versions; it does not implement Canal allocation,
//! infer awards, establish source rights, or prove physical transit feasibility.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

/// UTC Unix time in whole seconds. Date-string parsing and local-time conversion
/// belong at the source-ingestion boundary, not in this policy-version selector.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PolicyTime(i64);

impl PolicyTime {
    pub const fn from_unix_seconds(seconds: i64) -> Self {
        Self(seconds)
    }

    pub const fn unix_seconds(self) -> i64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PolicyLockSystem {
    Panamax,
    Neopanamax,
}

/// A missing scope field means the rule places no restriction on that dimension.
/// Missing query context never satisfies a rule that requires that dimension.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PolicyScope {
    pub lock_system: Option<PolicyLockSystem>,
    pub market_segment: Option<String>,
    pub booking_period: Option<String>,
}

impl PolicyScope {
    pub fn any() -> Self {
        Self::default()
    }

    pub fn lock_system(lock_system: PolicyLockSystem) -> Self {
        Self {
            lock_system: Some(lock_system),
            ..Self::default()
        }
    }

    fn matches(&self, query: &Self) -> bool {
        self.lock_system
            .is_none_or(|required| query.lock_system == Some(required))
            && self
                .market_segment
                .as_ref()
                .is_none_or(|required| query.market_segment.as_ref() == Some(required))
            && self
                .booking_period
                .as_ref()
                .is_none_or(|required| query.booking_period.as_ref() == Some(required))
    }

    fn has_blank_label(&self) -> bool {
        self.market_segment
            .as_ref()
            .is_some_and(|value| value.trim().is_empty())
            || self
                .booking_period
                .as_ref()
                .is_some_and(|value| value.trim().is_empty())
    }
}

/// Qualification requires independent review of source bytes/hash, clause
/// locator, and permitted use. A URL or transcription alone is not sufficient.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PolicyEvidenceStatus {
    Qualified,
    Unqualified { reason: String },
}

/// One immutable version of a logical rule family.
///
/// published_at prevents public-knowledge lookahead. recorded_at prevents replaying
/// evidence before it entered the declared corpus. effective_from is inclusive
/// and effective_until is exclusive. Versions only supersede named predecessors.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolicyRuleVersion {
    pub family_id: String,
    pub version_id: String,
    pub published_at: PolicyTime,
    pub recorded_at: PolicyTime,
    pub effective_from: PolicyTime,
    pub effective_until: Option<PolicyTime>,
    pub scope: PolicyScope,
    pub supersedes: BTreeSet<String>,
    pub evidence_status: PolicyEvidenceStatus,
    pub source_sha256: Option<String>,
    pub source_locator: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PolicyBookError {
    InvalidRule { version_id: String, reason: &'static str },
    DuplicateVersionId(String),
    UnknownSupersededVersion { version_id: String, superseded_version_id: String },
    CrossFamilySupersession { version_id: String, superseded_version_id: String },
    SupersessionCycle(String),
}

impl fmt::Display for PolicyBookError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRule { version_id, reason } =>
                write!(formatter, "invalid policy rule {version_id}: {reason}"),
            Self::DuplicateVersionId(version_id) =>
                write!(formatter, "duplicate policy rule version ID {version_id}"),
            Self::UnknownSupersededVersion { version_id, superseded_version_id } =>
                write!(formatter, "policy rule {version_id} supersedes unknown version {superseded_version_id}"),
            Self::CrossFamilySupersession { version_id, superseded_version_id } =>
                write!(formatter, "policy rule {version_id} supersedes version {superseded_version_id} from another family"),
            Self::SupersessionCycle(version_id) =>
                write!(formatter, "policy supersession cycle includes {version_id}"),
        }
    }
}

impl std::error::Error for PolicyBookError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolicyQuery {
    pub family_id: String,
    pub event_time: PolicyTime,
    pub knowledge_cutoff: PolicyTime,
    pub scope: PolicyScope,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PolicyResolution {
    Resolved {
        family_id: String,
        version_id: String,
        source_sha256: String,
        source_locator: String,
    },
    UnknownFamily { family_id: String },
    NoKnownVersion { family_id: String },
    OutOfScope { family_id: String },
    NoEffectiveVersion { family_id: String },
    UnqualifiedEvidence { family_id: String, version_id: String, reason: String },
    Ambiguous { family_id: String, version_ids: Vec<String> },
}

/// Validated, immutable index of policy versions with provenance and acyclic history.
#[derive(Clone, Debug, Default)]
pub struct PolicyRuleBook {
    by_family: BTreeMap<String, Vec<PolicyRuleVersion>>,
    by_version: BTreeMap<String, PolicyRuleVersion>,
}

impl PolicyRuleBook {
    pub fn new(rules: Vec<PolicyRuleVersion>) -> Result<Self, PolicyBookError> {
        let mut by_version = BTreeMap::new();

        for rule in rules {
            if rule.family_id.trim().is_empty() {
                return Err(PolicyBookError::InvalidRule {
                    version_id: rule.version_id.clone(),
                    reason: "family ID is blank",
                });
            }
            if rule.version_id.trim().is_empty() {
                return Err(PolicyBookError::InvalidRule {
                    version_id: rule.version_id.clone(),
                    reason: "version ID is blank",
                });
            }
            if rule.effective_until.is_some_and(|until| until <= rule.effective_from) {
                return Err(PolicyBookError::InvalidRule {
                    version_id: rule.version_id.clone(),
                    reason: "effective interval must be non-empty and half-open",
                });
            }
            if rule.scope.has_blank_label() {
                return Err(PolicyBookError::InvalidRule {
                    version_id: rule.version_id.clone(),
                    reason: "scope contains a blank label",
                });
            }
            if rule.evidence_status == PolicyEvidenceStatus::Qualified {
                let digest_valid = rule.source_sha256.as_ref().is_some_and(|digest| {
                    digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
                });
                if !digest_valid {
                    return Err(PolicyBookError::InvalidRule {
                        version_id: rule.version_id.clone(),
                        reason: "qualified evidence requires a 64-character SHA-256 digest",
                    });
                }
                if rule.source_locator.as_ref().is_none_or(|locator| locator.trim().is_empty()) {
                    return Err(PolicyBookError::InvalidRule {
                        version_id: rule.version_id.clone(),
                        reason: "qualified evidence requires an exact source locator",
                    });
                }
            }

            let version_id = rule.version_id.clone();
            if by_version.insert(version_id.clone(), rule).is_some() {
                return Err(PolicyBookError::DuplicateVersionId(version_id));
            }
        }

        for rule in by_version.values() {
            for superseded_id in &rule.supersedes {
                let Some(superseded) = by_version.get(superseded_id) else {
                    return Err(PolicyBookError::UnknownSupersededVersion {
                        version_id: rule.version_id.clone(),
                        superseded_version_id: superseded_id.clone(),
                    });
                };
                if superseded.family_id != rule.family_id {
                    return Err(PolicyBookError::CrossFamilySupersession {
                        version_id: rule.version_id.clone(),
                        superseded_version_id: superseded_id.clone(),
                    });
                }
            }
        }

        // A cycle could make every simultaneously active rule suppress another.
        for start_id in by_version.keys() {
            let mut pending = vec![start_id.clone()];
            let mut visited = BTreeSet::new();
            while let Some(current_id) = pending.pop() {
                if !visited.insert(current_id.clone()) {
                    continue;
                }
                if let Some(current) = by_version.get(&current_id) {
                    for predecessor_id in &current.supersedes {
                        if predecessor_id == start_id {
                            return Err(PolicyBookError::SupersessionCycle(start_id.clone()));
                        }
                        pending.push(predecessor_id.clone());
                    }
                }
            }
        }

        let mut by_family: BTreeMap<String, Vec<PolicyRuleVersion>> = BTreeMap::new();
        for rule in by_version.values() {
            by_family.entry(rule.family_id.clone()).or_default().push(rule.clone());
        }

        Ok(Self { by_family, by_version })
    }

    pub fn resolve(&self, query: &PolicyQuery) -> PolicyResolution {
        let Some(family_rules) = self.by_family.get(&query.family_id) else {
            return PolicyResolution::UnknownFamily { family_id: query.family_id.clone() };
        };

        // Both clocks matter: publication alone is not enough if this corpus
        // did not contain the source at the simulated knowledge cutoff.
        let known = family_rules.iter().filter(|rule| {
            rule.published_at <= query.knowledge_cutoff
                && rule.recorded_at <= query.knowledge_cutoff
        }).collect::<Vec<_>>();

        if known.is_empty() {
            return PolicyResolution::NoKnownVersion { family_id: query.family_id.clone() };
        }

        let scoped = known.into_iter()
            .filter(|rule| rule.scope.matches(&query.scope))
            .collect::<Vec<_>>();
        if scoped.is_empty() {
            return PolicyResolution::OutOfScope { family_id: query.family_id.clone() };
        }

        let effective = scoped.into_iter().filter(|rule| {
            rule.effective_from <= query.event_time
                && rule.effective_until.is_none_or(|until| query.event_time < until)
        }).collect::<Vec<_>>();
        if effective.is_empty() {
            return PolicyResolution::NoEffectiveVersion { family_id: query.family_id.clone() };
        }

        // Scope is filtered first; a Neopanamax amendment cannot erase a Panamax
        // candidate. "Latest version wins" is intentionally not implicit.
        let active = effective.iter().copied().filter(|candidate| {
            !effective.iter().any(|other| {
                other.version_id != candidate.version_id
                    && other.supersedes.contains(&candidate.version_id)
            })
        }).collect::<Vec<_>>();

        if active.len() > 1 {
            return PolicyResolution::Ambiguous {
                family_id: query.family_id.clone(),
                version_ids: active.iter().map(|rule| rule.version_id.clone()).collect(),
            };
        }

        let rule = active[0];
        match &rule.evidence_status {
            PolicyEvidenceStatus::Unqualified { reason } => PolicyResolution::UnqualifiedEvidence {
                family_id: rule.family_id.clone(),
                version_id: rule.version_id.clone(),
                reason: reason.clone(),
            },
            PolicyEvidenceStatus::Qualified => {
                // The constructor requires both provenance fields for qualified rules.
                let (Some(source_sha256), Some(source_locator)) =
                    (&rule.source_sha256, &rule.source_locator)
                else {
                    return PolicyResolution::UnqualifiedEvidence {
                        family_id: rule.family_id.clone(),
                        version_id: rule.version_id.clone(),
                        reason: "qualified evidence is missing required provenance fields".into(),
                    };
                };
                PolicyResolution::Resolved {
                    family_id: rule.family_id.clone(),
                    version_id: rule.version_id.clone(),
                    source_sha256: source_sha256.clone(),
                    source_locator: source_locator.clone(),
                }
            }
        }
    }

    pub fn version_count(&self) -> usize {
        self.by_version.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn time(seconds: i64) -> PolicyTime {
        PolicyTime::from_unix_seconds(seconds)
    }

    fn qualified_rule(
        version_id: &str,
        published_at: i64,
        recorded_at: i64,
        effective_from: i64,
        effective_until: Option<i64>,
        scope: PolicyScope,
        supersedes: &[&str],
    ) -> PolicyRuleVersion {
        PolicyRuleVersion {
            family_id: "reservation.cancellation".into(),
            version_id: version_id.into(),
            published_at: time(published_at),
            recorded_at: time(recorded_at),
            effective_from: time(effective_from),
            effective_until: effective_until.map(time),
            scope,
            supersedes: supersedes.iter().map(|value| (*value).into()).collect(),
            evidence_status: PolicyEvidenceStatus::Qualified,
            source_sha256: Some("a".repeat(64)),
            source_locator: Some("advisory.pdf#page=2/table=1".into()),
        }
    }

    fn query(event_time: i64, knowledge_cutoff: i64, scope: PolicyScope) -> PolicyQuery {
        PolicyQuery {
            family_id: "reservation.cancellation".into(),
            event_time: time(event_time),
            knowledge_cutoff: time(knowledge_cutoff),
            scope,
        }
    }

    fn resolved_version(result: PolicyResolution) -> String {
        match result {
            PolicyResolution::Resolved { version_id, .. } => version_id,
            other => panic!("expected a resolved version, got {other:?}"),
        }
    }

    #[test]
    fn future_publication_and_late_ingestion_cannot_leak_into_a_replay() {
        let old = qualified_rule(
            "v1", 10, 11, 12, None,
            PolicyScope::lock_system(PolicyLockSystem::Panamax), &[],
        );
        let new = qualified_rule(
            "v2", 50, 55, 20, None,
            PolicyScope::lock_system(PolicyLockSystem::Panamax), &["v1"],
        );
        let book = PolicyRuleBook::new(vec![new, old]).expect("valid multi-temporal rule book");

        assert_eq!(resolved_version(book.resolve(&query(
            60, 40, PolicyScope::lock_system(PolicyLockSystem::Panamax),
        ))), "v1");
        assert_eq!(resolved_version(book.resolve(&query(
            60, 55, PolicyScope::lock_system(PolicyLockSystem::Panamax),
        ))), "v2");
    }

    #[test]
    fn source_ingested_after_cutoff_does_not_leak_when_already_published() {
        let scope = PolicyScope::lock_system(PolicyLockSystem::Panamax);
        let old = qualified_rule("v1", 10, 11, 12, None, scope.clone(), &[]);
        // Public since time 20, but absent from this replay corpus until time 50.
        let late_ingested = qualified_rule("v2", 20, 50, 20, None, scope.clone(), &["v1"]);
        let book = PolicyRuleBook::new(vec![old, late_ingested]).expect("valid multi-temporal history");

        assert_eq!(resolved_version(book.resolve(&query(60, 40, scope.clone()))), "v1");
        assert_eq!(resolved_version(book.resolve(&query(60, 50, scope))), "v2");
    }

    #[test]
    fn effective_interval_is_half_open_and_not_the_publication_time() {
        let rule = qualified_rule(
            "v1", 10, 10, 20, Some(30),
            PolicyScope::lock_system(PolicyLockSystem::Panamax), &[],
        );
        let book = PolicyRuleBook::new(vec![rule]).expect("valid rule");

        assert!(matches!(book.resolve(&query(
            19, 20, PolicyScope::lock_system(PolicyLockSystem::Panamax),
        )), PolicyResolution::NoEffectiveVersion { .. }));
        assert_eq!(resolved_version(book.resolve(&query(
            20, 20, PolicyScope::lock_system(PolicyLockSystem::Panamax),
        ))), "v1");
        assert!(matches!(book.resolve(&query(
            30, 30, PolicyScope::lock_system(PolicyLockSystem::Panamax),
        )), PolicyResolution::NoEffectiveVersion { .. }));
    }

    #[test]
    fn lock_specificity_prevents_cross_lock_rule_leakage() {
        let rule = qualified_rule(
            "neo-only", 10, 10, 10, None,
            PolicyScope::lock_system(PolicyLockSystem::Neopanamax), &[],
        );
        let book = PolicyRuleBook::new(vec![rule]).expect("valid rule");

        assert!(matches!(book.resolve(&query(
            20, 20, PolicyScope::lock_system(PolicyLockSystem::Panamax),
        )), PolicyResolution::OutOfScope { .. }));
    }

    #[test]
    fn overlapping_versions_without_explicit_supersession_are_ambiguous() {
        let scope = PolicyScope::lock_system(PolicyLockSystem::Panamax);
        let first = qualified_rule("v1", 10, 10, 10, None, scope.clone(), &[]);
        let second = qualified_rule("v2", 11, 11, 10, None, scope.clone(), &[]);
        let book = PolicyRuleBook::new(vec![first, second]).expect("valid but overlapping rules");

        assert_eq!(book.resolve(&query(20, 20, scope)), PolicyResolution::Ambiguous {
            family_id: "reservation.cancellation".into(),
            version_ids: vec!["v1".into(), "v2".into()],
        });
    }

    #[test]
    fn unqualified_source_never_becomes_a_resolved_policy() {
        let mut rule = qualified_rule(
            "transcribed-only", 10, 10, 10, None,
            PolicyScope::lock_system(PolicyLockSystem::Panamax), &[],
        );
        rule.evidence_status = PolicyEvidenceStatus::Unqualified {
            reason: "original bytes not hash-frozen and permitted use unreviewed".into(),
        };
        rule.source_sha256 = None;
        rule.source_locator = Some("https://example.invalid/advisory.pdf".into());
        let book = PolicyRuleBook::new(vec![rule]).expect("unqualified records may be indexed");

        assert!(matches!(book.resolve(&query(
            20, 20, PolicyScope::lock_system(PolicyLockSystem::Panamax),
        )), PolicyResolution::UnqualifiedEvidence { version_id, .. }
            if version_id == "transcribed-only"));
    }

    #[test]
    fn scope_filtering_happens_before_supersession() {
        let old = qualified_rule(
            "panamax-v1", 10, 10, 10, None,
            PolicyScope::lock_system(PolicyLockSystem::Panamax), &[],
        );
        let neo_amendment = qualified_rule(
            "neo-v2", 11, 11, 11, None,
            PolicyScope::lock_system(PolicyLockSystem::Neopanamax), &["panamax-v1"],
        );
        let book = PolicyRuleBook::new(vec![neo_amendment, old]).expect("valid scoped history");

        assert_eq!(resolved_version(book.resolve(&query(
            20, 20, PolicyScope::lock_system(PolicyLockSystem::Panamax),
        ))), "panamax-v1");
        assert_eq!(resolved_version(book.resolve(&query(
            20, 20, PolicyScope::lock_system(PolicyLockSystem::Neopanamax),
        ))), "neo-v2");
    }

    #[test]
    fn qualified_evidence_requires_a_hash_and_exact_locator() {
        let mut rule = qualified_rule("missing-hash", 10, 10, 10, None, PolicyScope::any(), &[]);
        rule.source_sha256 = None;

        assert!(matches!(PolicyRuleBook::new(vec![rule]), Err(PolicyBookError::InvalidRule {
            reason: "qualified evidence requires a 64-character SHA-256 digest",
            ..
        })));
    }

    #[test]
    fn unknown_supersession_fails_closed_and_unknown_family_is_explicit() {
        let rule = qualified_rule("v1", 10, 10, 10, None, PolicyScope::any(), &["missing"]);
        assert!(matches!(
            PolicyRuleBook::new(vec![rule]),
            Err(PolicyBookError::UnknownSupersededVersion { .. })
        ));

        let book = PolicyRuleBook::new(Vec::new()).expect("empty book is valid");
        assert!(matches!(
            book.resolve(&query(10, 10, PolicyScope::any())),
            PolicyResolution::UnknownFamily { .. }
        ));
    }

    #[test]
    fn supersession_cycles_are_rejected() {
        let first = qualified_rule("v1", 10, 10, 10, None, PolicyScope::any(), &["v2"]);
        let second = qualified_rule("v2", 11, 11, 11, None, PolicyScope::any(), &["v1"]);

        assert!(matches!(
            PolicyRuleBook::new(vec![first, second]),
            Err(PolicyBookError::SupersessionCycle(_))
        ));
    }

    #[test]
    fn family_resolution_is_input_order_independent() {
        let older = qualified_rule("v1", 10, 10, 10, None, PolicyScope::any(), &[]);
        let newer = qualified_rule("v2", 20, 20, 20, None, PolicyScope::any(), &["v1"]);
        let query = query(30, 30, PolicyScope::any());

        let forward = PolicyRuleBook::new(vec![older.clone(), newer.clone()])
            .expect("valid ordered book")
            .resolve(&query);
        let reversed = PolicyRuleBook::new(vec![newer, older])
            .expect("valid reversed book")
            .resolve(&query);

        assert_eq!(forward, reversed);
        assert_eq!(resolved_version(forward), "v2");
    }
}
