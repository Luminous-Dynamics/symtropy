// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Observation-space metrics and an auditable conservation ledger.
//!
//! These utilities measure discrepancies against a declared reference or
//! observation. They do not choose physically meaningful variables, validate
//! an observation operator, set scientific tolerances, or establish model skill.

/// Errors from field comparisons whose samples cannot be compared fairly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldMetricError {
    EmptySample,
    LengthMismatch,
    NonFiniteValue,
    InvalidWeight,
    ZeroTotalWeight,
}

/// Mean absolute error for paired predicted/reference values.
pub fn mean_absolute_error(predicted: &[f64], reference: &[f64]) -> Result<f64, FieldMetricError> {
    validate_pair(predicted, reference)?;
    finite_metric(
        predicted
            .iter()
            .zip(reference)
            .map(|(predicted, reference)| (predicted - reference).abs())
            .sum::<f64>()
            / predicted.len() as f64,
    )
}

/// Root mean square error for paired predicted/reference values.
pub fn root_mean_square_error(
    predicted: &[f64],
    reference: &[f64],
) -> Result<f64, FieldMetricError> {
    validate_pair(predicted, reference)?;
    let mean_square = predicted
        .iter()
        .zip(reference)
        .map(|(predicted, reference)| (predicted - reference).powi(2))
        .sum::<f64>()
        / predicted.len() as f64;
    finite_metric(mean_square.sqrt())
}

/// Mean signed bias, defined as prediction minus reference.
pub fn mean_bias(predicted: &[f64], reference: &[f64]) -> Result<f64, FieldMetricError> {
    validate_pair(predicted, reference)?;
    finite_metric(
        predicted
            .iter()
            .zip(reference)
            .map(|(predicted, reference)| predicted - reference)
            .sum::<f64>()
            / predicted.len() as f64,
    )
}

/// Maximum absolute paired error. Useful for guardrails, not a replacement for distributional metrics.
pub fn maximum_absolute_error(
    predicted: &[f64],
    reference: &[f64],
) -> Result<f64, FieldMetricError> {
    validate_pair(predicted, reference)?;
    finite_metric(
        predicted
            .iter()
            .zip(reference)
            .map(|(predicted, reference)| (predicted - reference).abs())
            .fold(0.0, f64::max),
    )
}

/// Weighted RMSE for a declared observation operator (e.g. cell-area weighting).
///
/// Weights must be finite and non-negative, with positive total weight. The
/// caller is responsible for recording what each weight means and its units.
pub fn weighted_root_mean_square_error(
    predicted: &[f64],
    reference: &[f64],
    weights: &[f64],
) -> Result<f64, FieldMetricError> {
    validate_pair(predicted, reference)?;
    if weights.len() != predicted.len() {
        return Err(FieldMetricError::LengthMismatch);
    }
    if weights.iter().any(|weight| !weight.is_finite()) {
        return Err(FieldMetricError::NonFiniteValue);
    }
    if weights.iter().any(|weight| *weight < 0.0) {
        return Err(FieldMetricError::InvalidWeight);
    }
    let weight_sum = weights.iter().sum::<f64>();
    if !weight_sum.is_finite() {
        return Err(FieldMetricError::NonFiniteValue);
    }
    if weight_sum <= 0.0 {
        return Err(FieldMetricError::ZeroTotalWeight);
    }
    let weighted_mean_square = predicted
        .iter()
        .zip(reference)
        .zip(weights)
        .map(|((predicted, reference), weight)| weight * (predicted - reference).powi(2))
        .sum::<f64>()
        / weight_sum;
    if !weighted_mean_square.is_finite() {
        return Err(FieldMetricError::NonFiniteValue);
    }
    finite_metric(weighted_mean_square.sqrt())
}

fn finite_metric(value: f64) -> Result<f64, FieldMetricError> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(FieldMetricError::NonFiniteValue)
    }
}

fn validate_pair(predicted: &[f64], reference: &[f64]) -> Result<(), FieldMetricError> {
    if predicted.is_empty() || reference.is_empty() {
        return Err(FieldMetricError::EmptySample);
    }
    if predicted.len() != reference.len() {
        return Err(FieldMetricError::LengthMismatch);
    }
    if predicted
        .iter()
        .chain(reference)
        .any(|value| !value.is_finite())
    {
        return Err(FieldMetricError::NonFiniteValue);
    }
    Ok(())
}

fn finite_ledger_value(value: f64) -> Result<f64, LedgerError> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(LedgerError::NonFiniteAmount)
    }
}

/// Errors from a declared scalar conservation ledger.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedgerError {
    EmptyQuantity,
    EmptyUnits,
    NonFiniteAmount,
    NegativeAmount,
    InvalidTolerance,
}

/// One scalar stock/flux ledger, e.g. water volume, sediment mass, or energy.
///
/// Convention: residual = start + inputs - outputs - end. Amounts are totals
/// over one declared interval and use consistent units. The ledger only tests
/// accounting closure; it does not prove that a physical equation is correct.
#[derive(Debug, Clone, PartialEq)]
pub struct ConservationLedger {
    pub quantity: String,
    pub units: String,
    pub start: f64,
    pub inputs: f64,
    pub outputs: f64,
    pub end: f64,
    pub absolute_tolerance: f64,
}

impl ConservationLedger {
    pub fn validate(&self) -> Result<(), LedgerError> {
        if self.quantity.trim().is_empty() {
            return Err(LedgerError::EmptyQuantity);
        }
        if self.units.trim().is_empty() {
            return Err(LedgerError::EmptyUnits);
        }
        if !self.start.is_finite()
            || !self.inputs.is_finite()
            || !self.outputs.is_finite()
            || !self.end.is_finite()
        {
            return Err(LedgerError::NonFiniteAmount);
        }
        if [self.start, self.inputs, self.outputs, self.end]
            .iter()
            .any(|amount| *amount < 0.0)
        {
            return Err(LedgerError::NegativeAmount);
        }
        if !self.absolute_tolerance.is_finite() || self.absolute_tolerance < 0.0 {
            return Err(LedgerError::InvalidTolerance);
        }
        Ok(())
    }

    /// Signed closure residual in the declared units; positive means unaccounted quantity.
    pub fn residual(&self) -> Result<f64, LedgerError> {
        self.validate()?;
        finite_ledger_value(self.start + self.inputs - self.outputs - self.end)
    }

    /// A scale for relative residuals. Returns None for a completely empty ledger.
    pub fn scale(&self) -> Result<Option<f64>, LedgerError> {
        self.validate()?;
        let incoming = self.start + self.inputs;
        let outgoing = self.outputs + self.end;
        let scale = incoming.max(outgoing);
        if !scale.is_finite() {
            return Err(LedgerError::NonFiniteAmount);
        }
        Ok((scale > 0.0).then_some(scale))
    }

    /// Relative signed residual; None when every ledger amount is zero.
    pub fn relative_residual(&self) -> Result<Option<f64>, LedgerError> {
        let residual = self.residual()?;
        match self.scale()? {
            Some(scale) => finite_ledger_value(residual / scale).map(Some),
            None => Ok(None),
        }
    }

    /// Closure within the declared absolute tolerance, after input validation.
    pub fn closes(&self) -> Result<bool, LedgerError> {
        Ok(self.residual()?.abs() <= self.absolute_tolerance)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() <= 1.0e-12,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn paired_metrics_match_hand_calculated_values() {
        let predicted = [2.0, 4.0, 6.0];
        let reference = [1.0, 5.0, 2.0];
        close(mean_absolute_error(&predicted, &reference).unwrap(), 2.0);
        close(
            root_mean_square_error(&predicted, &reference).unwrap(),
            6.0_f64.sqrt(),
        );
        close(mean_bias(&predicted, &reference).unwrap(), 4.0 / 3.0);
        close(
            maximum_absolute_error(&predicted, &reference).unwrap(),
            4.0,
        );
    }

    #[test]
    fn weighted_rmse_respects_declared_sample_weights() {
        let score = weighted_root_mean_square_error(
            &[2.0, 4.0, 6.0],
            &[1.0, 5.0, 2.0],
            &[1.0, 1.0, 2.0],
        )
        .unwrap();
        close(score, 8.5_f64.sqrt());
    }

    #[test]
    fn paired_metrics_fail_closed_on_empty_mismatched_and_non_finite_samples() {
        assert_eq!(
            mean_absolute_error(&[], &[]),
            Err(FieldMetricError::EmptySample)
        );
        assert_eq!(
            root_mean_square_error(&[1.0], &[1.0, 2.0]),
            Err(FieldMetricError::LengthMismatch)
        );
        assert_eq!(
            mean_bias(&[f64::NAN], &[0.0]),
            Err(FieldMetricError::NonFiniteValue)
        );
        assert_eq!(
            weighted_root_mean_square_error(&[1.0], &[0.0], &[]),
            Err(FieldMetricError::LengthMismatch)
        );
        assert_eq!(
            weighted_root_mean_square_error(&[1.0], &[0.0], &[-1.0]),
            Err(FieldMetricError::InvalidWeight)
        );
        assert_eq!(
            weighted_root_mean_square_error(&[1.0], &[0.0], &[0.0]),
            Err(FieldMetricError::ZeroTotalWeight)
        );
    }

    fn balanced_ledger() -> ConservationLedger {
        ConservationLedger {
            quantity: "water_volume".into(),
            units: "m^3".into(),
            start: 100.0,
            inputs: 20.0,
            outputs: 30.0,
            end: 90.0,
            absolute_tolerance: 1.0e-9,
        }
    }

    #[test]
    fn conservation_ledger_computes_closure_and_relative_residual() {
        let ledger = balanced_ledger();
        close(ledger.residual().unwrap(), 0.0);
        close(ledger.relative_residual().unwrap().unwrap(), 0.0);
        assert_eq!(ledger.closes(), Ok(true));
    }

    #[test]
    fn finite_inputs_that_overflow_the_metric_are_rejected() {
        assert_eq!(
            root_mean_square_error(&[f64::MAX], &[-f64::MAX]),
            Err(FieldMetricError::NonFiniteValue)
        );
        assert_eq!(
            mean_absolute_error(&[f64::MAX, f64::MAX], &[-f64::MAX, -f64::MAX]),
            Err(FieldMetricError::NonFiniteValue)
        );
    }

    #[test]
    fn conservation_ledger_rejects_broken_or_undefined_ledgers() {
        let mut broken = balanced_ledger();
        broken.end = 89.0;
        close(broken.residual().unwrap(), 1.0);
        close(broken.relative_residual().unwrap().unwrap(), 1.0 / 120.0);
        assert_eq!(broken.closes(), Ok(false));

        let empty = ConservationLedger {
            quantity: "water_volume".into(),
            units: "m^3".into(),
            start: 0.0,
            inputs: 0.0,
            outputs: 0.0,
            end: 0.0,
            absolute_tolerance: 0.0,
        };
        assert_eq!(empty.relative_residual(), Ok(None));
        assert_eq!(empty.closes(), Ok(true));

        let mut negative = balanced_ledger();
        negative.inputs = -1.0;
        assert_eq!(negative.validate(), Err(LedgerError::NegativeAmount));

        let mut overflowing = balanced_ledger();
        overflowing.start = f64::MAX;
        overflowing.inputs = f64::MAX;
        assert_eq!(overflowing.residual(), Err(LedgerError::NonFiniteAmount));

        let mut invalid_tolerance = balanced_ledger();
        invalid_tolerance.absolute_tolerance = f64::NAN;
        assert_eq!(
            invalid_tolerance.validate(),
            Err(LedgerError::InvalidTolerance)
        );
    }
}
