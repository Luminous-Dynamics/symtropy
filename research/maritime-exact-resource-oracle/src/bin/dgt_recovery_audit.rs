use maritime_exact_resource_oracle::{
    enumerate, CandidateAction, ObjectiveVector, Reservation, ResourceCapacity, Scenario,
};
use std::collections::BTreeMap;
use std::error::Error;

const FIXTURE: &str = include_str!("../../fixtures/dgt-recovery.tsv");

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

fn main() -> Result<(), Box<dyn Error>> {
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
            Some("CAPACITY") if fields.len() == 4 => capacities.push(ResourceCapacity {
                resource_id: fields[1].to_owned(),
                slot: parse_u32(fields[2], "capacity_slot")?,
                capacity: parse_u32(fields[3], "capacity")?,
            }),
            Some("ACTION") if fields.len() == 8 => {
                let candidate_id = fields[1].to_owned();
                let action = CandidateAction {
                    candidate_id: candidate_id.clone(),
                    vessel_id: fields[2].to_owned(),
                    reservations: Vec::new(),
                    objective: ObjectiveVector {
                        vessel_wait: parse_u64(fields[3], "vessel_wait")?,
                        receiver_delay: parse_u64(fields[4], "receiver_delay")?,
                        energy_cost: parse_u64(fields[5], "energy_cost")?,
                        emissions: parse_u64(fields[6], "emissions")?,
                        missed_connections: parse_u64(fields[7], "missed_connections")?,
                    },
                };
                if candidates.insert(candidate_id.clone(), action).is_some() {
                    return Err(format!(
                        "line {}: duplicate candidate {candidate_id}",
                        line_number + 1
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
            Some(kind) => {
                return Err(format!("line {}: malformed record {kind}", line_number + 1).into())
            }
            None => return Err(format!("line {}: empty record", line_number + 1).into()),
        }
    }

    for (candidate_id, reservation) in uses {
        let action = candidates
            .get_mut(&candidate_id)
            .ok_or_else(|| format!("USE references unknown candidate {candidate_id}"))?;
        action.reservations.push(reservation);
    }

    let scenario = Scenario {
        vessel_ids,
        capacities,
        candidates: candidates.into_values().collect(),
    };
    let result = enumerate(&scenario)?;

    println!("DGT_RESOURCE_RECEIPT\t0.1");
    println!("FEASIBLE_COUNT\t{}", result.feasible_plans.len());
    for plan in &result.feasible_plans {
        println!(
            "FEASIBLE\t{}\t{}\t{}\t{}\t{}\t{}",
            plan.candidate_ids.join(","),
            plan.objective.vessel_wait,
            plan.objective.receiver_delay,
            plan.objective.energy_cost,
            plan.objective.emissions,
            plan.objective.missed_connections
        );
    }
    println!("PARETO_COUNT\t{}", result.pareto_front.len());
    for plan in &result.pareto_front {
        println!(
            "PARETO\t{}\t{}\t{}\t{}\t{}\t{}",
            plan.candidate_ids.join(","),
            plan.objective.vessel_wait,
            plan.objective.receiver_delay,
            plan.objective.energy_cost,
            plan.objective.emissions,
            plan.objective.missed_connections
        );
    }
    println!(
        "SUMMARY\tPASS\t{}/{}",
        result.feasible_plans.len(),
        result.feasible_plans.len()
    );
    Ok(())
}
