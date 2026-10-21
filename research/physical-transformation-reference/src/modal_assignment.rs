// SPDX-License-Identifier: AGPL-3.0-or-later
//! Fail-closed one-to-one matching of isolated modes across two model runs.
//!
//! Frequency rank is not mode identity when modes cross. This module maximizes
//! total scalar MAC subject to explicit relative-frequency and minimum-MAC
//! gates using deterministic Hungarian assignment. Repeated / tightly clustered
//! modes must be compared as subspaces instead of assigning scalar vectors.

use crate::Check;

const FORBIDDEN_COST: f64 = 1.0e6;

/// An isolated reference/candidate mode pair passing both predeclared gates.
#[derive(Clone, Debug, PartialEq)]
pub struct IsolatedModeMatch {
    pub reference_mode_index: usize,
    pub candidate_mode_index: usize,
    pub reference_frequency_hz: f64,
    pub candidate_frequency_hz: f64,
    pub relative_frequency_gap: f64,
    pub modal_assurance_criterion: f64,
}

/// An accepted one-to-one assignment between two sets of isolated modes.
#[derive(Clone, Debug, PartialEq)]
pub struct IsolatedModeAssignment {
    pub matches: Vec<IsolatedModeMatch>,
    pub minimum_matched_mac: f64,
    pub maximum_relative_frequency_gap: f64,
    pub mean_matched_mac: f64,
}

/// Match isolated modes with explicit admissibility thresholds.
///
/// mac_matrix[reference_index][candidate_index] is the scalar MAC measured on
/// common physical DOFs or a common physical sample grid. Frequencies in each
/// model's list must be sorted, but the assignment may pair indices out of rank
/// order. The method maximizes total MAC among admissible one-to-one matchings.
///
/// Ok(Some(...)) means every mode has one unique admissible partner. Ok(None)
/// means no complete assignment satisfies both gates and the comparison
/// abstains. Malformed inputs return Err. Repeated/clustered mode groups are
/// not appropriate for scalar matching: compare their subspaces separately.
pub fn assign_isolated_modes(
    reference_frequencies_hz: &[f64],
    candidate_frequencies_hz: &[f64],
    mac_matrix: &[Vec<f64>],
    maximum_relative_frequency_gap: f64,
    minimum_mac: f64,
) -> Check<Option<IsolatedModeAssignment>> {
    if reference_frequencies_hz.is_empty()
        || reference_frequencies_hz.len() != candidate_frequencies_hz.len()
    {
        return Err("reference and candidate mode counts must be equal and non-zero".to_owned());
    }
    let size = reference_frequencies_hz.len();
    if mac_matrix.len() != size || mac_matrix.iter().any(|row| row.len() != size) {
        return Err("MAC matrix must be square and match the mode count".to_owned());
    }
    if reference_frequencies_hz
        .iter()
        .chain(candidate_frequencies_hz)
        .any(|frequency| !frequency.is_finite() || *frequency <= 0.0)
    {
        return Err("mode frequencies must be finite and positive".to_owned());
    }
    if reference_frequencies_hz.windows(2).any(|pair| pair[0] > pair[1])
        || candidate_frequencies_hz.windows(2).any(|pair| pair[0] > pair[1])
    {
        return Err("frequency arrays must be sorted in nondecreasing order".to_owned());
    }
    if !maximum_relative_frequency_gap.is_finite()
        || maximum_relative_frequency_gap < 0.0
        || maximum_relative_frequency_gap >= 1.0
    {
        return Err("maximum relative frequency gap must be finite and in [0, 1)".to_owned());
    }
    if !minimum_mac.is_finite() || !(0.0..=1.0).contains(&minimum_mac) {
        return Err("minimum MAC must be finite and in [0, 1]".to_owned());
    }
    if mac_matrix
        .iter()
        .flatten()
        .any(|mac| !mac.is_finite() || !(0.0..=1.0).contains(mac))
    {
        return Err("MAC entries must be finite and in [0, 1]".to_owned());
    }

    let mut costs = vec![vec![FORBIDDEN_COST; size]; size];
    let mut gaps = vec![vec![0.0; size]; size];
    for reference_index in 0..size {
        for candidate_index in 0..size {
            let reference_frequency = reference_frequencies_hz[reference_index];
            let candidate_frequency = candidate_frequencies_hz[candidate_index];
            let relative_gap = (candidate_frequency - reference_frequency).abs()
                / candidate_frequency.abs().max(reference_frequency.abs());
            gaps[reference_index][candidate_index] = relative_gap;
            let mac = mac_matrix[reference_index][candidate_index];
            if relative_gap <= maximum_relative_frequency_gap && mac >= minimum_mac {
                // Constraints are filters, not trade-off weights. Among the
                // remaining feasible edges, minimizing 1-MAC maximizes total MAC.
                costs[reference_index][candidate_index] = 1.0 - mac;
            }
        }
    }

    let candidate_for_reference = hungarian_min_assignment(&costs)?;
    let mut matches = Vec::with_capacity(size);
    for (reference_index, candidate_index) in candidate_for_reference.into_iter().enumerate() {
        let relative_gap = gaps[reference_index][candidate_index];
        let mac = mac_matrix[reference_index][candidate_index];
        if costs[reference_index][candidate_index] >= FORBIDDEN_COST
            || relative_gap > maximum_relative_frequency_gap
            || mac < minimum_mac
        {
            return Ok(None);
        }
        matches.push(IsolatedModeMatch {
            reference_mode_index: reference_index,
            candidate_mode_index: candidate_index,
            reference_frequency_hz: reference_frequencies_hz[reference_index],
            candidate_frequency_hz: candidate_frequencies_hz[candidate_index],
            relative_frequency_gap: relative_gap,
            modal_assurance_criterion: mac,
        });
    }

    let minimum_matched_mac = matches
        .iter()
        .map(|matched| matched.modal_assurance_criterion)
        .fold(f64::INFINITY, f64::min);
    let maximum_relative_frequency_gap = matches
        .iter()
        .map(|matched| matched.relative_frequency_gap)
        .fold(0.0_f64, f64::max);
    let mean_matched_mac = matches
        .iter()
        .map(|matched| matched.modal_assurance_criterion)
        .sum::<f64>()
        / matches.len() as f64;
    if !minimum_matched_mac.is_finite()
        || !maximum_relative_frequency_gap.is_finite()
        || !mean_matched_mac.is_finite()
    {
        return Err("accepted mode assignment diagnostics are non-finite".to_owned());
    }

    Ok(Some(IsolatedModeAssignment {
        matches,
        minimum_matched_mac,
        maximum_relative_frequency_gap,
        mean_matched_mac,
    }))
}

/// Deterministic O(n^3) Hungarian minimum-cost assignment for a square matrix.
fn hungarian_min_assignment(cost: &[Vec<f64>]) -> Check<Vec<usize>> {
    let size = cost.len();
    if size == 0 || cost.iter().any(|row| row.len() != size) {
        return Err("assignment cost matrix must be non-empty and square".to_owned());
    }
    if cost.iter().flatten().any(|value| !value.is_finite() || *value < 0.0) {
        return Err("assignment cost matrix contains an invalid cost".to_owned());
    }

    let mut row_potential = vec![0.0; size + 1];
    let mut column_potential = vec![0.0; size + 1];
    let mut row_for_column = vec![0_usize; size + 1];
    let mut previous_column = vec![0_usize; size + 1];

    for row in 1..=size {
        row_for_column[0] = row;
        let mut current_column = 0_usize;
        let mut minimum_reduced_cost = vec![f64::INFINITY; size + 1];
        let mut used = vec![false; size + 1];
        loop {
            used[current_column] = true;
            let current_row = row_for_column[current_column];
            let mut delta = f64::INFINITY;
            let mut next_column = 0_usize;
            for column in 1..=size {
                if used[column] {
                    continue;
                }
                let reduced_cost = cost[current_row - 1][column - 1]
                    - row_potential[current_row]
                    - column_potential[column];
                if reduced_cost < minimum_reduced_cost[column] {
                    minimum_reduced_cost[column] = reduced_cost;
                    previous_column[column] = current_column;
                }
                if minimum_reduced_cost[column] < delta {
                    delta = minimum_reduced_cost[column];
                    next_column = column;
                }
            }
            if !delta.is_finite() || next_column == 0 {
                return Err("Hungarian assignment could not find a finite augmenting path".to_owned());
            }
            for column in 0..=size {
                if used[column] {
                    row_potential[row_for_column[column]] += delta;
                    column_potential[column] -= delta;
                } else {
                    minimum_reduced_cost[column] -= delta;
                }
            }
            current_column = next_column;
            if row_for_column[current_column] == 0 {
                break;
            }
        }
        loop {
            let next_column = previous_column[current_column];
            row_for_column[current_column] = row_for_column[next_column];
            current_column = next_column;
            if current_column == 0 {
                break;
            }
        }
    }

    let mut candidate_for_reference = vec![usize::MAX; size];
    for column in 1..=size {
        let row = row_for_column[column];
        if row == 0 {
            return Err("Hungarian assignment left a row unmatched".to_owned());
        }
        candidate_for_reference[row - 1] = column - 1;
    }
    if candidate_for_reference.iter().any(|index| *index == usize::MAX) {
        return Err("Hungarian assignment produced an incomplete result".to_owned());
    }
    Ok(candidate_for_reference)
}

#[cfg(test)]
mod tests {
    use super::assign_isolated_modes;

    #[test]
    fn mode_crossing_is_tracked_by_mac_within_explicit_frequency_gate() {
        let assignment = assign_isolated_modes(
            &[10.0, 20.0],
            &[10.1, 19.9],
            &[vec![0.05, 0.98], vec![0.99, 0.04]],
            0.6,
            0.9,
        )
        .expect("valid mode-crossing input")
        .expect("complete admissible assignment");
        assert_eq!(assignment.matches.len(), 2);
        assert_eq!(assignment.matches[0].candidate_mode_index, 1);
        assert_eq!(assignment.matches[1].candidate_mode_index, 0);
        assert!((assignment.minimum_matched_mac - 0.98).abs() < 1.0e-14);
        assert!((assignment.mean_matched_mac - 0.985).abs() < 1.0e-14);
        assert!(assignment.matches.iter().all(|matched| {
            matched.relative_frequency_gap <= 0.6 && matched.modal_assurance_criterion >= 0.9
        }));
    }

    #[test]
    fn assignment_abstains_when_no_complete_matching_meets_both_gates() {
        assert!(
            assign_isolated_modes(
                &[10.0, 20.0],
                &[10.1, 20.1],
                &[vec![0.7, 0.2], vec![0.4, 0.7]],
                0.05,
                0.8,
            )
            .expect("valid unmatched input")
            .is_none()
        );
        assert!(
            assign_isolated_modes(
                &[10.0, 100.0],
                &[50.0, 200.0],
                &[vec![0.99, 0.99], vec![0.99, 0.99]],
                0.1,
                0.9,
            )
            .expect("valid frequency-incompatible input")
            .is_none()
        );
    }

    #[test]
    fn assignment_maximizes_total_mac_globally_not_greedily() {
        let assignment = assign_isolated_modes(
            &[10.0, 10.1, 10.2],
            &[10.0, 10.1, 10.2],
            &[
                vec![0.90, 0.89, 0.01],
                vec![0.88, 0.10, 0.87],
                vec![0.02, 0.86, 0.85],
            ],
            0.03,
            0.01,
        )
        .expect("valid assignment")
        .expect("complete assignment");
        let got = assignment
            .matches
            .iter()
            .map(|matched| matched.candidate_mode_index)
            .collect::<Vec<_>>();
        assert_eq!(got, vec![0, 2, 1]);
        assert!((assignment.mean_matched_mac - (0.90 + 0.87 + 0.86) / 3.0).abs() < 1.0e-14);
    }

    #[test]
    fn malformed_inputs_fail_closed() {
        assert!(assign_isolated_modes(&[], &[], &[], 0.1, 0.5).is_err());
        assert!(assign_isolated_modes(&[10.0], &[10.0, 20.0], &[vec![1.0]], 0.1, 0.5).is_err());
        assert!(assign_isolated_modes(&[20.0, 10.0], &[10.0, 20.0], &[vec![1.0, 0.0], vec![0.0, 1.0]], 0.1, 0.5).is_err());
        assert!(assign_isolated_modes(&[10.0], &[10.0], &[vec![f64::NAN]], 0.1, 0.5).is_err());
        assert!(assign_isolated_modes(&[10.0], &[10.0], &[vec![1.2]], 0.1, 0.5).is_err());
        assert!(assign_isolated_modes(&[10.0], &[10.0], &[vec![1.0]], 1.0, 0.5).is_err());
        assert!(assign_isolated_modes(&[10.0], &[10.0], &[vec![1.0]], 0.1, f64::NAN).is_err());
    }
}
