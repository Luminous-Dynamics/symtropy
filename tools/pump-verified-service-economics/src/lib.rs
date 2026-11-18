//! Cost and energy per independently qualified delivered pump service.
//!
//! This is a narrow composition evaluator. It consumes synthetic, normalized
//! owner receipts from manufacturing economics, pump transferability, whole-system
//! energy and per-area service-floor owners. It does not create those owner records,
//! compute hydraulics, authorize pumping, or make water-quality claims.

use std::collections::{BTreeMap, BTreeSet};

pub const CASES: &str = include_str!("../fixtures/cases-v0.1.tsv");
pub const AREAS: &str = include_str!("../fixtures/service-areas-v0.1.tsv");
pub const EXPECTED: &str = include_str!("../fixtures/expected-v0.1.tsv");

const CASE_HEADER: &str = "case_id\tasset_ref\tserial_ref\tconfig_ref\ttransferability_ref\ttransferability_disposition\ttransfer_serial_ref\ttransfer_config_ref\tcost_bundle_ref\tcost_allocation_ref\tcost_evidence_ref\tcurrency\tamount_unit\tcost_low_minor\tcost_high_minor\tcost_period_start\tcost_period_end\tenergy_evidence_ref\tenergy_meter_ref\tenergy_calibration_ref\tenergy_boundary_ref\tenergy_unit\tenergy_low_wh\tenergy_high_wh\tenergy_window_start\tenergy_window_end\tservice_profile_ref\tservice_boundary_ref\tstudy_start\tstudy_end\trequired_area_count\tpotable_claim\tevidence_state";
const AREA_HEADER: &str = "case_id\tarea_id\tfloor_profile_ref\tfloor_receipt_ref\tfloor_disposition\tobservation_ref\tvolume_calibration_ref\tservice_boundary_ref\twindow_start\twindow_end\tvolume_unit\tvolume_low_ml\tvolume_high_ml\tevidence_state";
const EXPECTED_HEADER: &str = "case_id\texpected_disposition";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Case {
    pub case_id: String,
    pub asset_ref: String,
    pub serial_ref: String,
    pub config_ref: String,
    pub transferability_ref: String,
    pub transferability_disposition: String,
    pub transfer_serial_ref: String,
    pub transfer_config_ref: String,
    pub cost_bundle_ref: String,
    pub cost_allocation_ref: String,
    pub cost_evidence_ref: String,
    pub currency: String,
    pub amount_unit: String,
    pub cost_low_minor: u64,
    pub cost_high_minor: u64,
    pub cost_period_start: String,
    pub cost_period_end: String,
    pub energy_evidence_ref: String,
    pub energy_meter_ref: String,
    pub energy_calibration_ref: String,
    pub energy_boundary_ref: String,
    pub energy_unit: String,
    pub energy_low_wh: u64,
    pub energy_high_wh: u64,
    pub energy_window_start: String,
    pub energy_window_end: String,
    pub service_profile_ref: String,
    pub service_boundary_ref: String,
    pub study_start: String,
    pub study_end: String,
    pub required_area_count: usize,
    pub potable_claim: bool,
    pub evidence_state: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServiceArea {
    pub case_id: String,
    pub area_id: String,
    pub floor_profile_ref: String,
    pub floor_receipt_ref: String,
    pub floor_disposition: String,
    pub observation_ref: String,
    pub volume_calibration_ref: String,
    pub service_boundary_ref: String,
    pub window_start: String,
    pub window_end: String,
    pub volume_unit: String,
    pub volume_low_ml: u64,
    pub volume_high_ml: u64,
    pub evidence_state: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Disposition {
    QualifiedCostPerService,
    ServiceFloorBreached,
    ServiceFloorBreachedWithUnknown,
    Indeterminate,
    EvidenceIncomplete,
    NotTransferable,
    ProfileIncomplete,
    ZeroQualifiedService,
    UnsupportedClaim,
    IdentityMismatch,
    Invalid,
}

impl Disposition {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::QualifiedCostPerService => "QUALIFIED_COST_PER_SERVICE",
            Self::ServiceFloorBreached => "SERVICE_FLOOR_BREACHED",
            Self::ServiceFloorBreachedWithUnknown => "SERVICE_FLOOR_BREACHED_WITH_UNKNOWN",
            Self::Indeterminate => "INDETERMINATE",
            Self::EvidenceIncomplete => "EVIDENCE_INCOMPLETE",
            Self::NotTransferable => "NOT_TRANSFERABLE",
            Self::ProfileIncomplete => "PROFILE_INCOMPLETE",
            Self::ZeroQualifiedService => "ZERO_QUALIFIED_SERVICE",
            Self::UnsupportedClaim => "UNSUPPORTED_CLAIM",
            Self::IdentityMismatch => "IDENTITY_MISMATCH",
            Self::Invalid => "INVALID",
        }
    }

    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "QUALIFIED_COST_PER_SERVICE" => Ok(Self::QualifiedCostPerService),
            "SERVICE_FLOOR_BREACHED" => Ok(Self::ServiceFloorBreached),
            "SERVICE_FLOOR_BREACHED_WITH_UNKNOWN" => Ok(Self::ServiceFloorBreachedWithUnknown),
            "INDETERMINATE" => Ok(Self::Indeterminate),
            "EVIDENCE_INCOMPLETE" => Ok(Self::EvidenceIncomplete),
            "NOT_TRANSFERABLE" => Ok(Self::NotTransferable),
            "PROFILE_INCOMPLETE" => Ok(Self::ProfileIncomplete),
            "ZERO_QUALIFIED_SERVICE" => Ok(Self::ZeroQualifiedService),
            "UNSUPPORTED_CLAIM" => Ok(Self::UnsupportedClaim),
            "IDENTITY_MISMATCH" => Ok(Self::IdentityMismatch),
            "INVALID" => Ok(Self::Invalid),
            _ => Err(format!("unknown expected disposition: {value}")),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExpectedRow {
    pub case_id: String,
    pub disposition: Disposition,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Evaluation {
    pub case_id: String,
    pub disposition: Disposition,
    pub reason: String,
    pub asset_ref: String,
    pub serial_ref: String,
    pub transferability_ref: String,
    pub cost_bundle_ref: String,
    pub cost_allocation_ref: String,
    pub energy_evidence_ref: String,
    pub service_profile_ref: String,
    pub currency: String,
    pub amount_unit: String,
    pub qualified_area_count: usize,
    pub breached_area_count: usize,
    pub unknown_area_count: usize,
    pub qualified_volume_low_ml: Option<u64>,
    pub qualified_volume_high_ml: Option<u64>,
    pub cost_per_m3_milli_minor_low: Option<u128>,
    pub cost_per_m3_milli_minor_high: Option<u128>,
    pub kwh_per_m3_milli_low: Option<u128>,
    pub kwh_per_m3_milli_high: Option<u128>,
}

fn valid_ref(value: &str) -> bool {
    !value.is_empty()
        && value != "-"
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || b":/_-.@".contains(&byte)
        })
}

fn valid_optional_ref(value: &str) -> bool {
    value == "-" || valid_ref(value)
}

fn valid_currency(value: &str) -> bool {
    value.len() == 3 && value.bytes().all(|byte| byte.is_ascii_uppercase())
}

fn valid_money_unit(currency: &str, unit: &str) -> bool {
    unit.len() > 4
        && unit.starts_with(currency)
        && unit.as_bytes().get(3) == Some(&b'_')
        && unit[4..]
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
}

fn valid_date(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 10
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || !bytes.iter().enumerate().all(|(i, b)| i == 4 || i == 7 || b.is_ascii_digit())
    {
        return false;
    }
    let Ok(year) = value[0..4].parse::<u32>() else { return false };
    let Ok(month) = value[5..7].parse::<u32>() else { return false };
    let Ok(day) = value[8..10].parse::<u32>() else { return false };
    if year == 0 || !(1..=12).contains(&month) { return false; }
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return false,
    };
    (1..=days).contains(&day)
}

fn split_row<'a>(line: &'a str, expected: usize, source: &str, line_no: usize) -> Result<Vec<&'a str>, String> {
    let fields: Vec<&str> = line.split('\t').collect();
    if fields.len() != expected {
        return Err(format!("{source}:{line_no}: expected {expected} fields, found {}", fields.len()));
    }
    if fields.iter().any(|f| f.is_empty() || f.trim() != *f) {
        return Err(format!("{source}:{line_no}: empty or padded fields are forbidden"));
    }
    Ok(fields)
}

fn parse_u64(value: &str, source: &str, line_no: usize, field: &str) -> Result<u64, String> {
    value.parse::<u64>().map_err(|_| format!("{source}:{line_no}: invalid {field}: {value}"))
}

fn parse_usize(value: &str, source: &str, line_no: usize, field: &str) -> Result<usize, String> {
    value.parse::<usize>().map_err(|_| format!("{source}:{line_no}: invalid {field}: {value}"))
}

pub fn parse_cases(input: &str) -> Result<Vec<Case>, String> {
    let mut lines = input.lines();
    match lines.next() {
        Some(h) if h == CASE_HEADER => {}
        Some(_) => return Err("cases: header mismatch".into()),
        None => return Err("cases: empty input".into()),
    }
    let mut cases = Vec::new();
    let mut seen = BTreeSet::new();
    for (i, line) in lines.enumerate() {
        if line.is_empty() { continue; }
        let r = split_row(line, 33, "cases", i + 2)?;
        for (field, value) in [
            ("case_id", r[0]), ("asset_ref", r[1]), ("serial_ref", r[2]), ("config_ref", r[3]),
            ("currency", r[11]), ("amount_unit", r[12]), ("cost_period_start", r[15]),
            ("cost_period_end", r[16]), ("energy_unit", r[21]), ("energy_window_start", r[24]),
            ("energy_window_end", r[25]), ("service_profile_ref", r[26]),
            ("service_boundary_ref", r[27]), ("study_start", r[28]), ("study_end", r[29]),
            ("evidence_state", r[32]),
        ] {
            if !valid_ref(value) {
                return Err(format!("cases:{}: invalid {field}", i + 2));
            }
        }
        for (field, value) in [
            ("transferability_ref", r[4]), ("transfer_serial_ref", r[6]), ("transfer_config_ref", r[7]),
            ("cost_bundle_ref", r[8]), ("cost_allocation_ref", r[9]), ("cost_evidence_ref", r[10]),
            ("energy_evidence_ref", r[17]), ("energy_meter_ref", r[18]),
            ("energy_calibration_ref", r[19]), ("energy_boundary_ref", r[20]),
        ] {
            if !valid_optional_ref(value) {
                return Err(format!("cases:{}: invalid {field}", i + 2));
            }
        }
        if !valid_currency(r[11]) {
            return Err(format!("cases:{}: malformed currency code", i + 2));
        }
        if !matches!(
            r[5],
            "TRANSFERABLE_WITHIN_DECLARED_ENVELOPE" | "NOT_TRANSFERABLE"
                | "INDETERMINATE" | "PROFILE_INCOMPLETE" | "EVIDENCE_INCOMPLETE"
        ) {
            return Err(format!("cases:{}: unsupported transferability disposition", i + 2));
        }
        if !valid_date(r[15]) || !valid_date(r[16]) || !valid_date(r[24]) || !valid_date(r[25])
            || !valid_date(r[28]) || !valid_date(r[29])
        {
            return Err(format!("cases:{}: invalid study/cost/energy dates", i + 2));
        }
        if r[31] != "YES" && r[31] != "NO" {
            return Err(format!("cases:{}: potable_claim must be YES or NO", i + 2));
        }
        if !seen.insert(r[0].to_owned()) {
            return Err(format!("cases:{}: duplicate case_id {}", i + 2, r[0]));
        }
        let case = Case {
            case_id: r[0].into(), asset_ref: r[1].into(), serial_ref: r[2].into(), config_ref: r[3].into(),
            transferability_ref: r[4].into(), transferability_disposition: r[5].into(),
            transfer_serial_ref: r[6].into(), transfer_config_ref: r[7].into(),
            cost_bundle_ref: r[8].into(), cost_allocation_ref: r[9].into(), cost_evidence_ref: r[10].into(),
            currency: r[11].into(), amount_unit: r[12].into(),
            cost_low_minor: parse_u64(r[13], "cases", i + 2, "cost_low_minor")?,
            cost_high_minor: parse_u64(r[14], "cases", i + 2, "cost_high_minor")?,
            cost_period_start: r[15].into(), cost_period_end: r[16].into(),
            energy_evidence_ref: r[17].into(), energy_meter_ref: r[18].into(),
            energy_calibration_ref: r[19].into(), energy_boundary_ref: r[20].into(), energy_unit: r[21].into(),
            energy_low_wh: parse_u64(r[22], "cases", i + 2, "energy_low_wh")?,
            energy_high_wh: parse_u64(r[23], "cases", i + 2, "energy_high_wh")?,
            energy_window_start: r[24].into(), energy_window_end: r[25].into(),
            service_profile_ref: r[26].into(), service_boundary_ref: r[27].into(),
            study_start: r[28].into(), study_end: r[29].into(),
            required_area_count: parse_usize(r[30], "cases", i + 2, "required_area_count")?,
            potable_claim: r[31] == "YES", evidence_state: r[32].into(),
        };
        validate_case(&case)?;
        cases.push(case);
    }
    if cases.is_empty() { return Err("cases: no case rows".into()); }
    Ok(cases)
}

fn validate_case(c: &Case) -> Result<(), String> {
    for (field, value) in [
        ("case_id", c.case_id.as_str()), ("asset_ref", c.asset_ref.as_str()),
        ("serial_ref", c.serial_ref.as_str()), ("config_ref", c.config_ref.as_str()),
        ("service_profile_ref", c.service_profile_ref.as_str()), ("service_boundary_ref", c.service_boundary_ref.as_str()),
    ] {
        if !valid_ref(value) { return Err(format!("case {}: invalid direct {field}", c.case_id)); }
    }
    if !valid_date(&c.study_start) || !valid_date(&c.study_end) || !valid_date(&c.cost_period_start)
        || !valid_date(&c.cost_period_end) || !valid_date(&c.energy_window_start) || !valid_date(&c.energy_window_end)
    {
        return Err(format!("case {}: invalid direct date", c.case_id));
    }
    if !valid_currency(&c.currency) || !valid_ref(&c.amount_unit) {
        return Err(format!("case {}: malformed direct currency/amount unit", c.case_id));
    }
    if c.required_area_count == 0 {
        return Err(format!("case {}: required_area_count must be positive", c.case_id));
    }
    Ok(())
}

pub fn parse_service_areas(input: &str) -> Result<Vec<ServiceArea>, String> {
    let mut lines = input.lines();
    match lines.next() {
        Some(h) if h == AREA_HEADER => {}
        Some(_) => return Err("service areas: header mismatch".into()),
        None => return Err("service areas: empty input".into()),
    }
    let mut areas = Vec::new();
    for (i, line) in lines.enumerate() {
        if line.is_empty() { continue; }
        let r = split_row(line, 14, "service areas", i + 2)?;
        for (field, value) in [
            ("case_id", r[0]), ("area_id", r[1]), ("floor_profile_ref", r[2]),
            ("volume_unit", r[10]), ("evidence_state", r[13]),
        ] {
            if !valid_ref(value) { return Err(format!("service areas:{}: invalid {field}", i + 2)); }
        }
        for (field, value) in [
            ("floor_receipt_ref", r[3]), ("observation_ref", r[5]), ("volume_calibration_ref", r[6]),
            ("service_boundary_ref", r[7]),
        ] {
            if !valid_optional_ref(value) { return Err(format!("service areas:{}: invalid {field}", i + 2)); }
        }
        if !valid_date(r[8]) || !valid_date(r[9]) {
            return Err(format!("service areas:{}: invalid service window", i + 2));
        }
        if !matches!(r[4], "MET" | "BREACHED" | "UNKNOWN") {
            return Err(format!("service areas:{}: unsupported floor disposition", i + 2));
        }
        areas.push(ServiceArea {
            case_id: r[0].into(), area_id: r[1].into(), floor_profile_ref: r[2].into(),
            floor_receipt_ref: r[3].into(), floor_disposition: r[4].into(),
            observation_ref: r[5].into(), volume_calibration_ref: r[6].into(),
            service_boundary_ref: r[7].into(), window_start: r[8].into(), window_end: r[9].into(),
            volume_unit: r[10].into(),
            volume_low_ml: parse_u64(r[11], "service areas", i + 2, "volume_low_ml")?,
            volume_high_ml: parse_u64(r[12], "service areas", i + 2, "volume_high_ml")?,
            evidence_state: r[13].into(),
        });
    }
    if areas.is_empty() { return Err("service areas: no rows".into()); }
    Ok(areas)
}

pub fn parse_expected(input: &str) -> Result<Vec<ExpectedRow>, String> {
    let mut lines = input.lines();
    match lines.next() {
        Some(h) if h == EXPECTED_HEADER => {}
        Some(_) => return Err("expected: header mismatch".into()),
        None => return Err("expected: empty input".into()),
    }
    let mut rows = Vec::new();
    let mut seen = BTreeSet::new();
    for (i, line) in lines.enumerate() {
        if line.is_empty() { continue; }
        let r = split_row(line, 2, "expected", i + 2)?;
        if !valid_ref(r[0]) || !seen.insert(r[0].to_owned()) {
            return Err(format!("expected:{}: invalid or duplicate case_id", i + 2));
        }
        rows.push(ExpectedRow { case_id: r[0].into(), disposition: Disposition::parse(r[1])? });
    }
    Ok(rows)
}

fn floor_ratio_floor(numerator: u64, scale: u64, denominator: u64) -> Result<u128, Disposition> {
    if denominator == 0 { return Err(Disposition::ZeroQualifiedService); }
    let product = (numerator as u128)
        .checked_mul(scale as u128)
        .ok_or(Disposition::Invalid)?;
    Ok(product / denominator as u128)
}

fn floor_ratio_ceil(numerator: u64, scale: u64, denominator: u64) -> Result<u128, Disposition> {
    if denominator == 0 { return Err(Disposition::ZeroQualifiedService); }
    let product = (numerator as u128)
        .checked_mul(scale as u128)
        .ok_or(Disposition::Invalid)?;
    let d = denominator as u128;
    Ok(product.checked_add(d - 1).ok_or(Disposition::Invalid)? / d)
}

fn eval_one(case: &Case, areas: &[ServiceArea]) -> Evaluation {
    let mut out = Evaluation {
        case_id: case.case_id.clone(),
        disposition: Disposition::EvidenceIncomplete,
        reason: "service economics evidence is incomplete".into(),
        asset_ref: case.asset_ref.clone(), serial_ref: case.serial_ref.clone(),
        transferability_ref: case.transferability_ref.clone(), cost_bundle_ref: case.cost_bundle_ref.clone(),
        cost_allocation_ref: case.cost_allocation_ref.clone(), energy_evidence_ref: case.energy_evidence_ref.clone(),
        service_profile_ref: case.service_profile_ref.clone(), currency: case.currency.clone(), amount_unit: case.amount_unit.clone(),
        qualified_area_count: 0, breached_area_count: 0, unknown_area_count: 0,
        qualified_volume_low_ml: None, qualified_volume_high_ml: None,
        cost_per_m3_milli_minor_low: None, cost_per_m3_milli_minor_high: None,
        kwh_per_m3_milli_low: None, kwh_per_m3_milli_high: None,
    };
    macro_rules! finish { ($d:expr,$r:expr) => {{ out.disposition=$d;out.reason=$r.into();return out; }}; }

    if case.potable_claim {
        finish!(Disposition::UnsupportedClaim, "hydraulic service evidence does not establish potable-water quality");
    }
    if !valid_ref(&case.transferability_ref) {
        finish!(Disposition::EvidenceIncomplete, "pump transferability receipt reference is missing");
    }
    if case.transferability_disposition == "NOT_TRANSFERABLE" {
        finish!(Disposition::NotTransferable, "exact pump acceptance-to-service owner rejected this asset/duty");
    }
    if case.transfer_serial_ref != case.serial_ref || case.transfer_config_ref != case.config_ref {
        finish!(Disposition::IdentityMismatch, "transferability receipt does not bind the exact asset serial/configuration");
    }
    if case.transferability_disposition != "TRANSFERABLE_WITHIN_DECLARED_ENVELOPE" {
        finish!(Disposition::Indeterminate, "pump acceptance-to-service transferability is not positively established");
    }
    if case.evidence_state != "COMPLETE" {
        finish!(Disposition::EvidenceIncomplete, "case-level evidence is missing or stale");
    }
    if case.study_start != case.cost_period_start || case.study_end != case.cost_period_end {
        finish!(Disposition::EvidenceIncomplete, "cost allocation period does not match the service study horizon");
    }
    if case.study_start != case.energy_window_start || case.study_end != case.energy_window_end {
        finish!(Disposition::EvidenceIncomplete, "energy window does not match the service study horizon");
    }
    if case.cost_low_minor > case.cost_high_minor || case.energy_low_wh > case.energy_high_wh {
        finish!(Disposition::Invalid, "cost or energy interval is reversed");
    }
    if case.currency.len() != 3 || !valid_money_unit(&case.currency, &case.amount_unit) {
        finish!(Disposition::ProfileIncomplete, "cost currency and amount unit do not match");
    }
    for r in [&case.cost_bundle_ref, &case.cost_allocation_ref, &case.cost_evidence_ref] {
        if !valid_ref(r) { finish!(Disposition::EvidenceIncomplete, "cost bundle/allocation/evidence reference is missing"); }
    }
    for r in [&case.energy_evidence_ref, &case.energy_meter_ref, &case.energy_calibration_ref, &case.energy_boundary_ref] {
        if !valid_ref(r) { finish!(Disposition::EvidenceIncomplete, "energy evidence, meter, calibration or boundary reference is missing"); }
    }
    if case.energy_unit != "WH" {
        finish!(Disposition::ProfileIncomplete, "energy input must use the declared Wh unit profile");
    }
    if case.energy_low_wh > case.energy_high_wh {
        finish!(Disposition::Invalid, "energy interval is reversed");
    }
    if case.required_area_count == 0 || !valid_ref(&case.service_profile_ref) || !valid_ref(&case.service_boundary_ref) {
        finish!(Disposition::ProfileIncomplete, "service profile/boundary/required-area contract is incomplete");
    }
    if areas.len() != case.required_area_count {
        finish!(Disposition::EvidenceIncomplete, "observed service-area count does not equal the required-area population");
    }

    let mut area_ids = BTreeSet::new();
    let mut volume_low = 0_u64;
    let mut volume_high = 0_u64;
    let mut has_breach = false;
    let mut has_unknown = false;
    for area in areas {
        if !area_ids.insert(area.area_id.as_str()) {
            finish!(Disposition::Invalid, "duplicate service-area identity");
        }
        if !valid_ref(&area.area_id) || !valid_ref(&area.floor_profile_ref) {
            finish!(Disposition::Invalid, "invalid area/profile identity");
        }
        if area.floor_profile_ref != case.service_profile_ref {
            finish!(Disposition::ProfileIncomplete, "service-area receipt uses a different floor-profile revision");
        }
        if area.service_boundary_ref != case.service_boundary_ref {
            finish!(Disposition::ProfileIncomplete, "service-area measurement uses a different delivery boundary");
        }
        if area.window_start != case.study_start || area.window_end != case.study_end {
            finish!(Disposition::EvidenceIncomplete, "service-area window does not match the study horizon");
        }
        if area.volume_unit != "ML" {
            finish!(Disposition::ProfileIncomplete, "service volume must use the declared ML unit profile");
        }
        if area.volume_low_ml > area.volume_high_ml {
            finish!(Disposition::Invalid, "service volume interval is reversed");
        }
        if area.evidence_state != "COMPLETE" {
            finish!(Disposition::EvidenceIncomplete, "service-area observation is missing or stale");
        }
        for r in [&area.floor_receipt_ref, &area.observation_ref, &area.volume_calibration_ref] {
            if !valid_ref(r) {
                finish!(Disposition::EvidenceIncomplete, "service floor, observation or volume-calibration reference is missing");
            }
        }
        match area.floor_disposition.as_str() {
            "MET" => {
                out.qualified_area_count += 1;
                volume_low = match volume_low.checked_add(area.volume_low_ml) {
                    Some(v) => v, None => finish!(Disposition::Invalid, "qualified volume sum overflow"),
                };
                volume_high = match volume_high.checked_add(area.volume_high_ml) {
                    Some(v) => v, None => finish!(Disposition::Invalid, "qualified volume sum overflow"),
                };
            }
            "BREACHED" => { has_breach = true; out.breached_area_count += 1; }
            "UNKNOWN" => { has_unknown = true; out.unknown_area_count += 1; }
            _ => finish!(Disposition::Invalid, "unknown floor disposition"),
        }
    }
    if has_breach && has_unknown {
        finish!(Disposition::ServiceFloorBreachedWithUnknown, "at least one required area is breached and another is unknown");
    }
    if has_breach {
        finish!(Disposition::ServiceFloorBreached, "at least one required service area failed its non-compensatory floor");
    }
    if has_unknown {
        finish!(Disposition::Indeterminate, "one or more required service areas are not independently established as meeting all floors");
    }
    if out.qualified_area_count != case.required_area_count {
        finish!(Disposition::EvidenceIncomplete, "not every required area has a verified MET disposition");
    }
    if volume_high == 0 {
        finish!(Disposition::ZeroQualifiedService, "no positive qualified service volume was delivered in the study horizon");
    }
    if volume_low == 0 {
        finish!(Disposition::Indeterminate, "qualified service volume lower bound is zero; finite conservative upper cost/energy ratio cannot be established");
    }

    out.qualified_volume_low_ml = Some(volume_low);
    out.qualified_volume_high_ml = Some(volume_high);
    out.cost_per_m3_milli_minor_low = match floor_ratio_floor(case.cost_low_minor, 1_000_000_000, volume_high) {
        Ok(v) => Some(v), Err(d) => finish!(d, "cost-per-service lower interval arithmetic failed"),
    };
    out.cost_per_m3_milli_minor_high = match floor_ratio_ceil(case.cost_high_minor, 1_000_000_000, volume_low) {
        Ok(v) => Some(v), Err(d) => finish!(d, "cost-per-service upper interval arithmetic failed"),
    };
    out.kwh_per_m3_milli_low = match floor_ratio_floor(case.energy_low_wh, 1_000_000, volume_high) {
        Ok(v) => Some(v), Err(d) => finish!(d, "energy-per-service lower interval arithmetic failed"),
    };
    out.kwh_per_m3_milli_high = match floor_ratio_ceil(case.energy_high_wh, 1_000_000, volume_low) {
        Ok(v) => Some(v), Err(d) => finish!(d, "energy-per-service upper interval arithmetic failed"),
    };
    finish!(
        Disposition::QualifiedCostPerService,
        "all transferability, cost, energy, per-area service-floor, boundary and interval requirements are complete"
    );
}

pub fn evaluate(cases: &[Case], areas: &[ServiceArea]) -> Result<Vec<Evaluation>, String> {
    if cases.is_empty() { return Err("evaluator input contains no cases".into()); }
    let mut cases_by_id = BTreeMap::new();
    for case in cases {
        validate_case(case)?;
        if cases_by_id.insert(case.case_id.as_str(), case).is_some() {
            return Err(format!("duplicate case_id: {}", case.case_id));
        }
    }
    let mut area_by_case: BTreeMap<&str, Vec<&ServiceArea>> = BTreeMap::new();
    for area in areas {
        if !cases_by_id.contains_key(area.case_id.as_str()) {
            return Err(format!("orphan service-area row for case {}", area.case_id));
        }
        area_by_case.entry(area.case_id.as_str()).or_default().push(area);
    }
    let mut results=Vec::with_capacity(cases.len());
    for case in cases {
        let selected: Vec<ServiceArea> = area_by_case.get(case.case_id.as_str()).into_iter()
            .flat_map(|rows| rows.iter().map(|row| (**row).clone())).collect();
        results.push(eval_one(case,&selected));
    }
    Ok(results)
}

pub fn verify_expected(actual: &[Evaluation], expected: &[ExpectedRow]) -> Result<(), String> {
    if actual.len()!=expected.len() { return Err(format!("expected {} rows, got {}",expected.len(),actual.len())); }
    let map: BTreeMap<&str,&Evaluation>=actual.iter().map(|r|(r.case_id.as_str(),r)).collect();
    for row in expected {
        let got=map.get(row.case_id.as_str()).ok_or_else(||format!("missing case {}",row.case_id))?;
        if got.disposition!=row.disposition {
            return Err(format!("{}: expected {}, got {} ({})",row.case_id,row.disposition.as_str(),got.disposition.as_str(),got.reason));
        }
    }
    Ok(())
}

pub fn evaluate_corpus(cases_tsv:&str,areas_tsv:&str,expected_tsv:&str)->Result<Vec<Evaluation>,String>{
    let cases=parse_cases(cases_tsv)?;
    let areas=parse_service_areas(areas_tsv)?;
    let expected=parse_expected(expected_tsv)?;
    let actual=evaluate(&cases,&areas)?;
    verify_expected(&actual,&expected)?;
    Ok(actual)
}

fn opt_u128(value:Option<u128>)->String{
    value.map_or_else(||"-".to_owned(),|n|n.to_string())
}
fn opt_u64(value:Option<u64>)->String{
    value.map_or_else(||"-".to_owned(),|n|n.to_string())
}

pub fn render_report(rows:&[Evaluation])->String{
    let mut out=String::from("case_id\tdisposition\treason\tasset_ref\tserial_ref\ttransferability_ref\tcost_bundle_ref\tcost_allocation_ref\tenergy_evidence_ref\tservice_profile_ref\tcurrency\tamount_unit\tqualified_area_count\tbreached_area_count\tunknown_area_count\tqualified_volume_low_ml\tqualified_volume_high_ml\tcost_per_m3_milli_minor_low\tcost_per_m3_milli_minor_high\tkwh_per_m3_milli_low\tkwh_per_m3_milli_high\n");
    for r in rows{
        out.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            r.case_id,r.disposition.as_str(),r.reason,r.asset_ref,r.serial_ref,r.transferability_ref,
            r.cost_bundle_ref,r.cost_allocation_ref,r.energy_evidence_ref,r.service_profile_ref,
            r.currency,r.amount_unit,r.qualified_area_count,r.breached_area_count,r.unknown_area_count,
            opt_u64(r.qualified_volume_low_ml),opt_u64(r.qualified_volume_high_ml),
            opt_u128(r.cost_per_m3_milli_minor_low),opt_u128(r.cost_per_m3_milli_minor_high),
            opt_u128(r.kwh_per_m3_milli_low),opt_u128(r.kwh_per_m3_milli_high)
        ));
    }
    out
}

#[cfg(test)]
mod tests{
    use super::*;
    fn run()->Vec<Evaluation>{evaluate_corpus(CASES,AREAS,EXPECTED).unwrap()}
    fn one(id:&str)->Evaluation{run().into_iter().find(|r|r.case_id==id).unwrap()}

    #[test] fn frozen_thirty_two_case_oracle_matches(){assert_eq!(run().len(),32);}
    #[test] fn positive_case_emits_cost_and_energy_intervals(){
        let r=one("positive_verified_service");
        assert_eq!(r.disposition,Disposition::QualifiedCostPerService);
        assert!(r.cost_per_m3_milli_minor_low.is_some());
        assert!(r.kwh_per_m3_milli_low.is_some());
    }
    #[test] fn nontransferable_pump_cannot_receive_service_cost_score(){
        assert_eq!(one("pump_not_transferable").disposition,Disposition::NotTransferable);
    }
    #[test] fn indeterminate_transferability_does_not_pass(){
        assert_eq!(one("transferability_indeterminate").disposition,Disposition::Indeterminate);
    }
    #[test] fn one_breached_area_blocks_score_despite_other_area_volume(){
        assert_eq!(one("one_area_pressure_breach").disposition,Disposition::ServiceFloorBreached);
        assert_eq!(one("one_area_flow_breach").disposition,Disposition::ServiceFloorBreached);
    }
    #[test] fn unknown_area_does_not_become_met(){
        assert_eq!(one("one_area_unknown").disposition,Disposition::Indeterminate);
    }
    #[test] fn breach_and_unknown_are_preserved_separately(){
        assert_eq!(one("breach_and_unknown").disposition,Disposition::ServiceFloorBreachedWithUnknown);
    }
    #[test] fn missing_required_area_is_incomplete(){
        assert_eq!(one("missing_service_area").disposition,Disposition::EvidenceIncomplete);
    }
    #[test] fn service_profile_and_boundary_must_match(){
        assert_eq!(one("service_profile_mismatch").disposition,Disposition::ProfileIncomplete);
        assert_eq!(one("service_boundary_mismatch").disposition,Disposition::ProfileIncomplete);
    }
    #[test] fn floor_and_observation_evidence_are_mandatory(){
        assert_eq!(one("floor_receipt_missing").disposition,Disposition::EvidenceIncomplete);
        assert_eq!(one("stale_service_observation").disposition,Disposition::EvidenceIncomplete);
        assert_eq!(one("service_observation_reference_missing").disposition,Disposition::EvidenceIncomplete);
    }
    #[test] fn volume_calibration_is_required(){
        assert_eq!(one("volume_calibration_missing").disposition,Disposition::EvidenceIncomplete);
    }
    #[test] fn volume_unit_must_be_declared(){
        assert_eq!(one("volume_unit_mismatch").disposition,Disposition::ProfileIncomplete);
    }
    #[test] fn energy_window_boundary_and_calibration_are_required(){
        assert_eq!(one("energy_window_mismatch").disposition,Disposition::EvidenceIncomplete);
        assert_eq!(one("energy_boundary_missing").disposition,Disposition::EvidenceIncomplete);
        assert_eq!(one("energy_calibration_missing").disposition,Disposition::EvidenceIncomplete);
        assert_eq!(one("energy_evidence_missing").disposition,Disposition::EvidenceIncomplete);
    }
    #[test] fn energy_units_are_not_guessed(){
        assert_eq!(one("energy_unit_unsupported").disposition,Disposition::ProfileIncomplete);
    }
    #[test] fn cost_bundle_and_allocation_evidence_are_required(){
        assert_eq!(one("cost_source_missing").disposition,Disposition::EvidenceIncomplete);
        assert_eq!(one("allocation_policy_missing").disposition,Disposition::ProfileIncomplete);
    }
    #[test] fn cost_period_must_match_service_horizon(){
        assert_eq!(one("cost_period_mismatch").disposition,Disposition::EvidenceIncomplete);
    }
    #[test] fn currency_unit_mismatch_is_a_case_disposition_not_a_corpus_parse_abort(){
        assert_eq!(one("currency_unit_mismatch").disposition,Disposition::ProfileIncomplete);
    }
    #[test] fn missing_transfer_receipt_reference_is_evidence_incomplete(){
        assert_eq!(
            one("transfer_receipt_missing").disposition,
            Disposition::EvidenceIncomplete
        );
    }
    #[test] fn currency_and_unit_prefix_must_match(){
        assert_eq!(one("currency_unit_mismatch").disposition,Disposition::ProfileIncomplete);
    }
    #[test] fn potable_claim_is_not_inferred_from_hydraulic_service(){
        assert_eq!(one("potable_claim_unsupported").disposition,Disposition::UnsupportedClaim);
    }
    #[test] fn no_zero_denominator_is_hidden(){
        assert_eq!(one("qualified_volume_lower_zero").disposition,Disposition::Indeterminate);
        assert_eq!(one("zero_qualified_service").disposition,Disposition::ZeroQualifiedService);
    }
    #[test] fn reversed_cost_or_energy_intervals_are_invalid(){
        assert_eq!(one("cost_interval_reversed").disposition,Disposition::Invalid);
        assert_eq!(one("energy_interval_reversed").disposition,Disposition::Invalid);
    }
    #[test] fn duplicate_area_identity_is_invalid(){
        assert_eq!(one("duplicate_service_area_id").disposition,Disposition::Invalid);
    }
    #[test] fn transfer_receipt_serial_and_configuration_are_bound(){
        assert_eq!(one("transfer_asset_identity_mismatch").disposition,Disposition::IdentityMismatch);
    }
    #[test] fn report_contains_interval_and_noncompensatory_state(){
        let report=render_report(&run());
        assert!(report.contains("qualified_volume_low_ml"));
        assert!(report.contains("breached_area_count"));
        assert!(report.contains("cost_per_m3_milli_minor_low"));
    }
    #[test] fn report_rows_have_fixed_tsv_width(){
        let report=render_report(&run());
        let widths:BTreeSet<usize>=report.lines().map(|l|l.split('\t').count()).collect();
        assert_eq!(widths,BTreeSet::from([22]));
    }
    #[test] fn cost_and_energy_interval_rounding_is_outward(){
        assert_eq!(floor_ratio_floor(2,1_000_000_000,3),Ok(666_666_666));
        assert_eq!(floor_ratio_ceil(2,1_000_000_000,3),Ok(666_666_667));
    }
    #[test] fn potable_claim_flag_is_fail_closed_on_direct_api(){
        let mut cases=parse_cases(CASES).unwrap();
        cases[0].potable_claim=true;
        let areas=parse_service_areas(AREAS).unwrap();
        assert_eq!(evaluate(&cases,&areas).unwrap().iter().find(|r|r.case_id=="positive_verified_service").unwrap().disposition,Disposition::UnsupportedClaim);
    }
}
