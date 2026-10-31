//! Deterministic part-level aggregation of scalar manufacturing conformity receipts.
//!
//! This layer does not recompute metrology or probability. It consumes scalar
//! dispositions and preserves exact genealogy, current/superseded receipts and
//! missingness. CHARACTERISTICS_ACCEPTED is NOT product release.
//!
//! Compile:
//!   rustc --edition=2024 -D warnings --test tools/manufacturing-conformity/part_aggregate.rs -o /tmp/part-aggregate-tests
//!   /tmp/part-aggregate-tests
//!   rustc --edition=2024 -D warnings tools/manufacturing-conformity/part_aggregate.rs -o /tmp/part-aggregate
//!   /tmp/part-aggregate --verify
//!   /tmp/part-aggregate --csv

const HASH_LEN: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EvidenceState {
    Verified,
    Missing,
    Stale,
    Mismatched,
    Revoked,
    NotQualified,
}

impl EvidenceState {
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PartOutcome {
    CharacteristicsAccepted,
    RejectNonconforming,
    Indeterminate,
    InvalidManifest,
}

impl PartOutcome {
    fn code(self) -> &'static str {
        match self {
            Self::CharacteristicsAccepted => "CHARACTERISTICS_ACCEPTED",
            Self::RejectNonconforming => "REJECT_NONCONFORMING",
            Self::Indeterminate => "INDETERMINATE",
            Self::InvalidManifest => "INVALID_MANIFEST",
        }
    }
}

#[derive(Clone, Debug)]
struct Requirement {
    characteristic_id: String,
    mandatory: bool,
    measurand_id: String,
    unit: String,
    specification_id: String,
    specification_revision: String,
    current_receipt_id: Option<String>,
}

#[derive(Clone, Debug)]
struct Manifest {
    manifest_id: String,
    manifest_revision: String,
    manifest_sha256: String,
    part_id: String,
    serial_id: String,
    lot_id: String,
    product_revision: String,
    operation_id: String,
    effective_from_ns: i64,
    effective_until_ns: i64,
    requirements: Vec<Requirement>,
}

#[derive(Clone, Debug)]
struct ScalarReceipt {
    receipt_id: String,
    receipt_sha256: String,
    characteristic_id: String,
    part_id: String,
    serial_id: String,
    lot_id: String,
    product_revision: String,
    operation_id: String,
    measurand_id: String,
    unit: String,
    specification_id: String,
    specification_revision: String,
    outcome: ScalarOutcome,
    reason_code: String,
    evidence_state: EvidenceState,
    is_current: bool,
    supersedes_receipt_id: Option<String>,
    measured_at_ns: i64,
    issued_at_ns: i64,
}

#[derive(Clone, Debug)]
struct Request {
    manifest: Manifest,
    decision_time_ns: i64,
    child_receipts: Vec<ScalarReceipt>,
}

#[derive(Clone, Debug)]
struct CharacteristicResult {
    characteristic_id: String,
    mandatory: bool,
    current_receipt_id: Option<String>,
    current_receipt_sha256: Option<String>,
    scalar_outcome: Option<ScalarOutcome>,
    scalar_reason: Option<String>,
    historical_receipt_ids: Vec<String>,
    disposition: &'static str,
}

#[derive(Clone, Debug)]
struct Report {
    manifest_id: String,
    manifest_revision: String,
    manifest_sha256: String,
    part_id: String,
    serial_id: String,
    lot_id: String,
    product_revision: String,
    operation_id: String,
    decision_time_ns: i64,
    required_count: usize,
    mandatory_count: usize,
    accepted_count: usize,
    rejected_count: usize,
    indeterminate_count: usize,
    missing_count: usize,
    optional_missing_count: usize,
    extra_receipt_count: usize,
    outcome: PartOutcome,
    reason: &'static str,
    characteristic_results: Vec<CharacteristicResult>,
    extra_characteristic_ids: Vec<String>,
    child_receipts: Vec<ScalarReceipt>,
}

fn safe_id(value: &str) -> bool {
    !value.trim().is_empty()
        && !value.contains(',')
        && !value.contains('|')
        && !value.contains('\n')
        && !value.contains('\r')
}

fn valid_sha(value: &str) -> bool {
    value.len() == HASH_LEN && value.bytes().all(|b| b.is_ascii_hexdigit())
}

fn invalid_manifest(request: &Request, reason: &'static str) -> Report {
    Report {
        manifest_id: request.manifest.manifest_id.clone(),
        manifest_revision: request.manifest.manifest_revision.clone(),
        manifest_sha256: request.manifest.manifest_sha256.clone(),
        part_id: request.manifest.part_id.clone(),
        serial_id: request.manifest.serial_id.clone(),
        lot_id: request.manifest.lot_id.clone(),
        product_revision: request.manifest.product_revision.clone(),
        operation_id: request.manifest.operation_id.clone(),
        decision_time_ns: request.decision_time_ns,
        required_count: request.manifest.requirements.len(),
        mandatory_count: request.manifest.requirements.iter().filter(|r| r.mandatory).count(),
        accepted_count: 0,
        rejected_count: 0,
        indeterminate_count: 0,
        missing_count: 0,
        optional_missing_count: 0,
        extra_receipt_count: request.child_receipts.len(),
        outcome: PartOutcome::InvalidManifest,
        reason,
        characteristic_results: Vec::new(),
        extra_characteristic_ids: Vec::new(),
        child_receipts: {
            let mut receipts = request.child_receipts.clone();
            receipts.sort_by(|a, b| (&a.characteristic_id, a.issued_at_ns, &a.receipt_id)
                .cmp(&(&b.characteristic_id, b.issued_at_ns, &b.receipt_id)));
            receipts
        },
    }
}

fn empty_result(requirement: &Requirement, disposition: &'static str) -> CharacteristicResult {
    CharacteristicResult {
        characteristic_id: requirement.characteristic_id.clone(),
        mandatory: requirement.mandatory,
        current_receipt_id: None,
        current_receipt_sha256: None,
        scalar_outcome: None,
        scalar_reason: None,
        historical_receipt_ids: Vec::new(),
        disposition,
    }
}

fn validate_manifest(request: &Request) -> Result<(), &'static str> {
    let m = &request.manifest;
    for id in [
        m.manifest_id.as_str(), m.manifest_revision.as_str(), m.part_id.as_str(),
        m.serial_id.as_str(), m.lot_id.as_str(), m.product_revision.as_str(),
        m.operation_id.as_str(),
    ] {
        if !safe_id(id) {
            return Err("MANIFEST_REQUIRED_IDENTITY_MISSING_OR_UNSAFE");
        }
    }
    if !valid_sha(&m.manifest_sha256) {
        return Err("MANIFEST_DIGEST_INVALID");
    }
    if m.effective_from_ns > m.effective_until_ns
        || request.decision_time_ns < m.effective_from_ns
        || request.decision_time_ns > m.effective_until_ns
    {
        return Err("MANIFEST_NOT_EFFECTIVE_AT_DECISION_TIME");
    }
    if m.requirements.is_empty() {
        return Err("REQUIRED_CHARACTERISTIC_MANIFEST_EMPTY");
    }
    let mut ids = Vec::with_capacity(m.requirements.len());
    for requirement in &m.requirements {
        if !safe_id(&requirement.characteristic_id)
            || !safe_id(&requirement.measurand_id)
            || !safe_id(&requirement.unit)
            || !safe_id(&requirement.specification_id)
            || !safe_id(&requirement.specification_revision)
        {
            return Err("REQUIREMENT_IDENTITY_MISSING_OR_UNSAFE");
        }
        if requirement.mandatory
            && requirement.current_receipt_id.as_deref().is_none_or(|id| !safe_id(id))
        {
            return Err("MANDATORY_CURRENT_RECEIPT_REFERENCE_MISSING");
        }
        ids.push(requirement.characteristic_id.as_str());
    }
    ids.sort_unstable();
    if ids.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err("DUPLICATE_REQUIRED_CHARACTERISTIC");
    }
    Ok(())
}

fn receipt_scope_matches(manifest: &Manifest, requirement: &Requirement, receipt: &ScalarReceipt) -> bool {
    receipt.part_id == manifest.part_id
        && receipt.serial_id == manifest.serial_id
        && receipt.lot_id == manifest.lot_id
        && receipt.product_revision == manifest.product_revision
        && receipt.operation_id == manifest.operation_id
        && receipt.characteristic_id == requirement.characteristic_id
        && receipt.measurand_id == requirement.measurand_id
        && receipt.unit == requirement.unit
        && receipt.specification_id == requirement.specification_id
        && receipt.specification_revision == requirement.specification_revision
        && receipt.measured_at_ns >= manifest.effective_from_ns
        && receipt.measured_at_ns <= manifest.effective_until_ns
        && receipt.issued_at_ns >= receipt.measured_at_ns
}

fn validate_receipt_identity(receipt: &ScalarReceipt) -> Result<(), &'static str> {
    for value in [
        receipt.receipt_id.as_str(), receipt.characteristic_id.as_str(),
        receipt.part_id.as_str(), receipt.serial_id.as_str(), receipt.lot_id.as_str(),
        receipt.product_revision.as_str(), receipt.operation_id.as_str(),
        receipt.measurand_id.as_str(), receipt.unit.as_str(),
        receipt.specification_id.as_str(), receipt.specification_revision.as_str(),
        receipt.reason_code.as_str(),
    ] {
        if !safe_id(value) {
            return Err("SCALAR_RECEIPT_IDENTITY_INVALID");
        }
    }
    if !valid_sha(&receipt.receipt_sha256) {
        return Err("SCALAR_RECEIPT_DIGEST_INVALID");
    }
    Ok(())
}

fn validate_linear_supersession_chain(receipts: &[&ScalarReceipt], current_id: &str) -> Result<Vec<String>, &'static str> {
    if receipts.is_empty() {
        return Err("REQUIRED_SCALAR_RECEIPT_MISSING");
    }
    let current_nodes: Vec<&ScalarReceipt> = receipts.iter().copied()
        .filter(|receipt| receipt.is_current)
        .collect();
    if current_nodes.len() != 1 || current_nodes[0].receipt_id != current_id {
        return Err("CURRENT_RECEIPT_CARDINALITY_OR_REFERENCE_INVALID");
    }
    let current = current_nodes[0];

    // Every supersession edge must point to one existing receipt in this
    // characteristic's chain and must move strictly forward in issued time.
    let mut child_count = vec![0_usize; receipts.len()];
    for (index, child) in receipts.iter().enumerate() {
        if let Some(parent_id) = &child.supersedes_receipt_id {
            let parent_index = receipts.iter().position(|candidate| candidate.receipt_id == *parent_id)
                .ok_or("SUPERSESSION_PARENT_MISSING")?;
            let parent = receipts[parent_index];
            if parent_index == index || child.issued_at_ns <= parent.issued_at_ns {
                return Err("SUPERSESSION_ORDER_INVALID");
            }
            child_count[parent_index] += 1;
            if child_count[parent_index] > 1 {
                return Err("SUPERSESSION_BRANCH_DETECTED");
            }
        }
    }
    if receipts.iter().enumerate().any(|(index, receipt)| {
        receipt.is_current && child_count[index] != 0
    }) {
        return Err("CURRENT_RECEIPT_HAS_SUCCESSOR");
    }
    let root_count = receipts.iter().filter(|r| r.supersedes_receipt_id.is_none()).count();
    if root_count != 1 {
        return Err("SUPERSESSION_CHAIN_ROOT_CARDINALITY_INVALID");
    }

    let mut visited = Vec::new();
    let mut cursor = current;
    loop {
        visited.push(cursor.receipt_id.clone());
        match &cursor.supersedes_receipt_id {
            Some(parent_id) => {
                cursor = receipts.iter().copied()
                    .find(|candidate| candidate.receipt_id == *parent_id)
                    .ok_or("SUPERSESSION_PARENT_MISSING")?;
                if visited.iter().any(|id| id == &cursor.receipt_id) {
                    return Err("SUPERSESSION_CYCLE_DETECTED");
                }
            }
            None => break,
        }
    }
    if visited.len() != receipts.len() {
        return Err("SUPERSESSION_CHAIN_DISCONNECTED");
    }
    visited.reverse();
    Ok(visited)
}

fn aggregate(request: &Request) -> Report {
    if let Err(reason) = validate_manifest(request) {
        return invalid_manifest(request, reason);
    }

    let mut receipt_ids = Vec::with_capacity(request.child_receipts.len());
    for receipt in &request.child_receipts {
        if let Err(reason) = validate_receipt_identity(receipt) {
            return invalid_manifest(request, reason);
        }
        receipt_ids.push(receipt.receipt_id.as_str());
    }
    receipt_ids.sort_unstable();
    if receipt_ids.windows(2).any(|pair| pair[0] == pair[1]) {
        return invalid_manifest(request, "DUPLICATE_SCALAR_RECEIPT_ID");
    }

    let mut requirement_refs: Vec<&Requirement> = request.manifest.requirements.iter().collect();
    requirement_refs.sort_by(|a, b| a.characteristic_id.cmp(&b.characteristic_id));

    let mut results = Vec::with_capacity(requirement_refs.len());
    let mut accepted = 0;
    let mut rejected = 0;
    let mut indeterminate = 0;
    let mut missing = 0;
    let mut optional_missing = 0;
    let mut has_valid_reject = false;
    let mut has_indeterminate = false;

    for requirement in requirement_refs {
        let matching: Vec<&ScalarReceipt> = request.child_receipts.iter()
            .filter(|receipt| receipt.characteristic_id == requirement.characteristic_id)
            .collect();

        if matching.is_empty() {
            if requirement.mandatory {
                missing += 1;
                indeterminate += 1;
                has_indeterminate = true;
                results.push(empty_result(requirement, "MISSING_MANDATORY_RECEIPT"));
            } else {
                optional_missing += 1;
                results.push(empty_result(requirement, "OPTIONAL_NOT_MEASURED"));
            }
            continue;
        }

        if matching.iter().any(|receipt| !receipt_scope_matches(&request.manifest, requirement, receipt)) {
            indeterminate += usize::from(requirement.mandatory);
            has_indeterminate |= requirement.mandatory;
            results.push(empty_result(requirement, "RECEIPT_SCOPE_MISMATCH"));
            continue;
        }

        let current_id = match requirement.current_receipt_id.as_deref() {
            Some(id) => id,
            None if !requirement.mandatory => {
                if matching.iter().any(|receipt| receipt.is_current) {
                    indeterminate += 1;
                    has_indeterminate = true;
                    results.push(empty_result(requirement, "OPTIONAL_CURRENT_RECEIPT_NOT_MANIFESTED"));
                } else {
                    results.push(empty_result(requirement, "OPTIONAL_HISTORICAL_ONLY"));
                }
                continue;
            }
            None => {
                missing += 1;
                indeterminate += 1;
                has_indeterminate = true;
                results.push(empty_result(requirement, "MANDATORY_CURRENT_RECEIPT_ID_MISSING"));
                continue;
            }
        };

        let history = match validate_linear_supersession_chain(&matching, current_id) {
            Ok(history) => history,
            Err(reason) => {
                indeterminate += usize::from(requirement.mandatory);
                has_indeterminate |= requirement.mandatory;
                results.push(empty_result(requirement, reason));
                continue;
            }
        };

        let current = matching.iter().copied()
            .find(|receipt| receipt.receipt_id == current_id);
        let current = match current {
            Some(value) => value,
            None => {
                missing += 1;
                indeterminate += usize::from(requirement.mandatory);
                has_indeterminate |= requirement.mandatory;
                results.push(empty_result(requirement, "MANIFEST_CURRENT_RECEIPT_NOT_FOUND"));
                continue;
            }
        };

        if current.evidence_state != EvidenceState::Verified {
            indeterminate += usize::from(requirement.mandatory);
            has_indeterminate |= requirement.mandatory;
            results.push(CharacteristicResult {
                characteristic_id: requirement.characteristic_id.clone(),
                mandatory: requirement.mandatory,
                current_receipt_id: Some(current.receipt_id.clone()),
                current_receipt_sha256: Some(current.receipt_sha256.clone()),
                scalar_outcome: Some(current.outcome),
                scalar_reason: Some(current.reason_code.clone()),
                historical_receipt_ids: history,
                disposition: "CURRENT_EVIDENCE_NOT_VERIFIED",
            });
            continue;
        }

        let disposition = match current.outcome {
            ScalarOutcome::Accept if requirement.mandatory => {
                accepted += 1;
                "CURRENT_ACCEPT"
            }
            ScalarOutcome::Accept => "OPTIONAL_CURRENT_ACCEPT",
            ScalarOutcome::RejectNonconforming if requirement.mandatory => {
                rejected += 1;
                has_valid_reject = true;
                "CURRENT_REJECT_NONCONFORMING"
            }
            ScalarOutcome::RejectNonconforming => "OPTIONAL_CURRENT_REJECT",
            ScalarOutcome::Indeterminate if requirement.mandatory => {
                indeterminate += 1;
                has_indeterminate = true;
                "CURRENT_INDETERMINATE"
            }
            ScalarOutcome::Indeterminate => "OPTIONAL_CURRENT_INDETERMINATE",
            ScalarOutcome::EvidenceIncomplete if requirement.mandatory => {
                indeterminate += 1;
                has_indeterminate = true;
                "CURRENT_EVIDENCE_INCOMPLETE"
            }
            ScalarOutcome::EvidenceIncomplete => "OPTIONAL_CURRENT_EVIDENCE_INCOMPLETE",
        };

        results.push(CharacteristicResult {
            characteristic_id: requirement.characteristic_id.clone(),
            mandatory: requirement.mandatory,
            current_receipt_id: Some(current.receipt_id.clone()),
            current_receipt_sha256: Some(current.receipt_sha256.clone()),
            scalar_outcome: Some(current.outcome),
            scalar_reason: Some(current.reason_code.clone()),
            historical_receipt_ids: history.into_iter()
                .filter(|id| id != current_id)
                .collect(),
            disposition,
        });
    }

    let extra_ids: Vec<String> = request.child_receipts.iter()
        .filter(|receipt| !request.manifest.requirements.iter()
            .any(|requirement| requirement.characteristic_id == receipt.characteristic_id))
        .map(|receipt| receipt.characteristic_id.clone())
        .collect();
    let mut extra_ids = extra_ids;
    extra_ids.sort();
    extra_ids.dedup();

    let outcome = if has_valid_reject {
        PartOutcome::RejectNonconforming
    } else if has_indeterminate {
        PartOutcome::Indeterminate
    } else {
        PartOutcome::CharacteristicsAccepted
    };
    let reason = match outcome {
        PartOutcome::RejectNonconforming => "VALID_MANDATORY_CHARACTERISTIC_NONCONFORMING",
        PartOutcome::Indeterminate => "MANDATORY_CHARACTERISTIC_UNRESOLVED_OR_EVIDENCE_INCOMPLETE",
        PartOutcome::CharacteristicsAccepted => "ALL_MANDATORY_CHARACTERISTICS_ACCEPTED",
        PartOutcome::InvalidManifest => "MANIFEST_OR_CHILD_EVIDENCE_INVALID",
    };

    Report {
        manifest_id: request.manifest.manifest_id.clone(),
        manifest_revision: request.manifest.manifest_revision.clone(),
        manifest_sha256: request.manifest.manifest_sha256.clone(),
        part_id: request.manifest.part_id.clone(),
        serial_id: request.manifest.serial_id.clone(),
        lot_id: request.manifest.lot_id.clone(),
        product_revision: request.manifest.product_revision.clone(),
        operation_id: request.manifest.operation_id.clone(),
        decision_time_ns: request.decision_time_ns,
        required_count: request.manifest.requirements.len(),
        mandatory_count: request.manifest.requirements.iter().filter(|r| r.mandatory).count(),
        accepted_count: accepted,
        rejected_count: rejected,
        indeterminate_count: indeterminate,
        missing_count: missing,
        optional_missing_count: optional_missing,
        extra_receipt_count: request.child_receipts.iter()
            .filter(|receipt| !request.manifest.requirements.iter()
                .any(|requirement| requirement.characteristic_id == receipt.characteristic_id))
            .count(),
        outcome,
        reason,
        characteristic_results: results,
        extra_characteristic_ids: extra_ids,
        child_receipts: {
            let mut receipts = request.child_receipts.clone();
            receipts.sort_by(|a, b| (&a.characteristic_id, a.issued_at_ns, &a.receipt_id)
                .cmp(&(&b.characteristic_id, b.issued_at_ns, &b.receipt_id)));
            receipts
        },
    }
}

fn csv_header() -> &'static str {
    "row_type,manifest_id,manifest_revision,manifest_sha256,part_id,serial_id,lot_id,product_revision,operation_id,decision_time_ns,required_count,mandatory_count,accepted_count,rejected_count,indeterminate_count,missing_count,optional_missing_count,extra_receipt_count,extra_characteristic_ids,part_outcome,part_reason,characteristic_id,mandatory,current_receipt_id,current_receipt_sha256,scalar_outcome,scalar_reason,historical_receipt_ids,characteristic_disposition,source_receipt_id,source_receipt_sha256,source_receipt_characteristic_id,source_receipt_measurand_id,source_receipt_unit,source_receipt_specification_id,source_receipt_specification_revision,source_receipt_outcome,source_receipt_reason,source_receipt_evidence_state,source_receipt_is_current,source_receipt_supersedes_id,source_receipt_measured_at_ns,source_receipt_issued_at_ns"
}

fn csv_rows(report: &Report) -> Vec<String> {
    let mut rows = Vec::new();
    let summary_prefix = vec![
        report.manifest_id.clone(), report.manifest_revision.clone(),
        report.manifest_sha256.clone(), report.part_id.clone(), report.serial_id.clone(),
        report.lot_id.clone(), report.product_revision.clone(), report.operation_id.clone(),
        report.decision_time_ns.to_string(), report.required_count.to_string(),
        report.mandatory_count.to_string(), report.accepted_count.to_string(),
        report.rejected_count.to_string(), report.indeterminate_count.to_string(),
        report.missing_count.to_string(), report.optional_missing_count.to_string(),
        report.extra_receipt_count.to_string(), report.extra_characteristic_ids.join("|"),
        report.outcome.code().to_string(), report.reason.to_string(),
    ];

    for item in &report.characteristic_results {
        let mut cells = vec!["REQUIREMENT".to_string()];
        cells.extend(summary_prefix.clone());
        cells.extend([
            item.characteristic_id.clone(),
            item.mandatory.to_string(),
            item.current_receipt_id.clone().unwrap_or_default(),
            item.current_receipt_sha256.clone().unwrap_or_default(),
            item.scalar_outcome.map(ScalarOutcome::code).unwrap_or_default().to_string(),
            item.scalar_reason.clone().unwrap_or_default(),
            item.historical_receipt_ids.join("|"),
            item.disposition.to_string(),
        ]);
        cells.extend(std::iter::repeat_n(String::new(), 14));
        rows.push(cells.join(","));
    }

    // Emit every source receipt as its own row, including historical/superseded
    // and undeclared extra receipts. The report can therefore be replayed
    // without relying on only the selected current child receipt.
    for child in &report.child_receipts {
        let mut cells = vec!["SOURCE_RECEIPT".to_string()];
        cells.extend(summary_prefix.clone());
        cells.extend(std::iter::repeat_n(String::new(), 8));
        cells.extend([
            child.receipt_id.clone(),
            child.receipt_sha256.clone(),
            child.characteristic_id.clone(),
            child.measurand_id.clone(),
            child.unit.clone(),
            child.specification_id.clone(),
            child.specification_revision.clone(),
            child.outcome.code().to_string(),
            child.reason_code.clone(),
            child.evidence_state.code().to_string(),
            child.is_current.to_string(),
            child.supersedes_receipt_id.clone().unwrap_or_default(),
            child.measured_at_ns.to_string(),
            child.issued_at_ns.to_string(),
        ]);
        rows.push(cells.join(","));
    }
    rows
}

fn receipt(characteristic: &str, id: &str, outcome: ScalarOutcome) -> ScalarReceipt {
    ScalarReceipt {
        receipt_id: id.into(),
        receipt_sha256: "a".repeat(HASH_LEN),
        characteristic_id: characteristic.into(),
        part_id: "PART-0001".into(),
        serial_id: "SERIAL-001".into(),
        lot_id: "LOT-001".into(),
        product_revision: "REV-C".into(),
        operation_id: "FINAL-INSPECTION".into(),
        measurand_id: format!("measurand-{characteristic}"),
        unit: if characteristic == "winding-resistance" { "ohm" } else { "mm" }.into(),
        specification_id: format!("spec-{characteristic}"),
        specification_revision: "SPEC-REV-3".into(),
        outcome,
        reason_code: format!("SCALAR_{}", outcome.code()),
        evidence_state: EvidenceState::Verified,
        is_current: true,
        supersedes_receipt_id: None,
        measured_at_ns: 5_000,
        issued_at_ns: 5_100,
    }
}

fn request(requirements: Vec<Requirement>, child_receipts: Vec<ScalarReceipt>) -> Request {
    Request {
        manifest: Manifest {
            manifest_id: "MANIFEST-001".into(),
            manifest_revision: "MANIFEST-REV-4".into(),
            manifest_sha256: "b".repeat(HASH_LEN),
            part_id: "PART-0001".into(),
            serial_id: "SERIAL-001".into(),
            lot_id: "LOT-001".into(),
            product_revision: "REV-C".into(),
            operation_id: "FINAL-INSPECTION".into(),
            effective_from_ns: 1_000,
            effective_until_ns: 9_000,
            requirements,
        },
        decision_time_ns: 6_000,
        child_receipts,
    }
}

fn req(characteristic: &str, mandatory: bool, current: Option<&str>) -> Requirement {
    Requirement {
        characteristic_id: characteristic.into(),
        mandatory,
        measurand_id: format!("measurand-{characteristic}"),
        unit: if characteristic == "winding-resistance" { "ohm" } else { "mm" }.into(),
        specification_id: format!("spec-{characteristic}"),
        specification_revision: "SPEC-REV-3".into(),
        current_receipt_id: current.map(str::to_string),
    }
}

fn verify() -> Result<(), String> {
    let all_accept = request(
        vec![req("diameter", true, Some("R-DIA-1")), req("runout", true, Some("R-RUN-1"))],
        vec![
            receipt("diameter", "R-DIA-1", ScalarOutcome::Accept),
            receipt("runout", "R-RUN-1", ScalarOutcome::Accept),
        ],
    );
    let accepted = aggregate(&all_accept);
    if accepted.outcome != PartOutcome::CharacteristicsAccepted
        || accepted.accepted_count != 2 {
        return Err("ALL_ACCEPT_KNOWN_ANSWER_FAILED".into());
    }

    let one_reject = request(
        vec![req("diameter", true, Some("R-DIA-1")), req("runout", true, Some("R-RUN-1"))],
        vec![
            receipt("diameter", "R-DIA-1", ScalarOutcome::Accept),
            receipt("runout", "R-RUN-1", ScalarOutcome::RejectNonconforming),
        ],
    );
    if aggregate(&one_reject).outcome != PartOutcome::RejectNonconforming {
        return Err("ONE_REJECT_MUST_REJECT_PART".into());
    }

    let one_indeterminate = request(
        vec![req("diameter", true, Some("R-DIA-1")), req("runout", true, Some("R-RUN-1"))],
        vec![
            receipt("diameter", "R-DIA-1", ScalarOutcome::Accept),
            receipt("runout", "R-RUN-1", ScalarOutcome::Indeterminate),
        ],
    );
    if aggregate(&one_indeterminate).outcome != PartOutcome::Indeterminate {
        return Err("ONE_INDETERMINATE_MUST_BLOCK_ACCEPT".into());
    }

    let missing = request(
        vec![req("diameter", true, Some("R-DIA-1")), req("runout", true, Some("R-RUN-1"))],
        vec![receipt("diameter", "R-DIA-1", ScalarOutcome::Accept)],
    );
    if aggregate(&missing).outcome != PartOutcome::Indeterminate {
        return Err("MISSING_MANDATORY_REPORT_MUST_BLOCK_ACCEPT".into());
    }

    let optional_missing = request(
        vec![req("diameter", true, Some("R-DIA-1")), req("cosmetic-mark", false, None)],
        vec![receipt("diameter", "R-DIA-1", ScalarOutcome::Accept)],
    );
    let optional = aggregate(&optional_missing);
    if optional.outcome != PartOutcome::CharacteristicsAccepted
        || optional.optional_missing_count != 1 {
        return Err("OPTIONAL_MISSING_SHOULD_NOT_BLOCK_ACCEPT".into());
    }

    let optional_reject = request(
        vec![req("diameter", true, Some("R-DIA-1")), req("cosmetic-mark", false, Some("R-COS-1"))],
        vec![
            receipt("diameter", "R-DIA-1", ScalarOutcome::Accept),
            receipt("cosmetic-mark", "R-COS-1", ScalarOutcome::RejectNonconforming),
        ],
    );
    if aggregate(&optional_reject).outcome != PartOutcome::CharacteristicsAccepted {
        return Err("OPTIONAL_REJECT_SHOULD_NOT_CHANGE_REQUIRED_SET_DISPOSITION".into());
    }

    let mut extra = missing.clone();
    extra.child_receipts.push(receipt("hardness-extra", "R-EXTRA", ScalarOutcome::Accept));
    let extra_report = aggregate(&extra);
    if extra_report.outcome != PartOutcome::Indeterminate
        || extra_report.extra_receipt_count != 1
        || !extra_report.extra_characteristic_ids.contains(&"hardness-extra".to_string()) {
        return Err("EXTRA_REPORT_MUST_NOT_SUBSTITUTE_FOR_MISSING_REQUIREMENT".into());
    }

    // A valid successor receipt supersedes an earlier receipt and becomes current.
    let mut old = receipt("diameter", "R-DIA-OLD", ScalarOutcome::Indeterminate);
    old.is_current = false;
    old.issued_at_ns = 5_100;
    let mut new = receipt("diameter", "R-DIA-NEW", ScalarOutcome::Accept);
    new.supersedes_receipt_id = Some("R-DIA-OLD".into());
    new.issued_at_ns = 5_200;
    let successor = request(
        vec![req("diameter", true, Some("R-DIA-NEW"))],
        vec![old, new],
    );
    let successor_report = aggregate(&successor);
    if successor_report.outcome != PartOutcome::CharacteristicsAccepted
        || successor_report.characteristic_results[0].historical_receipt_ids != vec!["R-DIA-OLD"] {
        return Err("VALID_SUPERSESSION_CHAIN_FAILED".into());
    }

    Ok(())
}

fn emit_csv() -> Result<(), String> {
    println!("{}", csv_header());
    let scenarios = vec![
        request(
            vec![req("diameter", true, Some("R-DIA-1")), req("runout", true, Some("R-RUN-1"))],
            vec![
                receipt("diameter", "R-DIA-1", ScalarOutcome::Accept),
                receipt("runout", "R-RUN-1", ScalarOutcome::Accept),
            ],
        ),
        request(
            vec![req("diameter", true, Some("R-DIA-1")), req("runout", true, Some("R-RUN-1"))],
            vec![
                receipt("diameter", "R-DIA-1", ScalarOutcome::Accept),
                receipt("runout", "R-RUN-1", ScalarOutcome::RejectNonconforming),
            ],
        ),
        request(
            vec![req("diameter", true, Some("R-DIA-1")), req("runout", true, Some("R-RUN-1"))],
            vec![receipt("diameter", "R-DIA-1", ScalarOutcome::Accept)],
        ),
    ];
    for scenario in scenarios {
        let report = aggregate(&scenario);
        for row in csv_rows(&report) {
            println!("{row}");
        }
    }
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.as_slice() {
        [] => verify().map(|()| println!("MFG_PART_AGGREGATE_OK: exact mandatory-characteristic evidence composition")),
        [arg] if arg == "--verify" => verify().map(|()| println!("MFG_PART_AGGREGATE_OK: exact mandatory-characteristic evidence composition")),
        [arg] if arg == "--csv" => emit_csv(),
        [arg] if arg == "--help" || arg == "-h" => {
            println!("Usage: part-aggregate [--verify|--csv|--help]");
            Ok(())
        }
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
    fn all_required_accept_produces_characteristics_accepted_not_product_release() {
        let r = request(
            vec![req("diameter", true, Some("R1")), req("runout", true, Some("R2"))],
            vec![receipt("diameter", "R1", ScalarOutcome::Accept),
                 receipt("runout", "R2", ScalarOutcome::Accept)],
        );
        let report = aggregate(&r);
        assert_eq!(report.outcome, PartOutcome::CharacteristicsAccepted);
        assert_eq!(report.reason, "ALL_MANDATORY_CHARACTERISTICS_ACCEPTED");
    }

    #[test]
    fn one_valid_mandatory_reject_dominates_unknown_other_characteristics() {
        let r = request(
            vec![req("diameter", true, Some("R1")), req("runout", true, Some("R2"))],
            vec![receipt("diameter", "R1", ScalarOutcome::RejectNonconforming)],
        );
        assert_eq!(aggregate(&r).outcome, PartOutcome::RejectNonconforming);
    }

    #[test]
    fn missing_indeterminate_and_incomplete_child_reports_block_acceptance() {
        for outcome in [ScalarOutcome::Indeterminate, ScalarOutcome::EvidenceIncomplete] {
            let r = request(
                vec![req("diameter", true, Some("R1")), req("runout", true, Some("R2"))],
                vec![receipt("diameter", "R1", ScalarOutcome::Accept),
                     receipt("runout", "R2", outcome)],
            );
            assert_eq!(aggregate(&r).outcome, PartOutcome::Indeterminate);
        }
    }

    #[test]
    fn absent_optional_requirement_does_not_block_acceptance() {
        let r = request(
            vec![req("diameter", true, Some("R1")), req("marking", false, None)],
            vec![receipt("diameter", "R1", ScalarOutcome::Accept)],
        );
        assert_eq!(aggregate(&r).outcome, PartOutcome::CharacteristicsAccepted);
    }

    #[test]
    fn extra_characteristic_is_preserved_but_never_substitutes_for_requirement() {
        let r = request(
            vec![req("diameter", true, Some("R1")), req("runout", true, Some("R2"))],
            vec![receipt("diameter", "R1", ScalarOutcome::Accept),
                 receipt("hardness-extra", "RX", ScalarOutcome::Accept)],
        );
        let report = aggregate(&r);
        assert_eq!(report.outcome, PartOutcome::Indeterminate);
        assert_eq!(report.extra_receipt_count, 1);
        assert_eq!(report.extra_characteristic_ids, vec!["hardness-extra"]);
    }

    #[test]
    fn stale_or_mismatched_scope_child_never_accepts() {
        let mut stale = receipt("diameter", "R1", ScalarOutcome::Accept);
        stale.evidence_state = EvidenceState::Stale;
        let r = request(vec![req("diameter", true, Some("R1"))], vec![stale]);
        assert_eq!(aggregate(&r).outcome, PartOutcome::Indeterminate);

        let mut wrong = receipt("diameter", "R1", ScalarOutcome::Accept);
        wrong.lot_id = "OTHER-LOT".into();
        let r = request(vec![req("diameter", true, Some("R1"))], vec![wrong]);
        assert_eq!(aggregate(&r).outcome, PartOutcome::Indeterminate);
    }

    #[test]
    fn duplicate_current_receipt_and_duplicate_receipt_id_fail_closed() {
        let r = request(
            vec![req("diameter", true, Some("R1"))],
            vec![receipt("diameter", "R1", ScalarOutcome::Accept),
                 receipt("diameter", "R2", ScalarOutcome::Accept)],
        );
        assert_eq!(aggregate(&r).outcome, PartOutcome::Indeterminate);

        let r = request(
            vec![req("diameter", true, Some("R1"))],
            vec![receipt("diameter", "R1", ScalarOutcome::Accept),
                 receipt("diameter", "R1", ScalarOutcome::Accept)],
        );
        assert_eq!(aggregate(&r).outcome, PartOutcome::InvalidManifest);
    }

    #[test]
    fn supersession_chain_requires_a_single_linear_current_successor() {
        let mut old = receipt("diameter", "R-OLD", ScalarOutcome::Indeterminate);
        old.is_current = false;
        let mut new = receipt("diameter", "R-NEW", ScalarOutcome::Accept);
        new.supersedes_receipt_id = Some("R-OLD".into());
        new.issued_at_ns = old.issued_at_ns + 1;
        let r = request(vec![req("diameter", true, Some("R-NEW"))], vec![old.clone(), new]);
        assert_eq!(aggregate(&r).outcome, PartOutcome::CharacteristicsAccepted);

        let disconnected = request(
            vec![req("diameter", true, Some("R-NEW"))],
            vec![old, receipt("diameter", "R-NEW", ScalarOutcome::Accept)],
        );
        assert_eq!(aggregate(&disconnected).outcome, PartOutcome::Indeterminate);
    }

    #[test]
    fn report_is_input_order_invariant_and_deterministic() {
        let r = request(
            vec![req("diameter", true, Some("R1")), req("runout", true, Some("R2"))],
            vec![receipt("runout", "R2", ScalarOutcome::Accept),
                 receipt("diameter", "R1", ScalarOutcome::Accept)],
        );
        let a = aggregate(&r);
        let mut reversed = r.clone();
        reversed.child_receipts.reverse();
        let b = aggregate(&reversed);
        assert_eq!(a.outcome, b.outcome);
        assert_eq!(csv_rows(&a), csv_rows(&b));
    }

    #[test]
    fn aggregate_replay_preserves_every_current_historical_and_extra_source_digest() {
        let mut old = receipt("diameter", "R-OLD", ScalarOutcome::Indeterminate);
        old.is_current = false;
        old.receipt_sha256 = "b".repeat(HASH_LEN);
        let mut current = receipt("diameter", "R-CURRENT", ScalarOutcome::Accept);
        current.supersedes_receipt_id = Some("R-OLD".into());
        current.receipt_sha256 = "c".repeat(HASH_LEN);
        let extra = receipt("unlisted-feature", "R-EXTRA", ScalarOutcome::Accept);
        let r = request(
            vec![req("diameter", true, Some("R-CURRENT"))],
            vec![old, current, extra],
        );
        let report = aggregate(&r);
        let rows = csv_rows(&report).join("\n");
        assert!(rows.contains("R-OLD"));
        assert!(rows.contains("R-CURRENT"));
        assert!(rows.contains("R-EXTRA"));
        assert!(rows.contains(&"b".repeat(HASH_LEN)));
        assert!(rows.contains(&"c".repeat(HASH_LEN)));
        assert!(rows.contains("unlisted-feature"));
        assert_eq!(report.extra_receipt_count, 1);
    }

    #[test]
    fn frozen_verification_campaign_passes() {
        verify().unwrap();
    }
}
