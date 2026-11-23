use jsonschema::Validator;
use serde_json::{Value, json};
use std::process::ExitCode;

const SCHEMA_JSON: &str =
    include_str!("../../../schemas/manufacturing/fan_article_release_packet_v0_1.schema.json");
const PACKET_JSON: &str =
    include_str!("../../../fixtures/manufacturing/fan_article_release_packet_v0_1.json");

fn compile_schema() -> Result<Validator, String> {
    let schema: Value =
        serde_json::from_str(SCHEMA_JSON).map_err(|error| format!("schema_parse:{error}"))?;
    jsonschema::draft202012::options()
        .offline()
        .build(&schema)
        .map_err(|error| format!("schema_compile:{error}"))
}

fn validate_packet(validator: &Validator, packet: &Value) -> Result<(), String> {
    validator
        .validate(packet)
        .map_err(|error| format!("packet_validation:{error}"))
}

fn run() -> Result<(), String> {
    let validator = compile_schema()?;
    let packet: Value =
        serde_json::from_str(PACKET_JSON).map_err(|error| format!("packet_parse:{error}"))?;
    validate_packet(&validator, &packet)?;

    let status = packet
        .get("status")
        .and_then(Value::as_str)
        .ok_or_else(|| "packet_status_missing".to_owned())?;
    let claim_ceiling = packet
        .get("claim_ceiling")
        .and_then(Value::as_str)
        .ok_or_else(|| "claim_ceiling_missing".to_owned())?;

    println!(
        "{}",
        json!({
            "schema_id": "FAN-ARTICLE-RELEASE-PACKET-0.1",
            "status": status,
            "schema_valid": true,
            "fabrication_authorized": false,
            "physical_acceptance_claimed": false,
            "claim_ceiling": claim_ceiling
        })
    );
    Ok(())
}

#[cfg(test)]
fn evidence(owner: &str, name: &str) -> Value {
    json!({
        "uri": format!("evidence://{owner}/{name}"),
        "sha256": "a".repeat(64),
        "revision": "rev-1",
        "source_owner": owner
    })
}

#[cfg(test)]
fn set_gate(packet: &mut Value, gate: &str) {
    packet["gates"][gate] = json!({
        "status": "PASS",
        "evidence_refs": [evidence("product-owner", &format!("{gate}-evidence"))],
        "reviewer_id": "reviewer-1",
        "decision_ref": evidence("product-owner", &format!("{gate}-decision")),
        "recorded_at": "2026-11-23T00:00:00Z"
    });
}

#[cfg(test)]
fn released_packet() -> Value {
    // Synthetic-only values for schema tests; not a product specification or release input.
    let mut packet: Value =
        serde_json::from_str(PACKET_JSON).expect("frozen intake template parses");
    packet["status"] = json!("RELEASED_FOR_FABRICATION");

    packet["product"]["product_revision"] = json!("fan-rev-1");
    packet["product"]["operating_environment"] =
        json!("indoor tabletop use; owner-defined envelope");
    packet["product"]["controller_interface"] = json!("documented low-voltage interface");
    packet["product"]["wiring_drawing_ref"] = evidence("product-owner", "wiring-drawing");

    let cartridge = &mut packet["product"]["cartridge"];
    cartridge["manufacturer"] = json!("Example Manufacturer");
    cartridge["part_number"] = json!("FAN-CARTRIDGE-001");
    cartridge["revision"] = json!("rev-1");
    cartridge["datasheet_ref"] = evidence("product-owner", "cartridge-datasheet");
    cartridge["rated_voltage_v"] = json!(12.0);
    cartridge["continuous_current_a"] = json!(1.0);
    cartridge["startup_or_stall_current_a"] = json!(2.0);
    cartridge["polarity"] = json!("documented polarity");
    cartridge["connector"] = json!("documented connector");
    cartridge["pinout_ref"] = evidence("product-owner", "cartridge-pinout");
    cartridge["dimensional_drawing_ref"] = evidence("product-owner", "cartridge-drawing");

    let supply = &mut packet["product"]["external_power_supply"];
    supply["manufacturer"] = json!("Example Manufacturer");
    supply["part_number"] = json!("PSU-001");
    supply["revision"] = json!("rev-1");
    supply["datasheet_ref"] = evidence("product-owner", "supply-datasheet");
    supply["output_voltage_v"] = json!(12.0);
    supply["output_current_a"] = json!(2.0);
    supply["protection_features"] =
        json!("over-current and over-temperature protection per datasheet");
    supply["safety_documentation_ref"] = evidence("product-owner", "supply-safety");

    packet["operation"]["operation_id"] = json!("ASSEMBLY-OP-001");
    packet["operation"]["operation_revision"] = json!("rev-1");
    packet["operation"]["description"] =
        json!("Hold the declared subassembly during the released manual assembly step.");
    packet["operation"]["sequence"] = json!([
        "Locate subassembly",
        "Perform owner-defined assembly step",
        "Remove completed assembly"
    ]);
    packet["operation"]["component_revisions"] =
        json!([{"component_id":"SUBASSEMBLY-001","revision":"rev-1"}]);
    packet["operation"]["contact_surfaces"] = json!(["owner-defined datum contact A"]);
    packet["operation"]["datums"] = json!(["A", "B", "C"]);
    packet["operation"]["loads_and_forces"] = json!(
        "Measured/owner-specified loads and prohibited force envelope are recorded in the linked controlled protocol."
    );
    packet["operation"]["keep_out_zones"] =
        json!(["electrical connector", "rotating impeller envelope"]);
    packet["operation"]["downstream_operation"] = json!("Independent post-assembly inspection");

    packet["geometry"]["controlled_drawing_ref"] = evidence("product-owner", "fixture-drawing");
    packet["geometry"]["native_cad_ref"] = evidence("design-owner", "native-cad");
    packet["geometry"]["cad_revision"] = json!("rev-1");
    packet["geometry"]["cad_sha256"] = json!("b".repeat(64));
    packet["geometry"]["datum_scheme"] = json!("A-B-C per controlled drawing");
    packet["geometry"]["critical_features"] = json!([{
        "feature_id":"FEATURE-001",
        "nominal_value":100.0,
        "lower_tolerance":-0.1,
        "upper_tolerance":0.1,
        "unit":"mm",
        "functional_basis":"supports repeatable location for the released assembly operation",
        "measurement_method_ref":evidence("metrology-owner","feature-method")
    }]);
    packet["geometry"]["clearance_envelope_ref"] = evidence("design-owner", "clearance-envelope");
    packet["geometry"]["measurement_method"] =
        json!("calibrated dimensional inspection per linked protocol");
    packet["geometry"]["instrument_ref"] = evidence("metrology-owner", "instrument");
    packet["geometry"]["calibration_ref"] = evidence("metrology-owner", "calibration");
    packet["geometry"]["measurement_uncertainty"] =
        json!("uncertainty budget linked and evaluated against the functional tolerance");

    packet["process_material"]["material_specification"] = json!("owner-approved material grade");
    packet["process_material"]["material_supplier"] = json!("qualified material supplier");
    packet["process_material"]["material_lot"] = json!("lot-001");
    packet["process_material"]["conditioning"] = json!("condition per material specification");
    packet["process_material"]["environmental_exposure"] =
        json!("indoor assembly environment per owner packet");
    packet["process_material"]["machine_profile_ref"] =
        evidence("manufacturing-owner", "machine-profile");
    packet["process_material"]["process_recipe_ref"] =
        evidence("manufacturing-owner", "process-recipe");
    packet["process_material"]["orientation"] = json!("released build orientation");
    packet["process_material"]["load_deflection_and_creep_basis"] =
        json!("fixture load and service-life basis approved by owner");

    packet["rights"]["status"] = json!("CLEARED");
    packet["rights"]["licence_or_permission_ref"] = evidence("rights-owner", "permission");

    packet["demand_and_baseline"]["demand_classification"] = json!("FORECAST");
    packet["demand_and_baseline"]["quantity"] = json!(1.0);
    packet["demand_and_baseline"]["observation_period_start"] = json!("2026-11-01");
    packet["demand_and_baseline"]["observation_period_end"] = json!("2026-11-23");
    packet["demand_and_baseline"]["demand_source_ref"] =
        evidence("product-owner", "demand-forecast");
    packet["demand_and_baseline"]["baseline_quotes"] = json!([{
        "supplier_or_route":"matched external fixture supplier",
        "quote_ref":evidence("procurement-owner","quote"),
        "quote_date":"2026-11-23",
        "cost":100.0,
        "currency":"ZAR",
        "lead_time_hours":120.0
    }]);

    packet["acceptance"]["fixture_dimension_protocol_ref"] =
        evidence("metrology-owner", "fixture-dimension-protocol");
    packet["acceptance"]["repeatability_protocol_ref"] =
        evidence("quality-owner", "repeatability-protocol");
    packet["acceptance"]["load_or_deflection_test_ref"] =
        evidence("engineering-owner", "load-test-protocol");
    packet["acceptance"]["assembly_trial_protocol_ref"] =
        evidence("product-owner", "assembly-trial-protocol");
    packet["acceptance"]["product_damage_limits"] = json!("no damage beyond product-owner limits");
    packet["acceptance"]["cycle_time_method_ref"] =
        evidence("manufacturing-owner", "cycle-time-method");
    packet["acceptance"]["post_use_verification_ref"] =
        evidence("quality-owner", "post-use-verification");

    for gate in ["G0", "G1", "G2", "G3"] {
        set_gate(&mut packet, gate);
    }
    packet["owner_disposition"] = json!({
        "owner_id":"product-owner-1",
        "reviewer_id":"reviewer-1",
        "disposition":"RELEASED_FOR_FABRICATION",
        "decision_ref":evidence("product-owner","fabrication-release-decision"),
        "recorded_at":"2026-11-23T00:00:00Z"
    });

    packet
}

#[cfg(test)]
mod tests {
    use super::*;

    fn validator() -> Validator {
        compile_schema().expect("Draft 2020-12 schema compiles offline")
    }

    #[test]
    fn incomplete_unknown_intake_template_is_valid_but_not_authorized() {
        let packet: Value = serde_json::from_str(PACKET_JSON).unwrap();
        let validator = validator();
        assert!(validator.is_valid(&packet));
        assert_eq!(packet["status"], "INCOMPLETE");
        assert_eq!(
            packet["product"]["cartridge"]["rated_voltage_v"],
            Value::Null
        );
    }

    #[test]
    fn unexpected_fields_are_rejected() {
        let mut packet: Value = serde_json::from_str(PACKET_JSON).unwrap();
        packet["unexpected_field"] = json!(true);
        assert!(!validator().is_valid(&packet));
    }

    #[test]
    fn release_status_cannot_bypass_unknown_product_or_gate_evidence() {
        let mut packet: Value = serde_json::from_str(PACKET_JSON).unwrap();
        packet["status"] = json!("RELEASED_FOR_FABRICATION");
        assert!(!validator().is_valid(&packet));
    }

    #[test]
    fn released_packet_with_required_owner_evidence_is_schema_valid() {
        let packet = released_packet();
        assert!(validator().is_valid(&packet));
    }

    #[test]
    fn released_packet_with_null_critical_electrical_rating_is_rejected() {
        let mut packet = released_packet();
        packet["product"]["cartridge"]["rated_voltage_v"] = Value::Null;
        assert!(!validator().is_valid(&packet));
    }

    #[test]
    fn accepted_for_use_requires_g4_and_first_article_evidence() {
        let mut packet = released_packet();
        packet["status"] = json!("ACCEPTED_FOR_USE");
        packet["owner_disposition"]["disposition"] = json!("ACCEPTED_FOR_USE");
        packet["acceptance"]["first_article_results_ref"] =
            evidence("quality-owner", "first-article-results");
        assert!(!validator().is_valid(&packet));

        set_gate(&mut packet, "G4");
        assert!(validator().is_valid(&packet));
    }

    #[test]
    fn evidence_digest_must_be_a_sha256_hex_string() {
        let mut packet = released_packet();
        packet["product"]["cartridge"]["datasheet_ref"]["sha256"] = json!("not-a-digest");
        assert!(!validator().is_valid(&packet));
    }

    #[test]
    fn unresolved_rights_cannot_be_released() {
        let mut packet = released_packet();
        packet["rights"]["status"] = json!("UNKNOWN");
        assert!(!validator().is_valid(&packet));
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
