// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! HarmonyField: a phenomenological spatial influence model for simulations.
//!
//! The model uses explicit software rules to translate nonnegative activation
//! profiles into spatially varying influences. It is inspired by ideas about
//! field-mediated coupling, but it does not simulate electromagnetic fields,
//! establish the CEMI theory, or imply subjective experience. It is a tunable
//! simulation mechanism, not a validated physical law.
//!
//! # Model Contract
//! - Each source emits a radial influence profile over nine activation channels.
//! - The Plummer-softened kernel uses p_D = max(D - 1, 1). The D=1 attenuation
//!   floor is a modeling choice, not a universal dimension law.
//! - Sources superpose. Nonnegative profiles can express similarity and
//!   orthogonality; they do not represent signed anti-alignment or interference.
//! - The coupled friction effect reduces friction for aligned profiles and
//!   leaves orthogonal or absent profiles at baseline.
//! - Emotional contagion is a separate, proximity-weighted agent update; it is
//!   not an electromagnetic-field simulation.

use nalgebra::SVector;
use symtropy_math::Point;

/// Number of harmonies in the Nine Harmonies system (Eight + Emotional Contagion).
pub const NUM_HARMONIES: usize = 9;

/// Index of the Emotional Contagion harmony (ninth dimension, zero-indexed).
///
/// Emotional contagion is the social spread of affective states via proximity.
/// Dunbar (2012): synchronized emotional states bind social groups.
/// McFadden (2020): EM field coupling creates temporal coherence across minds.
/// High activation → strong social emotional influence on and from neighbors.
pub const EMOTIONAL_CONTAGION_IDX: usize = 8;

/// Rate of emotional contagion transfer: fraction per second toward social mean.
pub const EMOTIONAL_CONTAGION_RATE: f64 = 0.15;

/// Decay rate of emotional activation per second when no social contact.
pub const EMOTIONAL_CONTAGION_DECAY: f64 = 0.05;

/// Maximum radius for emotional contagion influence (world units).
pub const EMOTIONAL_CONTAGION_RADIUS: f64 = 20.0;

/// Typed validation failures for harmony-field sources and numerical queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HarmonyFieldInputError {
    NonFiniteQueryCoordinate { axis: usize },
    NonFiniteSourcePosition { source_index: usize, axis: usize },
    NonFiniteActivation { source_index: usize, index: usize },
    ActivationOutOfRange { source_index: usize, index: usize },
    NonFiniteStrength { source_index: usize },
    NegativeStrength { source_index: usize },
    NonFiniteRadius { source_index: usize },
    NonPositiveRadius { source_index: usize },
    NonFiniteCreatedAt { source_index: usize },
    NegativeCreatedAt { source_index: usize },
    NonFinitePropagationSpeed { source_index: usize },
    NonPositivePropagationSpeed { source_index: usize },
    NonFiniteEntityActivation { index: usize },
    EntityActivationOutOfRange { index: usize },
    NonFiniteResonanceInput { vector: u8, index: usize },
    NegativeResonanceInput { vector: u8, index: usize },
    NonFiniteCurrentTime,
    NegativeCurrentTime,
    NonFiniteDistance { source_index: usize },
    NonFiniteKernel { source_index: usize },
    NonFinitePropagationDelay { source_index: usize },
    NonFiniteFieldValue { harmony_index: usize },
    NonFiniteFieldEnergy,
    NonFiniteResonance,
}

impl std::fmt::Display for HarmonyFieldInputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid harmony-field input or result: {self:?}")
    }
}

impl std::error::Error for HarmonyFieldInputError {}

fn validate_point<const D: usize>(point: &Point<D>) -> Result<(), HarmonyFieldInputError> {
    for axis in 0..D {
        if !point.coord(axis).is_finite() {
            return Err(HarmonyFieldInputError::NonFiniteQueryCoordinate { axis });
        }
    }
    Ok(())
}

fn validate_source<const D: usize>(
    source: &HarmonySource<D>,
    source_index: usize,
) -> Result<(), HarmonyFieldInputError> {
    for axis in 0..D {
        if !source.position.coord(axis).is_finite() {
            return Err(HarmonyFieldInputError::NonFiniteSourcePosition { source_index, axis });
        }
    }
    for (index, activation) in source.activations.iter().enumerate() {
        if !activation.is_finite() {
            return Err(HarmonyFieldInputError::NonFiniteActivation {
                source_index,
                index,
            });
        }
        if !(0.0..=1.0).contains(activation) {
            return Err(HarmonyFieldInputError::ActivationOutOfRange {
                source_index,
                index,
            });
        }
    }
    if !source.strength.is_finite() {
        return Err(HarmonyFieldInputError::NonFiniteStrength { source_index });
    }
    if source.strength < 0.0 {
        return Err(HarmonyFieldInputError::NegativeStrength { source_index });
    }
    if !source.radius.is_finite() {
        return Err(HarmonyFieldInputError::NonFiniteRadius { source_index });
    }
    if source.radius <= 0.0 {
        return Err(HarmonyFieldInputError::NonPositiveRadius { source_index });
    }
    if !source.created_at.is_finite() {
        return Err(HarmonyFieldInputError::NonFiniteCreatedAt { source_index });
    }
    if source.created_at < 0.0 {
        return Err(HarmonyFieldInputError::NegativeCreatedAt { source_index });
    }
    if !source.propagation_speed.is_finite() {
        return Err(HarmonyFieldInputError::NonFinitePropagationSpeed { source_index });
    }
    // f64::MAX is the documented sentinel for instant propagation.
    if source.propagation_speed <= 0.0 {
        return Err(HarmonyFieldInputError::NonPositivePropagationSpeed { source_index });
    }
    Ok(())
}

fn validate_entity_harmonies(
    harmonies: &[f64; NUM_HARMONIES],
) -> Result<(), HarmonyFieldInputError> {
    for (index, value) in harmonies.iter().enumerate() {
        if !value.is_finite() {
            return Err(HarmonyFieldInputError::NonFiniteEntityActivation { index });
        }
        if !(0.0..=1.0).contains(value) {
            return Err(HarmonyFieldInputError::EntityActivationOutOfRange { index });
        }
    }
    Ok(())
}

/// A harmony field source: an entity emitting harmony activations.
#[derive(Debug, Clone)]
pub struct HarmonySource<const D: usize> {
    /// Position of the source entity.
    pub position: Point<D>,
    /// Harmony activations [0.0, 1.0] for each of the 9 harmonies.
    /// Index 8 = Emotional Contagion.
    pub activations: [f64; NUM_HARMONIES],
    /// Finite, nonnegative field-strength multiplier (scales with consciousness level).
    pub strength: f64,
    /// Finite, positive influence radius; zero/negative radii are invalid.
    pub radius: f64,
    /// Finite, nonnegative simulation time when this source was created/activated.
    /// Used for finite propagation delay. Default 0.0 (instant).
    pub created_at: f64,
    /// Finite, positive field propagation speed in units/sec.
    /// f64::MAX is the documented instant-propagation sentinel for moving agents.
    /// Use finite values below f64::MAX for stationary sources (wells, sanctuaries).
    pub propagation_speed: f64,
}

impl<const D: usize> HarmonySource<D> {
    /// Create a source while enforcing the documented numerical contract.
    ///
    /// Public fields remain available for source compatibility. Call `validate()`
    /// after direct mutation; every checked field computation also revalidates sources.
    pub fn try_new(
        position: Point<D>,
        activations: [f64; NUM_HARMONIES],
        strength: f64,
        radius: f64,
        created_at: f64,
        propagation_speed: f64,
    ) -> Result<Self, HarmonyFieldInputError> {
        let source = Self {
            position,
            activations,
            strength,
            radius,
            created_at,
            propagation_speed,
        };
        validate_source(&source, 0)?;
        Ok(source)
    }

    /// Validate all source fields using the documented contract.
    ///
    /// The returned source index is zero because this validates one standalone source.
    pub fn validate(&self) -> Result<(), HarmonyFieldInputError> {
        validate_source(self, 0)
    }
}

/// The harmony field: aggregates all sources and computes field effects.
pub struct HarmonyField<const D: usize> {
    pub sources: Vec<HarmonySource<D>>,
}

impl<const D: usize> HarmonyField<D> {
    pub fn new() -> Self {
        Self {
            sources: Vec::new(),
        }
    }

    /// Construct a field only from validated sources.
    pub fn try_from_sources(
        sources: Vec<HarmonySource<D>>,
    ) -> Result<Self, HarmonyFieldInputError> {
        let field = Self { sources };
        field.validate()?;
        Ok(field)
    }

    /// Validate every source, including sources inserted through the public vector.
    pub fn validate(&self) -> Result<(), HarmonyFieldInputError> {
        for (index, source) in self.sources.iter().enumerate() {
            validate_source(source, index)?;
        }
        Ok(())
    }

    /// Add a source only after validating it and the field's existing sources.
    pub fn try_add_source(
        &mut self,
        source: HarmonySource<D>,
    ) -> Result<(), HarmonyFieldInputError> {
        self.validate()?;
        validate_source(&source, self.sources.len())?;
        self.sources.push(source);
        Ok(())
    }

    /// Checked field sampling using p_D = max(D - 1, 1) and Plummer softening.
    ///
    /// Invalid public-field mutations and non-finite intermediate results are
    /// errors, not numeric-looking field values. The source activations are
    /// nonnegative and bounded by [0, 1]; field samples may exceed 1 after
    /// superposition across sources.
    pub fn try_sample(
        &self,
        point: &Point<D>,
    ) -> Result<[f64; NUM_HARMONIES], HarmonyFieldInputError> {
        self.try_sample_internal(point, None)
    }

    /// Checked field sampling with finite propagation delay.
    pub fn try_sample_at_time(
        &self,
        point: &Point<D>,
        current_time: f64,
    ) -> Result<[f64; NUM_HARMONIES], HarmonyFieldInputError> {
        self.try_sample_internal(point, Some(current_time))
    }

    fn try_sample_internal(
        &self,
        point: &Point<D>,
        current_time: Option<f64>,
    ) -> Result<[f64; NUM_HARMONIES], HarmonyFieldInputError> {
        validate_point(point)?;
        if let Some(time) = current_time {
            if !time.is_finite() {
                return Err(HarmonyFieldInputError::NonFiniteCurrentTime);
            }
            if time < 0.0 {
                return Err(HarmonyFieldInputError::NegativeCurrentTime);
            }
        }

        let mut total = [0.0f64; NUM_HARMONIES];
        let exponent = (D as f64 - 1.0).max(1.0);
        for (source_index, source) in self.sources.iter().enumerate() {
            // Validate before radius/time filters so invalid sources are never
            // silently ignored merely because they are currently out of range.
            validate_source(source, source_index)?;
            let dist = source.position.distance(point);
            if !dist.is_finite() {
                return Err(HarmonyFieldInputError::NonFiniteDistance { source_index });
            }
            if dist >= source.radius {
                continue;
            }

            if let Some(time) = current_time {
                // f64::MAX is the instant-propagation sentinel.
                if source.propagation_speed < f64::MAX {
                    let travel_time = dist / source.propagation_speed;
                    let arrival_time = source.created_at + travel_time;
                    if !travel_time.is_finite() || !arrival_time.is_finite() {
                        return Err(HarmonyFieldInputError::NonFinitePropagationDelay {
                            source_index,
                        });
                    }
                    if time < arrival_time {
                        continue;
                    }
                }
            }

            // hypot avoids overflow in dist². If the power itself overflows,
            // the logarithmic fallback preserves cases where source strength
            // is so large that the final ratio is still representable.
            let softened_distance = dist.hypot(Self::SOFTENING_EPSILON);
            let denominator = softened_distance.powf(exponent);
            let falloff = if denominator.is_finite() {
                source.strength / denominator
            } else if source.strength == 0.0 {
                0.0
            } else {
                (source.strength.ln() - exponent * softened_distance.ln()).exp()
            };
            if !falloff.is_finite() {
                return Err(HarmonyFieldInputError::NonFiniteKernel { source_index });
            }

            for (index, total_value) in total.iter_mut().enumerate() {
                *total_value += source.activations[index] * falloff;
                if !total_value.is_finite() {
                    return Err(HarmonyFieldInputError::NonFiniteFieldValue {
                        harmony_index: index,
                    });
                }
            }
        }
        Ok(total)
    }

    /// Compatibility API. Prefer `try_sample` so invalid data remains explicit.
    /// Invalid input produces a NaN sentinel instead of a plausible zero field.
    pub fn sample(&self, point: &Point<D>) -> [f64; NUM_HARMONIES] {
        self.try_sample(point).unwrap_or([f64::NAN; NUM_HARMONIES])
    }

    /// Checked cosine resonance for finite, nonnegative vectors.
    ///
    /// Zero-norm vectors have defined neutral resonance 0.0. Scaling each
    /// vector before norm/dot calculations avoids overflow for extreme finite
    /// magnitudes. Negative components are rejected: anti-alignment is outside
    /// the documented nonnegative activation contract.
    pub fn try_resonance(
        a: &[f64; NUM_HARMONIES],
        b: &[f64; NUM_HARMONIES],
    ) -> Result<f64, HarmonyFieldInputError> {
        for (vector, values) in [(0_u8, a), (1_u8, b)] {
            for (index, value) in values.iter().enumerate() {
                if !value.is_finite() {
                    return Err(HarmonyFieldInputError::NonFiniteResonanceInput { vector, index });
                }
                if *value < 0.0 {
                    return Err(HarmonyFieldInputError::NegativeResonanceInput { vector, index });
                }
            }
        }

        let scale_a = a.iter().copied().fold(0.0_f64, f64::max);
        let scale_b = b.iter().copied().fold(0.0_f64, f64::max);
        if scale_a == 0.0 || scale_b == 0.0 {
            return Ok(0.0);
        }

        let normalized_a: [f64; NUM_HARMONIES] = std::array::from_fn(|i| a[i] / scale_a);
        let normalized_b: [f64; NUM_HARMONIES] = std::array::from_fn(|i| b[i] / scale_b);
        let dot: f64 = normalized_a
            .iter()
            .zip(normalized_b.iter())
            .map(|(x, y)| x * y)
            .sum();
        let norm_a = normalized_a.iter().map(|x| x * x).sum::<f64>().sqrt();
        let norm_b = normalized_b.iter().map(|x| x * x).sum::<f64>().sqrt();
        let resonance = dot / (norm_a * norm_b);
        if !resonance.is_finite() {
            return Err(HarmonyFieldInputError::NonFiniteResonance);
        }
        Ok(resonance.clamp(0.0, 1.0))
    }

    /// Compatibility API. Prefer `try_resonance` to receive typed errors.
    pub fn resonance(a: &[f64; NUM_HARMONIES], b: &[f64; NUM_HARMONIES]) -> f64 {
        Self::try_resonance(a, b).unwrap_or(f64::NAN)
    }

    /// Checked friction modulation at a point.
    ///
    /// Under the documented nonnegative activation contract, returns [0.5, 1.0]:
    /// aligned vectors approach 0.5, while orthogonal or absent fields return 1.0.
    pub fn try_friction_multiplier(
        &self,
        point: &Point<D>,
        entity_harmonies: &[f64; NUM_HARMONIES],
    ) -> Result<f64, HarmonyFieldInputError> {
        validate_entity_harmonies(entity_harmonies)?;
        let field = self.try_sample(point)?;
        let resonance = Self::try_resonance(&field, entity_harmonies)?;
        Ok(1.0 - resonance * 0.5)
    }

    /// Checked collision impulse modulation at a point.
    ///
    /// Under the documented nonnegative activation contract, returns [0.5, 1.0]:
    /// aligned vectors approach 0.5, while orthogonal or absent fields return 1.0.
    pub fn try_impulse_multiplier(
        &self,
        point: &Point<D>,
        entity_harmonies: &[f64; NUM_HARMONIES],
    ) -> Result<f64, HarmonyFieldInputError> {
        validate_entity_harmonies(entity_harmonies)?;
        let field = self.try_sample(point)?;
        let resonance = Self::try_resonance(&field, entity_harmonies)?;
        Ok(1.0 - resonance * 0.5)
    }

    /// Compatibility API. Invalid input produces a NaN sentinel; use the
    /// checked method in code that requires an explicit error boundary.
    pub fn friction_multiplier(
        &self,
        point: &Point<D>,
        entity_harmonies: &[f64; NUM_HARMONIES],
    ) -> f64 {
        self.try_friction_multiplier(point, entity_harmonies)
            .unwrap_or(f64::NAN)
    }

    /// Compatibility API. Invalid input produces a NaN sentinel; use the
    /// checked method in code that requires an explicit error boundary.
    pub fn impulse_multiplier(
        &self,
        point: &Point<D>,
        entity_harmonies: &[f64; NUM_HARMONIES],
    ) -> f64 {
        self.try_impulse_multiplier(point, entity_harmonies)
            .unwrap_or(f64::NAN)
    }

    /// Checked total field energy. Overflow cannot masquerade as a valid scalar.
    pub fn try_field_energy(&self, point: &Point<D>) -> Result<f64, HarmonyFieldInputError> {
        let field = self.try_sample(point)?;
        let energy = field.iter().sum::<f64>();
        if !energy.is_finite() {
            return Err(HarmonyFieldInputError::NonFiniteFieldEnergy);
        }
        Ok(energy)
    }

    /// Compatibility API. Prefer `try_sample_at_time` so invalid data remains explicit.
    /// Invalid input produces a NaN sentinel instead of a plausible zero field.
    pub fn sample_at_time(&self, point: &Point<D>, current_time: f64) -> [f64; NUM_HARMONIES] {
        self.try_sample_at_time(point, current_time)
            .unwrap_or([f64::NAN; NUM_HARMONIES])
    }

    /// Compatibility API. Prefer `try_field_energy` to receive typed errors.
    pub fn field_energy(&self, point: &Point<D>) -> f64 {
        self.try_field_energy(point).unwrap_or(f64::NAN)
    }

    /// Softening parameter ε for Plummer softening: (r² + ε²)^(n/2).
    /// Prevents singularity at r→0. Standard N-body astrophysics technique.
    const SOFTENING_EPSILON: f64 = 1.0;

    /// Gradient of conformal parameter σ = scale × field_energy (for curvature).
    /// Computed via central finite differences.
    #[cfg(feature = "consciousness-curvature")]
    pub fn sigma_gradient(
        &self,
        point: &Point<D>,
        curvature_scale: f64,
        epsilon: f64,
    ) -> nalgebra::SVector<f64, D> {
        let mut grad = nalgebra::SVector::<f64, D>::zeros();
        for i in 0..D {
            let mut p_plus = *point;
            let mut p_minus = *point;
            *p_plus.coord_mut(i) += epsilon;
            *p_minus.coord_mut(i) -= epsilon;
            let e_plus = self.field_energy(&p_plus) * curvature_scale;
            let e_minus = self.field_energy(&p_minus) * curvature_scale;
            grad[i] = (e_plus - e_minus) / (2.0 * epsilon);
        }
        grad
    }

    /// Laplacian of conformal parameter σ (for Ricci scalar, Fix 8).
    #[cfg(feature = "consciousness-curvature")]
    pub fn sigma_laplacian(&self, point: &Point<D>, curvature_scale: f64, epsilon: f64) -> f64 {
        let center = self.field_energy(point) * curvature_scale;
        let mut sum = 0.0;
        for i in 0..D {
            let mut p_plus = *point;
            let mut p_minus = *point;
            *p_plus.coord_mut(i) += epsilon;
            *p_minus.coord_mut(i) -= epsilon;
            let e_plus = self.field_energy(&p_plus) * curvature_scale;
            let e_minus = self.field_energy(&p_minus) * curvature_scale;
            sum += (e_plus - 2.0 * center + e_minus) / (epsilon * epsilon);
        }
        sum
    }
}

impl<const D: usize> Default for HarmonyField<D> {
    fn default() -> Self {
        Self::new()
    }
}

/// Compute the updated emotional contagion value for a single entity after one tick.
///
/// Implements a simple agent-based affect update. Nearby sources contribute
/// to a proximity-weighted social mean; the receiver drifts toward that mean at
/// a rate scaled by the model's Φ input. In isolation, emotion decays toward
/// zero at `EMOTIONAL_CONTAGION_DECAY`. This is a phenomenological rule, not
/// an electromagnetic-field model or a validation of CEMI theory.
///
/// # Parameters
/// - `position`: this entity's position as a nalgebra SVector
/// - `current_emotion`: entity's current `harmony[EMOTIONAL_CONTAGION_IDX]`
/// - `phi`: entity's consciousness level [0, 1] — gates reception sensitivity
/// - `sources`: (position, emotion, source_phi) tuples of *other* nearby entities
/// - `dt`: simulation time step in seconds
///
/// # Returns
/// New emotion value clamped to [0, 1].
pub fn contagion_update<const D: usize>(
    position: &SVector<f64, D>,
    current_emotion: f64,
    phi: f64,
    sources: &[(SVector<f64, D>, f64, f64)],
    dt: f64,
) -> f64 {
    let mut total_weight = 0.0f64;
    let mut weighted_emotion = 0.0f64;

    for (src_pos, src_emotion, src_phi) in sources {
        let dist = (src_pos - position).norm();
        if !(1e-6..=EMOTIONAL_CONTAGION_RADIUS).contains(&dist) {
            continue;
        }
        // Social field: sender's Φ amplifies their emotional broadcast strength.
        // 1/r falloff within the social radius (softer than EM 1/r² — social
        // emotion attenuates less steeply than physical fields).
        let weight = src_phi / dist.max(1.0);
        weighted_emotion += src_emotion * weight;
        total_weight += weight;
    }

    if total_weight > 1e-10 {
        let social_mean = weighted_emotion / total_weight;
        // Receiver's Φ gates how strongly they are influenced.
        // A zombie (Φ≈0) is immune to social emotion — no consciousness, no contagion.
        let reception = phi.clamp(0.0, 1.0);
        let drift = (social_mean - current_emotion) * EMOTIONAL_CONTAGION_RATE * reception * dt;
        (current_emotion + drift).clamp(0.0, 1.0)
    } else {
        // Social isolation: decay toward zero
        let decay = current_emotion * EMOTIONAL_CONTAGION_DECAY * dt;
        (current_emotion - decay).max(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stillness_source<const D: usize>(pos: Point<D>) -> HarmonySource<D> {
        let mut activations = [0.0; NUM_HARMONIES];
        activations[7] = 1.0; // Sacred Stillness
        HarmonySource {
            position: pos,
            activations,
            strength: 10.0,
            radius: 100.0,
            created_at: 0.0,
            propagation_speed: f64::MAX,
        }
    }

    #[test]
    fn sample_at_source() {
        let mut field = HarmonyField::<3>::new();
        field.sources.push(stillness_source(Point::origin()));

        let sample = field.sample(&Point::origin());
        // At distance ~1 (clamped), strength 10, stillness 1.0 → 10.0
        assert!(sample[7] > 1.0, "stillness = {}", sample[7]);
    }

    #[test]
    fn sample_falls_off_with_distance() {
        let mut field = HarmonyField::<3>::new();
        field.sources.push(stillness_source(Point::origin()));

        let near = field.sample(&Point::new([2.0, 0.0, 0.0]));
        let far = field.sample(&Point::new([10.0, 0.0, 0.0]));
        assert!(
            near[7] > far[7],
            "near {} should be > far {}",
            near[7],
            far[7]
        );
    }

    #[test]
    fn sample_zero_outside_radius() {
        let mut source = stillness_source::<3>(Point::origin());
        source.radius = 5.0;
        let mut field = HarmonyField::new();
        field.sources.push(source);

        let outside = field.sample(&Point::new([10.0, 0.0, 0.0]));
        assert!(outside[7] < 1e-10);
    }

    #[test]
    fn resonance_aligned() {
        let a = [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0];
        let b = [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0];
        let res = HarmonyField::<3>::resonance(&a, &b);
        assert!(
            (res - 1.0).abs() < 1e-10,
            "identical harmonies should resonate"
        );
    }

    #[test]
    fn resonance_opposed() {
        let a = [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        let b = [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0];
        let res = HarmonyField::<3>::resonance(&a, &b);
        assert!(
            (res - 0.0).abs() < 1e-10,
            "orthogonal harmonies = zero resonance"
        );
    }

    #[test]
    fn friction_reduced_by_resonance() {
        let mut field = HarmonyField::<3>::new();
        field.sources.push(stillness_source(Point::origin()));

        let mut entity_harmonies = [0.0; NUM_HARMONIES];
        entity_harmonies[7] = 1.0; // Same harmony → resonance

        let mult = field.friction_multiplier(&Point::new([2.0, 0.0, 0.0]), &entity_harmonies);
        assert!(
            mult < 1.0,
            "resonant friction mult {} should be < 1.0",
            mult
        );
    }

    #[test]
    fn friction_increased_by_dissonance() {
        let mut field = HarmonyField::<3>::new();
        field.sources.push(stillness_source(Point::origin()));

        let mut entity_harmonies = [0.0; NUM_HARMONIES];
        entity_harmonies[0] = 1.0; // Different harmony → less resonance

        // Since orthogonal harmonies give resonance=0, friction should be ~1.0
        let mult = field.friction_multiplier(&Point::new([2.0, 0.0, 0.0]), &entity_harmonies);
        assert!(
            (mult - 1.0).abs() < 0.5,
            "orthogonal friction mult {} should be near 1.0",
            mult
        );
    }

    #[test]
    fn multiple_sources_superpose() {
        let mut field = HarmonyField::<3>::new();
        field
            .sources
            .push(stillness_source(Point::new([5.0, 0.0, 0.0])));
        field
            .sources
            .push(stillness_source(Point::new([-5.0, 0.0, 0.0])));

        let single_source = {
            let mut f = HarmonyField::<3>::new();
            f.sources
                .push(stillness_source(Point::new([5.0, 0.0, 0.0])));
            f.sample(&Point::origin())
        };

        let both = field.sample(&Point::origin());
        assert!(
            both[7] > single_source[7],
            "superposition should be stronger"
        );
    }

    #[test]
    fn field_energy_sums() {
        let mut field = HarmonyField::<3>::new();
        field.sources.push(stillness_source(Point::origin()));
        let energy = field.field_energy(&Point::new([2.0, 0.0, 0.0]));
        assert!(energy > 0.0);
    }

    #[test]
    fn harmony_field_4d() {
        let mut field = HarmonyField::<4>::new();
        let source = HarmonySource {
            position: Point::origin(),
            activations: [0.5; NUM_HARMONIES],
            strength: 5.0,
            radius: 50.0,
            created_at: 0.0,
            propagation_speed: f64::MAX,
        };
        field.sources.push(source);
        let sample = field.sample(&Point::new([3.0, 0.0, 0.0, 0.0]));
        assert!(sample[0] > 0.0);
    }

    #[test]
    fn source_constructor_rejects_invalid_contract_values() {
        let valid_activations = [0.5; NUM_HARMONIES];
        assert_eq!(
            HarmonySource::<3>::try_new(
                Point::origin(),
                valid_activations,
                -1.0,
                10.0,
                0.0,
                f64::MAX,
            )
            .unwrap_err(),
            HarmonyFieldInputError::NegativeStrength { source_index: 0 }
        );
        assert_eq!(
            HarmonySource::<3>::try_new(
                Point::origin(),
                valid_activations,
                1.0,
                0.0,
                0.0,
                f64::MAX,
            )
            .unwrap_err(),
            HarmonyFieldInputError::NonPositiveRadius { source_index: 0 }
        );
        assert!(matches!(
            HarmonySource::<3>::try_new(
                Point::new([f64::NAN, 0.0, 0.0]),
                valid_activations,
                1.0,
                10.0,
                0.0,
                f64::MAX,
            ),
            Err(HarmonyFieldInputError::NonFiniteSourcePosition { axis: 0, .. })
        ));
        let mut negative_activation = valid_activations;
        negative_activation[4] = -0.1;
        assert!(matches!(
            HarmonySource::<3>::try_new(
                Point::origin(),
                negative_activation,
                1.0,
                10.0,
                0.0,
                f64::MAX,
            ),
            Err(HarmonyFieldInputError::ActivationOutOfRange { index: 4, .. })
        ));
        let mut nan_activation = valid_activations;
        nan_activation[3] = f64::NAN;
        assert!(matches!(
            HarmonySource::<3>::try_new(Point::origin(), nan_activation, 1.0, 10.0, 0.0, f64::MAX,),
            Err(HarmonyFieldInputError::NonFiniteActivation { index: 3, .. })
        ));
        assert!(matches!(
            HarmonySource::<3>::try_new(Point::origin(), valid_activations, 1.0, 10.0, 0.0, 0.0,),
            Err(HarmonyFieldInputError::NonPositivePropagationSpeed { .. })
        ));
        assert!(matches!(
            HarmonySource::<3>::try_new(
                Point::origin(),
                valid_activations,
                f64::INFINITY,
                10.0,
                0.0,
                f64::MAX,
            ),
            Err(HarmonyFieldInputError::NonFiniteStrength { .. })
        ));
        assert!(matches!(
            HarmonySource::<3>::try_new(
                Point::origin(),
                valid_activations,
                1.0,
                f64::NAN,
                0.0,
                f64::MAX,
            ),
            Err(HarmonyFieldInputError::NonFiniteRadius { .. })
        ));
        assert!(matches!(
            HarmonySource::<3>::try_new(
                Point::origin(),
                valid_activations,
                1.0,
                -1.0,
                0.0,
                f64::MAX,
            ),
            Err(HarmonyFieldInputError::NonPositiveRadius { .. })
        ));
        assert!(matches!(
            HarmonySource::<3>::try_new(
                Point::origin(),
                valid_activations,
                1.0,
                10.0,
                f64::NAN,
                f64::MAX,
            ),
            Err(HarmonyFieldInputError::NonFiniteCreatedAt { .. })
        ));
        assert!(matches!(
            HarmonySource::<3>::try_new(
                Point::origin(),
                valid_activations,
                1.0,
                10.0,
                -1.0,
                f64::MAX,
            ),
            Err(HarmonyFieldInputError::NegativeCreatedAt { .. })
        ));
        assert!(matches!(
            HarmonySource::<3>::try_new(Point::origin(), valid_activations, 1.0, 10.0, 0.0, -1.0,),
            Err(HarmonyFieldInputError::NonPositivePropagationSpeed { .. })
        ));
        let mut over_activation = valid_activations;
        over_activation[6] = 1.01;
        assert!(matches!(
            HarmonySource::<3>::try_new(Point::origin(), over_activation, 1.0, 10.0, 0.0, f64::MAX,),
            Err(HarmonyFieldInputError::ActivationOutOfRange { index: 6, .. })
        ));
    }

    #[test]
    fn checked_add_rejects_invalid_source_without_mutating_field() {
        let mut field = HarmonyField::<3>::new();
        let mut invalid = stillness_source(Point::origin());
        invalid.radius = -1.0;
        assert_eq!(
            field.try_add_source(invalid).unwrap_err(),
            HarmonyFieldInputError::NonPositiveRadius { source_index: 0 }
        );
        assert!(field.sources.is_empty());
    }

    #[test]
    fn checked_field_detects_invalid_public_mutation() {
        let mut field = HarmonyField::<3>::new();
        let mut source = stillness_source(Point::origin());
        source.strength = f64::NAN;
        // Public fields remain source-compatible; checked operations revalidate them.
        field.sources.push(source);
        assert_eq!(
            field.try_sample(&Point::origin()).unwrap_err(),
            HarmonyFieldInputError::NonFiniteStrength { source_index: 0 }
        );
        assert!(field.sample(&Point::origin())[7].is_nan());
        assert!(field.field_energy(&Point::origin()).is_nan());
        assert!(
            field
                .friction_multiplier(&Point::origin(), &[0.5; NUM_HARMONIES])
                .is_nan()
        );
    }

    #[test]
    fn checked_field_rejects_invalid_query_and_time() {
        let field = HarmonyField::<3>::new();
        assert_eq!(
            field
                .try_sample(&Point::new([f64::INFINITY, 0.0, 0.0]))
                .unwrap_err(),
            HarmonyFieldInputError::NonFiniteQueryCoordinate { axis: 0 }
        );
        assert_eq!(
            field
                .try_sample_at_time(&Point::origin(), f64::NAN)
                .unwrap_err(),
            HarmonyFieldInputError::NonFiniteCurrentTime
        );
        assert_eq!(
            field
                .try_sample_at_time(&Point::origin(), -1.0)
                .unwrap_err(),
            HarmonyFieldInputError::NegativeCurrentTime
        );
    }

    #[test]
    fn checked_field_rejects_non_finite_distance_and_propagation_delay() {
        let mut source = stillness_source(Point::new([f64::MAX, 0.0, 0.0]));
        source.radius = f64::MAX;
        let mut field = HarmonyField::<3>::new();
        field.try_add_source(source).unwrap();
        assert_eq!(
            field
                .try_sample(&Point::new([-f64::MAX, 0.0, 0.0]))
                .unwrap_err(),
            HarmonyFieldInputError::NonFiniteDistance { source_index: 0 }
        );

        let source = HarmonySource::<3>::try_new(
            Point::origin(),
            [1.0; NUM_HARMONIES],
            1.0,
            f64::MAX,
            0.0,
            1.0e-200,
        )
        .unwrap();
        let field = HarmonyField::try_from_sources(vec![source]).unwrap();
        assert_eq!(
            field
                .try_sample_at_time(&Point::new([1.0e150, 0.0, 0.0]), 1.0)
                .unwrap_err(),
            HarmonyFieldInputError::NonFinitePropagationDelay { source_index: 0 }
        );
    }

    #[test]
    fn checked_field_energy_rejects_overflow_across_harmonies() {
        let source = HarmonySource::<3>::try_new(
            Point::origin(),
            [1.0; NUM_HARMONIES],
            f64::MAX,
            10.0,
            0.0,
            f64::MAX,
        )
        .unwrap();
        let field = HarmonyField::try_from_sources(vec![source]).unwrap();
        assert!(field.try_sample(&Point::origin()).is_ok());
        assert_eq!(
            field.try_field_energy(&Point::origin()).unwrap_err(),
            HarmonyFieldInputError::NonFiniteFieldEnergy
        );
    }

    #[test]
    fn checked_resonance_defines_zero_norm_and_handles_extreme_magnitudes() {
        assert_eq!(
            HarmonyField::<3>::try_resonance(&[0.0; NUM_HARMONIES], &[1.0; NUM_HARMONIES]).unwrap(),
            0.0
        );
        let huge = [f64::MAX; NUM_HARMONIES];
        let resonance = HarmonyField::<3>::try_resonance(&huge, &huge).unwrap();
        assert!(resonance.is_finite());
        assert!((resonance - 1.0).abs() < 1e-12);
        let mut negative = [0.0; NUM_HARMONIES];
        negative[2] = -1.0;
        assert_eq!(
            HarmonyField::<3>::try_resonance(&negative, &[1.0; NUM_HARMONIES]).unwrap_err(),
            HarmonyFieldInputError::NegativeResonanceInput {
                vector: 0,
                index: 2
            }
        );
    }

    #[test]
    fn checked_field_rejects_overflow_from_multiple_extreme_sources() {
        let mut activations = [0.0; NUM_HARMONIES];
        activations[0] = 1.0;
        let source = HarmonySource::<3>::try_new(
            Point::origin(),
            activations,
            f64::MAX,
            10.0,
            0.0,
            f64::MAX,
        )
        .unwrap();
        let mut field = HarmonyField::<3>::new();
        field.try_add_source(source.clone()).unwrap();
        field.try_add_source(source).unwrap();
        assert_eq!(
            field.try_sample(&Point::origin()).unwrap_err(),
            HarmonyFieldInputError::NonFiniteFieldValue { harmony_index: 0 }
        );
    }

    #[test]
    fn checked_constructors_accept_documented_instant_propagation_sentinel() {
        let source = HarmonySource::<3>::try_new(
            Point::origin(),
            [0.5; NUM_HARMONIES],
            1.0,
            10.0,
            0.0,
            f64::MAX,
        )
        .unwrap();
        let field = HarmonyField::try_from_sources(vec![source]).unwrap();
        assert!(field.try_sample_at_time(&Point::origin(), 0.0).is_ok());
    }

    // ── Emotional Contagion (harmony index 8) tests ──────────────────────────

    #[test]
    fn contagion_spreads_toward_social_mean() {
        // Entity at origin with emotion=0.0, nearby high-emotion agent at (5, 0)
        let pos = nalgebra::SVector::<f64, 2>::from([0.0, 0.0]);
        let sources = [(
            nalgebra::SVector::<f64, 2>::from([5.0, 0.0]),
            0.8, // src_emotion
            0.9, // src_phi (high consciousness = strong broadcast)
        )];
        let new_val = contagion_update(&pos, 0.0, 0.8, &sources, 1.0);
        assert!(
            new_val > 0.0,
            "emotion should spread from nearby agent, got {new_val}"
        );
        assert!(new_val < 0.8, "should not fully equalise in one tick");
    }

    #[test]
    fn contagion_decay_in_isolation() {
        // Entity with emotion=0.6, no neighbors → decays toward zero
        let pos = nalgebra::SVector::<f64, 2>::from([0.0, 0.0]);
        let new_val = contagion_update(&pos, 0.6, 0.8, &[], 1.0);
        assert!(
            new_val < 0.6,
            "isolated emotion should decay, got {new_val}"
        );
        assert!(new_val > 0.0, "should not decay to zero in one tick");
    }

    #[test]
    fn contagion_low_phi_reduces_reception() {
        // Zombie entity (phi≈0) should barely receive contagion
        let pos = nalgebra::SVector::<f64, 2>::from([0.0, 0.0]);
        let sources = [(
            nalgebra::SVector::<f64, 2>::from([3.0, 0.0]),
            1.0, // src_emotion
            0.9, // src_phi
        )];
        let zombie_val = contagion_update(&pos, 0.0, 0.0, &sources, 1.0);
        let conscious_val = contagion_update(&pos, 0.0, 0.9, &sources, 1.0);
        assert!(
            zombie_val < conscious_val,
            "low-phi entity should receive less contagion: zombie={zombie_val:.4} vs conscious={conscious_val:.4}"
        );
    }

    #[test]
    fn contagion_zero_beyond_radius() {
        // Source outside EMOTIONAL_CONTAGION_RADIUS → no transfer
        let pos = nalgebra::SVector::<f64, 2>::from([0.0, 0.0]);
        let sources = [(
            nalgebra::SVector::<f64, 2>::from([EMOTIONAL_CONTAGION_RADIUS + 5.0, 0.0]),
            1.0, // src_emotion
            1.0, // src_phi
        )];
        let new_val = contagion_update(&pos, 0.0, 1.0, &sources, 1.0);
        assert_eq!(new_val, 0.0, "out-of-radius source should have no effect");
    }

    #[test]
    fn contagion_output_stays_in_unit_interval() {
        // Stress test: extreme inputs should not escape [0, 1]
        let pos = nalgebra::SVector::<f64, 2>::from([0.0, 0.0]);
        let sources = [
            (nalgebra::SVector::<f64, 2>::from([1.0, 0.0]), 1.0, 1.0),
            (nalgebra::SVector::<f64, 2>::from([2.0, 0.0]), 1.0, 1.0),
        ];
        // Very large dt should still clamp output
        let new_val = contagion_update(&pos, 0.0, 1.0, &sources, 100.0);
        assert!(
            (0.0..=1.0).contains(&new_val),
            "output must be in [0,1], got {new_val}"
        );
    }
}
