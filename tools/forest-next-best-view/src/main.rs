use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::VecDeque, env, fs, process::ExitCode};

const W: usize = 5;
const N: usize = W * W;
const RANGE: i32 = 3;
type Check<T> = Result<T, String>;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Corpus { suite_id: String, suite_version: String, evidence_class: String, claim_ceiling: String, scenarios: Vec<Spec> }
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Spec {
    scenario_id: String, scene: Scene, energy_budget_steps: u16, return_reserve_steps: u16, max_viewpoints: u8,
    expected_greedy_coverage_cells: usize, expected_oracle_coverage_cells: usize, expected_greedy_ratio_basis_points: u16,
}
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Scene { OpenGrid, OccludedClearing, BlockedCorridor, UnknownBarrier }
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Occupancy { Clear, Obstacle, Unknown }
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
enum Heading { North, East, South, West }
const HEADINGS: [Heading; 4] = [Heading::North, Heading::East, Heading::South, Heading::West];

#[derive(Debug, Clone, Serialize)]
struct GridMap { width: usize, height: usize, cells: Vec<Occupancy> }
#[derive(Debug, Clone, Serialize)]
struct PlannerConfig {
    start_cell: usize, energy_budget_steps: u16, return_reserve_steps: u16, max_viewpoints: u8,
    sensor_range_cells: i32, horizontal_fov_degrees: u16, turn_cost_steps: u16,
    greedy_algorithm: String, oracle_algorithm: String,
}
#[derive(Debug, Clone, Serialize)]
struct Viewpoint { x: usize, y: usize, heading: Heading, marginal_gain_cells: usize, incremental_cost_steps: u16 }
#[derive(Debug, Clone, Serialize)]
struct PlanResult {
    covered_cells: usize, outbound_cost_steps: u16, return_distance_steps: u16, reserved_steps: u16,
    total_budgeted_steps: u16, budget_feasible: bool, viewpoints: Vec<Viewpoint>,
}
#[derive(Debug, Serialize)]
struct ScenarioResult {
    scenario_id: String, greedy: PlanResult, exhaustive_oracle: PlanResult, greedy_to_oracle_basis_points: u16,
    map_sha256: String, config_sha256: String,
}
#[derive(Debug, Serialize)]
struct Receipt {
    result: &'static str, planner_version: &'static str, suite_id: String, suite_version: String,
    evidence_class: String, claim_ceiling: String, source_commit: String, corpus_sha256: String,
    scenario_count: usize, greedy_budget_feasible_count: usize, oracle_budget_feasible_count: usize,
    exact_oracle_coverage_match_count: usize, scenarios: Vec<ScenarioResult>,
}
#[derive(Debug, Clone)]
struct Candidate { cell: usize, heading: Heading, visible: Vec<usize> }
#[derive(Debug, Clone)]
struct State { covered: Vec<bool>, outbound: u16, selected: Vec<usize> }
struct Planner { candidates: Vec<Candidate>, dist: Vec<Vec<Option<u16>>>, config: PlannerConfig }
#[derive(Clone, Copy)]
struct Coord { x: usize, y: usize }

fn hash(bytes: &[u8]) -> String { format!("{:x}", Sha256::digest(bytes)) }
fn encode<T: Serialize>(v: &T) -> Check<Vec<u8>> { serde_json::to_vec(v).map_err(|e| e.to_string()) }
fn idx(x: usize, y: usize) -> usize { y * W + x }
fn xy(i: usize) -> Coord { Coord { x: i % W, y: i / W } }

fn map_for(scene: Scene) -> GridMap {
    let mut cells = vec![Occupancy::Clear; N];
    match scene {
        Scene::OpenGrid => {}
        Scene::OccludedClearing => for y in 1..W { cells[idx(2, y)] = Occupancy::Obstacle; },
        Scene::BlockedCorridor => for y in 0..W { cells[idx(2, y)] = Occupancy::Obstacle; },
        Scene::UnknownBarrier => for y in 1..W { cells[idx(2, y)] = Occupancy::Unknown; },
    }
    GridMap { width: W, height: W, cells }
}
fn clear(map: &GridMap, cell: usize) -> bool { map.cells.get(cell) == Some(&Occupancy::Clear) }
fn neighbors(i: usize) -> Vec<usize> {
    let c = xy(i); let mut n = Vec::with_capacity(4);
    if c.y > 0 { n.push(idx(c.x, c.y - 1)); }
    if c.x > 0 { n.push(idx(c.x - 1, c.y)); }
    if c.x + 1 < W { n.push(idx(c.x + 1, c.y)); }
    if c.y + 1 < W { n.push(idx(c.x, c.y + 1)); }
    n
}
fn dijkstra(map: &GridMap, start: usize) -> Vec<Option<u16>> {
    let mut d: Vec<Option<u16>> = vec![None; N];
    if !clear(map, start) { return d; }
    let mut q = VecDeque::new(); d[start] = Some(0); q.push_back(start);
    while let Some(cur) = q.pop_front() {
        let nd = d[cur].unwrap_or(0).saturating_add(1);
        for next in neighbors(cur) {
            if clear(map, next) && d[next].is_none() { d[next] = Some(nd); q.push_back(next); }
        }
    }
    d
}
fn ray_blocks(map: &GridMap, x: i32, y: i32) -> bool {
    x < 0 || y < 0 || x >= W as i32 || y >= W as i32
        || map.cells[idx(x as usize, y as usize)] != Occupancy::Clear
}
fn line_of_sight(map: &GridMap, from: usize, to: usize) -> bool {
    let a = xy(from); let b = xy(to);
    let (mut x, mut y) = (a.x as i32, a.y as i32); let (tx, ty) = (b.x as i32, b.y as i32);
    let dx = (tx - x).abs(); let dy = (ty - y).abs();
    let sx = if x < tx { 1 } else if x > tx { -1 } else { 0 };
    let sy = if y < ty { 1 } else if y > ty { -1 } else { 0 };
    let mut err = dx - dy;
    loop {
        if x == tx && y == ty { return true; }
        let e2 = 2 * err; let step_x = e2 > -dy; let step_y = e2 < dx;
        if step_x && step_y && (ray_blocks(map, x + sx, y) || ray_blocks(map, x, y + sy)) { return false; }
        if step_x { err -= dy; x += sx; }
        if step_y { err += dx; y += sy; }
        if (x, y) != (tx, ty) && ray_blocks(map, x, y) { return false; }
    }
}
fn visibility(map: &GridMap, cell: usize, heading: Heading) -> Vec<usize> {
    let c = xy(cell);
    let (fx, fy) = match heading { Heading::North => (0, -1), Heading::East => (1, 0), Heading::South => (0, 1), Heading::West => (-1, 0) };
    (0..N).filter(|t| {
        let p = xy(*t); let dx = p.x as i32 - c.x as i32; let dy = p.y as i32 - c.y as i32;
        dx * dx + dy * dy <= RANGE * RANGE && dx * fx + dy * fy >= 0 && line_of_sight(map, cell, *t)
    }).collect()
}
fn cover_count(c: &[bool]) -> usize { c.iter().filter(|v| **v).count() }
fn gain(c: &[bool], visible: &[usize]) -> usize { visible.iter().filter(|i| !c[**i]).count() }
fn add(c: &mut [bool], visible: &[usize]) { for i in visible { c[*i] = true; } }
fn start_heading() -> Heading { Heading::North }

impl Planner {
    fn new(map: &GridMap, spec: &Spec) -> Check<Self> {
        if !clear(map, 0) { return Err("start cell is not traversable".into()); }
        if spec.max_viewpoints == 0 || spec.max_viewpoints > 4 { return Err("max_viewpoints must be in 1..=4".into()); }
        if spec.energy_budget_steps <= spec.return_reserve_steps { return Err("budget must exceed return reserve".into()); }
        let config = PlannerConfig {
            start_cell: 0, energy_budget_steps: spec.energy_budget_steps,
            return_reserve_steps: spec.return_reserve_steps, max_viewpoints: spec.max_viewpoints,
            sensor_range_cells: RANGE, horizontal_fov_degrees: 180, turn_cost_steps: 1,
            greedy_algorithm: "marginal_gain / max(incremental_cost, 1), stable candidate-order tie-break".into(),
            oracle_algorithm: "exhaustive unique-viewpoint sequences; maximize unique visible cells, then minimize budgeted steps".into(),
        };
        let dist = (0..N).map(|i| dijkstra(map, i)).collect();
        let candidates = (0..N).filter(|i| clear(map, *i)).flat_map(|cell| HEADINGS.into_iter().map(move |heading|
            Candidate { cell, heading, visible: visibility(map, cell, heading) })).collect();
        Ok(Self { candidates, dist, config })
    }
    fn transition(&self, from: usize, h: Heading, to: &Candidate) -> Option<u16> {
        Some(self.dist[from][to.cell]?.saturating_add(if h == to.heading { 0 } else { self.config.turn_cost_steps() }))
    }
    fn total(&self, out: u16, selected: &[usize]) -> Option<(u16, u16)> {
        let last = selected.last().map(|i| self.candidates[*i].cell).unwrap_or(self.config.start_cell);
        let back = self.dist[last][self.config.start_cell]?;
        Some((back, out.saturating_add(back).saturating_add(self.config.return_reserve_steps)))
    }
    fn feasible(&self, out: u16, selected: &[usize]) -> bool {
        self.total(out, selected).is_some_and(|(_, t)| t <= self.config.energy_budget_steps)
    }
    fn greedy(&self) -> State {
        let mut s = State { covered: vec![false; N], outbound: 0, selected: Vec::new() };
        let (mut at, mut h) = (self.config.start_cell, start_heading());
        while s.selected.len() < usize::from(self.config.max_viewpoints) {
            let mut choice: Option<(usize, usize, u16)> = None;
            for (ci, c) in self.candidates.iter().enumerate() {
                if s.selected.contains(&ci) { continue; }
                let Some(cost) = self.transition(at, h, c) else { continue; };
                let mut picks = s.selected.clone(); picks.push(ci);
                if !self.feasible(s.outbound.saturating_add(cost), &picks) { continue; }
                let g = gain(&s.covered, &c.visible); if g == 0 { continue; }
                let better = match choice {
                    None => true,
                    Some((_, og, oc)) => {
                        let ns = (g as u32) * u32::from(oc.max(1));
                        let os = (og as u32) * u32::from(cost.max(1));
                        ns > os || (ns == os && (g > og || (g == og && cost < oc)))
                    }
                };
                if better { choice = Some((ci, g, cost)); }
            }
            let Some((ci, _, cost)) = choice else { break; };
            let c = &self.candidates[ci]; add(&mut s.covered, &c.visible);
            s.outbound = s.outbound.saturating_add(cost); at = c.cell; h = c.heading; s.selected.push(ci);
        }
        s
    }
    fn exhaustive(&self) -> State {
        let initial = State { covered: vec![false; N], outbound: 0, selected: Vec::new() };
        let mut best = initial.clone();
        self.search(self.config.start_cell, start_heading(), &initial, &mut best);
        best
    }
    fn search(&self, at: usize, h: Heading, s: &State, best: &mut State) {
        if self.better(s, best) { *best = s.clone(); }
        if s.selected.len() >= usize::from(self.config.max_viewpoints) { return; }
        for (ci, c) in self.candidates.iter().enumerate() {
            if s.selected.contains(&ci) { continue; }
            let Some(cost) = self.transition(at, h, c) else { continue; };
            let out = s.outbound.saturating_add(cost);
            let mut picks = s.selected.clone(); picks.push(ci);
            if !self.feasible(out, &picks) { continue; }
            if gain(&s.covered, &c.visible) == 0 { continue; }
            let mut covered = s.covered.clone(); add(&mut covered, &c.visible);
            self.search(c.cell, c.heading, &State { covered, outbound: out, selected: picks }, best);
        }
    }
    fn better(&self, a: &State, b: &State) -> bool {
        let ac = cover_count(&a.covered); let bc = cover_count(&b.covered);
        if ac != bc { return ac > bc; }
        let at = self.total(a.outbound, &a.selected).map(|x| x.1).unwrap_or(u16::MAX);
        let bt = self.total(b.outbound, &b.selected).map(|x| x.1).unwrap_or(u16::MAX);
        if at != bt { return at < bt; }
        a.selected.len() < b.selected.len()
    }
    fn result(&self, s: &State) -> PlanResult {
        let (back, total) = self.total(s.outbound, &s.selected).unwrap_or((u16::MAX, u16::MAX));
        let mut at = self.config.start_cell; let mut h = start_heading(); let mut covered = vec![false; N];
        let viewpoints = s.selected.iter().map(|ci| {
            let c = &self.candidates[*ci];
            let cost = self.transition(at, h, c).unwrap_or(u16::MAX);
            let marginal = gain(&covered, &c.visible); add(&mut covered, &c.visible);
            let p = xy(c.cell); at = c.cell; h = c.heading;
            Viewpoint { x: p.x, y: p.y, heading: c.heading, marginal_gain_cells: marginal, incremental_cost_steps: cost }
        }).collect();
        PlanResult {
            covered_cells: cover_count(&s.covered), outbound_cost_steps: s.outbound, return_distance_steps: back,
            reserved_steps: self.config.return_reserve_steps, total_budgeted_steps: total,
            budget_feasible: total <= self.config.energy_budget_steps, viewpoints,
        }
    }
}

fn validate(c: &Corpus) -> Check<Vec<ScenarioResult>> {
    if c.suite_id.trim().is_empty() || c.suite_version.trim().is_empty() || c.claim_ceiling.trim().is_empty() {
        return Err("suite identity and claim ceiling must not be empty".into());
    }
    if c.evidence_class != "synthetic" { return Err("synthetic scenes only".into()); }
    if c.scenarios.is_empty() || c.scenarios.len() > 20 { return Err("scenario count must be in 1..=20".into()); }
    let mut ids = std::collections::BTreeSet::new(); let mut out = Vec::new();
    for s in &c.scenarios {
        if s.scenario_id.trim().is_empty() || !ids.insert(s.scenario_id.as_str()) { return Err(format!("empty/duplicate id {}", s.scenario_id)); }
        let map = map_for(s.scene); let p = Planner::new(&map, s)?;
        let g = p.result(&p.greedy()); let o = p.result(&p.exhaustive());
        if !g.budget_feasible || !o.budget_feasible { return Err(format!("{}: budget infeasible plan", s.scenario_id)); }
        if o.covered_cells < g.covered_cells { return Err(format!("{}: oracle below greedy", s.scenario_id)); }
        let ratio = if o.covered_cells == 0 { 10000 } else { ((g.covered_cells as u64 * 10000) / o.covered_cells as u64) as u16 };
        if g.covered_cells != s.expected_greedy_coverage_cells || o.covered_cells != s.expected_oracle_coverage_cells || ratio != s.expected_greedy_ratio_basis_points {
            return Err(format!("{} expected {}/{}/{}, got {}/{}/{}", s.scenario_id,
                s.expected_greedy_coverage_cells, s.expected_oracle_coverage_cells, s.expected_greedy_ratio_basis_points,
                g.covered_cells, o.covered_cells, ratio));
        }
        out.push(ScenarioResult {
            scenario_id: s.scenario_id.clone(), greedy: g, exhaustive_oracle: o,
            greedy_to_oracle_basis_points: ratio, greedy_expected_coverage_match: true, oracle_expected_coverage_match: true,
            map_sha256: hash(&encode(&map)?), config_sha256: hash(&encode(&p.config)?),
        });
    }
    Ok(out)
}
fn receipt(c: &Corpus, corpus_sha256: String, source_commit: String, scenarios: Vec<ScenarioResult>) -> Receipt {
    Receipt {
        result: "validated", planner_version: "0.1.0", suite_id: c.suite_id.clone(), suite_version: c.suite_version.clone(),
        evidence_class: c.evidence_class.clone(), claim_ceiling: c.claim_ceiling.clone(), source_commit, corpus_sha256,
        scenario_count: scenarios.len(), greedy_budget_feasible_count: scenarios.iter().filter(|s| s.greedy.budget_feasible).count(),
        oracle_budget_feasible_count: scenarios.iter().filter(|s| s.exhaustive_oracle.budget_feasible).count(),
        exact_oracle_coverage_match_count: scenarios.iter().filter(|s| s.oracle_expected_coverage_match).count(), scenarios,
    }
}
fn run() -> Check<()> {
    let mut args = env::args().skip(1);
    let path = args.next().ok_or_else(|| "usage: forest-next-best-view CORPUS.json --source-commit <40-hex-sha>".to_owned())?;
    if args.next().as_deref() != Some("--source-commit") { return Err("expected --source-commit".into()); }
    let source = args.next().ok_or_else(|| "missing source commit".to_owned())?;
    if source.len() != 40 || !source.bytes().all(|b| b.is_ascii_hexdigit()) { return Err("source commit must be 40 hex chars".into()); }
    if args.next().is_some() { return Err("unexpected trailing arguments".into()); }
    let bytes = fs::read(&path).map_err(|e| format!("cannot read {path}: {e}"))?;
    let c: Corpus = serde_json::from_slice(&bytes).map_err(|e| format!("invalid JSON: {e}"))?;
    let results = validate(&c)?;
    println!("{}", serde_json::to_string_pretty(&receipt(&c, hash(&bytes), source, results)).map_err(|e| e.to_string())?);
    Ok(())
}
fn main() -> ExitCode {
    match run() { Ok(()) => ExitCode::SUCCESS, Err(e) => { eprintln!("forest-next-best-view: {e}"); ExitCode::from(2) } }
}

#[cfg(test)]
mod tests {
    use super::*;
    const FIXTURE: &str = include_str!("../../../docs/tech/forest-next-best-view-corpus-v0.1.json");
    fn corpus() -> Corpus { serde_json::from_str(FIXTURE).expect("fixture parses") }

    #[test]
    fn frozen_corpus_matches_greedy_and_oracle() {
        let c = corpus(); let r = validate(&c).expect("exact frozen outputs");
        assert_eq!(r.len(), 6);
        assert!(r.iter().all(|x| x.greedy.budget_feasible && x.exhaustive_oracle.budget_feasible));
    }
    #[test]
    fn exhaustive_oracle_never_scores_below_greedy() {
        let c = corpus();
        for s in &c.scenarios {
            let map = map_for(s.scene); let p = Planner::new(&map, s).unwrap();
            assert!(cover_count(&p.exhaustive().covered) >= cover_count(&p.greedy().covered));
        }
    }
    #[test]
    fn baseline_greediness_gap_is_measured_not_hidden() {
        let r = validate(&corpus()).unwrap();
        let open = r.iter().find(|x| x.scenario_id == "OPEN_GRID_BUDGETED").unwrap();
        assert_eq!((open.greedy.covered_cells, open.exhaustive_oracle.covered_cells, open.greedy_to_oracle_basis_points), (19, 23, 8260));
        let single = r.iter().find(|x| x.scenario_id == "SINGLE_SCAN_BUDGET").unwrap();
        assert_eq!((single.greedy.covered_cells, single.exhaustive_oracle.covered_cells, single.greedy_to_oracle_basis_points), (11, 16, 6875));
    }
    #[test]
    fn obstacles_occlude_space_behind_them() {
        let map = map_for(Scene::OccludedClearing);
        assert!(!visibility(&map, idx(1, 2), Heading::East).contains(&idx(3, 2)));
    }
    #[test]
    fn unknown_space_is_not_reachable_and_occludes_cells_behind_it() {
        let map = map_for(Scene::UnknownBarrier);
        assert!(!clear(&map, idx(2, 2)));
        let visible = visibility(&map, idx(1, 2), Heading::East);
        assert!(visible.contains(&idx(2, 2)));
        assert!(!visible.contains(&idx(3, 2)));
    }
    #[test]
    fn return_reserve_is_always_accounted_for() {
        let c = corpus();
        for spec in &c.scenarios {
            let map = map_for(spec.scene); let p = Planner::new(&map, spec).unwrap();
            for result in [p.result(&p.greedy()), p.result(&p.exhaustive())] {
                assert_eq!(result.total_budgeted_steps, result.outbound_cost_steps + result.return_distance_steps + result.reserved_steps);
                assert!(result.total_budgeted_steps <= spec.energy_budget_steps);
            }
        }
    }
    #[test]
    fn output_is_deterministic_for_fixed_inputs() {
        let c = corpus(); let s = &c.scenarios[0]; let map = map_for(s.scene); let p = Planner::new(&map, s).unwrap();
        let a = serde_json::to_vec(&p.result(&p.exhaustive())).unwrap();
        let b = serde_json::to_vec(&p.result(&p.exhaustive())).unwrap();
        assert_eq!(a, b);
    }
    #[test]
    fn scene_hashes_change_when_geometry_changes() {
        let a = map_for(Scene::OpenGrid); let b = map_for(Scene::BlockedCorridor);
        assert_ne!(hash(&encode(&a).unwrap()), hash(&encode(&b).unwrap()));
    }
}
