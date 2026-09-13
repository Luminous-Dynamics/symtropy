// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! External/public-surface adversarial corpus for ECON-04.

use symtropy_core::prelude::*;

fn actor(value: &str) -> ActorId {
    ActorId::new(value).unwrap()
}

fn cause(value: &str) -> CausalId {
    CausalId::new(value).unwrap()
}

fn spec_id(value: &str) -> CommoditySpecId {
    CommoditySpecId::new(value).unwrap()
}

fn unit_id(value: &str) -> QuantityUnitId {
    QuantityUnitId::new(value).unwrap()
}

fn dimension(value: &str) -> QuantityDimensionId {
    QuantityDimensionId::new(value).unwrap()
}

fn unit(
    id: &str,
    dimension_id: &str,
    numerator: u64,
    denominator: u64,
) -> QuantityUnitDefinition {
    QuantityUnitDefinition {
        unit_id: unit_id(id),
        dimension_id: dimension(dimension_id),
        canonical_numerator: numerator,
        canonical_denominator: denominator,
    }
}

fn spec(
    id: &str,
    unit: &str,
    tracking_mode: CommodityTrackingMode,
) -> CommoditySpecDefinition {
    CommoditySpecDefinition {
        commodity_spec_id: spec_id(id),
        quantity_unit_id: unit_id(unit),
        tracking_mode,
        grade_id: None,
        quality_specification_id: None,
        physical_material: None,
    }
}

fn stock_with_lot(id: &str, spec: &str, quantity: u64) -> StockLedger {
    let mut stock = StockLedger::new();
    stock
        .establish(
            StockLot::new(
                LotId::new(id).unwrap(),
                spec_id(spec),
                quantity,
                actor("owner"),
                actor("custodian"),
                LocationId::new("warehouse").unwrap(),
            )
            .unwrap(),
            StockOrigin::QualifiedInitial {
                evidence_id: cause("initial-stock"),
            },
        )
        .unwrap();
    stock
}

fn metadata(id: &str, spec: &str, tracking_identity: LotTrackingIdentity) -> CommodityLotMetadata {
    CommodityLotMetadata {
        lot_id: LotId::new(id).unwrap(),
        commodity_spec_id: spec_id(spec),
        tracking_identity,
        provenance_evidence_id: cause(&format!("{id}-provenance")),
        quality_evidence_id: None,
    }
}

#[test]
fn public_surface_builds_complete_registry() {
    let stock = stock_with_lot("steel-a", "steel", 100);
    let registry = CommodityIdentityRegistry::new(
        &stock,
        vec![unit("kg", "mass", 1, 1)],
        vec![spec("steel", "kg", CommodityTrackingMode::Fungible)],
        vec![metadata("steel-a", "steel", LotTrackingIdentity::Fungible)],
    )
    .unwrap();

    assert_eq!(registry.units().len(), 1);
    assert_eq!(registry.specs().len(), 1);
    assert_eq!(registry.lots().len(), 1);
    registry.validate_against_stock(&stock).unwrap();
}

#[test]
fn stale_metadata_for_non_live_lot_fails_closed() {
    let stock = stock_with_lot("steel-a", "steel", 100);
    let error = CommodityIdentityRegistry::new(
        &stock,
        vec![unit("kg", "mass", 1, 1)],
        vec![spec("steel", "kg", CommodityTrackingMode::Fungible)],
        vec![
            metadata("steel-a", "steel", LotTrackingIdentity::Fungible),
            metadata("steel-depleted", "steel", LotTrackingIdentity::Fungible),
        ],
    )
    .unwrap_err();

    assert_eq!(error, CommodityError::LotMetadataCoverageMismatch);
}

#[test]
fn tracking_mode_mismatch_fails_closed() {
    let stock = stock_with_lot("steel-a", "steel", 100);
    let error = CommodityIdentityRegistry::new(
        &stock,
        vec![unit("kg", "mass", 1, 1)],
        vec![spec("steel", "kg", CommodityTrackingMode::BatchTracked)],
        vec![metadata("steel-a", "steel", LotTrackingIdentity::Fungible)],
    )
    .unwrap_err();

    assert!(matches!(error, CommodityError::TrackingModeMismatch { .. }));
}

#[test]
fn unexpected_quality_evidence_fails_closed() {
    let stock = stock_with_lot("steel-a", "steel", 100);
    let mut lot_metadata = metadata("steel-a", "steel", LotTrackingIdentity::Fungible);
    lot_metadata.quality_evidence_id = Some(cause("unbound-quality-claim"));

    let error = CommodityIdentityRegistry::new(
        &stock,
        vec![unit("kg", "mass", 1, 1)],
        vec![spec("steel", "kg", CommodityTrackingMode::Fungible)],
        vec![lot_metadata],
    )
    .unwrap_err();

    assert!(matches!(error, CommodityError::UnexpectedQualityEvidence { .. }));
}

#[test]
fn zero_unit_scale_fails_closed() {
    let stock = stock_with_lot("steel-a", "steel", 100);
    let error = CommodityIdentityRegistry::new(
        &stock,
        vec![unit("kg", "mass", 0, 1)],
        vec![spec("steel", "kg", CommodityTrackingMode::Fungible)],
        vec![metadata("steel-a", "steel", LotTrackingIdentity::Fungible)],
    )
    .unwrap_err();

    assert!(matches!(error, CommodityError::ZeroUnitScale { .. }));
}

#[test]
fn conversion_intermediate_overflow_fails_closed() {
    let stock = stock_with_lot("steel-a", "steel", 1);
    let registry = CommodityIdentityRegistry::new(
        &stock,
        vec![
            unit("huge-a", "mass", u64::MAX, 1),
            unit("tiny-b", "mass", 1, u64::MAX),
        ],
        vec![spec("steel", "huge-a", CommodityTrackingMode::Fungible)],
        vec![metadata("steel-a", "steel", LotTrackingIdentity::Fungible)],
    )
    .unwrap();

    assert_eq!(
        registry.convert_quantity_exact(u64::MAX, &unit_id("huge-a"), &unit_id("tiny-b")),
        Err(CommodityError::ArithmeticOverflow)
    );
}

#[test]
fn physical_material_reference_is_opaque_identity_binding_only() {
    let stock = stock_with_lot("alloy-a", "alloy", 10);
    let mut alloy = spec("alloy", "kg", CommodityTrackingMode::BatchTracked);
    alloy.physical_material = Some(PhysicalMaterialReference {
        authority_id: MaterialAuthorityId::new("future-material-authority").unwrap(),
        material_id: PhysicalMaterialId::new("alloy-physical-identity").unwrap(),
        evidence_id: cause("material-binding-evidence"),
    });

    CommodityIdentityRegistry::new(
        &stock,
        vec![unit("kg", "mass", 1, 1)],
        vec![alloy],
        vec![metadata(
            "alloy-a",
            "alloy",
            LotTrackingIdentity::Batch {
                batch_id: BatchId::new("heat-42").unwrap(),
            },
        )],
    )
    .unwrap();
}
