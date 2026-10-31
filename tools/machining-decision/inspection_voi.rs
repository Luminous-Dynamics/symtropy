//! Exact finite-state first-passage risk and inspection value-of-information oracle.
//!
//! Synthetic decision-math reference for MACH-DECISION-001 (#2977).
//! It does not predict real wear, qualify a machine, or authorize maintenance.
//!
//! Compile directly:
//!   rustc --edition=2024 -D warnings --test tools/machining-decision/inspection_voi.rs -o /tmp/inspection-voi-tests
//!   rustc --edition=2024 -D warnings tools/machining-decision/inspection_voi.rs -o /tmp/inspection-voi
//!   /tmp/inspection-voi --verify
//!   /tmp/inspection-voi --csv

const STATES: usize = 5;
const HORIZON_CYCLES: usize = 4;
const RUN_COST: f64 = 2.0;
const FAILURE_LOSS: f64 = 250.0;
const REPLACEMENT_COST: f64 = 65.0;
const INSPECTION_COST: f64 = 8.0;
const MEAN_WEAR_REPLACE_THRESHOLD: f64 = 2.5;
const TOL: f64 = 1.0e-12;

type Distribution = [f64; STATES];
type Matrix = [[f64; STATES]; STATES];

const NORMAL_TRANSITION: Matrix = [
    [0.88, 0.12, 0.00, 0.00, 0.00],
    [0.00, 0.82, 0.18, 0.00, 0.00],
    [0.00, 0.00, 0.78, 0.22, 0.00],
    [0.00, 0.00, 0.00, 0.65, 0.35],
    [0.00, 0.00, 0.00, 0.00, 1.00],
];

const HIGH_LOAD_TRANSITION: Matrix = [
    [0.65, 0.35, 0.00, 0.00, 0.00],
    [0.00, 0.55, 0.45, 0.00, 0.00],
    [0.00, 0.00, 0.40, 0.60, 0.00],
    [0.00, 0.00, 0.00, 0.20, 0.80],
    [0.00, 0.00, 0.00, 0.00, 1.00],
];

// Rows are true discrete wear states; columns are inspection readings.
const INSPECTION_LIKELIHOOD: Matrix = [
    [0.85, 0.12, 0.03, 0.00, 0.00],
    [0.10, 0.75, 0.13, 0.02, 0.00],
    [0.02, 0.12, 0.72, 0.12, 0.02],
    [0.00, 0.02, 0.13, 0.75, 0.10],
    [0.00, 0.00, 0.03, 0.12, 0.85],
];

#[derive(Clone, Copy, Debug)]
struct Scenario {
    id: &'static str,
    regime: &'static str,
    prior: Distribution,
    transition: Matrix,
    inspection: Matrix,
}

impl Scenario {
    fn all() -> [Self; 5] {
        [
            Self { id: "N01_low_wear_normal", regime: "normal",
                prior: [0.75, 0.20, 0.05, 0.00, 0.00],
                transition: NORMAL_TRANSITION, inspection: INSPECTION_LIKELIHOOD },
            Self { id: "N02_mid_uncertain_normal", regime: "normal",
                prior: [0.08, 0.22, 0.35, 0.25, 0.10],
                transition: NORMAL_TRANSITION, inspection: INSPECTION_LIKELIHOOD },
            Self { id: "N03_late_wear_normal", regime: "normal",
                prior: [0.00, 0.05, 0.15, 0.35, 0.45],
                transition: NORMAL_TRANSITION, inspection: INSPECTION_LIKELIHOOD },
            Self { id: "H01_mid_wear_high_load", regime: "high_load",
                prior: [0.25, 0.35, 0.25, 0.10, 0.05],
                transition: HIGH_LOAD_TRANSITION, inspection: INSPECTION_LIKELIHOOD },
            Self { id: "H02_late_wear_high_load", regime: "high_load",
                prior: [0.05, 0.10, 0.20, 0.30, 0.35],
                transition: HIGH_LOAD_TRANSITION, inspection: INSPECTION_LIKELIHOOD },
        ]
    }

    fn validate(self) -> Result<(), String> {
        if self.id.trim().is_empty() || self.regime.trim().is_empty() {
            return Err("SCENARIO_ID_INVALID: scenario identity is required".into());
        }
        validate_distribution(self.prior)?;
        validate_transition(self.transition)?;
        validate_likelihood(self.inspection)?;
        for value in [RUN_COST, FAILURE_LOSS, REPLACEMENT_COST, INSPECTION_COST] {
            if !value.is_finite() || value < 0.0 {
                return Err("COST_INVALID: all costs must be finite and nonnegative".into());
            }
        }
        if REPLACEMENT_COST <= RUN_COST {
            return Err("COST_INVALID: replacement cost must exceed ordinary run cost".into());
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Action {
    Continue,
    Replace,
    Inspect,
}

impl Action {
    fn as_str(self) -> &'static str {
        match self {
            Self::Continue => "CONTINUE",
            Self::Replace => "REPLACE",
            Self::Inspect => "INSPECT",
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Evaluation {
    mean_wear_state: f64,
    first_passage_probability: f64,
    continue_expected_cost: f64,
    replace_cost: f64,
    risk_action: Action,
    risk_policy_cost: f64,
    mean_policy_action: Action,
    mean_policy_cost: f64,
    always_inspect_cost: f64,
    net_evsi: f64,
    probability_inspection_changes_action: f64,
    evsi_action: Action,
    evsi_policy_cost: f64,
    clairvoyant_lower_bound: f64,
    regret_to_clairvoyant: f64,
}

fn validate_distribution(p: Distribution) -> Result<(), String> {
    if p.iter().any(|x| !x.is_finite() || *x < 0.0 || *x > 1.0) {
        return Err("PROBABILITY_INVALID: distribution entries must be finite and in [0,1]".into());
    }
    let sum = p.iter().sum::<f64>();
    if (sum - 1.0).abs() > TOL {
        return Err(format!("PROBABILITY_NOT_NORMALIZED: sum={sum:.16e}"));
    }
    Ok(())
}

fn validate_probability_vector(p: Distribution) -> Result<(), String> {
    if p.iter().any(|x| !x.is_finite() || *x < 0.0 || *x > 1.0) {
        return Err("EVENT_PROBABILITY_INVALID: entries must be finite and in [0,1]".into());
    }
    Ok(())
}

fn validate_transition(m: Matrix) -> Result<(), String> {
    for (state, row) in m.iter().enumerate() {
        let sum = row.iter().sum::<f64>();
        if row.iter().any(|x| !x.is_finite() || *x < 0.0 || *x > 1.0) {
            return Err("TRANSITION_INVALID: entries must be finite and in [0,1]".into());
        }
        if (sum - 1.0).abs() > TOL {
            return Err("TRANSITION_NOT_NORMALIZED: each state row must sum to one".into());
        }
        for next_state in 0..state {
            if row[next_state] > TOL {
                return Err("WEAR_NONMONOTONE: transition permits wear to decrease".into());
            }
        }
    }
    if (m[STATES - 1][STATES - 1] - 1.0).abs() > TOL {
        return Err("EOL_NOT_ABSORBING: end-of-life state must be absorbing".into());
    }
    Ok(())
}

fn validate_likelihood(m: Matrix) -> Result<(), String> {
    for row in m {
        let sum = row.iter().sum::<f64>();
        if row.iter().any(|x| !x.is_finite() || *x < 0.0 || *x > 1.0) {
            return Err("LIKELIHOOD_INVALID: entries must be finite and in [0,1]".into());
        }
        if (sum - 1.0).abs() > TOL {
            return Err("LIKELIHOOD_NOT_NORMALIZED: each hidden-state row must sum to one".into());
        }
    }
    Ok(())
}

fn propagate(p: Distribution, transition: Matrix) -> Distribution {
    let mut next = [0.0; STATES];
    for from in 0..STATES {
        for to in 0..STATES {
            next[to] += p[from] * transition[from][to];
        }
    }
    next
}

fn first_passage_by_initial_state(transition: Matrix, horizon: usize) -> Result<Distribution, String> {
    validate_transition(transition)?;
    if horizon == 0 {
        return Err("HORIZON_INVALID: first-passage horizon must be positive".into());
    }
    let mut risk = [0.0; STATES];
    for initial in 0..STATES {
        let mut p = [0.0; STATES];
        p[initial] = 1.0;
        for _ in 0..horizon {
            p = propagate(p, transition);
        }
        risk[initial] = p[STATES - 1];
    }
    validate_probability_vector(risk)?;
    Ok(risk)
}

fn posterior_after_observation(
    prior: Distribution,
    likelihood: Matrix,
    observed_bucket: usize,
) -> Result<(f64, Distribution), String> {
    validate_distribution(prior)?;
    validate_likelihood(likelihood)?;
    if observed_bucket >= STATES {
        return Err("OBSERVATION_INVALID: inspection bucket outside state range".into());
    }
    let mut weighted = [0.0; STATES];
    for state in 0..STATES {
        weighted[state] = prior[state] * likelihood[state][observed_bucket];
    }
    let evidence_probability = weighted.iter().sum::<f64>();
    if !evidence_probability.is_finite() || evidence_probability <= 0.0 {
        return Err("OBSERVATION_IMPOSSIBLE: evidence has zero probability under prior".into());
    }
    for value in &mut weighted {
        *value /= evidence_probability;
    }
    validate_distribution(weighted)?;
    Ok((evidence_probability, weighted))
}

fn expected_continue_cost(prior: Distribution, first_passage: Distribution) -> Result<f64, String> {
    validate_distribution(prior)?;
    validate_probability_vector(first_passage)?;
    let risk = prior.iter().zip(first_passage.iter())
        .map(|(p, r)| p * r).sum::<f64>();
    Ok(RUN_COST + risk * FAILURE_LOSS)
}

fn mean_wear(prior: Distribution) -> f64 {
    prior.iter().enumerate().map(|(state, p)| state as f64 * p).sum()
}

fn bayes_action(continue_cost: f64) -> (Action, f64) {
    if continue_cost <= REPLACEMENT_COST {
        (Action::Continue, continue_cost)
    } else {
        (Action::Replace, REPLACEMENT_COST)
    }
}

fn evaluate(scenario: Scenario) -> Result<Evaluation, String> {
    scenario.validate()?;
    let first_passage = first_passage_by_initial_state(scenario.transition, HORIZON_CYCLES)?;
    let expected_risk = scenario.prior.iter().zip(first_passage.iter())
        .map(|(p, q)| p * q).sum::<f64>();
    let continue_cost = expected_continue_cost(scenario.prior, first_passage)?;
    let (risk_action, risk_policy_cost) = bayes_action(continue_cost);

    let mean = mean_wear(scenario.prior);
    let mean_policy_action = if mean >= MEAN_WEAR_REPLACE_THRESHOLD {
        Action::Replace
    } else {
        Action::Continue
    };
    let mean_policy_cost = match mean_policy_action {
        Action::Replace => REPLACEMENT_COST,
        Action::Continue => continue_cost,
        Action::Inspect => unreachable!(),
    };

    let mut posterior_decision_cost = 0.0_f64;
    let mut action_change_probability = 0.0_f64;
    for observation in 0..STATES {
        let (p_observation, posterior) =
            posterior_after_observation(scenario.prior, scenario.inspection, observation)?;
        let posterior_continue_cost = expected_continue_cost(posterior, first_passage)?;
        let (posterior_action, best_cost) = bayes_action(posterior_continue_cost);
        posterior_decision_cost += p_observation * best_cost;
        if posterior_action != risk_action {
            action_change_probability += p_observation;
        }
    }

    let always_inspect_cost = INSPECTION_COST + posterior_decision_cost;
    let net_evsi = risk_policy_cost - always_inspect_cost;
    let (evsi_action, evsi_policy_cost) = if net_evsi > TOL {
        (Action::Inspect, always_inspect_cost)
    } else {
        (risk_action, risk_policy_cost)
    };

    let mut clairvoyant_lower_bound = 0.0_f64;
    for state in 0..STATES {
        let state_continue = RUN_COST + first_passage[state] * FAILURE_LOSS;
        clairvoyant_lower_bound += scenario.prior[state] * state_continue.min(REPLACEMENT_COST);
    }

    if !expected_risk.is_finite()
        || !always_inspect_cost.is_finite()
        || !net_evsi.is_finite()
        || evsi_policy_cost + TOL < clairvoyant_lower_bound
    {
        return Err("EVALUATION_INVALID: non-finite cost or policy beat the clairvoyant lower bound".into());
    }

    Ok(Evaluation {
        mean_wear_state: mean,
        first_passage_probability: expected_risk,
        continue_expected_cost: continue_cost,
        replace_cost: REPLACEMENT_COST,
        risk_action,
        risk_policy_cost,
        mean_policy_action,
        mean_policy_cost,
        always_inspect_cost,
        net_evsi,
        probability_inspection_changes_action: action_change_probability,
        evsi_action,
        evsi_policy_cost,
        clairvoyant_lower_bound,
        regret_to_clairvoyant: evsi_policy_cost - clairvoyant_lower_bound,
    })
}

fn scenarios() -> [Scenario; 5] { Scenario::all() }

fn verify() -> Result<(), String> {
    let mut saw_inspection = false;
    let mut saw_no_inspection = false;
    let mut saw_mean_policy_disagreement = false;

    for scenario in scenarios() {
        let evaluation = evaluate(scenario)?;
        saw_inspection |= evaluation.evsi_action == Action::Inspect;
        saw_no_inspection |= evaluation.evsi_action != Action::Inspect;
        saw_mean_policy_disagreement |= evaluation.mean_policy_action != evaluation.risk_action;

        if !(0.0..=1.0).contains(&evaluation.first_passage_probability)
            || !(0.0..=1.0).contains(&evaluation.probability_inspection_changes_action)
            || evaluation.evsi_policy_cost + TOL < evaluation.clairvoyant_lower_bound
        {
            return Err(format!("verification bounds failed for {}", scenario.id));
        }
        if evaluation.net_evsi > TOL
            && (evaluation.evsi_action != Action::Inspect
                || (evaluation.evsi_policy_cost - evaluation.always_inspect_cost).abs() > TOL)
        {
            return Err("positive net EVSI did not select inspection".into());
        }
        if evaluation.net_evsi <= TOL && evaluation.evsi_action == Action::Inspect {
            return Err("non-positive net EVSI selected inspection".into());
        }
    }

    if !saw_inspection || !saw_no_inspection || !saw_mean_policy_disagreement {
        return Err("scenario suite failed to exercise inspection, non-inspection and disagreement cases".into());
    }
    Ok(())
}

fn emit_csv() -> Result<(), String> {
    println!("scenario_id,regime,horizon_cycles,mean_wear_state,first_passage_probability,continue_expected_cost,replace_cost,risk_policy_action,risk_policy_cost,mean_threshold_action,mean_threshold_cost,always_inspect_cost,net_evsi,inspection_changes_action_probability,evsi_policy_action,evsi_policy_cost,clairvoyant_lower_bound,regret_to_clairvoyant,status");
    for scenario in scenarios() {
        let e = evaluate(scenario)?;
        println!(
            "{},{},{},{:.12e},{:.12e},{:.12e},{:.12e},{},{:.12e},{},{:.12e},{:.12e},{:.12e},{:.12e},{},{:.12e},{:.12e},{:.12e},PASS",
            scenario.id, scenario.regime, HORIZON_CYCLES,
            e.mean_wear_state, e.first_passage_probability, e.continue_expected_cost,
            e.replace_cost, e.risk_action.as_str(), e.risk_policy_cost,
            e.mean_policy_action.as_str(), e.mean_policy_cost, e.always_inspect_cost,
            e.net_evsi, e.probability_inspection_changes_action,
            e.evsi_action.as_str(), e.evsi_policy_cost, e.clairvoyant_lower_bound,
            e.regret_to_clairvoyant
        );
    }
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.as_slice() {
        [] => verify().map(|()| println!("MACH_DECISION_OK: first-passage risk and inspection EVSI")),
        [arg] if arg == "--verify" => verify().map(|()| println!("MACH_DECISION_OK: first-passage risk and inspection EVSI")),
        [arg] if arg == "--csv" => emit_csv(),
        [arg] if arg == "--help" || arg == "-h" => {
            println!("Usage: inspection-voi [--verify|--csv|--help]");
            Ok(())
        }
        _ => Err("expected --verify, --csv or --help".into()),
    };
    if let Err(error) = result {
        eprintln!("MACH_DECISION_ERROR: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transition_and_observation_models_are_stochastic() {
        validate_transition(NORMAL_TRANSITION).unwrap();
        validate_transition(HIGH_LOAD_TRANSITION).unwrap();
        validate_likelihood(INSPECTION_LIKELIHOOD).unwrap();
    }

    #[test]
    fn first_passage_risk_is_monotonic_in_wear_and_process_load() {
        let normal = first_passage_by_initial_state(NORMAL_TRANSITION, HORIZON_CYCLES).unwrap();
        let high = first_passage_by_initial_state(HIGH_LOAD_TRANSITION, HORIZON_CYCLES).unwrap();
        for state in 1..STATES {
            assert!(normal[state] + TOL >= normal[state - 1]);
            assert!(high[state] + TOL >= high[state - 1]);
            assert!(high[state] + TOL >= normal[state]);
        }
        assert_eq!(normal[STATES - 1], 1.0);
        assert_eq!(high[STATES - 1], 1.0);
    }

    #[test]
    fn posterior_is_normalized_for_every_possible_reading() {
        let prior = [0.08, 0.22, 0.35, 0.25, 0.10];
        for observation in 0..STATES {
            let (p_observation, posterior) =
                posterior_after_observation(prior, INSPECTION_LIKELIHOOD, observation).unwrap();
            assert!(p_observation > 0.0 && p_observation <= 1.0);
            assert!((posterior.iter().sum::<f64>() - 1.0).abs() < TOL);
        }
    }

    #[test]
    fn perfect_inspection_reaches_the_clairvoyant_decision_bound() {
        let mut perfect = [[0.0; STATES]; STATES];
        for state in 0..STATES {
            perfect[state][state] = 1.0;
        }
        let scenario = Scenario { inspection: perfect, ..Scenario::all()[1] };
        let evaluation = evaluate(scenario).unwrap();
        assert!((evaluation.always_inspect_cost
            - (INSPECTION_COST + evaluation.clairvoyant_lower_bound)).abs() < TOL);
        assert!(evaluation.net_evsi > 0.0);
        assert_eq!(evaluation.evsi_action, Action::Inspect);
    }

    #[test]
    fn uninformative_inspection_does_not_change_the_action() {
        let uninformative = [[0.2; STATES]; STATES];
        let scenario = Scenario { inspection: uninformative, ..Scenario::all()[1] };
        let evaluation = evaluate(scenario).unwrap();
        assert!((evaluation.net_evsi + INSPECTION_COST).abs() < TOL);
        assert_ne!(evaluation.evsi_action, Action::Inspect);
    }

    #[test]
    fn inspection_can_change_a_replacement_decision_when_information_is_valuable() {
        let evaluation = evaluate(Scenario::all()[1]).unwrap();
        assert_eq!(evaluation.risk_action, Action::Replace);
        assert_eq!(evaluation.evsi_action, Action::Inspect);
        assert!(evaluation.net_evsi > 0.0);
        assert!(evaluation.probability_inspection_changes_action > 0.0);
    }

    #[test]
    fn inspection_is_not_worthwhile_when_risk_is_low_or_state_is_late() {
        let low = evaluate(Scenario::all()[0]).unwrap();
        let late = evaluate(Scenario::all()[2]).unwrap();
        assert_eq!(low.evsi_action, Action::Continue);
        assert_eq!(late.evsi_action, Action::Replace);
        assert!(low.net_evsi < 0.0);
        assert!(late.net_evsi < 0.0);
    }

    #[test]
    fn operating_regime_changes_risk_even_at_identical_wear_posterior() {
        let base = Scenario::all()[3];
        let normal = Scenario { transition: NORMAL_TRANSITION, ..base };
        let high = Scenario { transition: HIGH_LOAD_TRANSITION, ..base };
        let normal_eval = evaluate(normal).unwrap();
        let high_eval = evaluate(high).unwrap();
        assert_eq!(normal_eval.mean_wear_state, high_eval.mean_wear_state);
        assert!(high_eval.first_passage_probability > normal_eval.first_passage_probability);
        assert_eq!(normal_eval.mean_policy_action, Action::Continue);
        assert_eq!(high_eval.mean_policy_action, Action::Continue);
        assert_eq!(high_eval.risk_action, Action::Replace);
    }

    #[test]
    fn evsi_policy_never_pays_for_information_when_it_does_not_reduce_total_cost() {
        for scenario in scenarios() {
            let e = evaluate(scenario).unwrap();
            assert!(e.evsi_policy_cost <= e.risk_policy_cost + TOL);
            if e.evsi_action == Action::Inspect {
                assert!(e.always_inspect_cost + TOL < e.risk_policy_cost);
            } else {
                assert!((e.evsi_policy_cost - e.risk_policy_cost).abs() < TOL);
            }
        }
    }

    #[test]
    fn invalid_probability_models_and_horizons_reject() {
        assert!(validate_distribution([0.2, 0.2, 0.2, 0.2, 0.3]).is_err());
        let mut bad = NORMAL_TRANSITION;
        bad[0][0] = -0.1;
        assert!(validate_transition(bad).is_err());
        assert!(first_passage_by_initial_state(NORMAL_TRANSITION, 0).is_err());
    }

    #[test]
    fn golden_campaign_is_deterministic_and_verified() {
        verify().unwrap();
        let one = scenarios().map(|s| evaluate(s).unwrap().evsi_policy_cost);
        let two = scenarios().map(|s| evaluate(s).unwrap().evsi_policy_cost);
        assert_eq!(one, two);
    }
}
