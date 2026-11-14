//! Exact finite-dimensional source-to-observer transfer controls.
//!
//! For a declared frequency-domain pencil L, source B, observation C, and
//! direct channel D, the transfer map is H = D + C L^(-1) B.
//! This crate solves linear systems rather than explicitly forming L^(-1).
//!
//! Its first variation is
//!
//! δH = δD + δC G B - C G δL G B + C G δB,  G = L^(-1).
//!
//! This is a mathematical reference primitive. It is not a Teukolsky solver,
//! and it does not identify arbitrary matrix perturbations with physical
//! spacetime or environmental perturbations.

use core::fmt;

use nalgebra::DMatrix;
use num_complex::Complex64;

/// Diagnostics accompanying one finite-dimensional transfer evaluation.
#[derive(Debug, Clone, PartialEq)]
pub struct TransferReport {
    /// Complex source-to-observer transfer matrix.
    pub response: DMatrix<Complex64>,
    /// Smallest singular value of the frequency-domain pencil L.
    pub sigma_min_pencil: f64,
    /// 2-norm condition estimate of L.
    ///
    /// Callers must compare this with their declared numerical error budget.
    /// No universal condition-number threshold is imposed by this crate.
    pub condition_2: f64,
}

/// Fail-closed errors for malformed, singular, or non-finite problems.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferError {
    EmptyPencil,
    NonSquarePencil,
    EmptySource,
    EmptyObservation,
    DimensionMismatch,
    NonFiniteInput,
    SingularPencil,
    NonFiniteResult,
}

impl fmt::Display for TransferError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyPencil => write!(f, "frequency-domain pencil must be non-empty"),
            Self::NonSquarePencil => write!(f, "frequency-domain pencil must be square"),
            Self::EmptySource => write!(f, "source map must have at least one input column"),
            Self::EmptyObservation => {
                write!(f, "observation map must have at least one output row")
            }
            Self::DimensionMismatch => write!(
                f,
                "pencil, source, observation and direct maps have incompatible dimensions"
            ),
            Self::NonFiniteInput => write!(f, "all input matrices must contain only finite values"),
            Self::SingularPencil => write!(
                f,
                "frequency-domain pencil is singular at the requested frequency"
            ),
            Self::NonFiniteResult => write!(f, "transfer calculation produced a non-finite result"),
        }
    }
}

impl std::error::Error for TransferError {}

fn finite_scalar(value: Complex64) -> bool {
    value.re.is_finite() && value.im.is_finite()
}

fn finite_matrix(matrix: &DMatrix<Complex64>) -> bool {
    matrix.iter().all(|value| finite_scalar(*value))
}

fn validate_shapes(
    pencil: &DMatrix<Complex64>,
    source: &DMatrix<Complex64>,
    observation: &DMatrix<Complex64>,
    direct: &DMatrix<Complex64>,
) -> Result<(usize, usize, usize), TransferError> {
    if pencil.nrows() == 0 || pencil.ncols() == 0 {
        return Err(TransferError::EmptyPencil);
    }
    if !pencil.is_square() {
        return Err(TransferError::NonSquarePencil);
    }
    if source.ncols() == 0 {
        return Err(TransferError::EmptySource);
    }
    if observation.nrows() == 0 {
        return Err(TransferError::EmptyObservation);
    }

    let state_dimension = pencil.nrows();
    let input_dimension = source.ncols();
    let output_dimension = observation.nrows();

    if source.nrows() != state_dimension
        || observation.ncols() != state_dimension
        || direct.nrows() != output_dimension
        || direct.ncols() != input_dimension
    {
        return Err(TransferError::DimensionMismatch);
    }

    if !finite_matrix(pencil)
        || !finite_matrix(source)
        || !finite_matrix(observation)
        || !finite_matrix(direct)
    {
        return Err(TransferError::NonFiniteInput);
    }

    Ok((state_dimension, output_dimension, input_dimension))
}

fn solve(
    pencil: &DMatrix<Complex64>,
    right_hand_side: &DMatrix<Complex64>,
) -> Result<DMatrix<Complex64>, TransferError> {
    let solution = pencil
        .clone()
        .lu()
        .solve(right_hand_side)
        .ok_or(TransferError::SingularPencil)?;

    if !finite_matrix(&solution) {
        return Err(TransferError::NonFiniteResult);
    }
    Ok(solution)
}

fn pencil_diagnostics(pencil: &DMatrix<Complex64>) -> Result<(f64, f64), TransferError> {
    let singular_values = pencil.clone().svd(false, false).singular_values;
    let sigma_max = singular_values.iter().copied().fold(0.0_f64, f64::max);
    let sigma_min = singular_values
        .iter()
        .copied()
        .fold(f64::INFINITY, f64::min);

    if !sigma_min.is_finite() || !sigma_max.is_finite() || sigma_min <= 0.0 {
        return Err(TransferError::SingularPencil);
    }

    let condition_2 = sigma_max / sigma_min;
    if !condition_2.is_finite() {
        return Err(TransferError::NonFiniteResult);
    }
    Ok((sigma_min, condition_2))
}

/// Evaluate H = D + C L^(-1) B with conditioning diagnostics.
///
/// The function solves L X = B and returns D + C X. It does not explicitly
/// form the inverse of L.
///
/// # Errors
///
/// Returns a typed error for invalid dimensions, non-finite inputs, or a
/// singular pencil. A finite but ill-conditioned pencil is returned with its
/// condition estimate; the caller is responsible for applying a declared
/// numerical acceptance threshold.
pub fn transfer_response(
    pencil: &DMatrix<Complex64>,
    source: &DMatrix<Complex64>,
    observation: &DMatrix<Complex64>,
    direct: &DMatrix<Complex64>,
) -> Result<TransferReport, TransferError> {
    validate_shapes(pencil, source, observation, direct)?;
    let (sigma_min_pencil, condition_2) = pencil_diagnostics(pencil)?;
    let state = solve(pencil, source)?;
    let response = observation * &state + direct;

    if !finite_matrix(&response) {
        return Err(TransferError::NonFiniteResult);
    }

    Ok(TransferReport {
        response,
        sigma_min_pencil,
        condition_2,
    })
}

/// Perturbations of the four maps in the source-to-observer transfer function.
#[derive(Debug, Clone, Copy)]
pub struct TransferPerturbation<'a> {
    /// Perturbation of the frequency-domain operator pencil.
    pub delta_pencil: &'a DMatrix<Complex64>,
    /// Perturbation of source excitation / initial data.
    pub delta_source: &'a DMatrix<Complex64>,
    /// Perturbation of the observation / extraction map.
    pub delta_observation: &'a DMatrix<Complex64>,
    /// Perturbation of the direct / feedthrough channel.
    pub delta_direct: &'a DMatrix<Complex64>,
}

/// Evaluate the first variation of H with respect to L, B, C, and D.
///
/// All perturbation matrices must have exactly the same shape as their
/// corresponding baseline matrices. The result is the differential at the
/// baseline, not a finite-step prediction.
///
/// The implementation uses
///
///     X = L^(-1) B
///     δX = L^(-1) (δB - δL X)
///     δH = δD + δC X + C δX.
///
/// # Errors
///
/// Returns a typed error for incompatible dimensions, non-finite data, or a
/// singular baseline pencil.
pub fn first_order_response_variation(
    pencil: &DMatrix<Complex64>,
    source: &DMatrix<Complex64>,
    observation: &DMatrix<Complex64>,
    direct: &DMatrix<Complex64>,
    perturbation: TransferPerturbation<'_>,
) -> Result<DMatrix<Complex64>, TransferError> {
    let TransferPerturbation {
        delta_pencil,
        delta_source,
        delta_observation,
        delta_direct,
    } = perturbation;
    let (state_dimension, output_dimension, input_dimension) =
        validate_shapes(pencil, source, observation, direct)?;

    if delta_pencil.nrows() != state_dimension
        || delta_pencil.ncols() != state_dimension
        || delta_source.nrows() != state_dimension
        || delta_source.ncols() != input_dimension
        || delta_observation.nrows() != output_dimension
        || delta_observation.ncols() != state_dimension
        || delta_direct.nrows() != output_dimension
        || delta_direct.ncols() != input_dimension
    {
        return Err(TransferError::DimensionMismatch);
    }

    if !finite_matrix(delta_pencil)
        || !finite_matrix(delta_source)
        || !finite_matrix(delta_observation)
        || !finite_matrix(delta_direct)
    {
        return Err(TransferError::NonFiniteInput);
    }

    let state = solve(pencil, source)?;
    let delta_rhs = delta_source - delta_pencil * &state;
    let delta_state = solve(pencil, &delta_rhs)?;
    let delta_response = delta_observation * &state + observation * delta_state + delta_direct;

    if !finite_matrix(&delta_response) {
        return Err(TransferError::NonFiniteResult);
    }

    Ok(delta_response)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn z(re: f64, im: f64) -> Complex64 {
        Complex64::new(re, im)
    }

    fn diagonal(first: Complex64, second: Complex64) -> DMatrix<Complex64> {
        DMatrix::from_row_slice(2, 2, &[first, z(0.0, 0.0), z(0.0, 0.0), second])
    }

    fn scalar(value: Complex64) -> DMatrix<Complex64> {
        DMatrix::from_element(1, 1, value)
    }

    fn two_mode_pencil(s: Complex64) -> DMatrix<Complex64> {
        diagonal(s + z(1.0, 0.0), s + z(2.0, 0.0))
    }

    fn base_maps() -> (DMatrix<Complex64>, DMatrix<Complex64>, DMatrix<Complex64>) {
        let source = DMatrix::from_column_slice(2, 1, &[z(1.0, 0.0), z(1.0, 0.0)]);
        let observation = DMatrix::from_row_slice(1, 2, &[z(1.0, 0.0), z(1.0, 0.0)]);
        let direct = scalar(z(0.0, 0.0));
        (source, observation, direct)
    }

    fn evaluate(
        s: Complex64,
        source: &DMatrix<Complex64>,
        observation: &DMatrix<Complex64>,
        direct: &DMatrix<Complex64>,
    ) -> TransferReport {
        transfer_response(&two_mode_pencil(s), source, observation, direct).unwrap()
    }

    #[test]
    fn diagonal_two_mode_response_matches_analytic_oracle() {
        let s = z(0.4, 0.7);
        let (source, observation, direct) = base_maps();
        let report = evaluate(s, &source, &observation, &direct);
        let expected = z(1.0, 0.0) / (s + z(1.0, 0.0)) + z(1.0, 0.0) / (s + z(2.0, 0.0));

        assert!((report.response[(0, 0)] - expected).norm() < 1.0e-12);
        assert!(report.sigma_min_pencil > 0.0);
        assert!(report.condition_2 >= 1.0);
    }

    #[test]
    fn source_only_intervention_changes_residue_not_the_operator_pencil() {
        let s = z(0.4, 0.7);
        let (source, observation, direct) = base_maps();
        let alternate_source = DMatrix::from_column_slice(2, 1, &[z(1.0, 0.0), z(0.25, 0.0)]);

        // Pass the identical operator pencil to both evaluations. Only B changes.
        let pencil = two_mode_pencil(s);
        let baseline = transfer_response(&pencil, &source, &observation, &direct).unwrap();
        let changed = transfer_response(&pencil, &alternate_source, &observation, &direct).unwrap();
        let expected_changed = z(1.0, 0.0) / (s + z(1.0, 0.0)) + z(0.25, 0.0) / (s + z(2.0, 0.0));

        assert!((changed.response[(0, 0)] - expected_changed).norm() < 1.0e-12);
        assert!((changed.response[(0, 0)] - baseline.response[(0, 0)]).norm() > 1.0e-3);
    }

    #[test]
    fn observation_only_intervention_changes_residue_not_the_operator_pencil() {
        let s = z(0.4, 0.7);
        let (source, observation, direct) = base_maps();
        let alternate_observation = DMatrix::from_row_slice(1, 2, &[z(1.0, 0.0), z(0.5, 0.0)]);

        let baseline = evaluate(s, &source, &observation, &direct);
        let changed = evaluate(s, &source, &alternate_observation, &direct);
        let expected_changed = z(1.0, 0.0) / (s + z(1.0, 0.0)) + z(0.5, 0.0) / (s + z(2.0, 0.0));

        assert!((changed.response[(0, 0)] - expected_changed).norm() < 1.0e-12);
        assert!((changed.response[(0, 0)] - baseline.response[(0, 0)]).norm() > 1.0e-3);
    }

    #[test]
    fn inaccessible_or_unobservable_modes_are_absent_from_the_transfer_map() {
        let direct = scalar(z(0.0, 0.0));
        let both_modes = DMatrix::from_column_slice(2, 1, &[z(1.0, 0.0), z(1.0, 0.0)]);
        let first_mode_only = DMatrix::from_column_slice(2, 1, &[z(1.0, 0.0), z(0.0, 0.0)]);
        let observe_both = DMatrix::from_row_slice(1, 2, &[z(1.0, 0.0), z(1.0, 0.0)]);
        let observe_first_only = DMatrix::from_row_slice(1, 2, &[z(1.0, 0.0), z(0.0, 0.0)]);

        for s in [z(0.2, 0.7), z(1.1, -0.3), z(-0.5, 0.8)] {
            let pencil = two_mode_pencil(s);
            let expected = z(1.0, 0.0) / (s + z(1.0, 0.0));

            // The second mode is present in L but cannot be excited by B.
            let source_hidden =
                transfer_response(&pencil, &first_mode_only, &observe_both, &direct).unwrap();
            // The second mode is excited, but C cannot observe it.
            let observer_hidden =
                transfer_response(&pencil, &both_modes, &observe_first_only, &direct).unwrap();

            assert!((source_hidden.response[(0, 0)] - expected).norm() < 1.0e-12);
            assert!((observer_hidden.response[(0, 0)] - expected).norm() < 1.0e-12);
        }
    }

    #[test]
    fn operator_only_intervention_moves_the_analytic_pole() {
        let s = z(0.4, 0.7);
        let (source, observation, direct) = base_maps();
        let changed_pencil = diagonal(s + z(1.25, 0.0), s + z(2.0, 0.0));
        let changed = transfer_response(&changed_pencil, &source, &observation, &direct).unwrap();
        let expected = z(1.0, 0.0) / (s + z(1.25, 0.0)) + z(1.0, 0.0) / (s + z(2.0, 0.0));

        assert!((changed.response[(0, 0)] - expected).norm() < 1.0e-12);
    }

    #[test]
    fn matched_state_similarity_preserves_transfer_response() {
        let pencil =
            DMatrix::from_row_slice(2, 2, &[z(1.2, 0.3), z(0.2, 0.0), z(0.0, 0.1), z(2.1, -0.1)]);
        let source = DMatrix::from_column_slice(2, 1, &[z(1.0, 0.2), z(0.4, -0.1)]);
        let observation = DMatrix::from_row_slice(1, 2, &[z(0.7, 0.0), z(1.1, 0.1)]);
        let direct = scalar(z(0.05, 0.02));

        let scale = diagonal(z(10.0, 0.0), z(0.1, 0.0));
        let inverse_scale = diagonal(z(0.1, 0.0), z(10.0, 0.0));
        let transformed_pencil = &scale * &pencil * &inverse_scale;
        let transformed_source = &scale * &source;
        let transformed_observation = &observation * &inverse_scale;

        let original = transfer_response(&pencil, &source, &observation, &direct).unwrap();
        let transformed = transfer_response(
            &transformed_pencil,
            &transformed_source,
            &transformed_observation,
            &direct,
        )
        .unwrap();

        assert!((original.response - transformed.response).norm() < 1.0e-12);
    }

    #[test]
    fn direct_channel_is_added_without_conflating_it_with_the_resonant_term() {
        let s = z(0.4, 0.7);
        let (source, observation, _) = base_maps();
        let direct = scalar(z(0.12, -0.03));
        let report = evaluate(s, &source, &observation, &direct);
        let resonant = z(1.0, 0.0) / (s + z(1.0, 0.0)) + z(1.0, 0.0) / (s + z(2.0, 0.0));

        assert!((report.response[(0, 0)] - (resonant + direct[(0, 0)])).norm() < 1.0e-12);
    }

    #[test]
    fn first_variation_matches_central_finite_differences_with_second_order_error() {
        let pencil =
            DMatrix::from_row_slice(2, 2, &[z(1.2, 0.3), z(0.2, 0.0), z(0.0, 0.1), z(2.1, -0.1)]);
        let source = DMatrix::from_column_slice(2, 1, &[z(1.0, 0.2), z(0.4, -0.1)]);
        let observation = DMatrix::from_row_slice(1, 2, &[z(0.7, 0.0), z(1.1, 0.1)]);
        let direct = scalar(z(0.05, 0.02));
        let delta_pencil = DMatrix::from_row_slice(
            2,
            2,
            &[z(0.0, 0.1), z(0.03, 0.0), z(-0.02, 0.0), z(0.04, 0.0)],
        );
        let delta_source = DMatrix::from_column_slice(2, 1, &[z(0.02, 0.0), z(0.0, 0.1)]);
        let delta_observation = DMatrix::from_row_slice(1, 2, &[z(0.01, 0.0), z(0.0, -0.03)]);
        let delta_direct = scalar(z(0.0, 0.02));

        let perturbation = TransferPerturbation {
            delta_pencil: &delta_pencil,
            delta_source: &delta_source,
            delta_observation: &delta_observation,
            delta_direct: &delta_direct,
        };
        let derivative =
            first_order_response_variation(&pencil, &source, &observation, &direct, perturbation)
                .unwrap();

        let central_difference = |eta: f64| {
            let plus = transfer_response(
                &(&pencil + &delta_pencil * z(eta, 0.0)),
                &(&source + &delta_source * z(eta, 0.0)),
                &(&observation + &delta_observation * z(eta, 0.0)),
                &(&direct + &delta_direct * z(eta, 0.0)),
            )
            .unwrap();
            let minus = transfer_response(
                &(&pencil - &delta_pencil * z(eta, 0.0)),
                &(&source - &delta_source * z(eta, 0.0)),
                &(&observation - &delta_observation * z(eta, 0.0)),
                &(&direct - &delta_direct * z(eta, 0.0)),
            )
            .unwrap();
            (plus.response - minus.response) * z(0.5 / eta, 0.0)
        };

        let error_coarse = (central_difference(1.0e-2) - &derivative).norm();
        let error_fine = (central_difference(5.0e-3) - &derivative).norm();

        assert!(error_coarse.is_finite() && error_fine.is_finite());
        assert!(error_fine < error_coarse);
        assert!(error_coarse / error_fine > 3.8);
        assert!(error_coarse / error_fine < 4.2);
    }

    #[test]
    fn singular_and_nearly_singular_pencils_are_not_silently_regularized() {
        let (source, observation, direct) = base_maps();
        let singular = diagonal(z(1.0, 0.0), z(0.0, 0.0));
        assert_eq!(
            transfer_response(&singular, &source, &observation, &direct),
            Err(TransferError::SingularPencil)
        );

        let nearly_singular = diagonal(z(1.0, 0.0), z(1.0e-14, 0.0));
        let report = transfer_response(&nearly_singular, &source, &observation, &direct).unwrap();
        assert!(report.condition_2 >= 1.0e13);
    }

    #[test]
    fn invalid_dimensions_and_non_finite_inputs_fail_closed() {
        let (source, observation, direct) = base_maps();
        let pencil = two_mode_pencil(z(0.4, 0.7));
        let mismatched_observation =
            DMatrix::from_row_slice(1, 3, &[z(1.0, 0.0), z(1.0, 0.0), z(0.0, 0.0)]);
        assert_eq!(
            transfer_response(&pencil, &source, &mismatched_observation, &direct),
            Err(TransferError::DimensionMismatch)
        );

        let non_finite_source = DMatrix::from_column_slice(2, 1, &[z(f64::NAN, 0.0), z(1.0, 0.0)]);
        assert_eq!(
            transfer_response(&pencil, &non_finite_source, &observation, &direct),
            Err(TransferError::NonFiniteInput)
        );
    }

    #[test]
    fn perturbation_dimension_mismatch_fails_closed() {
        let (source, observation, direct) = base_maps();
        let pencil = two_mode_pencil(z(0.4, 0.7));
        let delta_pencil = DMatrix::zeros(1, 1);
        let delta_source = DMatrix::zeros(2, 1);
        let delta_observation = DMatrix::zeros(1, 2);
        let delta_direct = scalar(z(0.0, 0.0));

        assert_eq!(
            first_order_response_variation(
                &pencil,
                &source,
                &observation,
                &direct,
                TransferPerturbation {
                    delta_pencil: &delta_pencil,
                    delta_source: &delta_source,
                    delta_observation: &delta_observation,
                    delta_direct: &delta_direct,
                },
            ),
            Err(TransferError::DimensionMismatch)
        );
    }
}
