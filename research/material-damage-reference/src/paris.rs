//! Constant-Delta-K Paris-law crack-growth reference.
//!
//! The model evaluates da/dN = C * (Delta-K)^m for one declared, constant
//! stress-intensity-factor range. It does not calculate geometry factors or
//! infer a Delta-K history from loads and crack geometry.

use crate::{DamageError, DamageResult, require_positive_finite};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParisLaw {
    /// Paris coefficient in (m/cycle) / (MPa*sqrt(m))^exponent.
    pub coefficient_m_per_cycle_per_delta_k_power: f64,
    /// Paris exponent m.
    pub exponent: f64,
    /// Lower bound of the caller-declared valid Paris regime, MPa*sqrt(m).
    pub minimum_delta_k_mpa_sqrt_m: f64,
    /// Upper bound of the caller-declared valid Paris regime, MPa*sqrt(m).
    pub maximum_delta_k_mpa_sqrt_m: f64,
}

impl ParisLaw {
    pub fn new(
        coefficient_m_per_cycle_per_delta_k_power: f64,
        exponent: f64,
        minimum_delta_k_mpa_sqrt_m: f64,
        maximum_delta_k_mpa_sqrt_m: f64,
    ) -> DamageResult<Self> {
        require_positive_finite(
            coefficient_m_per_cycle_per_delta_k_power,
            "Paris coefficient",
        )?;
        require_positive_finite(exponent, "Paris exponent")?;
        require_positive_finite(minimum_delta_k_mpa_sqrt_m, "minimum Paris Delta-K")?;
        require_positive_finite(maximum_delta_k_mpa_sqrt_m, "maximum Paris Delta-K")?;
        if maximum_delta_k_mpa_sqrt_m <= minimum_delta_k_mpa_sqrt_m {
            return Err(DamageError::InvalidParisRange);
        }

        Ok(Self {
            coefficient_m_per_cycle_per_delta_k_power,
            exponent,
            minimum_delta_k_mpa_sqrt_m,
            maximum_delta_k_mpa_sqrt_m,
        })
    }

    /// Evaluate growth for a constant Delta-K and a declared cycle count.
    ///
    /// Crack length is in metres; Delta-K is in MPa*sqrt(m). The caller must
    /// supply a Delta-K that already accounts for its specimen/structure
    /// geometry and load ratio. The accepted range is caller-declared and is
    /// not itself proof that a material's Paris regime was experimentally
    /// established.
    pub fn constant_delta_k_growth(
        &self,
        initial_crack_length_m: f64,
        delta_k_mpa_sqrt_m: f64,
        cycles: u64,
    ) -> DamageResult<ParisGrowthResult> {
        require_positive_finite(initial_crack_length_m, "initial crack length")?;
        require_positive_finite(delta_k_mpa_sqrt_m, "Delta-K")?;
        if cycles == 0 {
            return Err(DamageError::ZeroCycleCount);
        }

        if delta_k_mpa_sqrt_m < self.minimum_delta_k_mpa_sqrt_m
            || delta_k_mpa_sqrt_m > self.maximum_delta_k_mpa_sqrt_m
        {
            return Err(DamageError::DeltaKOutsideParisRange {
                delta_k_mpa_sqrt_m,
                minimum_mpa_sqrt_m: self.minimum_delta_k_mpa_sqrt_m,
                maximum_mpa_sqrt_m: self.maximum_delta_k_mpa_sqrt_m,
            });
        }

        let delta_k_power = delta_k_mpa_sqrt_m.powf(self.exponent);
        let growth_rate_m_per_cycle =
            self.coefficient_m_per_cycle_per_delta_k_power * delta_k_power;
        let crack_extension_m = growth_rate_m_per_cycle * cycles as f64;
        let final_crack_length_m = initial_crack_length_m + crack_extension_m;

        if !delta_k_power.is_finite()
            || !growth_rate_m_per_cycle.is_finite()
            || !crack_extension_m.is_finite()
            || !final_crack_length_m.is_finite()
        {
            return Err(DamageError::ArithmeticOverflow {
                field: "Paris crack-growth result",
            });
        }

        Ok(ParisGrowthResult {
            initial_crack_length_m,
            final_crack_length_m,
            crack_extension_m,
            growth_rate_m_per_cycle,
            delta_k_mpa_sqrt_m,
            cycles,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParisGrowthResult {
    pub initial_crack_length_m: f64,
    pub final_crack_length_m: f64,
    pub crack_extension_m: f64,
    pub growth_rate_m_per_cycle: f64,
    pub delta_k_mpa_sqrt_m: f64,
    pub cycles: u64,
}
