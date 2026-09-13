// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Canonical economic commodity identity, units, grades, and lot tracking.
//!
//! ECON-04 gives `CommoditySpecId` explicit economic semantics without becoming a
//! materials-science database or manufacturing authority. Physical-material and
//! quality references are opaque evidence bindings. Stock transformations still
//! require explicit stock depletion/establishment under the appropriate upstream
//! authority; this module contains no generic reclassification or relabel operation.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::economic::{AssetId, CausalId, CommoditySpecId, LotId, StockLedger};

const MAX_ID_LEN: usize = 256;
const MAX_UNITS: usize = 4096;
const MAX_SPECS: usize = 65_536;
const MAX_LOT_METADATA: usize = 65_536;

macro_rules! id_type {
    ($name:ident, $kind:literal) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, CommodityError> {
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

id_type!(QuantityUnitId, "quantity-unit");
id_type!(QuantityDimensionId, "quantity-dimension");
id_type!(CommodityGradeId, "commodity-grade");
id_type!(QualitySpecificationId, "quality-specification");
id_type!(BatchId, "batch");
id_type!(MaterialAuthorityId, "material-authority");
id_type!(PhysicalMaterialId, "physical-material");

/// Exact rational scale to one unnamed canonical unit for a dimension.
///
/// `1 unit == canonical_numerator / canonical_denominator canonical units`.
/// The pair must be positive and reduced. ECON-04 never uses floating-point unit
/// conversion internally.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuantityUnitDefinition {
    pub unit_id: QuantityUnitId,
    pub dimension_id: QuantityDimensionId,
    pub canonical_numerator: u64,
    pub canonical_denominator: u64,
}

impl QuantityUnitDefinition {
    fn validate(&self) -> Result<(), CommodityError> {
        if self.canonical_numerator == 0 || self.canonical_denominator == 0 {
            return Err(CommodityError::ZeroUnitScale {
                unit_id: self.unit_id.clone(),
            });
        }
        if gcd(self.canonical_numerator, self.canonical_denominator) != 1 {
            return Err(CommodityError::NonReducedUnitScale {
                unit_id: self.unit_id.clone(),
            });
        }
        Ok(())
    }
}

/// How stock under one commodity specification must be identity-tracked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommodityTrackingMode {
    /// Individual lot identity is operational only; material is otherwise fungible
    /// under this exact commodity specification.
    Fungible,
    /// Every live lot must bind a persistent batch identity. Several split lots may
    /// legitimately retain the same batch ID.
    BatchTracked,
    /// Every live lot represents exactly one persistent serialized asset.
    Serialized,
}

/// Opaque bridge to an external physical/material ontology.
///
/// ECON-04 validates only identity syntax and evidence presence. It does not assert
/// density, composition, strength, chemistry, or any other physical property.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhysicalMaterialReference {
    pub authority_id: MaterialAuthorityId,
    pub material_id: PhysicalMaterialId,
    pub evidence_id: CausalId,
}

/// Canonical economic meaning of one `CommoditySpecId`.
///
/// Grade and quality IDs are identities, not numerical claims. A grade change is a
/// change of commodity specification and therefore cannot be performed by this
/// registry as an in-place lot relabel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommoditySpecDefinition {
    pub commodity_spec_id: CommoditySpecId,
    pub quantity_unit_id: QuantityUnitId,
    pub tracking_mode: CommodityTrackingMode,
    pub grade_id: Option<CommodityGradeId>,
    pub quality_specification_id: Option<QualitySpecificationId>,
    pub physical_material: Option<PhysicalMaterialReference>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LotTrackingIdentity {
    Fungible,
    Batch { batch_id: BatchId },
    Serialized { asset_id: AssetId },
}

/// Complete ECON-04 metadata for one currently live ECON-00/01 lot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommodityLotMetadata {
    pub lot_id: LotId,
    pub commodity_spec_id: CommoditySpecId,
    pub tracking_identity: LotTrackingIdentity,
    /// Evidence that this lot is entitled to the declared commodity specification.
    pub provenance_evidence_id: CausalId,
    /// Evidence of conformance to `quality_specification_id`, when one exists.
    pub quality_evidence_id: Option<CausalId>,
}

/// Immutable semantic registry for one live stock state.
///
/// V0.1 deliberately has no public mutator. A changed specification catalog or stock
/// state is validated into a new registry snapshot instead of editing semantic
/// identity in place.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommodityIdentityRegistry {
    units: BTreeMap<QuantityUnitId, QuantityUnitDefinition>,
    specs: BTreeMap<CommoditySpecId, CommoditySpecDefinition>,
    lots: BTreeMap<LotId, CommodityLotMetadata>,
}

impl CommodityIdentityRegistry {
    pub fn new(
        stock: &StockLedger,
        units: impl IntoIterator<Item = QuantityUnitDefinition>,
        specs: impl IntoIterator<Item = CommoditySpecDefinition>,
        lots: impl IntoIterator<Item = CommodityLotMetadata>,
    ) -> Result<Self, CommodityError> {
        stock.validate().map_err(CommodityError::Stock)?;

        let mut unit_map = BTreeMap::new();
        for unit in units {
            unit.validate()?;
            let unit_id = unit.unit_id.clone();
            if unit_map.insert(unit_id.clone(), unit).is_some() {
                return Err(CommodityError::DuplicateUnit { unit_id });
            }
        }
        if unit_map.is_empty() {
            return Err(CommodityError::NoUnits);
        }
        if unit_map.len() > MAX_UNITS {
            return Err(CommodityError::ModelTooLarge);
        }

        let mut spec_map = BTreeMap::new();
        for spec in specs {
            if !unit_map.contains_key(&spec.quantity_unit_id) {
                return Err(CommodityError::UnknownUnit {
                    commodity_spec_id: spec.commodity_spec_id.clone(),
                    unit_id: spec.quantity_unit_id.clone(),
                });
            }
            let spec_id = spec.commodity_spec_id.clone();
            if spec_map.insert(spec_id.clone(), spec).is_some() {
                return Err(CommodityError::DuplicateCommoditySpec {
                    commodity_spec_id: spec_id,
                });
            }
        }
        if spec_map.is_empty() {
            return Err(CommodityError::NoCommoditySpecs);
        }
        if spec_map.len() > MAX_SPECS {
            return Err(CommodityError::ModelTooLarge);
        }

        let mut lot_map = BTreeMap::new();
        for metadata in lots {
            let lot_id = metadata.lot_id.clone();
            if lot_map.insert(lot_id.clone(), metadata).is_some() {
                return Err(CommodityError::DuplicateLotMetadata { lot_id });
            }
        }
        if lot_map.len() > MAX_LOT_METADATA {
            return Err(CommodityError::ModelTooLarge);
        }

        let registry = Self {
            units: unit_map,
            specs: spec_map,
            lots: lot_map,
        };
        registry.validate_against_stock(stock)?;
        Ok(registry)
    }

    pub fn unit(&self, unit_id: &QuantityUnitId) -> Option<&QuantityUnitDefinition> {
        self.units.get(unit_id)
    }

    pub fn spec(&self, spec_id: &CommoditySpecId) -> Option<&CommoditySpecDefinition> {
        self.specs.get(spec_id)
    }

    pub fn lot_metadata(&self, lot_id: &LotId) -> Option<&CommodityLotMetadata> {
        self.lots.get(lot_id)
    }

    pub fn units(&self) -> impl ExactSizeIterator<Item = &QuantityUnitDefinition> {
        self.units.values()
    }

    pub fn specs(&self) -> impl ExactSizeIterator<Item = &CommoditySpecDefinition> {
        self.specs.values()
    }

    pub fn lots(&self) -> impl ExactSizeIterator<Item = &CommodityLotMetadata> {
        self.lots.values()
    }

    pub fn validate_against_stock(&self, stock: &StockLedger) -> Result<(), CommodityError> {
        stock.validate().map_err(CommodityError::Stock)?;
        if self.units.is_empty() {
            return Err(CommodityError::NoUnits);
        }
        if self.specs.is_empty() {
            return Err(CommodityError::NoCommoditySpecs);
        }
        if self.units.len() > MAX_UNITS
            || self.specs.len() > MAX_SPECS
            || self.lots.len() > MAX_LOT_METADATA
        {
            return Err(CommodityError::ModelTooLarge);
        }

        for (unit_id, unit) in &self.units {
            unit.validate()?;
            if unit_id != &unit.unit_id {
                return Err(CommodityError::UnitKeyMismatch {
                    key: unit_id.clone(),
                    definition: unit.unit_id.clone(),
                });
            }
        }
        for (spec_id, spec) in &self.specs {
            if spec_id != &spec.commodity_spec_id {
                return Err(CommodityError::SpecKeyMismatch {
                    key: spec_id.clone(),
                    definition: spec.commodity_spec_id.clone(),
                });
            }
            if !self.units.contains_key(&spec.quantity_unit_id) {
                return Err(CommodityError::UnknownUnit {
                    commodity_spec_id: spec.commodity_spec_id.clone(),
                    unit_id: spec.quantity_unit_id.clone(),
                });
            }
        }

        let live_lot_ids: BTreeSet<_> = stock.lots().map(|lot| lot.lot_id().clone()).collect();
        let metadata_lot_ids: BTreeSet<_> = self.lots.keys().cloned().collect();
        if live_lot_ids != metadata_lot_ids {
            return Err(CommodityError::LotMetadataCoverageMismatch);
        }

        let mut serialized_assets = BTreeSet::new();
        for stock_lot in stock.lots() {
            let metadata = self
                .lots
                .get(stock_lot.lot_id())
                .ok_or(CommodityError::LotMetadataCoverageMismatch)?;
            if metadata.lot_id != *stock_lot.lot_id() {
                return Err(CommodityError::LotMetadataKeyMismatch {
                    lot_id: stock_lot.lot_id().clone(),
                });
            }
            if metadata.commodity_spec_id != *stock_lot.commodity_spec_id() {
                return Err(CommodityError::LotCommoditySpecMismatch {
                    lot_id: stock_lot.lot_id().clone(),
                    stock_spec_id: stock_lot.commodity_spec_id().clone(),
                    metadata_spec_id: metadata.commodity_spec_id.clone(),
                });
            }
            let spec = self
                .specs
                .get(stock_lot.commodity_spec_id())
                .ok_or_else(|| CommodityError::UnknownCommoditySpec {
                    lot_id: stock_lot.lot_id().clone(),
                    commodity_spec_id: stock_lot.commodity_spec_id().clone(),
                })?;

            match (&spec.tracking_mode, &metadata.tracking_identity) {
                (CommodityTrackingMode::Fungible, LotTrackingIdentity::Fungible) => {}
                (
                    CommodityTrackingMode::BatchTracked,
                    LotTrackingIdentity::Batch { .. },
                ) => {}
                (
                    CommodityTrackingMode::Serialized,
                    LotTrackingIdentity::Serialized { asset_id },
                ) => {
                    if stock_lot.quantity() != 1 {
                        return Err(CommodityError::SerializedLotQuantityMustBeOne {
                            lot_id: stock_lot.lot_id().clone(),
                            quantity: stock_lot.quantity(),
                        });
                    }
                    if !serialized_assets.insert(asset_id.clone()) {
                        return Err(CommodityError::DuplicateSerializedAsset {
                            asset_id: asset_id.clone(),
                        });
                    }
                }
                _ => {
                    return Err(CommodityError::TrackingModeMismatch {
                        lot_id: stock_lot.lot_id().clone(),
                        expected: spec.tracking_mode,
                    })
                }
            }

            match (
                spec.quality_specification_id.is_some(),
                metadata.quality_evidence_id.is_some(),
            ) {
                (true, false) => {
                    return Err(CommodityError::MissingQualityEvidence {
                        lot_id: stock_lot.lot_id().clone(),
                    })
                }
                (false, true) => {
                    return Err(CommodityError::UnexpectedQualityEvidence {
                        lot_id: stock_lot.lot_id().clone(),
                    })
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// Exact rational quantity conversion between two units of the same dimension.
    ///
    /// This is calculation only. It does not mutate a stock lot or change its
    /// commodity specification. Fractional results fail closed rather than round.
    pub fn convert_quantity_exact(
        &self,
        amount: u64,
        from_unit_id: &QuantityUnitId,
        to_unit_id: &QuantityUnitId,
    ) -> Result<u64, CommodityError> {
        let from = self
            .units
            .get(from_unit_id)
            .ok_or_else(|| CommodityError::UnknownUnitId {
                unit_id: from_unit_id.clone(),
            })?;
        let to = self
            .units
            .get(to_unit_id)
            .ok_or_else(|| CommodityError::UnknownUnitId {
                unit_id: to_unit_id.clone(),
            })?;
        if from.dimension_id != to.dimension_id {
            return Err(CommodityError::IncompatibleUnitDimensions {
                from_dimension_id: from.dimension_id.clone(),
                to_dimension_id: to.dimension_id.clone(),
            });
        }

        let numerator = u128::from(amount)
            .checked_mul(u128::from(from.canonical_numerator))
            .and_then(|value| value.checked_mul(u128::from(to.canonical_denominator)))
            .ok_or(CommodityError::ArithmeticOverflow)?;
        let denominator = u128::from(from.canonical_denominator)
            .checked_mul(u128::from(to.canonical_numerator))
            .ok_or(CommodityError::ArithmeticOverflow)?;
        if numerator % denominator != 0 {
            return Err(CommodityError::FractionalQuantityConversion);
        }
        let converted = numerator / denominator;
        u64::try_from(converted).map_err(|_| CommodityError::ArithmeticOverflow)
    }
}

fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let remainder = a % b;
        a = b;
        b = remainder;
    }
    a
}

fn validate_id(kind: &'static str, value: &str) -> Result<(), CommodityError> {
    if value.is_empty() {
        return Err(CommodityError::EmptyId { kind });
    }
    if value.len() > MAX_ID_LEN {
        return Err(CommodityError::IdTooLong {
            kind,
            max_len: MAX_ID_LEN,
        });
    }
    if value.trim() != value {
        return Err(CommodityError::IdHasSurroundingWhitespace { kind });
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommodityError {
    Stock(crate::economic::EconomicError),
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
    NoUnits,
    NoCommoditySpecs,
    ModelTooLarge,
    ZeroUnitScale {
        unit_id: QuantityUnitId,
    },
    NonReducedUnitScale {
        unit_id: QuantityUnitId,
    },
    DuplicateUnit {
        unit_id: QuantityUnitId,
    },
    DuplicateCommoditySpec {
        commodity_spec_id: CommoditySpecId,
    },
    DuplicateLotMetadata {
        lot_id: LotId,
    },
    UnitKeyMismatch {
        key: QuantityUnitId,
        definition: QuantityUnitId,
    },
    SpecKeyMismatch {
        key: CommoditySpecId,
        definition: CommoditySpecId,
    },
    UnknownUnit {
        commodity_spec_id: CommoditySpecId,
        unit_id: QuantityUnitId,
    },
    UnknownUnitId {
        unit_id: QuantityUnitId,
    },
    UnknownCommoditySpec {
        lot_id: LotId,
        commodity_spec_id: CommoditySpecId,
    },
    LotMetadataCoverageMismatch,
    LotMetadataKeyMismatch {
        lot_id: LotId,
    },
    LotCommoditySpecMismatch {
        lot_id: LotId,
        stock_spec_id: CommoditySpecId,
        metadata_spec_id: CommoditySpecId,
    },
    TrackingModeMismatch {
        lot_id: LotId,
        expected: CommodityTrackingMode,
    },
    SerializedLotQuantityMustBeOne {
        lot_id: LotId,
        quantity: u64,
    },
    DuplicateSerializedAsset {
        asset_id: AssetId,
    },
    MissingQualityEvidence {
        lot_id: LotId,
    },
    UnexpectedQualityEvidence {
        lot_id: LotId,
    },
    IncompatibleUnitDimensions {
        from_dimension_id: QuantityDimensionId,
        to_dimension_id: QuantityDimensionId,
    },
    FractionalQuantityConversion,
    ArithmeticOverflow,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::economic::{ActorId, LocationId, StockLot, StockOrigin};

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
    fn complete_fungible_registry_validates() {
        let stock = stock_with_lot("steel-a", "steel", 100);
        let registry = CommodityIdentityRegistry::new(
            &stock,
            vec![unit("kg", "mass", 1, 1)],
            vec![spec("steel", "kg", CommodityTrackingMode::Fungible)],
            vec![metadata("steel-a", "steel", LotTrackingIdentity::Fungible)],
        )
        .unwrap();
        registry.validate_against_stock(&stock).unwrap();
    }

    #[test]
    fn missing_live_lot_metadata_fails_closed() {
        let stock = stock_with_lot("steel-a", "steel", 100);
        assert_eq!(
            CommodityIdentityRegistry::new(
                &stock,
                vec![unit("kg", "mass", 1, 1)],
                vec![spec("steel", "kg", CommodityTrackingMode::Fungible)],
                Vec::<CommodityLotMetadata>::new(),
            )
            .unwrap_err(),
            CommodityError::LotMetadataCoverageMismatch
        );
    }

    #[test]
    fn unknown_stock_spec_fails_closed() {
        let stock = stock_with_lot("steel-a", "unregistered-steel", 100);
        let error = CommodityIdentityRegistry::new(
            &stock,
            vec![unit("kg", "mass", 1, 1)],
            vec![spec("steel", "kg", CommodityTrackingMode::Fungible)],
            vec![metadata(
                "steel-a",
                "unregistered-steel",
                LotTrackingIdentity::Fungible,
            )],
        )
        .unwrap_err();
        assert!(matches!(error, CommodityError::UnknownCommoditySpec { .. }));
    }

    #[test]
    fn batch_identity_survives_lot_split() {
        let mut stock = stock_with_lot("batch-parent", "alloy", 100);
        stock
            .split(
                LotId::new("batch-parent").unwrap(),
                LotId::new("batch-child").unwrap(),
                40,
                cause("warehouse-split"),
            )
            .unwrap();
        let batch_id = BatchId::new("heat-42").unwrap();
        let registry = CommodityIdentityRegistry::new(
            &stock,
            vec![unit("kg", "mass", 1, 1)],
            vec![spec("alloy", "kg", CommodityTrackingMode::BatchTracked)],
            vec![
                metadata(
                    "batch-parent",
                    "alloy",
                    LotTrackingIdentity::Batch {
                        batch_id: batch_id.clone(),
                    },
                ),
                metadata(
                    "batch-child",
                    "alloy",
                    LotTrackingIdentity::Batch { batch_id },
                ),
            ],
        )
        .unwrap();
        registry.validate_against_stock(&stock).unwrap();
    }

    #[test]
    fn serialized_lot_must_have_quantity_one() {
        let stock = stock_with_lot("machine-a", "machine", 2);
        let error = CommodityIdentityRegistry::new(
            &stock,
            vec![unit("item", "count", 1, 1)],
            vec![spec("machine", "item", CommodityTrackingMode::Serialized)],
            vec![metadata(
                "machine-a",
                "machine",
                LotTrackingIdentity::Serialized {
                    asset_id: AssetId::new("asset-machine-a").unwrap(),
                },
            )],
        )
        .unwrap_err();
        assert!(matches!(
            error,
            CommodityError::SerializedLotQuantityMustBeOne { .. }
        ));
    }

    #[test]
    fn serialized_asset_cannot_back_two_live_lots() {
        let mut stock = StockLedger::new();
        for lot in ["machine-a", "machine-b"] {
            stock
                .establish(
                    StockLot::new(
                        LotId::new(lot).unwrap(),
                        spec_id("machine"),
                        1,
                        actor("owner"),
                        actor("custodian"),
                        LocationId::new("warehouse").unwrap(),
                    )
                    .unwrap(),
                    StockOrigin::QualifiedInitial {
                        evidence_id: cause(&format!("{lot}-initial")),
                    },
                )
                .unwrap();
        }
        let asset_id = AssetId::new("one-machine").unwrap();
        let error = CommodityIdentityRegistry::new(
            &stock,
            vec![unit("item", "count", 1, 1)],
            vec![spec("machine", "item", CommodityTrackingMode::Serialized)],
            vec![
                metadata(
                    "machine-a",
                    "machine",
                    LotTrackingIdentity::Serialized {
                        asset_id: asset_id.clone(),
                    },
                ),
                metadata(
                    "machine-b",
                    "machine",
                    LotTrackingIdentity::Serialized { asset_id },
                ),
            ],
        )
        .unwrap_err();
        assert!(matches!(error, CommodityError::DuplicateSerializedAsset { .. }));
    }

    #[test]
    fn quality_spec_requires_per_lot_evidence() {
        let stock = stock_with_lot("steel-a", "grade-a-steel", 100);
        let mut grade_a = spec("grade-a-steel", "kg", CommodityTrackingMode::Fungible);
        grade_a.grade_id = Some(CommodityGradeId::new("grade-a").unwrap());
        grade_a.quality_specification_id =
            Some(QualitySpecificationId::new("astm-like-test-profile").unwrap());
        let error = CommodityIdentityRegistry::new(
            &stock,
            vec![unit("kg", "mass", 1, 1)],
            vec![grade_a],
            vec![metadata(
                "steel-a",
                "grade-a-steel",
                LotTrackingIdentity::Fungible,
            )],
        )
        .unwrap_err();
        assert!(matches!(error, CommodityError::MissingQualityEvidence { .. }));
    }

    #[test]
    fn exact_unit_conversion_never_rounds() {
        let stock = stock_with_lot("steel-a", "steel", 100);
        let registry = CommodityIdentityRegistry::new(
            &stock,
            vec![unit("kg", "mass", 1, 1), unit("g", "mass", 1, 1000)],
            vec![spec("steel", "kg", CommodityTrackingMode::Fungible)],
            vec![metadata("steel-a", "steel", LotTrackingIdentity::Fungible)],
        )
        .unwrap();
        assert_eq!(
            registry
                .convert_quantity_exact(2, &unit_id("kg"), &unit_id("g"))
                .unwrap(),
            2000
        );
        assert_eq!(
            registry.convert_quantity_exact(1, &unit_id("g"), &unit_id("kg")),
            Err(CommodityError::FractionalQuantityConversion)
        );
    }

    #[test]
    fn cross_dimension_conversion_is_rejected() {
        let stock = stock_with_lot("steel-a", "steel", 100);
        let registry = CommodityIdentityRegistry::new(
            &stock,
            vec![unit("kg", "mass", 1, 1), unit("item", "count", 1, 1)],
            vec![spec("steel", "kg", CommodityTrackingMode::Fungible)],
            vec![metadata("steel-a", "steel", LotTrackingIdentity::Fungible)],
        )
        .unwrap();
        assert!(matches!(
            registry
                .convert_quantity_exact(1, &unit_id("kg"), &unit_id("item"))
                .unwrap_err(),
            CommodityError::IncompatibleUnitDimensions { .. }
        ));
    }

    #[test]
    fn unit_scale_must_be_canonical_reduced_fraction() {
        let stock = stock_with_lot("steel-a", "steel", 100);
        assert!(matches!(
            CommodityIdentityRegistry::new(
                &stock,
                vec![unit("bad", "mass", 2, 2)],
                vec![spec("steel", "bad", CommodityTrackingMode::Fungible)],
                vec![metadata("steel-a", "steel", LotTrackingIdentity::Fungible)],
            )
            .unwrap_err(),
            CommodityError::NonReducedUnitScale { .. }
        ));
    }
}
