use crate::{Case, Disposition, evaluate_case, sha256_hex};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const PLANNING_RECEIPT_SCHEMA: &str =
    "luminous.orbital-truss-inspectability.planning-receipt.v0.2";
pub const MAX_EXACT_VIEWS: usize = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PlanningDisposition {
    Feasible,
    Infeasible,
    Indeterminate,
    BudgetInfeasible,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlanningFeature {
    pub feature_id: String,
    pub required_view_count: usize,
    pub required_failure_domains: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FeatureCoverage {
    pub feature_id: String,
    pub required_view_count: usize,
    pub nominal_view_count: usize,
    pub robust_view_count: usize,
    pub required_failure_domains: usize,
    pub nominal_failure_domain_count: usize,
    pub robust_failure_domain_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlanningView {
    pub view_id: String,
    pub failure_domain_id: String,
    pub cost_units: u64,
    pub nominal_feature_ids: Vec<String>,
    pub robust_feature_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlanningProblem {
    pub case_id: String,
    pub coordinate_frame_id: String,
    pub features: Vec<PlanningFeature>,
    pub views: Vec<PlanningView>,
    pub max_selected_views: usize,
    pub max_total_cost_units: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PlanSolution {
    pub selected_view_ids: Vec<String>,
    pub total_cost_units: u64,
    pub selected_view_count: usize,
    pub robustly_covered: bool,
    pub uncovered_feature_ids: Vec<String>,
    pub undercovered_view_count_feature_ids: Vec<String>,
    pub undercovered_failure_domain_feature_ids: Vec<String>,
    pub feature_coverage: Vec<FeatureCoverage>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PlanningReceipt {
    pub schema: String,
    pub case_id: String,
    pub coordinate_frame_id: String,
    pub input_sha256: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub candidate_geometry_disposition: Disposition,
    pub planning_disposition: PlanningDisposition,
    pub candidate_view_count: usize,
    pub candidate_pool_coverage: Vec<FeatureCoverage>,
    pub max_selected_views: usize,
    pub max_total_cost_units: u64,
    pub subsets_evaluated: u64,
    pub budget_eligible_subsets: u64,
    pub nominally_feasible_subsets: u64,
    pub robustly_feasible_subsets: u64,
    pub exact_optimum: Option<PlanSolution>,
    pub pareto_frontier: Vec<PlanSolution>,
    pub greedy_next_best_view: PlanSolution,
    pub cheapest_first: PlanSolution,
    pub greedy_cost_gap_units: Option<u64>,
    pub cheapest_first_cost_gap_units: Option<u64>,
    pub authority_granted: bool,
    pub structural_acceptance_authorized: bool,
    pub flight_authorized: bool,
    pub physical_validation: String,
}

#[derive(Debug, Clone)]
struct ViewCoverage {
    domain_index: usize,
    cost_units: u64,
    nominal_features: Vec<usize>,
    robust_features: Vec<usize>,
}

#[derive(Debug, Default)]
struct SearchState {
    nominal_domain_refs: Vec<Vec<u8>>,
    robust_domain_refs: Vec<Vec<u8>>,
    nominal_view_counts: Vec<usize>,
    robust_view_counts: Vec<usize>,
    nominal_distinct: Vec<usize>,
    robust_distinct: Vec<usize>,
    selected_mask: u32,
    selected_count: usize,
    selected_cost: u64,
}

#[derive(Debug, Default)]
struct SearchResults {
    subsets_evaluated: u64,
    budget_eligible_subsets: u64,
    nominally_feasible_subsets: u64,
    robustly_feasible_subsets: u64,
    exact_optimum: Option<PlanSolution>,
    best_by_cost: BTreeMap<u64, PlanSolution>,
}

struct ExactSearch<'a> {
    problem: &'a PlanningProblem,
    views: Vec<ViewCoverage>,
    required_view_counts: Vec<usize>,
    required_failure_domains: Vec<usize>,
    view_ids: Vec<String>,
    results: SearchResults,
}

impl<'a> ExactSearch<'a> {
    fn add_view(&self, state: &mut SearchState, index: usize) {
        let view = &self.views[index];
        for feature in &view.nominal_features {
            state.nominal_view_counts[*feature] += 1;
            let references = &mut state.nominal_domain_refs[*feature][view.domain_index];
            if *references == 0 {
                state.nominal_distinct[*feature] += 1;
            }
            *references += 1;
        }
        for feature in &view.robust_features {
            state.robust_view_counts[*feature] += 1;
            let references = &mut state.robust_domain_refs[*feature][view.domain_index];
            if *references == 0 {
                state.robust_distinct[*feature] += 1;
            }
            *references += 1;
        }
        state.selected_mask |= 1_u32 << index;
        state.selected_count += 1;
        state.selected_cost += view.cost_units;
    }

    fn remove_view(&self, state: &mut SearchState, index: usize) {
        let view = &self.views[index];
        for feature in &view.nominal_features {
            state.nominal_view_counts[*feature] -= 1;
            let references = &mut state.nominal_domain_refs[*feature][view.domain_index];
            *references -= 1;
            if *references == 0 {
                state.nominal_distinct[*feature] -= 1;
            }
        }
        for feature in &view.robust_features {
            state.robust_view_counts[*feature] -= 1;
            let references = &mut state.robust_domain_refs[*feature][view.domain_index];
            *references -= 1;
            if *references == 0 {
                state.robust_distinct[*feature] -= 1;
            }
        }
        state.selected_mask &= !(1_u32 << index);
        state.selected_count -= 1;
        state.selected_cost -= view.cost_units;
    }

    fn solution(&self, state: &SearchState) -> PlanSolution {
        let mut selected_view_ids = self
            .view_ids
            .iter()
            .enumerate()
            .filter(|(index, _)| state.selected_mask & (1_u32 << index) != 0)
            .map(|(_, view_id)| view_id.clone())
            .collect::<Vec<_>>();
        selected_view_ids.sort();

        let mut uncovered_feature_ids = Vec::new();
        let mut undercovered_view_count_feature_ids = Vec::new();
        let mut undercovered_failure_domain_feature_ids = Vec::new();
        let feature_coverage = self
            .problem
            .features
            .iter()
            .enumerate()
            .map(|(index, feature)| {
                let nominal_failure_domain_count = state.nominal_distinct[index];
                let robust_failure_domain_count = state.robust_distinct[index];
                let nominal_view_count = state.nominal_view_counts[index];
                let robust_view_count = state.robust_view_counts[index];
                if robust_view_count < self.required_view_counts[index] {
                    undercovered_view_count_feature_ids.push(feature.feature_id.clone());
                }
                if robust_failure_domain_count < self.required_failure_domains[index] {
                    undercovered_failure_domain_feature_ids.push(feature.feature_id.clone());
                }
                if robust_view_count < self.required_view_counts[index]
                    || robust_failure_domain_count < self.required_failure_domains[index]
                {
                    uncovered_feature_ids.push(feature.feature_id.clone());
                }
                FeatureCoverage {
                    feature_id: feature.feature_id.clone(),
                    required_view_count: feature.required_view_count,
                    nominal_view_count,
                    robust_view_count,
                    required_failure_domains: feature.required_failure_domains,
                    nominal_failure_domain_count,
                    robust_failure_domain_count,
                }
            })
            .collect::<Vec<_>>();

        PlanSolution {
            selected_view_ids,
            total_cost_units: state.selected_cost,
            selected_view_count: state.selected_count,
            robustly_covered: uncovered_feature_ids.is_empty(),
            uncovered_feature_ids,
            undercovered_view_count_feature_ids,
            undercovered_failure_domain_feature_ids,
            feature_coverage,
        }
    }

    fn visit(&mut self, index: usize, state: &mut SearchState) {
        if index == self.views.len() {
            self.results.subsets_evaluated += 1;
            if state.selected_count > self.problem.max_selected_views
                || state.selected_cost > self.problem.max_total_cost_units
            {
                return;
            }

            self.results.budget_eligible_subsets += 1;
            let nominally_feasible = state
                .nominal_view_counts
                .iter()
                .zip(&self.required_view_counts)
                .all(|(actual, required)| actual >= required)
                && state
                    .nominal_distinct
                    .iter()
                    .zip(&self.required_failure_domains)
                    .all(|(actual, required)| actual >= required);
            let robustly_feasible = state
                .robust_view_counts
                .iter()
                .zip(&self.required_view_counts)
                .all(|(actual, required)| actual >= required)
                && state
                    .robust_distinct
                    .iter()
                    .zip(&self.required_failure_domains)
                    .all(|(actual, required)| actual >= required);

            if nominally_feasible {
                self.results.nominally_feasible_subsets += 1;
            }
            if !robustly_feasible {
                return;
            }

            self.results.robustly_feasible_subsets += 1;
            let candidate = self.solution(state);
            let replace_exact = self
                .results
                .exact_optimum
                .as_ref()
                .is_none_or(|current| solution_key(&candidate) < solution_key(current));
            if replace_exact {
                self.results.exact_optimum = Some(candidate.clone());
            }

            let replace_cost = self
                .results
                .best_by_cost
                .get(&candidate.total_cost_units)
                .is_none_or(|current| solution_key(&candidate) < solution_key(current));
            if replace_cost {
                self.results
                    .best_by_cost
                    .insert(candidate.total_cost_units, candidate);
            }
            return;
        }

        self.visit(index + 1, state);
        self.add_view(state, index);
        self.visit(index + 1, state);
        self.remove_view(state, index);
    }
}

fn solution_key(solution: &PlanSolution) -> (u64, usize, &[String]) {
    (
        solution.total_cost_units,
        solution.selected_view_count,
        &solution.selected_view_ids,
    )
}

fn validate_problem(problem: &PlanningProblem) -> Result<(), String> {
    if problem.case_id.trim().is_empty()
        || problem.coordinate_frame_id.trim().is_empty()
        || problem.features.is_empty()
        || problem.views.is_empty()
        || problem.views.len() > MAX_EXACT_VIEWS
        || problem.max_selected_views > problem.views.len()
    {
        return Err("PLANNING_PROBLEM_BOUNDS_INVALID".to_owned());
    }

    let mut feature_ids = BTreeSet::new();
    for feature in &problem.features {
        if feature.feature_id.trim().is_empty()
            || !feature_ids.insert(feature.feature_id.as_str())
            || feature.required_view_count == 0
            || feature.required_failure_domains == 0
        {
            return Err("PLANNING_FEATURE_INVALID".to_owned());
        }
    }

    let known_features = feature_ids;
    let mut view_ids = BTreeSet::new();
    let mut total_cost = 0_u64;
    for view in &problem.views {
        if view.view_id.trim().is_empty()
            || !view_ids.insert(view.view_id.as_str())
            || view.failure_domain_id.trim().is_empty()
            || view.cost_units == 0
        {
            return Err("PLANNING_VIEW_INVALID".to_owned());
        }
        total_cost = total_cost
            .checked_add(view.cost_units)
            .ok_or_else(|| "PLANNING_TOTAL_COST_OVERFLOW".to_owned())?;

        let nominal: BTreeSet<&str> = view
            .nominal_feature_ids
            .iter()
            .map(String::as_str)
            .collect();
        let robust: BTreeSet<&str> = view.robust_feature_ids.iter().map(String::as_str).collect();
        if nominal.len() != view.nominal_feature_ids.len()
            || robust.len() != view.robust_feature_ids.len()
            || !nominal.is_subset(&known_features)
            || !robust.is_subset(&nominal)
        {
            return Err("PLANNING_VIEW_COVERAGE_INVALID".to_owned());
        }
    }
    let _ = total_cost;
    Ok(())
}

pub fn problem_from_case(
    case: &Case,
    view_cost_units: &BTreeMap<String, u64>,
    max_selected_views: usize,
    max_total_cost_units: u64,
) -> Result<PlanningProblem, String> {
    let evaluated = evaluate_case(case)?;
    let case_view_ids: BTreeSet<&str> = case
        .views
        .iter()
        .map(|view| view.view_id.as_str())
        .collect();
    if view_cost_units.len() != case_view_ids.len()
        || view_cost_units
            .keys()
            .any(|id| !case_view_ids.contains(id.as_str()))
        || case_view_ids
            .iter()
            .any(|id| !view_cost_units.contains_key(*id))
    {
        return Err("VIEW_COST_MAP_MUST_MATCH_CASE_VIEWS_EXACTLY".to_owned());
    }

    let evaluated_features: BTreeMap<&str, _> = evaluated
        .features
        .iter()
        .map(|feature| (feature.feature_id.as_str(), feature))
        .collect();
    if evaluated_features.len() != case.features.len() {
        return Err("GEOMETRY_RESULT_FEATURE_SET_MISMATCH".to_owned());
    }

    let features = case
        .features
        .iter()
        .map(|feature| PlanningFeature {
            feature_id: feature.feature_id.clone(),
            required_view_count: 1,
            required_failure_domains: feature.minimum_distinct_failure_domains,
        })
        .collect();

    let mut views = Vec::with_capacity(case.views.len());
    for view in &case.views {
        let mut nominal_feature_ids = Vec::new();
        let mut robust_feature_ids = Vec::new();
        for feature in &evaluated.features {
            if feature.nominal_view_ids.contains(&view.view_id) {
                nominal_feature_ids.push(feature.feature_id.clone());
            }
            if feature.robust_view_ids.contains(&view.view_id) {
                robust_feature_ids.push(feature.feature_id.clone());
            }
        }
        views.push(PlanningView {
            view_id: view.view_id.clone(),
            failure_domain_id: view.failure_domain_id.clone(),
            cost_units: *view_cost_units
                .get(&view.view_id)
                .ok_or_else(|| "VIEW_COST_MISSING".to_owned())?,
            nominal_feature_ids,
            robust_feature_ids,
        });
    }

    let problem = PlanningProblem {
        case_id: case.case_id.clone(),
        coordinate_frame_id: case.coordinate_frame_id.clone(),
        features,
        views,
        max_selected_views,
        max_total_cost_units,
    };
    validate_problem(&problem)?;
    Ok(problem)
}

fn coverage_records(problem: &PlanningProblem, selected: &[bool]) -> Vec<FeatureCoverage> {
    let mut nominal_view_counts = vec![0_usize; problem.features.len()];
    let mut robust_view_counts = vec![0_usize; problem.features.len()];
    let mut nominal_domains = vec![BTreeSet::<&str>::new(); problem.features.len()];
    let mut robust_domains = vec![BTreeSet::<&str>::new(); problem.features.len()];
    let feature_indices: BTreeMap<&str, usize> = problem
        .features
        .iter()
        .enumerate()
        .map(|(index, feature)| (feature.feature_id.as_str(), index))
        .collect();

    for (view, is_selected) in problem.views.iter().zip(selected) {
        if !is_selected {
            continue;
        }
        for feature_id in &view.nominal_feature_ids {
            let index = *feature_indices
                .get(feature_id.as_str())
                .expect("validated feature id");
            nominal_view_counts[index] += 1;
            nominal_domains[index].insert(view.failure_domain_id.as_str());
        }
        for feature_id in &view.robust_feature_ids {
            let index = *feature_indices
                .get(feature_id.as_str())
                .expect("validated feature id");
            robust_view_counts[index] += 1;
            robust_domains[index].insert(view.failure_domain_id.as_str());
        }
    }

    problem
        .features
        .iter()
        .enumerate()
        .map(|(index, feature)| FeatureCoverage {
            feature_id: feature.feature_id.clone(),
            required_view_count: feature.required_view_count,
            nominal_view_count: nominal_view_counts[index],
            robust_view_count: robust_view_counts[index],
            required_failure_domains: feature.required_failure_domains,
            nominal_failure_domain_count: nominal_domains[index].len(),
            robust_failure_domain_count: robust_domains[index].len(),
        })
        .collect()
}

fn total_robust_deficit(coverage: &[FeatureCoverage]) -> usize {
    coverage
        .iter()
        .map(|feature| {
            feature
                .required_view_count
                .saturating_sub(feature.robust_view_count)
                + feature
                    .required_failure_domains
                    .saturating_sub(feature.robust_failure_domain_count)
        })
        .sum()
}

fn solution_from_selection(problem: &PlanningProblem, selected: &[bool]) -> PlanSolution {
    let feature_coverage = coverage_records(problem, selected);
    let undercovered_view_count_feature_ids = feature_coverage
        .iter()
        .filter(|feature| feature.robust_view_count < feature.required_view_count)
        .map(|feature| feature.feature_id.clone())
        .collect::<Vec<_>>();
    let undercovered_failure_domain_feature_ids = feature_coverage
        .iter()
        .filter(|feature| feature.robust_failure_domain_count < feature.required_failure_domains)
        .map(|feature| feature.feature_id.clone())
        .collect::<Vec<_>>();
    let mut uncovered_feature_ids = undercovered_view_count_feature_ids.clone();
    uncovered_feature_ids.extend(
        undercovered_failure_domain_feature_ids
            .iter()
            .filter(|id| !uncovered_feature_ids.contains(id))
            .cloned(),
    );
    uncovered_feature_ids.sort();

    let mut selected_view_ids = problem
        .views
        .iter()
        .zip(selected)
        .filter(|(_, include)| **include)
        .map(|(view, _)| view.view_id.clone())
        .collect::<Vec<_>>();
    selected_view_ids.sort();
    let total_cost_units = problem
        .views
        .iter()
        .zip(selected)
        .filter(|(_, include)| **include)
        .map(|(view, _)| view.cost_units)
        .sum();

    PlanSolution {
        selected_view_count: selected_view_ids.len(),
        selected_view_ids,
        total_cost_units,
        robustly_covered: uncovered_feature_ids.is_empty(),
        uncovered_feature_ids,
        undercovered_view_count_feature_ids,
        undercovered_failure_domain_feature_ids,
        feature_coverage,
    }
}

fn greedy_solution(problem: &PlanningProblem, use_ratio: bool) -> PlanSolution {
    let mut selected = vec![false; problem.views.len()];
    let mut selected_count = 0_usize;
    let mut total_cost = 0_u64;

    loop {
        let current_coverage = coverage_records(problem, &selected);
        let current_deficit = total_robust_deficit(&current_coverage);
        if current_deficit == 0 || selected_count >= problem.max_selected_views {
            break;
        }

        let mut best: Option<(usize, usize)> = None;
        for (index, view) in problem.views.iter().enumerate() {
            if selected[index]
                || total_cost.saturating_add(view.cost_units) > problem.max_total_cost_units
            {
                continue;
            }

            selected[index] = true;
            let next_deficit = total_robust_deficit(&coverage_records(problem, &selected));
            selected[index] = false;
            let gain = current_deficit.saturating_sub(next_deficit);
            if gain == 0 {
                continue;
            }

            let replace = match best {
                None => true,
                Some((best_index, best_gain)) => {
                    let best_view = &problem.views[best_index];
                    if use_ratio {
                        let left = (gain as u128) * (best_view.cost_units as u128);
                        let right = (best_gain as u128) * (view.cost_units as u128);
                        left > right
                            || (left == right
                                && (view.cost_units, view.view_id.as_str())
                                    < (best_view.cost_units, best_view.view_id.as_str()))
                    } else {
                        (view.cost_units, view.view_id.as_str())
                            < (best_view.cost_units, best_view.view_id.as_str())
                    }
                }
            };
            if replace {
                best = Some((index, gain));
            }
        }

        let Some((best_index, _)) = best else {
            break;
        };
        selected[best_index] = true;
        selected_count += 1;
        total_cost += problem.views[best_index].cost_units;
    }

    solution_from_selection(problem, &selected)
}

pub fn solve(problem: &PlanningProblem) -> Result<PlanningReceipt, String> {
    validate_problem(problem)?;

    let domains: BTreeSet<&str> = problem
        .views
        .iter()
        .map(|view| view.failure_domain_id.as_str())
        .collect();
    let domain_indices: BTreeMap<&str, usize> = domains
        .iter()
        .enumerate()
        .map(|(index, domain)| (*domain, index))
        .collect();
    let feature_indices: BTreeMap<&str, usize> = problem
        .features
        .iter()
        .enumerate()
        .map(|(index, feature)| (feature.feature_id.as_str(), index))
        .collect();

    let mut views = Vec::with_capacity(problem.views.len());
    for view in &problem.views {
        let nominal_features = view
            .nominal_feature_ids
            .iter()
            .map(|id| {
                *feature_indices
                    .get(id.as_str())
                    .expect("validated feature id")
            })
            .collect();
        let robust_features = view
            .robust_feature_ids
            .iter()
            .map(|id| {
                *feature_indices
                    .get(id.as_str())
                    .expect("validated feature id")
            })
            .collect();
        views.push(ViewCoverage {
            domain_index: *domain_indices
                .get(view.failure_domain_id.as_str())
                .expect("validated domain id"),
            cost_units: view.cost_units,
            nominal_features,
            robust_features,
        });
    }

    let required_view_counts = problem
        .features
        .iter()
        .map(|feature| feature.required_view_count)
        .collect::<Vec<_>>();
    let required_failure_domains = problem
        .features
        .iter()
        .map(|feature| feature.required_failure_domains)
        .collect::<Vec<_>>();
    let mut search = ExactSearch {
        problem,
        views,
        required_view_counts,
        required_failure_domains,
        view_ids: problem
            .views
            .iter()
            .map(|view| view.view_id.clone())
            .collect(),
        results: SearchResults::default(),
    };
    let mut state = SearchState {
        nominal_domain_refs: vec![vec![0; domains.len()]; problem.features.len()],
        robust_domain_refs: vec![vec![0; domains.len()]; problem.features.len()],
        nominal_view_counts: vec![0; problem.features.len()],
        robust_view_counts: vec![0; problem.features.len()],
        nominal_distinct: vec![0; problem.features.len()],
        robust_distinct: vec![0; problem.features.len()],
        ..SearchState::default()
    };
    search.visit(0, &mut state);

    let all_selected = vec![true; problem.views.len()];
    let candidate_pool_coverage = coverage_records(problem, &all_selected);
    let nominal_candidate_shortfall = candidate_pool_coverage.iter().any(|feature| {
        feature.nominal_view_count < feature.required_view_count
            || feature.nominal_failure_domain_count < feature.required_failure_domains
    });
    let robust_candidate_shortfall = candidate_pool_coverage.iter().any(|feature| {
        feature.robust_view_count < feature.required_view_count
            || feature.robust_failure_domain_count < feature.required_failure_domains
    });
    let candidate_geometry_disposition = if nominal_candidate_shortfall {
        Disposition::Infeasible
    } else if robust_candidate_shortfall {
        Disposition::Indeterminate
    } else {
        Disposition::Feasible
    };

    let planning_disposition = if search.results.exact_optimum.is_some() {
        PlanningDisposition::Feasible
    } else {
        match candidate_geometry_disposition {
            Disposition::Infeasible => PlanningDisposition::Infeasible,
            Disposition::Indeterminate => PlanningDisposition::Indeterminate,
            Disposition::Feasible => PlanningDisposition::BudgetInfeasible,
        }
    };

    let mut pareto_frontier = Vec::new();
    let mut best_count = usize::MAX;
    for plan in search.results.best_by_cost.values() {
        if plan.selected_view_count < best_count {
            best_count = plan.selected_view_count;
            pareto_frontier.push(plan.clone());
        }
    }

    let exact_cost = search
        .results
        .exact_optimum
        .as_ref()
        .map(|plan| plan.total_cost_units);
    let greedy_next_best_view = greedy_solution(problem, true);
    let cheapest_first = greedy_solution(problem, false);
    let greedy_cost_gap_units = match (exact_cost, greedy_next_best_view.robustly_covered) {
        (Some(exact), true) => Some(greedy_next_best_view.total_cost_units - exact),
        _ => None,
    };
    let cheapest_first_cost_gap_units = match (exact_cost, cheapest_first.robustly_covered) {
        (Some(exact), true) => Some(cheapest_first.total_cost_units - exact),
        _ => None,
    };

    let input_bytes = serde_json::to_vec(problem)
        .map_err(|error| format!("PLANNING_INPUT_SERIALIZATION_FAILED:{error}"))?;
    Ok(PlanningReceipt {
        schema: PLANNING_RECEIPT_SCHEMA.to_owned(),
        case_id: problem.case_id.clone(),
        coordinate_frame_id: problem.coordinate_frame_id.clone(),
        input_sha256: sha256_hex(&input_bytes),
        evidence_class: "synthetic".to_owned(),
        claim_ceiling: "synthetic_2d_geometric_observability_and_view_selection_only".to_owned(),
        candidate_geometry_disposition,
        planning_disposition,
        candidate_view_count: problem.views.len(),
        candidate_pool_coverage,
        max_selected_views: problem.max_selected_views,
        max_total_cost_units: problem.max_total_cost_units,
        subsets_evaluated: search.results.subsets_evaluated,
        budget_eligible_subsets: search.results.budget_eligible_subsets,
        nominally_feasible_subsets: search.results.nominally_feasible_subsets,
        robustly_feasible_subsets: search.results.robustly_feasible_subsets,
        exact_optimum: search.results.exact_optimum,
        pareto_frontier,
        greedy_next_best_view,
        cheapest_first,
        greedy_cost_gap_units,
        cheapest_first_cost_gap_units,
        authority_granted: false,
        structural_acceptance_authorized: false,
        flight_authorized: false,
        physical_validation: "NOT_ASSESSED".to_owned(),
    })
}

pub fn plan_case(
    case: &Case,
    view_cost_units: &BTreeMap<String, u64>,
    max_selected_views: usize,
    max_total_cost_units: u64,
) -> Result<PlanningReceipt, String> {
    let problem = problem_from_case(
        case,
        view_cost_units,
        max_selected_views,
        max_total_cost_units,
    )?;
    solve(&problem)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view(
        view_id: &str,
        domain: &str,
        cost: u64,
        nominal: &[&str],
        robust: &[&str],
    ) -> PlanningView {
        PlanningView {
            view_id: view_id.to_owned(),
            failure_domain_id: domain.to_owned(),
            cost_units: cost,
            nominal_feature_ids: nominal.iter().map(|value| (*value).to_owned()).collect(),
            robust_feature_ids: robust.iter().map(|value| (*value).to_owned()).collect(),
        }
    }

    fn feature(
        id: &str,
        required_view_count: usize,
        required_failure_domains: usize,
    ) -> PlanningFeature {
        PlanningFeature {
            feature_id: id.to_owned(),
            required_view_count,
            required_failure_domains,
        }
    }

    fn problem(
        features: Vec<PlanningFeature>,
        views: Vec<PlanningView>,
        max_views: usize,
        max_cost: u64,
    ) -> PlanningProblem {
        PlanningProblem {
            case_id: "SYNTHETIC-PLANNER-TEST".to_owned(),
            coordinate_frame_id: "ABSTRACT-COVERAGE-MATRIX".to_owned(),
            features,
            views,
            max_selected_views: max_views,
            max_total_cost_units: max_cost,
        }
    }

    #[test]
    fn exact_optimizer_defeats_a_greedy_trap() {
        let problem = problem(
            vec![
                feature("F1", 1, 1),
                feature("F2", 1, 1),
                feature("F3", 1, 1),
                feature("F4", 1, 1),
                feature("F5", 1, 1),
                feature("F6", 1, 1),
            ],
            vec![
                view(
                    "VIEW-A",
                    "DOMAIN-A",
                    1,
                    &["F1", "F2", "F3", "F4"],
                    &["F1", "F2", "F3", "F4"],
                ),
                view(
                    "VIEW-B",
                    "DOMAIN-B",
                    1,
                    &["F1", "F2", "F5"],
                    &["F1", "F2", "F5"],
                ),
                view(
                    "VIEW-C",
                    "DOMAIN-C",
                    1,
                    &["F3", "F4", "F6"],
                    &["F3", "F4", "F6"],
                ),
            ],
            3,
            3,
        );

        let receipt = solve(&problem).unwrap();
        let exact = receipt.exact_optimum.unwrap();
        assert_eq!(exact.selected_view_ids, vec!["VIEW-B", "VIEW-C"]);
        assert_eq!(exact.total_cost_units, 2);
        assert_eq!(receipt.greedy_next_best_view.selected_view_count, 3);
        assert_eq!(receipt.greedy_cost_gap_units, Some(1));
        assert_eq!(receipt.cheapest_first.selected_view_count, 3);
        assert_eq!(receipt.subsets_evaluated, 8);
        assert_eq!(receipt.planning_disposition, PlanningDisposition::Feasible);
        assert!(!receipt.authority_granted);
    }

    #[test]
    fn duplicate_declared_domains_do_not_satisfy_two_path_requirement() {
        let problem = problem(
            vec![feature("F1", 2, 2)],
            vec![
                view("VIEW-A", "SHARED-CALIBRATION", 1, &["F1"], &["F1"]),
                view("VIEW-B", "SHARED-CALIBRATION", 1, &["F1"], &["F1"]),
            ],
            2,
            2,
        );

        let receipt = solve(&problem).unwrap();
        assert_eq!(
            receipt.candidate_geometry_disposition,
            Disposition::Infeasible
        );
        assert_eq!(receipt.robustly_feasible_subsets, 0);
        assert_eq!(
            receipt.planning_disposition,
            PlanningDisposition::Infeasible
        );
    }

    #[test]
    fn nominal_only_coverage_is_indeterminate_not_feasible() {
        let problem = problem(
            vec![feature("F1", 2, 2)],
            vec![
                view("VIEW-A", "DOMAIN-A", 1, &["F1"], &["F1"]),
                view("VIEW-B", "DOMAIN-B", 1, &["F1"], &[]),
            ],
            2,
            2,
        );

        let receipt = solve(&problem).unwrap();
        assert_eq!(
            receipt.candidate_geometry_disposition,
            Disposition::Indeterminate
        );
        assert_eq!(
            receipt.planning_disposition,
            PlanningDisposition::Indeterminate
        );
        assert_eq!(receipt.nominally_feasible_subsets, 1);
        assert_eq!(receipt.robustly_feasible_subsets, 0);
        assert!(!receipt.greedy_next_best_view.robustly_covered);
    }

    #[test]
    fn budget_failure_is_separate_from_geometric_infeasibility() {
        let problem = problem(
            vec![feature("F1", 2, 2)],
            vec![
                view("VIEW-A", "DOMAIN-A", 1, &["F1"], &["F1"]),
                view("VIEW-B", "DOMAIN-B", 1, &["F1"], &["F1"]),
            ],
            1,
            1,
        );

        let receipt = solve(&problem).unwrap();
        assert_eq!(
            receipt.candidate_geometry_disposition,
            Disposition::Feasible
        );
        assert_eq!(
            receipt.planning_disposition,
            PlanningDisposition::BudgetInfeasible
        );
        assert_eq!(receipt.robustly_feasible_subsets, 0);
    }

    #[test]
    fn k_view_coverage_is_distinct_from_failure_domain_diversity() {
        let problem = problem(
            vec![feature("SURFACE-1", 2, 1)],
            vec![
                view(
                    "VIEW-A",
                    "SAME-SENSOR-SYSTEM",
                    1,
                    &["SURFACE-1"],
                    &["SURFACE-1"],
                ),
                view(
                    "VIEW-B",
                    "SAME-SENSOR-SYSTEM",
                    1,
                    &["SURFACE-1"],
                    &["SURFACE-1"],
                ),
            ],
            2,
            2,
        );

        let receipt = solve(&problem).unwrap();
        assert_eq!(
            receipt.candidate_geometry_disposition,
            Disposition::Feasible
        );
        let optimum = receipt.exact_optimum.unwrap();
        assert_eq!(optimum.selected_view_count, 2);
        assert_eq!(optimum.feature_coverage[0].robust_view_count, 2);
        assert_eq!(optimum.feature_coverage[0].robust_failure_domain_count, 1);
        assert!(optimum.robustly_covered);
    }

    #[test]
    fn failure_domain_diversity_is_distinct_from_k_view_requirement() {
        let problem = problem(
            vec![feature("SURFACE-1", 1, 2)],
            vec![
                view("VIEW-A", "DOMAIN-A", 1, &["SURFACE-1"], &["SURFACE-1"]),
                view("VIEW-B", "DOMAIN-B", 1, &["SURFACE-1"], &["SURFACE-1"]),
            ],
            2,
            2,
        );

        let receipt = solve(&problem).unwrap();
        let optimum = receipt.exact_optimum.unwrap();
        assert_eq!(optimum.selected_view_count, 2);
        assert_eq!(optimum.feature_coverage[0].robust_view_count, 2);
        assert_eq!(optimum.feature_coverage[0].robust_failure_domain_count, 2);
        assert_eq!(optimum.feature_coverage[0].required_view_count, 1);
        assert_eq!(optimum.feature_coverage[0].required_failure_domains, 2);
    }

    #[test]
    fn requested_k_is_not_silently_capped_to_candidate_pool_size() {
        let problem = problem(
            vec![feature("SURFACE-1", 3, 1)],
            vec![
                view("VIEW-A", "DOMAIN-A", 1, &["SURFACE-1"], &["SURFACE-1"]),
                view("VIEW-B", "DOMAIN-A", 1, &["SURFACE-1"], &["SURFACE-1"]),
            ],
            2,
            2,
        );

        let receipt = solve(&problem).unwrap();
        assert_eq!(
            receipt.candidate_geometry_disposition,
            Disposition::Infeasible
        );
        assert!(receipt.exact_optimum.is_none());
        assert_eq!(receipt.candidate_pool_coverage[0].required_view_count, 3);
        assert_eq!(receipt.candidate_pool_coverage[0].robust_view_count, 2);
        assert_eq!(
            receipt.candidate_pool_coverage[0].robust_failure_domain_count,
            1
        );
    }

    #[test]
    fn exact_search_covers_the_full_twenty_view_boundary() {
        let views = (0..MAX_EXACT_VIEWS)
            .map(|index| {
                let view_id = format!("VIEW-{index:02}");
                let domain_id = format!("DOMAIN-{index:02}");
                PlanningView {
                    view_id,
                    failure_domain_id: domain_id,
                    cost_units: 1,
                    nominal_feature_ids: vec!["F1".to_owned()],
                    robust_feature_ids: vec!["F1".to_owned()],
                }
            })
            .collect();
        let problem = problem(vec![feature("F1", 1, 1)], views, MAX_EXACT_VIEWS, 20);

        let receipt = solve(&problem).unwrap();
        let all_subsets = 1_u64 << MAX_EXACT_VIEWS;
        assert_eq!(receipt.subsets_evaluated, all_subsets);
        assert_eq!(receipt.budget_eligible_subsets, all_subsets);
        assert_eq!(receipt.nominally_feasible_subsets, all_subsets - 1);
        assert_eq!(receipt.robustly_feasible_subsets, all_subsets - 1);
        assert_eq!(
            receipt.exact_optimum.unwrap().selected_view_ids,
            vec!["VIEW-00"]
        );
    }

    #[test]
    fn exact_search_rejects_more_than_twenty_candidate_views() {
        let views = (0..=MAX_EXACT_VIEWS)
            .map(|index| {
                let view_id = format!("VIEW-{index:02}");
                let domain_id = format!("DOMAIN-{index:02}");
                PlanningView {
                    view_id,
                    failure_domain_id: domain_id,
                    cost_units: 1,
                    nominal_feature_ids: vec!["F1".to_owned()],
                    robust_feature_ids: vec!["F1".to_owned()],
                }
            })
            .collect();
        let problem = problem(vec![feature("F1", 1, 1)], views, MAX_EXACT_VIEWS + 1, 21);

        assert_eq!(
            solve(&problem),
            Err("PLANNING_PROBLEM_BOUNDS_INVALID".to_owned())
        );
    }

    #[test]
    fn input_rejects_robust_visibility_not_present_in_nominal_set() {
        let problem = problem(
            vec![feature("F1", 1, 1)],
            vec![view("VIEW-A", "DOMAIN-A", 1, &[], &["F1"])],
            1,
            1,
        );
        assert_eq!(
            solve(&problem),
            Err("PLANNING_VIEW_COVERAGE_INVALID".to_owned())
        );
    }
}
pub const PLANNING_CORPUS_SCHEMA: &str =
    "luminous.orbital-truss-inspectability.planning-corpus.v0.2";
pub const PLANNING_CORPUS_BENCHMARK_ID: &str = "ORBITAL-TRUSS-PLANNING-002";
pub const PLANNING_CORPUS_RECEIPT_SCHEMA: &str =
    "luminous.orbital-truss-inspectability.planning-corpus-receipt.v0.2";
pub const PLANNING_CASE_COUNT: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlanningCorpus {
    pub schema: String,
    pub benchmark_id: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub cases: Vec<PlanningCorpusCase>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlanningCorpusCase {
    pub case_id: String,
    pub expected_candidate_geometry_disposition: Disposition,
    pub expected_planning_disposition: PlanningDisposition,
    pub problem: PlanningProblem,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PlanningCorpusCaseReceipt {
    pub case_id: String,
    pub expected_candidate_geometry_disposition: Disposition,
    pub expected_planning_disposition: PlanningDisposition,
    pub candidate_geometry_matches: bool,
    pub planning_matches: bool,
    pub planning: PlanningReceipt,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PlanningCorpusReceipt {
    pub schema: String,
    pub benchmark_id: String,
    pub source_commit: String,
    pub input_sha256: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub case_count: usize,
    pub expected_outcome_matches: usize,
    pub expected_outcome_mismatches: usize,
    pub authority_granted: bool,
    pub physical_validation: String,
    pub cases: Vec<PlanningCorpusCaseReceipt>,
}

pub fn audit_planning_corpus(
    bytes: &[u8],
    source_commit: &str,
) -> Result<PlanningCorpusReceipt, String> {
    if source_commit.len() != 40 || !source_commit.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("SOURCE_COMMIT_MUST_BE_40_HEX_CHARACTERS".to_owned());
    }

    let corpus: PlanningCorpus = serde_json::from_slice(bytes)
        .map_err(|error| format!("PLANNING_CORPUS_PARSE_FAILED:{error}"))?;
    if corpus.schema != PLANNING_CORPUS_SCHEMA
        || corpus.benchmark_id != PLANNING_CORPUS_BENCHMARK_ID
        || corpus.evidence_class != "synthetic"
        || corpus.claim_ceiling != "synthetic_2d_geometric_observability_and_view_selection_only"
        || corpus.cases.len() != PLANNING_CASE_COUNT
    {
        return Err("PLANNING_CORPUS_IDENTITY_OR_CLAIM_CEILING_INVALID".to_owned());
    }

    let mut case_ids = BTreeSet::new();
    let mut receipts = Vec::with_capacity(corpus.cases.len());
    let mut expected_outcome_matches = 0_usize;
    for case in &corpus.cases {
        if case.case_id.trim().is_empty()
            || !case_ids.insert(case.case_id.as_str())
            || case.problem.case_id != case.case_id
        {
            return Err("PLANNING_CORPUS_CASE_ID_INVALID".to_owned());
        }
        let planning = solve(&case.problem)?;
        let candidate_geometry_matches =
            planning.candidate_geometry_disposition == case.expected_candidate_geometry_disposition;
        let planning_matches = planning.planning_disposition == case.expected_planning_disposition;
        if candidate_geometry_matches && planning_matches {
            expected_outcome_matches += 1;
        }
        receipts.push(PlanningCorpusCaseReceipt {
            case_id: case.case_id.clone(),
            expected_candidate_geometry_disposition: case.expected_candidate_geometry_disposition,
            expected_planning_disposition: case.expected_planning_disposition,
            candidate_geometry_matches,
            planning_matches,
            planning,
        });
    }

    Ok(PlanningCorpusReceipt {
        schema: PLANNING_CORPUS_RECEIPT_SCHEMA.to_owned(),
        benchmark_id: PLANNING_CORPUS_BENCHMARK_ID.to_owned(),
        source_commit: source_commit.to_ascii_lowercase(),
        input_sha256: sha256_hex(bytes),
        evidence_class: corpus.evidence_class,
        claim_ceiling: corpus.claim_ceiling,
        case_count: receipts.len(),
        expected_outcome_matches,
        expected_outcome_mismatches: receipts.len() - expected_outcome_matches,
        authority_granted: false,
        physical_validation: "NOT_ASSESSED".to_owned(),
        cases: receipts,
    })
}
