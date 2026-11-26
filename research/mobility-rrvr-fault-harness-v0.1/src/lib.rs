// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

const ORIGINAL_EPOCH: u64 = 7;
const ORIGINAL_QUEUE_GENERATION: u64 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arm {
    NativeRetry,
    Rrvr,
}

impl Arm {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NativeRetry => "native_retry",
            Self::Rrvr => "rrvr",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Disposition {
    Applied,
    NoEffect,
    Unknown,
}

impl Disposition {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Applied => "applied",
            Self::NoEffect => "no_effect",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scenario {
    pub scenario_id: &'static str,
    pub seed: u64,
    pub effect_before_ack: bool,
    pub acknowledgement_lost: bool,
    pub endpoint_restarts: bool,
    pub nvm_dedupe_loss: bool,
    pub observer_available: bool,
    pub observation_window_complete: bool,
    pub holder_inventory_complete: bool,
    pub edge_inventory_complete: bool,
    pub delayed_original_message: bool,
}

impl Scenario {
    pub fn frozen_input_id(&self) -> String {
        format!(
            "{}:seed={}:effect={}:ack_lost={}:endpoint_restart={}:nvm_loss={}:observer={}:window={}:holders={}:edges={}:delayed={}",
            self.scenario_id,
            self.seed,
            self.effect_before_ack,
            self.acknowledgement_lost,
            self.endpoint_restarts,
            self.nvm_dedupe_loss,
            self.observer_available,
            self.observation_window_complete,
            self.holder_inventory_complete,
            self.edge_inventory_complete,
            self.delayed_original_message
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceEntry {
    pub tick: u64,
    pub sequence: u64,
    pub event: String,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArmResult {
    pub pair_id: String,
    pub frozen_input_id: String,
    pub arm: Arm,
    pub actual_effect_count: u32,
    pub reported_disposition: Disposition,
    pub fresh_attempt_admitted: bool,
    pub unsafe_admission: bool,
    pub false_no_effect: bool,
    pub false_applied: bool,
    pub false_hold: bool,
    pub endpoint_fenced: bool,
    pub temporal_closure_issued: bool,
    pub temporal_closure_valid: bool,
    pub stale_effect_after_fence: bool,
    pub duplicate_one_shot_effect: bool,
    pub unresolved_outcome: bool,
    pub trace: Vec<TraceEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchedPair {
    pub pair_id: String,
    pub frozen_input_id: String,
    pub native: ArmResult,
    pub rrvr: ArmResult,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CampaignReport {
    pub campaign_id: &'static str,
    pub claim_ceiling: &'static str,
    pub pairs: Vec<MatchedPair>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Event {
    ReserveOriginal,
    DispatchOriginal,
    PhysicalEffectOriginal,
    Acknowledgement,
    CoordinatorCrash,
    EndpointRestart,
    CoordinatorRestart,
    Reconcile,
    Fence,
    CloseGraph,
    DecideRetry,
    PhysicalEffectRetry,
    DelayedOriginalMessage,
    Finalize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ScheduledEvent {
    tick: u64,
    sequence: u64,
    event: Event,
}

#[derive(Debug, Default)]
struct Scheduler {
    events: BTreeMap<(u64, u64), Event>,
    next_sequence: u64,
}

impl Scheduler {
    fn schedule(&mut self, tick: u64, event: Event) {
        let sequence = self.next_sequence;
        self.next_sequence += 1;
        self.events.insert((tick, sequence), event);
    }

    fn pop_next(&mut self) -> Option<ScheduledEvent> {
        let ((tick, sequence), event) = self.events.pop_first()?;
        Some(ScheduledEvent {
            tick,
            sequence,
            event,
        })
    }
}

#[derive(Debug)]
struct Endpoint {
    accepted_epoch: u64,
    queue_generation: u64,
    deduplicated_commands: BTreeSet<String>,
    effect_count: u32,
    fenced: bool,
}

impl Endpoint {
    fn new() -> Self {
        Self {
            accepted_epoch: ORIGINAL_EPOCH,
            queue_generation: ORIGINAL_QUEUE_GENERATION,
            deduplicated_commands: BTreeSet::new(),
            effect_count: 0,
            fenced: false,
        }
    }

    fn receive(
        &mut self,
        command_id: &str,
        command_epoch: u64,
        command_queue_generation: u64,
        trace_detail: &mut Vec<String>,
    ) -> bool {
        if command_epoch < self.accepted_epoch {
            trace_detail.push(format!("rejected_old_epoch:{command_id}"));
            return false;
        }
        if command_queue_generation != self.queue_generation {
            trace_detail.push(format!("rejected_queue_generation:{command_id}"));
            return false;
        }
        if self.deduplicated_commands.contains(command_id) {
            trace_detail.push(format!("rejected_duplicate_command:{command_id}"));
            return false;
        }
        self.deduplicated_commands.insert(command_id.to_owned());
        self.effect_count += 1;
        trace_detail.push(format!("effect_applied:{command_id}"));
        true
    }

    fn restart(&mut self, lose_dedupe: bool) {
        self.queue_generation += 1;
        if lose_dedupe {
            self.deduplicated_commands.clear();
        }
    }

    fn fence(&mut self) {
        self.accepted_epoch += 1;
        self.queue_generation += 1;
        self.fenced = true;
    }
}

struct Simulation<'a> {
    scenario: &'a Scenario,
    arm: Arm,
    scheduler: Scheduler,
    endpoint: Endpoint,
    trace: Vec<TraceEntry>,
    reported_disposition: Disposition,
    observer_qualified: bool,
    closure_issued: bool,
    closure_valid: bool,
    fresh_attempt_admitted: bool,
    unsafe_admission: bool,
    false_hold: bool,
    stale_effect_after_fence: bool,
    original_command_id: String,
    retry_command_id: String,
    original_effect_happened: bool,
}

impl<'a> Simulation<'a> {
    fn new(scenario: &'a Scenario, arm: Arm) -> Self {
        let mut scheduler = Scheduler::default();
        for (tick, event) in [
            (0, Event::ReserveOriginal),
            (1, Event::DispatchOriginal),
            (2, Event::PhysicalEffectOriginal),
            (3, Event::Acknowledgement),
            (4, Event::CoordinatorCrash),
            (5, Event::EndpointRestart),
            (6, Event::CoordinatorRestart),
            (7, Event::Reconcile),
            (8, Event::Fence),
            (9, Event::CloseGraph),
            (10, Event::DecideRetry),
            (11, Event::PhysicalEffectRetry),
            (12, Event::DelayedOriginalMessage),
            (13, Event::Finalize),
        ] {
            scheduler.schedule(tick, event);
        }
        let pair = scenario.scenario_id.replace('-', "_");
        Self {
            scenario,
            arm,
            scheduler,
            endpoint: Endpoint::new(),
            trace: Vec::new(),
            reported_disposition: Disposition::Unknown,
            observer_qualified: false,
            closure_issued: false,
            closure_valid: false,
            fresh_attempt_admitted: false,
            unsafe_admission: false,
            false_hold: false,
            stale_effect_after_fence: false,
            original_command_id: format!("cmd-{pair}-attempt-1"),
            retry_command_id: format!("cmd-{pair}-attempt-2"),
            original_effect_happened: false,
        }
    }

    fn push_trace(&mut self, tick: u64, sequence: u64, event: &str, detail: &str) {
        self.trace.push(TraceEntry {
            tick,
            sequence,
            event: event.to_owned(),
            detail: detail.to_owned(),
        });
    }

    fn run(mut self) -> ArmResult {
        let mut endpoint_notes = Vec::new();
        while let Some(scheduled) = self.scheduler.pop_next() {
            endpoint_notes.clear();
            let event_name = match scheduled.event {
                Event::ReserveOriginal => {
                    let command_id = self.original_command_id.clone();
                    self.push_trace(
                        scheduled.tick,
                        scheduled.sequence,
                        "reserve",
                        &command_id,
                    );
                    "reserve"
                }
                Event::DispatchOriginal => {
                    self.push_trace(
                        scheduled.tick,
                        scheduled.sequence,
                        "dispatch",
                        &format!(
                            "{} epoch={} queue={}",
                            self.original_command_id, ORIGINAL_EPOCH, ORIGINAL_QUEUE_GENERATION
                        ),
                    );
                    "dispatch"
                }
                Event::PhysicalEffectOriginal => {
                    if self.scenario.effect_before_ack {
                        let command = self.original_command_id.clone();
                        self.endpoint.receive(
                            &command,
                            ORIGINAL_EPOCH,
                            ORIGINAL_QUEUE_GENERATION,
                            &mut endpoint_notes,
                        );
                        self.original_effect_happened = self.endpoint.effect_count > 0;
                    }
                    self.push_trace(
                        scheduled.tick,
                        scheduled.sequence,
                        "original_effect",
                        &format!("{:?}", endpoint_notes),
                    );
                    "original_effect"
                }
                Event::Acknowledgement => {
                    let detail = if self.scenario.acknowledgement_lost {
                        "acknowledgement_lost"
                    } else {
                        "acknowledgement_received"
                    };
                    self.push_trace(scheduled.tick, scheduled.sequence, "ack", detail);
                    "ack"
                }
                Event::CoordinatorCrash => {
                    self.push_trace(
                        scheduled.tick,
                        scheduled.sequence,
                        "coordinator_crash",
                        "volatile coordinator state lost",
                    );
                    "coordinator_crash"
                }
                Event::EndpointRestart => {
                    if self.scenario.endpoint_restarts {
                        self.endpoint.restart(false);
                    }
                    if self.scenario.nvm_dedupe_loss {
                        self.endpoint.deduplicated_commands.clear();
                    }
                    self.push_trace(
                        scheduled.tick,
                        scheduled.sequence,
                        "endpoint_restart",
                        &format!(
                            "restarted={} queue_generation={} dedupe_lost={}",
                            self.scenario.endpoint_restarts,
                            self.endpoint.queue_generation,
                            self.scenario.endpoint_restarts && self.scenario.nvm_dedupe_loss
                        ),
                    );
                    "endpoint_restart"
                }
                Event::CoordinatorRestart => {
                    self.push_trace(
                        scheduled.tick,
                        scheduled.sequence,
                        "coordinator_restart",
                        "recovered with no acknowledgement",
                    );
                    "coordinator_restart"
                }
                Event::Reconcile => {
                    if self.arm == Arm::NativeRetry {
                        self.reported_disposition = Disposition::NoEffect;
                        self.observer_qualified = false;
                    } else if self.scenario.observer_available
                        && self.scenario.observation_window_complete
                    {
                        self.reported_disposition = if self.original_effect_happened {
                            Disposition::Applied
                        } else {
                            Disposition::NoEffect
                        };
                        self.observer_qualified = true;
                    } else {
                        self.reported_disposition = Disposition::Unknown;
                        self.observer_qualified = false;
                    }
                    self.push_trace(
                        scheduled.tick,
                        scheduled.sequence,
                        "reconcile",
                        self.reported_disposition.as_str(),
                    );
                    "reconcile"
                }
                Event::Fence => {
                    if self.arm == Arm::Rrvr {
                        self.endpoint.fence();
                    }
                    self.push_trace(
                        scheduled.tick,
                        scheduled.sequence,
                        "fence",
                        &format!(
                            "accepted={} epoch={} queue_generation={}",
                            self.endpoint.fenced,
                            self.endpoint.accepted_epoch,
                            self.endpoint.queue_generation
                        ),
                    );
                    "fence"
                }
                Event::CloseGraph => {
                    if self.arm == Arm::Rrvr {
                        self.closure_issued = self.endpoint.fenced
                            && self.scenario.holder_inventory_complete
                            && self.scenario.edge_inventory_complete;
                        self.closure_valid = self.closure_issued;
                    }
                    self.push_trace(
                        scheduled.tick,
                        scheduled.sequence,
                        "temporal_closure",
                        &format!(
                            "issued={} valid={} holders_complete={} edges_complete={}",
                            self.closure_issued,
                            self.closure_valid,
                            self.scenario.holder_inventory_complete,
                            self.scenario.edge_inventory_complete
                        ),
                    );
                    "temporal_closure"
                }
                Event::DecideRetry => {
                    if self.arm == Arm::NativeRetry {
                        self.fresh_attempt_admitted = true;
                        self.unsafe_admission = self.original_effect_happened;
                    } else {
                        self.fresh_attempt_admitted = self.reported_disposition
                            == Disposition::NoEffect
                            && self.observer_qualified
                            && self.closure_valid;
                        self.unsafe_admission = self.fresh_attempt_admitted
                            && (self.reported_disposition != Disposition::NoEffect
                                || !self.observer_qualified
                                || !self.closure_valid);
                        self.false_hold = !self.fresh_attempt_admitted
                            && self.reported_disposition == Disposition::NoEffect
                            && self.observer_qualified
                            && self.closure_valid;
                    }
                    self.push_trace(
                        scheduled.tick,
                        scheduled.sequence,
                        "fresh_attempt_decision",
                        &format!(
                            "admitted={} unsafe={} false_hold={}",
                            self.fresh_attempt_admitted, self.unsafe_admission, self.false_hold
                        ),
                    );
                    "fresh_attempt_decision"
                }
                Event::PhysicalEffectRetry => {
                    if self.fresh_attempt_admitted {
                        let command = self.retry_command_id.clone();
                        self.endpoint.receive(
                            &command,
                            self.endpoint.accepted_epoch,
                            self.endpoint.queue_generation,
                            &mut endpoint_notes,
                        );
                    }
                    self.push_trace(
                        scheduled.tick,
                        scheduled.sequence,
                        "retry_effect",
                        &format!("{:?}", endpoint_notes),
                    );
                    "retry_effect"
                }
                Event::DelayedOriginalMessage => {
                    if self.scenario.delayed_original_message {
                        let command = self.original_command_id.clone();
                        let before = self.endpoint.effect_count;
                        self.endpoint.receive(
                            &command,
                            ORIGINAL_EPOCH,
                            ORIGINAL_QUEUE_GENERATION,
                            &mut endpoint_notes,
                        );
                        if self.endpoint.effect_count > before && self.endpoint.fenced {
                            self.stale_effect_after_fence = true;
                        }
                    }
                    self.push_trace(
                        scheduled.tick,
                        scheduled.sequence,
                        "delayed_original_message",
                        &format!("{:?}", endpoint_notes),
                    );
                    "delayed_original_message"
                }
                Event::Finalize => {
                    self.push_trace(
                        scheduled.tick,
                        scheduled.sequence,
                        "finalize",
                        &format!("effect_count={}", self.endpoint.effect_count),
                    );
                    "finalize"
                }
            };
            debug_assert_eq!(
                self.trace.last().map(|entry| entry.event.as_str()),
                Some(event_name)
            );
        }

        // The oracle disposition is scoped to the original attempt at reconciliation time.
        // A later authorised retry may legitimately create an effect and must not retroactively
        // turn the original NoEffect observation into a false verdict.
        let false_no_effect =
            self.reported_disposition == Disposition::NoEffect && self.original_effect_happened;
        let false_applied =
            self.reported_disposition == Disposition::Applied && !self.original_effect_happened;
        let duplicate_one_shot_effect = self.endpoint.effect_count > 1;
        let unresolved_outcome = self.reported_disposition == Disposition::Unknown;

        ArmResult {
            pair_id: self.scenario.scenario_id.to_owned(),
            frozen_input_id: self.scenario.frozen_input_id(),
            arm: self.arm,
            actual_effect_count: self.endpoint.effect_count,
            reported_disposition: self.reported_disposition,
            fresh_attempt_admitted: self.fresh_attempt_admitted,
            unsafe_admission: self.unsafe_admission,
            false_no_effect,
            false_applied,
            false_hold: self.false_hold,
            endpoint_fenced: self.endpoint.fenced,
            temporal_closure_issued: self.closure_issued,
            temporal_closure_valid: self.closure_valid,
            stale_effect_after_fence: self.stale_effect_after_fence,
            duplicate_one_shot_effect,
            unresolved_outcome,
            trace: self.trace,
        }
    }
}

pub fn run_arm(scenario: &Scenario, arm: Arm) -> ArmResult {
    Simulation::new(scenario, arm).run()
}

pub fn run_pair(scenario: &Scenario) -> MatchedPair {
    let frozen_input_id = scenario.frozen_input_id();
    let native = run_arm(scenario, Arm::NativeRetry);
    let rrvr = run_arm(scenario, Arm::Rrvr);
    debug_assert_eq!(native.frozen_input_id, frozen_input_id);
    debug_assert_eq!(rrvr.frozen_input_id, frozen_input_id);
    MatchedPair {
        pair_id: scenario.scenario_id.to_owned(),
        frozen_input_id,
        native,
        rrvr,
    }
}

pub fn known_scenarios() -> Vec<Scenario> {
    vec![
        Scenario {
            scenario_id: "effect_then_lost_ack",
            seed: 11,
            effect_before_ack: true,
            acknowledgement_lost: true,
            endpoint_restarts: false,
            nvm_dedupe_loss: false,
            observer_available: true,
            observation_window_complete: true,
            holder_inventory_complete: true,
            edge_inventory_complete: true,
            delayed_original_message: true,
        },
        Scenario {
            scenario_id: "no_effect_then_lost_ack",
            seed: 12,
            effect_before_ack: false,
            acknowledgement_lost: true,
            endpoint_restarts: true,
            nvm_dedupe_loss: true,
            observer_available: true,
            observation_window_complete: true,
            holder_inventory_complete: true,
            edge_inventory_complete: true,
            delayed_original_message: true,
        },
        Scenario {
            scenario_id: "observer_outage",
            seed: 13,
            effect_before_ack: true,
            acknowledgement_lost: true,
            endpoint_restarts: false,
            nvm_dedupe_loss: false,
            observer_available: false,
            observation_window_complete: false,
            holder_inventory_complete: true,
            edge_inventory_complete: true,
            delayed_original_message: true,
        },
        Scenario {
            scenario_id: "incomplete_command_graph",
            seed: 14,
            effect_before_ack: false,
            acknowledgement_lost: true,
            endpoint_restarts: false,
            nvm_dedupe_loss: false,
            observer_available: true,
            observation_window_complete: true,
            holder_inventory_complete: true,
            edge_inventory_complete: false,
            delayed_original_message: true,
        },
        Scenario {
            scenario_id: "restart_loses_endpoint_dedupe",
            seed: 15,
            effect_before_ack: true,
            acknowledgement_lost: true,
            endpoint_restarts: true,
            nvm_dedupe_loss: true,
            observer_available: true,
            observation_window_complete: true,
            holder_inventory_complete: true,
            edge_inventory_complete: true,
            delayed_original_message: true,
        },
        Scenario {
            scenario_id: "dedupe_loss_without_queue_rollover",
            seed: 16,
            effect_before_ack: true,
            acknowledgement_lost: true,
            endpoint_restarts: false,
            nvm_dedupe_loss: true,
            observer_available: true,
            observation_window_complete: true,
            holder_inventory_complete: true,
            edge_inventory_complete: true,
            delayed_original_message: true,
        },
    ]
}

pub fn run_campaign() -> CampaignReport {
    CampaignReport {
        campaign_id: "rrvr-crash-replay-known-cases-v0.1",
        claim_ceiling: "Deterministic process-fault reference model only; no simulator or physical campaign.",
        pairs: known_scenarios().iter().map(run_pair).collect(),
    }
}

fn json_escape(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len() + 2);
    for character in value.chars() {
        match character {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            c if c.is_control() => escaped.push('?'),
            c => escaped.push(c),
        }
    }
    escaped
}

fn arm_json(result: &ArmResult) -> String {
    let trace = result
        .trace
        .iter()
        .map(|entry| {
            format!(
                "{{\"tick\":{},\"sequence\":{},\"event\":\"{}\",\"detail\":\"{}\"}}",
                entry.tick,
                entry.sequence,
                json_escape(&entry.event),
                json_escape(&entry.detail)
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"arm\":\"{}\",\"actual_effect_count\":{},\"reported_disposition\":\"{}\",\"fresh_attempt_admitted\":{},\"unsafe_admission\":{},\"false_no_effect\":{},\"false_applied\":{},\"false_hold\":{},\"endpoint_fenced\":{},\"temporal_closure_issued\":{},\"temporal_closure_valid\":{},\"stale_effect_after_fence\":{},\"duplicate_one_shot_effect\":{},\"unresolved_outcome\":{},\"trace\":[{}]}}",
        result.arm.as_str(),
        result.actual_effect_count,
        result.reported_disposition.as_str(),
        result.fresh_attempt_admitted,
        result.unsafe_admission,
        result.false_no_effect,
        result.false_applied,
        result.false_hold,
        result.endpoint_fenced,
        result.temporal_closure_issued,
        result.temporal_closure_valid,
        result.stale_effect_after_fence,
        result.duplicate_one_shot_effect,
        result.unresolved_outcome,
        trace
    )
}

pub fn report_json(report: &CampaignReport) -> String {
    let pairs = report
        .pairs
        .iter()
        .map(|pair| {
            format!(
                "{{\"pair_id\":\"{}\",\"frozen_input_id\":\"{}\",\"native\":{},\"rrvr\":{}}}",
                json_escape(&pair.pair_id),
                json_escape(&pair.frozen_input_id),
                arm_json(&pair.native),
                arm_json(&pair.rrvr)
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"campaign_id\":\"{}\",\"claim_ceiling\":\"{}\",\"pair_n\":{},\"pairs\":[{}]}}",
        json_escape(report.campaign_id),
        json_escape(report.claim_ceiling),
        report.pairs.len(),
        pairs
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scenario(
        id: &'static str,
        effect: bool,
        observer: bool,
        coverage: bool,
        graph: bool,
    ) -> Scenario {
        Scenario {
            scenario_id: id,
            seed: 42,
            effect_before_ack: effect,
            acknowledgement_lost: true,
            endpoint_restarts: false,
            nvm_dedupe_loss: false,
            observer_available: observer,
            observation_window_complete: coverage,
            holder_inventory_complete: graph,
            edge_inventory_complete: graph,
            delayed_original_message: true,
        }
    }

    #[test]
    fn matched_arms_share_frozen_inputs_but_not_realized_outcomes() {
        let input = scenario("matched-case", true, true, true, true);
        let pair = run_pair(&input);
        assert_eq!(pair.native.frozen_input_id, pair.rrvr.frozen_input_id);
        assert_eq!(pair.native.reported_disposition, Disposition::NoEffect);
        assert_eq!(pair.rrvr.reported_disposition, Disposition::Applied);
    }

    #[test]
    fn native_blind_retry_can_duplicate_an_effect_after_lost_ack() {
        let pair = run_pair(&scenario("blind-retry", true, true, true, true));
        assert!(pair.native.unsafe_admission);
        assert!(pair.native.false_no_effect);
        assert!(pair.native.duplicate_one_shot_effect);
        assert_eq!(pair.native.actual_effect_count, 2);
    }

    #[test]
    fn rrvr_reconciles_applied_effect_and_rejects_stale_message() {
        let pair = run_pair(&scenario("applied-reconcile", true, true, true, true));
        assert!(!pair.rrvr.fresh_attempt_admitted);
        assert!(pair.rrvr.endpoint_fenced);
        assert!(pair.rrvr.temporal_closure_valid);
        assert!(!pair.rrvr.stale_effect_after_fence);
        assert_eq!(pair.rrvr.actual_effect_count, 1);
    }

    #[test]
    fn rrvr_retries_only_after_observed_no_effect_and_graph_closure() {
        let pair = run_pair(&scenario("no-effect-reconcile", false, true, true, true));
        assert_eq!(pair.rrvr.reported_disposition, Disposition::NoEffect);
        assert!(pair.rrvr.fresh_attempt_admitted);
        assert!(!pair.rrvr.unsafe_admission);
        assert_eq!(pair.rrvr.actual_effect_count, 1);
        assert!(!pair.rrvr.false_no_effect);
    }

    #[test]
    fn observer_outage_preserves_unknown_and_blocks_fresh_attempt() {
        let pair = run_pair(&scenario("observer-failed", true, false, false, true));
        assert_eq!(pair.rrvr.reported_disposition, Disposition::Unknown);
        assert!(pair.rrvr.unresolved_outcome);
        assert!(!pair.rrvr.fresh_attempt_admitted);
    }

    #[test]
    fn incomplete_graph_blocks_fresh_attempt_even_after_no_effect_observation() {
        let pair = run_pair(&scenario("incomplete-graph", false, true, true, false));
        assert_eq!(pair.rrvr.reported_disposition, Disposition::NoEffect);
        assert!(!pair.rrvr.temporal_closure_valid);
        assert!(!pair.rrvr.fresh_attempt_admitted);
    }

    #[test]
    fn dedupe_state_loss_allows_native_old_command_replay_but_fence_blocks_rrvr() {
        let mut input = scenario("nvm-loss", true, true, true, true);
        input.nvm_dedupe_loss = true;
        let pair = run_pair(&input);
        assert!(pair.native.duplicate_one_shot_effect);
        assert_eq!(pair.native.actual_effect_count, 3);
        assert!(!pair.rrvr.stale_effect_after_fence);
        assert_eq!(pair.rrvr.actual_effect_count, 1);
    }

    #[test]
    fn scheduler_trace_is_deterministic() {
        let input = scenario("deterministic", false, true, true, true);
        assert_eq!(run_pair(&input), run_pair(&input));
        assert_eq!(report_json(&run_campaign()), report_json(&run_campaign()));
    }
}
