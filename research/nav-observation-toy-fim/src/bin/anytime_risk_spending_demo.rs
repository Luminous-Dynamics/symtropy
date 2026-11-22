use nav_observation_toy_fim::anytime_risk_spending::{
    allocate_telescoping_anytime_risk, sum_telescoping_anytime_prefix,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mission_budget = 1_000_000_000_000_u64;
    println!("mission_budget_quanta={mission_budget}");
    for index in 1..=5 {
        let share = allocate_telescoping_anytime_risk(mission_budget, index)?;
        println!("check_index={} allocated_quanta={} denominator={}", share.check_index, share.allocated_quanta, share.denominator);
    }
    for count in [10, 100, 1_000] {
        let spent = sum_telescoping_anytime_prefix(mission_budget, count)?;
        println!("prefix_checks={count} allocated_quanta={spent} remaining_quanta={}", mission_budget - spent);
    }
    println!("scope=discrete_per_check_false_alarm_budget_union_bound_not_continuous_time_coverage_or_an_e_process");
    Ok(())
}
