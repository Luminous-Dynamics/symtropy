// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Deterministic, low-order known-truth cases for GEO-SIM-001.
//!
//! These fixtures are analytic observation-model controls, not an Earth model,
//! geodynamo solver, or evidence of real-Earth state recovery. The estimator-facing
//! module exposes only the observation operator and synthetic observations. Truth
//! fixtures live in the separately gated `scoring` module, compiled only for unit
//! tests or with the explicit `benchmark-scoring` feature.

const SYMMETRY_TOLERANCE: f64 = 1.0e-12;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FixtureId {
    ComplementarySensors,
    WeakHighMode,
    ExactNullMode,
    CorrelatedDuplicate,
}

impl FixtureId {
    pub const ALL: [Self; 4] = [
        Self::ComplementarySensors,
        Self::WeakHighMode,
        Self::ExactNullMode,
        Self::CorrelatedDuplicate,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ComplementarySensors => "h0-complementary-sensors",
            Self::WeakHighMode => "h0-weak-high-mode",
            Self::ExactNullMode => "h0-exact-null-mode",
            Self::CorrelatedDuplicate => "h0-correlated-duplicate",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TruthVisibility {
    WithheldUntilScoring,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvidenceClass {
    SyntheticKnownTruth,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FixtureProvenance {
    pub generator_id: &'static str,
    pub generator_version: &'static str,
    pub truth_visibility: TruthVisibility,
    pub evidence_class: EvidenceClass,
    pub randomness: &'static str,
    pub units_note: &'static str,
    pub claim_ceiling: &'static str,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PublicObservationFixture {
    pub fixture_id: FixtureId,
    pub parameter_ids: [&'static str; 2],
    pub observation_ids: [&'static str; 2],
    /// Declared acquisition cost for each observation channel in normalized cost units.
    pub observation_costs: [f64; 2],
    /// Observation matrix H, with observations in rows and state coordinates in columns.
    pub observation_operator: [[f64; 2]; 2],
    /// Full measurement-error covariance. Off-diagonal terms are intentional.
    pub observation_covariance: [[f64; 2]; 2],
    /// Covariance of the declared Gaussian state prior in dimensionless proxy coordinates.
    pub prior_covariance: [[f64; 2]; 2],
    /// Noise-free analytic observations, y = Hx, for the paired hidden state(s).
    pub observed_values: [f64; 2],
    /// Relative simulation seconds, not UTC timestamps or time in the real Earth.
    pub model_relative_seconds: [f64; 2],
    pub observation_units: &'static str,
    pub provenance: FixtureProvenance,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FixtureValidationError {
    DuplicateParameterId,
    DuplicateObservationId,
    NonFiniteOperator,
    NonFiniteObservation,
    NonFiniteCovariance,
    AsymmetricCovariance,
    CovarianceNotPositiveDefinite,
    InvalidTimeAxis,
    NonFiniteState,
    InvalidObservationCost,
    InvalidTolerance,
    NonFiniteIntermediate,
}
/// Local, covariance-whitened observability summary for this 2x2 H0 control.
///
/// Numerical rank depends on the declared tolerances and observation covariance.
/// This is a tiny analytic reference diagnostic, not a global-identifiability
/// proof or a replacement for a reviewed SVD backend on large problems.
#[derive(Clone, Debug, PartialEq)]
pub struct ObservabilitySummary {
    pub whitened_operator: [[f64; 2]; 2],
    /// Singular values in descending order.
    pub singular_values: [f64; 2],
    pub numerical_rank: usize,
    pub condition_number: Option<f64>,
    /// Right-space null/weak directions in parameter order.
    pub null_space_basis: Vec<[f64; 2]>,
    pub absolute_tolerance: f64,
    pub relative_tolerance: f64,
}

fn validate_covariance(matrix: [[f64; 2]; 2]) -> Result<(), FixtureValidationError> {
    if matrix.iter().flatten().any(|value| !value.is_finite()) {
        return Err(FixtureValidationError::NonFiniteCovariance);
    }
    if (matrix[0][1] - matrix[1][0]).abs() > SYMMETRY_TOLERANCE {
        return Err(FixtureValidationError::AsymmetricCovariance);
    }
    let determinant = matrix[0][0] * matrix[1][1] - matrix[0][1] * matrix[1][0];
    if !determinant.is_finite() {
        return Err(FixtureValidationError::NonFiniteIntermediate);
    }
    if matrix[0][0] <= 0.0 || matrix[1][1] <= 0.0 || determinant <= 0.0 {
        return Err(FixtureValidationError::CovarianceNotPositiveDefinite);
    }
    Ok(())
}

impl PublicObservationFixture {
    /// Fail-closed validation for small, two-parameter analytic controls.
    pub fn validate(&self) -> Result<(), FixtureValidationError> {
        if self.parameter_ids[0] == self.parameter_ids[1] {
            return Err(FixtureValidationError::DuplicateParameterId);
        }
        if self
            .observation_costs
            .iter()
            .any(|cost| !cost.is_finite() || *cost <= 0.0)
        {
            return Err(FixtureValidationError::InvalidObservationCost);
        }
        if self.observation_ids[0] == self.observation_ids[1] {
            return Err(FixtureValidationError::DuplicateObservationId);
        }
        if self
            .observation_operator
            .iter()
            .flatten()
            .any(|value| !value.is_finite())
        {
            return Err(FixtureValidationError::NonFiniteOperator);
        }
        if self.observed_values.iter().any(|value| !value.is_finite()) {
            return Err(FixtureValidationError::NonFiniteObservation);
        }
        if self
            .observation_covariance
            .iter()
            .flatten()
            .any(|value| !value.is_finite())
        {
            return Err(FixtureValidationError::NonFiniteCovariance);
        }
        let covariance = self.observation_covariance;
        validate_covariance(covariance)?;
        let prior_covariance = self.prior_covariance;
        validate_covariance(prior_covariance)?;
        if self
            .model_relative_seconds
            .iter()
            .any(|value| !value.is_finite())
            || self.model_relative_seconds[0] > self.model_relative_seconds[1]
        {
            return Err(FixtureValidationError::InvalidTimeAxis);
        }
        Ok(())
    }

    /// Analyze local observability after whitening by the full observation covariance.
    ///
    /// The 2x2 singular values are computed from the closed-form eigenvalues of
    /// WᵀW, where W = L⁻¹H and R = LLᵀ. This is an exact low-order H0 oracle,
    /// not a replacement for a reviewed numerical backend on large problems.
    pub fn analyze_observability(
        &self,
        absolute_tolerance: f64,
        relative_tolerance: f64,
    ) -> Result<ObservabilitySummary, FixtureValidationError> {
        self.validate()?;
        if !absolute_tolerance.is_finite()
            || !relative_tolerance.is_finite()
            || absolute_tolerance < 0.0
            || relative_tolerance < 0.0
            || (absolute_tolerance == 0.0 && relative_tolerance == 0.0)
        {
            return Err(FixtureValidationError::InvalidTolerance);
        }

        let covariance = self.observation_covariance;
        let l00 = covariance[0][0].sqrt();
        let l10 = covariance[1][0] / l00;
        let pivot = covariance[1][1] - l10 * l10;
        if !l00.is_finite() || l00 <= 0.0 || !l10.is_finite() || !pivot.is_finite() || pivot <= 0.0
        {
            return Err(FixtureValidationError::NonFiniteIntermediate);
        }
        let l11 = pivot.sqrt();

        // W = L⁻¹ H, so W has unit observation-error covariance.
        let h = self.observation_operator;
        let whitened = [
            [h[0][0] / l00, h[0][1] / l00],
            [
                (h[1][0] - l10 * h[0][0] / l00) / l11,
                (h[1][1] - l10 * h[0][1] / l00) / l11,
            ],
        ];
        if whitened.iter().flatten().any(|value| !value.is_finite()) {
            return Err(FixtureValidationError::NonFiniteIntermediate);
        }

        // Eigenvalues of WᵀW. Use det(W)^2 / lambda_max for the small
        // eigenvalue to reduce cancellation in rank-deficient fixtures.
        let alpha = whitened[0][0] * whitened[0][0] + whitened[1][0] * whitened[1][0];
        let beta = whitened[0][0] * whitened[0][1] + whitened[1][0] * whitened[1][1];
        let gamma = whitened[0][1] * whitened[0][1] + whitened[1][1] * whitened[1][1];
        let discriminant = (alpha - gamma).hypot(2.0 * beta);
        let lambda_max = 0.5 * (alpha + gamma + discriminant);
        let determinant = whitened[0][0] * whitened[1][1] - whitened[0][1] * whitened[1][0];
        let lambda_min = if lambda_max > 0.0 {
            (determinant * determinant / lambda_max).max(0.0)
        } else {
            0.0
        };
        if !alpha.is_finite()
            || !beta.is_finite()
            || !gamma.is_finite()
            || !lambda_max.is_finite()
            || !lambda_min.is_finite()
        {
            return Err(FixtureValidationError::NonFiniteIntermediate);
        }

        let singular_values = [lambda_max.sqrt(), lambda_min.sqrt()];
        let threshold = absolute_tolerance.max(relative_tolerance * singular_values[0]);
        let numerical_rank = singular_values
            .iter()
            .filter(|value| **value > threshold)
            .count();
        let condition_number = if numerical_rank == 2 && singular_values[1] > 0.0 {
            Some(singular_values[0] / singular_values[1])
        } else {
            None
        };

        let mut null_space_basis = Vec::new();
        if numerical_rank == 0 {
            null_space_basis.push([1.0, 0.0]);
            null_space_basis.push([0.0, 1.0]);
        } else if numerical_rank == 1 {
            let candidate_a = [beta, lambda_min - alpha];
            let candidate_b = [lambda_min - gamma, beta];
            let norm_a = candidate_a[0].hypot(candidate_a[1]);
            let norm_b = candidate_b[0].hypot(candidate_b[1]);
            let candidate = if norm_a >= norm_b {
                candidate_a
            } else {
                candidate_b
            };
            let norm = candidate[0].hypot(candidate[1]);
            let null_vector = if norm > 0.0 && norm.is_finite() {
                [candidate[0] / norm, candidate[1] / norm]
            } else if alpha <= gamma {
                [1.0, 0.0]
            } else {
                [0.0, 1.0]
            };
            null_space_basis.push(null_vector);
        }

        Ok(ObservabilitySummary {
            whitened_operator: whitened,
            singular_values,
            numerical_rank,
            condition_number,
            null_space_basis,
            absolute_tolerance,
            relative_tolerance,
        })
    }

    /// Exact Gaussian expected information gain in nats for the declared covariance prior.
    ///
    /// EIG = 0.5 * ln(det(I + Sa Hᵀ R⁻¹ H)), using the full correlated
    /// observation covariance R and declared state-prior covariance Sa.
    /// This is a 2x2 analytic reference utility, not a general design optimizer.
    pub fn expected_information_gain_nats(&self) -> Result<f64, FixtureValidationError> {
        self.validate()?;
        let covariance = self.observation_covariance;
        let determinant = covariance[0][0] * covariance[1][1] - covariance[0][1] * covariance[1][0];
        if !determinant.is_finite() || determinant <= 0.0 {
            return Err(FixtureValidationError::NonFiniteIntermediate);
        }
        let inverse_covariance = [
            [
                covariance[1][1] / determinant,
                -covariance[0][1] / determinant,
            ],
            [
                -covariance[1][0] / determinant,
                covariance[0][0] / determinant,
            ],
        ];
        let h = self.observation_operator;
        let weighted_h = [
            [
                inverse_covariance[0][0] * h[0][0] + inverse_covariance[0][1] * h[1][0],
                inverse_covariance[0][0] * h[0][1] + inverse_covariance[0][1] * h[1][1],
            ],
            [
                inverse_covariance[1][0] * h[0][0] + inverse_covariance[1][1] * h[1][0],
                inverse_covariance[1][0] * h[0][1] + inverse_covariance[1][1] * h[1][1],
            ],
        ];
        let data_information = [
            [
                h[0][0] * weighted_h[0][0] + h[1][0] * weighted_h[1][0],
                h[0][0] * weighted_h[0][1] + h[1][0] * weighted_h[1][1],
            ],
            [
                h[0][1] * weighted_h[0][0] + h[1][1] * weighted_h[1][0],
                h[0][1] * weighted_h[0][1] + h[1][1] * weighted_h[1][1],
            ],
        ];
        let prior = self.prior_covariance;
        let prior_data = [
            [
                prior[0][0] * data_information[0][0] + prior[0][1] * data_information[1][0],
                prior[0][0] * data_information[0][1] + prior[0][1] * data_information[1][1],
            ],
            [
                prior[1][0] * data_information[0][0] + prior[1][1] * data_information[1][0],
                prior[1][0] * data_information[0][1] + prior[1][1] * data_information[1][1],
            ],
        ];
        let precision = [
            [1.0 + prior_data[0][0], prior_data[0][1]],
            [prior_data[1][0], 1.0 + prior_data[1][1]],
        ];
        let precision_determinant =
            precision[0][0] * precision[1][1] - precision[0][1] * precision[1][0];
        if !precision_determinant.is_finite() || precision_determinant <= 0.0 {
            return Err(FixtureValidationError::NonFiniteIntermediate);
        }
        let gain = 0.5 * precision_determinant.ln();
        if gain.is_finite() && gain >= 0.0 {
            Ok(gain)
        } else {
            Err(FixtureValidationError::NonFiniteIntermediate)
        }
    }

    /// Expected information gain divided by total declared acquisition cost.
    pub fn expected_information_gain_per_cost(&self) -> Result<f64, FixtureValidationError> {
        self.validate()?;
        let total_cost: f64 = self.observation_costs.iter().sum();
        if !total_cost.is_finite() || total_cost <= 0.0 {
            return Err(FixtureValidationError::InvalidObservationCost);
        }
        let utility = self.expected_information_gain_nats()?;
        let value = utility / total_cost;
        if value.is_finite() {
            Ok(value)
        } else {
            Err(FixtureValidationError::NonFiniteIntermediate)
        }
    }

    /// Forward-project a candidate state through H. This method never reveals fixture truth.
    pub fn predict(&self, state: [f64; 2]) -> Result<[f64; 2], FixtureValidationError> {
        if state.iter().any(|value| !value.is_finite()) {
            return Err(FixtureValidationError::NonFiniteState);
        }
        Ok([
            self.observation_operator[0][0] * state[0] + self.observation_operator[0][1] * state[1],
            self.observation_operator[1][0] * state[0] + self.observation_operator[1][1] * state[1],
        ])
    }
}

const fn provenance() -> FixtureProvenance {
    FixtureProvenance {
        generator_id: "symtropy-geophysics-h0",
        generator_version: "0.1.0",
        truth_visibility: TruthVisibility::WithheldUntilScoring,
        evidence_class: EvidenceClass::SyntheticKnownTruth,
        randomness: "not_used_exact_analytic_fixture",
        units_note: "dimensionless proxy coordinates; not calibrated physical Earth units",
        claim_ceiling: "analytic observability control only; not a geodynamo model or real-Earth inference",
    }
}

/// Return the estimator-facing part of a frozen H0 fixture.
/// Hidden truth is not stored in this value or exposed by the default library API.
pub fn public_fixture(fixture_id: FixtureId) -> PublicObservationFixture {
    let provenance = provenance();
    match fixture_id {
        FixtureId::ComplementarySensors => PublicObservationFixture {
            fixture_id,
            parameter_ids: ["low_degree_field_proxy", "flow_mode_proxy"],
            observation_ids: ["sensor_low_degree", "sensor_flow_mode"],
            observation_costs: [1.0, 1.0],
            observation_operator: [[1.0, 0.0], [0.0, 1.0]],
            observation_covariance: [[0.25, 0.0], [0.0, 0.25]],
            prior_covariance: [[1.0, 0.0], [0.0, 1.0]],
            observed_values: [2.0, -1.0],
            model_relative_seconds: [0.0, 0.0],
            observation_units: "synthetic_dimensionless",
            provenance,
        },
        FixtureId::WeakHighMode => PublicObservationFixture {
            fixture_id,
            parameter_ids: ["low_degree_field_proxy", "high_degree_flow_proxy"],
            observation_ids: ["sensor_low_degree", "sensor_high_degree"],
            observation_costs: [1.0, 1.0],
            observation_operator: [[1.0, 0.0], [0.0, 0.1]],
            observation_covariance: [[0.25, 0.0], [0.0, 1.0]],
            prior_covariance: [[1.0, 0.0], [0.0, 1.0]],
            observed_values: [2.0, -0.3],
            model_relative_seconds: [0.0, 0.0],
            observation_units: "synthetic_dimensionless",
            provenance,
        },
        FixtureId::ExactNullMode => PublicObservationFixture {
            fixture_id,
            parameter_ids: ["low_degree_field_proxy", "unobservable_mode_proxy"],
            observation_ids: ["sensor_a", "sensor_b"],
            observation_costs: [1.0, 1.0],
            observation_operator: [[1.0, 0.0], [1.0, 0.0]],
            observation_covariance: [[1.0, 0.0], [0.0, 1.0]],
            prior_covariance: [[1.0, 0.0], [0.0, 1.0]],
            observed_values: [2.0, 2.0],
            model_relative_seconds: [0.0, 0.0],
            observation_units: "synthetic_dimensionless",
            provenance,
        },
        FixtureId::CorrelatedDuplicate => PublicObservationFixture {
            fixture_id,
            parameter_ids: ["low_degree_field_proxy", "unobservable_mode_proxy"],
            observation_ids: ["sensor_a", "sensor_a_repeat"],
            observation_costs: [1.0, 1.0],
            observation_operator: [[1.0, 0.0], [1.0, 0.0]],
            observation_covariance: [[0.25, 0.2475], [0.2475, 0.25]],
            prior_covariance: [[1.0, 0.0], [0.0, 1.0]],
            observed_values: [2.0, 2.0],
            model_relative_seconds: [0.0, 0.0],
            observation_units: "synthetic_dimensionless",
            provenance,
        },
    }
}

#[cfg(any(test, feature = "benchmark-scoring"))]
pub mod scoring;

#[cfg(test)]
mod tests {
    use super::*;

    const PUBLIC_MANIFEST: &str = include_str!("../fixtures/h0-public-manifest.json");

    fn close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() <= 1.0e-12,
            "actual {actual} differs from expected {expected}"
        );
    }

    #[test]
    fn frozen_public_manifest_matches_code_and_observability_references() {
        let manifest: serde_json::Value =
            serde_json::from_str(PUBLIC_MANIFEST).expect("public manifest is valid JSON");
        assert_eq!(manifest["benchmark_id"], "geo-sim-h0-known-truth-001");
        assert_eq!(manifest["status"], "benchmark_specification_not_executed");
        assert!(
            !manifest["claim_boundary"]["hidden_truth_is_in_public_manifest"]
                .as_bool()
                .unwrap()
        );
        let records = manifest["fixtures"].as_array().expect("fixtures array");
        assert_eq!(records.len(), FixtureId::ALL.len());

        for fixture_id in FixtureId::ALL {
            let fixture = public_fixture(fixture_id);
            let record = records
                .iter()
                .find(|record| record["id"] == fixture_id.as_str())
                .expect("fixture ID is present in manifest");
            assert_eq!(
                record["parameter_ids"],
                serde_json::to_value(fixture.parameter_ids).unwrap()
            );
            assert_eq!(
                record["observation_ids"],
                serde_json::to_value(fixture.observation_ids).unwrap()
            );
            assert_eq!(
                record["observation_costs"],
                serde_json::to_value(fixture.observation_costs).unwrap()
            );
            assert_eq!(
                record["observation_operator"],
                serde_json::to_value(fixture.observation_operator).unwrap()
            );
            assert_eq!(
                record["observation_covariance"],
                serde_json::to_value(fixture.observation_covariance).unwrap()
            );
            assert_eq!(
                record["prior_covariance"],
                serde_json::to_value(fixture.prior_covariance).unwrap()
            );
            assert_eq!(
                record["observed_values"],
                serde_json::to_value(fixture.observed_values).unwrap()
            );
            assert_eq!(
                record["model_relative_seconds"],
                serde_json::to_value(fixture.model_relative_seconds).unwrap()
            );

            close(
                fixture.expected_information_gain_nats().unwrap(),
                record["expected_information_gain_nats"].as_f64().unwrap(),
            );
            close(
                fixture.expected_information_gain_per_cost().unwrap(),
                record["expected_information_gain_per_cost"]
                    .as_f64()
                    .unwrap(),
            );
            let expected = &record["expected_observability"];
            let report = fixture.analyze_observability(1e-12, 1e-12).unwrap();
            assert_eq!(
                report.numerical_rank,
                expected["numerical_rank"].as_u64().unwrap() as usize,
                "{}",
                fixture_id.as_str()
            );
            let expected_singular_values = expected["singular_values"]
                .as_array()
                .expect("singular_values array");
            for (actual, value) in report.singular_values.iter().zip(expected_singular_values) {
                close(*actual, value.as_f64().unwrap());
            }
            match (
                report.condition_number,
                expected["condition_number"].as_f64(),
            ) {
                (Some(actual), Some(expected)) => close(actual, expected),
                (None, None) => {}
                (actual, expected) => {
                    panic!("condition number mismatch: {actual:?} vs {expected:?}");
                }
            }
            let expected_null = expected["null_space_basis"]
                .as_array()
                .expect("null space array");
            assert_eq!(report.null_space_basis.len(), expected_null.len());
            for (actual, vector) in report.null_space_basis.iter().zip(expected_null) {
                let values = vector.as_array().expect("null-space vector");
                close(actual[0], values[0].as_f64().unwrap());
                close(actual[1].abs(), values[1].as_f64().unwrap().abs());
            }
        }
    }

    #[test]
    fn all_public_fixtures_are_deterministic_and_valid() {
        for fixture_id in FixtureId::ALL {
            let first = public_fixture(fixture_id);
            let second = public_fixture(fixture_id);
            assert_eq!(first, second);
            assert_eq!(first.validate(), Ok(()));
            assert_eq!(
                first.provenance.truth_visibility,
                TruthVisibility::WithheldUntilScoring
            );
            assert_eq!(
                first.provenance.evidence_class,
                EvidenceClass::SyntheticKnownTruth
            );
            assert_eq!(
                first.provenance.randomness,
                "not_used_exact_analytic_fixture"
            );
        }
    }

    #[test]
    fn complementary_sensor_fixture_recovers_both_coordinates_exactly() {
        let fixture = public_fixture(FixtureId::ComplementarySensors);
        let truth = scoring::truth_states(FixtureId::ComplementarySensors)[0];
        let predicted = fixture.predict(truth).unwrap();
        close(predicted[0], fixture.observed_values[0]);
        close(predicted[1], fixture.observed_values[1]);
        assert_eq!(fixture.observation_operator, [[1.0, 0.0], [0.0, 1.0]]);
    }

    #[test]
    fn weak_high_mode_fixture_does_not_hide_its_low_sensitivity() {
        let fixture = public_fixture(FixtureId::WeakHighMode);
        assert_eq!(fixture.observation_operator[1][1], 0.1);
        assert_eq!(fixture.observation_covariance[1][1], 1.0);
        let truth = scoring::truth_states(FixtureId::WeakHighMode)[0];
        let predicted = fixture.predict(truth).unwrap();
        close(predicted[0], 2.0);
        close(predicted[1], -0.3);
    }

    #[test]
    fn exact_null_mode_allows_distinct_truths_with_identical_observations() {
        let fixture = public_fixture(FixtureId::ExactNullMode);
        let states = scoring::truth_states(FixtureId::ExactNullMode);
        assert_eq!(states.len(), 2);
        assert_ne!(states[0][1], states[1][1]);
        assert_eq!(fixture.predict(states[0]).unwrap(), fixture.observed_values);
        assert_eq!(fixture.predict(states[1]).unwrap(), fixture.observed_values);
    }

    #[test]
    fn correlated_duplicate_is_not_an_independent_complementary_sensor() {
        let fixture = public_fixture(FixtureId::CorrelatedDuplicate);
        assert!(fixture.observation_covariance[0][1] > 0.0);
        assert!(fixture.observation_covariance[0][1] < fixture.observation_covariance[0][0]);
        assert_ne!(
            fixture.observation_covariance,
            public_fixture(FixtureId::ExactNullMode).observation_covariance
        );
        let states = scoring::truth_states(FixtureId::CorrelatedDuplicate);
        assert_eq!(fixture.predict(states[0]).unwrap(), fixture.observed_values);
        assert_eq!(fixture.predict(states[1]).unwrap(), fixture.observed_values);
    }

    #[test]
    fn validation_rejects_asymmetric_or_non_positive_covariance() {
        let mut fixture = public_fixture(FixtureId::CorrelatedDuplicate);
        fixture.observation_covariance[1][0] = 0.1;
        assert_eq!(
            fixture.validate(),
            Err(FixtureValidationError::AsymmetricCovariance)
        );

        fixture.observation_covariance = [[1.0, 1.0], [1.0, 1.0]];
        assert_eq!(
            fixture.validate(),
            Err(FixtureValidationError::CovarianceNotPositiveDefinite)
        );
    }

    #[test]
    fn non_finite_states_and_operators_fail_closed() {
        let fixture = public_fixture(FixtureId::ComplementarySensors);
        assert_eq!(
            fixture.predict([f64::NAN, 0.0]),
            Err(FixtureValidationError::NonFiniteState)
        );
        let mut invalid = fixture;
        invalid.observation_operator[0][0] = f64::INFINITY;
        assert_eq!(
            invalid.validate(),
            Err(FixtureValidationError::NonFiniteOperator)
        );
    }
    #[test]
    fn complementary_sensors_are_full_rank_with_unit_condition_number() {
        let fixture = public_fixture(FixtureId::ComplementarySensors);
        let report = fixture.analyze_observability(1e-12, 1e-12).unwrap();
        assert_eq!(report.numerical_rank, 2);
        assert_eq!(report.null_space_basis.len(), 0);
        close(report.singular_values[0], 2.0);
        close(report.singular_values[1], 2.0);
        close(report.condition_number.unwrap(), 1.0);
    }

    #[test]
    fn weak_high_mode_reports_large_condition_number_not_fake_rank_loss() {
        let fixture = public_fixture(FixtureId::WeakHighMode);
        let report = fixture.analyze_observability(1e-12, 1e-12).unwrap();
        assert_eq!(report.numerical_rank, 2);
        close(report.condition_number.unwrap(), 20.0);
        assert!(report.singular_values[1] < report.singular_values[0] / 10.0);
    }

    #[test]
    fn exact_null_mode_and_correlated_duplicate_remain_rank_deficient_after_whitening() {
        for fixture_id in [FixtureId::ExactNullMode, FixtureId::CorrelatedDuplicate] {
            let report = public_fixture(fixture_id)
                .analyze_observability(1e-12, 1e-12)
                .unwrap();
            assert_eq!(report.numerical_rank, 1, "{}", fixture_id.as_str());
            assert_eq!(report.condition_number, None);
            assert_eq!(report.null_space_basis.len(), 1);
            assert!(report.null_space_basis[0][1].abs() > 0.999_999);
        }
    }

    #[test]
    fn shared_bias_cannot_be_counted_as_independent_sensor_information() {
        let mut fixture = public_fixture(FixtureId::CorrelatedDuplicate);
        fixture.observation_covariance = [[1.25, 1.0], [1.0, 1.25]];
        let shared_bias_gain = fixture.expected_information_gain_nats().unwrap();

        let mut dropped_correlation = fixture.clone();
        dropped_correlation.observation_covariance = [[1.25, 0.0], [0.0, 1.25]];
        let diagonalized_gain = dropped_correlation
            .expected_information_gain_nats()
            .unwrap();

        let mut omitted_bias = fixture.clone();
        omitted_bias.observation_covariance = [[0.25, 0.0], [0.0, 0.25]];
        let no_bias_gain = omitted_bias.expected_information_gain_nats().unwrap();

        close(shared_bias_gain, 0.31799438335999836);
        close(diagonalized_gain, 0.4777557225137182);
        close(no_bias_gain, 1.0986122886681098);
        assert!(shared_bias_gain < diagonalized_gain);
        assert!(diagonalized_gain < no_bias_gain);
    }

    #[test]
    fn correlated_duplicate_does_not_receive_independent_sensor_information_credit() {
        let fixture = public_fixture(FixtureId::CorrelatedDuplicate);
        let correlated_gain = fixture.expected_information_gain_nats().unwrap();
        let mut falsely_independent = fixture.clone();
        falsely_independent.observation_covariance = [[0.25, 0.0], [0.0, 0.25]];
        let naive_gain = falsely_independent
            .expected_information_gain_nats()
            .unwrap();
        close(correlated_gain, 0.8067249769620305);
        close(naive_gain, 1.0986122886681098);
        assert!(correlated_gain < naive_gain);
    }

    #[test]
    fn complementary_observations_beat_a_correlated_duplicate_at_equal_cost() {
        let complementary = public_fixture(FixtureId::ComplementarySensors);
        let duplicate = public_fixture(FixtureId::CorrelatedDuplicate);
        assert_eq!(
            complementary.observation_costs.iter().sum::<f64>(),
            duplicate.observation_costs.iter().sum::<f64>()
        );
        let complementary_eig = complementary.expected_information_gain_nats().unwrap();
        let duplicate_eig = duplicate.expected_information_gain_nats().unwrap();
        let complementary_per_cost = complementary.expected_information_gain_per_cost().unwrap();
        let duplicate_per_cost = duplicate.expected_information_gain_per_cost().unwrap();
        close(complementary_eig, 1.6094379124341003);
        close(duplicate_eig, 0.8067249769620305);
        close(complementary_per_cost, 0.8047189562170501);
        close(duplicate_per_cost, 0.40336248848101525);
        assert!(complementary_per_cost > duplicate_per_cost);
    }

    #[test]
    fn non_positive_or_non_finite_observation_cost_fails_closed() {
        let mut fixture = public_fixture(FixtureId.ComplementarySensors);
        fixture.observation_costs[0] = 0.0;
        assert_eq!(
            fixture.validate(),
            Err(FixtureValidationError::InvalidObservationCost)
        );
        fixture.observation_costs = [1.0, f64::INFINITY];
        assert_eq!(
            fixture.expected_information_gain_per_cost(),
            Err(FixtureValidationError::InvalidObservationCost)
        );
    }

    #[test]
    fn tightening_weak_mode_prior_reduces_expected_information_gain() {
        let mut fixture = public_fixture(FixtureId::WeakHighMode);
        let baseline = fixture.expected_information_gain_nats().unwrap();
        fixture.prior_covariance[1][1] = 0.01;
        let tight_prior = fixture.expected_information_gain_nats().unwrap();
        assert!(tight_prior < baseline - 0.004);
    }

    #[test]
    fn simultaneous_observation_channels_may_share_a_model_time() {
        for fixture_id in FixtureId::ALL {
            assert_eq!(
                public_fixture(fixture_id).model_relative_seconds,
                [0.0, 0.0]
            );
            assert_eq!(public_fixture(fixture_id).validate(), Ok(()));
        }
    }

    #[test]
    fn time_axis_rejects_out_of_order_but_allows_equal_timestamps() {
        let mut fixture = public_fixture(FixtureId::WeakHighMode);
        fixture.model_relative_seconds = [2.0, 1.0];
        assert_eq!(
            fixture.validate(),
            Err(FixtureValidationError::InvalidTimeAxis)
        );
        fixture.model_relative_seconds = [1.0, 1.0];
        assert_eq!(fixture.validate(), Ok(()));
    }

    #[test]
    fn observation_row_permutation_preserves_rank_singular_values_and_information_gain() {
        let original = public_fixture(FixtureId::WeakHighMode);
        let mut permuted = original.clone();
        permuted.observation_ids.swap(0, 1);
        permuted.observation_costs.swap(0, 1);
        permuted.observed_values.swap(0, 1);
        permuted.model_relative_seconds.swap(0, 1);
        permuted.observation_operator.swap(0, 1);
        permuted.observation_covariance.swap(0, 1);
        for row in &mut permuted.observation_covariance {
            row.swap(0, 1);
        }

        let a = original.analyze_observability(1e-12, 1e-12).unwrap();
        let b = permuted.analyze_observability(1e-12, 1e-12).unwrap();
        assert_eq!(a.numerical_rank, b.numerical_rank);
        close(a.singular_values[0], b.singular_values[0]);
        close(a.singular_values[1], b.singular_values[1]);
        close(
            original.expected_information_gain_nats().unwrap(),
            permuted.expected_information_gain_nats().unwrap(),
        );
    }

    #[test]
    fn observability_rejects_zero_negative_or_non_finite_tolerances() {
        let fixture = public_fixture(FixtureId::ComplementarySensors);
        for (absolute, relative) in [
            (0.0, 0.0),
            (-1.0, 1e-8),
            (1e-8, f64::NAN),
            (f64::INFINITY, 1e-8),
        ] {
            assert_eq!(
                fixture.analyze_observability(absolute, relative),
                Err(FixtureValidationError::InvalidTolerance)
            );
        }
    }

}
