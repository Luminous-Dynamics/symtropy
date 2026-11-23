// SPDX-License-Identifier: AGPL-3.0-or-later
//! Exact small-patch oracle for a deliberately restricted 3D contact-wrench model.
//!
//! All points share +Z contact normal; total normal load is fixed to one;
//! each point has a Coulomb friction disk |t_i| <= mu * lambda_i; lambda_i >= 0;
//! no torsional spin moment is available at an individual point. This is a
//! research oracle, not a general rigid-body contact solver.

use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::error::Error;
use std::fmt::{self, Display, Formatter};

pub const SCHEMA_VERSION: &str = "wrench-support-oracle-v0.1";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OracleError {
    pub reason: String,
}

impl OracleError {
    fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
    }
}

impl Display for OracleError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.reason)
    }
}

impl Error for OracleError {}

#[derive(Clone, Debug, PartialEq)]
pub struct ContactPoint {
    pub id: String,
    /// Position relative to the explicitly chosen wrench reference origin.
    pub position: [f64; 3],
}

#[derive(Clone, Debug, PartialEq)]
pub struct WrenchDirection {
    pub id: String,
    /// Dimensionless direction [Fx, Fy, Fz, Mx, My, Mz].
    /// Moments are evaluated after division by characteristic_length.
    pub components: [f64; 6],
}

#[derive(Clone, Debug, PartialEq)]
pub struct ContactPatch {
    pub points: Vec<ContactPoint>,
    pub directions: Vec<WrenchDirection>,
    pub friction_coefficient: f64,
    /// Positive length used to normalize moments relative to force.
    pub characteristic_length: f64,
    /// Positive floor for stable normalized support-deficit reporting.
    pub support_floor: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DirectionalSupport {
    pub direction_id: String,
    pub full_patch_support: f64,
    pub retained_patch_support: f64,
    pub normalized_deficit: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WrenchSupportReport {
    pub retained_point_ids: Vec<String>,
    pub directions: Vec<DirectionalSupport>,
    pub maximum_normalized_deficit: f64,
    pub mean_normalized_deficit: f64,
    pub subsets_evaluated: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExactSelectorConfig {
    pub keep: usize,
    pub maximum_subsets: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GreedySelectorConfig {
    pub keep: usize,
    pub maximum_evaluations: u64,
}

fn is_finite_vector(values: &[f64]) -> bool {
    values.iter().all(|value| value.is_finite())
}

fn stable_norm(values: &[f64]) -> f64 {
    values
        .iter()
        .fold(0.0_f64, |norm, value| norm.hypot(*value))
}

fn validate_patch(
    patch: &ContactPatch,
) -> Result<(Vec<ContactPoint>, Vec<WrenchDirection>), OracleError> {
    if !patch.friction_coefficient.is_finite() || patch.friction_coefficient < 0.0 {
        return Err(OracleError::new(
            "friction coefficient must be finite and non-negative",
        ));
    }
    if !patch.characteristic_length.is_finite() || patch.characteristic_length <= 0.0 {
        return Err(OracleError::new(
            "characteristic length must be finite and positive",
        ));
    }
    if !patch.support_floor.is_finite() || patch.support_floor <= 0.0 {
        return Err(OracleError::new(
            "support floor must be finite and positive",
        ));
    }
    if patch.points.is_empty() {
        return Err(OracleError::new(
            "contact patch must contain at least one point",
        ));
    }
    if patch.directions.is_empty() {
        return Err(OracleError::new(
            "at least one wrench direction is required",
        ));
    }

    let mut points = patch.points.clone();
    points.sort_by(|left, right| left.id.cmp(&right.id));
    let mut point_ids = BTreeSet::new();
    for point in &points {
        if point.id.trim().is_empty() || !point_ids.insert(point.id.as_str()) {
            return Err(OracleError::new("point IDs must be non-empty and unique"));
        }
        if !is_finite_vector(&point.position) {
            return Err(OracleError::new("contact point positions must be finite"));
        }
    }

    let reference_z = points[0].position[2];
    for point in &points[1..] {
        let tolerance = 1.0e-10 * 1.0_f64.max(reference_z.abs()).max(point.position[2].abs());
        if (point.position[2] - reference_z).abs() > tolerance {
            return Err(OracleError::new(
                "all contact points must lie on one plane parallel to XY",
            ));
        }
    }

    let mut directions = patch.directions.clone();
    directions.sort_by(|left, right| left.id.cmp(&right.id));
    let mut direction_ids = BTreeSet::new();
    for direction in &directions {
        if direction.id.trim().is_empty() || !direction_ids.insert(direction.id.as_str()) {
            return Err(OracleError::new(
                "direction IDs must be non-empty and unique",
            ));
        }
        let direction_norm = stable_norm(&direction.components);
        if !is_finite_vector(&direction.components)
            || direction_norm == 0.0
            || !direction_norm.is_finite()
        {
            return Err(OracleError::new(
                "wrench directions must have finite non-zero norm",
            ));
        }
    }

    Ok((points, directions))
}

fn normalized_direction(direction: &WrenchDirection) -> [f64; 6] {
    let norm = stable_norm(&direction.components);
    std::array::from_fn(|index| direction.components[index] / norm)
}

/// Support in direction d for one point's unit-normal-load friction cone.
///
/// With n = +Z and q = d_F + (d_M x r) / L, the best tangential
/// contribution is mu * ||q_xy||. The normal contribution is q_z.
/// The total unit normal load can be assigned to whichever point maximizes this.
fn point_support(
    point: &ContactPoint,
    direction: &[f64; 6],
    friction_coefficient: f64,
    characteristic_length: f64,
) -> f64 {
    let [x, y, z] = point.position;
    let [fx, fy, fz, mx, my, mz] = *direction;
    let inverse_length = 1.0 / characteristic_length;

    let qx = fx + (my * z - mz * y) * inverse_length;
    let qy = fy + (mz * x - mx * z) * inverse_length;
    let qz = fz + (mx * y - my * x) * inverse_length;

    qz + friction_coefficient * qx.hypot(qy)
}

fn support_for_points(
    points: &[ContactPoint],
    direction: &[f64; 6],
    friction_coefficient: f64,
    characteristic_length: f64,
) -> Result<f64, OracleError> {
    let mut support = f64::NEG_INFINITY;
    for point in points {
        support = support.max(point_support(
            point,
            direction,
            friction_coefficient,
            characteristic_length,
        ));
    }
    if !support.is_finite() {
        return Err(OracleError::new(
            "support evaluation produced a non-finite value",
        ));
    }
    Ok(support)
}

fn build_report(
    patch: &ContactPatch,
    points: &[ContactPoint],
    directions: &[WrenchDirection],
    retained_indices: &[usize],
    subsets_evaluated: u64,
) -> Result<WrenchSupportReport, OracleError> {
    let retained = retained_indices
        .iter()
        .map(|index| points[*index].clone())
        .collect::<Vec<_>>();
    let mut directional = Vec::with_capacity(directions.len());
    let mut maximum_normalized_deficit = 0.0_f64;
    let mut deficit_sum = 0.0_f64;

    for direction in directions {
        let unit_direction = normalized_direction(direction);
        let full_support = support_for_points(
            points,
            &unit_direction,
            patch.friction_coefficient,
            patch.characteristic_length,
        )?;
        let retained_support = support_for_points(
            &retained,
            &unit_direction,
            patch.friction_coefficient,
            patch.characteristic_length,
        )?;
        let raw_deficit = (full_support - retained_support).max(0.0);
        let normalized_deficit = raw_deficit / full_support.abs().max(patch.support_floor);
        if !normalized_deficit.is_finite() {
            return Err(OracleError::new("normalized support deficit is non-finite"));
        }
        maximum_normalized_deficit = maximum_normalized_deficit.max(normalized_deficit);
        deficit_sum += normalized_deficit;
        directional.push(DirectionalSupport {
            direction_id: direction.id.clone(),
            full_patch_support: full_support,
            retained_patch_support: retained_support,
            normalized_deficit,
        });
    }

    let mean_normalized_deficit = deficit_sum / directions.len() as f64;
    if !mean_normalized_deficit.is_finite() {
        return Err(OracleError::new("mean support deficit is non-finite"));
    }

    Ok(WrenchSupportReport {
        retained_point_ids: retained.iter().map(|point| point.id.clone()).collect(),
        directions: directional,
        maximum_normalized_deficit,
        mean_normalized_deficit,
        subsets_evaluated,
    })
}

/// Evaluate any declared retained set against the full candidate patch.
pub fn evaluate_retained_set(
    patch: &ContactPatch,
    retained_point_ids: &[String],
) -> Result<WrenchSupportReport, OracleError> {
    let (points, directions) = validate_patch(patch)?;
    if retained_point_ids.is_empty() {
        return Err(OracleError::new("retained set must not be empty"));
    }
    let mut requested = BTreeSet::new();
    for id in retained_point_ids {
        if id.trim().is_empty() || !requested.insert(id.as_str()) {
            return Err(OracleError::new(
                "retained point IDs must be non-empty and unique",
            ));
        }
    }
    let indices = points
        .iter()
        .enumerate()
        .filter_map(|(index, point)| requested.contains(point.id.as_str()).then_some(index))
        .collect::<Vec<_>>();
    if indices.len() != requested.len() {
        return Err(OracleError::new(
            "retained set contains an unknown point ID",
        ));
    }
    build_report(patch, &points, &directions, &indices, 1)
}

fn combination_count(n: usize, k: usize) -> Result<u64, OracleError> {
    let reduced_k = k.min(n - k);
    let mut value = 1_u128;
    for index in 1..=reduced_k {
        let numerator = (n - reduced_k + index) as u128;
        let product = value
            .checked_mul(numerator)
            .ok_or_else(|| OracleError::new("combination count exceeds u128"))?;
        value = product / index as u128;
        if value > u64::MAX as u128 {
            return Err(OracleError::new("combination count exceeds u64"));
        }
    }
    Ok(value as u64)
}

fn compare_reports(left: &WrenchSupportReport, right: &WrenchSupportReport) -> Ordering {
    left.maximum_normalized_deficit
        .total_cmp(&right.maximum_normalized_deficit)
        .then_with(|| {
            left.mean_normalized_deficit
                .total_cmp(&right.mean_normalized_deficit)
        })
        .then_with(|| left.retained_point_ids.cmp(&right.retained_point_ids))
}

fn visit_combinations<F>(
    point_count: usize,
    keep: usize,
    start: usize,
    current: &mut Vec<usize>,
    visit: &mut F,
) -> Result<(), OracleError>
where
    F: FnMut(&[usize]) -> Result<(), OracleError>,
{
    if current.len() == keep {
        return visit(current);
    }
    let needed = keep - current.len();
    let final_start = point_count - needed;
    for index in start..=final_start {
        current.push(index);
        visit_combinations(point_count, keep, index + 1, current, visit)?;
        current.pop();
    }
    Ok(())
}

/// Deterministic greedy baseline. At each step it adds the point that gives
/// the best current (max deficit, mean deficit, stable-ID) tuple.
/// Unlike the exact selector, this is not globally optimal in general.
pub fn select_greedy_subset(
    patch: &ContactPatch,
    config: GreedySelectorConfig,
) -> Result<WrenchSupportReport, OracleError> {
    let (points, directions) = validate_patch(patch)?;
    if config.keep == 0 || config.keep > points.len() {
        return Err(OracleError::new(
            "keep must be in 1..=candidate point count",
        ));
    }
    if config.maximum_evaluations == 0 {
        return Err(OracleError::new("maximum_evaluations must be positive"));
    }

    let mut selected_indices = Vec::with_capacity(config.keep);
    let mut evaluation_count = 0_u64;
    let mut selected_report: Option<WrenchSupportReport> = None;

    while selected_indices.len() < config.keep {
        let mut best_candidate: Option<(WrenchSupportReport, Vec<usize>)> = None;
        for candidate_index in 0..points.len() {
            if selected_indices.contains(&candidate_index) {
                continue;
            }
            if evaluation_count >= config.maximum_evaluations {
                return Err(OracleError::new(
                    "greedy evaluation count exceeds configured maximum",
                ));
            }
            let mut trial_indices = selected_indices.clone();
            trial_indices.push(candidate_index);
            trial_indices.sort_unstable();
            let candidate_report = build_report(patch, &points, &directions, &trial_indices, 0)?;
            evaluation_count += 1;

            if best_candidate
                .as_ref()
                .is_none_or(|(incumbent, _)| compare_reports(&candidate_report, incumbent).is_lt())
            {
                best_candidate = Some((candidate_report, trial_indices));
            }
        }

        let (report, indices) = best_candidate
            .ok_or_else(|| OracleError::new("greedy selector found no remaining candidate"))?;
        selected_indices = indices;
        selected_report = Some(report);
    }

    let mut result = selected_report
        .ok_or_else(|| OracleError::new("greedy selector produced no retained set"))?;
    result.subsets_evaluated = evaluation_count;
    Ok(result)
}

/// Exhaustively selects the k-point subset minimizing worst normalized
/// directional support loss, then mean loss, then lexicographic point IDs.
/// Exact only for the declared finite direction set and restricted contact model.
pub fn select_exact_subset(
    patch: &ContactPatch,
    config: ExactSelectorConfig,
) -> Result<WrenchSupportReport, OracleError> {
    let (points, directions) = validate_patch(patch)?;
    if config.keep == 0 || config.keep > points.len() {
        return Err(OracleError::new(
            "keep must be in 1..=candidate point count",
        ));
    }
    if config.maximum_subsets == 0 {
        return Err(OracleError::new("maximum_subsets must be positive"));
    }
    let total = combination_count(points.len(), config.keep)?;
    if total > config.maximum_subsets {
        return Err(OracleError::new(format!(
            "exact search requires {total} subsets, exceeding configured maximum {}",
            config.maximum_subsets
        )));
    }

    let mut best: Option<WrenchSupportReport> = None;
    let mut current = Vec::with_capacity(config.keep);
    let mut evaluated = 0_u64;
    visit_combinations(points.len(), config.keep, 0, &mut current, &mut |indices| {
        let candidate = build_report(patch, &points, &directions, indices, 0)?;
        evaluated += 1;
        if best
            .as_ref()
            .is_none_or(|incumbent| compare_reports(&candidate, incumbent).is_lt())
        {
            best = Some(candidate);
        }
        Ok(())
    })?;

    let mut result =
        best.ok_or_else(|| OracleError::new("exact subset search produced no candidate"))?;
    result.subsets_evaluated = evaluated;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn patch() -> ContactPatch {
        ContactPatch {
            points: vec![
                ContactPoint {
                    id: "north-east".into(),
                    position: [1.0, 1.0, 0.0],
                },
                ContactPoint {
                    id: "south-west".into(),
                    position: [-1.0, -1.0, 0.0],
                },
                ContactPoint {
                    id: "north-west".into(),
                    position: [-1.0, 1.0, 0.0],
                },
                ContactPoint {
                    id: "south-east".into(),
                    position: [1.0, -1.0, 0.0],
                },
            ],
            directions: vec![
                WrenchDirection {
                    id: "positive-mx".into(),
                    components: [0.0, 0.0, 0.0, 1.0, 0.0, 0.0],
                },
                WrenchDirection {
                    id: "negative-mx".into(),
                    components: [0.0, 0.0, 0.0, -1.0, 0.0, 0.0],
                },
            ],
            friction_coefficient: 0.5,
            characteristic_length: 1.0,
            support_floor: 1.0e-9,
        }
    }

    #[test]
    fn retaining_all_points_has_zero_support_deficit() {
        let source = patch();
        let ids = source
            .points
            .iter()
            .map(|point| point.id.clone())
            .collect::<Vec<_>>();
        let report = evaluate_retained_set(&source, &ids).expect("full set");
        assert_eq!(report.maximum_normalized_deficit, 0.0);
        assert_eq!(report.mean_normalized_deficit, 0.0);
    }

    #[test]
    fn exact_two_point_subset_preserves_both_moment_extremes() {
        let report = select_exact_subset(
            &patch(),
            ExactSelectorConfig {
                keep: 2,
                maximum_subsets: 100,
            },
        )
        .expect("exact two-point subset");
        assert_eq!(report.maximum_normalized_deficit, 0.0);
        assert!(
            report
                .retained_point_ids
                .iter()
                .any(|id| id.starts_with("north"))
        );
        assert!(
            report
                .retained_point_ids
                .iter()
                .any(|id| id.starts_with("south"))
        );
        assert_eq!(report.subsets_evaluated, 6);
    }

    #[test]
    fn single_point_subset_cannot_preserve_both_moment_extremes() {
        let report = select_exact_subset(
            &patch(),
            ExactSelectorConfig {
                keep: 1,
                maximum_subsets: 10,
            },
        )
        .expect("exact one-point subset");
        assert!(report.maximum_normalized_deficit > 0.0);
        assert_eq!(report.subsets_evaluated, 4);
    }

    #[test]
    fn point_and_direction_permutations_do_not_change_selection() {
        let first = select_exact_subset(
            &patch(),
            ExactSelectorConfig {
                keep: 2,
                maximum_subsets: 100,
            },
        )
        .expect("first result");
        let mut reordered = patch();
        reordered.points.reverse();
        reordered.directions.reverse();
        let second = select_exact_subset(
            &reordered,
            ExactSelectorConfig {
                keep: 2,
                maximum_subsets: 100,
            },
        )
        .expect("permuted result");
        assert_eq!(first.retained_point_ids, second.retained_point_ids);
        assert_eq!(
            first.maximum_normalized_deficit,
            second.maximum_normalized_deficit
        );
    }

    #[test]
    fn geometric_scaling_with_characteristic_length_is_invariant() {
        let first = select_exact_subset(
            &patch(),
            ExactSelectorConfig {
                keep: 2,
                maximum_subsets: 100,
            },
        )
        .expect("unit patch");
        let mut scaled = patch();
        for point in &mut scaled.points {
            for coordinate in &mut point.position {
                *coordinate *= 100.0;
            }
        }
        scaled.characteristic_length *= 100.0;
        let second = select_exact_subset(
            &scaled,
            ExactSelectorConfig {
                keep: 2,
                maximum_subsets: 100,
            },
        )
        .expect("scaled patch");
        assert_eq!(first.retained_point_ids, second.retained_point_ids);
        assert!(
            (first.maximum_normalized_deficit - second.maximum_normalized_deficit).abs() < 1.0e-12
        );
    }

    #[test]
    fn friction_direction_has_position_independent_support_for_shared_normal() {
        let mut source = patch();
        source.directions = vec![WrenchDirection {
            id: "positive-fx".into(),
            components: [1.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        }];
        let report = evaluate_retained_set(&source, &["north-east".into()]).expect("one contact");
        assert!((report.directions[0].full_patch_support - 0.5).abs() < 1.0e-12);
        assert!((report.directions[0].retained_patch_support - 0.5).abs() < 1.0e-12);
    }

    #[test]
    fn direction_magnitude_does_not_change_normalized_report() {
        let first = select_exact_subset(
            &patch(),
            ExactSelectorConfig {
                keep: 1,
                maximum_subsets: 10,
            },
        )
        .expect("unit direction");
        let mut doubled = patch();
        for direction in &mut doubled.directions {
            for component in &mut direction.components {
                *component *= 100.0;
            }
        }
        let second = select_exact_subset(
            &doubled,
            ExactSelectorConfig {
                keep: 1,
                maximum_subsets: 10,
            },
        )
        .expect("scaled directions");
        assert_eq!(first.retained_point_ids, second.retained_point_ids);
        assert!(
            (first.maximum_normalized_deficit - second.maximum_normalized_deficit).abs() < 1.0e-12
        );
    }

    #[test]
    fn greedy_matches_exact_on_the_symmetric_known_answer_patch() {
        let exact = select_exact_subset(
            &patch(),
            ExactSelectorConfig {
                keep: 2,
                maximum_subsets: 100,
            },
        )
        .expect("exact selector");
        let greedy = select_greedy_subset(
            &patch(),
            GreedySelectorConfig {
                keep: 2,
                maximum_evaluations: 100,
            },
        )
        .expect("greedy selector");
        assert_eq!(greedy.retained_point_ids, exact.retained_point_ids);
        assert_eq!(
            greedy.maximum_normalized_deficit,
            exact.maximum_normalized_deficit
        );
        assert_eq!(greedy.subsets_evaluated, 7);
    }

    #[test]
    fn exact_selector_is_never_worse_than_greedy_on_same_finite_objective() {
        let exact = select_exact_subset(
            &patch(),
            ExactSelectorConfig {
                keep: 1,
                maximum_subsets: 100,
            },
        )
        .expect("exact selector");
        let greedy = select_greedy_subset(
            &patch(),
            GreedySelectorConfig {
                keep: 1,
                maximum_evaluations: 100,
            },
        )
        .expect("greedy selector");
        assert!(exact.maximum_normalized_deficit <= greedy.maximum_normalized_deficit);
    }

    #[test]
    fn exact_oracle_exposes_a_real_greedy_local_choice_gap() {
        let source = ContactPatch {
            points: vec![
                ContactPoint {
                    id: "p0".into(),
                    position: [-0.8053331255487975, -1.0022880202777662, 0.0],
                },
                ContactPoint {
                    id: "p1".into(),
                    position: [-0.00663309450661842, 0.24817392184595977, 0.0],
                },
                ContactPoint {
                    id: "p2".into(),
                    position: [-0.9469860377145608, -1.4553152497193032, 0.0],
                },
                ContactPoint {
                    id: "p3".into(),
                    position: [-0.0866003133286175, 0.6847299845497847, 0.0],
                },
                ContactPoint {
                    id: "p4".into(),
                    position: [1.25580147532063, 0.37660201720639197, 0.0],
                },
            ],
            directions: vec![
                WrenchDirection {
                    id: "mx-positive".into(),
                    components: [0.0, 0.0, 0.0, 1.0, 0.0, 0.0],
                },
                WrenchDirection {
                    id: "mx-negative".into(),
                    components: [0.0, 0.0, 0.0, -1.0, 0.0, 0.0],
                },
                WrenchDirection {
                    id: "my-positive".into(),
                    components: [0.0, 0.0, 0.0, 0.0, 1.0, 0.0],
                },
                WrenchDirection {
                    id: "my-negative".into(),
                    components: [0.0, 0.0, 0.0, 0.0, -1.0, 0.0],
                },
                WrenchDirection {
                    id: "fx-positive".into(),
                    components: [1.0, 0.0, 0.0, 0.0, 0.0, 0.0],
                },
                WrenchDirection {
                    id: "fx-negative".into(),
                    components: [-1.0, 0.0, 0.0, 0.0, 0.0, 0.0],
                },
                WrenchDirection {
                    id: "fy-positive".into(),
                    components: [0.0, 1.0, 0.0, 0.0, 0.0, 0.0],
                },
                WrenchDirection {
                    id: "fy-negative".into(),
                    components: [0.0, -1.0, 0.0, 0.0, 0.0, 0.0],
                },
                WrenchDirection {
                    id: "fz-positive".into(),
                    components: [0.0, 0.0, 1.0, 0.0, 0.0, 0.0],
                },
            ],
            friction_coefficient: 0.6,
            characteristic_length: 1.0,
            support_floor: 0.1,
        };
        let exact = select_exact_subset(
            &source,
            ExactSelectorConfig {
                keep: 3,
                maximum_subsets: 100,
            },
        )
        .expect("exact subset");
        let greedy = select_greedy_subset(
            &source,
            GreedySelectorConfig {
                keep: 3,
                maximum_evaluations: 100,
            },
        )
        .expect("greedy subset");
        assert_eq!(exact.retained_point_ids, vec!["p2", "p3", "p4"]);
        assert_eq!(exact.maximum_normalized_deficit, 0.0);
        assert_eq!(greedy.retained_point_ids, vec!["p1", "p2", "p4"]);
        assert!(greedy.maximum_normalized_deficit > 0.4);
        assert!(exact.maximum_normalized_deficit < greedy.maximum_normalized_deficit);
    }

    #[test]
    fn greedy_evaluation_limit_fails_closed() {
        let result = select_greedy_subset(
            &patch(),
            GreedySelectorConfig {
                keep: 2,
                maximum_evaluations: 6,
            },
        );
        assert!(result.is_err());
    }

    #[test]
    fn exact_search_refuses_combinatorial_explosion() {
        let result = select_exact_subset(
            &patch(),
            ExactSelectorConfig {
                keep: 2,
                maximum_subsets: 5,
            },
        );
        assert!(result.is_err());
    }

    #[test]
    fn invalid_or_duplicate_point_ids_are_rejected() {
        let mut source = patch();
        source.points[1].id = source.points[0].id.clone();
        assert!(
            select_exact_subset(
                &source,
                ExactSelectorConfig {
                    keep: 2,
                    maximum_subsets: 100
                }
            )
            .is_err()
        );
    }

    #[test]
    fn unknown_retained_point_is_rejected() {
        assert!(evaluate_retained_set(&patch(), &["not-a-point".into()]).is_err());
    }

    #[test]
    fn invalid_patch_parameters_are_rejected() {
        let mut source = patch();
        source.friction_coefficient = f64::NAN;
        assert!(
            select_exact_subset(
                &source,
                ExactSelectorConfig {
                    keep: 2,
                    maximum_subsets: 100
                }
            )
            .is_err()
        );
    }

    #[test]
    fn non_coplanar_points_are_rejected_by_shared_normal_model() {
        let mut source = patch();
        source.points[0].position[2] = 0.25;
        assert!(
            select_exact_subset(
                &source,
                ExactSelectorConfig {
                    keep: 2,
                    maximum_subsets: 100,
                }
            )
            .is_err()
        );
    }

    #[test]
    fn invalid_direction_is_rejected() {
        let mut source = patch();
        source.directions[0].components = [0.0; 6];
        assert!(evaluate_retained_set(&source, &["north-east".into()]).is_err());
    }
}
