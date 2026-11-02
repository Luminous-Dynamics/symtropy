// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Matched-service Fuel Flexibility Efficiency Retention (FFER).
//!
//! Candidates are compared against one frozen net-AC demand profile. Surplus
//! generation is reported but never credited as useful service. Small
//! under-delivery is admitted only within an explicit metering tolerance and
//! the common matched service is the lower delivered energy for each segment.
//!
//! This first implementation applies to the reciprocating-engine / generator
//! duty-cycle model. It does not yet compare DC-native, fuel-cell, linear
//! generator, battery or propulsion-work topologies.

use core::fmt;

use serde::Serialize;

use crate::{BatchBoundDutyCycleResult, TimeWindow};

/// Required net AC energy for one half-open time interval.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct ServiceDemandSegment {
    time_window: TimeWindow,
    required_net_ac_energy_j: f64,
}

impl ServiceDemandSegment {
    pub fn new(
        time_window: TimeWindow,
        required_net_ac_energy_j: f64,
    ) -> Result<Self, MatchedServiceError> {
        if !required_net_ac_energy_j.is_finite() || required_net_ac_energy_j < 0.0 {
            return Err(MatchedServiceError::InvalidDemandEnergy {
                value: required_net_ac_energy_j,
            });
        }

        Ok(Self {
            time_window,
            required_net_ac_energy_j,
        })
    }

    pub const fn time_window(self) -> TimeWindow {
        self.time_window
    }

    pub const fn required_net_ac_energy_j(self) -> f64 {
        self.required_net_ac_energy_j
    }
}

/// Frozen service request shared by all candidate systems.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ServiceDemandProfile {
    profile_id: String,
    version: String,
    segments: Vec<ServiceDemandSegment>,
    total_required_energy_j: f64,
}

impl ServiceDemandProfile {
    pub fn new(
        profile_id: impl Into<String>,
        version: impl Into<String>,
        segments: Vec<ServiceDemandSegment>,
    ) -> Result<Self, MatchedServiceError> {
        let profile_id = profile_id.into();
        let version = version.into();

        if profile_id.trim().is_empty() {
            return Err(MatchedServiceError::MissingProfileId);
        }
        if version.trim().is_empty() {
            return Err(MatchedServiceError::MissingProfileVersion);
        }
        if segments.is_empty() {
            return Err(MatchedServiceError::EmptyProfile);
        }

        let mut total_required_energy_j = 0.0;
        for (index, segment) in segments.iter().enumerate() {
            if let Some(previous) = index.checked_sub(1).and_then(|i| segments.get(i)) {
                if previous.time_window().end_unix_seconds()
                    != segment.time_window().start_unix_seconds()
                {
                    return Err(MatchedServiceError::NonContiguousProfile {
                        previous_end: previous.time_window().end_unix_seconds(),
                        next_start: segment.time_window().start_unix_seconds(),
                    });
                }
            }

            total_required_energy_j += segment.required_net_ac_energy_j();
            if !total_required_energy_j.is_finite() {
                return Err(MatchedServiceError::InvalidDemandEnergy {
                    value: total_required_energy_j,
                });
            }
        }

        if total_required_energy_j <= 0.0 {
            return Err(MatchedServiceError::ZeroTotalDemand);
        }

        Ok(Self {
            profile_id,
            version,
            segments,
            total_required_energy_j,
        })
    }

    pub fn profile_id(&self) -> &str {
        &self.profile_id
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn segments(&self) -> &[ServiceDemandSegment] {
        &self.segments
    }

    pub const fn total_required_energy_j(&self) -> f64 {
        self.total_required_energy_j
    }
}

/// Absolute and relative tolerance for net-AC service matching.
///
/// The permitted deviation for each segment is max(absolute, relative ×
/// required energy). This is a measurement / acceptance tolerance, not a
/// physical reserve margin or a relaxed engineering requirement.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct ServiceTolerance {
    absolute_energy_j: f64,
    relative_fraction: f64,
}

impl ServiceTolerance {
    pub fn new(
        absolute_energy_j: f64,
        relative_fraction: f64,
    ) -> Result<Self, MatchedServiceError> {
        if !absolute_energy_j.is_finite()
            || absolute_energy_j < 0.0
            || !relative_fraction.is_finite()
            || !(0.0..=1.0).contains(&relative_fraction)
        {
            return Err(MatchedServiceError::InvalidTolerance {
                absolute_energy_j,
                relative_fraction,
            });
        }

        Ok(Self {
            absolute_energy_j,
            relative_fraction,
        })
    }

    fn allowed_energy_j(self, required_energy_j: f64) -> f64 {
        self.absolute_energy_j
            .max(self.relative_fraction * required_energy_j)
    }
}

/// One segment's common delivered service and non-credited surplus.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct MatchedServiceSegmentResult {
    time_window: TimeWindow,
    required_energy_j: f64,
    reference_generated_energy_j: f64,
    candidate_generated_energy_j: f64,
    common_matched_service_energy_j: f64,
    reference_surplus_energy_j: f64,
    candidate_surplus_energy_j: f64,
    reference_unserved_energy_j: f64,
    candidate_unserved_energy_j: f64,
}

impl MatchedServiceSegmentResult {
    pub const fn time_window(self) -> TimeWindow {
        self.time_window
    }

    pub const fn required_energy_j(self) -> f64 {
        self.required_energy_j
    }

    pub const fn reference_generated_energy_j(self) -> f64 {
        self.reference_generated_energy_j
    }

    pub const fn candidate_generated_energy_j(self) -> f64 {
        self.candidate_generated_energy_j
    }

    pub const fn common_matched_service_energy_j(self) -> f64 {
        self.common_matched_service_energy_j
    }

    pub const fn reference_surplus_energy_j(self) -> f64 {
        self.reference_surplus_energy_j
    }

    pub const fn candidate_surplus_energy_j(self) -> f64 {
        self.candidate_surplus_energy_j
    }

    pub const fn reference_unserved_energy_j(self) -> f64 {
        self.reference_unserved_energy_j
    }

    pub const fn candidate_unserved_energy_j(self) -> f64 {
        self.candidate_unserved_energy_j
    }
}

/// Candidate results evaluated on exactly the same credited service energy.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MatchedServiceComparison {
    profile_id: String,
    profile_version: String,
    reference_name: String,
    candidate_name: String,
    required_service_energy_j: f64,
    common_matched_service_energy_j: f64,
    reference_chemical_input_energy_j: f64,
    candidate_chemical_input_energy_j: f64,
    reference_surplus_energy_j: f64,
    candidate_surplus_energy_j: f64,
    reference_unserved_energy_j: f64,
    candidate_unserved_energy_j: f64,
    reference_matched_service_efficiency: f64,
    candidate_matched_service_efficiency: f64,
    ffer: f64,
    segments: Vec<MatchedServiceSegmentResult>,
}

impl MatchedServiceComparison {
    pub fn profile_id(&self) -> &str {
        &self.profile_id
    }

    pub fn profile_version(&self) -> &str {
        &self.profile_version
    }

    pub fn reference_name(&self) -> &str {
        &self.reference_name
    }

    pub fn candidate_name(&self) -> &str {
        &self.candidate_name
    }

    pub const fn required_service_energy_j(&self) -> f64 {
        self.required_service_energy_j
    }

    pub const fn common_matched_service_energy_j(&self) -> f64 {
        self.common_matched_service_energy_j
    }

    pub const fn reference_chemical_input_energy_j(&self) -> f64 {
        self.reference_chemical_input_energy_j
    }

    pub const fn candidate_chemical_input_energy_j(&self) -> f64 {
        self.candidate_chemical_input_energy_j
    }

    pub const fn reference_surplus_energy_j(&self) -> f64 {
        self.reference_surplus_energy_j
    }

    pub const fn candidate_surplus_energy_j(&self) -> f64 {
        self.candidate_surplus_energy_j
    }

    pub const fn reference_unserved_energy_j(&self) -> f64 {
        self.reference_unserved_energy_j
    }

    pub const fn candidate_unserved_energy_j(&self) -> f64 {
        self.candidate_unserved_energy_j
    }

    pub const fn reference_matched_service_efficiency(&self) -> f64 {
        self.reference_matched_service_efficiency
    }

    pub const fn candidate_matched_service_efficiency(&self) -> f64 {
        self.candidate_matched_service_efficiency
    }

    /// Candidate matched-service efficiency divided by reference efficiency.
    ///
    /// FFER > 1 means less chemical fuel energy was required by the candidate
    /// for the same credited net-AC service. Surplus generation is not credited.
    pub const fn ffer(&self) -> f64 {
        self.ffer
    }

    pub fn segments(&self) -> &[MatchedServiceSegmentResult] {
        &self.segments
    }
}

/// Compare two batch-bound generator cycles against one frozen net-AC demand.
///
/// A candidate that misses a segment beyond the declared tolerance is
/// ineligible; the function returns an error rather than publishing an
/// efficiency ratio. Both systems must cover the exact same time windows.
pub fn compare_matched_service(
    profile: &ServiceDemandProfile,
    reference_name: impl Into<String>,
    reference: &BatchBoundDutyCycleResult,
    candidate_name: impl Into<String>,
    candidate: &BatchBoundDutyCycleResult,
    tolerance: ServiceTolerance,
) -> Result<MatchedServiceComparison, MatchedServiceError> {
    let reference_name = reference_name.into();
    let candidate_name = candidate_name.into();

    if reference_name.trim().is_empty() {
        return Err(MatchedServiceError::MissingSystemName);
    }
    if candidate_name.trim().is_empty() {
        return Err(MatchedServiceError::MissingSystemName);
    }

    let reference_segments = reference.segment_results();
    let candidate_segments = candidate.segment_results();

    if reference_segments.len() != profile.segments().len()
        || candidate_segments.len() != profile.segments().len()
    {
        return Err(MatchedServiceError::SegmentCountMismatch {
            expected: profile.segments().len(),
            reference: reference_segments.len(),
            candidate: candidate_segments.len(),
        });
    }

    let reference_input = reference.chemical_input_energy_j();
    let candidate_input = candidate.chemical_input_energy_j();
    if !reference_input.is_finite()
        || !candidate_input.is_finite()
        || reference_input <= 0.0
        || candidate_input <= 0.0
    {
        return Err(MatchedServiceError::InvalidChemicalInputEnergy {
            reference: reference_input,
            candidate: candidate_input,
        });
    }

    let mut segments = Vec::with_capacity(profile.segments().len());
    let mut common_matched_service_energy_j = 0.0;
    let mut reference_surplus_energy_j = 0.0;
    let mut candidate_surplus_energy_j = 0.0;
    let mut reference_unserved_energy_j = 0.0;
    let mut candidate_unserved_energy_j = 0.0;

    for index in 0..profile.segments().len() {
        let demand = profile.segments()[index];
        let reference_segment = &reference_segments[index];
        let candidate_segment = &candidate_segments[index];

        if reference_segment.time_window() != demand.time_window()
            || candidate_segment.time_window() != demand.time_window()
        {
            return Err(MatchedServiceError::TimeWindowMismatch {
                index,
                expected: demand.time_window(),
                reference: reference_segment.time_window(),
                candidate: candidate_segment.time_window(),
            });
        }

        let duration_s = demand.time_window().duration_seconds() as f64;
        let reference_power_w = reference_segment
            .evaluation()
            .result()
            .generator
            .net_electrical_power
            .get();
        let candidate_power_w = candidate_segment
            .evaluation()
            .result()
            .generator
            .net_electrical_power
            .get();

        let reference_generated = reference_power_w * duration_s;
        let candidate_generated = candidate_power_w * duration_s;
        if !reference_generated.is_finite()
            || reference_generated < 0.0
            || !candidate_generated.is_finite()
            || candidate_generated < 0.0
        {
            return Err(MatchedServiceError::InvalidGeneratedEnergy {
                index,
                reference: reference_generated,
                candidate: candidate_generated,
            });
        }

        let required = demand.required_net_ac_energy_j();
        let allowed = tolerance.allowed_energy_j(required);
        if required > 0.0 && (!allowed.is_finite() || allowed >= required) {
            return Err(MatchedServiceError::ToleranceExceedsDemand {
                index,
                required_energy_j: required,
                allowed_deficit_energy_j: allowed,
            });
        }
        let reference_deficit = (required - reference_generated).max(0.0);
        let candidate_deficit = (required - candidate_generated).max(0.0);

        if reference_deficit > allowed {
            return Err(MatchedServiceError::ServiceDeficit {
                system: reference_name.clone(),
                index,
                required_energy_j: required,
                generated_energy_j: reference_generated,
                deficit_energy_j: reference_deficit,
                allowed_deficit_energy_j: allowed,
            });
        }
        if candidate_deficit > allowed {
            return Err(MatchedServiceError::ServiceDeficit {
                system: candidate_name.clone(),
                index,
                required_energy_j: required,
                generated_energy_j: candidate_generated,
                deficit_energy_j: candidate_deficit,
                allowed_deficit_energy_j: allowed,
            });
        }

        // Credit only the lower delivered energy, capped at requested service.
        // Excess generation is explicitly reported as surplus and never improves FFER.
        let reference_delivered = reference_generated.min(required);
        let candidate_delivered = candidate_generated.min(required);
        let common_service = reference_delivered.min(candidate_delivered);

        common_matched_service_energy_j += common_service;
        reference_surplus_energy_j += (reference_generated - required).max(0.0);
        candidate_surplus_energy_j += (candidate_generated - required).max(0.0);
        reference_unserved_energy_j += reference_deficit;
        candidate_unserved_energy_j += candidate_deficit;

        segments.push(MatchedServiceSegmentResult {
            time_window: demand.time_window(),
            required_energy_j: required,
            reference_generated_energy_j: reference_generated,
            candidate_generated_energy_j: candidate_generated,
            common_matched_service_energy_j: common_service,
            reference_surplus_energy_j: (reference_generated - required).max(0.0),
            candidate_surplus_energy_j: (candidate_generated - required).max(0.0),
            reference_unserved_energy_j: reference_deficit,
            candidate_unserved_energy_j: candidate_deficit,
        });
    }

    if !common_matched_service_energy_j.is_finite() || common_matched_service_energy_j <= 0.0 {
        return Err(MatchedServiceError::ZeroMatchedService);
    }

    let reference_efficiency = common_matched_service_energy_j / reference_input;
    let candidate_efficiency = common_matched_service_energy_j / candidate_input;
    let ffer = candidate_efficiency / reference_efficiency;

    if !reference_efficiency.is_finite()
        || reference_efficiency <= 0.0
        || !candidate_efficiency.is_finite()
        || candidate_efficiency <= 0.0
        || !ffer.is_finite()
        || ffer <= 0.0
    {
        return Err(MatchedServiceError::InvalidDerivedEfficiency {
            reference: reference_efficiency,
            candidate: candidate_efficiency,
            ffer,
        });
    }

    Ok(MatchedServiceComparison {
        profile_id: profile.profile_id().to_string(),
        profile_version: profile.version().to_string(),
        reference_name,
        candidate_name,
        required_service_energy_j: profile.total_required_energy_j(),
        common_matched_service_energy_j,
        reference_chemical_input_energy_j: reference_input,
        candidate_chemical_input_energy_j: candidate_input,
        reference_surplus_energy_j,
        candidate_surplus_energy_j,
        reference_unserved_energy_j,
        candidate_unserved_energy_j,
        reference_matched_service_efficiency: reference_efficiency,
        candidate_matched_service_efficiency: candidate_efficiency,
        ffer,
        segments,
    })
}

#[derive(Debug, Clone, PartialEq)]
pub enum MatchedServiceError {
    MissingProfileId,
    MissingProfileVersion,
    MissingSystemName,
    EmptyProfile,
    ZeroTotalDemand,
    InvalidDemandEnergy {
        value: f64,
    },
    InvalidTolerance {
        absolute_energy_j: f64,
        relative_fraction: f64,
    },
    NonContiguousProfile {
        previous_end: i64,
        next_start: i64,
    },
    SegmentCountMismatch {
        expected: usize,
        reference: usize,
        candidate: usize,
    },
    TimeWindowMismatch {
        index: usize,
        expected: TimeWindow,
        reference: TimeWindow,
        candidate: TimeWindow,
    },
    InvalidChemicalInputEnergy {
        reference: f64,
        candidate: f64,
    },
    InvalidGeneratedEnergy {
        index: usize,
        reference: f64,
        candidate: f64,
    },
    ServiceDeficit {
        system: String,
        index: usize,
        required_energy_j: f64,
        generated_energy_j: f64,
        deficit_energy_j: f64,
        allowed_deficit_energy_j: f64,
    },
    ToleranceExceedsDemand {
        index: usize,
        required_energy_j: f64,
        allowed_deficit_energy_j: f64,
    },
    ZeroMatchedService,
    InvalidDerivedEfficiency {
        reference: f64,
        candidate: f64,
        ffer: f64,
    },
}

impl fmt::Display for MatchedServiceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingProfileId => write!(f, "service profile ID must not be empty"),
            Self::MissingProfileVersion => write!(f, "service profile version must not be empty"),
            Self::MissingSystemName => write!(f, "system name must not be empty"),
            Self::EmptyProfile => write!(f, "service profile must contain at least one segment"),
            Self::ZeroTotalDemand => write!(f, "total requested service energy must be positive"),
            Self::InvalidDemandEnergy { value } => {
                write!(f, "required service energy must be finite and non-negative; got {value}")
            }
            Self::InvalidTolerance {
                absolute_energy_j,
                relative_fraction,
            } => write!(
                f,
                "service tolerance must be finite and non-negative; got absolute={absolute_energy_j} J, relative={relative_fraction}"
            ),
            Self::NonContiguousProfile {
                previous_end,
                next_start,
            } => write!(
                f,
                "service profile must be contiguous; previous end {previous_end}, next start {next_start}"
            ),
            Self::SegmentCountMismatch {
                expected,
                reference,
                candidate,
            } => write!(
                f,
                "segment count mismatch: expected {expected}, reference {reference}, candidate {candidate}"
            ),
            Self::TimeWindowMismatch {
                index,
                expected,
                reference,
                candidate,
            } => write!(
                f,
                "time-window mismatch at segment {index}: expected [{}, {}), reference [{}, {}), candidate [{}, {})",
                expected.start_unix_seconds(),
                expected.end_unix_seconds(),
                reference.start_unix_seconds(),
                reference.end_unix_seconds(),
                candidate.start_unix_seconds(),
                candidate.end_unix_seconds()
            ),
            Self::InvalidChemicalInputEnergy {
                reference,
                candidate,
            } => write!(
                f,
                "chemical input energy must be finite and positive; reference={reference}, candidate={candidate}"
            ),
            Self::InvalidGeneratedEnergy {
                index,
                reference,
                candidate,
            } => write!(
                f,
                "generated net-AC energy invalid at segment {index}: reference={reference}, candidate={candidate}"
            ),
            Self::ServiceDeficit {
                system,
                index,
                required_energy_j,
                generated_energy_j,
                deficit_energy_j,
                allowed_deficit_energy_j,
            } => write!(
                f,
                "{system} failed service segment {index}: required={required_energy_j} J, generated={generated_energy_j} J, deficit={deficit_energy_j} J, allowed={allowed_deficit_energy_j} J"
            ),
            Self::ToleranceExceedsDemand {
                index,
                required_energy_j,
                allowed_deficit_energy_j,
            } => write!(
                f,
                "tolerance cannot consume the whole service request at segment {index}: required={required_energy_j} J, allowed={allowed_deficit_energy_j} J"
            ),
            Self::ZeroMatchedService => write!(f, "matched useful service must be positive"),
            Self::InvalidDerivedEfficiency {
                reference,
                candidate,
                ffer,
            } => write!(
                f,
                "derived service efficiency / FFER invalid: reference={reference}, candidate={candidate}, FFER={ffer}"
            ),
        }
    }
}

impl std::error::Error for MatchedServiceError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        BatchBoundDutyCycle, BatchDutyCycleSegment, Efficiency, EngineModel, EvidenceClass,
        EvidenceReference, FuelBatch, FuelFamily, FuelFlowObservation, FuelProperty, GeneratorModel,
        LowerHeatingValueJPerKg, MassFlowKgPerSecond, TimeWindow, Watts,
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

    fn cycle(flows: [f64; 2], engine_efficiency: f64) -> BatchBoundDutyCycleResult {
        let windows = [(0, 60), (60, 90)];
        let mut segments = Vec::new();

        for (index, (start, end)) in windows.into_iter().enumerate() {
            let batch_id = format!("BATCH-{index}");
            let time_window = TimeWindow::new(start, end).unwrap();
            let flow = FuelProperty::new(
                MassFlowKgPerSecond::new(flows[index]).unwrap(),
                Some(0.0001),
                evidence(&format!("FLOW-{index}")),
            )
            .unwrap();
            let observation =
                FuelFlowObservation::new(batch_id.clone(), time_window, flow).unwrap();
            let engine = EngineModel::new(Efficiency::new(engine_efficiency).unwrap());
            let generator = GeneratorModel::new(
                Efficiency::new(0.96).unwrap(),
                Watts::new(if index == 0 { 5_760.0 } else { 1_700.0 }).unwrap(),
            );
            segments.push(
                BatchDutyCycleSegment::new(
                    format!("SEGMENT-{index}"),
                    time_window,
                    batch(&batch_id),
                    observation,
                    engine,
                    generator,
                )
                .unwrap(),
            );
        }

        BatchBoundDutyCycle::new(segments)
            .unwrap()
            .evaluate()
            .unwrap()
    }

    fn profile(demands: [f64; 2]) -> ServiceDemandProfile {
        ServiceDemandProfile::new(
            "GEN-VARIABLE-LOAD",
            "v1",
            vec![
                ServiceDemandSegment::new(TimeWindow::new(0, 60).unwrap(), demands[0]).unwrap(),
                ServiceDemandSegment::new(TimeWindow::new(60, 90).unwrap(), demands[1]).unwrap(),
            ],
        )
        .unwrap()
    }

    fn tolerance() -> ServiceTolerance {
        ServiceTolerance::new(0.0, 1e-9).unwrap()
    }

    #[test]
    fn matched_service_ffer_compares_fuel_input_for_identical_service() {
        let reference = cycle([0.01, 0.005], 0.45);
        let candidate = cycle([0.009, 0.0045], 0.50);
        let demand = profile([10_800_000.0, 2_735_400.0]);

        let comparison = compare_matched_service(
            &demand,
            "reference",
            &reference,
            "candidate",
            &candidate,
            tolerance(),
        )
        .unwrap();

        assert!((comparison.common_matched_service_energy_j() - 13_535_400.0).abs() < 1e-6);
        assert!((comparison.reference_chemical_input_energy_j() - 32_250_000.0).abs() < 1e-6);
        assert!((comparison.candidate_chemical_input_energy_j() - 29_025_000.0).abs() < 1e-6);
        assert!((comparison.ffer() - (32_250_000.0 / 29_025_000.0)).abs() < 1e-12);
        assert!(comparison.candidate_surplus_energy_j().abs() < 1e-6);
        assert!(comparison.reference_surplus_energy_j().abs() < 1e-6);
    }

    #[test]
    fn surplus_generation_is_reported_but_never_credited_as_service() {
        let reference = cycle([0.01, 0.005], 0.45);
        let candidate = cycle([0.009, 0.0045], 0.45);
        let demand = profile([9_685_440.0, 2_456_760.0]);

        let comparison = compare_matched_service(
            &demand,
            "reference",
            &reference,
            "candidate",
            &candidate,
            tolerance(),
        )
        .unwrap();

        assert!(
            (comparison.common_matched_service_energy_j() - 12_142_200.0).abs() < 1e-6
        );
        assert!(comparison.reference_surplus_energy_j() > 1_390_000.0);
        assert!(comparison.candidate_surplus_energy_j().abs() < 1e-6);
        assert!(
            comparison.common_matched_service_energy_j()
                <= comparison.required_service_energy_j()
        );
    }

    #[test]
    fn rejects_candidate_that_does_not_meet_required_service() {
        let reference = cycle([0.01, 0.005], 0.45);
        let candidate = cycle([0.008, 0.004], 0.50);
        let demand = profile([10_800_000.0, 2_735_400.0]);

        assert!(matches!(
            compare_matched_service(
                &demand,
                "reference",
                &reference,
                "candidate",
                &candidate,
                tolerance()
            ),
            Err(MatchedServiceError::ServiceDeficit { system, .. })
                if system == "candidate"
        ));
    }

    #[test]
    fn rejects_time_window_mismatch_even_when_total_duration_matches() {
        let reference = cycle([0.01, 0.005], 0.45);
        let candidate = cycle([0.009, 0.0045], 0.50);
        let demand = ServiceDemandProfile::new(
            "shifted",
            "v1",
            vec![
                ServiceDemandSegment::new(TimeWindow::new(1, 61).unwrap(), 9_720_000.0).unwrap(),
                ServiceDemandSegment::new(TimeWindow::new(61, 91).unwrap(), 2_160_000.0).unwrap(),
            ],
        )
        .unwrap();

        assert!(matches!(
            compare_matched_service(
                &demand,
                "reference",
                &reference,
                "candidate",
                &candidate,
                tolerance()
            ),
            Err(MatchedServiceError::TimeWindowMismatch { .. })
        ));
    }

    #[test]
    fn rejects_profile_with_different_segment_count() {
        let reference = cycle([0.01, 0.005], 0.45);
        let candidate = cycle([0.009, 0.0045], 0.50);
        let demand = ServiceDemandProfile::new(
            "wrong-count",
            "v1",
            vec![
                ServiceDemandSegment::new(TimeWindow::new(0, 90).unwrap(), 11_880_000.0)
                    .unwrap(),
            ],
        )
        .unwrap();

        assert!(matches!(
            compare_matched_service(
                &demand,
                "reference",
                &reference,
                "candidate",
                &candidate,
                tolerance()
            ),
            Err(MatchedServiceError::SegmentCountMismatch { .. })
        ));
    }

    #[test]
    fn reports_tolerance_shortfall_without_crediting_undelivered_energy() {
        let reference = cycle([0.01, 0.005], 0.45);
        let candidate = cycle([0.008999999, 0.004499999], 0.50);
        let demand = profile([10_800_000.0, 2_735_400.0]);
        let tolerance = ServiceTolerance::new(100.0, 0.0).unwrap();

        let comparison = compare_matched_service(
            &demand,
            "reference",
            &reference,
            "candidate",
            &candidate,
            tolerance,
        )
        .unwrap();

        assert!(comparison.candidate_unserved_energy_j() > 0.0);
        assert!(comparison.common_matched_service_energy_j() < demand.total_required_energy_j());
    }

    #[test]
    fn rejects_tolerance_that_can_hide_a_complete_service_failure() {
        let reference = cycle([0.01, 0.005], 0.45);
        let candidate = cycle([0.0, 0.0], 0.50);
        let demand = profile([10_800_000.0, 2_735_400.0]);

        let tolerance = ServiceTolerance::new(20_000_000.0, 0.0).unwrap();
        assert!(matches!(
            compare_matched_service(
                &demand,
                "reference",
                &reference,
                "candidate",
                &candidate,
                tolerance
            ),
            Err(MatchedServiceError::ToleranceExceedsDemand { .. })
        ));
    }

    #[test]
    fn rejects_invalid_profile_and_tolerance_inputs() {
        assert!(matches!(
            ServiceDemandProfile::new(" ", "v1", vec![]),
            Err(MatchedServiceError::MissingProfileId)
        ));
        assert!(matches!(
            ServiceDemandProfile::new(
                "zero",
                "v1",
                vec![ServiceDemandSegment::new(TimeWindow::new(0, 60).unwrap(), 0.0).unwrap()]
            ),
            Err(MatchedServiceError::ZeroTotalDemand)
        ));
        assert!(ServiceTolerance::new(-1.0, 0.0).is_err());
        assert!(ServiceTolerance::new(0.0, f64::NAN).is_err());
        assert!(ServiceDemandSegment::new(TimeWindow::new(0, 60).unwrap(), f64::INFINITY).is_err());
    }
}
