use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const MULTICOVER_CORPUS_SCHEMA: &str =
    "luminous.orbital-truss.multicover-scale-corpus.v0.1";
pub const MULTICOVER_BENCHMARK_ID: &str = "ORBITAL-MULTICOVER-SCALE-001";
pub const MULTICOVER_RECEIPT_SCHEMA: &str =
    "luminous.orbital-truss.multicover-scale-receipt.v0.1";
pub const MAX_CANDIDATE_VIEWS: usize = 100_000;
pub const MAX_FEATURES: usize = 250_000;
pub const MAX_VISIBILITY_EDGES: usize = 50_000_000;
pub const MAX_REQUESTED_K: usize = 10_000;
const FIXED_SCALE: u64 = 1_000_000_000_000_000_000;
const EXP_NEGATIVE_ONE_FIXED: u64 = 367_879_441_171_442_321;
const MAX_NORMALIZED_EXPONENT: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MultiCoverFeature {
    pub feature_id: String,
    pub requested_k: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MultiCoverView {
    pub view_id: String,
    pub failure_domain_id: String,
    pub nominal_feature_ids: Vec<String>,
    pub robust_feature_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MultiCoverProblem {
    pub case_id: String,
    pub coordinate_frame_id: String,
    pub features: Vec<MultiCoverFeature>,
    pub views: Vec<MultiCoverView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MultiCoverCorpus {
    pub schema: String,
    pub benchmark_id: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub problem: MultiCoverProblem,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MultiCoverFeatureCoverage {
    pub feature_id: String,
    pub requested_k: usize,
    pub available_nominal_candidate_views: usize,
    pub available_robust_candidate_views: usize,
    pub attainable_capped_target: usize,
    pub selected_nominal_views: usize,
    pub selected_robust_views: usize,
    pub requested_k_met: bool,
    pub attainable_target_met: bool,
    pub robust_requested_k_met: bool,
    pub nominal_failure_domain_count_available: usize,
    pub robust_failure_domain_count_available: usize,
    pub nominal_failure_domain_count_selected: usize,
    pub robust_failure_domain_count_selected: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DualBoundCertificate {
    pub method: String,
    pub ordering: String,
    pub objective_numerator: u128,
    pub objective_denominator: u64,
    pub lower_bound_floor: u64,
    pub lower_bound_ceiling: u64,
    pub maximum_candidate_constraint_load_numerator: u64,
    pub candidate_constraint_count: usize,
    pub dual_feature_weight_numerators: Vec<u64>,
    pub dual_constraints_verified: bool,
    pub minimum_cardinality_optimum_certified: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MultiCoverReceipt {
    pub schema: String,
    pub benchmark_id: String,
    pub case_id: String,
    pub coordinate_frame_id: String,
    pub source_commit: String,
    pub input_sha256: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub candidate_view_count: usize,
    pub feature_count: usize,
    pub nominal_visibility_edge_count: usize,
    pub robust_visibility_edge_count: usize,
    pub selected_view_ids: Vec<String>,
    pub selection_order_view_ids: Vec<String>,
    pub cleanup_removed_view_ids: Vec<String>,
    pub selected_view_count: usize,
    pub candidate_pool_requested_k_fully_available: bool,
    pub selected_requested_k_fully_covered: bool,
    pub selected_attainable_targets_fully_covered: bool,
    pub selected_robust_requested_k_fully_covered: bool,
    pub feature_coverage: Vec<MultiCoverFeatureCoverage>,
    pub dual_bound: DualBoundCertificate,
    pub selected_minus_dual_lower_bound: u64,
    pub relative_excess_over_dual_lower_bound_basis_points: Option<u64>,
    pub authority_granted: bool,
    pub physical_validation: String,
}

#[derive(Debug, Clone)]
struct IndexedProblem {
    feature_ids: Vec<String>,
    view_ids: Vec<String>,
    requested_k: Vec<usize>,
    failure_domain_ids: Vec<String>,
    nominal_by_view: Vec<Vec<usize>>,
    robust_by_view: Vec<Vec<usize>>,
    feature_to_views: Vec<Vec<usize>>,
    nominal_candidate_counts: Vec<usize>,
    robust_candidate_counts: Vec<usize>,
    nominal_available_domains: Vec<BTreeSet<String>>,
    robust_available_domains: Vec<BTreeSet<String>>,
}

fn validate_problem(problem: &MultiCoverProblem) -> Result<IndexedProblem, String> {
    if problem.case_id.trim().is_empty()
        || problem.coordinate_frame_id.trim().is_empty()
        || problem.features.is_empty()
        || problem.features.len() > MAX_FEATURES
        || problem.views.len() > MAX_CANDIDATE_VIEWS
    {
        return Err("MULTICOVER_PROBLEM_BOUNDS_INVALID".to_owned());
    }

    let mut feature_indices = BTreeMap::new();
    let mut feature_ids = Vec::with_capacity(problem.features.len());
    let mut requested_k = Vec::with_capacity(problem.features.len());
    for (index, feature) in problem.features.iter().enumerate() {
        if feature.feature_id.trim().is_empty()
            || feature.requested_k == 0
            || feature.requested_k > MAX_REQUESTED_K
            || feature_indices.insert(feature.feature_id.as_str(), index).is_some()
        {
            return Err("MULTICOVER_FEATURE_INVALID".to_owned());
        }
        feature_ids.push(feature.feature_id.clone());
        requested_k.push(feature.requested_k);
    }

    let mut view_ids = BTreeSet::new();
    let mut nominal_by_view = Vec::with_capacity(problem.views.len());
    let mut robust_by_view = Vec::with_capacity(problem.views.len());
    let mut feature_to_views = vec![Vec::new(); problem.features.len()];
    let mut nominal_candidate_counts = vec![0_usize; problem.features.len()];
    let mut robust_candidate_counts = vec![0_usize; problem.features.len()];
    let mut nominal_available_domains = vec![BTreeSet::new(); problem.features.len()];
    let mut robust_available_domains = vec![BTreeSet::new(); problem.features.len()];
    let mut nominal_edge_count = 0_usize;
    let mut robust_edge_count = 0_usize;
    let mut failure_domain_ids = Vec::with_capacity(problem.views.len());

    for (view_index, view) in problem.views.iter().enumerate() {
        if view.view_id.trim().is_empty()
            || !view_ids.insert(view.view_id.as_str())
            || view.failure_domain_id.trim().is_empty()
        {
            return Err("MULTICOVER_VIEW_IDENTITY_INVALID".to_owned());
        }
        failure_domain_ids.push(view.failure_domain_id.clone());

        let nominal_names: BTreeSet<&str> = view.nominal_feature_ids.iter().map(String::as_str).collect();
        let robust_names: BTreeSet<&str> = view.robust_feature_ids.iter().map(String::as_str).collect();
        if nominal_names.len() != view.nominal_feature_ids.len()
            || robust_names.len() != view.robust_feature_ids.len()
            || !robust_names.is_subset(&nominal_names)
        {
            return Err("MULTICOVER_VIEW_COVERAGE_INVALID".to_owned());
        }

        let mut nominal = Vec::with_capacity(view.nominal_feature_ids.len());
        for feature_id in &view.nominal_feature_ids {
            let index = *feature_indices
                .get(feature_id.as_str())
                .ok_or_else(|| "MULTICOVER_UNKNOWN_FEATURE_ID".to_owned())?;
            nominal.push(index);
            nominal_candidate_counts[index] += 1;
            nominal_available_domains[index].insert(view.failure_domain_id.clone());
            feature_to_views[index].push(view_index);
        }
        let mut robust = Vec::with_capacity(view.robust_feature_ids.len());
        for feature_id in &view.robust_feature_ids {
            let index = *feature_indices
                .get(feature_id.as_str())
                .ok_or_else(|| "MULTICOVER_UNKNOWN_FEATURE_ID".to_owned())?;
            robust.push(index);
            robust_candidate_counts[index] += 1;
            robust_available_domains[index].insert(view.failure_domain_id.clone());
        }
        nominal.sort_unstable();
        robust.sort_unstable();
        nominal_edge_count = nominal_edge_count
            .checked_add(nominal.len())
            .ok_or_else(|| "MULTICOVER_EDGE_COUNT_OVERFLOW".to_owned())?;
        robust_edge_count = robust_edge_count
            .checked_add(robust.len())
            .ok_or_else(|| "MULTICOVER_EDGE_COUNT_OVERFLOW".to_owned())?;
        if nominal_edge_count > MAX_VISIBILITY_EDGES || robust_edge_count > MAX_VISIBILITY_EDGES {
            return Err("MULTICOVER_VISIBILITY_EDGE_LIMIT_EXCEEDED".to_owned());
        }
        nominal_by_view.push(nominal);
        robust_by_view.push(robust);
    }

    Ok(IndexedProblem {
        feature_ids,
        requested_k,
        feature_ids,
        view_ids: problem.views.iter().map(|view| view.view_id.clone()).collect(),
        requested_k,
        failure_domain_ids,
        nominal_by_view,
        robust_by_view,
        feature_to_views,
        nominal_candidate_counts,
        robust_candidate_counts,
        nominal_available_domains,
        robust_available_domains,
    })
}

fn exponential_decay_table() -> Vec<u64> {
    let scale = u128::from(FIXED_SCALE);
    let decay = u128::from(EXP_NEGATIVE_ONE_FIXED);
    let mut values = Vec::with_capacity(MAX_NORMALIZED_EXPONENT + 1);
    values.push(FIXED_SCALE);
    for _ in 0..MAX_NORMALIZED_EXPONENT {
        let next = (u128::from(*values.last().expect("decay table is nonempty")) * decay) / scale;
        values.push(next as u64);
    }
    values
}

fn weight_vector(
    indexed: &IndexedProblem,
    target: &[usize],
    covered: &[usize],
    remaining: &[usize],
    exp_table: &[u64],
) -> Vec<u64> {
    let mut minimum_sigma = usize::MAX;
    let mut sigmas = vec![0_usize; indexed.feature_ids.len()];
    let mut deficits = vec![0_usize; indexed.feature_ids.len()];
    for index in 0..indexed.feature_ids.len() {
        let deficit = target[index].saturating_sub(covered[index]);
        deficits[index] = deficit;
        if deficit > 0 {
            let sigma = remaining[index].saturating_sub(deficit);
            sigmas[index] = sigma;
            minimum_sigma = minimum_sigma.min(sigma);
        }
    }
    if minimum_sigma == usize::MAX {
        return vec![0; indexed.feature_ids.len()];
    }

    let mut weights = vec![0_u64; indexed.feature_ids.len()];
    for index in 0..indexed.feature_ids.len() {
        let deficit = deficits[index];
        if deficit == 0 {
            continue;
        }
        let delta_sigma = sigmas[index].saturating_sub(minimum_sigma);
        let exp_weight = exp_table.get(delta_sigma).copied().unwrap_or(0);
        let deficit_squared = (deficit as u128) * (deficit as u128);
        let k_squared = (indexed.requested_k[index] as u128)
            * (indexed.requested_k[index] as u128);
        let weight = (u128::from(exp_weight) * deficit_squared) / k_squared;
        weights[index] = weight.min(u128::from(u64::MAX)) as u64;
    }
    weights
}

fn select_greedily(indexed: &IndexedProblem) -> Result<(Vec<bool>, Vec<usize>, Vec<usize>), String> {
    let target = indexed
        .requested_k
        .iter()
        .zip(&indexed.nominal_candidate_counts)
        .map(|(requested, available)| (*requested).min(*available))
        .collect::<Vec<_>>();
    let mut remaining = indexed.nominal_candidate_counts.clone();
    let mut covered = vec![0_usize; indexed.feature_ids.len()];
    let mut selected = vec![false; indexed.nominal_by_view.len()];
    let mut selection_order = Vec::new();
    let exp_table = exponential_decay_table();

    loop {
        if covered.iter().zip(&target).all(|(actual, required)| actual >= required) {
            break;
        }
        let weights = weight_vector(indexed, &target, &covered, &remaining, &exp_table);
        let mut best: Option<(usize, u128)> = None;
        for (view_index, feature_indices) in indexed.nominal_by_view.iter().enumerate() {
            if selected[view_index] {
                continue;
            }
            let mut score = 0_u128;
            for feature_index in feature_indices {
                score = score
                    .checked_add(u128::from(weights[*feature_index]))
                    .ok_or_else(|| "MULTICOVER_VIEW_SCORE_OVERFLOW".to_owned())?;
            }
            let replace = match best {
                None => score > 0,
                Some((best_index, best_score)) => {
                    score > best_score
                        || (score == best_score
                            && score > 0
                            && indexed.view_ids[view_index]
                                < indexed.view_ids[best_index])
                }
            };
            if replace {
                best = Some((view_index, score));
            }
        }

        let Some((best_index, best_score)) = best else {
            return Err("MULTICOVER_GREEDY_STALLED_WITH_UNMET_CAPPED_TARGET".to_owned());
        };
        if best_score == 0 {
            return Err("MULTICOVER_GREEDY_ZERO_SCORE_WITH_UNMET_CAPPED_TARGET".to_owned());
        }
        selected[best_index] = true;
        selection_order.push(best_index);
        for feature_index in &indexed.nominal_by_view[best_index] {
            covered[*feature_index] += 1;
            remaining[*feature_index] = remaining[*feature_index]
                .checked_sub(1)
                .ok_or_else(|| "MULTICOVER_REMAINING_COUNT_UNDERFLOW".to_owned())?;
        }
    }

    let mut cleanup_removed = Vec::new();
    let mut changed = true;
    while changed {
        changed = false;
        for view_index in &selection_order {
            if !selected[*view_index] {
                continue;
            }
            let removable = indexed.nominal_by_view[*view_index]
                .iter()
                .all(|feature_index| {
                    covered[*feature_index].saturating_sub(1) >= target[*feature_index]
                });
            if removable {
                selected[*view_index] = false;
                cleanup_removed.push(*view_index);
                for feature_index in &indexed.nominal_by_view[*view_index] {
                    covered[*feature_index] -= 1;
                }
                changed = true;
                break;
            }
        }
    }

    Ok((selected, selection_order, cleanup_removed))
}

#[derive(Debug, Clone)]
struct DualCandidate {
    ordering: &'static str,
    objective_numerator: u128,
    weights: Vec<u64>,
    max_constraint_load: u64,
}

fn build_dual_for_order(
    indexed: &IndexedProblem,
    target: &[usize],
    ordering: &'static str,
) -> Result<DualCandidate, String> {
    let mut order = (0..indexed.feature_ids.len())
        .filter(|index| target[*index] > 0 && !indexed.feature_to_views[*index].is_empty())
        .collect::<Vec<_>>();
    order.sort_by(|left, right| {
        let left_degree = indexed.feature_to_views[*left].len();
        let right_degree = indexed.feature_to_views[*right].len();
        let comparison = match ordering {
            "DEMAND_PER_CANDIDATE_DESC" => {
                let left_product = (target[*left] as u128) * (right_degree as u128);
                let right_product = (target[*right] as u128) * (left_degree as u128);
                right_product.cmp(&left_product)
            }
            "CANDIDATE_SCARCITY_ASC" => left_degree.cmp(&right_degree),
            "DEMAND_DESC" => target[*right].cmp(&target[*left]),
            _ => std::cmp::Ordering::Equal,
        };
        comparison.then_with(|| indexed.feature_ids[*left].cmp(&indexed.feature_ids[*right]))
    });

    let mut weights = vec![0_u64; indexed.feature_ids.len()];
    let mut candidate_loads = vec![0_u64; indexed.nominal_by_view.len()];
    let mut objective_numerator = 0_u128;
    for feature_index in order {
        let mut slack = FIXED_SCALE;
        for view_index in &indexed.feature_to_views[feature_index] {
            slack = slack.min(FIXED_SCALE.saturating_sub(candidate_loads[*view_index]));
        }
        weights[feature_index] = slack;
        let objective_term = (target[feature_index] as u128)
            .checked_mul(slack as u128)
            .ok_or_else(|| "MULTICOVER_DUAL_OBJECTIVE_OVERFLOW".to_owned())?;
        objective_numerator = objective_numerator
            .checked_add(objective_term)
            .ok_or_else(|| "MULTICOVER_DUAL_OBJECTIVE_OVERFLOW".to_owned())?;
        for view_index in &indexed.feature_to_views[feature_index] {
            candidate_loads[*view_index] = candidate_loads[*view_index]
                .checked_add(slack)
                .ok_or_else(|| "MULTICOVER_DUAL_LOAD_OVERFLOW".to_owned())?;
        }
    }

    let max_constraint_load = candidate_loads.iter().copied().max().unwrap_or(0);
    Ok(DualCandidate {
        ordering,
        objective_numerator,
        weights,
        max_constraint_load,
    })
}

fn dual_certificate(indexed: &IndexedProblem) -> Result<DualBoundCertificate, String> {
    let target = indexed
        .requested_k
        .iter()
        .zip(&indexed.nominal_candidate_counts)
        .map(|(requested, available)| (*requested).min(*available))
        .collect::<Vec<_>>();
    let mut candidates = Vec::new();
    for ordering in [
        "DEMAND_PER_CANDIDATE_DESC",
        "CANDIDATE_SCARCITY_ASC",
        "DEMAND_DESC",
    ] {
        candidates.push(build_dual_for_order(indexed, &target, ordering)?);
    }
    let best = candidates
        .into_iter()
        .max_by(|left, right| {
            left.objective_numerator
                .cmp(&right.objective_numerator)
                .then_with(|| right.ordering.cmp(left.ordering))
        })
        .ok_or_else(|| "MULTICOVER_DUAL_CANDIDATE_SET_EMPTY".to_owned())?;
    if best.max_constraint_load > FIXED_SCALE {
        return Err("MULTICOVER_DUAL_CERTIFICATE_INFEASIBLE".to_owned());
    }
    let denominator = u128::from(FIXED_SCALE);
    let lower_bound_floor = (best.objective_numerator / denominator) as u64;
    let lower_bound_ceiling = lower_bound_floor
        + u64::from(best.objective_numerator % denominator != 0);
    Ok(DualBoundCertificate {
        method: "FEASIBLE_SET_MULTICOVER_LP_DUAL_V0_1".to_owned(),
        ordering: best.ordering.to_owned(),
        objective_numerator: best.objective_numerator,
        objective_denominator: FIXED_SCALE,
        lower_bound_floor,
        lower_bound_ceiling,
        maximum_candidate_constraint_load_numerator: best.max_constraint_load,
        candidate_constraint_count: indexed.nominal_by_view.len(),
        dual_feature_weight_numerators: best.weights,
        dual_constraints_verified: true,
        minimum_cardinality_optimum_certified: false,
    })
}

fn build_feature_coverage(
    indexed: &IndexedProblem,
    selected: &[bool],
) -> Vec<MultiCoverFeatureCoverage> {
    let feature_count = indexed.feature_ids.len();
    let mut selected_nominal = vec![0_usize; feature_count];
    let mut selected_robust = vec![0_usize; feature_count];
    let mut selected_nominal_domains = vec![BTreeSet::<String>::new(); feature_count];
    let mut selected_robust_domains = vec![BTreeSet::<String>::new(); feature_count];
    for (view_index, include) in selected.iter().enumerate() {
        if !include {
            continue;
        }
        for feature_index in &indexed.nominal_by_view[view_index] {
            selected_nominal[*feature_index] += 1;
            selected_nominal_domains[*feature_index]
                .insert(indexed.failure_domain_ids[view_index].clone());
        }
        for feature_index in &indexed.robust_by_view[view_index] {
            selected_robust[*feature_index] += 1;
            selected_robust_domains[*feature_index]
                .insert(indexed.failure_domain_ids[view_index].clone());
        }
    }
    (0..feature_count)
        .map(|index| {
            let requested_k = indexed.requested_k[index];
            let available_nominal = indexed.nominal_candidate_counts[index];
            let attainable = requested_k.min(available_nominal);
            MultiCoverFeatureCoverage {
                feature_id: indexed.feature_ids[index].clone(),
                requested_k,
                available_nominal_candidate_views: available_nominal,
                available_robust_candidate_views: indexed.robust_candidate_counts[index],
                attainable_capped_target: attainable,
                selected_nominal_views: selected_nominal[index],
                selected_robust_views: selected_robust[index],
                requested_k_met: selected_nominal[index] >= requested_k,
                attainable_target_met: selected_nominal[index] >= attainable,
                robust_requested_k_met: selected_robust[index] >= requested_k,
                nominal_failure_domain_count_available: indexed.nominal_available_domains[index].len(),
                robust_failure_domain_count_available: indexed.robust_available_domains[index].len(),
                nominal_failure_domain_count_selected: selected_nominal_domains[index].len(),
                robust_failure_domain_count_selected: selected_robust_domains[index].len(),
            }
        })
        .collect()
}

pub fn solve_multicover(problem: &MultiCoverProblem) -> Result<MultiCoverReceipt, String> {
    let indexed = validate_problem(problem)?;
    let (selected, selection_order, cleanup_removed) = select_greedily(&indexed)?;
    let dual_bound = dual_certificate(&indexed)?;
    let feature_coverage = build_feature_coverage(&indexed, &selected);

    let mut selected_view_ids = problem
        .views
        .iter()
        .zip(&selected)
        .filter(|(_, include)| **include)
        .map(|(view, _)| view.view_id.clone())
        .collect::<Vec<_>>();
    selected_view_ids.sort();
    let selection_order_view_ids = selection_order
        .iter()
        .map(|index| problem.views[*index].view_id.clone())
        .collect::<Vec<_>>();
    let cleanup_removed_view_ids = cleanup_removed
        .iter()
        .map(|index| problem.views[*index].view_id.clone())
        .collect::<Vec<_>>();
    let selected_count = selected_view_ids.len();
    if dual_bound.lower_bound_ceiling > selected_count as u64 {
        return Err("MULTICOVER_DUAL_LOWER_BOUND_EXCEEDS_FEASIBLE_INCUMBENT".to_owned());
    }

    let candidate_pool_requested_k_fully_available = feature_coverage
        .iter()
        .all(|feature| feature.available_nominal_candidate_views >= feature.requested_k);
    let selected_requested_k_fully_covered =
        feature_coverage.iter().all(|feature| feature.requested_k_met);
    let selected_attainable_targets_fully_covered =
        feature_coverage.iter().all(|feature| feature.attainable_target_met);
    let selected_robust_requested_k_fully_covered =
        feature_coverage.iter().all(|feature| feature.robust_requested_k_met);
    let excess = selected_count as u64 - dual_bound.lower_bound_ceiling;
    let relative_excess_over_dual_lower_bound_basis_points =
        (dual_bound.lower_bound_ceiling > 0).then(|| {
            let numerator = (excess as u128) * 10_000_u128;
            let denominator = dual_bound.lower_bound_ceiling as u128;
            ((numerator + denominator - 1) / denominator) as u64
        });

    let nominal_visibility_edge_count = indexed.nominal_by_view.iter().map(Vec::len).sum();
    let robust_visibility_edge_count = indexed.robust_by_view.iter().map(Vec::len).sum();
    Ok(MultiCoverReceipt {
        schema: MULTICOVER_RECEIPT_SCHEMA.to_owned(),
        benchmark_id: MULTICOVER_BENCHMARK_ID.to_owned(),
        case_id: problem.case_id.clone(),
        coordinate_frame_id: problem.coordinate_frame_id.clone(),
        source_commit: String::new(),
        input_sha256: String::new(),
        evidence_class: "synthetic".to_owned(),
        claim_ceiling: "synthetic_sparse_nominal_k_multicover_with_robust_diagnostics_only".to_owned(),
        candidate_view_count: problem.views.len(),
        feature_count: problem.features.len(),
        nominal_visibility_edge_count,
        robust_visibility_edge_count,
        selected_view_ids,
        selection_order_view_ids,
        cleanup_removed_view_ids,
        selected_view_count: selected_count,
        candidate_pool_requested_k_fully_available,
        selected_requested_k_fully_covered,
        selected_attainable_targets_fully_covered,
        selected_robust_requested_k_fully_covered,
        feature_coverage,
        dual_bound,
        selected_minus_dual_lower_bound: excess,
        relative_excess_over_dual_lower_bound_basis_points,
        authority_granted: false,
        physical_validation: "NOT_ASSESSED".to_owned(),
    })
}

pub fn audit_multicover(bytes: &[u8], source_commit: &str) -> Result<MultiCoverReceipt, String> {
    if source_commit.len() != 40 || !source_commit.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("SOURCE_COMMIT_MUST_BE_40_HEX_CHARACTERS".to_owned());
    }
    let corpus: MultiCoverCorpus =
        serde_json::from_slice(bytes).map_err(|error| format!("MULTICOVER_CORPUS_PARSE_FAILED:{error}"))?;
    if corpus.schema != MULTICOVER_CORPUS_SCHEMA
        || corpus.benchmark_id != MULTICOVER_BENCHMARK_ID
        || corpus.evidence_class != "synthetic"
        || corpus.claim_ceiling != "synthetic_sparse_nominal_k_multicover_with_robust_diagnostics_only"
    {
        return Err("MULTICOVER_CORPUS_IDENTITY_OR_CLAIM_CEILING_INVALID".to_owned());
    }
    let mut receipt = solve_multicover(&corpus.problem)?;
    receipt.source_commit = source_commit.to_ascii_lowercase();
    receipt.input_sha256 = Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    Ok(receipt)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../../../docs/research/orbital-truss/fixtures/multicover-scale-v0_1.json");

    fn toy_problem() -> MultiCoverProblem {
        MultiCoverProblem {
            case_id: "TOY-PRIMAL-DUAL".to_owned(),
            coordinate_frame_id: "ABSTRACT-SPARSE-MATRIX".to_owned(),
            features: vec![
                MultiCoverFeature { feature_id: "F1".to_owned(), requested_k: 2 },
                MultiCoverFeature { feature_id: "F2".to_owned(), requested_k: 1 },
                MultiCoverFeature { feature_id: "F3".to_owned(), requested_k: 1 },
            ],
            views: vec![
                MultiCoverView { view_id: "V1".to_owned(), failure_domain_id: "D1".to_owned(), nominal_feature_ids: vec!["F1".to_owned(), "F2".to_owned()], robust_feature_ids: vec!["F1".to_owned(), "F2".to_owned()] },
                MultiCoverView { view_id: "V2".to_owned(), failure_domain_id: "D2".to_owned(), nominal_feature_ids: vec!["F1".to_owned(), "F3".to_owned()], robust_feature_ids: vec!["F1".to_owned(), "F3".to_owned()] },
                MultiCoverView { view_id: "V3".to_owned(), failure_domain_id: "D3".to_owned(), nominal_feature_ids: vec!["F2".to_owned(), "F3".to_owned()], robust_feature_ids: vec!["F2".to_owned(), "F3".to_owned()] },
                MultiCoverView { view_id: "V4".to_owned(), failure_domain_id: "D4".to_owned(), nominal_feature_ids: vec!["F1".to_owned()], robust_feature_ids: vec!["F1".to_owned()] },
            ],
        }
    }

    #[test]
    fn large_pool_solver_exceeds_exact_oracle_limit() {
        let receipt = audit_multicover(FIXTURE.as_bytes(), &"a".repeat(40)).unwrap();
        assert!(receipt.candidate_view_count > 20);
        assert!(receipt.feature_count >= 20);
        assert!(receipt.selected_attainable_targets_fully_covered);
        assert!(receipt.dual_bound.dual_constraints_verified);
        assert!(receipt.dual_bound.maximum_candidate_constraint_load_numerator <= FIXED_SCALE);
        assert!(receipt.dual_bound.lower_bound_ceiling <= receipt.selected_view_count as u64);
        assert!(!receipt.dual_bound.minimum_cardinality_optimum_certified);
        assert!(!receipt.authority_granted);
    }

    #[test]
    fn dual_bound_is_below_exact_small_instance_optimum() {
        let problem = toy_problem();
        let receipt = solve_multicover(&problem).unwrap();
        let mut exact = usize::MAX;
        for mask in 0_u64..(1_u64 << problem.views.len()) {
            let mut counts = vec![0_usize; problem.features.len()];
            for (index, view) in problem.views.iter().enumerate() {
                if mask & (1_u64 << index) == 0 {
                    continue;
                }
                for feature in &view.nominal_feature_ids {
                    let feature_index = problem.features.iter().position(|f| f.feature_id == *feature).unwrap();
                    counts[feature_index] += 1;
                }
            }
            if counts.iter().zip(&problem.features).all(|(count, feature)| {
                *count >= feature.requested_k.min(
                    problem.views.iter().filter(|view| view.nominal_feature_ids.contains(&feature.feature_id)).count()
                )
            }) {
                exact = exact.min(mask.count_ones() as usize);
            }
        }
        assert_ne!(exact, usize::MAX);
        assert!(receipt.dual_bound.lower_bound_ceiling <= exact as u64);
        assert!(exact <= receipt.selected_view_count);
    }

    #[test]
    fn capped_target_and_robust_diagnostics_remain_separate() {
        let problem = MultiCoverProblem {
            case_id: "K-SHORTFALL-ROBUSTNESS".to_owned(),
            coordinate_frame_id: "ABSTRACT-SPARSE-MATRIX".to_owned(),
            features: vec![MultiCoverFeature { feature_id: "SURFACE".to_owned(), requested_k: 3 }],
            views: vec![
                MultiCoverView { view_id: "VIEW-A".to_owned(), failure_domain_id: "D1".to_owned(), nominal_feature_ids: vec!["SURFACE".to_owned()], robust_feature_ids: vec!["SURFACE".to_owned()] },
                MultiCoverView { view_id: "VIEW-B".to_owned(), failure_domain_id: "D1".to_owned(), nominal_feature_ids: vec!["SURFACE".to_owned()], robust_feature_ids: vec![] },
            ],
        };
        let receipt = solve_multicover(&problem).unwrap();
        let feature = &receipt.feature_coverage[0];
        assert_eq!(feature.available_nominal_candidate_views, 2);
        assert_eq!(feature.available_robust_candidate_views, 1);
        assert_eq!(feature.attainable_capped_target, 2);
        assert!(feature.attainable_target_met);
        assert!(!feature.requested_k_met);
        assert!(!feature.robust_requested_k_met);
        assert!(!receipt.candidate_pool_requested_k_fully_available);
    }

    #[test]
    fn candidate_order_permutation_preserves_selected_view_identity() {
        let problem = toy_problem();
        let expected = solve_multicover(&problem).unwrap().selected_view_ids;
        let mut reversed = problem;
        reversed.views.reverse();
        let actual = solve_multicover(&reversed).unwrap().selected_view_ids;
        assert_eq!(actual, expected);
    }
}
