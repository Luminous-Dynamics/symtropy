// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Deliberately separate JSON-level oracle for the assurance disposition projection.
//!
//! This oracle does not call the production evaluator or its helpers. It is an
//! implementation-diverse consistency check, not independent domain adjudication.

use serde_json::{Value, json};
use std::collections::BTreeSet;
use symtropy_aero_material_qual::assurance_disposition_v0_1::{
    AssuranceStateCaseV0_1, evaluate_assurance_state_case_v0_1,
};

const FIXTURE: &str = include_str!(
    "../../../../docs/research/aviation/examples/aero_assurance_state_vectors_v0_1.json"
);

fn text_at<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn flag_at(value: &Value, key: &str) -> bool {
    value.get(key).and_then(Value::as_bool).unwrap_or(false)
}

fn evidence_refs_are_valid(obligation: &Value) -> bool {
    let Some(references) = obligation.get("evidence_refs").and_then(Value::as_array) else {
        return false;
    };
    let mut distinct = BTreeSet::new();
    references.iter().all(|entry| {
        let Some(reference) = entry.as_str() else {
            return false;
        };
        !reference.trim().is_empty()
            && reference.trim() == reference
            && distinct.insert(reference)
    })
}

fn independent_disposition(case: &Value) -> &'static str {
    let mut reject = text_at(case, "subject_time_effectivity") == Some("INVALID_INTERVAL");
    let mut out_of_scope = false;
    let mut late_knowledge = false;
    let not_effective = text_at(case, "subject_time_effectivity")
        == Some("DEFINITELY_NOT_YET_EFFECTIVE");
    let mut unverified = false;
    let mut incomplete = text_at(case, "subject_time_effectivity") == Some("BOUNDARY_AMBIGUOUS");

    let context = case.get("claim_context").unwrap_or(&Value::Null);
    let digest = text_at(context, "trust_policy_snapshot_digest").unwrap_or("");
    let case_id_ok = text_at(case, "case_id").is_some_and(|id| {
        !id.trim().is_empty() && id == id.trim()
    });
    let context_ok = [
        "claim_id",
        "physical_subject_ref",
        "configuration_ref",
        "as_known_cutoff",
    ]
    .iter()
    .all(|field| {
        text_at(context, field)
            .is_some_and(|value| !value.trim().is_empty() && value == value.trim())
    });
    let digest_ok = digest
        .strip_prefix("sha256:")
        .is_some_and(|hex| {
            hex.len() == 64
                && hex
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        });
    let obligations = case
        .get("obligations")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    if !case_id_ok
        || !context_ok
        || !digest_ok
        || obligations.is_empty()
        || !obligations.iter().any(|item| flag_at(item, "mandatory"))
    {
        incomplete = true;
    }

    for required_kind in [
        "CLAIM_IDENTITY",
        "EVIDENCE_INTEGRITY",
        "PRODUCER_AUTHORITY",
    ] {
        let matching: Vec<&Value> = obligations
            .iter()
            .filter(|item| text_at(item, "kind") == Some(required_kind))
            .collect();
        if matching.is_empty()
            || matching
                .iter()
                .any(|item| !flag_at(item, "mandatory") || !flag_at(item, "hard_gate"))
        {
            unverified = true;
        }
    }

    let mut ids = BTreeSet::new();
    for obligation in obligations {
        let mandatory = flag_at(obligation, "mandatory");
        let id = text_at(obligation, "obligation_id").unwrap_or("");
        if id.trim().is_empty() || id != id.trim() || !ids.insert(id) {
            incomplete = true;
            continue;
        }
        if !mandatory {
            continue;
        }

        let evidence_state = text_at(obligation, "proposition_evidence").unwrap_or("");
        let applicability = text_at(obligation, "applicability").unwrap_or("");
        let authority = text_at(obligation, "authority").unwrap_or("");
        let time = text_at(obligation, "temporal_knowledge").unwrap_or("");
        let observer = text_at(obligation, "observer_coverage").unwrap_or("");
        let verification = text_at(obligation, "verification").unwrap_or("");
        let hard_gate = flag_at(obligation, "hard_gate");
        let refs_valid = evidence_refs_are_valid(obligation);
        let has_reference = refs_valid
            && obligation
                .get("evidence_refs")
                .and_then(Value::as_array)
                .is_some_and(|items| !items.is_empty());

        if !refs_valid || (evidence_state != "NEITHER" && !has_reference) {
            incomplete = true;
        }
        if text_at(obligation, "kind") == Some("PRODUCER_AUTHORITY")
            && authority == "NOT_REQUIRED"
        {
            incomplete = true;
        }

        if hard_gate
            && (((evidence_state == "REFUTED_ONLY" && verification == "VERIFIED")
                || verification == "FAILED")
                && has_reference)
        {
            reject = true;
        }
        if hard_gate && authority == "DENIED" && verification == "VERIFIED" {
            reject = true;
        }
        if applicability == "OUT_OF_SCOPE" && verification == "VERIFIED" {
            out_of_scope = true;
        }
        if time == "NOT_KNOWN_AT_CUTOFF" {
            late_knowledge = true;
        }
        if authority == "UNKNOWN"
            || verification == "UNVERIFIED"
            || (verification == "NOT_RUN"
                && obligation
                    .get("evidence_refs")
                    .and_then(Value::as_array)
                    .is_some_and(|items| !items.is_empty()))
        {
            unverified = true;
        }
        if verification == "NOT_RUN"
            && obligation
                .get("evidence_refs")
                .and_then(Value::as_array)
                .is_some_and(Vec::is_empty)
        {
            incomplete = true;
        }
        if verification == "FAILED" && !hard_gate {
            incomplete = true;
        }
        if applicability == "UNKNOWN"
            || time == "TEMPORAL_CONFLICT"
            || evidence_state != "SUPPORTED_ONLY"
            || observer == "INCOMPLETE"
            || observer == "UNKNOWN"
        {
            incomplete = true;
        }
        if authority == "DENIED" && !(hard_gate && verification == "VERIFIED") {
            incomplete = true;
        }
        if applicability == "OUT_OF_SCOPE" && verification != "VERIFIED" {
            unverified = true;
        }
    }

    if case
        .get("defeaters")
        .and_then(Value::as_array)
        .is_some_and(|defeaters| {
            defeaters.iter().any(|item| {
                text_at(item, "materiality") == Some("BLOCKING")
                    && matches!(
                        text_at(item, "status"),
                        Some("UNRESOLVED" | "CONFIRMED")
                    )
            })
        })
    {
        incomplete = true;
    }

    if reject {
        "REJECTED"
    } else if out_of_scope {
        "OUT_OF_SCOPE"
    } else if late_knowledge {
        "NOT_KNOWN_AT_SNAPSHOT"
    } else if unverified {
        "UNVERIFIED"
    } else if incomplete {
        "INCOMPLETE"
    } else if not_effective {
        "NOT_EFFECTIVE_AT_SUBJECT_TIME"
    } else {
        "REVIEWABLE_FOR_INDEPENDENT_DECISION"
    }
}

fn remove_oracle_annotations(case: &mut Value) {
    if let Some(object) = case.as_object_mut() {
        object.remove("expected_disposition");
        object.remove("expected_blocking_obligation_ids");
        object.remove("expected_reason_codes");
    }
}

fn typed_case(value: &Value) -> AssuranceStateCaseV0_1 {
    serde_json::from_value(value.clone()).expect("serialized vector matches Rust contract")
}

#[test]
fn independent_json_oracle_matches_all_published_state_vectors() {
    let corpus: Value = serde_json::from_str(FIXTURE).expect("state-vector corpus parses");
    let cases = corpus["cases"].as_array().expect("state vector cases");
    assert_eq!(cases.len(), 13);

    for source in cases {
        let expected = text_at(source, "expected_disposition").expect("expected disposition");
        let mut value = source.clone();
        remove_oracle_annotations(&mut value);

        let oracle = independent_disposition(&value);
        let production = evaluate_assurance_state_case_v0_1(&typed_case(&value));
        assert_eq!(oracle, expected, "oracle fixture case {}", source["case_id"]);
        assert_eq!(
            production.disposition.as_str(),
            oracle,
            "production/oracle disagreement for {}",
            source["case_id"]
        );
    }
}

#[test]
fn independent_oracle_agrees_on_12288_product_states() {
    let corpus: Value = serde_json::from_str(FIXTURE).expect("state-vector corpus parses");
    let positive = corpus["cases"]
        .as_array()
        .expect("cases")
        .iter()
        .find(|case| text_at(case, "case_id") == Some("ASV01"))
        .expect("positive control")
        .clone();

    let evidence = ["NEITHER", "SUPPORTED_ONLY", "REFUTED_ONLY", "BOTH"];
    let applicability = ["IN_SCOPE", "OUT_OF_SCOPE", "UNKNOWN"];
    let authority = ["AUTHORIZED", "DENIED", "UNKNOWN", "NOT_REQUIRED"];
    let temporal = [
        "KNOWN_AT_CUTOFF",
        "NOT_KNOWN_AT_CUTOFF",
        "TEMPORAL_CONFLICT",
        "NOT_TEMPORAL",
    ];
    let observer = ["COMPLETE", "INCOMPLETE", "UNKNOWN", "NOT_APPLICABLE"];
    let verification = ["VERIFIED", "FAILED", "UNVERIFIED", "NOT_RUN"];
    let effectivity = [
        "DEFINITELY_EFFECTIVE",
        "DEFINITELY_NOT_YET_EFFECTIVE",
        "BOUNDARY_AMBIGUOUS",
        "INVALID_INTERVAL",
    ];

    let mut compared = 0usize;
    for evidence_state in evidence {
        for applicability_state in applicability {
            for authority_state in authority {
                for temporal_state in temporal {
                    for observer_state in observer {
                        for verification_state in verification {
                            for effectivity_state in effectivity {
                                let mut value = positive.clone();
                                let semantic = value["obligations"]
                                    .as_array_mut()
                                    .expect("obligations")
                                    .iter_mut()
                                    .find(|item| text_at(item, "obligation_id") == Some("semantic"))
                                    .expect("semantic obligation");
                                semantic["proposition_evidence"] = json!(evidence_state);
                                semantic["applicability"] = json!(applicability_state);
                                semantic["authority"] = json!(authority_state);
                                semantic["temporal_knowledge"] = json!(temporal_state);
                                semantic["observer_coverage"] = json!(observer_state);
                                semantic["verification"] = json!(verification_state);
                                value["subject_time_effectivity"] = json!(effectivity_state);
                                remove_oracle_annotations(&mut value);

                                let oracle = independent_disposition(&value);
                                let production =
                                    evaluate_assurance_state_case_v0_1(&typed_case(&value));
                                assert_eq!(
                                    production.disposition.as_str(),
                                    oracle,
                                    "mismatch in product state: evidence={evidence_state}, applicability={applicability_state}, authority={authority_state}, temporal={temporal_state}, observer={observer_state}, verification={verification_state}, effectivity={effectivity_state}"
                                );
                                compared += 1;
                            }
                        }
                    }
                }
            }
        }
    }
    assert_eq!(compared, 12_288);
}

#[test]
fn obligation_order_permutations_preserve_oracle_and_production_results() {
    let corpus: Value = serde_json::from_str(FIXTURE).expect("state-vector corpus parses");
    let source = &corpus["cases"][0];
    let mut original = source.clone();
    remove_oracle_annotations(&mut original);

    let baseline_oracle = independent_disposition(&original);
    let baseline_production =
        evaluate_assurance_state_case_v0_1(&typed_case(&original)).disposition.as_str();

    let mut reordered = original.clone();
    reordered["obligations"].as_array_mut().expect("obligations").reverse();
    let permuted_oracle = independent_disposition(&reordered);
    let permuted_production =
        evaluate_assurance_state_case_v0_1(&typed_case(&reordered)).disposition.as_str();

    assert_eq!(baseline_oracle, permuted_oracle);
    assert_eq!(baseline_production, permuted_production);
    assert_eq!(baseline_oracle, baseline_production);
}
