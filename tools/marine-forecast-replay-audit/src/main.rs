use chrono::{DateTime, Duration, FixedOffset};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::process::ExitCode;

const ENGINE_VERSION: &str = "0.1.0";
const VALID_RECORD_STATUSES: &[&str] = &[
    "synthetic", "shadow_replay", "measured_campaign", "published_reproduction",
];
const VALID_AVAILABILITY_EVIDENCE: &[&str] = &[
    "publisher_publication_record",
    "operational_distribution_log",
    "verified_archive_manifest",
    "first_seen_capture",
    "synthetic_fixture",
];

#[derive(Clone, Debug)]
struct Finding {
    code: String,
    severity: String,
    message: String,
}

#[derive(Clone, Debug)]
struct DecisionReport {
    decision_id: String,
    disposition: String,
    policy_input_candidates: Vec<String>,
    admitted_policy_inputs: Vec<String>,
    evaluator_only_inputs: Vec<String>,
    validation_only_inputs: Vec<String>,
    findings: Vec<Finding>,
}

impl DecisionReport {
    fn to_value(&self) -> Value {
        json!({
            "decision_id": self.decision_id,
            "disposition": self.disposition,
            "policy_input_candidates": self.policy_input_candidates,
            "admitted_policy_inputs": self.admitted_policy_inputs,
            "evaluator_only_inputs": self.evaluator_only_inputs,
            "validation_only_inputs": self.validation_only_inputs,
            "findings": self.findings.iter().map(|finding| json!({
                "code": finding.code,
                "severity": finding.severity,
                "message": finding.message
            })).collect::<Vec<_>>()
        })
    }
}

fn add(findings: &mut Vec<Finding>, code: &str, severity: &str, message: &str) {
    findings.push(Finding {
        code: code.to_owned(),
        severity: severity.to_owned(),
        message: message.to_owned(),
    });
}

fn parse_time(value: &str) -> Option<DateTime<FixedOffset>> {
    DateTime::parse_from_rfc3339(value).ok()
}

fn string_at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a str> {
    path.iter().try_fold(value, |current, key| current.get(*key))?.as_str()
}

fn time_at(
    value: &Value,
    path: &[&str],
    label: &str,
    findings: &mut Vec<Finding>,
    missing_severity: &str,
) -> Option<DateTime<FixedOffset>> {
    match string_at(value, path) {
        Some(raw) => match parse_time(raw) {
            Some(parsed) => Some(parsed),
            None => {
                add(findings, "INVALID_TIMESTAMP", "ERROR",
                    &format!("{label} must be RFC 3339 with an explicit UTC offset."));
                None
            }
        },
        None => {
            add(findings, "MISSING_TIMESTAMP", missing_severity,
                &format!("{label} is required; retrieval time is not a substitute."));
            None
        }
    }
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn check_common_source_fields(source: &Value, id: &str, findings: &mut Vec<Finding>) {
    for path in [
        &["provider"][..], &["product"][..], &["product_version"][..],
        &["artifact_uri"][..], &["retrieved_at"][..],
    ] {
        if string_at(source, path).is_none_or(str::is_empty) {
            add(findings, "MISSING_SOURCE_METADATA", "INCOMPLETE",
                &format!("source '{id}' is missing a required identity, URI, version or retrieval field."));
        }
    }

    match string_at(source, &["artifact_sha256"]) {
        Some(digest) if valid_sha256(digest) => {}
        Some(_) => add(findings, "INVALID_ARTIFACT_DIGEST", "ERROR",
            &format!("source '{id}' must carry a 64-character SHA-256 digest.")),
        None => add(findings, "MISSING_ARTIFACT_DIGEST", "INCOMPLETE",
            &format!("source '{id}' has no immutable artifact digest.")),
    }

    let retrieved = string_at(source, &["retrieved_at"]).and_then(parse_time);
    if string_at(source, &["retrieved_at"]).is_some() && retrieved.is_none() {
        add(findings, "INVALID_TIMESTAMP", "ERROR",
            &format!("source '{id}' retrieved_at must be RFC 3339."));
    }

    let empty_rights = Value::Null;
    let rights = source.get("rights").unwrap_or(&empty_rights);
    match string_at(rights, &["status"]).unwrap_or("unknown") {
        "reviewed_research_use" | "permissioned_research_use" => {
            if rights.get("research_use_allowed").and_then(Value::as_bool) != Some(true) {
                add(findings, "RESEARCH_USE_NOT_AUTHORIZED", "BLOCKER",
                    &format!("source '{id}' rights status does not permit research use."));
            }
            if string_at(rights, &["review_ref"]).is_none_or(str::is_empty) {
                add(findings, "RIGHTS_REVIEW_REFERENCE_MISSING", "INCOMPLETE",
                    &format!("source '{id}' needs a rights/permission review reference."));
            }
        }
        "synthetic_only" => {}
        "restricted" => add(findings, "SOURCE_RIGHTS_RESTRICTED", "BLOCKER",
            &format!("source '{id}' is restricted for this replay.")),
        "unknown" => add(findings, "SOURCE_RIGHTS_UNKNOWN", "INCOMPLETE",
            &format!("source '{id}' rights have not been reviewed.")),
        _ => add(findings, "INVALID_RIGHTS_STATUS", "ERROR",
            &format!("source '{id}' has an unsupported rights status.")),
    }

    if rights.get("research_use_allowed").and_then(Value::as_bool).is_none() {
        add(findings, "MISSING_RESEARCH_RIGHTS_DECISION", "INCOMPLETE",
            &format!("source '{id}' must explicitly state whether research use is allowed."));
    }
    if rights.get("redistributable").and_then(Value::as_bool).is_none() {
        add(findings, "MISSING_REDISTRIBUTION_STATUS", "INCOMPLETE",
            &format!("source '{id}' must explicitly state whether redistribution is permitted."));
    }
}

fn check_forecast_source(
    source: &Value,
    id: &str,
    decision_cutoff: &DateTime<FixedOffset>,
    target_valid_time: &DateTime<FixedOffset>,
    findings: &mut Vec<Finding>,
) {
    let issue = time_at(source, &["forecast", "source_issue_time"],
        "forecast source_issue_time", findings, "INCOMPLETE");
    let available = time_at(source, &["forecast", "first_available_at"],
        "forecast first_available_at", findings, "INCOMPLETE");
    let valid = time_at(source, &["forecast", "valid_time"],
        "forecast valid_time", findings, "INCOMPLETE");

    if let (Some(issue), Some(available)) = (issue.as_ref(), available.as_ref()) {
        if issue > available {
            add(findings, "FORECAST_AVAILABLE_BEFORE_ISSUE", "ERROR",
                &format!("source '{id}' first availability precedes its issue time."));
        } else if available > decision_cutoff {
            add(findings, "FUTURE_FORECAST_LEAKAGE", "BLOCKER",
                &format!("source '{id}' was not demonstrably available by the policy decision cutoff."));
        }
    }

    if let Some(valid_time) = valid.as_ref() {
        if valid_time != target_valid_time {
            add(findings, "FORECAST_VALID_TIME_MISMATCH", "BLOCKER",
                &format!("source '{id}' is valid at a different instant than this decision's target time."));
        }
    }

    match string_at(source, &["forecast", "availability_evidence"]) {
        Some(evidence) if VALID_AVAILABILITY_EVIDENCE.contains(&evidence) => {}
        Some("unknown") | None => add(findings, "AVAILABILITY_EVIDENCE_UNKNOWN", "INCOMPLETE",
            &format!("source '{id}' has no defensible first-availability evidence.")),
        Some(_) => add(findings, "INVALID_AVAILABILITY_EVIDENCE", "ERROR",
            &format!("source '{id}' uses an unsupported availability evidence class.")),
    }
    if string_at(source, &["forecast", "availability_evidence_ref"]).is_none_or(str::is_empty) {
        add(findings, "AVAILABILITY_EVIDENCE_REFERENCE_MISSING", "INCOMPLETE",
            &format!("source '{id}' needs a stable reference to the availability evidence artifact."));
    }
    match string_at(source, &["forecast", "availability_evidence_sha256"]) {
        Some(digest) if valid_sha256(digest) => {}
        Some(_) => add(findings, "AVAILABILITY_EVIDENCE_DIGEST_INVALID", "ERROR",
            &format!("source '{id}' availability evidence requires a 64-character SHA-256 digest.")),
        None => add(findings, "AVAILABILITY_EVIDENCE_DIGEST_MISSING", "INCOMPLETE",
            &format!("source '{id}' availability evidence has no immutable digest.")),
    }

    if string_at(source, &["forecast", "cycle_id"]).is_none_or(str::is_empty)
        || string_at(source, &["forecast", "parameter_set"]).is_none_or(str::is_empty)
    {
        add(findings, "FORECAST_CYCLE_METADATA_INCOMPLETE", "INCOMPLETE",
            &format!("source '{id}' needs exact cycle and parameter-set identity."));
    }
    let lead_hours = source.get("forecast").and_then(|v| v.get("lead_hours"))
        .and_then(Value::as_u64);
    if lead_hours.is_none() {
        add(findings, "FORECAST_LEAD_MISSING", "INCOMPLETE",
            &format!("source '{id}' requires a non-negative forecast lead."));
    }
    if let (Some(issue), Some(valid), Some(lead)) = (
        issue.as_ref(), valid.as_ref(), lead_hours
    ) {
        if let Ok(hours) = i64::try_from(lead) {
            if *issue + Duration::hours(hours) != *valid {
                add(findings, "FORECAST_LEAD_VALID_TIME_MISMATCH", "ERROR",
                    &format!("source '{id}' cycle issue time + forecast lead does not equal valid_time."));
            }
        } else {
            add(findings, "FORECAST_LEAD_OUT_OF_RANGE", "ERROR",
                &format!("source '{id}' forecast lead is outside supported integer-hour range."));
        }
    }
    let retrieved = string_at(source, &["retrieved_at"]).and_then(parse_time);
    if let (Some(retrieved), Some(available)) = (
        retrieved.as_ref(), available.as_ref()
    ) {
        if retrieved < available {
            add(findings, "RETRIEVAL_PRECEDES_AVAILABILITY", "ERROR",
                &format!("source '{id}' retrieval time precedes its declared first availability."));
        }
    }

    let empty_grid = Value::Null;
    let grid = source.get("grid").unwrap_or(&empty_grid);
    for key in ["grid_id", "coordinate_reference", "axis_order"] {
        if string_at(grid, &[key]).is_none_or(str::is_empty) {
            add(findings, "GRID_METADATA_INCOMPLETE", "INCOMPLETE",
                &format!("source '{id}' lacks grid field '{key}'."));
        }
    }
    if grid.get("variable_units").and_then(Value::as_object).is_none_or(|units| units.is_empty()) {
        add(findings, "VARIABLE_UNITS_MISSING", "INCOMPLETE",
            &format!("source '{id}' requires an explicit variable-to-unit map."));
    }
}

fn check_nonforecast_source(source: &Value, id: &str, class: &str, findings: &mut Vec<Finding>) {
    match class {
        "realized_evaluator_context" => {
            if source.get("evaluator").is_none() {
                add(findings, "EVALUATOR_METADATA_MISSING", "INCOMPLETE",
                    &format!("source '{id}' requires evaluator-only metadata."));
            }
        }
        "in_situ_validation" => {
            let station = string_at(source, &["observation", "station_id"]);
            let sensor = string_at(source, &["observation", "sensor_id"]);
            let observed = string_at(source, &["observation", "observation_time"]);
            if station.is_none_or(str::is_empty)
                || sensor.is_none_or(str::is_empty)
                || observed.and_then(parse_time).is_none()
                || source.get("observation").and_then(|v| v.get("quality_flags"))
                    .and_then(Value::as_array).is_none()
            {
                add(findings, "OBSERVATION_METADATA_INCOMPLETE", "INCOMPLETE",
                    &format!("source '{id}' requires station, sensor, timestamp and QC metadata."));
            }
        }
        "commercial_route_reference" => {
            if string_at(source, &["commercial", "export_format"]).is_none_or(str::is_empty)
                || string_at(source, &["commercial", "settings_digest"]).is_none_or(|digest| !valid_sha256(digest))
            {
                add(findings, "COMMERCIAL_ROUTE_METADATA_INCOMPLETE", "INCOMPLETE",
                    &format!("source '{id}' requires an export format and a digest for captured routing settings."));
            }
        }
        _ => add(findings, "UNKNOWN_INFORMATION_CLASS", "ERROR",
            &format!("source '{id}' has unsupported information class '{class}'.")),
    }
}

fn audit_decision(
    decision: &Value,
    classes: &BTreeMap<String, String>,
    sources: &BTreeMap<String, &Value>,
    synthetic: bool,
) -> DecisionReport {
    let decision_id = string_at(decision, &["decision_id"]).unwrap_or("<missing>").to_owned();
    let mut findings = Vec::new();
    let cutoff = time_at(decision, &["decision_cutoff"], "decision cutoff", &mut findings, "ERROR");
    let target = time_at(decision, &["target_valid_time"], "target valid time", &mut findings, "ERROR");

    let mut policy_candidates = Vec::new();
    let mut evaluator_inputs = Vec::new();
    let mut validation_inputs = Vec::new();
    let mut seen = BTreeSet::new();
    let policy_ids = decision.get("policy_input_source_ids").and_then(Value::as_array);
    let evaluator_ids = decision.get("evaluator_context_source_ids").and_then(Value::as_array);
    let validation_ids = decision.get("validation_observation_source_ids").and_then(Value::as_array);

    for (field, values) in [
        ("policy_input_source_ids", policy_ids),
        ("evaluator_context_source_ids", evaluator_ids),
        ("validation_observation_source_ids", validation_ids),
    ] {
        if values.is_none() {
            add(&mut findings, "MISSING_SOURCE_REFERENCE_LIST", "ERROR",
                &format!("decision '{decision_id}' requires the '{field}' array."));
        }
    }

    for (field, values) in [
        ("policy_input_source_ids", policy_ids),
        ("evaluator_context_source_ids", evaluator_ids),
        ("validation_observation_source_ids", validation_ids),
    ] {
        for raw in values.into_iter().flatten() {
            let Some(id) = raw.as_str() else {
                add(&mut findings, "INVALID_SOURCE_REFERENCE", "ERROR",
                    &format!("decision '{decision_id}' contains a non-string source reference."));
                continue;
            };
            if !seen.insert(id.to_owned()) {
                add(&mut findings, "SOURCE_ROLE_REUSE", "ERROR",
                    &format!("source '{id}' appears in more than one decision information class."));
            }
            let Some(source) = sources.get(id) else {
                add(&mut findings, "SOURCE_REFERENCE_NOT_FOUND", "ERROR",
                    &format!("decision '{decision_id}' references unknown source '{id}'."));
                continue;
            };
            let class = classes.get(id).map(String::as_str).unwrap_or("unknown");
            match field {
                "policy_input_source_ids" => {
                    if class != "deployable_forecast" {
                        add(&mut findings, "NONFORECAST_IN_POLICY_INPUT", "BLOCKER",
                            &format!("source '{id}' is '{class}' and cannot enter deployable forecast policy inputs."));
                    } else if let (Some(cutoff), Some(target)) = (cutoff.as_ref(), target.as_ref()) {
                        check_common_source_fields(source, id, &mut findings);
                        check_forecast_source(source, id, cutoff, target, &mut findings);
                    }
                    policy_candidates.push(id.to_owned());
                }
                "evaluator_context_source_ids" => {
                    if class != "realized_evaluator_context" {
                        add(&mut findings, "NONREALIZED_EVALUATOR_CONTEXT", "ERROR",
                            &format!("source '{id}' is not a realized evaluator context."));
                    }
                    let evaluator_time = time_at(
                        source, &["evaluator", "valid_time"], "evaluator valid_time",
                        &mut findings, "INCOMPLETE"
                    );
                    if evaluator_time.as_ref().is_none_or(|value| Some(value) != target.as_ref()) {
                        add(&mut findings, "EVALUATOR_VALID_TIME_MISMATCH", "INCOMPLETE",
                            &format!("evaluator source '{id}' does not identify this decision's target valid time."));
                    }
                    evaluator_inputs.push(id.to_owned());
                }
                "validation_observation_source_ids" => {
                    if class != "in_situ_validation" {
                        add(&mut findings, "NONOBSERVATION_VALIDATION_INPUT", "ERROR",
                            &format!("source '{id}' is not an in-situ observation."));
                    }
                    validation_inputs.push(id.to_owned());
                }
                _ => {}
            }
        }
    }

    let disposition = if findings.iter().any(|f| f.severity == "ERROR") {
        "INVALID"
    } else if findings.iter().any(|f| f.severity == "BLOCKER") {
        "BLOCKED"
    } else if findings.iter().any(|f| f.severity == "INCOMPLETE") {
        "INCOMPLETE"
    } else if synthetic {
        "SYNTHETIC_ONLY"
    } else {
        "ADMISSIBLE_READ_ONLY"
    };

    DecisionReport {
        decision_id,
        disposition: disposition.to_owned(),
        policy_input_candidates: policy_candidates,
        admitted_policy_inputs: if disposition == "ADMISSIBLE_READ_ONLY" {
            policy_candidates_for_admission(decision, classes)
        } else {
            Vec::new()
        },
        evaluator_only_inputs: evaluator_inputs,
        validation_only_inputs: validation_inputs,
        findings,
    }
}

fn policy_candidates_for_admission(
    decision: &Value,
    classes: &BTreeMap<String, String>,
) -> Vec<String> {
    decision.get("policy_input_source_ids").and_then(Value::as_array)
        .into_iter().flatten().filter_map(Value::as_str)
        .filter(|id| classes.get(*id).map(String::as_str) == Some("deployable_forecast"))
        .map(str::to_owned).collect()
}

fn audit_bundle(raw: &[u8], bundle: &Value) -> Value {
    let bundle_id = string_at(bundle, &["bundle_id"]).unwrap_or("<missing>").to_owned();
    let mut global_findings = Vec::new();

    if !bundle.is_object() {
        add(&mut global_findings, "BUNDLE_NOT_OBJECT", "ERROR", "Bundle root must be a JSON object.");
    }
    if string_at(bundle, &["schema_version"]) != Some("0.1.0") {
        add(&mut global_findings, "UNSUPPORTED_SCHEMA_VERSION", "ERROR", "schema_version must be 0.1.0.");
    }
    let record_status = string_at(bundle, &["record_status"]).unwrap_or("unknown");
    if !VALID_RECORD_STATUSES.contains(&record_status) {
        add(&mut global_findings, "INVALID_RECORD_STATUS", "ERROR", "record_status is not recognized.");
    }
    if string_at(bundle, &["claim_ceiling"]) != Some("replay_admission_only") {
        add(&mut global_findings, "CLAIM_CEILING_MISMATCH", "ERROR",
            "This tool only audits replay admission; it cannot claim forecast accuracy or vessel safety.");
    }

    let classes = source_class_map(bundle, &mut global_findings);
    let mut sources: BTreeMap<String, &Value> = BTreeMap::new();
    if let Some(items) = bundle.get("sources").and_then(Value::as_array) {
        for source in items {
            let id = string_at(source, &["source_id"]).unwrap_or("");
            if id.is_empty() { continue; }
            sources.insert(id.to_owned(), source);
            check_common_source_fields(source, id, &mut global_findings);

            let evidence_class = string_at(source, &["evidence_class"]).unwrap_or("unknown");
            let allowed_evidence_classes = ["synthetic", "published", "measured", "validated_model", "derived"];
            if !allowed_evidence_classes.contains(&evidence_class) {
                add(&mut global_findings, "INVALID_SOURCE_EVIDENCE_CLASS", "ERROR",
                    &format!("source '{id}' has an unsupported evidence class."));
            }
            if record_status == "synthetic" && evidence_class != "synthetic" {
                add(&mut global_findings, "SYNTHETIC_EVIDENCE_CLASS_MISMATCH", "ERROR",
                    &format!("synthetic bundle source '{id}' must be marked evidence_class=synthetic."));
            }
            if record_status != "synthetic" && evidence_class == "synthetic" {
                add(&mut global_findings, "SYNTHETIC_EVIDENCE_NOT_PROMOTABLE", "BLOCKER",
                    &format!("non-synthetic bundle contains synthetic source '{id}'."));
            }
            if record_status != "synthetic"
                && string_at(source, &["rights", "status"]) == Some("synthetic_only")
            {
                add(&mut global_findings, "SYNTHETIC_RIGHTS_NOT_PROMOTABLE", "BLOCKER",
                    &format!("non-synthetic bundle source '{id}' carries synthetic-only rights status."));
            }
            let availability_evidence = string_at(source, &["forecast", "availability_evidence"]);
            if record_status == "synthetic" && evidence_class == "synthetic"
                && classes.get(id).map(String::as_str) == Some("deployable_forecast")
                && availability_evidence != Some("synthetic_fixture")
            {
                add(&mut global_findings, "SYNTHETIC_AVAILABILITY_EVIDENCE_MISMATCH", "ERROR",
                    &format!("synthetic forecast source '{id}' must use the synthetic_fixture availability marker."));
            }
            if record_status != "synthetic" && availability_evidence == Some("synthetic_fixture") {
                add(&mut global_findings, "SYNTHETIC_AVAILABILITY_NOT_PROMOTABLE", "BLOCKER",
                    &format!("non-synthetic forecast source '{id}' carries a synthetic-only availability marker."));
            }

            let class = classes.get(id).map(String::as_str).unwrap_or("unknown");
            if class == "deployable_forecast" {
                let dummy_cutoff = parse_time("9999-12-31T23:59:59Z").expect("valid RFC 3339");
                let dummy_target = string_at(source, &["forecast", "valid_time"])
                    .and_then(parse_time).unwrap_or(dummy_cutoff);
                check_forecast_source(source, id, &dummy_cutoff, &dummy_target, &mut global_findings);
            } else {
                check_nonforecast_source(source, id, class, &mut global_findings);
            }
        }
    }

    let synthetic = record_status == "synthetic";
    let mut decisions = Vec::new();
    let mut seen_decisions = BTreeSet::new();
    match bundle.get("decisions").and_then(Value::as_array) {
        Some(items) if !items.is_empty() => {
            for decision in items {
                let decision_id = string_at(decision, &["decision_id"]).unwrap_or("");
                if decision_id.is_empty() || !seen_decisions.insert(decision_id.to_owned()) {
                    add(&mut global_findings, "INVALID_DECISION_ID", "ERROR",
                        "decision identifiers must be non-empty and unique.");
                }
                decisions.push(audit_decision(decision, &classes, &sources, synthetic));
            }
        }
        _ => add(&mut global_findings, "MISSING_DECISIONS", "ERROR",
            "At least one decision snapshot is required."),
    }

    let all_admissible = global_findings.is_empty()
        && decisions.iter().all(|decision| decision.disposition == "ADMISSIBLE_READ_ONLY");
    let has_error = global_findings.iter().any(|f| f.severity == "ERROR")
        || decisions.iter().any(|d| d.disposition == "INVALID");
    let has_blocker = global_findings.iter().any(|f| f.severity == "BLOCKER")
        || decisions.iter().any(|d| d.disposition == "BLOCKED");
    let has_incomplete = global_findings.iter().any(|f| f.severity == "INCOMPLETE")
        || decisions.iter().any(|d| d.disposition == "INCOMPLETE");

    let disposition = if has_error {
        "INVALID"
    } else if has_blocker {
        "BLOCKED"
    } else if has_incomplete {
        "INCOMPLETE"
    } else if synthetic {
        "SYNTHETIC_ONLY"
    } else if all_admissible {
        "REPLAY_ADMISSIBLE_READ_ONLY"
    } else {
        "INCOMPLETE"
    };

    let raw_digest = Sha256::digest(raw);
    let input_digest = raw_digest.iter().map(|byte| format!("{byte:02x}")).collect::<String>();
    json!({
        "audit_version": ENGINE_VERSION,
        "bundle_id": bundle_id,
        "disposition": disposition,
        "input_bundle_sha256": input_digest,
        "decision_reports": decisions.iter().map(DecisionReport::to_value).collect::<Vec<_>>(),
        "global_findings": global_findings.iter().map(|finding| json!({
            "code": finding.code,
            "severity": finding.severity,
            "message": finding.message
        })).collect::<Vec<_>>(),
        "claim_ceiling": "information_set_admission_only",
        "digest_note": "The bundle digest identifies exact input bytes. SHA-256 shape/content identity does not authenticate the publisher, prove data quality, or establish physical truth."
    })
}

fn source_class_map(bundle: &Value, findings: &mut Vec<Finding>) -> BTreeMap<String, String> {
    let mut classes = BTreeMap::new();
    let Some(sources) = bundle.get("sources").and_then(Value::as_array) else {
        add(findings, "MISSING_SOURCES", "ERROR", "sources must be an array.");
        return classes;
    };
    for (index, source) in sources.iter().enumerate() {
        let id = string_at(source, &["source_id"]).unwrap_or("");
        if id.is_empty() {
            add(findings, "MISSING_SOURCE_ID", "ERROR", &format!("sources[{index}] has no source_id."));
            continue;
        }
        let class = string_at(source, &["information_class"]).unwrap_or("unknown");
        if classes.insert(id.to_owned(), class.to_owned()).is_some() {
            add(findings, "DUPLICATE_SOURCE_ID", "ERROR", &format!("source_id '{id}' is duplicated."));
        }
    }
    classes
}


fn validate_split_manifest(raw: &[u8], manifest: &Value) -> Value {
    let mut findings = Vec::<Finding>::new();
    let manifest_id = string_at(manifest, &["manifest_id"]).unwrap_or("<missing>").to_owned();
    let record_status = string_at(manifest, &["record_status"]).unwrap_or("unknown");
    let synthetic = record_status == "synthetic";

    if string_at(manifest, &["schema_version"]) != Some("0.1.0") {
        add(&mut findings, "SPLIT_SCHEMA_VERSION_INVALID", "ERROR",
            "Split manifest schema_version must be 0.1.0.");
    }
    if !VALID_RECORD_STATUSES.contains(&record_status) {
        add(&mut findings, "SPLIT_RECORD_STATUS_INVALID", "ERROR",
            "Split manifest record_status is not recognized.");
    }
    if string_at(manifest, &["claim_ceiling"]) != Some("holdout_partition_integrity_only") {
        add(&mut findings, "SPLIT_CLAIM_CEILING_INVALID", "ERROR",
            "Split validation only establishes partition integrity, not statistical power or model quality.");
    }
    let grouping_unit = string_at(manifest, &["grouping_unit"]).unwrap_or("unknown");
    if !["whole_voyage", "weather_event"].contains(&grouping_unit) {
        add(&mut findings, "SPLIT_GROUPING_UNIT_INVALID", "ERROR",
            "grouping_unit must be whole_voyage or weather_event.");
    }
    if string_at(manifest, &["split_algorithm"]).is_none_or(str::is_empty)
        || string_at(manifest, &["split_seed"]).is_none_or(str::is_empty)
    {
        add(&mut findings, "SPLIT_ALGORITHM_UNPINNED", "INCOMPLETE",
            "Split algorithm and seed must be pinned before evaluating model results.");
    }

    let expected_folds = ["development", "validation", "test"];
    let fold_values = manifest.get("folds").and_then(Value::as_array);
    let declared_folds: BTreeSet<String> = fold_values.into_iter().flatten()
        .filter_map(Value::as_str).map(str::to_owned).collect();
    if fold_values.is_none_or(|items| items.len() != 3)
        || expected_folds.iter().any(|fold| !declared_folds.contains(*fold))
        || declared_folds.len() != 3
    {
        add(&mut findings, "SPLIT_FOLD_ROSTER_INVALID", "ERROR",
            "Exactly the development, validation and test folds must be declared.");
    }

    let episodes: &[Value] = match manifest.get("episodes").and_then(Value::as_array) {
        Some(items) if !items.is_empty() => items.as_slice(),
        _ => {
            add(&mut findings, "SPLIT_EPISODES_MISSING", "ERROR",
                "At least one grouped episode is required.");
            &[]
        }
    };

    let mut episode_ids = BTreeSet::new();
    let mut decision_ids = BTreeSet::new();
    let mut sample_counts = BTreeMap::<String, usize>::new();
    let mut event_fold = BTreeMap::<String, String>::new();
    let mut voyage_fold = BTreeMap::<String, String>::new();
    let mut bundle_fold = BTreeMap::<String, String>::new();
    let mut voyage_windows = BTreeMap::<String, Vec<(DateTime<FixedOffset>, DateTime<FixedOffset>, String)>>();
    let mut test_strata = BTreeSet::new();

    for (index, episode) in episodes.iter().enumerate() {
        let episode_id = string_at(episode, &["episode_id"]).unwrap_or("");
        let fold = string_at(episode, &["fold"]).unwrap_or("");
        let voyage_id = string_at(episode, &["voyage_id"]).unwrap_or("");
        let weather_event_id = string_at(episode, &["weather_event_id"]).unwrap_or("");
        let bundle_digest = string_at(episode, &["replay_bundle_sha256"]).unwrap_or("");

        if episode_id.is_empty() || !episode_ids.insert(episode_id.to_owned()) {
            add(&mut findings, "SPLIT_EPISODE_ID_INVALID", "ERROR",
                &format!("episode[{index}] must have a unique non-empty episode_id."));
        }
        if !expected_folds.contains(&fold) {
            add(&mut findings, "SPLIT_EPISODE_FOLD_INVALID", "ERROR",
                &format!("episode '{episode_id}' uses an undeclared fold."));
        } else {
            *sample_counts.entry(fold.to_owned()).or_default() += 1;
        }
        if voyage_id.is_empty() || weather_event_id.is_empty() {
            add(&mut findings, "SPLIT_GROUP_ID_MISSING", "ERROR",
                &format!("episode '{episode_id}' requires both voyage_id and weather_event_id."));
        } else {
            match voyage_fold.get(voyage_id) {
                Some(existing_fold) if existing_fold != fold => add(
                    &mut findings, "CROSS_FOLD_GROUP_LEAKAGE", "BLOCKER",
                    &format!("voyage '{voyage_id}' appears in both '{existing_fold}' and '{fold}' folds.")
                ),
                None => { voyage_fold.insert(voyage_id.to_owned(), fold.to_owned()); }
                _ => {}
            }
            match event_fold.get(weather_event_id) {
                Some(existing_fold) if existing_fold != fold => add(
                    &mut findings, "CROSS_FOLD_GROUP_LEAKAGE", "BLOCKER",
                    &format!("weather event '{weather_event_id}' appears in both '{existing_fold}' and '{fold}' folds.")
                ),
                None => { event_fold.insert(weather_event_id.to_owned(), fold.to_owned()); }
                _ => {}
            }
        }

        if !valid_sha256(bundle_digest) {
            add(&mut findings, "SPLIT_REPLAY_BUNDLE_DIGEST_INVALID", "ERROR",
                &format!("episode '{episode_id}' requires a SHA-256 replay bundle digest."));
        } else if let Some(existing_fold) = bundle_fold.get(bundle_digest) {
            if existing_fold != fold {
                add(&mut findings, "CROSS_FOLD_BUNDLE_REUSE", "BLOCKER",
                    &format!("replay bundle '{bundle_digest}' is reused across folds."));
            }
        } else {
            bundle_fold.insert(bundle_digest.to_owned(), fold.to_owned());
        }

        let start = time_at(episode, &["start_time"], "episode start_time",
            &mut findings, "ERROR");
        let end = time_at(episode, &["end_time"], "episode end_time",
            &mut findings, "ERROR");
        if let (Some(start), Some(end)) = (start.as_ref(), end.as_ref()) {
            if start >= end {
                add(&mut findings, "SPLIT_EPISODE_WINDOW_INVALID", "ERROR",
                    &format!("episode '{episode_id}' start_time must precede end_time."));
            } else if !voyage_id.is_empty() {
                voyage_windows.entry(voyage_id.to_owned()).or_default()
                    .push((start.clone(), end.clone(), episode_id.to_owned()));
            }
        }

        let strata = episode.get("environment_strata").and_then(Value::as_array);
        match strata {
            Some(items) if !items.is_empty() => {
                let mut seen_episode_strata = BTreeSet::new();
                for stratum in items {
                    let Some(stratum) = stratum.as_str() else {
                        add(&mut findings, "SPLIT_STRATUM_INVALID", "ERROR",
                            &format!("episode '{episode_id}' contains a non-string environment stratum."));
                        continue;
                    };
                    if stratum.is_empty() || !seen_episode_strata.insert(stratum.to_owned()) {
                        add(&mut findings, "SPLIT_STRATUM_INVALID", "ERROR",
                            &format!("episode '{episode_id}' contains an empty or duplicated stratum."));
                    }
                    if fold == "test" {
                        test_strata.insert(stratum.to_owned());
                    }
                }
            }
            _ => add(&mut findings, "SPLIT_ENVIRONMENT_STRATA_MISSING", "INCOMPLETE",
                &format!("episode '{episode_id}' requires at least one declared environment stratum.")),
        }

        let samples = match episode.get("decision_samples").and_then(Value::as_array) {
            Some(items) if !items.is_empty() => items,
            _ => {
                add(&mut findings, "SPLIT_DECISION_SAMPLES_MISSING", "INCOMPLETE",
                    &format!("episode '{episode_id}' requires decision sample references."));
                continue;
            }
        };
        let mut previous_cutoff: Option<DateTime<FixedOffset>> = None;
        for (sample_index, sample) in samples.iter().enumerate() {
            let decision_id = string_at(sample, &["decision_id"]).unwrap_or("");
            if decision_id.is_empty() || !decision_ids.insert(decision_id.to_owned()) {
                add(&mut findings, "SPLIT_DECISION_ID_DUPLICATED", "ERROR",
                    &format!("episode '{episode_id}' sample[{sample_index}] has a missing or reused decision_id."));
            }
            let cutoff = time_at(sample, &["decision_cutoff"], "sample decision_cutoff",
                &mut findings, "ERROR");
            let target = time_at(sample, &["target_valid_time"], "sample target_valid_time",
                &mut findings, "ERROR");
            if let (Some(start), Some(end), Some(cutoff), Some(target)) =
                (start.as_ref(), end.as_ref(), cutoff.as_ref(), target.as_ref())
            {
                if cutoff < start || cutoff > end || target < cutoff || target > end {
                    add(&mut findings, "SPLIT_SAMPLE_OUTSIDE_EPISODE", "ERROR",
                        &format!("decision '{decision_id}' cutoff/target falls outside episode '{episode_id}'."));
                }
            }
            if let Some(cutoff) = cutoff {
                if previous_cutoff.as_ref().is_some_and(|prior| &cutoff < prior) {
                    add(&mut findings, "SPLIT_DECISION_ORDER_NONMONOTONIC", "ERROR",
                        &format!("decision cutoffs in episode '{episode_id}' must be nondecreasing."));
                }
                previous_cutoff = Some(cutoff);
            }
        }
    }

    let voyages = voyage_windows.keys().cloned().collect::<Vec<_>>();
    for voyage in voyages {
        if let Some(windows) = voyage_windows.get_mut(&voyage) {
            windows.sort_by(|left, right| left.0.cmp(&right.0));
            for pair in windows.windows(2) {
                if pair[1].0 < pair[0].1 {
                    add(&mut findings, "OVERLAPPING_VOYAGE_EPISODES", "BLOCKER",
                        &format!("episodes '{}' and '{}' overlap within voyage '{}'.",
                            pair[0].2, pair[1].2, voyage));
                }
            }
        }
    }

    for fold in expected_folds {
        if sample_counts.get(fold).copied().unwrap_or(0) == 0 {
            add(&mut findings, "SPLIT_FOLD_EMPTY", "INCOMPLETE",
                &format!("fold '{fold}' contains no episodes."));
        }
    }
    if test_strata.len() < 3 {
        add(&mut findings, "TEST_STRATA_COVERAGE_INSUFFICIENT", "INCOMPLETE",
            "The held-out test fold must cover at least three declared environment strata.");
    }

    let has_error = findings.iter().any(|finding| finding.severity == "ERROR");
    let has_blocker = findings.iter().any(|finding| finding.severity == "BLOCKER");
    let has_incomplete = findings.iter().any(|finding| finding.severity == "INCOMPLETE");
    let disposition = if has_error {
        "INVALID"
    } else if has_blocker {
        "BLOCKED"
    } else if has_incomplete {
        "INCOMPLETE"
    } else if synthetic {
        "SYNTHETIC_ONLY"
    } else {
        "SPLIT_VALID_READ_ONLY"
    };
    let digest = Sha256::digest(raw);
    let digest_hex = digest.iter().map(|byte| format!("{byte:02x}")).collect::<String>();
    json!({
        "audit_version": ENGINE_VERSION,
        "manifest_id": manifest_id,
        "disposition": disposition,
        "input_manifest_sha256": digest_hex,
        "episode_count": episodes.len(),
        "decision_sample_count": decision_ids.len(),
        "episodes_by_fold": sample_counts,
        "held_out_test_strata": test_strata,
        "claim_ceiling": "episode_and_fold_partition_integrity_only",
        "findings": findings.iter().map(|finding| json!({
            "code": finding.code,
            "severity": finding.severity,
            "message": finding.message
        })).collect::<Vec<_>>(),
        "note": "A valid split manifest does not establish statistical power, independence of real weather events, forecast skill, or benchmark superiority."
    })
}

fn run() -> Result<(), String> {
    let arguments: Vec<String> = env::args().collect();
    if arguments.len() != 3 {
        return Err("Usage: marine-forecast-replay-audit audit-bundle <bundle.json> | validate-splits <split.json>".to_owned());
    }
    let raw = fs::read(&arguments[2]).map_err(|error| format!("{}: {error}", arguments[2]))?;
    let value: Value = serde_json::from_slice(&raw)
        .map_err(|error| format!("{}: invalid JSON: {error}", arguments[2]))?;
    let report = match arguments[1].as_str() {
        "audit-bundle" => audit_bundle(&raw, &value),
        "validate-splits" => validate_split_manifest(&raw, &value),
        _ => return Err("unknown subcommand".to_owned()),
    };
    println!("{}", serde_json::to_string_pretty(&report).map_err(|error| error.to_string())?);
    match string_at(&report, &["disposition"]) {
        Some("REPLAY_ADMISSIBLE_READ_ONLY" | "SYNTHETIC_ONLY" | "SPLIT_VALID_READ_ONLY") => Ok(()),
        Some(disposition) => Err(format!("replay audit disposition: {disposition}")),
        None => Err("audit report has no disposition".to_owned()),
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Value {
        serde_json::from_str(include_str!("../fixtures/synthetic-replay-bundle-0001.json"))
            .expect("synthetic replay bundle must parse")
    }

    fn set_research_admissible(bundle: &mut Value) {
        bundle["record_status"] = json!("shadow_replay");
        for source in bundle["sources"].as_array_mut().unwrap() {
            source["rights"]["status"] = json!("reviewed_research_use");
            source["rights"]["research_use_allowed"] = json!(true);
            source["rights"]["review_ref"] = json!("TEST-RESEARCH-RIGHTS-REVIEW");
            if source["information_class"] == "deployable_forecast" {
                source["forecast"]["availability_evidence"] = json!("operational_distribution_log");
                source["forecast"]["availability_evidence_ref"] = json!("TEST-OPERATIONAL-DISTRIBUTION-RECEIPT");
                source["forecast"]["availability_evidence_sha256"] = json!("dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd");
            }
            source["evidence_class"] = match source["information_class"].as_str().unwrap() {
                "deployable_forecast" => json!("published"),
                "realized_evaluator_context" => json!("derived"),
                "in_situ_validation" => json!("measured"),
                _ => json!("published"),
            };
        }
    }

    fn audit(value: &Value) -> Value {
        let raw = serde_json::to_vec(value).expect("bundle serializes");
        audit_bundle(&raw, value)
    }

    #[test]
    fn synthetic_fixture_is_never_promoted_to_a_real_replay() {
        let report = audit(&fixture());
        assert_eq!(report["disposition"], "SYNTHETIC_ONLY");
        assert_eq!(report["decision_reports"][0]["disposition"], "SYNTHETIC_ONLY");
        assert!(report["decision_reports"][0]["admitted_policy_inputs"].as_array().unwrap().is_empty());
    }

    #[test]
    fn archive_retrieval_after_cutoff_does_not_replace_original_availability() {
        let mut value = fixture();
        set_research_admissible(&mut value);
        value["decisions"][0]["decision_cutoff"] = json!("2026-11-19T06:30:00Z");
        value["sources"][0]["retrieved_at"] = json!("2026-11-25T12:00:00Z");
        assert_eq!(audit(&value)["disposition"], "REPLAY_ADMISSIBLE_READ_ONLY");
    }

    #[test]
    fn first_available_after_cutoff_is_blocked() {
        let mut value = fixture();
        set_research_admissible(&mut value);
        value["sources"][0]["forecast"]["first_available_at"] = json!("2026-11-19T06:35:00Z");
        let report = audit(&value);
        assert_eq!(report["disposition"], "BLOCKED");
        assert!(report["decision_reports"][0]["findings"].as_array().unwrap()
            .iter().any(|finding| finding["code"] == "FUTURE_FORECAST_LEAKAGE"));
    }

    #[test]
    fn unknown_original_availability_is_incomplete_even_when_retrieved_early() {
        let mut value = fixture();
        set_research_admissible(&mut value);
        value["sources"][0]["forecast"]["availability_evidence"] = json!("unknown");
        assert_eq!(audit(&value)["disposition"], "INCOMPLETE");
    }

    #[test]
    fn realized_reanalysis_cannot_enter_policy_inputs() {
        let mut value = fixture();
        value["decisions"][0]["policy_input_source_ids"] = json!(["gfs-cycle-a", "era5-analysis"]);
        value["decisions"][0]["evaluator_context_source_ids"] = json!([]);
        let report = audit(&value);
        assert_eq!(report["disposition"], "BLOCKED");
        assert!(report["decision_reports"][0]["findings"].as_array().unwrap()
            .iter().any(|finding| finding["code"] == "NONFORECAST_IN_POLICY_INPUT"));
    }

    #[test]
    fn buoy_observations_remain_validation_only() {
        let mut value = fixture();
        value["decisions"][0]["policy_input_source_ids"] = json!(["gfs-cycle-a", "ndbc-44013"]);
        value["decisions"][0]["validation_observation_source_ids"] = json!([]);
        let report = audit(&value);
        assert_eq!(report["disposition"], "BLOCKED");
        assert!(report["decision_reports"][0]["findings"].as_array().unwrap()
            .iter().any(|finding| finding["code"] == "NONFORECAST_IN_POLICY_INPUT"));
    }

    #[test]
    fn issue_time_must_not_follow_first_availability() {
        let mut value = fixture();
        set_research_admissible(&mut value);
        value["sources"][0]["forecast"]["source_issue_time"] = json!("2026-11-19T06:25:00Z");
        value["sources"][0]["forecast"]["first_available_at"] = json!("2026-11-19T06:20:00Z");
        assert_eq!(audit(&value)["disposition"], "INVALID");
    }

    #[test]
    fn wrong_forecast_valid_time_is_blocked() {
        let mut value = fixture();
        set_research_admissible(&mut value);
        value["sources"][0]["forecast"]["valid_time"] = json!("2026-11-19T10:00:00Z");
        assert_eq!(audit(&value)["disposition"], "BLOCKED");
    }

    #[test]
    fn restricted_source_is_blocked() {
        let mut value = fixture();
        value["sources"][0]["rights"]["status"] = json!("restricted");
        assert_eq!(audit(&value)["disposition"], "BLOCKED");
    }

    #[test]
    fn unknown_rights_are_not_assumed_to_allow_research() {
        let mut value = fixture();
        value["sources"][0]["rights"]["status"] = json!("unknown");
        assert_eq!(audit(&value)["disposition"], "INCOMPLETE");
    }

    #[test]
    fn malformed_digest_is_invalid() {
        let mut value = fixture();
        value["sources"][0]["artifact_sha256"] = json!("abc");
        assert_eq!(audit(&value)["disposition"], "INVALID");
    }


    #[test]
    fn missing_first_availability_is_not_inferred_from_retrieval_time() {
        let mut value = fixture();
        set_research_admissible(&mut value);
        value["sources"][0]["forecast"]["first_available_at"] = Value::Null;
        assert_eq!(audit(&value)["disposition"], "INCOMPLETE");
    }

    #[test]
    fn forecast_retrieval_cannot_precede_declared_availability() {
        let mut value = fixture();
        set_research_admissible(&mut value);
        value["sources"][0]["retrieved_at"] = json!("2026-11-19T06:10:00Z");
        assert_eq!(audit(&value)["disposition"], "INVALID");
    }

    #[test]
    fn cycle_issue_plus_lead_must_equal_forecast_valid_time() {
        let mut value = fixture();
        set_research_admissible(&mut value);
        value["sources"][0]["forecast"]["lead_hours"] = json!(6);
        assert_eq!(audit(&value)["disposition"], "INVALID");
    }



    #[test]
    fn availability_category_without_immutable_receipt_is_incomplete() {
        let mut value = fixture();
        set_research_admissible(&mut value);
        value["sources"][0]["forecast"]["availability_evidence_ref"] = Value::Null;
        assert_eq!(audit(&value)["disposition"], "INCOMPLETE");
    }

    #[test]
    fn synthetic_availability_marker_cannot_be_promoted() {
        let mut value = fixture();
        set_research_admissible(&mut value);
        value["sources"][0]["forecast"]["availability_evidence"] = json!("synthetic_fixture");
        assert_eq!(audit(&value)["disposition"], "BLOCKED");
    }

    #[test]
    fn a_source_cannot_be_both_policy_and_evaluator_input() {
        let mut value = fixture();
        value["decisions"][0]["evaluator_context_source_ids"] = json!(["era5-analysis", "gfs-cycle-a"]);
        let report = audit(&value);
        assert_eq!(report["disposition"], "INVALID");
        assert!(report["decision_reports"][0]["findings"].as_array().unwrap()
            .iter().any(|finding| finding["code"] == "SOURCE_ROLE_REUSE"));
    }
}
