//! Deterministic, evidence-gated block hoist sequence and grid-path planner.
//!
//! V0.1 is a finite, discrete, offline candidate generator. It jointly
//! enumerates lift order and uses A* for unloaded/loaded grid routes. It does
//! not produce physical motion envelopes, certify clearance, authorize a lift,
//! model suspended-load dynamics, or control a crane.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap, VecDeque};

pub const MAX_JOBS: usize = 6;
pub const MAX_GRID_DIMENSION: u16 = 1_024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Cell {
    pub x: u16,
    pub y: u16,
}

impl Cell {
    pub const fn new(x: u16, y: u16) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Workspace {
    pub site_epoch_id: String,
    pub frame_id: String,
    pub clock_domain_id: String,
    pub workspace_revision: String,
    pub clearance_profile_revision: String,
    pub width_cells: u16,
    pub height_cells: u16,
    /// Physical length represented by one orthogonal grid step.
    pub cell_size_mm: u32,
    /// Cells admitted for empty hook/trolley travel. Omitted cells are unknown
    /// and are never treated as free.
    pub unloaded_free_cells: BTreeSet<Cell>,
    /// Cells admitted for suspended-block travel. This is normally a subset of
    /// empty travel space because the load has a larger envelope.
    pub loaded_free_cells: BTreeSet<Cell>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreflightDisposition {
    ReadyForHumanReview,
    Rejected,
    Indeterminate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreflightBinding {
    pub block_id: String,
    pub block_revision: String,
    pub crane_id: String,
    pub crane_configuration_revision: String,
    pub site_epoch_id: String,
    pub frame_id: String,
    pub clock_domain_id: String,
    pub disposition: PreflightDisposition,
    /// SHA-256 shape only; source authenticity is owned by the preflight system.
    pub input_sha256: String,
    pub report_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HoistJob {
    pub job_id: String,
    pub block_id: String,
    pub block_revision: String,
    pub pick_cell: Cell,
    pub drop_cell: Cell,
    pub predecessor_job_ids: Vec<String>,
    pub preflight: PreflightBinding,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannerInput {
    pub crane_id: String,
    pub crane_configuration_revision: String,
    pub initial_hook_cell: Cell,
    pub workspace: Workspace,
    pub jobs: Vec<HoistJob>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Rejected,
    Indeterminate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: &'static str,
    pub severity: DiagnosticSeverity,
    pub subject_id: Option<String>,
    pub detail: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanDisposition {
    /// A discrete candidate is available for human engineering review.
    CandidateForHumanReview,
    /// No route/order exists inside the explicitly declared free-cell sets.
    InfeasibleWithinDeclaredWorkspace,
    /// The input lacks required identity, evidence or supported semantics.
    Indeterminate,
    /// A known hard constraint (such as cyclic precedence or rejected lift
    /// preflight) makes the proposed work set invalid.
    Rejected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MotionPhase {
    TravelToPickUnloaded,
    TravelToDropLoaded,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MotionSegment {
    pub job_id: String,
    pub phase: MotionPhase,
    pub cells: Vec<Cell>,
    pub distance_mm: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidatePlan {
    pub job_order: Vec<String>,
    pub motion_segments: Vec<MotionSegment>,
    pub total_distance_mm: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanReport {
    pub disposition: PlanDisposition,
    pub candidate: Option<CandidatePlan>,
    pub diagnostics: Vec<Diagnostic>,
    /// Number of recursive partial-sequence states visited by the exact search.
    pub search_nodes: u64,
}

fn diagnostic(
    diagnostics: &mut Vec<Diagnostic>,
    code: &'static str,
    severity: DiagnosticSeverity,
    subject_id: Option<&str>,
    detail: impl Into<String>,
) {
    diagnostics.push(Diagnostic {
        code,
        severity,
        subject_id: subject_id.map(str::to_owned),
        detail: detail.into(),
    });
}

fn nonblank(value: &str) -> bool {
    !value.trim().is_empty()
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn in_bounds(workspace: &Workspace, cell: Cell) -> bool {
    cell.x < workspace.width_cells && cell.y < workspace.height_cells
}

fn manhattan_steps(first: Cell, second: Cell) -> u64 {
    u64::from(first.x.abs_diff(second.x)) + u64::from(first.y.abs_diff(second.y))
}

fn neighbours(workspace: &Workspace, cell: Cell) -> [Option<Cell>; 4] {
    [
        cell.x.checked_sub(1).map(|x| Cell::new(x, cell.y)),
        cell.y.checked_sub(1).map(|y| Cell::new(cell.x, y)),
        cell.x
            .checked_add(1)
            .filter(|x| *x < workspace.width_cells)
            .map(|x| Cell::new(x, cell.y)),
        cell.y
            .checked_add(1)
            .filter(|y| *y < workspace.height_cells)
            .map(|y| Cell::new(cell.x, y)),
    ]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct QueueEntry {
    estimate_total: u64,
    cost_so_far: u64,
    cell: Cell,
}

impl Ord for QueueEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        // BinaryHeap is a max-heap; reverse each key to pop lowest f, then g,
        // then lexicographically smallest cell. This fixes tie behaviour.
        other
            .estimate_total
            .cmp(&self.estimate_total)
            .then_with(|| other.cost_so_far.cmp(&self.cost_so_far))
            .then_with(|| other.cell.cmp(&self.cell))
    }
}

impl PartialOrd for QueueEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn reconstruct_path(
    parents: &BTreeMap<Cell, Cell>,
    start: Cell,
    goal: Cell,
) -> Option<Vec<Cell>> {
    let mut path = vec![goal];
    let mut current = goal;
    while current != start {
        current = *parents.get(&current)?;
        path.push(current);
    }
    path.reverse();
    Some(path)
}

/// Deterministic A* on an explicitly admitted four-neighbour grid.
///
/// Omitted cells are not free. Every edge has the same positive integer cost,
/// and Manhattan distance is an admissible/consistent heuristic.
fn a_star_path(
    workspace: &Workspace,
    free_cells: &BTreeSet<Cell>,
    start: Cell,
    goal: Cell,
) -> Option<Vec<Cell>> {
    if !in_bounds(workspace, start)
        || !in_bounds(workspace, goal)
        || !free_cells.contains(&start)
        || !free_cells.contains(&goal)
    {
        return None;
    }
    if start == goal {
        return Some(vec![start]);
    }

    let mut open = BinaryHeap::new();
    let mut best_cost = BTreeMap::<Cell, u64>::new();
    let mut parents = BTreeMap::<Cell, Cell>::new();
    best_cost.insert(start, 0);
    open.push(QueueEntry {
        estimate_total: manhattan_steps(start, goal),
        cost_so_far: 0,
        cell: start,
    });

    while let Some(entry) = open.pop() {
        if best_cost.get(&entry.cell).copied() != Some(entry.cost_so_far) {
            continue;
        }
        if entry.cell == goal {
            return reconstruct_path(&parents, start, goal);
        }

        for next in neighbours(workspace, entry.cell).into_iter().flatten() {
            if !free_cells.contains(&next) {
                continue;
            }
            let Some(next_cost) = entry.cost_so_far.checked_add(1) else {
                continue;
            };
            let improve = best_cost
                .get(&next)
                .is_none_or(|known_cost| next_cost < *known_cost);
            if !improve {
                continue;
            }

            let Some(estimate_total) =
                next_cost.checked_add(manhattan_steps(next, goal))
            else {
                continue;
            };
            best_cost.insert(next, next_cost);
            parents.insert(next, entry.cell);
            open.push(QueueEntry {
                estimate_total,
                cost_so_far: next_cost,
                cell: next,
            });
        }
    }
    None
}

fn path_distance_mm(path: &[Cell], cell_size_mm: u32) -> Option<u64> {
    let steps = u64::try_from(path.len().checked_sub(1)?).ok()?;
    steps.checked_mul(u64::from(cell_size_mm))
}

fn validate_input(input: &PlannerInput) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let workspace = &input.workspace;

    for (name, value) in [
        ("crane_id", input.crane_id.as_str()),
        (
            "crane_configuration_revision",
            input.crane_configuration_revision.as_str(),
        ),
        ("site_epoch_id", workspace.site_epoch_id.as_str()),
        ("frame_id", workspace.frame_id.as_str()),
        ("clock_domain_id", workspace.clock_domain_id.as_str()),
        (
            "workspace_revision",
            workspace.workspace_revision.as_str(),
        ),
        (
            "clearance_profile_revision",
            workspace.clearance_profile_revision.as_str(),
        ),
    ] {
        if !nonblank(value) {
            diagnostic(
                &mut diagnostics,
                "CONTEXT_FIELD_EMPTY",
                DiagnosticSeverity::Indeterminate,
                None,
                format!("{name} must be nonblank"),
            );
        }
    }

    if workspace.width_cells == 0
        || workspace.height_cells == 0
        || workspace.width_cells > MAX_GRID_DIMENSION
        || workspace.height_cells > MAX_GRID_DIMENSION
        || workspace.cell_size_mm == 0
    {
        diagnostic(
            &mut diagnostics,
            "WORKSPACE_DIMENSIONS_INVALID",
            DiagnosticSeverity::Indeterminate,
            None,
            "workspace dimensions must be positive, bounded, and have a positive cell size",
        );
    }
    if !in_bounds(workspace, input.initial_hook_cell)
        || !workspace
            .unloaded_free_cells
            .contains(&input.initial_hook_cell)
    {
        diagnostic(
            &mut diagnostics,
            "INITIAL_HOOK_CELL_NOT_ADMITTED",
            DiagnosticSeverity::Indeterminate,
            None,
            "initial hook cell must lie inside the workspace and in the admitted unloaded-free set",
        );
    }

    for cell in workspace
        .unloaded_free_cells
        .iter()
        .chain(workspace.loaded_free_cells.iter())
    {
        if !in_bounds(workspace, *cell) {
            diagnostic(
                &mut diagnostics,
                "FREE_CELL_OUTSIDE_WORKSPACE",
                DiagnosticSeverity::Indeterminate,
                None,
                format!("free cell ({},{}) lies outside the declared grid", cell.x, cell.y),
            );
        }
    }
    if !workspace
        .loaded_free_cells
        .is_subset(&workspace.unloaded_free_cells)
    {
        diagnostic(
            &mut diagnostics,
            "LOADED_FREE_SET_NOT_SUBSET",
            DiagnosticSeverity::Indeterminate,
            None,
            "loaded-path free cells must also be admitted for the empty crane/trolley envelope",
        );
    }

    if input.jobs.is_empty() {
        diagnostic(
            &mut diagnostics,
            "HOIST_JOB_SET_EMPTY",
            DiagnosticSeverity::Indeterminate,
            None,
            "at least one block hoist job is required",
        );
    }
    if input.jobs.len() > MAX_JOBS {
        diagnostic(
            &mut diagnostics,
            "HOIST_JOB_LIMIT_EXCEEDED",
            DiagnosticSeverity::Indeterminate,
            None,
            format!(
                "V0.1 exact sequence enumeration is bounded to {MAX_JOBS} jobs"
            ),
        );
    }

    let mut jobs = BTreeMap::<String, &HoistJob>::new();
    let mut blocks = BTreeSet::<String>::new();
    for job in &input.jobs {
        if !nonblank(&job.job_id)
            || !nonblank(&job.block_id)
            || !nonblank(&job.block_revision)
        {
            diagnostic(
                &mut diagnostics,
                "JOB_IDENTITY_INCOMPLETE",
                DiagnosticSeverity::Indeterminate,
                Some(&job.job_id),
                "job, block and block revision identities must be nonblank",
            );
        }
        if jobs.insert(job.job_id.clone(), job).is_some() {
            diagnostic(
                &mut diagnostics,
                "DUPLICATE_JOB_ID",
                DiagnosticSeverity::Indeterminate,
                Some(&job.job_id),
                "hoist job IDs must be unique",
            );
        }
        if !blocks.insert(job.block_id.clone()) {
            diagnostic(
                &mut diagnostics,
                "DUPLICATE_BLOCK_LIFT",
                DiagnosticSeverity::Indeterminate,
                Some(&job.block_id),
                "V0.1 admits one lift job per physical block identity",
            );
        }

        let preflight = &job.preflight;
        match preflight.disposition {
            PreflightDisposition::ReadyForHumanReview => {}
            PreflightDisposition::Rejected => diagnostic(
                &mut diagnostics,
                "LIFT_PREFLIGHT_REJECTED",
                DiagnosticSeverity::Rejected,
                Some(&job.job_id),
                "the exact per-lift preflight is rejected; no route may be planned",
            ),
            PreflightDisposition::Indeterminate => diagnostic(
                &mut diagnostics,
                "LIFT_PREFLIGHT_INDETERMINATE",
                DiagnosticSeverity::Indeterminate,
                Some(&job.job_id),
                "the exact per-lift preflight is not sufficiently evidenced",
            ),
        }

        if preflight.block_id != job.block_id
            || preflight.block_revision != job.block_revision
            || preflight.crane_id != input.crane_id
            || preflight.crane_configuration_revision
                != input.crane_configuration_revision
            || preflight.site_epoch_id != workspace.site_epoch_id
            || preflight.frame_id != workspace.frame_id
            || preflight.clock_domain_id != workspace.clock_domain_id
        {
            diagnostic(
                &mut diagnostics,
                "LIFT_PREFLIGHT_CONTEXT_MISMATCH",
                DiagnosticSeverity::Indeterminate,
                Some(&job.job_id),
                "preflight must bind the exact block/revision, crane/configuration, site epoch, frame and clock domain",
            );
        }
        if !is_sha256(&preflight.input_sha256) || !is_sha256(&preflight.report_sha256) {
            diagnostic(
                &mut diagnostics,
                "LIFT_PREFLIGHT_DIGEST_INVALID",
                DiagnosticSeverity::Indeterminate,
                Some(&job.job_id),
                "preflight input and report references must be 64-character SHA-256 hex digests",
            );
        }

        for (label, cell) in [
            ("pick", job.pick_cell),
            ("drop", job.drop_cell),
        ] {
            if !in_bounds(workspace, cell)
                || !workspace.unloaded_free_cells.contains(&cell)
                || !workspace.loaded_free_cells.contains(&cell)
            {
                diagnostic(
                    &mut diagnostics,
                    "PICK_DROP_CELL_NOT_ADMITTED",
                    DiagnosticSeverity::Indeterminate,
                    Some(&job.job_id),
                    format!(
                        "{label} cell ({},{}) must be admitted for both empty and loaded envelopes",
                        cell.x, cell.y
                    ),
                );
            }
        }

        let mut predecessors = BTreeSet::new();
        for predecessor in &job.predecessor_job_ids {
            if !predecessors.insert(predecessor) {
                diagnostic(
                    &mut diagnostics,
                    "DUPLICATE_PREDECESSOR",
                    DiagnosticSeverity::Indeterminate,
                    Some(&job.job_id),
                    format!("predecessor {predecessor} is listed more than once"),
                );
            }
            if predecessor == &job.job_id {
                diagnostic(
                    &mut diagnostics,
                    "SELF_PRECEDENCE",
                    DiagnosticSeverity::Rejected,
                    Some(&job.job_id),
                    "a hoist job cannot precede itself",
                );
            } else if !input.jobs.iter().any(|item| item.job_id == *predecessor) {
                diagnostic(
                    &mut diagnostics,
                    "PREDECESSOR_NOT_FOUND",
                    DiagnosticSeverity::Indeterminate,
                    Some(&job.job_id),
                    format!("predecessor job {predecessor} is absent from this input"),
                );
            }
        }
    }

    if !input.jobs.is_empty() && input.jobs.len() <= MAX_JOBS {
        let mut indegree = BTreeMap::<String, usize>::new();
        let mut children = BTreeMap::<String, BTreeSet<String>>::new();
        for job in &input.jobs {
            indegree.insert(job.job_id.clone(), job.predecessor_job_ids.len());
            children.entry(job.job_id.clone()).or_default();
        }
        for job in &input.jobs {
            for predecessor in &job.predecessor_job_ids {
                if let Some(next) = children.get_mut(predecessor) {
                    next.insert(job.job_id.clone());
                }
            }
        }
        let mut ready: BTreeSet<String> = indegree
            .iter()
            .filter(|(_, degree)| **degree == 0)
            .map(|(id, _)| id.clone())
            .collect();
        let mut visited = 0usize;
        while let Some(id) = ready.iter().next().cloned() {
            ready.remove(&id);
            visited += 1;
            if let Some(next_jobs) = children.get(&id) {
                for next in next_jobs {
                    if let Some(degree) = indegree.get_mut(next) {
                        *degree = degree.saturating_sub(1);
                        if *degree == 0 {
                            ready.insert(next.clone());
                        }
                    }
                }
            }
        }
        if visited != input.jobs.len() {
            diagnostic(
                &mut diagnostics,
                "PRECEDENCE_CYCLE",
                DiagnosticSeverity::Rejected,
                None,
                "hoist-job precedence graph contains a cycle",
            );
        }
    }

    diagnostics.sort_by(|left, right| {
        left.code
            .cmp(right.code)
            .then_with(|| left.subject_id.cmp(&right.subject_id))
            .then_with(|| left.detail.cmp(&right.detail))
    });
    diagnostics
}

struct SearchState {
    best: Option<CandidatePlan>,
    nodes_visited: u64,
}

fn candidate_is_better(candidate: &CandidatePlan, current: &CandidatePlan) -> bool {
    candidate
        .total_distance_mm
        .cmp(&current.total_distance_mm)
        .then_with(|| candidate.job_order.cmp(&current.job_order))
        .is_lt()
}

fn search_sequences(
    input: &PlannerInput,
    jobs: &BTreeMap<String, HoistJob>,
    remaining: &mut BTreeSet<String>,
    completed: &BTreeSet<String>,
    current_cell: Cell,
    accumulated_distance_mm: u64,
    order: &mut Vec<String>,
    segments: &mut Vec<MotionSegment>,
    state: &mut SearchState,
) {
    state.nodes_visited = state.nodes_visited.saturating_add(1);
    if remaining.is_empty() {
        let candidate = CandidatePlan {
            job_order: order.clone(),
            motion_segments: segments.clone(),
            total_distance_mm: accumulated_distance_mm,
        };
        let replace = state
            .best
            .as_ref()
            .is_none_or(|best| candidate_is_better(&candidate, best));
        if replace {
            state.best = Some(candidate);
        }
        return;
    }

    let mut ready_jobs: Vec<String> = remaining
        .iter()
        .filter(|id| {
            jobs[*id]
                .predecessor_job_ids
                .iter()
                .all(|predecessor| completed.contains(predecessor))
        })
        .cloned()
        .collect();
    ready_jobs.sort();

    for job_id in ready_jobs {
        let job = &jobs[&job_id];
        let Some(to_pick) = a_star_path(
            &input.workspace,
            &input.workspace.unloaded_free_cells,
            current_cell,
            job.pick_cell,
        ) else {
            continue;
        };
        let Some(loaded_to_drop) = a_star_path(
            &input.workspace,
            &input.workspace.loaded_free_cells,
            job.pick_cell,
            job.drop_cell,
        ) else {
            continue;
        };

        let Some(empty_distance) =
            path_distance_mm(&to_pick, input.workspace.cell_size_mm)
        else {
            continue;
        };
        let Some(loaded_distance) =
            path_distance_mm(&loaded_to_drop, input.workspace.cell_size_mm)
        else {
            continue;
        };
        let Some(job_distance) = empty_distance.checked_add(loaded_distance) else {
            continue;
        };
        let Some(next_total) = accumulated_distance_mm.checked_add(job_distance) else {
            continue;
        };

        let segment_start = segments.len();
        segments.push(MotionSegment {
            job_id: job_id.clone(),
            phase: MotionPhase::TravelToPickUnloaded,
            cells: to_pick,
            distance_mm: empty_distance,
        });
        segments.push(MotionSegment {
            job_id: job_id.clone(),
            phase: MotionPhase::TravelToDropLoaded,
            cells: loaded_to_drop,
            distance_mm: loaded_distance,
        });

        remaining.remove(&job_id);
        let mut next_completed = completed.clone();
        next_completed.insert(job_id.clone());
        order.push(job_id.clone());

        search_sequences(
            input,
            jobs,
            remaining,
            &next_completed,
            job.drop_cell,
            next_total,
            order,
            segments,
            state,
        );

        order.pop();
        remaining.insert(job_id);
        segments.truncate(segment_start);
    }
}

/// Jointly enumerate feasible job orders and use deterministic A* for each
/// unloaded move to a pick point and loaded move to its drop point.
///
/// Exact sequence search is deliberately bounded to six jobs in V0.1.
/// The returned candidate is not a physical motion envelope or lift instruction.
pub fn plan(input: &PlannerInput) -> PlanReport {
    let diagnostics = validate_input(input);
    if !diagnostics.is_empty() {
        let disposition = if diagnostics
            .iter()
            .any(|item| item.severity == DiagnosticSeverity::Rejected)
        {
            PlanDisposition::Rejected
        } else {
            PlanDisposition::Indeterminate
        };
        return PlanReport {
            disposition,
            candidate: None,
            diagnostics,
            search_nodes: 0,
        };
    }

    let jobs: BTreeMap<String, HoistJob> = input
        .jobs
        .iter()
        .cloned()
        .map(|job| (job.job_id.clone(), job))
        .collect();
    let mut remaining: BTreeSet<String> = jobs.keys().cloned().collect();
    let completed = BTreeSet::new();
    let mut order = Vec::with_capacity(jobs.len());
    let mut segments = Vec::with_capacity(jobs.len() * 2);
    let mut state = SearchState {
        best: None,
        nodes_visited: 0,
    };

    search_sequences(
        input,
        &jobs,
        &mut remaining,
        &completed,
        input.initial_hook_cell,
        0,
        &mut order,
        &mut segments,
        &mut state,
    );

    match state.best {
        Some(candidate) => PlanReport {
            disposition: PlanDisposition::CandidateForHumanReview,
            candidate: Some(candidate),
            diagnostics: Vec::new(),
            search_nodes: state.nodes_visited,
        },
        None => PlanReport {
            disposition: PlanDisposition::InfeasibleWithinDeclaredWorkspace,
            candidate: None,
            diagnostics: vec![Diagnostic {
                code: "NO_FEASIBLE_SEQUENCE_WITHIN_DECLARED_WORKSPACE",
                severity: DiagnosticSeverity::Rejected,
                subject_id: None,
                detail: "no topological lift order has admitted unloaded and loaded paths inside the declared free-cell sets".to_owned(),
            }],
            search_nodes: state.nodes_visited,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cells(width: u16, height: u16) -> BTreeSet<Cell> {
        (0..width)
            .flat_map(|x| (0..height).map(move |y| Cell::new(x, y)))
            .collect()
    }

    fn workspace(width: u16, height: u16) -> Workspace {
        let all = cells(width, height);
        Workspace {
            site_epoch_id: "yard-epoch-01".to_owned(),
            frame_id: "yard-grid-mm".to_owned(),
            clock_domain_id: "yard-monotonic-ms".to_owned(),
            workspace_revision: "yard-layout-r1".to_owned(),
            clearance_profile_revision: "loaded-clearance-r1".to_owned(),
            width_cells: width,
            height_cells: height,
            cell_size_mm: 1_000,
            unloaded_free_cells: all.clone(),
            loaded_free_cells: all,
        }
    }

    fn preflight(block_id: &str, block_revision: &str) -> PreflightBinding {
        PreflightBinding {
            block_id: block_id.to_owned(),
            block_revision: block_revision.to_owned(),
            crane_id: "gantry-01".to_owned(),
            crane_configuration_revision: "gantry-config-r1".to_owned(),
            site_epoch_id: "yard-epoch-01".to_owned(),
            frame_id: "yard-grid-mm".to_owned(),
            clock_domain_id: "yard-monotonic-ms".to_owned(),
            disposition: PreflightDisposition::ReadyForHumanReview,
            input_sha256: "a".repeat(64),
            report_sha256: "b".repeat(64),
        }
    }

    fn job(
        id: &str,
        pick: Cell,
        drop: Cell,
        predecessors: &[&str],
    ) -> HoistJob {
        HoistJob {
            job_id: id.to_owned(),
            block_id: format!("block-{id}"),
            block_revision: format!("block-{id}-r1"),
            pick_cell: pick,
            drop_cell: drop,
            predecessor_job_ids: predecessors.iter().map(|id| (*id).to_owned()).collect(),
            preflight: preflight(&format!("block-{id}"), &format!("block-{id}-r1")),
        }
    }

    fn input() -> PlannerInput {
        PlannerInput {
            crane_id: "gantry-01".to_owned(),
            crane_configuration_revision: "gantry-config-r1".to_owned(),
            initial_hook_cell: Cell::new(0, 1),
            workspace: workspace(3, 3),
            jobs: vec![job(
                "A",
                Cell::new(0, 0),
                Cell::new(2, 0),
                &[],
            )],
        }
    }

    fn shortest_path_bfs(
        width: u16,
        height: u16,
        free: &BTreeSet<Cell>,
        start: Cell,
        goal: Cell,
    ) -> Option<Vec<Cell>> {
        if !free.contains(&start) || !free.contains(&goal) {
            return None;
        }
        let mut queue = VecDeque::new();
        let mut parent = BTreeMap::<Cell, Option<Cell>>::new();
        queue.push_back(start);
        parent.insert(start, None);

        while let Some(cell) = queue.pop_front() {
            if cell == goal {
                let mut path = vec![goal];
                let mut cursor = goal;
                while let Some(Some(previous)) = parent.get(&cursor).copied() {
                    cursor = previous;
                    path.push(cursor);
                }
                path.reverse();
                return Some(path);
            }
            let next_cells = [
                cell.x.checked_sub(1).map(|x| Cell::new(x, cell.y)),
                cell.y.checked_sub(1).map(|y| Cell::new(cell.x, y)),
                cell.x
                    .checked_add(1)
                    .filter(|x| *x < width)
                    .map(|x| Cell::new(x, cell.y)),
                cell.y
                    .checked_add(1)
                    .filter(|y| *y < height)
                    .map(|y| Cell::new(cell.x, y)),
            ];
            for next in next_cells.into_iter().flatten() {
                if free.contains(&next) && !parent.contains_key(&next) {
                    parent.insert(next, Some(cell));
                    queue.push_back(next);
                }
            }
        }
        None
    }

    fn reference_best_sequence(input: &PlannerInput) -> Option<(Vec<String>, u64)> {
        fn recurse(
            input: &PlannerInput,
            remaining: &mut BTreeSet<String>,
            completed: &BTreeSet<String>,
            current: Cell,
            distance: u64,
            order: &mut Vec<String>,
            best: &mut Option<(Vec<String>, u64)>,
        ) {
            if remaining.is_empty() {
                let candidate = (order.clone(), distance);
                if best.as_ref().is_none_or(|known| {
                    candidate
                        .1
                        .cmp(&known.1)
                        .then_with(|| candidate.0.cmp(&known.0))
                        .is_lt()
                }) {
                    *best = Some(candidate);
                }
                return;
            }
            let mut ready: Vec<&HoistJob> = input
                .jobs
                .iter()
                .filter(|job| {
                    remaining.contains(&job.job_id)
                        && job
                            .predecessor_job_ids
                            .iter()
                            .all(|predecessor| completed.contains(predecessor))
                })
                .collect();
            ready.sort_by(|left, right| left.job_id.cmp(&right.job_id));

            for job in ready {
                let Some(empty) = shortest_path_bfs(
                    input.workspace.width_cells,
                    input.workspace.height_cells,
                    &input.workspace.unloaded_free_cells,
                    current,
                    job.pick_cell,
                ) else {
                    continue;
                };
                let Some(loaded) = shortest_path_bfs(
                    input.workspace.width_cells,
                    input.workspace.height_cells,
                    &input.workspace.loaded_free_cells,
                    job.pick_cell,
                    job.drop_cell,
                ) else {
                    continue;
                };
                let step_count = (empty.len() - 1) + (loaded.len() - 1);
                let Some(job_distance) =
                    u64::try_from(step_count).ok().and_then(|steps| {
                        steps.checked_mul(u64::from(input.workspace.cell_size_mm))
                    })
                else {
                    continue;
                };
                let Some(next_distance) = distance.checked_add(job_distance) else {
                    continue;
                };
                remaining.remove(&job.job_id);
                let mut next_completed = completed.clone();
                next_completed.insert(job.job_id.clone());
                order.push(job.job_id.clone());
                recurse(
                    input,
                    remaining,
                    &next_completed,
                    job.drop_cell,
                    next_distance,
                    order,
                    best,
                );
                order.pop();
                remaining.insert(job.job_id.clone());
            }
        }

        let mut remaining: BTreeSet<String> =
            input.jobs.iter().map(|job| job.job_id.clone()).collect();
        let mut best = None;
        recurse(
            input,
            &mut remaining,
            &BTreeSet::new(),
            input.initial_hook_cell,
            0,
            &mut Vec::new(),
            &mut best,
        );
        best
    }

    #[test]
    fn known_shortest_detour_uses_admitted_cells_only() {
        let mut grid = workspace(3, 3);
        grid.unloaded_free_cells.remove(&Cell::new(1, 1));
        let path = a_star_path(
            &grid,
            &grid.unloaded_free_cells,
            Cell::new(0, 1),
            Cell::new(2, 1),
        )
        .expect("the route has a one-cell detour");
        assert_eq!(path.first(), Some(&Cell::new(0, 1)));
        assert_eq!(path.last(), Some(&Cell::new(2, 1)));
        assert_eq!(path.len() - 1, 4);
    }

    #[test]
    fn a_star_matches_independent_bfs_for_all_tiny_three_by_three_masks() {
        let all = cells(3, 3);
        let mut cases = 0usize;
        for start in all.iter().copied() {
            for goal in all.iter().copied() {
                if start == goal {
                    continue;
                }
                let variable: Vec<Cell> = all
                    .iter()
                    .copied()
                    .filter(|cell| *cell != start && *cell != goal)
                    .collect();
                for mask in 0_u16..(1_u16 << variable.len()) {
                    let mut free = all.clone();
                    for (bit, cell) in variable.iter().enumerate() {
                        if mask & (1_u16 << bit) != 0 {
                            free.remove(cell);
                        }
                    }
                    let grid = workspace(3, 3);
                    let expected = shortest_path_bfs(3, 3, &free, start, goal);
                    let actual = a_star_path(&grid, &free, start, goal);
                    assert_eq!(
                        actual.as_ref().map(Vec::len),
                        expected.as_ref().map(Vec::len),
                        "A*/BFS reachability or path-length mismatch: start={start:?}, goal={goal:?}, mask={mask}"
                    );
                    if let Some(path) = actual {
                        assert_eq!(path.first(), Some(&start));
                        assert_eq!(path.last(), Some(&goal));
                        assert!(path.iter().all(|cell| free.contains(cell)));
                        assert!(path.windows(2).all(|pair| manhattan_steps(pair[0], pair[1]) == 1));
                    }
                    cases += 1;
                }
            }
        }
        assert_eq!(cases, 9 * 8 * 128);
    }

    #[test]
    fn joint_sequence_and_paths_match_independent_bfs_permutation_oracle() {
        let mut test_input = PlannerInput {
            crane_id: "gantry-01".to_owned(),
            crane_configuration_revision: "gantry-config-r1".to_owned(),
            initial_hook_cell: Cell::new(1, 1),
            workspace: workspace(4, 4),
            jobs: vec![
                job("A", Cell::new(0, 0), Cell::new(3, 0), &[]),
                job("B", Cell::new(0, 3), Cell::new(3, 3), &[]),
                job("C", Cell::new(1, 2), Cell::new(2, 2), &[]),
            ],
        };
        test_input.workspace.loaded_free_cells.remove(&Cell::new(2, 1));
        let expected = reference_best_sequence(&test_input).expect("a plan exists");
        let report = plan(&test_input);
        let candidate = report.candidate.expect("candidate exists");
        assert_eq!(report.disposition, PlanDisposition::CandidateForHumanReview);
        assert_eq!(candidate.job_order, expected.0);
        assert_eq!(candidate.total_distance_mm, expected.1);
    }

    #[test]
    fn non_ready_preflight_is_indeterminate_or_rejected() {
        let mut test_input = input();
        test_input.jobs[0].preflight.disposition = PreflightDisposition::Indeterminate;
        let report = plan(&test_input);
        assert_eq!(report.disposition, PlanDisposition::Indeterminate);
        assert!(report
            .diagnostics
            .iter()
            .any(|item| item.code == "LIFT_PREFLIGHT_INDETERMINATE"));

        test_input.jobs[0].preflight.disposition = PreflightDisposition::Rejected;
        let rejected = plan(&test_input);
        assert_eq!(rejected.disposition, PlanDisposition::Rejected);
        assert!(rejected
            .diagnostics
            .iter()
            .any(|item| item.code == "LIFT_PREFLIGHT_REJECTED"));
    }

    #[test]
    fn preflight_must_match_exact_block_crane_site_frame_and_clock() {
        let mut test_input = input();
        test_input.jobs[0].preflight.crane_configuration_revision =
            "gantry-config-r2".to_owned();
        let report = plan(&test_input);
        assert_eq!(report.disposition, PlanDisposition::Indeterminate);
        assert!(report
            .diagnostics
            .iter()
            .any(|item| item.code == "LIFT_PREFLIGHT_CONTEXT_MISMATCH"));
    }

    #[test]
    fn unknown_cells_are_not_treated_as_free_route_space() {
        let mut test_input = input();
        test_input.workspace.width_cells = 3;
        test_input.workspace.height_cells = 1;
        test_input.initial_hook_cell = Cell::new(0, 0);
        test_input.workspace.unloaded_free_cells =
            [Cell::new(0, 0), Cell::new(2, 0)].into_iter().collect();
        test_input.workspace.loaded_free_cells =
            [Cell::new(0, 0), Cell::new(2, 0)].into_iter().collect();
        test_input.jobs[0].pick_cell = Cell::new(0, 0);
        test_input.jobs[0].drop_cell = Cell::new(2, 0);

        let report = plan(&test_input);
        assert_eq!(
            report.disposition,
            PlanDisposition::InfeasibleWithinDeclaredWorkspace
        );
        assert!(report
            .diagnostics
            .iter()
            .any(|item| item.code == "NO_FEASIBLE_SEQUENCE_WITHIN_DECLARED_WORKSPACE"));
    }

    #[test]
    fn precedence_is_respected_and_cycles_are_rejected() {
        let mut test_input = PlannerInput {
            crane_id: "gantry-01".to_owned(),
            crane_configuration_revision: "gantry-config-r1".to_owned(),
            initial_hook_cell: Cell::new(0, 0),
            workspace: workspace(3, 3),
            jobs: vec![
                job("A", Cell::new(0, 0), Cell::new(1, 0), &[]),
                job("B", Cell::new(1, 0), Cell::new(2, 0), &["A"]),
            ],
        };
        let report = plan(&test_input);
        let candidate = report.candidate.expect("precedence-feasible candidate");
        assert!(candidate.job_order.iter().position(|id| id == "A")
            < candidate.job_order.iter().position(|id| id == "B"));

        test_input.jobs[0].predecessor_job_ids = vec!["B".to_owned()];
        let cycle = plan(&test_input);
        assert_eq!(cycle.disposition, PlanDisposition::Rejected);
        assert!(cycle
            .diagnostics
            .iter()
            .any(|item| item.code == "PRECEDENCE_CYCLE"));
    }

    #[test]
    fn job_input_permutation_preserves_candidate_plan() {
        let mut first = PlannerInput {
            crane_id: "gantry-01".to_owned(),
            crane_configuration_revision: "gantry-config-r1".to_owned(),
            initial_hook_cell: Cell::new(1, 1),
            workspace: workspace(4, 4),
            jobs: vec![
                job("A", Cell::new(0, 0), Cell::new(3, 0), &[]),
                job("B", Cell::new(0, 3), Cell::new(3, 3), &[]),
                job("C", Cell::new(1, 2), Cell::new(2, 2), &[]),
            ],
        };
        let mut second = first.clone();
        second.jobs.reverse();
        assert_eq!(plan(&first), plan(&second));
        first.jobs.rotate_left(1);
        assert_eq!(plan(&first), plan(&second));
    }

    #[test]
    fn missing_or_duplicate_job_identity_is_indeterminate() {
        let mut test_input = input();
        test_input.jobs.push(test_input.jobs[0].clone());
        let report = plan(&test_input);
        assert_eq!(report.disposition, PlanDisposition::Indeterminate);
        assert!(report
            .diagnostics
            .iter()
            .any(|item| item.code == "DUPLICATE_JOB_ID"));
    }

    #[test]
    fn preflight_receipt_digest_shape_is_required() {
        let mut test_input = input();
        test_input.jobs[0].preflight.report_sha256 = "not-a-digest".to_owned();
        let report = plan(&test_input);
        assert_eq!(report.disposition, PlanDisposition::Indeterminate);
        assert!(report
            .diagnostics
            .iter()
            .any(|item| item.code == "LIFT_PREFLIGHT_DIGEST_INVALID"));
    }
}
