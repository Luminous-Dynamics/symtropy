// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Exact six-leg joint stance/swing configuration, dwell and transition metrics.
//!
//! This module complements per-leg duty factor, pairwise phase-vector strength
//! and tripod coordination strength. A full state is a 6-bit mask, with bit 1
//! denoting stance, in the fixed order LEFT_FORE, LEFT_MIDDLE, LEFT_HIND,
//! RIGHT_FORE, RIGHT_MIDDLE, RIGHT_HIND.
//!
//! Inputs are synchronized half-open integer-tick intervals. Incomplete leg
//! observations never become swing observations. Transition counts are
//! empirical contiguous-boundary counts, not a declaration of Markovity.

use std::fmt;

pub const JOINT_LEG_COUNT: usize = 6;
pub const JOINT_STATE_COUNT: usize = 1 << JOINT_LEG_COUNT;
pub const COMPLETE_LEG_MASK: u8 = (1 << JOINT_LEG_COUNT) - 1;

/// Canonical bit order for every joint-state mask.
pub const JOINT_LEG_ORDER: [&str; JOINT_LEG_COUNT] = [
    "LEFT_FORE",
    "LEFT_MIDDLE",
    "LEFT_HIND",
    "RIGHT_FORE",
    "RIGHT_MIDDLE",
    "RIGHT_HIND",
];

/// Conventional Drosophila tripod groups in the canonical leg order:
/// L1-R2-L3 = [left fore, right middle, left hind];
/// R1-L2-R3 = [right fore, left middle, right hind].
pub const TRIPOD_GROUPS: [[usize; 3]; 2] = [[0, 4, 2], [3, 1, 5]];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnavailableLegStatus {
    MissingTrack,
    OutOfSupport,
    FailedAssay,
    NotAssessed,
    Disputed,
}

impl UnavailableLegStatus {
    pub const ALL: [Self; 5] = [
        Self::MissingTrack,
        Self::OutOfSupport,
        Self::FailedAssay,
        Self::NotAssessed,
        Self::Disputed,
    ];

    pub const fn index(self) -> usize {
        match self {
            Self::MissingTrack => 0,
            Self::OutOfSupport => 1,
            Self::FailedAssay => 2,
            Self::NotAssessed => 3,
            Self::Disputed => 4,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LegSampleState {
    Stance,
    Swing,
    MissingTrack,
    OutOfSupport,
    FailedAssay,
    NotAssessed,
    Disputed,
}

impl LegSampleState {
    fn unavailable_status(self) -> Option<UnavailableLegStatus> {
        match self {
            Self::Stance | Self::Swing => None,
            Self::MissingTrack => Some(UnavailableLegStatus::MissingTrack),
            Self::OutOfSupport => Some(UnavailableLegStatus::OutOfSupport),
            Self::FailedAssay => Some(UnavailableLegStatus::FailedAssay),
            Self::NotAssessed => Some(UnavailableLegStatus::NotAssessed),
            Self::Disputed => Some(UnavailableLegStatus::Disputed),
        }
    }
}

/// One synchronized, half-open interval [start_tick, end_tick).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct JointStateInterval {
    pub start_tick: u64,
    pub end_tick: u64,
    pub legs: [LegSampleState; JOINT_LEG_COUNT],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JointStateError {
    EmptyIntervals,
    InvalidInterval,
    OverlappingOrUnorderedIntervals,
    ArithmeticOverflow,
    InvalidStateMask,
    NoCompleteObservation,
}

impl fmt::Display for JointStateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyIntervals => formatter.write_str("no joint-state intervals supplied"),
            Self::InvalidInterval => formatter.write_str("interval must have positive duration"),
            Self::OverlappingOrUnorderedIntervals => {
                formatter.write_str("joint-state intervals overlap or are not ordered")
            }
            Self::ArithmeticOverflow => formatter.write_str("joint-state tick/count overflow"),
            Self::InvalidStateMask => {
                formatter.write_str("joint-state mask contains bits outside the six-leg domain")
            }
            Self::NoCompleteObservation => {
                formatter.write_str("no fully observed six-leg interval is available")
            }
        }
    }
}

impl std::error::Error for JointStateError {}

/// Exact reduced probability mass. No floating-point conversion is performed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExactMass {
    pub numerator: u64,
    pub denominator: u64,
}

fn gcd(mut left: u64, mut right: u64) -> u64 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left.max(1)
}

fn checked_add(target: &mut u64, value: u64) -> Result<(), JointStateError> {
    *target = target
        .checked_add(value)
        .ok_or(JointStateError::ArithmeticOverflow)?;
    Ok(())
}

fn is_fully_observed(states: &[LegSampleState; JOINT_LEG_COUNT]) -> bool {
    states.iter().all(|state| state.unavailable_status().is_none())
}

/// Encode six states to a stance mask and an observed-leg mask.
///
/// The stance mask is meaningful as a full joint state only when the observed
/// mask equals COMPLETE_LEG_MASK. Unknown legs are never interpreted as swing.
pub fn encode_joint_state(
    states: &[LegSampleState; JOINT_LEG_COUNT],
) -> (u8, u8) {
    let mut stance_mask = 0_u8;
    let mut observed_mask = 0_u8;
    for (index, state) in states.iter().copied().enumerate() {
        match state {
            LegSampleState::Stance => {
                stance_mask |= 1 << index;
                observed_mask |= 1 << index;
            }
            LegSampleState::Swing => observed_mask |= 1 << index,
            _ => {}
        }
    }
    (stance_mask, observed_mask)
}

/// Decode a complete stance mask. Bits are mapped according to JOINT_LEG_ORDER.
pub fn decode_joint_state(mask: u8) -> Result<[LegSampleState; JOINT_LEG_COUNT], JointStateError> {
    if mask & !COMPLETE_LEG_MASK != 0 {
        return Err(JointStateError::InvalidStateMask);
    }
    Ok(std::array::from_fn(|index| {
        if mask & (1 << index) != 0 {
            LegSampleState::Stance
        } else {
            LegSampleState::Swing
        }
    }))
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JointStateReport {
    /// Exact duration of each of the 64 fully observed six-leg configurations.
    pub joint_state_ticks: [u64; JOINT_STATE_COUNT],
    /// Exact duration grouped by the number of legs simultaneously in stance.
    pub count_by_stance_legs_ticks: [u64; JOINT_LEG_COUNT + 1],
    /// Exact duration in stance for each canonical leg.
    pub per_leg_stance_ticks: [u64; JOINT_LEG_COUNT],
    /// Exact simultaneous-stance duration for each ordered leg pair.
    pub pairwise_stance_ticks: [[u64; JOINT_LEG_COUNT]; JOINT_LEG_COUNT],
    /// Both conventional tripod groups, kept separate.
    pub tripod_swing_overlap_ticks: [u64; 2],
    pub tripod_stance_overlap_ticks: [u64; 2],
    pub total_fully_observed_ticks: u64,
    pub total_incomplete_ticks: u64,
    /// Incomplete duration indexed by observed-leg mask; this retains which
    /// subset of legs was visible for each excluded interval.
    pub incomplete_ticks_by_observed_mask: [u64; JOINT_STATE_COUNT],
    /// Unavailable duration per leg and typed unavailable status.
    pub unavailable_ticks_by_leg_status: [[u64; 5]; JOINT_LEG_COUNT],
    /// Unrecorded time gaps between input intervals; no transition crosses one.
    pub observed_gap_ticks: u64,
    /// Directed changes between different states at contiguous fully observed
    /// interval boundaries. Equal-state boundaries are merged into dwell.
    pub transition_counts: [[u64; JOINT_STATE_COUNT]; JOINT_STATE_COUNT],
    /// Total duration in each state, equal to joint_state_ticks by construction.
    pub dwell_ticks_by_state: [u64; JOINT_STATE_COUNT],
    /// Number of maximal contiguous episodes spent in each state.
    pub dwell_episode_count_by_state: [u64; JOINT_STATE_COUNT],
}

impl JointStateReport {
    /// Return the exact duration mass for one complete stance mask.
    pub fn state_mass(&self, mask: u8) -> Result<ExactMass, JointStateError> {
        if mask & !COMPLETE_LEG_MASK != 0 {
            return Err(JointStateError::InvalidStateMask);
        }
        if self.total_fully_observed_ticks == 0 {
            return Err(JointStateError::NoCompleteObservation);
        }
        let numerator = self.joint_state_ticks[mask as usize];
        let divisor = gcd(numerator, self.total_fully_observed_ticks);
        Ok(ExactMass {
            numerator: numerator / divisor,
            denominator: self.total_fully_observed_ticks / divisor,
        })
    }

    /// Return the exact duration mass for K simultaneously stance legs.
    pub fn stance_count_mass(&self, count: usize) -> Result<ExactMass, JointStateError> {
        if count > JOINT_LEG_COUNT {
            return Err(JointStateError::InvalidStateMask);
        }
        if self.total_fully_observed_ticks == 0 {
            return Err(JointStateError::NoCompleteObservation);
        }
        let numerator = self.count_by_stance_legs_ticks[count];
        let divisor = gcd(numerator, self.total_fully_observed_ticks);
        Ok(ExactMass {
            numerator: numerator / divisor,
            denominator: self.total_fully_observed_ticks / divisor,
        })
    }
}

fn tripod_mask(group: [usize; 3]) -> u8 {
    group.iter().fold(0_u8, |mask, index| mask | (1 << index))
}

/// Compute one trial's duration-weighted joint six-leg configuration and
/// contiguous state-transition/dwell summaries.
///
/// Intervals are half-open and must be ordered without overlap. Gaps are
/// retained as observed_gap_ticks and break transition continuity. An interval
/// contributes to the 64-state distribution only when all six legs are either
/// STANCE or SWING. Incomplete intervals are counted by unavailable status and
/// observed-leg mask; no transition is inferred across them.
pub fn joint_state_report(
    intervals: &[JointStateInterval],
) -> Result<JointStateReport, JointStateError> {
    if intervals.is_empty() {
        return Err(JointStateError::EmptyIntervals);
    }

    let mut report = JointStateReport {
        joint_state_ticks: [0; JOINT_STATE_COUNT],
        count_by_stance_legs_ticks: [0; JOINT_LEG_COUNT + 1],
        per_leg_stance_ticks: [0; JOINT_LEG_COUNT],
        pairwise_stance_ticks: [[0; JOINT_LEG_COUNT]; JOINT_LEG_COUNT],
        tripod_swing_overlap_ticks: [0; 2],
        tripod_stance_overlap_ticks: [0; 2],
        total_fully_observed_ticks: 0,
        total_incomplete_ticks: 0,
        incomplete_ticks_by_observed_mask: [0; JOINT_STATE_COUNT],
        unavailable_ticks_by_leg_status: [[0; 5]; JOINT_LEG_COUNT],
        observed_gap_ticks: 0,
        transition_counts: [[0; JOINT_STATE_COUNT]; JOINT_STATE_COUNT],
        dwell_ticks_by_state: [0; JOINT_STATE_COUNT],
        dwell_episode_count_by_state: [0; JOINT_STATE_COUNT],
    };

    let group_masks = [tripod_mask(TRIPOD_GROUPS[0]), tripod_mask(TRIPOD_GROUPS[1])];
    let mut previous_interval_end: Option<u64> = None;
    let mut previous_complete_state: Option<(u64, u8)> = None;

    for interval in intervals {
        if interval.end_tick <= interval.start_tick {
            return Err(JointStateError::InvalidInterval);
        }
        if let Some(previous_end) = previous_interval_end {
            if interval.start_tick < previous_end {
                return Err(JointStateError::OverlappingOrUnorderedIntervals);
            }
            if interval.start_tick > previous_end {
                checked_add(
                    &mut report.observed_gap_ticks,
                    interval.start_tick - previous_end,
                )?;
                previous_complete_state = None;
            }
        }

        let duration = interval.end_tick - interval.start_tick;
        let (stance_mask, observed_mask) = encode_joint_state(&interval.legs);

        if !is_fully_observed(&interval.legs) {
            checked_add(&mut report.total_incomplete_ticks, duration)?;
            checked_add(
                &mut report.incomplete_ticks_by_observed_mask[observed_mask as usize],
                duration,
            )?;
            for (leg_index, state) in interval.legs.iter().copied().enumerate() {
                if let Some(status) = state.unavailable_status() {
                    checked_add(
                        &mut report.unavailable_ticks_by_leg_status[leg_index][status.index()],
                        duration,
                    )?;
                }
            }
            previous_complete_state = None;
            previous_interval_end = Some(interval.end_tick);
            continue;
        }

        let state_index = stance_mask as usize;
        checked_add(&mut report.joint_state_ticks[state_index], duration)?;
        checked_add(
            &mut report.total_fully_observed_ticks,
            duration,
        )?;
        checked_add(
            &mut report.count_by_stance_legs_ticks[stance_mask.count_ones() as usize],
            duration,
        )?;

        for leg_index in 0..JOINT_LEG_COUNT {
            if stance_mask & (1 << leg_index) != 0 {
                checked_add(&mut report.per_leg_stance_ticks[leg_index], duration)?;
            }
            for other_index in 0..JOINT_LEG_COUNT {
                if stance_mask & (1 << leg_index) != 0
                    && stance_mask & (1 << other_index) != 0
                {
                    checked_add(
                        &mut report.pairwise_stance_ticks[leg_index][other_index],
                        duration,
                    )?;
                }
            }
        }

        for (group_index, group_mask) in group_masks.iter().copied().enumerate() {
            if stance_mask & group_mask == 0 {
                checked_add(
                    &mut report.tripod_swing_overlap_ticks[group_index],
                    duration,
                )?;
            }
            if stance_mask & group_mask == group_mask {
                checked_add(
                    &mut report.tripod_stance_overlap_ticks[group_index],
                    duration,
                )?;
            }
        }

        checked_add(&mut report.dwell_ticks_by_state[state_index], duration)?;
        match previous_complete_state {
            Some((previous_end, previous_mask)) if previous_end == interval.start_tick => {
                if previous_mask != stance_mask {
                    checked_add(
                        &mut report.transition_counts[previous_mask as usize][state_index],
                        1,
                    )?;
                    checked_add(&mut report.dwell_episode_count_by_state[state_index], 1)?;
                }
            }
            _ => {
                checked_add(&mut report.dwell_episode_count_by_state[state_index], 1)?;
            }
        }

        previous_complete_state = Some((interval.end_tick, stance_mask));
        previous_interval_end = Some(interval.end_tick);
    }

    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mask_interval(mask: u8, start_tick: u64) -> JointStateInterval {
        JointStateInterval {
            start_tick,
            end_tick: start_tick + 1,
            legs: decode_joint_state(mask).unwrap(),
        }
    }

    fn parity_intervals(parity: u32) -> Vec<JointStateInterval> {
        (0_u8..64)
            .filter(|mask| mask.count_ones() % 2 == parity)
            .enumerate()
            .map(|(index, mask)| mask_interval(mask, index as u64))
            .collect()
    }

    #[test]
    fn exact_parity_worlds_tie_on_every_lower_order_projection_but_not_joint_state() {
        let even = joint_state_report(&parity_intervals(0)).unwrap();
        let odd = joint_state_report(&parity_intervals(1)).unwrap();

        assert_eq!(even.total_fully_observed_ticks, 32);
        assert_eq!(odd.total_fully_observed_ticks, 32);
        assert_eq!(even.per_leg_stance_ticks, [16; JOINT_LEG_COUNT]);
        assert_eq!(odd.per_leg_stance_ticks, [16; JOINT_LEG_COUNT]);
        assert_eq!(even.pairwise_stance_ticks, odd.pairwise_stance_ticks);
        assert_eq!(even.tripod_stance_overlap_ticks, [4; 2]);
        assert_eq!(odd.tripod_stance_overlap_ticks, [4; 2]);
        assert_eq!(even.tripod_swing_overlap_ticks, [4; 2]);
        assert_eq!(odd.tripod_swing_overlap_ticks, [4; 2]);

        assert_ne!(even.joint_state_ticks, odd.joint_state_ticks);
        assert_eq!(even.count_by_stance_legs_ticks, [1, 0, 15, 0, 15, 0, 1]);
        assert_eq!(odd.count_by_stance_legs_ticks, [0, 6, 0, 20, 0, 6, 0]);
        assert_eq!(even.state_mass(0).unwrap(), ExactMass { numerator: 1, denominator: 32 });
        assert_eq!(odd.state_mass(0).unwrap(), ExactMass { numerator: 0, denominator: 1 });
    }

    #[test]
    fn occupancy_can_tie_while_ordered_transition_structure_differs() {
        let forward_masks = [0_u8, 1, 2, 0];
        let reverse_masks = [0_u8, 2, 1, 0];
        let forward: Vec<_> = forward_masks
            .iter()
            .enumerate()
            .map(|(index, mask)| mask_interval(*mask, index as u64))
            .collect();
        let reverse: Vec<_> = reverse_masks
            .iter()
            .enumerate()
            .map(|(index, mask)| mask_interval(*mask, index as u64))
            .collect();

        let forward_report = joint_state_report(&forward).unwrap();
        let reverse_report = joint_state_report(&reverse).unwrap();
        assert_eq!(forward_report.joint_state_ticks, reverse_report.joint_state_ticks);
        assert_ne!(forward_report.transition_counts, reverse_report.transition_counts);
        assert_eq!(forward_report.dwell_ticks_by_state, reverse_report.dwell_ticks_by_state);
    }

    #[test]
    fn adjacent_same_state_intervals_merge_into_one_dwell_episode() {
        let intervals = [
            mask_interval(0b000011, 0),
            mask_interval(0b000011, 1),
            mask_interval(0b000001, 2),
        ];
        let report = joint_state_report(&intervals).unwrap();
        assert_eq!(report.dwell_ticks_by_state[0b000011], 2);
        assert_eq!(report.dwell_episode_count_by_state[0b000011], 1);
        assert_eq!(report.transition_counts[0b000011][0b000001], 1);
    }

    #[test]
    fn incomplete_observations_are_excluded_without_swing_imputation() {
        let interval = JointStateInterval {
            start_tick: 0,
            end_tick: 5,
            legs: [
                LegSampleState::Stance,
                LegSampleState::Swing,
                LegSampleState::MissingTrack,
                LegSampleState::OutOfSupport,
                LegSampleState::FailedAssay,
                LegSampleState::NotAssessed,
            ],
        };
        let report = joint_state_report(&[interval]).unwrap();
        assert_eq!(report.total_fully_observed_ticks, 0);
        assert_eq!(report.total_incomplete_ticks, 5);
        assert_eq!(report.incomplete_ticks_by_observed_mask[0b000011], 5);
        assert_eq!(report.unavailable_ticks_by_leg_status[2][0], 5);
        assert_eq!(report.unavailable_ticks_by_leg_status[3][1], 5);
        assert_eq!(report.unavailable_ticks_by_leg_status[4][2], 5);
        assert_eq!(report.unavailable_ticks_by_leg_status[5][3], 5);
        assert!(report.state_mass(0).is_err());
    }

    #[test]
    fn transition_is_not_inferred_across_an_incomplete_interval() {
        let intervals = [
            mask_interval(0b000001, 0),
            JointStateInterval {
                start_tick: 1,
                end_tick: 2,
                legs: [
                    LegSampleState::Stance,
                    LegSampleState::MissingTrack,
                    LegSampleState::Swing,
                    LegSampleState::Swing,
                    LegSampleState::Swing,
                    LegSampleState::Swing,
                ],
            },
            mask_interval(0b000010, 2),
        ];
        let report = joint_state_report(&intervals).unwrap();
        assert_eq!(report.transition_counts[0b000001][0b000010], 0);
    }

    #[test]
    fn transition_is_not_inferred_across_a_time_gap() {
        let intervals = [mask_interval(0b000001, 0), mask_interval(0b000010, 3)];
        let report = joint_state_report(&intervals).unwrap();
        assert_eq!(report.observed_gap_ticks, 2);
        assert_eq!(report.transition_counts[0b000001][0b000010], 0);
        assert_eq!(report.dwell_episode_count_by_state[0b000001], 1);
        assert_eq!(report.dwell_episode_count_by_state[0b000010], 1);
    }

    #[test]
    fn rejects_zero_width_and_reversed_intervals() {
        let zero = JointStateInterval {
            start_tick: 5,
            end_tick: 5,
            legs: decode_joint_state(0).unwrap(),
        };
        let reversed = JointStateInterval {
            start_tick: 8,
            end_tick: 7,
            legs: decode_joint_state(0).unwrap(),
        };
        assert_eq!(joint_state_report(&[zero]), Err(JointStateError::InvalidInterval));
        assert_eq!(joint_state_report(&[reversed]), Err(JointStateError::InvalidInterval));
    }

    #[test]
    fn rejects_overlapping_or_unordered_intervals() {
        let intervals = [mask_interval(0, 0), mask_interval(1, 0)];
        assert_eq!(
            joint_state_report(&intervals),
            Err(JointStateError::OverlappingOrUnorderedIntervals)
        );
    }

    #[test]
    fn full_mask_mapping_is_canonical_and_stable() {
        assert_eq!(decode_joint_state(0).unwrap(), [LegSampleState::Swing; 6]);
        assert_eq!(
            encode_joint_state(&decode_joint_state(0b10_1001).unwrap()),
            (0b10_1001, COMPLETE_LEG_MASK)
        );
        assert_eq!(decode_joint_state(0b100_0000), Err(JointStateError::InvalidStateMask));
    }

    #[test]
    fn exact_mass_is_reduced_and_never_uses_float_rounding() {
        let report = joint_state_report(&[
            JointStateInterval {
                start_tick: 0,
                end_tick: 2,
                legs: decode_joint_state(0).unwrap(),
            },
            JointStateInterval {
                start_tick: 2,
                end_tick: 6,
                legs: decode_joint_state(1).unwrap(),
            },
        ])
        .unwrap();
        assert_eq!(report.state_mass(0).unwrap(), ExactMass { numerator: 1, denominator: 3 });
        assert_eq!(report.state_mass(1).unwrap(), ExactMass { numerator: 2, denominator: 3 });
    }

    #[test]
    fn all_complete_state_histograms_sum_to_the_complete_duration() {
        let report = joint_state_report(&parity_intervals(0)).unwrap();
        assert_eq!(
            report.joint_state_ticks.iter().sum::<u64>(),
            report.total_fully_observed_ticks
        );
        assert_eq!(
            report.count_by_stance_legs_ticks.iter().sum::<u64>(),
            report.total_fully_observed_ticks
        );
        assert_eq!(report.dwell_ticks_by_state, report.joint_state_ticks);
    }
}
