use safrb_tournament::{ScenarioOutcome, TournamentMetrics, ZoneOutcome};

fn main() -> Result<(), String> {
    let scenarios = vec![
        ScenarioOutcome {
            scenario_id: "normal".into(),
            weight: 95,
            total_unserved_l: 0,
            total_critical_demand_l: 100,
            total_critical_delivered_l: 100,
            total_critical_unserved_l: 0,
            logistics_cost_units: 100,
            zones: vec![ZoneOutcome {
                zone_id: "inland-critical".into(),
                critical_demand_l: 100,
                critical_delivered_l: 100,
                critical_unserved_l: 0,
            }],
        },
        ScenarioOutcome {
            scenario_id: "supplier-port-compound".into(),
            weight: 5,
            total_unserved_l: 100,
            total_critical_demand_l: 100,
            total_critical_delivered_l: 0,
            total_critical_unserved_l: 100,
            logistics_cost_units: 120,
            zones: vec![ZoneOutcome {
                zone_id: "inland-critical".into(),
                critical_demand_l: 100,
                critical_delivered_l: 0,
                critical_unserved_l: 100,
            }],
        },
    ];
    let metrics = TournamentMetrics::evaluate(&scenarios, 9_500)?;
    println!("metric,value");
    println!("scenario_count,{}", metrics.scenario_count);
    println!(
        "expected_critical_unserved_l_ceil,{}",
        metrics.expected_critical_unserved_l_ceil
    );
    println!(
        "critical_service_rate_bps,{}",
        metrics.critical_service_rate_bps
    );
    println!(
        "var95_critical_unserved_l,{}",
        metrics.var_critical_unserved_l
    );
    println!(
        "cvar95_critical_unserved_l_ceil,{}",
        metrics.cvar_critical_unserved_l_ceil
    );
    println!(
        "worst_critical_unserved_l,{}",
        metrics.worst_critical_unserved_l
    );
    println!(
        "weighted_logistics_cost_units_ceil,{}",
        metrics.weighted_logistics_cost_units_ceil
    );
    Ok(())
}
