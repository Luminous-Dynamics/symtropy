// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! External/public-surface adversarial corpus for ECON-04C.

use symtropy_core::prelude::*;

fn actor(value: &str) -> ActorId {
    ActorId::new(value).unwrap()
}

fn cause(value: &str) -> CausalId {
    CausalId::new(value).unwrap()
}

fn lot_id(value: &str) -> LotId {
    LotId::new(value).unwrap()
}

fn snapshot_id(value: &str) -> EconomicSnapshotId {
    EconomicSnapshotId::new(value).unwrap()
}

fn stock() -> StockLedger {
    let mut stock = StockLedger::new();
    stock
        .establish(
            StockLot::new(
                lot_id("steel-a"),
                CommoditySpecId::new("steel").unwrap(),
                100,
                actor("owner"),
                actor("warehouse"),
                LocationId::new("yard").unwrap(),
            )
            .unwrap(),
            StockOrigin::QualifiedInitial {
                evidence_id: cause("stock-qualified"),
            },
        )
        .unwrap();
    stock
}

fn reservations(stock: &StockLedger) -> StockReservationLedger {
    let mut reservations = StockReservationLedger::new();
    reservations
        .reserve(
            stock,
            StockReservationId::new("reserve-steel").unwrap(),
            lot_id("steel-a"),
            20,
            actor("owner"),
            actor("buyer"),
            cause("purchase-intent"),
            cause("owner-authorization"),
        )
        .unwrap();
    reservations
}

fn financial_state() -> (FinancialBook, FinancialRegistrySnapshot) {
    let currency = CurrencyDefinition {
        currency_id: CurrencyId::new("CR").unwrap(),
        monetary_authority_id: MonetaryAuthorityId::new("mint").unwrap(),
        monetary_authority_actor_id: actor("treasury"),
        minor_unit_exponent: 2,
    };
    let account = FinancialAccount {
        account_id: FinancialAccountId::new("cash").unwrap(),
        owner_id: actor("owner"),
        currency_id: currency.currency_id.clone(),
        class: FinancialAccountClass::Asset,
    };
    let currencies = vec![currency];
    let accounts = vec![account];
    let book = FinancialBook::new(currencies.clone(), accounts.clone()).unwrap();
    let registry = FinancialRegistrySnapshot::new(currencies, accounts).unwrap();
    (book, registry)
}

fn financial_state_with_extra_account() -> (FinancialBook, FinancialRegistrySnapshot) {
    let currency = CurrencyDefinition {
        currency_id: CurrencyId::new("CR").unwrap(),
        monetary_authority_id: MonetaryAuthorityId::new("mint").unwrap(),
        monetary_authority_actor_id: actor("treasury"),
        minor_unit_exponent: 2,
    };
    let accounts = vec![
        FinancialAccount {
            account_id: FinancialAccountId::new("cash").unwrap(),
            owner_id: actor("owner"),
            currency_id: currency.currency_id.clone(),
            class: FinancialAccountClass::Asset,
        },
        FinancialAccount {
            account_id: FinancialAccountId::new("escrow-shadow").unwrap(),
            owner_id: actor("owner"),
            currency_id: currency.currency_id.clone(),
            class: FinancialAccountClass::Asset,
        },
    ];
    let currencies = vec![currency];
    let book = FinancialBook::new(currencies.clone(), accounts.clone()).unwrap();
    let registry = FinancialRegistrySnapshot::new(currencies, accounts).unwrap();
    (book, registry)
}

fn exact_resolution_named<'a>(
    id: &str,
    stock: &'a StockLedger,
    book: &'a FinancialBook,
    registry: &'a FinancialRegistrySnapshot,
) -> EconomicResolutionLedger {
    EconomicResolutionLedger::new(
        snapshot_id(id),
        EconomicResolutionTier::ActiveSite,
        cause(&format!("{id}-exact-detail")),
        ExactEconomicStateRef::Financial {
            stock_ledger: stock,
            financial_book: book,
            registry,
        },
    )
    .unwrap()
}

fn exact_resolution<'a>(
    stock: &'a StockLedger,
    book: &'a FinancialBook,
    registry: &'a FinancialRegistrySnapshot,
) -> EconomicResolutionLedger {
    exact_resolution_named("active-0", stock, book, registry)
}

fn retention(source: &str, id: &str) -> DetailRetentionRef {
    DetailRetentionRef {
        retention_id: DetailRetentionId::new(id).unwrap(),
        source_snapshot_id: snapshot_id(source),
        evidence_id: cause(&format!("{id}-evidence")),
    }
}

#[test]
fn active_reservation_requires_retained_detail_after_demotion() {
    let stock = stock();
    let reservations = reservations(&stock);
    let (book, registry) = financial_state();
    let mut resolution = exact_resolution(&stock, &book, &registry);

    let exact = ReservationResolutionBinding::bind(
        resolution.current_snapshot(),
        &stock,
        &reservations,
    )
    .unwrap();
    assert_eq!(exact.tier(), EconomicResolutionTier::ActiveSite);
    assert!(exact.retention().is_none());

    resolution
        .demote_exact(
            snapshot_id("planet-0"),
            EconomicResolutionTier::PlanetaryAggregate,
            None,
            cause("discard-detail"),
        )
        .unwrap();

    assert_eq!(
        ReservationResolutionBinding::bind(
            resolution.current_snapshot(),
            &stock,
            &reservations,
        ),
        Err(ReservationReconciliationError::ReservedLotDetailUnavailable)
    );
}

#[test]
fn no_active_reservation_allows_existing_econ03_detail_discard_policy() {
    let stock = stock();
    let reservations = StockReservationLedger::new();
    let (book, registry) = financial_state();
    let mut resolution = exact_resolution(&stock, &book, &registry);
    resolution
        .demote_exact(
            snapshot_id("planet-empty-claims"),
            EconomicResolutionTier::PlanetaryAggregate,
            None,
            cause("discard-unneeded-detail"),
        )
        .unwrap();

    let binding = ReservationResolutionBinding::bind(
        resolution.current_snapshot(),
        &stock,
        &reservations,
    )
    .unwrap();
    assert!(binding.manifest().active().is_empty());
    assert!(binding.retention().is_none());
}

#[test]
fn retained_detail_preserves_active_reservation_and_economic_binding() {
    let stock = stock();
    let reservations = reservations(&stock);
    let (book, registry) = financial_state();
    let mut resolution = exact_resolution(&stock, &book, &registry);
    let exact = ReservationResolutionBinding::bind(
        resolution.current_snapshot(),
        &stock,
        &reservations,
    )
    .unwrap();
    let retained = retention("active-0", "retain-active-0");

    resolution
        .demote_exact(
            snapshot_id("distant-0"),
            EconomicResolutionTier::DistantRegion,
            Some(retained.clone()),
            cause("demote-with-retention"),
        )
        .unwrap();
    let coarse = exact
        .rebind_unchanged(resolution.current_snapshot(), &stock, &reservations)
        .unwrap();

    assert_eq!(coarse.retention(), Some(&retained));
    assert_eq!(coarse.manifest(), exact.manifest());
    assert_eq!(coarse.economic_manifest(), exact.economic_manifest());
    assert_eq!(coarse.generation(), exact.generation() + 1);
}

#[test]
fn bind_rejects_stock_state_from_a_different_economic_instant() {
    let stock = stock();
    let reservations = reservations(&stock);
    let (book, registry) = financial_state();
    let resolution = exact_resolution(&stock, &book, &registry);

    let mut drifted_stock = stock.clone();
    drifted_stock
        .deplete(
            lot_id("steel-a"),
            1,
            StockDepletionCause::Loss {
                incident_id: cause("post-snapshot-loss"),
            },
        )
        .unwrap();

    assert_eq!(
        ReservationResolutionBinding::bind(
            resolution.current_snapshot(),
            &drifted_stock,
            &reservations,
        ),
        Err(ReservationReconciliationError::StockManifestMismatch)
    );
}

#[test]
fn pure_resolution_rebind_rejects_same_snapshot_replay() {
    let stock = stock();
    let reservations = reservations(&stock);
    let (book, registry) = financial_state();
    let resolution = exact_resolution(&stock, &book, &registry);
    let binding = ReservationResolutionBinding::bind(
        resolution.current_snapshot(),
        &stock,
        &reservations,
    )
    .unwrap();

    assert_eq!(
        binding.rebind_unchanged(resolution.current_snapshot(), &stock, &reservations),
        Err(ReservationReconciliationError::ResolutionSnapshotIdentityReused)
    );
}

#[test]
fn pure_resolution_rebind_rejects_non_forward_generation() {
    let stock = stock();
    let reservations = reservations(&stock);
    let (book, registry) = financial_state();
    let source = exact_resolution_named("source-0", &stock, &book, &registry);
    let parallel = exact_resolution_named("parallel-0", &stock, &book, &registry);
    let binding = ReservationResolutionBinding::bind(
        source.current_snapshot(),
        &stock,
        &reservations,
    )
    .unwrap();

    assert_eq!(
        binding.rebind_unchanged(parallel.current_snapshot(), &stock, &reservations),
        Err(ReservationReconciliationError::ResolutionGenerationNotForward {
            from: 0,
            to: 0,
        })
    );
}

#[test]
fn pure_resolution_rebind_rejects_reservation_mutation() {
    let stock = stock();
    let mut reservations = reservations(&stock);
    let (book, registry) = financial_state();
    let mut resolution = exact_resolution(&stock, &book, &registry);
    let binding = ReservationResolutionBinding::bind(
        resolution.current_snapshot(),
        &stock,
        &reservations,
    )
    .unwrap();
    resolution
        .demote_exact(
            snapshot_id("distant-reservation-drift"),
            EconomicResolutionTier::DistantRegion,
            Some(retention("active-0", "retain-reservation-drift")),
            cause("demote-before-reservation-drift"),
        )
        .unwrap();

    reservations
        .reserve(
            &stock,
            StockReservationId::new("second-claim").unwrap(),
            lot_id("steel-a"),
            10,
            actor("owner"),
            actor("buyer-2"),
            cause("second-purpose"),
            cause("second-authorization"),
        )
        .unwrap();

    assert_eq!(
        binding.rebind_unchanged(resolution.current_snapshot(), &stock, &reservations),
        Err(ReservationReconciliationError::ReservationChangedDuringResolution)
    );
}

#[test]
fn pure_resolution_rebind_rejects_economic_manifest_mutation_with_unchanged_reservations() {
    let stock = stock();
    let reservations = reservations(&stock);
    let (book, registry) = financial_state();
    let source_resolution = exact_resolution(&stock, &book, &registry);
    let binding = ReservationResolutionBinding::bind(
        source_resolution.current_snapshot(),
        &stock,
        &reservations,
    )
    .unwrap();

    let (drifted_book, drifted_registry) = financial_state_with_extra_account();
    let mut drifted_resolution = exact_resolution(&stock, &drifted_book, &drifted_registry);
    drifted_resolution
        .demote_exact(
            snapshot_id("distant-drifted"),
            EconomicResolutionTier::DistantRegion,
            Some(retention("active-0", "retain-drifted")),
            cause("hide-financial-registry-change"),
        )
        .unwrap();

    assert_eq!(
        binding.rebind_unchanged(
            drifted_resolution.current_snapshot(),
            &stock,
            &reservations,
        ),
        Err(ReservationReconciliationError::EconomicManifestChangedDuringResolution)
    );
}

#[test]
fn reservation_follows_stock_partition_and_repartition_changes_placement_only() {
    let stock = stock();
    let reservations = reservations(&stock);
    let (book, registry) = financial_state();
    let resolution = exact_resolution(&stock, &book, &registry);
    let manifest = resolution.current_snapshot().manifest();
    let region_a = EconomicPartitionId::new("region-a").unwrap();
    let region_b = EconomicPartitionId::new("region-b").unwrap();

    let plan_a = EconomicPartitionPlan::new(
        vec![region_a.clone(), region_b.clone()],
        manifest
            .stock
            .iter()
            .map(|record| (record.key.clone(), region_a.clone()))
            .collect(),
        manifest
            .accounts
            .iter()
            .map(|record| (record.account_id.clone(), region_a.clone()))
            .collect(),
    )
    .unwrap();
    let economic_a = EconomicPartitionSet::partition(
        resolution.current_snapshot().snapshot_id().clone(),
        manifest,
        &plan_a,
    )
    .unwrap();
    let reservation_a =
        ReservationPartitionSet::partition(&economic_a, &stock, &reservations).unwrap();
    assert_eq!(
        reservation_a.partition_by_id(&region_a).unwrap().active().len(),
        1
    );
    assert_eq!(
        reservation_a.partition_by_id(&region_b).unwrap().active().len(),
        0
    );

    let plan_b = EconomicPartitionPlan::new(
        vec![region_a.clone(), region_b.clone()],
        manifest
            .stock
            .iter()
            .map(|record| (record.key.clone(), region_b.clone()))
            .collect(),
        manifest
            .accounts
            .iter()
            .map(|record| (record.account_id.clone(), region_a.clone()))
            .collect(),
    )
    .unwrap();
    let economic_b = EconomicPartitionSet::partition(
        resolution.current_snapshot().snapshot_id().clone(),
        manifest,
        &plan_b,
    )
    .unwrap();
    let reservation_b =
        ReservationPartitionSet::partition(&economic_b, &stock, &reservations).unwrap();

    reconcile_reservation_repartition(&reservation_a, &reservation_b).unwrap();
    assert_eq!(
        reservation_b.partition_by_id(&region_a).unwrap().active().len(),
        0
    );
    assert_eq!(
        reservation_b.partition_by_id(&region_b).unwrap().active().len(),
        1
    );
    assert_eq!(
        reservation_a.reconstruct_manifest().unwrap(),
        reservation_b.reconstruct_manifest().unwrap()
    );

    let economic_other_instant = EconomicPartitionSet::partition(
        snapshot_id("other-economic-instant"),
        manifest,
        &plan_b,
    )
    .unwrap();
    let reservation_other_instant =
        ReservationPartitionSet::partition(&economic_other_instant, &stock, &reservations).unwrap();
    assert_eq!(
        reconcile_reservation_repartition(&reservation_a, &reservation_other_instant),
        Err(ReservationReconciliationError::RepartitionSourceSnapshotMismatch {
            before: resolution.current_snapshot().snapshot_id().clone(),
            after: snapshot_id("other-economic-instant"),
        })
    );
}
