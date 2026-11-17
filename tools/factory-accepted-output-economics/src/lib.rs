//! Synthetic accepted-output economics accounting for matched factory routes.
//!
//! This crate does not decide route capability, product conformity, source truth,
//! currency conversion, accounting policy or production release authority. It
//! consumes route dispositions, cost-item evidence and final output dispositions.
//! Synthetic fixture inputs are not real monetary or industrial measurements.

use std::collections::{BTreeMap, BTreeSet};

pub const CASES: &str = include_str!("../fixtures/cases-v0.1.tsv");
pub const COST_ITEMS: &str = include_str!("../fixtures/cost-items-v0.1.tsv");
pub const OUTPUTS: &str = include_str!("../fixtures/outputs-v0.1.tsv");
pub const EXPECTED: &str = include_str!("../fixtures/expected-v0.1.tsv");

const CASE_HEADER: &str = "case_id\troute_ref\troute_status\tproduct_profile_ref\tacceptance_profile_ref\tbatch_ref\tcurrency\tprice_date\tamount_unit";
const COST_HEADER: &str = "case_id\tcost_item_id\tcategory\tamount_minor_low\tamount_minor_high\tcurrency\tprice_date\tamount_unit\tevidence_class\tscope\tsource_ref";
const OUTPUT_HEADER: &str = "case_id\toutput_id\tfinal_disposition\trelease_ref";
const EXPECTED_HEADER: &str = "case_id\tdisposition\ttotal_low_minor\ttotal_high_minor\tstarted\tfirst_pass_released\tfinal_released";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RouteStatus {
    Eligible,
    Ineligible,
    Unknown,
}

impl RouteStatus {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "ELIGIBLE" => Ok(Self::Eligible),
            "INELIGIBLE" => Ok(Self::Ineligible),
            "UNKNOWN" => Ok(Self::Unknown),
            _ => Err(format!("unknown route status: {value}")),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CaseManifest {
    pub case_id: String,
    pub route_ref: String,
    pub route_status: RouteStatus,
    pub product_profile_ref: String,
    pub acceptance_profile_ref: String,
    pub batch_ref: String,
    pub currency: String,
    pub price_date: String,
    pub amount_unit: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CostItem {
    pub case_id: String,
    pub cost_item_id: String,
    pub category: String,
    pub amount_minor_low: Option<u64>,
    pub amount_minor_high: Option<u64>,
    pub currency: String,
    pub price_date: String,
    pub amount_unit: String,
    pub evidence_class: String,
    pub scope: String,
    pub source_ref: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FinalDisposition {
    ReleasedFirstPass,
    ReleasedAfterRework,
    Scrap,
    WorkInProgress,
    AwaitingInspection,
    Indeterminate,
}

impl FinalDisposition {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "RELEASED_FIRST_PASS" => Ok(Self::ReleasedFirstPass),
            "RELEASED_AFTER_REWORK" => Ok(Self::ReleasedAfterRework),
            "SCRAP" => Ok(Self::Scrap),
            "WIP" => Ok(Self::WorkInProgress),
            "AWAITING_INSPECTION" => Ok(Self::AwaitingInspection),
            "INDETERMINATE" => Ok(Self::Indeterminate),
            _ => Err(format!("unknown final disposition: {value}")),
        }
    }

    fn is_released(self) -> bool {
        matches!(self, Self::ReleasedFirstPass | Self::ReleasedAfterRework)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OutputRecord {
    pub case_id: String,
    pub output_id: String,
    pub final_disposition: FinalDisposition,
    pub release_ref: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvaluationDisposition {
    Computed,
    CostIncomplete,
    ZeroReleased,
    RouteIneligible,
    RouteUnknown,
}

impl EvaluationDisposition {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Computed => "COMPUTED",
            Self::CostIncomplete => "COST_INCOMPLETE",
            Self::ZeroReleased => "ZERO_RELEASED",
            Self::RouteIneligible => "ROUTE_INELIGIBLE",
            Self::RouteUnknown => "ROUTE_UNKNOWN",
        }
    }

    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "COMPUTED" => Ok(Self::Computed),
            "COST_INCOMPLETE" => Ok(Self::CostIncomplete),
            "ZERO_RELEASED" => Ok(Self::ZeroReleased),
            "ROUTE_INELIGIBLE" => Ok(Self::RouteIneligible),
            "ROUTE_UNKNOWN" => Ok(Self::RouteUnknown),
            _ => Err(format!("unknown expected disposition: {value}")),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CostBreakdown {
    pub category: String,
    pub evidence_class: String,
    pub amount_low_minor: u64,
    pub amount_high_minor: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EconomicsEvaluation {
    pub case_id: String,
    pub disposition: EvaluationDisposition,
    pub total_cost_low_minor: Option<u64>,
    pub total_cost_high_minor: Option<u64>,
    pub units_started: usize,
    pub first_pass_released: usize,
    pub final_released: usize,
    /// Component totals retain their evidence class; an estimate is never
    /// silently described as paid or measured cost.
    pub cost_breakdown: Vec<CostBreakdown>,
    /// Exact minor-unit numerator and unique released-output denominator.
    /// No floating-point unit-cost value is produced.
    pub cost_per_released_low: Option<(u64, usize)>,
    pub cost_per_released_high: Option<(u64, usize)>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExpectedEvaluation {
    pub case_id: String,
    pub disposition: EvaluationDisposition,
    pub total_cost_low_minor: Option<u64>,
    pub total_cost_high_minor: Option<u64>,
    pub units_started: usize,
    pub first_pass_released: usize,
    pub final_released: usize,
}

fn valid_ref(value: &str) -> bool {
    !value.is_empty()
        && value != "-"
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b":/_-.@".contains(&byte))
}

fn valid_currency(value: &str) -> bool {
    value.len() == 3 && value.bytes().all(|byte| byte.is_ascii_uppercase())
}

fn valid_date(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| index == 4 || index == 7 || byte.is_ascii_digit())
}

fn split_row<'a>(
    line: &'a str,
    expected: usize,
    source: &str,
    line_no: usize,
) -> Result<Vec<&'a str>, String> {
    let fields: Vec<&str> = line.split('\t').collect();
    if fields.len() != expected {
        return Err(format!(
            "{source}:{line_no}: expected {expected} tab-separated fields, found {}",
            fields.len()
        ));
    }
    if fields.iter().any(|field| field.trim() != *field) {
        return Err(format!(
            "{source}:{line_no}: leading/trailing whitespace is forbidden"
        ));
    }
    if fields.iter().any(|field| field.is_empty()) {
        return Err(format!("{source}:{line_no}: empty fields are forbidden"));
    }
    Ok(fields)
}

fn parse_amount(value: &str, source: &str, line_no: usize) -> Result<Option<u64>, String> {
    if value == "-" {
        return Ok(None);
    }
    value
        .parse::<u64>()
        .map(Some)
        .map_err(|_| format!("{source}:{line_no}: invalid non-negative integer amount: {value}"))
}

fn parse_expected_amount(value: &str, source: &str, line_no: usize) -> Result<Option<u64>, String> {
    parse_amount(value, source, line_no)
}

pub fn parse_cases(input: &str) -> Result<Vec<CaseManifest>, String> {
    let mut lines = input.lines();
    match lines.next() {
        Some(header) if header == CASE_HEADER => {}
        Some(_) => return Err("cases: header mismatch".into()),
        None => return Err("cases: empty input".into()),
    }

    let mut cases = Vec::new();
    let mut ids = BTreeSet::new();
    for (index, line) in lines.enumerate() {
        if line.is_empty() {
            continue;
        }
        let row = split_row(line, 9, "cases", index + 2)?;
        for (label, value) in [
            ("case_id", row[0]),
            ("route_ref", row[1]),
            ("product_profile_ref", row[3]),
            ("acceptance_profile_ref", row[4]),
            ("batch_ref", row[5]),
        ] {
            if !valid_ref(value) {
                return Err(format!("cases:{}: invalid {label}: {value}", index + 2));
            }
        }
        if !valid_currency(row[6]) || !valid_date(row[7]) || !valid_ref(row[8]) {
            return Err(format!("cases:{}: invalid currency, price date or amount unit", index + 2));
        }
        if !ids.insert(row[0].to_owned()) {
            return Err(format!("cases:{}: duplicate case_id {}", index + 2, row[0]));
        }
        cases.push(CaseManifest {
            case_id: row[0].to_owned(),
            route_ref: row[1].to_owned(),
            route_status: RouteStatus::parse(row[2])?,
            product_profile_ref: row[3].to_owned(),
            acceptance_profile_ref: row[4].to_owned(),
            batch_ref: row[5].to_owned(),
            currency: row[6].to_owned(),
            price_date: row[7].to_owned(),
            amount_unit: row[8].to_owned(),
        });
    }
    if cases.is_empty() {
        return Err("cases: no case rows".into());
    }
    Ok(cases)
}

pub fn parse_cost_items(input: &str) -> Result<Vec<CostItem>, String> {
    let mut lines = input.lines();
    match lines.next() {
        Some(header) if header == COST_HEADER => {}
        Some(_) => return Err("cost items: header mismatch".into()),
        None => return Err("cost items: empty input".into()),
    }

    let mut rows = Vec::new();
    for (index, line) in lines.enumerate() {
        if line.is_empty() {
            continue;
        }
        let row = split_row(line, 11, "cost items", index + 2)?;
        for (label, value) in [
            ("case_id", row[0]),
            ("cost_item_id", row[1]),
            ("category", row[2]),
            ("currency", row[5]),
            ("price_date", row[6]),
            ("amount_unit", row[7]),
            ("evidence_class", row[8]),
            ("scope", row[9]),
            ("source_ref", row[10]),
        ] {
            if !valid_ref(value) {
                return Err(format!("cost items:{}: invalid {label}: {value}", index + 2));
            }
        }
        if !valid_currency(row[5]) || !valid_date(row[6]) {
            return Err(format!("cost items:{}: invalid currency or price date", index + 2));
        }
        if !matches!(row[8], "SYNTHETIC" | "ESTIMATE" | "QUOTE" | "PAID" | "PUBLIC" | "MEASURED") {
            return Err(format!("cost items:{}: unsupported evidence class {}", index + 2, row[8]));
        }
        if !matches!(row[9], "IN_SCOPE" | "OUT_OF_SCOPE") {
            return Err(format!("cost items:{}: unsupported scope {}", index + 2, row[9]));
        }
        let low = parse_amount(row[3], "cost items", index + 2)?;
        let high = parse_amount(row[4], "cost items", index + 2)?;
        match (low, high) {
            (Some(low), Some(high)) if low > high => {
                return Err(format!("cost items:{}: lower amount exceeds upper amount", index + 2));
            }
            (Some(_), None) | (None, Some(_)) => {
                return Err(format!("cost items:{}: amount interval must provide both bounds or neither", index + 2));
            }
            _ => {}
        }
        rows.push(CostItem {
            case_id: row[0].to_owned(),
            cost_item_id: row[1].to_owned(),
            category: row[2].to_owned(),
            amount_minor_low: low,
            amount_minor_high: high,
            currency: row[5].to_owned(),
            price_date: row[6].to_owned(),
            amount_unit: row[7].to_owned(),
            evidence_class: row[8].to_owned(),
            scope: row[9].to_owned(),
            source_ref: row[10].to_owned(),
        });
    }
    Ok(rows)
}

pub fn parse_outputs(input: &str) -> Result<Vec<OutputRecord>, String> {
    let mut lines = input.lines();
    match lines.next() {
        Some(header) if header == OUTPUT_HEADER => {}
        Some(_) => return Err("outputs: header mismatch".into()),
        None => return Err("outputs: empty input".into()),
    }

    let mut rows = Vec::new();
    for (index, line) in lines.enumerate() {
        if line.is_empty() {
            continue;
        }
        let row = split_row(line, 4, "outputs", index + 2)?;
        if !valid_ref(row[0]) || !valid_ref(row[1]) {
            return Err(format!("outputs:{}: invalid case or output identity", index + 2));
        }
        let disposition = FinalDisposition::parse(row[2])?;
        if disposition.is_released() {
            if !valid_ref(row[3]) {
                return Err(format!("outputs:{}: released output requires a release reference", index + 2));
            }
        } else if row[3] != "-" {
            return Err(format!("outputs:{}: non-released output cannot carry a release reference", index + 2));
        }
        rows.push(OutputRecord {
            case_id: row[0].to_owned(),
            output_id: row[1].to_owned(),
            final_disposition: disposition,
            release_ref: row[3].to_owned(),
        });
    }
    Ok(rows)
}

pub fn parse_expected(input: &str) -> Result<Vec<ExpectedEvaluation>, String> {
    let mut lines = input.lines();
    match lines.next() {
        Some(header) if header == EXPECTED_HEADER => {}
        Some(_) => return Err("expected: header mismatch".into()),
        None => return Err("expected: empty input".into()),
    }

    let mut rows = Vec::new();
    for (index, line) in lines.enumerate() {
        if line.is_empty() {
            continue;
        }
        let row = split_row(line, 7, "expected", index + 2)?;
        if !valid_ref(row[0]) {
            return Err(format!("expected:{}: invalid case identity", index + 2));
        }
        rows.push(ExpectedEvaluation {
            case_id: row[0].to_owned(),
            disposition: EvaluationDisposition::parse(row[1])?,
            total_cost_low_minor: parse_expected_amount(row[2], "expected", index + 2)?,
            total_cost_high_minor: parse_expected_amount(row[3], "expected", index + 2)?,
            units_started: row[4].parse().map_err(|_| format!("expected:{}: invalid started count", index + 2))?,
            first_pass_released: row[5].parse().map_err(|_| format!("expected:{}: invalid first-pass count", index + 2))?,
            final_released: row[6].parse().map_err(|_| format!("expected:{}: invalid final count", index + 2))?,
        });
    }
    Ok(rows)
}

pub fn evaluate(
    cases: &[CaseManifest],
    costs: &[CostItem],
    outputs: &[OutputRecord],
) -> Result<Vec<EconomicsEvaluation>, String> {
    if cases.is_empty() {
        return Err("evaluator input contains no cases".into());
    }

    let mut cases_by_id: BTreeMap<&str, &CaseManifest> = BTreeMap::new();
    for case in cases {
        for (label, value) in [
            ("case_id", case.case_id.as_str()),
            ("route_ref", case.route_ref.as_str()),
            ("product_profile_ref", case.product_profile_ref.as_str()),
            ("acceptance_profile_ref", case.acceptance_profile_ref.as_str()),
            ("batch_ref", case.batch_ref.as_str()),
            ("amount_unit", case.amount_unit.as_str()),
        ] {
            if !valid_ref(value) {
                return Err(format!("case {}: invalid {label}: {value}", case.case_id));
            }
        }
        if !valid_currency(&case.currency) || !valid_date(&case.price_date) {
            return Err(format!("case {}: invalid currency or price date", case.case_id));
        }
        if cases_by_id.insert(case.case_id.as_str(), case).is_some() {
            return Err(format!("duplicate case_id in evaluator input: {}", case.case_id));
        }
    }

    let mut cost_ids = BTreeSet::new();
    for item in costs {
        if !cases_by_id.contains_key(item.case_id.as_str()) {
            return Err(format!("orphan cost item for unknown case: {}", item.case_id));
        }
        for (label, value) in [
            ("cost_item_id", item.cost_item_id.as_str()),
            ("category", item.category.as_str()),
            ("source_ref", item.source_ref.as_str()),
        ] {
            if !valid_ref(value) {
                return Err(format!("cost item {}: invalid {label}", item.cost_item_id));
            }
        }
        if !cost_ids.insert(item.cost_item_id.as_str()) {
            return Err(format!("duplicate cost_item_id: {}", item.cost_item_id));
        }
        if !valid_currency(&item.currency) || !valid_date(&item.price_date) {
            return Err(format!("cost item {}: invalid currency or price date", item.cost_item_id));
        }
        if !matches!(item.evidence_class.as_str(), "SYNTHETIC" | "ESTIMATE" | "QUOTE" | "PAID" | "PUBLIC" | "MEASURED") {
            return Err(format!("cost item {}: unsupported evidence class", item.cost_item_id));
        }
        if !matches!(item.scope.as_str(), "IN_SCOPE" | "OUT_OF_SCOPE") {
            return Err(format!("cost item {}: unsupported scope", item.cost_item_id));
        }
        match (item.amount_minor_low, item.amount_minor_high) {
            (Some(low), Some(high)) if low > high => {
                return Err(format!("cost item {}: lower amount exceeds upper amount", item.cost_item_id));
            }
            (Some(_), None) | (None, Some(_)) => {
                return Err(format!("cost item {}: amount interval has only one bound", item.cost_item_id));
            }
            _ => {}
        }
        let case = cases_by_id[item.case_id.as_str()];
        if item.currency != case.currency
            || item.price_date != case.price_date
            || item.amount_unit != case.amount_unit
        {
            return Err(format!(
                "cost item {}: currency/date/unit does not match case {}",
                item.cost_item_id, case.case_id
            ));
        }
    }

    let mut output_ids = BTreeSet::new();
    let mut release_refs = BTreeSet::new();
    for output in outputs {
        if !cases_by_id.contains_key(output.case_id.as_str()) {
            return Err(format!("orphan output for unknown case: {}", output.case_id));
        }
        if !valid_ref(&output.output_id) {
            return Err(format!("invalid output identity: {}", output.output_id));
        }
        if !output_ids.insert(output.output_id.as_str()) {
            return Err(format!("output identity reused across records: {}", output.output_id));
        }
        if output.final_disposition.is_released() {
            if !valid_ref(&output.release_ref) {
                return Err(format!("released output {} lacks a valid release reference", output.output_id));
            }
            if !release_refs.insert(output.release_ref.as_str()) {
                return Err(format!("release reference reused across outputs: {}", output.release_ref));
            }
        } else if output.release_ref != "-" {
            return Err(format!("non-released output {} carries a release reference", output.output_id));
        }
    }

    let mut results = Vec::with_capacity(cases.len());
    for case in cases {
        let case_costs: Vec<&CostItem> = costs
            .iter()
            .filter(|item| item.case_id == case.case_id && item.scope == "IN_SCOPE")
            .collect();
        let case_outputs: Vec<&OutputRecord> = outputs
            .iter()
            .filter(|output| output.case_id == case.case_id)
            .collect();

        let units_started = case_outputs.len();
        let first_pass_released = case_outputs
            .iter()
            .filter(|output| output.final_disposition == FinalDisposition::ReleasedFirstPass)
            .count();
        let final_released = case_outputs
            .iter()
            .filter(|output| output.final_disposition.is_released())
            .count();

        let mut result = EconomicsEvaluation {
            case_id: case.case_id.clone(),
            disposition: EvaluationDisposition::RouteUnknown,
            total_cost_low_minor: None,
            total_cost_high_minor: None,
            units_started,
            first_pass_released,
            final_released,
            cost_breakdown: Vec::new(),
            cost_per_released_low: None,
            cost_per_released_high: None,
        };

        match case.route_status {
            RouteStatus::Ineligible => {
                result.disposition = EvaluationDisposition::RouteIneligible;
                results.push(result);
                continue;
            }
            RouteStatus::Unknown => {
                result.disposition = EvaluationDisposition::RouteUnknown;
                results.push(result);
                continue;
            }
            RouteStatus::Eligible => {}
        }

        if case_costs.is_empty()
            || case_costs
                .iter()
                .any(|item| item.amount_minor_low.is_none() || item.amount_minor_high.is_none())
        {
            result.disposition = EvaluationDisposition::CostIncomplete;
            results.push(result);
            continue;
        }

        let mut total_low = 0_u64;
        let mut total_high = 0_u64;
        let mut breakdown: BTreeMap<(String, String), (u64, u64)> = BTreeMap::new();
        for item in &case_costs {
            let low = item
                .amount_minor_low
                .ok_or_else(|| format!("case {}: missing lower cost bound", case.case_id))?;
            let high = item
                .amount_minor_high
                .ok_or_else(|| format!("case {}: missing upper cost bound", case.case_id))?;
            total_low = total_low
                .checked_add(low)
                .ok_or_else(|| format!("case {}: lower cost total overflow", case.case_id))?;
            total_high = total_high
                .checked_add(high)
                .ok_or_else(|| format!("case {}: upper cost total overflow", case.case_id))?;
            let component = breakdown
                .entry((item.category.clone(), item.evidence_class.clone()))
                .or_insert((0, 0));
            component.0 = component
                .0
                .checked_add(low)
                .ok_or_else(|| format!("case {}: component lower total overflow", case.case_id))?;
            component.1 = component
                .1
                .checked_add(high)
                .ok_or_else(|| format!("case {}: component upper total overflow", case.case_id))?;
        }
        result.cost_breakdown = breakdown
            .into_iter()
            .map(|((category, evidence_class), (amount_low_minor, amount_high_minor))| {
                CostBreakdown {
                    category,
                    evidence_class,
                    amount_low_minor,
                    amount_high_minor,
                }
            })
            .collect();
        result.total_cost_low_minor = Some(total_low);
        result.total_cost_high_minor = Some(total_high);

        if final_released == 0 {
            result.disposition = EvaluationDisposition::ZeroReleased;
        } else {
            result.disposition = EvaluationDisposition::Computed;
            result.cost_per_released_low = Some((total_low, final_released));
            result.cost_per_released_high = Some((total_high, final_released));
        }
        results.push(result);
    }

    Ok(results)
}

pub fn verify_expected(
    actual: &[EconomicsEvaluation],
    expected: &[ExpectedEvaluation],
) -> Result<(), String> {
    if actual.len() != expected.len() {
        return Err(format!(
            "expected {} case results, evaluator returned {}",
            expected.len(),
            actual.len()
        ));
    }
    let mut seen = BTreeSet::new();
    for expected_row in expected {
        if !seen.insert(expected_row.case_id.as_str()) {
            return Err(format!("duplicate expected case_id: {}", expected_row.case_id));
        }
        let actual_row = actual
            .iter()
            .find(|row| row.case_id == expected_row.case_id)
            .ok_or_else(|| format!("missing actual result for {}", expected_row.case_id))?;
        if actual_row.disposition != expected_row.disposition
            || actual_row.total_cost_low_minor != expected_row.total_cost_low_minor
            || actual_row.total_cost_high_minor != expected_row.total_cost_high_minor
            || actual_row.units_started != expected_row.units_started
            || actual_row.first_pass_released != expected_row.first_pass_released
            || actual_row.final_released != expected_row.final_released
        {
            return Err(format!(
                "known-answer mismatch for {}: expected {:?}, got {:?}",
                expected_row.case_id, expected_row, actual_row
            ));
        }
    }
    Ok(())
}

pub fn evaluate_corpus(
    cases_tsv: &str,
    costs_tsv: &str,
    outputs_tsv: &str,
    expected_tsv: &str,
) -> Result<Vec<EconomicsEvaluation>, String> {
    let cases = parse_cases(cases_tsv)?;
    let costs = parse_cost_items(costs_tsv)?;
    let outputs = parse_outputs(outputs_tsv)?;
    let expected = parse_expected(expected_tsv)?;
    let results = evaluate(&cases, &costs, &outputs)?;
    verify_expected(&results, &expected)?;
    Ok(results)
}

pub fn render_cost_breakdown(results: &[EconomicsEvaluation]) -> String {
    let mut report = String::from(
        "case_id\tcategory\tevidence_class\tamount_low_minor\tamount_high_minor\n",
    );
    for row in results {
        for item in &row.cost_breakdown {
            report.push_str(&format!(
                "{}\t{}\t{}\t{}\t{}\n",
                row.case_id,
                item.category,
                item.evidence_class,
                item.amount_low_minor,
                item.amount_high_minor
            ));
        }
    }
    report
}

pub fn render_report(results: &[EconomicsEvaluation]) -> String {
    let mut report = String::from(
        "case_id\tdisposition\ttotal_low_minor\ttotal_high_minor\tunits_started\tfirst_pass_released\tfinal_released\tcost_per_released_low_num\tcost_per_released_low_den\tcost_per_released_high_num\tcost_per_released_high_den\n",
    );
    for row in results {
        let low = row.total_cost_low_minor.map_or_else(|| "-".into(), |value| value.to_string());
        let high = row.total_cost_high_minor.map_or_else(|| "-".into(), |value| value.to_string());
        let low_num = row.cost_per_released_low.map_or_else(|| "-".into(), |value| value.0.to_string());
        let low_den = row.cost_per_released_low.map_or_else(|| "-".into(), |value| value.1.to_string());
        let high_num = row.cost_per_released_high.map_or_else(|| "-".into(), |value| value.0.to_string());
        let high_den = row.cost_per_released_high.map_or_else(|| "-".into(), |value| value.1.to_string());
        report.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            row.case_id,
            row.disposition.as_str(),
            low,
            high,
            row.units_started,
            row.first_pass_released,
            row.final_released,
            low_num,
            low_den,
            high_num,
            high_den
        ));
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed() -> (Vec<CaseManifest>, Vec<CostItem>, Vec<OutputRecord>) {
        (
            parse_cases(CASES).unwrap(),
            parse_cost_items(COST_ITEMS).unwrap(),
            parse_outputs(OUTPUTS).unwrap(),
        )
    }

    #[test]
    fn frozen_twenty_case_corpus_matches_hand_calculated_oracle() {
        let results = evaluate_corpus(CASES, COST_ITEMS, OUTPUTS, EXPECTED).unwrap();
        assert_eq!(results.len(), 20);
        assert_eq!(
            results
                .iter()
                .filter(|row| row.disposition == EvaluationDisposition::Computed)
                .count(),
            15
        );
        assert_eq!(
            results
                .iter()
                .filter(|row| row.disposition == EvaluationDisposition::ZeroReleased)
                .count(),
            2
        );
    }

    #[test]
    fn rework_changes_disposition_not_unique_output_count() {
        let (cases, costs, outputs) = parsed();
        let row = evaluate(&cases, &costs, &outputs)
            .unwrap()
            .into_iter()
            .find(|row| row.case_id == "rework_one")
            .unwrap();
        assert_eq!(row.units_started, 3);
        assert_eq!(row.first_pass_released, 2);
        assert_eq!(row.final_released, 3);
        assert_eq!(row.cost_per_released_low, Some((140_000, 3)));
    }

    #[test]
    fn zero_released_has_no_finite_unit_cost() {
        let (cases, costs, outputs) = parsed();
        let row = evaluate(&cases, &costs, &outputs)
            .unwrap()
            .into_iter()
            .find(|row| row.case_id == "zero_released")
            .unwrap();
        assert_eq!(row.disposition, EvaluationDisposition::ZeroReleased);
        assert_eq!(row.total_cost_low_minor, Some(55_000));
        assert_eq!(row.cost_per_released_low, None);
    }

    #[test]
    fn unknown_in_scope_cost_is_not_imputed_as_zero() {
        let (cases, costs, outputs) = parsed();
        let row = evaluate(&cases, &costs, &outputs)
            .unwrap()
            .into_iter()
            .find(|row| row.case_id == "unknown_cost")
            .unwrap();
        assert_eq!(row.disposition, EvaluationDisposition::CostIncomplete);
        assert_eq!(row.total_cost_low_minor, None);
        assert_eq!(row.cost_per_released_low, None);
    }

    #[test]
    fn route_ineligibility_precedes_economic_comparison() {
        let (cases, costs, outputs) = parsed();
        let row = evaluate(&cases, &costs, &outputs)
            .unwrap()
            .into_iter()
            .find(|row| row.case_id == "route_ineligible")
            .unwrap();
        assert_eq!(row.disposition, EvaluationDisposition::RouteIneligible);
        assert_eq!(row.total_cost_low_minor, None);
    }

    #[test]
    fn unknown_route_cannot_be_promoted_by_low_cost() {
        let (cases, costs, outputs) = parsed();
        let row = evaluate(&cases, &costs, &outputs)
            .unwrap()
            .into_iter()
            .find(|row| row.case_id == "route_unknown")
            .unwrap();
        assert_eq!(row.disposition, EvaluationDisposition::RouteUnknown);
        assert_eq!(row.total_cost_low_minor, None);
    }

    #[test]
    fn explicitly_out_of_scope_unknown_cost_is_excluded_not_a_blocker() {
        let (cases, costs, outputs) = parsed();
        let row = evaluate(&cases, &costs, &outputs)
            .unwrap()
            .into_iter()
            .find(|row| row.case_id == "out_of_scope_excluded")
            .unwrap();
        assert_eq!(row.disposition, EvaluationDisposition::Computed);
        assert_eq!(row.total_cost_low_minor, Some(35_000));
    }

    #[test]
    fn bounded_cost_intervals_propagate_without_rounding() {
        let (cases, costs, outputs) = parsed();
        let row = evaluate(&cases, &costs, &outputs)
            .unwrap()
            .into_iter()
            .find(|row| row.case_id == "bounded_cost")
            .unwrap();
        assert_eq!(row.total_cost_low_minor, Some(12_000));
        assert_eq!(row.total_cost_high_minor, Some(15_000));
        assert_eq!(row.cost_per_released_low, Some((12_000, 2)));
        assert_eq!(row.cost_per_released_high, Some((15_000, 2)));
    }

    #[test]
    fn component_breakdown_preserves_category_and_evidence_class() {
        let (cases, mut costs, outputs) = parsed();
        let row = costs
            .iter_mut()
            .find(|item| item.case_id == "baseline_three_first_pass" && item.category == "ENERGY")
            .unwrap();
        row.evidence_class = "ESTIMATE".into();
        let result = evaluate(&cases, &costs, &outputs).unwrap();
        let evaluated = result
            .iter()
            .find(|row| row.case_id == "baseline_three_first_pass")
            .unwrap();
        assert!(evaluated.cost_breakdown.iter().any(|item| {
            item.category == "ENERGY" && item.evidence_class == "ESTIMATE"
        }));
        assert!(evaluated.cost_breakdown.iter().any(|item| {
            item.category == "MATERIAL" && item.evidence_class == "SYNTHETIC"
        }));
    }

    #[test]
    fn zero_cost_is_distinct_from_missing_cost() {
        let (cases, costs, outputs) = parsed();
        let row = evaluate(&cases, &costs, &outputs)
            .unwrap()
            .into_iter()
            .find(|row| row.case_id == "zero_cost")
            .unwrap();
        assert_eq!(row.disposition, EvaluationDisposition::Computed);
        assert_eq!(row.total_cost_low_minor, Some(0));
        assert_eq!(row.cost_per_released_low, Some((0, 1)));
    }

    #[test]
    fn duplicate_output_identity_is_rejected_across_cases() {
        let (cases, _costs, mut outputs) = parsed();
        outputs[0].output_id = outputs[1].output_id.clone();
        let costs = parse_cost_items(COST_ITEMS).unwrap();
        assert!(evaluate(&cases, &costs, &outputs)
            .unwrap_err()
            .contains("output identity reused"));
    }

    #[test]
    fn duplicate_cost_identity_is_rejected() {
        let (cases, mut costs, outputs) = parsed();
        costs[1].cost_item_id = costs[0].cost_item_id.clone();
        assert!(evaluate(&cases, &costs, &outputs)
            .unwrap_err()
            .contains("duplicate cost_item_id"));
    }

    #[test]
    fn mixed_currency_is_rejected_not_converted_implicitly() {
        let (cases, mut costs, outputs) = parsed();
        costs[0].currency = "USD".into();
        assert!(evaluate(&cases, &costs, &outputs)
            .unwrap_err()
            .contains("currency/date/unit"));
    }

    #[test]
    fn mixed_price_date_is_rejected() {
        let (cases, mut costs, outputs) = parsed();
        costs[0].price_date = "2026-10-01".into();
        assert!(evaluate(&cases, &costs, &outputs)
            .unwrap_err()
            .contains("currency/date/unit"));
    }

    #[test]
    fn mixed_amount_unit_is_rejected() {
        let (cases, mut costs, outputs) = parsed();
        costs[0].amount_unit = "USD_CENT".into();
        assert!(evaluate(&cases, &costs, &outputs)
            .unwrap_err()
            .contains("currency/date/unit"));
    }

    #[test]
    fn released_output_requires_release_reference() {
        let (cases, costs, mut outputs) = parsed();
        outputs[0].release_ref = "-".into();
        assert!(evaluate(&cases, &costs, &outputs)
            .unwrap_err()
            .contains("lacks a valid release reference"));
    }

    #[test]
    fn duplicate_release_reference_is_rejected_across_outputs() {
        let (cases, costs, mut outputs) = parsed();
        outputs[1].release_ref = outputs[0].release_ref.clone();
        assert!(evaluate(&cases, &costs, &outputs)
            .unwrap_err()
            .contains("release reference reused across outputs"));
    }

    #[test]
    fn unreleased_output_cannot_carry_release_reference() {
        let (cases, costs, mut outputs) = parsed();
        outputs.iter_mut().find(|row| row.final_disposition == FinalDisposition::Scrap).unwrap().release_ref =
            "release:invalid-scrap-release".into();
        assert!(evaluate(&cases, &costs, &outputs)
            .unwrap_err()
            .contains("non-released output"));
    }

    #[test]
    fn missing_cost_interval_bound_is_rejected() {
        let (cases, mut costs, outputs) = parsed();
        costs[0].amount_minor_high = None;
        assert!(evaluate(&cases, &costs, &outputs)
            .unwrap_err()
            .contains("amount interval has only one bound"));
    }

    #[test]
    fn reversed_cost_interval_is_rejected() {
        let (cases, mut costs, outputs) = parsed();
        costs[0].amount_minor_low = Some(20);
        costs[0].amount_minor_high = Some(10);
        assert!(evaluate(&cases, &costs, &outputs)
            .unwrap_err()
            .contains("lower amount exceeds upper amount"));
    }

    #[test]
    fn integer_overflow_is_rejected() {
        let (cases, mut costs, outputs) = parsed();
        costs.retain(|item| item.case_id != "baseline_three_first_pass");
        costs.push(CostItem {
            case_id: "baseline_three_first_pass".into(),
            cost_item_id: "cost:overflow:a".into(),
            category: "MATERIAL".into(),
            amount_minor_low: Some(u64::MAX),
            amount_minor_high: Some(u64::MAX),
            currency: "ZAR".into(),
            price_date: "2026-11-01".into(),
            amount_unit: "ZAR_CENT".into(),
            evidence_class: "SYNTHETIC".into(),
            scope: "IN_SCOPE".into(),
            source_ref: "source:overflow:a".into(),
        });
        costs.push(CostItem {
            case_id: "baseline_three_first_pass".into(),
            cost_item_id: "cost:overflow:b".into(),
            category: "LABOUR".into(),
            amount_minor_low: Some(1),
            amount_minor_high: Some(1),
            currency: "ZAR".into(),
            price_date: "2026-11-01".into(),
            amount_unit: "ZAR_CENT".into(),
            evidence_class: "SYNTHETIC".into(),
            scope: "IN_SCOPE".into(),
            source_ref: "source:overflow:b".into(),
        });
        assert!(evaluate(&cases, &costs, &outputs)
            .unwrap_err()
            .contains("cost total overflow"));
    }

    #[test]
    fn output_order_does_not_change_result() {
        let (cases, costs, mut outputs) = parsed();
        let forward = evaluate(&cases, &costs, &outputs).unwrap();
        outputs.reverse();
        let reverse = evaluate(&cases, &costs, &outputs).unwrap();
        assert_eq!(forward, reverse);
    }

    #[test]
    fn negative_cost_amount_is_rejected_by_parser() {
        let corrupted = COST_ITEMS.replacen("100000", "-1", 1);
        assert!(parse_cost_items(&corrupted)
            .unwrap_err()
            .contains("invalid non-negative integer amount"));
    }

    #[test]
    fn eligible_case_without_in_scope_cost_items_is_incomplete() {
        let (cases, mut costs, outputs) = parsed();
        costs.retain(|item| item.case_id != "baseline_three_first_pass");
        let row = evaluate(&cases, &costs, &outputs)
            .unwrap()
            .into_iter()
            .find(|row| row.case_id == "baseline_three_first_pass")
            .unwrap();
        assert_eq!(row.disposition, EvaluationDisposition::CostIncomplete);
        assert_eq!(row.total_cost_low_minor, None);
    }

    #[test]
    fn component_breakdown_reconciles_to_total_cost_interval() {
        let (cases, costs, outputs) = parsed();
        let row = evaluate(&cases, &costs, &outputs)
            .unwrap()
            .into_iter()
            .find(|row| row.case_id == "baseline_three_first_pass")
            .unwrap();
        let low = row
            .cost_breakdown
            .iter()
            .map(|item| item.amount_low_minor)
            .sum::<u64>();
        let high = row
            .cost_breakdown
            .iter()
            .map(|item| item.amount_high_minor)
            .sum::<u64>();
        assert_eq!(Some(low), row.total_cost_low_minor);
        assert_eq!(Some(high), row.total_cost_high_minor);
    }

    #[test]
    fn every_case_has_exactly_one_expected_row() {
        let expected = parse_expected(EXPECTED).unwrap();
        let mut ids: BTreeSet<&str> = BTreeSet::new();
        assert!(expected.iter().all(|row| ids.insert(row.case_id.as_str())));
        assert_eq!(expected.len(), parse_cases(CASES).unwrap().len());
    }
}
