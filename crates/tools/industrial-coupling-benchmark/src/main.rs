#![forbid(unsafe_code)]

use industrial_coupling_benchmark::{
    Scenario, evaluate, evaluate_robust, verify_optimal_plan, verify_robust_plan,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cases = [
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
        },
        Scenario {
            scenario_id: "IC-001-NEGATIVE-GRADE-SYNTHETIC".to_owned(),
            source_surplus_kwh: 60,
            sink_heat_demand_kwh: 40,
            link_capacity_kwh: 60,
            transfer_efficiency_bps: 9_000,
            source_supply_temp_c: 130,
            sink_required_temp_c: 120,
            minimum_approach_c: 20,
            avoided_utility_cost_milli_per_kwh: 2_500,
            source_opportunity_cost_milli_per_kwh: 100,
            transfer_variable_cost_milli_per_input_kwh: 200,
            coordination_fixed_cost_milli: 1_000,
            source_surplus_share_bps: 5_000,
            source_minimum_participation_surplus_milli: 10_000,
            sink_minimum_participation_surplus_milli: 20_000,
        },
        Scenario {
            scenario_id: "IC-001-NEGATIVE-ECONOMICS-SYNTHETIC".to_owned(),
            source_surplus_kwh: 60,
            sink_heat_demand_kwh: 40,
            link_capacity_kwh: 60,
            transfer_efficiency_bps: 9_000,
            source_supply_temp_c: 180,
            sink_required_temp_c: 120,
            minimum_approach_c: 20,
            avoided_utility_cost_milli_per_kwh: 2_500,
            source_opportunity_cost_milli_per_kwh: 100,
            transfer_variable_cost_milli_per_input_kwh: 3_000,
            coordination_fixed_cost_milli: 1_000,
            source_surplus_share_bps: 5_000,
            source_minimum_participation_surplus_milli: 10_000,
            sink_minimum_participation_surplus_milli: 20_000,
        },
    ];

    println!("INDUSTRIAL-COUPLING-001 — deterministic synthetic reference cases");
    println!("Energy: integer kWh-th | money: milli-credits | field performance: not measured");
    for case in cases {
        let plan = evaluate(&case)?;
        verify_optimal_plan(&case, &plan)?;
        println!(
            "scenario={} verified=pass input_kwh={} delivered_kwh={} loss_kwh={} net_surplus_milli={} source_alloc_milli={} sink_alloc_milli={}",
            case.scenario_id,
            plan.transfer_input_kwh,
            plan.heat_delivered_kwh,
            plan.transfer_loss_kwh,
            plan.network_surplus_milli,
            plan.source_surplus_allocation_milli,
            plan.sink_surplus_allocation_milli
        );
    }

    // A single transfer quantity must remain acceptable under both scenarios.
    // The objective maximizes the worst-case surplus, not the average.
    let conservative = Scenario {
        scenario_id: "IC-001-CONSERVATIVE-SYNTHETIC".to_owned(),
        source_surplus_kwh: 60,
        sink_heat_demand_kwh: 35,
        link_capacity_kwh: 60,
        transfer_efficiency_bps: 8_500,
        source_supply_temp_c: 180,
        sink_required_temp_c: 120,
        minimum_approach_c: 20,
        avoided_utility_cost_milli_per_kwh: 1_800,
        source_opportunity_cost_milli_per_kwh: 150,
        transfer_variable_cost_milli_per_input_kwh: 250,
        coordination_fixed_cost_milli: 2_000,
        source_surplus_share_bps: 5_000,
        source_minimum_participation_surplus_milli: 10_000,
        sink_minimum_participation_surplus_milli: 20_000,
    };
    let robust_scenarios = vec![
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
        },
        conservative,
    ];
    let robust = evaluate_robust(&robust_scenarios)?;
    verify_robust_plan(&robust_scenarios, &robust)?;
    println!(
        "robust_set={} scenarios={} verified=pass common_input_kwh={} worst_case_surplus_milli={} total_surplus_milli={}",
        "IC-001-TWO-SCENARIO-SYNTHETIC",
        robust.scenario_plans.len(),
        robust.transfer_input_kwh,
        robust.worst_case_surplus_milli,
        robust.total_surplus_milli
    );
    Ok(())
}
