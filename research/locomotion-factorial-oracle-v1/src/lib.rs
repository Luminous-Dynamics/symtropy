// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Synthetic known-answer oracle for three-factor locomotion intervention designs.
//!
//! This module tests algebraic estimability of a declared 2x2x2 factorial model.
//! It is not a biological simulator or a statistical power calculation.

#![forbid(unsafe_code)]
#![allow(clippy::needless_range_loop)]

const FACTORIAL_TERMS: usize = 8;
const RANK_TOLERANCE: f64 = 1.0e-10;

/// One combination in the central-drive × local-reflex × load-feedback design.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FactorialCell {
    /// Whether central rhythm / descending drive is in its reference (+1) state.
    pub central_drive: bool,
    /// Whether the local proprioceptive reflex is in its reference (+1) state.
    pub local_reflex: bool,
    /// Whether peripheral load / proprioceptive feedback is in its reference (+1) state.
    pub load_feedback: bool,
}

impl FactorialCell {
    /// Creates a factorial cell from its three binary factor states.
    pub const fn new(central_drive: bool, local_reflex: bool, load_feedback: bool) -> Self {
        Self {
            central_drive,
            local_reflex,
            load_feedback,
        }
    }

    /// Returns the intercept, main-effect, pairwise and three-way design row.
    ///
    /// False is coded -1 and true is coded +1.
    #[must_use]
    pub fn design_row(self) -> [f64; FACTORIAL_TERMS] {
        let a = if self.central_drive { 1.0 } else { -1.0 };
        let b = if self.local_reflex { 1.0 } else { -1.0 };
        let c = if self.load_feedback { 1.0 } else { -1.0 };
        [1.0, a, b, c, a * b, a * c, b * c, a * b * c]
    }
}

/// One cell-level outcome supplied to the design oracle.
///
/// A production analysis must preserve individual/session-level observations and
/// estimate this cell-level outcome with an appropriate clustered model first.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FactorialObservation {
    /// The factor combination represented by this observation.
    pub cell: FactorialCell,
    /// Preregistered outcome on the declared additive scale.
    pub outcome: f64,
}

/// Coefficients of the complete three-factor factorial response surface.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FactorialCoefficients {
    /// Grand mean / intercept.
    pub intercept: f64,
    /// Central-drive main coefficient.
    pub central_drive: f64,
    /// Local-reflex main coefficient.
    pub local_reflex: f64,
    /// Load-feedback main coefficient.
    pub load_feedback: f64,
    /// Central-drive × local-reflex coefficient.
    pub central_by_reflex: f64,
    /// Central-drive × load-feedback coefficient.
    pub central_by_load: f64,
    /// Local-reflex × load-feedback coefficient.
    pub reflex_by_load: f64,
    /// Three-way central-drive × local-reflex × load-feedback coefficient.
    pub three_way: f64,
}

impl FactorialCoefficients {
    /// Returns coefficients in the same order as FactorialCell::design_row.
    #[must_use]
    pub const fn as_array(self) -> [f64; FACTORIAL_TERMS] {
        [
            self.intercept,
            self.central_drive,
            self.local_reflex,
            self.load_feedback,
            self.central_by_reflex,
            self.central_by_load,
            self.reflex_by_load,
            self.three_way,
        ]
    }

    /// Predicts the response for one factor combination.
    #[must_use]
    pub fn predict(self, cell: FactorialCell) -> f64 {
        let row = cell.design_row();
        let coefficients = self.as_array();
        let mut prediction = 0.0;
        for index in 0..FACTORIAL_TERMS {
            prediction += coefficients[index] * row[index];
        }
        prediction
    }
}

/// Diagnostics for the design matrix, not for an experiment's statistical power.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DesignDiagnostics {
    /// Number of distinct observed factorial cells.
    pub observed_cells: usize,
    /// Number of factorial cells absent from this design.
    pub missing_cells: usize,
    /// Numerical rank of the eight-column design matrix.
    pub matrix_rank: usize,
    /// True only when all eight declared factorial coefficients are identifiable.
    pub all_terms_identifiable: bool,
}

/// Distinct failure modes; non-identifiability is not converted into a fit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesignError {
    /// No factorial observations were supplied.
    EmptyDesign,
    /// More than the eight unique cells were supplied.
    TooManyRows,
    /// A factorial cell appeared more than once; V1 expects one cell estimate per row.
    DuplicateCell,
    /// An outcome contained NaN or infinity.
    NonFiniteOutcome,
    /// The requested interaction-classification tolerance is invalid.
    InvalidTolerance,
    /// The design matrix cannot uniquely identify all eight coefficients.
    NonIdentifiable {
        /// Numerical rank of the supplied design matrix.
        rank: usize,
    },
}

/// Main and interaction-term flags at a preregistered absolute coefficient tolerance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EffectPattern {
    /// Main effects: central drive, local reflex, load feedback.
    pub main_effects: [bool; 3],
    /// Pairwise interactions: central×reflex, central×load, reflex×load.
    pub pairwise_interactions: [bool; 3],
    /// Whether the three-way interaction is nonzero at the supplied tolerance.
    pub three_way_interaction: bool,
}

/// Returns all eight cells in deterministic lexicographic boolean order.
#[must_use]
pub fn full_factorial_cells() -> Vec<FactorialCell> {
    let mut cells = Vec::with_capacity(FACTORIAL_TERMS);
    for central_drive in [false, true] {
        for local_reflex in [false, true] {
            for load_feedback in [false, true] {
                cells.push(FactorialCell::new(
                    central_drive,
                    local_reflex,
                    load_feedback,
                ));
            }
        }
    }
    cells
}

/// Validates the design and reports the rank of its eight-column design matrix.
///
/// Rank eight is necessary and sufficient for unique recovery of all eight
/// coefficients in this saturated cell-level model.
pub fn diagnose_design(
    observations: &[FactorialObservation],
) -> Result<DesignDiagnostics, DesignError> {
    if observations.is_empty() {
        return Err(DesignError::EmptyDesign);
    }
    if observations.len() > FACTORIAL_TERMS {
        return Err(DesignError::TooManyRows);
    }

    let mut observed_cells = Vec::with_capacity(observations.len());
    let mut matrix = Vec::with_capacity(observations.len());
    for observation in observations {
        if !observation.outcome.is_finite() {
            return Err(DesignError::NonFiniteOutcome);
        }
        if observed_cells.contains(&observation.cell) {
            return Err(DesignError::DuplicateCell);
        }
        observed_cells.push(observation.cell);
        matrix.push(observation.cell.design_row());
    }

    let matrix_rank = numerical_rank(matrix);
    let observed_count = observed_cells.len();
    Ok(DesignDiagnostics {
        observed_cells: observed_count,
        missing_cells: FACTORIAL_TERMS - observed_count,
        matrix_rank,
        all_terms_identifiable: matrix_rank == FACTORIAL_TERMS,
    })
}

/// Recovers coefficients only from a full-rank 2x2x2 design.
///
/// The full factorial is orthogonal under ±1 coding, so each coefficient is
/// the corresponding signed sum divided by eight. No optimizer or hidden
/// regularization is involved.
pub fn recover_full_factorial(
    observations: &[FactorialObservation],
) -> Result<FactorialCoefficients, DesignError> {
    let diagnostics = diagnose_design(observations)?;
    if !diagnostics.all_terms_identifiable {
        return Err(DesignError::NonIdentifiable {
            rank: diagnostics.matrix_rank,
        });
    }

    let mut coefficients = [0.0_f64; FACTORIAL_TERMS];
    for observation in observations {
        let row = observation.cell.design_row();
        for index in 0..FACTORIAL_TERMS {
            coefficients[index] += observation.outcome * row[index];
        }
    }
    for coefficient in &mut coefficients {
        *coefficient /= FACTORIAL_TERMS as f64;
    }

    Ok(FactorialCoefficients {
        intercept: coefficients[0],
        central_drive: coefficients[1],
        local_reflex: coefficients[2],
        load_feedback: coefficients[3],
        central_by_reflex: coefficients[4],
        central_by_load: coefficients[5],
        reflex_by_load: coefficients[6],
        three_way: coefficients[7],
    })
}

/// Classifies nonzero terms on the declared additive outcome scale.
///
/// This is not a scale-free definition of synergy. Freeze the tolerance before
/// examining held-out outcomes.
pub fn classify_effects(
    coefficients: FactorialCoefficients,
    absolute_tolerance: f64,
) -> Result<EffectPattern, DesignError> {
    if !absolute_tolerance.is_finite() || absolute_tolerance < 0.0 {
        return Err(DesignError::InvalidTolerance);
    }
    Ok(EffectPattern {
        main_effects: [
            coefficients.central_drive.abs() > absolute_tolerance,
            coefficients.local_reflex.abs() > absolute_tolerance,
            coefficients.load_feedback.abs() > absolute_tolerance,
        ],
        pairwise_interactions: [
            coefficients.central_by_reflex.abs() > absolute_tolerance,
            coefficients.central_by_load.abs() > absolute_tolerance,
            coefficients.reflex_by_load.abs() > absolute_tolerance,
        ],
        three_way_interaction: coefficients.three_way.abs() > absolute_tolerance,
    })
}

fn numerical_rank(mut matrix: Vec<[f64; FACTORIAL_TERMS]>) -> usize {
    let row_count = matrix.len();
    let mut rank = 0;

    for column in 0..FACTORIAL_TERMS {
        if rank >= row_count {
            break;
        }
        let pivot = (rank..row_count).max_by(|left, right| {
            matrix[*left][column].abs().total_cmp(&matrix[*right][column].abs())
        });
        let Some(pivot_row) = pivot else {
            continue;
        };
        if matrix[pivot_row][column].abs() <= RANK_TOLERANCE {
            continue;
        }

        matrix.swap(rank, pivot_row);
        let pivot_value = matrix[rank][column];
        for entry in column..FACTORIAL_TERMS {
            matrix[rank][entry] /= pivot_value;
        }

        for row in 0..row_count {
            if row == rank {
                continue;
            }
            let factor = matrix[row][column];
            if factor.abs() <= RANK_TOLERANCE {
                continue;
            }
            for entry in column..FACTORIAL_TERMS {
                matrix[row][entry] -= factor * matrix[rank][entry];
            }
        }
        rank += 1;
    }

    rank
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(clippy::too_many_arguments)]
    fn coefficients(
        intercept: f64,
        a: f64,
        b: f64,
        c: f64,
        ab: f64,
        ac: f64,
        bc: f64,
        abc: f64,
    ) -> FactorialCoefficients {
        FactorialCoefficients {
            intercept,
            central_drive: a,
            local_reflex: b,
            load_feedback: c,
            central_by_reflex: ab,
            central_by_load: ac,
            reflex_by_load: bc,
            three_way: abc,
        }
    }

    fn observations_from(model: FactorialCoefficients) -> Vec<FactorialObservation> {
        full_factorial_cells()
            .into_iter()
            .map(|cell| FactorialObservation {
                cell,
                outcome: model.predict(cell),
            })
            .collect()
    }

    fn assert_coefficients_close(actual: FactorialCoefficients, expected: FactorialCoefficients) {
        for (left, right) in actual.as_array().into_iter().zip(expected.as_array()) {
            assert!((left - right).abs() < 1.0e-12, "{left} != {right}");
        }
    }

    #[test]
    fn full_factorial_design_has_rank_eight() {
        let observations =
            observations_from(coefficients(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0));
        let diagnostics = diagnose_design(&observations).expect("valid full factorial");
        assert_eq!(diagnostics.observed_cells, 8);
        assert_eq!(diagnostics.missing_cells, 0);
        assert_eq!(diagnostics.matrix_rank, 8);
        assert!(diagnostics.all_terms_identifiable);
    }

    #[test]
    fn recovers_all_known_main_and_interaction_terms() {
        let expected = coefficients(4.0, 1.0, -0.5, 0.25, 0.75, -1.25, 0.5, 2.0);
        let observations = observations_from(expected);
        let actual = recover_full_factorial(&observations).expect("full-rank recovery");
        assert_coefficients_close(actual, expected);
    }

    #[test]
    fn additive_negative_control_has_no_interaction_terms() {
        let expected = coefficients(2.0, 1.0, -0.5, 0.25, 0.0, 0.0, 0.0, 0.0);
        let observations = observations_from(expected);
        let recovered = recover_full_factorial(&observations).expect("full-rank recovery");
        let pattern = classify_effects(recovered, 1.0e-12).expect("valid tolerance");
        assert_eq!(pattern.main_effects, [true, true, true]);
        assert_eq!(pattern.pairwise_interactions, [false, false, false]);
        assert!(!pattern.three_way_interaction);
    }

    #[test]
    fn three_way_interaction_is_separated_from_pairwise_terms() {
        let expected = coefficients(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.5);
        let recovered = recover_full_factorial(&observations_from(expected)).expect("recovery");
        let pattern = classify_effects(recovered, 1.0e-12).expect("valid tolerance");
        assert_eq!(pattern.main_effects, [false, false, false]);
        assert_eq!(pattern.pairwise_interactions, [false, false, false]);
        assert!(pattern.three_way_interaction);
    }

    #[test]
    fn one_missing_factorial_cell_forces_abstention() {
        let mut observations =
            observations_from(coefficients(1.0, 0.5, 0.25, 0.75, 0.2, 0.1, 0.3, 1.0));
        observations.pop();
        let diagnostics = diagnose_design(&observations).expect("valid incomplete design");
        assert_eq!(diagnostics.observed_cells, 7);
        assert_eq!(diagnostics.missing_cells, 1);
        assert_eq!(diagnostics.matrix_rank, 7);
        assert!(!diagnostics.all_terms_identifiable);
        assert_eq!(
            recover_full_factorial(&observations),
            Err(DesignError::NonIdentifiable { rank: 7 })
        );
    }

    #[test]
    fn four_cell_fractional_design_is_rank_deficient() {
        let observations =
            observations_from(coefficients(1.0, 0.5, 0.25, 0.75, 0.2, 0.1, 0.3, 1.0));
        let fractional: Vec<_> = observations
            .into_iter()
            .filter(|observation| observation.cell.design_row()[7] > 0.0)
            .collect();
        let diagnostics = diagnose_design(&fractional).expect("valid fractional design");
        assert_eq!(diagnostics.observed_cells, 4);
        assert_eq!(diagnostics.matrix_rank, 4);
        assert!(!diagnostics.all_terms_identifiable);
    }

    #[test]
    fn duplicate_factorial_cells_are_rejected() {
        let cell = FactorialCell::new(false, false, false);
        let observations = [
            FactorialObservation { cell, outcome: 1.0 },
            FactorialObservation { cell, outcome: 2.0 },
        ];
        assert_eq!(
            diagnose_design(&observations),
            Err(DesignError::DuplicateCell)
        );
    }

    #[test]
    fn non_finite_outcomes_are_rejected() {
        let observations = [FactorialObservation {
            cell: FactorialCell::new(false, false, false),
            outcome: f64::NAN,
        }];
        assert_eq!(
            diagnose_design(&observations),
            Err(DesignError::NonFiniteOutcome)
        );
    }

    #[test]
    fn empty_design_is_rejected() {
        assert_eq!(diagnose_design(&[]), Err(DesignError::EmptyDesign));
    }

    #[test]
    fn invalid_effect_tolerance_is_rejected() {
        let model = coefficients(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
        assert_eq!(
            classify_effects(model, f64::NAN),
            Err(DesignError::InvalidTolerance)
        );
        assert_eq!(
            classify_effects(model, -1.0),
            Err(DesignError::InvalidTolerance)
        );
    }

    #[test]
    fn prediction_matches_the_declared_coded_response_surface() {
        let model = coefficients(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0);
        let cell = FactorialCell::new(true, false, true);
        let expected = 1.0 + 2.0 - 3.0 + 4.0 - 5.0 + 6.0 - 7.0 - 8.0;
        assert!((model.predict(cell) - expected).abs() < 1.0e-12);
    }
}
