// SPDX-License-Identifier: AGPL-3.0-or-later
//! Bounded analytical reference models for cumulative material damage.
//!
//! These functions are verification targets, not a general fatigue or fracture
//! solver. Inputs are already cycle-counted or supplied as a constant stress
//! intensity range. Material calibration, load-history reconstruction, FE
//! stress extraction, and engineering release decisions are out of scope.

pub mod fatigue;
pub mod paris;

#[derive(Clone, Debug, PartialEq)]
pub enum DamageError {
    InsufficientSnPoints { count: usize },
    InvalidValue { field: &'static str },
    NonFinite { field: &'static str },
    InvalidSnOrdering,
    StressOutsideSnRange {
        stress_amplitude_mpa: f64,
        minimum_mpa: f64,
        maximum_mpa: f64,
    },
    EmptyLoadSpectrum,
    ZeroCycleCount,
    InvalidParisRange,
    DeltaKOutsideParisRange {
        delta_k_mpa_sqrt_m: f64,
        minimum_mpa_sqrt_m: f64,
        maximum_mpa_sqrt_m: f64,
    },
    ArithmeticOverflow { field: &'static str },
}

impl std::fmt::Display for DamageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InsufficientSnPoints { count } => {
                write!(f, "S-N curve requires at least two points; got {count}")
            }
            Self::InvalidValue { field } => write!(f, "{field} must be positive and valid"),
            Self::NonFinite { field } => write!(f, "{field} must be finite"),
            Self::InvalidSnOrdering => write!(
                f,
                "S-N points must have strictly increasing life and strictly decreasing stress"
            ),
            Self::StressOutsideSnRange {
                stress_amplitude_mpa,
                minimum_mpa,
                maximum_mpa,
            } => write!(
                f,
                "stress amplitude {stress_amplitude_mpa} MPa is outside the calibrated S-N range [{minimum_mpa}, {maximum_mpa}] MPa"
            ),
            Self::EmptyLoadSpectrum => write!(f, "load spectrum must contain at least one block"),
            Self::ZeroCycleCount => write!(f, "cycle count must be greater than zero"),
            Self::InvalidParisRange => write!(
                f,
                "Paris-regime Delta-K bounds must be positive, finite, and strictly ordered"
            ),
            Self::DeltaKOutsideParisRange {
                delta_k_mpa_sqrt_m,
                minimum_mpa_sqrt_m,
                maximum_mpa_sqrt_m,
            } => write!(
                f,
                "Delta-K {delta_k_mpa_sqrt_m} MPa*sqrt(m) is outside the declared Paris regime [{minimum_mpa_sqrt_m}, {maximum_mpa_sqrt_m}]"
            ),
            Self::ArithmeticOverflow { field } => {
                write!(f, "{field} overflowed or produced a non-finite result")
            }
        }
    }
}

impl std::error::Error for DamageError {}

pub type DamageResult<T> = Result<T, DamageError>;

pub(crate) fn require_positive_finite(value: f64, field: &'static str) -> DamageResult<()> {
    if !value.is_finite() {
        return Err(DamageError::NonFinite { field });
    }
    if value <= 0.0 {
        return Err(DamageError::InvalidValue { field });
    }
    Ok(())
}
