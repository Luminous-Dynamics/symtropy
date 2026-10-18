//! Tiny exact-enumeration industrial heat-exchange benchmark.
//!
//! This is a discrete, constant-temperature screening model for validating
//! coordination economics and ledgers. It is not a pinch solver, dynamic
//! process simulator, or plant-control system.

#![forbid(unsafe_code)]

use std::fmt;

pub const MAX_EXHAUSTIVE_TRANSFER_KWH: u64 = 100_000;
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BenchmarkError {
    InvalidScenario(&'static str),
    ArithmeticOverflow,
    SearchSpaceTooLarge { bound_kwh: u64, maximum_kwh: u64 },
    InfeasiblePlan(&'static str),
    AccountingMismatch(&'static str),
    NonOptimalPlan,
}

impl fmt::Display for BenchmarkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidScenario(message) => write!(f, "invalid scenario: {message}"),
            Self::ArithmeticOverflow => write!(f, "integer ledger arithmetic overflow"),
            Self::SearchSpaceTooLarge { bound_kwh, maximum_kwh } => write!(
                f,
                "exact search bound {bound_kwh} kWh exceeds configured maximum {maximum_kwh} kWh"
            ),
            Self::InfeasiblePlan(message) => write!(f, "infeasible plan: {message}"),
            Self::AccountingMismatch(message) => write!(f, "accounting mismatch: {message}"),
            Self::NonOptimalPlan => write!(f, "plan is feasible but not optimal for this exact case"),
        }
    }
}

impl std::error::Error for BenchmarkError {}

impl Scenario {
    pub fn validate(&self) -> Result<(), BenchmarkError> {
        if self.scenario_id.trim().is_empty() {
            return Err(BenchmarkError::InvalidScenario("scenario_id must not be empty"));
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
    let reserved = s.source_minimum_participation_surplus_milli
        .checked_add(s.sink_minimum_participation_surplus_milli)
        .ok_or(BenchmarkError::ArithmeticOverflow)?;
    if surplus < reserved as i128 {
        return Ok(None);
    }
    let remaining = distributable - reserved;
    let source_share = s.source_minimum_participation_surplus_milli
        + (((remaining as u128) * (s.source_surplus_share_bps as u128)
            / (BASIS_POINTS as u128)) as u64);
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
    let source_cost = checked_cost(p.transfer_input_kwh, s.source_opportunity_cost_milli_per_kwh)?;
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
    let reserved = s.source_minimum_participation_surplus_milli
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
        + (((remaining as u128) * (s.source_surplus_share_bps as u128)
            / (BASIS_POINTS as u128)) as u64);
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
    let upper = s.maximum_transfer_kwh();
    if upper > MAX_EXHAUSTIVE_TRANSFER_KWH {
        return Err(BenchmarkError::SearchSpaceTooLarge {
            bound_kwh: upper,
            maximum_kwh: MAX_EXHAUSTIVE_TRANSFER_KWH,
        });
    }
    let reservation_total = s.source_minimum_participation_surplus_milli
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
        assert!(matches!(evaluate(&s), Err(BenchmarkError::InvalidScenario(_))));
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
    fn identical_fixture_is_deterministic() {
        let s = scenario();
        assert_eq!(evaluate(&s).unwrap(), evaluate(&s).unwrap());
    }
}
