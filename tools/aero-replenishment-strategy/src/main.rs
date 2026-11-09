// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Exact finite mission-path enumeration and replenishment Pareto evaluation.
//! This consumes upstream-declared edge costs; it is not an aircraft or battery model.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::{env, fs, process::ExitCode};

const HEADER: [&str; 22] = [
    "case_id",
    "evidence_class",
    "start_node",
    "terminal_node",
    "deadline_slots",
    "capacity_mwh",
    "initial_battery_mwh",
    "reserve_mwh",
    "site_energy_plane_id",
    "battery_energy_plane_id",
    "edge_id",
    "from_node",
    "to_node",
    "edge_kind",
    "mode_id",
    "duration_slots",
    "battery_gain_mwh",
    "aircraft_draw_mwh",
    "site_energy_mwh",
    "asset_id",
    "installed_mass_g",
    "available",
];

const MAX_NODES: usize = 64;
const MAX_EDGES: usize = 512;
const MAX_PATH_EXPANSIONS: usize = 200_000;

#[derive(Clone, Debug, PartialEq, Eq)]
struct Metadata {
    case_id: String,
    evidence_class: String,
    start_node: String,
    terminal_node: String,
    deadline_slots: u64,
    capacity_mwh: u64,
    initial_battery_mwh: u64,
    reserve_mwh: u64,
    site_energy_plane_id: String,
    battery_energy_plane_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Edge {
    edge_id: String,
    from_node: String,
    to_node: String,
    edge_kind: String,
    mode_id: String,
    duration_slots: u64,
    battery_gain_mwh: u64,
    aircraft_draw_mwh: u64,
    site_energy_mwh: u64,
    asset_id: String,
    installed_mass_g: u64,
    available: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Case {
    meta: Metadata,
    edges: Vec<Edge>,
    outgoing: BTreeMap<String, Vec<usize>>,
    unavailable_edge_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Strategy {
    edge_ids: Vec<String>,
    mode_ids: Vec<String>,
    elapsed_slots: u64,
    site_energy_mwh: u64,
    aircraft_draw_mwh: u64,
    installed_mass_g: u64,
    terminal_battery_mwh: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Rejections {
    reserve: usize,
    capacity: usize,
    deadline: usize,
    arithmetic_overflow: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Evaluation {
    all_feasible_paths: Vec<Strategy>,
    pareto_frontier: Vec<Strategy>,
    unavailable_edge_count: usize,
    rejections: Rejections,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PartialPath {
    node: String,
    elapsed_slots: u64,
    battery_mwh: u64,
    site_energy_mwh: u64,
    aircraft_draw_mwh: u64,
    installed_assets: BTreeMap<String, u64>,
    edge_ids: Vec<String>,
    mode_ids: Vec<String>,
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.trim() == value
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._:-".contains(&byte))
}

fn parse_u64(value: &str, field: &str) -> Result<u64, String> {
    if value.is_empty() || value.trim() != value {
        return Err(format!("{field}: empty or untrimmed integer"));
    }
    value
        .parse::<u64>()
        .map_err(|_| format!("{field}: expected non-negative integer"))
}

fn parse_case(input: &str) -> Result<Case, Vec<String>> {
    let mut lines = input.split_terminator('\n');
    let Some(raw_header) = lines.next() else {
        return Err(vec!["input is empty".to_string()]);
    };
    let header = raw_header.strip_suffix('\r').unwrap_or(raw_header);
    if header.split('\t').collect::<Vec<_>>().as_slice() != HEADER.as_slice() {
        return Err(vec![format!(
            "header must contain exactly {} declared columns",
            HEADER.len()
        )]);
    }

    let mut metadata: Option<Vec<String>> = None;
    let mut parsed_meta: Option<Metadata> = None;
    let mut edges = Vec::new();
    let mut edge_ids = BTreeSet::new();
    let mut asset_masses = BTreeMap::<String, u64>::new();
    let mut nodes = BTreeSet::new();
    let mut errors = Vec::new();

    for (offset, raw_line) in lines.enumerate() {
        let line_no = offset + 2;
        let line = raw_line.strip_suffix('\r').unwrap_or(raw_line);
        if line.is_empty() {
            errors.push(format!("line {line_no}: blank rows are not allowed"));
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() != HEADER.len() {
            errors.push(format!(
                "line {line_no}: expected {} columns, found {}",
                HEADER.len(),
                fields.len()
            ));
            continue;
        }

        let row_metadata: Vec<String> =
            fields[..10].iter().map(|field| (*field).to_string()).collect();
        if let Some(first) = &metadata {
            if *first != row_metadata {
                errors.push(format!("line {line_no}: case-level metadata differs from first row"));
                continue;
            }
        } else {
            metadata = Some(row_metadata);
            let numbers = (4..=7)
                .map(|index| parse_u64(fields[index], &format!("line {line_no} column {}", index + 1)))
                .collect::<Result<Vec<_>, _>>();
            let numbers = match numbers {
                Ok(values) => values,
                Err(error) => {
                    errors.push(error);
                    continue;
                }
            };

            if !valid_id(fields[0])
                || fields[1] != "SYNTHETIC_KNOWN_TRUTH"
                || !valid_id(fields[2])
                || !valid_id(fields[3])
                || fields[2] == fields[3]
                || numbers[0] == 0
                || numbers[1] == 0
                || numbers[2] > numbers[1]
                || numbers[3] > numbers[2]
                || !valid_id(fields[8])
                || !valid_id(fields[9])
                || fields[9] != "BATTERY_TERMINAL"
            {
                errors.push(format!(
                    "line {line_no}: invalid case metadata, evidence class, battery plane or energy bounds"
                ));
                continue;
            }

            parsed_meta = Some(Metadata {
                case_id: fields[0].to_string(),
                evidence_class: fields[1].to_string(),
                start_node: fields[2].to_string(),
                terminal_node: fields[3].to_string(),
                deadline_slots: numbers[0],
                capacity_mwh: numbers[1],
                initial_battery_mwh: numbers[2],
                reserve_mwh: numbers[3],
                site_energy_plane_id: fields[8].to_string(),
                battery_energy_plane_id: fields[9].to_string(),
            });
        }

        let edge_id = fields[10];
        let from_node = fields[11];
        let to_node = fields[12];
        let edge_kind = fields[13];
        let mode_id = fields[14];
        let asset_id = fields[19];

        if !valid_id(edge_id)
            || !valid_id(from_node)
            || !valid_id(to_node)
            || from_node == to_node
            || !valid_id(mode_id)
            || !(edge_kind == "MISSION_LEG" || edge_kind == "REPLENISHMENT")
            || !(asset_id == "NONE" || valid_id(asset_id))
        {
            errors.push(format!("line {line_no}: invalid edge, node, mode, asset or edge kind"));
            continue;
        }
        if !edge_ids.insert(edge_id.to_string()) {
            errors.push(format!("line {line_no}: duplicate edge_id {edge_id}"));
            continue;
        }

        let numbers = (15..=18)
            .chain(std::iter::once(20))
            .map(|index| parse_u64(fields[index], &format!("line {line_no} column {}", index + 1)))
            .collect::<Result<Vec<_>, _>>();
        let numbers = match numbers {
            Ok(values) => values,
            Err(error) => {
                errors.push(error);
                continue;
            }
        };
        let available = match fields[21] {
            "true" => true,
            "false" => false,
            other => {
                errors.push(format!("line {line_no}: available must be true or false, found {other:?}"));
                continue;
            }
        };

        let duration_slots = numbers[0];
        let battery_gain_mwh = numbers[1];
        let aircraft_draw_mwh = numbers[2];
        let site_energy_mwh = numbers[3];
        let installed_mass_g = numbers[4];

        if duration_slots == 0 {
            errors.push(format!("line {line_no}: duration_slots must be positive"));
            continue;
        }
        if edge_kind == "MISSION_LEG" {
            if battery_gain_mwh != 0
                || site_energy_mwh != 0
                || asset_id != "NONE"
                || installed_mass_g != 0
            {
                errors.push(format!(
                    "line {line_no}: MISSION_LEG must not create battery energy, site energy or installed asset mass"
                ));
                continue;
            }
        } else if battery_gain_mwh == 0 || site_energy_mwh == 0 {
            errors.push(format!(
                "line {line_no}: REPLENISHMENT must declare positive battery-terminal gain and source-side energy"
            ));
            continue;
        }
        if asset_id == "NONE" && installed_mass_g != 0 {
            errors.push(format!(
                "line {line_no}: asset_id NONE requires zero installed_mass_g"
            ));
            continue;
        }
        if asset_id != "NONE" {
            if let Some(existing) = asset_masses.get(asset_id) {
                if *existing != installed_mass_g {
                    errors.push(format!(
                        "line {line_no}: installed asset {asset_id} has conflicting mass values"
                    ));
                    continue;
                }
            } else {
                asset_masses.insert(asset_id.to_string(), installed_mass_g);
            }
        }

        nodes.insert(from_node.to_string());
        nodes.insert(to_node.to_string());
        edges.push(Edge {
            edge_id: edge_id.to_string(),
            from_node: from_node.to_string(),
            to_node: to_node.to_string(),
            edge_kind: edge_kind.to_string(),
            mode_id: mode_id.to_string(),
            duration_slots,
            battery_gain_mwh,
            aircraft_draw_mwh,
            site_energy_mwh,
            asset_id: asset_id.to_string(),
            installed_mass_g,
            available,
        });
        if edges.len() > MAX_EDGES {
            errors.push(format!("case exceeds {MAX_EDGES} mission edges"));
            break;
        }
    }

    let Some(meta) = parsed_meta else {
        return Err(if errors.is_empty() {
            vec!["no mission edge rows found".to_string()]
        } else {
            errors
        });
    };
    if !nodes.contains(&meta.start_node) || !nodes.contains(&meta.terminal_node) {
        errors.push("start_node and terminal_node must occur in the edge graph".to_string());
    }
    if nodes.len() > MAX_NODES {
        errors.push(format!("case exceeds {MAX_NODES} graph nodes"));
    }
    if !errors.is_empty() {
        return Err(errors);
    }

    // Only available edges can be traversed, so cycle detection is over that directed graph.
    let mut outgoing = BTreeMap::<String, Vec<usize>>::new();
    let mut indegree = BTreeMap::<String, usize>::new();
    for node in &nodes {
        indegree.insert(node.clone(), 0);
    }
    for (index, edge) in edges.iter().enumerate() {
        if edge.available {
            outgoing.entry(edge.from_node.clone()).or_default().push(index);
            *indegree.entry(edge.to_node.clone()).or_default() += 1;
        }
    }
    for indices in outgoing.values_mut() {
        indices.sort_by(|left, right| edges[*left].edge_id.cmp(&edges[*right].edge_id));
    }

    let mut queue: VecDeque<String> = indegree
        .iter()
        .filter(|(_, degree)| **degree == 0)
        .map(|(node, _)| node.clone())
        .collect();
    let mut visited = 0usize;
    while let Some(node) = queue.pop_front() {
        visited += 1;
        if let Some(indices) = outgoing.get(&node) {
            for index in indices {
                let next = &edges[*index].to_node;
                let degree = indegree.get_mut(next).expect("node was registered");
                *degree -= 1;
                if *degree == 0 {
                    queue.push_back(next.clone());
                }
            }
        }
    }
    if visited != nodes.len() {
        return Err(vec!["available mission edges contain a directed cycle".to_string()]);
    }
    if outgoing.contains_key(&meta.terminal_node) {
        return Err(vec!["terminal_node must not have available outgoing edges".to_string()]);
    }

    let unavailable_edge_count = edges.iter().filter(|edge| !edge.available).count();
    Ok(Case {
        meta,
        edges,
        outgoing,
        unavailable_edge_count,
    })
}

fn path_asset_mass(assets: &BTreeMap<String, u64>) -> Option<u64> {
    assets.values().try_fold(0u64, |sum, mass| sum.checked_add(*mass))
}

fn strategy_key(strategy: &Strategy) -> String {
    strategy.edge_ids.join(">")
}

fn strategy_score(strategy: &Strategy) -> (u64, u64, u64, u64, std::cmp::Reverse<u64>, String) {
    (
        strategy.elapsed_slots,
        strategy.site_energy_mwh,
        strategy.aircraft_draw_mwh,
        strategy.installed_mass_g,
        std::cmp::Reverse(strategy.terminal_battery_mwh),
        strategy_key(strategy),
    )
}

fn dominates(left: &Strategy, right: &Strategy) -> bool {
    let no_worse = left.elapsed_slots <= right.elapsed_slots
        && left.site_energy_mwh <= right.site_energy_mwh
        && left.aircraft_draw_mwh <= right.aircraft_draw_mwh
        && left.installed_mass_g <= right.installed_mass_g
        && left.terminal_battery_mwh >= right.terminal_battery_mwh;
    if !no_worse {
        return false;
    }
    left.elapsed_slots < right.elapsed_slots
        || left.site_energy_mwh < right.site_energy_mwh
        || left.aircraft_draw_mwh < right.aircraft_draw_mwh
        || left.installed_mass_g < right.installed_mass_g
        || left.terminal_battery_mwh > right.terminal_battery_mwh
}

fn pareto_frontier(mut strategies: Vec<Strategy>) -> Vec<Strategy> {
    strategies.sort_by_key(strategy_score);
    let mut frontier = Vec::new();
    for candidate in strategies {
        if frontier.iter().any(|existing| dominates(existing, &candidate)) {
            continue;
        }
        frontier.retain(|existing| !dominates(&candidate, existing));
        frontier.push(candidate);
    }
    frontier.sort_by_key(strategy_score);
    frontier
}

fn walk(
    case: &Case,
    state: &PartialPath,
    strategies: &mut Vec<Strategy>,
    rejections: &mut Rejections,
    expansions: &mut usize,
) -> Result<(), String> {
    if state.node == case.meta.terminal_node {
        let installed_mass_g = path_asset_mass(&state.installed_assets)
            .ok_or_else(|| "installed asset mass overflow".to_string())?;
        let energy_lhs = case
            .meta
            .initial_battery_mwh
            .checked_add(
                state
                    .edge_ids
                    .iter()
                    .filter_map(|id| case.edges.iter().find(|edge| &edge.edge_id == id))
                    .try_fold(0u64, |sum, edge| sum.checked_add(edge.battery_gain_mwh))
                    .ok_or_else(|| "battery gain sum overflow".to_string())?,
            )
            .ok_or_else(|| "battery energy balance overflow".to_string())?;
        let energy_rhs = state
            .battery_mwh
            .checked_add(state.aircraft_draw_mwh)
            .ok_or_else(|| "battery draw sum overflow".to_string())?;
        if energy_lhs != energy_rhs {
            return Err("internal energy balance invariant failed".to_string());
        }
        strategies.push(Strategy {
            edge_ids: state.edge_ids.clone(),
            mode_ids: state.mode_ids.clone(),
            elapsed_slots: state.elapsed_slots,
            site_energy_mwh: state.site_energy_mwh,
            aircraft_draw_mwh: state.aircraft_draw_mwh,
            installed_mass_g,
            terminal_battery_mwh: state.battery_mwh,
        });
        return Ok(());
    }

    let Some(outgoing) = case.outgoing.get(&state.node) else {
        return Ok(());
    };

    for edge_index in outgoing {
        *expansions += 1;
        if *expansions > MAX_PATH_EXPANSIONS {
            return Err(format!(
                "path enumeration exceeded {MAX_PATH_EXPANSIONS} expansions; no truncated result is emitted"
            ));
        }
        let edge = &case.edges[*edge_index];

        let Some(elapsed_slots) = state.elapsed_slots.checked_add(edge.duration_slots) else {
            rejections.arithmetic_overflow += 1;
            continue;
        };
        if elapsed_slots > case.meta.deadline_slots {
            rejections.deadline += 1;
            continue;
        }

        let next_battery_i128 = i128::from(state.battery_mwh)
            + i128::from(edge.battery_gain_mwh)
            - i128::from(edge.aircraft_draw_mwh);
        if next_battery_i128 < 0 {
            rejections.reserve += 1;
            continue;
        }
        if next_battery_i128 > i128::from(case.meta.capacity_mwh) {
            rejections.capacity += 1;
            continue;
        }
        let next_battery_mwh = next_battery_i128 as u64;
        if next_battery_mwh < case.meta.reserve_mwh {
            rejections.reserve += 1;
            continue;
        }

        let Some(site_energy_mwh) = state.site_energy_mwh.checked_add(edge.site_energy_mwh) else {
            rejections.arithmetic_overflow += 1;
            continue;
        };
        let Some(aircraft_draw_mwh) = state.aircraft_draw_mwh.checked_add(edge.aircraft_draw_mwh) else {
            rejections.arithmetic_overflow += 1;
            continue;
        };

        let mut installed_assets = state.installed_assets.clone();
        if edge.asset_id != "NONE" {
            if let Some(existing) = installed_assets.get(&edge.asset_id) {
                if *existing != edge.installed_mass_g {
                    return Err(format!(
                        "path encountered inconsistent mass for asset {}",
                        edge.asset_id
                    ));
                }
            } else {
                installed_assets.insert(edge.asset_id.clone(), edge.installed_mass_g);
            }
        }

        let mut next = state.clone();
        next.node = edge.to_node.clone();
        next.elapsed_slots = elapsed_slots;
        next.battery_mwh = next_battery_mwh;
        next.site_energy_mwh = site_energy_mwh;
        next.aircraft_draw_mwh = aircraft_draw_mwh;
        next.installed_assets = installed_assets;
        next.edge_ids.push(edge.edge_id.clone());
        next.mode_ids.push(edge.mode_id.clone());
        walk(case, &next, strategies, rejections, expansions)?;
    }
    Ok(())
}

fn evaluate(case: &Case) -> Result<Evaluation, String> {
    let initial = PartialPath {
        node: case.meta.start_node.clone(),
        elapsed_slots: 0,
        battery_mwh: case.meta.initial_battery_mwh,
        site_energy_mwh: 0,
        aircraft_draw_mwh: 0,
        installed_assets: BTreeMap::new(),
        edge_ids: Vec::new(),
        mode_ids: Vec::new(),
    };
    let mut all_feasible_paths = Vec::new();
    let mut rejections = Rejections::default();
    let mut expansions = 0usize;
    walk(
        case,
        &initial,
        &mut all_feasible_paths,
        &mut rejections,
        &mut expansions,
    )?;
    let pareto_frontier = pareto_frontier(all_feasible_paths.clone());
    Ok(Evaluation {
        all_feasible_paths,
        pareto_frontier,
        unavailable_edge_count: case.unavailable_edge_count,
        rejections,
    })
}

fn run(path: &str) -> Result<(), String> {
    let input = fs::read_to_string(path)
        .map_err(|error| format!("cannot read {path}: {error}"))?;
    let case = parse_case(&input).map_err(|errors| errors.join("\n"))?;
    let result = evaluate(&case)?;

    println!("case_id={}", case.meta.case_id);
    println!("evidence_class={}", case.meta.evidence_class);
    println!("site_energy_plane_id={}", case.meta.site_energy_plane_id);
    println!("battery_energy_plane_id={}", case.meta.battery_energy_plane_id);
    println!("deadline_slots={}", case.meta.deadline_slots);
    println!("initial_battery_mwh={}", case.meta.initial_battery_mwh);
    println!("reserve_mwh={}", case.meta.reserve_mwh);
    println!("capacity_mwh={}", case.meta.capacity_mwh);
    println!("unavailable_edge_count={}", result.unavailable_edge_count);
    println!("feasible_complete_path_count={}", result.all_feasible_paths.len());
    println!("pareto_strategy_count={}", result.pareto_frontier.len());
    println!(
        "rejections=reserve:{};capacity:{};deadline:{};overflow:{}",
        result.rejections.reserve,
        result.rejections.capacity,
        result.rejections.deadline,
        result.rejections.arithmetic_overflow
    );

    if result.pareto_frontier.is_empty() {
        println!("disposition=NO_FEASIBLE_STRATEGY");
    } else {
        println!("disposition=SYNTHETIC_PARETO_STRATEGIES_NO_OPERATIONAL_AUTHORITY");
        for (index, strategy) in result.pareto_frontier.iter().enumerate() {
            println!(
                "pareto[{}]=elapsed_slots:{};site_energy_mwh:{};aircraft_draw_mwh:{};installed_mass_g:{};terminal_battery_mwh:{};modes:{};edges:{}",
                index + 1,
                strategy.elapsed_slots,
                strategy.site_energy_mwh,
                strategy.aircraft_draw_mwh,
                strategy.installed_mass_g,
                strategy.terminal_battery_mwh,
                strategy.mode_ids.join(">"),
                strategy.edge_ids.join(">")
            );
        }
    }
    println!("operational_permit=false");
    println!("authority=NONE_SYNTHETIC_MISSION_PATH_EVALUATION_ONLY");
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("usage: aero-replenishment-strategy <mission-edges.tsv>");
        return ExitCode::from(2);
    }
    match run(&args[1]) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("ERROR: {error}");
            ExitCode::from(1)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str =
        include_str!("../../../research/aero-replenishment-strategy-v0/synthetic-mission-edges.tsv");

    fn parsed(input: &str) -> Case {
        parse_case(input).unwrap_or_else(|errors| panic!("{}", errors.join("\n")))
    }

    fn by_edges<'a>(strategies: &'a [Strategy], expected: &[&str]) -> &'a Strategy {
        strategies
            .iter()
            .find(|strategy| {
                strategy.edge_ids.iter().map(String::as_str).collect::<Vec<_>>() == expected
            })
            .expect("expected strategy path is present")
    }

    // Deliberately simple, test-only exhaustive oracle: enumerate edge sequences first,
    // then independently recompute all path balances from the selected sequence.
    fn oracle(case: &Case) -> Vec<Strategy> {
        fn collect_paths(
            case: &Case,
            node: &str,
            prefix: &mut Vec<usize>,
            paths: &mut Vec<Vec<usize>>,
        ) {
            if node == case.meta.terminal_node {
                paths.push(prefix.clone());
                return;
            }
            if let Some(indices) = case.outgoing.get(node) {
                for index in indices {
                    prefix.push(*index);
                    collect_paths(case, &case.edges[*index].to_node, prefix, paths);
                    prefix.pop();
                }
            }
        }

        let mut edge_paths = Vec::new();
        collect_paths(
            case,
            &case.meta.start_node,
            &mut Vec::new(),
            &mut edge_paths,
        );

        let mut result = Vec::new();
        for path in edge_paths {
            let mut battery = i128::from(case.meta.initial_battery_mwh);
            let mut elapsed = 0u64;
            let mut site = 0u64;
            let mut draw = 0u64;
            let mut assets = BTreeMap::<String, u64>::new();
            let mut edge_ids = Vec::new();
            let mut mode_ids = Vec::new();
            let mut valid = true;

            for index in &path {
                let edge = &case.edges[*index];
                elapsed = match elapsed.checked_add(edge.duration_slots) {
                    Some(value) => value,
                    None => {
                        valid = false;
                        break;
                    }
                };
                battery = battery + i128::from(edge.battery_gain_mwh)
                    - i128::from(edge.aircraft_draw_mwh);
                site = match site.checked_add(edge.site_energy_mwh) {
                    Some(value) => value,
                    None => {
                        valid = false;
                        break;
                    }
                };
                draw = match draw.checked_add(edge.aircraft_draw_mwh) {
                    Some(value) => value,
                    None => {
                        valid = false;
                        break;
                    }
                };
                if edge.asset_id != "NONE" {
                    assets.insert(edge.asset_id.clone(), edge.installed_mass_g);
                }
                edge_ids.push(edge.edge_id.clone());
                mode_ids.push(edge.mode_id.clone());
                if battery < i128::from(case.meta.reserve_mwh)
                    || battery > i128::from(case.meta.capacity_mwh)
                    || elapsed > case.meta.deadline_slots
                {
                    valid = false;
                    break;
                }
            }
            if !valid {
                continue;
            }
            let mass = assets.values().copied().sum::<u64>();
            result.push(Strategy {
                edge_ids,
                mode_ids,
                elapsed_slots: elapsed,
                site_energy_mwh: site,
                aircraft_draw_mwh: draw,
                installed_mass_g: mass,
                terminal_battery_mwh: battery as u64,
            });
        }
        result.sort_by_key(strategy_score);
        result
    }

    #[test]
    fn exact_solver_matches_independent_exhaustive_path_oracle() {
        let case = parsed(FIXTURE);
        let evaluated = evaluate(&case).expect("evaluation");
        let expected = oracle(&case);
        let actual_keys: BTreeSet<String> = evaluated.all_feasible_paths.iter().map(strategy_key).collect();
        let expected_keys: BTreeSet<String> = expected.iter().map(strategy_key).collect();
        assert_eq!(actual_keys, expected_keys);
        assert_eq!(evaluated.all_feasible_paths.len(), expected.len());
    }

    #[test]
    fn battery_only_path_preserves_hard_reserve_without_source_energy() {
        let case = parsed(FIXTURE);
        let result = evaluate(&case).expect("evaluation");
        let strategy = by_edges(
            &result.all_feasible_paths,
            &[
                "mission_leg_1",
                "battery_only_continue",
                "battery_leg",
                "final_mission_leg",
            ],
        );
        assert_eq!(strategy.terminal_battery_mwh, 25);
        assert_eq!(strategy.elapsed_slots, 4);
        assert_eq!(strategy.site_energy_mwh, 0);
        assert_eq!(strategy.aircraft_draw_mwh, 90);
        assert_eq!(strategy.installed_mass_g, 0);
    }

    #[test]
    fn battery_exchange_is_not_free_energy_and_counts_source_boundary() {
        let case = parsed(FIXTURE);
        let result = evaluate(&case).expect("evaluation");
        let strategy = by_edges(
            &result.all_feasible_paths,
            &[
                "mission_leg_1",
                "battery_exchange",
                "battery_leg",
                "final_mission_leg",
            ],
        );
        assert_eq!(strategy.terminal_battery_mwh, 73);
        assert_eq!(strategy.site_energy_mwh, 55);
        assert_eq!(strategy.elapsed_slots, 4);
        assert_eq!(strategy.installed_mass_g, 80);
    }

    #[test]
    fn mixed_contact_and_cpt_path_is_a_valid_architecture_strategy() {
        let case = parsed(FIXTURE);
        let result = evaluate(&case).expect("evaluation");
        let strategy = by_edges(
            &result.all_feasible_paths,
            &[
                "mission_leg_1",
                "conductive_contact",
                "cpt_dock",
                "final_mission_leg",
            ],
        );
        assert_eq!(strategy.terminal_battery_mwh, 82);
        assert_eq!(strategy.elapsed_slots, 7);
        assert_eq!(strategy.site_energy_mwh, 78);
        assert_eq!(strategy.installed_mass_g, 400);
    }

    #[test]
    fn same_installed_receiver_is_charged_once_across_multiple_sessions() {
        let case = parsed(FIXTURE);
        let result = evaluate(&case).expect("evaluation");
        let strategy = by_edges(
            &result.all_feasible_paths,
            &[
                "mission_leg_1",
                "optical_beam",
                "optical_top_up",
                "final_mission_leg",
            ],
        );
        assert_eq!(strategy.installed_mass_g, 2_800);
        assert_eq!(strategy.site_energy_mwh, 240);
        assert_eq!(strategy.terminal_battery_mwh, 80);
    }

    #[test]
    fn pareto_frontier_contains_multiple_non_dominated_service_strategies() {
        let case = parsed(FIXTURE);
        let result = evaluate(&case).expect("evaluation");
        assert!(result.pareto_frontier.len() >= 3);
        for candidate in &result.pareto_frontier {
            assert!(
                !result
                    .all_feasible_paths
                    .iter()
                    .any(|other| strategy_key(other) != strategy_key(candidate) && dominates(other, candidate))
            );
        }
    }

    #[test]
    fn no_complete_path_is_returned_when_initial_leg_breaks_reserve() {
        let input = FIXTURE.replace("\t115\t25\tSITE_GRID_REAL_ENERGY", "\t60\t25\tSITE_GRID_REAL_ENERGY");
        let case = parsed(&input);
        let result = evaluate(&case).expect("evaluation");
        assert!(result.all_feasible_paths.is_empty());
        assert!(result.pareto_frontier.is_empty());
        assert!(result.rejections.reserve > 0);
    }

    #[test]
    fn capacity_overflow_rejects_replenishment_but_keeps_battery_only() {
        let input = FIXTURE
            .replace("\t120\t115\t25\tSITE_GRID_REAL_ENERGY", "\t80\t80\t25\tSITE_GRID_REAL_ENERGY")
            .replace("\tMISSION_LEG_1\t1\t0\t40\t", "\tMISSION_LEG_1\t1\t0\t10\t")
            .replace("\tMISSION_LEG_FINAL\t1\t0\t30\t", "\tMISSION_LEG_FINAL\t1\t0\t20\t");
        let case = parsed(&input);
        let result = evaluate(&case).expect("evaluation");
        assert!(result.rejections.capacity > 0);
        assert!(!result.all_feasible_paths.is_empty());
        assert!(result.all_feasible_paths.iter().all(|strategy| {
            !strategy.edge_ids.iter().any(|id| {
                matches!(id.as_str(), "conductive_contact" | "robotic_contact" | "battery_exchange" | "optical_beam" | "ipt_dock" | "cpt_dock" | "optical_top_up")
            })
        }));
        assert!(result.all_feasible_paths.iter().any(|strategy| {
            strategy.edge_ids == ["mission_leg_1", "battery_only_continue", "battery_leg", "final_mission_leg"]
        }));
    }

    #[test]
    fn deadline_is_a_hard_constraint_for_every_complete_path() {
        let input = FIXTURE.replace("\tTERMINAL\t8\t120", "\tTERMINAL\t3\t120");
        let case = parsed(&input);
        let result = evaluate(&case).expect("evaluation");
        assert!(result.all_feasible_paths.is_empty());
        assert!(result.rejections.deadline > 0);
    }

    #[test]
    fn unavailable_modes_never_appear_in_a_selected_path() {
        let unavailable_edge = FIXTURE
            .lines()
            .nth(3)
            .expect("conductive replenishment edge")
            .replace("conductive_contact", "unavailable_contact")
            .replace("\ttrue", "\tfalse");
        let input = format!("{FIXTURE}{unavailable_edge}\n");
        let case = parsed(&input);
        let result = evaluate(&case).expect("evaluation");
        assert_eq!(result.unavailable_edge_count, 1);
        assert!(result.all_feasible_paths.iter().all(|strategy| !strategy.edge_ids.contains(&"unavailable_contact".to_string())));
    }

    #[test]
    fn available_graph_cycles_are_rejected() {
        let extra = FIXTURE.lines().nth(1).expect("mission leg row")
            .replace("mission_leg_1", "cycle_edge")
            .replace("START\tCHECKPOINT_1", "TERMINAL\tSTART")
            .replace("MISSION_LEG_1", "CYCLE_CONTROL");
        let cyclic = format!("{FIXTURE}{extra}\n");
        assert!(parse_case(&cyclic).is_err());
    }

    #[test]
    fn duplicate_edge_ids_are_rejected() {
        let duplicate = format!(
            "{FIXTURE}{}\n",
            FIXTURE.lines().nth(1).expect("mission leg row")
        );
        assert!(parse_case(&duplicate).is_err());
    }

    #[test]
    fn inconsistent_installed_asset_mass_is_rejected() {
        let conflict = FIXTURE.replacen(
            "OPTICAL_RX\t2800\ttrue",
            "OPTICAL_RX\t2801\ttrue",
            1,
        );
        assert!(parse_case(&conflict).is_err());
    }

    #[test]
    fn mission_leg_cannot_hide_source_energy_or_receiver_mass() {
        let invalid = FIXTURE.replace(
            "MISSION_LEG_1\t1\t0\t40\t0\tNONE\t0\ttrue",
            "MISSION_LEG_1\t1\t0\t40\t1\tNONE\t0\ttrue",
        );
        assert!(parse_case(&invalid).is_err());
    }

    #[test]
    fn row_permutation_preserves_feasible_paths_and_frontier() {
        let mut rows: Vec<&str> = FIXTURE.lines().collect();
        let header = rows.remove(0);
        rows.reverse();
        let permuted = format!("{header}\n{}\n", rows.join("\n"));
        let original = evaluate(&parsed(FIXTURE)).expect("original");
        let reordered = evaluate(&parsed(&permuted)).expect("permuted");
        assert_eq!(original, reordered);
    }

    #[test]
    fn energy_balance_closes_for_every_complete_path() {
        let case = parsed(FIXTURE);
        let result = evaluate(&case).expect("evaluation");
        for strategy in &result.all_feasible_paths {
            assert_eq!(
                case.meta.initial_battery_mwh
                    + strategy
                        .edge_ids
                        .iter()
                        .map(|id| case.edges.iter().find(|edge| &edge.edge_id == id).unwrap().battery_gain_mwh)
                        .sum::<u64>(),
                strategy.terminal_battery_mwh + strategy.aircraft_draw_mwh
            );
        }
    }
}
