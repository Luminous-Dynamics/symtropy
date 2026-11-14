use serde_json::Value;
use std::collections::HashSet;
use std::env;
use std::fs;
use std::process::ExitCode;

const EXPECTED_SCHEMA_URI: &str = "https://json-schema.org/draft/2020-12/schema";
const EXPECTED_BENCHMARK_ID: &str = "SHIPYARD-WELD-PHYSICS-VVUQ-001";
const EXPECTED_SCHEMA_VERSION: &str = "0.1.0";
const DEFAULT_CORPUS: &str = "benchmarks/shipyard/weld-physics/vvuq-corpus-v0.1.json";
const DEFAULT_SCHEMA: &str = "docs/tech/SHIPYARD_WELD_PHYSICS_VVUQ_V0_1.schema.json";

#[derive(Debug, PartialEq, Eq)]
struct Summary {
    source_count: usize,
    case_count: usize,
    positive_count: usize,
    negative_count: usize,
}

fn string_at<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn is_date(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| index == 4 || index == 7 || byte.is_ascii_digit())
}

fn valid_source_id(value: &str) -> bool {
    !value.is_empty()
        && value.split('-').all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        })
}

fn valid_case_id(value: &str) -> bool {
    let Some(suffix) = value.strip_prefix("WPHYS-") else {
        return false;
    };
    suffix.len() == 3 && suffix.bytes().all(|b| b.is_ascii_digit())
}

fn validate_manifest(schema_text: &str, corpus_text: &str) -> Result<Summary, Vec<String>> {
    let mut errors = Vec::new();
    let schema: Value = match serde_json::from_str(schema_text) {
        Ok(value) => value,
        Err(error) => return Err(vec![format!("schema JSON parse failed: {error}")]),
    };
    let corpus: Value = match serde_json::from_str(corpus_text) {
        Ok(value) => value,
        Err(error) => return Err(vec![format!("corpus JSON parse failed: {error}")]),
    };

    if string_at(&schema, "$schema") != Some(EXPECTED_SCHEMA_URI) {
        errors.push("schema must declare JSON Schema Draft 2020-12".to_owned());
    }
    if schema.get("additionalProperties").and_then(Value::as_bool) != Some(false) {
        errors.push("manifest schema must reject undeclared top-level properties".to_owned());
    }
    if schema.get("$defs").and_then(Value::as_object).is_none() {
        errors.push("manifest schema is missing $defs".to_owned());
    }

    if string_at(&corpus, "schema_version") != Some(EXPECTED_SCHEMA_VERSION) {
        errors.push("corpus schema_version is not 0.1.0".to_owned());
    }
    if string_at(&corpus, "benchmark_id") != Some(EXPECTED_BENCHMARK_ID) {
        errors.push("corpus benchmark_id does not match the VVUQ owner".to_owned());
    }
    if string_at(&corpus, "snapshot_date").is_none_or(|value| !is_date(value)) {
        errors.push("snapshot_date must use YYYY-MM-DD".to_owned());
    }
    if string_at(&corpus, "claim_ceiling").is_none_or(str::is_empty) {
        errors.push("claim_ceiling must be a non-empty string".to_owned());
    }

    let Some(sources) = corpus.get("sources").and_then(Value::as_array) else {
        return Err(vec!["sources must be an array".to_owned()]);
    };
    let Some(cases) = corpus.get("cases").and_then(Value::as_array) else {
        return Err(vec!["cases must be an array".to_owned()]);
    };
    if sources.is_empty() {
        errors.push("source inventory must not be empty".to_owned());
    }
    if cases.is_empty() {
        errors.push("case corpus must not be empty".to_owned());
    }

    let allowed_admission = [
        "ADMITTED_RAW",
        "ADMITTED_TABLE",
        "ADMITTED_DIGITIZED",
        "METADATA_ONLY",
        "RIGHTS_PENDING",
        "INCOMPLETE_INPUTS",
        "CONFLICTED",
        "REJECTED",
    ];
    let mut source_ids = HashSet::new();

    for (index, source) in sources.iter().enumerate() {
        let context = format!("sources[{index}]");
        let Some(source_id) = string_at(source, "source_id") else {
            errors.push(format!("{context}.source_id must be a string"));
            continue;
        };
        if !valid_source_id(source_id) {
            errors.push(format!("{context}.source_id has invalid syntax"));
        }
        if !source_ids.insert(source_id.to_owned()) {
            errors.push(format!("duplicate source_id: {source_id}"));
        }

        for field in [
            "title",
            "citation_uri",
            "known_scope_summary",
            "admission_notes",
        ] {
            if string_at(source, field).is_none_or(str::is_empty) {
                errors.push(format!("{context}.{field} must be a non-empty string"));
            }
        }
        if string_at(source, "citation_uri")
            .is_none_or(|uri| !uri.starts_with("https://") && !uri.starts_with("http://"))
        {
            errors.push(format!("{context}.citation_uri must be an HTTP(S) URI"));
        }
        if string_at(source, "retrieved_on").is_none_or(|value| !is_date(value)) {
            errors.push(format!("{context}.retrieved_on must use YYYY-MM-DD"));
        }
        if string_at(source, "admission_status")
            .is_none_or(|status| !allowed_admission.contains(&status))
        {
            errors.push(format!("{context}.admission_status is unknown"));
        }
        if source
            .get("numerical_scoring_eligible")
            .and_then(Value::as_bool)
            != Some(false)
        {
            errors.push(
                "v0.1 sources cannot score numerical data; revise the admission contract first"
                    .to_owned(),
            );
        }
        if source
            .get("raw_data_bytes_available")
            .and_then(Value::as_bool)
            != Some(false)
        {
            errors.push(format!(
                "{context} must not claim raw measurement bytes in the metadata-only v0.1 inventory"
            ));
        }
        if source
            .get("source_bytes_sha256")
            .is_none_or(|value| !value.is_null())
        {
            errors.push(
                "source digest must be null until exact source bytes are admitted".to_owned(),
            );
        }
        if source
            .get("measurement_modalities")
            .and_then(Value::as_array)
            .is_none()
        {
            errors.push(format!("{context}.measurement_modalities must be an array"));
        }
    }

    let allowed_classes = ["positive_contract_control", "negative_control"];
    let allowed_dispositions = [
        "METADATA_ONLY_RETAINED",
        "BLOCK_NUMERICAL_SCORING",
        "REJECT_SOURCE",
        "BLOCK_VALIDATION",
        "ABSTAIN_OUT_OF_DOMAIN",
        "BLOCK_CLAIM_PROMOTION",
    ];
    let mut case_ids = HashSet::new();
    let mut positive_count = 0usize;
    let mut negative_count = 0usize;

    for (index, case) in cases.iter().enumerate() {
        let context = format!("cases[{index}]");
        let Some(case_id) = string_at(case, "case_id") else {
            errors.push(format!("{context}.case_id must be a string"));
            continue;
        };
        if !valid_case_id(case_id) {
            errors.push(format!("{context}.case_id must match WPHYS-NNN"));
        }
        if !case_ids.insert(case_id.to_owned()) {
            errors.push(format!("duplicate case_id: {case_id}"));
        }

        let class = string_at(case, "case_class");
        match class {
            Some("positive_contract_control") => positive_count += 1,
            Some("negative_control") => negative_count += 1,
            _ => errors.push(format!("{context}.case_class is unknown")),
        }
        if class.is_none_or(|value| !allowed_classes.contains(&value)) {
            errors.push(format!(
                "{context}.case_class is not in the frozen vocabulary"
            ));
        }
        for field in ["target", "mutation", "rationale"] {
            if string_at(case, field).is_none_or(str::is_empty) {
                errors.push(format!("{context}.{field} must be a non-empty string"));
            }
        }
        if string_at(case, "expected_disposition")
            .is_none_or(|value| !allowed_dispositions.contains(&value))
        {
            errors.push(format!("{context}.expected_disposition is unknown"));
        }
        if case_id == "WPHYS-027" {
            let claim_scope = case.get("claim_scope");
            let output_scope_matches = claim_scope.is_some_and(|scope| {
                string_at(scope, "validated_output") == Some("post_weld_residual_stress")
                    && string_at(scope, "claimed_output") == Some("molten_pool_peak_temperature")
                    && string_at(scope, "documented_limitation")
                        == Some("liquid_solid_coupling_omitted")
                    && scope
                        .get("supports_claimed_output")
                        .and_then(Value::as_bool)
                        == Some(false)
            });
            if !output_scope_matches
                || string_at(case, "expected_disposition") != Some("BLOCK_VALIDATION")
            {
                errors.push("{context} must block cross-output validation promotion".to_owned());
            }
        }

        if case_id == "WPHYS-028"
            && (string_at(case, "source_id") != Some("s460g2m-multipass-gmaw-distortion-2022")
                || string_at(case, "expected_disposition") != Some("BLOCK_NUMERICAL_SCORING"))
        {
            errors.push(
                "{context} must keep abstract-only S460 error percentages non-scoring".to_owned(),
            );
        }

        if let Some(source_id) = case.get("source_id").and_then(Value::as_str)
            && !source_ids.contains(source_id)
        {
            errors.push(format!(
                "{context} references unknown source_id {source_id}"
            ));
        }
        if class == Some("positive_contract_control")
            && string_at(case, "expected_disposition") != Some("METADATA_ONLY_RETAINED")
        {
            errors.push(format!(
                "{context} positive control must preserve metadata without numerical scoring"
            ));
        }
    }

    if sources.len() != 10 {
        errors.push(format!("expected 10 v0.1 sources, found {}", sources.len()));
    }
    if cases.len() != 28 {
        errors.push(format!(
            "v0.1 frozen corpus must contain 28 cases, found {}",
            cases.len()
        ));
    }
    if positive_count != 1 || negative_count != 27 {
        errors.push(format!(
            "expected 1 positive and 27 negative controls; got {positive_count}/{negative_count}"
        ));
    }

    if errors.is_empty() {
        Ok(Summary {
            source_count: sources.len(),
            case_count: cases.len(),
            positive_count,
            negative_count,
        })
    } else {
        Err(errors)
    }
}

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let corpus_path = args.next().unwrap_or_else(|| DEFAULT_CORPUS.to_owned());
    let schema_path = args.next().unwrap_or_else(|| DEFAULT_SCHEMA.to_owned());
    if args.next().is_some() {
        eprintln!("usage: shipyard-weld-physics-vvuq [CORPUS_PATH] [SCHEMA_PATH]");
        return ExitCode::from(2);
    }

    let schema_text = match fs::read_to_string(&schema_path) {
        Ok(text) => text,
        Err(error) => {
            eprintln!("cannot read schema {}: {error}", schema_path);
            return ExitCode::FAILURE;
        }
    };
    let corpus_text = match fs::read_to_string(&corpus_path) {
        Ok(text) => text,
        Err(error) => {
            eprintln!("cannot read corpus {}: {error}", corpus_path);
            return ExitCode::FAILURE;
        }
    };

    match validate_manifest(&schema_text, &corpus_text) {
        Ok(summary) => {
            println!(
                "PASS: {sources} sources, {cases} cases \
                 ({positive} positive / {negative} negative); \
                 v0.1 inventory non-scoring",
                sources = summary.source_count,
                cases = summary.case_count,
                positive = summary.positive_count,
                negative = summary.negative_count
            );
            ExitCode::SUCCESS
        }
        Err(errors) => {
            eprintln!("FAIL: {} contract violation(s)", errors.len());
            for error in errors {
                eprintln!("- {error}");
            }
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCHEMA_TEXT: &str =
        include_str!("../../../../docs/tech/SHIPYARD_WELD_PHYSICS_VVUQ_V0_1.schema.json");
    const CORPUS_TEXT: &str =
        include_str!("../../../../benchmarks/shipyard/weld-physics/vvuq-corpus-v0.1.json");

    fn corpus_value() -> Value {
        serde_json::from_str(CORPUS_TEXT).expect("frozen corpus must parse")
    }

    #[test]
    fn frozen_v0_1_corpus_passes() {
        let summary = validate_manifest(SCHEMA_TEXT, CORPUS_TEXT).expect("baseline should pass");
        assert_eq!(
            summary,
            Summary {
                source_count: 10,
                case_count: 28,
                positive_count: 1,
                negative_count: 27,
            }
        );
    }

    #[test]
    fn residual_stress_agreement_does_not_validate_molten_pool_temperature() {
        let mut corpus = corpus_value();
        let case = corpus["cases"]
            .as_array_mut()
            .expect("cases array")
            .iter_mut()
            .find(|case| case["case_id"] == "WPHYS-027")
            .expect("output-scope negative control");
        case["claim_scope"]["supports_claimed_output"] = Value::Bool(true);
        let serialized = serde_json::to_string(&corpus).expect("serialize mutation");
        assert!(validate_manifest(SCHEMA_TEXT, &serialized).is_err());
    }

    #[test]
    fn abstract_error_percentages_cannot_be_promoted_to_leaderboard_scores() {
        let mut corpus = corpus_value();
        let case = corpus["cases"]
            .as_array_mut()
            .expect("cases array")
            .iter_mut()
            .find(|case| case["case_id"] == "WPHYS-028")
            .expect("S460 metadata-only negative control");
        case["expected_disposition"] = Value::String("BLOCK_VALIDATION".to_owned());
        let serialized = serde_json::to_string(&corpus).expect("serialize mutation");
        assert!(validate_manifest(SCHEMA_TEXT, &serialized).is_err());
    }

    #[test]
    fn duplicate_source_identity_is_rejected() {
        let mut corpus = corpus_value();
        let duplicate = corpus["sources"][0]["source_id"].clone();
        corpus["sources"][1]["source_id"] = duplicate;
        let serialized = serde_json::to_string(&corpus).expect("serialize mutation");
        assert!(validate_manifest(SCHEMA_TEXT, &serialized).is_err());
    }

    #[test]
    fn metadata_source_cannot_be_promoted_to_numerical_scoring() {
        let mut corpus = corpus_value();
        corpus["sources"][0]["numerical_scoring_eligible"] = Value::Bool(true);
        let serialized = serde_json::to_string(&corpus).expect("serialize mutation");
        assert!(validate_manifest(SCHEMA_TEXT, &serialized).is_err());
    }

    #[test]
    fn unknown_source_reference_is_rejected() {
        let mut corpus = corpus_value();
        corpus["cases"][0]["source_id"] = Value::String("not-in-inventory".to_owned());
        let serialized = serde_json::to_string(&corpus).expect("serialize mutation");
        assert!(validate_manifest(SCHEMA_TEXT, &serialized).is_err());
    }

    #[test]
    fn invalid_disposition_is_rejected() {
        let mut corpus = corpus_value();
        corpus["cases"][1]["expected_disposition"] = Value::String("PASS".to_owned());
        let serialized = serde_json::to_string(&corpus).expect("serialize mutation");
        assert!(validate_manifest(SCHEMA_TEXT, &serialized).is_err());
    }

    #[test]
    fn missing_negative_control_is_rejected() {
        let mut corpus = corpus_value();
        let cases = corpus["cases"].as_array_mut().expect("cases array");
        assert!(cases.pop().is_some());
        let serialized = serde_json::to_string(&corpus).expect("serialize mutation");
        assert!(validate_manifest(SCHEMA_TEXT, &serialized).is_err());
    }
}
