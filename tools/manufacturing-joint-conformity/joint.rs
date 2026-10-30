//! Dependence-robust joint part-conformity bounds.
//! No independence assumption is used for the decision.
//! The independence product is emitted only as a diagnostic comparator.
//!
//! rustc --edition=2024 -D warnings --test tools/manufacturing-joint-conformity/joint.rs -o /tmp/mfg-joint-tests
//! rustc --edition=2024 -D warnings tools/manufacturing-joint-conformity/joint.rs -o /tmp/mfg-joint

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
    fn failure(self) -> Option<&'static str> {
        match self {
            Self::Verified => None,
            Self::Missing => Some("MARGINAL_EVIDENCE_MISSING"),
            Self::Stale => Some("MARGINAL_EVIDENCE_STALE"),
            Self::Mismatched => Some("MARGINAL_EVIDENCE_MISMATCHED"),
            Self::Revoked => Some("MARGINAL_EVIDENCE_REVOKED"),
            Self::NotQualified => Some("MARGINAL_NOT_QUALIFIED"),
        }
    }
}

#[derive(Clone, Debug)]
struct Marginal {
    characteristic_id: String,
    measurand_id: String,
    unit: String,
    specification_revision: String,
    probability_lower: f64,
    probability_upper: f64,
    source_sha256: String,
    evidence: EvidenceState,
}

#[derive(Clone, Debug)]
struct Request {
    part_id: String,
    lot_id: String,
    operation_id: String,
    policy_id: String,
    policy_revision: String,
    policy_owner: String,
    min_accept_joint_pc: f64,
    max_reject_joint_pc: f64,
    policy_approved: bool,
    decision_time_ns: i64,
    policy_from_ns: i64,
    policy_until_ns: i64,
    required_characteristics: Vec<String>,
    manifest_sha256: String,
    marginals: Vec<Marginal>,
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
    part_id: String, lot_id: String, operation_id: String,
    policy_id: String, policy_revision: String, policy_owner: String,
    manifest_sha256: String, characteristic_count: usize,
    outcome: Outcome, reason: &'static str,
    joint_lower: Option<f64>, joint_upper: Option<f64>,
    independence_product_midpoint: Option<f64>,
    min_accept_joint_pc: f64, max_reject_joint_pc: f64,
    decision_time_ns: i64, marginal_receipts: Vec<String>,
}

impl Report {
    fn csv_row(&self) -> String {
        let f = |v: Option<f64>| v.map(|x| format!("{x:.12e}")).unwrap_or_default();
        let receipts = self.marginal_receipts.join(";");
        format!(
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
            self.part_id, self.lot_id, self.operation_id, self.policy_id,
            self.policy_revision, self.policy_owner, self.manifest_sha256,
            self.characteristic_count, self.outcome.code(), self.reason,
            f(self.joint_lower), f(self.joint_upper),
            f(self.independence_product_midpoint),
            self.min_accept_joint_pc, self.max_reject_joint_pc,
            // decision time and marginal receipts are appended below to keep the
            // formatter's positional fields auditable.
        ) + &format!(",{},{}", self.decision_time_ns, receipts)
    }
}

fn present(v: &str) -> bool { !v.trim().is_empty() }
fn valid_sha(v: &str) -> bool {
    v.len() == HASH_LEN && v.bytes().all(|b| b.is_ascii_hexdigit())
}

fn marginal_receipts(r: &Request) -> Vec<String> {
    let mut receipts: Vec<String> = r.marginals.iter().map(|m| format!(
        "{}:{}:{}:{}:{:.12e}:{:.12e}:{}:{}",
        m.characteristic_id, m.measurand_id, m.unit, m.specification_revision,
        m.probability_lower, m.probability_upper, m.source_sha256, m.evidence.code()
    )).collect();
    receipts.sort();
    receipts
}

fn incomplete(r: &Request, reason: &'static str) -> Report {
    Report {
        part_id: r.part_id.clone(), lot_id: r.lot_id.clone(), operation_id: r.operation_id.clone(),
        policy_id: r.policy_id.clone(), policy_revision: r.policy_revision.clone(),
        policy_owner: r.policy_owner.clone(), manifest_sha256: r.manifest_sha256.clone(),
        characteristic_count: r.marginals.len(), outcome: Outcome::EvidenceIncomplete, reason,
        joint_lower: None, joint_upper: None, independence_product_midpoint: None,
        min_accept_joint_pc: r.min_accept_joint_pc, max_reject_joint_pc: r.max_reject_joint_pc,
        decision_time_ns: r.decision_time_ns,
        marginal_receipts: marginal_receipts(r),
    }
}

fn validate(r: &Request) -> Result<(), &'static str> {
    for id in [&r.part_id, &r.lot_id, &r.operation_id, &r.policy_id,
               &r.policy_revision, &r.policy_owner] {
        if !present(id) { return Err("REQUIRED_IDENTITY_MISSING"); }
    }
    if !r.policy_approved { return Err("DECISION_POLICY_NOT_APPROVED"); }
    if !r.min_accept_joint_pc.is_finite() || !r.max_reject_joint_pc.is_finite()
        || r.min_accept_joint_pc < 0.0 || r.min_accept_joint_pc > 1.0
        || r.max_reject_joint_pc < 0.0 || r.max_reject_joint_pc > 1.0
        || r.max_reject_joint_pc >= r.min_accept_joint_pc {
        return Err("DECISION_THRESHOLDS_INVALID");
    }
    if r.policy_from_ns > r.policy_until_ns
        || r.decision_time_ns < r.policy_from_ns
        || r.decision_time_ns > r.policy_until_ns {
        return Err("DECISION_POLICY_NOT_EFFECTIVE");
    }
    if !valid_sha(&r.manifest_sha256) { return Err("CHARACTERISTIC_MANIFEST_DIGEST_INVALID"); }
    if r.marginals.len() < 2 { return Err("JOINT_CONFORMITY_REQUIRES_AT_LEAST_TWO_CHARACTERISTICS"); }
    if r.required_characteristics.len() != r.marginals.len() {
        return Err("REQUIRED_CHARACTERISTIC_SET_INCOMPLETE");
    }
    if r.required_characteristics.iter().any(|id| !present(id)) {
        return Err("REQUIRED_CHARACTERISTIC_ID_MISSING");
    }

    let mut ids = Vec::with_capacity(r.marginals.len());
    for m in &r.marginals {
        if let Some(reason) = m.evidence.failure() { return Err(reason); }
        if !present(&m.characteristic_id) || !present(&m.measurand_id)
            || !present(&m.unit) || !present(&m.specification_revision) {
            return Err("MARGINAL_IDENTITY_MISSING");
        }
        if !m.probability_lower.is_finite() || !m.probability_upper.is_finite()
            || m.probability_lower < 0.0 || m.probability_upper > 1.0
            || m.probability_lower > m.probability_upper {
            return Err("MARGINAL_PROBABILITY_INTERVAL_INVALID");
        }
        if !valid_sha(&m.source_sha256) { return Err("MARGINAL_SOURCE_DIGEST_INVALID"); }
        if ids.iter().any(|id: &String| id == &m.characteristic_id) {
            return Err("DUPLICATE_CHARACTERISTIC_ID");
        }
        ids.push(m.characteristic_id.clone());
    }

    let mut required = r.required_characteristics.clone();
    let mut observed = ids;
    required.sort();
    observed.sort();
    if required != observed { return Err("REQUIRED_CHARACTERISTIC_SET_MISMATCH"); }
    Ok(())
}

fn frechet_bounds(marginals: &[Marginal]) -> Result<(f64, f64, f64), &'static str> {
    if marginals.len() < 2 { return Err("JOINT_CONFORMITY_REQUIRES_AT_LEAST_TWO_CHARACTERISTICS"); }
    let mut sum_lower = 0.0_f64;
    let mut effective_characteristics = 0_usize;
    let mut upper = 1.0_f64;
    let mut product_midpoint = 1.0_f64;
    for m in marginals {
        if !m.probability_lower.is_finite() || !m.probability_upper.is_finite()
            || m.probability_lower < 0.0 || m.probability_upper > 1.0
            || m.probability_lower > m.probability_upper {
            return Err("MARGINAL_PROBABILITY_INTERVAL_INVALID");
        }
        // A [1,1] marginal is an identity event: it must not perturb the
        // Fréchet lower bound through floating-point add/subtract cancellation.
        if m.probability_lower != 1.0 || m.probability_upper != 1.0 {
            sum_lower += m.probability_lower;
            effective_characteristics += 1;
        }
        upper = upper.min(m.probability_upper);
        product_midpoint *= 0.5 * (m.probability_lower + m.probability_upper);
    }
    let lower = if effective_characteristics == 0 {
        1.0
    } else {
        (sum_lower - (effective_characteristics - 1) as f64).max(0.0).min(upper)
    };
    if !lower.is_finite() || !upper.is_finite() || !product_midpoint.is_finite()
        || lower > upper + TOL {
        return Err("FRECHET_BOUND_INVALID");
    }
    Ok((lower, upper, product_midpoint))
}

fn classify(r: &Request) -> Report {
    if let Err(reason) = validate(r) { return incomplete(r, reason); }
    let (lower, upper, independence_product) = match frechet_bounds(&r.marginals) {
        Ok(v) => v,
        Err(reason) => return incomplete(r, reason),
    };
    let (outcome, reason) = if lower >= r.min_accept_joint_pc {
        (Outcome::Accept, "FRECHET_LOWER_BOUND_MEETS_ACCEPT_THRESHOLD")
    } else if upper <= r.max_reject_joint_pc {
        (Outcome::Reject, "FRECHET_UPPER_BOUND_MEETS_REJECT_THRESHOLD")
    } else {
        (Outcome::Indeterminate, "FRECHET_ENCLOSURE_CROSSES_POLICY_REGION")
    };
    Report {
        part_id: r.part_id.clone(), lot_id: r.lot_id.clone(), operation_id: r.operation_id.clone(),
        policy_id: r.policy_id.clone(), policy_revision: r.policy_revision.clone(),
        policy_owner: r.policy_owner.clone(), manifest_sha256: r.manifest_sha256.clone(),
        characteristic_count: r.marginals.len(), outcome, reason,
        joint_lower: Some(lower), joint_upper: Some(upper),
        independence_product_midpoint: Some(independence_product),
        min_accept_joint_pc: r.min_accept_joint_pc, max_reject_joint_pc: r.max_reject_joint_pc,
        decision_time_ns: r.decision_time_ns,
        marginal_receipts: marginal_receipts(r),
    }
}

fn make_request(probabilities: &[(f64, f64)]) -> Request {
    let marginals: Vec<Marginal> = probabilities.iter().enumerate().map(|(i, (lo, hi))| Marginal {
        characteristic_id: format!("CHAR-{i:03}"),
        measurand_id: format!("MEASURAND-{i:03}"),
        unit: "mm".into(),
        specification_revision: "SPEC-REV-1".into(),
        probability_lower: *lo,
        probability_upper: *hi,
        source_sha256: format!("{:064x}", i + 1),
        evidence: EvidenceState::Verified,
    }).collect();
    Request {
        part_id: "PART-0001".into(), lot_id: "LOT-0001".into(),
        operation_id: "OP-FINAL-INSPECTION".into(),
        policy_id: "POLICY-PART-01".into(), policy_revision: "POLICY-3".into(),
        policy_owner: "QUALITY-AUTHORITY-01".into(),
        min_accept_joint_pc: 0.80, max_reject_joint_pc: 0.60,
        policy_approved: true, decision_time_ns: 1_750_000_000,
        policy_from_ns: 1_600_000_000, policy_until_ns: 1_900_000_000,
        required_characteristics: marginals.iter().map(|m| m.characteristic_id.clone()).collect(),
        manifest_sha256: "a".repeat(HASH_LEN), marginals,
    }
}

fn campaign() -> Vec<(&'static str, Request)> {
    vec![
        ("two_marginals_090", make_request(&[(0.9,0.9),(0.9,0.9)])),
        ("five_marginals_090", make_request(&[(0.9,0.9);5])),
        ("twenty_marginals_099", make_request(&[(0.99,0.99);20])),
        ("unknown_marginal", make_request(&[(0.99,0.99),(0.0,1.0),(0.99,0.99)])),
        ("perfect_added_characteristic", make_request(&[(0.9,0.9),(0.9,0.9),(1.0,1.0)])),
    ]
}

fn verify() -> Result<(), String> {
    let expected = [
        (0.8,0.9,Outcome::Accept),
        (0.5,0.9,Outcome::Indeterminate),
        (0.8,0.99,Outcome::Indeterminate),
        (0.0,0.99,Outcome::Indeterminate),
        (0.8,0.9,Outcome::Accept),
    ];
    for ((name, request), (lo,hi,outcome)) in campaign().iter().zip(expected.iter()) {
        let report = classify(request);
        if report.outcome != *outcome { return Err(format!("JOINT_OUTCOME_MISMATCH {name}")); }
        if (report.joint_lower.unwrap()-lo).abs()>TOL || (report.joint_upper.unwrap()-hi).abs()>TOL {
            return Err(format!("JOINT_BOUND_MISMATCH {name}"));
        }
    }
    Ok(())
}

fn emit_csv() {
    println!("case_id,part_id,lot_id,operation_id,policy_id,policy_revision,policy_owner,manifest_sha256,characteristic_count,outcome,reason,joint_lower,joint_upper,independence_product_midpoint,min_accept_joint_pc,max_reject_joint_pc,decision_time_ns,marginal_receipts");
    for (name, request) in campaign() { println!("{name},{}", classify(&request).csv_row()); }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.as_slice() {
        [] => verify().map(|()| println!("MFG_JOINT_OK: dependence-robust Frechet conformity bounds")),
        [a] if a == "--verify" => verify().map(|()| println!("MFG_JOINT_OK: dependence-robust Frechet conformity bounds")),
        [a] if a == "--csv" => { emit_csv(); Ok(()) },
        [a] if a == "--help" || a == "-h" => { println!("Usage: mfg-joint [--verify|--csv|--help]"); Ok(()) },
        _ => Err("expected --verify, --csv or --help".into()),
    };
    if let Err(error) = result { eprintln!("MFG_JOINT_ERROR: {error}"); std::process::exit(1); }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn split_circle_interval(start: f64, length: f64) -> Vec<(f64,f64)> {
        if length <= 0.0 { return Vec::new(); }
        let end = start + length;
        if end <= 1.0 { vec![(start,end)] }
        else { vec![(start,1.0),(0.0,end-1.0)] }
    }

    fn event_atoms(marginals: &[f64], minimize_intersection: bool) -> Vec<f64> {
        let n = marginals.len();
        let mut intervals: Vec<Vec<(f64,f64)>> = vec![Vec::new();n];
        if minimize_intersection {
            let mut offset: f64=0.0;
            for (i,p) in marginals.iter().enumerate() {
                let q=1.0-p;
                intervals[i]=split_circle_interval(offset.rem_euclid(1.0),q);
                offset += q;
            }
        } else {
            for (i,p) in marginals.iter().enumerate() {
                intervals[i]=vec![(0.0,*p)];
            }
        }
        let mut points=vec![0.0,1.0];
        for pieces in &intervals {
            for (a,b) in pieces { points.push(*a); points.push(*b); }
        }
        points.sort_by(|a,b| a.total_cmp(b));
        points.dedup_by(|a,b| (*a-*b).abs()<1.0e-14);
        let mut atoms=vec![0.0;1usize<<n];
        for edge in points.windows(2) {
            let a=edge[0]; let b=edge[1];
            if b<=a { continue; }
            let x=0.5*(a+b);
            let mut mask=0usize;
            for i in 0..n {
                let in_complement=intervals[i].iter().any(|(l,u)| x>=*l && x<*u);
                let event_holds=if minimize_intersection { !in_complement } else { x<marginals[i] };
                if event_holds { mask|=1usize<<i; }
            }
            atoms[mask]+=b-a;
        }
        atoms
    }

    fn atom_marginals(atoms:&[f64], n:usize)->Vec<f64> {
        (0..n).map(|i| atoms.iter().enumerate()
            .filter(|(mask,_)| *mask & (1usize<<i)!=0).map(|(_,mass)|*mass).sum()).collect()
    }

    fn all_event_probability(atoms:&[f64], n:usize)->f64 { atoms[(1usize<<n)-1] }

    #[test]
    fn known_frechet_bounds_for_two_five_and_twenty_characteristics() { verify().unwrap(); }

    #[test]
    fn independence_product_is_diagnostic_not_the_decision_bound() {
        let r=campaign()[1].1.clone();
        let report=classify(&r);
        assert!((report.independence_product_midpoint.unwrap()-0.59049).abs()<TOL);
        assert_eq!(report.outcome,Outcome::Indeterminate);
        assert!((report.joint_lower.unwrap()-0.5).abs()<TOL);
        assert!((report.joint_upper.unwrap()-0.9).abs()<TOL);
    }

    #[test]
    fn exhaustive_binary_atom_constructions_attain_both_frechet_bounds() {
        for p in [
            vec![0.9,0.9],
            vec![0.8,0.7,0.6],
            vec![0.2,0.4,0.6],
            vec![0.95,0.85,0.75,0.65],
        ] {
            let n=p.len();
            for minimize in [true,false] {
                let atoms=event_atoms(&p,minimize);
                assert!((atoms.iter().sum::<f64>()-1.0).abs()<1.0e-12);
                for (actual,expected) in atom_marginals(&atoms,n).iter().zip(p.iter()) {
                    assert!((actual-expected).abs()<1.0e-12);
                }
                let intersection=all_event_probability(&atoms,n);
                let (lo,hi,_)=frechet_bounds(&p.iter().map(|x|Marginal {
                    characteristic_id:"x".into(),measurand_id:"m".into(),unit:"u".into(),
                    specification_revision:"s".into(),probability_lower:*x,probability_upper:*x,
                    source_sha256:"a".repeat(HASH_LEN),evidence:EvidenceState::Verified,
                }).collect::<Vec<_>>()).unwrap();
                if minimize { assert!((intersection-lo).abs()<1.0e-12); }
                else { assert!((intersection-hi).abs()<1.0e-12); }
            }
        }
    }

    #[test]
    fn unknown_marginal_widens_result_and_cannot_be_omitted() {
        let r=campaign()[3].1.clone();
        let report=classify(&r);
        assert_eq!(report.outcome,Outcome::Indeterminate);
        assert_eq!(report.joint_lower,Some(0.0));
        assert_eq!(report.joint_upper,Some(0.99));
    }

    #[test]
    fn adding_a_characteristic_never_increases_joint_bounds() {
        let base=make_request(&[(0.9,0.9),(0.8,0.9)]);
        let extra=make_request(&[(0.9,0.9),(0.8,0.9),(0.7,0.8)]);
        let a=frechet_bounds(&base.marginals).unwrap();
        let b=frechet_bounds(&extra.marginals).unwrap();
        assert!(b.0<=a.0+TOL);
        assert!(b.1<=a.1+TOL);
    }

    #[test]
    fn adding_perfect_characteristic_preserves_bounds() {
        let a=frechet_bounds(&campaign()[0].1.marginals).unwrap();
        let b=frechet_bounds(&campaign()[4].1.marginals).unwrap();
        assert!((a.0-b.0).abs()<TOL);
        assert!((a.1-b.1).abs()<TOL);
    }

    #[test]
    fn duplicate_or_missing_required_characteristics_fail_closed() {
        let mut r=campaign()[0].1.clone();
        r.marginals[1].characteristic_id=r.marginals[0].characteristic_id.clone();
        assert_eq!(classify(&r).reason,"DUPLICATE_CHARACTERISTIC_ID");
        let mut r=campaign()[0].1.clone();
        r.required_characteristics.pop();
        assert_eq!(classify(&r).reason,"REQUIRED_CHARACTERISTIC_SET_INCOMPLETE");
    }

    #[test]
    fn mismatched_manifest_and_stale_evidence_fail_closed() {
        let mut r=campaign()[0].1.clone();
        r.required_characteristics[0]="NOT-IN-PRESENT-MARGINALS".into();
        assert_eq!(classify(&r).reason,"REQUIRED_CHARACTERISTIC_SET_MISMATCH");
        let mut r=campaign()[0].1.clone();
        r.marginals[0].evidence=EvidenceState::Stale;
        assert_eq!(classify(&r).outcome,Outcome::EvidenceIncomplete);
    }

    #[test]
    fn every_nonverified_evidence_state_abstains() {
        for state in [
            EvidenceState::Missing,
            EvidenceState::Stale,
            EvidenceState::Mismatched,
            EvidenceState::Revoked,
            EvidenceState::NotQualified,
        ] {
            let mut request = campaign()[0].1.clone();
            request.marginals[0].evidence = state;
            let report = classify(&request);
            assert_eq!(report.outcome, Outcome::EvidenceIncomplete);
            assert!(report.reason != "FRECHET_LOWER_BOUND_MEETS_ACCEPT_THRESHOLD");
        }
    }

    #[test]
    fn invalid_probabilities_and_policy_fail_closed() {
        let mut r=campaign()[0].1.clone();
        r.marginals[0].probability_lower=-0.1;
        assert_eq!(classify(&r).reason,"MARGINAL_PROBABILITY_INTERVAL_INVALID");
        let mut r=campaign()[0].1.clone();
        r.max_reject_joint_pc=0.9;
        assert_eq!(classify(&r).reason,"DECISION_THRESHOLDS_INVALID");
    }

    #[test]
    fn permutation_does_not_change_bounds_or_outcome() {
        let mut r=campaign()[1].1.clone();
        let a=classify(&r);
        r.marginals.reverse();
        r.required_characteristics.reverse();
        let b=classify(&r);
        assert!((a.joint_lower.unwrap()-b.joint_lower.unwrap()).abs()<TOL);
        assert!((a.joint_upper.unwrap()-b.joint_upper.unwrap()).abs()<TOL);
        assert_eq!(a.outcome,b.outcome);
        assert_eq!(a.csv_row(),b.csv_row());
    }
}
