// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Experimental-context comparability gates for cross-study fungal biology.
//!
//! A matching strain name does not make two studies directly comparable.
//! Host exposure, carbon source, medium, plate geometry, time origin,
//! acquisition cadence, readout and method are explicit context dimensions.
//!
//! This module does not estimate effects or authorize pooling. It classifies
//! whether a declared within-study single-factor contrast is structurally
//! supported, whether contexts differ on multiple axes, or whether metadata is
//! insufficient. Even a randomized contrast candidate still requires a
//! preregistered statistical analysis and source-qualified observations.

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StudyId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostExposure {
    Present,
    Absent,
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CarbonSourceProfile {
    NoExogenousSupplement,
    MyristicAcid { concentration_milli_molar: u32 },
    OtherDefined { source_code: String },
    MixedOrUnresolved,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExperimentTimeOrigin {
    SporeGermination,
    RootColonization,
    HostAttachment,
    AcquisitionStart,
    OtherDefined,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeasurementReadout {
    HyphalLength,
    NetworkArea,
    HyphalLengthDensity,
    BulkCytoplasmicFlowVelocity,
    FluorescentLipidCargoVelocity,
    FlowTrajectoryCount,
    SporeProduction,
    OtherDefined,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeasurementMethod {
    BrightfieldTimeLapse,
    Kymograph,
    FluorescenceKymograph,
    SoilCoreMicroscopy,
    ImageSegmentation,
    OtherDefined,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllocationMethod {
    Randomized,
    NotRandomized,
    NotReported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ContextFactor {
    Taxon,
    Strain,
    HostExposure,
    CarbonSource,
    MediumComposition,
    PlateGeometry,
    TimeOrigin,
    MeasurementReadout,
    MeasurementMethod,
    SamplingCadence,
    ObservationDuration,
    Temperature,
}

impl ContextFactor {
    /// Factors that can reasonably be declared as the single treatment axis
    /// in this design gate. Readout/method differences are never treatments.
    pub const fn is_allowed_intervention(self) -> bool {
        matches!(self, Self::HostExposure | Self::CarbonSource)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissingContextField {
    Taxon,
    Strain,
    HostExposure,
    CarbonSource,
    MediumFingerprint,
    PlateGeometryFingerprint,
    TimeOrigin,
    MeasurementReadout,
    MeasurementMethod,
    SamplingCadence,
    ObservationDuration,
    Temperature,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextSide {
    Left,
    Right,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StudyContextError {
    ZeroStudyId,
    EmptyTaxon,
    EmptyStrain,
    EmptyMediumFingerprint,
    EmptyPlateGeometryFingerprint,
    EmptyOtherCarbonSource,
    InvalidSamplingCadence,
    InvalidObservationDuration,
}

impl fmt::Display for StudyContextError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroStudyId => write!(f, "study identity must be non-zero"),
            Self::EmptyTaxon => write!(f, "fungal taxon must not be empty"),
            Self::EmptyStrain => write!(f, "fungal strain must not be empty"),
            Self::EmptyMediumFingerprint => write!(f, "medium fingerprint must not be empty"),
            Self::EmptyPlateGeometryFingerprint => {
                write!(f, "plate/geometry fingerprint must not be empty")
            }
            Self::EmptyOtherCarbonSource => {
                write!(f, "defined carbon source code must not be empty")
            }
            Self::InvalidSamplingCadence => {
                write!(f, "sampling cadence must be finite, positive and ordered")
            }
            Self::InvalidObservationDuration => {
                write!(f, "observation duration must be finite and positive")
            }
        }
    }
}

impl std::error::Error for StudyContextError {}

/// Exact acquisition/biological context for one experimental arm.
///
/// Fingerprints are source-defined canonical descriptors or digests. This
/// module never assumes that two missing fingerprints are equivalent.
#[derive(Debug, Clone, PartialEq)]
pub struct StudyContext {
    pub study: StudyId,
    pub fungal_taxon: Option<String>,
    pub fungal_strain: Option<String>,
    pub host_exposure: HostExposure,
    pub carbon_source: CarbonSourceProfile,
    pub medium_fingerprint: Option<String>,
    pub plate_geometry_fingerprint: Option<String>,
    pub time_origin: ExperimentTimeOrigin,
    pub measurement_readout: MeasurementReadout,
    pub measurement_method: MeasurementMethod,
    /// Lower and upper sampling intervals in minutes, as actually reported.
    pub sampling_interval_minutes: Option<(u32, u32)>,
    /// Duration from the declared time origin, in thousandths of a day.
    pub observation_duration_milli_days: Option<u32>,
    /// Temperature in tenths of a degree Celsius.
    pub temperature_tenths_c: Option<i32>,
    pub allocation: AllocationMethod,
}

impl StudyContext {
    pub fn validate(&self) -> Result<(), StudyContextError> {
        if self.study.0 == 0 {
            return Err(StudyContextError::ZeroStudyId);
        }
        if self
            .fungal_taxon
            .as_ref()
            .is_some_and(|value| value.trim().is_empty())
        {
            return Err(StudyContextError::EmptyTaxon);
        }
        if self
            .fungal_strain
            .as_ref()
            .is_some_and(|value| value.trim().is_empty())
        {
            return Err(StudyContextError::EmptyStrain);
        }
        if self
            .medium_fingerprint
            .as_ref()
            .is_some_and(|value| value.trim().is_empty())
        {
            return Err(StudyContextError::EmptyMediumFingerprint);
        }
        if self
            .plate_geometry_fingerprint
            .as_ref()
            .is_some_and(|value| value.trim().is_empty())
        {
            return Err(StudyContextError::EmptyPlateGeometryFingerprint);
        }
        if matches!(
            &self.carbon_source,
            CarbonSourceProfile::OtherDefined { source_code } if source_code.trim().is_empty()
        ) {
            return Err(StudyContextError::EmptyOtherCarbonSource);
        }
        if self
            .sampling_interval_minutes
            .is_some_and(|(min, max)| min == 0 || max == 0 || min > max)
        {
            return Err(StudyContextError::InvalidSamplingCadence);
        }
        if self
            .observation_duration_milli_days
            .is_some_and(|duration| duration == 0)
        {
            return Err(StudyContextError::InvalidObservationDuration);
        }
        Ok(())
    }

    pub fn missing_fields(&self) -> Vec<MissingContextField> {
        let mut missing = Vec::new();
        if self.fungal_taxon.is_none() {
            missing.push(MissingContextField::Taxon);
        }
        if self.fungal_strain.is_none() {
            missing.push(MissingContextField::Strain);
        }
        if self.host_exposure == HostExposure::Unknown {
            missing.push(MissingContextField::HostExposure);
        }
        if matches!(
            &self.carbon_source,
            CarbonSourceProfile::Unknown | CarbonSourceProfile::MixedOrUnresolved
        ) {
            missing.push(MissingContextField::CarbonSource);
        }
        if self.medium_fingerprint.is_none() {
            missing.push(MissingContextField::MediumFingerprint);
        }
        if self.plate_geometry_fingerprint.is_none() {
            missing.push(MissingContextField::PlateGeometryFingerprint);
        }
        if self.time_origin == ExperimentTimeOrigin::Unknown {
            missing.push(MissingContextField::TimeOrigin);
        }
        if self.measurement_readout == MeasurementReadout::Unknown {
            missing.push(MissingContextField::MeasurementReadout);
        }
        if self.measurement_method == MeasurementMethod::Unknown {
            missing.push(MissingContextField::MeasurementMethod);
        }
        if self.sampling_interval_minutes.is_none() {
            missing.push(MissingContextField::SamplingCadence);
        }
        if self.observation_duration_milli_days.is_none() {
            missing.push(MissingContextField::ObservationDuration);
        }
        if self.temperature_tenths_c.is_none() {
            missing.push(MissingContextField::Temperature);
        }
        missing
    }

    fn differences(&self, other: &Self) -> Vec<ContextFactor> {
        let mut differences = Vec::new();
        if self.fungal_taxon != other.fungal_taxon {
            differences.push(ContextFactor::Taxon);
        }
        if self.fungal_strain != other.fungal_strain {
            differences.push(ContextFactor::Strain);
        }
        if self.host_exposure != other.host_exposure {
            differences.push(ContextFactor::HostExposure);
        }
        if self.carbon_source != other.carbon_source {
            differences.push(ContextFactor::CarbonSource);
        }
        if self.medium_fingerprint != other.medium_fingerprint {
            differences.push(ContextFactor::MediumComposition);
        }
        if self.plate_geometry_fingerprint != other.plate_geometry_fingerprint {
            differences.push(ContextFactor::PlateGeometry);
        }
        if self.time_origin != other.time_origin {
            differences.push(ContextFactor::TimeOrigin);
        }
        if self.measurement_readout != other.measurement_readout {
            differences.push(ContextFactor::MeasurementReadout);
        }
        if self.measurement_method != other.measurement_method {
            differences.push(ContextFactor::MeasurementMethod);
        }
        if self.sampling_interval_minutes != other.sampling_interval_minutes {
            differences.push(ContextFactor::SamplingCadence);
        }
        if self.observation_duration_milli_days != other.observation_duration_milli_days {
            differences.push(ContextFactor::ObservationDuration);
        }
        if self.temperature_tenths_c != other.temperature_tenths_c {
            differences.push(ContextFactor::Temperature);
        }
        differences
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum StudyComparability {
    MatchedWithinStudyContext,
    /// Design-level qualification only; no effect size or causal conclusion.
    RandomizedSingleFactorContrastCandidate {
        factor: ContextFactor,
    },
    /// One declared intervention differs, but allocation was not randomized
    /// or was not reported. This is not a randomized causal contrast.
    NonRandomizedSingleFactorContrast {
        factor: ContextFactor,
    },
    NonComparableWithinStudy {
        differing_factors: Vec<ContextFactor>,
    },
    /// Even identical measured contexts from different studies are not
    /// automatically pooled; study/batch provenance remains distinct.
    CrossStudyContextShift {
        differing_factors: Vec<ContextFactor>,
    },
    InsufficientMetadata {
        left_missing: Vec<MissingContextField>,
        right_missing: Vec<MissingContextField>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StudyComparabilityError {
    InvalidContext {
        side: ContextSide,
        reason: StudyContextError,
    },
}

impl fmt::Display for StudyComparabilityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidContext { side, reason } => {
                write!(f, "invalid {:?} study context: {}", side, reason)
            }
        }
    }
}

impl std::error::Error for StudyComparabilityError {}

/// Assess whether two study arms can support a declared context contrast.
///
/// The only permitted single-factor contrast axes are host exposure and
/// carbon source. A difference in readout, method, strain, geometry, timing,
/// medium or temperature is never re-labelled as a treatment effect.
///
/// Different StudyId values always produce CrossStudyContextShift, even when
/// every listed context factor matches. This explicitly prevents automatic
/// pooling across study/batch boundaries.
pub fn assess_study_comparability(
    left: &StudyContext,
    right: &StudyContext,
    declared_intervention: Option<ContextFactor>,
) -> Result<StudyComparability, StudyComparabilityError> {
    left.validate()
        .map_err(|reason| StudyComparabilityError::InvalidContext {
            side: ContextSide::Left,
            reason,
        })?;
    right
        .validate()
        .map_err(|reason| StudyComparabilityError::InvalidContext {
            side: ContextSide::Right,
            reason,
        })?;

    let left_missing = left.missing_fields();
    let right_missing = right.missing_fields();
    if !left_missing.is_empty() || !right_missing.is_empty() {
        return Ok(StudyComparability::InsufficientMetadata {
            left_missing,
            right_missing,
        });
    }

    let differing_factors = left.differences(right);
    if left.study != right.study {
        return Ok(StudyComparability::CrossStudyContextShift { differing_factors });
    }
    if differing_factors.is_empty() {
        return Ok(StudyComparability::MatchedWithinStudyContext);
    }

    if differing_factors.len() == 1 {
        let factor = differing_factors[0];
        if declared_intervention == Some(factor) && factor.is_allowed_intervention() {
            if left.allocation == AllocationMethod::Randomized
                && right.allocation == AllocationMethod::Randomized
            {
                return Ok(StudyComparability::RandomizedSingleFactorContrastCandidate { factor });
            }
            return Ok(StudyComparability::NonRandomizedSingleFactorContrast { factor });
        }
    }

    Ok(StudyComparability::NonComparableWithinStudy { differing_factors })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context(study: u64) -> StudyContext {
        StudyContext {
            study: StudyId(study),
            fungal_taxon: Some("Rhizophagus irregularis".to_owned()),
            fungal_strain: Some("A5 / DAOM 664344".to_owned()),
            host_exposure: HostExposure::Absent,
            carbon_source: CarbonSourceProfile::NoExogenousSupplement,
            medium_fingerprint: Some("msr-0.3-phytagel-inorganic-n".to_owned()),
            plate_geometry_fingerprint: Some("split-plate-90mm-round".to_owned()),
            time_origin: ExperimentTimeOrigin::SporeGermination,
            measurement_readout: MeasurementReadout::BulkCytoplasmicFlowVelocity,
            measurement_method: MeasurementMethod::Kymograph,
            sampling_interval_minutes: Some((120, 240)),
            observation_duration_milli_days: Some(57_000),
            temperature_tenths_c: Some(250),
            allocation: AllocationMethod::Randomized,
        }
    }

    #[test]
    fn same_study_myristic_acid_arms_are_a_single_factor_randomized_candidate() {
        let left = context(1);
        let mut right = context(1);
        right.carbon_source = CarbonSourceProfile::MyristicAcid {
            concentration_milli_molar: 500,
        };
        assert_eq!(
            assess_study_comparability(&left, &right, Some(ContextFactor::CarbonSource)),
            Ok(
                StudyComparability::RandomizedSingleFactorContrastCandidate {
                    factor: ContextFactor::CarbonSource
                }
            )
        );
    }

    #[test]
    fn bulk_cytoplasm_and_lipid_cargo_are_distinct_outcomes() {
        let left = context(1);
        let mut right = context(1);
        right.measurement_readout = MeasurementReadout::FluorescentLipidCargoVelocity;
        assert_eq!(
            assess_study_comparability(&left, &right, Some(ContextFactor::MeasurementReadout)),
            Ok(StudyComparability::NonComparableWithinStudy {
                differing_factors: vec![ContextFactor::MeasurementReadout]
            })
        );
    }

    #[test]
    fn host_associated_vs_host_free_cross_study_is_not_a_host_effect() {
        let mut host_associated = context(2025);
        host_associated.host_exposure = HostExposure::Present;
        host_associated.carbon_source = CarbonSourceProfile::OtherDefined {
            source_code: "host-derived-carbon".to_owned(),
        };
        host_associated.medium_fingerprint = Some("root-organ-culture-medium".to_owned());
        host_associated.plate_geometry_fingerprint = Some("split-petri-fungi-arena".to_owned());
        host_associated.time_origin = ExperimentTimeOrigin::RootColonization;
        host_associated.measurement_method = MeasurementMethod::BrightfieldTimeLapse;

        let host_free = context(2026);
        let result = assess_study_comparability(
            &host_associated,
            &host_free,
            Some(ContextFactor::HostExposure),
        )
        .unwrap();
        let StudyComparability::CrossStudyContextShift { differing_factors } = result else {
            panic!("different studies must never be treated as a within-study host contrast");
        };
        assert!(differing_factors.contains(&ContextFactor::HostExposure));
        assert!(differing_factors.contains(&ContextFactor::CarbonSource));
        assert!(differing_factors.contains(&ContextFactor::MediumComposition));
        assert!(differing_factors.contains(&ContextFactor::PlateGeometry));
        assert!(differing_factors.contains(&ContextFactor::TimeOrigin));
    }

    #[test]
    fn identical_context_in_different_studies_is_not_automatically_pooled() {
        let left = context(1);
        let right = context(2);
        assert_eq!(
            assess_study_comparability(&left, &right, None),
            Ok(StudyComparability::CrossStudyContextShift {
                differing_factors: Vec::new()
            })
        );
    }

    #[test]
    fn missing_geometry_or_time_origin_fails_closed() {
        let left = context(1);
        let mut right = context(1);
        right.plate_geometry_fingerprint = None;
        right.time_origin = ExperimentTimeOrigin::Unknown;
        let result = assess_study_comparability(&left, &right, None).unwrap();
        let StudyComparability::InsufficientMetadata { right_missing, .. } = result else {
            panic!("missing context must not be considered a match");
        };
        assert!(right_missing.contains(&MissingContextField::PlateGeometryFingerprint));
        assert!(right_missing.contains(&MissingContextField::TimeOrigin));
    }

    #[test]
    fn two_context_differences_are_not_a_single_factor_contrast() {
        let left = context(1);
        let mut right = context(1);
        right.carbon_source = CarbonSourceProfile::MyristicAcid {
            concentration_milli_molar: 500,
        };
        right.plate_geometry_fingerprint = Some("rectangular-plate".to_owned());
        assert_eq!(
            assess_study_comparability(&left, &right, Some(ContextFactor::CarbonSource)),
            Ok(StudyComparability::NonComparableWithinStudy {
                differing_factors: vec![ContextFactor::CarbonSource, ContextFactor::PlateGeometry]
            })
        );
    }

    #[test]
    fn unreported_randomization_is_not_promoted_to_randomized_candidate() {
        let mut left = context(1);
        let mut right = context(1);
        left.allocation = AllocationMethod::NotReported;
        right.allocation = AllocationMethod::NotReported;
        right.carbon_source = CarbonSourceProfile::MyristicAcid {
            concentration_milli_molar: 500,
        };
        assert_eq!(
            assess_study_comparability(&left, &right, Some(ContextFactor::CarbonSource)),
            Ok(StudyComparability::NonRandomizedSingleFactorContrast {
                factor: ContextFactor::CarbonSource
            })
        );
    }

    #[test]
    fn different_strains_remain_separate_strata() {
        let left = context(1);
        let mut right = context(1);
        right.fungal_strain = Some("C2 / DAOM 664346".to_owned());
        assert_eq!(
            assess_study_comparability(&left, &right, Some(ContextFactor::HostExposure)),
            Ok(StudyComparability::NonComparableWithinStudy {
                differing_factors: vec![ContextFactor::Strain]
            })
        );
    }

    #[test]
    fn cadence_and_duration_differences_are_explicit() {
        let left = context(1);
        let mut right = context(1);
        right.sampling_interval_minutes = Some((30, 60));
        right.observation_duration_milli_days = Some(20_000);
        assert_eq!(
            assess_study_comparability(&left, &right, None),
            Ok(StudyComparability::NonComparableWithinStudy {
                differing_factors: vec![
                    ContextFactor::SamplingCadence,
                    ContextFactor::ObservationDuration
                ]
            })
        );
    }

    #[test]
    fn invalid_sampling_cadence_is_rejected() {
        let mut left = context(1);
        left.sampling_interval_minutes = Some((240, 120));
        assert_eq!(
            assess_study_comparability(&left, &context(1), None),
            Err(StudyComparabilityError::InvalidContext {
                side: ContextSide::Left,
                reason: StudyContextError::InvalidSamplingCadence
            })
        );
    }

    #[test]
    fn non_intervention_context_factor_cannot_be_declared_as_treatment() {
        let left = context(1);
        let mut right = context(1);
        right.measurement_method = MeasurementMethod::FluorescenceKymograph;
        assert_eq!(
            assess_study_comparability(&left, &right, Some(ContextFactor::MeasurementMethod)),
            Ok(StudyComparability::NonComparableWithinStudy {
                differing_factors: vec![ContextFactor::MeasurementMethod]
            })
        );
    }
}
