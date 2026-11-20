use chrono::{DateTime, FixedOffset};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::process::ExitCode;

const ENGINE_VERSION: &str = "marine-telemetry-evidence/0.1.0";
const MAX_SOURCES_FOR_INDEPENDENCE: usize = 16;

fn string_at<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn number_at(value: &Value, key: &str) -> Option<f64> {
    value.get(key).and_then(Value::as_f64).filter(|number| number.is_finite())
}

fn u64_at(value: &Value, key: &str) -> Option<u64> {
    value.get(key).and_then(Value::as_u64)
}

fn valid_digest(value: Option<&str>) -> bool {
    value.is_some_and(|digest| {
        digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
    })
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>()
}

fn canonical_digest(value: &Value) -> Option<String> {
    let bytes = serde_json::to_vec(value).ok()?;
    Some(sha256_hex(&bytes))
}

fn digest_without_field(value: &Value, field: &str) -> Option<String> {
    let mut canonical = value.clone();
    canonical.as_object_mut()?.remove(field)?;
    canonical_digest(&canonical)
}

fn parse_time(value: Option<&str>) -> Option<DateTime<FixedOffset>> {
    value.and_then(|raw| DateTime::parse_from_rfc3339(raw).ok())
}

fn seconds_between(later: DateTime<FixedOffset>, earlier: DateTime<FixedOffset>) -> f64 {
    later.signed_duration_since(earlier).num_milliseconds() as f64 / 1_000.0
}

fn decode_hex(value: &str) -> Option<Vec<u8>> {
    hex::decode(value).ok()
}

fn max_independent_source_count(failure_domains: &[Vec<String>]) -> usize {
    fn visit(
        index: usize,
        failure_domains: &[Vec<String>],
        used: &mut BTreeSet<String>,
    ) -> usize {
        if index == failure_domains.len() {
            return 0;
        }
        let skip = visit(index + 1, failure_domains, used);
        let domains = &failure_domains[index];
        if domains.iter().any(|domain| used.contains(domain)) {
            return skip;
        }
        for domain in domains {
            used.insert(domain.clone());
        }
        let take = 1 + visit(index + 1, failure_domains, used);
        for domain in domains {
            used.remove(domain);
        }
        skip.max(take)
    }

    if failure_domains.is_empty() || failure_domains.len() > MAX_SOURCES_FOR_INDEPENDENCE {
        return 0;
    }
    visit(0, failure_domains, &mut BTreeSet::new())
}

#[derive(Clone)]
struct SourceRecord {
    gateway_id: String,
    transport: String,
    trust_status: String,
    identity_status: String,
    firmware_sha256: String,
    decoder_id: String,
    decoder_version: String,
    decoder_sha256: String,
    active_boot_epoch_id: String,
    active_boot_epoch_ordinal: u64,
    failure_domains: Vec<String>,
}

#[derive(Clone)]
struct SignalRecord {
    unit: String,
    minimum_value: f64,
    maximum_value: f64,
    max_observation_age_seconds: i64,
    max_future_clock_skew_seconds: f64,
    max_clock_uncertainty_seconds: f64,
    max_measurement_uncertainty: f64,
    max_source_disagreement: f64,
    minimum_independent_sources: usize,
    minimum_coverage_fraction: f64,
}

#[derive(Clone)]
struct QualifiedObservation {
    sample_id: String,
    value: f64,
    uncertainty: f64,
    age_seconds: f64,
    source_id: String,
    failure_domains: Vec<String>,
}

#[derive(Clone)]
struct SourceCursor {
    boot_epoch_id: String,
    boot_epoch_ordinal: u64,
    source_sequence: u64,
}

fn build_source_registry(
    value: &Value,
    errors: &mut Vec<String>,
) -> BTreeMap<String, SourceRecord> {
    let mut sources = BTreeMap::<String, SourceRecord>::new();
    let Some(items) = value.as_array() else {
        errors.push("source_registry must be an array".to_owned());
        return sources;
    };
    for (index, item) in items.iter().enumerate() {
        let source_id = string_at(item, "source_id").unwrap_or("").to_owned();
        let failure_domains = item.get("failure_domains")
            .and_then(Value::as_array)
            .map(|items| items.iter().filter_map(Value::as_str).map(str::to_owned).collect::<Vec<_>>())
            .unwrap_or_default();
        let mut unique_domains = BTreeSet::<String>::new();
        if source_id.is_empty() || sources.contains_key(&source_id) {
            errors.push(format!("source_registry[{index}] has a missing or duplicate source_id"));
            continue;
        }
        if failure_domains.is_empty()
            || failure_domains.iter().any(|domain| domain.is_empty() || !unique_domains.insert(domain.clone()))
        {
            errors.push(format!("source '{source_id}' has missing or duplicated failure-domain IDs"));
        }
        for field in ["identity_evidence_sha256", "firmware_sha256", "decoder_sha256"] {
            if !valid_digest(string_at(item, field)) {
                errors.push(format!("source '{source_id}' has invalid {field}"));
            }
        }
        if string_at(item, "gateway_id").is_none_or(str::is_empty)
            || string_at(item, "device_identity_ref").is_none_or(str::is_empty)
            || string_at(item, "decoder_id").is_none_or(str::is_empty)
            || string_at(item, "decoder_version").is_none_or(str::is_empty)
        {
            errors.push(format!("source '{source_id}' lacks gateway, device or decoder identity"));
        }
        let record = SourceRecord {
            gateway_id: string_at(item, "gateway_id").unwrap_or("").to_owned(),
            transport: string_at(item, "transport").unwrap_or("").to_owned(),
            trust_status: string_at(item, "trust_status").unwrap_or("").to_owned(),
            identity_status: string_at(item, "identity_status").unwrap_or("").to_owned(),
            firmware_sha256: string_at(item, "firmware_sha256").unwrap_or("").to_owned(),
            decoder_id: string_at(item, "decoder_id").unwrap_or("").to_owned(),
            decoder_version: string_at(item, "decoder_version").unwrap_or("").to_owned(),
            decoder_sha256: string_at(item, "decoder_sha256").unwrap_or("").to_owned(),
            active_boot_epoch_id: string_at(item, "active_boot_epoch_id").unwrap_or("").to_owned(),
            active_boot_epoch_ordinal: u64_at(item, "active_boot_epoch_ordinal").unwrap_or(0),
            failure_domains,
        };
        sources.insert(source_id, record);
    }
    sources
}

fn build_signal_contracts(
    value: &Value,
    errors: &mut Vec<String>,
) -> BTreeMap<String, SignalRecord> {
    let mut contracts = BTreeMap::<String, SignalRecord>::new();
    let Some(items) = value.as_array() else {
        errors.push("signal_contracts must be an array".to_owned());
        return contracts;
    };
    for (index, item) in items.iter().enumerate() {
        let signal_id = string_at(item, "signal_id").unwrap_or("").to_owned();
        let minimum_value = number_at(item, "minimum_value");
        let maximum_value = number_at(item, "maximum_value");
        let max_age = item.get("max_observation_age_seconds").and_then(Value::as_i64);
        let min_sources = u64_at(item, "minimum_independent_sources");
        if signal_id.is_empty() || contracts.contains_key(&signal_id) {
            errors.push(format!("signal_contracts[{index}] has a missing or duplicate signal_id"));
            continue;
        }
        let (Some(minimum_value), Some(maximum_value), Some(max_age), Some(min_sources)) =
            (minimum_value, maximum_value, max_age, min_sources) else {
                errors.push(format!("signal '{signal_id}' has invalid range, freshness or quorum values"));
                continue;
            };
        if minimum_value > maximum_value || max_age < 0
            || min_sources == 0 || min_sources as usize > MAX_SOURCES_FOR_INDEPENDENCE
        {
            errors.push(format!("signal '{signal_id}' has invalid range, age or quorum bounds"));
            continue;
        }
        let numeric_fields = [
            "max_future_clock_skew_seconds",
            "max_clock_uncertainty_seconds",
            "max_measurement_uncertainty",
            "max_source_disagreement",
            "minimum_coverage_fraction",
        ];
        if numeric_fields.iter().any(|field| number_at(item, field).is_none()) {
            errors.push(format!("signal '{signal_id}' has non-finite numeric contract values"));
            continue;
        }
        if string_at(item, "unit").is_none_or(str::is_empty) {
            errors.push(format!("signal '{signal_id}' has no physical unit"));
            continue;
        }
        let contract = SignalRecord {
            unit: string_at(item, "unit").unwrap_or("").to_owned(),
            minimum_value,
            maximum_value,
            max_observation_age_seconds: max_age,
            max_future_clock_skew_seconds: number_at(item, "max_future_clock_skew_seconds").unwrap_or(0.0),
            max_clock_uncertainty_seconds: number_at(item, "max_clock_uncertainty_seconds").unwrap_or(0.0),
            max_measurement_uncertainty: number_at(item, "max_measurement_uncertainty").unwrap_or(0.0),
            max_source_disagreement: number_at(item, "max_source_disagreement").unwrap_or(0.0),
            minimum_independent_sources: min_sources as usize,
            minimum_coverage_fraction: number_at(item, "minimum_coverage_fraction").unwrap_or(0.0),
        };
        contracts.insert(signal_id, contract);
    }
    contracts
}

fn validate(raw: &[u8], value: &Value) -> Value {
    let mut errors = Vec::<String>::new();
    let mut observation_issues = Vec::<Value>::new();
    if string_at(value, "schema_version") != Some("0.1.0") {
        errors.push("schema_version must be 0.1.0".to_owned());
    }
    if string_at(value, "scorecard_id").is_none_or(str::is_empty) {
        errors.push("scorecard_id is required".to_owned());
    }
    let status = string_at(value, "record_status").unwrap_or("");
    if !["synthetic", "shadow_replay", "measured_campaign", "published_reproduction"].contains(&status) {
        errors.push("record_status is missing or unsupported".to_owned());
    }
    let Some(decision_cutoff) = parse_time(string_at(value, "decision_cutoff")) else {
        errors.push("decision_cutoff must be RFC3339".to_owned());
        return json!({"engine_version":ENGINE_VERSION,"disposition":"INVALID","findings":{"errors":errors}});
    };

    let signal_contract_value = value.get("signal_contracts").unwrap_or(&Value::Null);
    let source_registry_value = value.get("source_registry").unwrap_or(&Value::Null);
    let observations_value = value.get("observations").unwrap_or(&Value::Null);
    for (field, canonical) in [
        ("signal_contracts_sha256", canonical_digest(signal_contract_value)),
        ("source_registry_sha256", canonical_digest(source_registry_value)),
        ("observations_sha256", canonical_digest(observations_value)),
        ("batch_sha256", digest_without_field(value, "batch_sha256")),
    ] {
        match canonical {
            Some(calculated) if string_at(value, field) == Some(calculated.as_str()) => {}
            Some(_) => errors.push(format!("{field} does not match its canonical payload")),
            None => errors.push(format!("{field} payload cannot be canonically serialized")),
        }
    }

    let sources = build_source_registry(source_registry_value, &mut errors);
    let contracts = build_signal_contracts(signal_contract_value, &mut errors);
    let mut cursors = BTreeMap::<String, SourceCursor>::new();
    let mut seen_sample_ids = BTreeSet::<String>::new();
    let mut seen_ingest_indices = BTreeSet::<u64>::new();
    let mut previous_ingest_index = 0_u64;
    let mut latest_by_signal = BTreeMap::<String, BTreeMap<String, QualifiedObservation>>::new();
    let mut signal_issue_reasons = BTreeMap::<String, Vec<String>>::new();

    let Some(observations) = observations_value.as_array() else {
        errors.push("observations must be an array".to_owned());
        return json!({"engine_version":ENGINE_VERSION,"disposition":"INVALID","findings":{"errors":errors}});
    };
    for (index, observation) in observations.iter().enumerate() {
        let sample_id = string_at(observation, "sample_id").unwrap_or("").to_owned();
        let source_id = string_at(observation, "source_id").unwrap_or("").to_owned();
        let signal_id = string_at(observation, "signal_id").unwrap_or("").to_owned();
        if sample_id.is_empty() || !seen_sample_ids.insert(sample_id.clone()) {
            errors.push(format!("observations[{index}] has a missing or duplicate sample_id"));
            continue;
        }
        match digest_without_field(observation, "observation_sha256") {
            Some(calculated) if string_at(observation, "observation_sha256") == Some(calculated.as_str()) => {}
            _ => errors.push(format!("sample '{sample_id}' observation_sha256 does not bind its canonical payload")),
        }
        let ingest_index = u64_at(observation, "gateway_ingest_index");
        match ingest_index {
            Some(index_value) if index_value > previous_ingest_index && seen_ingest_indices.insert(index_value) => {
                previous_ingest_index = index_value;
            }
            _ => errors.push(format!("sample '{sample_id}' has duplicate, missing or out-of-order gateway_ingest_index")),
        }
        let Some(source) = sources.get(&source_id) else {
            observation_issues.push(json!({"sample_id":sample_id,"signal_id":signal_id,"disposition":"UNTRUSTED","reason":"source is absent from the trusted source registry"}));
            signal_issue_reasons.entry(signal_id).or_default().push("unregistered source".to_owned());
            continue;
        };
        let Some(contract) = contracts.get(&signal_id) else {
            errors.push(format!("sample '{sample_id}' references unknown signal '{signal_id}'"));
            continue;
        };

        let raw_hex = string_at(observation, "raw_frame_hex").unwrap_or("");
        match decode_hex(raw_hex) {
            Some(bytes) if string_at(observation, "raw_frame_sha256") == Some(sha256_hex(&bytes).as_str()) => {}
            _ => errors.push(format!("sample '{sample_id}' raw_frame_sha256 does not match raw_frame_hex")),
        }
        for field in ["raw_frame_sha256","firmware_sha256","decoder_sha256",
                      "time_sync_evidence_sha256","calibration_evidence_sha256",
                      "boot_epoch_transition_sha256"] {
            if !valid_digest(string_at(observation,field)) {
                errors.push(format!("sample '{sample_id}' has invalid {field}"));
            }
        }

        let trust_mismatch = source.trust_status != "qualified"
            || source.identity_status != "verified"
            || string_at(observation,"transport") != Some(source.transport.as_str())
            || string_at(observation,"gateway_id") != Some(source.gateway_id.as_str())
            || string_at(observation,"firmware_sha256") != Some(source.firmware_sha256.as_str())
            || string_at(observation,"decoder_id") != Some(source.decoder_id.as_str())
            || string_at(observation,"decoder_version") != Some(source.decoder_version.as_str())
            || string_at(observation,"decoder_sha256") != Some(source.decoder_sha256.as_str());
        if trust_mismatch {
            observation_issues.push(json!({"sample_id":sample_id,"signal_id":signal_id,"disposition":"UNTRUSTED","reason":"source trust, identity, gateway, firmware, transport or decoder does not match registry"}));
            signal_issue_reasons.entry(signal_id).or_default().push("untrusted or mismatched source".to_owned());
            continue;
        }

        let Some(observed_at) = parse_time(string_at(observation,"observed_at")) else {
            errors.push(format!("sample '{sample_id}' observed_at must be RFC3339"));
            continue;
        };
        let Some(received_at) = parse_time(string_at(observation,"received_at")) else {
            errors.push(format!("sample '{sample_id}' received_at must be RFC3339"));
            continue;
        };
        if received_at > decision_cutoff {
            errors.push(format!("sample '{sample_id}' was received after the decision cutoff"));
            continue;
        }
        let cursor = SourceCursor {
            boot_epoch_id: string_at(observation,"boot_epoch_id").unwrap_or("").to_owned(),
            boot_epoch_ordinal: u64_at(observation,"boot_epoch_ordinal").unwrap_or(0),
            source_sequence: u64_at(observation,"source_sequence").unwrap_or(0),
        };
        if cursor.boot_epoch_id.is_empty() || cursor.source_sequence == 0 {
            errors.push(format!("sample '{sample_id}' has missing boot epoch or source sequence"));
            continue;
        }
        if let Some(previous) = cursors.get(&source_id) {
            if cursor.boot_epoch_id == previous.boot_epoch_id {
                if cursor.boot_epoch_ordinal != previous.boot_epoch_ordinal
                    || cursor.source_sequence <= previous.source_sequence
                {
                    errors.push(format!("sample '{sample_id}' is a replay or non-monotonic sequence within a boot epoch"));
                    continue;
                }
            } else if cursor.boot_epoch_ordinal <= previous.boot_epoch_ordinal
                || string_at(observation,"boot_epoch_transition_ref").is_none_or(str::is_empty)
                || !valid_digest(string_at(observation,"boot_epoch_transition_sha256"))
            {
                errors.push(format!("sample '{sample_id}' boot epoch changed without increasing ordinal and transition receipt"));
                continue;
            }
        } else if cursor.boot_epoch_id != source.active_boot_epoch_id
            || cursor.boot_epoch_ordinal != source.active_boot_epoch_ordinal
        {
            if cursor.boot_epoch_ordinal <= source.active_boot_epoch_ordinal
                || string_at(observation,"boot_epoch_transition_ref").is_none_or(str::is_empty)
                || !valid_digest(string_at(observation,"boot_epoch_transition_sha256"))
            {
                errors.push(format!("sample '{sample_id}' is outside the registered boot epoch without a transition receipt"));
                continue;
            }
        }
        cursors.insert(source_id.clone(),cursor);

        let source_future_skew = seconds_between(observed_at,received_at);
        if source_future_skew > contract.max_future_clock_skew_seconds {
            observation_issues.push(json!({"sample_id":sample_id,"signal_id":signal_id,"disposition":"UNTRUSTED","reason":"source timestamp exceeds permitted future clock skew"}));
            signal_issue_reasons.entry(signal_id).or_default().push("future clock skew exceeded".to_owned());
            continue;
        }
        let age_seconds = seconds_between(decision_cutoff,observed_at).max(0.0);
        if age_seconds > contract.max_observation_age_seconds as f64 {
            observation_issues.push(json!({"sample_id":sample_id,"signal_id":signal_id,"disposition":"STALE","age_seconds":age_seconds,"reason":"observation exceeds signal freshness contract"}));
            signal_issue_reasons.entry(signal_id).or_default().push("stale observation".to_owned());
            continue;
        }
        if string_at(observation,"clock_quality") == Some("unknown")
            || !["synchronized","holdover"].contains(&string_at(observation,"clock_quality").unwrap_or(""))
            || number_at(observation,"clock_uncertainty_seconds").is_none_or(|value| value > contract.max_clock_uncertainty_seconds)
            || number_at(observation,"clock_offset_seconds").is_none_or(|value| value.abs() > contract.max_future_clock_skew_seconds)
        {
            observation_issues.push(json!({"sample_id":sample_id,"signal_id":signal_id,"disposition":"DEGRADED","reason":"clock synchronization quality or uncertainty does not satisfy contract"}));
            signal_issue_reasons.entry(signal_id).or_default().push("clock quality degraded".to_owned());
            continue;
        }

        let (Some(calibration_from),Some(calibration_until)) = (
            parse_time(string_at(observation,"calibration_valid_from")),
            parse_time(string_at(observation,"calibration_valid_until"))
        ) else {
            errors.push(format!("sample '{sample_id}' has invalid calibration validity interval"));
            continue;
        };
        if calibration_from > observed_at || calibration_until < decision_cutoff {
            observation_issues.push(json!({"sample_id":sample_id,"signal_id":signal_id,"disposition":"DEGRADED","reason":"calibration is not valid for observation and decision cutoff"}));
            signal_issue_reasons.entry(signal_id).or_default().push("calibration expired or not yet valid".to_owned());
            continue;
        }
        if string_at(observation,"quality") != Some("good")
            || number_at(observation,"uncertainty").is_none_or(|value| value > contract.max_measurement_uncertainty)
        {
            observation_issues.push(json!({"sample_id":sample_id,"signal_id":signal_id,"disposition":"DEGRADED","reason":"sensor quality or measurement uncertainty exceeds contract"}));
            signal_issue_reasons.entry(signal_id).or_default().push("measurement quality degraded".to_owned());
            continue;
        }
        let value_number = number_at(observation,"value");
        let uncertainty = number_at(observation,"uncertainty");
        if string_at(observation,"unit") != Some(contract.unit.as_str())
            || value_number.is_none_or(|v| v < contract.minimum_value || v > contract.maximum_value)
        {
            errors.push(format!("sample '{sample_id}' has wrong signal unit or value is outside contract range"));
            continue;
        }
        let value_number = value_number.unwrap();
        let uncertainty = uncertainty.unwrap_or(f64::INFINITY);
        latest_by_signal.entry(signal_id.clone()).or_default().insert(source_id.clone(),QualifiedObservation {
            sample_id:sample_id.clone(),
            value:value_number,
            uncertainty,
            age_seconds,
            source_id:source_id.clone(),
            failure_domains:source.failure_domains.clone()
        });
        observation_issues.push(json!({"sample_id":sample_id,"signal_id":signal_id,"disposition":"QUALIFIED","age_seconds":age_seconds}));
    }

    let mut signal_reports = Vec::<Value>::new();
    let mut any_conflict = false;
    let mut any_degraded = false;
    let mut qualified_signal_count = 0_usize;
    for (signal_id,contract) in &contracts {
        let values = latest_by_signal.get(signal_id);
        let accepted = values.map(|map| map.values().cloned().collect::<Vec<_>>()).unwrap_or_default();
        let mut unique_values = accepted.iter().map(|item| item.value).collect::<Vec<_>>();
        unique_values.sort_by(f64::total_cmp);
        let failure_domains = accepted.iter().map(|item| item.failure_domains.clone()).collect::<Vec<_>>();
        let independent_sources = max_independent_source_count(&failure_domains);
        let disagreement = match (unique_values.first(),unique_values.last()) {
            (Some(minimum),Some(maximum)) => maximum-minimum,
            _ => 0.0,
        };
        let envelope_lower = accepted.iter().map(|item| item.value-item.uncertainty).fold(f64::INFINITY,f64::min);
        let envelope_upper = accepted.iter().map(|item| item.value+item.uncertainty).fold(f64::NEG_INFINITY,f64::max);
        let issues = signal_issue_reasons.get(signal_id).cloned().unwrap_or_default();
        let conflict = accepted.len() >= 2 && disagreement > contract.max_source_disagreement;
        let enough_independent = independent_sources >= contract.minimum_independent_sources;
        let coverage = if contract.minimum_independent_sources == 0 {0.0} else {
            (independent_sources as f64 / contract.minimum_independent_sources as f64).min(1.0)
        };
        let (disposition,usable) = if conflict {
            any_conflict = true;
            ("CONFLICT",false)
        } else if accepted.is_empty() {
            any_degraded = true;
            ("UNKNOWN",false)
        } else if !enough_independent || !issues.is_empty() || coverage < contract.minimum_coverage_fraction {
            any_degraded = true;
            ("DEGRADED",false)
        } else {
            qualified_signal_count += 1;
            ("QUALIFIED",true)
        };
        signal_reports.push(json!({
            "signal_id":signal_id,
            "disposition":disposition,
            "usable":usable,
            "accepted_source_count":accepted.len(),
            "independent_source_count":independent_sources,
            "required_independent_source_count":contract.minimum_independent_sources,
            "coverage_fraction":coverage,
            "source_disagreement":disagreement,
            "maximum_allowed_disagreement":contract.max_source_disagreement,
            "qualified_envelope_lower":if envelope_lower.is_finite(){json!(envelope_lower)}else{Value::Null},
            "qualified_envelope_upper":if envelope_upper.is_finite(){json!(envelope_upper)}else{Value::Null},
            "maximum_observation_age_seconds":accepted.iter().map(|item|item.age_seconds).fold(0.0,f64::max),
            "contributing_samples":accepted.iter().map(|item|json!({
                "sample_id":item.sample_id,"source_id":item.source_id,"value":item.value,
                "uncertainty":item.uncertainty,"failure_domains":item.failure_domains
            })).collect::<Vec<_>>(),
            "issues":issues
        }));
    }

    let fatal = !errors.is_empty();
    let disposition = if fatal {
        "INVALID"
    } else if any_conflict {
        "CONFLICT/ABSTAIN"
    } else if any_degraded || qualified_signal_count != contracts.len() {
        "DEGRADED/ABSTAIN"
    } else if status == "synthetic" {
        "SYNTHETIC_ONLY"
    } else {
        "QUALIFIED_READ_ONLY"
    };
    json!({
        "engine_version":ENGINE_VERSION,
        "scorecard_id":string_at(value,"scorecard_id"),
        "disposition":disposition,
        "read_only":true,
        "actuation_authorized":false,
        "input_sha256":sha256_hex(raw),
        "signal_count":contracts.len(),
        "qualified_signal_count":qualified_signal_count,
        "signals":signal_reports,
        "observation_dispositions":observation_issues,
        "findings":{"errors":errors},
        "claim_ceiling":"evidence_qualification_and_freshness_audit_only_not_sensor_authenticity_not_maritime_standard_conformance_not_navigation_authority",
        "note":"A digest binds bytes but does not authenticate a device or publisher. Identity status, calibration authority and decoder qualification must be independently provisioned. Conflicting or insufficiently independent observations are not averaged or used as mission state."
    })
}

fn run() -> Result<(),String> {
    let args=env::args().collect::<Vec<_>>();
    if args.len()!=3 || args[1]!="audit-telemetry" {
        return Err("Usage: marine-telemetry-evidence audit-telemetry <batch.json>".to_owned());
    }
    let raw=fs::read(&args[2]).map_err(|error|format!("{}: {error}",args[2]))?;
    let value:Value=serde_json::from_slice(&raw).map_err(|error|format!("{}: invalid JSON: {error}",args[2]))?;
    let report=validate(&raw,&value);
    println!("{}",serde_json::to_string_pretty(&report).map_err(|error|error.to_string())?);
    match string_at(&report,"disposition") {
        Some("SYNTHETIC_ONLY"|"QUALIFIED_READ_ONLY"|"DEGRADED/ABSTAIN"|"CONFLICT/ABSTAIN")=>Ok(()),
        Some(disposition)=>Err(format!("telemetry audit disposition: {disposition}")),
        None=>Err("telemetry report has no disposition".to_owned())
    }
}

fn main()->ExitCode {
    match run() {
        Ok(())=>ExitCode::SUCCESS,
        Err(error)=>{eprintln!("{error}");ExitCode::from(2)}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Value {
        serde_json::from_str(include_str!("../fixtures/synthetic-qualified-telemetry-0001.json"))
            .expect("synthetic qualified telemetry fixture parses")
    }

    fn common_cause_fixture() -> Value {
        serde_json::from_str(include_str!("../fixtures/synthetic-common-cause-not-independent-0001.json"))
            .expect("synthetic common-cause fixture parses")
    }

    fn reseal(value: &mut Value) {
        let contracts = value["signal_contracts"].clone();
        let sources = value["source_registry"].clone();
        value["signal_contracts_sha256"] = json!(canonical_digest(&contracts).unwrap());
        value["source_registry_sha256"] = json!(canonical_digest(&sources).unwrap());
        let observations = value["observations"].as_array_mut().unwrap();
        for observation in observations {
            observation["observation_sha256"] = Value::Null;
            let digest = digest_without_field(observation,"observation_sha256").unwrap();
            observation["observation_sha256"] = json!(digest);
        }
        let observations_copy=value["observations"].clone();
        value["observations_sha256"]=json!(canonical_digest(&observations_copy).unwrap());
        value["batch_sha256"]=Value::Null;
        value["batch_sha256"]=json!(digest_without_field(value,"batch_sha256").unwrap());
    }

    fn audit(value: &Value) -> Value {
        let raw=serde_json::to_vec(value).expect("batch serializes");
        validate(&raw,value)
    }

    #[test]
    fn independent_fresh_calibrated_sources_qualify_synthetic_signals() {
        let report=audit(&fixture());
        assert_eq!(report["disposition"],"SYNTHETIC_ONLY");
        assert_eq!(report["qualified_signal_count"],2);
        assert_eq!(report["signals"][0]["usable"],true);
        assert_eq!(report["read_only"],true);
        assert_eq!(report["actuation_authorized"],false);
    }

    #[test]
    fn shared_gateway_and_network_do_not_count_as_independent_sensors() {
        let report=audit(&common_cause_fixture());
        assert_eq!(report["disposition"],"DEGRADED/ABSTAIN");
        let tank=report["signals"].as_array().unwrap().iter()
            .find(|signal|signal["signal_id"]=="freshwater_tank_level").unwrap();
        assert_eq!(tank["independent_source_count"],1);
        assert_eq!(tank["usable"],false);
    }

    #[test]
    fn stale_observation_is_not_used_for_signal_state() {
        let mut value=fixture();
        value["decision_cutoff"]=json!("2026-11-20T06:00:00Z");
        reseal(&mut value);
        assert_eq!(audit(&value)["disposition"],"DEGRADED/ABSTAIN");
    }

    #[test]
    fn replayed_source_sequence_is_invalid() {
        let mut value=fixture();
        value["observations"][1]["source_id"]=json!("tank-level-a");
        value["observations"][1]["boot_epoch_id"]=json!("boot-a-001");
        value["observations"][1]["boot_epoch_ordinal"]=json!(1);
        value["observations"][1]["source_sequence"]=json!(1001);
        value["observations"][1]["gateway_id"]=json!("gateway-a");
        value["observations"][1]["transport"]=json!("nmea2000_can");
        value["observations"][1]["firmware_sha256"]=value["source_registry"][0]["firmware_sha256"].clone();
        value["observations"][1]["decoder_id"]=value["source_registry"][0]["decoder_id"].clone();
        value["observations"][1]["decoder_version"]=value["source_registry"][0]["decoder_version"].clone();
        value["observations"][1]["decoder_sha256"]=value["source_registry"][0]["decoder_sha256"].clone();
        reseal(&mut value);
        assert_eq!(audit(&value)["disposition"],"INVALID");
    }

    #[test]
    fn expired_calibration_degrades_the_signal() {
        let mut value=fixture();
        value["observations"][0]["calibration_valid_until"]=json!("2026-11-19T00:00:00Z");
        reseal(&mut value);
        assert_eq!(audit(&value)["disposition"],"DEGRADED/ABSTAIN");
    }

    #[test]
    fn untrusted_firmware_or_decoder_identity_is_not_accepted() {
        let mut value=fixture();
        value["observations"][0]["firmware_sha256"]=json!("f".repeat(64));
        reseal(&mut value);
        assert_eq!(audit(&value)["disposition"],"DEGRADED/ABSTAIN");
    }

    #[test]
    fn conflicting_qualified_sensor_values_abstain_without_averaging() {
        let mut value=fixture();
        value["observations"][1]["value"]=json!(180.0);
        reseal(&mut value);
        assert_eq!(audit(&value)["disposition"],"CONFLICT/ABSTAIN");
        let tank=audit(&value)["signals"].as_array().unwrap().iter()
            .find(|signal|signal["signal_id"]=="freshwater_tank_level").unwrap().clone();
        assert_eq!(tank["usable"],false);
    }

    #[test]
    fn wrong_units_are_invalid_not_silently_converted() {
        let mut value=fixture();
        value["observations"][0]["unit"]=json!("gal");
        reseal(&mut value);
        assert_eq!(audit(&value)["disposition"],"INVALID");
    }

    #[test]
    fn raw_frame_digest_is_recomputed_from_captured_hex_bytes() {
        let mut value=fixture();
        value["observations"][0]["raw_frame_hex"]=json!("ffffffff");
        reseal(&mut value);
        assert_eq!(audit(&value)["disposition"],"INVALID");
    }

    #[test]
    fn future_clock_skew_degrades_instead_of_rewriting_event_time() {
        let mut value=fixture();
        value["observations"][0]["observed_at"]=json!("2026-11-20T05:01:00Z");
        reseal(&mut value);
        assert_eq!(audit(&value)["disposition"],"DEGRADED/ABSTAIN");
    }

    #[test]
    fn unknown_clock_quality_is_not_treated_as_synchronized() {
        let mut value=fixture();
        value["observations"][0]["clock_quality"]=json!("unknown");
        reseal(&mut value);
        assert_eq!(audit(&value)["disposition"],"DEGRADED/ABSTAIN");
    }

    #[test]
    fn boot_epoch_change_requires_a_monotonic_ordinal_and_receipt() {
        let mut value=fixture();
        value["observations"][1]["source_id"]=json!("tank-level-a");
        value["observations"][1]["boot_epoch_id"]=json!("boot-a-002");
        value["observations"][1]["boot_epoch_ordinal"]=json!(2);
        value["observations"][1]["source_sequence"]=json!(1);
        value["observations"][1]["gateway_id"]=json!("gateway-a");
        value["observations"][1]["transport"]=json!("nmea2000_can");
        value["observations"][1]["firmware_sha256"]=value["source_registry"][0]["firmware_sha256"].clone();
        value["observations"][1]["decoder_id"]=value["source_registry"][0]["decoder_id"].clone();
        value["observations"][1]["decoder_version"]=value["source_registry"][0]["decoder_version"].clone();
        value["observations"][1]["decoder_sha256"]=value["source_registry"][0]["decoder_sha256"].clone();
        value["observations"][1]["boot_epoch_transition_ref"]=json!("");
        reseal(&mut value);
        assert_eq!(audit(&value)["disposition"],"INVALID");
    }

    #[test]
    fn batch_digest_detects_unresealed_tampering() {
        let mut value=fixture();
        value["observations"][0]["value"]=json!(147.0);
        assert_eq!(audit(&value)["disposition"],"INVALID");
    }
}
