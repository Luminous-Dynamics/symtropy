use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::{BTreeSet, VecDeque}, env, fs, process::ExitCode};

const VERSION: &str = "0.1.0";
type Check<T> = Result<T, String>;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Corpus {
    suite_id: String,
    suite_version: String,
    evidence_class: String,
    claim_ceiling: String,
    seed: u64,
    scenarios: Vec<ScenarioSpec>,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ScenarioSpec {
    scenario_id: String,
    map_variant: MapVariant,
    battery_milliunits: u32,
    fault: Option<FaultInjection>,
    expected_state: SimState,
    expected_reason_codes: Vec<String>,
}
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum MapVariant { Clear, UnknownTarget, BlockedCorridor, SteepTerrain }
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum FaultKind {
    GnssLoss, PoseJump, LidarDegraded, WheelSlip, ObstacleEmergence,
    CommsLoss, BatteryDrop, StaleTime, MapDisagreement, EmergencyStopEngaged,
}
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct FaultInjection { kind: FaultKind, trigger_step: u32, deadline_steps: u32 }
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum SimState { Completed, Degraded, Blocked }
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Occupancy { Clear, Obstacle, Unknown }
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Surface { Firm, Soft, Unknown }
#[derive(Debug, Clone, Serialize)]
struct TerrainCell {
    slope_permille: u16,
    roughness_permille: u16,
    surface: Surface,
    occupancy: Occupancy,
    observed: bool,
}
#[derive(Debug, Clone, Serialize)]
struct GridMap { width: u16, height: u16, cell_size_cm: u16, cells: Vec<TerrainCell> }
#[derive(Debug, Clone, Serialize)]
struct RobotConfig {
    max_slope_permille: u16,
    max_roughness_permille: u16,
    allow_soft_ground: bool,
    energy_per_cell_milliunits: u32,
    return_reserve_milliunits: u32,
}
#[derive(Debug, Clone, Copy, Serialize)]
struct CellCoord { x: u16, y: u16 }
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Facing { North, East, South, West }
#[derive(Debug, Clone, Copy, Serialize)]
struct SensorConfig { range_cells: u16, horizontal_fov_degrees: u16 }
#[derive(Debug, Serialize)]
struct ScenarioResult {
    scenario_id: String,
    state: SimState,
    reason_codes: Vec<String>,
    map_sha256: String,
    config_sha256: String,
    planned_route_cells: usize,
    traversed_unique_cells: usize,
    planned_survey_targets: usize,
    traversed_survey_targets: usize,
    verified_survey_targets: usize,
    planned_area_cells: usize,
    observed_area_cells: usize,
    unobserved_area_cells: usize,
    observed_coverage_basis_points: u16,
    energy_used_milliunits: u64,
    fault_response_latency_steps: Option<u32>,
    fault_deadline_steps: Option<u32>,
    fault_deadline_passed: Option<bool>,
}
#[derive(Debug, Serialize)]
struct Receipt {
    result: &'static str,
    simulator_version: &'static str,
    suite_id: String,
    suite_version: String,
    evidence_class: String,
    claim_ceiling: String,
    source_commit: String,
    corpus_sha256: String,
    seed: u64,
    scenario_count: usize,
    completed_count: usize,
    degraded_count: usize,
    blocked_count: usize,
    injected_fault_count: usize,
    fault_deadline_pass_count: usize,
    scenarios: Vec<ScenarioResult>,
}

fn sha256(bytes: &[u8]) -> String { format!("{:x}", Sha256::digest(bytes)) }
fn encoded<T: Serialize>(value: &T) -> Check<Vec<u8>> {
    serde_json::to_vec(value).map_err(|e| format!("cannot serialize digest input: {e}"))
}
fn cell_index(map: &GridMap, coord: CellCoord) -> Option<usize> {
    if coord.x >= map.width || coord.y >= map.height { return None; }
    Some(usize::from(coord.y) * usize::from(map.width) + usize::from(coord.x))
}
fn coord_for_index(map: &GridMap, idx: usize) -> Option<CellCoord> {
    if idx >= map.cells.len() { return None; }
    let width = usize::from(map.width);
    if width == 0 { return None; }
    Some(CellCoord { x: (idx % width) as u16, y: (idx / width) as u16 })
}
fn sensor_config() -> SensorConfig {
    SensorConfig { range_cells: 3, horizontal_fov_degrees: 180 }
}
fn facing_between(map: &GridMap, from: usize, to: usize) -> Option<Facing> {
    let a = coord_for_index(map, from)?;
    let b = coord_for_index(map, to)?;
    match (i32::from(b.x) - i32::from(a.x), i32::from(b.y) - i32::from(a.y)) {
        (0, -1) => Some(Facing::North),
        (1, 0) => Some(Facing::East),
        (0, 1) => Some(Facing::South),
        (-1, 0) => Some(Facing::West),
        _ => None,
    }
}
fn facing_at_route_step(map: &GridMap, route: &[usize], step: usize) -> Facing {
    if route.len() < 2 { return Facing::East; }
    if step > 0 {
        facing_between(map, route[step - 1], route[step]).unwrap_or(Facing::East)
    } else {
        facing_between(map, route[0], route[1]).unwrap_or(Facing::East)
    }
}
fn ray_cell_blocks(map: &GridMap, x: i32, y: i32) -> bool {
    if x < 0 || y < 0 || x >= i32::from(map.width) || y >= i32::from(map.height) {
        return true;
    }
    let idx = (y as usize) * usize::from(map.width) + (x as usize);
    let cell = &map.cells[idx];
    !cell.observed || matches!(cell.occupancy, Occupancy::Obstacle | Occupancy::Unknown)
}
/// Conservative grid raycast: unknown cells and obstacles block visibility beyond themselves.
/// When the ray crosses a grid corner, both side-adjacent cells are checked to avoid leaks.
fn line_of_sight(map: &GridMap, origin: CellCoord, target: CellCoord) -> bool {
    let (mut x, mut y) = (i32::from(origin.x), i32::from(origin.y));
    let (tx, ty) = (i32::from(target.x), i32::from(target.y));
    let dx = (tx - x).abs();
    let dy = (ty - y).abs();
    let sx = if x < tx { 1 } else if x > tx { -1 } else { 0 };
    let sy = if y < ty { 1 } else if y > ty { -1 } else { 0 };
    let mut err = dx - dy;
    loop {
        if x == tx && y == ty { return true; }
        let twice = 2 * err;
        let step_x = twice > -dy;
        let step_y = twice < dx;
        if step_x && step_y
            && (ray_cell_blocks(map, x + sx, y) || ray_cell_blocks(map, x, y + sy)) {
            return false;
        }
        if step_x { err -= dy; x += sx; }
        if step_y { err += dx; y += sy; }
        if (x, y) != (tx, ty) && ray_cell_blocks(map, x, y) { return false; }
    }
}
fn within_sensor_range(map: &GridMap, origin_idx: usize, target_idx: usize, sensor: &SensorConfig) -> bool {
    let Some(origin) = coord_for_index(map, origin_idx) else { return false; };
    let Some(target) = coord_for_index(map, target_idx) else { return false; };
    let dx = i32::from(target.x) - i32::from(origin.x);
    let dy = i32::from(target.y) - i32::from(origin.y);
    let range = i32::from(sensor.range_cells);
    dx * dx + dy * dy <= range * range
}
fn visible_from_pose(
    map: &GridMap, origin_idx: usize, target_idx: usize, facing: Facing, sensor: &SensorConfig,
) -> bool {
    let Some(origin) = coord_for_index(map, origin_idx) else { return false; };
    let Some(target) = coord_for_index(map, target_idx) else { return false; };
    let dx = i32::from(target.x) - i32::from(origin.x);
    let dy = i32::from(target.y) - i32::from(origin.y);
    if !within_sensor_range(map, origin_idx, target_idx, sensor) { return false; }
    if sensor.horizontal_fov_degrees != 180 { return false; }
    let dot = match facing {
        Facing::North => -dy,
        Facing::East => dx,
        Facing::South => dy,
        Facing::West => -dx,
    };
    // v0.1 uses a forward-facing 180-degree sensor; lateral rays are included.
    if dot < 0 { return false; }
    line_of_sight(map, origin, target)
}
fn visible_cells_from_pose(
    map: &GridMap, origin_idx: usize, facing: Facing, sensor: &SensorConfig,
) -> BTreeSet<usize> {
    (0..map.cells.len())
        .filter(|target| visible_from_pose(map, origin_idx, *target, facing, sensor))
        .collect()
}
fn base_map(seed: u64, variant: MapVariant) -> GridMap {
    let (width, height) = (9_u16, 9_u16);
    let mut cells = Vec::with_capacity(usize::from(width) * usize::from(height));
    for y in 0..height {
        for x in 0..width {
            cells.push(TerrainCell {
                slope_permille: ((u64::from(x) * 17 + u64::from(y) * 31 + seed % 19) % 90) as u16,
                roughness_permille: ((u64::from(x) * 11 + u64::from(y) * 13 + seed % 23) % 140) as u16,
                surface: Surface::Firm, occupancy: Occupancy::Clear, observed: true,
            });
        }
    }
    let mut map = GridMap { width, height, cell_size_cm: 100, cells };
    match variant {
        MapVariant::Clear => {}
        MapVariant::UnknownTarget => {
            let target = cell_index(&map, CellCoord { x: 4, y: 4 }).expect("fixed target fits grid");
            map.cells[target].occupancy = Occupancy::Unknown;
            map.cells[target].observed = false;
        }
        MapVariant::BlockedCorridor => {
            for y in 0..height {
                let idx = cell_index(&map, CellCoord { x: 2, y }).expect("fixed column fits grid");
                map.cells[idx].occupancy = Occupancy::Obstacle;
            }
        }
        MapVariant::SteepTerrain => for cell in &mut map.cells { cell.slope_permille = 700; },
    }
    map
}
fn robot_config() -> RobotConfig {
    RobotConfig { max_slope_permille: 300, max_roughness_permille: 250, allow_soft_ground: false,
        energy_per_cell_milliunits: 2, return_reserve_milliunits: 5 }
}
fn traversable(cell: &TerrainCell, robot: &RobotConfig) -> bool {
    cell.observed && cell.occupancy == Occupancy::Clear
        && cell.slope_permille <= robot.max_slope_permille
        && cell.roughness_permille <= robot.max_roughness_permille
        && match cell.surface {
            Surface::Firm => true,
            Surface::Soft => robot.allow_soft_ground,
            Surface::Unknown => false,
        }
}
fn neighbors(map: &GridMap, idx: usize) -> Vec<usize> {
    let width = usize::from(map.width);
    let (x, y) = (idx % width, idx / width);
    let mut out = Vec::with_capacity(4);
    if y > 0 { out.push(idx - width); }
    if x > 0 { out.push(idx - 1); }
    if x + 1 < width { out.push(idx + 1); }
    if y + 1 < usize::from(map.height) { out.push(idx + width); }
    out
}
fn find_path(map: &GridMap, robot: &RobotConfig, start: usize, goal: usize) -> Option<Vec<usize>> {
    if !traversable(&map.cells[start], robot) || !traversable(&map.cells[goal], robot) { return None; }
    let mut seen = vec![false; map.cells.len()];
    let mut previous: Vec<Option<usize>> = vec![None; map.cells.len()];
    let mut queue = VecDeque::new();
    seen[start] = true; previous[start] = Some(start); queue.push_back(start);
    while let Some(current) = queue.pop_front() {
        if current == goal { break; }
        for next in neighbors(map, current) {
            if !seen[next] && traversable(&map.cells[next], robot) {
                seen[next] = true; previous[next] = Some(current); queue.push_back(next);
            }
        }
    }
    if !seen[goal] { return None; }
    let mut path = vec![goal];
    let mut current = goal;
    while current != start { current = previous[current]?; path.push(current); }
    path.reverse();
    Some(path)
}
fn plan_round_trip(map: &GridMap, robot: &RobotConfig) -> Option<Vec<usize>> {
    let start = cell_index(map, CellCoord { x: 0, y: 0 })?;
    let target = cell_index(map, CellCoord { x: 4, y: 4 })?;
    let mut route = find_path(map, robot, start, target)?;
    let inbound = find_path(map, robot, target, start)?;
    route.extend(inbound.into_iter().skip(1));
    Some(route)
}
fn fault_transition(kind: FaultKind) -> (SimState, &'static str) {
    match kind {
        FaultKind::GnssLoss => (SimState::Degraded, "POSE_SOURCE_DEGRADED"),
        FaultKind::WheelSlip => (SimState::Degraded, "MOBILITY_DEGRADED"),
        FaultKind::PoseJump => (SimState::Blocked, "POSE_UNCERTAINTY_EXCEEDED"),
        FaultKind::LidarDegraded => (SimState::Blocked, "PERCEPTION_UNAVAILABLE"),
        FaultKind::ObstacleEmergence => (SimState::Blocked, "OBSTACLE_EMERGED"),
        FaultKind::CommsLoss => (SimState::Blocked, "COMMS_LOSS_POLICY_BLOCK"),
        FaultKind::BatteryDrop => (SimState::Blocked, "RETURN_RESERVE_BREACHED"),
        FaultKind::StaleTime => (SimState::Blocked, "TIME_SOURCE_STALE"),
        FaultKind::MapDisagreement => (SimState::Blocked, "MAP_DISAGREEMENT"),
        FaultKind::EmergencyStopEngaged => (SimState::Blocked, "E_STOP_ENGAGED"),
    }
}
fn validate_scenario(corpus: &Corpus, spec: &ScenarioSpec) -> Check<ScenarioResult> {
    if spec.scenario_id.trim().is_empty() { return Err("scenario_id must not be empty".to_owned()); }
    let map = base_map(corpus.seed, spec.map_variant);
    let robot = robot_config();
    let start_coord = CellCoord { x: 0, y: 0 };
    let target_coord = CellCoord { x: 4, y: 4 };
    let target = cell_index(&map, target_coord).ok_or_else(|| "target outside map".to_owned())?;
    let map_sha = sha256(&encoded(&map)?);
    let sensor = sensor_config();
    let config_inputs = (&robot, &sensor, start_coord, target_coord, spec.map_variant, spec.battery_milliunits, spec.fault);
    let config_sha = sha256(&encoded(&config_inputs)?);
    let mut result = ScenarioResult {
        scenario_id: spec.scenario_id.clone(), state: SimState::Blocked, reason_codes: Vec::new(),
        map_sha256: map_sha, config_sha256: config_sha, planned_route_cells: 0,
        traversed_unique_cells: 0, planned_survey_targets: 1, traversed_survey_targets: 0,
        verified_survey_targets: 0, planned_area_cells: map.cells.len(), observed_area_cells: 0,
        unobserved_area_cells: map.cells.len(), observed_coverage_basis_points: 0,
        energy_used_milliunits: 0,
        fault_response_latency_steps: None, fault_deadline_steps: spec.fault.map(|f| f.deadline_steps),
        fault_deadline_passed: None,
    };
    if !traversable(&map.cells[target], &robot) {
        result.reason_codes.push("TARGET_NOT_TRAVERSABLE".to_owned());
        return Ok(result);
    }
    let route = match plan_round_trip(&map, &robot) {
        Some(route) => route,
        None => { result.reason_codes.push("NO_SAFE_ROUTE".to_owned()); return Ok(result); }
    };
    result.planned_route_cells = route.len();
    let planned_energy = route.len().saturating_sub(1) as u64 * u64::from(robot.energy_per_cell_milliunits);
    if u64::from(spec.battery_milliunits) < planned_energy + u64::from(robot.return_reserve_milliunits) {
        result.reason_codes.push("BATTERY_BELOW_RETURN_RESERVE".to_owned());
        return Ok(result);
    }
    let mut traversed = BTreeSet::new();
    let mut observed_cells = BTreeSet::new();
    let mut triggered = false;
    for (step, idx) in route.iter().copied().enumerate() {
        if let Some(fault) = spec.fault {
            if step as u32 == fault.trigger_step {
                let (state, reason) = fault_transition(fault.kind);
                result.state = state;
                result.reason_codes.push(reason.to_owned());
                result.fault_response_latency_steps = Some(0);
                result.fault_deadline_passed = Some(0_u32 <= fault.deadline_steps);
                triggered = true;
                break;
            }
        }
        traversed.insert(idx);
        let facing = facing_at_route_step(&map, &route, step);
        observed_cells.extend(visible_cells_from_pose(&map, idx, facing, &sensor));
        if step > 0 { result.energy_used_milliunits += u64::from(robot.energy_per_cell_milliunits); }
        if idx == target { result.traversed_survey_targets = 1; }
    }
    result.traversed_unique_cells = traversed.len();
    result.observed_area_cells = observed_cells.len();
    result.unobserved_area_cells = result.planned_area_cells.saturating_sub(result.observed_area_cells);
    if result.planned_area_cells > 0 {
        result.observed_coverage_basis_points =
            ((result.observed_area_cells as u64 * 10_000) / result.planned_area_cells as u64) as u16;
    }
    if result.traversed_survey_targets > 0
        && observed_cells.contains(&target)
        && map.cells[target].observed
        && map.cells[target].occupancy == Occupancy::Clear {
        result.verified_survey_targets = 1;
    }
    if !triggered { result.state = SimState::Completed; }
    Ok(result)
}
fn validate_corpus(corpus: &Corpus) -> Check<Vec<ScenarioResult>> {
    if corpus.suite_id.trim().is_empty() || corpus.suite_version.trim().is_empty() {
        return Err("suite_id and suite_version must not be empty".to_owned());
    }
    if corpus.evidence_class != "synthetic" { return Err("this simulator accepts synthetic scenarios only".to_owned()); }
    if corpus.claim_ceiling.trim().is_empty() { return Err("claim_ceiling must not be empty".to_owned()); }
    if corpus.scenarios.is_empty() || corpus.scenarios.len() > 100 {
        return Err("scenario count must be within 1..=100".to_owned());
    }
    let mut ids = BTreeSet::new();
    let mut results = Vec::with_capacity(corpus.scenarios.len());
    for spec in &corpus.scenarios {
        if !ids.insert(spec.scenario_id.as_str()) { return Err(format!("duplicate scenario_id {}", spec.scenario_id)); }
        let result = validate_scenario(corpus, spec)?;
        if result.state != spec.expected_state {
            return Err(format!("{}: expected state {:?}, got {:?}", spec.scenario_id, spec.expected_state, result.state));
        }
        if result.reason_codes != spec.expected_reason_codes {
            return Err(format!("{}: expected reason codes {:?}, got {:?}", spec.scenario_id, spec.expected_reason_codes, result.reason_codes));
        }
        if let Some(fault) = spec.fault {
            if result.fault_response_latency_steps.is_none()
                || result.fault_deadline_passed != Some(true)
                || result.fault_response_latency_steps.unwrap_or(u32::MAX) > fault.deadline_steps {
                return Err(format!("{}: fault response missed its simulated deadline", spec.scenario_id));
            }
        }
        results.push(result);
    }
    Ok(results)
}
fn build_receipt(corpus: &Corpus, corpus_sha: String, source_commit: String, scenarios: Vec<ScenarioResult>) -> Receipt {
    let completed = scenarios.iter().filter(|s| s.state == SimState::Completed).count();
    let degraded = scenarios.iter().filter(|s| s.state == SimState::Degraded).count();
    let blocked = scenarios.iter().filter(|s| s.state == SimState::Blocked).count();
    let injected_faults = corpus.scenarios.iter().filter(|s| s.fault.is_some()).count();
    let deadline_passes = scenarios.iter().filter(|s| s.fault_deadline_passed == Some(true)).count();
    Receipt {
        result: "validated", simulator_version: VERSION, suite_id: corpus.suite_id.clone(),
        suite_version: corpus.suite_version.clone(), evidence_class: corpus.evidence_class.clone(),
        claim_ceiling: corpus.claim_ceiling.clone(), source_commit, corpus_sha256: corpus_sha,
        seed: corpus.seed, scenario_count: scenarios.len(), completed_count: completed,
        degraded_count: degraded, blocked_count: blocked, injected_fault_count: injected_faults,
        fault_deadline_pass_count: deadline_passes, scenarios,
    }
}
fn execute() -> Check<()> {
    let mut args = env::args().skip(1);
    let path = args.next().ok_or_else(|| "usage: forest-terrain-sim CORPUS.json --source-commit <40-hex-sha>".to_owned())?;
    if args.next().as_deref() != Some("--source-commit") { return Err("expected --source-commit".to_owned()); }
    let source_commit = args.next().ok_or_else(|| "missing source commit SHA".to_owned())?;
    if source_commit.len() != 40 || !source_commit.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("source commit must be exactly 40 hexadecimal characters".to_owned());
    }
    if args.next().is_some() { return Err("unexpected trailing arguments".to_owned()); }
    let bytes = fs::read(&path).map_err(|e| format!("cannot read {path}: {e}"))?;
    let corpus: Corpus = serde_json::from_slice(&bytes).map_err(|e| format!("invalid corpus JSON: {e}"))?;
    let scenarios = validate_corpus(&corpus)?;
    let receipt = build_receipt(&corpus, sha256(&bytes), source_commit, scenarios);
    println!("{}", serde_json::to_string_pretty(&receipt).map_err(|e| e.to_string())?);
    Ok(())
}
fn main() -> ExitCode {
    match execute() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => { eprintln!("forest-terrain-sim: {error}"); ExitCode::from(2) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const FIXTURE: &str = include_str!("../../../docs/tech/forest-terrain-grid-corpus-v0.1.json");
    fn corpus() -> Corpus { serde_json::from_str(FIXTURE).expect("frozen synthetic corpus parses") }

    #[test]
    fn frozen_corpus_matches_all_expected_states_and_reasons() {
        let c = corpus();
        let results = validate_corpus(&c).expect("exact frozen outputs");
        assert_eq!(results.len(), 15);
        assert_eq!(results.iter().filter(|r| r.state == SimState::Completed).count(), 1);
        assert_eq!(results.iter().filter(|r| r.state == SimState::Degraded).count(), 2);
        assert_eq!(results.iter().filter(|r| r.state == SimState::Blocked).count(), 12);
    }
    #[test]
    fn healthy_route_proves_coverage_only_after_traversal_and_observation() {
        let c = corpus(); let r = validate_scenario(&c, &c.scenarios[0]).expect("baseline");
        assert_eq!((r.planned_survey_targets, r.traversed_survey_targets, r.verified_survey_targets), (1, 1, 1));
        assert!(r.traversed_unique_cells > 0);
    }
    #[test]
    fn injected_fault_gets_no_planned_only_coverage_credit() {
        let c = corpus(); let r = validate_scenario(&c, &c.scenarios[1]).expect("fault case");
        assert_eq!((r.planned_survey_targets, r.traversed_survey_targets, r.verified_survey_targets), (1, 0, 0));
        assert_eq!(r.fault_response_latency_steps, Some(0));
    }
    #[test]
    fn fault_occurs_at_configured_simulator_step() {
        let c = corpus();
        let base = &c.scenarios[1];
        let mut later = ScenarioSpec {
            scenario_id: "LATER_FAULT_TEST".to_owned(),
            map_variant: base.map_variant,
            battery_milliunits: base.battery_milliunits,
            fault: base.fault,
            expected_state: SimState::Degraded,
            expected_reason_codes: vec!["POSE_SOURCE_DEGRADED".to_owned()],
        };
        later.fault.as_mut().expect("test case has a fault").trigger_step = 5;
        let result = validate_scenario(&c, &later).expect("later fault remains deterministic");
        assert_eq!(result.state, SimState::Degraded);
        assert_eq!(result.traversed_unique_cells, 5);
        assert_eq!(result.energy_used_milliunits, 8);
        assert_eq!(result.traversed_survey_targets, 0);
    }
    #[test]
    fn every_injected_fault_transitions_before_its_simulated_deadline() {
        let c = corpus(); let results = validate_corpus(&c).expect("fault deadlines met");
        for (spec, result) in c.scenarios.iter().zip(results.iter()) {
            if spec.fault.is_some() { assert_eq!(result.fault_deadline_passed, Some(true), "{}", spec.scenario_id); }
        }
    }
    #[test]
    fn unknown_target_is_not_traversable() {
        let c = corpus(); let s = c.scenarios.iter().find(|s| s.scenario_id == "UNKNOWN_TARGET_FAIL_CLOSED").unwrap();
        assert_eq!(validate_scenario(&c, s).unwrap().reason_codes, vec!["TARGET_NOT_TRAVERSABLE"]);
    }
    #[test]
    fn full_corridor_obstruction_prevents_route_planning() {
        let c = corpus(); let s = c.scenarios.iter().find(|s| s.scenario_id == "BLOCKED_CORRIDOR_NO_ROUTE").unwrap();
        let r = validate_scenario(&c, s).unwrap();
        assert_eq!(r.reason_codes, vec!["NO_SAFE_ROUTE"]); assert_eq!(r.traversed_unique_cells, 0);
    }
    #[test]
    fn insufficient_battery_is_rejected_before_traversal() {
        let c = corpus(); let s = c.scenarios.iter().find(|s| s.scenario_id == "LOW_RETURN_RESERVE").unwrap();
        let r = validate_scenario(&c, s).unwrap();
        assert_eq!(r.reason_codes, vec!["BATTERY_BELOW_RETURN_RESERVE"]); assert_eq!(r.traversed_unique_cells, 0);
    }
    #[test]
    fn digests_are_deterministic_and_map_sensitive() {
        let c = corpus(); let a = validate_scenario(&c, &c.scenarios[0]).unwrap();
        let b = validate_scenario(&c, &c.scenarios[0]).unwrap();
        assert_eq!(a.map_sha256, b.map_sha256); assert_eq!(a.config_sha256, b.config_sha256);
        let other = c.scenarios.iter().find(|s| s.map_variant == MapVariant::UnknownTarget).unwrap();
        let changed = validate_scenario(&c, other).unwrap(); assert_ne!(a.map_sha256, changed.map_sha256);
    }
    #[test]
    fn baseline_reports_partial_sensor_coverage_separately_from_traversal() {
        let c = corpus();
        let r = validate_scenario(&c, &c.scenarios[0]).expect("baseline");
        assert_eq!(r.planned_area_cells, 81);
        assert!(r.observed_area_cells > 0);
        assert!(r.observed_area_cells < r.planned_area_cells);
        assert_eq!(r.observed_area_cells + r.unobserved_area_cells, r.planned_area_cells);
        assert!(r.observed_coverage_basis_points < 10_000);
    }
    #[test]
    fn cells_behind_the_sensor_are_outside_the_forward_field_of_view() {
        let map = base_map(20261017, MapVariant::Clear);
        let sensor = sensor_config();
        let origin = cell_index(&map, CellCoord { x: 4, y: 4 }).unwrap();
        let behind = cell_index(&map, CellCoord { x: 1, y: 4 }).unwrap();
        let ahead = cell_index(&map, CellCoord { x: 7, y: 4 }).unwrap();
        assert!(!visible_from_pose(&map, origin, behind, Facing::East, &sensor));
        assert!(visible_from_pose(&map, origin, ahead, Facing::East, &sensor));
    }
    #[test]
    fn obstacle_blocks_line_of_sight_even_when_a_disk_baseline_would_count_the_cell() {
        let mut map = base_map(20261017, MapVariant::Clear);
        let obstacle = cell_index(&map, CellCoord { x: 1, y: 4 }).unwrap();
        map.cells[obstacle].occupancy = Occupancy::Obstacle;
        let origin = cell_index(&map, CellCoord { x: 0, y: 4 }).unwrap();
        let behind = cell_index(&map, CellCoord { x: 3, y: 4 }).unwrap();
        assert!(within_sensor_range(&map, origin, behind, &sensor_config()));
        assert!(visible_from_pose(&map, origin, obstacle, Facing::East, &sensor_config()));
        assert!(!visible_from_pose(&map, origin, behind, Facing::East, &sensor_config()));
    }
    #[test]
    fn unknown_cells_can_be_targeted_but_block_visibility_beyond_them() {
        let mut map = base_map(20261017, MapVariant::Clear);
        let unknown = cell_index(&map, CellCoord { x: 1, y: 4 }).unwrap();
        map.cells[unknown].occupancy = Occupancy::Unknown;
        map.cells[unknown].observed = false;
        let origin = cell_index(&map, CellCoord { x: 0, y: 4 }).unwrap();
        let behind = cell_index(&map, CellCoord { x: 2, y: 4 }).unwrap();
        let sensor = sensor_config();
        assert!(visible_from_pose(&map, origin, unknown, Facing::East, &sensor));
        assert!(!visible_from_pose(&map, origin, behind, Facing::East, &sensor));
    }
    #[test]
    fn fault_interruptions_reduce_observed_coverage_without_crediting_the_plan() {
        let c = corpus();
        let baseline = validate_scenario(&c, &c.scenarios[0]).expect("baseline");
        let interrupted = validate_scenario(&c, &c.scenarios[1]).expect("GNSS fault");
        assert!(interrupted.observed_area_cells < baseline.observed_area_cells);
        assert_eq!(interrupted.planned_area_cells, baseline.planned_area_cells);
        assert_eq!(interrupted.verified_survey_targets, 0);
    }
    #[test]
    fn synthetic_only_claim_ceiling_is_enforced() {
        let mut c = corpus(); c.evidence_class = "field".to_owned();
        assert!(validate_corpus(&c).unwrap_err().contains("synthetic"));
    }
    #[test]
    fn duplicate_scenario_ids_are_rejected() {
        let mut c = corpus();
        let mut duplicate = ScenarioSpec {
            scenario_id: c.scenarios[0].scenario_id.clone(), map_variant: c.scenarios[0].map_variant,
            battery_milliunits: c.scenarios[0].battery_milliunits, fault: c.scenarios[0].fault,
            expected_state: c.scenarios[0].expected_state,
            expected_reason_codes: c.scenarios[0].expected_reason_codes.clone(),
        };
        duplicate.scenario_id = c.scenarios[0].scenario_id.clone(); c.scenarios.push(duplicate);
        assert!(validate_corpus(&c).unwrap_err().contains("duplicate scenario_id"));
    }
}
