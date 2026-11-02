//! Deterministic time-resolved real-power adequacy reference evaluator.
//!
//! This is a simple synthetic baseline, not a grid/power-flow or nuclear model.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadInterval {
    /// Time basis is an integer campaign timeline, in seconds.
    pub start_tick: u64,
    pub duration_seconds: u64,
    pub critical_real_power_w: u64,
    pub flexible_real_power_w: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupplyInterval {
    pub start_tick: u64,
    pub duration_seconds: u64,
    /// Firm real-power capacity made available to this interval.
    pub firm_real_power_w: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BatterySpec {
    /// Maximum AC real-power discharge.
    pub discharge_power_w: u64,
    /// Stored usable energy at the start of the campaign, in watt-seconds.
    pub initial_stored_energy_ws: u128,
    pub usable_capacity_ws: u128,
    /// Energy that must remain stored after every discharge.
    pub minimum_reserve_ws: u128,
    /// 1..=1000 represents 0.1%..=100% discharge efficiency.
    pub discharge_efficiency_permille: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerError {
    EmptyCampaign,
    IntervalCountMismatch,
    ZeroDuration,
    NonContiguousLoadTimeline,
    SupplyIntervalMisaligned,
    LoadPowerOverflow,
    InvalidBatteryEfficiency,
    BatteryInitialEnergyExceedsCapacity,
    BatteryReserveExceedsInitialEnergy,
    ArithmeticOverflow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IntervalResult {
    pub start_tick: u64,
    pub duration_seconds: u64,
    pub total_load_energy_ws: u128,
    pub firm_supply_energy_ws: u128,
    pub battery_delivered_energy_ws: u128,
    pub critical_unserved_energy_ws: u128,
    pub flexible_unserved_energy_ws: u128,
    pub stored_battery_energy_after_ws: u128,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CampaignResult {
    pub intervals: Vec<IntervalResult>,
    pub total_load_energy_ws: u128,
    pub total_firm_supply_energy_ws: u128,
    pub total_battery_delivered_energy_ws: u128,
    pub total_critical_unserved_energy_ws: u128,
    pub total_flexible_unserved_energy_ws: u128,
    /// Conservative upper bound: sums whole intervals containing any unserved critical energy.
    /// Exact within-interval outage timing is not modeled.
    pub critical_unserved_interval_seconds: u64,
    pub final_battery_energy_ws: u128,
}

fn interval_energy_ws(power_w: u64, duration_seconds: u64) -> Result<u128, PowerError> {
    u128::from(power_w)
        .checked_mul(u128::from(duration_seconds))
        .ok_or(PowerError::ArithmeticOverflow)
}

fn add_total(total: &mut u128, value: u128) -> Result<(), PowerError> {
    *total = total
        .checked_add(value)
        .ok_or(PowerError::ArithmeticOverflow)?;
    Ok(())
}

/// Run a simple critical-first firm-supply + greedy-battery adequacy baseline.
///
/// Firm supply serves critical loads first, then flexible loads. A battery covers
/// the remaining deficit subject to both inverter power and remaining stored energy
/// above reserve. No future intervals are consulted.
#[must_use]
pub fn evaluate_campaign(
    loads: &[LoadInterval],
    supplies: &[SupplyInterval],
    battery: Option<BatterySpec>,
) -> Result<CampaignResult, PowerError> {
    if loads.is_empty() {
        return Err(PowerError::EmptyCampaign);
    }
    if loads.len() != supplies.len() {
        return Err(PowerError::IntervalCountMismatch);
    }

    let mut battery_energy = if let Some(spec) = battery {
        if spec.discharge_efficiency_permille == 0 || spec.discharge_efficiency_permille > 1_000 {
            return Err(PowerError::InvalidBatteryEfficiency);
        }
        if spec.initial_stored_energy_ws > spec.usable_capacity_ws {
            return Err(PowerError::BatteryInitialEnergyExceedsCapacity);
        }
        if spec.minimum_reserve_ws > spec.initial_stored_energy_ws {
            return Err(PowerError::BatteryReserveExceedsInitialEnergy);
        }
        spec.initial_stored_energy_ws
    } else {
        0
    };

    let mut expected_start = loads[0].start_tick;
    let mut intervals = Vec::with_capacity(loads.len());
    let mut total_load_energy_ws = 0_u128;
    let mut total_firm_supply_energy_ws = 0_u128;
    let mut total_battery_delivered_energy_ws = 0_u128;
    let mut total_critical_unserved_energy_ws = 0_u128;
    let mut total_flexible_unserved_energy_ws = 0_u128;
    let mut critical_unserved_interval_seconds = 0_u64;

    for (load, supply) in loads.iter().zip(supplies) {
        if load.duration_seconds == 0 || supply.duration_seconds == 0 {
            return Err(PowerError::ZeroDuration);
        }
        if load.start_tick != expected_start {
            return Err(PowerError::NonContiguousLoadTimeline);
        }
        if supply.start_tick != load.start_tick || supply.duration_seconds != load.duration_seconds
        {
            return Err(PowerError::SupplyIntervalMisaligned);
        }
        expected_start = expected_start
            .checked_add(load.duration_seconds)
            .ok_or(PowerError::ArithmeticOverflow)?;

        let total_load_w = load
            .critical_real_power_w
            .checked_add(load.flexible_real_power_w)
            .ok_or(PowerError::LoadPowerOverflow)?;
        let total_load_ws = interval_energy_ws(total_load_w, load.duration_seconds)?;

        // Allocate the firm source to critical demand before flexible demand.
        let firm_to_critical_w = load.critical_real_power_w.min(supply.firm_real_power_w);
        let firm_remaining_w = supply.firm_real_power_w - firm_to_critical_w;
        let firm_to_flexible_w = load.flexible_real_power_w.min(firm_remaining_w);
        let critical_deficit_w = load.critical_real_power_w - firm_to_critical_w;
        let flexible_deficit_w = load.flexible_real_power_w - firm_to_flexible_w;
        let total_deficit_w = critical_deficit_w
            .checked_add(flexible_deficit_w)
            .ok_or(PowerError::ArithmeticOverflow)?;

        let firm_to_load_w = firm_to_critical_w
            .checked_add(firm_to_flexible_w)
            .ok_or(PowerError::ArithmeticOverflow)?;
        let firm_energy_ws = interval_energy_ws(firm_to_load_w, load.duration_seconds)?;

        let mut battery_delivered_ws = 0_u128;
        if let Some(spec) = battery {
            let energy_above_reserve_ws = battery_energy.saturating_sub(spec.minimum_reserve_ws);
            let max_ac_energy_ws = energy_above_reserve_ws
                .checked_mul(u128::from(spec.discharge_efficiency_permille))
                .ok_or(PowerError::ArithmeticOverflow)?
                / 1_000;
            let max_power_energy_ws =
                interval_energy_ws(spec.discharge_power_w, load.duration_seconds)?;
            let deficit_energy_ws = interval_energy_ws(total_deficit_w, load.duration_seconds)?;
            battery_delivered_ws = deficit_energy_ws
                .min(max_ac_energy_ws)
                .min(max_power_energy_ws);

            // Round cell-side energy consumption upward: never overstate remaining SOC.
            let numerator = battery_delivered_ws
                .checked_mul(1_000)
                .ok_or(PowerError::ArithmeticOverflow)?;
            let efficiency = u128::from(spec.discharge_efficiency_permille);
            let cell_energy_used_ws = numerator
                .checked_add(efficiency - 1)
                .ok_or(PowerError::ArithmeticOverflow)?
                / efficiency;
            battery_energy = battery_energy
                .checked_sub(cell_energy_used_ws)
                .ok_or(PowerError::ArithmeticOverflow)?;
        }

        // Convert the battery's delivered watt-seconds back into the interval's
        // critical-first allocation, preserving integer conservation exactly.
        let critical_deficit_ws = interval_energy_ws(critical_deficit_w, load.duration_seconds)?;
        let battery_to_critical_ws = battery_delivered_ws.min(critical_deficit_ws);
        let battery_to_flexible_ws = battery_delivered_ws - battery_to_critical_ws;
        let critical_unserved_ws = critical_deficit_ws - battery_to_critical_ws;
        let flexible_deficit_ws = interval_energy_ws(flexible_deficit_w, load.duration_seconds)?;
        let flexible_unserved_ws = flexible_deficit_ws - battery_to_flexible_ws;

        if critical_unserved_ws > 0 {
            critical_unserved_interval_seconds = critical_unserved_interval_seconds
                .checked_add(load.duration_seconds)
                .ok_or(PowerError::ArithmeticOverflow)?;
        }

        add_total(&mut total_load_energy_ws, total_load_ws)?;
        add_total(&mut total_firm_supply_energy_ws, firm_energy_ws)?;
        add_total(&mut total_battery_delivered_energy_ws, battery_delivered_ws)?;
        add_total(&mut total_critical_unserved_energy_ws, critical_unserved_ws)?;
        add_total(&mut total_flexible_unserved_energy_ws, flexible_unserved_ws)?;

        intervals.push(IntervalResult {
            start_tick: load.start_tick,
            duration_seconds: load.duration_seconds,
            total_load_energy_ws: total_load_ws,
            firm_supply_energy_ws: firm_energy_ws,
            battery_delivered_energy_ws: battery_delivered_ws,
            critical_unserved_energy_ws: critical_unserved_ws,
            flexible_unserved_energy_ws: flexible_unserved_ws,
            stored_battery_energy_after_ws: battery_energy,
        });
    }

    Ok(CampaignResult {
        intervals,
        total_load_energy_ws,
        total_firm_supply_energy_ws,
        total_battery_delivered_energy_ws,
        total_critical_unserved_energy_ws,
        total_flexible_unserved_energy_ws,
        critical_unserved_interval_seconds,
        final_battery_energy_ws: battery_energy,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load(start: u64, duration: u64, critical: u64, flexible: u64) -> LoadInterval {
        LoadInterval {
            start_tick: start,
            duration_seconds: duration,
            critical_real_power_w: critical,
            flexible_real_power_w: flexible,
        }
    }

    fn supply(start: u64, duration: u64, firm: u64) -> SupplyInterval {
        SupplyInterval {
            start_tick: start,
            duration_seconds: duration,
            firm_real_power_w: firm,
        }
    }

    fn battery(power: u64, stored_ws: u128, reserve_ws: u128, efficiency: u16) -> BatterySpec {
        BatterySpec {
            discharge_power_w: power,
            initial_stored_energy_ws: stored_ws,
            usable_capacity_ws: stored_ws,
            minimum_reserve_ws: reserve_ws,
            discharge_efficiency_permille: efficiency,
        }
    }

    #[test]
    fn exact_wh_energy_integral_for_one_kw_one_hour() {
        let result = evaluate_campaign(
            &[load(0, 3_600, 1_000, 0)],
            &[supply(0, 3_600, 1_000)],
            None,
        )
        .unwrap();
        assert_eq!(result.total_load_energy_ws, 3_600_000);
        assert_eq!(result.total_firm_supply_energy_ws, 3_600_000);
        assert_eq!(result.total_critical_unserved_energy_ws, 0);
    }

    #[test]
    fn firm_supply_exactly_meets_load() {
        let result =
            evaluate_campaign(&[load(0, 60, 800, 200)], &[supply(0, 60, 1_000)], None).unwrap();
        assert_eq!(result.total_critical_unserved_energy_ws, 0);
        assert_eq!(result.total_flexible_unserved_energy_ws, 0);
    }

    #[test]
    fn critical_load_is_served_before_flexible_load() {
        let result =
            evaluate_campaign(&[load(0, 60, 800, 600)], &[supply(0, 60, 1_000)], None).unwrap();
        assert_eq!(result.total_critical_unserved_energy_ws, 0);
        assert_eq!(result.total_flexible_unserved_energy_ws, 24_000);
        assert_eq!(result.critical_unserved_interval_seconds, 0);
    }

    #[test]
    fn firm_supply_shortfall_on_critical_load_is_reported() {
        let result =
            evaluate_campaign(&[load(0, 60, 1_000, 0)], &[supply(0, 60, 700)], None).unwrap();
        assert_eq!(result.total_critical_unserved_energy_ws, 18_000);
        assert_eq!(result.critical_unserved_interval_seconds, 60);
    }

    #[test]
    fn battery_covers_shortfall_when_power_and_energy_are_sufficient() {
        let result = evaluate_campaign(
            &[load(0, 3_600, 1_500, 0)],
            &[supply(0, 3_600, 1_000)],
            Some(battery(500, 2_000_000, 0, 1_000)),
        )
        .unwrap();
        assert_eq!(result.total_battery_delivered_energy_ws, 1_800_000);
        assert_eq!(result.total_critical_unserved_energy_ws, 0);
        assert_eq!(result.final_battery_energy_ws, 200_000);
    }

    #[test]
    fn battery_power_limit_can_leave_unserved_critical_load() {
        let result = evaluate_campaign(
            &[load(0, 60, 2_000, 0)],
            &[supply(0, 60, 1_000)],
            Some(battery(500, 1_000_000, 0, 1_000)),
        )
        .unwrap();
        assert_eq!(result.total_battery_delivered_energy_ws, 30_000);
        assert_eq!(result.total_critical_unserved_energy_ws, 30_000);
    }

    #[test]
    fn battery_energy_limit_can_leave_unserved_load_even_with_enough_kw() {
        let result = evaluate_campaign(
            &[load(0, 3_600, 2_000, 0)],
            &[supply(0, 3_600, 1_000)],
            Some(battery(2_000, 1_000_000, 0, 1_000)),
        )
        .unwrap();
        assert_eq!(result.total_battery_delivered_energy_ws, 1_000_000);
        assert_eq!(result.total_critical_unserved_energy_ws, 2_600_000);
    }

    #[test]
    fn battery_reserve_is_not_spent_to_preserve_production() {
        let result = evaluate_campaign(
            &[load(0, 3_600, 1_500, 0)],
            &[supply(0, 3_600, 1_000)],
            Some(battery(500, 2_000_000, 1_000_000, 1_000)),
        )
        .unwrap();
        assert_eq!(result.total_battery_delivered_energy_ws, 1_000_000);
        assert_eq!(result.final_battery_energy_ws, 1_000_000);
        assert_eq!(result.total_critical_unserved_energy_ws, 800_000);
    }

    #[test]
    fn discharge_efficiency_reduces_cell_side_energy() {
        let result = evaluate_campaign(
            &[load(0, 1_000, 1_000, 0)],
            &[supply(0, 1_000, 500)],
            Some(battery(500, 1_000_000, 0, 500)),
        )
        .unwrap();
        assert_eq!(result.total_battery_delivered_energy_ws, 500_000);
        assert_eq!(result.final_battery_energy_ws, 0);
    }

    #[test]
    fn microreactor_20mwe_does_not_cover_22mw_tb_m_full_nameplate_case() {
        let result = evaluate_campaign(
            &[load(0, 3_600, 22_000_000, 0)],
            &[supply(0, 3_600, 20_000_000)],
            None,
        )
        .unwrap();
        assert_eq!(result.total_critical_unserved_energy_ws, 7_200_000_000);
    }

    #[test]
    fn microreactor_20mwe_does_not_cover_44_18mw_dredger_full_nameplate_case() {
        let result = evaluate_campaign(
            &[load(0, 3_600, 44_180_000, 0)],
            &[supply(0, 3_600, 20_000_000)],
            None,
        )
        .unwrap();
        assert_eq!(result.total_critical_unserved_energy_ws, 87_048_000_000);
    }

    #[test]
    fn auxiliaries_can_make_a_nominally_feasible_prime_mover_infeasible() {
        let result = evaluate_campaign(
            &[load(0, 3_600, 18_000_000, 3_000_000)],
            &[supply(0, 3_600, 20_000_000)],
            None,
        )
        .unwrap();
        assert_eq!(result.total_critical_unserved_energy_ws, 0);
        assert_eq!(result.total_flexible_unserved_energy_ws, 3_600_000_000);
    }

    #[test]
    fn interval_energy_accounting_reconciles_exactly() {
        let result = evaluate_campaign(
            &[load(0, 60, 1_200, 800)],
            &[supply(0, 60, 1_000)],
            Some(battery(500, 30_000, 0, 1_000)),
        )
        .unwrap();
        let interval = result.intervals[0];
        assert_eq!(
            interval.total_load_energy_ws,
            interval.firm_supply_energy_ws
                + interval.battery_delivered_energy_ws
                + interval.critical_unserved_energy_ws
                + interval.flexible_unserved_energy_ws
        );
    }

    #[test]
    fn unserved_interval_metric_is_conservative_for_partial_battery_depletion() {
        let result = evaluate_campaign(
            &[load(0, 3_600, 1_500, 0)],
            &[supply(0, 3_600, 1_000)],
            Some(battery(500, 1_000_000, 0, 1_000)),
        )
        .unwrap();
        assert!(result.total_critical_unserved_energy_ws > 0);
        // The baseline reports the full interval because it does not model the
        // exact point in the interval at which stored energy is exhausted.
        assert_eq!(result.critical_unserved_interval_seconds, 3_600);
    }

    #[test]
    fn grid_outage_is_visible_as_time_varying_firm_capacity() {
        let result = evaluate_campaign(
            &[load(0, 60, 1_000, 0), load(60, 60, 1_000, 0)],
            &[supply(0, 60, 1_000), supply(60, 60, 0)],
            None,
        )
        .unwrap();
        assert_eq!(result.total_critical_unserved_energy_ws, 60_000);
        assert_eq!(result.critical_unserved_interval_seconds, 60);
    }

    #[test]
    fn supply_derating_reduces_adequacy() {
        let result =
            evaluate_campaign(&[load(0, 60, 1_000, 0)], &[supply(0, 60, 800)], None).unwrap();
        assert_eq!(result.total_critical_unserved_energy_ws, 12_000);
    }

    #[test]
    fn zero_duration_is_rejected() {
        assert_eq!(
            evaluate_campaign(&[load(0, 0, 1, 0)], &[supply(0, 0, 1)], None),
            Err(PowerError::ZeroDuration)
        );
    }

    #[test]
    fn noncontiguous_load_timeline_is_rejected() {
        assert_eq!(
            evaluate_campaign(
                &[load(0, 60, 1, 0), load(61, 60, 1, 0)],
                &[supply(0, 60, 1), supply(61, 60, 1)],
                None,
            ),
            Err(PowerError::NonContiguousLoadTimeline)
        );
    }

    #[test]
    fn misaligned_supply_interval_is_rejected() {
        assert_eq!(
            evaluate_campaign(&[load(0, 60, 1, 0)], &[supply(1, 60, 1)], None,),
            Err(PowerError::SupplyIntervalMisaligned)
        );
    }

    #[test]
    fn empty_schedule_is_rejected() {
        assert_eq!(
            evaluate_campaign(&[], &[], None),
            Err(PowerError::EmptyCampaign)
        );
    }

    #[test]
    fn interval_count_mismatch_is_rejected() {
        assert_eq!(
            evaluate_campaign(&[load(0, 60, 1, 0)], &[], None),
            Err(PowerError::IntervalCountMismatch)
        );
    }

    #[test]
    fn invalid_battery_efficiency_is_rejected() {
        assert_eq!(
            evaluate_campaign(
                &[load(0, 60, 1, 0)],
                &[supply(0, 60, 0)],
                Some(battery(1, 10, 0, 0)),
            ),
            Err(PowerError::InvalidBatteryEfficiency)
        );
    }

    #[test]
    fn battery_efficiency_above_one_thousand_permille_is_rejected() {
        assert_eq!(
            evaluate_campaign(
                &[load(0, 60, 1, 0)],
                &[supply(0, 60, 0)],
                Some(battery(1, 10, 0, 1_001)),
            ),
            Err(PowerError::InvalidBatteryEfficiency)
        );
    }

    #[test]
    fn initial_battery_energy_above_capacity_is_rejected() {
        let mut b = battery(10, 100, 0, 1_000);
        b.usable_capacity_ws = 99;
        assert_eq!(
            evaluate_campaign(&[load(0, 60, 1, 0)], &[supply(0, 60, 0)], Some(b)),
            Err(PowerError::BatteryInitialEnergyExceedsCapacity)
        );
    }

    #[test]
    fn battery_reserve_above_initial_energy_is_rejected() {
        assert_eq!(
            evaluate_campaign(
                &[load(0, 60, 1, 0)],
                &[supply(0, 60, 0)],
                Some(battery(10, 100, 101, 1_000)),
            ),
            Err(PowerError::BatteryReserveExceedsInitialEnergy)
        );
    }

    #[test]
    fn load_power_overflow_is_rejected() {
        assert_eq!(
            evaluate_campaign(&[load(0, 60, u64::MAX, 1)], &[supply(0, 60, 0)], None,),
            Err(PowerError::LoadPowerOverflow)
        );
    }

    #[test]
    fn identical_interval_inputs_replay_deterministically() {
        let loads = [load(0, 60, 500, 500), load(60, 60, 700, 300)];
        let supplies = [supply(0, 60, 800), supply(60, 60, 900)];
        let b = Some(battery(200, 100_000, 0, 950));
        assert_eq!(
            evaluate_campaign(&loads, &supplies, b),
            evaluate_campaign(&loads, &supplies, b)
        );
    }
}
