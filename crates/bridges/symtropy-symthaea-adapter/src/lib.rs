// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Bridge a Symthaea [`EmbodimentBridge`] into the Symtropy [`MotorPlanner`]
//! protocol.
//!
//! The Symtropy [`RoboticAgent`](symtropy_robotics_bridge::RoboticAgent)
//! computes a scalar NRC safety gain each tick and then hands that gain to a
//! [`MotorPlanner`] to produce per-actuator commands. Platform crates in
//! Symthaea (`symthaea-humanoid`, `symthaea-flight`, `symthaea-manipulator`,
//! etc.) implement [`EmbodimentBridge`], whose `step(thought_hv, dt, phi)`
//! internally translates thought to per-joint torques and advances the
//! platform's own physics.
//!
//! [`EmbodimentPlanner`] glues those two worlds together. It:
//! 1. Encodes the observation vector into a [`ContinuousHV`] "thought" using
//!    compatibility tiling or an opt-in deterministic semantic HDC encoder.
//! 2. Calls `bridge.step(&thought_hv, dt, gain as f64)`.
//! 3. Returns a per-actuator `Vec<f64>` sized to `num_actuators`, where each
//!    slot holds the bridge's commanded authority for that actuator — gated
//!    by the bridge's reported `MotorSafetyLevel` and capped by the incoming
//!    NRC gain.
//!
//! The commands are currently uniform across actuators, reflecting the
//! information available in [`EmbodimentResult`]. A future version can shift
//! to per-joint telemetry if [`EmbodimentBridge`] grows a
//! `last_motor_commands()` accessor.
//!
//! # Example
//!
//! ```ignore
//! use symtropy_robotics_bridge::{MotorPlanner, PlatformType, RoboticAgent};
//! use symtropy_symthaea_adapter::EmbodimentPlanner;
//! use symthaea_core::embodiment::EmbodimentBridge;
//! use symtropy_physics::body::BodyHandle;
//!
//! let bridge: Box<dyn EmbodimentBridge> = make_humanoid_bridge();
//! let mut planner = EmbodimentPlanner::new(bridge);
//!
//! let mut agent = RoboticAgent::new(BodyHandle(0), PlatformType::Humanoid, "Bot");
//! let commands = agent.tick_motor_commands_with(&[0.5, 0.3, 0.1, 0.2], 0.1, &mut planner);
//! assert_eq!(commands.len(), 21);
//! ```

use symthaea_core::embodiment::EmbodimentBridge;
use symthaea_core::hdc::ContinuousHV;
use symtropy_hdc_core::{
    BipolarBundle, EncoderSpec, HdcError, ItemMemory, LevelEncoder, StableHash64,
};
use symtropy_robotics_bridge_core::motor::MotorPlanner;

/// Default simulation timestep fed to `EmbodimentBridge::step` (60 Hz).
pub const DEFAULT_DT: f32 = 1.0 / 60.0;

/// Default hypervector dimension used for the encoded thought.
///
/// Matches Symthaea's 16,384D HDC convention. Platforms that expect a
/// different dimension will internally rebuild their encoders, but the
/// default is correct for every built-in platform.
pub const DEFAULT_HV_DIM: usize = 16_384;

/// Wraps a Symthaea [`EmbodimentBridge`] as a Symtropy [`MotorPlanner`].
pub struct EmbodimentPlanner {
    bridge: Box<dyn EmbodimentBridge>,
    dt: f32,
    hv_dim: usize,
    observation_encoder: ObservationEncoding,
}

/// Observation encoding used before a thought vector enters Symthaea.
///
/// Legacy tiling remains the default for source and behavior compatibility.
/// Semantic HDC is opt-in until platform-level validation is complete.
pub enum ObservationEncoding {
    LegacyTiled,
    SemanticHdc(SemanticObservationEncoder),
}

/// Deterministic role/value encoder for numeric observation channels.
pub struct SemanticObservationEncoder {
    memory: ItemMemory,
    scalar: LevelEncoder,
    value_extent: f64,
}

impl SemanticObservationEncoder {
    pub fn new(spec: EncoderSpec, value_extent: f64) -> Result<Self, HdcError> {
        if !value_extent.is_finite() || value_extent <= 0.0 {
            return Err(HdcError::InvalidEncoderSpec(
                "observation value_extent must be finite and positive".to_owned(),
            ));
        }
        let memory = ItemMemory::new(spec)?;
        let scalar = memory.scalar_encoder("symtropy.symthaea.observation.v1");
        Ok(Self {
            memory,
            scalar,
            value_extent,
        })
    }

    pub fn dimension(&self) -> usize {
        self.memory.spec().dimension
    }

    /// Stable provenance fingerprint for the semantic observation contract.
    pub fn fingerprint(&self) -> u64 {
        let mut hash = StableHash64::with_seed(self.memory.spec().fingerprint());
        hash.write(b"symtropy.symthaea.observation.v1");
        hash.write_u64(self.value_extent.to_bits());
        hash.finish()
    }

    pub fn encode(&self, observation: &[f64]) -> Result<ContinuousHV, HdcError> {
        let mut bundle = BipolarBundle::new(self.dimension());
        bundle.add(&self.memory.item("record-kind", "numeric-observation"))?;
        bundle.add(&self.memory.bind_role_value(
            "channel-count",
            "channel-count",
            &observation.len().to_string(),
        ))?;
        if observation.iter().any(|value| !value.is_finite()) {
            return Err(HdcError::NonFiniteValue);
        }
        for (index, value) in observation.iter().copied().enumerate() {
            let role = self.memory.role(&format!("channel.{index}"));
            let encoded = self
                .scalar
                .encode(value, -self.value_extent, self.value_extent)?;
            bundle.add(&role.bind(&encoded)?)?;
        }
        let bipolar = bundle.finish(&self.memory.tie_breaker("numeric-observation"))?;
        let normalization = (bipolar.len() as f32).sqrt().recip();
        Ok(ContinuousHV::from_vec(
            bipolar
                .values
                .iter()
                .map(|value| f32::from(*value) * normalization)
                .collect(),
        ))
    }
}

impl EmbodimentPlanner {
    /// Wrap a bridge with default timestep (60 Hz) and 16,384D HVs.
    pub fn new(bridge: Box<dyn EmbodimentBridge>) -> Self {
        Self {
            bridge,
            dt: DEFAULT_DT,
            hv_dim: DEFAULT_HV_DIM,
            observation_encoder: ObservationEncoding::LegacyTiled,
        }
    }

    /// Override the simulation timestep fed to `bridge.step`.
    pub fn with_dt(mut self, dt: f32) -> Self {
        self.dt = dt;
        self
    }

    /// Override the hypervector dimension used by legacy tiling.
    /// Semantic HDC uses the dimension in its `EncoderSpec`.
    pub fn with_hv_dim(mut self, hv_dim: usize) -> Self {
        self.hv_dim = hv_dim;
        self
    }

    /// Opt into deterministic semantic HDC observation encoding.
    ///
    /// The supplied encoder spec becomes part of experiment provenance. Legacy
    /// tiling remains the default so existing platform behavior does not change
    /// silently.
    pub fn with_semantic_hdc(
        mut self,
        spec: EncoderSpec,
        value_extent: f64,
    ) -> Result<Self, HdcError> {
        let encoder = SemanticObservationEncoder::new(spec, value_extent)?;
        self.hv_dim = encoder.dimension();
        self.observation_encoder = ObservationEncoding::SemanticHdc(encoder);
        Ok(self)
    }

    /// Restore the compatibility encoder.
    pub fn with_legacy_tiling(mut self) -> Self {
        self.observation_encoder = ObservationEncoding::LegacyTiled;
        self
    }

    /// Read-only access to the wrapped bridge.
    pub fn bridge(&self) -> &dyn EmbodimentBridge {
        self.bridge.as_ref()
    }

    /// Mutable access to the wrapped bridge (for reset, safety overrides, ...).
    pub fn bridge_mut(&mut self) -> &mut dyn EmbodimentBridge {
        self.bridge.as_mut()
    }

    /// Current simulation timestep.
    pub fn dt(&self) -> f32 {
        self.dt
    }

    /// Semantic encoder fingerprint for research provenance, or `None` while
    /// compatibility tiling is active.
    pub fn observation_encoder_fingerprint(&self) -> Option<u64> {
        match &self.observation_encoder {
            ObservationEncoding::LegacyTiled => None,
            ObservationEncoding::SemanticHdc(encoder) => Some(encoder.fingerprint()),
        }
    }
}

impl MotorPlanner for EmbodimentPlanner {
    fn plan(&mut self, observation: &[f64], gain: f64, num_actuators: usize) -> Vec<f64> {
        // Invalid observations or control parameters must not be converted to
        // ordinary numeric values and then allowed to influence actuation.
        // In particular, NaN bypasses ordinary range comparisons/clamps.
        if !gain.is_finite()
            || !self.dt.is_finite()
            || self.dt <= 0.0
            || self.hv_dim == 0
            // The legacy tiled encoder narrows f64 channels to f32. A finite
            // f64 can overflow to infinity during that conversion, so validate
            // representability before constructing the hypervector.
            || observation
                .iter()
                .any(|value| !value.is_finite() || !(*value as f32).is_finite())
        {
            return vec![0.0; num_actuators];
        }

        // Legacy tiling stores each channel directly as f32 and the bridge
        // may compute an f32 L2 norm. Finite individual components are not enough:
        // their squared sum must also remain representable.
        if matches!(&self.observation_encoder, ObservationEncoding::LegacyTiled)
            && !legacy_tiled_norm_is_representable(observation, self.hv_dim)
        {
            return vec![0.0; num_actuators];
        }

        let capped_gain = gain.clamp(0.0, 1.0);
        let thought_hv = match &self.observation_encoder {
            ObservationEncoding::LegacyTiled => encode_observation(observation, self.hv_dim),
            ObservationEncoding::SemanticHdc(encoder) => match encoder.encode(observation) {
                Ok(encoded) => encoded,
                Err(_) => return vec![0.0; num_actuators],
            },
        };
        let result = self.bridge.step(&thought_hv, self.dt, capped_gain);

        // A failed bridge step or malformed telemetry conveys no authority.
        // Do not let a finite-looking effort mask an explicit failure, NaN,
        // or infinity in the rest of the result.
        if !result.success
            || !result.control_effort.is_finite()
            || !result.prediction_error.is_finite()
            || !result.observation_confidence.is_finite()
        {
            return vec![0.0; num_actuators];
        }

        // Translate the bridge's scalar response into a bounded per-actuator
        // authority. This adapter exposes authority, not a signed actuator
        // torque; platform-specific command semantics remain bridge-owned.
        // A configured safety override is part of the bridge's live state.
        // The per-step result may still contain an earlier safety level, so
        // honor the more restrictive of the reported and currently enforced levels.
        let result_gain = result.safety_level.motor_gain() as f64;
        let live_bridge_gain = self.bridge.safety_level().motor_gain() as f64;
        let bridge_gain = result_gain.min(live_bridge_gain);
        let effort = (result.control_effort as f64).abs();
        let raw_authority = bridge_gain * effort;
        if !raw_authority.is_finite() {
            return vec![0.0; num_actuators];
        }

        let per_joint = raw_authority.min(capped_gain).clamp(0.0, 1.0);
        vec![per_joint; num_actuators]
    }
}

/// Check that the legacy tiled vector's f32 L2 norm can be accumulated safely.
///
/// This computes the repeated-channel squared norm analytically, avoiding a
/// loop proportional to `hv_dim` just for validation. The half-MAX bound leaves
/// headroom for f32 multiplication and summation rounding.
fn legacy_tiled_norm_is_representable(obs: &[f64], hv_dim: usize) -> bool {
    if obs.is_empty() || hv_dim == 0 {
        return true;
    }

    let repeats_each = hv_dim / obs.len();
    let extra_repeats = hv_dim % obs.len();
    let mut squared_norm = 0.0_f64;
    let limit = (f32::MAX as f64) * 0.5;

    for (index, value) in obs.iter().copied().enumerate() {
        let narrowed = value as f32;
        if !narrowed.is_finite() {
            return false;
        }
        let repeats = repeats_each + usize::from(index < extra_repeats);
        squared_norm += (narrowed as f64) * (narrowed as f64) * (repeats as f64);
        if !squared_norm.is_finite() || squared_norm > limit {
            return false;
        }
    }
    true
}

/// Encode a short observation vector into a dim-wide [`ContinuousHV`].
///
/// Empty observations produce a zero HV. Non-empty observations are tiled
/// into the HV (each slot `i` gets `obs[i % obs.len()]`), which is sufficient
/// for platforms whose encoders re-embed the thought via their own HDC
/// projection. Platforms that require a specific semantic encoding should
/// compose their own planner rather than relying on this default.
pub fn encode_observation(obs: &[f64], hv_dim: usize) -> ContinuousHV {
    if obs.is_empty() {
        return ContinuousHV::zero(hv_dim);
    }
    let mut values = Vec::with_capacity(hv_dim);
    for i in 0..hv_dim {
        values.push(obs[i % obs.len()] as f32);
    }
    ContinuousHV::from_vec(values)
}

/// Encode numeric observations with versioned semantic HDC rather than raw
/// value tiling. This helper is useful for research harnesses that do not need
/// an `EmbodimentPlanner`.
pub fn encode_observation_semantic(
    observation: &[f64],
    spec: EncoderSpec,
    value_extent: f64,
) -> Result<ContinuousHV, HdcError> {
    SemanticObservationEncoder::new(spec, value_extent)?.encode(observation)
}

#[cfg(test)]
mod tests {
    use super::*;
    use symthaea_core::embodiment::{
        AgentIdentity, EmbodimentPlatform, EmbodimentResult, EmbodimentTelemetry, MotorSafetyLevel,
    };

    /// Minimal EmbodimentBridge for testing the adapter without pulling a
    /// real platform crate. Records every call to step() and returns a
    /// configurable EmbodimentResult.
    struct MockBridge {
        step_count: usize,
        last_thought_norm: f32,
        last_dt: f32,
        last_phi: f64,
        result_safety: MotorSafetyLevel,
        result_effort: f32,
        result_success: bool,
        result_prediction_error: f32,
        result_observation_confidence: f32,
        num_actuators: usize,
        safety_override: Option<MotorSafetyLevel>,
    }

    impl MockBridge {
        fn new() -> Self {
            Self {
                step_count: 0,
                last_thought_norm: 0.0,
                last_dt: 0.0,
                last_phi: 0.0,
                result_safety: MotorSafetyLevel::Green,
                result_effort: 1.0,
                result_success: true,
                result_prediction_error: 0.1,
                result_observation_confidence: 0.9,
                num_actuators: 21,
                safety_override: None,
            }
        }
    }

    impl EmbodimentBridge for MockBridge {
        fn step(&mut self, thought_hv: &ContinuousHV, dt: f32, phi: f64) -> EmbodimentResult {
            self.step_count += 1;
            self.last_thought_norm = thought_hv.values.iter().map(|v| v * v).sum::<f32>().sqrt();
            self.last_dt = dt;
            self.last_phi = phi;
            EmbodimentResult {
                num_actuators: self.num_actuators,
                control_effort: self.result_effort,
                success: self.result_success,
                prediction_error: self.result_prediction_error,
                safety_level: self.result_safety,
                epistemic_grounding: 0,
                observation_confidence: self.result_observation_confidence,
            }
        }

        fn encode_perception(&mut self) -> ContinuousHV {
            ContinuousHV::zero(DEFAULT_HV_DIM)
        }

        fn reset(&mut self) {
            self.step_count = 0;
        }

        fn safety_level(&self) -> MotorSafetyLevel {
            self.safety_override.unwrap_or(self.result_safety)
        }

        fn set_safety_override(&mut self, level: MotorSafetyLevel) {
            self.safety_override = Some(level);
        }

        fn clear_safety_override(&mut self) {
            self.safety_override = None;
        }

        fn platform(&self) -> EmbodimentPlatform {
            EmbodimentPlatform::None
        }

        fn num_actuators(&self) -> usize {
            self.num_actuators
        }

        fn total_steps(&self) -> usize {
            self.step_count
        }

        fn telemetry(&self) -> EmbodimentTelemetry {
            EmbodimentTelemetry {
                total_steps: self.step_count as u64,
                ..Default::default()
            }
        }

        fn agent_identity(&self) -> AgentIdentity {
            AgentIdentity::new("mock")
        }
    }

    fn make_planner() -> EmbodimentPlanner {
        EmbodimentPlanner::new(Box::new(MockBridge::new()))
    }

    #[test]
    fn semantic_observation_encoding_is_deterministic_and_unit_norm() {
        let spec = EncoderSpec {
            schema_version: 1,
            dimension: 4_096,
            seed: 9,
            scalar_levels: 129,
        };
        let a = encode_observation_semantic(&[0.2, -0.7, 1.1], spec.clone(), 2.0).unwrap();
        let b = encode_observation_semantic(&[0.2, -0.7, 1.1], spec, 2.0).unwrap();
        assert_eq!(a.values, b.values);
        let norm = a
            .values
            .iter()
            .map(|value| value * value)
            .sum::<f32>()
            .sqrt();
        assert!((norm - 1.0).abs() < 1.0e-5);
    }

    #[test]
    fn semantic_observation_encoding_preserves_channel_roles() {
        let spec = EncoderSpec {
            schema_version: 1,
            dimension: 4_096,
            seed: 11,
            scalar_levels: 129,
        };
        let left = encode_observation_semantic(&[0.9, -0.3], spec.clone(), 1.0).unwrap();
        let swapped = encode_observation_semantic(&[-0.3, 0.9], spec, 1.0).unwrap();
        assert_ne!(left.values, swapped.values);
    }

    #[test]
    fn semantic_planner_exposes_encoder_fingerprint() {
        let spec = EncoderSpec {
            schema_version: 1,
            dimension: 4_096,
            seed: 13,
            scalar_levels: 129,
        };
        let planner = make_planner().with_semantic_hdc(spec, 2.0).unwrap();
        assert!(planner.observation_encoder_fingerprint().is_some());
    }

    #[test]
    fn default_dt_is_60hz() {
        let p = make_planner();
        assert!((p.dt() - 1.0 / 60.0).abs() < 1e-9);
    }

    #[test]
    fn with_dt_overrides_timestep() {
        let p = EmbodimentPlanner::new(Box::new(MockBridge::new())).with_dt(0.01);
        assert!((p.dt() - 0.01).abs() < 1e-9);
    }

    #[test]
    fn plan_returns_vec_of_requested_length() {
        let mut p = make_planner();
        let cmds = p.plan(&[0.5], 1.0, 21);
        assert_eq!(cmds.len(), 21);
    }

    #[test]
    fn plan_invokes_bridge_step_once() {
        let mut p = make_planner();
        let _ = p.plan(&[0.5, 0.3], 1.0, 21);
        assert_eq!(p.bridge().total_steps(), 1);
        let _ = p.plan(&[0.5, 0.3], 1.0, 21);
        assert_eq!(p.bridge().total_steps(), 2);
    }

    #[test]
    fn plan_forwards_gain_as_phi() {
        let mut p = make_planner();
        let _ = p.plan(&[0.5], 0.42, 21);
        // We can't see phi from the trait, but we can observe that a lower
        // gain cap is respected in the output (Green × 1.0 = 1.0, capped to
        // 0.42).
        let cmds = p.plan(&[0.5], 0.42, 4);
        for c in &cmds {
            assert!(*c <= 0.42 + 1e-12, "gain cap violated: {c} > 0.42");
        }
    }

    #[test]
    fn red_safety_zeros_output() {
        let mut bridge = MockBridge::new();
        bridge.result_safety = MotorSafetyLevel::Red;
        let mut p = EmbodimentPlanner::new(Box::new(bridge));
        let cmds = p.plan(&[0.5], 1.0, 21);
        for c in &cmds {
            assert_eq!(*c, 0.0);
        }
    }

    #[test]
    fn yellow_safety_scales_output() {
        let mut bridge = MockBridge::new();
        bridge.result_safety = MotorSafetyLevel::Yellow;
        bridge.result_effort = 1.0;
        let mut p = EmbodimentPlanner::new(Box::new(bridge));
        let cmds = p.plan(&[0.5], 1.0, 4);
        // Yellow gain is 0.6, effort is 1.0, incoming cap is 1.0 → 0.6
        for c in &cmds {
            assert!((c - 0.6).abs() < 1e-6);
        }
    }

    #[test]
    fn zero_actuators_yields_empty_vec() {
        let mut p = make_planner();
        let cmds = p.plan(&[0.5], 1.0, 0);
        assert!(cmds.is_empty());
    }

    #[test]
    fn empty_observation_produces_zero_hv_and_still_steps() {
        let mut p = make_planner();
        let _ = p.plan(&[], 1.0, 21);
        assert_eq!(p.bridge().total_steps(), 1);
    }

    #[test]
    fn reset_propagates_to_bridge() {
        let mut p = make_planner();
        let _ = p.plan(&[0.5], 1.0, 21);
        assert_eq!(p.bridge().total_steps(), 1);
        p.bridge_mut().reset();
        assert_eq!(p.bridge().total_steps(), 0);
    }

    #[test]
    fn safety_override_plumbs_through() {
        let mut p = make_planner();
        p.bridge_mut().set_safety_override(MotorSafetyLevel::Orange);
        assert_eq!(p.bridge().safety_level(), MotorSafetyLevel::Orange);
        p.bridge_mut().clear_safety_override();
        assert_eq!(p.bridge().safety_level(), MotorSafetyLevel::Green);
    }

    #[test]
    fn encode_observation_empty_is_zero() {
        let hv = encode_observation(&[], 64);
        assert_eq!(hv.values.len(), 64);
        for v in &hv.values {
            assert_eq!(*v, 0.0);
        }
    }

    #[test]
    fn encode_observation_tiles_short_input() {
        let hv = encode_observation(&[0.25, -0.5], 8);
        assert_eq!(
            hv.values,
            vec![0.25, -0.5, 0.25, -0.5, 0.25, -0.5, 0.25, -0.5]
        );
    }

    #[test]
    fn encode_observation_respects_hv_dim() {
        let hv = encode_observation(&[1.0], 128);
        assert_eq!(hv.values.len(), 128);
    }

    #[test]
    fn control_effort_modulates_output() {
        let mut bridge = MockBridge::new();
        bridge.result_safety = MotorSafetyLevel::Green;
        bridge.result_effort = 0.5;
        let mut p = EmbodimentPlanner::new(Box::new(bridge));
        let cmds = p.plan(&[0.5], 1.0, 4);
        // Green gain = 1.0, effort = 0.5, cap = 1.0 → 0.5
        for c in &cmds {
            assert!((c - 0.5).abs() < 1e-6);
        }
    }

    #[test]
    fn gain_cap_binds_even_when_bridge_authority_is_full() {
        let mut bridge = MockBridge::new();
        bridge.result_safety = MotorSafetyLevel::Green;
        bridge.result_effort = 1.0;
        let mut p = EmbodimentPlanner::new(Box::new(bridge));
        let cmds = p.plan(&[0.5], 0.3, 4);
        // Bridge authority = 1.0 × 1.0 = 1.0, but cap is 0.3
        for c in &cmds {
            assert!((c - 0.3).abs() < 1e-6);
        }
    }

    #[test]
    fn non_finite_gain_fails_closed_without_stepping_bridge() {
        let mut p = make_planner();
        let commands = p.plan(&[0.5], f64::NAN, 4);
        assert_eq!(commands, vec![0.0; 4]);
        assert_eq!(p.bridge().total_steps(), 0);
    }

    #[test]
    fn non_finite_observation_fails_closed_without_stepping_bridge() {
        let mut p = make_planner();
        let commands = p.plan(&[0.5, f64::INFINITY], 1.0, 4);
        assert_eq!(commands, vec![0.0; 4]);
        assert_eq!(p.bridge().total_steps(), 0);
    }

    #[test]
    fn finite_f64_that_overflows_f32_fails_closed_without_stepping_bridge() {
        let mut p = make_planner();
        let commands = p.plan(&[f64::MAX], 1.0, 4);
        assert_eq!(commands, vec![0.0; 4]);
        assert_eq!(p.bridge().total_steps(), 0);
    }

    #[test]
    fn finite_f32_components_with_overflowing_norm_fail_closed() {
        let mut p = make_planner();
        // 1e20 is finite in f32, but its repeated squared sum is not.
        let commands = p.plan(&[1.0e20], 1.0, 4);
        assert_eq!(commands, vec![0.0; 4]);
        assert_eq!(p.bridge().total_steps(), 0);
    }

    #[test]
    fn invalid_timestep_fails_closed_without_stepping_bridge() {
        let mut p = EmbodimentPlanner::new(Box::new(MockBridge::new())).with_dt(0.0);
        let commands = p.plan(&[0.5], 1.0, 4);
        assert_eq!(commands, vec![0.0; 4]);
        assert_eq!(p.bridge().total_steps(), 0);
    }

    #[test]
    fn unsuccessful_bridge_result_fails_closed() {
        let mut bridge = MockBridge::new();
        bridge.result_success = false;
        let mut p = EmbodimentPlanner::new(Box::new(bridge));
        let commands = p.plan(&[0.5], 1.0, 4);
        assert_eq!(commands, vec![0.0; 4]);
        assert_eq!(p.bridge().total_steps(), 1);
    }

    #[test]
    fn non_finite_bridge_telemetry_fails_closed() {
        let mut bridge = MockBridge::new();
        bridge.result_effort = f32::NAN;
        let mut p = EmbodimentPlanner::new(Box::new(bridge));
        let commands = p.plan(&[0.5], 1.0, 4);
        assert_eq!(commands, vec![0.0; 4]);
        assert_eq!(p.bridge().total_steps(), 1);
    }

    #[test]
    fn authority_is_capped_at_one_even_if_effort_exceeds_one() {
        let mut bridge = MockBridge::new();
        bridge.result_effort = 20.0;
        let mut p = EmbodimentPlanner::new(Box::new(bridge));
        let commands = p.plan(&[0.5], 2.0, 4);
        assert_eq!(commands, vec![1.0; 4]);
    }

    #[test]
    fn bridge_safety_override_caps_authority_even_if_step_result_is_green() {
        let mut p = make_planner();
        p.bridge_mut().set_safety_override(MotorSafetyLevel::Red);

        // The mock step reports Green, while the live bridge safety
        // override reports Red. The override must dominate the stale result.
        let commands = p.plan(&[0.5], 1.0, 4);
        assert_eq!(commands, vec![0.0; 4]);
        assert_eq!(p.bridge().total_steps(), 1);
    }

    #[test]
    fn semantic_encoder_rejects_non_finite_observation_channels() {
        let spec = EncoderSpec {
            schema_version: 1,
            dimension: 4_096,
            seed: 17,
            scalar_levels: 129,
        };
        let result = encode_observation_semantic(&[0.2, f64::NAN], spec, 2.0);
        assert!(result.is_err());
    }

    #[test]
    fn negative_control_effort_is_abs_valued() {
        let mut bridge = MockBridge::new();
        bridge.result_safety = MotorSafetyLevel::Green;
        bridge.result_effort = -0.8;
        let mut p = EmbodimentPlanner::new(Box::new(bridge));
        let cmds = p.plan(&[0.5], 1.0, 4);
        // |-0.8| = 0.8
        for c in &cmds {
            assert!((c - 0.8).abs() < 1e-6);
        }
    }
}
