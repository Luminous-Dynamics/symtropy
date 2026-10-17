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
    /// Observation matrix H, with observations in rows and state coordinates in columns.
    pub observation_operator: [[f64; 2]; 2],
    /// Full measurement-error covariance. Off-diagonal terms are intentional.
    pub observation_covariance: [[f64; 2]; 2],
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
}

impl PublicObservationFixture {
    /// Fail-closed validation for small, two-parameter analytic controls.
    pub fn validate(&self) -> Result<(), FixtureValidationError> {
        if self.parameter_ids[0] == self.parameter_ids[1] {
            return Err(FixtureValidationError::DuplicateParameterId);
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
        if (covariance[0][1] - covariance[1][0]).abs() > SYMMETRY_TOLERANCE {
            return Err(FixtureValidationError::AsymmetricCovariance);
        }
        let determinant =
            covariance[0][0] * covariance[1][1] - covariance[0][1] * covariance[1][0];
        if covariance[0][0] <= 0.0 || covariance[1][1] <= 0.0 || determinant <= 0.0 {
            return Err(FixtureValidationError::CovarianceNotPositiveDefinite);
        }
        if self
            .model_relative_seconds
            .iter()
            .any(|value| !value.is_finite())
            || self.model_relative_seconds[0] >= self.model_relative_seconds[1]
        {
            return Err(FixtureValidationError::InvalidTimeAxis);
        }
        Ok(())
    }

    /// Forward-project a candidate state through H. This method never reveals fixture truth.
    pub fn predict(&self, state: [f64; 2]) -> Result<[f64; 2], FixtureValidationError> {
        if state.iter().any(|value| !value.is_finite()) {
            return Err(FixtureValidationError::NonFiniteState);
        }
        Ok([
            self.observation_operator[0][0] * state[0]
                + self.observation_operator[0][1] * state[1],
            self.observation_operator[1][0] * state[0]
                + self.observation_operator[1][1] * state[1],
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
            observation_operator: [[1.0, 0.0], [0.0, 1.0]],
            observation_covariance: [[0.25, 0.0], [0.0, 0.25]],
            observed_values: [2.0, -1.0],
            model_relative_seconds: [0.0, 1.0],
            observation_units: "synthetic_dimensionless",
            provenance,
        },
        FixtureId::WeakHighMode => PublicObservationFixture {
            fixture_id,
            parameter_ids: ["low_degree_field_proxy", "high_degree_flow_proxy"],
            observation_ids: ["sensor_low_degree", "sensor_high_degree"],
            observation_operator: [[1.0, 0.0], [0.0, 0.1]],
            observation_covariance: [[0.25, 0.0], [0.0, 1.0]],
            observed_values: [2.0, -0.3],
            model_relative_seconds: [0.0, 1.0],
            observation_units: "synthetic_dimensionless",
            provenance,
        },
        FixtureId::ExactNullMode => PublicObservationFixture {
            fixture_id,
            parameter_ids: ["low_degree_field_proxy", "unobservable_mode_proxy"],
            observation_ids: ["sensor_a", "sensor_b"],
            observation_operator: [[1.0, 0.0], [1.0, 0.0]],
            observation_covariance: [[1.0, 0.0], [0.0, 1.0]],
            observed_values: [2.0, 2.0],
            model_relative_seconds: [0.0, 1.0],
            observation_units: "synthetic_dimensionless",
            provenance,
        },
        FixtureId::CorrelatedDuplicate => PublicObservationFixture {
            fixture_id,
            parameter_ids: ["low_degree_field_proxy", "unobservable_mode_proxy"],
            observation_ids: ["sensor_a", "sensor_a_repeat"],
            observation_operator: [[1.0, 0.0], [1.0, 0.0]],
            observation_covariance: [[0.25, 0.2475], [0.2475, 0.25]],
            observed_values: [2.0, 2.0],
            model_relative_seconds: [0.0, 1.0],
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

    fn close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() <= 1.0e-12,
            "actual {actual} differs from expected {expected}"
        );
    }

    #[test]
    fn all_public_fixtures_are_deterministic_and_valid() {
        for fixture_id in FixtureId::ALL {
            let first = public_fixture(fixture_id);
            let second = public_fixture(fixture_id);
            assert_eq!(first, second);
            assert_eq!(first.validate(), Ok(()));
            assert_eq!(first.provenance.truth_visibility, TruthVisibility::WithheldUntilScoring);
            assert_eq!(first.provenance.evidence_class, EvidenceClass::SyntheticKnownTruth);
            assert_eq!(first.provenance.randomness, "not_used_exact_analytic_fixture");
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
        assert_eq!(fixture.validate(), Err(FixtureValidationError::AsymmetricCovariance));

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
        assert_eq!(invalid.validate(), Err(FixtureValidationError::NonFiniteOperator));
    }
}
