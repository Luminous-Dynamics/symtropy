// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
#![forbid(unsafe_code)]

use serde::de::{MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use serde_json::{Map, Number, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::env;
use std::fmt;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::ExitCode;

struct StrictJson(Value);

impl<'de> Deserialize<'de> for StrictJson {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct StrictVisitor;

        impl<'de> Visitor<'de> for StrictVisitor {
            type Value = Value;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("JSON with unique object keys")
            }
            fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
                Ok(Value::Bool(value))
            }
            fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
                Ok(Value::Number(Number::from(value)))
            }
            fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
                Ok(Value::Number(Number::from(value)))
            }
            fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Number::from_f64(value)
                    .map(Value::Number)
                    .ok_or_else(|| E::custom("non-finite JSON number"))
            }
            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E> {
                Ok(Value::String(value.to_owned()))
            }
            fn visit_string<E>(self, value: String) -> Result<Self::Value, E> {
                Ok(Value::String(value))
            }
            fn visit_unit<E>(self) -> Result<Self::Value, E> {
                Ok(Value::Null)
            }
            fn visit_none<E>(self) -> Result<Self::Value, E> {
                Ok(Value::Null)
            }
            fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: Deserializer<'de>,
            {
                StrictJson::deserialize(deserializer).map(|parsed| parsed.0)
            }
            fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let mut values = Vec::new();
                while let Some(value) = sequence.next_element::<StrictJson>()? {
                    values.push(value.0);
                }
                Ok(Value::Array(values))
            }
            fn visit_map<A>(self, mut object: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut values = Map::new();
                while let Some(key) = object.next_key::<String>()? {
                    if values.contains_key(&key) {
                        return Err(serde::de::Error::custom(format!(
                            "duplicate JSON object key: {key}"
                        )));
                    }
                    let value = object.next_value::<StrictJson>()?;
                    values.insert(key, value.0);
                }
                Ok(Value::Object(values))
            }
        }

        deserializer.deserialize_any(StrictVisitor).map(StrictJson)
    }
}

fn parse_json(bytes: &[u8]) -> Result<Value, serde_json::Error> {
    let mut decoder = serde_json::Deserializer::from_slice(bytes);
    let value = StrictJson::deserialize(&mut decoder)?.0;
    decoder.end()?;
    Ok(value)
}

fn at<'a>(root: &'a Value, path: &str) -> Option<&'a Value> {
    let mut current = root;
    for part in path.split('.') {
        current = match current {
            Value::Object(object) => object.get(part)?,
            Value::Array(array) => array.get(part.parse::<usize>().ok()?)?,
            _ => return None,
        };
    }
    Some(current)
}
fn text<'a>(root: &'a Value, path: &str) -> Option<&'a str> {
    at(root, path)?.as_str()
}
fn boolean(root: &Value, path: &str) -> Option<bool> {
    at(root, path)?.as_bool()
}
fn number(root: &Value, path: &str) -> Option<u64> {
    at(root, path)?.as_u64()
}

fn set_path(root: &mut Value, path: &str, replacement: Value) -> Result<(), String> {
    let parts: Vec<&str> = path.split('.').collect();
    if parts.is_empty() || parts.iter().any(|part| part.is_empty()) {
        return Err(format!("invalid mutation path {path}"));
    }
    let mut current = root;
    for part in &parts[..parts.len() - 1] {
        current = match current {
            Value::Object(object) => object
                .get_mut(*part)
                .ok_or_else(|| format!("unresolved mutation path {path}"))?,
            Value::Array(array) => {
                let index = part
                    .parse::<usize>()
                    .map_err(|_| format!("invalid array index in {path}"))?;
                array
                    .get_mut(index)
                    .ok_or_else(|| format!("unresolved mutation path {path}"))?
            }
            _ => return Err(format!("mutation traverses scalar at {path}")),
        };
    }
    let leaf = parts[parts.len() - 1];
    match current {
        Value::Object(object) => {
            let target = object
                .get_mut(leaf)
                .ok_or_else(|| format!("unresolved mutation leaf {path}"))?;
            *target = replacement;
        }
        Value::Array(array) => {
            let index = leaf
                .parse::<usize>()
                .map_err(|_| format!("invalid array index in {path}"))?;
            let target = array
                .get_mut(index)
                .ok_or_else(|| format!("unresolved mutation leaf {path}"))?;
            *target = replacement;
        }
        _ => return Err(format!("mutation leaf is scalar at {path}")),
    }
    Ok(())
}

fn utc_seconds(value: &str) -> Option<i64> {
    let bytes = value.as_bytes();
    if bytes.len() != 20
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16] != b':'
        || bytes[19] != b'Z'
    {
        return None;
    }
    for (start, end) in [(0, 4), (5, 7), (8, 10), (11, 13), (14, 16), (17, 19)] {
        if !bytes[start..end].iter().all(u8::is_ascii_digit) {
            return None;
        }
    }
    let y = value[0..4].parse::<i64>().ok()?;
    let m = value[5..7].parse::<i64>().ok()?;
    let d = value[8..10].parse::<i64>().ok()?;
    let hh = value[11..13].parse::<i64>().ok()?;
    let mm = value[14..16].parse::<i64>().ok()?;
    let ss = value[17..19].parse::<i64>().ok()?;
    let leap = y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
    let days_in_month = match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return None,
    };
    if y == 0 || !(1..=days_in_month).contains(&d)
        || !(0..=23).contains(&hh) || !(0..=59).contains(&mm)
        || !(0..=59).contains(&ss)
    {
        return None;
    }
    let adjusted_y = y - if m <= 2 { 1 } else { 0 };
    let era = adjusted_y / 400;
    let yoe = adjusted_y - era * 400;
    let mp = m + if m > 2 { -3 } else { 9 };
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    Some(days * 86_400 + hh * 3_600 + mm * 60 + ss)
}
fn time_at(root: &Value, path: &str) -> Option<i64> {
    utc_seconds(text(root, path)?)
}
fn same_fields(left: &Value, right: &Value, fields: &[&str]) -> bool {
    fields.iter().all(|field| left.get(*field) == right.get(*field))
}

fn schema_guard(schema: &Value) -> Result<(), String> {
    if text(schema, "$schema") != Some("https://json-schema.org/draft/2020-12/schema") {
        return Err("schema must declare JSON Schema 2020-12".to_owned());
    }
    if number(schema, "properties.cases.minItems") != Some(32) {
        return Err("schema must require at least 32 cases".to_owned());
    }
    for (definition, field) in [
        ("federate", "asset_id"),
        ("federate", "serial_id"),
        ("federate", "parent_assembly_id"),
        ("relationship", "asset_id"),
        ("relationship", "serial_id"),
        ("relationship", "parent_assembly_id"),
    ] {
        let required = at(schema, &format!("$defs.{definition}.required"))
            .and_then(Value::as_array)
            .ok_or_else(|| format!("missing $defs.{definition}.required"))?;
        if !required.iter().any(|item| item.as_str() == Some(field))
            || text(schema, &format!("$defs.{definition}.properties.{field}.type"))
                != Some("string")
        {
            return Err(format!("schema must require string field {definition}.{field}"));
        }
    }
    Ok(())
}

fn effectivity(composition: &Value, cutoff: i64) -> &'static str {
    let Some(start) = time_at(composition, "subject.effectivity_start_utc") else {
        return "INVALID_SUBJECT_EFFECTIVITY";
    };
    if start > cutoff {
        return "EFFECTIVITY_NOT_VALID_AT_CUTOFF";
    }
    match at(composition, "subject.effectivity_end_utc") {
        Some(Value::Null) => "EFFECTIVITY_VALID",
        Some(Value::String(value)) => match utc_seconds(value) {
            Some(end) if end > cutoff => "EFFECTIVITY_VALID",
            Some(_) => "EFFECTIVITY_EXPIRED",
            None => "INVALID_SUBJECT_EFFECTIVITY",
        },
        _ => "INVALID_SUBJECT_EFFECTIVITY",
    }
}

fn identity_and_lineage(composition: &Value) -> &'static str {
    let Some(subject) = at(composition, "subject") else {
        return "SUBJECT_IDENTITY_MISSING";
    };
    let Some(federates) = at(composition, "federates").and_then(Value::as_array) else {
        return "FEDERATE_SET_MISSING";
    };
    let Some(relationships) = at(composition, "relationships").and_then(Value::as_array) else {
        return "RELATIONSHIP_SET_MISSING";
    };
    let fields = ["asset_id", "serial_id", "parent_assembly_id"];
    let expected_component = text(subject, "component_id");

    for federate in federates {
        if text(federate, "subject_id") != expected_component {
            if text(federate, "system_role") == Some("maintenance_asset") {
                return "REPLACEMENT_LINEAGE_MISSING";
            }
            return "IDENTITY_MISMATCH";
        }
        if !same_fields(federate, subject, &fields) {
            return "IDENTITY_MISMATCH";
        }
    }
    for relationship in relationships {
        if text(relationship, "subject_id") != expected_component
            || !same_fields(relationship, subject, &fields)
        {
            return "RELATIONSHIP_SUBJECT_MISMATCH";
        }
    }
    "IDENTITY_AND_LINEAGE_VALID"
}

fn configurations(composition: &Value) -> &'static str {
    let Some(expected) = text(composition, "subject.configuration_revision") else {
        return "CONFIGURATION_REVISION_MISSING";
    };
    let Some(federates) = at(composition, "federates").and_then(Value::as_array) else {
        return "FEDERATE_SET_MISSING";
    };
    for federate in federates {
        let Some(actual) = text(federate, "configuration_revision") else {
            return "CONFIGURATION_REVISION_MISSING";
        };
        if actual == expected {
            continue;
        }
        match text(federate, "federate_id") {
            Some("mes") => return "CONFIGURATION_MISMATCH",
            Some("eam") if actual < expected => return "SUPERSEDED_REVISION",
            Some("eam") if actual > expected => return "FEDERATE_STATE_CONFLICT",
            _ => return "CONFIGURATION_MISMATCH",
        }
    }
    "CONFIGURATION_VALID"
}

fn temporal_budgets(composition: &Value) -> &'static str {
    let Some(cutoff) = time_at(composition, "evaluation_cutoff_utc") else {
        return "INVALID_DECISION_CUTOFF";
    };
    let Some(max_source_age) = number(composition, "commissioning.max_source_age_s") else {
        return "TEMPORAL_BUDGET_MISSING";
    };
    let Some(max_latency) = number(composition, "commissioning.max_update_latency_s") else {
        return "TEMPORAL_BUDGET_MISSING";
    };
    let skew = number(composition, "commissioning.clock_skew_tolerance_s").unwrap_or(0) as i64;
    let Some(federates) = at(composition, "federates").and_then(Value::as_array) else {
        return "FEDERATE_SET_MISSING";
    };

    for federate in federates {
        let Some(source) = time_at(federate, "source_time_utc") else {
            return "INVALID_SOURCE_TIME";
        };
        let Some(transaction) = time_at(federate, "transaction_time_utc") else {
            return "INVALID_TRANSACTION_TIME";
        };
        let Some(retrieved) = time_at(federate, "retrieved_at_utc") else {
            return "INVALID_RETRIEVAL_TIME";
        };
        if source > cutoff || retrieved > cutoff {
            return "TEMPORAL_ROLE_INCONSISTENCY";
        }
        if cutoff - source > max_source_age as i64
            || cutoff - retrieved > max_source_age as i64
        {
            return "SOURCE_STALE";
        }
        if retrieved < source {
            return "TEMPORAL_ROLE_INCONSISTENCY";
        }
        if transaction > retrieved {
            if transaction - retrieved > skew {
                return "CLOCK_SKEW_EXCEEDED";
            }
        } else {
            if transaction < source {
                return "TEMPORAL_ROLE_INCONSISTENCY";
            }
            if retrieved - transaction > max_latency as i64 {
                return "UPDATE_LATENCY_EXCEEDED";
            }
        }
    }
    "TEMPORAL_BUDGETS_VALID"
}

fn authority(composition: &Value, cutoff: i64) -> &'static str {
    let Some(federates) = at(composition, "federates").and_then(Value::as_array) else {
        return "FEDERATE_SET_MISSING";
    };
    for federate in federates {
        match text(federate, "approval.status") {
            Some("revoked") => return "APPROVAL_REVOKED",
            Some("unknown" | "pending") => return "SOURCE_AUTHENTICITY_UNKNOWN",
            Some("approved" | "accepted" | "recorded") => {}
            _ => return "APPROVAL_STATUS_INVALID",
        }
        match at(federate, "approval.expires_at_utc") {
            Some(Value::Null) => {}
            Some(Value::String(value)) => match utc_seconds(value) {
                Some(expiry) if expiry > cutoff => {}
                Some(_) => return "APPROVAL_EXPIRED",
                None => return "INVALID_APPROVAL_EXPIRY",
            },
            _ => return "INVALID_APPROVAL_EXPIRY",
        }
        if text(federate, "system_role") == Some("quality_inspection") {
            let Some(record_id) = text(federate, "record_id") else {
                return "AUTHORITY_SCOPE_MISMATCH";
            };
            let expected_scope = format!("inspection_record:{record_id}");
            if text(federate, "approval.scope") != Some(expected_scope.as_str()) {
                return "AUTHORITY_SCOPE_MISMATCH";
            }
        }
    }
    "AUTHORITY_VALID"
}

fn evaluate(composition: &Value) -> &'static str {
    let Some(cutoff) = time_at(composition, "evaluation_cutoff_utc") else {
        return "INVALID_DECISION_CUTOFF";
    };
    if boolean(composition, "projection.source_state_writeback") == Some(true) {
        return "SYNTHETIC_WRITEBACK_FORBIDDEN";
    }
    if text(composition, "projection.projection_revision")
        .is_none_or(|revision| revision.is_empty() || revision == "latest")
    {
        return "MUTABLE_PROJECTION_REVISION";
    }

    let Some(required) = at(composition, "commissioning.required_federates")
        .and_then(Value::as_array) else {
            return "REQUIRED_FEDERATE_MISSING";
        };
    let Some(federates) = at(composition, "federates").and_then(Value::as_array) else {
        return "FEDERATE_SET_MISSING";
    };
    for required_id in required.iter().filter_map(Value::as_str) {
        if !federates.iter().any(|f| text(f, "federate_id") == Some(required_id)) {
            return "REQUIRED_FEDERATE_MISSING";
        }
    }

    let mut record_ids = BTreeSet::new();
    for federate in federates {
        let Some(record_id) = text(federate, "record_id") else {
            return "SOURCE_RECORD_ID_MISSING";
        };
        if !record_ids.insert(record_id) {
            return "DUPLICATE_RECORD_ID";
        }
        if boolean(federate, "system_of_record") == Some(false)
            && text(federate, "field_ownership") == Some("owner_writable")
        {
            return "SOURCE_OWNERSHIP_CONFLICT";
        }
        if text(federate, "projection_write_mode") != Some("read_only") {
            return "PROJECTION_WRITEBACK_FORBIDDEN";
        }
    }

    let Some(relationships) = at(composition, "relationships").and_then(Value::as_array) else {
        return "RELATIONSHIP_SET_MISSING";
    };
    let mut relationship_ids = BTreeSet::new();
    for relationship in relationships {
        let Some(id) = text(relationship, "relationship_id") else {
            return "RELATIONSHIP_ID_MISSING";
        };
        if !relationship_ids.insert(id) {
            return "DUPLICATE_RELATIONSHIP_ID";
        }
    }

    let result = effectivity(composition, cutoff);
    if result != "EFFECTIVITY_VALID" {
        return result;
    }
    let result = identity_and_lineage(composition);
    if result != "IDENTITY_AND_LINEAGE_VALID" {
        return result;
    }
    let result = configurations(composition);
    if result != "CONFIGURATION_VALID" {
        return result;
    }
    let result = temporal_budgets(composition);
    if result != "TEMPORAL_BUDGETS_VALID" {
        return result;
    }
    let result = authority(composition, cutoff);
    if result != "AUTHORITY_VALID" {
        return result;
    }

    let Some(mappings) = at(composition, "mappings").and_then(Value::as_array) else {
        return "MAPPING_SET_MISSING";
    };
    for mapping in mappings {
        match text(mapping, "mapping_state") {
            Some("unsupported") => return "REQUIRED_MAPPING_UNSUPPORTED",
            Some("lossy") => return "REQUIRED_MAPPING_NOT_EXACT_OR_NORMALIZED",
            Some("exact" | "normalized") => {}
            _ => return "MAPPING_STATE_INVALID",
        }
    }

    let maintenance_changed = federates
        .iter()
        .any(|f| text(f, "state_class") == Some("as_maintained"))
        || relationships
            .iter()
            .any(|r| text(r, "kind") == Some("repair_completed"));
    if maintenance_changed
        && !relationships
            .iter()
            .any(|r| text(r, "kind") == Some("inspection_after_maintenance"))
    {
        return "POST_MAINTENANCE_EVIDENCE_MISSING";
    }

    let projected_state = text(composition, "projection.projected_state_class");
    if projected_state.is_none()
        || !federates.iter().any(|f| text(f, "state_class") == projected_state)
    {
        return "STATE_CLASS_PROMOTION_FORBIDDEN";
    }

    "COMPOSITION_CONTRACT_VALID_NOT_COMMISSIONED"
}

fn apply_case(base: &Value, case: &Value) -> Result<Value, String> {
    let mut composition = base.clone();
    let mutations = at(case, "mutations")
        .and_then(Value::as_array)
        .ok_or_else(|| "case mutations must be an array".to_owned())?;
    for mutation in mutations {
        if text(mutation, "op") != Some("set") {
            return Err("only set mutations are supported".to_owned());
        }
        let path = text(mutation, "path")
            .ok_or_else(|| "mutation path missing".to_owned())?;
        let value = mutation.get("value")
            .cloned()
            .ok_or_else(|| format!("mutation value missing at {path}"))?;
        set_path(&mut composition, path, value)?;
    }
    Ok(composition)
}

struct Summary {
    total: usize,
    matched: usize,
    mismatches: Vec<String>,
    digest: String,
}

fn verify(schema_bytes: &[u8], corpus_bytes: &[u8]) -> Result<Summary, String> {
    let schema = parse_json(schema_bytes).map_err(|error| format!("schema JSON: {error}"))?;
    let corpus = parse_json(corpus_bytes).map_err(|error| format!("corpus JSON: {error}"))?;
    schema_guard(&schema)?;
    if text(&corpus, "corpus_id")
        != Some("luminous.symtropy.maritime-asset-twin-composition")
        || text(&corpus, "corpus_version") != Some("0.1.0")
    {
        return Err("unexpected corpus id or version".to_owned());
    }
    let base = at(&corpus, "base_composition")
        .ok_or_else(|| "base_composition missing".to_owned())?;
    if text(base, "fixture_kind") != Some("synthetic_fixture") {
        return Err("base fixture must be explicitly synthetic".to_owned());
    }
    let cases = at(&corpus, "cases")
        .and_then(Value::as_array)
        .ok_or_else(|| "cases must be an array".to_owned())?;
    if cases.len() != 32 {
        return Err(format!("expected 32 frozen cases, found {}", cases.len()));
    }

    let mut ids = BTreeSet::new();
    let mut matched = 0;
    let mut mismatches = Vec::new();
    for case in cases {
        let id = text(case, "id").ok_or_else(|| "case id missing".to_owned())?;
        if !ids.insert(id) {
            return Err(format!("duplicate case id: {id}"));
        }
        let expected = text(case, "expected_code")
            .ok_or_else(|| format!("expected_code missing for {id}"))?;
        match apply_case(base, case) {
            Ok(composition) => {
                let actual = evaluate(&composition);
                if actual == expected {
                    matched += 1;
                } else {
                    mismatches.push(format!("{id}: expected {expected}, got {actual}"));
                }
            }
            Err(error) => mismatches.push(format!("{id}: invalid mutation: {error}")),
        }
    }

    Ok(Summary {
        total: cases.len(),
        matched,
        mismatches,
        digest: format!("{:x}", Sha256::digest(corpus_bytes)),
    })
}

fn repo_root() -> Result<PathBuf, String> {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .map_err(|error| format!("repository root unavailable: {error}"))
}
fn read_repo_file(root: &Path, relative: &str) -> Result<Vec<u8>, String> {
    let path = Path::new(relative);
    if path.is_absolute()
        || path.components().any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err(format!("unsafe repository path: {relative}"));
    }
    let canonical = root
        .join(path)
        .canonicalize()
        .map_err(|error| format!("{relative}: {error}"))?;
    if !canonical.starts_with(root) {
        return Err(format!("repository path escapes root: {relative}"));
    }
    fs::read(canonical).map_err(|error| format!("{relative}: {error}"))
}

fn run() -> Result<Summary, String> {
    let root = repo_root()?;
    let mut args = env::args().skip(1);
    let schema_path = args.next().unwrap_or_else(|| {
        "docs/tech/MARITIME_ASSET_TWIN_COMPOSITION_V0_1.schema.json".to_owned()
    });
    let corpus_path = args.next().unwrap_or_else(|| {
        "benchmarks/maritime/asset-twin-composition-corpus-v0.1.json".to_owned()
    });
    if args.next().is_some() {
        return Err("usage: maritime-asset-twin [schema-path] [corpus-path]".to_owned());
    }
    let schema = read_repo_file(&root, &schema_path)?;
    let corpus = read_repo_file(&root, &corpus_path)?;
    verify(&schema, &corpus)
}

fn main() -> ExitCode {
    match run() {
        Ok(result) if result.mismatches.is_empty() && result.matched == result.total => {
            println!(
                "CORPUS_ORACLE_PASS cases={} matched={} sha256={}",
                result.total, result.matched, result.digest
            );
            ExitCode::SUCCESS
        }
        Ok(result) => {
            eprintln!(
                "CORPUS_ORACLE_FAIL cases={} matched={} sha256={}",
                result.total, result.matched, result.digest
            );
            for failure in result.mismatches {
                eprintln!("{failure}");
            }
            ExitCode::FAILURE
        }
        Err(error) => {
            eprintln!("CORPUS_ORACLE_ERROR {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicate_object_keys_are_rejected() {
        assert!(parse_json(br#"{"x":1,"x":2}"#).is_err());
    }

    #[test]
    fn nested_unique_json_is_accepted() {
        assert!(parse_json(br#"{"a":[{"b":1},{"c":2}]}"#).is_ok());
    }

    #[test]
    fn calendar_validation_rejects_impossible_dates() {
        assert!(utc_seconds("2026-02-30T00:00:00Z").is_none());
        assert!(utc_seconds("2024-02-29T00:00:00Z").is_some());
        assert!(utc_seconds("2025-02-29T00:00:00Z").is_none());
    }

    #[test]
    fn all_frozen_cases_match_the_independent_oracle() {
        let root = repo_root().expect("repository root");
        let schema = read_repo_file(
            &root,
            "docs/tech/MARITIME_ASSET_TWIN_COMPOSITION_V0_1.schema.json",
        )
        .expect("schema");
        let corpus = read_repo_file(
            &root,
            "benchmarks/maritime/asset-twin-composition-corpus-v0.1.json",
        )
        .expect("corpus");
        let result = verify(&schema, &corpus).expect("verify corpus");
        assert_eq!(result.total, 32);
        assert_eq!(result.matched, 32, "{:?}", result.mismatches);
        assert!(result.mismatches.is_empty());
    }
}
