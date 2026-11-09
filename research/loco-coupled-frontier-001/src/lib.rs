//! Finite multidimensional uncertainty/viability frontier auditor.
//!
//! This crate consumes already-qualified oracle results over a declared
//! uncertainty grid. It does not compute physical transitions or viability.

use std::collections::BTreeSet;

pub const MAX_GRID_POINTS: usize = 65_536;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Axis {
    pub axis_id: String,
    pub unit: String,
    pub levels: Vec<u32>,
    /// True only when increasing this coordinate widens a nested uncertainty
    /// set under the declared constructor semantics.
    pub nested: bool,
    pub constructor_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FamilyIdentity {
    pub initial_state_digest: String,
    pub model_digest: String,
    pub abstraction_digest: String,
    pub morphology_digest: String,
    pub controller_digest: String,
    pub action_alphabet_digest: String,
    pub solver_digest: String,
    pub task_id: String,
    pub oracle_id: String,
    pub horizon_steps: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OracleStatus {
    Viable,
    Nonviable { witness_id: String },
    Unknown { reason: String },
    Invalid { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OracleResult {
    pub result_digest: String,
    pub status: OracleStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sample {
    /// Coordinates align with FrontierFamily.axes before canonicalization.
    pub theta: Vec<u32>,
    pub viability: OracleResult,
    /// Recovery is deliberately separate from task viability.
    pub recovery: Option<OracleResult>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrontierFamily {
    pub family_id: String,
    pub identity: FamilyIdentity,
    pub axes: Vec<Axis>,
    pub samples: Vec<Sample>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Predicate {
    Viability,
    Recovery,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonotonicityViolation {
    pub predicate: Predicate,
    pub lower_theta: Vec<u32>,
    pub higher_theta: Vec<u32>,
    pub lower_witness_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PredicateFrontier {
    /// Maximal points among sampled Viable outcomes, not a continuous boundary.
    pub maximal_sampled_safe: Vec<Vec<u32>>,
    /// Minimal points among sampled Nonviable outcomes, not a continuous boundary.
    pub minimal_sampled_unsafe: Vec<Vec<u32>>,
    pub viable_sample_count: usize,
    pub nonviable_sample_count: usize,
    pub unknown_sample_count: usize,
    pub invalid_sample_count: usize,
    pub unreported_sample_count: usize,
    pub complete_for_sampled_grid: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrontierReport {
    pub family_id: String,
    /// Axes are canonicalized by stable axis_id.
    pub canonical_axis_ids: Vec<String>,
    pub sample_count: usize,
    pub expected_grid_point_count: usize,
    pub missing_grid_points: Vec<Vec<u32>>,
    pub family_qualified: bool,
    pub unqualified_reasons: Vec<String>,
    pub viability: PredicateFrontier,
    pub recovery: Option<PredicateFrontier>,
    pub monotonicity_violations: Vec<MonotonicityViolation>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrontierError {
    EmptyIdentifier,
    InvalidHorizon,
    NoAxes,
    NoSamples,
    DuplicateAxisId,
    DuplicateAxisLevel,
    EmptyAxisLevels,
    AxisLevelsNotStrictlyIncreasing,
    SampleCoordinateCardinalityMismatch,
    SampleCoordinateOutsideAxis,
    DuplicateSampleCoordinate,
    MissingOracleResultDigest,
    MissingNonviabilityWitness,
    MissingUnknownReason,
    MissingInvalidReason,
    GridSizeOverflow,
    GridTooLarge,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CanonicalSample {
    theta: Vec<u32>,
    viability: OracleResult,
    recovery: Option<OracleResult>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CanonicalizedFamily {
    axes: Vec<Axis>,
    samples: Vec<CanonicalSample>,
    expected_grid_point_count: usize,
    missing_grid_points: Vec<Vec<u32>>,
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn validate_result(result: &OracleResult) -> Result<(), FrontierError> {
    if !nonempty(&result.result_digest) {
        return Err(FrontierError::MissingOracleResultDigest);
    }
    match &result.status {
        OracleStatus::Viable => {}
        OracleStatus::Nonviable { witness_id } => {
            if !nonempty(witness_id) {
                return Err(FrontierError::MissingNonviabilityWitness);
            }
        }
        OracleStatus::Unknown { reason } => {
            if !nonempty(reason) {
                return Err(FrontierError::MissingUnknownReason);
            }
        }
        OracleStatus::Invalid { reason } => {
            if !nonempty(reason) {
                return Err(FrontierError::MissingInvalidReason);
            }
        }
    }
    Ok(())
}

fn validate_identity(identity: &FamilyIdentity) -> Result<(), FrontierError> {
    let values = [
        identity.initial_state_digest.as_str(),
        identity.model_digest.as_str(),
        identity.abstraction_digest.as_str(),
        identity.morphology_digest.as_str(),
        identity.controller_digest.as_str(),
        identity.action_alphabet_digest.as_str(),
        identity.solver_digest.as_str(),
        identity.task_id.as_str(),
        identity.oracle_id.as_str(),
    ];
    if values.iter().any(|value| !nonempty(value)) {
        return Err(FrontierError::EmptyIdentifier);
    }
    if identity.horizon_steps == 0 {
        return Err(FrontierError::InvalidHorizon);
    }
    Ok(())
}

fn validate_and_canonicalize(
    family: &FrontierFamily,
) -> Result<CanonicalizedFamily, FrontierError> {
    if !nonempty(&family.family_id) {
        return Err(FrontierError::EmptyIdentifier);
    }
    validate_identity(&family.identity)?;
    if family.axes.is_empty() {
        return Err(FrontierError::NoAxes);
    }
    if family.samples.is_empty() {
        return Err(FrontierError::NoSamples);
    }

    let mut axis_indices: Vec<usize> = (0..family.axes.len()).collect();
    axis_indices
        .sort_by(|left, right| family.axes[*left].axis_id.cmp(&family.axes[*right].axis_id));

    let mut axes = Vec::with_capacity(family.axes.len());
    let mut axis_ids = BTreeSet::new();
    for index in &axis_indices {
        let axis = &family.axes[*index];
        if !nonempty(&axis.axis_id) || !nonempty(&axis.unit) || !nonempty(&axis.constructor_digest)
        {
            return Err(FrontierError::EmptyIdentifier);
        }
        if !axis_ids.insert(axis.axis_id.as_str()) {
            return Err(FrontierError::DuplicateAxisId);
        }
        if axis.levels.is_empty() {
            return Err(FrontierError::EmptyAxisLevels);
        }
        if axis.levels.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(FrontierError::DuplicateAxisLevel);
        }
        if axis.levels.windows(2).any(|pair| pair[0] > pair[1]) {
            return Err(FrontierError::AxisLevelsNotStrictlyIncreasing);
        }
        axes.push(axis.clone());
    }

    let mut expected_grid_point_count = 1_usize;
    for axis in &axes {
        expected_grid_point_count = expected_grid_point_count
            .checked_mul(axis.levels.len())
            .ok_or(FrontierError::GridSizeOverflow)?;
    }
    if expected_grid_point_count > MAX_GRID_POINTS {
        return Err(FrontierError::GridTooLarge);
    }

    let mut samples = Vec::with_capacity(family.samples.len());
    let mut coordinates = BTreeSet::new();
    for sample in &family.samples {
        if sample.theta.len() != family.axes.len() {
            return Err(FrontierError::SampleCoordinateCardinalityMismatch);
        }

        let mut theta = Vec::with_capacity(axis_indices.len());
        for (canonical_index, original_index) in axis_indices.iter().enumerate() {
            let value = sample.theta[*original_index];
            if !axes[canonical_index].levels.contains(&value) {
                return Err(FrontierError::SampleCoordinateOutsideAxis);
            }
            theta.push(value);
        }
        if !coordinates.insert(theta.clone()) {
            return Err(FrontierError::DuplicateSampleCoordinate);
        }

        validate_result(&sample.viability)?;
        if let Some(recovery) = &sample.recovery {
            validate_result(recovery)?;
        }

        samples.push(CanonicalSample {
            theta,
            viability: sample.viability.clone(),
            recovery: sample.recovery.clone(),
        });
    }
    samples.sort_by(|left, right| left.theta.cmp(&right.theta));

    let mut all_points = Vec::with_capacity(expected_grid_point_count);
    enumerate_grid(
        &axes,
        0,
        &mut Vec::with_capacity(axes.len()),
        &mut all_points,
    );
    let missing = all_points
        .into_iter()
        .filter(|theta| !coordinates.contains(theta))
        .collect();

    Ok(CanonicalizedFamily {
        axes,
        samples,
        expected_grid_point_count,
        missing_grid_points: missing,
    })
}

fn enumerate_grid(axes: &[Axis], index: usize, current: &mut Vec<u32>, output: &mut Vec<Vec<u32>>) {
    if index == axes.len() {
        output.push(current.clone());
        return;
    }
    for level in &axes[index].levels {
        current.push(*level);
        enumerate_grid(axes, index + 1, current, output);
        let _ = current.pop();
    }
}

fn dominates_or_equals(lower: &[u32], higher: &[u32]) -> bool {
    lower.len() == higher.len()
        && lower
            .iter()
            .zip(higher)
            .all(|(lower_value, higher_value)| lower_value <= higher_value)
}

fn strictly_dominates(lower: &[u32], higher: &[u32]) -> bool {
    dominates_or_equals(lower, higher) && lower != higher
}

fn result_for(sample: &CanonicalSample, predicate: Predicate) -> Option<&OracleResult> {
    match predicate {
        Predicate::Viability => Some(&sample.viability),
        Predicate::Recovery => sample.recovery.as_ref(),
    }
}

fn monotonicity_violations(
    samples: &[CanonicalSample],
    predicate: Predicate,
) -> Vec<MonotonicityViolation> {
    let mut violations = Vec::new();
    for lower in samples {
        let Some(lower_result) = result_for(lower, predicate) else {
            continue;
        };
        let OracleStatus::Nonviable { witness_id } = &lower_result.status else {
            continue;
        };

        for higher in samples {
            if !strictly_dominates(&lower.theta, &higher.theta) {
                continue;
            }
            let Some(higher_result) = result_for(higher, predicate) else {
                continue;
            };
            if matches!(&higher_result.status, OracleStatus::Viable) {
                violations.push(MonotonicityViolation {
                    predicate,
                    lower_theta: lower.theta.clone(),
                    higher_theta: higher.theta.clone(),
                    lower_witness_id: witness_id.clone(),
                });
            }
        }
    }
    violations.sort_by(|left, right| {
        left.predicate
            .cmp(&right.predicate)
            .then_with(|| left.lower_theta.cmp(&right.lower_theta))
            .then_with(|| left.higher_theta.cmp(&right.higher_theta))
    });
    violations
}

fn compute_predicate_frontier(
    samples: &[CanonicalSample],
    predicate: Predicate,
    missing_grid_point_count: usize,
) -> Option<PredicateFrontier> {
    let reported_count = samples
        .iter()
        .filter(|sample| result_for(sample, predicate).is_some())
        .count();
    if reported_count == 0 {
        return None;
    }

    let mut viable = Vec::new();
    let mut nonviable = Vec::new();
    let mut unknown_count = 0_usize;
    let mut invalid_count = 0_usize;
    let unreported_count = samples.len() - reported_count;

    for sample in samples {
        let Some(result) = result_for(sample, predicate) else {
            continue;
        };
        match &result.status {
            OracleStatus::Viable => viable.push(sample.theta.clone()),
            OracleStatus::Nonviable { .. } => nonviable.push(sample.theta.clone()),
            OracleStatus::Unknown { .. } => unknown_count += 1,
            OracleStatus::Invalid { .. } => invalid_count += 1,
        }
    }

    let maximal_sampled_safe = viable
        .iter()
        .filter(|point| !viable.iter().any(|other| strictly_dominates(point, other)))
        .cloned()
        .collect();

    let minimal_sampled_unsafe = nonviable
        .iter()
        .filter(|point| {
            !nonviable
                .iter()
                .any(|other| strictly_dominates(other, point))
        })
        .cloned()
        .collect();

    Some(PredicateFrontier {
        maximal_sampled_safe,
        minimal_sampled_unsafe,
        viable_sample_count: viable.len(),
        nonviable_sample_count: nonviable.len(),
        unknown_sample_count: unknown_count,
        invalid_sample_count: invalid_count,
        unreported_sample_count: unreported_count,
        complete_for_sampled_grid: missing_grid_point_count == 0
            && unknown_count == 0
            && invalid_count == 0
            && unreported_count == 0,
    })
}

/// Audit a finite uncertainty grid. Unknown/Invalid samples are not classified
/// as safe or unsafe. A nonviable lower point dominated by a viable higher
/// point is a monotonicity contradiction and suppresses family qualification.
pub fn audit_frontier(family: &FrontierFamily) -> Result<FrontierReport, FrontierError> {
    let canonical = validate_and_canonicalize(family)?;
    let axes = canonical.axes;
    let samples = canonical.samples;
    let expected_grid_point_count = canonical.expected_grid_point_count;
    let missing_grid_points = canonical.missing_grid_points;

    let all_axes_nested = axes.iter().all(|axis| axis.nested);
    let mut violations = Vec::new();
    if all_axes_nested {
        violations.extend(monotonicity_violations(&samples, Predicate::Viability));
        violations.extend(monotonicity_violations(&samples, Predicate::Recovery));
    }
    violations.sort_by(|left, right| {
        left.predicate
            .cmp(&right.predicate)
            .then_with(|| left.lower_theta.cmp(&right.lower_theta))
            .then_with(|| left.higher_theta.cmp(&right.higher_theta))
    });

    let viability =
        compute_predicate_frontier(&samples, Predicate::Viability, missing_grid_points.len())
            .ok_or(FrontierError::NoSamples)?;
    let recovery =
        compute_predicate_frontier(&samples, Predicate::Recovery, missing_grid_points.len());

    let invalid_results = samples.iter().any(|sample| {
        matches!(&sample.viability.status, OracleStatus::Invalid { .. })
            || sample
                .recovery
                .as_ref()
                .is_some_and(|result| matches!(&result.status, OracleStatus::Invalid { .. }))
    });

    let mut unqualified_reasons = Vec::new();
    if !all_axes_nested {
        unqualified_reasons
            .push("one or more axes do not declare nested uncertainty sets".to_owned());
    }
    if !violations.is_empty() {
        unqualified_reasons.push("coordinatewise monotonicity contradiction detected".to_owned());
    }
    if invalid_results {
        unqualified_reasons.push("one or more oracle samples are Invalid".to_owned());
    }

    Ok(FrontierReport {
        family_id: family.family_id.clone(),
        canonical_axis_ids: axes.iter().map(|axis| axis.axis_id.clone()).collect(),
        sample_count: samples.len(),
        expected_grid_point_count,
        missing_grid_points,
        family_qualified: unqualified_reasons.is_empty(),
        unqualified_reasons,
        viability,
        recovery,
        monotonicity_violations: violations,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn axis(axis_id: &str, levels: &[u32]) -> Axis {
        Axis {
            axis_id: axis_id.to_owned(),
            unit: "severity_level".to_owned(),
            levels: levels.to_vec(),
            nested: true,
            constructor_digest: format!("constructor-{axis_id}"),
        }
    }

    fn identity() -> FamilyIdentity {
        FamilyIdentity {
            initial_state_digest: "state".to_owned(),
            model_digest: "model".to_owned(),
            abstraction_digest: "abstraction".to_owned(),
            morphology_digest: "morphology".to_owned(),
            controller_digest: "controller".to_owned(),
            action_alphabet_digest: "actions".to_owned(),
            solver_digest: "solver".to_owned(),
            task_id: "rough-terrain".to_owned(),
            oracle_id: "robust-oracle-v0".to_owned(),
            horizon_steps: 12,
        }
    }

    fn result(status: OracleStatus, digest: &str) -> OracleResult {
        OracleResult {
            result_digest: digest.to_owned(),
            status,
        }
    }

    fn viable(digest: &str) -> OracleResult {
        result(OracleStatus::Viable, digest)
    }

    fn nonviable(witness: &str, digest: &str) -> OracleResult {
        result(
            OracleStatus::Nonviable {
                witness_id: witness.to_owned(),
            },
            digest,
        )
    }

    fn sample(theta: &[u32], is_viable: bool) -> Sample {
        Sample {
            theta: theta.to_vec(),
            viability: if is_viable {
                viable("result")
            } else {
                nonviable("counterexample", "result")
            },
            recovery: None,
        }
    }

    fn family_2d(samples: Vec<Sample>) -> FrontierFamily {
        FrontierFamily {
            family_id: "family-2d".to_owned(),
            identity: identity(),
            axes: vec![axis("friction", &[0, 1, 2]), axis("impact", &[0, 1, 2])],
            samples,
        }
    }

    fn staircase_family() -> FrontierFamily {
        let mut samples = Vec::new();
        for x in 0..=2 {
            for y in 0..=2 {
                let safe = x + y <= 2;
                let recovery_safe = matches!((x, y), (0, 0) | (0, 1) | (0, 2) | (1, 0));
                let mut item = sample(&[x, y], safe);
                item.recovery = Some(if recovery_safe {
                    viable("recovery-result")
                } else {
                    nonviable("recovery-counterexample", "recovery-result")
                });
                samples.push(item);
            }
        }
        family_2d(samples)
    }

    #[test]
    fn staircase_frontier_exposes_nonseparable_interaction() {
        let report = audit_frontier(&staircase_family()).expect("valid staircase");
        assert!(report.family_qualified);
        assert!(report.viability.complete_for_sampled_grid);
        assert_eq!(
            report.viability.maximal_sampled_safe,
            vec![vec![0, 2], vec![1, 1], vec![2, 0]]
        );
        assert_eq!(
            report.viability.minimal_sampled_unsafe,
            vec![vec![1, 2], vec![2, 1]]
        );
        let recovery = report.recovery.expect("recovery predicate");
        assert_eq!(recovery.maximal_sampled_safe, vec![vec![0, 2], vec![1, 0]]);
        assert_eq!(
            recovery.minimal_sampled_unsafe,
            vec![vec![1, 1], vec![2, 0]]
        );
    }

    #[test]
    fn rectangular_control_has_one_maximal_safe_corner() {
        let mut samples = Vec::new();
        for x in 0..=2 {
            for y in 0..=2 {
                samples.push(sample(&[x, y], x <= 1 && y <= 1));
            }
        }
        let report = audit_frontier(&family_2d(samples)).expect("rectangular family");
        assert!(report.family_qualified);
        assert_eq!(report.viability.maximal_sampled_safe, vec![vec![1, 1]]);
        assert_eq!(
            report.viability.minimal_sampled_unsafe,
            vec![vec![0, 2], vec![2, 0]]
        );
    }

    #[test]
    fn monotonicity_contradiction_suppresses_qualification() {
        let family = FrontierFamily {
            family_id: "nonmonotone".to_owned(),
            identity: identity(),
            axes: vec![axis("friction", &[0, 1, 2])],
            samples: vec![
                sample(&[0], true),
                Sample {
                    theta: vec![1],
                    viability: nonviable("lower-fails", "result-1"),
                    recovery: None,
                },
                Sample {
                    theta: vec![2],
                    viability: viable("result-2"),
                    recovery: None,
                },
            ],
        };
        let report = audit_frontier(&family).expect("report contradiction");
        assert!(!report.family_qualified);
        assert_eq!(report.monotonicity_violations.len(), 1);
        assert_eq!(report.monotonicity_violations[0].lower_theta, vec![1]);
        assert_eq!(report.monotonicity_violations[0].higher_theta, vec![2]);
        assert_eq!(
            report.monotonicity_violations[0].lower_witness_id,
            "lower-fails"
        );
    }

    #[test]
    fn unknown_and_missing_grid_points_are_not_classified() {
        let family = family_2d(vec![
            sample(&[0, 0], true),
            Sample {
                theta: vec![0, 1],
                viability: result(
                    OracleStatus::Unknown {
                        reason: "oracle_timeout".to_owned(),
                    },
                    "unknown-result",
                ),
                recovery: None,
            },
            sample(&[1, 0], true),
            sample(&[1, 1], false),
            sample(&[2, 0], false),
        ]);
        let report = audit_frontier(&family).expect("partial grid");
        assert!(report.family_qualified);
        assert_eq!(
            report.missing_grid_points,
            vec![vec![0, 2], vec![1, 2], vec![2, 1], vec![2, 2]]
        );
        assert_eq!(report.viability.unknown_sample_count, 1);
        assert_eq!(report.viability.nonviable_sample_count, 2);
        assert!(!report.viability.complete_for_sampled_grid);
        assert!(!report.viability.maximal_sampled_safe.is_empty());
    }

    #[test]
    fn viability_and_recovery_frontiers_are_separate() {
        let mut samples = Vec::new();
        for (theta, viability_safe, recovery_safe) in [
            ([0, 0], true, true),
            ([0, 1], true, true),
            ([1, 0], true, true),
            ([1, 1], true, false),
            ([2, 0], true, false),
            ([0, 2], false, false),
        ] {
            let mut item = sample(&theta, viability_safe);
            item.recovery = Some(if recovery_safe {
                viable("recovery")
            } else {
                nonviable("recovery-witness", "recovery")
            });
            samples.push(item);
        }
        let report = audit_frontier(&family_2d(samples)).expect("separate predicates");
        assert!(report.family_qualified);
        assert_eq!(
            report.viability.maximal_sampled_safe,
            vec![vec![1, 1], vec![2, 0]]
        );
        let recovery = report.recovery.expect("recovery report");
        assert_eq!(recovery.maximal_sampled_safe, vec![vec![0, 1], vec![1, 0]]);
    }

    #[test]
    fn three_dimensional_antichain_matches_brute_force_fixture() {
        let axes = vec![
            axis("friction", &[0, 1]),
            axis("impact", &[0, 1]),
            axis("torque", &[0, 1]),
        ];
        let mut samples = Vec::new();
        for a in 0..=1 {
            for b in 0..=1 {
                for c in 0..=1 {
                    samples.push(sample(&[a, b, c], a + b + c <= 2));
                }
            }
        }
        let report = audit_frontier(&FrontierFamily {
            family_id: "family-3d".to_owned(),
            identity: identity(),
            axes,
            samples,
        })
        .expect("3d antichain");
        assert!(report.family_qualified);
        assert_eq!(
            report.viability.maximal_sampled_safe,
            vec![vec![0, 1, 1], vec![1, 0, 1], vec![1, 1, 0]]
        );
        assert_eq!(report.viability.minimal_sampled_unsafe, vec![vec![1, 1, 1]]);
    }

    #[test]
    fn axis_and_sample_permutations_preserve_canonical_report() {
        let original = audit_frontier(&staircase_family()).expect("original");
        let mut permuted = staircase_family();
        permuted.axes.reverse();
        for item in &mut permuted.samples {
            item.theta.reverse();
        }
        permuted.samples.reverse();
        let reordered = audit_frontier(&permuted).expect("reordered");
        assert_eq!(original, reordered);
    }

    #[test]
    fn duplicate_coordinate_and_non_nested_axis_fail_closed() {
        let mut duplicate = family_2d(vec![sample(&[0, 0], true), sample(&[0, 0], false)]);
        assert_eq!(
            audit_frontier(&duplicate),
            Err(FrontierError::DuplicateSampleCoordinate)
        );

        duplicate.samples = vec![sample(&[0, 0], true)];
        duplicate.axes[0].nested = false;
        let report = audit_frontier(&duplicate).expect("non-nested family report");
        assert!(!report.family_qualified);
        assert!(
            report
                .unqualified_reasons
                .iter()
                .any(|reason| reason.contains("nested"))
        );
    }

    #[test]
    fn invalid_oracle_result_is_not_safe_or_unsafe() {
        let family = FrontierFamily {
            family_id: "invalid-result".to_owned(),
            identity: identity(),
            axes: vec![axis("friction", &[0, 1])],
            samples: vec![
                sample(&[0], true),
                Sample {
                    theta: vec![1],
                    viability: result(
                        OracleStatus::Invalid {
                            reason: "digest mismatch".to_owned(),
                        },
                        "invalid",
                    ),
                    recovery: None,
                },
            ],
        };
        let report = audit_frontier(&family).expect("invalid sample report");
        assert!(!report.family_qualified);
        assert_eq!(report.viability.invalid_sample_count, 1);
        assert_eq!(report.viability.nonviable_sample_count, 0);
    }

    #[test]
    fn malformed_nonviability_without_witness_is_rejected() {
        let family = FrontierFamily {
            family_id: "missing-witness".to_owned(),
            identity: identity(),
            axes: vec![axis("friction", &[0])],
            samples: vec![Sample {
                theta: vec![0],
                viability: result(
                    OracleStatus::Nonviable {
                        witness_id: String::new(),
                    },
                    "result",
                ),
                recovery: None,
            }],
        };
        assert_eq!(
            audit_frontier(&family),
            Err(FrontierError::MissingNonviabilityWitness)
        );
    }

    #[test]
    fn grid_size_is_bounded_before_cartesian_enumeration() {
        let too_many = Axis {
            axis_id: "huge".to_owned(),
            unit: "level".to_owned(),
            levels: (0..=MAX_GRID_POINTS as u32).collect(),
            nested: true,
            constructor_digest: "constructor-huge".to_owned(),
        };
        let family = FrontierFamily {
            family_id: "too-large".to_owned(),
            identity: identity(),
            axes: vec![too_many],
            samples: vec![sample(&[0], true)],
        };
        assert_eq!(audit_frontier(&family), Err(FrontierError::GridTooLarge));
    }
}
