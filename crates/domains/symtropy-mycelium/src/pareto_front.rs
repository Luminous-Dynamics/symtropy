// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Host-stratified Pareto analysis for AM fungal exploration/exploitation.
//!
//! The PNAS 2026 phenotype plane uses hyphal surface-area density (rho_S,
//! m^2/m^3 = m^-1) and travelling-wave speed (m/hour). Both are maximized.
//! Host genotypes are never pooled: the published result reports host-dependent
//! front shifts. Fungal strain and growth-unit identities remain on each point.
//! Successive time windows from one plate are repeated observations, not
//! independent replicates. This module performs descriptive geometry only.
//!
//! Hypervolume and additive epsilon are dimensionless because objective scales
//! are explicit inputs. The reference point is also explicit; metrics from
//! different scales/reference points must not be compared directly.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ParetoSampleId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PlantGenotypeId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FungalStrainId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GrowthUnitId(pub u64);

/// Canonical physical units: surface area density in m^-1 and speed in m/hour.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParetoObjectives {
    pub surface_area_density_per_m: f64,
    pub wave_speed_m_per_hour: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParetoPhenotypeSample {
    pub id: ParetoSampleId,
    pub plant_genotype: PlantGenotypeId,
    pub fungal_strain: FungalStrainId,
    /// Complete plate/growth replicate. Repeated windows share this identity.
    pub growth_unit: GrowthUnitId,
    pub window_start_hours: f64,
    pub window_end_hours: f64,
    pub objectives: ParetoObjectives,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParetoNormalization {
    pub surface_area_density_scale_per_m: f64,
    pub wave_speed_scale_m_per_hour: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParetoReferencePoint {
    pub surface_area_density_per_m: f64,
    pub wave_speed_m_per_hour: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HostParetoFront {
    pub plant_genotype: PlantGenotypeId,
    /// Sorted by density ascending, speed descending, then sample identity.
    pub non_dominated: Vec<ParetoPhenotypeSample>,
    pub dominated_sample_ids: Vec<ParetoSampleId>,
    /// Dimensionless 2D dominated hypervolume relative to the supplied origin.
    pub normalized_hypervolume: f64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParetoAnalysisError {
    EmptySamples,
    EmptyApproximation,
    EmptyReferenceFront,
    DuplicateSampleId(ParetoSampleId),
    InvalidTimeWindow(ParetoSampleId),
    InvalidObjective(ParetoSampleId),
    InvalidGrowthUnitIdentity(GrowthUnitId),
    OverlappingGrowthUnitWindows {
        growth_unit: GrowthUnitId,
        earlier: ParetoSampleId,
        later: ParetoSampleId,
    },
    InvalidNormalization,
    InvalidReferencePoint,
    ReferencePointNotDominated(ParetoSampleId),
    MixedHostGenotypes,
}

impl fmt::Display for ParetoAnalysisError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptySamples => write!(f, "Pareto analysis requires at least one sample"),
            Self::EmptyApproximation => {
                write!(f, "approximation front must contain at least one sample")
            }
            Self::EmptyReferenceFront => {
                write!(f, "reference front must contain at least one sample")
            }
            Self::DuplicateSampleId(id) => {
                write!(f, "duplicate Pareto phenotype sample identity {:?}", id)
            }
            Self::InvalidTimeWindow(id) => {
                write!(f, "sample {:?} has an invalid time window", id)
            }
            Self::InvalidObjective(id) => {
                write!(
                    f,
                    "sample {:?} has a non-finite or non-positive objective",
                    id
                )
            }
            Self::InvalidGrowthUnitIdentity(id) => {
                write!(
                    f,
                    "growth unit {:?} changes host genotype or fungal strain",
                    id
                )
            }
            Self::OverlappingGrowthUnitWindows {
                growth_unit,
                earlier,
                later,
            } => write!(
                f,
                "time windows for growth unit {:?} overlap between samples {:?} and {:?}",
                growth_unit, earlier, later
            ),
            Self::InvalidNormalization => {
                write!(f, "Pareto objective scales must be finite and positive")
            }
            Self::InvalidReferencePoint => {
                write!(
                    f,
                    "Pareto reference coordinates must be finite and non-negative"
                )
            }
            Self::ReferencePointNotDominated(id) => write!(
                f,
                "Pareto reference point is not weakly dominated by sample {:?}",
                id
            ),
            Self::MixedHostGenotypes => {
                write!(f, "Pareto front comparison cannot mix plant host genotypes")
            }
        }
    }
}

impl std::error::Error for ParetoAnalysisError {}

impl ParetoNormalization {
    pub fn validate(self) -> Result<(), ParetoAnalysisError> {
        if !self.surface_area_density_scale_per_m.is_finite()
            || self.surface_area_density_scale_per_m <= 0.0
            || !self.wave_speed_scale_m_per_hour.is_finite()
            || self.wave_speed_scale_m_per_hour <= 0.0
        {
            return Err(ParetoAnalysisError::InvalidNormalization);
        }
        Ok(())
    }
}

impl ParetoReferencePoint {
    pub fn validate(self) -> Result<(), ParetoAnalysisError> {
        if !self.surface_area_density_per_m.is_finite()
            || self.surface_area_density_per_m < 0.0
            || !self.wave_speed_m_per_hour.is_finite()
            || self.wave_speed_m_per_hour < 0.0
        {
            return Err(ParetoAnalysisError::InvalidReferencePoint);
        }
        Ok(())
    }
}

fn validate_samples(samples: &[ParetoPhenotypeSample]) -> Result<(), ParetoAnalysisError> {
    if samples.is_empty() {
        return Err(ParetoAnalysisError::EmptySamples);
    }

    let mut sample_ids = BTreeSet::new();
    let mut growth_unit_identity =
        BTreeMap::<GrowthUnitId, (PlantGenotypeId, FungalStrainId)>::new();
    let mut growth_unit_windows = BTreeMap::<GrowthUnitId, Vec<(f64, f64, ParetoSampleId)>>::new();

    for sample in samples {
        if !sample_ids.insert(sample.id) {
            return Err(ParetoAnalysisError::DuplicateSampleId(sample.id));
        }
        if !sample.window_start_hours.is_finite()
            || !sample.window_end_hours.is_finite()
            || sample.window_start_hours < 0.0
            || sample.window_end_hours <= sample.window_start_hours
        {
            return Err(ParetoAnalysisError::InvalidTimeWindow(sample.id));
        }
        if !sample.objectives.surface_area_density_per_m.is_finite()
            || sample.objectives.surface_area_density_per_m <= 0.0
            || !sample.objectives.wave_speed_m_per_hour.is_finite()
            || sample.objectives.wave_speed_m_per_hour <= 0.0
        {
            return Err(ParetoAnalysisError::InvalidObjective(sample.id));
        }

        let identity = (sample.plant_genotype, sample.fungal_strain);
        if let Some(existing) = growth_unit_identity.insert(sample.growth_unit, identity)
            && existing != identity
        {
            return Err(ParetoAnalysisError::InvalidGrowthUnitIdentity(
                sample.growth_unit,
            ));
        }
        growth_unit_windows
            .entry(sample.growth_unit)
            .or_default()
            .push((
                sample.window_start_hours,
                sample.window_end_hours,
                sample.id,
            ));
    }

    for (growth_unit, windows) in &mut growth_unit_windows {
        windows.sort_by(|left, right| {
            left.0
                .total_cmp(&right.0)
                .then_with(|| left.1.total_cmp(&right.1))
                .then_with(|| left.2.cmp(&right.2))
        });
        for pair in windows.windows(2) {
            if pair[1].0 < pair[0].1 {
                return Err(ParetoAnalysisError::OverlappingGrowthUnitWindows {
                    growth_unit: *growth_unit,
                    earlier: pair[0].2,
                    later: pair[1].2,
                });
            }
        }
    }

    Ok(())
}

/// Strict Pareto dominance for two maximized objectives.
fn dominates(left: ParetoObjectives, right: ParetoObjectives) -> bool {
    left.surface_area_density_per_m >= right.surface_area_density_per_m
        && left.wave_speed_m_per_hour >= right.wave_speed_m_per_hour
        && (left.surface_area_density_per_m > right.surface_area_density_per_m
            || left.wave_speed_m_per_hour > right.wave_speed_m_per_hour)
}

fn validate_reference_coverage(
    samples: &[ParetoPhenotypeSample],
    reference: ParetoReferencePoint,
) -> Result<(), ParetoAnalysisError> {
    for sample in samples {
        if sample.objectives.surface_area_density_per_m < reference.surface_area_density_per_m
            || sample.objectives.wave_speed_m_per_hour < reference.wave_speed_m_per_hour
        {
            return Err(ParetoAnalysisError::ReferencePointNotDominated(sample.id));
        }
    }
    Ok(())
}

fn normalized_hypervolume(
    front: &[ParetoPhenotypeSample],
    reference: ParetoReferencePoint,
    normalization: ParetoNormalization,
) -> f64 {
    let reference_x =
        reference.surface_area_density_per_m / normalization.surface_area_density_scale_per_m;
    let reference_y = reference.wave_speed_m_per_hour / normalization.wave_speed_scale_m_per_hour;
    let mut previous_x = reference_x;
    let mut area = 0.0;

    for sample in front {
        let x = sample.objectives.surface_area_density_per_m
            / normalization.surface_area_density_scale_per_m;
        let y = sample.objectives.wave_speed_m_per_hour / normalization.wave_speed_scale_m_per_hour;
        area += (x - previous_x).max(0.0) * (y - reference_y);
        previous_x = x;
    }

    area
}

/// Compute a non-dominated frontier independently for every plant genotype.
///
/// Multiple time windows from one plate are retained as observations with the
/// same growth-unit identity; this method does not treat them as independent
/// biological replicates.
pub fn analyze_by_plant_genotype(
    samples: &[ParetoPhenotypeSample],
    reference: ParetoReferencePoint,
    normalization: ParetoNormalization,
) -> Result<Vec<HostParetoFront>, ParetoAnalysisError> {
    validate_samples(samples)?;
    reference.validate()?;
    normalization.validate()?;
    validate_reference_coverage(samples, reference)?;

    let mut by_host = BTreeMap::<PlantGenotypeId, Vec<ParetoPhenotypeSample>>::new();
    for sample in samples {
        by_host
            .entry(sample.plant_genotype)
            .or_default()
            .push(*sample);
    }

    let mut results = Vec::with_capacity(by_host.len());
    for (plant_genotype, host_samples) in by_host {
        let mut front = Vec::new();
        let mut dominated = Vec::new();

        for candidate in &host_samples {
            let is_dominated = host_samples.iter().any(|other| {
                other.id != candidate.id && dominates(other.objectives, candidate.objectives)
            });
            if is_dominated {
                dominated.push(candidate.id);
            } else {
                front.push(*candidate);
            }
        }

        front.sort_by(|left, right| {
            left.objectives
                .surface_area_density_per_m
                .total_cmp(&right.objectives.surface_area_density_per_m)
                .then_with(|| {
                    right
                        .objectives
                        .wave_speed_m_per_hour
                        .total_cmp(&left.objectives.wave_speed_m_per_hour)
                })
                .then_with(|| left.id.cmp(&right.id))
        });
        dominated.sort_unstable();

        results.push(HostParetoFront {
            plant_genotype,
            normalized_hypervolume: normalized_hypervolume(&front, reference, normalization),
            non_dominated: front,
            dominated_sample_ids: dominated,
        });
    }

    Ok(results)
}

/// Additive epsilon indicator for two maximization fronts.
///
/// For every reference point, find the smallest normalized additive shift that
/// lets an approximation point weakly dominate it; return the worst such
/// shift. Negative values mean the approximation exceeds the reference front.
/// Both inputs must belong to the same plant genotype.
pub fn additive_epsilon_indicator(
    approximation: &[ParetoPhenotypeSample],
    reference: &[ParetoPhenotypeSample],
    normalization: ParetoNormalization,
) -> Result<f64, ParetoAnalysisError> {
    if approximation.is_empty() {
        return Err(ParetoAnalysisError::EmptyApproximation);
    }
    if reference.is_empty() {
        return Err(ParetoAnalysisError::EmptyReferenceFront);
    }
    normalization.validate()?;
    validate_samples(approximation)?;
    validate_samples(reference)?;

    let host = approximation[0].plant_genotype;
    if approximation
        .iter()
        .any(|sample| sample.plant_genotype != host)
        || reference.iter().any(|sample| sample.plant_genotype != host)
    {
        return Err(ParetoAnalysisError::MixedHostGenotypes);
    }

    let mut worst = f64::NEG_INFINITY;
    for target in reference {
        let mut best_for_target = f64::INFINITY;
        for candidate in approximation {
            let density_gap = (target.objectives.surface_area_density_per_m
                - candidate.objectives.surface_area_density_per_m)
                / normalization.surface_area_density_scale_per_m;
            let speed_gap = (target.objectives.wave_speed_m_per_hour
                - candidate.objectives.wave_speed_m_per_hour)
                / normalization.wave_speed_scale_m_per_hour;
            best_for_target = best_for_target.min(density_gap.max(speed_gap));
        }
        worst = worst.max(best_for_target);
    }

    Ok(worst)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(
        id: u64,
        host: u64,
        strain: u64,
        growth_unit: u64,
        objectives: (f64, f64),
        window: (f64, f64),
    ) -> ParetoPhenotypeSample {
        ParetoPhenotypeSample {
            id: ParetoSampleId(id),
            plant_genotype: PlantGenotypeId(host),
            fungal_strain: FungalStrainId(strain),
            growth_unit: GrowthUnitId(growth_unit),
            window_start_hours: window.0,
            window_end_hours: window.1,
            objectives: ParetoObjectives {
                surface_area_density_per_m: objectives.0,
                wave_speed_m_per_hour: objectives.1,
            },
        }
    }

    fn normalization() -> ParetoNormalization {
        ParetoNormalization {
            surface_area_density_scale_per_m: 1.0,
            wave_speed_scale_m_per_hour: 1.0,
        }
    }

    fn origin() -> ParetoReferencePoint {
        ParetoReferencePoint {
            surface_area_density_per_m: 0.0,
            wave_speed_m_per_hour: 0.0,
        }
    }

    #[test]
    fn known_tradeoff_front_and_hypervolume_match_hand_calculation() {
        let samples = [
            sample(1, 1, 10, 1, (1.0, 3.0), (0.0, 10.0)),
            sample(2, 1, 10, 2, (2.0, 2.0), (0.0, 10.0)),
            sample(3, 1, 10, 3, (3.0, 1.0), (0.0, 10.0)),
            sample(4, 1, 10, 4, (1.0, 1.0), (0.0, 10.0)),
        ];

        let result = analyze_by_plant_genotype(&samples, origin(), normalization()).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(
            result[0]
                .non_dominated
                .iter()
                .map(|item| item.id)
                .collect::<Vec<_>>(),
            vec![ParetoSampleId(1), ParetoSampleId(2), ParetoSampleId(3)]
        );
        assert_eq!(result[0].dominated_sample_ids, vec![ParetoSampleId(4)]);
        assert!((result[0].normalized_hypervolume - 6.0).abs() < 1e-12);
    }

    #[test]
    fn host_genotypes_receive_separate_fronts_and_hypervolumes() {
        let samples = [
            sample(1, 1, 10, 1, (1.0, 2.0), (0.0, 10.0)),
            sample(2, 1, 10, 1, (2.0, 1.0), (10.0, 20.0)),
            sample(3, 2, 10, 2, (2.0, 3.0), (0.0, 10.0)),
            sample(4, 2, 10, 2, (3.0, 2.0), (10.0, 20.0)),
        ];

        let result = analyze_by_plant_genotype(&samples, origin(), normalization()).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].plant_genotype, PlantGenotypeId(1));
        assert_eq!(result[1].plant_genotype, PlantGenotypeId(2));
        assert_eq!(result[0].non_dominated.len(), 2);
        assert_eq!(result[1].non_dominated.len(), 2);
    }

    #[test]
    fn host_fronts_are_independent_of_input_order() {
        let samples = [
            sample(1, 1, 10, 1, (1.0, 3.0), (0.0, 10.0)),
            sample(2, 1, 10, 2, (2.0, 2.0), (0.0, 10.0)),
            sample(3, 1, 10, 3, (3.0, 1.0), (0.0, 10.0)),
            sample(4, 2, 11, 4, (2.0, 2.0), (0.0, 10.0)),
            sample(5, 2, 11, 5, (3.0, 1.5), (0.0, 10.0)),
        ];
        let reversed = samples.iter().rev().copied().collect::<Vec<_>>();

        assert_eq!(
            analyze_by_plant_genotype(&samples, origin(), normalization()).unwrap(),
            analyze_by_plant_genotype(&reversed, origin(), normalization()).unwrap()
        );
    }

    #[test]
    fn repeated_time_windows_remain_on_the_same_growth_unit() {
        let samples = [
            sample(1, 1, 10, 7, (1.0, 3.0), (0.0, 10.0)),
            sample(2, 1, 10, 7, (2.0, 2.0), (10.0, 20.0)),
            sample(3, 1, 10, 7, (3.0, 1.0), (20.0, 30.0)),
        ];

        let result = analyze_by_plant_genotype(&samples, origin(), normalization()).unwrap();
        assert_eq!(result[0].non_dominated.len(), 3);
        assert!(result[0].dominated_sample_ids.is_empty());
    }

    #[test]
    fn overlapping_time_windows_within_a_growth_unit_are_rejected() {
        let samples = [
            sample(1, 1, 10, 7, (1.0, 3.0), (0.0, 10.0)),
            sample(2, 1, 10, 7, (2.0, 2.0), (9.0, 19.0)),
        ];
        assert_eq!(
            analyze_by_plant_genotype(&samples, origin(), normalization()),
            Err(ParetoAnalysisError::OverlappingGrowthUnitWindows {
                growth_unit: GrowthUnitId(7),
                earlier: ParetoSampleId(1),
                later: ParetoSampleId(2),
            })
        );
    }

    #[test]
    fn adjacent_time_windows_within_a_growth_unit_are_allowed() {
        let samples = [
            sample(1, 1, 10, 7, (1.0, 3.0), (0.0, 10.0)),
            sample(2, 1, 10, 7, (2.0, 2.0), (10.0, 20.0)),
        ];
        assert!(analyze_by_plant_genotype(&samples, origin(), normalization()).is_ok());
    }

    #[test]
    fn a_growth_unit_cannot_change_host_or_fungal_identity() {
        let samples = [
            sample(1, 1, 10, 7, (1.0, 3.0), (0.0, 10.0)),
            sample(2, 2, 10, 7, (2.0, 2.0), (10.0, 20.0)),
        ];
        assert_eq!(
            analyze_by_plant_genotype(&samples, origin(), normalization()),
            Err(ParetoAnalysisError::InvalidGrowthUnitIdentity(
                GrowthUnitId(7)
            ))
        );

        let mixed_strain = [
            sample(1, 1, 10, 7, (1.0, 3.0), (0.0, 10.0)),
            sample(2, 1, 11, 7, (2.0, 2.0), (10.0, 20.0)),
        ];
        assert_eq!(
            analyze_by_plant_genotype(&mixed_strain, origin(), normalization()),
            Err(ParetoAnalysisError::InvalidGrowthUnitIdentity(
                GrowthUnitId(7)
            ))
        );
    }

    #[test]
    fn duplicate_sample_ids_are_rejected() {
        let samples = [
            sample(1, 1, 10, 1, (1.0, 3.0), (0.0, 10.0)),
            sample(1, 1, 10, 2, (2.0, 2.0), (0.0, 10.0)),
        ];
        assert_eq!(
            analyze_by_plant_genotype(&samples, origin(), normalization()),
            Err(ParetoAnalysisError::DuplicateSampleId(ParetoSampleId(1)))
        );
    }

    #[test]
    fn invalid_objectives_and_time_windows_are_rejected() {
        let mut item = sample(1, 1, 10, 1, (0.0, 3.0), (0.0, 10.0));
        assert_eq!(
            analyze_by_plant_genotype(&[item], origin(), normalization()),
            Err(ParetoAnalysisError::InvalidObjective(ParetoSampleId(1)))
        );

        item = sample(2, 1, 10, 2, (f64::NAN, 1.0), (0.0, 10.0));
        assert_eq!(
            analyze_by_plant_genotype(&[item], origin(), normalization()),
            Err(ParetoAnalysisError::InvalidObjective(ParetoSampleId(2)))
        );

        item = sample(3, 1, 10, 3, (1.0, 1.0), (10.0, 10.0));
        assert_eq!(
            analyze_by_plant_genotype(&[item], origin(), normalization()),
            Err(ParetoAnalysisError::InvalidTimeWindow(ParetoSampleId(3)))
        );
    }

    #[test]
    fn normalization_and_reference_point_are_validated() {
        let samples = [sample(1, 1, 10, 1, (1.0, 1.0), (0.0, 10.0))];
        let bad_normalization = ParetoNormalization {
            surface_area_density_scale_per_m: 0.0,
            wave_speed_scale_m_per_hour: 1.0,
        };
        assert_eq!(
            analyze_by_plant_genotype(&samples, origin(), bad_normalization),
            Err(ParetoAnalysisError::InvalidNormalization)
        );

        let bad_reference = ParetoReferencePoint {
            surface_area_density_per_m: f64::NAN,
            wave_speed_m_per_hour: 0.0,
        };
        assert_eq!(
            analyze_by_plant_genotype(&samples, bad_reference, normalization()),
            Err(ParetoAnalysisError::InvalidReferencePoint)
        );
    }

    #[test]
    fn reference_point_must_be_weakly_dominated_by_every_sample() {
        let samples = [sample(1, 1, 10, 1, (1.0, 1.0), (0.0, 10.0))];
        let reference = ParetoReferencePoint {
            surface_area_density_per_m: 0.5,
            wave_speed_m_per_hour: 1.5,
        };
        assert_eq!(
            analyze_by_plant_genotype(&samples, reference, normalization()),
            Err(ParetoAnalysisError::ReferencePointNotDominated(
                ParetoSampleId(1)
            ))
        );
    }

    #[test]
    fn additive_epsilon_matches_known_positive_and_negative_values() {
        let approximation = [sample(1, 1, 10, 1, (2.0, 2.0), (0.0, 10.0))];
        let reference = [sample(2, 1, 10, 2, (3.0, 4.0), (0.0, 10.0))];
        assert_eq!(
            additive_epsilon_indicator(&approximation, &reference, normalization()),
            Ok(2.0)
        );

        let dominated_reference = [sample(3, 1, 10, 3, (1.0, 1.0), (0.0, 10.0))];
        assert_eq!(
            additive_epsilon_indicator(&approximation, &dominated_reference, normalization()),
            Ok(-1.0)
        );
    }

    #[test]
    fn epsilon_comparison_rejects_mixed_host_genotypes() {
        let approximation = [sample(1, 1, 10, 1, (2.0, 2.0), (0.0, 10.0))];
        let reference = [sample(2, 2, 10, 2, (3.0, 4.0), (0.0, 10.0))];
        assert_eq!(
            additive_epsilon_indicator(&approximation, &reference, normalization()),
            Err(ParetoAnalysisError::MixedHostGenotypes)
        );
    }
}
