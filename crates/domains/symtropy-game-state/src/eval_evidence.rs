// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Canonical integrity and reveal ordering for blinded evaluation runs.
//!
//! This module contains no vision, oracle, or scoring semantics. It commits
//! exact artifact bytes and their roles, links events in a canonical hash chain,
//! and enforces a precommitted blind-stage pipeline before reveal.

use sha2::{Digest, Sha256};
use std::{error::Error, fmt};

pub const EVIDENCE_EVENT_SCHEMA_ID: &str = "sym-eval.evidence-event.v1";
pub const EVIDENCE_EVENT_SCHEMA_VERSION: u32 = 1;
pub const ARTIFACT_DIGEST_DOMAIN: &str = "sym-eval.artifact.v1";
pub const EVENT_DIGEST_DOMAIN: &str = "sym-eval.event.v1";
pub const BLIND_ASSIGNMENT_DOMAIN: &str = "sym-eval.blind-assignment.v1";
pub const SCENARIO_CONTRACT_ROLE: &str = "scenario-contract";

const MAX_TOKEN_BYTES: usize = 128;
const MAX_BINDING_VALUE_BYTES: usize = 512;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DigestAlgorithmV1 {
    Sha256V1,
}

impl DigestAlgorithmV1 {
    const fn tag(self) -> u8 {
        match self {
            Self::Sha256V1 => 1,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Digest32V1([u8; 32]);

impl Digest32V1 {
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    pub fn to_hex(self) -> String {
        hex(&self.0)
    }
}

impl fmt::Debug for Digest32V1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("Digest32V1")
            .field(&self.to_hex())
            .finish()
    }
}

impl fmt::Display for Digest32V1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.to_hex())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ArtifactDigestV1 {
    algorithm: DigestAlgorithmV1,
    digest: Digest32V1,
    byte_len: u64,
}

impl ArtifactDigestV1 {
    pub fn hash_bytes(bytes: &[u8]) -> Result<Self, EvidenceError> {
        let byte_len = u64::try_from(bytes.len()).map_err(|_| EvidenceError::LengthOverflow)?;
        Ok(Self {
            algorithm: DigestAlgorithmV1::Sha256V1,
            digest: domain_hash(ARTIFACT_DIGEST_DOMAIN, bytes),
            byte_len,
        })
    }

    pub const fn algorithm(self) -> DigestAlgorithmV1 {
        self.algorithm
    }

    pub const fn digest(self) -> Digest32V1 {
        self.digest
    }

    pub const fn byte_len(self) -> u64 {
        self.byte_len
    }

    fn encode(&self, encoder: &mut CanonicalEncoder) {
        encoder.u8(self.algorithm.tag());
        encoder.fixed(self.digest.as_bytes());
        encoder.u64(self.byte_len);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SchemaBindingV1 {
    schema_id: String,
    schema_version: u32,
}

impl SchemaBindingV1 {
    pub fn new(schema_id: impl Into<String>, schema_version: u32) -> Result<Self, EvidenceError> {
        if schema_version == 0 {
            return Err(EvidenceError::ZeroSchemaVersion);
        }
        Ok(Self {
            schema_id: validate_token("schema_id", schema_id.into())?,
            schema_version,
        })
    }

    pub fn schema_id(&self) -> &str {
        &self.schema_id
    }

    pub const fn schema_version(&self) -> u32 {
        self.schema_version
    }

    fn encode(&self, encoder: &mut CanonicalEncoder) {
        encoder.string(&self.schema_id);
        encoder.u32(self.schema_version);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProducerBindingV1 {
    repository: String,
    commit: String,
    component: String,
}

impl ProducerBindingV1 {
    pub fn new(
        repository: impl Into<String>,
        commit: impl Into<String>,
        component: impl Into<String>,
    ) -> Result<Self, EvidenceError> {
        let commit = commit.into();
        if !matches!(commit.len(), 40 | 64) || !commit.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(EvidenceError::InvalidCommit(commit));
        }
        Ok(Self {
            repository: validate_token("repository", repository.into())?,
            commit: commit.to_ascii_lowercase(),
            component: validate_token("component", component.into())?,
        })
    }

    pub fn repository(&self) -> &str {
        &self.repository
    }

    pub fn commit(&self) -> &str {
        &self.commit
    }

    pub fn component(&self) -> &str {
        &self.component
    }

    fn encode(&self, encoder: &mut CanonicalEncoder) {
        encoder.string(&self.repository);
        encoder.string(&self.commit);
        encoder.string(&self.component);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ArtifactRefV1 {
    role: String,
    digest: ArtifactDigestV1,
    schema: Option<SchemaBindingV1>,
    producer: Option<ProducerBindingV1>,
}

impl ArtifactRefV1 {
    pub fn new(
        role: impl Into<String>,
        digest: ArtifactDigestV1,
        schema: Option<SchemaBindingV1>,
        producer: Option<ProducerBindingV1>,
    ) -> Result<Self, EvidenceError> {
        Ok(Self {
            role: validate_token("artifact_role", role.into())?,
            digest,
            schema,
            producer,
        })
    }

    pub fn role(&self) -> &str {
        &self.role
    }

    pub const fn digest(&self) -> ArtifactDigestV1 {
        self.digest
    }

    pub fn schema(&self) -> Option<&SchemaBindingV1> {
        self.schema.as_ref()
    }

    pub fn producer(&self) -> Option<&ProducerBindingV1> {
        self.producer.as_ref()
    }

    fn encode(&self, encoder: &mut CanonicalEncoder) {
        encoder.string(&self.role);
        self.digest.encode(encoder);
        encoder.option(self.schema.as_ref(), SchemaBindingV1::encode);
        encoder.option(self.producer.as_ref(), ProducerBindingV1::encode);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EvidenceBindingV1 {
    key: String,
    value: String,
}

impl EvidenceBindingV1 {
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Result<Self, EvidenceError> {
        let value = value.into();
        if value.is_empty()
            || value.len() > MAX_BINDING_VALUE_BYTES
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_graphic() || byte == b' ')
        {
            return Err(EvidenceError::InvalidText {
                field: "binding_value",
                value,
            });
        }
        Ok(Self {
            key: validate_token("binding_key", key.into())?,
            value,
        })
    }

    pub fn key(&self) -> &str {
        &self.key
    }

    pub fn value(&self) -> &str {
        &self.value
    }

    fn encode(&self, encoder: &mut CanonicalEncoder) {
        encoder.string(&self.key);
        encoder.string(&self.value);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BlindTrialV1 {
    A,
    B,
}

impl BlindTrialV1 {
    const fn tag(self) -> u8 {
        match self {
            Self::A => 0,
            Self::B => 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BlindStageV1 {
    SensorInput,
    ProducerOutput,
    AdapterOutput,
}

impl BlindStageV1 {
    const fn tag(self) -> u8 {
        match self {
            Self::SensorInput => 1,
            Self::ProducerOutput => 2,
            Self::AdapterOutput => 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinaryBlindAssignmentV1 {
    Variant0InA,
    Variant0InB,
}

impl BinaryBlindAssignmentV1 {
    const fn tag(self) -> u8 {
        match self {
            Self::Variant0InA => 0,
            Self::Variant0InB => 1,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct BlindingNonceV1([u8; 32]);

impl BlindingNonceV1 {
    /// Rejects obvious sentinel-like patterns. This is not an entropy estimator;
    /// production callers must still source secret bytes from a CSPRNG.
    pub fn from_secret_bytes(bytes: [u8; 32]) -> Result<Self, EvidenceError> {
        let all_same = bytes.iter().all(|byte| *byte == bytes[0]);
        let repeated_half = bytes[..16] == bytes[16..];
        if all_same || repeated_half {
            Err(EvidenceError::DegenerateBlindingNonce)
        } else {
            Ok(Self(bytes))
        }
    }

    const fn bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for BlindingNonceV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("BlindingNonceV1([REDACTED])")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlindCommitmentDigestV1(Digest32V1);

impl BlindCommitmentDigestV1 {
    pub const fn digest(self) -> Digest32V1 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlindAssignmentCommitmentV1 {
    experiment_id: String,
    scenario_contract: ArtifactDigestV1,
    digest: BlindCommitmentDigestV1,
}

impl BlindAssignmentCommitmentV1 {
    pub fn commit(
        experiment_id: impl Into<String>,
        scenario_contract: ArtifactDigestV1,
        assignment: BinaryBlindAssignmentV1,
        nonce: BlindingNonceV1,
    ) -> Result<Self, EvidenceError> {
        let experiment_id = validate_token("experiment_id", experiment_id.into())?;
        let digest = BlindCommitmentDigestV1(blind_commitment_digest(
            &experiment_id,
            scenario_contract,
            assignment,
            nonce,
        ));
        Ok(Self {
            experiment_id,
            scenario_contract,
            digest,
        })
    }

    pub fn experiment_id(&self) -> &str {
        &self.experiment_id
    }

    pub const fn scenario_contract(&self) -> ArtifactDigestV1 {
        self.scenario_contract
    }

    pub const fn digest(&self) -> BlindCommitmentDigestV1 {
        self.digest
    }

    fn verify_secret(
        &self,
        assignment: BinaryBlindAssignmentV1,
        nonce: BlindingNonceV1,
    ) -> Result<(), EvidenceError> {
        let actual = BlindCommitmentDigestV1(blind_commitment_digest(
            &self.experiment_id,
            self.scenario_contract,
            assignment,
            nonce,
        ));
        if actual == self.digest {
            Ok(())
        } else {
            Err(EvidenceError::BlindCommitmentMismatch)
        }
    }

    fn encode(&self, encoder: &mut CanonicalEncoder) {
        encoder.string(&self.experiment_id);
        self.scenario_contract.encode(encoder);
        encoder.fixed(self.digest.0.as_bytes());
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlindRevealV1 {
    commitment: BlindAssignmentCommitmentV1,
    assignment: BinaryBlindAssignmentV1,
    nonce: BlindingNonceV1,
}

impl BlindRevealV1 {
    pub fn new(
        commitment: BlindAssignmentCommitmentV1,
        assignment: BinaryBlindAssignmentV1,
        nonce: BlindingNonceV1,
    ) -> Result<Self, EvidenceError> {
        commitment.verify_secret(assignment, nonce)?;
        Ok(Self {
            commitment,
            assignment,
            nonce,
        })
    }

    pub fn commitment(&self) -> &BlindAssignmentCommitmentV1 {
        &self.commitment
    }

    pub const fn assignment(&self) -> BinaryBlindAssignmentV1 {
        self.assignment
    }

    fn encode(&self, encoder: &mut CanonicalEncoder) {
        self.commitment.encode(encoder);
        encoder.u8(self.assignment.tag());
        encoder.fixed(self.nonce.bytes());
    }
}

/// Blind-output stages that must exist for both A and B before reveal.
///
/// Stages are frozen before blind execution and also define the only legal
/// per-trial stage pipeline. `AdapterOutput` is never valid without a preceding
/// `ProducerOutput` stage in the profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevealRequirementsV1 {
    stages: Vec<BlindStageV1>,
}

impl RevealRequirementsV1 {
    pub fn new(mut stages: Vec<BlindStageV1>) -> Result<Self, EvidenceError> {
        if stages.is_empty() {
            return Err(EvidenceError::EmptyRevealRequirements);
        }
        stages.sort_unstable();
        if stages.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(EvidenceError::DuplicateRevealStage);
        }
        if stages.contains(&BlindStageV1::AdapterOutput)
            && !stages.contains(&BlindStageV1::ProducerOutput)
        {
            return Err(EvidenceError::InvalidBlindStageProfile(
                "adapter output requires producer output",
            ));
        }
        Ok(Self { stages })
    }

    pub fn stages(&self) -> &[BlindStageV1] {
        &self.stages
    }

    fn contains(&self, stage: BlindStageV1) -> bool {
        self.stages.contains(&stage)
    }

    fn encode(&self, encoder: &mut CanonicalEncoder) {
        encoder.sequence(&self.stages, |stage, encoder| encoder.u8(stage.tag()));
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvidenceEventKindV1 {
    Genesis,
    ContractCommitted,
    BlindingCommitted {
        commitment: BlindAssignmentCommitmentV1,
        requirements: RevealRequirementsV1,
    },
    BlindOutputCommitted {
        trial: BlindTrialV1,
        stage: BlindStageV1,
    },
    Reveal(BlindRevealV1),
    PostRevealOracle,
    ScoreCommitted,
}

impl EvidenceEventKindV1 {
    fn encode(&self, encoder: &mut CanonicalEncoder) {
        match self {
            Self::Genesis => encoder.u8(0),
            Self::ContractCommitted => encoder.u8(1),
            Self::BlindingCommitted {
                commitment,
                requirements,
            } => {
                encoder.u8(2);
                commitment.encode(encoder);
                requirements.encode(encoder);
            }
            Self::BlindOutputCommitted { trial, stage } => {
                encoder.u8(3);
                encoder.u8(trial.tag());
                encoder.u8(stage.tag());
            }
            Self::Reveal(reveal) => {
                encoder.u8(4);
                reveal.encode(encoder);
            }
            Self::PostRevealOracle => encoder.u8(5),
            Self::ScoreCommitted => encoder.u8(6),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EventDigestV1(Digest32V1);

impl EventDigestV1 {
    pub const fn digest(self) -> Digest32V1 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceEventV1 {
    sequence: u64,
    previous: Option<EventDigestV1>,
    experiment_id: String,
    kind: EvidenceEventKindV1,
    artifacts: Vec<ArtifactRefV1>,
    bindings: Vec<EvidenceBindingV1>,
    digest: EventDigestV1,
}

impl EvidenceEventV1 {
    pub const fn sequence(&self) -> u64 {
        self.sequence
    }

    pub const fn previous_event_digest(&self) -> Option<EventDigestV1> {
        self.previous
    }

    pub fn experiment_id(&self) -> &str {
        &self.experiment_id
    }

    pub fn kind(&self) -> &EvidenceEventKindV1 {
        &self.kind
    }

    pub fn artifacts(&self) -> &[ArtifactRefV1] {
        &self.artifacts
    }

    pub fn bindings(&self) -> &[EvidenceBindingV1] {
        &self.bindings
    }

    pub const fn event_digest(&self) -> EventDigestV1 {
        self.digest
    }

    fn calculate_digest(&self) -> EventDigestV1 {
        EventDigestV1(domain_hash(EVENT_DIGEST_DOMAIN, &self.canonical_bytes()))
    }

    fn canonical_bytes(&self) -> Vec<u8> {
        let mut encoder = CanonicalEncoder::default();
        encoder.string(EVIDENCE_EVENT_SCHEMA_ID);
        encoder.u32(EVIDENCE_EVENT_SCHEMA_VERSION);
        encoder.u64(self.sequence);
        encoder.option(self.previous.as_ref(), |digest, encoder| {
            encoder.fixed(digest.0.as_bytes());
        });
        encoder.string(&self.experiment_id);
        self.kind.encode(&mut encoder);
        encoder.sequence(&self.artifacts, ArtifactRefV1::encode);
        encoder.sequence(&self.bindings, EvidenceBindingV1::encode);
        encoder.finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceChainV1 {
    experiment_id: String,
    events: Vec<EvidenceEventV1>,
}

impl EvidenceChainV1 {
    pub fn new(experiment_id: impl Into<String>) -> Result<Self, EvidenceError> {
        let experiment_id = validate_token("experiment_id", experiment_id.into())?;
        let mut chain = Self {
            experiment_id,
            events: Vec::new(),
        };
        chain.append_unchecked(EvidenceEventKindV1::Genesis, Vec::new(), Vec::new())?;
        Ok(chain)
    }

    pub fn experiment_id(&self) -> &str {
        &self.experiment_id
    }

    pub fn events(&self) -> &[EvidenceEventV1] {
        &self.events
    }

    pub fn head_digest(&self) -> EventDigestV1 {
        self.events
            .last()
            .expect("EvidenceChainV1 always contains genesis")
            .digest
    }

    pub fn append_contract(
        &mut self,
        artifacts: Vec<ArtifactRefV1>,
        bindings: Vec<EvidenceBindingV1>,
    ) -> Result<EventDigestV1, EvidenceError> {
        if self.blinding_policy().is_some() || self.has_reveal() {
            return Err(EvidenceError::InvalidEventOrder(
                "contract after blinding commitment",
            ));
        }
        require_artifacts(&artifacts, "contract")?;
        for artifact in &artifacts {
            if self.contract_role_is_committed(artifact.role()) {
                return Err(EvidenceError::DuplicateContractRole(
                    artifact.role().to_owned(),
                ));
            }
        }
        self.append_unchecked(EvidenceEventKindV1::ContractCommitted, artifacts, bindings)
    }

    pub fn append_blinding_commitment(
        &mut self,
        commitment: BlindAssignmentCommitmentV1,
        requirements: RevealRequirementsV1,
        bindings: Vec<EvidenceBindingV1>,
    ) -> Result<EventDigestV1, EvidenceError> {
        if commitment.experiment_id() != self.experiment_id {
            return Err(EvidenceError::ExperimentMismatch);
        }
        if self.blinding_policy().is_some() || self.has_reveal() {
            return Err(EvidenceError::DuplicateBlindingCommitment);
        }
        if !self.scenario_contract_is_committed(commitment.scenario_contract()) {
            return Err(EvidenceError::ScenarioContractNotCommitted);
        }
        self.append_unchecked(
            EvidenceEventKindV1::BlindingCommitted {
                commitment,
                requirements,
            },
            Vec::new(),
            bindings,
        )
    }

    pub fn append_blind_output(
        &mut self,
        trial: BlindTrialV1,
        stage: BlindStageV1,
        artifacts: Vec<ArtifactRefV1>,
        bindings: Vec<EvidenceBindingV1>,
    ) -> Result<EventDigestV1, EvidenceError> {
        if self.has_reveal() {
            return Err(EvidenceError::InvalidEventOrder(
                "blind output after reveal",
            ));
        }
        let (_, requirements) = self
            .blinding_policy()
            .ok_or(EvidenceError::MissingBlindingCommitment)?;
        if !requirements.contains(stage) {
            return Err(EvidenceError::UnplannedBlindStage { trial, stage });
        }
        if self.has_blind_output(trial, stage) {
            return Err(EvidenceError::DuplicateBlindOutput { trial, stage });
        }
        let expected = self.next_blind_stage(trial, requirements);
        if expected != Some(stage) {
            return Err(EvidenceError::BlindStageOutOfOrder {
                trial,
                expected,
                actual: stage,
            });
        }
        require_artifacts(&artifacts, "blind output")?;
        self.append_unchecked(
            EvidenceEventKindV1::BlindOutputCommitted { trial, stage },
            artifacts,
            bindings,
        )
    }

    pub fn append_reveal(
        &mut self,
        reveal: BlindRevealV1,
        bindings: Vec<EvidenceBindingV1>,
    ) -> Result<EventDigestV1, EvidenceError> {
        if self.has_reveal() {
            return Err(EvidenceError::DuplicateReveal);
        }
        let (recorded, requirements) = self
            .blinding_policy()
            .ok_or(EvidenceError::MissingBlindingCommitment)?;
        if recorded != reveal.commitment() {
            return Err(EvidenceError::BlindCommitmentMismatch);
        }
        ensure_requirements(requirements, |trial, stage| {
            self.has_blind_output(trial, stage)
        })?;
        self.append_unchecked(EvidenceEventKindV1::Reveal(reveal), Vec::new(), bindings)
    }

    pub fn append_post_reveal_oracle(
        &mut self,
        artifacts: Vec<ArtifactRefV1>,
        bindings: Vec<EvidenceBindingV1>,
    ) -> Result<EventDigestV1, EvidenceError> {
        if !self.has_reveal() {
            return Err(EvidenceError::RevealRequired);
        }
        if self.has_oracle() {
            return Err(EvidenceError::DuplicatePostRevealOracle);
        }
        if self.has_score() {
            return Err(EvidenceError::InvalidEventOrder("oracle after final score"));
        }
        require_artifacts(&artifacts, "post-reveal oracle")?;
        self.append_unchecked(EvidenceEventKindV1::PostRevealOracle, artifacts, bindings)
    }

    pub fn append_score(
        &mut self,
        artifacts: Vec<ArtifactRefV1>,
        bindings: Vec<EvidenceBindingV1>,
    ) -> Result<EventDigestV1, EvidenceError> {
        if !self.has_reveal() {
            return Err(EvidenceError::RevealRequired);
        }
        if !self.has_oracle() {
            return Err(EvidenceError::PostRevealOracleRequired);
        }
        if self.has_score() {
            return Err(EvidenceError::DuplicateScore);
        }
        require_artifacts(&artifacts, "score")?;
        self.append_unchecked(EvidenceEventKindV1::ScoreCommitted, artifacts, bindings)
    }

    pub fn verify(&self) -> Result<(), EvidenceError> {
        if self.events.is_empty() {
            return Err(EvidenceError::MissingGenesis);
        }

        let mut previous = None;
        let mut contract_roles = Vec::<String>::new();
        let mut scenario_contracts = Vec::<ArtifactDigestV1>::new();
        let mut policy: Option<(&BlindAssignmentCommitmentV1, &RevealRequirementsV1)> = None;
        let mut outputs = Vec::<(BlindTrialV1, BlindStageV1)>::new();
        let mut revealed = false;
        let mut oracle_seen = false;
        let mut score_seen = false;

        for (index, event) in self.events.iter().enumerate() {
            if score_seen {
                return Err(EvidenceError::InvalidEventOrder("event after final score"));
            }
            let expected_sequence =
                u64::try_from(index).map_err(|_| EvidenceError::LengthOverflow)?;
            if event.sequence != expected_sequence {
                return Err(EvidenceError::SequenceMismatch {
                    expected: expected_sequence,
                    actual: event.sequence,
                });
            }
            if event.experiment_id != self.experiment_id {
                return Err(EvidenceError::ExperimentMismatch);
            }
            if event.previous != previous {
                return Err(EvidenceError::ParentDigestMismatch);
            }
            if event.calculate_digest() != event.digest {
                return Err(EvidenceError::EventDigestMismatch {
                    sequence: event.sequence,
                });
            }
            verify_canonical_collections(event)?;

            match &event.kind {
                EvidenceEventKindV1::Genesis => {
                    if index != 0 || !event.artifacts.is_empty() || !event.bindings.is_empty() {
                        return Err(EvidenceError::InvalidGenesis);
                    }
                }
                EvidenceEventKindV1::ContractCommitted => {
                    if index == 0 || policy.is_some() || revealed {
                        return Err(EvidenceError::InvalidEventOrder("invalid contract event"));
                    }
                    require_artifacts(&event.artifacts, "contract")?;
                    for artifact in &event.artifacts {
                        if contract_roles.iter().any(|role| role == artifact.role()) {
                            return Err(EvidenceError::DuplicateContractRole(
                                artifact.role().to_owned(),
                            ));
                        }
                        contract_roles.push(artifact.role().to_owned());
                        if artifact.role() == SCENARIO_CONTRACT_ROLE {
                            scenario_contracts.push(artifact.digest());
                        }
                    }
                }
                EvidenceEventKindV1::BlindingCommitted {
                    commitment,
                    requirements,
                } => {
                    if policy.is_some() || revealed || !event.artifacts.is_empty() {
                        return Err(EvidenceError::InvalidEventOrder(
                            "invalid blinding commitment",
                        ));
                    }
                    if commitment.experiment_id() != self.experiment_id {
                        return Err(EvidenceError::ExperimentMismatch);
                    }
                    if !scenario_contracts.contains(&commitment.scenario_contract()) {
                        return Err(EvidenceError::ScenarioContractNotCommitted);
                    }
                    validate_stage_profile(requirements)?;
                    policy = Some((commitment, requirements));
                }
                EvidenceEventKindV1::BlindOutputCommitted { trial, stage } => {
                    if revealed {
                        return Err(EvidenceError::InvalidEventOrder(
                            "blind output after reveal",
                        ));
                    }
                    let (_, requirements) =
                        policy.ok_or(EvidenceError::MissingBlindingCommitment)?;
                    if !requirements.contains(*stage) {
                        return Err(EvidenceError::UnplannedBlindStage {
                            trial: *trial,
                            stage: *stage,
                        });
                    }
                    if outputs.contains(&(*trial, *stage)) {
                        return Err(EvidenceError::DuplicateBlindOutput {
                            trial: *trial,
                            stage: *stage,
                        });
                    }
                    let expected = requirements
                        .stages()
                        .iter()
                        .copied()
                        .find(|candidate| !outputs.contains(&(*trial, *candidate)));
                    if expected != Some(*stage) {
                        return Err(EvidenceError::BlindStageOutOfOrder {
                            trial: *trial,
                            expected,
                            actual: *stage,
                        });
                    }
                    require_artifacts(&event.artifacts, "blind output")?;
                    outputs.push((*trial, *stage));
                }
                EvidenceEventKindV1::Reveal(reveal) => {
                    if revealed || !event.artifacts.is_empty() {
                        return Err(EvidenceError::InvalidEventOrder("invalid reveal"));
                    }
                    let (commitment, requirements) =
                        policy.ok_or(EvidenceError::MissingBlindingCommitment)?;
                    if commitment != reveal.commitment() {
                        return Err(EvidenceError::BlindCommitmentMismatch);
                    }
                    commitment.verify_secret(reveal.assignment, reveal.nonce)?;
                    ensure_requirements(requirements, |trial, stage| {
                        outputs.contains(&(trial, stage))
                    })?;
                    revealed = true;
                }
                EvidenceEventKindV1::PostRevealOracle => {
                    if !revealed {
                        return Err(EvidenceError::InvalidEventOrder("oracle before reveal"));
                    }
                    if oracle_seen {
                        return Err(EvidenceError::DuplicatePostRevealOracle);
                    }
                    require_artifacts(&event.artifacts, "post-reveal oracle")?;
                    oracle_seen = true;
                }
                EvidenceEventKindV1::ScoreCommitted => {
                    if !revealed {
                        return Err(EvidenceError::InvalidEventOrder("score before reveal"));
                    }
                    if !oracle_seen {
                        return Err(EvidenceError::PostRevealOracleRequired);
                    }
                    if score_seen {
                        return Err(EvidenceError::DuplicateScore);
                    }
                    require_artifacts(&event.artifacts, "score")?;
                    score_seen = true;
                }
            }

            previous = Some(event.digest);
        }
        Ok(())
    }

    fn blinding_policy(&self) -> Option<(&BlindAssignmentCommitmentV1, &RevealRequirementsV1)> {
        self.events.iter().find_map(|event| match &event.kind {
            EvidenceEventKindV1::BlindingCommitted {
                commitment,
                requirements,
            } => Some((commitment, requirements)),
            _ => None,
        })
    }

    fn contract_role_is_committed(&self, role: &str) -> bool {
        self.events.iter().any(|event| {
            matches!(&event.kind, EvidenceEventKindV1::ContractCommitted)
                && event
                    .artifacts
                    .iter()
                    .any(|artifact| artifact.role() == role)
        })
    }

    fn scenario_contract_is_committed(&self, digest: ArtifactDigestV1) -> bool {
        self.events.iter().any(|event| {
            matches!(&event.kind, EvidenceEventKindV1::ContractCommitted)
                && event.artifacts.iter().any(|artifact| {
                    artifact.role() == SCENARIO_CONTRACT_ROLE && artifact.digest() == digest
                })
        })
    }

    fn has_reveal(&self) -> bool {
        self.events
            .iter()
            .any(|event| matches!(&event.kind, EvidenceEventKindV1::Reveal(_)))
    }

    fn has_oracle(&self) -> bool {
        self.events
            .iter()
            .any(|event| matches!(&event.kind, EvidenceEventKindV1::PostRevealOracle))
    }

    fn has_score(&self) -> bool {
        self.events
            .iter()
            .any(|event| matches!(&event.kind, EvidenceEventKindV1::ScoreCommitted))
    }

    fn has_blind_output(&self, trial: BlindTrialV1, stage: BlindStageV1) -> bool {
        self.events.iter().any(|event| {
            matches!(
                &event.kind,
                EvidenceEventKindV1::BlindOutputCommitted {
                    trial: found_trial,
                    stage: found_stage,
                } if *found_trial == trial && *found_stage == stage
            )
        })
    }

    fn next_blind_stage(
        &self,
        trial: BlindTrialV1,
        requirements: &RevealRequirementsV1,
    ) -> Option<BlindStageV1> {
        requirements
            .stages()
            .iter()
            .copied()
            .find(|stage| !self.has_blind_output(trial, *stage))
    }

    fn append_unchecked(
        &mut self,
        kind: EvidenceEventKindV1,
        mut artifacts: Vec<ArtifactRefV1>,
        mut bindings: Vec<EvidenceBindingV1>,
    ) -> Result<EventDigestV1, EvidenceError> {
        artifacts.sort_unstable();
        bindings.sort_unstable();
        validate_canonical_collections(&artifacts, &bindings)?;

        let sequence =
            u64::try_from(self.events.len()).map_err(|_| EvidenceError::LengthOverflow)?;
        let previous = self.events.last().map(|event| event.digest);
        let mut event = EvidenceEventV1 {
            sequence,
            previous,
            experiment_id: self.experiment_id.clone(),
            kind,
            artifacts,
            bindings,
            digest: EventDigestV1(Digest32V1::from_bytes([0; 32])),
        };
        event.digest = event.calculate_digest();
        let digest = event.digest;
        self.events.push(event);
        Ok(digest)
    }
}

fn validate_stage_profile(requirements: &RevealRequirementsV1) -> Result<(), EvidenceError> {
    if requirements.stages().is_empty() {
        return Err(EvidenceError::EmptyRevealRequirements);
    }
    if requirements
        .stages()
        .windows(2)
        .any(|pair| pair[0] >= pair[1])
    {
        return Err(EvidenceError::InvalidBlindStageProfile(
            "stage profile is not canonical",
        ));
    }
    if requirements.contains(BlindStageV1::AdapterOutput)
        && !requirements.contains(BlindStageV1::ProducerOutput)
    {
        return Err(EvidenceError::InvalidBlindStageProfile(
            "adapter output requires producer output",
        ));
    }
    Ok(())
}

fn require_artifacts(artifacts: &[ArtifactRefV1], kind: &'static str) -> Result<(), EvidenceError> {
    if artifacts.is_empty() {
        Err(EvidenceError::EventRequiresArtifacts(kind))
    } else {
        Ok(())
    }
}

fn ensure_requirements(
    requirements: &RevealRequirementsV1,
    has_output: impl Fn(BlindTrialV1, BlindStageV1) -> bool,
) -> Result<(), EvidenceError> {
    for &stage in requirements.stages() {
        for trial in [BlindTrialV1::A, BlindTrialV1::B] {
            if !has_output(trial, stage) {
                return Err(EvidenceError::MissingRequiredBlindOutput { trial, stage });
            }
        }
    }
    Ok(())
}

fn validate_canonical_collections(
    artifacts: &[ArtifactRefV1],
    bindings: &[EvidenceBindingV1],
) -> Result<(), EvidenceError> {
    if artifacts.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(EvidenceError::DuplicateArtifactRef);
    }
    if let Some(pair) = artifacts
        .windows(2)
        .find(|pair| pair[0].role == pair[1].role)
    {
        return Err(EvidenceError::DuplicateArtifactRole(pair[0].role.clone()));
    }
    if let Some(pair) = bindings.windows(2).find(|pair| pair[0].key == pair[1].key) {
        return Err(EvidenceError::DuplicateBindingKey(pair[0].key.clone()));
    }
    Ok(())
}

fn verify_canonical_collections(event: &EvidenceEventV1) -> Result<(), EvidenceError> {
    if event.artifacts.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(EvidenceError::NonCanonicalArtifactOrder);
    }
    if event
        .artifacts
        .windows(2)
        .any(|pair| pair[0].role == pair[1].role)
    {
        return Err(EvidenceError::DuplicateArtifactRole(
            event
                .artifacts
                .windows(2)
                .find(|pair| pair[0].role == pair[1].role)
                .expect("duplicate role exists")[0]
                .role
                .clone(),
        ));
    }
    if event
        .bindings
        .windows(2)
        .any(|pair| pair[0].key >= pair[1].key)
    {
        return Err(EvidenceError::NonCanonicalBindingOrder);
    }
    Ok(())
}

fn blind_commitment_digest(
    experiment_id: &str,
    scenario_contract: ArtifactDigestV1,
    assignment: BinaryBlindAssignmentV1,
    nonce: BlindingNonceV1,
) -> Digest32V1 {
    let mut encoder = CanonicalEncoder::default();
    encoder.u32(1);
    encoder.string(experiment_id);
    scenario_contract.encode(&mut encoder);
    encoder.u8(assignment.tag());
    encoder.fixed(nonce.bytes());
    domain_hash(BLIND_ASSIGNMENT_DOMAIN, &encoder.finish())
}

fn domain_hash(domain: &str, payload: &[u8]) -> Digest32V1 {
    let mut hasher = Sha256::new();
    let domain_len = u32::try_from(domain.len()).expect("static domain fits u32");
    let payload_len = u64::try_from(payload.len()).expect("in-memory payload fits u64");
    hasher.update(domain_len.to_be_bytes());
    hasher.update(domain.as_bytes());
    hasher.update(payload_len.to_be_bytes());
    hasher.update(payload);
    Digest32V1::from_bytes(hasher.finalize().into())
}

fn validate_token(field: &'static str, value: String) -> Result<String, EvidenceError> {
    let valid = !value.is_empty()
        && value.len() <= MAX_TOKEN_BYTES
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_' | b':' | b'/' | b'@')
        });
    if valid {
        Ok(value)
    } else {
        Err(EvidenceError::InvalidText { field, value })
    }
}

#[derive(Default)]
struct CanonicalEncoder {
    bytes: Vec<u8>,
}

impl CanonicalEncoder {
    fn u8(&mut self, value: u8) {
        self.bytes.push(value);
    }

    fn u32(&mut self, value: u32) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    fn u64(&mut self, value: u64) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    fn fixed(&mut self, bytes: &[u8]) {
        self.bytes.extend_from_slice(bytes);
    }

    fn string(&mut self, value: &str) {
        let len = u32::try_from(value.len()).expect("validated string fits u32");
        self.u32(len);
        self.bytes.extend_from_slice(value.as_bytes());
    }

    fn option<T>(&mut self, value: Option<&T>, encode: impl FnOnce(&T, &mut Self)) {
        match value {
            Some(value) => {
                self.u8(1);
                encode(value, self);
            }
            None => self.u8(0),
        }
    }

    fn sequence<T>(&mut self, values: &[T], encode: impl Fn(&T, &mut Self)) {
        let len = u32::try_from(values.len()).expect("bounded sequence fits u32");
        self.u32(len);
        for value in values {
            encode(value, self);
        }
    }

    fn finish(self) -> Vec<u8> {
        self.bytes
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvidenceError {
    InvalidText {
        field: &'static str,
        value: String,
    },
    InvalidCommit(String),
    ZeroSchemaVersion,
    LengthOverflow,
    DegenerateBlindingNonce,
    BlindCommitmentMismatch,
    ExperimentMismatch,
    EmptyRevealRequirements,
    DuplicateRevealStage,
    InvalidBlindStageProfile(&'static str),
    DuplicateArtifactRef,
    DuplicateArtifactRole(String),
    DuplicateBindingKey(String),
    DuplicateContractRole(String),
    DuplicateBlindOutput {
        trial: BlindTrialV1,
        stage: BlindStageV1,
    },
    UnplannedBlindStage {
        trial: BlindTrialV1,
        stage: BlindStageV1,
    },
    BlindStageOutOfOrder {
        trial: BlindTrialV1,
        expected: Option<BlindStageV1>,
        actual: BlindStageV1,
    },
    NonCanonicalArtifactOrder,
    NonCanonicalBindingOrder,
    EventRequiresArtifacts(&'static str),
    ScenarioContractNotCommitted,
    MissingBlindingCommitment,
    DuplicateBlindingCommitment,
    DuplicateReveal,
    RevealRequired,
    DuplicatePostRevealOracle,
    PostRevealOracleRequired,
    DuplicateScore,
    MissingRequiredBlindOutput {
        trial: BlindTrialV1,
        stage: BlindStageV1,
    },
    MissingGenesis,
    InvalidGenesis,
    SequenceMismatch {
        expected: u64,
        actual: u64,
    },
    ParentDigestMismatch,
    EventDigestMismatch {
        sequence: u64,
    },
    InvalidEventOrder(&'static str),
}

impl fmt::Display for EvidenceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidText { field, value } => write!(formatter, "invalid {field}: {value:?}"),
            Self::InvalidCommit(value) => {
                write!(formatter, "invalid exact commit identity: {value:?}")
            }
            Self::ZeroSchemaVersion => formatter.write_str("schema version must be nonzero"),
            Self::LengthOverflow => {
                formatter.write_str("evidence length exceeded portable representation")
            }
            Self::DegenerateBlindingNonce => {
                formatter.write_str("blinding nonce is an obvious degenerate sentinel")
            }
            Self::BlindCommitmentMismatch => {
                formatter.write_str("blinding reveal does not match the prior commitment")
            }
            Self::ExperimentMismatch => {
                formatter.write_str("evidence belongs to a different experiment")
            }
            Self::EmptyRevealRequirements => {
                formatter.write_str("reveal requirements cannot be empty")
            }
            Self::DuplicateRevealStage => {
                formatter.write_str("reveal requirements contain a duplicate stage")
            }
            Self::InvalidBlindStageProfile(reason) => {
                write!(formatter, "invalid blind stage profile: {reason}")
            }
            Self::DuplicateArtifactRef => {
                formatter.write_str("event contains a duplicate artifact reference")
            }
            Self::DuplicateArtifactRole(role) => {
                write!(formatter, "event contains duplicate artifact role {role:?}")
            }
            Self::DuplicateBindingKey(key) => {
                write!(formatter, "event contains duplicate binding key {key:?}")
            }
            Self::DuplicateContractRole(role) => {
                write!(formatter, "contract role {role:?} was already committed")
            }
            Self::DuplicateBlindOutput { trial, stage } => write!(
                formatter,
                "blind output for {trial:?}/{stage:?} was already committed"
            ),
            Self::UnplannedBlindStage { trial, stage } => write!(
                formatter,
                "blind output for {trial:?}/{stage:?} is not in the precommitted stage profile"
            ),
            Self::BlindStageOutOfOrder {
                trial,
                expected,
                actual,
            } => write!(
                formatter,
                "blind trial {trial:?} expected next stage {expected:?}, got {actual:?}"
            ),
            Self::NonCanonicalArtifactOrder => {
                formatter.write_str("artifact references are not in canonical strict order")
            }
            Self::NonCanonicalBindingOrder => {
                formatter.write_str("evidence bindings are not in canonical strict order")
            }
            Self::EventRequiresArtifacts(kind) => {
                write!(formatter, "{kind} event requires at least one artifact")
            }
            Self::ScenarioContractNotCommitted => formatter.write_str(
                "blinding commitment references no committed scenario-contract artifact",
            ),
            Self::MissingBlindingCommitment => {
                formatter.write_str("blind output/reveal requires a prior blinding commitment")
            }
            Self::DuplicateBlindingCommitment => {
                formatter.write_str("only one blinding assignment commitment is allowed")
            }
            Self::DuplicateReveal => formatter.write_str("only one reveal event is allowed"),
            Self::RevealRequired => {
                formatter.write_str("post-reveal evidence cannot be committed before reveal")
            }
            Self::DuplicatePostRevealOracle => {
                formatter.write_str("only one authoritative post-reveal oracle event is allowed")
            }
            Self::PostRevealOracleRequired => {
                formatter.write_str("score requires the authoritative post-reveal oracle first")
            }
            Self::DuplicateScore => {
                formatter.write_str("only one authoritative final score event is allowed")
            }
            Self::MissingRequiredBlindOutput { trial, stage } => write!(
                formatter,
                "reveal is missing required {stage:?} output for blind trial {trial:?}"
            ),
            Self::MissingGenesis => formatter.write_str("evidence chain has no genesis event"),
            Self::InvalidGenesis => {
                formatter.write_str("evidence chain genesis is malformed or misplaced")
            }
            Self::SequenceMismatch { expected, actual } => write!(
                formatter,
                "evidence sequence mismatch: expected {expected}, got {actual}"
            ),
            Self::ParentDigestMismatch => {
                formatter.write_str("evidence parent digest does not match the preceding event")
            }
            Self::EventDigestMismatch { sequence } => write!(
                formatter,
                "evidence event {sequence} digest does not match canonical bytes"
            ),
            Self::InvalidEventOrder(reason) => {
                write!(formatter, "invalid evidence event order: {reason}")
            }
        }
    }
}

impl Error for EvidenceError {}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        output.push(char::from(DIGITS[usize::from(byte >> 4)]));
        output.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn artifact(role: &str, bytes: &[u8]) -> ArtifactRefV1 {
        ArtifactRefV1::new(
            role,
            ArtifactDigestV1::hash_bytes(bytes).expect("small artifact"),
            None,
            None,
        )
        .expect("valid artifact ref")
    }

    fn nonce(seed: u8) -> BlindingNonceV1 {
        let mut bytes = [0_u8; 32];
        for (index, byte) in bytes.iter_mut().enumerate() {
            let offset = u8::try_from(index).expect("index fits u8");
            *byte = seed.wrapping_add(offset).wrapping_add(1);
        }
        BlindingNonceV1::from_secret_bytes(bytes).expect("test nonce")
    }

    fn committed_chain_with(stages: Vec<BlindStageV1>) -> (EvidenceChainV1, BlindRevealV1) {
        let mut chain = EvidenceChainV1::new("experiment-001").expect("chain");
        let contract = artifact(SCENARIO_CONTRACT_ROLE, b"scenario-v1");
        let contract_digest = contract.digest();
        chain
            .append_contract(vec![contract], Vec::new())
            .expect("contract");
        let assignment = BinaryBlindAssignmentV1::Variant0InA;
        let nonce = nonce(41);
        let commitment = BlindAssignmentCommitmentV1::commit(
            "experiment-001",
            contract_digest,
            assignment,
            nonce,
        )
        .expect("commitment");
        chain
            .append_blinding_commitment(
                commitment.clone(),
                RevealRequirementsV1::new(stages).expect("requirements"),
                Vec::new(),
            )
            .expect("commitment event");
        let reveal = BlindRevealV1::new(commitment, assignment, nonce).expect("reveal");
        (chain, reveal)
    }

    fn committed_chain() -> (EvidenceChainV1, BlindRevealV1) {
        committed_chain_with(vec![BlindStageV1::ProducerOutput])
    }

    fn append_required_outputs(chain: &mut EvidenceChainV1, stages: &[BlindStageV1]) {
        for trial in [BlindTrialV1::A, BlindTrialV1::B] {
            for &stage in stages {
                chain
                    .append_blind_output(
                        trial,
                        stage,
                        vec![artifact("blind-artifact", &[trial.tag(), stage.tag()])],
                        Vec::new(),
                    )
                    .expect("blind output");
            }
        }
    }

    #[test]
    fn artifact_digest_has_frozen_domain_vector() {
        let digest = ArtifactDigestV1::hash_bytes(b"abc").expect("digest");
        assert_eq!(digest.algorithm(), DigestAlgorithmV1::Sha256V1);
        assert_eq!(digest.byte_len(), 3);
        assert_eq!(
            digest.digest().to_hex(),
            "a832bad61f1c5acedb7b543269ae5274c9e1637c78d97230f02b92d21760d639"
        );
    }

    #[test]
    fn artifact_mutation_and_role_change_change_commitment() {
        let a = ArtifactDigestV1::hash_bytes(b"payload-a").expect("digest");
        let b = ArtifactDigestV1::hash_bytes(b"payload-b").expect("digest");
        assert_ne!(a, b);

        let mut first = EvidenceChainV1::new("role-test").expect("chain");
        let mut second = EvidenceChainV1::new("role-test").expect("chain");
        let digest = ArtifactDigestV1::hash_bytes(b"same").expect("digest");
        first
            .append_contract(
                vec![ArtifactRefV1::new("sensor", digest, None, None).expect("ref")],
                Vec::new(),
            )
            .expect("event");
        second
            .append_contract(
                vec![ArtifactRefV1::new("producer", digest, None, None).expect("ref")],
                Vec::new(),
            )
            .expect("event");
        assert_ne!(first.head_digest(), second.head_digest());
    }

    #[test]
    fn unordered_inputs_are_canonicalized() {
        let mut first = EvidenceChainV1::new("order-test").expect("chain");
        let mut second = EvidenceChainV1::new("order-test").expect("chain");
        let a = artifact("a", b"one");
        let b = artifact("b", b"two");
        let x = EvidenceBindingV1::new("camera", "v1").expect("binding");
        let y = EvidenceBindingV1::new("renderer", "cpu").expect("binding");
        first
            .append_contract(vec![a.clone(), b.clone()], vec![x.clone(), y.clone()])
            .expect("event");
        second
            .append_contract(vec![b, a], vec![y, x])
            .expect("event");
        assert_eq!(first.head_digest(), second.head_digest());
    }

    #[test]
    fn duplicate_artifact_role_and_contract_role_fail_closed() {
        let mut chain = EvidenceChainV1::new("roles").expect("chain");
        assert_eq!(
            chain.append_contract(
                vec![artifact("same-role", b"a"), artifact("same-role", b"b")],
                Vec::new(),
            ),
            Err(EvidenceError::DuplicateArtifactRole("same-role".to_owned()))
        );
        chain
            .append_contract(vec![artifact("camera", b"v1")], Vec::new())
            .expect("camera contract");
        assert_eq!(
            chain.append_contract(vec![artifact("camera", b"v2")], Vec::new()),
            Err(EvidenceError::DuplicateContractRole("camera".to_owned()))
        );
    }

    #[test]
    fn scenario_commitment_must_reference_scenario_contract_role() {
        let mut chain = EvidenceChainV1::new("scenario-role").expect("chain");
        let wrong = artifact("camera-contract", b"scenario-bytes");
        let digest = wrong.digest();
        chain
            .append_contract(vec![wrong], Vec::new())
            .expect("contract");
        let commitment = BlindAssignmentCommitmentV1::commit(
            "scenario-role",
            digest,
            BinaryBlindAssignmentV1::Variant0InA,
            nonce(1),
        )
        .expect("commitment");
        assert_eq!(
            chain.append_blinding_commitment(
                commitment,
                RevealRequirementsV1::new(vec![BlindStageV1::ProducerOutput])
                    .expect("requirements"),
                Vec::new(),
            ),
            Err(EvidenceError::ScenarioContractNotCommitted)
        );
    }

    #[test]
    fn degenerate_nonce_is_rejected() {
        assert_eq!(
            BlindingNonceV1::from_secret_bytes([0; 32]),
            Err(EvidenceError::DegenerateBlindingNonce)
        );
        assert_eq!(
            BlindingNonceV1::from_secret_bytes([9; 32]),
            Err(EvidenceError::DegenerateBlindingNonce)
        );
    }

    #[test]
    fn wrong_assignment_and_nonce_cannot_open_commitment() {
        let contract = ArtifactDigestV1::hash_bytes(b"contract").expect("digest");
        let good_nonce = nonce(7);
        let commitment = BlindAssignmentCommitmentV1::commit(
            "blind-test",
            contract,
            BinaryBlindAssignmentV1::Variant0InA,
            good_nonce,
        )
        .expect("commitment");
        assert_eq!(
            BlindRevealV1::new(
                commitment.clone(),
                BinaryBlindAssignmentV1::Variant0InB,
                good_nonce,
            ),
            Err(EvidenceError::BlindCommitmentMismatch)
        );
        assert_eq!(
            BlindRevealV1::new(commitment, BinaryBlindAssignmentV1::Variant0InA, nonce(99),),
            Err(EvidenceError::BlindCommitmentMismatch)
        );
    }

    #[test]
    fn adapter_stage_requires_producer_in_profile() {
        assert_eq!(
            RevealRequirementsV1::new(
                vec![BlindStageV1::SensorInput, BlindStageV1::AdapterOutput,]
            ),
            Err(EvidenceError::InvalidBlindStageProfile(
                "adapter output requires producer output"
            ))
        );
    }

    #[test]
    fn stage_not_in_profile_is_rejected() {
        let (mut chain, _reveal) = committed_chain();
        assert_eq!(
            chain.append_blind_output(
                BlindTrialV1::A,
                BlindStageV1::SensorInput,
                vec![artifact("sensor", b"frame")],
                Vec::new(),
            ),
            Err(EvidenceError::UnplannedBlindStage {
                trial: BlindTrialV1::A,
                stage: BlindStageV1::SensorInput,
            })
        );
    }

    #[test]
    fn blind_stage_pipeline_is_monotonic_per_trial() {
        let stages = vec![
            BlindStageV1::SensorInput,
            BlindStageV1::ProducerOutput,
            BlindStageV1::AdapterOutput,
        ];
        let (mut chain, _reveal) = committed_chain_with(stages);
        assert_eq!(
            chain.append_blind_output(
                BlindTrialV1::A,
                BlindStageV1::AdapterOutput,
                vec![artifact("adapter", b"early")],
                Vec::new(),
            ),
            Err(EvidenceError::BlindStageOutOfOrder {
                trial: BlindTrialV1::A,
                expected: Some(BlindStageV1::SensorInput),
                actual: BlindStageV1::AdapterOutput,
            })
        );
        chain
            .append_blind_output(
                BlindTrialV1::A,
                BlindStageV1::SensorInput,
                vec![artifact("sensor", b"a")],
                Vec::new(),
            )
            .expect("sensor");
        assert_eq!(
            chain.append_blind_output(
                BlindTrialV1::A,
                BlindStageV1::AdapterOutput,
                vec![artifact("adapter", b"still-early")],
                Vec::new(),
            ),
            Err(EvidenceError::BlindStageOutOfOrder {
                trial: BlindTrialV1::A,
                expected: Some(BlindStageV1::ProducerOutput),
                actual: BlindStageV1::AdapterOutput,
            })
        );
    }

    #[test]
    fn reveal_requires_both_trials_for_each_precommitted_stage() {
        let (mut chain, reveal) = committed_chain();
        chain
            .append_blind_output(
                BlindTrialV1::A,
                BlindStageV1::ProducerOutput,
                vec![artifact("producer-output", b"a")],
                Vec::new(),
            )
            .expect("A output");
        assert_eq!(
            chain.append_reveal(reveal.clone(), Vec::new()),
            Err(EvidenceError::MissingRequiredBlindOutput {
                trial: BlindTrialV1::B,
                stage: BlindStageV1::ProducerOutput,
            })
        );
        chain
            .append_blind_output(
                BlindTrialV1::B,
                BlindStageV1::ProducerOutput,
                vec![artifact("producer-output", b"b")],
                Vec::new(),
            )
            .expect("B output");
        chain.append_reveal(reveal, Vec::new()).expect("reveal");
        chain.verify().expect("valid chain");
    }

    #[test]
    fn duplicate_blind_stage_is_rejected() {
        let (mut chain, _reveal) = committed_chain();
        chain
            .append_blind_output(
                BlindTrialV1::A,
                BlindStageV1::ProducerOutput,
                vec![artifact("output", b"first")],
                Vec::new(),
            )
            .expect("first output");
        assert_eq!(
            chain.append_blind_output(
                BlindTrialV1::A,
                BlindStageV1::ProducerOutput,
                vec![artifact("output", b"second")],
                Vec::new(),
            ),
            Err(EvidenceError::DuplicateBlindOutput {
                trial: BlindTrialV1::A,
                stage: BlindStageV1::ProducerOutput,
            })
        );
    }

    #[test]
    fn score_requires_single_authoritative_oracle_and_is_terminal() {
        let (mut chain, reveal) = committed_chain();
        append_required_outputs(&mut chain, &[BlindStageV1::ProducerOutput]);
        chain.append_reveal(reveal, Vec::new()).expect("reveal");
        assert_eq!(
            chain.append_score(vec![artifact("score", b"early")], Vec::new()),
            Err(EvidenceError::PostRevealOracleRequired)
        );
        chain
            .append_post_reveal_oracle(vec![artifact("oracle", b"truth")], Vec::new())
            .expect("oracle");
        assert_eq!(
            chain.append_post_reveal_oracle(vec![artifact("oracle-2", b"other")], Vec::new(),),
            Err(EvidenceError::DuplicatePostRevealOracle)
        );
        chain
            .append_score(vec![artifact("score", b"metrics")], Vec::new())
            .expect("score");
        assert_eq!(
            chain.append_score(vec![artifact("score-2", b"metrics-2")], Vec::new()),
            Err(EvidenceError::DuplicateScore)
        );
        chain.verify().expect("valid terminal chain");
    }

    #[test]
    fn full_multi_stage_chain_verifies() {
        let stages = vec![
            BlindStageV1::SensorInput,
            BlindStageV1::ProducerOutput,
            BlindStageV1::AdapterOutput,
        ];
        let (mut chain, reveal) = committed_chain_with(stages.clone());
        append_required_outputs(&mut chain, &stages);
        chain.append_reveal(reveal, Vec::new()).expect("reveal");
        chain
            .append_post_reveal_oracle(vec![artifact("oracle", b"truth")], Vec::new())
            .expect("oracle");
        chain
            .append_score(vec![artifact("score", b"metrics")], Vec::new())
            .expect("score");
        chain.verify().expect("valid chain");
    }

    #[test]
    fn parent_tamper_and_sequence_rollback_are_detected() {
        let mut parent = EvidenceChainV1::new("parent-test").expect("chain");
        parent
            .append_contract(vec![artifact("contract", b"v1")], Vec::new())
            .expect("contract");
        parent.events[1].previous = None;
        assert_eq!(parent.verify(), Err(EvidenceError::ParentDigestMismatch));

        let mut sequence = EvidenceChainV1::new("sequence-test").expect("chain");
        sequence
            .append_contract(vec![artifact("contract", b"v1")], Vec::new())
            .expect("contract");
        sequence.events[1].sequence = 0;
        assert_eq!(
            sequence.verify(),
            Err(EvidenceError::SequenceMismatch {
                expected: 1,
                actual: 0,
            })
        );
    }

    #[test]
    fn blind_output_after_reveal_is_rejected() {
        let (mut chain, reveal) = committed_chain();
        append_required_outputs(&mut chain, &[BlindStageV1::ProducerOutput]);
        chain.append_reveal(reveal, Vec::new()).expect("reveal");
        assert_eq!(
            chain.append_blind_output(
                BlindTrialV1::A,
                BlindStageV1::ProducerOutput,
                vec![artifact("late", b"late")],
                Vec::new(),
            ),
            Err(EvidenceError::InvalidEventOrder(
                "blind output after reveal"
            ))
        );
    }
}
