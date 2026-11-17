// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Analytical two-resistance plant hydraulic null model.
//!
//! This is a deliberately small, steady-state reference baseline for separating
//! soil-side supply limitation from atmospheric demand. It is not a calibrated
//! plant model, does not represent capacitance or dynamic signalling, and is
//! not a reproduction of HydroRoot, Rockwell (2026), or Buckley et al. (2026).
//!
//! Root geometry and soil hydraulic state remain inputs owned by their
//! authoritative models. The two conductances below are equivalent boundary
//! parameters supplied by those models or by a synthetic benchmark fixture.
//!
//! Stomatal closure is a linear benchmark null hypothesis over leaf water
//! potential. It is intentionally simple so that more sophisticated models can
//! be compared against a transparent, analytically constrained reference.

const WATER_MOLAR_MASS_KG_PER_MOL: f64 = 0.018_015_28;
const BISECTION_ITERATIONS: usize = 96;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlantHydraulicInputs {
    /// Soil water potential at the root-zone boundary (MPa).
    pub soil_water_potential_mpa: f64,
    /// Equivalent soil-to-root conductance (kg m⁻² s⁻¹ MPa⁻¹).
    pub soil_to_root_conductance_kg_m2_s_mpa: f64,
    /// Equivalent root-to-leaf/xylem conductance (kg m⁻² s⁻¹ MPa⁻¹).
    pub root_to_leaf_conductance_kg_m2_s_mpa: f64,
    /// Maximum stomatal conductance to water vapour (mol m⁻² s⁻¹).
    pub maximum_stomatal_conductance_mol_m2_s: f64,
    /// Leaf-to-air vapour-pressure deficit (Pa).
    pub vapor_pressure_deficit_pa: f64,
    /// Ambient atmospheric pressure (Pa).
    pub atmospheric_pressure_pa: f64,
    /// Leaf water potential at which the stomatal null is fully open (MPa).
    pub stomatal_open_leaf_potential_mpa: f64,
    /// Leaf water potential at which the stomatal null is fully closed (MPa).
    /// Must be more negative than the open threshold.
    pub stomatal_closed_leaf_potential_mpa: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlantHydraulicError {
    NonFiniteInput(&'static str),
    NonPositiveConductance(&'static str),
    NegativeStomatalConductance,
    NegativeVaporPressureDeficit,
    NonPositiveAtmosphericPressure,
    InvalidStomatalThresholdOrder,
    NonFiniteDerivedValue(&'static str),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlantHydraulicState {
    /// Soil water potential at the root-zone boundary (MPa).
    pub soil_water_potential_mpa: f64,
    /// Water potential at the lumped root/xylem junction (MPa).
    pub root_water_potential_mpa: f64,
    /// Leaf water potential (MPa).
    pub leaf_water_potential_mpa: f64,
    /// Equivalent conductance of the two resistances in series
    /// (kg m⁻² s⁻¹ MPa⁻¹).
    pub equivalent_hydraulic_conductance_kg_m2_s_mpa: f64,
    /// Water flux across the soil-to-root boundary (kg m⁻² s⁻¹).
    pub soil_to_root_water_flux_kg_m2_s: f64,
    /// Water flux across the root-to-leaf boundary (kg m⁻² s⁻¹).
    pub root_to_leaf_water_flux_kg_m2_s: f64,
    /// Hydraulic transpiration flux (kg m⁻² s⁻¹).
    pub hydraulic_transpiration_kg_m2_s: f64,
    /// Stomatal demand implied by the solved conductance (kg m⁻² s⁻¹).
    pub stomatal_transpiration_kg_m2_s: f64,
    /// Solved stomatal conductance to water vapour (mol m⁻² s⁻¹).
    pub stomatal_conductance_mol_m2_s: f64,
    /// Fraction of maximum stomatal conductance in [0, 1].
    pub stomatal_open_fraction: f64,
    /// Pressure-potential drop across the soil-to-root resistance (MPa).
    pub soil_to_root_pressure_drop_mpa: f64,
    /// Pressure-potential drop across the root-to-leaf resistance (MPa).
    pub root_to_leaf_pressure_drop_mpa: f64,
    /// Difference between hydraulic supply and stomatal demand
    /// (kg m⁻² s⁻¹); should be close to zero within floating-point tolerance.
    pub water_balance_residual_kg_m2_s: f64,
}

/// Solve a steady-state, two-resistance hydraulic circuit with a linear
/// leaf-potential stomatal closure null.
///
/// Transpiration demand is
/// g_sw * (VPD / P_atm) * M_H2O, with boundary-layer resistance omitted.
/// Hydraulic supply and stomatal demand are solved together by deterministic
/// bisection. No empirical biological parameters are embedded: all conductance
/// and threshold values must be supplied by the caller.
///
/// This reference is useful for synthetic checks that distinguish reduced
/// soil-side conductance / water potential from increased atmospheric demand.
/// It must not be used as an uncalibrated biological forecast.
pub fn solve_two_resistance_null(
    inputs: PlantHydraulicInputs,
) -> Result<PlantHydraulicState, PlantHydraulicError> {
    validate_inputs(inputs)?;

    let soil_to_root = inputs.soil_to_root_conductance_kg_m2_s_mpa;
    let root_to_leaf = inputs.root_to_leaf_conductance_kg_m2_s_mpa;
    let smaller = soil_to_root.min(root_to_leaf);
    let larger = soil_to_root.max(root_to_leaf);
    // Algebraically equal to (K1 * K2) / (K1 + K2), while avoiding
    // multiplication of two potentially large conductance values.
    let equivalent_conductance = smaller / (1.0 + smaller / larger);

    let vapor_fraction = inputs.vapor_pressure_deficit_pa / inputs.atmospheric_pressure_pa;
    let maximum_demand = inputs.maximum_stomatal_conductance_mol_m2_s
        * vapor_fraction
        * WATER_MOLAR_MASS_KG_PER_MOL;

    if !equivalent_conductance.is_finite() || equivalent_conductance <= 0.0 {
        return Err(PlantHydraulicError::NonFiniteDerivedValue(
            "equivalent_hydraulic_conductance",
        ));
    }
    if !maximum_demand.is_finite() {
        return Err(PlantHydraulicError::NonFiniteDerivedValue(
            "maximum_stomatal_demand",
        ));
    }

    let demand_at_flux = |flux_kg_m2_s: f64| {
        let leaf_potential =
            inputs.soil_water_potential_mpa - flux_kg_m2_s / equivalent_conductance;
        let open_fraction = stomatal_open_fraction(
            leaf_potential,
            inputs.stomatal_open_leaf_potential_mpa,
            inputs.stomatal_closed_leaf_potential_mpa,
        );
        inputs.maximum_stomatal_conductance_mol_m2_s
            * open_fraction
            * vapor_fraction
            * WATER_MOLAR_MASS_KG_PER_MOL
    };

    // A substrate already below the closure threshold has no admissible
    // positive-flow solution under this benchmark null.
    let mut lower = 0.0;
    let mut upper = if stomatal_open_fraction(
        inputs.soil_water_potential_mpa,
        inputs.stomatal_open_leaf_potential_mpa,
        inputs.stomatal_closed_leaf_potential_mpa,
    ) == 0.0
    {
        0.0
    } else {
        maximum_demand
    };

    // At maximum open-stomata demand, hydraulic flux is at least as large as
    // demand because pressure feedback cannot increase stomatal conductance.
    for _ in 0..BISECTION_ITERATIONS {
        let midpoint = lower + (upper - lower) * 0.5;
        if midpoint <= lower || midpoint >= upper {
            break;
        }
        if midpoint > demand_at_flux(midpoint) {
            upper = midpoint;
        } else {
            lower = midpoint;
        }
    }
    let hydraulic_flux = lower + (upper - lower) * 0.5;

    let soil_to_root_drop = hydraulic_flux / soil_to_root;
    let root_to_leaf_drop = hydraulic_flux / root_to_leaf;
    let root_potential = inputs.soil_water_potential_mpa - soil_to_root_drop;
    let leaf_potential = root_potential - root_to_leaf_drop;
    let open_fraction = stomatal_open_fraction(
        leaf_potential,
        inputs.stomatal_open_leaf_potential_mpa,
        inputs.stomatal_closed_leaf_potential_mpa,
    );
    let stomatal_conductance =
        inputs.maximum_stomatal_conductance_mol_m2_s * open_fraction;
    let stomatal_transpiration =
        stomatal_conductance * vapor_fraction * WATER_MOLAR_MASS_KG_PER_MOL;
    let residual = hydraulic_flux - stomatal_transpiration;

    for (field, value) in [
        ("root_water_potential", root_potential),
        ("leaf_water_potential", leaf_potential),
        ("soil_to_root_pressure_drop", soil_to_root_drop),
        ("root_to_leaf_pressure_drop", root_to_leaf_drop),
        ("stomatal_transpiration", stomatal_transpiration),
        ("water_balance_residual", residual),
    ] {
        if !value.is_finite() {
            return Err(PlantHydraulicError::NonFiniteDerivedValue(field));
        }
    }

    Ok(PlantHydraulicState {
        soil_water_potential_mpa: inputs.soil_water_potential_mpa,
        root_water_potential_mpa: root_potential,
        leaf_water_potential_mpa: leaf_potential,
        equivalent_hydraulic_conductance_kg_m2_s_mpa: equivalent_conductance,
        soil_to_root_water_flux_kg_m2_s: hydraulic_flux,
        root_to_leaf_water_flux_kg_m2_s: hydraulic_flux,
        hydraulic_transpiration_kg_m2_s: hydraulic_flux,
        stomatal_transpiration_kg_m2_s: stomatal_transpiration,
        stomatal_conductance_mol_m2_s: stomatal_conductance,
        stomatal_open_fraction: open_fraction,
        soil_to_root_pressure_drop_mpa: soil_to_root_drop,
        root_to_leaf_pressure_drop_mpa: root_to_leaf_drop,
        water_balance_residual_kg_m2_s: residual,
    })
}

fn validate_inputs(inputs: PlantHydraulicInputs) -> Result<(), PlantHydraulicError> {
    for (field, value) in [
        ("soil_water_potential_mpa", inputs.soil_water_potential_mpa),
        (
            "soil_to_root_conductance_kg_m2_s_mpa",
            inputs.soil_to_root_conductance_kg_m2_s_mpa,
        ),
        (
            "root_to_leaf_conductance_kg_m2_s_mpa",
            inputs.root_to_leaf_conductance_kg_m2_s_mpa,
        ),
        (
            "maximum_stomatal_conductance_mol_m2_s",
            inputs.maximum_stomatal_conductance_mol_m2_s,
        ),
        ("vapor_pressure_deficit_pa", inputs.vapor_pressure_deficit_pa),
        ("atmospheric_pressure_pa", inputs.atmospheric_pressure_pa),
        (
            "stomatal_open_leaf_potential_mpa",
            inputs.stomatal_open_leaf_potential_mpa,
        ),
        (
            "stomatal_closed_leaf_potential_mpa",
            inputs.stomatal_closed_leaf_potential_mpa,
        ),
    ] {
        if !value.is_finite() {
            return Err(PlantHydraulicError::NonFiniteInput(field));
        }
    }

    if inputs.soil_to_root_conductance_kg_m2_s_mpa <= 0.0 {
        return Err(PlantHydraulicError::NonPositiveConductance(
            "soil_to_root_conductance",
        ));
    }
    if inputs.root_to_leaf_conductance_kg_m2_s_mpa <= 0.0 {
        return Err(PlantHydraulicError::NonPositiveConductance(
            "root_to_leaf_conductance",
        ));
    }
    if inputs.maximum_stomatal_conductance_mol_m2_s < 0.0 {
        return Err(PlantHydraulicError::NegativeStomatalConductance);
    }
    if inputs.vapor_pressure_deficit_pa < 0.0 {
        return Err(PlantHydraulicError::NegativeVaporPressureDeficit);
    }
    if inputs.atmospheric_pressure_pa <= 0.0 {
        return Err(PlantHydraulicError::NonPositiveAtmosphericPressure);
    }
    if inputs.stomatal_open_leaf_potential_mpa
        <= inputs.stomatal_closed_leaf_potential_mpa
    {
        return Err(PlantHydraulicError::InvalidStomatalThresholdOrder);
    }

    Ok(())
}

fn stomatal_open_fraction(
    leaf_potential_mpa: f64,
    open_threshold_mpa: f64,
    closed_threshold_mpa: f64,
) -> f64 {
    if leaf_potential_mpa >= open_threshold_mpa {
        1.0
    } else if leaf_potential_mpa <= closed_threshold_mpa {
        0.0
    } else {
        (leaf_potential_mpa - closed_threshold_mpa)
            / (open_threshold_mpa - closed_threshold_mpa)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reference_inputs() -> PlantHydraulicInputs {
        PlantHydraulicInputs {
            soil_water_potential_mpa: 0.0,
            soil_to_root_conductance_kg_m2_s_mpa: 0.01,
            root_to_leaf_conductance_kg_m2_s_mpa: 0.01,
            maximum_stomatal_conductance_mol_m2_s: 0.2,
            vapor_pressure_deficit_pa: 1_000.0,
            atmospheric_pressure_pa: 100_000.0,
            stomatal_open_leaf_potential_mpa: -0.5,
            stomatal_closed_leaf_potential_mpa: -2.0,
        }
    }

    #[test]
    fn well_watered_reference_remains_fully_open() {
        let state = solve_two_resistance_null(reference_inputs()).unwrap();

        assert_eq!(state.stomatal_open_fraction, 1.0);
        assert_eq!(state.stomatal_conductance_mol_m2_s, 0.2);
        assert!(state.leaf_water_potential_mpa > -0.5);
    }

    #[test]
    fn atmospheric_demand_pulse_increases_transpiration_when_unconstrained() {
        let baseline = solve_two_resistance_null(reference_inputs()).unwrap();
        let mut high_vpd = reference_inputs();
        high_vpd.vapor_pressure_deficit_pa = 2_000.0;
        let stressed = solve_two_resistance_null(high_vpd).unwrap();

        assert!(
            stressed.hydraulic_transpiration_kg_m2_s
                > baseline.hydraulic_transpiration_kg_m2_s
        );
        assert!(
            (stressed.hydraulic_transpiration_kg_m2_s
                / baseline.hydraulic_transpiration_kg_m2_s
                - 2.0)
                .abs()
                < 1e-10
        );
    }

    #[test]
    fn soil_supply_pulse_reduces_stomatal_conductance_and_transpiration() {
        let mut wet = reference_inputs();
        wet.soil_to_root_conductance_kg_m2_s_mpa = 0.00005;
        wet.root_to_leaf_conductance_kg_m2_s_mpa = 0.0002;
        wet.stomatal_open_leaf_potential_mpa = -0.3;
        wet.stomatal_closed_leaf_potential_mpa = -2.0;
        let wet_state = solve_two_resistance_null(wet).unwrap();

        let mut dry = wet;
        dry.soil_water_potential_mpa = -1.0;
        let dry_state = solve_two_resistance_null(dry).unwrap();

        assert!(dry_state.stomatal_conductance_mol_m2_s < wet_state.stomatal_conductance_mol_m2_s);
        assert!(
            dry_state.hydraulic_transpiration_kg_m2_s
                < wet_state.hydraulic_transpiration_kg_m2_s
        );
        assert!(dry_state.leaf_water_potential_mpa < wet_state.leaf_water_potential_mpa);
    }

    #[test]
    fn reduced_soil_to_root_conductance_reduces_transpiration() {
        let baseline = solve_two_resistance_null(reference_inputs()).unwrap();
        let mut restricted = reference_inputs();
        restricted.soil_to_root_conductance_kg_m2_s_mpa = 0.00005;
        restricted.root_to_leaf_conductance_kg_m2_s_mpa = 0.0002;
        let restricted_state = solve_two_resistance_null(restricted).unwrap();

        assert!(
            restricted_state.hydraulic_transpiration_kg_m2_s
                < baseline.hydraulic_transpiration_kg_m2_s
        );
        assert!(
            restricted_state.stomatal_conductance_mol_m2_s
                < baseline.stomatal_conductance_mol_m2_s
        );
    }

    #[test]
    fn hydraulic_fluxes_close_and_pressure_drops_sum() {
        let state = solve_two_resistance_null(reference_inputs()).unwrap();

        assert_eq!(state.soil_to_root_water_flux_kg_m2_s, state.root_to_leaf_water_flux_kg_m2_s);
        assert_eq!(state.hydraulic_transpiration_kg_m2_s, state.soil_to_root_water_flux_kg_m2_s);
        assert!(
            state.water_balance_residual_kg_m2_s.abs() < 1e-18,
            "residual={}",
            state.water_balance_residual_kg_m2_s
        );
        let total_drop =
            state.soil_to_root_pressure_drop_mpa + state.root_to_leaf_pressure_drop_mpa;
        assert!(
            (state.soil_water_potential_mpa - state.leaf_water_potential_mpa - total_drop)
                .abs()
                < 1e-12
        );
    }

    #[test]
    fn dominant_resistance_receives_larger_pressure_drop() {
        let mut inputs = reference_inputs();
        inputs.soil_to_root_conductance_kg_m2_s_mpa = 0.0001;
        inputs.root_to_leaf_conductance_kg_m2_s_mpa = 0.001;
        let state = solve_two_resistance_null(inputs).unwrap();

        assert!(state.soil_to_root_pressure_drop_mpa > state.root_to_leaf_pressure_drop_mpa);
    }

    #[test]
    fn soil_potential_below_closure_threshold_yields_zero_flux() {
        let mut inputs = reference_inputs();
        inputs.soil_water_potential_mpa = -3.0;
        let state = solve_two_resistance_null(inputs).unwrap();

        assert_eq!(state.hydraulic_transpiration_kg_m2_s, 0.0);
        assert_eq!(state.stomatal_conductance_mol_m2_s, 0.0);
        assert_eq!(state.leaf_water_potential_mpa, -3.0);
    }

    #[test]
    fn zero_vpd_yields_zero_flux() {
        let mut inputs = reference_inputs();
        inputs.vapor_pressure_deficit_pa = 0.0;
        let state = solve_two_resistance_null(inputs).unwrap();

        assert_eq!(state.hydraulic_transpiration_kg_m2_s, 0.0);
        assert_eq!(state.stomatal_transpiration_kg_m2_s, 0.0);
    }

    #[test]
    fn invalid_conductance_is_rejected_without_clamping() {
        let mut inputs = reference_inputs();
        inputs.soil_to_root_conductance_kg_m2_s_mpa = 0.0;

        assert_eq!(
            solve_two_resistance_null(inputs),
            Err(PlantHydraulicError::NonPositiveConductance(
                "soil_to_root_conductance"
            ))
        );
    }

    #[test]
    fn invalid_stomatal_threshold_order_is_rejected() {
        let mut inputs = reference_inputs();
        inputs.stomatal_open_leaf_potential_mpa = -2.0;
        inputs.stomatal_closed_leaf_potential_mpa = -0.5;

        assert_eq!(
            solve_two_resistance_null(inputs),
            Err(PlantHydraulicError::InvalidStomatalThresholdOrder)
        );
    }

    #[test]
    fn non_finite_input_is_rejected() {
        let mut inputs = reference_inputs();
        inputs.vapor_pressure_deficit_pa = f64::NAN;

        assert_eq!(
            solve_two_resistance_null(inputs),
            Err(PlantHydraulicError::NonFiniteInput(
                "vapor_pressure_deficit_pa"
            ))
        );
    }
}
