// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Structural qualification for the AFEX root-pathway model tournament.
//!
//! This checks comparator coverage and evidence/claim boundaries. It does not fit
//! ecological models or assert that any acquisition pathway is identifiable.

use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fmt::Write as FmtWrite;

const REQUIRED_MODELS: [&str; 8] = [
    "M0_TREATMENT_ONLY",
    "M1_DIRECT_ROOT_INORGANIC",
    "M2_ROOT_PLUS_AM_INORGANIC",
    "M3_ROOT_AM_INORGANIC_ORGANIC",
    "M4_MYCO_C_N_COUPLED",
    "M5_MYCO_TYPE_HIERARCHICAL_TRAIT",
    "M6_NO_MEDIATION",
    "M7_OMITTED_MECHANISM",
];

fn string_at(value: &Value, key: &str) -> Result<String, String> {
    value.get(key).and_then(Value::as_str).map(ToOwned::to_owned)
        .ok_or_else(|| format!("{key} must be a string"))
}

fn sha256_hex(raw: &[u8]) -> String {
    let digest = Sha256::digest(raw);
    let mut output = String::with_capacity(64);
    for byte in digest {
        write!(&mut output, "{byte:02x}").expect("String write cannot fail");
    }
    output
}

fn string_array(value: &Value, key: &str) -> Result<Vec<String>, String> {
    value.get(key).and_then(Value::as_array)
        .ok_or_else(|| format!("{key} must be an array"))?
        .iter()
        .map(|item| item.as_str().map(ToOwned::to_owned)
            .ok_or_else(|| format!("{key} entries must be strings")))
        .collect()
}

fn contains_exact(values: &[String], expected: &str) -> bool {
    values.iter().any(|value| value == expected)
}

fn model<'a>(models: &'a [Value], id: &str) -> Result<&'a Value, String> {
    models.iter()
        .find(|item| item.get("model_id").and_then(Value::as_str) == Some(id))
        .ok_or_else(|| format!("required model {id} is missing"))
}

pub fn validate_corpus(raw: &[u8], manifest: &Value) -> Result<Value, String> {
    if string_at(manifest, "schema_version")? != "0.1.0"
        || string_at(manifest, "benchmark_id")? != "LUMINOUS-TROPICAL-FOREST-ECO-QUAL"
        || string_at(manifest, "corpus_id")? != "AFEX-ROOT-PATHWAY-MODEL-TOURNAMENT-001"
    {
        return Err("AFEX root-pathway tournament identity mismatch".to_string());
    }

    let claim_ceiling = string_at(manifest, "claim_ceiling")?;
    if !claim_ceiling.contains("AFEX raw files remain un-ingested")
        || !claim_ceiling.contains("not evidence that any candidate currently outperforms")
    {
        return Err("AFEX root-pathway claim ceiling is incomplete".to_string());
    }

    let target = manifest.get("empirical_target")
        .ok_or_else(|| "empirical_target is required".to_string())?;
    if target.get("assignment_unit").and_then(Value::as_str) != Some("plot")
        || target.get("blocks").and_then(Value::as_u64) != Some(4)
        || target.get("plots").and_then(Value::as_u64) != Some(32)
    {
        return Err("AFEX target must preserve the 4-block / 32-plot plot-assignment design".to_string());
    }

    let models = manifest.get("model_structures").and_then(Value::as_array)
        .ok_or_else(|| "model_structures must be an array".to_string())?;
    if models.len() != REQUIRED_MODELS.len() {
        return Err(format!("expected {} comparator structures", REQUIRED_MODELS.len()));
    }

    let mut ids = BTreeSet::new();
    let mut checks = Vec::new();
    let mut all_have_failure_test = true;
    let mut all_have_required_data = true;

    for item in models {
        let id = string_at(item, "model_id")?;
        if !ids.insert(id.clone()) {
            return Err(format!("duplicate model_id {id}"));
        }
        if !REQUIRED_MODELS.contains(&id.as_str()) {
            return Err(format!("unregistered model structure {id}"));
        }
        if string_at(item, "name")?.trim().is_empty()
            || string_at(item, "role")?.trim().is_empty()
            || string_at(item, "status")?.trim().is_empty()
        {
            return Err(format!("model {id} requires name, role and status"));
        }
        let required_data = string_array(item, "required_data")?;
        let failure_test = string_at(item, "principal_failure_test")?;
        all_have_failure_test &= !failure_test.trim().is_empty();
        all_have_required_data &= !required_data.is_empty();
        checks.push(json!({
            "model_id": id,
            "required_data_count": required_data.len(),
            "failure_test_present": !failure_test.trim().is_empty(),
            "status": string_at(item, "status")?
        }));
    }

    for expected in REQUIRED_MODELS {
        if !ids.contains(expected) {
            return Err(format!("mandatory baseline/challenger {expected} is missing"));
        }
    }
    if !all_have_failure_test || !all_have_required_data {
        return Err("every candidate needs both required data and a principal failure test".to_string());
    }

    let three_path = model(models, "M3_ROOT_AM_INORGANIC_ORGANIC")?;
    let three_plant = string_array(three_path, "plant_pathways")?;
    let three_fungal = string_array(three_path, "fungal_pathways")?;
    let three_support = three_path.get("support_limitations").and_then(Value::as_array)
        .ok_or_else(|| "three-pathway comparator requires support_limitations".to_string())?;
    if !contains_exact(&three_plant, "direct inorganic nutrient uptake by uncolonised fine roots")
        || !contains_exact(&three_plant, "indirect inorganic nutrient acquisition by mycorrhizal roots")
        || !contains_exact(&three_fungal, "mycorrhizal inorganic nutrient acquisition")
        || !contains_exact(&three_fungal, "mycorrhizal organic nutrient acquisition")
        || three_support.is_empty()
        || string_at(three_path, "status")? != "external architecture comparator; AFEX organic pathway currently unidentifiable"
    {
        return Err("ELM-MYCI-style three-pathway comparator lacks pathways or AFEX identifiability limits".to_string());
    }

    let myco_c = model(models, "M4_MYCO_C_N_COUPLED")?;
    if myco_c.get("external_comparator")
        .and_then(|v| v.get("name"))
        .and_then(Value::as_str) != Some("Myco-CORPSE")
        || !string_at(myco_c, "status")?.contains("not fit")
        || !string_at(myco_c, "carbon_cost")?.contains("explicit carbon allocation")
    {
        return Err("Myco-CORPSE comparator must retain its external/unfitted status and carbon-cost requirement".to_string());
    }

    let type_model = model(models, "M5_MYCO_TYPE_HIERARCHICAL_TRAIT")?;
    let type_limit = string_array(type_model, "support_limitations")?;
    if type_limit.is_empty()
        || !type_limit.iter().any(|v| v.contains("community-level"))
    {
        return Err("mycorrhizal-type model must disclose the species-level AFEX support limitation".to_string());
    }

    let no_mediation = model(models, "M6_NO_MEDIATION")?;
    let omitted = model(models, "M7_OMITTED_MECHANISM")?;
    if string_at(no_mediation, "role")? != "mandatory_causal_null"
        || string_at(omitted, "role")? != "mandatory_open_world_challenge"
    {
        return Err("no-mediation and omitted-mechanism challengers are mandatory".to_string());
    }

    let rules = manifest.get("mandatory_rules").and_then(Value::as_array)
        .ok_or_else(|| "mandatory_rules must be an array".to_string())?;
    if rules.len() < 10 {
        return Err("too few mandatory model-comparison rules".to_string());
    }
    let joined_rules = rules.iter().filter_map(Value::as_str).collect::<Vec<_>>().join(" ");
    for phrase in [
        "Do not compare R² values",
        "Use plot as the AFEX treatment assignment unit",
        "Keep an explicit no-mediation model",
        "must improve held-out predictions",
    ] {
        if !joined_rules.contains(phrase) {
            return Err(format!("mandatory rule missing: {phrase}"));
        }
    }

    Ok(json!({
        "schema_version": "0.1.0",
        "benchmark_id": "LUMINOUS-TROPICAL-FOREST-ECO-QUAL",
        "corpus_id": "AFEX-ROOT-PATHWAY-MODEL-TOURNAMENT-001",
        "result": "PASS_SYNTHETIC_ROOT_PATHWAY_TOURNAMENT_SPEC",
        "source_manifest_sha256": sha256_hex(raw),
        "model_count": models.len(),
        "mandatory_model_ids": REQUIRED_MODELS,
        "all_models_have_required_data": all_have_required_data,
        "all_models_have_failure_tests": all_have_failure_test,
        "no_mediation_present": true,
        "omitted_mechanism_present": true,
        "three_pathway_organic_acquisition_status": "UNIDENTIFIABLE_FROM_CURRENT_AFEX_REGISTER",
        "fungal_carbon_cost_status": "NOT_MEASURED_IN_CURRENT_AFEX_REGISTER",
        "species_level_mycorrhizal_type_status": "NOT_ADMITTED_IN_CURRENT_AFEX_REGISTER",
        "fit_or_empirical_superiority": "NOT_ESTABLISHED",
        "model_checks": checks
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../../cases/afex-root-pathway-model-tournament-v0.1.json");

    #[test]
    fn eight_model_tournament_preserves_external_comparator_claim_ceilings() {
        let manifest: Value = serde_json::from_str(FIXTURE).expect("fixture parses");
        let receipt = validate_corpus(FIXTURE.as_bytes(), &manifest).expect("receipt");
        assert_eq!(receipt["result"], "PASS_SYNTHETIC_ROOT_PATHWAY_TOURNAMENT_SPEC");
        assert_eq!(receipt["model_count"], 8);
        assert_eq!(receipt["no_mediation_present"], true);
        assert_eq!(receipt["omitted_mechanism_present"], true);
        assert_eq!(receipt["fit_or_empirical_superiority"], "NOT_ESTABLISHED");
        assert_eq!(receipt["source_manifest_sha256"], sha256_hex(FIXTURE.as_bytes()));
    }

    #[test]
    fn removal_of_no_mediation_baseline_fails() {
        let mut manifest: Value = serde_json::from_str(FIXTURE).expect("fixture");
        manifest["model_structures"].as_array_mut().expect("models")
            .retain(|model| model["model_id"] != "M6_NO_MEDIATION");
        let raw = serde_json::to_vec(&manifest).expect("serialize");
        assert!(validate_corpus(&raw, &manifest).is_err());
    }

    #[test]
    fn organic_pathway_cannot_be_claimed_as_afex_identified() {
        let mut manifest: Value = serde_json::from_str(FIXTURE).expect("fixture");
        manifest["model_structures"][3]["status"] = json!("AFEX_PATHWAY_IDENTIFIED");
        let raw = serde_json::to_vec(&manifest).expect("serialize");
        assert!(validate_corpus(&raw, &manifest).is_err());
    }

    #[test]
    fn community_means_cannot_be_passed_as_species_level_mycorrhizal_type_data() {
        let mut manifest: Value = serde_json::from_str(FIXTURE).expect("fixture");
        manifest["model_structures"][5]["support_limitations"] = json!([]);
        let raw = serde_json::to_vec(&manifest).expect("serialize");
        assert!(validate_corpus(&raw, &manifest).is_err());
    }
}
