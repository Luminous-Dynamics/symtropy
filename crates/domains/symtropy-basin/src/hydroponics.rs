// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Deterministic bookkeeping primitives for passive and hybrid hydroponics.
//!
//! This module is intentionally an accounting kernel, not a crop-growth model.
//! It enforces water and dissolved-nutrient mass balance for each daily step.
//! Crop-specific uptake, oxygen availability, thermal stress, plant growth, and
//! pathogen dynamics must be supplied by validated models or measurements.

/// Reservoir contents at the start or end of an accounting interval.
/// Water is measured in litres; aggregate dissolved nutrient mass is grams.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReservoirState {
    pub water_l: f64,
    pub dissolved_nutrients_g: f64,
}

/// External inflows and outflows over one accounting interval.
/// Each value is an interval total, not a rate. Rain adds water, not nutrients.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct DailyFlux {
    pub rainwater_input_l: f64,
    pub supplemental_water_input_l: f64,
    pub crop_water_uptake_l: f64,
    pub evaporation_l: f64,
    pub leakage_or_overflow_l: f64,
    pub nutrient_input_g: f64,
    pub crop_nutrient_uptake_g: f64,
    pub nutrient_discharge_g: f64,
}

/// Result of one valid accounting interval.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DailyBalance {
    pub after: ReservoirState,
    pub water_balance_residual_l: f64,
    pub nutrient_balance_residual_g: f64,
    pub nutrient_concentration_g_per_l: Option<f64>,
}

/// Invalid or physically impossible accounting input.
/// The model fails closed instead of silently clamping negative inventory.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BalanceError {
    NonFinite { field: &'static str },
    Negative { field: &'static str },
    WaterDemandExceedsAvailable { available_l: f64, demanded_l: f64 },
    NutrientDemandExceedsAvailable { available_g: f64, demanded_g: f64 },
}

fn validate_nonnegative(field: &'static str, value: f64) -> Result<(), BalanceError> {
    if !value.is_finite() {
        return Err(BalanceError::NonFinite { field });
    }
    if value < 0.0 {
        return Err(BalanceError::Negative { field });
    }
    Ok(())
}

impl ReservoirState {
    fn validate(self) -> Result<(), BalanceError> {
        validate_nonnegative("water_l", self.water_l)?;
        validate_nonnegative("dissolved_nutrients_g", self.dissolved_nutrients_g)
    }
}

impl DailyFlux {
    fn validate(self) -> Result<(), BalanceError> {
        for (field, value) in [
            ("rainwater_input_l", self.rainwater_input_l),
            (
                "supplemental_water_input_l",
                self.supplemental_water_input_l,
            ),
            ("crop_water_uptake_l", self.crop_water_uptake_l),
            ("evaporation_l", self.evaporation_l),
            ("leakage_or_overflow_l", self.leakage_or_overflow_l),
            ("nutrient_input_g", self.nutrient_input_g),
            ("crop_nutrient_uptake_g", self.crop_nutrient_uptake_g),
            ("nutrient_discharge_g", self.nutrient_discharge_g),
        ] {
            validate_nonnegative(field, value)?;
        }
        Ok(())
    }
}

/// Apply one interval of externally supplied fluxes to a reservoir.
///
/// This kernel does not infer fluxes from weather or plant traits. It accounts
/// for supplied fluxes and returns an error if inventory would be overdrawn.
pub fn step(before: ReservoirState, flux: DailyFlux) -> Result<DailyBalance, BalanceError> {
    before.validate()?;
    flux.validate()?;

    let water_in = flux.rainwater_input_l + flux.supplemental_water_input_l;
    if !water_in.is_finite() {
        return Err(BalanceError::NonFinite {
            field: "water_in_l",
        });
    }
    let water_out = flux.crop_water_uptake_l + flux.evaporation_l + flux.leakage_or_overflow_l;
    if !water_out.is_finite() {
        return Err(BalanceError::NonFinite {
            field: "water_out_l",
        });
    }
    let water_available = before.water_l + water_in;
    if !water_available.is_finite() {
        return Err(BalanceError::NonFinite {
            field: "water_available_l",
        });
    }
    if water_out > water_available {
        return Err(BalanceError::WaterDemandExceedsAvailable {
            available_l: water_available,
            demanded_l: water_out,
        });
    }

    let nutrients_available = before.dissolved_nutrients_g + flux.nutrient_input_g;
    if !nutrients_available.is_finite() {
        return Err(BalanceError::NonFinite {
            field: "nutrients_available_g",
        });
    }
    let nutrients_out = flux.crop_nutrient_uptake_g + flux.nutrient_discharge_g;
    if !nutrients_out.is_finite() {
        return Err(BalanceError::NonFinite {
            field: "nutrients_out_g",
        });
    }
    if nutrients_out > nutrients_available {
        return Err(BalanceError::NutrientDemandExceedsAvailable {
            available_g: nutrients_available,
            demanded_g: nutrients_out,
        });
    }

    let after = ReservoirState {
        water_l: water_available - water_out,
        dissolved_nutrients_g: nutrients_available - nutrients_out,
    };
    let water_balance_residual_l = after.water_l - (before.water_l + water_in - water_out);
    let nutrient_balance_residual_g = after.dissolved_nutrients_g
        - (before.dissolved_nutrients_g + flux.nutrient_input_g - nutrients_out);
    let nutrient_concentration_g_per_l = if after.water_l > 0.0 {
        let concentration = after.dissolved_nutrients_g / after.water_l;
        if !concentration.is_finite() {
            return Err(BalanceError::NonFinite {
                field: "nutrient_concentration_g_per_l",
            });
        }
        Some(concentration)
    } else {
        None
    };

    Ok(DailyBalance {
        after,
        water_balance_residual_l,
        nutrient_balance_residual_g,
        nutrient_concentration_g_per_l,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f64 = 1e-10;

    #[test]
    fn water_and_nutrient_mass_balance_close() {
        let before = ReservoirState {
            water_l: 100.0,
            dissolved_nutrients_g: 10.0,
        };
        let flux = DailyFlux {
            rainwater_input_l: 2.0,
            crop_water_uptake_l: 7.0,
            evaporation_l: 1.0,
            leakage_or_overflow_l: 0.5,
            nutrient_input_g: 0.5,
            crop_nutrient_uptake_g: 1.25,
            nutrient_discharge_g: 0.25,
            ..DailyFlux::default()
        };
        let result = step(before, flux).expect("valid balance");
        assert!((result.after.water_l - 93.5).abs() < EPSILON);
        assert!((result.after.dissolved_nutrients_g - 9.0).abs() < EPSILON);
        assert!(result.water_balance_residual_l.abs() < EPSILON);
        assert!(result.nutrient_balance_residual_g.abs() < EPSILON);
    }

    #[test]
    fn rain_dilutes_solution_without_inventing_nutrients() {
        let before = ReservoirState {
            water_l: 50.0,
            dissolved_nutrients_g: 5.0,
        };
        let result = step(
            before,
            DailyFlux {
                rainwater_input_l: 5.0,
                ..DailyFlux::default()
            },
        )
        .expect("rain-only balance");
        assert_eq!(result.after.water_l, 55.0);
        assert_eq!(result.after.dissolved_nutrients_g, 5.0);
        assert!((result.nutrient_concentration_g_per_l.unwrap() - (5.0 / 55.0)).abs() < EPSILON);
    }

    #[test]
    fn rejects_water_overdraw_instead_of_clamping() {
        let error = step(
            ReservoirState {
                water_l: 1.0,
                dissolved_nutrients_g: 1.0,
            },
            DailyFlux {
                crop_water_uptake_l: 1.1,
                ..DailyFlux::default()
            },
        )
        .unwrap_err();
        assert!(matches!(
            error,
            BalanceError::WaterDemandExceedsAvailable { .. }
        ));
    }

    #[test]
    fn rejects_nutrient_overdraw_instead_of_clamping() {
        let error = step(
            ReservoirState {
                water_l: 1.0,
                dissolved_nutrients_g: 0.2,
            },
            DailyFlux {
                crop_nutrient_uptake_g: 0.3,
                ..DailyFlux::default()
            },
        )
        .unwrap_err();
        assert!(matches!(
            error,
            BalanceError::NutrientDemandExceedsAvailable { .. }
        ));
    }

    #[test]
    fn rejects_non_finite_inputs() {
        let error = step(
            ReservoirState {
                water_l: f64::NAN,
                dissolved_nutrients_g: 1.0,
            },
            DailyFlux::default(),
        )
        .unwrap_err();
        assert_eq!(error, BalanceError::NonFinite { field: "water_l" });
    }

    #[test]
    fn rejects_overflowing_flux_totals() {
        let error = step(
            ReservoirState {
                water_l: 0.0,
                dissolved_nutrients_g: 0.0,
            },
            DailyFlux {
                rainwater_input_l: f64::MAX,
                supplemental_water_input_l: f64::MAX,
                ..DailyFlux::default()
            },
        )
        .unwrap_err();
        assert_eq!(
            error,
            BalanceError::NonFinite {
                field: "water_in_l"
            }
        );
    }

    #[test]
    fn rejects_unrepresentable_concentration() {
        let error = step(
            ReservoirState {
                water_l: f64::MIN_POSITIVE,
                dissolved_nutrients_g: f64::MAX,
            },
            DailyFlux::default(),
        )
        .unwrap_err();
        assert_eq!(
            error,
            BalanceError::NonFinite {
                field: "nutrient_concentration_g_per_l"
            }
        );
    }

    #[test]
    fn rejects_negative_flux() {
        let error = step(
            ReservoirState {
                water_l: 1.0,
                dissolved_nutrients_g: 1.0,
            },
            DailyFlux {
                evaporation_l: -0.1,
                ..DailyFlux::default()
            },
        )
        .unwrap_err();
        assert_eq!(
            error,
            BalanceError::Negative {
                field: "evaporation_l"
            }
        );
    }

    #[test]
    fn zero_water_has_no_defined_solution_concentration() {
        let result = step(
            ReservoirState {
                water_l: 1.0,
                dissolved_nutrients_g: 0.0,
            },
            DailyFlux {
                crop_water_uptake_l: 1.0,
                ..DailyFlux::default()
            },
        )
        .expect("fully consumed water");
        assert_eq!(result.nutrient_concentration_g_per_l, None);
        assert_eq!(result.after.water_l, 0.0);
    }
}
