// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Single-spend settlement for powered-frame resource grants.
//!
//! Arbitration decides *how much* each subsystem may receive. Settlement prevents an
//! integration from applying the same grant repeatedly or to the wrong subsystem.
//! The owning simulation remains authoritative for the upstream energy source.

use crate::{ArbitrationReceipt, PoweredSubsystem};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, error::Error, fmt};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ResourceEpoch {
    /// Stable name of the caller-owned source/budget authority.
    pub source: String,
    /// Authoritative simulation tick for this budget.
    pub simulation_tick: u64,
    /// Monotonic source-local generation for multiple budgets at one tick.
    pub generation: u64,
}

impl ResourceEpoch {
    pub fn new(
        source: impl Into<String>,
        simulation_tick: u64,
        generation: u64,
    ) -> Result<Self, SettlementError> {
        let source = source.into();
        if source.trim().is_empty() {
            return Err(SettlementError::EmptyEpochSource);
        }
        Ok(Self {
            source,
            simulation_tick,
            generation,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConsumptionReceipt {
    pub consumption_id: String,
    pub epoch: ResourceEpoch,
    pub subsystem: PoweredSubsystem,
    pub amount: f64,
    pub remaining_before: f64,
    pub remaining_after: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResourceSettlement {
    pub epoch: ResourceEpoch,
    pub allocation: ArbitrationReceipt,
    remaining: BTreeMap<PoweredSubsystem, f64>,
    consumptions: BTreeMap<String, ConsumptionReceipt>,
}

impl ResourceSettlement {
    pub fn new(epoch: ResourceEpoch, allocation: ArbitrationReceipt) -> Self {
        let remaining = allocation
            .grants
            .iter()
            .map(|(subsystem, grant)| (*subsystem, grant.granted))
            .collect();
        Self {
            epoch,
            allocation,
            remaining,
            consumptions: BTreeMap::new(),
        }
    }

    pub fn remaining(&self, subsystem: PoweredSubsystem) -> f64 {
        self.remaining.get(&subsystem).copied().unwrap_or(0.0)
    }

    pub fn consumption(&self, consumption_id: &str) -> Option<&ConsumptionReceipt> {
        self.consumptions.get(consumption_id)
    }

    /// Spend part of one subsystem grant exactly once per consumption identity.
    ///
    /// Repeating the same `consumption_id`, subsystem, and amount is idempotent and
    /// returns the original receipt without spending again. Reusing an identity for a
    /// different request fails closed.
    pub fn consume(
        &mut self,
        consumption_id: impl Into<String>,
        subsystem: PoweredSubsystem,
        amount: f64,
    ) -> Result<ConsumptionReceipt, SettlementError> {
        let consumption_id = consumption_id.into();
        if consumption_id.trim().is_empty() {
            return Err(SettlementError::EmptyConsumptionId);
        }
        if !amount.is_finite() || amount < 0.0 {
            return Err(SettlementError::InvalidAmount(amount));
        }

        if let Some(existing) = self.consumptions.get(&consumption_id) {
            if existing.subsystem == subsystem && existing.amount == amount {
                return Ok(existing.clone());
            }
            return Err(SettlementError::ConsumptionIdCollision(consumption_id));
        }

        if !self.allocation.grants.contains_key(&subsystem) {
            return Err(SettlementError::UnknownSubsystem(subsystem));
        }

        let remaining_before = self.remaining(subsystem);
        if amount > remaining_before + 1e-12 {
            return Err(SettlementError::GrantExceeded {
                subsystem,
                requested: amount,
                remaining: remaining_before,
            });
        }

        let remaining_after = (remaining_before - amount).max(0.0);
        self.remaining.insert(subsystem, remaining_after);
        let receipt = ConsumptionReceipt {
            consumption_id: consumption_id.clone(),
            epoch: self.epoch.clone(),
            subsystem,
            amount,
            remaining_before,
            remaining_after,
        };
        self.consumptions.insert(consumption_id, receipt.clone());
        Ok(receipt)
    }

    pub fn total_remaining(&self) -> f64 {
        self.remaining.values().copied().sum()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SettlementError {
    EmptyEpochSource,
    EmptyConsumptionId,
    InvalidAmount(f64),
    UnknownSubsystem(PoweredSubsystem),
    GrantExceeded {
        subsystem: PoweredSubsystem,
        requested: f64,
        remaining: f64,
    },
    ConsumptionIdCollision(String),
}

impl fmt::Display for SettlementError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyEpochSource => write!(f, "resource epoch source must not be empty"),
            Self::EmptyConsumptionId => write!(f, "consumption id must not be empty"),
            Self::InvalidAmount(amount) => {
                write!(f, "consumption amount must be finite and non-negative, got {amount}")
            }
            Self::UnknownSubsystem(subsystem) => {
                write!(f, "no allocation grant exists for {subsystem:?}")
            }
            Self::GrantExceeded {
                subsystem,
                requested,
                remaining,
            } => write!(
                f,
                "consumption for {subsystem:?} requests {requested}, but only {remaining} remains"
            ),
            Self::ConsumptionIdCollision(id) => {
                write!(f, "consumption id {id} was reused with different semantics")
            }
        }
    }
}

impl Error for SettlementError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ResourceArbiter, ResourceMode, ResourceRequest};

    fn settlement() -> ResourceSettlement {
        let allocation = ResourceArbiter::new(ResourceMode::Balanced)
            .allocate(
                10.0,
                &[
                    ResourceRequest {
                        subsystem: PoweredSubsystem::Mobility,
                        requested: 6.0,
                        minimum: 2.0,
                    },
                    ResourceRequest {
                        subsystem: PoweredSubsystem::Protection,
                        requested: 6.0,
                        minimum: 0.0,
                    },
                ],
            )
            .unwrap();
        ResourceSettlement::new(ResourceEpoch::new("pack:alpha", 42, 0).unwrap(), allocation)
    }

    #[test]
    fn identical_retry_is_idempotent() {
        let mut settlement = settlement();
        let first = settlement
            .consume("field-recharge:42", PoweredSubsystem::Protection, 1.0)
            .unwrap();
        let after_first = settlement.remaining(PoweredSubsystem::Protection);
        let retry = settlement
            .consume("field-recharge:42", PoweredSubsystem::Protection, 1.0)
            .unwrap();
        assert_eq!(first, retry);
        assert_eq!(settlement.remaining(PoweredSubsystem::Protection), after_first);
    }

    #[test]
    fn grant_cannot_be_double_spent_under_new_ids() {
        let mut settlement = settlement();
        let granted = settlement.remaining(PoweredSubsystem::Protection);
        settlement
            .consume("first", PoweredSubsystem::Protection, granted)
            .unwrap();
        assert!(matches!(
            settlement.consume("second", PoweredSubsystem::Protection, 0.1),
            Err(SettlementError::GrantExceeded { .. })
        ));
    }

    #[test]
    fn consumption_identity_cannot_change_meaning() {
        let mut settlement = settlement();
        settlement
            .consume("same", PoweredSubsystem::Protection, 1.0)
            .unwrap();
        assert!(matches!(
            settlement.consume("same", PoweredSubsystem::Mobility, 1.0),
            Err(SettlementError::ConsumptionIdCollision(_))
        ));
    }

    #[test]
    fn cannot_spend_an_ungranted_subsystem() {
        let mut settlement = settlement();
        assert!(matches!(
            settlement.consume("sensor", PoweredSubsystem::Sensors, 0.0),
            Err(SettlementError::UnknownSubsystem(PoweredSubsystem::Sensors))
        ));
    }

    #[test]
    fn settlement_replays_deterministically() {
        let mut a = settlement();
        let mut b = settlement();
        for (id, subsystem, amount) in [
            ("m1", PoweredSubsystem::Mobility, 1.0),
            ("p1", PoweredSubsystem::Protection, 1.0),
            ("m2", PoweredSubsystem::Mobility, 0.5),
        ] {
            assert_eq!(
                a.consume(id, subsystem, amount),
                b.consume(id, subsystem, amount)
            );
        }
        assert_eq!(a, b);
    }
}
