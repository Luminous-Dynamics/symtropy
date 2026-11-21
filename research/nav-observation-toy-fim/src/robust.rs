//! Finite-envelope sensitivity-aware geometry selection.
//!
//! Keep at least a declared fraction of nominal optimal information, then
//! maximize worst-case retention over explicit frozen perturbation scenarios.
//! This is exact only over the finite subset and scenario sets supplied.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::DesignCriterion;
use crate::correlated::{correlated_fisher, CorrelatedError, CorrelatedRangeAction, CorrelatedSelection};

const MAX_ACTIONS: usize = 16;
const RANK_TOLERANCE: f64 = 1.0e-10;
const EPSILON: f64 = 1.0e-12;

#[derive(Clone, Debug, PartialEq)]
pub struct ActionPerturbation {
    pub action_id: String,
    pub bearing_delta_rad: f64,
    pub private_sigma_scale: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PerturbationScenario {
    pub scenario_id: String,
    pub actions: Vec<ActionPerturbation>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ScenarioRetention {
    pub scenario_id: String,
    pub rank: u8,
    pub quality: Option<f64>,
    pub retention: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RobustSelection {
    pub nominal_optimum: CorrelatedSelection,
    pub selected_nominal: CorrelatedSelection,
    pub worst_case: CorrelatedSelection,
    pub worst_case_scenario_id: String,
    pub nominal_information_retention: f64,
    pub worst_case_information_retention: f64,
    pub scenario_results: Vec<ScenarioRetention>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum RobustDesignError {
    Model(CorrelatedError),
    TooManyActions(usize),
    EmptyScenarioId,
    DuplicateScenarioId(String),
    NoScenarios,
    InvalidRetentionFloor,
    InvalidScenario(String),
    MissingAction(String),
    UnknownAction(String),
    DuplicateAction(String),
    NoFullRankNominalPlan,
    NoCandidateMeetsRetention,
    CostOverflow,
}

impl fmt::Display for RobustDesignError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Model(e) => write!(f, "correlated model error: {e}"),
            Self::TooManyActions(n) => write!(f, "{n} actions exceed robust oracle limit {MAX_ACTIONS}"),
            Self::EmptyScenarioId => write!(f, "scenario ID must not be empty"),
            Self::DuplicateScenarioId(id) => write!(f, "duplicate scenario ID: {id}"),
            Self::NoScenarios => write!(f, "at least one perturbation scenario is required"),
            Self::InvalidRetentionFloor => write!(f, "retention floor must be finite and in (0, 1]"),
            Self::InvalidScenario(id) => write!(f, "invalid perturbation values in scenario: {id}"),
            Self::MissingAction(id) => write!(f, "scenario is missing action: {id}"),
            Self::UnknownAction(id) => write!(f, "scenario references unknown action: {id}"),
            Self::DuplicateAction(id) => write!(f, "scenario repeats action: {id}"),
            Self::NoFullRankNominalPlan => write!(f, "no full-rank nominal plan exists within budget"),
            Self::NoCandidateMeetsRetention => write!(f, "no subset meets the nominal information-retention floor"),
            Self::CostOverflow => write!(f, "total candidate cost overflowed u32"),
        }
    }
}
impl std::error::Error for RobustDesignError {}
impl From<CorrelatedError> for RobustDesignError {
    fn from(value: CorrelatedError) -> Self { Self::Model(value) }
}

fn quality(s: &CorrelatedSelection, criterion: DesignCriterion) -> Option<f64> {
    if s.rank < 2 { return None; }
    let q = match criterion {
        DesignCriterion::DOptimal => s.fisher.determinant(),
        DesignCriterion::AOptimal => {
            let v = s.a_optimal_trace_inverse?;
            if !v.is_finite() || v <= 0.0 { return None; }
            1.0 / v
        }
        DesignCriterion::EOptimal => s.minimum_eigenvalue,
    };
    (q.is_finite() && q > 0.0).then_some(q)
}

fn build_selection(
    actions: &[CorrelatedRangeAction],
    mask: u64,
) -> Result<CorrelatedSelection, RobustDesignError> {
    let chosen: Vec<_> = actions.iter().enumerate()
        .filter(|(i, _)| mask & (1_u64 << i) != 0)
        .map(|(_, a)| a.clone()).collect();
    let total_cost_units = chosen.iter().try_fold(0_u32, |sum, a| {
        sum.checked_add(a.cost_units).ok_or(RobustDesignError::CostOverflow)
    })?;
    let fisher = correlated_fisher(&chosen)?;
    let (minimum_eigenvalue, _) = fisher.eigenvalues();
    if !minimum_eigenvalue.is_finite() {
        return Err(RobustDesignError::Model(CorrelatedError::NonFiniteInformation));
    }
    Ok(CorrelatedSelection {
        action_ids: chosen.iter().map(|a| a.id.clone()).collect(),
        total_cost_units,
        fisher,
        rank: fisher.rank(RANK_TOLERANCE),
        minimum_eigenvalue,
        log_determinant: fisher.log_determinant(RANK_TOLERANCE),
        a_optimal_trace_inverse: fisher.a_optimal_trace_inverse(RANK_TOLERANCE),
        condition_number: fisher.condition_number(RANK_TOLERANCE),
    })
}

fn validate_scenarios(
    actions: &[CorrelatedRangeAction],
    scenarios: &[PerturbationScenario],
) -> Result<Vec<BTreeMap<String, ActionPerturbation>>, RobustDesignError> {
    if scenarios.is_empty() { return Err(RobustDesignError::NoScenarios); }
    let known: BTreeSet<String> = actions.iter().map(|a| a.id.clone()).collect();
    let mut scenario_ids = BTreeSet::new();
    let mut all = Vec::with_capacity(scenarios.len());

    for scenario in scenarios {
        if scenario.scenario_id.trim().is_empty() { return Err(RobustDesignError::EmptyScenarioId); }
        if !scenario_ids.insert(scenario.scenario_id.clone()) {
            return Err(RobustDesignError::DuplicateScenarioId(scenario.scenario_id.clone()));
        }
        let mut map = BTreeMap::new();
        for p in &scenario.actions {
            if !known.contains(&p.action_id) { return Err(RobustDesignError::UnknownAction(p.action_id.clone())); }
            if !p.bearing_delta_rad.is_finite()
                || !p.private_sigma_scale.is_finite()
                || p.private_sigma_scale <= 0.0
            { return Err(RobustDesignError::InvalidScenario(scenario.scenario_id.clone())); }
            if map.insert(p.action_id.clone(), p.clone()).is_some() {
                return Err(RobustDesignError::DuplicateAction(p.action_id.clone()));
            }
        }
        for id in &known {
            if !map.contains_key(id) { return Err(RobustDesignError::MissingAction(id.clone())); }
        }
        all.push(map);
    }
    Ok(all)
}

fn perturb_subset(
    subset: &[CorrelatedRangeAction],
    perturbations: &BTreeMap<String, ActionPerturbation>,
) -> Result<Vec<CorrelatedRangeAction>, RobustDesignError> {
    subset.iter().map(|a| {
        let p = perturbations.get(&a.id).ok_or_else(|| RobustDesignError::MissingAction(a.id.clone()))?;
        let mut changed = a.clone();
        changed.line_of_sight_rad += p.bearing_delta_rad;
        changed.independent_sigma_m *= p.private_sigma_scale;
        if !changed.line_of_sight_rad.is_finite()
            || !changed.independent_sigma_m.is_finite()
            || changed.independent_sigma_m <= 0.0
        { return Err(RobustDesignError::InvalidScenario(a.id.clone())); }
        Ok(changed)
    }).collect()
}

fn better(a: &RobustSelection, b: &RobustSelection) -> bool {
    match a.worst_case_information_retention.total_cmp(&b.worst_case_information_retention) {
        Ordering::Greater => true,
        Ordering::Less => false,
        Ordering::Equal => match a.nominal_information_retention.total_cmp(&b.nominal_information_retention) {
            Ordering::Greater => true,
            Ordering::Less => false,
            Ordering::Equal => match a.selected_nominal.total_cost_units.cmp(&b.selected_nominal.total_cost_units) {
                Ordering::Less => true,
                Ordering::Greater => false,
                Ordering::Equal => a.selected_nominal.action_ids < b.selected_nominal.action_ids,
            }
        }
    }
}

/// Exact subset search over a finite, caller-declared perturbation envelope.
/// The nominal-retention floor prevents robustness from selecting a geometry
/// with arbitrarily poor nominal information.
pub fn robust_select_correlated(
    actions: &[CorrelatedRangeAction],
    budget: u32,
    criterion: DesignCriterion,
    scenarios: &[PerturbationScenario],
    nominal_retention_floor: f64,
) -> Result<RobustSelection, RobustDesignError> {
    if !nominal_retention_floor.is_finite() || nominal_retention_floor <= 0.0 || nominal_retention_floor > 1.0 {
        return Err(RobustDesignError::InvalidRetentionFloor);
    }
    if actions.len() > MAX_ACTIONS { return Err(RobustDesignError::TooManyActions(actions.len())); }
    correlated_fisher(actions)?;
    let scenario_maps = validate_scenarios(actions, scenarios)?;
    let mut ordered = actions.to_vec();
    ordered.sort_by(|a, b| a.id.cmp(&b.id));

    let nominal_optimum = crate::correlated::exhaustive_select_correlated(&ordered, budget, criterion)?;
    let best_quality = quality(&nominal_optimum, criterion).ok_or(RobustDesignError::NoFullRankNominalPlan)?;
    let mut best: Option<RobustSelection> = None;

    for mask in 1..(1_u64 << ordered.len()) {
        let mut cost = 0_u32;
        let mut feasible = true;
        for (i, a) in ordered.iter().enumerate() {
            if mask & (1_u64 << i) != 0 {
                match cost.checked_add(a.cost_units) {
                    Some(v) if v <= budget => cost = v,
                    _ => { feasible = false; break; }
                }
            }
        }
        if !feasible { continue; }

        let subset: Vec<_> = ordered.iter().enumerate()
            .filter(|(i, _)| mask & (1_u64 << i) != 0)
            .map(|(_, a)| a.clone()).collect();
        let full_mask = (1_u64 << subset.len()) - 1;
        let nominal = build_selection(&subset, full_mask)?;
        let Some(q_nominal) = quality(&nominal, criterion) else { continue; };
        let nominal_retention = (q_nominal / best_quality).min(1.0);
        if nominal_retention + EPSILON < nominal_retention_floor { continue; }

        let mut scenario_results = Vec::with_capacity(scenarios.len());
        let mut worst: Option<CorrelatedSelection> = None;
        let mut worst_id = String::new();
        let mut worst_retention = 1.0_f64;
        let mut worst_quality = f64::INFINITY;

        for (scenario, map) in scenarios.iter().zip(&scenario_maps) {
            let perturbed = perturb_subset(&subset, map)?;
            let scenario_selection = build_selection(&perturbed, (1_u64 << perturbed.len()) - 1)?;
            let q = quality(&scenario_selection, criterion);
            let retention = q.map(|v| (v / q_nominal).clamp(0.0, 1.0)).unwrap_or(0.0);
            scenario_results.push(ScenarioRetention {
                scenario_id: scenario.scenario_id.clone(),
                rank: scenario_selection.rank,
                quality: q,
                retention,
            });
            if retention < worst_retention || (retention == worst_retention && q.unwrap_or(0.0) < worst_quality) {
                worst_retention = retention;
                worst_quality = q.unwrap_or(0.0);
                worst_id = scenario.scenario_id.clone();
                worst = Some(scenario_selection);
            }
        }

        let candidate = RobustSelection {
            nominal_optimum: nominal_optimum.clone(),
            selected_nominal: nominal,
            worst_case: worst.expect("at least one validated scenario"),
            worst_case_scenario_id: worst_id,
            nominal_information_retention: nominal_retention,
            worst_case_information_retention: worst_retention,
            scenario_results,
        };
        if best.as_ref().is_none_or(|current| better(&candidate, current)) {
            best = Some(candidate);
        }
    }

    best.ok_or(RobustDesignError::NoCandidateMeetsRetention)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn action(id: &str, deg: f64, sigma: f64) -> CorrelatedRangeAction {
        CorrelatedRangeAction::independent(id, deg.to_radians(), sigma, 1)
    }
    fn p(id: &str, delta: f64) -> ActionPerturbation {
        ActionPerturbation { action_id: id.into(), bearing_delta_rad: delta, private_sigma_scale: 1.0 }
    }
    fn sc(id: &str, values: Vec<ActionPerturbation>) -> PerturbationScenario {
        PerturbationScenario { scenario_id: id.into(), actions: values }
    }
    fn candidates() -> Vec<CorrelatedRangeAction> {
        vec![
            action("fragile-east", 0.0, 0.2),
            action("fragile-north", 90.0, 0.2),
            action("stable-diagonal", 45.0, 0.205),
            action("stable-antidiagonal", 135.0, 0.205),
        ]
    }
    fn frozen() -> Vec<PerturbationScenario> {
        vec![
            sc("positive", vec![p("fragile-east",0.35),p("fragile-north",-0.35),p("stable-diagonal",0.01),p("stable-antidiagonal",-0.01)]),
            sc("negative", vec![p("fragile-east",-0.35),p("fragile-north",0.35),p("stable-diagonal",-0.01),p("stable-antidiagonal",0.01)]),
        ]
    }

    #[test]
    fn sensitivity_selection_spends_about_ten_percent_nominal_information_for_robustness() {
        let actions = candidates();
        let nominal = crate::correlated::exhaustive_select_correlated(&actions,2,DesignCriterion::DOptimal).unwrap();
        assert_eq!(nominal.action_ids,vec!["fragile-east".to_owned(),"fragile-north".to_owned()]);
        let robust = robust_select_correlated(&actions,2,DesignCriterion::DOptimal,&frozen(),0.90).unwrap();
        assert_eq!(robust.selected_nominal.action_ids,vec!["stable-antidiagonal".to_owned(),"stable-diagonal".to_owned()]);
        assert!(robust.nominal_information_retention>=0.90);
        assert!(robust.worst_case_information_retention>0.99);
    }

    #[test]
    fn rank_loss_in_every_candidate_geometry_produces_zero_retention() {
        let actions=vec![action("east",0.0,1.0),action("north",90.0,1.0),action("diagonal",45.0,1.0)];
        let scenario=sc("collapse",vec![p("east",0.0),p("north",-std::f64::consts::FRAC_PI_2),p("diagonal",-std::f64::consts::FRAC_PI_4)]);
        let robust=robust_select_correlated(&actions,2,DesignCriterion::DOptimal,&[scenario],0.5).unwrap();
        assert_eq!(robust.worst_case.rank,1);
        assert_eq!(robust.worst_case_information_retention,0.0);
    }

    #[test]
    fn missing_duplicate_and_unknown_action_perturbations_fail_closed() {
        let actions=vec![action("east",0.0,1.0),action("north",90.0,1.0)];
        let missing=vec![sc("missing",vec![p("east",0.0)])];
        assert!(matches!(robust_select_correlated(&actions,2,DesignCriterion::DOptimal,&missing,0.5),Err(RobustDesignError::MissingAction(id)) if id=="north"));
        let duplicate=vec![sc("duplicate",vec![p("east",0.0),p("east",0.1),p("north",0.0)])];
        assert!(matches!(robust_select_correlated(&actions,2,DesignCriterion::DOptimal,&duplicate,0.5),Err(RobustDesignError::DuplicateAction(id)) if id=="east"));
        let unknown=vec![sc("unknown",vec![p("east",0.0),p("north",0.0),p("ghost",0.0)])];
        assert!(matches!(robust_select_correlated(&actions,2,DesignCriterion::DOptimal,&unknown,0.5),Err(RobustDesignError::UnknownAction(id)) if id=="ghost"));
    }

    #[test]
    fn retention_floor_and_scenario_ids_are_validated() {
        let actions=candidates();
        assert_eq!(robust_select_correlated(&actions,2,DesignCriterion::DOptimal,&frozen(),1.1),Err(RobustDesignError::InvalidRetentionFloor));
        let duplicate=vec![frozen()[0].clone(),frozen()[0].clone()];
        assert!(matches!(robust_select_correlated(&actions,2,DesignCriterion::DOptimal,&duplicate,0.9),Err(RobustDesignError::DuplicateScenarioId(id)) if id=="positive"));
    }

    #[test]
    fn robust_selection_is_permutation_invariant() {
        let mut actions=candidates();
        let scenarios=frozen();
        let a=robust_select_correlated(&actions,2,DesignCriterion::DOptimal,&scenarios,0.90).unwrap();
        actions.reverse();
        let b=robust_select_correlated(&actions,2,DesignCriterion::DOptimal,&scenarios,0.90).unwrap();
        assert_eq!(a.selected_nominal.action_ids,b.selected_nominal.action_ids);
        assert!((a.worst_case_information_retention-b.worst_case_information_retention).abs()<1e-12);
    }
}
