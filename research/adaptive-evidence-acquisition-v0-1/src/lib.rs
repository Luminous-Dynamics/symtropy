//! Deterministic, evidence-deficit-driven acquisition planning.
//!
//! This planner selects among preflighted candidate viewpoints/actions. It
//! trusts candidate coverage only when an upstream geometry/sensor evaluator
//! marks that candidate Qualified. It is not a motion planner or safety gate.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

const TAU: f64 = 2.0 * std::f64::consts::PI;
const MAX_ABSOLUTE_COORDINATE_M: f64 = 1.0e9;

#[derive(Clone, Debug, PartialEq)]
pub struct EvidenceTarget {
    pub target_id: String,
    pub required_independent_passes: usize,
    pub minimum_view_angle_separation_rad: f64,
    /// Priority weight supplied by the upstream decision/evidence request owner.
    /// This crate consumes the weight; it does not estimate decision value.
    pub weight: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct QualifiedExistingPass {
    pub target_id: String,
    pub pass_id: String,
    pub view_angle_rad: f64,
}

/// This is an upstream qualification result, not a planner-generated opinion.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CandidateQuality {
    Qualified,
    NotQualified,
    Unknown,
    InvalidEvidence,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AcquisitionCandidate {
    pub candidate_id: String,
    /// Identifies the independent acquisition pass/trackline/approach.
    pub pass_id: String,
    pub u_m: f64,
    pub v_m: f64,
    pub view_angle_rad: f64,
    /// Targets whose complete required geometry is covered by this candidate.
    /// This list must be produced by an upstream geometric evaluator.
    pub covered_target_ids: Vec<String>,
    pub quality: CandidateQuality,
    pub acquisition_time_s: f64,
    pub energy_wh: f64,
    /// Explicit risk cost expressed in time-equivalent seconds.
    pub risk_penalty_s: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AcquisitionBudget {
    pub start_u_m: f64,
    pub start_v_m: f64,
    pub travel_speed_m_per_s: f64,
    pub maximum_elapsed_time_s: f64,
    pub maximum_energy_wh: f64,
    /// Converts energy cost into time-equivalent utility cost, not elapsed time.
    pub energy_penalty_s_per_wh: f64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlanDisposition {
    Complete,
    Partial,
    Blocked,
}

/// Selection objectives used for matched-policy comparisons. Every policy
/// shares the same candidate qualification, evidence independence and budget
/// constraints; only the ordering objective changes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SelectionPolicy {
    /// Weighted marginal evidence gain per time-equivalent cost.
    EvidenceGainPerCost,
    /// Greedy shortest travel distance among feasible candidates.
    NearestFeasible,
    /// Highest weighted marginal evidence gain, without cost normalization.
    MaximumMarginalEvidence,
    /// Stable input order (use the fixed raster/waypoint order as input).
    InputOrder,
    /// Reproducible pseudo-random order keyed by seed and candidate ID.
    SeededHashOrder { seed: u64 },
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlannedStep {
    pub ordinal: usize,
    pub candidate_id: String,
    pub pass_id: String,
    pub marginal_target_ids: Vec<String>,
    pub travel_distance_m: f64,
    pub travel_time_s: f64,
    pub acquisition_time_s: f64,
    pub risk_penalty_s: f64,
    pub energy_wh: f64,
    pub utility_score_per_s: f64,
    pub cumulative_elapsed_time_s: f64,
    pub cumulative_energy_wh: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TargetProgress {
    pub target_id: String,
    pub qualifying_passes: usize,
    pub required_independent_passes: usize,
    pub remaining_passes: usize,
    pub complete: bool,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CandidateBlockerCounts {
    pub not_qualified: usize,
    pub unknown: usize,
    pub invalid_evidence: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AcquisitionPlan {
    pub disposition: PlanDisposition,
    pub steps: Vec<PlannedStep>,
    pub targets: Vec<TargetProgress>,
    pub completed_targets: usize,
    pub total_targets: usize,
    pub cumulative_elapsed_time_s: f64,
    pub cumulative_energy_wh: f64,
    pub blockers: CandidateBlockerCounts,
}

/// Choose the next candidate by weighted marginal evidence gain per
/// time-equivalent cost. Candidate qualification and target coverage are
/// supplied by an independent upstream evaluator.
pub fn plan_acquisition(
    targets: &[EvidenceTarget],
    existing_passes: &[QualifiedExistingPass],
    candidates: &[AcquisitionCandidate],
    budget: &AcquisitionBudget,
) -> Result<AcquisitionPlan, String> {
    plan_acquisition_with_policy(
        targets,
        existing_passes,
        candidates,
        budget,
        SelectionPolicy::EvidenceGainPerCost,
    )
}

/// Execute the same fail-closed greedy planner with an alternate selection
/// policy. This enables matched-candidate, matched-budget benchmark comparisons
/// without relaxing evidence qualification or budget constraints.
pub fn plan_acquisition_with_policy(
    targets: &[EvidenceTarget],
    existing_passes: &[QualifiedExistingPass],
    candidates: &[AcquisitionCandidate],
    budget: &AcquisitionBudget,
    policy: SelectionPolicy,
) -> Result<AcquisitionPlan, String> {
    validate_budget(budget)?;

    if targets.is_empty() {
        return Err("acquisition plan requires at least one evidence target".to_owned());
    }

    let mut target_map = BTreeMap::new();
    for target in targets {
        if target.target_id.trim().is_empty()
            || target.required_independent_passes == 0
            || !target.minimum_view_angle_separation_rad.is_finite()
            || !(0.0..=std::f64::consts::PI).contains(&target.minimum_view_angle_separation_rad)
            || !target.weight.is_finite()
            || target.weight <= 0.0
            || target.weight > 1.0e12
        {
            return Err(format!("invalid evidence target: {}", target.target_id));
        }
        if target_map
            .insert(target.target_id.clone(), target.clone())
            .is_some()
        {
            return Err(format!(
                "duplicate evidence target ID: {}",
                target.target_id
            ));
        }
    }

    let mut evidence: BTreeMap<String, Vec<QualifiedExistingPass>> = target_map
        .keys()
        .map(|target_id| (target_id.clone(), Vec::new()))
        .collect();
    let mut seen_existing = BTreeSet::new();
    for pass in existing_passes {
        if pass.target_id.trim().is_empty()
            || pass.pass_id.trim().is_empty()
            || !pass.view_angle_rad.is_finite()
            || !(-std::f64::consts::PI..=std::f64::consts::PI).contains(&pass.view_angle_rad)
        {
            return Err("existing qualified pass contains invalid identity or angle".to_owned());
        }
        if !target_map.contains_key(&pass.target_id) {
            return Err(format!(
                "existing pass references unknown target: {}",
                pass.target_id
            ));
        }
        if !seen_existing.insert((pass.target_id.clone(), pass.pass_id.clone())) {
            return Err(format!(
                "duplicate existing pass identity for target {}: {}",
                pass.target_id, pass.pass_id
            ));
        }
        evidence
            .get_mut(&pass.target_id)
            .expect("target existence was checked")
            .push(pass.clone());
    }

    let mut seen_candidates = BTreeSet::new();
    let mut blockers = CandidateBlockerCounts::default();
    for candidate in candidates {
        validate_candidate(candidate, &target_map)?;
        if !seen_candidates.insert(candidate.candidate_id.clone()) {
            return Err(format!(
                "duplicate candidate ID: {}",
                candidate.candidate_id
            ));
        }
        match candidate.quality {
            CandidateQuality::Qualified => {}
            CandidateQuality::NotQualified => blockers.not_qualified += 1,
            CandidateQuality::Unknown => blockers.unknown += 1,
            CandidateQuality::InvalidEvidence => blockers.invalid_evidence += 1,
        }
    }

    let mut remaining: Vec<AcquisitionCandidate> = candidates
        .iter()
        .filter(|candidate| candidate.quality == CandidateQuality::Qualified)
        .cloned()
        .collect();
    let mut current_u = budget.start_u_m;
    let mut current_v = budget.start_v_m;
    let mut elapsed = 0.0;
    let mut energy = 0.0;
    let mut steps = Vec::new();

    loop {
        let all_complete = target_map.iter().all(|(target_id, target)| {
            evidence[target_id].len() >= target.required_independent_passes
        });
        if all_complete || remaining.is_empty() {
            break;
        }

        let mut best: Option<(usize, f64, f64, f64, f64, Vec<String>)> = None;
        for (index, candidate) in remaining.iter().enumerate() {
            let covered: BTreeSet<&str> = candidate
                .covered_target_ids
                .iter()
                .map(String::as_str)
                .collect();
            let mut marginal = Vec::new();
            let mut gain = 0.0;
            for target_id in covered {
                let target = &target_map[target_id];
                let passes = &evidence[target_id];
                if passes.len() >= target.required_independent_passes {
                    continue;
                }
                if passes.iter().any(|pass| pass.pass_id == candidate.pass_id) {
                    continue;
                }
                let angle_is_independent = passes.iter().all(|pass| {
                    angular_separation(pass.view_angle_rad, candidate.view_angle_rad)
                        >= target.minimum_view_angle_separation_rad
                });
                if angle_is_independent {
                    marginal.push(target_id.to_owned());
                    gain += target.weight / target.required_independent_passes as f64;
                }
            }
            if marginal.is_empty() || !gain.is_finite() {
                continue;
            }

            let travel_distance = (candidate.u_m - current_u).hypot(candidate.v_m - current_v);
            let travel_time = travel_distance / budget.travel_speed_m_per_s;
            let step_elapsed = travel_time + candidate.acquisition_time_s;
            if elapsed + step_elapsed > budget.maximum_elapsed_time_s
                || energy + candidate.energy_wh > budget.maximum_energy_wh
            {
                continue;
            }
            let utility_cost = step_elapsed
                + candidate.risk_penalty_s
                + candidate.energy_wh * budget.energy_penalty_s_per_wh;
            if !utility_cost.is_finite() || utility_cost < 0.0 {
                continue;
            }
            let score = gain / utility_cost.max(1.0e-9);
            if !score.is_finite() {
                continue;
            }

            let policy_score = match policy {
                SelectionPolicy::EvidenceGainPerCost => score,
                SelectionPolicy::NearestFeasible => -travel_distance,
                SelectionPolicy::MaximumMarginalEvidence => gain,
                SelectionPolicy::InputOrder => -(index as f64),
                SelectionPolicy::SeededHashOrder { seed } => {
                    deterministic_candidate_rank(seed, &candidate.candidate_id)
                }
            };
            let replace = match &best {
                None => true,
                Some((best_index, best_policy_score, _, _, _, _)) => {
                    policy_score.total_cmp(best_policy_score).is_gt()
                        || (policy_score.total_cmp(best_policy_score).is_eq()
                            && candidate.candidate_id < remaining[*best_index].candidate_id)
                }
            };
            if replace {
                best = Some((
                    index,
                    policy_score,
                    score,
                    travel_distance,
                    travel_time,
                    marginal,
                ));
            }
        }

        let Some((index, _policy_score, score, travel_distance, travel_time, marginal)) = best
        else {
            break;
        };
        let candidate = remaining.remove(index);
        let step_elapsed = travel_time + candidate.acquisition_time_s;
        elapsed += step_elapsed;
        energy += candidate.energy_wh;

        for target_id in &marginal {
            evidence
                .get_mut(target_id)
                .expect("marginal targets came from the validated target map")
                .push(QualifiedExistingPass {
                    target_id: target_id.clone(),
                    pass_id: candidate.pass_id.clone(),
                    view_angle_rad: candidate.view_angle_rad,
                });
        }

        current_u = candidate.u_m;
        current_v = candidate.v_m;
        steps.push(PlannedStep {
            ordinal: steps.len(),
            candidate_id: candidate.candidate_id,
            pass_id: candidate.pass_id,
            marginal_target_ids: marginal,
            travel_distance_m: travel_distance,
            travel_time_s: travel_time,
            acquisition_time_s: candidate.acquisition_time_s,
            risk_penalty_s: candidate.risk_penalty_s,
            energy_wh: candidate.energy_wh,
            utility_score_per_s: score,
            cumulative_elapsed_time_s: elapsed,
            cumulative_energy_wh: energy,
        });
    }

    let mut progress = Vec::with_capacity(target_map.len());
    let mut completed_targets = 0;
    for (target_id, target) in target_map {
        let count = evidence[&target_id].len();
        let complete = count >= target.required_independent_passes;
        if complete {
            completed_targets += 1;
        }
        progress.push(TargetProgress {
            target_id,
            qualifying_passes: count,
            required_independent_passes: target.required_independent_passes,
            remaining_passes: target.required_independent_passes.saturating_sub(count),
            complete,
        });
    }

    let total_targets = progress.len();
    let disposition = if completed_targets == total_targets {
        PlanDisposition::Complete
    } else if steps.is_empty() {
        PlanDisposition::Blocked
    } else {
        PlanDisposition::Partial
    };

    Ok(AcquisitionPlan {
        disposition,
        steps,
        targets: progress,
        completed_targets,
        total_targets,
        cumulative_elapsed_time_s: elapsed,
        cumulative_energy_wh: energy,
        blockers,
    })
}

fn validate_budget(budget: &AcquisitionBudget) -> Result<(), String> {
    if !budget.start_u_m.is_finite()
        || !budget.start_v_m.is_finite()
        || budget.start_u_m.abs() > MAX_ABSOLUTE_COORDINATE_M
        || budget.start_v_m.abs() > MAX_ABSOLUTE_COORDINATE_M
        || !budget.travel_speed_m_per_s.is_finite()
        || budget.travel_speed_m_per_s <= 0.0
        || !budget.maximum_elapsed_time_s.is_finite()
        || budget.maximum_elapsed_time_s <= 0.0
        || !budget.maximum_energy_wh.is_finite()
        || budget.maximum_energy_wh < 0.0
        || !budget.energy_penalty_s_per_wh.is_finite()
        || budget.energy_penalty_s_per_wh < 0.0
    {
        return Err("acquisition budget contains invalid position, speed or limits".to_owned());
    }
    Ok(())
}

fn validate_candidate(
    candidate: &AcquisitionCandidate,
    targets: &BTreeMap<String, EvidenceTarget>,
) -> Result<(), String> {
    if candidate.candidate_id.trim().is_empty()
        || candidate.pass_id.trim().is_empty()
        || !candidate.u_m.is_finite()
        || !candidate.v_m.is_finite()
        || candidate.u_m.abs() > MAX_ABSOLUTE_COORDINATE_M
        || candidate.v_m.abs() > MAX_ABSOLUTE_COORDINATE_M
        || !candidate.view_angle_rad.is_finite()
        || !(-std::f64::consts::PI..=std::f64::consts::PI).contains(&candidate.view_angle_rad)
        || !candidate.acquisition_time_s.is_finite()
        || candidate.acquisition_time_s < 0.0
        || !candidate.energy_wh.is_finite()
        || candidate.energy_wh < 0.0
        || !candidate.risk_penalty_s.is_finite()
        || candidate.risk_penalty_s < 0.0
    {
        return Err(format!(
            "candidate {} contains invalid fields",
            candidate.candidate_id
        ));
    }
    let mut seen = BTreeSet::new();
    for target_id in &candidate.covered_target_ids {
        if !targets.contains_key(target_id) {
            return Err(format!(
                "candidate {} references unknown target: {}",
                candidate.candidate_id, target_id
            ));
        }
        if !seen.insert(target_id) {
            return Err(format!(
                "candidate {} repeats target coverage: {}",
                candidate.candidate_id, target_id
            ));
        }
    }
    Ok(())
}

fn deterministic_candidate_rank(seed: u64, candidate_id: &str) -> f64 {
    let mut hash = 0xcbf29ce484222325_u64 ^ seed;
    for byte in candidate_id.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    (hash as f64) / (u64::MAX as f64)
}

fn angular_separation(first: f64, second: f64) -> f64 {
    let difference = (first - second).abs().rem_euclid(TAU);
    difference.min(TAU - difference)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target(id: &str, required: usize, angle: f64, weight: f64) -> EvidenceTarget {
        EvidenceTarget {
            target_id: id.to_owned(),
            required_independent_passes: required,
            minimum_view_angle_separation_rad: angle,
            weight,
        }
    }

    fn candidate(
        id: &str,
        pass: &str,
        x: f64,
        angle: f64,
        targets: &[&str],
    ) -> AcquisitionCandidate {
        AcquisitionCandidate {
            candidate_id: id.to_owned(),
            pass_id: pass.to_owned(),
            u_m: x,
            v_m: 0.0,
            view_angle_rad: angle,
            covered_target_ids: targets.iter().map(|value| (*value).to_owned()).collect(),
            quality: CandidateQuality::Qualified,
            acquisition_time_s: 1.0,
            energy_wh: 1.0,
            risk_penalty_s: 0.0,
        }
    }

    fn budget() -> AcquisitionBudget {
        AcquisitionBudget {
            start_u_m: 0.0,
            start_v_m: 0.0,
            travel_speed_m_per_s: 1.0,
            maximum_elapsed_time_s: 100.0,
            maximum_energy_wh: 100.0,
            energy_penalty_s_per_wh: 0.1,
        }
    }

    #[test]
    fn selects_multi_target_candidate_by_marginal_gain() {
        let targets = [target("a", 1, 0.0, 1.0), target("b", 1, 0.0, 1.0)];
        let candidates = [
            candidate("single", "p1", 1.0, 0.0, &["a"]),
            candidate("double", "p2", 1.0, 0.0, &["a", "b"]),
        ];
        let plan = plan_acquisition(&targets, &[], &candidates, &budget()).unwrap();
        assert_eq!(plan.disposition, PlanDisposition::Complete);
        assert_eq!(plan.steps[0].candidate_id, "double");
        assert_eq!(plan.steps[0].marginal_target_ids, vec!["a", "b"]);
    }

    #[test]
    fn rejects_replayed_pass_and_selects_new_pass() {
        let targets = [target("a", 2, 0.4, 1.0)];
        let existing = [QualifiedExistingPass {
            target_id: "a".to_owned(),
            pass_id: "pass-1".to_owned(),
            view_angle_rad: 0.0,
        }];
        let candidates = [
            candidate("replay", "pass-1", 1.0, 0.8, &["a"]),
            candidate("new-pass", "pass-2", 1.0, 0.8, &["a"]),
        ];
        let plan = plan_acquisition(&targets, &existing, &candidates, &budget()).unwrap();
        assert_eq!(plan.steps.len(), 1);
        assert_eq!(plan.steps[0].candidate_id, "new-pass");
        assert_eq!(plan.targets[0].qualifying_passes, 2);
    }

    #[test]
    fn rejects_insufficient_view_angle_separation() {
        let targets = [target("a", 2, 0.5, 1.0)];
        let existing = [QualifiedExistingPass {
            target_id: "a".to_owned(),
            pass_id: "pass-1".to_owned(),
            view_angle_rad: 0.0,
        }];
        let candidates = [candidate("same-view", "pass-2", 1.0, 0.2, &["a"])];
        let plan = plan_acquisition(&targets, &existing, &candidates, &budget()).unwrap();
        assert_eq!(plan.disposition, PlanDisposition::Blocked);
        assert!(plan.steps.is_empty());
    }

    #[test]
    fn never_selects_unknown_or_invalid_candidates() {
        let targets = [target("a", 1, 0.0, 1.0)];
        let mut unknown = candidate("unknown", "p1", 1.0, 0.0, &["a"]);
        unknown.quality = CandidateQuality::Unknown;
        let mut invalid = candidate("invalid", "p2", 1.0, 0.0, &["a"]);
        invalid.quality = CandidateQuality::InvalidEvidence;
        let plan = plan_acquisition(&targets, &[], &[unknown, invalid], &budget()).unwrap();
        assert_eq!(plan.disposition, PlanDisposition::Blocked);
        assert_eq!(plan.blockers.unknown, 1);
        assert_eq!(plan.blockers.invalid_evidence, 1);
    }

    #[test]
    fn enforces_energy_and_elapsed_time_budgets() {
        let targets = [target("a", 1, 0.0, 1.0)];
        let mut expensive = candidate("expensive", "p1", 1.0, 0.0, &["a"]);
        expensive.energy_wh = 5.0;
        let mut limited = budget();
        limited.maximum_energy_wh = 4.0;
        let plan = plan_acquisition(&targets, &[], &[expensive], &limited).unwrap();
        assert_eq!(plan.disposition, PlanDisposition::Blocked);

        let mut short = budget();
        short.maximum_elapsed_time_s = 1.5;
        let plan = plan_acquisition(
            &targets,
            &[],
            &[candidate("too-far", "p1", 2.0, 0.0, &["a"])],
            &short,
        )
        .unwrap();
        assert_eq!(plan.disposition, PlanDisposition::Blocked);
    }

    #[test]
    fn tie_breaking_is_stable_by_candidate_id() {
        let targets = [target("a", 1, 0.0, 1.0)];
        let candidates = [
            candidate("zeta", "p-z", 1.0, 0.0, &["a"]),
            candidate("alpha", "p-a", 1.0, 0.0, &["a"]),
        ];
        let first = plan_acquisition(&targets, &[], &candidates, &budget()).unwrap();
        let second = plan_acquisition(&targets, &[], &candidates, &budget()).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.steps[0].candidate_id, "alpha");
    }

    #[test]
    fn marginal_gain_is_normalized_by_required_pass_count() {
        let targets = [
            target("two-pass", 2, 0.0, 2.0),
            target("one-pass", 1, 0.0, 1.5),
        ];
        let candidates = [
            candidate("candidate-two-pass", "p-two", 1.0, 0.0, &["two-pass"]),
            candidate("candidate-one-pass", "p-one", 1.0, 0.0, &["one-pass"]),
            candidate(
                "candidate-two-pass-second",
                "p-two-2",
                1.0,
                1.0,
                &["two-pass"],
            ),
        ];
        let plan = plan_acquisition(&targets, &[], &candidates, &budget()).unwrap();
        assert_eq!(plan.steps[0].candidate_id, "candidate-one-pass");
        assert_eq!(plan.steps[1].candidate_id, "candidate-two-pass");
        assert_eq!(plan.steps[2].candidate_id, "candidate-two-pass-second");
        assert_eq!(plan.disposition, PlanDisposition::Complete);
    }

    #[test]
    fn policy_comparison_uses_identical_gates_but_changes_ordering() {
        let targets = [target("a", 1, 0.0, 1.0), target("b", 1, 0.0, 1.0)];
        let near_single = candidate("near-single", "p-near", 1.0, 0.0, &["a"]);
        let far_multi = candidate("far-multi", "p-far", 10.0, 0.0, &["a", "b"]);
        let candidates = [near_single, far_multi];

        let cost_sensitive = plan_acquisition_with_policy(
            &targets,
            &[],
            &candidates,
            &budget(),
            SelectionPolicy::EvidenceGainPerCost,
        )
        .unwrap();
        let max_gain = plan_acquisition_with_policy(
            &targets,
            &[],
            &candidates,
            &budget(),
            SelectionPolicy::MaximumMarginalEvidence,
        )
        .unwrap();
        let nearest = plan_acquisition_with_policy(
            &targets,
            &[],
            &candidates,
            &budget(),
            SelectionPolicy::NearestFeasible,
        )
        .unwrap();

        assert_eq!(cost_sensitive.steps[0].candidate_id, "near-single");
        assert_eq!(nearest.steps[0].candidate_id, "near-single");
        assert_eq!(max_gain.steps[0].candidate_id, "far-multi");
        for plan in [&cost_sensitive, &max_gain, &nearest] {
            assert_eq!(plan.disposition, PlanDisposition::Complete);
            assert!(plan.cumulative_elapsed_time_s <= budget().maximum_elapsed_time_s);
            assert!(plan.cumulative_energy_wh <= budget().maximum_energy_wh);
        }
    }

    #[test]
    fn every_policy_keeps_unknown_candidates_ineligible() {
        let targets = [target("a", 1, 0.0, 1.0)];
        let mut unknown = candidate("unknown", "p-unknown", 1.0, 0.0, &["a"]);
        unknown.quality = CandidateQuality::Unknown;
        let policies = [
            SelectionPolicy::EvidenceGainPerCost,
            SelectionPolicy::NearestFeasible,
            SelectionPolicy::MaximumMarginalEvidence,
            SelectionPolicy::InputOrder,
            SelectionPolicy::SeededHashOrder { seed: 17 },
        ];
        for policy in policies {
            let plan =
                plan_acquisition_with_policy(&targets, &[], &[unknown.clone()], &budget(), policy)
                    .unwrap();
            assert_eq!(plan.disposition, PlanDisposition::Blocked);
            assert!(plan.steps.is_empty());
            assert_eq!(plan.blockers.unknown, 1);
        }
    }

    #[test]
    fn seeded_hash_policy_is_independent_of_candidate_input_order() {
        let targets = [target("a", 1, 0.0, 1.0)];
        let first = candidate("alpha", "p-a", 1.0, 0.0, &["a"]);
        let second = candidate("beta", "p-b", 1.0, 0.0, &["a"]);
        let forward = plan_acquisition_with_policy(
            &targets,
            &[],
            &[first.clone(), second.clone()],
            &budget(),
            SelectionPolicy::SeededHashOrder { seed: 99 },
        )
        .unwrap();
        let reverse = plan_acquisition_with_policy(
            &targets,
            &[],
            &[second, first],
            &budget(),
            SelectionPolicy::SeededHashOrder { seed: 99 },
        )
        .unwrap();
        assert_eq!(forward.steps[0].candidate_id, reverse.steps[0].candidate_id);
    }

    #[test]
    fn empty_target_set_is_rejected() {
        assert!(plan_acquisition(&[], &[], &[], &budget()).is_err());
    }

    #[test]
    fn invalid_candidate_target_reference_fails_closed() {
        let targets = [target("a", 1, 0.0, 1.0)];
        let candidates = [candidate("bad", "p1", 1.0, 0.0, &["missing"])];
        assert!(plan_acquisition(&targets, &[], &candidates, &budget()).is_err());
    }

    #[test]
    fn angular_separation_wraps_at_pi_boundary() {
        assert!(
            (angular_separation(std::f64::consts::PI - 0.1, -std::f64::consts::PI + 0.1) - 0.2)
                .abs()
                < 1.0e-9
        );
    }
}
