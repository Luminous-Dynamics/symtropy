// SPDX-License-Identifier: AGPL-3.0-or-later
//! Minimal stand-in for the real `symthaea-orbital` crate (private). Only
//! implements the real, published `EmbodimentBridge` trait using the real
//! `MotorSafetyLevel` Phi-tier logic; does not reimplement the private
//! crate's zero-g dual-body physics simulator/controller/encoder.

pub mod embodiment {
    use symthaea_core::embodiment::{
        EmbodimentBridge, EmbodimentPlatform, EmbodimentResult, EmbodimentTelemetry,
        GROUNDING_SENSORIMOTOR, MoralGateInput, MotorSafetyLevel, grounding_from_prediction_error,
        grounding_label,
    };
    use symthaea_core::genesis::GenesisSeed;
    use symthaea_core::hdc::ContinuousHV;

    const NUM_ACTUATORS: usize = 7;
    const HV_DIM: usize = 16_384;

    pub struct OrbitalEmbodiment {
        safety: MotorSafetyLevel,
        safety_ov: Option<MotorSafetyLevel>,
        moral_safety: Option<MotorSafetyLevel>,
        steps: usize,
        effort: f32,
        pe: f32,
    }

    impl OrbitalEmbodiment {
        pub fn new(_genesis: &GenesisSeed) -> Self {
            Self {
                safety: MotorSafetyLevel::Green,
                safety_ov: None,
                moral_safety: None,
                steps: 0,
                effort: 0.0,
                pe: 0.0,
            }
        }
    }

    impl EmbodimentBridge for OrbitalEmbodiment {
        fn step(&mut self, _hv: &ContinuousHV, _dt: f32, phi: f64) -> EmbodimentResult {
            let mut safety = MotorSafetyLevel::from_phi(phi);
            if let Some(ov) = self.safety_ov {
                safety = safety.max(ov);
            }
            if let Some(m) = self.moral_safety {
                safety = safety.max(m);
            }
            self.safety = safety;
            self.effort = safety.motor_gain();
            self.pe = 0.0;
            self.steps += 1;
            EmbodimentResult {
                num_actuators: NUM_ACTUATORS,
                control_effort: self.effort,
                success: true,
                prediction_error: self.pe,
                safety_level: self.safety,
                epistemic_grounding: GROUNDING_SENSORIMOTOR,
                observation_confidence: grounding_from_prediction_error(self.pe),
            }
        }

        fn encode_perception(&mut self) -> ContinuousHV {
            ContinuousHV::zero(HV_DIM)
        }

        fn reset(&mut self) {
            self.safety = MotorSafetyLevel::Green;
            self.safety_ov = None;
            self.moral_safety = None;
            self.steps = 0;
            self.effort = 0.0;
            self.pe = 0.0;
        }

        fn safety_level(&self) -> MotorSafetyLevel {
            self.safety
        }

        fn set_safety_override(&mut self, level: MotorSafetyLevel) {
            self.safety_ov = Some(level);
        }

        fn clear_safety_override(&mut self) {
            self.safety_ov = None;
        }

        fn platform(&self) -> EmbodimentPlatform {
            EmbodimentPlatform::Orbital
        }

        fn num_actuators(&self) -> usize {
            NUM_ACTUATORS
        }

        fn total_steps(&self) -> usize {
            self.steps
        }

        fn telemetry(&self) -> EmbodimentTelemetry {
            EmbodimentTelemetry {
                total_steps: self.steps as u64,
                control_effort: self.effort,
                prediction_error: self.pe,
                safety_level: format!("{:?}", self.safety),
                platform: "orbital".to_string(),
                num_actuators: NUM_ACTUATORS,
                epistemic_grounding: grounding_label(GROUNDING_SENSORIMOTOR).to_string(),
                observation_confidence: grounding_from_prediction_error(self.pe),
                platform_specific: Vec::new(),
            }
        }

        fn apply_moral_gate(&mut self, gate: MoralGateInput) {
            self.moral_safety = if gate.ahimsa_violated || gate.verdict == MoralGateInput::VERDICT_BLOCKED
            {
                Some(MotorSafetyLevel::Red)
            } else if gate.consent_violation {
                Some(MotorSafetyLevel::Orange)
            } else if gate.verdict == MoralGateInput::VERDICT_CAUTION {
                Some(MotorSafetyLevel::Yellow)
            } else {
                None
            };
        }
    }
}
