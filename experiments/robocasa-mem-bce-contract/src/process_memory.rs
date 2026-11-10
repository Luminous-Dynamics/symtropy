//! Synthetic process-memory evaluation oracle.
//!
//! This module tests a narrow benchmark contract: switching between task
//! threads must not erase unfinished obligations. It is evaluator-side
//! bookkeeping for synthetic fixtures, not a production memory store, task
//! planner, controller, or authority system.

use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObligationEventKind {
    Opened,
    Paused,
    Resumed,
    Satisfied,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObligationEvent {
    pub event_id: String,
    pub sequence: u64,
    pub task_id: String,
    pub obligation_id: String,
    pub kind: ObligationEventKind,
}

/// Evaluator-declared process obligation expected to appear in the episode's
/// interaction history. This manifest is evaluator-private; it must not be
/// exposed to the acting policy as a hidden completion target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpectedObligation {
    pub task_id: String,
    pub obligation_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObligationQualification {
    pub expected_count: usize,
    pub satisfied_count: usize,
    pub pending_obligation_ids: Vec<String>,
    pub may_submit: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObligationState {
    Active,
    Paused,
    Satisfied,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObligationRecord {
    pub task_id: String,
    pub obligation_id: String,
    pub state: ObligationState,
    pub opened_at: u64,
    pub last_event_at: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessMemoryError {
    EmptyIdentity,
    DuplicateEventId(String),
    DuplicateSequence(u64),
    NonMonotonicSequence {
        previous: u64,
        received: u64,
    },
    DuplicateObligation(String),
    UnknownObligation(String),
    TaskMismatch {
        obligation_id: String,
        expected_task: String,
        received_task: String,
    },
    InvalidTransition {
        obligation_id: String,
        state: ObligationState,
        event: ObligationEventKind,
    },
    DuplicateExpectedObligation(String),
    EmptyExpectedManifest,
    MissingDeclaredObligation(String),
    UndeclaredObligation(String),
    DeclaredTaskMismatch {
        obligation_id: String,
        expected_task: String,
        observed_task: String,
    },
}

/// Fold a chronological event stream into current obligation state. Task
/// switching is represented by interleaved events from different task IDs.
/// An unfinished obligation remains present until a matching Satisfied event.
pub fn fold_obligations(
    events: &[ObligationEvent],
) -> Result<BTreeMap<String, ObligationRecord>, ProcessMemoryError> {
    let mut records = BTreeMap::<String, ObligationRecord>::new();
    let mut seen_events = std::collections::BTreeSet::new();
    let mut seen_sequences = std::collections::BTreeSet::new();
    let mut previous_sequence = None;

    for event in events {
        if event.event_id.trim().is_empty()
            || event.task_id.trim().is_empty()
            || event.obligation_id.trim().is_empty()
        {
            return Err(ProcessMemoryError::EmptyIdentity);
        }
        if !seen_events.insert(event.event_id.as_str()) {
            return Err(ProcessMemoryError::DuplicateEventId(event.event_id.clone()));
        }
        if !seen_sequences.insert(event.sequence) {
            return Err(ProcessMemoryError::DuplicateSequence(event.sequence));
        }
        if let Some(previous) = previous_sequence {
            if event.sequence <= previous {
                return Err(ProcessMemoryError::NonMonotonicSequence {
                    previous,
                    received: event.sequence,
                });
            }
        }
        previous_sequence = Some(event.sequence);

        match event.kind {
            ObligationEventKind::Opened => {
                if records.contains_key(&event.obligation_id) {
                    return Err(ProcessMemoryError::DuplicateObligation(
                        event.obligation_id.clone(),
                    ));
                }
                records.insert(
                    event.obligation_id.clone(),
                    ObligationRecord {
                        task_id: event.task_id.clone(),
                        obligation_id: event.obligation_id.clone(),
                        state: ObligationState::Active,
                        opened_at: event.sequence,
                        last_event_at: event.sequence,
                    },
                );
            }
            transition => {
                let record = records.get_mut(&event.obligation_id).ok_or_else(|| {
                    ProcessMemoryError::UnknownObligation(event.obligation_id.clone())
                })?;
                if record.task_id != event.task_id {
                    return Err(ProcessMemoryError::TaskMismatch {
                        obligation_id: event.obligation_id.clone(),
                        expected_task: record.task_id.clone(),
                        received_task: event.task_id.clone(),
                    });
                }

                let next_state = match (record.state, transition) {
                    (ObligationState::Active, ObligationEventKind::Paused) => {
                        ObligationState::Paused
                    }
                    (ObligationState::Paused, ObligationEventKind::Resumed) => {
                        ObligationState::Active
                    }
                    (ObligationState::Active, ObligationEventKind::Satisfied) => {
                        ObligationState::Satisfied
                    }
                    (state, kind) => {
                        return Err(ProcessMemoryError::InvalidTransition {
                            obligation_id: record.obligation_id.clone(),
                            state,
                            event: kind,
                        });
                    }
                };

                record.state = next_state;
                record.last_event_at = event.sequence;
            }
        }
    }

    Ok(records)
}

/// A task may be submitted only when every *observed* obligation is
/// satisfied. This is a low-level state predicate, not benchmark completion:
/// an empty map passes vacuously. Use `qualify_against_manifest` whenever the
/// evaluator has a declared set of expected process obligations.
pub fn can_submit(records: &BTreeMap<String, ObligationRecord>) -> bool {
    records
        .values()
        .all(|record| record.state == ObligationState::Satisfied)
}

/// Qualify the observed process history against an evaluator-private,
/// preregistered obligation manifest. Every expected obligation must have an
/// opened record, every observed obligation must be declared, task ownership
/// must match, and every expected obligation must be satisfied before
/// `may_submit` is true. Missing obligations are never treated as completed.
pub fn qualify_against_manifest(
    expected: &[ExpectedObligation],
    events: &[ObligationEvent],
) -> Result<ObligationQualification, ProcessMemoryError> {
    if expected.is_empty() {
        return Err(ProcessMemoryError::EmptyExpectedManifest);
    }

    let mut manifest = BTreeMap::<String, String>::new();
    for item in expected {
        if item.task_id.trim().is_empty() || item.obligation_id.trim().is_empty() {
            return Err(ProcessMemoryError::EmptyIdentity);
        }
        if manifest
            .insert(item.obligation_id.clone(), item.task_id.clone())
            .is_some()
        {
            return Err(ProcessMemoryError::DuplicateExpectedObligation(
                item.obligation_id.clone(),
            ));
        }
    }

    let records = fold_obligations(events)?;

    for record in records.values() {
        match manifest.get(&record.obligation_id) {
            None => {
                return Err(ProcessMemoryError::UndeclaredObligation(
                    record.obligation_id.clone(),
                ));
            }
            Some(expected_task) if expected_task != &record.task_id => {
                return Err(ProcessMemoryError::DeclaredTaskMismatch {
                    obligation_id: record.obligation_id.clone(),
                    expected_task: expected_task.clone(),
                    observed_task: record.task_id.clone(),
                });
            }
            Some(_) => {}
        }
    }

    let mut satisfied_count = 0;
    let mut pending_obligation_ids = Vec::new();
    for obligation_id in manifest.keys() {
        let Some(record) = records.get(obligation_id) else {
            return Err(ProcessMemoryError::MissingDeclaredObligation(
                obligation_id.clone(),
            ));
        };
        if record.state == ObligationState::Satisfied {
            satisfied_count += 1;
        } else {
            pending_obligation_ids.push(obligation_id.clone());
        }
    }

    let may_submit = pending_obligation_ids.is_empty();
    Ok(ObligationQualification {
        expected_count: manifest.len(),
        satisfied_count,
        pending_obligation_ids,
        may_submit,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(
        id: &str,
        sequence: u64,
        task: &str,
        obligation: &str,
        kind: ObligationEventKind,
    ) -> ObligationEvent {
        ObligationEvent {
            event_id: id.into(),
            sequence,
            task_id: task.into(),
            obligation_id: obligation.into(),
            kind,
        }
    }

    #[test]
    fn empty_expected_manifest_cannot_vacuously_qualify_submission() {
        assert_eq!(
            qualify_against_manifest(&[], &[]),
            Err(ProcessMemoryError::EmptyExpectedManifest)
        );
    }

    #[test]
    fn empty_history_cannot_satisfy_a_nonempty_expected_manifest() {
        let expected = vec![ExpectedObligation {
            task_id: "heat".into(),
            obligation_id: "turn-off-stove".into(),
        }];
        assert_eq!(
            qualify_against_manifest(&expected, &[]),
            Err(ProcessMemoryError::MissingDeclaredObligation(
                "turn-off-stove".into()
            ))
        );
    }

    #[test]
    fn missing_one_declared_obligation_blocks_even_when_observed_ones_are_satisfied() {
        let expected = vec![
            ExpectedObligation {
                task_id: "heat".into(),
                obligation_id: "turn-off-stove".into(),
            },
            ExpectedObligation {
                task_id: "fill".into(),
                obligation_id: "turn-off-faucet".into(),
            },
        ];
        let events = vec![
            event(
                "stove-on",
                1,
                "heat",
                "turn-off-stove",
                ObligationEventKind::Opened,
            ),
            event(
                "stove-off",
                2,
                "heat",
                "turn-off-stove",
                ObligationEventKind::Satisfied,
            ),
        ];
        assert_eq!(
            qualify_against_manifest(&expected, &events),
            Err(ProcessMemoryError::MissingDeclaredObligation(
                "turn-off-faucet".into()
            ))
        );
    }

    #[test]
    fn manifest_coverage_and_completion_are_both_required_for_submission() {
        let expected = vec![
            ExpectedObligation {
                task_id: "heat".into(),
                obligation_id: "turn-off-stove".into(),
            },
            ExpectedObligation {
                task_id: "inspect".into(),
                obligation_id: "inspect-drawer".into(),
            },
        ];
        let events = vec![
            event(
                "stove-on",
                1,
                "heat",
                "turn-off-stove",
                ObligationEventKind::Opened,
            ),
            event(
                "drawer-open",
                2,
                "inspect",
                "inspect-drawer",
                ObligationEventKind::Opened,
            ),
            event(
                "drawer-done",
                3,
                "inspect",
                "inspect-drawer",
                ObligationEventKind::Satisfied,
            ),
        ];
        let qualification = qualify_against_manifest(&expected, &events).unwrap();
        assert_eq!(qualification.expected_count, 2);
        assert_eq!(qualification.satisfied_count, 1);
        assert_eq!(
            qualification.pending_obligation_ids,
            vec!["turn-off-stove".to_string()]
        );
        assert!(!qualification.may_submit);
    }

    #[test]
    fn all_declared_obligations_satisfied_after_task_switch_can_submit() {
        let expected = vec![
            ExpectedObligation {
                task_id: "heat".into(),
                obligation_id: "turn-off-stove".into(),
            },
            ExpectedObligation {
                task_id: "inspect".into(),
                obligation_id: "inspect-drawer".into(),
            },
        ];
        let events = vec![
            event(
                "stove-on",
                1,
                "heat",
                "turn-off-stove",
                ObligationEventKind::Opened,
            ),
            event(
                "pause-heat",
                2,
                "heat",
                "turn-off-stove",
                ObligationEventKind::Paused,
            ),
            event(
                "drawer-open",
                3,
                "inspect",
                "inspect-drawer",
                ObligationEventKind::Opened,
            ),
            event(
                "drawer-done",
                4,
                "inspect",
                "inspect-drawer",
                ObligationEventKind::Satisfied,
            ),
            event(
                "resume-heat",
                5,
                "heat",
                "turn-off-stove",
                ObligationEventKind::Resumed,
            ),
            event(
                "stove-off",
                6,
                "heat",
                "turn-off-stove",
                ObligationEventKind::Satisfied,
            ),
        ];
        let qualification = qualify_against_manifest(&expected, &events).unwrap();
        assert_eq!(qualification.expected_count, 2);
        assert_eq!(qualification.satisfied_count, 2);
        assert!(qualification.pending_obligation_ids.is_empty());
        assert!(qualification.may_submit);
    }

    #[test]
    fn undeclared_and_wrong_task_obligations_reject() {
        let expected = vec![ExpectedObligation {
            task_id: "heat".into(),
            obligation_id: "turn-off-stove".into(),
        }];
        let undeclared = vec![event(
            "drawer-open",
            1,
            "inspect",
            "inspect-drawer",
            ObligationEventKind::Opened,
        )];
        assert_eq!(
            qualify_against_manifest(&expected, &undeclared),
            Err(ProcessMemoryError::UndeclaredObligation(
                "inspect-drawer".into()
            ))
        );

        let wrong_task = vec![event(
            "stove-on",
            1,
            "inspect",
            "turn-off-stove",
            ObligationEventKind::Opened,
        )];
        assert_eq!(
            qualify_against_manifest(&expected, &wrong_task),
            Err(ProcessMemoryError::DeclaredTaskMismatch {
                obligation_id: "turn-off-stove".into(),
                expected_task: "heat".into(),
                observed_task: "inspect".into(),
            })
        );
    }

    #[test]
    fn duplicate_expected_obligations_reject() {
        let expected = vec![
            ExpectedObligation {
                task_id: "heat".into(),
                obligation_id: "turn-off-stove".into(),
            },
            ExpectedObligation {
                task_id: "heat".into(),
                obligation_id: "turn-off-stove".into(),
            },
        ];
        assert_eq!(
            qualify_against_manifest(&expected, &[]),
            Err(ProcessMemoryError::DuplicateExpectedObligation(
                "turn-off-stove".into()
            ))
        );
    }

    #[test]
    fn task_switch_preserves_first_tasks_pending_obligation() {
        let events = vec![
            event(
                "stove-on",
                1,
                "heat",
                "turn-off-stove",
                ObligationEventKind::Opened,
            ),
            event(
                "inspect-drawer",
                2,
                "inspect",
                "inspect-drawer-1",
                ObligationEventKind::Opened,
            ),
            event(
                "drawer-done",
                3,
                "inspect",
                "inspect-drawer-1",
                ObligationEventKind::Satisfied,
            ),
        ];

        let state = fold_obligations(&events).unwrap();
        assert_eq!(state["turn-off-stove"].state, ObligationState::Active);
        assert!(!can_submit(&state));
    }

    #[test]
    fn paused_task_thread_can_be_resumed_after_another_thread_finishes() {
        let events = vec![
            event("stove-on", 1, "heat", "turn-off-stove", ObligationEventKind::Opened),
            event(
                "pause-heat",
                2,
                "heat",
                "turn-off-stove",
                ObligationEventKind::Paused,
            ),
            event(
                "drawer-open",
                3,
                "inspect",
                "inspect-drawer",
                ObligationEventKind::Opened,
            ),
            event(
                "drawer-done",
                4,
                "inspect",
                "inspect-drawer",
                ObligationEventKind::Satisfied,
            ),
            event(
                "resume-heat",
                5,
                "heat",
                "turn-off-stove",
                ObligationEventKind::Resumed,
            ),
            event(
                "stove-off",
                6,
                "heat",
                "turn-off-stove",
                ObligationEventKind::Satisfied,
            ),
        ];

        let state = fold_obligations(&events).unwrap();
        assert_eq!(state["turn-off-stove"].state, ObligationState::Satisfied);
        assert_eq!(state["inspect-drawer"].state, ObligationState::Satisfied);
        assert!(can_submit(&state));
    }

    #[test]
    fn unresolved_obligation_blocks_submission() {
        let events = vec![event(
            "faucet-on",
            1,
            "fill",
            "turn-off-faucet",
            ObligationEventKind::Opened,
        )];

        let state = fold_obligations(&events).unwrap();
        assert!(!can_submit(&state));
    }

    #[test]
    fn unknown_obligation_cannot_be_satisfied() {
        let events = vec![event(
            "stove-off",
            1,
            "heat",
            "turn-off-stove",
            ObligationEventKind::Satisfied,
        )];

        assert_eq!(
            fold_obligations(&events),
            Err(ProcessMemoryError::UnknownObligation(
                "turn-off-stove".into()
            ))
        );
    }

    #[test]
    fn one_task_cannot_satisfy_another_tasks_obligation() {
        let events = vec![
            event(
                "faucet-on",
                1,
                "fill",
                "turn-off-faucet",
                ObligationEventKind::Opened,
            ),
            event(
                "wrong-task-off",
                2,
                "retrieve",
                "turn-off-faucet",
                ObligationEventKind::Satisfied,
            ),
        ];

        assert!(matches!(
            fold_obligations(&events),
            Err(ProcessMemoryError::TaskMismatch { .. })
        ));
    }

    #[test]
    fn duplicate_open_does_not_create_a_second_obligation() {
        let events = vec![
            event("stove-on", 1, "heat", "turn-off-stove", ObligationEventKind::Opened),
            event(
                "stove-on-replay",
                2,
                "heat",
                "turn-off-stove",
                ObligationEventKind::Opened,
            ),
        ];

        assert_eq!(
            fold_obligations(&events),
            Err(ProcessMemoryError::DuplicateObligation(
                "turn-off-stove".into()
            ))
        );
    }

    #[test]
    fn completed_obligation_cannot_be_reopened_by_late_resume() {
        let events = vec![
            event("stove-on", 1, "heat", "turn-off-stove", ObligationEventKind::Opened),
            event(
                "stove-off",
                2,
                "heat",
                "turn-off-stove",
                ObligationEventKind::Satisfied,
            ),
            event(
                "late-resume",
                3,
                "heat",
                "turn-off-stove",
                ObligationEventKind::Resumed,
            ),
        ];

        assert!(matches!(
            fold_obligations(&events),
            Err(ProcessMemoryError::InvalidTransition { .. })
        ));
    }

    #[test]
    fn duplicate_and_reordered_events_fail_closed() {
        let duplicate = vec![
            event("e1", 1, "heat", "stove", ObligationEventKind::Opened),
            event("e1", 2, "heat", "stove", ObligationEventKind::Satisfied),
        ];
        assert!(matches!(
            fold_obligations(&duplicate),
            Err(ProcessMemoryError::DuplicateEventId(_))
        ));

        let reordered = vec![
            event("e1", 2, "heat", "stove", ObligationEventKind::Opened),
            event("e2", 1, "heat", "stove", ObligationEventKind::Satisfied),
        ];
        assert!(matches!(
            fold_obligations(&reordered),
            Err(ProcessMemoryError::NonMonotonicSequence { .. })
        ));
    }
}
