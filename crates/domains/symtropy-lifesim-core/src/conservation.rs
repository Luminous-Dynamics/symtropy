// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Typed ecological conservation primitives.
//!
//! This module does not pretend that water, carbon, biomass, and mineral
//! nutrients are mutually interchangeable. Instead it tracks one conserved
//! quantity at a time as that quantity moves between ecological compartments.
//! Cross-quantity reactions (for example, biochemical conversion) must remain
//! explicit at the domain layer rather than being hidden inside an untyped
//! "mass" scalar.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

/// Conserved quantity tracked by the ecological ledger.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ConservedQuantity {
    BiomassMass,
    CarbonMass,
    WaterMass,
    MineralNutrientMass,
}

/// Internal ecological compartment holding a conserved quantity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EcologicalCompartment {
    Living,
    Detritus,
    Soil,
    WaterColumn,
    Atmosphere,
    Infrastructure,
    Storage,
}

/// Key for one quantity held in one internal compartment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PoolKey {
    pub quantity: ConservedQuantity,
    pub compartment: EcologicalCompartment,
}

impl PoolKey {
    pub const fn new(
        quantity: ConservedQuantity,
        compartment: EcologicalCompartment,
    ) -> Self {
        Self {
            quantity,
            compartment,
        }
    }
}

/// Deterministic accounting state for ecological transfers.
///
/// External inputs and outputs are tracked separately so conservation can be
/// checked over an open-system interval without creating a fake "external"
/// internal pool.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct EcologicalLedger {
    pools: BTreeMap<PoolKey, f64>,
    external_in: BTreeMap<ConservedQuantity, f64>,
    external_out: BTreeMap<ConservedQuantity, f64>,
}

impl EcologicalLedger {
    /// Read one internal pool. Uninitialized pools are exactly zero.
    pub fn amount(&self, key: PoolKey) -> f64 {
        self.pools.get(&key).copied().unwrap_or(0.0)
    }

    /// Sum one conserved quantity over every internal compartment.
    pub fn total(&self, quantity: ConservedQuantity) -> f64 {
        self.pools
            .iter()
            .filter_map(|(key, amount)| (key.quantity == quantity).then_some(*amount))
            .sum()
    }

    /// Cumulative external input recorded for one quantity.
    pub fn external_input(&self, quantity: ConservedQuantity) -> f64 {
        self.external_in.get(&quantity).copied().unwrap_or(0.0)
    }

    /// Cumulative external output recorded for one quantity.
    pub fn external_output(&self, quantity: ConservedQuantity) -> f64 {
        self.external_out.get(&quantity).copied().unwrap_or(0.0)
    }

    /// Seed an internal pool for initial-state construction.
    ///
    /// This is intentionally distinct from runtime external input. Callers
    /// constructing a baseline may seed arbitrary non-negative finite values,
    /// then clone the ledger and use [`Self::balance_since`] for subsequent
    /// qualification.
    pub fn seed(&mut self, key: PoolKey, amount: f64) -> Result<(), ConservationError> {
        validate_amount(amount)?;
        self.pools.insert(key, amount);
        Ok(())
    }

    /// Move one conserved quantity between internal compartments.
    ///
    /// Failure is atomic: insufficient source quantity or invalid input leaves
    /// the ledger unchanged.
    pub fn transfer(
        &mut self,
        quantity: ConservedQuantity,
        from: EcologicalCompartment,
        to: EcologicalCompartment,
        amount: f64,
    ) -> Result<(), ConservationError> {
        validate_amount(amount)?;
        if from == to || amount == 0.0 {
            return Ok(());
        }

        let source_key = PoolKey::new(quantity, from);
        let target_key = PoolKey::new(quantity, to);
        let available = self.amount(source_key);
        if available < amount {
            return Err(ConservationError::InsufficientSource {
                quantity,
                compartment: from,
                available,
                requested: amount,
            });
        }

        self.pools.insert(source_key, available - amount);
        let target = self.amount(target_key);
        self.pools.insert(target_key, target + amount);
        Ok(())
    }

    /// Add a quantity from outside the simulated accounting boundary.
    pub fn input(
        &mut self,
        quantity: ConservedQuantity,
        to: EcologicalCompartment,
        amount: f64,
    ) -> Result<(), ConservationError> {
        validate_amount(amount)?;
        let target_key = PoolKey::new(quantity, to);
        let target = self.amount(target_key);
        self.pools.insert(target_key, target + amount);
        *self.external_in.entry(quantity).or_default() += amount;
        Ok(())
    }

    /// Remove a quantity through an explicit system-boundary output.
    ///
    /// Failure is atomic if the source pool is insufficient.
    pub fn output(
        &mut self,
        quantity: ConservedQuantity,
        from: EcologicalCompartment,
        amount: f64,
    ) -> Result<(), ConservationError> {
        validate_amount(amount)?;
        let source_key = PoolKey::new(quantity, from);
        let available = self.amount(source_key);
        if available < amount {
            return Err(ConservationError::InsufficientSource {
                quantity,
                compartment: from,
                available,
                requested: amount,
            });
        }

        self.pools.insert(source_key, available - amount);
        *self.external_out.entry(quantity).or_default() += amount;
        Ok(())
    }

    /// Compare current totals against an earlier ledger snapshot.
    ///
    /// For each quantity:
    ///
    /// `expected = baseline_total + external_inputs - external_outputs`
    ///
    /// where external deltas are measured since the baseline snapshot.
    pub fn balance_since(&self, baseline: &Self) -> ConservationReport {
        let mut entries = BTreeMap::new();
        for quantity in ConservedQuantity::ALL {
            let baseline_total = baseline.total(quantity);
            let input_delta = self.external_input(quantity) - baseline.external_input(quantity);
            let output_delta = self.external_output(quantity) - baseline.external_output(quantity);
            let expected = baseline_total + input_delta - output_delta;
            let actual = self.total(quantity);
            entries.insert(
                quantity,
                ConservationBalance {
                    baseline: baseline_total,
                    external_input: input_delta,
                    external_output: output_delta,
                    expected,
                    actual,
                    residual: actual - expected,
                },
            );
        }
        ConservationReport { entries }
    }
}

impl ConservedQuantity {
    pub const ALL: [Self; 4] = [
        Self::BiomassMass,
        Self::CarbonMass,
        Self::WaterMass,
        Self::MineralNutrientMass,
    ];
}

/// Balance for one conserved quantity over a qualification interval.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConservationBalance {
    pub baseline: f64,
    pub external_input: f64,
    pub external_output: f64,
    pub expected: f64,
    pub actual: f64,
    pub residual: f64,
}

/// Multi-quantity conservation result.
#[derive(Debug, Clone, PartialEq)]
pub struct ConservationReport {
    entries: BTreeMap<ConservedQuantity, ConservationBalance>,
}

impl ConservationReport {
    pub fn balance(&self, quantity: ConservedQuantity) -> ConservationBalance {
        self.entries[&quantity]
    }

    /// Return true only when every tracked quantity satisfies the absolute
    /// residual tolerance.
    pub fn within_tolerance(&self, absolute_tolerance: f64) -> bool {
        absolute_tolerance.is_finite()
            && absolute_tolerance >= 0.0
            && self
                .entries
                .values()
                .all(|entry| entry.residual.abs() <= absolute_tolerance)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConservationError {
    NonFiniteAmount(f64),
    NegativeAmount(f64),
    InsufficientSource {
        quantity: ConservedQuantity,
        compartment: EcologicalCompartment,
        available: f64,
        requested: f64,
    },
}

impl fmt::Display for ConservationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteAmount(amount) => {
                write!(formatter, "ecological amount must be finite, got {amount}")
            }
            Self::NegativeAmount(amount) => {
                write!(formatter, "ecological amount must be non-negative, got {amount}")
            }
            Self::InsufficientSource {
                quantity,
                compartment,
                available,
                requested,
            } => write!(
                formatter,
                "insufficient {quantity:?} in {compartment:?}: available {available}, requested {requested}"
            ),
        }
    }
}

impl Error for ConservationError {}

fn validate_amount(amount: f64) -> Result<(), ConservationError> {
    if !amount.is_finite() {
        return Err(ConservationError::NonFiniteAmount(amount));
    }
    if amount < 0.0 {
        return Err(ConservationError::NegativeAmount(amount));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seeded_biomass() -> EcologicalLedger {
        let mut ledger = EcologicalLedger::default();
        ledger
            .seed(
                PoolKey::new(
                    ConservedQuantity::BiomassMass,
                    EcologicalCompartment::Living,
                ),
                100.0,
            )
            .unwrap();
        ledger
    }

    #[test]
    fn internal_transfer_conserves_typed_quantity_exactly() {
        let mut ledger = seeded_biomass();
        let baseline = ledger.clone();

        ledger
            .transfer(
                ConservedQuantity::BiomassMass,
                EcologicalCompartment::Living,
                EcologicalCompartment::Detritus,
                35.0,
            )
            .unwrap();

        assert_eq!(
            ledger.amount(PoolKey::new(
                ConservedQuantity::BiomassMass,
                EcologicalCompartment::Living,
            )),
            65.0
        );
        assert_eq!(
            ledger.amount(PoolKey::new(
                ConservedQuantity::BiomassMass,
                EcologicalCompartment::Detritus,
            )),
            35.0
        );
        assert!(ledger.balance_since(&baseline).within_tolerance(0.0));
    }

    #[test]
    fn explicit_external_fluxes_are_accounted_for() {
        let mut ledger = EcologicalLedger::default();
        ledger
            .seed(
                PoolKey::new(
                    ConservedQuantity::WaterMass,
                    EcologicalCompartment::Soil,
                ),
                10.0,
            )
            .unwrap();
        let baseline = ledger.clone();

        ledger
            .input(
                ConservedQuantity::WaterMass,
                EcologicalCompartment::Soil,
                7.5,
            )
            .unwrap();
        ledger
            .output(
                ConservedQuantity::WaterMass,
                EcologicalCompartment::Soil,
                2.0,
            )
            .unwrap();

        let balance = ledger
            .balance_since(&baseline)
            .balance(ConservedQuantity::WaterMass);
        assert_eq!(balance.baseline, 10.0);
        assert_eq!(balance.external_input, 7.5);
        assert_eq!(balance.external_output, 2.0);
        assert_eq!(balance.expected, 15.5);
        assert_eq!(balance.actual, 15.5);
        assert_eq!(balance.residual, 0.0);
    }

    #[test]
    fn insufficient_transfer_is_fail_closed_and_atomic() {
        let mut ledger = seeded_biomass();
        let before = ledger.clone();

        let result = ledger.transfer(
            ConservedQuantity::BiomassMass,
            EcologicalCompartment::Living,
            EcologicalCompartment::Detritus,
            101.0,
        );

        assert!(matches!(
            result,
            Err(ConservationError::InsufficientSource { .. })
        ));
        assert_eq!(ledger, before);
    }

    #[test]
    fn invalid_amounts_never_mutate_ledger() {
        let mut ledger = seeded_biomass();
        let before = ledger.clone();

        assert!(matches!(
            ledger.input(
                ConservedQuantity::WaterMass,
                EcologicalCompartment::Soil,
                f64::NAN,
            ),
            Err(ConservationError::NonFiniteAmount(_))
        ));
        assert!(matches!(
            ledger.output(
                ConservedQuantity::BiomassMass,
                EcologicalCompartment::Living,
                -1.0,
            ),
            Err(ConservationError::NegativeAmount(_))
        ));
        assert_eq!(ledger, before);
    }

    #[test]
    fn tolerance_is_checked_for_every_quantity() {
        let ledger = seeded_biomass();
        let report = ledger.balance_since(&ledger);

        assert!(report.within_tolerance(0.0));
        assert!(!report.within_tolerance(-1.0));
        assert!(!report.within_tolerance(f64::NAN));
    }
}
