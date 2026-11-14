// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub const REPORT_SCHEMA: &str = "luminous.habitat_evidence_invalidation_report.v0.1";
pub const CLAIM_CEILING: &str = "synthetic_evidence_invalidation_not_real_world_authority";
const FIXTURE_SHA256: &str = "f9f7c16c3b370170a2f2be175f74564b36062cbbe2b7c0caf96ef68323af5b1b";
const ALLOWED_STATUSES: [&str; 7] = [
    "valid_for_claim", "unknown", "stale", "revoked", "incomplete_coverage",
    "contradictory", "out_of_scope",
];

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Fixture {
    pub schema_version: String,
    pub fixture_id: String,
    pub status: String,
    pub synthetic_only: bool,
    pub claim_ceiling: String,
    pub intended_use: String,
    pub evidence_statuses: Vec<String>,
    pub artifacts: Vec<EvidenceArtifact>,
    pub events: Vec<ChangeEvent>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct EvidenceArtifact {
    pub id: String,
    pub kind: String,
    pub version: u64,
    pub claim_id: String,
    pub scope_id: String,
    pub status: String,
    pub coverage_complete: bool,
    pub content_sha256: String,
    pub synthetic_payload: String,
    pub validity_rule_id: String,
    pub depends_on: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
pub struct ChangeEvent {
    pub id: String,
    pub trigger_artifact_id: String,
    pub new_version: u64,
    pub reason: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct EventImpact {
    pub event_id: String,
    pub trigger_artifact_id: String,
    pub superseded_artifact_id: String,
    pub invalidated_descendant_ids: Vec<String>,
    pub affected_artifact_count: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BenchmarkReport {
    pub schema_version: String,
    pub fixture_id: String,
    pub input_sha256: String,
    pub verification_status: String,
    pub claim_ceiling: String,
    pub artifact_count: usize,
    pub input_event_count: usize,
    pub unique_event_count: usize,
    pub duplicate_events_ignored: usize,
    pub event_impacts: Vec<EventImpact>,
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

pub fn artifact_supports_claim(
    artifact: &EvidenceArtifact,
    claim_id: &str,
    scope_id: &str,
) -> bool {
    artifact.status == "valid_for_claim"
        && artifact.coverage_complete
        && artifact.claim_id == claim_id
        && artifact.scope_id == scope_id
        && valid_sha256(&artifact.content_sha256)
        && sha256_hex(artifact.synthetic_payload.as_bytes()) == artifact.content_sha256
}

fn artifact_map(fixture: &Fixture) -> BTreeMap<&str, &EvidenceArtifact> {
    fixture.artifacts.iter().map(|item| (item.id.as_str(), item)).collect()
}

pub fn verify_fixture_bytes(bytes: &[u8]) -> Result<BenchmarkReport, Vec<String>> {
    let digest = sha256_hex(bytes);
    if digest != FIXTURE_SHA256 {
        return Err(vec![format!(
            "frozen evidence fixture digest mismatch: expected {FIXTURE_SHA256}, got {digest}"
        )]);
    }
    let fixture: Fixture = serde_json::from_slice(bytes)
        .map_err(|error| vec![format!("evidence fixture rejected: {error}")])?;
    evaluate_fixture(&fixture, digest)
}

fn evaluate_fixture(
    fixture: &Fixture,
    input_sha256: String,
) -> Result<BenchmarkReport, Vec<String>> {
    let errors = validate_fixture(fixture);
    if !errors.is_empty() {
        return Err(errors);
    }

    // Identical duplicate deliveries are idempotent. Conflicting reuse of an ID fails closed.
    let mut canonical_events: BTreeMap<String, ChangeEvent> = BTreeMap::new();
    for event in &fixture.events {
        if let Some(previous) = canonical_events.get(&event.id) {
            if previous != event {
                return Err(vec![format!(
                    "event ID {:?} was reused with conflicting contents", event.id
                )]);
            }
        } else {
            canonical_events.insert(event.id.clone(), event.clone());
        }
    }

    // Each change event is an independent counterfactual impact test. Do not
    // merge their results into one pretend chronology of mutually exclusive events.
    let mut event_impacts = Vec::new();
    for event in canonical_events.values() {
        let root = event.trigger_artifact_id.as_str();
        let affected = invalidation_closure(fixture, root);
        let descendants = affected
            .iter()
            .filter(|id| id.as_str() != root)
            .cloned()
            .collect::<Vec<_>>();
        event_impacts.push(EventImpact {
            event_id: event.id.clone(),
            trigger_artifact_id: root.to_owned(),
            superseded_artifact_id: root.to_owned(),
            invalidated_descendant_ids: descendants,
            affected_artifact_count: affected.len(),
        });
    }

    Ok(BenchmarkReport {
        schema_version: REPORT_SCHEMA.to_owned(),
        fixture_id: fixture.fixture_id.clone(),
        input_sha256,
        verification_status: "SYNTHETIC_GRAPH_AND_EVIDENCE_CONTRACT_ONLY".to_owned(),
        claim_ceiling: CLAIM_CEILING.to_owned(),
        artifact_count: fixture.artifacts.len(),
        input_event_count: fixture.events.len(),
        unique_event_count: canonical_events.len(),
        duplicate_events_ignored: fixture.events.len() - canonical_events.len(),
        event_impacts,
    })
}

/// Forward transitive closure using the declared dependency edges.
fn invalidation_closure(fixture: &Fixture, trigger_id: &str) -> BTreeSet<String> {
    let mut affected = BTreeSet::from([trigger_id.to_owned()]);
    loop {
        let mut changed = false;
        for artifact in &fixture.artifacts {
            if !affected.contains(&artifact.id)
                && artifact.depends_on.iter().any(|dependency| affected.contains(dependency))
            {
                affected.insert(artifact.id.clone());
                changed = true;
            }
        }
        if !changed {
            return affected;
        }
    }
}

fn validate_fixture(fixture: &Fixture) -> Vec<String> {
    let mut errors = Vec::new();
    if fixture.schema_version != "luminous.habitat_evidence_invalidation_fixture.v0.1" {
        errors.push(format!("unsupported schema_version {:?}", fixture.schema_version));
    }
    if fixture.fixture_id.trim().is_empty()
        || fixture.intended_use.trim().is_empty()
    {
        errors.push("fixture ID and intended use must not be blank".to_owned());
    }
    if fixture.status != "proposed_synthetic_evidence_invalidation_benchmark" {
        errors.push(format!("unsupported status {:?}", fixture.status));
    }
    if !fixture.synthetic_only {
        errors.push("synthetic_only must remain true".to_owned());
    }
    if fixture.claim_ceiling != CLAIM_CEILING {
        errors.push(format!("claim ceiling must be {CLAIM_CEILING:?}"));
    }

    let declared: BTreeSet<&str> =
        fixture.evidence_statuses.iter().map(String::as_str).collect();
    let allowed: BTreeSet<&str> = ALLOWED_STATUSES.into_iter().collect();
    if declared != allowed {
        errors.push("evidence_statuses must match the v0.1 status vocabulary".to_owned());
    }

    let mut artifacts = BTreeMap::new();
    for artifact in &fixture.artifacts {
        if artifact.id.trim().is_empty() {
            errors.push("artifact ID must not be blank".to_owned());
        }
        if artifacts.insert(artifact.id.as_str(), artifact).is_some() {
            errors.push(format!("duplicate artifact ID {:?}", artifact.id));
        }
        if artifact.kind.trim().is_empty() || artifact.claim_id.trim().is_empty()
            || artifact.scope_id.trim().is_empty() || artifact.validity_rule_id.trim().is_empty()
        {
            errors.push(format!(
                "{} must declare kind, claim, scope and validity rule",
                artifact.id
            ));
        }
        if artifact.version == 0 {
            errors.push(format!("{} version must be positive", artifact.id));
        }
        if !allowed.contains(artifact.status.as_str()) {
            errors.push(format!(
                "{} has unsupported evidence status {:?}",
                artifact.id, artifact.status
            ));
        }
        if !valid_sha256(&artifact.content_sha256)
            || sha256_hex(artifact.synthetic_payload.as_bytes()) != artifact.content_sha256
        {
            errors.push(format!("{} synthetic payload digest mismatch", artifact.id));
        }
        if artifact.status == "valid_for_claim" && !artifact.coverage_complete {
            errors.push(format!(
                "{} cannot be valid_for_claim with incomplete coverage",
                artifact.id
            ));
        }
        let mut dependencies = BTreeSet::new();
        for dependency in &artifact.depends_on {
            if dependency == &artifact.id {
                errors.push(format!("{} cannot depend on itself", artifact.id));
            }
            if !dependencies.insert(dependency.as_str()) {
                errors.push(format!("{} repeats dependency {:?}", artifact.id, dependency));
            }
        }
    }
    for artifact in &fixture.artifacts {
        for dependency in &artifact.depends_on {
            if !artifacts.contains_key(dependency.as_str()) {
                errors.push(format!(
                    "{} references unknown dependency {:?}",
                    artifact.id, dependency
                ));
            }
        }
    }

    // Kahn's topological pass rejects dependency cycles before propagation.
    let mut indegree: BTreeMap<&str, usize> = fixture.artifacts.iter()
        .map(|item| (item.id.as_str(), item.depends_on.len())).collect();
    let mut children: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for artifact in &fixture.artifacts {
        for dependency in &artifact.depends_on {
            if artifacts.contains_key(dependency.as_str()) {
                children.entry(dependency.as_str()).or_default().push(artifact.id.as_str());
            }
        }
    }
    let mut queue: VecDeque<&str> = indegree.iter()
        .filter_map(|(id, degree)| (*degree == 0).then_some(*id)).collect();
    let mut visited = 0usize;
    while let Some(id) = queue.pop_front() {
        visited += 1;
        if let Some(dependents) = children.get(id) {
            for dependent in dependents {
                if let Some(degree) = indegree.get_mut(dependent) {
                    *degree = degree.saturating_sub(1);
                    if *degree == 0 { queue.push_back(dependent); }
                }
            }
        }
    }
    if visited != fixture.artifacts.len() {
        errors.push("artifact dependency graph contains a cycle".to_owned());
    }

    let mut event_by_id: BTreeMap<&str, &ChangeEvent> = BTreeMap::new();
    for event in &fixture.events {
        if event.id.trim().is_empty() || event.reason.trim().is_empty() {
            errors.push("event ID and reason must not be blank".to_owned());
        }
        if let Some(previous) = event_by_id.get(event.id.as_str()) {
            if **previous != *event {
                errors.push(format!("event ID {:?} has conflicting contents", event.id));
            }
        } else {
            event_by_id.insert(event.id.as_str(), event);
        }
        match artifacts.get(event.trigger_artifact_id.as_str()) {
            Some(artifact) if event.new_version <= artifact.version => errors.push(format!(
                "{} new_version must exceed current artifact version", event.id
            )),
            None => errors.push(format!(
                "{} references unknown trigger artifact {:?}", event.id, event.trigger_artifact_id
            )),
            _ => {}
        }
    }
    errors
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE_BYTES: &[u8] = include_bytes!(
        "../../../docs/research/fixtures/HABITAT_EVIDENCE_INVALIDATION_V0_1.json"
    );

    fn read_fixture() -> Fixture {
        serde_json::from_slice(FIXTURE_BYTES).expect("frozen fixture should parse")
    }
    fn report() -> BenchmarkReport {
        verify_fixture_bytes(FIXTURE_BYTES).expect("frozen fixture should verify")
    }
    fn impact<'a>(report: &'a BenchmarkReport, event_id: &str) -> &'a EventImpact {
        report.event_impacts.iter().find(|item| item.event_id == event_id).unwrap()
    }

    fn reference_oracle(fixture: &Fixture, trigger: &str) -> BTreeSet<String> {
        fn depends_on_target(
            candidate: &EvidenceArtifact,
            target: &str,
            by_id: &BTreeMap<&str, &EvidenceArtifact>,
            visited: &mut BTreeSet<String>,
        ) -> bool {
            if candidate.id == target { return true; }
            if !visited.insert(candidate.id.clone()) { return false; }
            candidate.depends_on.iter().any(|dependency| {
                dependency == target || by_id.get(dependency.as_str()).is_some_and(|parent| {
                    depends_on_target(parent, target, by_id, visited)
                })
            })
        }
        let by_id = artifact_map(fixture);
        fixture.artifacts.iter()
            .filter(|candidate| depends_on_target(candidate, trigger, &by_id, &mut BTreeSet::new()))
            .map(|candidate| candidate.id.clone()).collect()
    }

    #[test]
    fn frozen_fixture_census_and_affected_counts_are_exact() {
        let report = report();
        assert_eq!(report.input_sha256, FIXTURE_SHA256);
        assert_eq!(report.artifact_count, 10);
        assert_eq!(report.input_event_count, 4);
        assert_eq!(report.unique_event_count, 4);
        assert_eq!(report.duplicate_events_ignored, 0);
        assert_eq!(impact(&report, "EV-01-TERRAIN-REVISION").affected_artifact_count, 10);
        assert_eq!(impact(&report, "EV-02-SURVEY-STALE").affected_artifact_count, 9);
        assert_eq!(
            impact(&report, "EV-03-MATERIAL-SUBSTITUTION").affected_artifact_count,
            3
        );
        assert_eq!(impact(&report, "EV-04-FURNISHING-SUBSTITUTION").affected_artifact_count, 2);
    }

    #[test]
    fn generated_dag_sweep_matches_oracle_for_every_trigger_node() {
        let base = read_fixture();
        for graph_index in 0..32usize {
            let node_count = 4 + graph_index % 21;
            let mut fixture = base.clone();
            fixture.artifacts = (0..node_count)
                .map(|index| {
                    let mut artifact = base.artifacts[index % base.artifacts.len()].clone();
                    artifact.id = format!("G{graph_index:02}-NODE-{index:02}");
                    artifact.kind = "generated_test_node".to_owned();
                    artifact.claim_id = format!("claim-{index}");
                    artifact.scope_id = format!("scope-{index}");
                    artifact.version = 1;
                    artifact.synthetic_payload =
                        format!("SYNTHETIC GENERATED DAG | graph={graph_index} | node={index}");
                    artifact.content_sha256 = sha256_hex(artifact.synthetic_payload.as_bytes());
                    artifact.depends_on = (0..index)
                        .filter(|parent| {
                            *parent + 1 == index
                                || (graph_index * 31 + index * 7 + *parent * 3) % 13 < 3
                        })
                        .map(|parent| {
                            format!("G{graph_index:02}-NODE-{parent:02}")
                        })
                        .collect();
                    artifact
                })
                .collect();
            if graph_index % 2 == 1 {
                fixture.artifacts.reverse();
                for artifact in &mut fixture.artifacts {
                    artifact.depends_on.reverse();
                }
            }
            for trigger in &fixture.artifacts {
                assert_eq!(
                    invalidation_closure(&fixture, &trigger.id),
                    reference_oracle(&fixture, &trigger.id),
                    "generated graph {graph_index} oracle mismatch at {}",
                    trigger.id
                );
            }
        }
    }

    #[test]
    fn exact_frozen_bytes_are_required() {
        let mut changed = FIXTURE_BYTES.to_vec();
        changed.push(b' ');
        let errors = verify_fixture_bytes(&changed).expect_err("changed bytes must fail");
        assert!(errors.iter().any(|error| error.contains("digest mismatch")));
    }

    #[test]
    fn sha256_matches_known_answer() {
        assert_eq!(sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    }

    #[test]
    fn artifact_requires_matching_claim_scope_status_coverage_and_digest() {
        let fixture = read_fixture();
        let artifact = &fixture.artifacts[0];
        assert!(artifact_supports_claim(artifact, &artifact.claim_id, &artifact.scope_id));
        assert!(!artifact_supports_claim(artifact, &artifact.claim_id, "another-scope"));
        let mut stale = artifact.clone();
        stale.status = "stale".to_owned();
        assert!(!artifact_supports_claim(&stale, &stale.claim_id, &stale.scope_id));
        let mut incomplete = artifact.clone();
        incomplete.coverage_complete = false;
        assert!(!artifact_supports_claim(
            &incomplete, &incomplete.claim_id, &incomplete.scope_id
        ));
        let mut tampered = artifact.clone();
        tampered.synthetic_payload.push_str(" tampered");
        assert!(!artifact_supports_claim(&tampered, &tampered.claim_id, &tampered.scope_id));
    }

    #[test]
    fn production_closure_agrees_with_independent_dependency_trace_oracle() {
        let fixture = read_fixture();
        for event in &fixture.events {
            assert_eq!(
                invalidation_closure(&fixture, &event.trigger_artifact_id),
                reference_oracle(&fixture, &event.trigger_artifact_id),
                "oracle mismatch for {}", event.id
            );
        }
    }

    #[test]
    fn supplier_change_invalidates_as_built_and_commissioning_not_upstream_site_evidence() {
        let report = report();
        assert_eq!(
            impact(&report, "EV-03-MATERIAL-SUBSTITUTION").invalidated_descendant_ids,
            vec!["D-ASBUILT-RECEIPT".to_owned(), "D-COMMISSIONING-RECEIPT".to_owned()]
        );
        for id in ["E-GEO-SOURCE", "E-FLOOD-SURVEY", "D-SITE-DECISION", "D-STRUCTURAL-CALC"] {
            assert!(!supplier.invalidated_descendant_ids.iter().any(|item| item == id));
        }
    }

    #[test]
    fn furnishing_change_does_not_invalidate_terrain_or_structure() {
        let report = report();
        assert_eq!(
            impact(&report, "EV-04-FURNISHING-SUBSTITUTION").invalidated_descendant_ids,
            vec!["D-FURNISHING-INSTALL".to_owned()]
        );
        let furnishing = impact(&report, "EV-04-FURNISHING-SUBSTITUTION");
        for id in ["E-GEO-SOURCE", "D-SITE-DECISION", "D-STRUCTURAL-CALC"] {
            assert!(!furnishing.invalidated_descendant_ids.iter().any(|item| item == id));
        }
    }

    #[test]
    fn duplicate_event_delivery_is_idempotent() {
        let mut fixture = read_fixture();
        fixture.events.push(fixture.events[2].clone());
        let repeated = evaluate_fixture(&fixture, FIXTURE_SHA256.to_owned()).unwrap();
        let baseline = report();
        assert_eq!(repeated.duplicate_events_ignored, 1);
        assert_eq!(repeated.unique_event_count, baseline.unique_event_count);
        assert_eq!(repeated.event_impacts, baseline.event_impacts);
    }

    #[test]
    fn event_and_artifact_order_do_not_change_the_report() {
        let mut fixture = read_fixture();
        let baseline = evaluate_fixture(&fixture, FIXTURE_SHA256.to_owned()).unwrap();
        fixture.events.reverse();
        fixture.artifacts.reverse();
        for artifact in &mut fixture.artifacts { artifact.depends_on.reverse(); }
        let reordered = evaluate_fixture(&fixture, FIXTURE_SHA256.to_owned()).unwrap();
        assert_eq!(baseline, reordered);
    }

    #[test]
    fn conflicting_event_ids_are_rejected() {
        let mut fixture = read_fixture();
        let mut duplicate = fixture.events[0].clone();
        duplicate.reason.push_str(" conflicting payload");
        fixture.events.push(duplicate);
        assert!(!validate_fixture(&fixture).is_empty());
        assert!(evaluate_fixture(&fixture, FIXTURE_SHA256.to_owned()).is_err());
    }

    #[test]
    fn unknown_dependency_and_cycle_fail_closed() {
        let mut fixture = read_fixture();
        fixture.artifacts[0].depends_on.push("UNKNOWN".to_owned());
        assert!(!validate_fixture(&fixture).is_empty());
        let mut fixture = read_fixture();
        fixture.artifacts[0].depends_on.push("D-COMMISSIONING-RECEIPT".to_owned());
        assert!(validate_fixture(&fixture).iter().any(|error| error.contains("cycle")));
    }

    #[test]
    fn incomplete_coverage_cannot_be_marked_valid() {
        let mut fixture = read_fixture();
        fixture.artifacts[0].coverage_complete = false;
        assert!(validate_fixture(&fixture)
            .iter()
            .any(|error| error.contains("incomplete coverage")));
    }

    #[test]
    fn unknown_fields_are_rejected() {
        let mut value: serde_json::Value = serde_json::from_slice(FIXTURE_BYTES).unwrap();
        value["unreviewed_approval_override"] = serde_json::Value::Bool(true);
        assert!(serde_json::from_value::<Fixture>(value).is_err());
    }
}
