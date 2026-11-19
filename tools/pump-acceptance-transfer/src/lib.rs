//! Conservative pump factory-test to installed-duty transferability evaluator.
//!
//! This evaluator consumes a product-specific acceptance applicability receipt,
//! measured Q-H test points, a separate manufacturer NPSHr declaration, and
//! site/system NPSHa evidence. It does not compute hydraulic network state,
//! authorize installation or actuation, or certify a pump or pressure system.
//!
//! NPSH3, manufacturer NPSHr and site NPSHa are intentionally distinct types/fields.
//! The factory NPSH3 result is never substituted for NPSHr or NPSHa.

use std::collections::{BTreeMap, BTreeSet};

pub const CASES: &str = include_str!("../fixtures/cases-v0.1.tsv");
pub const CURVES: &str = include_str!("../fixtures/curve-points-v0.1.tsv");
pub const EXPECTED: &str = include_str!("../fixtures/expected-v0.1.tsv");

const CASE_HEADER: &str = "case_id\tserial\tconfig_ref\ttest_speed_rpm\ttest_fluid\ttest_boundary_ref\tunit_profile\tapplicability_ref\tapplicability_disposition\tstandard_ref\tstandard_edition\tcatalog_snapshot_date\tassessment_as_of\tfacility_ref\tacceptance_grade\ttest_report_ref\tcalibration_ref\tnpsh3_source_ref\tnpsh3_low_mm\tnpsh3_high_mm\tmanufacturer_curve_ref\tmanufacturer_serial\tmanufacturer_config_ref\tmanufacturer_speed_rpm\tmanufacturer_npshr_low_mm\tmanufacturer_npshr_high_mm\tsite_serial\tsite_config_ref\tsite_speed_rpm\tsite_fluid\tsite_boundary_ref\tflow_ml_s\trequired_head_low_mm\trequired_head_high_mm\tnpsha_source_ref\tnpsha_method_ref\tnpsha_calibration_ref\tnpsha_low_mm\tnpsha_high_mm\tmargin_profile_ref\tmargin_required_mm\trepair_state\tsite_evidence_state";
const CURVE_HEADER: &str = "case_id\tpoint_index\tflow_ml_s\thead_mm\thead_uncertainty_mm";
const EXPECTED_HEADER: &str = "case_id\texpected_disposition";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Case {
    pub case_id: String,
    pub serial: String,
    pub config_ref: String,
    pub test_speed_rpm: u64,
    pub test_fluid: String,
    pub test_boundary_ref: String,
    pub unit_profile: String,
    pub applicability_ref: String,
    pub applicability_disposition: String,
    pub standard_ref: String,
    pub standard_edition: String,
    pub catalog_snapshot_date: String,
    pub assessment_as_of: String,
    pub facility_ref: String,
    pub acceptance_grade: String,
    pub test_report_ref: String,
    pub calibration_ref: String,
    pub npsh3_source_ref: String,
    pub npsh3_low_mm: Option<u64>,
    pub npsh3_high_mm: Option<u64>,
    pub manufacturer_curve_ref: String,
    pub manufacturer_serial: String,
    pub manufacturer_config_ref: String,
    pub manufacturer_speed_rpm: Option<u64>,
    pub manufacturer_npshr_low_mm: Option<u64>,
    pub manufacturer_npshr_high_mm: Option<u64>,
    pub site_serial: String,
    pub site_config_ref: String,
    pub site_speed_rpm: Option<u64>,
    pub site_fluid: String,
    pub site_boundary_ref: String,
    pub flow_ml_s: u64,
    pub required_head_low_mm: Option<u64>,
    pub required_head_high_mm: Option<u64>,
    pub npsha_source_ref: String,
    pub npsha_method_ref: String,
    pub npsha_calibration_ref: String,
    pub npsha_low_mm: Option<u64>,
    pub npsha_high_mm: Option<u64>,
    pub margin_profile_ref: String,
    pub margin_required_mm: Option<u64>,
    pub repair_state: String,
    pub site_evidence_state: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CurvePoint {
    pub case_id: String,
    pub point_index: usize,
    pub flow_ml_s: u64,
    pub head_mm: u64,
    pub head_uncertainty_mm: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Disposition {
    TransferableWithinDeclaredEnvelope,
    NotTransferable,
    InsufficientNpshMargin,
    Indeterminate,
    ProfileIncomplete,
    IdentityMismatch,
    EvidenceIncomplete,
    OutOfDomain,
}

impl Disposition {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::TransferableWithinDeclaredEnvelope => "TRANSFERABLE_WITHIN_DECLARED_ENVELOPE",
            Self::NotTransferable => "NOT_TRANSFERABLE",
            Self::InsufficientNpshMargin => "INSUFFICIENT_NPSH_MARGIN",
            Self::Indeterminate => "INDETERMINATE",
            Self::ProfileIncomplete => "PROFILE_INCOMPLETE",
            Self::IdentityMismatch => "IDENTITY_MISMATCH",
            Self::EvidenceIncomplete => "EVIDENCE_INCOMPLETE",
            Self::OutOfDomain => "OUT_OF_DOMAIN",
        }
    }

    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "TRANSFERABLE_WITHIN_DECLARED_ENVELOPE" => Ok(Self::TransferableWithinDeclaredEnvelope),
            "NOT_TRANSFERABLE" => Ok(Self::NotTransferable),
            "INSUFFICIENT_NPSH_MARGIN" => Ok(Self::InsufficientNpshMargin),
            "INDETERMINATE" => Ok(Self::Indeterminate),
            "PROFILE_INCOMPLETE" => Ok(Self::ProfileIncomplete),
            "IDENTITY_MISMATCH" => Ok(Self::IdentityMismatch),
            "EVIDENCE_INCOMPLETE" => Ok(Self::EvidenceIncomplete),
            "OUT_OF_DOMAIN" => Ok(Self::OutOfDomain),
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
    pub serial: String,
    pub config_ref: String,
    pub applicability_ref: String,
    pub applicability_disposition: String,
    pub standard_ref: String,
    pub standard_edition: String,
    pub catalog_snapshot_date: String,
    pub assessment_as_of: String,
    pub test_fluid: String,
    pub site_fluid: String,
    pub test_boundary_ref: String,
    pub site_boundary_ref: String,
    pub unit_profile: String,
    pub test_speed_rpm: u64,
    pub manufacturer_speed_rpm: Option<u64>,
    pub site_speed_rpm: Option<u64>,
    pub facility_ref: String,
    pub acceptance_grade: String,
    pub test_report_ref: String,
    pub calibration_ref: String,
    pub flow_ml_s: u64,
    pub required_head_low_mm: Option<u64>,
    pub required_head_high_mm: Option<u64>,
    pub interpolated_head_low_mm: Option<u64>,
    pub interpolated_head_high_mm: Option<u64>,
    pub npsh3_source_ref: String,
    pub npsh3_low_mm: Option<u64>,
    pub npsh3_high_mm: Option<u64>,
    pub manufacturer_curve_ref: String,
    pub manufacturer_npshr_low_mm: Option<u64>,
    pub manufacturer_npshr_high_mm: Option<u64>,
    pub npsha_source_ref: String,
    pub npsha_method_ref: String,
    pub npsha_calibration_ref: String,
    pub npsha_low_mm: Option<u64>,
    pub npsha_high_mm: Option<u64>,
    pub margin_profile_ref: String,
    pub margin_required_mm: Option<u64>,
    pub repair_state: String,
    pub site_evidence_state: String,
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
    if year == 0 || !(1..=12).contains(&month) {
        return false;
    }
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let max_day = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return false,
    };
    (1..=max_day).contains(&day)
}

fn split_row<'a>(
    line: &'a str,
    expected: usize,
    source: &str,
    line_no: usize,
) -> Result<Vec<&'a str>, String> {
    let fields: Vec<&str> = line.split('\t').collect();
    if fields.len() != expected {
        return Err(format!("{source}:{line_no}: expected {expected} fields, found {}", fields.len()));
    }
    if fields.iter().any(|field| field.is_empty() || field.trim() != *field) {
        return Err(format!("{source}:{line_no}: empty or padded fields are forbidden"));
    }
    Ok(fields)
}

fn parse_u64(value: &str, source: &str, line_no: usize, field: &str) -> Result<u64, String> {
    value.parse::<u64>()
        .map_err(|_| format!("{source}:{line_no}: invalid {field}: {value}"))
}

fn parse_optional_u64(value: &str, source: &str, line_no: usize, field: &str) -> Result<Option<u64>, String> {
    if value == "-" {
        Ok(None)
    } else {
        parse_u64(value, source, line_no, field).map(Some)
    }
}

pub fn parse_cases(input: &str) -> Result<Vec<Case>, String> {
    let mut lines = input.lines();
    match lines.next() {
        Some(header) if header == CASE_HEADER => {}
        Some(_) => return Err("cases: header mismatch".into()),
        None => return Err("cases: empty input".into()),
    }

    let mut cases = Vec::new();
    let mut ids = BTreeSet::new();
    for (index, line) in lines.enumerate() {
        if line.is_empty() { continue; }
        let row = split_row(line, 43, "cases", index + 2)?;
        for (field, value) in [
            ("case_id", row[0]), ("serial", row[1]), ("config_ref", row[2]),
            ("test_fluid", row[4]), ("test_boundary_ref", row[5]),
            ("unit_profile", row[6]), ("standard_edition", row[10]),
            ("facility_ref", row[13]), ("acceptance_grade", row[14]),
            ("site_fluid", row[29]), ("repair_state", row[40]),
            ("site_evidence_state", row[41]),
        ] {
            if !valid_ref(value) {
                return Err(format!("cases:{}: invalid {field}", index + 2));
            }
        }
        for (field, value) in [
            ("applicability_ref", row[7]), ("standard_ref", row[9]),
            ("test_report_ref", row[15]), ("calibration_ref", row[16]),
            ("npsh3_source_ref", row[17]), ("manufacturer_curve_ref", row[20]),
            ("manufacturer_serial", row[21]), ("manufacturer_config_ref", row[22]),
            ("site_serial", row[26]), ("site_config_ref", row[27]),
            ("site_boundary_ref", row[30]), ("npsha_source_ref", row[34]),
            ("npsha_method_ref", row[35]), ("npsha_calibration_ref", row[36]),
            ("margin_profile_ref", row[39]),
        ] {
            if !valid_optional_ref(value) {
                return Err(format!("cases:{}: invalid {field}", index + 2));
            }
        }
        if !matches!(row[8], "APPLICABLE" | "NOT_APPLICABLE" | "INDETERMINATE" | "PROFILE_INCOMPLETE") {
            return Err(format!("cases:{}: unsupported applicability disposition", index + 2));
        }
        if !valid_date(row[11]) || !valid_date(row[12]) {
            return Err(format!("cases:{}: invalid catalogue or assessment date", index + 2));
        }
        if !ids.insert(row[0].to_owned()) {
            return Err(format!("cases:{}: duplicate case_id {}", index + 2, row[0]));
        }

        let case = Case {
            case_id: row[0].to_owned(),
            serial: row[1].to_owned(),
            config_ref: row[2].to_owned(),
            test_speed_rpm: parse_u64(row[3], "cases", index + 2, "test_speed_rpm")?,
            test_fluid: row[4].to_owned(),
            test_boundary_ref: row[5].to_owned(),
            unit_profile: row[6].to_owned(),
            applicability_ref: row[7].to_owned(),
            applicability_disposition: row[8].to_owned(),
            standard_ref: row[9].to_owned(),
            standard_edition: row[10].to_owned(),
            catalog_snapshot_date: row[11].to_owned(),
            assessment_as_of: row[12].to_owned(),
            facility_ref: row[13].to_owned(),
            acceptance_grade: row[14].to_owned(),
            test_report_ref: row[15].to_owned(),
            calibration_ref: row[16].to_owned(),
            npsh3_source_ref: row[17].to_owned(),
            npsh3_low_mm: parse_optional_u64(row[18], "cases", index + 2, "npsh3_low_mm")?,
            npsh3_high_mm: parse_optional_u64(row[19], "cases", index + 2, "npsh3_high_mm")?,
            manufacturer_curve_ref: row[20].to_owned(),
            manufacturer_serial: row[21].to_owned(),
            manufacturer_config_ref: row[22].to_owned(),
            manufacturer_speed_rpm: parse_optional_u64(row[23], "cases", index + 2, "manufacturer_speed_rpm")?,
            manufacturer_npshr_low_mm: parse_optional_u64(row[24], "cases", index + 2, "manufacturer_npshr_low_mm")?,
            manufacturer_npshr_high_mm: parse_optional_u64(row[25], "cases", index + 2, "manufacturer_npshr_high_mm")?,
            site_serial: row[26].to_owned(),
            site_config_ref: row[27].to_owned(),
            site_speed_rpm: parse_optional_u64(row[28], "cases", index + 2, "site_speed_rpm")?,
            site_fluid: row[29].to_owned(),
            site_boundary_ref: row[30].to_owned(),
            flow_ml_s: parse_u64(row[31], "cases", index + 2, "flow_ml_s")?,
            required_head_low_mm: parse_optional_u64(row[32], "cases", index + 2, "required_head_low_mm")?,
            required_head_high_mm: parse_optional_u64(row[33], "cases", index + 2, "required_head_high_mm")?,
            npsha_source_ref: row[34].to_owned(),
            npsha_method_ref: row[35].to_owned(),
            npsha_calibration_ref: row[36].to_owned(),
            npsha_low_mm: parse_optional_u64(row[37], "cases", index + 2, "npsha_low_mm")?,
            npsha_high_mm: parse_optional_u64(row[38], "cases", index + 2, "npsha_high_mm")?,
            margin_profile_ref: row[39].to_owned(),
            margin_required_mm: parse_optional_u64(row[40], "cases", index + 2, "margin_required_mm")?,
            repair_state: row[41].to_owned(),
            site_evidence_state: row[42].to_owned(),
        };
        validate_case(&case)?;
        cases.push(case);
    }
    if cases.is_empty() { return Err("cases: no case rows".into()); }
    Ok(cases)
}

fn validate_case(case: &Case) -> Result<(), String> {
    if !valid_ref(&case.case_id) || !valid_ref(&case.serial) || !valid_ref(&case.config_ref) {
        return Err(format!("case {}: invalid direct identity", case.case_id));
    }
    if !valid_date(&case.catalog_snapshot_date) || !valid_date(&case.assessment_as_of) {
        return Err(format!("case {}: invalid direct date", case.case_id));
    }
    for (field, value) in [
        ("test_fluid", case.test_fluid.as_str()),
        ("test_boundary_ref", case.test_boundary_ref.as_str()),
        ("site_fluid", case.site_fluid.as_str()),
        ("repair_state", case.repair_state.as_str()),
        ("site_evidence_state", case.site_evidence_state.as_str()),
    ] {
        if !valid_ref(value) {
            return Err(format!("case {}: invalid direct {field}", case.case_id));
        }
    }
    if case.test_speed_rpm == 0 || case.flow_ml_s == 0 {
        return Err(format!("case {}: speed and flow must be positive", case.case_id));
    }
    if !valid_optional_ref(&case.applicability_ref)
        || !valid_optional_ref(&case.standard_ref)
        || !valid_optional_ref(&case.test_report_ref)
        || !valid_optional_ref(&case.calibration_ref)
        || !valid_optional_ref(&case.npsh3_source_ref)
        || !valid_optional_ref(&case.manufacturer_curve_ref)
        || !valid_optional_ref(&case.npsha_source_ref)
        || !valid_optional_ref(&case.npsha_method_ref)
        || !valid_optional_ref(&case.npsha_calibration_ref)
        || !valid_optional_ref(&case.margin_profile_ref)
    {
        return Err(format!("case {}: malformed direct evidence reference", case.case_id));
    }
    Ok(())
}

pub fn parse_curve_points(input: &str) -> Result<Vec<CurvePoint>, String> {
    let mut lines = input.lines();
    match lines.next() {
        Some(header) if header == CURVE_HEADER => {}
        Some(_) => return Err("curve points: header mismatch".into()),
        None => return Err("curve points: empty input".into()),
    }
    let mut points = Vec::new();
    for (index, line) in lines.enumerate() {
        if line.is_empty() { continue; }
        let row = split_row(line, 5, "curve points", index + 2)?;
        if !valid_ref(row[0]) {
            return Err(format!("curve points:{}: invalid case_id", index + 2));
        }
        let point = CurvePoint {
            case_id: row[0].to_owned(),
            point_index: usize::try_from(parse_u64(row[1], "curve points", index + 2, "point_index")?)
                .map_err(|_| format!("curve points:{}: point_index exceeds platform range", index + 2))?,
            flow_ml_s: parse_u64(row[2], "curve points", index + 2, "flow_ml_s")?,
            head_mm: parse_u64(row[3], "curve points", index + 2, "head_mm")?,
            head_uncertainty_mm: parse_u64(row[4], "curve points", index + 2, "head_uncertainty_mm")?,
        };
        if point.flow_ml_s == 0 || point.head_mm == 0 {
            return Err(format!("curve points:{}: flow and head must be positive", index + 2));
        }
        points.push(point);
    }
    if points.is_empty() { return Err("curve points: no rows".into()); }
    Ok(points)
}

pub fn parse_expected(input: &str) -> Result<Vec<ExpectedRow>, String> {
    let mut lines = input.lines();
    match lines.next() {
        Some(header) if header == EXPECTED_HEADER => {}
        Some(_) => return Err("expected: header mismatch".into()),
        None => return Err("expected: empty input".into()),
    }
    let mut rows = Vec::new();
    let mut seen = BTreeSet::new();
    for (index, line) in lines.enumerate() {
        if line.is_empty() { continue; }
        let row = split_row(line, 2, "expected", index + 2)?;
        if !valid_ref(row[0]) || !seen.insert(row[0].to_owned()) {
            return Err(format!("expected:{}: invalid or duplicate case_id", index + 2));
        }
        rows.push(ExpectedRow {
            case_id: row[0].to_owned(),
            disposition: Disposition::parse(row[1])?,
        });
    }
    Ok(rows)
}

fn interpolate_floor(a: u64, b: u64, numerator: u64, denominator: u64) -> Option<u64> {
    if denominator == 0 || numerator > denominator { return None; }
    let n = (a as u128)
        .checked_mul((denominator - numerator) as u128)?
        .checked_add((b as u128).checked_mul(numerator as u128)?)?;
    u64::try_from(n / denominator as u128).ok()
}

fn interpolate_ceil(a: u64, b: u64, numerator: u64, denominator: u64) -> Option<u64> {
    if denominator == 0 || numerator > denominator { return None; }
    let n = (a as u128)
        .checked_mul((denominator - numerator) as u128)?
        .checked_add((b as u128).checked_mul(numerator as u128)?)?;
    let d = denominator as u128;
    let rounded = n.checked_add(d - 1)?;
    u64::try_from(rounded / d).ok()
}

fn curve_head_interval(points: &[CurvePoint], flow: u64) -> Result<(u64, u64), Disposition> {
    if points.len() < 2 {
        return Err(Disposition::ProfileIncomplete);
    }
    for (i, point) in points.iter().enumerate() {
        if point.point_index != i || point.flow_ml_s == 0 || point.head_mm == 0 {
            return Err(Disposition::ProfileIncomplete);
        }
        if i > 0 && points[i - 1].flow_ml_s >= point.flow_ml_s {
            return Err(Disposition::ProfileIncomplete);
        }
    }
    if flow < points[0].flow_ml_s || flow > points[points.len() - 1].flow_ml_s {
        return Err(Disposition::OutOfDomain);
    }
    if let Some(point) = points.iter().find(|point| point.flow_ml_s == flow) {
        return Ok((
            point.head_mm.saturating_sub(point.head_uncertainty_mm),
            point.head_mm.saturating_add(point.head_uncertainty_mm),
        ));
    }

    for pair in points.windows(2) {
        let left = &pair[0];
        let right = &pair[1];
        if left.flow_ml_s < flow && flow < right.flow_ml_s {
            let denominator = right.flow_ml_s - left.flow_ml_s;
            let numerator = flow - left.flow_ml_s;
            let left_low = left.head_mm.saturating_sub(left.head_uncertainty_mm);
            let right_low = right.head_mm.saturating_sub(right.head_uncertainty_mm);
            let left_high = left.head_mm.checked_add(left.head_uncertainty_mm)
                .ok_or(Disposition::ProfileIncomplete)?;
            let right_high = right.head_mm.checked_add(right.head_uncertainty_mm)
                .ok_or(Disposition::ProfileIncomplete)?;
            let low = interpolate_floor(left_low, right_low, numerator, denominator)
                .ok_or(Disposition::ProfileIncomplete)?;
            let high = interpolate_ceil(left_high, right_high, numerator, denominator)
                .ok_or(Disposition::ProfileIncomplete)?;
            return Ok((low, high));
        }
    }
    Err(Disposition::OutOfDomain)
}

fn eval_one(case: &Case, points: &[CurvePoint]) -> Evaluation {
    let mut result = Evaluation {
        case_id: case.case_id.clone(),
        disposition: Disposition::ProfileIncomplete,
        reason: "transferability profile is incomplete".into(),
        serial: case.serial.clone(),
        config_ref: case.config_ref.clone(),
        applicability_ref: case.applicability_ref.clone(),
        applicability_disposition: case.applicability_disposition.clone(),
        standard_ref: case.standard_ref.clone(),
        standard_edition: case.standard_edition.clone(),
        catalog_snapshot_date: case.catalog_snapshot_date.clone(),
        assessment_as_of: case.assessment_as_of.clone(),
        test_fluid: case.test_fluid.clone(),
        site_fluid: case.site_fluid.clone(),
        test_boundary_ref: case.test_boundary_ref.clone(),
        site_boundary_ref: case.site_boundary_ref.clone(),
        unit_profile: case.unit_profile.clone(),
        test_speed_rpm: case.test_speed_rpm,
        manufacturer_speed_rpm: case.manufacturer_speed_rpm,
        site_speed_rpm: case.site_speed_rpm,
        facility_ref: case.facility_ref.clone(),
        acceptance_grade: case.acceptance_grade.clone(),
        test_report_ref: case.test_report_ref.clone(),
        calibration_ref: case.calibration_ref.clone(),
        flow_ml_s: case.flow_ml_s,
        required_head_low_mm: case.required_head_low_mm,
        required_head_high_mm: case.required_head_high_mm,
        interpolated_head_low_mm: None,
        interpolated_head_high_mm: None,
        npsh3_source_ref: case.npsh3_source_ref.clone(),
        npsh3_low_mm: case.npsh3_low_mm,
        npsh3_high_mm: case.npsh3_high_mm,
        manufacturer_curve_ref: case.manufacturer_curve_ref.clone(),
        manufacturer_npshr_low_mm: case.manufacturer_npshr_low_mm,
        manufacturer_npshr_high_mm: case.manufacturer_npshr_high_mm,
        npsha_source_ref: case.npsha_source_ref.clone(),
        npsha_method_ref: case.npsha_method_ref.clone(),
        npsha_calibration_ref: case.npsha_calibration_ref.clone(),
        npsha_low_mm: case.npsha_low_mm,
        npsha_high_mm: case.npsha_high_mm,
        margin_profile_ref: case.margin_profile_ref.clone(),
        margin_required_mm: case.margin_required_mm,
        repair_state: case.repair_state.clone(),
        site_evidence_state: case.site_evidence_state.clone(),
    };

    macro_rules! finish {
        ($d:expr, $r:expr) => {{
            result.disposition = $d;
            result.reason = $r.into();
            return result;
        }};
    }

    if case.unit_profile != "SI_MLPS_MM" {
        finish!(Disposition::ProfileIncomplete, "unsupported or undeclared flow/head/NPSH unit profile");
    }
    if case.applicability_disposition == "NOT_APPLICABLE" {
        finish!(Disposition::NotTransferable, "factory acceptance standard/profile is not applicable to this product/test scope");
    }
    if case.applicability_disposition != "APPLICABLE" {
        finish!(Disposition::ProfileIncomplete, "factory acceptance applicability is not established");
    }
    if case.catalog_snapshot_date != case.assessment_as_of {
        finish!(Disposition::EvidenceIncomplete, "catalogue snapshot does not cover the requested assessment date");
    }
    for value in [
        case.applicability_ref.as_str(),
        case.standard_ref.as_str(),
        case.facility_ref.as_str(),
        case.acceptance_grade.as_str(),
        case.test_report_ref.as_str(),
        case.calibration_ref.as_str(),
        case.npsh3_source_ref.as_str(),
    ] {
        if !valid_ref(value) {
            finish!(Disposition::EvidenceIncomplete, "required factory applicability, facility, grade, report, calibration or NPSH3 evidence is missing");
        }
    }
    if case.repair_state != "NO_POST_TEST_CHANGE" {
        finish!(Disposition::EvidenceIncomplete, "product was changed/repaired after test without a bound post-change retest");
    }
    if case.site_evidence_state != "COMPLETE" {
        finish!(Disposition::EvidenceIncomplete, "site/system evidence is missing, stale or otherwise incomplete");
    }

    if case.serial != case.site_serial
        || case.serial != case.manufacturer_serial
        || case.config_ref != case.site_config_ref
        || case.config_ref != case.manufacturer_config_ref
    {
        finish!(Disposition::IdentityMismatch, "factory test, manufacturer declaration and site duty do not bind the same serial/configuration");
    }
    if case.test_fluid != case.site_fluid {
        finish!(Disposition::NotTransferable, "installed fluid profile differs from the factory test fluid profile");
    }
    if case.test_boundary_ref != case.site_boundary_ref {
        finish!(Disposition::NotTransferable, "factory curve and installed duty use different hydraulic boundaries");
    }
    let manufacturer_speed = match case.manufacturer_speed_rpm {
        Some(value) if value > 0 => value,
        _ => finish!(Disposition::EvidenceIncomplete, "manufacturer NPSHr curve speed is missing"),
    };
    let site_speed = match case.site_speed_rpm {
        Some(value) if value > 0 => value,
        _ => finish!(Disposition::EvidenceIncomplete, "site duty speed is missing"),
    };
    if !valid_ref(&case.manufacturer_curve_ref)
        || case.manufacturer_curve_ref == "-"
        || case.manufacturer_npshr_low_mm.is_none()
        || case.manufacturer_npshr_high_mm.is_none()
    {
        finish!(Disposition::EvidenceIncomplete, "manufacturer-supplied NPSHr curve and interval are missing");
    }
    if case.test_speed_rpm != site_speed || case.test_speed_rpm != manufacturer_speed {
        finish!(Disposition::OutOfDomain, "site or manufacturer NPSHr speed differs from the exact tested speed; no speed-law correction is admitted");
    }

    let (npsh3_low, npsh3_high) = match (case.npsh3_low_mm, case.npsh3_high_mm) {
        (Some(low), Some(high)) if low <= high => (low, high),
        (Some(_), Some(_)) => finish!(Disposition::ProfileIncomplete, "NPSH3 test interval is malformed"),
        _ => finish!(Disposition::EvidenceIncomplete, "factory NPSH3 result is missing"),
    };
    let _npsh3_test_interval = (npsh3_low, npsh3_high);

    if !valid_ref(&case.manufacturer_curve_ref)
        || case.manufacturer_curve_ref == case.npsh3_source_ref
    {
        finish!(Disposition::EvidenceIncomplete, "manufacturer NPSHr curve must have a distinct source reference from factory NPSH3");
    }
    let (npshr_low, npshr_high) = match (
        case.manufacturer_npshr_low_mm,
        case.manufacturer_npshr_high_mm,
    ) {
        (Some(low), Some(high)) if low <= high => (low, high),
        (Some(_), Some(_)) => finish!(Disposition::ProfileIncomplete, "manufacturer NPSHr interval is malformed"),
        _ => finish!(Disposition::EvidenceIncomplete, "manufacturer-supplied NPSHr interval is missing"),
    };
    let (npsha_low, npsha_high) = match (case.npsha_low_mm, case.npsha_high_mm) {
        (Some(low), Some(high)) if low <= high => (low, high),
        (Some(_), Some(_)) => finish!(Disposition::ProfileIncomplete, "site NPSHa interval is malformed"),
        _ => finish!(Disposition::EvidenceIncomplete, "site-specific NPSHa interval is missing"),
    };
    if !valid_ref(&case.npsha_source_ref)
        || !valid_ref(&case.npsha_method_ref)
        || !valid_ref(&case.npsha_calibration_ref)
    {
        finish!(Disposition::EvidenceIncomplete, "site NPSHa requires an exact source, method and site-side calibration reference");
    }
    let margin = match case.margin_required_mm {
        Some(value) => value,
        None => finish!(Disposition::ProfileIncomplete, "application-specific NPSH margin is missing"),
    };
    if !valid_ref(&case.margin_profile_ref) {
        finish!(Disposition::ProfileIncomplete, "application-specific NPSH margin profile reference is missing");
    }
    if case.required_head_low_mm.is_none()
        || case.required_head_high_mm.is_none()
    {
        finish!(Disposition::ProfileIncomplete, "required head interval is missing");
    }
    let required_head_low = case.required_head_low_mm.unwrap_or(0);
    let required_head_high = case.required_head_high_mm.unwrap_or(0);
    if required_head_low > required_head_high {
        finish!(Disposition::ProfileIncomplete, "required head interval is malformed");
    }

    let (pump_head_low, pump_head_high) = match curve_head_interval(points, case.flow_ml_s) {
        Ok(interval) => interval,
        Err(disposition) => finish!(disposition, "required flow is outside measured curve coverage or curve points are invalid"),
    };
    result.interpolated_head_low_mm = Some(pump_head_low);
    result.interpolated_head_high_mm = Some(pump_head_high);

    if pump_head_high < required_head_low {
        finish!(Disposition::NotTransferable, "upper measured pump-head bound is below the required system-head interval");
    }
    if pump_head_low < required_head_high {
        finish!(Disposition::Indeterminate, "measured pump-head and required system-head intervals overlap");
    }

    let required_npsha_low = match npshr_low.checked_add(margin) {
        Some(value) => value,
        None => finish!(Disposition::ProfileIncomplete, "NPSHr plus application margin exceeds the representable unit range"),
    };
    let required_npsha_high = match npshr_high.checked_add(margin) {
        Some(value) => value,
        None => finish!(Disposition::ProfileIncomplete, "NPSHr plus application margin exceeds the representable unit range"),
    };
    if npsha_high < required_npsha_low {
        finish!(Disposition::InsufficientNpshMargin, "site NPSHa upper bound is below manufacturer NPSHr plus the approved application margin");
    }
    if npsha_low < required_npsha_high {
        finish!(Disposition::Indeterminate, "site NPSHa interval overlaps manufacturer NPSHr plus the approved application margin");
    }

    finish!(
        Disposition::TransferableWithinDeclaredEnvelope,
        "exact standard, serial, configuration, fluid, speed, boundary, measured Q-H domain and separately sourced NPSH margin evidence support this declared duty"
    );
}

pub fn evaluate(
    cases: &[Case],
    curve_points: &[CurvePoint],
) -> Result<Vec<Evaluation>, String> {
    if cases.is_empty() { return Err("evaluator input contains no cases".into()); }

    let mut cases_by_id: BTreeMap<&str, &Case> = BTreeMap::new();
    for case in cases {
        validate_case(case)?;
        if cases_by_id.insert(case.case_id.as_str(), case).is_some() {
            return Err(format!("duplicate case_id: {}", case.case_id));
        }
    }

    let mut curves_by_case: BTreeMap<&str, Vec<&CurvePoint>> = BTreeMap::new();
    for point in curve_points {
        if !cases_by_id.contains_key(point.case_id.as_str()) {
            return Err(format!("orphan curve point for case {}", point.case_id));
        }
        curves_by_case.entry(point.case_id.as_str()).or_default().push(point);
    }

    let mut results = Vec::with_capacity(cases.len());
    for case in cases {
        let mut owned_points: Vec<CurvePoint> = curves_by_case
            .get(case.case_id.as_str())
            .into_iter()
            .flat_map(|rows| rows.iter().map(|row| (**row).clone()))
            .collect();
        owned_points.sort_by_key(|row| row.point_index);
        results.push(eval_one(case, &owned_points));
    }
    Ok(results)
}

pub fn verify_expected(actual: &[Evaluation], expected: &[ExpectedRow]) -> Result<(), String> {
    if actual.len() != expected.len() {
        return Err(format!("expected {} results, got {}", expected.len(), actual.len()));
    }
    let actual_by_id: BTreeMap<&str, &Evaluation> = actual
        .iter()
        .map(|row| (row.case_id.as_str(), row))
        .collect();
    for row in expected {
        let got = actual_by_id.get(row.case_id.as_str())
            .ok_or_else(|| format!("missing result for {}", row.case_id))?;
        if got.disposition != row.disposition {
            return Err(format!(
                "{}: expected {}, got {} ({})",
                row.case_id, row.disposition.as_str(), got.disposition.as_str(), got.reason
            ));
        }
    }
    Ok(())
}

pub fn evaluate_corpus(
    cases_tsv: &str,
    curves_tsv: &str,
    expected_tsv: &str,
) -> Result<Vec<Evaluation>, String> {
    let cases = parse_cases(cases_tsv)?;
    let curves = parse_curve_points(curves_tsv)?;
    let expected = parse_expected(expected_tsv)?;
    let results = evaluate(&cases, &curves)?;
    verify_expected(&results, &expected)?;
    Ok(results)
}

fn render_optional_u64(value: Option<u64>) -> String {
    value.map_or_else(|| "-".to_owned(), |number| number.to_string())
}

pub fn render_report(results: &[Evaluation]) -> String {
    let mut output = String::from("case_id\tdisposition\treason\tserial\tconfig_ref\tapplicability_ref\tapplicability_disposition\tstandard_ref\tstandard_edition\tcatalog_snapshot_date\tassessment_as_of\ttest_fluid\tsite_fluid\ttest_boundary_ref\tsite_boundary_ref\tunit_profile\ttest_speed_rpm\tmanufacturer_speed_rpm\tsite_speed_rpm\tfacility_ref\tacceptance_grade\ttest_report_ref\ttest_calibration_ref\tflow_ml_s\trequired_head_low_mm\trequired_head_high_mm\tpump_head_low_mm\tpump_head_high_mm\tnpsh3_source_ref\tnpsh3_low_mm\tnpsh3_high_mm\tmanufacturer_curve_ref\tmanufacturer_npshr_low_mm\tmanufacturer_npshr_high_mm\tnpsha_source_ref\tnpsha_method_ref\tnpsha_calibration_ref\tnpsha_low_mm\tnpsha_high_mm\tmargin_profile_ref\tmargin_required_mm\trepair_state\tsite_evidence_state\n");
    for row in results {
        output.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            row.case_id,
            row.disposition.as_str(),
            row.reason,
            row.serial,
            row.config_ref,
            row.applicability_ref,
            row.applicability_disposition,
            row.standard_ref,
            row.standard_edition,
            row.catalog_snapshot_date,
            row.assessment_as_of,
            row.test_fluid,
            row.site_fluid,
            row.test_boundary_ref,
            row.site_boundary_ref,
            row.unit_profile,
            row.test_speed_rpm.to_string(),
            render_optional_u64(row.manufacturer_speed_rpm),
            render_optional_u64(row.site_speed_rpm),
            row.facility_ref,
            row.acceptance_grade,
            row.test_report_ref,
            row.calibration_ref,
            row.flow_ml_s.to_string(),
            render_optional_u64(row.required_head_low_mm),
            render_optional_u64(row.required_head_high_mm),
            render_optional_u64(row.interpolated_head_low_mm),
            render_optional_u64(row.interpolated_head_high_mm),
            row.npsh3_source_ref,
            render_optional_u64(row.npsh3_low_mm),
            render_optional_u64(row.npsh3_high_mm),
            row.manufacturer_curve_ref,
            render_optional_u64(row.manufacturer_npshr_low_mm),
            render_optional_u64(row.manufacturer_npshr_high_mm),
            row.npsha_source_ref,
            row.npsha_method_ref,
            row.npsha_calibration_ref,
            render_optional_u64(row.npsha_low_mm),
            render_optional_u64(row.npsha_high_mm),
            row.margin_profile_ref,
            render_optional_u64(row.margin_required_mm),
            row.repair_state,
            row.site_evidence_state
        ));
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run() -> Vec<Evaluation> {
        evaluate_corpus(CASES, CURVES, EXPECTED).unwrap()
    }

    fn one(id: &str) -> Evaluation {
        run().into_iter().find(|row| row.case_id == id).unwrap()
    }

    #[test]
    fn frozen_thirty_one_case_oracle_matches() {
        assert_eq!(run().len(), 31);
    }

    #[test]
    fn positive_bracketed_curve_is_transferable() {
        assert_eq!(
            one("positive_bracketed_curve").disposition,
            Disposition::TransferableWithinDeclaredEnvelope
        );
    }

    #[test]
    fn npsh3_is_not_substituted_for_manufacturer_npshr() {
        assert_eq!(
            one("npshr_copied_from_npsh3_source").disposition,
            Disposition::EvidenceIncomplete
        );
    }

    #[test]
    fn npsha_is_not_inferred_from_factory_acceptance() {
        assert_eq!(
            one("site_npsha_missing").disposition,
            Disposition::EvidenceIncomplete
        );
    }

    #[test]
    fn adequate_npsh_margin_is_separately_required() {
        assert_eq!(
            one("npsh_margin_clears").disposition,
            Disposition::TransferableWithinDeclaredEnvelope
        );
    }

    #[test]
    fn insufficient_npsh_margin_is_noncompensatory() {
        assert_eq!(
            one("npsh_margin_insufficient").disposition,
            Disposition::InsufficientNpshMargin
        );
    }

    #[test]
    fn overlapping_npsh_intervals_are_indeterminate() {
        assert_eq!(
            one("npsh_margin_interval_overlap").disposition,
            Disposition::Indeterminate
        );
    }

    #[test]
    fn qh_extrapolation_is_never_admitted() {
        assert_eq!(one("flow_above_curve_domain").disposition, Disposition::OutOfDomain);
        assert_eq!(one("flow_below_curve_domain").disposition, Disposition::OutOfDomain);
    }

    #[test]
    fn duplicate_flow_points_are_rejected() {
        assert_eq!(one("duplicate_curve_flow").disposition, Disposition::ProfileIncomplete);
    }

    #[test]
    fn head_interval_overlap_is_indeterminate() {
        assert_eq!(one("pump_head_interval_overlap").disposition, Disposition::Indeterminate);
    }

    #[test]
    fn pump_curve_below_required_head_is_not_transferable() {
        assert_eq!(one("pump_head_below_required").disposition, Disposition::NotTransferable);
    }

    #[test]
    fn serial_and_configuration_are_exactly_bound() {
        assert_eq!(one("serial_mismatch").disposition, Disposition::IdentityMismatch);
        assert_eq!(one("site_configuration_mismatch").disposition, Disposition::IdentityMismatch);
    }

    #[test]
    fn manufacturer_serial_and_configuration_are_exactly_bound() {
        assert_eq!(one("manufacturer_serial_mismatch").disposition, Disposition::IdentityMismatch);
        assert_eq!(one("manufacturer_configuration_mismatch").disposition, Disposition::IdentityMismatch);
    }

    #[test]
    fn test_and_site_speeds_must_match_without_affinity_law_assumption() {
        assert_eq!(one("site_speed_outside_test").disposition, Disposition::OutOfDomain);
        assert_eq!(one("manufacturer_speed_mismatch").disposition, Disposition::OutOfDomain);
    }

    #[test]
    fn fluids_and_hydraulic_boundaries_must_match() {
        assert_eq!(one("fluid_profile_mismatch").disposition, Disposition::NotTransferable);
        assert_eq!(one("test_boundary_mismatch").disposition, Disposition::NotTransferable);
    }

    #[test]
    fn non_applicable_factory_standard_cannot_support_transfer() {
        assert_eq!(one("standard_scope_not_applicable").disposition, Disposition::NotTransferable);
    }

    #[test]
    fn current_catalogue_snapshot_cannot_claim_historical_status() {
        assert_eq!(one("catalogue_snapshot_mismatch").disposition, Disposition::EvidenceIncomplete);
    }

    #[test]
    fn missing_calibration_or_stale_site_evidence_cannot_pass() {
        assert_eq!(one("calibration_missing").disposition, Disposition::EvidenceIncomplete);
        assert_eq!(one("stale_site_evidence").disposition, Disposition::EvidenceIncomplete);
    }

    #[test]
    fn post_test_repair_requires_new_test_evidence() {
        assert_eq!(one("repaired_without_posttest_retest").disposition, Disposition::EvidenceIncomplete);
    }

    #[test]
    fn npshr_and_npsha_intervals_are_required_and_ordered() {
        assert_eq!(one("manufacturer_npshr_missing").disposition, Disposition::EvidenceIncomplete);
        assert_eq!(one("site_npsha_missing").disposition, Disposition::EvidenceIncomplete);
        assert_eq!(one("invalid_npshr_interval").disposition, Disposition::ProfileIncomplete);
        assert_eq!(one("invalid_npsha_interval").disposition, Disposition::ProfileIncomplete);
    }

    #[test]
    fn missing_site_npsha_calibration_cannot_pass() {
        assert_eq!(
            one("site_npsha_calibration_missing").disposition,
            Disposition::EvidenceIncomplete
        );
    }

    #[test]
    fn npsh3_interval_must_be_separately_present_and_ordered() {
        assert_eq!(one("invalid_npsh3_interval").disposition, Disposition::ProfileIncomplete);
    }

    #[test]
    fn approved_margin_profile_is_required() {
        assert_eq!(one("margin_profile_missing").disposition, Disposition::ProfileIncomplete);
    }

    #[test]
    fn unsupported_unit_profile_does_not_get_coerced() {
        assert_eq!(one("unsupported_unit_profile").disposition, Disposition::ProfileIncomplete);
    }

    #[test]
    fn required_head_interval_must_be_ordered() {
        assert_eq!(one("invalid_required_head_interval").disposition, Disposition::ProfileIncomplete);
    }

    #[test]
    fn report_carries_source_and_npsh_lineage() {
        let report = render_report(&run());
        assert!(report.contains("npsh3_source_ref"));
        assert!(report.contains("manufacturer_curve_ref"));
        assert!(report.contains("npsha_source_ref"));
        assert!(report.contains("npsha_method_ref"));
        assert!(report.contains("npsha_calibration_ref"));
        assert!(report.contains("test_speed_rpm"));
        assert!(report.contains("site_speed_rpm"));
        assert!(report.contains("required_head_low_mm"));
        assert!(report.contains("repair_state"));
        assert!(report.contains("applicability:positive_bracketed_curve"));
    }

    #[test]
    fn all_report_rows_have_same_tsv_width() {
        let report = render_report(&run());
        let widths: BTreeSet<usize> = report.lines().map(|line| line.split('\t').count()).collect();
        assert_eq!(widths, BTreeSet::from([43]));
    }

    #[test]
    fn direct_api_rejects_invalid_test_date() {
        let mut cases = parse_cases(CASES).unwrap();
        cases[0].assessment_as_of = "2026-02-31".into();
        assert!(evaluate(&cases, &parse_curve_points(CURVES).unwrap())
            .unwrap_err()
            .contains("invalid direct date"));
    }

    #[test]
    fn deterministic_output_is_independent_of_case_order() {
        let mut cases = parse_cases(CASES).unwrap();
        let curves = parse_curve_points(CURVES).unwrap();
        let forward = render_report(&evaluate(&cases, &curves).unwrap());
        cases.reverse();
        let reverse = render_report(&evaluate(&cases, &curves).unwrap());
        assert_eq!(forward.lines().skip(1).collect::<BTreeSet<_>>(),
                   reverse.lines().skip(1).collect::<BTreeSet<_>>());
    }

    #[test]
    fn interpolation_is_conservatively_rounded() {
        assert_eq!(interpolate_floor(10, 11, 1, 2), Some(10));
        assert_eq!(interpolate_ceil(10, 11, 1, 2), Some(11));
    }

    #[test]
    fn invalid_curve_order_is_profile_incomplete() {
        let points = vec![
            CurvePoint { case_id:"c".into(),point_index:0,flow_ml_s:1000,head_mm:10000,head_uncertainty_mm:10 },
            CurvePoint { case_id:"c".into(),point_index:1,flow_ml_s:900,head_mm:9000,head_uncertainty_mm:10 },
        ];
        assert_eq!(curve_head_interval(&points,950),Err(Disposition::ProfileIncomplete));
    }
}
