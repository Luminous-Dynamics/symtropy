// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Exact finite-state motion-bounded observability policy oracle.
//! Inputs are upstream-provided transition and observation relations.
//! This is not a sensor estimator, physical model, controller, or permit issuer.

use std::collections::{BTreeMap, BTreeSet};
use std::{env, fs, process::ExitCode};

const HEADER: [&str; 25] = [
    "case_id",
    "evidence_class",
    "initial_states",
    "frame_sha256",
    "calibration_sha256",
    "energy_budget_uj",
    "latency_budget_us",
    "max_actions",
    "max_ripple_ppm",
    "min_zvs_margin_permille",
    "max_service_drop_ppm",
    "action_id",
    "action_kind",
    "energy_cost_uj",
    "duration_us",
    "ripple_ppm",
    "zvs_margin_permille",
    "service_drop_ppm",
    "from_state",
    "from_pose_id",
    "to_state",
    "to_pose_id",
    "outcomes",
    "transition_in_domain",
    "valid_until_us",
];

const MAX_STATES: usize = 32;
const MAX_ACTIONS: usize = 16;
const MAX_DEPTH: usize = 12;

#[derive(Clone, Debug, PartialEq, Eq)]
struct Case {
    case_id: String,
    evidence_class: String,
    initial_states: BTreeSet<String>,
    frame_sha256: String,
    calibration_sha256: String,
    energy_budget_uj: u64,
    latency_budget_us: u64,
    max_actions: usize,
    max_ripple_ppm: u64,
    min_zvs_margin_permille: u64,
    max_service_drop_ppm: u64,
    state_poses: BTreeMap<String, String>,
    actions: BTreeMap<String, Action>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Target {
    to_state: String,
    to_pose_id: String,
    outcomes: BTreeSet<String>,
    in_domain: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Action {
    id: String,
    kind: String,
    energy_cost_uj: u64,
    duration_us: u64,
    ripple_ppm: u64,
    zvs_margin_permille: u64,
    service_drop_ppm: u64,
    valid_until_us: u64,
    transitions: BTreeMap<String, Vec<Target>>,
}

impl Action {
    fn same_metadata(&self, other: &Self) -> bool {
        self.id == other.id
            && self.kind == other.kind
            && self.energy_cost_uj == other.energy_cost_uj
            && self.duration_us == other.duration_us
            && self.ripple_ppm == other.ripple_ppm
            && self.zvs_margin_permille == other.zvs_margin_permille
            && self.service_drop_ppm == other.service_drop_ppm
            && self.valid_until_us == other.valid_until_us
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Plan {
    action_id: Option<String>,
    terminal_pose: Option<String>,
    branches: BTreeMap<String, Plan>,
    worst_case_energy_uj: u64,
    worst_case_latency_us: u64,
    worst_case_actions: usize,
}

impl Plan {
    fn terminal(pose: String) -> Self {
        Self {
            action_id: None,
            terminal_pose: Some(pose),
            branches: BTreeMap::new(),
            worst_case_energy_uj: 0,
            worst_case_latency_us: 0,
            worst_case_actions: 0,
        }
    }
}

fn parse_u64(value: &str, field: &str) -> Result<u64, String> {
    if value.is_empty() || value.trim() != value {
        return Err(format!("{field}: empty or untrimmed integer"));
    }
    value
        .parse::<u64>()
        .map_err(|_| format!("{field}: expected non-negative integer"))
}

fn digest_valid(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn parse_ids(value: &str, field: &str) -> Result<BTreeSet<String>, String> {
    let values: Vec<&str> = value.split(';').collect();
    if values.is_empty()
        || values
            .iter()
            .any(|part| part.is_empty() || part.trim() != *part)
    {
        return Err(format!("{field}: empty or untrimmed set member"));
    }
    let result: BTreeSet<String> = values.iter().map(|part| (*part).to_string()).collect();
    if result.len() != values.len() {
        return Err(format!("{field}: duplicate set member"));
    }
    Ok(result)
}

fn insert_state_pose(
    state_poses: &mut BTreeMap<String, String>,
    state: &str,
    pose: &str,
    line_no: usize,
) -> Result<(), String> {
    if state.is_empty()
        || pose.is_empty()
        || state.trim() != state
        || pose.trim() != pose
    {
        return Err(format!("line {line_no}: state and pose IDs must be non-empty and trimmed"));
    }
    if let Some(existing) = state_poses.get(state) {
        if existing != pose {
            return Err(format!(
                "line {line_no}: state {state} maps to conflicting pose IDs"
            ));
        }
    } else {
        state_poses.insert(state.to_string(), pose.to_string());
    }
    Ok(())
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
    let mut parsed_case: Option<Case> = None;
    let mut errors = Vec::new();

    for (offset, raw_line) in lines.enumerate() {
        let line_no = offset + 2;
        let line = raw_line.strip_suffix('\r').unwrap_or(raw_line);
        if line.is_empty() {
            errors.push(format!("line {line_no}: blank records are not allowed"));
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

        let current_metadata: Vec<String> =
            fields[..11].iter().map(|value| (*value).to_string()).collect();
        if let Some(first) = &metadata {
            if *first != current_metadata {
                errors.push(format!(
                    "line {line_no}: case metadata differs from the first row"
                ));
                continue;
            }
        } else {
            metadata = Some(current_metadata);
            let initial_states = match parse_ids(fields[2], "initial_states") {
                Ok(ids) => ids,
                Err(error) => {
                    errors.push(error);
                    continue;
                }
            };
            if initial_states.is_empty() || initial_states.len() > MAX_STATES {
                errors.push(format!(
                    "line {line_no}: initial_states must contain 1..={MAX_STATES} IDs"
                ));
                continue;
            }
            if fields[0].is_empty()
                || fields[0].trim() != fields[0]
                || fields[1] != "SYNTHETIC_KNOWN_TRUTH"
                || !digest_valid(fields[3])
                || !digest_valid(fields[4])
            {
                errors.push(format!(
                    "line {line_no}: invalid case identity, evidence class, frame digest or calibration digest"
                ));
                continue;
            }

            let numbers = (5..=10)
                .map(|index| {
                    parse_u64(fields[index], &format!("line {line_no} column {}", index + 1))
                })
                .collect::<Result<Vec<_>, _>>();
            let numbers = match numbers {
                Ok(values) => values,
                Err(error) => {
                    errors.push(error);
                    continue;
                }
            };
            if numbers[0] == 0
                || numbers[1] == 0
                || numbers[2] == 0
                || numbers[2] > MAX_DEPTH as u64
                || numbers[3] > 1_000_000
                || numbers[4] > 1_000
                || numbers[5] > 1_000_000
            {
                errors.push(format!("line {line_no}: case budget or limit is outside v0 bounds"));
                continue;
            }

            parsed_case = Some(Case {
                case_id: fields[0].to_string(),
                evidence_class: fields[1].to_string(),
                initial_states,
                frame_sha256: fields[3].to_string(),
                calibration_sha256: fields[4].to_string(),
                energy_budget_uj: numbers[0],
                latency_budget_us: numbers[1],
                max_actions: numbers[2] as usize,
                max_ripple_ppm: numbers[3],
                min_zvs_margin_permille: numbers[4],
                max_service_drop_ppm: numbers[5],
                state_poses: BTreeMap::new(),
                actions: BTreeMap::new(),
            });
        }

        let Some(case) = parsed_case.as_mut() else {
            errors.push(format!("line {line_no}: case metadata could not be initialized"));
            continue;
        };

        let action_id = fields[11];
        if action_id.is_empty() || action_id.trim() != action_id {
            errors.push(format!("line {line_no}: action_id must be non-empty and trimmed"));
            continue;
        }
        let kind = fields[12];
        if !["PASSIVE_OBSERVE", "ACTIVE_EXCITATION"].contains(&kind) {
            errors.push(format!("line {line_no}: unsupported action_kind {kind:?}"));
            continue;
        }

        let action_numbers = (13..=17)
            .map(|index| {
                parse_u64(fields[index], &format!("line {line_no} column {}", index + 1))
            })
            .collect::<Result<Vec<_>, _>>();
        let action_numbers = match action_numbers {
            Ok(values) => values,
            Err(error) => {
                errors.push(error);
                continue;
            }
        };
        let valid_until_us = match parse_u64(fields[24], "valid_until_us") {
            Ok(value) => value,
            Err(error) => {
                errors.push(error);
                continue;
            }
        };

        if action_numbers[1] == 0
            || (kind == "ACTIVE_EXCITATION" && action_numbers[0] == 0)
            || action_numbers[2] > 1_000_000
            || action_numbers[3] > 1_000
            || action_numbers[4] > 1_000_000
        {
            errors.push(format!(
                "line {line_no}: duration must be positive; active excitation energy must be positive; electrical metrics must be within v0 bounds"
            ));
            continue;
        }

        let mut candidate = Action {
            id: action_id.to_string(),
            kind: kind.to_string(),
            energy_cost_uj: action_numbers[0],
            duration_us: action_numbers[1],
            ripple_ppm: action_numbers[2],
            zvs_margin_permille: action_numbers[3],
            service_drop_ppm: action_numbers[4],
            valid_until_us,
            transitions: BTreeMap::new(),
        };

        if let Some(existing) = case.actions.get_mut(action_id) {
            if !existing.same_metadata(&candidate) {
                errors.push(format!(
                    "line {line_no}: action {action_id} has inconsistent metadata"
                ));
                continue;
            }
            candidate.transitions = std::mem::take(&mut existing.transitions);
            *existing = candidate;
        } else {
            case.actions.insert(action_id.to_string(), candidate);
        }

        let from_state = fields[18];
        let from_pose = fields[19];
        let to_state = fields[20];
        let to_pose = fields[21];
        if let Err(error) = insert_state_pose(&mut case.state_poses, from_state, from_pose, line_no)
        {
            errors.push(error);
            continue;
        }
        if let Err(error) = insert_state_pose(&mut case.state_poses, to_state, to_pose, line_no) {
            errors.push(error);
            continue;
        }

        let outcomes = match parse_ids(fields[22], "outcomes") {
            Ok(ids) => ids,
            Err(error) => {
                errors.push(format!("line {line_no}: {error}"));
                continue;
            }
        };
        let in_domain = match fields[23] {
            "true" => true,
            "false" => false,
            other => {
                errors.push(format!(
                    "line {line_no}: transition_in_domain must be true or false, found {other:?}"
                ));
                continue;
            }
        };

        let target = Target {
            to_state: to_state.to_string(),
            to_pose_id: to_pose.to_string(),
            outcomes,
            in_domain,
        };
        let action = case.actions.get_mut(action_id).expect("action was inserted");
        let edges = action
            .transitions
            .entry(from_state.to_string())
            .or_default();
        if edges.iter().any(|edge| edge.to_state == target.to_state) {
            errors.push(format!(
                "line {line_no}: duplicate transition edge {action_id}:{from_state}->{to_state}"
            ));
            continue;
        }
        edges.push(target);
        edges.sort_by(|left, right| left.to_state.cmp(&right.to_state));
    }

    let Some(case) = parsed_case else {
        return Err(if errors.is_empty() {
            vec!["no case rows found".to_string()]
        } else {
            errors
        });
    };
    if case.actions.is_empty() {
        errors.push("case has no actions".to_string());
    }
    if case.actions.len() > MAX_ACTIONS {
        errors.push(format!("case exceeds {MAX_ACTIONS} action limit"));
    }
    if case.initial_states.iter().any(|state| !case.state_poses.contains_key(state)) {
        errors.push("one or more initial states have no declared pose mapping".to_string());
    }
    for (action_id, action) in &case.actions {
        if action.transitions.is_empty() {
            errors.push(format!("action {action_id} has no transitions"));
        }
        for (from_state, edges) in &action.transitions {
            if !case.state_poses.contains_key(from_state) || edges.is_empty() {
                errors.push(format!(
                    "action {action_id} has an unknown source state or empty transition relation"
                ));
            }
            for edge in edges {
                if !case.state_poses.contains_key(&edge.to_state) {
                    errors.push(format!(
                        "action {action_id} references unknown target state {}",
                        edge.to_state
                    ));
                }
            }
        }
    }

    if errors.is_empty() {
        Ok(case)
    } else {
        Err(errors)
    }
}

fn unique_pose(case: &Case, belief: &BTreeSet<String>) -> Option<String> {
    let mut poses = belief
        .iter()
        .filter_map(|state| case.state_poses.get(state));
    let first = poses.next()?.clone();
    if poses.all(|pose| pose == &first) {
        Some(first)
    } else {
        None
    }
}

fn plan_key(plan: &Plan) -> String {
    if let Some(pose) = &plan.terminal_pose {
        return format!("T:{pose}");
    }
    let action = plan.action_id.as_deref().unwrap_or("?");
    let branches = plan
        .branches
        .iter()
        .map(|(outcome, child)| format!("{outcome}:{}", plan_key(child)))
        .collect::<Vec<_>>()
        .join("|");
    format!("A:{action}[{branches}]")
}

fn score(plan: &Plan) -> (u64, u64, usize, String) {
    (
        plan.worst_case_energy_uj,
        plan.worst_case_latency_us,
        plan.worst_case_actions,
        plan_key(plan),
    )
}

fn dominates(left: &Plan, right: &Plan) -> bool {
    let no_worse = left.worst_case_energy_uj <= right.worst_case_energy_uj
        && left.worst_case_latency_us <= right.worst_case_latency_us
        && left.worst_case_actions <= right.worst_case_actions;
    if !no_worse {
        return false;
    }
    let strictly_better = left.worst_case_energy_uj < right.worst_case_energy_uj
        || left.worst_case_latency_us < right.worst_case_latency_us
        || left.worst_case_actions < right.worst_case_actions;
    strictly_better || plan_key(left) <= plan_key(right)
}

fn pareto_prune(mut plans: Vec<Plan>) -> Vec<Plan> {
    plans.sort_by_key(score);
    let mut frontier: Vec<Plan> = Vec::new();
    for candidate in plans {
        if frontier.iter().any(|existing| dominates(existing, &candidate)) {
            continue;
        }
        frontier.retain(|existing| !dominates(&candidate, existing));
        frontier.push(candidate);
    }
    frontier.sort_by_key(score);
    frontier
}

fn frontier(
    case: &Case,
    belief: &BTreeSet<String>,
    elapsed_us: u64,
    evidence_deadline_us: u64,
    remaining_actions: usize,
    memo: &mut BTreeMap<(Vec<String>, u64, u64, usize), Vec<Plan>>,
) -> Vec<Plan> {
    if let Some(pose) = unique_pose(case, belief) {
        return if elapsed_us <= evidence_deadline_us {
            vec![Plan::terminal(pose)]
        } else {
            Vec::new()
        };
    }
    if remaining_actions == 0 {
        return Vec::new();
    }

    let key = (
        belief.iter().cloned().collect::<Vec<_>>(),
        elapsed_us,
        evidence_deadline_us,
        remaining_actions,
    );
    if let Some(cached) = memo.get(&key) {
        return cached.clone();
    }

    let mut candidates = Vec::new();

    for action in case.actions.values() {
        if action.ripple_ppm > case.max_ripple_ppm
            || action.zvs_margin_permille < case.min_zvs_margin_permille
            || action.service_drop_ppm > case.max_service_drop_ppm
        {
            continue;
        }
        let Some(end_time_us) = elapsed_us.checked_add(action.duration_us) else {
            continue;
        };
        if end_time_us > action.valid_until_us {
            continue;
        }

        let mut next_by_outcome: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        let mut action_valid = true;

        for state in belief {
            let Some(edges) = action.transitions.get(state) else {
                action_valid = false;
                break;
            };
            if edges.is_empty() {
                action_valid = false;
                break;
            }
            for edge in edges {
                if !edge.in_domain {
                    action_valid = false;
                    break;
                }
                for outcome in &edge.outcomes {
                    next_by_outcome
                        .entry(outcome.clone())
                        .or_default()
                        .insert(edge.to_state.clone());
                }
            }
            if !action_valid {
                break;
            }
        }
        if !action_valid || next_by_outcome.is_empty() {
            continue;
        }

        // Every possible observation outcome must have a complete continuation.
        let mut branch_combinations: Vec<BTreeMap<String, Plan>> =
            vec![BTreeMap::new()];
        for (outcome, next_belief) in next_by_outcome {
            let child_plans = frontier(
                case,
                &next_belief,
                end_time_us,
                evidence_deadline_us.min(action.valid_until_us),
                remaining_actions - 1,
                memo,
            );
            if child_plans.is_empty() {
                action_valid = false;
                break;
            }
            let mut expanded = Vec::new();
            for combination in &branch_combinations {
                for child in &child_plans {
                    let mut next = combination.clone();
                    next.insert(outcome.clone(), child.clone());
                    expanded.push(next);
                }
            }
            branch_combinations = expanded;
        }
        if !action_valid {
            continue;
        }

        for branches in branch_combinations {
            let child_energy = branches
                .values()
                .map(|child| child.worst_case_energy_uj)
                .max()
                .unwrap_or(0);
            let child_latency = branches
                .values()
                .map(|child| child.worst_case_latency_us)
                .max()
                .unwrap_or(0);
            let child_actions = branches
                .values()
                .map(|child| child.worst_case_actions)
                .max()
                .unwrap_or(0);
            let Some(energy) = action.energy_cost_uj.checked_add(child_energy) else {
                continue;
            };
            let Some(latency) = action.duration_us.checked_add(child_latency) else {
                continue;
            };
            let Some(actions) = 1usize.checked_add(child_actions) else {
                continue;
            };
            if energy > case.energy_budget_uj
                || latency > case.latency_budget_us
                || actions > case.max_actions
            {
                continue;
            }

            candidates.push(Plan {
                action_id: Some(action.id.clone()),
                terminal_pose: None,
                branches,
                worst_case_energy_uj: energy,
                worst_case_latency_us: latency,
                worst_case_actions: actions,
            });
        }
    }

    let result = pareto_prune(candidates);
    memo.insert(key, result.clone());
    result
}

fn solve(case: &Case) -> Option<Plan> {
    let mut memo = BTreeMap::new();
    let mut plans = frontier(
        case,
        &case.initial_states,
        0,
        u64::MAX,
        case.max_actions,
        &mut memo,
    );
    plans.sort_by_key(score);
    plans.into_iter().next()
}

fn print_plan(plan: &Plan, indent: usize) {
    let pad = " ".repeat(indent);
    if let Some(pose) = &plan.terminal_pose {
        println!("{pad}UNIQUE_CURRENT_POSE {pose}");
        return;
    }
    let action = plan.action_id.as_deref().unwrap_or("UNKNOWN");
    println!("{pad}ACTION {action}");
    for (outcome, child) in &plan.branches {
        println!("{pad}  OUTCOME {outcome}");
        print_plan(child, indent + 4);
    }
}

fn run(path: &str) -> Result<(), String> {
    let input = fs::read_to_string(path)
        .map_err(|error| format!("cannot read {path}: {error}"))?;
    let case = parse_case(&input).map_err(|errors| errors.join("\n"))?;
    println!("case_id={}", case.case_id);
    println!("evidence_class={}", case.evidence_class);
    println!("frame_sha256={}", case.frame_sha256);
    println!("calibration_sha256={}", case.calibration_sha256);

    if let Some(plan) = solve(&case) {
        println!("disposition=SYNTHETIC_POLICY_FOUND_NO_POWER_OR_FLIGHT_AUTHORITY");
        println!("worst_case_energy_uj={}", plan.worst_case_energy_uj);
        println!("worst_case_latency_us={}", plan.worst_case_latency_us);
        println!("worst_case_actions={}", plan.worst_case_actions);
        print_plan(&plan, 0);
    } else {
        println!("disposition=ABSTAIN_NO_COMPLETE_FEASIBLE_POLICY");
    }
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!(
            "usage: wpt-motion-bounded-observability <case.tsv>"
        );
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

    const STATIONARY: &str = include_str!(
        "../../../research/wpt-motion-bounded-observability-v0/stationary-identity.tsv"
    );
    const MOVING: &str = include_str!(
        "../../../research/wpt-motion-bounded-observability-v0/moving-ambiguity.tsv"
    );

    fn parsed(input: &str) -> Case {
        parse_case(input).unwrap_or_else(|errors| panic!("{}", errors.join("\n")))
    }

    // Independent, deliberately non-memoized score enumerator for small cases.
    fn oracle(
        case: &Case,
        belief: &BTreeSet<String>,
        elapsed_us: u64,
        evidence_deadline_us: u64,
        remaining_actions: usize,
    ) -> Vec<(u64, u64, usize)> {
        if unique_pose(case, belief).is_some() {
            return if elapsed_us <= evidence_deadline_us {
                vec![(0, 0, 0)]
            } else {
                Vec::new()
            };
        }
        if remaining_actions == 0 {
            return Vec::new();
        }

        let mut all_scores = Vec::new();
        for action in case.actions.values() {
            if action.ripple_ppm > case.max_ripple_ppm
                || action.zvs_margin_permille < case.min_zvs_margin_permille
                || action.service_drop_ppm > case.max_service_drop_ppm
            {
                continue;
            }
            let Some(end_time) = elapsed_us.checked_add(action.duration_us) else {
                continue;
            };
            if end_time > action.valid_until_us {
                continue;
            }

            let mut outcome_states: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
            let mut valid = true;
            for state in belief {
                match action.transitions.get(state) {
                    None => {
                        valid = false;
                        break;
                    }
                    Some(edges) if edges.is_empty() => {
                        valid = false;
                        break;
                    }
                    Some(edges) => {
                        for edge in edges {
                            if !edge.in_domain {
                                valid = false;
                                break;
                            }
                            for outcome in &edge.outcomes {
                                outcome_states
                                    .entry(outcome.clone())
                                    .or_default()
                                    .insert(edge.to_state.clone());
                            }
                        }
                    }
                }
                if !valid {
                    break;
                }
            }
            if !valid || outcome_states.is_empty() {
                continue;
            }

            let mut combinations = vec![Vec::<(u64, u64, usize)>::new()];
            for next_belief in outcome_states.values() {
                let child_scores = oracle(
                    case,
                    next_belief,
                    end_time,
                    evidence_deadline_us.min(action.valid_until_us),
                    remaining_actions - 1,
                );
                if child_scores.is_empty() {
                    valid = false;
                    break;
                }
                let mut expanded = Vec::new();
                for prior in &combinations {
                    for child in &child_scores {
                        let mut next = prior.clone();
                        next.push(*child);
                        expanded.push(next);
                    }
                }
                combinations = expanded;
            }
            if !valid {
                continue;
            }

            for combination in combinations {
                let max_energy = combination.iter().map(|score| score.0).max().unwrap_or(0);
                let max_latency = combination.iter().map(|score| score.1).max().unwrap_or(0);
                let max_actions = combination.iter().map(|score| score.2).max().unwrap_or(0);
                let Some(energy) = action.energy_cost_uj.checked_add(max_energy) else {
                    continue;
                };
                let Some(latency) = action.duration_us.checked_add(max_latency) else {
                    continue;
                };
                let actions = 1 + max_actions;
                if energy <= case.energy_budget_uj
                    && latency <= case.latency_budget_us
                    && actions <= case.max_actions
                {
                    all_scores.push((energy, latency, actions));
                }
            }
        }
        all_scores
    }

    fn assert_all_leaves_unique(plan: &Plan) {
        if plan.terminal_pose.is_some() {
            assert!(plan.branches.is_empty());
            return;
        }
        assert!(!plan.branches.is_empty());
        for child in plan.branches.values() {
            assert_all_leaves_unique(child);
        }
    }

    #[test]
    fn stationary_identity_matches_static_adaptive_known_answer() {
        let case = parsed(STATIONARY);
        let plan = solve(&case).expect("stationary case must have a feasible policy");
        assert_eq!(plan.worst_case_energy_uj, 15);
        assert_eq!(plan.worst_case_latency_us, 7);
        assert_eq!(plan.worst_case_actions, 2);
        assert_eq!(plan.action_id.as_deref(), Some("passive_telemetry_split"));
        assert_eq!(
            plan.branches.get("0").and_then(|branch| branch.action_id.as_deref()),
            Some("left_discriminator")
        );
        assert_eq!(
            plan.branches.get("1").and_then(|branch| branch.action_id.as_deref()),
            Some("right_discriminator")
        );
        assert_all_leaves_unique(&plan);

        let oracle_scores = oracle(
            &case,
            &case.initial_states,
            0,
            u64::MAX,
            case.max_actions,
        );
        let oracle_best = oracle_scores.into_iter().min().expect("oracle has a policy");
        assert_eq!(
            oracle_best,
            (
                plan.worst_case_energy_uj,
                plan.worst_case_latency_us,
                plan.worst_case_actions
            )
        );
    }

    #[test]
    fn motion_can_preserve_ambiguity_that_static_pose_labels_would_hide() {
        let case = parsed(MOVING);
        let plan = solve(&case).expect("second probe should resolve the moving ambiguity");
        assert_eq!(plan.worst_case_energy_uj, 5);
        assert_eq!(plan.worst_case_latency_us, 2);
        assert_eq!(plan.action_id.as_deref(), Some("first_probe"));
        assert_eq!(
            plan.branches.get("0").and_then(|branch| branch.action_id.as_deref()),
            Some("second_probe")
        );
        assert_all_leaves_unique(&plan);
        let oracle_scores = oracle(&case, &case.initial_states, 0, u64::MAX, case.max_actions);
        let oracle_best = oracle_scores.into_iter().min().expect("moving oracle has a policy");
        assert_eq!(
            oracle_best,
            (
                plan.worst_case_energy_uj,
                plan.worst_case_latency_us,
                plan.worst_case_actions
            )
        );
        assert_eq!(
            plan.branches.get("0").and_then(|branch| branch.action_id.as_deref()),
            Some("second_probe")
        );
    }

    #[test]
    fn nondeterministic_motion_expands_the_reachable_state_set() {
        let case = parsed(MOVING);
        let reachable = BTreeSet::from(["A1".to_string(), "B1".to_string()]);
        assert_eq!(reachable.len(), 2);
        assert_eq!(unique_pose(&case, &reachable), None);
    }

    #[test]
    fn any_possible_out_of_domain_transition_rejects_the_action() {
        let input = MOVING
            .lines()
            .take(3)
            .collect::<Vec<_>>()
            .join("\n")
            .replacen("\ttrue\t10", "\tfalse\t10", 1)
            + "\n";
        let case = parsed(&input);
        assert!(solve(&case).is_none());
    }

    #[test]
    fn expired_action_validity_causes_abstention() {
        let input = MOVING.replace("\ttrue\t10", "\ttrue\t0");
        let case = parsed(&input);
        assert!(solve(&case).is_none());
    }

    #[test]
    fn earlier_observation_expiry_invalidates_a_later_terminal_decision() {
        let mut input = MOVING.to_string();
        for _ in 0..4 {
            input = input.replacen("\ttrue\t10", "\ttrue\t1", 1);
        }
        let case = parsed(&input);
        assert!(solve(&case).is_none());
    }

    #[test]
    fn row_permutation_does_not_change_the_policy() {
        let mut lines: Vec<&str> = STATIONARY.lines().collect();
        let header = lines.remove(0);
        lines.reverse();
        let permuted = format!("{header}\n{}\n", lines.join("\n"));
        let original = solve(&parsed(STATIONARY)).expect("original policy");
        let reordered = solve(&parsed(&permuted)).expect("permuted policy");
        assert_eq!(plan_key(&original), plan_key(&reordered));
        assert_eq!(score(&original), score(&reordered));
    }

    #[test]
    fn malformed_or_conflicting_input_is_rejected() {
        let malformed = STATIONARY.replace("SYNTHETIC_KNOWN_TRUTH", "UNREVIEWED");
        assert!(parse_case(&malformed).is_err());

        let conflict = STATIONARY.replacen(
            "P0\tP0\tP0\tP0\t0",
            "P0\tWRONG_POSE\tP0\tP0\t0",
            1,
        );
        assert!(parse_case(&conflict).is_err());
    }

    #[test]
    fn no_pose_metric_is_invented_from_opaque_state_ids() {
        let case = parsed(MOVING);
        assert_eq!(case.state_poses.get("A1").map(String::as_str), Some("POSE_A"));
        assert_eq!(case.state_poses.get("B1").map(String::as_str), Some("POSE_B"));
        assert_ne!(
            case.state_poses.get("A1"),
            case.state_poses.get("B1")
        );
    }
}
