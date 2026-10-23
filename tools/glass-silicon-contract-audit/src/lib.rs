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
        for (index, record) in artifact_records.iter().enumerate() {
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

                if report.status != "FAIL" || report.findings.iter().all(|finding| !finding.contains(&filename)) {
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
