//! Small, dependency-free linear-Gaussian contact-information reference.
//!
//! This package validates posterior arithmetic and action-conditioned
//! expected information gain for declared observation models. It does not
//! infer that a sensor model is correct, verify a physical action is safe, or
//! select/execute actions. Any safety/viability filter remains independent.

use std::collections::BTreeMap;

pub const MAX_DIMENSION: usize = 8;
const SYMMETRY_TOLERANCE: f64 = 1.0e-10;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GaussianError {
    EmptyDimension,
    DimensionLimitExceeded,
    MatrixLengthMismatch,
    VectorLengthMismatch,
    NonFiniteValue,
    MatrixNotSquare,
    MatrixNotSymmetric,
    MatrixNotPositiveDefinite,
    DuplicateActionId(String),
    EmptyActionId,
    NoActions,
    InformationGainNumericallyNegative,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Matrix {
    rows: usize,
    columns: usize,
    values: Vec<f64>,
}

impl Matrix {
    pub fn new(rows: usize, columns: usize, values: Vec<f64>) -> Result<Self, GaussianError> {
        if rows == 0 || columns == 0 {
            return Err(GaussianError::EmptyDimension);
        }
        if rows > MAX_DIMENSION || columns > MAX_DIMENSION {
            return Err(GaussianError::DimensionLimitExceeded);
        }
        let expected = rows
            .checked_mul(columns)
            .ok_or(GaussianError::MatrixLengthMismatch)?;
        if values.len() != expected {
            return Err(GaussianError::MatrixLengthMismatch);
        }
        if values.iter().any(|value| !value.is_finite()) {
            return Err(GaussianError::NonFiniteValue);
        }
        Ok(Self {
            rows,
            columns,
            values,
        })
    }

    pub fn identity(dimension: usize) -> Result<Self, GaussianError> {
        if dimension == 0 {
            return Err(GaussianError::EmptyDimension);
        }
        if dimension > MAX_DIMENSION {
            return Err(GaussianError::DimensionLimitExceeded);
        }
        let mut values = vec![0.0; dimension * dimension];
        for index in 0..dimension {
            values[index * dimension + index] = 1.0;
        }
        Self::new(dimension, dimension, values)
    }

    pub fn rows(&self) -> usize {
        self.rows
    }

    pub fn columns(&self) -> usize {
        self.columns
    }

    pub fn values(&self) -> &[f64] {
        &self.values
    }

    pub fn get(&self, row: usize, column: usize) -> f64 {
        self.values[row * self.columns + column]
    }

    fn set(&mut self, row: usize, column: usize, value: f64) {
        self.values[row * self.columns + column] = value;
    }

    pub fn transpose(&self) -> Result<Self, GaussianError> {
        let mut result = vec![0.0; self.values.len()];
        for row in 0..self.rows {
            for column in 0..self.columns {
                result[column * self.rows + row] = self.get(row, column);
            }
        }
        Self::new(self.columns, self.rows, result)
    }

    pub fn multiply(&self, right: &Self) -> Result<Self, GaussianError> {
        if self.columns != right.rows {
            return Err(GaussianError::MatrixLengthMismatch);
        }
        let mut result = vec![0.0; self.rows * right.columns];
        for row in 0..self.rows {
            for column in 0..right.columns {
                let mut sum = 0.0;
                for inner in 0..self.columns {
                    sum += self.get(row, inner) * right.get(inner, column);
                }
                if !sum.is_finite() {
                    return Err(GaussianError::NonFiniteValue);
                }
                result[row * right.columns + column] = sum;
            }
        }
        Self::new(self.rows, right.columns, result)
    }

    pub fn add(&self, right: &Self) -> Result<Self, GaussianError> {
        if self.rows != right.rows || self.columns != right.columns {
            return Err(GaussianError::MatrixLengthMismatch);
        }
        let values = self
            .values
            .iter()
            .zip(&right.values)
            .map(|(left, right)| left + right)
            .collect::<Vec<_>>();
        Self::new(self.rows, self.columns, values)
    }

    pub fn multiply_vector(&self, vector: &[f64]) -> Result<Vec<f64>, GaussianError> {
        if self.columns != vector.len() {
            return Err(GaussianError::VectorLengthMismatch);
        }
        if vector.iter().any(|value| !value.is_finite()) {
            return Err(GaussianError::NonFiniteValue);
        }
        let mut result = vec![0.0; self.rows];
        for row in 0..self.rows {
            let mut sum = 0.0;
            for column in 0..self.columns {
                sum += self.get(row, column) * vector[column];
            }
            if !sum.is_finite() {
                return Err(GaussianError::NonFiniteValue);
            }
            result[row] = sum;
        }
        Ok(result)
    }

    pub fn cholesky_lower(&self) -> Result<Self, GaussianError> {
        if self.rows != self.columns {
            return Err(GaussianError::MatrixNotSquare);
        }
        let dimension = self.rows;
        for row in 0..dimension {
            for column in 0..row {
                let left = self.get(row, column);
                let right = self.get(column, row);
                let scale = 1.0_f64.max(left.abs()).max(right.abs());
                if (left - right).abs() > SYMMETRY_TOLERANCE * scale {
                    return Err(GaussianError::MatrixNotSymmetric);
                }
            }
        }

        let mut lower = Self::new(
            dimension,
            dimension,
            vec![0.0; dimension * dimension],
        )?;
        for row in 0..dimension {
            for column in 0..=row {
                let mut value = self.get(row, column);
                for inner in 0..column {
                    value -= lower.get(row, inner) * lower.get(column, inner);
                }
                if row == column {
                    if !value.is_finite() || value <= 0.0 {
                        return Err(GaussianError::MatrixNotPositiveDefinite);
                    }
                    lower.set(row, column, value.sqrt());
                } else {
                    let diagonal = lower.get(column, column);
                    let entry = value / diagonal;
                    if !entry.is_finite() {
                        return Err(GaussianError::NonFiniteValue);
                    }
                    lower.set(row, column, entry);
                }
            }
        }
        Ok(lower)
    }

    pub fn inverse_spd(&self) -> Result<Self, GaussianError> {
        let lower = self.cholesky_lower()?;
        let dimension = self.rows;
        let mut inverse = Self::new(
            dimension,
            dimension,
            vec![0.0; dimension * dimension],
        )?;

        for column in 0..dimension {
            let mut forward = vec![0.0; dimension];
            for row in 0..dimension {
                let mut value = if row == column { 1.0 } else { 0.0 };
                for inner in 0..row {
                    value -= lower.get(row, inner) * forward[inner];
                }
                forward[row] = value / lower.get(row, row);
            }

            let mut solution = vec![0.0; dimension];
            for reverse in 0..dimension {
                let row = dimension - 1 - reverse;
                let mut value = forward[row];
                for inner in (row + 1)..dimension {
                    value -= lower.get(inner, row) * solution[inner];
                }
                solution[row] = value / lower.get(row, row);
            }

            for row in 0..dimension {
                inverse.set(row, column, solution[row]);
            }
        }

        // Remove small solve asymmetry without hiding a failed SPD check.
        for row in 0..dimension {
            for column in 0..row {
                let average = 0.5 * (inverse.get(row, column) + inverse.get(column, row));
                inverse.set(row, column, average);
                inverse.set(column, row, average);
            }
        }
        Ok(inverse)
    }

    pub fn log_determinant_spd(&self) -> Result<f64, GaussianError> {
        let lower = self.cholesky_lower()?;
        let mut log_determinant = 0.0;
        for index in 0..self.rows {
            log_determinant += 2.0 * lower.get(index, index).ln();
        }
        if log_determinant.is_finite() {
            Ok(log_determinant)
        } else {
            Err(GaussianError::NonFiniteValue)
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GaussianBelief {
    pub mean: Vec<f64>,
    pub covariance: Matrix,
}

impl GaussianBelief {
    pub fn new(mean: Vec<f64>, covariance: Matrix) -> Result<Self, GaussianError> {
        if mean.is_empty() {
            return Err(GaussianError::EmptyDimension);
        }
        if mean.len() > MAX_DIMENSION {
            return Err(GaussianError::DimensionLimitExceeded);
        }
        if mean.iter().any(|value| !value.is_finite()) {
            return Err(GaussianError::NonFiniteValue);
        }
        if covariance.rows != mean.len() || covariance.columns != mean.len() {
            return Err(GaussianError::MatrixLengthMismatch);
        }
        // Validate covariance immediately; callers never receive an invalid
        // Gaussian belief that fails only after it has been propagated.
        covariance.cholesky_lower()?;
        Ok(Self { mean, covariance })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ActionConditionedObservation {
    pub action_id: String,
    /// H(u,m) in y = H x + epsilon. Different actions may expose different
    /// latent contact/terrain dimensions.
    pub observation_matrix: Matrix,
    /// R, the calibrated covariance of this action-conditioned observation.
    pub noise_covariance: Matrix,
}

impl ActionConditionedObservation {
    pub fn new(
        action_id: String,
        observation_matrix: Matrix,
        noise_covariance: Matrix,
    ) -> Result<Self, GaussianError> {
        if action_id.trim().is_empty() {
            return Err(GaussianError::EmptyActionId);
        }
        if observation_matrix.rows != noise_covariance.rows
            || noise_covariance.rows != noise_covariance.columns
        {
            return Err(GaussianError::MatrixLengthMismatch);
        }
        noise_covariance.cholesky_lower()?;
        Ok(Self {
            action_id,
            observation_matrix,
            noise_covariance,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PosteriorUpdate {
    pub posterior: GaussianBelief,
    /// Expected information gain for the declared Gaussian observation model,
    /// in nats. It is independent of the realized measurement vector.
    pub expected_information_gain_nats: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ActionInformation {
    pub action_id: String,
    pub expected_information_gain_nats: f64,
}

/// Return the information matrix H^T R^-1 H and its information gain
/// 0.5 * log det(I + L^T H^T R^-1 H L), where P = L L^T.
fn information_terms(
    prior: &GaussianBelief,
    model: &ActionConditionedObservation,
) -> Result<(Matrix, f64), GaussianError> {
    let state_dimension = prior.mean.len();
    if model.observation_matrix.columns != state_dimension {
        return Err(GaussianError::MatrixLengthMismatch);
    }
    let observation_dimension = model.observation_matrix.rows;
    if model.noise_covariance.rows != observation_dimension
        || model.noise_covariance.columns != observation_dimension
    {
        return Err(GaussianError::MatrixLengthMismatch);
    }

    let prior_lower = prior.covariance.cholesky_lower()?;
    let noise_precision = model.noise_covariance.inverse_spd()?;
    let h_transpose = model.observation_matrix.transpose()?;
    let information = h_transpose
        .multiply(&noise_precision)?
        .multiply(&model.observation_matrix)?;

    let left = prior_lower.transpose()?;
    let scaled_information = left.multiply(&information)?.multiply(&prior_lower)?;
    let identity = Matrix::identity(state_dimension)?;
    let determinant_argument = identity.add(&scaled_information)?;
    // The information matrix can be rank deficient; I + (...) remains SPD.
    let expected_information_gain_nats =
        0.5 * determinant_argument.log_determinant_spd()?;

    if expected_information_gain_nats < -1.0e-10 {
        return Err(GaussianError::InformationGainNumericallyNegative);
    }
    Ok((
        information,
        expected_information_gain_nats.max(0.0),
    ))
}

/// Compute expected information gain for one declared action-conditioned
/// observation model. This is a measurement-model calculation, not a safety
/// or action-feasibility verdict.
pub fn expected_information_gain(
    prior: &GaussianBelief,
    model: &ActionConditionedObservation,
) -> Result<f64, GaussianError> {
    Ok(information_terms(prior, model)?.1)
}

/// Apply one realized observation to a Gaussian prior.
///
/// Posterior parameters use the information-form update:
/// P+ = (P^-1 + H^T R^-1 H)^-1
/// mu+ = P+ (P^-1 mu + H^T R^-1 y)
///
/// The returned expected information gain depends on H, R and P, not y.
pub fn update_belief(
    prior: &GaussianBelief,
    model: &ActionConditionedObservation,
    measurement: &[f64],
) -> Result<PosteriorUpdate, GaussianError> {
    if measurement.len() != model.observation_matrix.rows {
        return Err(GaussianError::VectorLengthMismatch);
    }
    if measurement.iter().any(|value| !value.is_finite()) {
        return Err(GaussianError::NonFiniteValue);
    }

    let (information, expected_information_gain_nats) = information_terms(prior, model)?;
    let prior_precision = prior.covariance.inverse_spd()?;
    let noise_precision = model.noise_covariance.inverse_spd()?;
    let h_transpose = model.observation_matrix.transpose()?;

    let posterior_precision = prior_precision.add(&information)?;
    let posterior_covariance = posterior_precision.inverse_spd()?;

    let prior_information_vector = prior_precision.multiply_vector(&prior.mean)?;
    let weighted_measurement = noise_precision.multiply_vector(measurement)?;
    let observation_information_vector =
        h_transpose.multiply_vector(&weighted_measurement)?;
    let information_vector = prior_information_vector
        .iter()
        .zip(observation_information_vector)
        .map(|(left, right)| left + right)
        .collect::<Vec<_>>();
    if information_vector.iter().any(|value| !value.is_finite()) {
        return Err(GaussianError::NonFiniteValue);
    }

    let posterior_mean = posterior_covariance.multiply_vector(&information_vector)?;
    let posterior = GaussianBelief::new(posterior_mean, posterior_covariance)?;
    Ok(PosteriorUpdate {
        posterior,
        expected_information_gain_nats,
    })
}

/// Compare information gain across actions with distinct H(u,m) and/or R.
/// Results are sorted by decreasing expected information gain, then action ID.
/// This function does not certify that any action is physically admissible.
pub fn compare_action_information(
    prior: &GaussianBelief,
    actions: &[ActionConditionedObservation],
) -> Result<Vec<ActionInformation>, GaussianError> {
    if actions.is_empty() {
        return Err(GaussianError::NoActions);
    }
    let mut ids = BTreeMap::<String, ()>::new();
    let mut results = Vec::with_capacity(actions.len());
    for action in actions {
        if ids.insert(action.action_id.clone(), ()).is_some() {
            return Err(GaussianError::DuplicateActionId(action.action_id.clone()));
        }
        results.push(ActionInformation {
            action_id: action.action_id.clone(),
            expected_information_gain_nats: expected_information_gain(prior, action)?,
        });
    }
    results.sort_by(|left, right| {
        right
            .expected_information_gain_nats
            .total_cmp(&left.expected_information_gain_nats)
            .then_with(|| left.action_id.cmp(&right.action_id))
    });
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn matrix(rows: usize, columns: usize, values: &[f64]) -> Matrix {
        Matrix::new(rows, columns, values.to_vec()).unwrap()
    }

    fn prior_1d() -> GaussianBelief {
        GaussianBelief::new(vec![0.0], matrix(1, 1, &[1.0])).unwrap()
    }

    fn action_1d(id: &str, h: &[f64], r: &[f64]) -> ActionConditionedObservation {
        ActionConditionedObservation::new(
            id.to_owned(),
            matrix(1, 1, h),
            matrix(1, 1, r),
        )
        .unwrap()
    }

    #[test]
    fn scalar_posterior_matches_closed_form() {
        let prior = prior_1d();
        let model = action_1d("normal-probe", &[1.0], &[1.0]);
        let result = update_belief(&prior, &model, &[2.0]).unwrap();

        assert!((result.posterior.mean[0] - 1.0).abs() < 1.0e-12);
        assert!((result.posterior.covariance.get(0, 0) - 0.5).abs() < 1.0e-12);
        assert!((result.expected_information_gain_nats - 0.5 * 2.0_f64.ln()).abs() < 1.0e-12);
    }

    #[test]
    fn rank_deficient_observation_only_updates_observable_contact_direction() {
        let prior = GaussianBelief::new(
            vec![0.0, 0.0],
            matrix(2, 2, &[1.0, 0.0, 0.0, 1.0]),
        )
        .unwrap();
        let action = ActionConditionedObservation::new(
            "normal-only".to_owned(),
            matrix(1, 2, &[1.0, 0.0]),
            matrix(1, 1, &[1.0]),
        )
        .unwrap();
        let result = update_belief(&prior, &action, &[1.0]).unwrap();

        assert!((result.posterior.mean[0] - 0.5).abs() < 1.0e-12);
        assert!(result.posterior.mean[1].abs() < 1.0e-12);
        assert!((result.posterior.covariance.get(0, 0) - 0.5).abs() < 1.0e-12);
        assert!((result.posterior.covariance.get(1, 1) - 1.0).abs() < 1.0e-12);
        assert!(result.posterior.covariance.get(0, 1).abs() < 1.0e-12);
    }

    #[test]
    fn action_conditioned_observation_maps_change_expected_information() {
        let prior = GaussianBelief::new(
            vec![0.0, 0.0],
            matrix(2, 2, &[1.0, 0.0, 0.0, 1.0]),
        )
        .unwrap();
        let normal_only = ActionConditionedObservation::new(
            "normal-only".to_owned(),
            matrix(1, 2, &[1.0, 0.0]),
            matrix(1, 1, &[1.0]),
        )
        .unwrap();
        let coupled_probe = ActionConditionedObservation::new(
            "coupled-probe".to_owned(),
            matrix(1, 2, &[1.0, 1.0]),
            matrix(1, 1, &[1.0]),
        )
        .unwrap();

        let ranked = compare_action_information(&prior, &[normal_only, coupled_probe]).unwrap();
        assert_eq!(ranked[0].action_id, "coupled-probe");
        assert_eq!(ranked[1].action_id, "normal-only");
        assert!((ranked[0].expected_information_gain_nats - 0.5 * 3.0_f64.ln()).abs() < 1.0e-12);
        assert!((ranked[1].expected_information_gain_nats - 0.5 * 2.0_f64.ln()).abs() < 1.0e-12);
    }

    #[test]
    fn expected_information_gain_is_independent_of_realized_measurement() {
        let prior = prior_1d();
        let model = action_1d("same-probe", &[1.0], &[1.0]);
        let first = update_belief(&prior, &model, &[0.0]).unwrap();
        let second = update_belief(&prior, &model, &[2.0]).unwrap();

        assert_eq!(
            first.expected_information_gain_nats,
            second.expected_information_gain_nats
        );
        assert!((first.posterior.mean[0] - 0.0).abs() < 1.0e-12);
        assert!((second.posterior.mean[0] - 1.0).abs() < 1.0e-12);
    }

    #[test]
    fn zero_observation_matrix_has_zero_information_and_unchanged_belief() {
        let prior = GaussianBelief::new(
            vec![0.2, -0.1],
            matrix(2, 2, &[1.0, 0.2, 0.2, 2.0]),
        )
        .unwrap();
        let model = ActionConditionedObservation::new(
            "no-information".to_owned(),
            matrix(1, 2, &[0.0, 0.0]),
            matrix(1, 1, &[1.0]),
        )
        .unwrap();
        let result = update_belief(&prior, &model, &[100.0]).unwrap();

        assert!(result.expected_information_gain_nats.abs() < 1.0e-12);
        assert!((result.posterior.mean[0] - prior.mean[0]).abs() < 1.0e-12);
        assert!((result.posterior.mean[1] - prior.mean[1]).abs() < 1.0e-12);
        for (left, right) in result.posterior.covariance.values().iter().zip(prior.covariance.values()) {
            assert!((left - right).abs() < 1.0e-12);
        }
    }

    #[test]
    fn correlated_observation_noise_is_supported_without_independence_assumption() {
        let prior = GaussianBelief::new(
            vec![0.0, 0.0],
            matrix(2, 2, &[1.0, 0.0, 0.0, 1.0]),
        )
        .unwrap();
        let model = ActionConditionedObservation::new(
            "correlated-sensors".to_owned(),
            matrix(2, 2, &[1.0, 0.0, 0.0, 1.0]),
            matrix(2, 2, &[1.0, 0.25, 0.25, 1.0]),
        )
        .unwrap();
        let result = update_belief(&prior, &model, &[0.5, -0.5]).unwrap();
        assert!(result.expected_information_gain_nats > 0.0);
        assert!(result.posterior.covariance.cholesky_lower().is_ok());
    }

    #[test]
    fn singular_noise_covariance_is_rejected() {
        let result = ActionConditionedObservation::new(
            "singular-noise".to_owned(),
            matrix(2, 2, &[1.0, 0.0, 0.0, 1.0]),
            matrix(2, 2, &[1.0, 1.0, 1.0, 1.0]),
        );
        assert_eq!(result, Err(GaussianError::MatrixNotPositiveDefinite));
    }

    #[test]
    fn nonsymmetric_covariance_is_rejected() {
        let result = Matrix::new(2, 2, vec![1.0, 0.1, 0.2, 1.0])
            .unwrap()
            .cholesky_lower();
        assert_eq!(result, Err(GaussianError::MatrixNotSymmetric));
    }

    #[test]
    fn indefinite_covariance_is_rejected() {
        let result = Matrix::new(2, 2, vec![1.0, 2.0, 2.0, 1.0])
            .unwrap()
            .cholesky_lower();
        assert_eq!(result, Err(GaussianError::MatrixNotPositiveDefinite));
    }

    #[test]
    fn invalid_shapes_and_measurement_lengths_are_rejected() {
        let prior = prior_1d();
        let wrong_h = ActionConditionedObservation::new(
            "wrong-dimension".to_owned(),
            matrix(1, 2, &[1.0, 0.0]),
            matrix(1, 1, &[1.0]),
        )
        .unwrap();
        assert_eq!(
            expected_information_gain(&prior, &wrong_h),
            Err(GaussianError::MatrixLengthMismatch)
        );

        let valid = action_1d("valid", &[1.0], &[1.0]);
        assert_eq!(
            update_belief(&prior, &valid, &[1.0, 2.0]),
            Err(GaussianError::VectorLengthMismatch)
        );
    }

    #[test]
    fn nonfinite_values_and_empty_ids_are_rejected() {
        assert_eq!(
            Matrix::new(1, 1, vec![f64::NAN]),
            Err(GaussianError::NonFiniteValue)
        );
        assert_eq!(
            ActionConditionedObservation::new(
                " ".to_owned(),
                matrix(1, 1, &[1.0]),
                matrix(1, 1, &[1.0]),
            ),
            Err(GaussianError::EmptyActionId)
        );
    }

    #[test]
    fn action_ids_must_be_unique_and_action_list_nonempty() {
        let prior = prior_1d();
        let action_a = action_1d("duplicate", &[1.0], &[1.0]);
        let action_b = action_1d("duplicate", &[2.0], &[1.0]);
        assert_eq!(
            compare_action_information(&prior, &[action_a.clone(), action_b]),
            Err(GaussianError::DuplicateActionId("duplicate".to_owned()))
        );
        assert_eq!(
            compare_action_information(&prior, &[]),
            Err(GaussianError::NoActions)
        );
    }

    #[test]
    fn cholesky_inverse_and_logdet_agree_for_diagonal_matrix() {
        let covariance = matrix(2, 2, &[4.0, 0.0, 0.0, 9.0]);
        let inverse = covariance.inverse_spd().unwrap();
        assert!((inverse.get(0, 0) - 0.25).abs() < 1.0e-12);
        assert!((inverse.get(1, 1) - (1.0 / 9.0)).abs() < 1.0e-12);
        assert!((covariance.log_determinant_spd() - 4.0_f64.ln() - 9.0_f64.ln()).abs() < 1.0e-12);
    }
}
