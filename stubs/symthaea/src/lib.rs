// SPDX-License-Identifier: AGPL-3.0-or-later
//! Minimal stand-in for the real `symthaea` crate (private, unpublished,
//! ~1.68M LOC). Only reimplements the tiny surface actually used in this
//! repo (see `crates/bridges/symthaea-bevy-brain/src/lib.rs`): a trivial
//! `CognitiveLoopService` that returns zeroed telemetry every cycle. Real
//! HDC/CfC/Phi cognition lives in the private monorepo -- this stub exists
//! purely so the public repo's Cargo graph resolves and `cargo check`
//! succeeds.

pub use symthaea_core;

pub mod cognitive_loop {
    use symthaea_core::hdc::unified_hv::ContinuousHV;

    #[derive(Default, Clone, Debug)]
    pub struct EmbodiedTelemetry {
        pub body_phi_modulation: f64,
    }

    #[derive(Default, Clone, Debug)]
    pub struct CycleMetadata {
        pub mce_softmin: f64,
        pub mce_weighted_sum: f64,
        pub perception_attention_sensitivity: f32,
        pub mce_social: f64,
        pub mce_narrative: f64,
        pub memory_recall_quality: f32,
        pub embodied: EmbodiedTelemetry,
    }

    #[derive(Clone, Debug, Default)]
    pub struct CycleResult {
        pub output: Vec<f32>,
        pub metadata: CycleMetadata,
    }

    /// Trivial cognitive loop: produces zeroed output and telemetry every
    /// cycle. See module docs for why.
    pub struct CognitiveLoopService {
        cfc_neurons: usize,
        priors_mean: Vec<f64>,
        priors_precision: Vec<f64>,
    }

    impl CognitiveLoopService {
        pub fn inject_priors(&mut self, mean: Vec<f64>, precision: Vec<f64>) {
            self.priors_mean = mean;
            self.priors_precision = precision;
        }

        pub fn cycle(&mut self, _input: &str) -> CycleResult {
            CycleResult {
                output: vec![0.0; self.cfc_neurons],
                metadata: CycleMetadata::default(),
            }
        }

        pub fn cycle_with_hv(&mut self, _hv: &ContinuousHV) -> CycleResult {
            CycleResult {
                output: vec![0.0; self.cfc_neurons],
                metadata: CycleMetadata::default(),
            }
        }

        pub fn consciousness_level(&self) -> f32 {
            0.5
        }
    }

    #[derive(Default)]
    pub struct CognitiveLoopBuilder {
        cfc_neurons: usize,
        genesis_phrase: String,
    }

    impl CognitiveLoopBuilder {
        pub fn new() -> Self {
            Self::default()
        }

        pub fn with_cfc_neurons(mut self, n: usize) -> Self {
            self.cfc_neurons = n;
            self
        }

        pub fn with_genesis_phrase(mut self, phrase: &str) -> Self {
            self.genesis_phrase = phrase.to_string();
            self
        }

        pub fn build(self) -> Result<CognitiveLoopService, String> {
            let _ = &self.genesis_phrase;
            Ok(CognitiveLoopService {
                cfc_neurons: self.cfc_neurons,
                priors_mean: Vec::new(),
                priors_precision: Vec::new(),
            })
        }
    }
}
