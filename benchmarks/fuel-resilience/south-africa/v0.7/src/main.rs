use safrb_capacity_access::{Disposition, base_case};

fn main() -> Result<(), String> {
    let result = base_case().evaluate()?;
    println!("metric,value,unit");
    println!("resource_id,{},label", result.resource_id);
    println!("installed_capacity,{},L", result.installed_capacity);
    println!("operable_capacity,{},L", result.operable_capacity);
    println!("entitlement_limit,{},L", result.entitlement_limit);
    println!("provider_offer_limit,{},L", result.provider_offer_limit);
    println!("physical_limit,{},L", result.physical_limit);
    println!(
        "access_limited_capacity,{},L",
        result.access_limited_capacity
    );
    println!("existing_consumed,{},L", result.existing_capacity_consumed);
    println!(
        "residual_schedulable,{},L",
        result.residual_schedulable_capacity
    );
    println!("candidate_requested,{},L", result.candidate_requested);
    println!("candidate_excess,{},L", result.candidate_excess);
    println!("disposition,{:?},label", result.disposition);
    if result.disposition == Disposition::FitsWithinEvidenceBoundedCapacity {
        println!(
            "# Feasibility only; no reservation, provider commitment or operational authorization was created."
        );
    }
    Ok(())
}
