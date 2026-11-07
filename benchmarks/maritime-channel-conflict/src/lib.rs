#![forbid(unsafe_code)]

//! Fail-closed verification of declared maritime traffic topology.
//!
//! The crate consumes fixed fleet-oracle operation placements. It verifies
//! direction windows, same-segment headway and a complete pairwise interaction
//! matrix. It does not schedule traffic, infer channel geometry, run VTS,
//! simulate ship handling, or issue navigational authority.

use std::collections::{BTreeMap, BTreeSet};

use maritime_service_chain_contract::TickWindow;
use maritime_service_chain_fleet::OperationPlacement;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EvidenceState {
    Current,
    Stale,
    Unknown,
    Disputed,
    Revoked,
    Unsupported,
}

/// Evidence reference scoped to an exact segment or segment-pair subject.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceReceipt {
    pub evidence_id: String,
    pub source_id: String,
    pub source_revision: String,
    pub subject_id: String,
    pub subject_revision: String,
    pub state: EvidenceState,
    pub valid_from_tick: u64,
    pub valid_through_tick: u64,
    pub claim_ceiling: String,
}

impl EvidenceReceipt {
    fn validate(&self) -> Result<(), TrafficError> {
        if self.evidence_id.trim().is_empty()
            || self.source_id.trim().is_empty()
            || self.source_revision.trim().is_empty()
            || self.subject_id.trim().is_empty()
            || self.subject_revision.trim().is_empty()
            || self.claim_ceiling.trim().is_empty()
            || self.valid_from_tick > self.valid_through_tick
        {
            return Err(TrafficError::InvalidEvidenceReceipt(self.evidence_id.clone()));
        }
        Ok(())
    }

    fn covers(&self, window: TickWindow) -> bool {
        self.valid_from_tick <= window.start_tick
            && self.valid_through_tick >= window.end_tick
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TravelDirection {
    Forward,
    Reverse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DirectionPermission {
    ForwardOnly,
    ReverseOnly,
    Closed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectionWindow {
    pub window: TickWindow,
    pub permission: DirectionPermission,
    pub evidence: EvidenceReceipt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelSegmentRule {
    pub segment_id: String,
    pub from_node_id: String,
    pub to_node_id: String,
    pub route_revision: String,
    /// Minimum empty interval between two uses of this same segment.
    /// A zero value still forbids overlapping half-open occupancy intervals.
    pub same_segment_min_separation_ticks: u64,
    pub headway_evidence: EvidenceReceipt,
    /// Gaps are unknown, never implicitly open.
    pub direction_windows: Vec<DirectionWindow>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InteractionRelation {
    /// The reviewed topology declares no traffic conflict between the pair.
    Independent,
    /// The pair shares a conflict area and needs this minimum empty interval.
    Conflict {
        minimum_separation_ticks: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SegmentInteraction {
    pub left_segment_id: String,
    pub right_segment_id: String,
    pub route_revision: String,
    pub relation: InteractionRelation,
    /// Must bind canonical subject left_segment_id|right_segment_id.
    pub evidence: EvidenceReceipt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelMovement {
    pub movement_id: String,
    pub segment_id: String,
    pub route_revision: String,
    pub from_node_id: String,
    pub to_node_id: String,
    /// Exact fixed operation from the shared fleet oracle.
    pub operation: OperationPlacement,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelWorkload {
    pub corridor_id: String,
    pub time_basis_id: String,
    pub route_revision: String,
    pub decision_tick: u64,
    pub segments: Vec<ChannelSegmentRule>,
    /// Closed-world pair matrix: every unordered pair of distinct segments
    /// must be explicitly declared Independent or Conflict.
    pub interactions: Vec<SegmentInteraction>,
    pub movements: Vec<ChannelMovement>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrafficDisposition {
    ReadyForComparison,
    NeedsEvidence,
    Infeasible,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum TrafficViolation {
    MissingDirectionCoverage(String),
    DirectionEvidenceNotCurrent {
        segment_id: String,
        evidence_id: String,
        state: EvidenceState,
    },
    DirectionEvidenceWindowInsufficient {
        segment_id: String,
        evidence_id: String,
    },
    SegmentRouteRevisionMismatch(String),
    MovementOnClosedSegment(String),
    MovementDirectionNotPermitted(String),
    HeadwayEvidenceNotCurrent {
        segment_id: String,
        evidence_id: String,
        state: EvidenceState,
    },
    HeadwayEvidenceWindowInsufficient {
        segment_id: String,
        evidence_id: String,
    },
    SameSegmentHeadwayConflict {
        segment_id: String,
        first_movement_id: String,
        second_movement_id: String,
    },
    MissingSegmentInteraction {
        left_segment_id: String,
        right_segment_id: String,
    },
    InteractionEvidenceNotCurrent {
        left_segment_id: String,
        right_segment_id: String,
        evidence_id: String,
        state: EvidenceState,
    },
    InteractionEvidenceWindowInsufficient {
        left_segment_id: String,
        right_segment_id: String,
        evidence_id: String,
    },
    CrossSegmentConflict {
        left_segment_id: String,
        right_segment_id: String,
        first_movement_id: String,
        second_movement_id: String,
    },
}

impl TrafficViolation {
    fn is_unsupported(&self) -> bool {
        matches!(
            self,
            Self::DirectionEvidenceNotCurrent {
                state: EvidenceState::Revoked | EvidenceState::Unsupported,
                ..
            } | Self::HeadwayEvidenceNotCurrent {
                state: EvidenceState::Revoked | EvidenceState::Unsupported,
                ..
            } | Self::InteractionEvidenceNotCurrent {
                state: EvidenceState::Revoked | EvidenceState::Unsupported,
                ..
            }
        )
    }

    fn needs_evidence(&self) -> bool {
        matches!(
            self,
            Self::MissingDirectionCoverage(_)
                | Self::DirectionEvidenceWindowInsufficient { .. }
                | Self::SegmentRouteRevisionMismatch(_)
                | Self::HeadwayEvidenceWindowInsufficient { .. }
                | Self::InteractionEvidenceWindowInsufficient { .. }
                | Self::MissingSegmentInteraction { .. }
                | Self::DirectionEvidenceNotCurrent {
                    state: EvidenceState::Stale | EvidenceState::Unknown | EvidenceState::Disputed,
                    ..
                }
                | Self::HeadwayEvidenceNotCurrent {
                    state: EvidenceState::Stale | EvidenceState::Unknown | EvidenceState::Disputed,
                    ..
                }
                | Self::InteractionEvidenceNotCurrent {
                    state: EvidenceState::Stale | EvidenceState::Unknown | EvidenceState::Disputed,
                    ..
                }
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrafficReport {
    pub disposition: TrafficDisposition,
    pub violations: Vec<TrafficViolation>,
    pub movements_checked: usize,
    pub segment_pairs_checked: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrafficError {
    EmptyCorridor,
    EmptyTimeBasis,
    EmptyRouteRevision,
    DuplicateSegment(String),
    DuplicateDirectionWindow(String),
    DuplicateInteraction(String),
    DuplicateMovement(String),
    InvalidSegment(String),
    InvalidDirectionWindow(String),
    OverlappingDirectionWindows(String),
    InvalidInteraction(String),
    InvalidMovement(String),
    UnknownSegment(String),
    InvalidEvidenceReceipt(String),
}

fn pair_key(left: &str, right: &str) -> (String, String) {
    if left <= right {
        (left.to_owned(), right.to_owned())
    } else {
        (right.to_owned(), left.to_owned())
    }
}

fn pair_subject(left: &str, right: &str) -> String {
    let (first, second) = pair_key(left, right);
    format!("{first}|{second}")
}

fn validate_direction_windows(
    segment_id: &str,
    windows: &[DirectionWindow],
) -> Result<(), TrafficError> {
    for window in windows {
        if !window.window.is_valid() {
            return Err(TrafficError::InvalidDirectionWindow(segment_id.to_owned()));
        }
        window.evidence.validate()?;
        if window.evidence.subject_id != segment_id {
            return Err(TrafficError::InvalidEvidenceReceipt(
                window.evidence.evidence_id.clone(),
            ));
        }
    }

    for pair in windows.windows(2) {
        if pair[0].window.start_tick > pair[1].window.start_tick {
            return Err(TrafficError::InvalidDirectionWindow(segment_id.to_owned()));
        }
        if pair[0].window.end_tick > pair[1].window.start_tick {
            return Err(TrafficError::OverlappingDirectionWindows(
                segment_id.to_owned(),
            ));
        }
    }
    Ok(())
}

fn movement_direction(
    rule: &ChannelSegmentRule,
    movement: &ChannelMovement,
) -> Option<TravelDirection> {
    if movement.from_node_id == rule.from_node_id && movement.to_node_id == rule.to_node_id {
        Some(TravelDirection::Forward)
    } else if movement.from_node_id == rule.to_node_id
        && movement.to_node_id == rule.from_node_id
    {
        Some(TravelDirection::Reverse)
    } else {
        None
    }
}

fn separated(
    first: TickWindow,
    second: TickWindow,
    minimum_separation_ticks: u64,
) -> bool {
    (first.end_tick <= second.start_tick
        && second.start_tick - first.end_tick >= minimum_separation_ticks)
        || (second.end_tick <= first.start_tick
            && first.start_tick - second.end_tick >= minimum_separation_ticks)
}

fn record_direction_evidence(
    segment_id: &str,
    evidence: &EvidenceReceipt,
    movement_window: TickWindow,
    violations: &mut BTreeSet<TrafficViolation>,
) {
    if evidence.state != EvidenceState::Current {
        violations.insert(TrafficViolation::DirectionEvidenceNotCurrent {
            segment_id: segment_id.to_owned(),
            evidence_id: evidence.evidence_id.clone(),
            state: evidence.state,
        });
    } else if !evidence.covers(movement_window) {
        violations.insert(TrafficViolation::DirectionEvidenceWindowInsufficient {
            segment_id: segment_id.to_owned(),
            evidence_id: evidence.evidence_id.clone(),
        });
    }
}

/// Verify a fixed traffic plan without scheduling or modifying it.
///
/// The interaction matrix is closed-world: a missing pair declaration is
/// NeedsEvidence. A pair is not assumed independent merely because no conflict
/// rule was supplied.
pub fn evaluate_channel_workload(
    workload: &ChannelWorkload,
) -> Result<TrafficReport, TrafficError> {
    if workload.corridor_id.trim().is_empty() {
        return Err(TrafficError::EmptyCorridor);
    }
    if workload.time_basis_id.trim().is_empty() {
        return Err(TrafficError::EmptyTimeBasis);
    }
    if workload.route_revision.trim().is_empty() {
        return Err(TrafficError::EmptyRouteRevision);
    }

    let mut segment_by_id = BTreeMap::new();
    for segment in &workload.segments {
        if segment.segment_id.trim().is_empty()
            || segment.from_node_id.trim().is_empty()
            || segment.to_node_id.trim().is_empty()
            || segment.from_node_id == segment.to_node_id
            || segment.route_revision.trim().is_empty()
        {
            return Err(TrafficError::InvalidSegment(segment.segment_id.clone()));
        }
        segment.headway_evidence.validate()?;
        if segment.headway_evidence.subject_id != segment.segment_id
            || segment.headway_evidence.subject_revision != segment.route_revision
        {
            return Err(TrafficError::InvalidEvidenceReceipt(
                segment.headway_evidence.evidence_id.clone(),
            ));
        }
        validate_direction_windows(&segment.segment_id, &segment.direction_windows)?;
        for window in &segment.direction_windows {
            if window.evidence.subject_revision != segment.route_revision {
                return Err(TrafficError::InvalidEvidenceReceipt(
                    window.evidence.evidence_id.clone(),
                ));
            }
        }
        if segment_by_id.insert(segment.segment_id.as_str(), segment).is_some() {
            return Err(TrafficError::DuplicateSegment(segment.segment_id.clone()));
        }
    }

    let mut interactions = BTreeMap::new();
    for interaction in &workload.interactions {
        if interaction.left_segment_id.trim().is_empty()
            || interaction.right_segment_id.trim().is_empty()
            || interaction.left_segment_id == interaction.right_segment_id
            || interaction.route_revision.trim().is_empty()
            || !segment_by_id.contains_key(interaction.left_segment_id.as_str())
            || !segment_by_id.contains_key(interaction.right_segment_id.as_str())
        {
            return Err(TrafficError::InvalidInteraction(
                interaction.evidence.evidence_id.clone(),
            ));
        }
        interaction.evidence.validate()?;
        let key = pair_key(&interaction.left_segment_id, &interaction.right_segment_id);
        if interaction.route_revision != workload.route_revision
            || interaction.evidence.subject_id != pair_subject(&key.0, &key.1)
            || interaction.evidence.subject_revision != interaction.route_revision
        {
            return Err(TrafficError::InvalidEvidenceReceipt(
                interaction.evidence.evidence_id.clone(),
            ));
        }
        if interactions.insert(key.clone(), interaction).is_some() {
            return Err(TrafficError::DuplicateInteraction(format!(
                "{}|{}",
                key.0, key.1
            )));
        }
    }

    let mut movement_by_id = BTreeMap::new();
    let mut movements_by_segment = BTreeMap::<&str, Vec<&ChannelMovement>>::new();
    let mut violations = BTreeSet::new();

    for movement in &workload.movements {
        if movement.movement_id.trim().is_empty()
            || movement.segment_id.trim().is_empty()
            || movement.route_revision.trim().is_empty()
            || movement.from_node_id.trim().is_empty()
            || movement.to_node_id.trim().is_empty()
            || movement.operation.start_tick >= movement.operation.end_tick
            || movement.operation.start_tick < workload.decision_tick
            || movement.operation.segment_id != movement.segment_id
        {
            return Err(TrafficError::InvalidMovement(movement.movement_id.clone()));
        }
        let Some(rule) = segment_by_id.get(movement.segment_id.as_str()).copied() else {
            return Err(TrafficError::UnknownSegment(movement.segment_id.clone()));
        };
        if movement_by_id
            .insert(movement.movement_id.as_str(), movement)
            .is_some()
        {
            return Err(TrafficError::DuplicateMovement(movement.movement_id.clone()));
        }

        if movement.route_revision != workload.route_revision
            || movement.route_revision != rule.route_revision
        {
            violations.insert(TrafficViolation::SegmentRouteRevisionMismatch(
                movement.segment_id.clone(),
            ));
        }

        let Some(direction) = movement_direction(rule, movement) else {
            return Err(TrafficError::InvalidMovement(movement.movement_id.clone()));
        };

        let movement_window = TickWindow {
            start_tick: movement.operation.start_tick,
            end_tick: movement.operation.end_tick,
        };

        let Some(direction_window) = rule
            .direction_windows
            .iter()
            .find(|window| window.window.covers(movement_window))
        else {
            violations.insert(TrafficViolation::MissingDirectionCoverage(
                movement.segment_id.clone(),
            ));
            movements_by_segment
                .entry(movement.segment_id.as_str())
                .or_default()
                .push(movement);
            continue;
        };

        record_direction_evidence(
            &movement.segment_id,
            &direction_window.evidence,
            movement_window,
            &mut violations,
        );
        match direction_window.permission {
            DirectionPermission::Closed => {
                violations.insert(TrafficViolation::MovementOnClosedSegment(
                    movement.movement_id.clone(),
                ));
            }
            DirectionPermission::ForwardOnly if direction != TravelDirection::Forward => {
                violations.insert(TrafficViolation::MovementDirectionNotPermitted(
                    movement.movement_id.clone(),
                ));
            }
            DirectionPermission::ReverseOnly if direction != TravelDirection::Reverse => {
                violations.insert(TrafficViolation::MovementDirectionNotPermitted(
                    movement.movement_id.clone(),
                ));
            }
            DirectionPermission::ForwardOnly | DirectionPermission::ReverseOnly => {}
        }

        if rule.headway_evidence.state != EvidenceState::Current {
            violations.insert(TrafficViolation::HeadwayEvidenceNotCurrent {
                segment_id: movement.segment_id.clone(),
                evidence_id: rule.headway_evidence.evidence_id.clone(),
                state: rule.headway_evidence.state,
            });
        } else if !rule.headway_evidence.covers(movement_window) {
            violations.insert(TrafficViolation::HeadwayEvidenceWindowInsufficient {
                segment_id: movement.segment_id.clone(),
                evidence_id: rule.headway_evidence.evidence_id.clone(),
            });
        }

        movements_by_segment
            .entry(movement.segment_id.as_str())
            .or_default()
            .push(movement);
    }

    for (segment_id, movements) in &mut movements_by_segment {
        movements.sort_by(|left, right| {
            left.operation
                .start_tick
                .cmp(&right.operation.start_tick)
                .then_with(|| left.operation.end_tick.cmp(&right.operation.end_tick))
                .then_with(|| left.movement_id.cmp(&right.movement_id))
        });
        let rule = segment_by_id[segment_id];
        for pair in movements.windows(2) {
            let first = pair[0];
            let second = pair[1];
            let first_window = TickWindow {
                start_tick: first.operation.start_tick,
                end_tick: first.operation.end_tick,
            };
            let second_window = TickWindow {
                start_tick: second.operation.start_tick,
                end_tick: second.operation.end_tick,
            };
            let evidence_window = TickWindow {
                start_tick: first_window.start_tick.min(second_window.start_tick),
                end_tick: first_window.end_tick.max(second_window.end_tick),
            };
            if rule.headway_evidence.state != EvidenceState::Current {
                continue;
            }
            if !rule.headway_evidence.covers(evidence_window) {
                violations.insert(TrafficViolation::HeadwayEvidenceWindowInsufficient {
                    segment_id: (*segment_id).to_owned(),
                    evidence_id: rule.headway_evidence.evidence_id.clone(),
                });
                continue;
            }
            if !separated(
                first_window,
                second_window,
                rule.same_segment_min_separation_ticks,
            ) {
                violations.insert(TrafficViolation::SameSegmentHeadwayConflict {
                    segment_id: (*segment_id).to_owned(),
                    first_movement_id: first.movement_id.clone(),
                    second_movement_id: second.movement_id.clone(),
                });
            }
        }
    }

    let segment_ids: Vec<&str> = segment_by_id.keys().copied().collect();
    let mut segment_pairs_checked = 0;
    for left_index in 0..segment_ids.len() {
        for right_index in (left_index + 1)..segment_ids.len() {
            let left_id = segment_ids[left_index];
            let right_id = segment_ids[right_index];
            let key = pair_key(left_id, right_id);
            let Some(interaction) = interactions.get(&key).copied() else {
                violations.insert(TrafficViolation::MissingSegmentInteraction {
                    left_segment_id: key.0,
                    right_segment_id: key.1,
                });
                continue;
            };
            segment_pairs_checked += 1;
            let Some(left_movements) = movements_by_segment.get(left_id) else {
                continue;
            };
            let Some(right_movements) = movements_by_segment.get(right_id) else {
                continue;
            };

            for left_movement in left_movements {
                for right_movement in right_movements {
                    let evidence_window = TickWindow {
                        start_tick: left_movement
                            .operation
                            .start_tick
                            .min(right_movement.operation.start_tick),
                        end_tick: left_movement
                            .operation
                            .end_tick
                            .max(right_movement.operation.end_tick),
                    };
                    if interaction.evidence.state != EvidenceState::Current {
                        violations.insert(TrafficViolation::InteractionEvidenceNotCurrent {
                            left_segment_id: key.0.clone(),
                            right_segment_id: key.1.clone(),
                            evidence_id: interaction.evidence.evidence_id.clone(),
                            state: interaction.evidence.state,
                        });
                        continue;
                    }
                    if !interaction.evidence.covers(evidence_window) {
                        violations.insert(
                            TrafficViolation::InteractionEvidenceWindowInsufficient {
                                left_segment_id: key.0.clone(),
                                right_segment_id: key.1.clone(),
                                evidence_id: interaction.evidence.evidence_id.clone(),
                            },
                        );
                        continue;
                    }

                    if let InteractionRelation::Conflict {
                        minimum_separation_ticks,
                    } = interaction.relation
                    {
                        let left_window = TickWindow {
                            start_tick: left_movement.operation.start_tick,
                            end_tick: left_movement.operation.end_tick,
                        };
                        let right_window = TickWindow {
                            start_tick: right_movement.operation.start_tick,
                            end_tick: right_movement.operation.end_tick,
                        };
                        if !separated(left_window, right_window, minimum_separation_ticks) {
                            violations.insert(TrafficViolation::CrossSegmentConflict {
                                left_segment_id: key.0.clone(),
                                right_segment_id: key.1.clone(),
                                first_movement_id: left_movement.movement_id.clone(),
                                second_movement_id: right_movement.movement_id.clone(),
                            });
                        }
                    }
                }
            }
        }
    }

    let disposition = if violations.iter().any(TrafficViolation::is_unsupported) {
        TrafficDisposition::Unsupported
    } else if violations.iter().any(TrafficViolation::needs_evidence) {
        TrafficDisposition::NeedsEvidence
    } else if !violations.is_empty() {
        TrafficDisposition::Infeasible
    } else {
        TrafficDisposition::ReadyForComparison
    };

    Ok(TrafficReport {
        disposition,
        violations: violations.into_iter().collect(),
        movements_checked: movement_by_id.len(),
        segment_pairs_checked,
    })
}
