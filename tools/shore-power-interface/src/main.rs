//! Independent, fail-closed synthetic shore-to-ship interface contract verifier.
//!
//! This crate does not design electrical systems, operate breakers, energize
//! equipment, interpret relay settings, or grant connection authority.
//! It evaluates only the declared synthetic interface/evidence contract.

use serde::de::{MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use serde_json::{Map, Number, Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::env;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

struct StrictJson(Value);

impl<'de> Deserialize<'de> for StrictJson {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct StrictValueVisitor;

        impl<'de> Visitor<'de> for StrictValueVisitor {
            type Value = Value;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a JSON value with unique object keys")
            }

            fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E>
            where E: serde::de::Error,
            {
                Ok(Value::Bool(value))
            }

            fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
            where E: serde::de::Error,
            {
                Ok(Value::Number(Number::from(value)))
            }

            fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
            where E: serde::de::Error,
            {
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

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where E: serde::de::Error,
            {
                Ok(Value::String(value.to_owned()))
            }

            fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
            where E: serde::de::Error,
            {
                Ok(Value::String(value))
            }

            fn visit_unit<E>(self) -> Result<Self::Value, E>
            where E: serde::de::Error,
            {
                Ok(Value::Null)
            }

            fn visit_none<E>(self) -> Result<Self::Value, E>
            where E: serde::de::Error,
            {
                Ok(Value::Null)
            }

            fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: Deserializer<'de>,
            {
                StrictJson::deserialize(deserializer).map(|value| value.0)
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

        deserializer
            .deserialize_any(StrictValueVisitor)
            .map(StrictJson)
    }
}

fn parse_strict_json(bytes: &[u8]) -> Result<Value, serde_json::Error> {
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let value = StrictJson::deserialize(&mut deserializer)?.0;
    deserializer.end()?;
    Ok(value)
}

fn value_at<'a>(root: &'a Value, path: &str) -> Option<&'a Value> {
    let mut current = root;
    for segment in path.split('.') {
        if let Some(array) = current.as_array() {
            current = array.get(segment.parse::<usize>().ok()?)?;
        } else {
            current = current.get(segment)?;
        }
    }
    Some(current)
}

fn text_at<'a>(root: &'a Value, path: &str) -> Option<&'a str> {
    value_at(root, path).and_then(Value::as_str)
}

fn number_at(root: &Value, path: &str) -> Option<f64> {
    value_at(root, path).and_then(Value::as_f64)
}

fn bool_at(root: &Value, path: &str) -> Option<bool> {
    value_at(root, path).and_then(Value::as_bool)
}

fn is_canonical_utc(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 20
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16] != b':'
        || bytes[19] != b'Z'
    {
        return false;
    }
    let parse = |range: std::ops::Range<usize>| -> Option<u32> {
        value.get(range)?.parse().ok()
    };
    let (Some(year), Some(month), Some(day), Some(hour), Some(minute), Some(second)) = (
        parse(0..4),
        parse(5..7),
        parse(8..10),
        parse(11..13),
        parse(14..16),
        parse(17..19),
    ) else {
        return false;
    };
    if year == 0 || !(1..=12).contains(&month) || hour > 23 || minute > 59 || second > 59 {
        return false;
    }
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return false,
    };
    (1..=days).contains(&day)
}

fn date_at(root: &Value, path: &str) -> Option<&str> {
    text_at(root, path).filter(|value| is_canonical_utc(value))
}

fn same_text(left: Option<&str>, right: Option<&str>) -> bool {
    left.is_some() && left == right
}

fn evaluate(case_value: &Value) -> &'static str {
    let ship = value_at(case_value, "ship");
    let berth = value_at(case_value, "berth");
    let shore = value_at(case_value, "shore");
    let standards = value_at(case_value, "standards");
    let grid = value_at(case_value, "grid_observation");
    let commitment = value_at(case_value, "commitment");
    let protection = value_at(case_value, "protection");
    let communications = value_at(case_value, "communications");
    let power_quality = value_at(case_value, "power_quality");
    let authorities = value_at(case_value, "authorities");
    let connection = value_at(case_value, "connection");
    if [
        ship, berth, shore, standards, grid, commitment, protection, communications,
        power_quality, authorities, connection,
    ]
    .iter()
    .any(Option::is_none)
    {
        return "INTERFACE_CONTRACT_INCOMPLETE";
    }

    let cutoff = match date_at(case_value, "evaluation_cutoff_utc") {
        Some(value) => value,
        None => return "INVALID_EVALUATION_CUTOFF",
    };

    // Bind the requested vessel, berth and physical connection point first.
    if !same_text(
        text_at(case_value, "ship.ship_id"),
        text_at(case_value, "commitment.ship_id"),
    ) || !same_text(
        text_at(case_value, "berth.berth_id"),
        text_at(case_value, "commitment.berth_id"),
    ) || !same_text(
        text_at(case_value, "berth.connection_point_id"),
        text_at(case_value, "commitment.connection_point_id"),
    ) {
        return "COMMITMENT_SUBJECT_MISMATCH";
    }
    if text_at(case_value, "berth.facility_revision").is_none_or(|revision| {
        revision.eq_ignore_ascii_case("latest") || revision.trim().is_empty()
    }) {
        return "MUTABLE_FACILITY_REVISION";
    }
    if text_at(case_value, "ship.electrical_system_class")
        != text_at(case_value, "shore.connection_system_class")
    {
        return "CONNECTION_CLASS_MISMATCH";
    }

    let class = match text_at(case_value, "ship.electrical_system_class") {
        Some("LVSC") => "LVSC",
        Some("HVSC") => "HVSC",
        _ => return "CONNECTION_CLASS_MISMATCH",
    };
    let standard_id = match class {
        "LVSC" => "IEC/IEEE 80005-3:2025",
        "HVSC" => "IEC/IEEE 80005-1:2019+AMD1:2022+AMD2:2023",
        _ => unreachable!(),
    };
    if text_at(case_value, "standards.connection_standard_id") != Some(standard_id)
        || text_at(case_value, "standards.connection_standard_status") != Some("published")
    {
        if bool_at(case_value, "standards.edition3_claimed") == Some(true) {
            return "UNPUBLISHED_STANDARD_CLAIM";
        }
        return "CONNECTION_STANDARD_SCOPE_MISMATCH";
    }
    if bool_at(case_value, "standards.edition3_claimed") == Some(true) {
        return "UNPUBLISHED_STANDARD_CLAIM";
    }
    if text_at(case_value, "standards.communication_standard_id")
        != Some("IEC/IEEE 80005-2:2016")
        || text_at(case_value, "standards.communication_standard_status")
            != Some("published")
    {
        return "COMMUNICATION_PROFILE_UNAVAILABLE";
    }

    let (Some(voltage), Some(min_voltage), Some(max_voltage)) = (
        number_at(case_value, "shore.nominal_voltage_v"),
        number_at(case_value, "ship.min_voltage_v"),
        number_at(case_value, "ship.max_voltage_v"),
    ) else {
        return "VOLTAGE_INCOMPATIBLE";
    };
    if voltage < min_voltage || voltage > max_voltage {
        return "VOLTAGE_INCOMPATIBLE";
    }
    let (Some(frequency), Some(min_frequency), Some(max_frequency)) = (
        number_at(case_value, "shore.frequency_hz"),
        number_at(case_value, "ship.min_frequency_hz"),
        number_at(case_value, "ship.max_frequency_hz"),
    ) else {
        return "FREQUENCY_INCOMPATIBLE";
    };
    if frequency < min_frequency || frequency > max_frequency {
        return "FREQUENCY_INCOMPATIBLE";
    }
    if value_at(case_value, "shore.phase_count")
        != value_at(case_value, "ship.phase_count")
    {
        return "PHASE_INCOMPATIBLE";
    }
    if number_at(case_value, "shore.rated_current_a")
        .zip(number_at(case_value, "ship.required_current_a"))
        .is_none_or(|(available, required)| available < required)
    {
        return "CURRENT_CAPACITY_INSUFFICIENT";
    }
    if number_at(case_value, "shore.rated_power_mva")
        .zip(number_at(case_value, "ship.required_power_mva"))
        .is_none_or(|(available, required)| available < required)
    {
        return "SHORE_POWER_RATING_INSUFFICIENT";
    }
    if number_at(case_value, "shore.transformer_capacity_mva")
        .zip(number_at(case_value, "ship.required_power_mva"))
        .is_none_or(|(available, required)| available < required)
    {
        return "TRANSFORMER_CAPACITY_INSUFFICIENT";
    }
    if text_at(case_value, "shore.connector_family")
        != text_at(case_value, "ship.connector_family")
    {
        return "CONNECTOR_INCOMPATIBLE";
    }
    if number_at(case_value, "shore.available_cable_reach_m")
        .zip(number_at(case_value, "ship.required_cable_reach_m"))
        .is_none_or(|(available, required)| available < required)
    {
        return "CABLE_REACH_INSUFFICIENT";
    }
    if !same_text(
        text_at(case_value, "shore.installed_transformer_revision"),
        text_at(case_value, "shore.required_transformer_revision"),
    ) {
        return "TRANSFORMER_REVISION_MISMATCH";
    }
    if text_at(case_value, "shore.earthing_arrangement").is_none_or(|value| {
        value == "unknown" || value.trim().is_empty()
    }) || !same_text(
        text_at(case_value, "shore.earthing_arrangement"),
        text_at(case_value, "shore.required_earthing_arrangement"),
    ) {
        return "EARTHING_INCOMPATIBLE_OR_UNKNOWN";
    }

    // Protection and physical interlocking cannot be inferred from communications.
    if text_at(case_value, "protection.commissioning_state") != Some("commissioned")
        || text_at(case_value, "protection.protective_tests_state") != Some("current")
        || text_at(case_value, "protection.earthing_state") != Some("verified")
        || bool_at(case_value, "protection.emergency_safety_independent") != Some(true)
    {
        return "PROTECTION_NOT_COMMISSIONED";
    }
    if text_at(case_value, "protection.safety_loop_state") != Some("verified") {
        return "SAFETY_LOOP_NOT_VERIFIED";
    }
    if text_at(case_value, "protection.interlock_state") != Some("verified")
        || text_at(case_value, "protection.breaker_control_state") != Some("verified")
    {
        return "INTERLOCK_NOT_VERIFIED";
    }
    if text_at(case_value, "communications.state") != Some("available")
        || bool_at(case_value, "communications.emergency_functions_carried") != Some(false)
    {
        return "COMMUNICATION_PROFILE_UNAVAILABLE";
    }
    if text_at(case_value, "grid_observation.evidence_class") != Some("observed_synthetic")
        || text_at(case_value, "grid_observation.state") != Some("measured")
    {
        return "LIVE_EVIDENCE_CLASS_REQUIRED";
    }
    let grid_observed = match date_at(case_value, "grid_observation.observed_at_utc") {
        Some(value) => value,
        None => return "GRID_OBSERVATION_AFTER_CUTOFF",
    };
    if grid_observed > cutoff {
        return "GRID_OBSERVATION_AFTER_CUTOFF";
    }
    let grid_valid = match date_at(case_value, "grid_observation.valid_until_utc") {
        Some(value) => value,
        None => return "GRID_OBSERVATION_STALE",
    };
    if grid_valid < cutoff {
        return "GRID_OBSERVATION_STALE";
    }
    if number_at(case_value, "grid_observation.available_capacity_mva")
        .zip(number_at(case_value, "commitment.reserved_power_mva"))
        .is_none_or(|(available, reserved)| available < reserved)
    {
        return "LIVE_CAPACITY_INSUFFICIENT";
    }
    if number_at(case_value, "commitment.reserved_power_mva")
        .zip(number_at(case_value, "ship.required_power_mva"))
        .is_none_or(|(reserved, required)| reserved < required)
    {
        return "COMMITTED_POWER_INSUFFICIENT";
    }
    if text_at(case_value, "commitment.state") != Some("accepted") {
        return "COMMITMENT_NOT_ACCEPTED";
    }
    if bool_at(case_value, "commitment.revoked") != Some(false) {
        return "COMMITMENT_REVOKED";
    }
    if text_at(case_value, "commitment.provider_id").is_none_or(|value| value.trim().is_empty()) {
        return "COMMITMENT_PROVIDER_MISSING";
    }
    let window_start = match date_at(case_value, "commitment.window_start_utc") {
        Some(value) => value,
        None => return "COMMITMENT_WINDOW_EXPIRED",
    };
    let window_end = match date_at(case_value, "commitment.window_end_utc") {
        Some(value) => value,
        None => return "COMMITMENT_WINDOW_EXPIRED",
    };
    if cutoff < window_start || cutoff > window_end {
        return "COMMITMENT_WINDOW_EXPIRED";
    }

    if text_at(case_value, "power_quality.state") != Some("within_declared_limits") {
        return "POWER_QUALITY_OUTSIDE_DECLARED";
    }
    if text_at(case_value, "power_quality.metering_boundary_id")
        != text_at(case_value, "grid_observation.metering_boundary_id")
    {
        return "METERING_BOUNDARY_MISMATCH";
    }
    let pq_observed = match date_at(case_value, "power_quality.observed_at_utc") {
        Some(value) => value,
        None => return "POWER_QUALITY_STALE",
    };
    let pq_valid = match date_at(case_value, "power_quality.valid_until_utc") {
        Some(value) => value,
        None => return "POWER_QUALITY_STALE",
    };
    if pq_observed > cutoff || pq_valid < cutoff {
        return "POWER_QUALITY_STALE";
    }
    if number_at(case_value, "ship.requested_load_step_mva")
        .zip(number_at(case_value, "ship.max_load_step_mva"))
        .is_none_or(|(requested, maximum)| requested > maximum)
    {
        return "SHIPBOARD_TRANSIENT_ENVELOPE_EXCEEDED";
    }

    if text_at(case_value, "authorities.port_electrical_owner.status") != Some("approved")
        || text_at(case_value, "authorities.port_electrical_owner.scope")
            != text_at(case_value, "berth.connection_point_id")
    {
        return "PORT_AUTHORITY_NOT_APPROVED";
    }
    if text_at(case_value, "authorities.ship_electrical_authority.status") != Some("approved")
        || text_at(case_value, "authorities.ship_electrical_authority.scope")
            != text_at(case_value, "ship.ship_id")
    {
        return "SHIP_AUTHORITY_NOT_APPROVED";
    }
    if bool_at(case_value, "connection.source_writeback") != Some(false) {
        return "LIVE_STATE_WRITEBACK_FORBIDDEN";
    }
    if text_at(case_value, "connection.claimed_state") != Some("ready_for_connection") {
        return "ENERGIZATION_CONTROL_OUT_OF_SCOPE";
    }
    if text_at(case_value, "connection.evidence_class") != Some("observed_synthetic") {
        return "LIVE_EVIDENCE_CLASS_REQUIRED";
    }
    "INTERFACE_CONTRACT_VALID_SYNTHETIC_ONLY"
}

fn apply_mutation(root: &mut Value, mutation: &Value) -> Result<(), String> {
    let path = mutation
        .get("path")
        .and_then(Value::as_str)
        .ok_or_else(|| "mutation path is missing".to_owned())?;
    let replacement = mutation
        .get("value")
        .ok_or_else(|| format!("mutation {path} has no value"))?
        .clone();
    let mut segments: Vec<&str> = path.split('.').collect();
    let leaf = segments.pop().ok_or_else(|| "empty mutation path".to_owned())?;
    let mut current = root;
    for segment in segments {
        if let Some(array) = current.as_array_mut() {
            let index = segment
                .parse::<usize>()
                .map_err(|_| format!("invalid array index in {path}"))?;
            current = array
                .get_mut(index)
                .ok_or_else(|| format!("array index out of range in {path}"))?;
        } else {
            current = current
                .as_object_mut()
                .and_then(|object| object.get_mut(segment))
                .ok_or_else(|| format!("missing mutation path {path}"))?;
        }
    }
    if let Some(array) = current.as_array_mut() {
        let index = leaf
            .parse::<usize>()
            .map_err(|_| format!("invalid leaf array index in {path}"))?;
        let target = array
            .get_mut(index)
            .ok_or_else(|| format!("leaf array index out of range in {path}"))?;
        *target = replacement;
    } else {
        let target = current
            .as_object_mut()
            .and_then(|object| object.get_mut(leaf))
            .ok_or_else(|| format!("missing mutation leaf {path}"))?;
        *target = replacement;
    }
    Ok(())
}

fn verify_corpus(corpus: &Value) -> Result<Value, String> {
    if corpus.get("corpus_id").and_then(Value::as_str)
        != Some("luminous.symtropy.shore-power-interface")
        || corpus.get("corpus_version").and_then(Value::as_str) != Some("0.1.0")
    {
        return Err("unsupported shore-power corpus identity or version".to_owned());
    }
    let bases = corpus
        .get("base_cases")
        .and_then(Value::as_object)
        .ok_or_else(|| "corpus base_cases must be an object".to_owned())?;
    let cases = corpus
        .get("cases")
        .and_then(Value::as_array)
        .ok_or_else(|| "corpus cases must be an array".to_owned())?;
    if cases.len() != 42 {
        return Err(format!("frozen shore-power corpus must contain 42 cases, found {}", cases.len()));
    }
    if !bases.contains_key("valid_lvsc") || !bases.contains_key("valid_hvsc") {
        return Err("corpus must declare valid_lvsc and valid_hvsc base fixtures".to_owned());
    }

    let mut ids = BTreeSet::new();
    let mut results = Vec::with_capacity(cases.len());
    let mut failures = Vec::new();
    for case in cases {
        let id = case
            .get("id")
            .and_then(Value::as_str)
            .ok_or_else(|| "case has no id".to_owned())?;
        if !ids.insert(id) {
            return Err(format!("duplicate shore-power case id: {id}"));
        }
        let base_name = case
            .get("base")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("case {id} has no base name"))?;
        let mut input = bases
            .get(base_name)
            .ok_or_else(|| format!("case {id} references unknown base {base_name}"))?
            .clone();
        let mutations = case
            .get("mutations")
            .and_then(Value::as_array)
            .ok_or_else(|| format!("case {id} mutations must be an array"))?;
        for mutation in mutations {
            apply_mutation(&mut input, mutation)
                .map_err(|error| format!("case {id}: {error}"))?;
        }
        let expected = case
            .get("expected_code")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("case {id} has no expected_code"))?;
        let observed = evaluate(&input);
        let passed = observed == expected;
        if !passed {
            failures.push(json!({"case_id":id,"expected":expected,"observed":observed}));
        }
        results.push(json!({"case_id":id,"expected":expected,"observed":observed,"passed":passed}));
    }
    let passed_count = results.iter().filter(|item| item["passed"] == true).count();
    Ok(json!({
        "status":if failures.is_empty(){"PASS_SYNTHETIC_INTERFACE_CORPUS"}else{"FAIL_SYNTHETIC_INTERFACE_CORPUS"},
        "corpus_id":corpus["corpus_id"],
        "corpus_version":corpus["corpus_version"],
        "case_count":cases.len(),
        "passed_count":passed_count,
        "failed_count":failures.len(),
        "positive_count":cases.iter().filter(|c|c["expected_code"]=="INTERFACE_CONTRACT_VALID_SYNTHETIC_ONLY").count(),
        "negative_count":cases.len()-cases.iter().filter(|c|c["expected_code"]=="INTERFACE_CONTRACT_VALID_SYNTHETIC_ONLY").count(),
        "failures":failures,
        "results":results
    }))
}

fn main() -> ExitCode {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let path = env::args_os().nth(1).map(PathBuf::from)
        .unwrap_or_else(|| repo_root.join("benchmarks/maritime/shore-power-interface-corpus-v0.1.json"));
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("cannot read {}: {error}", path.display());
            return ExitCode::FAILURE;
        }
    };
    let corpus = match parse_strict_json(&bytes) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("invalid or ambiguous corpus JSON: {error}");
            return ExitCode::FAILURE;
        }
    };
    let mut report = match verify_corpus(&corpus) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("corpus verification could not run: {error}");
            return ExitCode::FAILURE;
        }
    };
    report["corpus_sha256"] = Value::String(format!("{:x}", Sha256::digest(&bytes)));
    println!("{}", serde_json::to_string_pretty(&report).expect("JSON report serializes"));
    if report["failed_count"].as_u64() == Some(0) { ExitCode::SUCCESS } else { ExitCode::FAILURE }
}

#[cfg(test)]
mod tests {
    use super::{evaluate, parse_strict_json, verify_corpus};
    use serde_json::Value;

    const CORPUS: &str = include_str!("../../../benchmarks/maritime/shore-power-interface-corpus-v0.1.json");

    #[test]
    fn strict_json_rejects_duplicate_keys_at_all_depths() {
        assert!(parse_strict_json(br#"{"ship_id":"a","ship_id":"b"}"#).is_err());
        assert!(parse_strict_json(br#"{"outer":{"voltage":1,"voltage":2}}"#).is_err());
        assert!(parse_strict_json(br#"{"ship_id":"a","voltage":690}"#).is_ok());
    }

    #[test]
    fn frozen_lvsc_and_hvsc_bases_are_contract_valid_only() {
        let corpus: Value = parse_strict_json(CORPUS.as_bytes()).expect("strict corpus parse");
        assert_eq!(evaluate(&corpus["base_cases"]["valid_lvsc"]), "INTERFACE_CONTRACT_VALID_SYNTHETIC_ONLY");
        assert_eq!(evaluate(&corpus["base_cases"]["valid_hvsc"]), "INTERFACE_CONTRACT_VALID_SYNTHETIC_ONLY");
    }

    #[test]
    fn all_frozen_cases_match_independent_dispositions() {
        let corpus: Value = parse_strict_json(CORPUS.as_bytes()).expect("strict corpus parse");
        let report = verify_corpus(&corpus).expect("corpus verifies");
        assert_eq!(report["case_count"].as_u64(), Some(42));
        assert_eq!(report["positive_count"].as_u64(), Some(2));
        assert_eq!(report["negative_count"].as_u64(), Some(40));
        assert_eq!(report["passed_count"].as_u64(), Some(42));
        assert_eq!(report["failed_count"].as_u64(), Some(0));
    }

    #[test]
    fn mutable_or_wrong_standard_claims_never_qualify() {
        let mut corpus: Value = parse_strict_json(CORPUS.as_bytes()).expect("strict corpus parse");
        let case = corpus["base_cases"]["valid_hvsc"].clone();
        let mut changed = case;
        changed["standards"]["edition3_claimed"] = Value::Bool(true);
        assert_eq!(evaluate(&changed), "UNPUBLISHED_STANDARD_CLAIM");
    }
}
