// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Effective-free-path distributions for geometry-limited bacterial runs.
//!
//! This is a coarse-grained length model. It does not represent pore topology,
//! explicit cell-wall collisions, hydrodynamic forces or contact mechanics.

use std::error::Error;
use std::fmt;

use super::{SampleSet, SeededRng};

/// A positive effective free-path length distribution, measured in micrometres.
#[derive(Debug, Clone, PartialEq)]
pub enum FreePathLaw {
    /// Non-parametric resampling of observed effective free-path lengths.
    Empirical(SampleSet),
    /// Exponential free-path distribution parameterized by its mean length.
    Exponential { mean_um: f64 },
}

impl FreePathLaw {
    pub fn empirical(samples_um: Vec<f64>) -> Self {
        Self::Empirical(SampleSet::new(samples_um))
    }

    pub fn validate(&self) -> Result<(), FreePathLawError> {
        match self {
            Self::Empirical(samples) => {
                if samples.is_empty() {
                    return Err(FreePathLawError::EmptyEmpiricalSamples);
                }
                if samples
                    .values()
                    .iter()
                    .any(|value| !value.is_finite() || *value <= 0.0)
                {
                    return Err(FreePathLawError::InvalidEmpiricalSample);
                }
            }
            Self::Exponential { mean_um } => {
                if !mean_um.is_finite() || *mean_um <= 0.0 {
                    return Err(FreePathLawError::InvalidMeanFreePath);
                }
            }
        }
        Ok(())
    }

    pub fn mean_um(&self) -> Option<f64> {
        self.validate().ok()?;
        match self {
            Self::Empirical(samples) => {
                let mean = samples.values().iter().sum::<f64>() / samples.len() as f64;
                mean.is_finite().then_some(mean)
            }
            Self::Exponential { mean_um } => Some(*mean_um),
        }
    }

    pub fn sample_with_seed(&self, seed: u64) -> Result<f64, FreePathLawError> {
        self.validate()?;
        let mut rng = SeededRng::new(seed);
        Ok(self.sample_with_rng(&mut rng))
    }

    pub(crate) fn sample_with_rng(&self, rng: &mut SeededRng) -> f64 {
        match self {
            Self::Empirical(samples) => samples.sample(rng),
            Self::Exponential { mean_um } => {
                let unit = rng.next_unit_f64().clamp(f64::EPSILON, 1.0 - f64::EPSILON);
                -mean_um * (1.0 - unit).ln()
            }
        }
    }
}

/// Errors from an invalid effective-free-path distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FreePathLawError {
    EmptyEmpiricalSamples,
    InvalidEmpiricalSample,
    InvalidMeanFreePath,
}

impl fmt::Display for FreePathLawError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyEmpiricalSamples => {
                write!(formatter, "empirical free-path law needs samples")
            }
            Self::InvalidEmpiricalSample => {
                write!(formatter, "empirical free-path lengths must be finite and positive")
            }
            Self::InvalidMeanFreePath => {
                write!(formatter, "mean free path must be finite and positive")
            }
        }
    }
}

impl Error for FreePathLawError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empirical_free_path_preserves_mean_and_seeded_sampling() {
        let law = FreePathLaw::empirical(vec![4.0, 6.0, 8.0]);
        law.validate().expect("valid empirical path law");
        assert_eq!(law.mean_um(), Some(6.0));
        assert_eq!(law.sample_with_seed(7), law.sample_with_seed(7));
    }

    #[test]
    fn exponential_free_path_has_declared_mean() {
        let law = FreePathLaw::Exponential { mean_um: 8.0 };
        law.validate().expect("valid exponential path law");
        assert_eq!(law.mean_um(), Some(8.0));
        assert!(law.sample_with_seed(41).unwrap() > 0.0);
    }

    #[test]
    fn invalid_free_path_laws_fail_closed() {
        assert_eq!(
            FreePathLaw::empirical(vec![]).validate(),
            Err(FreePathLawError::EmptyEmpiricalSamples)
        );
        assert_eq!(
            FreePathLaw::empirical(vec![1.0, f64::NAN]).validate(),
            Err(FreePathLawError::InvalidEmpiricalSample)
        );
        assert_eq!(
            FreePathLaw::Exponential { mean_um: 0.0 }.validate(),
            Err(FreePathLawError::InvalidMeanFreePath)
        );
    }
}
