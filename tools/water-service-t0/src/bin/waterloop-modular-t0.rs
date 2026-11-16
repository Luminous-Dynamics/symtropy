// SPDX-License-Identifier: AGPL-3.0-or-later
//! Structural T0 verifier for the WaterLoop modular treatment architecture.
//!
//! This is a source/module compatibility and fail-safe contract checker only.
//! It is not a treatment-process model, hydraulic solver or water-quality test.

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn field<'a>(value: &'a Value, key: &str) -> Result<&'a Value, String> {
    value
        .get(key)
        .ok_or_else(|| format!("missing field: {key}"))
}

fn string(value: &Value, key: &str) -> Result<String, String> {
    field(value, key)?
        .as_str()
        .map(str::to_owned)
        .filter(|text| !text.trim().is_empty())
        .ok_or_else(|| format!("{key}: expected non-empty string"))
}

fn strings(value: &Value, key: &str) -> Result<Vec<String>, String> {
    field(value, key)?
        .as_array()
        .ok_or_else(|| format!("{key}: expected array"))?
        .iter()
        .enumerate()
        .map(|(index, item)| {
            item.as_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("{key}[{index}]: expected string"))
        })
        .collect()
}

fn rows<'a>(value: &'a Value, key: &str) -> Result<&'a Vec<Value>, String> {
    field(value, key)?
        .as_array()
        .ok_or_else(|| format!("{key}: expected array"))
}

fn unique_ids(value: &Value, key: &str, kind: &str) -> Result<BTreeSet<String>, String> {
    let mut ids = BTreeSet::new();
    for row in rows(value, key)? {
        let id = string(row, "id")?;
        if !ids.insert(id.clone()) {
            return Err(format!("duplicate {kind} ID: {id}"));
        }
    }
    Ok(ids)
}

fn require_contains(values: &[String], required: &str, field_name: &str) -> Result<(), String> {
    if values.iter().any(|value| value == required) {
        Ok(())
    } else {
        Err(format!("{field_name}: required value missing: {required}"))
    }
}

pub fn validate_architecture(contract: &Value) -> Result<Value, String> {
    if string(contract, "schema_version")? != "waterloop-modular-architecture-v0.1" {
        return Err("unsupported modular architecture schema_version".to_owned());
    }
    if string(contract, "evidence_class")? != "design_reference_only" {
        return Err("architecture must remain design_reference_only".to_owned());
    }
    if string(contract, "qualification_status")? != "NotRun" {
        return Err("T0 structural checks cannot promote qualification_status".to_owned());
    }

    let sources = unique_ids(contract, "source_classes", "source")?;
    let modules = unique_ids(contract, "candidate_modules", "module")?;
    let trains = unique_ids(contract, "candidate_train_families", "train")?;

    for required in [
        "SRC-LGW-BATH-SHOWER",
        "SRC-LAUNDRY",
        "SRC-KITCHEN-DISHWASHER",
        "SRC-TOILET-BIDET-FAECAL",
        "SRC-RAINWATER",
        "SRC-MAINS-BACKUP",
    ] {
        if !sources.contains(required) {
            return Err(format!("required source class missing: {required}"));
        }
    }

    let module_interface = field(contract, "module_interface")?;
    let hydraulic_ports = strings(module_interface, "hydraulic_ports")?;
    for required in [
        "feed_in",
        "qualified_product_out",
        "reject_out",
        "purge_drain",
        "sample_in",
        "sample_out",
        "isolation_bypass",
    ] {
        require_contains(
            &hydraulic_ports,
            required,
            "module_interface.hydraulic_ports",
        )?;
    }
    let identity_fields = strings(module_interface, "mandatory_identity")?;
    require_contains(
        &identity_fields,
        "configuration_sha256",
        "module_interface.mandatory_identity",
    )?;
    let unknown_policy = string(module_interface, "unknown_required_field_policy")?;
    if !unknown_policy.contains("UNASSIGNED") || !unknown_policy.contains("DO_NOT_INFER") {
        return Err(
            "unknown module engineering fields must remain unassigned, not inferred".to_owned(),
        );
    }
    let change_effect = string(module_interface, "module_change_effect")?;
    if !change_effect.contains("new whole-train configuration identity") {
        return Err(
            "module substitution must change whole-train configuration identity".to_owned(),
        );
    }

    for module in rows(contract, "candidate_modules")? {
        let id = string(module, "id")?;
        if strings(module, "inputs")?.is_empty()
            || strings(module, "outputs")?.is_empty()
            || strings(module, "hazards")?.is_empty()
            || strings(module, "required_controls")?.is_empty()
        {
            return Err(format!(
                "module {id} has an empty interface, hazard or control list"
            ));
        }
    }

    for required in [
        "M01-SOURCE-SELECTOR",
        "M02-COARSE-SCREEN",
        "M03-EQUALIZATION",
        "M09-NONPOTABLE-RESERVOIR",
        "M10-USE-DELIVERY",
    ] {
        if !modules.contains(required) {
            return Err(format!("required module missing: {required}"));
        }
    }

    for train in rows(contract, "candidate_train_families")? {
        let train_id = string(train, "id")?;
        let selection = string(train, "selection")?;
        if ["production_ready", "qualified", "approved_for_installation"]
            .iter()
            .any(|claim| selection.to_lowercase().contains(claim))
        {
            return Err(format!(
                "train {train_id} promotes an unqualified design claim"
            ));
        }

        for source_id in strings(train, "sources")? {
            if !sources.contains(&source_id) {
                return Err(format!(
                    "train {train_id} references unknown source: {source_id}"
                ));
            }
            let source = rows(contract, "source_classes")?
                .iter()
                .find(|source| source["id"] == source_id)
                .ok_or_else(|| format!("source not found: {source_id}"))?;
            let status = string(source, "status")?;
            if [
                "prohibited_from_greywater_inlet",
                "prohibited_from_initial_reference",
                "excluded_from_initial_reference_train",
            ]
            .contains(&status.as_str())
            {
                return Err(format!(
                    "train {train_id} accepts prohibited source {source_id}"
                ));
            }
            if status == "segregated_challenge_only"
                && !selection.contains("challenge_population_only_no_reuse_credit")
            {
                return Err(format!(
                    "train {train_id} grants reuse eligibility to segregated challenge source {source_id}"
                ));
            }
        }

        let sequence = strings(train, "candidate_sequence")?;
        if sequence.is_empty() {
            return Err(format!("train {train_id} has an empty process sequence"));
        }
        for module_id in &sequence {
            if !modules.contains(module_id) {
                return Err(format!(
                    "train {train_id} references unknown module: {module_id}"
                ));
            }
        }
        if strings(train, "sources")?.contains(&"SRC-LAUNDRY".to_owned()) {
            let allowed = [
                "M01-SOURCE-SELECTOR",
                "M02-COARSE-SCREEN",
                "M03-EQUALIZATION",
            ];
            if sequence
                .iter()
                .any(|module_id| !allowed.contains(&module_id.as_str()))
            {
                return Err(format!(
                    "laundry challenge train {train_id} may not claim a treatment/reuse configuration"
                ));
            }
        }
    }

    let uses = rows(contract, "intended_use_classes")?;
    let drinking = uses
        .iter()
        .find(|use_class| use_class["id"] == "USE-DRINKING-COOKING")
        .ok_or_else(|| "drinking/cooking use class missing".to_owned())?;
    if string(drinking, "status")? != "excluded" {
        return Err("drinking/cooking must remain excluded from this architecture".to_owned());
    }

    let fail_safe = field(contract, "fail_safe_state")?;
    for (key, required) in [
        ("on_invalid_quality_evidence", "ISOLATE_NONPOTABLE_DELIVERY"),
        ("on_power_loss", "STOP_TREATMENT_AND_REUSE_DELIVERY"),
        ("on_barrier_fault", "FAIL_CLOSED"),
        ("on_unknown_source", "REJECT_OR_HOLD_BATCH"),
        ("on_overflow_or_blocked_reject", "STOP_FEED_AND_ALARM"),
    ] {
        if !string(fail_safe, key)?.contains(required) {
            return Err(format!("fail-safe rule {key} is missing {required}"));
        }
    }

    let hydraulic_sizing = string(field(contract, "boundaries")?, "hydraulic_sizing")?;
    if !hydraulic_sizing.contains("unassigned") {
        return Err("unverified hydraulic sizes must remain unassigned".to_owned());
    }

    let nonclaims = strings(contract, "nonclaims")?;
    if nonclaims.len() < 4 {
        return Err("architecture nonclaims are incomplete".to_owned());
    }

    Ok(json!({
        "verification_state": "MatchedArchitectureContract",
        "qualification_status": "NotRun",
        "evidence_class": "design_reference_only",
        "source_count": sources.len(),
        "module_count": modules.len(),
        "train_count": trains.len(),
        "source_segregation_checked": true,
        "module_references_checked": true,
        "laundry_reuse_credit_blocked": true,
        "potable_use_excluded": true,
        "required_fail_safe_rules_checked": 5,
        "nonclaim_count": nonclaims.len(),
        "claim_ceiling": contract["claim_ceiling"]
    }))
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn read_bytes(path: &Path) -> Result<Vec<u8>, String> {
    fs::read(path).map_err(|error| format!("read {}: {error}", path.display()))
}

fn run() -> Result<i32, String> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.len() > 3 {
        return Err(
            "usage: waterloop-modular-t0 [ARCHITECTURE_JSON] [SCHEMA_JSON] [RECEIPT_JSON]"
                .to_owned(),
        );
    }
    let architecture_path =
        PathBuf::from(args.first().map(String::as_str).unwrap_or(
            "research/water-service-engineering/waterloop-modular-architecture-v0.1.json",
        ));
    let schema_path = PathBuf::from(args.get(1).map(String::as_str).unwrap_or(
        "research/water-service-engineering/waterloop-modular-architecture-v0.1.schema.json",
    ));
    let receipt_path = PathBuf::from(
        args.get(2)
            .map(String::as_str)
            .unwrap_or("artifacts/water-service-t0/waterloop-modular-receipt.json"),
    );

    let architecture_bytes = read_bytes(&architecture_path)?;
    let schema_bytes = read_bytes(&schema_path)?;
    let architecture: Value = serde_json::from_slice(&architecture_bytes)
        .map_err(|error| format!("parse {}: {error}", architecture_path.display()))?;
    let schema: Value = serde_json::from_slice(&schema_bytes)
        .map_err(|error| format!("parse {}: {error}", schema_path.display()))?;
    if schema["$schema"] != "https://json-schema.org/draft/2020-12/schema" {
        return Err("unsupported or missing JSON Schema dialect".to_owned());
    }
    let result = validate_architecture(&architecture)?;
    let revision = env::var("WATERLOOP_MODULAR_T0_REVISION")
        .or_else(|_| env::var("GITHUB_SHA"))
        .ok()
        .filter(|value| {
            value.len() == 40
                && value
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        });
    let receipt = json!({
        "schema_version": "waterloop-modular-t0-run-v0.1",
        "verification_state": if revision.is_some() { "PassedExactRevision" } else { "PassedLocalUnpinned" },
        "qualification_status": "NotRun",
        "implementation_revision": revision,
        "architecture_sha256": sha256(&architecture_bytes),
        "schema_sha256": sha256(&schema_bytes),
        "result": result,
        "nonclaims": [
            "This checks architecture structure and source/module compatibility only.",
            "It does not validate the full JSON Schema with a schema engine.",
            "It does not simulate hydraulics or treatment chemistry.",
            "It does not admit water-quality evidence or authorize installation."
        ]
    });
    if let Some(parent) = receipt_path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)
            .map_err(|error| format!("create {}: {error}", parent.display()))?;
    }
    let serialized = serde_json::to_string_pretty(&receipt)
        .map_err(|error| format!("serialize receipt: {error}"))?
        + "\n";
    fs::write(&receipt_path, serialized.as_bytes())
        .map_err(|error| format!("write {}: {error}", receipt_path.display()))?;
    println!("{serialized}");
    Ok(0)
}

fn main() {
    match run() {
        Ok(code) => std::process::exit(code),
        Err(error) => {
            eprintln!("WATERLOOP_MODULAR_T0_ERROR: {error}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Value {
        serde_json::from_str(include_str!(
            "../../../research/water-service-engineering/waterloop-modular-architecture-v0.1.json"
        ))
        .expect("modular architecture fixture")
    }

    #[test]
    fn reference_architecture_passes_structural_contract() {
        let result = validate_architecture(&fixture()).expect("valid architecture");
        assert_eq!(result["source_count"], 8);
        assert_eq!(result["module_count"], 10);
        assert_eq!(result["train_count"], 4);
        assert_eq!(result["qualification_status"], "NotRun");
    }

    #[test]
    fn prohibited_source_cannot_enter_a_candidate_train() {
        let mut contract = fixture();
        contract["candidate_train_families"][0]["sources"]
            .as_array_mut()
            .expect("sources")
            .push(json!("SRC-TOILET-BIDET-FAECAL"));
        assert!(
            validate_architecture(&contract)
                .expect_err("toilet source must be rejected")
                .contains("accepts prohibited source")
        );
    }

    #[test]
    fn laundry_remains_a_segregated_challenge_not_a_reuse_train() {
        let mut contract = fixture();
        contract["candidate_train_families"][2]["candidate_sequence"]
            .as_array_mut()
            .expect("sequence")
            .push(json!("M08-FINAL-DISINFECTION"));
        assert!(
            validate_architecture(&contract)
                .expect_err("laundry challenge cannot become a reuse train")
                .contains("may not claim a treatment/reuse configuration")
        );
    }

    #[test]
    fn module_substitution_requires_a_new_configuration_identity() {
        let mut contract = fixture();
        contract["module_interface"]["module_change_effect"] =
            json!("module may be replaced without qualification impact");
        assert!(
            validate_architecture(&contract)
                .expect_err("module substitution changes configuration identity")
                .contains("whole-train configuration identity")
        );
    }

    #[test]
    fn missing_hydraulic_reject_port_is_rejected() {
        let mut contract = fixture();
        contract["module_interface"]["hydraulic_ports"]
            .as_array_mut()
            .expect("ports")
            .retain(|port| port != "reject_out");
        assert!(
            validate_architecture(&contract)
                .expect_err("reject port required")
                .contains("required value missing: reject_out")
        );
    }

    #[test]
    fn unknown_source_cannot_be_silently_assigned() {
        let mut contract = fixture();
        contract["candidate_train_families"][0]["sources"][0] = json!("SRC-UNKNOWN");
        assert!(
            validate_architecture(&contract)
                .expect_err("unknown source rejected")
                .contains("references unknown source")
        );
    }
}
