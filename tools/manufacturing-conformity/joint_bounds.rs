//! Dependence-robust part-level conformity bounds from scalar marginal posteriors.
//!
//! Never assumes independence. The only joint-probability claim is the
//! Frechet enclosure implied by the complete required-characteristic set.
//!
//! Compile:
//!   rustc --edition=2024 -D warnings --test tools/manufacturing-conformity/joint_bounds.rs -o /tmp/joint-bounds-tests
//!   /tmp/joint-bounds-tests
//!   rustc --edition=2024 -D warnings tools/manufacturing-conformity/joint_bounds.rs -o /tmp/joint-bounds
//!   /tmp/joint-bounds --verify
//!   /tmp/joint-bounds --csv

const HASH_LEN: usize = 64;
const FLOAT_TOLERANCE: f64 = 64.0 * f64::EPSILON;

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

    fn failure(self, family: &'static str) -> Option<&'static str> {
        match (self, family) {
            (Self::Verified, _) => None,
            (Self::Missing, "posterior") => Some("POSTERIOR_EVIDENCE_MISSING"),
            (Self::Missing, "model_calibration") => Some("MODEL_CALIBRATION_EVIDENCE_MISSING"),
            (Self::Missing, "measurement") => Some("MEASUREMENT_EVIDENCE_MISSING"),
            (Self::Missing, "msa") => Some("MSA_EVIDENCE_MISSING"),
            (Self::Missing, "calibration") => Some("CALIBRATION_EVIDENCE_MISSING"),
            (Self::Missing, "uncertainty") => Some("UNCERTAINTY_EVIDENCE_MISSING"),
            (Self::Missing, "specification") => Some("SPECIFICATION_EVIDENCE_MISSING"),
            (Self::Missing, _) => Some("POLICY_EVIDENCE_MISSING"),
            (Self::Stale, "posterior") => Some("POSTERIOR_EVIDENCE_STALE"),
            (Self::Stale, "model_calibration") => Some("MODEL_CALIBRATION_EVIDENCE_STALE"),
            (Self::Stale, "measurement") => Some("MEASUREMENT_EVIDENCE_STALE"),
            (Self::Stale, "msa") => Some("MSA_EVIDENCE_STALE"),
            (Self::Stale, "calibration") => Some("CALIBRATION_EVIDENCE_STALE"),
            (Self::Stale, "uncertainty") => Some("UNCERTAINTY_EVIDENCE_STALE"),
            (Self::Stale, "specification") => Some("SPECIFICATION_EVIDENCE_STALE"),
            (Self::Stale, _) => Some("POLICY_EVIDENCE_STALE"),
            (Self::Mismatched, "posterior") => Some("POSTERIOR_EVIDENCE_MISMATCHED"),
            (Self::Mismatched, "model_calibration") => Some("MODEL_CALIBRATION_EVIDENCE_MISMATCHED"),
            (Self::Mismatched, "measurement") => Some("MEASUREMENT_EVIDENCE_MISMATCHED"),
            (Self::Mismatched, "msa") => Some("MSA_EVIDENCE_MISMATCHED"),
            (Self::Mismatched, "calibration") => Some("CALIBRATION_EVIDENCE_MISMATCHED"),
            (Self::Mismatched, "uncertainty") => Some("UNCERTAINTY_EVIDENCE_MISMATCHED"),
            (Self::Mismatched, "specification") => Some("SPECIFICATION_EVIDENCE_MISMATCHED"),
            (Self::Mismatched, _) => Some("POLICY_EVIDENCE_MISMATCHED"),
            (Self::Revoked, "posterior") => Some("POSTERIOR_EVIDENCE_REVOKED"),
            (Self::Revoked, "model_calibration") => Some("MODEL_CALIBRATION_EVIDENCE_REVOKED"),
            (Self::Revoked, "measurement") => Some("MEASUREMENT_EVIDENCE_REVOKED"),
            (Self::Revoked, "msa") => Some("MSA_EVIDENCE_REVOKED"),
            (Self::Revoked, "calibration") => Some("CALIBRATION_EVIDENCE_REVOKED"),
            (Self::Revoked, "uncertainty") => Some("UNCERTAINTY_EVIDENCE_REVOKED"),
            (Self::Revoked, "specification") => Some("SPECIFICATION_EVIDENCE_REVOKED"),
            (Self::Revoked, _) => Some("POLICY_EVIDENCE_REVOKED"),
            (Self::NotQualified, "posterior") => Some("POSTERIOR_NOT_QUALIFIED"),
            (Self::NotQualified, "model_calibration") => Some("MODEL_CALIBRATION_NOT_QUALIFIED"),
            (Self::NotQualified, "measurement") => Some("MEASUREMENT_NOT_QUALIFIED"),
            (Self::NotQualified, "msa") => Some("MSA_NOT_QUALIFIED"),
            (Self::NotQualified, "calibration") => Some("CALIBRATION_NOT_QUALIFIED"),
            (Self::NotQualified, "uncertainty") => Some("UNCERTAINTY_BUDGET_NOT_QUALIFIED"),
            (Self::NotQualified, "specification") => Some("SPECIFICATION_NOT_QUALIFIED"),
            (Self::NotQualified, _) => Some("POLICY_NOT_QUALIFIED"),
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct EvidenceChecks {
    posterior: EvidenceState,
    model_calibration: EvidenceState,
    measurement: EvidenceState,
    msa: EvidenceState,
    calibration: EvidenceState,
    uncertainty: EvidenceState,
    specification: EvidenceState,
}

impl EvidenceChecks {
    fn first_failure(self) -> Option<&'static str> {
        [
            ("posterior", self.posterior),
            ("model_calibration", self.model_calibration),
            ("measurement", self.measurement),
            ("msa", self.msa),
            ("calibration", self.calibration),
            ("uncertainty", self.uncertainty),
            ("specification", self.specification),
        ]
        .into_iter()
        .find_map(|(family, state)| state.failure(family))
    }
}

#[derive(Clone, Debug)]
struct Manifest {
    id: String,
    revision: String,
    part_id: String,
    lot_id: String,
    operation_id: String,
    product_family: String,
    decision_context_id: String,
    decision_time_ns: i64,
    required_characteristics: Vec<String>,
    manifest_sha256: String,
}

#[derive(Clone, Debug)]
struct Marginal {
    characteristic_id: String,
    part_id: String,
    lot_id: String,
    operation_id: String,
    decision_context_id: String,
    decision_time_ns: i64,
    measurand_id: String,
    unit: String,
    specification_id: String,
    specification_revision: String,
    posterior_id: String,
    posterior_model_revision: String,
    probability_lower: f64,
    probability_upper: f64,
    valid_from_ns: i64,
    valid_until_ns: i64,
    posterior_sha256: String,
    model_calibration_sha256: String,
    measurement_sha256: String,
    msa_sha256: String,
    calibration_sha256: String,
    uncertainty_sha256: String,
    specification_sha256: String,
    evidence: EvidenceChecks,
}

#[derive(Clone, Debug)]
struct Policy {
    id: String,
    revision: String,
    product_family: String,
    decision_context_id: String,
    effective_from_ns: i64,
    effective_until_ns: i64,
    accept_threshold: f64,
    reject_threshold: f64,
    approved: bool,
    source_sha256: String,
    evidence: EvidenceState,
}

#[derive(Clone, Debug)]
struct Request {
    manifest: Manifest,
    marginals: Vec<Marginal>,
    policy: Policy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Outcome {
    Accept,
    RejectNonconforming,
    Indeterminate,
    EvidenceIncomplete,
    InvalidInput,
}

impl Outcome {
    fn code(self) -> &'static str {
        match self {
            Self::Accept => "ACCEPT",
            Self::RejectNonconforming => "REJECT_NONCONFORMING",
            Self::Indeterminate => "INDETERMINATE",
            Self::EvidenceIncomplete => "EVIDENCE_INCOMPLETE",
            Self::InvalidInput => "INVALID_INPUT",
        }
    }
}

#[derive(Clone, Debug)]
struct Report {
    manifest_id: String,
    manifest_revision: String,
    manifest_sha256: String,
    part_id: String,
    lot_id: String,
    operation_id: String,
    product_family: String,
    policy_id: String,
    policy_revision: String,
    policy_sha256: String,
    decision_context_id: String,
    decision_time_ns: i64,
    required_count: usize,
    observed_count: usize,
    joint_lower: Option<f64>,
    joint_upper: Option<f64>,
    accept_threshold: Option<f64>,
    reject_threshold: Option<f64>,
    consumer_risk_upper: Option<f64>,
    producer_risk_upper: Option<f64>,
    outcome: Outcome,
    reason: &'static str,
    marginal_rows: Vec<Marginal>,
}

fn nonempty(s: &str) -> bool {
    !s.trim().is_empty() && !s.contains(',') && !s.contains('|') && !s.contains('\n')
}

fn valid_sha(s: &str) -> bool {
    s.len() == HASH_LEN && s.bytes().all(|b| b.is_ascii_hexdigit())
}

fn finite_option(value: f64) -> Option<f64> {
    value.is_finite().then_some(value)
}

fn invalid(request: &Request, reason: &'static str) -> Report {
    Report {
        manifest_id: request.manifest.id.clone(),
        manifest_revision: request.manifest.revision.clone(),
        manifest_sha256: request.manifest.manifest_sha256.clone(),
        part_id: request.manifest.part_id.clone(),
        lot_id: request.manifest.lot_id.clone(),
        operation_id: request.manifest.operation_id.clone(),
        product_family: request.manifest.product_family.clone(),
        policy_id: request.policy.id.clone(),
        policy_revision: request.policy.revision.clone(),
        policy_sha256: request.policy.source_sha256.clone(),
        decision_context_id: request.manifest.decision_context_id.clone(),
        decision_time_ns: request.manifest.decision_time_ns,
        required_count: request.manifest.required_characteristics.len(),
        observed_count: request.marginals.len(),
        joint_lower: None,
        joint_upper: None,
        accept_threshold: finite_option(request.policy.accept_threshold),
        reject_threshold: finite_option(request.policy.reject_threshold),
        consumer_risk_upper: None,
        producer_risk_upper: None,
        outcome: Outcome::InvalidInput,
        reason,
        marginal_rows: request.marginals.clone(),
    }
}

fn evidence_incomplete(request: &Request, reason: &'static str) -> Report {
    let mut report = invalid(request, reason);
    report.outcome = Outcome::EvidenceIncomplete;
    report
}

fn validate_identifiers(request: &Request) -> Result<(), &'static str> {
    let m = &request.manifest;
    let p = &request.policy;
    for id in [
        m.id.as_str(), m.revision.as_str(), m.part_id.as_str(), m.lot_id.as_str(),
        m.operation_id.as_str(), m.product_family.as_str(), m.decision_context_id.as_str(),
        p.id.as_str(), p.revision.as_str(), p.product_family.as_str(),
        p.decision_context_id.as_str(),
    ] {
        if !nonempty(id) {
            return Err("REQUIRED_IDENTITY_MISSING_OR_UNSAFE");
        }
    }
    if !valid_sha(&m.manifest_sha256) || !valid_sha(&p.source_sha256) {
        return Err("MANIFEST_OR_POLICY_DIGEST_INVALID");
    }
    for id in &m.required_characteristics {
        if !nonempty(id) {
            return Err("REQUIRED_CHARACTERISTIC_ID_INVALID");
        }
    }
    Ok(())
}

fn validate_manifest_and_marginals(request: &Request) -> Result<(), &'static str> {
    let manifest = &request.manifest;
    if manifest.required_characteristics.len() < 2 {
        return Err("AT_LEAST_TWO_REQUIRED_CHARACTERISTICS");
    }
    let mut expected = manifest.required_characteristics.clone();
    expected.sort();
    if expected.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err("DUPLICATE_REQUIRED_CHARACTERISTIC_ID");
    }
    if request.marginals.len() != expected.len() {
        return Err("REQUIRED_CHARACTERISTIC_SET_INCOMPLETE_OR_EXTRANEOUS");
    }

    let mut observed: Vec<&str> = request.marginals.iter()
        .map(|marginal| marginal.characteristic_id.as_str())
        .collect();
    observed.sort_unstable();
    if observed.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err("DUPLICATE_MARGINAL_CHARACTERISTIC_ID");
    }
    let expected_refs: Vec<&str> = expected.iter().map(String::as_str).collect();
    if observed != expected_refs {
        return Err("REQUIRED_CHARACTERISTIC_SET_MISMATCH");
    }

    for marginal in &request.marginals {
        for id in [
            marginal.characteristic_id.as_str(), marginal.part_id.as_str(),
            marginal.lot_id.as_str(), marginal.operation_id.as_str(),
            marginal.decision_context_id.as_str(), marginal.measurand_id.as_str(),
            marginal.unit.as_str(), marginal.specification_id.as_str(),
            marginal.specification_revision.as_str(), marginal.posterior_id.as_str(),
            marginal.posterior_model_revision.as_str(),
        ] {
            if !nonempty(id) {
                return Err("MARGINAL_IDENTITY_MISSING_OR_UNSAFE");
            }
        }
        if !marginal.probability_lower.is_finite()
            || !marginal.probability_upper.is_finite()
            || marginal.probability_lower < 0.0
            || marginal.probability_upper > 1.0
            || marginal.probability_lower > marginal.probability_upper
        {
            return Err("MARGINAL_PROBABILITY_ENCLOSURE_INVALID");
        }
        if marginal.part_id != manifest.part_id
            || marginal.lot_id != manifest.lot_id
            || marginal.operation_id != manifest.operation_id
            || marginal.decision_context_id != manifest.decision_context_id
            || marginal.decision_time_ns != manifest.decision_time_ns {
            return Err("MARGINAL_SCOPE_MISMATCH");
        }
        if marginal.valid_from_ns > marginal.valid_until_ns
            || manifest.decision_time_ns < marginal.valid_from_ns
            || manifest.decision_time_ns > marginal.valid_until_ns
        {
            return Err("POSTERIOR_NOT_VALID_AT_DECISION_TIME");
        }
        for digest in [
            marginal.posterior_sha256.as_str(),
            marginal.model_calibration_sha256.as_str(),
            marginal.measurement_sha256.as_str(),
            marginal.msa_sha256.as_str(),
            marginal.calibration_sha256.as_str(),
            marginal.uncertainty_sha256.as_str(),
            marginal.specification_sha256.as_str(),
        ] {
            if !valid_sha(digest) {
                return Err("MARGINAL_EVIDENCE_DIGEST_INVALID");
            }
        }
        if let Some(reason) = marginal.evidence.first_failure() {
            return Err(reason);
        }
    }
    Ok(())
}

fn validate_policy(request: &Request) -> Result<(), &'static str> {
    let p = &request.policy;
    if p.evidence != EvidenceState::Verified {
        return Err(p.evidence.failure("policy").unwrap_or("POLICY_NOT_VERIFIED"));
    }
    if !p.approved {
        return Err("POLICY_NOT_APPROVED");
    }
    if !p.accept_threshold.is_finite()
        || !p.reject_threshold.is_finite()
        || p.reject_threshold < 0.0
        || p.accept_threshold > 1.0
        || p.reject_threshold >= p.accept_threshold
    {
        return Err("POLICY_THRESHOLDS_INVALID");
    }
    if p.product_family != request.manifest.product_family
        || p.decision_context_id != request.manifest.decision_context_id
    {
        return Err("POLICY_SCOPE_MISMATCH");
    }
    if p.effective_from_ns > p.effective_until_ns
        || request.manifest.decision_time_ns < p.effective_from_ns
        || request.manifest.decision_time_ns > p.effective_until_ns
    {
        return Err("POLICY_NOT_EFFECTIVE_AT_DECISION_TIME");
    }
    Ok(())
}

fn frechet_bounds(marginals: &[(f64, f64)]) -> Result<(f64, f64), &'static str> {
    if marginals.len() < 2 {
        return Err("AT_LEAST_TWO_MARGINALS_REQUIRED");
    }
    let mut lower_sum = 0.0_f64;
    let mut upper_min = 1.0_f64;
    for (lower, upper) in marginals {
        if !lower.is_finite() || !upper.is_finite()
            || *lower < 0.0 || *upper > 1.0 || *lower > *upper {
            return Err("MARGINAL_PROBABILITY_ENCLOSURE_INVALID");
        }
        lower_sum += *lower;
        upper_min = upper_min.min(*upper);
    }
    if !lower_sum.is_finite() {
        return Err("JOINT_PROBABILITY_SUM_OVERFLOW");
    }
    let raw_lower = (lower_sum - (marginals.len() as f64 - 1.0)).max(0.0);
    let lower = raw_lower.min(upper_min).clamp(0.0, 1.0);
    let upper = upper_min.clamp(0.0, 1.0);
    if lower > upper + FLOAT_TOLERANCE {
        return Err("FRECHET_BOUNDS_INCONSISTENT");
    }
    Ok((lower, upper))
}

fn classify(request: &Request) -> Report {
    if let Err(reason) = validate_identifiers(request) {
        return invalid(request, reason);
    }
    if let Err(reason) = validate_manifest_and_marginals(request) {
        if reason.contains("EVIDENCE") || reason.contains("QUALIFIED")
            || reason == "POSTERIOR_NOT_VALID_AT_DECISION_TIME"
            || reason == "MARGINAL_SCOPE_MISMATCH"
            || reason == "REQUIRED_CHARACTERISTIC_SET_INCOMPLETE_OR_EXTRANEOUS"
            || reason == "REQUIRED_CHARACTERISTIC_SET_MISMATCH" {
            return evidence_incomplete(request, reason);
        }
        return invalid(request, reason);
    }
    if let Err(reason) = validate_policy(request) {
        if reason.contains("EVIDENCE") || reason.contains("QUALIFIED")
            || reason.contains("APPROVED") || reason.contains("EFFECTIVE") {
            return evidence_incomplete(request, reason);
        }
        return invalid(request, reason);
    }

    let bounds: Vec<(f64, f64)> = request.marginals.iter()
        .map(|m| (m.probability_lower, m.probability_upper))
        .collect();
    let (joint_lower, joint_upper) = match frechet_bounds(&bounds) {
        Ok(bounds) => bounds,
        Err(reason) => return invalid(request, reason),
    };

    let (outcome, reason) = if joint_lower >= request.policy.accept_threshold {
        (Outcome::Accept, "FRECHET_LOWER_BOUND_MEETS_ACCEPT_THRESHOLD")
    } else if joint_upper <= request.policy.reject_threshold {
        (Outcome::RejectNonconforming, "FRECHET_UPPER_BOUND_BELOW_REJECT_THRESHOLD")
    } else {
        (Outcome::Indeterminate, "FRECHET_ENCLOSURE_CROSSES_DECISION_THRESHOLDS")
    };

    Report {
        manifest_id: request.manifest.id.clone(),
        manifest_revision: request.manifest.revision.clone(),
        manifest_sha256: request.manifest.manifest_sha256.clone(),
        part_id: request.manifest.part_id.clone(),
        lot_id: request.manifest.lot_id.clone(),
        operation_id: request.manifest.operation_id.clone(),
        product_family: request.manifest.product_family.clone(),
        policy_id: request.policy.id.clone(),
        policy_revision: request.policy.revision.clone(),
        policy_sha256: request.policy.source_sha256.clone(),
        decision_context_id: request.manifest.decision_context_id.clone(),
        decision_time_ns: request.manifest.decision_time_ns,
        required_count: request.manifest.required_characteristics.len(),
        observed_count: request.marginals.len(),
        joint_lower: Some(joint_lower),
        joint_upper: Some(joint_upper),
        accept_threshold: Some(request.policy.accept_threshold),
        reject_threshold: Some(request.policy.reject_threshold),
        consumer_risk_upper: (outcome == Outcome::Accept).then_some(1.0 - joint_lower),
        producer_risk_upper: (outcome == Outcome::RejectNonconforming).then_some(joint_upper),
        outcome,
        reason,
        marginal_rows: request.marginals.clone(),
    }
}

fn report_csv_header() -> &'static str {
    "manifest_id,manifest_revision,manifest_sha256,manifest_part_id,manifest_lot_id,manifest_operation_id,manifest_product_family,manifest_decision_context_id,manifest_decision_time_ns,policy_id,policy_revision,policy_sha256,required_count,observed_count,joint_lower,joint_upper,accept_threshold,reject_threshold,consumer_risk_upper,producer_risk_upper,outcome,reason,characteristic_id,marginal_part_id,marginal_lot_id,marginal_operation_id,marginal_decision_context_id,marginal_decision_time_ns,measurand_id,unit,specification_id,specification_revision,posterior_id,posterior_model_revision,marginal_lower,marginal_upper,valid_from_ns,valid_until_ns,posterior_sha256,model_calibration_sha256,measurement_sha256,msa_sha256,calibration_sha256,uncertainty_sha256,specification_sha256,posterior_evidence,model_calibration_evidence,measurement_evidence,msa_evidence,calibration_evidence,uncertainty_evidence,specification_evidence"
}

fn csv_row(report: &Report, marginal: &Marginal) -> String {
    let f = |value: Option<f64>| value.map(|v| format!("{v:.12e}")).unwrap_or_default();
    vec![
        report.manifest_id.clone(),
        report.manifest_revision.clone(),
        report.manifest_sha256.clone(),
        report.part_id.clone(),
        report.lot_id.clone(),
        report.operation_id.clone(),
        report.product_family.clone(),
        report.decision_context_id.clone(),
        report.decision_time_ns.to_string(),
        report.policy_id.clone(),
        report.policy_revision.clone(),
        report.policy_sha256.clone(),
        report.required_count.to_string(),
        report.observed_count.to_string(),
        f(report.joint_lower),
        f(report.joint_upper),
        f(report.accept_threshold),
        f(report.reject_threshold),
        f(report.consumer_risk_upper),
        f(report.producer_risk_upper),
        report.outcome.code().to_string(),
        report.reason.to_string(),
        marginal.characteristic_id.clone(),
        marginal.part_id.clone(),
        marginal.lot_id.clone(),
        marginal.operation_id.clone(),
        marginal.decision_context_id.clone(),
        marginal.decision_time_ns.to_string(),
        marginal.measurand_id.clone(),
        marginal.unit.clone(),
        marginal.specification_id.clone(),
        marginal.specification_revision.clone(),
        marginal.posterior_id.clone(),
        marginal.posterior_model_revision.clone(),
        f(Some(marginal.probability_lower)),
        f(Some(marginal.probability_upper)),
        marginal.valid_from_ns.to_string(),
        marginal.valid_until_ns.to_string(),
        marginal.posterior_sha256.clone(),
        marginal.model_calibration_sha256.clone(),
        marginal.measurement_sha256.clone(),
        marginal.msa_sha256.clone(),
        marginal.calibration_sha256.clone(),
        marginal.uncertainty_sha256.clone(),
        marginal.specification_sha256.clone(),
        marginal.evidence.posterior.code().to_string(),
        marginal.evidence.model_calibration.code().to_string(),
        marginal.evidence.measurement.code().to_string(),
        marginal.evidence.msa.code().to_string(),
        marginal.evidence.calibration.code().to_string(),
        marginal.evidence.uncertainty.code().to_string(),
        marginal.evidence.specification.code().to_string(),
    ].join(",")
}

fn fixture_marginal(id: &str, lower: f64, upper: f64) -> Marginal {
    Marginal {
        characteristic_id: id.into(),
        part_id: "PART-0001".into(),
        lot_id: "LOT-0001".into(),
        operation_id: "FINAL-INSPECTION".into(),
        decision_context_id: "COU-SHAFT-RELEASE".into(),
        decision_time_ns: 5_000,
        measurand_id: format!("measurand-{id}"),
        unit: "mm".into(),
        specification_id: format!("spec-{id}"),
        specification_revision: "REV-1".into(),
        posterior_id: format!("posterior-{id}"),
        posterior_model_revision: "MODEL-4".into(),
        probability_lower: lower,
        probability_upper: upper,
        valid_from_ns: 1_000,
        valid_until_ns: 9_000,
        posterior_sha256: "a".repeat(HASH_LEN),
        model_calibration_sha256: "b".repeat(HASH_LEN),
        measurement_sha256: "c".repeat(HASH_LEN),
        msa_sha256: "d".repeat(HASH_LEN),
        calibration_sha256: "e".repeat(HASH_LEN),
        uncertainty_sha256: "f".repeat(HASH_LEN),
        specification_sha256: "1".repeat(HASH_LEN),
        evidence: EvidenceChecks {
            posterior: EvidenceState::Verified,
            model_calibration: EvidenceState::Verified,
            measurement: EvidenceState::Verified,
            msa: EvidenceState::Verified,
            calibration: EvidenceState::Verified,
            uncertainty: EvidenceState::Verified,
            specification: EvidenceState::Verified,
        },
    }
}

fn request_owned(marginals: Vec<Marginal>, required: Vec<String>) -> Request {
    Request {
        manifest: Manifest {
            id: "MANIFEST-PART-1".into(),
            revision: "MANIFEST-REV-3".into(),
            part_id: "PART-0001".into(),
            lot_id: "LOT-0001".into(),
            operation_id: "FINAL-INSPECTION".into(),
            product_family: "SHAFT-ASSEMBLY".into(),
            decision_context_id: "COU-SHAFT-RELEASE".into(),
            decision_time_ns: 5_000,
            required_characteristics: required,
            manifest_sha256: "2".repeat(HASH_LEN),
        },
        marginals,
        policy: Policy {
            id: "POLICY-PART-CONFORMITY".into(),
            revision: "POLICY-2".into(),
            product_family: "SHAFT-ASSEMBLY".into(),
            decision_context_id: "COU-SHAFT-RELEASE".into(),
            effective_from_ns: 1_000,
            effective_until_ns: 10_000,
            accept_threshold: 0.95,
            reject_threshold: 0.60,
            approved: true,
            source_sha256: "3".repeat(HASH_LEN),
            evidence: EvidenceState::Verified,
        },
    }
}

fn request(marginals: Vec<Marginal>, required: Vec<&str>) -> Request {
    request_owned(marginals, required.into_iter().map(str::to_string).collect())
}

fn emit_csv() -> Result<(), String> {
    println!("{}", report_csv_header());
    let scenarios = vec![
        ("two_equal_090", request(vec![
            fixture_marginal("diameter", 0.90, 0.90),
            fixture_marginal("runout", 0.90, 0.90),
        ], vec!["diameter", "runout"])),
        ("five_equal_090", request_owned(
            (0..5).map(|i| fixture_marginal(&format!("char-{i}"), 0.90, 0.90)).collect(),
            (0..5).map(|i| format!("char-{i}")).collect())),
        ("twenty_equal_099", request_owned(
            (0..20).map(|i| fixture_marginal(&format!("char-{i}"), 0.99, 0.99)).collect(),
            (0..20).map(|i| format!("char-{i}")).collect())),
    ];
    for (_, req) in scenarios {
        let report = classify(&req);
        for marginal in &report.marginal_rows {
            println!("{}", csv_row(&report, marginal));
        }
    }
    Ok(())
}

fn verify() -> Result<(), String> {
    let two = request(vec![
        fixture_marginal("diameter", 0.90, 0.90),
        fixture_marginal("runout", 0.90, 0.90),
    ], vec!["diameter", "runout"]);
    let r = classify(&two);
    if r.outcome != Outcome::Indeterminate
        || (r.joint_lower.unwrap() - 0.80).abs() > FLOAT_TOLERANCE
        || (r.joint_upper.unwrap() - 0.90).abs() > FLOAT_TOLERANCE {
        return Err("TWO_CHARACTERISTIC_KNOWN_ANSWER_FAILED".into());
    }

    let five_marginals: Vec<Marginal> = (0..5)
        .map(|i| fixture_marginal(&format!("char-{i}"), 0.90, 0.90)).collect();
    let five_ids: Vec<String> = (0..5).map(|i| format!("char-{i}")).collect();
    let five = request(five_marginals, five_ids.iter().map(String::as_str).collect());
    let r = classify(&five);
    if (r.joint_lower.unwrap() - 0.50).abs() > FLOAT_TOLERANCE
        || (r.joint_upper.unwrap() - 0.90).abs() > FLOAT_TOLERANCE
        || r.outcome != Outcome::Indeterminate {
        return Err("FIVE_CHARACTERISTIC_INDEPENDENCE_NEGATIVE_CONTROL_FAILED".into());
    }
    // Product of 0.9 marginals would be 0.59049 and would cross this
    // policy's 0.60 rejection threshold. It is not a valid bound here.
    let independence_product = 0.9_f64.powi(5);
    if independence_product >= r.joint_upper.unwrap()
        || independence_product <= r.joint_lower.unwrap() {
        return Err("INDEPENDENCE_DIAGNOSTIC_NOT_SEPARATED_FROM_FRECHET_BOUNDS".into());
    }

    let twenty_marginals: Vec<Marginal> = (0..20)
        .map(|i| fixture_marginal(&format!("char-{i}"), 0.99, 0.99)).collect();
    let twenty_ids: Vec<String> = (0..20).map(|i| format!("char-{i}")).collect();
    let twenty = request(twenty_marginals, twenty_ids.iter().map(String::as_str).collect());
    let r = classify(&twenty);
    if (r.joint_lower.unwrap() - 0.80).abs() > 1.0e-12
        || (r.joint_upper.unwrap() - 0.99).abs() > FLOAT_TOLERANCE {
        return Err("TWENTY_CHARACTERISTIC_KNOWN_ANSWER_FAILED".into());
    }

    let base_five_bounds = frechet_bounds(&[(0.90, 0.90); 5]).unwrap();
    // An unknown characteristic must not be silently dropped.
    let mut with_unknown = five.clone();
    with_unknown.manifest.required_characteristics.push("unknown".into());
    let unknown = fixture_marginal("unknown", 0.0, 1.0);
    with_unknown.marginals.push(unknown);
    let r = classify(&with_unknown);
    if (r.joint_lower.unwrap() - 0.0).abs() > FLOAT_TOLERANCE
        || (r.joint_upper.unwrap() - 0.90).abs() > FLOAT_TOLERANCE {
        return Err("UNKNOWN_CHARACTERISTIC_DID_NOT_WIDEN_BOUND".into());
    }

    // A guaranteed-conforming extra characteristic [1,1] leaves bounds unchanged.
    let mut add_certain = five.clone();
    add_certain.manifest.required_characteristics.push("certain".into());
    add_certain.marginals.push(fixture_marginal("certain", 1.0, 1.0));
    let certain = classify(&add_certain);
    if (certain.joint_lower.unwrap() - 0.50).abs() > FLOAT_TOLERANCE
        || (certain.joint_upper.unwrap() - 0.90).abs() > FLOAT_TOLERANCE {
        return Err("CERTAIN_CHARACTERISTIC_DID_NOT_PRESERVE_BOUNDS".into());
    }

    // Adding a required marginal cannot increase either Frechet bound.
    let base_lower = base_five_bounds.0;
    let base_upper = base_five_bounds.1;
    let more = classify(&add_certain);
    if more.joint_lower.unwrap() > base_lower + FLOAT_TOLERANCE
        || more.joint_upper.unwrap() > base_upper + FLOAT_TOLERANCE {
        return Err("ADDING_CHARACTERISTIC_INCREASED_JOINT_BOUND".into());
    }

    // Permutation of input order must not affect the result.
    let mut permuted = five.clone();
    permuted.marginals.reverse();
    let rp = classify(&permuted);
    if (rp.joint_lower.unwrap() - 0.50).abs() > FLOAT_TOLERANCE
        || (rp.joint_upper.unwrap() - 0.90).abs() > FLOAT_TOLERANCE {
        return Err("CHARACTERISTIC_PERMUTATION_INVARIANCE_FAILED".into());
    }

    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.as_slice() {
        [] => verify().map(|()| println!("MFG_JOINT_BOUNDS_OK: dependence-robust Frechet enclosure")),
        [arg] if arg == "--verify" => verify().map(|()| println!("MFG_JOINT_BOUNDS_OK: dependence-robust Frechet enclosure")),
        [arg] if arg == "--csv" => emit_csv(),
        [arg] if arg == "--help" || arg == "-h" => {
            println!("Usage: joint-bounds [--verify|--csv|--help]");
            Ok(())
        }
        _ => Err("expected --verify, --csv or --help".into()),
    };
    if let Err(error) = result {
        eprintln!("MFG_JOINT_BOUNDS_ERROR: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn extremal_atom_masses(marginals: &[f64], lower_bound: bool) -> Vec<f64> {
        let n = marginals.len();
        assert!((2..=8).contains(&n));
        let mut cuts = vec![0.0, 1.0];
        if lower_bound {
            let mut offset = 0.0_f64;
            for probability in marginals {
                let failure = 1.0 - probability;
                cuts.push(offset.rem_euclid(1.0));
                cuts.push((offset + failure).rem_euclid(1.0));
                offset += failure;
            }
        } else {
            cuts.extend(marginals.iter().copied());
        }
        cuts.sort_by(f64::total_cmp);
        cuts.dedup_by(|a, b| (*a - *b).abs() <= 8.0 * f64::EPSILON);

        let mut atoms = vec![0.0; 1_usize << n];
        for pair in cuts.windows(2) {
            let left = pair[0];
            let right = pair[1];
            if right <= left { continue; }
            let midpoint = 0.5 * (left + right);
            let mut mask = 0_usize;
            if lower_bound {
                let mut offset = 0.0_f64;
                for (i, probability) in marginals.iter().enumerate() {
                    let failure = 1.0 - probability;
                    let start = offset.rem_euclid(1.0);
                    let end = (offset + failure).rem_euclid(1.0);
                    let in_failure = if failure >= 1.0 {
                        true
                    } else if failure <= 0.0 {
                        false
                    } else if start < end {
                        midpoint >= start && midpoint < end
                    } else {
                        midpoint >= start || midpoint < end
                    };
                    if !in_failure { mask |= 1_usize << i; }
                    offset += failure;
                }
            } else {
                for (i, probability) in marginals.iter().enumerate() {
                    if midpoint < *probability { mask |= 1_usize << i; }
                }
            }
            atoms[mask] += right - left;
        }
        atoms
    }

    fn verify_atom_distribution(atoms: &[f64], marginals: &[f64], expected_joint: f64) {
        let n = marginals.len();
        assert_eq!(atoms.len(), 1_usize << n);
        assert!((atoms.iter().sum::<f64>() - 1.0).abs() < 2.0e-12);
        for (i, expected) in marginals.iter().enumerate() {
            let observed = atoms.iter().enumerate()
                .filter(|(mask, _)| (mask & (1_usize << i)) != 0)
                .map(|(_, mass)| *mass)
                .sum::<f64>();
            assert!((observed - expected).abs() < 2.0e-12);
        }
        let all_success_atom = atoms[(1_usize << n) - 1];
        assert!((all_success_atom - expected_joint).abs() < 2.0e-12);
    }

    #[test]
    fn two_characteristic_known_answer_is_not_independence_product() {
        let result = frechet_bounds(&[(0.9, 0.9), (0.9, 0.9)]).unwrap();
        assert!((result.0 - 0.8).abs() < FLOAT_TOLERANCE);
        assert!((result.1 - 0.9).abs() < FLOAT_TOLERANCE);
        assert!((0.9_f64 * 0.9_f64 - 0.81).abs() < FLOAT_TOLERANCE);
        assert!((result.0 - 0.81).abs() > 0.01);
    }

    #[test]
    fn five_characteristics_expose_false_rejection_from_independence() {
        let r = classify(&request((0..5).map(|i|
            fixture_marginal(&format!("c{i}"), 0.9, 0.9)).collect(),
            (0..5).map(|i| format!("c{i}")).collect::<Vec<_>>().iter().map(String::as_str).collect()));
        assert_eq!(r.outcome, Outcome::Indeterminate);
        assert!((r.joint_lower.unwrap() - 0.5).abs() < FLOAT_TOLERANCE);
        assert!((r.joint_upper.unwrap() - 0.9).abs() < FLOAT_TOLERANCE);
        assert!(0.9_f64.powi(5) < 0.60);
    }

    #[test]
    fn twenty_characteristic_known_answer_is_080_to_099() {
        let b = frechet_bounds(&vec![(0.99, 0.99); 20]).unwrap();
        assert!((b.0 - 0.80).abs() < 1.0e-12);
        assert!((b.1 - 0.99).abs() < FLOAT_TOLERANCE);
    }

    #[test]
    fn adding_characteristics_never_tightens_the_robust_enclosure() {
        for base_n in 2..12 {
            for (low, high) in [(0.70, 0.90), (0.90, 0.90), (0.99, 1.0), (0.0, 1.0)] {
                let base = vec![(low, high); base_n];
                let before = frechet_bounds(&base).unwrap();
                let mut extended = base;
                extended.push((0.6, 0.95));
                let after = frechet_bounds(&extended).unwrap();
                assert!(after.0 <= before.0 + FLOAT_TOLERANCE);
                assert!(after.1 <= before.1 + FLOAT_TOLERANCE);
            }
        }
    }

    #[test]
    fn required_characteristic_manifest_is_exact_and_deduplicated() {
        let good = request(vec![
            fixture_marginal("diameter", 0.9, 0.95),
            fixture_marginal("runout", 0.8, 0.9),
        ], vec!["diameter", "runout"]);
        assert_eq!(classify(&good).outcome, Outcome::Indeterminate);

        let missing = request(vec![fixture_marginal("diameter", 0.9, 0.95)],
            vec!["diameter", "runout"]);
        assert_eq!(classify(&missing).outcome, Outcome::EvidenceIncomplete);

        let duplicate = request(vec![
            fixture_marginal("diameter", 0.9, 0.95),
            fixture_marginal("diameter", 0.8, 0.9),
        ], vec!["diameter", "diameter"]);
        assert_eq!(classify(&duplicate).outcome, Outcome::InvalidInput);
    }

    #[test]
    fn stale_wrong_scope_or_uncalibrated_posterior_is_not_admitted() {
        let mut stale = request(vec![
            fixture_marginal("diameter", 0.9, 0.95),
            fixture_marginal("runout", 0.8, 0.9),
        ], vec!["diameter", "runout"]);
        stale.marginals[1].evidence.model_calibration = EvidenceState::Stale;
        assert_eq!(classify(&stale).outcome, Outcome::EvidenceIncomplete);

        let mut policy = request(vec![
            fixture_marginal("diameter", 0.9, 0.95),
            fixture_marginal("runout", 0.8, 0.9),
        ], vec!["diameter", "runout"]);
        policy.policy.approved = false;
        assert_eq!(classify(&policy).outcome, Outcome::EvidenceIncomplete);

        let mut wrong_scope = request(vec![
            fixture_marginal("diameter", 0.9, 0.95),
            fixture_marginal("runout", 0.8, 0.9),
        ], vec!["diameter", "runout"]);
        wrong_scope.policy.product_family = "OTHER".into();
        assert_eq!(classify(&wrong_scope).outcome, Outcome::InvalidInput);
    }

    #[test]
    fn joint_csv_header_and_rows_have_identical_fixed_width() {
        let r = request(vec![
            fixture_marginal("diameter", 0.9, 0.95),
            fixture_marginal("runout", 0.8, 0.9),
        ], vec!["diameter", "runout"]);
        let report = classify(&r);
        let header_width = report_csv_header().split(',').count();
        assert_eq!(header_width, 52);
        for marginal in &report.marginal_rows {
            assert_eq!(csv_row(&report, marginal).split(',').count(), header_width);
        }
    }

    #[test]
    fn every_marginal_is_bound_to_the_exact_part_lot_operation_and_decision_time() {
        for mutate in 0..5 {
            let mut request = request(vec![
                fixture_marginal("diameter", 0.9, 0.95),
                fixture_marginal("runout", 0.8, 0.9),
            ], vec!["diameter", "runout"]);
            match mutate {
                0 => request.marginals[0].part_id = "OTHER-PART".into(),
                1 => request.marginals[0].lot_id = "OTHER-LOT".into(),
                2 => request.marginals[0].operation_id = "OTHER-OP".into(),
                3 => request.marginals[0].decision_context_id = "OTHER-CONTEXT".into(),
                _ => request.marginals[0].decision_time_ns += 1,
            }
            assert_eq!(classify(&request).outcome, Outcome::EvidenceIncomplete);
        }
    }

    #[test]
    fn no_characteristic_can_be_dropped_by_unknown_or_missing_mass() {
        let mut r = request(vec![
            fixture_marginal("diameter", 0.9, 0.95),
            fixture_marginal("runout", 0.8, 0.9),
            fixture_marginal("hardness", 0.0, 1.0),
        ], vec!["diameter", "runout", "hardness"]);
        let report = classify(&r);
        assert_eq!(report.outcome, Outcome::Indeterminate);
        let previous = frechet_bounds(&[(0.9, 0.95), (0.8, 0.9)]).unwrap();
        assert!(report.joint_lower.unwrap() <= previous.0);
        assert!(report.joint_upper.unwrap() >= previous.1.min(1.0));
        r.manifest.required_characteristics.pop();
        assert_eq!(classify(&r).outcome, Outcome::EvidenceIncomplete);
    }

    #[test]
    fn permutation_is_invariant() {
        let r1 = request(vec![
            fixture_marginal("diameter", 0.9, 0.95),
            fixture_marginal("runout", 0.8, 0.9),
            fixture_marginal("hardness", 0.7, 0.85),
        ], vec!["diameter", "runout", "hardness"]);
        let mut r2 = r1.clone();
        r2.marginals.reverse();
        let a = classify(&r1);
        let b = classify(&r2);
        assert_eq!(a.joint_lower, b.joint_lower);
        assert_eq!(a.joint_upper, b.joint_upper);
        assert_eq!(a.outcome, b.outcome);
    }

    #[test]
    fn exhaustive_binary_event_atoms_attain_frechet_bounds() {
        for p in [
            vec![0.9, 0.9],
            vec![0.2, 0.7, 0.95],
            vec![0.5, 0.5, 0.5, 0.5],
            vec![0.1, 0.2, 0.3, 0.4, 0.5],
        ] {
            let expected_lower = (p.iter().sum::<f64>() - (p.len() as f64 - 1.0)).max(0.0);
            let expected_upper = p.iter().copied().fold(1.0_f64, f64::min);
            let lower_atoms = extremal_atom_masses(&p, true);
            let upper_atoms = extremal_atom_masses(&p, false);
            verify_atom_distribution(&lower_atoms, &p, expected_lower);
            verify_atom_distribution(&upper_atoms, &p, expected_upper);
        }
    }

    #[test]
    fn widening_marginal_probability_intervals_cannot_shrink_joint_bounds() {
        let tight = frechet_bounds(&[(0.9, 0.95), (0.8, 0.9), (0.85, 0.95)]).unwrap();
        let wide = frechet_bounds(&[(0.7, 1.0), (0.6, 1.0), (0.5, 1.0)]).unwrap();
        assert!(wide.0 <= tight.0 + FLOAT_TOLERANCE);
        assert!(wide.1 >= tight.1 - FLOAT_TOLERANCE);
    }

    #[test]
    fn malformed_marginals_and_policy_reject() {
        assert!(frechet_bounds(&[(0.8, 0.7), (0.9, 0.95)]).is_err());
        assert!(frechet_bounds(&[(f64::NAN, 1.0), (0.9, 1.0)]).is_err());
        assert!(frechet_bounds(&[(0.2, 1.1), (0.9, 1.0)]).is_err());
        let mut r = request(vec![
            fixture_marginal("diameter", 0.9, 0.95),
            fixture_marginal("runout", 0.8, 0.9),
        ], vec!["diameter", "runout"]);
        r.policy.accept_threshold = 0.5;
        r.policy.reject_threshold = 0.6;
        assert_eq!(classify(&r).outcome, Outcome::InvalidInput);
    }

    #[test]
    fn complete_verification_and_replay_are_deterministic() {
        verify().unwrap();
        let r = request(vec![
            fixture_marginal("diameter", 0.9, 0.95),
            fixture_marginal("runout", 0.8, 0.9),
        ], vec!["diameter", "runout"]);
        let a = classify(&r);
        let b = classify(&r);
        assert_eq!(a.joint_lower, b.joint_lower);
        assert_eq!(a.joint_upper, b.joint_upper);
        assert_eq!(a.reason, b.reason);
    }
}
