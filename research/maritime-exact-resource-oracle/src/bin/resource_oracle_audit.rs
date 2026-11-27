use maritime_exact_resource_oracle::{
    enumerate, CandidateAction, ObjectiveVector, Reservation, ResourceCapacity, Scenario,
};
use std::collections::BTreeMap;
use std::error::Error;

const FIXTURE: &str = include_str!("../../fixtures/tiny-port.tsv");

fn parse_u32(value: &str, field: &str) -> Result<u32, Box<dyn Error>> {
    value
        .parse::<u32>()
        .map_err(|error| format!("invalid {field}: {error}").into())
}

fn parse_u64(value: &str, field: &str) -> Result<u64, Box<dyn Error>> {
    value
        .parse::<u64>()
        .map_err(|error| format!("invalid {field}: {error}").into())
}

fn objective_fields(fields: &[&str]) -> Result<ObjectiveVector, Box<dyn Error>> {
    if fields.len() != 5 {
        return Err(format!("objective requires 5 values, got {}", fields.len()).into());
    }
    Ok(ObjectiveVector {
        vessel_wait: parse_u64(fields[0], "vessel_wait")?,
        receiver_delay: parse_u64(fields[1], "receiver_delay")?,
        energy_cost: parse_u64(fields[2], "energy_cost")?,
        emissions: parse_u64(fields[3], "emissions")?,
        missed_connections: parse_u64(fields[4], "missed_connections")?,
    })
}

fn parse_scenario() -> Result<Scenario, Box<dyn Error>> {
    let mut vessel_ids = Vec::new();
    let mut capacities = Vec::new();
    let mut candidates: BTreeMap<String, CandidateAction> = BTreeMap::new();
    let mut uses = Vec::new();

    for (line_number, raw_line) in FIXTURE.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        match fields.first().copied() {
            Some("VESSEL") if fields.len() == 2 => vessel_ids.push(fields[1].to_owned()),
            Some("CAPACITY") if fields.len() == 4 => {
                capacities.push(ResourceCapacity {
                    resource_id: fields[1].to_owned(),
                    slot: parse_u32(fields[2], "capacity_slot")?,
                    capacity: parse_u32(fields[3], "capacity")?,
                });
            }
            Some("ACTION") if fields.len() == 8 => {
                let objective = objective_fields(&fields[3..8])?;
                let candidate = CandidateAction {
                    candidate_id: fields[1].to_owned(),
                    vessel_id: fields[2].to_owned(),
                    reservations: Vec::new(),
                    objective,
                };
                if candidates
                    .insert(candidate.candidate_id.clone(), candidate)
                    .is_some()
                {
                    return Err(format!(
                        "fixture line {}: duplicate candidate ID {}",
                        line_number + 1,
                        fields[1]
                    )
                    .into());
                }
            }
            Some("USE") if fields.len() == 5 => uses.push((
                fields[1].to_owned(),
                Reservation {
                    resource_id: fields[2].to_owned(),
                    slot: parse_u32(fields[3], "reservation_slot")?,
                    units: parse_u32(fields[4], "reservation_units")?,
                },
            )),
            Some(record_type) => {
                return Err(format!(
                    "fixture line {}: unknown or malformed record {record_type}",
                    line_number + 1
                )
                .into());
            }
            None => return Err(format!("fixture line {} is empty", line_number + 1).into()),
        }
    }

    for (candidate_id, reservation) in uses {
        let candidate = candidates
            .get_mut(&candidate_id)
            .ok_or_else(|| format!("USE references unknown candidate {candidate_id}"))?;
        candidate.reservations.push(reservation);
    }

    Ok(Scenario {
        vessel_ids,
        capacities,
        candidates: candidates.into_values().collect(),
    })
}

fn objective_line(objective: ObjectiveVector) -> String {
    format!(
        "{}\t{}\t{}\t{}\t{}",
        objective.vessel_wait,
        objective.receiver_delay,
        objective.energy_cost,
        objective.emissions,
        objective.missed_connections
    )
}

fn plan_line(plan: &maritime_exact_resource_oracle::Plan) -> String {
    format!(
        "{}\t{}",
        plan.candidate_ids.join(","),
        objective_line(plan.objective)
    )
}

fn main() -> Result<(), Box<dyn Error>> {
    let scenario = parse_scenario()?;
    let result = enumerate(&scenario)?;

    println!("RESOURCE_ORACLE_RECEIPT\t0.1.0");

    let mut vessel_ids = scenario.vessel_ids.clone();
    vessel_ids.sort();
    for vessel_id in vessel_ids {
        println!("SCENARIO_VESSEL\t{vessel_id}");
    }

    let mut capacities = scenario.capacities.clone();
    capacities.sort_by(|left, right| {
        (&left.resource_id, left.slot, left.capacity).cmp(&(
            &right.resource_id,
            right.slot,
            right.capacity,
        ))
    });
    for capacity in capacities {
        println!(
            "SCENARIO_CAPACITY\t{}\t{}\t{}",
            capacity.resource_id, capacity.slot, capacity.capacity
        );
    }

    let mut candidates = scenario.candidates.clone();
    candidates.sort_by(|left, right| left.candidate_id.cmp(&right.candidate_id));
    for candidate in candidates {
        println!(
            "SCENARIO_CANDIDATE\t{}\t{}\t{}",
            candidate.candidate_id,
            candidate.vessel_id,
            objective_line(candidate.objective)
        );
        let mut reservations = candidate.reservations.clone();
        reservations.sort_by(|left, right| {
            (&left.resource_id, left.slot, left.units).cmp(&(
                &right.resource_id,
                right.slot,
                right.units,
            ))
        });
        for reservation in reservations {
            println!(
                "SCENARIO_RESERVATION\t{}\t{}\t{}\t{}",
                candidate.candidate_id,
                reservation.resource_id,
                reservation.slot,
                reservation.units
            );
        }
    }

    println!("FEASIBLE_COUNT\t{}", result.feasible_plans.len());
    for plan in &result.feasible_plans {
        println!("FEASIBLE\t{}", plan_line(plan));
    }
    println!("PARETO_COUNT\t{}", result.pareto_front.len());
    for plan in &result.pareto_front {
        println!("PARETO\t{}", plan_line(plan));
    }
    println!("SUMMARY\tPASS");
    Ok(())
}
