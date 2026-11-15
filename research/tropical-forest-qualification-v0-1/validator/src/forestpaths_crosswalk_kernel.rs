// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Structural admission checks for the paper-specific ForestPaths D-BEN source crosswalk.
//!
//! This validates source-code metadata completeness only. It does not inspect NetCDF/CSV
//! archive bytes, authenticate the upstream code, or establish semantic correctness of
//! any model output. Model-specific transformations remain source-adapter obligations.

use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fmt::Write as FmtWrite;

fn sha256_hex(raw: &[u8]) -> String {
    let digest = Sha256::digest(raw);
    let mut output = String::with_capacity(64);
    for byte in digest {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

fn string_at<'a>(value: &'a Value, path: &[&str]) -> Result<&'a str, String> {
    let mut current = value;
    for key in path {
        current = current
            .get(*key)
            .ok_or_else(|| format!("missing field {}", path.join(".")))?;
    }
    current
        .as_str()
        .ok_or_else(|| format!("{} must be a string", path.join(".")))
}

fn array_at<'a>(value: &'a Value, path: &[&str]) -> Result<&'a Vec<Value>, String> {
    let mut current = value;
    for key in path {
        current = current
            .get(*key)
            .ok_or_else(|| format!("missing field {}", path.join(".")))?;
    }
    current
        .as_array()
        .ok_or_else(|| format!("{} must be an array", path.join(".")))
}

fn string_array(value: &Value, path: &[&str]) -> Result<Vec<String>, String> {
    array_at(value, path)?
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            entry
                .as_str()
                .filter(|item| !item.trim().is_empty())
                .map(ToOwned::to_owned)
                .ok_or_else(|| format!("{}[{index}] must be a non-empty string", path.join(".")))
        })
        .collect()
}

fn is_hex_sha40(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn check(id: &str, passed: bool, detail: impl Into<String>) -> Value {
    json!({
        "id": id,
        "passed": passed,
        "detail": detail.into()
    })
}

pub fn validate_corpus(raw: &[u8], manifest: &Value) -> Result<Value, String> {
    let schema = string_at(manifest, &["schema_version"])?;
    let crosswalk_id = string_at(manifest, &["crosswalk_id"])?;
    let status = string_at(manifest, &["status"])?;
    let claim_ceiling = string_at(manifest, &["claim_ceiling"])?;
    if schema != "0.1.0"
        || crosswalk_id != "FORESTPATHS-D-BEN-ADMISSION-001"
        || status != "SOURCE_CODE_AUDIT_ONLY_ARCHIVE_NOT_INGESTED"
        || !claim_ceiling.contains("does not establish archive ingestion")
    {
        return Err("ForestPaths crosswalk identity or claim ceiling is invalid".to_string());
    }

    let model_helpers = string_array(manifest, &["model_helpers"])?;
    let mut model_ids = BTreeSet::new();
    if model_helpers.len() != 9
        || model_helpers.iter().any(|model| !model_ids.insert(model.clone()))
    {
        return Err("model_helpers must contain nine unique model IDs".to_string());
    }

    let sites = string_array(manifest, &["sites"])?;
    if sites != vec!["FIN".to_string(), "BIA".to_string(), "BCI".to_string()] {
        return Err("sites must be FIN, BIA and BCI in the declared order".to_string());
    }

    let run_mapping = manifest
        .get("protocol_run_mapping_p0_to_p7")
        .and_then(Value::as_object)
        .ok_or_else(|| "protocol_run_mapping_p0_to_p7 must be an object".to_string())?;
    let mapping_keys: BTreeSet<String> = run_mapping.keys().cloned().collect();
    let expected_keys: BTreeSet<String> = model_helpers.iter().cloned().collect();
    if mapping_keys != expected_keys {
        return Err("run mapping keys must equal the nine model helper IDs".to_string());
    }
    let mut run_mapping_valid = true;
    for model in &model_helpers {
        let values = string_array(
            manifest,
            &["protocol_run_mapping_p0_to_p7", model.as_str()],
        )?;
        run_mapping_valid &= values.len() == 8;
    }

    let equilibrium = manifest
        .get("equilibrium_windows_by_site_and_model_order")
        .ok_or_else(|| "equilibrium_windows_by_site_and_model_order is required".to_string())?;
    let equilibrium_order = string_array(
        equilibrium,
        &["model_order"],
    )?;
    let model_order_matches = equilibrium_order == model_helpers;
    let mut equilibrium_valid = true;
    for site in ["FIN", "BIA", "BCI"] {
        let lower = array_at(equilibrium, &[site, "lower"])?;
        let upper = array_at(equilibrium, &[site, "upper"])?;
        if lower.len() != model_helpers.len() || upper.len() != model_helpers.len() {
            equilibrium_valid = false;
            continue;
        }
        for (index, (low, high)) in lower.iter().zip(upper.iter()).enumerate() {
            let low = low.as_u64().ok_or_else(|| {
                format!("{site}.lower[{index}] must be a non-negative integer")
            })?;
            let high = high.as_u64().ok_or_else(|| {
                format!("{site}.upper[{index}] must be a non-negative integer")
            })?;
            if low >= high {
                equilibrium_valid = false;
            }
        }
    }
    let visually_selected = equilibrium
        .get("windows_are_visually_selected")
        .and_then(Value::as_bool)
        == Some(true);

    let source_files = array_at(manifest, &["source_files"])?;
    let mut source_paths = BTreeSet::new();
    let mut source_shas_valid = source_files.len() == 4;
    for (index, file) in source_files.iter().enumerate() {
        let path = file
            .get("path")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("source_files[{index}].path must be a string"))?;
        let sha = file
            .get("blob_sha")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("source_files[{index}].blob_sha must be a string"))?;
        source_shas_valid &= is_hex_sha40(sha);
        source_paths.insert(path.to_string());
    }
    source_shas_valid &= source_paths.len() == 4
        && source_paths.contains("scripts/DBEN_helper_functions.R")
        && source_paths.contains("scripts/Format-DBEN_Eckes-Shephard_et_al2025.R")
        && source_paths.contains("scripts/Eckes-Shephard_et_al2025.Rmd")
        && source_paths.contains("LICENSE");

    let quantities = array_at(manifest, &["format_quantities"])?;
    let mut quantity_ids = BTreeSet::new();
    let mut quantities_valid = quantities.len() >= 20;
    for (index, quantity) in quantities.iter().enumerate() {
        let id = quantity
            .get("id")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("format_quantities[{index}].id must be a string"))?;
        let meaning = quantity
            .get("meaning")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("format_quantities[{index}].meaning must be a string"))?;
        let unit = quantity
            .get("format_unit")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("format_quantities[{index}].format_unit must be a string"))?;
        quantities_valid &= !id.trim().is_empty()
            && !meaning.trim().is_empty()
            && !unit.trim().is_empty()
            && quantity_ids.insert(id.to_string());
    }

    let composites = array_at(manifest, &["source_semantic_composites"])?;
    let mut has_biome_growth = false;
    let mut has_biome_mortality = false;
    let mut has_fates_stemmort = false;
    let mut has_generic_cmort = false;
    for composite in composites {
        let model = composite.get("model").and_then(Value::as_str).unwrap_or("");
        let source_variable = composite
            .get("source_variable")
            .and_then(Value::as_str)
            .unwrap_or("");
        if model == "BiomeE-Standalone" && source_variable == "WDgrowTot" {
            let components = string_array(composite, &["components"])?;
            has_biome_growth = components == vec!["WDgrow".to_string(), "WDrepr".to_string(), "WDmgrt".to_string()]
                && string_at(composite, &["mapped_variable"])? == "WBgrowth"
                && string_at(composite, &["pure_growth_claim"])? == "NOT_SUPPORTED";
        }
        if model == "BiomeE-Standalone" && source_variable == "WDmortTot" {
            let components = string_array(composite, &["components"])?;
            has_biome_mortality = components == vec!["WDmort".to_string(), "WDkill".to_string(), "WDdstb".to_string()]
                && string_at(composite, &["mapped_variable"])? == "cmort"
                && string_at(composite, &["pure_mortality_claim"])? == "NOT_SUPPORTED";
        }
        if model == "FATES" && source_variable == "stemmort" {
            has_fates_stemmort = string_at(composite, &["assembly"])?
                .contains("understory and overstory");
        }
        if model == "All" && source_variable == "cmort" {
            has_generic_cmort = string_at(composite, &["pure_mortality_claim"])?
                == "NOT_SUPPORTED_WITHOUT_SUBTYPE_EVIDENCE";
        }
    }

    let size_families = manifest
        .get("size_class_families")
        .and_then(Value::as_object)
        .ok_or_else(|| "size_class_families must be an object".to_string())?;
    let required_families = ["shared_dben", "orchidee", "fates", "paper1", "establishment"];
    let mut size_families_valid = true;
    let mut serialized_families = BTreeSet::new();
    for family in required_families {
        let bins = size_families
            .get(family)
            .and_then(Value::as_array)
            .ok_or_else(|| format!("size_class_families.{family} must be an array"))?;
        let serialized = serde_json::to_string(bins)
            .map_err(|error| format!("could not serialize size-class family {family}: {error}"))?;
        size_families_valid &= !bins.is_empty()
            && bins.iter().all(|bin| bin.as_str().is_some_and(|value| !value.trim().is_empty()))
            && serialized_families.insert(serialized);
    }
    size_families_valid &= string_at(
        manifest,
        &["size_class_families", "common_support_status"],
    )? == "NOT_ESTABLISHED; require exact raw bin edges and conservation-preserving redistribution before cross-model comparison";

    let forcing = manifest
        .get("forcing_identifiers_observed_in_helpers")
        .and_then(Value::as_object)
        .ok_or_else(|| "forcing_identifiers_observed_in_helpers must be an object".to_string())?;
    let forcing_keys: BTreeSet<String> = forcing
        .keys()
        .filter(|key| key.as_str() != "warning")
        .cloned()
        .collect();
    let model_set_matches = forcing_keys == expected_keys;
    let forcing_values_valid = forcing
        .iter()
        .filter(|(key, _)| key.as_str() != "warning")
        .all(|(_, value)| value.as_str().is_some_and(|item| !item.trim().is_empty()));

    let dataset_doi = string_at(manifest, &["benchmark", "dataset_record_doi"])?;
    let code_doi = string_at(manifest, &["benchmark", "code_archive", "doi"])?;
    let rmd_doi = string_at(manifest, &["benchmark", "rmd_setup_doi"])?;
    let archive_roles_separated = dataset_doi == "10.5281/zenodo.14415143"
        && code_doi == "10.5281/zenodo.15870478"
        && rmd_doi == "10.5281/zenodo.14415142"
        && dataset_doi != code_doi
        && rmd_doi != dataset_doi
        && rmd_doi != code_doi;

    let checks = vec![
        check(
            "CROSSWALK-IDENTITY-001",
            schema == "0.1.0" && crosswalk_id == "FORESTPATHS-D-BEN-ADMISSION-001",
            format!("schema={schema}; crosswalk={crosswalk_id}"),
        ),
        check(
            "SOURCE-BLOB-IDENTITY-001",
            source_shas_valid,
            format!("{} unique source files with 40-hex blob IDs", source_files.len()),
        ),
        check(
            "MODEL-RUN-MAPPING-001",
            run_mapping_valid && model_order_matches,
            format!("{} model IDs; eight run IDs per model; order matches", model_helpers.len()),
        ),
        check(
            "SITE-EQUILIBRIUM-WINDOWS-001",
            equilibrium_valid && visually_selected,
            "three sites have nine increasing, visually selected model windows",
        ),
        check(
            "VARIABLE-UNIT-CROSSWALK-001",
            quantities_valid,
            format!("{} unique format quantities with declared units", quantities.len()),
        ),
        check(
            "COMPOSITE-FLUX-SEMANTICS-001",
            has_biome_growth && has_biome_mortality && has_fates_stemmort && has_generic_cmort,
            "mortality/turnover and BiomeE aggregate semantics remain explicit",
        ),
        check(
            "SIZE-CLASS-SUPPORT-001",
            size_families_valid,
            "shared, ORCHIDEE, FATES, Paper1 and establishment bins remain distinct",
        ),
        check(
            "MODEL-FORCING-IDENTITY-001",
            model_set_matches && forcing_values_valid,
            "each model ID has a non-empty source forcing identifier",
        ),
        check(
            "DATA-CODE-DOI-SEPARATION-001",
            archive_roles_separated,
            format!("dataset={dataset_doi}; code={code_doi}; Rmd setup reference={rmd_doi}"),
        ),
        check(
            "NO-EMPIRICAL-PROMOTION-001",
            status == "SOURCE_CODE_AUDIT_ONLY_ARCHIVE_NOT_INGESTED"
                && claim_ceiling.contains("does not establish archive ingestion"),
            "source metadata audit must not claim model-output ingestion or reproduction",
        ),
    ];
    let checks_passed = checks
        .iter()
        .filter(|item| item["passed"].as_bool() == Some(true))
        .count();
    let passed = checks_passed == checks.len();

    Ok(json!({
        "receipt_version": "0.1.0",
        "validator": "symtropy-tropical-forest-qualifier",
        "benchmark_id": "LUMINOUS-TROPICAL-FOREST-ECO-QUAL",
        "crosswalk_id": crosswalk_id,
        "source_manifest_sha256": sha256_hex(raw),
        "result": if passed { "PASS_SYNTHETIC_FORESTPATHS_CROSSWALK" } else { "FAIL" },
        "checks_passed": checks_passed,
        "checks_total": checks.len(),
        "checks": checks,
        "model_count": model_helpers.len(),
        "site_ids": sites,
        "quantity_count": quantities.len(),
        "source_file_count": source_files.len(),
        "data_ingested": false,
        "model_outputs_validated": false,
        "empirical_reproduction": "NOT_RUN",
        "rights_adjudication": "NOT_ESTABLISHED",
        "dben_standard_status": "INFORMAL_NOT_SETTLED"
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str =
        include_str!("../../cases/forestpaths-dben-source-crosswalk-v0.1.json");

    fn manifest() -> Value {
        serde_json::from_str(FIXTURE).expect("valid ForestPaths crosswalk JSON")
    }

    #[test]
    fn source_crosswalk_passes_nine_models_and_three_sites() {
        let receipt = validate_corpus(FIXTURE.as_bytes(), &manifest()).expect("crosswalk parses");
        assert_eq!(receipt["result"], "PASS_SYNTHETIC_FORESTPATHS_CROSSWALK");
        assert_eq!(receipt["model_count"], 9);
        assert_eq!(receipt["site_ids"].as_array().unwrap().len(), 3);
    }

    #[test]
    fn source_crosswalk_binds_every_model_to_eight_protocol_runs() {
        let receipt = validate_corpus(FIXTURE.as_bytes(), &manifest()).expect("crosswalk parses");
        assert_eq!(receipt["result"], "PASS_SYNTHETIC_FORESTPATHS_CROSSWALK");
        assert_eq!(receipt["checks_total"], 10);
    }

    #[test]
    fn source_crosswalk_rejects_missing_model_run_mapping() {
        let mut value = manifest();
        value["protocol_run_mapping_p0_to_p7"]
            .as_object_mut().unwrap().remove("FATES");
        let raw = serde_json::to_vec(&value).expect("serialize missing run mapping");
        assert!(validate_corpus(&raw, &value).is_err());
    }

    #[test]
    fn source_crosswalk_rejects_bad_equilibrium_window_order() {
        let mut value = manifest();
        value["equilibrium_windows_by_site_and_model_order"]["FIN"]["lower"][0] = json!(500);
        let raw = serde_json::to_vec(&value).expect("serialize invalid window");
        let receipt = validate_corpus(&raw, &value).expect("well-formed fixture returns receipt");
        assert_eq!(receipt["result"], "FAIL");
    }

    #[test]
    fn source_crosswalk_rejects_missing_quantity_units() {
        let mut value = manifest();
        value["format_quantities"][0]["format_unit"] = json!("");
        let raw = serde_json::to_vec(&value).expect("serialize missing unit");
        let receipt = validate_corpus(&raw, &value).expect("well-formed fixture returns receipt");
        assert_eq!(receipt["result"], "FAIL");
    }

    #[test]
    fn source_crosswalk_rejects_composite_growth_relabelled_as_pure_growth() {
        let mut value = manifest();
        value["source_semantic_composites"][0]["pure_growth_claim"] =
            json!("SUPPORTED");
        let raw = serde_json::to_vec(&value).expect("serialize semantic mutation");
        let receipt = validate_corpus(&raw, &value).expect("well-formed fixture returns receipt");
        assert_eq!(receipt["result"], "FAIL");
    }

    #[test]
    fn source_crosswalk_rejects_identical_size_class_families() {
        let mut value = manifest();
        value["size_class_families"]["orchidee"] =
            value["size_class_families"]["shared_dben"].clone();
        let raw = serde_json::to_vec(&value).expect("serialize size class mutation");
        let receipt = validate_corpus(&raw, &value).expect("well-formed fixture returns receipt");
        assert_eq!(receipt["result"], "FAIL");
    }

    #[test]
    fn source_crosswalk_rejects_merging_code_and_data_dois() {
        let mut value = manifest();
        value["benchmark"]["code_archive"]["doi"] =
            value["benchmark"]["dataset_record_doi"].clone();
        let raw = serde_json::to_vec(&value).expect("serialize DOI mutation");
        let receipt = validate_corpus(&raw, &value).expect("well-formed fixture returns receipt");
        assert_eq!(receipt["result"], "FAIL");
    }

    #[test]
    fn source_crosswalk_receipt_is_bound_to_exact_manifest_bytes() {
        let first = validate_corpus(FIXTURE.as_bytes(), &manifest()).expect("first receipt");
        let second = validate_corpus(FIXTURE.as_bytes(), &manifest()).expect("second receipt");
        assert_eq!(first, second);
        assert_eq!(first["source_manifest_sha256"], sha256_hex(FIXTURE.as_bytes()));
    }
}
