//! Stage 0 cross-owner receipt conformance for material-accounted factory routes.
//!
//! This crate deliberately does NOT calculate mass balance, thermodynamics, process
//! physics, quality conformity, or production scheduling. It verifies exact bindings
//! and conservatively composes status receipts emitted by their canonical owners.
//!
//! A well-formed source digest is only shape-checked here; the tool does not retrieve
//! source bytes, authenticate issuers, or verify digest contents.

use std::collections::{BTreeMap, BTreeSet};

pub const CASES: &str = include_str!("../fixtures/cases-v0.1.tsv");
pub const RECEIPTS: &str = include_str!("../fixtures/receipts-v0.1.tsv");

const CASE_HEADER: &str = "case_id\tmanifest_ref\troute_ref\tprocess_ref\tinput_lot_set_ref\tmaterial_state_ref\tfacility_profile_ref\tacceptance_profile_ref\toutput_ref\texpected_disposition";
const RECEIPT_HEADER: &str = "case_id\tmanifest_ref\tgate\treceipt_ref\tsubject_ref\tscope_ref\tsource_digest\tstatus";

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Gate {
    Route,
    MassBalance,
    Utility,
    MaterialGrade,
    CircularAllocation,
    Inspection,
    Provenance,
    ClaimCeiling,
}

impl Gate {
    pub const ALL: [Gate; 8] = [
        Gate::Route,
        Gate::MassBalance,
        Gate::Utility,
        Gate::MaterialGrade,
        Gate::CircularAllocation,
        Gate::Inspection,
        Gate::Provenance,
        Gate::ClaimCeiling,
    ];

    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "ROUTE" => Ok(Self::Route),
            "MASS_BALANCE" => Ok(Self::MassBalance),
            "UTILITY" => Ok(Self::Utility),
            "MATERIAL_GRADE" => Ok(Self::MaterialGrade),
            "CIRCULAR_ALLOCATION" => Ok(Self::CircularAllocation),
            "INSPECTION" => Ok(Self::Inspection),
            "PROVENANCE" => Ok(Self::Provenance),
            "CLAIM_CEILING" => Ok(Self::ClaimCeiling),
            _ => Err(format!("unknown gate: {value}")),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Route => "ROUTE",
            Self::MassBalance => "MASS_BALANCE",
            Self::Utility => "UTILITY",
            Self::MaterialGrade => "MATERIAL_GRADE",
            Self::CircularAllocation => "CIRCULAR_ALLOCATION",
            Self::Inspection => "INSPECTION",
            Self::Provenance => "PROVENANCE",
            Self::ClaimCeiling => "CLAIM_CEILING",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Disposition {
    AcceptedSynthetic,
    Blocked,
    Indeterminate,
}

impl Disposition {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AcceptedSynthetic => "ACCEPTED_SYNTHETIC",
            Self::Blocked => "BLOCKED",
            Self::Indeterminate => "INDETERMINATE",
        }
    }

    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "ACCEPTED_SYNTHETIC" => Ok(Self::AcceptedSynthetic),
            "BLOCKED" => Ok(Self::Blocked),
            "INDETERMINATE" => Ok(Self::Indeterminate),
            _ => Err(format!("unknown expected disposition: {value}")),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CaseManifest {
    pub case_id: String,
    pub manifest_ref: String,
    pub route_ref: String,
    pub process_ref: String,
    pub input_lot_set_ref: String,
    pub material_state_ref: String,
    pub facility_profile_ref: String,
    pub acceptance_profile_ref: String,
    pub output_ref: String,
    pub expected: Disposition,
}

impl CaseManifest {
    fn expected_binding(&self, gate: Gate) -> (&str, &str) {
        match gate {
            Gate::Route => (&self.route_ref, &self.process_ref),
            Gate::MassBalance => (&self.process_ref, &self.input_lot_set_ref),
            Gate::Utility => (&self.process_ref, &self.facility_profile_ref),
            Gate::MaterialGrade => (&self.material_state_ref, &self.input_lot_set_ref),
            Gate::CircularAllocation => (&self.process_ref, &self.input_lot_set_ref),
            Gate::Inspection => (&self.output_ref, &self.acceptance_profile_ref),
            Gate::Provenance => (&self.output_ref, &self.process_ref),
            Gate::ClaimCeiling => (&self.output_ref, &self.manifest_ref),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Receipt {
    pub case_id: String,
    pub manifest_ref: String,
    pub gate: Gate,
    pub receipt_ref: String,
    pub subject_ref: String,
    pub scope_ref: String,
    pub source_digest: String,
    pub status: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CaseEvaluation {
    pub case_id: String,
    pub expected: Disposition,
    pub actual: Disposition,
    pub expected_matches: bool,
    pub blocked_reasons: Vec<String>,
    pub indeterminate_reasons: Vec<String>,
    pub digest_check: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum GateResult {
    Pass,
    Blocked,
    Indeterminate,
}

fn valid_ref(value: &str) -> bool {
    !value.is_empty()
        && value != "-"
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b":/_-.@".contains(&byte))
}

fn valid_sha256_shape(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn split_row<'a>(line: &'a str, expected: usize, source: &str, line_no: usize) -> Result<Vec<&'a str>, String> {
    let fields: Vec<&str> = line.split('\t').collect();
    if fields.len() != expected {
        return Err(format!(
            "{source}:{line_no}: expected {expected} tab-separated fields, found {}",
            fields.len()
        ));
    }
    if fields.iter().any(|field| field.trim() != *field) {
        return Err(format!("{source}:{line_no}: leading/trailing whitespace is forbidden"));
    }
    Ok(fields)
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
        let row = split_row(line, 10, "cases", index + 2)?;
        if row.iter().any(|field| field.is_empty()) {
            return Err(format!("cases:{}: empty field", index + 2));
        }
        for (label, value) in [
            ("case_id", row[0]),
            ("manifest_ref", row[1]),
            ("route_ref", row[2]),
            ("process_ref", row[3]),
            ("input_lot_set_ref", row[4]),
            ("material_state_ref", row[5]),
            ("facility_profile_ref", row[6]),
            ("acceptance_profile_ref", row[7]),
            ("output_ref", row[8]),
        ] {
            if !valid_ref(value) {
                return Err(format!("cases:{}: invalid {label}: {value}", index + 2));
            }
        }
        if !ids.insert(row[0].to_owned()) {
            return Err(format!("cases:{}: duplicate case_id {}", index + 2, row[0]));
        }
        cases.push(CaseManifest {
            case_id: row[0].to_owned(),
            manifest_ref: row[1].to_owned(),
            route_ref: row[2].to_owned(),
            process_ref: row[3].to_owned(),
            input_lot_set_ref: row[4].to_owned(),
            material_state_ref: row[5].to_owned(),
            facility_profile_ref: row[6].to_owned(),
            acceptance_profile_ref: row[7].to_owned(),
            output_ref: row[8].to_owned(),
            expected: Disposition::parse(row[9])?,
        });
    }
    if cases.is_empty() {
        return Err("cases: no case rows".into());
    }
    Ok(cases)
}

pub fn parse_receipts(input: &str) -> Result<Vec<Receipt>, String> {
    let mut lines = input.lines();
    match lines.next() {
        Some(header) if header == RECEIPT_HEADER => {}
        Some(_) => return Err("receipts: header mismatch".into()),
        None => return Err("receipts: empty input".into()),
    }

    let mut receipts = Vec::new();
    for (index, line) in lines.enumerate() {
        if line.is_empty() {
            continue;
        }
        let row = split_row(line, 8, "receipts", index + 2)?;
        if row.iter().any(|field| field.is_empty()) {
            return Err(format!("receipts:{}: empty field", index + 2));
        }
        if !valid_ref(row[0]) || !valid_ref(row[1]) || !valid_ref(row[3]) {
            return Err(format!("receipts:{}: invalid case, manifest or receipt reference", index + 2));
        }
        let gate = Gate::parse(row[2])?;
        if row[4] != "-" && !valid_ref(row[4]) {
            return Err(format!("receipts:{}: invalid subject_ref", index + 2));
        }
        if row[5] != "-" && !valid_ref(row[5]) {
            return Err(format!("receipts:{}: invalid scope_ref", index + 2));
        }
        receipts.push(Receipt {
            case_id: row[0].to_owned(),
            manifest_ref: row[1].to_owned(),
            gate,
            receipt_ref: row[3].to_owned(),
            subject_ref: row[4].to_owned(),
            scope_ref: row[5].to_owned(),
            source_digest: row[6].to_owned(),
            status: row[7].to_owned(),
        });
    }
    Ok(receipts)
}

fn gate_result(gate: Gate, status: &str) -> GateResult {
    use GateResult::{Blocked, Indeterminate, Pass};
    match gate {
        Gate::Route => match status {
            "FEASIBLE" => Pass,
            "BLOCKED" => Blocked,
            "UNKNOWN" | "MISSING" => Indeterminate,
            _ => Blocked,
        },
        Gate::MassBalance => match status {
            "EXACT_BALANCED" => Pass,
            "POSSIBLE_WITH_UNCERTAINTY" | "UNKNOWN" | "MISSING" => Indeterminate,
            "EXACT_UNBALANCED" | "IMPOSSIBLE_WITHIN_BOUNDS" | "INVALID" => Blocked,
            _ => Blocked,
        },
        Gate::Utility => match status {
            "GUARANTEED" => Pass,
            "POSSIBLE" | "UNKNOWN" | "MISSING" => Indeterminate,
            "IMPOSSIBLE" | "INVALID" => Blocked,
            _ => Blocked,
        },
        Gate::MaterialGrade => match status {
            "ADMITTED" => Pass,
            "UNRESOLVED" | "UNKNOWN" | "MISSING" => Indeterminate,
            "REJECTED" => Blocked,
            _ => Blocked,
        },
        Gate::CircularAllocation => match status {
            "UNIQUE_ALLOCATION" | "NO_RECOVERY_IN_SCOPE" => Pass,
            "UNKNOWN" | "MISSING" => Indeterminate,
            "DOUBLE_ALLOCATED" | "INVALID" => Blocked,
            _ => Blocked,
        },
        Gate::Inspection => match status {
            "ACCEPTED" => Pass,
            "INDETERMINATE" | "UNKNOWN" | "MISSING" => Indeterminate,
            "REJECTED" => Blocked,
            _ => Blocked,
        },
        Gate::Provenance => match status {
            "COMPLETE" => Pass,
            "UNKNOWN" | "MISSING" => Indeterminate,
            "INCOMPLETE" => Blocked,
            _ => Blocked,
        },
        Gate::ClaimCeiling => match status {
            "SYNTHETIC_ONLY" => Pass,
            "UNKNOWN" | "MISSING" => Indeterminate,
            "PHYSICAL_CLAIM" => Blocked,
            _ => Blocked,
        },
    }
}

fn evaluate_case(case: &CaseManifest, receipts: &[Receipt]) -> CaseEvaluation {
    let mut blocked = Vec::new();
    let mut indeterminate = Vec::new();
    let mut by_gate: BTreeMap<Gate, Vec<&Receipt>> = BTreeMap::new();
    let mut receipt_refs = BTreeSet::new();
    let mut all_digests_well_formed = true;
    let mut missing_digest = false;

    for receipt in receipts {
        by_gate.entry(receipt.gate).or_default().push(receipt);
        if !receipt_refs.insert(receipt.receipt_ref.as_str()) {
            blocked.push(format!("receipt reference reused: {}", receipt.receipt_ref));
        }
    }

    for gate in Gate::ALL {
        let entries = by_gate.get(&gate).map(Vec::as_slice).unwrap_or(&[]);
        if entries.is_empty() {
            indeterminate.push(format!("{} receipt missing", gate.as_str()));
            continue;
        }
        if entries.len() != 1 {
            blocked.push(format!("{} has {} receipts; exactly one is required", gate.as_str(), entries.len()));
            continue;
        }

        let receipt = entries[0];
        if receipt.manifest_ref != case.manifest_ref {
            blocked.push(format!("{} manifest binding mismatch", gate.as_str()));
        }
        let (expected_subject, expected_scope) = case.expected_binding(gate);
        if receipt.subject_ref != expected_subject {
            blocked.push(format!("{} exact subject binding mismatch", gate.as_str()));
        }
        if receipt.scope_ref != expected_scope {
            blocked.push(format!("{} exact scope binding mismatch", gate.as_str()));
        }

        if receipt.source_digest == "-" {
            missing_digest = true;
            indeterminate.push(format!("{} source digest missing", gate.as_str()));
        } else if !valid_sha256_shape(&receipt.source_digest) {
            all_digests_well_formed = false;
            blocked.push(format!("{} source digest malformed", gate.as_str()));
        }

        match gate_result(gate, &receipt.status) {
            GateResult::Pass => {}
            GateResult::Blocked => blocked.push(format!("{} status {}", gate.as_str(), receipt.status)),
            GateResult::Indeterminate => indeterminate.push(format!("{} status {}", gate.as_str(), receipt.status)),
        }
    }

    let actual = if !blocked.is_empty() {
        Disposition::Blocked
    } else if !indeterminate.is_empty() {
        Disposition::Indeterminate
    } else {
        Disposition::AcceptedSynthetic
    };

    CaseEvaluation {
        case_id: case.case_id.clone(),
        expected: case.expected,
        actual,
        expected_matches: actual == case.expected,
        blocked_reasons: blocked,
        indeterminate_reasons: indeterminate,
        digest_check: if !all_digests_well_formed {
            "MALFORMED"
        } else if missing_digest {
            "INCOMPLETE"
        } else {
            "FORMAT_ONLY_NOT_AUTHENTICATED"
        },
    }
}

pub fn evaluate(cases: &[CaseManifest], receipts: &[Receipt]) -> Result<Vec<CaseEvaluation>, String> {
    let cases_by_id: BTreeMap<&str, &CaseManifest> =
        cases.iter().map(|case| (case.case_id.as_str(), case)).collect();
    for receipt in receipts {
        if !cases_by_id.contains_key(receipt.case_id.as_str()) {
            return Err(format!("orphan receipt for unknown case: {}", receipt.case_id));
        }
    }

    let mut grouped: BTreeMap<&str, Vec<&Receipt>> = BTreeMap::new();
    for receipt in receipts {
        grouped.entry(receipt.case_id.as_str()).or_default().push(receipt);
    }

    Ok(cases
        .iter()
        .map(|case| {
            let rows: Vec<Receipt> = grouped
                .get(case.case_id.as_str())
                .into_iter()
                .flat_map(|rows| rows.iter().map(|row| (*row).clone()))
                .collect();
            evaluate_case(case, &rows)
        })
        .collect())
}

pub fn evaluate_corpus(cases_tsv: &str, receipts_tsv: &str) -> Result<Vec<CaseEvaluation>, String> {
    let cases = parse_cases(cases_tsv)?;
    let receipts = parse_receipts(receipts_tsv)?;
    evaluate(&cases, &receipts)
}

pub fn render_report(evaluations: &[CaseEvaluation]) -> String {
    let mut output = String::from(
        "case_id\texpected\tactual\texpectation_match\tdigest_check\tblocked_reasons\tindeterminate_reasons\n",
    );
    for row in evaluations {
        output.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            row.case_id,
            row.expected.as_str(),
            row.actual.as_str(),
            row.expected_matches,
            row.digest_check,
            if row.blocked_reasons.is_empty() { "-" } else { &row.blocked_reasons.join(";") },
            if row.indeterminate_reasons.is_empty() { "-" } else { &row.indeterminate_reasons.join(";") },
        ));
    }
    output
}

impl Disposition {
    fn as_str(self) -> &'static str {
        Disposition::as_str(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frozen_corpus_matches_all_expected_dispositions() {
        let report = evaluate_corpus(CASES, RECEIPTS).expect("frozen corpus parses");
        assert!(report.len() >= 20);
        assert!(report.iter().all(|case| case.expected_matches), "{report:#?}");
        assert!(report.iter().any(|case| case.actual == Disposition::AcceptedSynthetic));
        assert!(report.iter().any(|case| case.actual == Disposition::Blocked));
        assert!(report.iter().any(|case| case.actual == Disposition::Indeterminate));
    }

    #[test]
    fn missing_gate_receipt_is_indeterminate_not_accepted() {
        let cases = parse_cases(CASES).unwrap();
        let mut receipts = parse_receipts(RECEIPTS).unwrap();
        receipts.retain(|row| !(row.case_id == "baseline_exact_closed" && row.gate == Gate::MassBalance));
        let case = cases.iter().find(|case| case.case_id == "baseline_exact_closed").unwrap();
        let own: Vec<_> = receipts.iter().filter(|row| row.case_id == case.case_id).cloned().collect();
        let result = evaluate_case(case, &own);
        assert_eq!(result.actual, Disposition::Indeterminate);
        assert!(!result.expected_matches);
    }

    #[test]
    fn receipt_subject_and_scope_are_exact_not_fuzzy() {
        let cases = parse_cases(CASES).unwrap();
        let mut receipts = parse_receipts(RECEIPTS).unwrap();
        let row = receipts.iter_mut().find(|row| row.case_id == "baseline_exact_closed" && row.gate == Gate::MassBalance).unwrap();
        row.subject_ref = "process:similar-looking-name".into();
        let case = cases.iter().find(|case| case.case_id == "baseline_exact_closed").unwrap();
        let own: Vec<_> = receipts.iter().filter(|row| row.case_id == case.case_id).cloned().collect();
        assert_eq!(evaluate_case(case, &own).actual, Disposition::Blocked);
    }

    #[test]
    fn duplicate_receipt_reference_is_blocked() {
        let cases = parse_cases(CASES).unwrap();
        let mut receipts = parse_receipts(RECEIPTS).unwrap();
        let index_a = receipts.iter().position(|row| row.case_id == "baseline_exact_closed" && row.gate == Gate::Inspection).unwrap();
        let index_b = receipts.iter().position(|row| row.case_id == "baseline_exact_closed" && row.gate == Gate::Provenance).unwrap();
        receipts[index_b].receipt_ref = receipts[index_a].receipt_ref.clone();
        let case = cases.iter().find(|case| case.case_id == "baseline_exact_closed").unwrap();
        let own: Vec<_> = receipts.iter().filter(|row| row.case_id == case.case_id).cloned().collect();
        assert_eq!(evaluate_case(case, &own).actual, Disposition::Blocked);
    }

    #[test]
    fn malformed_digest_is_blocked_but_missing_digest_is_indeterminate() {
        let cases = parse_cases(CASES).unwrap();
        let mut receipts = parse_receipts(RECEIPTS).unwrap();
        let case = cases.iter().find(|case| case.case_id == "baseline_exact_closed").unwrap();

        let row = receipts.iter_mut().find(|row| row.case_id == case.case_id && row.gate == Gate::Inspection).unwrap();
        row.source_digest = "not-a-digest".into();
        let own: Vec<_> = receipts.iter().filter(|row| row.case_id == case.case_id).cloned().collect();
        assert_eq!(evaluate_case(case, &own).actual, Disposition::Blocked);

        let row = receipts.iter_mut().find(|row| row.case_id == case.case_id && row.gate == Gate::Inspection).unwrap();
        row.source_digest = "-".into();
        let own: Vec<_> = receipts.iter().filter(|row| row.case_id == case.case_id).cloned().collect();
        assert_eq!(evaluate_case(case, &own).actual, Disposition::Indeterminate);
    }

    #[test]
    fn orphan_receipt_is_rejected() {
        let cases = parse_cases(CASES).unwrap();
        let mut receipts = parse_receipts(RECEIPTS).unwrap();
        receipts[0].case_id = "case:not-in-manifest".into();
        assert!(evaluate(&cases, &receipts).is_err());
    }

    #[test]
    fn unknown_gate_status_cannot_pass() {
        let cases = parse_cases(CASES).unwrap();
        let mut receipts = parse_receipts(RECEIPTS).unwrap();
        let row = receipts.iter_mut().find(|row| row.case_id == "baseline_exact_closed" && row.gate == Gate::Utility).unwrap();
        row.status = "GREEN_ENOUGH".into();
        let case = cases.iter().find(|case| case.case_id == "baseline_exact_closed").unwrap();
        let own: Vec<_> = receipts.iter().filter(|row| row.case_id == case.case_id).cloned().collect();
        assert_eq!(evaluate_case(case, &own).actual, Disposition::Blocked);
    }
}
