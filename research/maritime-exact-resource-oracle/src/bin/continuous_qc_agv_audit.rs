use maritime_exact_resource_oracle::continuous::{
    evaluate_continuous_tournament, CandidatePlan, ContinuousInstance, ContinuousTask,
    TaskDirection, VehicleRoute,
};
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;

const FIXTURE: &str = include_str!("../../fixtures/continuous-qc-agv.tsv");

fn parse_f64(value: &str, field: &str) -> Result<f64, Box<dyn Error>> {
    value
        .parse::<f64>()
        .map_err(|error| format!("invalid {field}: {error}").into())
}

fn number(value: f64) -> String {
    format!("{value:.6}")
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut instance_id = None;
    let mut gamma = None;
    let mut qc_weight = None;
    let mut agv_weight = None;
    let mut tasks = Vec::new();
    let mut plan_ids = BTreeSet::new();
    let mut route_steps: BTreeMap<(String, String), Vec<(String, f64)>> = BTreeMap::new();

    for (line_number, raw_line) in FIXTURE.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        match fields.first().copied() {
            Some("INSTANCE") if fields.len() == 5 => {
                if instance_id.is_some() {
                    return Err("duplicate INSTANCE record".into());
                }
                instance_id = Some(fields[1].to_owned());
                gamma = Some(parse_f64(fields[2], "gamma")?);
                qc_weight = Some(parse_f64(fields[3], "qc_wait_weight")?);
                agv_weight = Some(parse_f64(fields[4], "agv_wait_weight")?);
            }
            Some("TASK") if fields.len() == 9 => {
                let direction = match fields[7] {
                    "LOADING" => TaskDirection::Loading,
                    "UNLOADING" => TaskDirection::Unloading,
                    other => return Err(format!("invalid task direction {other}").into()),
                };
                tasks.push(ContinuousTask {
                    task_id: fields[1].to_owned(),
                    qc_id: fields[2].to_owned(),
                    qc_predecessor: if fields[3] == "-" {
                        None
                    } else {
                        Some(fields[3].to_owned())
                    },
                    qc_release_at: parse_f64(fields[4], "qc_release_at")?,
                    nominal_service: parse_f64(fields[5], "nominal_service")?,
                    service_deviation: parse_f64(fields[6], "service_deviation")?,
                    direction,
                    loaded_delivery_after: parse_f64(fields[8], "loaded_delivery_after")?,
                });
            }
            Some("PLAN") if fields.len() == 2 => {
                if !plan_ids.insert(fields[1].to_owned()) {
                    return Err(format!("duplicate plan ID {}", fields[1]).into());
                }
            }
            Some("ROUTE") if fields.len() == 5 => {
                if !plan_ids.contains(fields[1]) {
                    return Err(format!(
                        "fixture line {} references unknown plan {}",
                        line_number + 1,
                        fields[1]
                    )
                    .into());
                }
                route_steps
                    .entry((fields[1].to_owned(), fields[2].to_owned()))
                    .or_default()
                    .push((
                        fields[3].to_owned(),
                        parse_f64(fields[4], "travel_before")?,
                    ));
            }
            Some(record_type) => {
                return Err(format!(
                    "fixture line {}: malformed or unknown record {record_type}",
                    line_number + 1
                )
                .into())
            }
            None => return Err(format!("fixture line {} is empty", line_number + 1).into()),
        }
    }

    let mut plans = Vec::new();
    for plan_id in &plan_ids {
        let mut routes = Vec::new();
        for ((route_plan, vehicle_id), steps) in &route_steps {
            if route_plan != plan_id {
                continue;
            }
            routes.push(VehicleRoute {
                vehicle_id: vehicle_id.clone(),
                task_order: steps.iter().map(|(task_id, _)| task_id.clone()).collect(),
                travel_before: steps.iter().map(|(_, travel)| *travel).collect(),
            });
        }
        plans.push(CandidatePlan {
            plan_id: plan_id.clone(),
            vehicle_routes: routes,
        });
    }

    let instance = ContinuousInstance {
        tasks,
        plans,
        qc_wait_weight: qc_weight.ok_or("missing INSTANCE qc_wait_weight")?,
        agv_wait_weight: agv_weight.ok_or("missing INSTANCE agv_wait_weight")?,
        gamma: gamma.ok_or("missing INSTANCE gamma")?,
    };
    let instance_id = instance_id.ok_or("missing INSTANCE record")?;
    let result = evaluate_continuous_tournament(&instance)?;

    println!("CONTINUOUS_QC_AGV_RECEIPT\t0.1");
    println!(
        "INSTANCE\t{}\t{}\t{}\t{}",
        instance_id,
        number(result.gamma),
        number(instance.qc_wait_weight),
        number(instance.agv_wait_weight)
    );
    for task in &instance.tasks {
        println!(
            "TASK_INPUT\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            task.task_id,
            task.qc_id,
            task.qc_predecessor.as_deref().unwrap_or("-"),
            number(task.qc_release_at),
            number(task.nominal_service),
            number(task.service_deviation),
            match task.direction {
                TaskDirection::Loading => "LOADING",
                TaskDirection::Unloading => "UNLOADING",
            },
            number(task.loaded_delivery_after)
        );
    }
    for plan in &instance.plans {
        println!("PLAN_INPUT\t{}", plan.plan_id);
        for route in &plan.vehicle_routes {
            for (index, task_id) in route.task_order.iter().enumerate() {
                println!(
                    "ROUTE_INPUT\t{}\t{}\t{}\t{}\t{}",
                    plan.plan_id,
                    route.vehicle_id,
                    index,
                    task_id,
                    number(route.travel_before[index])
                );
            }
        }
    }

    for plan in &result.plans {
        let witness = plan
            .witness
            .iter()
            .map(|(task_id, value)| format!("{task_id}={}", number(*value)))
            .collect::<Vec<_>>()
            .join(";");
        println!(
            "PLAN_RESULT\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            plan.plan_id,
            number(plan.nominal_cost),
            number(plan.worst_case_cost),
            number(plan.worst_case_qc_wait),
            number(plan.worst_case_agv_wait),
            number(plan.worst_case_makespan),
            witness
        );
        println!(
            "PLAN_ENUMERATION\t{}\t{}\t{}",
            plan.plan_id,
            plan.active_set_combinations_examined,
            plan.feasible_region_vertices
        );
        for event in &plan.worst_case_events {
            println!(
                "EVENT\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                plan.plan_id,
                event.task_id,
                number(event.qc_ready),
                number(event.agv_ready),
                number(event.service_start),
                number(event.service_finish),
                number(event.vehicle_release),
                number(event.qc_wait),
                number(event.agv_wait)
            );
        }
    }
    println!(
        "WINNERS\t{}\t{}",
        result.nominal_best_plan,
        result.robust_best_plan
    );
    println!("SUMMARY\tPASS\t{}/{}", result.plans.len(), result.plans.len());
    Ok(())
}
