//! Scientific evidence envelopes with pluggable SCITT/COSE verification.
//!
//! This module verifies an evidence dependency graph and delegates canonical
//! encoding, issuer signature verification and transparency receipt proofs to
//! explicit providers. It does not implement COSE, CBOR, a signature algorithm
//! or a Verifiable Data Structure. A provider must be backed by a real,
//! independently tested implementation before its result is production-grade.

use std::collections::{BTreeMap, BTreeSet};

const MAX_ID_BYTES: usize = 256;
const MAX_STATEMENTS: usize = 10_000;
const MAX_ARTIFACTS: usize = 100_000;
const MAX_RECEIPTS_PER_STATEMENT: usize = 16;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum ScientificArtifactRole {
    SourceRevision,
    DependencyLockfile,
    BuildAttestation,
    InputManifest,
    SplitManifest,
    CalibrationRecord,
    RawObservation,
    ActionRealization,
    PhysicalTransition,
    InferenceReceipt,
    PolicyPlan,
    RiskReceipt,
    CounterexampleAdmission,
    LedgerTranscript,
    BenchmarkTrial,
    StatisticalAnalysis,
    Other,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvidenceArtifactReference {
    pub artifact_id: String,
    pub content_digest: String,
    pub media_type: String,
    pub role: ScientificArtifactRole,
    pub byte_length: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScientificEvidenceStatement {
    pub schema_id: String,
    pub schema_version: String,
    pub issuer_id: String,
    pub subject_id: String,
    pub statement_type: String,
    pub issued_at_unix_seconds: u64,
    pub source_repository: String,
    pub source_commit_sha: String,
    pub source_tree_digest: String,
    pub dependency_lock_digest: String,
    pub workflow_run_id: String,
    pub workflow_head_sha: String,
    pub verification_profile: String,
    pub artifact_references: Vec<EvidenceArtifactReference>,
    /// Content-addressed predecessor statement IDs. This is an ordered
    /// revision history, not a mutable pointer to the latest statement.
    pub predecessor_statement_digests: Vec<String>,
    /// Limitations are mandatory; an evidence envelope cannot declare itself
    /// universally valid merely because its signature and inclusion proofs pass.
    pub declared_limitations: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransparencyReceiptEnvelope {
    pub transparency_service_id: String,
    pub vds_profile: String,
    /// Opaque RFC 9942 / COSE receipt bytes interpreted by the provider.
    pub cose_receipt_bytes: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignedScientificEvidenceStatement {
    pub statement: ScientificEvidenceStatement,
    /// Canonical statement payload as encoded by the selected evidence profile.
    pub canonical_payload: Vec<u8>,
    /// Opaque COSE_Sign1 (or profile-specific) signed statement bytes.
    pub signed_statement_bytes: Vec<u8>,
    pub signing_key_id: String,
    pub signature_algorithm: String,
    pub transparency_receipts: Vec<TransparencyReceiptEnvelope>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvidenceArtifactBlob {
    pub content_digest: String,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScientificEvidencePackage {
    pub statements: Vec<SignedScientificEvidenceStatement>,
    pub artifacts: Vec<EvidenceArtifactBlob>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedIssuerIdentity {
    pub issuer_id: String,
    pub key_id: String,
    pub signature_algorithm: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedTransparencyReceipt {
    pub transparency_service_id: String,
    pub vds_profile: String,
    pub statement_digest: String,
    pub leaf_index: u64,
    pub tree_size: u64,
    pub receipt_signature_verified: bool,
    pub inclusion_proof_verified: bool,
    pub consistency_proof_verified: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedStatementReceipt {
    pub statement_digest: String,
    pub issuer_id: String,
    pub subject_id: String,
    pub verified_artifact_count: usize,
    pub verified_transparency_services: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScientificEvidenceVerificationReceipt {
    pub statement_count: usize,
    pub artifact_count: usize,
    pub verified_statements: Vec<VerifiedStatementReceipt>,
    pub all_canonical_payloads_match: bool,
    pub all_artifact_digests_match: bool,
    pub all_issuer_signatures_verified: bool,
    pub all_transparency_receipts_verified: bool,
    pub predecessor_graph_acyclic: bool,
    pub no_unreferenced_artifacts: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ScientificEvidenceStage {
    PackageEnvelope,
    StatementStructure,
    CanonicalEncoding,
    StatementDigest,
    ArtifactDigest,
    PredecessorGraph,
    IssuerSignature,
    TransparencyReceipt,
    FinalConsistency,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScientificEvidenceError {
    pub stage: ScientificEvidenceStage,
    pub statement_digest: Option<String>,
    pub artifact_digest: Option<String>,
    pub reason: String,
}

impl std::fmt::Display for ScientificEvidenceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{:?}: {}", self.stage, self.reason)
    }
}

impl std::error::Error for ScientificEvidenceError {}

/// Canonical payload encoding for the selected evidence profile.
///
/// A production SCITT profile should use deterministic CBOR/CDDL structures
/// corresponding to RFC 9943 Signed Statements. This trait deliberately
/// prevents the structural model from claiming that a Rust struct is itself a
/// standards-compliant wire encoding.
pub trait EvidencePayloadEncoder {
    fn encode_canonical_payload(
        &self,
        statement: &ScientificEvidenceStatement,
    ) -> Result<Vec<u8>, String>;
}

/// Digest provider over exact bytes. Production implementations must use a
/// cryptographic hash selected by the evidence profile (e.g. SHA-256).
pub trait EvidenceDigestProvider {
    fn sha256(&self, bytes: &[u8]) -> Result<[u8; 32], String>;
}

/// COSE/signature adapter. The implementation must verify the signed bytes
/// against the exact canonical payload and return the identity proven by the
/// protected signing structure, not merely echo caller-supplied claims.
pub trait IssuerSignatureVerifier {
    fn verify_signed_statement(
        &self,
        signed_statement_bytes: &[u8],
        canonical_payload: &[u8],
    ) -> Result<VerifiedIssuerIdentity, String>;
}

/// RFC 9942 / VDS adapter. The implementation must verify the receipt
/// signature and its inclusion proof, and verify consistency against the
/// profile's required prior tree checkpoint.
pub trait TransparencyReceiptVerifier {
    fn verify_receipt(
        &self,
        expected_statement_digest: &str,
        receipt: &TransparencyReceiptEnvelope,
    ) -> Result<VerifiedTransparencyReceipt, String>;
}

fn fail(
    stage: ScientificEvidenceStage,
    statement_digest: Option<&str>,
    artifact_digest: Option<&str>,
    reason: impl Into<String>,
) -> ScientificEvidenceError {
    ScientificEvidenceError {
        stage,
        statement_digest: statement_digest.map(str::to_string),
        artifact_digest: artifact_digest.map(str::to_string),
        reason: reason.into(),
    }
}

fn valid_id(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= MAX_ID_BYTES
}

pub fn format_sha256_digest(digest: [u8; 32]) -> String {
    let mut value = String::with_capacity(71);
    value.push_str("sha256:");
    for byte in digest {
        use std::fmt::Write;
        let _ = write!(&mut value, "{byte:02x}");
    }
    value
}

fn valid_sha256_digest(value: &str) -> bool {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return false;
    };
    hex.len() == 64
        && hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn valid_git_commit(value: &str) -> bool {
    (value.len() == 40 || value.len() == 64)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn validate_statement_structure(
    statement: &ScientificEvidenceStatement,
) -> Result<(), ScientificEvidenceError> {
    for (value, label) in [
        (&statement.schema_id, "schema ID"),
        (&statement.schema_version, "schema version"),
        (&statement.issuer_id, "issuer identity"),
        (&statement.subject_id, "subject identity"),
        (&statement.statement_type, "statement type"),
        (&statement.source_repository, "source repository"),
        (&statement.workflow_run_id, "workflow run ID"),
        (&statement.verification_profile, "verification profile"),
    ] {
        if !valid_id(value) {
            return Err(fail(
                ScientificEvidenceStage::StatementStructure,
                None,
                None,
                format!("{label} is empty or overlong"),
            ));
        }
    }
    if statement.issued_at_unix_seconds == 0 {
        return Err(fail(
            ScientificEvidenceStage::StatementStructure,
            None,
            None,
            "statement issue time must be a positive Unix timestamp",
        ));
    }
    if !valid_git_commit(&statement.source_commit_sha)
        || !valid_git_commit(&statement.workflow_head_sha)
        || !valid_sha256_digest(&statement.source_tree_digest)
        || !valid_sha256_digest(&statement.dependency_lock_digest)
    {
        return Err(fail(
            ScientificEvidenceStage::StatementStructure,
            None,
            None,
            "source/workflow commit or source-tree/lockfile digest has an invalid format",
        ));
    }
    if statement.artifact_references.is_empty() {
        return Err(fail(
            ScientificEvidenceStage::StatementStructure,
            None,
            None,
            "evidence statement must reference at least one artifact",
        ));
    }
    let mut artifact_ids = BTreeSet::new();
    for reference in &statement.artifact_references {
        if !valid_id(&reference.artifact_id)
            || !valid_id(&reference.media_type)
            || !valid_sha256_digest(&reference.content_digest)
        {
            return Err(fail(
                ScientificEvidenceStage::StatementStructure,
                None,
                Some(&reference.content_digest),
                "artifact reference has an invalid ID, media type or SHA-256 digest",
            ));
        }
        if !artifact_ids.insert(reference.artifact_id.clone()) {
            return Err(fail(
                ScientificEvidenceStage::StatementStructure,
                None,
                Some(&reference.content_digest),
                "artifact ID is duplicated within a statement",
            ));
        }
    }
    let mut predecessor_ids = BTreeSet::new();
    for predecessor in &statement.predecessor_statement_digests {
        if !valid_sha256_digest(predecessor) || !predecessor_ids.insert(predecessor.clone()) {
            return Err(fail(
                ScientificEvidenceStage::StatementStructure,
                None,
                Some(predecessor),
                "predecessor digest is invalid or duplicated",
            ));
        }
    }
    if statement.declared_limitations.is_empty()
        || statement
            .declared_limitations
            .iter()
            .any(|limitation| !valid_id(limitation))
    {
        return Err(fail(
            ScientificEvidenceStage::StatementStructure,
            None,
            None,
            "statement must contain explicit, non-empty limitations",
        ));
    }
    Ok(())
}

fn visit_predecessors(
    statement_digest: &str,
    subject_id: &str,
    issued_at: u64,
    statements: &BTreeMap<String, &ScientificEvidenceStatement>,
    state: &mut BTreeMap<String, u8>,
) -> Result<(), ScientificEvidenceError> {
    match state.get(statement_digest).copied() {
        Some(1) => {
            return Err(fail(
                ScientificEvidenceStage::PredecessorGraph,
                Some(statement_digest),
                None,
                "predecessor graph contains a cycle",
            ));
        }
        Some(2) => return Ok(()),
        _ => {}
    }
    state.insert(statement_digest.to_string(), 1);
    let statement = statements.get(statement_digest).ok_or_else(|| {
        fail(
            ScientificEvidenceStage::PredecessorGraph,
            Some(statement_digest),
            None,
            "statement disappeared from the predecessor graph",
        )
    })?;
    for predecessor_digest in &statement.predecessor_statement_digests {
        let predecessor = statements.get(predecessor_digest).ok_or_else(|| {
            fail(
                ScientificEvidenceStage::PredecessorGraph,
                Some(statement_digest),
                Some(predecessor_digest),
                "predecessor statement is absent from the offline package",
            )
        })?;
        if predecessor.subject_id != subject_id || predecessor.issued_at_unix_seconds >= issued_at {
            return Err(fail(
                ScientificEvidenceStage::PredecessorGraph,
                Some(statement_digest),
                Some(predecessor_digest),
                "predecessor subject differs or predecessor is not strictly older",
            ));
        }
        visit_predecessors(
            predecessor_digest,
            &predecessor.subject_id,
            predecessor.issued_at_unix_seconds,
            statements,
            state,
        )?;
    }
    state.insert(statement_digest.to_string(), 2);
    Ok(())
}

fn validate_predecessor_graph(
    statements: &BTreeMap<String, &ScientificEvidenceStatement>,
) -> Result<(), ScientificEvidenceError> {
    let mut state = BTreeMap::new();
    for (digest, statement) in statements {
        visit_predecessors(
            digest,
            &statement.subject_id,
            statement.issued_at_unix_seconds,
            statements,
            &mut state,
        )?;
    }
    Ok(())
}

/// Verify a complete scientific evidence package offline.
///
/// A successful result establishes only the layers supplied by the providers:
/// exact canonical encoding, content digests, issuer signature and transparency
/// receipt proofs. It does not establish physical truth or model adequacy.
/// Production callers must supply independently tested cryptographic/COSE/VDS
/// providers; unit-test mocks are never suitable for production verification.
pub fn verify_scientific_evidence_package<E, D, S, T>(
    package: &ScientificEvidencePackage,
    encoder: &E,
    digests: &D,
    signatures: &S,
    transparency: &T,
) -> Result<ScientificEvidenceVerificationReceipt, ScientificEvidenceError>
where
    E: EvidencePayloadEncoder,
    D: EvidenceDigestProvider,
    S: IssuerSignatureVerifier,
    T: TransparencyReceiptVerifier,
{
    if package.statements.is_empty()
        || package.statements.len() > MAX_STATEMENTS
        || package.artifacts.len() > MAX_ARTIFACTS
    {
        return Err(fail(
            ScientificEvidenceStage::PackageEnvelope,
            None,
            None,
            "evidence package has an empty statement set or exceeds configured caps",
        ));
    }

    let mut artifact_store = BTreeMap::<String, &EvidenceArtifactBlob>::new();
    for artifact in &package.artifacts {
        if !valid_sha256_digest(&artifact.content_digest)
            || artifact_store
                .insert(artifact.content_digest.clone(), artifact)
                .is_some()
        {
            return Err(fail(
                ScientificEvidenceStage::PackageEnvelope,
                None,
                Some(&artifact.content_digest),
                "artifact digest is invalid or duplicated in the package",
            ));
        }
    }

    let mut statements = BTreeMap::<String, &ScientificEvidenceStatement>::new();
    let mut signed_by_digest =
        BTreeMap::<String, &SignedScientificEvidenceStatement>::new();
    for signed in &package.statements {
        validate_statement_structure(&signed.statement)?;
        if signed.signed_statement_bytes.is_empty()
            || !valid_id(&signed.signing_key_id)
            || !valid_id(&signed.signature_algorithm)
            || signed.transparency_receipts.is_empty()
            || signed.transparency_receipts.len() > MAX_RECEIPTS_PER_STATEMENT
        {
            return Err(fail(
                ScientificEvidenceStage::PackageEnvelope,
                None,
                None,
                "signed statement lacks signing metadata or required transparency receipts",
            ));
        }

        let canonical = encoder
            .encode_canonical_payload(&signed.statement)
            .map_err(|reason| {
                fail(
                    ScientificEvidenceStage::CanonicalEncoding,
                    None,
                    None,
                    format!("canonical payload encoder failed: {reason}"),
                )
            })?;
        if canonical != signed.canonical_payload {
            return Err(fail(
                ScientificEvidenceStage::CanonicalEncoding,
                None,
                None,
                "stored canonical payload differs from independently encoded statement",
            ));
        }
        let digest = digests.sha256(&canonical).map_err(|reason| {
            fail(
                ScientificEvidenceStage::StatementDigest,
                None,
                None,
                format!("statement digest provider failed: {reason}"),
            )
        })?;
        let statement_digest = format_sha256_digest(digest);
        if statements
            .insert(statement_digest.clone(), &signed.statement)
            .is_some()
        {
            return Err(fail(
                ScientificEvidenceStage::StatementDigest,
                Some(&statement_digest),
                None,
                "statement digest is duplicated in the package",
            ));
        }
        signed_by_digest.insert(statement_digest, signed);
    }

    validate_predecessor_graph(&statements)?;

    let mut referenced_artifacts = BTreeSet::new();
    for statement in statements.values() {
        for reference in &statement.artifact_references {
            referenced_artifacts.insert(reference.content_digest.clone());
            let artifact = artifact_store.get(&reference.content_digest).ok_or_else(|| {
                fail(
                    ScientificEvidenceStage::ArtifactDigest,
                    None,
                    Some(&reference.content_digest),
                    "referenced artifact bytes are absent from the offline package",
                )
            })?;
            let actual = digests.sha256(&artifact.bytes).map_err(|reason| {
                fail(
                    ScientificEvidenceStage::ArtifactDigest,
                    None,
                    Some(&reference.content_digest),
                    format!("artifact digest provider failed: {reason}"),
                )
            })?;
            if format_sha256_digest(actual) != reference.content_digest
                || u64::try_from(artifact.bytes.len()).ok() != Some(reference.byte_length)
            {
                return Err(fail(
                    ScientificEvidenceStage::ArtifactDigest,
                    None,
                    Some(&reference.content_digest),
                    "artifact content digest or byte length differs from its reference",
                ));
            }
        }
    }
    if referenced_artifacts.len() != artifact_store.len() {
        return Err(fail(
            ScientificEvidenceStage::FinalConsistency,
            None,
            None,
            "offline package contains unreferenced artifact blobs",
        ));
    }

    let mut verified_statements = Vec::with_capacity(package.statements.len());
    for (statement_digest, signed) in signed_by_digest {
        let identity = signatures
            .verify_signed_statement(&signed.signed_statement_bytes, &signed.canonical_payload)
            .map_err(|reason| {
                fail(
                    ScientificEvidenceStage::IssuerSignature,
                    Some(&statement_digest),
                    None,
                    format!("issuer signature verification failed: {reason}"),
                )
            })?;
        if identity.issuer_id != signed.statement.issuer_id
            || identity.key_id != signed.signing_key_id
            || identity.signature_algorithm != signed.signature_algorithm
        {
            return Err(fail(
                ScientificEvidenceStage::IssuerSignature,
                Some(&statement_digest),
                None,
                "verified signature identity does not match the statement issuer/key/algorithm",
            ));
        }

        let mut service_ids = BTreeSet::new();
        let mut verified_services = Vec::with_capacity(signed.transparency_receipts.len());
        for receipt in &signed.transparency_receipts {
            if !valid_id(&receipt.transparency_service_id)
                || !valid_id(&receipt.vds_profile)
                || receipt.cose_receipt_bytes.is_empty()
                || !service_ids.insert(receipt.transparency_service_id.clone())
            {
                return Err(fail(
                    ScientificEvidenceStage::TransparencyReceipt,
                    Some(&statement_digest),
                    None,
                    "transparency receipt identity is invalid, empty or duplicated",
                ));
            }
            let verified = transparency
                .verify_receipt(&statement_digest, receipt)
                .map_err(|reason| {
                    fail(
                        ScientificEvidenceStage::TransparencyReceipt,
                        Some(&statement_digest),
                        None,
                        format!("transparency receipt verification failed: {reason}"),
                    )
                })?;
            if verified.transparency_service_id != receipt.transparency_service_id
                || verified.vds_profile != receipt.vds_profile
                || verified.statement_digest != statement_digest
                || verified.tree_size == 0
                || verified.leaf_index >= verified.tree_size
                || !verified.receipt_signature_verified
                || !verified.inclusion_proof_verified
                || !verified.consistency_proof_verified
            {
                return Err(fail(
                    ScientificEvidenceStage::TransparencyReceipt,
                    Some(&statement_digest),
                    None,
                    "receipt proof is incomplete or does not bind the expected statement/service/VDS",
                ));
            }
            verified_services.push(receipt.transparency_service_id.clone());
        }
        verified_services.sort();
        verified_statements.push(VerifiedStatementReceipt {
            statement_digest,
            issuer_id: identity.issuer_id,
            subject_id: signed.statement.subject_id.clone(),
            verified_artifact_count: signed.statement.artifact_references.len(),
            verified_transparency_services: verified_services,
        });
    }

    verified_statements.sort_by(|left, right| left.statement_digest.cmp(&right.statement_digest));
    Ok(ScientificEvidenceVerificationReceipt {
        statement_count: verified_statements.len(),
        artifact_count: artifact_store.len(),
        verified_statements,
        all_canonical_payloads_match: true,
        all_artifact_digests_match: true,
        all_issuer_signatures_verified: true,
        all_transparency_receipts_verified: true,
        predecessor_graph_acyclic: true,
        no_unreferenced_artifacts: true,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        EvidenceArtifactBlob, EvidenceArtifactReference, EvidenceArtifactRole,
        EvidenceDigestProvider, EvidencePayloadEncoder, ScientificEvidencePackage,
        IssuerSignatureVerifier, ScientificEvidenceStatement,
        SignedScientificEvidenceStatement, TransparencyReceiptEnvelope,
        TransparencyReceiptVerifier, VerifiedIssuerIdentity,
        VerifiedTransparencyReceipt, format_sha256_digest,
        verify_scientific_evidence_package,
    };

    struct TestDigest;

    impl EvidenceDigestProvider for TestDigest {
        fn sha256(&self, bytes: &[u8]) -> Result<[u8; 32], String> {
            // Deterministic test double only. This is NOT a cryptographic hash
            // and must never be used outside these unit tests.
            let mut state = [0u8; 32];
            for (index, byte) in bytes.iter().enumerate() {
                let slot = index % 32;
                state[slot] = state[slot]
                    .wrapping_mul(31)
                    .wrapping_add(*byte)
                    .wrapping_add(index as u8);
            }
            Ok(state)
        }
    }

    struct TestEncoder;

    impl EvidencePayloadEncoder for TestEncoder {
        fn encode_canonical_payload(
            &self,
            statement: &ScientificEvidenceStatement,
        ) -> Result<Vec<u8>, String> {
            let mut bytes = Vec::new();
            for value in [
                &statement.schema_id,
                &statement.schema_version,
                &statement.issuer_id,
                &statement.subject_id,
                &statement.statement_type,
                &statement.source_repository,
                &statement.source_commit_sha,
                &statement.source_tree_digest,
                &statement.dependency_lock_digest,
                &statement.workflow_run_id,
                &statement.workflow_head_sha,
                &statement.verification_profile,
            ] {
                bytes.extend_from_slice(&(value.len() as u64).to_be_bytes());
                bytes.extend_from_slice(value.as_bytes());
            }
            bytes.extend_from_slice(&statement.issued_at_unix_seconds.to_be_bytes());
            for reference in &statement.artifact_references {
                bytes.extend_from_slice(reference.artifact_id.as_bytes());
                bytes.push(0);
                bytes.extend_from_slice(reference.content_digest.as_bytes());
                bytes.push(0);
                bytes.extend_from_slice(reference.media_type.as_bytes());
                bytes.push(reference.role as u8);
                bytes.extend_from_slice(&reference.byte_length.to_be_bytes());
            }
            for predecessor in &statement.predecessor_statement_digests {
                bytes.extend_from_slice(predecessor.as_bytes());
            }
            for limitation in &statement.declared_limitations {
                bytes.extend_from_slice(limitation.as_bytes());
            }
            Ok(bytes)
        }
    }

    struct TestSignatureVerifier {
        issuer_id: String,
        key_id: String,
        algorithm: String,
    }

    impl IssuerSignatureVerifier for TestSignatureVerifier {
        fn verify_signed_statement(
            &self,
            signed_statement_bytes: &[u8],
            _canonical_payload: &[u8],
        ) -> Result<VerifiedIssuerIdentity, String> {
            if signed_statement_bytes != b"mock-signature-valid" {
                return Err("mock signature does not match".into());
            }
            Ok(VerifiedIssuerIdentity {
                issuer_id: self.issuer_id.clone(),
                key_id: self.key_id.clone(),
                signature_algorithm: self.algorithm.clone(),
            })
        }
    }

    struct TestTransparencyVerifier;

    impl TransparencyReceiptVerifier for TestTransparencyVerifier {
        fn verify_receipt(
            &self,
            expected_statement_digest: &str,
            receipt: &TransparencyReceiptEnvelope,
        ) -> Result<VerifiedTransparencyReceipt, String> {
            if receipt.cose_receipt_bytes != b"mock-receipt-valid" {
                return Err("mock receipt does not match".into());
            }
            Ok(VerifiedTransparencyReceipt {
                transparency_service_id: receipt.transparency_service_id.clone(),
                vds_profile: receipt.vds_profile.clone(),
                statement_digest: expected_statement_digest.into(),
                leaf_index: 3,
                tree_size: 8,
                receipt_signature_verified: true,
                inclusion_proof_verified: true,
                consistency_proof_verified: true,
            })
        }
    }

    fn artifact(bytes: &[u8]) -> EvidenceArtifactBlob {
        EvidenceArtifactBlob {
            content_digest: format_sha256_digest(TestDigest.sha256(bytes).unwrap()),
            bytes: bytes.to_vec(),
        }
    }

    fn statement(
        subject: &str,
        time: u64,
        artifact_blob: &EvidenceArtifactBlob,
        predecessors: Vec<String>,
    ) -> ScientificEvidenceStatement {
        ScientificEvidenceStatement {
            schema_id: "urn:luminous:evidence".into(),
            schema_version: "1".into(),
            issuer_id: "lab-issuer".into(),
            subject_id: subject.into(),
            statement_type: "scientific-result".into(),
            issued_at_unix_seconds: time,
            source_repository: "Luminous-Dynamics/symtropy".into(),
            source_commit_sha: "0123456789abcdef0123456789abcdef01234567".into(),
            source_tree_digest: format_sha256_digest(TestDigest.sha256(b"tree").unwrap()),
            dependency_lock_digest: format_sha256_digest(TestDigest.sha256(b"lock").unwrap()),
            workflow_run_id: "workflow-123".into(),
            workflow_head_sha: "0123456789abcdef0123456789abcdef01234567".into(),
            verification_profile: "scitt-scientific-evidence-v1".into(),
            artifact_references: vec![EvidenceArtifactReference {
                artifact_id: "primary-result".into(),
                content_digest: artifact_blob.content_digest.clone(),
                media_type: "application/octet-stream".into(),
                role: EvidenceArtifactRole::BenchmarkTrial,
                byte_length: artifact_blob.bytes.len() as u64,
            }],
            predecessor_statement_digests: predecessors,
            declared_limitations: vec!["simulation-only evidence".into()],
        }
    }

    fn signed(statement: ScientificEvidenceStatement) -> SignedScientificEvidenceStatement {
        let canonical_payload = TestEncoder.encode_canonical_payload(&statement).unwrap();
        SignedScientificEvidenceStatement {
            statement,
            canonical_payload,
            signed_statement_bytes: b"mock-signature-valid".to_vec(),
            signing_key_id: "issuer-key-1".into(),
            signature_algorithm: "ES256".into(),
            transparency_receipts: vec![TransparencyReceiptEnvelope {
                transparency_service_id: "transparency-a".into(),
                vds_profile: "RFC9162_SHA256".into(),
                cose_receipt_bytes: b"mock-receipt-valid".to_vec(),
            }],
        }
    }

    fn package() -> ScientificEvidencePackage {
        let blob = artifact(b"raw-result");
        let first = signed(statement("campaign-1", 100, &blob, Vec::new()));
        let first_payload = TestEncoder.encode_canonical_payload(&first.statement).unwrap();
        let first_id = format_sha256_digest(TestDigest.sha256(&first_payload).unwrap());
        let second = signed(statement("campaign-1", 200, &blob, vec![first_id]));
        ScientificEvidencePackage {
            statements: vec![first, second],
            artifacts: vec![blob],
        }
    }

    fn signature_verifier() -> TestSignatureVerifier {
        TestSignatureVerifier {
            issuer_id: "lab-issuer".into(),
            key_id: "issuer-key-1".into(),
            algorithm: "ES256".into(),
        }
    }

    #[test]
    fn accepts_content_addressed_predecessor_chain_with_receipts_from_independent_services() {
        let mut package = package();
        package.statements[0].transparency_receipts.push(TransparencyReceiptEnvelope {
            transparency_service_id: "transparency-b".into(),
            vds_profile: "RFC9162_SHA256".into(),
            cose_receipt_bytes: b"mock-receipt-valid".to_vec(),
        });
        let receipt = verify_scientific_evidence_package(
            &package,
            &TestEncoder,
            &TestDigest,
            &signature_verifier(),
            &TestTransparencyVerifier,
        )
        .unwrap();
        assert_eq!(receipt.statement_count, 2);
        assert_eq!(receipt.artifact_count, 1);
        assert!(receipt.all_issuer_signatures_verified);
        assert!(receipt.all_transparency_receipts_verified);
        assert_eq!(receipt.verified_statements[0].verified_transparency_services.len(), 2);
    }

    #[test]
    fn rejects_canonical_payload_mutation_and_artifact_content_tampering() {
        let mut changed = package();
        changed.statements[0].canonical_payload.push(0xff);
        assert!(verify_scientific_evidence_package(
            &changed,
            &TestEncoder,
            &TestDigest,
            &signature_verifier(),
            &TestTransparencyVerifier,
        )
        .is_err());

        let mut changed = package();
        changed.artifacts[0].bytes.push(0xff);
        assert!(verify_scientific_evidence_package(
            &changed,
            &TestEncoder,
            &TestDigest,
            &signature_verifier(),
            &TestTransparencyVerifier,
        )
        .is_err());
    }

    #[test]
    fn rejects_missing_predecessor_and_wrong_subject_or_time() {
        let mut changed = package();
        changed.statements[1].statement.predecessor_statement_digests[0] =
            format_sha256_digest(TestDigest.sha256(b"missing").unwrap());
        changed.statements[1].canonical_payload =
            TestEncoder.encode_canonical_payload(&changed.statements[1].statement).unwrap();
        assert!(verify_scientific_evidence_package(
            &changed,
            &TestEncoder,
            &TestDigest,
            &signature_verifier(),
            &TestTransparencyVerifier,
        )
        .is_err());

        let mut changed = package();
        changed.statements[1].statement.subject_id = "other-campaign".into();
        changed.statements[1].canonical_payload =
            TestEncoder.encode_canonical_payload(&changed.statements[1].statement).unwrap();
        assert!(verify_scientific_evidence_package(
            &changed,
            &TestEncoder,
            &TestDigest,
            &signature_verifier(),
            &TestTransparencyVerifier,
        )
        .is_err());
    }

    #[test]
    fn rejects_wrong_issuer_identity_and_incomplete_transparency_proofs() {
        let valid = package();
        let wrong_signer = TestSignatureVerifier {
            issuer_id: "other-issuer".into(),
            ..signature_verifier()
        };
        assert!(verify_scientific_evidence_package(
            &valid,
            &TestEncoder,
            &TestDigest,
            &wrong_signer,
            &TestTransparencyVerifier,
        )
        .is_err());

        struct IncompleteTransparency;
        impl TransparencyReceiptVerifier for IncompleteTransparency {
            fn verify_receipt(
                &self,
                expected_statement_digest: &str,
                receipt: &TransparencyReceiptEnvelope,
            ) -> Result<VerifiedTransparencyReceipt, String> {
                Ok(VerifiedTransparencyReceipt {
                    transparency_service_id: receipt.transparency_service_id.clone(),
                    vds_profile: receipt.vds_profile.clone(),
                    statement_digest: expected_statement_digest.into(),
                    leaf_index: 4,
                    tree_size: 4,
                    receipt_signature_verified: true,
                    inclusion_proof_verified: true,
                    consistency_proof_verified: false,
                })
            }
        }
        assert!(verify_scientific_evidence_package(
            &valid,
            &TestEncoder,
            &TestDigest,
            &signature_verifier(),
            &IncompleteTransparency,
        )
        .is_err());
    }

    #[test]
    fn rejects_unreferenced_artifacts_duplicate_receipt_services_and_empty_limitations() {
        let mut changed = package();
        changed.artifacts.push(artifact(b"unreferenced"));
        assert!(verify_scientific_evidence_package(
            &changed,
            &TestEncoder,
            &TestDigest,
            &signature_verifier(),
            &TestTransparencyVerifier,
        )
        .is_err());

        let mut changed = package();
        changed.statements[0].transparency_receipts.push(
            changed.statements[0].transparency_receipts[0].clone()
        );
        assert!(verify_scientific_evidence_package(
            &changed,
            &TestEncoder,
            &TestDigest,
            &signature_verifier(),
            &TestTransparencyVerifier,
        )
        .is_err());

        let mut changed = package();
        changed.statements[0].statement.declared_limitations.clear();
        changed.statements[0].canonical_payload =
            TestEncoder.encode_canonical_payload(&changed.statements[0].statement).unwrap();
        assert!(verify_scientific_evidence_package(
            &changed,
            &TestEncoder,
            &TestDigest,
            &signature_verifier(),
            &TestTransparencyVerifier,
        )
        .is_err());
    }
}
