//! Distribution-aware manufacturing conformity reference.
//! Consumes an explicit post-measurement posterior; never infers a PDF from U=k*u.
//! Offline research only: ACCEPT is not a product-release authorization.
//!
//! rustc --edition=2024 -D warnings --test tools/manufacturing-conformity-risk/risk.rs -o /tmp/mfg-risk-tests
//! rustc --edition=2024 -D warnings tools/manufacturing-conformity-risk/risk.rs -o /tmp/mfg-risk

const TOL: f64 = 1.0e-12;
const HASH_LEN: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EvidenceState { Verified, Missing, Stale, Mismatched, Revoked, NotQualified }

impl EvidenceState {
    fn code(self) -> &'static str {
        match self {
            Self::Verified => "VERIFIED", Self::Missing => "MISSING", Self::Stale => "STALE",
            Self::Mismatched => "MISMATCHED", Self::Revoked => "REVOKED",
            Self::NotQualified => "NOT_QUALIFIED",
        }
    }
    fn failure(self, family: &'static str) -> Option<&'static str> {
        match (self, family) {
            (Self::Verified, _) => None,
            (Self::Missing, "posterior") => Some("POSTERIOR_EVIDENCE_MISSING"),
            (Self::Missing, "specification") => Some("SPECIFICATION_EVIDENCE_MISSING"),
            (Self::Missing, "calibration") => Some("CALIBRATION_EVIDENCE_MISSING"),
            (Self::Missing, "msa") => Some("MSA_EVIDENCE_MISSING"),
            (Self::Missing, "uncertainty") => Some("UNCERTAINTY_EVIDENCE_MISSING"),
            (Self::Missing, _) => Some("POLICY_EVIDENCE_MISSING"),
            (Self::Stale, "posterior") => Some("POSTERIOR_EVIDENCE_STALE"),
            (Self::Stale, "specification") => Some("SPECIFICATION_EVIDENCE_STALE"),
            (Self::Stale, "calibration") => Some("CALIBRATION_EVIDENCE_STALE"),
            (Self::Stale, "msa") => Some("MSA_EVIDENCE_STALE"),
            (Self::Stale, "uncertainty") => Some("UNCERTAINTY_EVIDENCE_STALE"),
            (Self::Stale, _) => Some("POLICY_EVIDENCE_STALE"),
            (Self::Mismatched, "posterior") => Some("POSTERIOR_EVIDENCE_MISMATCHED"),
            (Self::Mismatched, "specification") => Some("SPECIFICATION_EVIDENCE_MISMATCHED"),
            (Self::Mismatched, "calibration") => Some("CALIBRATION_EVIDENCE_MISMATCHED"),
            (Self::Mismatched, "msa") => Some("MSA_EVIDENCE_MISMATCHED"),
            (Self::Mismatched, "uncertainty") => Some("UNCERTAINTY_EVIDENCE_MISMATCHED"),
            (Self::Mismatched, _) => Some("POLICY_EVIDENCE_MISMATCHED"),
            (Self::Revoked, "posterior") => Some("POSTERIOR_EVIDENCE_REVOKED"),
            (Self::Revoked, "specification") => Some("SPECIFICATION_EVIDENCE_REVOKED"),
            (Self::Revoked, "calibration") => Some("CALIBRATION_EVIDENCE_REVOKED"),
            (Self::Revoked, "msa") => Some("MSA_EVIDENCE_REVOKED"),
            (Self::Revoked, "uncertainty") => Some("UNCERTAINTY_EVIDENCE_REVOKED"),
            (Self::Revoked, _) => Some("POLICY_EVIDENCE_REVOKED"),
            (Self::NotQualified, "posterior") => Some("POSTERIOR_NOT_QUALIFIED"),
            (Self::NotQualified, "specification") => Some("SPECIFICATION_NOT_QUALIFIED"),
            (Self::NotQualified, "calibration") => Some("CALIBRATION_NOT_QUALIFIED"),
            (Self::NotQualified, "msa") => Some("MSA_NOT_QUALIFIED"),
            (Self::NotQualified, "uncertainty") => Some("UNCERTAINTY_NOT_QUALIFIED"),
            (Self::NotQualified, _) => Some("POLICY_NOT_QUALIFIED"),
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Evidence {
    posterior: EvidenceState, specification: EvidenceState, calibration: EvidenceState,
    msa: EvidenceState, uncertainty: EvidenceState, policy: EvidenceState,
}
impl Evidence {
    fn first_failure(self) -> Option<&'static str> {
        [
            ("posterior", self.posterior), ("specification", self.specification),
            ("calibration", self.calibration), ("msa", self.msa),
            ("uncertainty", self.uncertainty), ("policy", self.policy),
        ].into_iter().find_map(|(name, state)| state.failure(name))
    }
}

#[derive(Clone, Copy, Debug)]
struct Bin { lower: f64, upper: f64, mass: f64 }

#[derive(Clone, Debug)]
struct Posterior {
    id: String, measurement_id: String, part_id: String, lot_id: String,
    measurand: String, unit: String, spec_revision: String,
    calibration_id: String, msa_study_id: String, model_revision: String,
    sha256: String, bins: Vec<Bin>, unknown_tail_mass: f64,
}
#[derive(Clone, Debug)]
struct Specification {
    id: String, revision: String, measurand: String, unit: String,
    lower: f64, upper: f64, effective_from_ns: i64, effective_until_ns: i64,
    sha256: String,
}
#[derive(Clone, Debug)]
struct Policy {
    id: String, revision: String, owner: String, min_accept_pc: f64, max_reject_pc: f64,
    effective_from_ns: i64, effective_until_ns: i64, approved: bool, sha256: String,
}
#[derive(Clone, Debug)]
struct Provenance {
    uncertainty_budget_id: String, uncertainty_sha256: String,
    calibration_sha256: String, msa_sha256: String,
}
#[derive(Clone, Debug)]
struct Request {
    posterior: Posterior, spec: Specification, policy: Policy,
    provenance: Provenance, evidence: Evidence, decision_time_ns: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Outcome { Accept, Reject, Indeterminate, EvidenceIncomplete }
impl Outcome {
    fn code(self) -> &'static str {
        match self {
            Self::Accept => "ACCEPT",
            Self::Reject => "REJECT_NONCONFORMING",
            Self::Indeterminate => "INDETERMINATE",
            Self::EvidenceIncomplete => "EVIDENCE_INCOMPLETE",
        }
    }
}

#[derive(Clone, Debug)]
struct Report {
    posterior_id: String, measurement_id: String, part_id: String, lot_id: String,
    spec_id: String, spec_revision: String, policy_id: String, policy_revision: String,
    uncertainty_id: String, model_revision: String, posterior_sha256: String,
    spec_sha256: String, policy_sha256: String, uncertainty_sha256: String,
    calibration_sha256: String, msa_sha256: String, calibration_id: String, msa_study_id: String,
    policy_owner: String, spec_lower: f64, spec_upper: f64, min_accept_pc: f64,
    max_reject_pc: f64, decision_time_ns: i64, measurand: String, unit: String,
    outcome: Outcome, reason: &'static str, pc_model: Option<f64>,
    pc_lower: Option<f64>, pc_upper: Option<f64>, consumer_risk_upper: Option<f64>,
    producer_risk_upper: Option<f64>, tail_mass: Option<f64>, bin_count: usize,
    evidence: Evidence,
}
impl Report {
    fn csv_row(&self) -> String {
        let f = |v: Option<f64>| v.map(|x| format!("{x:.12e}")).unwrap_or_default();
        vec![
            self.posterior_id.clone(), self.measurement_id.clone(), self.part_id.clone(),
            self.lot_id.clone(), self.spec_id.clone(), self.spec_revision.clone(),
            self.policy_id.clone(), self.policy_revision.clone(), self.uncertainty_id.clone(),
            self.model_revision.clone(), self.posterior_sha256.clone(), self.spec_sha256.clone(),
            self.policy_sha256.clone(), self.uncertainty_sha256.clone(),
            self.calibration_sha256.clone(), self.msa_sha256.clone(),
            self.calibration_id.clone(), self.msa_study_id.clone(), self.policy_owner.clone(),
            format!("{:.12e}", self.spec_lower), format!("{:.12e}", self.spec_upper),
            format!("{:.12e}", self.min_accept_pc), format!("{:.12e}", self.max_reject_pc),
            self.decision_time_ns.to_string(), self.measurand.clone(), self.unit.clone(),
            self.outcome.code().to_string(), self.reason.to_string(),
            f(self.pc_model), f(self.pc_lower), f(self.pc_upper),
            f(self.consumer_risk_upper), f(self.producer_risk_upper), f(self.tail_mass),
            self.bin_count.to_string(), self.evidence.posterior.code().to_string(),
            self.evidence.specification.code().to_string(), self.evidence.calibration.code().to_string(),
            self.evidence.msa.code().to_string(), self.evidence.uncertainty.code().to_string(),
            self.evidence.policy.code().to_string(),
        ].join(",")
    }
}
fn present(v: &str) -> bool { !v.trim().is_empty() }
fn valid_sha(v: &str) -> bool {
    v.len() == HASH_LEN && v.bytes().all(|b| b.is_ascii_hexdigit())
}
fn finite(v: f64) -> Option<f64> { v.is_finite().then_some(v) }

fn incomplete(r: &Request, reason: &'static str) -> Report {
    Report {
        posterior_id: r.posterior.id.clone(), measurement_id: r.posterior.measurement_id.clone(),
        part_id: r.posterior.part_id.clone(), lot_id: r.posterior.lot_id.clone(),
        spec_id: r.spec.id.clone(), spec_revision: r.spec.revision.clone(),
        policy_id: r.policy.id.clone(), policy_revision: r.policy.revision.clone(),
        uncertainty_id: r.provenance.uncertainty_budget_id.clone(),
        model_revision: r.posterior.model_revision.clone(),
        posterior_sha256: r.posterior.sha256.clone(), spec_sha256: r.spec.sha256.clone(),
        policy_sha256: r.policy.sha256.clone(),
        uncertainty_sha256: r.provenance.uncertainty_sha256.clone(),
        calibration_sha256: r.provenance.calibration_sha256.clone(),
        msa_sha256: r.provenance.msa_sha256.clone(),
        calibration_id: r.posterior.calibration_id.clone(),
        msa_study_id: r.posterior.msa_study_id.clone(), policy_owner: r.policy.owner.clone(),
        spec_lower: r.spec.lower, spec_upper: r.spec.upper,
        min_accept_pc: r.policy.min_accept_pc, max_reject_pc: r.policy.max_reject_pc,
        decision_time_ns: r.decision_time_ns,
        measurand: r.posterior.measurand.clone(), unit: r.posterior.unit.clone(),
        outcome: Outcome::EvidenceIncomplete, reason, pc_model: None, pc_lower: None,
        pc_upper: None, consumer_risk_upper: None, producer_risk_upper: None,
        tail_mass: finite(r.posterior.unknown_tail_mass), bin_count: r.posterior.bins.len(),
        evidence: r.evidence,
    }
}

fn validate(r: &Request) -> Result<(), &'static str> {
    if let Some(reason) = r.evidence.first_failure() { return Err(reason); }
    let p = &r.posterior; let s = &r.spec; let q = &r.policy; let v = &r.provenance;
    let ids = [
        p.id.as_str(), p.measurement_id.as_str(), p.part_id.as_str(), p.lot_id.as_str(),
        p.measurand.as_str(), p.unit.as_str(), p.spec_revision.as_str(),
        p.calibration_id.as_str(), p.msa_study_id.as_str(), p.model_revision.as_str(),
        s.id.as_str(), s.revision.as_str(), s.measurand.as_str(), s.unit.as_str(),
        q.id.as_str(), q.revision.as_str(), q.owner.as_str(), v.uncertainty_budget_id.as_str(),
    ];
    if ids.iter().any(|x| !present(x)) { return Err("REQUIRED_IDENTITY_MISSING"); }
    if !s.lower.is_finite() || !s.upper.is_finite() || s.lower >= s.upper {
        return Err("SPECIFICATION_LIMITS_INVALID");
    }
    if !q.min_accept_pc.is_finite() || !q.max_reject_pc.is_finite()
        || q.min_accept_pc < 0.0 || q.min_accept_pc > 1.0
        || q.max_reject_pc < 0.0 || q.max_reject_pc > 1.0
        || q.max_reject_pc >= q.min_accept_pc {
        return Err("DECISION_THRESHOLDS_INVALID");
    }
    if !q.approved { return Err("DECISION_POLICY_NOT_APPROVED"); }
    if q.effective_from_ns > q.effective_until_ns || s.effective_from_ns > s.effective_until_ns {
        return Err("EVIDENCE_VALIDITY_INTERVAL_INVALID");
    }
    if r.decision_time_ns < q.effective_from_ns || r.decision_time_ns > q.effective_until_ns {
        return Err("DECISION_POLICY_NOT_EFFECTIVE");
    }
    if r.decision_time_ns < s.effective_from_ns || r.decision_time_ns > s.effective_until_ns {
        return Err("SPECIFICATION_NOT_EFFECTIVE");
    }
    if p.measurand != s.measurand { return Err("MEASURAND_ID_MISMATCH"); }
    if p.unit != s.unit { return Err("UNIT_MISMATCH"); }
    if p.spec_revision != s.revision { return Err("SPECIFICATION_REVISION_MISMATCH"); }
    if !valid_sha(&p.sha256) || !valid_sha(&s.sha256) || !valid_sha(&q.sha256)
        || !valid_sha(&v.uncertainty_sha256) || !valid_sha(&v.calibration_sha256)
        || !valid_sha(&v.msa_sha256) {
        return Err("SOURCE_DIGEST_INVALID");
    }
    if !present(&v.uncertainty_budget_id) { return Err("UNCERTAINTY_BUDGET_ID_MISSING"); }
    if !p.unknown_tail_mass.is_finite() || p.unknown_tail_mass < 0.0 || p.unknown_tail_mass > 1.0 {
        return Err("UNKNOWN_TAIL_MASS_INVALID");
    }
    if p.bins.is_empty() { return Err("POSTERIOR_BINS_EMPTY"); }
    let mut total = p.unknown_tail_mass;
    let mut previous_upper: Option<f64> = None;
    for bin in &p.bins {
        if !bin.lower.is_finite() || !bin.upper.is_finite() || !bin.mass.is_finite() {
            return Err("POSTERIOR_BIN_NONFINITE");
        }
        if bin.lower >= bin.upper { return Err("POSTERIOR_BIN_WIDTH_INVALID"); }
        if bin.mass < 0.0 || bin.mass > 1.0 { return Err("POSTERIOR_BIN_MASS_INVALID"); }
        if let Some(previous) = previous_upper {
            if bin.lower < previous { return Err("POSTERIOR_BINS_OVERLAP_OR_UNSORTED"); }
            if bin.lower > previous { return Err("POSTERIOR_BINS_GAPPED"); }
        }
        previous_upper = Some(bin.upper);
        total += bin.mass;
    }
    if !total.is_finite() || (total - 1.0).abs() > TOL {
        return Err("POSTERIOR_MASS_NOT_NORMALIZED");
    }
    Ok(())
}

fn probability_bounds(r: &Request) -> Result<(f64, f64), &'static str> {
    validate(r)?;
    let mut represented = 0.0_f64;
    for bin in &r.posterior.bins {
        let overlap = (bin.upper.min(r.spec.upper) - bin.lower.max(r.spec.lower)).max(0.0);
        represented += bin.mass * overlap / (bin.upper - bin.lower);
    }
    if !represented.is_finite() || represented < -TOL || represented > 1.0 + TOL {
        return Err("CONFORMANCE_INTEGRAL_INVALID");
    }
    let lower = represented.clamp(0.0, 1.0);
    let upper = (lower + r.posterior.unknown_tail_mass).clamp(lower, 1.0);
    Ok((lower, upper))
}

fn classify(r: &Request) -> Report {
    if let Err(reason) = validate(r) { return incomplete(r, reason); }
    let (lower, upper) = match probability_bounds(r) {
        Ok(v) => v,
        Err(reason) => return incomplete(r, reason),
    };
    let (outcome, reason) = if lower >= r.policy.min_accept_pc {
        (Outcome::Accept, "PC_LOWER_BOUND_MEETS_ACCEPT_THRESHOLD")
    } else if upper <= r.policy.max_reject_pc {
        (Outcome::Reject, "PC_UPPER_BOUND_MEETS_REJECT_THRESHOLD")
    } else {
        (Outcome::Indeterminate, "PC_ENCLOSURE_CROSSES_POLICY_REGION")
    };
    Report {
        posterior_id: r.posterior.id.clone(), measurement_id: r.posterior.measurement_id.clone(),
        part_id: r.posterior.part_id.clone(), lot_id: r.posterior.lot_id.clone(),
        spec_id: r.spec.id.clone(), spec_revision: r.spec.revision.clone(),
        policy_id: r.policy.id.clone(), policy_revision: r.policy.revision.clone(),
        uncertainty_id: r.provenance.uncertainty_budget_id.clone(),
        model_revision: r.posterior.model_revision.clone(),
        posterior_sha256: r.posterior.sha256.clone(), spec_sha256: r.spec.sha256.clone(),
        policy_sha256: r.policy.sha256.clone(),
        uncertainty_sha256: r.provenance.uncertainty_sha256.clone(),
        calibration_sha256: r.provenance.calibration_sha256.clone(),
        msa_sha256: r.provenance.msa_sha256.clone(),
        calibration_id: r.posterior.calibration_id.clone(),
        msa_study_id: r.posterior.msa_study_id.clone(), policy_owner: r.policy.owner.clone(),
        spec_lower: r.spec.lower, spec_upper: r.spec.upper,
        min_accept_pc: r.policy.min_accept_pc, max_reject_pc: r.policy.max_reject_pc,
        decision_time_ns: r.decision_time_ns,
        measurand: r.posterior.measurand.clone(), unit: r.posterior.unit.clone(),
        outcome, reason, pc_model: Some(lower), pc_lower: Some(lower), pc_upper: Some(upper),
        consumer_risk_upper: (outcome == Outcome::Accept).then_some(1.0 - lower),
        producer_risk_upper: (outcome == Outcome::Reject).then_some(upper),
        tail_mass: Some(r.posterior.unknown_tail_mass), bin_count: r.posterior.bins.len(),
        evidence: r.evidence,
    }
}

fn base(bins: Vec<Bin>, tail: f64) -> Request {
    Request {
        posterior: Posterior {
            id: "POST-0001".into(), measurement_id: "MEAS-0001".into(),
            part_id: "PART-0001".into(), lot_id: "LOT-0001".into(),
            measurand: "shaft-diameter".into(), unit: "mm".into(), spec_revision: "REV-7".into(),
            calibration_id: "CAL-CMM-01".into(), msa_study_id: "MSA-CROSSED-01".into(),
            model_revision: "POSTERIOR-MODEL-1".into(), sha256: "a".repeat(HASH_LEN),
            bins, unknown_tail_mass: tail,
        },
        spec: Specification {
            id: "SPEC-SHAFT-01".into(), revision: "REV-7".into(),
            measurand: "shaft-diameter".into(), unit: "mm".into(), lower: -1.0, upper: 2.0,
            effective_from_ns: 1_600_000_000, effective_until_ns: 1_900_000_000,
            sha256: "b".repeat(HASH_LEN),
        },
        policy: Policy {
            id: "POLICY-QUALITY-01".into(), revision: "POLICY-4".into(),
            owner: "QUALITY-AUTHORITY-01".into(), min_accept_pc: 0.95, max_reject_pc: 0.05,
            effective_from_ns: 1_600_000_000, effective_until_ns: 1_900_000_000,
            approved: true, sha256: "c".repeat(HASH_LEN),
        },
        provenance: Provenance {
            uncertainty_budget_id: "UQ-BUDGET-01".into(),
            uncertainty_sha256: "d".repeat(HASH_LEN),
            calibration_sha256: "e".repeat(HASH_LEN), msa_sha256: "f".repeat(HASH_LEN),
        },
        evidence: Evidence {
            posterior: EvidenceState::Verified, specification: EvidenceState::Verified,
            calibration: EvidenceState::Verified, msa: EvidenceState::Verified,
            uncertainty: EvidenceState::Verified, policy: EvidenceState::Verified,
        },
        decision_time_ns: 1_750_000_000,
    }
}

fn campaign() -> Vec<(&'static str, Request)> {
    vec![
        ("asymmetric_skewed_accept", base(vec![
            Bin { lower: -3.0, upper: -2.0, mass: 0.01 },
            Bin { lower: -2.0, upper: -1.0, mass: 0.01 },
            Bin { lower: -1.0, upper: 0.0, mass: 0.10 },
            Bin { lower: 0.0, upper: 1.0, mass: 0.70 },
            Bin { lower: 1.0, upper: 2.0, mass: 0.15 },
            Bin { lower: 2.0, upper: 5.0, mass: 0.03 },
        ], 0.0)),
        ("multimodal_indeterminate", base(vec![
            Bin { lower: -3.0, upper: -2.0, mass: 0.25 },
            Bin { lower: -0.5, upper: 0.5, mass: 0.50 },
            Bin { lower: 2.0, upper: 3.0, mass: 0.25 },
        ], 0.0)),
        ("out_of_spec_reject", base(vec![
            Bin { lower: 0.0, upper: 1.0, mass: 0.02 },
            Bin { lower: 2.0, upper: 3.0, mass: 0.98 },
        ], 0.0)),
        ("unknown_tail_abstains", base(vec![
            Bin { lower: -1.0, upper: 1.0, mass: 0.98 },
        ], 0.02)),
        ("uniform_half_inside", base(vec![
            Bin { lower: -2.0, upper: 2.0, mass: 1.0 },
        ], 0.0)),
    ]
}

fn verify() -> Result<(), String> {
    let expected = [
        (Outcome::Accept, 0.95, 0.95),
        (Outcome::Indeterminate, 0.50, 0.50),
        (Outcome::Reject, 0.02, 0.02),
        (Outcome::Indeterminate, 0.98, 1.00),
        (Outcome::Indeterminate, 0.75, 0.75),
    ];
    for ((name, r), (expected_outcome, expected_lower, expected_upper)) in
        campaign().iter().zip(expected.iter())
    {
        let report = classify(r);
        if report.outcome != *expected_outcome {
            return Err(format!("GOLDEN_OUTCOME_MISMATCH {name}: {} != {}",
                report.outcome.code(), expected_outcome.code()));
        }
        let lo = report.pc_lower.ok_or("PC_LOWER_MISSING")?;
        let hi = report.pc_upper.ok_or("PC_UPPER_MISSING")?;
        if (lo - expected_lower).abs() > TOL || (hi - expected_upper).abs() > TOL {
            return Err(format!("GOLDEN_PC_MISMATCH {name}: [{lo:.16e},{hi:.16e}]"));
        }
    }
    Ok(())
}

fn emit_csv() {
    println!("case_id,posterior_id,measurement_id,part_id,lot_id,specification_id,spec_revision,policy_id,policy_revision,uncertainty_budget_id,model_revision,posterior_sha256,spec_sha256,policy_sha256,uncertainty_sha256,calibration_sha256,msa_sha256,calibration_id,msa_study_id,policy_owner,spec_lower,spec_upper,min_accept_pc,max_reject_pc,decision_time_ns,measurand,unit,outcome,reason,pc_model,pc_lower,pc_upper,consumer_risk_upper,producer_risk_upper,tail_mass,bin_count,posterior_evidence,spec_evidence,calibration_evidence,msa_evidence,uncertainty_evidence,policy_evidence");
    for (name, r) in campaign() { println!("{name},{}", classify(&r).csv_row()); }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.as_slice() {
        [] => verify().map(|()| println!("MFG_RISK_OK: posterior integration and bounded conformity risk")),
        [a] if a == "--verify" => verify().map(|()| println!("MFG_RISK_OK: posterior integration and bounded conformity risk")),
        [a] if a == "--csv" => { emit_csv(); Ok(()) },
        [a] if a == "--help" || a == "-h" => { println!("Usage: mfg-risk [--verify|--csv|--help]"); Ok(()) },
        _ => Err("expected --verify, --csv or --help".into()),
    };
    if let Err(error) = result { eprintln!("MFG_RISK_ERROR: {error}"); std::process::exit(1); }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asymmetric_posterior_meets_acceptance_threshold() {
        let r = classify(&campaign()[0].1);
        assert_eq!(r.outcome, Outcome::Accept);
        assert!((r.pc_lower.unwrap() - 0.95).abs() < TOL);
        assert!((r.consumer_risk_upper.unwrap() - 0.05).abs() < TOL);
    }

    #[test]
    fn multimodal_posterior_does_not_assume_gaussian_shape() {
        let r = classify(&campaign()[1].1);
        assert_eq!(r.outcome, Outcome::Indeterminate);
        assert!((r.pc_lower.unwrap() - 0.50).abs() < TOL);
    }

    #[test]
    fn posterior_outside_specification_meets_rejection_threshold() {
        let r = classify(&campaign()[2].1);
        assert_eq!(r.outcome, Outcome::Reject);
        assert!((r.producer_risk_upper.unwrap() - 0.02).abs() < TOL);
    }

    #[test]
    fn unknown_tail_mass_widens_bounds_and_forces_abstention() {
        let r = classify(&campaign()[3].1);
        assert_eq!(r.outcome, Outcome::Indeterminate);
        assert!((r.pc_lower.unwrap() - 0.98).abs() < TOL);
        assert!((r.pc_upper.unwrap() - 1.0).abs() < TOL);
    }

    #[test]
    fn uniform_histogram_probability_is_integrated_exactly() {
        let (lo, hi) = probability_bounds(&campaign()[4].1).unwrap();
        assert!((lo - 0.75).abs() < TOL);
        assert!((hi - 0.75).abs() < TOL);
    }

    #[test]
    fn bin_refinement_preserves_piecewise_uniform_probability() {
        let original = campaign()[0].1.clone();
        let mut refined = original.clone();
        let i = refined.posterior.bins.iter()
            .position(|b| b.lower == -1.0 && b.upper == 0.0).unwrap();
        refined.posterior.bins.remove(i);
        refined.posterior.bins.insert(i, Bin { lower: -0.5, upper: 0.0, mass: 0.05 });
        refined.posterior.bins.insert(i, Bin { lower: -1.0, upper: -0.5, mass: 0.05 });
        let a = probability_bounds(&original).unwrap();
        let b = probability_bounds(&refined).unwrap();
        assert!((a.0 - b.0).abs() < TOL);
        assert!((a.1 - b.1).abs() < TOL);
        assert_eq!(classify(&original).outcome, classify(&refined).outcome);
    }

    #[test]
    fn invalid_probability_mass_bin_geometry_and_gaps_reject() {
        let mut r = campaign()[0].1.clone();
        r.posterior.bins[0].mass = -0.1;
        assert_eq!(classify(&r).reason, "POSTERIOR_BIN_MASS_INVALID");

        let mut r = campaign()[0].1.clone();
        r.posterior.bins[2].lower = -1.1;
        assert_eq!(classify(&r).reason, "POSTERIOR_BINS_OVERLAP_OR_UNSORTED");

        let mut r = campaign()[0].1.clone();
        r.posterior.bins[2].lower = -0.9;
        assert_eq!(classify(&r).reason, "POSTERIOR_BINS_GAPPED");

        let mut r = campaign()[0].1.clone();
        r.posterior.unknown_tail_mass = 0.1;
        assert_eq!(classify(&r).reason, "POSTERIOR_MASS_NOT_NORMALIZED");
    }

    #[test]
    fn invalid_policy_and_unapproved_thresholds_fail_closed() {
        let mut r = campaign()[0].1.clone();
        r.policy.min_accept_pc = 0.01;
        r.policy.max_reject_pc = 0.05;
        assert_eq!(classify(&r).reason, "DECISION_THRESHOLDS_INVALID");

        let mut r = campaign()[0].1.clone();
        r.policy.approved = false;
        assert_eq!(classify(&r).reason, "DECISION_POLICY_NOT_APPROVED");
    }

    #[test]
    fn unit_spec_revision_and_effectivity_mismatches_fail_closed() {
        let mut r = campaign()[0].1.clone();
        r.spec.unit = "inch".into();
        assert_eq!(classify(&r).reason, "UNIT_MISMATCH");

        let mut r = campaign()[0].1.clone();
        r.posterior.spec_revision = "REV-OLD".into();
        assert_eq!(classify(&r).reason, "SPECIFICATION_REVISION_MISMATCH");

        let mut r = campaign()[0].1.clone();
        r.decision_time_ns = r.policy.effective_until_ns + 1;
        assert_eq!(classify(&r).reason, "DECISION_POLICY_NOT_EFFECTIVE");
    }

    #[test]
    fn stale_and_missing_evidence_never_return_product_disposition() {
        let mut r = campaign()[0].1.clone();
        r.evidence.msa = EvidenceState::Stale;
        assert_eq!(classify(&r).outcome, Outcome::EvidenceIncomplete);

        let mut r = campaign()[2].1.clone();
        r.evidence.posterior = EvidenceState::Missing;
        assert_eq!(classify(&r).outcome, Outcome::EvidenceIncomplete);
    }

    #[test]
    fn posterior_digest_change_changes_report_bytes() {
        let original = classify(&campaign()[0].1).csv_row();
        let mut changed_request = campaign()[0].1.clone();
        changed_request.posterior.sha256 = "9".repeat(HASH_LEN);
        let changed = classify(&changed_request).csv_row();
        assert_ne!(original.as_bytes(), changed.as_bytes());
    }

    #[test]
    fn unit_conversion_preserves_probability() {
        let mm = campaign()[0].1.clone();
        let mut um = mm.clone();
        for bin in &mut um.posterior.bins { bin.lower *= 1000.0; bin.upper *= 1000.0; }
        um.posterior.unit = "um".into();
        um.spec.unit = "um".into();
        let a = probability_bounds(&mm).unwrap();
        let b = probability_bounds(&um).unwrap();
        assert!((a.0 - b.0).abs() < TOL);
        assert!((a.1 - b.1).abs() < TOL);
    }

    #[test]
    fn frozen_campaign_is_deterministic() {
        verify().unwrap();
        assert_eq!(classify(&campaign()[0].1).csv_row(), classify(&campaign()[0].1).csv_row());
    }
}
