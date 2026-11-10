use jsonschema::{Validator, draft202012};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

const SCHEMA_ID: &str = "https://luminousdynamics.io/schemas/transport/ev-test-protocol-evidence/0.1";
const CORPUS_VERSION: &str = "ev-test-protocol-evidence-corpus/0.1";
const PROTOCOL_VERSION: &str = "ev-test-protocol-evidence/0.1";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Corpus {
    corpus_version: String,
    schema_id: String,
    cases: Vec<CorpusCase>,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CorpusCase {
    case_id: String,
    expectation: String,
    record: Value,
}
#[derive(Debug, Serialize)]
struct CaseAudit {
    case_id: String,
    expectation: String,
    accepted: bool,
    errors: Vec<String>,
    standard_id: Option<String>,
    article_scope: Option<String>,
    qualification_disposition: Option<String>,
    claim_ceiling: String,
}
#[derive(Debug, Serialize)]
struct ReportPayload {
    corpus_version: String,
    schema_id: String,
    schema_sha256: String,
    canonical_corpus_sha256: String,
    case_count: usize,
    accepted_count: usize,
    rejected_count: usize,
    non_physical_count: usize,
    cases: Vec<CaseAudit>,
}
#[derive(Debug, Serialize)]
pub struct CorpusReport {
    #[serde(flatten)]
    payload: ReportPayload,
    report_sha256: String,
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|byte| format!("{byte:02x}")).collect()
}
fn is_digest(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn nonempty(value: Option<&str>) -> bool {
    value.is_some_and(|value| !value.trim().is_empty())
}
fn str_at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a str> {
    path.iter().try_fold(value, |current, key| current.get(*key))?.as_str()
}
fn schema_validator(bytes: &[u8]) -> Result<Validator, String> {
    let schema: Value = serde_json::from_slice(bytes).map_err(|e| format!("schema JSON: {e}"))?;
    if schema.get("$schema").and_then(Value::as_str) != Some("https://json-schema.org/draft/2020-12/schema") {
        return Err("schema must declare JSON Schema Draft 2020-12".to_owned());
    }
    if schema.get("$id").and_then(Value::as_str) != Some(SCHEMA_ID) {
        return Err("schema identity does not match EV test protocol evidence v0.1".to_owned());
    }
    if !draft202012::meta::is_valid(&schema) {
        return Err("schema fails Draft 2020-12 meta-schema validation".to_owned());
    }
    draft202012::options().should_validate_formats(true).build(&schema)
        .map_err(|e| format!("schema compilation: {e}"))
}
fn scope_compatible(standard: &str, scope: &str) -> bool {
    match standard {
        "IEC 62660-1" => scope == "cell",
        "IEC 62660-2" => matches!(scope, "cell" | "cell_block"),
        "ISO 12405-4" => matches!(scope, "pack" | "system"),
        "SAE J2464" => matches!(scope, "cell" | "cell_block" | "module" | "pack" | "system"),
        "SAE J2929" => matches!(scope, "pack" | "system" | "vehicle"),
        "OTHER" => true,
        _ => false,
    }
}
fn purpose_compatible(standard: &str, purpose: &str) -> bool {
    match standard {
        "IEC 62660-1" => matches!(purpose, "performance" | "capacity" | "power" | "energy" | "life_cycle"),
        "IEC 62660-2" => matches!(purpose, "reliability" | "abuse"),
        "ISO 12405-4" => matches!(purpose, "performance" | "capacity" | "power" | "energy" | "reliability" | "electrical_functionality"),
        "SAE J2464" => purpose == "abuse",
        "SAE J2929" => purpose == "safety_conformity",
        "OTHER" => true,
        _ => false,
    }
}
fn validate_record(record: &Value) -> Vec<String> {
    let mut errors = Vec::new();
    if str_at(record, &["protocol_version"]) != Some(PROTOCOL_VERSION) || !nonempty(str_at(record, &["record_id"])) {
        errors.push("protocol version or record identity is invalid".to_owned());
    }
    let standard = str_at(record, &["standard", "standard_id"]).unwrap_or_default();
    let scope = str_at(record, &["specimen", "article_scope"]).unwrap_or_default();
    let purpose = str_at(record, &["test_event", "test_purpose"]).unwrap_or_default();
    let evidence_class = str_at(record, &["evidence_class"]).unwrap_or_default();
    let disposition = str_at(record, &["qualification_disposition"]).unwrap_or_default();

    if !scope_compatible(standard, scope) {
        errors.push(format!("{standard} is not applicable to declared specimen scope {scope}"));
    }
    if !purpose_compatible(standard, purpose) {
        errors.push(format!("{standard} does not support declared test purpose {purpose}"));
    }
    if standard == "SAE J2464"
        && (str_at(record, &["standard", "normative_or_practice"]) != Some("recommended_practice")
            || disposition != "not_assessed")
    {
        errors.push("SAE J2464 is an abuse-test recommended practice and must not be represented as pass/fail qualification".to_owned());
    }
    if matches!(evidence_class, "simulation" | "synthetic_fixture" | "vendor_claim") && disposition != "not_assessed" {
        errors.push("simulation, synthetic and vendor-claim evidence cannot establish qualification disposition".to_owned());
    }

    let executed = str_at(record, &["test_event", "executed_at"]).unwrap_or_default();
    let start = record.pointer("/test_event/start_time_ms").and_then(Value::as_u64);
    let end = record.pointer("/test_event/end_time_ms").and_then(Value::as_u64);
    if !executed.ends_with('Z') || start.zip(end).map_or(true, |(a, b)| b <= a) {
        errors.push("test event execution time or start/end ordering is invalid".to_owned());
    }
    let lab_scope = str_at(record, &["laboratory", "facility_scope_ref"]).unwrap_or_default();
    if !lab_scope.contains(scope) {
        errors.push("laboratory declared scope does not identify specimen article scope".to_owned());
    }

    let instruments: BTreeSet<String> = record.get("measurement_chain").and_then(Value::as_array)
        .into_iter().flatten()
        .filter_map(|row| row.get("instrument_ref").and_then(Value::as_str).map(str::to_owned))
        .collect();
    if instruments.is_empty() {
        errors.push("measurement chain has no declared instruments".to_owned());
    }
    for instrument in record.get("measurement_chain").and_then(Value::as_array).into_iter().flatten() {
        let id = instrument.get("instrument_ref").and_then(Value::as_str).unwrap_or("unknown");
        let calibrated_at = instrument.get("calibrated_at").and_then(Value::as_str).unwrap_or_default();
        let valid_until = instrument.get("valid_until").and_then(Value::as_str).unwrap_or_default();
        if instrument.get("calibration_status").and_then(Value::as_str) != Some("valid")
            || !calibrated_at.ends_with('Z') || !valid_until.ends_with('Z')
            || calibrated_at > executed || valid_until < executed
        {
            errors.push(format!("instrument {id} calibration is not valid at test execution time"));
        }
        for field in ["instrument_serial_digest", "calibration_certificate_digest", "calibration_traceability_digest"] {
            if !instrument.get(field).and_then(Value::as_str).is_some_and(is_digest) {
                errors.push(format!("instrument {id} has invalid {field}"));
            }
        }
    }
    for channel in record.get("channels").and_then(Value::as_array).into_iter().flatten() {
        let id = channel.get("channel_id").and_then(Value::as_str).unwrap_or("unknown");
        let instrument = channel.get("instrument_ref").and_then(Value::as_str).unwrap_or_default();
        if !instruments.contains(instrument) {
            errors.push(format!("channel {id} references an instrument absent from the calibrated measurement chain"));
        }
    }
    for deviation in record.get("deviations").and_then(Value::as_array).into_iter().flatten() {
        let status = deviation.get("approval_status").and_then(Value::as_str).unwrap_or_default();
        match status {
            "not_required" => {},
            "approved" => {
                if !nonempty(deviation.get("approval_authority_ref").and_then(Value::as_str))
                    || !deviation.get("approval_evidence_digest").and_then(Value::as_str).is_some_and(is_digest)
                {
                    errors.push("approved deviation lacks authority or approval evidence".to_owned());
                }
            },
            _ => errors.push("test deviation is pending, rejected or otherwise not approved".to_owned()),
        }
        if !deviation.get("impact_assessment_digest").and_then(Value::as_str).is_some_and(is_digest) {
            errors.push("test deviation lacks impact-assessment evidence".to_owned());
        }
    }

    let review = record.get("independent_review").unwrap_or(&Value::Null);
    if evidence_class == "physical_observation" {
        if review.get("review_status").and_then(Value::as_str) != Some("reviewed")
            || !review.get("signature_or_attestation_digest").and_then(Value::as_str).is_some_and(is_digest)
        {
            errors.push("physical test report requires independent review and signature/attestation evidence".to_owned());
        }
        if str_at(record, &["laboratory", "accreditation_status"]) == Some("accredited_scope_declared")
            && !record.pointer("/laboratory/accreditation_evidence_digest").and_then(Value::as_str).is_some_and(is_digest)
        {
            errors.push("declared laboratory accreditation lacks scope evidence".to_owned());
        }
    }
    for field in ["raw_data_digest", "report_digest"] {
        if !record.get(field).and_then(Value::as_str).is_some_and(is_digest) {
            errors.push(format!("{field} is missing or invalid"));
        }
    }
    if record.get("results").and_then(Value::as_array).map_or(true, |rows| rows.is_empty()) {
        errors.push("test record contains no declared result rows".to_owned());
    }
    if str_at(record, &["standard", "recorded_status"]) == Some("withdrawn") && disposition != "not_assessed" {
        errors.push("withdrawn standards may be preserved historically but cannot support a current qualification disposition".to_owned());
    }
    errors
}
fn canonicalize(value: &mut Value) {
    if let Some(cases) = value.get_mut("cases").and_then(Value::as_array_mut) {
        cases.sort_by(|a, b| a.get("case_id").and_then(Value::as_str).cmp(&b.get("case_id").and_then(Value::as_str)));
    }
}
pub fn audit_corpus(schema_bytes: &[u8], corpus_bytes: &[u8]) -> Result<CorpusReport, String> {
    let validator = schema_validator(schema_bytes)?;
    let mut value: Value = serde_json::from_slice(corpus_bytes).map_err(|e| format!("corpus JSON: {e}"))?;
    canonicalize(&mut value);
    let canonical = serde_json::to_vec(&value).map_err(|e| format!("canonical corpus: {e}"))?;
    let corpus: Corpus = serde_json::from_slice(&canonical).map_err(|e| format!("typed corpus: {e}"))?;
    if corpus.corpus_version != CORPUS_VERSION || corpus.schema_id != SCHEMA_ID {
        return Err("corpus version or schema identity mismatch".to_owned());
    }
    let mut seen = BTreeSet::new();
    let mut cases = Vec::new();
    let mut accepted_count = 0usize;
    let mut rejected_count = 0usize;
    let mut non_physical_count = 0usize;
    for case in corpus.cases {
        if case.case_id.trim().is_empty() || !seen.insert(case.case_id.clone()) {
            return Err(format!("case identity is empty or duplicated: {}", case.case_id));
        }
        if !matches!(case.expectation.as_str(), "accept" | "reject") {
            return Err(format!("case {} has unknown expectation", case.case_id));
        }
        let mut errors: Vec<String> = validator.iter_errors(&case.record).map(|e| e.to_string()).collect();
        if errors.is_empty() { errors.extend(validate_record(&case.record)); }
        let accepted = errors.is_empty();
        if accepted != (case.expectation == "accept") {
            let detail = if accepted { "unexpected acceptance".to_owned() } else { errors.join("; ") };
            return Err(format!("case {} expectation mismatch: {detail}", case.case_id));
        }
        if accepted {
            accepted_count += 1;
            if case.record.get("evidence_class").and_then(Value::as_str) != Some("physical_observation") {
                non_physical_count += 1;
            }
        } else {
            rejected_count += 1;
        }
        cases.push(CaseAudit {
            case_id: case.case_id,
            expectation: case.expectation,
            accepted,
            errors,
            standard_id: str_at(&case.record, &["standard", "standard_id"]).map(str::to_owned),
            article_scope: str_at(&case.record, &["specimen", "article_scope"]).map(str::to_owned),
            qualification_disposition: str_at(&case.record, &["qualification_disposition"]).map(str::to_owned),
            claim_ceiling: if case.record.get("evidence_class").and_then(Value::as_str) == Some("physical_observation") {
                "PRODUCER_DECLARED_PHYSICAL_TEST_EVIDENCE_NOT_CERTIFICATION".to_owned()
            } else {
                "NON_PHYSICAL_OR_VENDOR_EVIDENCE_NOT_A_QUALIFICATION".to_owned()
            },
        });
    }
    let payload = ReportPayload {
        corpus_version: CORPUS_VERSION.to_owned(),
        schema_id: SCHEMA_ID.to_owned(),
        schema_sha256: sha256(schema_bytes),
        canonical_corpus_sha256: sha256(&canonical),
        case_count: cases.len(),
        accepted_count,
        rejected_count,
        non_physical_count,
        cases,
    };
    let bytes = serde_json::to_vec(&payload).map_err(|e| format!("report serialization: {e}"))?;
    Ok(CorpusReport { payload, report_sha256: sha256(&bytes) })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    const SCHEMA: &[u8] = include_bytes!("../../../schemas/transport/ev_test_protocol_evidence_v0_1.schema.json");
    const CORPUS: &[u8] = include_bytes!("../../../fixtures/ev_battery/test_protocol_evidence_scenarios_v0_1.json");
    fn report() -> CorpusReport { audit_corpus(SCHEMA, CORPUS).expect("frozen EV test-protocol evidence corpus") }

    #[test]
    fn frozen_corpus_accepts_six_and_rejects_eight_controls() {
        let r = report();
        assert_eq!(r.payload.case_count, 14);
        assert_eq!(r.payload.accepted_count, 6);
        assert_eq!(r.payload.rejected_count, 8);
        assert_eq!(r.payload.non_physical_count, 1);
    }
    #[test]
    fn cell_and_pack_protocols_are_not_interchangeable() {
        let r = report();
        for id in ["iec-cell-standard-must-not-claim-pack-scope", "iso-pack-standard-must-not-claim-cell-scope"] {
            assert!(!r.payload.cases.iter().find(|c| c.case_id == id).unwrap().accepted);
        }
    }
    #[test]
    fn abuse_practice_cannot_be_coerced_into_pass_fail_qualification() {
        let r = report();
        assert_eq!(r.payload.cases.iter().find(|c| c.case_id == "sae-j2464-abuse-guidance-no-pass-fail").unwrap().qualification_disposition.as_deref(), Some("not_assessed"));
        assert!(!r.payload.cases.iter().find(|c| c.case_id == "j2464-practice-cannot-claim-pass-fail").unwrap().accepted);
    }
    #[test]
    fn calibration_must_be_valid_at_the_actual_test_time() {
        assert!(!report().payload.cases.iter().find(|c| c.case_id == "expired-calibration-rejected").unwrap().accepted);
    }
    #[test]
    fn deviations_require_approval_and_impact_evidence() {
        assert!(!report().payload.cases.iter().find(|c| c.case_id == "unapproved-deviation-rejected").unwrap().accepted);
    }
    #[test]
    fn channels_must_bind_to_a_calibrated_instrument() {
        assert!(!report().payload.cases.iter().find(|c| c.case_id == "measurement-chain-missing-channel-instrument-rejected").unwrap().accepted);
    }
    #[test]
    fn physical_test_reports_require_independent_review() {
        assert!(!report().payload.cases.iter().find(|c| c.case_id == "physical-test-must-have-independent-review").unwrap().accepted);
    }
    #[test]
    fn raw_data_and_report_digests_are_mandatory() {
        assert!(!report().payload.cases.iter().find(|c| c.case_id == "raw-data-digest-missing-rejected").unwrap().accepted);
    }
    #[test]
    fn synthetic_evidence_cannot_claim_qualification() {
        let c = report().payload.cases.iter().find(|c| c.case_id == "synthetic-fixture-is-not-physical-test").unwrap();
        assert_eq!(c.claim_ceiling, "NON_PHYSICAL_OR_VENDOR_EVIDENCE_NOT_A_QUALIFICATION");
    }
    #[test]
    fn canonical_report_is_invariant_to_case_order() {
        let original = report();
        let mut v: Value = serde_json::from_slice(CORPUS).unwrap();
        v["cases"].as_array_mut().unwrap().reverse();
        let replay = audit_corpus(SCHEMA, &serde_json::to_vec(&v).unwrap()).unwrap();
        assert_eq!(original.report_sha256, replay.report_sha256);
        assert_eq!(original.payload.canonical_corpus_sha256, replay.payload.canonical_corpus_sha256);
    }
}
