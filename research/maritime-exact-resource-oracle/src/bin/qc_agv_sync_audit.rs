use maritime_exact_resource_oracle::sync::{
    evaluate_sync_tournament, SyncInstance, SyncRoute, SyncTask,
};
use std::collections::BTreeMap;
use std::error::Error;

const FIXTURE: &str = include_str!("../../fixtures/qc-agv-sync.tsv");

fn parse_u32(value: &str, field: &str) -> Result<u32, Box<dyn Error>> {
    value
        .parse::<u32>()
        .map_err(|error| format!("invalid {field}: {error}").into())
}

fn parse_usize(value: &str, field: &str) -> Result<usize, Box<dyn Error>> {
    value
        .parse::<usize>()
        .map_err(|error| format!("invalid {field}: {error}").into())
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut instance_id = None;
    let mut gamma = None;
    let mut qc_wait_weight = None;
    let mut agv_wait_weight = None;
    let mut tasks = Vec::new();
    let mut routes: BTreeMap<String, Vec<(String, u32)>> = BTreeMap::new();

    for (line_number, raw_line) in FIXTURE.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        match fields.first().copied() {
            Some("INSTANCE") if fields.len() == 5 => {
                if instance_id.is_some() {
                    return Err("duplicate INSTANCE header".into());
                }
                instance_id = Some(fields[1].to_owned());
                gamma = Some(parse_usize(fields[2], "gamma")?);
                qc_wait_weight = Some(u64::from(parse_u32(fields[3], "qc_wait_weight")?));
                agv_wait_weight = Some(u64::from(parse_u32(fields[4], "agv_wait_weight")?));
            }
            Some("TASK") if fields.len() == 6 => {
                tasks.push(SyncTask {
                    task_id: fields[1].to_owned(),
                    qc_id: fields[2].to_owned(),
                    nominal_service: parse_u32(fields[3], "nominal_service")?,
                    service_deviation: parse_u32(fields[4], "service_deviation")?,
                    qc_predecessor: if fields[5] == "-" {
                        None
                    } else {
                        Some(fields[5].to_owned())
                    },
                });
            }
            Some("ROUTE") if fields.len() == 2 => {
                if routes.insert(fields[1].to_owned(), Vec::new()).is_some() {
                    return Err(format!("line {}: duplicate route {}", line_number + 1, fields[1]).into());
                }
            }
            Some("STEP") if fields.len() == 4 => {
                let route = routes.get_mut(fields[1])
                    .ok_or_else(|| format!("line {}: STEP before ROUTE", line_number + 1))?;
                route.push((fields[2].to_owned(), parse_u32(fields[3], "travel_before")?));
            }
            Some(kind) => {
                return Err(format!("line {}: malformed record {kind}", line_number + 1).into());
            }
            None => return Err(format!("line {}: empty row", line_number + 1).into()),
        }
    }

    let mut route_values = Vec::new();
    for (route_id, steps) in routes {
        if steps.is_empty() {
            return Err(format!("route {route_id} has no steps").into());
        }
        let task_order = steps.iter().map(|(task_id, _)| task_id.clone()).collect();
        let initial_travel = steps[0].1;
        let inter_task_travel = steps.iter().skip(1).map(|(_, travel)| *travel).collect();
        route_values.push(SyncRoute {
            route_id,
            task_order,
            initial_travel,
            inter_task_travel,
        });
    }

    let instance = SyncInstance {
        instance_id: instance_id.ok_or("missing INSTANCE header")?,
        tasks,
        routes: route_values,
        qc_wait_weight: qc_wait_weight.ok_or("missing QC wait weight")?,
        agv_wait_weight: agv_wait_weight.ok_or("missing AGV wait weight")?,
        gamma: gamma.ok_or("missing Gamma")?,
    };

    let result = evaluate_sync_tournament(&instance)?;
    println!("QC_AGV_SYNC_RECEIPT\t0.1");
    println!(
        "INSTANCE\t{}\t{}\t{}",
        result.instance_id,
        result.gamma,
        result.uncertainty_scenario_count
    );
    for route in &result.routes {
        let witness = route
            .worst_case
            .scenario
            .iter()
            .map(i8::to_string)
            .collect::<Vec<_>>()
            .join(",");
        println!(
            "ROUTE\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            route.route_id,
            route.nominal.weighted_wait_cost,
            route.worst_case.weighted_wait_cost,
            route.worst_case.total_qc_wait,
            route.worst_case.total_agv_wait,
            route.worst_case.makespan,
            witness,
            route.nominal.makespan
        );
    }
    println!("NOMINAL_BEST\t{}", result.nominal_best_route);
    println!("ROBUST_BEST\t{}", result.robust_best_route);
    println!("SUMMARY\tPASS");
    Ok(())
}
