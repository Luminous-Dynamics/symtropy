// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
#![forbid(unsafe_code)]

use serde::de::{MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use serde_json::{Map, Number, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fmt;

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
                    .ok_or_else(|| E::custom("non-finite number"))
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
    let parsed = StrictJson::deserialize(&mut decoder)?.0;
    decoder.end()?;
    Ok(parsed)
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

fn integer(root: &Value, path: &str) -> Option<u64> {
    at(root, path)?.as_u64()
}

fn set_path(root: &mut Value, path: &str, replacement: Value) -> Result<(), String> {
    let parts: Vec<&str> = path.split('.').collect();
    if parts.is_empty() || parts.iter().any(|part| part.is_empty()) {
        return Err(format!("invalid mutation path: {path}"));
    }

    let mut current = root;
    for part in &parts[..parts.len() - 1] {
        current = match current {
            Value::Object(object) => object
                .get_mut(*part)
                .ok_or_else(|| format!("unresolved mutation path: {path}"))?,
            Value::Array(array) => {
                let index = part
                    .parse::<usize>()
                    .map_err(|_| format!("invalid array index in path: {path}"))?;
                array
                    .get_mut(index)
                    .ok_or_else(|| format!("unresolved mutation path: {path}"))?
            }
            _ => return Err(format!("mutation traverses scalar: {path}")),
        };
    }

    let leaf = parts[parts.len() - 1];
    match current {
        Value::Object(object) => {
            let target = object
                .get_mut(leaf)
                .ok_or_else(|| format!("unresolved mutation leaf: {path}"))?;
            *target = replacement;
        }
        Value::Array(array) => {
            let index = leaf
                .parse::<usize>()
                .map_err(|_| format!("invalid array index in path: {path}"))?;
            let target = array
                .get_mut(index)
                .ok_or_else(|| format!("unresolved mutation leaf: {path}"))?;
            *target = replacement;
        }
        _ => return Err(format!("mutation leaf is not a container: {path}")),
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

    let year = value[0..4].parse::<i64>().ok()?;
    let month = value[5..7].parse::<i64>().ok()?;
    let day = value[8..10].parse::<i64>().ok()?;
    let hour = value[11..13].parse::<i64>().ok()?;
    let minute = value[14..16].parse::<i64>().ok()?;
    let second = value[17..19].parse::<i64>().ok()?;

    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days_in_month = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return None,
    };
    if year == 0
        || !(1..=days_in_month).contains(&day)
        || !(0..=23).contains(&hour)
        || !(0..=59).contains(&minute)
        || !(0..=59).contains(&second)
    {
        return None;
    }

    let adjusted_year = year - i64::from(month <= 2);
    let era = adjusted_year / 400;
    let year_of_era = adjusted_year - era * 400;
    let adjusted_month = month + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * adjusted_month + 2) / 5 + day - 1;
    let day_of_era =
        year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    let days_since_epoch = era * 146_097 + day_of_era - 719_468;

    Some(days_since_epoch * 86_400 + hour * 3_600 + minute * 60 + second)
}

fn time_at(root: &Value, path: &str) -> Option<i64> {
    utc_seconds(text(root, path)?)
}

fn is_null(root: &Value, path: &str) -> bool {
    at(root, path).is_none_or(Value::is_null)
}

fn is_proven_claim(bundle: &Value, index: usize) -> bool {
    text(bundle, &format!("claims.{index}.status")) == Some("proven_in_declared_scope")
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn schema_layer_rejection(bundle: &Value) -> &'static str {
    for product in ["s131", "s127", "s123", "s124", "s129"] {
        if text(bundle, &format!("{product}.product_edition")) != Some("2.0.0")
            || text(bundle, &format!("{product}.s100_edition")) != Some("5.2.0")
        {
            return "PRODUCT_DEPENDENCY_MISMATCH";
        }
    }

    for product in ["s131", "s127", "s123", "s124", "s129"] {
        let Some(digest) = text(bundle, &format!("{product}.source.artifact_sha256")) else {
            return "SOURCE_ARTIFACT_DIGEST_INVALID";
        };
        if !valid_sha256(digest) {
            return "SOURCE_ARTIFACT_DIGEST_INVALID";
        }
    }

    "SCHEMA_LAYER_VALID"
}

fn evaluate(bundle: &Value) -> &'static str {
    let Some(cutoff) = time_at(bundle, "evaluation_cutoff_utc") else {
        return "INVALID_EVALUATION_CUTOFF";
    };

    if text(bundle, "run_evidence.writeback_state") != Some("effects_disabled") {
        return "SOURCE_WRITEBACK_FORBIDDEN";
    }

    // S-131 describes infrastructure. A berth feature is not live inventory.
    if text(bundle, "s131.berth.availability_state")
        == Some("provider_confirmed_available")
        && is_null(bundle, "s131.berth.live_inventory_source_ref")
    {
        return "BERTH_AVAILABILITY_SOURCE_MISSING";
    }
    if is_proven_claim(bundle, 0)
        && text(bundle, "s131.berth.availability_state") == Some("unknown")
    {
        return "S131_CONTEXT_NOT_AVAILABILITY";
    }

    // S-127 service areas do not create pilot assignments or VTS clearances.
    if text(bundle, "s127.traffic.pilot_assignment_state") == Some("assigned_by_provider")
        && is_null(bundle, "s127.traffic.pilot_assignment_ref")
    {
        return "PILOT_ASSIGNMENT_AUTHORITY_MISSING";
    }
    if text(bundle, "s127.traffic.clearance_state") == Some("provider_confirmed")
        && is_null(bundle, "s127.traffic.clearance_ref")
    {
        return "VTS_CLEARANCE_SOURCE_MISSING";
    }

    // S-123 coverage is not measured link quality or vessel delivery.
    if text(bundle, "s123.radio.live_link_state") == Some("measured_available")
        && is_null(bundle, "s123.radio.link_measurement_ref")
    {
        return "RADIO_LINK_MEASUREMENT_MISSING";
    }
    if text(bundle, "s123.radio.vessel_delivery_state")
        == Some("received_with_device_receipt")
        && is_null(bundle, "s123.radio.delivery_receipt_ref")
    {
        return "RADIO_DELIVERY_RECEIPT_MISSING";
    }

    // S-124 warning state and bulletin completeness are distinct from vessel receipt.
    if text(bundle, "s124.warning.lifecycle_state") == Some("cancelled")
        && is_proven_claim(bundle, 3)
    {
        return "CANCELLED_WARNING_NOT_CURRENT";
    }
    if text(bundle, "s124.warning.bulletin_complete") == Some("false")
        && is_proven_claim(bundle, 3)
    {
        return "WARNING_COMPLETENESS_UNKNOWN";
    }
    if let Some(retrieved) = time_at(bundle, "s124.warning.bulletin_retrieved_at_utc") {
        let max_age = integer(bundle, "freshness_policy.max_warning_bulletin_age_s")
            .unwrap_or(0) as i64;
        if cutoff.saturating_sub(retrieved) > max_age {
            return "WARNING_CURRENTNESS_UNKNOWN";
        }
    } else {
        return "WARNING_CURRENTNESS_UNKNOWN";
    }
    if text(bundle, "s124.warning.vessel_delivery_state") != Some("unknown")
        && is_null(bundle, "s124.warning.delivery_receipt_ref")
    {
        return "WARNING_RECEIPT_EVIDENCE_MISSING";
    }

    // S-129 plan applicability, freshness, replacement and claim limits.
    if text(bundle, "s129.ukc.purpose") == Some("pre_plan") && is_proven_claim(bundle, 4) {
        return "PREPLAN_NOT_ACTUAL_PLAN";
    }
    if text(bundle, "s129.ukc.vessel_id") != text(bundle, "subject.vessel_id") {
        return "UKC_PLAN_VESSEL_MISMATCH";
    }
    if text(bundle, "s129.ukc.route_id") != text(bundle, "subject.route_id")
        || integer(bundle, "s129.ukc.route_version")
            != integer(bundle, "subject.route_version")
    {
        return "UKC_PLAN_ROUTE_MISMATCH";
    }
    let retained = at(bundle, "s129.ukc.retained_previous_feature_ids")
        .and_then(Value::as_array)
        .is_some_and(|values| !values.is_empty());
    if text(bundle, "s129.ukc.purpose") == Some("actual_update")
        && (text(bundle, "s129.ukc.replacement_scope") != Some("whole_dataset") || retained)
    {
        return "UKC_PARTIAL_REPLACEMENT_FORBIDDEN";
    }
    if let Some(expiry) = time_at(bundle, "s129.ukc.valid_until_utc") {
        if expiry <= cutoff {
            return "UKC_PLAN_EXPIRED";
        }
    } else {
        return "UKC_PLAN_EXPIRED";
    }
    if let Some(retrieved) = time_at(bundle, "s129.ukc.retrieved_at_utc") {
        let max_age = integer(bundle, "freshness_policy.max_ukc_plan_age_s")
            .unwrap_or(0) as i64;
        if cutoff.saturating_sub(retrieved) > max_age {
            return "UKC_PLAN_STALE_UNKNOWN";
        }
    } else {
        return "UKC_PLAN_STALE_UNKNOWN";
    }
    if text(bundle, "s129.ukc.alarm_claim_state") == Some("unsupported_alarm_claim") {
        return "UKC_ALARM_NOT_PROVIDED";
    }
    if is_proven_claim(bundle, 4)
        && matches!(
            text(bundle, "s129.ukc.physical_qualification_state"),
            Some("unknown" | "not_qualified")
        )
    {
        return "UKC_PHYSICAL_QUALIFICATION_MISSING";
    }

    // DCSA endpoint conformance is directional and distinct from ETA calibration.
    if text(bundle, "dcsa.api_direction") != text(bundle, "dcsa.tested_api_direction") {
        return "DCSA_DIRECTIONAL_CONFORMANCE_UNPROVEN";
    }
    if is_proven_claim(bundle, 6)
        && text(bundle, "dcsa.eta_accuracy_state")
            != Some("calibrated_in_declared_population")
    {
        return "ETA_CALIBRATION_NOT_ESTABLISHED";
    }
    if let (Some(event), Some(retrieved)) = (
        time_at(bundle, "dcsa.event_time_utc"),
        time_at(bundle, "dcsa.retrieved_at_utc"),
    ) {
        if event > retrieved || event > cutoff {
            return "DCSA_TEMPORAL_ORDER_INVALID";
        }
        let max_age = integer(bundle, "freshness_policy.max_dcsa_event_age_s")
            .unwrap_or(0) as i64;
        if cutoff.saturating_sub(event) > max_age {
            return "DCSA_EVENT_STALE_UNKNOWN";
        }
    } else {
        return "DCSA_TEMPORAL_ORDER_INVALID";
    }
    if text(bundle, "dcsa.semantic_validation_state") == Some("invalid")
        && is_proven_claim(bundle, 6)
    {
        return "DCSA_EVENT_SEMANTICS_INVALID";
    }

    // Provider state and authority remain with the provider / Mycelix owner.
    if text(bundle, "provider.revocation_state") == Some("revoked")
        || text(bundle, "provider.status") == Some("revoked")
    {
        return "PROVIDER_COMMITMENT_REVOKED";
    }
    if let Some(expiry) = time_at(bundle, "provider.valid_until_utc") {
        if expiry <= cutoff && is_proven_claim(bundle, 7) {
            return "PROVIDER_COMMITMENT_EXPIRED";
        }
    }
    if text(bundle, "provider.status") != Some("committed") && is_proven_claim(bundle, 7) {
        return "PROVIDER_COMMITMENT_NOT_CONFIRMED";
    }
    if text(bundle, "provider.booking_authority_state") == Some("granted_by_owner")
        && is_null(bundle, "provider.booking_authority_evidence_ref")
    {
        return "BOOKING_AUTHORITY_OWNER_BOUNDARY";
    }

    // A replay with synthetic-only physics is not physical qualification.
    if is_proven_claim(bundle, 8)
        && text(bundle, "run_evidence.supported_physics_state") == Some("synthetic_only")
    {
        return "SYNTHETIC_RUN_NOT_PHYSICAL_QUALIFICATION";
    }
    if is_proven_claim(bundle, 8)
        && text(bundle, "run_evidence.replay_state") != Some("deterministic_replay_verified")
    {
        return "RUN_REPLAY_NOT_VERIFIED";
    }

    "CONTEXT_BUNDLE_VALID_NOT_OPERATIONAL_AUTHORITY"
}

fn apply_case(base: &Value, case: &Value) -> Result<Value, String> {
    let mut bundle = base.clone();
    let mutations = at(case, "mutations")
        .and_then(Value::as_array)
        .ok_or_else(|| "case mutations must be an array".to_owned())?;

    for mutation in mutations {
        if text(mutation, "op") != Some("set") {
            return Err("only explicit set mutations are supported".to_owned());
        }
        let path = text(mutation, "path")
            .ok_or_else(|| "mutation path missing".to_owned())?;
        let value = mutation
            .get("value")
            .cloned()
            .ok_or_else(|| format!("mutation value missing at {path}"))?;
        set_path(&mut bundle, path, value)?;
    }
    Ok(bundle)
}

#[derive(Debug)]
pub struct Verification {
    pub total: usize,
    pub matched: usize,
    pub schema_cases: usize,
    pub semantic_cases: usize,
    pub mismatches: Vec<String>,
    pub corpus_sha256: String,
}

pub fn verify(schema_bytes: &[u8], corpus_bytes: &[u8]) -> Result<Verification, String> {
    let schema = parse_json(schema_bytes).map_err(|error| format!("schema JSON: {error}"))?;
    let corpus = parse_json(corpus_bytes).map_err(|error| format!("corpus JSON: {error}"))?;

    if text(&schema, "$schema") != Some("https://json-schema.org/draft/2020-12/schema") {
        return Err("MOCB schema must declare JSON Schema Draft 2020-12".to_owned());
    }
    if integer(&schema, "properties.cases.minItems") != Some(32) {
        return Err("MOCB schema must require at least 32 cases".to_owned());
    }
    if text(&corpus, "corpus_id")
        != Some("luminous.symtropy.maritime-operational-context-bundle")
        || text(&corpus, "corpus_version") != Some("0.1.0")
    {
        return Err("unexpected MOCB corpus identity or version".to_owned());
    }

    let base = at(&corpus, "base_bundle")
        .ok_or_else(|| "base_bundle missing".to_owned())?;
    if text(base, "fixture_kind") != Some("synthetic_fixture") {
        return Err("MOCB base bundle must be an explicit synthetic fixture".to_owned());
    }
    if schema_layer_rejection(base) != "SCHEMA_LAYER_VALID" {
        return Err("MOCB baseline source profile or artifact digest is invalid".to_owned());
    }

    let cases = at(&corpus, "cases")
        .and_then(Value::as_array)
        .ok_or_else(|| "MOCB cases must be an array".to_owned())?;
    if cases.len() != 32 {
        return Err(format!("expected 32 MOCB cases, found {}", cases.len()));
    }

    let mut ids = BTreeSet::new();
    let mut matched = 0;
    let mut schema_cases = 0;
    let mut semantic_cases = 0;
    let mut mismatches = Vec::new();

    for case in cases {
        let id = text(case, "id").ok_or_else(|| "MOCB case id missing".to_owned())?;
        if !ids.insert(id) {
            return Err(format!("duplicate MOCB case id: {id}"));
        }
        let expected = text(case, "expected_code")
            .ok_or_else(|| format!("expected_code missing for {id}"))?;
        let layer = text(case, "expected_layer")
            .ok_or_else(|| format!("expected_layer missing for {id}"))?;
        let bundle = match apply_case(base, case) {
            Ok(value) => value,
            Err(error) => {
                mismatches.push(format!("{id}: invalid mutation: {error}"));
                continue;
            }
        };

        let actual = match layer {
            "schema" => {
                schema_cases += 1;
                schema_layer_rejection(&bundle)
            }
            "semantic" => {
                semantic_cases += 1;
                if schema_layer_rejection(&bundle) != "SCHEMA_LAYER_VALID" {
                    "UNEXPECTED_SCHEMA_REJECTION"
                } else {
                    evaluate(&bundle)
                }
            }
            _ => {
                mismatches.push(format!("{id}: unsupported expected_layer {layer}"));
                continue;
            }
        };

        if actual == expected {
            matched += 1;
        } else {
            mismatches.push(format!("{id}: expected {expected}, got {actual}"));
        }
    }

    if schema_cases != 2 || semantic_cases != 30 {
        mismatches.push(format!(
            "expected two schema and thirty semantic cases, got {schema_cases} schema / {semantic_cases} semantic"
        ));
    }

    Ok(Verification {
        total: cases.len(),
        matched,
        schema_cases,
        semantic_cases,
        mismatches,
        corpus_sha256: format!("{:x}", Sha256::digest(corpus_bytes)),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicate_json_object_keys_are_rejected() {
        assert!(parse_json(br#"{"x":1,"x":2}"#).is_err());
    }

    #[test]
    fn strict_json_accepts_nested_unique_objects() {
        assert!(parse_json(br#"{"a":[{"b":1},{"c":2}]}"#).is_ok());
    }

    #[test]
    fn utc_parser_rejects_impossible_calendar_dates() {
        assert!(utc_seconds("2026-02-30T00:00:00Z").is_none());
        assert!(utc_seconds("2024-02-29T00:00:00Z").is_some());
        assert!(utc_seconds("2025-02-29T00:00:00Z").is_none());
    }

    #[test]
    fn valid_base_bundle_does_not_promote_operational_authority() {
        let base: Value = serde_json::from_str(
            include_str!("../../../../benchmarks/maritime/operational-context-bundle-corpus-v0.1.json")
        ).expect("corpus JSON");
        let bundle = at(&base, "base_bundle").expect("base bundle");
        assert_eq!(evaluate(bundle), "CONTEXT_BUNDLE_VALID_NOT_OPERATIONAL_AUTHORITY");
    }

    #[test]
    fn all_frozen_mocb_cases_match_the_independent_oracle() {
        let schema = include_bytes!("../../../../docs/tech/MARITIME_OPERATIONAL_CONTEXT_BUNDLE_V0_1.schema.json");
        let corpus = include_bytes!("../../../../benchmarks/maritime/operational-context-bundle-corpus-v0.1.json");
        let result = verify(schema, corpus).expect("verify MOCB corpus");
        assert_eq!(result.total, 32);
        assert_eq!(result.matched, 32, "{:?}", result.mismatches);
        assert_eq!(result.schema_cases, 2);
        assert_eq!(result.semantic_cases, 30);
        assert!(result.mismatches.is_empty());
    }
}
