use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::{BTreeMap, BTreeSet, VecDeque}, env, fs, process::ExitCode};

const W: usize = 5;
const N: usize = W * W;
const RANGE: i32 = 3;
type Check<T> = Result<T, String>;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Corpus {
    suite_id: String,
    suite_version: String,
    evidence_class: String,
    claim_ceiling: String,
    generator_version: String,
    training_seeds: Vec<u64>,
    held_out_seeds: Vec<u64>,
    target_count_per_case: usize,
    scenario_bindings: Vec<ScenarioBinding>,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ScenarioBinding {
    planner_scenario_id: String,
    scene: Scene,
    energy_budget_steps: u16,
    return_reserve_steps: u16,
    max_viewpoints: u8,
}
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Scene { OpenGrid, OccludedClearing, BlockedCorridor, UnknownBarrier }
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Occupancy { Clear, Obstacle, Unknown }
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Heading { North, East, South, West }

#[derive(Debug, Deserialize)]
struct PlannerReceipt {
    source_commit: String,
    corpus_sha256: String,
    scenarios: Vec<PlannerScenario>,
}
#[derive(Debug, Deserialize)]
struct PlannerScenario {
    scenario_id: String,
    greedy: PlannerPlan,
    two_step_lookahead: PlannerPlan,
    exhaustive_oracle: PlannerPlan,
    map_sha256: String,
    config_sha256: String,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
struct PlannerPlan {
    budget_feasible: bool,
    outbound_cost_steps: u16,
    return_distance_steps: u16,
    reserved_steps: u16,
    total_budgeted_steps: u16,
    viewpoints: Vec<Viewpoint>,
}
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
struct Viewpoint { x: usize, y: usize, heading: Heading }

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum TargetKind { TrunkDiameterEdges, LogEndContour }
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct Target {
    target_id: String,
    kind: TargetKind,
    center_cell: usize,
    required_feature_cells: Vec<usize>,
}
#[derive(Debug, Clone, Serialize)]
struct TargetScore {
    target_id: String,
    kind: TargetKind,
    center_cell: usize,
    required_feature_cells: Vec<usize>,
    required_feature_count: usize,
    visible_feature_count: usize,
    completeness_basis_points: u16,
    complete: bool,
}
#[derive(Debug, Clone, Serialize)]
struct MethodScore {
    budget_feasible: bool,
    target_count: usize,
    complete_target_count: usize,
    required_feature_count: usize,
    visible_feature_count: usize,
    complete_target_rate_basis_points: u16,
    feature_visibility_basis_points: u16,
    targets: Vec<TargetScore>,
}
#[derive(Debug, Clone, Serialize)]
struct TargetOracleResult {
    objective: String,
    candidate_count: usize,
    expanded_state_count: usize,
    plan: PlannerPlan,
    score: MethodScore,
}
#[derive(Debug, Clone, Serialize)]
struct TargetRegret {
    oracle_complete_target_count: usize,
    compared_complete_target_count: usize,
    complete_target_regret: usize,
    complete_target_ratio_basis_points: Option<u16>,
    oracle_visible_feature_count: usize,
    compared_visible_feature_count: usize,
    visible_feature_delta_to_oracle: i64,
}
#[derive(Debug, Clone, Serialize)]
struct Evaluation {
    planner_scenario_id: String,
    scene: Scene,
    seed: u64,
    split: String,
    generated_target_count: usize,
    map_sha256: String,
    config_sha256: String,
    greedy: MethodScore,
    lookahead: MethodScore,
    coverage_oracle: MethodScore,
    target_oracle: TargetOracleResult,
    greedy_vs_target_oracle: TargetRegret,
    lookahead_vs_target_oracle: TargetRegret,
    coverage_oracle_vs_target_oracle: TargetRegret,
}
#[derive(Debug, Clone, Serialize)]
struct AggregateMethod {
    evaluation_count: usize,
    target_count: usize,
    complete_target_count: usize,
    required_feature_count: usize,
    visible_feature_count: usize,
    complete_target_rate_basis_points: u16,
    feature_visibility_basis_points: u16,
    worst_case_feature_visibility_basis_points: u16,
}
#[derive(Debug, Clone, Serialize)]
struct AggregateRegret {
    evaluation_count: usize,
    oracle_complete_target_count: usize,
    compared_complete_target_count: usize,
    total_complete_target_regret: usize,
    worst_case_complete_target_regret: usize,
    complete_target_ratio_basis_points: Option<u16>,
    worst_case_complete_target_ratio_basis_points: Option<u16>,
    oracle_visible_feature_count: usize,
    compared_visible_feature_count: usize,
    visible_feature_delta_to_oracle_total: i64,
    worst_case_visible_feature_delta_to_oracle: i64,
}
#[derive(Debug, Clone, Serialize)]
struct SplitSummary {
    seed_count: usize,
    evaluation_count: usize,
    greedy: AggregateMethod,
    lookahead: AggregateMethod,
    coverage_oracle: AggregateMethod,
    target_objective_oracle: AggregateMethod,
    greedy_vs_target_oracle: AggregateRegret,
    lookahead_vs_target_oracle: AggregateRegret,
    coverage_oracle_vs_target_oracle: AggregateRegret,
}
#[derive(Debug, Serialize)]
struct Receipt {
    result: &'static str,
    suite_id: String,
    suite_version: String,
    evidence_class: String,
    claim_ceiling: String,
    generator_version: String,
    target_objective: String,
    source_commit: String,
    planner_source_commit: String,
    planner_corpus_sha256: String,
    planner_receipt_sha256: String,
    target_corpus_sha256: String,
    scenario_count: usize,
    training_seed_count: usize,
    held_out_seed_count: usize,
    seed_overlap_count: usize,
    evaluation_count: usize,
    training_evaluation_count: usize,
    held_out_evaluation_count: usize,
    training: SplitSummary,
    held_out: SplitSummary,
    evaluations: Vec<Evaluation>,
}
#[derive(Debug, Clone, Serialize)]
struct GridMap { width: usize, height: usize, cells: Vec<Occupancy> }
#[derive(Debug, Clone, Serialize)]
struct PlannerConfig {
    start_cell: usize, energy_budget_steps: u16, return_reserve_steps: u16, max_viewpoints: u8,
    sensor_range_cells: i32, horizontal_fov_degrees: u16, turn_cost_steps: u16,
    greedy_algorithm: String, oracle_algorithm: String, lookahead_algorithm: String,
}
#[derive(Debug, Clone, Copy)]
struct Coord { x: usize, y: usize }
#[derive(Debug, Clone)]
struct Rng(u64);

fn hash(bytes: &[u8]) -> String { format!("{:x}", Sha256::digest(bytes)) }
fn digest_json<T: Serialize>(value: &T) -> Check<String> {
    serde_json::to_vec(value).map(|bytes| hash(&bytes)).map_err(|e| e.to_string())
}
fn idx(x: usize, y: usize) -> usize { y * W + x }
fn xy(i: usize) -> Coord { Coord { x: i % W, y: i / W } }
fn clear(map: &GridMap, cell: usize) -> bool { map.cells.get(cell) == Some(&Occupancy::Clear) }
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
fn rng_for(seed: u64, scenario_id: &str) -> Rng {
    let mut fnv = 0xcbf29ce484222325_u64;
    for b in scenario_id.as_bytes() { fnv = (fnv ^ u64::from(*b)).wrapping_mul(0x100000001b3); }
    let mixed = seed ^ fnv;
    Rng(if mixed == 0 { 0x9e3779b97f4a7c15 } else { mixed })
}
impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    fn shuffle<T>(&mut self, values: &mut [T]) {
        for i in (1..values.len()).rev() {
            let j = (self.next() as usize) % (i + 1);
            values.swap(i, j);
        }
    }
}
fn adjacent(map: &GridMap, cell: usize) -> Vec<usize> {
    let p = xy(cell);
    let mut out = Vec::with_capacity(4);
    if p.y > 0 { out.push(idx(p.x, p.y - 1)); }
    if p.x > 0 { out.push(idx(p.x - 1, p.y)); }
    if p.x + 1 < W { out.push(idx(p.x + 1, p.y)); }
    if p.y + 1 < W { out.push(idx(p.x, p.y + 1)); }
    out
}
fn distances(map: &GridMap, start: usize) -> Vec<Option<u16>> {
    let mut dist = vec![None; N];
    if !clear(map, start) { return dist; }
    let mut queue = VecDeque::new();
    dist[start] = Some(0);
    queue.push_back(start);
    while let Some(current) = queue.pop_front() {
        let next_distance = dist[current].unwrap_or(0).saturating_add(1);
        for next in adjacent(map, current) {
            if clear(map, next) && dist[next].is_none() {
                dist[next] = Some(next_distance);
                queue.push_back(next);
            }
        }
    }
    dist
}
fn verify_plan_contract(map: &GridMap, plan: &PlannerPlan, config: &PlannerConfig, label: &str) -> Check<()> {
    if !plan.budget_feasible { return Err(format!("{label}: planner says plan is budget-infeasible")); }
    if plan.viewpoints.len() > usize::from(config.max_viewpoints) {
        return Err(format!("{label}: viewpoint count exceeds configured limit"));
    }
    let mut outbound = 0u16;
    let mut visited_views = BTreeSet::new();
    let (mut at, mut heading) = (config.start_cell, Heading::North);
    if !clear(map, at) { return Err(format!("{label}: start is not traversable")); }
    for view in &plan.viewpoints {
        if view.x >= W || view.y >= W { return Err(format!("{label}: viewpoint outside map")); }
        let cell = idx(view.x, view.y);
        if !clear(map, cell) { return Err(format!("{label}: viewpoint on obstacle/unknown cell")); }
        if !visited_views.insert((view.x, view.y, view.heading)) { return Err(format!("{label}: repeated viewpoint candidate")); }
        let travel = distances(map, at)[cell].ok_or_else(|| format!("{label}: viewpoint unreachable"))?;
        let turn = if heading == view.heading { 0 } else { config.turn_cost_steps };
        outbound = outbound.saturating_add(travel).saturating_add(turn);
        at = cell;
        heading = view.heading;
    }
    let back = distances(map, at)[config.start_cell].ok_or_else(|| format!("{label}: no feasible return path"))?;
    let total = outbound.saturating_add(back).saturating_add(config.return_reserve_steps);
    if plan.outbound_cost_steps != outbound
        || plan.return_distance_steps != back
        || plan.reserved_steps != config.return_reserve_steps
        || plan.total_budgeted_steps != total
        || total > config.energy_budget_steps {
        return Err(format!("{label}: receipt cost fields do not match independently recomputed route/turn/return/reserve costs"));
    }
    Ok(())
}
fn neighbors_for_target(map: &GridMap, center: usize) -> Vec<usize> {
    let c = xy(center);
    (0..N).filter(|i| {
        if *i == center || !clear(map, *i) { return false; }
        let p = xy(*i);
        (p.x.abs_diff(c.x) <= 2) && (p.y.abs_diff(c.y) <= 2) && line_of_sight(map, center, *i)
    }).collect()
}
fn make_target(map: &GridMap, rng: &mut Rng, target_id: String, kind: TargetKind) -> Check<Target> {
    let mut centers: Vec<usize> = (0..N).filter(|i| clear(map, *i)).collect();
    rng.shuffle(&mut centers);
    let required = match kind { TargetKind::TrunkDiameterEdges => 2, TargetKind::LogEndContour => 4 };
    for center in centers {
        let c = xy(center);
        let mut nearby = neighbors_for_target(map, center);
        if nearby.len() < required { continue; }
        for _ in 0..256 {
            rng.shuffle(&mut nearby);
            let features: Vec<usize> = nearby.iter().copied().take(required).collect();
            let valid = match kind {
                TargetKind::TrunkDiameterEdges => {
                    let a = xy(features[0]);
                    let b = xy(features[1]);
                    (a.y == c.y && b.y == c.y && ((a.x < c.x && b.x > c.x) || (b.x < c.x && a.x > c.x)))
                        || (a.x == c.x && b.x == c.x && ((a.y < c.y && b.y > c.y) || (b.y < c.y && a.y > c.y)))
                }
                TargetKind::LogEndContour => {
                    let xs: BTreeSet<usize> = features.iter().map(|i| xy(*i).x).collect();
                    let ys: BTreeSet<usize> = features.iter().map(|i| xy(*i).y).collect();
                    xs.len() >= 2 && ys.len() >= 2
                }
            };
            if valid {
                let mut sorted = features;
                sorted.sort_unstable();
                return Ok(Target { target_id, kind, center_cell: center, required_feature_cells: sorted });
            }
        }
    }
    Err(format!("seeded generator could not place target {target_id} ({kind:?})"))
}
fn generate_targets(map: &GridMap, seed: u64, scenario_id: &str, count: usize) -> Check<Vec<Target>> {
    if count == 0 || count > 12 { return Err("target_count_per_case must be in 1..=12".into()); }
    let mut rng = rng_for(seed, scenario_id);
    let mut targets = Vec::with_capacity(count);
    for i in 0..count {
        let kind = if i % 3 == 1 { TargetKind::LogEndContour } else { TargetKind::TrunkDiameterEdges };
        let id = format!("{scenario_id}-seed-{seed}-target-{i:02}");
        targets.push(make_target(map, &mut rng, id, kind)?);
    }
    Ok(targets)
}
fn ray_blocks(map: &GridMap, x: i32, y: i32) -> bool {
    x < 0 || y < 0 || x >= W as i32 || y >= W as i32
        || map.cells[idx(x as usize, y as usize)] != Occupancy::Clear
}
fn line_of_sight(map: &GridMap, from: usize, to: usize) -> bool {
    let a = xy(from); let b = xy(to);
    let (mut x, mut y) = (a.x as i32, a.y as i32);
    let (tx, ty) = (b.x as i32, b.y as i32);
    let dx = (tx - x).abs(); let dy = (ty - y).abs();
    let sx = if x < tx { 1 } else if x > tx { -1 } else { 0 };
    let sy = if y < ty { 1 } else if y > ty { -1 } else { 0 };
    let mut err = dx - dy;
    loop {
        if x == tx && y == ty { return true; }
        let e2 = 2 * err;
        let step_x = e2 > -dy;
        let step_y = e2 < dx;
        if step_x && step_y && (ray_blocks(map, x + sx, y) || ray_blocks(map, x, y + sy)) { return false; }
        if step_x { err -= dy; x += sx; }
        if step_y { err += dx; y += sy; }
        if (x, y) != (tx, ty) && ray_blocks(map, x, y) { return false; }
    }
}
fn can_see(map: &GridMap, view: Viewpoint, target_cell: usize) -> bool {
    if view.x >= W || view.y >= W { return false; }
    let from = idx(view.x, view.y);
    if !clear(map, from) || from == target_cell || !clear(map, target_cell) { return false; }
    let a = xy(from); let b = xy(target_cell);
    let dx = b.x as i32 - a.x as i32;
    let dy = b.y as i32 - a.y as i32;
    if dx * dx + dy * dy > RANGE * RANGE { return false; }
    let (fx, fy) = match view.heading {
        Heading::North => (0, -1), Heading::East => (1, 0),
        Heading::South => (0, 1), Heading::West => (-1, 0),
    };
    dx * fx + dy * fy >= 0 && line_of_sight(map, from, target_cell)
}
fn basis_points(num: usize, den: usize) -> u16 {
    if den == 0 { 0 } else { ((num as u64 * 10_000) / den as u64).min(10_000) as u16 }
}
fn score_plan(map: &GridMap, plan: &PlannerPlan, targets: &[Target]) -> Check<MethodScore> {
    if !plan.budget_feasible { return Err("planner supplied a budget-infeasible plan".into()); }
    if plan.viewpoints.iter().any(|v| v.x >= W || v.y >= W) { return Err("viewpoint outside grid".into()); }
    let mut target_scores = Vec::with_capacity(targets.len());
    let mut required_total = 0usize;
    let mut visible_total = 0usize;
    let mut complete_total = 0usize;
    for target in targets {
        if target.required_feature_cells.is_empty() { return Err("target with empty required-feature set".into()); }
        let visible = target.required_feature_cells.iter().filter(|cell|
            plan.viewpoints.iter().any(|view| can_see(map, *view, **cell))
        ).count();
        let required = target.required_feature_cells.len();
        let complete = visible == required;
        required_total += required;
        visible_total += visible;
        if complete { complete_total += 1; }
        target_scores.push(TargetScore {
            target_id: target.target_id.clone(), kind: target.kind.clone(),
            center_cell: target.center_cell, required_feature_cells: target.required_feature_cells.clone(),
            required_feature_count: required, visible_feature_count: visible,
            completeness_basis_points: basis_points(visible, required), complete,
        });
    }
    Ok(MethodScore {
        budget_feasible: plan.budget_feasible,
        target_count: targets.len(),
        complete_target_count: complete_total,
        required_feature_count: required_total,
        visible_feature_count: visible_total,
        complete_target_rate_basis_points: basis_points(complete_total, targets.len()),
        feature_visibility_basis_points: basis_points(visible_total, required_total),
        targets: target_scores,
    })
}
fn config_for(binding: &ScenarioBinding) -> PlannerConfig {
    PlannerConfig {
        start_cell: 0,
        energy_budget_steps: binding.energy_budget_steps,
        return_reserve_steps: binding.return_reserve_steps,
        max_viewpoints: binding.max_viewpoints,
        sensor_range_cells: RANGE,
        horizontal_fov_degrees: 180,
        turn_cost_steps: 1,
        greedy_algorithm: "marginal_gain / max(incremental_cost, 1), stable candidate-order tie-break".into(),
        oracle_algorithm: "exhaustive unique-viewpoint sequences; maximize unique visible cells, then minimize budgeted steps".into(),
        lookahead_algorithm: "receding horizon; select first action from best feasible sequence of up to two added viewpoints".into(),
    }
}
#[derive(Debug, Clone)]
struct OracleCandidate {
    cell: usize,
    view: Viewpoint,
    feature_mask: u64,
}
#[derive(Debug, Clone)]
struct OracleSearchState {
    selected: Vec<usize>,
    feature_mask: u64,
    outbound: u16,
    at: usize,
    heading: Heading,
}
struct TargetOracleSolver<'a> {
    map: &'a GridMap,
    targets: &'a [Target],
    config: PlannerConfig,
    candidates: Vec<OracleCandidate>,
    distances: Vec<Vec<Option<u16>>>,
    target_masks: Vec<u64>,
    best: Option<OracleSearchState>,
    expanded_state_count: usize,
}
impl<'a> TargetOracleSolver<'a> {
    fn new(map: &'a GridMap, targets: &'a [Target], config: PlannerConfig) -> Check<Self> {
        if config.max_viewpoints > 3 {
            return Err("exact target oracle is bounded to at most three viewpoints".into());
        }
        if !clear(map, config.start_cell) {
            return Err("target oracle start cell is not traversable".into());
        }
        if config.energy_budget_steps <= config.return_reserve_steps {
            return Err("target oracle budget must exceed return reserve".into());
        }
        let mut target_ids = BTreeSet::new();
        for target in targets {
            if target.target_id.trim().is_empty() || !target_ids.insert(target.target_id.as_str()) {
                return Err("target oracle rejects empty or duplicate target IDs".into());
            }
        }
        let feature_cells: Vec<usize> = targets.iter()
            .flat_map(|t| t.required_feature_cells.iter().copied()).collect();
        if feature_cells.is_empty() || feature_cells.len() > 64 {
            return Err("target oracle requires 1..=64 required feature samples".into());
        }
        if feature_cells.iter().any(|cell| *cell >= N || !clear(map, *cell)) {
            return Err("target oracle feature must lie on a clear grid cell".into());
        }
        let mut target_masks = Vec::with_capacity(targets.len());
        let mut offset = 0usize;
        for target in targets {
            if target.required_feature_cells.is_empty() {
                return Err(format!("{} has no required features", target.target_id));
            }
            let mut mask = 0u64;
            for _ in &target.required_feature_cells {
                mask |= 1u64 << offset;
                offset += 1;
            }
            target_masks.push(mask);
        }
        let distance_matrix: Vec<Vec<Option<u16>>> = (0..N).map(|i| distances(map, i)).collect();
        let mut candidates = Vec::new();
        for cell in 0..N {
            if !clear(map, cell) || distance_matrix[config.start_cell][cell].is_none() { continue; }
            let p = xy(cell);
            for heading in [Heading::North, Heading::East, Heading::South, Heading::West] {
                let view = Viewpoint { x: p.x, y: p.y, heading };
                let feature_mask = feature_cells.iter().enumerate().fold(0u64, |mask, (bit, target_cell)| {
                    if can_see(map, view, *target_cell) { mask | (1u64 << bit) } else { mask }
                });
                if feature_mask != 0 {
                    candidates.push(OracleCandidate { cell, view, feature_mask });
                }
            }
        }
        Ok(Self { map, targets, config, candidates, distances: distance_matrix, target_masks, best: None, expanded_state_count: 0 })
    }
    fn objective(&self, mask: u64) -> (usize, usize) {
        let complete = self.target_masks.iter().filter(|target_mask| (mask & **target_mask) == **target_mask).count();
        (complete, mask.count_ones() as usize)
    }
    fn total(&self, outbound: u16, selected: &[usize]) -> Option<(u16, u16)> {
        let last = selected.last().map(|i| self.candidates[*i].cell).unwrap_or(self.config.start_cell);
        let back = self.distances[last][self.config.start_cell]?;
        Some((back, outbound.saturating_add(back).saturating_add(self.config.return_reserve_steps)))
    }
    fn better(&self, candidate: &OracleSearchState, current: &OracleSearchState) -> bool {
        let (cc, cf) = self.objective(candidate.feature_mask);
        let (bc, bf) = self.objective(current.feature_mask);
        if cc != bc { return cc > bc; }
        if cf != bf { return cf > bf; }
        let ct = self.total(candidate.outbound, &candidate.selected).map(|v| v.1).unwrap_or(u16::MAX);
        let bt = self.total(current.outbound, &current.selected).map(|v| v.1).unwrap_or(u16::MAX);
        if ct != bt { return ct < bt; }
        if candidate.selected.len() != current.selected.len() { return candidate.selected.len() < current.selected.len(); }
        candidate.selected < current.selected
    }
    fn search(&mut self, state: OracleSearchState) {
        self.expanded_state_count = self.expanded_state_count.saturating_add(1);
        if self.best.as_ref().map_or(true, |best| self.better(&state, best)) {
            self.best = Some(state.clone());
        }
        if state.selected.len() >= usize::from(self.config.max_viewpoints) { return; }
        for candidate_index in 0..self.candidates.len() {
            if state.selected.contains(&candidate_index) { continue; }
            let (cell, view, candidate_mask) = {
                let candidate = &self.candidates[candidate_index];
                (candidate.cell, candidate.view, candidate.feature_mask)
            };
            if candidate_mask & !state.feature_mask == 0 { continue; }
            let Some(travel) = self.distances[state.at][cell] else { continue; };
            let turn = if state.heading == view.heading { 0 } else { self.config.turn_cost_steps };
            let outbound = state.outbound.saturating_add(travel).saturating_add(turn);
            let Some(back) = self.distances[cell][self.config.start_cell] else { continue; };
            let total = outbound.saturating_add(back).saturating_add(self.config.return_reserve_steps);
            if total > self.config.energy_budget_steps { continue; }
            let mut selected = state.selected.clone();
            selected.push(candidate_index);
            self.search(OracleSearchState {
                selected,
                feature_mask: state.feature_mask | candidate_mask,
                outbound,
                at: cell,
                heading: view.heading,
            });
        }
    }
    fn solve(mut self) -> Check<TargetOracleResult> {
        let initial = OracleSearchState {
            selected: Vec::new(), feature_mask: 0, outbound: 0,
            at: self.config.start_cell, heading: Heading::North,
        };
        self.search(initial);
        let candidate_count = self.candidates.len();
        let expanded_state_count = self.expanded_state_count;
        let best = self.best.take().ok_or_else(|| "target oracle found no feasible state".to_owned())?;
        let (back, total) = self.total(best.outbound, &best.selected)
            .ok_or_else(|| "target oracle best state has no return path".to_owned())?;
        let viewpoints: Vec<Viewpoint> = best.selected.iter()
            .map(|i| self.candidates[*i].view).collect();
        let plan = PlannerPlan {
            budget_feasible: total <= self.config.energy_budget_steps,
            outbound_cost_steps: best.outbound,
            return_distance_steps: back,
            reserved_steps: self.config.return_reserve_steps,
            total_budgeted_steps: total,
            viewpoints,
        };
        verify_plan_contract(self.map, &plan, &self.config, "target-objective oracle")?;
        let score = score_plan(self.map, &plan, self.targets)?;
        let (complete, visible) = self.objective(best.feature_mask);
        if score.complete_target_count != complete || score.visible_feature_count != visible {
            return Err("target oracle bit-mask objective disagrees with independently scored plan".into());
        }
        Ok(TargetOracleResult {
            candidate_count, expanded_state_count,
            objective: "maximize complete targets; then maximize visible required feature samples; then minimize budgeted steps; then minimize viewpoint count; final ties use stable candidate order".into(),
            plan,
            score,
        })
    }
}
fn target_regret(oracle: &MethodScore, compared: &MethodScore) -> TargetRegret {
    TargetRegret {
        oracle_complete_target_count: oracle.complete_target_count,
        compared_complete_target_count: compared.complete_target_count,
        complete_target_regret: oracle.complete_target_count.saturating_sub(compared.complete_target_count),
        complete_target_ratio_basis_points: if oracle.complete_target_count == 0 {
            None
        } else {
            Some(basis_points(compared.complete_target_count, oracle.complete_target_count))
        },
        oracle_visible_feature_count: oracle.visible_feature_count,
        compared_visible_feature_count: compared.visible_feature_count,
        visible_feature_delta_to_oracle: oracle.visible_feature_count as i64 - compared.visible_feature_count as i64,
    }
}

fn validate_corpus(c: &Corpus) -> Check<()> {
    if c.suite_id.trim().is_empty() || c.suite_version.trim().is_empty()
        || c.generator_version.trim().is_empty() || c.claim_ceiling.trim().is_empty() {
        return Err("suite identity, generator version and claim ceiling must not be empty".into());
    }
    if c.evidence_class != "synthetic" { return Err("only synthetic target corpora are accepted".into()); }
    if c.scenario_bindings.is_empty() || c.scenario_bindings.len() > 20 { return Err("scenario binding count must be in 1..=20".into()); }
    if c.training_seeds.is_empty() || c.held_out_seeds.is_empty() { return Err("training and held-out seeds must both be non-empty".into()); }
    if c.training_seeds.len() > 32 || c.held_out_seeds.len() > 32 { return Err("each seed partition is capped at 32".into()); }
    if c.target_count_per_case == 0 || c.target_count_per_case > 12 { return Err("target_count_per_case must be in 1..=12".into()); }
    let mut training = BTreeSet::new();
    for seed in &c.training_seeds {
        if *seed == 0 || !training.insert(*seed) { return Err(format!("zero or duplicate training seed {seed}")); }
    }
    let mut held_out = BTreeSet::new();
    for seed in &c.held_out_seeds {
        if *seed == 0 || !held_out.insert(*seed) { return Err(format!("zero or duplicate held-out seed {seed}")); }
        if training.contains(seed) { return Err(format!("train/test seed overlap at {seed}")); }
    }
    let mut ids = BTreeSet::new();
    for binding in &c.scenario_bindings {
        if binding.planner_scenario_id.trim().is_empty() || !ids.insert(binding.planner_scenario_id.as_str()) {
            return Err("empty or duplicate planner_scenario_id".into());
        }
        if binding.energy_budget_steps <= binding.return_reserve_steps || binding.max_viewpoints == 0 || binding.max_viewpoints > 3 {
            return Err(format!("{}: invalid energy, reserve, or viewpoint limit", binding.planner_scenario_id));
        }
    }
    Ok(())
}
fn validate_planner_receipt<'a>(receipt: &'a PlannerReceipt, c: &Corpus, source_commit: &str) -> Check<BTreeMap<String, &'a PlannerScenario>> {
    if receipt.source_commit != source_commit { return Err("planner receipt source_commit does not match exact checked-out head".into()); }
    if receipt.corpus_sha256.len() != 64 || !receipt.corpus_sha256.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("planner corpus SHA-256 is malformed".into());
    }
    let mut scenarios = BTreeMap::new();
    for scenario in &receipt.scenarios {
        if scenario.scenario_id.trim().is_empty() || scenarios.insert(scenario.scenario_id.clone(), scenario).is_some() {
            return Err("empty or duplicate planner scenario id".into());
        }
    }
    let expected: BTreeSet<&str> = c.scenario_bindings.iter().map(|b| b.planner_scenario_id.as_str()).collect();
    let actual: BTreeSet<&str> = scenarios.keys().map(String::as_str).collect();
    if expected != actual { return Err(format!("planner scenario IDs do not exactly match target bindings: expected {expected:?}, got {actual:?}")); }
    for scenario in scenarios.values() {
        if !scenario.greedy.budget_feasible || !scenario.two_step_lookahead.budget_feasible || !scenario.exhaustive_oracle.budget_feasible {
            return Err(format!("{}: planner receipt contains infeasible plan", scenario.scenario_id));
        }
        for digest in [&scenario.map_sha256, &scenario.config_sha256] {
            if digest.len() != 64 || !digest.bytes().all(|b| b.is_ascii_hexdigit()) {
                return Err(format!("{}: malformed map/config digest", scenario.scenario_id));
            }
        }
    }
    Ok(scenarios)
}
#[derive(Default)]
struct Acc {
    evaluation_count: usize, target_count: usize, complete_target_count: usize,
    required_feature_count: usize, visible_feature_count: usize, worst_case_feature_visibility_basis_points: u16,
}
impl Acc {
    fn add(&mut self, score: &MethodScore) {
        self.evaluation_count += 1;
        self.target_count += score.target_count;
        self.complete_target_count += score.complete_target_count;
        self.required_feature_count += score.required_feature_count;
        self.visible_feature_count += score.visible_feature_count;
        if self.evaluation_count == 1 { self.worst_case_feature_visibility_basis_points = score.feature_visibility_basis_points; }
        else { self.worst_case_feature_visibility_basis_points = self.worst_case_feature_visibility_basis_points.min(score.feature_visibility_basis_points); }
    }
    fn finish(self) -> AggregateMethod {
        AggregateMethod {
            evaluation_count: self.evaluation_count,
            target_count: self.target_count,
            complete_target_count: self.complete_target_count,
            required_feature_count: self.required_feature_count,
            visible_feature_count: self.visible_feature_count,
            complete_target_rate_basis_points: basis_points(self.complete_target_count, self.target_count),
            feature_visibility_basis_points: basis_points(self.visible_feature_count, self.required_feature_count),
            worst_case_feature_visibility_basis_points: self.worst_case_feature_visibility_basis_points,
        }
    }
}
#[derive(Default)]
struct RegretAcc {
    evaluation_count: usize,
    oracle_complete_target_count: usize,
    compared_complete_target_count: usize,
    total_complete_target_regret: usize,
    worst_case_complete_target_regret: usize,
    worst_case_complete_target_ratio_basis_points: Option<u16>,
    oracle_visible_feature_count: usize,
    compared_visible_feature_count: usize,
    visible_feature_delta_to_oracle_total: i64,
    worst_case_visible_feature_delta_to_oracle: i64,
}
impl RegretAcc {
    fn add(&mut self, oracle: &MethodScore, compared: &MethodScore) {
        self.evaluation_count += 1;
        self.oracle_complete_target_count += oracle.complete_target_count;
        self.compared_complete_target_count += compared.complete_target_count;
        let regret = oracle.complete_target_count.saturating_sub(compared.complete_target_count);
        self.total_complete_target_regret += regret;
        self.worst_case_complete_target_regret = self.worst_case_complete_target_regret.max(regret);
        if let Some(ratio) = target_regret(oracle, compared).complete_target_ratio_basis_points {
            self.worst_case_complete_target_ratio_basis_points = Some(
                self.worst_case_complete_target_ratio_basis_points.map_or(ratio, |prior| prior.min(ratio))
            );
        }
        self.oracle_visible_feature_count += oracle.visible_feature_count;
        self.compared_visible_feature_count += compared.visible_feature_count;
        let delta = oracle.visible_feature_count as i64 - compared.visible_feature_count as i64;
        self.visible_feature_delta_to_oracle_total += delta;
        if self.evaluation_count == 1 { self.worst_case_visible_feature_delta_to_oracle = delta; }
        else { self.worst_case_visible_feature_delta_to_oracle = self.worst_case_visible_feature_delta_to_oracle.max(delta); }
    }
    fn finish(self) -> AggregateRegret {
        AggregateRegret {
            evaluation_count: self.evaluation_count,
            oracle_complete_target_count: self.oracle_complete_target_count,
            compared_complete_target_count: self.compared_complete_target_count,
            total_complete_target_regret: self.total_complete_target_regret,
            worst_case_complete_target_regret: self.worst_case_complete_target_regret,
            complete_target_ratio_basis_points: if self.oracle_complete_target_count == 0 {
                None
            } else {
                Some(basis_points(self.compared_complete_target_count, self.oracle_complete_target_count))
            },
            worst_case_complete_target_ratio_basis_points: self.worst_case_complete_target_ratio_basis_points,
            oracle_visible_feature_count: self.oracle_visible_feature_count,
            compared_visible_feature_count: self.compared_visible_feature_count,
            visible_feature_delta_to_oracle_total: self.visible_feature_delta_to_oracle_total,
            worst_case_visible_feature_delta_to_oracle: self.worst_case_visible_feature_delta_to_oracle,
        }
    }
}
fn summarize(evaluations: &[Evaluation], split: &str, seed_count: usize) -> SplitSummary {
    let mut greedy = Acc::default();
    let mut lookahead = Acc::default();
    let mut coverage_oracle = Acc::default();
    let mut target_oracle = Acc::default();
    let mut greedy_regret = RegretAcc::default();
    let mut lookahead_regret = RegretAcc::default();
    let mut coverage_regret = RegretAcc::default();
    let mut count = 0usize;
    for e in evaluations.iter().filter(|e| e.split == split) {
        count += 1;
        greedy.add(&e.greedy);
        lookahead.add(&e.lookahead);
        coverage_oracle.add(&e.coverage_oracle);
        target_oracle.add(&e.target_oracle.score);
        greedy_regret.add(&e.target_oracle.score, &e.greedy);
        lookahead_regret.add(&e.target_oracle.score, &e.lookahead);
        coverage_regret.add(&e.target_oracle.score, &e.coverage_oracle);
    }
    SplitSummary {
        seed_count, evaluation_count: count,
        greedy: greedy.finish(), lookahead: lookahead.finish(),
        coverage_oracle: coverage_oracle.finish(), target_objective_oracle: target_oracle.finish(),
        greedy_vs_target_oracle: greedy_regret.finish(), lookahead_vs_target_oracle: lookahead_regret.finish(),
        coverage_oracle_vs_target_oracle: coverage_regret.finish(),
    }
}
fn build_receipt(
    c: &Corpus, target_bytes: &[u8], planner_bytes: &[u8], source_commit: String,
    planner: &PlannerReceipt, evaluations: Vec<Evaluation>,
) -> Receipt {
    let training_count = c.training_seeds.len() * c.scenario_bindings.len();
    let held_count = c.held_out_seeds.len() * c.scenario_bindings.len();
    Receipt {
        result: "validated", suite_id: c.suite_id.clone(), suite_version: c.suite_version.clone(),
        evidence_class: c.evidence_class.clone(), claim_ceiling: c.claim_ceiling.clone(),
        generator_version: c.generator_version.clone(),
        target_objective: "maximize complete targets; then maximize visible required feature samples; then minimize budgeted steps; then minimize viewpoint count; final ties use stable candidate order".into(),
        source_commit,
        planner_source_commit: planner.source_commit.clone(), planner_corpus_sha256: planner.corpus_sha256.clone(),
        planner_receipt_sha256: hash(planner_bytes), target_corpus_sha256: hash(target_bytes),
        scenario_count: c.scenario_bindings.len(), training_seed_count: c.training_seeds.len(),
        held_out_seed_count: c.held_out_seeds.len(), seed_overlap_count: 0,
        evaluation_count: evaluations.len(), training_evaluation_count: training_count,
        held_out_evaluation_count: held_count,
        training: summarize(&evaluations, "training", c.training_seeds.len()),
        held_out: summarize(&evaluations, "held_out", c.held_out_seeds.len()),
        evaluations,
    }
}
fn validate_and_score(c: &Corpus, planner_receipt: &PlannerReceipt, source_commit: &str) -> Check<Vec<Evaluation>> {
    let by_id = validate_planner_receipt(planner_receipt, c, source_commit)?;
    let mut evaluations = Vec::new();
    for binding in &c.scenario_bindings {
        let scenario = by_id.get(binding.planner_scenario_id.as_str()).ok_or_else(|| format!("missing plan {}", binding.planner_scenario_id))?;
        let map = map_for(binding.scene);
        let config = config_for(binding);
        if digest_json(&map)? != scenario.map_sha256 { return Err(format!("{}: planner map digest does not match target-audit geometry", binding.planner_scenario_id)); }
        if digest_json(&config)? != scenario.config_sha256 { return Err(format!("{}: planner config digest does not match target-audit binding", binding.planner_scenario_id)); }
        verify_plan_contract(&map, &scenario.greedy, &config, "greedy")?;
        verify_plan_contract(&map, &scenario.two_step_lookahead, &config, "lookahead")?;
        verify_plan_contract(&map, &scenario.exhaustive_oracle, &config, "coverage oracle")?;
        let seeds: Vec<(&str, u64)> = c.training_seeds.iter().map(|s| ("training", *s))
            .chain(c.held_out_seeds.iter().map(|s| ("held_out", *s))).collect();
        for (split, seed) in seeds {
            let targets = generate_targets(&map, seed, &binding.planner_scenario_id, c.target_count_per_case)?;
            let greedy = score_plan(&map, &scenario.greedy, &targets)?;
            let lookahead = score_plan(&map, &scenario.two_step_lookahead, &targets)?;
            let coverage_oracle = score_plan(&map, &scenario.exhaustive_oracle, &targets)?;
            let target_oracle = TargetOracleSolver::new(&map, &targets, config.clone())?.solve()?;
            for (name, compared) in [("greedy", &greedy), ("lookahead", &lookahead), ("coverage oracle", &coverage_oracle)] {
                if target_oracle.score.complete_target_count < compared.complete_target_count
                    || (target_oracle.score.complete_target_count == compared.complete_target_count
                        && target_oracle.score.visible_feature_count < compared.visible_feature_count) {
                    return Err(format!("{name}: feasible plan beats the claimed exact target-objective oracle for {} seed {seed}", binding.planner_scenario_id));
                }
            }
            let greedy_vs_target_oracle = target_regret(&target_oracle.score, &greedy);
            let lookahead_vs_target_oracle = target_regret(&target_oracle.score, &lookahead);
            let coverage_oracle_vs_target_oracle = target_regret(&target_oracle.score, &coverage_oracle);
            evaluations.push(Evaluation {
                planner_scenario_id: binding.planner_scenario_id.clone(), scene: binding.scene,
                seed, split: split.to_owned(), generated_target_count: targets.len(),
                map_sha256: scenario.map_sha256.clone(), config_sha256: scenario.config_sha256.clone(),
                greedy, lookahead, coverage_oracle, target_oracle,
                greedy_vs_target_oracle, lookahead_vs_target_oracle, coverage_oracle_vs_target_oracle,
            });
        }
    }
    Ok(evaluations)
}
fn valid_sha(value: &str) -> bool { value.len() == 40 && value.bytes().all(|b| b.is_ascii_hexdigit()) }
fn validate_planner_corpus(receipt: &PlannerReceipt, bytes: &[u8]) -> Check<()> {
    let actual = hash(bytes);
    if receipt.corpus_sha256 != actual {
        return Err(format!("planner corpus digest mismatch: receipt {}, actual {actual}", receipt.corpus_sha256));
    }
    Ok(())
}
fn run() -> Check<()> {
    let mut args = env::args().skip(1);
    let planner_path = args.next().ok_or_else(|| "usage: forest-target-audit PLANNER_RECEIPT.json --planner-corpus PLANNER_CORPUS.json --target-corpus TARGET_CORPUS.json --source-commit <40-hex-sha>".to_owned())?;
    if args.next().as_deref() != Some("--planner-corpus") { return Err("expected --planner-corpus".into()); }
    let planner_corpus_path = args.next().ok_or_else(|| "missing planner corpus path".to_owned())?;
    if args.next().as_deref() != Some("--target-corpus") { return Err("expected --target-corpus".into()); }
    let target_path = args.next().ok_or_else(|| "missing target corpus path".to_owned())?;
    if args.next().as_deref() != Some("--source-commit") { return Err("expected --source-commit".into()); }
    let source = args.next().ok_or_else(|| "missing source commit".to_owned())?;
    if !valid_sha(&source) { return Err("source commit must be 40 hex chars".into()); }
    if args.next().is_some() { return Err("unexpected trailing arguments".into()); }
    let planner_bytes = fs::read(&planner_path).map_err(|e| format!("cannot read {planner_path}: {e}"))?;
    let planner_corpus_bytes = fs::read(&planner_corpus_path).map_err(|e| format!("cannot read {planner_corpus_path}: {e}"))?;
    let target_bytes = fs::read(&target_path).map_err(|e| format!("cannot read {target_path}: {e}"))?;
    let planner: PlannerReceipt = serde_json::from_slice(&planner_bytes).map_err(|e| format!("invalid planner receipt JSON: {e}"))?;
    validate_planner_corpus(&planner, &planner_corpus_bytes)?;
    let corpus: Corpus = serde_json::from_slice(&target_bytes).map_err(|e| format!("invalid target corpus JSON: {e}"))?;
    validate_corpus(&corpus)?;
    let evaluations = validate_and_score(&corpus, &planner, &source)?;
    if evaluations.len() != corpus.scenario_bindings.len() * (corpus.training_seeds.len() + corpus.held_out_seeds.len()) {
        return Err("evaluation count does not match scenario-by-seed product".into());
    }
    let receipt = build_receipt(&corpus, &target_bytes, &planner_bytes, source, &planner, evaluations);
    println!("{}", serde_json::to_string_pretty(&receipt).map_err(|e| e.to_string())?);
    Ok(())
}
fn main() -> ExitCode {
    match run() { Ok(()) => ExitCode::SUCCESS, Err(e) => { eprintln!("forest-target-audit: {e}"); ExitCode::from(2) } }
}

#[cfg(test)]
mod tests {
    use super::*;
    const FIXTURE: &str = include_str!("../../../docs/tech/forest-target-completeness-corpus-v0.1.json");
    const PLANNER_CORPUS: &str = include_str!("../../../docs/tech/forest-next-best-view-corpus-v0.1.json");
    fn corpus() -> Corpus { serde_json::from_str(FIXTURE).expect("fixture parses") }

    #[test]
    fn planner_corpus_digest_must_match_exact_input_bytes() {
        let bytes = PLANNER_CORPUS.as_bytes();
        let receipt = PlannerReceipt {
            source_commit: "0".repeat(40),
            corpus_sha256: hash(bytes),
            scenarios: Vec::new(),
        };
        assert!(validate_planner_corpus(&receipt, bytes).is_ok());
        let mut modified = bytes.to_vec();
        modified.push(b' ');
        assert!(validate_planner_corpus(&receipt, &modified).is_err());
    }

    #[test]
    fn train_and_held_out_seeds_are_disjoint_and_unique() {
        let c = corpus();
        validate_corpus(&c).expect("frozen seed split");
        let train: BTreeSet<u64> = c.training_seeds.iter().copied().collect();
        assert!(c.held_out_seeds.iter().all(|s| !train.contains(s)));
    }
    #[test]
    fn generator_is_deterministic_and_seed_sensitive() {
        let c = corpus();
        let binding = &c.scenario_bindings[0];
        let map = map_for(binding.scene);
        let a = generate_targets(&map, c.held_out_seeds[0], &binding.planner_scenario_id, c.target_count_per_case).unwrap();
        let b = generate_targets(&map, c.held_out_seeds[0], &binding.planner_scenario_id, c.target_count_per_case).unwrap();
        let different = generate_targets(&map, c.held_out_seeds[1], &binding.planner_scenario_id, c.target_count_per_case).unwrap();
        let shape = |targets: &[Target]| targets.iter().map(|t| (t.kind, t.center_cell, t.required_feature_cells.clone())).collect::<Vec<_>>();
        assert_eq!(a, b);
        assert_ne!(shape(&a), shape(&different));
    }
    #[test]
    fn seeded_target_generation_works_for_every_scene_and_seed() {
        let c = corpus();
        for binding in &c.scenario_bindings {
            let map = map_for(binding.scene);
            for seed in c.training_seeds.iter().chain(c.held_out_seeds.iter()) {
                let targets = generate_targets(&map, *seed, &binding.planner_scenario_id, c.target_count_per_case).unwrap();
                assert_eq!(targets.len(), c.target_count_per_case);
                assert!(targets.iter().all(|t| t.required_feature_cells.len() >= 2
                    && t.required_feature_cells.iter().all(|cell| clear(&map, *cell))));
            }
        }
    }
    #[test]
    fn blocked_and_unknown_cells_occlude_features_behind_them() {
        let blocked = map_for(Scene::BlockedCorridor);
        let unknown = map_for(Scene::UnknownBarrier);
        let view = Viewpoint { x: 1, y: 2, heading: Heading::East };
        assert!(!can_see(&blocked, view, idx(3, 2)));
        assert!(!can_see(&unknown, view, idx(3, 2)));
    }
    #[test]
    fn target_features_outside_range_are_not_visible() {
        let map = map_for(Scene::OpenGrid);
        let view = Viewpoint { x: 0, y: 2, heading: Heading::East };
        assert!(!can_see(&map, view, idx(4, 2)));
    }
    #[test]
    fn same_cell_view_is_not_credited_as_target_observation() {
        let map = map_for(Scene::OpenGrid);
        let view = Viewpoint { x: 1, y: 2, heading: Heading::East };
        assert!(!can_see(&map, view, idx(1, 2)));
    }
    #[test]
    fn incomplete_target_cannot_be_marked_complete() {
        let map = map_for(Scene::OpenGrid);
        let plan = PlannerPlan { budget_feasible: true, outbound_cost_steps: 0, return_distance_steps: 0, reserved_steps: 0, total_budgeted_steps: 0, viewpoints: vec![Viewpoint { x: 0, y: 2, heading: Heading::East }] };
        let target = Target {
            target_id: "two-edge-negative-control".into(), kind: TargetKind::TrunkDiameterEdges,
            center_cell: idx(2, 2), required_feature_cells: vec![idx(1, 2), idx(4, 2)],
        };
        let score = score_plan(&map, &plan, &[target]).unwrap();
        assert_eq!(score.targets[0].visible_feature_count, 1);
        assert!(!score.targets[0].complete);
        assert_eq!(score.targets[0].completeness_basis_points, 5_000);
    }

    #[test]
    fn exact_target_oracle_dominates_a_feasible_baseline_under_its_lexicographic_objective() {
        let c = corpus();
        let binding = &c.scenario_bindings[0];
        let map = map_for(binding.scene);
        let targets = generate_targets(&map, c.training_seeds[0], &binding.planner_scenario_id, c.target_count_per_case).unwrap();
        let config = config_for(binding);
        let baseline = PlannerPlan {
            budget_feasible: true,
            outbound_cost_steps: 0,
            return_distance_steps: 0,
            reserved_steps: config.return_reserve_steps,
            total_budgeted_steps: config.return_reserve_steps,
            viewpoints: vec![Viewpoint { x: 0, y: 0, heading: Heading::North }],
        };
        verify_plan_contract(&map, &baseline, &config, "test baseline").unwrap();
        let baseline_score = score_plan(&map, &baseline, &targets).unwrap();
        let oracle = TargetOracleSolver::new(&map, &targets, config.clone()).unwrap().solve().unwrap();
        assert!(oracle.plan.budget_feasible);
        assert!(oracle.score.complete_target_count >= baseline_score.complete_target_count);
        if oracle.score.complete_target_count == baseline_score.complete_target_count {
            assert!(oracle.score.visible_feature_count >= baseline_score.visible_feature_count);
        }
        assert_eq!(oracle.score.complete_target_count,
            oracle.score.targets.iter().filter(|t| t.complete).count());
        let mut unsupported = config;
        unsupported.max_viewpoints = 4;
        assert!(TargetOracleSolver::new(&map, &targets, unsupported).is_err());
    }

    #[test]
    fn exact_target_oracle_rejects_duplicate_target_identity() {
        let c = corpus();
        let binding = &c.scenario_bindings[0];
        let map = map_for(binding.scene);
        let mut targets = generate_targets(&map, c.training_seeds[0], &binding.planner_scenario_id, c.target_count_per_case).unwrap();
        targets[1].target_id = targets[0].target_id.clone();
        assert!(TargetOracleSolver::new(&map, &targets, config_for(binding)).is_err());
    }

    #[test]
    fn exact_target_oracle_completes_both_required_edges_when_one_feasible_pose_sees_them() {
        let map = map_for(Scene::OpenGrid);
        let target = Target {
            target_id: "known-two-edge-target".into(),
            kind: TargetKind::TrunkDiameterEdges,
            center_cell: idx(2, 2),
            required_feature_cells: vec![idx(1, 2), idx(3, 2)],
        };
        let config = PlannerConfig {
            start_cell: 0,
            energy_budget_steps: 7,
            return_reserve_steps: 2,
            max_viewpoints: 1,
            sensor_range_cells: RANGE,
            horizontal_fov_degrees: 180,
            turn_cost_steps: 1,
            greedy_algorithm: "test".into(),
            oracle_algorithm: "test".into(),
            lookahead_algorithm: "test".into(),
        };
        let targets = vec![target];
        let result = TargetOracleSolver::new(&map, &targets, config).unwrap().solve().unwrap();
        assert_eq!(result.score.complete_target_count, 1);
        assert_eq!(result.score.visible_feature_count, 2);
        assert_eq!(result.plan.viewpoints.len(), 1);
        assert!(result.plan.total_budgeted_steps <= 7);
        assert!(result.candidate_count > 0);
        assert!(result.expanded_state_count > 0);
    }

    #[test]
    fn independent_route_verifier_rejects_tampered_cost_receipts() {
        let c = corpus();
        let binding = &c.scenario_bindings[0];
        let map = map_for(binding.scene);
        let config = config_for(binding);
        let targets = generate_targets(&map, c.training_seeds[0], &binding.planner_scenario_id, c.target_count_per_case).unwrap();
        let result = TargetOracleSolver::new(&map, &targets, config.clone()).unwrap().solve().unwrap();
        let mut tampered = result.plan;
        tampered.total_budgeted_steps = tampered.total_budgeted_steps.saturating_add(1);
        assert!(verify_plan_contract(&map, &tampered, &config, "tamper control").is_err());
    }


    #[test]
    fn target_oracle_is_deterministic_and_reports_expansion_bound() {
        let c = corpus();
        let binding = &c.scenario_bindings[0];
        let map = map_for(binding.scene);
        let targets = generate_targets(&map, c.training_seeds[0], &binding.planner_scenario_id, c.target_count_per_case).unwrap();
        let a = TargetOracleSolver::new(&map, &targets, config_for(binding)).unwrap().solve().unwrap();
        let b = TargetOracleSolver::new(&map, &targets, config_for(binding)).unwrap().solve().unwrap();
        assert_eq!(serde_json::to_vec(&a).unwrap(), serde_json::to_vec(&b).unwrap());
        assert!(a.expanded_state_count <= 980_201);
        assert!(a.candidate_count <= 100);
    }

    #[test]
    fn unreachable_target_has_no_invented_completeness_ratio() {
        let map = map_for(Scene::BlockedCorridor);
        let target = Target {
            target_id: "unreachable-negative-control".into(),
            kind: TargetKind::TrunkDiameterEdges,
            center_cell: idx(3, 2),
            required_feature_cells: vec![idx(3, 1), idx(3, 3)],
        };
        let targets = vec![target];
        let config = PlannerConfig {
            start_cell: 0,
            energy_budget_steps: 10,
            return_reserve_steps: 2,
            max_viewpoints: 3,
            sensor_range_cells: RANGE,
            horizontal_fov_degrees: 180,
            turn_cost_steps: 1,
            greedy_algorithm: "test".into(),
            oracle_algorithm: "test".into(),
            lookahead_algorithm: "test".into(),
        };
        let oracle = TargetOracleSolver::new(&map, &targets, config).unwrap().solve().unwrap();
        assert_eq!(oracle.score.complete_target_count, 0);
        assert_eq!(oracle.score.visible_feature_count, 0);
        let baseline = score_plan(&map, &oracle.plan, &targets).unwrap();
        assert_eq!(target_regret(&oracle.score, &baseline).complete_target_ratio_basis_points, None);
    }

    #[test]
    fn serialized_scores_are_bounded_and_deterministic() {
        let map = map_for(Scene::OpenGrid);
        let plan = PlannerPlan { budget_feasible: true, outbound_cost_steps: 0, return_distance_steps: 0, reserved_steps: 0, total_budgeted_steps: 0, viewpoints: vec![Viewpoint { x: 0, y: 2, heading: Heading::East }] };
        let target = Target {
            target_id: "range-control".into(), kind: TargetKind::TrunkDiameterEdges,
            center_cell: idx(2, 2), required_feature_cells: vec![idx(1, 2), idx(4, 2)],
        };
        let a = serde_json::to_vec(&score_plan(&map, &plan, &[target.clone()]).unwrap()).unwrap();
        let b = serde_json::to_vec(&score_plan(&map, &plan, &[target]).unwrap()).unwrap();
        assert_eq!(a, b);
        let parsed: MethodScore = serde_json::from_slice(&a).unwrap();
        assert!(parsed.feature_visibility_basis_points <= 10_000);
        assert!(parsed.complete_target_rate_basis_points <= 10_000);
    }
}
