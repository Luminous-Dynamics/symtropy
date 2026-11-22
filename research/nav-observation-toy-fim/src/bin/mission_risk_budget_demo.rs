use nav_observation_toy_fim::mission_risk_budget::allocate_equal_mission_risk;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mission_budget = 1.0e-3;
    let declared_checks = 785;
    let allocation = allocate_equal_mission_risk(mission_budget, declared_checks)?;
    println!("mission_risk_budget={mission_budget}");
    println!("declared_checks={declared_checks}");
    println!("per_check_risk_budget={:.12e}", allocation.per_check_risk_budgets[0]);
    println!("union_bound_allocated_risk={:.12e}", allocation.allocated_risk);
    println!("unallocated_risk={:.12e}", allocation.unallocated_risk);
    println!("scope=finite_declared_checks_union_bound_no_epoch_independence_assumption_no_continuous_time_claim");
    Ok(())
}
