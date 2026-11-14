use std::process::ExitCode;

use za_energy_billing_bench::{
    Account, AssessedInvoiceLine, CITY_POWER_UTILITY_ID, DayClass, DemandHistoryPoint,
    MeteredInterval, SOURCE_PROFILE_ID, TariffInput, VoltageClass, calculate_bill,
};

fn day_class_for_july_2026(day: u8) -> DayClass {
    // 1 July 2026 is Wednesday. Monday=0, Saturday=5, Sunday=6.
    match (day + 1) % 7 {
        5 => DayClass::SaturdayOrPublicHoliday,
        6 => DayClass::Sunday,
        _ => DayClass::Weekday,
    }
}

fn days_in_month(year: i32, month: u8) -> u8 {
    match month {
        2 if year % 400 == 0 || (year % 4 == 0 && year % 100 != 0) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

fn month_index(year: i32, month: u8) -> i32 {
    year * 12 + i32::from(month) - 1
}

fn demand_history_for(billing_month: &str, demand_kva: f64) -> Vec<DemandHistoryPoint> {
    let (year_text, month_text) = billing_month.split_once('-').expect("YYYY-MM");
    let year: i32 = year_text.parse().expect("valid year");
    let month: u8 = month_text.parse().expect("valid month");
    let current = month_index(year, month);
    let mut history = Vec::new();

    for month_offset in 0..12 {
        let absolute = current - 12 + month_offset;
        let history_year = absolute.div_euclid(12);
        let history_month = (absolute.rem_euclid(12) + 1) as u8;
        for day in 1..=days_in_month(history_year, history_month) {
            let local_date = format!("{history_year:04}-{history_month:02}-{day:02}");
            for local_start_minute in (0..1_440_u16).step_by(30) {
                history.push(DemandHistoryPoint {
                    local_date: local_date.clone(),
                    local_start_minute,
                    measured_demand_kva: demand_kva,
                });
            }
        }
    }
    history
}

fn demo_input() -> TariffInput {
    let preceding_12_month_demand_history = demand_history_for("2026-07", 125.0);

    // Synthetic 31-day, 15-minute complete month. The weekday lighting load
    // is placed at the winter peak periods; this is a reference fixture, not
    // a crop schedule or a measured facility profile.
    let mut intervals = Vec::with_capacity(31 * 96);
    let mut declared_active_energy_kwh = 0.0;

    for day in 1..=31_u8 {
        let local_date = format!("2026-07-{day:02}");
        let day_class = day_class_for_july_2026(day);

        for slot in 0..96_u16 {
            let local_start_minute = slot * 15;
            let mut energy_kwh = 1.0;
            if day_class == DayClass::Weekday
                && (local_start_minute == 420 || local_start_minute == 1_080)
            {
                energy_kwh += 5.0;
            }

            // The ambiguous source-printed interval is explicitly confirmed
            // only for this synthetic account fixture.
            let saturday_evening = day_class == DayClass::SaturdayOrPublicHoliday
                && (1_080..1_200).contains(&local_start_minute);

            declared_active_energy_kwh += energy_kwh;
            intervals.push(MeteredInterval {
                local_date: local_date.clone(),
                day_class,
                local_start_minute,
                duration_minutes: 15,
                energy_kwh,
                saturday_evening_confirmed: saturday_evening,
                saturday_confirmation_reference: saturday_evening
                    .then(|| "SYNTHETIC-SATURDAY-TOU-CONFIRMATION".to_owned()),
                public_holiday_evidence_reference: None,
            });
        }
    }

    TariffInput {
        source_profile_id: SOURCE_PROFILE_ID.to_owned(),
        profile_version: "0.1".to_owned(),
        billing_month: "2026-07".to_owned(),
        time_zone: "Africa/Johannesburg".to_owned(),
        account: Account {
            utility_id: CITY_POWER_UTILITY_ID.to_owned(),
            account_reference: "SYNTHETIC-CITYPOWER-ACCOUNT".to_owned(),
            supply_capacity_kva: 150.0,
            voltage_class: VoltageClass::Low,
            industrial_tou_approved: true,
            approval_reference: Some("SYNTHETIC-APPROVED-TOU-ACCOUNT".to_owned()),
            legacy_pre_1999_demand_tariff: false,
            service_connection_classification_reference: Some(
                "SYNTHETIC-LV-LPU-SERVICE-CLASS".to_owned(),
            ),
        },
        intervals,
        interval_series_complete: true,
        interval_coverage_evidence_reference:
            "SYNTHETIC-COMPLETE-2026-07-31-DAY-15MIN-INTERVAL-SERIES".to_owned(),
        declared_active_energy_kwh,
        measured_monthly_peak_demand_kva: 95.0,
        preceding_12_month_demand_history,
        demand_history_interval_minutes: 30,
        demand_history_interval_evidence_reference: "SYNTHETIC-30-MINUTE-DEMAND-METER-INTERVAL".to_owned(),
        demand_history_coverage_evidence_reference: "SYNTHETIC-COMPLETE-PRECEDING-12-MONTH-DEMAND-TRACE".to_owned(),
        power_factor: 0.95,
        measured_reactive_energy_kvarh: 10.0,
        network_surcharge_applies: Some(true),
        network_surcharge_evidence_reference: Some(
            "SYNTHETIC-ACCOUNT-NETWORK-SURCHARGE-STATUS".to_owned(),
        ),
        business_lpu_surcharge: Some(AssessedInvoiceLine {
            amount_zar: 12.34,
            evidence_reference: "SYNTHETIC-ASSESSED-BUSINESS-LPU-LINE".to_owned(),
        }),
        vat: Some(AssessedInvoiceLine {
            amount_zar: 56.78,
            evidence_reference: "SYNTHETIC-ASSESSED-VAT-LINE".to_owned(),
        }),
        calendar_mapping_reference: "SYNTHETIC-JOHANNESBURG-JULY-2026-CALENDAR".to_owned(),
        timebase_reference: "AFRICA-JOHANNESBURG-SAST".to_owned(),
        meter_boundary_reference: "SYNTHETIC-PCC-METER-BOUNDARY".to_owned(),
        meter_calibration_reference: "SYNTHETIC-METER-CALIBRATION".to_owned(),
        input_manifest_reference: "SYNTHETIC-INPUT-MANIFEST".to_owned(),
    }
}

fn main() -> ExitCode {
    let input = demo_input();
    match calculate_bill(&input) {
        Ok(receipt) => {
            println!("{}", receipt.to_json());
            if receipt.breakdown.invoice_total_zar.is_some() {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Err(error) => {
            eprintln!("reference evaluation failed: {error}");
            ExitCode::FAILURE
        }
    }
}
