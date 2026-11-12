//! Dependency-free reference implementation of the published POMDAR task score.
//!
//! This crate computes score components from caller-supplied observations. It
//! does not parse MuJoCo output, determine task completion, establish a human
//! baseline, or qualify a simulated/physical hand.
//!
//! Published task score:
//! `0.8 * correctness + 0.2 * (human_baseline_time / system_time)`.
//!
//! Correctness is in [0, 1]. The speed ratio is intentionally unbounded, and
//! the final score may exceed 1.0. Always retain and report the components.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

/// Published POMDAR weighting for task correctness.
pub const CORRECTNESS_WEIGHT: f64 = 0.8;
/// Published POMDAR weighting for execution speed relative to the human baseline.
pub const SPEED_WEIGHT: f64 = 0.2;
/// Full-turn reference used by POMDAR continuous-rotation correctness.
pub const FULL_ROTATION_DEGREES: f64 = 360.0;

/// Discrete result for a POMDAR pure-grasping relocation task.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PureGraspOutcome {
    /// The object was not lifted from its initial support.
    NotLifted,
    /// The object was lifted but dropped during relocation.
    LiftedThenDropped,
    /// The object was grasped and successfully relocated.
    SuccessfullyRelocated,
}

/// Task-specific observation from which POMDAR correctness is derived.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TaskCorrectness {
    /// Scaffolded progress, such as notches traversed or curves cleared.
    ScaffoldedProgress {
        /// Number of task steps completed.
        completed_steps: u32,
        /// Total number of task steps in the protocol.
        total_steps: u32,
    },
    /// Achieved rotation for a continuous-rotation task; valid range is 0–360°.
    ContinuousRotation {
        /// Achieved rotation in degrees.
        achieved_degrees: f64,
    },
    /// Discrete result for a pure-grasping relocation task.
    PureGrasping(PureGraspOutcome),
}

impl TaskCorrectness {
    /// Derive the normalized POMDAR correctness component.
    pub fn normalized(self) -> Result<f64, ScoreError> {
        match self {
            Self::ScaffoldedProgress {
                completed_steps,
                total_steps,
            } => {
                if total_steps == 0 {
                    return Err(ScoreError::ZeroTaskDenominator);
                }
                if completed_steps > total_steps {
                    return Err(ScoreError::ProgressExceedsGoal {
                        completed_steps,
                        total_steps,
                    });
                }
                Ok(f64::from(completed_steps) / f64::from(total_steps))
            }
            Self::ContinuousRotation { achieved_degrees } => {
                if !achieved_degrees.is_finite()
                    || !(0.0..=FULL_ROTATION_DEGREES).contains(&achieved_degrees)
                {
                    return Err(ScoreError::InvalidRotation { achieved_degrees });
                }
                Ok(achieved_degrees / FULL_ROTATION_DEGREES)
            }
            Self::PureGrasping(outcome) => Ok(match outcome {
                PureGraspOutcome::NotLifted => 0.0,
                PureGraspOutcome::LiftedThenDropped => 0.5,
                PureGraspOutcome::SuccessfullyRelocated => 1.0,
            }),
        }
    }
}

/// Inputs needed to calculate one task's speed component.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TaskTiming {
    /// Human-reference completion time, in seconds.
    pub human_baseline_seconds: f64,
    /// Protocol-admitted system trial time, in seconds. Eligibility and timing boundaries are external.
    pub system_seconds: f64,
}

/// Auditable components of one POMDAR task score.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScoreBreakdown {
    /// Correctness in [0, 1].
    pub correctness: f64,
    /// Human baseline time divided by system time. Intentionally unbounded.
    pub speed_ratio: f64,
    /// 0.8 * correctness + 0.2 * speed_ratio; may exceed 1.0.
    pub score: f64,
}

/// Errors in caller-supplied POMDAR observations or timing data.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScoreError {
    /// A scaffolded task must declare at least one target step.
    ZeroTaskDenominator,
    /// Completed task steps cannot exceed the declared task goal.
    ProgressExceedsGoal {
        /// Observed completed steps.
        completed_steps: u32,
        /// Declared target steps.
        total_steps: u32,
    },
    /// Rotation must be finite and within the declared 0–360 degree range.
    InvalidRotation {
        /// Supplied rotation observation.
        achieved_degrees: f64,
    },
    /// The human baseline must be finite and strictly positive.
    InvalidHumanBaselineTime,
    /// System time must be finite and strictly positive.
    InvalidSystemTime,
    /// Finite time inputs produced an overflowing ratio or final score.
    NonFiniteScore,
}

/// Calculate one task's published POMDAR throughput score.
///
/// Times must use the same unit; this API uses seconds. The function neither
/// clamps the speed ratio nor rounds intermediate values. Human-baseline
/// provenance, protocol eligibility, failure/timeout timing, trial exclusions
/// and measurement boundaries remain the caller's responsibility. This
/// arithmetic oracle does not decide whether an incomplete trial is scoreable.
pub fn score_task(
    correctness: TaskCorrectness,
    timing: TaskTiming,
) -> Result<ScoreBreakdown, ScoreError> {
    let correctness = correctness.normalized()?;
    if !timing.human_baseline_seconds.is_finite() || timing.human_baseline_seconds <= 0.0 {
        return Err(ScoreError::InvalidHumanBaselineTime);
    }
    if !timing.system_seconds.is_finite() || timing.system_seconds <= 0.0 {
        return Err(ScoreError::InvalidSystemTime);
    }

    let speed_ratio = timing.human_baseline_seconds / timing.system_seconds;
    let score = CORRECTNESS_WEIGHT * correctness + SPEED_WEIGHT * speed_ratio;
    if !speed_ratio.is_finite() || !score.is_finite() {
        return Err(ScoreError::NonFiniteScore);
    }

    Ok(ScoreBreakdown {
        correctness,
        speed_ratio,
        score,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn timing(human: f64, system: f64) -> TaskTiming {
        TaskTiming {
            human_baseline_seconds: human,
            system_seconds: system,
        }
    }

    #[test]
    fn scaffolded_progress_uses_completed_fraction() {
        let result = score_task(
            TaskCorrectness::ScaffoldedProgress {
                completed_steps: 3,
                total_steps: 4,
            },
            timing(10.0, 10.0),
        )
        .unwrap();
        assert_eq!(result.correctness, 0.75);
        assert_eq!(result.speed_ratio, 1.0);
        assert!((result.score - 0.8).abs() < 1e-12);
    }

    #[test]
    fn continuous_rotation_normalizes_against_full_turn() {
        assert_eq!(
            TaskCorrectness::ContinuousRotation {
                achieved_degrees: 180.0
            }
            .normalized()
            .unwrap(),
            0.5
        );
    }

    #[test]
    fn pure_grasping_has_three_distinct_correctness_outcomes() {
        for (outcome, expected) in [
            (PureGraspOutcome::NotLifted, 0.0),
            (PureGraspOutcome::LiftedThenDropped, 0.5),
            (PureGraspOutcome::SuccessfullyRelocated, 1.0),
        ] {
            assert_eq!(
                TaskCorrectness::PureGrasping(outcome).normalized().unwrap(),
                expected
            );
        }
    }

    #[test]
    fn speed_ratio_is_not_clamped_and_score_can_exceed_one() {
        let result = score_task(
            TaskCorrectness::PureGrasping(PureGraspOutcome::SuccessfullyRelocated),
            timing(10.0, 2.0),
        )
        .unwrap();
        assert_eq!(result.correctness, 1.0);
        assert_eq!(result.speed_ratio, 5.0);
        assert!((result.score - 1.8).abs() < 1e-12);
    }

    #[test]
    fn aggregate_score_must_not_replace_correctness_component() {
        // Arithmetic-only edge vector: with C=0 and a speed ratio of 5,
        // the formula gives score=1.0. This does not assert that a failed
        // trial is eligible for a POMDAR speed time. Consumers must apply
        // the exact protocol's failure/timeout eligibility rules and report C.
        let result = score_task(
            TaskCorrectness::PureGrasping(PureGraspOutcome::NotLifted),
            timing(10.0, 2.0),
        )
        .unwrap();
        assert_eq!(result.correctness, 0.0);
        assert_eq!(result.speed_ratio, 5.0);
        assert!((result.score - 1.0).abs() < 1e-12);
    }

    #[test]
    fn rejects_zero_denominator_and_progress_beyond_goal() {
        assert_eq!(
            TaskCorrectness::ScaffoldedProgress {
                completed_steps: 0,
                total_steps: 0,
            }
            .normalized(),
            Err(ScoreError::ZeroTaskDenominator)
        );
        assert_eq!(
            TaskCorrectness::ScaffoldedProgress {
                completed_steps: 5,
                total_steps: 4,
            }
            .normalized(),
            Err(ScoreError::ProgressExceedsGoal {
                completed_steps: 5,
                total_steps: 4,
            })
        );
    }

    #[test]
    fn rejects_nonfinite_or_out_of_range_rotation() {
        for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -1.0, 360.1] {
            assert!(matches!(
                TaskCorrectness::ContinuousRotation {
                    achieved_degrees: value,
                }
                .normalized(),
                Err(ScoreError::InvalidRotation { .. })
            ));
        }
    }

    #[test]
    fn rejects_nonpositive_or_nonfinite_timings() {
        assert_eq!(
            score_task(
                TaskCorrectness::PureGrasping(PureGraspOutcome::SuccessfullyRelocated),
                timing(0.0, 1.0)
            ),
            Err(ScoreError::InvalidHumanBaselineTime)
        );
        for value in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert_eq!(
                score_task(
                    TaskCorrectness::PureGrasping(PureGraspOutcome::SuccessfullyRelocated),
                    timing(1.0, value)
                ),
                Err(ScoreError::InvalidSystemTime)
            );
        }
    }

    #[test]
    fn rejects_overflow_from_extreme_finite_timings() {
        assert_eq!(
            score_task(
                TaskCorrectness::PureGrasping(PureGraspOutcome::SuccessfullyRelocated),
                timing(f64::MAX, f64::MIN_POSITIVE)
            ),
            Err(ScoreError::NonFiniteScore)
        );
    }

    #[test]
    fn slower_than_human_keeps_partial_correctness_visible() {
        let result = score_task(
            TaskCorrectness::ScaffoldedProgress {
                completed_steps: 1,
                total_steps: 4,
            },
            timing(10.0, 20.0),
        )
        .unwrap();
        assert_eq!(result.correctness, 0.25);
        assert_eq!(result.speed_ratio, 0.5);
        assert!((result.score - 0.3).abs() < 1e-12);
    }
}
