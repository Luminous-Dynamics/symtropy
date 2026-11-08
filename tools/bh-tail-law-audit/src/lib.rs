//! Typed reference selector for the published Kerr sourced-tail asymptotics.
//!
//! This crate transcribes only the spin-weighted (s != 0) regime statement in
//! Ling & Wong, Phys. Rev. D 114, 064092 (2026), DOI 10.1103/b44t-dlb7.
//! It is not a Teukolsky solver and does not compute tail amplitudes.
//!
//! For a mode (ell,m), spin weight s != 0, and source falloff r^(-beta), the
//! paper reports:
//!
//! - sourced tail t^(-(ell + beta + s)) for beta <= 1 or
//!   beta >= ell - s + 2;
//! - Price tail t^(-(2 ell + 3)) for 2 <= beta <= ell - s + 1.
//!
//! The open beta intervals not covered by those statements are rejected.
//! In particular, for 1 < beta < 2 the selector returns UnsupportedBetaRegime.
//! The scalar s=0 numerical demonstration in the paper is not a license to
//! substitute s=0 into this spin-weighted formula.

use core::fmt;

/// A spin-weighted angular mode. Construction validates |s| <= ell and
/// |m| <= ell. The asymptotic formula below additionally requires s != 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpinWeightedMode {
    ell: u32,
    m: i32,
    spin_weight: i32,
}

impl SpinWeightedMode {
    pub fn new(ell: u32, m: i32, spin_weight: i32) -> Result<Self, TailLawError> {
        let invalid_mode = i32::try_from(ell).is_err()
            || m.unsigned_abs() > ell
            || spin_weight.unsigned_abs() > ell;
        if invalid_mode {
            return Err(TailLawError::InvalidMode);
        }
        if spin_weight == 0 {
            return Err(TailLawError::ScalarFormulaNotSpecified);
        }
        Ok(Self {
            ell,
            m,
            spin_weight,
        })
    }

    pub const fn ell(self) -> u32 {
        self.ell
    }

    pub const fn m(self) -> i32 {
        self.m
    }

    pub const fn spin_weight(self) -> i32 {
        self.spin_weight
    }
}

/// Which published asymptotic branch applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TailBranch {
    /// Source-generated tail with beta <= 1.
    SourcedLowBeta,
    /// Price tail for 2 <= beta <= ell - s + 1.
    Price,
    /// Source-generated tail with beta >= ell - s + 2.
    SourcedHighBeta,
}

/// A power-law prediction of the form amplitude times t^(-power).
///
/// Only power is returned here. The amplitude depends on source and
/// propagation details and is deliberately not inferred by this selector.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TailLaw {
    pub branch: TailBranch,
    pub power: f64,
    pub beta: f64,
}

/// Fail-closed errors for unsupported or malformed inputs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TailLawError {
    InvalidMode,
    ScalarFormulaNotSpecified,
    NonFiniteBeta,
    UnsupportedBetaRegime {
        beta: f64,
        price_upper: f64,
        sourced_high_lower: f64,
    },
    NonFinitePower,
}

impl fmt::Display for TailLawError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMode => write!(
                f,
                "invalid spin-weighted mode: require |m| <= ell and |s| <= ell"
            ),
            Self::ScalarFormulaNotSpecified => {
                write!(
                    f,
                    "the cited spin-weighted sourced-tail formula is not specified for s = 0"
                )
            }
            Self::NonFiniteBeta => write!(f, "source falloff beta must be finite"),
            Self::UnsupportedBetaRegime {
                beta,
                price_upper,
                sourced_high_lower,
            } => write!(
                f,
                "beta={beta} is outside the published branches; Price branch ends at {price_upper}, sourced high-beta branch starts at {sourced_high_lower}"
            ),
            Self::NonFinitePower => write!(f, "computed tail power is not finite"),
        }
    }
}

impl std::error::Error for TailLawError {}

/// Select the published spin-weighted sourced-tail / Price-tail branch.
///
/// This function reproduces an asymptotic exponent only. It does not validate
/// the far-field approximation, perturbative source, mode excitation,
/// amplitude, extraction radius, or detector observability. Those require
/// independent physical and numerical experiments.
pub fn classify_tail_law(mode: SpinWeightedMode, beta: f64) -> Result<TailLaw, TailLawError> {
    if !beta.is_finite() {
        return Err(TailLawError::NonFiniteBeta);
    }

    let ell = mode.ell as f64;
    let spin = mode.spin_weight as f64;
    let price_upper = ell - spin + 1.0;
    let sourced_high_lower = ell - spin + 2.0;

    let (branch, power) = if beta <= 1.0 {
        (TailBranch::SourcedLowBeta, ell + beta + spin)
    } else if beta >= sourced_high_lower {
        (TailBranch::SourcedHighBeta, ell + beta + spin)
    } else if beta >= 2.0 && beta <= price_upper {
        (TailBranch::Price, 2.0 * ell + 3.0)
    } else {
        return Err(TailLawError::UnsupportedBetaRegime {
            beta,
            price_upper,
            sourced_high_lower,
        });
    };

    if !power.is_finite() {
        return Err(TailLawError::NonFinitePower);
    }

    Ok(TailLaw {
        branch,
        power,
        beta,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mode(ell: u32, m: i32, s: i32) -> SpinWeightedMode {
        SpinWeightedMode::new(ell, m, s).unwrap()
    }

    #[test]
    fn low_beta_uses_sourced_tail_exponent() {
        let law = classify_tail_law(mode(2, 2, -2), 1.0).unwrap();
        assert_eq!(law.branch, TailBranch::SourcedLowBeta);
        assert!((law.power - 1.0).abs() < 1.0e-14);
    }

    #[test]
    fn middle_beta_uses_price_tail_exponent() {
        let law = classify_tail_law(mode(2, 2, -2), 3.0).unwrap();
        assert_eq!(law.branch, TailBranch::Price);
        assert!((law.power - 7.0).abs() < 1.0e-14);
    }

    #[test]
    fn upper_beta_uses_sourced_tail_exponent() {
        let law = classify_tail_law(mode(2, 2, -2), 6.0).unwrap();
        assert_eq!(law.branch, TailBranch::SourcedHighBeta);
        assert!((law.power - 6.0).abs() < 1.0e-14);
    }

    #[test]
    fn both_uncovered_beta_intervals_fail_closed() {
        let m = mode(2, 2, -2);
        assert!(matches!(
            classify_tail_law(m, 1.5),
            Err(TailLawError::UnsupportedBetaRegime { .. })
        ));
        assert!(matches!(
            classify_tail_law(m, 5.5),
            Err(TailLawError::UnsupportedBetaRegime { .. })
        ));
    }

    #[test]
    fn exact_beta_boundaries_follow_the_published_inclusive_ranges() {
        let m = mode(2, 2, -2);
        assert_eq!(
            classify_tail_law(m, 1.0).unwrap().branch,
            TailBranch::SourcedLowBeta
        );
        assert_eq!(classify_tail_law(m, 2.0).unwrap().branch, TailBranch::Price);
        assert_eq!(classify_tail_law(m, 5.0).unwrap().branch, TailBranch::Price);
        assert_eq!(
            classify_tail_law(m, 6.0).unwrap().branch,
            TailBranch::SourcedHighBeta
        );
    }

    #[test]
    fn empty_price_interval_is_not_fabricated_for_positive_spin() {
        let m = mode(2, 2, 2);
        assert!(matches!(
            classify_tail_law(m, 1.5),
            Err(TailLawError::UnsupportedBetaRegime { .. })
        ));
        assert_eq!(
            classify_tail_law(m, 2.0).unwrap().branch,
            TailBranch::SourcedHighBeta
        );
    }

    #[test]
    fn scalar_formula_is_not_silently_extrapolated() {
        assert_eq!(
            SpinWeightedMode::new(0, 0, 0),
            Err(TailLawError::ScalarFormulaNotSpecified)
        );
    }

    #[test]
    fn invalid_angular_modes_are_rejected() {
        assert_eq!(
            SpinWeightedMode::new(1, 2, 1),
            Err(TailLawError::InvalidMode)
        );
        assert_eq!(
            SpinWeightedMode::new(1, 0, 2),
            Err(TailLawError::InvalidMode)
        );
    }

    #[test]
    fn non_finite_beta_is_rejected() {
        let m = mode(2, 2, -2);
        assert_eq!(
            classify_tail_law(m, f64::NAN),
            Err(TailLawError::NonFiniteBeta)
        );
        assert_eq!(
            classify_tail_law(m, f64::INFINITY),
            Err(TailLawError::NonFiniteBeta)
        );
    }
}
