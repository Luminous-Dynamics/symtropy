// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Fictional active-field protection for Symtropy worlds.
//!
//! This is deliberately **game simulation**, not an engineering model or claim
//! that personal force fields are physically realizable. The field owns one
//! shared charge pool across all supported effect kinds, accumulates abstract
//! thermal load, and exposes an operating signature. Its residual effect can be
//! passed into the ordinary `ProtectionStack` for armor/body resolution.

use crate::{Effect, EffectKind, ProtectionError};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActiveFieldConfig {
    /// Maximum shared field charge in abstract simulation units.
    pub max_charge: f64,
    /// Maximum charge restored per simulation second before efficiency.
    pub max_recharge_rate: f64,
    /// Fraction [0,1] of supplied recharge resource stored as field charge.
    pub recharge_efficiency: f64,
    /// Per-effect attempted absorption fractions.
    pub absorption: BTreeMap<EffectKind, f64>,
    /// Abstract heat generated per unit magnitude absorbed.
    pub heat_per_absorbed: f64,
    /// Abstract passive heat rejection per simulation second.
    pub cooling_rate: f64,
    /// Thermal load where absorption/recharge begin to derate.
    pub derate_start: f64,
    /// Thermal load where the field is forced offline.
    pub shutdown_heat: f64,
    /// Base detectable signature while enabled, [0,1].
    pub base_signature: f64,
}

impl ActiveFieldConfig {
    pub fn validate(&self) -> Result<(), FieldError> {
        if !self.max_charge.is_finite() || self.max_charge <= 0.0 {
            return Err(FieldError::InvalidConfig("max_charge"));
        }
        if !self.max_recharge_rate.is_finite() || self.max_recharge_rate < 0.0 {
            return Err(FieldError::InvalidConfig("max_recharge_rate"));
        }
        if !self.recharge_efficiency.is_finite()
            || !(0.0..=1.0).contains(&self.recharge_efficiency)
        {
            return Err(FieldError::InvalidConfig("recharge_efficiency"));
        }
        if !self.heat_per_absorbed.is_finite() || self.heat_per_absorbed < 0.0 {
            return Err(FieldError::InvalidConfig("heat_per_absorbed"));
        }
        if !self.cooling_rate.is_finite() || self.cooling_rate < 0.0 {
            return Err(FieldError::InvalidConfig("cooling_rate"));
        }
        if !self.derate_start.is_finite()
            || !self.shutdown_heat.is_finite()
            || self.derate_start < 0.0
            || self.shutdown_heat <= self.derate_start
        {
            return Err(FieldError::InvalidConfig("thermal thresholds"));
        }
        if !self.base_signature.is_finite() || !(0.0..=1.0).contains(&self.base_signature) {
            return Err(FieldError::InvalidConfig("base_signature"));
        }
        for fraction in self.absorption.values() {
            if !fraction.is_finite() || !(0.0..=1.0).contains(fraction) {
                return Err(FieldError::InvalidConfig("absorption"));
            }
        }
        Ok(())
    }
}

impl Default for ActiveFieldConfig {
    fn default() -> Self {
        Self {
            max_charge: 100.0,
            max_recharge_rate: 15.0,
            recharge_efficiency: 0.85,
            absorption: BTreeMap::from([
                (EffectKind::Impact, 0.90),
                (EffectKind::Thermal, 0.70),
                (EffectKind::Electrical, 0.45),
                (EffectKind::Radiation, 0.30),
                (EffectKind::Pressure, 0.80),
                (EffectKind::Chemical, 0.0),
                (EffectKind::Acoustic, 0.25),
            ]),
            heat_per_absorbed: 0.35,
            cooling_rate: 4.0,
            derate_start: 60.0,
            shutdown_heat: 100.0,
            base_signature: 0.35,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActiveField {
    config: ActiveFieldConfig,
    charge: f64,
    heat: f64,
    enabled: bool,
}

impl ActiveField {
    pub fn new(config: ActiveFieldConfig) -> Result<Self, FieldError> {
        config.validate()?;
        Ok(Self {
            charge: config.max_charge,
            heat: 0.0,
            enabled: true,
            config,
        })
    }

    pub fn charge(&self) -> f64 {
        self.charge
    }

    pub fn charge_fraction(&self) -> f64 {
        (self.charge / self.config.max_charge).clamp(0.0, 1.0)
    }

    pub fn heat(&self) -> f64 {
        self.heat
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn thermal_scale(&self) -> f64 {
        if self.heat <= self.config.derate_start {
            1.0
        } else if self.heat >= self.config.shutdown_heat {
            0.0
        } else {
            let span = self.config.shutdown_heat - self.config.derate_start;
            1.0 - (self.heat - self.config.derate_start) / span
        }
    }

    /// Abstract detectable operating signature. Disabled fields are silent in v0.1;
    /// active fields become more conspicuous while hot and while intercepting/recharging.
    pub fn signature(&self) -> f64 {
        if !self.enabled {
            return 0.0;
        }
        let thermal = (self.heat / self.config.shutdown_heat).clamp(0.0, 1.0);
        (self.config.base_signature + 0.40 * thermal).clamp(0.0, 1.0)
    }

    /// Intercept one incoming effect using the single shared charge pool.
    pub fn intercept(&mut self, effect: Effect) -> Result<FieldResolution, FieldError> {
        effect.validate().map_err(FieldError::Protection)?;
        let incoming = effect.magnitude;
        if !self.enabled || self.charge <= 0.0 || self.thermal_scale() <= 0.0 {
            return Ok(FieldResolution {
                original: effect,
                residual: effect,
                absorbed: 0.0,
                charge_before: self.charge,
                charge_after: self.charge,
                heat_after: self.heat,
                signature_after: self.signature(),
            });
        }

        let fraction = self
            .config
            .absorption
            .get(&effect.kind)
            .copied()
            .unwrap_or(0.0)
            * self.thermal_scale();
        let charge_before = self.charge;
        let attempted = incoming * fraction;
        let absorbed = attempted.min(self.charge);
        self.charge -= absorbed;
        self.heat += absorbed * self.config.heat_per_absorbed;

        Ok(FieldResolution {
            original: effect,
            residual: Effect {
                magnitude: (incoming - absorbed).max(0.0),
                kind: effect.kind,
            },
            absorbed,
            charge_before,
            charge_after: self.charge,
            heat_after: self.heat,
            signature_after: self.signature(),
        })
    }

    /// Advance cooling and recharge from a caller-supplied resource budget.
    ///
    /// The caller owns the actual energy/power authority. `supplied_resource` is
    /// merely the amount granted to this fictional field for this step; the field
    /// cannot withdraw from an exoframe battery by itself.
    pub fn service(&mut self, supplied_resource: f64, dt: f64) -> Result<FieldService, FieldError> {
        if !supplied_resource.is_finite() || supplied_resource < 0.0 {
            return Err(FieldError::InvalidInput("supplied_resource"));
        }
        if !dt.is_finite() || dt <= 0.0 {
            return Err(FieldError::InvalidInput("dt"));
        }

        self.heat = (self.heat - self.config.cooling_rate * dt).max(0.0);
        let charge_before = self.charge;
        let thermal_scale = self.thermal_scale();
        let max_input = self.config.max_recharge_rate * dt;
        let accepted_input = if self.enabled {
            supplied_resource.min(max_input)
        } else {
            0.0
        };
        let stored = accepted_input * self.config.recharge_efficiency * thermal_scale;
        self.charge = (self.charge + stored).min(self.config.max_charge);

        Ok(FieldService {
            supplied_resource,
            accepted_resource: accepted_input,
            stored_charge: self.charge - charge_before,
            charge_after: self.charge,
            heat_after: self.heat,
            signature_after: self.signature(),
        })
    }
}

impl Default for ActiveField {
    fn default() -> Self {
        Self::new(ActiveFieldConfig::default()).expect("default active-field config is valid")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct FieldResolution {
    pub original: Effect,
    pub residual: Effect,
    pub absorbed: f64,
    pub charge_before: f64,
    pub charge_after: f64,
    pub heat_after: f64,
    pub signature_after: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct FieldService {
    pub supplied_resource: f64,
    pub accepted_resource: f64,
    pub stored_charge: f64,
    pub charge_after: f64,
    pub heat_after: f64,
    pub signature_after: f64,
}

#[derive(Debug)]
pub enum FieldError {
    InvalidConfig(&'static str),
    InvalidInput(&'static str),
    Protection(ProtectionError),
}

impl std::fmt::Display for FieldError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidConfig(field) => write!(f, "invalid active-field config: {field}"),
            Self::InvalidInput(field) => write!(f, "invalid active-field input: {field}"),
            Self::Protection(error) => write!(f, "invalid incoming effect: {error}"),
        }
    }
}

impl std::error::Error for FieldError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_supported_effects_share_one_charge_pool() {
        let mut field = ActiveField::default();
        let start = field.charge();
        let impact = field
            .intercept(Effect {
                magnitude: 20.0,
                kind: EffectKind::Impact,
            })
            .unwrap();
        let thermal = field
            .intercept(Effect {
                magnitude: 20.0,
                kind: EffectKind::Thermal,
            })
            .unwrap();
        assert_eq!(field.charge(), start - impact.absorbed - thermal.absorbed);
    }

    #[test]
    fn field_can_break_and_residual_passes_inward() {
        let config = ActiveFieldConfig {
            max_charge: 5.0,
            ..Default::default()
        };
        let mut field = ActiveField::new(config).unwrap();
        let result = field
            .intercept(Effect {
                magnitude: 20.0,
                kind: EffectKind::Impact,
            })
            .unwrap();
        assert_eq!(result.absorbed, 5.0);
        assert_eq!(result.residual.magnitude, 15.0);
        assert_eq!(field.charge(), 0.0);
    }

    #[test]
    fn disabled_field_does_not_absorb_or_emit_signature() {
        let mut field = ActiveField::default();
        field.set_enabled(false);
        let result = field
            .intercept(Effect {
                magnitude: 10.0,
                kind: EffectKind::Impact,
            })
            .unwrap();
        assert_eq!(result.absorbed, 0.0);
        assert_eq!(result.residual.magnitude, 10.0);
        assert_eq!(field.signature(), 0.0);
    }

    #[test]
    fn recharge_cannot_create_more_than_supplied_budget_allows() {
        let mut field = ActiveField::default();
        field
            .intercept(Effect {
                magnitude: 50.0,
                kind: EffectKind::Impact,
            })
            .unwrap();
        let before = field.charge();
        let service = field.service(4.0, 1.0).unwrap();
        assert!(service.accepted_resource <= 4.0);
        assert!(field.charge() - before <= 4.0);
    }

    #[test]
    fn sustained_absorption_heats_and_derates_field() {
        let config = ActiveFieldConfig {
            max_charge: 10_000.0,
            heat_per_absorbed: 2.0,
            derate_start: 10.0,
            shutdown_heat: 20.0,
            ..Default::default()
        };
        let mut field = ActiveField::new(config).unwrap();
        field
            .intercept(Effect {
                magnitude: 8.0,
                kind: EffectKind::Impact,
            })
            .unwrap();
        assert!(field.heat() > 10.0);
        assert!(field.thermal_scale() < 1.0);
    }

    #[test]
    fn identical_sequence_replays_identically() {
        let mut a = ActiveField::default();
        let mut b = ActiveField::default();
        for i in 0..30 {
            let effect = Effect {
                magnitude: (i + 1) as f64,
                kind: if i % 2 == 0 {
                    EffectKind::Impact
                } else {
                    EffectKind::Thermal
                },
            };
            assert_eq!(a.intercept(effect).unwrap(), b.intercept(effect).unwrap());
            assert_eq!(a.service(2.0, 0.1).unwrap(), b.service(2.0, 0.1).unwrap());
        }
        assert_eq!(a, b);
    }
}
