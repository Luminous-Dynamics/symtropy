// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Deterministic subsystem resource arbitration for fictional Symtropy powered frames.
//!
//! This crate owns game-world allocation policy, not physical battery engineering.
//! An owning integration supplies an already-available resource budget for a tick.
//! Subsystems request portions of that budget; the arbiter grants no more than exists
//! and returns an auditable allocation receipt. The same semantics can later consume
//! a qualified Symthaea power budget without making this crate authoritative for it.

use serde::{Deserialize, Serialize};
use std::{collections::{BTreeMap, BTreeSet}, error::Error, fmt};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum PoweredSubsystem {
    LifeSupport,
    Cooling,
    Mobility,
    Communications,
    Sensors,
    Protection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResourceMode {
    Balanced,
    MobilityPriority,
    ProtectionPriority,
    LowSignature,
    Emergency,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ResourceRequest {
    pub subsystem: PoweredSubsystem,
    /// Desired resource for this arbitration interval.
    pub requested: f64,
    /// Minimum useful/safe amount requested by this subsystem.
    /// A request is still not a guarantee when the shared budget is insufficient.
    pub minimum: f64,
}

impl ResourceRequest {
    fn validate(&self) -> Result<(), ArbitrationError> {
        if !self.requested.is_finite() || self.requested < 0.0 {
            return Err(ArbitrationError::InvalidRequest(self.subsystem));
        }
        if !self.minimum.is_finite() || self.minimum < 0.0 || self.minimum > self.requested {
            return Err(ArbitrationError::InvalidMinimum(self.subsystem));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ResourceGrant {
    pub subsystem: PoweredSubsystem,
    pub requested: f64,
    pub minimum: f64,
    pub granted: f64,
}

impl ResourceGrant {
    pub fn request_fraction(&self) -> f64 {
        if self.requested == 0.0 {
            1.0
        } else {
            (self.granted / self.requested).clamp(0.0, 1.0)
        }
    }

    pub fn minimum_satisfied(&self) -> bool {
        self.granted + f64::EPSILON >= self.minimum
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArbitrationReceipt {
    pub mode: ResourceMode,
    pub available: f64,
    pub allocated: f64,
    pub unallocated: f64,
    pub grants: BTreeMap<PoweredSubsystem, ResourceGrant>,
}

impl ArbitrationReceipt {
    pub fn grant(&self, subsystem: PoweredSubsystem) -> Option<&ResourceGrant> {
        self.grants.get(&subsystem)
    }

    pub fn unmet_minimum_total(&self) -> f64 {
        self.grants
            .values()
            .map(|grant| (grant.minimum - grant.granted).max(0.0))
            .sum()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ResourceArbiter {
    pub mode: ResourceMode,
}

impl ResourceArbiter {
    pub fn new(mode: ResourceMode) -> Self {
        Self { mode }
    }

    /// Allocate a finite caller-owned resource budget.
    ///
    /// Minimum requests are serviced first in a fixed safety-oriented order. If
    /// resource remains, unmet desired demand receives deterministic weighted-share
    /// allocation according to the selected operating mode. No subsystem can receive
    /// more than it requested and total grants can never exceed `available`.
    pub fn allocate(
        &self,
        available: f64,
        requests: &[ResourceRequest],
    ) -> Result<ArbitrationReceipt, ArbitrationError> {
        if !available.is_finite() || available < 0.0 {
            return Err(ArbitrationError::InvalidAvailable);
        }

        let mut seen = BTreeSet::new();
        let mut grants = BTreeMap::new();
        for request in requests {
            request.validate()?;
            if !seen.insert(request.subsystem) {
                return Err(ArbitrationError::DuplicateSubsystem(request.subsystem));
            }
            grants.insert(
                request.subsystem,
                ResourceGrant {
                    subsystem: request.subsystem,
                    requested: request.requested,
                    minimum: request.minimum,
                    granted: 0.0,
                },
            );
        }

        let mut remaining = available;
        // Safety-oriented minimum-service order. This does not assert that every
        // world has life support; absent subsystems simply have no request.
        let minimum_order = [
            PoweredSubsystem::LifeSupport,
            PoweredSubsystem::Cooling,
            PoweredSubsystem::Mobility,
            PoweredSubsystem::Communications,
            PoweredSubsystem::Sensors,
            PoweredSubsystem::Protection,
        ];
        for subsystem in minimum_order {
            let Some(grant) = grants.get_mut(&subsystem) else {
                continue;
            };
            let amount = grant.minimum.min(remaining);
            grant.granted += amount;
            remaining -= amount;
            if remaining <= f64::EPSILON {
                remaining = 0.0;
                break;
            }
        }

        // Weighted water-filling for desired demand. Iterate at most once per
        // subsystem plus one because each capped participant can only leave the
        // eligible set once.
        for _ in 0..=grants.len() {
            if remaining <= f64::EPSILON {
                remaining = 0.0;
                break;
            }
            let eligible: Vec<(PoweredSubsystem, f64, f64)> = grants
                .iter()
                .filter_map(|(subsystem, grant)| {
                    let unmet = (grant.requested - grant.granted).max(0.0);
                    if unmet <= f64::EPSILON {
                        None
                    } else {
                        Some((*subsystem, unmet, weight(self.mode, *subsystem)))
                    }
                })
                .collect();
            if eligible.is_empty() {
                break;
            }
            let total_weight: f64 = eligible.iter().map(|(_, _, w)| *w).sum();
            if total_weight <= 0.0 {
                break;
            }

            let round_budget = remaining;
            let mut consumed = 0.0;
            for (subsystem, unmet, subsystem_weight) in eligible {
                let share = round_budget * subsystem_weight / total_weight;
                let amount = share.min(unmet).min(remaining);
                if let Some(grant) = grants.get_mut(&subsystem) {
                    grant.granted += amount;
                }
                consumed += amount;
                remaining -= amount;
            }
            if consumed <= f64::EPSILON {
                break;
            }
        }

        let allocated: f64 = grants.values().map(|grant| grant.granted).sum();
        // Recompute from authoritative totals to avoid accumulating tiny subtraction
        // drift into an apparently negative unallocated amount.
        let unallocated = (available - allocated).max(0.0);

        Ok(ArbitrationReceipt {
            mode: self.mode,
            available,
            allocated,
            unallocated,
            grants,
        })
    }
}

impl Default for ResourceArbiter {
    fn default() -> Self {
        Self::new(ResourceMode::Balanced)
    }
}

fn weight(mode: ResourceMode, subsystem: PoweredSubsystem) -> f64 {
    use PoweredSubsystem::*;
    use ResourceMode::*;
    match mode {
        Balanced => match subsystem {
            LifeSupport => 2.0,
            Cooling => 1.5,
            Mobility | Communications | Sensors | Protection => 1.0,
        },
        MobilityPriority => match subsystem {
            LifeSupport => 2.0,
            Cooling => 1.5,
            Mobility => 4.0,
            Communications | Sensors => 1.0,
            Protection => 0.75,
        },
        ProtectionPriority => match subsystem {
            LifeSupport => 2.0,
            Cooling => 2.0,
            Protection => 4.0,
            Mobility => 1.5,
            Communications | Sensors => 0.75,
        },
        LowSignature => match subsystem {
            LifeSupport => 3.0,
            Cooling => 2.0,
            Mobility => 1.5,
            Sensors => 1.0,
            Communications => 0.4,
            Protection => 0.25,
        },
        Emergency => match subsystem {
            LifeSupport => 5.0,
            Cooling => 3.0,
            Mobility => 3.0,
            Communications => 2.0,
            Sensors => 1.0,
            Protection => 0.5,
        },
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArbitrationError {
    InvalidAvailable,
    InvalidRequest(PoweredSubsystem),
    InvalidMinimum(PoweredSubsystem),
    DuplicateSubsystem(PoweredSubsystem),
}

impl fmt::Display for ArbitrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidAvailable => write!(f, "available resource must be finite and non-negative"),
            Self::InvalidRequest(subsystem) => write!(f, "invalid request for {subsystem:?}"),
            Self::InvalidMinimum(subsystem) => write!(f, "invalid minimum for {subsystem:?}"),
            Self::DuplicateSubsystem(subsystem) => write!(f, "duplicate request for {subsystem:?}"),
        }
    }
}

impl Error for ArbitrationError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(subsystem: PoweredSubsystem, requested: f64, minimum: f64) -> ResourceRequest {
        ResourceRequest { subsystem, requested, minimum }
    }

    #[test]
    fn never_allocates_more_than_exists() {
        let receipt = ResourceArbiter::default()
            .allocate(
                10.0,
                &[
                    request(PoweredSubsystem::Mobility, 20.0, 2.0),
                    request(PoweredSubsystem::Protection, 20.0, 2.0),
                    request(PoweredSubsystem::Sensors, 20.0, 1.0),
                ],
            )
            .unwrap();
        assert!(receipt.allocated <= 10.0 + 1e-12);
        assert!(receipt.unallocated >= 0.0);
    }

    #[test]
    fn life_support_minimum_wins_first_under_severe_shortage() {
        let receipt = ResourceArbiter::default()
            .allocate(
                3.0,
                &[
                    request(PoweredSubsystem::Protection, 10.0, 2.0),
                    request(PoweredSubsystem::LifeSupport, 10.0, 3.0),
                ],
            )
            .unwrap();
        assert_eq!(receipt.grant(PoweredSubsystem::LifeSupport).unwrap().granted, 3.0);
        assert_eq!(receipt.grant(PoweredSubsystem::Protection).unwrap().granted, 0.0);
        assert!(receipt.unmet_minimum_total() > 0.0);
    }

    #[test]
    fn protection_mode_gives_protection_more_surplus_than_mobility() {
        let receipt = ResourceArbiter::new(ResourceMode::ProtectionPriority)
            .allocate(
                10.0,
                &[
                    request(PoweredSubsystem::Mobility, 10.0, 0.0),
                    request(PoweredSubsystem::Protection, 10.0, 0.0),
                ],
            )
            .unwrap();
        assert!(
            receipt.grant(PoweredSubsystem::Protection).unwrap().granted
                > receipt.grant(PoweredSubsystem::Mobility).unwrap().granted
        );
    }

    #[test]
    fn low_signature_deprioritizes_field_and_comms_surplus() {
        let receipt = ResourceArbiter::new(ResourceMode::LowSignature)
            .allocate(
                10.0,
                &[
                    request(PoweredSubsystem::Mobility, 10.0, 0.0),
                    request(PoweredSubsystem::Protection, 10.0, 0.0),
                    request(PoweredSubsystem::Communications, 10.0, 0.0),
                ],
            )
            .unwrap();
        let mobility = receipt.grant(PoweredSubsystem::Mobility).unwrap().granted;
        let protection = receipt.grant(PoweredSubsystem::Protection).unwrap().granted;
        let comms = receipt.grant(PoweredSubsystem::Communications).unwrap().granted;
        assert!(mobility > comms);
        assert!(comms > protection);
    }

    #[test]
    fn duplicate_subsystems_fail_closed() {
        assert!(matches!(
            ResourceArbiter::default().allocate(
                10.0,
                &[
                    request(PoweredSubsystem::Sensors, 2.0, 1.0),
                    request(PoweredSubsystem::Sensors, 3.0, 1.0),
                ],
            ),
            Err(ArbitrationError::DuplicateSubsystem(PoweredSubsystem::Sensors))
        ));
    }

    #[test]
    fn input_order_does_not_change_receipt() {
        let arbiter = ResourceArbiter::new(ResourceMode::Balanced);
        let a = arbiter
            .allocate(
                17.0,
                &[
                    request(PoweredSubsystem::Protection, 10.0, 1.0),
                    request(PoweredSubsystem::Mobility, 12.0, 2.0),
                    request(PoweredSubsystem::Cooling, 5.0, 1.0),
                ],
            )
            .unwrap();
        let b = arbiter
            .allocate(
                17.0,
                &[
                    request(PoweredSubsystem::Cooling, 5.0, 1.0),
                    request(PoweredSubsystem::Mobility, 12.0, 2.0),
                    request(PoweredSubsystem::Protection, 10.0, 1.0),
                ],
            )
            .unwrap();
        assert_eq!(a, b);
    }
}
