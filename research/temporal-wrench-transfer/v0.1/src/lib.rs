// SPDX-License-Identifier: AGPL-3.0-or-later
//! Exact temporal transfer of planar shared-normal contact impulse.
//!
//! Model: all contacts share +Z normal; cached normal impulses are non-negative;
//! the total normal impulse is conserved; new impulses may be reallocated freely.
//! The only controllable moment components are Mx and My, represented by the
//! planar center of pressure. Tangential friction, torsional moment and changing
//! normals are intentionally outside this v0.1 model.

use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::error::Error;
use std::fmt::{self, Display, Formatter};

pub const SCHEMA_VERSION: &str = "temporal-wrench-transfer-v0.1";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransferError {
    pub reason: String,
}

impl TransferError {
    fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
    }
}

impl Display for TransferError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.reason)
    }
}

impl Error for TransferError {}

#[derive(Clone, Debug, PartialEq)]
pub struct PreviousNormalImpulse {
    pub contact_id: String,
    pub position: [f64; 2],
    pub normal_impulse: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NewContactCandidate {
    pub contact_id: String,
    pub position: [f64; 2],
}

#[derive(Clone, Debug, PartialEq)]
pub struct ImpulseAllocation {
    pub contact_id: String,
    pub normal_impulse: f64,
    pub normalized_load_fraction: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TemporalTransferReport {
    pub source_contact_ids: Vec<String>,
    pub retained_contact_ids: Vec<String>,
    /// Previous frame's total normal impulse, used as the target force.
    pub total_normal_impulse: f64,
    /// Total impulse actually assigned to the new manifold.
    pub transferred_normal_impulse: f64,
    /// Transferred minus source total normal impulse.
    pub force_error: f64,
    pub target_center_of_pressure: [f64; 2],
    pub achieved_center_of_pressure: [f64; 2],
    /// Achieved minus target moment components [Mx, My].
    pub moment_error: [f64; 2],
    pub moment_error_norm: f64,
    pub center_of_pressure_error_norm: f64,
    pub impulse_sum_error: f64,
    pub allocations: Vec<ImpulseAllocation>,
    pub exact_wrench_preservation: bool,
}

#[derive(Clone, Debug)]
struct CandidateAllocation {
    weights: Vec<f64>,
    achieved: [f64; 2],
    residual_squared: f64,
    active_ids: Vec<String>,
}

fn norm2(value: [f64; 2]) -> f64 {
    value[0].hypot(value[1])
}

fn cross2(left: [f64; 2], right: [f64; 2]) -> f64 {
    left[0] * right[1] - left[1] * right[0]
}

fn distance_squared(left: [f64; 2], right: [f64; 2]) -> f64 {
    (left[0] - right[0]).powi(2) + (left[1] - right[1]).powi(2)
}

fn validate_inputs(
    previous: &[PreviousNormalImpulse],
    new_contacts: &[NewContactCandidate],
) -> Result<
    (
        Vec<PreviousNormalImpulse>,
        Vec<NewContactCandidate>,
        f64,
        [f64; 2],
    ),
    TransferError,
> {
    if previous.is_empty() {
        return Err(TransferError::new(
            "previous contact impulse set must not be empty",
        ));
    }
    if new_contacts.is_empty() {
        return Err(TransferError::new(
            "new contact candidate set must not be empty",
        ));
    }

    let mut old = previous.to_vec();
    old.sort_by(|left, right| left.contact_id.cmp(&right.contact_id));
    let mut old_ids = BTreeSet::new();
    let mut total_impulse = 0.0_f64;
    let mut weighted_position = [0.0_f64; 2];
    for contact in &old {
        if contact.contact_id.trim().is_empty() || !old_ids.insert(contact.contact_id.as_str()) {
            return Err(TransferError::new(
                "previous contact IDs must be non-empty and unique",
            ));
        }
        if !contact.position.iter().all(|value| value.is_finite())
            || !contact.normal_impulse.is_finite()
            || contact.normal_impulse < 0.0
        {
            return Err(TransferError::new(
                "previous contact position and non-negative impulse must be finite",
            ));
        }
        total_impulse += contact.normal_impulse;
        weighted_position[0] += contact.position[0] * contact.normal_impulse;
        weighted_position[1] += contact.position[1] * contact.normal_impulse;
    }
    if !total_impulse.is_finite() || total_impulse <= 0.0 {
        return Err(TransferError::new(
            "total previous normal impulse must be finite and positive",
        ));
    }

    let target_center = [
        weighted_position[0] / total_impulse,
        weighted_position[1] / total_impulse,
    ];
    if !target_center.iter().all(|value| value.is_finite()) {
        return Err(TransferError::new(
            "target center of pressure is non-finite",
        ));
    }

    let mut new = new_contacts.to_vec();
    new.sort_by(|left, right| left.contact_id.cmp(&right.contact_id));
    let mut new_ids = BTreeSet::new();
    for contact in &new {
        if contact.contact_id.trim().is_empty() || !new_ids.insert(contact.contact_id.as_str()) {
            return Err(TransferError::new(
                "new contact IDs must be non-empty and unique",
            ));
        }
        if !contact.position.iter().all(|value| value.is_finite()) {
            return Err(TransferError::new("new contact positions must be finite"));
        }
    }

    Ok((old, new, total_impulse, target_center))
}

fn make_candidate(
    new_contacts: &[NewContactCandidate],
    target: [f64; 2],
    entries: &[(usize, f64)],
) -> Option<CandidateAllocation> {
    let mut weights = vec![0.0; new_contacts.len()];
    let mut weight_sum = 0.0_f64;
    for (index, weight) in entries {
        if *index >= new_contacts.len() || !weight.is_finite() || *weight < -1.0e-12 {
            return None;
        }
        weights[*index] += weight.max(0.0);
        weight_sum += weight.max(0.0);
    }
    if !weight_sum.is_finite() || weight_sum <= 0.0 {
        return None;
    }
    for weight in &mut weights {
        *weight /= weight_sum;
    }

    let mut achieved = [0.0_f64; 2];
    let mut active_ids = Vec::new();
    for (index, weight) in weights.iter().enumerate() {
        achieved[0] += new_contacts[index].position[0] * weight;
        achieved[1] += new_contacts[index].position[1] * weight;
        if *weight > 1.0e-12 {
            active_ids.push(new_contacts[index].contact_id.clone());
        }
    }
    let residual_squared = distance_squared(achieved, target);
    if !residual_squared.is_finite() {
        return None;
    }
    Some(CandidateAllocation {
        weights,
        achieved,
        residual_squared,
        active_ids,
    })
}

fn compare_candidates(left: &CandidateAllocation, right: &CandidateAllocation) -> Ordering {
    left.residual_squared
        .total_cmp(&right.residual_squared)
        .then_with(|| left.active_ids.len().cmp(&right.active_ids.len()))
        .then_with(|| left.active_ids.cmp(&right.active_ids))
        .then_with(|| {
            for (left_weight, right_weight) in left.weights.iter().zip(&right.weights) {
                let order = left_weight.total_cmp(right_weight);
                if order != Ordering::Equal {
                    return order;
                }
            }
            Ordering::Equal
        })
}

fn consider(best: &mut Option<CandidateAllocation>, candidate: Option<CandidateAllocation>) {
    if let Some(candidate) = candidate
        && best
            .as_ref()
            .is_none_or(|incumbent| compare_candidates(&candidate, incumbent).is_lt())
    {
        *best = Some(candidate);
    }
}

fn exact_convex_hull_projection(
    new_contacts: &[NewContactCandidate],
    target: [f64; 2],
) -> Result<CandidateAllocation, TransferError> {
    let mut best = None;

    // Vertices.
    for (index, _contact) in new_contacts.iter().enumerate() {
        consider(
            &mut best,
            make_candidate(new_contacts, target, &[(index, 1.0)]),
        );
    }

    // Every segment. A closest boundary point of a 2D convex hull lies on
    // a hull edge, and every hull edge is represented by one candidate pair.
    for (left, left_contact) in new_contacts.iter().enumerate() {
        for (right, right_contact) in new_contacts.iter().enumerate().skip(left + 1) {
            let a = left_contact.position;
            let b = right_contact.position;
            let edge = [b[0] - a[0], b[1] - a[1]];
            let denominator = edge[0] * edge[0] + edge[1] * edge[1];
            if denominator <= 1.0e-30 {
                continue;
            }
            let to_target = [target[0] - a[0], target[1] - a[1]];
            let t =
                ((to_target[0] * edge[0] + to_target[1] * edge[1]) / denominator).clamp(0.0, 1.0);
            consider(
                &mut best,
                make_candidate(new_contacts, target, &[(left, 1.0 - t), (right, t)]),
            );
        }
    }

    // By Carathéodory's theorem, any point in a 2D convex hull has a
    // representation using at most three vertices. Enumerate all triangles
    // to detect exact interior solutions.
    for (first, first_contact) in new_contacts.iter().enumerate() {
        for (second, second_contact) in new_contacts.iter().enumerate().skip(first + 1) {
            for (third, third_contact) in new_contacts.iter().enumerate().skip(second + 1) {
                let a = first_contact.position;
                let b = second_contact.position;
                let c = third_contact.position;
                let ab = [b[0] - a[0], b[1] - a[1]];
                let ac = [c[0] - a[0], c[1] - a[1]];
                let at = [target[0] - a[0], target[1] - a[1]];
                let determinant = cross2(ab, ac);
                if determinant.abs() <= 1.0e-15 {
                    continue;
                }
                let weight_b = cross2(at, ac) / determinant;
                let weight_c = cross2(ab, at) / determinant;
                let weight_a = 1.0 - weight_b - weight_c;
                if weight_a >= -1.0e-12 && weight_b >= -1.0e-12 && weight_c >= -1.0e-12 {
                    consider(
                        &mut best,
                        make_candidate(
                            new_contacts,
                            target,
                            &[(first, weight_a), (second, weight_b), (third, weight_c)],
                        ),
                    );
                }
            }
        }
    }

    best.ok_or_else(|| TransferError::new("convex-hull projection produced no feasible allocation"))
}

fn build_transfer_report(
    old: &[PreviousNormalImpulse],
    new: &[NewContactCandidate],
    total_impulse: f64,
    target_center: [f64; 2],
    weights: &[f64],
    transfer_scale: f64,
) -> Result<TemporalTransferReport, TransferError> {
    if weights.len() != new.len()
        || !transfer_scale.is_finite()
        || transfer_scale <= 0.0
        || transfer_scale > 1.0
    {
        return Err(TransferError::new("transfer weights and scale are invalid"));
    }
    let transferred_normal_impulse = total_impulse * transfer_scale;
    let mut allocations = Vec::new();
    let mut impulse_sum = 0.0_f64;
    let mut achieved_center = [0.0_f64; 2];
    for (index, fraction) in weights.iter().enumerate() {
        if !fraction.is_finite() || *fraction < 0.0 {
            return Err(TransferError::new(
                "normalized impulse weights must be finite and non-negative",
            ));
        }
        achieved_center[0] += new[index].position[0] * fraction;
        achieved_center[1] += new[index].position[1] * fraction;
        if *fraction > 1.0e-12 {
            let normal_impulse = transferred_normal_impulse * fraction;
            impulse_sum += normal_impulse;
            allocations.push(ImpulseAllocation {
                contact_id: new[index].contact_id.clone(),
                normal_impulse,
                normalized_load_fraction: *fraction,
            });
        }
    }
    let weight_sum: f64 = weights.iter().sum();
    if (weight_sum - 1.0).abs() > 1.0e-12 {
        return Err(TransferError::new(
            "normalized impulse weights must sum to one",
        ));
    }

    let force_error = transferred_normal_impulse - total_impulse;
    let moment_error = [
        transferred_normal_impulse * achieved_center[1] - total_impulse * target_center[1],
        -transferred_normal_impulse * achieved_center[0] + total_impulse * target_center[0],
    ];
    let moment_error_norm = norm2(moment_error);
    let center_of_pressure_error_norm = norm2([
        achieved_center[0] - target_center[0],
        achieved_center[1] - target_center[1],
    ]);
    let impulse_sum_error = impulse_sum - transferred_normal_impulse;
    if !moment_error_norm.is_finite()
        || !center_of_pressure_error_norm.is_finite()
        || !impulse_sum_error.is_finite()
        || !force_error.is_finite()
    {
        return Err(TransferError::new(
            "resultant wrench transfer residual is non-finite",
        ));
    }
    let coordinate_scale = norm2(target_center).max(norm2(achieved_center)).max(1.0);
    let center_tolerance = 1.0e-12 * coordinate_scale;
    let impulse_tolerance = 1.0e-12 * total_impulse.max(1.0);
    let force_tolerance = impulse_tolerance;

    Ok(TemporalTransferReport {
        source_contact_ids: old
            .iter()
            .map(|contact| contact.contact_id.clone())
            .collect(),
        retained_contact_ids: allocations
            .iter()
            .map(|item| item.contact_id.clone())
            .collect(),
        total_normal_impulse: total_impulse,
        transferred_normal_impulse,
        force_error,
        target_center_of_pressure: target_center,
        achieved_center_of_pressure: achieved_center,
        moment_error,
        moment_error_norm,
        center_of_pressure_error_norm,
        impulse_sum_error,
        allocations,
        exact_wrench_preservation: center_of_pressure_error_norm <= center_tolerance
            && force_error.abs() <= force_tolerance
            && moment_error_norm <= force_tolerance * coordinate_scale
            && impulse_sum_error.abs() <= impulse_tolerance,
    })
}

/// Redistribute the previous frame's total normal impulse over a new planar
/// contact set while minimizing center-of-pressure (and therefore normal-force
/// moment) error. All new impulses are non-negative and their sum is exact.
pub fn transfer_normal_impulse(
    previous: &[PreviousNormalImpulse],
    new_contacts: &[NewContactCandidate],
) -> Result<TemporalTransferReport, TransferError> {
    let (old, new, total_impulse, target_center) = validate_inputs(previous, new_contacts)?;
    let selected = exact_convex_hull_projection(&new, target_center)?;
    build_transfer_report(
        &old,
        &new,
        total_impulse,
        target_center,
        &selected.weights,
        1.0,
    )
}

/// Reproduce the current scalar-cache baseline: scale the prior total normal
/// impulse, then distribute it equally across all new manifold points. The
/// production warm-start currently uses a scale of 0.8.
pub fn transfer_equal_distribution(
    previous: &[PreviousNormalImpulse],
    new_contacts: &[NewContactCandidate],
    warm_start_scale: f64,
) -> Result<TemporalTransferReport, TransferError> {
    let (old, new, total_impulse, target_center) = validate_inputs(previous, new_contacts)?;
    if !warm_start_scale.is_finite() || warm_start_scale <= 0.0 || warm_start_scale > 1.0 {
        return Err(TransferError::new("warm-start scale must be in (0, 1]"));
    }
    let equal_fraction = 1.0 / new.len() as f64;
    let weights = vec![equal_fraction; new.len()];
    build_transfer_report(
        &old,
        &new,
        total_impulse,
        target_center,
        &weights,
        warm_start_scale,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn old(id: &str, x: f64, y: f64, impulse: f64) -> PreviousNormalImpulse {
        PreviousNormalImpulse {
            contact_id: id.into(),
            position: [x, y],
            normal_impulse: impulse,
        }
    }

    fn new(id: &str, x: f64, y: f64) -> NewContactCandidate {
        NewContactCandidate {
            contact_id: id.into(),
            position: [x, y],
        }
    }

    #[test]
    fn center_of_pressure_inside_new_hull_is_preserved_exactly() {
        let previous = vec![
            old("old-a", -1.0, -1.0, 1.0),
            old("old-b", 1.0, -1.0, 1.0),
            old("old-c", -1.0, 1.0, 1.0),
            old("old-d", 1.0, 1.0, 1.0),
        ];
        let next = vec![new("new-a", -1.0, 1.0), new("new-b", 1.0, -1.0)];
        let report = transfer_normal_impulse(&previous, &next).expect("exact transfer");
        assert!(report.exact_wrench_preservation);
        assert_eq!(report.total_normal_impulse, 4.0);
        assert_eq!(report.allocations.len(), 2);
        assert!((report.allocations[0].normal_impulse - 2.0).abs() < 1.0e-12);
        assert!((report.allocations[1].normal_impulse - 2.0).abs() < 1.0e-12);
        assert!(report.moment_error_norm < 1.0e-12);
    }

    #[test]
    fn center_of_pressure_outside_new_hull_is_projected_without_negative_impulse() {
        let previous = vec![old("old", 2.0, 0.0, 2.0)];
        let next = vec![new("left", -1.0, 0.0), new("right", 1.0, 0.0)];
        let report = transfer_normal_impulse(&previous, &next).expect("bounded transfer");
        assert!(!report.exact_wrench_preservation);
        assert!((report.achieved_center_of_pressure[0] - 1.0).abs() < 1.0e-12);
        assert!((report.center_of_pressure_error_norm - 1.0).abs() < 1.0e-12);
        assert!((report.moment_error_norm - 2.0).abs() < 1.0e-12);
        assert!(
            report
                .allocations
                .iter()
                .all(|item| item.normal_impulse >= 0.0)
        );
        assert!(
            (report
                .allocations
                .iter()
                .map(|item| item.normal_impulse)
                .sum::<f64>()
                - 2.0)
                .abs()
                < 1.0e-12
        );
    }

    #[test]
    fn triangle_interior_representation_preserves_target_moment() {
        let previous = vec![old("old", 0.25, 0.25, 3.0)];
        let next = vec![new("a", 0.0, 0.0), new("b", 1.0, 0.0), new("c", 0.0, 1.0)];
        let report = transfer_normal_impulse(&previous, &next).expect("triangle transfer");
        assert!(report.exact_wrench_preservation);
        assert_eq!(report.allocations.len(), 3);
        let weights = report
            .allocations
            .iter()
            .map(|item| item.normalized_load_fraction)
            .collect::<Vec<_>>();
        assert!((weights.iter().sum::<f64>() - 1.0).abs() < 1.0e-12);
    }

    #[test]
    fn one_new_point_clamps_all_warm_start_load_to_that_point() {
        let previous = vec![old("old-a", -1.0, 0.0, 1.0), old("old-b", 1.0, 0.0, 1.0)];
        let report = transfer_normal_impulse(&previous, &[new("only", 0.5, 0.0)])
            .expect("single contact transfer");
        assert_eq!(report.allocations.len(), 1);
        assert_eq!(report.allocations[0].normal_impulse, 2.0);
        assert!(!report.exact_wrench_preservation);
    }

    #[test]
    fn contact_and_source_permutations_do_not_change_transfer() {
        let previous = vec![old("old-b", 1.0, -1.0, 1.0), old("old-a", -1.0, 1.0, 1.0)];
        let next = vec![new("new-b", 1.0, 1.0), new("new-a", -1.0, -1.0)];
        let first = transfer_normal_impulse(&previous, &next).expect("first");
        let mut previous_reversed = previous.clone();
        previous_reversed.reverse();
        let mut next_reversed = next.clone();
        next_reversed.reverse();
        let second = transfer_normal_impulse(&previous_reversed, &next_reversed).expect("reversed");
        assert_eq!(first.allocations, second.allocations);
        assert_eq!(
            first.achieved_center_of_pressure,
            second.achieved_center_of_pressure
        );
        assert_eq!(first.moment_error, second.moment_error);
    }

    #[test]
    fn current_equal_warm_start_baseline_reports_force_and_moment_loss() {
        let previous = vec![old("old-a", -1.0, 0.0, 1.0), old("old-b", 1.0, 0.0, 1.0)];
        let next = vec![new("left", -1.0, 0.0), new("right", 3.0, 0.0)];
        let baseline = transfer_equal_distribution(&previous, &next, 0.8)
            .expect("current equal-distribution baseline");
        let transported =
            transfer_normal_impulse(&previous, &next).expect("wrench-preserving transfer");

        assert!((baseline.transferred_normal_impulse - 1.6).abs() < 1.0e-12);
        assert!((baseline.force_error + 0.4).abs() < 1.0e-12);
        assert!((baseline.achieved_center_of_pressure[0] - 1.0).abs() < 1.0e-12);
        assert!((baseline.moment_error_norm - 1.6).abs() < 1.0e-12);
        assert!(!baseline.exact_wrench_preservation);

        assert!((transported.transferred_normal_impulse - 2.0).abs() < 1.0e-12);
        assert!(transported.exact_wrench_preservation);
        assert!((transported.achieved_center_of_pressure[0]).abs() < 1.0e-12);
    }

    #[test]
    fn equal_distribution_scale_must_be_valid() {
        let previous = vec![old("old", 0.0, 0.0, 1.0)];
        let next = vec![new("new", 0.0, 0.0)];
        assert!(transfer_equal_distribution(&previous, &next, 0.0).is_err());
        assert!(transfer_equal_distribution(&previous, &next, 1.1).is_err());
        assert!(transfer_equal_distribution(&previous, &next, f64::NAN).is_err());
    }

    #[test]
    fn total_normal_impulse_is_conserved_even_when_moment_is_not() {
        let previous = vec![old("old", 3.0, 2.0, 7.25)];
        let next = vec![new("a", -1.0, 0.0), new("b", 1.0, 0.0)];
        let report = transfer_normal_impulse(&previous, &next).expect("transfer");
        let transferred: f64 = report
            .allocations
            .iter()
            .map(|item| item.normal_impulse)
            .sum();
        assert!((transferred - 7.25).abs() < 1.0e-12);
        assert!(report.impulse_sum_error.abs() < 1.0e-12);
        assert!(!report.exact_wrench_preservation);
    }

    #[test]
    fn translating_both_manifolds_preserves_transfer_and_moment_error() {
        let previous = vec![old("old-a", -1.0, 0.0, 1.0), old("old-b", 1.0, 0.0, 1.0)];
        let next = vec![new("new-a", -1.0, 0.0), new("new-b", 3.0, 0.0)];
        let baseline = transfer_normal_impulse(&previous, &next).expect("baseline");
        let shifted_previous = previous
            .iter()
            .map(|item| {
                old(
                    &item.contact_id,
                    item.position[0] + 7.0,
                    item.position[1] - 4.0,
                    item.normal_impulse,
                )
            })
            .collect::<Vec<_>>();
        let shifted_next = next
            .iter()
            .map(|item| {
                new(
                    &item.contact_id,
                    item.position[0] + 7.0,
                    item.position[1] - 4.0,
                )
            })
            .collect::<Vec<_>>();
        let shifted =
            transfer_normal_impulse(&shifted_previous, &shifted_next).expect("shifted");

        assert_eq!(baseline.retained_contact_ids, shifted.retained_contact_ids);
        assert_eq!(baseline.allocations, shifted.allocations);
        assert!(
            (baseline.center_of_pressure_error_norm - shifted.center_of_pressure_error_norm).abs()
                < 1.0e-12
        );
        assert!((baseline.moment_error_norm - shifted.moment_error_norm).abs() < 1.0e-12);
    }

    #[test]
    fn positive_impulse_scaling_preserves_normalized_allocation() {
        let previous = vec![old("old-a", -1.0, 0.0, 1.0), old("old-b", 1.0, 0.0, 3.0)];
        let next = vec![new("new-a", -2.0, 0.0), new("new-b", 2.0, 0.0)];
        let baseline = transfer_normal_impulse(&previous, &next).expect("baseline");
        let scaled_previous = previous
            .iter()
            .map(|item| {
                old(
                    &item.contact_id,
                    item.position[0],
                    item.position[1],
                    item.normal_impulse * 5.0,
                )
            })
            .collect::<Vec<_>>();
        let scaled = transfer_normal_impulse(&scaled_previous, &next).expect("scaled");
        assert_eq!(baseline.retained_contact_ids, scaled.retained_contact_ids);
        assert_eq!(baseline.allocations.len(), scaled.allocations.len());
        for (first, second) in baseline.allocations.iter().zip(&scaled.allocations) {
            assert!(
                (first.normalized_load_fraction - second.normalized_load_fraction).abs() < 1.0e-12
            );
            assert!((second.normal_impulse - first.normal_impulse * 5.0).abs() < 1.0e-12);
        }
    }

    #[test]
    fn non_finite_and_negative_previous_impulses_are_rejected() {
        assert!(
            transfer_normal_impulse(&[old("bad", 0.0, 0.0, f64::NAN)], &[new("new", 0.0, 0.0)])
                .is_err()
        );
        assert!(
            transfer_normal_impulse(&[old("bad", 0.0, 0.0, -1.0)], &[new("new", 0.0, 0.0)])
                .is_err()
        );
    }

    #[test]
    fn duplicate_or_empty_contact_ids_are_rejected() {
        let previous = vec![old("old", 0.0, 0.0, 1.0)];
        assert!(
            transfer_normal_impulse(&previous, &[new("x", 0.0, 0.0), new("x", 1.0, 0.0)]).is_err()
        );
        assert!(transfer_normal_impulse(&previous, &[new("", 0.0, 0.0)]).is_err());
    }

    #[test]
    fn zero_total_impulse_is_rejected_as_undefined_center_of_pressure() {
        assert!(
            transfer_normal_impulse(&[old("zero", 0.0, 0.0, 0.0)], &[new("new", 0.0, 0.0)])
                .is_err()
        );
    }
}
