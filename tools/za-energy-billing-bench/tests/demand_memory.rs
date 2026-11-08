// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Small exact demand-memory counterexample for CEA-DEMAND-MEMORY-001.
//!
//! This is a synthetic two-month decision problem. The billing engine is the
//! independent tariff-component reference; the action cost and controllable
//! kVA reduction are declared synthetic inputs, not a battery model.

use za_energy_billing_bench::{
    Account, CITY_POWER_UTILITY_ID, DayClass, DemandHistoryPoint, MeteredInterval,
    SOURCE_PROFILE_ID, TariffInput, VoltageClass, calculate_bill,
};

const MONTHLY_SHAVE_KVA: f64 = 10.0;
const SYNTHETIC_SHAVE_COST_ZAR: f64 = 1_000.0;
const LV_DEMAND_RATE_ZAR_PER_KVA: f64 = 461.28;

fn days_in_month(year: i32, month: u8) -> u8 {
    match month {
        2 if year % 400 == 0 || (year % 4 == 0 && year % 100 != 0) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// Returns Monday=0 through Sunday=6 using the Gregorian calendar.
fn weekday_monday_zero(year: i32, month: u8, day: u8) -> u8 {
    let mut y = year;
    let mut m = i32::from(month);
    if m < 3 {
        y -= 1;
        m += 12;
    }
    let year_of_century = y % 100;
    let zero_based_century = y / 100;
    let zeller = (i32::from(day)
        + (13 * (m + 1)) / 5
        + year_of_century
        + year_of_century / 4
        + zero_based_century / 4
        + 5 * zero_based_century)
        % 7;
    match zeller {
        0 => 5,                  // Saturday
        1 => 6,                  // Sunday
        _ => (zeller - 2) as u8, // Monday through Friday
    }
}

fn month_index(year: i32, month: u8) -> i32 {
    year * 12 + i32::from(month) - 1
}

fn month_from_index(index: i32) -> String {
    let year = index.div_euclid(12);
    let month = index.rem_euclid(12) + 1;
    format!("{year:04}-{month:02}")
}

/// Synthetic 12-month history for July 2026:
/// July 2025=250 kVA, August 2025=240 kVA, and ten months at 140 kVA.
/// The 80%-of-top-three ratchet is 168 kVA.
fn initial_history() -> Vec<DemandHistoryPoint> {
    let current = month_index(2026, 7);
    (0..12)
        .map(|offset| DemandHistoryPoint {
            billing_month: month_from_index(current - 12 + offset),
            measured_peak_kva: match offset {
                0 => 250.0,
                1 => 240.0,
                _ => 140.0,
            },
        })
        .collect()
}

/// Move one billing month forward: discard the oldest month and append the
/// just-completed month's measured peak. No predicted or estimated values are
/// silently substituted.
fn roll_history(
    history: &[DemandHistoryPoint],
    completed_month: &str,
    completed_peak_kva: f64,
) -> Vec<DemandHistoryPoint> {
    assert_eq!(history.len(), 12);
    let mut next = history[1..].to_vec();
    next.push(DemandHistoryPoint {
        billing_month: completed_month.to_owned(),
        measured_peak_kva: completed_peak_kva,
    });
    next
}

fn monthly_input(
    billing_month: &str,
    measured_peak_kva: f64,
    history: &[DemandHistoryPoint],
) -> TariffInput {
    let (year_text, month_text) = billing_month.split_once('-').unwrap();
    let year: i32 = year_text.parse().unwrap();
    let month: u8 = month_text.parse().unwrap();
    let days = days_in_month(year, month);

    // One-hour synthetic intervals are contiguous and align with all published
    // City Power tariff boundaries. Each Saturday 18:00-20:00 interval carries
    // an explicit synthetic confirmation because the budget prints "20h0".
    let mut intervals = Vec::with_capacity(usize::from(days) * 24);
    let mut declared_active_energy_kwh = 0.0;

    for day in 1..=days {
        let local_date = format!("{year:04}-{month:02}-{day:02}");
        let day_class = match weekday_monday_zero(year, month, day) {
            5 => DayClass::SaturdayOrPublicHoliday,
            6 => DayClass::Sunday,
            _ => DayClass::Weekday,
        };

        for hour in 0..24_u16 {
            let saturday_evening = day_class == DayClass::SaturdayOrPublicHoliday
                && (18..20).contains(&hour);
            let energy_kwh = 1.0;
            declared_active_energy_kwh += energy_kwh;
            intervals.push(MeteredInterval {
                local_date: local_date.clone(),
                day_class,
                local_start_minute: hour * 60,
                duration_minutes: 60,
                energy_kwh,
                saturday_evening_confirmed: saturday_evening,
                saturday_confirmation_reference: saturday_evening
                    .then(|| "SYNTHETIC-SATURDAY-ENDPOINT-CONFIRMATION".to_owned()),
                public_holiday_evidence_reference: None,
            });
        }
    }

    TariffInput {
        source_profile_id: SOURCE_PROFILE_ID.to_owned(),
        profile_version: "0.1".to_owned(),
        billing_month: billing_month.to_owned(),
        time_zone: "Africa/Johannesburg".to_owned(),
        account: Account {
            utility_id: CITY_POWER_UTILITY_ID.to_owned(),
            account_reference: "SYNTHETIC-DEMAND-MEMORY-ACCOUNT".to_owned(),
            supply_capacity_kva: 150.0,
            voltage_class: VoltageClass::Low,
            industrial_tou_approved: true,
            approval_reference: Some("SYNTHETIC-TOU-APPROVAL".to_owned()),
            legacy_pre_1999_demand_tariff: false,
            service_connection_classification_reference: Some(
                "SYNTHETIC-LV-LPU-SERVICE-CLASS".to_owned(),
            ),
        },
        intervals,
        interval_series_complete: true,
        interval_coverage_evidence_reference: format!(
            "SYNTHETIC-COMPLETE-{billing_month}-HOURLY-INTERVAL-TRACE"
        ),
        declared_active_energy_kwh,
        measured_monthly_peak_demand_kva: measured_peak_kva,
        preceding_12_month_demand_history: history.to_vec(),
        power_factor: 1.0,
        measured_reactive_energy_kvarh: 0.0,
        network_surcharge_applies: Some(false),
        network_surcharge_evidence_reference: Some(
            "SYNTHETIC-ACCOUNT-EXEMPT-NETWORK-SURCHARGE".to_owned(),
        ),
        business_lpu_surcharge: None,
        vat: None,
        calendar_mapping_reference: "SYNTHETIC-GREGORIAN-CALENDAR".to_owned(),
        timebase_reference: "AFRICA-JOHANNESBURG-SAST".to_owned(),
        meter_boundary_reference: "SYNTHETIC-PCC-DEMAND-METER".to_owned(),
        meter_calibration_reference: "SYNTHETIC-METER-CALIBRATION".to_owned(),
        input_manifest_reference: "SYNTHETIC-DEMAND-MEMORY-MANIFEST".to_owned(),
    }
}

fn close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1e-7,
        "actual={actual:.10}, expected={expected:.10}"
    );
}

#[test]
fn shaving_july_peak_does_not_change_july_bill_when_historical_ratchet_binds() {
    let history = initial_history();
    let baseline = calculate_bill(&monthly_input("2026-07", 160.0, &history)).unwrap();
    let shaved = calculate_bill(&monthly_input(
        "2026-07",
        160.0 - MONTHLY_SHAVE_KVA,
        &history,
    ))
    .unwrap();

    close(baseline.breakdown.billed_demand_kva, 168.0);
    close(shaved.breakdown.billed_demand_kva, 168.0);
    close(
        baseline.breakdown.demand_charge_zar,
        168.0 * LV_DEMAND_RATE_ZAR_PER_KVA,
    );
    close(
        shaved.breakdown.demand_charge_zar,
        baseline.breakdown.demand_charge_zar,
    );

    // The synthetic input intentionally omits assessed VAT / Business-LPU
    // amounts, so it cannot be mistaken for a complete customer invoice.
    assert!(baseline.breakdown.invoice_total_zar.is_none());
    assert!(shaved.breakdown.invoice_total_zar.is_none());
}

#[test]
fn july_peak_is_retained_as_august_billing_memory_and_changes_the_ratchet() {
    let history = initial_history();

    let july_unshaved = 160.0;
    let july_shaved = july_unshaved - MONTHLY_SHAVE_KVA;
    let august_history_unshaved = roll_history(&history, "2026-07", july_unshaved);
    let august_history_shaved = roll_history(&history, "2026-07", july_shaved);

    let august_unshaved = calculate_bill(&monthly_input(
        "2026-08",
        135.0,
        &august_history_unshaved,
    ))
    .unwrap();
    let august_shaved = calculate_bill(&monthly_input(
        "2026-08",
        135.0,
        &august_history_shaved,
    ))
    .unwrap();

    // August's preceding-year history has rolled out July 2025's 250 kVA
    // peak. The top three become [240,160,140] vs [240,150,140].
    close(august_unshaved.breakdown.billed_demand_kva, 144.0);
    close(
        august_shaved.breakdown.billed_demand_kva,
        0.8 * (240.0 + 150.0 + 140.0) / 3.0,
    );
    close(
        august_unshaved.breakdown.billed_demand_kva
            - august_shaved.breakdown.billed_demand_kva,
        8.0 / 3.0,
    );
}

#[test]
fn two_month_exact_oracle_beats_a_current_month_only_policy() {
    let history = initial_history();

    let july_without_shave =
        calculate_bill(&monthly_input("2026-07", 160.0, &history)).unwrap();
    let july_with_shave = calculate_bill(&monthly_input(
        "2026-07",
        160.0 - MONTHLY_SHAVE_KVA,
        &history,
    ))
    .unwrap();

    let august_history_without_shave = roll_history(&history, "2026-07", 160.0);
    let august_history_with_shave =
        roll_history(&history, "2026-07", 160.0 - MONTHLY_SHAVE_KVA);

    let august_without_shave = calculate_bill(&monthly_input(
        "2026-08",
        135.0,
        &august_history_without_shave,
    ))
    .unwrap();
    let august_after_july_shave = calculate_bill(&monthly_input(
        "2026-08",
        135.0,
        &august_history_with_shave,
    ))
    .unwrap();

    // A current-month-only decision sees identical July demand charges and
    // chooses no shave because it incurs the synthetic R1,000 action cost.
    let myopic_no_action_cost = july_without_shave.breakdown.demand_charge_zar
        + august_without_shave.breakdown.demand_charge_zar;
    let myopic_shave_cost = july_with_shave.breakdown.demand_charge_zar
        + august_without_shave.breakdown.demand_charge_zar
        + SYNTHETIC_SHAVE_COST_ZAR;
    assert!(myopic_no_action_cost < myopic_shave_cost);

    // Exact two-month enumeration evaluates both possible July actions. The
    // shave does not lower July's billed kVA, but it lowers August's ratchet.
    let oracle_no_shave = july_without_shave.breakdown.demand_charge_zar
        + august_without_shave.breakdown.demand_charge_zar;
    let oracle_shave = july_with_shave.breakdown.demand_charge_zar
        + august_after_july_shave.breakdown.demand_charge_zar
        + SYNTHETIC_SHAVE_COST_ZAR;
    assert!(oracle_shave < oracle_no_shave);

    close(
        august_without_shave.breakdown.demand_charge_zar
            - august_after_july_shave.breakdown.demand_charge_zar,
        1_230.08,
    );
    close(oracle_no_shave - oracle_shave, 230.08);
}
