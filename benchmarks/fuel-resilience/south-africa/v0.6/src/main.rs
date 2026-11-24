use safrb_service_portfolio::base_case;

fn main() -> Result<(), String> {
    let result = base_case().evaluate()?;
    println!("metric,value,unit");
    println!("scenario_id,{},label", result.scenario_id);
    println!(
        "qualified_source_lower,{},Wh",
        result.qualified_source_lower_wh
    );
    println!(
        "qualified_source_upper,{},Wh",
        result.qualified_source_upper_wh
    );
    println!(
        "excluded_source_upper,{},Wh",
        result.excluded_source_upper_wh
    );
    println!("service_demand,{},Wh", result.total_demand_wh);
    println!("served_lower,{},Wh", result.served_lower_wh);
    println!("served_upper,{},Wh", result.served_upper_wh);
    println!("unserved_lower,{},Wh", result.unserved_lower_wh);
    println!("unserved_upper,{},Wh", result.unserved_upper_wh);
    println!("curtailed_lower,{},Wh", result.curtailed_lower_wh);
    println!("curtailed_upper,{},Wh", result.curtailed_upper_wh);
    println!("# Multi-carrier portfolio fixture; not a physical port model or optimized dispatch.");
    Ok(())
}
