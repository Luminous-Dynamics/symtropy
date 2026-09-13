// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Authoritative partition/merge reconciliation for ECON-03 manifests.
//!
//! Regional simulation must not duplicate economic authority. ECON-03B therefore
//! partitions authoritative stock groups and financial accounts exactly once while
//! keeping currency supply and canonical economic history in one shared-global
//! authority. Read-only regional mirrors may exist elsewhere, but they are not part
//! of this authoritative partition set.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::economic_resolution::{
    AccountConservationRecord, CurrencyConservationRecord, EconomicConservationManifest,
    EconomicHistoryManifest, EconomicSnapshotId, StockConservationKey, StockConservationRecord,
};
use crate::financial::FinancialAccountId;

const MAX_ID_LEN: usize = 256;
const MAX_PARTITIONS: usize = 65_536;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EconomicPartitionId(String);

impl EconomicPartitionId {
    pub fn new(value: impl Into<String>) -> Result<Self, PartitionError> {
        let value = value.into();
        validate_id("economic-partition", &value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for EconomicPartitionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Complete placement plan for authoritative partitionable state.
///
/// Each stock conservation key and each financial account ID may appear exactly once
/// in the plan. Currency definitions/supply and canonical histories are deliberately
/// absent because they remain shared-global authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EconomicPartitionPlan {
    partition_ids: Vec<EconomicPartitionId>,
    stock_assignments: BTreeMap<StockConservationKey, EconomicPartitionId>,
    account_assignments: BTreeMap<FinancialAccountId, EconomicPartitionId>,
}

impl EconomicPartitionPlan {
    pub fn new(
        mut partition_ids: Vec<EconomicPartitionId>,
        stock_assignments: Vec<(StockConservationKey, EconomicPartitionId)>,
        account_assignments: Vec<(FinancialAccountId, EconomicPartitionId)>,
    ) -> Result<Self, PartitionError> {
        if partition_ids.is_empty() {
            return Err(PartitionError::NoPartitions);
        }
        if partition_ids.len() > MAX_PARTITIONS {
            return Err(PartitionError::ModelTooLarge);
        }
        partition_ids.sort();
        for pair in partition_ids.windows(2) {
            if pair[0] == pair[1] {
                return Err(PartitionError::DuplicatePartitionId {
                    partition_id: pair[0].clone(),
                });
            }
        }

        let mut stock_map = BTreeMap::new();
        for (key, partition_id) in stock_assignments {
            if stock_map.insert(key.clone(), partition_id).is_some() {
                return Err(PartitionError::DuplicateStockAssignment { key });
            }
        }

        let mut account_map = BTreeMap::new();
        for (account_id, partition_id) in account_assignments {
            if account_map
                .insert(account_id.clone(), partition_id)
                .is_some()
            {
                return Err(PartitionError::DuplicateAccountAssignment { account_id });
            }
        }

        Ok(Self {
            partition_ids,
            stock_assignments: stock_map,
            account_assignments: account_map,
        })
    }

    pub fn partition_ids(&self) -> &[EconomicPartitionId] {
        &self.partition_ids
    }

    fn validate_against(
        &self,
        source: &EconomicConservationManifest,
    ) -> Result<(), PartitionError> {
        let known: BTreeSet<_> = self.partition_ids.iter().cloned().collect();
        for partition_id in self
            .stock_assignments
            .values()
            .chain(self.account_assignments.values())
        {
            if !known.contains(partition_id) {
                return Err(PartitionError::UnknownPartition {
                    partition_id: partition_id.clone(),
                });
            }
        }

        let source_stock: BTreeSet<_> = source.stock.iter().map(|r| r.key.clone()).collect();
        let planned_stock: BTreeSet<_> = self.stock_assignments.keys().cloned().collect();
        if source_stock != planned_stock {
            return Err(PartitionError::IncompleteStockAssignment);
        }

        let source_accounts: BTreeSet<_> = source
            .accounts
            .iter()
            .map(|r| r.account_id.clone())
            .collect();
        let planned_accounts: BTreeSet<_> = self.account_assignments.keys().cloned().collect();
        if source_accounts != planned_accounts {
            return Err(PartitionError::IncompleteAccountAssignment);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EconomicPartitionManifest {
    partition_id: EconomicPartitionId,
    stock: Vec<StockConservationRecord>,
    accounts: Vec<AccountConservationRecord>,
}

impl EconomicPartitionManifest {
    pub fn partition_id(&self) -> &EconomicPartitionId {
        &self.partition_id
    }

    pub fn stock(&self) -> &[StockConservationRecord] {
        &self.stock
    }

    pub fn accounts(&self) -> &[AccountConservationRecord] {
        &self.accounts
    }
}

/// State that remains authoritative exactly once for the complete partition set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SharedGlobalEconomicState {
    currencies: Vec<CurrencyConservationRecord>,
    history: EconomicHistoryManifest,
}

impl SharedGlobalEconomicState {
    pub fn currencies(&self) -> &[CurrencyConservationRecord] {
        &self.currencies
    }

    pub fn history(&self) -> &EconomicHistoryManifest {
        &self.history
    }
}

/// One authoritative decomposition of an ECON-03 conservation manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EconomicPartitionSet {
    source_snapshot_id: EconomicSnapshotId,
    declared_partitions: Vec<EconomicPartitionId>,
    partitions: Vec<EconomicPartitionManifest>,
    shared_global: SharedGlobalEconomicState,
}

impl EconomicPartitionSet {
    pub fn partition(
        source_snapshot_id: EconomicSnapshotId,
        source: &EconomicConservationManifest,
        plan: &EconomicPartitionPlan,
    ) -> Result<Self, PartitionError> {
        validate_manifest_structure(source)?;
        plan.validate_against(source)?;

        let mut partition_map: BTreeMap<EconomicPartitionId, EconomicPartitionManifest> = plan
            .partition_ids
            .iter()
            .cloned()
            .map(|partition_id| {
                (
                    partition_id.clone(),
                    EconomicPartitionManifest {
                        partition_id,
                        stock: Vec::new(),
                        accounts: Vec::new(),
                    },
                )
            })
            .collect();

        for record in &source.stock {
            let partition_id = plan
                .stock_assignments
                .get(&record.key)
                .ok_or(PartitionError::IncompleteStockAssignment)?;
            partition_map
                .get_mut(partition_id)
                .ok_or_else(|| PartitionError::UnknownPartition {
                    partition_id: partition_id.clone(),
                })?
                .stock
                .push(record.clone());
        }

        for record in &source.accounts {
            let partition_id = plan
                .account_assignments
                .get(&record.account_id)
                .ok_or(PartitionError::IncompleteAccountAssignment)?;
            partition_map
                .get_mut(partition_id)
                .ok_or_else(|| PartitionError::UnknownPartition {
                    partition_id: partition_id.clone(),
                })?
                .accounts
                .push(record.clone());
        }

        let set = Self {
            source_snapshot_id,
            declared_partitions: plan.partition_ids.clone(),
            partitions: partition_map.into_values().collect(),
            shared_global: SharedGlobalEconomicState {
                currencies: source.currencies.clone(),
                history: source.history.clone(),
            },
        };
        set.validate_against(set.source_snapshot_id(), source)?;
        Ok(set)
    }

    pub fn source_snapshot_id(&self) -> &EconomicSnapshotId {
        &self.source_snapshot_id
    }

    pub fn partitions(&self) -> &[EconomicPartitionManifest] {
        &self.partitions
    }

    pub fn shared_global(&self) -> &SharedGlobalEconomicState {
        &self.shared_global
    }

    pub fn partition_by_id(
        &self,
        partition_id: &EconomicPartitionId,
    ) -> Option<&EconomicPartitionManifest> {
        self.partitions
            .binary_search_by(|partition| partition.partition_id.cmp(partition_id))
            .ok()
            .map(|index| &self.partitions[index])
    }

    /// Rebuild one global ECON-03 manifest from authoritative partitions plus the
    /// single shared-global currency/history authority.
    pub fn reconstruct_manifest(&self) -> Result<EconomicConservationManifest, PartitionError> {
        validate_partition_identity_set(&self.declared_partitions, &self.partitions)?;

        let mut stock = BTreeMap::new();
        let mut accounts = BTreeMap::new();
        for partition in &self.partitions {
            for record in &partition.stock {
                if stock.insert(record.key.clone(), record.clone()).is_some() {
                    return Err(PartitionError::DuplicateAuthoritativeStock {
                        key: record.key.clone(),
                    });
                }
            }
            for record in &partition.accounts {
                if accounts
                    .insert(record.account_id.clone(), record.clone())
                    .is_some()
                {
                    return Err(PartitionError::DuplicateAuthoritativeAccount {
                        account_id: record.account_id.clone(),
                    });
                }
            }
        }

        let reconstructed = EconomicConservationManifest {
            stock: stock.into_values().collect(),
            accounts: accounts.into_values().collect(),
            currencies: self.shared_global.currencies.clone(),
            history: self.shared_global.history.clone(),
        };
        validate_manifest_structure(&reconstructed)?;
        Ok(reconstructed)
    }

    pub fn validate_against(
        &self,
        source_snapshot_id: &EconomicSnapshotId,
        source: &EconomicConservationManifest,
    ) -> Result<(), PartitionError> {
        if &self.source_snapshot_id != source_snapshot_id {
            return Err(PartitionError::SourceSnapshotMismatch {
                expected: self.source_snapshot_id.clone(),
                actual: source_snapshot_id.clone(),
            });
        }
        validate_manifest_structure(source)?;
        let reconstructed = self.reconstruct_manifest()?;
        if reconstructed != *source {
            return Err(PartitionError::PartitionReconciliationMismatch);
        }
        Ok(())
    }
}

fn validate_partition_identity_set(
    declared: &[EconomicPartitionId],
    partitions: &[EconomicPartitionManifest],
) -> Result<(), PartitionError> {
    if declared.is_empty() || partitions.is_empty() {
        return Err(PartitionError::NoPartitions);
    }
    if declared.len() > MAX_PARTITIONS || partitions.len() > MAX_PARTITIONS {
        return Err(PartitionError::ModelTooLarge);
    }
    if declared.len() != partitions.len() {
        return Err(PartitionError::PartitionIdentityMismatch);
    }
    for pair in declared.windows(2) {
        if pair[0] >= pair[1] {
            return Err(PartitionError::PartitionIdentityMismatch);
        }
    }
    for pair in partitions.windows(2) {
        if pair[0].partition_id >= pair[1].partition_id {
            return Err(PartitionError::PartitionIdentityMismatch);
        }
    }
    if declared
        .iter()
        .zip(partitions.iter())
        .any(|(declared_id, partition)| declared_id != &partition.partition_id)
    {
        return Err(PartitionError::PartitionIdentityMismatch);
    }
    Ok(())
}

fn validate_manifest_structure(manifest: &EconomicConservationManifest) -> Result<(), PartitionError> {
    for record in &manifest.stock {
        if record.quantity == 0 {
            return Err(PartitionError::ZeroStockQuantity {
                key: record.key.clone(),
            });
        }
    }
    for pair in manifest.stock.windows(2) {
        if pair[0].key >= pair[1].key {
            return Err(PartitionError::NonCanonicalStockManifest);
        }
    }
    for pair in manifest.accounts.windows(2) {
        if pair[0].account_id >= pair[1].account_id {
            return Err(PartitionError::NonCanonicalAccountManifest);
        }
    }
    for pair in manifest.currencies.windows(2) {
        if pair[0].definition.currency_id >= pair[1].definition.currency_id {
            return Err(PartitionError::NonCanonicalCurrencyManifest);
        }
    }

    let currency_ids: BTreeSet<_> = manifest
        .currencies
        .iter()
        .map(|record| record.definition.currency_id.clone())
        .collect();
    for account in &manifest.accounts {
        if !currency_ids.contains(&account.currency_id) {
            return Err(PartitionError::UnknownAccountCurrency {
                account_id: account.account_id.clone(),
            });
        }
    }

    for (index, event) in manifest.history.stock_events.iter().enumerate() {
        let expected = history_sequence(index)?;
        if event.sequence != expected {
            return Err(PartitionError::InvalidStockHistorySequence {
                expected,
                actual: event.sequence,
            });
        }
    }
    for (index, event) in manifest.history.monetary_events.iter().enumerate() {
        let expected = history_sequence(index)?;
        if event.sequence != expected {
            return Err(PartitionError::InvalidMonetaryHistorySequence {
                expected,
                actual: event.sequence,
            });
        }
    }

    let mut journal_ids = BTreeSet::new();
    for transaction_id in &manifest.history.journal_transactions {
        if !journal_ids.insert(transaction_id.clone()) {
            return Err(PartitionError::DuplicateJournalIdentity);
        }
    }
    let mut settlement_ids = BTreeSet::new();
    for settlement_id in &manifest.history.settlements {
        if !settlement_ids.insert(settlement_id.clone()) {
            return Err(PartitionError::DuplicateSettlementIdentity);
        }
    }
    Ok(())
}

fn history_sequence(index: usize) -> Result<u64, PartitionError> {
    u64::try_from(index)
        .map_err(|_| PartitionError::ArithmeticOverflow)?
        .checked_add(1)
        .ok_or(PartitionError::ArithmeticOverflow)
}

fn validate_id(kind: &'static str, value: &str) -> Result<(), PartitionError> {
    if value.is_empty() {
        return Err(PartitionError::EmptyId { kind });
    }
    if value.len() > MAX_ID_LEN {
        return Err(PartitionError::IdTooLong {
            kind,
            max_len: MAX_ID_LEN,
        });
    }
    if value.trim() != value {
        return Err(PartitionError::IdHasSurroundingWhitespace { kind });
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PartitionError {
    EmptyId { kind: &'static str },
    IdTooLong { kind: &'static str, max_len: usize },
    IdHasSurroundingWhitespace { kind: &'static str },
    NoPartitions,
    ModelTooLarge,
    DuplicatePartitionId { partition_id: EconomicPartitionId },
    DuplicateStockAssignment { key: StockConservationKey },
    DuplicateAccountAssignment { account_id: FinancialAccountId },
    UnknownPartition { partition_id: EconomicPartitionId },
    IncompleteStockAssignment,
    IncompleteAccountAssignment,
    DuplicateAuthoritativeStock { key: StockConservationKey },
    DuplicateAuthoritativeAccount { account_id: FinancialAccountId },
    PartitionIdentityMismatch,
    SourceSnapshotMismatch {
        expected: EconomicSnapshotId,
        actual: EconomicSnapshotId,
    },
    PartitionReconciliationMismatch,
    ZeroStockQuantity { key: StockConservationKey },
    NonCanonicalStockManifest,
    NonCanonicalAccountManifest,
    NonCanonicalCurrencyManifest,
    UnknownAccountCurrency { account_id: FinancialAccountId },
    InvalidStockHistorySequence { expected: u64, actual: u64 },
    InvalidMonetaryHistorySequence { expected: u64, actual: u64 },
    DuplicateJournalIdentity,
    DuplicateSettlementIdentity,
    ArithmeticOverflow,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::economic::{
        ActorId, CausalId, CommoditySpecId, LocationId, LotId, StockLedger, StockLot, StockOrigin,
    };
    use crate::economic_resolution::{
        EconomicResolutionLedger, EconomicResolutionTier, ExactEconomicStateRef,
        FinancialRegistrySnapshot,
    };
    use crate::financial::{
        CurrencyDefinition, CurrencyId, FinancialAccount, FinancialAccountClass, FinancialBook,
        MonetaryAuthorityId,
    };

    fn actor(value: &str) -> ActorId {
        ActorId::new(value).unwrap()
    }

    fn cause(value: &str) -> CausalId {
        CausalId::new(value).unwrap()
    }

    fn currency(value: &str) -> CurrencyId {
        CurrencyId::new(value).unwrap()
    }

    fn account(value: &str) -> FinancialAccountId {
        FinancialAccountId::new(value).unwrap()
    }

    fn partition(value: &str) -> EconomicPartitionId {
        EconomicPartitionId::new(value).unwrap()
    }

    fn source_snapshot() -> (EconomicSnapshotId, EconomicConservationManifest) {
        let registry = FinancialRegistrySnapshot::new(
            vec![CurrencyDefinition {
                currency_id: currency("CR"),
                monetary_authority_id: MonetaryAuthorityId::new("mint").unwrap(),
                monetary_authority_actor_id: actor("treasury"),
                minor_unit_exponent: 2,
            }],
            vec![
                FinancialAccount {
                    account_id: account("north-account"),
                    owner_id: actor("north-firm"),
                    currency_id: currency("CR"),
                    class: FinancialAccountClass::Asset,
                },
                FinancialAccount {
                    account_id: account("south-account"),
                    owner_id: actor("south-firm"),
                    currency_id: currency("CR"),
                    class: FinancialAccountClass::Asset,
                },
            ],
        )
        .unwrap();
        let mut book = FinancialBook::new(
            registry.currencies().to_vec(),
            registry.accounts().to_vec(),
        )
        .unwrap();
        book.issue_currency(
            currency("CR"),
            MonetaryAuthorityId::new("mint").unwrap(),
            100,
            actor("treasury"),
            cause("issue-100"),
        )
        .unwrap();

        let mut stock = StockLedger::new();
        stock
            .establish(
                StockLot::new(
                    LotId::new("north-steel").unwrap(),
                    CommoditySpecId::new("steel").unwrap(),
                    40,
                    actor("north-firm"),
                    actor("north-warehouse"),
                    LocationId::new("north-yard").unwrap(),
                )
                .unwrap(),
                StockOrigin::QualifiedInitial {
                    evidence_id: cause("north-stock"),
                },
            )
            .unwrap();
        stock
            .establish(
                StockLot::new(
                    LotId::new("south-steel").unwrap(),
                    CommoditySpecId::new("steel").unwrap(),
                    60,
                    actor("south-firm"),
                    actor("south-warehouse"),
                    LocationId::new("south-yard").unwrap(),
                )
                .unwrap(),
                StockOrigin::QualifiedInitial {
                    evidence_id: cause("south-stock"),
                },
            )
            .unwrap();

        let snapshot_id = EconomicSnapshotId::new("global-0").unwrap();
        let ledger = EconomicResolutionLedger::new(
            snapshot_id.clone(),
            EconomicResolutionTier::ActiveSite,
            cause("exact-global"),
            ExactEconomicStateRef::Financial {
                stock_ledger: &stock,
                financial_book: &book,
                registry: &registry,
            },
        )
        .unwrap();
        (snapshot_id, ledger.current_snapshot().manifest().clone())
    }

    fn valid_plan(source: &EconomicConservationManifest) -> EconomicPartitionPlan {
        let north = partition("north");
        let south = partition("south");
        EconomicPartitionPlan::new(
            vec![north.clone(), south.clone()],
            source
                .stock
                .iter()
                .map(|record| {
                    let target = if record.key.location_id.as_str().starts_with("north") {
                        north.clone()
                    } else {
                        south.clone()
                    };
                    (record.key.clone(), target)
                })
                .collect(),
            source
                .accounts
                .iter()
                .map(|record| {
                    let target = if record.account_id.as_str().starts_with("north") {
                        north.clone()
                    } else {
                        south.clone()
                    };
                    (record.account_id.clone(), target)
                })
                .collect(),
        )
        .unwrap()
    }

    #[test]
    fn split_then_merge_reconstructs_exact_global_manifest() {
        let (snapshot_id, source) = source_snapshot();
        let set = EconomicPartitionSet::partition(snapshot_id.clone(), &source, &valid_plan(&source))
            .unwrap();
        assert_eq!(set.partitions().len(), 2);
        assert_eq!(set.reconstruct_manifest().unwrap(), source);
        set.validate_against(&snapshot_id, &source).unwrap();
    }

    #[test]
    fn currency_supply_exists_once_in_shared_global_state() {
        let (snapshot_id, source) = source_snapshot();
        let set = EconomicPartitionSet::partition(snapshot_id, &source, &valid_plan(&source)).unwrap();
        assert_eq!(set.shared_global().currencies().len(), 1);
        assert_eq!(set.shared_global().currencies()[0].declared_supply, 100);
        assert!(set.partitions().iter().all(|partition| {
            partition.stock().iter().all(|_| true) && partition.accounts().iter().all(|_| true)
        }));
    }

    #[test]
    fn missing_stock_assignment_fails_closed() {
        let (snapshot_id, source) = source_snapshot();
        let north = partition("north");
        let south = partition("south");
        let plan = EconomicPartitionPlan::new(
            vec![north.clone(), south.clone()],
            vec![(source.stock[0].key.clone(), north)],
            source
                .accounts
                .iter()
                .map(|record| (record.account_id.clone(), south.clone()))
                .collect(),
        )
        .unwrap();
        assert_eq!(
            EconomicPartitionSet::partition(snapshot_id, &source, &plan).unwrap_err(),
            PartitionError::IncompleteStockAssignment
        );
    }

    #[test]
    fn zero_balance_account_still_requires_exactly_one_partition() {
        let (snapshot_id, source) = source_snapshot();
        let north = partition("north");
        let south = partition("south");
        let plan = EconomicPartitionPlan::new(
            vec![north.clone(), south.clone()],
            source
                .stock
                .iter()
                .map(|record| (record.key.clone(), north.clone()))
                .collect(),
            vec![(source.accounts[0].account_id.clone(), south)],
        )
        .unwrap();
        assert_eq!(
            EconomicPartitionSet::partition(snapshot_id, &source, &plan).unwrap_err(),
            PartitionError::IncompleteAccountAssignment
        );
    }

    #[test]
    fn assignment_to_unknown_partition_fails() {
        let (snapshot_id, source) = source_snapshot();
        let mut plan = valid_plan(&source);
        let first_key = source.stock[0].key.clone();
        plan.stock_assignments
            .insert(first_key, partition("not-declared"));
        assert!(matches!(
            EconomicPartitionSet::partition(snapshot_id, &source, &plan).unwrap_err(),
            PartitionError::UnknownPartition { .. }
        ));
    }

    #[test]
    fn duplicate_authoritative_stock_across_regions_is_detected() {
        let (snapshot_id, source) = source_snapshot();
        let mut set = EconomicPartitionSet::partition(snapshot_id.clone(), &source, &valid_plan(&source))
            .unwrap();
        let duplicate = set.partitions[0].stock[0].clone();
        set.partitions[1].stock.push(duplicate);
        assert!(matches!(
            set.validate_against(&snapshot_id, &source).unwrap_err(),
            PartitionError::DuplicateAuthoritativeStock { .. }
        ));
    }

    #[test]
    fn duplicate_authoritative_account_across_regions_is_detected() {
        let (snapshot_id, source) = source_snapshot();
        let mut set = EconomicPartitionSet::partition(snapshot_id.clone(), &source, &valid_plan(&source))
            .unwrap();
        let duplicate = set.partitions[0].accounts[0].clone();
        set.partitions[1].accounts.push(duplicate);
        assert!(matches!(
            set.validate_against(&snapshot_id, &source).unwrap_err(),
            PartitionError::DuplicateAuthoritativeAccount { .. }
        ));
    }

    #[test]
    fn partition_set_is_bound_to_exact_source_snapshot() {
        let (snapshot_id, source) = source_snapshot();
        let set = EconomicPartitionSet::partition(snapshot_id, &source, &valid_plan(&source)).unwrap();
        assert!(matches!(
            set.validate_against(&EconomicSnapshotId::new("other-snapshot").unwrap(), &source)
                .unwrap_err(),
            PartitionError::SourceSnapshotMismatch { .. }
        ));
    }

    #[test]
    fn duplicate_assignments_are_rejected_before_partitioning() {
        let (_, source) = source_snapshot();
        let north = partition("north");
        assert!(matches!(
            EconomicPartitionPlan::new(
                vec![north.clone()],
                vec![
                    (source.stock[0].key.clone(), north.clone()),
                    (source.stock[0].key.clone(), north.clone()),
                ],
                source
                    .accounts
                    .iter()
                    .map(|record| (record.account_id.clone(), north.clone()))
                    .collect(),
            )
            .unwrap_err(),
            PartitionError::DuplicateStockAssignment { .. }
        ));
    }
}
