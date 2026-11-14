// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

#![forbid(unsafe_code)]

//! Sensitivity analysis for author-derived HIST-001 Table 2 population
//! estimates. This crate does not infer a census or simulate demographics.

use core::fmt;

/// A phase row as reported in Table 2.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PublishedPhase {
    pub name: &'static str,
    pub homesteads: u32,
    pub duration_years: f64,
    pub published_general_population: f64,
    /// Published capital estimate. None means the paper prints "none".
    pub capital_population: Option<u32>,
}

/// Assumptions used by the published homestead estimator.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HomesteadAssumptions {
    pub persons_per_homestead: f64,
    pub homestead_use_years: f64,
}

/// One calculated subinterval.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PopulationEstimate {
    pub phase: &'static str,
    pub subinterval: &'static str,
    pub duration_years: f64,
    pub general_population: f64,
    pub capital_population: Option<u32>,
}

impl PopulationEstimate {
    pub fn total_population_if_capital_added(self) -> Option<f64> {
        self.capital_population
            .map(|capital| self.general_population + f64::from(capital))
    }
}

/// Invalid assumptions for the published estimator.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EstimateError {
    NonPositivePersonsPerHomestead,
    NonPositiveHomesteadUseYears,
    NonPositivePhaseDuration,
    InvalidK2Split,
    NonFiniteInput,
}

impl fmt::Display for EstimateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonPositivePersonsPerHomestead =>
                write!(f, "persons per homestead must be positive"),
            Self::NonPositiveHomesteadUseYears =>
                write!(f, "homestead use duration must be positive"),
            Self::NonPositivePhaseDuration =>
                write!(f, "phase duration must be positive"),
            Self::InvalidK2Split =>
                write!(f, "K2 initial and later subintervals must be positive and sum to the phase duration"),
            Self::NonFiniteInput =>
                write!(f, "estimator inputs must be finite"),
        }
    }
}

impl std::error::Error for EstimateError {}

fn validate(
    assumptions: HomesteadAssumptions,
    duration_years: f64,
) -> Result<(), EstimateError> {
    if !assumptions.persons_per_homestead.is_finite()
        || !assumptions.homestead_use_years.is_finite()
        || !duration_years.is_finite()
    {
        return Err(EstimateError::NonFiniteInput);
    }
    if assumptions.persons_per_homestead <= 0.0 {
        return Err(EstimateError::NonPositivePersonsPerHomestead);
    }
    if assumptions.homestead_use_years <= 0.0 {
        return Err(EstimateError::NonPositiveHomesteadUseYears);
    }
    if duration_years <= 0.0 {
        return Err(EstimateError::NonPositivePhaseDuration);
    }
    Ok(())
}

/// Calculate one ordinary Table 2 phase using the paper's homestead rule.
pub fn estimate_phase(
    phase: PublishedPhase,
    assumptions: HomesteadAssumptions,
) -> Result<PopulationEstimate, EstimateError> {
    validate(assumptions, phase.duration_years)?;
    let total_homestead_people =
        f64::from(phase.homesteads) * assumptions.persons_per_homestead;
    let temporal_units = phase.duration_years / assumptions.homestead_use_years;
    let general_population = total_homestead_people / temporal_units;

    Ok(PopulationEstimate {
        phase: phase.name,
        subinterval: "reported phase interval",
        duration_years: phase.duration_years,
        general_population,
        capital_population: phase.capital_population,
    })
}

/// K2 is a special case: the published table separates its first 25 years
/// from the subsequent 125 years because the capital was initially smaller.
///
/// The initial interval is kept fixed at 25 calendar years. Its general
/// population is allocated as that fraction of the full-phase homestead
/// population. The later interval receives the remaining homestead-persons,
/// divided by its duration in homestead-use units.
pub fn estimate_k2(
    homesteads: u32,
    total_duration_years: f64,
    initial_duration_years: f64,
    initial_capital_population: u32,
    later_capital_population: u32,
    assumptions: HomesteadAssumptions,
) -> Result<[PopulationEstimate; 2], EstimateError> {
    validate(assumptions, total_duration_years)?;
    if !initial_duration_years.is_finite()
        || initial_duration_years <= 0.0
        || initial_duration_years >= total_duration_years
    {
        return Err(EstimateError::InvalidK2Split);
    }

    let later_duration_years = total_duration_years - initial_duration_years;
    let total_homestead_people =
        f64::from(homesteads) * assumptions.persons_per_homestead;
    let initial_general =
        total_homestead_people * initial_duration_years / total_duration_years;
    let remaining_homestead_people = total_homestead_people - initial_general;
    let later_units = later_duration_years / assumptions.homestead_use_years;
    let later_general = remaining_homestead_people / later_units;

    Ok([
        PopulationEstimate {
            phase: "K2",
            subinterval: "initial smaller-capital interval",
            duration_years: initial_duration_years,
            general_population: initial_general,
            capital_population: Some(initial_capital_population),
        },
        PopulationEstimate {
            phase: "K2",
            subinterval: "later interval",
            duration_years: later_duration_years,
            general_population: later_general,
            capital_population: Some(later_capital_population),
        },
    ])
}

/// Published phase inputs. Capital values are held fixed during this
/// sensitivity analysis; only the general-population estimator is varied.
pub const PUBLISHED_PHASES: [PublishedPhase; 4] = [
    PublishedPhase {
        name: "Mapungubwe",
        homesteads: 114,
        duration_years: 50.0,
        published_general_population: 5700.0,
        capital_population: Some(5000),
    },
    PublishedPhase {
        name: "Transitional",
        homesteads: 143,
        duration_years: 120.0,
        published_general_population: 2979.0,
        capital_population: Some(2500),
    },
    PublishedPhase {
        name: "Leokwe",
        homesteads: 63,
        duration_years: 200.0,
        published_general_population: 787.0,
        capital_population: None,
    },
    PublishedPhase {
        name: "Zhizo",
        homesteads: 22,
        duration_years: 100.0,
        published_general_population: 550.0,
        capital_population: Some(300),
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> HomesteadAssumptions {
        HomesteadAssumptions {
            persons_per_homestead: 50.0,
            homestead_use_years: 50.0,
        }
    }

    #[test]
    fn reproduces_published_ordinary_phase_estimates_within_one_person() {
        for phase in PUBLISHED_PHASES {
            let estimate = estimate_phase(phase, base()).unwrap();
            assert!(
                (estimate.general_population - phase.published_general_population).abs() <= 1.0,
                "{}: calculated {}, published {}",
                phase.name,
                estimate.general_population,
                phase.published_general_population
            );
        }
    }

    #[test]
    fn reproduces_both_published_k2_subinterval_estimates() {
        let estimates = estimate_k2(153, 150.0, 25.0, 300, 1500, base()).unwrap();
        assert!((estimates[0].general_population - 1275.0).abs() < 1e-9);
        assert!((estimates[1].general_population - 2550.0).abs() < 1e-9);
        assert_eq!(estimates[0].duration_years + estimates[1].duration_years, 150.0);
        assert_eq!(estimates[0].total_population_if_capital_added(), Some(1575.0));
        assert_eq!(estimates[1].total_population_if_capital_added(), Some(4050.0));
    }

    #[test]
    fn population_estimate_increases_monotonically_with_people_and_use_duration() {
        let phase = PUBLISHED_PHASES[1];
        let low = estimate_phase(
            phase,
            HomesteadAssumptions {
                persons_per_homestead: 40.0,
                homestead_use_years: 40.0,
            },
        )
        .unwrap();
        let base = estimate_phase(phase, self::base()).unwrap();
        let high = estimate_phase(
            phase,
            HomesteadAssumptions {
                persons_per_homestead: 60.0,
                homestead_use_years: 60.0,
            },
        )
        .unwrap();

        assert!(low.general_population < base.general_population);
        assert!(base.general_population < high.general_population);
    }

    #[test]
    fn rejects_non_positive_and_non_finite_assumptions() {
        let phase = PUBLISHED_PHASES[0];
        assert_eq!(
            estimate_phase(
                phase,
                HomesteadAssumptions {
                    persons_per_homestead: 0.0,
                    homestead_use_years: 50.0,
                }
            ),
            Err(EstimateError::NonPositivePersonsPerHomestead)
        );
        assert_eq!(
            estimate_phase(
                phase,
                HomesteadAssumptions {
                    persons_per_homestead: 50.0,
                    homestead_use_years: f64::NAN,
                }
            ),
            Err(EstimateError::NonFiniteInput)
        );
    }

    #[test]
    fn rejects_invalid_k2_temporal_split() {
        assert_eq!(
            estimate_k2(153, 150.0, 150.0, 300, 1500, base()),
            Err(EstimateError::InvalidK2Split)
        );
    }
}
