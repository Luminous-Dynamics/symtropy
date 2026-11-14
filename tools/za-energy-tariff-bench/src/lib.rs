#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

const SUMMER_MONTHS: [u8; 9] = [9, 10, 11, 12, 1, 2, 3, 4, 5];
const NETWORK_SURCHARGE_C_PER_KWH: f64 = 6.0;
const BUSINESS_LPU_SURCHARGE_PERCENT: f64 = 2.0;
const DEMAND_FLOOR_KVA: f64 = 70.0;
const DEMAND_RATCHET_FRACTION: f64 = 0.80;
const REACTIVE_ENERGY_CHARGE_C_PER_KVARH: f64 = 46.25;
const POWER_FACTOR_THRESHOLD: f64 = 0.96;
const REACTIVE_ENERGY_THRESHOLD_FRACTION: f64 = 0.30;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VoltageClass {
    Lv,
    Mv,
    Hv,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DayType {
    Weekday,
    SaturdayOrPublicHoliday,
    Sunday,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TariffPeriod {
    Peak,
    Standard,
    OffPeak,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SurchargeBasis {
    EnergyChargeOnly,
    EnergyAndNetworkSurcharge,
    AllVariableCharges,
    AllPreVatCharges,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Fixture {
    pub schema_version: u8,
    pub profile_id: String,
    pub source_note: String,
    pub cases: Vec<BillingCase>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BillingCase {
    pub case_id: String,
    pub description: String,
    pub billing_month: u8,
    pub account: Account,
    pub measured_monthly_max_demand_kva: f64,
    pub previous_twelve_month_top_three_kva: [f64; 3],
    pub power_factor: f64,
    pub reactive_energy_kvarh: f64,
    pub apply_business_lpu_surcharge: bool,
    pub surcharge_basis: Option<SurchargeBasis>,
    pub surcharge_basis_confirmed: bool,
    pub vat_rate: Option<f64>,
    pub vat_basis_confirmed: bool,
    pub intervals: Vec<Interval>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Account {
    pub distributor: String,
    pub voltage_class: VoltageClass,
    pub supply_capacity_kva: f64,
    pub industrial_tou_application_approved: bool,
    pub contract_confirmed: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Interval {
    pub day_type: DayType,
    /// Local minutes after midnight. Intervals must not cross midnight.
    pub start_minute: u16,
    pub duration_minutes: u16,
    pub energy_kwh: f64,
    /// Maximum apparent power during this interval, not interval-average kW.
    pub maximum_demand_kva: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Evaluation {
    pub case_id: String,
    pub billing_month: u8,
    pub season: String,
    pub site_tariff_eligibility: String,
    pub eligibility_reasons: Vec<String>,
    pub billing_claim_status: String,
    pub energy_kwh: f64,
    pub peak_energy_kwh: f64,
    pub standard_energy_kwh: f64,
    pub off_peak_energy_kwh: f64,
    pub energy_charge_zar: f64,
    pub network_surcharge_zar: f64,
    pub fixed_service_charge_zar: f64,
    pub fixed_network_charge_zar: f64,
    pub measured_monthly_max_demand_kva: f64,
    pub rolling_top_three_average_kva: f64,
    pub demand_floor_kva: f64,
    pub billed_demand_kva: f64,
    pub demand_charge_zar: f64,
    pub reactive_energy_charge_zar: f64,
    pub reactive_energy_billable_kvarh: f64,
    pub business_lpu_surcharge_zar: Option<f64>,
    pub surcharge_basis_status: String,
    pub subtotal_basis_status: String,
    pub subtotal_before_vat_zar: f64,
    pub vat_zar: Option<f64>,
    pub vat_basis_status: String,
    pub invoice_equivalent_total_zar: Option<f64>,
    pub warnings: Vec<String>,
    pub line_item_notes: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
struct Rates {
    peak_c_per_kwh: f64,
    standard_c_per_kwh: f64,
    off_peak_c_per_kwh: f64,
    service_zar_per_month: f64,
    network_zar_per_month: f64,
    demand_zar_per_kva_month: f64,
}

fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

fn valid_non_negative(value: f64, at: &str) -> Result<(), String> {
    if !value.is_finite() || value < 0.0 {
        return Err(format!("{at}: expected a finite non-negative value"));
    }
    Ok(())
}

fn rates_for(month: u8, voltage: VoltageClass) -> Result<(&'static str, Rates), String> {
    if !(1..=12).contains(&month) {
        return Err(format!("billing_month: expected 1..=12, got {month}"));
    }

    let summer = SUMMER_MONTHS.contains(&month);
    let (peak, standard, off_peak) = if summer {
        (322.00, 242.42, 186.35)
    } else {
        (766.24, 292.56, 200.44)
    };

    let (service, network, demand) = match voltage {
        VoltageClass::Lv => (2444.32, 2185.32, 461.28),
        VoltageClass::Mv => (3360.96, 9347.51, 431.11),
        VoltageClass::Hv => (42915.09, 43848.71, 400.93),
    };

    Ok((
        if summer { "summer" } else { "winter" },
        Rates {
            peak_c_per_kwh: peak,
            standard_c_per_kwh: standard,
            off_peak_c_per_kwh: off_peak,
            service_zar_per_month: service,
            network_zar_per_month: network,
            demand_zar_per_kva_month: demand,
        },
    ))
}

fn classify_minute(day_type: DayType, minute: u16) -> Result<TariffPeriod, String> {
    if minute >= 1440 {
        return Err(format!("time minute {minute} is outside 00:00..23:59"));
    }

    let result = match day_type {
        DayType::Weekday => {
            if (420..600).contains(&minute) || (1080..1200).contains(&minute) {
                TariffPeriod::Peak
            } else if (360..420).contains(&minute)
                || (600..1080).contains(&minute)
                || (1200..1320).contains(&minute)
            {
                TariffPeriod::Standard
            } else {
                TariffPeriod::OffPeak
            }
        }
        DayType::SaturdayOrPublicHoliday => {
            // The source budget prints "18h00-20h0". Do not silently infer
            // the missing digit for billing-grade calculations.
            if (1080..1200).contains(&minute) {
                return Err(
                    "SOURCE_AMBIGUITY: Saturday/public-holiday standard interval is printed as '18h00-20h0'; confirm the endpoint with City Power or an actual invoice"
                        .to_owned(),
                );
            }
            if (420..720).contains(&minute) {
                TariffPeriod::Standard
            } else {
                TariffPeriod::OffPeak
            }
        }
        DayType::Sunday => {
            if (1020..1140).contains(&minute) {
                TariffPeriod::Standard
            } else {
                TariffPeriod::OffPeak
            }
        }
    };

    Ok(result)
}

fn classify_interval(interval: &Interval) -> Result<TariffPeriod, String> {
    if interval.duration_minutes == 0 {
        return Err("interval duration must be positive".to_owned());
    }
    let end = interval
        .start_minute
        .checked_add(interval.duration_minutes)
        .ok_or_else(|| "interval time overflow".to_owned())?;
    if end > 1440 {
        return Err(
            "interval crosses local midnight; split it into calendar-day intervals".to_owned(),
        );
    }

    let first = classify_minute(interval.day_type, interval.start_minute)?;
    for minute in (interval.start_minute + 1)..end {
        let current = classify_minute(interval.day_type, minute)?;
        if current != first {
            return Err(format!(
                "interval crosses a tariff boundary at local minute {minute}; split the interval"
            ));
        }
    }
    Ok(first)
}

fn eligibility(account: &Account) -> (bool, Vec<String>) {
    let mut reasons = Vec::new();
    if account.distributor != "City Power Johannesburg" {
        reasons.push("distributor is not City Power Johannesburg".to_owned());
    }
    if account.supply_capacity_kva <= 100.0 {
        reasons.push("published eligibility requires supply capacity exceeding 100 kVA".to_owned());
    }
    if !account.industrial_tou_application_approved {
        reasons.push("industrial TOU application/City Power approval is not confirmed".to_owned());
    }
    if !account.contract_confirmed {
        reasons.push("actual account tariff/contract is not confirmed".to_owned());
    }
    (reasons.is_empty(), reasons)
}

fn surcharge_base(
    basis: SurchargeBasis,
    energy_charge: f64,
    network_surcharge: f64,
    demand_charge: f64,
    reactive_charge: f64,
    service_charge: f64,
    fixed_network_charge: f64,
) -> f64 {
    match basis {
        SurchargeBasis::EnergyChargeOnly => energy_charge,
        SurchargeBasis::EnergyAndNetworkSurcharge => energy_charge + network_surcharge,
        SurchargeBasis::AllVariableCharges => {
            energy_charge + network_surcharge + demand_charge + reactive_charge
        }
        SurchargeBasis::AllPreVatCharges => {
            energy_charge
                + network_surcharge
                + demand_charge
                + reactive_charge
                + service_charge
                + fixed_network_charge
        }
    }
}

pub fn evaluate(case: &BillingCase) -> Result<Evaluation, String> {
    let (season, rates) = rates_for(case.billing_month, case.account.voltage_class)?;
    valid_non_negative(case.account.supply_capacity_kva, "supply_capacity_kva")?;
    valid_non_negative(
        case.measured_monthly_max_demand_kva,
        "measured_monthly_max_demand_kva",
    )?;
    valid_non_negative(case.power_factor, "power_factor")?;
    valid_non_negative(case.reactive_energy_kvarh, "reactive_energy_kvarh")?;
    for (index, demand) in case.previous_twelve_month_top_three_kva.iter().enumerate() {
        valid_non_negative(
            *demand,
            &format!("previous_twelve_month_top_three_kva[{index}]"),
        )?;
    }
    if case.power_factor > 1.0 {
        return Err("power_factor must be <= 1.0".to_owned());
    }
    if let Some(vat) = case.vat_rate {
        if !vat.is_finite() || !(0.0..=1.0).contains(&vat) {
            return Err("vat_rate must be a fraction between 0 and 1".to_owned());
        }
    }

    let mut energy_kwh = 0.0;
    let mut peak_energy_kwh = 0.0;
    let mut standard_energy_kwh = 0.0;
    let mut off_peak_energy_kwh = 0.0;
    let mut energy_charge_zar = 0.0;
    let mut interval_max_kva: f64 = 0.0;

    if case.intervals.is_empty() {
        return Err("at least one interval is required".to_owned());
    }

    for (index, interval) in case.intervals.iter().enumerate() {
        let label = format!("interval[{index}]");
        valid_non_negative(interval.energy_kwh, &format!("{label}.energy_kwh"))?;
        valid_non_negative(
            interval.maximum_demand_kva,
            &format!("{label}.maximum_demand_kva"),
        )?;
        let period = classify_interval(interval)?;
        energy_kwh += interval.energy_kwh;
        interval_max_kva = interval_max_kva.max(interval.maximum_demand_kva);
        match period {
            TariffPeriod::Peak => {
                peak_energy_kwh += interval.energy_kwh;
                energy_charge_zar += interval.energy_kwh * rates.peak_c_per_kwh / 100.0;
            }
            TariffPeriod::Standard => {
                standard_energy_kwh += interval.energy_kwh;
                energy_charge_zar += interval.energy_kwh * rates.standard_c_per_kwh / 100.0;
            }
            TariffPeriod::OffPeak => {
                off_peak_energy_kwh += interval.energy_kwh;
                energy_charge_zar += interval.energy_kwh * rates.off_peak_c_per_kwh / 100.0;
            }
        }
    }

    let measured_max = case.measured_monthly_max_demand_kva.max(interval_max_kva);
    let rolling_top_three_average =
        case.previous_twelve_month_top_three_kva.iter().sum::<f64>() / 3.0;
    let demand_floor = DEMAND_FLOOR_KVA.max(DEMAND_RATCHET_FRACTION * rolling_top_three_average);
    let billed_demand = measured_max.max(demand_floor);
    let demand_charge_zar = billed_demand * rates.demand_zar_per_kva_month;

    let reactive_energy_billable_kvarh = if case.power_factor < POWER_FACTOR_THRESHOLD {
        (case.reactive_energy_kvarh - REACTIVE_ENERGY_THRESHOLD_FRACTION * energy_kwh).max(0.0)
    } else {
        0.0
    };
    let reactive_energy_charge_zar =
        reactive_energy_billable_kvarh * REACTIVE_ENERGY_CHARGE_C_PER_KVARH / 100.0;

    let network_surcharge_zar = energy_kwh * NETWORK_SURCHARGE_C_PER_KWH / 100.0;
    let subtotal_before_surcharge_zar = energy_charge_zar
        + network_surcharge_zar
        + rates.service_zar_per_month
        + rates.network_zar_per_month
        + demand_charge_zar
        + reactive_energy_charge_zar;

    let (business_lpu_surcharge_zar, surcharge_basis_status, surcharge_amount) =
        if !case.apply_business_lpu_surcharge {
            (Some(0.0), "not_applicable_by_case_input".to_owned(), 0.0)
        } else if let Some(basis) = case.surcharge_basis {
            let base = surcharge_base(
                basis,
                energy_charge_zar,
                network_surcharge_zar,
                demand_charge_zar,
                reactive_energy_charge_zar,
                rates.service_zar_per_month,
                rates.network_zar_per_month,
            );
            let amount = base * BUSINESS_LPU_SURCHARGE_PERCENT / 100.0;
            (
                Some(round2(amount)),
                if case.surcharge_basis_confirmed {
                    format!("confirmed_basis_{basis:?}")
                } else {
                    format!("assumed_basis_requires_invoice_confirmation_{basis:?}")
                },
                amount,
            )
        } else {
            (None, "not_calculated_basis_not_confirmed".to_owned(), 0.0)
        };

    let surcharge_calculable =
        !case.apply_business_lpu_surcharge || business_lpu_surcharge_zar.is_some();
    let surcharge_confirmed = !case.apply_business_lpu_surcharge
        || (business_lpu_surcharge_zar.is_some() && case.surcharge_basis_confirmed);
    let subtotal_before_vat_zar = subtotal_before_surcharge_zar + surcharge_amount;
    let vat_basis_status = if case.vat_rate.is_none() {
        "vat_rate_not_supplied".to_owned()
    } else if case.vat_basis_confirmed {
        "vat_basis_confirmed".to_owned()
    } else {
        "vat_basis_not_confirmed".to_owned()
    };
    let vat_zar = if surcharge_calculable && case.vat_basis_confirmed {
        case.vat_rate
            .map(|rate| round2(subtotal_before_vat_zar * rate))
    } else {
        None
    };
    let (is_eligible, eligibility_reasons) = eligibility(&case.account);
    let mut warnings = eligibility_reasons.clone();
    warnings.push(
        "6 c/kWh network surcharge is separate from the monthly fixed network charge".to_owned(),
    );
    warnings.push(
        "the 2% business/LPU surcharge basis must be verified against the actual tariff/invoice"
            .to_owned(),
    );
    warnings.push(
        "VAT is only calculated when an explicit vat_rate is provided; no tax rate is inferred"
            .to_owned(),
    );
    warnings.push(
        "tariff values are the budget-published FY2026/27 schedule, not proof of a site's approved tariff"
            .to_owned(),
    );

    let invoice_equivalent_total_zar = match (
        is_eligible,
        surcharge_confirmed,
        case.vat_basis_confirmed,
        vat_zar,
    ) {
        (true, true, true, Some(vat)) => Some(round2(subtotal_before_vat_zar + vat)),
        _ => None,
    };

    Ok(Evaluation {
        case_id: case.case_id.clone(),
        billing_month: case.billing_month,
        season: season.to_owned(),
        site_tariff_eligibility: if is_eligible {
            "eligible_and_contract_confirmed".to_owned()
        } else {
            "not_confirmed_for_invoice_equivalent_claim".to_owned()
        },
        eligibility_reasons,
        billing_claim_status: if invoice_equivalent_total_zar.is_some() {
            "contract_confirmed_estimate".to_owned()
        } else {
            "reference_scenario_only".to_owned()
        },
        energy_kwh: round2(energy_kwh),
        peak_energy_kwh: round2(peak_energy_kwh),
        standard_energy_kwh: round2(standard_energy_kwh),
        off_peak_energy_kwh: round2(off_peak_energy_kwh),
        energy_charge_zar: round2(energy_charge_zar),
        network_surcharge_zar: round2(network_surcharge_zar),
        fixed_service_charge_zar: round2(rates.service_zar_per_month),
        fixed_network_charge_zar: round2(rates.network_zar_per_month),
        measured_monthly_max_demand_kva: round2(measured_max),
        rolling_top_three_average_kva: round2(rolling_top_three_average),
        demand_floor_kva: round2(demand_floor),
        billed_demand_kva: round2(billed_demand),
        demand_charge_zar: round2(demand_charge_zar),
        reactive_energy_charge_zar: round2(reactive_energy_charge_zar),
        reactive_energy_billable_kvarh: round2(reactive_energy_billable_kvarh),
        business_lpu_surcharge_zar,
        surcharge_basis_status,
        subtotal_basis_status: if surcharge_calculable {
            "all_declared_line_items_included".to_owned()
        } else {
            "excludes_unresolved_business_lpu_surcharge".to_owned()
        },
        subtotal_before_vat_zar: round2(subtotal_before_vat_zar),
        vat_zar,
        vat_basis_status,
        invoice_equivalent_total_zar,
        warnings,
        line_item_notes: vec![
            "Energy charges use the budget-published cents/kWh rates for the specified month and classified local time bands.".to_owned(),
            "Monthly billed demand = max(measured monthly maximum, 70 kVA, 80% × average of prior 12-month top-three demands).".to_owned(),
            "Reactive-energy charge is calculated only when PF < 0.96 and only on reactive kVArh above 30% of active kWh; verify this interpretation against the actual account tariff.".to_owned(),
            "The Saturday/public-holiday interval 18:00–20:00 fails closed because the source prints 18h00-20h0.".to_owned(),
        ],
    })
}

pub fn evaluate_fixture(fixture: &Fixture) -> Result<Vec<Evaluation>, String> {
    if fixture.schema_version != 1 {
        return Err(format!(
            "unsupported schema_version {}",
            fixture.schema_version
        ));
    }
    if fixture.cases.is_empty() {
        return Err("fixture must contain at least one case".to_owned());
    }
    let mut case_ids = BTreeSet::new();
    for case in &fixture.cases {
        if case.case_id.trim().is_empty() {
            return Err("case_id must not be empty".to_owned());
        }
        if !case_ids.insert(case.case_id.as_str()) {
            return Err(format!("duplicate case_id {}", case.case_id));
        }
    }
    fixture.cases.iter().map(evaluate).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn account() -> Account {
        Account {
            distributor: "City Power Johannesburg".to_owned(),
            voltage_class: VoltageClass::Lv,
            supply_capacity_kva: 160.0,
            industrial_tou_application_approved: true,
            contract_confirmed: true,
        }
    }

    fn base_case() -> BillingCase {
        BillingCase {
            case_id: "unit-test".to_owned(),
            description: "unit test".to_owned(),
            billing_month: 7,
            account: account(),
            measured_monthly_max_demand_kva: 80.0,
            previous_twelve_month_top_three_kva: [125.0, 125.0, 125.0],
            power_factor: 0.98,
            reactive_energy_kvarh: 0.0,
            apply_business_lpu_surcharge: false,
            surcharge_basis: None,
            surcharge_basis_confirmed: false,
            vat_rate: None,
            vat_basis_confirmed: false,
            intervals: vec![Interval {
                day_type: DayType::Weekday,
                start_minute: 23 * 60,
                duration_minutes: 30,
                energy_kwh: 100.0,
                maximum_demand_kva: 80.0,
            }],
        }
    }

    #[test]
    fn duplicate_case_ids_are_rejected() {
        let mut fixture = Fixture {
            schema_version: 1,
            profile_id: "test".to_owned(),
            source_note: "test".to_owned(),
            cases: vec![base_case(), base_case()],
        };
        fixture.cases[1].case_id = fixture.cases[0].case_id.clone();
        assert!(
            evaluate_fixture(&fixture)
                .expect_err("duplicate identities must be rejected")
                .contains("duplicate case_id")
        );
    }

    #[test]
    fn winter_weekday_peak_to_off_peak_energy_spread_is_565_80_for_100kwh() {
        let mut peak = base_case();
        peak.case_id = "peak".to_owned();
        peak.intervals[0].start_minute = 7 * 60 + 30;

        let mut off_peak = base_case();
        off_peak.case_id = "off-peak".to_owned();

        let peak_eval = evaluate(&peak).expect("peak evaluates");
        let off_peak_eval = evaluate(&off_peak).expect("off-peak evaluates");
        assert!((peak_eval.energy_charge_zar - 766.24).abs() < 0.001);
        assert!((off_peak_eval.energy_charge_zar - 200.44).abs() < 0.001);
        assert!(
            (peak_eval.energy_charge_zar - off_peak_eval.energy_charge_zar - 565.80).abs() < 0.001
        );
    }

    #[test]
    fn prior_top_three_ratchet_can_hold_billed_demand_constant() {
        let mut before = base_case();
        before.measured_monthly_max_demand_kva = 95.0;
        before.intervals[0].maximum_demand_kva = 95.0;
        before.intervals[0].start_minute = 7 * 60 + 30;

        let mut after = base_case();
        after.measured_monthly_max_demand_kva = 80.0;
        after.intervals[0].maximum_demand_kva = 80.0;

        let before_eval = evaluate(&before).expect("before evaluates");
        let after_eval = evaluate(&after).expect("after evaluates");
        assert!((before_eval.billed_demand_kva - 100.0).abs() < 0.001);
        assert!((after_eval.billed_demand_kva - 100.0).abs() < 0.001);
        assert!((before_eval.demand_charge_zar - after_eval.demand_charge_zar).abs() < 0.001);
    }

    #[test]
    fn saturday_source_typo_fails_closed() {
        let mut case = base_case();
        case.intervals[0].day_type = DayType::SaturdayOrPublicHoliday;
        case.intervals[0].start_minute = 18 * 60;
        let err = evaluate(&case).expect_err("ambiguous source band must fail");
        assert!(err.contains("SOURCE_AMBIGUITY"));
    }

    #[test]
    fn tariff_boundary_crossing_requires_split_intervals() {
        let mut case = base_case();
        case.intervals[0].start_minute = 6 * 60 + 30;
        case.intervals[0].duration_minutes = 60;
        let err = evaluate(&case).expect_err("cross-boundary interval must fail");
        assert!(err.contains("tariff boundary"));
    }

    #[test]
    fn sunday_peak_is_never_created() {
        let mut case = base_case();
        case.intervals[0].day_type = DayType::Sunday;
        case.intervals[0].start_minute = 8 * 60;
        let result = evaluate(&case).expect("Sunday off-peak evaluates");
        assert!(result.peak_energy_kwh.abs() < 0.001);
        assert!((result.off_peak_energy_kwh - 100.0).abs() < 0.001);
    }

    #[test]
    fn reactive_energy_charge_applies_only_below_power_factor_threshold() {
        let mut case = base_case();
        case.power_factor = 0.95;
        case.reactive_energy_kvarh = 50.0;
        let result = evaluate(&case).expect("reactive energy evaluates");
        assert!((result.reactive_energy_billable_kvarh - 20.0).abs() < 0.001);
        assert!((result.reactive_energy_charge_zar - 9.25).abs() < 0.001);

        case.power_factor = 0.96;
        let result_at_threshold = evaluate(&case).expect("threshold evaluates");
        assert!(result_at_threshold.reactive_energy_charge_zar.abs() < 0.001);
    }

    #[test]
    fn account_surcharge_basis_is_not_invented() {
        let mut case = base_case();
        case.apply_business_lpu_surcharge = true;
        case.surcharge_basis = None;
        case.vat_rate = Some(0.15);
        let result = evaluate(&case).expect("scenario is still calculable");
        assert_eq!(result.business_lpu_surcharge_zar, None);
        assert_eq!(result.invoice_equivalent_total_zar, None);
        assert_eq!(result.vat_zar, None);
        assert!(result.surcharge_basis_status.contains("not_confirmed"));
        assert_eq!(
            result.subtotal_basis_status,
            "excludes_unresolved_business_lpu_surcharge"
        );
    }

    #[test]
    fn unconfirmed_distributor_cannot_receive_invoice_equivalent_claim() {
        let mut case = base_case();
        case.account.distributor = "Other municipality".to_owned();
        case.vat_rate = Some(0.15);
        let result = evaluate(&case).expect("reference calculation is allowed");
        assert_eq!(result.billing_claim_status, "reference_scenario_only");
        assert_eq!(result.invoice_equivalent_total_zar, None);
    }

    #[test]
    fn assumed_surcharge_basis_does_not_create_invoice_total() {
        let mut case = base_case();
        case.apply_business_lpu_surcharge = true;
        case.surcharge_basis = Some(SurchargeBasis::EnergyAndNetworkSurcharge);
        case.surcharge_basis_confirmed = false;
        case.vat_rate = Some(0.15);
        case.vat_basis_confirmed = true;
        let result = evaluate(&case).expect("scenario calculation is allowed");
        assert!(result.business_lpu_surcharge_zar.is_some());
        assert!(result.vat_zar.is_some());
        assert_eq!(result.invoice_equivalent_total_zar, None);
        assert!(
            result
                .surcharge_basis_status
                .contains("requires_invoice_confirmation")
        );
    }

    #[test]
    fn invoice_total_requires_confirmed_eligibility_surcharge_basis_and_vat() {
        let mut case = base_case();
        case.apply_business_lpu_surcharge = true;
        case.surcharge_basis = Some(SurchargeBasis::EnergyAndNetworkSurcharge);
        case.surcharge_basis_confirmed = true;
        case.vat_rate = Some(0.15);
        case.vat_basis_confirmed = true;
        let result = evaluate(&case).expect("confirmed reference inputs evaluate");
        assert!(result.business_lpu_surcharge_zar.is_some());
        assert!(result.vat_zar.is_some());
        assert!(result.invoice_equivalent_total_zar.is_some());
        assert_eq!(result.billing_claim_status, "contract_confirmed_estimate");
    }

    #[test]
    fn capacity_of_exactly_100_kva_is_not_eligible() {
        let mut case = base_case();
        case.account.supply_capacity_kva = 100.0;
        case.vat_rate = Some(0.15);
        let result = evaluate(&case).expect("reference scenario can still be calculated");
        assert_eq!(
            result.site_tariff_eligibility,
            "not_confirmed_for_invoice_equivalent_claim"
        );
        assert_eq!(result.invoice_equivalent_total_zar, None);
    }

    #[test]
    fn invalid_month_and_non_finite_power_factor_are_rejected() {
        let mut case = base_case();
        case.billing_month = 13;
        assert!(
            evaluate(&case)
                .expect_err("month must be rejected")
                .contains("billing_month")
        );
        case = base_case();
        case.power_factor = f64::NAN;
        assert!(
            evaluate(&case)
                .expect_err("NaN PF must be rejected")
                .contains("power_factor")
        );
    }

    #[test]
    fn vat_rate_without_confirmed_taxable_basis_does_not_create_vat_or_invoice_total() {
        let mut case = base_case();
        case.vat_rate = Some(0.15);
        case.vat_basis_confirmed = false;
        let result = evaluate(&case).expect("reference scenario evaluates");
        assert_eq!(result.vat_zar, None);
        assert_eq!(result.vat_basis_status, "vat_basis_not_confirmed");
        assert_eq!(result.invoice_equivalent_total_zar, None);
    }

    #[test]
    fn fixed_and_network_surcharges_are_separate_line_items() {
        let result = evaluate(&base_case()).expect("case evaluates");
        assert!((result.network_surcharge_zar - 6.0).abs() < 0.001);
        assert!((result.fixed_network_charge_zar - 2185.32).abs() < 0.001);
        assert!((result.network_surcharge_zar - result.fixed_network_charge_zar).abs() > 1.0);
    }
}
