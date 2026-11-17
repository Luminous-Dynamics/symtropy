// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Clean-room equation-level reference for Buckley et al. (2026), Eq. 3a/3b.
//!
//! Reference:
//! Buckley et al. (2026), "A Simple Stomatal Model That Unifies the Metabolic
//! and Hydraulic Control of Carbon and Water Flux", Global Change Biology.
//! DOI: 10.1111/gcb.70999.
//!
//! This is an independently written implementation of the published equations,
//! not a port of the authors' R code and not a reproduction of the paper's
//! datasets or figures. It is a comparator module; source-faithful data
//! evaluation, parameter fitting and cross-species validation remain separate.
//!
//! The source model is:
//!   b = A_m - A + b_o
//!   g_sw = s*b*(psi_s - psi_c) / (1 + s*b*delta_w/K)
//!
//! Units are explicit in the field names and follow the paper's Table 1:
//! K: mmol m^-2 s^-1 MPa^-1
//! delta_w: mmol mol^-1
//! s: mol umol^-1 MPa^-1
//! A_m, A, b_o and b: umol m^-2 s^-1
//! g_sw: mol m^-2 s^-1
//! E: mmol m^-2 s^-1

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuckleyStomatalError {
    NonFiniteInput(&'static str),
    NonPositiveHydraulicConductance,
    NonPositiveSensitivity,
    NegativeVaporMoleFractionDifference,
    NegativeBasalBioenergeticCapacity,
    InvalidAtmosphericPressure,
    NegativeVaporPressureDeficit,
    NegativeElectronTransportRate,
    NonPositiveBioenergeticCapacity,
    NonFiniteDerivedValue(&'static str),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BuckleyStomatalInputs {
    /// Soil water potential (MPa).
    pub soil_water_potential_mpa: f64,
    /// Stomatal closure point (MPa), generally negative.
    pub stomatal_closure_potential_mpa: f64,
    /// Soil-to-leaf hydraulic conductance (mmol m^-2 s^-1 MPa^-1).
    pub soil_leaf_hydraulic_conductance_mmol_m2_s_mpa: f64,
    /// Leaf-to-air water-vapor mole-fraction difference (mmol mol^-1).
    /// This is not VPD in kPa; use delta_w_from_vpd_pa to convert a
    /// measured leaf-to-air vapor-pressure difference.
    pub delta_w_mmol_per_mol: f64,
    /// Stomatal scaling and sensitivity coefficient s
    /// (mol umol^-1 MPa^-1); one of the two coefficients requiring fitting.
    pub sensitivity_mol_per_umol_mpa: f64,
    /// CO2-saturated assimilation rate A_m (umol m^-2 s^-1).
    pub co2_saturated_assimilation_umol_m2_s: f64,
    /// Net CO2 assimilation rate A (umol m^-2 s^-1).
    pub net_assimilation_umol_m2_s: f64,
    /// Basal bioenergetic capacity b_o (umol m^-2 s^-1);
    /// the other coefficient requiring fitting.
    pub basal_bioenergetic_capacity_umol_m2_s: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BuckleyStomatalState {
    /// Residual photosynthetic capacity plus basal opening capacity
    /// b = A_m - A + b_o (umol m^-2 s^-1).
    pub bioenergetic_capacity_umol_m2_s: f64,
    /// Stomatal conductance to water vapor g_sw (mol m^-2 s^-1).
    pub stomatal_conductance_mol_m2_s: f64,
    /// Steady-state transpiration implied by g_sw * delta_w
    /// (mmol m^-2 s^-1).
    pub transpiration_mmol_m2_s: f64,
    /// True when soil potential is at or below the closure point and the
    /// physical output is clipped to zero rather than a negative conductance.
    pub closure_limited: bool,
}

/// Evaluate the Buckley et al. (2026) process-based stomatal Eq. 3a/3b.
///
/// The only modeled empirical coefficients are s and b_o. K and psi_c are
/// explicit biophysical parameters and may be supplied from measurements or
/// separately qualified model owners. The equation assumes the atmospheric
/// water-vapor mole-fraction difference and the soil-to-leaf hydraulic
/// conductance are defined on compatible leaf-area and time bases.
///
/// At or below the closure point, the implementation returns a closed
/// stomatal state (zero stomatal conductance) rather than a negative value
/// outside the physical domain of Eq. 3a.
pub fn evaluate_buckley_stomatal_model(
    inputs: BuckleyStomatalInputs,
) -> Result<BuckleyStomatalState, BuckleyStomatalError> {
    validate_inputs(inputs)?;

    let bioenergetic_capacity = inputs.co2_saturated_assimilation_umol_m2_s
        - inputs.net_assimilation_umol_m2_s
        + inputs.basal_bioenergetic_capacity_umol_m2_s;

    if !bioenergetic_capacity.is_finite() {
        return Err(BuckleyStomatalError::NonFiniteDerivedValue(
            "bioenergetic_capacity",
        ));
    }
    if bioenergetic_capacity <= 0.0 {
        return Err(BuckleyStomatalError::NonPositiveBioenergeticCapacity);
    }

    let closure_limited =
        inputs.soil_water_potential_mpa <= inputs.stomatal_closure_potential_mpa;
    if closure_limited {
        return Ok(BuckleyStomatalState {
            bioenergetic_capacity_umol_m2_s: bioenergetic_capacity,
            stomatal_conductance_mol_m2_s: 0.0,
            transpiration_mmol_m2_s: 0.0,
            closure_limited: true,
        });
    }

    let sb = inputs.sensitivity_mol_per_umol_mpa * bioenergetic_capacity;
    let water_feedback = sb * inputs.delta_w_mmol_per_mol
        / inputs.soil_leaf_hydraulic_conductance_mmol_m2_s_mpa;
    let denominator = 1.0 + water_feedback;
    let numerator = sb
        * (inputs.soil_water_potential_mpa - inputs.stomatal_closure_potential_mpa);
    let conductance = numerator / denominator;
    let transpiration = conductance * inputs.delta_w_mmol_per_mol;

    for (field, value) in [
        ("sb", sb),
        ("water_feedback", water_feedback),
        ("denominator", denominator),
        ("numerator", numerator),
        ("stomatal_conductance", conductance),
        ("transpiration", transpiration),
    ] {
        if !value.is_finite() {
            return Err(BuckleyStomatalError::NonFiniteDerivedValue(field));
        }
    }
    if denominator <= 0.0 || conductance < 0.0 || transpiration < 0.0 {
        return Err(BuckleyStomatalError::NonFiniteDerivedValue(
            "non_physical_equation_result",
        ));
    }

    Ok(BuckleyStomatalState {
        bioenergetic_capacity_umol_m2_s: bioenergetic_capacity,
        stomatal_conductance_mol_m2_s: conductance,
        transpiration_mmol_m2_s: transpiration,
        closure_limited: false,
    })
}

/// Convert leaf-to-air vapor-pressure deficit (Pa) to an approximate
/// leaf-to-air vapor mole-fraction difference (mmol mol^-1):
/// delta_w ~= 1000 * VPD / ambient_pressure.
///
/// This ideal-gas ratio is a boundary conversion, not a leaf-energy or
/// leaf-temperature model. VPD should be calculated at the relevant leaf
/// temperature and measurement height before calling this function.
pub fn delta_w_from_vpd_pa(
    vapor_pressure_deficit_pa: f64,
    atmospheric_pressure_pa: f64,
) -> Result<f64, BuckleyStomatalError> {
    if !vapor_pressure_deficit_pa.is_finite() {
        return Err(BuckleyStomatalError::NonFiniteInput(
            "vapor_pressure_deficit_pa",
        ));
    }
    if !atmospheric_pressure_pa.is_finite() {
        return Err(BuckleyStomatalError::NonFiniteInput(
            "atmospheric_pressure_pa",
        ));
    }
    if vapor_pressure_deficit_pa < 0.0 {
        return Err(BuckleyStomatalError::NegativeVaporPressureDeficit);
    }
    if atmospheric_pressure_pa <= 0.0 {
        return Err(BuckleyStomatalError::InvalidAtmosphericPressure);
    }

    let delta_w = 1_000.0 * vapor_pressure_deficit_pa / atmospheric_pressure_pa;
    if !delta_w.is_finite() {
        return Err(BuckleyStomatalError::NonFiniteDerivedValue(
            "delta_w_from_vpd",
        ));
    }
    Ok(delta_w)
}

/// Calculate A_m for a C3 leaf using the paper's relation A_m = J/4 - R_d.
/// J and R_d are both in umol m^-2 s^-1. This is not the C4 expression.
pub fn c3_co2_saturated_assimilation(
    potential_electron_transport_umol_m2_s: f64,
    dark_respiration_umol_m2_s: f64,
) -> Result<f64, BuckleyStomatalError> {
    if !potential_electron_transport_umol_m2_s.is_finite() {
        return Err(BuckleyStomatalError::NonFiniteInput(
            "potential_electron_transport_umol_m2_s",
        ));
    }
    if !dark_respiration_umol_m2_s.is_finite() {
        return Err(BuckleyStomatalError::NonFiniteInput(
            "dark_respiration_umol_m2_s",
        ));
    }
    if potential_electron_transport_umol_m2_s < 0.0 {
        return Err(BuckleyStomatalError::NegativeElectronTransportRate);
    }

    let assimilation = 0.25 * potential_electron_transport_umol_m2_s
        - dark_respiration_umol_m2_s;
    if !assimilation.is_finite() {
        return Err(BuckleyStomatalError::NonFiniteDerivedValue(
            "c3_co2_saturated_assimilation",
        ));
    }
    Ok(assimilation)
}

fn validate_inputs(inputs: BuckleyStomatalInputs) -> Result<(), BuckleyStomatalError> {
    for (field, value) in [
        ("soil_water_potential_mpa", inputs.soil_water_potential_mpa),
        (
            "stomatal_closure_potential_mpa",
            inputs.stomatal_closure_potential_mpa,
        ),
        (
            "soil_leaf_hydraulic_conductance",
            inputs.soil_leaf_hydraulic_conductance_mmol_m2_s_mpa,
        ),
        ("delta_w_mmol_per_mol", inputs.delta_w_mmol_per_mol),
        (
            "sensitivity_mol_per_umol_mpa",
            inputs.sensitivity_mol_per_umol_mpa,
        ),
        (
            "co2_saturated_assimilation",
            inputs.co2_saturated_assimilation_umol_m2_s,
        ),
        (
            "net_assimilation",
            inputs.net_assimilation_umol_m2_s,
        ),
        (
            "basal_bioenergetic_capacity",
            inputs.basal_bioenergetic_capacity_umol_m2_s,
        ),
    ] {
        if !value.is_finite() {
            return Err(BuckleyStomatalError::NonFiniteInput(field));
        }
    }

    if inputs.soil_leaf_hydraulic_conductance_mmol_m2_s_mpa <= 0.0 {
        return Err(BuckleyStomatalError::NonPositiveHydraulicConductance);
    }
    if inputs.sensitivity_mol_per_umol_mpa <= 0.0 {
        return Err(BuckleyStomatalError::NonPositiveSensitivity);
    }
    if inputs.delta_w_mmol_per_mol < 0.0 {
        return Err(BuckleyStomatalError::NegativeVaporMoleFractionDifference);
    }
    if inputs.basal_bioenergetic_capacity_umol_m2_s < 0.0 {
        return Err(BuckleyStomatalError::NegativeBasalBioenergeticCapacity);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reference_inputs() -> BuckleyStomatalInputs {
        BuckleyStomatalInputs {
            soil_water_potential_mpa: 0.0,
            stomatal_closure_potential_mpa: -2.0,
            soil_leaf_hydraulic_conductance_mmol_m2_s_mpa: 3.0,
            delta_w_mmol_per_mol: 10.0,
            sensitivity_mol_per_umol_mpa: 0.02,
            co2_saturated_assimilation_umol_m2_s: 10.0,
            net_assimilation_umol_m2_s: 8.0,
            basal_bioenergetic_capacity_umol_m2_s: 0.3,
        }
    }

    #[test]
    fn evaluates_published_equation_3a_and_3b() {
        let inputs = reference_inputs();
        let state = evaluate_buckley_stomatal_model(inputs).unwrap();
        let b = 10.0 - 8.0 + 0.3;
        let expected =
            0.02 * b * 2.0 / (1.0 + (0.02 * b * 10.0 / 3.0));

        assert!((state.bioenergetic_capacity_umol_m2_s - b).abs() < 1e-12);
        assert!((state.stomatal_conductance_mol_m2_s - expected).abs() < 1e-12);
        assert!(
            (state.transpiration_mmol_m2_s
                - expected * inputs.delta_w_mmol_per_mol)
                .abs()
                < 1e-12
        );
        assert!(!state.closure_limited);
    }

    #[test]
    fn lower_soil_leaf_conductance_reduces_stomatal_conductance() {
        let baseline = evaluate_buckley_stomatal_model(reference_inputs()).unwrap();
        let mut restricted = reference_inputs();
        restricted.soil_leaf_hydraulic_conductance_mmol_m2_s_mpa = 1.0;
        let stressed = evaluate_buckley_stomatal_model(restricted).unwrap();

        assert!(
            stressed.stomatal_conductance_mol_m2_s
                < baseline.stomatal_conductance_mol_m2_s
        );
    }

    #[test]
    fn higher_vapor_demand_reduces_conductance_but_increases_transpiration() {
        let baseline = evaluate_buckley_stomatal_model(reference_inputs()).unwrap();
        let mut high_demand = reference_inputs();
        high_demand.delta_w_mmol_per_mol = 20.0;
        let stressed = evaluate_buckley_stomatal_model(high_demand).unwrap();

        assert!(
            stressed.stomatal_conductance_mol_m2_s
                < baseline.stomatal_conductance_mol_m2_s
        );
        assert!(stressed.transpiration_mmol_m2_s > baseline.transpiration_mmol_m2_s);
    }

    #[test]
    fn reduced_soil_water_potential_closes_stomata_at_the_threshold() {
        let mut dry = reference_inputs();
        dry.soil_water_potential_mpa = -2.0;
        let state = evaluate_buckley_stomatal_model(dry).unwrap();

        assert!(state.closure_limited);
        assert_eq!(state.stomatal_conductance_mol_m2_s, 0.0);
        assert_eq!(state.transpiration_mmol_m2_s, 0.0);
    }

    #[test]
    fn basal_capacity_preserves_stomatal_component_in_darkness() {
        let mut dark = reference_inputs();
        dark.co2_saturated_assimilation_umol_m2_s = 0.0;
        dark.net_assimilation_umol_m2_s = 0.0;
        let state = evaluate_buckley_stomatal_model(dark).unwrap();

        assert!(state.stomatal_conductance_mol_m2_s > 0.0);
        assert_eq!(state.bioenergetic_capacity_umol_m2_s, 0.3);
    }

    #[test]
    fn increased_residual_photosynthetic_capacity_increases_conductance() {
        let baseline = evaluate_buckley_stomatal_model(reference_inputs()).unwrap();
        let mut more_residual_capacity = reference_inputs();
        more_residual_capacity.net_assimilation_umol_m2_s = 5.0;
        let state = evaluate_buckley_stomatal_model(more_residual_capacity).unwrap();

        assert!(
            state.stomatal_conductance_mol_m2_s
                > baseline.stomatal_conductance_mol_m2_s
        );
    }

    #[test]
    fn c3_co2_saturated_assimilation_uses_quarter_electron_transport_minus_rd() {
        let am = c3_co2_saturated_assimilation(80.0, 2.0).unwrap();
        assert_eq!(am, 18.0);
    }

    #[test]
    fn converts_vpd_to_mole_fraction_difference() {
        let delta_w = delta_w_from_vpd_pa(1_000.0, 100_000.0).unwrap();
        assert!((delta_w - 10.0).abs() < 1e-12);
    }

    #[test]
    fn zero_vpd_gives_finite_conductance_and_zero_transpiration() {
        let mut inputs = reference_inputs();
        inputs.delta_w_mmol_per_mol = 0.0;
        let state = evaluate_buckley_stomatal_model(inputs).unwrap();

        assert!(state.stomatal_conductance_mol_m2_s > 0.0);
        assert_eq!(state.transpiration_mmol_m2_s, 0.0);
    }

    #[test]
    fn zero_hydraulic_conductance_is_rejected() {
        let mut inputs = reference_inputs();
        inputs.soil_leaf_hydraulic_conductance_mmol_m2_s_mpa = 0.0;
        assert_eq!(
            evaluate_buckley_stomatal_model(inputs),
            Err(BuckleyStomatalError::NonPositiveHydraulicConductance)
        );
    }

    #[test]
    fn negative_delta_w_is_rejected() {
        let mut inputs = reference_inputs();
        inputs.delta_w_mmol_per_mol = -1.0;
        assert_eq!(
            evaluate_buckley_stomatal_model(inputs),
            Err(BuckleyStomatalError::NegativeVaporMoleFractionDifference)
        );
    }

    #[test]
    fn non_finite_parameter_is_rejected() {
        let mut inputs = reference_inputs();
        inputs.sensitivity_mol_per_umol_mpa = f64::NAN;
        assert_eq!(
            evaluate_buckley_stomatal_model(inputs),
            Err(BuckleyStomatalError::NonFiniteInput(
                "sensitivity_mol_per_umol_mpa"
            ))
        );
    }
}
