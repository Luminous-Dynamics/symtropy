//! Deterministic part-level aggregation of scalar manufacturing conformity receipts.
//!
//! This consumes a separately qualified scalar conformity report. It does not
//! re-run metrology, authenticate signatures, or authorize product release.
//!
//! Compile:
//!   rustc --edition=2024 -D warnings --test tools/manufacturing-conformity/part_aggregate.rs -o /tmp/part-aggregate-tests
//!   /tmp/part-aggregate-tests
//!   rustc --edition=2024 -D warnings tools/manufacturing-conformity/part_aggregate.rs -o /tmp/part-aggregate
//!   /tmp/part-aggregate --verify
//!   /tmp/part-aggregate --csv

use std::collections::{BTreeMap, BTreeSet};

const HASH_LEN: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ReceiptState {
    Verified,
    Missing,
    Stale,
    Mismatched,
    Revoked,
    NotQualified,
}

impl ReceiptState {
    fn code(self) -> &'static str {
        match self {
            Self::Verified => "VERIFIED",
            Self::Missing => "MISSING",
            Self::Stale => "STALE",
            Self::Mismatched => "MISMATCHED",
            Self::Revoked => "REVOKED",
            Self::NotQualified => "NOT_QUALIFIED",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ScalarOutcome {
    Accept,
    RejectNonconforming,
    Indeterminate,
    EvidenceIncomplete,
}

impl ScalarOutcome {
    fn code(self) -> &'static str {
        match self {
            Self::Accept => "ACCEPT",
            Self::RejectNonconforming => "REJECT_NONCONFORMING",
            Self::Indeterminate => "INDETERMINATE",
            Self::EvidenceIncomplete => "EVIDENCE_INCOMPLETE",
        }
    }
}

#[derive(Clone, Debug)]
struct Requirement {
    requirement_id: String,
    characteristic_id: String,
    measurand_id: String,
    unit: String,
    specification_id: String,
    specification_revision: String,
    mandatory: bool,
}

#[derive(Clone, Debug)]
struct Manifest {
    manifest_id: String,
    manifest_revision: String,
    manifest_sha256: String,
    product_id: String,
    part_id: String,
    lot_id: String,
    product_revision: String,
    operation_id: String,
    effective_from_ns: i64,
    effective_until_ns: i64,
    requirements: Vec<Requirement>,
}

#[derive(Clone, Debug)]
struct ScalarReport {
    report_id: String,
    report_sha256: String,
    measurement_sha256: String,
    requirement_id: String,
    part_id: String,
    lot_id: String,
    product_revision: String,
    operation_id: String,
    measurand_id: String,
    unit: String,
    specification_id: String,
    specification_revision: String,
    measured_at_ns: i64,
    receipt_state: ReceiptState,
    outcome: ScalarOutcome,
    reason_code: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AggregateOutcome {
    CharacteristicsAccepted,
    RejectNonconforming,
    Indeterminate,
}

impl AggregateOutcome {
    fn code(self) -> &'static str {
        match self {
            Self::CharacteristicsAccepted => "CHARACTERISTICS_ACCEPTED",
            Self::RejectNonconforming => "REJECT_NONCONFORMING",
            Self::Indeterminate => "INDETERMINATE",
        }
    }
}

#[derive(Clone, Debug)]
struct ChildDisposition {
    requirement_id: String,
    characteristic_id: String,
    disposition: String,
    scalar_outcome: String,
    reason: String,
    report_refs: Vec<String>,
}

#[derive(Clone, Debug)]
struct AggregateReport {
    manifest_id: String,
    manifest_revision: String,
    manifest_sha256: String,
    product_id: String,
    part_id: String,
    lot_id: String,
    product_revision: String,
    operation_id: String,
    outcome: AggregateOutcome,
    reason: &'static str,
    required_total: usize,
    accepted_required: usize,
    rejected_required: usize,
    unresolved_required: usize,
    missing_required: usize,
    duplicate_required: usize,
    extra_report_count: usize,
    children: Vec<ChildDisposition>,
    extras: Vec<String>,
}

impl AggregateReport {
    fn child_detail(&self) -> String {
        let mut values = Vec::new();
        for child in &self.children {
            let refs = if child.report_refs.is_empty() {
                "NONE".to_string()
            } else {
                child.report_refs.join("+")
            };
            values.push(format!(
                "{}:{}:{}:{}:{}",
                child.requirement_id,
                child.disposition,
                child.scalar_outcome,
                child.reason,
                refs
            ));
        }
        for extra in &self.extras {
            values.push(format!("EXTRA:{extra}"));
        }
        values.join("|")
    }

    fn csv_row(&self, scenario_id: &str) -> String {
        let strings = [
            scenario_id,
            &self.manifest_id,
            &self.manifest_revision,
            &self.manifest_sha256,
            &self.product_id,
            &self.part_id,
            &self.lot_id,
            &self.product_revision,
            &self.operation_id,
            self.outcome.code(),
            self.reason,
        ];
        let mut fields: Vec<String> = strings.iter().map(|s| csv_escape(s)).collect();
        fields.extend([
            self.required_total.to_string(),
            self.accepted_required.to_string(),
            self.rejected_required.to_string(),
            self.unresolved_required.to_string(),
            self.missing_required.to_string(),
            self.duplicate_required.to_string(),
            self.extra_report_count.to_string(),
            csv_escape(&self.child_detail()),
        ]);
        fields.join(",")
    }
}

fn present(value: &str) -> bool {
    !value.trim().is_empty()
}

fn valid_sha256(value: &str) -> bool {
    value.len() == HASH_LEN && value.bytes().all(|b| b.is_ascii_hexdigit())
}

fn csv_escape(value: &str) -> String {
    if value.chars().any(|ch| matches!(ch, ',' | '"' | '\r' | '\n')) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

fn validate_manifest(manifest: &Manifest) -> Result<(), &'static str> {
    for value in [
        manifest.manifest_id.as_str(),
        manifest.manifest_revision.as_str(),
        manifest.product_id.as_str(),
        manifest.part_id.as_str(),
        manifest.lot_id.as_str(),
        manifest.product_revision.as_str(),
        manifest.operation_id.as_str(),
    ] {
        if !present(value) {
            return Err("MANIFEST_REQUIRED_IDENTITY_MISSING");
        }
    }
    if !valid_sha256(&manifest.manifest_sha256) {
        return Err("MANIFEST_DIGEST_INVALID");
    }
    if manifest.effective_from_ns > manifest.effective_until_ns {
        return Err("MANIFEST_VALIDITY_INTERVAL_INVALID");
    }
    if manifest.requirements.is_empty() {
        return Err("MANIFEST_HAS_NO_CHARACTERISTICS");
    }

    let mut requirement_ids = BTreeSet::new();
    let mut characteristic_ids = BTreeSet::new();
    let mut mandatory_count = 0usize;
    for requirement in &manifest.requirements {
        for value in [
            requirement.requirement_id.as_str(),
            requirement.characteristic_id.as_str(),
            requirement.measurand_id.as_str(),
            requirement.unit.as_str(),
            requirement.specification_id.as_str(),
            requirement.specification_revision.as_str(),
        ] {
            if !present(value) {
                return Err("REQUIREMENT_IDENTITY_MISSING");
            }
        }
        if !requirement_ids.insert(requirement.requirement_id.as_str()) {
            return Err("DUPLICATE_REQUIREMENT_ID");
        }
        if !characteristic_ids.insert(requirement.characteristic_id.as_str()) {
            return Err("DUPLICATE_CHARACTERISTIC_ID");
        }
        if requirement.mandatory { mandatory_count += 1; }
    }
    if mandatory_count == 0 {
        return Err("MANIFEST_HAS_NO_MANDATORY_CHARACTERISTICS");
    }
    Ok(())
}

fn scalar_matches(
    report: &ScalarReport,
    manifest: &Manifest,
    requirement: &Requirement,
) -> Result<(), &'static str> {
    for value in [
        report.report_id.as_str(),
        report.reason_code.as_str(),
        report.requirement_id.as_str(),
        report.part_id.as_str(),
        report.lot_id.as_str(),
        report.product_revision.as_str(),
        report.operation_id.as_str(),
        report.measurand_id.as_str(),
        report.unit.as_str(),
        report.specification_id.as_str(),
        report.specification_revision.as_str(),
    ] {
        if !present(value) {
            return Err("SCALAR_REPORT_IDENTITY_MISSING");
        }
    }
    if !valid_sha256(&report.report_sha256) || !valid_sha256(&report.measurement_sha256) {
        return Err("SCALAR_REPORT_DIGEST_INVALID");
    }
    if report.receipt_state != ReceiptState::Verified {
        return Err(match report.receipt_state {
            ReceiptState::Missing => "SCALAR_RECEIPT_MISSING",
            ReceiptState::Stale => "SCALAR_RECEIPT_STALE",
            ReceiptState::Mismatched => "SCALAR_RECEIPT_MISMATCHED",
            ReceiptState::Revoked => "SCALAR_RECEIPT_REVOKED",
            ReceiptState::NotQualified => "SCALAR_RECEIPT_NOT_QUALIFIED",
            ReceiptState::Verified => "UNEXPECTED_RECEIPT_STATE",
        });
    }
    if report.part_id != manifest.part_id
        || report.lot_id != manifest.lot_id
        || report.product_revision != manifest.product_revision
        || report.operation_id != manifest.operation_id
    {
        return Err("SCALAR_REPORT_PART_SCOPE_MISMATCH");
    }
    if report.requirement_id != requirement.requirement_id
        || report.measurand_id != requirement.measurand_id
        || report.unit != requirement.unit
        || report.specification_id != requirement.specification_id
        || report.specification_revision != requirement.specification_revision
    {
        return Err("SCALAR_REPORT_REQUIREMENT_SCOPE_MISMATCH");
    }
    if report.measured_at_ns < manifest.effective_from_ns
        || report.measured_at_ns > manifest.effective_until_ns
    {
        return Err("SCALAR_REPORT_OUTSIDE_MANIFEST_VALIDITY");
    }
    Ok(())
}

fn ref_for(report: &ScalarReport) -> String {
    format!("{}@{}", report.report_id, report.report_sha256)
}

fn aggregate(manifest: &Manifest, reports: &[ScalarReport]) -> AggregateReport {
    let mut result = AggregateReport {
        manifest_id: manifest.manifest_id.clone(),
        manifest_revision: manifest.manifest_revision.clone(),
        manifest_sha256: manifest.manifest_sha256.clone(),
        product_id: manifest.product_id.clone(),
        part_id: manifest.part_id.clone(),
        lot_id: manifest.lot_id.clone(),
        product_revision: manifest.product_revision.clone(),
        operation_id: manifest.operation_id.clone(),
        outcome: AggregateOutcome::Indeterminate,
        reason: "AGGREGATION_NOT_EVALUATED",
        required_total: manifest.requirements.iter().filter(|r| r.mandatory).count(),
        accepted_required: 0,
        rejected_required: 0,
        unresolved_required: 0,
        missing_required: 0,
        duplicate_required: 0,
        extra_report_count: 0,
        children: Vec::new(),
        extras: Vec::new(),
    };

    if let Err(reason) = validate_manifest(manifest) {
        result.reason = reason;
        result.unresolved_required = result.required_total;
        return result;
    }

    let mut report_ids = BTreeMap::<&str, usize>::new();
    let mut by_requirement = BTreeMap::<&str, Vec<&ScalarReport>>::new();
    for report in reports {
        *report_ids.entry(report.report_id.as_str()).or_insert(0) += 1;
        by_requirement
            .entry(report.requirement_id.as_str())
            .or_default()
            .push(report);
    }
    let duplicate_report_ids: BTreeSet<&str> = report_ids
        .iter()
        .filter_map(|(id, count)| (*count > 1).then_some(*id))
        .collect();

    let known_requirements: BTreeSet<&str> = manifest
        .requirements
        .iter()
        .map(|r| r.requirement_id.as_str())
        .collect();

    let mut has_verified_rejection = false;
    let mut has_unresolved_mandatory = false;

    for requirement in &manifest.requirements {
        let mut candidates = by_requirement
            .get(requirement.requirement_id.as_str())
            .cloned()
            .unwrap_or_default();
        candidates.sort_by(|a, b| a.report_id.cmp(&b.report_id));

        let references: Vec<String> = candidates.iter().map(|r| ref_for(r)).collect();
        let outcome_codes: Vec<String> = candidates
            .iter()
            .map(|r| r.outcome.code().to_string())
            .collect();

        if !requirement.mandatory {
            let disposition = if candidates.is_empty() {
                "OPTIONAL_NOT_REPORTED"
            } else {
                "OPTIONAL_EVIDENCE_PRESERVED"
            };
            result.children.push(ChildDisposition {
                requirement_id: requirement.requirement_id.clone(),
                characteristic_id: requirement.characteristic_id.clone(),
                disposition: disposition.to_string(),
                scalar_outcome: outcome_codes.join("+"),
                reason: "OPTIONAL_REQUIREMENT_DOES_NOT_GATE_ACCEPTANCE".into(),
                report_refs: references,
            });
            continue;
        }

        if candidates.is_empty() {
            result.missing_required += 1;
            result.unresolved_required += 1;
            has_unresolved_mandatory = true;
            result.children.push(ChildDisposition {
                requirement_id: requirement.requirement_id.clone(),
                characteristic_id: requirement.characteristic_id.clone(),
                disposition: "MISSING".into(),
                scalar_outcome: "NONE".into(),
                reason: "REQUIRED_CHARACTERISTIC_REPORT_MISSING".into(),
                report_refs: Vec::new(),
            });
            continue;
        }

        if candidates.len() > 1 {
            result.duplicate_required += 1;
            result.unresolved_required += 1;
            has_unresolved_mandatory = true;
            result.children.push(ChildDisposition {
                requirement_id: requirement.requirement_id.clone(),
                characteristic_id: requirement.characteristic_id.clone(),
                disposition: "DUPLICATE".into(),
                scalar_outcome: outcome_codes.join("+"),
                reason: "MULTIPLE_REPORTS_REQUIRE_EXPLICIT_SUCCESSOR_BINDING".into(),
                report_refs: references,
            });
            continue;
        }

        let report = candidates[0];
        let duplicate_id = duplicate_report_ids.contains(report.report_id.as_str());
        let scope_result = scalar_matches(report, manifest, requirement);
        if duplicate_id || scope_result.is_err() {
            result.unresolved_required += 1;
            has_unresolved_mandatory = true;
            result.children.push(ChildDisposition {
                requirement_id: requirement.requirement_id.clone(),
                characteristic_id: requirement.characteristic_id.clone(),
                disposition: "INDETERMINATE".into(),
                scalar_outcome: report.outcome.code().into(),
                reason: if duplicate_id {
                    "DUPLICATE_SCALAR_REPORT_ID".into()
                } else {
                    scope_result.unwrap_err().into()
                },
                report_refs: references,
            });
            continue;
        }

        match report.outcome {
            ScalarOutcome::Accept => {
                result.accepted_required += 1;
                result.children.push(ChildDisposition {
                    requirement_id: requirement.requirement_id.clone(),
                    characteristic_id: requirement.characteristic_id.clone(),
                    disposition: "ACCEPTED".into(),
                    scalar_outcome: report.outcome.code().into(),
                    reason: report.reason_code.clone(),
                    report_refs: references,
                });
            }
            ScalarOutcome::RejectNonconforming => {
                result.rejected_required += 1;
                has_verified_rejection = true;
                result.children.push(ChildDisposition {
                    requirement_id: requirement.requirement_id.clone(),
                    characteristic_id: requirement.characteristic_id.clone(),
                    disposition: "REJECTED".into(),
                    scalar_outcome: report.outcome.code().into(),
                    reason: report.reason_code.clone(),
                    report_refs: references,
                });
            }
            ScalarOutcome::Indeterminate | ScalarOutcome::EvidenceIncomplete => {
                result.unresolved_required += 1;
                has_unresolved_mandatory = true;
                result.children.push(ChildDisposition {
                    requirement_id: requirement.requirement_id.clone(),
                    characteristic_id: requirement.characteristic_id.clone(),
                    disposition: "INDETERMINATE".into(),
                    scalar_outcome: report.outcome.code().into(),
                    reason: report.reason_code.clone(),
                    report_refs: references,
                });
            }
        }
    }

    let mut extras: Vec<&ScalarReport> = reports
        .iter()
        .filter(|r| !known_requirements.contains(r.requirement_id.as_str()))
        .collect();
    extras.sort_by(|a, b| a.report_id.cmp(&b.report_id));
    result.extra_report_count = extras.len();
    result.extras = extras.iter().map(|r| ref_for(r)).collect();

    if has_verified_rejection {
        result.outcome = AggregateOutcome::RejectNonconforming;
        result.reason = "AT_LEAST_ONE_VERIFIED_MANDATORY_CHARACTERISTIC_REJECTED";
    } else if has_unresolved_mandatory || result.accepted_required != result.required_total {
        result.outcome = AggregateOutcome::Indeterminate;
        result.reason = "MANDATORY_CHARACTERISTIC_EVIDENCE_INCOMPLETE_OR_UNRESOLVED";
    } else {
        result.outcome = AggregateOutcome::CharacteristicsAccepted;
        result.reason = "ALL_MANDATORY_CHARACTERISTICS_ACCEPTED";
    }
    result
}

fn manifest_fixture() -> Manifest {
    Manifest {
        manifest_id: "REQSET-SHAFT-01".into(),
        manifest_revision: "REQSET-REV-4".into(),
        manifest_sha256: "a".repeat(HASH_LEN),
        product_id: "PRODUCT-SHAFT-01".into(),
        part_id: "PART-0001".into(),
        lot_id: "LOT-0001".into(),
        product_revision: "PART-REV-7".into(),
        operation_id: "OP-FINAL-INSPECTION".into(),
        effective_from_ns: 1_700_000_000,
        effective_until_ns: 1_800_000_000,
        requirements: vec![
            Requirement {
                requirement_id: "REQ-DIA".into(), characteristic_id: "shaft-diameter".into(),
                measurand_id: "diameter".into(), unit: "mm".into(),
                specification_id: "SPEC-DIA".into(), specification_revision: "DIA-REV-3".into(),
                mandatory: true,
            },
            Requirement {
                requirement_id: "REQ-RUN".into(), characteristic_id: "shaft-runout".into(),
                measurand_id: "runout".into(), unit: "mm".into(),
                specification_id: "SPEC-RUN".into(), specification_revision: "RUN-REV-2".into(),
                mandatory: true,
            },
            Requirement {
                requirement_id: "REQ-HARD".into(), characteristic_id: "surface-hardness".into(),
                measurand_id: "hardness".into(), unit: "HV".into(),
                specification_id: "SPEC-HARD".into(), specification_revision: "HARD-REV-1".into(),
                mandatory: true,
            },
            Requirement {
                requirement_id: "REQ-COAT".into(), characteristic_id: "coating-thickness".into(),
                measurand_id: "thickness".into(), unit: "um".into(),
                specification_id: "SPEC-COAT".into(), specification_revision: "COAT-REV-1".into(),
                mandatory: false,
            },
        ],
    }
}

fn scalar_fixture(requirement_id: &str, report_id: &str, outcome: ScalarOutcome) -> ScalarReport {
    let manifest = manifest_fixture();
    let requirement = manifest.requirements.iter()
        .find(|r| r.requirement_id == requirement_id)
        .expect("fixture requirement exists");
    ScalarReport {
        report_id: report_id.into(),
        report_sha256: format!("{:0<64}", report_id),
        measurement_sha256: format!("{:0<64}", format!("MEAS-{report_id}")),
        requirement_id: requirement.requirement_id.clone(),
        part_id: manifest.part_id,
        lot_id: manifest.lot_id,
        product_revision: manifest.product_revision,
        operation_id: manifest.operation_id,
        measurand_id: requirement.measurand_id.clone(),
        unit: requirement.unit.clone(),
        specification_id: requirement.specification_id.clone(),
        specification_revision: requirement.specification_revision.clone(),
        measured_at_ns: 1_750_000_000,
        receipt_state: ReceiptState::Verified,
        outcome,
        reason_code: format!("SYNTHETIC_{}", outcome.code()),
    }
}

fn all_accept_reports() -> Vec<ScalarReport> {
    vec![
        scalar_fixture("REQ-DIA", "REPORT-DIA", ScalarOutcome::Accept),
        scalar_fixture("REQ-RUN", "REPORT-RUN", ScalarOutcome::Accept),
        scalar_fixture("REQ-HARD", "REPORT-HARD", ScalarOutcome::Accept),
    ]
}

struct Scenario {
    id: &'static str,
    manifest: Manifest,
    reports: Vec<ScalarReport>,
    expected: AggregateOutcome,
}

fn scenarios() -> Vec<Scenario> {
    let all_accept = all_accept_reports();

    let mut one_reject = all_accept_reports();
    one_reject[1].outcome = ScalarOutcome::RejectNonconforming;

    let mut one_indeterminate = all_accept_reports();
    one_indeterminate[1].outcome = ScalarOutcome::Indeterminate;

    let mut missing = all_accept_reports();
    missing.remove(1);

    let mut duplicate = all_accept_reports();
    duplicate.push(scalar_fixture("REQ-DIA", "REPORT-DIA-OLD", ScalarOutcome::Accept));

    let optional_missing = all_accept_reports();

    let mut optional_reject = all_accept_reports();
    optional_reject.push(scalar_fixture("REQ-COAT", "REPORT-COAT", ScalarOutcome::RejectNonconforming));

    let mut extra = all_accept_reports();
    let mut unknown = scalar_fixture("REQ-DIA", "REPORT-EXTRA", ScalarOutcome::RejectNonconforming);
    unknown.requirement_id = "REQ-UNKNOWN".into();
    extra.push(unknown);

    let mut wrong_scope = all_accept_reports();
    wrong_scope[0].specification_revision = "DIA-REV-OLD".into();

    let mut revoked = all_accept_reports();
    revoked[0].receipt_state = ReceiptState::Revoked;

    let mut reject_and_missing = all_accept_reports();
    reject_and_missing[0].outcome = ScalarOutcome::RejectNonconforming;
    reject_and_missing.remove(1);

    let mut invalid_manifest = manifest_fixture();
    invalid_manifest.requirements[1].requirement_id = invalid_manifest.requirements[0].requirement_id.clone();

    vec![
        Scenario { id: "all_accept", manifest: manifest_fixture(), reports: all_accept, expected: AggregateOutcome::CharacteristicsAccepted },
        Scenario { id: "one_reject", manifest: manifest_fixture(), reports: one_reject, expected: AggregateOutcome::RejectNonconforming },
        Scenario { id: "one_indeterminate", manifest: manifest_fixture(), reports: one_indeterminate, expected: AggregateOutcome::Indeterminate },
        Scenario { id: "missing_required", manifest: manifest_fixture(), reports: missing, expected: AggregateOutcome::Indeterminate },
        Scenario { id: "duplicate_required", manifest: manifest_fixture(), reports: duplicate, expected: AggregateOutcome::Indeterminate },
        Scenario { id: "optional_missing", manifest: manifest_fixture(), reports: optional_missing, expected: AggregateOutcome::CharacteristicsAccepted },
        Scenario { id: "optional_reject_preserved", manifest: manifest_fixture(), reports: optional_reject, expected: AggregateOutcome::CharacteristicsAccepted },
        Scenario { id: "unknown_extra_preserved", manifest: manifest_fixture(), reports: extra, expected: AggregateOutcome::CharacteristicsAccepted },
        Scenario { id: "spec_revision_mismatch", manifest: manifest_fixture(), reports: wrong_scope, expected: AggregateOutcome::Indeterminate },
        Scenario { id: "revoked_scalar_receipt", manifest: manifest_fixture(), reports: revoked, expected: AggregateOutcome::Indeterminate },
        Scenario { id: "reject_precedes_other_missing", manifest: manifest_fixture(), reports: reject_and_missing, expected: AggregateOutcome::RejectNonconforming },
        Scenario { id: "invalid_manifest", manifest: invalid_manifest, reports: all_accept_reports(), expected: AggregateOutcome::Indeterminate },
    ]
}

fn verify() -> Result<(), String> {
    for scenario in scenarios() {
        let report = aggregate(&scenario.manifest, &scenario.reports);
        if report.outcome != scenario.expected {
            return Err(format!(
                "AGGREGATE_GOLDEN_MISMATCH {}: got {}, expected {}",
                scenario.id, report.outcome.code(), scenario.expected.code()
            ));
        }
        if report.outcome == AggregateOutcome::CharacteristicsAccepted
            && report.accepted_required != report.required_total
        {
            return Err(format!("FALSE_CHARACTERISTICS_ACCEPTED: {}", scenario.id));
        }
        if report.outcome == AggregateOutcome::RejectNonconforming
            && report.rejected_required == 0
        {
            return Err(format!("FALSE_REJECT: {}", scenario.id));
        }
    }

    // Exhaustive 4^3 truth table over ACCEPT, REJECT, INDETERMINATE, EVIDENCE_INCOMPLETE.
    let outcomes = [
        ScalarOutcome::Accept,
        ScalarOutcome::RejectNonconforming,
        ScalarOutcome::Indeterminate,
        ScalarOutcome::EvidenceIncomplete,
    ];
    for a in outcomes {
        for b in outcomes {
            for c in outcomes {
                let reports = vec![
                    scalar_fixture("REQ-DIA", "TT-DIA", a),
                    scalar_fixture("REQ-RUN", "TT-RUN", b),
                    scalar_fixture("REQ-HARD", "TT-HARD", c),
                ];
                let actual = aggregate(&manifest_fixture(), &reports).outcome;
                let expected = if [a, b, c].contains(&ScalarOutcome::RejectNonconforming) {
                    AggregateOutcome::RejectNonconforming
                } else if [a, b, c].iter().all(|outcome| *outcome == ScalarOutcome::Accept) {
                    AggregateOutcome::CharacteristicsAccepted
                } else {
                    AggregateOutcome::Indeterminate
                };
                if actual != expected {
                    return Err(format!("TRUTH_TABLE_MISMATCH: {a:?}/{b:?}/{c:?}"));
                }
            }
        }
    }

    // Input report order must not change the canonical receipt.
    let forward = all_accept_reports();
    let mut reverse = forward.clone();
    reverse.reverse();
    let a = aggregate(&manifest_fixture(), &forward).csv_row("order_test");
    let b = aggregate(&manifest_fixture(), &reverse).csv_row("order_test");
    if a != b {
        return Err("REPORT_ORDER_NONDETERMINISM".into());
    }

    Ok(())
}

fn emit_csv() {
    println!("scenario_id,manifest_id,manifest_revision,manifest_sha256,product_id,part_id,lot_id,product_revision,operation_id,aggregate_outcome,reason,required_total,accepted_required,rejected_required,unresolved_required,missing_required,duplicate_required,extra_report_count,child_detail");
    for scenario in scenarios() {
        let report = aggregate(&scenario.manifest, &scenario.reports);
        println!("{}", report.csv_row(scenario.id));
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.as_slice() {
        [] => verify().map(|()| println!("MFG_PART_AGGREGATE_OK: multi-characteristic conformity truth table")),
        [arg] if arg == "--verify" => verify().map(|()| println!("MFG_PART_AGGREGATE_OK: multi-characteristic conformity truth table")),
        [arg] if arg == "--csv" => { emit_csv(); Ok(()) },
        [arg] if arg == "--help" || arg == "-h" => { println!("Usage: part-aggregate [--verify|--csv|--help]"); Ok(()) },
        _ => Err("expected --verify, --csv or --help".into()),
    };
    if let Err(error) = result {
        eprintln!("MFG_PART_AGGREGATE_ERROR: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_mandatory_characteristics_must_accept() {
        let report = aggregate(&manifest_fixture(), &all_accept_reports());
        assert_eq!(report.outcome, AggregateOutcome::CharacteristicsAccepted);
        assert_eq!(report.accepted_required, 3);
        assert_eq!(report.required_total, 3);
    }

    #[test]
    fn verified_rejection_takes_precedence_over_unresolved_other_characteristic() {
        let mut reports = all_accept_reports();
        reports[0].outcome = ScalarOutcome::RejectNonconforming;
        reports.remove(1);
        let report = aggregate(&manifest_fixture(), &reports);
        assert_eq!(report.outcome, AggregateOutcome::RejectNonconforming);
        assert_eq!(report.rejected_required, 1);
        assert_eq!(report.missing_required, 1);
    }

    #[test]
    fn any_unresolved_or_missing_mandatory_characteristic_blocks_acceptance() {
        for outcome in [ScalarOutcome::Indeterminate, ScalarOutcome::EvidenceIncomplete] {
            let mut reports = all_accept_reports();
            reports[1].outcome = outcome;
            assert_eq!(aggregate(&manifest_fixture(), &reports).outcome, AggregateOutcome::Indeterminate);
        }
        let mut reports = all_accept_reports();
        reports.remove(2);
        assert_eq!(aggregate(&manifest_fixture(), &reports).outcome, AggregateOutcome::Indeterminate);
    }

    #[test]
    fn duplicate_required_reports_block_acceptance_without_guessing_latest() {
        let mut reports = all_accept_reports();
        reports.push(scalar_fixture("REQ-DIA", "REPORT-DIA-OLD", ScalarOutcome::Accept));
        let report = aggregate(&manifest_fixture(), &reports);
        assert_eq!(report.outcome, AggregateOutcome::Indeterminate);
        assert_eq!(report.duplicate_required, 1);
    }

    #[test]
    fn optional_characteristics_do_not_gate_but_remain_in_receipt() {
        let manifest = manifest_fixture();
        let report = aggregate(&manifest, &all_accept_reports());
        assert_eq!(report.outcome, AggregateOutcome::CharacteristicsAccepted);
        assert!(report.children.iter().any(|child| child.requirement_id == "REQ-COAT"
            && child.disposition == "OPTIONAL_NOT_REPORTED"));

        let mut reports = all_accept_reports();
        reports.push(scalar_fixture("REQ-COAT", "REPORT-COAT", ScalarOutcome::RejectNonconforming));
        let report = aggregate(&manifest, &reports);
        assert_eq!(report.outcome, AggregateOutcome::CharacteristicsAccepted);
        assert!(report.children.iter().any(|child| child.requirement_id == "REQ-COAT"
            && child.scalar_outcome == "REJECT_NONCONFORMING"));
    }

    #[test]
    fn unknown_extra_evidence_is_preserved_not_substituted() {
        let mut reports = all_accept_reports();
        let mut extra = scalar_fixture("REQ-DIA", "REPORT-EXTRA", ScalarOutcome::RejectNonconforming);
        extra.requirement_id = "REQ-UNKNOWN".into();
        reports.push(extra);
        let report = aggregate(&manifest_fixture(), &reports);
        assert_eq!(report.outcome, AggregateOutcome::CharacteristicsAccepted);
        assert_eq!(report.extra_report_count, 1);
        assert!(report.extras[0].starts_with("REPORT-EXTRA@"));
    }

    #[test]
    fn scope_mismatch_revocation_and_duplicate_receipt_id_abstain() {
        let mut reports = all_accept_reports();
        reports[0].specification_revision = "DIA-REV-OLD".into();
        assert_eq!(aggregate(&manifest_fixture(), &reports).outcome, AggregateOutcome::Indeterminate);

        let mut reports = all_accept_reports();
        reports[0].receipt_state = ReceiptState::Revoked;
        assert_eq!(aggregate(&manifest_fixture(), &reports).outcome, AggregateOutcome::Indeterminate);

        let mut reports = all_accept_reports();
        reports[1].report_id = reports[0].report_id.clone();
        assert_eq!(aggregate(&manifest_fixture(), &reports).outcome, AggregateOutcome::Indeterminate);
    }

    #[test]
    fn malformed_manifest_cannot_produce_characteristics_accepted() {
        let mut manifest = manifest_fixture();
        manifest.requirements[1].requirement_id = manifest.requirements[0].requirement_id.clone();
        assert_eq!(aggregate(&manifest, &all_accept_reports()).outcome, AggregateOutcome::Indeterminate);
    }

    #[test]
    fn exhaustive_three_characteristic_truth_table_passes() {
        verify().unwrap();
    }

    #[test]
    fn report_input_order_does_not_change_canonical_receipt() {
        let forward = all_accept_reports();
        let mut reverse = forward.clone();
        reverse.reverse();
        assert_eq!(
            aggregate(&manifest_fixture(), &forward).csv_row("replay"),
            aggregate(&manifest_fixture(), &reverse).csv_row("replay")
        );
    }

    #[test]
    fn no_product_release_outcome_exists() {
        assert_eq!(AggregateOutcome::CharacteristicsAccepted.code(), "CHARACTERISTICS_ACCEPTED");
        assert_ne!(AggregateOutcome::CharacteristicsAccepted.code(), "PRODUCT_RELEASED");
    }
}
