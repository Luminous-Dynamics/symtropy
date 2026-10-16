// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Deterministic, fail-closed replay for synthetic accessible-home evidence.
//! It validates declared evidence consistency; it cannot establish real-world truth.

use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const VALIDATOR_VERSION: &str = "accessible-home-evidence-lifecycle-validator/0.1.0";
const FIXTURE: &str = include_str!(
    "../../../docs/built-environment/accessible-home-evidence-lifecycle-v0.1.example.json"
);

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Lifecycle {
    pub schema_version: String,
    pub fixture_kind: String,
    pub lifecycle_id: String,
    pub claim_boundary: String,
    pub evaluation_time: String,
    pub facets: Vec<Facet>,
    pub claims: Vec<Claim>,
    pub change_events: Vec<ChangeEvent>,
    pub paths: Vec<AccessiblePath>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Facet {
    pub facet_id: String,
    pub initial_revision_id: String,
    pub scope_ref: String,
    pub description: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    pub facet_id: String,
    pub revision_id: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceState {
    VerifiedForDeclaredScope,
    KnownBarrier,
    Unknown,
    Proposed,
    NeedsSiteMeasurement,
    NeedsProfessionalReview,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Currentness {
    Current,
    NeedsRevalidation,
    Expired,
    Superseded,
    Conflict,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Assessment {
    BlockedByDeclaredBarrier,
    NotEvaluable,
    EvidenceCompleteForFixture,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct RequiredNullableString(pub Option<String>);

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Claim {
    pub claim_id: String,
    pub scope_ref: String,
    pub bindings: Vec<Binding>,
    pub evidence_state: EvidenceState,
    pub source_ref: String,
    pub review_role: String,
    pub observed_at: String,
    pub review_due: RequiredNullableString,
    pub supersedes_claim_id: RequiredNullableString,
    pub expected_currentness: Currentness,
    pub note: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FacetChange {
    pub facet_id: String,
    pub from_revision_id: String,
    pub to_revision_id: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ChangeEvent {
    pub event_id: String,
    pub occurred_at: String,
    pub source_ref: String,
    pub changes: Vec<FacetChange>,
    pub note: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AccessiblePath {
    pub path_id: String,
    pub required_claim_ids: Vec<String>,
    pub expected_assessment: Assessment,
    pub note: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct FacetResult {
    pub facet_id: String,
    pub current_revision_id: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct ClaimResult {
    pub claim_id: String,
    pub evidence_state: EvidenceState,
    pub currentness: Currentness,
    pub invalidated_by_facets: Vec<String>,
    pub reason: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct PathResult {
    pub path_id: String,
    pub assessment: Assessment,
    pub expected_assessment: Assessment,
    pub required_claim_count: usize,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct ValidationReport {
    pub report_schema_version: String,
    pub validator_version: String,
    pub lifecycle_id: String,
    pub fixture_kind: String,
    pub input_sha256: String,
    pub facet_count: usize,
    pub claim_count: usize,
    pub event_count: usize,
    pub path_count: usize,
    pub current_facets: Vec<FacetResult>,
    pub claim_results: Vec<ClaimResult>,
    pub path_results: Vec<PathResult>,
    pub claim_limit: String,
}

fn parse_time(value: &str, field: &str, errors: &mut Vec<String>) -> Option<DateTime<FixedOffset>> {
    match DateTime::parse_from_rfc3339(value) {
        Ok(parsed) => Some(parsed),
        Err(_) => {
            errors.push(format!("{field} must be a valid RFC 3339 date-time"));
            None
        }
    }
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 80
        && (value.as_bytes()[0].is_ascii_lowercase() || value.as_bytes()[0].is_ascii_digit())
        && value.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        && !value.ends_with('-')
        && !value.contains("--")
}

fn is_synthetic_ref(value: &str) -> bool {
    value.strip_prefix("synthetic://").is_some_and(|tail| {
        !tail.is_empty()
            && tail.bytes().all(|b| {
                b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'/' || b == b'_' || b == b'-'
            })
    })
}

fn push_unique_id(
    seen: &mut BTreeSet<String>,
    id: &str,
    kind: &str,
    errors: &mut Vec<String>,
) {
    if !valid_id(id) {
        errors.push(format!("{kind} ID {id:?} does not match the stable synthetic ID pattern"));
    }
    if !seen.insert(id.to_owned()) {
        errors.push(format!("duplicate {kind} ID {id:?}"));
    }
}

fn validate_and_replay(data: &Lifecycle) -> Result<BTreeMap<String, String>, Vec<String>> {
    let mut errors = Vec::new();
    if data.schema_version != "0.1.0" {
        errors.push(format!("unsupported schema_version {:?}", data.schema_version));
    }
    if data.fixture_kind != "synthetic_evidence_lifecycle" {
        errors.push("fixture_kind must be synthetic_evidence_lifecycle; real-home inputs are rejected".into());
    }
    if !data.claim_boundary.to_ascii_lowercase().contains("synthetic") {
        errors.push("claim_boundary must explicitly state the synthetic-only boundary".into());
    }
    if !valid_id(&data.lifecycle_id) {
        errors.push("lifecycle_id does not match the stable synthetic ID pattern".into());
    }
    if data.facets.is_empty() {
        errors.push("at least one facet is required".into());
    }
    if data.claims.is_empty() {
        errors.push("at least one claim is required".into());
    }
    if data.paths.is_empty() {
        errors.push("at least one path is required".into());
    }

    let evaluation_time = parse_time(&data.evaluation_time, "evaluation_time", &mut errors);
    let mut facet_ids = BTreeSet::new();
    let mut revisions_by_facet: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut current_revisions = BTreeMap::new();
    for facet in &data.facets {
        push_unique_id(&mut facet_ids, &facet.facet_id, "facet", &mut errors);
        if !valid_id(&facet.initial_revision_id) {
            errors.push(format!("facet {:?} initial revision ID is invalid", facet.facet_id));
        }
        if !is_synthetic_ref(&facet.scope_ref) {
            errors.push(format!("facet {:?} scope_ref must be synthetic://", facet.facet_id));
        }
        if facet.description.trim().is_empty() {
            errors.push(format!("facet {:?} description must not be empty", facet.facet_id));
        }
        current_revisions.insert(facet.facet_id.clone(), facet.initial_revision_id.clone());
        revisions_by_facet
            .entry(facet.facet_id.clone())
            .or_default()
            .insert(facet.initial_revision_id.clone());
    }

    let mut claim_ids = BTreeSet::new();
    let claims_by_id: BTreeMap<&str, &Claim> = data
        .claims
        .iter()
        .map(|claim| (claim.claim_id.as_str(), claim))
        .collect();
    for claim in &data.claims {
        push_unique_id(&mut claim_ids, &claim.claim_id, "claim", &mut errors);
        if !is_synthetic_ref(&claim.scope_ref) || !is_synthetic_ref(&claim.source_ref) {
            errors.push(format!("claim {:?} must use synthetic:// scope and source references", claim.claim_id));
        }
        if claim.bindings.is_empty() {
            errors.push(format!("claim {:?} must bind at least one facet revision", claim.claim_id));
        }
        if claim.review_role.trim().is_empty() || claim.note.trim().is_empty() {
            errors.push(format!("claim {:?} review_role and note must not be empty", claim.claim_id));
        }
        let mut bound = BTreeSet::new();
        for binding in &claim.bindings {
            if !bound.insert(binding.facet_id.as_str()) {
                errors.push(format!(
                    "claim {:?} repeats facet binding {:?}",
                    claim.claim_id, binding.facet_id
                ));
            }
            if !valid_id(&binding.revision_id) {
                errors.push(format!("claim {:?} has invalid revision ID {:?}", claim.claim_id, binding.revision_id));
            }
            if !facet_ids.contains(&binding.facet_id) {
                errors.push(format!("claim {:?} references unknown facet {:?}", claim.claim_id, binding.facet_id));
            }
        }
        if let Some(observed) = parse_time(&claim.observed_at, &format!("claim {} observed_at", claim.claim_id), &mut errors) {
            if evaluation_time.as_ref().is_some_and(|end| observed > (*end).clone()) {
                errors.push(format!("claim {:?} is observed after evaluation_time", claim.claim_id));
            }
        }
        if let Some(due) = claim.review_due.0.as_deref() {
            parse_time(due, &format!("claim {} review_due", claim.claim_id), &mut errors);
        }
        if let Some(old_id) = claim.supersedes_claim_id.0.as_deref() {
            if old_id == claim.claim_id {
                errors.push(format!("claim {:?} cannot supersede itself", claim.claim_id));
            }
            if !claims_by_id.contains_key(old_id) {
                errors.push(format!("claim {:?} supersedes unknown claim {:?}", claim.claim_id, old_id));
            } else if let Some(old) = claims_by_id.get(old_id) {
                if old.scope_ref != claim.scope_ref {
                    errors.push(format!(
                        "supersession {:?} -> {:?} must retain the same scope_ref",
                        claim.claim_id, old_id
                    ));
                }
                let newer = parse_time(&claim.observed_at, "superseding claim observed_at", &mut errors);
                let older = parse_time(&old.observed_at, "superseded claim observed_at", &mut errors);
                if newer.zip(older).is_some_and(|(newer, older)| newer < older) {
                    errors.push(format!("claim {:?} cannot supersede a later observation", claim.claim_id));
                }
            }
        }
    }

    // A supersession chain is a directed graph. A cycle has no defensible current tip.
    let parent_by_id: BTreeMap<&str, &str> = data
        .claims
        .iter()
        .filter_map(|claim| claim.supersedes_claim_id.0.as_deref().map(|old| (claim.claim_id.as_str(), old)))
        .collect();
    for start in parent_by_id.keys() {
        let mut visited = BTreeSet::new();
        let mut cursor = Some(*start);
        while let Some(id) = cursor {
            if !visited.insert(id) {
                errors.push(format!("supersession cycle includes claim {id:?}"));
                break;
            }
            cursor = parent_by_id.get(id).copied();
        }
    }

    let mut event_ids = BTreeSet::new();
    let mut last_event_time: Option<DateTime<FixedOffset>> = None;
    for event in &data.change_events {
        push_unique_id(&mut event_ids, &event.event_id, "event", &mut errors);
        if !is_synthetic_ref(&event.source_ref) {
            errors.push(format!("event {:?} source_ref must be synthetic://", event.event_id));
        }
        if event.changes.is_empty() {
            errors.push(format!("event {:?} must change at least one facet", event.event_id));
        }
        let event_time = parse_time(&event.occurred_at, &format!("event {} occurred_at", event.event_id), &mut errors);
        if let Some(time) = event_time {
            if evaluation_time.as_ref().is_some_and(|end| time > (*end).clone()) {
                errors.push(format!("event {:?} occurs after evaluation_time", event.event_id));
            }
            if last_event_time.is_some_and(|previous| time < previous) {
                errors.push(format!("event {:?} violates declared causal event order", event.event_id));
            }
            last_event_time = Some(time);
        }
        let mut changed_facets = BTreeSet::new();
        for change in &event.changes {
            if !changed_facets.insert(change.facet_id.as_str()) {
                errors.push(format!("event {:?} changes facet {:?} more than once", event.event_id, change.facet_id));
            }
            if !facet_ids.contains(&change.facet_id) {
                errors.push(format!("event {:?} references unknown facet {:?}", event.event_id, change.facet_id));
                continue;
            }
            if !valid_id(&change.from_revision_id) || !valid_id(&change.to_revision_id) {
                errors.push(format!("event {:?} contains an invalid revision ID", event.event_id));
            }
            if change.from_revision_id == change.to_revision_id {
                errors.push(format!("event {:?} must advance to a different revision", event.event_id));
            }
            revisions_by_facet
                .entry(change.facet_id.clone())
                .or_default()
                .insert(change.from_revision_id.clone());
            revisions_by_facet
                .entry(change.facet_id.clone())
                .or_default()
                .insert(change.to_revision_id.clone());
            match current_revisions.get(&change.facet_id).cloned() {
                Some(current) if current == change.from_revision_id => {
                    current_revisions.insert(change.facet_id.clone(), change.to_revision_id.clone());
                }
                Some(current) => errors.push(format!(
                    "event {:?} from_revision_id {:?} does not match current revision {:?} for facet {:?}",
                    event.event_id, change.from_revision_id, current, change.facet_id
                )),
                None => {}
            }
        }
    }

    // A claim may legitimately refer to a previous revision, but not to a revision
    // that never occurs in the declared history for that facet.
    for claim in &data.claims {
        for binding in &claim.bindings {
            if revisions_by_facet
                .get(&binding.facet_id)
                .is_some_and(|known| !known.contains(&binding.revision_id))
            {
                errors.push(format!(
                    "claim {:?} binds unknown revision {:?} of facet {:?}",
                    claim.claim_id, binding.revision_id, binding.facet_id
                ));
            }
        }
    }

    let mut path_ids = BTreeSet::new();
    for path in &data.paths {
        push_unique_id(&mut path_ids, &path.path_id, "path", &mut errors);
        if path.required_claim_ids.is_empty() {
            errors.push(format!("path {:?} must require at least one claim", path.path_id));
        }
        let mut required = BTreeSet::new();
        for claim_id in &path.required_claim_ids {
            if !required.insert(claim_id.as_str()) {
                errors.push(format!("path {:?} repeats claim reference {:?}", path.path_id, claim_id));
            }
            if !claim_ids.contains(claim_id) {
                errors.push(format!("path {:?} references unknown claim {:?}", path.path_id, claim_id));
            }
        }
        if path.note.trim().is_empty() {
            errors.push(format!("path {:?} note must not be empty", path.path_id));
        }
    }

    if !errors.is_empty() {
        errors.sort();
        errors.dedup();
        return Err(errors);
    }
    Ok(current_revisions)
}

fn derive_claim_results(
    data: &Lifecycle,
    current_revisions: &BTreeMap<String, String>,
) -> Vec<ClaimResult> {
    let superseded: BTreeSet<&str> = data
        .claims
        .iter()
        .filter_map(|claim| claim.supersedes_claim_id.0.as_deref())
        .collect();
    let evaluation_time = DateTime::parse_from_rfc3339(&data.evaluation_time)
        .expect("evaluation_time has already been validated");
    let mut results: Vec<ClaimResult> = data.claims.iter().map(|claim| {
        let mut invalidated_by_facets = Vec::new();
        for binding in &claim.bindings {
            if current_revisions.get(&binding.facet_id) != Some(&binding.revision_id) {
                invalidated_by_facets.push(binding.facet_id.clone());
            }
        }
        invalidated_by_facets.sort();
        invalidated_by_facets.dedup();

        let (currentness, reason) = if superseded.contains(claim.claim_id.as_str()) {
            (Currentness::Superseded, "A later declared claim supersedes this claim.".to_owned())
        } else if !invalidated_by_facets.is_empty() {
            (Currentness::NeedsRevalidation, format!(
                "Bound revision changed for facet(s): {}.",
                invalidated_by_facets.join(", ")
            ))
        } else if claim.review_due.0.as_deref().and_then(|due| DateTime::parse_from_rfc3339(due).ok()).is_some_and(|due| due <= evaluation_time) {
            (Currentness::Expired, "The declared review deadline is at or before evaluation_time.".to_owned())
        } else {
            (Currentness::Current, "All bound revisions match; the review window has not lapsed.".to_owned())
        };
        ClaimResult {
            claim_id: claim.claim_id.clone(),
            evidence_state: claim.evidence_state,
            currentness,
            invalidated_by_facets,
            reason,
        }
    }).collect();

    // Same-scope current claims with incompatible declared states are unresolved.
    // Timestamps are not used to choose which conflicting claim is "true".
    let mut claims_per_scope: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
    for (index, claim) in data.claims.iter().enumerate() {
        if results[index].currentness == Currentness::Current {
            claims_per_scope.entry(claim.scope_ref.as_str()).or_default().push(index);
        }
    }
    for indices in claims_per_scope.values() {
        let states: BTreeSet<u8> = indices.iter().map(|index| match results[*index].evidence_state {
            EvidenceState::VerifiedForDeclaredScope => 0,
            EvidenceState::KnownBarrier => 1,
            EvidenceState::Unknown => 2,
            EvidenceState::Proposed => 3,
            EvidenceState::NeedsSiteMeasurement => 4,
            EvidenceState::NeedsProfessionalReview => 5,
        }).collect();
        if states.len() > 1 {
            for index in indices {
                results[*index].currentness = Currentness::Conflict;
                results[*index].reason = "Multiple current claims for this scope declare incompatible evidence states; resolve explicitly.".into();
            }
        }
    }
    results
}

fn assess_path(path: &AccessiblePath, claims: &BTreeMap<&str, &ClaimResult>) -> Assessment {
    let required: Vec<&ClaimResult> = path
        .required_claim_ids
        .iter()
        .filter_map(|claim_id| claims.get(claim_id.as_str()).copied())
        .collect();
    // A current explicit barrier is actionable even if another claim is stale.
    if required.iter().any(|claim| {
        claim.currentness == Currentness::Current && claim.evidence_state == EvidenceState::KnownBarrier
    }) {
        return Assessment::BlockedByDeclaredBarrier;
    }
    if required.iter().any(|claim| {
        claim.currentness != Currentness::Current
            || claim.evidence_state != EvidenceState::VerifiedForDeclaredScope
    }) {
        return Assessment::NotEvaluable;
    }
    Assessment::EvidenceCompleteForFixture
}

/// Parse, validate, replay and emit a deterministic receipt only if fixture expectations agree.
pub fn evaluate_bytes(input: &[u8]) -> Result<ValidationReport, Vec<String>> {
    let data: Lifecycle = serde_json::from_slice(input)
        .map_err(|error| vec![format!("invalid lifecycle JSON/shape: {error}")])?;
    let current_revisions = validate_and_replay(&data)?;
    let claim_results = derive_claim_results(&data, &current_revisions);
    let claim_result_by_id: BTreeMap<&str, &ClaimResult> = claim_results
        .iter()
        .map(|result| (result.claim_id.as_str(), result))
        .collect();
    let mut errors = Vec::new();

    for claim in &data.claims {
        let actual = claim_result_by_id[claim.claim_id.as_str()].currentness;
        if actual != claim.expected_currentness {
            errors.push(format!(
                "claim {:?} expected currentness {:?}, derived {:?}",
                claim.claim_id, claim.expected_currentness, actual
            ));
        }
    }
    let path_results: Vec<PathResult> = data.paths.iter().map(|path| {
        let actual = assess_path(path, &claim_result_by_id);
        if actual != path.expected_assessment {
            errors.push(format!(
                "path {:?} expected assessment {:?}, derived {:?}",
                path.path_id, path.expected_assessment, actual
            ));
        }
        PathResult {
            path_id: path.path_id.clone(),
            assessment: actual,
            expected_assessment: path.expected_assessment,
            required_claim_count: path.required_claim_ids.len(),
        }
    }).collect();
    if !errors.is_empty() {
        errors.sort();
        return Err(errors);
    }

    let current_facets: Vec<FacetResult> = current_revisions
        .iter()
        .map(|(facet_id, revision_id)| FacetResult {
            facet_id: facet_id.clone(),
            current_revision_id: revision_id.clone(),
        })
        .collect();
    let digest = Sha256::digest(input);
    let input_sha256 = digest.iter().map(|byte| format!("{byte:02x}")).collect();
    Ok(ValidationReport {
        report_schema_version: "0.1.0".into(),
        validator_version: VALIDATOR_VERSION.into(),
        lifecycle_id: data.lifecycle_id,
        fixture_kind: data.fixture_kind,
        input_sha256,
        facet_count: data.facets.len(),
        claim_count: data.claims.len(),
        event_count: data.change_events.len(),
        path_count: data.paths.len(),
        current_facets,
        claim_results,
        path_results,
        claim_limit: "Consistency of synthetic claims only. This report does not prove physical accessibility, safety, usability, legal compliance, or suitability for a real resident.".into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    fn fixture_value() -> Value {
        serde_json::from_str(FIXTURE).expect("committed fixture parses")
    }

    fn evaluate_value(value: &Value) -> Result<ValidationReport, Vec<String>> {
        evaluate_bytes(&serde_json::to_vec(value).expect("fixture serializes"))
    }

    fn has_error(errors: &[String], needle: &str) -> bool {
        errors.iter().any(|error| error.contains(needle))
    }

    #[test]
    fn committed_fixture_replays_with_declared_expectations() {
        let report = evaluate_bytes(FIXTURE.as_bytes()).expect("synthetic baseline should validate");
        assert_eq!(report.facet_count, 4);
        assert_eq!(report.claim_count, 3);
        assert_eq!(report.event_count, 1);
        assert_eq!(report.path_count, 3);
        assert_eq!(report.path_results[0].assessment, Assessment::NotEvaluable);
        assert_eq!(report.path_results[1].assessment, Assessment::EvidenceCompleteForFixture);
        assert_eq!(report.path_results[2].assessment, Assessment::NotEvaluable);
    }

    #[test]
    fn unrelated_facet_change_does_not_invalidate_notification_claim() {
        let report = evaluate_bytes(FIXTURE.as_bytes()).expect("fixture should validate");
        let claim = report.claim_results.iter().find(|claim| claim.claim_id == "visitor-alert-path").unwrap();
        assert_eq!(claim.currentness, Currentness::Current);
        assert!(claim.invalidated_by_facets.is_empty());
    }

    #[test]
    fn wrong_from_revision_is_rejected() {
        let mut value = fixture_value();
        value["change_events"][0]["changes"][0]["from_revision_id"] = json!("furniture-r0");
        let errors = evaluate_value(&value).expect_err("wrong causal start must fail");
        assert!(has_error(&errors, "does not match current revision"));
    }

    #[test]
    fn event_cannot_occur_after_evaluation_time() {
        let mut value = fixture_value();
        value["change_events"][0]["occurred_at"] = json!("2030-01-01T00:00:00Z");
        let errors = evaluate_value(&value).expect_err("future event must fail");
        assert!(has_error(&errors, "occurs after evaluation_time"));
    }

    #[test]
    fn expired_claim_cannot_pass() {
        let report = evaluate_bytes(FIXTURE.as_bytes()).expect("expired expectation is declared");
        let claim = report.claim_results.iter().find(|claim| claim.claim_id == "kitchen-control-review").unwrap();
        assert_eq!(claim.currentness, Currentness::Expired);
        assert_eq!(report.path_results[2].assessment, Assessment::NotEvaluable);
    }

    #[test]
    fn current_hard_barrier_dominates_an_independent_stale_claim() {
        let mut value = fixture_value();
        value["claims"][1]["evidence_state"] = json!("known_barrier");
        value["paths"][1]["expected_assessment"] = json!("blocked_by_declared_barrier");
        let report = evaluate_value(&value).expect("current explicit barrier blocks");
        assert_eq!(report.path_results[1].assessment, Assessment::BlockedByDeclaredBarrier);
    }

    #[test]
    fn stale_old_barrier_does_not_claim_to_block_the_new_configuration() {
        let mut value = fixture_value();
        value["claims"][0]["evidence_state"] = json!("known_barrier");
        let report = evaluate_value(&value).expect("old barrier is stale and path stays not evaluable");
        assert_eq!(report.claim_results[0].currentness, Currentness::NeedsRevalidation);
        assert_eq!(report.path_results[0].assessment, Assessment::NotEvaluable);
    }

    #[test]
    fn conflicting_current_claims_are_not_resolved_by_timestamp() {
        let mut value = fixture_value();
        let mut alternate = value["claims"][1].clone();
        alternate["claim_id"] = json!("visitor-alert-contradiction");
        alternate["evidence_state"] = json!("known_barrier");
        alternate["expected_currentness"] = json!("conflict");
        value["claims"][1]["expected_currentness"] = json!("conflict");
        value["claims"].as_array_mut().unwrap().push(alternate);
        value["paths"][1]["expected_assessment"] = json!("not_evaluable");
        let report = evaluate_value(&value).expect("conflict remains explicit and non-passing");
        assert_eq!(report.claim_results[1].currentness, Currentness::Conflict);
        assert_eq!(report.claim_results[3].currentness, Currentness::Conflict);
        assert_eq!(report.path_results[1].assessment, Assessment::NotEvaluable);
    }

    #[test]
    fn supersession_cycle_fails_closed() {
        let mut value = fixture_value();
        let shared_scope = value["claims"][1]["scope_ref"].clone();
        value["claims"][0]["scope_ref"] = shared_scope;
        value["claims"][0]["supersedes_claim_id"] = json!("visitor-alert-path");
        value["claims"][1]["supersedes_claim_id"] = json!("night-route-clearance");
        let errors = evaluate_value(&value).expect_err("cycle must fail");
        assert!(has_error(&errors, "supersession cycle"));
    }

    #[test]
    fn duplicate_event_ids_are_rejected() {
        let mut value = fixture_value();
        let duplicate = value["change_events"][0].clone();
        value["change_events"].as_array_mut().unwrap().push(duplicate);
        let errors = evaluate_value(&value).expect_err("duplicate event must fail");
        assert!(has_error(&errors, "duplicate event ID"));
    }

    #[test]
    fn unknown_facet_reference_is_rejected() {
        let mut value = fixture_value();
        value["claims"][0]["bindings"][0]["facet_id"] = json!("unlisted-facet");
        let errors = evaluate_value(&value).expect_err("unknown facet must fail");
        assert!(has_error(&errors, "references unknown facet"));
    }

    #[test]
    fn unknown_path_claim_reference_is_rejected() {
        let mut value = fixture_value();
        value["paths"][0]["required_claim_ids"][0] = json!("missing-claim");
        let errors = evaluate_value(&value).expect_err("dangling claim reference must fail");
        assert!(has_error(&errors, "references unknown claim"));
    }

    #[test]
    fn real_home_fixture_kind_is_rejected() {
        let mut value = fixture_value();
        value["fixture_kind"] = json!("real_home_assessment");
        let errors = evaluate_value(&value).expect_err("real-home data is outside this validator contract");
        assert!(has_error(&errors, "real-home inputs are rejected"));
    }

    #[test]
    fn repeated_replay_is_byte_deterministic() {
        let first = evaluate_bytes(FIXTURE.as_bytes()).expect("first replay");
        let second = evaluate_bytes(FIXTURE.as_bytes()).expect("second replay");
        assert_eq!(first, second);
        assert_eq!(first.input_sha256, second.input_sha256);
    }

    #[test]
    fn one_thousand_independent_claims_and_paths_replay() {
        let mut value = fixture_value();
        for index in 0..1_000 {
            let suffix = format!("{index:04}");
            let facet_id = format!("scale-facet-{suffix}");
            let revision_id = format!("scale-revision-{suffix}");
            let claim_id = format!("scale-claim-{suffix}");
            let scope_ref = format!("synthetic://scale/facet-{suffix}");
            value["facets"].as_array_mut().unwrap().push(json!({
                "facet_id": facet_id,
                "initial_revision_id": revision_id,
                "scope_ref": scope_ref,
                "description": "Synthetic scale-test facet; no physical home represented."
            }));
            value["claims"].as_array_mut().unwrap().push(json!({
                "claim_id": claim_id,
                "scope_ref": scope_ref,
                "bindings": [{
                    "facet_id": facet_id,
                    "revision_id": revision_id
                }],
                "evidence_state": "verified_for_declared_scope",
                "source_ref": format!("synthetic://scale/evidence/{suffix}"),
                "review_role": "synthetic scale-test reviewer",
                "observed_at": "2026-09-01T00:00:00Z",
                "review_due": "2027-09-01T00:00:00Z",
                "supersedes_claim_id": null,
                "expected_currentness": "current",
                "note": "Generated synthetic test claim."
            }));
            value["paths"].as_array_mut().unwrap().push(json!({
                "path_id": format!("scale-path-{suffix}"),
                "required_claim_ids": [claim_id],
                "expected_assessment": "evidence_complete_for_fixture",
                "note": "Generated synthetic test path."
            }));
        }
        let report = evaluate_value(&value).expect("1,000 independent synthetic claims should replay");
        assert_eq!(report.facet_count, 1_004);
        assert_eq!(report.claim_count, 1_003);
        assert_eq!(report.path_count, 1_003);
    }

    #[test]
    fn receipt_hash_binds_exact_input_bytes() {
        let first = evaluate_bytes(FIXTURE.as_bytes()).expect("first replay");
        let mut value = fixture_value();
        value["claim_boundary"] = json!("Synthetic fixture with an explicitly altered boundary note, still not a real assessment.");
        let second = evaluate_value(&value).expect("altered synthetic fixture remains internally valid");
        assert_ne!(first.input_sha256, second.input_sha256);
    }

    #[test]
    fn missing_nullable_contract_field_is_rejected() {
        let mut value = fixture_value();
        value["claims"][0].as_object_mut().unwrap().remove("review_due");
        let errors = evaluate_value(&value).expect_err("required nullable fields must still be present");
        assert!(has_error(&errors, "review_due"));
    }

    #[test]
    fn unknown_revision_for_known_facet_is_rejected() {
        let mut value = fixture_value();
        value["claims"][0]["bindings"][0]["revision_id"] = json!("furniture-never-existed");
        let errors = evaluate_value(&value).expect_err("unknown revision must fail");
        assert!(has_error(&errors, "binds unknown revision"));
    }
}
