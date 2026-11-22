//! Multi-cause failure-domain model for exact contingent navigation sensing.
//!
//! Latent state = (binary service state, up to four binary failure-domain
//! regimes). Each action declares its failure-domain membership. Its
//! likelihood/acquisition vectors must be invariant to all undeclared causes.
//! Shared declared cause bits induce dependence after marginalization.

use std::collections::BTreeSet;
use std::fmt;

use crate::decision_value::{ActionCost, DecisionLoss, ResourceBudget};

const EPS: f64 = 1.0e-9;
pub const MAX_FAILURE_DOMAINS: usize = 4;

#[derive(Clone, Debug, PartialEq)]
pub struct FailureDomainModel {
    /// Canonically sorted, unique cause IDs. Bit 0 maps to the first ID.
    pub cause_ids: Vec<String>,
    /// Joint prior in service-state-major order, then cause bitmask ascending.
    /// First 2^K entries are StateZero; next 2^K are StateOne.
    pub prior: Vec<f64>,
}

impl FailureDomainModel {
    pub fn new(cause_ids: Vec<String>, prior: Vec<f64>) -> Result<Self, DependencyGraphError> {
        if cause_ids.len() > MAX_FAILURE_DOMAINS {
            return Err(DependencyGraphError::TooManyFailureDomains(cause_ids.len()));
        }
        if cause_ids.iter().any(|id| id.trim().is_empty()) {
            return Err(DependencyGraphError::EmptyFailureDomainId);
        }
        if cause_ids.windows(2).any(|pair| pair[0] >= pair[1]) {
            return Err(DependencyGraphError::FailureDomainIdsNotCanonical);
        }
        let state_count = 1_usize << cause_ids.len();
        if prior.len() != 2 * state_count {
            return Err(DependencyGraphError::PriorLengthMismatch {
                expected: 2 * state_count,
                actual: prior.len(),
            });
        }
        if prior.iter().any(|p| !p.is_finite() || !(0.0..=1.0).contains(p)) {
            return Err(DependencyGraphError::InvalidPriorComponent);
        }
        let total: f64 = prior.iter().sum();
        if (total - 1.0).abs() > EPS {
            return Err(DependencyGraphError::PriorMassMismatch(total));
        }
        Ok(Self { cause_ids, prior })
    }

    pub fn cause_pattern_count(&self) -> usize {
        1_usize << self.cause_ids.len()
    }

    pub fn joint_state_count(&self) -> usize {
        2 * self.cause_pattern_count()
    }

    pub fn p_service_zero(&self, belief: &[f64]) -> f64 {
        belief[..self.cause_pattern_count()].iter().sum()
    }

    pub fn p_service_one(&self, belief: &[f64]) -> f64 {
        belief[self.cause_pattern_count()..].iter().sum()
    }

    pub fn p_failure_domain(&self, belief: &[f64], id: &str) -> Result<f64, DependencyGraphError> {
        self.validate_belief(belief)?;
        let bit = self.cause_ids.iter().position(|candidate| candidate == id)
            .ok_or_else(|| DependencyGraphError::UnknownFailureDomain(id.to_owned()))?;
        let pattern_count = self.cause_pattern_count();
        let mut probability = 0.0;
        for (index, mass) in belief.iter().enumerate() {
            let mask = index % pattern_count;
            if (mask & (1 << bit)) != 0 {
                probability += mass;
            }
        }
        Ok(probability)
    }

    pub fn validate_belief(&self, belief: &[f64]) -> Result<(), DependencyGraphError> {
        if belief.len() != self.joint_state_count() {
            return Err(DependencyGraphError::BeliefLengthMismatch {
                expected: self.joint_state_count(),
                actual: belief.len(),
            });
        }
        if belief.iter().any(|p| !p.is_finite() || !(0.0..=1.0).contains(p)) {
            return Err(DependencyGraphError::InvalidPriorComponent);
        }
        let sum: f64 = belief.iter().sum();
        if (sum - 1.0).abs() > EPS {
            return Err(DependencyGraphError::PriorMassMismatch(sum));
        }
        Ok(())
    }

    fn cause_bit(&self, id: &str) -> Option<usize> {
        self.cause_ids.iter().position(|candidate| candidate == id)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DomainOutcome {
    pub id: String,
    /// P(outcome | service state, failure-domain bit pattern).
    pub likelihood: Vec<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DomainAction {
    pub id: String,
    pub cost: ActionCost,
    /// Canonically sorted list of declared failure domains affecting this source.
    /// Use an explicit shared "dependency-unknown" cause if the inventory is
    /// incomplete; do not silently provide an empty set to claim independence.
    pub failure_domains: Vec<String>,
    /// P(acquisition succeeds | service state, failure-domain pattern).
    pub acquisition_success: Vec<f64>,
    pub outcomes: Vec<DomainOutcome>,
}

impl DomainAction {
    pub fn validate(&self, model: &FailureDomainModel) -> Result<(), DependencyGraphError> {
        if self.id.trim().is_empty() {
            return Err(DependencyGraphError::EmptyActionId);
        }
        if self.failure_domains.is_empty() {
            return Err(DependencyGraphError::UndeclaredFailureDomains(self.id.clone()));
        }
        if self.failure_domains.windows(2).any(|pair| pair[0] >= pair[1]) {
            return Err(DependencyGraphError::ActionFailureDomainsNotCanonical(self.id.clone()));
        }
        for domain in &self.failure_domains {
            if model.cause_bit(domain).is_none() {
                return Err(DependencyGraphError::UnknownFailureDomain(domain.clone()));
            }
        }
        let state_count = model.joint_state_count();
        if self.acquisition_success.len() != state_count {
            return Err(DependencyGraphError::ActionVectorLengthMismatch {
                action: self.id.clone(),
                expected: state_count,
                actual: self.acquisition_success.len(),
            });
        }
        if self.acquisition_success.iter().any(|p| !p.is_finite() || !(0.0..=1.0).contains(p)) {
            return Err(DependencyGraphError::InvalidAcquisitionProbability(self.id.clone()));
        }
        if self.outcomes.is_empty() {
            return Err(DependencyGraphError::EmptyOutcomeSet(self.id.clone()));
        }

        let mut outcome_ids = BTreeSet::new();
        let mut masses = vec![0.0; state_count];
        for outcome in &self.outcomes {
            if outcome.id.trim().is_empty() {
                return Err(DependencyGraphError::EmptyOutcomeId(self.id.clone()));
            }
            if !outcome_ids.insert(outcome.id.clone()) {
                return Err(DependencyGraphError::DuplicateOutcomeId {
                    action: self.id.clone(),
                    outcome: outcome.id.clone(),
                });
            }
            if outcome.likelihood.len() != state_count {
                return Err(DependencyGraphError::ActionVectorLengthMismatch {
                    action: format!("{}:{}", self.id, outcome.id),
                    expected: state_count,
                    actual: outcome.likelihood.len(),
                });
            }
            for (index, probability) in outcome.likelihood.iter().enumerate() {
                if !probability.is_finite() || !(0.0..=1.0).contains(probability) {
                    return Err(DependencyGraphError::InvalidOutcomeLikelihood {
                        action: self.id.clone(),
                        outcome: outcome.id.clone(),
                        latent_state: index,
                    });
                }
                masses[index] += probability;
            }
        }
        for (index, mass) in masses.iter().enumerate() {
            if (*mass - 1.0).abs() > EPS {
                return Err(DependencyGraphError::OutcomeMassMismatch {
                    action: self.id.clone(),
                    latent_state: index,
                    sum: *mass,
                });
            }
        }

        // Fail closed: an action cannot depend on a cause absent from its
        // declared membership. Check every cause bit, both service states,
        // every outcome and acquisition success.
        for bit in 0..model.cause_ids.len() {
            let cause_id = &model.cause_ids[bit];
            if self.failure_domains.binary_search(cause_id).is_ok() {
                continue;
            }
            let bit_mask = 1_usize << bit;
            for service_state in 0..2 {
                for cause_mask in 0..model.cause_pattern_count() {
                    if cause_mask & bit_mask != 0 {
                        continue;
                    }
                    let left = service_state * model.cause_pattern_count() + cause_mask;
                    let right = left + bit_mask;
                    if (self.acquisition_success[left] - self.acquisition_success[right]).abs() > EPS {
                        return Err(DependencyGraphError::UndeclaredDependency {
                            action: self.id.clone(),
                            domain: cause_id.clone(),
                        });
                    }
                    for outcome in &self.outcomes {
                        if (outcome.likelihood[left] - outcome.likelihood[right]).abs() > EPS {
                            return Err(DependencyGraphError::UndeclaredDependency {
                                action: self.id.clone(),
                                domain: cause_id.clone(),
                            });
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DomainBranch {
    pub branch_id: String,
    pub probability: f64,
    pub posterior: Vec<f64>,
    pub acquired: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DomainBranchPlan {
    pub branch_id: String,
    pub probability: f64,
    pub posterior_service_one: f64,
    pub posterior_failure_domains: Vec<(String, f64)>,
    pub second_action: Option<String>,
    pub expected_terminal_risk: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DomainPlan {
    pub first_action: String,
    pub branches: Vec<DomainBranchPlan>,
    pub baseline_risk: f64,
    pub expected_terminal_risk: f64,
    pub expected_decision_value: f64,
    pub maximum_path_cost: ActionCost,
}

#[derive(Clone, Debug, PartialEq)]
pub enum DependencyGraphError {
    TooManyFailureDomains(usize),
    EmptyFailureDomainId,
    FailureDomainIdsNotCanonical,
    UnknownFailureDomain(String),
    PriorLengthMismatch { expected: usize, actual: usize },
    BeliefLengthMismatch { expected: usize, actual: usize },
    InvalidPriorComponent,
    PriorMassMismatch(f64),
    InvalidDecisionLoss,
    EmptyActionId,
    DuplicateActionId(String),
    UndeclaredFailureDomains(String),
    ActionFailureDomainsNotCanonical(String),
    ActionVectorLengthMismatch { action: String, expected: usize, actual: usize },
    InvalidAcquisitionProbability(String),
    EmptyOutcomeSet(String),
    EmptyOutcomeId(String),
    DuplicateOutcomeId { action: String, outcome: String },
    InvalidOutcomeLikelihood { action: String, outcome: String, latent_state: usize },
    OutcomeMassMismatch { action: String, latent_state: usize, sum: f64 },
    UndeclaredDependency { action: String, domain: String },
    BranchMassMismatch { action: String, sum: f64 },
}

impl fmt::Display for DependencyGraphError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooManyFailureDomains(count) => write!(f, "{count} failure domains exceed the exact-model cap of {MAX_FAILURE_DOMAINS}"),
            Self::EmptyFailureDomainId => write!(f, "failure-domain ID must not be empty"),
            Self::FailureDomainIdsNotCanonical => write!(f, "failure-domain IDs must be unique and lexicographically sorted"),
            Self::UnknownFailureDomain(id) => write!(f, "unknown failure-domain ID: {id}"),
            Self::PriorLengthMismatch { expected, actual } => write!(f, "prior length {actual}, expected {expected}"),
            Self::BeliefLengthMismatch { expected, actual } => write!(f, "belief length {actual}, expected {expected}"),
            Self::InvalidPriorComponent => write!(f, "belief/prior components must be finite and in [0,1]"),
            Self::PriorMassMismatch(sum) => write!(f, "joint probability mass sums to {sum}, not 1"),
            Self::InvalidDecisionLoss => write!(f, "decision loss is invalid"),
            Self::EmptyActionId => write!(f, "action ID must not be empty"),
            Self::DuplicateActionId(id) => write!(f, "duplicate action ID: {id}"),
            Self::UndeclaredFailureDomains(id) => write!(f, "action {id} must declare at least one failure domain (use an explicit unknown/shared domain if necessary)"),
            Self::ActionFailureDomainsNotCanonical(id) => write!(f, "failure-domain list for {id} must be unique and sorted"),
            Self::ActionVectorLengthMismatch { action, expected, actual } => write!(f, "action vector {action} has length {actual}, expected {expected}"),
            Self::InvalidAcquisitionProbability(id) => write!(f, "invalid acquisition probabilities for {id}"),
            Self::EmptyOutcomeSet(id) => write!(f, "action {id} has no outcomes"),
            Self::EmptyOutcomeId(id) => write!(f, "action {id} has an empty outcome ID"),
            Self::DuplicateOutcomeId { action, outcome } => write!(f, "duplicate outcome {outcome} in {action}"),
            Self::InvalidOutcomeLikelihood { action, outcome, latent_state } => write!(f, "invalid likelihood for {outcome} in {action}, latent state {latent_state}"),
            Self::OutcomeMassMismatch { action, latent_state, sum } => write!(f, "outcome mass for {action}, latent state {latent_state} sums to {sum}"),
            Self::UndeclaredDependency { action, domain } => write!(f, "action {action} varies with undeclared failure domain {domain}"),
            Self::BranchMassMismatch { action, sum } => write!(f, "branch mass for {action} is {sum}, not 1"),
        }
    }
}
impl std::error::Error for DependencyGraphError {}

pub fn enumerate_domain_branches(action:&DomainAction, model:&FailureDomainModel, prior:&[f64])->Result<Vec<DomainBranch>,DependencyGraphError>{
    action.validate(model)?;
    model.validate_belief(prior)?;
    let mut branches=Vec::new();
    let mut outcomes=action.outcomes.clone();
    outcomes.sort_by(|a,b|a.id.cmp(&b.id));
    for outcome in outcomes {
        let joint:Vec<f64>=(0..model.joint_state_count()).map(|i|prior[i]*action.acquisition_success[i]*outcome.likelihood[i]).collect();
        let p:f64=joint.iter().sum();
        if !p.is_finite()||p<0.0{return Err(DependencyGraphError::BranchMassMismatch{action:action.id.clone(),sum:p});}
        if p>0.0{branches.push(DomainBranch{branch_id:format!("measurement:{}",outcome.id),probability:p,posterior:joint.iter().map(|x|x/p).collect(),acquired:true});}
    }
    let failed:Vec<f64>=(0..model.joint_state_count()).map(|i|prior[i]*(1.0-action.acquisition_success[i])).collect();
    let pf:f64=failed.iter().sum();
    if !pf.is_finite()||pf<0.0{return Err(DependencyGraphError::BranchMassMismatch{action:action.id.clone(),sum:pf});}
    if pf>0.0{branches.push(DomainBranch{branch_id:"no_observation".to_owned(),probability:pf,posterior:failed.iter().map(|x|x/pf).collect(),acquired:false});}
    branches.sort_by(|a,b|a.branch_id.cmp(&b.branch_id));
    let total:f64=branches.iter().map(|b|b.probability).sum();
    if (total-1.0).abs()>EPS{return Err(DependencyGraphError::BranchMassMismatch{action:action.id.clone(),sum:total});}
    Ok(branches)
}

fn p_service_zero(model:&FailureDomainModel, belief:&[f64])->f64{belief[..model.cause_pattern_count()].iter().sum()}
fn p_service_one(model:&FailureDomainModel, belief:&[f64])->f64{belief[model.cause_pattern_count()..].iter().sum()}
fn risk(model:&FailureDomainModel,belief:&[f64],loss:DecisionLoss)->Result<f64,DependencyGraphError>{
    model.validate_belief(belief)?;
    loss.validate().map_err(|_|DependencyGraphError::InvalidDecisionLoss)?;
    Ok((p_service_zero(model,belief)*loss.false_safe_loss).min(p_service_one(model,belief)*loss.false_degraded_loss))
}
fn expected_risk(model:&FailureDomainModel,action:&DomainAction,belief:&[f64],loss:DecisionLoss)->Result<f64,DependencyGraphError>{
    Ok(enumerate_domain_branches(action,model,belief)?.iter().map(|b|risk(model,&b.posterior,loss).map(|r|b.probability*r)).collect::<Result<Vec<_>,_>>()?.iter().sum())
}
fn remaining_budget(b:ResourceBudget,c:ActionCost)->ResourceBudget{ResourceBudget{energy_units:b.energy_units-c.energy_units,latency_ticks:b.latency_ticks-c.latency_ticks,bandwidth_units:b.bandwidth_units-c.bandwidth_units}}
fn combine(a:ActionCost,b:ActionCost)->ActionCost{ActionCost{energy_units:a.energy_units.saturating_add(b.energy_units),latency_ticks:a.latency_ticks.saturating_add(b.latency_ticks),bandwidth_units:a.bandwidth_units.saturating_add(b.bandwidth_units)}}
fn prefer_action(a:&DomainAction,b:Option<&DomainAction>)->bool{b.is_some_and(|v|(a.cost.energy_units,a.cost.latency_ticks,a.cost.bandwidth_units,a.id.as_str())<(v.cost.energy_units,v.cost.latency_ticks,v.cost.bandwidth_units,v.id.as_str()))}

fn plan_for_first(first:&DomainAction,actions:&[DomainAction],model:&FailureDomainModel,prior:&[f64],budget:ResourceBudget,loss:DecisionLoss)->Result<DomainPlan,DependencyGraphError>{
    let rest=remaining_budget(budget,first.cost);
    let base=risk(model,prior,loss)?;
    let branches=enumerate_domain_branches(first,model,prior)?;
    let mut plans=Vec::new();let mut terminal=0.0;let mut max_cost=first.cost;
    for branch in branches{
        let stop=risk(model,&branch.posterior,loss)?;
        let mut best_risk=stop;let mut best:Option<&DomainAction>=None;
        for candidate in actions{
            if candidate.id==first.id||!candidate.cost.fits(rest){continue;}
            let value=expected_risk(model,candidate,&branch.posterior,loss)?;
            if value+1.0e-12<stop && (best.is_none()||value.total_cmp(&best_risk).is_lt()||(value.total_cmp(&best_risk).is_eq()&&prefer_action(candidate,best))){
                best=Some(candidate);best_risk=value;
            }
        }
        if let Some(second)=best{let path=combine(first.cost,second.cost);max_cost.energy_units=max_cost.energy_units.max(path.energy_units);max_cost.latency_ticks=max_cost.latency_ticks.max(path.latency_ticks);max_cost.bandwidth_units=max_cost.bandwidth_units.max(path.bandwidth_units);}
        terminal+=branch.probability*best_risk;
        let mut causes=Vec::new();
        for id in &model.cause_ids{let q=model.p_failure_domain(&branch.posterior,id)?;causes.push((id.clone(),q));}
        plans.push(DomainBranchPlan{branch_id:branch.branch_id,probability:branch.probability,posterior_service_one:p_service_one(model,&branch.posterior),posterior_failure_domains:causes,second_action:best.map(|a|a.id.clone()),expected_terminal_risk:best_risk});
    }
    plans.sort_by(|a,b|a.branch_id.cmp(&b.branch_id));
    Ok(DomainPlan{first_action:first.id.clone(),branches:plans,baseline_risk:base,expected_terminal_risk:terminal,expected_decision_value:base-terminal,maximum_path_cost:max_cost})
}

fn plan_better(a:&DomainPlan,b:&DomainPlan)->bool{match a.expected_terminal_risk.total_cmp(&b.expected_terminal_risk){std::cmp::Ordering::Less=>true,std::cmp::Ordering::Greater=>false,std::cmp::Ordering::Equal=>(a.first_action.as_str(),a.maximum_path_cost.energy_units,a.maximum_path_cost.latency_ticks,a.maximum_path_cost.bandwidth_units)<(b.first_action.as_str(),b.maximum_path_cost.energy_units,b.maximum_path_cost.latency_ticks,b.maximum_path_cost.bandwidth_units)}}

fn validate_universe(actions:&[DomainAction],model:&FailureDomainModel,prior:&[f64],loss:DecisionLoss)->Result<(),DependencyGraphError>{
    model.validate_belief(prior)?;loss.validate().map_err(|_|DependencyGraphError::InvalidDecisionLoss)?;
    let mut ids=BTreeSet::new();for a in actions{a.validate(model)?;if !ids.insert(a.id.clone()){return Err(DependencyGraphError::DuplicateActionId(a.id.clone()));}}
    Ok(())
}

pub fn exact_domain_horizon_two(actions:&[DomainAction],model:&FailureDomainModel,prior:&[f64],budget:ResourceBudget,loss:DecisionLoss)->Result<Option<DomainPlan>,DependencyGraphError>{
    validate_universe(actions,model,prior,loss)?;
    let mut ordered:Vec<&DomainAction>=actions.iter().collect();ordered.sort_by(|a,b|a.id.cmp(&b.id));
    let mut best:Option<DomainPlan>=None;
    for first in ordered{if !first.cost.fits(budget){continue;}let p=plan_for_first(first,actions,model,prior,budget,loss)?;if best.as_ref().is_none_or(|v|plan_better(&p,v)){best=Some(p);}}
    Ok(best.filter(|p|p.expected_decision_value>1.0e-12))
}

pub fn greedy_first_domain_horizon_two(actions:&[DomainAction],model:&FailureDomainModel,prior:&[f64],budget:ResourceBudget,loss:DecisionLoss)->Result<Option<DomainPlan>,DependencyGraphError>{
    validate_universe(actions,model,prior,loss)?;
    let mut ordered:Vec<&DomainAction>=actions.iter().collect();ordered.sort_by(|a,b|a.id.cmp(&b.id));
    let mut first:Option<(&DomainAction,f64)>=None;
    for candidate in ordered{if !candidate.cost.fits(budget){continue;}let value=expected_risk(model,candidate,prior,loss)?;
        let replace=first.is_none_or(|(current,best)|value.total_cmp(&best).is_lt()||(value.total_cmp(&best).is_eq()&&prefer_action(candidate,Some(current))));
        if replace{first=Some((candidate,value));}}
    let Some((candidate,_))=first else{return Ok(None)};
    let plan=plan_for_first(candidate,actions,model,prior,budget,loss)?;
    Ok((plan.expected_decision_value>1.0e-12).then_some(plan))
}

/// Actual and naive-independent probability of two successful sensor outcomes
/// given StateZero. This marginalizes the full declared failure-domain prior.
pub fn compare_source_outcomes_given_state_zero(model:&FailureDomainModel,prior:&[f64],a:&DomainAction,a_outcome:&str,b:&DomainAction,b_outcome:&str)->Result<(f64,f64),DependencyGraphError>{
    model.validate_belief(prior)?;a.validate(model)?;b.validate(model)?;
    let ao=a.outcomes.iter().find(|x|x.id==a_outcome).ok_or_else(||DependencyGraphError::EmptyOutcomeId(a.id.clone()))?;
    let bo=b.outcomes.iter().find(|x|x.id==b_outcome).ok_or_else(||DependencyGraphError::EmptyOutcomeId(b.id.clone()))?;
    let p0=p_service_zero(model,prior);if p0<=0.0{return Ok((0.0,0.0));}
    let n=model.cause_pattern_count();let mut pa=0.0;let mut pb=0.0;let mut joint=0.0;
    for mask in 0..n{
        let w=prior[mask]/p0;
        let x=a.acquisition_success[mask]*ao.likelihood[mask];
        let y=b.acquisition_success[mask]*bo.likelihood[mask];
        pa+=w*x;pb+=w*y;joint+=w*x*y;
    }
    Ok((joint,pa*pb))
}

#[cfg(test)]
mod tests{
    use super::*;
    fn model()->FailureDomainModel{
        // Cause order is lexicographic: clock=bit0, power=bit1.
        // Independent 10% clock and power common-cause probabilities,
        // independent of the 50/50 service state.
        let p=[0.81,0.09,0.09,0.01];
        let mut prior=Vec::new();for _ in 0..2{prior.extend(p.map(|v|v*0.5));}
        FailureDomainModel::new(vec!["clock".into(),"power".into()],prior).unwrap()
    }
    fn make_action(id:&str,domains:Vec<&str>,n:u32,kind:&str)->DomainAction{
        let m=model();let mut safe=Vec::new();
        for state in 0..2{for mask in 0..m.cause_pattern_count(){
            let clock=(mask&1)!=0;let power=(mask&2)!=0;
            let p=match(kind,state,power,clock){
                ("power",0,false,_)=>0.05,("power",0,true,_)=>0.90,("power",1,_,_)=>0.95,
                ("clock",0,_,false)=>0.15,("clock",0,_,true)=>0.35,("clock",1,_,false)=>0.85,("clock",1,_,true)=>0.75,
                ("mixed",0,false,_)=>0.08,("mixed",0,true,_)=>0.88,("mixed",1,_,_)=>0.92,
                _=>0.5
            };
            safe.push(p);
        }}
        let count=m.joint_state_count();
        DomainAction{
            id:id.into(),cost:ActionCost{energy_units:n,latency_ticks:n,bandwidth_units:n},
            failure_domains:domains.into_iter().map(str::to_owned).collect(),
            acquisition_success:vec![1.0;count],
            outcomes:vec![DomainOutcome{id:"safe-reading".into(),likelihood:safe.clone()},DomainOutcome{id:"unsafe-reading".into(),likelihood:safe.iter().map(|p|1.0-p).collect()}]
        }
    }
    fn actions()->Vec<DomainAction>{vec![
        make_action("uwb",vec!["power"],1,"power"),
        make_action("camera",vec!["power"],1,"mixed"),
        make_action("imu",vec!["clock"],1,"clock")
    ]}
    fn budget()->ResourceBudget{ResourceBudget{energy_units:2,latency_ticks:2,bandwidth_units:2}}

    #[test]fn shared_power_induces_dependence_but_separate_clock_does_not(){
        let m=model();let prior=&m.prior;let a=actions();
        let(shared,independent)=compare_source_outcomes_given_state_zero(&m,prior,&a[0],"safe-reading",&a[1],"safe-reading").unwrap();
        assert!(shared>independent*3.0);
        let(disjoint,disjoint_independent)=compare_source_outcomes_given_state_zero(&m,prior,&a[0],"safe-reading",&a[2],"safe-reading").unwrap();
        assert!((disjoint-disjoint_independent).abs()<1.0e-9);
    }
    #[test]fn undeclared_cause_dependence_fails_closed(){
        let m=model();let mut bad=actions()[0].clone();
        bad.outcomes[0].likelihood[1]=0.2; // changes only clock=1 though action declares power only
        bad.outcomes[1].likelihood[1]=0.8;
        assert!(matches!(bad.validate(&m),Err(DependencyGraphError::UndeclaredDependency{..})));
    }
    #[test]fn model_requires_canonical_domains_and_caps_state_space(){
        assert!(matches!(FailureDomainModel::new(vec!["power".into(),"clock".into()],vec![0.25;8]),Err(DependencyGraphError::FailureDomainIdsNotCanonical)));
        let ids=vec!["a".into(),"b".into(),"c".into(),"d".into(),"e".into()];
        assert!(matches!(FailureDomainModel::new(ids,vec![0.0;64]),Err(DependencyGraphError::TooManyFailureDomains(5))));
    }
    #[test]fn branch_posterior_updates_only_relevant_cause(){
        let m=model();let a=actions();let branches=enumerate_domain_branches(&a[0],&m,&m.prior).unwrap();
        let safe=branches.iter().find(|b|b.branch_id=="measurement:safe-reading").unwrap();
        let power_prior=m.p_failure_domain(&m.prior,"power").unwrap();
        let power_after=m.p_failure_domain(&safe.posterior,"power").unwrap();
        let clock_prior=m.p_failure_domain(&m.prior,"clock").unwrap();
        let clock_after=m.p_failure_domain(&safe.posterior,"clock").unwrap();
        assert!(power_after>power_prior);
        assert!((clock_after-clock_prior).abs()<1.0e-8);
    }
    #[test]fn exact_contingent_plan_respects_budgets_and_greedy_lower_bound(){
        let m=model();let a=actions();let loss=DecisionLoss::zero_one();
        let exact=exact_domain_horizon_two(&a,&m,&m.prior,budget(),loss).unwrap().unwrap();
        let greedy=greedy_first_domain_horizon_two(&a,&m,&m.prior,budget(),loss).unwrap().unwrap();
        assert!(exact.expected_terminal_risk<=greedy.expected_terminal_risk+EPS);
        assert!(exact.maximum_path_cost.energy_units<=budget().energy_units);
        assert!(exact.maximum_path_cost.latency_ticks<=budget().latency_ticks);
        assert!(exact.maximum_path_cost.bandwidth_units<=budget().bandwidth_units);
    }
    #[test]fn action_input_order_is_deterministic(){
        let m=model();let mut a=actions();let mut b=actions();b.reverse();
        assert_eq!(exact_domain_horizon_two(&a,&m,&m.prior,budget(),DecisionLoss::zero_one()).unwrap(),exact_domain_horizon_two(&b,&m,&m.prior,budget(),DecisionLoss::zero_one()).unwrap());
        a.reverse();
    }
    #[test]fn state_dependent_acquisition_failure_is_informative_only_when_declared(){
        let m=model();let mut a=actions()[0].clone();
        for i in 0..m.joint_state_count(){let mask=i%m.cause_pattern_count();a.acquisition_success[i]=if mask&2!=0{0.05}else{0.9};}
        let branches=enumerate_domain_branches(&a,&m,&m.prior).unwrap();
        let failed=branches.iter().find(|b|b.branch_id=="no_observation").unwrap();
        assert!(m.p_failure_domain(&failed.posterior,"power").unwrap()>0.5);
    }
}
