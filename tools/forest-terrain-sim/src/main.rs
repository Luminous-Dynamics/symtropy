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
fn base_map(seed: u64, variant: MapVariant) -> GridMap {
    let (width, height) = (5_u16, 5_u16);
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
    let config_inputs = (&robot, start_coord, target_coord, spec.map_variant, spec.battery_milliunits, spec.fault);
    let config_sha = sha256(&encoded(&config_inputs)?);
    let mut result = ScenarioResult {
        scenario_id: spec.scenario_id.clone(), state: SimState::Blocked, reason_codes: Vec::new(),
        map_sha256: map_sha, config_sha256: config_sha, planned_route_cells: 0,
        traversed_unique_cells: 0, planned_survey_targets: 1, traversed_survey_targets: 0,
        verified_survey_targets: 0, energy_used_milliunits: 0,
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
        if step > 0 { result.energy_used_milliunits += u64::from(robot.energy_per_cell_milliunits); }
        if idx == target {
            result.traversed_survey_targets = 1;
            if map.cells[target].observed && map.cells[target].occupancy == Occupancy::Clear {
                result.verified_survey_targets = 1;
            }
        }
    }
    result.traversed_unique_cells = traversed.len();
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
