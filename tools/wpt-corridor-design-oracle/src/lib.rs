// SPDX-License-Identifier: AGPL-3.0-or-later
//! Bounded exact corridor siting oracle with N-1 single-segment outage service.
//!
//! The evaluator enumerates every candidate segment subset, checks feeder and
//! segment capacities jointly with per-route useful-energy floors, and keeps
//! minimum-CAPEX designs that satisfy normal operation plus every single
//! installed-segment outage. This is a synthetic design reference, not a
//! physical WPT, feeder-outage, pavement-life or financial model.

use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

const MAX_FEEDERS: usize = 4;
const MAX_ROUTES: usize = 8;
const MAX_CANDIDATE_SEGMENTS: usize = 12;
const MAX_TOTAL_ROUTE_REQUIREMENT_KWH: u64 = 1_000_000;

fn fail(path: &str, message: impl AsRef<str>) -> String {
    format!("{path}: {}", message.as_ref())
}

fn object<'a>(value: &'a Value, path: &str) -> Result<&'a serde_json::Map<String, Value>, String> {
    value
        .as_object()
        .ok_or_else(|| fail(path, "must be an object"))
}

fn exact_keys(value: &Value, allowed: &[&str], path: &str) -> Result<(), String> {
    for key in object(value, path)?.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(fail(path, format!("unknown property {key}")));
        }
    }
    Ok(())
}

fn field<'a>(value: &'a Value, key: &str, path: &str) -> Result<&'a Value, String> {
    value
        .as_object()
        .and_then(|map| map.get(key))
        .ok_or_else(|| fail(path, format!("missing property {key}")))
}

fn string_field<'a>(value: &'a Value, key: &str, path: &str) -> Result<&'a str, String> {
    field(value, key, path)?
        .as_str()
        .filter(|text| !text.trim().is_empty())
        .ok_or_else(|| fail(&format!("{path}.{key}"), "must be a non-empty string"))
}

fn uint_field(value: &Value, key: &str, path: &str) -> Result<u64, String> {
    field(value, key, path)?
        .as_u64()
        .ok_or_else(|| fail(&format!("{path}.{key}"), "must be a non-negative integer"))
}

fn array_field<'a>(value: &'a Value, key: &str, path: &str) -> Result<&'a [Value], String> {
    field(value, key, path)?
        .as_array()
        .map(Vec::as_slice)
        .ok_or_else(|| fail(&format!("{path}.{key}"), "must be an array"))
}

#[derive(Debug, Clone)]
struct Feeder {
    feeder_id: String,
    capacity_kwh: u64,
}

#[derive(Debug, Clone)]
struct Route {
    route_id: String,
    minimum_useful_energy_kwh: u64,
}

#[derive(Debug, Clone)]
struct CandidateSegment {
    segment_id: String,
    feeder_index: usize,
    max_deliverable_energy_kwh: u64,
    capex_units: u64,
    route_indices: Vec<usize>,
}

#[derive(Debug, Clone)]
struct Scenario {
    corridor_id: String,
    feeders: Vec<Feeder>,
    routes: Vec<Route>,
    segments: Vec<CandidateSegment>,
    total_required_kwh: u64,
}

fn parse_scenario(value: &Value) -> Result<Scenario, String> {
    exact_keys(
        value,
        &[
            "schema_version",
            "corridor_id",
            "evidence_kind",
            "feeders",
            "routes",
            "candidate_segments",
            "quality_flags",
        ],
        "$",
    )?;
    if string_field(value, "schema_version", "$")? != "wpt-corridor-design-v0" {
        return Err(fail("$.schema_version", "unsupported schema version"));
    }
    let corridor_id = string_field(value, "corridor_id", "$")?.to_string();
    if string_field(value, "evidence_kind", "$")? != "SYNTHETIC_KNOWN_TRUTH" {
        return Err(fail(
            "$.evidence_kind",
            "v0 admits synthetic known-truth scenarios only",
        ));
    }
    let flags = array_field(value, "quality_flags", "$")?;
    if flags.len() != 1 || flags[0].as_str() != Some("SYNTHETIC_ONLY") {
        return Err(fail(
            "$.quality_flags",
            "v0 scenarios must carry exactly SYNTHETIC_ONLY",
        ));
    }

    let feeder_rows = array_field(value, "feeders", "$")?;
    if feeder_rows.is_empty() || feeder_rows.len() > MAX_FEEDERS {
        return Err(fail(
            "$.feeders",
            format!("must contain 1..={MAX_FEEDERS} feeders"),
        ));
    }
    let mut feeders = Vec::with_capacity(feeder_rows.len());
    let mut feeder_indices = BTreeMap::new();
    for (index, row) in feeder_rows.iter().enumerate() {
        let path = format!("$.feeders[{index}]");
        exact_keys(
            row,
            &["feeder_id", "available_deliverable_energy_kwh"],
            &path,
        )?;
        let feeder_id = string_field(row, "feeder_id", &path)?.to_string();
        let capacity_kwh = uint_field(row, "available_deliverable_energy_kwh", &path)?;
        if capacity_kwh == 0 {
            return Err(fail(&path, "feeder capacity must be positive"));
        }
        if feeder_indices.insert(feeder_id.clone(), index).is_some() {
            return Err(fail(&path, format!("duplicate feeder_id {feeder_id}")));
        }
        feeders.push(Feeder {
            feeder_id,
            capacity_kwh,
        });
    }

    let route_rows = array_field(value, "routes", "$")?;
    if route_rows.is_empty() || route_rows.len() > MAX_ROUTES {
        return Err(fail(
            "$.routes",
            format!("must contain 1..={MAX_ROUTES} routes"),
        ));
    }
    let mut routes = Vec::with_capacity(route_rows.len());
    let mut route_indices = BTreeMap::new();
    let mut total_required_kwh = 0_u64;
    for (index, row) in route_rows.iter().enumerate() {
        let path = format!("$.routes[{index}]");
        exact_keys(row, &["route_id", "minimum_useful_energy_kwh"], &path)?;
        let route_id = string_field(row, "route_id", &path)?.to_string();
        let minimum_useful_energy_kwh = uint_field(row, "minimum_useful_energy_kwh", &path)?;
        if minimum_useful_energy_kwh == 0 {
            return Err(fail(&path, "route service floor must be positive"));
        }
        total_required_kwh = total_required_kwh
            .checked_add(minimum_useful_energy_kwh)
            .ok_or_else(|| fail("$.routes", "total route energy requirement overflowed"))?;
        if total_required_kwh > MAX_TOTAL_ROUTE_REQUIREMENT_KWH {
            return Err(fail(
                "$.routes",
                format!(
                    "total route requirement exceeds bounded limit {MAX_TOTAL_ROUTE_REQUIREMENT_KWH} kWh"
                ),
            ));
        }
        if route_indices.insert(route_id.clone(), index).is_some() {
            return Err(fail(&path, format!("duplicate route_id {route_id}")));
        }
        routes.push(Route {
            route_id,
            minimum_useful_energy_kwh,
        });
    }

    let segment_rows = array_field(value, "candidate_segments", "$")?;
    if segment_rows.is_empty() || segment_rows.len() > MAX_CANDIDATE_SEGMENTS {
        return Err(fail(
            "$.candidate_segments",
            format!("must contain 1..={MAX_CANDIDATE_SEGMENTS} candidate segments"),
        ));
    }
    let mut segments = Vec::with_capacity(segment_rows.len());
    let mut segment_ids = BTreeSet::new();
    for (index, row) in segment_rows.iter().enumerate() {
        let path = format!("$.candidate_segments[{index}]");
        exact_keys(
            row,
            &[
                "segment_id",
                "feeder_id",
                "max_deliverable_energy_kwh",
                "capex_units",
                "served_route_ids",
            ],
            &path,
        )?;
        let segment_id = string_field(row, "segment_id", &path)?.to_string();
        if !segment_ids.insert(segment_id.clone()) {
            return Err(fail(&path, format!("duplicate segment_id {segment_id}")));
        }
        let feeder_id = string_field(row, "feeder_id", &path)?;
        let feeder_index = *feeder_indices
            .get(feeder_id)
            .ok_or_else(|| fail(&path, format!("unknown feeder_id {feeder_id}")))?;
        let max_deliverable_energy_kwh = uint_field(row, "max_deliverable_energy_kwh", &path)?;
        if max_deliverable_energy_kwh == 0 {
            return Err(fail(&path, "segment energy capacity must be positive"));
        }
        let capex_units = uint_field(row, "capex_units", &path)?;
        if capex_units == 0 {
            return Err(fail(&path, "capex_units must be positive"));
        }
        let served_rows = array_field(row, "served_route_ids", &path)?;
        if served_rows.is_empty() || served_rows.len() > MAX_ROUTES {
            return Err(fail(
                &path,
                format!("served_route_ids must contain 1..={MAX_ROUTES} entries"),
            ));
        }
        let mut seen_routes = BTreeSet::new();
        let mut route_indices_for_segment = Vec::with_capacity(served_rows.len());
        for (route_index, route_value) in served_rows.iter().enumerate() {
            let route_id = route_value
                .as_str()
                .filter(|text| !text.trim().is_empty())
                .ok_or_else(|| {
                    fail(
                        &format!("{path}.served_route_ids[{route_index}]"),
                        "route ID must be a non-empty string",
                    )
                })?;
            if !seen_routes.insert(route_id.to_string()) {
                return Err(fail(&path, format!("duplicate served route {route_id}")));
            }
            let route_index = *route_indices
                .get(route_id)
                .ok_or_else(|| fail(&path, format!("unknown route_id {route_id}")))?;
            route_indices_for_segment.push(route_index);
        }
        route_indices_for_segment.sort_unstable();
        segments.push(CandidateSegment {
            segment_id,
            feeder_index,
            max_deliverable_energy_kwh,
            capex_units,
            route_indices: route_indices_for_segment,
        });
    }

    Ok(Scenario {
        corridor_id,
        feeders,
        routes,
        segments,
        total_required_kwh,
    })
}

#[derive(Debug, Clone)]
struct Edge {
    to: usize,
    reverse_index: usize,
    residual_kwh: u64,
}

fn add_edge(graph: &mut [Vec<Edge>], from: usize, to: usize, capacity_kwh: u64) -> usize {
    let forward_index = graph[from].len();
    let reverse_index = graph[to].len();
    graph[from].push(Edge {
        to,
        reverse_index,
        residual_kwh: capacity_kwh,
    });
    graph[to].push(Edge {
        to: from,
        reverse_index: forward_index,
        residual_kwh: 0,
    });
    forward_index
}

fn max_flow(graph: &mut [Vec<Edge>], source: usize, sink: usize) -> Result<u64, String> {
    let mut total_flow = 0_u64;
    loop {
        let mut parent = vec![None::<(usize, usize)>; graph.len()];
        let mut queue = VecDeque::new();
        parent[source] = Some((source, usize::MAX));
        queue.push_back(source);

        while let Some(from) = queue.pop_front() {
            for (edge_index, edge) in graph[from].iter().enumerate() {
                if edge.residual_kwh > 0 && parent[edge.to].is_none() {
                    parent[edge.to] = Some((from, edge_index));
                    queue.push_back(edge.to);
                    if edge.to == sink {
                        break;
                    }
                }
            }
            if parent[sink].is_some() {
                break;
            }
        }

        if parent[sink].is_none() {
            break;
        }

        let mut augmentation = u64::MAX;
        let mut node = sink;
        while node != source {
            let (from, edge_index) = parent[node]
                .ok_or_else(|| "internal max-flow predecessor disappeared".to_string())?;
            augmentation = augmentation.min(graph[from][edge_index].residual_kwh);
            node = from;
        }
        if augmentation == 0 || augmentation == u64::MAX {
            return Err("internal max-flow augmentation was not positive".into());
        }

        node = sink;
        while node != source {
            let (from, edge_index) = parent[node]
                .ok_or_else(|| "internal max-flow predecessor disappeared".to_string())?;
            let to = graph[from][edge_index].to;
            let reverse_index = graph[from][edge_index].reverse_index;
            graph[from][edge_index].residual_kwh -= augmentation;
            graph[to][reverse_index].residual_kwh = graph[to][reverse_index]
                .residual_kwh
                .checked_add(augmentation)
                .ok_or_else(|| "residual capacity overflowed".to_string())?;
            node = from;
        }
        total_flow = total_flow
            .checked_add(augmentation)
            .ok_or_else(|| "total max-flow value overflowed".to_string())?;
    }
    Ok(total_flow)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteService {
    pub route_id: String,
    pub required_kwh: u64,
    pub delivered_kwh: u64,
    pub shortfall_kwh: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenarioCheck {
    pub scenario_id: String,
    pub failed_segment_id: Option<String>,
    pub failed_feeder_id: Option<String>,
    pub total_required_kwh: u64,
    pub total_delivered_kwh: u64,
    pub feasible: bool,
    pub route_service: Vec<RouteService>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BestDesign {
    pub selected_segment_ids: Vec<String>,
    pub capex_units: u64,
    pub scenario_checks: Vec<ScenarioCheck>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorridorDesignReport {
    pub corridor_id: String,
    pub candidate_segment_count: usize,
    pub subsets_evaluated: u64,
    pub robust_feasible_design_count: u64,
    pub minimum_capex_units: Option<u64>,
    pub best_designs: Vec<BestDesign>,
    pub claim_ceiling: &'static str,
}

fn evaluate_service_scenario(
    scenario: &Scenario,
    selected_segments: &[usize],
    failed_segment: Option<usize>,
    failed_feeder: Option<usize>,
) -> Result<ScenarioCheck, String> {
    let source = 0;
    let feeder_start = 1;
    let segment_start = feeder_start + scenario.feeders.len();
    let route_start = segment_start + scenario.segments.len();
    let sink = route_start + scenario.routes.len();
    let mut graph = vec![Vec::<Edge>::new(); sink + 1];

    for (index, feeder) in scenario.feeders.iter().enumerate() {
        if Some(index) == failed_feeder {
            continue;
        }
        add_edge(
            &mut graph,
            source,
            feeder_start + index,
            feeder.capacity_kwh,
        );
    }

    for segment_index in selected_segments {
        if Some(*segment_index) == failed_segment {
            continue;
        }
        let segment = &scenario.segments[*segment_index];
        if Some(segment.feeder_index) == failed_feeder {
            continue;
        }
        let segment_node = segment_start + *segment_index;
        let feeder_node = feeder_start + segment.feeder_index;
        add_edge(
            &mut graph,
            feeder_node,
            segment_node,
            segment.max_deliverable_energy_kwh,
        );
        for route_index in &segment.route_indices {
            add_edge(
                &mut graph,
                segment_node,
                route_start + *route_index,
                segment.max_deliverable_energy_kwh,
            );
        }
    }

    let mut route_sink_edges = Vec::with_capacity(scenario.routes.len());
    for (route_index, route) in scenario.routes.iter().enumerate() {
        let edge_index = add_edge(
            &mut graph,
            route_start + route_index,
            sink,
            route.minimum_useful_energy_kwh,
        );
        route_sink_edges.push((route_start + route_index, edge_index));
    }

    let total_delivered_kwh = max_flow(&mut graph, source, sink)?;
    let mut route_service = Vec::with_capacity(scenario.routes.len());
    for (route_index, route) in scenario.routes.iter().enumerate() {
        let (from, edge_index) = route_sink_edges[route_index];
        let delivered_kwh = route.minimum_useful_energy_kwh - graph[from][edge_index].residual_kwh;
        route_service.push(RouteService {
            route_id: route.route_id.clone(),
            required_kwh: route.minimum_useful_energy_kwh,
            delivered_kwh,
            shortfall_kwh: route.minimum_useful_energy_kwh - delivered_kwh,
        });
    }
    route_service.sort_by_key(|row| row.route_id.clone());

    Ok(ScenarioCheck {
        scenario_id: match (failed_segment, failed_feeder) {
            (None, None) => "NORMAL".to_string(),
            (Some(index), None) => {
                format!("SEGMENT_OUTAGE:{}", scenario.segments[index].segment_id)
            }
            (None, Some(index)) => {
                format!("FEEDER_OUTAGE:{}", scenario.feeders[index].feeder_id)
            }
            (Some(_), Some(_)) => {
                return Err(
                    "internal outage scenario cannot remove both a segment and feeder".into(),
                );
            }
        },
        failed_segment_id: failed_segment.map(|index| scenario.segments[index].segment_id.clone()),
        failed_feeder_id: failed_feeder.map(|index| scenario.feeders[index].feeder_id.clone()),
        total_required_kwh: scenario.total_required_kwh,
        total_delivered_kwh,
        feasible: total_delivered_kwh == scenario.total_required_kwh,
        route_service,
    })
}

fn selected_segment_ids(scenario: &Scenario, selected: &[usize]) -> Vec<String> {
    let mut ids = selected
        .iter()
        .map(|index| scenario.segments[*index].segment_id.clone())
        .collect::<Vec<_>>();
    ids.sort();
    ids
}

/// Enumerate every install/uninstall subset and return the minimum-CAPEX design
/// that preserves every route's useful-energy floor in normal operation and
/// after each single installed-segment or single feeder outage (one at a time).
pub fn evaluate_corridor(value: &Value) -> Result<CorridorDesignReport, String> {
    let scenario = parse_scenario(value)?;
    let candidate_count = scenario.segments.len();
    let subset_count = 1_u64 << candidate_count;
    let mut subsets_evaluated = 0_u64;
    let mut robust_feasible_design_count = 0_u64;
    let mut minimum_capex_units = None::<u64>;
    let mut best_designs = Vec::<BestDesign>::new();

    for mask in 1_u64..subset_count {
        subsets_evaluated += 1;
        let selected_segments = (0..candidate_count)
            .filter(|index| mask & (1_u64 << index) != 0)
            .collect::<Vec<_>>();
        let capex_units = selected_segments.iter().try_fold(0_u64, |total, index| {
            total
                .checked_add(scenario.segments[*index].capex_units)
                .ok_or_else(|| "design capex overflowed u64".to_string())
        })?;

        let normal = evaluate_service_scenario(&scenario, &selected_segments, None, None)?;
        if !normal.feasible {
            continue;
        }
        let mut scenario_checks = vec![normal];
        let mut robust = true;
        for failed_segment in &selected_segments {
            let check = evaluate_service_scenario(
                &scenario,
                &selected_segments,
                Some(*failed_segment),
                None,
            )?;
            if !check.feasible {
                robust = false;
                break;
            }
            scenario_checks.push(check);
        }
        if !robust {
            continue;
        }
        for failed_feeder in 0..scenario.feeders.len() {
            let check = evaluate_service_scenario(
                &scenario,
                &selected_segments,
                None,
                Some(failed_feeder),
            )?;
            if !check.feasible {
                robust = false;
                break;
            }
            scenario_checks.push(check);
        }
        if !robust {
            continue;
        }

        robust_feasible_design_count += 1;
        let design = BestDesign {
            selected_segment_ids: selected_segment_ids(&scenario, &selected_segments),
            capex_units,
            scenario_checks,
        };
        match minimum_capex_units {
            None => {
                minimum_capex_units = Some(capex_units);
                best_designs.clear();
                best_designs.push(design);
            }
            Some(current) if capex_units < current => {
                minimum_capex_units = Some(capex_units);
                best_designs.clear();
                best_designs.push(design);
            }
            Some(current) if capex_units == current => best_designs.push(design),
            Some(_) => {}
        }
    }

    for design in &mut best_designs {
        design
            .scenario_checks
            .sort_by_key(|check| check.scenario_id.clone());
    }
    best_designs.sort_by(|left, right| left.selected_segment_ids.cmp(&right.selected_segment_ids));

    Ok(CorridorDesignReport {
        corridor_id: scenario.corridor_id,
        candidate_segment_count: candidate_count,
        subsets_evaluated,
        robust_feasible_design_count,
        minimum_capex_units,
        best_designs,
        claim_ceiling: "Exact minimum-CAPEX enumeration for bounded synthetic corridor graphs and N-1 single-segment and single-feeder outages (one component at a time) only; does not validate simultaneous/common-mode failures, physical WPT coupling, AC/EMT constraints, construction cost, pavement life, traffic demand, uncertainty or field service.",
    })
}

pub fn bundled_fixture() -> Result<Value, String> {
    serde_json::from_str(include_str!(
        "../../../research/wpt-corridor-design-v0/corridor.synthetic-example.json"
    ))
    .map_err(|error| format!("synthetic WPT corridor design scenario JSON: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Value {
        bundled_fixture().unwrap()
    }

    #[test]
    fn known_truth_finds_exact_minimum_cost_n_minus_one_design() {
        let report = evaluate_corridor(&fixture()).unwrap();
        assert_eq!(report.candidate_segment_count, 6);
        assert_eq!(report.subsets_evaluated, 63);
        assert_eq!(report.robust_feasible_design_count, 25);
        assert_eq!(report.minimum_capex_units, Some(16));
        assert_eq!(report.best_designs.len(), 1);
        let best = &report.best_designs[0];
        assert_eq!(
            best.selected_segment_ids,
            vec![
                "a-route-east".to_string(),
                "a-route-west".to_string(),
                "b-route-east".to_string(),
                "b-route-west".to_string()
            ]
        );
        assert_eq!(best.scenario_checks.len(), 7);
        assert!(best.scenario_checks.iter().all(|check| check.feasible));
        assert!(best.scenario_checks.iter().all(|check| {
            check
                .route_service
                .iter()
                .all(|route| route.delivered_kwh == route.required_kwh && route.shortfall_kwh == 0)
        }));
    }

    #[test]
    fn normal_feasibility_without_single_segment_recourse_is_rejected() {
        let mut scenario = fixture();
        scenario["candidate_segments"] =
            Value::Array(vec![scenario["candidate_segments"][4].clone()]);
        let report = evaluate_corridor(&scenario).unwrap();
        assert_eq!(report.minimum_capex_units, None);
        assert!(report.best_designs.is_empty());
    }

    #[test]
    fn selected_segment_and_declared_feeder_outages_are_all_checked() {
        let report = evaluate_corridor(&fixture()).unwrap();
        let best = &report.best_designs[0];
        let failed_segments = best
            .scenario_checks
            .iter()
            .filter_map(|check| check.failed_segment_id.as_deref())
            .collect::<BTreeSet<_>>();
        let failed_feeders = best
            .scenario_checks
            .iter()
            .filter_map(|check| check.failed_feeder_id.as_deref())
            .collect::<BTreeSet<_>>();
        assert_eq!(failed_segments.len(), 4);
        assert_eq!(
            failed_feeders,
            BTreeSet::from(["feeder-north", "feeder-south"])
        );
        assert_eq!(best.scenario_checks.len(), 7);
    }

    #[test]
    fn one_feeder_is_not_mistaken_for_redundant_supply() {
        let mut scenario = fixture();
        for segment in scenario["candidate_segments"].as_array_mut().unwrap() {
            segment["feeder_id"] = Value::String("feeder-north".into());
        }
        let report = evaluate_corridor(&scenario).unwrap();
        assert_eq!(report.minimum_capex_units, None);
        assert!(report.best_designs.is_empty());
    }

    #[test]
    fn unknown_feeder_and_route_references_fail_closed() {
        let mut scenario = fixture();
        scenario["candidate_segments"][0]["feeder_id"] =
            Value::String("unregistered-feeder".into());
        assert!(
            evaluate_corridor(&scenario)
                .unwrap_err()
                .contains("unknown feeder_id")
        );

        let mut scenario = fixture();
        scenario["candidate_segments"][0]["served_route_ids"][0] =
            Value::String("unregistered-route".into());
        assert!(
            evaluate_corridor(&scenario)
                .unwrap_err()
                .contains("unknown route_id")
        );
    }

    #[test]
    fn duplicate_identity_unknown_fields_and_non_synthetic_data_fail_closed() {
        let mut scenario = fixture();
        let duplicate_id = scenario["candidate_segments"][0]["segment_id"].clone();
        scenario["candidate_segments"][1]["segment_id"] = duplicate_id;
        assert!(
            evaluate_corridor(&scenario)
                .unwrap_err()
                .contains("duplicate segment_id")
        );

        let mut scenario = fixture();
        scenario["unmodelled_optimizer_switch"] = Value::Bool(true);
        assert!(
            evaluate_corridor(&scenario)
                .unwrap_err()
                .contains("unknown property")
        );

        let mut scenario = fixture();
        scenario["evidence_kind"] = Value::String("MEASURED".into());
        assert!(
            evaluate_corridor(&scenario)
                .unwrap_err()
                .contains("synthetic known-truth")
        );
    }

    #[test]
    fn input_row_order_does_not_change_minimum_cost_or_selected_ids() {
        let first = evaluate_corridor(&fixture()).unwrap();
        let mut reversed = fixture();
        reversed["feeders"].as_array_mut().unwrap().reverse();
        reversed["routes"].as_array_mut().unwrap().reverse();
        reversed["candidate_segments"]
            .as_array_mut()
            .unwrap()
            .reverse();
        for segment in reversed["candidate_segments"].as_array_mut().unwrap() {
            segment["served_route_ids"]
                .as_array_mut()
                .unwrap()
                .reverse();
        }
        let second = evaluate_corridor(&reversed).unwrap();
        assert_eq!(first.minimum_capex_units, second.minimum_capex_units);
        assert_eq!(first.best_designs, second.best_designs);
        assert_eq!(
            first.robust_feasible_design_count,
            second.robust_feasible_design_count
        );
    }

    #[test]
    fn cost_and_capacity_boundaries_are_strict_integers() {
        let mut scenario = fixture();
        scenario["candidate_segments"][0]["capex_units"] = Value::from(0);
        assert!(
            evaluate_corridor(&scenario)
                .unwrap_err()
                .contains("capex_units")
        );
        let mut scenario = fixture();
        scenario["candidate_segments"][0]["max_deliverable_energy_kwh"] = Value::from("5.0");
        assert!(
            evaluate_corridor(&scenario)
                .unwrap_err()
                .contains("non-negative integer")
        );
    }
}
