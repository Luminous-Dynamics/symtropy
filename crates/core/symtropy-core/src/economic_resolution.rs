// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Economic conservation across simulation-resolution transitions.
//!
//! ECON-03 is deliberately a reconciliation kernel, not a world scheduler. It
//! establishes one rule: changing simulation fidelity cannot itself create, destroy,
//! transfer, settle, or invent economically relevant state.
//!
//! Demotion may discard active fine-grained simulation detail, but the conserved
//! economic manifest remains unchanged. Promotion is fail-closed: a coarse state may
//! become finer only when retained-detail provenance exists and a supplied exact state
//! independently reconstructs the same conservation manifest. If detail was discarded
//! without a retention reference, this authority refuses to synthesize replacement
//! lots, accounts, histories, or ownership facts.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::economic::{
    ActorId, CausalId, CommoditySpecId, EconomicError, LocationId, StockDepletionCause,
    StockEvent, StockLedger, StockOrigin,
};
use crate::financial::{
    AccountTotals, CurrencyDefinition, CurrencyId, FinancialAccount, FinancialAccountClass,
    FinancialAccountId, FinancialBook, FinancialError, JournalTransactionId, MonetarySupplyEvent,
};
use crate::settlement::{MonetarySettlementLedger, SettlementError, SettlementId};

const MAX_ID_LEN: usize = 256;
const MAX_RESOLUTION_TRANSITIONS: usize = 65_536;

macro_rules! id_type {
    ($name:ident, $kind:literal) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, ResolutionError> {
                let value = value.into();
                validate_id($kind, &value)?;
                Ok(Self(value))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }
    };
}

id_type!(EconomicSnapshotId, "economic-snapshot");
id_type!(DetailRetentionId, "detail-retention");

/// Economic simulation fidelity, ordered from finest to coarsest.
///
/// This enum describes representation fidelity only. It does not grant authority to
/// mutate the represented economy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EconomicResolutionTier {
    ActiveSite,
    LocalRegion,
    DistantRegion,
    PlanetaryAggregate,
}

impl EconomicResolutionTier {
    const fn rank(self) -> u8 {
        match self {
            Self::ActiveSite => 0,
            Self::LocalRegion => 1,
            Self::DistantRegion => 2,
            Self::PlanetaryAggregate => 3,
        }
    }

    fn is_coarser_than(self, other: Self) -> bool {
        self.rank() > other.rank()
    }

    fn is_finer_than(self, other: Self) -> bool {
        self.rank() < other.rank()
    }
}

/// Canonical complete static registry needed to reconstruct a `FinancialBook`.
///
/// ECON-03 does not infer the registry from transaction history because a valid
/// zero-balance account or zero-supply currency might never appear in a transaction.
/// Completeness is proven by reconstructing the complete book from this registry plus
/// the book's own histories and requiring exact `FinancialBook` equality.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FinancialRegistrySnapshot {
    currencies: Vec<CurrencyDefinition>,
    accounts: Vec<FinancialAccount>,
}

impl FinancialRegistrySnapshot {
    pub fn new(
        mut currencies: Vec<CurrencyDefinition>,
        mut accounts: Vec<FinancialAccount>,
    ) -> Result<Self, ResolutionError> {
        currencies.sort_by(|a, b| a.currency_id.cmp(&b.currency_id));
        accounts.sort_by(|a, b| a.account_id.cmp(&b.account_id));

        for pair in currencies.windows(2) {
            if pair[0].currency_id == pair[1].currency_id {
                return Err(ResolutionError::DuplicateRegistryCurrency {
                    currency_id: pair[0].currency_id.clone(),
                });
            }
        }
        for pair in accounts.windows(2) {
            if pair[0].account_id == pair[1].account_id {
                return Err(ResolutionError::DuplicateRegistryAccount {
                    account_id: pair[0].account_id.clone(),
                });
            }
        }

        Ok(Self {
            currencies,
            accounts,
        })
    }

    pub fn currencies(&self) -> &[CurrencyDefinition] {
        &self.currencies
    }

    pub fn accounts(&self) -> &[FinancialAccount] {
        &self.accounts
    }

    fn validate_against(&self, book: &FinancialBook) -> Result<(), ResolutionError> {
        let reconstructed = FinancialBook::from_history(
            self.currencies.clone(),
            self.accounts.clone(),
            book.journal_entries().to_vec(),
            book.monetary_entries().to_vec(),
        )?;
        if &reconstructed != book {
            return Err(ResolutionError::FinancialRegistryMismatch);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct StockConservationKey {
    pub commodity_spec_id: CommoditySpecId,
    pub owner_id: ActorId,
    pub custodian_id: ActorId,
    pub location_id: LocationId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StockConservationRecord {
    pub key: StockConservationKey,
    pub quantity: u128,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountConservationRecord {
    pub account_id: FinancialAccountId,
    pub owner_id: ActorId,
    pub currency_id: CurrencyId,
    pub class: FinancialAccountClass,
    pub totals: AccountTotals,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrencyConservationRecord {
    pub definition: CurrencyDefinition,
    pub declared_supply: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StockHistoryIdentity {
    pub sequence: u64,
    pub cause_id: CausalId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonetaryHistoryIdentity {
    pub sequence: u64,
    pub cause_id: CausalId,
}

/// Identity-bearing watermarks retained across resolution changes.
///
/// These are intentionally more than raw counts. Transaction, settlement, and causal
/// identities make accidental history substitution harder to mistake for a harmless
/// LOD transition. Exact byte-level archival identity remains delegated to the
/// retained-detail evidence reference in v0.1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EconomicHistoryManifest {
    pub stock_events: Vec<StockHistoryIdentity>,
    pub journal_transactions: Vec<JournalTransactionId>,
    pub monetary_events: Vec<MonetaryHistoryIdentity>,
    pub settlements: Vec<SettlementId>,
}

/// Economically relevant state that must remain identical across a pure resolution
/// transition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EconomicConservationManifest {
    pub stock: Vec<StockConservationRecord>,
    pub accounts: Vec<AccountConservationRecord>,
    pub currencies: Vec<CurrencyConservationRecord>,
    pub history: EconomicHistoryManifest,
}

impl EconomicConservationManifest {
    fn capture(
        stock_ledger: &StockLedger,
        financial_book: &FinancialBook,
        registry: &FinancialRegistrySnapshot,
        settlement_ids: Vec<SettlementId>,
    ) -> Result<Self, ResolutionError> {
        stock_ledger.validate()?;
        financial_book.validate()?;
        registry.validate_against(financial_book)?;

        let mut grouped_stock: BTreeMap<StockConservationKey, u128> = BTreeMap::new();
        for lot in stock_ledger.lots() {
            let key = StockConservationKey {
                commodity_spec_id: lot.commodity_spec_id().clone(),
                owner_id: lot.owner_id().clone(),
                custodian_id: lot.custodian_id().clone(),
                location_id: lot.location_id().clone(),
            };
            let total = grouped_stock.entry(key).or_insert(0);
            *total = total
                .checked_add(u128::from(lot.quantity()))
                .ok_or(ResolutionError::ArithmeticOverflow)?;
        }
        let stock = grouped_stock
            .into_iter()
            .map(|(key, quantity)| StockConservationRecord { key, quantity })
            .collect();

        let mut accounts = Vec::with_capacity(registry.accounts.len());
        for account in &registry.accounts {
            let exact = financial_book
                .account(&account.account_id)
                .ok_or_else(|| ResolutionError::FinancialRegistryMismatch)?;
            if exact != account {
                return Err(ResolutionError::FinancialRegistryMismatch);
            }
            let totals = financial_book
                .account_totals(&account.account_id)
                .ok_or(ResolutionError::FinancialRegistryMismatch)?;
            accounts.push(AccountConservationRecord {
                account_id: account.account_id.clone(),
                owner_id: account.owner_id.clone(),
                currency_id: account.currency_id.clone(),
                class: account.class,
                totals,
            });
        }

        let mut currencies = Vec::with_capacity(registry.currencies.len());
        for currency in &registry.currencies {
            let exact = financial_book
                .currency(&currency.currency_id)
                .ok_or(ResolutionError::FinancialRegistryMismatch)?;
            if exact != currency {
                return Err(ResolutionError::FinancialRegistryMismatch);
            }
            let declared_supply = financial_book
                .monetary_supply(&currency.currency_id)
                .ok_or(ResolutionError::FinancialRegistryMismatch)?;
            currencies.push(CurrencyConservationRecord {
                definition: currency.clone(),
                declared_supply,
            });
        }

        let stock_events = stock_ledger
            .entries()
            .iter()
            .map(|entry| StockHistoryIdentity {
                sequence: entry.sequence,
                cause_id: stock_event_cause(&entry.event).clone(),
            })
            .collect();
        let journal_transactions = financial_book
            .journal_entries()
            .iter()
            .map(|entry| entry.transaction.transaction_id.clone())
            .collect();
        let monetary_events = financial_book
            .monetary_entries()
            .iter()
            .map(|entry| MonetaryHistoryIdentity {
                sequence: entry.sequence,
                cause_id: monetary_event_cause(&entry.event).clone(),
            })
            .collect();

        Ok(Self {
            stock,
            accounts,
            currencies,
            history: EconomicHistoryManifest {
                stock_events,
                journal_transactions,
                monetary_events,
                settlements: settlement_ids,
            },
        })
    }
}

/// Exact economic state supplied to a resolution transition.
///
/// `Financial` captures ECON-00/01 + ECON-02 state. `Settled` additionally binds the
/// complete ECON-02B settlement-ID sequence and validates the settlement ledger before
/// reading its current financial book.
pub enum ExactEconomicStateRef<'a> {
    Financial {
        stock_ledger: &'a StockLedger,
        financial_book: &'a FinancialBook,
        registry: &'a FinancialRegistrySnapshot,
    },
    Settled {
        stock_ledger: &'a StockLedger,
        settlement_ledger: &'a MonetarySettlementLedger,
        registry: &'a FinancialRegistrySnapshot,
    },
}

impl ExactEconomicStateRef<'_> {
    fn capture_manifest(&self) -> Result<EconomicConservationManifest, ResolutionError> {
        match self {
            Self::Financial {
                stock_ledger,
                financial_book,
                registry,
            } => EconomicConservationManifest::capture(
                stock_ledger,
                financial_book,
                registry,
                Vec::new(),
            ),
            Self::Settled {
                stock_ledger,
                settlement_ledger,
                registry,
            } => {
                settlement_ledger.validate()?;
                let settlement_ids = settlement_ledger
                    .entries()
                    .iter()
                    .map(|entry| entry.settlement.settlement_id.clone())
                    .collect();
                EconomicConservationManifest::capture(
                    stock_ledger,
                    settlement_ledger.financial_book(),
                    registry,
                    settlement_ids,
                )
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetailRetentionRef {
    pub retention_id: DetailRetentionId,
    pub source_snapshot_id: EconomicSnapshotId,
    /// External evidence identifying where/how exact detail was retained.
    ///
    /// V0.1 binds this identity but does not cryptographically verify the external
    /// archive or storage service.
    pub evidence_id: CausalId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EconomicDetailState {
    Exact {
        evidence_id: CausalId,
    },
    Aggregated {
        retention: Option<DetailRetentionRef>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EconomicResolutionSnapshot {
    snapshot_id: EconomicSnapshotId,
    tier: EconomicResolutionTier,
    generation: u64,
    manifest: EconomicConservationManifest,
    detail: EconomicDetailState,
}

impl EconomicResolutionSnapshot {
    pub fn snapshot_id(&self) -> &EconomicSnapshotId {
        &self.snapshot_id
    }

    pub const fn tier(&self) -> EconomicResolutionTier {
        self.tier
    }

    pub const fn generation(&self) -> u64 {
        self.generation
    }

    pub fn manifest(&self) -> &EconomicConservationManifest {
        &self.manifest
    }

    pub fn detail(&self) -> &EconomicDetailState {
        &self.detail
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolutionTransition {
    DemoteExact {
        target_tier: EconomicResolutionTier,
        retention: Option<DetailRetentionRef>,
    },
    CoarsenAggregate {
        target_tier: EconomicResolutionTier,
    },
    Promote {
        target_tier: EconomicResolutionTier,
        restoration: DetailRetentionRef,
        restored_detail_evidence_id: CausalId,
        restored_manifest: EconomicConservationManifest,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolutionLedgerEntry {
    pub sequence: u64,
    pub from_snapshot_id: EconomicSnapshotId,
    pub to_snapshot_id: EconomicSnapshotId,
    pub cause_id: CausalId,
    pub transition: ResolutionTransition,
}

/// Append-only reconciliation history for one frozen economic instant.
///
/// The economic conservation manifest may never change inside this ledger. If the
/// economy itself changes, callers must finish that economic mutation under the
/// appropriate ECON authority and capture a new resolution ledger/snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EconomicResolutionLedger {
    base_snapshot: EconomicResolutionSnapshot,
    current_snapshot: EconomicResolutionSnapshot,
    entries: Vec<ResolutionLedgerEntry>,
}

impl EconomicResolutionLedger {
    pub fn new(
        snapshot_id: EconomicSnapshotId,
        tier: EconomicResolutionTier,
        exact_detail_evidence_id: CausalId,
        state: ExactEconomicStateRef<'_>,
    ) -> Result<Self, ResolutionError> {
        let manifest = state.capture_manifest()?;
        let base_snapshot = EconomicResolutionSnapshot {
            snapshot_id,
            tier,
            generation: 0,
            manifest,
            detail: EconomicDetailState::Exact {
                evidence_id: exact_detail_evidence_id,
            },
        };
        Ok(Self {
            current_snapshot: base_snapshot.clone(),
            base_snapshot,
            entries: Vec::new(),
        })
    }

    pub fn base_snapshot(&self) -> &EconomicResolutionSnapshot {
        &self.base_snapshot
    }

    pub fn current_snapshot(&self) -> &EconomicResolutionSnapshot {
        &self.current_snapshot
    }

    pub fn entries(&self) -> &[ResolutionLedgerEntry] {
        &self.entries
    }

    pub fn validate(&self) -> Result<(), ResolutionError> {
        if self.entries.len() > MAX_RESOLUTION_TRANSITIONS {
            return Err(ResolutionError::ModelTooLarge);
        }
        let replayed = replay_resolution_history(&self.base_snapshot, &self.entries)?;
        if replayed != self.current_snapshot {
            return Err(ResolutionError::ResolutionHistoryMismatch);
        }
        Ok(())
    }

    /// Demote an exact representation to any strictly coarser tier.
    ///
    /// `retention=None` is an explicit irreversible discard under ECON-03 v0.1:
    /// promotion through this authority will later fail closed instead of inventing
    /// detail.
    pub fn demote_exact(
        &mut self,
        to_snapshot_id: EconomicSnapshotId,
        target_tier: EconomicResolutionTier,
        retention: Option<DetailRetentionRef>,
        cause_id: CausalId,
    ) -> Result<(), ResolutionError> {
        if !matches!(self.current_snapshot.detail, EconomicDetailState::Exact { .. }) {
            return Err(ResolutionError::ExactDetailRequired);
        }
        if !target_tier.is_coarser_than(self.current_snapshot.tier) {
            return Err(ResolutionError::InvalidDemotionDirection {
                from: self.current_snapshot.tier,
                to: target_tier,
            });
        }
        if let Some(retention) = &retention {
            if retention.source_snapshot_id != self.current_snapshot.snapshot_id {
                return Err(ResolutionError::RetentionSourceMismatch {
                    expected: self.current_snapshot.snapshot_id.clone(),
                    actual: retention.source_snapshot_id.clone(),
                });
            }
        }

        self.commit_transition(
            to_snapshot_id,
            cause_id,
            ResolutionTransition::DemoteExact {
                target_tier,
                retention,
            },
        )
    }

    /// Move an already-aggregated representation to a still coarser tier while
    /// carrying the exact same retention provenance forward unchanged.
    pub fn coarsen_aggregate(
        &mut self,
        to_snapshot_id: EconomicSnapshotId,
        target_tier: EconomicResolutionTier,
        cause_id: CausalId,
    ) -> Result<(), ResolutionError> {
        if !matches!(
            self.current_snapshot.detail,
            EconomicDetailState::Aggregated { .. }
        ) {
            return Err(ResolutionError::AggregatedDetailRequired);
        }
        if !target_tier.is_coarser_than(self.current_snapshot.tier) {
            return Err(ResolutionError::InvalidDemotionDirection {
                from: self.current_snapshot.tier,
                to: target_tier,
            });
        }
        self.commit_transition(
            to_snapshot_id,
            cause_id,
            ResolutionTransition::CoarsenAggregate { target_tier },
        )
    }

    /// Promote an aggregated representation only with the exact retained-detail
    /// reference and a supplied exact state that independently reconstructs the same
    /// conservation manifest.
    pub fn promote(
        &mut self,
        to_snapshot_id: EconomicSnapshotId,
        target_tier: EconomicResolutionTier,
        restoration: DetailRetentionRef,
        restored_detail_evidence_id: CausalId,
        restored_state: ExactEconomicStateRef<'_>,
        cause_id: CausalId,
    ) -> Result<(), ResolutionError> {
        if !target_tier.is_finer_than(self.current_snapshot.tier) {
            return Err(ResolutionError::InvalidPromotionDirection {
                from: self.current_snapshot.tier,
                to: target_tier,
            });
        }

        let expected_retention = match &self.current_snapshot.detail {
            EconomicDetailState::Aggregated {
                retention: Some(retention),
            } => retention,
            EconomicDetailState::Aggregated { retention: None } => {
                return Err(ResolutionError::DetailUnavailable)
            }
            EconomicDetailState::Exact { .. } => {
                return Err(ResolutionError::AggregatedDetailRequired)
            }
        };
        if expected_retention != &restoration {
            return Err(ResolutionError::RestorationReferenceMismatch);
        }

        let restored_manifest = restored_state.capture_manifest()?;
        if restored_manifest != self.current_snapshot.manifest {
            return Err(ResolutionError::ConservationMismatch);
        }

        self.commit_transition(
            to_snapshot_id,
            cause_id,
            ResolutionTransition::Promote {
                target_tier,
                restoration,
                restored_detail_evidence_id,
                restored_manifest,
            },
        )
    }

    fn commit_transition(
        &mut self,
        to_snapshot_id: EconomicSnapshotId,
        cause_id: CausalId,
        transition: ResolutionTransition,
    ) -> Result<(), ResolutionError> {
        if self.entries.len() >= MAX_RESOLUTION_TRANSITIONS {
            return Err(ResolutionError::ModelTooLarge);
        }
        if snapshot_id_exists(&self.base_snapshot, &self.entries, &to_snapshot_id) {
            return Err(ResolutionError::DuplicateSnapshotId {
                snapshot_id: to_snapshot_id,
            });
        }

        let sequence = next_sequence(self.entries.len())?;
        let entry = ResolutionLedgerEntry {
            sequence,
            from_snapshot_id: self.current_snapshot.snapshot_id.clone(),
            to_snapshot_id,
            cause_id,
            transition,
        };

        let mut candidate_entries = self.entries.clone();
        candidate_entries.push(entry);
        let candidate_snapshot = replay_resolution_history(&self.base_snapshot, &candidate_entries)?;

        self.entries = candidate_entries;
        self.current_snapshot = candidate_snapshot;
        Ok(())
    }
}

fn replay_resolution_history(
    base: &EconomicResolutionSnapshot,
    entries: &[ResolutionLedgerEntry],
) -> Result<EconomicResolutionSnapshot, ResolutionError> {
    if entries.len() > MAX_RESOLUTION_TRANSITIONS {
        return Err(ResolutionError::ModelTooLarge);
    }
    let mut current = base.clone();
    let mut snapshot_ids = BTreeSet::new();
    snapshot_ids.insert(base.snapshot_id.clone());

    for (index, entry) in entries.iter().enumerate() {
        let expected_sequence = next_sequence(index)?;
        if entry.sequence != expected_sequence {
            return Err(ResolutionError::InvalidResolutionSequence {
                expected: expected_sequence,
                actual: entry.sequence,
            });
        }
        if entry.from_snapshot_id != current.snapshot_id {
            return Err(ResolutionError::StaleResolutionSource {
                expected: current.snapshot_id.clone(),
                actual: entry.from_snapshot_id.clone(),
            });
        }
        if !snapshot_ids.insert(entry.to_snapshot_id.clone()) {
            return Err(ResolutionError::DuplicateSnapshotId {
                snapshot_id: entry.to_snapshot_id.clone(),
            });
        }

        let next_generation = current
            .generation
            .checked_add(1)
            .ok_or(ResolutionError::ArithmeticOverflow)?;

        let (target_tier, next_detail) = match &entry.transition {
            ResolutionTransition::DemoteExact {
                target_tier,
                retention,
            } => {
                if !matches!(current.detail, EconomicDetailState::Exact { .. }) {
                    return Err(ResolutionError::ExactDetailRequired);
                }
                if !target_tier.is_coarser_than(current.tier) {
                    return Err(ResolutionError::InvalidDemotionDirection {
                        from: current.tier,
                        to: *target_tier,
                    });
                }
                if let Some(retention) = retention {
                    if retention.source_snapshot_id != current.snapshot_id {
                        return Err(ResolutionError::RetentionSourceMismatch {
                            expected: current.snapshot_id.clone(),
                            actual: retention.source_snapshot_id.clone(),
                        });
                    }
                }
                (
                    *target_tier,
                    EconomicDetailState::Aggregated {
                        retention: retention.clone(),
                    },
                )
            }
            ResolutionTransition::CoarsenAggregate { target_tier } => {
                let retention = match &current.detail {
                    EconomicDetailState::Aggregated { retention } => retention.clone(),
                    EconomicDetailState::Exact { .. } => {
                        return Err(ResolutionError::AggregatedDetailRequired)
                    }
                };
                if !target_tier.is_coarser_than(current.tier) {
                    return Err(ResolutionError::InvalidDemotionDirection {
                        from: current.tier,
                        to: *target_tier,
                    });
                }
                (
                    *target_tier,
                    EconomicDetailState::Aggregated { retention },
                )
            }
            ResolutionTransition::Promote {
                target_tier,
                restoration,
                restored_detail_evidence_id,
                restored_manifest,
            } => {
                if !target_tier.is_finer_than(current.tier) {
                    return Err(ResolutionError::InvalidPromotionDirection {
                        from: current.tier,
                        to: *target_tier,
                    });
                }
                let expected = match &current.detail {
                    EconomicDetailState::Aggregated {
                        retention: Some(retention),
                    } => retention,
                    EconomicDetailState::Aggregated { retention: None } => {
                        return Err(ResolutionError::DetailUnavailable)
                    }
                    EconomicDetailState::Exact { .. } => {
                        return Err(ResolutionError::AggregatedDetailRequired)
                    }
                };
                if expected != restoration {
                    return Err(ResolutionError::RestorationReferenceMismatch);
                }
                if restored_manifest != &current.manifest {
                    return Err(ResolutionError::ConservationMismatch);
                }
                (
                    *target_tier,
                    EconomicDetailState::Exact {
                        evidence_id: restored_detail_evidence_id.clone(),
                    },
                )
            }
        };

        current = EconomicResolutionSnapshot {
            snapshot_id: entry.to_snapshot_id.clone(),
            tier: target_tier,
            generation: next_generation,
            manifest: current.manifest,
            detail: next_detail,
        };
    }

    Ok(current)
}

fn snapshot_id_exists(
    base: &EconomicResolutionSnapshot,
    entries: &[ResolutionLedgerEntry],
    snapshot_id: &EconomicSnapshotId,
) -> bool {
    &base.snapshot_id == snapshot_id
        || entries
            .iter()
            .any(|entry| &entry.to_snapshot_id == snapshot_id)
}

fn stock_event_cause(event: &StockEvent) -> &CausalId {
    match event {
        StockEvent::Establish { origin, .. } => match origin {
            StockOrigin::QualifiedInitial { evidence_id }
            | StockOrigin::IndustrialProduction { evidence_id, .. }
            | StockOrigin::IndustrialRecycling { evidence_id, .. }
            | StockOrigin::Salvage { evidence_id, .. } => evidence_id,
        },
        StockEvent::Split { cause_id, .. }
        | StockEvent::TransferOwnership { cause_id, .. }
        | StockEvent::TransferCustody { cause_id, .. }
        | StockEvent::Relocate { cause_id, .. } => cause_id,
        StockEvent::Deplete { cause, .. } => match cause {
            StockDepletionCause::IndustrialDemand { evidence_id, .. } => evidence_id,
            StockDepletionCause::TransformationInput { process_id } => process_id,
            StockDepletionCause::HouseholdConsumption { activity_id } => activity_id,
            StockDepletionCause::Loss { incident_id } => incident_id,
            StockDepletionCause::Disposal { record_id } => record_id,
        },
    }
}

fn monetary_event_cause(event: &MonetarySupplyEvent) -> &CausalId {
    match event {
        MonetarySupplyEvent::Issue { cause_id, .. }
        | MonetarySupplyEvent::Retire { cause_id, .. } => cause_id,
    }
}

fn next_sequence(len: usize) -> Result<u64, ResolutionError> {
    u64::try_from(len)
        .map_err(|_| ResolutionError::ArithmeticOverflow)?
        .checked_add(1)
        .ok_or(ResolutionError::ArithmeticOverflow)
}

fn validate_id(kind: &'static str, value: &str) -> Result<(), ResolutionError> {
    if value.is_empty() {
        return Err(ResolutionError::EmptyId { kind });
    }
    if value.len() > MAX_ID_LEN {
        return Err(ResolutionError::IdTooLong {
            kind,
            max_len: MAX_ID_LEN,
        });
    }
    if value.trim() != value {
        return Err(ResolutionError::IdHasSurroundingWhitespace { kind });
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolutionError {
    Economic(EconomicError),
    Financial(FinancialError),
    Settlement(SettlementError),
    EmptyId {
        kind: &'static str,
    },
    IdTooLong {
        kind: &'static str,
        max_len: usize,
    },
    IdHasSurroundingWhitespace {
        kind: &'static str,
    },
    DuplicateRegistryCurrency {
        currency_id: CurrencyId,
    },
    DuplicateRegistryAccount {
        account_id: FinancialAccountId,
    },
    FinancialRegistryMismatch,
    InvalidDemotionDirection {
        from: EconomicResolutionTier,
        to: EconomicResolutionTier,
    },
    InvalidPromotionDirection {
        from: EconomicResolutionTier,
        to: EconomicResolutionTier,
    },
    ExactDetailRequired,
    AggregatedDetailRequired,
    DetailUnavailable,
    RetentionSourceMismatch {
        expected: EconomicSnapshotId,
        actual: EconomicSnapshotId,
    },
    RestorationReferenceMismatch,
    ConservationMismatch,
    DuplicateSnapshotId {
        snapshot_id: EconomicSnapshotId,
    },
    InvalidResolutionSequence {
        expected: u64,
        actual: u64,
    },
    StaleResolutionSource {
        expected: EconomicSnapshotId,
        actual: EconomicSnapshotId,
    },
    ResolutionHistoryMismatch,
    ArithmeticOverflow,
    ModelTooLarge,
}

impl From<EconomicError> for ResolutionError {
    fn from(value: EconomicError) -> Self {
        Self::Economic(value)
    }
}

impl From<FinancialError> for ResolutionError {
    fn from(value: FinancialError) -> Self {
        Self::Financial(value)
    }
}

impl From<SettlementError> for ResolutionError {
    fn from(value: SettlementError) -> Self {
        Self::Settlement(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::economic::{LotId, StockLot};
    use crate::financial::{
        FinancialAccountClass, JournalTransaction, MonetaryAuthorityId, Posting, PostingSide,
    };
    use crate::settlement::{
        MonetarySettlement, MonetarySettlementLedger, SettlementAuthorizationRef,
    };

    fn actor(value: &str) -> ActorId {
        ActorId::new(value).unwrap()
    }

    fn cause(value: &str) -> CausalId {
        CausalId::new(value).unwrap()
    }

    fn snapshot_id(value: &str) -> EconomicSnapshotId {
        EconomicSnapshotId::new(value).unwrap()
    }

    fn retention_id(value: &str) -> DetailRetentionId {
        DetailRetentionId::new(value).unwrap()
    }

    fn currency_id(value: &str) -> CurrencyId {
        CurrencyId::new(value).unwrap()
    }

    fn account_id(value: &str) -> FinancialAccountId {
        FinancialAccountId::new(value).unwrap()
    }

    fn authority(value: &str) -> MonetaryAuthorityId {
        MonetaryAuthorityId::new(value).unwrap()
    }

    fn registry() -> FinancialRegistrySnapshot {
        FinancialRegistrySnapshot::new(
            vec![CurrencyDefinition {
                currency_id: currency_id("CR"),
                monetary_authority_id: authority("mint"),
                monetary_authority_actor_id: actor("treasury"),
                minor_unit_exponent: 2,
            }],
            vec![
                FinancialAccount {
                    account_id: account_id("alice-cash"),
                    owner_id: actor("alice"),
                    currency_id: currency_id("CR"),
                    class: FinancialAccountClass::Asset,
                },
                FinancialAccount {
                    account_id: account_id("issuance-equity"),
                    owner_id: actor("treasury"),
                    currency_id: currency_id("CR"),
                    class: FinancialAccountClass::Equity,
                },
                FinancialAccount {
                    account_id: account_id("unused-reserve"),
                    owner_id: actor("treasury"),
                    currency_id: currency_id("CR"),
                    class: FinancialAccountClass::Asset,
                },
            ],
        )
        .unwrap()
    }

    fn financial_book() -> FinancialBook {
        let registry = registry();
        FinancialBook::new(registry.currencies.clone(), registry.accounts.clone()).unwrap()
    }

    fn stock_ledger() -> StockLedger {
        let mut ledger = StockLedger::new();
        ledger
            .establish(
                StockLot::new(
                    LotId::new("steel-lot-a").unwrap(),
                    CommoditySpecId::new("structural-steel").unwrap(),
                    100,
                    actor("alice"),
                    actor("warehouse-a"),
                    LocationId::new("yard-a").unwrap(),
                )
                .unwrap(),
                StockOrigin::QualifiedInitial {
                    evidence_id: cause("stock-qualified"),
                },
            )
            .unwrap();
        ledger
    }

    fn exact_state<'a>(
        stock: &'a StockLedger,
        book: &'a FinancialBook,
        registry: &'a FinancialRegistrySnapshot,
    ) -> ExactEconomicStateRef<'a> {
        ExactEconomicStateRef::Financial {
            stock_ledger: stock,
            financial_book: book,
            registry,
        }
    }

    fn retention(source: &str, id: &str) -> DetailRetentionRef {
        DetailRetentionRef {
            retention_id: retention_id(id),
            source_snapshot_id: snapshot_id(source),
            evidence_id: cause(&format!("{id}-evidence")),
        }
    }

    #[test]
    fn demotion_preserves_complete_conservation_manifest() {
        let stock = stock_ledger();
        let book = financial_book();
        let registry = registry();
        let mut ledger = EconomicResolutionLedger::new(
            snapshot_id("active-0"),
            EconomicResolutionTier::ActiveSite,
            cause("exact-active"),
            exact_state(&stock, &book, &registry),
        )
        .unwrap();
        let before = ledger.current_snapshot().manifest().clone();

        ledger
            .demote_exact(
                snapshot_id("local-0"),
                EconomicResolutionTier::LocalRegion,
                Some(retention("active-0", "retain-active-0")),
                cause("demote-local"),
            )
            .unwrap();

        assert_eq!(ledger.current_snapshot().manifest(), &before);
        assert_eq!(ledger.current_snapshot().tier(), EconomicResolutionTier::LocalRegion);
        assert!(matches!(
            ledger.current_snapshot().detail(),
            EconomicDetailState::Aggregated { retention: Some(_) }
        ));
        ledger.validate().unwrap();
    }

    #[test]
    fn zero_balance_accounts_are_part_of_conserved_state() {
        let stock = stock_ledger();
        let book = financial_book();
        let complete = registry();
        let incomplete = FinancialRegistrySnapshot::new(
            complete.currencies.clone(),
            complete
                .accounts
                .iter()
                .filter(|account| account.account_id != account_id("unused-reserve"))
                .cloned()
                .collect(),
        )
        .unwrap();

        let error = EconomicResolutionLedger::new(
            snapshot_id("active-0"),
            EconomicResolutionTier::ActiveSite,
            cause("exact-active"),
            exact_state(&stock, &book, &incomplete),
        )
        .unwrap_err();
        assert_eq!(error, ResolutionError::FinancialRegistryMismatch);
    }

    #[test]
    fn discard_without_retention_is_irreversible_under_this_authority() {
        let stock = stock_ledger();
        let book = financial_book();
        let registry = registry();
        let mut ledger = EconomicResolutionLedger::new(
            snapshot_id("active-0"),
            EconomicResolutionTier::ActiveSite,
            cause("exact-active"),
            exact_state(&stock, &book, &registry),
        )
        .unwrap();
        ledger
            .demote_exact(
                snapshot_id("planet-0"),
                EconomicResolutionTier::PlanetaryAggregate,
                None,
                cause("discard-detail"),
            )
            .unwrap();

        let error = ledger
            .promote(
                snapshot_id("active-1"),
                EconomicResolutionTier::ActiveSite,
                retention("active-0", "nonexistent"),
                cause("restored-detail"),
                exact_state(&stock, &book, &registry),
                cause("promote"),
            )
            .unwrap_err();
        assert_eq!(error, ResolutionError::DetailUnavailable);
    }

    #[test]
    fn promotion_requires_exact_retention_reference() {
        let stock = stock_ledger();
        let book = financial_book();
        let registry = registry();
        let expected = retention("active-0", "retain-active-0");
        let mut ledger = EconomicResolutionLedger::new(
            snapshot_id("active-0"),
            EconomicResolutionTier::ActiveSite,
            cause("exact-active"),
            exact_state(&stock, &book, &registry),
        )
        .unwrap();
        ledger
            .demote_exact(
                snapshot_id("distant-0"),
                EconomicResolutionTier::DistantRegion,
                Some(expected),
                cause("demote"),
            )
            .unwrap();

        let error = ledger
            .promote(
                snapshot_id("active-1"),
                EconomicResolutionTier::ActiveSite,
                retention("active-0", "wrong-retention"),
                cause("restored-detail"),
                exact_state(&stock, &book, &registry),
                cause("promote"),
            )
            .unwrap_err();
        assert_eq!(error, ResolutionError::RestorationReferenceMismatch);
    }

    #[test]
    fn promotion_rejects_stock_drift_instead_of_hiding_it_as_lod() {
        let stock = stock_ledger();
        let book = financial_book();
        let registry = registry();
        let retained = retention("active-0", "retain-active-0");
        let mut ledger = EconomicResolutionLedger::new(
            snapshot_id("active-0"),
            EconomicResolutionTier::ActiveSite,
            cause("exact-active"),
            exact_state(&stock, &book, &registry),
        )
        .unwrap();
        ledger
            .demote_exact(
                snapshot_id("planet-0"),
                EconomicResolutionTier::PlanetaryAggregate,
                Some(retained.clone()),
                cause("demote"),
            )
            .unwrap();

        let mut drifted_stock = stock.clone();
        drifted_stock
            .deplete(
                LotId::new("steel-lot-a").unwrap(),
                1,
                StockDepletionCause::Loss {
                    incident_id: cause("hidden-loss"),
                },
            )
            .unwrap();

        let before = ledger.clone();
        let error = ledger
            .promote(
                snapshot_id("active-1"),
                EconomicResolutionTier::ActiveSite,
                retained,
                cause("restored-detail"),
                exact_state(&drifted_stock, &book, &registry),
                cause("promote"),
            )
            .unwrap_err();
        assert_eq!(error, ResolutionError::ConservationMismatch);
        assert_eq!(ledger, before);
    }

    #[test]
    fn promotion_rejects_financial_drift() {
        let stock = stock_ledger();
        let book = financial_book();
        let registry = registry();
        let retained = retention("active-0", "retain-active-0");
        let mut ledger = EconomicResolutionLedger::new(
            snapshot_id("active-0"),
            EconomicResolutionTier::ActiveSite,
            cause("exact-active"),
            exact_state(&stock, &book, &registry),
        )
        .unwrap();
        ledger
            .demote_exact(
                snapshot_id("distant-0"),
                EconomicResolutionTier::DistantRegion,
                Some(retained.clone()),
                cause("demote"),
            )
            .unwrap();

        let mut drifted_book = book.clone();
        drifted_book
            .post_transaction(
                JournalTransaction::new(
                    crate::financial::JournalTransactionId::new("hidden-journal").unwrap(),
                    cause("hidden-financial-change"),
                    currency_id("CR"),
                    vec![
                        Posting {
                            account_id: account_id("alice-cash"),
                            side: PostingSide::Debit,
                            amount: 10,
                        },
                        Posting {
                            account_id: account_id("issuance-equity"),
                            side: PostingSide::Credit,
                            amount: 10,
                        },
                    ],
                )
                .unwrap(),
            )
            .unwrap();

        let error = ledger
            .promote(
                snapshot_id("active-1"),
                EconomicResolutionTier::ActiveSite,
                retained,
                cause("restored-detail"),
                exact_state(&stock, &drifted_book, &registry),
                cause("promote"),
            )
            .unwrap_err();
        assert_eq!(error, ResolutionError::ConservationMismatch);
    }

    #[test]
    fn aggregate_coarsening_carries_retention_without_rebinding_it() {
        let stock = stock_ledger();
        let book = financial_book();
        let registry = registry();
        let retained = retention("active-0", "retain-active-0");
        let mut ledger = EconomicResolutionLedger::new(
            snapshot_id("active-0"),
            EconomicResolutionTier::ActiveSite,
            cause("exact-active"),
            exact_state(&stock, &book, &registry),
        )
        .unwrap();
        ledger
            .demote_exact(
                snapshot_id("local-0"),
                EconomicResolutionTier::LocalRegion,
                Some(retained.clone()),
                cause("demote-local"),
            )
            .unwrap();
        ledger
            .coarsen_aggregate(
                snapshot_id("planet-0"),
                EconomicResolutionTier::PlanetaryAggregate,
                cause("coarsen-planet"),
            )
            .unwrap();

        assert_eq!(
            ledger.current_snapshot().detail(),
            &EconomicDetailState::Aggregated {
                retention: Some(retained.clone())
            }
        );

        ledger
            .promote(
                snapshot_id("active-1"),
                EconomicResolutionTier::ActiveSite,
                retained,
                cause("restored-detail"),
                exact_state(&stock, &book, &registry),
                cause("promote-active"),
            )
            .unwrap();
        assert!(matches!(
            ledger.current_snapshot().detail(),
            EconomicDetailState::Exact { .. }
        ));
        ledger.validate().unwrap();
    }

    #[test]
    fn snapshot_ids_cannot_be_reused() {
        let stock = stock_ledger();
        let book = financial_book();
        let registry = registry();
        let mut ledger = EconomicResolutionLedger::new(
            snapshot_id("active-0"),
            EconomicResolutionTier::ActiveSite,
            cause("exact-active"),
            exact_state(&stock, &book, &registry),
        )
        .unwrap();
        let error = ledger
            .demote_exact(
                snapshot_id("active-0"),
                EconomicResolutionTier::LocalRegion,
                Some(retention("active-0", "retain-active-0")),
                cause("demote"),
            )
            .unwrap_err();
        assert!(matches!(error, ResolutionError::DuplicateSnapshotId { .. }));
    }

    #[test]
    fn retention_must_bind_the_exact_source_snapshot() {
        let stock = stock_ledger();
        let book = financial_book();
        let registry = registry();
        let mut ledger = EconomicResolutionLedger::new(
            snapshot_id("active-0"),
            EconomicResolutionTier::ActiveSite,
            cause("exact-active"),
            exact_state(&stock, &book, &registry),
        )
        .unwrap();
        let error = ledger
            .demote_exact(
                snapshot_id("local-0"),
                EconomicResolutionTier::LocalRegion,
                Some(retention("some-other-snapshot", "retain")),
                cause("demote"),
            )
            .unwrap_err();
        assert!(matches!(
            error,
            ResolutionError::RetentionSourceMismatch { .. }
        ));
    }

    #[test]
    fn settled_state_binds_settlement_identity_sequence() {
        let stock = stock_ledger();
        let registry = registry();
        let mut settlement_ledger = MonetarySettlementLedger::new(financial_book()).unwrap();
        let issue_cause = cause("issue-100");
        settlement_ledger
            .settle(MonetarySettlement {
                settlement_id: SettlementId::new("settlement-1").unwrap(),
                transaction: JournalTransaction::new(
                    crate::financial::JournalTransactionId::new("journal-issue-1").unwrap(),
                    issue_cause.clone(),
                    currency_id("CR"),
                    vec![
                        Posting {
                            account_id: account_id("alice-cash"),
                            side: PostingSide::Debit,
                            amount: 100,
                        },
                        Posting {
                            account_id: account_id("issuance-equity"),
                            side: PostingSide::Credit,
                            amount: 100,
                        },
                    ],
                )
                .unwrap(),
                monetary_event: MonetarySupplyEvent::Issue {
                    currency_id: currency_id("CR"),
                    authority_id: authority("mint"),
                    amount: 100,
                    beneficiary_actor_id: actor("alice"),
                    cause_id: issue_cause.clone(),
                },
                settlement_account_id: account_id("alice-cash"),
                authorization: SettlementAuthorizationRef {
                    monetary_authority_id: authority("mint"),
                    authority_actor_id: actor("treasury"),
                    evidence_id: cause("mint-signoff"),
                },
            })
            .unwrap();

        let ledger = EconomicResolutionLedger::new(
            snapshot_id("active-settled"),
            EconomicResolutionTier::ActiveSite,
            cause("exact-settled"),
            ExactEconomicStateRef::Settled {
                stock_ledger: &stock,
                settlement_ledger: &settlement_ledger,
                registry: &registry,
            },
        )
        .unwrap();

        assert_eq!(
            ledger.current_snapshot().manifest().history.settlements,
            vec![SettlementId::new("settlement-1").unwrap()]
        );
        assert_eq!(
            ledger.current_snapshot().manifest().currencies[0].declared_supply,
            100
        );
    }

    #[test]
    fn corrupted_resolution_history_is_detected() {
        let stock = stock_ledger();
        let book = financial_book();
        let registry = registry();
        let mut ledger = EconomicResolutionLedger::new(
            snapshot_id("active-0"),
            EconomicResolutionTier::ActiveSite,
            cause("exact-active"),
            exact_state(&stock, &book, &registry),
        )
        .unwrap();
        ledger
            .demote_exact(
                snapshot_id("local-0"),
                EconomicResolutionTier::LocalRegion,
                Some(retention("active-0", "retain-active-0")),
                cause("demote"),
            )
            .unwrap();
        ledger.entries[0].sequence = 2;
        assert_eq!(
            ledger.validate(),
            Err(ResolutionError::InvalidResolutionSequence {
                expected: 1,
                actual: 2
            })
        );
    }
}
