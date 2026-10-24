// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Analytic standing-loss integration for a lumped sensible store.
//!
//! Assumptions: constant ambient temperature and conductance during an interval,
//! thermal equilibrium within the store, piecewise-linear cp(T), and no ambient
//! heat gain. Unlike explicit Euler, this solves the loss trajectory across the
//! interval without requiring timestep sub-division.

use super::{check, EnergyJ, Kelvin, ModelError, Seconds, StoreConfig, StoreState};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AnalyticStandingLossReport {
    pub temperature_before: Kelvin,
    pub temperature_after: Kelvin,
    pub ambient_temperature: Kelvin,
    pub duration: Seconds,
    pub heat_lost: EnergyJ,
    /// True only when the state reached the configured minimum temperature.
    pub reached_minimum_temperature: bool,
}

/// Cooling time from from to to under piecewise-linear cp(T).
///
/// Because dE/dT = m cp(T) and dE/dt = -UA (T - Tamb), cooling time is
/// m/UA times the integral of cp(T)/(T - Tamb). The segment integral is
/// analytic because cp(T) is linear within each declared segment.
fn cooling_time(
    config: &StoreConfig,
    from: Kelvin,
    to: Kelvin,
    ambient: Kelvin,
) -> Result<f64, ModelError> {
    if from.value() < to.value() {
        return Err(ModelError::ReportMismatch("loss_integrator.temperature_order"));
    }
    if from.value() == to.value() {
        return Ok(0.0);
    }
    if to.value() <= ambient.value() {
        return Ok(f64::INFINITY);
    }
    if config.standing_loss_conductance.value() == 0.0 {
        return Ok(f64::INFINITY);
    }

    let points = &config.heat_capacity.points;
    let low = to.value();
    let high = from.value();
    let mut integral = 0.0;

    for pair in points.windows(2) {
        let t0 = pair[0].temperature.value();
        let t1 = pair[1].temperature.value();
        let a = low.max(t0);
        let b = high.min(t1);
        if b <= a {
            continue;
        }

        let cp0 = pair[0].specific_heat.value();
        let cp1 = pair[1].specific_heat.value();
        let slope = (cp1 - cp0) / (t1 - t0);
        let cp_a = cp0 + slope * (a - t0);
        let delta = b - a;
        let gap = a - ambient.value();
        if gap <= 0.0 {
            return Ok(f64::INFINITY);
        }
        let log_ratio = (delta / gap).ln_1p();
        integral += cp_a * log_ratio + slope * (delta - gap * log_ratio);
    }

    let time = config.mass.value() / config.standing_loss_conductance.value() * integral;
    if !time.is_finite() || time < 0.0 {
        return Err(ModelError::NonFinite("analytic_cooling_time_s"));
    }
    Ok(time)
}

fn solve_final_temperature(
    config: &StoreConfig,
    start: Kelvin,
    ambient: Kelvin,
    duration: Seconds,
) -> Result<(Kelvin, bool), ModelError> {
    let floor = config.min_temperature;
    if config.standing_loss_conductance.value() == 0.0
        || start.value() <= ambient.value()
    {
        return Ok((start, false));
    }

    // If ambient is below the store's declared energy floor, the store reaches
    // Tmin in finite time. Do not clamp an over-large Euler loss: solve the
    // trajectory first and mark the exact floor event explicitly.
    if ambient.value() < floor.value() {
        let floor_time = cooling_time(config, start, floor, ambient)?;
        if duration.value() >= floor_time {
            return Ok((floor, true));
        }
        let mut low = floor.value();
        let mut high = start.value();
        for _ in 0..80 {
            let mid = low + (high - low) * 0.5;
            if mid <= low || mid >= high {
                break;
            }
            let needed = cooling_time(config, start, Kelvin::new(mid)?, ambient)?;
            if needed > duration.value() {
                low = mid;
            } else {
                high = mid;
            }
        }
        return Ok((Kelvin::new(low + (high - low) * 0.5)?, false));
    }

    // For ambient >= Tmin, the continuous model approaches ambient
    // asymptotically and cannot cross it in finite time. Bisection is bounded.
    let mut low = ambient.value();
    let mut high = start.value();
    for _ in 0..80 {
        let mid = low + (high - low) * 0.5;
        if mid <= low || mid >= high {
            break;
        }
        let needed = cooling_time(config, start, Kelvin::new(mid)?, ambient)?;
        if needed > duration.value() {
            low = mid;
        } else {
            high = mid;
        }
    }
    let result = low + (high - low) * 0.5;
    // Preserve a representable temperature strictly above ambient; a finite
    // interval must not be rounded to the asymptote.
    let result = if result <= ambient.value() { high } else { result };
    Ok((Kelvin::new(result)?, false))
}

/// Apply standing loss using the analytic trajectory for the declared cp(T).
///
/// This is an alternative to the existing explicit-Euler approximation; it does
/// not silently replace legacy fixtures. The ambient boundary omits heat gain.
pub fn apply_analytic_standing_loss(
    config: &StoreConfig,
    state: StoreState,
    ambient: Kelvin,
    duration: Seconds,
) -> Result<(StoreState, AnalyticStandingLossReport), ModelError> {
    config.stored_energy(state)?;
    let (temperature_after, reached_minimum_temperature) =
        solve_final_temperature(config, state.temperature, ambient, duration)?;
    let before_energy = config.stored_energy(state)?.value();
    let next_state = StoreState { temperature: temperature_after };
    let after_energy = config.stored_energy(next_state)?.value();
    let heat_lost = EnergyJ::new((before_energy - after_energy).max(0.0))?;
    let report = AnalyticStandingLossReport {
        temperature_before: state.temperature,
        temperature_after,
        ambient_temperature: ambient,
        duration,
        heat_lost,
        reached_minimum_temperature,
    };
    Ok((next_state, report))
}

/// Independent trajectory check.
///
/// This does not call the production solver. It checks the elapsed-time relation
/// from the reported end state, physical bounds, event flag, and energy ledger.
/// The reference expression intentionally uses a log-ratio form separate from
/// the production ln(1 + delta/gap) segment expression.
pub fn verify_analytic_standing_loss_report(
    config: &StoreConfig,
    state: StoreState,
    ambient: Kelvin,
    duration: Seconds,
    claimed: AnalyticStandingLossReport,
) -> Result<(), ModelError> {
    config.stored_energy(state)?;
    check("analytic_loss.temperature_before",
        claimed.temperature_before.value(), state.temperature.value())?;
    check("analytic_loss.ambient", claimed.ambient_temperature.value(), ambient.value())?;
    check("analytic_loss.duration", claimed.duration.value(), duration.value())?;

    let start = state.temperature.value();
    let floor = config.min_temperature.value();
    let end = claimed.temperature_after.value();
    let conductance = config.standing_loss_conductance.value();
    let no_loss = conductance == 0.0 || start <= ambient.value();

    if no_loss {
        check("analytic_loss.no_loss_temperature", end, start)?;
        if claimed.reached_minimum_temperature {
            return Err(ModelError::ReportMismatch("analytic_loss.false_floor_event"));
        }
    } else if claimed.reached_minimum_temperature {
        if ambient.value() >= floor {
            return Err(ModelError::ReportMismatch("analytic_loss.impossible_floor_event"));
        }
        check("analytic_loss.floor_temperature", end, floor)?;
        let time_to_floor = reference_cooling_time(config, state.temperature, config.min_temperature, ambient)?;
        if time_to_floor > duration.value()
            && !super::close(time_to_floor, duration.value())
        {
            return Err(ModelError::ReportMismatch("analytic_loss.early_floor_event"));
        }
    } else {
        if end < floor || end > start || end <= ambient.value() {
            return Err(ModelError::ReportMismatch("analytic_loss.temperature_bounds"));
        }
        if ambient.value() < floor && end <= floor {
            return Err(ModelError::ReportMismatch("analytic_loss.missing_floor_event"));
        }
        let elapsed = reference_cooling_time(config, state.temperature, claimed.temperature_after, ambient)?;
        check("analytic_loss.elapsed_time", elapsed, duration.value())?;
    }

    let before_energy = config.stored_energy(state)?.value();
    let end_state = StoreState { temperature: claimed.temperature_after };
    let after_energy = config.stored_energy(end_state)?.value();
    check("analytic_loss.energy", claimed.heat_lost.value(), before_energy - after_energy)?;
    check("analytic_loss.energy_nonnegative", claimed.heat_lost.value(),
        claimed.heat_lost.value().max(0.0))?;
    check("analytic_loss.first_law", before_energy, after_energy + claimed.heat_lost.value())
}

/// Independent closed-form elapsed-time expression used by the verifier.
/// This integrates each clipped segment using a direct logarithmic ratio.
fn reference_cooling_time(
    config: &StoreConfig,
    from: Kelvin,
    to: Kelvin,
    ambient: Kelvin,
) -> Result<f64, ModelError> {
    if from.value() < to.value() {
        return Err(ModelError::ReportMismatch("reference_loss.temperature_order"));
    }
    if from.value() == to.value() {
        return Ok(0.0);
    }
    if to.value() <= ambient.value() {
        return Ok(f64::INFINITY);
    }
    if config.standing_loss_conductance.value() == 0.0 {
        return Ok(f64::INFINITY);
    }

    let points = &config.heat_capacity.points;
    let mut sum = 0.0;
    let range_low = to.value();
    let range_high = from.value();
    for pair in points.windows(2) {
        let left = pair[0];
        let right = pair[1];
        let segment_low = range_low.max(left.temperature.value());
        let segment_high = range_high.min(right.temperature.value());
        if segment_high <= segment_low {
            continue;
        }

        let slope = (right.specific_heat.value() - left.specific_heat.value())
            / (right.temperature.value() - left.temperature.value());
        let cp_at_low = left.specific_heat.value()
            + slope * (segment_low - left.temperature.value());
        let gap = segment_low - ambient.value();
        if gap <= 0.0 {
            return Ok(f64::INFINITY);
        }
        let ratio = (segment_high - ambient.value()) / gap;
        let log_ratio = ratio.ln();
        sum += cp_at_low * log_ratio
            + slope * ((segment_high - segment_low) - gap * log_ratio);
    }

    let elapsed = config.mass.value() / config.standing_loss_conductance.value() * sum;
    if !elapsed.is_finite() || elapsed < 0.0 {
        return Err(ModelError::NonFinite("reference_cooling_time_s"));
    }
    Ok(elapsed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        HeatCapacityCurve, HeatCapacityPoint, MassKg, PowerW, SpecificHeatJPerKgK,
        StoreConfigSpec, ThermalConductanceWPerK,
    };

    fn k(value: f64) -> Kelvin { Kelvin::new(value).unwrap() }
    fn seconds(value: f64) -> Seconds { Seconds::new(value).unwrap() }

    fn config(cp_low: f64, cp_high: f64, ua: f64, initial: f64) -> StoreConfig {
        let curve = HeatCapacityCurve::new(vec![
            HeatCapacityPoint::new(k(300.0), SpecificHeatJPerKgK::new(cp_low).unwrap()),
            HeatCapacityPoint::new(k(500.0), SpecificHeatJPerKgK::new(cp_high).unwrap()),
        ]).unwrap();
        StoreConfig::new(StoreConfigSpec {
            mass: MassKg::new(10.0).unwrap(),
            heat_capacity: curve,
            min_temperature: k(300.0),
            max_temperature: k(500.0),
            initial_temperature: k(initial),
            max_charge_power: PowerW::new(10_000.0).unwrap(),
            max_discharge_power: PowerW::new(10_000.0).unwrap(),
            standing_loss_conductance: ThermalConductanceWPerK::new(ua).unwrap(),
        }).unwrap()
    }

    #[test]
    fn constant_cp_matches_closed_form_exponential_solution() {
        let cfg = config(1_000.0, 1_000.0, 0.1, 400.0);
        let initial = cfg.initial_state();
        let tau = cfg.mass.value() * 1_000.0 / 0.1;
        let (after, report) = apply_analytic_standing_loss(&cfg, initial, k(300.0), seconds(tau)).unwrap();
        let exact_temperature = 300.0 + 100.0 * (-1.0_f64).exp();
        assert!((after.temperature.value() - exact_temperature).abs() < 1.0e-9);
        assert!((report.heat_lost.value() - (1_000_000.0 * (1.0 - (-1.0_f64).exp()))).abs() < 1.0e-5);
        verify_analytic_standing_loss_report(&cfg, initial, k(300.0), seconds(tau), report).unwrap();
    }

    #[test]
    fn variable_cp_curve_closes_analytic_time_relation_and_energy_ledger() {
        let cfg = config(1_000.0, 2_000.0, 0.1, 400.0);
        let initial = cfg.initial_state();
        let (after, report) = apply_analytic_standing_loss(&cfg, initial, k(300.0), seconds(10_000.0)).unwrap();
        // Independent scalar root of the closed-form elapsed-time equation
        // for cp(T)=5T-500 J/(kg K): T_end=393.48144459132857 K.
        assert!((after.temperature.value() - 393.48144459132857).abs() < 1.0e-8);
        // Integrating cp(T) between T_end and 400 K gives this stored-energy loss.
        assert!((report.heat_lost.value() - 96_716.04201467388).abs() < 1.0e-4);
        assert!(!report.reached_minimum_temperature);
        verify_analytic_standing_loss_report(&cfg, initial, k(300.0), seconds(10_000.0), report).unwrap();
    }

    #[test]
    fn reports_exact_minimum_temperature_event_without_euler_clamp() {
        let cfg = config(1_000.0, 1_000.0, 0.1, 400.0);
        let initial = cfg.initial_state();
        let (after, report) = apply_analytic_standing_loss(&cfg, initial, k(250.0), seconds(500_000.0)).unwrap();
        assert_eq!(after.temperature, k(300.0));
        assert!(report.reached_minimum_temperature);
        assert!((report.heat_lost.value() - 1_000_000.0).abs() < 1.0e-8);
        verify_analytic_standing_loss_report(&cfg, initial, k(250.0), seconds(500_000.0), report).unwrap();
    }

    #[test]
    fn no_ambient_heat_gain_when_ambient_exceeds_store_temperature() {
        let cfg = config(1_000.0, 1_000.0, 0.1, 350.0);
        let initial = cfg.initial_state();
        let (after, report) = apply_analytic_standing_loss(&cfg, initial, k(360.0), seconds(10_000.0)).unwrap();
        assert_eq!(after, initial);
        assert_eq!(report.heat_lost.value(), 0.0);
        verify_analytic_standing_loss_report(&cfg, initial, k(360.0), seconds(10_000.0), report).unwrap();
    }

    #[test]
    fn verifier_rejects_tampered_temperature_even_with_original_input() {
        let cfg = config(1_000.0, 2_000.0, 0.1, 400.0);
        let initial = cfg.initial_state();
        let (_, report) = apply_analytic_standing_loss(&cfg, initial, k(300.0), seconds(10_000.0)).unwrap();
        let tampered = AnalyticStandingLossReport {
            temperature_after: k(399.0),
            ..report
        };
        assert!(matches!(
            verify_analytic_standing_loss_report(&cfg, initial, k(300.0), seconds(10_000.0), tampered),
            Err(ModelError::ReportMismatch(_))
        ));
    }

    #[test]
    fn zero_conductance_preserves_state_exactly() {
        let cfg = config(1_000.0, 1_000.0, 0.0, 400.0);
        let initial = cfg.initial_state();
        let (after, report) = apply_analytic_standing_loss(&cfg, initial, k(300.0), seconds(100_000.0)).unwrap();
        assert_eq!(after, initial);
        assert_eq!(report.heat_lost.value(), 0.0);
        verify_analytic_standing_loss_report(&cfg, initial, k(300.0), seconds(100_000.0), report).unwrap();
    }
    fn multi_segment_config() -> StoreConfig {
        let curve = HeatCapacityCurve::new(vec![
            HeatCapacityPoint::new(k(300.0), SpecificHeatJPerKgK::new(1_000.0).unwrap()),
            HeatCapacityPoint::new(k(350.0), SpecificHeatJPerKgK::new(1_200.0).unwrap()),
            HeatCapacityPoint::new(k(400.0), SpecificHeatJPerKgK::new(1_800.0).unwrap()),
            HeatCapacityPoint::new(k(500.0), SpecificHeatJPerKgK::new(2_000.0).unwrap()),
        ]).unwrap();
        StoreConfig::new(StoreConfigSpec {
            mass: MassKg::new(10.0).unwrap(),
            heat_capacity: curve,
            min_temperature: k(300.0),
            max_temperature: k(500.0),
            initial_temperature: k(480.0),
            max_charge_power: PowerW::new(10_000.0).unwrap(),
            max_discharge_power: PowerW::new(10_000.0).unwrap(),
            standing_loss_conductance: ThermalConductanceWPerK::new(0.1).unwrap(),
        }).unwrap()
    }

    #[test]
    fn analytic_loss_composes_across_time_interval_splits() {
        let cfg = config(1_000.0, 2_000.0, 0.1, 400.0);
        let ambient = k(300.0);
        let initial = cfg.initial_state();
        let (after_first, first_report) =
            apply_analytic_standing_loss(&cfg, initial, ambient, seconds(4_000.0)).unwrap();
        let (after_split, split_report) =
            apply_analytic_standing_loss(&cfg, after_first, ambient, seconds(6_000.0)).unwrap();
        let (after_whole, whole_report) =
            apply_analytic_standing_loss(&cfg, initial, ambient, seconds(10_000.0)).unwrap();

        assert!((after_split.temperature.value() - after_whole.temperature.value()).abs() < 1.0e-9);
        assert!((first_report.heat_lost.value() + split_report.heat_lost.value()
            - whole_report.heat_lost.value()).abs() < 1.0e-6);
        verify_analytic_standing_loss_report(
            &cfg, after_first, ambient, seconds(6_000.0), split_report,
        ).unwrap();
    }

    #[test]
    fn analytic_loss_integrates_across_piecewise_cp_segment_boundaries() {
        let cfg = multi_segment_config();
        let initial = cfg.initial_state();
        let (after, report) =
            apply_analytic_standing_loss(&cfg, initial, k(290.0), seconds(250_000.0)).unwrap();
        assert!(after.temperature.value() < 400.0);
        assert!(after.temperature.value() > 300.0);
        assert!(!report.reached_minimum_temperature);
        verify_analytic_standing_loss_report(
            &cfg, initial, k(290.0), seconds(250_000.0), report,
        ).unwrap();
    }

    #[test]
    fn coarse_euler_step_clamps_while_analytic_solution_preserves_exponential_tail() {
        let cfg = config(1_000.0, 1_000.0, 0.1, 400.0);
        let initial = cfg.initial_state();
        let interval = seconds(100_000.0);
        let (euler_state, euler_report) =
            crate::apply_standing_loss(&cfg, initial, k(300.0), interval).unwrap();
        let (analytic_state, analytic_report) =
            apply_analytic_standing_loss(&cfg, initial, k(300.0), interval).unwrap();

        assert_eq!(euler_state.temperature, k(300.0));
        assert!((analytic_state.temperature.value()
            - (300.0 + 100.0 * (-1.0_f64).exp())).abs() < 1.0e-9);
        assert!(euler_report.heat_lost.value() > analytic_report.heat_lost.value() + 300_000.0);
        assert!(!analytic_report.reached_minimum_temperature);
        verify_analytic_standing_loss_report(
            &cfg, initial, k(300.0), interval, analytic_report,
        ).unwrap();
    }

    #[test]
    fn constant_cp_solution_matches_exponential_across_four_dimensionless_steps() {
        let cfg = config(1_000.0, 1_000.0, 0.1, 400.0);
        let initial = cfg.initial_state();
        let tau = cfg.mass.value() * 1_000.0 / 0.1;
        for ratio in [0.01_f64, 0.1, 1.0, 10.0] {
            let duration = seconds(tau * ratio);
            let (after, report) =
                apply_analytic_standing_loss(&cfg, initial, k(300.0), duration).unwrap();
            let expected_temperature = 300.0 + 100.0 * (-ratio).exp();
            let expected_loss = 1_000_000.0 * (1.0 - (-ratio).exp());
            assert!(
                (after.temperature.value() - expected_temperature).abs() < 1.0e-8,
                "temperature mismatch for dimensionless step {ratio}"
            );
            assert!(
                (report.heat_lost.value() - expected_loss).abs() < 1.0e-4,
                "energy mismatch for dimensionless step {ratio}"
            );
            verify_analytic_standing_loss_report(
                &cfg, initial, k(300.0), duration, report,
            ).unwrap();
        }
    }

}
