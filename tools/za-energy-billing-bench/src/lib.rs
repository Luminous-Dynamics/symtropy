// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Deterministic component accounting for the City Power 2026/27 Industrial TOU
//! budget-published tariff. This is a reference/conformance evaluator only.
//!
//! It does not predict prices, infer account eligibility, parse timezones,
//! reconstruct an invoice from incomplete account evidence, or dispatch loads.
//! All calendar classifications and account-specific add-on amounts are
//! explicit caller inputs. Unknown charges stay unknown.

use core::fmt;

pub const EVALUATOR_VERSION: &str = "0.1.0";
pub const SOURCE_PROFILE_ID: &str = "ZA-CITYPOWER-INDUSTRIAL-TOU-2026-27-BUDGET-PROPOSED";
const APPROVAL_THRESHOLD_KVA: f64 = 100.0;
const MINIMUM_BILLED_DEMAND_KVA: f64 = 70.0;
const DEMAND_RATCHET_FRACTION: f64 = 0.80;
const REACTIVE_PF_THRESHOLD: f64 = 0.96;
const REACTIVE_ENERGY_THRESHOLD: f64 = 0.30;
const REACTIVE_ENERGY_RATE_CENTS_PER_KVARH: f64 = 46.25;
const NETWORK_SURCHARGE_CENTS_PER_KWH: f64 = 6.0;

#[derive(Debug, Clone, PartialEq)]
pub enum BillingError {
    WrongUtility,
    SupplyCapacityNotEligible { supply_kva: f64 },
    LegacyContractRequiresManualReview,
    CityPowerApprovalMissing,
    LoadShiftQualificationMissing,
    MissingEvidence(&'static str),
    EvidenceMismatch(&'static str),
    InvalidValue { name: &'static str, value: f64 },
    InvalidMonth(u8),
    InvalidLocalMinute(u16),
    InvalidIntervalDuration(u16),
    IntervalCrossesMidnight,
    IntervalCrossesTariffBoundary,
    SaturdayEndpointNeedsConfirmation,
    NoIntervals,
    WrongSourceProfile { supplied: String },
    InvalidInputDigest,
}

impl fmt::Display for BillingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongUtility => write!(f, "the supplied electricity utility is not City Power"),
            Self::SupplyCapacityNotEligible { supply_kva } => write!(
                f,
                "new industrial TOU eligibility requires supply capacity strictly greater than 100 kVA; got {supply_kva}"
            ),
            Self::LegacyContractRequiresManualReview => write!(
                f,
                "legacy pre-1-July-1999 demand-tariff agreement requires manual account review; it is not an automatic industrial TOU eligibility bypass"
            ),
            Self::CityPowerApprovalMissing => write!(f, "City Power approval evidence is missing"),
            Self::LoadShiftQualificationMissing => write!(f, "large-consumer load-shifting qualification evidence is missing"),
            Self::MissingEvidence(name) => write!(f, "required evidence is missing: {name}"),
            Self::EvidenceMismatch(name) => write!(f, "evidence references do not match: {name}"),
            Self::InvalidValue { name, value } => write!(f, "{name} must be finite and non-negative; got {value}"),
            Self::InvalidMonth(month) => write!(f, "month must be in 1..=12; got {month}"),
            Self::InvalidLocalMinute(minute) => write!(f, "local minute must be in 0..1440; got {minute}"),
            Self::InvalidIntervalDuration(duration) => write!(f, "interval duration must be in 1..=1440 minutes; got {duration}"),
            Self::IntervalCrossesMidnight => write!(f, "v0.1 intervals must be split at local midnight"),
            Self::IntervalCrossesTariffBoundary => write!(f, "interval crosses a published tariff boundary; split the interval before evaluation"),
            Self::SaturdayEndpointNeedsConfirmation => write!(f, "Saturday/public-holiday 18:00-20:00 classification is blocked until the source-printed 18h00-20h0 endpoint is confirmed"),
            Self::NoIntervals => write!(f, "at least one metered interval is required"),
            Self::WrongSourceProfile { supplied } => write!(f, "source profile mismatch: expected {SOURCE_PROFILE_ID}, got {supplied}"),
            Self::InvalidInputDigest => write!(f, "input manifest digest must be 64 lowercase hexadecimal characters"),
        }
    }
}

impl std::error::Error for BillingError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Utility {
    CityPower,
    Other(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoltageClass {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DayClass {
    Weekday,
    SaturdayOrPublicHoliday,
    Sunday,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Season {
    Summer,
    Winter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnergyPeriod {
    Peak,
    Standard,
    OffPeak,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AccountEvidence {
    pub utility: Utility,
    pub supply_capacity_kva: f64,
    pub city_power_approved: bool,
    pub approval_reference: Option<String>,
    pub load_shift_qualification_reference: Option<String>,
    pub legacy_demand_contract_before_1999: bool,
    pub network_surcharge_applies: bool,
    pub network_surcharge_applicability_reference: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EvidenceReferences {
    pub source_profile_id: String,
    pub account_id: String,
    /// Caller-supplied digest of the frozen meter/calendar/account input manifest.
    /// The evaluator pins this value but does not hash or authenticate the bytes.
    pub input_manifest_digest: String,
    pub tariff_approval_reference: String,
    pub meter_boundary_reference: String,
    pub meter_calibration_reference: String,
    pub calendar_mapping_reference: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MeteredInterval {
    pub month: u8,
    pub day_class: DayClass,
    pub local_start_minute: u16,
    pub duration_minutes: u16,
    pub active_energy_kwh: f64,
    pub measured_demand_kva: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BillingInput {
    pub account: AccountEvidence,
    pub voltage_class: VoltageClass,
    pub intervals: Vec<MeteredInterval>,
    /// Twelve monthly peak-demand observations covering the complete
    /// preceding 12-month lookback. Values are not reconstructed from kWh.
    pub preceding_12_month_peak_kva: [f64; 12],
    pub power_factor: f64,
    pub reactive_energy_kvarh: f64,
    /// Actual invoice line amount, not calculated as 2% of an assumed base.
    pub assessed_business_lpu_surcharge_zar: Option<f64>,
    /// Actual tax invoice line amount, not calculated using a hardcoded VAT rate.
    pub assessed_vat_zar: Option<f64>,
    pub business_lpu_line_reference: Option<String>,
    pub vat_line_reference: Option<String>,
    pub evidence: EvidenceReferences,
    /// The budget prints "18h00-20h0". This must be true only when the
    /// customer tariff statement or City Power has confirmed the endpoint.
    pub saturday_18_to_20_confirmed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PeriodEnergy {
    pub peak_kwh: f64,
    pub standard_kwh: f64,
    pub off_peak_kwh: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BillingReport {
    pub evaluator_version: &'static str,
    pub source_profile_id: &'static str,
    pub account_id: String,
    pub input_manifest_digest: String,
    pub tariff_approval_reference: String,
    pub meter_boundary_reference: String,
    pub meter_calibration_reference: String,
    pub calendar_mapping_reference: String,
    pub season_energy: PeriodEnergy,
    pub total_active_energy_kwh: f64,
    pub energy_charge_zar: f64,
    pub monthly_service_charge_zar: f64,
    pub monthly_network_charge_zar: f64,
    pub measured_peak_demand_kva: f64,
    pub ratchet_floor_kva: f64,
    pub billed_demand_kva: f64,
    pub demand_charge_zar: f64,
    pub network_surcharge_zar: f64,
    pub reactive_energy_excess_kvarh: f64,
    pub reactive_energy_charge_zar: f64,
    pub assessed_business_lpu_surcharge_zar: Option<f64>,
    pub assessed_vat_zar: Option<f64>,
    pub known_component_subtotal_zar: f64,
    pub full_invoice_total_zar: Option<f64>,
    pub invoice_reconciliation_complete: bool,
    pub claim_ceiling: &'static str,
}

fn require_non_empty(name: &'static str, value: &str) -> Result<(), BillingError> {
    if value.trim().is_empty() {
        Err(BillingError::MissingEvidence(name))
    } else {
        Ok(())
    }
}

fn require_finite_non_negative(name: &'static str, value: f64) -> Result<(), BillingError> {
    if !value.is_finite() || value < 0.0 {
        Err(BillingError::InvalidValue { name, value })
    } else {
        Ok(())
    }
}

fn round_currency(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

pub fn validate_account(account: &AccountEvidence) -> Result<(), BillingError> {
    require_finite_non_negative("supply capacity kVA", account.supply_capacity_kva)?;
    match &account.utility {
        Utility::CityPower => {}
        Utility::Other(_) => return Err(BillingError::WrongUtility),
    }
    if account.supply_capacity_kva <= APPROVAL_THRESHOLD_KVA {
        if account.legacy_demand_contract_before_1999 {
            return Err(BillingError::LegacyContractRequiresManualReview);
        }
        return Err(BillingError::SupplyCapacityNotEligible {
            supply_kva: account.supply_capacity_kva,
        });
    }
    if !account.city_power_approved {
        return Err(BillingError::CityPowerApprovalMissing);
    }
    require_non_empty(
        "City Power approval reference",
        account.approval_reference.as_deref().unwrap_or(""),
    )?;
    require_non_empty(
        "load-shift qualification reference",
        account.load_shift_qualification_reference.as_deref().unwrap_or(""),
    )?;
    require_non_empty(
        "network-surcharge applicability reference",
        account.network_surcharge_applicability_reference.as_deref().unwrap_or(""),
    )?;
    Ok(())
}

pub fn season_for_month(month: u8) -> Result<Season, BillingError> {
    match month {
        1..=5 | 9..=12 => Ok(Season::Summer),
        6..=8 => Ok(Season::Winter),
        _ => Err(BillingError::InvalidMonth(month)),
    }
}

fn period_at_minute(
    day: DayClass,
    minute: u16,
    saturday_18_to_20_confirmed: bool,
) -> Result<EnergyPeriod, BillingError> {
    if minute >= 1440 {
        return Err(BillingError::InvalidLocalMinute(minute));
    }
    match day {
        DayClass::Weekday => Ok(match minute {
            0..=359 => EnergyPeriod::OffPeak,
            360..=419 => EnergyPeriod::Standard,
            420..=599 => EnergyPeriod::Peak,
            600..=1079 => EnergyPeriod::Standard,
            1080..=1199 => EnergyPeriod::Peak,
            1200..=1319 => EnergyPeriod::Standard,
            _ => EnergyPeriod::OffPeak,
        }),
        DayClass::SaturdayOrPublicHoliday => {
            if (1080..1200).contains(&minute) && !saturday_18_to_20_confirmed {
                return Err(BillingError::SaturdayEndpointNeedsConfirmation);
            }
            Ok(match minute {
                0..=419 => EnergyPeriod::OffPeak,
                420..=719 => EnergyPeriod::Standard,
                720..=1079 => EnergyPeriod::OffPeak,
                1080..=1199 => EnergyPeriod::Standard,
                _ => EnergyPeriod::OffPeak,
            })
        }
        DayClass::Sunday => Ok(match minute {
            0..=1019 => EnergyPeriod::OffPeak,
            1020..=1139 => EnergyPeriod::Standard,
            _ => EnergyPeriod::OffPeak,
        }),
    }
}

pub fn classify_interval(
    interval: MeteredInterval,
    saturday_18_to_20_confirmed: bool,
) -> Result<(Season, EnergyPeriod), BillingError> {
    let season = season_for_month(interval.month)?;
    if interval.local_start_minute >= 1440 {
        return Err(BillingError::InvalidLocalMinute(interval.local_start_minute));
    }
    if interval.duration_minutes == 0 || interval.duration_minutes > 1440 {
        return Err(BillingError::InvalidIntervalDuration(interval.duration_minutes));
    }
    let end = interval.local_start_minute as u32 + interval.duration_minutes as u32;
    if end > 1440 {
        return Err(BillingError::IntervalCrossesMidnight);
    }
    let first = period_at_minute(
        interval.day_class,
        interval.local_start_minute,
        saturday_18_to_20_confirmed,
    )?;
    for minute in (interval.local_start_minute as u32 + 1)..end {
        let current = period_at_minute(
            interval.day_class,
            minute as u16,
            saturday_18_to_20_confirmed,
        )?;
        if current != first {
            return Err(BillingError::IntervalCrossesTariffBoundary);
        }
    }
    Ok((season, first))
}

fn tariff_rates(voltage: VoltageClass) -> (f64, f64, f64) {
    match voltage {
        VoltageClass::Low => (2444.32, 2185.32, 461.28),
        VoltageClass::Medium => (3360.96, 9347.51, 431.11),
        VoltageClass::High => (42915.09, 43848.71, 400.93),
    }
}

fn energy_rate_cents(season: Season, period: EnergyPeriod) -> f64 {
    match (season, period) {
        (Season::Summer, EnergyPeriod::Peak) => 322.00,
        (Season::Summer, EnergyPeriod::Standard) => 242.42,
        (Season::Summer, EnergyPeriod::OffPeak) => 186.35,
        (Season::Winter, EnergyPeriod::Peak) => 766.24,
        (Season::Winter, EnergyPeriod::Standard) => 292.56,
        (Season::Winter, EnergyPeriod::OffPeak) => 200.44,
    }
}

fn validate_input_evidence(input: &BillingInput) -> Result<(), BillingError> {
    if input.evidence.source_profile_id != SOURCE_PROFILE_ID {
        return Err(BillingError::WrongSourceProfile {
            supplied: input.evidence.source_profile_id.clone(),
        });
    }
    require_non_empty("account ID", &input.evidence.account_id)?;
    require_non_empty("tariff approval reference", &input.evidence.tariff_approval_reference)?;
    if input.account.approval_reference.as_deref()
        != Some(input.evidence.tariff_approval_reference.as_str())
    {
        return Err(BillingError::EvidenceMismatch("City Power approval reference"));
    }
    require_non_empty("meter boundary reference", &input.evidence.meter_boundary_reference)?;
    require_non_empty("meter calibration reference", &input.evidence.meter_calibration_reference)?;
    require_non_empty("calendar mapping reference", &input.evidence.calendar_mapping_reference)?;
    let digest = input.evidence.input_manifest_digest.as_str();
    if digest.len() != 64
        || !digest
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(BillingError::InvalidInputDigest);
    }
    Ok(())
}

pub fn evaluate_bill(input: &BillingInput) -> Result<BillingReport, BillingError> {
    validate_account(&input.account)?;
    validate_input_evidence(input)?;
    if input.intervals.is_empty() {
        return Err(BillingError::NoIntervals);
    }
    require_finite_non_negative("power factor", input.power_factor)?;
    if input.power_factor > 1.0 {
        return Err(BillingError::InvalidValue {
            name: "power factor",
            value: input.power_factor,
        });
    }
    require_finite_non_negative("reactive energy kVArh", input.reactive_energy_kvarh)?;
    if let Some(value) = input.assessed_business_lpu_surcharge_zar {
        require_finite_non_negative("assessed Business/LPU surcharge", value)?;
        require_non_empty(
            "Business/LPU invoice-line reference",
            input.business_lpu_line_reference.as_deref().unwrap_or(""),
        )?;
    }
    if let Some(value) = input.assessed_vat_zar {
        require_finite_non_negative("assessed VAT", value)?;
        require_non_empty("VAT invoice-line reference", input.vat_line_reference.as_deref().unwrap_or(""))?;
    }

    let mut period_energy = PeriodEnergy {
        peak_kwh: 0.0,
        standard_kwh: 0.0,
        off_peak_kwh: 0.0,
    };
    let mut measured_peak_demand_kva: f64 = 0.0;
    let mut energy_charge_zar = 0.0;
    for interval in &input.intervals {
        require_finite_non_negative("interval active energy kWh", interval.active_energy_kwh)?;
        require_finite_non_negative("interval measured demand kVA", interval.measured_demand_kva)?;
        let (season, period) = classify_interval(*interval, input.saturday_18_to_20_confirmed)?;
        match period {
            EnergyPeriod::Peak => period_energy.peak_kwh += interval.active_energy_kwh,
            EnergyPeriod::Standard => period_energy.standard_kwh += interval.active_energy_kwh,
            EnergyPeriod::OffPeak => period_energy.off_peak_kwh += interval.active_energy_kwh,
        }
        energy_charge_zar += interval.active_energy_kwh * energy_rate_cents(season, period) / 100.0;
        measured_peak_demand_kva = measured_peak_demand_kva.max(interval.measured_demand_kva);
    }

    let total_active_energy_kwh =
        period_energy.peak_kwh + period_energy.standard_kwh + period_energy.off_peak_kwh;
    let mut prior_peaks = input.preceding_12_month_peak_kva;
    for value in prior_peaks {
        require_finite_non_negative("preceding 12-month peak demand kVA", value)?;
    }
    prior_peaks.sort_by(f64::total_cmp);
    let top_three_average = (prior_peaks[11] + prior_peaks[10] + prior_peaks[9]) / 3.0;
    let ratchet_floor_kva = DEMAND_RATCHET_FRACTION * top_three_average;
    let billed_demand_kva = measured_peak_demand_kva
        .max(MINIMUM_BILLED_DEMAND_KVA)
        .max(ratchet_floor_kva);

    let (service, network, demand_rate) = tariff_rates(input.voltage_class);
    let demand_charge_zar = billed_demand_kva * demand_rate;
    let network_surcharge_zar = if input.account.network_surcharge_applies {
        total_active_energy_kwh * NETWORK_SURCHARGE_CENTS_PER_KWH / 100.0
    } else {
        0.0
    };

    let reactive_energy_excess_kvarh = if input.power_factor < REACTIVE_PF_THRESHOLD {
        (input.reactive_energy_kvarh - REACTIVE_ENERGY_THRESHOLD * total_active_energy_kwh).max(0.0)
    } else {
        0.0
    };
    let reactive_energy_charge_zar =
        reactive_energy_excess_kvarh * REACTIVE_ENERGY_RATE_CENTS_PER_KVARH / 100.0;

    let known_component_subtotal_zar = round_currency(
        energy_charge_zar
            + service
            + network
            + demand_charge_zar
            + network_surcharge_zar
            + reactive_energy_charge_zar,
    );
    let invoice_reconciliation_complete = input.assessed_business_lpu_surcharge_zar.is_some()
        && input.assessed_vat_zar.is_some();
    let full_invoice_total_zar = if invoice_reconciliation_complete {
        Some(round_currency(
            known_component_subtotal_zar
                + input.assessed_business_lpu_surcharge_zar.unwrap_or(0.0)
                + input.assessed_vat_zar.unwrap_or(0.0),
        ))
    } else {
        None
    };

    Ok(BillingReport {
        evaluator_version: EVALUATOR_VERSION,
        source_profile_id: SOURCE_PROFILE_ID,
        account_id: input.evidence.account_id.clone(),
        input_manifest_digest: input.evidence.input_manifest_digest.clone(),
        tariff_approval_reference: input.evidence.tariff_approval_reference.clone(),
        meter_boundary_reference: input.evidence.meter_boundary_reference.clone(),
        meter_calibration_reference: input.evidence.meter_calibration_reference.clone(),
        calendar_mapping_reference: input.evidence.calendar_mapping_reference.clone(),
        season_energy: period_energy,
        total_active_energy_kwh: round_currency(total_active_energy_kwh),
        energy_charge_zar: round_currency(energy_charge_zar),
        monthly_service_charge_zar: service,
        monthly_network_charge_zar: network,
        measured_peak_demand_kva: round_currency(measured_peak_demand_kva),
        ratchet_floor_kva: round_currency(ratchet_floor_kva),
        billed_demand_kva: round_currency(billed_demand_kva),
        demand_charge_zar: round_currency(demand_charge_zar),
        network_surcharge_zar: round_currency(network_surcharge_zar),
        reactive_energy_excess_kvarh: round_currency(reactive_energy_excess_kvarh),
        reactive_energy_charge_zar: round_currency(reactive_energy_charge_zar),
        assessed_business_lpu_surcharge_zar: input.assessed_business_lpu_surcharge_zar,
        assessed_vat_zar: input.assessed_vat_zar,
        known_component_subtotal_zar,
        full_invoice_total_zar,
        invoice_reconciliation_complete,
        claim_ceiling: "synthetic/account-supplied component reference only; not a production invoice or savings claim",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn account() -> AccountEvidence {
        AccountEvidence {
            utility: Utility::CityPower,
            supply_capacity_kva: 125.0,
            city_power_approved: true,
            approval_reference: Some("SYNTHETIC-CP-APPROVAL".to_owned()),
            load_shift_qualification_reference: Some("SYNTHETIC-LOAD-SHIFT-QUALIFICATION".to_owned()),
            legacy_demand_contract_before_1999: false,
            network_surcharge_applies: true,
            network_surcharge_applicability_reference: Some("SYNTHETIC-ACCOUNT-CHARGE-STATUS".to_owned()),
        }
    }

    fn evidence() -> EvidenceReferences {
        EvidenceReferences {
            source_profile_id: SOURCE_PROFILE_ID.to_owned(),
            account_id: "SYNTHETIC-ACCOUNT-001".to_owned(),
            input_manifest_digest: "a".repeat(64),
            tariff_approval_reference: "SYNTHETIC-CP-APPROVAL".to_owned(),
            meter_boundary_reference: "SYNTHETIC-PCC-METER".to_owned(),
            meter_calibration_reference: "SYNTHETIC-METER-CALIBRATION".to_owned(),
            calendar_mapping_reference: "SYNTHETIC-CALENDAR-MAP".to_owned(),
        }
    }

    fn input_with(intervals: Vec<MeteredInterval>) -> BillingInput {
        BillingInput {
            account: account(),
            voltage_class: VoltageClass::Low,
            intervals,
            preceding_12_month_peak_kva: [80.0, 90.0, 100.0, 95.0, 110.0, 120.0, 125.0, 130.0, 115.0, 105.0, 100.0, 90.0],
            power_factor: 0.98,
            reactive_energy_kvarh: 0.0,
            assessed_business_lpu_surcharge_zar: None,
            assessed_vat_zar: None,
            business_lpu_line_reference: None,
            vat_line_reference: None,
            evidence: evidence(),
            saturday_18_to_20_confirmed: false,
        }
    }

    fn interval(month: u8, day_class: DayClass, minute: u16, duration: u16, kwh: f64, kva: f64) -> MeteredInterval {
        MeteredInterval {
            month,
            day_class,
            local_start_minute: minute,
            duration_minutes: duration,
            active_energy_kwh: kwh,
            measured_demand_kva: kva,
        }
    }

    #[test]
    fn approves_qualified_city_power_account_above_100_kva() {
        assert!(validate_account(&account()).is_ok());
    }

    #[test]
    fn rejects_exactly_100_kva() {
        let mut a = account();
        a.supply_capacity_kva = 100.0;
        assert!(matches!(validate_account(&a), Err(BillingError::SupplyCapacityNotEligible { .. })));
    }

    #[test]
    fn rejects_other_distribution_utility() {
        let mut a = account();
        a.utility = Utility::Other("Other Municipal Utility".to_owned());
        assert_eq!(validate_account(&a), Err(BillingError::WrongUtility));
    }

    #[test]
    fn rejects_missing_city_power_approval() {
        let mut a = account();
        a.city_power_approved = false;
        assert_eq!(validate_account(&a), Err(BillingError::CityPowerApprovalMissing));
    }

    #[test]
    fn legacy_below_threshold_requires_manual_review() {
        let mut a = account();
        a.supply_capacity_kva = 80.0;
        a.legacy_demand_contract_before_1999 = true;
        assert_eq!(validate_account(&a), Err(BillingError::LegacyContractRequiresManualReview));
    }

    #[test]
    fn months_map_to_source_seasons() {
        assert_eq!(season_for_month(1), Ok(Season::Summer));
        assert_eq!(season_for_month(5), Ok(Season::Summer));
        assert_eq!(season_for_month(6), Ok(Season::Winter));
        assert_eq!(season_for_month(8), Ok(Season::Winter));
        assert_eq!(season_for_month(9), Ok(Season::Summer));
        assert!(season_for_month(13).is_err());
    }

    #[test]
    fn weekday_peak_standard_and_offpeak_bands_map() {
        assert_eq!(classify_interval(interval(6, DayClass::Weekday, 7 * 60, 15, 1.0, 1.0), false), Ok((Season::Winter, EnergyPeriod::Peak)));
        assert_eq!(classify_interval(interval(6, DayClass::Weekday, 11 * 60, 15, 1.0, 1.0), false), Ok((Season::Winter, EnergyPeriod::Standard)));
        assert_eq!(classify_interval(interval(6, DayClass::Weekday, 23 * 60, 15, 1.0, 1.0), false), Ok((Season::Winter, EnergyPeriod::OffPeak)));
    }

    #[test]
    fn saturday_evening_needs_confirmation() {
        let result = classify_interval(interval(6, DayClass::SaturdayOrPublicHoliday, 18 * 60, 15, 1.0, 1.0), false);
        assert_eq!(result, Err(BillingError::SaturdayEndpointNeedsConfirmation));
    }

    #[test]
    fn saturday_evening_is_standard_after_explicit_confirmation() {
        assert_eq!(classify_interval(interval(6, DayClass::SaturdayOrPublicHoliday, 18 * 60, 15, 1.0, 1.0), true), Ok((Season::Winter, EnergyPeriod::Standard)));
    }

    #[test]
    fn saturday_20_00_is_offpeak_under_confirmed_schedule() {
        assert_eq!(classify_interval(interval(6, DayClass::SaturdayOrPublicHoliday, 20 * 60, 15, 1.0, 1.0), false), Ok((Season::Winter, EnergyPeriod::OffPeak)));
    }

    #[test]
    fn sunday_bands_map() {
        assert_eq!(classify_interval(interval(6, DayClass::Sunday, 17 * 60, 15, 1.0, 1.0), false), Ok((Season::Winter, EnergyPeriod::Standard)));
        assert_eq!(classify_interval(interval(6, DayClass::Sunday, 20 * 60, 15, 1.0, 1.0), false), Ok((Season::Winter, EnergyPeriod::OffPeak)));
    }

    #[test]
    fn rejects_interval_crossing_peak_boundary() {
        let result = classify_interval(interval(6, DayClass::Weekday, 9 * 60 + 55, 15, 1.0, 1.0), false);
        assert_eq!(result, Err(BillingError::IntervalCrossesTariffBoundary));
    }

    #[test]
    fn rejects_interval_crossing_local_midnight() {
        let result = classify_interval(interval(6, DayClass::Weekday, 23 * 60 + 55, 15, 1.0, 1.0), false);
        assert_eq!(result, Err(BillingError::IntervalCrossesMidnight));
    }

    #[test]
    fn energy_charge_uses_winter_peak_rate() {
        let input = input_with(vec![interval(6, DayClass::Weekday, 8 * 60, 15, 100.0, 95.0)]);
        let report = evaluate_bill(&input).unwrap();
        assert_eq!(report.energy_charge_zar, 766.24);
    }

    #[test]
    fn energy_charge_uses_summer_peak_rate() {
        let input = input_with(vec![interval(9, DayClass::Weekday, 8 * 60, 15, 100.0, 95.0)]);
        let report = evaluate_bill(&input).unwrap();
        assert_eq!(report.energy_charge_zar, 322.0);
    }

    #[test]
    fn energy_charge_uses_offpeak_rate() {
        let input = input_with(vec![interval(6, DayClass::Weekday, 23 * 60, 15, 100.0, 95.0)]);
        let report = evaluate_bill(&input).unwrap();
        assert_eq!(report.energy_charge_zar, 200.44);
    }

    #[test]
    fn demand_floor_applies_at_70_kva() {
        let mut input = input_with(vec![interval(6, DayClass::Weekday, 8 * 60, 15, 1.0, 20.0)]);
        input.preceding_12_month_peak_kva = [0.0; 12];
        let report = evaluate_bill(&input).unwrap();
        assert_eq!(report.billed_demand_kva, 70.0);
    }

    #[test]
    fn eighty_percent_of_prior_top_three_ratchet_binds() {
        let mut input = input_with(vec![interval(6, DayClass::Weekday, 8 * 60, 15, 1.0, 95.0)]);
        input.preceding_12_month_peak_kva = [100.0, 120.0, 125.0, 80.0, 90.0, 85.0, 75.0, 80.0, 88.0, 92.0, 96.0, 99.0];
        let report = evaluate_bill(&input).unwrap();
        assert_eq!(report.ratchet_floor_kva, 92.0);
        assert_eq!(report.billed_demand_kva, 95.0);
    }

    #[test]
    fn ratchet_keeps_billed_demand_unchanged_after_measured_peak_falls() {
        let mut input = input_with(vec![interval(6, DayClass::Weekday, 8 * 60, 15, 1.0, 80.0)]);
        input.preceding_12_month_peak_kva = [125.0, 125.0, 125.0, 80.0, 90.0, 85.0, 75.0, 80.0, 88.0, 92.0, 96.0, 99.0];
        let report = evaluate_bill(&input).unwrap();
        assert_eq!(report.ratchet_floor_kva, 100.0);
        assert_eq!(report.billed_demand_kva, 100.0);
    }

    #[test]
    fn measured_demand_above_ratchet_is_billed() {
        let mut input = input_with(vec![interval(6, DayClass::Weekday, 8 * 60, 15, 1.0, 145.0)]);
        let report = evaluate_bill(&input).unwrap();
        assert_eq!(report.billed_demand_kva, 145.0);
    }

    #[test]
    fn pf_below_threshold_charges_only_reactive_energy_above_30_percent() {
        let mut input = input_with(vec![interval(6, DayClass::Weekday, 8 * 60, 15, 100.0, 95.0)]);
        input.power_factor = 0.95;
        input.reactive_energy_kvarh = 50.0;
        let report = evaluate_bill(&input).unwrap();
        assert_eq!(report.reactive_energy_excess_kvarh, 20.0);
        assert_eq!(report.reactive_energy_charge_zar, 9.25);
    }

    #[test]
    fn pf_exactly_096_has_no_reactive_charge() {
        let mut input = input_with(vec![interval(6, DayClass::Weekday, 8 * 60, 15, 100.0, 95.0)]);
        input.power_factor = 0.96;
        input.reactive_energy_kvarh = 100.0;
        let report = evaluate_bill(&input).unwrap();
        assert_eq!(report.reactive_energy_charge_zar, 0.0);
    }

    #[test]
    fn pf_below_threshold_but_reactive_below_30_percent_has_no_charge() {
        let mut input = input_with(vec![interval(6, DayClass::Weekday, 8 * 60, 15, 100.0, 95.0)]);
        input.power_factor = 0.95;
        input.reactive_energy_kvarh = 20.0;
        let report = evaluate_bill(&input).unwrap();
        assert_eq!(report.reactive_energy_charge_zar, 0.0);
    }

    #[test]
    fn six_cent_network_surcharge_is_separate_from_monthly_network_charge() {
        let input = input_with(vec![interval(6, DayClass::Weekday, 8 * 60, 15, 100.0, 95.0)]);
        let report = evaluate_bill(&input).unwrap();
        assert_eq!(report.network_surcharge_zar, 6.0);
        assert_eq!(report.monthly_network_charge_zar, 2185.32);
    }

    #[test]
    fn network_surcharge_is_omitted_when_account_says_not_applicable() {
        let mut input = input_with(vec![interval(6, DayClass::Weekday, 8 * 60, 15, 100.0, 95.0)]);
        input.account.network_surcharge_applies = false;
        let report = evaluate_bill(&input).unwrap();
        assert_eq!(report.network_surcharge_zar, 0.0);
    }

    #[test]
    fn subtotal_is_not_labelled_total_without_business_surcharge_and_vat() {
        let input = input_with(vec![interval(6, DayClass::Weekday, 8 * 60, 15, 100.0, 95.0)]);
        let report = evaluate_bill(&input).unwrap();
        assert!(!report.invoice_reconciliation_complete);
        assert_eq!(report.full_invoice_total_zar, None);
    }

    #[test]
    fn full_synthetic_invoice_requires_actual_surcharge_and_vat_lines() {
        let mut input = input_with(vec![interval(6, DayClass::Weekday, 8 * 60, 15, 100.0, 95.0)]);
        input.assessed_business_lpu_surcharge_zar = Some(12.34);
        input.business_lpu_line_reference = Some("SYNTHETIC-LPU-LINE".to_owned());
        input.assessed_vat_zar = Some(100.00);
        input.vat_line_reference = Some("SYNTHETIC-VAT-LINE".to_owned());
        let report = evaluate_bill(&input).unwrap();
        assert!(report.invoice_reconciliation_complete);
        assert_eq!(report.full_invoice_total_zar, Some(round_currency(report.known_component_subtotal_zar + 112.34)));
    }

    #[test]
    fn missing_lpu_line_reference_is_rejected() {
        let mut input = input_with(vec![interval(6, DayClass::Weekday, 8 * 60, 15, 100.0, 95.0)]);
        input.assessed_business_lpu_surcharge_zar = Some(1.0);
        assert!(matches!(evaluate_bill(&input), Err(BillingError::MissingEvidence(_))));
    }

    #[test]
    fn source_profile_mismatch_is_rejected() {
        let mut input = input_with(vec![interval(6, DayClass::Weekday, 8 * 60, 15, 1.0, 20.0)]);
        input.evidence.source_profile_id = "STALE-PROFILE".to_owned();
        assert!(matches!(evaluate_bill(&input), Err(BillingError::WrongSourceProfile { .. })));
    }

    #[test]
    fn non_finite_meter_values_are_rejected() {
        let input = input_with(vec![interval(6, DayClass::Weekday, 8 * 60, 15, f64::NAN, 1.0)]);
        assert!(matches!(evaluate_bill(&input), Err(BillingError::InvalidValue { .. })));
    }

    #[test]
    fn negative_reactive_energy_is_rejected() {
        let mut input = input_with(vec![interval(6, DayClass::Weekday, 8 * 60, 15, 1.0, 1.0)]);
        input.reactive_energy_kvarh = -1.0;
        assert!(matches!(evaluate_bill(&input), Err(BillingError::InvalidValue { .. })));
    }

    #[test]
    fn empty_interval_trace_is_rejected() {
        let input = input_with(vec![]);
        assert_eq!(evaluate_bill(&input), Err(BillingError::NoIntervals));
    }
}
