// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Exact adaptive probe-policy solver over finite synthetic pose hypotheses.
//! It selects among upstream-qualified actions; it is not an estimator or inverter controller.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::{env, fs, path::Path, process::ExitCode};

const HEADER: [&str; 19] = [
    "case_id", "evidence_class", "pose_case_sha256", "action_library_sha256",
    "pose_ids", "energy_budget_uj", "latency_budget_us", "max_actions",
    "max_ripple_ppm", "min_zvs_margin_permille", "max_service_drop_ppm",
    "action_id", "action_kind", "energy_cost_uj", "duration_us", "ripple_ppm",
    "zvs_margin_permille", "service_drop_ppm", "outcomes",
];
const MAX_POSES: usize = 16;
const MAX_ACTIONS: usize = 16;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeAction {
    pub id: String,
    pub kind: String,
    pub energy_uj: u64,
    pub duration_us: u64,
    pub ripple_ppm: u64,
    pub zvs_margin_permille: u64,
    pub service_drop_ppm: u64,
    pub outcomes: Vec<BTreeSet<u32>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeCase {
    pub case_id: String,
    pub evidence_class: String,
    pub pose_case_sha256: String,
    pub action_library_sha256: String,
    pub pose_ids: Vec<String>,
    pub energy_budget_uj: u64,
    pub latency_budget_us: u64,
    pub max_actions: usize,
    pub max_ripple_ppm: u64,
    pub min_zvs_margin_permille: u64,
    pub max_service_drop_ppm: u64,
    pub actions: Vec<ProbeAction>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyPlan {
    pub terminal_pose: Option<String>,
    pub action_id: Option<String>,
    pub branches: Vec<(u32, Box<PolicyPlan>)>,
    pub worst_case_energy_uj: u64,
    pub worst_case_latency_us: u64,
    pub worst_case_actions: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixedPlan {
    pub action_ids: Vec<String>,
    pub total_energy_uj: u64,
    pub total_latency_us: u64,
    pub within_energy_budget: bool,
    pub within_latency_budget: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyReport {
    pub case_id: String,
    pub status: String,
    pub adaptive_plan: Option<PolicyPlan>,
    pub best_fixed_plan: Option<FixedPlan>,
    pub best_feasible_fixed_plan: Option<FixedPlan>,
    pub excluded_actions: Vec<(String, String)>,
}

fn digest_valid(value: &str) -> bool {
    value.len() == 64
        && value.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

fn parse_u64(value: &str, path: &str) -> Result<u64, String> {
    if value.trim() != value || value.is_empty() {
        return Err(format!("{path}: empty or untrimmed integer"));
    }
    value.parse::<u64>().map_err(|_| format!("{path}: expected non-negative integer"))
}

fn parse_list<T, F>(value: &str, path: &str, mut parse: F) -> Result<Vec<T>, String>
where
    F: FnMut(&str, &str) -> Result<T, String>,
{
    let parts: Vec<_> = value.split(',').collect();
    if parts.is_empty() || parts.iter().any(|p| p.is_empty() || p.trim() != *p) {
        return Err(format!("{path}: empty or untrimmed list element"));
    }
    parts.iter().enumerate().map(|(i, p)| parse(p, &format!("{path}[{i}]"))).collect()
}

pub fn parse_case(input: &str) -> Result<ProbeCase, Vec<String>> {
    let mut lines = input.split_terminator('\n');
    let Some(raw_header) = lines.next() else { return Err(vec!["input is empty".into()]); };
    let header = raw_header.strip_suffix('\r').unwrap_or(raw_header);
    if header.split('\t').collect::<Vec<_>>().as_slice() != HEADER.as_slice() {
        return Err(vec![format!("header must contain exactly {} declared columns", HEADER.len())]);
    }

    let mut metadata: Option<Vec<String>> = None;
    let mut case: Option<ProbeCase> = None;
    let mut errors = Vec::new();
    let mut action_ids = BTreeSet::new();

    for (offset, raw_line) in lines.enumerate() {
        let line_no = offset + 2;
        let line = raw_line.strip_suffix('\r').unwrap_or(raw_line);
        if line.is_empty() {
            errors.push(format!("line {line_no}: blank records are not allowed"));
            continue;
        }
        let fields: Vec<_> = line.split('\t').collect();
        if fields.len() != HEADER.len() {
            errors.push(format!("line {line_no}: expected {} columns, found {}", HEADER.len(), fields.len()));
            continue;
        }
        let meta = fields[..11].iter().map(|x| (*x).to_string()).collect::<Vec<_>>();
        if let Some(first) = &metadata {
            if *first != meta {
                errors.push(format!("line {line_no}: case metadata differs from the first action row"));
                continue;
            }
        } else {
            metadata = Some(meta.clone());
            if fields[0].trim().is_empty() || fields[0].trim() != fields[0] {
                errors.push(format!("line {line_no}: case_id must be a non-empty trimmed string"));
                continue;
            }
            let pose_ids = match parse_list(fields[4], "pose_ids", |s, _| {
                if s.trim().is_empty() { Err("pose ID must be non-empty".into()) } else { Ok(s.to_string()) }
            }) {
                Ok(v) => v,
                Err(e) => { errors.push(e); continue; }
            };
            let unique_pose_ids: BTreeSet<_> = pose_ids.iter().collect();
            if pose_ids.len() < 2 || pose_ids.len() > MAX_POSES || unique_pose_ids.len() != pose_ids.len() {
                errors.push(format!("line {line_no}: pose_ids must contain 2..={MAX_POSES} unique IDs"));
                continue;
            }
            if fields[1] != "SYNTHETIC_KNOWN_TRUTH" {
                errors.push(format!("line {line_no}: v0 accepts SYNTHETIC_KNOWN_TRUTH only"));
                continue;
            }
            if !digest_valid(fields[2]) || !digest_valid(fields[3]) {
                errors.push(format!("line {line_no}: pose-case and action-library digests must be lowercase SHA-256"));
                continue;
            }
            let nums = (5..=10).map(|i| parse_u64(fields[i], &format!("line {line_no} column {}", i + 1)))
                .collect::<Result<Vec<_>, _>>();
            let nums = match nums { Ok(v) => v, Err(e) => { errors.push(e); continue; } };
            if nums[0] == 0 || nums[1] == 0 || nums[2] == 0 || nums[2] > MAX_ACTIONS as u64
                || nums[3] > 1_000_000 || nums[4] > 1_000 || nums[5] > 1_000_000 {
                errors.push(format!("line {line_no}: budget or power-quality limit outside supported bounds"));
                continue;
            }
            case = Some(ProbeCase {
                case_id: fields[0].to_string(),
                evidence_class: fields[1].to_string(),
                pose_case_sha256: fields[2].to_string(),
                action_library_sha256: fields[3].to_string(),
                pose_ids,
                energy_budget_uj: nums[0],
                latency_budget_us: nums[1],
                max_actions: nums[2] as usize,
                max_ripple_ppm: nums[3],
                min_zvs_margin_permille: nums[4],
                max_service_drop_ppm: nums[5],
                actions: Vec::new(),
            });
        }

        let id = fields[11];
        if id.trim().is_empty() || id.trim() != id {
            errors.push(format!("line {line_no}: action_id must be non-empty and trimmed"));
            continue;
        }
        if !action_ids.insert(id.to_string()) {
            errors.push(format!("line {line_no}: duplicate action_id {id}"));
            continue;
        }
        let kind = fields[12];
        if !["PASSIVE_OBSERVE", "ACTIVE_EXCITATION"].contains(&kind) {
            errors.push(format!("line {line_no}: unsupported action_kind {kind:?}"));
            continue;
        }
        let nums = (13..=17).map(|i| parse_u64(fields[i], &format!("line {line_no} column {}", i + 1)))
            .collect::<Result<Vec<_>, _>>();
        let nums = match nums { Ok(v) => v, Err(e) => { errors.push(e); continue; } };
        if nums[1] == 0
            || (kind == "ACTIVE_EXCITATION" && nums[0] == 0)
            || nums[3] > 1_000
            || nums[4] > 1_000_000 {
            errors.push(format!("line {line_no}: duration must be positive, active excitation energy must be positive, and electrical metrics must be within bounds"));
            continue;
        }
        let outcomes = match parse_list(fields[18], &format!("line {line_no} outcomes"), |pose_bins, path| {
            let mut bins = BTreeSet::new();
            for (index, value) in pose_bins.split('|').enumerate() {
                if value.is_empty() || value.trim() != value {
                    return Err(format!("{path}[{index}]: empty or untrimmed outcome"));
                }
                let parsed = value.parse::<u32>()
                    .map_err(|_| format!("{path}[{index}]: outcome must be a non-negative integer class"))?;
                if !bins.insert(parsed) {
                    return Err(format!("{path}[{index}]: duplicate outcome class"));
                }
            }
            if bins.is_empty() {
                return Err(format!("{path}: at least one possible outcome is required"));
            }
            Ok(bins)
        }) {
            Ok(v) => v,
            Err(e) => { errors.push(e); continue; }
        };
        if let Some(c) = &case {
            if outcomes.len() != c.pose_ids.len() {
                errors.push(format!("line {line_no}: outcomes length does not match pose_ids"));
                continue;
            }
        }
        if let Some(c) = &mut case {
            c.actions.push(ProbeAction {
                id: id.to_string(),
                kind: kind.to_string(),
                energy_uj: nums[0],
                duration_us: nums[1],
                ripple_ppm: nums[2],
                zvs_margin_permille: nums[3],
                service_drop_ppm: nums[4],
                outcomes,
            });
        }
    }

    if let Some(c) = &mut case {
        c.actions.sort_by(|a, b| a.id.cmp(&b.id));
        if c.actions.is_empty() { errors.push("case contains no probe actions".into()); }
        if c.actions.len() > MAX_ACTIONS { errors.push(format!("action library exceeds v0 limit of {MAX_ACTIONS}")); }
    } else if errors.is_empty() {
        errors.push("case contains no usable metadata row".into());
    }
    if errors.is_empty() { Ok(case.expect("case created when errors empty")) } else { Err(errors) }
}

fn exclusion_reason(action: &ProbeAction, case: &ProbeCase) -> Option<String> {
    if action.ripple_ppm > case.max_ripple_ppm {
        return Some("RIPPLE_LIMIT".into());
    }
    if action.zvs_margin_permille < case.min_zvs_margin_permille {
        return Some("ZVS_MARGIN".into());
    }
    if action.service_drop_ppm > case.max_service_drop_ppm {
        return Some("SERVICE_DISTURBANCE".into());
    }
    None
}

fn singleton_index(mask: u16) -> Option<usize> {
    if mask.count_ones() == 1 { Some(mask.trailing_zeros() as usize) } else { None }
}

type MemoKey = (u16, u64, u64, usize);

fn outcome_groups(mask: u16, action: &ProbeAction, pose_count: usize) -> BTreeMap<u32, u16> {
    let mut possible_outcomes = BTreeSet::new();
    for pose_index in 0..pose_count {
        if (mask & (1_u16 << pose_index)) != 0 {
            possible_outcomes.extend(action.outcomes[pose_index].iter().copied());
        }
    }
    let mut groups: BTreeMap<u32, u16> = BTreeMap::new();
    for outcome in possible_outcomes {
        let mut group_mask = 0_u16;
        for pose_index in 0..pose_count {
            if (mask & (1_u16 << pose_index)) != 0
                && action.outcomes[pose_index].contains(&outcome) {
                group_mask |= 1_u16 << pose_index;
            }
        }
        groups.insert(outcome, group_mask);
    }
    groups
}

fn minimum_worst_case_energy(
    mask: u16,
    energy_left: u64,
    latency_left: u64,
    actions_left: usize,
    case: &ProbeCase,
    memo: &mut HashMap<MemoKey, Option<u64>>,
) -> Option<u64> {
    if singleton_index(mask).is_some() { return Some(0); }
    let key = (mask, energy_left, latency_left, actions_left);
    if let Some(cached) = memo.get(&key) { return *cached; }
    if actions_left == 0 { memo.insert(key, None); return None; }

    let mut best: Option<u64> = None;
    for action in &case.actions {
        if exclusion_reason(action, case).is_some()
            || action.energy_uj > energy_left
            || action.duration_us > latency_left {
            continue;
        }
        let groups = outcome_groups(mask, action, case.pose_ids.len());
        if groups.len() < 2 || groups.values().any(|child_mask| *child_mask == mask) { continue; }
        let mut worst_child = 0_u64;
        let mut feasible = true;
        for child_mask in groups.values() {
            match minimum_worst_case_energy(
                *child_mask,
                energy_left - action.energy_uj,
                latency_left - action.duration_us,
                actions_left - 1,
                case,
                memo,
            ) {
                Some(value) => worst_child = worst_child.max(value),
                None => { feasible = false; break; }
            }
        }
        if !feasible { continue; }
        let candidate = action.energy_uj + worst_child;
        best = Some(best.map(|current| current.min(candidate)).unwrap_or(candidate));
    }
    memo.insert(key, best);
    best
}

fn minimum_worst_case_latency(
    mask: u16,
    energy_left: u64,
    latency_left: u64,
    actions_left: usize,
    case: &ProbeCase,
    memo: &mut HashMap<MemoKey, Option<u64>>,
) -> Option<u64> {
    if singleton_index(mask).is_some() { return Some(0); }
    let key = (mask, energy_left, latency_left, actions_left);
    if let Some(cached) = memo.get(&key) { return *cached; }
    if actions_left == 0 { memo.insert(key, None); return None; }

    let mut best: Option<u64> = None;
    for action in &case.actions {
        if exclusion_reason(action, case).is_some()
            || action.energy_uj > energy_left
            || action.duration_us > latency_left {
            continue;
        }
        let groups = outcome_groups(mask, action, case.pose_ids.len());
        if groups.len() < 2 || groups.values().any(|child_mask| *child_mask == mask) { continue; }
        let mut worst_child = 0_u64;
        let mut feasible = true;
        for child_mask in groups.values() {
            match minimum_worst_case_latency(
                *child_mask,
                energy_left - action.energy_uj,
                latency_left - action.duration_us,
                actions_left - 1,
                case,
                memo,
            ) {
                Some(value) => worst_child = worst_child.max(value),
                None => { feasible = false; break; }
            }
        }
        if !feasible { continue; }
        let candidate = action.duration_us + worst_child;
        best = Some(best.map(|current| current.min(candidate)).unwrap_or(candidate));
    }
    memo.insert(key, best);
    best
}

fn build_min_latency_policy(
    mask: u16,
    energy_left: u64,
    latency_left: u64,
    actions_left: usize,
    case: &ProbeCase,
    latency_memo: &mut HashMap<MemoKey, Option<u64>>,
) -> Option<PolicyPlan> {
    if let Some(index) = singleton_index(mask) {
        return Some(PolicyPlan {
            terminal_pose: Some(case.pose_ids[index].clone()),
            action_id: None,
            branches: Vec::new(),
            worst_case_energy_uj: 0,
            worst_case_latency_us: 0,
            worst_case_actions: 0,
        });
    }
    if actions_left == 0 { return None; }
    let target_latency = minimum_worst_case_latency(
        mask, energy_left, latency_left, actions_left, case, latency_memo,
    )?;

    let mut best: Option<PolicyPlan> = None;
    for action in &case.actions {
        if exclusion_reason(action, case).is_some()
            || action.energy_uj > energy_left
            || action.duration_us > latency_left {
            continue;
        }
        let groups = outcome_groups(mask, action, case.pose_ids.len());
        if groups.len() < 2 || groups.values().any(|child_mask| *child_mask == mask) { continue; }
        let remaining_energy = energy_left - action.energy_uj;
        let remaining_latency = latency_left - action.duration_us;

        let mut expected_latency = 0_u64;
        let mut feasible = true;
        for child_mask in groups.values() {
            match minimum_worst_case_latency(
                *child_mask, remaining_energy, remaining_latency,
                actions_left - 1, case, latency_memo,
            ) {
                Some(value) => expected_latency = expected_latency.max(value),
                None => { feasible = false; break; }
            }
        }
        if !feasible || action.duration_us + expected_latency != target_latency {
            continue;
        }

        let mut branches = Vec::with_capacity(groups.len());
        let mut worst_child_energy = 0_u64;
        let mut worst_child_latency = 0_u64;
        let mut worst_child_actions = 0_usize;
        for (outcome, child_mask) in groups {
            let child = build_min_latency_policy(
                child_mask, remaining_energy, remaining_latency,
                actions_left - 1, case, latency_memo,
            )?;
            worst_child_energy = worst_child_energy.max(child.worst_case_energy_uj);
            worst_child_latency = worst_child_latency.max(child.worst_case_latency_us);
            worst_child_actions = worst_child_actions.max(child.worst_case_actions);
            branches.push((outcome, Box::new(child)));
        }
        let candidate = PolicyPlan {
            terminal_pose: None,
            action_id: Some(action.id.clone()),
            branches,
            worst_case_energy_uj: action.energy_uj + worst_child_energy,
            worst_case_latency_us: action.duration_us + worst_child_latency,
            worst_case_actions: 1 + worst_child_actions,
        };
        let replace = best.as_ref().map(|current| {
            candidate.action_id.as_deref().unwrap_or("")
                < current.action_id.as_deref().unwrap_or("")
        }).unwrap_or(true);
        if replace { best = Some(candidate); }
    }
    best
}

pub fn adaptive_plan(case: &ProbeCase) -> Option<PolicyPlan> {
    if case.pose_ids.len() > MAX_POSES || case.actions.len() > MAX_ACTIONS { return None; }
    let initial_mask = (1_u16 << case.pose_ids.len()) - 1;
    let mut energy_memo = HashMap::new();
    let optimal_energy = minimum_worst_case_energy(
        initial_mask, case.energy_budget_uj, case.latency_budget_us,
        case.max_actions, case, &mut energy_memo,
    )?;
    let mut latency_memo = HashMap::new();
    let plan = build_min_latency_policy(
        initial_mask, optimal_energy, case.latency_budget_us,
        case.max_actions, case, &mut latency_memo,
    )?;
    if plan.worst_case_energy_uj <= optimal_energy
        && plan.worst_case_latency_us <= case.latency_budget_us {
        Some(plan)
    } else {
        None
    }
}

fn jointly_unique(chosen: &[usize], case: &ProbeCase) -> bool {
    for left in 0..case.pose_ids.len() {
        for right in (left + 1)..case.pose_ids.len() {
            let separated = chosen.iter().any(|index| {
                let a = &case.actions[*index].outcomes[left];
                let b = &case.actions[*index].outcomes[right];
                a.is_disjoint(b)
            });
            if !separated { return false; }
        }
    }
    true
}

fn fixed_search(
    start: usize,
    chosen: &mut Vec<usize>,
    case: &ProbeCase,
    require_budget_fit: bool,
    best: &mut Option<FixedPlan>,
) {
    if !chosen.is_empty() && jointly_unique(chosen, case) {
        let energy: u64 = chosen.iter().map(|index| case.actions[*index].energy_uj).sum();
        let latency: u64 = chosen.iter().map(|index| case.actions[*index].duration_us).sum();
        let mut ids: Vec<_> = chosen.iter().map(|index| case.actions[*index].id.clone()).collect();
        ids.sort();
        let plan = FixedPlan {
            action_ids: ids,
            total_energy_uj: energy,
            total_latency_us: latency,
            within_energy_budget: energy <= case.energy_budget_uj,
            within_latency_budget: latency <= case.latency_budget_us,
        };
        if require_budget_fit && (!plan.within_energy_budget || !plan.within_latency_budget) {
            return;
        }
        let key = (plan.total_energy_uj, plan.total_latency_us, plan.action_ids.clone());
        let replace = best.as_ref().map(|b| {
            key < (b.total_energy_uj, b.total_latency_us, b.action_ids.clone())
        }).unwrap_or(true);
        if replace { *best = Some(plan); }
        return;
    }
    if chosen.len() >= case.max_actions { return; }
    for i in start..case.actions.len() {
        if exclusion_reason(&case.actions[i], case).is_some() { continue; }
        chosen.push(i);
        fixed_search(i + 1, chosen, case, require_budget_fit, best);
        chosen.pop();
    }
}

pub fn best_fixed_plan(case: &ProbeCase) -> Option<FixedPlan> {
    let mut best = None;
    fixed_search(0, &mut Vec::<usize>::new(), case, false, &mut best);
    best
}

pub fn best_feasible_fixed_plan(case: &ProbeCase) -> Option<FixedPlan> {
    let mut best = None;
    fixed_search(0, &mut Vec::<usize>::new(), case, true, &mut best);
    best
}

pub fn evaluate(case: &ProbeCase) -> PolicyReport {
    let plan = adaptive_plan(case);
    let fixed = best_fixed_plan(case);
    let excluded_actions = case.actions.iter().filter_map(|a| {
        exclusion_reason(a, case).map(|reason| (a.id.clone(), reason))
    }).collect();
    PolicyReport {
        case_id: case.case_id.clone(),
        status: if plan.is_some() { "POLICY_FOUND".into() } else { "ABSTAIN_NO_FEASIBLE_POLICY".into() },
        adaptive_plan: plan,
        best_fixed_plan: fixed,
        best_feasible_fixed_plan: best_feasible_fixed_plan(case),
        excluded_actions,
    }
}

fn render_plan(plan: &PolicyPlan, indent: usize, output: &mut String) {
    let pad = " ".repeat(indent);
    if let Some(pose) = &plan.terminal_pose {
        output.push_str(&format!("{pad}RESOLVED pose={pose}\n"));
        return;
    }
    let action = plan.action_id.as_deref().unwrap_or("UNKNOWN");
    output.push_str(&format!("{pad}APPLY {action}\n"));
    for (outcome, child) in &plan.branches {
        output.push_str(&format!("{pad}  WHEN outcome={outcome}\n"));
        render_plan(child, indent + 4, output);
    }
}

fn run(path: &Path) -> Result<(), String> {
    let input = fs::read_to_string(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let case = parse_case(&input).map_err(|e| e.join("\n"))?;
    let report = evaluate(&case);
    println!("WPT adaptive probe policy v0 — {}", case.evidence_class);
    println!("Case: {}", report.case_id);
    println!("ACTION LIBRARY:");
    for action in &case.actions {
        println!(
            "  {} [{}] incremental_energy={} synthetic µJ duration={} synthetic µs ripple={} ppm zvs_margin={} permille service_drop={} ppm",
            action.id, action.kind, action.energy_uj, action.duration_us,
            action.ripple_ppm, action.zvs_margin_permille, action.service_drop_ppm
        );
    }
    for (id, reason) in &report.excluded_actions {
        println!("EXCLUDED {id}: {reason}");
    }
    match &report.adaptive_plan {
        Some(plan) => {
            println!("ADAPTIVE POLICY: {}", report.status);
            println!("Worst-case probe energy: {} synthetic µJ", plan.worst_case_energy_uj);
            println!("Worst-case acquisition latency: {} synthetic µs", plan.worst_case_latency_us);
            println!("Worst-case actions: {}", plan.worst_case_actions);
            let mut tree = String::new();
            render_plan(plan, 0, &mut tree);
            print!("{tree}");
        }
        None => println!("ADAPTIVE POLICY: ABSTAIN — no feasible policy resolves every pose within all declared budgets"),
    }
    match &report.best_fixed_plan {
        Some(fixed) => println!(
            "BEST UNCONSTRAINED FIXED SEQUENCE: [{}] energy={} synthetic µJ latency={} synthetic µs fits_energy={} fits_latency={}",
            fixed.action_ids.join(", "),
            fixed.total_energy_uj,
            fixed.total_latency_us,
            fixed.within_energy_budget,
            fixed.within_latency_budget
        ),
        None => println!("BEST UNCONSTRAINED FIXED SEQUENCE: none"),
    }
    match &report.best_feasible_fixed_plan {
        Some(fixed) => println!(
            "BEST BUDGET-FEASIBLE FIXED SEQUENCE: [{}] energy={} synthetic µJ latency={} synthetic µs",
            fixed.action_ids.join(", "), fixed.total_energy_uj, fixed.total_latency_us
        ),
        None => println!("BEST BUDGET-FEASIBLE FIXED SEQUENCE: none"),
    }
    if let (Some(adaptive), Some(fixed)) = (&report.adaptive_plan, &report.best_fixed_plan) {
        if fixed.total_energy_uj > 0 && adaptive.worst_case_energy_uj < fixed.total_energy_uj {
            let saved = fixed.total_energy_uj - adaptive.worst_case_energy_uj;
            println!("WORST-CASE ENERGY REDUCTION VS BEST FIXED: {:.2}%", 100.0 * saved as f64 / fixed.total_energy_uj as f64);
        }
    }
    println!("CLAIM CEILING: exact decision-tree planning over synthetic outcome classes; no physical estimator, switching or safety claim");
    Ok(())
}

fn main() -> ExitCode {
    let file = env::args().nth(1).unwrap_or_else(|| "research/wpt-active-probe-policy-v0/synthetic-actions.tsv".into());
    match run(Path::new(&file)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => { eprintln!("NOT_CONFORMS: {e}"); ExitCode::FAILURE }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const FIXTURE: &str = include_str!("../../../research/wpt-active-probe-policy-v0/synthetic-actions.tsv");
    fn case() -> ProbeCase { parse_case(FIXTURE).unwrap() }

    type Cost = (u64, u64);

    fn prune_costs(mut costs: Vec<Cost>) -> Vec<Cost> {
        costs.sort_unstable();
        costs.dedup();
        let all = costs.clone();
        costs.retain(|(energy, latency)| {
            !all.iter().any(|(other_energy, other_latency)| {
                other_energy <= energy
                    && other_latency <= latency
                    && (other_energy < energy || other_latency < latency)
            })
        });
        costs.sort_unstable();
        costs
    }

    fn combine_branch_frontiers(frontiers: &[Vec<Cost>]) -> Vec<Cost> {
        let mut combined = vec![(0, 0)];
        for frontier in frontiers {
            let mut next = Vec::new();
            for (energy, latency) in &combined {
                for (child_energy, child_latency) in frontier {
                    next.push(((*energy).max(*child_energy), (*latency).max(*child_latency)));
                }
            }
            combined = prune_costs(next);
        }
        combined
    }

    // Intentionally un-memoized exhaustive oracle, structurally separate from the
    // production two-pass DP. It enumerates every admissible adaptive decision tree.
    fn brute_force_cost_frontier(
        mask: u16,
        energy_left: u64,
        latency_left: u64,
        actions_left: usize,
        case: &ProbeCase,
    ) -> Vec<Cost> {
        if singleton_index(mask).is_some() { return vec![(0, 0)]; }
        if actions_left == 0 { return Vec::new(); }

        let mut costs = Vec::new();
        for action in &case.actions {
            if exclusion_reason(action, case).is_some()
                || action.energy_uj > energy_left
                || action.duration_us > latency_left {
                continue;
            }
            let groups = outcome_groups(mask, action, case.pose_ids.len());
            if groups.len() < 2 || groups.values().any(|child| *child == mask) {
                continue;
            }

            let mut child_frontiers = Vec::new();
            let mut feasible = true;
            for child_mask in groups.values() {
                let frontier = brute_force_cost_frontier(
                    *child_mask,
                    energy_left - action.energy_uj,
                    latency_left - action.duration_us,
                    actions_left - 1,
                    case,
                );
                if frontier.is_empty() {
                    feasible = false;
                    break;
                }
                child_frontiers.push(frontier);
            }
            if !feasible { continue; }

            for (child_energy, child_latency) in combine_branch_frontiers(&child_frontiers) {
                let total_energy = action.energy_uj + child_energy;
                let total_latency = action.duration_us + child_latency;
                if total_energy <= energy_left && total_latency <= latency_left {
                    costs.push((total_energy, total_latency));
                }
            }
        }
        prune_costs(costs)
    }

    fn assert_dp_matches_exhaustive_oracle(case: &ProbeCase) {
        let mask = (1_u16 << case.pose_ids.len()) - 1;
        let frontier = brute_force_cost_frontier(
            mask, case.energy_budget_uj, case.latency_budget_us, case.max_actions, case,
        );
        let expected = frontier.into_iter().min().expect("oracle must find a feasible policy");
        let actual = adaptive_plan(case).expect("DP must find a feasible policy");
        assert_eq!((actual.worst_case_energy_uj, actual.worst_case_latency_us), expected);
    }

    #[test]
    fn two_pass_dp_matches_independent_exhaustive_tree_oracle() {
        assert_dp_matches_exhaustive_oracle(&case());

        let set = |values: &[u32]| values.iter().copied().collect::<BTreeSet<_>>();
        let noisy = ProbeCase {
            case_id: "exhaustive-noisy-support".into(),
            evidence_class: "SYNTHETIC_KNOWN_TRUTH".into(),
            pose_case_sha256: "a".repeat(64),
            action_library_sha256: "b".repeat(64),
            pose_ids: vec!["P0".into(), "P1".into(), "P2".into()],
            energy_budget_uj: 20,
            latency_budget_us: 10,
            max_actions: 2,
            max_ripple_ppm: 1_000,
            min_zvs_margin_permille: 0,
            max_service_drop_ppm: 1_000,
            actions: vec![
                ProbeAction { id: "0_root".into(), kind: "ACTIVE_EXCITATION".into(), energy_uj: 5, duration_us: 1, ripple_ppm: 0, zvs_margin_permille: 1_000, service_drop_ppm: 0,
                    outcomes: vec![set(&[0, 1]), set(&[1, 2]), set(&[3])] },
                ProbeAction { id: "1_refine".into(), kind: "ACTIVE_EXCITATION".into(), energy_uj: 4, duration_us: 2, ripple_ppm: 0, zvs_margin_permille: 1_000, service_drop_ppm: 0,
                    outcomes: vec![set(&[0]), set(&[1]), set(&[0])] },
            ],
        };
        assert_dp_matches_exhaustive_oracle(&noisy);
    }

    #[test]
    fn adaptive_policy_uses_branch_specific_probes_and_resolves_all_hypotheses() {
        let c = case();
        let p = adaptive_plan(&c).unwrap();
        assert_eq!(p.action_id.as_deref(), Some("passive_telemetry_split"));
        assert_eq!(c.actions.iter().find(|a| a.id == "passive_telemetry_split").unwrap().kind, "PASSIVE_OBSERVE");
        assert_eq!(p.worst_case_energy_uj, 15);
        assert_eq!(p.worst_case_latency_us, 7);
        assert_eq!(p.worst_case_actions, 2);
        assert_eq!(p.branches.len(), 2);
        assert_eq!(p.branches[0].1.action_id.as_deref(), Some("left_discriminator"));
        assert_eq!(p.branches[1].1.action_id.as_deref(), Some("right_discriminator"));
    }

    #[test]
    fn adaptive_policy_beats_best_fixed_sequence_and_meets_budget() {
        let c = case();
        let r = evaluate(&c);
        let a = r.adaptive_plan.unwrap();
        let f = r.best_fixed_plan.unwrap();
        assert_eq!(a.worst_case_energy_uj, 15);
        assert_eq!(f.total_energy_uj, 27);
        assert_eq!(f.action_ids, vec!["left_discriminator".to_string(), "passive_telemetry_split".to_string(), "right_discriminator".to_string()]);
        assert!(!f.within_energy_budget);
        assert!(!f.within_latency_budget);
        assert!(r.best_feasible_fixed_plan.is_none());
        assert!(a.worst_case_energy_uj <= c.energy_budget_uj);
        assert!(a.worst_case_latency_us <= c.latency_budget_us);
    }

    #[test]
    fn feasible_fixed_baseline_is_reported_separately_from_unconstrained_optimum() {
        let mut c = case();
        c.energy_budget_uj = 30;
        c.latency_budget_us = 10;
        let fixed = best_feasible_fixed_plan(&c).unwrap();
        assert_eq!(fixed.action_ids, vec!["global_probe".to_string()]);
        assert_eq!(fixed.total_energy_uj, 30);
        assert_eq!(fixed.total_latency_us, 8);
        assert!(fixed.within_energy_budget && fixed.within_latency_budget);
    }

    #[test]
    fn unsafe_high_information_actions_are_excluded_before_planning() {
        let r = evaluate(&case());
        assert!(r.excluded_actions.contains(&("unsafe_ripple".into(), "RIPPLE_LIMIT".into())));
        assert!(r.excluded_actions.contains(&("unsafe_zvs".into(), "ZVS_MARGIN".into())));
        assert!(r.excluded_actions.contains(&("unsafe_service_drop".into(), "SERVICE_DISTURBANCE".into())));
        let p = r.adaptive_plan.unwrap();
        fn contains(plan: &PolicyPlan, id: &str) -> bool {
            plan.action_id.as_deref() == Some(id) || plan.branches.iter().any(|(_, b)| contains(b, id))
        }
        for id in ["unsafe_ripple", "unsafe_zvs", "unsafe_service_drop"] {
            assert!(!contains(&p, id));
        }
    }

    #[test]
    fn insufficient_energy_budget_fails_closed() {
        let mut c = case();
        c.energy_budget_uj = 14;
        assert!(adaptive_plan(&c).is_none());
        assert_eq!(evaluate(&c).status, "ABSTAIN_NO_FEASIBLE_POLICY");
    }

    #[test]
    fn insufficient_latency_budget_fails_closed() {
        let mut c = case();
        c.latency_budget_us = 6;
        assert!(adaptive_plan(&c).is_none());
    }

    #[test]
    fn action_row_permutation_does_not_change_plan() {
        let rows: Vec<_> = FIXTURE.lines().collect();
        let input = format!("{}\n{}\n", rows[0], rows[1..].iter().rev().copied().collect::<Vec<_>>().join("\n"));
        let a = evaluate(&case());
        let b = evaluate(&parse_case(&input).unwrap());
        assert_eq!(a, b);
    }

    #[test]
    fn duplicate_action_id_fails_closed() {
        let row = FIXTURE.lines().nth(1).unwrap();
        let mut input = FIXTURE.to_string();
        input.push_str(row);
        input.push('\n');
        assert!(parse_case(&input).unwrap_err().iter().any(|e| e.contains("duplicate action_id")));
    }

    #[test]
    fn passive_observation_allows_zero_incremental_energy_but_requires_positive_latency() {
        let parsed = case();
        let passive = parsed.actions.iter().find(|a| a.id == "passive_telemetry_split").unwrap();
        assert_eq!(passive.kind, "PASSIVE_OBSERVE");
        assert_eq!(passive.energy_uj, 0);
        assert!(passive.duration_us > 0);

        let tab = char::from(9);
        let newline = char::from(10);
        let mut rows: Vec<String> = FIXTURE.lines().map(str::to_string).collect();
        let mut fields: Vec<String> = rows.last().unwrap().split(tab).map(str::to_string).collect();
        fields[14] = "0".to_string();
        let last = rows.len() - 1;
        rows[last] = fields.join(&tab.to_string());
        let bad = rows.join(&newline.to_string());
        assert!(parse_case(&bad).is_err());
    }

    #[test]
    fn active_excitation_cannot_claim_zero_incremental_energy() {
        let tab = char::from(9);
        let newline = char::from(10);
        let mut rows: Vec<String> = FIXTURE.lines().map(str::to_string).collect();
        let mut fields: Vec<String> = rows[1].split(tab).map(str::to_string).collect();
        fields[13] = "0".to_string();
        rows[1] = fields.join(&tab.to_string());
        assert!(parse_case(&rows.join(&newline.to_string())).is_err());
    }

    #[test]
    fn unknown_action_kind_fails_closed() {
        let tab = char::from(9);
        let newline = char::from(10);
        let mut rows: Vec<String> = FIXTURE.lines().map(str::to_string).collect();
        let mut fields: Vec<String> = rows[1].split(tab).map(str::to_string).collect();
        fields[12] = "MAGIC_PROBE".to_string();
        rows[1] = fields.join(&tab.to_string());
        assert!(parse_case(&rows.join(&newline.to_string())).is_err());
    }

    #[test]
    fn outcome_support_sets_parse_and_duplicate_bins_fail_closed() {
        let mut rows: Vec<String> = FIXTURE.lines().map(str::to_string).collect();
        let mut fields: Vec<String> = rows[1].split('\t').map(str::to_string).collect();
        fields[18] = "0|1,0,1,1".to_string();
        rows[1] = fields.join("\t");
        let supported = rows.join("\n");
        let parsed = parse_case(&supported).unwrap();
        let root = parsed.actions.iter().find(|a| a.id == "root_split").unwrap();
        assert!(root.outcomes[0].contains(&0));
        assert!(root.outcomes[0].contains(&1));
        let duplicate = supported.replacen("0|1,0,1,1", "0|0,0,1,1", 1);
        assert!(parse_case(&duplicate).is_err());
    }

    #[test]
    fn outcome_vector_length_must_match_pose_hypotheses() {
        let mut rows: Vec<String> = FIXTURE.lines().map(str::to_string).collect();
        let mut fields: Vec<String> = rows[1].split('\t').map(str::to_string).collect();
        fields[18] = "0,0,1".to_string();
        rows[1] = fields.join("\t");
        let bad = rows.join("\n");
        assert!(parse_case(&bad).is_err());
    }

    #[test]
    fn case_metadata_must_be_identical_across_action_rows() {
        let mut rows: Vec<String> = FIXTURE.lines().map(str::to_string).collect();
        let mut fields: Vec<String> = rows[2].split('\t').map(str::to_string).collect();
        fields[5] = "26".to_string();
        rows[2] = fields.join("\t");
        let bad = rows.join("\n");
        assert!(parse_case(&bad).is_err());
    }

    #[test]
    fn duplicate_pose_ids_and_invalid_digests_fail_closed() {
        let dup = FIXTURE.replace("P0,P1,P2,P3", "P0,P1,P1,P3");
        assert!(parse_case(&dup).is_err());
        let bad_digest = FIXTURE.replacen(&"a".repeat(64), &"A".repeat(64), 1);
        assert!(parse_case(&bad_digest).is_err());
    }

    #[test]
    fn set_valued_noisy_outcomes_keep_overlapping_pose_hypotheses_until_refined() {
        let set = |values: &[u32]| values.iter().copied().collect::<BTreeSet<_>>();
        let c = ProbeCase {
            case_id: "set-valued-noise".into(),
            evidence_class: "SYNTHETIC_KNOWN_TRUTH".into(),
            pose_case_sha256: "a".repeat(64),
            action_library_sha256: "b".repeat(64),
            pose_ids: vec!["P0".into(), "P1".into(), "P2".into()],
            energy_budget_uj: 20,
            latency_budget_us: 10,
            max_actions: 2,
            max_ripple_ppm: 1_000,
            min_zvs_margin_permille: 0,
            max_service_drop_ppm: 1_000,
            actions: vec![
                ProbeAction { id: "0_root".into(), kind: "ACTIVE_EXCITATION".into(), energy_uj: 5, duration_us: 1, ripple_ppm: 0, zvs_margin_permille: 1_000, service_drop_ppm: 0,
                    outcomes: vec![set(&[0, 1]), set(&[1, 2]), set(&[3])] },
                ProbeAction { id: "1_refine".into(), kind: "ACTIVE_EXCITATION".into(), energy_uj: 4, duration_us: 2, ripple_ppm: 0, zvs_margin_permille: 1_000, service_drop_ppm: 0,
                    outcomes: vec![set(&[0]), set(&[1]), set(&[0])] },
            ],
        };
        let p = adaptive_plan(&c).unwrap();
        assert_eq!(p.action_id.as_deref(), Some("0_root"));
        assert_eq!(p.worst_case_energy_uj, 9);
        assert_eq!(p.worst_case_latency_us, 3);
        let ambiguous_branch = p.branches.iter().find(|(outcome, _)| *outcome == 1).unwrap();
        assert_eq!(ambiguous_branch.1.action_id.as_deref(), Some("1_refine"));
    }

    #[test]
    fn global_latency_tiebreak_uses_energy_slack_from_the_worst_branch() {
        let singleton = |value| [value].into_iter().collect::<BTreeSet<u32>>();
        let c = ProbeCase {
            case_id: "global-tie-break".into(),
            evidence_class: "SYNTHETIC_KNOWN_TRUTH".into(),
            pose_case_sha256: "a".repeat(64),
            action_library_sha256: "b".repeat(64),
            pose_ids: vec!["P0".into(), "P1".into(), "P2".into(), "P3".into()],
            energy_budget_uj: 50,
            latency_budget_us: 50,
            max_actions: 3,
            max_ripple_ppm: 1_000,
            min_zvs_margin_permille: 0,
            max_service_drop_ppm: 1_000,
            actions: vec![
                ProbeAction { id: "root".into(), kind: "ACTIVE_EXCITATION".into(), energy_uj: 1, duration_us: 1, ripple_ppm: 0, zvs_margin_permille: 1_000, service_drop_ppm: 0, outcomes: vec![singleton(0), singleton(0), singleton(1), singleton(1)] },
                ProbeAction { id: "left_fast".into(), kind: "ACTIVE_EXCITATION".into(), energy_uj: 6, duration_us: 1, ripple_ppm: 0, zvs_margin_permille: 1_000, service_drop_ppm: 0, outcomes: vec![singleton(0), singleton(1), singleton(0), singleton(0)] },
                ProbeAction { id: "left_slow".into(), kind: "ACTIVE_EXCITATION".into(), energy_uj: 5, duration_us: 10, ripple_ppm: 0, zvs_margin_permille: 1_000, service_drop_ppm: 0, outcomes: vec![singleton(0), singleton(1), singleton(0), singleton(0)] },
                ProbeAction { id: "right".into(), kind: "ACTIVE_EXCITATION".into(), energy_uj: 20, duration_us: 2, ripple_ppm: 0, zvs_margin_permille: 1_000, service_drop_ppm: 0, outcomes: vec![singleton(0), singleton(0), singleton(1), singleton(0)] },
            ],
        };
        let p = adaptive_plan(&c).unwrap();
        assert_eq!(p.worst_case_energy_uj, 21);
        assert_eq!(p.worst_case_latency_us, 3);
        assert_eq!(p.action_id.as_deref(), Some("root"));
        assert_eq!(p.branches[0].1.action_id.as_deref(), Some("left_fast"));
    }

    #[test]
    fn constant_outcomes_produce_abstention() {
        let tab = char::from(9);
        let newline = char::from(10);
        let mut rows: Vec<String> = FIXTURE.lines().map(str::to_string).collect();
        for row in rows.iter_mut().skip(1) {
            let mut fields: Vec<String> = row.split(tab).map(str::to_string).collect();
            fields[18] = "0,0,0,0".to_string();
            *row = fields.join(&tab.to_string());
        }
        let bad = rows.join(&newline.to_string());
        let c = parse_case(&bad).unwrap();
        assert!(adaptive_plan(&c).is_none());
    }
}
