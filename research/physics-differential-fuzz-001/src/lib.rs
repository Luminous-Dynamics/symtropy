#![forbid(unsafe_code)]

//! Deterministic, dependency-free seed for PHYS-DIFF-FUZZ-001.
//!
//! This crate generates physically admissible one-dimensional point-impact cases
//! and evaluates their analytic event oracle. It does not execute a backend,
//! define portable scenario identity, or decide whether two engines are equivalent.
//! Those responsibilities remain with #4366, #1172/#3060, #443 and #460.

use std::error::Error;
use std::fmt;

/// A one-dimensional point mass above a rigid plane at y = 0.
///
/// SI units are used: height [m], velocity [m/s], gravity [m/s^2].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointImpactCase {
    pub initial_height_m: f64,
    pub initial_velocity_m_s: f64,
    pub gravity_m_s2: f64,
    pub restitution: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PostImpactMode {
    /// A rebound branch for 0 < restitution <= 1.
    Rebound { normal_velocity_m_s: f64 },
    /// A constrained resting-contact branch for zero restitution.
    ///
    /// This is not a free-flight rebound with a zero coefficient.
    RestingContact,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ImpactPrediction {
    pub event_time_s: f64,
    pub pre_impact_velocity_m_s: f64,
    pub post_impact: PostImpactMode,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaseError {
    NonFiniteInput,
    NonPositiveHeight,
    NonPositiveGravity,
    RestitutionOutsideUnitInterval,
    InvalidUnitScale,
    CandidateCountTooLarge,
    NonFiniteOracleResult,
}

impl fmt::Display for CaseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteInput => formatter.write_str("case inputs must be finite"),
            Self::NonPositiveHeight => formatter.write_str("initial height must be positive"),
            Self::NonPositiveGravity => formatter.write_str("gravity magnitude must be positive"),
            Self::RestitutionOutsideUnitInterval => {
                formatter.write_str("restitution must lie in the closed interval [0, 1]")
            }
            Self::InvalidUnitScale => {
                formatter.write_str("length and time scales must be finite and positive")
            }
            Self::CandidateCountTooLarge => {
                formatter.write_str("candidate count exceeds the deterministic generator limit")
            }
            Self::NonFiniteOracleResult => {
                formatter.write_str("analytic impact prediction is non-finite or invalid")
            }
        }
    }
}

impl Error for CaseError {}

impl PointImpactCase {
    pub fn validate(self) -> Result<(), CaseError> {
        if !self.initial_height_m.is_finite()
            || !self.initial_velocity_m_s.is_finite()
            || !self.gravity_m_s2.is_finite()
            || !self.restitution.is_finite()
        {
            return Err(CaseError::NonFiniteInput);
        }
        if self.initial_height_m <= 0.0 {
            return Err(CaseError::NonPositiveHeight);
        }
        if self.gravity_m_s2 <= 0.0 {
            return Err(CaseError::NonPositiveGravity);
        }
        if !(0.0..=1.0).contains(&self.restitution) {
            return Err(CaseError::RestitutionOutsideUnitInterval);
        }
        Ok(())
    }

    /// Analytic first-impact event for
    /// y(t) = h + v0 t - (1/2) g t^2, with h > 0 and g > 0.
    ///
    /// The event time uses a branch-stable quadratic expression to reduce
    /// cancellation when the initial velocity is strongly downward.
    pub fn predict(self) -> Result<ImpactPrediction, CaseError> {
        self.validate()?;

        let height = self.initial_height_m;
        let velocity = self.initial_velocity_m_s;
        let gravity = self.gravity_m_s2;
        let discriminant = velocity.mul_add(velocity, 2.0 * gravity * height);
        if !discriminant.is_finite() || discriminant <= 0.0 {
            return Err(CaseError::NonFiniteOracleResult);
        }

        let impact_speed = discriminant.sqrt();
        let event_time = if velocity >= 0.0 {
            (velocity + impact_speed) / gravity
        } else {
            (2.0 * height) / (impact_speed - velocity)
        };
        let pre_impact_velocity = velocity - gravity * event_time;

        if !event_time.is_finite()
            || event_time <= 0.0
            || !pre_impact_velocity.is_finite()
            || pre_impact_velocity >= 0.0
        {
            return Err(CaseError::NonFiniteOracleResult);
        }

        let post_impact = if self.restitution == 0.0 {
            PostImpactMode::RestingContact
        } else {
            PostImpactMode::Rebound {
                normal_velocity_m_s: -self.restitution * pre_impact_velocity,
            }
        };

        Ok(ImpactPrediction {
            event_time_s: event_time,
            pre_impact_velocity_m_s: pre_impact_velocity,
            post_impact,
        })
    }

    /// Dimensionless coordinates for comparing unit-equivalent cases.
    ///
    /// eta = v0 / sqrt(g h), tau = t_hit sqrt(g / h).
    pub fn similarity(self) -> Result<ImpactSimilarity, CaseError> {
        let prediction = self.predict()?;
        let velocity_scale = (self.gravity_m_s2 * self.initial_height_m).sqrt();
        let similarity = ImpactSimilarity {
            dimensionless_initial_velocity: self.initial_velocity_m_s / velocity_scale,
            dimensionless_event_time: prediction.event_time_s
                * (self.gravity_m_s2 / self.initial_height_m).sqrt(),
            restitution: self.restitution,
        };
        if !similarity.dimensionless_initial_velocity.is_finite()
            || !similarity.dimensionless_event_time.is_finite()
        {
            return Err(CaseError::NonFiniteOracleResult);
        }
        Ok(similarity)
    }

    /// Apply a coherent length/time unit scaling without changing the physical case.
    ///
    /// If length is multiplied by L and time by T:
    /// height -> L h, velocity -> (L/T) v, gravity -> (L/T^2) g.
    pub fn rescale_units(self, length_scale: f64, time_scale: f64) -> Result<Self, CaseError> {
        self.validate()?;
        if !length_scale.is_finite()
            || !time_scale.is_finite()
            || length_scale <= 0.0
            || time_scale <= 0.0
        {
            return Err(CaseError::InvalidUnitScale);
        }

        let scaled = Self {
            initial_height_m: length_scale * self.initial_height_m,
            initial_velocity_m_s: (length_scale / time_scale) * self.initial_velocity_m_s,
            gravity_m_s2: (length_scale / (time_scale * time_scale)) * self.gravity_m_s2,
            restitution: self.restitution,
        };
        scaled.validate()?;
        if !scaled.initial_height_m.is_finite()
            || !scaled.initial_velocity_m_s.is_finite()
            || !scaled.gravity_m_s2.is_finite()
        {
            return Err(CaseError::NonFiniteOracleResult);
        }
        Ok(scaled)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ImpactSimilarity {
    pub dimensionless_initial_velocity: f64,
    pub dimensionless_event_time: f64,
    pub restitution: f64,
}

/// One generated candidate. Seed and ordinal identify the generator invocation,
/// not the portable physical-scenario identity owned by #3060.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GeneratedCandidate {
    pub seed: u64,
    pub ordinal: u64,
    pub case: PointImpactCase,
}

/// Small, explicit SplitMix64 implementation for deterministic candidate generation.
#[derive(Clone, Copy, Debug)]
struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        value ^ (value >> 31)
    }

    fn unit_interval(&mut self) -> f64 {
        const SCALE: f64 = 1.0 / ((1_u64 << 53) as f64);
        ((self.next_u64() >> 11) as f64) * SCALE
    }

    fn symmetric_unit(&mut self) -> f64 {
        2.0 * self.unit_interval() - 1.0
    }
}

/// Maximum number of candidates accepted by one generator invocation.
pub const MAX_CANDIDATES: usize = 100_000;

/// Generate bounded, physically valid point-impact candidates in a deterministic order.
///
/// A hard count cap bounds allocation and execution cost. Height and gravity are
/// sampled log-uniformly over declared ranges. Initial
/// velocity is sampled relative to sqrt(g h), avoiding a scale-dependent velocity
/// distribution. Every sixteenth candidate exercises e=0 and e=1 explicitly.
pub fn generate_candidates(seed: u64, count: usize) -> Result<Vec<GeneratedCandidate>, CaseError> {
    if count > MAX_CANDIDATES {
        return Err(CaseError::CandidateCountTooLarge);
    }
    let mut rng = SplitMix64::new(seed);
    let mut candidates = Vec::with_capacity(count);

    for ordinal in 0..count {
        let height_exponent = -2.0 + 5.0 * rng.unit_interval();
        let gravity_exponent = -2.0 + 3.0 * rng.unit_interval();
        let height = 10.0_f64.powf(height_exponent);
        let gravity = 10.0_f64.powf(gravity_exponent);
        let velocity_scale = (height * gravity).sqrt();
        let initial_velocity = 3.0 * rng.symmetric_unit() * velocity_scale;
        let restitution = match ordinal % 16 {
            0 => 0.0,
            1 => 1.0,
            _ => rng.unit_interval(),
        };

        let case = PointImpactCase {
            initial_height_m: height,
            initial_velocity_m_s: initial_velocity,
            gravity_m_s2: gravity,
            restitution,
        };
        case.validate()?;
        case.predict()?;
        candidates.push(GeneratedCandidate {
            seed,
            ordinal: ordinal as u64,
            case,
        });
    }

    Ok(candidates)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(actual: f64, expected: f64, tolerance: f64) {
        let scale = 1.0_f64.max(actual.abs()).max(expected.abs());
        assert!(
            (actual - expected).abs() <= tolerance * scale,
            "actual={actual:.17e}, expected={expected:.17e}, tolerance={tolerance:.3e}"
        );
    }

    #[test]
    fn first_impact_matches_the_closed_form_for_a_released_mass() {
        let case = PointImpactCase {
            initial_height_m: 2.0,
            initial_velocity_m_s: 0.0,
            gravity_m_s2: 9.81,
            restitution: 0.5,
        };
        let result = case.predict().expect("valid impact case");
        assert_close(result.event_time_s, (4.0_f64 / 9.81).sqrt(), 1.0e-14);
        assert_close(
            result.pre_impact_velocity_m_s,
            -(2.0_f64 * 9.81 * 2.0).sqrt(),
            1.0e-14,
        );
        assert_eq!(
            result.post_impact,
            PostImpactMode::Rebound {
                normal_velocity_m_s: -result.pre_impact_velocity_m_s * 0.5,
            }
        );
    }

    #[test]
    fn zero_restitution_is_a_resting_contact_mode_not_a_rebound() {
        let case = PointImpactCase {
            initial_height_m: 1.0,
            initial_velocity_m_s: -0.25,
            gravity_m_s2: 9.81,
            restitution: 0.0,
        };
        assert_eq!(
            case.predict().expect("valid inelastic impact").post_impact,
            PostImpactMode::RestingContact
        );
    }

    #[test]
    fn unit_rescaling_preserves_dimensionless_impact_and_scales_time() {
        let case = PointImpactCase {
            initial_height_m: 1.7,
            initial_velocity_m_s: -0.8,
            gravity_m_s2: 9.81,
            restitution: 0.73,
        };
        let scale_length = 3.25;
        let scale_time = 0.4;
        let original = case.predict().expect("original impact");
        let original_similarity = case.similarity().expect("original similarity");
        let scaled = case
            .rescale_units(scale_length, scale_time)
            .expect("valid coherent unit scaling");
        let scaled_prediction = scaled.predict().expect("scaled impact");
        let scaled_similarity = scaled.similarity().expect("scaled similarity");

        assert_close(
            scaled_prediction.event_time_s,
            scale_time * original.event_time_s,
            1.0e-13,
        );
        assert_close(
            scaled_prediction.pre_impact_velocity_m_s,
            (scale_length / scale_time) * original.pre_impact_velocity_m_s,
            1.0e-13,
        );
        assert_close(
            scaled_similarity.dimensionless_initial_velocity,
            original_similarity.dimensionless_initial_velocity,
            1.0e-13,
        );
        assert_close(
            scaled_similarity.dimensionless_event_time,
            original_similarity.dimensionless_event_time,
            1.0e-13,
        );
    }

    #[test]
    fn generator_is_deterministic_and_includes_restitution_extremes() {
        let first = generate_candidates(0xD1FF_EE71, 128).expect("valid generated corpus");
        let second = generate_candidates(0xD1FF_EE71, 128).expect("same seed");
        assert_eq!(first, second);
        assert_eq!(first[0].case.restitution, 0.0);
        assert_eq!(first[1].case.restitution, 1.0);
        assert!(
            first
                .iter()
                .all(|candidate| candidate.case.validate().is_ok())
        );
    }

    #[test]
    fn generated_oracles_satisfy_the_impact_guard_and_restitution_law() {
        for candidate in generate_candidates(0xC0FF_EE12, 2_048).expect("valid corpus") {
            let case = candidate.case;
            let prediction = case.predict().expect("valid prediction");
            let t = prediction.event_time_s;
            let guard = case.initial_height_m + case.initial_velocity_m_s * t
                - 0.5 * case.gravity_m_s2 * t * t;
            assert_close(guard, 0.0, 5.0e-12);

            let expected_incoming = -(case.initial_velocity_m_s.mul_add(
                case.initial_velocity_m_s,
                2.0 * case.gravity_m_s2 * case.initial_height_m,
            ))
            .sqrt();
            assert_close(
                prediction.pre_impact_velocity_m_s,
                expected_incoming,
                5.0e-12,
            );

            match prediction.post_impact {
                PostImpactMode::RestingContact => assert_eq!(case.restitution, 0.0),
                PostImpactMode::Rebound {
                    normal_velocity_m_s,
                } => {
                    assert!(case.restitution > 0.0);
                    assert_close(
                        normal_velocity_m_s,
                        -case.restitution * prediction.pre_impact_velocity_m_s,
                        1.0e-14,
                    );
                }
            }
        }
    }

    #[test]
    fn generator_enforces_its_declared_count_and_parameter_bounds() {
        assert_eq!(
            generate_candidates(7, MAX_CANDIDATES + 1),
            Err(CaseError::CandidateCountTooLarge)
        );
        for candidate in generate_candidates(9, 4_096).expect("bounded corpus") {
            let case = candidate.case;
            assert!((1.0e-2..=1.0e3).contains(&case.initial_height_m));
            assert!((1.0e-2..=1.0e1).contains(&case.gravity_m_s2));
            assert!((-3.0..=3.0).contains(
                &(case.initial_velocity_m_s / (case.initial_height_m * case.gravity_m_s2).sqrt())
            ));
            assert!((0.0..=1.0).contains(&case.restitution));
        }
    }

    #[test]
    fn invalid_cases_and_invalid_unit_scales_fail_closed() {
        let mut invalid = PointImpactCase {
            initial_height_m: 1.0,
            initial_velocity_m_s: 0.0,
            gravity_m_s2: 9.81,
            restitution: 0.5,
        };
        invalid.initial_height_m = f64::NAN;
        assert_eq!(invalid.validate(), Err(CaseError::NonFiniteInput));

        let valid = PointImpactCase {
            initial_height_m: 1.0,
            initial_velocity_m_s: 0.0,
            gravity_m_s2: 9.81,
            restitution: 0.5,
        };
        assert_eq!(
            valid.rescale_units(1.0, 0.0),
            Err(CaseError::InvalidUnitScale)
        );
    }
}
