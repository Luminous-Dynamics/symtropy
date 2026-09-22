// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Planet-neutral scenario-template to world-genesis semantics.
//!
//! This crate deliberately owns only the semantic boundary between reusable
//! scenario content and one newly admitted world-lineage occurrence. It does
//! not define production planet, time, scope, persistence, player-spawn, or
//! gameplay-mode authority.

use sha2::{Digest, Sha256};
use std::{error::Error, fmt};

pub const WORLD_START_SCHEMA_V1: u32 = 1;
const MAX_ID_BYTES: usize = 96;
const MAX_DISPLAY_BYTES: usize = 256;
const MAX_PARAMETER_VALUE_BYTES: usize = 16 * 1024;
const MAX_COLLECTION_ITEMS: usize = 1024;

const TEMPLATE_DOMAIN: &[u8] = b"symtropy.world-start.scenario-template.v1";
const REQUEST_DOMAIN: &[u8] = b"symtropy.world-start.genesis-request.v1";
const LINEAGE_DOMAIN: &[u8] = b"symtropy.world-start.lineage-ref.v1";
const CANDIDATE_DOMAIN: &[u8] = b"symtropy.world-start.genesis-candidate.v1";
const RECEIPT_DOMAIN: &[u8] = b"symtropy.world-start.receipt.v1";

/// Portable semantic identifier used only by the WORLD-START V1 boundary.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SemanticId(String);

impl SemanticId {
    pub fn parse(value: impl Into<String>) -> Result<Self, WorldStartError> {
        let value = value.into();
        let valid = !value.is_empty()
            && value.len() <= MAX_ID_BYTES
            && value.bytes().all(|byte| {
                byte.is_ascii_alphanumeric()
                    || matches!(byte, b'.' | b'-' | b'_' | b':' | b'/')
            });
        if valid {
            Ok(Self(value))
        } else {
            Err(WorldStartError::InvalidSemanticId(value))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SemanticId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// Serializer-independent SHA-256 semantic commitment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CanonicalDigest([u8; 32]);

impl CanonicalDigest {
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Display for CanonicalDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

/// Opaque bridge to one exact predecessor authority owned by another domain.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TypedAuthorityRefV1 {
    pub domain: SemanticId,
    pub schema_version: u32,
    pub profile: SemanticId,
    pub digest: CanonicalDigest,
}

impl TypedAuthorityRefV1 {
    pub fn new(
        domain: SemanticId,
        schema_version: u32,
        profile: SemanticId,
        digest: CanonicalDigest,
    ) -> Result<Self, WorldStartError> {
        if schema_version == 0 {
            return Err(WorldStartError::ZeroSchemaVersion);
        }
        Ok(Self {
            domain,
            schema_version,
            profile,
            digest,
        })
    }

    fn encode(&self, writer: &mut CanonicalWriter) -> Result<(), WorldStartError> {
        writer.push_str(self.domain.as_str())?;
        writer.push_u32(self.schema_version);
        writer.push_str(self.profile.as_str())?;
        writer.push_digest(self.digest);
        Ok(())
    }
}

/// Explicit provenance class. Tags are stable semantic values, not Rust enum
/// discriminants inferred by a serializer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ProvenanceClass {
    /// Known/current fact under the selected scenario contract.
    K,
    /// Deterministically generated conditional detail.
    D,
    /// Measured or assimilated evidence.
    M,
    /// Explicit authored assumption / fictional premise.
    A,
}

impl ProvenanceClass {
    const fn tag(self) -> u8 {
        match self {
            Self::K => 0,
            Self::D => 1,
            Self::M => 2,
            Self::A => 3,
        }
    }
}

/// Provenance binding for one semantic claim. A claim id may occur at most once
/// in a template, preventing ambiguous K/D/M/A relabeling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvenanceBindingV1 {
    pub claim_id: SemanticId,
    pub class: ProvenanceClass,
    pub source: TypedAuthorityRefV1,
}

impl ProvenanceBindingV1 {
    fn encode(&self, writer: &mut CanonicalWriter) -> Result<(), WorldStartError> {
        writer.push_str(self.claim_id.as_str())?;
        writer.push_u8(self.class.tag());
        self.source.encode(writer)
    }
}

/// Small semantic parameter used for exact template/request inputs whose owning
/// domain has not yet received a typed adapter in WORLD-START.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticParameterV1 {
    pub key: SemanticId,
    pub value: Vec<u8>,
}

impl SemanticParameterV1 {
    pub fn new(key: SemanticId, value: Vec<u8>) -> Result<Self, WorldStartError> {
        if value.len() > MAX_PARAMETER_VALUE_BYTES {
            return Err(WorldStartError::ParameterValueTooLarge {
                key,
                bytes: value.len(),
            });
        }
        Ok(Self { key, value })
    }

    fn encode(&self, writer: &mut CanonicalWriter) -> Result<(), WorldStartError> {
        writer.push_str(self.key.as_str())?;
        writer.push_bytes(&self.value)
    }
}

/// Reusable scenario content/policy. `display_name` is intentionally excluded
/// from semantic identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenarioTemplateV1 {
    pub template_id: SemanticId,
    pub genesis_profile: SemanticId,
    pub display_name: Option<String>,
    predecessors: Vec<TypedAuthorityRefV1>,
    provenance: Vec<ProvenanceBindingV1>,
    parameters: Vec<SemanticParameterV1>,
    digest: CanonicalDigest,
}

impl ScenarioTemplateV1 {
    pub fn new(
        template_id: SemanticId,
        genesis_profile: SemanticId,
        display_name: Option<String>,
        predecessors: Vec<TypedAuthorityRefV1>,
        provenance: Vec<ProvenanceBindingV1>,
        parameters: Vec<SemanticParameterV1>,
    ) -> Result<Self, WorldStartError> {
        if display_name
            .as_ref()
            .is_some_and(|name| name.len() > MAX_DISPLAY_BYTES)
        {
            return Err(WorldStartError::DisplayNameTooLarge);
        }
        check_collection_bound("predecessors", predecessors.len())?;
        check_collection_bound("provenance", provenance.len())?;
        check_collection_bound("parameters", parameters.len())?;

        let mut predecessors = predecessors;
        predecessors.sort();
        if predecessors.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(WorldStartError::DuplicatePredecessor);
        }

        let mut provenance = provenance;
        provenance.sort_by(|left, right| left.claim_id.cmp(&right.claim_id));
        if let Some(pair) = provenance
            .windows(2)
            .find(|pair| pair[0].claim_id == pair[1].claim_id)
        {
            return Err(WorldStartError::DuplicateProvenanceClaim(
                pair[0].claim_id.clone(),
            ));
        }

        let mut parameters = parameters;
        parameters.sort_by(|left, right| left.key.cmp(&right.key));
        if let Some(pair) = parameters
            .windows(2)
            .find(|pair| pair[0].key == pair[1].key)
        {
            return Err(WorldStartError::DuplicateParameter(pair[0].key.clone()));
        }

        let digest = template_digest(
            &template_id,
            &genesis_profile,
            &predecessors,
            &provenance,
            &parameters,
        )?;

        Ok(Self {
            template_id,
            genesis_profile,
            display_name,
            predecessors,
            provenance,
            parameters,
            digest,
        })
    }

    pub const fn semantic_digest(&self) -> CanonicalDigest {
        self.digest
    }

    pub fn predecessors(&self) -> &[TypedAuthorityRefV1] {
        &self.predecessors
    }

    pub fn provenance(&self) -> &[ProvenanceBindingV1] {
        &self.provenance
    }

    pub fn parameters(&self) -> &[SemanticParameterV1] {
        &self.parameters
    }
}

/// Explicit request intent. WORLD-START-00A only admits `NewWorld`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenesisIntentV1 {
    NewWorld,
    ResumeExisting,
}

impl GenesisIntentV1 {
    const fn tag(self) -> u8 {
        match self {
            Self::NewWorld => 0,
            Self::ResumeExisting => 1,
        }
    }
}

/// One attempt to create a fresh lineage occurrence from an exact template.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenesisRequestV1 {
    pub request_id: SemanticId,
    pub template_digest: CanonicalDigest,
    pub intent: GenesisIntentV1,
    parameters: Vec<SemanticParameterV1>,
    digest: CanonicalDigest,
}

impl GenesisRequestV1 {
    pub fn new(
        request_id: SemanticId,
        template_digest: CanonicalDigest,
        intent: GenesisIntentV1,
        parameters: Vec<SemanticParameterV1>,
    ) -> Result<Self, WorldStartError> {
        check_collection_bound("request parameters", parameters.len())?;
        let mut parameters = parameters;
        parameters.sort_by(|left, right| left.key.cmp(&right.key));
        if let Some(pair) = parameters
            .windows(2)
            .find(|pair| pair[0].key == pair[1].key)
        {
            return Err(WorldStartError::DuplicateParameter(pair[0].key.clone()));
        }
        let digest = request_digest(&request_id, template_digest, intent, &parameters)?;
        Ok(Self {
            request_id,
            template_digest,
            intent,
            parameters,
            digest,
        })
    }

    pub const fn semantic_digest(&self) -> CanonicalDigest {
        self.digest
    }

    pub fn parameters(&self) -> &[SemanticParameterV1] {
        &self.parameters
    }
}

/// WORLD-START-local lineage occurrence identity. It is deliberately not a
/// production `WorldInstanceId`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WorldLineageRefV1(CanonicalDigest);

impl WorldLineageRefV1 {
    pub const fn digest(self) -> CanonicalDigest {
        self.0
    }
}

/// Pure, complete candidate assembled before admission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenarioGenesisCandidateV1 {
    pub template_digest: CanonicalDigest,
    pub request_digest: CanonicalDigest,
    pub lineage: WorldLineageRefV1,
    pub candidate_digest: CanonicalDigest,
}

/// Immutable successful admission evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldStartReceiptV1 {
    pub schema_version: u32,
    pub template_digest: CanonicalDigest,
    pub request_digest: CanonicalDigest,
    pub lineage: WorldLineageRefV1,
    pub candidate_digest: CanonicalDigest,
    pub receipt_digest: CanonicalDigest,
}

impl WorldStartReceiptV1 {
    /// Recompute the complete admission result under the supplied current
    /// template/request and reject any changed predecessor or request context.
    pub fn revalidate(
        &self,
        template: &ScenarioTemplateV1,
        request: &GenesisRequestV1,
    ) -> Result<(), WorldStartError> {
        let expected = admit_new_world(template, request)?;
        if &expected == self {
            Ok(())
        } else {
            Err(WorldStartError::ReceiptMismatch)
        }
    }
}

/// Construct one complete candidate without publishing or mutating external
/// state.
pub fn build_genesis_candidate(
    template: &ScenarioTemplateV1,
    request: &GenesisRequestV1,
) -> Result<ScenarioGenesisCandidateV1, WorldStartError> {
    if request.intent != GenesisIntentV1::NewWorld {
        return Err(WorldStartError::ResumeIntentRejected);
    }
    if request.template_digest != template.semantic_digest() {
        return Err(WorldStartError::TemplateDigestMismatch {
            expected: template.semantic_digest(),
            supplied: request.template_digest,
        });
    }

    let lineage = WorldLineageRefV1(hash_domain(LINEAGE_DOMAIN, |writer| {
        writer.push_digest(template.semantic_digest());
        writer.push_digest(request.semantic_digest());
        Ok(())
    })?);

    let candidate_digest = hash_domain(CANDIDATE_DOMAIN, |writer| {
        writer.push_u32(WORLD_START_SCHEMA_V1);
        writer.push_digest(template.semantic_digest());
        writer.push_digest(request.semantic_digest());
        writer.push_digest(lineage.digest());
        Ok(())
    })?;

    Ok(ScenarioGenesisCandidateV1 {
        template_digest: template.semantic_digest(),
        request_digest: request.semantic_digest(),
        lineage,
        candidate_digest,
    })
}

/// Admit a pure candidate and produce immutable receipt evidence. Because 00A
/// owns no global world registry, exact retries are deterministic recomputation;
/// a later owner may add durable exactly-once publication around this theorem.
pub fn admit_new_world(
    template: &ScenarioTemplateV1,
    request: &GenesisRequestV1,
) -> Result<WorldStartReceiptV1, WorldStartError> {
    let candidate = build_genesis_candidate(template, request)?;
    let receipt_digest = hash_domain(RECEIPT_DOMAIN, |writer| {
        writer.push_u32(WORLD_START_SCHEMA_V1);
        writer.push_digest(candidate.template_digest);
        writer.push_digest(candidate.request_digest);
        writer.push_digest(candidate.lineage.digest());
        writer.push_digest(candidate.candidate_digest);
        Ok(())
    })?;

    Ok(WorldStartReceiptV1 {
        schema_version: WORLD_START_SCHEMA_V1,
        template_digest: candidate.template_digest,
        request_digest: candidate.request_digest,
        lineage: candidate.lineage,
        candidate_digest: candidate.candidate_digest,
        receipt_digest,
    })
}

fn template_digest(
    template_id: &SemanticId,
    genesis_profile: &SemanticId,
    predecessors: &[TypedAuthorityRefV1],
    provenance: &[ProvenanceBindingV1],
    parameters: &[SemanticParameterV1],
) -> Result<CanonicalDigest, WorldStartError> {
    hash_domain(TEMPLATE_DOMAIN, |writer| {
        writer.push_u32(WORLD_START_SCHEMA_V1);
        writer.push_str(template_id.as_str())?;
        writer.push_str(genesis_profile.as_str())?;
        writer.push_count(predecessors.len())?;
        for predecessor in predecessors {
            predecessor.encode(writer)?;
        }
        writer.push_count(provenance.len())?;
        for binding in provenance {
            binding.encode(writer)?;
        }
        writer.push_count(parameters.len())?;
        for parameter in parameters {
            parameter.encode(writer)?;
        }
        Ok(())
    })
}

fn request_digest(
    request_id: &SemanticId,
    template_digest: CanonicalDigest,
    intent: GenesisIntentV1,
    parameters: &[SemanticParameterV1],
) -> Result<CanonicalDigest, WorldStartError> {
    hash_domain(REQUEST_DOMAIN, |writer| {
        writer.push_u32(WORLD_START_SCHEMA_V1);
        writer.push_str(request_id.as_str())?;
        writer.push_digest(template_digest);
        writer.push_u8(intent.tag());
        writer.push_count(parameters.len())?;
        for parameter in parameters {
            parameter.encode(writer)?;
        }
        Ok(())
    })
}

fn hash_domain(
    domain: &[u8],
    encode: impl FnOnce(&mut CanonicalWriter) -> Result<(), WorldStartError>,
) -> Result<CanonicalDigest, WorldStartError> {
    let mut writer = CanonicalWriter::new(domain)?;
    encode(&mut writer)?;
    let digest = Sha256::digest(writer.finish());
    let mut bytes = [0_u8; 32];
    bytes.copy_from_slice(&digest);
    Ok(CanonicalDigest(bytes))
}

struct CanonicalWriter {
    bytes: Vec<u8>,
}

impl CanonicalWriter {
    fn new(domain: &[u8]) -> Result<Self, WorldStartError> {
        let mut writer = Self { bytes: Vec::new() };
        writer.push_bytes(domain)?;
        Ok(writer)
    }

    fn push_u8(&mut self, value: u8) {
        self.bytes.push(value);
    }

    fn push_u32(&mut self, value: u32) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    fn push_count(&mut self, count: usize) -> Result<(), WorldStartError> {
        let count = u32::try_from(count).map_err(|_| WorldStartError::LengthOverflow)?;
        self.push_u32(count);
        Ok(())
    }

    fn push_str(&mut self, value: &str) -> Result<(), WorldStartError> {
        self.push_bytes(value.as_bytes())
    }

    fn push_bytes(&mut self, value: &[u8]) -> Result<(), WorldStartError> {
        let length = u32::try_from(value.len()).map_err(|_| WorldStartError::LengthOverflow)?;
        self.push_u32(length);
        self.bytes.extend_from_slice(value);
        Ok(())
    }

    fn push_digest(&mut self, digest: CanonicalDigest) {
        self.bytes.extend_from_slice(digest.as_bytes());
    }

    fn finish(self) -> Vec<u8> {
        self.bytes
    }
}

fn check_collection_bound(name: &'static str, count: usize) -> Result<(), WorldStartError> {
    if count > MAX_COLLECTION_ITEMS {
        Err(WorldStartError::CollectionTooLarge { name, count })
    } else {
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorldStartError {
    InvalidSemanticId(String),
    ZeroSchemaVersion,
    DisplayNameTooLarge,
    ParameterValueTooLarge { key: SemanticId, bytes: usize },
    CollectionTooLarge { name: &'static str, count: usize },
    DuplicatePredecessor,
    DuplicateProvenanceClaim(SemanticId),
    DuplicateParameter(SemanticId),
    LengthOverflow,
    ResumeIntentRejected,
    TemplateDigestMismatch {
        expected: CanonicalDigest,
        supplied: CanonicalDigest,
    },
    ReceiptMismatch,
}

impl fmt::Display for WorldStartError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSemanticId(value) => write!(formatter, "invalid semantic id {value:?}"),
            Self::ZeroSchemaVersion => {
                formatter.write_str("authority schema version must be non-zero")
            }
            Self::DisplayNameTooLarge => {
                formatter.write_str("display name exceeds WORLD-START V1 bound")
            }
            Self::ParameterValueTooLarge { key, bytes } => write!(
                formatter,
                "semantic parameter {key} exceeds WORLD-START V1 byte bound ({bytes} bytes)"
            ),
            Self::CollectionTooLarge { name, count } => write!(
                formatter,
                "{name} contains {count} items, exceeding WORLD-START V1 bound"
            ),
            Self::DuplicatePredecessor => {
                formatter.write_str("duplicate exact predecessor authority")
            }
            Self::DuplicateProvenanceClaim(id) => {
                write!(formatter, "duplicate provenance claim {id}")
            }
            Self::DuplicateParameter(id) => {
                write!(formatter, "duplicate semantic parameter {id}")
            }
            Self::LengthOverflow => {
                formatter.write_str("canonical byte length exceeds u32 representation")
            }
            Self::ResumeIntentRejected => formatter
                .write_str("resume-existing intent is not a new-world genesis request"),
            Self::TemplateDigestMismatch { .. } => formatter
                .write_str("genesis request does not bind the supplied exact template"),
            Self::ReceiptMismatch => formatter
                .write_str("world-start receipt does not match the supplied current template/request"),
        }
    }
}

impl Error for WorldStartError {}
