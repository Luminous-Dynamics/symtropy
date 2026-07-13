// SPDX-License-Identifier: AGPL-3.0-or-later
//! Minimal stand-in for the real `symthaea-biometrics` crate (private).
//! Reimplements only the public shape (types + method signatures) that
//! symtropy calls; uses simple velocity/keystroke-rate EMA heuristics
//! rather than the real crate's Echo-State-Network surprise detector.

pub mod input_telemetry {
    use std::collections::VecDeque;

    const MAX_MOUSE_HISTORY: usize = 8;

    #[derive(Debug, Clone, Copy)]
    pub struct MouseSample {
        pub x: f32,
        pub y: f32,
        pub timestamp_ms: u64,
    }

    #[derive(Debug, Clone, Copy)]
    pub struct KeystrokeSample {
        pub key_down: bool,
        pub timestamp_ms: u64,
    }

    #[derive(Debug, Clone, Copy, Default)]
    pub struct StressVector {
        pub arousal: f32,
        pub valence: f32,
        pub dominance: f32,
    }

    pub struct InputTelemetryEncoder {
        mouse_history: VecDeque<MouseSample>,
        velocity_ema: f32,
        keystroke_rate_ema: f32,
    }

    impl InputTelemetryEncoder {
        pub fn new() -> Self {
            Self::with_seed(42)
        }

        pub fn with_seed(_seed: u64) -> Self {
            Self {
                mouse_history: VecDeque::with_capacity(MAX_MOUSE_HISTORY),
                velocity_ema: 0.0,
                keystroke_rate_ema: 0.0,
            }
        }

        pub fn push_mouse(&mut self, sample: MouseSample) {
            if let Some(prev) = self.mouse_history.back() {
                let dt_secs =
                    (sample.timestamp_ms.saturating_sub(prev.timestamp_ms)).max(1) as f32 / 1000.0;
                let dx = sample.x - prev.x;
                let dy = sample.y - prev.y;
                let v = (dx * dx + dy * dy).sqrt() / dt_secs;
                self.velocity_ema = self.velocity_ema * 0.85 + v.min(10.0) * 0.15;
            }
            if self.mouse_history.len() >= MAX_MOUSE_HISTORY {
                self.mouse_history.pop_front();
            }
            self.mouse_history.push_back(sample);
        }

        pub fn push_keystroke(&mut self, sample: KeystrokeSample) {
            if sample.key_down {
                self.keystroke_rate_ema = self.keystroke_rate_ema * 0.9 + 0.1;
            }
        }

        pub fn compute_stress_vector(&self) -> StressVector {
            let arousal = (self.velocity_ema * 0.5 + self.keystroke_rate_ema * 0.5).clamp(0.0, 1.0);
            StressVector {
                arousal,
                valence: 0.0,
                dominance: 0.0,
            }
        }
    }

    impl Default for InputTelemetryEncoder {
        fn default() -> Self {
            Self::new()
        }
    }
}

pub mod stress_model {
    use crate::input_telemetry::StressVector;

    const BURNOUT_THRESHOLD: f32 = 0.8;

    #[derive(Debug, Clone)]
    pub struct PlayerStressModel {
        pub allostatic_load: f32,
        pub cortisol_proxy: f32,
        pub da_baseline: f32,
        pub sht_baseline: f32,
        pub calm_duration_secs: f32,
    }

    impl PlayerStressModel {
        pub fn new() -> Self {
            Self {
                allostatic_load: 0.0,
                cortisol_proxy: 0.0,
                da_baseline: 0.5,
                sht_baseline: 0.5,
                calm_duration_secs: 0.0,
            }
        }

        pub fn tick(&mut self, stress: &StressVector, dt_secs: f32) {
            if stress.arousal > 0.6 {
                self.cortisol_proxy = (self.cortisol_proxy + 0.02 * dt_secs).min(1.0);
                self.allostatic_load = (self.allostatic_load + 0.01 * dt_secs).min(1.0);
                self.calm_duration_secs = 0.0;
            } else {
                self.cortisol_proxy = (self.cortisol_proxy - 0.005 * dt_secs).max(0.0);
                self.allostatic_load = (self.allostatic_load - 0.002 * dt_secs).max(0.0);
                self.calm_duration_secs += dt_secs;
            }
        }

        pub fn is_burnout(&self) -> bool {
            self.allostatic_load > BURNOUT_THRESHOLD
        }

        pub fn reset(&mut self) {
            *self = Self::new();
        }
    }

    impl Default for PlayerStressModel {
        fn default() -> Self {
            Self::new()
        }
    }
}
