use safrb_network::{Grade, base_case};

fn main() -> Result<(), String> {
    let result = base_case().simulate()?;
    println!("# SAFRB v0.2 — synthetic three-node chain; not a South African calibrated network");
    println!(
        "scenario,opening_l,cargo_receipts_l,production_l,delivered_l,unserved_l,transit_loss_l,closing_nodes_l,closing_transit_l,network_residual_l"
    );
    println!(
        "{},{},{},{},{},{},{},{},{},{}",
        result.scenario_id,
        result.opening_inventory_l,
        result.accepted_cargo_receipts_l,
        result.domestic_production_l,
        result.delivered_demand_l,
        result.unserved_demand_l,
        result.transit_loss_l,
        result.closing_node_inventory_l,
        result.closing_in_transit_l,
        result.network_balance_residual_l
    );
    let d50 = Grade::D50;
    let _d500 = Grade::D500;
    println!("# Grade separation is enforced; demonstration route supports {d50:?} only.");
    Ok(())
}
