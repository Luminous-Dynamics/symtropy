use serde_json::Value;
use std::collections::HashSet;
use std::env;
use std::fs;
use std::process::ExitCode;

const REGISTRY_ID: &str = "SHIPYARD-WELD-DATASET-INTEROP-001";
const ALIGNMENT_ID: &str = "WELD-MULTIMODAL-ALIGNMENT-ORACLE-001";
const DEFAULT_MANIFEST: &str = "benchmarks/shipyard/weld-dataset-interop/registry-v0.1.json";
const DEFAULT_SCHEMA: &str = "docs/tech/SHIPYARD_WELD_DATASET_INTEROP_V0_1.schema.json";
const DEFAULT_ALIGNMENT: &str = "benchmarks/shipyard/weld-dataset-interop/alignment-oracle-v0.1.json";
const DEFAULT_ALIGNMENT_SCHEMA: &str = "docs/tech/WELD_MULTIMODAL_ALIGNMENT_ORACLE_V0_1.schema.json";

#[derive(Debug, PartialEq, Eq)]
struct RegistrySummary {
    sources: usize,
    controls: usize,
    positive: usize,
    negative: usize,
}

#[derive(Debug, PartialEq, Eq)]
struct AlignmentSummary {
    cases: usize,
    samples: usize,
    missing_cases: usize,
}

fn str_at<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn bool_at(value: &Value, key: &str) -> Option<bool> {
    value.get(key).and_then(Value::as_bool)
}

fn integer_at(value: &Value, key: &str) -> Option<i64> {
    value.get(key).and_then(Value::as_i64)
}

fn number_at(value: &Value, key: &str) -> Option<f64> {
    value.get(key).and_then(Value::as_f64)
}

fn validate_registry(schema_text: &str, manifest_text: &str) -> Result<RegistrySummary, Vec<String>> {
    let mut errors = Vec::new();
    let schema: Value = serde_json::from_str(schema_text)
        .map_err(|error| vec![format!("registry schema JSON parse failed: {error}")])?;
    let manifest: Value = serde_json::from_str(manifest_text)
        .map_err(|error| vec![format!("registry manifest JSON parse failed: {error}")])?;

    if str_at(&schema, "$schema") != Some("https://json-schema.org/draft/2020-12/schema") {
        errors.push("registry schema must declare Draft 2020-12".to_owned());
    }
    if str_at(&manifest, "benchmark_id") != Some(REGISTRY_ID)
        || str_at(&manifest, "schema_version") != Some("0.1.0")
    {
        errors.push("registry benchmark identity/version mismatch".to_owned());
    }
    if bool_at(&manifest, "raw_files_downloaded") != Some(false)
        || bool_at(&manifest, "raw_files_vendored") != Some(false)
    {
        errors.push("metadata-only seed cannot download or vendor source files".to_owned());
    }

    let sources = manifest.get("sources").and_then(Value::as_array)
        .ok_or_else(|| vec!["sources must be an array".to_owned()])?;
    if sources.len() != 4 {
        errors.push(format!("expected four source cards, found {}", sources.len()));
    }
    let expected_ids = [
        "bam-tandem-gmaw-2026",
        "intel-robotic-welding-multimodal",
        "gmaw-thermography-profilometry-2025",
        "bam-scientific-tool-use-v0.1.0",
    ];
    let mut source_ids = HashSet::new();
    for (index, source) in sources.iter().enumerate() {
        let id = str_at(source, "dataset_id").unwrap_or("");
        if id != expected_ids[index.min(3)] || !source_ids.insert(id.to_owned()) {
            errors.push(format!("sources[{index}] has unexpected or duplicate dataset_id"));
        }
        if bool_at(source, "raw_files_downloaded") != Some(false)
            || bool_at(source, "raw_files_vendored") != Some(false)
        {
            errors.push(format!("{id}: raw files must remain outside this metadata-only registry"));
        }
        if bool_at(source, "verified_ndt_labels_present") != Some(false) {
            errors.push(format!("{id}: seed registry must not invent verified NDT labels"));
        }
        if bool_at(source, "modalities_have_distinct_sample_rates") != Some(true) {
            errors.push(format!("{id}: native modality cadence must be preserved"));
        }
    }

    let bam = sources.iter().find(|source| str_at(source, "dataset_id") == Some(expected_ids[0]));
    if let Some(bam) = bam {
        if str_at(bam, "rights_status") != Some("RIGHTS_PENDING")
            || str_at(bam, "raw_file_license") != Some("UNKNOWN")
            || str_at(bam, "article_license") != Some("CC-BY-4.0")
            || str_at(bam, "article_license_scope") != Some("article_text_and_metadata_only")
            || integer_at(bam, "independent_unit_count") != Some(5)
            || str_at(bam, "grouping_field") != Some("workpiece_id")
        {
            errors.push("BAM article/raw-archive rights or WID group boundary was altered".to_owned());
        }
    } else {
        errors.push("BAM source card missing".to_owned());
    }

    let intel = sources.iter().find(|source| str_at(source, "dataset_id") == Some(expected_ids[1]));
    if let Some(intel) = intel {
        if str_at(intel, "rights_status") != Some("RESEARCH_ONLY_NONCOMMERCIAL_GATED")
            || str_at(intel, "raw_file_license") != Some("Intel Research Use License Agreement")
            || str_at(intel, "access_mode") != Some("gated_research_only")
            || integer_at(intel, "independent_unit_count") != Some(236)
            || integer_at(intel, "label_count") != Some(12)
            || number_at(intel, "reported_size_gb") != Some(39.9)
            || str_at(intel, "grouping_field") != Some("session_or_weld_id")
            || bool_at(intel, "expert_defect_labels_present") != Some(false)
        {
            errors.push(
                "Intel research-only rights, session grouping or weak-label boundary altered"
                    .to_owned(),
            );
        }
    } else {
        errors.push("Intel source card missing".to_owned());
    }

    let thermo = sources.iter().find(|source| str_at(source, "dataset_id") == Some(expected_ids[2]));
    if let Some(thermo) = thermo {
        if str_at(thermo, "rights_status") != Some("RIGHTS_PENDING")
            || str_at(thermo, "raw_file_license") != Some("UNKNOWN_PENDING_ITEM_LEVEL_READBACK")
            || str_at(thermo, "article_license") != Some("CC-BY-NC-4.0")
            || integer_at(thermo, "independent_unit_count") != Some(4)
            || str_at(thermo, "grouping_field") != Some("experiment_run_id")
            || bool_at(thermo, "verified_ndt_labels_present") != Some(false)
        {
            errors.push(
                "thermography item-level rights, run grouping or macrograph/NDT boundary altered"
                    .to_owned(),
            );
        }
    } else {
        errors.push("thermography source card missing".to_owned());
    }

    let tool_use = sources
        .iter()
        .find(|source| str_at(source, "dataset_id") == Some(expected_ids[3]));
    if let Some(tool_use) = tool_use {
        let limitations = tool_use
            .get("known_limitations")
            .and_then(Value::as_array);
        let has_rights_conflict_note = limitations.is_some_and(|items| {
            items.iter().any(|item| {
                item.as_str()
                    .is_some_and(|text| text.contains("rights conflict is unresolved"))
            })
        });
        let has_score_ceiling_note = limitations.is_some_and(|items| {
            items.iter().any(|item| {
                item.as_str()
                    .is_some_and(|text| text.contains("observed score ceiling"))
            })
        });
        if str_at(tool_use, "rights_status") != Some("RIGHTS_RECONCILIATION_REQUIRED")
            || str_at(tool_use, "raw_file_license")
                != Some(
                    concat!(
                        "DERIVATIVE_CLAIMS_CC-BY-4.0; ",
                        "OFFICIAL_ZENODO_SOURCE_RECORD_SHOWS_COPYRIGHT_AND_BLANK_LICENSE_FIELD"
                    ),
                )
            || integer_at(tool_use, "independent_unit_count") != Some(5)
            || integer_at(tool_use, "task_count") != Some(160)
            || integer_at(tool_use, "answerable_task_count") != Some(130)
            || integer_at(tool_use, "constructed_withheld_task_count") != Some(30)
            || integer_at(tool_use, "introductory_sft_example_count") != Some(480)
            || integer_at(tool_use, "heldout_test_workpiece_count") != Some(1)
            || bool_at(tool_use, "published_models_all_solved_test_ceiling") != Some(true)
            || bool_at(tool_use, "raw_files_downloaded") != Some(false)
            || bool_at(tool_use, "raw_files_vendored") != Some(false)
            || !has_rights_conflict_note
            || !has_score_ceiling_note
        {
            errors.push(
                concat!(
                    "BAM scientific tool-use derivative must remain rights-reconciliation-required, ",
                    "five-WID and ceiling-limited"
                )
                .to_owned(),
            );
        }
    } else {
        errors.push("BAM scientific tool-use derivative source card missing".to_owned());
    }

    let split = manifest.get("split_policy").ok_or_else(|| vec!["split_policy missing".to_owned()])?;
    if bool_at(split, "random_sample_split_allowed") != Some(false)
        || str_at(split, "bam_group") != Some("workpiece_id")
        || str_at(split, "intel_group") != Some("welding_session_or_weld_id")
        || str_at(split, "thermography_group") != Some("experiment_run_id")
    {
        errors.push("physical group split policy was weakened".to_owned());
    }

    let claim = manifest.get("claim_policy").ok_or_else(|| vec!["claim_policy missing".to_owned()])?;
    if bool_at(claim, "process_observation_is_quality_truth") != Some(false)
        || bool_at(claim, "intended_defect_annotation_is_expert_verified") != Some(false)
        || bool_at(claim, "macrograph_is_ndt") != Some(false)
        || bool_at(claim, "verified_ndt_labels_present_in_seed_sources") != Some(false)
        || bool_at(claim, "cross_domain_accuracy_pooling_allowed") != Some(false)
        || bool_at(claim, "leave_one_domain_out_requires_semantically_matched_target") != Some(true)
    {
        errors.push("claim authority ladder or cross-domain target boundary altered".to_owned());
    }

    let controls = manifest.get("controls").and_then(Value::as_array)
        .ok_or_else(|| vec!["controls must be an array".to_owned()])?;
    if controls.len() != 46 {
        errors.push(format!("expected 46 controls, found {}", controls.len()));
    }
    let mut control_ids = HashSet::new();
    let mut positive = 0usize;
    let mut negative = 0usize;
    for (index, control) in controls.iter().enumerate() {
        let id = str_at(control, "control_id").unwrap_or("");
        let expected_id = format!("WDINT-{:03}", index + 1);
        if id != expected_id || !control_ids.insert(id.to_owned()) {
            errors.push(format!("control index {index} has an unexpected or duplicate ID"));
        }
        match str_at(control, "class") {
            Some("positive_contract_control") => positive += 1,
            Some("negative_control") => negative += 1,
            _ => errors.push(format!("{id}: unknown control class")),
        }
        if str_at(control, "mutation").is_none_or(str::is_empty)
            || str_at(control, "rationale").is_none_or(str::is_empty)
        {
            errors.push(format!("{id}: mutation and rationale must be non-empty"));
        }
    }
    if positive != 1 || negative != 45 {
        errors.push(format!("expected 1 positive / 45 negative controls, found {positive}/{negative}"));
    }

    if errors.is_empty() {
        Ok(RegistrySummary {
            sources: sources.len(),
            controls: controls.len(),
            positive,
            negative,
        })
    } else {
        Err(errors)
    }
}

fn correct_master_time(
    local_timestamp_ns: i64,
    clock_offset_local_minus_master_ns: i64,
    sensor_response_delay_ns: i64,
) -> Option<i64> {
    local_timestamp_ns
        .checked_sub(clock_offset_local_minus_master_ns)?
        .checked_sub(sensor_response_delay_ns)
}

fn validate_alignment(
    schema_text: &str,
    oracle_text: &str,
) -> Result<AlignmentSummary, Vec<String>> {
    let mut errors = Vec::new();
    let schema: Value = serde_json::from_str(schema_text)
        .map_err(|error| vec![format!("alignment schema JSON parse failed: {error}")])?;
    let oracle: Value = serde_json::from_str(oracle_text)
        .map_err(|error| vec![format!("alignment oracle JSON parse failed: {error}")])?;

    if str_at(&schema, "$schema") != Some("https://json-schema.org/draft/2020-12/schema")
        || str_at(&oracle, "benchmark_id") != Some(ALIGNMENT_ID)
        || bool_at(&oracle, "no_interpolation_across_missing_blocks") != Some(true)
    {
        errors.push("alignment oracle identity/schema/missingness policy invalid".to_owned());
    }
    let cases = oracle.get("cases").and_then(Value::as_array)
        .ok_or_else(|| vec!["alignment cases must be an array".to_owned()])?;
    if cases.len() != 5 {
        errors.push(format!("expected five alignment cases, found {}", cases.len()));
    }
    let mut sample_total = 0usize;
    let mut missing_total = 0usize;
    let mut case_ids = HashSet::new();

    for case in cases {
        let id = str_at(case, "case_id").unwrap_or("");
        if id.is_empty() || !case_ids.insert(id.to_owned()) {
            errors.push("alignment case IDs must be unique and non-empty".to_owned());
        }
        let interval = integer_at(case, "sample_interval_ns").unwrap_or(0);
        let expected_interval = match str_at(case, "modality") {
            Some("process_current_voltage") => Some(100_000),
            Some("robot_tcp") => Some(8_000_000),
            Some("power_source_opcua") => Some(100_000_000),
            Some("video_trigger") => Some(25_000_000),
            Some("missing_process_block") => Some(100_000),
            _ => None,
        };
        if expected_interval != Some(interval) {
            errors.push(format!("{id}: native modality sampling interval changed"));
        }
        let expected_missing = id == "ALIGN-MISSING-005";
        let clock_uncertainty = integer_at(case, "clock_uncertainty_ns").unwrap_or(-1);
        let latency_uncertainty = integer_at(case, "latency_uncertainty_ns").unwrap_or(-1);
        let maximum_uncertainty = integer_at(case, "maximum_alignment_uncertainty_ns").unwrap_or(-1);
        let uncertainty = clock_uncertainty.checked_add(latency_uncertainty);
        if interval <= 0 || clock_uncertainty < 0 || latency_uncertainty < 0
            || uncertainty.is_none_or(|value| value > maximum_uncertainty)
        {
            errors.push(format!("{id}: invalid cadence or alignment uncertainty exceeds threshold"));
        }
        let samples = case.get("samples").and_then(Value::as_array)
            .ok_or_else(|| vec![format!("{id}: samples must be an array")])?;
        if samples.is_empty() {
            if !expected_missing {
                errors.push(format!("{id}: unexpected empty sample block"));
            }
            missing_total += 1;
            continue;
        }
        if expected_missing {
            errors.push(format!("{id}: explicit missing block must remain empty"));
        }
        let mut previous_local = None;
        for sample in samples {
            sample_total += 1;
            let local = integer_at(sample, "local_timestamp_ns");
            let expected = integer_at(sample, "expected_master_event_time_ns");
            let (Some(local), Some(expected)) = (local, expected) else {
                errors.push(format!("{id}: sample timestamps must be integers"));
                continue;
            };
            if previous_local.is_some_and(|previous| local <= previous) {
                errors.push(format!("{id}: local timestamps must be strictly increasing"));
            }
            previous_local = Some(local);
            let corrected = correct_master_time(
                local,
                integer_at(case, "clock_offset_local_minus_master_ns").unwrap_or(i64::MAX),
                integer_at(case, "sensor_response_delay_ns").unwrap_or(i64::MAX),
            );
            if corrected != Some(expected) {
                errors.push(format!("{id}: corrected timestamp differs from frozen known answer"));
            }
        }
    }

    if errors.is_empty() {
        Ok(AlignmentSummary {
            cases: cases.len(),
            samples: sample_total,
            missing_cases: missing_total,
        })
    } else {
        Err(errors)
    }
}

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let manifest_path = args.next().unwrap_or_else(|| DEFAULT_MANIFEST.to_owned());
    let schema_path = args.next().unwrap_or_else(|| DEFAULT_SCHEMA.to_owned());
    let oracle_path = args.next().unwrap_or_else(|| DEFAULT_ALIGNMENT.to_owned());
    let alignment_schema_path = args.next().unwrap_or_else(|| DEFAULT_ALIGNMENT_SCHEMA.to_owned());
    if args.next().is_some() {
        eprintln!(
            concat!(
                "usage: shipyard-weld-dataset-interop [MANIFEST] [SCHEMA] ",
                "[ALIGNMENT_ORACLE] [ALIGNMENT_SCHEMA]"
            )
        );
        return ExitCode::from(2);
    }

    let read = |path: &str| -> Result<String, String> {
        fs::read_to_string(path).map_err(|error| format!("cannot read {path}: {error}"))
    };
    let schema_text = match read(&schema_path) {
        Ok(text) => text,
        Err(error) => { eprintln!("{error}"); return ExitCode::FAILURE; }
    };
    let manifest_text = match read(&manifest_path) {
        Ok(text) => text,
        Err(error) => { eprintln!("{error}"); return ExitCode::FAILURE; }
    };
    let alignment_schema_text = match read(&alignment_schema_path) {
        Ok(text) => text,
        Err(error) => { eprintln!("{error}"); return ExitCode::FAILURE; }
    };
    let alignment_text = match read(&oracle_path) {
        Ok(text) => text,
        Err(error) => { eprintln!("{error}"); return ExitCode::FAILURE; }
    };

    match (
        validate_registry(&schema_text, &manifest_text),
        validate_alignment(&alignment_schema_text, &alignment_text),
    ) {
        (Ok(registry), Ok(alignment)) => {
            println!(
                concat!(
                    "PASS: {sources} metadata sources; {controls} controls ",
                    "({positive} positive / {negative} negative); {cases} alignment cases; ",
                    "{samples} known-answer samples; {missing} explicit missing cases; ",
                    "no raw file admitted"
                ),
                sources = registry.sources,
                controls = registry.controls,
                positive = registry.positive,
                negative = registry.negative,
                cases=alignment.cases,samples=alignment.samples,missing=alignment.missing_cases
            );
            ExitCode::SUCCESS
        }
        (registry, alignment) => {
            if let Err(errors) = registry {
                eprintln!("REGISTRY FAIL: {} contract error(s)", errors.len());
                for error in errors { eprintln!("- {error}"); }
            }
            if let Err(errors) = alignment {
                eprintln!("ALIGNMENT FAIL: {} contract error(s)", errors.len());
                for error in errors { eprintln!("- {error}"); }
            }
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const REGISTRY_SCHEMA: &str = include_str!(
        "../../../../docs/tech/SHIPYARD_WELD_DATASET_INTEROP_V0_1.schema.json"
    );
    const REGISTRY_MANIFEST: &str = include_str!(
        "../../../../benchmarks/shipyard/weld-dataset-interop/registry-v0.1.json"
    );
    const ALIGNMENT_SCHEMA: &str = include_str!(
        "../../../../docs/tech/WELD_MULTIMODAL_ALIGNMENT_ORACLE_V0_1.schema.json"
    );
    const ALIGNMENT_CORPUS: &str = include_str!(
        "../../../../benchmarks/shipyard/weld-dataset-interop/alignment-oracle-v0.1.json"
    );

    fn registry_value() -> Value {
        serde_json::from_str(REGISTRY_MANIFEST).expect("registry manifest must parse")
    }
    fn alignment_value() -> Value {
        serde_json::from_str(ALIGNMENT_CORPUS).expect("alignment corpus must parse")
    }

    #[test]
    fn frozen_four_domain_registry_is_admitted_metadata_only() {
        let summary=validate_registry(REGISTRY_SCHEMA,REGISTRY_MANIFEST).expect("registry baseline");
        assert_eq!(summary,RegistrySummary{sources:4,controls:46,positive:1,negative:45});
    }

    #[test]
    fn bam_article_license_cannot_promote_raw_archive_rights() {
        let mut v=registry_value();
        v["sources"][0]["raw_file_license"]=Value::String("CC-BY-4.0".to_owned());
        v["sources"][0]["rights_status"]=Value::String("PUBLIC_REUSE".to_owned());
        assert!(validate_registry(REGISTRY_SCHEMA,&serde_json::to_string(&v).unwrap()).is_err());
    }

    #[test]
    fn intel_research_license_cannot_be_promoted_to_commercial() {
        let mut v=registry_value();
        v["sources"][1]["rights_status"]=Value::String("COMMERCIAL".to_owned());
        assert!(validate_registry(REGISTRY_SCHEMA,&serde_json::to_string(&v).unwrap()).is_err());
    }

    #[test]
    fn thermography_article_license_cannot_replace_item_level_license() {
        let mut v=registry_value();
        v["sources"][2]["raw_file_license"]=Value::String("CC-BY-NC-4.0".to_owned());
        v["sources"][2]["rights_status"]=Value::String("PUBLIC_REUSE".to_owned());
        assert!(validate_registry(REGISTRY_SCHEMA,&serde_json::to_string(&v).unwrap()).is_err());
    }

    #[test]
    fn derivative_cc_by_claim_cannot_override_source_zenodo_rights_conflict() {
        let mut value = registry_value();
        value["sources"][3]["rights_status"] =
            Value::String("RIGHTS_PENDING".to_owned());
        assert!(validate_registry(
            REGISTRY_SCHEMA,
            &serde_json::to_string(&value).unwrap()
        )
        .is_err());
    }

    #[test]
    fn task_rows_cannot_be_promoted_to_independent_physical_samples() {
        let mut value = registry_value();
        value["sources"][3]["independent_unit_count"] = Value::from(160);
        assert!(validate_registry(
            REGISTRY_SCHEMA,
            &serde_json::to_string(&value).unwrap()
        )
        .is_err());
    }

    #[test]
    fn model_score_ceiling_must_remain_explicit() {
        let mut value = registry_value();
        value["sources"][3]["published_models_all_solved_test_ceiling"] =
            Value::Bool(false);
        assert!(validate_registry(
            REGISTRY_SCHEMA,
            &serde_json::to_string(&value).unwrap()
        )
        .is_err());
    }

    #[test]
    fn constructed_withheld_files_are_not_natural_missingness_labels() {
        let mut value = registry_value();
        value["sources"][3]["constructed_withheld_task_count"] = Value::from(0);
        assert!(validate_registry(
            REGISTRY_SCHEMA,
            &serde_json::to_string(&value).unwrap()
        )
        .is_err());
    }

    #[test]
    fn process_labels_cannot_be_promoted_to_verified_quality() {
        let mut v=registry_value();
        v["claim_policy"]["process_observation_is_quality_truth"]=Value::Bool(true);
        assert!(validate_registry(REGISTRY_SCHEMA,&serde_json::to_string(&v).unwrap()).is_err());
    }

    #[test]
    fn macrograph_measurements_cannot_be_promoted_to_ndt() {
        let mut v=registry_value();
        v["claim_policy"]["macrograph_is_ndt"]=Value::Bool(true);
        assert!(validate_registry(REGISTRY_SCHEMA,&serde_json::to_string(&v).unwrap()).is_err());
    }

    #[test]
    fn intel_intended_defect_labels_cannot_be_promoted_to_expert_truth() {
        let mut v = registry_value();
        v["sources"][1]["expert_defect_labels_present"] = Value::Bool(true);
        assert!(validate_registry(REGISTRY_SCHEMA, &serde_json::to_string(&v).unwrap()).is_err());
    }

    #[test]
    fn incompatible_domain_targets_cannot_share_an_accuracy_denominator() {
        let mut v = registry_value();
        v["claim_policy"]["cross_domain_accuracy_pooling_allowed"] = Value::Bool(true);
        assert!(validate_registry(REGISTRY_SCHEMA, &serde_json::to_string(&v).unwrap()).is_err());
    }

    #[test]
    fn source_group_counts_are_not_replaced_with_sample_counts() {
        let mut v = registry_value();
        v["sources"][0]["independent_unit_count"] = Value::from(30);
        assert!(validate_registry(REGISTRY_SCHEMA, &serde_json::to_string(&v).unwrap()).is_err());
    }

    #[test]
    fn random_sample_split_is_rejected() {
        let mut v=registry_value();
        v["split_policy"]["random_sample_split_allowed"]=Value::Bool(true);
        assert!(validate_registry(REGISTRY_SCHEMA,&serde_json::to_string(&v).unwrap()).is_err());
    }

    #[test]
    fn source_files_cannot_be_vendored_under_metadata_only_seed() {
        let mut v=registry_value();
        v["raw_files_vendored"]=Value::Bool(true);
        assert!(validate_registry(REGISTRY_SCHEMA,&serde_json::to_string(&v).unwrap()).is_err());
    }

    #[test]
    fn alignment_known_answers_apply_clock_offset_and_sensor_latency() {
        let summary=validate_alignment(ALIGNMENT_SCHEMA,ALIGNMENT_CORPUS).expect("alignment baseline");
        assert_eq!(summary,AlignmentSummary{cases:5,samples:8,missing_cases:1});
    }

    #[test]
    fn missing_sample_block_remains_empty_without_interpolation() {
        let mut v=alignment_value();
        let last=v["cases"].as_array().unwrap().len()-1;
        assert!(v["cases"][last]["samples"].as_array().unwrap().is_empty());
        v["cases"][last]["samples"] = Value::Array(vec![serde_json::json!({
            "local_timestamp_ns": 1,
            "expected_master_event_time_ns": 1
        })]);
        assert!(validate_alignment(ALIGNMENT_SCHEMA,&serde_json::to_string(&v).unwrap()).is_err());
    }

    #[test]
    fn ignoring_sensor_latency_fails_the_known_answer() {
        let mut v=alignment_value();
        v["cases"][0]["sensor_response_delay_ns"]=Value::from(0);
        assert!(validate_alignment(ALIGNMENT_SCHEMA,&serde_json::to_string(&v).unwrap()).is_err());
    }

    #[test]
    fn clock_offset_sign_is_not_ambiguous() {
        let mut v=alignment_value();
        v["cases"][0]["clock_offset_local_minus_master_ns"]=Value::from(-2000);
        assert!(validate_alignment(ALIGNMENT_SCHEMA,&serde_json::to_string(&v).unwrap()).is_err());
    }

    #[test]
    fn uncertainty_uses_conservative_sum_and_enforces_threshold() {
        let mut v=alignment_value();
        v["cases"][0]["maximum_alignment_uncertainty_ns"]=Value::from(100);
        assert!(validate_alignment(ALIGNMENT_SCHEMA,&serde_json::to_string(&v).unwrap()).is_err());
    }

    #[test]
    fn mismatched_native_cadence_is_not_accepted_as_known_answer() {
        let mut v=alignment_value();
        v["cases"][1]["sample_interval_ns"]=Value::from(100000);
        assert!(validate_alignment(ALIGNMENT_SCHEMA,&serde_json::to_string(&v).unwrap()).is_err());
    }

    #[test]
    fn wrong_native_sampling_rate_is_rejected() {
        let mut v=alignment_value();
        v["cases"][1]["sample_interval_ns"]=Value::from(100_000);
        assert!(validate_alignment(ALIGNMENT_SCHEMA,&serde_json::to_string(&v).unwrap()).is_err());
    }

    #[test]
    fn missing_block_cannot_be_filled_with_a_synthetic_sample() {
        let mut v=alignment_value();
        let index=v["cases"].as_array().unwrap().len()-1;
        v["cases"][index]["samples"]=Value::Array(vec![
            serde_json::json!({"local_timestamp_ns":10,"expected_master_event_time_ns":10})
        ]);
        assert!(validate_alignment(ALIGNMENT_SCHEMA,&serde_json::to_string(&v).unwrap()).is_err());
    }

    #[test]
    fn unexpected_empty_modality_stream_is_rejected() {
        let mut v=alignment_value();
        v["cases"][0]["samples"]=Value::Array(vec![]);
        assert!(validate_alignment(ALIGNMENT_SCHEMA,&serde_json::to_string(&v).unwrap()).is_err());
    }

    #[test]
    fn timestamp_overflow_fails_closed() {
        assert_eq!(correct_master_time(i64::MAX, -1, 0), None);
    }
}
