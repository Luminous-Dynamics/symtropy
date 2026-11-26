//! Bounded analytical screening oracles for two space-system research questions.
//!
//! This crate deliberately does not contain an orbital propagator, optical
//! diffraction model, climate model, plasma/MHD solver, materials model or
//! flight-control system.

use std::f64::consts::PI;

/// Speed of light in vacuum, m/s.
pub const SPEED_OF_LIGHT_M_S: f64 = 299_792_458.0;
/// Newtonian screening is limited to beta <= 0.05 in this release.
pub const MAX_NEWTONIAN_BETA: f64 = 0.05;
/// Illustrative mean Earth radius, m.
pub const EARTH_MEAN_RADIUS_M: f64 = 6_371_000.0;
/// Illustrative total solar irradiance at 1 AU, W/m².
pub const SOLAR_IRRADIANCE_1_AU_W_M2: f64 = 1_361.0;

/// Idealized shade-aperture inputs.
///
/// aperture_effectiveness is an abstract first-order ratio mapping projected
/// aperture area to mean flux reduction over Earth's projected disk. It is
/// NOT simply material reflectivity and is not a qualified optical property.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SunshadeInput {
    pub earth_radius_m: f64,
    pub solar_irradiance_w_m2: f64,
    pub target_flux_reduction_fraction: f64,
    pub aperture_effectiveness: f64,
    /// Installed system areal density, including support hardware when known.
    pub installed_areal_density_kg_m2: f64,
    /// Scenario parameter only; not demonstrated lunar membrane capability.
    pub locally_sourced_material_fraction: f64,
}

/// First-order output of the shade-aperture lower-bound calculation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SunshadeEstimate {
    pub earth_projected_area_m2: f64,
    pub equivalent_aperture_area_m2: f64,
    pub equivalent_aperture_area_km2: f64,
    pub installed_mass_kg: f64,
    pub locally_sourced_mass_kg: f64,
    pub earth_supplied_mass_kg: f64,
    /// Intercepted-flux bookkeeping, not a climate forcing or temperature result.
    pub intercepted_power_reduction_w: f64,
}

/// Evaluate the idealized far-field aperture/mass relation.
///
/// A_aperture = f_reduction * pi * R_earth^2 / aperture_effectiveness.
///
/// This is a geometric-equivalent screening lower bound, not the physical
/// membrane footprint at L1. Finite-Sun umbra/penumbra, diffraction, scattering,
/// orbit family, attitude, sail dynamics and climate response are out of scope.
pub fn evaluate_sunshade(input: SunshadeInput) -> Result<SunshadeEstimate, &'static str> {
    if !finite_positive(input.earth_radius_m) {
        return Err("earth_radius_m must be finite and positive");
    }
    if !finite_positive(input.solar_irradiance_w_m2) {
        return Err("solar_irradiance_w_m2 must be finite and positive");
    }
    if !input.target_flux_reduction_fraction.is_finite()
        || !(0.0..1.0).contains(&input.target_flux_reduction_fraction)
    {
        return Err("target_flux_reduction_fraction must be finite and in [0, 1)");
    }
    if !input.aperture_effectiveness.is_finite()
        || !(0.0..=1.0).contains(&input.aperture_effectiveness)
        || input.aperture_effectiveness == 0.0
    {
        return Err("aperture_effectiveness must be finite and in (0, 1]");
    }
    if !finite_nonnegative(input.installed_areal_density_kg_m2) {
        return Err("installed_areal_density_kg_m2 must be finite and non-negative");
    }
    if !unit_interval(input.locally_sourced_material_fraction) {
        return Err("locally_sourced_material_fraction must be finite and in [0, 1]");
    }

    let earth_projected_area_m2 = PI * input.earth_radius_m.powi(2);
    let equivalent_aperture_area_m2 = input.target_flux_reduction_fraction
        * earth_projected_area_m2
        / input.aperture_effectiveness;
    let installed_mass_kg =
        equivalent_aperture_area_m2 * input.installed_areal_density_kg_m2;
    let locally_sourced_mass_kg =
        installed_mass_kg * input.locally_sourced_material_fraction;
    let earth_supplied_mass_kg = installed_mass_kg - locally_sourced_mass_kg;
    let intercepted_power_reduction_w = input.solar_irradiance_w_m2
        * input.target_flux_reduction_fraction
        * earth_projected_area_m2;

    let outputs = [
        earth_projected_area_m2,
        equivalent_aperture_area_m2,
        installed_mass_kg,
        locally_sourced_mass_kg,
        earth_supplied_mass_kg,
        intercepted_power_reduction_w,
    ];
    if !outputs.into_iter().all(f64::is_finite) {
        return Err("shade calculation overflowed its finite output domain");
    }

    Ok(SunshadeEstimate {
        earth_projected_area_m2,
        equivalent_aperture_area_m2,
        equivalent_aperture_area_km2: equivalent_aperture_area_m2 / 1.0e6,
        installed_mass_kg,
        locally_sourced_mass_kg,
        earth_supplied_mass_kg,
        intercepted_power_reduction_w,
    })
}

/// Inputs for a one-dimensional, Newtonian q-drive screening model.
///
/// effective_interaction_area_m2 and effective_drag_coefficient are
/// phenomenological parameters, not a plasma-magnet constitutive model.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QDriveInput {
    pub plasma_mass_density_kg_m3: f64,
    pub relative_speed_m_s: f64,
    pub effective_interaction_area_m2: f64,
    pub effective_drag_coefficient: f64,
    pub power_conversion_efficiency: f64,
    pub thruster_efficiency: f64,
    pub exhaust_velocity_m_s: f64,
    pub vehicle_mass_kg: f64,
    /// Ratio of non-harvesting parasitic drag to harvesting drag.
    pub parasitic_drag_ratio: f64,
}

/// Auditable first-order q-drive accounting outputs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QDriveEstimate {
    pub beta: f64,
    pub dynamic_pressure_pa: f64,
    pub harvesting_drag_n: f64,
    pub parasitic_drag_n: f64,
    pub harvested_mechanical_power_w: f64,
    pub bus_power_w: f64,
    pub power_conversion_loss_w: f64,
    pub jet_power_w: f64,
    pub thruster_conversion_loss_w: f64,
    pub energy_chain_residual_w: f64,
    pub ideal_thrust_n: f64,
    pub propellant_mass_flow_kg_s: f64,
    pub net_axial_force_n: f64,
    pub net_acceleration_m_s2: f64,
    pub break_even_exhaust_velocity_m_s: f64,
    /// 0.5 * mdot * exhaust_velocity^2 - jet_power; should be near zero.
    pub jet_power_identity_residual_w: f64,
}

/// Evaluate a simplified one-dimensional q-drive closure.
///
/// Assumptions:
/// - Newtonian dynamic-pressure proxy q = 0.5 * rho * v_rel^2;
/// - effective harvesting drag D_h = q * C_D * A_eff;
/// - harvested mechanical power P_h = D_h * v_rel;
/// - bus and jet power are reduced by explicit conversion efficiencies;
/// - exhaust kinetic power P_jet = 0.5 * mdot * u_e^2;
/// - thrust is aligned with the velocity axis and opposes combined drag.
///
/// The function rejects beta > 0.05 for both vehicle/medium relative speed and exhaust speed. The 0.05c-0.2c regime in q-drive
/// literature needs a relativistic energy-momentum derivation.
pub fn evaluate_qdrive(input: QDriveInput) -> Result<QDriveEstimate, &'static str> {
    if !finite_nonnegative(input.plasma_mass_density_kg_m3) {
        return Err("plasma_mass_density_kg_m3 must be finite and non-negative");
    }
    if !finite_nonnegative(input.relative_speed_m_s) {
        return Err("relative_speed_m_s must be finite and non-negative");
    }
    if input.relative_speed_m_s / SPEED_OF_LIGHT_M_S > MAX_NEWTONIAN_BETA {
        return Err("relative speed exceeds the v0 Newtonian screening limit of 0.05c");
    }
    if !finite_nonnegative(input.effective_interaction_area_m2) {
        return Err("effective_interaction_area_m2 must be finite and non-negative");
    }
    if !finite_nonnegative(input.effective_drag_coefficient) {
        return Err("effective_drag_coefficient must be finite and non-negative");
    }
    if !unit_interval(input.power_conversion_efficiency) {
        return Err("power_conversion_efficiency must be finite and in [0, 1]");
    }
    if !unit_interval(input.thruster_efficiency) {
        return Err("thruster_efficiency must be finite and in [0, 1]");
    }
    if !finite_positive(input.exhaust_velocity_m_s) {
        return Err("exhaust_velocity_m_s must be finite and positive");
    }
    if input.exhaust_velocity_m_s / SPEED_OF_LIGHT_M_S > MAX_NEWTONIAN_BETA {
        return Err("exhaust velocity exceeds the v0 Newtonian screening limit of 0.05c");
    }
    if !finite_positive(input.vehicle_mass_kg) {
        return Err("vehicle_mass_kg must be finite and positive");
    }
    if !finite_nonnegative(input.parasitic_drag_ratio) {
        return Err("parasitic_drag_ratio must be finite and non-negative");
    }

    let beta = input.relative_speed_m_s / SPEED_OF_LIGHT_M_S;
    let dynamic_pressure_pa =
        0.5 * input.plasma_mass_density_kg_m3 * input.relative_speed_m_s.powi(2);
    let harvesting_drag_n = dynamic_pressure_pa
        * input.effective_drag_coefficient
        * input.effective_interaction_area_m2;
    let parasitic_drag_n = harvesting_drag_n * input.parasitic_drag_ratio;
    let harvested_mechanical_power_w = harvesting_drag_n * input.relative_speed_m_s;
    let bus_power_w = harvested_mechanical_power_w * input.power_conversion_efficiency;
    let power_conversion_loss_w = harvested_mechanical_power_w - bus_power_w;
    let jet_power_w = bus_power_w * input.thruster_efficiency;
    let thruster_conversion_loss_w = bus_power_w - jet_power_w;
    let energy_chain_residual_w = harvested_mechanical_power_w
        - power_conversion_loss_w
        - thruster_conversion_loss_w
        - jet_power_w;
    let ideal_thrust_n = 2.0 * jet_power_w / input.exhaust_velocity_m_s;
    let propellant_mass_flow_kg_s = ideal_thrust_n / input.exhaust_velocity_m_s;
    let net_axial_force_n = ideal_thrust_n - harvesting_drag_n - parasitic_drag_n;
    let net_acceleration_m_s2 = net_axial_force_n / input.vehicle_mass_kg;
    let break_even_exhaust_velocity_m_s = if harvesting_drag_n > 0.0 {
        2.0
            * input.power_conversion_efficiency
            * input.thruster_efficiency
            * input.relative_speed_m_s
            / (1.0 + input.parasitic_drag_ratio)
    } else {
        0.0
    };
    let jet_power_identity_residual_w = 0.5
        * propellant_mass_flow_kg_s
        * input.exhaust_velocity_m_s.powi(2)
        - jet_power_w;

    let outputs = [
        beta,
        dynamic_pressure_pa,
        harvesting_drag_n,
        parasitic_drag_n,
        harvested_mechanical_power_w,
        bus_power_w,
        power_conversion_loss_w,
        jet_power_w,
        thruster_conversion_loss_w,
        energy_chain_residual_w,
        ideal_thrust_n,
        propellant_mass_flow_kg_s,
        net_axial_force_n,
        net_acceleration_m_s2,
        break_even_exhaust_velocity_m_s,
        jet_power_identity_residual_w,
    ];
    if !outputs.into_iter().all(f64::is_finite) {
        return Err("q-drive calculation overflowed its finite output domain");
    }

    Ok(QDriveEstimate {
        beta,
        dynamic_pressure_pa,
        harvesting_drag_n,
        parasitic_drag_n,
        harvested_mechanical_power_w,
        bus_power_w,
        power_conversion_loss_w,
        jet_power_w,
        thruster_conversion_loss_w,
        energy_chain_residual_w,
        ideal_thrust_n,
        propellant_mass_flow_kg_s,
        net_axial_force_n,
        net_acceleration_m_s2,
        break_even_exhaust_velocity_m_s,
        jet_power_identity_residual_w,
    })
}

/// Synthetic shade reference case; parameters are deliberately illustrative.
pub fn illustrative_sunshade_case() -> SunshadeInput {
    SunshadeInput {
        earth_radius_m: EARTH_MEAN_RADIUS_M,
        solar_irradiance_w_m2: SOLAR_IRRADIANCE_1_AU_W_M2,
        target_flux_reduction_fraction: 0.01,
        aperture_effectiveness: 0.90,
        installed_areal_density_kg_m2: 0.01,
        locally_sourced_material_fraction: 0.50,
    }
}

/// Synthetic q-drive arithmetic case, not a real plasma or hardware estimate.
pub fn illustrative_qdrive_case() -> QDriveInput {
    QDriveInput {
        plasma_mass_density_kg_m3: 1.0e-20,
        relative_speed_m_s: 0.05 * SPEED_OF_LIGHT_M_S,
        effective_interaction_area_m2: 1.0e8,
        effective_drag_coefficient: 1.0,
        power_conversion_efficiency: 0.80,
        thruster_efficiency: 0.60,
        exhaust_velocity_m_s: 0.043 * SPEED_OF_LIGHT_M_S,
        vehicle_mass_kg: 1.0e6,
        parasitic_drag_ratio: 0.10,
    }
}

fn finite_positive(value: f64) -> bool {
    value.is_finite() && value > 0.0
}

fn finite_nonnegative(value: f64) -> bool {
    value.is_finite() && value >= 0.0
}

fn unit_interval(value: f64) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(actual: f64, expected: f64, tolerance: f64) {
        assert!(
            (actual - expected).abs() <= tolerance,
            "actual {actual}, expected {expected}, tolerance {tolerance}"
        );
    }

    #[test]
    fn shade_fixture_matches_independent_geometric_oracle() {
        let result = evaluate_sunshade(illustrative_sunshade_case()).unwrap();
        close(result.earth_projected_area_m2, 1.275_161_179_774_470_6e14, 1.0);
        close(result.equivalent_aperture_area_m2, 1.416_845_755_304_967_5e12, 1.0);
        close(result.installed_mass_kg, 1.416_845_755_304_967_5e10, 0.01);
        close(
            result.intercepted_power_reduction_w,
            1.735_494_365_673_054_5e15,
            10_000.0,
        );
        close(
            result.locally_sourced_mass_kg + result.earth_supplied_mass_kg,
            result.installed_mass_kg,
            0.001,
        );
    }

    #[test]
    fn shade_area_scales_linearly_with_target_fraction() {
        let input = illustrative_sunshade_case();
        let base = evaluate_sunshade(input).unwrap();
        let doubled = evaluate_sunshade(SunshadeInput {
            target_flux_reduction_fraction: input.target_flux_reduction_fraction * 2.0,
            ..input
        })
        .unwrap();
        close(doubled.equivalent_aperture_area_m2, 2.0 * base.equivalent_aperture_area_m2, 1e-3);
        close(doubled.installed_mass_kg, 2.0 * base.installed_mass_kg, 1e-2);
    }

    #[test]
    fn shade_area_scales_inversely_with_aperture_effectiveness() {
        let input = illustrative_sunshade_case();
        let base = evaluate_sunshade(input).unwrap();
        let less_effective = evaluate_sunshade(SunshadeInput {
            aperture_effectiveness: input.aperture_effectiveness * 0.5,
            ..input
        })
        .unwrap();
        close(
            less_effective.equivalent_aperture_area_m2,
            2.0 * base.equivalent_aperture_area_m2,
            1e-3,
        );
    }

    #[test]
    fn shade_rejects_invalid_optics_mass_and_non_finite_values() {
        let input = illustrative_sunshade_case();
        assert!(evaluate_sunshade(SunshadeInput { aperture_effectiveness: 0.0, ..input }).is_err());
        assert!(evaluate_sunshade(SunshadeInput {
            locally_sourced_material_fraction: 1.1,
            ..input
        })
        .is_err());
        assert!(evaluate_sunshade(SunshadeInput {
            installed_areal_density_kg_m2: f64::NAN,
            ..input
        })
        .is_err());
    }

    #[test]
    fn qdrive_fixture_closes_ideal_jet_energy_identity() {
        let result = evaluate_qdrive(illustrative_qdrive_case()).unwrap();
        close(result.dynamic_pressure_pa, 1.123_443_973_421_022e-6, 1e-18);
        close(result.harvesting_drag_n, 112.344_397_342_102_2, 1e-9);
        close(result.harvested_mechanical_power_w, 1.684_000_151_085_874_3e9, 1e-3);
        close(result.bus_power_w, 1.347_200_120_868_699_6e9, 1e-3);
        close(result.power_conversion_loss_w, 3.368_000_302_171_747_7e8, 1e-3);
        close(result.jet_power_w, 8.083_200_725_212_197e8, 1e-3);
        close(result.thruster_conversion_loss_w, 5.388_800_483_474_798e8, 1e-3);
        close(result.energy_chain_residual_w, 0.0, 1e-6);
        close(result.ideal_thrust_n, 125.407_699_358_625_73, 1e-9);
        close(result.propellant_mass_flow_kg_s, 9.728_257_155_219_04e-6, 1e-14);
        close(result.net_axial_force_n, 1.828_862_282_313_309_6, 1e-9);
        close(result.net_acceleration_m_s2, 1.828_862_282_313_309_6e-6, 1e-15);
        close(result.jet_power_identity_residual_w, 0.0, 1e-6);
    }

    #[test]
    fn qdrive_zero_density_and_zero_speed_have_no_harvested_power() {
        let input = illustrative_qdrive_case();
        for changed in [
            QDriveInput { plasma_mass_density_kg_m3: 0.0, ..input },
            QDriveInput { relative_speed_m_s: 0.0, ..input },
        ] {
            let result = evaluate_qdrive(changed).unwrap();
            assert_eq!(result.harvested_mechanical_power_w, 0.0);
            assert_eq!(result.bus_power_w, 0.0);
            assert_eq!(result.jet_power_w, 0.0);
            assert_eq!(result.ideal_thrust_n, 0.0);
        }
    }

    #[test]
    fn qdrive_zero_efficiency_cannot_produce_powered_thrust() {
        let input = illustrative_qdrive_case();
        let result = evaluate_qdrive(QDriveInput {
            power_conversion_efficiency: 0.0,
            ..input
        })
        .unwrap();
        assert_eq!(result.bus_power_w, 0.0);
        assert_eq!(result.ideal_thrust_n, 0.0);
        assert!(result.net_axial_force_n < 0.0);
    }

    #[test]
    fn qdrive_positive_net_acceleration_requires_exhaust_below_break_even() {
        let input = illustrative_qdrive_case();
        let base = evaluate_qdrive(input).unwrap();
        assert!(base.net_acceleration_m_s2 > 0.0);
        let above_break_even = evaluate_qdrive(QDriveInput {
            exhaust_velocity_m_s: base.break_even_exhaust_velocity_m_s * 1.01,
            ..input
        })
        .unwrap();
        assert!(above_break_even.net_acceleration_m_s2 < 0.0);
    }

    #[test]
    fn qdrive_harvest_power_scales_with_effective_interaction_area() {
        let input = illustrative_qdrive_case();
        let base = evaluate_qdrive(input).unwrap();
        let doubled = evaluate_qdrive(QDriveInput {
            effective_interaction_area_m2: input.effective_interaction_area_m2 * 2.0,
            ..input
        })
        .unwrap();
        close(doubled.harvested_mechanical_power_w, 2.0 * base.harvested_mechanical_power_w, 1e-3);
        close(doubled.harvesting_drag_n, 2.0 * base.harvesting_drag_n, 1e-9);
    }

    #[test]
    fn qdrive_rejects_relativistic_extrapolation_and_invalid_parameters() {
        let input = illustrative_qdrive_case();
        assert!(evaluate_qdrive(QDriveInput {
            relative_speed_m_s: 0.051 * SPEED_OF_LIGHT_M_S,
            ..input
        })
        .is_err());
        assert!(evaluate_qdrive(QDriveInput {
            exhaust_velocity_m_s: 0.051 * SPEED_OF_LIGHT_M_S,
            ..input
        })
        .is_err());
        assert!(evaluate_qdrive(QDriveInput {
            power_conversion_efficiency: 1.01,
            ..input
        })
        .is_err());
        assert!(evaluate_qdrive(QDriveInput { plasma_mass_density_kg_m3: -1.0, ..input }).is_err());
        assert!(evaluate_qdrive(QDriveInput { exhaust_velocity_m_s: 0.0, ..input }).is_err());
    }
}
