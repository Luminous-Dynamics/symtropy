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
