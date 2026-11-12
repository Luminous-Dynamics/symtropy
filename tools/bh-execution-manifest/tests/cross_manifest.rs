use bh_execution_manifest::{canonical_input_set_sha256, parse_and_validate_json, sha256_hex, EvidenceStatus, NamedDigest};
use serde_json::Value;

const RESPONSE: &str = include_str!("../../bh-response-manifest/examples/paired-comparison-v0.2.json");
const DIRECT: &[u8] = include_bytes!("../examples/direct-pde.execution-v1.json");
const FREQUENCY: &[u8] = include_bytes!("../examples/frequency-green.execution-v1.json");

#[test]
fn execution_fixtures_parse_and_are_bound_to_response_manifest_methods() {
    let response: Value = serde_json::from_str(RESPONSE).unwrap();
    let methods = response["methods"].as_array().unwrap();
    for (bytes, expected_method) in [(DIRECT, "direct-pde"), (FREQUENCY, "frequency-green")] {
        let receipt = parse_and_validate_json(std::str::from_utf8(bytes).unwrap()).unwrap();
        assert_eq!(receipt.evidence_status, EvidenceStatus::IllustrativeNotExecuted);
        assert_eq!(receipt.method_id, expected_method);
        let method = methods.iter().find(|m| m["method_id"] == expected_method).unwrap();
        assert_eq!(method["execution_manifest_id"].as_str().unwrap(), receipt.manifest_id);
        assert_eq!(method["execution_manifest_sha256"].as_str().unwrap(), sha256_hex(bytes));
        assert_eq!(method["implementation_sha256"].as_str().unwrap(), receipt.implementation_sha256);
        assert_eq!(method["input_set_sha256"].as_str().unwrap(), receipt.invocation.input_set_sha256);
    }
}

#[test]
fn canonical_input_set_is_byte_order_independent_and_hash_case_insensitive() {
    let left = vec![NamedDigest { name: "z".into(), sha256: "B".repeat(64) }, NamedDigest { name: "a".into(), sha256: "A".repeat(64) }];
    let right = vec![NamedDigest { name: "a".into(), sha256: "a".repeat(64) }, NamedDigest { name: "z".into(), sha256: "b".repeat(64) }];
    assert_eq!(canonical_input_set_sha256(&left), canonical_input_set_sha256(&right));
}

#[test]
fn non_executed_fixture_cannot_claim_outputs_or_timestamps() {
    let mut value: Value = serde_json::from_slice(DIRECT).unwrap();
    value["outputs"] = serde_json::json!([{"path":"response.bin","role":"response","size_bytes":1,"sha256":"ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"}]);
    let err = parse_and_validate_json(&serde_json::to_string(&value).unwrap()).unwrap_err();
    assert!(err.to_string().contains("must not contain execution results or output artifacts"));
}

#[test]
fn unknown_fields_are_rejected() {
    let mut value: Value = serde_json::from_slice(DIRECT).unwrap();
    value["unreviewed_claim"] = Value::Bool(true);
    assert!(parse_and_validate_json(&serde_json::to_string(&value).unwrap()).is_err());
}

#[test]
fn completed_receipt_needs_exit_and_output_evidence() {
    let mut value: Value = serde_json::from_slice(DIRECT).unwrap();
    value["evidence_status"] = Value::String("completed".into());
    value["result"]["started_at_unix_ns"] = Value::from(10);
    value["result"]["finished_at_unix_ns"] = Value::from(20);
    value["result"]["elapsed_ns"] = Value::from(10);
    value["result"]["exit_code"] = Value::from(0);
    let err = parse_and_validate_json(&serde_json::to_string(&value).unwrap()).unwrap_err();
    assert!(err.to_string().contains("requires stdout and stderr digests"));
    assert!(err.to_string().contains("requires at least one output artifact"));
}
