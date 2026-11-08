// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Deterministic City Power 2026/27 Industrial TOU billing-component reference.
//!
//! This is a conformance evaluator for explicitly supplied account, interval,
//! demand-history and evidence inputs. It is not a production billing system,
//! tariff eligibility service, calendar engine, dispatch optimizer or
//! regulatory interpretation engine. Unknown account billing lines remain
//! unknown; they are never guessed.

use core::fmt;

pub const EVALUATOR_VERSION: &str = "0.1.0";
pub const SOURCE_PROFILE_ID: &str = "ZA-ENERGY-MARKET-SIGNAL-2026-27-V0.1";
pub const PROFILE_EFFECTIVE_FROM: &str = "2026-07";
pub const PROFILE_EFFECTIVE_THROUGH: &str = "2027-06";
pub const CITY_POWER_UTILITY_ID: &str = "CITY_POWER_JOHANNESBURG";

const TOU_ELIGIBILITY_THRESHOLD_KVA: f64 = 100.0;
const MINIMUM_BILLED_DEMAND_KVA: f64 = 70.0;
const DEMAND_RATCHET_FRACTION: f64 = 0.80;
/// Current City Power demand-interval comparator; the account tariff/meter must confirm it.
const DEMAND_HISTORY_INTERVAL_MINUTES: u16 = 30;
const REACTIVE_ENERGY_THRESHOLD_FRACTION: f64 = 0.30;
const NETWORK_SURCHARGE_R_PER_KWH: f64 = 0.06;
const REACTIVE_ENERGY_R_PER_KVARH: f64 = 0.4625;

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
pub enum EnergyBand {
    Peak,
    Standard,
    OffPeak,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvoiceStatus {
    Complete,
    IncompleteInvoiceReconciliation,
}

impl InvoiceStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Complete => "COMPLETE",
            Self::IncompleteInvoiceReconciliation => "INCOMPLETE_INVOICE_RECONCILIATION",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CalcError {
    WrongProfileId,
    WrongProfileVersion,
    OutsideProfilePeriod,
    InvalidBillingMonth,
    WrongUtility,
    InvalidSupplyCapacity,
    SupplyCapacityNotAbove100Kva,
    ApprovalMissing,
    ApprovalReferenceMissing,
    LegacyAccountRequiresManualReview,
    MissingEvidence(&'static str),
    InvalidDate,
    DateOutsideBillingMonth,
    DayClassDoesNotMatchDate,
    PublicHolidayEvidenceMissing,
    InvalidInterval,
    IntervalCrossesMidnight,
    IntervalCrossesPriceBoundary,
    AmbiguousSaturdayEndpoint,
    SaturdayConfirmationReferenceMissing,
    IntervalOrderOrOverlap,
    IncompleteIntervalCoverage,
    InconsistentDayClassification,
    SeasonDoesNotMatchBillingMonth,
    NoIntervals,
    IncompleteIntervalSeries,
    InvalidMeasurement(&'static str),
    ActiveEnergyMismatch {
        interval_kwh: f64,
        declared_kwh: f64,
    },
    IncompleteDemandHistory,
    DemandHistoryObservationMismatch,
    UnsupportedDemandHistoryInterval,
    InvalidPowerFactor,
    InvalidAssessedLine(&'static str),
}

impl fmt::Display for CalcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongProfileId => write!(f, "wrong or unsupported source profile ID"),
            Self::WrongProfileVersion => write!(f, "wrong or unsupported source profile version"),
            Self::OutsideProfilePeriod => {
                write!(f, "billing month is outside the admitted profile period")
            }
            Self::InvalidBillingMonth => write!(f, "billing month must be YYYY-MM"),
            Self::WrongUtility => write!(
                f,
                "City Power Industrial TOU cannot be applied to another utility"
            ),
            Self::InvalidSupplyCapacity => write!(f, "supply capacity must be finite and positive"),
            Self::SupplyCapacityNotAbove100Kva => write!(
                f,
                "Industrial TOU requires supply capacity strictly greater than 100 kVA"
            ),
            Self::ApprovalMissing => write!(f, "affirmative Industrial TOU approval is required"),
            Self::ApprovalReferenceMissing => write!(
                f,
                "approval reference is required when approval is asserted"
            ),
            Self::LegacyAccountRequiresManualReview => write!(
                f,
                "legacy pre-1-July-1999 demand-tariff account requires manual review"
            ),
            Self::MissingEvidence(name) => write!(f, "missing required evidence reference: {name}"),
            Self::InvalidDate => write!(f, "invalid local date; expected YYYY-MM-DD"),
            Self::DateOutsideBillingMonth => {
                write!(f, "interval date does not match the declared billing month")
            }
            Self::DayClassDoesNotMatchDate => {
                write!(
                    f,
                    "declared tariff day class does not match the Gregorian local date"
                )
            }
            Self::PublicHolidayEvidenceMissing => {
                write!(
                    f,
                    "weekday classified as Saturday/public holiday requires date-specific holiday evidence"
                )
            }
            Self::InvalidInterval => write!(
                f,
                "interval must have positive duration and a start within the local day"
            ),
            Self::IntervalCrossesMidnight => write!(
                f,
                "interval crosses midnight; split it before tariff classification"
            ),
            Self::IntervalCrossesPriceBoundary => write!(
                f,
                "interval crosses a tariff price boundary; split it before evaluation"
            ),
            Self::AmbiguousSaturdayEndpoint => write!(
                f,
                "Saturday 18:00-20:00 tariff interval needs explicit confirmation"
            ),
            Self::SaturdayConfirmationReferenceMissing => {
                write!(f, "Saturday 18:00-20:00 confirmation reference is required")
            }
            Self::IntervalOrderOrOverlap => {
                write!(f, "intervals must be chronological and non-overlapping")
            }
            Self::IncompleteIntervalCoverage => write!(
                f,
                "metered intervals must cover the complete billing month contiguously from 00:00 to 24:00"
            ),
            Self::InconsistentDayClassification => write!(
                f,
                "all intervals on a local date must share one tariff day class and public-holiday evidence reference"
            ),
            Self::SeasonDoesNotMatchBillingMonth => {
                write!(f, "season does not match the billing-month calendar")
            }
            Self::NoIntervals => write!(f, "at least one metered interval is required"),
            Self::IncompleteIntervalSeries => write!(
                f,
                "billing-period interval series must be declared complete and carry a coverage evidence reference"
            ),
            Self::InvalidMeasurement(name) => write!(f, "invalid measurement: {name}"),
            Self::ActiveEnergyMismatch {
                interval_kwh,
                declared_kwh,
            } => write!(
                f,
                "interval energy sum {interval_kwh} kWh differs from declared active energy {declared_kwh} kWh"
            ),
            Self::IncompleteDemandHistory => write!(
                f,
                "complete contiguous demand-interval observations for the preceding twelve calendar months are required"
            ),
            Self::DemandHistoryObservationMismatch => write!(
                f,
                "demand history timestamps must exactly cover the preceding twelve months at the admitted interval without gaps or duplicates"
            ),
            Self::UnsupportedDemandHistoryInterval => write!(
                f,
                "demand-history interval is not the admitted 30-minute City Power comparator; confirm account meter/tariff basis"
            ),
            Self::InvalidPowerFactor => write!(f, "power factor must be finite and in (0, 1]"),
            Self::InvalidAssessedLine(name) => write!(
                f,
                "assessed invoice line has invalid amount or missing evidence: {name}"
            ),
        }
    }
}

impl std::error::Error for CalcError {}

#[derive(Debug, Clone)]
pub struct Account {
    pub utility_id: String,
    pub account_reference: String,
    pub supply_capacity_kva: f64,
    pub voltage_class: VoltageClass,
    pub industrial_tou_approved: bool,
    pub approval_reference: Option<String>,
    /// Accounts relying on the pre-1-July-1999 grandfather clause are never
    /// auto-admitted by this v0.1 new-account evaluator.
    pub legacy_pre_1999_demand_tariff: bool,
    /// The 70-1,000 kVA LV service-connection range is recorded separately.
    /// It does not replace the tariff-specific >100 kVA eligibility rule.
    pub service_connection_classification_reference: Option<String>,
}

#[derive(Debug, Clone)]
pub struct MeteredInterval {
    pub local_date: String,
    pub day_class: DayClass,
    pub local_start_minute: u16,
    pub duration_minutes: u16,
    pub energy_kwh: f64,
    pub saturday_evening_confirmed: bool,
    pub saturday_confirmation_reference: Option<String>,
    /// Required when a Monday-Friday date is classified under the
    /// Saturday/public-holiday tariff bands.
    pub public_holiday_evidence_reference: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DemandHistoryPoint {
    /// Africa/Johannesburg local date of the demand averaging interval.
    pub local_date: String,
    /// Local minute from midnight at which the demand interval starts.
    pub local_start_minute: u16,
    /// Mean apparent demand over the complete, account-qualified interval.
    pub measured_demand_kva: f64,
}

#[derive(Debug, Clone)]
pub struct AssessedInvoiceLine {
    pub amount_zar: f64,
    pub evidence_reference: String,
}

#[derive(Debug, Clone)]
pub struct TariffInput {
    pub source_profile_id: String,
    pub profile_version: String,
    pub billing_month: String,
    pub time_zone: String,
    pub account: Account,
    pub intervals: Vec<MeteredInterval>,
    pub interval_series_complete: bool,
    pub interval_coverage_evidence_reference: String,
    pub declared_active_energy_kwh: f64,
    pub measured_monthly_peak_demand_kva: f64,
    pub preceding_12_month_demand_history: Vec<DemandHistoryPoint>,
    /// Confirmed apparent-demand averaging interval for the source account.
    pub demand_history_interval_minutes: u16,
    pub demand_history_interval_evidence_reference: String,
    pub demand_history_coverage_evidence_reference: String,
    pub power_factor: f64,
    pub measured_reactive_energy_kvarh: f64,
    /// None means the actual account's network-surcharge treatment is unknown.
    pub network_surcharge_applies: Option<bool>,
    pub network_surcharge_evidence_reference: Option<String>,
    /// This is an externally assessed amount. The evaluator never computes 2%
    /// over a guessed or inferred billing base.
    pub business_lpu_surcharge: Option<AssessedInvoiceLine>,
    /// VAT is supplied from the actual invoice/tax evidence, never hardcoded.
    pub vat: Option<AssessedInvoiceLine>,
    pub calendar_mapping_reference: String,
    pub timebase_reference: String,
    pub meter_boundary_reference: String,
    pub meter_calibration_reference: String,
    pub input_manifest_reference: String,
}

#[derive(Debug, Clone)]
pub struct TariffRates {
    pub service_charge_zar_month: f64,
    pub monthly_network_charge_zar: f64,
    pub demand_charge_zar_per_kva: f64,
}

pub fn rates_for_voltage(voltage: VoltageClass) -> TariffRates {
    match voltage {
        VoltageClass::Low => TariffRates {
            service_charge_zar_month: 2_444.32,
            monthly_network_charge_zar: 2_185.32,
            demand_charge_zar_per_kva: 461.28,
        },
        VoltageClass::Medium => TariffRates {
            service_charge_zar_month: 3_360.96,
            monthly_network_charge_zar: 9_347.51,
            demand_charge_zar_per_kva: 431.11,
        },
        VoltageClass::High => TariffRates {
            service_charge_zar_month: 42_915.09,
            monthly_network_charge_zar: 43_848.71,
            demand_charge_zar_per_kva: 400.93,
        },
    }
}

pub fn season_for_month(month: u8) -> Result<Season, CalcError> {
    match month {
        9 | 10 | 11 | 12 | 1 | 2 | 3 | 4 | 5 => Ok(Season::Summer),
        6..=8 => Ok(Season::Winter),
        _ => Err(CalcError::InvalidBillingMonth),
    }
}

pub fn energy_rate_cents_per_kwh(season: Season, band: EnergyBand) -> f64 {
    match (season, band) {
        (Season::Summer, EnergyBand::Peak) => 322.00,
        (Season::Summer, EnergyBand::Standard) => 242.42,
        (Season::Summer, EnergyBand::OffPeak) => 186.35,
        (Season::Winter, EnergyBand::Peak) => 766.24,
        (Season::Winter, EnergyBand::Standard) => 292.56,
        (Season::Winter, EnergyBand::OffPeak) => 200.44,
    }
}

fn classify_minute(day: DayClass, minute: u16) -> EnergyBand {
    match day {
        DayClass::Weekday => {
            if (420..600).contains(&minute) || (1_080..1_200).contains(&minute) {
                EnergyBand::Peak
            } else if (360..420).contains(&minute)
                || (600..1_080).contains(&minute)
                || (1_200..1_320).contains(&minute)
            {
                EnergyBand::Standard
            } else {
                EnergyBand::OffPeak
            }
        }
        DayClass::SaturdayOrPublicHoliday => {
            if (420..720).contains(&minute) {
                EnergyBand::Standard
            } else if (720..1_080).contains(&minute)
                || (0..420).contains(&minute)
                || (1_200..1_440).contains(&minute)
            {
                EnergyBand::OffPeak
            } else {
                // 18:00-20:00 is printed as 18h00-20h0 in the official budget.
                // The likely 20:00 endpoint must be admitted by account/source
                // confirmation before this interval can be classified.
                EnergyBand::Standard
            }
        }
        DayClass::Sunday => {
            if (1_020..1_140).contains(&minute) {
                EnergyBand::Standard
            } else {
                EnergyBand::OffPeak
            }
        }
    }
}

pub fn classify_interval(
    day: DayClass,
    start_minute: u16,
    duration_minutes: u16,
    saturday_evening_confirmed: bool,
    saturday_confirmation_reference: Option<&str>,
) -> Result<EnergyBand, CalcError> {
    if start_minute >= 1_440 || duration_minutes == 0 {
        return Err(CalcError::InvalidInterval);
    }
    let end = start_minute as u32 + duration_minutes as u32;
    if end > 1_440 {
        return Err(CalcError::IntervalCrossesMidnight);
    }

    if day == DayClass::SaturdayOrPublicHoliday && start_minute < 1_200 && end > 1_080 {
        if !saturday_evening_confirmed {
            return Err(CalcError::AmbiguousSaturdayEndpoint);
        }
        if saturday_confirmation_reference.is_none_or(|s| s.trim().is_empty()) {
            return Err(CalcError::SaturdayConfirmationReferenceMissing);
        }
    }

    let first = classify_minute(day, start_minute);
    for minute in (u32::from(start_minute) + 1)..end {
        if classify_minute(day, minute as u16) != first {
            return Err(CalcError::IntervalCrossesPriceBoundary);
        }
    }
    Ok(first)
}

#[derive(Debug, Clone)]
pub struct BillBreakdown {
    pub active_energy_kwh: f64,
    pub energy_charge_zar: f64,
    pub service_charge_zar: f64,
    pub monthly_network_charge_zar: f64,
    pub billed_demand_kva: f64,
    pub demand_charge_zar: f64,
    pub reactive_energy_charge_zar: f64,
    pub per_kwh_network_surcharge_zar: Option<f64>,
    pub business_lpu_surcharge_zar: Option<f64>,
    pub vat_zar: Option<f64>,
    pub known_component_subtotal_zar: f64,
    pub invoice_total_zar: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct BillReceipt {
    pub evaluator_version: &'static str,
    pub source_profile_id: String,
    pub profile_version: String,
    pub input_fingerprint_fnv1a64: String,
    pub billing_month: String,
    pub account_reference: String,
    pub voltage_class: VoltageClass,
    pub status: InvoiceStatus,
    pub breakdown: BillBreakdown,
    pub calendar_mapping_reference: String,
    pub timebase_reference: String,
    pub meter_boundary_reference: String,
    pub meter_calibration_reference: String,
    pub input_manifest_reference: String,
    pub claim_ceiling: &'static str,
}

fn nonblank(value: &str) -> bool {
    !value.trim().is_empty()
}

fn round_money(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

fn parse_month(month: &str) -> Result<(i32, u8), CalcError> {
    let mut parts = month.split('-');
    let year = parts.next().ok_or(CalcError::InvalidBillingMonth)?;
    let month_part = parts.next().ok_or(CalcError::InvalidBillingMonth)?;
    if parts.next().is_some() || year.len() != 4 || month_part.len() != 2 {
        return Err(CalcError::InvalidBillingMonth);
    }
    let year: i32 = year.parse().map_err(|_| CalcError::InvalidBillingMonth)?;
    let month: u8 = month_part
        .parse()
        .map_err(|_| CalcError::InvalidBillingMonth)?;
    if !(1..=12).contains(&month) {
        return Err(CalcError::InvalidBillingMonth);
    }
    Ok((year, month))
}

fn parse_date(date: &str) -> Result<(i32, u8, u8), CalcError> {
    let mut parts = date.split('-');
    let year = parts.next().ok_or(CalcError::InvalidDate)?;
    let month = parts.next().ok_or(CalcError::InvalidDate)?;
    let day = parts.next().ok_or(CalcError::InvalidDate)?;
    if parts.next().is_some() || year.len() != 4 || month.len() != 2 || day.len() != 2 {
        return Err(CalcError::InvalidDate);
    }
    let year: i32 = year.parse().map_err(|_| CalcError::InvalidDate)?;
    let month: u8 = month.parse().map_err(|_| CalcError::InvalidDate)?;
    let day: u8 = day.parse().map_err(|_| CalcError::InvalidDate)?;
    if year < 1 || !(1..=12).contains(&month) || day == 0 {
        return Err(CalcError::InvalidDate);
    }
    if day > days_in_month(year, month) {
        return Err(CalcError::InvalidDate);
    }
    Ok((year, month, day))
}

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
    let mut adjusted_year = year;
    let mut adjusted_month = i32::from(month);
    if adjusted_month < 3 {
        adjusted_year -= 1;
        adjusted_month += 12;
    }

    let year_of_century = adjusted_year % 100;
    let zero_based_century = adjusted_year / 100;
    let zeller = (i32::from(day)
        + (13 * (adjusted_month + 1)) / 5
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

fn validate_day_class(
    local_date: &str,
    day_class: DayClass,
    public_holiday_evidence_reference: Option<&str>,
) -> Result<(), CalcError> {
    let (year, month, day) = parse_date(local_date)?;
    let weekday = weekday_monday_zero(year, month, day);

    match (day_class, weekday) {
        (DayClass::Weekday, 0..=4) => Ok(()),
        (DayClass::SaturdayOrPublicHoliday, 5) => Ok(()),
        (DayClass::SaturdayOrPublicHoliday, 0..=4) => {
            if public_holiday_evidence_reference.is_some_and(nonblank) {
                Ok(())
            } else {
                Err(CalcError::PublicHolidayEvidenceMissing)
            }
        }
        (DayClass::Sunday, 6) => Ok(()),
        _ => Err(CalcError::DayClassDoesNotMatchDate),
    }
}

fn month_index(year: i32, month: u8) -> i32 {
    year * 12 + month as i32 - 1
}

fn fingerprint(input: &TariffInput) -> String {
    let mut canonical = String::new();
    use fmt::Write as _;
    let _ = write!(
        canonical,
        "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
        input.source_profile_id,
        input.profile_version,
        input.billing_month,
        input.time_zone,
        input.account.utility_id,
        input.account.account_reference,
        input.account.supply_capacity_kva,
        input.account.voltage_class as u8,
        input.account.industrial_tou_approved,
        input
            .account
            .approval_reference
            .as_deref()
            .unwrap_or("UNKNOWN"),
        input.account.legacy_pre_1999_demand_tariff,
        input.declared_active_energy_kwh,
        input.measured_monthly_peak_demand_kva,
        input.power_factor,
        input.measured_reactive_energy_kvarh
    );
    let _ = write!(
        canonical,
        "|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
        input
            .network_surcharge_applies
            .map(|x| x.to_string())
            .unwrap_or_else(|| "UNKNOWN".into()),
        input
            .business_lpu_surcharge
            .as_ref()
            .map(|x| format!("{}:{}", x.amount_zar, x.evidence_reference))
            .unwrap_or_else(|| "UNKNOWN".into()),
        input
            .vat
            .as_ref()
            .map(|x| format!("{}:{}", x.amount_zar, x.evidence_reference))
            .unwrap_or_else(|| "UNKNOWN".into()),
        input.calendar_mapping_reference,
        input.timebase_reference,
        input.meter_boundary_reference,
        input.meter_calibration_reference,
        input.input_manifest_reference,
        input.interval_series_complete,
        input.interval_coverage_evidence_reference,
        input
            .network_surcharge_evidence_reference
            .as_deref()
            .unwrap_or("UNKNOWN"),
        input
            .account
            .service_connection_classification_reference
            .as_deref()
            .unwrap_or("UNKNOWN"),
        input.preceding_12_month_demand_history.len(),
        input.demand_history_interval_minutes,
        input.demand_history_interval_evidence_reference,
        input.demand_history_coverage_evidence_reference
    );
    for point in &input.preceding_12_month_demand_history {
        let _ = write!(
            canonical,
            "|{}:{}:{}",
            point.local_date, point.local_start_minute, point.measured_demand_kva
        );
    }
    for interval in &input.intervals {
        let _ = write!(
            canonical,
            "|{}:{:?}:{}:{}:{}:{}:{}:{}:{}",
            interval.local_date,
            interval.day_class,
            interval.local_start_minute,
            interval.duration_minutes,
            interval.energy_kwh,
            interval.saturday_evening_confirmed,
            interval
                .saturday_confirmation_reference
                .as_deref()
                .unwrap_or("UNKNOWN"),
            interval
                .public_holiday_evidence_reference
                .as_deref()
                .unwrap_or("UNKNOWN"),
            season_for_date(&interval.local_date)
                .map(|s| format!("{s:?}"))
                .unwrap_or_else(|_| "INVALID".into())
        );
    }
    // Non-cryptographic fingerprint for reproducible input binding. It is not
    // an authenticity signature; callers retain the raw manifest and evidence.
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in canonical.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

fn season_for_date(date: &str) -> Result<Season, CalcError> {
    let (_, month, _) = parse_date(date)?;
    season_for_month(month)
}

fn validate_account(account: &Account) -> Result<(), CalcError> {
    if account.utility_id != CITY_POWER_UTILITY_ID {
        return Err(CalcError::WrongUtility);
    }
    if !account.supply_capacity_kva.is_finite() || account.supply_capacity_kva <= 0.0 {
        return Err(CalcError::InvalidSupplyCapacity);
    }
    if account.legacy_pre_1999_demand_tariff {
        return Err(CalcError::LegacyAccountRequiresManualReview);
    }
    if account.supply_capacity_kva <= TOU_ELIGIBILITY_THRESHOLD_KVA {
        return Err(CalcError::SupplyCapacityNotAbove100Kva);
    }
    if !account.industrial_tou_approved {
        return Err(CalcError::ApprovalMissing);
    }
    if !account.approval_reference.as_deref().is_some_and(nonblank) {
        return Err(CalcError::ApprovalReferenceMissing);
    }
    if !nonblank(&account.account_reference) {
        return Err(CalcError::MissingEvidence("account reference"));
    }
    Ok(())
}

fn validate_demand_history(
    billing_month: &str,
    history: &[DemandHistoryPoint],
    interval_minutes: u16,
    interval_evidence_reference: &str,
    coverage_evidence_reference: &str,
) -> Result<f64, CalcError> {
    if interval_minutes != DEMAND_HISTORY_INTERVAL_MINUTES {
        return Err(CalcError::UnsupportedDemandHistoryInterval);
    }
    if !nonblank(interval_evidence_reference) || !nonblank(coverage_evidence_reference) {
        return Err(CalcError::MissingEvidence("demand-history interval/coverage"));
    }

    let (year, month) = parse_month(billing_month)?;
    let current_index = month_index(year, month);
    let expected_start_index = current_index - 12;
    let intervals_per_day = 1_440 / interval_minutes;
    let mut expected_count = 0_usize;

    // Demand memory is a rolling stream of meter intervals, not twelve monthly
    // summaries. Every local half-hour in the preceding twelve calendar months
    // must be represented exactly once in this v0.1 source comparator.
    for month_offset in 0..12 {
        let absolute = expected_start_index + month_offset;
        let expected_year = absolute.div_euclid(12);
        let expected_month = (absolute.rem_euclid(12) + 1) as u8;
        for day in 1..=days_in_month(expected_year, expected_month) {
            let expected_date = format!("{expected_year:04}-{expected_month:02}-{day:02}");
            for slot in 0..intervals_per_day {
                let position = expected_count;
                let point = history
                    .get(position)
                    .ok_or(CalcError::IncompleteDemandHistory)?;
                let expected_start = slot * interval_minutes;
                if point.local_date != expected_date
                    || point.local_start_minute != expected_start
                    || point.local_start_minute >= 1_440
                    || !point.measured_demand_kva.is_finite()
                    || point.measured_demand_kva < 0.0
                {
                    return Err(CalcError::DemandHistoryObservationMismatch);
                }
                expected_count += 1;
            }
        }
    }

    if history.len() != expected_count {
        return Err(CalcError::IncompleteDemandHistory);
    }

    let mut peaks: Vec<f64> = history.iter().map(|p| p.measured_demand_kva).collect();
    peaks.sort_by(|a, b| b.total_cmp(a));
    let top_three_average = (peaks[0] + peaks[1] + peaks[2]) / 3.0;
    Ok(top_three_average)
}

fn validate_assessed_line(
    line: &Option<AssessedInvoiceLine>,
    name: &'static str,
) -> Result<(), CalcError> {
    if let Some(line) = line {
        if !line.amount_zar.is_finite() || line.amount_zar < 0.0 {
            return Err(CalcError::InvalidAssessedLine(name));
        }
        if !nonblank(&line.evidence_reference) {
            return Err(CalcError::InvalidAssessedLine(name));
        }
    }
    Ok(())
}

pub fn calculate_bill(input: &TariffInput) -> Result<BillReceipt, CalcError> {
    if input.source_profile_id != SOURCE_PROFILE_ID {
        return Err(CalcError::WrongProfileId);
    }
    if input.profile_version != "0.1" {
        return Err(CalcError::WrongProfileVersion);
    }
    let (year, month) = parse_month(&input.billing_month)?;
    if input.billing_month.as_str() < PROFILE_EFFECTIVE_FROM
        || input.billing_month.as_str() > PROFILE_EFFECTIVE_THROUGH
    {
        return Err(CalcError::OutsideProfilePeriod);
    }
    if input.time_zone != "Africa/Johannesburg" {
        return Err(CalcError::MissingEvidence("Africa/Johannesburg timebase"));
    }
    validate_account(&input.account)?;
    for (name, reference) in [
        (
            "calendar mapping",
            input.calendar_mapping_reference.as_str(),
        ),
        ("timebase", input.timebase_reference.as_str()),
        ("meter boundary", input.meter_boundary_reference.as_str()),
        (
            "meter calibration",
            input.meter_calibration_reference.as_str(),
        ),
        (
            "demand-history interval evidence",
            input.demand_history_interval_evidence_reference.as_str(),
        ),
        (
            "demand-history coverage evidence",
            input.demand_history_coverage_evidence_reference.as_str(),
        ),
        ("input manifest", input.input_manifest_reference.as_str()),
    ] {
        if !nonblank(reference) {
            return Err(CalcError::MissingEvidence(name));
        }
    }
    if input.intervals.is_empty() {
        return Err(CalcError::NoIntervals);
    }
    if !input.interval_series_complete || !nonblank(&input.interval_coverage_evidence_reference) {
        return Err(CalcError::IncompleteIntervalSeries);
    }
    if input.network_surcharge_applies.is_some()
        && !input
            .network_surcharge_evidence_reference
            .as_deref()
            .is_some_and(nonblank)
    {
        return Err(CalcError::MissingEvidence(
            "network surcharge account treatment",
        ));
    }
    if !input.declared_active_energy_kwh.is_finite() || input.declared_active_energy_kwh < 0.0 {
        return Err(CalcError::InvalidMeasurement("declared active energy"));
    }
    if !input.measured_monthly_peak_demand_kva.is_finite()
        || input.measured_monthly_peak_demand_kva < 0.0
    {
        return Err(CalcError::InvalidMeasurement(
            "monthly measured peak demand",
        ));
    }
    if !input.power_factor.is_finite() || input.power_factor <= 0.0 || input.power_factor > 1.0 {
        return Err(CalcError::InvalidPowerFactor);
    }
    if !input.measured_reactive_energy_kvarh.is_finite()
        || input.measured_reactive_energy_kvarh < 0.0
    {
        return Err(CalcError::InvalidMeasurement("reactive energy"));
    }
    validate_assessed_line(&input.business_lpu_surcharge, "Business/LPU surcharge")?;
    validate_assessed_line(&input.vat, "VAT")?;

    let mut interval_energy = 0.0;
    let mut energy_charge = 0.0;
    let mut prior_date = String::new();
    let mut prior_end = 0_u16;
    let mut prior_day_class: Option<DayClass> = None;
    let mut prior_public_holiday_evidence: Option<String> = None;

    for interval in &input.intervals {
        let (date_year, date_month, date_day) = parse_date(&interval.local_date)?;
        if date_year != year || date_month != month {
            return Err(CalcError::DateOutsideBillingMonth);
        }
        validate_day_class(
            &interval.local_date,
            interval.day_class,
            interval.public_holiday_evidence_reference.as_deref(),
        )?;
        if season_for_date(&interval.local_date)? != season_for_month(month)? {
            return Err(CalcError::SeasonDoesNotMatchBillingMonth);
        }
        if !interval.energy_kwh.is_finite() || interval.energy_kwh < 0.0 {
            return Err(CalcError::InvalidMeasurement("interval energy"));
        }
        if prior_date.is_empty() {
            if date_day != 1 || interval.local_start_minute != 0 {
                return Err(CalcError::IncompleteIntervalCoverage);
            }
        } else if interval.local_date < prior_date {
            return Err(CalcError::IntervalOrderOrOverlap);
        } else if interval.local_date == prior_date {
            if interval.day_class != prior_day_class.unwrap_or(interval.day_class)
                || interval.public_holiday_evidence_reference.as_deref()
                    != prior_public_holiday_evidence.as_deref()
            {
                return Err(CalcError::InconsistentDayClassification);
            }
            if interval.local_start_minute < prior_end {
                return Err(CalcError::IntervalOrderOrOverlap);
            }
            if interval.local_start_minute > prior_end {
                return Err(CalcError::IncompleteIntervalCoverage);
            }
        } else {
            let (_, _, previous_day) = parse_date(&prior_date)?;
            if date_day != previous_day + 1
                || prior_end != 1_440
                || interval.local_start_minute != 0
            {
                return Err(CalcError::IncompleteIntervalCoverage);
            }
        }

        let band = classify_interval(
            interval.day_class,
            interval.local_start_minute,
            interval.duration_minutes,
            interval.saturday_evening_confirmed,
            interval.saturday_confirmation_reference.as_deref(),
        )?;
        let rate = energy_rate_cents_per_kwh(season_for_month(month)?, band) / 100.0;
        interval_energy += interval.energy_kwh;
        energy_charge += interval.energy_kwh * rate;
        prior_date = interval.local_date.clone();
        prior_end = interval.local_start_minute + interval.duration_minutes;
        prior_day_class = Some(interval.day_class);
        prior_public_holiday_evidence = interval.public_holiday_evidence_reference.clone();
    }

    let (_, _, last_covered_day) = parse_date(&prior_date)?;
    if last_covered_day != days_in_month(year, month) || prior_end != 1_440 {
        return Err(CalcError::IncompleteIntervalCoverage);
    }

    if (interval_energy - input.declared_active_energy_kwh).abs() > 1e-6 {
        return Err(CalcError::ActiveEnergyMismatch {
            interval_kwh: interval_energy,
            declared_kwh: input.declared_active_energy_kwh,
        });
    }

    let top_three_average = validate_demand_history(
        &input.billing_month,
        &input.preceding_12_month_demand_history,
        input.demand_history_interval_minutes,
        &input.demand_history_interval_evidence_reference,
        &input.demand_history_coverage_evidence_reference,
    )?;
    let ratchet_floor = DEMAND_RATCHET_FRACTION * top_three_average;
    let billed_demand_kva = input
        .measured_monthly_peak_demand_kva
        .max(MINIMUM_BILLED_DEMAND_KVA)
        .max(ratchet_floor);

    let rates = rates_for_voltage(input.account.voltage_class);
    let service_charge = round_money(rates.service_charge_zar_month);
    let monthly_network_charge = round_money(rates.monthly_network_charge_zar);
    let demand_charge = round_money(billed_demand_kva * rates.demand_charge_zar_per_kva);

    let reactive_excess_kvarh = if input.power_factor < 0.96 {
        (input.measured_reactive_energy_kvarh
            - REACTIVE_ENERGY_THRESHOLD_FRACTION * input.declared_active_energy_kwh)
            .max(0.0)
    } else {
        0.0
    };
    let reactive_charge = round_money(reactive_excess_kvarh * REACTIVE_ENERGY_R_PER_KVARH);
    let energy_charge = round_money(energy_charge);

    let per_kwh_network_surcharge = input.network_surcharge_applies.map(|applies| {
        if applies {
            round_money(input.declared_active_energy_kwh * NETWORK_SURCHARGE_R_PER_KWH)
        } else {
            0.0
        }
    });

    let surcharge_amount = input
        .business_lpu_surcharge
        .as_ref()
        .map(|line| round_money(line.amount_zar));
    let vat_amount = input.vat.as_ref().map(|line| round_money(line.amount_zar));

    let mut subtotal =
        energy_charge + service_charge + monthly_network_charge + demand_charge + reactive_charge;
    if let Some(amount) = per_kwh_network_surcharge {
        subtotal += amount;
    }
    if let Some(amount) = surcharge_amount {
        subtotal += amount;
    }
    if let Some(amount) = vat_amount {
        subtotal += amount;
    }
    subtotal = round_money(subtotal);

    let complete =
        per_kwh_network_surcharge.is_some() && surcharge_amount.is_some() && vat_amount.is_some();
    let status = if complete {
        InvoiceStatus::Complete
    } else {
        InvoiceStatus::IncompleteInvoiceReconciliation
    };

    Ok(BillReceipt {
        evaluator_version: EVALUATOR_VERSION,
        source_profile_id: input.source_profile_id.clone(),
        profile_version: input.profile_version.clone(),
        input_fingerprint_fnv1a64: fingerprint(input),
        billing_month: input.billing_month.clone(),
        account_reference: input.account.account_reference.clone(),
        voltage_class: input.account.voltage_class,
        status,
        breakdown: BillBreakdown {
            active_energy_kwh: round_money(interval_energy),
            energy_charge_zar: energy_charge,
            service_charge_zar: service_charge,
            monthly_network_charge_zar: monthly_network_charge,
            billed_demand_kva: round_money(billed_demand_kva),
            demand_charge_zar: demand_charge,
            reactive_energy_charge_zar: reactive_charge,
            per_kwh_network_surcharge_zar: per_kwh_network_surcharge,
            business_lpu_surcharge_zar: surcharge_amount,
            vat_zar: vat_amount,
            known_component_subtotal_zar: subtotal,
            invoice_total_zar: if complete { Some(subtotal) } else { None },
        },
        calendar_mapping_reference: input.calendar_mapping_reference.clone(),
        timebase_reference: input.timebase_reference.clone(),
        meter_boundary_reference: input.meter_boundary_reference.clone(),
        meter_calibration_reference: input.meter_calibration_reference.clone(),
        input_manifest_reference: input.input_manifest_reference.clone(),
        claim_ceiling: "Synthetic/account-supplied tariff-component conformance only; not a customer invoice or saving claim.",
    })
}

impl BillReceipt {
    pub fn to_json(&self) -> String {
        let optional_json = |value: Option<f64>| {
            value
                .map(|v| format!("{v:.2}"))
                .unwrap_or_else(|| "null".to_owned())
        };
        let voltage = match self.voltage_class {
            VoltageClass::Low => "LOW",
            VoltageClass::Medium => "MEDIUM",
            VoltageClass::High => "HIGH",
        };
        let result = if self.status == InvoiceStatus::Complete {
            "pass"
        } else {
            "incomplete"
        };
        format!(
            concat!(
                "{{",
                "\"result\":\"{}\",",
                "\"evaluator_version\":\"{}\",",
                "\"source_profile_id\":\"{}\",",
                "\"profile_version\":\"{}\",",
                "\"input_fingerprint_fnv1a64\":\"{}\",",
                "\"billing_month\":\"{}\",",
                "\"account_reference\":\"{}\",",
                "\"voltage_class\":\"{}\",",
                "\"status\":\"{}\",",
                "\"breakdown\":{{",
                "\"active_energy_kwh\":{:.2},",
                "\"energy_charge_zar\":{:.2},",
                "\"service_charge_zar\":{:.2},",
                "\"monthly_network_charge_zar\":{:.2},",
                "\"billed_demand_kva\":{:.2},",
                "\"demand_charge_zar\":{:.2},",
                "\"reactive_energy_charge_zar\":{:.2},",
                "\"per_kwh_network_surcharge_zar\":{},",
                "\"business_lpu_surcharge_zar\":{},",
                "\"vat_zar\":{},",
                "\"known_component_subtotal_zar\":{:.2},",
                "\"invoice_total_zar\":{}",
                "}},",
                "\"evidence\":{{",
                "\"calendar_mapping_reference\":\"{}\",",
                "\"timebase_reference\":\"{}\",",
                "\"meter_boundary_reference\":\"{}\",",
                "\"meter_calibration_reference\":\"{}\",",
                "\"input_manifest_reference\":\"{}\"",
                "}},",
                "\"claim_ceiling\":\"{}\"",
                "}}"
            ),
            result,
            self.evaluator_version,
            json_escape(&self.source_profile_id),
            json_escape(&self.profile_version),
            self.input_fingerprint_fnv1a64,
            json_escape(&self.billing_month),
            json_escape(&self.account_reference),
            voltage,
            self.status.as_str(),
            self.breakdown.active_energy_kwh,
            self.breakdown.energy_charge_zar,
            self.breakdown.service_charge_zar,
            self.breakdown.monthly_network_charge_zar,
            self.breakdown.billed_demand_kva,
            self.breakdown.demand_charge_zar,
            self.breakdown.reactive_energy_charge_zar,
            optional_json(self.breakdown.per_kwh_network_surcharge_zar),
            optional_json(self.breakdown.business_lpu_surcharge_zar),
            optional_json(self.breakdown.vat_zar),
            self.breakdown.known_component_subtotal_zar,
            optional_json(self.breakdown.invoice_total_zar),
            json_escape(&self.calendar_mapping_reference),
            json_escape(&self.timebase_reference),
            json_escape(&self.meter_boundary_reference),
            json_escape(&self.meter_calibration_reference),
            json_escape(&self.input_manifest_reference),
            json_escape(self.claim_ceiling),
        )
    }
}

fn json_escape(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len() + 2);
    for ch in value.chars() {
        match ch {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            c if c.is_control() => escaped.push(' '),
            c => escaped.push(c),
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::*;

    fn history_for(billing_month: &str, peak: f64) -> Vec<DemandHistoryPoint> {
        let (year, month) = parse_month(billing_month).unwrap();
        let current = month_index(year, month);
        let mut history = Vec::new();
        for month_offset in 0..12 {
            let absolute = current - 12 + month_offset;
            let history_year = absolute.div_euclid(12);
            let history_month = (absolute.rem_euclid(12) + 1) as u8;
            for day in 1..=days_in_month(history_year, history_month) {
                let local_date = format!("{history_year:04}-{history_month:02}-{day:02}");
                for local_start_minute in
                    (0..1_440).step_by(usize::from(DEMAND_HISTORY_INTERVAL_MINUTES))
                {
                    history.push(DemandHistoryPoint {
                        local_date: local_date.clone(),
                        local_start_minute: local_start_minute as u16,
                        measured_demand_kva: peak,
                    });
                }
            }
        }
        history
    }

    fn fixture() -> TariffInput {
        let mut intervals = Vec::with_capacity(31 * 96);
        let mut declared_active_energy_kwh = 0.0;

        for day in 1..=31_u8 {
            let local_date = format!("2026-07-{day:02}");
            let weekday = weekday_monday_zero(2026, 7, day);
            let day_class = match weekday {
                5 => DayClass::SaturdayOrPublicHoliday,
                6 => DayClass::Sunday,
                _ => DayClass::Weekday,
            };

            for slot in 0..96_u16 {
                let local_start_minute = slot * 15;
                let energy_kwh = if day == 1 && local_start_minute == 420 {
                    10.0
                } else {
                    0.0
                };
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
                approval_reference: Some("SYNTHETIC-APPROVAL-REF".to_owned()),
                legacy_pre_1999_demand_tariff: false,
                service_connection_classification_reference: Some(
                    "SYNTHETIC-LV-LPU-SERVICE-CLASS".to_owned(),
                ),
            },
            intervals,
            interval_series_complete: true,
            interval_coverage_evidence_reference: "SYNTHETIC-COMPLETE-INTERVAL-COVERAGE".to_owned(),
            declared_active_energy_kwh,
            measured_monthly_peak_demand_kva: 95.0,
            preceding_12_month_demand_history: history_for("2026-07", 125.0),
            demand_history_interval_minutes: DEMAND_HISTORY_INTERVAL_MINUTES,
            demand_history_interval_evidence_reference: "SYNTHETIC-30-MINUTE-METER-INTERVAL".to_owned(),
            demand_history_coverage_evidence_reference: "SYNTHETIC-COMPLETE-PRECEDING-12-MONTH-DEMAND-TRACE".to_owned(),
            power_factor: 0.95,
            measured_reactive_energy_kvarh: 5.0,
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
            calendar_mapping_reference: "SYNTHETIC-CALENDAR-MAP".to_owned(),
            timebase_reference: "SAST-AFRICA-JOHANNESBURG".to_owned(),
            meter_boundary_reference: "SYNTHETIC-PCC-METER".to_owned(),
            meter_calibration_reference: "SYNTHETIC-METER-CALIBRATION".to_owned(),
            input_manifest_reference: "SYNTHETIC-INPUT-MANIFEST".to_owned(),
        }
    }

    fn close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1e-8,
            "actual={actual}, expected={expected}"
        );
    }

    #[test]
    fn approved_city_power_account_above_100_kva_is_eligible() {
        assert!(calculate_bill(&fixture()).is_ok());
    }

    #[test]
    fn exactly_100_kva_is_not_eligible() {
        let mut input = fixture();
        input.account.supply_capacity_kva = 100.0;
        assert_eq!(
            calculate_bill(&input).unwrap_err(),
            CalcError::SupplyCapacityNotAbove100Kva
        );
    }

    #[test]
    fn service_connection_70_kva_boundary_does_not_imply_tou_eligibility() {
        let mut input = fixture();
        input.account.supply_capacity_kva = 70.0;
        assert_eq!(
            calculate_bill(&input).unwrap_err(),
            CalcError::SupplyCapacityNotAbove100Kva
        );
    }

    #[test]
    fn other_utility_is_rejected() {
        let mut input = fixture();
        input.account.utility_id = "ESKOM".to_owned();
        assert_eq!(calculate_bill(&input).unwrap_err(), CalcError::WrongUtility);
    }

    #[test]
    fn account_without_approval_is_rejected() {
        let mut input = fixture();
        input.account.industrial_tou_approved = false;
        assert_eq!(
            calculate_bill(&input).unwrap_err(),
            CalcError::ApprovalMissing
        );
    }

    #[test]
    fn approval_without_evidence_reference_is_rejected() {
        let mut input = fixture();
        input.account.approval_reference = None;
        assert_eq!(
            calculate_bill(&input).unwrap_err(),
            CalcError::ApprovalReferenceMissing
        );
    }

    #[test]
    fn legacy_pre_1999_account_below_100_kva_requires_manual_review() {
        let mut input = fixture();
        input.account.supply_capacity_kva = 90.0;
        input.account.legacy_pre_1999_demand_tariff = true;
        assert_eq!(
            calculate_bill(&input).unwrap_err(),
            CalcError::LegacyAccountRequiresManualReview
        );
    }

    #[test]
    fn legacy_pre_1999_account_above_100_kva_still_requires_manual_review() {
        let mut input = fixture();
        input.account.supply_capacity_kva = 150.0;
        input.account.legacy_pre_1999_demand_tariff = true;
        assert_eq!(
            calculate_bill(&input).unwrap_err(),
            CalcError::LegacyAccountRequiresManualReview
        );
    }

    #[test]
    fn summer_rate_schedule_is_separate_from_winter() {
        close(
            energy_rate_cents_per_kwh(Season::Summer, EnergyBand::Peak),
            322.0,
        );
        close(
            energy_rate_cents_per_kwh(Season::Summer, EnergyBand::OffPeak),
            186.35,
        );
    }

    #[test]
    fn winter_rate_schedule_is_separate_from_summer() {
        close(
            energy_rate_cents_per_kwh(Season::Winter, EnergyBand::Peak),
            766.24,
        );
        close(
            energy_rate_cents_per_kwh(Season::Winter, EnergyBand::OffPeak),
            200.44,
        );
    }

    #[test]
    fn weekday_peak_standard_and_offpeak_boundaries_are_correct() {
        assert_eq!(
            classify_interval(DayClass::Weekday, 420, 15, false, None).unwrap(),
            EnergyBand::Peak
        );
        assert_eq!(
            classify_interval(DayClass::Weekday, 360, 15, false, None).unwrap(),
            EnergyBand::Standard
        );
        assert_eq!(
            classify_interval(DayClass::Weekday, 1_320, 15, false, None).unwrap(),
            EnergyBand::OffPeak
        );
    }

    #[test]
    fn weekday_price_boundary_crossing_is_rejected() {
        assert_eq!(
            classify_interval(DayClass::Weekday, 595, 15, false, None).unwrap_err(),
            CalcError::IntervalCrossesPriceBoundary
        );
    }

    #[test]
    fn sunday_standard_and_offpeak_are_correct() {
        assert_eq!(
            classify_interval(DayClass::Sunday, 1_020, 15, false, None).unwrap(),
            EnergyBand::Standard
        );
        assert_eq!(
            classify_interval(DayClass::Sunday, 1_140, 15, false, None).unwrap(),
            EnergyBand::OffPeak
        );
    }

    #[test]
    fn saturday_18_to_20_is_blocked_without_confirmation() {
        assert_eq!(
            classify_interval(DayClass::SaturdayOrPublicHoliday, 1_080, 15, false, None)
                .unwrap_err(),
            CalcError::AmbiguousSaturdayEndpoint
        );
    }

    #[test]
    fn saturday_18_to_20_is_admitted_with_confirmation_reference() {
        assert_eq!(
            classify_interval(
                DayClass::SaturdayOrPublicHoliday,
                1_080,
                15,
                true,
                Some("APPROVED-TARIFF-STATEMENT")
            )
            .unwrap(),
            EnergyBand::Standard
        );
    }

    #[test]
    fn saturday_20_00_is_offpeak() {
        assert_eq!(
            classify_interval(DayClass::SaturdayOrPublicHoliday, 1_200, 15, false, None).unwrap(),
            EnergyBand::OffPeak
        );
    }

    #[test]
    fn saturday_confirmation_without_reference_is_rejected() {
        assert_eq!(
            classify_interval(DayClass::SaturdayOrPublicHoliday, 1_080, 15, true, None)
                .unwrap_err(),
            CalcError::SaturdayConfirmationReferenceMissing
        );
    }

    #[test]
    fn interval_crossing_multiple_bands_is_rejected_even_when_endpoints_match() {
        // 06:55 is standard and 10:04 is also standard, but the interval
        // crosses the 07:00-10:00 weekday peak band in between.
        assert_eq!(
            classify_interval(DayClass::Weekday, 415, 190, false, None).unwrap_err(),
            CalcError::IntervalCrossesPriceBoundary
        );
    }

    #[test]
    fn interval_crossing_midnight_is_rejected() {
        assert_eq!(
            classify_interval(DayClass::Weekday, 1_430, 15, false, None).unwrap_err(),
            CalcError::IntervalCrossesMidnight
        );
    }

    #[test]
    fn lv_rates_are_selected_without_merging_network_components() {
        let rates = rates_for_voltage(VoltageClass::Low);
        close(rates.service_charge_zar_month, 2_444.32);
        close(rates.monthly_network_charge_zar, 2_185.32);
        close(rates.demand_charge_zar_per_kva, 461.28);
    }

    #[test]
    fn mv_rates_are_selected() {
        let rates = rates_for_voltage(VoltageClass::Medium);
        close(rates.service_charge_zar_month, 3_360.96);
        close(rates.monthly_network_charge_zar, 9_347.51);
        close(rates.demand_charge_zar_per_kva, 431.11);
    }

    #[test]
    fn hv_rates_are_selected() {
        let rates = rates_for_voltage(VoltageClass::High);
        close(rates.service_charge_zar_month, 42_915.09);
        close(rates.monthly_network_charge_zar, 43_848.71);
        close(rates.demand_charge_zar_per_kva, 400.93);
    }

    #[test]
    fn minimum_70_kva_demand_floor_binds_when_measured_and_ratchet_are_lower() {
        let mut input = fixture();
        input.account.voltage_class = VoltageClass::Low;
        input.measured_monthly_peak_demand_kva = 55.0;
        input.preceding_12_month_demand_history = history_for("2026-07", 50.0);
        close(
            calculate_bill(&input).unwrap().breakdown.billed_demand_kva,
            70.0,
        );
    }

    #[test]
    fn eighty_percent_top_three_ratchet_binds_after_measured_peak_falls() {
        let mut input = fixture();
        input.measured_monthly_peak_demand_kva = 80.0;
        input.preceding_12_month_demand_history = history_for("2026-07", 125.0);
        close(
            calculate_bill(&input).unwrap().breakdown.billed_demand_kva,
            100.0,
        );
    }

    #[test]
    fn measured_peak_above_ratchet_is_billed() {
        let mut input = fixture();
        input.measured_monthly_peak_demand_kva = 145.0;
        input.preceding_12_month_demand_history = history_for("2026-07", 125.0);
        close(
            calculate_bill(&input).unwrap().breakdown.billed_demand_kva,
            145.0,
        );
    }

    #[test]
    fn missing_prior_demand_month_is_rejected() {
        let mut input = fixture();
        input.preceding_12_month_demand_history.pop();
        assert_eq!(
            calculate_bill(&input).unwrap_err(),
            CalcError::IncompleteDemandHistory
        );
    }

    #[test]
    fn nonconsecutive_demand_history_is_rejected() {
        let mut input = fixture();
        input.preceding_12_month_demand_history[5].local_start_minute = 45;
        assert_eq!(
            calculate_bill(&input).unwrap_err(),
            CalcError::DemandHistoryObservationMismatch
        );
    }

    #[test]
    fn top_three_demands_may_all_come_from_one_day_of_the_rolling_interval_stream() {
        let mut input = fixture();
        let history = &mut input.preceding_12_month_demand_history;
        history[0].measured_demand_kva = 300.0;
        history[1].measured_demand_kva = 270.0;
        history[2].measured_demand_kva = 240.0;
        input.measured_monthly_peak_demand_kva = 100.0;
        close(
            calculate_bill(&input).unwrap().breakdown.billed_demand_kva,
            0.8 * (300.0 + 270.0 + 240.0) / 3.0,
        );
    }

    #[test]
    fn missing_single_half_hour_history_interval_is_rejected() {
        let mut input = fixture();
        input.preceding_12_month_demand_history.remove(100);
        assert_eq!(
            calculate_bill(&input).unwrap_err(),
            CalcError::IncompleteDemandHistory
        );
    }

    #[test]
    fn demand_history_interval_requires_account_specific_30_minute_evidence() {
        let mut input = fixture();
        input.demand_history_interval_minutes = 15;
        assert_eq!(
            calculate_bill(&input).unwrap_err(),
            CalcError::UnsupportedDemandHistoryInterval
        );
    }

    #[test]
    fn missing_demand_history_interval_evidence_is_rejected() {
        let mut input = fixture();
        input.demand_history_interval_evidence_reference.clear();
        assert_eq!(
            calculate_bill(&input).unwrap_err(),
            CalcError::MissingEvidence("demand-history interval/coverage")
        );
    }

    #[test]
    fn low_power_factor_charges_only_reactive_energy_above_thirty_percent() {
        let mut input = fixture();
        input.declared_active_energy_kwh = 100.0;
        input
            .intervals
            .iter_mut()
            .find(|interval| interval.energy_kwh > 0.0)
            .expect("fixture has one positive-energy interval")
            .energy_kwh = 100.0;
        input.power_factor = 0.95;
        input.measured_reactive_energy_kvarh = 40.0;
        close(
            calculate_bill(&input)
                .unwrap()
                .breakdown
                .reactive_energy_charge_zar,
            4.63,
        );
    }

    #[test]
    fn power_factor_at_0_96_has_no_reactive_charge() {
        let mut input = fixture();
        input.power_factor = 0.96;
        input.measured_reactive_energy_kvarh = 100.0;
        close(
            calculate_bill(&input)
                .unwrap()
                .breakdown
                .reactive_energy_charge_zar,
            0.0,
        );
    }

    #[test]
    fn reactive_energy_below_threshold_has_no_charge() {
        let mut input = fixture();
        input.power_factor = 0.95;
        input.measured_reactive_energy_kvarh = 2.0;
        close(
            calculate_bill(&input)
                .unwrap()
                .breakdown
                .reactive_energy_charge_zar,
            0.0,
        );
    }

    #[test]
    fn per_kwh_network_surcharge_is_applied_once() {
        let mut input = fixture();
        input.declared_active_energy_kwh = 100.0;
        input
            .intervals
            .iter_mut()
            .find(|interval| interval.energy_kwh > 0.0)
            .expect("fixture has one positive-energy interval")
            .energy_kwh = 100.0;
        input.network_surcharge_applies = Some(true);
        close(
            calculate_bill(&input)
                .unwrap()
                .breakdown
                .per_kwh_network_surcharge_zar
                .unwrap(),
            6.0,
        );
    }

    #[test]
    fn monthly_network_charge_is_separate_from_per_kwh_surcharge() {
        let receipt = calculate_bill(&fixture()).unwrap();
        close(receipt.breakdown.monthly_network_charge_zar, 2_185.32);
        close(
            receipt.breakdown.per_kwh_network_surcharge_zar.unwrap(),
            0.60,
        );
    }

    #[test]
    fn account_level_surcharge_exemption_is_respected() {
        let mut input = fixture();
        input.network_surcharge_applies = Some(false);
        close(
            calculate_bill(&input)
                .unwrap()
                .breakdown
                .per_kwh_network_surcharge_zar
                .unwrap(),
            0.0,
        );
    }

    #[test]
    fn winter_peak_to_offpeak_shift_reduces_charge_not_energy_or_billed_demand() {
        fn monthly_schedule(shift_lighting_offpeak: bool) -> TariffInput {
            let mut input = fixture();
            input.intervals.clear();
            input.declared_active_energy_kwh = 0.0;
            input.measured_monthly_peak_demand_kva = 95.0;
            input.preceding_12_month_demand_history = history_for("2026-07", 125.0);
            input.power_factor = 0.98;
            input.measured_reactive_energy_kvarh = 0.0;
            input.business_lpu_surcharge = None;
            input.vat = None;

            for day in 1..=31_u8 {
                let local_date = format!("2026-07-{day:02}");
                let weekday = weekday_monday_zero(2026, 7, day);
                let day_class = match weekday {
                    5 => DayClass::SaturdayOrPublicHoliday,
                    6 => DayClass::Sunday,
                    _ => DayClass::Weekday,
                };

                for slot in 0..96_u16 {
                    let local_start_minute = slot * 15;
                    let mut energy_kwh = 1.0;

                    if day_class == DayClass::Weekday {
                        if !shift_lighting_offpeak
                            && (local_start_minute == 420 || local_start_minute == 1_080)
                        {
                            energy_kwh += 5.0;
                        }
                        if shift_lighting_offpeak
                            && (local_start_minute == 120 || local_start_minute == 135)
                        {
                            energy_kwh += 5.0;
                        }
                    }

                    let saturday_evening = day_class == DayClass::SaturdayOrPublicHoliday
                        && (1_080..1_200).contains(&local_start_minute);
                    input.declared_active_energy_kwh += energy_kwh;
                    input.intervals.push(MeteredInterval {
                        local_date: local_date.clone(),
                        day_class,
                        local_start_minute,
                        duration_minutes: 15,
                        energy_kwh,
                        saturday_evening_confirmed: saturday_evening,
                        saturday_confirmation_reference: saturday_evening
                            .then(|| "SYNTHETIC-SATURDAY-BAND-CONFIRMATION".to_owned()),
                        public_holiday_evidence_reference: None,
                    });
                }
            }
            input.interval_series_complete = true;
            input.interval_coverage_evidence_reference =
                "SYNTHETIC-COMPLETE-2026-07-MONTH-INTERVAL-SERIES".to_owned();
            input
        }

        let peak_loaded = calculate_bill(&monthly_schedule(false)).unwrap();
        let offpeak_shifted = calculate_bill(&monthly_schedule(true)).unwrap();

        close(
            peak_loaded.breakdown.active_energy_kwh,
            offpeak_shifted.breakdown.active_energy_kwh,
        );
        close(
            peak_loaded.breakdown.billed_demand_kva,
            offpeak_shifted.breakdown.billed_demand_kva,
        );
        close(peak_loaded.breakdown.billed_demand_kva, 100.0);

        // 23 weekdays × 10 kWh moved from peak to off-peak × R5.658/kWh.
        close(
            peak_loaded.breakdown.energy_charge_zar - offpeak_shifted.breakdown.energy_charge_zar,
            1_301.34,
        );
        close(
            peak_loaded.breakdown.known_component_subtotal_zar
                - offpeak_shifted.breakdown.known_component_subtotal_zar,
            1_301.34,
        );

        // Missing assessed LPU and VAT lines prevent this from becoming
        // a complete invoice or a claim about total customer bill savings.
        assert_eq!(
            peak_loaded.status,
            InvoiceStatus::IncompleteInvoiceReconciliation
        );
        assert_eq!(
            offpeak_shifted.status,
            InvoiceStatus::IncompleteInvoiceReconciliation
        );
        assert_eq!(peak_loaded.breakdown.invoice_total_zar, None);
        assert_eq!(offpeak_shifted.breakdown.invoice_total_zar, None);
    }

    #[test]
    fn wrong_profile_version_is_rejected() {
        let mut input = fixture();
        input.profile_version = "0.2".to_owned();
        assert_eq!(
            calculate_bill(&input).unwrap_err(),
            CalcError::WrongProfileVersion
        );
    }

    #[test]
    fn incomplete_interval_coverage_is_rejected() {
        let mut input = fixture();
        input.interval_series_complete = false;
        assert_eq!(
            calculate_bill(&input).unwrap_err(),
            CalcError::IncompleteIntervalSeries
        );
    }

    #[test]
    fn network_surcharge_decision_requires_account_evidence() {
        let mut input = fixture();
        input.network_surcharge_evidence_reference = None;
        assert_eq!(
            calculate_bill(&input).unwrap_err(),
            CalcError::MissingEvidence("network surcharge account treatment")
        );
    }

    #[test]
    fn unknown_network_surcharge_treatment_keeps_invoice_incomplete() {
        let mut input = fixture();
        input.network_surcharge_applies = None;
        let receipt = calculate_bill(&input).unwrap();
        assert_eq!(
            receipt.status,
            InvoiceStatus::IncompleteInvoiceReconciliation
        );
        assert_eq!(receipt.breakdown.invoice_total_zar, None);
    }

    #[test]
    fn unknown_business_lpu_surcharge_keeps_invoice_incomplete() {
        let mut input = fixture();
        input.business_lpu_surcharge = None;
        let receipt = calculate_bill(&input).unwrap();
        assert_eq!(
            receipt.status,
            InvoiceStatus::IncompleteInvoiceReconciliation
        );
        assert_eq!(receipt.breakdown.invoice_total_zar, None);
    }

    #[test]
    fn unknown_vat_keeps_invoice_incomplete() {
        let mut input = fixture();
        input.vat = None;
        let receipt = calculate_bill(&input).unwrap();
        assert_eq!(
            receipt.status,
            InvoiceStatus::IncompleteInvoiceReconciliation
        );
        assert_eq!(receipt.breakdown.invoice_total_zar, None);
    }

    #[test]
    fn fully_assessed_synthetic_invoice_can_reconcile() {
        let receipt = calculate_bill(&fixture()).unwrap();
        assert_eq!(receipt.status, InvoiceStatus::Complete);
        assert!(receipt.breakdown.invoice_total_zar.is_some());
        assert!(receipt.input_fingerprint_fnv1a64.len() == 16);
    }

    #[test]
    fn negative_or_nonfinite_interval_energy_is_rejected() {
        let mut input = fixture();
        input.intervals[0].energy_kwh = f64::NAN;
        assert!(matches!(
            calculate_bill(&input),
            Err(CalcError::InvalidMeasurement("interval energy"))
        ));
    }

    #[test]
    fn source_profile_mismatch_is_rejected() {
        let mut input = fixture();
        input.source_profile_id = "OLD-TARIFF".to_owned();
        assert_eq!(
            calculate_bill(&input).unwrap_err(),
            CalcError::WrongProfileId
        );
    }

    #[test]
    fn out_of_period_tariff_month_is_rejected() {
        let mut input = fixture();
        input.billing_month = "2027-07".to_owned();
        assert_eq!(
            calculate_bill(&input).unwrap_err(),
            CalcError::OutsideProfilePeriod
        );
    }

    #[test]
    fn interval_date_must_match_billing_month() {
        let mut input = fixture();
        input.intervals[0].local_date = "2026-10-01".to_owned();
        assert_eq!(
            calculate_bill(&input).unwrap_err(),
            CalcError::DateOutsideBillingMonth
        );
    }

    #[test]
    fn date_and_day_class_must_agree() {
        let mut input = fixture();
        // 1 July 2026 is a Wednesday, so it cannot use Saturday/Public Holiday bands.
        for interval in input
            .intervals
            .iter_mut()
            .filter(|i| i.local_date == "2026-07-01")
        {
            interval.day_class = DayClass::SaturdayOrPublicHoliday;
        }
        assert_eq!(
            calculate_bill(&input).unwrap_err(),
            CalcError::PublicHolidayEvidenceMissing
        );

        for interval in input
            .intervals
            .iter_mut()
            .filter(|i| i.local_date == "2026-07-01")
        {
            interval.public_holiday_evidence_reference =
                Some("SYNTHETIC-DATE-SPECIFIC-PUBLIC-HOLIDAY-A".to_owned());
            if (1_080..1_200).contains(&interval.local_start_minute) {
                interval.saturday_evening_confirmed = true;
                interval.saturday_confirmation_reference =
                    Some("SYNTHETIC-SATURDAY-TOU-CONFIRMATION".to_owned());
            }
        }
        let holiday_receipt_a = calculate_bill(&input).unwrap();

        for interval in input
            .intervals
            .iter_mut()
            .filter(|i| i.local_date == "2026-07-01")
        {
            interval.public_holiday_evidence_reference =
                Some("SYNTHETIC-DATE-SPECIFIC-PUBLIC-HOLIDAY-B".to_owned());
        }
        let holiday_receipt_b = calculate_bill(&input).unwrap();
        assert_ne!(
            holiday_receipt_a.input_fingerprint_fnv1a64,
            holiday_receipt_b.input_fingerprint_fnv1a64,
            "the date-specific public-holiday evidence must be bound into the input fingerprint"
        );

        for interval in input
            .intervals
            .iter_mut()
            .filter(|i| i.local_date == "2026-07-01")
        {
            interval.public_holiday_evidence_reference = None;
            interval.day_class = DayClass::Sunday;
        }
        assert_eq!(
            calculate_bill(&input).unwrap_err(),
            CalcError::DayClassDoesNotMatchDate
        );

        for interval in input
            .intervals
            .iter_mut()
            .filter(|i| i.local_date == "2026-07-01")
        {
            interval.day_class = DayClass::Weekday;
            interval.public_holiday_evidence_reference = None;
        }
        for interval in input
            .intervals
            .iter_mut()
            .filter(|i| i.local_date == "2026-07-05")
        {
            interval.day_class = DayClass::Sunday;
        }
        assert!(calculate_bill(&input).is_ok(), "2026-07-05 is a Sunday");
    }

    #[test]
    fn saturday_date_cannot_be_misclassified_as_weekday() {
        let mut input = fixture();
        input.intervals[0].local_date = "2026-07-04".to_owned();
        assert_eq!(
            calculate_bill(&input).unwrap_err(),
            CalcError::DayClassDoesNotMatchDate
        );
    }

    #[test]
    fn impossible_gregorian_date_is_rejected() {
        assert_eq!(parse_date("2026-02-31"), Err(CalcError::InvalidDate));
    }

    #[test]
    fn nonleap_year_february_29_is_rejected() {
        assert_eq!(parse_date("2026-02-29"), Err(CalcError::InvalidDate));
    }

    #[test]
    fn leap_year_february_29_is_valid_date() {
        assert_eq!(parse_date("2028-02-29").unwrap(), (2028, 2, 29));
    }

    #[test]
    fn interval_energy_must_reconcile_to_declared_meter_total() {
        let mut input = fixture();
        input.declared_active_energy_kwh = 99.0;
        assert!(matches!(
            calculate_bill(&input),
            Err(CalcError::ActiveEnergyMismatch { .. })
        ));
    }

    #[test]
    fn overlapping_intervals_are_rejected() {
        let mut input = fixture();
        input.intervals.push(MeteredInterval {
            local_date: "2026-07-01".to_owned(),
            day_class: DayClass::Weekday,
            local_start_minute: 425,
            duration_minutes: 15,
            energy_kwh: 1.0,
            saturday_evening_confirmed: false,
            saturday_confirmation_reference: None,
            public_holiday_evidence_reference: None,
        });
        input.declared_active_energy_kwh += 1.0;
        assert_eq!(
            calculate_bill(&input).unwrap_err(),
            CalcError::IntervalOrderOrOverlap
        );
    }

    #[test]
    fn complete_month_must_start_at_local_midnight() {
        let mut input = fixture();
        input.intervals[0].local_start_minute = 15;
        assert_eq!(
            calculate_bill(&input).unwrap_err(),
            CalcError::IncompleteIntervalCoverage
        );
    }

    #[test]
    fn missing_interval_inside_a_day_is_rejected_even_when_energy_sum_matches() {
        let mut input = fixture();
        input.intervals.remove(20);
        // The manifest and declared kWh are unchanged; the time gap must still fail.
        assert_eq!(
            calculate_bill(&input).unwrap_err(),
            CalcError::IncompleteIntervalCoverage
        );
    }

    #[test]
    fn gap_between_days_is_rejected() {
        let mut input = fixture();
        let next_day_start = 96;
        input.intervals[next_day_start].local_start_minute = 15;
        assert_eq!(
            calculate_bill(&input).unwrap_err(),
            CalcError::IncompleteIntervalCoverage
        );
    }

    #[test]
    fn skipped_local_calendar_day_is_rejected() {
        let mut input = fixture();
        input.intervals.drain(96..192);
        assert_eq!(
            calculate_bill(&input).unwrap_err(),
            CalcError::IncompleteIntervalCoverage
        );
    }

    #[test]
    fn final_interval_must_end_at_local_midnight() {
        let mut input = fixture();
        input.intervals.last_mut().unwrap().duration_minutes = 14;
        assert_eq!(
            calculate_bill(&input).unwrap_err(),
            CalcError::IncompleteIntervalCoverage
        );
    }

    #[test]
    fn final_calendar_day_must_be_present() {
        let mut input = fixture();
        input.intervals.truncate(30 * 96);
        assert_eq!(
            calculate_bill(&input).unwrap_err(),
            CalcError::IncompleteIntervalCoverage
        );
    }

    #[test]
    fn day_class_must_be_constant_across_all_intervals_on_a_date() {
        let mut input = fixture();
        // Both labels are individually valid for a Wednesday only when the
        // Saturday/Public Holiday interval carries its date-specific reference.
        input.intervals[1].day_class = DayClass::SaturdayOrPublicHoliday;
        input.intervals[1].public_holiday_evidence_reference =
            Some("SYNTHETIC-DATE-SPECIFIC-PUBLIC-HOLIDAY".to_owned());
        assert_eq!(
            calculate_bill(&input).unwrap_err(),
            CalcError::InconsistentDayClassification
        );
    }

    #[test]
    fn public_holiday_evidence_must_be_constant_across_all_intervals_on_a_date() {
        let mut input = fixture();
        for interval in input
            .intervals
            .iter_mut()
            .filter(|i| i.local_date == "2026-07-01")
        {
            interval.day_class = DayClass::SaturdayOrPublicHoliday;
            interval.public_holiday_evidence_reference =
                Some("SYNTHETIC-DATE-SPECIFIC-PUBLIC-HOLIDAY-A".to_owned());
        }
        input.intervals[1].public_holiday_evidence_reference =
            Some("SYNTHETIC-DATE-SPECIFIC-PUBLIC-HOLIDAY-B".to_owned());
        assert_eq!(
            calculate_bill(&input).unwrap_err(),
            CalcError::InconsistentDayClassification
        );
    }

    #[test]
    fn zero_kwh_intervals_are_valid_and_complete_coverage_is_still_required() {
        let input = fixture();
        assert_eq!(input.intervals.len(), 31 * 96);
        assert!(input.intervals.iter().any(|i| i.energy_kwh == 0.0));
        assert!(calculate_bill(&input).is_ok());
    }

    #[test]
    fn nonuniform_but_contiguous_intervals_are_valid() {
        let mut input = fixture();
        // Merge 00:00-00:15 and 00:15-00:30 into one 30-minute off-peak interval.
        input.intervals[0].duration_minutes = 30;
        input.intervals.remove(1);
        assert!(calculate_bill(&input).is_ok());
    }

    #[test]
    fn missing_meter_calibration_evidence_is_rejected() {
        let mut input = fixture();
        input.meter_calibration_reference.clear();
        assert_eq!(
            calculate_bill(&input).unwrap_err(),
            CalcError::MissingEvidence("meter calibration")
        );
    }

    #[test]
    fn unsupported_timezone_is_rejected() {
        let mut input = fixture();
        input.time_zone = "UTC".to_owned();
        assert_eq!(
            calculate_bill(&input).unwrap_err(),
            CalcError::MissingEvidence("Africa/Johannesburg timebase")
        );
    }
}
