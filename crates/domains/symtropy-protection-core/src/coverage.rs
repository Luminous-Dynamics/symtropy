// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Localized and directional coverage for generic protection.
//!
//! Geometry/physics owns the question "where did the effect arrive from and what
//! region did it contact?" This module receives that resolved contact and applies
//! only protection layers whose coverage contains it. It does not perform raycasts,
//! collision detection, anatomy, or targeting.

use crate::{
    Effect, LayerReceipt, ProtectionError, ProtectionLayer, ProtectionResolution,
    active_field::{ActiveField, FieldError, FieldResolution},
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, error::Error, fmt};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ApproachSector {
    Front,
    Rear,
    Left,
    Right,
    Above,
    Below,
}

/// Stable semantic region selected by an owning body/vehicle/structure model.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProtectionRegion(String);

impl ProtectionRegion {
    pub fn parse(value: impl Into<String>) -> Result<Self, CoverageError> {
        let value = value.into();
        let valid = !value.is_empty()
            && value.len() <= 96
            && value.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_' | b':')
            });
        if valid {
            Ok(Self(value))
        } else {
            Err(CoverageError::InvalidRegion(value))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtectionContact {
    pub region: ProtectionRegion,
    pub approach: ApproachSector,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoverageMask {
    /// Empty means all semantic regions.
    pub regions: BTreeSet<ProtectionRegion>,
    /// Empty means all approach sectors.
    pub sectors: BTreeSet<ApproachSector>,
}

impl CoverageMask {
    pub fn all() -> Self {
        Self {
            regions: BTreeSet::new(),
            sectors: BTreeSet::new(),
        }
    }

    pub fn covers(&self, contact: &ProtectionContact) -> bool {
        (self.regions.is_empty() || self.regions.contains(&contact.region))
            && (self.sectors.is_empty() || self.sectors.contains(&contact.approach))
    }
}

impl Default for CoverageMask {
    fn default() -> Self {
        Self::all()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LocalizedLayer {
    pub coverage: CoverageMask,
    pub layer: ProtectionLayer,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LocalizedLayerReceipt {
    pub layer_id: String,
    pub covered: bool,
    pub receipt: Option<LayerReceipt>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LocalizedProtectionResolution {
    pub contact: ProtectionContact,
    pub resolution: ProtectionResolution,
    pub coverage: Vec<LocalizedLayerReceipt>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct LocalizedProtectionStack {
    /// Ordered outermost -> innermost.
    pub layers: Vec<LocalizedLayer>,
}

impl LocalizedProtectionStack {
    pub fn resolve(
        &mut self,
        contact: ProtectionContact,
        effect: Effect,
    ) -> Result<LocalizedProtectionResolution, CoverageError> {
        effect.validate().map_err(CoverageError::Protection)?;
        let mut residual = effect;
        let mut applied = Vec::new();
        let mut coverage = Vec::with_capacity(self.layers.len());

        for localized in &mut self.layers {
            let covered = localized.coverage.covers(&contact);
            if !covered {
                coverage.push(LocalizedLayerReceipt {
                    layer_id: localized.layer.id.clone(),
                    covered: false,
                    receipt: None,
                });
                continue;
            }

            let receipt = localized
                .layer
                .apply(residual)
                .map_err(CoverageError::Protection)?;
            residual.magnitude = receipt.transmitted;
            applied.push(receipt.clone());
            coverage.push(LocalizedLayerReceipt {
                layer_id: localized.layer.id.clone(),
                covered: true,
                receipt: Some(receipt),
            });
            if residual.magnitude == 0.0 {
                break;
            }
        }

        Ok(LocalizedProtectionResolution {
            contact,
            resolution: ProtectionResolution {
                original: effect,
                residual,
                layers: applied,
            },
            coverage,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DirectionalActiveField {
    pub coverage: CoverageMask,
    pub field: ActiveField,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DirectionalFieldResolution {
    /// Contact lies outside field coverage; field state is unchanged.
    Bypassed {
        contact: ProtectionContact,
        residual: Effect,
    },
    Intercepted {
        contact: ProtectionContact,
        resolution: FieldResolution,
    },
}

impl DirectionalActiveField {
    pub fn intercept(
        &mut self,
        contact: ProtectionContact,
        effect: Effect,
    ) -> Result<DirectionalFieldResolution, CoverageError> {
        effect.validate().map_err(CoverageError::Protection)?;
        if !self.coverage.covers(&contact) {
            return Ok(DirectionalFieldResolution::Bypassed {
                contact,
                residual: effect,
            });
        }
        let resolution = self.field.intercept(effect).map_err(CoverageError::Field)?;
        Ok(DirectionalFieldResolution::Intercepted {
            contact,
            resolution,
        })
    }
}

#[derive(Debug)]
pub enum CoverageError {
    InvalidRegion(String),
    Protection(ProtectionError),
    Field(FieldError),
}

impl fmt::Display for CoverageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRegion(value) => write!(f, "invalid protection region {value:?}"),
            Self::Protection(error) => error.fmt(f),
            Self::Field(error) => error.fmt(f),
        }
    }
}

impl Error for CoverageError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EffectKind, LayerResponse};
    use std::collections::BTreeMap;

    fn region(value: &str) -> ProtectionRegion {
        ProtectionRegion::parse(value).unwrap()
    }

    fn armor(id: &str, capacity: f64) -> ProtectionLayer {
        ProtectionLayer::new(
            id,
            1.0,
            BTreeMap::from([(
                EffectKind::Impact,
                LayerResponse {
                    absorption_fraction: 1.0,
                    capacity,
                },
            )]),
        )
        .unwrap()
    }

    #[test]
    fn uncovered_contact_does_not_consume_layer_capacity() {
        let chest = region("body:chest");
        let leg = region("body:left-leg");
        let mut stack = LocalizedProtectionStack {
            layers: vec![LocalizedLayer {
                coverage: CoverageMask {
                    regions: BTreeSet::from([chest]),
                    sectors: BTreeSet::new(),
                },
                layer: armor("chest-plate", 20.0),
            }],
        };
        let result = stack
            .resolve(
                ProtectionContact {
                    region: leg,
                    approach: ApproachSector::Front,
                },
                Effect {
                    magnitude: 10.0,
                    kind: EffectKind::Impact,
                },
            )
            .unwrap();
        assert_eq!(result.resolution.residual.magnitude, 10.0);
        assert_eq!(stack.layers[0].layer.remaining_capacity(EffectKind::Impact), 20.0);
        assert!(!result.coverage[0].covered);
    }

    #[test]
    fn directional_field_can_protect_front_without_protecting_rear() {
        let wearer = region("body:whole");
        let mut field = DirectionalActiveField {
            coverage: CoverageMask {
                regions: BTreeSet::new(),
                sectors: BTreeSet::from([ApproachSector::Front]),
            },
            field: ActiveField::default(),
        };
        let start = field.field.charge();
        let rear = field
            .intercept(
                ProtectionContact {
                    region: wearer.clone(),
                    approach: ApproachSector::Rear,
                },
                Effect {
                    magnitude: 10.0,
                    kind: EffectKind::Impact,
                },
            )
            .unwrap();
        assert!(matches!(rear, DirectionalFieldResolution::Bypassed { .. }));
        assert_eq!(field.field.charge(), start);

        let front = field
            .intercept(
                ProtectionContact {
                    region: wearer,
                    approach: ApproachSector::Front,
                },
                Effect {
                    magnitude: 10.0,
                    kind: EffectKind::Impact,
                },
            )
            .unwrap();
        assert!(matches!(front, DirectionalFieldResolution::Intercepted { .. }));
        assert!(field.field.charge() < start);
    }

    #[test]
    fn outer_to_inner_order_is_preserved_for_covered_layers() {
        let chest = region("body:chest");
        let contact = ProtectionContact {
            region: chest.clone(),
            approach: ApproachSector::Front,
        };
        let mut stack = LocalizedProtectionStack {
            layers: vec![
                LocalizedLayer {
                    coverage: CoverageMask {
                        regions: BTreeSet::from([chest.clone()]),
                        sectors: BTreeSet::new(),
                    },
                    layer: armor("outer", 5.0),
                },
                LocalizedLayer {
                    coverage: CoverageMask {
                        regions: BTreeSet::from([chest]),
                        sectors: BTreeSet::new(),
                    },
                    layer: armor("inner", 5.0),
                },
            ],
        };
        let result = stack
            .resolve(
                contact,
                Effect {
                    magnitude: 12.0,
                    kind: EffectKind::Impact,
                },
            )
            .unwrap();
        assert_eq!(result.resolution.layers[0].layer_id, "outer");
        assert_eq!(result.resolution.layers[1].layer_id, "inner");
        assert_eq!(result.resolution.residual.magnitude, 2.0);
    }
}
