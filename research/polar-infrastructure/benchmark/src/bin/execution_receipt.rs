use serde::Deserialize;
use std::collections::BTreeSet;
use std::{env, error::Error, fs};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Receipt {
    schema_version: String,
    artifact_kind: String,
    evidence_class: String,
    status: String,
    receipt_id: String,
    case_id: String,
    controller_id: String,
    execution_state: String,
    execution_performed: bool,
    scoring_eligible: bool,
    execution_attestation: ExecutionAttestation,
    provenance: Provenance,
    paired_input_bindings: InputBindings,
    artifact_inventory: Vec<Artifact>,
    resource_limits: Resources,
    resource_limits_sha256: String,
    observed_resources: NullableResources,
    execution_evidence: ExecutionEvidence,
    independent_verification: IndependentVerification,
    claim_policy: ClaimPolicy,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExecutionAttestation {
    scheme: String,
    status: String,
    issuer_id: Option<String>,
    signature_bundle_sha256: Option<String>,
    signed_payload_sha256: Option<String>,
    verifier_id: Option<String>,
    verifier_revision: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Provenance {
    receipt_producer: String,
    producer_version: String,
    receipt_created_at_utc: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct InputBindings {
    trajectory_id: String,
    trajectory_sha256: String,
    initial_state_id: String,
    initial_state_sha256: String,
    controller_artifact_sha256: String,
    #[serde(default)]
    information_budget_sha256: Option<String>,
    #[serde(default)]
    observation_trace_sha256: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Artifact {
    artifact_id: String,
    role: String,
    source_uri: Option<String>,
    payload: Option<String>,
    sha256: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Resources {
    wall_time_ms: u64,
    cpu_time_ms: u64,
    peak_memory_bytes: u64,
    simulator_calls: u64,
    observation_count: u64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct NullableResources {
    wall_time_ms: Option<u64>,
    cpu_time_ms: Option<u64>,
    peak_memory_bytes: Option<u64>,
    simulator_calls: Option<u64>,
    observation_count: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExecutionEvidence {
    started_at_utc: Option<String>,
    ended_at_utc: Option<String>,
    process_exit_code: Option<i64>,
    stdout_sha256: Option<String>,
    stderr_sha256: Option<String>,
    output_trace_sha256: Option<String>,
    output_artifacts: Vec<Artifact>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct IndependentVerification {
    status: String,
    verifier_id: Option<String>,
    verifier_revision: Option<String>,
    verified_artifact_count: usize,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ClaimPolicy {
    synthetic_payloads_are_execution_evidence: bool,
    scoring_requires_completed_execution: bool,
    scoring_requires_independent_output_hash_verification: bool,
    resource_limits_must_be_shared_across_comparison_arms: bool,
    scoring_requires_signed_execution_attestation: bool,
    signed_execution_attestation_verifier_implemented: bool,
}

fn nonempty(s: &str) -> bool {
    !s.trim().is_empty()
}

fn valid_sha256(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn valid_revision(s: &str) -> bool {
    (s.len() == 40 || s.len() == 64)
        && s.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn resources_empty(r: &NullableResources) -> bool {
    r.wall_time_ms.is_none()
        && r.cpu_time_ms.is_none()
        && r.peak_memory_bytes.is_none()
        && r.simulator_calls.is_none()
        && r.observation_count.is_none()
}

fn resources_present(r: &NullableResources) -> bool {
    r.wall_time_ms.is_some()
        && r.cpu_time_ms.is_some()
        && r.peak_memory_bytes.is_some()
        && r.simulator_calls.is_some()
        && r.observation_count.is_some()
}

fn valid_artifact(a: &Artifact) -> bool {
    nonempty(&a.artifact_id)
        && valid_sha256(&a.sha256)
        && match (&a.source_uri, &a.payload) {
            (Some(uri), None) => nonempty(uri),
            (None, Some(payload)) => !payload.is_empty(),
            _ => false,
        }
}

fn validate_failed_outputs(e: &ExecutionEvidence) -> Result<(), String> {
    let allowed: BTreeSet<&str> = ["output_trace", "stdout", "stderr"].into_iter().collect();
    let mut roles = BTreeSet::new();
    let mut ids = BTreeSet::new();
    let mut by_role = std::collections::BTreeMap::new();
    for artifact in &e.output_artifacts {
        if !valid_artifact(artifact) || !allowed.contains(artifact.role.as_str())
            || !roles.insert(artifact.role.as_str()) || !ids.insert(artifact.artifact_id.as_str())
        {
            return Err("failed receipt contains an invalid or duplicate output artifact".into());
        }
        by_role.insert(artifact.role.as_str(), artifact.sha256.as_str());
    }
    for (role, indexed) in [
        ("output_trace", e.output_trace_sha256.as_deref()),
        ("stdout", e.stdout_sha256.as_deref()),
        ("stderr", e.stderr_sha256.as_deref()),
    ] {
        match (by_role.get(role), indexed) {
            (Some(actual), Some(declared)) if *actual == declared => {}
            (None, None) => {}
            _ => return Err(format!("failed receipt {role} digest index disagrees with output inventory")),
        }
    }
    Ok(())
}

fn validate(r: &Receipt) -> Result<(), String> {
    if !["0.1.0", "0.2.0"].contains(&r.schema_version.as_str())
        || r.artifact_kind != "symtropy-controller-execution-receipt"
        || !nonempty(&r.receipt_id)
        || !nonempty(&r.case_id)
        || !nonempty(&r.controller_id)
        || !nonempty(&r.provenance.receipt_producer)
        || !nonempty(&r.provenance.producer_version)
    {
        return Err("receipt header or provenance is invalid".into());
    }
    for digest in [
        &r.paired_input_bindings.trajectory_sha256,
        &r.paired_input_bindings.initial_state_sha256,
        &r.paired_input_bindings.controller_artifact_sha256,
    ] {
        if !valid_sha256(digest) {
            return Err("paired input binding has invalid SHA-256 syntax".into());
        }
    }
    if r.schema_version == "0.1.0" {
        if r.paired_input_bindings.information_budget_sha256.is_some()
            || r.paired_input_bindings.observation_trace_sha256.is_some()
        {
            return Err("v0.1 receipt must not claim v0.2 causal information bindings".into());
        }
    } else if !r.paired_input_bindings.information_budget_sha256.as_deref().map(valid_sha256).unwrap_or(false)
        || !r.paired_input_bindings.observation_trace_sha256.as_deref().map(valid_sha256).unwrap_or(false)
    {
        return Err("v0.2 receipt requires valid information-budget and observation-trace SHA-256 bindings".into());
    }
    if !valid_sha256(&r.resource_limits_sha256) {
        return Err("resource_limits_sha256 has invalid SHA-256 syntax".into());
    }
    if !nonempty(&r.paired_input_bindings.trajectory_id)
        || !nonempty(&r.paired_input_bindings.initial_state_id)
    {
        return Err("paired input binding IDs cannot be empty".into());
    }
    if [
        r.resource_limits.wall_time_ms,
        r.resource_limits.cpu_time_ms,
        r.resource_limits.peak_memory_bytes,
        r.resource_limits.simulator_calls,
        r.resource_limits.observation_count,
    ].contains(&0) {
        return Err("resource limits must all be positive".into());
    }

    let required_roles: BTreeSet<&str> = [
        "plant_model",
        "run_configuration",
        "executable",
        "runtime_environment",
        "observation_policy",
        "resource_policy",
        "command",
        "controller_artifact",
    ].into_iter().collect();
    let mut inventory_ids = BTreeSet::new();
    let mut inventory_roles = BTreeSet::new();
    for a in &r.artifact_inventory {
        if !valid_artifact(a) {
            return Err(format!("invalid artifact inventory entry: {}", a.artifact_id));
        }
        if !inventory_ids.insert(a.artifact_id.as_str()) || !inventory_roles.insert(a.role.as_str()) {
            return Err("artifact inventory has duplicate IDs or roles".into());
        }
    }
    if inventory_roles != required_roles {
        return Err("artifact inventory must contain exactly one of each required input role".into());
    }
    let controller_artifact = r.artifact_inventory.iter().find(|a| a.role == "controller_artifact")
        .expect("required role was checked");
    if controller_artifact.sha256 != r.paired_input_bindings.controller_artifact_sha256 {
        return Err("controller artifact inventory digest does not match paired input binding".into());
    }
    let resource_policy = r.artifact_inventory.iter().find(|a| a.role == "resource_policy")
        .expect("required role was checked");
    let expected_resource_payload = format!(
        "{{\\"cpu_time_ms\\":{},\\"observation_count\\":{},\\"peak_memory_bytes\\":{},\\"simulator_calls\\":{},\\"wall_time_ms\\":{}}}",
        r.resource_limits.cpu_time_ms,
        r.resource_limits.observation_count,
        r.resource_limits.peak_memory_bytes,
        r.resource_limits.simulator_calls,
        r.resource_limits.wall_time_ms
    );
    if resource_policy.sha256 != r.resource_limits_sha256
        || resource_policy.payload.as_deref() != Some(expected_resource_payload.as_str())
    {
        return Err("resource policy artifact must match canonical numeric resource limits and digest".into());
    }

    if r.claim_policy.synthetic_payloads_are_execution_evidence
        || !r.claim_policy.scoring_requires_completed_execution
        || !r.claim_policy.scoring_requires_independent_output_hash_verification
        || !r.claim_policy.resource_limits_must_be_shared_across_comparison_arms
        || !r.claim_policy.scoring_requires_signed_execution_attestation
        || r.claim_policy.signed_execution_attestation_verifier_implemented
    {
        return Err("claim policy weakens execution, verification, signed-attestation, or equal-resource-budget requirements".into());
    }
    if r.scoring_eligible
        || r.execution_attestation.scheme != "none"
        || r.execution_attestation.status != "not_implemented"
        || r.execution_attestation.issuer_id.is_some()
        || r.execution_attestation.signature_bundle_sha256.is_some()
        || r.execution_attestation.signed_payload_sha256.is_some()
        || r.execution_attestation.verifier_id.is_some()
        || r.execution_attestation.verifier_revision.is_some()
    {
        return Err("v0.1 receipt contract cannot authorize scoring until a signed execution-attestation verifier is implemented".into());
    }
    match r.independent_verification.status.as_str() {
        "not_run" => {
            if r.independent_verification.verifier_id.is_some()
                || r.independent_verification.verifier_revision.is_some()
                || r.independent_verification.verified_artifact_count != 0
            {
                return Err("not_run verification must not claim verifier identity or verified artifacts".into());
            }
        }
        "passed" => {
            if !r.independent_verification.verifier_id.as_deref().map(nonempty).unwrap_or(false)
                || !r.independent_verification.verifier_revision.as_deref().map(valid_revision).unwrap_or(false)
                || r.independent_verification.verified_artifact_count < 3
            {
                return Err("passed verification requires a pinned verifier revision and at least three verified artifacts".into());
            }
        }
        "failed" => {}
        _ => return Err("unsupported independent verification status".into()),
    }

    match r.status.as_str() {
        "design_only_not_executed" => {
            if r.evidence_class != "synthetic_fixture"
                || r.execution_state != "not_executed"
                || r.execution_performed
                || r.scoring_eligible
                || r.provenance.receipt_created_at_utc.is_some()
                || !resources_empty(&r.observed_resources)
                || r.execution_evidence.started_at_utc.is_some()
                || r.execution_evidence.ended_at_utc.is_some()
                || r.execution_evidence.process_exit_code.is_some()
                || r.execution_evidence.stdout_sha256.is_some()
                || r.execution_evidence.stderr_sha256.is_some()
                || r.execution_evidence.output_trace_sha256.is_some()
                || !r.execution_evidence.output_artifacts.is_empty()
                || r.independent_verification.status != "not_run"
                || r.independent_verification.verifier_id.is_some()
                || r.independent_verification.verifier_revision.is_some()
                || r.independent_verification.verified_artifact_count != 0
            {
                return Err("design-only receipt must not claim execution, measurements, verification, or scoring eligibility".into());
            }
        }
        "completed" => {
            if r.evidence_class == "synthetic_fixture"
                || r.execution_state != "completed"
                || !r.execution_performed
                || r.provenance.receipt_created_at_utc.as_deref().map(nonempty) != Some(true)
                || r.execution_evidence.started_at_utc.as_deref().map(nonempty) != Some(true)
                || r.execution_evidence.ended_at_utc.as_deref().map(nonempty) != Some(true)
                || r.execution_evidence.process_exit_code != Some(0)
                || !resources_present(&r.observed_resources)
            {
                return Err("completed receipt is missing executed-run evidence or measurements".into());
            }
            for digest in [
                r.execution_evidence.stdout_sha256.as_deref(),
                r.execution_evidence.stderr_sha256.as_deref(),
                r.execution_evidence.output_trace_sha256.as_deref(),
            ] {
                if !digest.map(valid_sha256).unwrap_or(false) {
                    return Err("completed receipt output digests must be valid SHA-256".into());
                }
            }
            let mut output_roles = BTreeSet::new();
            let mut output_ids = BTreeSet::new();
            for a in &r.execution_evidence.output_artifacts {
                if !valid_artifact(a) || !output_roles.insert(a.role.as_str()) || !output_ids.insert(a.artifact_id.as_str()) {
                    return Err("completed receipt has invalid or duplicate output artifacts".into());
                }
            }
            let required_outputs: BTreeSet<&str> = ["output_trace", "stdout", "stderr"].into_iter().collect();
            if output_roles != required_outputs {
                return Err("completed receipt must bind output trace, stdout, and stderr artifacts".into());
            }
            let output_by_role: std::collections::BTreeMap<&str, &Artifact> = r.execution_evidence.output_artifacts.iter()
                .map(|a| (a.role.as_str(), a)).collect();
            if r.execution_evidence.output_trace_sha256.as_deref()
                    != output_by_role.get("output_trace").map(|a| a.sha256.as_str())
                || r.execution_evidence.stdout_sha256.as_deref()
                    != output_by_role.get("stdout").map(|a| a.sha256.as_str())
                || r.execution_evidence.stderr_sha256.as_deref()
                    != output_by_role.get("stderr").map(|a| a.sha256.as_str())
            {
                return Err("output digest index does not match the output artifact inventory".into());
            }
        }
        "failed" => {
            if r.evidence_class == "synthetic_fixture"
                || r.execution_state != "failed" || !r.execution_performed || r.scoring_eligible
                || r.provenance.receipt_created_at_utc.as_deref().map(nonempty) != Some(true)
                || r.execution_evidence.started_at_utc.as_deref().map(nonempty) != Some(true)
                || r.execution_evidence.ended_at_utc.as_deref().map(nonempty) != Some(true)
                || r.execution_evidence.process_exit_code.is_none()
                || !resources_present(&r.observed_resources)
            {
                return Err("failed run must have non-synthetic execution evidence and cannot be score eligible".into());
            }
            validate_failed_outputs(&r.execution_evidence)?;
        }
        "invalid" => {
            if r.execution_state != "invalid" || r.scoring_eligible {
                return Err("invalid receipt cannot be score eligible and must have invalid state".into());
            }
        }
        _ => return Err("unsupported receipt status".into()),
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let path = env::args().nth(1).ok_or("usage: execution_receipt <receipt.json>")?;
    let raw = fs::read_to_string(path)?;
    let receipt: Receipt = serde_json::from_str(&raw)?;
    validate(&receipt).map_err(|e| format!("execution receipt rejected: {e}"))?;
    println!("{}", serde_json::json!({
        "schema_version": "0.1.0",
        "report_kind": "controller-execution-receipt-validation",
        "contract_valid": true,
        "status": receipt.status,
        "execution_performed": receipt.execution_performed,
        "scoring_eligible": receipt.scoring_eligible,
        "independent_verification_status": receipt.independent_verification.status,
        "candidate_for_performance_claim": false,
        "result_scope": "receipt metadata and gate consistency only; payload hashes and paired input bindings require independent verifier"
    }));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load() -> Receipt {
        serde_json::from_str(include_str!("../../fixtures/controller-execution-receipt-template-2026.json")).unwrap()
    }

    fn completed_receipt() -> Receipt {
        let mut r = load();
        r.evidence_class = "simulator".into();
        r.status = "completed".into();
        r.execution_state = "completed".into();
        r.execution_performed = true;
        r.provenance.receipt_created_at_utc = Some("2026-10-29T00:00:00Z".into());
        r.observed_resources = NullableResources {
            wall_time_ms: Some(1000),
            cpu_time_ms: Some(800),
            peak_memory_bytes: Some(134217728),
            simulator_calls: Some(200),
            observation_count: Some(200),
        };
        r.execution_evidence = ExecutionEvidence {
            started_at_utc: Some("2026-10-29T00:00:00Z".into()),
            ended_at_utc: Some("2026-10-29T00:00:01Z".into()),
            process_exit_code: Some(0),
            stdout_sha256: Some("a".repeat(64)),
            stderr_sha256: Some("b".repeat(64)),
            output_trace_sha256: Some("c".repeat(64)),
            output_artifacts: vec![
                Artifact { artifact_id: "trace".into(), role: "output_trace".into(), source_uri: None, payload: Some("trace mock".into()), sha256: "c".repeat(64) },
                Artifact { artifact_id: "stdout".into(), role: "stdout".into(), source_uri: None, payload: Some("stdout mock".into()), sha256: "a".repeat(64) },
                Artifact { artifact_id: "stderr".into(), role: "stderr".into(), source_uri: None, payload: Some("stderr mock".into()), sha256: "b".repeat(64) },
            ],
        };
        r
    }

    #[test]
    fn accepts_completed_receipt_as_not_yet_score_eligible() {
        let r = completed_receipt();
        validate(&r).unwrap();
        assert!(!r.scoring_eligible);
    }

    #[test]
    fn rejects_output_index_that_disagrees_with_output_artifact() {
        let mut r = completed_receipt();
        r.execution_evidence.stdout_sha256 = Some("d".repeat(64));
        assert!(validate(&r).unwrap_err().contains("output digest index"));
    }

    #[test]
    fn rejects_score_eligibility_before_signed_attestation() {
        let mut r = completed_receipt();
        r.scoring_eligible = true;
        r.independent_verification = IndependentVerification {
            status: "passed".into(),
            verifier_id: Some("independent-verifier".into()),
            verifier_revision: Some("a".repeat(40)),
            verified_artifact_count: 3,
        };
        r.observed_resources.wall_time_ms = Some(r.resource_limits.wall_time_ms + 1);
        assert!(validate(&r).unwrap_err().contains("cannot authorize scoring"));
    }

    #[test]
    fn rejects_self_asserted_score_eligibility_without_attestation() {
        let mut r = completed_receipt();
        r.scoring_eligible = true;
        assert!(validate(&r).unwrap_err().contains("cannot authorize scoring"));
    }

    #[test]
    fn design_only_template_is_valid_but_not_score_eligible() {
        let r = load();
        validate(&r).unwrap();
        assert!(!r.scoring_eligible);
        assert!(!r.execution_performed);
    }

    #[test]
    fn rejects_synthetic_execution_claim() {
        let mut r = load();
        r.execution_performed = true;
        assert!(validate(&r).unwrap_err().contains("must not claim execution"));
    }

    #[test]
    fn rejects_synthetic_score_eligibility() {
        let mut r = load();
        r.scoring_eligible = true;
        assert!(validate(&r).unwrap_err().contains("cannot authorize scoring"));
    }

    #[test]
    fn rejects_self_asserted_signature_attestation() {
        let mut r = load();
        r.execution_attestation.scheme = "sigstore_bundle".into();
        r.execution_attestation.status = "verified".into();
        assert!(validate(&r).unwrap_err().contains("cannot authorize scoring"));
    }

    #[test]
    fn rejects_bad_paired_input_digest() {
        let mut r = load();
        r.paired_input_bindings.trajectory_sha256 = "bad".into();
        assert!(validate(&r).unwrap_err().contains("invalid SHA-256"));
    }

    #[test]
    fn rejects_payload_digest_format_error() {
        let mut r = load();
        r.artifact_inventory[0].sha256 = "bad".into();
        assert!(validate(&r).unwrap_err().contains("invalid artifact inventory"));
    }

    #[test]
    fn rejects_weakening_claim_policy() {
        let mut r = load();
        r.claim_policy.resource_limits_must_be_shared_across_comparison_arms = false;
        assert!(validate(&r).unwrap_err().contains("weakens"));
    }

    #[test]
    fn rejects_missing_required_input_role() {
        let mut r = load();
        r.artifact_inventory.pop();
        assert!(validate(&r).unwrap_err().contains("exactly one"));
    }

    #[test]
    fn rejects_controller_artifact_inventory_digest_drift() {
        let mut r = load();
        let artifact = r.artifact_inventory.iter_mut()
            .find(|a| a.role == "controller_artifact").unwrap();
        artifact.sha256 = "a".repeat(64);
        assert!(validate(&r).unwrap_err().contains("controller artifact inventory digest"));
    }

    #[test]
    fn rejects_resource_policy_payload_that_differs_from_numeric_limits() {
        let mut r = load();
        let artifact = r.artifact_inventory.iter_mut()
            .find(|a| a.role == "resource_policy").unwrap();
        artifact.payload = Some("{\\"wall_time_ms\\":1}".into());
        assert!(validate(&r).unwrap_err().contains("canonical numeric resource limits"));
    }
}
