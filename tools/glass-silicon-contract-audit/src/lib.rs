use md5::Md5;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::Read;
use std::path::{Component, Path};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct AuditReport {
    pub status: String,
    pub audit_kind: String,
    pub record_id: Option<String>,
    pub findings: Vec<String>,
    pub claim_ceiling: String,
}

impl AuditReport {
    fn new(kind: &str, record_id: Option<String>, ceiling: &str) -> Self {
        Self {
            status: "PASS".to_string(),
            audit_kind: kind.to_string(),
            record_id,
            findings: Vec::new(),
            claim_ceiling: ceiling.to_string(),
        }
    }

    fn fail(&mut self, code: &str, detail: impl AsRef<str>) {
        self.status = "FAIL".to_string();
        self.findings.push(format!("{code}:{}", detail.as_ref()));
    }

    fn finish(&mut self) {
        self.findings.sort();
        self.findings.dedup();
    }

    pub fn exit_code(&self) -> i32 {
        match self.status.as_str() {
            "PASS" => 0,
            "FAIL" => 2,
            _ => 3,
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct FeasibilityCandidateResult {
    pub candidate_id: String,
    pub status: String,
    pub expected_disposition: String,
    pub derived_disposition: String,
    pub blocking_gates: Vec<String>,
    pub unresolved_gates: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct FeasibilityAuditReport {
    pub status: String,
    pub audit_kind: String,
    pub benchmark_id: Option<String>,
    pub candidates: Vec<FeasibilityCandidateResult>,
    pub findings: Vec<String>,
    pub claim_ceiling: String,
}

impl FeasibilityAuditReport {
    pub fn exit_code(&self) -> i32 {
        match self.status.as_str() {
            "PASS" => 0,
            "FAIL" => 2,
            _ => 3,
        }
    }
}

const FEASIBILITY_GATE_ORDER: [&str; 5] = [
    "composition_validity",
    "glass_formation",
    "process_window",
    "joint_product_qualification",
    "manufacturing_feasibility",
];

const FEASIBILITY_GATE_STATUSES: [&str; 7] = [
    "PASS",
    "FAIL",
    "UNKNOWN",
    "NOT_ATTEMPTED",
    "CENSORED",
    "STALE",
    "OUT_OF_DOMAIN",
];

const FEASIBILITY_DISPOSITIONS: [&str; 3] = ["FEASIBLE", "INFEASIBLE", "UNRESOLVED"];

fn feasibility_exact_object_keys(value: &Value, allowed: &[&str], required: &[&str]) -> bool {
    let Some(object) = value.as_object() else { return false };
    object.keys().all(|key| allowed.contains(&key.as_str()))
        && required.iter().all(|key| object.contains_key(*key))
}

/// Deterministic staged feasibility audit for the synthetic benchmark lane.
/// A candidate is FEASIBLE only when every required gate is PASS. Any evidenced
/// hard-gate FAIL makes it INFEASIBLE; otherwise any missing, unknown, stale,
/// out-of-domain, censored, or not-attempted gate yields UNRESOLVED. This checks a
/// declared contract; it does not validate physical evidence or source rights.
pub fn audit_feasibility_corpus(corpus: &Value) -> FeasibilityAuditReport {
    let benchmark_id = object_string(corpus, "benchmark_id").map(ToOwned::to_owned);
    let mut findings = Vec::new();
    let mut results = Vec::new();
    let claim_ceiling = object_string(corpus, "claim_ceiling")
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("Synthetic contract only; no material or process performance is validated.")
        .to_string();

    if !feasibility_exact_object_keys(
        corpus,
        &["benchmark_id", "evidence_class", "claim_ceiling", "gate_order", "candidates"],
        &["benchmark_id", "evidence_class", "claim_ceiling", "gate_order", "candidates"],
    ) {
        findings.push("FEASIBILITY_ROOT_KEYS:missing or unknown top-level fields".to_string());
    }
    if benchmark_id.as_deref().map_or(true, |value| !is_stable_slug(value)) {
        findings.push("FEASIBILITY_BENCHMARK_ID:missing or invalid benchmark_id".to_string());
    }
    if object_string(corpus, "evidence_class") != Some("synthetic") {
        findings.push("FEASIBILITY_EVIDENCE_CLASS:executable control corpus must be explicitly synthetic".to_string());
    }
    if object_string(corpus, "claim_ceiling").map_or(true, |value| value.trim().is_empty()) {
        findings.push("FEASIBILITY_CLAIM_CEILING:claim ceiling must be non-empty".to_string());
    }

    let declared_gates = corpus
        .get("gate_order")
        .and_then(Value::as_array)
        .map(|items| items.iter().filter_map(Value::as_str).collect::<Vec<_>>());
    if declared_gates.as_deref() != Some(FEASIBILITY_GATE_ORDER.as_slice()) {
        findings.push("FEASIBILITY_GATE_ORDER:required ordered gate list is missing or changed".to_string());
    }

    let candidates = corpus.get("candidates").and_then(Value::as_array);
    if candidates.map_or(true, |items| items.is_empty()) {
        findings.push("FEASIBILITY_CANDIDATES:at least one candidate control is required".to_string());
    } else {
        let mut seen_ids = BTreeSet::new();
        let expected_gate_keys: BTreeSet<String> = FEASIBILITY_GATE_ORDER.iter().map(|value| value.to_string()).collect();

        for (index, candidate) in candidates.into_iter().flatten().enumerate() {
            let candidate_id = object_string(candidate, "id").unwrap_or("").to_string();
            let expected = object_string(candidate, "expected_disposition").unwrap_or("").to_string();
            let mut candidate_valid = true;

            if !feasibility_exact_object_keys(candidate, &["id", "expected_disposition", "gates"], &["id", "expected_disposition", "gates"]) {
                findings.push(format!("FEASIBILITY_CANDIDATE_KEYS:index {index} has missing or unknown fields"));
                candidate_valid = false;
            }
            if !is_stable_slug(&candidate_id) || !seen_ids.insert(candidate_id.clone()) {
                findings.push(format!("FEASIBILITY_CANDIDATE_ID:index {index} has missing, invalid or duplicate id"));
                candidate_valid = false;
            }
            if !FEASIBILITY_DISPOSITIONS.contains(&expected.as_str()) {
                findings.push(format!("FEASIBILITY_EXPECTED_DISPOSITION:{candidate_id} has unknown expected disposition"));
                candidate_valid = false;
            }

            let gates = candidate.get("gates").and_then(Value::as_object);
            let actual_gate_keys: BTreeSet<String> = gates
                .into_iter()
                .flat_map(|object| object.keys().cloned())
                .collect();
            if gates.is_none() || actual_gate_keys != expected_gate_keys {
                findings.push(format!("FEASIBILITY_GATE_CENSUS:{candidate_id} must include exactly the five required gates"));
                candidate_valid = false;
            }

            let mut blocking_gates = Vec::new();
            let mut unresolved_gates = Vec::new();
            if let Some(gates) = gates {
                for gate_name in FEASIBILITY_GATE_ORDER {
                    let Some(gate) = gates.get(gate_name) else { continue };
                    if !feasibility_exact_object_keys(
                        gate,
                        &["status", "evidence_class", "basis", "source_ref"],
                        &["status", "evidence_class", "basis"],
                    ) {
                        findings.push(format!("FEASIBILITY_GATE_KEYS:{candidate_id}/{gate_name} has missing or unknown fields"));
                        candidate_valid = false;
                        continue;
                    }
                    let status = object_string(gate, "status").unwrap_or("");
                    let evidence_class = object_string(gate, "evidence_class").unwrap_or("");
                    let basis = object_string(gate, "basis").unwrap_or("");
                    let source_ref = nonempty_string(gate, "source_ref");

                    if !FEASIBILITY_GATE_STATUSES.contains(&status) {
                        findings.push(format!("FEASIBILITY_GATE_STATUS:{candidate_id}/{gate_name} has unknown status"));
                        candidate_valid = false;
                        continue;
                    }
                    if evidence_class != "synthetic" {
                        findings.push(format!("FEASIBILITY_GATE_EVIDENCE_CLASS:{candidate_id}/{gate_name} is not synthetic"));
                        candidate_valid = false;
                    }
                    if basis.trim().is_empty() {
                        findings.push(format!("FEASIBILITY_GATE_BASIS:{candidate_id}/{gate_name} must explain its status"));
                        candidate_valid = false;
                    }
                    if gate.get("source_ref").is_some() && source_ref.is_none() {
                        findings.push(format!("FEASIBILITY_GATE_SOURCE_REF:{candidate_id}/{gate_name} source_ref is empty or not a string"));
                        candidate_valid = false;
                    }
                    if (status == "PASS" || status == "FAIL") && source_ref.is_none() {
                        findings.push(format!("FEASIBILITY_GATE_EVIDENCE_REQUIRED:{candidate_id}/{gate_name} PASS/FAIL needs a traceable evidence reference"));
                        candidate_valid = false;
                    }

                    match status {
                        "FAIL" => blocking_gates.push(gate_name.to_string()),
                        "PASS" => {}
                        _ => unresolved_gates.push(gate_name.to_string()),
                    }
                }
            }

            let derived = if !blocking_gates.is_empty() {
                "INFEASIBLE"
            } else if unresolved_gates.is_empty() && candidate_valid {
                "FEASIBLE"
            } else {
                "UNRESOLVED"
            }.to_string();

            let disposition_match = candidate_valid && expected == derived;
            if candidate_valid && !disposition_match {
                findings.push(format!("FEASIBILITY_DISPOSITION_MISMATCH:{candidate_id} expected {expected} but gates derive {derived}"));
            }
            results.push(FeasibilityCandidateResult {
                candidate_id,
                status: if disposition_match { "PASS".to_string() } else { "FAIL".to_string() },
                expected_disposition: expected,
                derived_disposition: derived,
                blocking_gates,
                unresolved_gates,
            });
        }
    }

    findings.sort();
    findings.dedup();
    FeasibilityAuditReport {
        status: if findings.is_empty() && results.iter().all(|result| result.status == "PASS") { "PASS".to_string() } else { "FAIL".to_string() },
        audit_kind: "synthetic_glass_feasibility_gate_corpus".to_string(),
        benchmark_id,
        candidates: results,
        findings,
        claim_ceiling,
    }
}

fn object_string<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.as_object()?.get(key)?.as_str()
}

fn nonempty_string(value: &Value, key: &str) -> Option<String> {
    object_string(value, key)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned)
}

fn schema_enum_set(schema: &Value, pointer: &str) -> BTreeSet<String> {
    schema.pointer(pointer)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(ToOwned::to_owned)
        .collect()
}

fn is_stable_slug(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with('-')
        && !value.ends_with('-')
        && value
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
}

fn is_hex_digest(value: &str, length: usize) -> bool {
    value.len() == length && value.bytes().all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

/// Check the frozen benchmark's structural and semantic contract invariants.
/// This is intentionally a semantic reference checker, not a general JSON Schema engine.
pub fn audit_corpus(schema: &Value, corpus: &Value) -> AuditReport {
    let mut report = AuditReport::new(
        "benchmark_corpus",
        object_string(corpus, "benchmark_id").map(ToOwned::to_owned),
        "Synthetic contract corpus only; no physical model, material, or plant performance is validated.",
    );

    if object_string(schema, "$schema") != Some("https://json-schema.org/draft/2020-12/schema") {
        report.fail("SCHEMA_DRAFT", "expected JSON Schema Draft 2020-12 declaration");
    }
    if schema.pointer("/properties/schema_version/const").and_then(Value::as_u64) != Some(1) {
        report.fail("SCHEMA_VERSION", "schema must pin schema_version=1");
    }
    if corpus.get("schema_version").and_then(Value::as_u64) != Some(1) {
        report.fail("CORPUS_SCHEMA_VERSION", "corpus schema_version must equal 1");
    }
    if object_string(corpus, "benchmark_id") != Some("glass-silicon-process-benchmark-v0.1") {
        report.fail("BENCHMARK_ID", "unexpected or missing benchmark_id");
    }
    if object_string(corpus, "status") != Some("synthetic_contract_corpus") {
        report.fail("CORPUS_STATUS", "corpus must remain synthetic_contract_corpus");
    }
    if nonempty_string(corpus, "claim_ceiling").is_none() {
        report.fail("CLAIM_CEILING", "missing corpus claim ceiling");
    }

    let invariants = corpus.get("suite_invariants").and_then(Value::as_array);
    if invariants.map_or(true, |items| items.len() < 3 || items.iter().any(|v| v.as_str().map_or(true, str::is_empty))) {
        report.fail("SUITE_INVARIANTS", "at least three non-empty suite invariants are required");
    }

    let fault_classes: BTreeSet<String> = schema
        .pointer("/properties/cases/items/properties/fault_class/oneOf/0/enum")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(ToOwned::to_owned)
        .collect();
    if fault_classes.is_empty() {
        report.fail("FAULT_TAXONOMY", "schema fault_class enum is absent or empty");
    }
    let routes = schema_enum_set(schema, "/properties/cases/items/properties/route/enum");
    let case_types = schema_enum_set(schema, "/properties/cases/items/properties/case_type/enum");
    let dispositions = schema_enum_set(schema, "/properties/cases/items/properties/expected_disposition/enum");
    if routes.is_empty() || case_types.is_empty() || dispositions.is_empty() {
        report.fail("SCHEMA_ENUMS", "route, case_type, and expected_disposition enums must be present");
    }

    let refs = corpus.get("research_references").and_then(Value::as_array);
    let mut seen_reference_ids = BTreeSet::new();
    if refs.map_or(true, |items| items.len() < 3) {
        report.fail("RESEARCH_REFERENCES", "at least three research references are required");
    } else {
        for (index, item) in refs.iter().flat_map(|items| items.iter()).enumerate() {
            let rid = nonempty_string(item, "reference_id");
            if rid.is_none() || !is_stable_slug(rid.as_deref().unwrap_or("")) {
                report.fail("REFERENCE_ID", format!("reference index {index} has missing or invalid slug ID"));
            } else if !seen_reference_ids.insert(rid.clone().unwrap_or_default()) {
                report.fail("DUPLICATE_REFERENCE", format!("duplicate reference ID at index {index}"));
            }
            let year = nonempty_string(item, "published_or_reviewed");
            if year.as_deref().map_or(true, |y| y.len() != 4 || !y.chars().all(|ch| ch.is_ascii_digit())) {
                report.fail("REFERENCE_YEAR", format!("reference index {index} must declare a four-digit year"));
            }
            for field in ["title", "url", "published_or_reviewed", "use", "limitation"] {
                if nonempty_string(item, field).is_none() {
                    report.fail("REFERENCE_FIELD", format!("reference index {index} missing {field}"));
                }
            }
            if object_string(item, "url").map_or(true, |u| !u.starts_with("https://")) {
                report.fail("REFERENCE_URL", format!("reference index {index} must use HTTPS"));
            }
        }
    }

    for required_ref in [
        "scale-glass-active-learning-2026",
        "glass-gfri-feasibility-2026",
        "scale-glass-zenodo-data-2026",
        "glass-futures-ai-glass-2026",
        "silicon-purification-route-review-2026",
        "pnnl-law-glass-validation-2026",
        "wa-silicon-electroreduction-pilot-2026",
    ] {
        if !seen_reference_ids.contains(required_ref) {
            report.fail("RESEARCH_BASELINE_MISSING", format!("missing required source record {required_ref}"));
        }
    }

    let artifact_records = corpus.get("artifact_admission").and_then(Value::as_array);
    if artifact_records.map_or(true, |items| items.is_empty()) {
        report.fail("ARTIFACT_ADMISSION_MANIFEST", "at least one explicit source artifact-admission record is required");
    } else {
        let mut seen_artifact_ids = BTreeSet::new();
        let mut scale_archive_record = false;
        for (index, record) in artifact_records.iter().flat_map(|items| items.iter()).enumerate() {
            let artifact_id = nonempty_string(record, "artifact_id");
            let source_ref = nonempty_string(record, "source_ref");
            if artifact_id.as_deref().map_or(true, |id| !is_stable_slug(id)) {
                report.fail("ARTIFACT_ID", format!("artifact record {index} has missing or invalid ID"));
            } else if !seen_artifact_ids.insert(artifact_id.clone().unwrap_or_default()) {
                report.fail("DUPLICATE_ARTIFACT_ID", format!("duplicate artifact ID at index {index}"));
            }
            if source_ref.as_deref().map_or(true, |id| !seen_reference_ids.contains(id)) {
                report.fail("ARTIFACT_SOURCE_REF", format!("artifact record {index} must point to a declared research reference"));
            }

            let record_status = object_string(record, "record_status").unwrap_or("");
            let rights_status = object_string(record, "rights_status").unwrap_or("");
            let sha256_status = object_string(record, "sha256_status").unwrap_or("");
            let admission_status = object_string(record, "benchmark_admission_status").unwrap_or("");
            if !["publicly_listed_not_downloaded", "downloaded_pending_hash", "downloaded_hashed"].contains(&record_status) {
                report.fail("ARTIFACT_RECORD_STATUS", format!("artifact record {index} has unknown record_status"));
            }
            if !["unresolved", "resolved", "not_applicable"].contains(&rights_status) {
                report.fail("ARTIFACT_RIGHTS_STATUS", format!("artifact record {index} has unknown rights_status"));
            }
            if rights_status == "resolved"
                && (nonempty_string(record, "rights_evidence_ref").is_none()
                    || nonempty_string(record, "rights_reviewed_on").map_or(true, |date| date.len() != 10 || date.chars().nth(4) != Some('-') || date.chars().nth(7) != Some('-')))
            {
                report.fail("ARTIFACT_RIGHTS_PROOF", format!("artifact record {index} claims resolved rights without a rights evidence reference and review date"));
            }
            if !["pending", "complete", "not_applicable"].contains(&sha256_status) {
                report.fail("ARTIFACT_SHA256_STATUS", format!("artifact record {index} has unknown sha256_status"));
            }
            if !["blocked", "admitted", "literature_only"].contains(&admission_status) {
                report.fail("ARTIFACT_ADMISSION_STATUS", format!("artifact record {index} has unknown benchmark_admission_status"));
            }
            if nonempty_string(record, "record_version").is_none()
                || nonempty_string(record, "published_on").map_or(true, |date| date.len() != 10 || date.chars().nth(4) != Some('-') || date.chars().nth(7) != Some('-'))
                || nonempty_string(record, "claim_ceiling").is_none()
            {
                report.fail("ARTIFACT_RECORD_METADATA", format!("artifact record {index} is missing version, date, or claim ceiling"));
            }

            let artifact_files = record.get("files").and_then(Value::as_array);
            if artifact_files.map_or(true, |items| items.is_empty()) {
                report.fail("ARTIFACT_FILES", format!("artifact record {index} has no file records"));
                continue;
            }
            let mut seen_file_names = BTreeSet::new();
            let mut every_file_has_sha256 = true;
            for (file_index, file) in artifact_files.iter().flat_map(|items| items.iter()).enumerate() {
                let filename = nonempty_string(file, "filename");
                if filename.is_none() {
                    report.fail("ARTIFACT_FILENAME", format!("artifact {index} file {file_index} has no filename"));
                } else if !seen_file_names.insert(filename.clone().unwrap_or_default()) {
                    report.fail("DUPLICATE_ARTIFACT_FILENAME", format!("artifact {index} repeats a filename"));
                }
                if nonempty_string(file, "size_display").is_none() {
                    report.fail("ARTIFACT_FILE_SIZE", format!("artifact {index} file {file_index} has no published size metadata"));
                }
                if object_string(file, "published_checksum_algorithm") != Some("MD5")
                    || object_string(file, "published_checksum").map_or(true, |sum| !is_hex_digest(sum, 32))
                {
                    report.fail("PUBLISHED_CHECKSUM", format!("artifact {index} file {file_index} has invalid published MD5 metadata"));
                }
                match file.get("sha256") {
                    Some(Value::String(sum)) if is_hex_digest(sum, 64) => {}
                    Some(Value::Null) => every_file_has_sha256 = false,
                    _ => {
                        every_file_has_sha256 = false;
                        report.fail("ARTIFACT_SHA256", format!("artifact {index} file {file_index} SHA-256 must be null pending calculation or a 64-digit lowercase hex digest"));
                    }
                }
            }

            if record_status == "downloaded_hashed" && (!every_file_has_sha256 || sha256_status != "complete") {
                report.fail("ARTIFACT_HASH_STATUS", format!("artifact record {index} claims download/hash completion without SHA-256 for every file"));
            }
            if admission_status == "admitted"
                && (record_status != "downloaded_hashed" || rights_status != "resolved" || sha256_status != "complete" || !every_file_has_sha256)
            {
                report.fail("ARTIFACT_ADMISSION_UNPROVEN", format!("artifact record {index} is admitted without downloaded bytes, resolved rights, and SHA-256 for every file"));
            }
            if artifact_id.as_deref() == Some("scale-zenodo-18430657-v1") {
                scale_archive_record = true;
                if source_ref.as_deref() != Some("scale-glass-zenodo-data-2026")
                    || object_string(record, "record_version") != Some("v1")
                    || artifact_files.map(|items| items.len()) != Some(9)
                {
                    report.fail("SCALE_ARTIFACT_IDENTITY", "SCALE archive identity must remain pinned to Zenodo v1 and its nine listed files");
                }
                if admission_status == "blocked" {
                    if record_status != "publicly_listed_not_downloaded"
                        || rights_status != "unresolved"
                        || sha256_status != "pending"
                        || every_file_has_sha256
                    {
                        report.fail("SCALE_ARTIFACT_GATE", "blocked SCALE v1 record must truthfully describe unresolved rights and uncomputed hashes");
                    }
                } else if admission_status == "admitted" {
                    if record_status != "downloaded_hashed"
                        || rights_status != "resolved"
                        || sha256_status != "complete"
                        || !every_file_has_sha256
                        || nonempty_string(record, "rights_evidence_ref").is_none()
                        || nonempty_string(record, "rights_reviewed_on").is_none()
                    {
                        report.fail("SCALE_ARTIFACT_GATE", "SCALE v1 may be admitted only after permitted retrieval, rights evidence/review, and SHA-256 for every exact file");
                    }
                } else {
                    report.fail("SCALE_ARTIFACT_GATE", "SCALE archive must be either blocked or fully admitted, not merely labeled literature_only");
                }
            }
        }
        if !scale_archive_record {
            report.fail("SCALE_ARTIFACT_GATE", "missing explicit source artifact-admission record for SCALE Zenodo v1");
        }
    }

    let cases = corpus.get("cases").and_then(Value::as_array);
    if cases.map_or(true, |items| items.len() < 10) {
        report.fail("CASE_COUNT", "at least ten cases are required");
        report.finish();
        return report;
    }

    let mut seen_ids = BTreeSet::new();
    let mut valid_glass = false;
    let mut valid_silicon = false;
    let mut scenario_fragility = false;
    let mut artifact_manifest_control = false;
    let mut calibration_bias_control = false;
    let mut multi_property_control = false;
    let mut pilot_maturity_control = false;
    let mut valid_count = 0usize;
    let mut negative_count = 0usize;
    let mut hold_count = 0usize;

    for (index, case) in cases.iter().flat_map(|items| items.iter()).enumerate() {
        let case_id = nonempty_string(case, "case_id");
        if case_id.is_none() || !is_stable_slug(case_id.as_deref().unwrap_or("")) {
            report.fail("CASE_ID", format!("case index {index} has missing or invalid slug ID"));
        } else if !seen_ids.insert(case_id.clone().unwrap_or_default()) {
            report.fail("DUPLICATE_CASE_ID", format!("duplicate case ID at index {index}"));
        }

        if case.get("synthetic_only").and_then(Value::as_bool) != Some(true)
            || object_string(case, "evidence_status") != Some("synthetic_test")
        {
            report.fail("SYNTHETIC_LABEL", format!("case index {index} is not explicitly synthetic-only"));
        }
        if nonempty_string(case, "claim_ceiling").is_none() {
            report.fail("CASE_CLAIM_CEILING", format!("case index {index} has no claim ceiling"));
        }

        let required = case.get("required_evidence").and_then(Value::as_array);
        if required.map_or(true, |items| items.is_empty() || items.iter().any(|v| v.as_str().map_or(true, |s| s.trim().is_empty()))) {
            report.fail("REQUIRED_EVIDENCE", format!("case index {index} has invalid required_evidence"));
        } else if let Some(items) = required {
            let entries: Vec<String> = items.iter().filter_map(Value::as_str).map(ToOwned::to_owned).collect();
            if entries.iter().collect::<BTreeSet<_>>().len() != entries.len() {
                report.fail("DUPLICATE_REQUIRED_EVIDENCE", format!("case index {index} repeats a required evidence key"));
            }
        }

        let route = object_string(case, "route").unwrap_or("");
        let case_type = object_string(case, "case_type").unwrap_or("");
        let expected = object_string(case, "expected_disposition").unwrap_or("");
        if !routes.contains(route) {
            report.fail("CASE_ROUTE", format!("case index {index} has route outside the declared schema enum"));
        }
        if !case_types.contains(case_type) {
            report.fail("CASE_TYPE_ENUM", format!("case index {index} has case_type outside the declared schema enum"));
        }
        if !dispositions.contains(expected) {
            report.fail("CASE_DISPOSITION_ENUM", format!("case index {index} has expected_disposition outside the declared schema enum"));
        }
        let fault = case.get("fault_class");
        let fault_str = fault.and_then(Value::as_str);
        let fault_is_null = fault.is_some_and(Value::is_null);

        match case_type {
            "valid_control" => {
                valid_count += 1;
                if expected != "accept_contract_only" || !fault_is_null {
                    report.fail("VALID_CONTROL_DISPOSITION", format!("case index {index} must accept contract only and have null fault_class"));
                }
                valid_glass |= route == "glass_container_reference";
                valid_silicon |= route == "silicon_mg_reference";
            }
            "negative_control" => {
                negative_count += 1;
                if expected != "reject" || fault_str.is_none() || !fault_classes.contains(fault_str.unwrap_or("")) {
                    report.fail("NEGATIVE_CONTROL_DISPOSITION", format!("case index {index} must reject with a schema-defined fault class"));
                }
            }
            "hold_control" => {
                hold_count += 1;
                if expected != "abstain" || fault_str.is_none() || !fault_classes.contains(fault_str.unwrap_or("")) {
                    report.fail("HOLD_CONTROL_DISPOSITION", format!("case index {index} must abstain with a schema-defined fault class"));
                }
            }
            _ => report.fail("CASE_TYPE", format!("case index {index} has unknown case_type")),
        }

        if case_id.as_deref() == Some("single-scenario-cost-winner")
            && case_type == "negative_control"
            && fault_str == Some("scenario_fragility_hidden")
            && expected == "reject"
        {
            scenario_fragility = true;
        }

        if case_id.as_deref() == Some("unversioned-reference-artifacts")
            && case_type == "negative_control"
            && fault_str == Some("artifact_manifest_unpinned")
            && expected == "reject"
        {
            artifact_manifest_control = true;
        }

        if case_id.as_deref() == Some("high-r2-unseen-bias-no-calibration")
            && case_type == "hold_control"
            && fault_str == Some("uncalibrated_model_bias_hidden")
            && expected == "abstain"
        {
            calibration_bias_control = true;
        }

        if case_id.as_deref() == Some("single-property-score-bypasses-property-gates")
            && case_type == "negative_control"
            && fault_str == Some("single_property_optimization_bypasses_constraints")
            && expected == "reject"
        {
            multi_property_control = true;
        }

        if case_id.as_deref() == Some("planned-silicon-pilot-treated-as-qualified")
            && case_type == "hold_control"
            && fault_str == Some("planned_pilot_treated_as_demonstrated")
            && expected == "abstain"
        {
            pilot_maturity_control = true;
        }
    }

    if valid_count == 0 || !valid_glass || !valid_silicon {
        report.fail("VALID_CONTROLS", "need accepted contract controls for both glass and MG-Si; reject-all must not pass");
    }
    if negative_count == 0 {
        report.fail("NEGATIVE_CONTROLS", "at least one negative control is required");
    }
    if hold_count == 0 {
        report.fail("HOLD_CONTROLS", "at least one explicit abstention control is required");
    }
    if !scenario_fragility {
        report.fail("ROBUST_SCENARIO_CONTROL", "missing single-scenario-cost-winner negative control");
    }
    if !artifact_manifest_control {
        report.fail("ARTIFACT_MANIFEST_CONTROL", "missing unversioned-reference-artifacts negative control");
    }
    if !calibration_bias_control {
        report.fail("CALIBRATION_BIAS_CONTROL", "missing high-R2/unseen-bias abstention control");
    }
    if !multi_property_control {
        report.fail("MULTI_PROPERTY_CONTROL", "missing single-property-score negative control");
    }
    if !pilot_maturity_control {
        report.fail("PILOT_MATURITY_CONTROL", "missing planned-pilot qualification abstention control");
    }

    report.finish();
    report
}


#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ArtifactFileHash {
    pub filename: String,
    pub size_bytes: Option<u64>,
    pub published_md5: Option<String>,
    pub observed_md5: Option<String>,
    pub md5_matches: Option<bool>,
    pub expected_sha256: Option<String>,
    pub observed_sha256: Option<String>,
    pub sha256_matches: Option<bool>,
    pub file_status: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ArtifactHashReport {
    pub status: String,
    pub audit_kind: String,
    pub artifact_id: String,
    pub source_ref: Option<String>,
    pub source_record_status: Option<String>,
    pub rights_status: Option<String>,
    pub manifest_sha256_status: Option<String>,
    pub manifest_admission_status: Option<String>,
    pub files: Vec<ArtifactFileHash>,
    pub findings: Vec<String>,
    pub claim_ceiling: String,
}

impl ArtifactHashReport {
    fn fail(&mut self, code: &str, detail: impl AsRef<str>) {
        self.status = "FAIL".to_string();
        self.findings.push(format!("{code}:{}", detail.as_ref()));
    }

    fn finish(&mut self) {
        self.files.sort_by(|a, b| a.filename.cmp(&b.filename));
        self.findings.sort();
        self.findings.dedup();
    }

    /// Exit 0 means local files were read and each published MD5 matched.
    /// It does not mean the artifact is rights-cleared or admitted as benchmark data.
    pub fn exit_code(&self) -> i32 {
        if self.status == "FAIL" { 2 } else { 0 }
    }
}

fn is_lower_hex_digest(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn is_safe_artifact_filename(value: &str) -> bool {
    if value.trim().is_empty()
        || value == "."
        || value == ".."
        || value.contains('/')
        || value.contains('\\')
        || value.contains('\0')
    {
        return false;
    }
    let path = Path::new(value);
    !path.is_absolute()
        && path.components().count() == 1
        && matches!(path.components().next(), Some(Component::Normal(_)))
}

fn hash_file_md5_sha256(path: &Path) -> std::io::Result<(u64, String, String)> {
    let mut file = File::open(path)?;
    let mut md5 = Md5::new();
    let mut sha256 = Sha256::new();
    let mut bytes = 0u64;
    let mut buffer = [0u8; 64 * 1024];

    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        bytes = bytes.saturating_add(count as u64);
        md5.update(&buffer[..count]);
        sha256.update(&buffer[..count]);
    }

    Ok((
        bytes,
        format!("{:x}", md5.finalize()),
        format!("{:x}", sha256.finalize()),
    ))
}

/// Hashes files already present in a caller-supplied directory against the
/// manifest's published MD5 values and reports candidate SHA-256 digests.
/// It never downloads files, changes the manifest, resolves rights, or admits data.
/// MD5 is used only for legacy source-record corroboration, not as a security primitive.
pub fn hash_artifact_files(
    corpus: &Value,
    artifact_id: &str,
    directory: &Path,
) -> ArtifactHashReport {
    let mut report = ArtifactHashReport {
        status: "HASHED_PENDING_ADMISSION".to_string(),
        audit_kind: "source_artifact_hash_receipt".to_string(),
        artifact_id: artifact_id.to_string(),
        source_ref: None,
        source_record_status: None,
        rights_status: None,
        manifest_sha256_status: None,
        manifest_admission_status: None,
        files: Vec::new(),
        findings: Vec::new(),
        claim_ceiling: "Hashes caller-supplied local files only. This receipt does not establish reuse rights, prove a source's authorship, update the corpus manifest, validate scientific results, or admit the archive as benchmark data.".to_string(),
    };

    let records = corpus.get("artifact_admission").and_then(Value::as_array);
    let Some(record) = records.and_then(|items| {
        items.iter().find(|item| item.get("artifact_id").and_then(Value::as_str) == Some(artifact_id))
    }) else {
        report.fail("ARTIFACT_RECORD_MISSING", "artifact_id is not present in artifact_admission");
        report.finish();
        return report;
    };

    report.source_ref = object_string(record, "source_ref").map(ToOwned::to_owned);
    report.source_record_status = object_string(record, "record_status").map(ToOwned::to_owned);
    report.rights_status = object_string(record, "rights_status").map(ToOwned::to_owned);
    report.manifest_sha256_status = object_string(record, "sha256_status").map(ToOwned::to_owned);
    report.manifest_admission_status = object_string(record, "benchmark_admission_status").map(ToOwned::to_owned);

    let Some(manifest_files) = record.get("files").and_then(Value::as_array) else {
        report.fail("ARTIFACT_FILE_LIST_MISSING", "artifact record has no files array");
        report.finish();
        return report;
    };
    if manifest_files.is_empty() {
        report.fail("ARTIFACT_FILE_LIST_EMPTY", "artifact record has an empty files array");
        report.finish();
        return report;
    }

    let canonical_directory = match fs::canonicalize(directory) {
        Ok(path) if path.is_dir() => path,
        _ => {
            report.fail("ARTIFACT_DIRECTORY_UNAVAILABLE", "local artifact directory cannot be resolved");
            report.finish();
            return report;
        }
    };

    for manifest_file in manifest_files {
        let findings_before_file = report.findings.len();
        let filename = nonempty_string(manifest_file, "filename").unwrap_or_default();
        let expected_md5 = nonempty_string(manifest_file, "published_checksum");
        let expected_sha256 = match manifest_file.get("sha256") {
            Some(Value::Null) | None => None,
            Some(Value::String(value)) => Some(value.clone()),
            _ => {
                report.fail("ARTIFACT_SHA256_TYPE", format!("{filename}: sha256 must be a string or null"));
                None
            }
        };

        let mut file_receipt = ArtifactFileHash {
            filename: filename.clone(),
            size_bytes: None,
            published_md5: expected_md5.clone(),
            observed_md5: None,
            md5_matches: None,
            expected_sha256: expected_sha256.clone(),
            observed_sha256: None,
            sha256_matches: None,
            file_status: "UNRESOLVED".to_string(),
        };

        if !is_safe_artifact_filename(&filename) {
            report.fail("ARTIFACT_FILENAME_UNSAFE", format!("unsafe artifact filename: {filename}"));
            file_receipt.file_status = "FAIL".to_string();
            report.files.push(file_receipt);
            continue;
        }

        if object_string(manifest_file, "published_checksum_algorithm") != Some("MD5")
            || expected_md5.as_deref().map_or(true, |value| !is_lower_hex_digest(value, 32))
        {
            report.fail("ARTIFACT_PUBLISHED_MD5_INVALID", format!("{filename}: expected a valid published MD5"));
        }

        if expected_sha256.as_deref().is_some_and(|value| !is_lower_hex_digest(value, 64)) {
            report.fail("ARTIFACT_EXPECTED_SHA256_INVALID", format!("{filename}: expected SHA-256 is malformed"));
        }

        let candidate_path = canonical_directory.join(&filename);
        let canonical_path = match fs::canonicalize(&candidate_path) {
            Ok(path) => path,
            Err(_) => {
                report.fail("ARTIFACT_FILE_UNAVAILABLE", format!("local artifact file not found or unreadable: {filename}"));
                file_receipt.file_status = "FAIL".to_string();
                report.files.push(file_receipt);
                continue;
            }
        };

        if !canonical_path.starts_with(&canonical_directory) || !canonical_path.is_file() {
            report.fail("ARTIFACT_PATH_ESCAPE", format!("resolved artifact file escapes the supplied directory or is not a file: {filename}"));
            file_receipt.file_status = "FAIL".to_string();
            report.files.push(file_receipt);
            continue;
        }

        match hash_file_md5_sha256(&canonical_path) {
            Ok((size, observed_md5, observed_sha256)) => {
                file_receipt.size_bytes = Some(size);
                file_receipt.observed_md5 = Some(observed_md5.clone());
                file_receipt.observed_sha256 = Some(observed_sha256.clone());

                if let Some(expected) = expected_md5.as_deref().filter(|value| is_lower_hex_digest(value, 32)) {
                    let matches = observed_md5 == expected;
                    file_receipt.md5_matches = Some(matches);
                    if !matches {
                        report.fail("ARTIFACT_PUBLISHED_MD5_MISMATCH", format!("{filename}: locally computed MD5 does not match source record"));
                    }
                }

                if let Some(expected) = expected_sha256.as_deref().filter(|value| is_lower_hex_digest(value, 64)) {
                    let matches = observed_sha256 == expected;
                    file_receipt.sha256_matches = Some(matches);
                    if !matches {
                        report.fail("ARTIFACT_PINNED_SHA256_MISMATCH", format!("{filename}: locally computed SHA-256 differs from the existing manifest pin"));
                    }
                }

                if report.findings.len() == findings_before_file {
                    file_receipt.file_status = "HASHED".to_string();
                } else {
                    file_receipt.file_status = "FAIL".to_string();
                }
                report.files.push(file_receipt);
            }
            Err(_) => {
                report.fail("ARTIFACT_FILE_READ_FAILED", format!("failed while reading artifact file: {filename}"));
                file_receipt.file_status = "FAIL".to_string();
                report.files.push(file_receipt);
            }
        }
    }

    if report.status != "FAIL" {
        report.status = "HASHED_PENDING_ADMISSION".to_string();
        if report.rights_status.as_deref() != Some("resolved") {
            report.findings.push("RIGHTS_REVIEW_STILL_REQUIRED".to_string());
        }
        report.findings.push("MANIFEST_NOT_MODIFIED".to_string());
        report.findings.push("HASH_RECEIPT_DOES_NOT_ADMIT_DATA".to_string());
    }
    report.finish();
    report
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct MassLedger {
    schema_version: Option<u64>,
    record_id: Option<String>,
    evidence_status: Option<String>,
    stream_census_complete: Option<bool>,
    mass_unit: Option<String>,
    inputs: Option<BTreeMap<String, f64>>,
    outputs: Option<BTreeMap<String, f64>>,
    inventory_change: Option<BTreeMap<String, f64>>,
    tolerance: Option<BTreeMap<String, f64>>,
    tolerance_basis: Option<BTreeMap<String, String>>,
    claim_ceiling: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct MassBalanceReport {
    pub status: String,
    pub audit_kind: String,
    pub record_id: Option<String>,
    pub mass_unit: Option<String>,
    pub residuals: BTreeMap<String, f64>,
    pub findings: Vec<String>,
    pub claim_ceiling: String,
}

impl MassBalanceReport {
    fn fail(&mut self, code: &str, detail: impl AsRef<str>) {
        self.status = "FAIL".to_string();
        self.findings.push(format!("{code}:{}", detail.as_ref()));
    }

    fn unresolved(&mut self, code: &str, detail: impl AsRef<str>) {
        if self.status == "PASS" {
            self.status = "UNRESOLVED".to_string();
        }
        self.findings.push(format!("{code}:{}", detail.as_ref()));
    }

    fn finish(&mut self) {
        self.findings.sort();
        self.findings.dedup();
    }

    pub fn exit_code(&self) -> i32 {
        match self.status.as_str() {
            "PASS" => 0,
            "FAIL" => 2,
            _ => 3,
        }
    }
}

/// Audit elemental masses in a single, declared mass unit.
/// inventory_change means final inventory minus initial inventory.
/// This checks supplied arithmetic, not whether the declared process boundary is complete in reality.
pub fn audit_mass_balance(ledger: &Value) -> MassBalanceReport {
    let parsed: Result<MassLedger, _> = serde_json::from_value(ledger.clone());
    let ceiling = "Elemental balance arithmetic only; no chemical model, complete physical stream census, calibrated measurement, or manufactured product is certified.".to_string();
    let mut report = MassBalanceReport {
        status: "PASS".to_string(),
        audit_kind: "elemental_mass_balance".to_string(),
        record_id: None,
        mass_unit: None,
        residuals: BTreeMap::new(),
        findings: Vec::new(),
        claim_ceiling: ceiling,
    };

    let ledger = match parsed {
        Ok(v) => v,
        Err(error) => {
            report.fail("LEDGER_PARSE", error.to_string());
            report.finish();
            return report;
        }
    };
    report.record_id = ledger.record_id.clone();
    report.mass_unit = ledger.mass_unit.clone();

    if ledger.schema_version != Some(1) {
        report.fail("SCHEMA_VERSION", "schema_version must equal 1");
    }
    if ledger.record_id.as_deref().map_or(true, |s| s.trim().is_empty()) {
        report.unresolved("RECORD_ID", "record_id is required to identify the audited evidence");
    }
    if ledger.claim_ceiling.as_deref().map_or(true, |s| s.trim().is_empty()) {
        report.unresolved("CLAIM_CEILING", "ledger must state the limits of its claim");
    }
    if ledger.evidence_status.as_deref().map_or(true, |s| !["measured", "published", "simulated", "synthetic_test"].contains(&s)) {
        report.unresolved("EVIDENCE_STATUS", "unknown or missing evidence class");
    }
    if ledger.stream_census_complete != Some(true) {
        report.unresolved("STREAM_CENSUS", "complete boundary stream census is not explicitly established");
    }
    let unit = ledger.mass_unit.as_deref().unwrap_or("");
    if !["kg", "g", "mg", "t"].contains(&unit) {
        report.unresolved("MASS_UNIT", "mass_unit must be one shared supported unit: kg, g, mg, or t");
    }

    let (inputs, outputs, delta, tolerance, tolerance_basis) = match (
        ledger.inputs.as_ref(),
        ledger.outputs.as_ref(),
        ledger.inventory_change.as_ref(),
        ledger.tolerance.as_ref(),
        ledger.tolerance_basis.as_ref(),
    ) {
        (Some(i), Some(o), Some(d), Some(t), Some(b)) => (i, o, d, t, b),
        _ => {
            report.unresolved("MISSING_MASS_MAP", "inputs, outputs, inventory_change, tolerance, and tolerance_basis maps are all required");
            report.finish();
            return report;
        }
    };

    let input_keys: BTreeSet<&String> = inputs.keys().collect();
    let output_keys: BTreeSet<&String> = outputs.keys().collect();
    let delta_keys: BTreeSet<&String> = delta.keys().collect();
    let tolerance_keys: BTreeSet<&String> = tolerance.keys().collect();
    let basis_keys: BTreeSet<&String> = tolerance_basis.keys().collect();
    if input_keys != output_keys
        || input_keys != delta_keys
        || input_keys != tolerance_keys
        || input_keys != basis_keys
    {
        report.unresolved("ELEMENT_CENSUS", "inputs, outputs, inventory_change, tolerance, and tolerance_basis must contain exactly the same element keys");
        report.finish();
        return report;
    }
    if inputs.is_empty() {
        report.unresolved("EMPTY_ELEMENT_CENSUS", "at least one element must be accounted for");
        report.finish();
        return report;
    }

    for element in inputs.keys() {
        let input = inputs[element];
        let output = outputs[element];
        let change = delta[element];
        let allowed = tolerance[element];
        let basis = tolerance_basis[element].trim();
        if basis.is_empty() {
            report.unresolved("TOLERANCE_BASIS", format!("missing tolerance justification for element {element}"));
        }

        if !input.is_finite() || !output.is_finite() || !change.is_finite() || !allowed.is_finite() {
            report.fail("NONFINITE_VALUE", format!("non-finite quantity for element {element}"));
            continue;
        }
        if input < 0.0 || output < 0.0 {
            report.fail("NEGATIVE_PHYSICAL_MASS", format!("negative input/output mass for element {element}"));
            continue;
        }
        if allowed < 0.0 {
            report.fail("NEGATIVE_TOLERANCE", format!("negative tolerance for element {element}"));
            continue;
        }

        let residual = input - output - change;
        if !residual.is_finite() {
            report.fail("NONFINITE_RESIDUAL", format!("non-finite computed residual for element {element}"));
            continue;
        }
        report.residuals.insert(element.clone(), residual);
        if residual.abs() > allowed {
            report.fail(
                "MASS_BALANCE_OUT_OF_TOLERANCE",
                format!("element {element} residual {residual} exceeds tolerance {allowed}"),
            );
        }
    }

    report.finish();
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temporary_directory() -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after UNIX epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "glass-silicon-contract-audit-{}-{stamp}",
            std::process::id()
        ));
        std::fs::create_dir(&path).expect("temporary test directory should be created");
        path
    }

    fn local_hash_corpus(filename: &str, published_md5: &str, sha256: Value) -> Value {
        json!({
            "artifact_admission": [{
                "artifact_id": "unit-test-artifact",
                "source_ref": "unit-test-source",
                "record_status": "publicly_listed_not_downloaded",
                "rights_status": "unresolved",
                "sha256_status": "pending",
                "benchmark_admission_status": "blocked",
                "files": [{
                    "filename": filename,
                    "size_display": "3 Bytes",
                    "published_checksum_algorithm": "MD5",
                    "published_checksum": published_md5,
                    "sha256": sha256
                }]
            }]
        })
    }

    fn ledger() -> Value {
        json!({
            "schema_version": 1,
            "record_id": "synthetic-balanced-case",
            "evidence_status": "synthetic_test",
            "stream_census_complete": true,
            "mass_unit": "kg",
            "inputs": {"Si": 20.0, "O": 30.0},
            "outputs": {"Si": 20.0, "O": 29.5},
            "inventory_change": {"Si": 0.0, "O": 0.5},
            "tolerance": {"Si": 0.01, "O": 0.01},
            "tolerance_basis": {"Si": "Synthetic contract test tolerance only.", "O": "Synthetic contract test tolerance only."},
            "claim_ceiling": "Synthetic arithmetic fixture only."
        })
    }

    #[test]
    fn corpus_has_required_controls() {
        let schema: Value = serde_json::from_str(include_str!("../../../docs/tech/glass-silicon-process-benchmark-v0.1.schema.json")).unwrap();
        let corpus: Value = serde_json::from_str(include_str!("../../../docs/tech/glass-silicon-process-benchmark-v0.1.json")).unwrap();
        let report = audit_corpus(&schema, &corpus);
        assert_eq!(report.status, "PASS", "{:?}", report.findings);
    }

    #[test]
    fn balanced_mass_ledger_passes_arithmetic() {
        let report = audit_mass_balance(&ledger());
        assert_eq!(report.status, "PASS", "{:?}", report.findings);
        assert_eq!(report.residuals.get("Si"), Some(&0.0));
        assert_eq!(report.residuals.get("O"), Some(&0.0));
    }

    #[test]
    fn out_of_tolerance_residual_fails_closed() {
        let mut value = ledger();
        value["outputs"]["Si"] = json!(19.0);
        let report = audit_mass_balance(&value);
        assert_eq!(report.status, "FAIL");
        assert!(report.findings.iter().any(|v| v.starts_with("MASS_BALANCE_OUT_OF_TOLERANCE:")));
    }

    #[test]
    fn incomplete_stream_census_is_unresolved() {
        let mut value = ledger();
        value["stream_census_complete"] = json!(false);
        let report = audit_mass_balance(&value);
        assert_eq!(report.status, "UNRESOLVED");
        assert!(report.findings.iter().any(|v| v.starts_with("STREAM_CENSUS:")));
    }

    #[test]
    fn mismatched_element_keys_are_unresolved() {
        let mut value = ledger();
        value["outputs"] = json!({"Si": 20.0});
        let report = audit_mass_balance(&value);
        assert_eq!(report.status, "UNRESOLVED");
        assert!(report.findings.iter().any(|v| v.starts_with("ELEMENT_CENSUS:")));
    }

    #[test]
    fn negative_input_mass_fails() {
        let mut value = ledger();
        value["inputs"]["Si"] = json!(-1.0);
        let report = audit_mass_balance(&value);
        assert_eq!(report.status, "FAIL");
        assert!(report.findings.iter().any(|v| v.starts_with("NEGATIVE_PHYSICAL_MASS:")));
    }

    #[test]
    fn negative_inventory_change_can_be_valid() {
        let mut value = ledger();
        value["inputs"]["Si"] = json!(20.0);
        value["outputs"]["Si"] = json!(21.0);
        value["inventory_change"]["Si"] = json!(-1.0);
        let report = audit_mass_balance(&value);
        assert_eq!(report.status, "PASS", "{:?}", report.findings);
    }

    #[test]
    fn unrecognized_evidence_status_is_unresolved() {
        let mut value = ledger();
        value["evidence_status"] = json!("unknown");
        let report = audit_mass_balance(&value);
        assert_eq!(report.status, "UNRESOLVED");
        assert!(report.findings.iter().any(|v| v.starts_with("EVIDENCE_STATUS:")));
    }

    #[test]
    fn missing_tolerance_is_unresolved() {
        let mut value = ledger();
        let _ = value.as_object_mut().unwrap().remove("tolerance");
        let report = audit_mass_balance(&value);
        assert_eq!(report.status, "UNRESOLVED");
        assert!(report.findings.iter().any(|v| v.starts_with("MISSING_MASS_MAP:")));
    }

    #[test]
    fn missing_tolerance_basis_is_unresolved() {
        let mut value = ledger();
        let _ = value.as_object_mut().unwrap().remove("tolerance_basis");
        let report = audit_mass_balance(&value);
        assert_eq!(report.status, "UNRESOLVED");
        assert!(report.findings.iter().any(|v| v.starts_with("MISSING_MASS_MAP:")));
    }

    #[test]
    fn empty_tolerance_basis_is_unresolved() {
        let mut value = ledger();
        value["tolerance_basis"]["Si"] = json!("   ");
        let report = audit_mass_balance(&value);
        assert_eq!(report.status, "UNRESOLVED");
        assert!(report.findings.iter().any(|v| v.starts_with("TOLERANCE_BASIS:")));
    }

    #[test]
    fn unknown_mass_ledger_fields_fail() {
        let mut value = ledger();
        value["unmodeled_credit"] = json!(0.0);
        let report = audit_mass_balance(&value);
        assert_eq!(report.status, "FAIL");
        assert!(report.findings.iter().any(|v| v.starts_with("LEDGER_PARSE:")));
    }

    #[test]
    fn nonfinite_computed_residual_fails() {
        let mut value = ledger();
        value["inputs"]["Si"] = json!(f64::MAX);
        value["outputs"]["Si"] = json!(0.0);
        value["inventory_change"]["Si"] = json!(-f64::MAX);
        let report = audit_mass_balance(&value);
        assert_eq!(report.status, "FAIL");
        assert!(report.findings.iter().any(|v| v.starts_with("NONFINITE_RESIDUAL:")));
    }

    #[test]
    fn corpus_missing_robustness_control_fails() {
        let schema: Value = serde_json::from_str(include_str!("../../../docs/tech/glass-silicon-process-benchmark-v0.1.schema.json")).unwrap();
        let mut corpus: Value = serde_json::from_str(include_str!("../../../docs/tech/glass-silicon-process-benchmark-v0.1.json")).unwrap();
        corpus["cases"].as_array_mut().unwrap().retain(|case| case["case_id"].as_str() != Some("single-scenario-cost-winner"));
        let report = audit_corpus(&schema, &corpus);
        assert_eq!(report.status, "FAIL");
        assert!(report.findings.iter().any(|v| v.starts_with("ROBUST_SCENARIO_CONTROL:")));
    }


    #[test]
    fn corpus_missing_artifact_manifest_control_fails() {
        let schema: Value = serde_json::from_str(include_str!("../../../docs/tech/glass-silicon-process-benchmark-v0.1.schema.json")).unwrap();
        let mut corpus: Value = serde_json::from_str(include_str!("../../../docs/tech/glass-silicon-process-benchmark-v0.1.json")).unwrap();
        corpus["cases"].as_array_mut().unwrap().retain(|case| case["case_id"].as_str() != Some("unversioned-reference-artifacts"));
        let report = audit_corpus(&schema, &corpus);
        assert_eq!(report.status, "FAIL");
        assert!(report.findings.iter().any(|v| v.starts_with("ARTIFACT_MANIFEST_CONTROL:")));
    }


    #[test]
    fn corpus_missing_calibration_bias_control_fails() {
        let schema: Value = serde_json::from_str(include_str!("../../../docs/tech/glass-silicon-process-benchmark-v0.1.schema.json")).unwrap();
        let mut corpus: Value = serde_json::from_str(include_str!("../../../docs/tech/glass-silicon-process-benchmark-v0.1.json")).unwrap();
        corpus["cases"].as_array_mut().unwrap().retain(|case| case["case_id"].as_str() != Some("high-r2-unseen-bias-no-calibration"));
        let report = audit_corpus(&schema, &corpus);
        assert_eq!(report.status, "FAIL");
        assert!(report.findings.iter().any(|v| v.starts_with("CALIBRATION_BIAS_CONTROL:")));
    }

    #[test]
    fn corpus_missing_sota_reference_fails() {
        let schema: Value = serde_json::from_str(include_str!("../../../docs/tech/glass-silicon-process-benchmark-v0.1.schema.json")).unwrap();
        let mut corpus: Value = serde_json::from_str(include_str!("../../../docs/tech/glass-silicon-process-benchmark-v0.1.json")).unwrap();
        corpus["research_references"].as_array_mut().unwrap().retain(|reference| reference["reference_id"].as_str() != Some("scale-glass-active-learning-2026"));
        let report = audit_corpus(&schema, &corpus);
        assert_eq!(report.status, "FAIL");
        assert!(report.findings.iter().any(|v| v.starts_with("RESEARCH_BASELINE_MISSING:")));
    }

    #[test]
    fn corpus_missing_multi_property_control_fails() {
        let schema: Value = serde_json::from_str(include_str!("../../../docs/tech/glass-silicon-process-benchmark-v0.1.schema.json")).unwrap();
        let mut corpus: Value = serde_json::from_str(include_str!("../../../docs/tech/glass-silicon-process-benchmark-v0.1.json")).unwrap();
        corpus["cases"].as_array_mut().unwrap().retain(|case| case["case_id"].as_str() != Some("single-property-score-bypasses-property-gates"));
        let report = audit_corpus(&schema, &corpus);
        assert_eq!(report.status, "FAIL");
        assert!(report.findings.iter().any(|v| v.starts_with("MULTI_PROPERTY_CONTROL:")));
    }

    #[test]
    fn corpus_missing_pilot_maturity_control_fails() {
        let schema: Value = serde_json::from_str(include_str!("../../../docs/tech/glass-silicon-process-benchmark-v0.1.schema.json")).unwrap();
        let mut corpus: Value = serde_json::from_str(include_str!("../../../docs/tech/glass-silicon-process-benchmark-v0.1.json")).unwrap();
        corpus["cases"].as_array_mut().unwrap().retain(|case| case["case_id"].as_str() != Some("planned-silicon-pilot-treated-as-qualified"));
        let report = audit_corpus(&schema, &corpus);
        assert_eq!(report.status, "FAIL");
        assert!(report.findings.iter().any(|v| v.starts_with("PILOT_MATURITY_CONTROL:")));
    }

    #[test]
    fn sha256_and_md5_match_known_abc_vector() {
        let directory = temporary_directory();
        let path = directory.join("example.txt");
        std::fs::write(&path, b"abc").expect("test file should be written");
        let (size, md5, sha256) = hash_file_md5_sha256(&path).expect("test file should hash");
        assert_eq!(size, 3);
        assert_eq!(md5, "900150983cd24fb0d6963f7d28e17f72");
        assert_eq!(sha256, "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
        std::fs::remove_dir_all(directory).expect("test directory should be removed");
    }

    #[test]
    fn artifact_hash_receipt_matches_md5_but_does_not_admit() {
        let directory = temporary_directory();
        std::fs::write(directory.join("example.txt"), b"abc").expect("test file should be written");
        let corpus = local_hash_corpus("example.txt", "900150983cd24fb0d6963f7d28e17f72", Value::Null);
        let report = hash_artifact_files(&corpus, "unit-test-artifact", &directory);
        assert_eq!(report.status, "HASHED_PENDING_ADMISSION");
        assert_eq!(report.exit_code(), 0);
        assert_eq!(report.files.len(), 1);
        assert_eq!(report.files[0].md5_matches, Some(true));
        assert_eq!(report.files[0].size_bytes, Some(3));
        assert_eq!(report.files[0].observed_sha256.as_deref(), Some("ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"));
        assert_eq!(report.rights_status.as_deref(), Some("unresolved"));
        assert_eq!(report.manifest_admission_status.as_deref(), Some("blocked"));
        assert!(report.findings.iter().any(|finding| finding == "HASH_RECEIPT_DOES_NOT_ADMIT_DATA"));
        assert_eq!(corpus["artifact_admission"][0]["files"][0]["sha256"], Value::Null);
        std::fs::remove_dir_all(directory).expect("test directory should be removed");
    }

    #[test]
    fn artifact_hash_receipt_fails_on_published_md5_mismatch() {
        let directory = temporary_directory();
        std::fs::write(directory.join("example.txt"), b"abc").expect("test file should be written");
        let corpus = local_hash_corpus("example.txt", "00000000000000000000000000000000", Value::Null);
        let report = hash_artifact_files(&corpus, "unit-test-artifact", &directory);
        assert_eq!(report.status, "FAIL");
        assert_eq!(report.exit_code(), 2);
        assert_eq!(report.files[0].md5_matches, Some(false));
        assert!(report.findings.iter().any(|finding| finding.starts_with("ARTIFACT_PUBLISHED_MD5_MISMATCH:")));
        std::fs::remove_dir_all(directory).expect("test directory should be removed");
    }

    #[test]
    fn artifact_hash_receipt_rejects_path_traversal() {
        let directory = temporary_directory();
        let corpus = local_hash_corpus("../example.txt", "900150983cd24fb0d6963f7d28e17f72", Value::Null);
        let report = hash_artifact_files(&corpus, "unit-test-artifact", &directory);
        assert_eq!(report.status, "FAIL");
        assert!(report.findings.iter().any(|finding| finding.starts_with("ARTIFACT_FILENAME_UNSAFE:")));
        std::fs::remove_dir_all(directory).expect("test directory should be removed");
    }

    #[test]
    fn artifact_hash_receipt_fails_on_pinned_sha256_mismatch() {
        let directory = temporary_directory();
        std::fs::write(directory.join("example.txt"), b"abc").expect("test file should be written");
        let corpus = local_hash_corpus("example.txt", "900150983cd24fb0d6963f7d28e17f72", json!("0000000000000000000000000000000000000000000000000000000000000000"));
        let report = hash_artifact_files(&corpus, "unit-test-artifact", &directory);
        assert_eq!(report.status, "FAIL");
        assert_eq!(report.files[0].sha256_matches, Some(false));
        assert!(report.findings.iter().any(|finding| finding.starts_with("ARTIFACT_PINNED_SHA256_MISMATCH:")));
        std::fs::remove_dir_all(directory).expect("test directory should be removed");
    }

    #[test]
    fn artifact_hash_receipt_accepts_matching_pinned_sha256() {
        let directory = temporary_directory();
        std::fs::write(directory.join("example.txt"), b"abc").expect("test file should be written");
        let corpus = local_hash_corpus(
            "example.txt",
            "900150983cd24fb0d6963f7d28e17f72",
            json!("ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad")
        );
        let report = hash_artifact_files(&corpus, "unit-test-artifact", &directory);
        assert_eq!(report.status, "HASHED_PENDING_ADMISSION");
        assert_eq!(report.files[0].md5_matches, Some(true));
        assert_eq!(report.files[0].sha256_matches, Some(true));
        std::fs::remove_dir_all(directory).expect("test directory should be removed");
    }

    #[test]
    fn corpus_missing_pnnl_and_pilot_sources_fails() {
        let schema: Value = serde_json::from_str(include_str!("../../../docs/tech/glass-silicon-process-benchmark-v0.1.schema.json")).unwrap();
        let mut corpus: Value = serde_json::from_str(include_str!("../../../docs/tech/glass-silicon-process-benchmark-v0.1.json")).unwrap();
        corpus["research_references"].as_array_mut().unwrap().retain(|reference| {
            !matches!(
                reference["reference_id"].as_str(),
                Some("pnnl-law-glass-validation-2026" | "wa-silicon-electroreduction-pilot-2026")
            )
        });
        let report = audit_corpus(&schema, &corpus);
        assert_eq!(report.status, "FAIL");
        assert!(report.findings.iter().any(|v| v.starts_with("RESEARCH_BASELINE_MISSING:")));
    }

    #[test]
    fn corpus_wrong_schema_version_fails() {
        let schema: Value = serde_json::from_str(include_str!("../../../docs/tech/glass-silicon-process-benchmark-v0.1.schema.json")).unwrap();
        let mut corpus: Value = serde_json::from_str(include_str!("../../../docs/tech/glass-silicon-process-benchmark-v0.1.json")).unwrap();
        corpus["schema_version"] = json!(2);
        let report = audit_corpus(&schema, &corpus);
        assert_eq!(report.status, "FAIL");
        assert!(report.findings.iter().any(|v| v.starts_with("CORPUS_SCHEMA_VERSION:")));
    }

    #[test]
    fn corpus_route_outside_schema_enum_fails() {
        let schema: Value = serde_json::from_str(include_str!("../../../docs/tech/glass-silicon-process-benchmark-v0.1.schema.json")).unwrap();
        let mut corpus: Value = serde_json::from_str(include_str!("../../../docs/tech/glass-silicon-process-benchmark-v0.1.json")).unwrap();
        corpus["cases"][0]["route"] = json!("invented-route");
        let report = audit_corpus(&schema, &corpus);
        assert_eq!(report.status, "FAIL");
        assert!(report.findings.iter().any(|v| v.starts_with("CASE_ROUTE:")));
    }

    #[test]
    fn corpus_missing_artifact_admission_manifest_fails() {
        let schema: Value = serde_json::from_str(include_str!("../../../docs/tech/glass-silicon-process-benchmark-v0.1.schema.json")).unwrap();
        let mut corpus: Value = serde_json::from_str(include_str!("../../../docs/tech/glass-silicon-process-benchmark-v0.1.json")).unwrap();
        corpus.as_object_mut().unwrap().remove("artifact_admission");
        let report = audit_corpus(&schema, &corpus);
        assert_eq!(report.status, "FAIL");
        assert!(report.findings.iter().any(|v| v.starts_with("ARTIFACT_ADMISSION_MANIFEST:")));
    }

    #[test]
    fn unhashed_artifact_cannot_be_marked_admitted() {
        let schema: Value = serde_json::from_str(include_str!("../../../docs/tech/glass-silicon-process-benchmark-v0.1.schema.json")).unwrap();
        let mut corpus: Value = serde_json::from_str(include_str!("../../../docs/tech/glass-silicon-process-benchmark-v0.1.json")).unwrap();
        corpus["artifact_admission"][0]["benchmark_admission_status"] = json!("admitted");
        let report = audit_corpus(&schema, &corpus);
        assert_eq!(report.status, "FAIL");
        assert!(report.findings.iter().any(|v| v.starts_with("ARTIFACT_ADMISSION_UNPROVEN:")));
        assert!(report.findings.iter().any(|v| v.starts_with("SCALE_ARTIFACT_GATE:")));
    }

    #[test]
    fn rights_cannot_be_resolved_without_review_evidence() {
        let schema: Value = serde_json::from_str(include_str!("../../../docs/tech/glass-silicon-process-benchmark-v0.1.schema.json")).unwrap();
        let mut corpus: Value = serde_json::from_str(include_str!("../../../docs/tech/glass-silicon-process-benchmark-v0.1.json")).unwrap();
        corpus["artifact_admission"][0]["rights_status"] = json!("resolved");
        let report = audit_corpus(&schema, &corpus);
        assert_eq!(report.status, "FAIL");
        assert!(report.findings.iter().any(|v| v.starts_with("ARTIFACT_RIGHTS_PROOF:")));
    }
}


// --- GLASS-SILICON-DECISION-VALUE-001: finite-pool regret evaluator ---

#[derive(Debug, Clone)]
struct DecisionAction {
    id: String,
    utility_by_scenario: BTreeMap<String, f64>,
    feasible_by_scenario: BTreeMap<String, bool>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct DecisionMeasurementReport {
    pub measurement_id: String,
    pub status: String,
    pub cost_units: f64,
    pub duration_seconds: f64,
    pub within_budget: bool,
    pub observation_groups: usize,
    pub post_measurement_worst_case_regret: Option<f64>,
    pub gross_worst_case_regret_reduction: Option<f64>,
    pub findings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct DecisionValueReport {
    pub status: String,
    pub audit_kind: String,
    pub benchmark_id: Option<String>,
    pub utility_unit: Option<String>,
    pub cost_unit: Option<String>,
    pub pre_measurement_worst_case_regret: Option<f64>,
    pub max_measurement_cost: Option<f64>,
    pub max_duration_seconds: Option<f64>,
    pub measurements: Vec<DecisionMeasurementReport>,
    pub pareto_frontier: Vec<String>,
    pub oracle_measurement_id: Option<String>,
    pub oracle_worst_case_regret_after: Option<f64>,
    pub oracle_gross_regret_reduction: Option<f64>,
    pub findings: Vec<String>,
    pub claim_ceiling: String,
}

impl DecisionValueReport {
    pub fn exit_code(&self) -> i32 {
        match self.status.as_str() {
            "PASS" => 0,
            "FAIL" => 2,
            _ => 3,
        }
    }
}

fn decision_invalid(
    benchmark_id: Option<String>,
    utility_unit: Option<String>,
    cost_unit: Option<String>,
    findings: Vec<String>,
) -> DecisionValueReport {
    let mut findings = findings;
    findings.sort();
    findings.dedup();
    DecisionValueReport {
        status: "FAIL".to_string(),
        audit_kind: "synthetic_decision_value_pool".to_string(),
        benchmark_id,
        utility_unit,
        cost_unit,
        pre_measurement_worst_case_regret: None,
        max_measurement_cost: None,
        max_duration_seconds: None,
        measurements: Vec::new(),
        pareto_frontier: Vec::new(),
        oracle_measurement_id: None,
        oracle_worst_case_regret_after: None,
        oracle_gross_regret_reduction: None,
        findings,
        claim_ceiling: "Synthetic finite-pool contract and arithmetic only; no physical, model, or manufacturing claim is validated.".to_string(),
    }
}

fn decision_exact_map_keys(value: &Value, expected: &BTreeSet<String>) -> bool {
    let Some(object) = value.as_object() else { return false };
    let actual = object.keys().cloned().collect::<BTreeSet<_>>();
    &actual == expected
}

fn decision_worst_regret(
    actions: &[DecisionAction],
    scenario_ids: &[String],
    best_utility: &BTreeMap<String, f64>,
) -> Option<f64> {
    actions
        .iter()
        .filter(|action| {
            scenario_ids.iter().all(|scenario| {
                action.feasible_by_scenario.get(scenario).copied().unwrap_or(false)
            })
        })
        .map(|action| {
            scenario_ids
                .iter()
                .map(|scenario| {
                    (best_utility[scenario] - action.utility_by_scenario[scenario]).max(0.0)
                })
                .fold(0.0_f64, f64::max)
        })
        .min_by(f64::total_cmp)
}

fn decision_post_regret(
    actions: &[DecisionAction],
    best_utility: &BTreeMap<String, f64>,
    observations: &BTreeMap<String, Vec<String>>,
) -> Option<f64> {
    let mut overall = 0.0_f64;
    for grouped_scenarios in observations.values() {
        let group_regret = decision_worst_regret(actions, grouped_scenarios, best_utility)?;
        overall = overall.max(group_regret);
    }
    Some(overall)
}

/// Audit a synthetic finite-pool measurement-selection problem.
///
/// The corpus contains evaluator-side ground truth. The producer-access declarations are
/// checked as a contract only: this function does not sandbox a separate producer process.
pub fn audit_decision_value_pool(pool: &Value) -> DecisionValueReport {
    let benchmark_id = nonempty_string(pool, "benchmark_id");
    let utility_unit = nonempty_string(pool, "utility_unit");
    let cost_unit = nonempty_string(pool, "cost_unit");
    let mut errors = Vec::new();

    if benchmark_id.as_deref() != Some("glass-silicon-decision-value-synthetic-v0.1") {
        errors.push("BENCHMARK_ID:expected glass-silicon-decision-value-synthetic-v0.1".to_string());
    }
    if object_string(pool, "evidence_class") != Some("synthetic") {
        errors.push("EVIDENCE_CLASS:finite-pool reference requires evidence_class=synthetic".to_string());
    }
    if utility_unit.is_none() {
        errors.push("UTILITY_UNIT:missing non-empty utility_unit".to_string());
    }
    if cost_unit.is_none() {
        errors.push("COST_UNIT:missing non-empty cost_unit".to_string());
    }
    if nonempty_string(pool, "claim_ceiling").is_none() {
        errors.push("CLAIM_CEILING:missing explicit claim ceiling".to_string());
    }

    let access_contract = pool.get("producer_access");
    for field in ["ground_truth", "unobserved_utilities", "oracle_scores", "future_outcomes"] {
        if access_contract.and_then(|v| v.get(field)).and_then(Value::as_bool) != Some(false) {
            errors.push(format!("PRODUCER_LEAKAGE:producer_access.{field} must be explicitly false"));
        }
    }

    let max_cost = pool.get("max_measurement_cost").and_then(Value::as_f64);
    let max_duration = pool.get("max_duration_seconds").and_then(Value::as_f64);
    if max_cost.map_or(true, |v| !v.is_finite() || v < 0.0) {
        errors.push("MAX_COST:missing, non-finite, or negative max_measurement_cost".to_string());
    }
    if max_duration.map_or(true, |v| !v.is_finite() || v < 0.0) {
        errors.push("MAX_DURATION:missing, non-finite, or negative max_duration_seconds".to_string());
    }

    let scenarios_value = pool.get("scenarios").and_then(Value::as_array);
    let mut scenario_ids = Vec::new();
    if scenarios_value.map_or(true, Vec::is_empty) {
        errors.push("SCENARIOS:non-empty scenarios array required".to_string());
    } else {
        for (index, scenario) in scenarios_value.unwrap().iter().enumerate() {
            let id = nonempty_string(scenario, "id");
            if id.as_deref().map_or(true, |s| !is_stable_slug(s)) {
                errors.push(format!("SCENARIO_ID:invalid ID at index {index}"));
            } else {
                scenario_ids.push(id.unwrap());
            }
        }
        let unique: BTreeSet<_> = scenario_ids.iter().cloned().collect();
        if unique.len() != scenario_ids.len() {
            errors.push("DUPLICATE_SCENARIO_ID:scenario IDs must be unique".to_string());
        }
    }
    let expected_scenarios: BTreeSet<String> = scenario_ids.iter().cloned().collect();

    let actions_value = pool.get("actions").and_then(Value::as_array);
    let mut actions = Vec::new();
    if actions_value.map_or(true, Vec::is_empty) {
        errors.push("ACTIONS:non-empty actions array required".to_string());
    } else {
        let mut seen = BTreeSet::new();
        for (index, action) in actions_value.unwrap().iter().enumerate() {
            let id = nonempty_string(action, "id");
            if id.as_deref().map_or(true, |s| !is_stable_slug(s)) {
                errors.push(format!("ACTION_ID:invalid ID at index {index}"));
                continue;
            }
            let id = id.unwrap();
            if !seen.insert(id.clone()) {
                errors.push(format!("DUPLICATE_ACTION_ID:duplicate action {id}"));
            }
            let utility_value = action.get("utility_by_scenario").unwrap_or(&Value::Null);
            let feasible_value = action.get("feasible_by_scenario").unwrap_or(&Value::Null);
            if !decision_exact_map_keys(utility_value, &expected_scenarios) {
                errors.push(format!("UTILITY_CENSUS:action {id} must have exactly one utility for each scenario"));
                continue;
            }
            if !decision_exact_map_keys(feasible_value, &expected_scenarios) {
                errors.push(format!("FEASIBILITY_CENSUS:action {id} must have exactly one feasibility value for each scenario"));
                continue;
            }
            let mut utilities = BTreeMap::new();
            let mut feasibility = BTreeMap::new();
            for scenario in &scenario_ids {
                let utility = utility_value.get(scenario).and_then(Value::as_f64);
                if utility.map_or(true, |value| !value.is_finite()) {
                    errors.push(format!("INVALID_UTILITY:action {id}, scenario {scenario} has non-finite or missing utility"));
                } else {
                    utilities.insert(scenario.clone(), utility.unwrap());
                }
                if let Some(feasible) = feasible_value.get(scenario).and_then(Value::as_bool) {
                    feasibility.insert(scenario.clone(), feasible);
                } else {
                    errors.push(format!("INVALID_FEASIBILITY:action {id}, scenario {scenario} must be boolean"));
                }
            }
            if utilities.len() == scenario_ids.len() && feasibility.len() == scenario_ids.len() {
                actions.push(DecisionAction { id, utility_by_scenario: utilities, feasible_by_scenario: feasibility });
            }
        }
    }

    let controls = pool.get("constraint_controls").and_then(Value::as_array);
    if controls.map_or(true, Vec::is_empty) {
        errors.push("CONSTRAINT_CONTROLS:at least one frozen feasibility control is required".to_string());
    } else {
        for (index, control) in controls.unwrap().iter().enumerate() {
            let action_id = nonempty_string(control, "action_id");
            let scenario_id = nonempty_string(control, "scenario_id");
            let expected = control.get("must_be_feasible").and_then(Value::as_bool);
            let action = action_id.as_ref().and_then(|id| actions.iter().find(|a| &a.id == id));
            let observed = action.and_then(|a| scenario_id.as_ref().and_then(|id| a.feasible_by_scenario.get(id))).copied();
            if action.is_none() || !scenario_id.as_ref().map_or(false, |id| expected_scenarios.contains(id)) || expected.is_none() {
                errors.push(format!("CONSTRAINT_CONTROL:malformed or unknown frozen control at index {index}"));
            } else if observed != expected {
                errors.push(format!("CONSTRAINT_CONTROL_BREACH:action {:?}, scenario {:?} differs from frozen feasibility control", action_id, scenario_id));
            }
        }
    }

    let measurements_value = pool.get("measurements").and_then(Value::as_array);
    let mut raw_measurements = Vec::new();
    if measurements_value.map_or(true, Vec::is_empty) {
        errors.push("MEASUREMENTS:non-empty measurements array required".to_string());
    } else {
        let mut seen = BTreeSet::new();
        for (index, measurement) in measurements_value.unwrap().iter().enumerate() {
            let id = nonempty_string(measurement, "id");
            if id.as_deref().map_or(true, |s| !is_stable_slug(s)) {
                errors.push(format!("MEASUREMENT_ID:invalid ID at index {index}"));
                continue;
            }
            let id = id.unwrap();
            if !seen.insert(id.clone()) {
                errors.push(format!("DUPLICATE_MEASUREMENT_ID:duplicate measurement {id}"));
            }
            let cost = measurement.get("cost_units").and_then(Value::as_f64);
            let duration = measurement.get("duration_seconds").and_then(Value::as_f64);
            if cost.map_or(true, |value| !value.is_finite() || value < 0.0) {
                errors.push(format!("MEASUREMENT_COST:measurement {id} has missing, non-finite, or negative cost_units"));
            }
            if duration.map_or(true, |value| !value.is_finite() || value < 0.0) {
                errors.push(format!("MEASUREMENT_DURATION:measurement {id} has missing, non-finite, or negative duration_seconds"));
            }
            let observation_value = measurement.get("observation_by_scenario").unwrap_or(&Value::Null);
            if !decision_exact_map_keys(observation_value, &expected_scenarios) {
                errors.push(format!("OBSERVATION_CENSUS:measurement {id} must map every scenario exactly once"));
                continue;
            }
            let mut observation_map = BTreeMap::new();
            for scenario in &scenario_ids {
                if let Some(observation) = observation_value.get(scenario).and_then(Value::as_str).filter(|s| !s.trim().is_empty()) {
                    observation_map.insert(scenario.clone(), observation.to_string());
                } else {
                    errors.push(format!("INVALID_OBSERVATION:measurement {id}, scenario {scenario} has no observation label"));
                }
            }
            if let (Some(cost), Some(duration)) = (cost.filter(|v| v.is_finite() && *v >= 0.0), duration.filter(|v| v.is_finite() && *v >= 0.0)) {
                if observation_map.len() == scenario_ids.len() {
                    raw_measurements.push((id, cost, duration, observation_map));
                }
            }
        }
    }

    if !errors.is_empty() {
        return decision_invalid(benchmark_id, utility_unit, cost_unit, errors);
    }
    let max_cost = max_cost.unwrap();
    let max_duration = max_duration.unwrap();

    let mut best_utility = BTreeMap::new();
    for scenario in &scenario_ids {
        let best = actions.iter()
            .filter(|action| action.feasible_by_scenario[scenario])
            .map(|action| action.utility_by_scenario[scenario])
            .max_by(f64::total_cmp);
        match best {
            Some(value) => { best_utility.insert(scenario.clone(), value); }
            None => {
                return DecisionValueReport {
                    status: "UNRESOLVED".to_string(),
                    audit_kind: "synthetic_decision_value_pool".to_string(),
                    benchmark_id, utility_unit, cost_unit,
                    pre_measurement_worst_case_regret: None,
                    max_measurement_cost: Some(max_cost),
                    max_duration_seconds: Some(max_duration),
                    measurements: Vec::new(), pareto_frontier: Vec::new(),
                    oracle_measurement_id: None, oracle_worst_case_regret_after: None,
                    oracle_gross_regret_reduction: None,
                    findings: vec![format!("NO_FEASIBLE_ACTION:scenario {scenario} has no feasible action")],
                    claim_ceiling: "Synthetic finite-pool contract and arithmetic only; no physical, model, or manufacturing claim is validated.".to_string(),
                }
            }
        }
    }

    let before = match decision_worst_regret(&actions, &scenario_ids, &best_utility) {
        Some(value) => value,
        None => return DecisionValueReport {
            status: "UNRESOLVED".to_string(),
            audit_kind: "synthetic_decision_value_pool".to_string(),
            benchmark_id, utility_unit, cost_unit,
            pre_measurement_worst_case_regret: None,
            max_measurement_cost: Some(max_cost), max_duration_seconds: Some(max_duration),
            measurements: Vec::new(), pareto_frontier: Vec::new(),
            oracle_measurement_id: None, oracle_worst_case_regret_after: None,
            oracle_gross_regret_reduction: None,
            findings: vec!["NO_PRE_OBSERVATION_ROBUST_ACTION:no single action is feasible in every scenario before measurement".to_string()],
            claim_ceiling: "Synthetic finite-pool contract and arithmetic only; no physical, model, or manufacturing claim is validated.".to_string(),
        },
    };

    let mut evaluated = Vec::new();
    let mut unresolved = Vec::new();
    for (id, cost, duration, observation_map) in raw_measurements {
        let mut groups: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for scenario in &scenario_ids {
            groups.entry(observation_map[scenario].clone()).or_default().push(scenario.clone());
        }
        let post = decision_post_regret(&actions, &best_utility, &groups);
        let within_budget = cost <= max_cost && duration <= max_duration;
        let mut findings = Vec::new();
        let reduction = post.map(|after| {
            if after > before + 1e-9 {
                findings.push("INFORMATION_MONOTONICITY:post-measurement worst-case regret exceeds pre-measurement regret".to_string());
                unresolved.push(format!("INFORMATION_MONOTONICITY:{id}"));
            }
            (before - after).max(0.0)
        });
        if post.is_none() {
            findings.push("NO_ROBUST_ACTION_FOR_OBSERVATION:at least one observation group has no action feasible across all scenarios in that group".to_string());
            unresolved.push(format!("NO_ROBUST_ACTION_FOR_OBSERVATION:{id}"));
        }
        evaluated.push(DecisionMeasurementReport {
            measurement_id: id,
            status: if post.is_some() && !findings.iter().any(|f| f.starts_with("INFORMATION_MONOTONICITY")) { "PASS".to_string() } else { "UNRESOLVED".to_string() },
            cost_units: cost,
            duration_seconds: duration,
            within_budget,
            observation_groups: groups.len(),
            post_measurement_worst_case_regret: post,
            gross_worst_case_regret_reduction: reduction,
            findings,
        });
    }

    evaluated.sort_by(|a, b| a.measurement_id.cmp(&b.measurement_id));
    let eligible: Vec<&DecisionMeasurementReport> = evaluated.iter()
        .filter(|m| m.within_budget && m.post_measurement_worst_case_regret.is_some() && m.gross_worst_case_regret_reduction.is_some())
        .collect();

    let mut frontier: Vec<String> = Vec::new();
    for candidate in &eligible {
        let dominated = eligible.iter().any(|other| {
            if other.measurement_id == candidate.measurement_id { return false; }
            let other_regret = other.post_measurement_worst_case_regret.unwrap();
            let candidate_regret = candidate.post_measurement_worst_case_regret.unwrap();
            other_regret <= candidate_regret + 1e-9
                && other.cost_units <= candidate.cost_units + 1e-9
                && other.duration_seconds <= candidate.duration_seconds + 1e-9
                && (other_regret < candidate_regret - 1e-9
                    || other.cost_units < candidate.cost_units - 1e-9
                    || other.duration_seconds < candidate.duration_seconds - 1e-9)
        });
        if !dominated { frontier.push(candidate.measurement_id.clone()); }
    }
    frontier.sort();

    let mut oracle = eligible.clone();
    oracle.sort_by(|a, b| {
        a.post_measurement_worst_case_regret.unwrap().total_cmp(&b.post_measurement_worst_case_regret.unwrap())
            .then_with(|| a.cost_units.total_cmp(&b.cost_units))
            .then_with(|| a.duration_seconds.total_cmp(&b.duration_seconds))
            .then_with(|| a.measurement_id.cmp(&b.measurement_id))
    });
    let chosen = oracle.first().copied();
    if eligible.is_empty() {
        unresolved.push("NO_BUDGET_FEASIBLE_MEASUREMENT:no evaluated measurement fits both declared budget caps".to_string());
    }

    let mut findings = unresolved;
    findings.sort();
    findings.dedup();
    let has_failure = evaluated.iter().any(|m| m.findings.iter().any(|f| f.starts_with("INFORMATION_MONOTONICITY")));
    let status = if has_failure { "FAIL" } else if !findings.is_empty() { "UNRESOLVED" } else { "PASS" };
    // Materialize oracle outputs before moving the owned measurement vector into the report.
    let oracle_measurement_id = chosen.map(|m| m.measurement_id.clone());
    let oracle_worst_case_regret_after = chosen.and_then(|m| m.post_measurement_worst_case_regret);
    let oracle_gross_regret_reduction = chosen.and_then(|m| m.gross_worst_case_regret_reduction);
    DecisionValueReport {
        status: status.to_string(),
        audit_kind: "synthetic_decision_value_pool".to_string(),
        benchmark_id,
        utility_unit,
        cost_unit,
        pre_measurement_worst_case_regret: Some(before),
        max_measurement_cost: Some(max_cost),
        max_duration_seconds: Some(max_duration),
        measurements: evaluated,
        pareto_frontier: frontier,
        oracle_measurement_id,
        oracle_worst_case_regret_after,
        oracle_gross_regret_reduction,
        findings,
        claim_ceiling: "Synthetic finite-pool contract and arithmetic only; no physical, model, or manufacturing claim is validated. Producer-access fields are declarations, not an executed isolation proof.".to_string(),
    }
}

#[cfg(test)]
mod decision_value_tests {
    use super::*;
    use serde_json::{json, Value};

    fn pool() -> Value {
        serde_json::from_str(include_str!("../fixtures/decision-value-pool.synthetic.json"))
            .expect("synthetic decision-value pool must be valid JSON")
    }

    #[test]
    fn decision_value_pool_computes_exact_regret_and_budget_frontier() {
        let report = audit_decision_value_pool(&pool());
        assert_eq!(report.status, "PASS");
        assert_eq!(report.pre_measurement_worst_case_regret, Some(5.0));
        assert_eq!(report.oracle_measurement_id.as_deref(), Some("measure-regime"));
        assert_eq!(report.oracle_worst_case_regret_after, Some(0.0));
        assert_eq!(report.oracle_gross_regret_reduction, Some(5.0));
        assert!(report.pareto_frontier.contains(&"measure-regime".to_string()));
        assert!(!report.pareto_frontier.contains(&"measure-noise".to_string()));
        assert!(report.measurements.iter().any(|m| m.measurement_id == "measure-expensive" && !m.within_budget));
    }

    #[test]
    fn decision_value_missing_cost_fails_closed() {
        let mut value = pool();
        value["measurements"][0].as_object_mut().unwrap().remove("cost_units");
        let report = audit_decision_value_pool(&value);
        assert_eq!(report.status, "FAIL");
        assert!(report.findings.iter().any(|f| f.starts_with("MEASUREMENT_COST:")));
    }

    #[test]
    fn decision_value_producer_truth_leak_declaration_fails() {
        let mut value = pool();
        value["producer_access"]["ground_truth"] = json!(true);
        let report = audit_decision_value_pool(&value);
        assert_eq!(report.status, "FAIL");
        assert!(report.findings.iter().any(|f| f.starts_with("PRODUCER_LEAKAGE:")));
    }

    #[test]
    fn decision_value_frozen_constraint_control_detects_feasibility_bypass() {
        let mut value = pool();
        value["actions"][1]["feasible_by_scenario"]["s3"] = json!(true);
        let report = audit_decision_value_pool(&value);
        assert_eq!(report.status, "FAIL");
        assert!(report.findings.iter().any(|f| f.starts_with("CONSTRAINT_CONTROL_BREACH:")));
    }

    #[test]
    fn decision_value_uninformative_measurement_has_zero_regret_reduction() {
        let report = audit_decision_value_pool(&pool());
        let noise = report.measurements.iter().find(|m| m.measurement_id == "measure-noise").unwrap();
        assert_eq!(noise.post_measurement_worst_case_regret, Some(5.0));
        assert_eq!(noise.gross_worst_case_regret_reduction, Some(0.0));
    }

    #[test]
    fn decision_value_rejects_incomplete_scenario_map() {
        let mut value = pool();
        value["measurements"][0]["observation_by_scenario"].as_object_mut().unwrap().remove("s4");
        let report = audit_decision_value_pool(&value);
        assert_eq!(report.status, "FAIL");
        assert!(report.findings.iter().any(|f| f.starts_with("OBSERVATION_CENSUS:")));
    }
}


// --- GLASS-SILICON-DECISION-VALUE-001: adaptive multi-step campaign oracle ---

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct AdaptivePlanNode {
    pub node_type: String,
    pub measurement_id: Option<String>,
    pub action_id: Option<String>,
    pub observation_branches: BTreeMap<String, AdaptivePlanNode>,
    pub worst_case_regret: f64,
    pub max_additional_cost_units: f64,
    pub max_additional_duration_seconds: f64,
    pub max_measurements_on_path: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct AdaptiveCampaignReport {
    pub status: String,
    pub audit_kind: String,
    pub benchmark_id: Option<String>,
    pub utility_unit: Option<String>,
    pub cost_unit: Option<String>,
    pub max_measurement_steps: Option<usize>,
    pub pre_measurement_worst_case_regret: Option<f64>,
    pub adaptive_oracle_worst_case_regret: Option<f64>,
    pub adaptive_oracle_gross_regret_reduction: Option<f64>,
    pub adaptive_oracle_max_path_cost_units: Option<f64>,
    pub adaptive_oracle_max_path_duration_seconds: Option<f64>,
    pub adaptive_oracle_max_measurements_on_path: Option<usize>,
    pub policy_tree: Option<AdaptivePlanNode>,
    pub findings: Vec<String>,
    pub claim_ceiling: String,
}

impl AdaptiveCampaignReport {
    pub fn exit_code(&self) -> i32 {
        match self.status.as_str() {
            "PASS" => 0,
            "FAIL" => 2,
            _ => 3,
        }
    }
}

#[derive(Debug, Clone)]
struct AdaptiveMeasurement {
    id: String,
    cost_units: f64,
    duration_seconds: f64,
    observation_by_scenario: BTreeMap<String, String>,
}

struct AdaptiveCampaignSolver {
    actions: Vec<DecisionAction>,
    measurements: Vec<AdaptiveMeasurement>,
    best_utility: BTreeMap<String, f64>,
    max_cost: f64,
    max_duration: f64,
    max_steps: usize,
    memo: BTreeMap<(Vec<String>, u16), Option<AdaptivePlanNode>>,
}

fn adaptive_robust_action(
    actions: &[DecisionAction],
    scenario_ids: &[String],
    best_utility: &BTreeMap<String, f64>,
) -> Option<(String, f64)> {
    actions.iter()
        .filter(|action| scenario_ids.iter().all(|scenario| {
            action.feasible_by_scenario.get(scenario).copied().unwrap_or(false)
        }))
        .map(|action| {
            let regret = scenario_ids.iter()
                .map(|scenario| (best_utility[scenario] - action.utility_by_scenario[scenario]).max(0.0))
                .fold(0.0_f64, f64::max);
            (action.id.clone(), regret)
        })
        .min_by(|(id_a, regret_a), (id_b, regret_b)| {
            regret_a.total_cmp(regret_b).then_with(|| id_a.cmp(id_b))
        })
}

impl AdaptiveCampaignSolver {
    fn used_cost_and_duration(&self, used_mask: u16) -> (f64, f64, usize) {
        let mut cost = 0.0;
        let mut duration = 0.0;
        let mut count = 0;
        for (index, measurement) in self.measurements.iter().enumerate() {
            if used_mask & (1_u16 << index) != 0 {
                cost += measurement.cost_units;
                duration += measurement.duration_seconds;
                count += 1;
            }
        }
        (cost, duration, count)
    }

    fn solve(&mut self, scenario_ids: Vec<String>, used_mask: u16) -> Option<AdaptivePlanNode> {
        let key = (scenario_ids.clone(), used_mask);
        if let Some(cached) = self.memo.get(&key) {
            return cached.clone();
        }

        let (used_cost, used_duration, used_count) = self.used_cost_and_duration(used_mask);
        let mut best_plan = adaptive_robust_action(&self.actions, &scenario_ids, &self.best_utility)
            .map(|(action_id, regret)| AdaptivePlanNode {
                node_type: "STOP".to_string(),
                measurement_id: None,
                action_id: Some(action_id),
                observation_branches: BTreeMap::new(),
                worst_case_regret: regret,
                max_additional_cost_units: 0.0,
                max_additional_duration_seconds: 0.0,
                max_measurements_on_path: 0,
            });

        if used_count < self.max_steps {
            let candidate_measurements = self.measurements.clone();
            for (index, measurement) in candidate_measurements.into_iter().enumerate() {
                let bit = 1_u16 << index;
                if used_mask & bit != 0 {
                    continue;
                }
                if used_cost + measurement.cost_units > self.max_cost + 1e-9
                    || used_duration + measurement.duration_seconds > self.max_duration + 1e-9
                {
                    continue;
                }

                let mut partitions: BTreeMap<String, Vec<String>> = BTreeMap::new();
                for scenario in &scenario_ids {
                    let observation = measurement.observation_by_scenario.get(scenario)?;
                    partitions.entry(observation.clone()).or_default().push(scenario.clone());
                }

                let mut branches = BTreeMap::new();
                let mut worst_regret = 0.0_f64;
                let mut child_cost = 0.0_f64;
                let mut child_duration = 0.0_f64;
                let mut child_steps = 0_usize;
                let mut all_branches_solved = true;
                for (observation, subset) in partitions {
                    match self.solve(subset, used_mask | bit) {
                        Some(plan) => {
                            worst_regret = worst_regret.max(plan.worst_case_regret);
                            child_cost = child_cost.max(plan.max_additional_cost_units);
                            child_duration = child_duration.max(plan.max_additional_duration_seconds);
                            child_steps = child_steps.max(plan.max_measurements_on_path);
                            branches.insert(observation, plan);
                        }
                        None => {
                            all_branches_solved = false;
                            break;
                        }
                    }
                }
                if !all_branches_solved || branches.is_empty() {
                    continue;
                }

                let candidate = AdaptivePlanNode {
                    node_type: "MEASURE".to_string(),
                    measurement_id: Some(measurement.id.clone()),
                    action_id: None,
                    observation_branches: branches,
                    worst_case_regret: worst_regret,
                    max_additional_cost_units: measurement.cost_units + child_cost,
                    max_additional_duration_seconds: measurement.duration_seconds + child_duration,
                    max_measurements_on_path: 1 + child_steps,
                };
                let replace = best_plan.as_ref().map_or(true, |incumbent| {
                    candidate.worst_case_regret.total_cmp(&incumbent.worst_case_regret).is_lt()
                        || (candidate.worst_case_regret.total_cmp(&incumbent.worst_case_regret).is_eq()
                            && (candidate.max_additional_cost_units.total_cmp(&incumbent.max_additional_cost_units).is_lt()
                                || (candidate.max_additional_cost_units.total_cmp(&incumbent.max_additional_cost_units).is_eq()
                                    && (candidate.max_additional_duration_seconds.total_cmp(&incumbent.max_additional_duration_seconds).is_lt()
                                        || (candidate.max_additional_duration_seconds.total_cmp(&incumbent.max_additional_duration_seconds).is_eq()
                                            && (candidate.max_measurements_on_path < incumbent.max_measurements_on_path
                                                || (candidate.max_measurements_on_path == incumbent.max_measurements_on_path
                                                    && candidate.measurement_id < incumbent.measurement_id)))))))
                });
                if replace {
                    best_plan = Some(candidate);
                }
            }
        }

        self.memo.insert(key, best_plan.clone());
        best_plan
    }
}

/// Exhaustively solves small synthetic campaigns with deterministic observations.
///
/// At each observation branch, the policy may stop and choose a single action feasible
/// in every still-possible scenario, or select another unused measurement. The objective
/// is lexicographic: minimize worst-case regret, then maximum path cost, duration and
/// measurement count. This is an exact oracle for the declared finite pool, not a
/// scalable policy and not a real-world value-of-information estimator.
pub fn audit_adaptive_decision_campaign(pool: &Value) -> AdaptiveCampaignReport {
    let base = audit_decision_value_pool(pool);
    let benchmark_id = nonempty_string(pool, "benchmark_id");
    let utility_unit = nonempty_string(pool, "utility_unit");
    let cost_unit = nonempty_string(pool, "cost_unit");
    let steps = pool.get("max_measurement_steps").and_then(Value::as_u64).map(|n| n as usize);
    // The one-step evaluator can legitimately be UNRESOLVED when no action is
    // feasible across all scenarios before measurement, or within a coarse
    // one-step observation group. A later adaptive measurement may resolve that
    // ambiguity. Structural FAIL remains disqualifying; only these known
    // decision-state unresolved findings may be handed to the multi-step solver.
    let allowed_base_unresolved = base.status == "UNRESOLVED"
        && !base.findings.is_empty()
        && base.findings.iter().all(|finding| {
            [
                "NO_PRE_OBSERVATION_ROBUST_ACTION:",
                "NO_ROBUST_ACTION_FOR_OBSERVATION:",
                "NO_BUDGET_FEASIBLE_MEASUREMENT:",
            ].iter().any(|prefix| finding.starts_with(prefix))
        });
    let base_failures = base.status == "FAIL"
        || (base.status == "UNRESOLVED" && !allowed_base_unresolved);
    let mut findings = Vec::new();

    if base_failures {
        findings.extend(base.findings.iter().cloned().map(|v| format!("BASE_POOL_{v}")));
    }
    if steps.map_or(true, |n| n == 0 || n > 8) {
        findings.push("MAX_STEPS:max_measurement_steps must be in 1..=8".to_string());
    }
    let scenarios_value = pool.get("scenarios").and_then(Value::as_array);
    if scenarios_value.map_or(true, |s| s.len() > 32) {
        findings.push("SCENARIO_LIMIT:adaptive exact oracle allows at most 32 scenarios".to_string());
    }
    let measurements_value = pool.get("measurements").and_then(Value::as_array);
    if measurements_value.map_or(true, |m| m.len() > 12) {
        findings.push("MEASUREMENT_LIMIT:adaptive exact oracle allows at most 12 measurements".to_string());
    }

    if !findings.is_empty() {
        findings.sort();
        findings.dedup();
        return AdaptiveCampaignReport {
            status: "FAIL".to_string(),
            audit_kind: "synthetic_adaptive_decision_campaign".to_string(),
            benchmark_id, utility_unit, cost_unit,
            max_measurement_steps: steps,
            pre_measurement_worst_case_regret: None,
            adaptive_oracle_worst_case_regret: None,
            adaptive_oracle_gross_regret_reduction: None,
            adaptive_oracle_max_path_cost_units: None,
            adaptive_oracle_max_path_duration_seconds: None,
            adaptive_oracle_max_measurements_on_path: None,
            policy_tree: None,
            findings,
            claim_ceiling: "Synthetic finite-pool arithmetic and planning only; no physical, model, or manufacturing claim is validated.".to_string(),
        };
    }

    let scenario_ids: Vec<String> = scenarios_value.unwrap().iter()
        .filter_map(|v| nonempty_string(v, "id")).collect();
    let mut actions = Vec::new();
    for action in pool.get("actions").and_then(Value::as_array).unwrap() {
        let id = nonempty_string(action, "id").unwrap();
        let utilities = scenario_ids.iter().map(|scenario| {
            (scenario.clone(), action.get("utility_by_scenario").and_then(|map| map.get(scenario.as_str())).and_then(Value::as_f64).unwrap())
        }).collect();
        let feasible = scenario_ids.iter().map(|scenario| {
            (scenario.clone(), action.get("feasible_by_scenario").and_then(|map| map.get(scenario.as_str())).and_then(Value::as_bool).unwrap())
        }).collect();
        actions.push(DecisionAction { id, utility_by_scenario: utilities, feasible_by_scenario: feasible });
    }
    actions.sort_by(|a, b| a.id.cmp(&b.id));

    let mut measurements = Vec::new();
    for measurement in pool.get("measurements").and_then(Value::as_array).unwrap() {
        let id = nonempty_string(measurement, "id").unwrap();
        let cost_units = measurement["cost_units"].as_f64().unwrap();
        let duration_seconds = measurement["duration_seconds"].as_f64().unwrap();
        let observations = scenario_ids.iter().map(|scenario| {
            (scenario.clone(), measurement.get("observation_by_scenario").and_then(|map| map.get(scenario.as_str())).and_then(Value::as_str).unwrap().to_string())
        }).collect();
        measurements.push(AdaptiveMeasurement { id, cost_units, duration_seconds, observation_by_scenario: observations });
    }
    measurements.sort_by(|a, b| a.id.cmp(&b.id));

    // Sorting makes bit positions stable and ties reproducible for an identical pool.
    // Record the no-measurement robust-regret baseline if one exists.
    let best_utility: BTreeMap<String, f64> = scenario_ids.iter().map(|scenario| {
        let best = actions.iter().filter(|a| a.feasible_by_scenario[scenario])
            .map(|a| a.utility_by_scenario[scenario]).max_by(f64::total_cmp).unwrap();
        (scenario.clone(), best)
    }).collect();
    // Absence of a pre-measurement robust action is itself meaningful. Keep
    // the baseline regret unknown, but still search for an adaptive policy
    // whose leaves are individually robust and feasible.
    let pre_regret = decision_worst_regret(&actions, &scenario_ids, &best_utility);

    let max_cost = pool["max_measurement_cost"].as_f64().unwrap();
    let max_duration = pool["max_duration_seconds"].as_f64().unwrap();
    let mut solver = AdaptiveCampaignSolver {
        actions, measurements, best_utility,
        max_cost, max_duration, max_steps: steps.unwrap(),
        memo: BTreeMap::new(),
    };
    let plan = solver.solve(scenario_ids, 0);
    match plan {
        Some(policy_tree) => {
            let gross_reduction = pre_regret.map(|before| {
                (before - policy_tree.worst_case_regret).max(0.0)
            });
            AdaptiveCampaignReport {
                status: "PASS".to_string(),
                audit_kind: "synthetic_adaptive_decision_campaign".to_string(),
                benchmark_id, utility_unit, cost_unit,
                max_measurement_steps: steps,
                pre_measurement_worst_case_regret: pre_regret,
                adaptive_oracle_worst_case_regret: Some(policy_tree.worst_case_regret),
                adaptive_oracle_gross_regret_reduction: gross_reduction,
                adaptive_oracle_max_path_cost_units: Some(policy_tree.max_additional_cost_units),
                adaptive_oracle_max_path_duration_seconds: Some(policy_tree.max_additional_duration_seconds),
                adaptive_oracle_max_measurements_on_path: Some(policy_tree.max_measurements_on_path),
                policy_tree: Some(policy_tree),
                findings: if pre_regret.is_none() {
                    vec!["PRE_MEASUREMENT_NO_ROBUST_ACTION: baseline regret is undefined; report only the adaptive policy's terminal regret".to_string()]
                } else {
                    Vec::new()
                },
                claim_ceiling: "Synthetic finite-pool arithmetic and planning only; no physical, model, or manufacturing claim is validated.".to_string(),
            }
        }
        None => AdaptiveCampaignReport {
            status: "UNRESOLVED".to_string(),
            audit_kind: "synthetic_adaptive_decision_campaign".to_string(),
            benchmark_id, utility_unit, cost_unit,
            max_measurement_steps: steps,
            pre_measurement_worst_case_regret: pre_regret,
            adaptive_oracle_worst_case_regret: None,
            adaptive_oracle_gross_regret_reduction: None,
            adaptive_oracle_max_path_cost_units: None,
            adaptive_oracle_max_path_duration_seconds: None,
            adaptive_oracle_max_measurements_on_path: None,
            policy_tree: None,
            findings: vec!["NO_FEASIBLE_ADAPTIVE_POLICY:no policy satisfies all robust observation branches within the declared caps".to_string()],
            claim_ceiling: "Synthetic finite-pool arithmetic and planning only; no physical, model, or manufacturing claim is validated.".to_string(),
        }
    }
}

#[cfg(test)]
mod adaptive_campaign_tests {
    use super::*;
    use serde_json::{json, Value};

    fn adaptive_pool() -> Value {
        serde_json::from_str(include_str!("../fixtures/adaptive-decision-pool.synthetic.json"))
            .expect("adaptive decision pool must be valid JSON")
    }

    #[test]
    fn adaptive_oracle_uses_second_measurement_to_resolve_ambiguous_pairs() {
        let report = audit_adaptive_decision_campaign(&adaptive_pool());
        assert_eq!(report.status, "PASS");
        assert_eq!(report.pre_measurement_worst_case_regret, Some(6.0));
        assert_eq!(report.adaptive_oracle_worst_case_regret, Some(0.0));
        assert_eq!(report.adaptive_oracle_gross_regret_reduction, Some(6.0));
        assert_eq!(report.adaptive_oracle_max_path_cost_units, Some(2.0));
        assert_eq!(report.adaptive_oracle_max_path_duration_seconds, Some(20.0));
        assert_eq!(report.adaptive_oracle_max_measurements_on_path, Some(2));
        let root = report.policy_tree.unwrap();
        assert_eq!(root.node_type, "MEASURE");
        assert_eq!(root.measurement_id.as_deref(), Some("measure-alpha"));
        assert_eq!(root.observation_branches.len(), 2);
        assert!(root.observation_branches.values().all(|branch| branch.node_type == "MEASURE"));
    }

    #[test]
    fn adaptive_campaign_can_resolve_when_no_pre_measurement_action_is_robust() {
        let mut value = adaptive_pool();
        value["actions"].as_array_mut().unwrap().retain(|action| action["id"] != "conventional");
        let report = audit_adaptive_decision_campaign(&value);
        assert_eq!(report.status, "PASS");
        assert_eq!(report.pre_measurement_worst_case_regret, None);
        assert_eq!(report.adaptive_oracle_worst_case_regret, Some(0.0));
        assert_eq!(report.adaptive_oracle_gross_regret_reduction, None);
        assert_eq!(report.adaptive_oracle_max_measurements_on_path, Some(2));
        assert!(report.policy_tree.as_ref().unwrap().node_type == "MEASURE");
    }

    #[test]
    fn adaptive_campaign_abstains_when_no_robust_baseline_and_budget_blocks_resolution() {
        let mut value = adaptive_pool();
        value["actions"].as_array_mut().unwrap().retain(|action| action["id"] != "conventional");
        value["max_measurement_cost"] = json!(1.0);
        value["max_duration_seconds"] = json!(10.0);
        let report = audit_adaptive_decision_campaign(&value);
        assert_eq!(report.status, "UNRESOLVED");
        assert_eq!(report.pre_measurement_worst_case_regret, None);
        assert_eq!(report.adaptive_oracle_worst_case_regret, None);
        assert!(report.policy_tree.is_none());
        assert!(report.findings.iter().any(|finding| finding.starts_with("NO_FEASIBLE_ADAPTIVE_POLICY:")));
    }

    #[test]
    fn adaptive_oracle_respects_one_step_cap() {
        let mut value = adaptive_pool();
        value["max_measurement_steps"] = json!(1);
        let report = audit_adaptive_decision_campaign(&value);
        assert_eq!(report.status, "PASS");
        assert_eq!(report.adaptive_oracle_worst_case_regret, Some(6.0));
        assert_eq!(report.adaptive_oracle_max_measurements_on_path, Some(0));
        // An equal-regret STOP wins the tie on cost, so the exact one-step cap does not force a probe.
    }

    #[test]
    fn adaptive_oracle_survives_sensor_drift_and_failed_primary_measurement() {
        let value: Value = serde_json::from_str(include_str!("../fixtures/adaptive-decision-pool-drift-failure.synthetic.json"))
            .expect("drift/failure pool must be valid JSON");
        let one_step = audit_decision_value_pool(&value);
        assert_eq!(one_step.status, "PASS");
        assert_eq!(one_step.pre_measurement_worst_case_regret, Some(6.0));
        assert_eq!(one_step.oracle_worst_case_regret_after, Some(6.0));
        assert_eq!(one_step.oracle_gross_regret_reduction, Some(0.0));

        let report = audit_adaptive_decision_campaign(&value);
        assert_eq!(report.status, "PASS");
        assert_eq!(report.pre_measurement_worst_case_regret, Some(6.0));
        assert_eq!(report.adaptive_oracle_worst_case_regret, Some(0.0));
        assert_eq!(report.adaptive_oracle_gross_regret_reduction, Some(6.0));
        assert_eq!(report.adaptive_oracle_max_path_cost_units, Some(4.0));
        assert_eq!(report.adaptive_oracle_max_path_duration_seconds, Some(20.0));
        assert_eq!(report.adaptive_oracle_max_measurements_on_path, Some(2));
        let root = report.policy_tree.expect("robust adaptive policy tree");
        assert_eq!(root.measurement_id.as_deref(), Some("measure-primary"));
        assert_eq!(root.observation_branches.len(), 3);
        let early_stop = root.observation_branches.get("signal-a").expect("clean signal branch");
        assert_eq!(early_stop.node_type, "STOP");
        assert_eq!(early_stop.action_id.as_deref(), Some("route-a"));
        for observation in ["signal-b", "invalid"] {
            let branch = root.observation_branches.get(observation).expect("ambiguous or invalid branch");
            assert_eq!(branch.node_type, "MEASURE");
            assert_eq!(branch.measurement_id.as_deref(), Some("measure-secondary"));
            assert!(branch.observation_branches.values().all(|leaf| leaf.node_type == "STOP"));
        }
        assert!(report.findings.is_empty());
    }

    #[test]
    fn adaptive_oracle_rejects_step_limit_overflow() {
        let mut value = adaptive_pool();
        value["max_measurement_steps"] = json!(9);
        let report = audit_adaptive_decision_campaign(&value);
        assert_eq!(report.status, "FAIL");
        assert!(report.findings.iter().any(|v| v.starts_with("MAX_STEPS:")));
    }

    #[test]
    fn adaptive_oracle_rejects_more_than_twelve_measurements() {
        let mut value = adaptive_pool();
        let extra = value["measurements"][0].clone();
        for n in 0..11 {
            let mut measurement = extra.clone();
            measurement["id"] = json!(format!("extra-{n}"));
            value["measurements"].as_array_mut().unwrap().push(measurement);
        }
        let report = audit_adaptive_decision_campaign(&value);
        assert_eq!(report.status, "FAIL");
        assert!(report.findings.iter().any(|v| v.starts_with("MEASUREMENT_LIMIT:")));
    }

    #[test]
    fn adaptive_oracle_finds_no_policy_when_budget_blocks_required_measurement() {
        let mut value = adaptive_pool();
        value["max_measurement_cost"] = json!(1.0);
        value["max_duration_seconds"] = json!(10.0);
        let report = audit_adaptive_decision_campaign(&value);
        assert_eq!(report.status, "PASS");
        assert_eq!(report.adaptive_oracle_worst_case_regret, Some(6.0));
        assert_eq!(report.adaptive_oracle_max_measurements_on_path, Some(0));
    }
}


#[cfg(test)]
mod feasibility_gate_tests {
    use super::*;
    use serde_json::{json, Value};

    fn fixture() -> Value {
        serde_json::from_str(include_str!("../fixtures/glass-feasibility-gate.synthetic.json"))
            .expect("synthetic feasibility-gate fixture must be valid JSON")
    }

    #[test]
    fn staged_fixture_preserves_feasible_infeasible_and_unresolved() {
        let report = audit_feasibility_corpus(&fixture());
        assert_eq!(report.status, "PASS", "{:?}", report.findings);
        assert_eq!(report.candidates.len(), 6);

        let disposition = |id: &str| {
            report.candidates.iter()
                .find(|candidate| candidate.candidate_id == id)
                .map(|candidate| candidate.derived_disposition.clone())
                .expect("candidate should exist")
        };
        assert_eq!(disposition("control-feasible"), "FEASIBLE");
        assert_eq!(disposition("control-non-forming"), "INFEASIBLE");
        assert_eq!(disposition("control-joint-property-failure"), "INFEASIBLE");
        assert_eq!(disposition("control-unknown-measurement"), "UNRESOLVED");
        assert_eq!(disposition("control-out-of-domain"), "UNRESOLVED");
        assert_eq!(disposition("control-stale-process-evidence"), "UNRESOLVED");
    }

    #[test]
    fn confirmed_non_forming_candidate_is_infeasible_even_if_later_gates_are_not_attempted() {
        let report = audit_feasibility_corpus(&fixture());
        let candidate = report.candidates.iter()
            .find(|candidate| candidate.candidate_id == "control-non-forming")
            .expect("non-forming control should exist");
        assert_eq!(candidate.derived_disposition, "INFEASIBLE");
        assert_eq!(candidate.blocking_gates, vec!["glass_formation".to_string()]);
        assert_eq!(candidate.unresolved_gates.len(), 3);
    }

    #[test]
    fn unknown_gate_cannot_be_promoted_to_pass_without_evidence_reference() {
        let mut mutated = fixture();
        mutated["candidates"][3]["gates"]["process_window"]["status"] = json!("PASS");
        let report = audit_feasibility_corpus(&mutated);
        assert_eq!(report.status, "FAIL");
        assert!(report.findings.iter().any(|finding| {
            finding.starts_with("FEASIBILITY_GATE_EVIDENCE_REQUIRED:control-unknown-measurement/process_window")
        }));
    }

    #[test]
    fn unknown_as_zero_payload_is_rejected_as_an_unrecognized_field() {
        let mut mutated = fixture();
        mutated["candidates"][3]["gates"]["process_window"]["value"] = json!(0);
        let report = audit_feasibility_corpus(&mutated);
        assert_eq!(report.status, "FAIL");
        assert!(report.findings.iter().any(|finding| {
            finding.starts_with("FEASIBILITY_GATE_KEYS:control-unknown-measurement/process_window")
        }));
    }

    #[test]
    fn missing_mandatory_gate_is_rejected() {
        let mut mutated = fixture();
        mutated["candidates"][0]["gates"].as_object_mut().unwrap()
            .remove("joint_product_qualification");
        let report = audit_feasibility_corpus(&mutated);
        assert_eq!(report.status, "FAIL");
        assert!(report.findings.iter().any(|finding| {
            finding.starts_with("FEASIBILITY_GATE_CENSUS:control-feasible")
        }));
    }

    #[test]
    fn unknown_gate_status_is_rejected_closed() {
        let mut mutated = fixture();
        mutated["candidates"][0]["gates"]["process_window"]["status"] = json!("MAYBE");
        let report = audit_feasibility_corpus(&mutated);
        assert_eq!(report.status, "FAIL");
        assert!(report.findings.iter().any(|finding| {
            finding.starts_with("FEASIBILITY_GATE_STATUS:control-feasible/process_window")
        }));
    }

    #[test]
    fn declared_feasible_disposition_cannot_override_unresolved_gates() {
        let mut mutated = fixture();
        mutated["candidates"][3]["expected_disposition"] = json!("FEASIBLE");
        let report = audit_feasibility_corpus(&mutated);
        assert_eq!(report.status, "FAIL");
        assert!(report.findings.iter().any(|finding| {
            finding.starts_with("FEASIBILITY_DISPOSITION_MISMATCH:control-unknown-measurement")
        }));
    }
}
