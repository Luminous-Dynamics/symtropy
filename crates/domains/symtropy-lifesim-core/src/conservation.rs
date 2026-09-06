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

    /// Sum one conserved quantity over every internal compartment, rejecting
    /// any arithmetic result that would cease to be finite.
    pub fn total(&self, quantity: ConservedQuantity) -> Result<f64, ConservationError> {
        let mut total = 0.0;
        for amount in self
            .pools
            .iter()
            .filter_map(|(key, amount)| (key.quantity == quantity).then_some(*amount))
        {
            total = checked_add("sum internal pools", quantity, total, amount)?;
        }
        Ok(total)
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
    /// qualification. The aggregate for the affected quantity must remain
    /// finite as well as every individual pool.
    pub fn seed(&mut self, key: PoolKey, amount: f64) -> Result<(), ConservationError> {
        validate_amount(amount)?;
        let total = self.total(key.quantity)?;
        let existing = self.amount(key);
        let without_existing = total - existing;
        let _next_total = checked_add(
            "seed aggregate quantity",
            key.quantity,
            without_existing,
            amount,
        )?;
        self.pools.insert(key, amount);
        Ok(())
    }

    /// Move one conserved quantity between internal compartments.
    ///
    /// Failure is atomic: insufficient source quantity, invalid input, or
    /// non-finite arithmetic leaves the ledger unchanged.
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

        let target = self.amount(target_key);
        let next_target = checked_add("transfer target", quantity, target, amount)?;
        let next_source = available - amount;

        self.pools.insert(source_key, next_source);
        self.pools.insert(target_key, next_target);
        Ok(())
    }

    /// Add a quantity from outside the simulated accounting boundary.
    ///
    /// Both internal totals and cumulative boundary counters must remain finite
    /// before any mutation is committed.
    pub fn input(
        &mut self,
        quantity: ConservedQuantity,
        to: EcologicalCompartment,
        amount: f64,
    ) -> Result<(), ConservationError> {
        validate_amount(amount)?;
        let target_key = PoolKey::new(quantity, to);
        let target = self.amount(target_key);
        let total = self.total(quantity)?;
        let current_external = self.external_input(quantity);

        let next_target = checked_add("external input target", quantity, target, amount)?;
        let _next_total = checked_add("external input aggregate", quantity, total, amount)?;
        let next_external = checked_add(
            "external input counter",
            quantity,
            current_external,
            amount,
        )?;

        self.pools.insert(target_key, next_target);
        self.external_in.insert(quantity, next_external);
        Ok(())
    }

    /// Remove a quantity through an explicit system-boundary output.
    ///
    /// Failure is atomic if the source pool is insufficient or the cumulative
    /// output counter would become non-finite.
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

        let current_external = self.external_output(quantity);
        let next_external = checked_add(
            "external output counter",
            quantity,
            current_external,
            amount,
        )?;
        let next_source = available - amount;

        self.pools.insert(source_key, next_source);
        self.external_out.insert(quantity, next_external);
        Ok(())
    }

    /// Compare current totals against an earlier ledger snapshot.
    ///
    /// For each quantity:
    ///
    /// `expected = baseline_total + external_inputs - external_outputs`
    ///
    /// where external deltas are measured since the baseline snapshot.
    pub fn balance_since(&self, baseline: &Self) -> Result<ConservationReport, ConservationError> {
        let mut entries = BTreeMap::new();
        for quantity in ConservedQuantity::ALL {
            let baseline_total = baseline.total(quantity)?;
            let input_delta = self.external_input(quantity) - baseline.external_input(quantity);
            let output_delta = self.external_output(quantity) - baseline.external_output(quantity);
            let expected_before_output = checked_add(
                "balance expected input",
                quantity,
                baseline_total,
                input_delta,
            )?;
            let expected = checked_sub(
                "balance expected output",
                quantity,
                expected_before_output,
                output_delta,
            )?;
            let actual = self.total(quantity)?;
            let residual = checked_sub("balance residual", quantity, actual, expected)?;
            entries.insert(
                quantity,
                ConservationBalance {
                    baseline: baseline_total,
                    external_input: input_delta,
                    external_output: output_delta,
                    expected,
                    actual,
                    residual,
                },
            );
        }
        Ok(ConservationReport { entries })
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
    NonFiniteArithmetic {
        operation: &'static str,
        quantity: ConservedQuantity,
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
            Self::NonFiniteArithmetic {
                operation,
                quantity,
            } => write!(
                formatter,
                "ecological accounting produced a non-finite result during {operation} for {quantity:?}"
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

fn checked_add(
    operation: &'static str,
    quantity: ConservedQuantity,
    left: f64,
    right: f64,
) -> Result<f64, ConservationError> {
    let result = left + right;
    if result.is_finite() {
        Ok(result)
    } else {
        Err(ConservationError::NonFiniteArithmetic {
            operation,
            quantity,
        })
    }
}

fn checked_sub(
    operation: &'static str,
    quantity: ConservedQuantity,
    left: f64,
    right: f64,
) -> Result<f64, ConservationError> {
    let result = left - right;
    if result.is_finite() {
        Ok(result)
    } else {
        Err(ConservationError::NonFiniteArithmetic {
            operation,
            quantity,
        })
    }
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
        assert!(
            ledger
                .balance_since(&baseline)
                .unwrap()
                .within_tolerance(0.0)
        );
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
            .unwrap()
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
    fn aggregate_seed_overflow_is_rejected_atomically() {
        let mut ledger = EcologicalLedger::default();
        ledger
            .seed(
                PoolKey::new(
                    ConservedQuantity::WaterMass,
                    EcologicalCompartment::Soil,
                ),
                f64::MAX,
            )
            .unwrap();
        let before = ledger.clone();

        let result = ledger.seed(
            PoolKey::new(
                ConservedQuantity::WaterMass,
                EcologicalCompartment::WaterColumn,
            ),
            f64::MAX,
        );

        assert!(matches!(
            result,
            Err(ConservationError::NonFiniteArithmetic { .. })
        ));
        assert_eq!(ledger, before);
    }

    #[test]
    fn external_counter_overflow_is_rejected_atomically() {
        let mut ledger = EcologicalLedger::default();
        ledger
            .input(
                ConservedQuantity::WaterMass,
                EcologicalCompartment::Soil,
                f64::MAX,
            )
            .unwrap();
        ledger
            .output(
                ConservedQuantity::WaterMass,
                EcologicalCompartment::Soil,
                f64::MAX,
            )
            .unwrap();
        let before = ledger.clone();

        let result = ledger.input(
            ConservedQuantity::WaterMass,
            EcologicalCompartment::Soil,
            f64::MAX,
        );

        assert!(matches!(
            result,
            Err(ConservationError::NonFiniteArithmetic { .. })
        ));
        assert_eq!(ledger, before);
    }

    #[test]
    fn tolerance_is_checked_for_every_quantity() {
        let ledger = seeded_biomass();
        let report = ledger.balance_since(&ledger).unwrap();

        assert!(report.within_tolerance(0.0));
        assert!(!report.within_tolerance(-1.0));
        assert!(!report.within_tolerance(f64::NAN));
    }
}
