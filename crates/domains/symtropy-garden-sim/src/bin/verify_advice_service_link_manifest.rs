// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Independent structural verifier for the frozen advice-to-service link corpus.
//!
//! It does not call the link assessor and does not establish that any referenced
//! advice, consent, source data, or service outcome is authentic. Rust tests run
//! separately and are the semantic oracle for the declared synthetic cases.

use std::collections::BTreeSet;
use std::process::ExitCode;

use serde_json::Value;

const FIXTURE: &str = include_str!("../../fixtures/advice_service_link_cases.v1.json");
const SCHEMA: &str = include_str!("../../fixtures/advice_service_link_cases.v1.schema.json");
const TEST_SOURCE: &str = include_str!("../advice_service_link.rs");
const EXPECTED_COUNT: usize = 20;
const DIALECT: &str = "https://json-schema.org/draft/2020-12/schema";
const MUTATIONS: [&str; EXPECTED_COUNT] = [
    "no_link",
    "valid_link",
    "scope_mismatch",
    "period_mismatch",
    "task_mismatch",
    "request_reference_mismatch",
    "missing_authorization_ref",
    "duplicate_outcome_ref",
    "empty_outcome_ref",
    "stale_link",
    "disputed_link",
    "unavailable_link",
    "revoked_sharing",
    "unsupported_schema",
    "empty_advice_episode_ref",
    "empty_source_ref",
    "empty_recorded_time",
    "empty_decision_ref",
    "digest_shaped_source",
    "pending_link",
];

fn string_enum<'a>(schema: &'a Value, pointer: &str) -> Result<BTreeSet<&'a str>, String> {
    let values = schema
        .pointer(pointer)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("schema enum missing at {pointer}"))?;
    let mut set = BTreeSet::new();
    for value in values {
        let item = value
            .as_str()
            .ok_or_else(|| format!("schema enum at {pointer} contains a non-string"))?;
        set.insert(item);
    }
    Ok(set)
}

fn verify() -> Result<(), String> {
    let fixture: Value =
        serde_json::from_str(FIXTURE).map_err(|error| format!("fixture JSON parse failed: {error}"))?;
    let schema: Value =
        serde_json::from_str(SCHEMA).map_err(|error| format!("schema JSON parse failed: {error}"))?;

    if schema.get("$schema").and_then(Value::as_str) != Some(DIALECT) {
        return Err("schema dialect missing or unsupported".to_owned());
    }
    if fixture.get("schema_version").and_then(Value::as_u64) != Some(1) {
        return Err("fixture schema_version must be 1".to_owned());
    }
    if fixture.get("status").and_then(Value::as_str) != Some("synthetic_contract_cases") {
        return Err("fixture status drifted".to_owned());
    }

    let cases = fixture
        .get("cases")
        .and_then(Value::as_array)
        .ok_or_else(|| "fixture cases array missing".to_owned())?;
    if cases.len() != EXPECTED_COUNT {
        return Err(format!("expected {EXPECTED_COUNT} cases, found {}", cases.len()));
    }

    let schema_mutations = string_enum(&schema, "/properties/cases/items/properties/mutation/enum")?;
    let schema_dispositions = string_enum(
        &schema,
        "/properties/cases/items/properties/expected_disposition/enum",
    )?;
    let schema_blockers = string_enum(
        &schema,
        "/properties/cases/items/properties/expected_blockers/items/enum",
    )?;

    let allowed_dispositions: BTreeSet<&str> = [
        "unlinked_service_request",
        "linked_reference_validated",
        "requires_resolution",
        "sharing_revoked",
        "blocked",
    ]
    .into_iter()
    .collect();

    if schema_dispositions != allowed_dispositions {
        return Err("schema disposition enum diverges from verifier contract".to_owned());
    }

    let mut ids = BTreeSet::new();
    let mut mutations = BTreeSet::new();
    for (index, case) in cases.iter().enumerate() {
        let required = ["id", "mutation", "expected_disposition", "expected_blockers"];
        for key in required {
            if case.get(key).is_none() {
                return Err(format!("case index {index} lacks required field {key}"));
            }
        }

        let id = case["id"]
            .as_str()
            .ok_or_else(|| format!("case index {index} id must be a string"))?;
        if !ids.insert(id) {
            return Err(format!("duplicate case id {id}"));
        }

        let mutation = case["mutation"]
            .as_str()
            .ok_or_else(|| format!("case {id}: mutation must be a string"))?;
        if !mutations.insert(mutation) {
            return Err(format!("duplicate mutation {mutation}"));
        }
        if !MUTATIONS.contains(&mutation) || !schema_mutations.contains(mutation) {
            return Err(format!("case {id}: mutation {mutation} is not in the pinned v1 contract"));
        }
        if !TEST_SOURCE.contains(&format!("\"{mutation}\" =>")) {
            return Err(format!("case {id}: mutation {mutation} has no matching Rust mutation arm"));
        }

        let disposition = case["expected_disposition"]
            .as_str()
            .ok_or_else(|| format!("case {id}: expected_disposition must be a string"))?;
        if !schema_dispositions.contains(disposition) {
            return Err(format!("case {id}: unknown expected disposition {disposition}"));
        }

        let blockers = case["expected_blockers"]
            .as_array()
            .ok_or_else(|| format!("case {id}: expected_blockers must be an array"))?;
        let mut unique_blockers = BTreeSet::new();
        for blocker in blockers {
            let blocker = blocker
                .as_str()
                .ok_or_else(|| format!("case {id}: blocker codes must be strings"))?;
            if !schema_blockers.contains(blocker) {
                return Err(format!("case {id}: blocker {blocker} is not in the schema enum"));
            }
            if !unique_blockers.insert(blocker) {
                return Err(format!("case {id}: duplicate expected blocker {blocker}"));
            }
        }
    }

    if mutations.len() != MUTATIONS.len()
        || MUTATIONS.iter().any(|mutation| !mutations.contains(*mutation))
    {
        return Err("fixture must cover every pinned mutation exactly once".to_owned());
    }

    println!(
        "STRUCTURE_VALID_ONLY cases={} mutations={} dialect={}",
        cases.len(),
        mutations.len(),
        DIALECT
    );
    println!(
        "Schema and fixture structure only; semantic outcomes require Rust tests, and links never prove authorization or causality."
    );
    Ok(())
}

fn main() -> ExitCode {
    match verify() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("advice-service-link manifest rejected: {error}");
            ExitCode::FAILURE
        }
    }
}
