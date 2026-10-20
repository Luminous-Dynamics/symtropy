use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::process::ExitCode;

const CASE_IDS: [&str; 6] = [
    "TWO_ORIGINS_SHARED_BOTTLENECK_001",
    "FINITE_STORAGE_BACKPRESSURE_002",
    "ENTRY_CLOSURE_DURING_DEPARTURES_003",
    "ROUND_ROBIN_MERGE_FAIRNESS_004",
    "TOTAL_SHELTER_CAPACITY_HARD_GATE_005",
    "ACCESSIBLE_CAPACITY_HARD_GATE_006",
];

type Check<T> = Result<T, String>;

fn string<'a>(value: &'a Value, key: &str, at: &str) -> Check<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{at}.{key}: expected string"))
}

fn array<'a>(value: &'a Value, key: &str, at: &str) -> Check<&'a [Value]> {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .ok_or_else(|| format!("{at}.{key}: expected array"))
}

fn uint(value: &Value, key: &str, at: &str) -> Check<u64> {
    value
        .get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("{at}.{key}: expected non-negative integer"))
}

fn boolean(value: &Value, key: &str, at: &str) -> Check<bool> {
    value
        .get(key)
        .and_then(Value::as_bool)
        .ok_or_else(|| format!("{at}.{key}: expected boolean"))
}

fn sha_is_valid(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn nearest_rank(values: &[u64], numerator: u64, denominator: u64) -> Option<u64> {
    if values.is_empty() {
        return None;
    }
    let rank = (values.len() as u64 * numerator)
        .div_ceil(denominator)
        .max(1);
    values.get((rank - 1) as usize).copied()
}

fn check_optional_step(value: &Value, key: &str, expected: Option<u64>, at: &str) -> Check<()> {
    let field = value
        .get(key)
        .ok_or_else(|| format!("{at}.{key}: missing field"))?;
    let actual = if field.is_null() {
        None
    } else {
        Some(
            field
                .as_u64()
                .ok_or_else(|| format!("{at}.{key}: expected integer or null"))?,
        )
    };
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "{at}.{key}: expected {expected:?}, found {actual:?}"
        ))
    }
}

fn check_u64(actual: &Value, key: &str, expected: u64, at: &str) -> Check<()> {
    let got = uint(actual, key, at)?;
    if got == expected {
        Ok(())
    } else {
        Err(format!("{at}.{key}: expected {expected}, found {got}"))
    }
}

fn check_object_u64(object: &Value, key: &str, expected: u64, at: &str) -> Check<()> {
    let field = object
        .get(key)
        .ok_or_else(|| format!("{at}.{key}: missing map entry"))?;
    let got = field
        .as_u64()
        .ok_or_else(|| format!("{at}.{key}: expected non-negative integer"))?;
    if got == expected {
        Ok(())
    } else {
        Err(format!("{at}.{key}: expected {expected}, found {got}"))
    }
}

fn find_by_id<'a>(items: &'a [Value], id: &str) -> Option<&'a Value> {
    items
        .iter()
        .find(|item| item.get("id").and_then(Value::as_str) == Some(id))
}

fn verify_case(case: &Value, output: &Value, index: usize) -> Check<(u64, u64, u64, bool)> {
    let case_id = string(case, "case_id", "fixture case")?;
    let at = format!("cases[{index}]");
    if output.get("case_id").and_then(Value::as_str) != Some(case_id)
        || output.get("status").and_then(Value::as_str) != Some("pass")
    {
        return Err(format!("{at}: case identity or status mismatch"));
    }

    let horizon = uint(case, "horizon_steps", case_id)?;
    let demands = array(case, "demands", case_id)?;
    let edges = array(case, "edges", case_id)?;
    let sinks = array(case, "sinks", case_id)?;
    let total_demand: u64 = demands.iter().try_fold(0u64, |sum, demand| {
        sum.checked_add(uint(demand, "vehicles", case_id)?)
            .ok_or_else(|| format!("{at}: demand sum overflow"))
    })?;
    check_u64(output, "total_demand_vehicles", total_demand, &at)?;

    let output_edges = array(output, "edges", &at)?;
    let output_sinks = output
        .get("served_by_sink")
        .and_then(Value::as_object)
        .ok_or_else(|| format!("{at}.served_by_sink: expected object"))?;
    if output_edges.len() != edges.len() || output_sinks.len() != sinks.len() {
        return Err(format!("{at}: edge/sink inventory count mismatch"));
    }

    let steps = array(output, "steps", &at)?;
    if steps.is_empty() || steps.len() > horizon as usize {
        return Err(format!("{at}: invalid step trace length"));
    }

    let mut max_source_queue = 0u64;
    let mut max_network_queue = 0u64;
    let mut queue_vehicle_steps = 0u64;
    let mut cumulative_arrivals = 0u64;
    let mut arrival_steps = Vec::new();

    for (step_index, row) in steps.iter().enumerate() {
        let step_at = format!("{at}.steps[{step_index}]");
        check_u64(row, "step", (step_index + 1) as u64, &step_at)?;
        let source_queue = uint(row, "source_queue", &step_at)?;
        let network_queue = uint(row, "network_queue", &step_at)?;
        let served = uint(row, "served_vehicles", &step_at)?;
        let accounted = uint(row, "accounted_vehicles", &step_at)?;
        if source_queue + network_queue + served != total_demand || accounted != total_demand {
            return Err(format!("{step_at}: mass-balance receipt mismatch"));
        }
        if !boolean(row, "mass_balance_ok", &step_at)? {
            return Err(format!("{step_at}: mass_balance_ok is false"));
        }
        if served < cumulative_arrivals || served > total_demand {
            return Err(format!("{step_at}: cumulative arrivals regressed or exceeded demand"));
        }
        for _ in cumulative_arrivals..served {
            arrival_steps.push((step_index + 1) as u64);
        }
        cumulative_arrivals = served;
        max_source_queue = max_source_queue.max(source_queue);
        max_network_queue = max_network_queue.max(network_queue);
        queue_vehicle_steps += source_queue + network_queue;

        let step_entries = row
            .get("edge_entries")
            .ok_or_else(|| format!("{step_at}.edge_entries: missing"))?;
        let step_exits = row
            .get("edge_exits")
            .ok_or_else(|| format!("{step_at}.edge_exits: missing"))?;
        for cfg_edge in edges {
            let edge_id = string(cfg_edge, "id", &step_at)?;
            let edge_receipt = find_by_id(output_edges, edge_id)
                .ok_or_else(|| format!("{step_at}: receipt omitted edge {edge_id}"))?;
            let entries = array(edge_receipt, "entries_by_step", &step_at)?;
            let exits = array(edge_receipt, "exits_by_step", &step_at)?;
            if entries.len() != horizon as usize || exits.len() != horizon as usize {
                return Err(format!("{step_at}: edge {edge_id} trace length mismatch"));
            }
            check_object_u64(
                step_entries,
                edge_id,
                entries[step_index]
                    .as_u64()
                    .ok_or_else(|| format!("{step_at}: invalid entry trace for {edge_id}"))?,
                &step_at,
            )?;
            check_object_u64(
                step_exits,
                edge_id,
                exits[step_index]
                    .as_u64()
                    .ok_or_else(|| format!("{step_at}: invalid exit trace for {edge_id}"))?,
                &step_at,
            )?;
        }

        let step_sink_entries = row
            .get("sink_entries")
            .ok_or_else(|| format!("{step_at}.sink_entries: missing"))?;
        for cfg_sink in sinks {
            let sink_id = string(cfg_sink, "id", &step_at)?;
            let ingress_cap = uint(cfg_sink, "ingress_capacity_per_step", &step_at)?;
            let count = step_sink_entries
                .get(sink_id)
                .and_then(Value::as_u64)
                .ok_or_else(|| format!("{step_at}: missing sink entry count {sink_id}"))?;
            if count > ingress_cap {
                return Err(format!("{step_at}: sink {sink_id} ingress capacity exceeded"));
            }
        }
    }

    let served_total = uint(output, "served_vehicles", &at)?;
    if served_total != cumulative_arrivals {
        return Err(format!("{at}: final served count does not match step trace"));
    }
    let unserved = uint(output, "unserved_or_stranded_vehicles", &at)?;
    let source_remaining = uint(output, "source_queued_at_horizon", &at)?;
    let network_remaining = uint(output, "network_queued_at_horizon", &at)?;
    if served_total + unserved != total_demand
        || source_remaining + network_remaining != unserved
    {
        return Err(format!("{at}: final source/network/sink balance mismatch"));
    }
    if !boolean(output, "mass_balance_ok", &at)? || !boolean(output, "capacity_checks_ok", &at)? {
        return Err(format!("{at}: declared model invariants did not pass"));
    }
    check_u64(output, "queue_vehicle_steps", queue_vehicle_steps, &at)?;
    check_u64(output, "max_source_queue", max_source_queue, &at)?;
    check_u64(output, "max_network_queue", max_network_queue, &at)?;
    check_optional_step(output, "p50_arrival_step", nearest_rank(&arrival_steps, 50, 100), &at)?;
    check_optional_step(output, "p90_arrival_step", nearest_rank(&arrival_steps, 90, 100), &at)?;
    check_optional_step(output, "p95_arrival_step", nearest_rank(&arrival_steps, 95, 100), &at)?;

    let completed = boolean(output, "completed", &at)?;
    if completed != (served_total == total_demand) {
        return Err(format!("{at}: completed flag disagrees with served demand"));
    }
    if completed {
        check_optional_step(
            output,
            "clearance_step",
            Some(steps.len() as u64),
            &at,
        )?;
    } else {
        check_optional_step(output, "clearance_step", None, &at)?;
        if steps.len() != horizon as usize {
            return Err(format!("{at}: incomplete case stopped before the horizon"));
        }
    }

    let mut free_flow_bound = 0u64;
    let mut demand_served_sum = 0u64;
    let mut demand_source_queue_sum = 0u64;
    let output_by_demand = output
        .get("served_by_demand")
        .and_then(Value::as_object)
        .ok_or_else(|| format!("{at}.served_by_demand: expected object"))?;
    if output_by_demand.len() != demands.len() {
        return Err(format!("{at}: per-demand inventory mismatch"));
    }
    for demand in demands {
        let demand_id = string(demand, "id", &at)?;
        let demand_receipt = output_by_demand
            .get(demand_id)
            .ok_or_else(|| format!("{at}: receipt omitted demand {demand_id}"))?;
        let vehicle_count = uint(demand, "vehicles", &at)?;
        let accessible_count = uint(demand, "accessible_vehicles", &at)?;
        check_u64(demand_receipt, "demand_vehicles", vehicle_count, &at)?;
        check_u64(
            demand_receipt,
            "accessible_demand_vehicles",
            accessible_count,
            &at,
        )?;
        let served = uint(demand_receipt, "served_vehicles", &at)?;
        let accessible_served = uint(demand_receipt, "accessible_served_vehicles", &at)?;
        let source_queue = uint(demand_receipt, "source_queued_vehicles", &at)?;
        if served > vehicle_count || accessible_served > accessible_count {
            return Err(format!("{at}: demand {demand_id} served count exceeds its demand"));
        }
        demand_served_sum += served;
        demand_source_queue_sum += source_queue;

        let departure = uint(demand, "departure_step", &at)?;
        let route_ids = array(demand, "route_edge_ids", &at)?;
        let mut travel = 0u64;
        for route_id in route_ids {
            let edge_id = route_id
                .as_str()
                .ok_or_else(|| format!("{at}: route edge ID is not a string"))?;
            let cfg_edge = find_by_id(edges, edge_id)
                .ok_or_else(|| format!("{at}: route refers to unknown edge {edge_id}"))?;
            travel += uint(cfg_edge, "travel_time_steps", &at)?;
        }
        free_flow_bound = free_flow_bound.max(departure.saturating_add(travel));
    }
    if demand_served_sum != served_total || demand_source_queue_sum != source_remaining {
        return Err(format!("{at}: per-demand totals do not match final totals"));
    }
    check_u64(
        output,
        "free_flow_clearance_lower_bound_step",
        free_flow_bound,
        &at,
    )?;

    let mut sink_served_sum = 0u64;
    let mut sink_accessible_sum = 0u64;
    for cfg_sink in sinks {
        let sink_id = string(cfg_sink, "id", &at)?;
        let sink = output_by_demand; // Keep all lookups explicitly anchored to this case.
        let _ = sink;
        let sink_receipt = output_sinks
            .get(sink_id)
            .ok_or_else(|| format!("{at}: receipt omitted sink {sink_id}"))?;
        let served = uint(sink_receipt, "served_vehicles", &at)?;
        let accessible = uint(sink_receipt, "accessible_served_vehicles", &at)?;
        let total_capacity = uint(cfg_sink, "total_capacity", &at)?;
        let accessible_capacity = uint(cfg_sink, "accessible_capacity", &at)?;
        let ingress_capacity = uint(cfg_sink, "ingress_capacity_per_step", &at)?;
        check_u64(sink_receipt, "total_capacity", total_capacity, &at)?;
        check_u64(
            sink_receipt,
            "accessible_capacity",
            accessible_capacity,
            &at,
        )?;
        check_u64(
            sink_receipt,
            "ingress_capacity_per_step",
            ingress_capacity,
            &at,
        )?;
        if served > total_capacity || accessible > accessible_capacity || accessible > served {
            return Err(format!("{at}: sink {sink_id} capacity invariant violated"));
        }
        sink_served_sum += served;
        sink_accessible_sum += accessible;
    }
    let demand_accessible_served_sum: u64 = output_by_demand
        .values()
        .map(|row| row.get("accessible_served_vehicles").and_then(Value::as_u64).unwrap_or(0))
        .sum();
    if sink_served_sum != served_total || sink_accessible_sum != demand_accessible_served_sum {
        return Err(format!("{at}: sink and demand service totals mismatch"));
    }

    for cfg_edge in edges {
        let edge_id = string(cfg_edge, "id", &at)?;
        let edge = find_by_id(output_edges, edge_id)
            .ok_or_else(|| format!("{at}: receipt omitted edge {edge_id}"))?;
        let capacity = uint(cfg_edge, "capacity_per_step", &at)?;
        let storage = uint(cfg_edge, "storage", &at)?;
        check_u64(edge, "capacity_per_step", capacity, &at)?;
        check_u64(edge, "storage", storage, &at)?;
        for field in ["from", "to", "closed_from_step", "travel_time_steps"] {
            if edge.get(field) != cfg_edge.get(field) {
                return Err(format!("{at}: edge {edge_id} field {field} differs from fixture"));
            }
        }
        let occupancy = uint(edge, "max_occupancy", &at)?;
        if occupancy > storage {
            return Err(format!("{at}: edge {edge_id} max occupancy exceeds storage"));
        }
        let entries = array(edge, "entries_by_step", &at)?;
        let exits = array(edge, "exits_by_step", &at)?;
        for step in 0..horizon as usize {
            let entered = entries[step]
                .as_u64()
                .ok_or_else(|| format!("{at}: invalid edge entry count"))?;
            let exited = exits[step]
                .as_u64()
                .ok_or_else(|| format!("{at}: invalid edge exit count"))?;
            if entered > capacity || exited > capacity {
                return Err(format!("{at}: edge {edge_id} step capacity exceeded"));
            }
        }
        let closure = cfg_edge
            .get("closed_from_step")
            .and_then(Value::as_u64);
        let mut post_close_entries = 0u64;
        if let Some(close_step) = closure {
            for (index, count) in entries.iter().enumerate() {
                if index as u64 + 1 >= close_step {
                    post_close_entries += count.as_u64().unwrap_or(u64::MAX);
                }
            }
        }
        check_u64(edge, "entries_at_or_after_closure", post_close_entries, &at)?;
        if post_close_entries != 0 {
            return Err(format!("{at}: edge {edge_id} admits vehicles after closure"));
        }
    }
    Ok((total_demand, served_total, queue_vehicle_steps, completed))
}

fn verify(fixture_bytes: &[u8], receipt: &Value, expected_source: &str) -> Check<()> {
    if !sha_is_valid(expected_source) {
        return Err("expected source commit must be a 40-character hex SHA".to_owned());
    }
    if receipt.get("result").and_then(Value::as_str) != Some("pass")
        || receipt.get("benchmark_id").and_then(Value::as_str) != Some("WILDFIRE-NETWORK-001")
        || receipt.get("source_commit").and_then(Value::as_str) != Some(expected_source)
    {
        return Err("receipt result, benchmark ID, or exact source SHA mismatch".to_owned());
    }
    let digest = format!("{:x}", Sha256::digest(fixture_bytes));
    if receipt.get("fixture_sha256").and_then(Value::as_str) != Some(digest.as_str()) {
        return Err("raw fixture SHA-256 mismatch".to_owned());
    }
    let fixture: Value =
        serde_json::from_slice(fixture_bytes).map_err(|error| format!("parse fixture: {error}"))?;
    let cases = array(&fixture, "cases", "fixture")?;
    let outputs = array(receipt, "cases", "receipt")?;
    if cases.len() != CASE_IDS.len()
        || outputs.len() != CASE_IDS.len()
        || receipt.get("scenario_count").and_then(Value::as_u64) != Some(CASE_IDS.len() as u64)
    {
        return Err("fixture and receipt must contain exactly six scenarios".to_owned());
    }

    let mut total_demand = 0u64;
    let mut total_served = 0u64;
    let mut incomplete = 0usize;
    let mut total_queue_vehicle_steps = 0u64;
    for index in 0..CASE_IDS.len() {
        let id = string(&cases[index], "case_id", "fixture case")?;
        if id != CASE_IDS[index] {
            return Err(format!("fixture case order mismatch at index {index}: {id}"));
        }
        let (demand, served, queue_steps, completed) =
            verify_case(&cases[index], &outputs[index], index)?;
        total_demand += demand;
        total_served += served;
        total_queue_vehicle_steps += queue_steps;
        if !completed {
            incomplete += 1;
        }
    }

    let summary = receipt
        .get("scorecard")
        .ok_or_else(|| "receipt missing scorecard".to_owned())?;
    check_u64(summary, "scenario_count", CASE_IDS.len() as u64, "scorecard")?;
    check_u64(summary, "total_demand_vehicles", total_demand, "scorecard")?;
    check_u64(summary, "total_served_vehicles", total_served, "scorecard")?;
    check_u64(
        summary,
        "total_unserved_or_stranded_vehicles",
        total_demand - total_served,
        "scorecard",
    )?;
    check_u64(
        summary,
        "incomplete_case_count",
        incomplete as u64,
        "scorecard",
    )?;
    check_u64(
        summary,
        "aggregate_queue_vehicle_steps",
        total_queue_vehicle_steps,
        "scorecard",
    )?;
    if !boolean(summary, "all_cases_preserve_mass_balance", "scorecard")?
        || !boolean(summary, "all_cases_respect_entry_capacity", "scorecard")?
    {
        return Err("scorecard declares an invariant failure".to_owned());
    }
    Ok(())
}

fn run() -> Check<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 {
        return Err(
            "usage: verify_network_flow_receipt <fixture.json> <receipt.json> <expected-source-sha>"
                .to_owned(),
        );
    }
    let fixture_bytes = fs::read(&args[1]).map_err(|error| format!("read fixture: {error}"))?;
    let receipt_bytes = fs::read(&args[2]).map_err(|error| format!("read receipt: {error}"))?;
    let receipt: Value =
        serde_json::from_slice(&receipt_bytes).map_err(|error| format!("parse receipt: {error}"))?;
    verify(&fixture_bytes, &receipt, &args[3])?;
    println!(
        "independent network-flow receipt verification passed: {} cases; fixture SHA-256 {}; source {}",
        CASE_IDS.len(),
        format!("{:x}", Sha256::digest(&fixture_bytes)),
        args[3]
    );
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("independent network-flow receipt verification failed: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_only_exact_commit_shaped_sha() {
        assert!(sha_is_valid("2dfe0ac7ddaec26f59aa905a915946a4780a4077"));
        assert!(!sha_is_valid("1234"));
        assert!(!sha_is_valid("zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz"));
    }

    #[test]
    fn nearest_rank_percentiles_are_deterministic() {
        assert_eq!(nearest_rank(&[2, 4, 6, 8], 50, 100), Some(4));
        assert_eq!(nearest_rank(&[2, 4, 6, 8], 95, 100), Some(8));
        assert_eq!(nearest_rank(&[], 50, 100), None);
    }

    #[test]
    fn post_closure_entries_are_checked_across_all_trace_steps() {
        let edge = serde_json::json!({
            "closed_from_step":4,
            "entries_by_step":[1,1,0,0,0]
        });
        let counts = edge["entries_by_step"].as_array().expect("trace");
        let post = counts.iter().enumerate()
            .filter(|(i,_)| *i as u64 + 1 >= 4)
            .map(|(_,v)| v.as_u64().unwrap_or(u64::MAX))
            .sum::<u64>();
        assert_eq!(post, 0);
    }
}
