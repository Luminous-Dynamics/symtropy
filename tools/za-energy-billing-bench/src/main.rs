use std::process::ExitCode;

use za_energy_billing_bench::{
    Account, AssessedInvoiceLine, CITY_POWER_UTILITY_ID, DayClass, DemandHistoryPoint,
    MeteredInterval, SOURCE_PROFILE_ID, TariffInput, VoltageClass, calculate_bill,
};

fn demo_input() -> TariffInput {
    let preceding_12_month_demand_history = (0..12)
        .map(|offset| {
            let absolute = 2026_i32 * 12 + 6 - 12 + offset;
            let year = absolute.div_euclid(12);
            let month = absolute.rem_euclid(12) + 1;
            DemandHistoryPoint {
                billing_month: format!("{year:04}-{month:02}"),
                measured_peak_kva: 125.0,
            }
        })
        .collect();

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
        intervals: vec![
            MeteredInterval {
                local_date: "2026-07-01".to_owned(),
                day_class: DayClass::Weekday,
                local_start_minute: 420,
                duration_minutes: 15,
                energy_kwh: 10.0,
                saturday_evening_confirmed: false,
                saturday_confirmation_reference: None,
            },
            MeteredInterval {
                local_date: "2026-07-01".to_owned(),
                day_class: DayClass::Weekday,
                local_start_minute: 1320,
                duration_minutes: 15,
                energy_kwh: 15.0,
                saturday_evening_confirmed: false,
                saturday_confirmation_reference: None,
            },
        ],
        interval_series_complete: true,
        interval_coverage_evidence_reference: "SYNTHETIC-COMPLETE-MONTHLY-INTERVAL-COVERAGE".to_owned(),
        declared_active_energy_kwh: 25.0,
        measured_monthly_peak_demand_kva: 95.0,
        preceding_12_month_demand_history,
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
        calendar_mapping_reference: "SYNTHETIC-CALENDAR-MAPPING".to_owned(),
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
