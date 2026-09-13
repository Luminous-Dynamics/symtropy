// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Controlled authority repartitioning for one frozen ECON-03 economic instant.
//!
//! ECON-03B proves that one partition set reconstructs the exact global economic
//! manifest. ECON-03C proves that changing the partition topology or authoritative
//! placement of stock/accounts does not itself change that manifest.
//!
//! Repartitioning here is not logistics, ownership transfer, account transfer, or
//! simulation-time advancement. Only which partition is authoritative for an
//! unchanged ECON-03 stock record or financial account may change.

use std::collections::BTreeMap;
use std::fmt;

use crate::economic::CausalId;
use crate::economic_partition::{
    EconomicPartitionId, EconomicPartitionPlan, EconomicPartitionSet, PartitionError,
};
use crate::economic_resolution::{EconomicSnapshotId, StockConservationKey};
use crate::financial::FinancialAccountId;

const MAX_ID_LEN: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EconomicRepartitionId(String);

impl EconomicRepartitionId {
    pub fn new(value: impl Into<String>) -> Result<Self, RepartitionError> {
        let value = value.into();
        validate_id("economic-repartition", &value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for EconomicRepartitionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StockAuthorityMove {
    pub key: StockConservationKey,
    pub from_partition_id: EconomicPartitionId,
    pub to_partition_id: EconomicPartitionId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountAuthorityMove {
    pub account_id: FinancialAccountId,
    pub from_partition_id: EconomicPartitionId,
    pub to_partition_id: EconomicPartitionId,
}

/// Complete deterministic receipt for one pure authority-placement change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EconomicRepartitionReceipt {
    repartition_id: EconomicRepartitionId,
    source_snapshot_id: EconomicSnapshotId,
    evidence_id: CausalId,
    source_partition_ids: Vec<EconomicPartitionId>,
    target_partition_ids: Vec<EconomicPartitionId>,
    stock_moves: Vec<StockAuthorityMove>,
    account_moves: Vec<AccountAuthorityMove>,
    unchanged_stock_authorities: usize,
    unchanged_account_authorities: usize,
}

impl EconomicRepartitionReceipt {
    pub fn repartition_id(&self) -> &EconomicRepartitionId {
        &self.repartition_id
    }

    pub fn source_snapshot_id(&self) -> &EconomicSnapshotId {
        &self.source_snapshot_id
    }

    pub fn evidence_id(&self) -> &CausalId {
        &self.evidence_id
    }

    pub fn source_partition_ids(&self) -> &[EconomicPartitionId] {
        &self.source_partition_ids
    }

    pub fn target_partition_ids(&self) -> &[EconomicPartitionId] {
        &self.target_partition_ids
    }

    pub fn stock_moves(&self) -> &[StockAuthorityMove] {
        &self.stock_moves
    }

    pub fn account_moves(&self) -> &[AccountAuthorityMove] {
        &self.account_moves
    }

    pub const fn unchanged_stock_authorities(&self) -> usize {
        self.unchanged_stock_authorities
    }

    pub const fn unchanged_account_authorities(&self) -> usize {
        self.unchanged_account_authorities
    }

    /// Recompute the complete receipt from source and target partition sets.
    pub fn validate(
        &self,
        source: &EconomicPartitionSet,
        target: &EconomicPartitionSet,
    ) -> Result<(), RepartitionError> {
        validate_same_economic_instant(source, target)?;
        let expected = derive_receipt(
            source,
            target,
            self.repartition_id.clone(),
            self.evidence_id.clone(),
        )?;
        if &expected != self {
            return Err(RepartitionError::ReceiptMismatch);
        }
        Ok(())
    }
}

/// Build a new authoritative partition set for the exact same ECON-03 snapshot and
/// emit the complete placement-change receipt.
///
/// The target partition set is constructed from the source set's reconstructed global
/// manifest. Therefore a target plan has no authority to modify any economic record.
pub fn repartition_authority(
    source: &EconomicPartitionSet,
    target_plan: &EconomicPartitionPlan,
    repartition_id: EconomicRepartitionId,
    evidence_id: CausalId,
) -> Result<(EconomicPartitionSet, EconomicRepartitionReceipt), RepartitionError> {
    let source_manifest = source.reconstruct_manifest()?;
    let target = EconomicPartitionSet::partition(
        source.source_snapshot_id().clone(),
        &source_manifest,
        target_plan,
    )?;

    validate_same_economic_instant(source, &target)?;
    let receipt = derive_receipt(source, &target, repartition_id, evidence_id)?;

    if receipt.source_partition_ids == receipt.target_partition_ids
        && receipt.stock_moves.is_empty()
        && receipt.account_moves.is_empty()
    {
        return Err(RepartitionError::NoOpRepartition);
    }

    receipt.validate(source, &target)?;
    Ok((target, receipt))
}

fn validate_same_economic_instant(
    source: &EconomicPartitionSet,
    target: &EconomicPartitionSet,
) -> Result<(), RepartitionError> {
    if source.source_snapshot_id() != target.source_snapshot_id() {
        return Err(RepartitionError::SourceSnapshotMismatch {
            source: source.source_snapshot_id().clone(),
            target: target.source_snapshot_id().clone(),
        });
    }

    let source_manifest = source.reconstruct_manifest()?;
    let target_manifest = target.reconstruct_manifest()?;
    if source_manifest != target_manifest {
        return Err(RepartitionError::EconomicManifestChanged);
    }
    if source.shared_global() != target.shared_global() {
        return Err(RepartitionError::SharedGlobalStateChanged);
    }
    Ok(())
}

fn derive_receipt(
    source: &EconomicPartitionSet,
    target: &EconomicPartitionSet,
    repartition_id: EconomicRepartitionId,
    evidence_id: CausalId,
) -> Result<EconomicRepartitionReceipt, RepartitionError> {
    let source_stock = stock_authority_map(source)?;
    let target_stock = stock_authority_map(target)?;
    if source_stock.keys().ne(target_stock.keys()) {
        return Err(RepartitionError::StockAuthorityKeySetChanged);
    }

    let source_accounts = account_authority_map(source)?;
    let target_accounts = account_authority_map(target)?;
    if source_accounts.keys().ne(target_accounts.keys()) {
        return Err(RepartitionError::AccountAuthorityKeySetChanged);
    }

    let mut stock_moves = Vec::new();
    let mut unchanged_stock_authorities = 0_usize;
    for (key, from_partition_id) in &source_stock {
        let to_partition_id = target_stock
            .get(key)
            .ok_or(RepartitionError::StockAuthorityKeySetChanged)?;
        if from_partition_id == to_partition_id {
            unchanged_stock_authorities = unchanged_stock_authorities
                .checked_add(1)
                .ok_or(RepartitionError::ArithmeticOverflow)?;
        } else {
            stock_moves.push(StockAuthorityMove {
                key: key.clone(),
                from_partition_id: from_partition_id.clone(),
                to_partition_id: to_partition_id.clone(),
            });
        }
    }

    let mut account_moves = Vec::new();
    let mut unchanged_account_authorities = 0_usize;
    for (account_id, from_partition_id) in &source_accounts {
        let to_partition_id = target_accounts
            .get(account_id)
            .ok_or(RepartitionError::AccountAuthorityKeySetChanged)?;
        if from_partition_id == to_partition_id {
            unchanged_account_authorities = unchanged_account_authorities
                .checked_add(1)
                .ok_or(RepartitionError::ArithmeticOverflow)?;
        } else {
            account_moves.push(AccountAuthorityMove {
                account_id: account_id.clone(),
                from_partition_id: from_partition_id.clone(),
                to_partition_id: to_partition_id.clone(),
            });
        }
    }

    Ok(EconomicRepartitionReceipt {
        repartition_id,
        source_snapshot_id: source.source_snapshot_id().clone(),
        evidence_id,
        source_partition_ids: source
            .partitions()
            .iter()
            .map(|partition| partition.partition_id().clone())
            .collect(),
        target_partition_ids: target
            .partitions()
            .iter()
            .map(|partition| partition.partition_id().clone())
            .collect(),
        stock_moves,
        account_moves,
        unchanged_stock_authorities,
        unchanged_account_authorities,
    })
}

fn stock_authority_map(
    set: &EconomicPartitionSet,
) -> Result<BTreeMap<StockConservationKey, EconomicPartitionId>, RepartitionError> {
    let mut map = BTreeMap::new();
    for partition in set.partitions() {
        for record in partition.stock() {
            if map
                .insert(record.key.clone(), partition.partition_id().clone())
                .is_some()
            {
                return Err(RepartitionError::DuplicateStockAuthority {
                    key: record.key.clone(),
                });
            }
        }
    }
    Ok(map)
}

fn account_authority_map(
    set: &EconomicPartitionSet,
) -> Result<BTreeMap<FinancialAccountId, EconomicPartitionId>, RepartitionError> {
    let mut map = BTreeMap::new();
    for partition in set.partitions() {
        for record in partition.accounts() {
            if map
                .insert(record.account_id.clone(), partition.partition_id().clone())
                .is_some()
            {
                return Err(RepartitionError::DuplicateAccountAuthority {
                    account_id: record.account_id.clone(),
                });
            }
        }
    }
    Ok(map)
}

fn validate_id(kind: &'static str, value: &str) -> Result<(), RepartitionError> {
    if value.is_empty() {
        return Err(RepartitionError::EmptyId { kind });
    }
    if value.len() > MAX_ID_LEN {
        return Err(RepartitionError::IdTooLong {
            kind,
            max_len: MAX_ID_LEN,
        });
    }
    if value.trim() != value {
        return Err(RepartitionError::IdHasSurroundingWhitespace { kind });
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepartitionError {
    Partition(PartitionError),
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
    NoOpRepartition,
    SourceSnapshotMismatch {
        source: EconomicSnapshotId,
        target: EconomicSnapshotId,
    },
    EconomicManifestChanged,
    SharedGlobalStateChanged,
    StockAuthorityKeySetChanged,
    AccountAuthorityKeySetChanged,
    DuplicateStockAuthority {
        key: StockConservationKey,
    },
    DuplicateAccountAuthority {
        account_id: FinancialAccountId,
    },
    ReceiptMismatch,
    ArithmeticOverflow,
}

impl From<PartitionError> for RepartitionError {
    fn from(value: PartitionError) -> Self {
        Self::Partition(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::economic::{ActorId, CausalId, CommoditySpecId, LocationId};
    use crate::economic_resolution::{
        AccountConservationRecord, CurrencyConservationRecord, EconomicConservationManifest,
        EconomicHistoryManifest, StockConservationRecord,
    };
    use crate::financial::{
        AccountTotals, CurrencyDefinition, CurrencyId, FinancialAccountClass, MonetaryAuthorityId,
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

    fn stock_key(owner: &str, location: &str) -> StockConservationKey {
        StockConservationKey {
            commodity_spec_id: CommoditySpecId::new("steel").unwrap(),
            owner_id: actor(owner),
            custodian_id: actor(&format!("{owner}-warehouse")),
            location_id: LocationId::new(location).unwrap(),
        }
    }

    fn source_manifest() -> EconomicConservationManifest {
        EconomicConservationManifest {
            stock: vec![
                StockConservationRecord {
                    key: stock_key("north", "north-yard"),
                    quantity: 40,
                },
                StockConservationRecord {
                    key: stock_key("south", "south-yard"),
                    quantity: 60,
                },
            ],
            accounts: vec![
                AccountConservationRecord {
                    account_id: account("north-account"),
                    owner_id: actor("north"),
                    currency_id: currency("CR"),
                    class: FinancialAccountClass::Asset,
                    totals: AccountTotals::default(),
                },
                AccountConservationRecord {
                    account_id: account("south-account"),
                    owner_id: actor("south"),
                    currency_id: currency("CR"),
                    class: FinancialAccountClass::Asset,
                    totals: AccountTotals::default(),
                },
            ],
            currencies: vec![CurrencyConservationRecord {
                definition: CurrencyDefinition {
                    currency_id: currency("CR"),
                    monetary_authority_id: MonetaryAuthorityId::new("mint").unwrap(),
                    monetary_authority_actor_id: actor("treasury"),
                    minor_unit_exponent: 2,
                },
                declared_supply: 100,
            }],
            history: EconomicHistoryManifest {
                stock_events: Vec::new(),
                journal_transactions: Vec::new(),
                monetary_events: Vec::new(),
                settlements: Vec::new(),
            },
        }
    }

    fn source_plan(manifest: &EconomicConservationManifest) -> EconomicPartitionPlan {
        let north = partition("north");
        let south = partition("south");
        EconomicPartitionPlan::new(
            vec![north.clone(), south.clone()],
            vec![
                (manifest.stock[0].key.clone(), north.clone()),
                (manifest.stock[1].key.clone(), south.clone()),
            ],
            vec![
                (manifest.accounts[0].account_id.clone(), north),
                (manifest.accounts[1].account_id.clone(), south),
            ],
        )
        .unwrap()
    }

    fn swapped_plan(manifest: &EconomicConservationManifest) -> EconomicPartitionPlan {
        let north = partition("north");
        let south = partition("south");
        EconomicPartitionPlan::new(
            vec![north.clone(), south.clone()],
            vec![
                (manifest.stock[0].key.clone(), south.clone()),
                (manifest.stock[1].key.clone(), north.clone()),
            ],
            vec![
                (manifest.accounts[0].account_id.clone(), south),
                (manifest.accounts[1].account_id.clone(), north),
            ],
        )
        .unwrap()
    }

    fn source_set() -> (EconomicConservationManifest, EconomicPartitionSet) {
        let manifest = source_manifest();
        let set = EconomicPartitionSet::partition(
            EconomicSnapshotId::new("global-0").unwrap(),
            &manifest,
            &source_plan(&manifest),
        )
        .unwrap();
        (manifest, set)
    }

    #[test]
    fn repartition_changes_authority_placement_only() {
        let (manifest, source) = source_set();
        let (target, receipt) = repartition_authority(
            &source,
            &swapped_plan(&manifest),
            EconomicRepartitionId::new("rebalance-1").unwrap(),
            cause("operator-plan-1"),
        )
        .unwrap();

        assert_eq!(target.reconstruct_manifest().unwrap(), manifest);
        assert_eq!(source.shared_global(), target.shared_global());
        assert_eq!(source.source_snapshot_id(), target.source_snapshot_id());
        assert_eq!(receipt.stock_moves().len(), 2);
        assert_eq!(receipt.account_moves().len(), 2);
        assert_eq!(receipt.unchanged_stock_authorities(), 0);
        assert_eq!(receipt.unchanged_account_authorities(), 0);
        receipt.validate(&source, &target).unwrap();
    }

    #[test]
    fn movement_receipt_is_canonical_by_economic_identity() {
        let (manifest, source) = source_set();
        let (_, receipt) = repartition_authority(
            &source,
            &swapped_plan(&manifest),
            EconomicRepartitionId::new("rebalance-1").unwrap(),
            cause("operator-plan-1"),
        )
        .unwrap();

        assert!(receipt
            .stock_moves()
            .windows(2)
            .all(|pair| pair[0].key < pair[1].key));
        assert!(receipt
            .account_moves()
            .windows(2)
            .all(|pair| pair[0].account_id < pair[1].account_id));
    }

    #[test]
    fn no_op_repartition_is_rejected() {
        let (manifest, source) = source_set();
        assert_eq!(
            repartition_authority(
                &source,
                &source_plan(&manifest),
                EconomicRepartitionId::new("noop").unwrap(),
                cause("noop-plan"),
            )
            .unwrap_err(),
            RepartitionError::NoOpRepartition
        );
    }

    #[test]
    fn topology_change_without_record_movement_is_still_a_real_repartition() {
        let (manifest, source) = source_set();
        let north = partition("north");
        let south = partition("south");
        let observer = partition("observer-empty");
        let plan = EconomicPartitionPlan::new(
            vec![north.clone(), observer.clone(), south.clone()],
            vec![
                (manifest.stock[0].key.clone(), north.clone()),
                (manifest.stock[1].key.clone(), south.clone()),
            ],
            vec![
                (manifest.accounts[0].account_id.clone(), north),
                (manifest.accounts[1].account_id.clone(), south),
            ],
        )
        .unwrap();

        let (target, receipt) = repartition_authority(
            &source,
            &plan,
            EconomicRepartitionId::new("add-empty-partition").unwrap(),
            cause("topology-change"),
        )
        .unwrap();
        assert!(receipt.stock_moves().is_empty());
        assert!(receipt.account_moves().is_empty());
        assert_eq!(receipt.target_partition_ids().len(), 3);
        assert!(target.partition_by_id(&observer).is_some());
        assert_eq!(target.reconstruct_manifest().unwrap(), manifest);
    }

    #[test]
    fn target_plan_cannot_drop_authoritative_state() {
        let (manifest, source) = source_set();
        let north = partition("north");
        let invalid = EconomicPartitionPlan::new(
            vec![north.clone()],
            vec![(manifest.stock[0].key.clone(), north.clone())],
            manifest
                .accounts
                .iter()
                .map(|record| (record.account_id.clone(), north.clone()))
                .collect(),
        )
        .unwrap();

        assert!(matches!(
            repartition_authority(
                &source,
                &invalid,
                EconomicRepartitionId::new("drop-stock").unwrap(),
                cause("bad-plan"),
            )
            .unwrap_err(),
            RepartitionError::Partition(PartitionError::IncompleteStockAssignment)
        ));
    }

    #[test]
    fn repartition_cannot_change_stock_location_or_quantity() {
        let (manifest, source) = source_set();
        let (target, _) = repartition_authority(
            &source,
            &swapped_plan(&manifest),
            EconomicRepartitionId::new("rebalance-1").unwrap(),
            cause("operator-plan-1"),
        )
        .unwrap();
        let reconstructed = target.reconstruct_manifest().unwrap();
        assert_eq!(reconstructed.stock, manifest.stock);
    }

    #[test]
    fn repartition_cannot_duplicate_global_currency_supply() {
        let (manifest, source) = source_set();
        let (target, _) = repartition_authority(
            &source,
            &swapped_plan(&manifest),
            EconomicRepartitionId::new("rebalance-1").unwrap(),
            cause("operator-plan-1"),
        )
        .unwrap();
        assert_eq!(target.shared_global().currencies().len(), 1);
        assert_eq!(target.shared_global().currencies()[0].declared_supply, 100);
        assert_eq!(target.shared_global(), source.shared_global());
    }

    #[test]
    fn tampered_receipt_is_detected() {
        let (manifest, source) = source_set();
        let (target, mut receipt) = repartition_authority(
            &source,
            &swapped_plan(&manifest),
            EconomicRepartitionId::new("rebalance-1").unwrap(),
            cause("operator-plan-1"),
        )
        .unwrap();
        receipt.stock_moves[0].to_partition_id = partition("invented");
        assert_eq!(
            receipt.validate(&source, &target),
            Err(RepartitionError::ReceiptMismatch)
        );
    }
}
