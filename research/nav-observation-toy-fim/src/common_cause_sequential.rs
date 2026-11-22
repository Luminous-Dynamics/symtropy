//! Exact contingent sensing with a shared latent common-cause regime.
//! Observation actions are independent only conditional on both the binary
//! service state S and binary common-cause state C. Marginalizing C induces
//! cross-sensor dependence.

use std::collections::BTreeSet;
use std::fmt;

use crate::decision_value::{ActionCost, DecisionLoss, ResourceBudget};

const EPS: f64 = 1.0e-9;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct JointBelief {
    /// (S0,C0), (S0,C1), (S1,C0), (S1,C1).
    pub probability: [f64; 4],
}

impl JointBelief {
    pub fn new(probability: [f64; 4]) -> Result<Self, CommonCauseError> {
        if probability.iter().any(|p| !p.is_finite() || !(0.0..=1.0).contains(p)) {
            return Err(CommonCauseError::InvalidJointPrior);
        }
        let sum: f64 = probability.iter().sum();
        if (sum - 1.0).abs() > EPS {
            return Err(CommonCauseError::PriorMassMismatch(sum));
        }
        Ok(Self { probability })
    }
    pub fn p_state_zero(self) -> f64 { self.probability[0] + self.probability[1] }
    pub fn p_state_one(self) -> f64 { self.probability[2] + self.probability[3] }
    pub fn p_common_cause(self) -> f64 { self.probability[1] + self.probability[3] }
    pub fn bayes_risk(self, loss: DecisionLoss) -> Result<f64, CommonCauseError> {
        loss.validate().map_err(|_| CommonCauseError::InvalidDecisionLoss)?;
        Ok((self.p_state_zero() * loss.false_safe_loss)
            .min(self.p_state_one() * loss.false_degraded_loss))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct JointOutcome {
    pub id: String,
    /// P(outcome | S,C) in the same four-state order as JointBelief.
    pub likelihood: [f64; 4],
}

#[derive(Clone, Debug, PartialEq)]
pub struct CommonCauseAction {
    pub id: String,
    pub cost: ActionCost,
    /// P(acquisition succeeds | S,C), in four-state order.
    pub acquisition_success: [f64; 4],
    /// Mutually exclusive outcomes; likelihoods sum to one for each (S,C).
    pub outcomes: Vec<JointOutcome>,
}

impl CommonCauseAction {
    pub fn validate(&self) -> Result<(), CommonCauseError> {
        if self.id.trim().is_empty() { return Err(CommonCauseError::EmptyActionId); }
        if self.outcomes.is_empty() { return Err(CommonCauseError::EmptyOutcomeSet(self.id.clone())); }
        if self.acquisition_success.iter().any(|p| !p.is_finite() || !(0.0..=1.0).contains(p)) {
            return Err(CommonCauseError::InvalidAcquisitionProbability(self.id.clone()));
        }
        let mut ids = BTreeSet::new();
        let mut mass = [0.0; 4];
        for outcome in &self.outcomes {
            if outcome.id.trim().is_empty() { return Err(CommonCauseError::EmptyOutcomeId(self.id.clone())); }
            if !ids.insert(outcome.id.clone()) {
                return Err(CommonCauseError::DuplicateOutcomeId { action: self.id.clone(), outcome: outcome.id.clone() });
            }
            for i in 0..4 {
                let p = outcome.likelihood[i];
                if !p.is_finite() || !(0.0..=1.0).contains(&p) {
                    return Err(CommonCauseError::InvalidOutcomeLikelihood { action: self.id.clone(), outcome: outcome.id.clone(), latent: i });
                }
                mass[i] += p;
            }
        }
        for (i, sum) in mass.iter().enumerate() {
            if (*sum - 1.0).abs() > EPS {
                return Err(CommonCauseError::OutcomeMassMismatch { action: self.id.clone(), latent: i, sum: *sum });
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct JointBranch {
    pub branch_id: String,
    pub probability: f64,
    pub posterior: JointBelief,
    pub acquired: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BranchPlan {
    pub branch_id: String,
    pub probability: f64,
    pub posterior_state_one: f64,
    pub posterior_common_cause: f64,
    pub second_action: Option<String>,
    pub terminal_bayes_risk: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CommonCausePlan {
    pub first_action: String,
    pub branches: Vec<BranchPlan>,
    pub baseline_risk: f64,
    pub expected_terminal_risk: f64,
    pub expected_decision_value: f64,
    pub expected_energy: f64,
    pub expected_latency: f64,
    pub expected_bandwidth: f64,
    pub maximum_path_cost: ActionCost,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DependenceComparison {
    pub actual_joint_probability: f64,
    pub independent_marginal_product: f64,
    pub actual_probability_given_state_zero: f64,
    pub independent_probability_given_state_zero: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum CommonCauseError {
    InvalidJointPrior,
    PriorMassMismatch(f64),
    InvalidDecisionLoss,
    EmptyActionId,
    DuplicateActionId(String),
    EmptyOutcomeSet(String),
    EmptyOutcomeId(String),
    DuplicateOutcomeId { action: String, outcome: String },
    InvalidAcquisitionProbability(String),
    InvalidOutcomeLikelihood { action: String, outcome: String, latent: usize },
    OutcomeMassMismatch { action: String, latent: usize, sum: f64 },
    BranchMassMismatch { action: String, sum: f64 },
    OutcomeNotFound { action: String, outcome: String },
}
impl fmt::Display for CommonCauseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidJointPrior => write!(f, "joint prior components must be finite in [0,1]"),
            Self::PriorMassMismatch(sum) => write!(f, "joint prior sums to {sum}, not 1"),
            Self::InvalidDecisionLoss => write!(f, "invalid asymmetric decision loss"),
            Self::EmptyActionId => write!(f, "action ID must not be empty"),
            Self::DuplicateActionId(id) => write!(f, "duplicate action ID: {id}"),
            Self::EmptyOutcomeSet(id) => write!(f, "empty outcome set for {id}"),
            Self::EmptyOutcomeId(id) => write!(f, "empty outcome ID for {id}"),
            Self::DuplicateOutcomeId { action, outcome } => write!(f, "duplicate outcome {outcome} in {action}"),
            Self::InvalidAcquisitionProbability(id) => write!(f, "invalid acquisition probability in {id}"),
            Self::InvalidOutcomeLikelihood { action, outcome, latent } => write!(f, "invalid likelihood {outcome} in {action}, latent {latent}"),
            Self::OutcomeMassMismatch { action, latent, sum } => write!(f, "outcome mass for {action}, latent {latent} is {sum}"),
            Self::BranchMassMismatch { action, sum } => write!(f, "branch mass for {action} is {sum}"),
            Self::OutcomeNotFound { action, outcome } => write!(f, "outcome {outcome} not found in {action}"),
        }
    }
}
impl std::error::Error for CommonCauseError {}

pub fn enumerate_joint_branches(
    action: &CommonCauseAction,
    prior: JointBelief,
) -> Result<Vec<JointBranch>, CommonCauseError> {
    action.validate()?;
    JointBelief::new(prior.probability)?;
    let mut outcomes = action.outcomes.clone();
    outcomes.sort_by(|a, b| a.id.cmp(&b.id));
    let mut branches = Vec::new();

    for outcome in outcomes {
        let joint = std::array::from_fn(|i| {
            prior.probability[i] * action.acquisition_success[i] * outcome.likelihood[i]
        });
        let p: f64 = joint.iter().sum();
        if !p.is_finite() || p < 0.0 {
            return Err(CommonCauseError::BranchMassMismatch { action: action.id.clone(), sum: p });
        }
        if p > 0.0 {
            branches.push(JointBranch {
                branch_id: format!("measurement:{}", outcome.id),
                probability: p,
                posterior: JointBelief::new(joint.map(|v| v / p))?,
                acquired: true,
            });
        }
    }

    let failed = std::array::from_fn(|i| prior.probability[i] * (1.0 - action.acquisition_success[i]));
    let p_failed: f64 = failed.iter().sum();
    if !p_failed.is_finite() || p_failed < 0.0 {
        return Err(CommonCauseError::BranchMassMismatch { action: action.id.clone(), sum: p_failed });
    }
    if p_failed > 0.0 {
        branches.push(JointBranch {
            branch_id: "no_observation".to_owned(),
            probability: p_failed,
            posterior: JointBelief::new(failed.map(|v| v / p_failed))?,
            acquired: false,
        });
    }
    branches.sort_by(|a, b| a.branch_id.cmp(&b.branch_id));
    let sum: f64 = branches.iter().map(|b| b.probability).sum();
    if (sum - 1.0).abs() > EPS {
        return Err(CommonCauseError::BranchMassMismatch { action: action.id.clone(), sum });
    }
    Ok(branches)
}

fn validate_actions(actions: &[CommonCauseAction], prior: JointBelief, loss: DecisionLoss) -> Result<(), CommonCauseError> {
    JointBelief::new(prior.probability)?;
    loss.validate().map_err(|_| CommonCauseError::InvalidDecisionLoss)?;
    let mut ids = BTreeSet::new();
    for action in actions {
        action.validate()?;
        if !ids.insert(action.id.clone()) {
            return Err(CommonCauseError::DuplicateActionId(action.id.clone()));
        }
    }
    Ok(())
}

fn remaining_budget(b: ResourceBudget, c: ActionCost) -> ResourceBudget {
    ResourceBudget { energy_units: b.energy_units-c.energy_units, latency_ticks: b.latency_ticks-c.latency_ticks, bandwidth_units: b.bandwidth_units-c.bandwidth_units }
}
fn combine(a: ActionCost, b: ActionCost) -> ActionCost {
    ActionCost { energy_units:a.energy_units.saturating_add(b.energy_units), latency_ticks:a.latency_ticks.saturating_add(b.latency_ticks), bandwidth_units:a.bandwidth_units.saturating_add(b.bandwidth_units) }
}

fn expected_risk(action: &CommonCauseAction, belief: JointBelief, loss: DecisionLoss) -> Result<f64, CommonCauseError> {
    Ok(enumerate_joint_branches(action, belief)?.iter()
        .map(|branch| branch.posterior.bayes_risk(loss).map(|risk| branch.probability*risk))
        .collect::<Result<Vec<_>,_>>()?.iter().sum())
}

fn tie_prefer(candidate: &CommonCauseAction, current: Option<&CommonCauseAction>) -> bool {
    current.is_some_and(|c| (candidate.cost.energy_units,candidate.cost.latency_ticks,candidate.cost.bandwidth_units,candidate.id.as_str())
        < (c.cost.energy_units,c.cost.latency_ticks,c.cost.bandwidth_units,c.id.as_str()))
}

fn plan_for_first(first: &CommonCauseAction, actions: &[CommonCauseAction], prior: JointBelief, budget: ResourceBudget, loss: DecisionLoss) -> Result<CommonCausePlan,CommonCauseError> {
    let remain=remaining_budget(budget,first.cost);
    let baseline=prior.bayes_risk(loss)?;
    let branches=enumerate_joint_branches(first,prior)?;
    let mut plans=Vec::new();
    let mut terminal=0.0;
    let mut expected_energy=first.cost.energy_units as f64;
    let mut expected_latency=first.cost.latency_ticks as f64;
    let mut expected_bandwidth=first.cost.bandwidth_units as f64;
    let mut max_path=first.cost;
    for branch in branches {
        let stop_risk=branch.posterior.bayes_risk(loss)?;
        let mut best_risk=stop_risk;
        let mut best:Option<&CommonCauseAction>=None;
        for action in actions {
            if action.id==first.id || !action.cost.fits(remain) {continue;}
            let risk=expected_risk(action,branch.posterior,loss)?;
            if risk+1.0e-12<stop_risk && (best.is_none() || risk.total_cmp(&best_risk).is_lt() || (risk.total_cmp(&best_risk).is_eq() && tie_prefer(action,best))) {
                best=Some(action); best_risk=risk;
            }
        }
        if let Some(second)=best {
            let path=combine(first.cost,second.cost);
            max_path.energy_units=max_path.energy_units.max(path.energy_units);
            max_path.latency_ticks=max_path.latency_ticks.max(path.latency_ticks);
            max_path.bandwidth_units=max_path.bandwidth_units.max(path.bandwidth_units);
            expected_energy+=branch.probability*second.cost.energy_units as f64;
            expected_latency+=branch.probability*second.cost.latency_ticks as f64;
            expected_bandwidth+=branch.probability*second.cost.bandwidth_units as f64;
        }
        terminal+=branch.probability*best_risk;
        plans.push(BranchPlan {
            branch_id:branch.branch_id, probability:branch.probability,
            posterior_state_one:branch.posterior.p_state_one(),
            posterior_common_cause:branch.posterior.p_common_cause(),
            second_action:best.map(|a|a.id.clone()), terminal_bayes_risk:best_risk
        });
    }
    plans.sort_by(|a,b|a.branch_id.cmp(&b.branch_id));
    Ok(CommonCausePlan {
        first_action:first.id.clone(), branches:plans, baseline_risk:baseline,
        expected_terminal_risk:terminal, expected_decision_value:baseline-terminal,
        expected_energy, expected_latency, expected_bandwidth, maximum_path_cost:max_path
    })
}

fn plan_better(a:&CommonCausePlan,b:&CommonCausePlan)->bool {
    match a.expected_terminal_risk.total_cmp(&b.expected_terminal_risk) {
        std::cmp::Ordering::Less=>true,
        std::cmp::Ordering::Greater=>false,
        std::cmp::Ordering::Equal=>(a.expected_energy,a.expected_latency,a.expected_bandwidth,a.first_action.as_str())
          <(b.expected_energy,b.expected_latency,b.expected_bandwidth,b.first_action.as_str())
    }
}

pub fn exact_common_cause_horizon_two(actions:&[CommonCauseAction],prior:JointBelief,budget:ResourceBudget,loss:DecisionLoss)->Result<Option<CommonCausePlan>,CommonCauseError>{
    validate_actions(actions,prior,loss)?;
    let mut ordered:Vec<&CommonCauseAction>=actions.iter().collect();
    ordered.sort_by(|a,b|a.id.cmp(&b.id));
    let mut best:Option<CommonCausePlan>=None;
    for first in ordered {
        if !first.cost.fits(budget){continue;}
        let plan=plan_for_first(first,actions,prior,budget,loss)?;
        if best.as_ref().is_none_or(|b|plan_better(&plan,b)){best=Some(plan);}
    }
    Ok(best.filter(|p|p.expected_decision_value>1.0e-12))
}

pub fn greedy_first_common_cause_horizon_two(actions:&[CommonCauseAction],prior:JointBelief,budget:ResourceBudget,loss:DecisionLoss)->Result<Option<CommonCausePlan>,CommonCauseError>{
    validate_actions(actions,prior,loss)?;
    let mut ordered:Vec<&CommonCauseAction>=actions.iter().collect();
    ordered.sort_by(|a,b|a.id.cmp(&b.id));
    let mut choice:Option<(&CommonCauseAction,f64)>=None;
    for candidate in ordered {
        if !candidate.cost.fits(budget){continue;}
        let risk=expected_risk(candidate,prior,loss)?;
        let replace=choice.is_none_or(|(current,cr)| risk.total_cmp(&cr).is_lt() || (risk.total_cmp(&cr).is_eq() && tie_prefer(candidate,Some(current))));
        if replace {choice=Some((candidate,risk));}
    }
    let Some((first,_))=choice else{return Ok(None)};
    let plan=plan_for_first(first,actions,prior,budget,loss)?;
    Ok((plan.expected_decision_value>1.0e-12).then_some(plan))
}

/// Compare actual P(A=a,B=b) with the naive product of marginal probabilities.
/// Under S0, common-cause marginalization is performed conditional on S0.
pub fn compare_two_sensor_outcomes(prior:JointBelief,a:&CommonCauseAction,a_id:&str,b:&CommonCauseAction,b_id:&str)->Result<DependenceComparison,CommonCauseError>{
    a.validate()?;b.validate()?;
    let ao=a.outcomes.iter().find(|o|o.id==a_id).ok_or_else(||CommonCauseError::OutcomeNotFound{action:a.id.clone(),outcome:a_id.to_owned()})?;
    let bo=b.outcomes.iter().find(|o|o.id==b_id).ok_or_else(||CommonCauseError::OutcomeNotFound{action:b.id.clone(),outcome:b_id.to_owned()})?;
    let mut pa=0.0;let mut pb=0.0;let mut pab=0.0;
    for i in 0..4 {
        pa+=prior.probability[i]*a.acquisition_success[i]*ao.likelihood[i];
        pb+=prior.probability[i]*b.acquisition_success[i]*bo.likelihood[i];
        pab+=prior.probability[i]*a.acquisition_success[i]*b.acquisition_success[i]*ao.likelihood[i]*bo.likelihood[i];
    }
    let p0=prior.p_state_zero();
    let mut pa0=0.0;let mut pb0=0.0;let mut pab0=0.0;
    if p0>0.0 {
        for i in 0..2 {
            let weight=prior.probability[i]/p0;
            let x=a.acquisition_success[i]*ao.likelihood[i];
            let y=b.acquisition_success[i]*bo.likelihood[i];
            pa0+=weight*x;pb0+=weight*y;pab0+=weight*x*y;
        }
    }
    Ok(DependenceComparison {
        actual_joint_probability:pab, independent_marginal_product:pa*pb,
        actual_probability_given_state_zero:pab0,
        independent_probability_given_state_zero:pa0*pb0
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn cost(n:u32)->ActionCost{ActionCost{energy_units:n,latency_ticks:n,bandwidth_units:n}}
    fn action(id:&str,n:u32,safe:[f64;4])->CommonCauseAction{
        CommonCauseAction{id:id.into(),cost:cost(n),acquisition_success:[1.0;4],outcomes:vec![
            JointOutcome{id:"safe-reading".into(),likelihood:safe},
            JointOutcome{id:"unsafe-reading".into(),likelihood:safe.map(|p|1.0-p)}
        ]}
    }
    fn prior()->JointBelief{JointBelief::new([0.45,0.05,0.45,0.05]).unwrap()}
    fn actions()->Vec<CommonCauseAction>{vec![
        action("uwb",1,[0.05,0.90,0.95,0.95]),
        action("camera",1,[0.08,0.88,0.90,0.92]),
        action("imu-motion",1,[0.15,0.35,0.85,0.75])
    ]}
    fn budget()->ResourceBudget{ResourceBudget{energy_units:2,latency_ticks:2,bandwidth_units:2}}

    #[test] fn prior_and_outcome_probabilities_fail_closed(){
        assert!(matches!(JointBelief::new([0.4,0.2,0.3,0.2]),Err(CommonCauseError::PriorMassMismatch(_))));
        let mut bad=actions();bad[0].outcomes[0].likelihood[1]=f64::NAN;
        assert!(matches!(bad[0].validate(),Err(CommonCauseError::InvalidOutcomeLikelihood{..})));
    }
    #[test] fn branch_mass_is_one_and_safe_measurement_updates_common_cause(){
        let a=actions();let branches=enumerate_joint_branches(&a[0],prior()).unwrap();
        assert!((branches.iter().map(|b|b.probability).sum::<f64>()-1.0).abs()<EPS);
        let safe=branches.iter().find(|b|b.branch_id=="measurement:safe-reading").unwrap();
        assert!(safe.posterior.p_common_cause()>prior().p_common_cause());
    }
    #[test] fn common_cause_induces_sensor_dependence(){
        let a=actions();let d=compare_two_sensor_outcomes(prior(),&a[0],"safe-reading",&a[1],"safe-reading").unwrap();
        assert!(d.actual_probability_given_state_zero>d.independent_probability_given_state_zero*3.0);
        assert!((d.actual_joint_probability-d.independent_marginal_product).abs()>1e-3);
    }
    #[test] fn joint_horizon_two_respects_budget_and_updates_both_latents(){
        let a=actions();let plan=exact_common_cause_horizon_two(&a,prior(),budget(),DecisionLoss::zero_one()).unwrap().unwrap();
        assert!(!plan.branches.is_empty());
        for b in &plan.branches {assert!((0.0..=1.0).contains(&b.posterior_state_one));assert!((0.0..=1.0).contains(&b.posterior_common_cause));}
        assert!(plan.maximum_path_cost.energy_units<=budget().energy_units);
        assert!(plan.maximum_path_cost.latency_ticks<=budget().latency_ticks);
        assert!(plan.maximum_path_cost.bandwidth_units<=budget().bandwidth_units);
    }
    #[test] fn exact_joint_policy_is_no_worse_than_greedy(){
        let a=actions();let exact=exact_common_cause_horizon_two(&a,prior(),budget(),DecisionLoss::zero_one()).unwrap().unwrap();
        let greedy=greedy_first_common_cause_horizon_two(&a,prior(),budget(),DecisionLoss::zero_one()).unwrap().unwrap();
        assert!(exact.expected_terminal_risk<=greedy.expected_terminal_risk+EPS);
    }
    #[test] fn action_permutation_does_not_change_plan(){
        let mut a=actions();let mut b=actions();b.reverse();
        assert_eq!(exact_common_cause_horizon_two(&a,prior(),budget(),DecisionLoss::zero_one()).unwrap(),exact_common_cause_horizon_two(&b,prior(),budget(),DecisionLoss::zero_one()).unwrap());
        a.reverse();
    }
    #[test] fn duplicate_action_ids_fail_closed(){
        let mut a=actions();a.push(a[0].clone());
        assert_eq!(exact_common_cause_horizon_two(&a,prior(),budget(),DecisionLoss::zero_one()),Err(CommonCauseError::DuplicateActionId("uwb".into())));
    }
    #[test] fn state_dependent_dropout_updates_shared_regime(){
        let mut a=action("dropout",1,[0.5;4]);a.acquisition_success=[0.9,0.1,0.9,0.1];
        let branches=enumerate_joint_branches(&a,prior()).unwrap();
        let missing=branches.iter().find(|b|b.branch_id=="no_observation").unwrap();
        assert!(!missing.acquired);
        assert!(missing.posterior.p_common_cause()>0.5);
    }
    #[test] fn asymmetric_decision_loss_is_supported(){
        let a=actions();let loss=DecisionLoss::new(10.0,1.0).unwrap();
        assert!(exact_common_cause_horizon_two(&a,prior(),budget(),loss).unwrap().is_some());
    }
}
