//! Standalone synthetic reference evaluator for adaptive Earth-to-orbit capital policies.
//!
//! This is a bounded decision-policy benchmark, not a real-world infrastructure
//! optimizer. It deliberately uses finite deterministic scenario sets and keeps
//! the output as an objective vector instead of hiding trade-offs in one score.

use std::collections::BTreeSet;

pub const SUITE_VERSION: &str = "earth-orbit-adaptive-capital-v0.1";
pub const FIXTURE_CSV: &str = include_str!("../fixtures/adaptive_capital_v0_1.csv");

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum EvidenceGate { Pass, Unknown, Fail }

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Signal { Low, High }

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum InitialAction { NoBuild, Lease, Build }

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum FollowUpAction { Hold, Expand }

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum EvaluationStatus { Feasible, NeedsEvidence, Infeasible }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Scenario {
    pub id: String,
    pub signal: Signal,
    pub demand: [u32; 3],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BenchmarkCase {
    pub id: String,
    pub family: String,
    pub base_capacity: u32,
    pub lease_capacity: u32,
    pub lease_cost: u64,
    pub build_capacity: u32,
    pub build_cost: u64,
    pub expansion_capacity: u32,
    pub expansion_cost: u64,
    pub operating_cost_per_capacity_period: u64,
    pub evidence_gate: EvidenceGate,
    pub scenarios: Vec<Scenario>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Policy {
    pub initial: InitialAction,
    pub after_low_signal: FollowUpAction,
    pub after_high_signal: FollowUpAction,
}

impl Policy {
    pub fn follow_up(self, signal: Signal) -> FollowUpAction {
        match signal {
            Signal::Low => self.after_low_signal,
            Signal::High => self.after_high_signal,
        }
    }

    fn requires_investment(self) -> bool {
        self.initial != InitialAction::NoBuild
            || self.after_low_signal != FollowUpAction::Hold
            || self.after_high_signal != FollowUpAction::Hold
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScenarioOutcome {
    pub scenario_id: String,
    pub delivered: u64,
    pub unmet: u64,
    pub lifecycle_cost: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RobustMetrics {
    pub minimum_delivered: u64,
    pub maximum_unmet: u64,
    pub maximum_lifecycle_cost: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolicyResult {
    pub policy: Policy,
    pub status: EvaluationStatus,
    pub outcomes: Vec<ScenarioOutcome>,
    pub robust: Option<RobustMetrics>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaseReport {
    pub case_id: String,
    pub evaluated_policies: usize,
    pub feasible_policies: usize,
    pub pareto_policies: Vec<PolicyResult>,
    pub needs_evidence_policies: usize,
    pub infeasible_policies: usize,
}

pub fn load_suite() -> Result<Vec<BenchmarkCase>, String> {
    let mut cases = Vec::new();
    let mut ids = BTreeSet::new();
    for (line_index, line) in FIXTURE_CSV.lines().enumerate().skip(1) {
        if line.trim().is_empty() || line.starts_with('#') { continue; }
        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() != 18 {
            return Err(format!("line {}: expected 18 fields, got {}", line_index + 1, fields.len()));
        }
        let id = fields[0].to_string();
        if id.trim().is_empty() || !ids.insert(id.clone()) {
            return Err(format!("line {}: empty or duplicate case id {}", line_index + 1, id));
        }
        let n = |i: usize| -> Result<u64, String> {
            fields[i].parse::<u64>().map_err(|e| format!("line {} field {}: {}", line_index + 1, i + 1, e))
        };
        let gate = match fields[17] {
            "PASS" => EvidenceGate::Pass,
            "UNKNOWN" => EvidenceGate::Unknown,
            "FAIL" => EvidenceGate::Fail,
            other => return Err(format!("line {}: unknown evidence gate {}", line_index + 1, other)),
        };
        let base_capacity = u32::try_from(n(2)?).map_err(|_| format!("line {}: base capacity overflow", line_index + 1))?;
        let lease_capacity = u32::try_from(n(3)?).map_err(|_| format!("line {}: lease capacity overflow", line_index + 1))?;
        let lease_cost = n(4)?;
        let build_capacity = u32::try_from(n(5)?).map_err(|_| format!("line {}: build capacity overflow", line_index + 1))?;
        let build_cost = n(6)?;
        let expansion_capacity = u32::try_from(n(7)?).map_err(|_| format!("line {}: expansion capacity overflow", line_index + 1))?;
        let expansion_cost = n(8)?;
        let operating_cost_per_capacity_period = n(9)?;
        let demand_0 = u32::try_from(n(10)?).map_err(|_| format!("line {}: demand overflow", line_index + 1))?;
        let low_1 = u32::try_from(n(11)?).map_err(|_| format!("line {}: demand overflow", line_index + 1))?;
        let low_2a = u32::try_from(n(12)?).map_err(|_| format!("line {}: demand overflow", line_index + 1))?;
        let low_2b = u32::try_from(n(13)?).map_err(|_| format!("line {}: demand overflow", line_index + 1))?;
        let high_1 = u32::try_from(n(14)?).map_err(|_| format!("line {}: demand overflow", line_index + 1))?;
        let high_2a = u32::try_from(n(15)?).map_err(|_| format!("line {}: demand overflow", line_index + 1))?;
        let high_2b = u32::try_from(n(16)?).map_err(|_| format!("line {}: demand overflow", line_index + 1))?;
        let family = fields[1].to_string();
        let scenarios = vec![
            Scenario { id: format!("{id}-LOW-A"), signal: Signal::Low, demand: [demand_0, low_1, low_2a] },
            Scenario { id: format!("{id}-LOW-B"), signal: Signal::Low, demand: [demand_0, low_1, low_2b] },
            Scenario { id: format!("{id}-HIGH-A"), signal: Signal::High, demand: [demand_0, high_1, high_2a] },
            Scenario { id: format!("{id}-HIGH-B"), signal: Signal::High, demand: [demand_0, high_1, high_2b] },
        ];
        cases.push(BenchmarkCase {
            id, family, base_capacity, lease_capacity, lease_cost,
            build_capacity, build_cost, expansion_capacity, expansion_cost,
            operating_cost_per_capacity_period, evidence_gate: gate, scenarios,
        });
    }
    if cases.len() != 30 {
        return Err(format!("expected 30 frozen cases, found {}", cases.len()));
    }
    Ok(cases)
}

pub fn enumerate_policies() -> Vec<Policy> {
    let initials = [InitialAction::NoBuild, InitialAction::Lease, InitialAction::Build];
    let follow_ups = [FollowUpAction::Hold, FollowUpAction::Expand];
    let mut policies = Vec::new();
    for initial in initials {
        for low in follow_ups {
            for high in follow_ups {
                policies.push(Policy { initial, after_low_signal: low, after_high_signal: high });
            }
        }
    }
    policies
}

pub fn evaluate_policy(case: &BenchmarkCase, policy: Policy) -> PolicyResult {
    if policy.requires_investment() {
        match case.evidence_gate {
            EvidenceGate::Pass => {}
            EvidenceGate::Unknown => return PolicyResult {
                policy, status: EvaluationStatus::NeedsEvidence, outcomes: Vec::new(), robust: None
            },
            EvidenceGate::Fail => return PolicyResult {
                policy, status: EvaluationStatus::Infeasible, outcomes: Vec::new(), robust: None
            },
        }
    }

    let mut outcomes = Vec::with_capacity(case.scenarios.len());
    for scenario in &case.scenarios {
        let leased = if policy.initial == InitialAction::Lease { case.lease_capacity } else { 0 };
        let built = if policy.initial == InitialAction::Build { case.build_capacity } else { 0 };
        let expanded = if policy.follow_up(scenario.signal) == FollowUpAction::Expand {
            case.expansion_capacity
        } else { 0 };

        let capacities = [
            case.base_capacity.saturating_add(leased),
            case.base_capacity.saturating_add(leased).saturating_add(built),
            case.base_capacity.saturating_add(leased).saturating_add(built).saturating_add(expanded),
        ];
        let mut delivered = 0_u64;
        let mut unmet = 0_u64;
        for (period, demand) in scenario.demand.iter().enumerate() {
            let served = u64::from((*demand).min(capacities[period]));
            delivered = delivered.saturating_add(served);
            unmet = unmet.saturating_add(u64::from(*demand) - served);
        }

        let initial_cost = match policy.initial {
            InitialAction::NoBuild => 0,
            InitialAction::Lease => case.lease_cost,
            InitialAction::Build => case.build_cost,
        };
        let follow_cost = if policy.follow_up(scenario.signal) == FollowUpAction::Expand {
            case.expansion_cost
        } else { 0 };
        let lease_periods = if leased > 0 { 3_u64 } else { 0 };
        let build_periods = if built > 0 { 2_u64 } else { 0 };
        let expansion_periods = if expanded > 0 { 1_u64 } else { 0 };
        let capacity_periods = u64::from(leased) * lease_periods
            + u64::from(built) * build_periods
            + u64::from(expanded) * expansion_periods;
        let operating = capacity_periods.saturating_mul(case.operating_cost_per_capacity_period);
        let lifecycle_cost = initial_cost.saturating_add(follow_cost).saturating_add(operating);

        outcomes.push(ScenarioOutcome {
            scenario_id: scenario.id.clone(), delivered, unmet, lifecycle_cost
        });
    }
    let minimum_delivered = outcomes.iter().map(|x| x.delivered).min().unwrap_or(0);
    let maximum_unmet = outcomes.iter().map(|x| x.unmet).max().unwrap_or(0);
    let maximum_lifecycle_cost = outcomes.iter().map(|x| x.lifecycle_cost).max().unwrap_or(0);
    PolicyResult {
        policy, status: EvaluationStatus::Feasible, outcomes,
        robust: Some(RobustMetrics { minimum_delivered, maximum_unmet, maximum_lifecycle_cost })
    }
}

fn dominates(a: RobustMetrics, b: RobustMetrics) -> bool {
    let no_worse = a.minimum_delivered >= b.minimum_delivered
        && a.maximum_unmet <= b.maximum_unmet
        && a.maximum_lifecycle_cost <= b.maximum_lifecycle_cost;
    let strictly_better = a.minimum_delivered > b.minimum_delivered
        || a.maximum_unmet < b.maximum_unmet
        || a.maximum_lifecycle_cost < b.maximum_lifecycle_cost;
    no_worse && strictly_better
}

pub fn evaluate_case(case: &BenchmarkCase) -> CaseReport {
    let all: Vec<PolicyResult> = enumerate_policies().into_iter()
        .map(|p| evaluate_policy(case, p)).collect();
    let feasible: Vec<PolicyResult> = all.iter()
        .filter(|r| r.status == EvaluationStatus::Feasible).cloned().collect();
    let pareto_policies: Vec<PolicyResult> = feasible.iter().filter(|candidate| {
        let Some(candidate_metrics) = candidate.robust else { return false; };
        !feasible.iter().any(|other| {
            other.policy != candidate.policy
                && other.robust.is_some_and(|m| dominates(m, candidate_metrics))
        })
    }).cloned().collect();
    CaseReport {
        case_id: case.id.clone(),
        evaluated_policies: all.len(),
        feasible_policies: feasible.len(),
        pareto_policies,
        needs_evidence_policies: all.iter().filter(|x| x.status == EvaluationStatus::NeedsEvidence).count(),
        infeasible_policies: all.iter().filter(|x| x.status == EvaluationStatus::Infeasible).count(),
    }
}

pub fn suite_summary() -> Result<String, String> {
    let cases = load_suite()?;
    let mut output = format!("suite={} cases={} policies_per_case={}\n", SUITE_VERSION, cases.len(), enumerate_policies().len());
    for case in cases {
        let report = evaluate_case(&case);
        output.push_str(&format!(
            "{} family={} feasible={} needs_evidence={} infeasible={} pareto={}\n",
            report.case_id, case.family, report.feasible_policies, report.needs_evidence_policies,
            report.infeasible_policies, report.pareto_policies.len()
        ));
    }
    Ok(output)
}

/// Non-deployable perfect-information service upper bound for a fixed initial
/// action. Each scenario independently chooses whether to expand after seeing
/// its entire future demand path. Incremental cost is ignored deliberately.
pub fn perfect_foresight_total_delivered(case: &BenchmarkCase, initial: InitialAction) -> Option<u64> {
    if initial != InitialAction::NoBuild && case.evidence_gate != EvidenceGate::Pass {
        return None;
    }
    let lease_units = if initial == InitialAction::Lease { case.lease_capacity } else { 0 };
    let build_units = if initial == InitialAction::Build { case.build_capacity } else { 0 };
    let mut total = 0_u64;
    for scenario in &case.scenarios {
        let base = case.base_capacity.saturating_add(lease_units);
        let mid = base.saturating_add(build_units);
        let hold_caps = [base, mid, mid];
        let expand_caps = [base, mid, mid.saturating_add(case.expansion_capacity)];
        let mut hold_delivered = 0_u64;
        let mut expand_delivered = 0_u64;
        for ((&demand, &hold_capacity), &expand_capacity) in scenario
            .demand
            .iter()
            .zip(hold_caps.iter())
            .zip(expand_caps.iter())
        {
            hold_delivered = hold_delivered.saturating_add(u64::from(demand.min(hold_capacity)));
            expand_delivered = expand_delivered.saturating_add(u64::from(demand.min(expand_capacity)));
        }
        let best = if case.evidence_gate == EvidenceGate::Pass {
            hold_delivered.max(expand_delivered)
        } else {
            hold_delivered
        };
        total = total.saturating_add(best);
    }
    Some(total)
}

// Independent exhaustive oracle used by tests. This implementation duplicates
// the finite transition arithmetic instead of calling evaluate_policy/dominates.
#[cfg(test)]
fn oracle_pareto_policies(case: &BenchmarkCase) -> BTreeSet<Policy> {
    let mut rows: Vec<(Policy, Option<(u64, u64, u64)>)> = Vec::new();
    for initial_index in 0..3 {
        for low_index in 0..2 {
            for high_index in 0..2 {
                let initial = match initial_index {
                    0 => InitialAction::NoBuild,
                    1 => InitialAction::Lease,
                    _ => InitialAction::Build,
                };
                let low_action = if low_index == 0 { FollowUpAction::Hold } else { FollowUpAction::Expand };
                let high_action = if high_index == 0 { FollowUpAction::Hold } else { FollowUpAction::Expand };
                let policy = Policy { initial, after_low_signal: low_action, after_high_signal: high_action };
                let invests = initial_index != 0 || low_index != 0 || high_index != 0;
                if invests && case.evidence_gate != EvidenceGate::Pass {
                    rows.push((policy, None));
                    continue;
                }
                let mut min_delivered = u64::MAX;
                let mut max_unmet = 0_u64;
                let mut max_cost = 0_u64;
                for scenario in &case.scenarios {
                    let lease_units = if initial_index == 1 { case.lease_capacity } else { 0 };
                    let build_units = if initial_index == 2 { case.build_capacity } else { 0 };
                    let do_expand = match scenario.signal {
                        Signal::Low => low_index == 1,
                        Signal::High => high_index == 1,
                    };
                    let expansion_units = if do_expand { case.expansion_capacity } else { 0 };
                    let c0 = case.base_capacity + lease_units;
                    let c1 = c0 + build_units;
                    let c2 = c1 + expansion_units;
                    let caps = [c0, c1, c2];
                    let mut delivered = 0_u64;
                    let mut unmet = 0_u64;
                    for (&demand, &capacity) in scenario.demand.iter().zip(caps.iter()) {
                        let required = u64::from(demand);
                        let served = required.min(u64::from(capacity));
                        delivered += served;
                        unmet += required - served;
                    }
                    let mut cost = 0_u64;
                    if initial_index == 1 {
                        cost += case.lease_cost + u64::from(lease_units) * 3 * case.operating_cost_per_capacity_period;
                    } else if initial_index == 2 {
                        cost += case.build_cost + u64::from(build_units) * 2 * case.operating_cost_per_capacity_period;
                    }
                    if do_expand {
                        cost += case.expansion_cost + u64::from(expansion_units) * case.operating_cost_per_capacity_period;
                    }
                    if delivered < min_delivered { min_delivered = delivered; }
                    if unmet > max_unmet { max_unmet = unmet; }
                    if cost > max_cost { max_cost = cost; }
                }
                rows.push((policy, Some((min_delivered, max_unmet, max_cost))));
            }
        }
    }
    let mut frontier = BTreeSet::new();
    for (index, (policy, metrics)) in rows.iter().enumerate() {
        let Some((d, u, c)) = metrics else { continue; };
        let dominated = rows.iter().enumerate().any(|(other_index, (_, other))| {
            if other_index == index { return false; }
            match other {
                Some((od, ou, oc)) => *od >= *d && *ou <= *u && *oc <= *c
                    && (*od > *d || *ou < *u || *oc < *c),
                None => false,
            }
        });
        if !dominated { frontier.insert(*policy); }
    }
    frontier
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixture_suite_has_thirty_unique_synthetic_cases() {
        let suite = load_suite().expect("valid frozen corpus");
        assert_eq!(suite.len(), 30);
        let mut ids = BTreeSet::new();
        assert!(suite.iter().all(|case| ids.insert(case.id.clone())));
    }

    #[test]
    fn each_case_enumerates_twelve_policies() {
        for case in load_suite().unwrap() {
            assert_eq!(evaluate_case(&case).evaluated_policies, 12, "{}", case.id);
        }
    }

    #[test]
    fn unknown_evidence_never_qualifies_an_investment() {
        let case = load_suite().unwrap().into_iter().find(|x| x.id == "A19").unwrap();
        let result = evaluate_policy(&case, Policy {
            initial: InitialAction::Lease,
            after_low_signal: FollowUpAction::Hold,
            after_high_signal: FollowUpAction::Hold,
        });
        assert_eq!(result.status, EvaluationStatus::NeedsEvidence);
        assert!(result.robust.is_none());
    }

    #[test]
    fn failed_gate_does_not_remove_no_build_baseline() {
        let case = load_suite().unwrap().into_iter().find(|x| x.id == "A11").unwrap();
        let result = evaluate_policy(&case, Policy {
            initial: InitialAction::NoBuild,
            after_low_signal: FollowUpAction::Hold,
            after_high_signal: FollowUpAction::Hold,
        });
        assert_eq!(result.status, EvaluationStatus::Feasible);
        assert_eq!(result.outcomes.len(), 4);
    }

    #[test]
    fn failed_gate_blocks_even_economically_attractive_build() {
        let case = load_suite().unwrap().into_iter().find(|x| x.id == "A11").unwrap();
        let result = evaluate_policy(&case, Policy {
            initial: InitialAction::Build,
            after_low_signal: FollowUpAction::Hold,
            after_high_signal: FollowUpAction::Hold,
        });
        assert_eq!(result.status, EvaluationStatus::Infeasible);
    }

    #[test]
    fn same_signal_maps_to_same_action_despite_different_future_realizations() {
        let case = load_suite().unwrap().into_iter().find(|x| x.id == "A29").unwrap();
        let policy = Policy {
            initial: InitialAction::NoBuild,
            after_low_signal: FollowUpAction::Expand,
            after_high_signal: FollowUpAction::Hold,
        };
        let low_paths: Vec<&Scenario> = case.scenarios.iter().filter(|x| x.signal == Signal::Low).collect();
        assert_eq!(low_paths.len(), 2);
        assert_ne!(low_paths[0].id, low_paths[1].id);
        assert_eq!(low_paths[0].signal, low_paths[1].signal);
        assert_eq!(policy.follow_up(low_paths[0].signal), policy.follow_up(low_paths[1].signal));
        assert_ne!(low_paths[0].demand[2], low_paths[1].demand[2]);
    }

    #[test]
    fn robust_frontier_never_contains_a_dominated_policy() {
        for case in load_suite().unwrap() {
            let report = evaluate_case(&case);
            for candidate in &report.pareto_policies {
                for other in &report.pareto_policies {
                    if candidate.policy == other.policy { continue; }
                    let a = candidate.robust.unwrap();
                    let b = other.robust.unwrap();
                    assert!(!dominates(a, b));
                }
            }
        }
    }

    #[test]
    fn independent_exhaustive_oracle_matches_production_pareto_set() {
        for case in load_suite().unwrap() {
            let production: BTreeSet<Policy> = evaluate_case(&case).pareto_policies
                .into_iter().map(|x| x.policy).collect();
            let oracle = oracle_pareto_policies(&case);
            assert_eq!(production, oracle, "{}", case.id);
        }
    }

    #[test]
    fn deterministic_suite_summary_is_byte_stable() {
        assert_eq!(suite_summary().unwrap(), suite_summary().unwrap());
    }

    #[test]
    fn perfect_foresight_is_a_non_deployable_service_upper_bound() {
        let case = load_suite().unwrap().into_iter().find(|x| x.id == "A29").unwrap();
        let bound = perfect_foresight_total_delivered(&case, InitialAction::NoBuild).unwrap();
        for policy in enumerate_policies().into_iter().filter(|p| p.initial == InitialAction::NoBuild) {
            let result = evaluate_policy(&case, policy);
            if result.status == EvaluationStatus::Feasible {
                let delivered: u64 = result.outcomes.iter().map(|x| x.delivered).sum();
                assert!(bound >= delivered);
            }
        }
    }

    #[test]
    fn no_build_is_always_in_the_candidate_policy_space() {
        for case in load_suite().unwrap() {
            let no_build = Policy {
                initial: InitialAction::NoBuild,
                after_low_signal: FollowUpAction::Hold,
                after_high_signal: FollowUpAction::Hold,
            };
            let result = evaluate_policy(&case, no_build);
            assert_eq!(result.status, EvaluationStatus::Feasible, "{}", case.id);
        }
    }
}
