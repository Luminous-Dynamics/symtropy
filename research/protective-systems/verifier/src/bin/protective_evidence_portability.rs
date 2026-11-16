//! E2 portability gate for W3C PROV-O and RO-Crate 1.3.
//!
//! Synthetic-only assurance: byte identity and graph/metadata semantics are
//! validated, but source authentication, license authenticity, model validity
//! and physical resilience are explicitly outside this executable's claim.
//!
//! The provenance checker enforces the E2 compact-JSON-LD profile. It does not
//! implement the full JSON-LD 1.1 processing algorithms or all W3C PROV-DM
//! normalization, validity and equivalence constraints; those require an
//! independent standards processor/validator stage.

use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::process;

const CRATE: &str = include_str!(
    "../../../fixtures/protective-evidence-crate-v0.1/ro-crate-metadata.json"
);
const PROV: &str = include_str!(
    "../../../fixtures/protective-evidence-crate-v0.1/provenance/e2-sparse-derivation.jsonld"
);
const E1_ENVELOPE: &str =
    include_str!("../../../fixtures/protective-model-evidence-envelope-v0.1.json");

const RO_CONTEXT: &str = "https://w3id.org/ro/crate/1.3/context";
const RO_SPEC: &str = "https://w3id.org/ro/crate/1.3";
const PROV_NS: &str = "http://www.w3.org/ns/prov#";
const E2_NS: &str = "https://luminousdynamics.io/protect-evidence/e2/";
const CLAIM_CEILING: &str = "SYNTHETIC_ENVELOPE_AND_ARTIFACT_INTEGRITY_ONLY";
const RIGHTS_STATUS: &str = "SYNTHETIC_OPEN_FIXTURE";
const PERMITTED_USE: &str = "SYNTHETIC_TEST_ONLY";

const ACTIVITY: &str = "ex:activity/analysis-001";
const AGENT: &str = "ex:agent/synthetic-provider";
const INPUT_WATER: &str = "ex:entity/input-water-demand";
const INPUT_POWER: &str = "ex:entity/input-power-availability";
const OUTPUT_WATER: &str = "ex:entity/output-water-service";
const OUTPUT_CLINIC: &str = "ex:entity/output-clinic-access";

const ARTIFACTS: [(&str, &[u8], &str); 7] = [
    (
        "model-evidence-metadata/synthetic-input-dataset.json",
        include_bytes!(
            "../../../fixtures/protective-evidence-crate-v0.1/model-evidence-metadata/synthetic-input-dataset.json"
        ),
        "application/json",
    ),
    (
        "model-evidence-metadata/synthetic-input-files.json",
        include_bytes!(
            "../../../fixtures/protective-evidence-crate-v0.1/model-evidence-metadata/synthetic-input-files.json"
        ),
        "application/json",
    ),
    (
        "model-evidence-payloads/synthetic-input.json",
        include_bytes!(
            "../../../fixtures/protective-evidence-crate-v0.1/model-evidence-payloads/synthetic-input.json"
        ),
        "application/json",
    ),
    (
        "model-evidence-metadata/synthetic-output-dataset.json",
        include_bytes!(
            "../../../fixtures/protective-evidence-crate-v0.1/model-evidence-metadata/synthetic-output-dataset.json"
        ),
        "application/json",
    ),
    (
        "model-evidence-metadata/synthetic-output-files.json",
        include_bytes!(
            "../../../fixtures/protective-evidence-crate-v0.1/model-evidence-metadata/synthetic-output-files.json"
        ),
        "application/json",
    ),
    (
        "model-evidence-payloads/synthetic-output.json",
        include_bytes!(
            "../../../fixtures/protective-evidence-crate-v0.1/model-evidence-payloads/synthetic-output.json"
        ),
        "application/json",
    ),
    (
        "provenance/e2-sparse-derivation.jsonld",
        include_bytes!(
            "../../../fixtures/protective-evidence-crate-v0.1/provenance/e2-sparse-derivation.jsonld"
        ),
        "application/ld+json",
    ),
];

#[derive(Debug, Serialize)]
struct GateReport {
    schema_version: &'static str,
    ro_crate_specification: &'static str,
    crate_file_count: usize,
    provenance_entity_count: usize,
    provenance_activity_count: usize,
    provenance_agent_count: usize,
    used_input_count: usize,
    generated_output_count: usize,
    declared_derivation_count: usize,
    verified_artifact_count: usize,
    crate_metadata_sha256: String,
    provenance_graph_sha256: String,
    json_ld_processing_scope: &'static str,
    prov_conformance_scope: &'static str,
    disposition: &'static str,
    structural_checks_passed: bool,
    claim_ceiling: &'static str,
}

fn required_text<'a>(value: &'a Value, key: &str, path: &str) -> Result<&'a str, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .ok_or_else(|| format!("{path}.{key} must be non-empty text"))
}

fn entity_id<'a>(value: &'a Value, path: &str) -> Result<&'a str, String> {
    required_text(value, "@id", path)
}

fn reference_id<'a>(value: &'a Value, path: &str) -> Result<&'a str, String> {
    required_text(value, "@id", path)
}

fn entity_type(value: &Value) -> Option<&str> {
    value.get("@type").and_then(Value::as_str)
}

fn require_type(value: &Value, expected: &str, path: &str) -> Result<(), String> {
    if entity_type(value) == Some(expected) {
        Ok(())
    } else {
        Err(format!("{path}.@type must equal {expected}"))
    }
}

fn unique_graph(value: &Value, label: &str) -> Result<BTreeMap<String, Value>, String> {
    let graph = value
        .get("@graph")
        .and_then(Value::as_array)
        .filter(|items| !items.is_empty())
        .ok_or_else(|| format!("{label} @graph must be a non-empty array"))?;
    let mut entities = BTreeMap::new();
    for (index, entity) in graph.iter().enumerate() {
        let path = format!("{label} @graph[{index}]");
        let id = entity_id(entity, &path)?.to_owned();
        if entities.insert(id.clone(), entity.clone()).is_some() {
            return Err(format!("duplicate {label} @id: {id}"));
        }
    }
    Ok(entities)
}

fn validate_claim_and_rights(value: &Value, path: &str) -> Result<(), String> {
    if required_text(value, "claimCeiling", path)? != CLAIM_CEILING {
        return Err(format!("{path}.claimCeiling escalates the synthetic claim ceiling"));
    }
    if required_text(value, "rightsStatus", path)? != RIGHTS_STATUS {
        return Err(format!("{path}.rightsStatus must remain {RIGHTS_STATUS}"));
    }
    if required_text(value, "permittedUse", path)? != PERMITTED_USE {
        return Err(format!("{path}.permittedUse must remain {PERMITTED_USE}"));
    }
    if value.get("license").is_some() {
        return Err(format!("{path}.license asserts an unauthenticated license grant"));
    }
    Ok(())
}

fn validate_crate(crate_text: &str, prov_text: &str) -> Result<GateReport, String> {
    let crate_root: Value = serde_json::from_str(crate_text)
        .map_err(|error| format!("RO-Crate JSON invalid: {error}"))?;
    let prov_root: Value = serde_json::from_str(prov_text)
        .map_err(|error| format!("PROV-O JSON-LD invalid: {error}"))?;
    let e1: Value = serde_json::from_str(E1_ENVELOPE)
        .map_err(|error| format!("E1 envelope JSON invalid: {error}"))?;

    let contexts = crate_root
        .get("@context")
        .and_then(Value::as_array)
        .ok_or_else(|| "RO-Crate @context must be an array".to_owned())?;
    if contexts.first().and_then(Value::as_str) != Some(RO_CONTEXT) {
        return Err("RO-Crate @context must pin the RO-Crate 1.3 context".to_owned());
    }
    let extension = contexts
        .get(1)
        .and_then(Value::as_object)
        .ok_or_else(|| "RO-Crate E2 extension context is required".to_owned())?;
    let required_extensions = [
        ("ex", "https://luminousdynamics.io/protect-evidence/e2/"),
        ("claimCeiling", "ex:claimCeiling"),
        ("rightsStatus", "ex:rightsStatus"),
        ("permittedUse", "ex:permittedUse"),
    ];
    for (key, expected) in required_extensions {
        if extension.get(key).and_then(Value::as_str) != Some(expected) {
            return Err(format!("RO-Crate extension context mapping for {key} is invalid"));
        }
    }

    let crate_entities = unique_graph(&crate_root, "RO-Crate")?;
    let descriptor = crate_entities
        .get("ro-crate-metadata.json")
        .ok_or_else(|| "RO-Crate metadata descriptor is missing".to_owned())?;
    require_type(descriptor, "CreativeWork", "RO-Crate descriptor")?;
    if reference_id(
        descriptor
            .get("conformsTo")
            .ok_or_else(|| "RO-Crate descriptor.conformsTo is required".to_owned())?,
        "RO-Crate descriptor.conformsTo",
    )? != RO_SPEC
    {
        return Err("RO-Crate descriptor must conform to RO-Crate 1.3".to_owned());
    }
    if reference_id(
        descriptor
            .get("about")
            .ok_or_else(|| "RO-Crate descriptor.about is required".to_owned())?,
        "RO-Crate descriptor.about",
    )? != "./"
    {
        return Err("RO-Crate descriptor.about must reference root ./".to_owned());
    }

    let root = crate_entities
        .get("./")
        .ok_or_else(|| "RO-Crate root data entity ./ is missing".to_owned())?;
    require_type(root, "Dataset", "RO-Crate root")?;
    validate_claim_and_rights(root, "RO-Crate root")?;
    if reference_id(
        root.get("publisher")
            .ok_or_else(|| "RO-Crate root.publisher is required".to_owned())?,
        "RO-Crate root.publisher",
    )? != "#synthetic-provider"
    {
        return Err("RO-Crate root.publisher must reference the declared synthetic provider".to_owned());
    }
    require_type(
        crate_entities
            .get("#synthetic-provider")
            .ok_or_else(|| "RO-Crate synthetic provider contextual entity is missing".to_owned())?,
        "Organization",
        "RO-Crate synthetic provider",
    )?;

    let expected_parts: BTreeSet<String> = ARTIFACTS
        .iter()
        .map(|(path, _, _)| (*path).to_owned())
        .collect();
    let has_parts = root
        .get("hasPart")
        .and_then(Value::as_array)
        .ok_or_else(|| "RO-Crate root.hasPart must be an array".to_owned())?;
    let mut part_ids = BTreeSet::new();
    for item in has_parts {
        let id = reference_id(item, "RO-Crate root.hasPart entry")?.to_owned();
        if !part_ids.insert(id.clone()) {
            return Err(format!("duplicate RO-Crate root.hasPart entry: {id}"));
        }
    }
    if part_ids != expected_parts {
        return Err("RO-Crate root.hasPart does not exactly match the frozen artifact inventory".to_owned());
    }
    let file_ids: BTreeSet<String> = crate_entities
        .iter()
        .filter(|(_, value)| entity_type(value) == Some("File"))
        .map(|(id, _)| id.clone())
        .collect();
    if file_ids != expected_parts {
        return Err("RO-Crate File entity inventory does not exactly match the root data entity inventory".to_owned());
    }

    let mut verified_artifacts = 0usize;
    for (path, bytes, media_type) in ARTIFACTS {
        let file = crate_entities
            .get(path)
            .ok_or_else(|| format!("RO-Crate File entity is missing: {path}"))?;
        require_type(file, "File", path)?;
        validate_claim_and_rights(file, path)?;
        if required_text(file, "encodingFormat", path)? != media_type {
            return Err(format!("{path}.encodingFormat is inconsistent"));
        }
        if required_text(file, "contentSize", path)? != format!("{} bytes", bytes.len()) {
            return Err(format!("{path}.contentSize does not match artifact byte length"));
        }
        let actual_sha = format!("{:x}", Sha256::digest(bytes));
        if required_text(file, "sha256", path)? != actual_sha {
            return Err(format!("{path}.sha256 does not match local artifact bytes"));
        }
        verified_artifacts += 1;
    }

    let e1_artifacts: BTreeMap<String, (u64, String)> = e1
        .get("datasets")
        .and_then(Value::as_array)
        .ok_or_else(|| "E1 envelope datasets must be an array".to_owned())?
        .iter()
        .flat_map(|dataset| {
            dataset
                .get("artifacts")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
        })
        .map(|artifact| {
            let path = required_text(artifact, "relative_path", "E1 artifact")?.to_owned();
            let length = artifact
                .get("byte_length")
                .and_then(Value::as_u64)
                .ok_or_else(|| format!("E1 artifact {path}.byte_length is invalid"))?;
            let sha = required_text(artifact, "sha256", &format!("E1 artifact {path}"))?.to_owned();
            Ok((path, (length, sha)))
        })
        .collect::<Result<BTreeMap<_, _>, String>>()?;
    for (path, bytes, _) in ARTIFACTS.iter().take(6) {
        let (length, sha) = e1_artifacts
            .get(*path)
            .ok_or_else(|| format!("E1 envelope does not register {path}"))?;
        if *length != bytes.len() as u64 || sha.as_str() != format!("{:x}", Sha256::digest(bytes)) {
            return Err(format!("{path} differs from the E1 envelope artifact record"));
        }
    }

    let prov_context = prov_root
        .get("@context")
        .and_then(Value::as_object)
        .ok_or_else(|| "PROV-O @context must be an object".to_owned())?;
    if prov_context.get("prov").and_then(Value::as_str) != Some(PROV_NS) {
        return Err("PROV-O context must bind prov to the W3C PROV namespace".to_owned());
    }
    let prov_context = prov_root
        .get("@context")
        .and_then(Value::as_object)
        .ok_or_else(|| "PROV-O @context must be an object".to_owned())?;
    if prov_context.get("prov").and_then(Value::as_str) != Some(PROV_NS) {
        return Err("PROV-O context must bind prov to the W3C PROV namespace".to_owned());
    }
    if prov_context.get("ex").and_then(Value::as_str) != Some(E2_NS) {
        return Err("PROV-O context must bind ex to the E2 fixture namespace".to_owned());
    }

    let prov_entities = unique_graph(&prov_root, "PROV-O")?;
    if prov_entities.len() != 6 {
        return Err(format!("E2 sparse fixture requires six PROV nodes, found {}", prov_entities.len()));
    }

    let activity = prov_entities
        .get(ACTIVITY)
        .ok_or_else(|| "PROV-O analysis Activity is missing".to_owned())?;
    require_type(activity, "prov:Activity", "PROV-O analysis")?;
    let agent = prov_entities
        .get(AGENT)
        .ok_or_else(|| "PROV-O Agent is missing".to_owned())?;
    require_type(agent, "prov:Agent", "PROV-O provider")?;

    let used = activity
        .get("prov:used")
        .and_then(Value::as_array)
        .ok_or_else(|| "PROV-O analysis must contain prov:used".to_owned())?;
    let used_ids: BTreeSet<String> = used
        .iter()
        .map(|item| reference_id(item, "prov:used").map(str::to_owned))
        .collect::<Result<_, _>>()?;
    let expected_used = BTreeSet::from([INPUT_WATER.to_owned(), INPUT_POWER.to_owned()]);
    if used.len() != 2 || used_ids != expected_used {
        return Err("prov:used must contain exactly two unique declared input entities".to_owned());
    }
    if reference_id(
        activity
            .get("prov:wasAssociatedWith")
            .ok_or_else(|| "PROV-O activity.prov:wasAssociatedWith is required".to_owned())?,
        "prov:wasAssociatedWith",
    )? != AGENT
        || activity
            .get("prov:wasAssociatedWith")
            .is_some_and(Value::is_array)
    {
        return Err("PROV-O activity must have exactly one declared associated Agent".to_owned());
    }

    for id in [INPUT_WATER, INPUT_POWER] {
        require_type(
            prov_entities
                .get(id)
                .ok_or_else(|| format!("PROV-O input Entity is missing: {id}"))?,
            "prov:Entity",
            id,
        )?;
    }
    for id in [OUTPUT_WATER, OUTPUT_CLINIC] {
        let output = prov_entities
            .get(id)
            .ok_or_else(|| format!("PROV-O output Entity is missing: {id}"))?;
        require_type(output, "prov:Entity", id)?;
        if reference_id(
            output
                .get("prov:wasGeneratedBy")
                .ok_or_else(|| format!("{id}.prov:wasGeneratedBy is required"))?,
            &format!("{id}.prov:wasGeneratedBy"),
        )? != ACTIVITY
        {
            return Err(format!("{id} was not generated by the declared activity"));
        }
    }

    if reference_id(
        prov_entities[OUTPUT_WATER]
            .get("prov:wasDerivedFrom")
            .ok_or_else(|| "water-service output lacks its one provider-declared derivation".to_owned())?,
        "output-water.prov:wasDerivedFrom",
    )? != INPUT_WATER
    {
        return Err("water-service output derivation differs from the sparse provider declaration".to_owned());
    }
    if prov_entities[OUTPUT_CLINIC].get("prov:wasDerivedFrom").is_some() {
        return Err("clinic-access output must not gain an undeclared derivation edge".to_owned());
    }
    let derivation_count = prov_entities
        .values()
        .filter(|entity| entity.get("prov:wasDerivedFrom").is_some())
        .count();
    if derivation_count != 1 {
        return Err(format!("exactly one provider-declared derivation is allowed, found {derivation_count}"));
    }

    let generation_edge_count = prov_entities
        .values()
        .filter(|value| value.get("prov:wasGeneratedBy").is_some())
        .count();
    if generation_edge_count != 2 {
        return Err(format!(
            "exactly two provider-declared prov:wasGeneratedBy edges are allowed, found {generation_edge_count}"
        ));
    }

    let entity_count = prov_entities
        .values()
        .filter(|value| entity_type(value) == Some("prov:Entity"))
        .count();
    let activity_count = prov_entities
        .values()
        .filter(|value| entity_type(value) == Some("prov:Activity"))
        .count();
    let agent_count = prov_entities
        .values()
        .filter(|value| entity_type(value) == Some("prov:Agent"))
        .count();
    if entity_count != 4 || activity_count != 1 || agent_count != 1 {
        return Err("PROV-O node-type inventory differs from the E2 sparse graph contract".to_owned());
    }

    Ok(GateReport {
        schema_version: "protect-evidence-portability/0.1",
        ro_crate_specification: RO_SPEC,
        crate_file_count: verified_artifacts,
        provenance_entity_count: entity_count,
        provenance_activity_count: activity_count,
        provenance_agent_count: agent_count,
        used_input_count: used_ids.len(),
        generated_output_count: 2,
        declared_derivation_count: derivation_count,
        verified_artifact_count: verified_artifacts,
        crate_metadata_sha256: format!("{:x}", Sha256::digest(crate_text.as_bytes())),
        provenance_graph_sha256: format!("{:x}", Sha256::digest(prov_text.as_bytes())),
        json_ld_processing_scope: "E2 compact-profile checks only; full JSON-LD 1.1 expansion is not implemented",
        prov_conformance_scope: "E2 PROV-O subset only; full PROV-DM constraints and equivalence are not certified",
        disposition: "ABSTAIN_SYNTHETIC_NOT_EXECUTED",
        structural_checks_passed: true,
        claim_ceiling: CLAIM_CEILING,
    })
}

fn run() -> Result<GateReport, String> {
    validate_crate(CRATE, PROV)
}

fn main() {
    match run() {
        Ok(report) => println!(
            "{}",
            serde_json::to_string_pretty(&report).expect("E2 gate report serialization")
        ),
        Err(error) => {
            eprintln!("{error}");
            process::exit(2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn crate_value() -> Value {
        serde_json::from_str(CRATE).expect("embedded RO-Crate JSON")
    }

    fn prov_value() -> Value {
        serde_json::from_str(PROV).expect("embedded PROV-O JSON-LD")
    }

    #[test]
    fn valid_ro_crate_and_sparse_prov_graph_are_accepted() {
        let report = run().expect("valid E2 synthetic package");
        assert_eq!(report.crate_file_count, 7);
        assert_eq!(report.provenance_entity_count, 4);
        assert_eq!(report.provenance_activity_count, 1);
        assert_eq!(report.provenance_agent_count, 1);
        assert_eq!(report.used_input_count, 2);
        assert_eq!(report.generated_output_count, 2);
        assert_eq!(report.declared_derivation_count, 1);
        assert!(report.json_ld_processing_scope.contains("not implemented"));
        assert!(report.prov_conformance_scope.contains("not certified"));
        assert_eq!(report.disposition, "ABSTAIN_SYNTHETIC_NOT_EXECUTED");
    }

    #[test]
    fn all_to_all_derivation_inference_is_rejected() {
        let mut prov = prov_value();
        let clinic = prov["@graph"]
            .as_array_mut()
            .expect("graph")
            .iter_mut()
            .find(|node| node["@id"] == OUTPUT_CLINIC)
            .expect("clinic output");
        clinic["prov:wasDerivedFrom"] = json!({"@id": INPUT_POWER});
        let error = validate_crate(CRATE, &prov.to_string()).expect_err("false derivation");
        assert!(error.contains("clinic-access output must not gain"));
    }

    #[test]
    fn wrong_sparse_derivation_target_is_rejected() {
        let mut prov = prov_value();
        let water = prov["@graph"]
            .as_array_mut()
            .expect("graph")
            .iter_mut()
            .find(|node| node["@id"] == OUTPUT_WATER)
            .expect("water output");
        water["prov:wasDerivedFrom"] = json!({"@id": INPUT_POWER});
        let error = validate_crate(CRATE, &prov.to_string()).expect_err("wrong derivation");
        assert!(error.contains("water-service output derivation differs"));
    }

    #[test]
    fn missing_used_input_is_rejected() {
        let mut prov = prov_value();
        let activity = prov["@graph"]
            .as_array_mut()
            .expect("graph")
            .iter_mut()
            .find(|node| node["@id"] == ACTIVITY)
            .expect("analysis");
        activity["prov:used"] = json!([{"@id": INPUT_WATER}]);
        let error = validate_crate(CRATE, &prov.to_string()).expect_err("missing input");
        assert!(error.contains("prov:used must contain exactly the two declared input entities"));
    }

    #[test]
    fn generated_output_without_activity_is_rejected() {
        let mut prov = prov_value();
        let clinic = prov["@graph"]
            .as_array_mut()
            .expect("graph")
            .iter_mut()
            .find(|node| node["@id"] == OUTPUT_CLINIC)
            .expect("clinic output");
        clinic.as_object_mut().expect("object").remove("prov:wasGeneratedBy");
        let error = validate_crate(CRATE, &prov.to_string()).expect_err("missing generation");
        assert!(error.contains("prov:wasGeneratedBy is required"));
    }

    #[test]
    fn claim_ceiling_escalation_is_rejected() {
        let mut metadata = crate_value();
        let root = metadata["@graph"]
            .as_array_mut()
            .expect("graph")
            .iter_mut()
            .find(|node| node["@id"] == "./")
            .expect("root");
        root["claimCeiling"] = Value::String("PHYSICALLY_VALIDATED".into());
        let error = validate_crate(&metadata.to_string(), PROV).expect_err("claim escalation");
        assert!(error.contains("claimCeiling escalates"));
    }

    #[test]
    fn rights_status_escalation_is_rejected() {
        let mut metadata = crate_value();
        let root = metadata["@graph"]
            .as_array_mut()
            .expect("graph")
            .iter_mut()
            .find(|node| node["@id"] == "./")
            .expect("root");
        root["rightsStatus"] = Value::String("LICENSED".into());
        let error = validate_crate(&metadata.to_string(), PROV).expect_err("rights escalation");
        assert!(error.contains("rightsStatus must remain"));
    }

    #[test]
    fn missing_ro_crate_part_is_rejected() {
        let mut metadata = crate_value();
        let root = metadata["@graph"]
            .as_array_mut()
            .expect("graph")
            .iter_mut()
            .find(|node| node["@id"] == "./")
            .expect("root");
        let _ = root["hasPart"].as_array_mut().expect("parts").pop();
        let error = validate_crate(&metadata.to_string(), PROV).expect_err("missing file part");
        assert!(error.contains("root.hasPart does not exactly match"));
    }

    #[test]
    fn artifact_digest_swap_is_rejected() {
        let mut metadata = crate_value();
        let input = metadata["@graph"]
            .as_array_mut()
            .expect("graph")
            .iter_mut()
            .find(|node| node["@id"] == "model-evidence-payloads/synthetic-input.json")
            .expect("input payload");
        input["sha256"] = Value::String(
            "ce1c4bf7409abfb7e6056cfad06552e694d8f147ca4a8e78242cecd4dcbe8cf6".into(),
        );
        let error = validate_crate(&metadata.to_string(), PROV).expect_err("digest swap");
        assert!(error.contains(".sha256 does not match"));
    }

    #[test]
    fn duplicate_ro_crate_ids_are_rejected() {
        let mut metadata = crate_value();
        let graph = metadata["@graph"].as_array_mut().expect("graph");
        let duplicate = graph[2].clone();
        graph.push(duplicate);
        let error = validate_crate(&metadata.to_string(), PROV).expect_err("duplicate ID");
        assert!(error.contains("duplicate RO-Crate @id"));
    }

    #[test]
    fn context_must_pin_ro_crate_13() {
        let mut metadata = crate_value();
        metadata["@context"][0] = Value::String("https://w3id.org/ro/crate/1.2/context".into());
        let error = validate_crate(&metadata.to_string(), PROV).expect_err("wrong context");
        assert!(error.contains("RO-Crate 1.3 context"));
    }

    #[test]
    fn unauthenticated_license_claim_is_rejected() {
        let mut metadata = crate_value();
        let root = metadata["@graph"]
            .as_array_mut()
            .expect("graph")
            .iter_mut()
            .find(|node| node["@id"] == "./")
            .expect("root");
        root["license"] = json!({"@id":"https://example.invalid/license"});
        let error = validate_crate(&metadata.to_string(), PROV).expect_err("license claim");
        assert!(error.contains("unauthenticated license grant"));
    }

    #[test]
    fn extra_file_entity_not_listed_by_root_is_rejected() {
        let mut metadata = crate_value();
        metadata["@graph"]
            .as_array_mut()
            .expect("graph")
            .push(json!({"@id":"unlisted.bin","@type":"File"}));
        let error = validate_crate(&metadata.to_string(), PROV).expect_err("unlisted File");
        assert!(error.contains("File entity inventory"));
    }

    #[test]
    fn prov_namespace_binding_is_required() {
        let mut prov = prov_value();
        prov["@context"]["prov"] = Value::String("https://example.invalid/prov#".into());
        let error = validate_crate(CRATE, &prov.to_string()).expect_err("wrong PROV namespace");
        assert!(error.contains("W3C PROV namespace"));
    }

    #[test]
    fn duplicate_used_input_is_rejected() {
        let mut prov = prov_value();
        let activity = prov["@graph"]
            .as_array_mut()
            .expect("graph")
            .iter_mut()
            .find(|node| node["@id"] == ACTIVITY)
            .expect("analysis");
        activity["prov:used"] = json!([
            {"@id": INPUT_WATER},
            {"@id": INPUT_WATER}
        ]);
        let error = validate_crate(CRATE, &prov.to_string()).expect_err("duplicate used edge");
        assert!(error.contains("exactly two unique declared input entities"));
    }

    #[test]
    fn unexpected_generation_edge_on_an_input_is_rejected() {
        let mut prov = prov_value();
        let input = prov["@graph"]
            .as_array_mut()
            .expect("graph")
            .iter_mut()
            .find(|node| node["@id"] == INPUT_POWER)
            .expect("power input");
        input["prov:wasGeneratedBy"] = json!({"@id": ACTIVITY});
        let error = validate_crate(CRATE, &prov.to_string()).expect_err("extra generation edge");
        assert!(error.contains("exactly two provider-declared prov:wasGeneratedBy edges"));
    }

    #[test]
    fn e2_namespace_binding_is_required() {
        let mut prov = prov_value();
        prov["@context"]["ex"] = Value::String("https://example.invalid/e2/".into());
        let error = validate_crate(CRATE, &prov.to_string()).expect_err("wrong E2 namespace");
        assert!(error.contains("E2 fixture namespace"));
    }

    #[test]
    fn descriptor_must_point_to_the_root_data_entity() {
        let mut metadata = crate_value();
        let descriptor = metadata["@graph"]
            .as_array_mut()
            .expect("graph")
            .iter_mut()
            .find(|node| node["@id"] == "ro-crate-metadata.json")
            .expect("descriptor");
        descriptor["about"] = json!({"@id":"./wrong"});
        let error = validate_crate(&metadata.to_string(), PROV).expect_err("wrong root");
        assert!(error.contains("descriptor.about"));
    }
}
