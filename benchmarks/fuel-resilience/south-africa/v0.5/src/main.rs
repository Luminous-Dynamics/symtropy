use safrb_service::base_case;

fn main() -> Result<(), String> {
    let result = base_case().simulate()?;
    println!("metric,value,unit");
    println!("scenario_id,{},label", result.scenario_id);
    println!(
        "qualified_fuel_receipts,{},L",
        result.qualified_fuel_receipts_l
    );
    println!("fuel_consumed,{},L", result.fuel_used_l);
    println!(
        "closing_fuel_inventory,{},L",
        result.closing_fuel_inventory_l
    );
    println!("chemical_input_lower,{},Wh", result.chemical_input_lower_wh);
    println!("chemical_input_upper,{},Wh", result.chemical_input_upper_wh);
    println!("net_generation_lower,{},Wh", result.net_generation_lower_wh);
    println!("net_generation_upper,{},Wh", result.net_generation_upper_wh);
    println!("service_served_lower,{},Wh", result.served_lower_wh);
    println!("service_served_upper,{},Wh", result.served_upper_wh);
    println!("service_unserved_lower,{},Wh", result.unserved_lower_wh);
    println!("service_unserved_upper,{},Wh", result.unserved_upper_wh);
    println!("# Synthetic fixture only; this is not a measured generator performance result.");
    Ok(())
}
