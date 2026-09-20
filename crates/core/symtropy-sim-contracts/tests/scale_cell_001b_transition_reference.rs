// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! SCALE-CELL-001B pure promotion/collapse reference fixture.
//!
//! This file freezes one synthetic authority handoff between a coarse exact
//! aggregate and a fine exact two-bucket representation. It is intentionally
//! reference-only: no production fidelity, persistence, physics, Bevy, or
//! domain conservation authority is introduced here.

use symtropy_sim_contracts::{
    ContractError, DigestAlgorithm, ScopeId, TypedDigest32, WorldInstanceId,
};

const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Generation(u64);

impl Generation {
    const GENESIS: Self = Self(0);

    fn next(self) -> Result<Self, RefError> {
        self.0
            .checked_add(1)
            .map(Self)
            .ok_or(RefError::GenerationOverflow)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProvenanceClass {
    Known,
    DerivedConditional,
}

impl ProvenanceClass {
    const fn code(self) -> u8 {
        match self {
            Self::Known => 0,
            Self::DerivedConditional => 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CoarseState {
    total: u64,
    total_provenance: ProvenanceClass,
    ancestry: TypedDigest32,
}

impl CoarseState {
    fn genesis(total: u64) -> Result<Self, RefError> {
        Ok(Self {
            total,
            total_provenance: ProvenanceClass::Known,
            ancestry: TypedDigest32::sha256(
                "symtropy.scale-cell-001b.coarse-genesis.v1",
                1,
                &total.to_le_bytes(),
            )?,
        })
    }

    fn validate(&self) -> Result<(), RefError> {
        self.ancestry.validate()?;
        if self.total_provenance != ProvenanceClass::Known {
            return Err(RefError::InvalidCoarseProvenance);
        }
        Ok(())
    }

    fn digest(&self) -> Result<TypedDigest32, RefError> {
        self.validate()?;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"symtropy.scale-cell-001b.coarse-state.v1\0");
        bytes.extend_from_slice(&self.total.to_le_bytes());
        bytes.push(self.total_provenance.code());
        push_digest(&mut bytes, &self.ancestry);
        Ok(TypedDigest32::sha256(
            "symtropy.scale-cell-001b.coarse-state.identity.v1",
            1,
            &bytes,
        )?)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FineState {
    buckets: [u64; 2],
    total_provenance: ProvenanceClass,
    detail_provenance: ProvenanceClass,
    source_coarse: TypedDigest32,
    information_profile: TypedDigest32,
}

impl FineState {
    fn total(&self) -> Result<u64, RefError> {
        self.buckets[0]
            .checked_add(self.buckets[1])
            .ok_or(RefError::QuantityOverflow)
    }

    fn validate(&self) -> Result<(), RefError> {
        self.source_coarse.validate()?;
        self.information_profile.validate()?;
        if self.total_provenance != ProvenanceClass::Known {
            return Err(RefError::InvalidFineTotalProvenance);
        }
        if self.detail_provenance != ProvenanceClass::DerivedConditional {
            return Err(RefError::InvalidFineDetailProvenance);
        }
        let _ = self.total()?;
        Ok(())
    }

    fn digest(&self) -> Result<TypedDigest32, RefError> {
        self.validate()?;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"symtropy.scale-cell-001b.fine-state.v1\0");
        bytes.extend_from_slice(&self.buckets[0].to_le_bytes());
        bytes.extend_from_slice(&self.buckets[1].to_le_bytes());
        bytes.push(self.total_provenance.code());
        bytes.push(self.detail_provenance.code());
        push_digest(&mut bytes, &self.source_coarse);
        push_digest(&mut bytes, &self.information_profile);
        Ok(TypedDigest32::sha256(
            "symtropy.scale-cell-001b.fine-state.identity.v1",
            1,
            &bytes,
        )?)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CurrentRepresentation {
    Coarse(CoarseState),
    Fine(FineState),
}

impl CurrentRepresentation {
    fn code(&self) -> u8 {
        match self {
            Self::Coarse(_) => 0,
            Self::Fine(_) => 1,
        }
    }

    fn state_digest(&self) -> Result<TypedDigest32, RefError> {
        match self {
            Self::Coarse(state) => state.digest(),
            Self::Fine(state) => state.digest(),
        }
    }

    fn total(&self) -> Result<u64, RefError> {
        match self {
            Self::Coarse(state) => Ok(state.total),
            Self::Fine(state) => state.total(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CellAuthority {
    world: WorldInstanceId,
    scope: ScopeId,
    generation: Generation,
    current: CurrentRepresentation,
}

impl CellAuthority {
    fn genesis(total: u64) -> Result<Self, RefError> {
        let authority = Self {
            world: WorldInstanceId::parse("world:scale-cell-001")?,
            scope: ScopeId::parse("micro:m0")?,
            generation: Generation::GENESIS,
            current: CurrentRepresentation::Coarse(CoarseState::genesis(total)?),
        };
        authority.validate()?;
        Ok(authority)
    }

    fn validate(&self) -> Result<(), RefError> {
        self.world.validate()?;
        self.scope.validate()?;
        match &self.current {
            CurrentRepresentation::Coarse(state) => state.validate()?,
            CurrentRepresentation::Fine(state) => state.validate()?,
        }
        Ok(())
    }

    fn commitment(&self) -> Result<TypedDigest32, RefError> {
        self.validate()?;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"symtropy.scale-cell-001b.authority.v1\0");
        push_string(&mut bytes, self.world.as_str());
        push_string(&mut bytes, self.scope.as_str());
        bytes.extend_from_slice(&self.generation.0.to_le_bytes());
        bytes.push(self.current.code());
        push_digest(&mut bytes, &self.current.state_digest()?);
        Ok(TypedDigest32::sha256(
            "symtropy.scale-cell-001b.authority-commitment.v1",
            SCHEMA_VERSION,
            &bytes,
        )?)
    }

    fn prepare_promotion(
        &self,
        demand: &ProcessDemand,
        first_bucket: u64,
    ) -> Result<PreparedPromotion, RefError> {
        self.validate()?;
        demand.validate()?;
        if demand.scope != self.scope {
            return Err(RefError::ScopeMismatch);
        }
        let source_commitment = self.commitment()?;
        if !source_commitment.same_typed_value(&demand.source_commitment) {
            return Err(RefError::StaleDemand);
        }

        let CurrentRepresentation::Coarse(coarse) = &self.current else {
            return Err(RefError::AlreadyFine);
        };
        if first_bucket > coarse.total {
            return Err(RefError::InvalidFinePartition);
        }

        let source_coarse = coarse.digest()?;
        let candidate = FineState {
            buckets: [first_bucket, coarse.total - first_bucket],
            total_provenance: ProvenanceClass::Known,
            detail_provenance: ProvenanceClass::DerivedConditional,
            source_coarse,
            information_profile: demand.information_profile.clone(),
        };
        candidate.validate()?;
        if candidate.total()? != coarse.total {
            return Err(RefError::ConservationFailure);
        }

        Ok(PreparedPromotion {
            scope: self.scope.clone(),
            source_generation: self.generation,
            result_generation: self.generation.next()?,
            source_commitment,
            target_digest: candidate.digest()?,
            candidate,
            request_identity: demand.request_identity()?,
        })
    }

    fn commit_promotion(
        &mut self,
        prepared: &PreparedPromotion,
    ) -> Result<PromotionReceipt, RefError> {
        self.validate()?;
        if prepared.scope != self.scope {
            return Err(RefError::ScopeMismatch);
        }
        if prepared.source_generation != self.generation {
            return Err(RefError::StaleGeneration);
        }
        if !self
            .commitment()?
            .same_typed_value(&prepared.source_commitment)
        {
            return Err(RefError::StaleSource);
        }
        let CurrentRepresentation::Coarse(coarse) = &self.current else {
            return Err(RefError::AlreadyFine);
        };
        prepared.candidate.validate()?;
        if prepared.candidate.total()? != coarse.total {
            return Err(RefError::ConservationFailure);
        }
        if !prepared
            .candidate
            .digest()?
            .same_typed_value(&prepared.target_digest)
        {
            return Err(RefError::PreparedTargetMismatch);
        }

        let before_total = coarse.total;
        let candidate = prepared.candidate.clone();
        self.current = CurrentRepresentation::Fine(candidate.clone());
        self.generation = prepared.result_generation;
        let result_commitment = self.commitment()?;

        Ok(PromotionReceipt {
            request_identity: prepared.request_identity.clone(),
            source_commitment: prepared.source_commitment.clone(),
            result_commitment,
            source_generation: prepared.source_generation,
            result_generation: prepared.result_generation,
            source_total: before_total,
            result_total: candidate.total()?,
            detail_provenance: candidate.detail_provenance,
            target_state: candidate.digest()?,
        })
    }

    fn prepare_collapse(&self, need: FutureInformationNeed) -> Result<PreparedCollapse, RefError> {
        self.validate()?;
        let CurrentRepresentation::Fine(fine) = &self.current else {
            return Err(RefError::NotFine);
        };
        if need == FutureInformationNeed::FineDetailRequired {
            return Err(RefError::InformationInsufficient);
        }

        let source_commitment = self.commitment()?;
        let source_fine = fine.digest()?;
        let total = fine.total()?;
        let ancestry = collapse_ancestry(&source_fine)?;
        let candidate = CoarseState {
            total,
            total_provenance: ProvenanceClass::Known,
            ancestry,
        };
        candidate.validate()?;
        if candidate.total != total {
            return Err(RefError::ConservationFailure);
        }

        Ok(PreparedCollapse {
            scope: self.scope.clone(),
            source_generation: self.generation,
            result_generation: self.generation.next()?,
            source_commitment,
            source_fine,
            target_digest: candidate.digest()?,
            candidate,
        })
    }

    fn commit_collapse(
        &mut self,
        prepared: &PreparedCollapse,
    ) -> Result<CollapseReceipt, RefError> {
        self.validate()?;
        if prepared.scope != self.scope {
            return Err(RefError::ScopeMismatch);
        }
        if prepared.source_generation != self.generation {
            return Err(RefError::StaleGeneration);
        }
        if !self
            .commitment()?
            .same_typed_value(&prepared.source_commitment)
        {
            return Err(RefError::StaleSource);
        }
        let CurrentRepresentation::Fine(fine) = &self.current else {
            return Err(RefError::NotFine);
        };
        if !fine.digest()?.same_typed_value(&prepared.source_fine) {
            return Err(RefError::StaleSource);
        }
        prepared.candidate.validate()?;
        if prepared.candidate.total != fine.total()? {
            return Err(RefError::ConservationFailure);
        }
        if !prepared
            .candidate
            .digest()?
            .same_typed_value(&prepared.target_digest)
        {
            return Err(RefError::PreparedTargetMismatch);
        }

        let before_total = fine.total()?;
        let candidate = prepared.candidate.clone();
        self.current = CurrentRepresentation::Coarse(candidate.clone());
        self.generation = prepared.result_generation;
        let result_commitment = self.commitment()?;

        Ok(CollapseReceipt {
            source_commitment: prepared.source_commitment.clone(),
            result_commitment,
            source_generation: prepared.source_generation,
            result_generation: prepared.result_generation,
            source_total: before_total,
            result_total: candidate.total,
            retained_fine_history: prepared.source_fine.clone(),
            target_state: candidate.digest()?,
        })
    }

    fn test_only_advance_coarse_total(&mut self, total: u64) -> Result<(), RefError> {
        let CurrentRepresentation::Coarse(current) = &self.current else {
            return Err(RefError::AlreadyFine);
        };
        let prior = current.digest()?;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"symtropy.scale-cell-001b.test-coarse-update.v1\0");
        push_digest(&mut bytes, &prior);
        bytes.extend_from_slice(&total.to_le_bytes());
        let ancestry = TypedDigest32::sha256(
            "symtropy.scale-cell-001b.coarse-update-ancestry.v1",
            1,
            &bytes,
        )?;
        self.current = CurrentRepresentation::Coarse(CoarseState {
            total,
            total_provenance: ProvenanceClass::Known,
            ancestry,
        });
        self.generation = self.generation.next()?;
        self.validate()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProcessDemand {
    scope: ScopeId,
    source_commitment: TypedDigest32,
    information_profile: TypedDigest32,
}

impl ProcessDemand {
    fn new(authority: &CellAuthority, profile: &[u8]) -> Result<Self, RefError> {
        Ok(Self {
            scope: authority.scope.clone(),
            source_commitment: authority.commitment()?,
            information_profile: TypedDigest32::sha256(
                "symtropy.scale-cell-001b.information-profile.v1",
                1,
                profile,
            )?,
        })
    }

    fn validate(&self) -> Result<(), RefError> {
        self.scope.validate()?;
        self.source_commitment.validate()?;
        self.information_profile.validate()?;
        Ok(())
    }

    fn request_identity(&self) -> Result<TypedDigest32, RefError> {
        self.validate()?;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"symtropy.scale-cell-001b.process-demand.v1\0");
        push_string(&mut bytes, self.scope.as_str());
        push_digest(&mut bytes, &self.source_commitment);
        push_digest(&mut bytes, &self.information_profile);
        Ok(TypedDigest32::sha256(
            "symtropy.scale-cell-001b.process-demand.identity.v1",
            1,
            &bytes,
        )?)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PreparedPromotion {
    scope: ScopeId,
    source_generation: Generation,
    result_generation: Generation,
    source_commitment: TypedDigest32,
    target_digest: TypedDigest32,
    candidate: FineState,
    request_identity: TypedDigest32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PromotionReceipt {
    request_identity: TypedDigest32,
    source_commitment: TypedDigest32,
    result_commitment: TypedDigest32,
    source_generation: Generation,
    result_generation: Generation,
    source_total: u64,
    result_total: u64,
    detail_provenance: ProvenanceClass,
    target_state: TypedDigest32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FutureInformationNeed {
    TotalOnly,
    FineDetailRequired,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PreparedCollapse {
    scope: ScopeId,
    source_generation: Generation,
    result_generation: Generation,
    source_commitment: TypedDigest32,
    source_fine: TypedDigest32,
    target_digest: TypedDigest32,
    candidate: CoarseState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CollapseReceipt {
    source_commitment: TypedDigest32,
    result_commitment: TypedDigest32,
    source_generation: Generation,
    result_generation: Generation,
    source_total: u64,
    result_total: u64,
    retained_fine_history: TypedDigest32,
    target_state: TypedDigest32,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct PresentationCopies {
    coarse: Option<TypedDigest32>,
    fine_preview: Option<TypedDigest32>,
}

fn collapse_ancestry(source_fine: &TypedDigest32) -> Result<TypedDigest32, RefError> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"symtropy.scale-cell-001b.collapse-ancestry.v1\0");
    push_digest(&mut bytes, source_fine);
    Ok(TypedDigest32::sha256(
        "symtropy.scale-cell-001b.collapse-ancestry.identity.v1",
        1,
        &bytes,
    )?)
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

#[derive(Debug, Clone, PartialEq, Eq)]
enum RefError {
    Contract,
    GenerationOverflow,
    QuantityOverflow,
    InvalidCoarseProvenance,
    InvalidFineTotalProvenance,
    InvalidFineDetailProvenance,
    ScopeMismatch,
    StaleDemand,
    AlreadyFine,
    NotFine,
    InvalidFinePartition,
    ConservationFailure,
    StaleGeneration,
    StaleSource,
    PreparedTargetMismatch,
    InformationInsufficient,
}

impl From<ContractError> for RefError {
    fn from(_: ContractError) -> Self {
        Self::Contract
    }
}

fn promote_60_40(authority: &mut CellAuthority) -> Result<PromotionReceipt, RefError> {
    let demand = ProcessDemand::new(authority, b"micro-exact-process-v1")?;
    let prepared = authority.prepare_promotion(&demand, 60)?;
    authority.commit_promotion(&prepared)
}

#[test]
fn promotion_is_read_only_until_commit_and_conserves_exact_total() {
    let mut authority = CellAuthority::genesis(100).unwrap();
    let before = authority.commitment().unwrap();
    let demand = ProcessDemand::new(&authority, b"micro-exact-process-v1").unwrap();
    let prepared = authority.prepare_promotion(&demand, 60).unwrap();

    assert_eq!(authority.commitment().unwrap(), before);
    assert!(matches!(authority.current, CurrentRepresentation::Coarse(_)));
    assert_eq!(prepared.candidate.buckets, [60, 40]);
    assert_eq!(prepared.candidate.total().unwrap(), 100);
    assert_eq!(
        prepared.candidate.detail_provenance,
        ProvenanceClass::DerivedConditional
    );

    let receipt = authority.commit_promotion(&prepared).unwrap();
    assert_eq!(receipt.source_total, 100);
    assert_eq!(receipt.result_total, 100);
    assert_eq!(receipt.detail_provenance, ProvenanceClass::DerivedConditional);
    assert_eq!(receipt.source_generation, Generation::GENESIS);
    assert_eq!(receipt.result_generation, Generation(1));
    assert_ne!(receipt.source_commitment, receipt.result_commitment);
    receipt.request_identity.validate().unwrap();
    receipt.target_state.validate().unwrap();
    assert!(matches!(authority.current, CurrentRepresentation::Fine(_)));
}

#[test]
fn stale_prepared_promotion_rejects_with_zero_additional_mutation() {
    let mut authority = CellAuthority::genesis(100).unwrap();
    let demand = ProcessDemand::new(&authority, b"micro-exact-process-v1").unwrap();
    let prepared = authority.prepare_promotion(&demand, 60).unwrap();

    authority.test_only_advance_coarse_total(101).unwrap();
    let after_competing_change = authority.commitment().unwrap();
    assert_eq!(
        authority.commit_promotion(&prepared),
        Err(RefError::StaleGeneration)
    );
    assert_eq!(authority.commitment().unwrap(), after_competing_change);
    assert_eq!(authority.current.total().unwrap(), 101);
}

#[test]
fn promotion_does_not_relabel_derived_micro_arrangement_as_known_history() {
    let mut authority = CellAuthority::genesis(100).unwrap();
    promote_60_40(&mut authority).unwrap();

    let CurrentRepresentation::Fine(fine) = &authority.current else {
        panic!("promotion must make fine representation current");
    };
    assert_eq!(fine.total_provenance, ProvenanceClass::Known);
    assert_eq!(fine.detail_provenance, ProvenanceClass::DerivedConditional);
    assert_eq!(fine.buckets, [60, 40]);
}

#[test]
fn dual_presentation_copies_do_not_create_dual_canonical_owners() {
    let mut authority = CellAuthority::genesis(100).unwrap();
    let coarse_digest = authority.current.state_digest().unwrap();
    let demand = ProcessDemand::new(&authority, b"micro-exact-process-v1").unwrap();
    let prepared = authority.prepare_promotion(&demand, 60).unwrap();
    let presentation = PresentationCopies {
        coarse: Some(coarse_digest),
        fine_preview: Some(prepared.target_digest.clone()),
    };

    assert!(matches!(authority.current, CurrentRepresentation::Coarse(_)));
    assert!(presentation.coarse.is_some());
    assert!(presentation.fine_preview.is_some());

    authority.commit_promotion(&prepared).unwrap();
    assert!(matches!(authority.current, CurrentRepresentation::Fine(_)));
    assert!(presentation.coarse.is_some());
    assert!(presentation.fine_preview.is_some());
}

#[test]
fn collapse_rejects_when_future_process_requires_fine_detail() {
    let mut authority = CellAuthority::genesis(100).unwrap();
    promote_60_40(&mut authority).unwrap();
    let before = authority.commitment().unwrap();

    assert_eq!(
        authority.prepare_collapse(FutureInformationNeed::FineDetailRequired),
        Err(RefError::InformationInsufficient)
    );
    assert_eq!(authority.commitment().unwrap(), before);
    assert!(matches!(authority.current, CurrentRepresentation::Fine(_)));
}

#[test]
fn admitted_collapse_conserves_total_and_retains_fine_ancestry() {
    let mut authority = CellAuthority::genesis(100).unwrap();
    let promotion = promote_60_40(&mut authority).unwrap();
    let promoted_commitment = authority.commitment().unwrap();
    let prepared = authority
        .prepare_collapse(FutureInformationNeed::TotalOnly)
        .unwrap();

    assert_eq!(authority.commitment().unwrap(), promoted_commitment);
    let receipt = authority.commit_collapse(&prepared).unwrap();
    assert_eq!(receipt.source_total, 100);
    assert_eq!(receipt.result_total, 100);
    assert_eq!(receipt.source_generation, Generation(1));
    assert_eq!(receipt.result_generation, Generation(2));
    assert_eq!(receipt.retained_fine_history, promotion.target_state);
    assert_ne!(receipt.source_commitment, receipt.result_commitment);
    receipt.target_state.validate().unwrap();

    let CurrentRepresentation::Coarse(coarse) = &authority.current else {
        panic!("collapse must make coarse representation current");
    };
    assert_eq!(coarse.total, 100);
    assert_eq!(coarse.total_provenance, ProvenanceClass::Known);
    assert_ne!(coarse.ancestry, CoarseState::genesis(100).unwrap().ancestry);
}

#[test]
fn equal_coarse_total_after_round_trip_is_not_equal_to_independent_genesis() {
    let genesis = CellAuthority::genesis(100).unwrap();
    let genesis_commitment = genesis.commitment().unwrap();

    let mut round_trip = CellAuthority::genesis(100).unwrap();
    promote_60_40(&mut round_trip).unwrap();
    let prepared = round_trip
        .prepare_collapse(FutureInformationNeed::TotalOnly)
        .unwrap();
    round_trip.commit_collapse(&prepared).unwrap();

    assert_eq!(round_trip.current.total().unwrap(), 100);
    assert_ne!(round_trip.commitment().unwrap(), genesis_commitment);
}

#[test]
fn invalid_partition_fails_before_any_candidate_or_authority_change() {
    let authority = CellAuthority::genesis(100).unwrap();
    let before = authority.commitment().unwrap();
    let demand = ProcessDemand::new(&authority, b"micro-exact-process-v1").unwrap();

    assert_eq!(
        authority.prepare_promotion(&demand, 101),
        Err(RefError::InvalidFinePartition)
    );
    assert_eq!(authority.commitment().unwrap(), before);
}

#[test]
fn demand_from_old_source_cannot_authorize_later_promotion() {
    let mut authority = CellAuthority::genesis(100).unwrap();
    let demand = ProcessDemand::new(&authority, b"micro-exact-process-v1").unwrap();
    authority.test_only_advance_coarse_total(100).unwrap();
    let before = authority.commitment().unwrap();

    assert_eq!(
        authority.prepare_promotion(&demand, 60),
        Err(RefError::StaleDemand)
    );
    assert_eq!(authority.commitment().unwrap(), before);
}
