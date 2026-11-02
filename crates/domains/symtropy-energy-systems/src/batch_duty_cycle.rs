// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Batch-bound quasi-steady-state generator duty cycles.
//!
//! Each segment binds a fuel batch to a mass-flow observation and an exact
//! Unix-time window. The observation window must equal the segment window.
//! The model integrates the explicitly supplied steady-state point over that
//! interval; it does not infer transient combustion or controller behavior.

use core::fmt;

use serde::Serialize;

use crate::{
    evaluate_fuel_batch, EngineModel, FuelBatch, FuelBatchEvaluation, FuelProperty,
    GeneratorModel, MassFlowKgPerSecond, ModelError,
};

/// Half-open interval [start, end) in Unix seconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct TimeWindow {
    start_unix_seconds: i64,
    end_unix_seconds: i64,
}

impl TimeWindow {
    pub fn new(
        start_unix_seconds: i64,
        end_unix_seconds: i64,
    ) -> Result<Self, BatchDutyCycleError> {
        if end_unix_seconds <= start_unix_seconds
            || end_unix_seconds.checked_sub(start_unix_seconds).is_none()
        {
            return Err(BatchDutyCycleError::InvalidTimeWindow {
                start_unix_seconds,
                end_unix_seconds,
            });
        }

        Ok(Self {
            start_unix_seconds,
            end_unix_seconds,
        })
    }

    pub const fn start_unix_seconds(self) -> i64 {
        self.start_unix_seconds
    }

    pub const fn end_unix_seconds(self) -> i64 {
        self.end_unix_seconds
    }

    pub fn duration_seconds(self) -> i64 {
        self.end_unix_seconds - self.start_unix_seconds
    }
}

/// A fuel mass-flow observation explicitly associated with one batch and one
/// measurement interval.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FuelFlowObservation {
    fuel_batch_id: String,
    time_window: TimeWindow,
    mass_flow: FuelProperty<MassFlowKgPerSecond>,
}

impl FuelFlowObservation {
    pub fn new(
        fuel_batch_id: impl Into<String>,
        time_window: TimeWindow,
        mass_flow: FuelProperty<MassFlowKgPerSecond>,
    ) -> Result<Self, BatchDutyCycleError> {
        let fuel_batch_id = fuel_batch_id.into();
        if fuel_batch_id.trim().is_empty() {
            return Err(BatchDutyCycleError::MissingFuelBatchId);
        }

        Ok(Self {
            fuel_batch_id,
            time_window,
            mass_flow,
        })
    }

    pub fn fuel_batch_id(&self) -> &str {
        &self.fuel_batch_id
    }

    pub const fn time_window(&self) -> TimeWindow {
        self.time_window
    }

    pub const fn mass_flow(&self) -> &FuelProperty<MassFlowKgPerSecond> {
        &self.mass_flow
    }
}

/// One evidence-bound steady-state operating segment.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BatchDutyCycleSegment {
    segment_id: String,
    time_window: TimeWindow,
    fuel_batch: FuelBatch,
    fuel_flow_observation: FuelFlowObservation,
    engine_model: EngineModel,
    generator_model: GeneratorModel,
}

impl BatchDutyCycleSegment {
    pub fn new(
        segment_id: impl Into<String>,
        time_window: TimeWindow,
        fuel_batch: FuelBatch,
        fuel_flow_observation: FuelFlowObservation,
        engine_model: EngineModel,
        generator_model: GeneratorModel,
    ) -> Result<Self, BatchDutyCycleError> {
        let segment_id = segment_id.into();
        if segment_id.trim().is_empty() {
            return Err(BatchDutyCycleError::MissingSegmentId);
        }

        if fuel_flow_observation.fuel_batch_id() != fuel_batch.batch_id() {
            return Err(BatchDutyCycleError::FuelBatchMismatch {
                expected: fuel_batch.batch_id().to_string(),
                observed: fuel_flow_observation.fuel_batch_id().to_string(),
            });
        }

        if fuel_flow_observation.time_window() != time_window {
            return Err(BatchDutyCycleError::MeasurementWindowMismatch {
                segment: time_window,
                observation: fuel_flow_observation.time_window(),
            });
        }

        Ok(Self {
            segment_id,
            time_window,
            fuel_batch,
            fuel_flow_observation,
            engine_model,
            generator_model,
        })
    }

    pub fn segment_id(&self) -> &str {
        &self.segment_id
    }

    pub const fn time_window(&self) -> TimeWindow {
        self.time_window
    }

    pub fn fuel_batch(&self) -> &FuelBatch {
        &self.fuel_batch
    }

    pub const fn fuel_flow_observation(&self) -> &FuelFlowObservation {
        &self.fuel_flow_observation
    }
}

/// Chronologically ordered, contiguous sequence of batch-bound segments.
///
/// Gaps must be represented explicitly (for example, with a zero-fuel segment)
/// so the total interval cannot silently omit time.
#[derive(Debug, Clone, PartialEq)]
pub struct BatchBoundDutyCycle {
    segments: Vec<BatchDutyCycleSegment>,
}

impl BatchBoundDutyCycle {
    pub fn new(segments: Vec<BatchDutyCycleSegment>) -> Result<Self, BatchDutyCycleError> {
        if segments.is_empty() {
            return Err(BatchDutyCycleError::EmptyDutyCycle);
        }

        let mut seen_ids = std::collections::BTreeSet::new();
        for (index, segment) in segments.iter().enumerate() {
            if !seen_ids.insert(segment.segment_id().to_string()) {
                return Err(BatchDutyCycleError::DuplicateSegmentId {
                    segment_id: segment.segment_id().to_string(),
                });
            }

            if let Some(previous) = index.checked_sub(1).and_then(|i| segments.get(i)) {
                if previous.time_window().end_unix_seconds()
                    != segment.time_window().start_unix_seconds()
                {
                    return Err(BatchDutyCycleError::NonContiguousSegments {
                        previous_end: previous.time_window().end_unix_seconds(),
                        next_start: segment.time_window().start_unix_seconds(),
                    });
                }
            }
        }

        Ok(Self { segments })
    }

    pub fn segments(&self) -> &[BatchDutyCycleSegment] {
        &self.segments
    }

    pub fn evaluate(&self) -> Result<BatchBoundDutyCycleResult, BatchDutyCycleError> {
        let mut segment_results = Vec::with_capacity(self.segments.len());
        let mut total_duration_seconds: i64 = 0;
        let mut total_fuel_mass_kg = 0.0;
        let mut chemical_input_energy_j = 0.0;
        let mut brake_work_j = 0.0;
        let mut engine_rejected_energy_j = 0.0;
        let mut gross_electrical_energy_j = 0.0;
        let mut generator_loss_energy_j = 0.0;
        let mut auxiliary_energy_j = 0.0;
        let mut net_electrical_energy_j = 0.0;

        for segment in &self.segments {
            let duration_s = segment.time_window.duration_seconds();
            total_duration_seconds = total_duration_seconds
                .checked_add(duration_s)
                .ok_or(BatchDutyCycleError::IntegratedValueOverflow {
                    quantity: "total duration",
                })?;

            let evaluation = evaluate_fuel_batch(
                &segment.fuel_batch,
                segment.fuel_flow_observation.mass_flow().clone(),
                segment.engine_model,
                segment.generator_model,
            )?;

            let seconds = duration_s as f64;
            let result = evaluation.result();

            total_fuel_mass_kg +=
                segment.fuel_flow_observation.mass_flow().value().get() * seconds;
            chemical_input_energy_j += result.balance.chemical_input_power.get() * seconds;
            brake_work_j += result.engine.brake_power.get() * seconds;
            engine_rejected_energy_j += result.engine.rejected_power.get() * seconds;
            gross_electrical_energy_j +=
                result.generator.gross_electrical_power.get() * seconds;
            generator_loss_energy_j +=
                result.generator.conversion_loss_power.get() * seconds;
            auxiliary_energy_j += result.generator.auxiliary_load.get() * seconds;
            net_electrical_energy_j += result.generator.net_electrical_power.get() * seconds;

            for (quantity, value) in [
                ("fuel mass", total_fuel_mass_kg),
                ("chemical input energy", chemical_input_energy_j),
                ("brake work", brake_work_j),
                ("engine rejected energy", engine_rejected_energy_j),
                ("gross electrical energy", gross_electrical_energy_j),
                ("generator loss energy", generator_loss_energy_j),
                ("auxiliary energy", auxiliary_energy_j),
                ("net electrical energy", net_electrical_energy_j),
            ] {
                if !value.is_finite() || value < 0.0 {
                    return Err(BatchDutyCycleError::InvalidIntegratedValue {
                        quantity,
                        value,
                    });
                }
            }

            segment_results.push(BatchDutyCycleSegmentResult {
                segment_id: segment.segment_id.clone(),
                time_window: segment.time_window,
                evaluation,
            });
        }

        Ok(BatchBoundDutyCycleResult {
            segment_results,
            total_duration_seconds,
            total_fuel_mass_kg,
            chemical_input_energy_j,
            brake_work_j,
            engine_rejected_energy_j,
            gross_electrical_energy_j,
            generator_loss_energy_j,
            auxiliary_energy_j,
            net_electrical_energy_j,
        })
    }
}

/// Result for one segment, preserving exact batch, flow and source evidence.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BatchDutyCycleSegmentResult {
    segment_id: String,
    time_window: TimeWindow,
    evaluation: FuelBatchEvaluation,
}

impl BatchDutyCycleSegmentResult {
    pub fn segment_id(&self) -> &str {
        &self.segment_id
    }

    pub const fn time_window(&self) -> TimeWindow {
        self.time_window
    }

    pub const fn evaluation(&self) -> &FuelBatchEvaluation {
        &self.evaluation
    }
}

/// Integrated energy and fuel totals for an evidence-bound duty cycle.
///
/// Values are point estimates. Segment-level input uncertainties remain
/// available in each evaluation, but this type deliberately does not aggregate
/// them because repeated use of a batch-level LHV shares correlated uncertainty.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BatchBoundDutyCycleResult {
    segment_results: Vec<BatchDutyCycleSegmentResult>,
    total_duration_seconds: i64,
    total_fuel_mass_kg: f64,
    chemical_input_energy_j: f64,
    brake_work_j: f64,
    engine_rejected_energy_j: f64,
    gross_electrical_energy_j: f64,
    generator_loss_energy_j: f64,
    auxiliary_energy_j: f64,
    net_electrical_energy_j: f64,
}

impl BatchBoundDutyCycleResult {
    pub fn segment_results(&self) -> &[BatchDutyCycleSegmentResult] {
        &self.segment_results
    }

    pub const fn total_duration_seconds(&self) -> i64 {
        self.total_duration_seconds
    }

    pub const fn total_fuel_mass_kg(&self) -> f64 {
        self.total_fuel_mass_kg
    }

    pub const fn chemical_input_energy_j(&self) -> f64 {
        self.chemical_input_energy_j
    }

    pub const fn brake_work_j(&self) -> f64 {
        self.brake_work_j
    }

    pub const fn engine_rejected_energy_j(&self) -> f64 {
        self.engine_rejected_energy_j
    }

    pub const fn gross_electrical_energy_j(&self) -> f64 {
        self.gross_electrical_energy_j
    }

    pub const fn generator_loss_energy_j(&self) -> f64 {
        self.generator_loss_energy_j
    }

    pub const fn auxiliary_energy_j(&self) -> f64 {
        self.auxiliary_energy_j
    }

    pub const fn net_electrical_energy_j(&self) -> f64 {
        self.net_electrical_energy_j
    }

    pub fn net_electrical_efficiency(&self) -> Option<f64> {
        if self.chemical_input_energy_j == 0.0 {
            None
        } else {
            Some(self.net_electrical_energy_j / self.chemical_input_energy_j)
        }
    }

    pub fn energy_balance_residual_j(&self) -> f64 {
        self.chemical_input_energy_j
            - self.engine_rejected_energy_j
            - self.generator_loss_energy_j
            - self.auxiliary_energy_j
            - self.net_electrical_energy_j
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum BatchDutyCycleError {
    EmptyDutyCycle,
    MissingSegmentId,
    MissingFuelBatchId,
    InvalidTimeWindow {
        start_unix_seconds: i64,
        end_unix_seconds: i64,
    },
    FuelBatchMismatch {
        expected: String,
        observed: String,
    },
    MeasurementWindowMismatch {
        segment: TimeWindow,
        observation: TimeWindow,
    },
    DuplicateSegmentId {
        segment_id: String,
    },
    NonContiguousSegments {
        previous_end: i64,
        next_start: i64,
    },
    IntegratedValueOverflow {
        quantity: &'static str,
    },
    InvalidIntegratedValue {
        quantity: &'static str,
        value: f64,
    },
    Model(ModelError),
}

impl From<ModelError> for BatchDutyCycleError {
    fn from(value: ModelError) -> Self {
        Self::Model(value)
    }
}

impl fmt::Display for BatchDutyCycleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyDutyCycle => write!(f, "batch-bound duty cycle must not be empty"),
            Self::MissingSegmentId => write!(f, "segment ID must not be empty"),
            Self::MissingFuelBatchId => write!(f, "fuel batch ID must not be empty"),
            Self::InvalidTimeWindow {
                start_unix_seconds,
                end_unix_seconds,
            } => write!(
                f,
                "time window must have end > start; got [{start_unix_seconds}, {end_unix_seconds})"
            ),
            Self::FuelBatchMismatch { expected, observed } => write!(
                f,
                "fuel-flow observation batch {observed} does not match segment batch {expected}"
            ),
            Self::MeasurementWindowMismatch {
                segment,
                observation,
            } => write!(
                f,
                "fuel-flow observation window [{}, {}) does not match segment window [{}, {})",
                observation.start_unix_seconds(),
                observation.end_unix_seconds(),
                segment.start_unix_seconds(),
                segment.end_unix_seconds()
            ),
            Self::DuplicateSegmentId { segment_id } => {
                write!(f, "duplicate duty-cycle segment ID {segment_id}")
            }
            Self::NonContiguousSegments {
                previous_end,
                next_start,
            } => write!(
                f,
                "duty-cycle segments must be contiguous; previous end {previous_end}, \
                 next start {next_start}"
            ),
            Self::IntegratedValueOverflow { quantity } => {
                write!(f, "integrated {quantity} overflowed")
            }
            Self::InvalidIntegratedValue { quantity, value } => {
                write!(f, "integrated {quantity} invalid: {value}")
            }
            Self::Model(error) => write!(f, "energy model error: {error}"),
        }
    }
}

impl std::error::Error for BatchDutyCycleError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Efficiency, EvidenceClass, EvidenceReference, FuelFamily, FuelRecordError,
        LowerHeatingValueJPerKg, Watts,
    };

    fn evidence(id: &str) -> EvidenceReference {
        EvidenceReference::new(
            id,
            format!("urn:luminous:test:{id}"),
            "laboratory_measurement",
            "2026-10-01T12:00:00Z",
            EvidenceClass::LaboratoryMeasurement,
        )
        .unwrap()
    }

    fn batch(batch_id: &str) -> FuelBatch {
        let lhv = FuelProperty::new(
            LowerHeatingValueJPerKg::new(43_000_000.0).unwrap(),
            Some(100_000.0),
            evidence("LAB-LHV-001"),
        )
        .unwrap();

        FuelBatch::new(
            batch_id,
            FuelFamily::Diesel,
            "reference grade",
            "2026-10-01T12:00:00Z",
            lhv,
            None,
        )
        .unwrap()
    }

    fn flow(batch_id: &str, start: i64, end: i64, mass_flow: f64) -> FuelFlowObservation {
        let property = FuelProperty::new(
            MassFlowKgPerSecond::new(mass_flow).unwrap(),
            Some(0.0001),
            evidence("LAB-FLOW-001"),
        )
        .unwrap();

        FuelFlowObservation::new(
            batch_id,
            TimeWindow::new(start, end).unwrap(),
            property,
        )
        .unwrap()
    }

    fn models(first_segment: bool) -> (EngineModel, GeneratorModel) {
        let engine_efficiency = if first_segment { 0.45 } else { 0.40 };
        let generator_efficiency = if first_segment { 0.96 } else { 0.95 };
        let auxiliary_power = if first_segment { 5_760.0 } else { 1_700.0 };

        (
            EngineModel::new(Efficiency::new(engine_efficiency).unwrap()),
            GeneratorModel::new(
                Efficiency::new(generator_efficiency).unwrap(),
                Watts::new(auxiliary_power).unwrap(),
            ),
        )
    }

    fn segment(
        id: &str,
        start: i64,
        end: i64,
        batch_id: &str,
        mass_flow: f64,
        first_segment: bool,
    ) -> BatchDutyCycleSegment {
        let fuel_batch = batch(batch_id);
        let observation = flow(batch_id, start, end, mass_flow);
        let (engine, generator) = models(first_segment);

        BatchDutyCycleSegment::new(
            id,
            TimeWindow::new(start, end).unwrap(),
            fuel_batch,
            observation,
            engine,
            generator,
        )
        .unwrap()
    }

    #[test]
    fn integrates_batch_bound_fuel_and_net_electrical_energy() {
        let cycle = BatchBoundDutyCycle::new(vec![
            segment("segment-1", 0, 60, "BATCH-001", 0.01, true),
            segment("segment-2", 60, 90, "BATCH-002", 0.005, false),
        ])
        .unwrap();

        let result = cycle.evaluate().unwrap();

        assert_eq!(result.total_duration_seconds(), 90);
        assert!((result.total_fuel_mass_kg() - 0.75).abs() < 1e-12);
        assert!((result.chemical_input_energy_j() - 32_250_000.0).abs() < 1e-6);
        assert!((result.net_electrical_energy_j() - 13_200_000.0).abs() < 1e-6);
        assert!(result.energy_balance_residual_j().abs() < 1e-6);
    }

    #[test]
    fn preserves_batch_and_flow_evidence_for_each_segment() {
        let cycle = BatchBoundDutyCycle::new(vec![
            segment("segment-1", 0, 60, "BATCH-001", 0.01, true),
            segment("segment-2", 60, 90, "BATCH-002", 0.005, false),
        ])
        .unwrap();

        let result = cycle.evaluate().unwrap();

        assert_eq!(
            result.segment_results()[0]
                .evaluation()
                .fuel_batch()
                .batch_id(),
            "BATCH-001"
        );
        assert_eq!(
            result.segment_results()[1]
                .evaluation()
                .fuel_batch()
                .batch_id(),
            "BATCH-002"
        );
        assert_eq!(
            result.segment_results()[0]
                .evaluation()
                .fuel_mass_flow()
                .evidence()
                .source_id(),
            "LAB-FLOW-001"
        );
    }

    #[test]
    fn rejects_flow_observation_from_different_batch() {
        let fuel_batch = batch("BATCH-A");
        let observation = flow("BATCH-B", 0, 60, 0.01);
        let (engine, generator) = models(true);

        assert!(matches!(
            BatchDutyCycleSegment::new(
                "segment-1",
                TimeWindow::new(0, 60).unwrap(),
                fuel_batch,
                observation,
                engine,
                generator,
            ),
            Err(BatchDutyCycleError::FuelBatchMismatch { .. })
        ));
    }

    #[test]
    fn rejects_flow_observation_for_different_time_window() {
        let fuel_batch = batch("BATCH-A");
        let observation = flow("BATCH-A", 0, 59, 0.01);
        let (engine, generator) = models(true);

        assert!(matches!(
            BatchDutyCycleSegment::new(
                "segment-1",
                TimeWindow::new(0, 60).unwrap(),
                fuel_batch,
                observation,
                engine,
                generator,
            ),
            Err(BatchDutyCycleError::MeasurementWindowMismatch { .. })
        ));
    }

    #[test]
    fn rejects_gaps_overlaps_and_reversed_segment_order() {
        let first = segment("segment-1", 0, 60, "BATCH-A", 0.01, true);
        let gap = segment("segment-2", 61, 90, "BATCH-B", 0.005, false);
        assert!(matches!(
            BatchBoundDutyCycle::new(vec![first.clone(), gap]),
            Err(BatchDutyCycleError::NonContiguousSegments { .. })
        ));

        let second = segment("segment-2", 60, 90, "BATCH-B", 0.005, false);
        assert!(matches!(
            BatchBoundDutyCycle::new(vec![second, first]),
            Err(BatchDutyCycleError::NonContiguousSegments { .. })
        ));
    }

    #[test]
    fn rejects_duplicate_segment_ids_and_empty_cycles() {
        let first = segment("segment-1", 0, 60, "BATCH-A", 0.01, true);
        let duplicate = segment("segment-1", 60, 90, "BATCH-B", 0.005, false);

        assert!(matches!(
            BatchBoundDutyCycle::new(Vec::new()),
            Err(BatchDutyCycleError::EmptyDutyCycle)
        ));
        assert!(matches!(
            BatchBoundDutyCycle::new(vec![first, duplicate]),
            Err(BatchDutyCycleError::DuplicateSegmentId { .. })
        ));
    }

    #[test]
    fn rejects_invalid_time_windows() {
        assert!(matches!(
            TimeWindow::new(60, 60),
            Err(BatchDutyCycleError::InvalidTimeWindow { .. })
        ));
        assert!(matches!(
            TimeWindow::new(61, 60),
            Err(BatchDutyCycleError::InvalidTimeWindow { .. })
        ));
        assert!(matches!(
            TimeWindow::new(i64::MIN, i64::MAX),
            Err(BatchDutyCycleError::InvalidTimeWindow { .. })
        ));
    }

    #[test]
    fn rejects_missing_batch_or_segment_identity() {
        assert!(matches!(
            FuelFlowObservation::new(
                " ",
                TimeWindow::new(0, 60).unwrap(),
                FuelProperty::new(
                    MassFlowKgPerSecond::new(0.01).unwrap(),
                    None,
                    evidence("LAB-FLOW-EMPTY")
                )
                .unwrap()
            ),
            Err(BatchDutyCycleError::MissingFuelBatchId)
        ));
        assert!(matches!(
            BatchDutyCycleSegment::new(
                "",
                TimeWindow::new(0, 60).unwrap(),
                batch("BATCH-A"),
                flow("BATCH-A", 0, 60, 0.01),
                models(true).0,
                models(true).1
            ),
            Err(BatchDutyCycleError::MissingSegmentId)
        ));
    }

    #[test]
    fn rejects_invalid_uncertainty_before_integrating() {
        let invalid: Result<FuelProperty<MassFlowKgPerSecond>, FuelRecordError> =
            FuelProperty::new(
            MassFlowKgPerSecond::new(0.01).unwrap(),
            Some(-0.1),
            evidence("LAB-FLOW-BAD"),
        );
        assert!(invalid.is_err());
    }
}
