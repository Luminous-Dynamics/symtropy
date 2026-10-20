use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::process::ExitCode;

const VERSION: &str = "0.1.0";
const MAX_HORIZON: u64 = 1_000;
const MAX_TOTAL_DEMAND: u64 = 100_000;
const CASE_IDS: [&str; 6] = [
    "TWO_ORIGINS_SHARED_BOTTLENECK_001",
    "FINITE_STORAGE_BACKPRESSURE_002",
    "ENTRY_CLOSURE_DURING_DEPARTURES_003",
    "ROUND_ROBIN_MERGE_FAIRNESS_004",
    "TOTAL_SHELTER_CAPACITY_HARD_GATE_005",
    "ACCESSIBLE_CAPACITY_HARD_GATE_006",
];

type Check<T> = Result<T, String>;

#[derive(Clone, Debug)]
struct Edge {
    id: String,
    from: String,
    to: String,
    capacity: u64,
    storage: u64,
    travel_time: u64,
    closed_from: Option<u64>,
}

#[derive(Clone, Debug)]
struct Sink {
    id: String,
    node_id: String,
    ingress_capacity: u64,
    total_capacity: u64,
    accessible_capacity: u64,
}

#[derive(Clone, Debug)]
struct Demand {
    id: String,
    sink_index: usize,
    route: Vec<usize>,
    vehicles: u64,
    accessible_vehicles: u64,
    departure_step: u64,
}

#[derive(Clone, Debug)]
struct Vehicle {
    demand_index: usize,
    number: u64,
    route_position: usize,
    entered_step: u64,
    accessible: bool,
}

impl Vehicle {
    fn key(&self) -> (usize, u64) {
        (self.demand_index, self.number)
    }
}

#[derive(Clone, Debug)]
struct Request {
    target_key: String,
    target_edge: Option<usize>,
    target_sink: Option<usize>,
    origin_edge: Option<usize>,
    source_demand: Option<usize>,
    vehicle: Vehicle,
}

#[derive(Clone, Debug)]
struct DemandRuntime {
    spec: Demand,
    next_vehicle: u64,
    served: u64,
    accessible_served: u64,
}

fn text<'a>(value: &'a Value, key: &str, at: &str) -> Check<&'a str> {
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

fn optional_step(value: &Value, key: &str, at: &str) -> Check<Option<u64>> {
    match value.get(key) {
        Some(Value::Null) => Ok(None),
        Some(Value::Number(n)) => n
            .as_u64()
            .map(Some)
            .ok_or_else(|| format!("{at}.{key}: expected a non-negative integer or null")),
        _ => Err(format!("{at}.{key}: missing or invalid optional step")),
    }
}

fn read_edges(case: &Value) -> Check<(Vec<Edge>, BTreeMap<String, usize>)> {
    let case_id = text(case, "case_id", "case")?;
    let raw_edges = array(case, "edges", case_id)?;
    if raw_edges.is_empty() {
        return Err(format!("{case_id}: at least one directed edge is required"));
    }
    let mut edges = Vec::with_capacity(raw_edges.len());
    let mut indices = BTreeMap::new();
    for (i, raw) in raw_edges.iter().enumerate() {
        let at = format!("{case_id}.edges[{i}]");
        let id = text(raw, "id", &at)?.to_owned();
        let from = text(raw, "from", &at)?.to_owned();
        let to = text(raw, "to", &at)?.to_owned();
        let capacity = uint(raw, "capacity_per_step", &at)?;
        let storage = uint(raw, "storage", &at)?;
        let travel_time = uint(raw, "travel_time_steps", &at)?;
        let closed_from = optional_step(raw, "closed_from_step", &at)?;
        if id.is_empty() || from.is_empty() || to.is_empty() || from == to {
            return Err(format!("{at}: invalid ID or endpoints"));
        }
        if indices.insert(id.clone(), i).is_some() {
            return Err(format!("{at}: duplicate edge ID {id}"));
        }
        if capacity == 0 || storage == 0 || travel_time == 0 || capacity > storage {
            return Err(format!(
                "{at}: require 0 < capacity <= storage and positive travel time"
            ));
        }
        if closed_from == Some(0) {
            return Err(format!("{at}: closure step is 1-based; use null for open"));
        }
        edges.push(Edge {
            id,
            from,
            to,
            capacity,
            storage,
            travel_time,
            closed_from,
        });
    }
    Ok((edges, indices))
}

fn read_sinks(case: &Value) -> Check<(Vec<Sink>, BTreeMap<String, usize>)> {
    let case_id = text(case, "case_id", "case")?;
    let raw_sinks = array(case, "sinks", case_id)?;
    if raw_sinks.is_empty() {
        return Err(format!("{case_id}: at least one sink is required"));
    }
    let mut sinks = Vec::with_capacity(raw_sinks.len());
    let mut indices = BTreeMap::new();
    for (i, raw) in raw_sinks.iter().enumerate() {
        let at = format!("{case_id}.sinks[{i}]");
        let id = text(raw, "id", &at)?.to_owned();
        let node_id = text(raw, "node_id", &at)?.to_owned();
        let ingress_capacity = uint(raw, "ingress_capacity_per_step", &at)?;
        let total_capacity = uint(raw, "total_capacity", &at)?;
        let accessible_capacity = uint(raw, "accessible_capacity", &at)?;
        if id.is_empty() || node_id.is_empty() || ingress_capacity == 0 {
            return Err(format!("{at}: invalid ID/node or zero ingress capacity"));
        }
        if accessible_capacity > total_capacity {
            return Err(format!(
                "{at}: accessible capacity cannot exceed total capacity"
            ));
        }
        if indices.insert(id.clone(), i).is_some() {
            return Err(format!("{at}: duplicate sink ID {id}"));
        }
        sinks.push(Sink {
            id,
            node_id,
            ingress_capacity,
            total_capacity,
            accessible_capacity,
        });
    }
    Ok((sinks, indices))
}

fn read_demands(
    case: &Value,
    edges: &[Edge],
    edge_indices: &BTreeMap<String, usize>,
    sinks: &[Sink],
    sink_indices: &BTreeMap<String, usize>,
) -> Check<Vec<Demand>> {
    let case_id = text(case, "case_id", "case")?;
    let raw_demands = array(case, "demands", case_id)?;
    if raw_demands.is_empty() {
        return Err(format!("{case_id}: at least one demand stream is required"));
    }
    let mut demands = Vec::with_capacity(raw_demands.len());
    let mut ids = BTreeSet::new();
    let mut total: u64 = 0;
    for (i, raw) in raw_demands.iter().enumerate() {
        let at = format!("{case_id}.demands[{i}]");
        let id = text(raw, "id", &at)?.to_owned();
        let origin_node = text(raw, "origin_node", &at)?.to_owned();
        let destination_id = text(raw, "destination_sink_id", &at)?;
        let vehicles = uint(raw, "vehicles", &at)?;
        let accessible_vehicles = uint(raw, "accessible_vehicles", &at)?;
        let departure_step = uint(raw, "departure_step", &at)?;
        let route_ids = array(raw, "route_edge_ids", &at)?;
        if id.is_empty() || !ids.insert(id.clone()) {
            return Err(format!("{at}: empty or duplicate demand ID"));
        }
        if vehicles == 0 || accessible_vehicles > vehicles || departure_step == 0 {
            return Err(format!(
                "{at}: require positive vehicles/departure step and accessible_vehicles <= vehicles"
            ));
        }
        total = total
            .checked_add(vehicles)
            .ok_or_else(|| format!("{case_id}: total demand overflow"))?;
        if total > MAX_TOTAL_DEMAND {
            return Err(format!(
                "{case_id}: total demand exceeds guardrail {MAX_TOTAL_DEMAND}"
            ));
        }
        if route_ids.is_empty() {
            return Err(format!("{at}: route must contain at least one edge"));
        }
        let sink_index = *sink_indices
            .get(destination_id)
            .ok_or_else(|| format!("{at}: unknown sink {destination_id}"))?;
        let mut route = Vec::with_capacity(route_ids.len());
        for route_id in route_ids {
            let route_id = route_id
                .as_str()
                .ok_or_else(|| format!("{at}: route edge IDs must be strings"))?;
            let edge_index = *edge_indices
                .get(route_id)
                .ok_or_else(|| format!("{at}: unknown route edge {route_id}"))?;
            route.push(edge_index);
        }
        if edges[route[0]].from != origin_node {
            return Err(format!(
                "{at}: first route edge does not start at origin {origin_node}"
            ));
        }
        for pair in route.windows(2) {
            if edges[pair[0]].to != edges[pair[1]].from {
                return Err(format!("{at}: route is not connected"));
            }
        }
        if edges[*route.last().expect("nonempty route")].to != sinks[sink_index].node_id {
            return Err(format!(
                "{at}: final route edge does not reach sink node {}",
                sinks[sink_index].node_id
            ));
        }
        demands.push(Demand {
            id,
            sink_index,
            route,
            vehicles,
            accessible_vehicles,
            departure_step,
        });
    }
    Ok(demands)
}

fn request_target_key(request: &Request) -> &str {
    &request.target_key
}

fn make_request(
    vehicle: Vehicle,
    target_edge: Option<usize>,
    target_sink: Option<usize>,
    origin_edge: Option<usize>,
    source_demand: Option<usize>,
    edges: &[Edge],
    sinks: &[Sink],
) -> Request {
    let target_key = if let Some(edge_index) = target_edge {
        format!("E:{}", edges[edge_index].id)
    } else {
        format!("S:{}", sinks[target_sink.expect("sink target")].id)
    };
    Request {
        target_key,
        target_edge,
        target_sink,
        origin_edge,
        source_demand,
        vehicle,
    }
}

fn nearest_rank_percentile(values: &[u64], numerator: u64, denominator: u64) -> Option<u64> {
    if values.is_empty() {
        return None;
    }
    let rank = (values.len() as u64 * numerator)
        .div_ceil(denominator)
        .max(1);
    values.get((rank - 1) as usize).copied()
}

fn simulate_case(case: &Value) -> Check<Value> {
    let case_id = text(case, "case_id", "case")?;
    let horizon = uint(case, "horizon_steps", case_id)?;
    if horizon == 0 || horizon > MAX_HORIZON {
        return Err(format!("{case_id}: horizon must be in 1..={MAX_HORIZON}"));
    }
    let (edges, edge_indices) = read_edges(case)?;
    let (sinks, sink_indices) = read_sinks(case)?;
    let demands = read_demands(case, &edges, &edge_indices, &sinks, &sink_indices)?;
    let total_demand: u64 = demands.iter().map(|d| d.vehicles).sum();

    let mut runtime: Vec<DemandRuntime> = demands
        .iter()
        .cloned()
        .map(|spec| DemandRuntime {
            spec,
            next_vehicle: 0,
            served: 0,
            accessible_served: 0,
        })
        .collect();
    let mut edge_vehicles: Vec<Vec<Vehicle>> = vec![Vec::new(); edges.len()];
    let mut sink_arrived = vec![0u64; sinks.len()];
    let mut sink_accessible_arrived = vec![0u64; sinks.len()];
    let mut arrivals_by_step = Vec::<u64>::new();
    let mut queue_vehicle_steps = 0u64;
    let mut clearance_step = None;
    let mut max_source_queue = 0u64;
    let mut max_network_queue = 0u64;
    let mut max_occupancy = vec![0u64; edges.len()];
    let mut entries_by_step = vec![vec![0u64; horizon as usize]; edges.len()];
    let mut exits_by_step = vec![vec![0u64; horizon as usize]; edges.len()];
    let mut step_metrics = Vec::new();
    let mut target_cursor = BTreeMap::<String, String>::new();
    let mut closure_blocked_entry_attempts = 0u64;
    let mut storage_limited_step_count = 0u64;
    let mut free_flow_lower_bound = 0u64;

    for demand in &demands {
        let travel: u64 = demand.route.iter().map(|i| edges[*i].travel_time).sum();
        free_flow_lower_bound =
            free_flow_lower_bound.max(demand.departure_step.saturating_add(travel));
    }

    for step in 1..=horizon {
        let mut streams = BTreeMap::<String, Vec<Request>>::new();

        for (demand_index, state) in runtime.iter().enumerate() {
            if step < state.spec.departure_step {
                continue;
            }
            let first_edge = state.spec.route[0];
            for number in state.next_vehicle..state.spec.vehicles {
                let vehicle = Vehicle {
                    demand_index,
                    number,
                    route_position: 0,
                    entered_step: step,
                    accessible: number < state.spec.accessible_vehicles,
                };
                let request = make_request(
                    vehicle,
                    Some(first_edge),
                    None,
                    None,
                    Some(demand_index),
                    &edges,
                    &sinks,
                );
                streams
                    .entry(format!("D:{}", state.spec.id))
                    .or_default()
                    .push(request);
            }
        }

        for (edge_index, current_edge) in edges.iter().enumerate() {
            for vehicle in &edge_vehicles[edge_index] {
                if step
                    < vehicle
                        .entered_step
                        .saturating_add(current_edge.travel_time)
                {
                    break;
                }
                let demand = &runtime[vehicle.demand_index].spec;
                if demand.route.get(vehicle.route_position) != Some(&edge_index) {
                    return Err(format!(
                        "{case_id}: vehicle route/edge state mismatch on {}",
                        current_edge.id
                    ));
                }
                let (target_edge, target_sink) = if vehicle.route_position + 1 < demand.route.len()
                {
                    (Some(demand.route[vehicle.route_position + 1]), None)
                } else {
                    (None, Some(demand.sink_index))
                };
                let request = make_request(
                    vehicle.clone(),
                    target_edge,
                    target_sink,
                    Some(edge_index),
                    None,
                    &edges,
                    &sinks,
                );
                streams
                    .entry(format!("E:{}", current_edge.id))
                    .or_default()
                    .push(request);
            }
        }

        let stream_keys: Vec<String> = streams.keys().cloned().collect();
        let stream_queues: Vec<Vec<Request>> =
            stream_keys.iter().map(|key| streams[key].clone()).collect();
        let mut positions = vec![0usize; stream_queues.len()];
        let target_keys: BTreeSet<String> = stream_queues
            .iter()
            .flatten()
            .map(|request| request.target_key.clone())
            .collect();
        let mut edge_entries = vec![0u64; edges.len()];
        let mut edge_exits = vec![0u64; edges.len()];
        let mut sink_entries = vec![0u64; sinks.len()];
        let mut removals: Vec<BTreeSet<(usize, u64)>> = vec![BTreeSet::new(); edges.len()];
        let mut staged_edges: Vec<Vec<Vehicle>> = vec![Vec::new(); edges.len()];
        for requests in &stream_queues {
            for request in requests {
                if let Some(edge_index) = request.target_edge {
                    if edges[edge_index]
                        .closed_from
                        .is_some_and(|close_step| step >= close_step)
                    {
                        closure_blocked_entry_attempts += 1;
                    }
                }
            }
        }

        let mut made_progress = true;
        while made_progress {
            made_progress = false;
            for target_key in &target_keys {
                let mut active: Vec<usize> = (0..stream_queues.len())
                    .filter(|stream_index| {
                        positions[*stream_index] < stream_queues[*stream_index].len()
                            && request_target_key(
                                &stream_queues[*stream_index][positions[*stream_index]],
                            ) == target_key
                    })
                    .collect();
                if active.is_empty() {
                    continue;
                }
                active.sort_by(|a, b| stream_keys[*a].cmp(&stream_keys[*b]));

                let target_edge = stream_queues[active[0]][positions[active[0]]].target_edge;
                let target_sink = stream_queues[active[0]][positions[active[0]]].target_sink;
                let target_limit = if let Some(edge_index) = target_edge {
                    let edge = &edges[edge_index];
                    if edge
                        .closed_from
                        .is_some_and(|close_step| step >= close_step)
                    {
                        continue;
                    }
                    edge.capacity.min(
                        edge.storage
                            .saturating_sub(edge_vehicles[edge_index].len() as u64),
                    )
                } else {
                    let sink_index = target_sink.expect("one target kind");
                    sinks[sink_index].ingress_capacity.min(
                        sinks[sink_index]
                            .total_capacity
                            .saturating_sub(sink_arrived[sink_index]),
                    )
                };

                loop {
                    let target_accepted = if let Some(edge_index) = target_edge {
                        edge_entries[edge_index]
                    } else {
                        sink_entries[target_sink.expect("sink target")]
                    };
                    if target_accepted >= target_limit {
                        break;
                    }

                    active = (0..stream_queues.len())
                        .filter(|stream_index| {
                            positions[*stream_index] < stream_queues[*stream_index].len()
                                && request_target_key(
                                    &stream_queues[*stream_index][positions[*stream_index]],
                                ) == target_key
                        })
                        .collect();
                    if active.is_empty() {
                        break;
                    }
                    active.sort_by(|a, b| stream_keys[*a].cmp(&stream_keys[*b]));

                    let cursor = target_cursor.get(target_key);
                    let start = cursor
                        .and_then(|wanted| {
                            active.iter().position(|idx| stream_keys[*idx] >= *wanted)
                        })
                        .unwrap_or(0);
                    let mut accepted_stream = None;

                    for offset in 0..active.len() {
                        let active_index = active[(start + offset) % active.len()];
                        let request = &stream_queues[active_index][positions[active_index]];
                        if let Some(origin_edge) = request.origin_edge {
                            if edge_exits[origin_edge] >= edges[origin_edge].capacity {
                                continue;
                            }
                        }
                        if let Some(sink_index) = request.target_sink {
                            if request.vehicle.accessible
                                && sink_accessible_arrived[sink_index]
                                    >= sinks[sink_index].accessible_capacity
                            {
                                continue;
                            }
                        }
                        accepted_stream = Some(active_index);
                        break;
                    }

                    let Some(stream_index) = accepted_stream else {
                        break;
                    };
                    let request = stream_queues[stream_index][positions[stream_index]].clone();
                    positions[stream_index] += 1;
                    made_progress = true;

                    let cursor_next = active
                        .iter()
                        .position(|index| *index == stream_index)
                        .map(|i| active[(i + 1) % active.len()])
                        .expect("selected active stream");
                    target_cursor.insert(target_key.clone(), stream_keys[cursor_next].clone());

                    if let Some(origin_edge) = request.origin_edge {
                        edge_exits[origin_edge] += 1;
                        removals[origin_edge].insert(request.vehicle.key());
                    }

                    if let Some(demand_index) = request.source_demand {
                        let state = &mut runtime[demand_index];
                        if state.next_vehicle != request.vehicle.number {
                            return Err(format!(
                                "{case_id}: source queue skipped vehicle {} of {}",
                                request.vehicle.number, state.spec.id
                            ));
                        }
                        state.next_vehicle += 1;
                    }

                    if let Some(target_edge_index) = request.target_edge {
                        edge_entries[target_edge_index] += 1;
                        let mut vehicle = request.vehicle;
                        if request.origin_edge.is_some() {
                            vehicle.route_position += 1;
                        }
                        vehicle.entered_step = step;
                        staged_edges[target_edge_index].push(vehicle);
                    } else {
                        let sink_index = request.target_sink.expect("sink target");
                        sink_entries[sink_index] += 1;
                        if request.vehicle.accessible {
                            sink_accessible_arrived[sink_index] += 1;
                            runtime[request.vehicle.demand_index].accessible_served += 1;
                        }
                        sink_arrived[sink_index] += 1;
                        runtime[request.vehicle.demand_index].served += 1;
                        arrivals_by_step.push(step);
                    }
                }
            }
        }

        for edge_index in 0..edges.len() {
            edge_vehicles[edge_index]
                .retain(|vehicle| !removals[edge_index].contains(&vehicle.key()));
            edge_vehicles[edge_index].append(&mut staged_edges[edge_index]);
            let occupancy = edge_vehicles[edge_index].len() as u64;
            if occupancy > edges[edge_index].storage {
                return Err(format!(
                    "{case_id}: edge {} exceeded storage at step {step}: {occupancy}>{}",
                    edges[edge_index].id, edges[edge_index].storage
                ));
            }
            max_occupancy[edge_index] = max_occupancy[edge_index].max(occupancy);
            if edge_entries[edge_index] > edges[edge_index].capacity
                || edge_exits[edge_index] > edges[edge_index].capacity
            {
                return Err(format!(
                    "{case_id}: edge {} exceeded entry/exit capacity at step {step}",
                    edges[edge_index].id
                ));
            }
            entries_by_step[edge_index][(step - 1) as usize] = edge_entries[edge_index];
            exits_by_step[edge_index][(step - 1) as usize] = edge_exits[edge_index];
        }

        let source_queue: u64 = runtime
            .iter()
            .map(|state| state.spec.vehicles - state.next_vehicle)
            .sum();
        let network_queue: u64 = edge_vehicles.iter().map(|queue| queue.len() as u64).sum();
        let served: u64 = sink_arrived.iter().sum();
        let accounted = source_queue + network_queue + served;
        if accounted != total_demand {
            return Err(format!(
                "{case_id}: mass balance violation at step {step}: {accounted}!={total_demand}"
            ));
        }
        for (sink_index, sink) in sinks.iter().enumerate() {
            if sink_arrived[sink_index] > sink.total_capacity
                || sink_accessible_arrived[sink_index] > sink.accessible_capacity
                || sink_entries[sink_index] > sink.ingress_capacity
            {
                return Err(format!(
                    "{case_id}: sink {} violated total/access/ingress capacity at step {step}",
                    sink.id
                ));
            }
        }

        max_source_queue = max_source_queue.max(source_queue);
        max_network_queue = max_network_queue.max(network_queue);
        queue_vehicle_steps += source_queue + network_queue;
        storage_limited_step_count += edges
            .iter()
            .enumerate()
            .filter(|(i, edge)| {
                edge.storage.saturating_sub(edge_vehicles[*i].len() as u64) < edge.capacity
                    && stream_queues
                        .iter()
                        .any(|stream| stream.iter().any(|request| request.target_edge == Some(*i)))
            })
            .count() as u64;
        step_metrics.push(json!({
            "step": step,
            "source_queue": source_queue,
            "network_queue": network_queue,
            "served_vehicles": served,
            "accounted_vehicles": accounted,
            "mass_balance_ok": true,
            "edge_entries": edges.iter().enumerate().map(|(i,e)|(e.id.clone(),edge_entries[i])).collect::<BTreeMap<_,_>>(),
            "edge_exits": edges.iter().enumerate().map(|(i,e)|(e.id.clone(),edge_exits[i])).collect::<BTreeMap<_,_>>(),
            "sink_entries": sinks.iter().enumerate().map(|(i,s)|(s.id.clone(),sink_entries[i])).collect::<BTreeMap<_,_>>()
        }));

        if source_queue == 0 && network_queue == 0 && served == total_demand {
            clearance_step = Some(step);
            break;
        }
    }

    let served_total: u64 = sink_arrived.iter().sum();
    let source_remaining: u64 = runtime
        .iter()
        .map(|state| state.spec.vehicles - state.next_vehicle)
        .sum();
    let network_remaining: u64 = edge_vehicles.iter().map(|queue| queue.len() as u64).sum();
    let unserved = source_remaining + network_remaining;
    if served_total + unserved != total_demand {
        return Err(format!("{case_id}: final mass balance failed"));
    }

    let by_demand: BTreeMap<String, Value> = runtime
        .iter()
        .map(|state| {
            (
                state.spec.id.clone(),
                json!({
                    "demand_vehicles": state.spec.vehicles,
                    "served_vehicles": state.served,
                    "accessible_demand_vehicles": state.spec.accessible_vehicles,
                    "accessible_served_vehicles": state.accessible_served,
                    "source_queued_vehicles": state.spec.vehicles - state.next_vehicle
                }),
            )
        })
        .collect();
    let by_sink: BTreeMap<String, Value> = sinks
        .iter()
        .enumerate()
        .map(|(i, sink)| {
            (
                sink.id.clone(),
                json!({
                    "served_vehicles": sink_arrived[i],
                    "accessible_served_vehicles": sink_accessible_arrived[i],
                    "total_capacity": sink.total_capacity,
                    "accessible_capacity": sink.accessible_capacity,
                    "ingress_capacity_per_step": sink.ingress_capacity
                }),
            )
        })
        .collect();
    let edge_summary: Vec<Value> = edges
        .iter()
        .enumerate()
        .map(|(i, edge)| {
            let entries_after_closure: u64 = if let Some(close_step) = edge.closed_from {
                entries_by_step[i]
                    .iter()
                    .enumerate()
                    .filter(|(t, _)| (*t as u64 + 1) >= close_step)
                    .map(|(_, value)| *value)
                    .sum()
            } else {
                0
            };
            json!({
                "id": edge.id,
                "from": edge.from,
                "to": edge.to,
                "capacity_per_step": edge.capacity,
                "storage": edge.storage,
                "travel_time_steps": edge.travel_time,
                "closed_from_step": edge.closed_from,
                "max_occupancy": max_occupancy[i],
                "entries_by_step": entries_by_step[i],
                "exits_by_step": exits_by_step[i],
                "entries_at_or_after_closure": entries_after_closure
            })
        })
        .collect();
    let all_mass_balances = step_metrics
        .iter()
        .all(|metric| metric["mass_balance_ok"].as_bool() == Some(true));
    let all_capacity_checks = edge_summary.iter().all(|edge| {
        edge["entries_by_step"]
            .as_array()
            .into_iter()
            .flatten()
            .all(|count| {
                count.as_u64().unwrap_or(u64::MAX)
                    <= edge["capacity_per_step"].as_u64().unwrap_or(0)
            })
    });
    let mut arrival_steps = arrivals_by_step;
    arrival_steps.sort_unstable();
    Ok(json!({
        "case_id": case_id,
        "status": "pass",
        "horizon_steps": horizon,
        "total_demand_vehicles": total_demand,
        "served_vehicles": served_total,
        "unserved_or_stranded_vehicles": unserved,
        "source_queued_at_horizon": source_remaining,
        "network_queued_at_horizon": network_remaining,
        "completed": served_total == total_demand,
        "clearance_step": clearance_step,
        "free_flow_clearance_lower_bound_step": free_flow_lower_bound,
        "p50_arrival_step": nearest_rank_percentile(&arrival_steps, 50, 100),
        "p90_arrival_step": nearest_rank_percentile(&arrival_steps, 90, 100),
        "p95_arrival_step": nearest_rank_percentile(&arrival_steps, 95, 100),
        "queue_vehicle_steps": queue_vehicle_steps,
        "max_source_queue": max_source_queue,
        "max_network_queue": max_network_queue,
        "storage_limited_step_count": storage_limited_step_count,
        "closure_blocked_entry_attempts": closure_blocked_entry_attempts,
        "mass_balance_ok": all_mass_balances,
        "capacity_checks_ok": all_capacity_checks,
        "served_by_demand": by_demand,
        "served_by_sink": by_sink,
        "edges": edge_summary,
        "steps": step_metrics,
        "claim_boundary": "synthetic fixed-route directed network flow prototype; not a full CTM or operational evacuation planner"
    }))
}

fn validate_fixture(fixture: &Value) -> Check<Vec<Value>> {
    if fixture.get("schema_version").and_then(Value::as_str) != Some(VERSION)
        || fixture.get("benchmark_id").and_then(Value::as_str) != Some("WILDFIRE-NETWORK-001")
        || fixture.get("classification").and_then(Value::as_str) != Some("synthetic_only")
    {
        return Err("fixture metadata mismatch".to_owned());
    }
    let cases = array(fixture, "cases", "fixture")?;
    if cases.len() != CASE_IDS.len() {
        return Err(format!(
            "fixture must contain exactly {} cases",
            CASE_IDS.len()
        ));
    }
    let mut seen = BTreeSet::new();
    let mut results = Vec::new();
    for (index, case) in cases.iter().enumerate() {
        let id = text(case, "case_id", "case")?;
        if id != CASE_IDS[index] || !seen.insert(id.to_owned()) {
            return Err(format!(
                "case inventory/order mismatch at index {index}: {id}"
            ));
        }
        results.push(simulate_case(case)?);
    }
    Ok(results)
}

fn scorecard(cases: &[Value]) -> Value {
    let mut total_demand = 0u64;
    let mut total_served = 0u64;
    let mut incomplete_cases = 0usize;
    let mut queue_vehicle_steps = 0u64;
    for case in cases {
        total_demand += case["total_demand_vehicles"].as_u64().unwrap_or(0);
        total_served += case["served_vehicles"].as_u64().unwrap_or(0);
        queue_vehicle_steps += case["queue_vehicle_steps"].as_u64().unwrap_or(0);
        if case["completed"].as_bool() != Some(true) {
            incomplete_cases += 1;
        }
    }
    json!({
        "scenario_count": cases.len(),
        "total_demand_vehicles": total_demand,
        "total_served_vehicles": total_served,
        "total_unserved_or_stranded_vehicles": total_demand - total_served,
        "incomplete_case_count": incomplete_cases,
        "aggregate_queue_vehicle_steps": queue_vehicle_steps,
        "all_cases_preserve_mass_balance": cases.iter().all(|case| case["mass_balance_ok"].as_bool() == Some(true)),
        "all_cases_respect_entry_capacity": cases.iter().all(|case| case["capacity_checks_ok"].as_bool() == Some(true)),
        "claim_boundary": "synthetic fixed-route directed network flow prototype; not a full CTM or operational evacuation planner"
    })
}

fn run() -> Check<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 || args[2] != "--source-commit" {
        return Err(
            "usage: wildfire-network-flow <fixture.json> --source-commit <40-hex-sha>".to_owned(),
        );
    }
    let source_commit = &args[3];
    if source_commit.len() != 40 || !source_commit.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("--source-commit must be a 40-character hex SHA".to_owned());
    }
    let bytes = fs::read(&args[1]).map_err(|error| format!("read fixture: {error}"))?;
    let fixture: Value =
        serde_json::from_slice(&bytes).map_err(|error| format!("parse fixture: {error}"))?;
    let cases = validate_fixture(&fixture)?;
    let receipt = json!({
        "result": "pass",
        "validator_version": VERSION,
        "benchmark_id": "WILDFIRE-NETWORK-001",
        "scenario_count": cases.len(),
        "fixture_sha256": format!("{:x}", Sha256::digest(&bytes)),
        "source_commit": source_commit,
        "scorecard": scorecard(&cases),
        "cases": cases
    });
    println!(
        "{}",
        serde_json::to_string(&receipt).map_err(|error| format!("serialize receipt: {error}"))?
    );
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("wildfire network flow benchmark failed: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Value {
        serde_json::from_str(include_str!(
            "../../../../benchmarks/fixtures/WILDFIRE_NETWORK_FLOW_V0_1.json"
        ))
        .expect("synthetic network fixture parses")
    }

    #[test]
    fn merge_streams_share_receiving_capacity_and_all_demand_arrives() {
        let results = validate_fixture(&fixture()).expect("fixture validates");
        let case = &results[0];
        assert_eq!(case["served_vehicles"], 8);
        assert_eq!(case["unserved_or_stranded_vehicles"], 0);
        assert_eq!(case["served_by_demand"]["DEMAND_A"]["served_vehicles"], 4);
        assert_eq!(case["served_by_demand"]["DEMAND_B"]["served_vehicles"], 4);
        let bottleneck = case["edges"]
            .as_array()
            .expect("edges")
            .iter()
            .find(|edge| edge["id"] == "J_K")
            .expect("shared bottleneck");
        assert!(
            bottleneck["entries_by_step"]
                .as_array()
                .expect("entry trace")
                .iter()
                .all(|entry| entry.as_u64().unwrap_or(u64::MAX) <= 2)
        );
    }

    #[test]
    fn finite_storage_and_sink_ingress_limit_create_backpressure() {
        let results = validate_fixture(&fixture()).expect("fixture validates");
        let case = &results[1];
        assert_eq!(case["served_vehicles"], 8);
        assert_eq!(case["unserved_or_stranded_vehicles"], 0);
        assert!(case["storage_limited_step_count"].as_u64().unwrap_or(0) > 0);
        for edge in case["edges"].as_array().expect("edges") {
            assert!(
                edge["max_occupancy"].as_u64().unwrap_or(u64::MAX)
                    <= edge["storage"].as_u64().unwrap_or(0)
            );
        }
    }

    #[test]
    fn a_closed_link_admits_no_new_vehicles_after_cutoff() {
        let results = validate_fixture(&fixture()).expect("fixture validates");
        let case = &results[2];
        let bridge = case["edges"]
            .as_array()
            .expect("edges")
            .iter()
            .find(|edge| edge["id"] == "A_BRIDGE")
            .expect("bridge edge");
        assert_eq!(bridge["entries_at_or_after_closure"], 0);
        assert!(case["unserved_or_stranded_vehicles"].as_u64().unwrap_or(0) > 0);
        assert_eq!(case["mass_balance_ok"], true);
    }

    #[test]
    fn deterministic_round_robin_merge_serves_both_origins() {
        let results = validate_fixture(&fixture()).expect("fixture validates");
        let case = &results[3];
        assert_eq!(case["served_by_demand"]["DEMAND_A"]["served_vehicles"], 3);
        assert_eq!(case["served_by_demand"]["DEMAND_B"]["served_vehicles"], 3);
        assert_eq!(case["served_vehicles"], 6);
    }

    #[test]
    fn total_and_accessible_shelter_capacity_are_hard_limits() {
        let results = validate_fixture(&fixture()).expect("fixture validates");
        assert_eq!(results[4]["served_vehicles"], 4);
        assert_eq!(
            results[4]["served_by_sink"]["SHELTER_S"]["accessible_served_vehicles"],
            1
        );
        assert_eq!(
            results[4]["served_by_sink"]["SHELTER_S"]["served_vehicles"],
            4
        );
        assert_eq!(
            results[5]["served_by_sink"]["SHELTER_S"]["accessible_served_vehicles"],
            1
        );
        assert!(
            results[5]["unserved_or_stranded_vehicles"]
                .as_u64()
                .unwrap_or(0)
                > 0
        );
    }

    #[test]
    fn input_order_does_not_change_served_counts_or_clearance() {
        let mut fixture = fixture();
        let before = validate_fixture(&fixture).expect("original fixture");
        for case in fixture["cases"].as_array_mut().expect("cases") {
            case["edges"].as_array_mut().expect("edges").reverse();
            case["sinks"].as_array_mut().expect("sinks").reverse();
            case["demands"].as_array_mut().expect("demands").reverse();
        }
        let after = validate_fixture(&fixture).expect("reordered fixture");
        for (a, b) in before.iter().zip(after.iter()) {
            assert_eq!(a["served_vehicles"], b["served_vehicles"]);
            assert_eq!(
                a["unserved_or_stranded_vehicles"],
                b["unserved_or_stranded_vehicles"]
            );
            assert_eq!(a["clearance_step"], b["clearance_step"]);
            assert_eq!(a["served_by_demand"], b["served_by_demand"]);
        }
    }

    #[test]
    fn rejects_disconnected_routes_and_capacity_guardrail_violations() {
        let mut disconnected_route_fixture = fixture();
        disconnected_route_fixture["cases"][0]["demands"][0]["route_edge_ids"] =
            json!(["A_J", "BRIDGE_S", "K_S"]);
        assert!(validate_fixture(&disconnected_route_fixture).is_err());

        let mut overlong_horizon_fixture = fixture();
        overlong_horizon_fixture["cases"][0]["horizon_steps"] = json!(MAX_HORIZON + 1);
        assert!(validate_fixture(&overlong_horizon_fixture).is_err());
    }
}
