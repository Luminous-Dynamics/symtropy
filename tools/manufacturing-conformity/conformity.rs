//! Evidence-gated manufacturing conformity decision reference.
//! Pure scalar rule; not a metrology authority or product-release authorization.
//! rustc --edition=2024 -D warnings --test tools/manufacturing-conformity/conformity.rs -o /tmp/conformity-tests
//! rustc --edition=2024 -D warnings tools/manufacturing-conformity/conformity.rs -o /tmp/conformity

const REL_TOL: f64 = 1.0e-10;
const HASH_LEN: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EvidenceState { Verified, Missing, Stale, Mismatched, Revoked, NotQualified }

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

    fn failure_code(self, family: &'static str) -> Option<&'static str> {
        match (self, family) {
            (Self::Verified, _) => None,
            (Self::Missing, "measurement") => Some("MEASUREMENT_EVIDENCE_MISSING"),
            (Self::Missing, "specification") => Some("SPECIFICATION_EVIDENCE_MISSING"),
            (Self::Missing, "calibration") => Some("CALIBRATION_EVIDENCE_MISSING"),
            (Self::Missing, "msa") => Some("MSA_EVIDENCE_MISSING"),
            (Self::Missing, "uncertainty") => Some("UNCERTAINTY_EVIDENCE_MISSING"),
            (Self::Missing, _) => Some("CONTEXT_POLICY_EVIDENCE_MISSING"),
            (Self::Stale, "measurement") => Some("MEASUREMENT_EVIDENCE_STALE"),
            (Self::Stale, "specification") => Some("SPECIFICATION_EVIDENCE_STALE"),
            (Self::Stale, "calibration") => Some("CALIBRATION_EVIDENCE_STALE"),
            (Self::Stale, "msa") => Some("MSA_EVIDENCE_STALE"),
            (Self::Stale, "uncertainty") => Some("UNCERTAINTY_EVIDENCE_STALE"),
            (Self::Stale, _) => Some("CONTEXT_POLICY_EVIDENCE_STALE"),
            (Self::Mismatched, "measurement") => Some("MEASUREMENT_EVIDENCE_MISMATCHED"),
            (Self::Mismatched, "specification") => Some("SPECIFICATION_EVIDENCE_MISMATCHED"),
            (Self::Mismatched, "calibration") => Some("CALIBRATION_EVIDENCE_MISMATCHED"),
            (Self::Mismatched, "msa") => Some("MSA_EVIDENCE_MISMATCHED"),
            (Self::Mismatched, "uncertainty") => Some("UNCERTAINTY_EVIDENCE_MISMATCHED"),
            (Self::Mismatched, _) => Some("CONTEXT_POLICY_EVIDENCE_MISMATCHED"),
            (Self::Revoked, "measurement") => Some("MEASUREMENT_EVIDENCE_REVOKED"),
            (Self::Revoked, "specification") => Some("SPECIFICATION_EVIDENCE_REVOKED"),
            (Self::Revoked, "calibration") => Some("CALIBRATION_EVIDENCE_REVOKED"),
            (Self::Revoked, "msa") => Some("MSA_EVIDENCE_REVOKED"),
            (Self::Revoked, "uncertainty") => Some("UNCERTAINTY_EVIDENCE_REVOKED"),
            (Self::Revoked, _) => Some("CONTEXT_POLICY_EVIDENCE_REVOKED"),
            (Self::NotQualified, "measurement") => Some("MEASUREMENT_NOT_QUALIFIED"),
            (Self::NotQualified, "specification") => Some("SPECIFICATION_NOT_QUALIFIED"),
            (Self::NotQualified, "calibration") => Some("CALIBRATION_NOT_QUALIFIED"),
            (Self::NotQualified, "msa") => Some("MSA_NOT_QUALIFIED"),
            (Self::NotQualified, "uncertainty") => Some("UNCERTAINTY_BUDGET_NOT_QUALIFIED"),
            (Self::NotQualified, _) => Some("CONTEXT_POLICY_NOT_QUALIFIED"),
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct EvidenceChecks {
    measurement: EvidenceState,
    specification: EvidenceState,
    calibration: EvidenceState,
    msa: EvidenceState,
    uncertainty: EvidenceState,
    context_policy: EvidenceState,
}

impl EvidenceChecks {
    fn first_failure(self) -> Option<&'static str> {
        [
            ("measurement", self.measurement),
            ("specification", self.specification),
            ("calibration", self.calibration),
            ("msa", self.msa),
            ("uncertainty", self.uncertainty),
            ("context", self.context_policy),
        ].into_iter().find_map(|(family, state)| state.failure_code(family))
    }
}

#[derive(Clone, Debug)]
struct Measurement {
    id: String, part: String, lot: String, operation: String, measurand: String,
    unit: String, value: f64, time_ns: i64, instrument: String, calibration: String,
    calibration_from_ns: i64, calibration_until_ns: i64, source_sha256: String,
}

#[derive(Clone, Debug)]
struct Specification {
    id: String, revision: String, measurand: String, unit: String,
    lower: f64, upper: f64, effective_from_ns: i64, effective_until_ns: i64,
    source_sha256: String,
}

#[derive(Clone, Debug)]
struct UncertaintyBudget {
    id: String, revision: String, part_scope: String, measurand: String, unit: String,
    msa_study: String, msa_qualified: bool, standard_uncertainty: f64,
    coverage_factor: f64, expanded_uncertainty: f64, coverage_probability: f64,
    combination_method: String, calibration: String, source_sha256: String,
}

#[derive(Clone, Debug)]
struct Context {
    id: String, decision_rule: String, revision: String,
    expected_part: String, expected_lot: String, expected_operation: String,
}

#[derive(Clone, Debug)]
struct Provenance {
    calibration_sha256: String,
    msa_sha256: String,
    context_policy_sha256: String,
}

#[derive(Clone, Debug)]
struct Request {
    measurement: Measurement, specification: Specification,
    uncertainty: UncertaintyBudget, context: Context,
    provenance: Provenance, evidence: EvidenceChecks,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Outcome { Accept, RejectNonconforming, Indeterminate, EvidenceIncomplete }

impl Outcome {
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
struct Report {
    measurement_id: String, part_id: String, lot_id: String, specification_id: String,
    specification_revision: String, uncertainty_budget_id: String, context_id: String,
    decision_rule_id: String, decision_rule_revision: String, measurand_id: String, unit: String,
    operation_id: String, measured_at_ns: i64, instrument_id: String, calibration_id: String,
    calibration_valid_from_ns: i64, calibration_valid_until_ns: i64,
    specification_effective_from_ns: i64, specification_effective_until_ns: i64,
    msa_study_id: String, msa_qualified: bool, uncertainty_revision: String,
    uncertainty_part_scope: String, uncertainty_combination_method: String,
    measurement_sha256: String, specification_sha256: String, uncertainty_sha256: String,
    calibration_sha256: String, msa_sha256: String, context_policy_sha256: String,
    evidence: EvidenceChecks, outcome: Outcome, reason: &'static str,
    value: Option<f64>, standard_u: Option<f64>, coverage_factor: Option<f64>,
    expanded_u: Option<f64>, interval_lower: Option<f64>, interval_upper: Option<f64>,
    spec_lower: Option<f64>, spec_upper: Option<f64>,
    acceptance_lower: Option<f64>, acceptance_upper: Option<f64>, coverage_probability: Option<f64>,
}

impl Report {
    fn csv_row(&self) -> String {
        let f = |v: Option<f64>| v.map(|x| format!("{x:.12e}")).unwrap_or_default();
        let s = |v: &str| csv_escape(v);
        vec![
            s(&self.measurement_id), s(&self.part_id), s(&self.lot_id),
            s(&self.specification_id), s(&self.specification_revision),
            s(&self.uncertainty_budget_id), s(&self.context_id),
            s(&self.decision_rule_id), s(&self.decision_rule_revision),
            s(self.outcome.code()), s(self.reason), f(self.value), f(self.expanded_u),
            f(self.interval_lower), f(self.interval_upper), f(self.spec_lower),
            f(self.spec_upper), f(self.acceptance_lower), f(self.acceptance_upper),
            f(self.coverage_probability), s(&self.measurand_id), s(&self.unit),
            s(&self.operation_id), self.measured_at_ns.to_string(), s(&self.instrument_id),
            s(&self.calibration_id), self.calibration_valid_from_ns.to_string(),
            self.calibration_valid_until_ns.to_string(),
            self.specification_effective_from_ns.to_string(),
            self.specification_effective_until_ns.to_string(), s(&self.msa_study_id),
            self.msa_qualified.to_string(), s(&self.uncertainty_revision),
            s(&self.uncertainty_part_scope), s(&self.uncertainty_combination_method),
            self.measurement_sha256.clone(), self.specification_sha256.clone(),
            self.calibration_sha256.clone(), self.msa_sha256.clone(),
            self.uncertainty_sha256.clone(), self.context_policy_sha256.clone(),
            self.evidence.measurement.code().to_string(),
            self.evidence.specification.code().to_string(),
            self.evidence.calibration.code().to_string(),
            self.evidence.msa.code().to_string(),
            self.evidence.uncertainty.code().to_string(),
            self.evidence.context_policy.code().to_string(),
            f(self.standard_u), f(self.coverage_factor),
        ].join(",")
    }
}

fn csv_escape(value: &str) -> String {
    if value.chars().any(|ch| matches!(ch, ',' | '"' | '\\r' | '\\n')) {
        format!("\\"{}\\"", value.replace('"', "\\"\\\"\\\""))
    } else {
        value.to_string()
    }
}

fn present(s: &str) -> bool { !s.trim().is_empty() }
fn valid_sha(s: &str) -> bool {
    s.len() == HASH_LEN && s.bytes().all(|b| b.is_ascii_hexdigit())
}
fn finite(v: f64) -> Option<f64> { v.is_finite().then_some(v) }

fn incomplete(r: &Request, reason: &'static str) -> Report {
    Report {
        measurement_id: r.measurement.id.clone(), part_id: r.measurement.part.clone(),
        lot_id: r.measurement.lot.clone(), specification_id: r.specification.id.clone(),
        specification_revision: r.specification.revision.clone(),
        uncertainty_budget_id: r.uncertainty.id.clone(), context_id: r.context.id.clone(),
        decision_rule_id: r.context.decision_rule.clone(),
        decision_rule_revision: r.context.revision.clone(),
        measurand_id: r.measurement.measurand.clone(), unit: r.measurement.unit.clone(),
        operation_id: r.measurement.operation.clone(), measured_at_ns: r.measurement.time_ns,
        instrument_id: r.measurement.instrument.clone(), calibration_id: r.measurement.calibration.clone(),
        calibration_valid_from_ns: r.measurement.calibration_from_ns,
        calibration_valid_until_ns: r.measurement.calibration_until_ns,
        specification_effective_from_ns: r.specification.effective_from_ns,
        specification_effective_until_ns: r.specification.effective_until_ns,
        msa_study_id: r.uncertainty.msa_study.clone(), msa_qualified: r.uncertainty.msa_qualified,
        uncertainty_revision: r.uncertainty.revision.clone(),
        uncertainty_part_scope: r.uncertainty.part_scope.clone(),
        uncertainty_combination_method: r.uncertainty.combination_method.clone(),
        measurement_sha256: r.measurement.source_sha256.clone(),
        specification_sha256: r.specification.source_sha256.clone(),
        uncertainty_sha256: r.uncertainty.source_sha256.clone(),
        calibration_sha256: r.provenance.calibration_sha256.clone(),
        msa_sha256: r.provenance.msa_sha256.clone(),
        context_policy_sha256: r.provenance.context_policy_sha256.clone(),
        evidence: r.evidence,
        outcome: Outcome::EvidenceIncomplete, reason,
        value: finite(r.measurement.value),
        standard_u: finite(r.uncertainty.standard_uncertainty),
        coverage_factor: finite(r.uncertainty.coverage_factor),
        expanded_u: finite(r.uncertainty.expanded_uncertainty),
        interval_lower: None, interval_upper: None,
        spec_lower: finite(r.specification.lower), spec_upper: finite(r.specification.upper),
        acceptance_lower: None, acceptance_upper: None,
        coverage_probability: finite(r.uncertainty.coverage_probability),
    }
}

fn validate(r: &Request) -> Result<(), &'static str> {
    if let Some(reason) = r.evidence.first_failure() { return Err(reason); }
    let m = &r.measurement; let s = &r.specification; let u = &r.uncertainty; let c = &r.context;
    let strings = [
        m.id.as_str(), m.part.as_str(), m.lot.as_str(), m.operation.as_str(), m.measurand.as_str(),
        m.unit.as_str(), m.instrument.as_str(), m.calibration.as_str(),
        s.id.as_str(), s.revision.as_str(), s.measurand.as_str(), s.unit.as_str(),
        u.id.as_str(), u.revision.as_str(), u.part_scope.as_str(), u.measurand.as_str(),
        u.unit.as_str(), u.msa_study.as_str(), u.combination_method.as_str(), u.calibration.as_str(),
        c.id.as_str(), c.decision_rule.as_str(), c.revision.as_str(),
        c.expected_part.as_str(), c.expected_lot.as_str(), c.expected_operation.as_str(),
    ];
    if strings.iter().any(|x| !present(x)) { return Err("REQUIRED_IDENTITY_MISSING"); }
    if !m.value.is_finite() || !s.lower.is_finite() || !s.upper.is_finite()
        || !u.standard_uncertainty.is_finite() || !u.coverage_factor.is_finite()
        || !u.expanded_uncertainty.is_finite() || !u.coverage_probability.is_finite() {
        return Err("NONFINITE_NUMERIC_INPUT");
    }
    if s.lower >= s.upper { return Err("SPECIFICATION_LIMITS_INVALID"); }
    if u.standard_uncertainty <= 0.0 || u.coverage_factor <= 0.0
        || u.expanded_uncertainty <= 0.0
        || !(0.0 < u.coverage_probability && u.coverage_probability <= 1.0) {
        return Err("UNCERTAINTY_PARAMETERS_INVALID");
    }
    if !u.msa_qualified { return Err("MSA_NOT_QUALIFIED"); }
    if !valid_sha(&m.source_sha256) || !valid_sha(&s.source_sha256) || !valid_sha(&u.source_sha256)
        || !valid_sha(&r.provenance.calibration_sha256)
        || !valid_sha(&r.provenance.msa_sha256)
        || !valid_sha(&r.provenance.context_policy_sha256) {
        return Err("SOURCE_DIGEST_INVALID");
    }
    if m.measurand != s.measurand || m.measurand != u.measurand {
        return Err("MEASURAND_ID_MISMATCH");
    }
    if m.unit != s.unit || m.unit != u.unit { return Err("UNIT_MISMATCH"); }
    if m.part != c.expected_part || m.lot != c.expected_lot || m.operation != c.expected_operation
        || u.part_scope != m.part {
        return Err("PART_LOT_OPERATION_SCOPE_MISMATCH");
    }
    if m.calibration != u.calibration { return Err("CALIBRATION_SCOPE_MISMATCH"); }
    if m.calibration_from_ns > m.calibration_until_ns || s.effective_from_ns > s.effective_until_ns {
        return Err("EVIDENCE_VALIDITY_INTERVAL_INVALID");
    }
    if m.time_ns < m.calibration_from_ns || m.time_ns > m.calibration_until_ns {
        return Err("CALIBRATION_NOT_VALID_AT_MEASUREMENT_TIME");
    }
    if m.time_ns < s.effective_from_ns || m.time_ns > s.effective_until_ns {
        return Err("SPECIFICATION_NOT_EFFECTIVE_AT_MEASUREMENT_TIME");
    }
    let expanded = u.standard_uncertainty * u.coverage_factor;
    if !expanded.is_finite() { return Err("UNCERTAINTY_EXPANSION_OVERFLOW"); }
    let tolerance = REL_TOL * expanded.abs().max(u.expanded_uncertainty.abs()).max(1.0e-300);
    if (expanded - u.expanded_uncertainty).abs() > tolerance {
        return Err("EXPANDED_UNCERTAINTY_INCONSISTENT_WITH_BUDGET");
    }
    Ok(())
}

fn classify(r: &Request) -> Report {
    if let Err(reason) = validate(r) { return incomplete(r, reason); }
    let m = &r.measurement; let s = &r.specification; let u = &r.uncertainty;
    let lo = m.value - u.expanded_uncertainty;
    let hi = m.value + u.expanded_uncertainty;
    let accept_lo = s.lower + u.expanded_uncertainty;
    let accept_hi = s.upper - u.expanded_uncertainty;
    if !lo.is_finite() || !hi.is_finite() || !accept_lo.is_finite() || !accept_hi.is_finite() {
        return incomplete(r, "UNCERTAINTY_INTERVAL_OVERFLOW");
    }
    let (outcome, reason) = if lo >= s.lower && hi <= s.upper {
        (Outcome::Accept, "UNCERTAINTY_INTERVAL_INSIDE_SPECIFICATION")
    } else if hi < s.lower || lo > s.upper {
        (Outcome::RejectNonconforming, "UNCERTAINTY_INTERVAL_STRICTLY_OUTSIDE_SPECIFICATION")
    } else {
        (Outcome::Indeterminate, "UNCERTAINTY_INTERVAL_OVERLAPS_SPECIFICATION_BOUNDARY")
    };
    Report {
        measurement_id: m.id.clone(), part_id: m.part.clone(), lot_id: m.lot.clone(),
        specification_id: s.id.clone(), specification_revision: s.revision.clone(),
        uncertainty_budget_id: u.id.clone(), context_id: r.context.id.clone(),
        decision_rule_revision: r.context.revision.clone(),
        measurand_id: m.measurand.clone(), unit: m.unit.clone(),
        measured_at_ns: m.time_ns, instrument_id: m.instrument.clone(),
        calibration_id: m.calibration.clone(), msa_study_id: u.msa_study.clone(),
        measurement_sha256: m.source_sha256.clone(), specification_sha256: s.source_sha256.clone(),
        uncertainty_sha256: u.source_sha256.clone(),
        calibration_sha256: r.provenance.calibration_sha256.clone(),
        msa_sha256: r.provenance.msa_sha256.clone(),
        context_policy_sha256: r.provenance.context_policy_sha256.clone(),
        evidence: r.evidence, outcome, reason,
        value: Some(m.value), standard_u: Some(u.standard_uncertainty),
        coverage_factor: Some(u.coverage_factor), expanded_u: Some(u.expanded_uncertainty),
        interval_lower: Some(lo), interval_upper: Some(hi),
        spec_lower: Some(s.lower), spec_upper: Some(s.upper),
        acceptance_lower: Some(accept_lo), acceptance_upper: Some(accept_hi),
        coverage_probability: Some(u.coverage_probability),
    }
}

fn request(value: f64, expanded_u: f64) -> Request {
    Request {
        measurement: Measurement {
            id: "MEAS-0001".into(), part: "PART-0001".into(), lot: "LOT-0001".into(),
            operation: "OP-FINISH-01".into(), measurand: "shaft-diameter".into(),
            unit: "mm".into(), value, time_ns: 1_750_000_000,
            instrument: "CMM-01".into(), calibration: "CAL-CMM-01".into(),
            calibration_from_ns: 1_700_000_000, calibration_until_ns: 1_800_000_000,
            source_sha256: "a".repeat(HASH_LEN),
        },
        specification: Specification {
            id: "SPEC-SHAFT-01".into(), revision: "REV-7".into(),
            measurand: "shaft-diameter".into(), unit: "mm".into(), lower: 10.0, upper: 20.0,
            effective_from_ns: 1_600_000_000, effective_until_ns: 1_900_000_000,
            source_sha256: "b".repeat(HASH_LEN),
        },
        uncertainty: UncertaintyBudget {
            id: "UQ-CMM-01".into(), revision: "UQ-REV-3".into(), part_scope: "PART-0001".into(),
            measurand: "shaft-diameter".into(), unit: "mm".into(), msa_study: "MSA-CROSSED-01".into(),
            msa_qualified: true, standard_uncertainty: expanded_u / 2.0,
            coverage_factor: 2.0, expanded_uncertainty: expanded_u, coverage_probability: 0.95,
            combination_method: "declared_covariance_aware_budget_v1".into(),
            calibration: "CAL-CMM-01".into(), source_sha256: "c".repeat(HASH_LEN),
        },
        context: Context {
            id: "COU-SHAFT-FINAL-INSPECTION".into(), decision_rule: "ISO14253-1-GUARDED-INTERVAL".into(),
            revision: "RULE-1.0".into(), expected_part: "PART-0001".into(),
            expected_lot: "LOT-0001".into(), expected_operation: "OP-FINISH-01".into(),
        },
        provenance: Provenance {
            calibration_sha256: "d".repeat(HASH_LEN),
            msa_sha256: "e".repeat(HASH_LEN),
            context_policy_sha256: "f".repeat(HASH_LEN),
        },
        evidence: EvidenceChecks {
            measurement: EvidenceState::Verified, specification: EvidenceState::Verified,
            calibration: EvidenceState::Verified, msa: EvidenceState::Verified,
            uncertainty: EvidenceState::Verified, context_policy: EvidenceState::Verified,
        },
    }
}

fn next_up(x: f64) -> f64 {
    if x.is_nan() || x == f64::INFINITY { return x; }
    if x == 0.0 { return f64::from_bits(1); }
    let b = x.to_bits();
    f64::from_bits(if x > 0.0 { b + 1 } else { b - 1 })
}
fn next_down(x: f64) -> f64 {
    if x.is_nan() || x == f64::NEG_INFINITY { return x; }
    if x == 0.0 { return -f64::from_bits(1); }
    let b = x.to_bits();
    f64::from_bits(if x > 0.0 { b - 1 } else { b + 1 })
}

fn campaign() -> Vec<(&'static str, Request)> {
    vec![
        ("inside", request(15.0, 1.0)),
        ("touch_lower_inside", request(11.0, 1.0)),
        ("touch_upper_inside", request(19.0, 1.0)),
        ("partial_lower", request(10.5, 1.0)),
        ("partial_upper", request(19.5, 1.0)),
        ("touch_lower_outside", request(9.0, 1.0)),
        ("touch_upper_outside", request(21.0, 1.0)),
        ("strictly_below", request(8.0, 1.0)),
        ("strictly_above", request(22.0, 1.0)),
        ("full_width_touch", request(15.0, 5.0)),
        ("full_width_exceeds", request(15.0, 5.5)),
    ]
}

fn verify() -> Result<(), String> {
    let expected = [
        Outcome::Accept, Outcome::Accept, Outcome::Accept, Outcome::Indeterminate,
        Outcome::Indeterminate, Outcome::Indeterminate, Outcome::Indeterminate,
        Outcome::RejectNonconforming, Outcome::RejectNonconforming, Outcome::Accept,
        Outcome::Indeterminate,
    ];
    for ((name, r), expected) in campaign().iter().zip(expected.iter()) {
        let actual = classify(r).outcome;
        if actual != *expected {
            return Err(format!("GOLDEN_CLASSIFICATION_MISMATCH {name}: got {}, expected {}",
                actual.code(), expected.code()));
        }
    }
    let u = 1.0;
    if classify(&request(next_down(11.0), u)).outcome != Outcome::Indeterminate
        || classify(&request(next_up(11.0), u)).outcome != Outcome::Accept
        || classify(&request(next_down(19.0), u)).outcome != Outcome::Accept
        || classify(&request(next_up(19.0), u)).outcome != Outcome::Indeterminate {
        return Err("NEXTAFTER_ACCEPTANCE_BOUNDARY_FAILED".into());
    }
    if classify(&request(next_down(9.0), u)).outcome != Outcome::RejectNonconforming
        || classify(&request(next_up(9.0), u)).outcome != Outcome::Indeterminate
        || classify(&request(next_down(21.0), u)).outcome != Outcome::Indeterminate
        || classify(&request(next_up(21.0), u)).outcome != Outcome::RejectNonconforming {
        return Err("NEXTAFTER_REJECTION_BOUNDARY_FAILED".into());
    }

    let millimetres = request(15.0, 1.0);
    let mut metres = millimetres.clone();
    metres.measurement.unit = "m".into();
    metres.measurement.value *= 0.001;
    metres.specification.unit = "m".into();
    metres.specification.lower *= 0.001;
    metres.specification.upper *= 0.001;
    metres.uncertainty.unit = "m".into();
    metres.uncertainty.standard_uncertainty *= 0.001;
    metres.uncertainty.expanded_uncertainty *= 0.001;
    if classify(&millimetres).outcome != classify(&metres).outcome {
        return Err("UNIT_SCALE_METAMORPHIC_INVARIANCE_FAILED".into());
    }

    let partial = classify(&request(10.5, 1.0)).outcome;
    let wider = classify(&request(10.5, 2.0)).outcome;
    if partial != Outcome::Indeterminate || wider == Outcome::Accept {
        return Err("UNCERTAINTY_WIDENING_MONOTONICITY_FAILED".into());
    }
    Ok(())
}

fn emit_csv() {
    println!("case_id,measurement_id,part_id,lot_id,specification_id,specification_revision,uncertainty_budget_id,context_id,decision_rule_id,decision_rule_revision,outcome,reason,value,expanded_u,interval_lower,interval_upper,spec_lower,spec_upper,acceptance_lower,acceptance_upper,coverage_probability,measurand_id,unit,operation_id,measured_at_ns,instrument_id,calibration_id,calibration_valid_from_ns,calibration_valid_until_ns,specification_effective_from_ns,specification_effective_until_ns,msa_study_id,msa_qualified,uncertainty_revision,uncertainty_part_scope,uncertainty_combination_method,measurement_sha256,specification_sha256,calibration_sha256,msa_sha256,uncertainty_sha256,context_policy_sha256,measurement_evidence,specification_evidence,calibration_evidence,msa_evidence,uncertainty_evidence,context_policy_evidence,standard_uncertainty,coverage_factor");
    for (name, r) in campaign() { println!("{name},{}", classify(&r).csv_row()); }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.as_slice() {
        [] => verify().map(|()| println!("MFG_CONFORMITY_OK: evidence-gated three-way guard-band decisions")),
        [a] if a == "--verify" => verify().map(|()| println!("MFG_CONFORMITY_OK: evidence-gated three-way guard-band decisions")),
        [a] if a == "--csv" => { emit_csv(); Ok(()) },
        [a] if a == "--help" || a == "-h" => { println!("Usage: conformity [--verify|--csv|--help]"); Ok(()) },
        _ => Err("expected --verify, --csv or --help".into()),
    };
    if let Err(e) = result { eprintln!("MFG_CONFORMITY_ERROR: {e}"); std::process::exit(1); }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closed_spec_three_way_semantics() {
        assert_eq!(classify(&request(15.0, 1.0)).outcome, Outcome::Accept);
        assert_eq!(classify(&request(10.5, 1.0)).outcome, Outcome::Indeterminate);
        assert_eq!(classify(&request(8.0, 1.0)).outcome, Outcome::RejectNonconforming);
        assert_eq!(classify(&request(22.0, 1.0)).outcome, Outcome::RejectNonconforming);
    }

    #[test]
    fn equality_inside_and_outside_is_explicit() {
        assert_eq!(classify(&request(11.0, 1.0)).outcome, Outcome::Accept);
        assert_eq!(classify(&request(19.0, 1.0)).outcome, Outcome::Accept);
        assert_eq!(classify(&request(9.0, 1.0)).outcome, Outcome::Indeterminate);
        assert_eq!(classify(&request(21.0, 1.0)).outcome, Outcome::Indeterminate);
    }

    #[test]
    fn exact_nextafter_boundaries_pass() { verify().unwrap(); }

    #[test]
    fn missing_stale_and_revoked_evidence_never_disposes_product() {
        for state in [EvidenceState::Missing, EvidenceState::Stale, EvidenceState::Revoked] {
            let mut r = request(15.0, 1.0);
            r.evidence.calibration = state;
            assert_eq!(classify(&r).outcome, Outcome::EvidenceIncomplete);
        }
        let mut r = request(8.0, 1.0);
        r.evidence.uncertainty = EvidenceState::Missing;
        assert_eq!(classify(&r).outcome, Outcome::EvidenceIncomplete);
    }

    #[test]
    fn mSA_and_context_policy_failures_remain_evidence_incomplete() {
        let mut r = request(15.0, 1.0);
        r.uncertainty.msa_qualified = false;
        assert_eq!(classify(&r).reason, "MSA_NOT_QUALIFIED");

        let mut r = request(15.0, 1.0);
        r.evidence.msa = EvidenceState::NotQualified;
        assert_eq!(classify(&r).reason, "MSA_EVIDENCE_NOT_QUALIFIED");

        let mut r = request(15.0, 1.0);
        r.evidence.context_policy = EvidenceState::Missing;
        assert_eq!(classify(&r).reason, "CONTEXT_POLICY_EVIDENCE_MISSING");
    }

    #[test]
    fn emitted_receipt_preserves_all_source_digests_and_evidence_states() {
        let row = classify(&request(15.0, 1.0)).csv_row();
        for digest in ["a".repeat(HASH_LEN), "b".repeat(HASH_LEN),
                       "c".repeat(HASH_LEN), "d".repeat(HASH_LEN),
                       "e".repeat(HASH_LEN), "f".repeat(HASH_LEN)] {
            assert!(row.contains(digest.as_str()));
        }
        assert!(row.contains("VERIFIED"));
        assert!(row.contains("MSA-CROSSED-01"));
        assert!(row.contains("CAL-CMM-01"));
        assert!(row.contains("shaft-diameter"));
        assert!(row.contains("COU-SHAFT-FINAL-INSPECTION"));
    }

    #[test]
    fn numeric_overflow_in_the_coverage_interval_abstains() {
        let mut r = request(1.0e308, 1.0e308);
        r.specification.lower = -1.0e308;
        r.specification.upper = 1.0e308;
        r.uncertainty.standard_uncertainty = 5.0e307;
        r.uncertainty.coverage_factor = 2.0;
        let report = classify(&r);
        assert_eq!(report.outcome, Outcome::EvidenceIncomplete);
        assert_eq!(report.reason, "UNCERTAINTY_INTERVAL_OVERFLOW");
    }

    #[test]
    fn unit_measurand_lot_and_calibration_scope_mismatches_fail_closed() {
        let mut r = request(15.0, 1.0);
        r.measurement.unit = "inch".into();
        assert_eq!(classify(&r).reason, "UNIT_MISMATCH");
        let mut r = request(15.0, 1.0);
        r.uncertainty.measurand = "shaft-runout".into();
        assert_eq!(classify(&r).reason, "MEASURAND_ID_MISMATCH");
        let mut r = request(15.0, 1.0);
        r.context.expected_lot = "LOT-OTHER".into();
        assert_eq!(classify(&r).reason, "PART_LOT_OPERATION_SCOPE_MISMATCH");
        let mut r = request(15.0, 1.0);
        r.uncertainty.calibration = "CAL-OTHER".into();
        assert_eq!(classify(&r).reason, "CALIBRATION_SCOPE_MISMATCH");
    }

    #[test]
    fn calibration_and_spec_must_be_current_at_measurement_time() {
        let mut r = request(15.0, 1.0);
        r.measurement.time_ns = r.measurement.calibration_until_ns + 1;
        assert_eq!(classify(&r).reason, "CALIBRATION_NOT_VALID_AT_MEASUREMENT_TIME");
        let mut r = request(15.0, 1.0);
        r.measurement.time_ns = r.specification.effective_until_ns + 1;
        assert_eq!(classify(&r).reason, "SPECIFICATION_NOT_EFFECTIVE_AT_MEASUREMENT_TIME");
    }

    #[test]
    fn malformed_uncertainty_spec_limits_and_hashes_fail_closed() {
        let mut r = request(15.0, 1.0);
        r.uncertainty.expanded_uncertainty = 1.1;
        assert_eq!(classify(&r).reason, "EXPANDED_UNCERTAINTY_INCONSISTENT_WITH_BUDGET");
        let mut r = request(15.0, 1.0);
        r.specification.lower = f64::NAN;
        assert_eq!(classify(&r).reason, "NONFINITE_NUMERIC_INPUT");
        let mut r = request(15.0, 1.0);
        r.specification.lower = 20.0;
        r.specification.upper = 10.0;
        assert_eq!(classify(&r).reason, "SPECIFICATION_LIMITS_INVALID");
        let mut r = request(15.0, 1.0);
        r.uncertainty.coverage_probability = 0.0;
        assert_eq!(classify(&r).reason, "UNCERTAINTY_PARAMETERS_INVALID");
        let mut r = request(15.0, 1.0);
        r.measurement.source_sha256 = "bad".into();
        assert_eq!(classify(&r).reason, "SOURCE_DIGEST_INVALID");
    }

    #[test]
    fn unit_scaling_and_uncertainty_monotonicity_hold() { verify().unwrap(); }

    #[test]
    fn deterministic_report_bytes() {
        let a = classify(&request(15.0, 1.0)).csv_row();
        let b = classify(&request(15.0, 1.0)).csv_row();
        assert_eq!(a.as_bytes(), b.as_bytes());
    }

    #[test]
    fn frozen_campaign_matches_golden_outcomes() { verify().unwrap(); }
}
