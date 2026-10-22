// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! VIS-MEM-014: belief trapping, progress monitoring, and requirement-specific recovery.
//!
//! Deterministic synthetic state-machine diagnostic. The controller receives
//! the visible task gap, observations, progress vector, revision, and action
//! availability. It does not receive evaluator-only trap labels or world mode.
//!
//! This is not a reproduction of PoS or SCOPER and is not a native benchmark.
//!
//! rustc --edition=2024 --test tools/vision-memory-belief-trapping.rs -o /tmp/vis-mem-trap-tests
//! /tmp/vis-mem-trap-tests
//! rustc --edition=2024 -O tools/vision-memory-belief-trapping.rs -o /tmp/vis-mem-trap
//! /tmp/vis-mem-trap > vis-mem-trap.tsv

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GapKind {
    Epistemic,
    Achievement,
}

impl GapKind {
    fn label(self) -> &'static str {
        match self {
            Self::Epistemic => "epistemic",
            Self::Achievement => "achievement",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TrapLabel {
    Static,
    Cycle,
    Drift,
    None,
}

impl TrapLabel {
    fn label(self) -> &'static str {
        match self {
            Self::Static => "static",
            Self::Cycle => "cycle",
            Self::Drift => "drift",
            Self::None => "none",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WorldMode {
    StaticEpistemic,
    StaticAchievement,
    CycleAchievement,
    DriftAchievement,
    HealthySlowAchievement,
    ExogenousRevisionAchievement,
    UnsolvableEpistemic,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Action {
    Observe,
    Advance,
    Probe,
    Replan,
    Abstain,
}

impl Action {
    fn label(self) -> &'static str {
        match self {
            Self::Observe => "observe",
            Self::Advance => "advance",
            Self::Probe => "probe",
            Self::Replan => "replan",
            Self::Abstain => "abstain",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Observation {
    state_id: i32,
    progress: u8,
    distance_to_goal: i16,
    revision: u32,
    epistemic_gap_open: bool,
    achievement_gap_open: bool,
    probe_available: bool,
    replan_available: bool,
}

impl Observation {
    fn gap_open(self, gap: GapKind) -> bool {
        match gap {
            GapKind::Epistemic => self.epistemic_gap_open,
            GapKind::Achievement => self.achievement_gap_open,
        }
    }

    fn solved(self) -> bool {
        !self.epistemic_gap_open && !self.achievement_gap_open
    }

    fn advances(self, previous: Self) -> bool {
        self.progress > previous.progress || self.distance_to_goal < previous.distance_to_goal
    }
}

#[derive(Clone, Debug)]
struct Scenario {
    name: &'static str,
    gap: GapKind,
    trap: TrapLabel, // evaluator-only; never passed to policy choice
    mode: WorldMode, // evaluator-only; never passed to policy choice
    feasible: bool,  // evaluator-only; only used for handoff scoring
    initial: Observation,
}

fn initial(
    epistemic_gap_open: bool,
    achievement_gap_open: bool,
    probe_available: bool,
    replan_available: bool,
) -> Observation {
    Observation {
        state_id: 0,
        progress: 0,
        distance_to_goal: 3,
        revision: 0,
        epistemic_gap_open,
        achievement_gap_open,
        probe_available,
        replan_available,
    }
}

fn scenarios() -> Vec<Scenario> {
    vec![
        Scenario {
            name: "static_epistemic",
            gap: GapKind::Epistemic,
            trap: TrapLabel::Static,
            mode: WorldMode::StaticEpistemic,
            feasible: true,
            initial: initial(true, false, true, true),
        },
        Scenario {
            name: "static_achievement",
            gap: GapKind::Achievement,
            trap: TrapLabel::Static,
            mode: WorldMode::StaticAchievement,
            feasible: true,
            initial: initial(false, true, true, true),
        },
        Scenario {
            name: "cycle_achievement",
            gap: GapKind::Achievement,
            trap: TrapLabel::Cycle,
            mode: WorldMode::CycleAchievement,
            feasible: true,
            initial: initial(false, true, true, true),
        },
        Scenario {
            name: "drift_achievement",
            gap: GapKind::Achievement,
            trap: TrapLabel::Drift,
            mode: WorldMode::DriftAchievement,
            feasible: true,
            initial: initial(false, true, true, true),
        },
        Scenario {
            name: "healthy_slow_progress",
            gap: GapKind::Achievement,
            trap: TrapLabel::None,
            mode: WorldMode::HealthySlowAchievement,
            feasible: true,
            initial: initial(false, true, true, true),
        },
        Scenario {
            name: "exogenous_revision",
            gap: GapKind::Achievement,
            trap: TrapLabel::None,
            mode: WorldMode::ExogenousRevisionAchievement,
            feasible: true,
            initial: initial(false, true, true, true),
        },
        Scenario {
            name: "unsolvable_no_probe",
            gap: GapKind::Epistemic,
            trap: TrapLabel::Static,
            mode: WorldMode::UnsolvableEpistemic,
            feasible: false,
            initial: initial(true, false, false, false),
        },
    ]
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Policy {
    RawLoop,
    GenericTimer,
    CycleMonitor,
    GapAwareMonitor,
}

impl Policy {
    const ALL: [Self; 4] = [
        Self::RawLoop,
        Self::GenericTimer,
        Self::CycleMonitor,
        Self::GapAwareMonitor,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::RawLoop => "raw_loop",
            Self::GenericTimer => "generic_timer",
            Self::CycleMonitor => "cycle_monitor",
            Self::GapAwareMonitor => "gap_aware_monitor",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TrapSignal {
    Static,
    Cycle,
    Drift,
    Unclassified,
}

impl TrapSignal {
    fn label(self) -> &'static str {
        match self {
            Self::Static => "static",
            Self::Cycle => "cycle",
            Self::Drift => "drift",
            Self::Unclassified => "unclassified",
        }
    }
}

#[derive(Clone, Debug)]
struct StepTrace {
    action: Action,
    observation: Observation,
    trap_signal: Option<TrapSignal>,
}

#[derive(Clone, Debug)]
struct RunResult {
    scenario: &'static str,
    policy: Policy,
    gap: GapKind,
    truth_trap: TrapLabel,
    solved: bool,
    detected: bool,
    detected_class: Option<TrapSignal>,
    correct_classification: bool,
    false_positive: bool,
    handoff_correct: bool,
    actions_used: usize,
    steps_to_detect: Option<usize>,
    steps_to_recover: Option<usize>,
    repeated_state_transitions: usize,
    total_no_progress: usize,
    final_progress: u8,
    trace: Vec<StepTrace>,
}

fn raw_action(gap: GapKind) -> Action {
    match gap {
        GapKind::Epistemic => Action::Observe,
        GapKind::Achievement => Action::Advance,
    }
}

fn same_revision_tail(history: &[Observation]) -> &[Observation] {
    let Some(last) = history.last() else {
        return history;
    };
    let start = history
        .iter()
        .rposition(|o| o.revision != last.revision)
        .map(|idx| idx + 1)
        .unwrap_or(0);
    &history[start..]
}

fn no_progress_streak(history: &[Observation]) -> usize {
    let tail = same_revision_tail(history);
    let Some(last) = tail.last().copied() else { return 0; };
    let mut count = 0;
    for pair in tail.windows(2).rev() {
        let prev = pair[0];
        let next = pair[1];
        if next.advances(prev) || next.solved() {
            break;
        }
        count += 1;
    }
    if last.solved() { 0 } else { count }
}

fn classify_trap(history: &[Observation]) -> Option<TrapSignal> {
    let tail = same_revision_tail(history);
    if tail.len() < 3 {
        return None;
    }
    let last = tail[tail.len() - 1];
    if last.solved() {
        return None;
    }

    // Drift: progress has not improved while distance to goal worsened
    // in each of the two latest transitions.
    if tail.len() >= 3 {
        let a = tail[tail.len() - 3];
        let b = tail[tail.len() - 2];
        let c = tail[tail.len() - 1];
        if a.progress == b.progress
            && b.progress == c.progress
            && b.distance_to_goal > a.distance_to_goal
            && c.distance_to_goal > b.distance_to_goal
        {
            return Some(TrapSignal::Drift);
        }
    }

    // Cycle: four observed states repeat an A-B-A-B pattern without progress.
    if tail.len() >= 4 {
        let a = tail[tail.len() - 4];
        let b = tail[tail.len() - 3];
        let c = tail[tail.len() - 2];
        let d = tail[tail.len() - 1];
        if a.state_id == c.state_id
            && b.state_id == d.state_id
            && a.state_id != b.state_id
            && a.progress == d.progress
            && !d.solved()
        {
            return Some(TrapSignal::Cycle);
        }
    }

    // Static stagnation: same state and progress persists through two actions.
    let a = tail[tail.len() - 3];
    let b = tail[tail.len() - 2];
    let c = tail[tail.len() - 1];
    if a.state_id == b.state_id
        && b.state_id == c.state_id
        && a.progress == b.progress
        && b.progress == c.progress
        && a.distance_to_goal == c.distance_to_goal
    {
        return Some(TrapSignal::Static);
    }
    None
}

fn choose_action(
    policy: Policy,
    gap: GapKind,
    history: &[Observation],
) -> (Action, Option<TrapSignal>) {
    let Some(current) = history.last().copied() else {
        return (raw_action(gap), None);
    };
    if current.solved() {
        return (Action::Abstain, None);
    }

    match policy {
        Policy::RawLoop => (raw_action(gap), None),
        Policy::GenericTimer => {
            if no_progress_streak(history) >= 2 {
                (
                    if current.replan_available { Action::Replan } else { Action::Abstain },
                    Some(TrapSignal::Unclassified),
                )
            } else {
                (raw_action(gap), None)
            }
        }
        Policy::CycleMonitor => {
            let signal = classify_trap(history);
            if matches!(signal, Some(TrapSignal::Static | TrapSignal::Cycle)) {
                let action = recovery_for_gap(gap, current);
                (action, signal)
            } else {
                // This policy is intentionally not a drift detector; do not
                // count an unhandled signal as a detected-and-managed trap.
                (raw_action(gap), None)
            }
        }
        Policy::GapAwareMonitor => {
            let signal = classify_trap(history);
            if signal.is_some() {
                (recovery_for_gap(gap, current), signal)
            } else {
                (raw_action(gap), None)
            }
        }
    }
}

fn recovery_for_gap(gap: GapKind, current: Observation) -> Action {
    match gap {
        GapKind::Epistemic => {
            if current.probe_available { Action::Probe } else { Action::Abstain }
        }
        GapKind::Achievement => {
            if current.replan_available { Action::Replan } else { Action::Abstain }
        }
    }
}

fn environment_step(
    scenario: &Scenario,
    mut obs: Observation,
    action: Action,
    step_index: usize,
) -> Observation {
    if action == Action::Abstain || obs.solved() {
        return obs;
    }

    match scenario.mode {
        WorldMode::StaticEpistemic => {
            if action == Action::Probe && obs.probe_available {
                obs.state_id += 1;
                obs.progress = obs.progress.saturating_add(1);
                obs.epistemic_gap_open = false;
            }
        }
        WorldMode::StaticAchievement => {
            if action == Action::Replan && obs.replan_available {
                obs.state_id += 1;
                obs.progress = obs.progress.saturating_add(1);
                obs.distance_to_goal = 0;
                obs.achievement_gap_open = false;
            }
        }
        WorldMode::CycleAchievement => match action {
            Action::Advance => {
                obs.state_id = if obs.state_id == 1 { 2 } else { 1 };
            }
            Action::Replan if obs.replan_available => {
                obs.state_id += 1;
                obs.progress = obs.progress.saturating_add(1);
                obs.distance_to_goal = 0;
                obs.achievement_gap_open = false;
            }
            _ => {}
        },
        WorldMode::DriftAchievement => match action {
            Action::Advance => {
                obs.state_id += 1;
                obs.distance_to_goal = obs.distance_to_goal.saturating_add(1);
            }
            Action::Replan if obs.replan_available => {
                obs.state_id += 1;
                obs.progress = obs.progress.saturating_add(1);
                obs.distance_to_goal = 0;
                obs.achievement_gap_open = false;
            }
            _ => {}
        },
        WorldMode::HealthySlowAchievement => {
            if action == Action::Advance && obs.achievement_gap_open {
                obs.state_id += 1;
                obs.progress = obs.progress.saturating_add(1);
                obs.distance_to_goal = obs.distance_to_goal.saturating_sub(1);
                if obs.progress >= 3 {
                    obs.achievement_gap_open = false;
                    obs.distance_to_goal = 0;
                }
            }
        }
        WorldMode::ExogenousRevisionAchievement => {
            if step_index == 0 {
                // An external state change is part of the visible environment
                // history and must reset trap evidence instead of count as a loop.
                obs.revision = obs.revision.saturating_add(1);
                obs.state_id += 100;
                obs.distance_to_goal = obs.distance_to_goal.saturating_sub(1);
            } else if action == Action::Advance && obs.achievement_gap_open {
                obs.state_id += 1;
                obs.progress = obs.progress.saturating_add(1);
                obs.distance_to_goal = obs.distance_to_goal.saturating_sub(1);
                if obs.progress >= 2 {
                    obs.achievement_gap_open = false;
                    obs.distance_to_goal = 0;
                }
            }
        }
        WorldMode::UnsolvableEpistemic => {
            // Neither observation nor probe supplies enough evidence in this
            // fixture; only an explicit abstention/handoff is correct.
        }
    }
    obs
}

fn run_scenario(scenario: &Scenario, policy: Policy) -> RunResult {
    let mut history = vec![scenario.initial];
    let mut trace = Vec::new();
    let mut steps_to_detect = None;
    let mut steps_to_recover = None;
    let max_steps = 8;

    for step_index in 0..max_steps {
        let (action, signal) = choose_action(policy, scenario.gap, &history);
        if signal.is_some() && steps_to_detect.is_none() {
            steps_to_detect = Some(step_index + 1);
        }
        let before = *history.last().unwrap();
        let after = environment_step(scenario, before, action, step_index);
        if action != raw_action(scenario.gap) && steps_to_recover.is_none() {
            steps_to_recover = Some(step_index + 1);
        }
        trace.push(StepTrace { action, observation: after, trap_signal: signal });

        if after == before && action == Action::Abstain {
            break;
        }
        history.push(after);
        if after.solved() {
            break;
        }
    }

    let final_obs = *history.last().unwrap();
    let detected_class = trace.iter().find_map(|t| t.trap_signal);
    let detected = detected_class.is_some();
    let truth_is_trap = scenario.trap != TrapLabel::None;
    let false_positive = detected && !truth_is_trap;
    let correct_classification = match (scenario.trap, detected_class) {
        (TrapLabel::Static, Some(TrapSignal::Static)) => true,
        (TrapLabel::Cycle, Some(TrapSignal::Cycle)) => true,
        (TrapLabel::Drift, Some(TrapSignal::Drift)) => true,
        (TrapLabel::None, None) => true,
        _ => false,
    };
    let handoff = trace.last().is_some_and(|t| t.action == Action::Abstain);
    let handoff_correct = handoff && !scenario.feasible;
    let repeated_state_transitions = history.windows(2)
        .filter(|w| w[0].state_id == w[1].state_id)
        .count();
    let total_no_progress = history.windows(2)
        .filter(|w| !w[1].advances(w[0]) && !w[1].solved())
        .count();

    RunResult {
        scenario: scenario.name,
        policy,
        gap: scenario.gap,
        truth_trap: scenario.trap,
        solved: final_obs.solved(),
        detected,
        detected_class,
        correct_classification,
        false_positive,
        handoff_correct,
        actions_used: trace.len(),
        steps_to_detect,
        steps_to_recover,
        repeated_state_transitions,
        total_no_progress,
        final_progress: final_obs.progress,
        trace,
    }
}

fn rate(n: usize, d: usize) -> f64 {
    if d == 0 { 0.0 } else { n as f64 / d as f64 }
}

fn format_report() -> String {
    let cases = scenarios();
    let mut out = String::new();
    out.push_str("META\tformat\tVIS-MEM-014-v0.1\n");
    out.push_str("META\tscope\tindependent synthetic belief-trapping mechanism diagnostic\n");
    out.push_str("META\ttruth_api\tevaluator-only trap labels and feasibility never passed to choose_action\n");
    out.push_str("CASE_HEADER\tcase\tpolicy\tgap\ttruth_trap\tsolved\tdetected\tdetected_class\tcorrect_classification\tfalse_positive\thandoff_correct\tactions\tsteps_to_detect\tsteps_to_recover\trepeated_state_transitions\tno_progress_transitions\tfinal_progress\n");

    let mut results = Vec::new();
    for scenario in &cases {
        for policy in Policy::ALL {
            let r = run_scenario(scenario, policy);
            out.push_str(&format!(
                "CASE\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
                r.scenario,
                r.policy.label(),
                r.gap.label(),
                r.truth_trap.label(),
                u8::from(r.solved),
                u8::from(r.detected),
                r.detected_class.map(TrapSignal::label).unwrap_or("none"),
                u8::from(r.correct_classification),
                u8::from(r.false_positive),
                u8::from(r.handoff_correct),
                r.actions_used,
                r.steps_to_detect.map(|v| v.to_string()).unwrap_or_else(|| "NA".to_owned()),
                r.steps_to_recover.map(|v| v.to_string()).unwrap_or_else(|| "NA".to_owned()),
                r.repeated_state_transitions,
                r.total_no_progress,
                r.final_progress
            ));
            results.push(r);
        }
    }

    out.push_str("SUMMARY_HEADER\tpolicy\tcases\tsuccess_rate\ttrap_detection_recall\ttrap_class_accuracy\tfalse_positive_rate\tcorrect_handoff_on_unsolvable\tmean_actions\tmean_no_progress\n");
    for policy in Policy::ALL {
        let rows: Vec<&RunResult> = results.iter().filter(|r| r.policy == policy).collect();
        let traps: Vec<&RunResult> = rows.iter().copied().filter(|r| r.truth_trap != TrapLabel::None).collect();
        let nontraps: Vec<&RunResult> = rows.iter().copied().filter(|r| r.truth_trap == TrapLabel::None).collect();
        let solved = rows.iter().filter(|r| r.solved).count();
        let detected = traps.iter().filter(|r| r.detected).count();
        let class_correct = traps.iter().filter(|r| r.correct_classification).count();
        let false_positives = nontraps.iter().filter(|r| r.false_positive).count();
        let handoff_correct = rows.iter().filter(|r| r.handoff_correct).count();
        let mean_actions = rows.iter().map(|r| r.actions_used as f64).sum::<f64>() / rows.len().max(1) as f64;
        let mean_no_progress = rows.iter().map(|r| r.total_no_progress as f64).sum::<f64>() / rows.len().max(1) as f64;
        out.push_str(&format!(
            "SUMMARY\t{}\t{}\t{:.3}\t{:.3}\t{:.3}\t{:.3}\t{}\t{:.3}\t{:.3}\n",
            policy.label(),
            rows.len(),
            rate(solved, rows.len()),
            rate(detected, traps.len()),
            rate(class_correct, traps.len()),
            rate(false_positives, nontraps.len()),
            handoff_correct,
            mean_actions,
            mean_no_progress
        ));
    }
    out
}

fn main() {
    print!("{}", format_report());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn case(name: &str) -> Scenario {
        scenarios().into_iter().find(|s| s.name == name).unwrap()
    }

    fn run(policy: Policy, name: &str) -> RunResult {
        run_scenario(&case(name), policy)
    }

    #[test]
    fn gap_aware_monitor_recovers_static_epistemic_trap_with_probe() {
        let r = run(Policy::GapAwareMonitor, "static_epistemic");
        assert!(r.solved);
        assert_eq!(r.trace.iter().find(|t| t.action == Action::Probe).unwrap().action, Action::Probe);
        assert_eq!(r.detected_class, Some(TrapSignal::Static));
    }

    #[test]
    fn gap_aware_monitor_recovers_static_achievement_trap_with_replan() {
        let r = run(Policy::GapAwareMonitor, "static_achievement");
        assert!(r.solved);
        assert!(r.trace.iter().any(|t| t.action == Action::Replan));
        assert_eq!(r.detected_class, Some(TrapSignal::Static));
    }

    #[test]
    fn cycle_detector_finds_abab_recurrence_and_recovers() {
        let r = run(Policy::GapAwareMonitor, "cycle_achievement");
        assert!(r.solved);
        assert_eq!(r.detected_class, Some(TrapSignal::Cycle));
    }

    #[test]
    fn drift_monitor_detects_increasing_distance_and_recovers() {
        let r = run(Policy::GapAwareMonitor, "drift_achievement");
        assert!(r.solved);
        assert_eq!(r.detected_class, Some(TrapSignal::Drift));
    }

    #[test]
    fn healthy_monotonic_progress_does_not_trigger_false_alarm() {
        for policy in [Policy::CycleMonitor, Policy::GapAwareMonitor] {
            let r = run(policy, "healthy_slow_progress");
            assert!(r.solved);
            assert!(!r.detected);
            assert!(!r.false_positive);
        }
    }

    #[test]
    fn exogenous_revision_resets_trap_history() {
        let r = run(Policy::GapAwareMonitor, "exogenous_revision");
        assert!(r.solved);
        assert!(!r.detected);
        assert!(!r.false_positive);
    }

    #[test]
    fn generic_timer_detects_stagnation_but_uses_wrong_recovery_for_epistemic_trap() {
        let r = run(Policy::GenericTimer, "static_epistemic");
        assert!(!r.solved);
        assert!(r.detected);
        assert_eq!(r.detected_class, Some(TrapSignal::Unclassified));
        assert!(!r.correct_classification);
    }

    #[test]
    fn generic_timer_recovers_by_timeout_without_classifying_cycle() {
        let static_r = run(Policy::GenericTimer, "static_achievement");
        assert!(static_r.solved);
        let cycle_r = run(Policy::GenericTimer, "cycle_achievement");
        assert!(cycle_r.solved);
        assert_eq!(cycle_r.detected_class, Some(TrapSignal::Unclassified));
        assert!(!cycle_r.correct_classification);
    }

    #[test]
    fn unsolvable_gap_gets_handoff_not_fake_success() {
        let r = run(Policy::GapAwareMonitor, "unsolvable_no_probe");
        assert!(!r.solved);
        assert!(r.handoff_correct);
        assert_eq!(r.trace.last().unwrap().action, Action::Abstain);
    }

    #[test]
    fn cycle_monitor_misses_drift_that_gap_aware_monitor_recovers() {
        let cycle_only = run(Policy::CycleMonitor, "drift_achievement");
        assert!(!cycle_only.solved);
        assert!(!cycle_only.detected);
        let gap_aware = run(Policy::GapAwareMonitor, "drift_achievement");
        assert!(gap_aware.solved);
        assert_eq!(gap_aware.detected_class, Some(TrapSignal::Drift));
    }

    #[test]
    fn raw_loop_fails_static_and_drift_controls() {
        assert!(!run(Policy::RawLoop, "static_achievement").solved);
        assert!(!run(Policy::RawLoop, "drift_achievement").solved);
    }

    #[test]
    fn every_policy_has_a_deterministic_bounded_report() {
        let a = format_report();
        let b = format_report();
        assert_eq!(a, b);
        assert_eq!(a.lines().filter(|l| l.starts_with("CASE\t")).count(), 28);
        assert_eq!(a.lines().filter(|l| l.starts_with("SUMMARY\t")).count(), 4);
    }

    #[test]
    fn detector_never_reads_evaluator_trap_label_or_world_mode() {
        let source = include_str!("vision-memory-belief-trapping.rs");
        let start = source.find("fn choose_action(").unwrap();
        let end = source[start..].find("\nfn recovery_for_gap(").unwrap() + start;
        let decider = &source[start..end];
        assert!(!decider.contains("scenario"));
        assert!(!decider.contains("truth_trap"));
        assert!(!decider.contains("WorldMode"));
        assert!(!decider.contains("feasible"));
    }

    #[test]
    fn current_visible_revision_is_part_of_trap_window() {
        let mut hist = vec![
            Observation { state_id:0, progress:0, distance_to_goal:3, revision:0, epistemic_gap_open:false, achievement_gap_open:true, probe_available:true,replan_available:true },
            Observation { state_id:0, progress:0, distance_to_goal:3, revision:0, epistemic_gap_open:false, achievement_gap_open:true, probe_available:true,replan_available:true },
            Observation { state_id:100, progress:0, distance_to_goal:2, revision:1, epistemic_gap_open:false, achievement_gap_open:true, probe_available:true,replan_available:true },
        ];
        assert_eq!(classify_trap(&hist), None);
        hist.push(Observation { state_id:101, progress:1, distance_to_goal:1, revision:1, epistemic_gap_open:false, achievement_gap_open:true, probe_available:true,replan_available:true });
        assert_eq!(classify_trap(&hist), None);
    }
}
