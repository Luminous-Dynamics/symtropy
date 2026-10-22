// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! VIS-MEM-011: deterministic transfer-under-mismatch navigation diagnostic.
//!
//! The task request exposes the task ID and final goal. The ordered waypoint
//! constraints are available only through stored demonstration memory. The
//! evaluator knows the required order but cannot feed it into the policy.
//!
//! This is a small synthetic mechanism test, not native MemTransfer results.
//!
//! rustc --edition=2024 --test tools/vision-memory-transfer-bench.rs -o /tmp/vis-mem-transfer-tests
//! /tmp/vis-mem-transfer-tests
//! rustc --edition=2024 -O tools/vision-memory-transfer-bench.rs -o /tmp/vis-mem-transfer-bench
//! /tmp/vis-mem-transfer-bench > vis-mem-transfer.tsv

use std::collections::VecDeque;

const NODE_COUNT: usize = 8;
const TASK_ID: u8 = 1;
const DISTRACTOR_TASK_ID: u8 = 9;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Edge(u8, u8);

impl Edge {
    fn new(a: u8, b: u8) -> Self {
        if a <= b { Self(a, b) } else { Self(b, a) }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct World {
    edges: Vec<Edge>,
    blocked: Vec<Edge>,
}

impl World {
    fn base() -> Self {
        let edges = [
            Edge::new(0, 1),
            Edge::new(1, 2),
            Edge::new(2, 3),
            Edge::new(3, 7),
            Edge::new(0, 4),
            Edge::new(4, 5),
            Edge::new(5, 3),
            Edge::new(5, 6),
            Edge::new(6, 7),
            Edge::new(1, 5),
        ];
        Self { edges: edges.into(), blocked: Vec::new() }
    }

    fn with_blocked(mut self, edges: &[Edge]) -> Self {
        self.blocked = edges.iter().copied().map(|e| Edge::new(e.0, e.1)).collect();
        self.blocked.sort();
        self.blocked.dedup();
        self
    }

    fn edge_open(&self, a: u8, b: u8) -> bool {
        let edge = Edge::new(a, b);
        self.edges.contains(&edge) && !self.blocked.contains(&edge)
    }

    fn neighbors(&self, node: u8) -> Vec<u8> {
        (0..NODE_COUNT as u8)
            .filter(|&other| other != node && self.edge_open(node, other))
            .collect()
    }

    fn shortest_path(&self, start: u8, goal: u8) -> Option<Vec<u8>> {
        self.shortest_path_avoiding(start, goal, &[])
    }

    fn shortest_path_avoiding(&self, start: u8, goal: u8, forbidden: &[u8]) -> Option<Vec<u8>> {
        if start as usize >= NODE_COUNT || goal as usize >= NODE_COUNT {
            return None;
        }
        if start == goal {
            return Some(vec![start]);
        }
        let mut seen = [false; NODE_COUNT];
        let mut parent: [Option<u8>; NODE_COUNT] = [None; NODE_COUNT];
        let mut queue = VecDeque::new();
        seen[start as usize] = true;
        queue.push_back(start);

        while let Some(node) = queue.pop_front() {
            for next in self.neighbors(node) {
                if forbidden.contains(&next) && next != goal { continue; }
                if seen[next as usize] { continue; }
                seen[next as usize] = true;
                parent[next as usize] = Some(node);
                if next == goal {
                    let mut path = vec![goal];
                    let mut cursor = goal;
                    while cursor != start {
                        cursor = parent[cursor as usize]?;
                        path.push(cursor);
                    }
                    path.reverse();
                    return Some(path);
                }
                queue.push_back(next);
            }
        }
        None
    }

    fn path_is_legal(&self, path: &[u8]) -> bool {
        path.iter().all(|&n| (n as usize) < NODE_COUNT)
            && path.windows(2).all(|w| self.edge_open(w[0], w[1]))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct TaskRequest {
    task_id: u8,
    goal: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Demonstration {
    task_id: u8,
    start: u8,
    waypoints: Vec<u8>,
    trajectory: Vec<u8>,
}

impl Demonstration {
    fn build(world: &World, task_id: u8, start: u8, waypoints: &[u8]) -> Option<Self> {
        let trajectory = plan_through(world, start, waypoints)?;
        Some(Self {
            task_id,
            start,
            waypoints: waypoints.to_vec(),
            trajectory,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Policy {
    NoMemory,
    ReplayFirst,
    NearestDemoReplay,
    ConstraintPlanner,
    GuardedHybrid,
}

impl Policy {
    const ALL: [Self; 5] = [
        Self::NoMemory,
        Self::ReplayFirst,
        Self::NearestDemoReplay,
        Self::ConstraintPlanner,
        Self::GuardedHybrid,
    ];

    fn name(self) -> &'static str {
        match self {
            Self::NoMemory => "no_memory",
            Self::ReplayFirst => "replay_first",
            Self::NearestDemoReplay => "nearest_demo_replay",
            Self::ConstraintPlanner => "constraint_planner",
            Self::GuardedHybrid => "guarded_hybrid",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Scenario {
    name: &'static str,
    start: u8,
    world: World,
    distractor_first: bool,
}

fn scenarios() -> Vec<Scenario> {
    let open = World::base();
    let edge_23 = [Edge::new(2, 3)];
    let edge_12 = [Edge::new(1, 2)];
    let edge_01 = [Edge::new(0, 1)];
    let isolate_3 = [Edge::new(2, 3), Edge::new(3, 5), Edge::new(3, 7)];
    vec![
        Scenario { name: "matched_open", start: 0, world: open.clone(), distractor_first: false },
        Scenario { name: "shifted_start", start: 4, world: open.clone(), distractor_first: false },
        Scenario { name: "matched_route_blocked", start: 0, world: open.clone().with_blocked(&edge_12), distractor_first: false },
        Scenario { name: "shifted_and_blocked", start: 4, world: open.clone().with_blocked(&edge_12), distractor_first: false },
        Scenario { name: "irrelevant_demo_first", start: 0, world: open.clone(), distractor_first: true },
        Scenario { name: "alternate_route", start: 0, world: open.clone().with_blocked(&edge_01), distractor_first: false },
        Scenario { name: "target_disconnected", start: 0, world: open.with_blocked(&isolate_3), distractor_first: false },
        Scenario { name: "far_start", start: 7, world: World::base(), distractor_first: false },
    ]
}

fn append_segment(path: &mut Vec<u8>, segment: &[u8]) {
    if path.is_empty() {
        path.extend_from_slice(segment);
    } else if !segment.is_empty() {
        path.extend_from_slice(&segment[1..]);
    }
}

fn plan_through(world: &World, start: u8, ordered_waypoints: &[u8]) -> Option<Vec<u8>> {
    let mut path = vec![start];
    let mut current = start;
    for (index, &target) in ordered_waypoints.iter().enumerate() {
        let future_waypoints = &ordered_waypoints[index + 1..];
        let segment = world.shortest_path_avoiding(current, target, future_waypoints)?;
        append_segment(&mut path, &segment);
        current = target;
    }
    Some(path)
}

fn has_ordered_waypoints(path: &[u8], required: &[u8]) -> bool {
    let mut next = 0;
    // The initial pose is not a waypoint visit. Once moving, entering a future
    // waypoint before its required turn invalidates the route; revisiting an
    // already completed waypoint is permitted.
    for &node in path.iter().skip(1) {
        if next < required.len() && node == required[next] {
            next += 1;
        } else if next < required.len() && required[next + 1..].contains(&node) {
            return false;
        }
    }
    next == required.len()
}

fn make_memories(scenario: &Scenario, required_waypoints: &[u8]) -> Vec<Demonstration> {
    let base = World::base();
    let relevant = Demonstration::build(&base, TASK_ID, 0, required_waypoints)
        .expect("canonical graph must support canonical demonstration");
    let distractor_waypoints = [2, 6, 7];
    let distractor = Demonstration::build(&base, DISTRACTOR_TASK_ID, 0, &distractor_waypoints)
        .expect("canonical graph must support distractor demonstration");
    if scenario.distractor_first {
        vec![distractor, relevant]
    } else {
        vec![relevant, distractor]
    }
}

#[derive(Clone, Debug)]
struct PolicyRun {
    policy: Policy,
    path: Vec<u8>,
    success: bool,
    constraint_fidelity: bool,
    illegal_edges: usize,
    replanned: bool,
    reason: &'static str,
}

fn execute_and_score(
    policy: Policy,
    request: &TaskRequest,
    scenario: &Scenario,
    memories: &[Demonstration],
    evaluator_waypoints: &[u8],
) -> PolicyRun {
    let selected = match policy {
        Policy::NoMemory => None,
        Policy::ReplayFirst => memories.first(),
        Policy::NearestDemoReplay => memories
            .iter()
            .filter(|m| m.task_id == request.task_id)
            .min_by_key(|m| scenario.world.shortest_path(scenario.start, m.start)
                .map(|p| p.len()).unwrap_or(usize::MAX)),
        Policy::ConstraintPlanner | Policy::GuardedHybrid => memories
            .iter()
            .find(|m| m.task_id == request.task_id),
    };

    let mut replanned = false;
    let planned: Option<Vec<u8>> = match policy {
        Policy::NoMemory => scenario.world.shortest_path(scenario.start, request.goal),
        Policy::ReplayFirst => selected.and_then(|m| {
            if m.trajectory.first().copied() != Some(scenario.start) {
                None
            } else if scenario.world.path_is_legal(&m.trajectory) {
                Some(m.trajectory.clone())
            } else {
                None
            }
        }),
        Policy::NearestDemoReplay => selected.and_then(|m| {
            let connector = scenario.world.shortest_path(scenario.start, m.start)?;
            if !scenario.world.path_is_legal(&m.trajectory) { return None; }
            let mut full = connector;
            append_segment(&mut full, &m.trajectory);
            Some(full)
        }),
        Policy::ConstraintPlanner => {
            replanned = true;
            selected.and_then(|m| {
                let mut required = m.waypoints.clone();
                if required.last().copied() != Some(request.goal) {
                    required.push(request.goal);
                }
                plan_through(&scenario.world, scenario.start, &required)
            })
        }
        Policy::GuardedHybrid => {
            let valid_match = selected.is_some_and(|m| {
                m.start == scenario.start
                    && scenario.world.path_is_legal(&m.trajectory)
                    && has_ordered_waypoints(&m.trajectory, &m.waypoints)
            });
            if valid_match {
                selected.map(|m| m.trajectory.clone())
            } else {
                replanned = true;
                selected.and_then(|m| {
                    let mut required = m.waypoints.clone();
                    if required.last().copied() != Some(request.goal) {
                        required.push(request.goal);
                    }
                    plan_through(&scenario.world, scenario.start, &required)
                })
            }
        }
    };

    let path = planned.unwrap_or_else(Vec::new);
    let legal = scenario.world.path_is_legal(&path);
    let constraint_fidelity = !path.is_empty()
        && has_ordered_waypoints(&path, evaluator_waypoints)
        && path.last().copied() == Some(request.goal);
    let success = legal
        && constraint_fidelity
        && path.first().copied() == Some(scenario.start);
    let illegal_edges = path.windows(2)
        .filter(|w| !scenario.world.edge_open(w[0], w[1]))
        .count();

    let reason = if success {
        "completed"
    } else if path.is_empty() {
        "no_path_or_no_applicable_memory"
    } else if !legal {
        "illegal_edge"
    } else if !constraint_fidelity {
        "constraint_or_order_failure"
    } else {
        "wrong_start_or_goal"
    };

    PolicyRun { policy, path, success, constraint_fidelity, illegal_edges, replanned, reason }
}

fn format_report() -> String {
    let required_waypoints = vec![3, 5, 7];
    let request = TaskRequest { task_id: TASK_ID, goal: 7 };
    let mut output = String::new();
    output.push_str("META\tformat\tVIS-MEM-011-v0.1\n");
    output.push_str("META\tscope\tsynthetic transfer/replanning mechanism test, not native robotics success\n");
    output.push_str("META\tpolicy_request\ttask_id and final goal only\n");
    output.push_str("META\tevaluator_only\tordered waypoint truth\n");
    output.push_str("HEADER\tscenario\tpolicy\tstart\tblocked_edges\tsuccess\tconstraint_fidelity\tillegal_edges\treplanned\tpath\treason\n");

    for scenario in scenarios() {
        let memories = make_memories(&scenario, &required_waypoints);
        for policy in Policy::ALL {
            let result = execute_and_score(
                policy,
                &request,
                &scenario,
                &memories,
                &required_waypoints,
            );
            let path = result.path.iter().map(u8::to_string).collect::<Vec<_>>().join(",");
            let blocked = scenario.world.blocked.iter()
                .map(|e| format!("{}-{}", e.0, e.1))
                .collect::<Vec<_>>().join(",");
            output.push_str(&format!(
                "METRIC\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
                scenario.name,
                result.policy.name(),
                scenario.start,
                if blocked.is_empty() { "none" } else { &blocked },
                u8::from(result.success),
                u8::from(result.constraint_fidelity),
                result.illegal_edges,
                u8::from(result.replanned),
                if path.is_empty() { "none".to_owned() } else { path },
                result.reason
            ));
        }
    }
    output
}

fn main() {
    print!("{}", format_report());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> (TaskRequest, Vec<u8>) {
        (TaskRequest { task_id: TASK_ID, goal: 7 }, vec![3, 5, 7])
    }

    #[test]
    fn bfs_matches_known_short_path() {
        let path = World::base().shortest_path(0, 1).unwrap();
        assert_eq!(path, vec![0, 1]);
        assert_eq!(World::base().shortest_path(0, 0).unwrap(), vec![0]);
    }

    #[test]
    fn planner_visits_waypoints_in_order() {
        let (request, waypoints) = setup();
        let s = scenarios().into_iter().find(|s| s.name == "shifted_start").unwrap();
        let memories = make_memories(&s, &waypoints);
        let r = execute_and_score(Policy::ConstraintPlanner, &request, &s, &memories, &waypoints);
        assert!(r.success, "{:?}", r);
        assert!(has_ordered_waypoints(&r.path, &waypoints));
    }

    #[test]
    fn fixed_replay_succeeds_on_matched_open_demo() {
        let (request, waypoints) = setup();
        let s = scenarios().into_iter().find(|s| s.name == "matched_open").unwrap();
        let memories = make_memories(&s, &waypoints);
        let r = execute_and_score(Policy::ReplayFirst, &request, &s, &memories, &waypoints);
        assert!(r.success, "{:?}", r);
    }

    #[test]
    fn fixed_replay_fails_on_shifted_start() {
        let (request, waypoints) = setup();
        let s = scenarios().into_iter().find(|s| s.name == "shifted_start").unwrap();
        let memories = make_memories(&s, &waypoints);
        let r = execute_and_score(Policy::ReplayFirst, &request, &s, &memories, &waypoints);
        assert!(!r.success);
        assert_eq!(r.reason, "no_path_or_no_applicable_memory");
    }

    #[test]
    fn guarded_hybrid_replans_after_start_shift() {
        let (request, waypoints) = setup();
        let s = scenarios().into_iter().find(|s| s.name == "shifted_start").unwrap();
        let memories = make_memories(&s, &waypoints);
        let r = execute_and_score(Policy::GuardedHybrid, &request, &s, &memories, &waypoints);
        assert!(r.success, "{:?}", r);
        assert!(r.replanned);
    }

    #[test]
    fn guarded_hybrid_replans_around_blocked_demo_edge() {
        let (request, waypoints) = setup();
        let s = scenarios().into_iter().find(|s| s.name == "matched_route_blocked").unwrap();
        let memories = make_memories(&s, &waypoints);
        let r = execute_and_score(Policy::GuardedHybrid, &request, &s, &memories, &waypoints);
        assert!(r.success, "{:?}", r);
        assert!(r.replanned);
    }

    #[test]
    fn query_matching_ignores_irrelevant_demo_inserted_first() {
        let (request, waypoints) = setup();
        let s = scenarios().into_iter().find(|s| s.name == "irrelevant_demo_first").unwrap();
        let memories = make_memories(&s, &waypoints);
        let bad = execute_and_score(Policy::ReplayFirst, &request, &s, &memories, &waypoints);
        let good = execute_and_score(Policy::ConstraintPlanner, &request, &s, &memories, &waypoints);
        assert!(!bad.success);
        assert!(good.success);
    }

    #[test]
    fn impossible_target_is_not_claimed_as_success() {
        let (request, waypoints) = setup();
        let s = scenarios().into_iter().find(|s| s.name == "target_disconnected").unwrap();
        let memories = make_memories(&s, &waypoints);
        let r = execute_and_score(Policy::ConstraintPlanner, &request, &s, &memories, &waypoints);
        assert!(!r.success);
        assert_eq!(r.reason, "no_path_or_no_applicable_memory");
    }

    #[test]
    fn blocked_route_is_replanned_if_alternative_exists() {
        let (request, waypoints) = setup();
        let s = scenarios().into_iter().find(|s| s.name == "alternate_route").unwrap();
        let memories = make_memories(&s, &waypoints);
        let r = execute_and_score(Policy::ConstraintPlanner, &request, &s, &memories, &waypoints);
        assert!(r.success, "{:?}", r);
        assert!(r.path.windows(2).all(|w| s.world.edge_open(w[0], w[1])));
    }


    #[test]
    fn planner_never_visits_future_waypoint_early() {
        let (request, waypoints) = setup();
        let s = scenarios().into_iter().find(|s| s.name == "shifted_start").unwrap();
        let memories = make_memories(&s, &waypoints);
        let r = execute_and_score(Policy::ConstraintPlanner, &request, &s, &memories, &waypoints);
        assert!(r.success, "{:?}", r);
        assert!(has_ordered_waypoints(&r.path, &waypoints));
        let first_three = r.path.iter().skip(1).position(|&n| n == 3).unwrap();
        let first_five = r.path.iter().skip(1).position(|&n| n == 5).unwrap();
        assert!(first_three < first_five, "path revisits future waypoint too early: {:?}", r.path);
    }

    #[test]
    fn evaluator_rejects_future_waypoint_visited_early() {
        assert!(!has_ordered_waypoints(&[4, 5, 3, 5, 7], &[3, 5, 7]));
        assert!(has_ordered_waypoints(&[4, 0, 1, 2, 3, 5, 3, 7], &[3, 5, 7]));
    }

    #[test]
    fn task_macro_outputs_are_deterministic() {
        let first = format_report();
        let second = format_report();
        assert_eq!(first, second);
    }
}
