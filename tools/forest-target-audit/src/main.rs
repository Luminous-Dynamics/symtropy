use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::{BTreeMap, BTreeSet}, env, fs, process::ExitCode};

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
#[derive(Debug, Deserialize)]
struct PlannerPlan {
    budget_feasible: bool,
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
struct SplitSummary {
    seed_count: usize,
    evaluation_count: usize,
    greedy: AggregateMethod,
    lookahead: AggregateMethod,
    coverage_oracle: AggregateMethod,
}
#[derive(Debug, Serialize)]
struct Receipt {
    result: &'static str,
    suite_id: String,
    suite_version: String,
    evidence_class: String,
    claim_ceiling: String,
    generator_version: String,
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
    }
    Ok(())
}
fn validate_planner_receipt<'a>(receipt: &'a PlannerReceipt, c: &Corpus, source_commit: &str) -> Check<BTreeMap<String, &'a PlannerScenario>> {
    if receipt.source_commit != source_commit { return Err("planner receipt source_commit does not match exact checked-out head".into()); }
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
fn summarize(evaluations: &[Evaluation], split: &str, seed_count: usize) -> SplitSummary {
    let mut greedy = Acc::default();
    let mut lookahead = Acc::default();
    let mut oracle = Acc::default();
    let mut count = 0usize;
    for e in evaluations.iter().filter(|e| e.split == split) {
        count += 1; greedy.add(&e.greedy); lookahead.add(&e.lookahead); oracle.add(&e.coverage_oracle);
    }
    SplitSummary { seed_count, evaluation_count: count, greedy: greedy.finish(), lookahead: lookahead.finish(), coverage_oracle: oracle.finish() }
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
        generator_version: c.generator_version.clone(), source_commit,
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
        if digest_json(&map)? != scenario.map_sha256 { return Err(format!("{}: planner map digest does not match target-audit geometry", binding.planner_scenario_id)); }
        if digest_json(&config_for(binding))? != scenario.config_sha256 { return Err(format!("{}: planner config digest does not match target-audit binding", binding.planner_scenario_id)); }
        let seeds: Vec<(&str, u64)> = c.training_seeds.iter().map(|s| ("training", *s))
            .chain(c.held_out_seeds.iter().map(|s| ("held_out", *s))).collect();
        for (split, seed) in seeds {
            let targets = generate_targets(&map, seed, &binding.planner_scenario_id, c.target_count_per_case)?;
            evaluations.push(Evaluation {
                planner_scenario_id: binding.planner_scenario_id.clone(), scene: binding.scene,
                seed, split: split.to_owned(), generated_target_count: targets.len(),
                map_sha256: scenario.map_sha256.clone(), config_sha256: scenario.config_sha256.clone(),
                greedy: score_plan(&map, &scenario.greedy, &targets)?,
                lookahead: score_plan(&map, &scenario.two_step_lookahead, &targets)?,
                coverage_oracle: score_plan(&map, &scenario.exhaustive_oracle, &targets)?,
            });
        }
    }
    Ok(evaluations)
}
fn valid_sha(value: &str) -> bool { value.len() == 40 && value.bytes().all(|b| b.is_ascii_hexdigit()) }
fn run() -> Check<()> {
    let mut args = env::args().skip(1);
    let planner_path = args.next().ok_or_else(|| "usage: forest-target-audit PLANNER_RECEIPT.json --target-corpus CORPUS.json --source-commit <40-hex-sha>".to_owned())?;
    if args.next().as_deref() != Some("--target-corpus") { return Err("expected --target-corpus".into()); }
    let target_path = args.next().ok_or_else(|| "missing target corpus path".to_owned())?;
    if args.next().as_deref() != Some("--source-commit") { return Err("expected --source-commit".into()); }
    let source = args.next().ok_or_else(|| "missing source commit".to_owned())?;
    if !valid_sha(&source) { return Err("source commit must be 40 hex chars".into()); }
    if args.next().is_some() { return Err("unexpected trailing arguments".into()); }
    let planner_bytes = fs::read(&planner_path).map_err(|e| format!("cannot read {planner_path}: {e}"))?;
    let target_bytes = fs::read(&target_path).map_err(|e| format!("cannot read {target_path}: {e}"))?;
    let planner: PlannerReceipt = serde_json::from_slice(&planner_bytes).map_err(|e| format!("invalid planner receipt JSON: {e}"))?;
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
    fn corpus() -> Corpus { serde_json::from_str(FIXTURE).expect("fixture parses") }

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
        assert_eq!(a, b);
        assert_ne!(a, different);
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
        let plan = PlannerPlan { budget_feasible: true, viewpoints: vec![Viewpoint { x: 0, y: 2, heading: Heading::East }] };
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
    fn serialized_scores_are_bounded_and_deterministic() {
        let map = map_for(Scene::OpenGrid);
        let plan = PlannerPlan { budget_feasible: true, viewpoints: vec![Viewpoint { x: 0, y: 2, heading: Heading::East }] };
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
