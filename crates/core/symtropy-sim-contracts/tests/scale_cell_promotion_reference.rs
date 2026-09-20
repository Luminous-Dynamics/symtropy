// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! SCALE-CELL-001B executable promotion/collapse reference oracle.
//!
//! This synthetic state machine freezes three independent properties:
//!
//! 1. representation handoff has exactly one current canonical owner;
//! 2. exact conserved total survives coarse <-> fine transitions;
//! 3. deterministic fine arrangement introduced during promotion remains
//!    Derived (D), never retroactively Known (K).
//!
//! It is deliberately not a production Living World adapter and does not claim
//! that its two-bucket microstate is ecological or physical truth.

use symtropy_sim_contracts::{
    DigestAlgorithm, ScopeId, TypedDigest32, WorldInstanceId,
};

const AUTHORITY_SCHEMA_VERSION: u32 = 1;
const AUTHORITY_DOMAIN: &[u8] = b"symtropy.scale-cell.authority-reference.v1\0";
const TRANSITION_DOMAIN: &[u8] = b"symtropy.scale-cell.transition-reference.v1\0";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AuthorityGeneration(u64);

impl AuthorityGeneration {
    const GENESIS: Self = Self(0);

    fn next(self) -> Result<Self, RefError> {
        self.0
            .checked_add(1)
            .map(Self)
            .ok_or(RefError::GenerationOverflow)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InformationProvenance {
    Known,
    Derived,
}

impl InformationProvenance {
    const fn code(self) -> u8 {
        match self {
            Self::Known => 0,
            Self::Derived => 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CoarseState {
    exact_total: u64,
    total_provenance: InformationProvenance,
    ancestry: TypedDigest32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FineState {
    exact_total: u64,
    buckets: [u64; 2],
    total_provenance: InformationProvenance,
    arrangement_provenance: InformationProvenance,
    ancestry: TypedDigest32,
}

impl FineState {
    fn checked_bucket_total(&self) -> Result<u64, RefError> {
        self.buckets[0]
            .checked_add(self.buckets[1])
            .ok_or(RefError::ArithmeticOverflow)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CurrentRepresentation {
    Coarse(CoarseState),
    Fine(FineState),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ScaleAuthority {
    world: WorldInstanceId,
    scope: ScopeId,
    generation: AuthorityGeneration,
    current: CurrentRepresentation,
}

impl ScaleAuthority {
    fn genesis(total: u64) -> Self {
        let ancestry = digest("symtropy.scale-cell.genesis.v1", &total.to_le_bytes());
        Self {
            world: world("world:scale-cell"),
            scope: scope("micro:m0"),
            generation: AuthorityGeneration::GENESIS,
            current: CurrentRepresentation::Coarse(CoarseState {
                exact_total: total,
                total_provenance: InformationProvenance::Known,
                ancestry,
            }),
        }
    }

    fn validate(&self) -> Result<(), RefError> {
        self.world.validate().map_err(|_| RefError::Contract)?;
        self.scope.validate().map_err(|_| RefError::Contract)?;

        match &self.current {
            CurrentRepresentation::Coarse(state) => {
                state
                    .ancestry
                    .validate()
                    .map_err(|_| RefError::Contract)?;
                if state.total_provenance != InformationProvenance::Known {
                    return Err(RefError::InvalidProvenance);
                }
            }
            CurrentRepresentation::Fine(state) => {
                state
                    .ancestry
                    .validate()
                    .map_err(|_| RefError::Contract)?;
                if state.total_provenance != InformationProvenance::Known
                    || state.arrangement_provenance != InformationProvenance::Derived
                {
                    return Err(RefError::InvalidProvenance);
                }
                if state.checked_bucket_total()? != state.exact_total {
                    return Err(RefError::ConservationMismatch);
                }
            }
        }

        Ok(())
    }

    fn canonical_bytes(&self) -> Result<Vec<u8>, RefError> {
        self.validate()?;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(AUTHORITY_DOMAIN);
        bytes.extend_from_slice(&AUTHORITY_SCHEMA_VERSION.to_le_bytes());
        push_string(&mut bytes, self.world.as_str());
        push_string(&mut bytes, self.scope.as_str());
        bytes.extend_from_slice(&self.generation.0.to_le_bytes());

        match &self.current {
            CurrentRepresentation::Coarse(state) => {
                bytes.push(0);
                bytes.extend_from_slice(&state.exact_total.to_le_bytes());
                bytes.push(state.total_provenance.code());
                push_digest(&mut bytes, &state.ancestry);
            }
            CurrentRepresentation::Fine(state) => {
                bytes.push(1);
                bytes.extend_from_slice(&state.exact_total.to_le_bytes());
                bytes.extend_from_slice(&state.buckets[0].to_le_bytes());
                bytes.extend_from_slice(&state.buckets[1].to_le_bytes());
                bytes.push(state.total_provenance.code());
                bytes.push(state.arrangement_provenance.code());
                push_digest(&mut bytes, &state.ancestry);
            }
        }

        Ok(bytes)
    }

    fn digest(&self) -> Result<TypedDigest32, RefError> {
        Ok(TypedDigest32::sha256(
            "symtropy.scale-cell.authority-reference.v1",
            AUTHORITY_SCHEMA_VERSION,
            &self.canonical_bytes()?,
        )
        .map_err(|_| RefError::Contract)?)
    }

    fn prepare_promotion(
        &self,
        profile: PromotionProfile,
    ) -> Result<PreparedPromotion, RefError> {
        self.validate()?;
        let CurrentRepresentation::Coarse(source) = &self.current else {
            return Err(RefError::WrongCurrentRepresentation);
        };

        let buckets = match profile {
            PromotionProfile::DeterministicSplitV1 => {
                let left = source.exact_total / 2;
                [left, source.exact_total - left]
            }
            PromotionProfile::NonConservingCanary => [
                source.exact_total,
                source
                    .exact_total
                    .checked_add(1)
                    .ok_or(RefError::ArithmeticOverflow)?,
            ],
        };

        let candidate_total = buckets[0]
            .checked_add(buckets[1])
            .ok_or(RefError::ArithmeticOverflow)?;
        if candidate_total != source.exact_total {
            return Err(RefError::ConservationMismatch);
        }

        let source_digest = self.digest()?;
        let candidate = FineState {
            exact_total: source.exact_total,
            buckets,
            total_provenance: InformationProvenance::Known,
            arrangement_provenance: InformationProvenance::Derived,
            ancestry: transition_ancestry(b"promotion", &source_digest),
        };
        let result_generation = self.generation.next()?;
        let result_authority = Self {
            world: self.world.clone(),
            scope: self.scope.clone(),
            generation: result_generation,
            current: CurrentRepresentation::Fine(candidate.clone()),
        };
        let result_digest = result_authority.digest()?;

        Ok(PreparedPromotion {
            source_generation: self.generation,
            result_generation,
            source_digest,
            result_digest,
            source_total: source.exact_total,
            candidate,
            profile,
        })
    }

    fn commit_promotion(
        &mut self,
        prepared: &PreparedPromotion,
    ) -> Result<TransitionReceipt, RefError> {
        self.validate()?;
        if self.generation != prepared.source_generation {
            return Err(RefError::StaleGeneration);
        }
        let CurrentRepresentation::Coarse(source) = &self.current else {
            return Err(RefError::WrongCurrentRepresentation);
        };
        if source.exact_total != prepared.source_total {
            return Err(RefError::StaleSource);
        }

        let current_digest = self.digest()?;
        if !current_digest.same_typed_value(&prepared.source_digest) {
            return Err(RefError::StaleSource);
        }
        if prepared.candidate.checked_bucket_total()? != prepared.source_total {
            return Err(RefError::ConservationMismatch);
        }

        let successor = Self {
            world: self.world.clone(),
            scope: self.scope.clone(),
            generation: prepared.result_generation,
            current: CurrentRepresentation::Fine(prepared.candidate.clone()),
        };
        let result_digest = successor.digest()?;
        if !result_digest.same_typed_value(&prepared.result_digest) {
            return Err(RefError::PreparedResultMismatch);
        }

        let receipt = TransitionReceipt::new(
            TransitionKind::Promote,
            prepared.source_generation,
            prepared.result_generation,
            prepared.source_digest.clone(),
            result_digest,
            prepared.source_total,
            prepared.candidate.exact_total,
        )?;
        *self = successor;
        Ok(receipt)
    }

    fn prepare_collapse(
        &self,
        requirement: SurvivingInformationRequirement,
    ) -> Result<PreparedCollapse, RefError> {
        self.validate()?;
        let CurrentRepresentation::Fine(source) = &self.current else {
            return Err(RefError::WrongCurrentRepresentation);
        };

        if requirement == SurvivingInformationRequirement::ExactArrangement {
            return Err(RefError::DestinationInformationInsufficient);
        }

        let source_digest = self.digest()?;
        let candidate = CoarseState {
            exact_total: source.exact_total,
            total_provenance: InformationProvenance::Known,
            ancestry: transition_ancestry(b"collapse", &source_digest),
        };
        let result_generation = self.generation.next()?;
        let result_authority = Self {
            world: self.world.clone(),
            scope: self.scope.clone(),
            generation: result_generation,
            current: CurrentRepresentation::Coarse(candidate.clone()),
        };
        let result_digest = result_authority.digest()?;

        Ok(PreparedCollapse {
            source_generation: self.generation,
            result_generation,
            source_digest,
            result_digest,
            source_total: source.exact_total,
            candidate,
            requirement,
        })
    }

    fn commit_collapse(
        &mut self,
        prepared: &PreparedCollapse,
    ) -> Result<TransitionReceipt, RefError> {
        self.validate()?;
        if self.generation != prepared.source_generation {
            return Err(RefError::StaleGeneration);
        }
        let CurrentRepresentation::Fine(source) = &self.current else {
            return Err(RefError::WrongCurrentRepresentation);
        };
        if source.exact_total != prepared.source_total {
            return Err(RefError::StaleSource);
        }
        if prepared.requirement != SurvivingInformationRequirement::TotalOnly {
            return Err(RefError::DestinationInformationInsufficient);
        }

        let current_digest = self.digest()?;
        if !current_digest.same_typed_value(&prepared.source_digest) {
            return Err(RefError::StaleSource);
        }

        let successor = Self {
            world: self.world.clone(),
            scope: self.scope.clone(),
            generation: prepared.result_generation,
            current: CurrentRepresentation::Coarse(prepared.candidate.clone()),
        };
        let result_digest = successor.digest()?;
        if !result_digest.same_typed_value(&prepared.result_digest) {
            return Err(RefError::PreparedResultMismatch);
        }

        let receipt = TransitionReceipt::new(
            TransitionKind::Collapse,
            prepared.source_generation,
            prepared.result_generation,
            prepared.source_digest.clone(),
            result_digest,
            prepared.source_total,
            prepared.candidate.exact_total,
        )?;
        *self = successor;
        Ok(receipt)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PromotionProfile {
    DeterministicSplitV1,
    NonConservingCanary,
}

impl PromotionProfile {
    const fn code(self) -> u8 {
        match self {
            Self::DeterministicSplitV1 => 0,
            Self::NonConservingCanary => 255,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SurvivingInformationRequirement {
    TotalOnly,
    ExactArrangement,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PreparedPromotion {
    source_generation: AuthorityGeneration,
    result_generation: AuthorityGeneration,
    source_digest: TypedDigest32,
    result_digest: TypedDigest32,
    source_total: u64,
    candidate: FineState,
    profile: PromotionProfile,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PreparedCollapse {
    source_generation: AuthorityGeneration,
    result_generation: AuthorityGeneration,
    source_digest: TypedDigest32,
    result_digest: TypedDigest32,
    source_total: u64,
    candidate: CoarseState,
    requirement: SurvivingInformationRequirement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TransitionKind {
    Promote,
    Collapse,
}

impl TransitionKind {
    const fn code(self) -> u8 {
        match self {
            Self::Promote => 0,
            Self::Collapse => 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TransitionReceipt {
    kind: TransitionKind,
    source_generation: AuthorityGeneration,
    result_generation: AuthorityGeneration,
    source_digest: TypedDigest32,
    result_digest: TypedDigest32,
    total_before: u64,
    total_after: u64,
    identity: TypedDigest32,
}

impl TransitionReceipt {
    fn new(
        kind: TransitionKind,
        source_generation: AuthorityGeneration,
        result_generation: AuthorityGeneration,
        source_digest: TypedDigest32,
        result_digest: TypedDigest32,
        total_before: u64,
        total_after: u64,
    ) -> Result<Self, RefError> {
        if total_before != total_after {
            return Err(RefError::ConservationMismatch);
        }

        let mut bytes = Vec::new();
        bytes.extend_from_slice(TRANSITION_DOMAIN);
        bytes.push(kind.code());
        bytes.extend_from_slice(&source_generation.0.to_le_bytes());
        bytes.extend_from_slice(&result_generation.0.to_le_bytes());
        push_digest(&mut bytes, &source_digest);
        push_digest(&mut bytes, &result_digest);
        bytes.extend_from_slice(&total_before.to_le_bytes());
        bytes.extend_from_slice(&total_after.to_le_bytes());
        let identity = TypedDigest32::sha256(
            "symtropy.scale-cell.transition-reference.v1",
            1,
            &bytes,
        )
        .map_err(|_| RefError::Contract)?;

        Ok(Self {
            kind,
            source_generation,
            result_generation,
            source_digest,
            result_digest,
            total_before,
            total_after,
            identity,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RefError {
    Contract,
    GenerationOverflow,
    ArithmeticOverflow,
    InvalidProvenance,
    ConservationMismatch,
    WrongCurrentRepresentation,
    StaleGeneration,
    StaleSource,
    PreparedResultMismatch,
    DestinationInformationInsufficient,
}

fn transition_ancestry(label: &[u8], source: &TypedDigest32) -> TypedDigest32 {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(label);
    push_digest(&mut bytes, source);
    digest("symtropy.scale-cell.transition-ancestry.v1", &bytes)
}

fn push_string(bytes: &mut Vec<u8>, value: &str) {
    bytes.extend_from_slice(&(value.len() as u32).to_le_bytes());
    bytes.extend_from_slice(value.as_bytes());
}

fn push_digest(bytes: &mut Vec<u8>, digest: &TypedDigest32) {
    push_string(bytes, &digest.domain);
    match &digest.algorithm {
        DigestAlgorithm::Sha256 => bytes.push(0),
        DigestAlgorithm::Other(name) => {
            bytes.push(1);
            push_string(bytes, name);
        }
    }
    bytes.extend_from_slice(&digest.schema_version.to_le_bytes());
    bytes.extend_from_slice(&digest.value);
}

fn digest(domain: &str, value: &[u8]) -> TypedDigest32 {
    TypedDigest32::sha256(domain, 1, value).unwrap()
}

fn world(value: &str) -> WorldInstanceId {
    WorldInstanceId::parse(value).unwrap()
}

fn scope(value: &str) -> ScopeId {
    ScopeId::parse(value).unwrap()
}

#[test]
fn promotion_preserves_total_but_marks_generated_arrangement_as_derived() {
    let mut authority = ScaleAuthority::genesis(100);
    let prepared = authority
        .prepare_promotion(PromotionProfile::DeterministicSplitV1)
        .unwrap();

    assert_eq!(prepared.source_total, 100);
    assert_eq!(prepared.candidate.buckets, [50, 50]);
    assert_eq!(
        prepared.candidate.total_provenance,
        InformationProvenance::Known
    );
    assert_eq!(
        prepared.candidate.arrangement_provenance,
        InformationProvenance::Derived
    );
    assert_eq!(authority.generation, AuthorityGeneration::GENESIS);
    assert!(matches!(authority.current, CurrentRepresentation::Coarse(_)));

    let receipt = authority.commit_promotion(&prepared).unwrap();
    assert_eq!(receipt.kind, TransitionKind::Promote);
    assert_eq!(receipt.total_before, 100);
    assert_eq!(receipt.total_after, 100);
    assert_ne!(receipt.source_digest, receipt.result_digest);
    assert_eq!(receipt.identity.domain, "symtropy.scale-cell.transition-reference.v1");

    let CurrentRepresentation::Fine(current) = &authority.current else {
        panic!("fine representation must be current after promotion");
    };
    assert_eq!(current.exact_total, 100);
    assert_eq!(current.checked_bucket_total().unwrap(), 100);
    assert_eq!(current.arrangement_provenance, InformationProvenance::Derived);
}

#[test]
fn nonconserving_promotion_candidate_is_rejected_before_mutation() {
    let authority = ScaleAuthority::genesis(100);
    let before = authority.clone();

    assert_eq!(
        authority.prepare_promotion(PromotionProfile::NonConservingCanary),
        Err(RefError::ConservationMismatch)
    );
    assert_eq!(authority, before);
}

#[test]
fn stale_prepared_promotion_cannot_create_a_second_current_owner() {
    let mut authority = ScaleAuthority::genesis(100);
    let stale = authority
        .prepare_promotion(PromotionProfile::DeterministicSplitV1)
        .unwrap();
    let winning = authority
        .prepare_promotion(PromotionProfile::DeterministicSplitV1)
        .unwrap();

    authority.commit_promotion(&winning).unwrap();
    let after_winner = authority.clone();

    assert_eq!(
        authority.commit_promotion(&stale),
        Err(RefError::StaleGeneration)
    );
    assert_eq!(authority, after_winner);
    assert!(matches!(authority.current, CurrentRepresentation::Fine(_)));
}

#[test]
fn collapse_rejects_when_surviving_process_requires_exact_arrangement() {
    let mut authority = ScaleAuthority::genesis(100);
    let promotion = authority
        .prepare_promotion(PromotionProfile::DeterministicSplitV1)
        .unwrap();
    authority.commit_promotion(&promotion).unwrap();
    let before = authority.clone();

    assert_eq!(
        authority.prepare_collapse(SurvivingInformationRequirement::ExactArrangement),
        Err(RefError::DestinationInformationInsufficient)
    );
    assert_eq!(authority, before);
    assert!(matches!(authority.current, CurrentRepresentation::Fine(_)));
}

#[test]
fn total_only_collapse_preserves_conservation_and_returns_one_coarse_owner() {
    let mut authority = ScaleAuthority::genesis(100);
    let promotion = authority
        .prepare_promotion(PromotionProfile::DeterministicSplitV1)
        .unwrap();
    authority.commit_promotion(&promotion).unwrap();
    let collapse = authority
        .prepare_collapse(SurvivingInformationRequirement::TotalOnly)
        .unwrap();
    let receipt = authority.commit_collapse(&collapse).unwrap();

    assert_eq!(receipt.kind, TransitionKind::Collapse);
    assert_eq!(receipt.total_before, 100);
    assert_eq!(receipt.total_after, 100);
    assert_eq!(receipt.result_generation.0, 2);

    let CurrentRepresentation::Coarse(current) = &authority.current else {
        panic!("coarse representation must be current after collapse");
    };
    assert_eq!(current.exact_total, 100);
    assert_eq!(current.total_provenance, InformationProvenance::Known);
}

#[test]
fn equal_visible_coarse_total_after_cycle_does_not_erase_transition_history() {
    let genesis = ScaleAuthority::genesis(100);
    let genesis_digest = genesis.digest().unwrap();
    let mut cycled = genesis.clone();

    let promotion = cycled
        .prepare_promotion(PromotionProfile::DeterministicSplitV1)
        .unwrap();
    cycled.commit_promotion(&promotion).unwrap();
    let collapse = cycled
        .prepare_collapse(SurvivingInformationRequirement::TotalOnly)
        .unwrap();
    cycled.commit_collapse(&collapse).unwrap();

    let CurrentRepresentation::Coarse(current) = &cycled.current else {
        panic!("cycled authority must be coarse");
    };
    assert_eq!(current.exact_total, 100);
    assert_eq!(cycled.generation.0, 2);
    assert_ne!(cycled.digest().unwrap(), genesis_digest);
}

#[test]
fn repeated_promote_collapse_does_not_ratchet_derived_arrangement_into_known_history() {
    let mut authority = ScaleAuthority::genesis(100);

    for _ in 0..2 {
        let promotion = authority
            .prepare_promotion(PromotionProfile::DeterministicSplitV1)
            .unwrap();
        assert_eq!(
            promotion.candidate.arrangement_provenance,
            InformationProvenance::Derived
        );
        authority.commit_promotion(&promotion).unwrap();

        let CurrentRepresentation::Fine(fine) = &authority.current else {
            panic!("promotion must make fine current");
        };
        assert_eq!(fine.arrangement_provenance, InformationProvenance::Derived);

        let collapse = authority
            .prepare_collapse(SurvivingInformationRequirement::TotalOnly)
            .unwrap();
        authority.commit_collapse(&collapse).unwrap();
    }

    let CurrentRepresentation::Coarse(coarse) = &authority.current else {
        panic!("final representation must be coarse");
    };
    assert_eq!(coarse.exact_total, 100);
    assert_eq!(coarse.total_provenance, InformationProvenance::Known);
    assert_eq!(authority.generation.0, 4);
}

#[test]
fn preparation_is_read_only_and_profile_identity_is_explicit() {
    let authority = ScaleAuthority::genesis(100);
    let before = authority.clone();
    let prepared = authority
        .prepare_promotion(PromotionProfile::DeterministicSplitV1)
        .unwrap();

    assert_eq!(prepared.profile.code(), 0);
    assert_eq!(prepared.source_generation.0, 0);
    assert_eq!(prepared.result_generation.0, 1);
    assert_eq!(authority, before);
}
