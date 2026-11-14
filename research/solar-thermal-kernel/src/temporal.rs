// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Multi-period storage dispatch mechanics for bounded, fixed-grade sensible heat.
//!
//! Each period applies standing loss first, then exactly one explicit action
//! (charge OR discharge). This intentionally excludes simultaneous charging and
//! discharging, direct process-to-process recovery, inter-period forecasting,
//! and optimization. It is an auditable state-transition layer, not a plant model.

use super::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PeriodAction {
    /// The store remains idle except for the period's declared standing loss.
    Idle,
    Charge { available_heat: EnergyJ },
    Discharge { request: DischargeRequest },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TemporalPeriod {
    pub duration: Seconds,
    pub ambient_temperature: Kelvin,
    pub action: PeriodAction,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PeriodActionReport {
    Idle,
    Charge(ChargeReport),
    Discharge(DischargeReport),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TemporalPeriodReport {
    /// Zero-based position in the caller-supplied schedule.
    pub period_index: usize,
    pub state_before: StoreState,
    pub state_after_standing_loss: StoreState,
    pub standing_loss: StandingLossReport,
    pub action: PeriodActionReport,
    pub state_after: StoreState,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TemporalScheduleReport {
    pub periods: Vec<TemporalPeriodReport>,
    pub initial_stored_energy: EnergyJ,
    pub final_stored_energy: EnergyJ,
    pub accepted_charge_heat: EnergyJ,
    pub rejected_charge_heat: EnergyJ,
    pub source_heat_withdrawn: EnergyJ,
    pub useful_heat_delivered: EnergyJ,
    pub heat_exchanger_loss: EnergyJ,
    pub unmet_useful_heat: EnergyJ,
    pub standing_heat_loss: EnergyJ,
    /// Signed first-law residual: initial + accepted charge - losses -
    /// source withdrawal - final stored energy. Values near zero close the ledger.
    pub energy_balance_residual_j: f64,
}

fn duration_matches(period: TemporalPeriod) -> Result<(), ModelError> {
    if let PeriodAction::Discharge { request } = period.action
        && !close(request.duration.value(), period.duration.value())
    {
        return Err(ModelError::PeriodDurationMismatch);
    }
    Ok(())
}

/// Execute a schedule in vector order. Each period applies its standing loss
/// before the period's single dispatch action. The store energy ledger uses the
/// existing cp(T)-integrated sensible-energy state and explicit-Euler loss model.
pub fn simulate_schedule(
    config: &StoreConfig,
    schedule: &[TemporalPeriod],
) -> Result<TemporalScheduleReport, ModelError> {
    if schedule.is_empty() {
        return Err(ModelError::EmptyTemporalSchedule);
    }

    let mut state = config.initial_state();
    let initial_energy = config.stored_energy(state)?.value();
    let mut accepted_charge = 0.0;
    let mut rejected_charge = 0.0;
    let mut source_withdrawn = 0.0;
    let mut useful_delivered = 0.0;
    let mut exchanger_loss = 0.0;
    let mut unmet_useful = 0.0;
    let mut standing_loss_total = 0.0;
    let mut reports = Vec::with_capacity(schedule.len());

    for (index, period) in schedule.iter().copied().enumerate() {
        duration_matches(period)?;
        let state_before = state;
        let (after_loss, loss_report) = apply_standing_loss(
            config,
            state_before,
            period.ambient_temperature,
            period.duration,
        )?;
        standing_loss_total += loss_report.heat_lost.value();

        let (next_state, action_report) = match period.action {
            PeriodAction::Idle => (after_loss, PeriodActionReport::Idle),
            PeriodAction::Charge { available_heat } => {
                let (next, report) = charge_store(config, after_loss, available_heat, period.duration)?;
                accepted_charge += report.accepted_input.value();
                rejected_charge += report.rejected_input.value();
                (next, PeriodActionReport::Charge(report))
            }
            PeriodAction::Discharge { request } => {
                let (next, report) = discharge_to_load(config, after_loss, request)?;
                source_withdrawn += report.source_heat_withdrawn.value();
                useful_delivered += report.useful_heat_delivered.value();
                exchanger_loss += report.heat_exchanger_loss.value();
                unmet_useful += report.unmet_useful_heat.value();
                (next, PeriodActionReport::Discharge(report))
            }
        };

        reports.push(TemporalPeriodReport {
            period_index: index,
            state_before,
            state_after_standing_loss: after_loss,
            standing_loss: loss_report,
            action: action_report,
            state_after: next_state,
        });
        state = next_state;
    }

    let final_energy = config.stored_energy(state)?.value();
    let residual = initial_energy + accepted_charge - standing_loss_total
        - source_withdrawn - final_energy;

    Ok(TemporalScheduleReport {
        periods: reports,
        initial_stored_energy: EnergyJ::new(initial_energy)?,
        final_stored_energy: EnergyJ::new(final_energy)?,
        accepted_charge_heat: EnergyJ::new(accepted_charge)?,
        rejected_charge_heat: EnergyJ::new(rejected_charge)?,
        source_heat_withdrawn: EnergyJ::new(source_withdrawn)?,
        useful_heat_delivered: EnergyJ::new(useful_delivered)?,
        heat_exchanger_loss: EnergyJ::new(exchanger_loss)?,
        unmet_useful_heat: EnergyJ::new(unmet_useful)?,
        standing_heat_loss: EnergyJ::new(standing_loss_total)?,
        energy_balance_residual_j: residual,
    })
}

/// Independently recompute period state transitions and the aggregate energy
/// ledger. This verifier deliberately does not call simulate_schedule(),
/// charge_store(), apply_standing_loss(), or discharge_to_load().
pub fn verify_schedule_report(
    config: &StoreConfig,
    schedule: &[TemporalPeriod],
    claimed: &TemporalScheduleReport,
) -> Result<(), ModelError> {
    if schedule.is_empty() {
        return Err(ModelError::EmptyTemporalSchedule);
    }
    if claimed.periods.len() != schedule.len() {
        return Err(ModelError::ReportMismatch("temporal.period_count"));
    }

    let mut state = config.initial_state();
    let initial_energy = stored_energy_at(config, state.temperature)?.value();
    let capacity = stored_energy_at(config, config.max_temperature)?.value();
    let mut accepted_charge = 0.0;
    let mut rejected_charge = 0.0;
    let mut source_withdrawn = 0.0;
    let mut useful_delivered = 0.0;
    let mut exchanger_loss = 0.0;
    let mut unmet_useful = 0.0;
    let mut standing_loss_total = 0.0;

    for (index, (period, report)) in schedule.iter().copied()
        .zip(claimed.periods.iter().copied()).enumerate()
    {
        duration_matches(period)?;
        if report.period_index != index {
            return Err(ModelError::ReportMismatch("temporal.period_index"));
        }
        check("temporal.state_before", report.state_before.temperature.value(),
            state.temperature.value())?;

        // Recompute the loss equation directly instead of trusting the
        // standing-loss operation/report helpers.
        let before_energy = stored_energy_at(config, state.temperature)?.value();
        let temperature_delta = (state.temperature.value()
            - period.ambient_temperature.value()).max(0.0);
        let expected_loss = (config.standing_loss_conductance.value()
            * temperature_delta * period.duration.value()).min(before_energy);
        let post_loss_energy = (before_energy - expected_loss).max(0.0);
        let post_loss_temperature = temperature_at_energy(
            config, EnergyJ::new(post_loss_energy)?,
        )?;
        check("temporal.loss.temperature_before",
            report.standing_loss.temperature_before.value(), state.temperature.value())?;
        check("temporal.loss.temperature_after",
            report.standing_loss.temperature_after.value(), post_loss_temperature.value())?;
        check("temporal.loss.ambient",
            report.standing_loss.ambient_temperature.value(), period.ambient_temperature.value())?;
        check("temporal.loss.energy", report.standing_loss.heat_lost.value(), expected_loss)?;
        check("temporal.loss.balance", before_energy,
            stored_energy_at(config, report.standing_loss.temperature_after)?.value()
                + report.standing_loss.heat_lost.value())?;
        check("temporal.state_after_loss", report.state_after_standing_loss.temperature.value(),
            post_loss_temperature.value())?;
        standing_loss_total += expected_loss;
        let post_loss_state = StoreState { temperature: post_loss_temperature };

        // Recompute the action equations directly. Reusing the public operation
        // functions here would make this verifier circular.
        let expected_final_temperature = match (period.action, report.action) {
            (PeriodAction::Idle, PeriodActionReport::Idle) => post_loss_temperature,
            (PeriodAction::Charge { available_heat }, PeriodActionReport::Charge(action)) => {
                let store_energy = stored_energy_at(config, post_loss_temperature)?.value();
                let room = (capacity - store_energy).max(0.0);
                let accepted = available_heat.value().min(
                    config.max_charge_power.value() * period.duration.value()
                ).min(room);
                let rejected = (available_heat.value() - accepted).max(0.0);
                let target = temperature_at_energy(
                    config, EnergyJ::new(store_energy + accepted)?,
                )?;
                check("temporal.charge.temperature_before",
                    action.temperature_before.value(), post_loss_temperature.value())?;
                check("temporal.charge.temperature_after", action.temperature_after.value(),
                    target.value())?;
                check("temporal.charge.requested", action.requested_input.value(),
                    available_heat.value())?;
                check("temporal.charge.accepted", action.accepted_input.value(), accepted)?;
                check("temporal.charge.rejected", action.rejected_input.value(), rejected)?;
                check("temporal.charge.store_balance", store_energy + accepted,
                    stored_energy_at(config, action.temperature_after)?.value())?;
                check("temporal.charge.input_balance", accepted + rejected,
                    available_heat.value())?;
                accepted_charge += accepted;
                rejected_charge += rejected;
                target
            }
            (PeriodAction::Discharge { request }, PeriodActionReport::Discharge(action)) => {
                if request.heat_exchanger_effectiveness.value() <= 0.0 {
                    return Err(ModelError::InvalidDischargeEfficiency);
                }
                let supply_cutoff = request.required_supply_temperature.value()
                    + request.minimum_approach.value();
                let minimum_source = Kelvin::new(
                    supply_cutoff.max(config.min_temperature.value())
                )?;
                let store_energy = stored_energy_at(config, post_loss_temperature)?.value();
                let blocked = request.requested_useful_heat.value() > 0.0
                    && post_loss_temperature.value() <= supply_cutoff;
                let (withdrawn, delivered, exchanger, unmet, target) =
                    if blocked || request.requested_useful_heat.value() == 0.0 {
                        (0.0, 0.0, 0.0, request.requested_useful_heat.value(),
                            post_loss_temperature)
                    } else {
                        let floor_energy = stored_energy_at(config, minimum_source)?.value();
                        let grade_limited = (store_energy - floor_energy).max(0.0);
                        let power_limited = config.max_discharge_power.value()
                            * period.duration.value();
                        let effectiveness = request.heat_exchanger_effectiveness.value();
                        if effectiveness <= 0.0 {
                            return Err(ModelError::InvalidDischargeEfficiency);
                        }
                        let requested_source = request.requested_useful_heat.value() / effectiveness;
                        let source = requested_source.min(grade_limited).min(power_limited);
                        let delivered = (source * effectiveness)
                            .min(request.requested_useful_heat.value());
                        let exchanger_loss = (source - delivered).max(0.0);
                        let unmet = (request.requested_useful_heat.value() - delivered).max(0.0);
                        let target = temperature_at_energy(
                            config, EnergyJ::new((store_energy - source).max(0.0))?,
                        )?;
                        (source, delivered, exchanger_loss, unmet, target)
                    };
                check("temporal.discharge.temperature_before",
                    action.temperature_before.value(), post_loss_temperature.value())?;
                check("temporal.discharge.temperature_after",
                    action.temperature_after.value(), target.value())?;
                check("temporal.discharge.request",
                    action.requested_useful_heat.value(), request.requested_useful_heat.value())?;
                check("temporal.discharge.withdrawn",
                    action.source_heat_withdrawn.value(), withdrawn)?;
                check("temporal.discharge.delivered",
                    action.useful_heat_delivered.value(), delivered)?;
                check("temporal.discharge.exchanger_loss",
                    action.heat_exchanger_loss.value(), exchanger)?;
                check("temporal.discharge.unmet",
                    action.unmet_useful_heat.value(), unmet)?;
                check("temporal.discharge.cutoff",
                    action.minimum_source_temperature.value(), minimum_source.value())?;
                if action.blocked_by_temperature != blocked {
                    return Err(ModelError::ReportMismatch("temporal.discharge.temperature_gate"));
                }
                check("temporal.discharge.store_balance", store_energy,
                    stored_energy_at(config, action.temperature_after)?.value() + withdrawn)?;
                check("temporal.discharge.exchanger_balance", withdrawn,
                    delivered + exchanger)?;
                check("temporal.discharge.service_balance",
                    request.requested_useful_heat.value(), delivered + unmet)?;
                source_withdrawn += withdrawn;
                useful_delivered += delivered;
                exchanger_loss += exchanger;
                unmet_useful += unmet;
                target
            }
            _ => return Err(ModelError::ReportMismatch("temporal.action_kind")),
        };

        check("temporal.state_after_action", report.state_after.temperature.value(),
            expected_final_temperature.value())?;
        // Explicitly verify this period's store energy transition, including loss.
        let after_energy = stored_energy_at(config, expected_final_temperature)?.value();
        let before_period_energy = stored_energy_at(config, state.temperature)?.value();
        let expected_action_energy_change = match period.action {
            PeriodAction::Charge { .. } => {
                let action = match report.action {
                    PeriodActionReport::Charge(a) => a,
                    PeriodActionReport::Discharge(_) | PeriodActionReport::Idle =>
                        return Err(ModelError::ReportMismatch("temporal.action_kind")),
                };
                action.accepted_input.value()
            }
            PeriodAction::Idle => match report.action {
                PeriodActionReport::Idle => 0.0,
                _ => return Err(ModelError::ReportMismatch("temporal.action_kind")),
            },
            PeriodAction::Discharge { .. } => {
                let action = match report.action {
                    PeriodActionReport::Discharge(a) => a,
                    PeriodActionReport::Charge(_) | PeriodActionReport::Idle =>
                        return Err(ModelError::ReportMismatch("temporal.action_kind")),
                };
                -action.source_heat_withdrawn.value()
            }
        };
        check("temporal.period_first_law", before_period_energy
            - expected_loss + expected_action_energy_change, after_energy)?;
        state = StoreState { temperature: expected_final_temperature };
        check("temporal.period_state_after", report.state_after.temperature.value(),
            state.temperature.value())?;
        // Keep a direct use of post_loss_state to make the boundary explicit and
        // catch an accidentally substituted state in future revisions.
        check("temporal.post_loss_boundary", post_loss_state.temperature.value(),
            post_loss_temperature.value())?;
    }

    let final_energy = stored_energy_at(config, state.temperature)?.value();
    let residual = initial_energy + accepted_charge - standing_loss_total
        - source_withdrawn - final_energy;
    check("temporal.initial_energy", claimed.initial_stored_energy.value(), initial_energy)?;
    check("temporal.final_energy", claimed.final_stored_energy.value(), final_energy)?;
    check("temporal.accepted_charge", claimed.accepted_charge_heat.value(), accepted_charge)?;
    check("temporal.rejected_charge", claimed.rejected_charge_heat.value(), rejected_charge)?;
    check("temporal.source_withdrawn", claimed.source_heat_withdrawn.value(), source_withdrawn)?;
    check("temporal.useful_delivered", claimed.useful_heat_delivered.value(), useful_delivered)?;
    check("temporal.exchanger_loss", claimed.heat_exchanger_loss.value(), exchanger_loss)?;
    check("temporal.unmet", claimed.unmet_useful_heat.value(), unmet_useful)?;
    check("temporal.standing_loss", claimed.standing_heat_loss.value(), standing_loss_total)?;
    check("temporal.balance_residual", claimed.energy_balance_residual_j, residual)?;
    check("temporal.global_energy_closure", initial_energy + accepted_charge,
        final_energy + standing_loss_total + source_withdrawn)?;
    check("temporal.delivered_service_closure",
        schedule.iter().map(|p| match p.action {
            PeriodAction::Discharge { request } => request.requested_useful_heat.value(),
            PeriodAction::Charge { .. } | PeriodAction::Idle => 0.0,
        }).sum(),
        useful_delivered + unmet_useful)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn k(value: f64) -> Kelvin { Kelvin::new(value).unwrap() }
    fn e(value: f64) -> EnergyJ { EnergyJ::new(value).unwrap() }
    fn seconds(value: f64) -> Seconds { Seconds::new(value).unwrap() }

    fn fixture_config() -> StoreConfig {
        let curve = HeatCapacityCurve::new(vec![
            HeatCapacityPoint::new(k(300.0), SpecificHeatJPerKgK::new(1_000.0).unwrap()),
            HeatCapacityPoint::new(k(400.0), SpecificHeatJPerKgK::new(1_000.0).unwrap()),
        ]).unwrap();
        StoreConfig::new(StoreConfigSpec {
            mass: MassKg::new(10.0).unwrap(),
            heat_capacity: curve,
            min_temperature: k(300.0),
            max_temperature: k(400.0),
            initial_temperature: k(300.0),
            max_charge_power: PowerW::new(5_000.0).unwrap(),
            max_discharge_power: PowerW::new(1_000.0).unwrap(),
            standing_loss_conductance: ThermalConductanceWPerK::new(0.1).unwrap(),
        }).unwrap()
    }

    fn canonical_day_night_schedule() -> Vec<TemporalPeriod> {
        vec![
            TemporalPeriod {
                duration: seconds(3_600.0),
                ambient_temperature: k(300.0),
                action: PeriodAction::Charge { available_heat: e(900_000.0) },
            },
            TemporalPeriod {
                duration: seconds(3_600.0),
                ambient_temperature: k(300.0),
                action: PeriodAction::Discharge { request: DischargeRequest {
                    requested_useful_heat: e(300_000.0),
                    required_supply_temperature: k(340.0),
                    minimum_approach: TemperatureDifferenceK::new(10.0).unwrap(),
                    heat_exchanger_effectiveness: Fraction::new(0.9).unwrap(),
                    duration: seconds(3_600.0),
                }},
            },
        ]
    }

    #[test]
    fn analytic_day_night_storage_closes_energy_and_meets_service() {
        let config = fixture_config();
        let schedule = canonical_day_night_schedule();
        let report = simulate_schedule(&config, &schedule).unwrap();
        assert!((report.accepted_charge_heat.value() - 900_000.0).abs() < 1.0e-6);
        assert!((report.standing_heat_loss.value() - 32_400.0).abs() < 1.0e-6);
        assert!((report.useful_heat_delivered.value() - 300_000.0).abs() < 1.0e-6);
        assert!(report.unmet_useful_heat.value() < 1.0e-6);
        assert!((report.heat_exchanger_loss.value() - (300_000.0 / 0.9 - 300_000.0)).abs() < 1.0e-6);
        assert!(report.final_stored_energy.value() > 0.0);
        assert!(report.energy_balance_residual_j.abs() < 1.0e-6);
        verify_schedule_report(&config, &schedule, &report).unwrap();
    }

    #[test]
    fn idle_period_applies_losses_without_fabricating_charge_or_service() {
        let config = fixture_config();
        let mut schedule = canonical_day_night_schedule();
        schedule.push(TemporalPeriod {
            duration: seconds(3_600.0),
            ambient_temperature: k(300.0),
            action: PeriodAction::Idle,
        });
        let report = simulate_schedule(&config, &schedule).unwrap();
        assert_eq!(report.periods.len(), 3);
        assert_eq!(report.periods[2].action, PeriodActionReport::Idle);
        assert!((report.periods[2].standing_loss.heat_lost.value() - 19_233.6).abs() < 1.0e-6);
        assert_eq!(
            report.periods[2].state_after,
            report.periods[2].state_after_standing_loss
        );
        assert!((report.accepted_charge_heat.value() - 900_000.0).abs() < 1.0e-6);
        assert!((report.useful_heat_delivered.value() - 300_000.0).abs() < 1.0e-6);
        assert!(report.unmet_useful_heat.value() < 1.0e-6);
        assert!(report.energy_balance_residual_j.abs() < 1.0e-6);
        verify_schedule_report(&config, &schedule, &report).unwrap();
    }

    #[test]
    fn independent_verifier_rejects_tampered_aggregate_loss() {
        let config = fixture_config();
        let schedule = canonical_day_night_schedule();
        let report = simulate_schedule(&config, &schedule).unwrap();
        let tampered = TemporalScheduleReport {
            standing_heat_loss: e(1.0),
            ..report
        };
        assert!(matches!(
            verify_schedule_report(&config, &schedule, &tampered),
            Err(ModelError::ReportMismatch(_))
        ));
    }

    #[test]
    fn independent_verifier_rejects_tampered_inter_period_state() {
        let config = fixture_config();
        let schedule = canonical_day_night_schedule();
        let report = simulate_schedule(&config, &schedule).unwrap();
        let mut periods = report.periods.clone();
        periods[1].state_before = StoreState { temperature: k(380.0) };
        let tampered = TemporalScheduleReport { periods, ..report };
        assert!(matches!(
            verify_schedule_report(&config, &schedule, &tampered),
            Err(ModelError::ReportMismatch(_))
        ));
    }

    #[test]
    fn rejects_discharge_power_budget_using_a_different_duration() {
        let config = fixture_config();
        let mut schedule = canonical_day_night_schedule();
        let request = match schedule[1].action {
            PeriodAction::Discharge { request } => request,
            PeriodAction::Charge { .. } | PeriodAction::Idle => unreachable!(),
        };
        schedule[1].action = PeriodAction::Discharge {
            request: DischargeRequest { duration: seconds(7_200.0), ..request },
        };
        assert_eq!(
            simulate_schedule(&config, &schedule),
            Err(ModelError::PeriodDurationMismatch)
        );
    }

    #[test]
    fn multi_period_schedule_closes_ledger_across_capacity_grade_and_power_limits() {
        let config = fixture_config();
        let schedule = [
            TemporalPeriod {
                duration: seconds(3_600.0),
                ambient_temperature: k(300.0),
                action: PeriodAction::Charge { available_heat: e(900_000.0) },
            },
            TemporalPeriod {
                duration: seconds(3_600.0),
                ambient_temperature: k(300.0),
                action: PeriodAction::Discharge { request: DischargeRequest {
                    requested_useful_heat: e(100_000.0),
                    required_supply_temperature: k(320.0),
                    minimum_approach: TemperatureDifferenceK::new(10.0).unwrap(),
                    heat_exchanger_effectiveness: Fraction::new(0.8).unwrap(),
                    duration: seconds(3_600.0),
                }},
            },
            TemporalPeriod {
                duration: seconds(300.0),
                ambient_temperature: k(300.0),
                action: PeriodAction::Charge { available_heat: e(500_000.0) },
            },
            TemporalPeriod {
                duration: seconds(7_200.0),
                ambient_temperature: k(290.0),
                action: PeriodAction::Discharge { request: DischargeRequest {
                    requested_useful_heat: e(500_000.0),
                    required_supply_temperature: k(360.0),
                    minimum_approach: TemperatureDifferenceK::new(10.0).unwrap(),
                    heat_exchanger_effectiveness: Fraction::new(1.0).unwrap(),
                    duration: seconds(7_200.0),
                }},
            },
        ];
        let report = simulate_schedule(&config, &schedule).unwrap();
        assert_eq!(report.periods.len(), 4);
        assert!((report.accepted_charge_heat.value() - 1_159_627.8).abs() < 1.0e-6);
        assert!((report.rejected_charge_heat.value() - 240_372.2).abs() < 1.0e-6);
        assert!((report.standing_heat_loss.value() - 113_827.8).abs() < 1.0e-6);
        assert!((report.source_heat_withdrawn.value() - 345_800.0).abs() < 1.0e-6);
        assert!((report.useful_heat_delivered.value() - 320_800.0).abs() < 1.0e-6);
        assert!((report.unmet_useful_heat.value() - 279_200.0).abs() < 1.0e-6);
        assert!((report.final_stored_energy.value() - 700_000.0).abs() < 1.0e-6);
        assert!((report.periods[3].state_after.temperature.value() - 370.0).abs() < 1.0e-8);
        assert!(report.energy_balance_residual_j.abs() < 1.0e-6);
        for pair in report.periods.windows(2) {
            assert_eq!(pair[0].state_after, pair[1].state_before);
        }
        verify_schedule_report(&config, &schedule, &report).unwrap();
    }

    #[test]
    fn rejects_empty_schedule() {
        assert_eq!(
            simulate_schedule(&fixture_config(), &[]),
            Err(ModelError::EmptyTemporalSchedule)
        );
    }

    #[test]
    fn charge_period_obeys_store_capacity_and_reports_spill() {
        let config = fixture_config();
        let schedule = [TemporalPeriod {
            duration: seconds(3_600.0),
            ambient_temperature: k(300.0),
            action: PeriodAction::Charge { available_heat: e(1_200_000.0) },
        }];
        let report = simulate_schedule(&config, &schedule).unwrap();
        assert!((report.accepted_charge_heat.value() - 1_000_000.0).abs() < 1.0e-6);
        assert!((report.rejected_charge_heat.value() - 200_000.0).abs() < 1.0e-6);
        verify_schedule_report(&config, &schedule, &report).unwrap();
    }

    #[test]
    fn grade_limited_night_load_is_reported_as_unmet_not_invented_heat() {
        let config = fixture_config();
        let schedule = [
            TemporalPeriod {
                duration: seconds(3_600.0),
                ambient_temperature: k(300.0),
                action: PeriodAction::Charge { available_heat: e(400_000.0) },
            },
            TemporalPeriod {
                duration: seconds(3_600.0),
                ambient_temperature: k(300.0),
                action: PeriodAction::Discharge { request: DischargeRequest {
                    requested_useful_heat: e(300_000.0),
                    required_supply_temperature: k(340.0),
                    minimum_approach: TemperatureDifferenceK::new(10.0).unwrap(),
                    heat_exchanger_effectiveness: Fraction::new(0.9).unwrap(),
                    duration: seconds(3_600.0),
                }},
            },
        ];
        let report = simulate_schedule(&config, &schedule).unwrap();
        assert!(report.unmet_useful_heat.value() > 0.0);
        verify_schedule_report(&config, &schedule, &report).unwrap();
    }
}
