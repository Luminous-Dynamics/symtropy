//! Axiswise viability-reserve threshold bracketing.
//!
//! This package consumes independently computed viability/recovery outcomes
//! over an explicitly ordered uncertainty ladder. It does not construct the
//! uncertainty sets, generate transitions, compute a viability kernel,
//! validate physical nestedness, or certify a robot.
//!
//! A formal bracket is emitted only when the caller declares the family nested
//! and the supplied outcomes satisfy the expected monotonicity. A rebound
//! (nonviable -> viable at a higher uncertainty level) is reported as a
//! violation; samples are never smoothed or silently repaired.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LadderError {
    EmptyAxisId,
    EmptyUnit,
    EmptyModelDigest,
    EmptySafeSetDigest,
    EmptyActionSetDigest,
    EmptyOracleVersion,
    EmptySampleSet,
    LevelNotStrictlyIncreasing { index: usize },
    MissingRecoveryTargetDigest,
    UnexpectedRecoveryTargetDigest,
    EmptyRecoveryTargetDigest,
}

/// One independently evaluated uncertainty level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LadderSample {
    /// Integer in the axis's declared unit; no interpolation or conversion.
    pub level: u64,
    pub invariant_viable: bool,
    /// Guaranteed reach-to-recovery result for the declared target, if
    /// evaluated independently at this level.
    pub recovery_viable: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AxisLadder {
    pub axis_id: String,
    pub unit: String,
    pub model_digest: String,
    pub safe_set_digest: String,
    pub action_set_digest: String,
    pub oracle_version: String,
    pub nested_family_declared: bool,
    pub recovery_target_digest: Option<String>,
    /// Samples must be strictly increasing by level; this order is preserved.
    pub samples: Vec<LadderSample>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MonotonicityStatus {
    NotDeclared,
    Monotone,
    /// Indices identify samples where a previously false outcome became true
    /// at a higher uncertainty level.
    Violated { rebound_sample_indices: Vec<usize> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BracketQualification {
    Qualified,
    UnqualifiedFamilyNotDeclaredNested,
    UnqualifiedMonotonicityViolation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BracketShape {
    /// Every tested level remains viable. Loss is right-censored above max.
    AllTestedLevelsViable { maximum_tested_level: u64 },
    /// Even the minimum tested level is nonviable. Threshold is left-censored.
    AllTestedLevelsNonViable { minimum_tested_level: u64 },
    /// The last passing and first failing tested levels bound the transition.
    Bounded {
        largest_viable_level: u64,
        smallest_nonviable_level: u64,
        largest_viable_sample_index: usize,
        smallest_nonviable_sample_index: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReserveBracket {
    pub qualification: BracketQualification,
    pub monotonicity: MonotonicityStatus,
    /// Present only for a qualified nested monotone ladder.
    pub shape: Option<BracketShape>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViabilityReserveReport {
    pub axis_id: String,
    pub unit: String,
    pub model_digest: String,
    pub safe_set_digest: String,
    pub action_set_digest: String,
    pub oracle_version: String,
    pub nested_family_declared: bool,
    pub recovery_target_digest: Option<String>,
    pub samples: Vec<LadderSample>,
    pub invariant_viability: ReserveBracket,
    pub guaranteed_recovery: Option<ReserveBracket>,
}

fn nonblank(value: &str) -> bool {
    !value.trim().is_empty()
}

fn find_rebound_indices(values: &[bool]) -> Vec<usize> {
    let mut has_seen_nonviable = false;
    let mut rebounds = Vec::new();

    for (index, viable) in values.iter().copied().enumerate() {
        if !viable {
            has_seen_nonviable = true;
        } else if has_seen_nonviable {
            rebounds.push(index);
        }
    }

    rebounds
}

fn make_bracket(values: &[bool], levels: &[u64], nested: bool) -> ReserveBracket {
    if !nested {
        return ReserveBracket {
            qualification: BracketQualification::UnqualifiedFamilyNotDeclaredNested,
            monotonicity: MonotonicityStatus::NotDeclared,
            shape: None,
        };
    }

    let rebounds = find_rebound_indices(values);
    if !rebounds.is_empty() {
        return ReserveBracket {
            qualification: BracketQualification::UnqualifiedMonotonicityViolation,
            monotonicity: MonotonicityStatus::Violated {
                rebound_sample_indices: rebounds,
            },
            shape: None,
        };
    }

    let shape = if values.iter().all(|value| *value) {
        BracketShape::AllTestedLevelsViable {
            maximum_tested_level: levels[levels.len() - 1],
        }
    } else if values.iter().all(|value| !*value) {
        BracketShape::AllTestedLevelsNonViable {
            minimum_tested_level: levels[0],
        }
    } else {
        let first_nonviable = values
            .iter()
            .position(|value| !*value)
            .expect("mixed sequence contains a nonviable sample");
        let largest_viable = first_nonviable - 1;
        BracketShape::Bounded {
            largest_viable_level: levels[largest_viable],
            smallest_nonviable_level: levels[first_nonviable],
            largest_viable_sample_index: largest_viable,
            smallest_nonviable_sample_index: first_nonviable,
        }
    };

    ReserveBracket {
        qualification: BracketQualification::Qualified,
        monotonicity: MonotonicityStatus::Monotone,
        shape: Some(shape),
    }
}

/// Analyze one uncertainty axis without interpolation.
///
/// If the family is not declared nested, or a monotonicity violation occurs,
/// all input samples are preserved but no qualified threshold shape is emitted.
pub fn analyze_axis_ladder(
    ladder: &AxisLadder,
) -> Result<ViabilityReserveReport, LadderError> {
    if !nonblank(&ladder.axis_id) {
        return Err(LadderError::EmptyAxisId);
    }
    if !nonblank(&ladder.unit) {
        return Err(LadderError::EmptyUnit);
    }
    if !nonblank(&ladder.model_digest) {
        return Err(LadderError::EmptyModelDigest);
    }
    if !nonblank(&ladder.safe_set_digest) {
        return Err(LadderError::EmptySafeSetDigest);
    }
    if !nonblank(&ladder.action_set_digest) {
        return Err(LadderError::EmptyActionSetDigest);
    }
    if !nonblank(&ladder.oracle_version) {
        return Err(LadderError::EmptyOracleVersion);
    }
    if ladder.samples.is_empty() {
        return Err(LadderError::EmptySampleSet);
    }

    for index in 1..ladder.samples.len() {
        if ladder.samples[index - 1].level >= ladder.samples[index].level {
            return Err(LadderError::LevelNotStrictlyIncreasing { index });
        }
    }

    let has_recovery = ladder
        .samples
        .iter()
        .any(|sample| sample.recovery_viable.is_some());
    let all_recovery = ladder
        .samples
        .iter()
        .all(|sample| sample.recovery_viable.is_some());

    match (&ladder.recovery_target_digest, has_recovery, all_recovery) {
        (None, true, _) => return Err(LadderError::MissingRecoveryTargetDigest),
        (Some(digest), false, _) if !nonblank(digest) => {
            return Err(LadderError::EmptyRecoveryTargetDigest);
        }
        (Some(_), false, _) => return Err(LadderError::UnexpectedRecoveryTargetDigest),
        (Some(digest), true, _) if !nonblank(digest) => {
            return Err(LadderError::EmptyRecoveryTargetDigest);
        }
        (Some(_), true, false) => return Err(LadderError::UnexpectedRecoveryTargetDigest),
        _ => {}
    }

    let levels = ladder
        .samples
        .iter()
        .map(|sample| sample.level)
        .collect::<Vec<_>>();
    let invariant_values = ladder
        .samples
        .iter()
        .map(|sample| sample.invariant_viable)
        .collect::<Vec<_>>();

    let invariant_viability =
        make_bracket(&invariant_values, &levels, ladder.nested_family_declared);

    let guaranteed_recovery = if has_recovery {
        let recovery_values = ladder
            .samples
            .iter()
            .map(|sample| sample.recovery_viable.unwrap_or(false))
            .collect::<Vec<_>>();
        Some(make_bracket(
            &recovery_values,
            &levels,
            ladder.nested_family_declared,
        ))
    } else {
        None
    };

    Ok(ViabilityReserveReport {
        axis_id: ladder.axis_id.clone(),
        unit: ladder.unit.clone(),
        model_digest: ladder.model_digest.clone(),
        safe_set_digest: ladder.safe_set_digest.clone(),
        action_set_digest: ladder.action_set_digest.clone(),
        oracle_version: ladder.oracle_version.clone(),
        nested_family_declared: ladder.nested_family_declared,
        recovery_target_digest: ladder.recovery_target_digest.clone(),
        samples: ladder.samples.clone(),
        invariant_viability,
        guaranteed_recovery,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ladder(samples: &[(u64, bool)]) -> AxisLadder {
        AxisLadder {
            axis_id: "contact-friction".to_owned(),
            unit: "milli-coefficient".to_owned(),
            model_digest: "model-digest-01".to_owned(),
            safe_set_digest: "safe-set-01".to_owned(),
            action_set_digest: "action-set-01".to_owned(),
            oracle_version: "loco-robust-viability-v0.1".to_owned(),
            nested_family_declared: true,
            recovery_target_digest: None,
            samples: samples
                .iter()
                .map(|(level, viable)| LadderSample {
                    level: *level,
                    invariant_viable: *viable,
                    recovery_viable: None,
                })
                .collect(),
        }
    }

    #[test]
    fn mixed_monotone_ladder_returns_exact_tested_bracket() {
        let input = ladder(&[(100, true), (200, true), (300, false), (400, false)]);
        let report = analyze_axis_ladder(&input).unwrap();
        assert_eq!(
            report.invariant_viability.shape,
            Some(BracketShape::Bounded {
                largest_viable_level: 200,
                smallest_nonviable_level: 300,
                largest_viable_sample_index: 1,
                smallest_nonviable_sample_index: 2,
            })
        );
        assert_eq!(
            report.invariant_viability.qualification,
            BracketQualification::Qualified
        );
        assert_eq!(
            report.invariant_viability.monotonicity,
            MonotonicityStatus::Monotone
        );
    }

    #[test]
    fn all_viable_ladder_is_right_censored_not_interpolated() {
        let input = ladder(&[(10, true), (20, true), (30, true)]);
        let report = analyze_axis_ladder(&input).unwrap();
        assert_eq!(
            report.invariant_viability.shape,
            Some(BracketShape::AllTestedLevelsViable {
                maximum_tested_level: 30
            })
        );
    }

    #[test]
    fn all_nonviable_ladder_is_left_censored() {
        let input = ladder(&[(10, false), (20, false), (30, false)]);
        let report = analyze_axis_ladder(&input).unwrap();
        assert_eq!(
            report.invariant_viability.shape,
            Some(BracketShape::AllTestedLevelsNonViable {
                minimum_tested_level: 10
            })
        );
    }

    #[test]
    fn rebound_after_failure_is_reported_without_formal_bracket() {
        let input = ladder(&[(10, true), (20, false), (30, true), (40, false)]);
        let report = analyze_axis_ladder(&input).unwrap();
        assert_eq!(
            report.invariant_viability.monotonicity,
            MonotonicityStatus::Violated {
                rebound_sample_indices: vec![2]
            }
        );
        assert_eq!(
            report.invariant_viability.qualification,
            BracketQualification::UnqualifiedMonotonicityViolation
        );
        assert_eq!(report.invariant_viability.shape, None);
        assert_eq!(report.samples, input.samples);
    }

    #[test]
    fn multiple_rebounds_are_all_reported() {
        let input = ladder(&[
            (10, true),
            (20, false),
            (30, true),
            (40, false),
            (50, true),
        ]);
        let report = analyze_axis_ladder(&input).unwrap();
        assert_eq!(
            report.invariant_viability.monotonicity,
            MonotonicityStatus::Violated {
                rebound_sample_indices: vec![2, 4]
            }
        );
    }

    #[test]
    fn undeclared_nested_family_never_gets_a_formal_bracket() {
        let mut input = ladder(&[(10, true), (20, false)]);
        input.nested_family_declared = false;
        let report = analyze_axis_ladder(&input).unwrap();
        assert_eq!(
            report.invariant_viability.qualification,
            BracketQualification::UnqualifiedFamilyNotDeclaredNested
        );
        assert_eq!(
            report.invariant_viability.monotonicity,
            MonotonicityStatus::NotDeclared
        );
        assert_eq!(report.invariant_viability.shape, None);
    }

    #[test]
    fn non_increasing_levels_are_rejected() {
        let input = ladder(&[(10, true), (10, false)]);
        assert_eq!(
            analyze_axis_ladder(&input),
            Err(LadderError::LevelNotStrictlyIncreasing { index: 1 })
        );
    }

    #[test]
    fn decreasing_levels_are_rejected_without_sorting() {
        let input = ladder(&[(20, true), (10, false)]);
        assert_eq!(
            analyze_axis_ladder(&input),
            Err(LadderError::LevelNotStrictlyIncreasing { index: 1 })
        );
    }

    #[test]
    fn empty_samples_are_rejected() {
        let mut input = ladder(&[(10, true)]);
        input.samples.clear();
        assert_eq!(analyze_axis_ladder(&input), Err(LadderError::EmptySampleSet));
    }

    #[test]
    fn identity_fields_are_required() {
        let mut input = ladder(&[(10, true)]);
        input.model_digest.clear();
        assert_eq!(analyze_axis_ladder(&input), Err(LadderError::EmptyModelDigest));

        let mut input = ladder(&[(10, true)]);
        input.axis_id = "  ".to_owned();
        assert_eq!(analyze_axis_ladder(&input), Err(LadderError::EmptyAxisId));
    }

    #[test]
    fn recovery_bracket_is_separate_from_invariant_viability() {
        let mut input = ladder(&[(10, true), (20, true), (30, false), (40, false)]);
        input.recovery_target_digest = Some("recovery-target-01".to_owned());
        for (sample, recovery) in input
            .samples
            .iter_mut()
            .zip([Some(true), Some(false), Some(false), Some(false)])
        {
            sample.recovery_viable = recovery;
        }

        let report = analyze_axis_ladder(&input).unwrap();
        assert_eq!(
            report.invariant_viability.shape,
            Some(BracketShape::Bounded {
                largest_viable_level: 20,
                smallest_nonviable_level: 30,
                largest_viable_sample_index: 1,
                smallest_nonviable_sample_index: 2,
            })
        );
        assert_eq!(
            report.guaranteed_recovery.as_ref().unwrap().shape,
            Some(BracketShape::Bounded {
                largest_viable_level: 10,
                smallest_nonviable_level: 20,
                largest_viable_sample_index: 0,
                smallest_nonviable_sample_index: 1,
            })
        );
        assert_eq!(report.recovery_target_digest.as_deref(), Some("recovery-target-01"));
    }

    #[test]
    fn missing_recovery_target_identity_is_rejected() {
        let mut input = ladder(&[(10, true)]);
        input.samples[0].recovery_viable = Some(true);
        assert_eq!(
            analyze_axis_ladder(&input),
            Err(LadderError::MissingRecoveryTargetDigest)
        );
    }

    #[test]
    fn partial_recovery_samples_are_rejected() {
        let mut input = ladder(&[(10, true), (20, false)]);
        input.recovery_target_digest = Some("target".to_owned());
        input.samples[0].recovery_viable = Some(true);
        assert_eq!(
            analyze_axis_ladder(&input),
            Err(LadderError::UnexpectedRecoveryTargetDigest)
        );
    }

    #[test]
    fn recovery_only_monotonicity_violation_is_separate() {
        let mut input = ladder(&[(10, true), (20, true), (30, true)]);
        input.recovery_target_digest = Some("target".to_owned());
        input.samples[0].recovery_viable = Some(true);
        input.samples[1].recovery_viable = Some(false);
        input.samples[2].recovery_viable = Some(true);
        let report = analyze_axis_ladder(&input).unwrap();

        assert_eq!(
            report.invariant_viability.qualification,
            BracketQualification::Qualified
        );
        assert_eq!(
            report.guaranteed_recovery.as_ref().unwrap().monotonicity,
            MonotonicityStatus::Violated {
                rebound_sample_indices: vec![2]
            }
        );
        assert_eq!(report.guaranteed_recovery.as_ref().unwrap().shape, None);
    }

    #[test]
    fn identical_input_produces_identical_report_and_preserves_order() {
        let input = ladder(&[(100, true), (200, true), (300, false)]);
        let first = analyze_axis_ladder(&input).unwrap();
        let second = analyze_axis_ladder(&input).unwrap();
        assert_eq!(first, second);
        assert_eq!(
            first.samples.iter().map(|sample| sample.level).collect::<Vec<_>>(),
            vec![100, 200, 300]
        );
    }
}
