// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Declared use and care profiles for garments that do not fit an everyday-wear model.
//!
//! Occasional use is not itself evidence of low value or a poor outcome. Formal
//! wear, couture, ceremonial garments, stage costumes and collection/display
//! textiles can have different service units, values, handling risks and
//! end-of-life options. This module validates that those distinctions are
//! explicit; it does not assign cultural value or calculate lifecycle impact.

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

/// Garment families with meaningfully different use/care profiles.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum GarmentArchetype {
    Everyday,
    Workwear,
    Occasionwear,
    CoutureOrDecorative,
    CeremonialOrHeritage,
    PerformanceOrCostume,
    ArchiveOrDisplay,
    ModularDecorative,
    DeadstockOrRunwaySample,
}

/// Expected pattern of service; no ordering implies greater or lesser value.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum UsePattern {
    Frequent,
    Seasonal,
    Occasional,
    EventSpecific,
    PerformanceSchedule,
    DisplayRotation,
    ArchivePreservation,
}

/// Basis used to define the functional unit for assessment.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FunctionalUnitBasis {
    WearAndCareCycles,
    DistinctEvents,
    PerformanceRuns,
    ExhibitionDays,
    ConservationYears,
    Custom(String),
}

/// Dimensions of value/function that may be relevant to a declared profile.
///
/// The evaluator must retain these as named dimensions, not convert them into
/// an invented universal cultural/aesthetic scalar.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum ValueDimension {
    ProtectionOrUtility,
    FitAndComfort,
    AestheticExpression,
    CulturalOrCeremonialMeaning,
    HistoricalEvidence,
    PerformanceFunction,
    IdentityOrBelonging,
    Collectability,
    AccessibilityAndEaseOfUse,
}

/// Declared care, handling, and preservation constraints.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum CareConstraint {
    MachineWashable,
    ProfessionalCleaning,
    SpotCleanOnly,
    NoWetCleaning,
    RemoveDetachableEmbellishmentsBeforeCleaning,
    HandSupportDuringHandling,
    PaddedOrFlatStorage,
    LightAndHumidityControlledStorage,
    PestInspection,
    SpecialistConservationReview,
    RepairByQualifiedEmbellishmentSpecialist,
}

/// Decoration/material features that can change care, repair, or recovery needs.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum DecorationRisk {
    Beadwork,
    Sequins,
    DenseEmbroidery,
    MetallicThread,
    FeathersOrNaturalTrim,
    AppliqueOrPatchwork,
    MixedMaterialTrim,
    AdhesiveBondedDecoration,
    ColorSensitiveSurface,
    HistoricOrDegradingMaterial,
    UnknownBlendOrDeadstockComposition,
    DetachableModularTrim,
}

/// Production strategy for a garment profile; not a guarantee of supply capacity.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum ProductionStrategy {
    MadeToOrder,
    PreorderOrDemandSignal,
    SmallBatch,
    ForecastDrivenBatch,
    RentalOrSharedInventory,
    OneOffArtisanal,
    CommissionedOrCeremonial,
    RepairOrAlterationFirst,
    ReuseExistingSample,
}

/// Permitted or intended second-life route; does not imply qualification.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum CirculationRoute {
    OwnerRetained,
    RepairAndReturn,
    AlterationOrRestyling,
    RentalOrCostumeHire,
    LendingOrSharedWardrobe,
    Resale,
    ArchiveOrMuseumCollection,
    ExhibitionOrDisplay,
    TransformIntoAnotherArticle,
    DonationOrCommunityTransfer,
    UnsoldInventoryReallocation,
    QualifiedMaterialRecovery,
}

/// A garment-specific declaration for functional comparison.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GarmentUseProfile {
    pub profile_id: String,
    pub archetype: GarmentArchetype,
    pub use_pattern: UsePattern,
    pub functional_unit: FunctionalUnitBasis,
    pub intended_service: String,
    pub value_dimensions: Vec<ValueDimension>,
    pub decoration_risks: Vec<DecorationRisk>,
    pub specialist_assessment_required: bool,
    pub production_strategies: Vec<ProductionStrategy>,
    pub care_constraints: Vec<CareConstraint>,
    pub circulation_routes: Vec<CirculationRoute>,
}

/// Structural errors in a use-profile declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GarmentProfileError {
    EmptyProfileId,
    EmptyIntendedService,
    EmptyCustomFunctionalUnit,
    MissingValueDimensions,
    DuplicateValueDimension(ValueDimension),
    DuplicateDecorationRisk(DecorationRisk),
    DuplicateProductionStrategy(ProductionStrategy),
    DuplicateCareConstraint(CareConstraint),
    DuplicateCirculationRoute(CirculationRoute),
}

impl fmt::Display for GarmentProfileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyProfileId => write!(f, "profile ID must not be empty"),
            Self::EmptyIntendedService => write!(f, "intended garment service must be declared"),
            Self::EmptyCustomFunctionalUnit => {
                write!(f, "custom functional-unit basis must not be empty")
            }
            Self::MissingValueDimensions => {
                write!(f, "at least one named value/function dimension is required")
            }
            Self::DuplicateValueDimension(value) => {
                write!(f, "duplicate value/function dimension: {value:?}")
            }
            Self::DuplicateDecorationRisk(value) => {
                write!(f, "duplicate decoration/material risk: {value:?}")
            }
            Self::DuplicateProductionStrategy(value) => {
                write!(f, "duplicate production strategy: {value:?}")
            }
            Self::DuplicateCareConstraint(value) => {
                write!(f, "duplicate care constraint: {value:?}")
            }
            Self::DuplicateCirculationRoute(value) => {
                write!(f, "duplicate circulation route: {value:?}")
            }
        }
    }
}

impl Error for GarmentProfileError {}

impl GarmentUseProfile {
    /// Validate declarations without scoring or ranking the type of garment.
    pub fn validate(&self) -> Result<(), GarmentProfileError> {
        if self.profile_id.trim().is_empty() {
            return Err(GarmentProfileError::EmptyProfileId);
        }
        if self.intended_service.trim().is_empty() {
            return Err(GarmentProfileError::EmptyIntendedService);
        }
        if matches!(
            &self.functional_unit,
            FunctionalUnitBasis::Custom(unit) if unit.trim().is_empty()
        ) {
            return Err(GarmentProfileError::EmptyCustomFunctionalUnit);
        }
        if self.value_dimensions.is_empty() {
            return Err(GarmentProfileError::MissingValueDimensions);
        }
        ensure_unique(
            self.value_dimensions.iter().copied(),
            GarmentProfileError::DuplicateValueDimension,
        )?;
        ensure_unique(
            self.decoration_risks.iter().copied(),
            GarmentProfileError::DuplicateDecorationRisk,
        )?;
        ensure_unique(
            self.production_strategies.iter().copied(),
            GarmentProfileError::DuplicateProductionStrategy,
        )?;
        ensure_unique(
            self.care_constraints.iter().copied(),
            GarmentProfileError::DuplicateCareConstraint,
        )?;
        ensure_unique(
            self.circulation_routes.iter().copied(),
            GarmentProfileError::DuplicateCirculationRoute,
        )?;
        Ok(())
    }

    /// Whether this profile declares any non-wear service unit.
    ///
    /// This is a descriptive helper only; callers must not infer low value or
    /// environmental harm from a non-wear service unit.
    pub fn has_non_wear_service_unit(&self) -> bool {
        matches!(
            &self.functional_unit,
            FunctionalUnitBasis::PerformanceRuns
                | FunctionalUnitBasis::ExhibitionDays
                | FunctionalUnitBasis::ConservationYears
                | FunctionalUnitBasis::Custom(_)
        )
    }
}

fn ensure_unique<T, E, I, F>(items: I, make_error: F) -> Result<(), E>
where
    T: Ord + Copy,
    I: IntoIterator<Item = T>,
    F: Fn(T) -> E,
{
    let mut seen = BTreeSet::new();
    for item in items {
        if !seen.insert(item) {
            return Err(make_error(item));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn couture_profile() -> GarmentUseProfile {
        GarmentUseProfile {
            profile_id: "COUTURE-BEADED-EVENING-GOWN-01".into(),
            archetype: GarmentArchetype::CoutureOrDecorative,
            use_pattern: UsePattern::EventSpecific,
            functional_unit: FunctionalUnitBasis::DistinctEvents,
            intended_service: "formal event appearance with beadwork integrity retained".into(),
            value_dimensions: vec![
                ValueDimension::AestheticExpression,
                ValueDimension::Collectability,
                ValueDimension::FitAndComfort,
            ],
            decoration_risks: vec![DecorationRisk::Beadwork, DecorationRisk::MetallicThread],
            specialist_assessment_required: true,
            production_strategies: vec![
                ProductionStrategy::MadeToOrder,
                ProductionStrategy::OneOffArtisanal,
                ProductionStrategy::ReuseExistingSample,
            ],
            care_constraints: vec![
                CareConstraint::SpotCleanOnly,
                CareConstraint::PaddedOrFlatStorage,
                CareConstraint::RepairByQualifiedEmbellishmentSpecialist,
            ],
            circulation_routes: vec![
                CirculationRoute::OwnerRetained,
                CirculationRoute::RepairAndReturn,
                CirculationRoute::Resale,
            ],
        }
    }

    #[test]
    fn rare_use_couture_profile_is_valid_without_frequency_penalty() {
        let profile = couture_profile();
        assert_eq!(profile.use_pattern, UsePattern::EventSpecific);
        assert_eq!(profile.validate(), Ok(()));
        assert_eq!(profile.functional_unit, FunctionalUnitBasis::DistinctEvents);
    }

    #[test]
    fn display_or_archive_can_use_non_wear_service_units() {
        let profile = GarmentUseProfile {
            profile_id: "HERITAGE-CEREMONIAL-01".into(),
            archetype: GarmentArchetype::CeremonialOrHeritage,
            use_pattern: UsePattern::ArchivePreservation,
            functional_unit: FunctionalUnitBasis::ConservationYears,
            intended_service: "preserve historic and ceremonial integrity".into(),
            value_dimensions: vec![
                ValueDimension::HistoricalEvidence,
                ValueDimension::CulturalOrCeremonialMeaning,
            ],
            decoration_risks: vec![DecorationRisk::HistoricOrDegradingMaterial],
            specialist_assessment_required: true,
            production_strategies: vec![
                ProductionStrategy::CommissionedOrCeremonial,
                ProductionStrategy::MadeToOrder,
                ProductionStrategy::RepairOrAlterationFirst,
            ],
            care_constraints: vec![
                CareConstraint::HandSupportDuringHandling,
                CareConstraint::PaddedOrFlatStorage,
                CareConstraint::LightAndHumidityControlledStorage,
                CareConstraint::SpecialistConservationReview,
            ],
            circulation_routes: vec![
                CirculationRoute::ArchiveOrMuseumCollection,
                CirculationRoute::ExhibitionOrDisplay,
            ],
        };
        assert_eq!(profile.validate(), Ok(()));
        assert!(profile.has_non_wear_service_unit());
    }

    #[test]
    fn performance_costume_uses_performance_runs_not_default_washes() {
        let profile = GarmentUseProfile {
            profile_id: "STAGE-COSTUME-01".into(),
            archetype: GarmentArchetype::PerformanceOrCostume,
            use_pattern: UsePattern::PerformanceSchedule,
            functional_unit: FunctionalUnitBasis::PerformanceRuns,
            intended_service: "meet costume, movement and appearance needs across a run".into(),
            value_dimensions: vec![
                ValueDimension::PerformanceFunction,
                ValueDimension::AestheticExpression,
                ValueDimension::AccessibilityAndEaseOfUse,
            ],
            decoration_risks: vec![
                DecorationRisk::DetachableModularTrim,
                DecorationRisk::MixedMaterialTrim,
            ],
            specialist_assessment_required: false,
            production_strategies: vec![
                ProductionStrategy::RentalOrSharedInventory,
                ProductionStrategy::ReuseExistingSample,
            ],
            care_constraints: vec![
                CareConstraint::RemoveDetachableEmbellishmentsBeforeCleaning,
                CareConstraint::RepairByQualifiedEmbellishmentSpecialist,
            ],
            circulation_routes: vec![
                CirculationRoute::RentalOrCostumeHire,
                CirculationRoute::RepairAndReturn,
                CirculationRoute::TransformIntoAnotherArticle,
            ],
        };
        assert_eq!(profile.validate(), Ok(()));
        assert!(profile.has_non_wear_service_unit());
    }

    #[test]
    fn decorative_garment_can_declare_high_repairability_and_modularity() {
        let profile = GarmentUseProfile {
            profile_id: "MODULAR-STATEMENT-GARMENT-01".into(),
            archetype: GarmentArchetype::ModularDecorative,
            use_pattern: UsePattern::Occasional,
            functional_unit: FunctionalUnitBasis::DistinctEvents,
            intended_service:
                "distinct styled appearances with detachable decorative modules".into(),
            value_dimensions: vec![
                ValueDimension::AestheticExpression,
                ValueDimension::IdentityOrBelonging,
            ],
            decoration_risks: vec![DecorationRisk::DetachableModularTrim],
            specialist_assessment_required: false,
            production_strategies: vec![
                ProductionStrategy::MadeToOrder,
                ProductionStrategy::SmallBatch,
                ProductionStrategy::RepairOrAlterationFirst,
            ],
            care_constraints: vec![
                CareConstraint::RemoveDetachableEmbellishmentsBeforeCleaning,
                CareConstraint::SpotCleanOnly,
            ],
            circulation_routes: vec![
                CirculationRoute::AlterationOrRestyling,
                CirculationRoute::RepairAndReturn,
                CirculationRoute::TransformIntoAnotherArticle,
            ],
        };

        assert_eq!(profile.validate(), Ok(()));
        assert_eq!(profile.use_pattern, UsePattern::Occasional);
    }

    #[test]
    fn unknown_deadstock_composition_can_require_specialist_assessment() {
        let mut profile = couture_profile();
        profile.archetype = GarmentArchetype::DeadstockOrRunwaySample;
        profile.decoration_risks = vec![DecorationRisk::UnknownBlendOrDeadstockComposition];
        profile.specialist_assessment_required = true;

        assert_eq!(profile.validate(), Ok(()));
        assert!(profile.specialist_assessment_required);
    }

    #[test]
    fn production_strategy_distinguishes_one_off_from_forecast_batch() {
        let mut profile = couture_profile();
        assert!(profile
            .production_strategies
            .contains(&ProductionStrategy::OneOffArtisanal));
        profile.profile_id = "DEADSTOCK-RUNWAY-SAMPLE-01".into();
        profile.archetype = GarmentArchetype::DeadstockOrRunwaySample;
        profile.production_strategies = vec![
            ProductionStrategy::ReuseExistingSample,
            ProductionStrategy::PreorderOrDemandSignal,
        ];
        profile.circulation_routes.push(CirculationRoute::UnsoldInventoryReallocation);

        assert_eq!(profile.validate(), Ok(()));
        assert!(!profile
            .production_strategies
            .contains(&ProductionStrategy::ForecastDrivenBatch));
    }

    #[test]
    fn duplicate_production_strategy_is_rejected() {
        let mut profile = couture_profile();
        profile
            .production_strategies
            .push(ProductionStrategy::MadeToOrder);
        assert_eq!(
            profile.validate(),
            Err(GarmentProfileError::DuplicateProductionStrategy(
                ProductionStrategy::MadeToOrder
            ))
        );
    }

    #[test]
    fn duplicate_decoration_risks_are_rejected() {
        let mut profile = couture_profile();
        profile.decoration_risks.push(DecorationRisk::Beadwork);
        assert_eq!(
            profile.validate(),
            Err(GarmentProfileError::DuplicateDecorationRisk(
                DecorationRisk::Beadwork
            ))
        );
    }

    #[test]
    fn missing_named_value_dimension_is_rejected() {
        let mut profile = couture_profile();
        profile.value_dimensions.clear();
        assert_eq!(
            profile.validate(),
            Err(GarmentProfileError::MissingValueDimensions)
        );
    }

    #[test]
    fn duplicate_care_constraints_are_rejected() {
        let mut profile = couture_profile();
        profile
            .care_constraints
            .push(CareConstraint::SpotCleanOnly);
        assert_eq!(
            profile.validate(),
            Err(GarmentProfileError::DuplicateCareConstraint(
                CareConstraint::SpotCleanOnly
            ))
        );
    }

    #[test]
    fn custom_unit_cannot_be_empty() {
        let mut profile = couture_profile();
        profile.functional_unit = FunctionalUnitBasis::Custom("  ".into());
        assert_eq!(
            profile.validate(),
            Err(GarmentProfileError::EmptyCustomFunctionalUnit)
        );
    }

    #[test]
    fn low_frequency_is_not_an_implicit_failure_state() {
        let mut profile = couture_profile();
        profile.use_pattern = UsePattern::Occasional;
        assert_eq!(profile.validate(), Ok(()));
        assert_eq!(profile.use_pattern, UsePattern::Occasional);
    }
}
