// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![forbid(unsafe_code)]

//! Evidence-bounded energy-stream contracts for geothermal/fusion integration.
//!
//! This crate is an integration ledger, not a reservoir, plasma, thermal-hydraulic,
//! CFD, neutronics, exergy, or plant-control solver. Numeric values are declared
//! at explicit boundaries and must be independently sourced and qualified.

use std::collections::BTreeSet;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnergyCarrier {
    /// Electrical power (MW_e) and electrical energy (MWh_e).
    Electricity,
    /// Thermal power (MW_th) and thermal energy (MWh_th).
    Heat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceKind {
    GeothermalPrimaryHeat,
    GeothermalSecondaryHeat,
    GeothermalGeneratedElectricity,
    FusionPrimaryHeat,
    FusionSecondaryHeat,
    FusionGeneratedElectricity,
    GridElectricity,
    ElectricalStorage,
    ThermalStorage,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceKind {
    DtPlasmaHeating,
    FusionElectricalAuxiliary,
    FacilityElectricalLoad,
    IndustrialProcessHeat,
    FacilityHeat,
    GridExport,
    ThermalStorageCharge,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceClass {
    Synthetic,
    PublishedDesign,
    VendorReported,
    Measured,
    CalibratedModel,
    IndependentlyValidatedModel,
    AuditedOperation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitKind {
    GeothermalPrimary,
    FusionPrimary,
    ProcessSecondary,
    FacilitySecondary,
    ThermalStorageSecondary,
    OtherSecondary,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThermalGrade {
    /// Source-side fluid temperature at the declared transfer boundary (°C).
    pub source_temperature_c: f64,
    /// Minimum temperature required at the receiving service (°C).
    pub required_sink_temperature_c: f64,
    /// Minimum exchanger approach temperature (K; temperature differences).
    pub minimum_approach_k: f64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThermalIsolationBoundary {
    /// Stable identifier for the intermediate heat exchanger / isolation boundary.
    pub interface_id: String,
    pub source_circuit_id: String,
    pub source_circuit_kind: CircuitKind,
    pub receiving_circuit_id: String,
    pub receiving_circuit_kind: CircuitKind,
    /// Declared isolation, not proof of physical qualification.
    pub receiving_loop_isolated: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnergyStream {
    pub stream_id: String,
    pub source_asset_id: String,
    pub destination_asset_id: String,
    pub source_kind: SourceKind,
    pub carrier: EnergyCarrier,
    pub service: ServiceKind,
    /// Power in MW_e or MW_th as determined by `carrier`.
    pub power_mw: f64,
    /// Duration in hours.
    pub duration_hours: f64,
    /// Energy in MWh_e or MWh_th as determined by `carrier`.
    pub energy_mwh: f64,
    pub thermal_grade: Option<ThermalGrade>,
    pub thermal_isolation: Option<ThermalIsolationBoundary>,
    pub measurement_boundary_id: String,
    pub evidence_class: EvidenceClass,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    EmptyIdentity(&'static str),
    NonFiniteOrNonPositive(&'static str),
    InvalidTolerance,
    EnergyPowerDurationMismatch,
    ThermalGradeMissing,
    ThermalGradeInvalid,
    InsufficientTemperatureApproach,
    UnexpectedThermalGrade,
    SourceCarrierMismatch,
    PrimaryThermalIsolationMissing,
    PrimaryThermalIsolationInvalid,
    PrimaryCircuitCrossConnection,
    PlasmaHeatingMustBeElectrical,
    PrimaryGeothermalHeatCannotFeedPlasma,
    ServiceCarrierMismatch,
    EmptyEnergyBalance,
    NegativeEnergyTerm(&'static str),
    ArithmeticOverflow,
    EmptySourceAllocationLedger,
    DuplicateAllocationId,
    SourceAllocationIdentityMissing(&'static str),
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for ValidationError {}

impl EnergyStream {
    /// Validate identifiers, units, energy arithmetic, temperature grade,
    /// service/carrier compatibility, and declared primary-loop isolation.
    ///
    /// `relative_tolerance` is dimensionless and applied to the power-duration
    /// energy identity. Passing validation does not certify source truth or plant safety.
    pub fn validate(&self, relative_tolerance: f64) -> Result<(), ValidationError> {
        for (name, value) in [
            ("stream ID", self.stream_id.as_str()),
            ("source asset ID", self.source_asset_id.as_str()),
            ("destination asset ID", self.destination_asset_id.as_str()),
            ("measurement boundary ID", self.measurement_boundary_id.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(ValidationError::EmptyIdentity(name));
            }
        }

        if !relative_tolerance.is_finite() || !(0.0..1.0).contains(&relative_tolerance) {
            return Err(ValidationError::InvalidTolerance);
        }
        for (name, value) in [
            ("power MW", self.power_mw),
            ("duration hours", self.duration_hours),
            ("energy MWh", self.energy_mwh),
        ] {
            if !value.is_finite() || value <= 0.0 {
                return Err(ValidationError::NonFiniteOrNonPositive(name));
            }
        }

        let expected_mwh = self.power_mw * self.duration_hours;
        if !expected_mwh.is_finite() || expected_mwh <= 0.0 {
            return Err(ValidationError::NonFiniteOrNonPositive("power × duration MWh"));
        }
        let scale = expected_mwh.abs().max(self.energy_mwh.abs());
        if (expected_mwh - self.energy_mwh).abs() > relative_tolerance * scale {
            return Err(ValidationError::EnergyPowerDurationMismatch);
        }

        match (self.carrier, self.thermal_grade) {
            (EnergyCarrier::Heat, Some(grade)) => {
                if !grade.source_temperature_c.is_finite()
                    || !grade.required_sink_temperature_c.is_finite()
                    || !grade.minimum_approach_k.is_finite()
                    || grade.source_temperature_c <= -273.15
                    || grade.required_sink_temperature_c <= -273.15
                    || grade.minimum_approach_k < 0.0
                {
                    return Err(ValidationError::ThermalGradeInvalid);
                }
                let available_delta =
                    grade.source_temperature_c - grade.required_sink_temperature_c;
                if available_delta < grade.minimum_approach_k {
                    return Err(ValidationError::InsufficientTemperatureApproach);
                }
            }
            (EnergyCarrier::Heat, None) => return Err(ValidationError::ThermalGradeMissing),
            (EnergyCarrier::Electricity, Some(_)) => {
                return Err(ValidationError::UnexpectedThermalGrade);
            }
            (EnergyCarrier::Electricity, None) => {}
        }

        match self.service {
            ServiceKind::DtPlasmaHeating => {
                if self.carrier != EnergyCarrier::Electricity {
                    if self.source_kind == SourceKind::GeothermalPrimaryHeat {
                        return Err(ValidationError::PrimaryGeothermalHeatCannotFeedPlasma);
                    }
                    return Err(ValidationError::PlasmaHeatingMustBeElectrical);
                }
                if self.source_kind == SourceKind::GeothermalPrimaryHeat {
                    return Err(ValidationError::PrimaryGeothermalHeatCannotFeedPlasma);
                }
            }
            ServiceKind::FusionElectricalAuxiliary
            | ServiceKind::FacilityElectricalLoad
            | ServiceKind::GridExport => {
                if self.carrier != EnergyCarrier::Electricity {
                    return Err(ValidationError::ServiceCarrierMismatch);
                }
            }
            ServiceKind::IndustrialProcessHeat
            | ServiceKind::FacilityHeat
            | ServiceKind::ThermalStorageCharge => {
                if self.carrier != EnergyCarrier::Heat {
                    return Err(ValidationError::ServiceCarrierMismatch);
                }
            }
            ServiceKind::Other => {}
        }

        let expected_source_carrier = match self.source_kind {
            SourceKind::GeothermalPrimaryHeat
            | SourceKind::GeothermalSecondaryHeat
            | SourceKind::FusionPrimaryHeat
            | SourceKind::FusionSecondaryHeat
            | SourceKind::ThermalStorage => Some(EnergyCarrier::Heat),
            SourceKind::GeothermalGeneratedElectricity
            | SourceKind::FusionGeneratedElectricity
            | SourceKind::GridElectricity
            | SourceKind::ElectricalStorage => Some(EnergyCarrier::Electricity),
            SourceKind::Other => None,
        };
        if expected_source_carrier.is_some_and(|expected| expected != self.carrier) {
            return Err(ValidationError::SourceCarrierMismatch);
        }

        match self.source_kind {
            SourceKind::GeothermalPrimaryHeat | SourceKind::FusionPrimaryHeat => {
                let boundary = self
                    .thermal_isolation
                    .as_ref()
                    .ok_or(ValidationError::PrimaryThermalIsolationMissing)?;
                if boundary.interface_id.trim().is_empty()
                    || boundary.source_circuit_id.trim().is_empty()
                    || boundary.receiving_circuit_id.trim().is_empty()
                    || boundary.source_circuit_id == boundary.receiving_circuit_id
                    || !boundary.receiving_loop_isolated
                {
                    return Err(ValidationError::PrimaryThermalIsolationInvalid);
                }
                let source_kind_matches = matches!(
                    (self.source_kind, boundary.source_circuit_kind),
                    (SourceKind::GeothermalPrimaryHeat, CircuitKind::GeothermalPrimary)
                        | (SourceKind::FusionPrimaryHeat, CircuitKind::FusionPrimary)
                );
                if !source_kind_matches {
                    return Err(ValidationError::PrimaryThermalIsolationInvalid);
                }
                if matches!(
                    boundary.receiving_circuit_kind,
                    CircuitKind::GeothermalPrimary | CircuitKind::FusionPrimary
                ) {
                    return Err(ValidationError::PrimaryCircuitCrossConnection);
                }
            }
            _ => {}
        }

        Ok(())
    }
}

/// One energy term at a named carrier boundary. All terms use MWh as the
/// common energy dimension; the carrier label is retained so electric and
/// thermal paths cannot be silently interchanged in downstream reporting.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EnergyTerm {
    pub carrier: EnergyCarrier,
    pub energy_mwh: f64,
}

/// First-law balance across a component that may convert between carriers.
/// Positive storage delta means the store gained energy; negative means it
/// discharged. Exergy is not inferred from this energy-only balance.
#[derive(Debug, Clone, PartialEq)]
pub struct EnergyBalance {
    pub inputs: Vec<EnergyTerm>,
    pub useful_outputs: Vec<EnergyTerm>,
    pub rejected_outputs: Vec<EnergyTerm>,
    pub conversion_losses: Vec<EnergyTerm>,
    pub storage_deltas: Vec<EnergyTerm>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BalanceResult {
    pub residual_mwh: f64,
    pub normalized_absolute_residual: f64,
    pub within_tolerance: bool,
}

fn sum_energy_terms(
    terms: &[EnergyTerm],
    name: &'static str,
    allow_negative: bool,
) -> Result<f64, ValidationError> {
    terms.iter().try_fold(0.0, |total, term| {
        if !term.energy_mwh.is_finite() {
            return Err(ValidationError::NonFiniteOrNonPositive(name));
        }
        if !allow_negative && term.energy_mwh < 0.0 {
            return Err(ValidationError::NegativeEnergyTerm(name));
        }
        let next = total + term.energy_mwh;
        if !next.is_finite() {
            return Err(ValidationError::ArithmeticOverflow);
        }
        Ok(next)
    })
}

impl EnergyBalance {
    /// Evaluate `sum(inputs) = sum(useful) + sum(rejected) + sum(losses) + Δstorage`.
    ///
    /// Electric and thermal terms may coexist in this first-law balance because
    /// both are energy quantities expressed in MWh. Their carrier labels remain
    /// attached to each term. This does not calculate exergy or conversion quality.
    pub fn evaluate(&self, relative_tolerance: f64) -> Result<BalanceResult, ValidationError> {
        if !relative_tolerance.is_finite() || !(0.0..1.0).contains(&relative_tolerance) {
            return Err(ValidationError::InvalidTolerance);
        }
        if self.inputs.is_empty()
            && self.useful_outputs.is_empty()
            && self.rejected_outputs.is_empty()
            && self.conversion_losses.is_empty()
            && self.storage_deltas.is_empty()
        {
            return Err(ValidationError::EmptyEnergyBalance);
        }

        let input = sum_energy_terms(&self.inputs, "input MWh", false)?;
        let useful = sum_energy_terms(&self.useful_outputs, "useful output MWh", false)?;
        let rejected = sum_energy_terms(&self.rejected_outputs, "rejected MWh", false)?;
        let losses = sum_energy_terms(&self.conversion_losses, "conversion loss MWh", false)?;
        let storage_delta = sum_energy_terms(&self.storage_deltas, "storage delta MWh", true)?;

        // Normalize before adding cross-category terms. The direct sum in the
        // denominator can overflow even when every category sum is finite; that
        // can otherwise turn a large imbalance into NaN and falsely report closure.
        let scale = [input.abs(), useful.abs(), rejected.abs(), losses.abs(), storage_delta.abs()]
            .into_iter()
            .fold(0.0_f64, f64::max)
            .max(f64::MIN_POSITIVE);
        let scaled_residual = input / scale
            - useful / scale
            - rejected / scale
            - losses / scale
            - storage_delta / scale;
        let normalized_denominator = input.abs() / scale
            + useful.abs() / scale
            + rejected.abs() / scale
            + losses.abs() / scale
            + storage_delta.abs() / scale;
        let normalized = if normalized_denominator == 0.0 {
            0.0
        } else {
            scaled_residual.abs() / normalized_denominator
        };
        let residual = scaled_residual * scale;
        if !scaled_residual.is_finite() || !normalized.is_finite() || !residual.is_finite() {
            return Err(ValidationError::ArithmeticOverflow);
        }
        Ok(BalanceResult {
            residual_mwh: residual,
            normalized_absolute_residual: normalized,
            within_tolerance: normalized <= relative_tolerance,
        })
    }
}


/// One use of energy leaving a single source boundary during one frozen interval.
/// The carrier remains attached so heat and electricity are not silently confused.
#[derive(Debug, Clone, PartialEq)]
pub struct EnergyAllocation {
    pub allocation_id: String,
    pub destination_asset_id: String,
    pub carrier: EnergyCarrier,
    pub service: ServiceKind,
    pub energy_mwh: f64,
}

/// Complete allocation of one source/carrier over one declared interval.
///
/// The ledger prevents a source energy quantity from being silently allocated
/// more than once across multiple destinations. Curtailment is explicit.
/// A positive residual means source energy remains unallocated; a negative
/// residual means the requested allocations exceed the available source energy.
#[derive(Debug, Clone, PartialEq)]
pub struct SourceAllocationLedger {
    pub source_asset_id: String,
    pub source_boundary_id: String,
    pub source_interval_id: String,
    pub carrier: EnergyCarrier,
    pub available_energy_mwh: f64,
    pub allocations: Vec<EnergyAllocation>,
    pub curtailed_energy_mwh: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SourceAllocationReport {
    pub allocated_energy_mwh: f64,
    pub curtailed_energy_mwh: f64,
    /// Positive = unallocated source energy; negative = over-allocation.
    pub residual_energy_mwh: f64,
    pub normalized_absolute_residual: f64,
    pub within_tolerance: bool,
}

impl SourceAllocationLedger {
    /// Validate a complete source interval allocation using a dimensionless
    /// relative tolerance in [0, 1). This is an arithmetic anti-double-count
    /// contract, not a meter authenticity or physical-source qualification.
    pub fn evaluate(
        &self,
        relative_tolerance: f64,
    ) -> Result<SourceAllocationReport, ValidationError> {
        for (name, value) in [
            ("source asset ID", self.source_asset_id.as_str()),
            ("source boundary ID", self.source_boundary_id.as_str()),
            ("source interval ID", self.source_interval_id.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(ValidationError::SourceAllocationIdentityMissing(name));
            }
        }
        if !relative_tolerance.is_finite() || !(0.0..1.0).contains(&relative_tolerance) {
            return Err(ValidationError::InvalidTolerance);
        }
        if !self.available_energy_mwh.is_finite() || self.available_energy_mwh <= 0.0 {
            return Err(ValidationError::NonFiniteOrNonPositive(
                "available source energy MWh",
            ));
        }
        if !self.curtailed_energy_mwh.is_finite() || self.curtailed_energy_mwh < 0.0 {
            return Err(ValidationError::NegativeEnergyTerm(
                "curtailed source energy MWh",
            ));
        }
        if self.allocations.is_empty() && self.curtailed_energy_mwh == 0.0 {
            return Err(ValidationError::EmptySourceAllocationLedger);
        }

        let mut allocation_ids = BTreeSet::new();
        let mut terms = Vec::with_capacity(self.allocations.len());
        for allocation in &self.allocations {
            if allocation.allocation_id.trim().is_empty() {
                return Err(ValidationError::SourceAllocationIdentityMissing(
                    "allocation ID",
                ));
            }
            if allocation.destination_asset_id.trim().is_empty() {
                return Err(ValidationError::SourceAllocationIdentityMissing(
                    "allocation destination asset ID",
                ));
            }
            if !allocation_ids.insert(allocation.allocation_id.as_str()) {
                return Err(ValidationError::DuplicateAllocationId);
            }
            if allocation.carrier != self.carrier {
                return Err(ValidationError::SourceCarrierMismatch);
            }
            if !allocation.energy_mwh.is_finite() || allocation.energy_mwh <= 0.0 {
                return Err(ValidationError::NonFiniteOrNonPositive(
                    "allocated source energy MWh",
                ));
            }
            terms.push(EnergyTerm {
                carrier: allocation.carrier,
                energy_mwh: allocation.energy_mwh,
            });
        }

        let allocated = sum_energy_terms(&terms, "allocated source energy MWh", false)?;
        let available = self.available_energy_mwh;
        let curtailed = self.curtailed_energy_mwh;

        // Scale before forming sums so a large allocation cannot overflow the
        // normalization denominator and masquerade as a balanced source.
        let scale = available
            .abs()
            .max(allocated.abs())
            .max(curtailed.abs())
            .max(f64::MIN_POSITIVE);
        let scaled_residual = available / scale - allocated / scale - curtailed / scale;
        let normalized_denominator =
            available / scale + allocated / scale + curtailed / scale;
        let normalized = if normalized_denominator == 0.0 {
            0.0
        } else {
            scaled_residual.abs() / normalized_denominator
        };
        let residual = scaled_residual * scale;
        if !scaled_residual.is_finite() || !normalized.is_finite() || !residual.is_finite() {
            return Err(ValidationError::ArithmeticOverflow);
        }

        Ok(SourceAllocationReport {
            allocated_energy_mwh: allocated,
            curtailed_energy_mwh: curtailed,
            residual_energy_mwh: residual,
            normalized_absolute_residual: normalized,
            within_tolerance: normalized <= relative_tolerance,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn electric_stream(source: SourceKind, service: ServiceKind) -> EnergyStream {
        EnergyStream {
            stream_id: "stream-1".into(),
            source_asset_id: "source-1".into(),
            destination_asset_id: "sink-1".into(),
            source_kind: source,
            carrier: EnergyCarrier::Electricity,
            service,
            power_mw: 8.0,
            duration_hours: 0.25,
            energy_mwh: 2.0,
            thermal_grade: None,
            thermal_isolation: None,
            measurement_boundary_id: "pcc-1".into(),
            evidence_class: EvidenceClass::Synthetic,
        }
    }

    fn geothermal_heat_stream() -> EnergyStream {
        EnergyStream {
            stream_id: "geo-heat-1".into(),
            source_asset_id: "geo-wellfield-1".into(),
            destination_asset_id: "process-loop-1".into(),
            source_kind: SourceKind::GeothermalPrimaryHeat,
            carrier: EnergyCarrier::Heat,
            service: ServiceKind::IndustrialProcessHeat,
            power_mw: 20.0,
            duration_hours: 1.0,
            energy_mwh: 20.0,
            thermal_grade: Some(ThermalGrade {
                source_temperature_c: 180.0,
                required_sink_temperature_c: 150.0,
                minimum_approach_k: 20.0,
            }),
            thermal_isolation: Some(ThermalIsolationBoundary {
                interface_id: "hx-1".into(),
                source_circuit_id: "geo-primary-1".into(),
                source_circuit_kind: CircuitKind::GeothermalPrimary,
                receiving_circuit_id: "process-secondary-1".into(),
                receiving_circuit_kind: CircuitKind::ProcessSecondary,
                receiving_loop_isolated: true,
            }),
            measurement_boundary_id: "hx-outlet-1".into(),
            evidence_class: EvidenceClass::Synthetic,
        }
    }

    fn allocation_ledger(
        allocations: Vec<EnergyAllocation>,
        curtailed_energy_mwh: f64,
    ) -> SourceAllocationLedger {
        SourceAllocationLedger {
            source_asset_id: "geo-wellfield-1".into(),
            source_boundary_id: "wellhead-hx-outlet-1".into(),
            source_interval_id: "2026-01-hour-0001".into(),
            carrier: EnergyCarrier::Heat,
            available_energy_mwh: 10.0,
            allocations,
            curtailed_energy_mwh,
        }
    }

    fn heat_allocation(
        allocation_id: &str,
        destination_asset_id: &str,
        energy_mwh: f64,
    ) -> EnergyAllocation {
        EnergyAllocation {
            allocation_id: allocation_id.into(),
            destination_asset_id: destination_asset_id.into(),
            carrier: EnergyCarrier::Heat,
            service: ServiceKind::IndustrialProcessHeat,
            energy_mwh,
        }
    }

    #[test]
    fn source_energy_can_be_split_across_heat_services_without_double_counting() {
        let ledger = allocation_ledger(
            vec![
                heat_allocation("direct-heat-edge", "process-plant", 6.0),
                heat_allocation("power-cycle-edge", "orc-evaporator", 3.0),
            ],
            1.0,
        );
        let result = ledger.evaluate(1e-12).unwrap();
        assert!(result.within_tolerance);
        assert_eq!(result.allocated_energy_mwh, 9.0);
        assert_eq!(result.curtailed_energy_mwh, 1.0);
        assert_eq!(result.residual_energy_mwh, 0.0);
    }

    #[test]
    fn source_overallocation_is_visible_and_not_silently_closed() {
        let ledger = allocation_ledger(
            vec![
                heat_allocation("direct-heat-edge", "process-plant", 6.0),
                heat_allocation("power-cycle-edge", "orc-evaporator", 5.0),
            ],
            0.0,
        );
        let result = ledger.evaluate(1e-3).unwrap();
        assert!(!result.within_tolerance);
        assert!((result.residual_energy_mwh + 1.0).abs() < 1e-12);
    }

    #[test]
    fn unallocated_source_energy_remains_visible() {
        let ledger = allocation_ledger(
            vec![heat_allocation("direct-heat-edge", "process-plant", 6.0)],
            2.0,
        );
        let result = ledger.evaluate(1e-3).unwrap();
        assert!(!result.within_tolerance);
        assert!((result.residual_energy_mwh - 2.0).abs() < 1e-12);
    }

    #[test]
    fn duplicate_allocation_ids_are_rejected() {
        let ledger = allocation_ledger(
            vec![
                heat_allocation("same-edge", "process-plant", 4.0),
                heat_allocation("same-edge", "water-plant", 3.0),
            ],
            3.0,
        );
        assert_eq!(
            ledger.evaluate(1e-9),
            Err(ValidationError::DuplicateAllocationId)
        );
    }

    #[test]
    fn source_allocation_carrier_mismatch_is_rejected() {
        let mut allocation = heat_allocation("electric-edge", "facility-bus", 2.0);
        allocation.carrier = EnergyCarrier::Electricity;
        let ledger = allocation_ledger(vec![allocation], 8.0);
        assert_eq!(
            ledger.evaluate(1e-9),
            Err(ValidationError::SourceCarrierMismatch)
        );
    }

    #[test]
    fn missing_source_interval_identity_is_rejected() {
        let mut ledger = allocation_ledger(
            vec![heat_allocation("direct-heat-edge", "process-plant", 10.0)],
            0.0,
        );
        ledger.source_interval_id.clear();
        assert_eq!(
            ledger.evaluate(1e-9),
            Err(ValidationError::SourceAllocationIdentityMissing(
                "source interval ID"
            ))
        );
    }

    #[test]
    fn negative_or_non_finite_allocation_is_rejected() {
        let ledger = allocation_ledger(
            vec![heat_allocation("bad-edge", "process-plant", f64::NAN)],
            0.0,
        );
        assert_eq!(
            ledger.evaluate(1e-9),
            Err(ValidationError::NonFiniteOrNonPositive(
                "allocated source energy MWh"
            ))
        );
    }

    #[test]
    fn geothermal_generated_electricity_can_feed_fusion_auxiliaries() {
        let stream = electric_stream(
            SourceKind::GeothermalGeneratedElectricity,
            ServiceKind::FusionElectricalAuxiliary,
        );
        assert_eq!(stream.validate(1e-9), Ok(()));
    }

    #[test]
    fn geothermal_generated_electricity_can_feed_plasma_heating_power_supply() {
        let stream = electric_stream(
            SourceKind::GeothermalGeneratedElectricity,
            ServiceKind::DtPlasmaHeating,
        );
        assert_eq!(stream.validate(1e-9), Ok(()));
    }

    #[test]
    fn direct_geothermal_primary_heat_to_dt_plasma_is_rejected() {
        let mut stream = geothermal_heat_stream();
        stream.service = ServiceKind::DtPlasmaHeating;
        assert_eq!(
            stream.validate(1e-9),
            Err(ValidationError::PrimaryGeothermalHeatCannotFeedPlasma)
        );
    }

    #[test]
    fn plasma_heating_service_must_be_electrical() {
        let mut stream = geothermal_heat_stream();
        stream.source_kind = SourceKind::Other;
        stream.service = ServiceKind::DtPlasmaHeating;
        assert_eq!(
            stream.validate(1e-9),
            Err(ValidationError::PlasmaHeatingMustBeElectrical)
        );
    }

    #[test]
    fn insufficient_temperature_approach_is_rejected() {
        let mut stream = geothermal_heat_stream();
        stream.thermal_grade.as_mut().unwrap().source_temperature_c = 165.0;
        assert_eq!(
            stream.validate(1e-9),
            Err(ValidationError::InsufficientTemperatureApproach)
        );
    }

    #[test]
    fn primary_heat_requires_declared_isolation_boundary() {
        let mut stream = geothermal_heat_stream();
        stream.thermal_isolation = None;
        assert_eq!(
            stream.validate(1e-9),
            Err(ValidationError::PrimaryThermalIsolationMissing)
        );
    }

    #[test]
    fn primary_circuits_cannot_be_cross_connected() {
        let mut stream = geothermal_heat_stream();
        stream
            .thermal_isolation
            .as_mut()
            .unwrap()
            .receiving_circuit_kind = CircuitKind::FusionPrimary;
        assert_eq!(
            stream.validate(1e-9),
            Err(ValidationError::PrimaryCircuitCrossConnection)
        );
    }

    #[test]
    fn primary_source_kind_must_match_interface_circuit_kind() {
        let mut stream = geothermal_heat_stream();
        stream
            .thermal_isolation
            .as_mut()
            .unwrap()
            .source_circuit_kind = CircuitKind::FusionPrimary;
        assert_eq!(
            stream.validate(1e-9),
            Err(ValidationError::PrimaryThermalIsolationInvalid)
        );
    }

    #[test]
    fn missing_isolation_declaration_is_rejected() {
        let mut stream = geothermal_heat_stream();
        stream
            .thermal_isolation
            .as_mut()
            .unwrap()
            .receiving_loop_isolated = false;
        assert_eq!(
            stream.validate(1e-9),
            Err(ValidationError::PrimaryThermalIsolationInvalid)
        );
    }

    #[test]
    fn energy_must_match_power_times_duration() {
        let mut stream = electric_stream(
            SourceKind::GridElectricity,
            ServiceKind::FacilityElectricalLoad,
        );
        stream.energy_mwh = 2.1;
        assert_eq!(
            stream.validate(1e-9),
            Err(ValidationError::EnergyPowerDurationMismatch)
        );
    }

    #[test]
    fn power_duration_overflow_is_rejected() {
        let mut stream = electric_stream(
            SourceKind::GridElectricity,
            ServiceKind::FacilityElectricalLoad,
        );
        stream.power_mw = f64::MAX;
        stream.duration_hours = 2.0;
        stream.energy_mwh = 2.0;
        assert_eq!(
            stream.validate(1e-9),
            Err(ValidationError::NonFiniteOrNonPositive("power × duration MWh"))
        );
    }

    #[test]
    fn relative_tolerance_must_be_less_than_one() {
        let stream = electric_stream(
            SourceKind::GridElectricity,
            ServiceKind::FacilityElectricalLoad,
        );
        assert_eq!(stream.validate(1.0), Err(ValidationError::InvalidTolerance));
        let balance = EnergyBalance {
            inputs: vec![EnergyTerm {
                carrier: EnergyCarrier::Heat,
                energy_mwh: 1.0,
            }],
            useful_outputs: vec![],
            rejected_outputs: vec![],
            conversion_losses: vec![],
            storage_deltas: vec![],
        };
        assert_eq!(balance.evaluate(1.0), Err(ValidationError::InvalidTolerance));
    }

    #[test]
    fn nan_and_infinity_are_rejected() {
        let mut stream = electric_stream(
            SourceKind::GridElectricity,
            ServiceKind::FacilityElectricalLoad,
        );
        stream.power_mw = f64::NAN;
        assert_eq!(
            stream.validate(1e-9),
            Err(ValidationError::NonFiniteOrNonPositive("power MW"))
        );
        stream.power_mw = f64::INFINITY;
        assert_eq!(
            stream.validate(1e-9),
            Err(ValidationError::NonFiniteOrNonPositive("power MW"))
        );
    }

    #[test]
    fn thermal_stream_requires_temperature_grade() {
        let mut stream = geothermal_heat_stream();
        stream.thermal_grade = None;
        assert_eq!(stream.validate(1e-9), Err(ValidationError::ThermalGradeMissing));
    }

    #[test]
    fn source_kind_must_match_carrier() {
        let stream = electric_stream(
            SourceKind::GeothermalPrimaryHeat,
            ServiceKind::FacilityElectricalLoad,
        );
        assert_eq!(stream.validate(1e-9), Err(ValidationError::SourceCarrierMismatch));
    }

    #[test]
    fn storage_discharge_uses_negative_storage_delta() {
        let balance = EnergyBalance {
            inputs: vec![],
            useful_outputs: vec![EnergyTerm {
                carrier: EnergyCarrier::Electricity,
                energy_mwh: 0.95,
            }],
            rejected_outputs: vec![],
            conversion_losses: vec![EnergyTerm {
                carrier: EnergyCarrier::Electricity,
                energy_mwh: 0.05,
            }],
            storage_deltas: vec![EnergyTerm {
                carrier: EnergyCarrier::Electricity,
                energy_mwh: -1.0,
            }],
        };
        let result = balance.evaluate(1e-12).unwrap();
        assert!(result.within_tolerance);
        assert_eq!(result.residual_mwh, 0.0);
    }

    #[test]
    fn first_law_balance_can_reconcile_thermal_to_electric_conversion() {
        let balance = EnergyBalance {
            inputs: vec![EnergyTerm {
                carrier: EnergyCarrier::Heat,
                energy_mwh: 10.0,
            }],
            useful_outputs: vec![EnergyTerm {
                carrier: EnergyCarrier::Electricity,
                energy_mwh: 2.0,
            }],
            rejected_outputs: vec![EnergyTerm {
                carrier: EnergyCarrier::Heat,
                energy_mwh: 7.0,
            }],
            conversion_losses: vec![EnergyTerm {
                carrier: EnergyCarrier::Heat,
                energy_mwh: 1.0,
            }],
            storage_deltas: vec![],
        };
        let result = balance.evaluate(1e-12).unwrap();
        assert!(result.within_tolerance);
        assert_eq!(result.residual_mwh, 0.0);
    }

    #[test]
    fn balance_exposes_mismatch_instead_of_silently_closing() {
        let balance = EnergyBalance {
            inputs: vec![EnergyTerm {
                carrier: EnergyCarrier::Electricity,
                energy_mwh: 10.0,
            }],
            useful_outputs: vec![EnergyTerm {
                carrier: EnergyCarrier::Electricity,
                energy_mwh: 8.0,
            }],
            rejected_outputs: vec![EnergyTerm {
                carrier: EnergyCarrier::Electricity,
                energy_mwh: 1.0,
            }],
            conversion_losses: vec![EnergyTerm {
                carrier: EnergyCarrier::Electricity,
                energy_mwh: 0.5,
            }],
            storage_deltas: vec![],
        };
        let result = balance.evaluate(0.001).unwrap();
        assert!(!result.within_tolerance);
        assert!((result.residual_mwh - 0.5).abs() < 1e-12);
    }


    #[test]
    fn cross_category_scale_overflow_cannot_create_false_balance_closure() {
        let balance = EnergyBalance {
            inputs: vec![EnergyTerm {
                carrier: EnergyCarrier::Electricity,
                energy_mwh: f64::MAX,
            }],
            useful_outputs: vec![EnergyTerm {
                carrier: EnergyCarrier::Electricity,
                energy_mwh: f64::MAX,
            }],
            rejected_outputs: vec![EnergyTerm {
                carrier: EnergyCarrier::Electricity,
                energy_mwh: f64::MAX,
            }],
            conversion_losses: vec![],
            storage_deltas: vec![],
        };
        let result = balance.evaluate(1e-9).unwrap();
        assert!(!result.within_tolerance);
        assert!((result.normalized_absolute_residual - (1.0 / 3.0)).abs() < 1e-12);
    }

    #[test]
    fn unrepresentable_balance_residual_is_rejected() {
        let balance = EnergyBalance {
            inputs: vec![],
            useful_outputs: vec![EnergyTerm {
                carrier: EnergyCarrier::Heat,
                energy_mwh: f64::MAX,
            }],
            rejected_outputs: vec![EnergyTerm {
                carrier: EnergyCarrier::Heat,
                energy_mwh: f64::MAX,
            }],
            conversion_losses: vec![],
            storage_deltas: vec![],
        };
        assert_eq!(
            balance.evaluate(1e-9),
            Err(ValidationError::ArithmeticOverflow)
        );
    }

    #[test]
    fn empty_balance_is_rejected() {
        let balance = EnergyBalance {
            inputs: vec![],
            useful_outputs: vec![],
            rejected_outputs: vec![],
            conversion_losses: vec![],
            storage_deltas: vec![],
        };
        assert_eq!(balance.evaluate(1e-9), Err(ValidationError::EmptyEnergyBalance));
    }

    #[test]
    fn negative_non_storage_energy_term_is_rejected() {
        let balance = EnergyBalance {
            inputs: vec![EnergyTerm {
                carrier: EnergyCarrier::Heat,
                energy_mwh: -1.0,
            }],
            useful_outputs: vec![],
            rejected_outputs: vec![],
            conversion_losses: vec![],
            storage_deltas: vec![],
        };
        assert_eq!(
            balance.evaluate(1e-9),
            Err(ValidationError::NegativeEnergyTerm("input MWh"))
        );
    }
}
