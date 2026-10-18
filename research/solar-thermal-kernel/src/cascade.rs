// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Two-store, two-temperature sensible-heat cascade transfer.
//!
//! This is a bounded transfer primitive, not a distributed heat-exchanger model.
//! It conservatively requires the hot store to remain above the low store's
//! configured maximum temperature plus the requested approach margin.

use super::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CascadeTransferRequest {
    /// Desired heat accepted by the low-temperature store, not source-side heat.
    pub requested_low_store_heat: EnergyJ,
    pub minimum_approach: TemperatureDifferenceK,
    pub heat_exchanger_effectiveness: Fraction,
    pub duration: Seconds,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CascadeTransferReport {
    pub high_temperature_before: Kelvin,
    pub high_temperature_after: Kelvin,
    pub low_temperature_before: Kelvin,
    pub low_temperature_after: Kelvin,
    pub requested_low_store_heat: EnergyJ,
    pub source_heat_withdrawn: EnergyJ,
    pub low_store_heat_accepted: EnergyJ,
    pub heat_exchanger_loss: EnergyJ,
    pub unmet_low_store_heat: EnergyJ,
    pub minimum_source_temperature: Kelvin,
    pub blocked_by_temperature: bool,
}

fn expected_transfer(
    high_config: &StoreConfig,
    high_state: StoreState,
    low_config: &StoreConfig,
    low_state: StoreState,
    request: CascadeTransferRequest,
) -> Result<CascadeTransferReport, ModelError> {
    if request.heat_exchanger_effectiveness.value() <= 0.0 {
        return Err(ModelError::InvalidDischargeEfficiency);
    }

    let high_before = stored_energy_at(high_config, high_state.temperature)?.value();
    let low_before = stored_energy_at(low_config, low_state.temperature)?.value();
    let low_capacity = stored_energy_at(low_config, low_config.max_temperature)?.value();
    let approach = request.minimum_approach.value();
    let source_cutoff = Kelvin::new(
        (low_config.max_temperature.value() + approach)
            .max(high_config.min_temperature.value()),
    )?;

    let blocked = request.requested_low_store_heat.value() > 0.0
        && (source_cutoff.value() > high_config.max_temperature.value()
            || high_state.temperature.value() <= source_cutoff.value());

    let (withdrawn, accepted, loss, high_after, low_after) =
        if blocked || request.requested_low_store_heat.value() == 0.0 {
            (0.0, 0.0, 0.0, high_state.temperature, low_state.temperature)
        } else {
            let cutoff_energy = stored_energy_at(high_config, source_cutoff)?.value();
            let available_above_grade = (high_before - cutoff_energy).max(0.0);
            let rate_limited_source = high_config.max_discharge_power.value()
                * request.duration.value();
            let source_available = available_above_grade.min(rate_limited_source);
            let low_room = (low_capacity - low_before).max(0.0);
            let accepted = request
                .requested_low_store_heat
                .value()
                .min(low_room)
                .min(source_available * request.heat_exchanger_effectiveness.value());
            let withdrawn = accepted / request.heat_exchanger_effectiveness.value();
            let loss = (withdrawn - accepted).max(0.0);
            let high_after = temperature_at_energy(
                high_config,
                EnergyJ::new((high_before - withdrawn).max(0.0))?,
            )?;
            let low_after = temperature_at_energy(
                low_config,
                EnergyJ::new((low_before + accepted).min(low_capacity))?,
            )?;
            (withdrawn, accepted, loss, high_after, low_after)
        };

    Ok(CascadeTransferReport {
        high_temperature_before: high_state.temperature,
        high_temperature_after: high_after,
        low_temperature_before: low_state.temperature,
        low_temperature_after: low_after,
        requested_low_store_heat: request.requested_low_store_heat,
        source_heat_withdrawn: EnergyJ::new(withdrawn)?,
        low_store_heat_accepted: EnergyJ::new(accepted)?,
        heat_exchanger_loss: EnergyJ::new(loss)?,
        unmet_low_store_heat: EnergyJ::new(
            (request.requested_low_store_heat.value() - accepted).max(0.0),
        )?,
        minimum_source_temperature: source_cutoff,
        blocked_by_temperature: blocked,
    })
}

/// Transfer heat from a higher-temperature store to a lower-temperature store.
///
/// The source store cannot cool below the low store's maximum temperature plus
/// the approach margin. The low store is limited by its own capacity; the source
/// is limited by energy above the temperature floor and by discharge power times
/// duration. Heat-exchanger loss equals source heat withdrawn minus heat accepted
/// by the low store. Unmet requested transfer is reported separately from loss.
pub fn transfer_to_lower_temperature_store(
    high_config: &StoreConfig,
    high_state: StoreState,
    low_config: &StoreConfig,
    low_state: StoreState,
    request: CascadeTransferRequest,
) -> Result<(StoreState, StoreState, CascadeTransferReport), ModelError> {
    let report = expected_transfer(
        high_config,
        high_state,
        low_config,
        low_state,
        request,
    )?;
    Ok((
        StoreState { temperature: report.high_temperature_after },
        StoreState { temperature: report.low_temperature_after },
        report,
    ))
}

/// Independent recomputation of the cascade transfer contract.
pub fn verify_cascade_transfer_report(
    high_config: &StoreConfig,
    high_state: StoreState,
    low_config: &StoreConfig,
    low_state: StoreState,
    request: CascadeTransferRequest,
    claimed: CascadeTransferReport,
) -> Result<(), ModelError> {
    if request.heat_exchanger_effectiveness.value() <= 0.0 {
        return Err(ModelError::InvalidDischargeEfficiency);
    }

    // Deliberately re-derive the claimed transition here instead of calling
    // expected_transfer, so this checker is a separate calculation path.
    let high_start_energy = stored_energy_at(high_config, high_state.temperature)?.value();
    let low_start_energy = stored_energy_at(low_config, low_state.temperature)?.value();
    let low_capacity = stored_energy_at(low_config, low_config.max_temperature)?.value();
    let cutoff_value = (low_config.max_temperature.value()
        + request.minimum_approach.value())
        .max(high_config.min_temperature.value());
    let cutoff = Kelvin::new(cutoff_value)?;
    let grade_blocked = request.requested_low_store_heat.value() > 0.0
        && (cutoff_value > high_config.max_temperature.value()
            || high_state.temperature.value() <= cutoff_value);

    let (expected_withdrawn, expected_accepted, expected_loss, expected_unmet,
        expected_high_temperature, expected_low_temperature) =
        if grade_blocked || request.requested_low_store_heat.value() == 0.0 {
            (0.0, 0.0, 0.0, request.requested_low_store_heat.value(),
                high_state.temperature, low_state.temperature)
        } else {
            let energy_at_cutoff = stored_energy_at(high_config, cutoff)?.value();
            let thermal_headroom = (high_start_energy - energy_at_cutoff).max(0.0);
            let source_power_budget = high_config.max_discharge_power.value()
                * request.duration.value();
            let low_store_room = (low_capacity - low_start_energy).max(0.0);
            let accepted = request.requested_low_store_heat.value()
                .min(low_store_room)
                .min(thermal_headroom.min(source_power_budget)
                    * request.heat_exchanger_effectiveness.value());
            let withdrawn = accepted / request.heat_exchanger_effectiveness.value();
            let loss = withdrawn - accepted;
            let high_energy_after = EnergyJ::new((high_start_energy - withdrawn).max(0.0))?;
            let low_energy_after = EnergyJ::new((low_start_energy + accepted).min(low_capacity))?;
            (
                withdrawn,
                accepted,
                loss.max(0.0),
                (request.requested_low_store_heat.value() - accepted).max(0.0),
                temperature_at_energy(high_config, high_energy_after)?,
                temperature_at_energy(low_config, low_energy_after)?,
            )
        };

    let fields = [
        ("cascade.high_before", claimed.high_temperature_before.value(), high_state.temperature.value()),
        ("cascade.high_after", claimed.high_temperature_after.value(), expected_high_temperature.value()),
        ("cascade.low_before", claimed.low_temperature_before.value(), low_state.temperature.value()),
        ("cascade.low_after", claimed.low_temperature_after.value(), expected_low_temperature.value()),
        ("cascade.requested", claimed.requested_low_store_heat.value(), request.requested_low_store_heat.value()),
        ("cascade.withdrawn", claimed.source_heat_withdrawn.value(), expected_withdrawn),
        ("cascade.accepted", claimed.low_store_heat_accepted.value(), expected_accepted),
        ("cascade.loss", claimed.heat_exchanger_loss.value(), expected_loss),
        ("cascade.unmet", claimed.unmet_low_store_heat.value(), expected_unmet),
        ("cascade.source_cutoff", claimed.minimum_source_temperature.value(), cutoff.value()),
    ];
    for (label, actual, expected) in fields {
        check(label, actual, expected)?;
    }
    if claimed.blocked_by_temperature != grade_blocked {
        return Err(ModelError::ReportMismatch("cascade.blocked_by_temperature"));
    }

    let high_final_energy = stored_energy_at(high_config, claimed.high_temperature_after)?.value();
    let low_final_energy = stored_energy_at(low_config, claimed.low_temperature_after)?.value();
    check(
        "cascade.total_store_energy_closure",
        high_start_energy + low_start_energy,
        high_final_energy + low_final_energy + claimed.heat_exchanger_loss.value(),
    )?;
    check(
        "cascade.requested_heat_split",
        claimed.requested_low_store_heat.value(),
        claimed.low_store_heat_accepted.value() + claimed.unmet_low_store_heat.value(),
    )?;
    check(
        "cascade.source_heat_split",
        claimed.source_heat_withdrawn.value(),
        claimed.low_store_heat_accepted.value() + claimed.heat_exchanger_loss.value(),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn k(value: f64) -> Kelvin { Kelvin::new(value).unwrap() }
    fn e(value: f64) -> EnergyJ { EnergyJ::new(value).unwrap() }
    fn seconds(value: f64) -> Seconds { Seconds::new(value).unwrap() }

    fn config(
        min_temperature: f64,
        max_temperature: f64,
        initial_temperature: f64,
        max_discharge_power: f64,
    ) -> StoreConfig {
        let curve = HeatCapacityCurve::new(vec![
            HeatCapacityPoint::new(k(min_temperature), SpecificHeatJPerKgK::new(1_000.0).unwrap()),
            HeatCapacityPoint::new(k(max_temperature), SpecificHeatJPerKgK::new(1_000.0).unwrap()),
        ]).unwrap();
        StoreConfig::new(StoreConfigSpec {
            mass: MassKg::new(1.0).unwrap(),
            heat_capacity: curve,
            min_temperature: k(min_temperature),
            max_temperature: k(max_temperature),
            initial_temperature: k(initial_temperature),
            max_charge_power: PowerW::new(10_000.0).unwrap(),
            max_discharge_power: PowerW::new(max_discharge_power).unwrap(),
            standing_loss_conductance: ThermalConductanceWPerK::new(0.0).unwrap(),
        }).unwrap()
    }

    fn request(heat: f64, approach: f64, effectiveness: f64, duration: f64) -> CascadeTransferRequest {
        CascadeTransferRequest {
            requested_low_store_heat: e(heat),
            minimum_approach: TemperatureDifferenceK::new(approach).unwrap(),
            heat_exchanger_effectiveness: Fraction::new(effectiveness).unwrap(),
            duration: seconds(duration),
        }
    }

    #[test]
    fn hand_computable_transfer_closes_energy_and_respects_grade() {
        let high_config = config(300.0, 500.0, 450.0, 10_000.0);
        let low_config = config(300.0, 360.0, 300.0, 10_000.0);
        let high_start = high_config.initial_state();
        let low_start = low_config.initial_state();
        let req = request(40_000.0, 10.0, 0.8, 10.0);

        let (high_end, low_end, report) = transfer_to_lower_temperature_store(
            &high_config, high_start, &low_config, low_start, req,
        ).unwrap();

        assert!((report.source_heat_withdrawn.value() - 50_000.0).abs() < 1e-8);
        assert!((report.low_store_heat_accepted.value() - 40_000.0).abs() < 1e-8);
        assert!((report.heat_exchanger_loss.value() - 10_000.0).abs() < 1e-8);
        assert!((high_end.temperature.value() - 400.0).abs() < 1e-8);
        assert!((low_end.temperature.value() - 340.0).abs() < 1e-8);
        assert!(high_end.temperature.value() >= report.minimum_source_temperature.value());
        verify_cascade_transfer_report(
            &high_config, high_start, &low_config, low_start, req, report,
        ).unwrap();
    }

    #[test]
    fn low_store_capacity_limits_transfer_without_overfill() {
        let high_config = config(300.0, 500.0, 450.0, 10_000.0);
        let low_config = config(300.0, 360.0, 340.0, 10_000.0);
        let low_start = low_config.initial_state();
        let req = request(50_000.0, 10.0, 0.8, 10.0);
        let (_, low_end, report) = transfer_to_lower_temperature_store(
            &high_config, high_config.initial_state(), &low_config, low_start, req,
        ).unwrap();

        assert!((report.low_store_heat_accepted.value() - 20_000.0).abs() < 1e-8);
        assert!((report.unmet_low_store_heat.value() - 30_000.0).abs() < 1e-8);
        assert!((low_end.temperature.value() - 360.0).abs() < 1e-8);
        verify_cascade_transfer_report(
            &high_config, high_config.initial_state(), &low_config, low_start, req, report,
        ).unwrap();
    }

    #[test]
    fn source_power_limits_transfer() {
        let high_config = config(300.0, 500.0, 450.0, 1_000.0);
        let low_config = config(300.0, 360.0, 300.0, 10_000.0);
        let req = request(50_000.0, 10.0, 0.8, 10.0);
        let (_, _, report) = transfer_to_lower_temperature_store(
            &high_config, high_config.initial_state(), &low_config, low_config.initial_state(), req,
        ).unwrap();

        assert!((report.source_heat_withdrawn.value() - 10_000.0).abs() < 1e-8);
        assert!((report.low_store_heat_accepted.value() - 8_000.0).abs() < 1e-8);
        assert!((report.unmet_low_store_heat.value() - 42_000.0).abs() < 1e-8);
    }

    #[test]
    fn source_cannot_cross_low_store_grade_plus_approach() {
        let high_config = config(300.0, 500.0, 380.0, 10_000.0);
        let low_config = config(300.0, 360.0, 300.0, 10_000.0);
        let (high_end, _, report) = transfer_to_lower_temperature_store(
            &high_config, high_config.initial_state(), &low_config, low_config.initial_state(),
            request(50_000.0, 10.0, 0.8, 10.0),
        ).unwrap();

        assert!((report.source_heat_withdrawn.value() - 10_000.0).abs() < 1e-8);
        assert!((high_end.temperature.value() - 370.0).abs() < 1e-8);
        assert!(!report.blocked_by_temperature);
    }

    #[test]
    fn blocks_transfer_when_hot_store_cannot_meet_approach() {
        let high_config = config(300.0, 500.0, 365.0, 10_000.0);
        let low_config = config(300.0, 360.0, 300.0, 10_000.0);
        let high_start = high_config.initial_state();
        let low_start = low_config.initial_state();
        let req = request(10_000.0, 10.0, 0.8, 10.0);
        let (high_end, low_end, report) = transfer_to_lower_temperature_store(
            &high_config, high_start, &low_config, low_start, req,
        ).unwrap();

        assert!(report.blocked_by_temperature);
        assert_eq!(high_end, high_start);
        assert_eq!(low_end, low_start);
        assert_eq!(report.source_heat_withdrawn.value(), 0.0);
        assert_eq!(report.unmet_low_store_heat.value(), 10_000.0);
        verify_cascade_transfer_report(
            &high_config, high_start, &low_config, low_start, req, report,
        ).unwrap();
    }

    #[test]
    fn verifier_rejects_tampered_cascade_report() {
        let high_config = config(300.0, 500.0, 450.0, 10_000.0);
        let low_config = config(300.0, 360.0, 300.0, 10_000.0);
        let high_start = high_config.initial_state();
        let low_start = low_config.initial_state();
        let req = request(40_000.0, 10.0, 0.8, 10.0);
        let (_, _, report) = transfer_to_lower_temperature_store(
            &high_config, high_start, &low_config, low_start, req,
        ).unwrap();
        let tampered = CascadeTransferReport {
            low_store_heat_accepted: e(41_000.0),
            ..report
        };

        assert!(matches!(
            verify_cascade_transfer_report(
                &high_config, high_start, &low_config, low_start, req, tampered,
            ),
            Err(ModelError::ReportMismatch(_))
        ));
    }

    #[test]
    fn rejects_zero_heat_exchanger_effectiveness() {
        let high_config = config(300.0, 500.0, 450.0, 10_000.0);
        let low_config = config(300.0, 360.0, 300.0, 10_000.0);
        let req = request(10_000.0, 10.0, 0.0, 10.0);
        assert_eq!(
            transfer_to_lower_temperature_store(
                &high_config, high_config.initial_state(), &low_config,
                low_config.initial_state(), req,
            ),
            Err(ModelError::InvalidDischargeEfficiency)
        );
    }
}
