//! Correlation-aware linearized range information.
//!
//! This is an isolated research benchmark companion to the independent-noise
//! FIM toy. It models one explicit scalar shared error per declared
//! common-mode group:
//!
//! R = diag(sigma_private^2) + sum_g tau_g^2 * 1_g * 1_g^T
//! J = H^T R^-1 H
//!
//! A group is an explicit modelling input. Different IDs or modalities do
//! not prove independence. The shared-error model is deliberately narrow:
//! it does not represent arbitrary clock, calibration, anchor-survey or map
//! correlations, and it is not an integrity bound.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use nav_observation_toy_fim::{DesignCriterion, Fisher2};

const MAX_CORRELATED_ACTIONS: usize = 16;

#[derive(Clone, Debug, PartialEq)]
pub struct CorrelatedRangeAction {
    pub id: String,
    pub line_of_sight_rad: f64,
    pub independent_sigma_m: f64,
    pub cost_units: u32,
    pub common_mode_group: Option<String>,
    pub common_mode_sigma_m: Option<f64>,
}

impl CorrelatedRangeAction {
    pub fn independent(id: impl Into<String>, line_of_sight_rad: f64, sigma_m: f64, cost_units: u32) -> Self {
        Self { id:id.into(), line_of_sight_rad, independent_sigma_m:sigma_m, cost_units, common_mode_group:None, common_mode_sigma_m:None }
    }
    pub fn with_common_mode(id: impl Into<String>, line_of_sight_rad: f64, independent_sigma_m: f64, common_mode_group: impl Into<String>, common_mode_sigma_m: f64, cost_units: u32) -> Self {
        Self { id:id.into(), line_of_sight_rad, independent_sigma_m, cost_units, common_mode_group:Some(common_mode_group.into()), common_mode_sigma_m:Some(common_mode_sigma_m) }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum CorrelatedError {
    TooManyActions(usize), EmptyId, DuplicateId(String), InvalidBearing(String),
    InvalidIndependentSigma(String), InvalidCommonMode(String),
    InconsistentCommonModeSigma(String), MissingCommonModeSigma(String),
    UnexpectedCommonModeSigma(String), ZeroCost(String),
    NonPositiveDefiniteCovariance, NonFiniteInformation, InvalidRankTolerance, CostOverflow,
}
impl fmt::Display for CorrelatedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooManyActions(n) => write!(f,"{n} actions exceed the correlated exhaustive limit of {MAX_CORRELATED_ACTIONS}"),
            Self::EmptyId => write!(f,"action ID must not be empty"),
            Self::DuplicateId(id) => write!(f,"duplicate action ID: {id}"),
            Self::InvalidBearing(id) => write!(f,"invalid bearing for action: {id}"),
            Self::InvalidIndependentSigma(id) => write!(f,"invalid independent sigma for action: {id}"),
            Self::InvalidCommonMode(id) => write!(f,"invalid common-mode group/sigma for action: {id}"),
            Self::InconsistentCommonModeSigma(group) => write!(f,"common-mode sigma differs within group: {group}"),
            Self::MissingCommonModeSigma(id) => write!(f,"common-mode group lacks common-mode sigma: {id}"),
            Self::UnexpectedCommonModeSigma(id) => write!(f,"common-mode sigma supplied without a group: {id}"),
            Self::ZeroCost(id) => write!(f,"action cost must be positive: {id}"),
            Self::NonPositiveDefiniteCovariance => write!(f,"observation covariance is not positive definite"),
            Self::NonFiniteInformation => write!(f,"correlated Fisher information is non-finite"),
            Self::InvalidRankTolerance => write!(f,"rank tolerance must be finite and nonnegative"),
            Self::CostOverflow => write!(f,"total action cost overflowed u32"),
        }
    }
}
impl std::error::Error for CorrelatedError {}

#[derive(Clone, Debug, PartialEq)]
pub struct CorrelatedSelection {
    pub action_ids: Vec<String>, pub total_cost_units: u32, pub fisher: Fisher2,
    pub rank: u8, pub minimum_eigenvalue: f64, pub log_determinant: Option<f64>,
    pub a_optimal_trace_inverse: Option<f64>, pub condition_number: Option<f64>,
}

fn canonical(actions: &[CorrelatedRangeAction]) -> Result<Vec<CorrelatedRangeAction>, CorrelatedError> {
    if actions.len() > MAX_CORRELATED_ACTIONS { return Err(CorrelatedError::TooManyActions(actions.len())); }
    let mut ids = BTreeSet::new();
    let mut group_sigmas: BTreeMap<String,f64> = BTreeMap::new();
    let mut ordered=actions.to_vec();
    for a in &ordered {
        if a.id.trim().is_empty() { return Err(CorrelatedError::EmptyId); }
        if !ids.insert(a.id.clone()) { return Err(CorrelatedError::DuplicateId(a.id.clone())); }
        if !a.line_of_sight_rad.is_finite() { return Err(CorrelatedError::InvalidBearing(a.id.clone())); }
        let v=a.independent_sigma_m*a.independent_sigma_m;
        if !a.independent_sigma_m.is_finite() || a.independent_sigma_m<=0.0 || !v.is_finite() || v<=0.0 { return Err(CorrelatedError::InvalidIndependentSigma(a.id.clone())); }
        if a.cost_units==0 { return Err(CorrelatedError::ZeroCost(a.id.clone())); }
        match (&a.common_mode_group,a.common_mode_sigma_m) {
            (None,None)=>{},
            (None,Some(_))=>return Err(CorrelatedError::UnexpectedCommonModeSigma(a.id.clone())),
            (Some(_),None)=>return Err(CorrelatedError::MissingCommonModeSigma(a.id.clone())),
            (Some(g),Some(sigma))=>{
                let vv=sigma*sigma;
                if g.trim().is_empty() || !sigma.is_finite() || sigma<=0.0 || !vv.is_finite() || vv<=0.0 { return Err(CorrelatedError::InvalidCommonMode(a.id.clone())); }
                if let Some(old)=group_sigmas.get(g) { if old.to_bits()!=sigma.to_bits() { return Err(CorrelatedError::InconsistentCommonModeSigma(g.clone())); } }
                else { group_sigmas.insert(g.clone(),sigma); }
            }
        }
    }
    ordered.sort_by(|a,b|a.id.cmp(&b.id));
    Ok(ordered)
}

fn cholesky(a:&[Vec<f64>])->Result<Vec<Vec<f64>>,CorrelatedError>{
    let n=a.len(); let mut l=vec![vec![0.0;n];n];
    for i in 0..n { for j in 0..=i {
        let mut v=a[i][j]; for k in 0..j { v-=l[i][k]*l[j][k]; }
        if i==j { if !v.is_finite()||v<=0.0{return Err(CorrelatedError::NonPositiveDefiniteCovariance);} l[i][j]=v.sqrt(); }
        else { let d=l[j][j]; if !d.is_finite()||d<=0.0{return Err(CorrelatedError::NonPositiveDefiniteCovariance);} l[i][j]=v/d; if !l[i][j].is_finite(){return Err(CorrelatedError::NonPositiveDefiniteCovariance);} }
    }} Ok(l)
}

fn solve_covariance(l:&[Vec<f64>],rhs:&[f64])->Result<Vec<f64>,CorrelatedError>{
    let n=rhs.len(); let mut y=vec![0.0;n];
    for i in 0..n { let mut v=rhs[i]; for j in 0..i {v-=l[i][j]*y[j];} y[i]=v/l[i][i]; if !y[i].is_finite(){return Err(CorrelatedError::NonPositiveDefiniteCovariance);} }
    let mut x=vec![0.0;n];
    for i in (0..n).rev(){let mut v=y[i];for j in (i+1)..n{v-=l[j][i]*x[j];}x[i]=v/l[i][i];if !x[i].is_finite(){return Err(CorrelatedError::NonPositiveDefiniteCovariance);}}
    Ok(x)
}

/// Compute J = H^T R^-1 H for an explicit shared additive-error model.
pub fn correlated_fisher(actions:&[CorrelatedRangeAction])->Result<Fisher2,CorrelatedError>{
    let ordered=canonical(actions)?; let n=ordered.len(); if n==0{return Ok(Fisher2::default());}
    let mut r=vec![vec![0.0;n];n]; let mut h=Vec::with_capacity(n); let mut groups=BTreeMap::new();
    for (i,a) in ordered.iter().enumerate(){r[i][i]=a.independent_sigma_m*a.independent_sigma_m;h.push([a.line_of_sight_rad.cos(),a.line_of_sight_rad.sin()]);if let(Some(g),Some(s))=(&a.common_mode_group,a.common_mode_sigma_m){groups.insert(g.clone(),s);}}
    for i in 0..n {for j in 0..n {if let(Some(gi),Some(gj))=(&ordered[i].common_mode_group,&ordered[j].common_mode_group){if gi==gj{let s=*groups.get(gi).ok_or_else(||CorrelatedError::MissingCommonModeSigma(ordered[i].id.clone()))?;r[i][j]+=s*s;}}}}
    let l=cholesky(&r)?; let hx:Vec<f64>=h.iter().map(|v|v[0]).collect();let hy:Vec<f64>=h.iter().map(|v|v[1]).collect();
    let rx=solve_covariance(&l,&hx)?;let ry=solve_covariance(&l,&hy)?;
    let xx=hx.iter().zip(&rx).map(|(a,b)|a*b).sum::<f64>();let xy=hx.iter().zip(&ry).map(|(a,b)|a*b).sum::<f64>();let yy=hy.iter().zip(&ry).map(|(a,b)|a*b).sum::<f64>();
    let j=Fisher2{xx,xy,yy};if !xx.is_finite()||!xy.is_finite()||!yy.is_finite()||!j.determinant().is_finite(){return Err(CorrelatedError::NonFiniteInformation);}Ok(j)
}

fn from_actions(actions:&[CorrelatedRangeAction],rank_tolerance:f64,mask:u64)->Result<CorrelatedSelection,CorrelatedError>{
    let chosen:Vec<_>=actions.iter().enumerate().filter(|(i,_)|mask&(1_u64<<i)!=0).map(|(_,a)|a.clone()).collect();
    let cost=chosen.iter().try_fold(0_u32,|s,a|s.checked_add(a.cost_units).ok_or(CorrelatedError::CostOverflow))?;
    let j=correlated_fisher(&chosen)?;let(minimum_eigenvalue,_)=j.eigenvalues();if !minimum_eigenvalue.is_finite(){return Err(CorrelatedError::NonFiniteInformation);}
    Ok(CorrelatedSelection{action_ids:chosen.iter().map(|a|a.id.clone()).collect(),total_cost_units:cost,fisher:j,rank:j.rank(rank_tolerance),minimum_eigenvalue,log_determinant:j.log_determinant(rank_tolerance),a_optimal_trace_inverse:j.a_optimal_trace_inverse(rank_tolerance),condition_number:j.condition_number(rank_tolerance)})
}

fn compare(a:&CorrelatedSelection,b:&CorrelatedSelection,c:DesignCriterion)->Ordering{
    match a.rank.cmp(&b.rank){Ordering::Equal=>{},x=>return x}
    if a.rank<2{return a.fisher.trace().total_cmp(&b.fisher.trace());}
    match c {DesignCriterion::DOptimal=>a.log_determinant.unwrap_or(f64::NEG_INFINITY).total_cmp(&b.log_determinant.unwrap_or(f64::NEG_INFINITY)),DesignCriterion::AOptimal=>b.a_optimal_trace_inverse.unwrap_or(f64::INFINITY).total_cmp(&a.a_optimal_trace_inverse.unwrap_or(f64::INFINITY)),DesignCriterion::EOptimal=>a.minimum_eigenvalue.total_cmp(&b.minimum_eigenvalue)}
}
fn prefer(a:&CorrelatedSelection,b:&CorrelatedSelection,c:DesignCriterion)->bool{match compare(a,b,c){Ordering::Greater=>true,Ordering::Less=>false,Ordering::Equal=>match a.total_cost_units.cmp(&b.total_cost_units){Ordering::Less=>true,Ordering::Greater=>false,Ordering::Equal=>a.action_ids<b.action_ids}}}

pub fn exhaustive_select_correlated(actions:&[CorrelatedRangeAction],budget:u32,criterion:DesignCriterion)->Result<CorrelatedSelection,CorrelatedError>{exhaustive_select_correlated_with_tolerance(actions,budget,criterion,1.0e-10)}
pub fn exhaustive_select_correlated_with_tolerance(actions:&[CorrelatedRangeAction],budget:u32,criterion:DesignCriterion,tol:f64)->Result<CorrelatedSelection,CorrelatedError>{
    if !tol.is_finite()||tol<0.0{return Err(CorrelatedError::InvalidRankTolerance);}let ordered=canonical(actions)?;let mut best=None;
    for mask in 0..(1_u64<<ordered.len()) {let mut cost=0_u32;let mut feasible=true;for(i,a)in ordered.iter().enumerate(){if mask&(1_u64<<i)!=0{match cost.checked_add(a.cost_units){Some(next)if next<=budget=>cost=next,_=>{feasible=false;break;}}}}}if !feasible{continue;}let s=from_actions(&ordered,tol,mask)?;if best.as_ref().is_none_or(|b|prefer(&s,b,criterion)){best=Some(s);}}
    best.ok_or(CorrelatedError::NonFiniteInformation)
}

pub fn greedy_select_correlated(actions:&[CorrelatedRangeAction],budget:u32,criterion:DesignCriterion)->Result<CorrelatedSelection,CorrelatedError>{
    let ordered=canonical(actions)?;let mut current=from_actions(&ordered,1.0e-10,0)?;
    loop{let mut best_next=None;let ids:BTreeSet<&str>=current.action_ids.iter().map(String::as_str).collect();
        for(i,a)in ordered.iter().enumerate(){if ids.contains(a.id.as_str()){continue;}let Some(next_cost)=current.total_cost_units.checked_add(a.cost_units)else{continue;};if next_cost>budget{continue;}
            let mask=ordered.iter().enumerate().filter(|(_,x)|ids.contains(x.id.as_str())).fold(1_u64<<i,|m,(j,_)|m|(1_u64<<j));let candidate=from_actions(&ordered,1.0e-10,mask)?;
            if compare(&candidate,&current,criterion)!=Ordering::Greater{continue;}if best_next.as_ref().is_none_or(|b|prefer(&candidate,b,criterion)){best_next=Some(candidate);}}
        match best_next{Some(s)=>current=s,None=>return Ok(current)}
    }
}

#[cfg(test)]
mod tests {
 use super::*;
 #[test] fn duplicate_same_bearing_observations_do_not_gain_independent_precision(){let a=vec![CorrelatedRangeAction::independent("a",0.0,1.0,1),CorrelatedRangeAction::independent("b",0.0,1.0,1)];let b=vec![CorrelatedRangeAction::with_common_mode("a",0.0,1.0,"clock-antenna-A",1.0,1),CorrelatedRangeAction::with_common_mode("b",0.0,1.0,"clock-antenna-A",1.0,1)];let ja=correlated_fisher(&a).unwrap();let jb=correlated_fisher(&b).unwrap();assert!((ja.xx-2.0).abs()<1e-12);assert!((jb.xx-2.0/3.0).abs()<1e-12);}
 #[test] fn shared_clock_bias_is_visible_in_cross_axis_information(){let a=vec![CorrelatedRangeAction::with_common_mode("east",0.0,1.0,"shared-clock",1.0,1),CorrelatedRangeAction::with_common_mode("north",std::f64::consts::FRAC_PI_2,1.0,"shared-clock",1.0,1)];let j=correlated_fisher(&a).unwrap();assert!((j.xx-2.0/3.0).abs()<1e-12);assert!((j.xy+1.0/3.0).abs()<1e-12);assert!((j.yy-2.0/3.0).abs()<1e-12);}
 #[test] fn same_domain_repeated_measurements_saturate_to_shared_error_floor(){let mut previous=0.0;for n in [2_usize,4,8,16]{let a:Vec<_>=(0..n).map(|i|CorrelatedRangeAction::with_common_mode(format!("rx-{i}"),0.0,0.1,"shared-correction",1.0,1)).collect();let info=correlated_fisher(&a).unwrap().xx;assert!(info>previous);assert!(info<1.0);previous=info;}assert!(previous>0.99);}
 #[test] fn distinct_declared_common_mode_groups_do_not_share_the_same_latent_error(){let a=vec![CorrelatedRangeAction::with_common_mode("a",0.0,1.0,"antenna-A",1.0,1),CorrelatedRangeAction::with_common_mode("b",0.0,1.0,"antenna-B",1.0,1)];assert!((correlated_fisher(&a).unwrap().xx-1.0).abs()<1e-12);}
 #[test] fn permutation_of_inputs_preserves_correlated_information(){let a=vec![CorrelatedRangeAction::with_common_mode("a",0.0,0.5,"shared",0.8,1),CorrelatedRangeAction::with_common_mode("b",0.7,0.6,"shared",0.8,1),CorrelatedRangeAction::independent("c",1.4,0.9,1)];let x=correlated_fisher(&a).unwrap();let mut rev=a;rev.reverse();let y=correlated_fisher(&rev).unwrap();assert!((x.xx-y.xx).abs()<1e-12);assert!((x.xy-y.xy).abs()<1e-12);assert!((x.yy-y.yy).abs()<1e-12);}
 #[test] fn different_common_mode_sigmas_in_one_group_are_rejected(){let a=vec![CorrelatedRangeAction::with_common_mode("a",0.0,1.0,"same",0.5,1),CorrelatedRangeAction::with_common_mode("b",1.0,1.0,"same",0.7,1)];assert_eq!(correlated_fisher(&a),Err(CorrelatedError::InconsistentCommonModeSigma("same".to_owned())));}
 #[test] fn criterion_oracle_and_greedy_use_the_correlated_model(){let a=vec![CorrelatedRangeAction::with_common_mode("a-colocated-1",0.0,1.0,"shared-front-end",1.0,1),CorrelatedRangeAction::with_common_mode("b-colocated-2",0.0,1.0,"shared-front-end",1.0,1),CorrelatedRangeAction::independent("c-north",std::f64::consts::FRAC_PI_2,1.0,1),CorrelatedRangeAction::independent("d-diagonal",std::f64::consts::FRAC_PI_4,1.0,1)];let x=exhaustive_select_correlated(&a,2,DesignCriterion::DOptimal).unwrap();let y=greedy_select_correlated(&a,2,DesignCriterion::DOptimal).unwrap();assert!(x.total_cost_units<=2);assert!(y.total_cost_units<=2);assert_eq!(x.rank,2);assert_eq!(y.rank,2);assert!(!x.action_ids.contains(&"b-colocated-2".to_owned())||!x.action_ids.contains(&"a-colocated-1".to_owned()));}
 #[test] fn missing_or_unexpected_common_mode_parameters_fail_closed(){let a=CorrelatedRangeAction{id:"a".into(),line_of_sight_rad:0.0,independent_sigma_m:1.0,cost_units:1,common_mode_group:Some("g".into()),common_mode_sigma_m:None};assert_eq!(correlated_fisher(&[a]),Err(CorrelatedError::MissingCommonModeSigma("a".to_owned())));let b=CorrelatedRangeAction{id:"b".into(),line_of_sight_rad:0.0,independent_sigma_m:1.0,cost_units:1,common_mode_group:None,common_mode_sigma_m:Some(1.0)};assert_eq!(correlated_fisher(&[b]),Err(CorrelatedError::UnexpectedCommonModeSigma("b".to_owned())));}
}
