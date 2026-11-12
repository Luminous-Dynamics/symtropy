// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Deterministic Preisach-style relay ensemble for OLI physical-memory controls.
//!
//! Independent bistable hysterons switch state only when a scalar drive crosses their
//! upper or lower thresholds. Between thresholds, each hysteron retains its previous
//! state. The ensemble therefore stores information about the drive history and exhibits
//! rate-independent hysteresis and return-point memory.
//!
//! This is a known-rule phenomenological model of physical memory. It has no task objective,
//! adaptive parameter update, reinforcement, learning, reproduction, metabolism, or agency.
//! Hysteresis / history retention must not be promoted to learning or intelligence.

use std::error::Error;
use std::fmt;

/// State of one bistable relay element.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HysteronState {
    Negative,
    Positive,
}

/// One rate-independent bistable relay with distinct switching thresholds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hysteron {
    lower_threshold: f64,
    upper_threshold: f64,
    weight: f64,
    state: HysteronState,
}

impl Hysteron {
    pub fn new(
        lower_threshold: f64,
        upper_threshold: f64,
        weight: f64,
        initial_state: HysteronState,
    ) -> Result<Self, HysteresisError> {
        if !lower_threshold.is_finite() {
            return Err(HysteresisError::NonFiniteParameter("lower_threshold"));
        }
        if !upper_threshold.is_finite() {
            return Err(HysteresisError::NonFiniteParameter("upper_threshold"));
        }
        if lower_threshold >= upper_threshold {
            return Err(HysteresisError::InvalidThresholdOrder {
                lower: lower_threshold,
                upper: upper_threshold,
            });
        }
        if !weight.is_finite() || weight <= 0.0 {
            return Err(HysteresisError::InvalidWeight(weight));
        }
        Ok(Self {
            lower_threshold,
            upper_threshold,
            weight,
            state: initial_state,
        })
    }

    pub const fn state(&self) -> HysteronState {
        self.state
    }

    pub const fn lower_threshold(&self) -> f64 {
        self.lower_threshold
    }

    pub const fn upper_threshold(&self) -> f64 {
        self.upper_threshold
    }

    pub const fn weight(&self) -> f64 {
        self.weight
    }

    fn drive(&mut self, input: f64) -> bool {
        let next_state = match self.state {
            HysteronState::Negative if input >= self.upper_threshold => {
                HysteronState::Positive
            }
            HysteronState::Positive if input <= self.lower_threshold => {
                HysteronState::Negative
            }
            state => state,
        };
        let changed = next_state != self.state;
        self.state = next_state;
        changed
    }
}

/// Aggregate observables after one drive update.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HysteresisMetrics {
    pub input: f64,
    /// Weight-normalized fraction of hysteron mass in the positive state, [0, 1].
    pub positive_weight_fraction: f64,
    /// Normalized weighted magnetization / order parameter, [-1, 1].
    pub weighted_magnetization: f64,
    pub switched_hysteron_count: usize,
    pub hysteron_count: usize,
}

/// Deterministic ensemble of independent Preisach-style relay hysterons.
#[derive(Debug, Clone, PartialEq)]
pub struct PreisachRelayEnsemble {
    hysterons: Vec<Hysteron>,
    total_weight: f64,
    last_input: Option<f64>,
}

impl PreisachRelayEnsemble {
    pub fn new(hysterons: Vec<Hysteron>) -> Result<Self, HysteresisError> {
        if hysterons.is_empty() {
            return Err(HysteresisError::EmptyEnsemble);
        }
        let total_weight = hysterons.iter().map(Hysteron::weight).sum::<f64>();
        if !total_weight.is_finite() || total_weight <= 0.0 {
            return Err(HysteresisError::InvalidTotalWeight(total_weight));
        }
        Ok(Self {
            hysterons,
            total_weight,
            last_input: None,
        })
    }

    pub fn hysterons(&self) -> &[Hysteron] {
        &self.hysterons
    }

    pub fn last_input(&self) -> Option<f64> {
        self.last_input
    }

    pub fn snapshot(&self) -> Vec<HysteronState> {
        self.hysterons.iter().map(Hysteron::state).collect()
    }

    pub fn hysteron_count(&self) -> usize {
        self.hysterons.len()
    }

    /// Apply one scalar drive value and retain the relay state between thresholds.
    pub fn drive(&mut self, input: f64) -> Result<HysteresisMetrics, HysteresisError> {
        if !input.is_finite() {
            return Err(HysteresisError::NonFiniteInput(input));
        }
        let mut switched_hysteron_count = 0;
        let mut positive_weight = 0.0;
        for hysteron in &mut self.hysterons {
            if hysteron.drive(input) {
                switched_hysteron_count += 1;
            }
            if hysteron.state == HysteronState::Positive {
                positive_weight += hysteron.weight;
            }
        }
        self.last_input = Some(input);
        let positive_weight_fraction = positive_weight / self.total_weight;
        Ok(HysteresisMetrics {
            input,
            positive_weight_fraction,
            weighted_magnetization: 2.0 * positive_weight_fraction - 1.0,
            switched_hysteron_count,
            hysteron_count: self.hysterons.len(),
        })
    }

    /// Replay a finite input sequence, returning one measurement per input value.
    pub fn replay(&mut self, inputs: &[f64]) -> Result<Vec<HysteresisMetrics>, HysteresisError> {
        inputs.iter().map(|input| self.drive(*input)).collect()
    }
}

/// Invalid hysteron or ensemble configuration / drive.
#[derive(Debug, Clone, PartialEq)]
pub enum HysteresisError {
    EmptyEnsemble,
    NonFiniteParameter(&'static str),
    InvalidThresholdOrder { lower: f64, upper: f64 },
    InvalidWeight(f64),
    InvalidTotalWeight(f64),
    NonFiniteInput(f64),
}

impl fmt::Display for HysteresisError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyEnsemble => write!(f, "Preisach relay ensemble must contain a hysteron"),
            Self::NonFiniteParameter(name) => write!(f, "{name} must be finite"),
            Self::InvalidThresholdOrder { lower, upper } => write!(
                f,
                "hysteron lower threshold {lower} must be below upper threshold {upper}"
            ),
            Self::InvalidWeight(weight) => {
                write!(f, "hysteron weight must be finite and > 0, got {weight}")
            }
            Self::InvalidTotalWeight(weight) => {
                write!(f, "ensemble total weight must be finite and > 0, got {weight}")
            }
            Self::NonFiniteInput(input) => {
                write!(f, "hysteresis drive input must be finite, got {input}")
            }
        }
    }
}

impl Error for HysteresisError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn ensemble(initial_state: HysteronState) -> PreisachRelayEnsemble {
        let hysterons = [
            (-1.0, 1.0, 1.0),
            (-0.5, 0.5, 1.0),
            (-0.2, 0.2, 1.0),
        ]
        .into_iter()
        .map(|(lower, upper, weight)| {
            Hysteron::new(lower, upper, weight, initial_state).unwrap()
        })
        .collect();
        PreisachRelayEnsemble::new(hysterons).unwrap()
    }

    #[test]
    fn same_present_input_can_have_different_outputs_after_different_histories() {
        let mut positive_history = ensemble(HysteronState::Negative);
        positive_history.replay(&[-2.0, 1.0]).unwrap();
        let positive_at_zero = positive_history.drive(0.0).unwrap();

        let mut negative_history = ensemble(HysteronState::Negative);
        negative_history.replay(&[-2.0, -1.0]).unwrap();
        let negative_at_zero = negative_history.drive(0.0).unwrap();

        assert_eq!(positive_at_zero.input, negative_at_zero.input);
        assert_eq!(positive_at_zero.weighted_magnetization, 1.0);
        assert_eq!(negative_at_zero.weighted_magnetization, -1.0);
        assert_ne!(
            positive_history.snapshot(),
            negative_history.snapshot()
        );
    }

    #[test]
    fn nested_minor_loop_recovers_the_return_point_state() {
        let mut model = ensemble(HysteronState::Negative);
        model.drive(-2.0).unwrap();
        model.drive(0.75).unwrap();
        let at_upper_turning_point = model.snapshot();

        model.drive(0.0).unwrap();
        model.drive(0.75).unwrap();

        assert_eq!(model.snapshot(), at_upper_turning_point);
    }

    #[test]
    fn major_loop_erases_history_at_the_opposite_saturation_field() {
        let mut model = ensemble(HysteronState::Negative);
        model.drive(2.0).unwrap();
        assert!(model
            .snapshot()
            .iter()
            .all(|state| *state == HysteronState::Positive));

        model.drive(-2.0).unwrap();
        assert!(model
            .snapshot()
            .iter()
            .all(|state| *state == HysteronState::Negative));
    }

    #[test]
    fn rate_independent_relay_states_ignore_dwell_count_for_same_monotone_sweep() {
        let mut coarse = ensemble(HysteronState::Negative);
        coarse.replay(&[-2.0, 1.0]).unwrap();

        let mut fine = ensemble(HysteronState::Negative);
        fine.replay(&[
            -2.0, -1.5, -1.0, -0.75, -0.5, -0.25, 0.0, 0.25, 0.5, 0.75, 1.0,
        ])
        .unwrap();

        assert_eq!(coarse.snapshot(), fine.snapshot());
        assert_eq!(
            coarse.drive(0.0).unwrap().weighted_magnetization,
            fine.drive(0.0).unwrap().weighted_magnetization
        );
    }

    #[test]
    fn repeated_drive_cycles_replay_without_learning_or_parameter_change() {
        let mut model = ensemble(HysteronState::Negative);
        let thresholds: Vec<_> = model
            .hysterons()
            .iter()
            .map(|hysteron| {
                (
                    hysteron.lower_threshold(),
                    hysteron.upper_threshold(),
                    hysteron.weight(),
                )
            })
            .collect();

        let first_cycle = model.replay(&[-2.0, 2.0, -2.0]).unwrap();
        let first_final_state = model.snapshot();
        let second_cycle = model.replay(&[-2.0, 2.0, -2.0]).unwrap();

        assert_eq!(first_final_state, model.snapshot());
        assert_eq!(
            first_cycle
                .iter()
                .map(|metrics| metrics.weighted_magnetization)
                .collect::<Vec<_>>(),
            second_cycle
                .iter()
                .map(|metrics| metrics.weighted_magnetization)
                .collect::<Vec<_>>()
        );
        let final_thresholds: Vec<_> = model
            .hysterons()
            .iter()
            .map(|hysteron| {
                (
                    hysteron.lower_threshold(),
                    hysteron.upper_threshold(),
                    hysteron.weight(),
                )
            })
            .collect();
        assert_eq!(thresholds, final_thresholds);
    }

    #[test]
    fn rejects_invalid_thresholds_weights_and_inputs() {
        assert!(matches!(
            Hysteron::new(1.0, 1.0, 1.0, HysteronState::Negative),
            Err(HysteresisError::InvalidThresholdOrder { .. })
        ));
        assert!(matches!(
            Hysteron::new(0.0, 1.0, 0.0, HysteronState::Negative),
            Err(HysteresisError::InvalidWeight(_))
        ));
        assert!(matches!(
            PreisachRelayEnsemble::new(Vec::new()),
            Err(HysteresisError::EmptyEnsemble)
        ));
        let mut model = ensemble(HysteronState::Negative);
        assert!(matches!(
            model.drive(f64::NAN),
            Err(HysteresisError::NonFiniteInput(_))
        ));
    }
}
