// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Item-accounted textile feed, singulation and sensor-presentation evidence.
//!
//! This contract distinguishes physical presentation from recognition and
//! downstream material acceptance. Every incoming item must receive exactly
//! one terminal disposition, including held, manually reviewed, rejected,
//! recirculated and not-recovered items. Derived rates retain their denominators.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceRef {
    pub evidence_id: String,
    /// SHA-256 of the exact observation artifact, lowercase hexadecimal.
    pub sha256: String,
    pub method_ref: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PresentationState {
    /// Exactly one item is inside the declared sensing field of view.
    SingleItemInFieldOfView,
    /// More than one item occupies the presentation/sensing zone.
    Overlap,
    /// The picker or feeder moved more than one item in one attempt.
    DoublePick,
    /// A fold, wrinkle or occlusion prevents a valid observation.
    OccludedOrFolded,
    /// Item is outside the declared sensing/pick zone.
    OutOfField,
    /// Jam, blocked conveyor or failed mechanical feed.
    JammedOrBlocked,
    /// Presentation state was not adjudicated.
    Unknown,
}

impl PresentationState {
    fn is_single(self) -> bool {
        self == Self::SingleItemInFieldOfView
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IncomingItem {
    pub item_id: String,
    pub lot_id: String,
    /// Unique sequence number assigned at intake; gaps are permitted.
    pub intake_sequence: u64,
    /// Independent reference destination, when available.
    pub reference_destination: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PresentationAttempt {
    pub attempt_id: String,
    pub item_id: String,
    /// One-based attempt number for this incoming item.
    pub attempt_number: u32,
    /// Link to the immediately preceding attempt for this item.
    pub predecessor_attempt_id: Option<String>,
    pub state: PresentationState,
    pub started_at_ns: u64,
    pub completed_at_ns: u64,
    pub evidence: EvidenceRef,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TerminalOutcome {
    Routed,
    ManualReview,
    Held,
    Rejected,
    NotRecovered,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerminalDisposition {
    pub item_id: String,
    pub outcome: TerminalOutcome,
    /// Required only when outcome is routed.
    pub actual_destination: Option<String>,
    pub completed_at_ns: u64,
    pub evidence: EvidenceRef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FeedRun {
    pub run_id: String,
    pub line_id: String,
    pub configuration_revision: String,
    pub incoming_items: Vec<IncomingItem>,
    pub attempts: Vec<PresentationAttempt>,
    pub terminal_dispositions: Vec<TerminalDisposition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ValidationFinding {
    pub code: String,
    pub item_id: Option<String>,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ValidationReport {
    pub valid: bool,
    pub findings: Vec<ValidationFinding>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CountRate {
    pub numerator: u64,
    pub denominator: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FeedMetrics {
    pub incoming_items: u64,
    pub presentation_attempts: u64,
    /// First attempt was a valid single-item presentation / all incoming items.
    pub first_pass_single_presentation: CountRate,
    /// At least one attempt achieved a valid single-item presentation / all incoming items.
    pub eventual_single_presentation: CountRate,
    /// Terminally routed items / all incoming items.
    pub end_to_end_routed: CountRate,
    /// Correctly routed items / all items with an independent reference destination.
    /// Held, rejected, manually reviewed and not-recovered items remain in the denominator.
    pub end_to_end_correct_route: Option<CountRate>,
    /// Correctly routed items / reference-labelled items that were routed.
    /// This is deliberately separate from the all-incoming route score.
    pub conditional_route_accuracy: Option<CountRate>,
    pub overlap_attempts: u64,
    pub double_pick_attempts: u64,
    pub occluded_or_folded_attempts: u64,
    pub out_of_field_attempts: u64,
    pub jammed_or_blocked_attempts: u64,
    pub unknown_presentation_attempts: u64,
    pub manual_review_items: u64,
    pub held_items: u64,
    pub rejected_items: u64,
    pub not_recovered_items: u64,
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .as_bytes()
            .iter()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(byte))
}

fn valid_evidence(evidence: &EvidenceRef) -> bool {
    nonempty(&evidence.evidence_id)
        && valid_sha256(&evidence.sha256)
        && nonempty(&evidence.method_ref)
}

impl FeedRun {
    pub fn validate(&self) -> ValidationReport {
        let mut findings = Vec::new();
        let mut add = |code: &str, item_id: Option<&str>, detail: &str| {
            findings.push(ValidationFinding {
                code: code.to_owned(),
                item_id: item_id.map(str::to_owned),
                detail: detail.to_owned(),
            });
        };

        for (field, value) in [
            ("run_id", self.run_id.as_str()),
            ("line_id", self.line_id.as_str()),
            ("configuration_revision", self.configuration_revision.as_str()),
        ] {
            if !nonempty(value) {
                add("EMPTY_RUN_IDENTITY", None, &format!("{field} must be non-empty"));
            }
        }
        if self.incoming_items.is_empty() {
            add("EMPTY_INCOMING_MANIFEST", None, "a feed run must contain at least one incoming item");
        }

        let mut item_by_id = BTreeMap::<&str, &IncomingItem>::new();
        let mut intake_sequences = BTreeSet::new();
        for item in &self.incoming_items {
            if !nonempty(&item.item_id) || !nonempty(&item.lot_id) {
                add("INVALID_INCOMING_ITEM", Some(&item.item_id), "item_id and lot_id must be non-empty");
            }
            if item_by_id.insert(&item.item_id, item).is_some() {
                add("DUPLICATE_INCOMING_ITEM", Some(&item.item_id), "incoming item identity appears more than once");
            }
            if !intake_sequences.insert(item.intake_sequence) {
                add("DUPLICATE_INTAKE_SEQUENCE", Some(&item.item_id), "intake sequence must be unique within a run");
            }
            if item.reference_destination.as_deref().is_some_and(|value| !nonempty(value)) {
                add("EMPTY_REFERENCE_DESTINATION", Some(&item.item_id), "reference destination, when present, must be non-empty");
            }
        }

        let mut attempts_by_item = BTreeMap::<&str, Vec<&PresentationAttempt>>::new();
        let mut attempt_ids = BTreeSet::new();
        let mut evidence_ids = BTreeSet::new();
        for attempt in &self.attempts {
            if !attempt_ids.insert(attempt.attempt_id.as_str()) {
                add("DUPLICATE_ATTEMPT_ID", Some(&attempt.item_id), "attempt identity appears more than once");
            }
            if !item_by_id.contains_key(attempt.item_id.as_str()) {
                add("ORPHAN_PRESENTATION_ATTEMPT", Some(&attempt.item_id), "attempt references an item absent from the incoming manifest");
            }
            if !nonempty(&attempt.attempt_id) || attempt.attempt_number == 0 {
                add("INVALID_ATTEMPT_IDENTITY", Some(&attempt.item_id), "attempt ID must be non-empty and attempt number must be one-based");
            }
            if attempt.started_at_ns >= attempt.completed_at_ns {
                add("INVALID_ATTEMPT_INTERVAL", Some(&attempt.item_id), "attempt finish must be later than attempt start");
            }
            if !valid_evidence(&attempt.evidence) {
                add("INVALID_ATTEMPT_EVIDENCE", Some(&attempt.item_id), "attempt evidence requires an ID, valid SHA-256 and method reference");
            }
            if !evidence_ids.insert(attempt.evidence.evidence_id.as_str()) {
                add("DUPLICATE_EVIDENCE_ID", Some(&attempt.item_id), "evidence identity is reused within the run");
            }
            attempts_by_item
                .entry(&attempt.item_id)
                .or_default()
                .push(attempt);
        }

        for (item_id, attempts) in attempts_by_item.iter_mut() {
            attempts.sort_by_key(|attempt| attempt.attempt_number);
            for (index, attempt) in attempts.iter().enumerate() {
                let expected_number = u32::try_from(index + 1).unwrap_or(u32::MAX);
                if attempt.attempt_number != expected_number {
                    add("ATTEMPT_NUMBER_GAP", Some(item_id), "attempt numbers must be contiguous from one");
                }
                let expected_predecessor = if index == 0 {
                    None
                } else {
                    Some(attempts[index - 1].attempt_id.as_str())
                };
                if attempt.predecessor_attempt_id.as_deref() != expected_predecessor {
                    add("ATTEMPT_CHAIN_MISMATCH", Some(item_id), "attempt must link to the immediately preceding attempt for this item");
                }
                if index > 0 && attempt.started_at_ns < attempts[index - 1].completed_at_ns {
                    add("ATTEMPT_TIME_OVERLAP", Some(item_id), "a retry cannot begin before the previous attempt completes");
                }
            }
        }

        let mut terminal_by_item = BTreeMap::<&str, &TerminalDisposition>::new();
        for terminal in &self.terminal_dispositions {
            if terminal_by_item.insert(&terminal.item_id, terminal).is_some() {
                add("DUPLICATE_TERMINAL_DISPOSITION", Some(&terminal.item_id), "each incoming item may have exactly one terminal disposition");
            }
            if !item_by_id.contains_key(terminal.item_id.as_str()) {
                add("ORPHAN_TERMINAL_DISPOSITION", Some(&terminal.item_id), "terminal disposition references an item absent from the incoming manifest");
            }
            match (terminal.outcome, terminal.actual_destination.as_deref()) {
                (TerminalOutcome::Routed, Some(destination)) if nonempty(destination) => {}
                (TerminalOutcome::Routed, _) => {
                    add("ROUTED_WITHOUT_DESTINATION", Some(&terminal.item_id), "routed outcome requires an actual destination");
                }
                (_, Some(_)) => {
                    add("NON_ROUTED_WITH_DESTINATION", Some(&terminal.item_id), "only a routed item may declare an actual destination");
                }
                (_, None) => {}
            }
            if !valid_evidence(&terminal.evidence) {
                add("INVALID_TERMINAL_EVIDENCE", Some(&terminal.item_id), "terminal disposition requires an ID, valid SHA-256 and method reference");
            }
            if !evidence_ids.insert(terminal.evidence.evidence_id.as_str()) {
                add("DUPLICATE_EVIDENCE_ID", Some(&terminal.item_id), "evidence identity is reused within the run");
            }
            if let Some(attempts) = attempts_by_item.get(terminal.item_id.as_str()) {
                if let Some(last_attempt) = attempts.iter().max_by_key(|attempt| attempt.attempt_number)
                    && terminal.completed_at_ns < last_attempt.completed_at_ns
                {
                    add("TERMINAL_PRECEDES_FINAL_ATTEMPT", Some(&terminal.item_id), "terminal disposition cannot precede the item's final presentation attempt");
                }
            }
        }

        for item in &self.incoming_items {
            if !terminal_by_item.contains_key(item.item_id.as_str()) {
                add("MISSING_TERMINAL_DISPOSITION", Some(&item.item_id), "every incoming item must end in exactly one terminal disposition");
            }
        }

        findings.sort_by(|left, right| {
            (&left.code, &left.item_id, &left.detail).cmp(&(&right.code, &right.item_id, &right.detail))
        });
        ValidationReport {
            valid: findings.is_empty(),
            findings,
        }
    }

    pub fn metrics(&self) -> Result<FeedMetrics, ValidationReport> {
        let report = self.validate();
        if !report.valid {
            return Err(report);
        }

        let terminal_by_item: BTreeMap<&str, &TerminalDisposition> = self
            .terminal_dispositions
            .iter()
            .map(|terminal| (terminal.item_id.as_str(), terminal))
            .collect();
        let attempts_by_item: BTreeMap<&str, Vec<&PresentationAttempt>> = {
            let mut map = BTreeMap::<&str, Vec<&PresentationAttempt>>::new();
            for attempt in &self.attempts {
                map.entry(&attempt.item_id).or_default().push(attempt);
            }
            for attempts in map.values_mut() {
                attempts.sort_by_key(|attempt| attempt.attempt_number);
            }
            map
        };

        let total = self.incoming_items.len() as u64;
        let mut first_pass_single = 0_u64;
        let mut eventual_single = 0_u64;
        let mut overlap = 0_u64;
        let mut double_pick = 0_u64;
        let mut occluded = 0_u64;
        let mut out_of_field = 0_u64;
        let mut jammed = 0_u64;
        let mut unknown = 0_u64;
        let mut routed = 0_u64;
        let mut manual_review = 0_u64;
        let mut held = 0_u64;
        let mut rejected = 0_u64;
        let mut not_recovered = 0_u64;
        let mut reference_items = 0_u64;
        let mut reference_routed = 0_u64;
        let mut correct_routes = 0_u64;

        for item in &self.incoming_items {
            let attempts = attempts_by_item.get(item.item_id.as_str());
            if let Some(attempts) = attempts {
                if attempts.first().is_some_and(|attempt| attempt.state.is_single()) {
                    first_pass_single += 1;
                }
                if attempts.iter().any(|attempt| attempt.state.is_single()) {
                    eventual_single += 1;
                }
                for attempt in attempts {
                    match attempt.state {
                        PresentationState::SingleItemInFieldOfView => {}
                        PresentationState::Overlap => overlap += 1,
                        PresentationState::DoublePick => double_pick += 1,
                        PresentationState::OccludedOrFolded => occluded += 1,
                        PresentationState::OutOfField => out_of_field += 1,
                        PresentationState::JammedOrBlocked => jammed += 1,
                        PresentationState::Unknown => unknown += 1,
                    }
                }
            }
            let terminal = terminal_by_item[item.item_id.as_str()];
            match terminal.outcome {
                TerminalOutcome::Routed => routed += 1,
                TerminalOutcome::ManualReview => manual_review += 1,
                TerminalOutcome::Held => held += 1,
                TerminalOutcome::Rejected => rejected += 1,
                TerminalOutcome::NotRecovered => not_recovered += 1,
            }
            if let Some(reference) = item.reference_destination.as_deref() {
                reference_items += 1;
                if terminal.outcome == TerminalOutcome::Routed {
                    reference_routed += 1;
                    if terminal.actual_destination.as_deref() == Some(reference) {
                        correct_routes += 1;
                    }
                }
            }
        }

        let optional_rate = |numerator, denominator| {
            (denominator > 0).then_some(CountRate { numerator, denominator })
        };

        Ok(FeedMetrics {
            incoming_items: total,
            presentation_attempts: self.attempts.len() as u64,
            first_pass_single_presentation: CountRate { numerator: first_pass_single, denominator: total },
            eventual_single_presentation: CountRate { numerator: eventual_single, denominator: total },
            end_to_end_routed: CountRate { numerator: routed, denominator: total },
            end_to_end_correct_route: optional_rate(correct_routes, reference_items),
            conditional_route_accuracy: optional_rate(correct_routes, reference_routed),
            overlap_attempts: overlap,
            double_pick_attempts: double_pick,
            occluded_or_folded_attempts: occluded,
            out_of_field_attempts: out_of_field,
            jammed_or_blocked_attempts: jammed,
            unknown_presentation_attempts: unknown,
            manual_review_items: manual_review,
            held_items: held,
            rejected_items: rejected,
            not_recovered_items: not_recovered,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn evidence(id: &str, byte: char) -> EvidenceRef {
        EvidenceRef {
            evidence_id: id.to_owned(),
            sha256: byte.to_string().repeat(64),
            method_ref: "feed-observation-v1".to_owned(),
        }
    }

    fn item(id: &str, sequence: u64, reference: &str) -> IncomingItem {
        IncomingItem {
            item_id: id.to_owned(),
            lot_id: "LOT-01".to_owned(),
            intake_sequence: sequence,
            reference_destination: Some(reference.to_owned()),
        }
    }

    fn attempt(
        id: &str,
        item_id: &str,
        number: u32,
        predecessor: Option<&str>,
        state: PresentationState,
        start: u64,
        finish: u64,
        digest_char: char,
    ) -> PresentationAttempt {
        PresentationAttempt {
            attempt_id: id.to_owned(),
            item_id: item_id.to_owned(),
            attempt_number: number,
            predecessor_attempt_id: predecessor.map(str::to_owned),
            state,
            started_at_ns: start,
            completed_at_ns: finish,
            evidence: evidence(&format!("EV-{id}"), digest_char),
        }
    }

    fn terminal(
        item_id: &str,
        outcome: TerminalOutcome,
        destination: Option<&str>,
        at: u64,
        digest_char: char,
    ) -> TerminalDisposition {
        TerminalDisposition {
            item_id: item_id.to_owned(),
            outcome,
            actual_destination: destination.map(str::to_owned),
            completed_at_ns: at,
            evidence: evidence(&format!("TERM-{item_id}"), digest_char),
        }
    }

    fn valid_run() -> FeedRun {
        FeedRun {
            run_id: "RUN-01".to_owned(),
            line_id: "LINE-01".to_owned(),
            configuration_revision: "FEEDER-R3".to_owned(),
            incoming_items: vec![
                item("ITEM-01", 1, "cotton"),
                item("ITEM-02", 2, "polyester"),
                item("ITEM-03", 3, "cotton"),
                item("ITEM-04", 4, "wool"),
            ],
            attempts: vec![
                attempt("A-01", "ITEM-01", 1, None, PresentationState::SingleItemInFieldOfView, 10, 20, 'a'),
                attempt("A-02", "ITEM-02", 1, None, PresentationState::Overlap, 11, 21, 'b'),
                attempt("A-03", "ITEM-02", 2, Some("A-02"), PresentationState::SingleItemInFieldOfView, 22, 32, 'c'),
                attempt("A-04", "ITEM-03", 1, None, PresentationState::DoublePick, 12, 22, 'd'),
            ],
            terminal_dispositions: vec![
                terminal("ITEM-01", TerminalOutcome::Routed, Some("cotton"), 25, 'e'),
                terminal("ITEM-02", TerminalOutcome::Routed, Some("polyester"), 35, 'f'),
                terminal("ITEM-03", TerminalOutcome::ManualReview, None, 28, '1'),
                terminal("ITEM-04", TerminalOutcome::Held, None, 15, '2'),
            ],
        }
    }

    #[test]
    fn complete_run_accounts_for_every_item_and_keeps_rate_denominators() {
        let run = valid_run();
        assert!(run.validate().valid, "{:#?}", run.validate().findings);
        let metrics = run.metrics().unwrap();
        assert_eq!(metrics.incoming_items, 4);
        assert_eq!(metrics.first_pass_single_presentation, CountRate { numerator: 1, denominator: 4 });
        assert_eq!(metrics.eventual_single_presentation, CountRate { numerator: 2, denominator: 4 });
        assert_eq!(metrics.end_to_end_routed, CountRate { numerator: 2, denominator: 4 });
        assert_eq!(metrics.end_to_end_correct_route, Some(CountRate { numerator: 2, denominator: 4 }));
        assert_eq!(metrics.conditional_route_accuracy, Some(CountRate { numerator: 2, denominator: 2 }));
        assert_eq!(metrics.overlap_attempts, 1);
        assert_eq!(metrics.double_pick_attempts, 1);
        assert_eq!(metrics.manual_review_items, 1);
        assert_eq!(metrics.held_items, 1);
    }

    #[test]
    fn missing_terminal_item_is_not_dropped_from_denominator() {
        let mut run = valid_run();
        run.terminal_dispositions.pop();
        let report = run.validate();
        assert!(!report.valid);
        assert!(report.findings.iter().any(|finding| finding.code == "MISSING_TERMINAL_DISPOSITION"));
        assert!(run.metrics().is_err());
    }

    #[test]
    fn duplicate_or_orphan_terminal_dispositions_are_rejected() {
        let mut run = valid_run();
        run.terminal_dispositions.push(run.terminal_dispositions[0].clone());
        assert!(run.validate().findings.iter().any(|finding| finding.code == "DUPLICATE_TERMINAL_DISPOSITION"));

        let mut run = valid_run();
        run.terminal_dispositions.push(terminal("NOT-IN-MANIFEST", TerminalOutcome::Held, None, 40, '3'));
        assert!(run.validate().findings.iter().any(|finding| finding.code == "ORPHAN_TERMINAL_DISPOSITION"));
    }

    #[test]
    fn attempts_must_reference_incoming_items_and_have_unique_ids() {
        let mut run = valid_run();
        run.attempts.push(attempt("A-99", "MISSING-ITEM", 1, None, PresentationState::Unknown, 1, 2, '9'));
        assert!(run.validate().findings.iter().any(|finding| finding.code == "ORPHAN_PRESENTATION_ATTEMPT"));

        let mut run = valid_run();
        run.attempts.push(run.attempts[0].clone());
        assert!(run.validate().findings.iter().any(|finding| finding.code == "DUPLICATE_ATTEMPT_ID"));
    }

    #[test]
    fn retries_must_have_contiguous_numbers_and_exact_predecessor_links() {
        let mut run = valid_run();
        run.attempts[2].attempt_number = 3;
        assert!(run.validate().findings.iter().any(|finding| finding.code == "ATTEMPT_NUMBER_GAP"));

        let mut run = valid_run();
        run.attempts[2].predecessor_attempt_id = Some("A-01".to_owned());
        assert!(run.validate().findings.iter().any(|finding| finding.code == "ATTEMPT_CHAIN_MISMATCH"));
    }

    #[test]
    fn retries_cannot_overlap_and_terminal_time_cannot_precede_last_attempt() {
        let mut run = valid_run();
        run.attempts[2].started_at_ns = 18;
        assert!(run.validate().findings.iter().any(|finding| finding.code == "ATTEMPT_TIME_OVERLAP"));

        let mut run = valid_run();
        run.terminal_dispositions[1].completed_at_ns = 30;
        assert!(run.validate().findings.iter().any(|finding| finding.code == "TERMINAL_PRECEDES_FINAL_ATTEMPT"));
    }

    #[test]
    fn routed_outcome_requires_actual_destination_and_other_outcomes_forbid_one() {
        let mut run = valid_run();
        run.terminal_dispositions[0].actual_destination = None;
        assert!(run.validate().findings.iter().any(|finding| finding.code == "ROUTED_WITHOUT_DESTINATION"));

        let mut run = valid_run();
        run.terminal_dispositions[2].actual_destination = Some("cotton".to_owned());
        assert!(run.validate().findings.iter().any(|finding| finding.code == "NON_ROUTED_WITH_DESTINATION"));
    }

    #[test]
    fn evidence_ids_and_sha256_references_must_be_valid_and_unique() {
        let mut run = valid_run();
        run.attempts[0].evidence.sha256 = "not-a-digest".to_owned();
        assert!(run.validate().findings.iter().any(|finding| finding.code == "INVALID_ATTEMPT_EVIDENCE"));

        let mut run = valid_run();
        run.terminal_dispositions[0].evidence.evidence_id = run.attempts[0].evidence.evidence_id.clone();
        assert!(run.validate().findings.iter().any(|finding| finding.code == "DUPLICATE_EVIDENCE_ID"));
    }

    #[test]
    fn duplicate_incoming_ids_and_intake_sequences_are_rejected() {
        let mut run = valid_run();
        run.incoming_items.push(run.incoming_items[0].clone());
        assert!(run.validate().findings.iter().any(|finding| finding.code == "DUPLICATE_INCOMING_ITEM"));

        let mut run = valid_run();
        run.incoming_items[1].intake_sequence = run.incoming_items[0].intake_sequence;
        assert!(run.validate().findings.iter().any(|finding| finding.code == "DUPLICATE_INTAKE_SEQUENCE"));
    }

    #[test]
    fn actual_route_errors_are_not_hidden_by_conditional_accuracy() {
        let mut run = valid_run();
        run.terminal_dispositions[1].actual_destination = Some("cotton".to_owned());
        let metrics = run.metrics().unwrap();
        assert_eq!(metrics.end_to_end_correct_route, Some(CountRate { numerator: 1, denominator: 4 }));
        assert_eq!(metrics.conditional_route_accuracy, Some(CountRate { numerator: 1, denominator: 2 }));
    }

    #[test]
    fn every_presentation_failure_remains_counted() {
        let mut run = valid_run();
        run.attempts.push(attempt("A-05", "ITEM-04", 1, None, PresentationState::OutOfField, 1, 2, '7'));
        run.terminal_dispositions[3].completed_at_ns = 3;
        let metrics = run.metrics().unwrap();
        assert_eq!(metrics.out_of_field_attempts, 1);
        assert_eq!(metrics.end_to_end_routed, CountRate { numerator: 2, denominator: 4 });
    }
}
