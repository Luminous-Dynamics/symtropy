//! Exact finite-support probability-based conformity risk oracle.
//!
//! Synthetic reference for PWR-MFG-RISK-001 (#3072). The process prior and
//! measurement likelihood are declared separately; no PDF is inferred from U.
//! This does not authorize product release or establish real production risks.

const N: usize = 8;
const LSL: f64 = 10.0;
const USL: f64 = 20.0;
const MAX_SPECIFIC_CONSUMER_RISK: f64 = 0.15;
const MAX_SPECIFIC_PRODUCER_RISK: f64 = 0.25;
const MAX_GLOBAL_CONSUMER_RISK: f64 = 0.05;
const MAX_GLOBAL_PRODUCER_RISK: f64 = 0.05;
const TOL: f64 = 1.0e-12;

type Vector = [f64; N];
type Matrix = [[f64; N]; N];

const TRUE_VALUES: Vector = [8.5, 9.5, 10.0, 10.5, 15.0, 19.5, 20.5, 21.5];
const PROCESS_PRIOR: Vector = [0.05, 0.10, 0.10, 0.20, 0.20, 0.15, 0.10, 0.10];

// Rows: true value support. Columns: measured-value bins.
// This synthetic row-stochastic likelihood deliberately includes tails.
const MEASUREMENT_LIKELIHOOD: Matrix = [
    [0.60, 0.25, 0.10, 0.04, 0.01, 0.00, 0.00, 0.00],
    [0.25, 0.40, 0.22, 0.10, 0.03, 0.00, 0.00, 0.00],
    [0.10, 0.22, 0.36, 0.22, 0.08, 0.02, 0.00, 0.00],
    [0.04, 0.10, 0.22, 0.30, 0.22, 0.10, 0.02, 0.00],
    [0.01, 0.04, 0.10, 0.22, 0.30, 0.22, 0.10, 0.01],
    [0.00, 0.02, 0.08, 0.20, 0.30, 0.22, 0.13, 0.05],
    [0.00, 0.00, 0.04, 0.10, 0.22, 0.30, 0.24, 0.10],
    [0.00, 0.00, 0.01, 0.04, 0.10, 0.25, 0.30, 0.30],
];

#[derive(Clone, Copy, Debug)]
struct RiskPolicy {
    max_specific_consumer_risk: f64,
    max_specific_producer_risk: f64,
    max_global_consumer_risk: f64,
    max_global_producer_risk: f64,
}

impl RiskPolicy {
    const fn synthetic() -> Self {
        Self {
            max_specific_consumer_risk: MAX_SPECIFIC_CONSUMER_RISK,
            max_specific_producer_risk: MAX_SPECIFIC_PRODUCER_RISK,
            max_global_consumer_risk: MAX_GLOBAL_CONSUMER_RISK,
            max_global_producer_risk: MAX_GLOBAL_PRODUCER_RISK,
        }
    }

    fn validate(self) -> Result<(), String> {
        let values = [
            self.max_specific_consumer_risk,
            self.max_specific_producer_risk,
            self.max_global_consumer_risk,
            self.max_global_producer_risk,
        ];
        if values.iter().any(|v| !v.is_finite() || !(0.0..=1.0).contains(v)) {
            return Err("RISK_POLICY_INVALID: risk limits must be finite and in [0,1]".into());
        }
        let accept_pc = 1.0 - self.max_specific_consumer_risk;
        let reject_pc = self.max_specific_producer_risk;
        if accept_pc <= reject_pc {
            return Err("RISK_POLICY_AMBIGUOUS: acceptance and rejection posterior regions overlap".into());
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Action {
    Accept,
    Reject,
    Indeterminate,
}

impl Action {
    fn code(self) -> &'static str {
        match self {
            Self::Accept => "ACCEPT",
            Self::Reject => "REJECT_NONCONFORMING",
            Self::Indeterminate => "INDETERMINATE",
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct BinResult {
    observation_index: usize,
    observation_value: f64,
    observation_probability: f64,
    posterior_conformance_probability: f64,
    specific_consumer_risk: Option<f64>,
    specific_producer_risk: Option<f64>,
    action: Action,
}

#[derive(Clone, Copy, Debug)]
struct RiskSummary {
    accepted_mass: f64,
    rejected_mass: f64,
    indeterminate_mass: f64,
    global_consumer_risk: f64,
    global_producer_risk: f64,
    false_accept_rate_given_accept: Option<f64>,
    false_reject_rate_given_reject: Option<f64>,
}

#[derive(Clone, Debug)]
struct Evaluation {
    bins: [BinResult; N],
    summary: RiskSummary,
}

fn validate_distribution(p: Vector, name: &str) -> Result<(), String> {
    if p.iter().any(|v| !v.is_finite() || *v < 0.0 || *v > 1.0) {
        return Err(format!("{name}_INVALID: entries must be finite and in [0,1]"));
    }
    if (p.iter().sum::<f64>() - 1.0).abs() > TOL {
        return Err(format!("{name}_NOT_NORMALIZED"));
    }
    Ok(())
}

fn validate_likelihood(likelihood: Matrix) -> Result<(), String> {
    for (i, row) in likelihood.iter().enumerate() {
        if row.iter().any(|v| !v.is_finite() || *v < 0.0 || *v > 1.0) {
            return Err(format!("LIKELIHOOD_ROW_{i}_INVALID"));
        }
        if (row.iter().sum::<f64>() - 1.0).abs() > TOL {
            return Err(format!("LIKELIHOOD_ROW_{i}_NOT_NORMALIZED"));
        }
    }
    Ok(())
}

fn is_conforming(value: f64) -> bool {
    value >= LSL && value <= USL
}

fn observed_probabilities(prior: Vector, likelihood: Matrix) -> Vector {
    let mut q = [0.0; N];
    for j in 0..N {
        for i in 0..N {
            q[j] += prior[i] * likelihood[i][j];
        }
    }
    q
}

fn posterior_conformance(prior: Vector, likelihood: Matrix, q: Vector) -> Result<Vector, String> {
    let mut pc = [0.0; N];
    for j in 0..N {
        if q[j] <= 0.0 || !q[j].is_finite() {
            return Err(format!("OBSERVATION_BIN_{j}_HAS_ZERO_OR_INVALID_MASS"));
        }
        let mut conforming_mass = 0.0;
        for i in 0..N {
            if is_conforming(TRUE_VALUES[i]) {
                conforming_mass += prior[i] * likelihood[i][j];
            }
        }
        pc[j] = conforming_mass / q[j];
        if !pc[j].is_finite() || !(0.0..=1.0).contains(&pc[j]) {
            return Err(format!("POSTERIOR_CONFORMANCE_INVALID_AT_BIN_{j}"));
        }
    }
    Ok(pc)
}

fn choose_action(pc: f64, policy: RiskPolicy) -> Action {
    if 1.0 - pc <= policy.max_specific_consumer_risk {
        Action::Accept
    } else if pc <= policy.max_specific_producer_risk {
        Action::Reject
    } else {
        Action::Indeterminate
    }
}

fn evaluate(
    prior: Vector,
    likelihood: Matrix,
    policy: RiskPolicy,
) -> Result<Evaluation, String> {
    validate_distribution(prior, "PROCESS_PRIOR")?;
    validate_likelihood(likelihood)?;
    policy.validate()?;

    if TRUE_VALUES.windows(2).any(|v| v[0] >= v[1]) {
        return Err("TRUE_SUPPORT_MUST_BE_STRICTLY_INCREASING".into());
    }
    if TRUE_VALUES.iter().any(|v| !v.is_finite()) || !(LSL < USL) {
        return Err("SPECIFICATION_OR_SUPPORT_INVALID".into());
    }

    let q = observed_probabilities(prior, likelihood);
    if (q.iter().sum::<f64>() - 1.0).abs() > 10.0 * TOL {
        return Err("JOINT_OBSERVATION_MASS_NOT_CONSERVED".into());
    }
    let pc = posterior_conformance(prior, likelihood, q)?;
    let mut bins = [BinResult {
        observation_index: 0,
        observation_value: 0.0,
        observation_probability: 0.0,
        posterior_conformance_probability: 0.0,
        specific_consumer_risk: None,
        specific_producer_risk: None,
        action: Action::Indeterminate,
    }; N];

    let mut accepted_mass = 0.0;
    let mut rejected_mass = 0.0;
    let mut indeterminate_mass = 0.0;
    let mut global_consumer_risk = 0.0;
    let mut global_producer_risk = 0.0;

    for j in 0..N {
        let action = choose_action(pc[j], policy);
        bins[j] = BinResult {
            observation_index: j,
            observation_value: TRUE_VALUES[j],
            observation_probability: q[j],
            posterior_conformance_probability: pc[j],
            specific_consumer_risk: (action == Action::Accept).then_some(1.0 - pc[j]),
            specific_producer_risk: (action == Action::Reject).then_some(pc[j]),
            action,
        };

        match action {
            Action::Accept => {
                accepted_mass += q[j];
                for i in 0..N {
                    if !is_conforming(TRUE_VALUES[i]) {
                        global_consumer_risk += prior[i] * likelihood[i][j];
                    }
                }
            }
            Action::Reject => {
                rejected_mass += q[j];
                for i in 0..N {
                    if is_conforming(TRUE_VALUES[i]) {
                        global_producer_risk += prior[i] * likelihood[i][j];
                    }
                }
            }
            Action::Indeterminate => indeterminate_mass += q[j],
        }
    }

    let total_outcome_mass = accepted_mass + rejected_mass + indeterminate_mass;
    if (total_outcome_mass - 1.0).abs() > 20.0 * TOL {
        return Err("OUTCOME_PARTITION_NOT_CONSERVED".into());
    }
    if global_consumer_risk > policy.max_global_consumer_risk + TOL {
        return Err(format!(
            "GLOBAL_CONSUMER_RISK_LIMIT_EXCEEDED:{global_consumer_risk:.12e}"
        ));
    }
    if global_producer_risk > policy.max_global_producer_risk + TOL {
        return Err(format!(
            "GLOBAL_PRODUCER_RISK_LIMIT_EXCEEDED:{global_producer_risk:.12e}"
        ));
    }

    let summary = RiskSummary {
        accepted_mass,
        rejected_mass,
        indeterminate_mass,
        global_consumer_risk,
        global_producer_risk,
        false_accept_rate_given_accept: (accepted_mass > 0.0)
            .then_some(global_consumer_risk / accepted_mass),
        false_reject_rate_given_reject: (rejected_mass > 0.0)
            .then_some(global_producer_risk / rejected_mass),
    };
    Ok(Evaluation { bins, summary })
}

fn baseline_shared_risk(prior: Vector, likelihood: Matrix) -> Result<(f64, f64), String> {
    validate_distribution(prior, "PROCESS_PRIOR")?;
    validate_likelihood(likelihood)?;
    let mut false_accept = 0.0;
    let mut false_reject = 0.0;
    for i in 0..N {
        for j in 0..N {
            let measured_inside = TRUE_VALUES[j] >= LSL && TRUE_VALUES[j] <= USL;
            if measured_inside && !is_conforming(TRUE_VALUES[i]) {
                false_accept += prior[i] * likelihood[i][j];
            }
            if !measured_inside && is_conforming(TRUE_VALUES[i]) {
                false_reject += prior[i] * likelihood[i][j];
            }
        }
    }
    Ok((false_accept, false_reject))
}

fn verify() -> Result<(), String> {
    let evaluation = evaluate(PROCESS_PRIOR, MEASUREMENT_LIKELIHOOD, RiskPolicy::synthetic())?;
    let s = evaluation.summary;
    if (s.global_consumer_risk - 0.026).abs() > 1.0e-12 {
        return Err(format!("GLOBAL_CONSUMER_GOLDEN_MISMATCH:{:.12e}", s.global_consumer_risk));
    }
    if (s.global_producer_risk - 0.0095).abs() > 1.0e-12 {
        return Err(format!("GLOBAL_PRODUCER_GOLDEN_MISMATCH:{:.12e}", s.global_producer_risk));
    }
    if (s.accepted_mass - 0.182).abs() > 1.0e-12
        || (s.rejected_mass - 0.0495).abs() > 1.0e-12
        || (s.indeterminate_mass - 0.7685).abs() > 1.0e-12 {
        return Err("OUTCOME_MASS_GOLDEN_MISMATCH".into());
    }
    for bin in &evaluation.bins {
        if !(0.0..=1.0).contains(&bin.posterior_conformance_probability)
            || !(0.0..=1.0).contains(&bin.observation_probability) {
            return Err("BIN_PROBABILITY_OUT_OF_RANGE".into());
        }
        if bin.action == Action::Accept
            && bin.specific_consumer_risk.unwrap_or(1.0)
                > MAX_SPECIFIC_CONSUMER_RISK + TOL {
            return Err("ACCEPT_EXCEEDED_SPECIFIC_CONSUMER_RISK".into());
        }
        if bin.action == Action::Reject
            && bin.specific_producer_risk.unwrap_or(1.0)
                > MAX_SPECIFIC_PRODUCER_RISK + TOL {
            return Err("REJECT_EXCEEDED_SPECIFIC_PRODUCER_RISK".into());
        }
    }

    let (shared_rc, shared_rp) = baseline_shared_risk(PROCESS_PRIOR, MEASUREMENT_LIKELIHOOD)?;
    if shared_rc <= s.global_consumer_risk || shared_rp <= s.global_producer_risk {
        return Err("GUARDED_POLICY_DID_NOT_IMPROVE_BOTH_RISKS_OVER_SHARED_RISK_FIXTURE".into());
    }
    Ok(())
}

fn emit_csv() -> Result<(), String> {
    let evaluation = evaluate(PROCESS_PRIOR, MEASUREMENT_LIKELIHOOD, RiskPolicy::synthetic())?;
    let s = evaluation.summary;
    println!("row_type,index,true_or_observed_value,process_probability,measurement_bin_probability,posterior_conformance_probability,action,specific_consumer_risk,specific_producer_risk,global_consumer_risk,global_producer_risk,accepted_mass,rejected_mass,indeterminate_mass,false_accept_rate_given_accept,false_reject_rate_given_reject");
    for bin in evaluation.bins {
        println!(
            "measurement_bin,{},{:.12e},,{:.12e},{:.12e},{},{},{},{},{},{},{},{},{},{}",
            bin.observation_index,
            bin.observation_value,
            bin.observation_probability,
            bin.posterior_conformance_probability,
            bin.action.code(),
            bin.specific_consumer_risk.map(|v| format!("{v:.12e}")).unwrap_or_default(),
            bin.specific_producer_risk.map(|v| format!("{v:.12e}")).unwrap_or_default(),
            format!("{:.12e}", s.global_consumer_risk),
            format!("{:.12e}", s.global_producer_risk),
            format!("{:.12e}", s.accepted_mass),
            format!("{:.12e}", s.rejected_mass),
            format!("{:.12e}", s.indeterminate_mass),
            s.false_accept_rate_given_accept.map(|v| format!("{v:.12e}")).unwrap_or_default(),
            s.false_reject_rate_given_reject.map(|v| format!("{v:.12e}")).unwrap_or_default()
        );
    }
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.as_slice() {
        [] => verify().map(|()| println!("MFG_CONFORMITY_RISK_OK: specific/global producer-consumer risks and outcome conservation")),
        [a] if a == "--verify" => verify().map(|()| println!("MFG_CONFORMITY_RISK_OK: specific/global producer-consumer risks and outcome conservation")),
        [a] if a == "--csv" => emit_csv(),
        [a] if a == "--help" || a == "-h" => {
            println!("Usage: conformity-risk [--verify|--csv|--help]");
            Ok(())
        }
        _ => Err("expected --verify, --csv or --help".into()),
    };
    if let Err(e) = result {
        eprintln!("MFG_CONFORMITY_RISK_ERROR: {e}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity_likelihood() -> Matrix {
        let mut m = [[0.0; N]; N];
        for i in 0..N { m[i][i] = 1.0; }
        m
    }

    fn uninformative_likelihood() -> Matrix {
        [[1.0 / N as f64; N]; N]
    }

    #[test]
    fn process_and_likelihood_models_are_stochastic() {
        validate_distribution(PROCESS_PRIOR, "PRIOR").unwrap();
        validate_likelihood(MEASUREMENT_LIKELIHOOD).unwrap();
    }

    #[test]
    fn posterior_and_joint_mass_are_normalized() {
        let e = evaluate(PROCESS_PRIOR, MEASUREMENT_LIKELIHOOD, RiskPolicy::synthetic()).unwrap();
        assert!((e.bins.iter().map(|b| b.observation_probability).sum::<f64>() - 1.0).abs() < TOL);
        assert!((e.summary.accepted_mass + e.summary.rejected_mass + e.summary.indeterminate_mass - 1.0).abs() < TOL);
        assert!(e.bins.iter().all(|b| (0.0..=1.0).contains(&b.posterior_conformance_probability)));
    }

    #[test]
    fn specific_risks_and_global_risks_match_exact_oracle() {
        let e = evaluate(PROCESS_PRIOR, MEASUREMENT_LIKELIHOOD, RiskPolicy::synthetic()).unwrap();
        let accepted = e.bins.iter().find(|b| b.action == Action::Accept).unwrap();
        let rejected = e.bins.iter().find(|b| b.action == Action::Reject).unwrap();
        assert!((accepted.specific_consumer_risk.unwrap() - (1.0 - 0.8571428571428571)).abs() < TOL);
        assert!((rejected.specific_producer_risk.unwrap() - 0.1919191919191919).abs() < TOL);
        assert!((e.summary.global_consumer_risk - 0.026).abs() < TOL);
        assert!((e.summary.global_producer_risk - 0.0095).abs() < TOL);
    }

    #[test]
    fn perfect_measurement_has_zero_false_accept_and_false_reject_risk() {
        let e = evaluate(PROCESS_PRIOR, identity_likelihood(), RiskPolicy::synthetic()).unwrap();
        assert_eq!(e.summary.global_consumer_risk, 0.0);
        assert_eq!(e.summary.global_producer_risk, 0.0);
        for (i, bin) in e.bins.iter().enumerate() {
            if is_conforming(TRUE_VALUES[i]) {
                assert_eq!(bin.action, Action::Accept);
            } else {
                assert_eq!(bin.action, Action::Reject);
            }
        }
    }

    #[test]
    fn uninformative_measurement_produces_identical_posteriors() {
        let e = evaluate(PROCESS_PRIOR, uninformative_likelihood(), RiskPolicy::synthetic()).unwrap();
        let first = e.bins[0].posterior_conformance_probability;
        assert!(e.bins.iter().all(|bin| (bin.posterior_conformance_probability - first).abs() < TOL));
    }

    #[test]
    fn process_prior_shift_changes_risk_without_changing_measurement_likelihood() {
        let mut shifted = PROCESS_PRIOR;
        shifted[0] = 0.10;
        shifted[7] = 0.05;
        shifted[4] = 0.20;
        shifted[3] = 0.20;
        shifted[5] = 0.15;
        shifted[6] = 0.10;
        shifted[1] = 0.10;
        shifted[2] = 0.10;
        let baseline = evaluate(PROCESS_PRIOR, MEASUREMENT_LIKELIHOOD, RiskPolicy::synthetic()).unwrap();
        let shifted_eval = evaluate(shifted, MEASUREMENT_LIKELIHOOD, RiskPolicy::synthetic()).unwrap();
        assert!((baseline.summary.global_consumer_risk - shifted_eval.summary.global_consumer_risk).abs() > 1.0e-6);
    }

    #[test]
    fn shared_risk_baseline_exposes_the_cost_of_binary_classification() {
        let (rc, rp) = baseline_shared_risk(PROCESS_PRIOR, MEASUREMENT_LIKELIHOOD).unwrap();
        let robust = evaluate(PROCESS_PRIOR, MEASUREMENT_LIKELIHOOD, RiskPolicy::synthetic()).unwrap();
        assert!(rc > robust.summary.global_consumer_risk);
        assert!(rp > robust.summary.global_producer_risk);
    }

    #[test]
    fn invalid_priors_likelihoods_and_policy_thresholds_reject() {
        assert!(validate_distribution([0.1; N], "bad").is_err());
        let mut bad = MEASUREMENT_LIKELIHOOD;
        bad[0][0] = -0.1;
        assert!(validate_likelihood(bad).is_err());
        let mut bad = MEASUREMENT_LIKELIHOOD;
        bad[0][0] = f64::NAN;
        assert!(validate_likelihood(bad).is_err());
        let bad_policy = RiskPolicy {
            max_specific_consumer_risk: 0.3,
            max_specific_producer_risk: 0.8,
            ..RiskPolicy::synthetic()
        };
        assert!(bad_policy.validate().is_err());
    }

    #[test]
    fn risk_limits_fail_closed_when_too_tight() {
        let too_tight = RiskPolicy {
            max_global_consumer_risk: 0.001,
            ..RiskPolicy::synthetic()
        };
        assert!(evaluate(PROCESS_PRIOR, MEASUREMENT_LIKELIHOOD, too_tight).is_err());
    }

    #[test]
    fn deterministic_evaluation_and_report() {
        let a = evaluate(PROCESS_PRIOR, MEASUREMENT_LIKELIHOOD, RiskPolicy::synthetic()).unwrap();
        let b = evaluate(PROCESS_PRIOR, MEASUREMENT_LIKELIHOOD, RiskPolicy::synthetic()).unwrap();
        assert_eq!(a.bins[3].posterior_conformance_probability, b.bins[3].posterior_conformance_probability);
        verify().unwrap();
    }
}
