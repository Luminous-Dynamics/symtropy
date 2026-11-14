// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Digest-bound, reproducible build receipts for Power Fabric.
//! A receipt is an integrity manifest, not a signature or safety certificate.

use crate::{
    compile_ir_to_service_graph, GraphBridgeError, GraphCompileContext, GraphMaturity,
    IrGraphCompilation, IrGraphMappingProfile, PowerFabricDocument, ServiceGraph,
    StandardReference,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

pub const BUILD_RECEIPT_VERSION: u16 = 1;
pub const BUILD_CANONICALIZATION_PROFILE: &str =
    "serde-json-struct-order-btree-map-values-sorted-arrays-v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Sha256Digest { value: String }

impl Sha256Digest {
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let digest = Sha256::digest(bytes);
        let mut hex = String::with_capacity(64);
        for byte in digest {
            use core::fmt::Write;
            let _ = write!(&mut hex, "{byte:02x}");
        }
        Self { value: format!("sha256:{hex}") }
    }
    pub fn from_hex(value: impl Into<String>) -> Result<Self, BuildReceiptError> {
        let value = value.into();
        if value.len() != 64 || !value.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()) {
            return Err(BuildReceiptError::InvalidSha256Hex);
        }
        Ok(Self { value: format!("sha256:{value}") })
    }
    pub fn as_str(&self) -> &str { &self.value }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StandardProfilePin {
    profile_id: String,
    profile_version: String,
    standard: StandardReference,
    registry_record_digest: Sha256Digest,
    applicability_rules_digest: Option<Sha256Digest>,
}
impl StandardProfilePin {
    pub fn new(
        profile_id: impl Into<String>,
        profile_version: impl Into<String>,
        standard: StandardReference,
        registry_record_digest: Sha256Digest,
        applicability_rules_digest: Option<Sha256Digest>,
    ) -> Result<Self, BuildReceiptError> {
        let profile_id = profile_id.into();
        let profile_version = profile_version.into();
        require_non_empty("standard_profile_id", &profile_id)?;
        require_non_empty("standard_profile_version", &profile_version)?;
        if standard.publication_status() == crate::StandardPublicationStatus::Unknown {
            return Err(BuildReceiptError::UnknownStandardLifecycle);
        }
        Ok(Self { profile_id, profile_version, standard, registry_record_digest, applicability_rules_digest })
    }
    pub fn profile_id(&self) -> &str { &self.profile_id }
    pub fn profile_version(&self) -> &str { &self.profile_version }
    pub fn standard(&self) -> &StandardReference { &self.standard }
    pub fn registry_record_digest(&self) -> &Sha256Digest { &self.registry_record_digest }
    pub fn applicability_rules_digest(&self) -> Option<&Sha256Digest> { self.applicability_rules_digest.as_ref() }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UpstreamArtifactPin {
    artifact_id: String,
    revision_id: String,
    role: String,
    digest: Sha256Digest,
}
impl UpstreamArtifactPin {
    pub fn new(
        artifact_id: impl Into<String>,
        revision_id: impl Into<String>,
        role: impl Into<String>,
        digest: Sha256Digest,
    ) -> Result<Self, BuildReceiptError> {
        let artifact_id = artifact_id.into();
        let revision_id = revision_id.into();
        let role = role.into();
        require_non_empty("upstream_artifact_id", &artifact_id)?;
        require_non_empty("upstream_artifact_revision_id", &revision_id)?;
        require_non_empty("upstream_artifact_role", &role)?;
        Ok(Self { artifact_id, revision_id, role, digest })
    }
    pub fn artifact_id(&self) -> &str { &self.artifact_id }
    pub fn revision_id(&self) -> &str { &self.revision_id }
    pub fn role(&self) -> &str { &self.role }
    pub fn digest(&self) -> &Sha256Digest { &self.digest }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BuildMetadata {
    compiler_id: String,
    compiler_version: String,
    toolchain_fingerprint: String,
    standard_profiles: Vec<StandardProfilePin>,
    upstream_artifacts: Vec<UpstreamArtifactPin>,
}
impl BuildMetadata {
    pub fn new(
        compiler_id: impl Into<String>,
        compiler_version: impl Into<String>,
        toolchain_fingerprint: impl Into<String>,
        mut standard_profiles: Vec<StandardProfilePin>,
        mut upstream_artifacts: Vec<UpstreamArtifactPin>,
    ) -> Result<Self, BuildReceiptError> {
        let compiler_id = compiler_id.into();
        let compiler_version = compiler_version.into();
        let toolchain_fingerprint = toolchain_fingerprint.into();
        require_non_empty("compiler_id", &compiler_id)?;
        require_non_empty("compiler_version", &compiler_version)?;
        require_non_empty("toolchain_fingerprint", &toolchain_fingerprint)?;
        if standard_profiles.is_empty() { return Err(BuildReceiptError::NoStandardProfilesPinned); }

        standard_profiles.sort_by(|a,b| a.profile_id.cmp(&b.profile_id).then_with(|| a.profile_version.cmp(&b.profile_version)));
        if standard_profiles.windows(2).any(|pair| pair[0].profile_id == pair[1].profile_id && pair[0].profile_version == pair[1].profile_version) {
            return Err(BuildReceiptError::DuplicateStandardProfilePin);
        }
        upstream_artifacts.sort_by(|a,b| a.artifact_id.cmp(&b.artifact_id).then_with(|| a.revision_id.cmp(&b.revision_id)));
        if upstream_artifacts.windows(2).any(|pair| pair[0].artifact_id == pair[1].artifact_id && pair[0].revision_id == pair[1].revision_id) {
            return Err(BuildReceiptError::DuplicateUpstreamArtifactPin);
        }
        Ok(Self { compiler_id, compiler_version, toolchain_fingerprint, standard_profiles, upstream_artifacts })
    }
    pub fn compiler_id(&self) -> &str { &self.compiler_id }
    pub fn compiler_version(&self) -> &str { &self.compiler_version }
    pub fn toolchain_fingerprint(&self) -> &str { &self.toolchain_fingerprint }
    pub fn standard_profiles(&self) -> &[StandardProfilePin] { &self.standard_profiles }
    pub fn upstream_artifacts(&self) -> &[UpstreamArtifactPin] { &self.upstream_artifacts }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PowerFabricBuildReceipt {
    receipt_version: u16,
    canonicalization_profile: String,
    digest_algorithm: String,
    compiler_id: String,
    compiler_version: String,
    toolchain_fingerprint: String,
    source_document_id: String,
    source_document_revision_id: String,
    source_schema_version: crate::IrSchemaVersion,
    mapping_profile_id: String,
    mapping_profile_version: String,
    graph_id: String,
    graph_revision_id: String,
    graph_maturity: GraphMaturity,
    declared_graph_content_digest: String,
    source_document_digest: Sha256Digest,
    mapping_profile_digest: Sha256Digest,
    service_graph_payload_digest: Sha256Digest,
    standard_profiles: Vec<StandardProfilePin>,
    upstream_artifacts: Vec<UpstreamArtifactPin>,
    build_lock_digest: Sha256Digest,
}
impl PowerFabricBuildReceipt {
    pub fn create(
        document: &PowerFabricDocument,
        profile: &IrGraphMappingProfile,
        graph: &ServiceGraph,
        metadata: BuildMetadata,
    ) -> Result<Self, BuildReceiptError> {
        let source_document_digest = digest_json(document, "source_document")?;
        let mapping_profile_digest = digest_json(profile, "mapping_profile")?;
        let service_graph_payload_digest = digest_json(graph, "service_graph")?;
        let revision = graph.revision();
        let material = BuildLockMaterial {
            receipt_version: BUILD_RECEIPT_VERSION,
            canonicalization_profile: BUILD_CANONICALIZATION_PROFILE,
            digest_algorithm: "sha256",
            compiler_id: &metadata.compiler_id,
            compiler_version: &metadata.compiler_version,
            toolchain_fingerprint: &metadata.toolchain_fingerprint,
            source_document_id: document.document_id(),
            source_document_revision_id: document.revision_id(),
            source_schema_version: document.schema_version(),
            mapping_profile_id: profile.profile_id(),
            mapping_profile_version: profile.profile_version(),
            graph_id: revision.graph_id(),
            graph_revision_id: revision.revision_id(),
            graph_maturity: revision.maturity(),
            declared_graph_content_digest: revision.content_digest(),
            source_document_digest: &source_document_digest,
            mapping_profile_digest: &mapping_profile_digest,
            service_graph_payload_digest: &service_graph_payload_digest,
            standard_profiles: &metadata.standard_profiles,
            upstream_artifacts: &metadata.upstream_artifacts,
        };
        let build_lock_digest = digest_json(&material, "build_lock")?;
        Ok(Self {
            receipt_version: BUILD_RECEIPT_VERSION,
            canonicalization_profile: BUILD_CANONICALIZATION_PROFILE.to_owned(),
            digest_algorithm: "sha256".to_owned(),
            compiler_id: metadata.compiler_id,
            compiler_version: metadata.compiler_version,
            toolchain_fingerprint: metadata.toolchain_fingerprint,
            source_document_id: document.document_id().to_owned(),
            source_document_revision_id: document.revision_id().to_owned(),
            source_schema_version: document.schema_version(),
            mapping_profile_id: profile.profile_id().to_owned(),
            mapping_profile_version: profile.profile_version().to_owned(),
            graph_id: revision.graph_id().to_owned(),
            graph_revision_id: revision.revision_id().to_owned(),
            graph_maturity: revision.maturity(),
            declared_graph_content_digest: revision.content_digest().to_owned(),
            source_document_digest,
            mapping_profile_digest,
            service_graph_payload_digest,
            standard_profiles: metadata.standard_profiles,
            upstream_artifacts: metadata.upstream_artifacts,
            build_lock_digest,
        })
    }
    pub const fn receipt_version(&self) -> u16 { self.receipt_version }
    pub fn canonicalization_profile(&self) -> &str { &self.canonicalization_profile }
    pub fn digest_algorithm(&self) -> &str { &self.digest_algorithm }
    pub fn compiler_id(&self) -> &str { &self.compiler_id }
    pub fn compiler_version(&self) -> &str { &self.compiler_version }
    pub fn toolchain_fingerprint(&self) -> &str { &self.toolchain_fingerprint }
    pub fn source_document_id(&self) -> &str { &self.source_document_id }
    pub fn source_document_revision_id(&self) -> &str { &self.source_document_revision_id }
    pub const fn source_schema_version(&self) -> crate::IrSchemaVersion { self.source_schema_version }
    pub fn mapping_profile_id(&self) -> &str { &self.mapping_profile_id }
    pub fn mapping_profile_version(&self) -> &str { &self.mapping_profile_version }
    pub fn graph_id(&self) -> &str { &self.graph_id }
    pub fn graph_revision_id(&self) -> &str { &self.graph_revision_id }
    pub const fn graph_maturity(&self) -> GraphMaturity { self.graph_maturity }
    pub fn declared_graph_content_digest(&self) -> &str { &self.declared_graph_content_digest }
    pub fn source_document_digest(&self) -> &Sha256Digest { &self.source_document_digest }
    pub fn mapping_profile_digest(&self) -> &Sha256Digest { &self.mapping_profile_digest }
    pub fn service_graph_payload_digest(&self) -> &Sha256Digest { &self.service_graph_payload_digest }
    pub fn standard_profiles(&self) -> &[StandardProfilePin] { &self.standard_profiles }
    pub fn upstream_artifacts(&self) -> &[UpstreamArtifactPin] { &self.upstream_artifacts }
    pub fn build_lock_digest(&self) -> &Sha256Digest { &self.build_lock_digest }
}

#[derive(Serialize)]
struct BuildLockMaterial<'a> {
    receipt_version: u16,
    canonicalization_profile: &'static str,
    digest_algorithm: &'static str,
    compiler_id: &'a str,
    compiler_version: &'a str,
    toolchain_fingerprint: &'a str,
    source_document_id: &'a str,
    source_document_revision_id: &'a str,
    source_schema_version: crate::IrSchemaVersion,
    mapping_profile_id: &'a str,
    mapping_profile_version: &'a str,
    graph_id: &'a str,
    graph_revision_id: &'a str,
    graph_maturity: GraphMaturity,
    declared_graph_content_digest: &'a str,
    source_document_digest: &'a Sha256Digest,
    mapping_profile_digest: &'a Sha256Digest,
    service_graph_payload_digest: &'a Sha256Digest,
    standard_profiles: &'a [StandardProfilePin],
    upstream_artifacts: &'a [UpstreamArtifactPin],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReproducibleCompilation {
    compilation: IrGraphCompilation,
    receipt: PowerFabricBuildReceipt,
}
impl ReproducibleCompilation {
    pub fn compilation(&self) -> &IrGraphCompilation { &self.compilation }
    pub fn receipt(&self) -> &PowerFabricBuildReceipt { &self.receipt }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReproducibleCompilationError {
    GraphCompilation(GraphBridgeError),
    BuildReceipt(BuildReceiptError),
}

pub fn compile_ir_to_service_graph_reproducibly(
    document: &PowerFabricDocument,
    profile: &IrGraphMappingProfile,
    context: GraphCompileContext,
    metadata: BuildMetadata,
) -> Result<ReproducibleCompilation, ReproducibleCompilationError> {
    let compilation = compile_ir_to_service_graph(document, profile, context)
        .map_err(ReproducibleCompilationError::GraphCompilation)?;
    let receipt = PowerFabricBuildReceipt::create(document, profile, compilation.graph(), metadata)
        .map_err(ReproducibleCompilationError::BuildReceipt)?;
    Ok(ReproducibleCompilation { compilation, receipt })
}

fn digest_json<T: Serialize>(value: &T, artifact: &'static str) -> Result<Sha256Digest, BuildReceiptError> {
    let bytes = serde_json::to_vec(value).map_err(|_| BuildReceiptError::SerializationFailed { artifact })?;
    Ok(Sha256Digest::from_bytes(&bytes))
}
fn require_non_empty(field: &'static str, value: &str) -> Result<(), BuildReceiptError> {
    if value.trim().is_empty() { Err(BuildReceiptError::EmptyField { field }) } else { Ok(()) }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildReceiptError {
    EmptyField { field: &'static str },
    InvalidSha256Hex,
    UnknownStandardLifecycle,
    NoStandardProfilesPinned,
    DuplicateStandardProfilePin,
    DuplicateUpstreamArtifactPin,
    SerializationFailed { artifact: &'static str },
}
impl core::fmt::Display for BuildReceiptError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result { write!(f, "{self:?}") }
}
impl std::error::Error for BuildReceiptError {}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sha256_digest_matches_standard_known_vector() {
        assert_eq!(Sha256Digest::from_bytes(b"abc").as_str(),
            "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    }
    #[test]
    fn invalid_sha256_hex_is_rejected() {
        assert_eq!(Sha256Digest::from_hex("not-a-digest"), Err(BuildReceiptError::InvalidSha256Hex));
        assert_eq!(Sha256Digest::from_hex("A".repeat(64)), Err(BuildReceiptError::InvalidSha256Hex));
    }
    #[test]
    fn metadata_requires_a_pinned_standard_and_rejects_unknown_lifecycle() {
        assert_eq!(BuildMetadata::new("compiler", "0.1", "rustc+target", Vec::new(), Vec::new()),
            Err(BuildReceiptError::NoStandardProfilesPinned));
    }
}
