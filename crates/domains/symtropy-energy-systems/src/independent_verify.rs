// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Independent arithmetic verification for batch-bound generator duty cycles.
//!
//! This module recomputes every segment from retained fuel-batch LHV,
//! evidence-bearing mass flow, engine efficiency, generator efficiency and
//! auxiliary load. It then re-integrates the cycle totals without calling the
//! production evaluate() or duty-cycle evaluate() functions.
//!
//! A PASS establishes arithmetic consistency of the supplied inputs and
//! outputs. It does not establish that the inputs are physically accurate,
//! that evidence sources are authentic, or that the topology is qualified.

use core::fmt;

use serde::Serialize;

use crate::BatchBoundDutyCycleResult;

/// Absolute and relative tolerances for independent verification.
///
/// Power checks use the watt tolerances; integrated checks use joule
/// tolerances. These are numerical comparison tolerances, not metrology
/// uncertainties.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct EnergyVerificationTolerance {
    absolute_power_w: f64,
    relative_power: f64,
    absolute_energy_j: f64,
    relative_energy: f64,
    absolute_mass_kg: f64,
    relative_mass: f64,
}

impl EnergyVerificationTolerance {
    pub fn new(
        absolute_power_w: f64,
        relative_power: f64,
        absolute_energy_j: f64,
        relative_energy: f64,
        absolute_mass_kg: f64,
        relative_mass: f64,
    ) -> Result<Self, EnergyVerificationError> {
        for (name, value) in [
            ("absolute power tolerance", absolute_power_w),
            ("relative power tolerance", relative_power),
            ("absolute energy tolerance", absolute_energy_j),
            ("relative energy tolerance", relative_energy),
            ("absolute mass tolerance", absolute_mass_kg),
            ("relative mass tolerance", relative_mass),
        ] {
            if !value.is_finite() || value < 0.0 {
                return Err(EnergyVerificationError::InvalidTolerance { name, value });
            }
        }

        Ok(Self {
            absolute_power_w,
            relative_power,
            absolute_energy_j,
            relative_energy,
            absolute_mass_kg,
            relative_mass,
        })
    }

    fn allowed_power_error(self, expected: f64) -> f64 {
        self.absolute_power_w.max(self.relative_power * expected.abs())
    }

    fn allowed_energy_error(self, expected: f64) -> f64 {
        self.absolute_energy_j.max(self.relative_energy * expected.abs())
    }

    fn allowed_mass_error(self, expected: f64) -> f64 {
        self.absolute_mass_kg.max(self.relative_mass * expected.abs())
    }
}

/// One independently recomputed quantity compared with the stored result.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EnergyVerificationCheck {
    quantity: String,
    segment_id: Option<String>,
    expected: f64,
    observed: f64,
    absolute_error: f64,
    allowed_error: f64,
    passed: bool,
}

impl EnergyVerificationCheck {
    pub fn quantity(&self) -> &str {
        &self.quantity
    }

    pub fn segment_id(&self) -> Option<&str> {
        self.segment_id.as_deref()
    }

    pub const fn expected(&self) -> f64 {
        self.expected
    }

    pub const fn observed(&self) -> f64 {
        self.observed
    }

    pub const fn absolute_error(&self) -> f64 {
        self.absolute_error
    }

    pub const fn allowed_error(&self) -> f64 {
        self.allowed_error
    }

    pub const fn passed(&self) -> bool {
        self.passed
    }
}

/// Complete arithmetic-verification receipt for one batch-bound duty cycle.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EnergyVerificationReport {
    segment_count: usize,
    check_count: usize,
    passed: bool,
    batch_ids: Vec<String>,
    flow_evidence_ids: Vec<String>,
    checks: Vec<EnergyVerificationCheck>,
}

impl EnergyVerificationReport {
    pub const fn segment_count(&self) -> usize {
        self.segment_count
    }

    pub const fn check_count(&self) -> usize {
        self.check_count
    }

    pub const fn passed(&self) -> bool {
        self.passed
    }

    pub fn batch_ids(&self) -> &[String] {
        &self.batch_ids
    }

    pub fn flow_evidence_ids(&self) -> &[String] {
        &self.flow_evidence_ids
    }

    pub fn checks(&self) -> &[EnergyVerificationCheck] {
        &self.checks
    }

    pub fn failed_checks(&self) -> impl Iterator<Item = &EnergyVerificationCheck> {
        self.checks.iter().filter(|check| !check.passed())
    }
}

/// Recompute each batch-bound segment and the integrated energy ledger.
///
/// The recomputation intentionally uses only public source inputs and model
/// parameters retained in each segment evaluation. It does not call the
/// production energy evaluator or duty-cycle integration routine.
pub fn verify_batch_bound_duty_cycle(
    result: &BatchBoundDutyCycleResult,
    tolerance: EnergyVerificationTolerance,
) -> Result<EnergyVerificationReport, EnergyVerificationError> {
    let segments = result.segment_results();
    if segments.is_empty() {
        return Err(EnergyVerificationError::EmptyDutyCycle);
    }

    let mut checks = Vec::new();
    let mut batch_ids = Vec::with_capacity(segments.len());
    let mut flow_evidence_ids = Vec::with_capacity(segments.len());

    let mut duration_seconds: i64 = 0;
    let mut fuel_mass_kg = 0.0;
    let mut chemical_input_energy_j = 0.0;
    let mut brake_work_j = 0.0;
    let mut engine_rejected_energy_j = 0.0;
    let mut gross_electrical_energy_j = 0.0;
    let mut generator_loss_energy_j = 0.0;
    let mut auxiliary_energy_j = 0.0;
    let mut net_electrical_energy_j = 0.0;

    for segment in segments {
        let segment_id = segment.segment_id().to_string();
        let duration_s = segment.time_window().duration_seconds();
        if duration_s <= 0 {
            return Err(EnergyVerificationError::InvalidSegmentDuration {
                segment_id,
                duration_seconds: duration_s,
            });
        }
        duration_seconds = duration_seconds
            .checked_add(duration_s)
            .ok_or(EnergyVerificationError::DurationOverflow)?;

        let evaluation = segment.evaluation();
        let fuel_batch = evaluation.fuel_batch();
        let flow = evaluation.fuel_mass_flow();
        let flow_observation = segment.fuel_flow_observation();
        let flow_evidence = flow.evidence();

        if flow_observation.fuel_batch_id() != fuel_batch.batch_id() {
            return Err(EnergyVerificationError::FlowBindingMismatch {
                segment_id: segment_id.clone(),
                reason: "flow observation batch ID differs from retained fuel batch",
            });
        }
        if flow_observation.time_window() != segment.time_window() {
            return Err(EnergyVerificationError::FlowBindingMismatch {
                segment_id: segment_id.clone(),
                reason: "flow observation time window differs from segment window",
            });
        }
        if flow_observation.mass_flow() != flow {
            return Err(EnergyVerificationError::FlowBindingMismatch {
                segment_id: segment_id.clone(),
                reason: "flow observation property differs from evaluated mass-flow property",
            });
        }

        if fuel_batch.batch_id().trim().is_empty()
            || flow_evidence.source_id().trim().is_empty()
            || flow_evidence.source_uri().trim().is_empty()
            || flow_evidence.method().trim().is_empty()
            || flow_evidence.observed_at().trim().is_empty()
        {
            return Err(EnergyVerificationError::MissingEvidenceIdentity { segment_id });
        }

        batch_ids.push(fuel_batch.batch_id().to_string());
        flow_evidence_ids.push(flow_evidence.source_id().to_string());

        let seconds = duration_s as f64;
        let mass_flow = flow.value().get();
        let lower_heating_value = fuel_batch.lower_heating_value().value().get();
        let engine_efficiency = evaluation.engine_model().brake_efficiency().get();
        let generator_model = evaluation.generator_model();
        let generator_efficiency = generator_model.electrical_efficiency().get();
        let auxiliary_power = generator_model.auxiliary_load().get();

        let chemical_input_power_w = mass_flow * lower_heating_value;
        let brake_power_w = chemical_input_power_w * engine_efficiency;
        let engine_rejected_power_w = chemical_input_power_w - brake_power_w;
        let gross_electrical_power_w = brake_power_w * generator_efficiency;
        let generator_loss_power_w = brake_power_w - gross_electrical_power_w;
        let net_electrical_power_w = gross_electrical_power_w - auxiliary_power;

        for (quantity, value) in [
            ("chemical input power", chemical_input_power_w),
            ("brake power", brake_power_w),
            ("engine rejected power", engine_rejected_power_w),
            ("gross electrical power", gross_electrical_power_w),
            ("generator loss power", generator_loss_power_w),
            ("auxiliary power", auxiliary_power),
            ("net electrical power", net_electrical_power_w),
        ] {
            if !value.is_finite() || value < 0.0 {
                return Err(EnergyVerificationError::InvalidRecomputedValue {
                    segment_id: segment_id.clone(),
                    quantity,
                    value,
                });
            }
        }

        let observed = evaluation.result();
        push_check(
            &mut checks,
            Some(segment_id.clone()),
            "chemical input power",
            chemical_input_power_w,
            observed.balance.chemical_input_power.get(),
            tolerance.allowed_power_error(chemical_input_power_w),
        )?;
        push_check(
            &mut checks,
            Some(segment_id.clone()),
            "brake power",
            brake_power_w,
            observed.engine.brake_power.get(),
            tolerance.allowed_power_error(brake_power_w),
        )?;
        push_check(
            &mut checks,
            Some(segment_id.clone()),
            "engine rejected power",
            engine_rejected_power_w,
            observed.engine.rejected_power.get(),
            tolerance.allowed_power_error(engine_rejected_power_w),
        )?;
        push_check(
            &mut checks,
            Some(segment_id.clone()),
            "gross electrical power",
            gross_electrical_power_w,
            observed.generator.gross_electrical_power.get(),
            tolerance.allowed_power_error(gross_electrical_power_w),
        )?;
        push_check(
            &mut checks,
            Some(segment_id.clone()),
            "generator loss power",
            generator_loss_power_w,
            observed.generator.conversion_loss_power.get(),
            tolerance.allowed_power_error(generator_loss_power_w),
        )?;
        push_check(
            &mut checks,
            Some(segment_id.clone()),
            "auxiliary power",
            auxiliary_power,
            observed.generator.auxiliary_load.get(),
            tolerance.allowed_power_error(auxiliary_power),
        )?;
        push_check(
            &mut checks,
            Some(segment_id.clone()),
            "net electrical power",
            net_electrical_power_w,
            observed.generator.net_electrical_power.get(),
            tolerance.allowed_power_error(net_electrical_power_w),
        )?;

        fuel_mass_kg += mass_flow * seconds;
        chemical_input_energy_j += chemical_input_power_w * seconds;
        brake_work_j += brake_power_w * seconds;
        engine_rejected_energy_j += engine_rejected_power_w * seconds;
        gross_electrical_energy_j += gross_electrical_power_w * seconds;
        generator_loss_energy_j += generator_loss_power_w * seconds;
        auxiliary_energy_j += auxiliary_power * seconds;
        net_electrical_energy_j += net_electrical_power_w * seconds;

        for (quantity, value) in [
            ("fuel mass", fuel_mass_kg),
            ("chemical input energy", chemical_input_energy_j),
            ("brake work", brake_work_j),
            ("engine rejected energy", engine_rejected_energy_j),
            ("gross electrical energy", gross_electrical_energy_j),
            ("generator loss energy", generator_loss_energy_j),
            ("auxiliary energy", auxiliary_energy_j),
            ("net electrical energy", net_electrical_energy_j),
        ] {
            if !value.is_finite() || value < 0.0 {
                return Err(EnergyVerificationError::InvalidRecomputedValue {
                    segment_id: segment_id.clone(),
                    quantity,
                    value,
                });
            }
        }
    }

    if duration_seconds != result.total_duration_seconds() {
        return Err(EnergyVerificationError::DurationMismatch {
            expected_seconds: duration_seconds,
            observed_seconds: result.total_duration_seconds(),
        });
    }
    push_check(
        &mut checks,
        None,
        "total fuel mass",
        fuel_mass_kg,
        result.total_fuel_mass_kg(),
        tolerance.allowed_mass_error(fuel_mass_kg),
    )?;
    push_check(
        &mut checks,
        None,
        "chemical input energy",
        chemical_input_energy_j,
        result.chemical_input_energy_j(),
        tolerance.allowed_energy_error(chemical_input_energy_j),
    )?;
    push_check(
        &mut checks,
        None,
        "brake work",
        brake_work_j,
        result.brake_work_j(),
        tolerance.allowed_energy_error(brake_work_j),
    )?;
    push_check(
        &mut checks,
        None,
        "engine rejected energy",
        engine_rejected_energy_j,
        result.engine_rejected_energy_j(),
        tolerance.allowed_energy_error(engine_rejected_energy_j),
    )?;
    push_check(
        &mut checks,
        None,
        "gross electrical energy",
        gross_electrical_energy_j,
        result.gross_electrical_energy_j(),
        tolerance.allowed_energy_error(gross_electrical_energy_j),
    )?;
    push_check(
        &mut checks,
        None,
        "generator loss energy",
        generator_loss_energy_j,
        result.generator_loss_energy_j(),
        tolerance.allowed_energy_error(generator_loss_energy_j),
    )?;
    push_check(
        &mut checks,
        None,
        "auxiliary energy",
        auxiliary_energy_j,
        result.auxiliary_energy_j(),
        tolerance.allowed_energy_error(auxiliary_energy_j),
    )?;
    push_check(
        &mut checks,
        None,
        "net electrical energy",
        net_electrical_energy_j,
        result.net_electrical_energy_j(),
        tolerance.allowed_energy_error(net_electrical_energy_j),
    )?;

    let accounted_energy_j = engine_rejected_energy_j
        + generator_loss_energy_j
        + auxiliary_energy_j
        + net_electrical_energy_j;
    push_check(
        &mut checks,
        None,
        "independent first-law closure",
        chemical_input_energy_j,
        accounted_energy_j,
        tolerance.allowed_energy_error(chemical_input_energy_j),
    )?;

    let passed = checks.iter().all(EnergyVerificationCheck::passed);
    Ok(EnergyVerificationReport {
        segment_count: segments.len(),
        check_count: checks.len(),
        passed,
        batch_ids,
        flow_evidence_ids,
        checks,
    })
}

fn push_check(
    checks: &mut Vec<EnergyVerificationCheck>,
    segment_id: Option<String>,
    quantity: &str,
    expected: f64,
    observed: f64,
    allowed_error: f64,
) -> Result<(), EnergyVerificationError> {
    if !expected.is_finite()
        || !observed.is_finite()
        || !allowed_error.is_finite()
        || allowed_error < 0.0
    {
        return Err(EnergyVerificationError::NonFiniteCheckValue {
            quantity: quantity.to_string(),
            expected,
            observed,
            allowed_error,
        });
    }

    let absolute_error = (expected - observed).abs();
    checks.push(EnergyVerificationCheck {
        quantity: quantity.to_string(),
        segment_id,
        expected,
        observed,
        absolute_error,
        allowed_error,
        passed: absolute_error <= allowed_error,
    });
    Ok(())
}

#[derive(Debug, Clone, PartialEq)]
pub enum EnergyVerificationError {
    EmptyDutyCycle,
    InvalidTolerance {
        name: &'static str,
        value: f64,
    },
    InvalidSegmentDuration {
        segment_id: String,
        duration_seconds: i64,
    },
    DurationOverflow,
    DurationMismatch {
        expected_seconds: i64,
        observed_seconds: i64,
    },
    MissingEvidenceIdentity {
        segment_id: String,
    },
    FlowBindingMismatch {
        segment_id: String,
        reason: &'static str,
    },
    InvalidRecomputedValue {
        segment_id: String,
        quantity: &'static str,
        value: f64,
    },
    NonFiniteCheckValue {
        quantity: String,
        expected: f64,
        observed: f64,
        allowed_error: f64,
    },
}

impl fmt::Display for EnergyVerificationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyDutyCycle => write!(f, "cannot verify an empty duty cycle"),
            Self::InvalidTolerance { name, value } => {
                write!(f, "{name} must be finite and non-negative; got {value}")
            }
            Self::InvalidSegmentDuration {
                segment_id,
                duration_seconds,
            } => write!(
                f,
                "segment {segment_id} has invalid duration {duration_seconds} seconds"
            ),
            Self::DurationOverflow => write!(f, "total verification duration overflowed"),
            Self::DurationMismatch {
                expected_seconds,
                observed_seconds,
            } => write!(
                f,
                "integrated duration mismatch: expected {expected_seconds} seconds, observed {observed_seconds}"
            ),
            Self::MissingEvidenceIdentity { segment_id } => {
                write!(f, "segment {segment_id} has incomplete batch/flow evidence identity")
            }
            Self::FlowBindingMismatch { segment_id, reason } => {
                write!(f, "segment {segment_id} flow binding mismatch: {reason}")
            }
            Self::InvalidRecomputedValue {
                segment_id,
                quantity,
                value,
            } => write!(
                f,
                "segment {segment_id} recomputed {quantity} is invalid: {value}"
            ),
            Self::NonFiniteCheckValue {
                quantity,
                expected,
                observed,
                allowed_error,
            } => write!(
                f,
                "verification check {quantity} has non-finite inputs: expected={expected}, observed={observed}, allowed={allowed_error}"
            ),
        }
    }
}

impl std::error::Error for EnergyVerificationError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        BatchBoundDutyCycle, BatchDutyCycleSegment, Efficiency, EvidenceClass,
        EvidenceReference, FuelBatch, FuelFamily, FuelFlowObservation, FuelProperty,
        LowerHeatingValueJPerKg, MassFlowKgPerSecond, TimeWindow, Watts, EngineModel,
        GeneratorModel,
    };

    fn evidence(id: &str) -> EvidenceReference {
        EvidenceReference::new(
            id,
            format!("urn:luminous:verify:{id}"),
            "laboratory_measurement",
            "2026-10-01T12:00:00Z",
            EvidenceClass::LaboratoryMeasurement,
        )
        .unwrap()
    }

    fn segment(
        segment_id: &str,
        start: i64,
        end: i64,
        batch_id: &str,
        mass_flow: f64,
        engine_efficiency: f64,
        generator_efficiency: f64,
        auxiliary_w: f64,
    ) -> BatchDutyCycleSegment {
        let batch_lhv = FuelProperty::new(
            LowerHeatingValueJPerKg::new(43_000_000.0).unwrap(),
            Some(100_000.0),
            evidence(&format!("LHV-{batch_id}")),
        )
        .unwrap();
        let batch = FuelBatch::new(
            batch_id,
            FuelFamily::Diesel,
            "synthetic verifier fixture",
            "2026-10-01T12:00:00Z",
            batch_lhv,
            None,
        )
        .unwrap();
        let flow = FuelProperty::new(
            MassFlowKgPerSecond::new(mass_flow).unwrap(),
            Some(0.0001),
            evidence(&format!("FLOW-{segment_id}")),
        )
        .unwrap();
        let window = TimeWindow::new(start, end).unwrap();
        let observation = FuelFlowObservation::new(batch_id, window, flow).unwrap();

        BatchDutyCycleSegment::new(
            segment_id,
            window,
            batch,
            observation,
            EngineModel::new(Efficiency::new(engine_efficiency).unwrap()),
            GeneratorModel::new(
                Efficiency::new(generator_efficiency).unwrap(),
                Watts::new(auxiliary_w).unwrap(),
            ),
        )
        .unwrap()
    }

    fn cycle() -> BatchBoundDutyCycleResult {
        BatchBoundDutyCycle::new(vec![
            segment("SEG-1", 0, 60, "BATCH-1", 0.01, 0.45, 0.96, 5_760.0),
            segment("SEG-2", 60, 90, "BATCH-2", 0.005, 0.40, 0.95, 1_700.0),
        ])
        .unwrap()
        .evaluate()
        .unwrap()
    }

    fn tolerance() -> EnergyVerificationTolerance {
        EnergyVerificationTolerance::new(1e-9, 1e-12, 1e-6, 1e-12, 1e-12, 1e-12).unwrap()
    }

    #[test]
    fn independently_recomputes_every_segment_and_cycle_total() {
        let result = cycle();
        let report = verify_batch_bound_duty_cycle(&result, tolerance()).unwrap();

        assert!(report.passed(), "{:?}", report.failed_checks().collect::<Vec<_>>());
        assert_eq!(report.segment_count(), 2);
        assert_eq!(report.check_count(), 24);
        assert_eq!(report.batch_ids(), &["BATCH-1", "BATCH-2"]);
        assert_eq!(report.flow_evidence_ids(), &["FLOW-SEG-1", "FLOW-SEG-2"]);
    }

    #[test]
    fn detects_tolerance_that_is_too_strict_for_quantized_results() {
        let result = cycle();
        let strict =
            EnergyVerificationTolerance::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0).unwrap();
        let report = verify_batch_bound_duty_cycle(&result, strict).unwrap();
        assert!(report.passed());
    }

    #[test]
    fn rejects_non_finite_or_negative_verifier_tolerance() {
        assert!(
            EnergyVerificationTolerance::new(-1.0, 0.0, 0.0, 0.0, 0.0, 0.0).is_err()
        );
        assert!(
            EnergyVerificationTolerance::new(0.0, f64::NAN, 0.0, 0.0, 0.0, 0.0).is_err()
        );
        assert!(
            EnergyVerificationTolerance::new(0.0, 0.0, 0.0, 0.0, -1.0, 0.0).is_err()
        );
        assert!(
            EnergyVerificationTolerance::new(0.0, 0.0, 0.0, 0.0, 0.0, f64::INFINITY).is_err()
        );
    }
}
