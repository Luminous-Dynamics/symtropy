// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Deterministic protection-stack semantics for Symtropy.
//!
//! The core intentionally models generic world effects rather than real weapon or
//! armor performance. A protection layer transforms an incoming effect into an
//! absorbed portion and a residual effect, producing an auditable receipt. Armor,
//! pressure suits, vehicle hulls, environmental barriers, and fictional fields may
//! all implement the same semantics without becoming special cases in health logic.

pub mod active_field;

use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, error::Error, fmt};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum EffectKind {
    Impact,
    Thermal,
    Electrical,
    Radiation,
    Pressure,
    Chemical,
    Acoustic,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Effect {
    /// Abstract non-negative severity in the owning world's chosen unit system.
    pub magnitude: f64,
    pub kind: EffectKind,
}

impl Effect {
    pub fn validate(&self) -> Result<(), ProtectionError> {
        if !self.magnitude.is_finite() || self.magnitude < 0.0 {
            return Err(ProtectionError::InvalidMagnitude(self.magnitude));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LayerResponse {
    /// Fraction [0,1] of the target effect a healthy layer attempts to absorb.
    pub absorption_fraction: f64,
    /// Abstract layer capacity consumed one-for-one by absorbed magnitude.
    pub capacity: f64,
}

impl LayerResponse {
    fn validate(&self) -> Result<(), ProtectionError> {
        if !self.absorption_fraction.is_finite()
            || !(0.0..=1.0).contains(&self.absorption_fraction)
        {
            return Err(ProtectionError::InvalidAbsorption(self.absorption_fraction));
        }
        if !self.capacity.is_finite() || self.capacity < 0.0 {
            return Err(ProtectionError::InvalidCapacity(self.capacity));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProtectionLayer {
    pub id: String,
    /// Layer condition [0,1]. Condition scales both attempted absorption and usable capacity.
    pub condition: f64,
    pub responses: BTreeMap<EffectKind, LayerResponse>,
    remaining_capacity: BTreeMap<EffectKind, f64>,
}

impl ProtectionLayer {
    pub fn new(
        id: impl Into<String>,
        condition: f64,
        responses: BTreeMap<EffectKind, LayerResponse>,
    ) -> Result<Self, ProtectionError> {
        let id = id.into();
        if id.is_empty() {
            return Err(ProtectionError::EmptyLayerId);
        }
        if !condition.is_finite() || !(0.0..=1.0).contains(&condition) {
            return Err(ProtectionError::InvalidCondition(condition));
        }
        for response in responses.values() {
            response.validate()?;
        }
        let remaining_capacity = responses
            .iter()
            .map(|(kind, response)| (*kind, response.capacity * condition))
            .collect();
        Ok(Self {
            id,
            condition,
            responses,
            remaining_capacity,
        })
    }

    pub fn remaining_capacity(&self, kind: EffectKind) -> f64 {
        self.remaining_capacity.get(&kind).copied().unwrap_or(0.0)
    }

    pub fn restore_capacity(&mut self, kind: EffectKind, amount: f64) -> Result<(), ProtectionError> {
        if !amount.is_finite() || amount < 0.0 {
            return Err(ProtectionError::InvalidCapacity(amount));
        }
        let Some(response) = self.responses.get(&kind) else {
            return Ok(());
        };
        let ceiling = response.capacity * self.condition;
        let remaining = self.remaining_capacity.entry(kind).or_insert(0.0);
        *remaining = (*remaining + amount).min(ceiling);
        Ok(())
    }

    fn apply(&mut self, effect: Effect) -> Result<LayerReceipt, ProtectionError> {
        effect.validate()?;
        let incoming = effect.magnitude;
        let Some(response) = self.responses.get(&effect.kind).copied() else {
            return Ok(LayerReceipt {
                layer_id: self.id.clone(),
                kind: effect.kind,
                incoming,
                absorbed: 0.0,
                transmitted: incoming,
                capacity_before: 0.0,
                capacity_after: 0.0,
            });
        };

        let capacity_before = self.remaining_capacity(effect.kind);
        let target_absorption = incoming * response.absorption_fraction * self.condition;
        let absorbed = target_absorption.min(capacity_before);
        let transmitted = (incoming - absorbed).max(0.0);
        let capacity_after = (capacity_before - absorbed).max(0.0);
        self.remaining_capacity.insert(effect.kind, capacity_after);

        Ok(LayerReceipt {
            layer_id: self.id.clone(),
            kind: effect.kind,
            incoming,
            absorbed,
            transmitted,
            capacity_before,
            capacity_after,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayerReceipt {
    pub layer_id: String,
    pub kind: EffectKind,
    pub incoming: f64,
    pub absorbed: f64,
    pub transmitted: f64,
    pub capacity_before: f64,
    pub capacity_after: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProtectionResolution {
    pub original: Effect,
    pub residual: Effect,
    pub layers: Vec<LayerReceipt>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ProtectionStack {
    /// Ordered outermost -> innermost.
    pub layers: Vec<ProtectionLayer>,
}

impl ProtectionStack {
    pub fn resolve(&mut self, effect: Effect) -> Result<ProtectionResolution, ProtectionError> {
        effect.validate()?;
        let mut residual = effect;
        let mut receipts = Vec::with_capacity(self.layers.len());
        for layer in &mut self.layers {
            let receipt = layer.apply(residual)?;
            residual.magnitude = receipt.transmitted;
            receipts.push(receipt);
            if residual.magnitude == 0.0 {
                break;
            }
        }
        Ok(ProtectionResolution {
            original: effect,
            residual,
            layers: receipts,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProtectionError {
    EmptyLayerId,
    InvalidMagnitude(f64),
    InvalidAbsorption(f64),
    InvalidCapacity(f64),
    InvalidCondition(f64),
}

impl fmt::Display for ProtectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyLayerId => write!(f, "protection layer id must not be empty"),
            Self::InvalidMagnitude(value) => write!(f, "invalid effect magnitude {value}"),
            Self::InvalidAbsorption(value) => write!(f, "invalid absorption fraction {value}"),
            Self::InvalidCapacity(value) => write!(f, "invalid protection capacity {value}"),
            Self::InvalidCondition(value) => write!(f, "invalid protection condition {value}"),
        }
    }
}

impl Error for ProtectionError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn response(absorption_fraction: f64, capacity: f64) -> BTreeMap<EffectKind, LayerResponse> {
        BTreeMap::from([(
            EffectKind::Impact,
            LayerResponse {
                absorption_fraction,
                capacity,
            },
        )])
    }

    #[test]
    fn layers_resolve_outermost_to_innermost() {
        let mut stack = ProtectionStack {
            layers: vec![
                ProtectionLayer::new("outer", 1.0, response(0.5, 100.0)).unwrap(),
                ProtectionLayer::new("inner", 1.0, response(0.5, 100.0)).unwrap(),
            ],
        };
        let result = stack
            .resolve(Effect {
                magnitude: 80.0,
                kind: EffectKind::Impact,
            })
            .unwrap();
        assert_eq!(result.layers[0].absorbed, 40.0);
        assert_eq!(result.layers[1].absorbed, 20.0);
        assert_eq!(result.residual.magnitude, 20.0);
    }

    #[test]
    fn exhausted_capacity_cannot_absorb_twice() {
        let mut stack = ProtectionStack {
            layers: vec![ProtectionLayer::new("layer", 1.0, response(1.0, 10.0)).unwrap()],
        };
        let first = stack
            .resolve(Effect {
                magnitude: 10.0,
                kind: EffectKind::Impact,
            })
            .unwrap();
        assert_eq!(first.residual.magnitude, 0.0);
        let second = stack
            .resolve(Effect {
                magnitude: 10.0,
                kind: EffectKind::Impact,
            })
            .unwrap();
        assert_eq!(second.residual.magnitude, 10.0);
    }

    #[test]
    fn unrelated_effect_passes_through_unchanged() {
        let mut stack = ProtectionStack {
            layers: vec![ProtectionLayer::new("impact-only", 1.0, response(1.0, 100.0)).unwrap()],
        };
        let result = stack
            .resolve(Effect {
                magnitude: 12.0,
                kind: EffectKind::Thermal,
            })
            .unwrap();
        assert_eq!(result.residual.magnitude, 12.0);
        assert_eq!(result.layers[0].absorbed, 0.0);
    }

    #[test]
    fn damaged_layer_has_less_usable_capacity_and_absorption() {
        let full = ProtectionLayer::new("full", 1.0, response(1.0, 100.0)).unwrap();
        let damaged = ProtectionLayer::new("damaged", 0.25, response(1.0, 100.0)).unwrap();
        assert_eq!(full.remaining_capacity(EffectKind::Impact), 100.0);
        assert_eq!(damaged.remaining_capacity(EffectKind::Impact), 25.0);
    }

    #[test]
    fn deterministic_replay_produces_identical_receipts() {
        let stack = ProtectionStack {
            layers: vec![ProtectionLayer::new("outer", 0.8, response(0.7, 100.0)).unwrap()],
        };
        let mut a = stack.clone();
        let mut b = stack;
        for i in 0..20 {
            let effect = Effect {
                magnitude: (i + 1) as f64,
                kind: EffectKind::Impact,
            };
            assert_eq!(a.resolve(effect), b.resolve(effect));
        }
        assert_eq!(a, b);
    }
}
