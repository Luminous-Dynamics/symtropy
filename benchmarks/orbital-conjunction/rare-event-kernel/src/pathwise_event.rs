// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Deterministic event semantics for joint, pathwise conjunction horizons.
//!
//! A single complete trajectory realization is classified once against every
//! declared encounter window. Window outcomes are combined by logical union;
//! their marginal probabilities are never added or treated as independent.
//!
//! This module does not propagate an orbit, generate trajectory samples,
//! prove that the windows cover the requested horizon, or estimate a
//! probability. It is a fail-closed semantic reducer over caller-supplied
//! window classifications.

use crate::KernelError;

/// One identified close-approach window inside a declared horizon.
#[derive(Debug, Clone, PartialEq)]
pub struct EncounterWindow {
    /// Stable identifier unique within the event definition.
    pub window_id: String,
    /// Inclusive start time in seconds relative to the declared epoch.
    pub start_seconds: f64,
    /// Inclusive end time in seconds relative to the declared epoch.
    pub end_seconds: f64,
}

/// Immutable-in-use definition of a multi-window conjunction event.
///
/// Windows may overlap, touch, be nested, or have identical bounds. The
/// reducer uses their set-union semantics and never assumes independence.
#[derive(Debug, Clone, PartialEq)]
pub struct HorizonEventDefinition {
    event_id: String,
    revision: String,
    epoch: String,
    time_scale: String,
    reference_frame: String,
    horizon_start_seconds: f64,
    horizon_end_seconds: f64,
    hard_body_radius_m: f64,
    window_inventory_coverage: WindowInventoryCoverage,
    windows: Vec<EncounterWindow>,
}

impl HorizonEventDefinition {
    /// Construct and validate a versioned horizon event definition.
    ///
    /// Times are seconds relative to the declared epoch, expressed in the
    /// declared time scale and reference frame. Window bounds must lie inside
    /// the horizon. The window list is stored in deterministic chronological
    /// order.
    pub fn new(
        event_id: impl Into<String>,
        revision: impl Into<String>,
        epoch: impl Into<String>,
        time_scale: impl Into<String>,
        reference_frame: impl Into<String>,
        horizon_start_seconds: f64,
        horizon_end_seconds: f64,
        hard_body_radius_m: f64,
        window_inventory_coverage: WindowInventoryCoverage,
        mut windows: Vec<EncounterWindow>,
    ) -> Result<Self, KernelError> {
        let event_id = event_id.into();
        let revision = revision.into();
        let epoch = epoch.into();
        let time_scale = time_scale.into();
        let reference_frame = reference_frame.into();

        if event_id.trim().is_empty()
            || revision.trim().is_empty()
            || epoch.trim().is_empty()
            || time_scale.trim().is_empty()
            || reference_frame.trim().is_empty()
        {
            return Err(KernelError::InvalidEventDefinition);
        }
        if !horizon_start_seconds.is_finite()
            || !horizon_end_seconds.is_finite()
            || horizon_start_seconds > horizon_end_seconds
        {
            return Err(KernelError::InvalidEventDefinition);
        }
        if !hard_body_radius_m.is_finite() || hard_body_radius_m <= 0.0 {
            return Err(KernelError::InvalidHardBodyRadius);
        }
        if windows.is_empty() {
            return Err(KernelError::EmptyEncounterWindowSet);
        }

        for window in &windows {
            if window.window_id.trim().is_empty()
                || !window.start_seconds.is_finite()
                || !window.end_seconds.is_finite()
                || window.start_seconds > window.end_seconds
                || window.start_seconds < horizon_start_seconds
                || window.end_seconds > horizon_end_seconds
            {
                return Err(KernelError::InvalidEncounterWindow);
            }
        }

        let mut identifiers = windows
            .iter()
            .map(|window| window.window_id.as_str())
            .collect::<Vec<_>>();
        identifiers.sort_unstable();
        if identifiers.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(KernelError::DuplicateEncounterWindow);
        }

        windows.sort_by(|left, right| {
            left.start_seconds
                .total_cmp(&right.start_seconds)
                .then_with(|| left.end_seconds.total_cmp(&right.end_seconds))
                .then_with(|| left.window_id.cmp(&right.window_id))
        });

        Ok(Self {
            event_id,
            revision,
            epoch,
            time_scale,
            reference_frame,
            horizon_start_seconds,
            horizon_end_seconds,
            hard_body_radius_m,
            window_inventory_coverage,
            windows,
        })
    }

    /// Stable event identifier.
    pub fn event_id(&self) -> &str {
        &self.event_id
    }

    /// Immutable event-definition revision.
    pub fn revision(&self) -> &str {
        &self.revision
    }

    /// Epoch to which the relative time coordinates refer.
    pub fn epoch(&self) -> &str {
        &self.epoch
    }

    /// Declared time scale, for example TAI or UTC.
    pub fn time_scale(&self) -> &str {
        &self.time_scale
    }

    /// Declared relative-position reference frame.
    pub fn reference_frame(&self) -> &str {
        &self.reference_frame
    }

    /// Inclusive horizon start in seconds relative to the epoch.
    pub fn horizon_start_seconds(&self) -> f64 {
        self.horizon_start_seconds
    }

    /// Inclusive horizon end in seconds relative to the epoch.
    pub fn horizon_end_seconds(&self) -> f64 {
        self.horizon_end_seconds
    }

    /// Hard-body radius in metres used by the declared event definition.
    pub fn hard_body_radius_m(&self) -> f64 {
        self.hard_body_radius_m
    }

    /// Whether the declared window inventory claims complete horizon coverage.
    ///
    /// Completeness is a caller declaration; this deterministic reducer cannot
    /// independently discover omitted encounter windows.
    pub fn window_inventory_coverage(&self) -> WindowInventoryCoverage {
        self.window_inventory_coverage
    }

    /// Encounter windows in deterministic chronological order.
    pub fn windows(&self) -> &[EncounterWindow] {
        &self.windows
    }

    /// Reduce one set of window outcomes for one joint trajectory realization.
    ///
    /// Exactly one outcome is required for each declared window, and every
    /// outcome must name the same trajectory-realization identifier. An
    /// overall contact is definite if any window is definitely contact; an
    /// overall no-contact result requires every window to be definitely
    /// outside. All remaining cases are ambiguous.
    ///
    /// A first-contact time enclosure is reported only if a definite contact
    /// is known, the inventory is complete, and no earlier unresolved window
    /// could contain an earlier contact.
    pub fn classify_realization(
        &self,
        outcomes: &[PathWindowOutcome],
    ) -> Result<PathwiseHorizonOutcome, KernelError> {
        if outcomes.len() != self.windows.len() {
            return Err(KernelError::MissingEncounterWindowOutcome);
        }

        let realization_id = outcomes
            .first()
            .map(|outcome| outcome.realization_id.trim())
            .filter(|value| !value.is_empty())
            .ok_or(KernelError::InvalidTrajectoryRealization)?;

        if outcomes
            .iter()
            .any(|outcome| outcome.realization_id.trim() != realization_id)
        {
            return Err(KernelError::MismatchedTrajectoryRealization);
        }

        let mut by_window = Vec::with_capacity(outcomes.len());
        for outcome in outcomes {
            let index = self
                .windows
                .iter()
                .position(|window| window.window_id == outcome.window_id)
                .ok_or(KernelError::UnknownEncounterWindow)?;
            if by_window.iter().any(|(seen, _)| *seen == index) {
                return Err(KernelError::DuplicateEncounterWindowOutcome);
            }

            if let PathWindowDisposition::DefinitelyContact {
                first_contact_time_lower_seconds,
                first_contact_time_upper_seconds,
            } = outcome.disposition
            {
                let window = &self.windows[index];
                if !first_contact_time_lower_seconds.is_finite()
                    || !first_contact_time_upper_seconds.is_finite()
                    || first_contact_time_lower_seconds > first_contact_time_upper_seconds
                    || first_contact_time_lower_seconds < window.start_seconds
                    || first_contact_time_upper_seconds > window.end_seconds
                {
                    return Err(KernelError::InvalidEncounterContactTime);
                }
            }
            by_window.push((index, outcome));
        }

        // Overlapping windows describe the same physical trajectory. Their
        // definite outcomes must therefore agree wherever a known first
        // contact lies inside another window.
        for (index, outcome) in &by_window {
            if let PathWindowDisposition::DefinitelyContact {
                first_contact_time_lower_seconds,
                first_contact_time_upper_seconds,
            } = outcome.disposition
            {
                for (other_index, other_outcome) in &by_window {
                    if index == other_index {
                        continue;
                    }
                    let other_window = &self.windows[*other_index];
                    let bracket_is_inside_other = first_contact_time_lower_seconds
                        >= other_window.start_seconds
                        && first_contact_time_upper_seconds <= other_window.end_seconds;
                    if bracket_is_inside_other {
                        match other_outcome.disposition {
                            PathWindowDisposition::DefinitelyNoContact => {
                                return Err(KernelError::ContradictoryEncounterWindowOutcome);
                            }
                            PathWindowDisposition::DefinitelyContact {
                                first_contact_time_lower_seconds: other_first_lower,
                                ..
                            } if other_first_lower > first_contact_time_upper_seconds => {
                                return Err(KernelError::ContradictoryEncounterWindowOutcome);
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        if self
            .windows
            .iter()
            .enumerate()
            .any(|(index, _)| !by_window.iter().any(|(seen, _)| *seen == index))
        {
            return Err(KernelError::MissingEncounterWindowOutcome);
        }

        // Canonicalize output order independently of caller submission order.
        by_window.sort_by_key(|(index, _)| *index);

        let mut contact_time_bounds = Vec::new();
        let mut contacting_window_ids = Vec::new();
        let mut unresolved_window_ids = Vec::new();

        for (index, outcome) in by_window {
            match outcome.disposition {
                PathWindowDisposition::DefinitelyNoContact => {}
                PathWindowDisposition::DefinitelyContact {
                    first_contact_time_lower_seconds,
                    first_contact_time_upper_seconds,
                } => {
                    contact_time_bounds.push((
                        first_contact_time_lower_seconds,
                        first_contact_time_upper_seconds,
                    ));
                    contacting_window_ids.push(self.windows[index].window_id.clone());
                }
                PathWindowDisposition::Ambiguous => {
                    unresolved_window_ids.push(self.windows[index].window_id.clone());
                }
            }
        }

        let disposition = if !contact_time_bounds.is_empty() {
            HorizonContactDisposition::DefinitelyContact
        } else if unresolved_window_ids.is_empty()
            && self.window_inventory_coverage == WindowInventoryCoverage::DeclaredComplete
        {
            HorizonContactDisposition::DefinitelyNoContact
        } else {
            HorizonContactDisposition::Ambiguous
        };

        let first_contact_time_bounds_seconds = if contact_time_bounds.is_empty() {
            None
        } else {
            let earliest_lower = contact_time_bounds
                .iter()
                .map(|(lower, _)| *lower)
                .fold(f64::INFINITY, f64::min);
            let earliest_upper = contact_time_bounds
                .iter()
                .map(|(_, upper)| *upper)
                .fold(f64::INFINITY, f64::min);
            let unresolved_could_be_earlier = self.window_inventory_coverage
                != WindowInventoryCoverage::DeclaredComplete
                || unresolved_window_ids.iter().any(|window_id| {
                    self.windows
                        .iter()
                        .find(|window| window.window_id == *window_id)
                        .is_some_and(|window| window.start_seconds < earliest_upper)
                });
            (!unresolved_could_be_earlier).then_some((earliest_lower, earliest_upper))
        };

        Ok(PathwiseHorizonOutcome {
            event_id: self.event_id.clone(),
            event_revision: self.revision.clone(),
            realization_id: realization_id.to_owned(),
            disposition,
            first_contact_time_bounds_seconds,
            contacting_window_ids,
            unresolved_window_ids,
            window_inventory_coverage: self.window_inventory_coverage,
        })
    }
}

/// Completeness declaration for the encounter-window inventory.
///
/// DeclaredComplete is an input assertion that the supplied windows, together
/// with independently excluded complementary portions of the horizon, account
/// for the entire horizon event. Merely enumerating local close-approach windows
/// is not sufficient to make this declaration. The reducer checks internal
/// consistency but does not prove the assertion or verify complement coverage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowInventoryCoverage {
    /// Caller declares that windows and separately excluded complement cover
    /// every possible contact time in the complete horizon.
    DeclaredComplete,
    /// Caller knows that one or more relevant windows may be absent.
    KnownIncomplete,
}

/// Three-way result supplied by a window-level trajectory evaluator.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PathWindowDisposition {
    /// The complete window is proven to contain no hard-body contact.
    DefinitelyNoContact,
    /// Contact is established and its earliest contact time is supplied.
    /// The caller must establish the first-hit semantics for this window.
    DefinitelyContact {
        /// Inclusive lower bound on the first contact time in seconds.
        first_contact_time_lower_seconds: f64,
        /// Inclusive upper bound on the first contact time in seconds.
        first_contact_time_upper_seconds: f64,
    },
    /// The window evaluator cannot decide within its declared assumptions
    /// and resource budget.
    Ambiguous,
}

/// One window's classification for one complete trajectory realization.
#[derive(Debug, Clone, PartialEq)]
pub struct PathWindowOutcome {
    /// Stable identifier for the joint trajectory realization.
    pub realization_id: String,
    /// Identifier of the encounter window being classified.
    pub window_id: String,
    /// Three-way window-level event disposition.
    pub disposition: PathWindowDisposition,
}

/// Pathwise event result after reducing all declared encounter windows.
#[derive(Debug, Clone, PartialEq)]
pub struct PathwiseHorizonOutcome {
    /// Stable event identifier.
    pub event_id: String,
    /// Event-definition revision used for classification.
    pub event_revision: String,
    /// Joint trajectory-realization identifier.
    pub realization_id: String,
    /// Horizon-level any-contact disposition.
    pub disposition: HorizonContactDisposition,
    /// Enclosure of the earliest horizon contact time, only when the window
    /// inventory is declared complete and earlier contact is not unresolved.
    pub first_contact_time_bounds_seconds: Option<(f64, f64)>,
    /// Window IDs that definitely contain contact. Overlapping windows may
    /// both appear here; this is not a count of distinct physical episodes.
    pub contacting_window_ids: Vec<String>,
    /// Window IDs whose contact status remains unresolved.
    pub unresolved_window_ids: Vec<String>,
    /// Caller-declared completeness of the encounter-window inventory.
    pub window_inventory_coverage: WindowInventoryCoverage,
}

/// Horizon-level result for the union of all declared encounter-window events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HorizonContactDisposition {
    /// At least one window is proven to contain contact.
    DefinitelyContact,
    /// Every declared window is proven to contain no contact, and the caller
    /// has declared that the full horizon is accounted for.
    DefinitelyNoContact,
    /// No definite contact exists and at least one window is unresolved.
    Ambiguous,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn window(id: &str, start: f64, end: f64) -> EncounterWindow {
        EncounterWindow {
            window_id: id.to_owned(),
            start_seconds: start,
            end_seconds: end,
        }
    }

    fn definition(windows: Vec<EncounterWindow>) -> HorizonEventDefinition {
        HorizonEventDefinition::new(
            "event-42",
            "revision-7",
            "2026-11-22T00:00:00",
            "TAI",
            "GCRF",
            0.0,
            100.0,
            5.0,
            WindowInventoryCoverage::DeclaredComplete,
            windows,
        )
        .expect("valid horizon definition")
    }

    fn outcome(
        realization: &str,
        id: &str,
        disposition: PathWindowDisposition,
    ) -> PathWindowOutcome {
        PathWindowOutcome {
            realization_id: realization.to_owned(),
            window_id: id.to_owned(),
            disposition,
        }
    }

    #[test]
    fn one_joint_contact_is_not_counted_once_per_overlapping_window() {
        let definition = definition(vec![
            window("early", 10.0, 30.0),
            window("late", 20.0, 40.0),
        ]);
        let result = definition
            .classify_realization(&[
                outcome(
                    "trajectory-1",
                    "late",
                    PathWindowDisposition::DefinitelyContact {
                        first_contact_time_lower_seconds: 24.0,
                        first_contact_time_upper_seconds: 24.0,
                    },
                ),
                outcome(
                    "trajectory-1",
                    "early",
                    PathWindowDisposition::DefinitelyContact {
                        first_contact_time_lower_seconds: 24.0,
                        first_contact_time_upper_seconds: 24.0,
                    },
                ),
            ])
            .expect("complete realization");

        assert_eq!(
            result.disposition,
            HorizonContactDisposition::DefinitelyContact
        );
        assert_eq!(result.first_contact_time_bounds_seconds, Some((24.0, 24.0)));
        assert_eq!(result.contacting_window_ids.len(), 2);
    }

    #[test]
    fn duplicate_bounds_are_allowed_but_union_remains_one_event() {
        let definition = definition(vec![window("a", 10.0, 20.0), window("b", 10.0, 20.0)]);
        let result = definition
            .classify_realization(&[
                outcome(
                    "trajectory-2",
                    "a",
                    PathWindowDisposition::DefinitelyContact {
                        first_contact_time_lower_seconds: 15.0,
                        first_contact_time_upper_seconds: 15.0,
                    },
                ),
                outcome(
                    "trajectory-2",
                    "b",
                    PathWindowDisposition::DefinitelyContact {
                        first_contact_time_lower_seconds: 15.0,
                        first_contact_time_upper_seconds: 15.0,
                    },
                ),
            ])
            .expect("complete realization");

        assert_eq!(
            result.disposition,
            HorizonContactDisposition::DefinitelyContact
        );
        assert_eq!(result.first_contact_time_bounds_seconds, Some((15.0, 15.0)));
    }

    #[test]
    fn earlier_ambiguous_window_preserves_any_contact_but_hides_first_hit_time() {
        let definition = definition(vec![
            window("early", 10.0, 20.0),
            window("late", 30.0, 40.0),
        ]);
        let result = definition
            .classify_realization(&[
                outcome("trajectory-3", "early", PathWindowDisposition::Ambiguous),
                outcome(
                    "trajectory-3",
                    "late",
                    PathWindowDisposition::DefinitelyContact {
                        first_contact_time_lower_seconds: 35.0,
                        first_contact_time_upper_seconds: 35.0,
                    },
                ),
            ])
            .expect("complete realization");

        assert_eq!(
            result.disposition,
            HorizonContactDisposition::DefinitelyContact
        );
        assert_eq!(result.first_contact_time_bounds_seconds, None);
        assert_eq!(result.unresolved_window_ids, vec!["early"]);
    }

    #[test]
    fn later_ambiguous_window_does_not_hide_an_earlier_known_first_hit() {
        let definition = definition(vec![
            window("early", 10.0, 20.0),
            window("late", 30.0, 40.0),
        ]);
        let result = definition
            .classify_realization(&[
                outcome(
                    "trajectory-4",
                    "early",
                    PathWindowDisposition::DefinitelyContact {
                        first_contact_time_lower_seconds: 15.0,
                        first_contact_time_upper_seconds: 15.0,
                    },
                ),
                outcome("trajectory-4", "late", PathWindowDisposition::Ambiguous),
            ])
            .expect("complete realization");

        assert_eq!(
            result.disposition,
            HorizonContactDisposition::DefinitelyContact
        );
        assert_eq!(result.first_contact_time_bounds_seconds, Some((15.0, 15.0)));
    }

    #[test]
    fn all_windows_must_be_proven_clear_for_horizon_no_contact() {
        let definition = definition(vec![window("a", 10.0, 20.0), window("b", 30.0, 40.0)]);
        let result = definition
            .classify_realization(&[
                outcome(
                    "trajectory-5",
                    "a",
                    PathWindowDisposition::DefinitelyNoContact,
                ),
                outcome(
                    "trajectory-5",
                    "b",
                    PathWindowDisposition::DefinitelyNoContact,
                ),
            ])
            .expect("complete realization");

        assert_eq!(
            result.disposition,
            HorizonContactDisposition::DefinitelyNoContact
        );
        assert_eq!(result.first_contact_time_bounds_seconds, None);
    }

    #[test]
    fn unresolved_without_definite_contact_is_ambiguous() {
        let definition = definition(vec![window("a", 10.0, 20.0), window("b", 30.0, 40.0)]);
        let result = definition
            .classify_realization(&[
                outcome("trajectory-6", "a", PathWindowDisposition::Ambiguous),
                outcome(
                    "trajectory-6",
                    "b",
                    PathWindowDisposition::DefinitelyNoContact,
                ),
            ])
            .expect("complete realization");

        assert_eq!(result.disposition, HorizonContactDisposition::Ambiguous);
    }

    #[test]
    fn outcome_submission_order_does_not_change_canonical_report_order() {
        let definition = definition(vec![
            window("later", 30.0, 40.0),
            window("earlier", 10.0, 20.0),
            window("middle", 20.0, 30.0),
        ]);
        let early = outcome(
            "trajectory-order",
            "earlier",
            PathWindowDisposition::DefinitelyContact {
                first_contact_time_lower_seconds: 15.0,
                first_contact_time_upper_seconds: 15.0,
            },
        );
        let middle = outcome(
            "trajectory-order",
            "middle",
            PathWindowDisposition::Ambiguous,
        );
        let late = outcome(
            "trajectory-order",
            "later",
            PathWindowDisposition::DefinitelyContact {
                first_contact_time_lower_seconds: 35.0,
                first_contact_time_upper_seconds: 35.0,
            },
        );

        let forward = definition
            .classify_realization(&[late.clone(), middle.clone(), early.clone()])
            .expect("complete forward ordering");
        let reverse = definition
            .classify_realization(&[early, middle, late])
            .expect("complete reverse ordering");
        assert_eq!(forward, reverse);
        assert_eq!(
            forward.contacting_window_ids,
            vec!["earlier".to_owned(), "later".to_owned()]
        );
        assert_eq!(
            forward.unresolved_window_ids,
            vec!["middle".to_owned()]
        );
    }

    #[test]
    fn windows_are_stored_in_deterministic_chronological_order() {
        let definition = definition(vec![
            window("nested", 15.0, 16.0),
            window("second", 20.0, 30.0),
            window("first", 10.0, 20.0),
        ]);
        let ids = definition
            .windows()
            .iter()
            .map(|window| window.window_id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(ids, vec!["first", "nested", "second"]);
    }

    #[test]
    fn missing_duplicate_and_unknown_window_outcomes_are_rejected() {
        let definition = definition(vec![window("a", 10.0, 20.0), window("b", 30.0, 40.0)]);
        assert_eq!(
            definition.classify_realization(&[outcome(
                "trajectory-7",
                "a",
                PathWindowDisposition::DefinitelyNoContact
            )]),
            Err(KernelError::MissingEncounterWindowOutcome)
        );
        assert_eq!(
            definition.classify_realization(&[
                outcome(
                    "trajectory-7",
                    "a",
                    PathWindowDisposition::DefinitelyNoContact,
                ),
                outcome(
                    "trajectory-7",
                    "a",
                    PathWindowDisposition::DefinitelyNoContact,
                ),
            ]),
            Err(KernelError::DuplicateEncounterWindowOutcome)
        );
        assert_eq!(
            definition.classify_realization(&[
                outcome(
                    "trajectory-7",
                    "a",
                    PathWindowDisposition::DefinitelyNoContact,
                ),
                outcome(
                    "trajectory-7",
                    "wrong",
                    PathWindowDisposition::DefinitelyNoContact,
                ),
            ]),
            Err(KernelError::UnknownEncounterWindow)
        );
    }

    #[test]
    fn mismatched_realization_ids_are_rejected() {
        let definition = definition(vec![window("a", 10.0, 20.0), window("b", 30.0, 40.0)]);
        assert_eq!(
            definition.classify_realization(&[
                outcome(
                    "trajectory-a",
                    "a",
                    PathWindowDisposition::DefinitelyNoContact,
                ),
                outcome(
                    "trajectory-b",
                    "b",
                    PathWindowDisposition::DefinitelyNoContact,
                ),
            ]),
            Err(KernelError::MismatchedTrajectoryRealization)
        );
    }

    #[test]
    fn first_contact_time_is_preserved_as_a_certified_bracket() {
        let definition = definition(vec![window("a", 10.0, 30.0)]);
        let result = definition
            .classify_realization(&[outcome(
                "trajectory-bracket",
                "a",
                PathWindowDisposition::DefinitelyContact {
                    first_contact_time_lower_seconds: 18.25,
                    first_contact_time_upper_seconds: 18.5,
                },
            )])
            .expect("valid first-contact bracket");
        assert_eq!(
            result.first_contact_time_bounds_seconds,
            Some((18.25, 18.5))
        );
    }

    #[test]
    fn reversed_first_contact_bracket_is_rejected() {
        let definition = definition(vec![window("a", 10.0, 30.0)]);
        assert_eq!(
            definition.classify_realization(&[outcome(
                "trajectory-bracket-invalid",
                "a",
                PathWindowDisposition::DefinitelyContact {
                    first_contact_time_lower_seconds: 19.0,
                    first_contact_time_upper_seconds: 18.0,
                },
            )]),
            Err(KernelError::InvalidEncounterContactTime)
        );
    }

    #[test]
    fn contact_time_must_lie_inside_its_window() {
        let definition = definition(vec![window("a", 10.0, 20.0)]);
        assert_eq!(
            definition.classify_realization(&[outcome(
                "trajectory-8",
                "a",
                PathWindowDisposition::DefinitelyContact {
                    first_contact_time_lower_seconds: 21.0,
                    first_contact_time_upper_seconds: 21.0,
                }
            )]),
            Err(KernelError::InvalidEncounterContactTime)
        );
    }

    #[test]
    fn exhaustive_three_window_union_matches_independent_finite_oracle() {
        let definition = definition(vec![
            window("a", 10.0, 11.0),
            window("b", 20.0, 21.0),
            window("c", 30.0, 31.0),
        ]);

        for mask in 0_u32..8 {
            let mut outcomes = Vec::new();
            let mut oracle_first_time = None;
            for (index, (id, contact_time)) in [("a", 10.5_f64), ("b", 20.5_f64), ("c", 30.5_f64)]
                .into_iter()
                .enumerate()
            {
                let is_contact = ((mask >> index) & 1) == 1;
                let disposition = if is_contact {
                    oracle_first_time = Some(
                        oracle_first_time
                            .map_or(contact_time, |current: f64| current.min(contact_time)),
                    );
                    PathWindowDisposition::DefinitelyContact {
                        first_contact_time_lower_seconds: contact_time,
                        first_contact_time_upper_seconds: contact_time,
                    }
                } else {
                    PathWindowDisposition::DefinitelyNoContact
                };
                outcomes.push(outcome("trajectory-enumerated", id, disposition));
            }

            let result = definition
                .classify_realization(&outcomes)
                .expect("complete enumerated realization");
            let expected = if mask == 0 {
                HorizonContactDisposition::DefinitelyNoContact
            } else {
                HorizonContactDisposition::DefinitelyContact
            };
            assert_eq!(result.disposition, expected);
            assert_eq!(
                result.first_contact_time_bounds_seconds,
                oracle_first_time.map(|time| (time, time))
            );
        }
    }

    #[test]
    fn invalid_definitions_fail_closed() {
        assert_eq!(
            HorizonEventDefinition::new(
                " ",
                "revision",
                "epoch",
                "TAI",
                "GCRF",
                0.0,
                10.0,
                1.0,
                WindowInventoryCoverage::DeclaredComplete,
                vec![window("a", 1.0, 2.0)],
            ),
            Err(KernelError::InvalidEventDefinition)
        );
        assert_eq!(
            HorizonEventDefinition::new(
                "event",
                "revision",
                "epoch",
                "TAI",
                "GCRF",
                10.0,
                0.0,
                1.0,
                WindowInventoryCoverage::DeclaredComplete,
                vec![window("a", 1.0, 2.0)],
            ),
            Err(KernelError::InvalidEventDefinition)
        );
        assert_eq!(
            HorizonEventDefinition::new(
                "event",
                "revision",
                "epoch",
                "TAI",
                "GCRF",
                0.0,
                10.0,
                1.0,
                WindowInventoryCoverage::DeclaredComplete,
                vec![window("a", 0.0, 2.0), window("a", 2.0, 3.0)],
            ),
            Err(KernelError::DuplicateEncounterWindow)
        );
        assert_eq!(
            HorizonEventDefinition::new(
                "event",
                "revision",
                "epoch",
                "TAI",
                "GCRF",
                0.0,
                10.0,
                1.0,
                WindowInventoryCoverage::DeclaredComplete,
                vec![window("a", 1.0, 11.0)],
            ),
            Err(KernelError::InvalidEncounterWindow)
        );
        assert_eq!(
            HorizonEventDefinition::new(
                "event",
                "revision",
                "epoch",
                "TAI",
                "GCRF",
                0.0,
                10.0,
                0.0,
                WindowInventoryCoverage::DeclaredComplete,
                vec![window("a", 1.0, 2.0)],
            ),
            Err(KernelError::InvalidHardBodyRadius)
        );
    }

    #[test]
    fn incomplete_window_inventory_cannot_prove_horizon_no_contact() {
        let definition = HorizonEventDefinition::new(
            "event-42",
            "revision-8",
            "epoch",
            "TAI",
            "GCRF",
            0.0,
            100.0,
            5.0,
            WindowInventoryCoverage::KnownIncomplete,
            vec![window("a", 10.0, 20.0)],
        )
        .expect("valid partial definition");
        let result = definition
            .classify_realization(&[outcome(
                "trajectory-10",
                "a",
                PathWindowDisposition::DefinitelyNoContact,
            )])
            .expect("valid partial evaluation");
        assert_eq!(result.disposition, HorizonContactDisposition::Ambiguous);
        assert_eq!(
            result.window_inventory_coverage,
            WindowInventoryCoverage::KnownIncomplete
        );
    }

    #[test]
    fn incomplete_inventory_with_a_proven_contact_proves_any_contact_but_not_first_time() {
        let definition = HorizonEventDefinition::new(
            "event-42",
            "revision-9",
            "epoch",
            "TAI",
            "GCRF",
            0.0,
            100.0,
            5.0,
            WindowInventoryCoverage::KnownIncomplete,
            vec![window("a", 10.0, 20.0)],
        )
        .expect("valid partial definition");
        let result = definition
            .classify_realization(&[outcome(
                "trajectory-11",
                "a",
                PathWindowDisposition::DefinitelyContact {
                    first_contact_time_lower_seconds: 15.0,
                    first_contact_time_upper_seconds: 15.0,
                },
            )])
            .expect("valid partial evaluation");
        assert_eq!(
            result.disposition,
            HorizonContactDisposition::DefinitelyContact
        );
        assert_eq!(result.first_contact_time_bounds_seconds, None);
    }

    #[test]
    fn contradictory_outcomes_for_overlapping_windows_are_rejected() {
        let definition = definition(vec![window("a", 10.0, 30.0), window("b", 20.0, 40.0)]);
        assert_eq!(
            definition.classify_realization(&[
                outcome(
                    "trajectory-12",
                    "a",
                    PathWindowDisposition::DefinitelyContact {
                        first_contact_time_lower_seconds: 24.0,
                        first_contact_time_upper_seconds: 24.0,
                    },
                ),
                outcome(
                    "trajectory-12",
                    "b",
                    PathWindowDisposition::DefinitelyNoContact,
                ),
            ]),
            Err(KernelError::ContradictoryEncounterWindowOutcome)
        );
    }

    #[test]
    fn every_window_is_required_even_after_a_definite_contact() {
        let definition = definition(vec![window("a", 10.0, 20.0), window("b", 30.0, 40.0)]);
        assert_eq!(
            definition.classify_realization(&[outcome(
                "trajectory-9",
                "a",
                PathWindowDisposition::DefinitelyContact {
                    first_contact_time_lower_seconds: 15.0,
                    first_contact_time_upper_seconds: 15.0,
                }
            )]),
            Err(KernelError::MissingEncounterWindowOutcome)
        );
    }
}
