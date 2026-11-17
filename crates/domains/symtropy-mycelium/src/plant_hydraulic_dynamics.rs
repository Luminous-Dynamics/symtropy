// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! One-compartment plant hydraulic capacitance reference.
//!
//! This module extends the steady-state two-resistance null with a lumped
//! water-storage state. It is intended for numerical verification and
//! controlled pulse experiments, not as a calibrated plant physiology model.
//!
//! Soil/root conductances and soil water potential remain supplied boundary
//! parameters. The model does not create a soil solver, root architecture,
//! embolism model, aquaporin kinetics, biochemical regulation, or signalling
//! pathway. A single capacitance is attached to the lumped leaf/xylem state.
//!
//! The implicit step solves:
//!
//! C * (psi_new - psi_old) / dt
//!   = K_eq * (psi_soil - psi_new) - E(psi_new)
//!
//! where E is the explicit linear stomatal-closure null from
//! plant_hydraulic_null. Backward Euler makes the scalar solve monotone and
//! avoids the explicit-step stability restriction for this passive subsystem.

use super::plant_hydraulic_null::{
    PlantHydraulicError, PlantHydraulicInputs, equivalent_hydraulic_conductance,
    stomatal_open_fraction,
};

const WATER_MOLAR_MASS_KG_PER_MOL: f64 = 0.018_015_28;
const BISECTION_ITERATIONS: usize = 96;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlantHydraulicDynamicsError {
    HydraulicInput(PlantHydraulicError),
    NonFiniteState(&'static str),
    InvalidCapacitance,
    InvalidTimeStep,
    NonFiniteDerivedValue(&'static str),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlantHydraulicDynamicState {
    /// Elapsed simulation time (s).
    pub elapsed_time_s: f64,
    /// Lumped plant leaf/xylem water potential (MPa).
    pub leaf_water_potential_mpa: f64,
    /// Signed cumulative soil-to-plant water transfer (kg m⁻²).
    /// Negative values indicate net flow from plant back to soil.
    pub cumulative_soil_water_uptake_kg_m2: f64,
    /// Cumulative transpired water (kg m⁻²).
    pub cumulative_transpired_water_kg_m2: f64,
    /// Cumulative change in capacitance-backed plant water storage (kg m⁻²).
    pub cumulative_storage_change_kg_m2: f64,
}

impl PlantHydraulicDynamicState {
    pub fn new(initial_leaf_water_potential_mpa: f64) -> Result<Self, PlantHydraulicDynamicsError> {
        if !initial_leaf_water_potential_mpa.is_finite() {
            return Err(PlantHydraulicDynamicsError::NonFiniteState(
                "initial_leaf_water_potential_mpa",
            ));
        }

        Ok(Self {
            elapsed_time_s: 0.0,
            leaf_water_potential_mpa: initial_leaf_water_potential_mpa,
            cumulative_soil_water_uptake_kg_m2: 0.0,
            cumulative_transpired_water_kg_m2: 0.0,
            cumulative_storage_change_kg_m2: 0.0,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlantHydraulicDynamicStep {
    pub state: PlantHydraulicDynamicState,
    /// Equivalent soil-to-root-to-leaf flux (kg m⁻² s⁻¹).
    pub soil_to_root_water_flux_kg_m2_s: f64,
    /// Equivalent root-to-leaf flux (kg m⁻² s⁻¹).
    pub root_to_leaf_water_flux_kg_m2_s: f64,
    /// Stomatal transpiration (kg m⁻² s⁻¹).
    pub transpiration_kg_m2_s: f64,
    /// Lumped root/xylem junction potential (MPa).
    pub root_water_potential_mpa: f64,
    /// Soil-to-root pressure-potential drop (MPa).
    pub soil_to_root_pressure_drop_mpa: f64,
    /// Root-to-leaf pressure-potential drop (MPa).
    pub root_to_leaf_pressure_drop_mpa: f64,
    /// Capacitance-backed storage change during this step (kg m⁻²).
    pub storage_change_kg_m2: f64,
    /// Per-step water balance residual (kg m⁻²).
    pub water_balance_residual_kg_m2: f64,
    /// Cumulative water balance residual since the initial state (kg m⁻²).
    pub cumulative_water_balance_residual_kg_m2: f64,
}

/// Advance the lumped plant hydraulic state by one backward-Euler step.
///
/// Capacitance is expressed in kg m⁻² MPa⁻¹ and time step in seconds.
/// The solver returns signed soil uptake, transpiration, storage change and
/// both per-step and cumulative water-balance residuals. All transfers are
/// calculated from the settled end-of-step state and are integrated over the
/// declared interval, matching the backward-Euler discretization.
pub fn step_plant_hydraulic_capacitance(
    inputs: PlantHydraulicInputs,
    hydraulic_capacitance_kg_m2_mpa: f64,
    time_step_s: f64,
    previous: PlantHydraulicDynamicState,
) -> Result<PlantHydraulicDynamicStep, PlantHydraulicDynamicsError> {
    if !hydraulic_capacitance_kg_m2_mpa.is_finite() || hydraulic_capacitance_kg_m2_mpa <= 0.0 {
        return Err(PlantHydraulicDynamicsError::InvalidCapacitance);
    }
    if !time_step_s.is_finite() || time_step_s <= 0.0 {
        return Err(PlantHydraulicDynamicsError::InvalidTimeStep);
    }

    for (field, value) in [
        ("elapsed_time_s", previous.elapsed_time_s),
        (
            "leaf_water_potential_mpa",
            previous.leaf_water_potential_mpa,
        ),
        (
            "cumulative_soil_water_uptake_kg_m2",
            previous.cumulative_soil_water_uptake_kg_m2,
        ),
        (
            "cumulative_transpired_water_kg_m2",
            previous.cumulative_transpired_water_kg_m2,
        ),
        (
            "cumulative_storage_change_kg_m2",
            previous.cumulative_storage_change_kg_m2,
        ),
    ] {
        if !value.is_finite() {
            return Err(PlantHydraulicDynamicsError::NonFiniteState(field));
        }
    }

    // Validate the shared hydraulic parameters and obtain the series
    // conductance without solving a second, unused steady-state trajectory.
    let equivalent_conductance = equivalent_hydraulic_conductance(inputs)
        .map_err(PlantHydraulicDynamicsError::HydraulicInput)?;
    let storage_rate = hydraulic_capacitance_kg_m2_mpa / time_step_s;
    let next_elapsed_time = previous.elapsed_time_s + time_step_s;

    if !storage_rate.is_finite() || !next_elapsed_time.is_finite() {
        return Err(PlantHydraulicDynamicsError::NonFiniteDerivedValue(
            "storage_rate_or_elapsed_time",
        ));
    }

    let vapor_fraction = inputs.vapor_pressure_deficit_pa / inputs.atmospheric_pressure_pa;
    let stomatal_demand = |leaf_potential_mpa: f64| {
        let open_fraction = stomatal_open_fraction(
            leaf_potential_mpa,
            inputs.stomatal_open_leaf_potential_mpa,
            inputs.stomatal_closed_leaf_potential_mpa,
        );
        inputs.maximum_stomatal_conductance_mol_m2_s
            * open_fraction
            * vapor_fraction
            * WATER_MOLAR_MASS_KG_PER_MOL
    };

    // The residual is monotone increasing in leaf potential. The lower bound
    // is no greater than the old state, soil potential or closure threshold,
    // making its residual non-positive. The upper bound is no less than the
    // old state, soil potential or open threshold, making its residual
    // non-negative. This gives a deterministic physical bracket.
    let mut lower = previous
        .leaf_water_potential_mpa
        .min(inputs.soil_water_potential_mpa)
        .min(inputs.stomatal_closed_leaf_potential_mpa);
    let mut upper = previous
        .leaf_water_potential_mpa
        .max(inputs.soil_water_potential_mpa)
        .max(inputs.stomatal_open_leaf_potential_mpa);

    let residual_at = |leaf_potential_mpa: f64| {
        storage_rate * (leaf_potential_mpa - previous.leaf_water_potential_mpa)
            - equivalent_conductance * (inputs.soil_water_potential_mpa - leaf_potential_mpa)
            + stomatal_demand(leaf_potential_mpa)
    };

    let lower_residual = residual_at(lower);
    let upper_residual = residual_at(upper);
    if !lower_residual.is_finite() || !upper_residual.is_finite() {
        return Err(PlantHydraulicDynamicsError::NonFiniteDerivedValue(
            "implicit_bracket_residual",
        ));
    }
    if lower_residual > 0.0 || upper_residual < 0.0 {
        return Err(PlantHydraulicDynamicsError::NonFiniteDerivedValue(
            "implicit_bracket_order",
        ));
    }

    for _ in 0..BISECTION_ITERATIONS {
        let width = upper - lower;
        if !width.is_finite() {
            return Err(PlantHydraulicDynamicsError::NonFiniteDerivedValue(
                "implicit_bracket_width",
            ));
        }
        let midpoint = lower + width * 0.5;
        if midpoint <= lower || midpoint >= upper {
            break;
        }
        let residual = residual_at(midpoint);
        if !residual.is_finite() {
            return Err(PlantHydraulicDynamicsError::NonFiniteDerivedValue(
                "implicit_midpoint_residual",
            ));
        }
        if residual > 0.0 {
            upper = midpoint;
        } else {
            lower = midpoint;
        }
    }

    let leaf_potential = lower + (upper - lower) * 0.5;
    let soil_to_root_flux =
        equivalent_conductance * (inputs.soil_water_potential_mpa - leaf_potential);
    let root_to_leaf_drop = soil_to_root_flux / inputs.root_to_leaf_conductance_kg_m2_s_mpa;
    let soil_to_root_drop = soil_to_root_flux / inputs.soil_to_root_conductance_kg_m2_s_mpa;
    let root_potential = inputs.soil_water_potential_mpa - soil_to_root_drop;
    let transpiration = stomatal_demand(leaf_potential);
    let storage_change =
        hydraulic_capacitance_kg_m2_mpa * (leaf_potential - previous.leaf_water_potential_mpa);
    let step_residual = storage_change - (soil_to_root_flux - transpiration) * time_step_s;

    let cumulative_soil =
        previous.cumulative_soil_water_uptake_kg_m2 + soil_to_root_flux * time_step_s;
    let cumulative_transpiration =
        previous.cumulative_transpired_water_kg_m2 + transpiration * time_step_s;
    let cumulative_storage = previous.cumulative_storage_change_kg_m2 + storage_change;
    let cumulative_residual = cumulative_storage - (cumulative_soil - cumulative_transpiration);

    for (field, value) in [
        ("leaf_water_potential", leaf_potential),
        ("soil_to_root_flux", soil_to_root_flux),
        ("root_to_leaf_pressure_drop", root_to_leaf_drop),
        ("soil_to_root_pressure_drop", soil_to_root_drop),
        ("root_water_potential", root_potential),
        ("transpiration", transpiration),
        ("storage_change", storage_change),
        ("water_balance_residual", step_residual),
        ("cumulative_soil_water_uptake", cumulative_soil),
        ("cumulative_transpiration", cumulative_transpiration),
        ("cumulative_storage_change", cumulative_storage),
        ("cumulative_water_balance_residual", cumulative_residual),
    ] {
        if !value.is_finite() {
            return Err(PlantHydraulicDynamicsError::NonFiniteDerivedValue(field));
        }
    }

    Ok(PlantHydraulicDynamicStep {
        state: PlantHydraulicDynamicState {
            elapsed_time_s: next_elapsed_time,
            leaf_water_potential_mpa: leaf_potential,
            cumulative_soil_water_uptake_kg_m2: cumulative_soil,
            cumulative_transpired_water_kg_m2: cumulative_transpiration,
            cumulative_storage_change_kg_m2: cumulative_storage,
        },
        soil_to_root_water_flux_kg_m2_s: soil_to_root_flux,
        root_to_leaf_water_flux_kg_m2_s: soil_to_root_flux,
        transpiration_kg_m2_s: transpiration,
        root_water_potential_mpa: root_potential,
        soil_to_root_pressure_drop_mpa: soil_to_root_drop,
        root_to_leaf_pressure_drop_mpa: root_to_leaf_drop,
        storage_change_kg_m2: storage_change,
        water_balance_residual_kg_m2: step_residual,
        cumulative_water_balance_residual_kg_m2: cumulative_residual,
    })
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
    fn zero_vpd_equilibrates_storage_toward_soil_potential() {
        let mut inputs = reference_inputs();
        inputs.vapor_pressure_deficit_pa = 0.0;
        let initial = PlantHydraulicDynamicState::new(-1.0).unwrap();

        let step = step_plant_hydraulic_capacitance(inputs, 0.02, 60.0, initial).unwrap();

        assert!(step.state.leaf_water_potential_mpa > -1.0);
        assert!(step.state.leaf_water_potential_mpa < 0.0);
        assert_eq!(step.transpiration_kg_m2_s, 0.0);
    }

    #[test]
    fn higher_vpd_increases_transpiration_and_depresses_leaf_potential() {
        let initial = PlantHydraulicDynamicState::new(-0.2).unwrap();
        let baseline =
            step_plant_hydraulic_capacitance(reference_inputs(), 0.02, 60.0, initial).unwrap();
        let mut high_vpd = reference_inputs();
        high_vpd.vapor_pressure_deficit_pa = 2_000.0;
        let stressed = step_plant_hydraulic_capacitance(high_vpd, 0.02, 60.0, initial).unwrap();

        assert!(stressed.transpiration_kg_m2_s > baseline.transpiration_kg_m2_s);
        assert!(stressed.state.leaf_water_potential_mpa < baseline.state.leaf_water_potential_mpa);
    }

    #[test]
    fn soil_supply_pulse_reduces_leaf_potential_and_uptake() {
        let initial = PlantHydraulicDynamicState::new(-0.2).unwrap();
        let baseline =
            step_plant_hydraulic_capacitance(reference_inputs(), 0.02, 60.0, initial).unwrap();
        let mut dry_soil = reference_inputs();
        dry_soil.soil_water_potential_mpa = -1.0;
        let stressed = step_plant_hydraulic_capacitance(dry_soil, 0.02, 60.0, initial).unwrap();

        assert!(stressed.state.leaf_water_potential_mpa < baseline.state.leaf_water_potential_mpa);
        assert!(
            stressed.soil_to_root_water_flux_kg_m2_s < baseline.soil_to_root_water_flux_kg_m2_s
        );
    }

    #[test]
    fn per_step_and_cumulative_water_balances_close() {
        let inputs = reference_inputs();
        let mut state = PlantHydraulicDynamicState::new(-0.2).unwrap();

        for _ in 0..500 {
            let step = step_plant_hydraulic_capacitance(inputs, 0.02, 10.0, state).unwrap();
            assert!(
                step.water_balance_residual_kg_m2.abs() < 1e-11,
                "step residual={}",
                step.water_balance_residual_kg_m2
            );
            state = step.state;
        }

        assert!(
            (state.cumulative_storage_change_kg_m2
                - (state.cumulative_soil_water_uptake_kg_m2
                    - state.cumulative_transpired_water_kg_m2))
                .abs()
                < 1e-8
        );
    }

    #[test]
    fn constant_forcing_converges_to_the_steady_state_null() {
        let inputs = reference_inputs();
        let steady = solve_two_resistance_null(inputs).unwrap();
        let mut state = PlantHydraulicDynamicState::new(-1.5).unwrap();

        for _ in 0..5_000 {
            state = step_plant_hydraulic_capacitance(inputs, 0.02, 10.0, state)
                .unwrap()
                .state;
        }

        assert!((state.leaf_water_potential_mpa - steady.leaf_water_potential_mpa).abs() < 1e-8);
    }

    #[test]
    fn rewatering_recovers_leaf_potential_after_supply_pulse() {
        let mut drought = reference_inputs();
        drought.soil_water_potential_mpa = -1.5;
        let mut state = PlantHydraulicDynamicState::new(-0.2).unwrap();

        for _ in 0..100 {
            state = step_plant_hydraulic_capacitance(drought, 0.02, 10.0, state)
                .unwrap()
                .state;
        }
        let drought_potential = state.leaf_water_potential_mpa;
        let recovered = step_plant_hydraulic_capacitance(reference_inputs(), 0.02, 10.0, state)
            .unwrap()
            .state;

        assert!(recovered.leaf_water_potential_mpa > drought_potential);
    }

    #[test]
    fn smaller_time_step_reduces_implicit_discretization_error() {
        let inputs = reference_inputs();
        let initial = PlantHydraulicDynamicState::new(-1.5).unwrap();
        let mut coarse = initial;
        let mut fine = initial;

        for _ in 0..10 {
            coarse = step_plant_hydraulic_capacitance(inputs, 0.02, 1.0, coarse)
                .unwrap()
                .state;
        }
        for _ in 0..20 {
            fine = step_plant_hydraulic_capacitance(inputs, 0.02, 0.5, fine)
                .unwrap()
                .state;
        }

        let steady = solve_two_resistance_null(inputs).unwrap();
        let coarse_error =
            (coarse.leaf_water_potential_mpa - steady.leaf_water_potential_mpa).abs();
        let fine_error = (fine.leaf_water_potential_mpa - steady.leaf_water_potential_mpa).abs();
        assert!(fine_error <= coarse_error + 1e-12);
    }

    #[test]
    fn identical_current_forcing_retains_lumped_storage_history() {
        let normal = reference_inputs();
        let mut drought = normal;
        drought.soil_water_potential_mpa = -1.5;
        let mut drought_history = PlantHydraulicDynamicState::new(-0.2).unwrap();
        let mut control_history = drought_history;

        for _ in 0..100 {
            drought_history =
                step_plant_hydraulic_capacitance(drought, 0.02, 10.0, drought_history)
                    .unwrap()
                    .state;
            control_history = step_plant_hydraulic_capacitance(normal, 0.02, 10.0, control_history)
                .unwrap()
                .state;
        }

        // The external forcing is now identical. Remaining differences are
        // due only to the modelled water-storage state, not acclimation.
        let drought_recovery =
            step_plant_hydraulic_capacitance(normal, 0.02, 10.0, drought_history).unwrap();
        let control_recovery =
            step_plant_hydraulic_capacitance(normal, 0.02, 10.0, control_history).unwrap();

        assert!(
            drought_recovery.state.leaf_water_potential_mpa
                < control_recovery.state.leaf_water_potential_mpa
        );
    }

    #[test]
    fn pressure_drops_reconstruct_leaf_potential() {
        let step = step_plant_hydraulic_capacitance(
            reference_inputs(),
            0.02,
            10.0,
            PlantHydraulicDynamicState::new(-1.0).unwrap(),
        )
        .unwrap();
        let total_drop = step.soil_to_root_pressure_drop_mpa + step.root_to_leaf_pressure_drop_mpa;

        assert!(
            (reference_inputs().soil_water_potential_mpa
                - step.state.leaf_water_potential_mpa
                - total_drop)
                .abs()
                < 1e-12
        );
    }

    #[test]
    fn invalid_capacitance_is_rejected() {
        let result = step_plant_hydraulic_capacitance(
            reference_inputs(),
            0.0,
            10.0,
            PlantHydraulicDynamicState::new(-1.0).unwrap(),
        );
        assert_eq!(result, Err(PlantHydraulicDynamicsError::InvalidCapacitance));
    }

    #[test]
    fn invalid_time_step_is_rejected() {
        let result = step_plant_hydraulic_capacitance(
            reference_inputs(),
            0.02,
            f64::NAN,
            PlantHydraulicDynamicState::new(-1.0).unwrap(),
        );
        assert_eq!(result, Err(PlantHydraulicDynamicsError::InvalidTimeStep));
    }

    #[test]
    fn non_finite_initial_state_is_rejected() {
        assert_eq!(
            PlantHydraulicDynamicState::new(f64::INFINITY),
            Err(PlantHydraulicDynamicsError::NonFiniteState(
                "initial_leaf_water_potential_mpa"
            ))
        );
    }
}
