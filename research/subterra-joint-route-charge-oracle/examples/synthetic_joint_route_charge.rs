use subterra_joint_route_charge_oracle::{
    ChargingStation, Edge, InspectionTask, PlanLimits, RobotSpec, Scenario, SolveDisposition, solve,
};

fn main() {
    let scenario = Scenario {
        scenario_id: "synthetic-two-robot-charging-corridor".into(),
        node_count: 4,
        edges: vec![
            Edge { id: "depot-a".into(), from_node: 0, to_node: 1, distance_mm: 1_000, energy_units: 3, duration_ticks: 1, concurrent_capacity: 1 },
            Edge { id: "a-depot".into(), from_node: 1, to_node: 0, distance_mm: 1_000, energy_units: 3, duration_ticks: 1, concurrent_capacity: 1 },
            Edge { id: "depot-b".into(), from_node: 0, to_node: 2, distance_mm: 1_500, energy_units: 4, duration_ticks: 2, concurrent_capacity: 1 },
            Edge { id: "b-depot".into(), from_node: 2, to_node: 0, distance_mm: 1_500, energy_units: 4, duration_ticks: 2, concurrent_capacity: 1 },
            Edge { id: "depot-c".into(), from_node: 0, to_node: 3, distance_mm: 1_800, energy_units: 4, duration_ticks: 2, concurrent_capacity: 1 },
            Edge { id: "c-depot".into(), from_node: 3, to_node: 0, distance_mm: 1_800, energy_units: 4, duration_ticks: 2, concurrent_capacity: 1 },
            Edge { id: "a-b".into(), from_node: 1, to_node: 2, distance_mm: 1_000, energy_units: 2, duration_ticks: 1, concurrent_capacity: 1 },
            Edge { id: "b-a".into(), from_node: 2, to_node: 1, distance_mm: 1_000, energy_units: 2, duration_ticks: 1, concurrent_capacity: 1 },
        ],
        tasks: vec![
            InspectionTask { id: "thermal-a".into(), node: 1, mandatory: true, required_capability_mask: 1, service_duration_ticks: 1, service_energy_per_tick: 1 },
            InspectionTask { id: "acoustic-b".into(), node: 2, mandatory: true, required_capability_mask: 2, service_duration_ticks: 2, service_energy_per_tick: 1 },
            InspectionTask { id: "lidar-c".into(), node: 3, mandatory: true, required_capability_mask: 4, service_duration_ticks: 1, service_energy_per_tick: 1 },
        ],
        charging_stations: vec![
            ChargingStation { id: "depot-fast-charge".into(), node: 0, charge_rate_units_per_tick: 2, concurrent_capacity: 1 },
            ChargingStation { id: "mid-corridor-charge".into(), node: 2, charge_rate_units_per_tick: 1, concurrent_capacity: 1 },
        ],
        robots: vec![
            RobotSpec { id: "robot-thermal".into(), start_node: 0, return_node: 0, capability_mask: 1, battery_capacity_units: 12, initial_battery_units: 8, minimum_return_reserve_units: 2, mission_interruption_upper_ppm: 100, robot_loss_upper_ppm: 20 },
            RobotSpec { id: "robot-acoustic-lidar".into(), start_node: 0, return_node: 0, capability_mask: 6, battery_capacity_units: 14, initial_battery_units: 10, minimum_return_reserve_units: 2, mission_interruption_upper_ppm: 150, robot_loss_upper_ppm: 30 },
        ],
        limits: PlanLimits {
            maximum_elapsed_ticks: 30,
            maximum_distance_mm: 20_000,
            maximum_battery_energy_used_units: 100,
            maximum_grid_energy_charged_units: 50,
            mission_interruption_upper_ppm: 300,
            robot_loss_upper_ppm: 100,
            maximum_search_nodes: 250_000,
        },
    };

    let report = solve(&scenario).expect("valid tiny synthetic mission");
    println!("scenario={}", scenario.scenario_id);
    println!("disposition={:?}", report.disposition);
    println!("expanded_labels={}", report.expanded_labels);
    println!("pareto_frontier_count={}", report.pareto_frontier.len());
    for (index, plan) in report.pareto_frontier.iter().enumerate() {
        println!("plan_{}_ticks={}", index, plan.metrics.elapsed_ticks);
        println!("plan_{}_distance_mm={}", index, plan.metrics.distance_mm);
        println!("plan_{}_battery_energy_used={}", index, plan.metrics.battery_energy_used_units);
        println!("plan_{}_grid_energy_charged={}", index, plan.metrics.grid_energy_charged_units);
        println!("plan_{}_actions={}", index, plan.action_trace.len());
    }
    println!("joint_allocation_route_charge_search=true");
    println!("synthetic_fixture=true; no physical energy or safety qualification");
    assert_eq!(report.disposition, SolveDisposition::ExactParetoFrontierProven);
}
