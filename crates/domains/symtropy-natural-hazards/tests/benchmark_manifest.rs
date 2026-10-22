// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Structural checks for the public multi-hazard benchmark manifest.
//! These tests validate inventory quality, not physical model skill.

use serde_json::Value;
use std::collections::BTreeSet;

const MANIFEST: &str =
    include_str!("../../../../docs/research/natural-hazards/benchmark-manifest-v0.1.json");

fn manifest() -> Value {
    serde_json::from_str(MANIFEST).expect("benchmark manifest must remain valid JSON")
}

#[test]
fn manifest_declares_correct_schema_and_claim_ceiling() {
    let root = manifest();
    assert_eq!(
        root["schema"].as_str(),
        Some("symtropy.natural_hazard_benchmark_manifest.v0.1")
    );
    assert_eq!(
        root["claim_ceiling"].as_str(),
        Some("catalogue_and_work_plan_only")
    );
    assert_eq!(
        root["global_gate_policy"]["unsupported_hazard"].as_str(),
        Some("unavailable")
    );
    assert_eq!(
        root["global_gate_policy"]["universal_realism_score"].as_bool(),
        Some(false)
    );
}

#[test]
fn every_hazard_has_unique_identity_and_measurable_candidate_outputs() {
    let root = manifest();
    let hazards = root["hazards"]
        .as_array()
        .expect("hazards must be an array");
    assert!(
        hazards.len() >= 15,
        "the initial catalogue must cover multiple distinct hazard families"
    );

    let allowed_statuses: BTreeSet<&str> = [
        "catalogued_only",
        "in_architecture_work_not_global_capability",
        "qualification_plan_created",
        "cascade_design_only",
    ]
    .into_iter()
    .collect();
    let mut ids = BTreeSet::new();

    for hazard in hazards {
        let id = hazard["id"].as_str().expect("hazard ID must be a string");
        assert!(!id.trim().is_empty(), "hazard ID must not be blank");
        assert!(ids.insert(id), "duplicate hazard ID: {id}");

        let family = hazard["family"]
            .as_str()
            .expect("hazard family is required");
        assert!(
            !family.trim().is_empty(),
            "{id}: hazard family must not be blank"
        );

        let status = hazard["coverage_status"]
            .as_str()
            .expect("coverage status is required");
        assert!(
            allowed_statuses.contains(status),
            "{id}: unknown coverage status {status}"
        );
        assert_ne!(
            status, "qualified",
            "{id}: the inventory must not imply qualification from catalogue presence"
        );

        let reference = hazard["candidate_reference"]
            .as_str()
            .expect("candidate reference is required");
        assert!(
            !reference.trim().is_empty(),
            "{id}: candidate reference must not be blank"
        );

        let metrics = hazard["candidate_metrics"]
            .as_array()
            .expect("candidate metrics must be an array");
        assert!(
            !metrics.is_empty(),
            "{id}: at least one candidate metric is required"
        );
        for metric in metrics {
            assert!(
                metric
                    .as_str()
                    .is_some_and(|value| !value.trim().is_empty()),
                "{id}: metric names must be non-empty strings"
            );
        }
    }
}

#[test]
fn all_reference_links_are_https_and_have_unique_names() {
    let root = manifest();
    let sources = root["source_references"]
        .as_array()
        .expect("source_references must be an array");
    assert!(!sources.is_empty());

    let mut names = BTreeSet::new();
    for source in sources {
        let name = source["name"].as_str().expect("source name is required");
        let url = source["url"].as_str().expect("source URL is required");
        assert!(!name.trim().is_empty());
        assert!(names.insert(name), "duplicate source name: {name}");
        assert!(
            url.starts_with("https://"),
            "{name}: source URL must use HTTPS"
        );
    }
}

#[test]
fn existing_work_links_are_issue_refs_not_fabricated_solver_claims() {
    let root = manifest();
    let hazards = root["hazards"]
        .as_array()
        .expect("hazards must be an array");
    let mut linked_work_items = 0usize;

    for hazard in hazards {
        if let Some(items) = hazard["local_work"].as_array() {
            for item in items {
                let issue = item.as_str().expect("local work ref must be a string");
                assert!(
                    issue.starts_with('#') && issue[1..].chars().all(|ch| ch.is_ascii_digit()),
                    "local work references must be explicit issue numbers: {issue}"
                );
                linked_work_items += 1;
            }
        }
    }

    assert!(
        linked_work_items >= 3,
        "related in-repository work should be traceable from the manifest"
    );
}
