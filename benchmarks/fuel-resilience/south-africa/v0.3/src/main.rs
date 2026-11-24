use safrb_planner::base_case;

fn main() -> Result<(), String> {
    let result = base_case().solve()?;
    println!("# SAFRB v0.3 — synthetic min-cost-flow allocation baseline");
    println!(
        "scenario,grade,requested_l,delivered_l,unserved_l,unallocated_eligible_supply_l,excluded_supply_l"
    );
    println!(
        "{},{:?},{},{},{},{},{}",
        result.scenario_id,
        result.grade,
        result.total_requested_l,
        result.total_delivered_l,
        result.total_unserved_l,
        result.unallocated_eligible_supply_l,
        result.excluded_supply_l
    );
    println!(
        "# {} route dispatches; one product grade per solve.",
        result.dispatches.len()
    );
    Ok(())
}
