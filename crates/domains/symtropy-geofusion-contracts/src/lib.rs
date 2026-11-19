// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![forbid(unsafe_code)]

//! Evidence-bounded energy-stream contracts for geothermal/fusion integration.
//!
//! This crate is an integration ledger, not a reservoir, plasma, thermal-hydraulic,
//! CFD, neutronics, exergy, or plant-control solver. Numeric values are declared
//! at explicit boundaries and must be independently sourced and qualified.

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

        if !relative_tolerance.is_finite() || relative_tolerance < 0.0 {
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
                if available_delta + relative_tolerance < grade.minimum_approach_k {
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

/// A first-law balance over one declared boundary and one energy carrier.
/// Positive `storage_delta_mwh` means stored energy increased; negative means
/// stored energy was discharged. Exergy is deliberately not inferred here.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EnergyBalance {
    pub input_mwh: f64,
    pub useful_output_mwh: f64,
    pub rejected_mwh: f64,
    pub conversion_loss_mwh: f64,
    pub storage_delta_mwh: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BalanceResult {
    pub residual_mwh: f64,
    pub normalized_absolute_residual: f64,
    pub within_tolerance: bool,
}

impl EnergyBalance {
    /// Evaluate `input = useful + rejected + losses + change_in_storage`.
    pub fn evaluate(&self, relative_tolerance: f64) -> Result<BalanceResult, ValidationError> {
        if !relative_tolerance.is_finite() || relative_tolerance < 0.0 {
            return Err(ValidationError::InvalidTolerance);
        }
        for (name, value) in [
            ("input MWh", self.input_mwh),
            ("useful output MWh", self.useful_output_mwh),
            ("rejected MWh", self.rejected_mwh),
            ("conversion loss MWh", self.conversion_loss_mwh),
            ("storage delta MWh", self.storage_delta_mwh),
        ] {
            if !value.is_finite() {
                return Err(ValidationError::NonFiniteOrNonPositive(name));
            }
        }
        if self.input_mwh < 0.0
            || self.useful_output_mwh < 0.0
            || self.rejected_mwh < 0.0
            || self.conversion_loss_mwh < 0.0
        {
            return Err(ValidationError::NonFiniteOrNonPositive("negative energy term"));
        }

        let residual = self.input_mwh
            - self.useful_output_mwh
            - self.rejected_mwh
            - self.conversion_loss_mwh
            - self.storage_delta_mwh;
        let scale = self
            .input_mwh
            .abs()
            .max(
                self.useful_output_mwh.abs()
                    + self.rejected_mwh.abs()
                    + self.conversion_loss_mwh.abs()
                    + self.storage_delta_mwh.abs(),
            )
            .max(f64::MIN_POSITIVE);
        let normalized = residual.abs() / scale;
        Ok(BalanceResult {
            residual_mwh: residual,
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
            input_mwh: 0.0,
            useful_output_mwh: 0.95,
            rejected_mwh: 0.0,
            conversion_loss_mwh: 0.05,
            storage_delta_mwh: -1.0,
        };
        let result = balance.evaluate(1e-12).unwrap();
        assert!(result.within_tolerance);
        assert_eq!(result.residual_mwh, 0.0);
    }

    #[test]
    fn balance_exposes_mismatch_instead_of_silently_closing() {
        let balance = EnergyBalance {
            input_mwh: 10.0,
            useful_output_mwh: 8.0,
            rejected_mwh: 1.0,
            conversion_loss_mwh: 0.5,
            storage_delta_mwh: 0.0,
        };
        let result = balance.evaluate(0.001).unwrap();
        assert!(!result.within_tolerance);
        assert!((result.residual_mwh - 0.5).abs() < 1e-12);
    }
}
