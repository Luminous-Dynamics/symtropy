// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Conservation of stock reservations across ECON-03 resolution and partitions.
//!
//! Active reservations retain exact `LotId` identity even when ordinary stock is
//! represented by a coarser `StockConservationKey`. An active claim therefore pins
//! the retained fine detail needed to interpret that lot identity.

use std::collections::BTreeMap;

use crate::economic::StockLedger;
use crate::economic_partition::{EconomicPartitionId, EconomicPartitionSet};
use crate::economic_resolution::{
    DetailRetentionRef, EconomicConservationManifest, EconomicDetailState,
    EconomicResolutionSnapshot, EconomicResolutionTier, EconomicSnapshotId, StockConservationKey,
    StockConservationRecord,
};
use crate::stock_reservation::{
    StockReservation, StockReservationId, StockReservationLedger, StockReservationLedgerEntry,
};

/// Exact reservation state that must not change during a pure fidelity or
/// partition-placement transition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReservationConservationManifest {
    active: Vec<StockReservation>,
    history: Vec<StockReservationLedgerEntry>,
}

impl ReservationConservationManifest {
    pub fn capture(
        stock: &StockLedger,
        reservations: &StockReservationLedger,
    ) -> Result<Self, ReservationReconciliationError> {
        reservations
            .validate_against_stock(stock)
            .map_err(|_| ReservationReconciliationError::InvalidReservationState)?;
        Ok(Self {
            active: reservations.active_reservations().cloned().collect(),
            history: reservations.entries().to_vec(),
        })
    }

    pub fn active(&self) -> &[StockReservation] {
        &self.active
    }

    pub fn history(&self) -> &[StockReservationLedgerEntry] {
        &self.history
    }

    pub fn has_active_claims(&self) -> bool {
        !self.active.is_empty()
    }
}

/// Reservation-side binding to one exact ECON-03 conservation snapshot.
///
/// The binding carries the complete ECON-03 conservation manifest it was attached
/// to. A pure fidelity rebind must preserve both that economic manifest and the
/// reservation manifest. This prevents callers from attaching unchanged
/// reservations to an unrelated economic snapshot and calling it a resolution
/// transition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReservationResolutionBinding {
    economic_snapshot_id: EconomicSnapshotId,
    tier: EconomicResolutionTier,
    generation: u64,
    retention: Option<DetailRetentionRef>,
    economic_manifest: EconomicConservationManifest,
    manifest: ReservationConservationManifest,
}

impl ReservationResolutionBinding {
    pub fn bind(
        snapshot: &EconomicResolutionSnapshot,
        stock: &StockLedger,
        reservations: &StockReservationLedger,
    ) -> Result<Self, ReservationReconciliationError> {
        let manifest = ReservationConservationManifest::capture(stock, reservations)?;
        validate_stock_matches_snapshot(snapshot.manifest(), stock)?;

        let retention = match snapshot.detail() {
            EconomicDetailState::Exact { .. } => None,
            EconomicDetailState::Aggregated { retention } => {
                if manifest.has_active_claims() && retention.is_none() {
                    return Err(
                        ReservationReconciliationError::ReservedLotDetailUnavailable,
                    );
                }
                retention.clone()
            }
        };
        Ok(Self {
            economic_snapshot_id: snapshot.snapshot_id().clone(),
            tier: snapshot.tier(),
            generation: snapshot.generation(),
            retention,
            economic_manifest: snapshot.manifest().clone(),
            manifest,
        })
    }

    /// Bind a later ECON-03 representation of the same economic instant.
    /// Reservation creation/release/resize and changes to the ECON-03 conservation
    /// manifest are economic mutations; neither may be hidden inside fidelity
    /// reconciliation. Rebinding is forward-only and cannot replay the same snapshot
    /// identity or a non-increasing ECON-03 generation.
    pub fn rebind_unchanged(
        &self,
        target: &EconomicResolutionSnapshot,
        stock: &StockLedger,
        reservations: &StockReservationLedger,
    ) -> Result<Self, ReservationReconciliationError> {
        if target.snapshot_id() == &self.economic_snapshot_id {
            return Err(ReservationReconciliationError::ResolutionSnapshotIdentityReused);
        }
        if target.generation() <= self.generation {
            return Err(
                ReservationReconciliationError::ResolutionGenerationNotForward {
                    from: self.generation,
                    to: target.generation(),
                },
            );
        }

        let candidate = Self::bind(target, stock, reservations)?;
        if candidate.economic_manifest != self.economic_manifest {
            return Err(
                ReservationReconciliationError::EconomicManifestChangedDuringResolution,
            );
        }
        if candidate.manifest != self.manifest {
            return Err(
                ReservationReconciliationError::ReservationChangedDuringResolution,
            );
        }
        Ok(candidate)
    }

    pub fn economic_snapshot_id(&self) -> &EconomicSnapshotId {
        &self.economic_snapshot_id
    }

    pub const fn tier(&self) -> EconomicResolutionTier {
        self.tier
    }

    pub const fn generation(&self) -> u64 {
        self.generation
    }

    pub fn retention(&self) -> Option<&DetailRetentionRef> {
        self.retention.as_ref()
    }

    pub fn economic_manifest(&self) -> &EconomicConservationManifest {
        &self.economic_manifest
    }

    pub fn manifest(&self) -> &ReservationConservationManifest {
        &self.manifest
    }
}

/// Reconstruct the stock portion of ECON-03 directly from the exact StockLedger
/// and require byte-for-byte semantic equality with the snapshot stock manifest.
/// This prevents an exact reservation overlay from being bound to a snapshot whose
/// coarse stock state came from a different economic instant.
fn validate_stock_matches_snapshot(
    economic_manifest: &EconomicConservationManifest,
    stock: &StockLedger,
) -> Result<(), ReservationReconciliationError> {
    stock
        .validate()
        .map_err(|_| ReservationReconciliationError::InvalidReservationState)?;
    let mut grouped: BTreeMap<StockConservationKey, u128> = BTreeMap::new();
    for lot in stock.lots() {
        let key = StockConservationKey {
            commodity_spec_id: lot.commodity_spec_id().clone(),
            owner_id: lot.owner_id().clone(),
            custodian_id: lot.custodian_id().clone(),
            location_id: lot.location_id().clone(),
        };
        let total = grouped.entry(key).or_default();
        *total = total
            .checked_add(u128::from(lot.quantity()))
            .ok_or(ReservationReconciliationError::ArithmeticOverflow)?;
    }
    let exact: Vec<_> = grouped
        .into_iter()
        .map(|(key, quantity)| StockConservationRecord { key, quantity })
        .collect();
    if exact != economic_manifest.stock {
        return Err(ReservationReconciliationError::StockManifestMismatch);
    }
    Ok(())
}

/// Reservation authority placed alongside one ECON-03B economic partition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReservationPartitionManifest {
    partition_id: EconomicPartitionId,
    active: Vec<StockReservation>,
}

impl ReservationPartitionManifest {
    pub fn partition_id(&self) -> &EconomicPartitionId {
        &self.partition_id
    }

    pub fn active(&self) -> &[StockReservation] {
        &self.active
    }
}

/// One decomposition of active reservation authority across the same partition
/// topology as ECON-03B. Canonical reservation history remains shared-global once.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReservationPartitionSet {
    source_snapshot_id: EconomicSnapshotId,
    partitions: Vec<ReservationPartitionManifest>,
    shared_history: Vec<StockReservationLedgerEntry>,
}

impl ReservationPartitionSet {
    pub fn partition(
        economic_partitions: &EconomicPartitionSet,
        stock: &StockLedger,
        reservations: &StockReservationLedger,
    ) -> Result<Self, ReservationReconciliationError> {
        let economic_manifest = economic_partitions
            .reconstruct_manifest()
            .map_err(|_| ReservationReconciliationError::InvalidEconomicPartitionSet)?;
        validate_stock_matches_snapshot(&economic_manifest, stock)?;
        let source = ReservationConservationManifest::capture(stock, reservations)?;

        let mut stock_placement: BTreeMap<StockConservationKey, EconomicPartitionId> =
            BTreeMap::new();
        let mut partition_map: BTreeMap<EconomicPartitionId, ReservationPartitionManifest> =
            BTreeMap::new();

        for partition in economic_partitions.partitions() {
            let partition_id = partition.partition_id().clone();
            if partition_map
                .insert(
                    partition_id.clone(),
                    ReservationPartitionManifest {
                        partition_id: partition_id.clone(),
                        active: Vec::new(),
                    },
                )
                .is_some()
            {
                return Err(ReservationReconciliationError::DuplicatePartitionIdentity);
            }
            for record in partition.stock() {
                if stock_placement
                    .insert(record.key.clone(), partition_id.clone())
                    .is_some()
                {
                    return Err(
                        ReservationReconciliationError::DuplicateStockPartitionAuthority,
                    );
                }
            }
        }

        for reservation in &source.active {
            let lot = stock
                .lot(reservation.lot_id())
                .ok_or(ReservationReconciliationError::InvalidReservationState)?;
            let key = StockConservationKey {
                commodity_spec_id: lot.commodity_spec_id().clone(),
                owner_id: lot.owner_id().clone(),
                custodian_id: lot.custodian_id().clone(),
                location_id: lot.location_id().clone(),
            };
            let partition_id = stock_placement
                .get(&key)
                .ok_or(ReservationReconciliationError::MissingStockPartitionAuthority)?;
            partition_map
                .get_mut(partition_id)
                .ok_or(ReservationReconciliationError::DuplicatePartitionIdentity)?
                .active
                .push(reservation.clone());
        }

        for partition in partition_map.values_mut() {
            partition
                .active
                .sort_by(|left, right| left.reservation_id().cmp(right.reservation_id()));
        }

        let set = Self {
            source_snapshot_id: economic_partitions.source_snapshot_id().clone(),
            partitions: partition_map.into_values().collect(),
            shared_history: source.history.clone(),
        };
        set.validate_against(&source)?;
        Ok(set)
    }

    pub fn source_snapshot_id(&self) -> &EconomicSnapshotId {
        &self.source_snapshot_id
    }

    pub fn partitions(&self) -> &[ReservationPartitionManifest] {
        &self.partitions
    }

    pub fn shared_history(&self) -> &[StockReservationLedgerEntry] {
        &self.shared_history
    }

    pub fn partition_by_id(
        &self,
        partition_id: &EconomicPartitionId,
    ) -> Option<&ReservationPartitionManifest> {
        self.partitions
            .binary_search_by(|partition| partition.partition_id.cmp(partition_id))
            .ok()
            .map(|index| &self.partitions[index])
    }

    pub fn reconstruct_manifest(
        &self,
    ) -> Result<ReservationConservationManifest, ReservationReconciliationError> {
        for pair in self.partitions.windows(2) {
            if pair[0].partition_id >= pair[1].partition_id {
                return Err(ReservationReconciliationError::NonCanonicalPartitions);
            }
        }

        let mut active = BTreeMap::<StockReservationId, StockReservation>::new();
        for partition in &self.partitions {
            for pair in partition.active.windows(2) {
                if pair[0].reservation_id() >= pair[1].reservation_id() {
                    return Err(
                        ReservationReconciliationError::NonCanonicalReservations,
                    );
                }
            }
            for reservation in &partition.active {
                if active
                    .insert(reservation.reservation_id().clone(), reservation.clone())
                    .is_some()
                {
                    return Err(
                        ReservationReconciliationError::DuplicateReservationAuthority,
                    );
                }
            }
        }

        Ok(ReservationConservationManifest {
            active: active.into_values().collect(),
            history: self.shared_history.clone(),
        })
    }

    pub fn validate_against(
        &self,
        source: &ReservationConservationManifest,
    ) -> Result<(), ReservationReconciliationError> {
        if self.reconstruct_manifest()? != *source {
            return Err(ReservationReconciliationError::ReconciliationMismatch);
        }
        Ok(())
    }
}

/// Prove a repartition changed placement only, not economic-instant identity or
/// reservation semantics/history.
pub fn reconcile_reservation_repartition(
    before: &ReservationPartitionSet,
    after: &ReservationPartitionSet,
) -> Result<(), ReservationReconciliationError> {
    if before.source_snapshot_id != after.source_snapshot_id {
        return Err(ReservationReconciliationError::RepartitionSourceSnapshotMismatch {
            before: before.source_snapshot_id.clone(),
            after: after.source_snapshot_id.clone(),
        });
    }
    if before.reconstruct_manifest()? != after.reconstruct_manifest()? {
        return Err(ReservationReconciliationError::ReconciliationMismatch);
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReservationReconciliationError {
    InvalidReservationState,
    InvalidEconomicPartitionSet,
    ArithmeticOverflow,
    StockManifestMismatch,
    ReservedLotDetailUnavailable,
    ResolutionSnapshotIdentityReused,
    ResolutionGenerationNotForward { from: u64, to: u64 },
    EconomicManifestChangedDuringResolution,
    ReservationChangedDuringResolution,
    DuplicatePartitionIdentity,
    DuplicateStockPartitionAuthority,
    MissingStockPartitionAuthority,
    NonCanonicalPartitions,
    NonCanonicalReservations,
    DuplicateReservationAuthority,
    RepartitionSourceSnapshotMismatch {
        before: EconomicSnapshotId,
        after: EconomicSnapshotId,
    },
    ReconciliationMismatch,
}
