// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Evidence-aware validation for AI data-centre electrical/workload traces.
//!
//! The validator prevents two common benchmark errors:
//! - summing nested meter readings (e.g. facility PCC plus its UPS output);
//! - computing cross-site correlations from clocks whose measured alignment
//!   error exceeds the experiment's declared tolerance.
//!
//! It validates trace structure and measurement lineage only. It does not
//! infer missing samples, estimate grid response, or certify physical meters.

use core::fmt;
use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

/// Clock source used to align samples from different facilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum TraceClockType {
    MonotonicHardware,
    PtpSynchronized,
    NtpSynchronized,
    WallClock,
    Synthetic,
}

/// Clock quality and nominal sampling interval for one trace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TraceTimebase {
    clock_id: String,
    clock_type: TraceClockType,
    nominal_sample_interval_ns: u64,
    measured_alignment_error_ns: u64,
}

impl TraceTimebase {
    pub fn new(
        clock_id: impl Into<String>,
        clock_type: TraceClockType,
        nominal_sample_interval_ns: u64,
        measured_alignment_error_ns: u64,
    ) -> Result<Self, PowerTraceError> {
        let clock_id = clock_id.into();
        if clock_id.trim().is_empty() {
            return Err(PowerTraceError::MissingIdentity {
                field: "clock ID",
            });
        }
        if nominal_sample_interval_ns == 0 {
            return Err(PowerTraceError::InvalidSampleInterval);
        }

        Ok(Self {
            clock_id,
            clock_type,
            nominal_sample_interval_ns,
            measured_alignment_error_ns,
        })
    }

    pub fn clock_id(&self) -> &str {
        &self.clock_id
    }

    pub const fn clock_type(&self) -> TraceClockType {
        self.clock_type
    }

    pub const fn nominal_sample_interval_ns(&self) -> u64 {
        self.nominal_sample_interval_ns
    }

    pub const fn measured_alignment_error_ns(&self) -> u64 {
        self.measured_alignment_error_ns
    }

    const fn supports_cross_site_alignment(&self) -> bool {
        matches!(
            self.clock_type,
            TraceClockType::PtpSynchronized | TraceClockType::NtpSynchronized
        )
    }
}

/// Electrical measurement boundary represented by a meter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum PowerMeasurementBoundary {
    AcceleratorRail,
    NodeInput,
    RackPdu,
    UpsOutput,
    FacilityPcc,
    CoolingPlant,
    FacilityAuxiliary,
}

/// Whether a meter is an aggregate or a nested component meter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum PowerMeasurementRole {
    ComponentMeter,
    SubsystemMeter,
    AggregateMeter,
}

/// Sign convention attached to a power reading.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum PowerSignConvention {
    PositiveImport,
    PositiveExport,
    BidirectionalSigned,
}

/// A site participating in the trace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TraceSite {
    site_id: String,
    grid_region: String,
}

impl TraceSite {
    pub fn new(
        site_id: impl Into<String>,
        grid_region: impl Into<String>,
    ) -> Result<Self, PowerTraceError> {
        let site_id = site_id.into();
        let grid_region = grid_region.into();
        if site_id.trim().is_empty() {
            return Err(PowerTraceError::MissingIdentity { field: "site ID" });
        }
        if grid_region.trim().is_empty() {
            return Err(PowerTraceError::MissingIdentity {
                field: "grid region",
            });
        }
        Ok(Self {
            site_id,
            grid_region,
        })
    }

    pub fn site_id(&self) -> &str {
        &self.site_id
    }

    pub fn grid_region(&self) -> &str {
        &self.grid_region
    }
}

/// A meter with explicit hierarchy, calibration and evidence identity.
///
/// `parent_measurement_point_id` points toward the containing upstream
/// meter. For example, a node meter may be a child of a rack meter, which is
/// a child of the facility point of connection (PCC).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PowerMeasurementPoint {
    measurement_point_id: String,
    site_id: String,
    parent_measurement_point_id: Option<String>,
    boundary: PowerMeasurementBoundary,
    role: PowerMeasurementRole,
    sign_convention: PowerSignConvention,
    calibration_id: String,
    evidence_id: String,
}

impl PowerMeasurementPoint {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        measurement_point_id: impl Into<String>,
        site_id: impl Into<String>,
        parent_measurement_point_id: Option<String>,
        boundary: PowerMeasurementBoundary,
        role: PowerMeasurementRole,
        sign_convention: PowerSignConvention,
        calibration_id: impl Into<String>,
        evidence_id: impl Into<String>,
    ) -> Result<Self, PowerTraceError> {
        let measurement_point_id = measurement_point_id.into();
        let site_id = site_id.into();
        let calibration_id = calibration_id.into();
        let evidence_id = evidence_id.into();

        for (field, value) in [
            ("measurement point ID", measurement_point_id.as_str()),
            ("site ID", site_id.as_str()),
            ("calibration ID", calibration_id.as_str()),
            ("measurement evidence ID", evidence_id.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(PowerTraceError::MissingIdentity { field });
            }
        }

        if parent_measurement_point_id
            .as_ref()
            .is_some_and(|parent| parent.trim().is_empty())
        {
            return Err(PowerTraceError::MissingIdentity {
                field: "parent measurement point ID",
            });
        }

        Ok(Self {
            measurement_point_id,
            site_id,
            parent_measurement_point_id,
            boundary,
            role,
            sign_convention,
            calibration_id,
            evidence_id,
        })
    }

    pub fn measurement_point_id(&self) -> &str {
        &self.measurement_point_id
    }

    pub fn site_id(&self) -> &str {
        &self.site_id
    }

    pub fn parent_measurement_point_id(&self) -> Option<&str> {
        self.parent_measurement_point_id.as_deref()
    }

    pub const fn boundary(&self) -> PowerMeasurementBoundary {
        self.boundary
    }

    pub const fn role(&self) -> PowerMeasurementRole {
        self.role
    }

    pub const fn sign_convention(&self) -> PowerSignConvention {
        self.sign_convention
    }

    pub fn calibration_id(&self) -> &str {
        &self.calibration_id
    }

    pub fn evidence_id(&self) -> &str {
        &self.evidence_id
    }
}

/// One workload's declared site and service contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TraceWorkloadSession {
    workload_id: String,
    site_id: String,
    sla_contract_id: String,
}

impl TraceWorkloadSession {
    pub fn new(
        workload_id: impl Into<String>,
        site_id: impl Into<String>,
        sla_contract_id: impl Into<String>,
    ) -> Result<Self, PowerTraceError> {
        let workload_id = workload_id.into();
        let site_id = site_id.into();
        let sla_contract_id = sla_contract_id.into();

        for (field, value) in [
            ("workload ID", workload_id.as_str()),
            ("workload site ID", site_id.as_str()),
            ("SLA contract ID", sla_contract_id.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(PowerTraceError::MissingIdentity { field });
            }
        }

        Ok(Self {
            workload_id,
            site_id,
            sla_contract_id,
        })
    }

    pub fn workload_id(&self) -> &str {
        &self.workload_id
    }

    pub fn site_id(&self) -> &str {
        &self.site_id
    }

    pub fn sla_contract_id(&self) -> &str {
        &self.sla_contract_id
    }
}

/// A timestamped power/workload observation at exactly one meter boundary.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PowerTraceSample {
    elapsed_ns: u64,
    site_id: String,
    measurement_point_id: String,
    power_w: f64,
    workload_id: Option<String>,
    evidence_id: String,
}

impl PowerTraceSample {
    pub fn new(
        elapsed_ns: u64,
        site_id: impl Into<String>,
        measurement_point_id: impl Into<String>,
        power_w: f64,
        workload_id: Option<String>,
        evidence_id: impl Into<String>,
    ) -> Result<Self, PowerTraceError> {
        let site_id = site_id.into();
        let measurement_point_id = measurement_point_id.into();
        let evidence_id = evidence_id.into();

        if site_id.trim().is_empty() {
            return Err(PowerTraceError::MissingIdentity {
                field: "sample site ID",
            });
        }
        if measurement_point_id.trim().is_empty() {
            return Err(PowerTraceError::MissingIdentity {
                field: "sample measurement point ID",
            });
        }
        if evidence_id.trim().is_empty() {
            return Err(PowerTraceError::MissingIdentity {
                field: "sample evidence ID",
            });
        }
        if !power_w.is_finite() {
            return Err(PowerTraceError::InvalidPower {
                measurement_point_id,
                value: power_w,
            });
        }

        Ok(Self {
            elapsed_ns,
            site_id,
            measurement_point_id,
            power_w,
            workload_id,
            evidence_id,
        })
    }

    pub const fn elapsed_ns(&self) -> u64 {
        self.elapsed_ns
    }

    pub fn site_id(&self) -> &str {
        &self.site_id
    }

    pub fn measurement_point_id(&self) -> &str {
        &self.measurement_point_id
    }

    pub const fn power_w(&self) -> f64 {
        self.power_w
    }

    pub fn workload_id(&self) -> Option<&str> {
        self.workload_id.as_deref()
    }

    pub fn evidence_id(&self) -> &str {
        &self.evidence_id
    }
}

/// Full source trace before any aggregation or correlation calculation.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DataCenterPowerTrace {
    trace_id: String,
    timebase: TraceTimebase,
    sites: Vec<TraceSite>,
    measurement_points: Vec<PowerMeasurementPoint>,
    workloads: Vec<TraceWorkloadSession>,
    samples: Vec<PowerTraceSample>,
}

impl DataCenterPowerTrace {
    pub fn new(
        trace_id: impl Into<String>,
        timebase: TraceTimebase,
        sites: Vec<TraceSite>,
        measurement_points: Vec<PowerMeasurementPoint>,
        workloads: Vec<TraceWorkloadSession>,
        samples: Vec<PowerTraceSample>,
    ) -> Result<Self, PowerTraceError> {
        let trace_id = trace_id.into();
        if trace_id.trim().is_empty() {
            return Err(PowerTraceError::MissingIdentity { field: "trace ID" });
        }
        if sites.is_empty() || measurement_points.is_empty() || samples.is_empty() {
            return Err(PowerTraceError::EmptyTrace);
        }

        let trace = Self {
            trace_id,
            timebase,
            sites,
            measurement_points,
            workloads,
            samples,
        };
        trace.validate()?;
        Ok(trace)
    }

    pub fn trace_id(&self) -> &str {
        &self.trace_id
    }

    pub const fn timebase(&self) -> &TraceTimebase {
        &self.timebase
    }

    pub fn sites(&self) -> &[TraceSite] {
        &self.sites
    }

    pub fn measurement_points(&self) -> &[PowerMeasurementPoint] {
        &self.measurement_points
    }

    pub fn workloads(&self) -> &[TraceWorkloadSession] {
        &self.workloads
    }

    pub fn samples(&self) -> &[PowerTraceSample] {
        &self.samples
    }

    /// Validate identities, meter parentage, workload joins and sample order.
    pub fn validate(&self) -> Result<PowerTraceValidationReport, PowerTraceError> {
        let site_map = unique_map(
            self.sites.iter().map(|site| (site.site_id(), site)),
            "site ID",
        )?;
        let point_map = unique_map(
            self.measurement_points
                .iter()
                .map(|point| (point.measurement_point_id(), point)),
            "measurement point ID",
        )?;
        let workload_map = unique_map(
            self.workloads
                .iter()
                .map(|workload| (workload.workload_id(), workload)),
            "workload ID",
        )?;

        for point in &self.measurement_points {
            if !site_map.contains_key(point.site_id()) {
                return Err(PowerTraceError::UnknownSite {
                    identity: point.measurement_point_id().to_string(),
                    site_id: point.site_id().to_string(),
                });
            }
            if let Some(parent_id) = point.parent_measurement_point_id() {
                let parent = point_map.get(parent_id).ok_or_else(|| {
                    PowerTraceError::UnknownParent {
                        measurement_point_id: point.measurement_point_id().to_string(),
                        parent_id: parent_id.to_string(),
                    }
                })?;
                if parent.site_id() != point.site_id() {
                    return Err(PowerTraceError::ParentSiteMismatch {
                        measurement_point_id: point.measurement_point_id().to_string(),
                        parent_id: parent_id.to_string(),
                    });
                }
            }
        }

        self.validate_parent_cycles(&point_map)?;

        for workload in &self.workloads {
            if !site_map.contains_key(workload.site_id()) {
                return Err(PowerTraceError::UnknownSite {
                    identity: workload.workload_id().to_string(),
                    site_id: workload.site_id().to_string(),
                });
            }
        }

        let mut last_timestamp_by_point = BTreeMap::<String, u64>::new();
        for sample in &self.samples {
            let point = point_map
                .get(sample.measurement_point_id())
                .ok_or_else(|| PowerTraceError::UnknownMeasurementPoint {
                    measurement_point_id: sample.measurement_point_id().to_string(),
                })?;

            if point.site_id() != sample.site_id() {
                return Err(PowerTraceError::SampleSiteMismatch {
                    measurement_point_id: sample.measurement_point_id().to_string(),
                    expected_site: point.site_id().to_string(),
                    observed_site: sample.site_id().to_string(),
                });
            }

            if !sample.power_w().is_finite() {
                return Err(PowerTraceError::InvalidPower {
                    measurement_point_id: sample.measurement_point_id().to_string(),
                    value: sample.power_w(),
                });
            }

            if sample.power_w() < 0.0
                && point.sign_convention() != PowerSignConvention::BidirectionalSigned
            {
                return Err(PowerTraceError::UnexpectedNegativePower {
                    measurement_point_id: sample.measurement_point_id().to_string(),
                    value: sample.power_w(),
                });
            }

            if let Some(workload_id) = sample.workload_id() {
                let workload = workload_map.get(workload_id).ok_or_else(|| {
                    PowerTraceError::UnknownWorkload {
                        workload_id: workload_id.to_string(),
                    }
                })?;
                if workload.site_id() != sample.site_id() {
                    return Err(PowerTraceError::WorkloadSiteMismatch {
                        workload_id: workload_id.to_string(),
                        sample_site: sample.site_id().to_string(),
                        workload_site: workload.site_id().to_string(),
                    });
                }
            }

            if let Some(previous) =
                last_timestamp_by_point.insert(sample.measurement_point_id().to_string(), sample.elapsed_ns())
                && sample.elapsed_ns() <= previous
            {
                return Err(PowerTraceError::NonMonotonicTimestamp {
                    measurement_point_id: sample.measurement_point_id().to_string(),
                    previous_elapsed_ns: previous,
                    next_elapsed_ns: sample.elapsed_ns(),
                });
            }
        }

        Ok(PowerTraceValidationReport {
            trace_id: self.trace_id.clone(),
            site_count: self.sites.len(),
            measurement_point_count: self.measurement_points.len(),
            workload_count: self.workloads.len(),
            sample_count: self.samples.len(),
        })
    }

    /// Validate a chosen customer boundary and its meter set before summing
    /// or correlating readings. Nested meters may never appear together.
    pub fn validate_aggregation(
        &self,
        selection: &PowerAggregationSelection,
    ) -> Result<(), PowerTraceError> {
        if selection.measurement_point_ids().is_empty() {
            return Err(PowerTraceError::EmptyAggregationSelection);
        }

        let point_map = self
            .measurement_points
            .iter()
            .map(|point| (point.measurement_point_id(), point))
            .collect::<BTreeMap<_, _>>();

        let mut seen = BTreeSet::new();
        let mut selected_points = Vec::with_capacity(selection.measurement_point_ids().len());
        let mut selected_sites = BTreeSet::new();

        for point_id in selection.measurement_point_ids() {
            if !seen.insert(point_id) {
                return Err(PowerTraceError::DuplicateAggregationPoint {
                    measurement_point_id: point_id.clone(),
                });
            }
            let point = point_map.get(point_id.as_str()).ok_or_else(|| {
                PowerTraceError::UnknownMeasurementPoint {
                    measurement_point_id: point_id.clone(),
                }
            })?;

            if point.boundary() != selection.boundary() {
                return Err(PowerTraceError::AggregationBoundaryMismatch {
                    measurement_point_id: point_id.clone(),
                });
            }

            selected_sites.insert(point.site_id().to_string());
            selected_points.push(*point);
        }

        for ancestor in &selected_points {
            for descendant in &selected_points {
                if ancestor.measurement_point_id() != descendant.measurement_point_id()
                    && Self::is_ancestor(ancestor.measurement_point_id(), descendant, &point_map)
                {
                    return Err(PowerTraceError::NestedMeterDoubleCount {
                        ancestor_id: ancestor.measurement_point_id().to_string(),
                        descendant_id: descendant.measurement_point_id().to_string(),
                    });
                }
            }
        }

        if selected_sites.len() > 1 {
            if !self.timebase.supports_cross_site_alignment() {
                return Err(PowerTraceError::UnsynchronizedClock {
                    clock_id: self.timebase.clock_id().to_string(),
                });
            }
            if self.timebase.measured_alignment_error_ns()
                > selection.maximum_clock_alignment_error_ns()
            {
                return Err(PowerTraceError::ClockAlignmentExceeded {
                    measured_error_ns: self.timebase.measured_alignment_error_ns(),
                    maximum_error_ns: selection.maximum_clock_alignment_error_ns(),
                });
            }

            let common_samples = self.common_sample_timestamps(selection.measurement_point_ids());
            if common_samples < selection.minimum_common_samples() {
                return Err(PowerTraceError::InsufficientCommonSamples {
                    observed: common_samples,
                    required: selection.minimum_common_samples(),
                });
            }
        }

        Ok(())
    }

    fn common_sample_timestamps(&self, selected_point_ids: &[String]) -> usize {
        let mut common: Option<BTreeSet<u64>> = None;
        for point_id in selected_point_ids {
            let timestamps = self
                .samples
                .iter()
                .filter(|sample| sample.measurement_point_id() == point_id)
                .map(PowerTraceSample::elapsed_ns)
                .collect::<BTreeSet<_>>();
            common = Some(match common {
                Some(previous) => previous.intersection(&timestamps).copied().collect(),
                None => timestamps,
            });
        }
        common.map_or(0, |timestamps| timestamps.len())
    }

    fn validate_parent_cycles(
        &self,
        point_map: &BTreeMap<&str, &PowerMeasurementPoint>,
    ) -> Result<(), PowerTraceError> {
        for point in &self.measurement_points {
            let mut visited = BTreeSet::new();
            let mut current = Some(point.measurement_point_id());

            while let Some(current_id) = current {
                if !visited.insert(current_id.to_string()) {
                    return Err(PowerTraceError::MeasurementHierarchyCycle {
                        measurement_point_id: point.measurement_point_id().to_string(),
                    });
                }
                current = point_map
                    .get(current_id)
                    .and_then(|ancestor| ancestor.parent_measurement_point_id());
            }
        }
        Ok(())
    }

    fn is_ancestor(
        ancestor_id: &str,
        descendant: &PowerMeasurementPoint,
        point_map: &BTreeMap<&str, &PowerMeasurementPoint>,
    ) -> bool {
        let mut current = descendant.parent_measurement_point_id();
        while let Some(current_id) = current {
            if current_id == ancestor_id {
                return true;
            }
            current = point_map
                .get(current_id)
                .and_then(|point| point.parent_measurement_point_id());
        }
        false
    }
}

/// Selection of mutually non-nested meters at one declared service boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PowerAggregationSelection {
    boundary: PowerMeasurementBoundary,
    measurement_point_ids: Vec<String>,
    maximum_clock_alignment_error_ns: u64,
    minimum_common_samples: usize,
}

impl PowerAggregationSelection {
    pub fn new(
        boundary: PowerMeasurementBoundary,
        measurement_point_ids: Vec<String>,
        maximum_clock_alignment_error_ns: u64,
        minimum_common_samples: usize,
    ) -> Result<Self, PowerTraceError> {
        if measurement_point_ids.is_empty() {
            return Err(PowerTraceError::EmptyAggregationSelection);
        }
        if minimum_common_samples < 2 {
            return Err(PowerTraceError::InvalidMinimumCommonSamples {
                value: minimum_common_samples,
            });
        }
        for id in &measurement_point_ids {
            if id.trim().is_empty() {
                return Err(PowerTraceError::MissingIdentity {
                    field: "aggregation measurement point ID",
                });
            }
        }
        Ok(Self {
            boundary,
            measurement_point_ids,
            maximum_clock_alignment_error_ns,
            minimum_common_samples,
        })
    }

    pub const fn boundary(&self) -> PowerMeasurementBoundary {
        self.boundary
    }

    pub fn measurement_point_ids(&self) -> &[String] {
        &self.measurement_point_ids
    }

    pub const fn maximum_clock_alignment_error_ns(&self) -> u64 {
        self.maximum_clock_alignment_error_ns
    }

    pub const fn minimum_common_samples(&self) -> usize {
        self.minimum_common_samples
    }
}

/// Structural validation receipt for one trace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PowerTraceValidationReport {
    trace_id: String,
    site_count: usize,
    measurement_point_count: usize,
    workload_count: usize,
    sample_count: usize,
}

impl PowerTraceValidationReport {
    pub fn trace_id(&self) -> &str {
        &self.trace_id
    }

    pub const fn site_count(&self) -> usize {
        self.site_count
    }

    pub const fn measurement_point_count(&self) -> usize {
        self.measurement_point_count
    }

    pub const fn workload_count(&self) -> usize {
        self.workload_count
    }

    pub const fn sample_count(&self) -> usize {
        self.sample_count
    }
}

fn unique_map<'a, T>(
    entries: impl Iterator<Item = (&'a str, &'a T)>,
    kind: &'static str,
) -> Result<BTreeMap<&'a str, &'a T>, PowerTraceError> {
    let mut map = BTreeMap::new();
    for (id, value) in entries {
        if map.insert(id, value).is_some() {
            return Err(PowerTraceError::DuplicateIdentity {
                kind,
                identity: id.to_string(),
            });
        }
    }
    Ok(map)
}

#[derive(Debug, Clone, PartialEq)]
pub enum PowerTraceError {
    MissingIdentity {
        field: &'static str,
    },
    EmptyTrace,
    InvalidSampleInterval,
    DuplicateIdentity {
        kind: &'static str,
        identity: String,
    },
    UnknownSite {
        identity: String,
        site_id: String,
    },
    UnknownParent {
        measurement_point_id: String,
        parent_id: String,
    },
    ParentSiteMismatch {
        measurement_point_id: String,
        parent_id: String,
    },
    MeasurementHierarchyCycle {
        measurement_point_id: String,
    },
    UnknownMeasurementPoint {
        measurement_point_id: String,
    },
    SampleSiteMismatch {
        measurement_point_id: String,
        expected_site: String,
        observed_site: String,
    },
    InvalidPower {
        measurement_point_id: String,
        value: f64,
    },
    UnexpectedNegativePower {
        measurement_point_id: String,
        value: f64,
    },
    UnknownWorkload {
        workload_id: String,
    },
    WorkloadSiteMismatch {
        workload_id: String,
        sample_site: String,
        workload_site: String,
    },
    NonMonotonicTimestamp {
        measurement_point_id: String,
        previous_elapsed_ns: u64,
        next_elapsed_ns: u64,
    },
    EmptyAggregationSelection,
    DuplicateAggregationPoint {
        measurement_point_id: String,
    },
    AggregationBoundaryMismatch {
        measurement_point_id: String,
    },
    NestedMeterDoubleCount {
        ancestor_id: String,
        descendant_id: String,
    },
    UnsynchronizedClock {
        clock_id: String,
    },
    ClockAlignmentExceeded {
        measured_error_ns: u64,
        maximum_error_ns: u64,
    },
    InvalidMinimumCommonSamples {
        value: usize,
    },
    InsufficientCommonSamples {
        observed: usize,
        required: usize,
    },
}

impl fmt::Display for PowerTraceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingIdentity { field } => write!(f, "{field} must not be empty"),
            Self::EmptyTrace => write!(f, "trace requires sites, measurement points and samples"),
            Self::InvalidSampleInterval => write!(f, "nominal sample interval must be positive"),
            Self::DuplicateIdentity { kind, identity } => {
                write!(f, "duplicate {kind}: {identity}")
            }
            Self::UnknownSite { identity, site_id } => {
                write!(f, "{identity} references unknown site {site_id}")
            }
            Self::UnknownParent {
                measurement_point_id,
                parent_id,
            } => write!(
                f,
                "measurement point {measurement_point_id} references unknown parent {parent_id}"
            ),
            Self::ParentSiteMismatch {
                measurement_point_id,
                parent_id,
            } => write!(
                f,
                "measurement point {measurement_point_id} and parent {parent_id} belong to different sites"
            ),
            Self::MeasurementHierarchyCycle {
                measurement_point_id,
            } => write!(
                f,
                "measurement hierarchy contains a cycle involving {measurement_point_id}"
            ),
            Self::UnknownMeasurementPoint {
                measurement_point_id,
            } => write!(f, "unknown measurement point {measurement_point_id}"),
            Self::SampleSiteMismatch {
                measurement_point_id,
                expected_site,
                observed_site,
            } => write!(
                f,
                "sample for {measurement_point_id} belongs to {observed_site}; expected {expected_site}"
            ),
            Self::InvalidPower {
                measurement_point_id,
                value,
            } => write!(
                f,
                "power reading for {measurement_point_id} must be finite; got {value}"
            ),
            Self::UnexpectedNegativePower {
                measurement_point_id,
                value,
            } => write!(
                f,
                "negative power reading for {measurement_point_id} conflicts with sign convention: {value} W"
            ),
            Self::UnknownWorkload { workload_id } => {
                write!(f, "sample references unknown workload {workload_id}")
            }
            Self::WorkloadSiteMismatch {
                workload_id,
                sample_site,
                workload_site,
            } => write!(
                f,
                "workload {workload_id} belongs to {workload_site}, not sample site {sample_site}"
            ),
            Self::NonMonotonicTimestamp {
                measurement_point_id,
                previous_elapsed_ns,
                next_elapsed_ns,
            } => write!(
                f,
                "non-monotonic time at {measurement_point_id}: previous {previous_elapsed_ns}, next {next_elapsed_ns}"
            ),
            Self::EmptyAggregationSelection => {
                write!(f, "aggregation selection must contain at least one meter")
            }
            Self::DuplicateAggregationPoint { measurement_point_id } => {
                write!(f, "aggregation selection duplicates {measurement_point_id}")
            }
            Self::AggregationBoundaryMismatch {
                measurement_point_id,
            } => write!(
                f,
                "measurement point {measurement_point_id} does not match the selected service boundary"
            ),
            Self::NestedMeterDoubleCount {
                ancestor_id,
                descendant_id,
            } => write!(
                f,
                "aggregation includes nested meters {ancestor_id} and {descendant_id}; this would double-count energy"
            ),
            Self::UnsynchronizedClock { clock_id } => write!(
                f,
                "clock {clock_id} is not declared PTP/NTP synchronized for cross-site analysis"
            ),
            Self::ClockAlignmentExceeded {
                measured_error_ns,
                maximum_error_ns,
            } => write!(
                f,
                "cross-site clock alignment error {measured_error_ns} ns exceeds allowed {maximum_error_ns} ns"
            ),
            Self::InvalidMinimumCommonSamples { value } => {
                write!(f, "minimum common sample count must be at least 2; got {value}")
            }
            Self::InsufficientCommonSamples { observed, required } => write!(
                f,
                "cross-site correlation has {observed} common timestamps; {required} required"
            ),
        }
    }
}

impl std::error::Error for PowerTraceError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn site(id: &str) -> TraceSite {
        TraceSite::new(id, "synthetic-grid").unwrap()
    }

    fn point(
        id: &str,
        site_id: &str,
        parent: Option<&str>,
        boundary: PowerMeasurementBoundary,
        role: PowerMeasurementRole,
    ) -> PowerMeasurementPoint {
        PowerMeasurementPoint::new(
            id,
            site_id,
            parent.map(str::to_string),
            boundary,
            role,
            PowerSignConvention::PositiveImport,
            format!("CAL-{id}"),
            format!("EVID-{id}"),
        )
        .unwrap()
    }

    fn sample(
        time: u64,
        site_id: &str,
        point_id: &str,
        power_w: f64,
        workload_id: Option<&str>,
    ) -> PowerTraceSample {
        PowerTraceSample::new(
            time,
            site_id,
            point_id,
            power_w,
            workload_id.map(str::to_string),
            format!("SAMPLE-{point_id}-{time}"),
        )
        .unwrap()
    }

    fn timebase(clock_type: TraceClockType, alignment_error_ns: u64) -> TraceTimebase {
        TraceTimebase::new("CLOCK-1", clock_type, 20_000_000, alignment_error_ns).unwrap()
    }

    fn valid_trace() -> DataCenterPowerTrace {
        DataCenterPowerTrace::new(
            "TRACE-VALID",
            timebase(TraceClockType::PtpSynchronized, 1_000_000),
            vec![site("SITE-A"), site("SITE-B")],
            vec![
                point(
                    "A-PCC",
                    "SITE-A",
                    None,
                    PowerMeasurementBoundary::FacilityPcc,
                    PowerMeasurementRole::AggregateMeter,
                ),
                point(
                    "A-NODE",
                    "SITE-A",
                    Some("A-PCC"),
                    PowerMeasurementBoundary::NodeInput,
                    PowerMeasurementRole::ComponentMeter,
                ),
                point(
                    "B-PCC",
                    "SITE-B",
                    None,
                    PowerMeasurementBoundary::FacilityPcc,
                    PowerMeasurementRole::AggregateMeter,
                ),
            ],
            vec![TraceWorkloadSession::new("JOB-A", "SITE-A", "SLA-A").unwrap()],
            vec![
                sample(0, "SITE-A", "A-PCC", 1_500_000.0, None),
                sample(20_000_000, "SITE-A", "A-PCC", 1_510_000.0, None),
                sample(0, "SITE-A", "A-NODE", 8_200.0, Some("JOB-A")),
                sample(20_000_000, "SITE-A", "A-NODE", 8_450.0, Some("JOB-A")),
                sample(0, "SITE-B", "B-PCC", 2_200_000.0, None),
                sample(20_000_000, "SITE-B", "B-PCC", 2_190_000.0, None),
            ],
        )
        .unwrap()
    }

    #[test]
    fn validates_aligned_multi_site_trace_and_poc_aggregation() {
        let trace = valid_trace();
        let report = trace.validate().unwrap();

        assert_eq!(report.site_count(), 2);
        assert_eq!(report.measurement_point_count(), 3);
        assert_eq!(report.workload_count(), 1);
        assert_eq!(report.sample_count(), 6);

        let selection = PowerAggregationSelection::new(
            PowerMeasurementBoundary::FacilityPcc,
            vec!["A-PCC".to_string(), "B-PCC".to_string()],
            5_000_000,
            2,
        )
        .unwrap();
        assert!(trace.validate_aggregation(&selection).is_ok());
    }

    #[test]
    fn rejects_facility_pcc_plus_nested_ups_or_node_meter() {
        let trace = valid_trace();
        let selection = PowerAggregationSelection::new(
            PowerMeasurementBoundary::FacilityPcc,
            vec!["A-PCC".to_string(), "A-NODE".to_string()],
            5_000_000,
            2,
        );
        // Boundary mismatch is also fail-closed because the node meter is not a PCC meter.
        assert!(matches!(
            trace.validate_aggregation(&selection.unwrap()),
            Err(PowerTraceError::AggregationBoundaryMismatch { .. })
        ));

        let nested = DataCenterPowerTrace::new(
            "TRACE-NESTED",
            timebase(TraceClockType::PtpSynchronized, 1_000_000),
            vec![site("SITE-A")],
            vec![
                point(
                    "PCC",
                    "SITE-A",
                    None,
                    PowerMeasurementBoundary::FacilityPcc,
                    PowerMeasurementRole::AggregateMeter,
                ),
                point(
                    "UPS",
                    "SITE-A",
                    Some("PCC"),
                    PowerMeasurementBoundary::FacilityPcc,
                    PowerMeasurementRole::SubsystemMeter,
                ),
            ],
            vec![],
            vec![
                sample(0, "SITE-A", "PCC", 1_000_000.0, None),
                sample(0, "SITE-A", "UPS", 800_000.0, None),
            ],
        )
        .unwrap();
        let selection = PowerAggregationSelection::new(
            PowerMeasurementBoundary::FacilityPcc,
            vec!["PCC".to_string(), "UPS".to_string()],
            5_000_000,
            2,
        )
        .unwrap();

        assert!(matches!(
            nested.validate_aggregation(&selection),
            Err(PowerTraceError::NestedMeterDoubleCount { .. })
        ));
    }

    #[test]
    fn rejects_cross_site_correlation_when_alignment_error_exceeds_limit() {
        let trace = DataCenterPowerTrace::new(
            "TRACE-UNSYNC",
            timebase(TraceClockType::NtpSynchronized, 25_000_000),
            vec![site("SITE-A"), site("SITE-B")],
            vec![
                point(
                    "A-PCC",
                    "SITE-A",
                    None,
                    PowerMeasurementBoundary::FacilityPcc,
                    PowerMeasurementRole::AggregateMeter,
                ),
                point(
                    "B-PCC",
                    "SITE-B",
                    None,
                    PowerMeasurementBoundary::FacilityPcc,
                    PowerMeasurementRole::AggregateMeter,
                ),
            ],
            vec![],
            vec![
                sample(0, "SITE-A", "A-PCC", 900_000.0, None),
                sample(0, "SITE-B", "B-PCC", 1_100_000.0, None),
            ],
        )
        .unwrap();
        let selection = PowerAggregationSelection::new(
            PowerMeasurementBoundary::FacilityPcc,
            vec!["A-PCC".to_string(), "B-PCC".to_string()],
            5_000_000,
            2,
        )
        .unwrap();

        assert!(matches!(
            trace.validate_aggregation(&selection),
            Err(PowerTraceError::ClockAlignmentExceeded { .. })
        ));
    }

    #[test]
    fn rejects_cross_site_correlation_without_common_sample_timestamps() {
        let trace = DataCenterPowerTrace::new(
            "TRACE-NO-COMMON-SAMPLES",
            timebase(TraceClockType::PtpSynchronized, 1_000_000),
            vec![site("SITE-A"), site("SITE-B")],
            vec![
                point(
                    "A-PCC",
                    "SITE-A",
                    None,
                    PowerMeasurementBoundary::FacilityPcc,
                    PowerMeasurementRole::AggregateMeter,
                ),
                point(
                    "B-PCC",
                    "SITE-B",
                    None,
                    PowerMeasurementBoundary::FacilityPcc,
                    PowerMeasurementRole::AggregateMeter,
                ),
            ],
            vec![],
            vec![
                sample(0, "SITE-A", "A-PCC", 900_000.0, None),
                sample(20_000_000, "SITE-A", "A-PCC", 910_000.0, None),
                sample(10_000_000, "SITE-B", "B-PCC", 1_100_000.0, None),
                sample(30_000_000, "SITE-B", "B-PCC", 1_090_000.0, None),
            ],
        )
        .unwrap();
        let selection = PowerAggregationSelection::new(
            PowerMeasurementBoundary::FacilityPcc,
            vec!["A-PCC".to_string(), "B-PCC".to_string()],
            5_000_000,
            2,
        )
        .unwrap();

        assert!(matches!(
            trace.validate_aggregation(&selection),
            Err(PowerTraceError::InsufficientCommonSamples {
                observed: 0,
                required: 2
            })
        ));
    }

    #[test]
    fn allows_single_site_energy_totals_without_cross_site_clock_requirement() {
        let trace = DataCenterPowerTrace::new(
            "TRACE-SINGLE-SITE",
            timebase(TraceClockType::WallClock, 25_000_000),
            vec![site("SITE-A")],
            vec![point(
                "A-PCC",
                "SITE-A",
                None,
                PowerMeasurementBoundary::FacilityPcc,
                PowerMeasurementRole::AggregateMeter,
            )],
            vec![],
            vec![sample(0, "SITE-A", "A-PCC", 900_000.0, None)],
        )
        .unwrap();
        let selection = PowerAggregationSelection::new(
            PowerMeasurementBoundary::FacilityPcc,
            vec!["A-PCC".to_string()],
            5_000_000,
            2,
        )
        .unwrap();

        assert!(trace.validate_aggregation(&selection).is_ok());
    }

    #[test]
    fn rejects_duplicate_site_or_measurement_point_ids() {
        let duplicate_site = DataCenterPowerTrace::new(
            "TRACE-DUP-SITE",
            timebase(TraceClockType::PtpSynchronized, 1),
            vec![site("SITE-A"), site("SITE-A")],
            vec![point(
                "A-PCC",
                "SITE-A",
                None,
                PowerMeasurementBoundary::FacilityPcc,
                PowerMeasurementRole::AggregateMeter,
            )],
            vec![],
            vec![sample(0, "SITE-A", "A-PCC", 100.0, None)],
        );

        assert!(matches!(
            duplicate_site,
            Err(PowerTraceError::DuplicateIdentity {
                kind: "site ID",
                ..
            })
        ));

        let duplicate_point = DataCenterPowerTrace::new(
            "TRACE-DUP-POINT",
            timebase(TraceClockType::PtpSynchronized, 1),
            vec![site("SITE-A")],
            vec![
                point(
                    "A-PCC",
                    "SITE-A",
                    None,
                    PowerMeasurementBoundary::FacilityPcc,
                    PowerMeasurementRole::AggregateMeter,
                ),
                point(
                    "A-PCC",
                    "SITE-A",
                    None,
                    PowerMeasurementBoundary::FacilityPcc,
                    PowerMeasurementRole::AggregateMeter,
                ),
            ],
            vec![],
            vec![sample(0, "SITE-A", "A-PCC", 100.0, None)],
        );

        assert!(matches!(
            duplicate_point,
            Err(PowerTraceError::DuplicateIdentity {
                kind: "measurement point ID",
                ..
            })
        ));
    }

    #[test]
    fn rejects_unknown_parent_and_hierarchy_cycles() {
        let unknown_parent = DataCenterPowerTrace::new(
            "TRACE-BAD-PARENT",
            timebase(TraceClockType::PtpSynchronized, 1),
            vec![site("SITE-A")],
            vec![point(
                "NODE",
                "SITE-A",
                Some("MISSING"),
                PowerMeasurementBoundary::NodeInput,
                PowerMeasurementRole::ComponentMeter,
            )],
            vec![],
            vec![sample(0, "SITE-A", "NODE", 100.0, None)],
        );

        assert!(matches!(
            unknown_parent,
            Err(PowerTraceError::UnknownParent { .. })
        ));

        let cycle = DataCenterPowerTrace::new(
            "TRACE-CYCLE",
            timebase(TraceClockType::PtpSynchronized, 1),
            vec![site("SITE-A")],
            vec![
                point(
                    "A",
                    "SITE-A",
                    Some("B"),
                    PowerMeasurementBoundary::NodeInput,
                    PowerMeasurementRole::ComponentMeter,
                ),
                point(
                    "B",
                    "SITE-A",
                    Some("A"),
                    PowerMeasurementBoundary::RackPdu,
                    PowerMeasurementRole::SubsystemMeter,
                ),
            ],
            vec![],
            vec![sample(0, "SITE-A", "A", 100.0, None)],
        );

        assert!(matches!(
            cycle,
            Err(PowerTraceError::MeasurementHierarchyCycle { .. })
        ));
    }

    #[test]
    fn rejects_non_monotonic_samples_for_a_meter() {
        let trace = DataCenterPowerTrace::new(
            "TRACE-OUT-OF-ORDER",
            timebase(TraceClockType::PtpSynchronized, 1),
            vec![site("SITE-A")],
            vec![point(
                "A-PCC",
                "SITE-A",
                None,
                PowerMeasurementBoundary::FacilityPcc,
                PowerMeasurementRole::AggregateMeter,
            )],
            vec![],
            vec![
                sample(20, "SITE-A", "A-PCC", 100.0, None),
                sample(10, "SITE-A", "A-PCC", 110.0, None),
            ],
        );

        assert!(matches!(
            trace,
            Err(PowerTraceError::NonMonotonicTimestamp { .. })
        ));
    }

    #[test]
    fn rejects_workload_to_sample_site_mismatch() {
        let trace = DataCenterPowerTrace::new(
            "TRACE-WORKLOAD-SITE",
            timebase(TraceClockType::PtpSynchronized, 1),
            vec![site("SITE-A"), site("SITE-B")],
            vec![point(
                "B-NODE",
                "SITE-B",
                None,
                PowerMeasurementBoundary::NodeInput,
                PowerMeasurementRole::ComponentMeter,
            )],
            vec![TraceWorkloadSession::new("JOB-A", "SITE-A", "SLA-A").unwrap()],
            vec![sample(0, "SITE-B", "B-NODE", 100.0, Some("JOB-A"))],
        );

        assert!(matches!(
            trace,
            Err(PowerTraceError::WorkloadSiteMismatch { .. })
        ));
    }

    #[test]
    fn rejects_negative_power_under_positive_import_sign_convention() {
        let trace = DataCenterPowerTrace::new(
            "TRACE-NEGATIVE-IMPORT",
            timebase(TraceClockType::PtpSynchronized, 1),
            vec![site("SITE-A")],
            vec![point(
                "A-PCC",
                "SITE-A",
                None,
                PowerMeasurementBoundary::FacilityPcc,
                PowerMeasurementRole::AggregateMeter,
            )],
            vec![],
            vec![sample(0, "SITE-A", "A-PCC", -100.0, None)],
        );

        assert!(matches!(
            trace,
            Err(PowerTraceError::UnexpectedNegativePower { .. })
        ));
    }

    #[test]
    fn requires_measurement_evidence_and_calibration_identity() {
        assert!(PowerMeasurementPoint::new(
            "METER",
            "SITE-A",
            None,
            PowerMeasurementBoundary::FacilityPcc,
            PowerMeasurementRole::AggregateMeter,
            PowerSignConvention::PositiveImport,
            " ",
            "EVIDENCE",
        )
        .is_err());
        assert!(PowerTraceSample::new(0, "SITE-A", "METER", f64::NAN, None, "EVIDENCE").is_err());
    }
}
