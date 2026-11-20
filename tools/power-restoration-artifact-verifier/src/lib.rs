use ed25519_dalek::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum SignerRole {
    Planner,
    TransitionEngine,
    ElectricalValidator,
    ServiceProvider,
    FieldObserver,
    EvidenceProvider,
    RunRecorder,
    ActuationWitness,
    Scorer,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ArtifactRecord {
    pub artifact_uri: String,
    pub relative_path: String,
    pub media_type: String,
    pub byte_length: u64,
    pub sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BundleManifest {
    pub schema_version: String,
    pub bundle_id: String,
    pub artifacts: Vec<ArtifactRecord>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AttestationPayload {
    pub schema_version: String,
    pub predicate_type: String,
    pub issuer_id: String,
    pub signer_role: SignerRole,
    pub subject_uri: String,
    pub subject_sha256: String,
    pub run_id: String,
    pub case_sha256: String,
    pub decision_step: u32,
    pub proposal_sha256: String,
    pub tool_source_commit: String,
    pub tool_configuration_sha256: String,
    pub issued_at_unix_ms: u64,
    pub expires_at_unix_ms: u64,
    pub parent_artifact_sha256: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AttestationEnvelope {
    pub key_id: String,
    pub payload: AttestationPayload,
    pub signature_hex: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TrustRoot {
    pub schema_version: String,
    pub root_id: String,
    pub valid_from_unix_ms: u64,
    pub valid_until_unix_ms: u64,
    pub keys: Vec<TrustedKey>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TrustedKey {
    pub key_id: String,
    pub producer_id: String,
    pub public_key_hex: String,
    pub roles: Vec<SignerRole>,
    pub not_before_unix_ms: u64,
    pub not_after_unix_ms: u64,
    pub revoked_at_unix_ms: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExpectedAttestationBinding {
    pub predicate_type: String,
    pub issuer_id: String,
    pub artifact_uri: String,
    pub artifact_sha256: String,
    pub run_id: String,
    pub case_sha256: String,
    pub decision_step: u32,
    pub proposal_sha256: String,
    pub signer_role: SignerRole,
    pub tool_source_commit: String,
    pub tool_configuration_sha256: String,
    pub parent_artifact_sha256: Vec<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VerifiedArtifact {
    pub artifact_uri: String,
    pub relative_path: String,
    pub byte_length: u64,
    pub sha256: String,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VerifiedAttestation {
    pub key_id: String,
    pub issuer_id: String,
    pub signer_role: SignerRole,
    pub subject_uri: String,
    pub subject_sha256: String,
    pub trust_root_sha256: String,
    pub signature_status: String,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BundleVerificationReport {
    pub bundle_id: String,
    pub manifest_sha256: String,
    pub artifact_count: usize,
    pub verified_artifacts: Vec<VerifiedArtifact>,
    pub result_status: String,
    pub claim_boundary: String,
}

/// Opaque proof that the bundle manifest and every declared artifact byte
/// were resolved and verified by this crate. Its private field prevents a
/// caller from fabricating a success-shaped report for attestation checking.
#[derive(Clone, Debug, PartialEq)]
pub struct VerifiedBundle {
    report: BundleVerificationReport,
}

impl VerifiedBundle {
    pub fn report(&self) -> &BundleVerificationReport {
        &self.report
    }

    /// Re-read a previously verified artifact and re-check its exact bytes
    /// immediately before a consumer parses them. This avoids parsing a
    /// stale path after the bundle directory has changed since verification.
    pub fn read_verified_artifact_bytes(
        &self,
        bundle_root: &Path,
        artifact_uri: &str,
        max_artifact_bytes: u64,
    ) -> Result<Vec<u8>, Vec<String>> {
        let Some(artifact) = self.report.verified_artifacts.iter()
            .find(|artifact| artifact.artifact_uri == artifact_uri)
        else {
            return Err(vec![format!(
                "artifact {artifact_uri} is not part of the verified bundle"
            )]);
        };

        if artifact.byte_length == 0 || artifact.byte_length > max_artifact_bytes {
            return Err(vec![format!(
                "verified artifact {artifact_uri} exceeds the requested byte limit"
            )]);
        }
        let relative_path = validate_relative_path(&artifact.relative_path)
            .map_err(|error| vec![format!("verified artifact {artifact_uri}: {error}")])?;
        let root_metadata = fs::symlink_metadata(bundle_root)
            .map_err(|error| vec![format!("cannot inspect bundle root: {error}")])?;
        if root_metadata.file_type().is_symlink() || !root_metadata.is_dir() {
            return Err(vec!["bundle root must be a real directory, not a symlink".to_string()]);
        }
        let canonical_root = fs::canonicalize(bundle_root)
            .map_err(|error| vec![format!("cannot canonicalize bundle root: {error}")])?;
        let mut current = canonical_root.clone();
        let components: Vec<_> = relative_path.components().collect();
        for (index, component) in components.iter().enumerate() {
            current.push(component.as_os_str());
            let metadata = fs::symlink_metadata(&current)
                .map_err(|error| vec![format!("verified artifact path component is missing: {error}")])?;
            if metadata.file_type().is_symlink() {
                return Err(vec!["verified artifact paths may not traverse symlinks".to_string()]);
            }
            if index + 1 < components.len() && !metadata.is_dir() {
                return Err(vec!["non-final verified artifact path component is not a directory".to_string()]);
            }
            if index + 1 == components.len() && !metadata.is_file() {
                return Err(vec!["verified artifact target must be a regular file".to_string()]);
            }
        }
        let canonical_target = fs::canonicalize(&current)
            .map_err(|error| vec![format!("cannot canonicalize verified artifact path: {error}")])?;
        if !canonical_target.starts_with(&canonical_root) {
            return Err(vec!["verified artifact path escapes the bundle root".to_string()]);
        }

        let bytes = fs::read(&canonical_target)
            .map_err(|error| vec![format!("cannot re-read verified artifact bytes: {error}")])?;
        if bytes.len() as u64 != artifact.byte_length
            || sha256_hex(&bytes) != artifact.sha256
        {
            return Err(vec![format!(
                "artifact {artifact_uri} changed after bundle verification"
            )]);
        }
        Ok(bytes)
    }
}

fn valid_id(value: &str) -> bool {
    !value.trim().is_empty()
}

fn is_lower_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

pub fn canonical_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>, String> {
    let json_value = serde_json::to_value(value)
        .map_err(|error| format!("JSON serialization failed: {error}"))?;
    serde_json::to_vec(&json_value)
        .map_err(|error| format!("canonical JSON serialization failed: {error}"))
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub fn canonical_sha256<T: Serialize>(value: &T) -> Result<String, String> {
    Ok(sha256_hex(&canonical_bytes(value)?))
}

fn decode_hex<const N: usize>(value: &str, label: &str) -> Result<[u8; N], String> {
    if value.len() != N * 2 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(format!("{label} must be exactly {} hexadecimal bytes", N));
    }
    let mut output = [0_u8; N];
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        let high = (pair[0] as char)
            .to_digit(16)
            .ok_or_else(|| format!("{label} contains invalid hexadecimal"))?;
        let low = (pair[1] as char)
            .to_digit(16)
            .ok_or_else(|| format!("{label} contains invalid hexadecimal"))?;
        output[index] = ((high << 4) | low) as u8;
    }
    Ok(output)
}

fn validate_trust_root(
    trust_root: &TrustRoot,
    pinned_root_sha256: &str,
    now_unix_ms: u64,
) -> Result<String, Vec<String>> {
    let mut errors = Vec::new();
    if trust_root.schema_version != "0.1.0"
        || !valid_id(&trust_root.root_id)
        || trust_root.valid_from_unix_ms >= trust_root.valid_until_unix_ms
    {
        errors.push("trust-root identity, version or validity interval is invalid".to_string());
    }
    if now_unix_ms < trust_root.valid_from_unix_ms
        || now_unix_ms >= trust_root.valid_until_unix_ms
    {
        errors.push("trust root is not valid at the verification time".to_string());
    }

    let digest = match canonical_sha256(trust_root) {
        Ok(value) => value,
        Err(error) => {
            errors.push(error);
            String::new()
        }
    };
    if !is_lower_hex(pinned_root_sha256, 64) || digest != pinned_root_sha256 {
        errors.push("trust-root digest does not match the caller-pinned trust root".to_string());
    }

    let mut key_ids = BTreeSet::new();
    for key in &trust_root.keys {
        if !valid_id(&key.key_id)
            || !valid_id(&key.producer_id)
            || !is_lower_hex(&key.public_key_hex, 64)
            || key.not_before_unix_ms >= key.not_after_unix_ms
            || key.roles.is_empty()
        {
            errors.push(format!("trusted key {} has invalid identity, key, role or time bounds", key.key_id));
        }
        if !key_ids.insert(key.key_id.as_str()) {
            errors.push(format!("duplicate trust-root key ID {}", key.key_id));
        }
        let mut roles = BTreeSet::new();
        for role in &key.roles {
            if !roles.insert(*role) {
                errors.push(format!("trusted key {} repeats a role", key.key_id));
            }
        }
    }

    if errors.is_empty() {
        Ok(digest)
    } else {
        Err(errors)
    }
}

fn validate_payload_shape(payload: &AttestationPayload) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    if payload.schema_version != "0.1.0"
        || !valid_id(&payload.predicate_type)
        || !valid_id(&payload.issuer_id)
        || !valid_id(&payload.subject_uri)
        || !valid_id(&payload.run_id)
        || !is_lower_hex(&payload.subject_sha256, 64)
        || !is_lower_hex(&payload.case_sha256, 64)
        || !is_lower_hex(&payload.proposal_sha256, 64)
        || !is_lower_hex(&payload.tool_source_commit, 40)
        || !is_lower_hex(&payload.tool_configuration_sha256, 64)
    {
        errors.push("attestation payload has invalid identity or digest fields".to_string());
    }
    if payload.issued_at_unix_ms == 0
        || payload.expires_at_unix_ms <= payload.issued_at_unix_ms
    {
        errors.push("attestation issue/expiry interval is invalid".to_string());
    }
    let mut parents = BTreeSet::new();
    for parent in &payload.parent_artifact_sha256 {
        if !is_lower_hex(parent, 64) || !parents.insert(parent.as_str()) {
            errors.push("attestation has an invalid or duplicate parent artifact digest".to_string());
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActuationHardwareFeedback {
    Unavailable,
    NoMovementObserved,
    MovementObserved,
    Conflicting,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ActuationCommandBinding {
    pub run_id: String,
    pub case_sha256: String,
    pub decision_step: u32,
    pub proposal_sha256: String,
    pub action_id: String,
    pub action_kind: String,
    pub command_id: String,
    pub command_sha256: String,
    pub challenge_nonce: String,
    pub controller_id: String,
    pub device_id: String,
    pub witness_issuer_id: String,
    pub witness_tool_source_commit: String,
    pub witness_tool_configuration_sha256: String,
    pub minimum_monotonic_sequence: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NoActuationWitnessReceipt {
    pub schema_version: String,
    pub witness_id: String,
    pub issuer_id: String,
    pub controller_id: String,
    pub device_id: String,
    pub run_id: String,
    pub case_sha256: String,
    pub decision_step: u32,
    pub proposal_sha256: String,
    pub action_id: String,
    pub action_kind: String,
    pub command_id: String,
    pub command_sha256: String,
    pub challenge_nonce: String,
    pub monotonic_sequence: u64,
    pub rejection_boundary: String,
    pub actuation_acquired: bool,
    pub queue_dispatched: bool,
    pub driver_acquired: bool,
    pub output_transition_observed: bool,
    pub hardware_feedback: ActuationHardwareFeedback,
    pub observed_at_unix_ms: u64,
    pub expires_at_unix_ms: u64,
    pub tool_source_commit: String,
    pub tool_configuration_sha256: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ActuationWitnessReplayState {
    pub used_challenge_nonces: BTreeSet<String>,
    pub last_sequence_by_controller_device: BTreeMap<String, u64>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VerifiedNoActuationWitness {
    pub witness_id: String,
    pub artifact_uri: String,
    pub artifact_sha256: String,
    pub command_id: String,
    pub command_sha256: String,
    pub challenge_nonce: String,
    pub monotonic_sequence: u64,
    pub verified_attestation: VerifiedAttestation,
    pub result_status: String,
    pub claim_boundary: String,
}

pub fn verify_no_actuation_witness(
    verified_bundle: &VerifiedBundle,
    receipt: &NoActuationWitnessReceipt,
    expected: &ActuationCommandBinding,
    envelope: &AttestationEnvelope,
    trust_root: &TrustRoot,
    pinned_root_sha256: &str,
    now_unix_ms: u64,
    max_witness_age_ms: u64,
    replay_state: &mut ActuationWitnessReplayState,
) -> Result<VerifiedNoActuationWitness, Vec<String>> {
    let mut errors = Vec::new();

    let artifact_uri = format!(
        "power-fabric://actuation-witness/{}",
        expected.command_sha256
    );
    let artifact_sha256 = canonical_sha256(receipt)
        .map_err(|error| vec![format!("actuation witness canonicalization failed: {error}")])?;
    let attestation_uri = artifact_envelope_uri(envelope)
        .map_err(|error| vec![format!("attestation canonicalization failed: {error}")])?;
    let attestation_sha256 = canonical_sha256(envelope)
        .map_err(|error| vec![format!("attestation canonicalization failed: {error}")])?;
    for (uri, digest) in [
        (artifact_uri.as_str(), artifact_sha256.as_str()),
        (attestation_uri.as_str(), attestation_sha256.as_str()),
    ] {
        let verified = verified_bundle.report().verified_artifacts.iter().any(|artifact| {
            artifact.artifact_uri == uri && artifact.sha256 == digest
        });
        if !verified {
            errors.push(format!(
                "actuation witness artifact {uri} is not present with exact bytes in the verified bundle"
            ));
        }
    }

    if receipt.schema_version != "0.1.0"
        || !valid_id(&receipt.witness_id)
        || receipt.issuer_id != expected.witness_issuer_id
        || receipt.controller_id != expected.controller_id
        || receipt.device_id != expected.device_id
        || receipt.run_id != expected.run_id
        || receipt.case_sha256 != expected.case_sha256
        || receipt.decision_step != expected.decision_step
        || receipt.proposal_sha256 != expected.proposal_sha256
        || receipt.action_id != expected.action_id
        || receipt.action_kind != expected.action_kind
        || receipt.command_id != expected.command_id
        || receipt.command_sha256 != expected.command_sha256
        || receipt.challenge_nonce != expected.challenge_nonce
        || receipt.rejection_boundary != "before_actuation"
        || receipt.actuation_acquired
        || receipt.queue_dispatched
        || receipt.driver_acquired
        || receipt.output_transition_observed
        || matches!(
            receipt.hardware_feedback,
            ActuationHardwareFeedback::MovementObserved
                | ActuationHardwareFeedback::Conflicting
        )
        || !is_lower_hex(&receipt.case_sha256, 64)
        || !is_lower_hex(&receipt.proposal_sha256, 64)
        || !is_lower_hex(&receipt.command_sha256, 64)
        || !is_lower_hex(&receipt.challenge_nonce, 64)
        || !is_lower_hex(&receipt.tool_source_commit, 40)
        || !is_lower_hex(&receipt.tool_configuration_sha256, 64)
        || receipt.tool_source_commit != expected.witness_tool_source_commit
        || receipt.tool_configuration_sha256 != expected.witness_tool_configuration_sha256
    {
        errors.push(
            "actuation witness does not prove a before-actuation, not-acquired outcome for the exact command context"
                .to_string(),
        );
    }

    if receipt.observed_at_unix_ms > now_unix_ms
        || receipt.expires_at_unix_ms <= receipt.observed_at_unix_ms
        || receipt.expires_at_unix_ms < now_unix_ms
        || now_unix_ms.saturating_sub(receipt.observed_at_unix_ms) > max_witness_age_ms
    {
        errors.push("actuation witness is future-dated, expired or too old".to_string());
    }

    let controller_device_key = format!("{}/{}", receipt.controller_id, receipt.device_id);
    let last_sequence = replay_state
        .last_sequence_by_controller_device
        .get(&controller_device_key)
        .copied()
        .unwrap_or(0);
    if receipt.monotonic_sequence <= last_sequence
        || receipt.monotonic_sequence <= expected.minimum_monotonic_sequence
    {
        errors.push("actuation witness monotonic sequence is replayed or stale".to_string());
    }
    if replay_state
        .used_challenge_nonces
        .contains(&receipt.challenge_nonce)
    {
        errors.push("actuation witness challenge nonce has already been consumed".to_string());
    }

    if envelope.payload.issued_at_unix_ms != receipt.observed_at_unix_ms
        || envelope.payload.expires_at_unix_ms != receipt.expires_at_unix_ms
    {
        errors.push("actuation witness receipt time window differs from its signed envelope".to_string());
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    let mut parents = vec![
        expected.command_sha256.clone(),
        expected.proposal_sha256.clone(),
    ];
    parents.sort();
    let expected_attestation = ExpectedAttestationBinding {
        predicate_type: "power-restoration/no-actuation-witness/v1".to_string(),
        issuer_id: expected.witness_issuer_id.clone(),
        artifact_uri: artifact_uri.clone(),
        artifact_sha256: artifact_sha256.clone(),
        run_id: expected.run_id.clone(),
        case_sha256: expected.case_sha256.clone(),
        decision_step: expected.decision_step,
        proposal_sha256: expected.proposal_sha256.clone(),
        signer_role: SignerRole::ActuationWitness,
        tool_source_commit: expected.witness_tool_source_commit.clone(),
        tool_configuration_sha256: expected.witness_tool_configuration_sha256.clone(),
        parent_artifact_sha256: parents,
    };

    let verified_attestation = verify_attestation(
        envelope,
        trust_root,
        pinned_root_sha256,
        now_unix_ms,
        &expected_attestation,
    )?;

    // Consume nonce and advance device sequence only after complete cryptographic
    // and semantic verification. Persistence/transactionality is the caller's
    // responsibility; this in-memory state is a replay guard, not durable storage.
    replay_state
        .used_challenge_nonces
        .insert(receipt.challenge_nonce.clone());
    replay_state
        .last_sequence_by_controller_device
        .insert(controller_device_key, receipt.monotonic_sequence);

    Ok(VerifiedNoActuationWitness {
        witness_id: receipt.witness_id.clone(),
        artifact_uri,
        artifact_sha256,
        command_id: receipt.command_id.clone(),
        command_sha256: receipt.command_sha256.clone(),
        challenge_nonce: receipt.challenge_nonce.clone(),
        monotonic_sequence: receipt.monotonic_sequence,
        verified_attestation,
        result_status: "no_actuation_witness_signature_and_command_binding_verified".to_string(),
        claim_boundary: "A distinct trusted actuation-witness key signed a fresh, one-time, command-bound statement that actuation was not acquired or dispatched. This does not prove physical equipment position, electrical safety or all possible alternate actuation paths; reconcile with independent device feedback and domain validators.".to_string(),
    })
}

pub fn verify_attestation(
    envelope: &AttestationEnvelope,
    trust_root: &TrustRoot,
    pinned_root_sha256: &str,
    now_unix_ms: u64,
    expected: &ExpectedAttestationBinding,
) -> Result<VerifiedAttestation, Vec<String>> {
    let mut errors = Vec::new();
    let root_digest = match validate_trust_root(trust_root, pinned_root_sha256, now_unix_ms) {
        Ok(value) => value,
        Err(root_errors) => {
            errors.extend(root_errors);
            String::new()
        }
    };
    if let Err(payload_errors) = validate_payload_shape(&envelope.payload) {
        errors.extend(payload_errors);
    }
    if !valid_id(&envelope.key_id) {
        errors.push("attestation key ID is empty".to_string());
    }
    if !is_lower_hex(&envelope.signature_hex, 128) {
        errors.push("attestation signature must be 128 lowercase hexadecimal characters".to_string());
    }

    let payload = &envelope.payload;
    if payload.predicate_type != expected.predicate_type
        || payload.issuer_id != expected.issuer_id
        || payload.subject_uri != expected.artifact_uri
        || payload.subject_sha256 != expected.artifact_sha256
        || payload.run_id != expected.run_id
        || payload.case_sha256 != expected.case_sha256
        || payload.decision_step != expected.decision_step
        || payload.proposal_sha256 != expected.proposal_sha256
        || payload.signer_role != expected.signer_role
        || payload.tool_source_commit != expected.tool_source_commit
        || payload.tool_configuration_sha256 != expected.tool_configuration_sha256
        || payload.parent_artifact_sha256 != expected.parent_artifact_sha256
    {
        errors.push("attestation subject/run/case/epoch/proposal/tool/parent binding does not match the expected run context".to_string());
    }

    if payload.issued_at_unix_ms > now_unix_ms
        || payload.expires_at_unix_ms <= now_unix_ms
    {
        errors.push("attestation is not currently valid (future-issued or expired)".to_string());
    }

    let matching_keys: Vec<&TrustedKey> = trust_root
        .keys
        .iter()
        .filter(|key| key.key_id == envelope.key_id)
        .collect();
    let trusted_key = match matching_keys.as_slice() {
        [key] => Some(*key),
        [] => {
            errors.push(format!("attestation key {} is not in the pinned trust root", envelope.key_id));
            None
        }
        _ => {
            errors.push(format!("trust root has ambiguous key ID {}", envelope.key_id));
            None
        }
    };

    if let Some(key) = trusted_key {
        if key.producer_id != payload.issuer_id || !key.roles.contains(&payload.signer_role) {
            errors.push("attestation issuer or signer role is not authorized by the trust root".to_string());
        }
        if now_unix_ms < key.not_before_unix_ms || now_unix_ms >= key.not_after_unix_ms {
            errors.push("attestation signing key is outside its validity interval".to_string());
        }
        if payload.issued_at_unix_ms < key.not_before_unix_ms
            || payload.expires_at_unix_ms > key.not_after_unix_ms
            || payload.expires_at_unix_ms > trust_root.valid_until_unix_ms
        {
            errors.push("attestation validity interval exceeds the trusted key or root validity window".to_string());
        }
        if key.revoked_at_unix_ms.is_some_and(|revoked| revoked <= now_unix_ms) {
            errors.push("attestation signing key is revoked at the verification time".to_string());
        }

        match decode_hex::<32>(&key.public_key_hex, "Ed25519 public key") {
            Ok(public_key_bytes) => match VerifyingKey::from_bytes(&public_key_bytes) {
                Ok(verifying_key) => match decode_hex::<64>(&envelope.signature_hex, "Ed25519 signature") {
                    Ok(signature_bytes) => {
                        let signature = Signature::from_bytes(&signature_bytes);
                        match canonical_bytes(payload) {
                            Ok(message) => {
                                if verifying_key.verify_strict(&message, &signature).is_err() {
                                    errors.push("Ed25519 signature verification failed".to_string());
                                }
                            }
                            Err(error) => errors.push(error),
                        }
                    }
                    Err(error) => errors.push(error),
                },
                Err(_) => errors.push("trusted Ed25519 public key encoding is invalid".to_string()),
            },
            Err(error) => errors.push(error),
        }
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    Ok(VerifiedAttestation {
        key_id: envelope.key_id.clone(),
        issuer_id: payload.issuer_id.clone(),
        signer_role: payload.signer_role,
        subject_uri: payload.subject_uri.clone(),
        subject_sha256: payload.subject_sha256.clone(),
        trust_root_sha256: root_digest,
        signature_status: "strict_ed25519_verified_against_pinned_trust_root".to_string(),
    })
}

fn validate_relative_path(relative_path: &str) -> Result<PathBuf, String> {
    if !valid_id(relative_path)
        || relative_path.contains('\\')
        || relative_path.contains('\0')
        || relative_path.contains(':')
    {
        return Err("artifact path is empty or uses a forbidden path character".to_string());
    }
    let path = Path::new(relative_path);
    if path.is_absolute() {
        return Err("artifact path must be relative to the bundle root".to_string());
    }
    let components: Vec<Component<'_>> = path.components().collect();
    if components.is_empty()
        || components.iter().any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err("artifact path may contain only normal relative path components".to_string());
    }
    if path.to_string_lossy() != relative_path {
        return Err("artifact path must use a normalized relative representation".to_string());
    }
    Ok(path.to_path_buf())
}

pub fn validate_role_separation(
    envelopes: &[AttestationEnvelope],
    trust_root: &TrustRoot,
) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    let mut role_key_fingerprints: BTreeMap<SignerRole, BTreeSet<String>> = BTreeMap::new();
    let mut fingerprint_issuers = BTreeMap::<String, String>::new();

    for envelope in envelopes {
        let matching_keys: Vec<&TrustedKey> = trust_root
            .keys
            .iter()
            .filter(|key| key.key_id == envelope.key_id)
            .collect();
        let key = match matching_keys.as_slice() {
            [key] => *key,
            [] => {
                errors.push(format!("attestation key {} is not trusted", envelope.key_id));
                continue;
            }
            _ => {
                errors.push(format!("trust root has ambiguous key ID {}", envelope.key_id));
                continue;
            }
        };
        if !key.roles.contains(&envelope.payload.signer_role) {
            errors.push(format!(
                "key {} is not authorized for {:?}",
                envelope.key_id, envelope.payload.signer_role
            ));
            continue;
        }
        let public_key = match decode_hex::<32>(&key.public_key_hex, "Ed25519 public key") {
            Ok(value) => value,
            Err(error) => {
                errors.push(error);
                continue;
            }
        };
        let fingerprint = sha256_hex(&public_key);
        if let Some(existing_issuer) = fingerprint_issuers.insert(
            fingerprint.clone(),
            envelope.payload.issuer_id.clone(),
        ) {
            if existing_issuer != envelope.payload.issuer_id {
                errors.push(format!(
                    "one public key fingerprint cannot authenticate multiple producer identities ({existing_issuer} and {})",
                    envelope.payload.issuer_id
                ));
            }
        }
        role_key_fingerprints
            .entry(envelope.payload.signer_role)
            .or_default()
            .insert(fingerprint);
    }

    // Every distinct assurance role requires a distinct cryptographic key identity.
    // Aliased key IDs cannot bypass this because fingerprints, not labels, are compared.
    let roles: Vec<SignerRole> = role_key_fingerprints.keys().copied().collect();
    for left_index in 0..roles.len() {
        for right_index in (left_index + 1)..roles.len() {
            let left = roles[left_index];
            let right = roles[right_index];
            let left_keys = role_key_fingerprints.get(&left).expect("role key set exists");
            let right_keys = role_key_fingerprints.get(&right).expect("role key set exists");
            for fingerprint in left_keys.intersection(right_keys) {
                errors.push(format!(
                    "public key fingerprint {fingerprint} cannot satisfy both {left:?} and {right:?} roles"
                ));
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub fn verify_artifact_bytes(
    bundle_root: &Path,
    artifact: &ArtifactRecord,
    max_artifact_bytes: u64,
) -> Result<VerifiedArtifact, Vec<String>> {
    let mut errors = Vec::new();
    if !valid_id(&artifact.artifact_uri)
        || !valid_id(&artifact.media_type)
        || !is_lower_hex(&artifact.sha256, 64)
        || artifact.byte_length == 0
        || artifact.byte_length > max_artifact_bytes
    {
        errors.push("artifact manifest entry has invalid identity, digest, length or size".to_string());
    }
    let relative_path = match validate_relative_path(&artifact.relative_path) {
        Ok(value) => value,
        Err(error) => {
            errors.push(error);
            PathBuf::new()
        }
    };
    if !errors.is_empty() {
        return Err(errors);
    }

    let root_metadata = fs::symlink_metadata(bundle_root)
        .map_err(|error| vec![format!("cannot inspect bundle root: {error}")])?;
    if root_metadata.file_type().is_symlink() || !root_metadata.is_dir() {
        return Err(vec!["bundle root must be a real directory, not a symlink".to_string()]);
    }
    let canonical_root = fs::canonicalize(bundle_root)
        .map_err(|error| vec![format!("cannot canonicalize bundle root: {error}")])?;

    let mut current = canonical_root.clone();
    let components: Vec<_> = relative_path.components().collect();
    for (index, component) in components.iter().enumerate() {
        current.push(component.as_os_str());
        let metadata = fs::symlink_metadata(&current)
            .map_err(|error| vec![format!("artifact path component is missing: {error}")])?;
        if metadata.file_type().is_symlink() {
            return Err(vec!["artifact paths may not traverse symlinks".to_string()]);
        }
        if index + 1 < components.len() && !metadata.is_dir() {
            return Err(vec!["non-final artifact path component is not a directory".to_string()]);
        }
        if index + 1 == components.len() && !metadata.is_file() {
            return Err(vec!["artifact target must be a regular file".to_string()]);
        }
    }

    let canonical_target = fs::canonicalize(&current)
        .map_err(|error| vec![format!("cannot canonicalize artifact path: {error}")])?;
    if !canonical_target.starts_with(&canonical_root) {
        return Err(vec!["artifact path escapes the bundle root".to_string()]);
    }

    let bytes = fs::read(&canonical_target)
        .map_err(|error| vec![format!("cannot read artifact bytes: {error}")])?;
    if bytes.len() as u64 != artifact.byte_length {
        errors.push("artifact byte length does not match the manifest".to_string());
    }
    let actual_sha256 = sha256_hex(&bytes);
    if actual_sha256 != artifact.sha256 {
        errors.push("artifact bytes do not match the manifest SHA-256".to_string());
    }
    if !errors.is_empty() {
        return Err(errors);
    }

    Ok(VerifiedArtifact {
        artifact_uri: artifact.artifact_uri.clone(),
        relative_path: artifact.relative_path.clone(),
        byte_length: artifact.byte_length,
        sha256: actual_sha256,
    })
}

/// Verifies a caller-declared, complete set of attestation obligations against
/// the bytes already resolved from the bundle manifest. The caller derives
/// obligations from the run receipt and chooses which roles/artifacts are
/// mandatory for the specific claim being evaluated.
pub fn verify_attestation_set(
    verified_bundle: &VerifiedBundle,
    trust_root: &TrustRoot,
    pinned_root_sha256: &str,
    now_unix_ms: u64,
    requirements: &[ExpectedAttestationBinding],
    envelopes: &[AttestationEnvelope],
) -> Result<Vec<VerifiedAttestation>, Vec<String>> {
    let mut errors = Vec::new();
    let manifest_report = verified_bundle.report();
    if requirements.is_empty() || requirements.len() != envelopes.len() {
        errors.push("attestation count must exactly match the non-empty caller-declared requirement set".to_string());
    }

    let mut requirement_keys = BTreeSet::new();
    for requirement in requirements {
        if !requirement_keys.insert((
            requirement.artifact_uri.as_str(),
            requirement.signer_role,
        )) {
            errors.push(format!(
                "duplicate attestation requirement for {} and {:?}",
                requirement.artifact_uri, requirement.signer_role
            ));
        }
        let Some(artifact) = manifest_report
            .verified_artifacts
            .iter()
            .find(|artifact| artifact.artifact_uri == requirement.artifact_uri)
        else {
            errors.push(format!(
                "required attestation subject {} is absent from the verified bundle",
                requirement.artifact_uri
            ));
            continue;
        };
        if artifact.sha256 != requirement.artifact_sha256 {
            errors.push(format!(
                "required subject digest for {} differs from the verified bundle bytes",
                requirement.artifact_uri
            ));
        }
    }

    let mut envelope_keys = BTreeSet::new();
    for envelope in envelopes {
        let key = (
            envelope.payload.subject_uri.as_str(),
            envelope.payload.signer_role,
        );
        if !envelope_keys.insert(key) {
            errors.push(format!(
                "duplicate attestation envelope for {} and {:?}",
                envelope.payload.subject_uri, envelope.payload.signer_role
            ));
        }
        if !requirement_keys.contains(&key) {
            errors.push(format!(
                "unexpected attestation envelope for {} and {:?}",
                envelope.payload.subject_uri, envelope.payload.signer_role
            ));
        }
    }

    if let Err(role_errors) = validate_role_separation(envelopes, trust_root) {
        errors.extend(role_errors);
    }

    let mut verified = Vec::new();
    for requirement in requirements {
        let matching: Vec<&AttestationEnvelope> = envelopes
            .iter()
            .filter(|envelope| {
                envelope.payload.subject_uri == requirement.artifact_uri
                    && envelope.payload.signer_role == requirement.signer_role
            })
            .collect();
        match matching.as_slice() {
            [envelope] => match verify_attestation(
                envelope,
                trust_root,
                pinned_root_sha256,
                now_unix_ms,
                requirement,
            ) {
                Ok(report) => verified.push(report),
                Err(attestation_errors) => errors.extend(attestation_errors),
            },
            [] => errors.push(format!(
                "missing required attestation for {} and {:?}",
                requirement.artifact_uri, requirement.signer_role
            )),
            _ => errors.push(format!(
                "ambiguous attestations for {} and {:?}",
                requirement.artifact_uri, requirement.signer_role
            )),
        }
    }

    if errors.is_empty() {
        Ok(verified)
    } else {
        Err(errors)
    }
}

pub fn verify_bundle_manifest(
    bundle_root: &Path,
    manifest: &BundleManifest,
    max_artifact_bytes: u64,
) -> Result<VerifiedBundle, Vec<String>> {
    let mut errors = Vec::new();
    if manifest.schema_version != "0.1.0" || !valid_id(&manifest.bundle_id) {
        errors.push("bundle manifest version or identity is invalid".to_string());
    }
    let manifest_sha256 = match canonical_sha256(manifest) {
        Ok(value) => value,
        Err(error) => {
            errors.push(error);
            String::new()
        }
    };

    let mut uris = BTreeSet::new();
    let mut paths = BTreeSet::new();
    for artifact in &manifest.artifacts {
        if !uris.insert(artifact.artifact_uri.as_str()) {
            errors.push(format!("duplicate artifact URI {}", artifact.artifact_uri));
        }
        if !paths.insert(artifact.relative_path.as_str()) {
            errors.push(format!("duplicate artifact path {}", artifact.relative_path));
        }
    }
    if manifest.artifacts.is_empty() {
        errors.push("bundle manifest contains no artifacts".to_string());
    }
    if !errors.is_empty() {
        return Err(errors);
    }

    let mut verified_artifacts = Vec::new();
    for artifact in &manifest.artifacts {
        match verify_artifact_bytes(bundle_root, artifact, max_artifact_bytes) {
            Ok(verified) => verified_artifacts.push(verified),
            Err(artifact_errors) => errors.extend(
                artifact_errors
                    .into_iter()
                    .map(|error| format!("{}: {error}", artifact.artifact_uri)),
            ),
        }
    }
    if !errors.is_empty() {
        return Err(errors);
    }

    Ok(VerifiedBundle {
        report: BundleVerificationReport {
            bundle_id: manifest.bundle_id.clone(),
            manifest_sha256,
            artifact_count: verified_artifacts.len(),
            verified_artifacts,
            result_status: "all_manifest_artifact_bytes_verified".to_string(),
            claim_boundary: "Bundle content identity and path containment only; signatures, issuer trust, physical truth and restoration success require separate verification".to_string(),
        },
    })
}


// -----------------------------------------------------------------------------
// End-to-end restoration claim evidence gate
// -----------------------------------------------------------------------------

use power_restoration_action_proposal::{ActionKind, ActionProposal};
use power_restoration_benchmark_case::{
    BenchmarkCase, ElectricalDataProfile, canonical_case_sha256,
};
use power_restoration_observation_evidence::{
    EvidenceArtifact, EvidenceBatch, EvidenceQuality, SourceProvenance, validate_evidence_batch,
};
use power_restoration_run_receipt::{
    RunReceipt, RunStatus, ServiceOutcome, StepDisposition, TerminalReason,
    TransitionOutcome, canonical_sha256 as receipt_canonical_sha256,
    qualify as qualify_run_receipt,
};
use power_restoration_service_trajectory_audit::{
    ServiceTrajectoryAuditReport, ServiceTrajectoryRecord, audit_service_trajectory,
};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ClaimProducerProfile {
    pub signer_role: SignerRole,
    pub issuer_id: String,
    pub tool_source_commit: String,
    pub tool_configuration_sha256: String,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RestorationClaimVerificationReport {
    pub run_id: String,
    pub run_receipt_sha256: String,
    pub case_sha256: String,
    pub run_status: String,
    pub case_maturity: String,
    pub electrical_data_profile: String,
    pub proposal_count: usize,
    pub observation_batch_count: usize,
    pub observation_request_count: usize,
    pub unsuccessful_observation_request_count: usize,
    pub evidence_item_count: usize,
    pub stale_observation_count: usize,
    pub conflicting_observation_count: usize,
    pub unverified_observation_count: usize,
    pub service_trajectory_audit: Option<ServiceTrajectoryAuditReport>,
    pub required_artifact_count: usize,
    pub verified_artifact_count: usize,
    pub required_attestation_count: usize,
    pub verified_attestation_count: usize,
    pub claim_status: String,
    pub claim_boundary: String,
}

fn is_information_request(kind: ActionKind) -> bool {
    matches!(
        kind,
        ActionKind::ObserveScada
            | ActionKind::ObserveCalls
            | ActionKind::ObserveNetworkModel
            | ActionKind::CrewInspect
            | ActionKind::ReadFaultIndicator
            | ActionKind::TestSwitch
            | ActionKind::SimulateSwitch
            | ActionKind::ForecastLoad
            | ActionKind::EscalateToHuman
    )
}

fn proposal_set_sha256(receipt: &RunReceipt) -> Result<String, String> {
    let mut proposals = Vec::new();
    for step in &receipt.decision_steps {
        if let Some(proposal) = &step.proposal {
            proposals.push((
                step.decision_step,
                canonical_sha256(proposal)?,
            ));
        }
    }
    canonical_sha256(&proposals)
}

fn profile_for_role<'a>(
    profiles: &'a [ClaimProducerProfile],
    role: SignerRole,
) -> Result<&'a ClaimProducerProfile, String> {
    let matching: Vec<&ClaimProducerProfile> = profiles
        .iter()
        .filter(|profile| profile.signer_role == role)
        .collect();
    match matching.as_slice() {
        [profile] => Ok(*profile),
        [] => Err(format!("no pinned producer profile for role {role:?}")),
        _ => Err(format!("multiple pinned producer profiles for singular role {role:?}")),
    }
}

fn profile_for<'a>(
    profiles: &'a [ClaimProducerProfile],
    role: SignerRole,
    issuer_id: &str,
) -> Result<&'a ClaimProducerProfile, String> {
    let matching: Vec<&ClaimProducerProfile> = profiles
        .iter()
        .filter(|profile| profile.signer_role == role && profile.issuer_id == issuer_id)
        .collect();
    match matching.as_slice() {
        [profile] => Ok(*profile),
        [] => Err(format!(
            "no pinned producer profile for role {role:?} and issuer {issuer_id}"
        )),
        _ => Err(format!(
            "duplicate pinned producer profiles for role {role:?} and issuer {issuer_id}"
        )),
    }
}

fn canonical_parent_digests(mut parents: Vec<String>) -> Vec<String> {
    parents.sort();
    parents.dedup();
    parents
}

fn add_expected_artifact(
    artifacts: &mut BTreeMap<String, String>,
    artifact_uri: &str,
    digest: &str,
    errors: &mut Vec<String>,
) {
    if !valid_id(artifact_uri) || !is_lower_hex(digest, 64) {
        errors.push(format!(
            "required artifact {artifact_uri} has invalid URI or digest"
        ));
        return;
    }
    if let Some(previous) = artifacts.insert(artifact_uri.to_string(), digest.to_string()) {
        if previous != digest {
            errors.push(format!(
                "artifact URI {artifact_uri} is reused with different content digests"
            ));
        }
    }
}

fn add_attested_artifact(
    artifacts: &mut BTreeMap<String, String>,
    requirements: &mut Vec<ExpectedAttestationBinding>,
    artifact_uri: &str,
    artifact_sha256: &str,
    role: SignerRole,
    issuer_id: &str,
    predicate_type: &str,
    run_id: &str,
    case_sha256: &str,
    decision_step: u32,
    proposal_sha256: &str,
    tool_source_commit: &str,
    tool_configuration_sha256: &str,
    parents: Vec<String>,
    errors: &mut Vec<String>,
) {
    add_expected_artifact(artifacts, artifact_uri, artifact_sha256, errors);
    if !is_lower_hex(tool_source_commit, 40)
        || !is_lower_hex(tool_configuration_sha256, 64)
        || !valid_id(issuer_id)
        || !valid_id(predicate_type)
    {
        errors.push(format!(
            "attestation producer identity for {artifact_uri} is invalid"
        ));
        return;
    }

    let requirement = ExpectedAttestationBinding {
        predicate_type: predicate_type.to_string(),
        issuer_id: issuer_id.to_string(),
        artifact_uri: artifact_uri.to_string(),
        artifact_sha256: artifact_sha256.to_string(),
        run_id: run_id.to_string(),
        case_sha256: case_sha256.to_string(),
        decision_step,
        proposal_sha256: proposal_sha256.to_string(),
        signer_role: role,
        tool_source_commit: tool_source_commit.to_string(),
        tool_configuration_sha256: tool_configuration_sha256.to_string(),
        parent_artifact_sha256: parents,
    };

    if let Some(existing) = requirements.iter().find(|existing| {
        existing.artifact_uri == requirement.artifact_uri
            && existing.signer_role == requirement.signer_role
    }) {
        if existing != &requirement {
            errors.push(format!(
                "artifact {} has conflicting attestation contexts for role {:?}",
                artifact_uri, role
            ));
        }
        return;
    }
    requirements.push(requirement);
}

fn artifact_envelope_uri(envelope: &AttestationEnvelope) -> Result<String, String> {
    let digest = canonical_sha256(envelope)?;
    Ok(format!("power-fabric://attestation/{digest}"))
}

fn add_component_attestation(
    artifacts: &mut BTreeMap<String, String>,
    requirements: &mut Vec<ExpectedAttestationBinding>,
    artifact_uri: &str,
    artifact_sha256: &str,
    role: SignerRole,
    component: &power_restoration_run_receipt::ComponentIdentity,
    predicate_type: &str,
    run_id: &str,
    case_sha256: &str,
    decision_step: u32,
    proposal_sha256: &str,
    parents: Vec<String>,
    errors: &mut Vec<String>,
) {
    add_attested_artifact(
        artifacts,
        requirements,
        artifact_uri,
        artifact_sha256,
        role,
        &component.component_id,
        predicate_type,
        run_id,
        case_sha256,
        decision_step,
        proposal_sha256,
        &component.source_commit,
        &component.configuration_sha256,
        parents,
        errors,
    );
}

/// Verifies a complete claim package without accepting a caller-provided
/// subset of required artifacts or signatures. Required obligations are
/// derived from the actual case, run receipt, proposals and observation batches.
fn verify_final_service_trajectory_bundle(
    bundle_root: &Path,
    verified_bundle: &VerifiedBundle,
    final_trajectory_ref: &power_restoration_run_receipt::ArtifactRef,
    receipt: &RunReceipt,
    case: &BenchmarkCase,
    producer_profiles: &[ClaimProducerProfile],
    max_artifact_bytes: u64,
) -> Result<(ServiceTrajectoryRecord, ServiceTrajectoryAuditReport), Vec<String>> {
    let trajectory_bytes = verified_bundle
        .read_verified_artifact_bytes(
            bundle_root,
            &final_trajectory_ref.artifact_ref,
            max_artifact_bytes,
        )
        .map_err(|artifact_errors| {
            artifact_errors
                .into_iter()
                .map(|error| format!("final service trajectory: {error}"))
                .collect::<Vec<_>>()
        })?;
    if sha256_hex(&trajectory_bytes) != final_trajectory_ref.sha256 {
        return Err(vec![
            "final service trajectory bytes do not match the run receipt artifact digest".to_string()
        ]);
    }

    let trajectory: ServiceTrajectoryRecord = serde_json::from_slice(&trajectory_bytes)
        .map_err(|error| vec![format!("final service trajectory JSON decode error: {error}")])?;
    let service_profile = profile_for_role(producer_profiles, SignerRole::ServiceProvider)
        .map_err(|error| vec![format!("service producer profile: {error}")])?;
    if trajectory.producer.producer_id != service_profile.issuer_id
        || trajectory.producer.source_commit != service_profile.tool_source_commit
        || trajectory.producer.configuration_sha256 != service_profile.tool_configuration_sha256
    {
        return Err(vec![
            "service trajectory producer identity/source/configuration does not match the pinned ServiceProvider profile".to_string()
        ]);
    }

    let service_report = audit_service_trajectory(&trajectory, receipt, case)
        .map_err(|service_errors| {
            service_errors
                .into_iter()
                .map(|error| format!("service trajectory: {error}"))
                .collect::<Vec<_>>()
        })?;

    if receipt.run_status == RunStatus::Completed
        && !matches!(
            service_report.result_status.as_str(),
            "modelled_service_and_score_reconciled"
                | "measured_service_and_score_reconciled"
        )
    {
        return Err(vec![format!(
            "completed run final service trajectory is not reconciled delivered service: {}",
            service_report.result_status
        )]);
    }

    let mut missing_sources = Vec::new();
    for source in &trajectory.source_artifacts {
        let source_bytes_verified = verified_bundle
            .report()
            .verified_artifacts
            .iter()
            .any(|artifact| {
                artifact.artifact_uri == source.artifact_ref
                    && artifact.sha256 == source.sha256
            });
        if !source_bytes_verified {
            missing_sources.push(format!(
                "service source artifact {} is missing from the byte-verified bundle or its digest differs",
                source.artifact_ref
            ));
        }
    }
    if !missing_sources.is_empty() {
        return Err(missing_sources);
    }

    Ok((trajectory, service_report))
}

fn add_service_source_attestations(
    trajectory: &ServiceTrajectoryRecord,
    receipt: &RunReceipt,
    case_sha256: &str,
    proposal_set_sha256: &str,
    producer_profiles: &[ClaimProducerProfile],
    expected_artifacts: &mut BTreeMap<String, String>,
    requirements: &mut Vec<ExpectedAttestationBinding>,
    errors: &mut Vec<String>,
) {
    let evidence_provider_id = trajectory
        .producer
        .evidence_provider_id
        .as_deref()
        .unwrap_or_default();
    let evidence_profile = match profile_for(
        producer_profiles,
        SignerRole::EvidenceProvider,
        evidence_provider_id,
    ) {
        Ok(profile) => profile.clone(),
        Err(error) => {
            errors.push(format!("service evidence producer profile: {error}"));
            return;
        }
    };
    let service_profile = match profile_for_role(
        producer_profiles,
        SignerRole::ServiceProvider,
    ) {
        Ok(profile) => profile,
        Err(error) => {
            errors.push(format!("service trajectory producer profile: {error}"));
            return;
        }
    };

    if evidence_profile.issuer_id == service_profile.issuer_id {
        errors.push(
            "service source EvidenceProvider and derived ServiceProvider must be distinct producer identities"
                .to_string(),
        );
        return;
    }

    let parents = canonical_parent_digests(vec![
        case_sha256.to_string(),
        proposal_set_sha256.to_string(),
    ]);
    for source in &trajectory.source_artifacts {
        add_attested_artifact(
            expected_artifacts,
            requirements,
            &source.artifact_ref,
            &source.sha256,
            SignerRole::EvidenceProvider,
            &evidence_profile.issuer_id,
            "power-restoration/service-source-evidence/v1",
            &receipt.run_id,
            case_sha256,
            0,
            proposal_set_sha256,
            &evidence_profile.tool_source_commit,
            &evidence_profile.tool_configuration_sha256,
            parents.clone(),
            errors,
        );
    }
}

pub fn verify_restoration_claim_evidence(
    bundle_root: &Path,
    case: &BenchmarkCase,
    receipt: &RunReceipt,
    observation_batches: &[EvidenceBatch],
    manifest: &BundleManifest,
    trust_root: &TrustRoot,
    pinned_trust_root_sha256: &str,
    attestations: &[AttestationEnvelope],
    producer_profiles: &[ClaimProducerProfile],
    run_receipt_artifact_uri: &str,
    verified_at_unix_ms: u64,
    max_artifact_bytes: u64,
) -> Result<RestorationClaimVerificationReport, Vec<String>> {
    let mut errors = Vec::new();
    let mut service_trajectory_audit: Option<ServiceTrajectoryAuditReport> = None;

    // Producer profiles are supplied out-of-band by the caller, not accepted
    // from the evidence bundle. Only these non-component producer roles are configurable.
    let mut producer_profile_keys = BTreeSet::new();
    for profile in producer_profiles {
        if !matches!(
            profile.signer_role,
            SignerRole::RunRecorder | SignerRole::ServiceProvider | SignerRole::EvidenceProvider
        ) {
            errors.push(format!(
                "producer profile role {:?} must be derived from the typed run record or source provenance",
                profile.signer_role
            ));
        }
        if !valid_id(&profile.issuer_id)
            || !is_lower_hex(&profile.tool_source_commit, 40)
            || !is_lower_hex(&profile.tool_configuration_sha256, 64)
        {
            errors.push(format!(
                "producer profile for {:?}/{} has invalid identity or digest",
                profile.signer_role, profile.issuer_id
            ));
        }
        if !producer_profile_keys.insert((profile.signer_role, profile.issuer_id.as_str())) {
            errors.push(format!(
                "duplicate producer profile for {:?}/{}",
                profile.signer_role, profile.issuer_id
            ));
        }
    }

    let case_sha256 = canonical_case_sha256(case)
        .map_err(|error| vec![format!("case canonicalization failed: {error}")])?;
    qualify_run_receipt(receipt, case)
        .map_err(|receipt_errors| receipt_errors.into_iter().map(|error| format!("run receipt: {error}")).collect::<Vec<_>>())?;
    let receipt_sha256 = receipt_canonical_sha256(receipt)
        .map_err(|error| vec![format!("run receipt canonicalization failed: {error}")])?;
    let proposal_set_sha = proposal_set_sha256(receipt)
        .map_err(|error| vec![format!("proposal-set canonicalization failed: {error}")])?;

    let mut expected_artifacts = BTreeMap::new();
    let mut requirements = Vec::new();

    // Preserve and resolve every imported source artifact used to construct the
    // frozen case. A VCS object ID without a raw byte digest is not enough for
    // this offline package verifier to prove that the imported bytes are present.
    for source_artifact in &case.provenance.source_artifacts {
        let Some(raw_sha256) = source_artifact.raw_sha256.as_deref() else {
            errors.push(format!(
                "case source artifact {} lacks a raw SHA-256 digest",
                source_artifact.path
            ));
            continue;
        };
        let source_path_valid = validate_relative_path(&source_artifact.path).is_ok();
        let vcs_object_id_valid = match source_artifact.vcs_object_format {
            power_restoration_benchmark_case::VcsObjectFormat::GitSha1 => {
                is_lower_hex(&source_artifact.vcs_object_id, 40)
            }
            power_restoration_benchmark_case::VcsObjectFormat::GitSha256 => {
                is_lower_hex(&source_artifact.vcs_object_id, 64)
            }
        };
        if !source_path_valid || !vcs_object_id_valid || !is_lower_hex(raw_sha256, 64) {
            errors.push(format!(
                "case source artifact {} has invalid relative path, VCS object ID for its declared format, or raw digest",
                source_artifact.path
            ));
            continue;
        }
        let path_digest = hash_text(&source_artifact.path);
        let source_uri = format!(
            "power-fabric://case-source/{}/{}",
            source_artifact.vcs_object_id, path_digest
        );
        add_expected_artifact(
            &mut expected_artifacts,
            &source_uri,
            raw_sha256,
            &mut errors,
        );
    }

    // The exact canonical run receipt itself must be present and signed.
    let recorder = match profile_for_role(
        producer_profiles,
        SignerRole::RunRecorder,
    ) {
        Ok(profile) => Some(profile),
        Err(error) => {
            errors.push(error);
            None
        }
    };
    if let Some(recorder) = recorder {
        add_attested_artifact(
            &mut expected_artifacts,
            &mut requirements,
            run_receipt_artifact_uri,
            &receipt_sha256,
            SignerRole::RunRecorder,
            &recorder.issuer_id,
            "power-restoration/run-receipt/v1",
            &receipt.run_id,
            &case_sha256,
            0,
            &proposal_set_sha,
            &recorder.tool_source_commit,
            &recorder.tool_configuration_sha256,
            canonical_parent_digests(vec![case_sha256.clone(), proposal_set_sha.clone()]),
            &mut errors,
        );
    }

    // Each information request is paired to exactly one result batch.
    let mut expected_batch_proposals = BTreeMap::<String, (&ActionProposal, u32)>::new();
    for step in &receipt.decision_steps {
        if step.disposition != StepDisposition::Executed {
            continue;
        }
        if let Some(proposal) = &step.proposal {
            if proposal.actions.iter().any(|action| is_information_request(action.action_kind)) {
                if expected_batch_proposals
                    .insert(proposal.proposal_id.clone(), (proposal, step.decision_step))
                    .is_some()
                {
                    errors.push(format!(
                        "duplicate proposal ID {} across executed decision steps",
                        proposal.proposal_id
                    ));
                }
            }
        }
    }

    let mut actual_batches = BTreeMap::<String, &EvidenceBatch>::new();
    for batch in observation_batches {
        if actual_batches.insert(batch.proposal_id.clone(), batch).is_some() {
            errors.push(format!(
                "duplicate observation result batch for proposal {}",
                batch.proposal_id
            ));
        }
    }
    if actual_batches.len() != expected_batch_proposals.len() {
        errors.push(format!(
            "observation batch coverage mismatch: {} required proposals, {} supplied batches",
            expected_batch_proposals.len(),
            actual_batches.len()
        ));
    }

    let mut observation_request_count = 0;
    let mut unsuccessful_observation_request_count = 0;
    let mut evidence_item_count = 0;
    let mut stale_observation_count = 0;
    let mut conflicting_observation_count = 0;
    let mut unverified_observation_count = 0;
    let mut all_sources = BTreeMap::<String, &SourceProvenance>::new();
    let mut evidence_by_ref = BTreeMap::<String, EvidenceArtifact>::new();
    let mut evidence_presentation_step = BTreeMap::<String, u32>::new();

    for (proposal_id, (proposal, proposal_step)) in &expected_batch_proposals {
        let Some(batch) = actual_batches.get(proposal_id) else {
            errors.push(format!(
                "information-request proposal {proposal_id} has no observation evidence batch"
            ));
            continue;
        };
        if batch.presentation_step < *proposal_step {
            errors.push(format!(
                "observation evidence batch for {proposal_id} predates its information request"
            ));
        }
        match validate_evidence_batch(batch, proposal, case) {
            Ok(report) => {
                observation_request_count += report.request_count;
                unsuccessful_observation_request_count += report.unsuccessful_request_count
                    + report.pending_request_count;
                evidence_item_count += report.evidence_count;
                stale_observation_count += report.stale_at_presentation_count;
                conflicting_observation_count += report.conflicting_evidence_count;
                unverified_observation_count += report.unverified_evidence_count;
                // Degraded observations remain part of the episode and denominator.
                // Admissibility is enforced at point-of-use below: only evidence actually
                // cited as an action precondition must be current, fresh and presented.
            }
            Err(batch_errors) => errors.extend(
                batch_errors
                    .into_iter()
                    .map(|error| format!("observation batch {}: {error}", batch.batch_id)),
            ),
        }
        for source in &batch.sources {
            if let Some(existing) = all_sources.insert(source.source_id.clone(), source) {
                if existing != source {
                    errors.push(format!(
                        "source {} changes provenance across observation batches",
                        source.source_id
                    ));
                }
            }
        }

        // The batch is itself immutable evidence and is tied to the exact proposal.
        let batch_sha = match canonical_sha256(batch) {
            Ok(value) => value,
            Err(error) => {
                errors.push(error);
                continue;
            }
        };
        let batch_uri = format!("power-fabric://observation-batch/{}", batch.batch_id);
        let proposal_sha = match canonical_sha256(proposal) {
            Ok(value) => value,
            Err(batch_error) => {
                errors.push(batch_error);
                continue;
            }
        };
        if let Some(recorder) = recorder {
            add_attested_artifact(
                &mut expected_artifacts,
                &mut requirements,
                &batch_uri,
                &batch_sha,
                SignerRole::RunRecorder,
                &recorder.issuer_id,
                "power-restoration/observation-result-batch/v1",
                &receipt.run_id,
                &case_sha256,
                batch.presentation_step,
                &proposal_sha,
                &recorder.tool_source_commit,
                &recorder.tool_configuration_sha256,
                canonical_parent_digests(vec![proposal_sha.clone()]),
                &mut errors,
            );
        } else {
            errors.push("observation batch has no pinned run-recorder profile".to_string());
        }

        for result in &batch.results {
            for item in &result.evidence {
                let source = match batch.sources.iter().find(|source| source.source_id == item.source_id) {
                    Some(source) => source,
                    None => continue,
                };
                let profile = match profile_for(
                    producer_profiles,
                    SignerRole::EvidenceProvider,
                    &source.producer_id,
                ) {
                    Ok(profile) => profile,
                    Err(error) => {
                        errors.push(error);
                        continue;
                    }
                };
                let evidence_uri = format!("power-fabric://evidence/{}", item.evidence_id);
                for evidence_key in [item.evidence_id.clone(), evidence_uri.clone()] {
                    if let Some(previous) = evidence_by_ref.insert(evidence_key.clone(), item.clone()) {
                        if previous != *item {
                            errors.push(format!(
                                "evidence key {evidence_key} changes content or timing across batches"
                            ));
                        }
                    }
                    if let Some(previous_step) =
                        evidence_presentation_step.insert(evidence_key.clone(), batch.presentation_step)
                    {
                        if previous_step != batch.presentation_step {
                            errors.push(format!(
                                "evidence key {evidence_key} is presented at multiple decision epochs"
                            ));
                        }
                    }
                }
                add_attested_artifact(
                    &mut expected_artifacts,
                    &mut requirements,
                    &evidence_uri,
                    &item.evidence_sha256,
                    SignerRole::EvidenceProvider,
                    &profile.issuer_id,
                    "power-restoration/observation-evidence/v1",
                    &receipt.run_id,
                    &case_sha256,
                    item.available_step,
                    &proposal_sha,
                    &profile.tool_source_commit,
                    &profile.tool_configuration_sha256,
                    vec![source.source_snapshot_sha256.clone()],
                    &mut errors,
                );
            }
        }
    }

    for batch_id in actual_batches.keys() {
        if !expected_batch_proposals.contains_key(batch_id) {
            errors.push(format!(
                "unexpected observation batch for non-requesting proposal {batch_id}"
            ));
        }
    }

    // Every source snapshot has its own provider attestation, and its declared
    // ancestry is preserved as the parent list of that signed snapshot claim.
    for source in all_sources.values() {
        if source.lineage_completeness != power_restoration_observation_evidence::LineageCompleteness::Complete {
            errors.push(format!(
                "source {} has incomplete declared provenance lineage",
                source.source_id
            ));
        }
        let profile = match profile_for(
            producer_profiles,
            SignerRole::EvidenceProvider,
            &source.producer_id,
        ) {
            Ok(profile) => profile,
            Err(error) => {
                errors.push(error);
                continue;
            }
        };
        let source_uri = format!("power-fabric://source/{}/snapshot", source.source_id);
        let matching_source_attestations: Vec<&AttestationEnvelope> = attestations
            .iter()
            .filter(|envelope| {
                envelope.payload.subject_uri == source_uri
                    && envelope.payload.subject_sha256 == source.source_snapshot_sha256
                    && envelope.payload.signer_role == SignerRole::EvidenceProvider
                    && envelope.payload.issuer_id == source.producer_id
            })
            .collect();
        match matching_source_attestations.as_slice() {
            [envelope] => {
                match (artifact_envelope_uri(envelope), canonical_sha256(envelope)) {
                    (Ok(expected_uri), Ok(expected_sha)) => {
                        if source.producer_attestation_ref != expected_uri
                            || source.producer_attestation_sha256 != expected_sha
                        {
                            errors.push(format!(
                                "source {} producer-attestation reference/digest does not bind the supplied signed envelope",
                                source.source_id
                            ));
                        }
                    }
                    (Err(error), _) | (_, Err(error)) => errors.push(error),
                }
            }
            [] => errors.push(format!(
                "source {} has no EvidenceProvider attestation for its snapshot",
                source.source_id
            )),
            _ => errors.push(format!(
                "source {} has multiple EvidenceProvider attestations for one snapshot",
                source.source_id
            )),
        }
        let mut parents = Vec::new();
        for parent_id in &source.parent_source_ids {
            match all_sources.get(parent_id) {
                Some(parent) => parents.push(parent.source_snapshot_sha256.clone()),
                None => errors.push(format!(
                    "source {} parent {} is absent from the claim package",
                    source.source_id, parent_id
                )),
            }
        }
        add_attested_artifact(
            &mut expected_artifacts,
            &mut requirements,
            &source_uri,
            &source.source_snapshot_sha256,
            SignerRole::EvidenceProvider,
            &profile.issuer_id,
            "power-restoration/source-snapshot/v1",
            &receipt.run_id,
            &case_sha256,
            0,
            &proposal_set_sha,
            &profile.tool_source_commit,
            &profile.tool_configuration_sha256,
            canonical_parent_digests(parents),
            &mut errors,
        );
    }

    let validator_role = match case.model_profile.electrical_data_profile {
        ElectricalDataProfile::TopologyOnly => SignerRole::IndependentValidator,
        ElectricalDataProfile::BranchImpedanceAndLimits
        | ElectricalDataProfile::CompleteAcModelInputs => SignerRole::ElectricalValidator,
    };

    let mut service_trajectory_digests = Vec::new();
    for step in receipt.decision_steps.iter().filter(|step| step.disposition == StepDisposition::Executed) {
        let proposal_sha = match &step.proposal {
            Some(proposal) => match canonical_sha256(proposal) {
                Ok(value) => value,
                Err(proposal_error) => {
                    errors.push(proposal_error);
                    proposal_set_sha.clone()
                }
            },
            None => proposal_set_sha.clone(),
        };

        let mut observation_parents = Vec::new();
        if let Some(proposal) = &step.proposal {
            for action in &proposal.actions {
                for precondition in &action.precondition_evidence {
                    let Some(evidence) = evidence_by_ref.get(&precondition.evidence_ref) else {
                        errors.push(format!(
                            "decision {} action {} references evidence {} absent from all acquired observation results",
                            step.decision_step, action.action_id, precondition.evidence_ref
                        ));
                        continue;
                    };
                    if evidence.evidence_sha256 != precondition.evidence_sha256 {
                        errors.push(format!(
                            "decision {} action {} evidence {} digest differs from acquired evidence",
                            step.decision_step, action.action_id, precondition.evidence_ref
                        ));
                    }
                    let result_presentation_step = evidence_presentation_step
                        .get(&precondition.evidence_ref)
                        .copied()
                        .unwrap_or(u32::MAX);
                    if evidence.available_step > step.decision_step
                        || evidence.observed_step > step.decision_step
                        || result_presentation_step > step.decision_step
                    {
                        errors.push(format!(
                            "decision {} action {} cites future or not-yet-presented evidence {}",
                            step.decision_step, action.action_id, precondition.evidence_ref
                        ));
                    } else if step.decision_step.saturating_sub(evidence.observed_step)
                        > evidence.max_age_steps
                    {
                        errors.push(format!(
                            "decision {} action {} cites stale evidence {} at use time",
                            step.decision_step, action.action_id, precondition.evidence_ref
                        ));
                    }
                    if evidence.quality != EvidenceQuality::Current {
                        errors.push(format!(
                            "decision {} action {} cites {:?} evidence {}",
                            step.decision_step, action.action_id, evidence.quality, precondition.evidence_ref
                        ));
                    }
                    observation_parents.push(evidence.evidence_sha256.clone());
                }
            }
        }

        if let Some(proposal) = &step.proposal {
            let proposal_uri = format!("power-fabric://planner-proposal/{}", proposal.proposal_id);
            let mut proposal_parents = observation_parents.clone();
            if let Some(observation) = &step.observation {
                proposal_parents.push(observation.sha256.clone());
            }
            add_attested_artifact(
                &mut expected_artifacts,
                &mut requirements,
                &proposal_uri,
                &proposal_sha,
                SignerRole::Planner,
                &receipt.planner.planner_id,
                "power-restoration/planner-proposal/v1",
                &receipt.run_id,
                &case_sha256,
                step.decision_step,
                &proposal_sha,
                &receipt.planner.source_commit,
                &receipt.planner.configuration_sha256,
                canonical_parent_digests(proposal_parents),
                &mut errors,
            );
        } else if let Some(raw_output) = &step.planner_output {
            let mut raw_parents = Vec::new();
            if let Some(observation) = &step.observation {
                raw_parents.push(observation.sha256.clone());
            }
            add_attested_artifact(
                &mut expected_artifacts,
                &mut requirements,
                &raw_output.artifact_ref,
                &raw_output.sha256,
                SignerRole::Planner,
                &receipt.planner.planner_id,
                "power-restoration/planner-raw-output/v1",
                &receipt.run_id,
                &case_sha256,
                step.decision_step,
                &proposal_set_sha,
                &receipt.planner.source_commit,
                &receipt.planner.configuration_sha256,
                canonical_parent_digests(raw_parents),
                &mut errors,
            );
        }

        if let Some(observation) = &step.observation {
            if let Some(recorder) = recorder {
                add_attested_artifact(
                    &mut expected_artifacts,
                    &mut requirements,
                    &observation.artifact_ref,
                    &observation.sha256,
                    SignerRole::RunRecorder,
                    &recorder.issuer_id,
                    "power-restoration/decision-observation/v1",
                    &receipt.run_id,
                    &case_sha256,
                    step.decision_step,
                    &proposal_sha,
                    &recorder.tool_source_commit,
                    &recorder.tool_configuration_sha256,
                    canonical_parent_digests(observation_parents.clone()),
                    &mut errors,
                );
            }
        }

        if let Some(transition) = &step.transition_receipt {
            if !matches!(
                step.transition_outcome,
                TransitionOutcome::Applied
                    | TransitionOutcome::Failed
                    | TransitionOutcome::Rejected
                    | TransitionOutcome::HumanOverride
            ) {
                errors.push(format!(
                    "decision {} has a transition receipt for an incompatible outcome",
                    step.decision_step
                ));
            }
            let mut parents = Vec::new();
            if let Some(observation) = &step.observation {
                parents.push(observation.sha256.clone());
            }
            parents.push(proposal_sha.clone());
            add_component_attestation(
                &mut expected_artifacts,
                &mut requirements,
                &transition.artifact_ref,
                &transition.sha256,
                SignerRole::TransitionEngine,
                &receipt.simulator,
                "power-restoration/transition/v1",
                &receipt.run_id,
                &case_sha256,
                step.decision_step,
                &proposal_sha,
                canonical_parent_digests(parents),
                &mut errors,
            );
        }

        if let Some(validation) = &step.validator_receipt {
            let mut parents = Vec::new();
            if let Some(observation) = &step.observation {
                parents.push(observation.sha256.clone());
            }
            parents.push(proposal_sha.clone());
            if let Some(transition) = &step.transition_receipt {
                parents.push(transition.sha256.clone());
            }
            add_component_attestation(
                &mut expected_artifacts,
                &mut requirements,
                &validation.artifact_ref,
                &validation.sha256,
                validator_role,
                &receipt.independent_validator,
                "power-restoration/independent-validation/v1",
                &receipt.run_id,
                &case_sha256,
                step.decision_step,
                &proposal_sha,
                canonical_parent_digests(parents),
                &mut errors,
            );
        }

        if step.service_outcome == ServiceOutcome::Scored {
            let Some(service) = &step.service_trajectory else {
                errors.push(format!(
                    "scored decision {} lacks a service trajectory",
                    step.decision_step
                ));
                continue;
            };
            let profile = match profile_for_role(producer_profiles, SignerRole::ServiceProvider) {
                Ok(profile) => profile,
                Err(error) => {
                    errors.push(error);
                    continue;
                }
            };
            let mut parents = Vec::new();
            if let Some(transition) = &step.transition_receipt {
                parents.push(transition.sha256.clone());
            }
            if let Some(validation) = &step.validator_receipt {
                parents.push(validation.sha256.clone());
            }
            add_attested_artifact(
                &mut expected_artifacts,
                &mut requirements,
                &service.artifact_ref,
                &service.sha256,
                SignerRole::ServiceProvider,
                &profile.issuer_id,
                "power-restoration/service-trajectory/v1",
                &receipt.run_id,
                &case_sha256,
                step.decision_step,
                &proposal_sha,
                &profile.tool_source_commit,
                &profile.tool_configuration_sha256,
                canonical_parent_digests(parents),
                &mut errors,
            );
            service_trajectory_digests.push(service.sha256.clone());
        }
    }

    if receipt.run_status == RunStatus::Completed {
        if receipt.decision_steps.iter().any(|step| {
            step.disposition == StepDisposition::Executed
                && step.service_outcome != ServiceOutcome::Scored
        }) {
            errors.push("completed restoration claim has an executed decision without an independently scored service trajectory".to_string());
        }
    }

    if let Some(final_trajectory) = &receipt.final_service_trajectory {
        let already_attested = expected_artifacts
            .get(&final_trajectory.artifact_ref)
            .is_some_and(|digest| digest == &final_trajectory.sha256)
            && requirements.iter().any(|requirement| {
                requirement.artifact_uri == final_trajectory.artifact_ref
                    && requirement.signer_role == SignerRole::ServiceProvider
            });
        if !already_attested {
            match profile_for_role(producer_profiles, SignerRole::ServiceProvider) {
                Ok(profile) => add_attested_artifact(
                    &mut expected_artifacts,
                    &mut requirements,
                    &final_trajectory.artifact_ref,
                    &final_trajectory.sha256,
                    SignerRole::ServiceProvider,
                    &profile.issuer_id,
                    "power-restoration/final-service-trajectory/v1",
                    &receipt.run_id,
                    &case_sha256,
                    case.timebase.horizon_steps.saturating_sub(1),
                    &proposal_set_sha,
                    &profile.tool_source_commit,
                    &profile.tool_configuration_sha256,
                    canonical_parent_digests(service_trajectory_digests),
                    &mut errors,
                ),
                Err(error) => errors.push(error),
            }
        }
    }

    if let Some(score) = &receipt.aggregate_score {
        add_component_attestation(
            &mut expected_artifacts,
            &mut requirements,
            &score.scorer_receipt.artifact_ref,
            &score.scorer_receipt.sha256,
            SignerRole::Scorer,
            &receipt.objective.scorer,
            "power-restoration/aggregate-score/v1",
            &receipt.run_id,
            &case_sha256,
            case.timebase.horizon_steps.saturating_sub(1),
            &proposal_set_sha,
            receipt
                .final_service_trajectory
                .as_ref()
                .map(|trajectory| vec![trajectory.sha256.clone()])
                .unwrap_or_default(),
            &mut errors,
        );
    }

    // Every signature envelope is itself included byte-for-byte in the bundle.
    for envelope in attestations {
        match artifact_envelope_uri(envelope) {
            Ok(uri) => match canonical_bytes(envelope) {
                Ok(bytes) => add_expected_artifact(
                    &mut expected_artifacts,
                    &uri,
                    &sha256_hex(&bytes),
                    &mut errors,
                ),
                Err(error) => errors.push(error),
            },
            Err(error) => errors.push(error),
        }
    }

    // Verify bundle bytes before semantic service parsing. The service parser
    // receives only bytes re-read and re-hashed through this verified-bundle
    // boundary, never caller-provided JSON or a success-shaped metadata report.
    let verified_bundle = verify_bundle_manifest(bundle_root, manifest, max_artifact_bytes)
        .map_err(|bundle_errors| bundle_errors.into_iter().map(|error| format!("bundle: {error}")).collect::<Vec<_>>())?;

    if let Some(final_trajectory_ref) = &receipt.final_service_trajectory {
        match verify_final_service_trajectory_bundle(
            bundle_root,
            &verified_bundle,
            final_trajectory_ref,
            receipt,
            case,
            producer_profiles,
            max_artifact_bytes,
        ) {
            Ok((trajectory, service_report)) => {
                add_service_source_attestations(
                    &trajectory,
                    receipt,
                    &case_sha256,
                    &proposal_set_sha,
                    producer_profiles,
                    &mut expected_artifacts,
                    &mut requirements,
                    &mut errors,
                );
                service_trajectory_audit = Some(service_report);
            }
            Err(service_errors) => errors.extend(service_errors),
        }
    }

    // A complete claim package is closed-world: no required artifact can be
    // omitted and no extra unsigned or unreferenced artifact can be smuggled in.
    let manifest_map: BTreeMap<&str, &str> = manifest
        .artifacts
        .iter()
        .map(|artifact| (artifact.artifact_uri.as_str(), artifact.sha256.as_str()))
        .collect();
    if manifest_map.len() != manifest.artifacts.len() {
        errors.push("bundle manifest contains duplicate artifact URIs".to_string());
    }
    if manifest_map.len() != expected_artifacts.len() {
        errors.push(format!(
            "artifact manifest is not closed-world: {} required artifacts, {} declared",
            expected_artifacts.len(),
            manifest_map.len()
        ));
    }
    for (uri, digest) in &expected_artifacts {
        match manifest_map.get(uri.as_str()) {
            Some(manifest_digest) if *manifest_digest == digest.as_str() => {}
            Some(_) => errors.push(format!(
                "manifest digest for required artifact {uri} does not match its typed record"
            )),
            None => errors.push(format!("required artifact {uri} is absent from the bundle manifest")),
        }
    }
    for uri in manifest_map.keys() {
        if !expected_artifacts.contains_key(*uri) {
            errors.push(format!("unexpected unreferenced artifact {uri} is present in the bundle"));
        }
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    // Every producer envelope must use the deterministic artifact URI above.
    for envelope in attestations {
        let uri = artifact_envelope_uri(envelope)
            .map_err(|error| vec![error])?;
        if !expected_artifacts.contains_key(&uri) {
            errors.push(format!("attestation envelope {uri} is not committed in the verified artifact set"));
        }
    }

    let verified_attestations = verify_attestation_set(
        &verified_bundle,
        trust_root,
        pinned_trust_root_sha256,
        verified_at_unix_ms,
        &requirements,
        attestations,
    )
    .map_err(|attestation_errors| attestation_errors.into_iter().map(|error| format!("attestation: {error}")).collect::<Vec<_>>())?;

    if !errors.is_empty() {
        return Err(errors);
    }

    let successful_terminal = receipt.run_status == RunStatus::Completed
        && receipt.terminal_reason == Some(TerminalReason::RestorationComplete);
    let claim_status = if successful_terminal {
        "signed_evidence_chain_complete_for_declared_scope"
    } else {
        "signed_evidence_chain_verified_but_run_not_complete"
    };

    Ok(RestorationClaimVerificationReport {
        run_id: receipt.run_id.clone(),
        run_receipt_sha256: receipt_sha256,
        case_sha256,
        run_status: format!("{:?}", receipt.run_status),
        case_maturity: format!("{:?}", case.maturity),
        electrical_data_profile: format!("{:?}", case.model_profile.electrical_data_profile),
        proposal_count: receipt.decision_steps.iter().filter(|step| step.proposal.is_some()).count(),
        observation_batch_count: observation_batches.len(),
        observation_request_count,
        unsuccessful_observation_request_count,
        evidence_item_count,
        stale_observation_count,
        conflicting_observation_count,
        unverified_observation_count,
        service_trajectory_audit,
        required_artifact_count: expected_artifacts.len(),
        verified_artifact_count: verified_bundle.report().artifact_count,
        required_attestation_count: requirements.len(),
        verified_attestation_count: verified_attestations.len(),
        claim_status: claim_status.to_string(),
        claim_boundary: "Complete content-addressed and role-separated attestation chain plus byte-verified service trajectory reconciliation for the declared case/run scope only. Cryptographic authenticity and service arithmetic do not prove signed statements are scientifically true, metering is physically accurate, power flow or protection is correct, switching is authorized or safe, or customers actually received the claimed service.".to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_BUNDLE_ID: AtomicU64 = AtomicU64::new(0);

    const TRUST_ROOT_SHA256: &str =
        "973efb60a07f0a0d49d50e0ee40ff42eaf0bc50a406c483fce39325f01c9cbd3";

    fn fixtures() -> (TrustRoot, AttestationEnvelope, ExpectedAttestationBinding, BundleManifest) {
        let trust: TrustRoot = serde_json::from_str(include_str!(
            "../fixtures/synthetic-trust-root-v0.1.json"
        ))
        .expect("trust root fixture must parse");
        let envelope: AttestationEnvelope = serde_json::from_str(include_str!(
            "../fixtures/synthetic-validator-attestation-v0.1.json"
        ))
        .expect("attestation fixture must parse");
        let manifest: BundleManifest = serde_json::from_str(include_str!(
            "../fixtures/synthetic-bundle-manifest-v0.1.json"
        ))
        .expect("bundle manifest fixture must parse");
        let expected = ExpectedAttestationBinding {
            predicate_type: "power-restoration/electrical-validation/v1".to_string(),
            issuer_id: "synthetic-independent-validator".to_string(),
            artifact_uri: "bundle://validator/decision-0.json".to_string(),
            artifact_sha256: "4ac0258529aca07bcc9c5c45cf0c23f05eb8176212b6af732f80c9bd8ad5f987".to_string(),
            run_id: "synthetic-run-0001".to_string(),
            case_sha256: "a".repeat(64),
            decision_step: 0,
            proposal_sha256: "b".repeat(64),
            signer_role: SignerRole::ElectricalValidator,
            tool_source_commit: "1".repeat(40),
            tool_configuration_sha256: "2".repeat(64),
            parent_artifact_sha256: vec!["b".repeat(64)],
        };
        (trust, envelope, expected, manifest)
    }

    fn actuation_witness_fixtures() -> (
        TrustRoot,
        NoActuationWitnessReceipt,
        ActuationCommandBinding,
        AttestationEnvelope,
        String,
    ) {
        use ed25519_dalek::{Signer, SigningKey};

        let signing_key = SigningKey::from_bytes(&[7_u8; 32]);
        let public_key = signing_key.verifying_key().to_bytes();
        let public_key_hex = public_key.iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();

        let issuer_id = "synthetic-utility-actuation-adapter".to_string();
        let tool_source_commit = "e".repeat(40);
        let tool_configuration_sha256 = "f".repeat(64);
        let trust_root = TrustRoot {
            schema_version: "0.1.0".to_string(),
            root_id: "synthetic-actuation-root".to_string(),
            valid_from_unix_ms: 0,
            valid_until_unix_ms: 10_000,
            keys: vec![TrustedKey {
                key_id: "actuation-adapter-key-1".to_string(),
                producer_id: issuer_id.clone(),
                public_key_hex,
                roles: vec![SignerRole::ActuationWitness],
                not_before_unix_ms: 0,
                not_after_unix_ms: 10_000,
                revoked_at_unix_ms: None,
            }],
        };
        let command_binding = ActuationCommandBinding {
            run_id: "synthetic-run-0001".to_string(),
            case_sha256: "c".repeat(64),
            decision_step: 3,
            proposal_sha256: "d".repeat(64),
            action_id: "switch-action-0003".to_string(),
            action_kind: "set_switch_state".to_string(),
            command_id: "cmd-0003".to_string(),
            command_sha256: "a".repeat(64),
            challenge_nonce: "b".repeat(64),
            controller_id: "controller-substation-1".to_string(),
            device_id: "switch-T56".to_string(),
            witness_issuer_id: issuer_id.clone(),
            witness_tool_source_commit: tool_source_commit.clone(),
            witness_tool_configuration_sha256: tool_configuration_sha256.clone(),
            minimum_monotonic_sequence: 40,
        };
        let receipt = NoActuationWitnessReceipt {
            schema_version: "0.1.0".to_string(),
            witness_id: "witness-controller-1".to_string(),
            issuer_id: issuer_id.clone(),
            controller_id: command_binding.controller_id.clone(),
            device_id: command_binding.device_id.clone(),
            run_id: command_binding.run_id.clone(),
            case_sha256: command_binding.case_sha256.clone(),
            decision_step: command_binding.decision_step,
            proposal_sha256: command_binding.proposal_sha256.clone(),
            action_id: command_binding.action_id.clone(),
            action_kind: command_binding.action_kind.clone(),
            command_id: command_binding.command_id.clone(),
            command_sha256: command_binding.command_sha256.clone(),
            challenge_nonce: command_binding.challenge_nonce.clone(),
            monotonic_sequence: 41,
            rejection_boundary: "before_actuation".to_string(),
            actuation_acquired: false,
            queue_dispatched: false,
            driver_acquired: false,
            output_transition_observed: false,
            hardware_feedback: ActuationHardwareFeedback::NoMovementObserved,
            observed_at_unix_ms: 1_500,
            expires_at_unix_ms: 5_000,
            tool_source_commit,
            tool_configuration_sha256,
        };
        let artifact_uri = format!(
            "power-fabric://actuation-witness/{}",
            command_binding.command_sha256
        );
        let artifact_sha256 = canonical_sha256(&receipt).unwrap();
        let mut parents = vec![
            command_binding.command_sha256.clone(),
            command_binding.proposal_sha256.clone(),
        ];
        parents.sort();
        let payload = AttestationPayload {
            schema_version: "0.1.0".to_string(),
            predicate_type: "power-restoration/no-actuation-witness/v1".to_string(),
            issuer_id,
            signer_role: SignerRole::ActuationWitness,
            subject_uri: artifact_uri,
            subject_sha256: artifact_sha256,
            run_id: command_binding.run_id.clone(),
            case_sha256: command_binding.case_sha256.clone(),
            decision_step: command_binding.decision_step,
            proposal_sha256: command_binding.proposal_sha256.clone(),
            tool_source_commit: receipt.tool_source_commit.clone(),
            tool_configuration_sha256: receipt.tool_configuration_sha256.clone(),
            issued_at_unix_ms: receipt.observed_at_unix_ms,
            expires_at_unix_ms: receipt.expires_at_unix_ms,
            parent_artifact_sha256: parents,
        };
        let signature = signing_key.sign(&canonical_bytes(&payload).unwrap());
        let signature_hex = signature.to_bytes().iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let envelope = AttestationEnvelope {
            key_id: "actuation-adapter-key-1".to_string(),
            payload,
            signature_hex,
        };
        let pinned_root_sha256 = canonical_sha256(&trust_root).unwrap();
        (trust_root, receipt, command_binding, envelope, pinned_root_sha256)
    }

    fn actuation_witness_bundle_fixture(
        receipt: &NoActuationWitnessReceipt,
        envelope: &AttestationEnvelope,
    ) -> (PathBuf, VerifiedBundle) {
        let root = test_bundle_root();
        fs::create_dir_all(root.join("artifacts/actuation")).unwrap();
        let receipt_bytes = canonical_bytes(receipt).unwrap();
        let envelope_bytes = canonical_bytes(envelope).unwrap();
        let receipt_uri = format!(
            "power-fabric://actuation-witness/{}",
            receipt.command_sha256
        );
        let envelope_uri = artifact_envelope_uri(envelope).unwrap();

        fs::write(root.join("artifacts/actuation/witness.json"), &receipt_bytes).unwrap();
        fs::write(root.join("artifacts/actuation/attestation.json"), &envelope_bytes).unwrap();

        let manifest = BundleManifest {
            schema_version: "0.1.0".to_string(),
            bundle_id: "synthetic-actuation-witness-bundle".to_string(),
            artifacts: vec![
                ArtifactRecord {
                    artifact_uri: receipt_uri,
                    relative_path: "artifacts/actuation/witness.json".to_string(),
                    media_type: "application/json".to_string(),
                    byte_length: receipt_bytes.len() as u64,
                    sha256: sha256_hex(&receipt_bytes),
                },
                ArtifactRecord {
                    artifact_uri: envelope_uri,
                    relative_path: "artifacts/actuation/attestation.json".to_string(),
                    media_type: "application/json".to_string(),
                    byte_length: envelope_bytes.len() as u64,
                    sha256: sha256_hex(&envelope_bytes),
                },
            ],
        };
        let bundle = verify_bundle_manifest(&root, &manifest, 1024 * 1024).unwrap();
        (root, bundle)
    }

    fn service_provider_profiles() -> Vec<ClaimProducerProfile> {
        vec![ClaimProducerProfile {
            signer_role: SignerRole::ServiceProvider,
            issuer_id: "synthetic-metering-system".to_string(),
            tool_source_commit: "1".repeat(40),
            tool_configuration_sha256: "2".repeat(64),
        }, ClaimProducerProfile {
            signer_role: SignerRole::EvidenceProvider,
            issuer_id: "synthetic-meter-data-source".to_string(),
            tool_source_commit: "3".repeat(40),
            tool_configuration_sha256: "4".repeat(64),
        }]
    }

    fn final_service_bundle_fixture(
        include_source_artifacts: bool,
    ) -> (
        PathBuf,
        VerifiedBundle,
        BenchmarkCase,
        RunReceipt,
        power_restoration_run_receipt::ArtifactRef,
    ) {
        let case: BenchmarkCase = serde_json::from_str(include_str!(
            "../../power-restoration-benchmark-case/fixtures/synthetic-restoration-case-v0.1.json"
        )).unwrap();
        let mut receipt: RunReceipt = serde_json::from_str(include_str!(
            "../../power-restoration-run-receipt/fixtures/synthetic-restoration-run-receipt-v0.1.json"
        )).unwrap();
        let case_sha = canonical_case_sha256(&case).unwrap();
        receipt.case.case_sha256 = case_sha.clone();
        receipt.case.case_ref = format!("power-restoration-case://{}", case.case_id);

        let meter_bytes = b"synthetic raw meter capture bytes";
        let calibration_bytes = b"synthetic calibration certificate bytes";
        let meter_uri = "meter-capture://synthetic-run-0001".to_string();
        let calibration_uri = "calibration://meter-set-001".to_string();
        let meter_sha = sha256_hex(meter_bytes);
        let calibration_sha = sha256_hex(calibration_bytes);

        let interval_duration_seconds = case.timebase.step_size * 60;
        let interval_duration_ms = u64::from(interval_duration_seconds) * 1_000;
        let first_start = 1_800_000_000_000_u64;
        let mut samples = Vec::new();
        for step in 0..case.timebase.horizon_steps {
            for load in &case.network.loads {
                let demand_kwh = load.active_power_kw
                    * f64::from(interval_duration_seconds) / 3_600.0;
                samples.push(
                    power_restoration_service_trajectory_audit::ServiceSample {
                        sample_id: format!("sample-{}-{step}", load.load_id),
                        load_id: load.load_id.clone(),
                        step,
                        interval_start_unix_ms: first_start
                            + u64::from(step) * interval_duration_ms,
                        utc_offset_minutes: 0,
                        demand_kwh,
                        delivered_kwh: Some(demand_kwh),
                        status: power_restoration_service_trajectory_audit::ServiceSampleStatus::Known,
                        evidence_ref: Some(meter_uri.clone()),
                        evidence_sha256: Some(meter_sha.clone()),
                        uncertainty_kwh: Some(0.01),
                        unknown_reason: None,
                        conflict_reason: None,
                        conflict_evidence: Vec::new(),
                    }
                );
            }
        }

        let mut parents = receipt.decision_steps.iter()
            .filter(|step| step.disposition == StepDisposition::Executed
                && step.service_outcome == ServiceOutcome::Scored)
            .filter_map(|step| step.service_trajectory.as_ref())
            .map(|artifact| power_restoration_service_trajectory_audit::ServiceSourceArtifact {
                artifact_ref: artifact.artifact_ref.clone(),
                sha256: artifact.sha256.clone(),
            })
            .collect::<Vec<_>>();
        parents.sort_by(|left, right| left.artifact_ref.cmp(&right.artifact_ref));

        let trajectory = ServiceTrajectoryRecord {
            schema_version: "0.1.0".to_string(),
            trajectory_id: "service-trajectory-claim-test-0001".to_string(),
            artifact_uri: "synthetic://final-service-trajectory".to_string(),
            run_id: receipt.run_id.clone(),
            case_ref: receipt.case.case_ref.clone(),
            case_sha256: case_sha.clone(),
            stream_kind: power_restoration_service_trajectory_audit::ServiceStreamKind::MeasuredService,
            service_id: "electricity-service".to_string(),
            service_type: "electricity".to_string(),
            service_boundary_id: "load-terminal".to_string(),
            native_unit: "kWh".to_string(),
            timebase: power_restoration_service_trajectory_audit::ServiceTimebase {
                timezone_id: "UTC".to_string(),
                first_interval_start_unix_ms: first_start,
                interval_duration_seconds,
                horizon_steps: case.timebase.horizon_steps,
            },
            producer: power_restoration_service_trajectory_audit::ServiceProducer {
                producer_id: "synthetic-metering-system".to_string(),
                source_commit: "1".repeat(40),
                configuration_sha256: "2".repeat(64),
                model_id: None,
                evidence_provider_id: Some("synthetic-meter-data-source".to_string()),
                instrument_id: Some("meter-set-001".to_string()),
                calibration_artifact_ref: Some(calibration_uri.clone()),
                calibration_artifact_sha256: Some(calibration_sha.clone()),
            },
            source_artifacts: vec![
                power_restoration_service_trajectory_audit::ServiceSourceArtifact {
                    artifact_ref: meter_uri.clone(),
                    sha256: meter_sha.clone(),
                },
                power_restoration_service_trajectory_audit::ServiceSourceArtifact {
                    artifact_ref: calibration_uri.clone(),
                    sha256: calibration_sha.clone(),
                },
            ],
            samples,
            work_milestones: Vec::new(),
            parent_service_artifacts: parents,
        };

        let weighted_delivered = trajectory.samples.iter().map(|sample| {
            let load = case.network.loads.iter()
                .find(|load| load.load_id == sample.load_id)
                .unwrap();
            sample.delivered_kwh.unwrap() * load.priority_weight
        }).sum::<f64>();
        receipt.objective.metric = case.objective.metric.clone();
        receipt.objective.unit = "weighted_kWh".to_string();
        let score = receipt.aggregate_score.as_mut().unwrap();
        score.metric_id = case.objective.metric.clone();
        score.unit = "weighted_kWh".to_string();
        score.value = weighted_delivered;

        let trajectory_bytes = canonical_bytes(&trajectory).unwrap();
        let final_ref = power_restoration_run_receipt::ArtifactRef {
            artifact_ref: trajectory.artifact_uri.clone(),
            sha256: sha256_hex(&trajectory_bytes),
        };
        receipt.final_service_trajectory = Some(final_ref.clone());

        let root = test_bundle_root();
        fs::create_dir_all(root.join("artifacts/service")).unwrap();
        fs::write(root.join("artifacts/service/final.json"), &trajectory_bytes).unwrap();
        let mut artifacts = vec![ArtifactRecord {
            artifact_uri: final_ref.artifact_ref.clone(),
            relative_path: "artifacts/service/final.json".to_string(),
            media_type: "application/json".to_string(),
            byte_length: trajectory_bytes.len() as u64,
            sha256: final_ref.sha256.clone(),
        }];

        if include_source_artifacts {
            fs::write(root.join("artifacts/service/meter.bin"), meter_bytes).unwrap();
            fs::write(root.join("artifacts/service/calibration.bin"), calibration_bytes).unwrap();
            artifacts.push(ArtifactRecord {
                artifact_uri: meter_uri,
                relative_path: "artifacts/service/meter.bin".to_string(),
                media_type: "application/octet-stream".to_string(),
                byte_length: meter_bytes.len() as u64,
                sha256: meter_sha,
            });
            artifacts.push(ArtifactRecord {
                artifact_uri: calibration_uri,
                relative_path: "artifacts/service/calibration.bin".to_string(),
                media_type: "application/octet-stream".to_string(),
                byte_length: calibration_bytes.len() as u64,
                sha256: calibration_sha,
            });
        }

        let manifest = BundleManifest {
            schema_version: "0.1.0".to_string(),
            bundle_id: "synthetic-final-service-trajectory-bundle".to_string(),
            artifacts,
        };
        let verified = verify_bundle_manifest(&root, &manifest, 8 * 1024 * 1024).unwrap();
        (root, verified, case, receipt, final_ref)
    }

    fn test_bundle_root() -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "power-restoration-artifact-verifier-{}-{}",
            std::process::id(),
            NEXT_BUNDLE_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("artifacts/validator")).expect("test bundle directories");
        fs::write(
            root.join("artifacts/validator/decision-0.json"),
            b"{\"accepted\":true,\"decisionStep\":0,\"scope\":\"synthetic-only\"}\n",
        )
        .expect("write test artifact");
        root
    }

    fn verified_single_artifact_fixture(bytes: &[u8]) -> (PathBuf, VerifiedBundle) {
        let root = test_bundle_root();
        fs::create_dir_all(root.join("artifacts/service")).unwrap();
        fs::write(root.join("artifacts/service/trajectory.json"), bytes).unwrap();
        let manifest = BundleManifest {
            schema_version: "0.1.0".to_string(),
            bundle_id: "verified-service-artifact-bundle".to_string(),
            artifacts: vec![ArtifactRecord {
                artifact_uri: "service://trajectory/final".to_string(),
                relative_path: "artifacts/service/trajectory.json".to_string(),
                media_type: "application/json".to_string(),
                byte_length: bytes.len() as u64,
                sha256: sha256_hex(bytes),
            }],
        };
        let verified = verify_bundle_manifest(&root, &manifest, 1024 * 1024).unwrap();
        (root, verified)
    }

    #[test]
    fn service_sources_require_distinct_evidence_provider_attestations() {
        let (root, bundle, case, receipt, final_ref) =
            final_service_bundle_fixture(true);
        let (trajectory, _) = verify_final_service_trajectory_bundle(
            &root,
            &bundle,
            &final_ref,
            &receipt,
            &case,
            &service_provider_profiles(),
            8 * 1024 * 1024,
        ).unwrap();
        let mut expected_artifacts = BTreeMap::new();
        let mut requirements = Vec::new();
        let mut errors = Vec::new();
        add_service_source_attestations(
            &trajectory,
            &receipt,
            &canonical_case_sha256(&case).unwrap(),
            &proposal_set_sha256(&receipt).unwrap(),
            &service_provider_profiles(),
            &mut expected_artifacts,
            &mut requirements,
            &mut errors,
        );
        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(requirements.len(), trajectory.source_artifacts.len());
        assert!(requirements.iter().all(|requirement| {
            requirement.signer_role == SignerRole::EvidenceProvider
                && requirement.issuer_id == "synthetic-meter-data-source"
                && requirement.predicate_type == "power-restoration/service-source-evidence/v1"
        }));
        assert_eq!(expected_artifacts.len(), trajectory.source_artifacts.len());
    }

    #[test]
    fn service_sources_reject_missing_evidence_provider_profile() {
        let (root, bundle, case, receipt, final_ref) =
            final_service_bundle_fixture(true);
        let (trajectory, _) = verify_final_service_trajectory_bundle(
            &root,
            &bundle,
            &final_ref,
            &receipt,
            &case,
            &service_provider_profiles(),
            8 * 1024 * 1024,
        ).unwrap();
        let profiles = service_provider_profiles().into_iter()
            .filter(|profile| profile.signer_role == SignerRole::ServiceProvider)
            .collect::<Vec<_>>();
        let mut expected_artifacts = BTreeMap::new();
        let mut requirements = Vec::new();
        let mut errors = Vec::new();
        add_service_source_attestations(
            &trajectory,
            &receipt,
            &canonical_case_sha256(&case).unwrap(),
            &proposal_set_sha256(&receipt).unwrap(),
            &profiles,
            &mut expected_artifacts,
            &mut requirements,
            &mut errors,
        );
        assert!(errors.iter().any(|error| error.contains("no pinned producer profile")));
        assert!(requirements.is_empty());
    }

    #[test]
    fn service_sources_reject_same_producer_identity_for_service_and_evidence() {
        let (root, bundle, case, receipt, final_ref) =
            final_service_bundle_fixture(true);
        let (mut trajectory, _) = verify_final_service_trajectory_bundle(
            &root,
            &bundle,
            &final_ref,
            &receipt,
            &case,
            &service_provider_profiles(),
            8 * 1024 * 1024,
        ).unwrap();
        trajectory.producer.evidence_provider_id = Some("synthetic-metering-system".to_string());
        let mut profiles = service_provider_profiles();
        let source_profile = profiles.iter_mut()
            .find(|profile| profile.signer_role == SignerRole::EvidenceProvider)
            .unwrap();
        source_profile.issuer_id = "synthetic-metering-system".to_string();
        let mut expected_artifacts = BTreeMap::new();
        let mut requirements = Vec::new();
        let mut errors = Vec::new();
        add_service_source_attestations(
            &trajectory,
            &receipt,
            &canonical_case_sha256(&case).unwrap(),
            &proposal_set_sha256(&receipt).unwrap(),
            &profiles,
            &mut expected_artifacts,
            &mut requirements,
            &mut errors,
        );
        assert!(errors.iter().any(|error| error.contains("must be distinct producer identities")));
        assert!(requirements.is_empty());
    }

    #[test]
    fn final_service_trajectory_passes_only_with_every_source_byte_in_verified_bundle() {
        let (root, bundle, case, receipt, final_ref) =
            final_service_bundle_fixture(true);
        let (trajectory, report) = verify_final_service_trajectory_bundle(
            &root,
            &bundle,
            &final_ref,
            &receipt,
            &case,
            &service_provider_profiles(),
            8 * 1024 * 1024,
        ).unwrap();
        assert_eq!(trajectory.artifact_uri, final_ref.artifact_ref);
        assert_eq!(report.result_status, "measured_service_and_score_reconciled");
    }

    #[test]
    fn final_service_trajectory_fails_when_source_bytes_are_omitted() {
        let (root, bundle, case, receipt, final_ref) =
            final_service_bundle_fixture(false);
        let errors = verify_final_service_trajectory_bundle(
            &root,
            &bundle,
            &final_ref,
            &receipt,
            &case,
            &service_provider_profiles(),
            8 * 1024 * 1024,
        ).unwrap_err();
        assert!(errors.iter().any(|error| error.contains("missing from the byte-verified bundle")));
    }

    #[test]
    fn verified_bundle_re_reads_exact_service_artifact_bytes() {
        let bytes = br#"{"status":"synthetic"}"#;
        let (root, bundle) = verified_single_artifact_fixture(bytes);
        let read = bundle.read_verified_artifact_bytes(
            &root,
            "service://trajectory/final",
            1024 * 1024,
        ).unwrap();
        assert_eq!(read, bytes);
    }

    #[test]
    fn verified_bundle_rejects_service_artifact_mutation_after_initial_verification() {
        let bytes = br#"{"status":"synthetic"}"#;
        let (root, bundle) = verified_single_artifact_fixture(bytes);
        fs::write(
            root.join("artifacts/service/trajectory.json"),
            br#"{"status":"substituted"}"#,
        ).unwrap();
        assert!(bundle.read_verified_artifact_bytes(
            &root,
            "service://trajectory/final",
            1024 * 1024,
        ).is_err());
    }

    #[test]
    fn verified_bundle_does_not_read_artifacts_outside_its_manifest() {
        let bytes = br#"{"status":"synthetic"}"#;
        let (root, bundle) = verified_single_artifact_fixture(bytes);
        assert!(bundle.read_verified_artifact_bytes(
            &root,
            "service://trajectory/other",
            1024 * 1024,
        ).is_err());
    }

    #[test]
    fn fresh_command_bound_actuation_witness_verifies_and_consumes_nonce() {
        let (trust, receipt, binding, envelope, root_sha256) =
            actuation_witness_fixtures();
        let (_bundle_root, verified_bundle) =
            actuation_witness_bundle_fixture(&receipt, &envelope);
        let mut replay = ActuationWitnessReplayState::default();
        let verified = verify_no_actuation_witness(
            &verified_bundle, &receipt, &binding, &envelope, &trust, &root_sha256, 2_000, 5_000, &mut replay,
        ).unwrap();
        assert_eq!(
            verified.result_status,
            "no_actuation_witness_signature_and_command_binding_verified"
        );
        assert!(replay.used_challenge_nonces.contains(&binding.challenge_nonce));
        assert_eq!(
            replay.last_sequence_by_controller_device
                .get("controller-substation-1/switch-T56"),
            Some(&41)
        );
    }

    #[test]
    fn actuation_witness_challenge_replay_is_rejected() {
        let (trust, receipt, binding, envelope, root_sha256) =
            actuation_witness_fixtures();
        let (_bundle_root, verified_bundle) =
            actuation_witness_bundle_fixture(&receipt, &envelope);
        let mut replay = ActuationWitnessReplayState::default();
        verify_no_actuation_witness(
            &verified_bundle, &receipt, &binding, &envelope, &trust, &root_sha256, 2_000, 5_000, &mut replay,
        ).unwrap();
        assert!(verify_no_actuation_witness(
            &verified_bundle, &receipt, &binding, &envelope, &trust, &root_sha256, 2_000, 5_000, &mut replay,
        ).is_err());
    }

    #[test]
    fn actuation_witness_cannot_be_replayed_for_another_command() {
        let (trust, receipt, mut binding, envelope, root_sha256) =
            actuation_witness_fixtures();
        binding.command_id = "cmd-substituted".to_string();
        let (_bundle_root, verified_bundle) =
            actuation_witness_bundle_fixture(&receipt, &envelope);
        let mut replay = ActuationWitnessReplayState::default();
        assert!(verify_no_actuation_witness(
            &verified_bundle, &receipt, &binding, &envelope, &trust, &root_sha256, 2_000, 5_000, &mut replay,
        ).is_err());
    }

    #[test]
    fn actuation_witness_rejects_contradictory_output_feedback() {
        let (trust, mut receipt, binding, envelope, root_sha256) =
            actuation_witness_fixtures();
        receipt.output_transition_observed = true;
        let (_bundle_root, verified_bundle) =
            actuation_witness_bundle_fixture(&receipt, &envelope);
        let mut replay = ActuationWitnessReplayState::default();
        assert!(verify_no_actuation_witness(
            &verified_bundle, &receipt, &binding, &envelope, &trust, &root_sha256, 2_000, 5_000, &mut replay,
        ).is_err());
    }

    #[test]
    fn actuation_witness_key_cannot_be_reused_for_transition_engine_role() {
        let (mut trust, _, _, envelope, _) = actuation_witness_fixtures();
        let mut alias = trust.keys[0].clone();
        alias.key_id = "transition-engine-alias".to_string();
        alias.producer_id = "synthetic-transition-engine".to_string();
        alias.roles = vec![SignerRole::TransitionEngine];
        trust.keys.push(alias);
        let mut transition_envelope = envelope.clone();
        transition_envelope.key_id = "transition-engine-alias".to_string();
        transition_envelope.payload.issuer_id = "synthetic-transition-engine".to_string();
        transition_envelope.payload.signer_role = SignerRole::TransitionEngine;
        assert!(validate_role_separation(&[envelope, transition_envelope], &trust).is_err());
    }

    #[test]
    fn actuation_witness_receipt_must_be_in_verified_bundle() {
        let (trust, receipt, binding, envelope, root_sha256) =
            actuation_witness_fixtures();
        let root = test_bundle_root();
        fs::create_dir_all(root.join("artifacts/actuation")).unwrap();
        let envelope_bytes = canonical_bytes(&envelope).unwrap();
        fs::write(root.join("artifacts/actuation/attestation.json"), &envelope_bytes).unwrap();
        let manifest = BundleManifest {
            schema_version: "0.1.0".to_string(),
            bundle_id: "attestation-only-bundle".to_string(),
            artifacts: vec![ArtifactRecord {
                artifact_uri: artifact_envelope_uri(&envelope).unwrap(),
                relative_path: "artifacts/actuation/attestation.json".to_string(),
                media_type: "application/json".to_string(),
                byte_length: envelope_bytes.len() as u64,
                sha256: sha256_hex(&envelope_bytes),
            }],
        };
        let bundle = verify_bundle_manifest(&root, &manifest, 1024 * 1024).unwrap();
        let mut replay = ActuationWitnessReplayState::default();
        let errors = verify_no_actuation_witness(
            &bundle, &receipt, &binding, &envelope, &trust, &root_sha256,
            2_000, 5_000, &mut replay,
        ).unwrap_err();
        assert!(errors.iter().any(|error| error.contains("not present with exact bytes")));
    }

    #[test]
    fn actuation_witness_rejects_stale_sequence_and_expired_window() {
        let (trust, mut receipt, binding, envelope, root_sha256) =
            actuation_witness_fixtures();
        receipt.monotonic_sequence = 40;
        receipt.expires_at_unix_ms = 1_999;
        let (_bundle_root, verified_bundle) =
            actuation_witness_bundle_fixture(&receipt, &envelope);
        let mut replay = ActuationWitnessReplayState::default();
        assert!(verify_no_actuation_witness(
            &verified_bundle, &receipt, &binding, &envelope, &trust, &root_sha256, 2_000, 5_000, &mut replay,
        ).is_err());
    }

    #[test]
    fn stale_unused_observation_does_not_invalidate_a_run_claim() {
        let (mut batch, proposal, case) = {
            let batch: EvidenceBatch = serde_json::from_str(include_str!(
                "../fixtures/synthetic-observation-evidence-batch-v0.1.json"
            )).unwrap();
            let proposal: power_restoration_action_proposal::ActionProposal = serde_json::from_str(include_str!(
                "../../power-restoration-action-proposal/fixtures/synthetic-information-request-proposal-v0.1.json"
            )).unwrap();
            let case: power_restoration_benchmark_case::BenchmarkCase = serde_json::from_str(include_str!(
                "../../power-restoration-benchmark-case/fixtures/synthetic-restoration-case-v0.1.json"
            )).unwrap();
            (batch, proposal, case)
        };
        batch.results[0].evidence[0].quality = EvidenceQuality::Stale;
        let report = validate_evidence_batch(&batch, &proposal, &case).unwrap();
        assert_eq!(report.stale_at_presentation_count, 1);
        // It remains a valid, visible observation record. A later proposal
        // citing it as a precondition is rejected by the integrated gate.
    }

    #[test]
    fn declared_case_source_artifact_requires_raw_bytes_digest() {
        let mut case: power_restoration_benchmark_case::BenchmarkCase = serde_json::from_str(include_str!(
            "../../power-restoration-benchmark-case/fixtures/synthetic-restoration-case-v0.1.json"
        )).unwrap();
        case.provenance.source_artifacts.push(
            power_restoration_benchmark_case::SourceArtifact {
                path: "feeders/feeder.dss".to_string(),
                vcs_object_id: "a".repeat(40),
                vcs_object_format: power_restoration_benchmark_case::VcsObjectFormat::GitSha1,
                raw_sha256: None,
            }
        );
        let receipt: power_restoration_run_receipt::RunReceipt = serde_json::from_str(include_str!(
            "../../power-restoration-run-receipt/fixtures/synthetic-restoration-run-receipt-v0.1.json"
        )).unwrap();
        let manifest = BundleManifest {
            schema_version: "0.1.0".to_string(),
            bundle_id: "empty-claim-bundle".to_string(),
            artifacts: Vec::new(),
        };
        let trust = TrustRoot {
            schema_version: "0.1.0".to_string(),
            root_id: "empty-root".to_string(),
            valid_from_unix_ms: 0,
            valid_until_unix_ms: 10_000,
            keys: Vec::new(),
        };
        let profiles = vec![ClaimProducerProfile {
            signer_role: SignerRole::RunRecorder,
            issuer_id: "synthetic-run-recorder".to_string(),
            tool_source_commit: "1".repeat(40),
            tool_configuration_sha256: "2".repeat(64),
        }];
        let pinned = "0".repeat(64);
        let result = verify_restoration_claim_evidence(
            Path::new("."),
            &case,
            &receipt,
            &[],
            &manifest,
            &trust,
            &pinned,
            &[],
            &profiles,
            "power-fabric://run/synthetic/receipt",
            2_000,
            1024,
        );
        assert!(result.unwrap_err().iter().any(|error| {
            error.contains("lacks a raw SHA-256 digest")
        }));
    }

    #[test]
    fn claim_verifier_rejects_unacquired_action_precondition_evidence() {
        let case: power_restoration_benchmark_case::BenchmarkCase = serde_json::from_str(include_str!(
            "../../power-restoration-benchmark-case/fixtures/synthetic-restoration-case-v0.1.json"
        )).unwrap();
        let receipt: power_restoration_run_receipt::RunReceipt = serde_json::from_str(include_str!(
            "../../power-restoration-run-receipt/fixtures/synthetic-restoration-run-receipt-v0.1.json"
        )).unwrap();
        let manifest = BundleManifest {
            schema_version: "0.1.0".to_string(),
            bundle_id: "empty-claim-bundle".to_string(),
            artifacts: Vec::new(),
        };
        let trust = TrustRoot {
            schema_version: "0.1.0".to_string(),
            root_id: "empty-root".to_string(),
            valid_from_unix_ms: 0,
            valid_until_unix_ms: 10_000,
            keys: Vec::new(),
        };
        let profile = ClaimProducerProfile {
            signer_role: SignerRole::RunRecorder,
            issuer_id: "synthetic-run-recorder".to_string(),
            tool_source_commit: "1".repeat(40),
            tool_configuration_sha256: "2".repeat(64),
        };
        let profiles = vec![profile];
        let pinned = "0".repeat(64);
        let result = verify_restoration_claim_evidence(
            Path::new("."),
            &case,
            &receipt,
            &[],
            &manifest,
            &trust,
            &pinned,
            &[],
            &profiles,
            "power-fabric://run/synthetic/receipt",
            2_000,
            1024,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().iter().any(|error| {
            error.contains("absent from all acquired observation results")
        }));
    }

    #[test]
    fn claim_verifier_requires_a_pinned_run_recorder_profile() {
        let case: power_restoration_benchmark_case::BenchmarkCase = serde_json::from_str(include_str!(
            "../../power-restoration-benchmark-case/fixtures/synthetic-restoration-case-v0.1.json"
        )).unwrap();
        let receipt: power_restoration_run_receipt::RunReceipt = serde_json::from_str(include_str!(
            "../../power-restoration-run-receipt/fixtures/synthetic-restoration-run-receipt-v0.1.json"
        )).unwrap();
        let manifest = BundleManifest {
            schema_version: "0.1.0".to_string(),
            bundle_id: "empty-claim-bundle".to_string(),
            artifacts: Vec::new(),
        };
        let trust = TrustRoot {
            schema_version: "0.1.0".to_string(),
            root_id: "empty-root".to_string(),
            valid_from_unix_ms: 0,
            valid_until_unix_ms: 10_000,
            keys: Vec::new(),
        };
        let pinned = "0".repeat(64);
        let result = verify_restoration_claim_evidence(
            Path::new("."),
            &case,
            &receipt,
            &[],
            &manifest,
            &trust,
            &pinned,
            &[],
            &[],
            "power-fabric://run/synthetic/receipt",
            2_000,
            1024,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().iter().any(|error| error.contains("RunRecorder")));
    }

    #[test]
    fn pinned_trust_root_and_strict_ed25519_signature_verify() {
        let (trust, envelope, expected, _) = fixtures();
        let report = verify_attestation(
            &envelope,
            &trust,
            TRUST_ROOT_SHA256,
            2_000,
            &expected,
        )
        .unwrap();
        assert_eq!(report.signature_status, "strict_ed25519_verified_against_pinned_trust_root");
        assert_eq!(report.signer_role, SignerRole::ElectricalValidator);
    }

    #[test]
    fn changed_signed_payload_is_rejected() {
        let (trust, mut envelope, expected, _) = fixtures();
        envelope.payload.run_id = "attacker-substituted-run".to_string();
        assert!(verify_attestation(&envelope, &trust, TRUST_ROOT_SHA256, 2_000, &expected).is_err());
    }

    #[test]
    fn untrusted_key_is_rejected() {
        let (trust, mut envelope, expected, _) = fixtures();
        envelope.key_id = "attacker-key".to_string();
        assert!(verify_attestation(&envelope, &trust, TRUST_ROOT_SHA256, 2_000, &expected).is_err());
    }

    #[test]
    fn expired_attestation_is_rejected() {
        let (trust, envelope, expected, _) = fixtures();
        assert!(verify_attestation(&envelope, &trust, TRUST_ROOT_SHA256, 9_000, &expected).is_err());
    }

    #[test]
    fn attestation_cannot_outlive_its_trusted_signing_key() {
        let (mut trust, envelope, expected, _) = fixtures();
        trust.keys[0].not_after_unix_ms = 8_000;
        let root_hash = canonical_sha256(&trust).unwrap();
        assert!(verify_attestation(&envelope, &trust, &root_hash, 2_000, &expected).is_err());
    }

    #[test]
    fn revoked_key_is_rejected() {
        let (mut trust, envelope, expected, _) = fixtures();
        trust.keys[0].revoked_at_unix_ms = Some(1_500);
        let root_hash = canonical_sha256(&trust).unwrap();
        assert!(verify_attestation(&envelope, &trust, &root_hash, 2_000, &expected).is_err());
    }

    #[test]
    fn caller_pinned_trust_root_prevents_root_substitution() {
        let (mut trust, envelope, expected, _) = fixtures();
        trust.root_id = "attacker-root".to_string();
        assert!(verify_attestation(&envelope, &trust, TRUST_ROOT_SHA256, 2_000, &expected).is_err());
    }

    #[test]
    fn wrong_issuer_is_rejected_even_with_a_trusted_key() {
        let (trust, envelope, mut expected, _) = fixtures();
        expected.issuer_id = "different-producer".to_string();
        assert!(verify_attestation(&envelope, &trust, TRUST_ROOT_SHA256, 2_000, &expected).is_err());
    }

    #[test]
    fn wrong_role_or_wrong_case_binding_is_rejected() {
        let (trust, envelope, mut expected, _) = fixtures();
        expected.signer_role = SignerRole::Scorer;
        assert!(verify_attestation(&envelope, &trust, TRUST_ROOT_SHA256, 2_000, &expected).is_err());
        expected.signer_role = SignerRole::ElectricalValidator;
        expected.case_sha256 = "c".repeat(64);
        assert!(verify_attestation(&envelope, &trust, TRUST_ROOT_SHA256, 2_000, &expected).is_err());
    }

    #[test]
    fn wrong_predicate_type_is_rejected_even_with_a_trusted_signer() {
        let (trust, envelope, mut expected, _) = fixtures();
        expected.predicate_type = "power-restoration/service-scoring/v1".to_string();
        assert!(verify_attestation(&envelope, &trust, TRUST_ROOT_SHA256, 2_000, &expected).is_err());
    }

    #[test]
    fn planner_key_cannot_also_satisfy_validator_or_scorer_role() {
        let (_, mut envelope, _, _) = fixtures();
        let mut planner = envelope.clone();
        planner.payload.signer_role = SignerRole::Planner;
        envelope.payload.signer_role = SignerRole::ElectricalValidator;
        let (trust, _, _, _) = fixtures();
        assert!(validate_role_separation(&[planner, envelope], &trust).is_err());
    }

    #[test]
    fn one_public_key_cannot_impersonate_multiple_producer_identities() {
        let (mut trust, envelope, _, _) = fixtures();
        let mut alias = trust.keys[0].clone();
        alias.key_id = "synthetic-validator-alias".to_string();
        alias.producer_id = "different-validator-identity".to_string();
        trust.keys.push(alias);
        let mut second = envelope.clone();
        second.key_id = "synthetic-validator-alias".to_string();
        second.payload.issuer_id = "different-validator-identity".to_string();
        // The alias has the same public key but cannot be treated as another producer.
        assert!(validate_role_separation(&[envelope, second], &trust).is_err());
    }

    #[test]
    fn duplicated_public_key_under_different_key_ids_cannot_fill_independent_roles() {
        let (mut trust, envelope, _, _) = fixtures();
        let mut alias = trust.keys[0].clone();
        alias.key_id = "synthetic-validator-key-alias".to_string();
        alias.roles = vec![SignerRole::Planner];
        trust.keys.push(alias);
        let mut planner = envelope.clone();
        planner.key_id = "synthetic-validator-key-alias".to_string();
        planner.payload.signer_role = SignerRole::Planner;
        assert!(validate_role_separation(&[planner, envelope], &trust).is_err());
    }

    #[test]
    fn valid_bundle_manifest_resolves_exact_bytes() {
        let root = test_bundle_root();
        let (trust, envelope, expected, manifest) = fixtures();
        let result = verify_bundle_manifest(&root, &manifest, 1024).unwrap();
        assert_eq!(result.report().artifact_count, 1);
        let verified = verify_attestation(
            &envelope,
            &trust,
            TRUST_ROOT_SHA256,
            2_000,
            &expected,
        )
        .unwrap();
        assert_eq!(verified.subject_sha256, result.report().verified_artifacts[0].sha256);
        fs::remove_dir_all(root).expect("remove test bundle");
    }

    #[test]
    fn attestation_set_requires_manifest_subjects_and_every_declared_role() {
        let root = test_bundle_root();
        let (trust, envelope, expected, manifest) = fixtures();
        let manifest_report = verify_bundle_manifest(&root, &manifest, 1024).unwrap();
        let verified = verify_attestation_set(
            &manifest_report,
            &trust,
            TRUST_ROOT_SHA256,
            2_000,
            &[expected.clone()],
            &[envelope.clone()],
        )
        .unwrap();
        assert_eq!(verified.len(), 1);

        assert!(verify_attestation_set(
            &manifest_report,
            &trust,
            TRUST_ROOT_SHA256,
            2_000,
            &[expected.clone()],
            &[],
        ).is_err());

        let mut missing_subject = expected;
        missing_subject.artifact_uri = "bundle://validator/missing.json".to_string();
        assert!(verify_attestation_set(
            &manifest_report,
            &trust,
            TRUST_ROOT_SHA256,
            2_000,
            &[missing_subject],
            &[envelope],
        ).is_err());
        fs::remove_dir_all(root).expect("remove test bundle");
    }

    #[test]
    fn attestation_set_rejects_duplicate_and_unexpected_envelopes() {
        let root = test_bundle_root();
        let (trust, envelope, expected, manifest) = fixtures();
        let manifest_report = verify_bundle_manifest(&root, &manifest, 1024).unwrap();
        assert!(verify_attestation_set(
            &manifest_report,
            &trust,
            TRUST_ROOT_SHA256,
            2_000,
            &[expected.clone()],
            &[envelope.clone(), envelope.clone()],
        ).is_err());

        let mut unexpected = envelope.clone();
        unexpected.payload.subject_uri = "bundle://validator/other.json".to_string();
        assert!(verify_attestation_set(
            &manifest_report,
            &trust,
            TRUST_ROOT_SHA256,
            2_000,
            &[expected],
            &[unexpected],
        ).is_err());
        fs::remove_dir_all(root).expect("remove test bundle");
    }

    #[test]
    fn byte_substitution_is_rejected() {
        let root = test_bundle_root();
        fs::write(
            root.join("artifacts/validator/decision-0.json"),
            b"{\"accepted\":false,\"decisionStep\":0,\"scope\":\"synthetic-only\"}\n",
        )
        .unwrap();
        let (_, _, _, manifest) = fixtures();
        assert!(verify_bundle_manifest(&root, &manifest, 1024).is_err());
        fs::remove_dir_all(root).expect("remove test bundle");
    }

    #[test]
    fn absolute_and_parent_traversal_paths_are_rejected() {
        assert!(validate_relative_path("/etc/passwd").is_err());
        assert!(validate_relative_path("../outside.json").is_err());
        assert!(validate_relative_path("a/../../outside.json").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn symlink_artifact_path_is_rejected() {
        use std::os::unix::fs::symlink;
        let root = test_bundle_root();
        let outside = root.join("outside.json");
        fs::write(&outside, b"outside").unwrap();
        let link = root.join("artifacts/validator/linked.json");
        symlink(&outside, &link).unwrap();
        let mut record = fixtures().3.artifacts[0].clone();
        record.relative_path = "artifacts/validator/linked.json".to_string();
        record.byte_length = 7;
        record.sha256 = sha256_hex(b"outside");
        assert!(verify_artifact_bytes(&root, &record, 1024).is_err());
        fs::remove_dir_all(root).expect("remove test bundle");
    }

    #[test]
    fn duplicate_artifact_uris_and_paths_are_rejected() {
        let root = test_bundle_root();
        let (_, _, _, mut manifest) = fixtures();
        manifest.artifacts.push(manifest.artifacts[0].clone());
        assert!(verify_bundle_manifest(&root, &manifest, 1024).is_err());
        fs::remove_dir_all(root).expect("remove test bundle");
    }

    #[test]
    fn invalid_signature_hex_is_rejected() {
        let (trust, mut envelope, expected, _) = fixtures();
        envelope.signature_hex = "not-a-signature".to_string();
        assert!(verify_attestation(&envelope, &trust, TRUST_ROOT_SHA256, 2_000, &expected).is_err());
    }
}
