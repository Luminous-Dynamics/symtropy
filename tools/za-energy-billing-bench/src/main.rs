// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use za_energy_billing_bench::{
    AccountEvidence, BillingInput, DayClass, EvidenceReferences, MeteredInterval,
    Utility, VoltageClass, evaluate_bill, SOURCE_PROFILE_ID,
};

fn main() {
    // All inputs are synthetic. The assessed surcharge/VAT amounts are explicit
    // synthetic invoice lines, not inferred from a guessed percentage/base.
    let input = BillingInput {
        account: AccountEvidence {
            utility: Utility::CityPower,
            supply_capacity_kva: 125.0,
            city_power_approved: true,
            approval_reference: Some("SYNTHETIC-CP-APPROVAL".to_owned()),
            load_shift_qualification_reference: Some("SYNTHETIC-LOAD-SHIFT-QUALIFICATION".to_owned()),
            legacy_demand_contract_before_1999: false,
            network_surcharge_applies: true,
            network_surcharge_applicability_reference: Some("SYNTHETIC-ACCOUNT-CHARGE-STATUS".to_owned()),
        },
        voltage_class: VoltageClass::Low,
        intervals: vec![
            MeteredInterval {
                month: 6,
                day_class: DayClass::Weekday,
                local_start_minute: 8 * 60,
                duration_minutes: 15,
                active_energy_kwh: 100.0,
                measured_demand_kva: 95.0,
            }
        ],
        preceding_12_month_peak_kva: [
            125.0, 125.0, 125.0, 80.0, 90.0, 85.0, 75.0, 80.0, 88.0, 92.0, 96.0, 99.0
        ],
        power_factor: 0.95,
        reactive_energy_kvarh: 50.0,
        assessed_business_lpu_surcharge_zar: Some(12.34),
        assessed_vat_zar: Some(100.00),
        business_lpu_line_reference: Some("SYNTHETIC-LPU-LINE".to_owned()),
        vat_line_reference: Some("SYNTHETIC-VAT-LINE".to_owned()),
        evidence: EvidenceReferences {
            source_profile_id: SOURCE_PROFILE_ID.to_owned(),
            account_id: "SYNTHETIC-ACCOUNT-001".to_owned(),
            input_manifest_digest: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_owned(),
            tariff_approval_reference: "SYNTHETIC-CP-APPROVAL".to_owned(),
            meter_boundary_reference: "SYNTHETIC-PCC-METER".to_owned(),
            meter_calibration_reference: "SYNTHETIC-METER-CALIBRATION".to_owned(),
            calendar_mapping_reference: "SYNTHETIC-CALENDAR-MAPPING".to_owned(),
        },
        saturday_18_to_20_confirmed: false,
    };

    match evaluate_bill(&input) {
        Ok(report) => {
            println!("ZA-ENERGY-BILLING-REFERENCE");
            println!("result=PASS");
            println!("source_profile_id={}", report.source_profile_id);
            println!("evaluator_version={}", report.evaluator_version);
            println!("input_manifest_digest={}", report.input_manifest_digest);
            println!("active_energy_kwh={:.2}", report.total_active_energy_kwh);
            println!("energy_charge_zar={:.2}", report.energy_charge_zar);
            println!("monthly_service_charge_zar={:.2}", report.monthly_service_charge_zar);
            println!("monthly_network_charge_zar={:.2}", report.monthly_network_charge_zar);
            println!("demand_charge_zar={:.2}", report.demand_charge_zar);
            println!("network_surcharge_zar={:.2}", report.network_surcharge_zar);
            println!("reactive_energy_charge_zar={:.2}", report.reactive_energy_charge_zar);
            println!("known_component_subtotal_zar={:.2}", report.known_component_subtotal_zar);
            println!("invoice_reconciliation_complete={}", report.invoice_reconciliation_complete);
            println!("synthetic_full_invoice_total_zar={:.2}", report.full_invoice_total_zar.unwrap_or_default());
            println!("claim_ceiling={}", report.claim_ceiling);
        }
        Err(error) => {
            eprintln!("result=FAIL");
            eprintln!("error={error}");
            std::process::exit(1);
        }
    }
}
