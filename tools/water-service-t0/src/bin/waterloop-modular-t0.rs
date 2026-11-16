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

fn boolean(value: &Value, key: &str) -> Result<bool, String> {
    field(value, key)?
        .as_bool()
        .ok_or_else(|| format!("{key}: expected boolean"))
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
        "SRC-BABY-NAPPY-WASH",
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

    for required in [
        "TRAIN-A-BATH-SHOWER-PASSIVE",
        "TRAIN-B-BATH-SHOWER-UF",
        "TRAIN-C-LAUNDRY-SEGREGATED",
        "TRAIN-D-NATURE-BASED",
        "TRAIN-E-BATH-SHOWER-OZONE-GAC",
    ] {
        if !trains.contains(required) {
            return Err(format!("required candidate train missing: {required}"));
        }
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
        ("on_cross_connection_suspected", "LATCH_FAULT"),
    ] {
        if !string(fail_safe, key)?.contains(required) {
            return Err(format!("fail-safe rule {key} is missing {required}"));
        }
    }

    let separation = field(contract, "potable_nonpotable_separation")?;
    for key in [
        "separate_physical_circuits",
        "common_tank_or_pipe_prohibited",
        "cross_connection_test_required",
        "future_makeup_requires_backflow_or_air_gap_review",
        "local_authority_approval_required_before_any_plumbed_makeup",
        "nonpotable_fixture_and_pipe_identification_required",
    ] {
        if !boolean(separation, key)? {
            return Err(format!(
                "potable/non-potable separation safeguard is false: {key}"
            ));
        }
    }
    if boolean(separation, "potable_makeup_connection_in_v0_1")? {
        return Err("v0.1 must not enable a potable makeup connection".to_owned());
    }
    let protection_method = string(separation, "future_makeup_protection_method")?;
    if !protection_method.contains("UNASSIGNED_PENDING")
        || !protection_method.contains("LOCAL_AUTHORITY_APPROVAL")
    {
        return Err(
            "future backflow/air-gap design must remain unassigned pending local approval"
                .to_owned(),
        );
    }
    if !string(separation, "cross_connection_suspected_action")?.contains("LATCH_FAULT") {
        return Err("cross-connection suspicion must latch a fault".to_owned());
    }

    let reject_routing = field(contract, "reject_and_overflow_routing")?;
    if boolean(reject_routing, "bypass_to_product_permitted")?
        || !boolean(reject_routing, "bypass_isolated_from_product")?
    {
        return Err("reject/bypass routing must be physically isolated from product".to_owned());
    }
    for key in ["reject_destination", "overflow_destination"] {
        if !string(reject_routing, key)?.contains("UNASSIGNED_PENDING") {
            return Err(format!(
                "{key} must remain unassigned pending site approval"
            ));
        }
    }
    if !string(reject_routing, "blocked_reject_action")?.contains("STOP_FEED_AND_ALARM")
        || string(reject_routing, "route_authorization")?
            != "NOT_AUTHORIZED_BY_THIS_DESIGN_REFERENCE"
    {
        return Err("reject routing must fail closed and remain unauthorized".to_owned());
    }

    let release = field(contract, "batch_quality_release_policy")?;
    for key in [
        "source_batch_identity_required",
        "module_configuration_identity_required",
        "independent_laboratory_sample_required",
        "sample_before_and_after_train_required",
        "quality_assessment_must_match_exact_intended_use",
        "sample_time_and_storage_age_required",
    ] {
        if !boolean(release, key)? {
            return Err(format!("batch quality release safeguard is false: {key}"));
        }
    }
    if boolean(release, "online_sensor_alone_can_unlock_release")?
        || string(release, "batch_release_authority")?
            != "INDEPENDENT_USE_SPECIFIC_QUALITY_ASSESSMENT_ONLY"
    {
        return Err("online sensors may not independently unlock batch release".to_owned());
    }

    let source_plan = field(contract, "source_characterization_protocol")?;
    let source_populations = strings(source_plan, "source_populations")?;
    for required in ["SRC-LGW-BATH-SHOWER", "SRC-LAUNDRY", "SRC-RAINWATER"] {
        require_contains(
            &source_populations,
            required,
            "source_characterization_protocol.source_populations",
        )?;
        if !sources.contains(required) {
            return Err(format!(
                "source-characterization plan references unknown source: {required}"
            ));
        }
    }
    let survey_days = field(source_plan, "minimum_source_survey_days")?
        .as_u64()
        .ok_or_else(|| "minimum_source_survey_days: expected integer".to_owned())?;
    let batches = field(
        source_plan,
        "minimum_independent_real_feed_batches_per_source_class",
    )?
    .as_u64()
    .ok_or_else(|| {
        "minimum_independent_real_feed_batches_per_source_class: expected integer".to_owned()
    })?;
    if survey_days < 7 || batches < 3 || !boolean(source_plan, "exploratory_only")? {
        return Err("source characterization minima must remain exploratory and include >=7 survey days and >=3 independent batches".to_owned());
    }
    let sample_fields = strings(source_plan, "sample_identity_fields")?;
    for required in [
        "source_id",
        "batch_id",
        "time_zone",
        "storage_age",
        "laboratory",
        "method_revision",
        "uncertainty",
        "chain_of_custody",
    ] {
        require_contains(
            &sample_fields,
            required,
            "source_characterization_protocol.sample_identity_fields",
        )?;
    }
    let common_panel = strings(source_plan, "common_characterization")?;
    for required in [
        "pH",
        "temperature",
        "conductivity",
        "turbidity",
        "TSS",
        "COD",
    ] {
        require_contains(
            &common_panel,
            required,
            "source_characterization_protocol.common_characterization",
        )?;
    }
    let laundry_panel = strings(source_plan, "laundry_characterization")?;
    for required in ["anionic_surfactants", "sodium", "boron", "chloride"] {
        require_contains(
            &laundry_panel,
            required,
            "source_characterization_protocol.laundry_characterization",
        )?;
    }
    if !string(source_plan, "microbial_characterization")?.contains("Mycelix #6100") {
        return Err("microbiological panel must be defined through Mycelix #6100".to_owned());
    }

    let experiment = field(contract, "matched_train_experiment")?;
    let experiment_trains = strings(experiment, "train_ids")?;
    for required in [
        "TRAIN-A-BATH-SHOWER-PASSIVE",
        "TRAIN-B-BATH-SHOWER-UF",
        "TRAIN-C-LAUNDRY-SEGREGATED",
        "TRAIN-D-NATURE-BASED",
        "TRAIN-E-BATH-SHOWER-OZONE-GAC",
    ] {
        require_contains(
            &experiment_trains,
            required,
            "matched_train_experiment.train_ids",
        )?;
        if !trains.contains(required) {
            return Err(format!(
                "matched experiment references unknown train: {required}"
            ));
        }
    }
    let matched_conditions = strings(experiment, "matched_conditions")?;
    for required in [
        "same_source_batch_or_matched_aliquot",
        "same_feed_volume_and_hydraulic_loading",
        "same_temperature_band",
        "same_run_duration_and_sampling_schedule",
        "same_intended_use_profile",
        "same_energy_and_residual_boundaries",
    ] {
        require_contains(
            &matched_conditions,
            required,
            "matched_train_experiment.matched_conditions",
        )?;
    }
    let replicate_count = field(experiment, "minimum_independent_runs_per_train_batch")?
        .as_u64()
        .ok_or_else(|| "minimum_independent_runs_per_train_batch: expected integer".to_owned())?;
    if replicate_count < 3 {
        return Err(
            "matched train experiment needs at least three exploratory replicates".to_owned(),
        );
    }
    for key in [
        "report_absolute_concentrations_and_mass_loads",
        "report_microbiology_separately_from_turbidity_and_COD",
        "report_energy_and_all_auxiliary_loads",
        "report_reject_backwash_concentrate_and_spent_media",
        "no_single_removal_score_may_select_the_train",
    ] {
        if !boolean(experiment, key)? {
            return Err(format!(
                "matched train experiment reporting requirement is false: {key}"
            ));
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
        "required_fail_safe_rules_checked": 6,
        "separation_and_reject_controls_checked": true,
        "batch_quality_release_controls_checked": true,
        "source_characterization_protocol_checked": true,
        "matched_train_experiment_checked": true,
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
            "../../../../research/water-service-engineering/waterloop-modular-architecture-v0.1.json"
        ))
        .expect("modular architecture fixture")
    }

    #[test]
    fn reference_architecture_passes_structural_contract() {
        let result = validate_architecture(&fixture()).expect("valid architecture");
        assert_eq!(result["source_count"], 9);
        assert_eq!(result["module_count"], 10);
        assert_eq!(result["train_count"], 5);
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
    fn baby_and_nappy_wash_remains_excluded() {
        let mut contract = fixture();
        contract["candidate_train_families"][0]["sources"]
            .as_array_mut()
            .expect("sources")
            .push(json!("SRC-BABY-NAPPY-WASH"));
        assert!(
            validate_architecture(&contract)
                .expect_err("nappy wash is prohibited")
                .contains("accepts prohibited source")
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
    fn potable_nonpotable_separation_must_remain_physical() {
        let mut contract = fixture();
        contract["potable_nonpotable_separation"]["separate_physical_circuits"] = json!(false);
        assert!(
            validate_architecture(&contract)
                .expect_err("physical separation is mandatory")
                .contains("potable/non-potable separation safeguard is false")
        );
    }

    #[test]
    fn a_potable_makeup_connection_is_not_enabled_in_v0_1() {
        let mut contract = fixture();
        contract["potable_nonpotable_separation"]["potable_makeup_connection_in_v0_1"] =
            json!(true);
        assert!(
            validate_architecture(&contract)
                .expect_err("makeup connection not authorized")
                .contains("must not enable a potable makeup connection")
        );
    }

    #[test]
    fn failure_bypass_cannot_reach_product() {
        let mut contract = fixture();
        contract["reject_and_overflow_routing"]["bypass_to_product_permitted"] = json!(true);
        assert!(
            validate_architecture(&contract)
                .expect_err("bypass to product rejected")
                .contains("reject/bypass routing must be physically isolated")
        );
    }

    #[test]
    fn batch_quality_cannot_be_unlocked_by_online_sensor_alone() {
        let mut contract = fixture();
        contract["batch_quality_release_policy"]["online_sensor_alone_can_unlock_release"] =
            json!(true);
        assert!(
            validate_architecture(&contract)
                .expect_err("online sensor cannot authorize release")
                .contains("online sensors may not independently unlock")
        );
    }

    #[test]
    fn source_characterization_requires_independent_real_feed_batches() {
        let mut contract = fixture();
        contract["source_characterization_protocol"]["minimum_independent_real_feed_batches_per_source_class"] =
            json!(1);
        assert!(
            validate_architecture(&contract)
                .expect_err("minimum batch count")
                .contains(">=3 independent batches")
        );
    }

    #[test]
    fn ozone_gac_comparator_is_present_as_candidate_not_selected() {
        let contract = fixture();
        let train = contract["candidate_train_families"]
            .as_array()
            .unwrap()
            .iter()
            .find(|train| train["id"] == "TRAIN-E-BATH-SHOWER-OZONE-GAC")
            .expect("ozone/GAC train");
        assert!(
            train["selection"]
                .as_str()
                .unwrap()
                .contains("candidate_for_matched_unoccupied_test_only")
        );
        assert_eq!(train["candidate_sequence"][3], "M06-OZONE-GAC");
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
