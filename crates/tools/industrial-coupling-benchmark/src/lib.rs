//! Tiny exact-enumeration industrial heat-exchange benchmark.
//!
//! This is a discrete, constant-temperature screening model for validating
//! coordination economics and ledgers. It is not a pinch solver, dynamic
//! process simulator, or plant-control system.

#![forbid(unsafe_code)]

use std::collections::BTreeSet;
use std::fmt;

pub const MAX_EXHAUSTIVE_TRANSFER_KWH: u64 = 100_000;
pub const MAX_ROBUST_SCENARIOS: usize = 64;
pub const MAX_ROBUST_EVALUATIONS: u64 = 2_000_000;
const BASIS_POINTS: u64 = 10_000;

/// One synthetic two-facility heat-exchange scenario.
///
/// Energy is integer kWh-th. Costs are integer milli-credits. Prices are per
/// kWh-th, and the transfer variable cost is charged per input kWh. This
/// integer representation makes the reference ledger deterministic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scenario {
    pub scenario_id: String,
    pub source_surplus_kwh: u64,
    pub sink_heat_demand_kwh: u64,
    pub link_capacity_kwh: u64,
    /// Efficiency in basis points: 9_000 means 90%.
    pub transfer_efficiency_bps: u16,
    pub source_supply_temp_c: i32,
    pub sink_required_temp_c: i32,
    pub minimum_approach_c: u32,
    pub avoided_utility_cost_milli_per_kwh: u64,
    pub source_opportunity_cost_milli_per_kwh: u64,
    pub transfer_variable_cost_milli_per_input_kwh: u64,
    pub coordination_fixed_cost_milli: u64,
    /// Share of positive network surplus allocated to the source participant.
    pub source_surplus_share_bps: u16,
    /// Minimum net surplus required by the source participant to agree to exchange.
    pub source_minimum_participation_surplus_milli: u64,
    /// Minimum net surplus required by the sink participant to agree to exchange.
    pub sink_minimum_participation_surplus_milli: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Plan {
    pub transfer_input_kwh: u64,
    pub heat_delivered_kwh: u64,
    pub transfer_loss_kwh: u64,
    pub utility_cost_avoided_milli: u64,
    pub source_opportunity_cost_milli: u64,
    pub transfer_variable_cost_milli: u64,
    pub coordination_fixed_cost_milli: u64,
    /// Avoided real costs minus opportunity and real coordination costs.
    /// Internal surplus allocations are transfers, not additional social costs.
    pub network_surplus_milli: i128,
    pub source_surplus_allocation_milli: u64,
    pub sink_surplus_allocation_milli: u64,
}

/// One common transfer decision evaluated against every scenario in a finite
/// uncertainty set. Scenario plans are kept in the same order as the input.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RobustPlan {
    pub transfer_input_kwh: u64,
    pub scenario_plans: Vec<Plan>,
    pub worst_case_surplus_milli: i128,
    pub total_surplus_milli: i128,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BenchmarkError {
    InvalidScenario(&'static str),
    ArithmeticOverflow,
    SearchSpaceTooLarge { bound_kwh: u64, maximum_kwh: u64 },
    InfeasiblePlan(&'static str),
    AccountingMismatch(&'static str),
    NonOptimalPlan,
    EmptyScenarioSet,
    DuplicateScenarioId(String),
    TooManyScenarios { count: usize, maximum: usize },
    RobustSearchTooLarge { evaluations: u64, maximum: u64 },
}

impl fmt::Display for BenchmarkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidScenario(message) => write!(f, "invalid scenario: {message}"),
            Self::ArithmeticOverflow => write!(f, "integer ledger arithmetic overflow"),
            Self::SearchSpaceTooLarge {
                bound_kwh,
                maximum_kwh,
            } => write!(
                f,
                "exact search bound {bound_kwh} kWh exceeds configured maximum {maximum_kwh} kWh"
            ),
            Self::InfeasiblePlan(message) => write!(f, "infeasible plan: {message}"),
            Self::AccountingMismatch(message) => write!(f, "accounting mismatch: {message}"),
            Self::NonOptimalPlan => {
                write!(f, "plan is feasible but not optimal for this exact case")
            }
            Self::EmptyScenarioSet => write!(f, "robust evaluation needs at least one scenario"),
            Self::DuplicateScenarioId(id) => write!(f, "duplicate scenario identifier: {id}"),
            Self::TooManyScenarios { count, maximum } => write!(
                f,
                "scenario count {count} exceeds configured maximum {maximum}"
            ),
            Self::RobustSearchTooLarge {
                evaluations,
                maximum,
            } => write!(
                f,
                "robust search requires {evaluations} scenario evaluations, above configured maximum {maximum}"
            ),
        }
    }
}

impl std::error::Error for BenchmarkError {}

impl Scenario {
    pub fn validate(&self) -> Result<(), BenchmarkError> {
        if self.scenario_id.trim().is_empty() {
            return Err(BenchmarkError::InvalidScenario(
                "scenario_id must not be empty",
            ));
        }
        if !(1..=10_000).contains(&self.transfer_efficiency_bps) {
            return Err(BenchmarkError::InvalidScenario(
                "transfer_efficiency_bps must be in 1..=10000",
            ));
        }
        if self.source_surplus_share_bps > 10_000 {
            return Err(BenchmarkError::InvalidScenario(
                "source_surplus_share_bps must be in 0..=10000",
            ));
        }
        self.source_minimum_participation_surplus_milli
            .checked_add(self.sink_minimum_participation_surplus_milli)
            .ok_or(BenchmarkError::ArithmeticOverflow)?;
        Ok(())
    }

    pub fn temperature_grade_is_compatible(&self) -> bool {
        (self.source_supply_temp_c as i64)
            >= (self.sink_required_temp_c as i64) + (self.minimum_approach_c as i64)
    }

    pub fn maximum_transfer_kwh(&self) -> u64 {
        self.source_surplus_kwh.min(self.link_capacity_kwh)
    }
}

/// Enumerate integer-kWh exchange volumes and retain the highest-surplus plan.
/// The zero-transfer plan is always feasible and protects the independent
/// baseline when exchange is physically infeasible or economically harmful.
pub fn evaluate(scenario: &Scenario) -> Result<Plan, BenchmarkError> {
    scenario.validate()?;
    let upper = scenario.maximum_transfer_kwh();
    if upper > MAX_EXHAUSTIVE_TRANSFER_KWH {
        return Err(BenchmarkError::SearchSpaceTooLarge {
            bound_kwh: upper,
            maximum_kwh: MAX_EXHAUSTIVE_TRANSFER_KWH,
        });
    }

    let mut best = Plan::default();
    for input_kwh in 1..=upper {
        let Some(candidate) = assess_candidate(scenario, input_kwh)? else {
            continue;
        };
        if candidate.network_surplus_milli > best.network_surplus_milli
            || (candidate.network_surplus_milli == best.network_surplus_milli
                && candidate.transfer_input_kwh < best.transfer_input_kwh)
        {
            best = candidate;
        }
    }
    verify_optimal_plan(scenario, &best)?;
    Ok(best)
}

fn assess_candidate(s: &Scenario, input_kwh: u64) -> Result<Option<Plan>, BenchmarkError> {
    if input_kwh == 0 || input_kwh > s.maximum_transfer_kwh() {
        return Ok(None);
    }
    if !s.temperature_grade_is_compatible() {
        return Ok(None);
    }

    let delivered = input_kwh
        .checked_mul(s.transfer_efficiency_bps as u64)
        .ok_or(BenchmarkError::ArithmeticOverflow)?
        / BASIS_POINTS;
    if delivered == 0 || delivered > s.sink_heat_demand_kwh {
        return Ok(None);
    }
    let avoided = checked_cost(delivered, s.avoided_utility_cost_milli_per_kwh)?;
    let source_cost = checked_cost(input_kwh, s.source_opportunity_cost_milli_per_kwh)?;
    let variable_cost = checked_cost(input_kwh, s.transfer_variable_cost_milli_per_input_kwh)?;
    let fixed_cost = s.coordination_fixed_cost_milli;
    let real_cost = source_cost
        .checked_add(variable_cost)
        .and_then(|v| v.checked_add(fixed_cost))
        .ok_or(BenchmarkError::ArithmeticOverflow)?;
    let surplus = (avoided as i128) - (real_cost as i128);
    let distributable = surplus.max(0) as u64;
    let reserved = s
        .source_minimum_participation_surplus_milli
        .checked_add(s.sink_minimum_participation_surplus_milli)
        .ok_or(BenchmarkError::ArithmeticOverflow)?;
    if surplus < reserved as i128 {
        return Ok(None);
    }
    let remaining = distributable - reserved;
    let source_share = s.source_minimum_participation_surplus_milli
        + (((remaining as u128) * (s.source_surplus_share_bps as u128) / (BASIS_POINTS as u128))
            as u64);
    let sink_share = s.sink_minimum_participation_surplus_milli
        + (remaining - (source_share - s.source_minimum_participation_surplus_milli));

    Ok(Some(Plan {
        transfer_input_kwh: input_kwh,
        heat_delivered_kwh: delivered,
        transfer_loss_kwh: input_kwh - delivered,
        utility_cost_avoided_milli: avoided,
        source_opportunity_cost_milli: source_cost,
        transfer_variable_cost_milli: variable_cost,
        coordination_fixed_cost_milli: fixed_cost,
        network_surplus_milli: surplus,
        source_surplus_allocation_milli: source_share,
        sink_surplus_allocation_milli: sink_share,
    }))
}

/// Verify the selected plan's physical bounds, heat ledger, money ledger, and
/// surplus allocation. This routine deliberately recomputes those quantities
/// rather than trusting optimizer-reported totals.
pub fn verify_plan(s: &Scenario, p: &Plan) -> Result<(), BenchmarkError> {
    s.validate()?;
    if p.transfer_input_kwh > s.maximum_transfer_kwh() {
        return Err(BenchmarkError::InfeasiblePlan(
            "transfer exceeds source surplus or link capacity",
        ));
    }
    if p.transfer_input_kwh == 0 {
        if *p != Plan::default() {
            return Err(BenchmarkError::AccountingMismatch(
                "zero-transfer baseline must have zero flows, costs, and allocations",
            ));
        }
        return Ok(());
    }
    if !s.temperature_grade_is_compatible() {
        return Err(BenchmarkError::InfeasiblePlan(
            "source temperature does not meet sink grade plus minimum approach",
        ));
    }

    let expected_delivery = p
        .transfer_input_kwh
        .checked_mul(s.transfer_efficiency_bps as u64)
        .ok_or(BenchmarkError::ArithmeticOverflow)?
        / BASIS_POINTS;
    if p.heat_delivered_kwh != expected_delivery {
        return Err(BenchmarkError::AccountingMismatch(
            "delivered heat does not match declared efficiency",
        ));
    }
    if p.heat_delivered_kwh == 0 || p.heat_delivered_kwh > s.sink_heat_demand_kwh {
        return Err(BenchmarkError::InfeasiblePlan(
            "delivered heat is zero or exceeds remaining sink demand",
        ));
    }
    if p.transfer_loss_kwh != p.transfer_input_kwh - p.heat_delivered_kwh {
        return Err(BenchmarkError::AccountingMismatch(
            "input heat must equal delivered heat plus transfer loss",
        ));
    }

    let avoided = checked_cost(p.heat_delivered_kwh, s.avoided_utility_cost_milli_per_kwh)?;
    let source_cost = checked_cost(
        p.transfer_input_kwh,
        s.source_opportunity_cost_milli_per_kwh,
    )?;
    let variable_cost = checked_cost(
        p.transfer_input_kwh,
        s.transfer_variable_cost_milli_per_input_kwh,
    )?;
    let fixed_cost = s.coordination_fixed_cost_milli;
    let real_cost = source_cost
        .checked_add(variable_cost)
        .and_then(|v| v.checked_add(fixed_cost))
        .ok_or(BenchmarkError::ArithmeticOverflow)?;
    let net = (avoided as i128) - (real_cost as i128);
    if net != p.network_surplus_milli {
        return Err(BenchmarkError::AccountingMismatch(
            "network surplus does not equal avoided real cost minus real costs",
        ));
    }
    if net < 0 {
        return Err(BenchmarkError::InfeasiblePlan(
            "exchange destroys value versus the zero-exchange baseline",
        ));
    }
    let reserved = s
        .source_minimum_participation_surplus_milli
        .checked_add(s.sink_minimum_participation_surplus_milli)
        .ok_or(BenchmarkError::ArithmeticOverflow)?;
    if (net as u64) < reserved {
        return Err(BenchmarkError::InfeasiblePlan(
            "net surplus cannot satisfy both participants' reservation floors",
        ));
    }
    if avoided != p.utility_cost_avoided_milli
        || source_cost != p.source_opportunity_cost_milli
        || variable_cost != p.transfer_variable_cost_milli
        || fixed_cost != p.coordination_fixed_cost_milli
    {
        return Err(BenchmarkError::AccountingMismatch(
            "one or more reported cost components do not reconcile",
        ));
    }

    let distributable = net as u64;
    let remaining = distributable - reserved;
    let expected_source_share = s.source_minimum_participation_surplus_milli
        + (((remaining as u128) * (s.source_surplus_share_bps as u128) / (BASIS_POINTS as u128))
            as u64);
    let expected_sink_share = s.sink_minimum_participation_surplus_milli
        + (remaining - (expected_source_share - s.source_minimum_participation_surplus_milli));
    if p.source_surplus_allocation_milli != expected_source_share
        || p.sink_surplus_allocation_milli != expected_sink_share
        || p.source_surplus_allocation_milli + p.sink_surplus_allocation_milli != distributable
    {
        return Err(BenchmarkError::AccountingMismatch(
            "participant surplus allocations do not reconcile to network surplus",
        ));
    }
    Ok(())
}

/// Independently scan the finite feasible domain to verify that no candidate
/// has a higher net surplus (ties prefer the lower transfer volume). The scoring
/// logic is intentionally restated here rather than calling the optimizer's
/// candidate evaluator.
pub fn verify_optimal_plan(s: &Scenario, p: &Plan) -> Result<(), BenchmarkError> {
    verify_plan(s, p)?;
    if p.transfer_input_kwh > 0 && p.network_surplus_milli == 0 {
        // The independent-operation baseline achieves the same surplus with less exchange.
        return Err(BenchmarkError::NonOptimalPlan);
    }
    let upper = s.maximum_transfer_kwh();
    if upper > MAX_EXHAUSTIVE_TRANSFER_KWH {
        return Err(BenchmarkError::SearchSpaceTooLarge {
            bound_kwh: upper,
            maximum_kwh: MAX_EXHAUSTIVE_TRANSFER_KWH,
        });
    }
    let reservation_total = s
        .source_minimum_participation_surplus_milli
        .checked_add(s.sink_minimum_participation_surplus_milli)
        .ok_or(BenchmarkError::ArithmeticOverflow)?;
    for q_in in 1..=upper {
        if q_in > s.maximum_transfer_kwh() || !s.temperature_grade_is_compatible() {
            continue;
        }
        let q_out = q_in
            .checked_mul(s.transfer_efficiency_bps as u64)
            .ok_or(BenchmarkError::ArithmeticOverflow)?
            / BASIS_POINTS;
        if q_out == 0 || q_out > s.sink_heat_demand_kwh {
            continue;
        }
        let avoided = checked_cost(q_out, s.avoided_utility_cost_milli_per_kwh)?;
        let foregone = checked_cost(q_in, s.source_opportunity_cost_milli_per_kwh)?;
        let transfer_cost = checked_cost(q_in, s.transfer_variable_cost_milli_per_input_kwh)?;
        let all_real_costs = foregone
            .checked_add(transfer_cost)
            .and_then(|v| v.checked_add(s.coordination_fixed_cost_milli))
            .ok_or(BenchmarkError::ArithmeticOverflow)?;
        let candidate_surplus = (avoided as i128) - (all_real_costs as i128);
        if candidate_surplus < reservation_total as i128 {
            continue;
        }
        if candidate_surplus > p.network_surplus_milli
            || (candidate_surplus == p.network_surplus_milli && q_in < p.transfer_input_kwh)
        {
            return Err(BenchmarkError::NonOptimalPlan);
        }
    }
    Ok(())
}

/// Choose one common exchange quantity that is feasible and meets both
/// participants' reservation floors in every supplied scenario. The primary
/// objective is maximum worst-case surplus; ties among positive worst-case
/// plans prefer greater total surplus, then lower transfer volume. A zero
/// exchange is always available and wins ties at zero to avoid needless
/// coordination overhead.
///
/// This is a finite-set robust decision rule, not a probability model. It is
/// appropriate only when a single common transfer quantity must remain valid
/// across all listed scenarios; recourse/adaptive policies are a later extension.
pub fn evaluate_robust(scenarios: &[Scenario]) -> Result<RobustPlan, BenchmarkError> {
    validate_scenario_set(scenarios)?;
    let upper = scenarios
        .iter()
        .map(Scenario::maximum_transfer_kwh)
        .min()
        .ok_or(BenchmarkError::EmptyScenarioSet)?;
    let evaluations = upper
        .checked_mul(scenarios.len() as u64)
        .ok_or(BenchmarkError::ArithmeticOverflow)?;
    if evaluations > MAX_ROBUST_EVALUATIONS {
        return Err(BenchmarkError::RobustSearchTooLarge {
            evaluations,
            maximum: MAX_ROBUST_EVALUATIONS,
        });
    }

    let mut best = RobustPlan {
        transfer_input_kwh: 0,
        scenario_plans: vec![Plan::default(); scenarios.len()],
        worst_case_surplus_milli: 0,
        total_surplus_milli: 0,
    };

    for input_kwh in 1..=upper {
        let mut candidate_plans = Vec::with_capacity(scenarios.len());
        let mut worst_case = i128::MAX;
        let mut total = 0_i128;
        let mut feasible_everywhere = true;

        for scenario in scenarios {
            match assess_candidate(scenario, input_kwh)? {
                Some(plan) => {
                    worst_case = worst_case.min(plan.network_surplus_milli);
                    total = total
                        .checked_add(plan.network_surplus_milli)
                        .ok_or(BenchmarkError::ArithmeticOverflow)?;
                    candidate_plans.push(plan);
                }
                None => {
                    feasible_everywhere = false;
                    break;
                }
            }
        }
        if !feasible_everywhere {
            continue;
        }

        let candidate = RobustPlan {
            transfer_input_kwh: input_kwh,
            scenario_plans: candidate_plans,
            worst_case_surplus_milli: worst_case,
            total_surplus_milli: total,
        };
        let better = candidate.worst_case_surplus_milli > best.worst_case_surplus_milli
            || (candidate.worst_case_surplus_milli == best.worst_case_surplus_milli
                && candidate.worst_case_surplus_milli > 0
                && (candidate.total_surplus_milli > best.total_surplus_milli
                    || (candidate.total_surplus_milli == best.total_surplus_milli
                        && candidate.transfer_input_kwh < best.transfer_input_kwh)));
        if better {
            best = candidate;
        }
    }

    verify_robust_plan(scenarios, &best)?;
    Ok(best)
}

/// Audit the robust plan, recompute every scenario's ledgers, and independently
/// rescan the bounded common-decision domain. A single bad scenario invalidates
/// a nonzero exchange plan.
pub fn verify_robust_plan(scenarios: &[Scenario], plan: &RobustPlan) -> Result<(), BenchmarkError> {
    validate_scenario_set(scenarios)?;
    if plan.scenario_plans.len() != scenarios.len() {
        return Err(BenchmarkError::AccountingMismatch(
            "robust plan scenario count does not match the input set",
        ));
    }
    let upper = scenarios
        .iter()
        .map(Scenario::maximum_transfer_kwh)
        .min()
        .ok_or(BenchmarkError::EmptyScenarioSet)?;
    if plan.transfer_input_kwh > upper {
        return Err(BenchmarkError::InfeasiblePlan(
            "common transfer exceeds at least one scenario's source or link capacity",
        ));
    }
    let evaluations = upper
        .checked_mul(scenarios.len() as u64)
        .ok_or(BenchmarkError::ArithmeticOverflow)?;
    if evaluations > MAX_ROBUST_EVALUATIONS {
        return Err(BenchmarkError::RobustSearchTooLarge {
            evaluations,
            maximum: MAX_ROBUST_EVALUATIONS,
        });
    }

    let mut observed_worst = i128::MAX;
    let mut observed_total = 0_i128;
    for (scenario, outcome) in scenarios.iter().zip(&plan.scenario_plans) {
        if outcome.transfer_input_kwh != plan.transfer_input_kwh {
            return Err(BenchmarkError::InfeasiblePlan(
                "scenario outcomes do not share the chosen transfer quantity",
            ));
        }
        verify_plan(scenario, outcome)?;
        observed_worst = observed_worst.min(outcome.network_surplus_milli);
        observed_total = observed_total
            .checked_add(outcome.network_surplus_milli)
            .ok_or(BenchmarkError::ArithmeticOverflow)?;
    }
    if observed_worst != plan.worst_case_surplus_milli || observed_total != plan.total_surplus_milli
    {
        return Err(BenchmarkError::AccountingMismatch(
            "robust aggregate metrics do not match the per-scenario outcomes",
        ));
    }
    if plan.transfer_input_kwh > 0 && plan.worst_case_surplus_milli <= 0 {
        return Err(BenchmarkError::NonOptimalPlan);
    }

    // Independent finite-domain rescan: do not call assess_candidate here.
    // Recompute feasibility and surplus from the scenario primitives.
    for input_kwh in 1..=upper {
        let mut candidate_worst = i128::MAX;
        let mut candidate_total = 0_i128;
        let mut feasible_everywhere = true;
        for scenario in scenarios {
            if !scenario.temperature_grade_is_compatible()
                || input_kwh > scenario.source_surplus_kwh
                || input_kwh > scenario.link_capacity_kwh
            {
                feasible_everywhere = false;
                break;
            }
            let delivered = input_kwh
                .checked_mul(scenario.transfer_efficiency_bps as u64)
                .ok_or(BenchmarkError::ArithmeticOverflow)?
                / BASIS_POINTS;
            if delivered == 0 || delivered > scenario.sink_heat_demand_kwh {
                feasible_everywhere = false;
                break;
            }
            let avoided = checked_cost(delivered, scenario.avoided_utility_cost_milli_per_kwh)?;
            let source_cost =
                checked_cost(input_kwh, scenario.source_opportunity_cost_milli_per_kwh)?;
            let variable_cost = checked_cost(
                input_kwh,
                scenario.transfer_variable_cost_milli_per_input_kwh,
            )?;
            let real_cost = source_cost
                .checked_add(variable_cost)
                .and_then(|value| value.checked_add(scenario.coordination_fixed_cost_milli))
                .ok_or(BenchmarkError::ArithmeticOverflow)?;
            let surplus = (avoided as i128) - (real_cost as i128);
            let reservation = scenario
                .source_minimum_participation_surplus_milli
                .checked_add(scenario.sink_minimum_participation_surplus_milli)
                .ok_or(BenchmarkError::ArithmeticOverflow)?;
            if surplus < reservation as i128 {
                feasible_everywhere = false;
                break;
            }
            candidate_worst = candidate_worst.min(surplus);
            candidate_total = candidate_total
                .checked_add(surplus)
                .ok_or(BenchmarkError::ArithmeticOverflow)?;
        }
        if !feasible_everywhere {
            continue;
        }

        let better = candidate_worst > plan.worst_case_surplus_milli
            || (candidate_worst == plan.worst_case_surplus_milli
                && candidate_worst > 0
                && (candidate_total > plan.total_surplus_milli
                    || (candidate_total == plan.total_surplus_milli
                        && input_kwh < plan.transfer_input_kwh)));
        if better {
            return Err(BenchmarkError::NonOptimalPlan);
        }
    }
    Ok(())
}

fn validate_scenario_set(scenarios: &[Scenario]) -> Result<(), BenchmarkError> {
    if scenarios.is_empty() {
        return Err(BenchmarkError::EmptyScenarioSet);
    }
    if scenarios.len() > MAX_ROBUST_SCENARIOS {
        return Err(BenchmarkError::TooManyScenarios {
            count: scenarios.len(),
            maximum: MAX_ROBUST_SCENARIOS,
        });
    }
    let mut ids = BTreeSet::new();
    for scenario in scenarios {
        scenario.validate()?;
        if !ids.insert(scenario.scenario_id.as_str()) {
            return Err(BenchmarkError::DuplicateScenarioId(
                scenario.scenario_id.clone(),
            ));
        }
    }
    Ok(())
}

fn checked_cost(kwh: u64, milli_credits_per_kwh: u64) -> Result<u64, BenchmarkError> {
    kwh.checked_mul(milli_credits_per_kwh)
        .ok_or(BenchmarkError::ArithmeticOverflow)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scenario() -> Scenario {
        Scenario {
            scenario_id: "IC-001-POSITIVE-SYNTHETIC".to_owned(),
            source_surplus_kwh: 60,
            sink_heat_demand_kwh: 40,
            link_capacity_kwh: 60,
            transfer_efficiency_bps: 9_000,
            source_supply_temp_c: 180,
            sink_required_temp_c: 120,
            minimum_approach_c: 20,
            avoided_utility_cost_milli_per_kwh: 2_500,
            source_opportunity_cost_milli_per_kwh: 100,
            transfer_variable_cost_milli_per_input_kwh: 200,
            coordination_fixed_cost_milli: 1_000,
            source_surplus_share_bps: 5_000,
            source_minimum_participation_surplus_milli: 10_000,
            sink_minimum_participation_surplus_milli: 20_000,
        }
    }

    #[test]
    fn positive_case_finds_maximum_value_feasible_transfer() {
        let s = scenario();
        let p = evaluate(&s).expect("scenario evaluates");
        assert_eq!(p.transfer_input_kwh, 45);
        assert_eq!(p.heat_delivered_kwh, 40);
        assert_eq!(p.transfer_loss_kwh, 5);
        assert_eq!(p.utility_cost_avoided_milli, 100_000);
        assert_eq!(p.source_opportunity_cost_milli, 4_500);
        assert_eq!(p.transfer_variable_cost_milli, 9_000);
        assert_eq!(p.coordination_fixed_cost_milli, 1_000);
        assert_eq!(p.network_surplus_milli, 85_500);
        assert_eq!(p.source_surplus_allocation_milli, 37_750);
        assert_eq!(p.sink_surplus_allocation_milli, 47_750);
        verify_optimal_plan(&s, &p).expect("exact plan audits");
    }

    #[test]
    fn grade_incompatibility_keeps_independent_baseline() {
        let mut s = scenario();
        s.scenario_id = "IC-001-NEGATIVE-GRADE-SYNTHETIC".to_owned();
        s.source_supply_temp_c = 130;
        let p = evaluate(&s).expect("incompatible grade is a valid negative control");
        assert_eq!(p, Plan::default());
        verify_plan(&s, &p).expect("zero exchange is valid");
    }

    #[test]
    fn high_transaction_cost_keeps_independent_baseline() {
        let mut s = scenario();
        s.scenario_id = "IC-001-NEGATIVE-ECONOMICS-SYNTHETIC".to_owned();
        s.transfer_variable_cost_milli_per_input_kwh = 3_000;
        let p = evaluate(&s).expect("uneconomic coordination is a valid negative control");
        assert_eq!(p, Plan::default());
    }

    #[test]
    fn zero_capacity_keeps_independent_baseline() {
        let mut s = scenario();
        s.link_capacity_kwh = 0;
        let p = evaluate(&s).expect("zero capacity is valid");
        assert_eq!(p, Plan::default());
    }

    #[test]
    fn verifier_rejects_tampered_delivery() {
        let s = scenario();
        let mut p = evaluate(&s).expect("scenario evaluates");
        p.heat_delivered_kwh += 1;
        assert!(matches!(
            verify_plan(&s, &p),
            Err(BenchmarkError::AccountingMismatch(_))
        ));
    }

    #[test]
    fn verifier_rejects_tampered_surplus() {
        let s = scenario();
        let mut p = evaluate(&s).expect("scenario evaluates");
        p.network_surplus_milli += 1;
        assert!(matches!(
            verify_plan(&s, &p),
            Err(BenchmarkError::AccountingMismatch(_))
        ));
    }

    #[test]
    fn verifier_rejects_physically_incompatible_exchange() {
        let mut s = scenario();
        s.source_supply_temp_c = 120;
        let mut p = evaluate(&s).expect("evaluator falls back to independent baseline");
        p.transfer_input_kwh = 1;
        assert!(matches!(
            verify_plan(&s, &p),
            Err(BenchmarkError::InfeasiblePlan(_))
        ));
    }

    #[test]
    fn invalid_share_is_rejected() {
        let mut s = scenario();
        s.source_surplus_share_bps = 10_001;
        assert!(matches!(
            evaluate(&s),
            Err(BenchmarkError::InvalidScenario(_))
        ));
    }

    #[test]
    fn search_bound_is_fail_closed() {
        let mut s = scenario();
        s.source_surplus_kwh = MAX_EXHAUSTIVE_TRANSFER_KWH + 1;
        s.link_capacity_kwh = MAX_EXHAUSTIVE_TRANSFER_KWH + 1;
        assert!(matches!(
            evaluate(&s),
            Err(BenchmarkError::SearchSpaceTooLarge { .. })
        ));
    }

    #[test]
    fn participant_reservation_floors_can_veto_cooperation() {
        let mut s = scenario();
        s.source_minimum_participation_surplus_milli = 50_000;
        s.sink_minimum_participation_surplus_milli = 40_000;
        let p = evaluate(&s).expect("unacceptable cooperation falls back cleanly");
        assert_eq!(p, Plan::default());
    }

    #[test]
    fn participant_allocations_meet_both_reservation_floors() {
        let s = scenario();
        let p = evaluate(&s).expect("scenario evaluates");
        assert!(p.source_surplus_allocation_milli >= s.source_minimum_participation_surplus_milli);
        assert!(p.sink_surplus_allocation_milli >= s.sink_minimum_participation_surplus_milli);
        assert_eq!(
            p.source_surplus_allocation_milli + p.sink_surplus_allocation_milli,
            p.network_surplus_milli as u64
        );
    }

    #[test]
    fn optimality_audit_prefers_no_exchange_on_zero_surplus_tie() {
        let mut s = scenario();
        s.transfer_efficiency_bps = 10_000;
        s.avoided_utility_cost_milli_per_kwh = 1_000;
        s.source_opportunity_cost_milli_per_kwh = 0;
        s.transfer_variable_cost_milli_per_input_kwh = 1_000;
        s.coordination_fixed_cost_milli = 0;
        s.source_minimum_participation_surplus_milli = 0;
        s.sink_minimum_participation_surplus_milli = 0;
        let tied_but_wasteful = Plan {
            transfer_input_kwh: 1,
            heat_delivered_kwh: 1,
            transfer_loss_kwh: 0,
            utility_cost_avoided_milli: 1_000,
            source_opportunity_cost_milli: 0,
            transfer_variable_cost_milli: 1_000,
            coordination_fixed_cost_milli: 0,
            network_surplus_milli: 0,
            source_surplus_allocation_milli: 0,
            sink_surplus_allocation_milli: 0,
        };
        assert!(matches!(
            verify_optimal_plan(&s, &tied_but_wasteful),
            Err(BenchmarkError::NonOptimalPlan)
        ));
    }

    fn conservative_scenario() -> Scenario {
        let mut s = scenario();
        s.scenario_id = "IC-001-CONSERVATIVE-SYNTHETIC".to_owned();
        s.sink_heat_demand_kwh = 35;
        s.transfer_efficiency_bps = 8_500;
        s.source_opportunity_cost_milli_per_kwh = 150;
        s.transfer_variable_cost_milli_per_input_kwh = 250;
        s.coordination_fixed_cost_milli = 2_000;
        s.avoided_utility_cost_milli_per_kwh = 1_800;
        s
    }

    #[test]
    fn robust_plan_optimizes_worst_case_across_scenarios() {
        let scenarios = vec![scenario(), conservative_scenario()];
        let plan = evaluate_robust(&scenarios).expect("scenario set evaluates");
        assert_eq!(plan.transfer_input_kwh, 42);
        assert_eq!(plan.scenario_plans.len(), 2);
        assert_eq!(plan.scenario_plans[0].network_surplus_milli, 78_900);
        assert_eq!(plan.scenario_plans[1].network_surplus_milli, 44_200);
        assert_eq!(plan.worst_case_surplus_milli, 44_200);
        assert_eq!(plan.total_surplus_milli, 123_100);
        verify_robust_plan(&scenarios, &plan).expect("independent audit passes");
    }

    #[test]
    fn robust_adverse_scenario_falls_back_to_no_exchange() {
        let mut adverse = conservative_scenario();
        adverse.scenario_id = "IC-001-ADVERSE-PRICE-SYNTHETIC".to_owned();
        adverse.avoided_utility_cost_milli_per_kwh = 100;
        adverse.coordination_fixed_cost_milli = 50_000;
        let scenarios = vec![scenario(), adverse];
        let plan = evaluate_robust(&scenarios).expect("adverse set falls back safely");
        assert_eq!(plan.transfer_input_kwh, 0);
        assert_eq!(plan.worst_case_surplus_milli, 0);
        assert_eq!(plan.total_surplus_milli, 0);
    }

    #[test]
    fn robust_auditor_rejects_tampered_worst_case_metric() {
        let scenarios = vec![scenario(), conservative_scenario()];
        let mut plan = evaluate_robust(&scenarios).expect("scenario set evaluates");
        plan.worst_case_surplus_milli += 1;
        assert!(matches!(
            verify_robust_plan(&scenarios, &plan),
            Err(BenchmarkError::AccountingMismatch(_))
        ));
    }

    #[test]
    fn robust_evaluation_rejects_empty_and_duplicate_scenario_sets() {
        assert!(matches!(
            evaluate_robust(&[]),
            Err(BenchmarkError::EmptyScenarioSet)
        ));
        let s = scenario();
        let duplicate = vec![s.clone(), s];
        assert!(matches!(
            evaluate_robust(&duplicate),
            Err(BenchmarkError::DuplicateScenarioId(_))
        ));
    }

    #[test]
    fn identical_fixture_is_deterministic() {
        let s = scenario();
        assert_eq!(evaluate(&s).unwrap(), evaluate(&s).unwrap());
    }
}
