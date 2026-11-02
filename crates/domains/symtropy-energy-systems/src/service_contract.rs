// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Typed delivered-service contracts for cross-topology comparisons.
//!
//! This layer describes what reaches the customer interface (net AC, regulated
//! DC or propulsion work) without pretending the internal converters share one
//! physical model. It only computes service matching. An input-energy ratio is
//! emitted only when both systems declare the same input-energy basis.

use core::fmt;

use serde::Serialize;

use crate::TimeWindow;

/// The customer-facing interface at which useful service is measured.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ServiceInterface {
    /// Net electrical energy at the declared AC terminal.
    NetAc,
    /// Regulated DC energy at the declared DC bus.
    RegulatedDc,
    /// Mechanical work at a declared propulsion measurement point.
    PropulsionWork(PropulsionWorkStage),
}

/// Mechanical measurement point for a propulsion mission.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum PropulsionWorkStage {
    Shaft,
    Wheel,
    Propulsor,
}

/// Complete identity of the service measurement boundary.
///
/// Equality is intentionally exact: two systems are not comparable merely
/// because both produce joules. Their interface, measurement point and
/// service specification must match.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ServiceBoundary {
    interface: ServiceInterface,
    boundary_id: String,
    specification_id: String,
}

impl ServiceBoundary {
    pub fn new(
        interface: ServiceInterface,
        boundary_id: impl Into<String>,
        specification_id: impl Into<String>,
    ) -> Result<Self, ServiceContractError> {
        let boundary_id = boundary_id.into();
        let specification_id = specification_id.into();

        if boundary_id.trim().is_empty() {
            return Err(ServiceContractError::MissingBoundaryId);
        }
        if specification_id.trim().is_empty() {
            return Err(ServiceContractError::MissingServiceSpecification);
        }

        Ok(Self {
            interface,
            boundary_id,
            specification_id,
        })
    }

    pub const fn interface(&self) -> ServiceInterface {
        self.interface
    }

    pub fn boundary_id(&self) -> &str {
        &self.boundary_id
    }

    pub fn specification_id(&self) -> &str {
        &self.specification_id
    }
}

/// One requested quantity of useful service over a precise time interval.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct TypedServiceDemandSegment {
    time_window: TimeWindow,
    required_energy_j: f64,
}

impl TypedServiceDemandSegment {
    pub fn new(
        time_window: TimeWindow,
        required_energy_j: f64,
    ) -> Result<Self, ServiceContractError> {
        if !required_energy_j.is_finite() || required_energy_j < 0.0 {
            return Err(ServiceContractError::InvalidEnergy {
                quantity: "required service energy",
                value: required_energy_j,
            });
        }

        Ok(Self {
            time_window,
            required_energy_j,
        })
    }

    pub const fn time_window(self) -> TimeWindow {
        self.time_window
    }

    pub const fn required_energy_j(self) -> f64 {
        self.required_energy_j
    }
}

/// Frozen customer service contract shared by all converter topologies.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TypedServiceDemandContract {
    profile_id: String,
    version: String,
    boundary: ServiceBoundary,
    segments: Vec<TypedServiceDemandSegment>,
    total_required_energy_j: f64,
}

impl TypedServiceDemandContract {
    pub fn new(
        profile_id: impl Into<String>,
        version: impl Into<String>,
        boundary: ServiceBoundary,
        segments: Vec<TypedServiceDemandSegment>,
    ) -> Result<Self, ServiceContractError> {
        let profile_id = profile_id.into();
        let version = version.into();

        if profile_id.trim().is_empty() {
            return Err(ServiceContractError::MissingProfileId);
        }
        if version.trim().is_empty() {
            return Err(ServiceContractError::MissingProfileVersion);
        }
        if segments.is_empty() {
            return Err(ServiceContractError::EmptyProfile);
        }

        let mut total_required_energy_j = 0.0;
        for (index, segment) in segments.iter().enumerate() {
            if let Some(previous) = index.checked_sub(1).and_then(|i| segments.get(i)) {
                if previous.time_window().end_unix_seconds()
                    != segment.time_window().start_unix_seconds()
                {
                    return Err(ServiceContractError::NonContiguousProfile {
                        previous_end: previous.time_window().end_unix_seconds(),
                        next_start: segment.time_window().start_unix_seconds(),
                    });
                }
            }

            total_required_energy_j += segment.required_energy_j();
            if !total_required_energy_j.is_finite() {
                return Err(ServiceContractError::InvalidEnergy {
                    quantity: "total requested service energy",
                    value: total_required_energy_j,
                });
            }
        }

        if total_required_energy_j <= 0.0 {
            return Err(ServiceContractError::ZeroTotalDemand);
        }

        Ok(Self {
            profile_id,
            version,
            boundary,
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

    pub const fn boundary(&self) -> &ServiceBoundary {
        &self.boundary
    }

    pub fn segments(&self) -> &[TypedServiceDemandSegment] {
        &self.segments
    }

    pub const fn total_required_energy_j(&self) -> f64 {
        self.total_required_energy_j
    }
}

/// The boundary at which a system's consumed input energy is metered.
///
/// Fuel chemical energy, purchased electricity and initially stored battery
/// energy are distinct bases. They must not be silently compared as equivalent
/// primary energy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum InputEnergyBasis {
    FuelChemicalLhv,
    FuelChemicalHhv,
    PurchasedElectricityAtMeter,
    StoredElectricalEnergyAtInitialSoc,
    Other(String),
}

/// Evidence-qualified input-energy total for one service trace.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct InputEnergyRecord {
    basis: InputEnergyBasis,
    energy_j: f64,
    evidence_id: Option<String>,
}

impl InputEnergyRecord {
    pub fn new(
        basis: InputEnergyBasis,
        energy_j: f64,
        evidence_id: Option<String>,
    ) -> Result<Self, ServiceContractError> {
        if !energy_j.is_finite() || energy_j <= 0.0 {
            return Err(ServiceContractError::InvalidEnergy {
                quantity: "input energy",
                value: energy_j,
            });
        }
        if evidence_id.as_ref().is_some_and(|id| id.trim().is_empty()) {
            return Err(ServiceContractError::MissingEvidenceId);
        }
        if matches!(&basis, InputEnergyBasis::Other(label) if label.trim().is_empty()) {
            return Err(ServiceContractError::MissingInputEnergyBasisLabel);
        }

        Ok(Self {
            basis,
            energy_j,
            evidence_id,
        })
    }

    pub const fn basis(&self) -> &InputEnergyBasis {
        &self.basis
    }

    pub const fn energy_j(&self) -> f64 {
        self.energy_j
    }

    pub fn evidence_id(&self) -> Option<&str> {
        self.evidence_id.as_deref()
    }
}

/// Delivered useful service for one segment at the declared customer boundary.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct DeliveredServiceSegment {
    time_window: TimeWindow,
    delivered_energy_j: f64,
}

impl DeliveredServiceSegment {
    pub fn new(
        time_window: TimeWindow,
        delivered_energy_j: f64,
    ) -> Result<Self, ServiceContractError> {
        if !delivered_energy_j.is_finite() || delivered_energy_j < 0.0 {
            return Err(ServiceContractError::InvalidEnergy {
                quantity: "delivered service energy",
                value: delivered_energy_j,
            });
        }

        Ok(Self {
            time_window,
            delivered_energy_j,
        })
    }

    pub const fn time_window(self) -> TimeWindow {
        self.time_window
    }

    pub const fn delivered_energy_j(self) -> f64 {
        self.delivered_energy_j
    }
}

/// Evidence-bearing output trace from one converter topology.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DeliveredServiceTrace {
    system_name: String,
    topology_id: String,
    configuration_digest: String,
    profile_id: String,
    profile_version: String,
    boundary: ServiceBoundary,
    segments: Vec<DeliveredServiceSegment>,
    input_energy: Option<InputEnergyRecord>,
}

impl DeliveredServiceTrace {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        system_name: impl Into<String>,
        topology_id: impl Into<String>,
        configuration_digest: impl Into<String>,
        profile_id: impl Into<String>,
        profile_version: impl Into<String>,
        boundary: ServiceBoundary,
        segments: Vec<DeliveredServiceSegment>,
        input_energy: Option<InputEnergyRecord>,
    ) -> Result<Self, ServiceContractError> {
        let system_name = system_name.into();
        let topology_id = topology_id.into();
        let configuration_digest = configuration_digest.into();
        let profile_id = profile_id.into();
        let profile_version = profile_version.into();

        for (field, value) in [
            ("system name", system_name.as_str()),
            ("topology ID", topology_id.as_str()),
            ("configuration digest", configuration_digest.as_str()),
            ("profile ID", profile_id.as_str()),
            ("profile version", profile_version.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(ServiceContractError::MissingIdentity { field });
            }
        }
        if segments.is_empty() {
            return Err(ServiceContractError::EmptyTrace);
        }

        for (index, segment) in segments.iter().enumerate() {
            if let Some(previous) = index.checked_sub(1).and_then(|i| segments.get(i)) {
                if previous.time_window().end_unix_seconds()
                    != segment.time_window().start_unix_seconds()
                {
                    return Err(ServiceContractError::NonContiguousTrace {
                        previous_end: previous.time_window().end_unix_seconds(),
                        next_start: segment.time_window().start_unix_seconds(),
                    });
                }
            }
        }

        Ok(Self {
            system_name,
            topology_id,
            configuration_digest,
            profile_id,
            profile_version,
            boundary,
            segments,
            input_energy,
        })
    }

    pub fn system_name(&self) -> &str {
        &self.system_name
    }

    pub fn topology_id(&self) -> &str {
        &self.topology_id
    }

    pub fn configuration_digest(&self) -> &str {
        &self.configuration_digest
    }

    pub fn profile_id(&self) -> &str {
        &self.profile_id
    }

    pub fn profile_version(&self) -> &str {
        &self.profile_version
    }

    pub const fn boundary(&self) -> &ServiceBoundary {
        &self.boundary
    }

    pub fn segments(&self) -> &[DeliveredServiceSegment] {
        &self.segments
    }

    pub fn input_energy(&self) -> Option<&InputEnergyRecord> {
        self.input_energy.as_ref()
    }
}

/// Absolute/relative tolerance for service shortfall at each segment.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct ServiceDeliveryTolerance {
    absolute_energy_j: f64,
    relative_fraction: f64,
}

impl ServiceDeliveryTolerance {
    pub fn new(
        absolute_energy_j: f64,
        relative_fraction: f64,
    ) -> Result<Self, ServiceContractError> {
        if !absolute_energy_j.is_finite()
            || absolute_energy_j < 0.0
            || !relative_fraction.is_finite()
            || !(0.0..=1.0).contains(&relative_fraction)
        {
            return Err(ServiceContractError::InvalidTolerance {
                absolute_energy_j,
                relative_fraction,
            });
        }

        Ok(Self {
            absolute_energy_j,
            relative_fraction,
        })
    }

    fn allowed_shortfall_j(self, required_energy_j: f64) -> f64 {
        self.absolute_energy_j
            .max(self.relative_fraction * required_energy_j)
    }
}

/// How the input-energy ratio should be interpreted.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum InputEnergyComparison {
    Comparable {
        basis: InputEnergyBasis,
        reference_input_energy_j: f64,
        candidate_input_energy_j: f64,
        candidate_service_per_input_ratio: f64,
        reference_evidence_id: Option<String>,
        candidate_evidence_id: Option<String>,
    },
    MissingReferenceInput,
    MissingCandidateInput,
    DifferentInputBases {
        reference_basis: InputEnergyBasis,
        candidate_basis: InputEnergyBasis,
    },
}

/// One segment's delivered service, credited service, surplus and deficit.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct TypedServiceComparisonSegment {
    time_window: TimeWindow,
    required_energy_j: f64,
    reference_delivered_energy_j: f64,
    candidate_delivered_energy_j: f64,
    common_service_energy_j: f64,
    reference_surplus_energy_j: f64,
    candidate_surplus_energy_j: f64,
    reference_unserved_energy_j: f64,
    candidate_unserved_energy_j: f64,
}

impl TypedServiceComparisonSegment {
    pub const fn time_window(self) -> TimeWindow {
        self.time_window
    }

    pub const fn required_energy_j(self) -> f64 {
        self.required_energy_j
    }

    pub const fn reference_delivered_energy_j(self) -> f64 {
        self.reference_delivered_energy_j
    }

    pub const fn candidate_delivered_energy_j(self) -> f64 {
        self.candidate_delivered_energy_j
    }

    pub const fn common_service_energy_j(self) -> f64 {
        self.common_service_energy_j
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

/// Matched useful service across two topologies at the same typed interface.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TypedServiceComparison {
    profile_id: String,
    profile_version: String,
    boundary: ServiceBoundary,
    reference_name: String,
    reference_topology_id: String,
    candidate_name: String,
    candidate_topology_id: String,
    required_service_energy_j: f64,
    common_service_energy_j: f64,
    reference_surplus_energy_j: f64,
    candidate_surplus_energy_j: f64,
    reference_unserved_energy_j: f64,
    candidate_unserved_energy_j: f64,
    input_energy_comparison: InputEnergyComparison,
    segments: Vec<TypedServiceComparisonSegment>,
}

impl TypedServiceComparison {
    pub fn profile_id(&self) -> &str {
        &self.profile_id
    }

    pub fn profile_version(&self) -> &str {
        &self.profile_version
    }

    pub const fn boundary(&self) -> &ServiceBoundary {
        &self.boundary
    }

    pub fn reference_name(&self) -> &str {
        &self.reference_name
    }

    pub fn candidate_name(&self) -> &str {
        &self.candidate_name
    }

    pub fn reference_topology_id(&self) -> &str {
        &self.reference_topology_id
    }

    pub fn candidate_topology_id(&self) -> &str {
        &self.candidate_topology_id
    }

    pub const fn required_service_energy_j(&self) -> f64 {
        self.required_service_energy_j
    }

    pub const fn common_service_energy_j(&self) -> f64 {
        self.common_service_energy_j
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

    pub const fn input_energy_comparison(&self) -> &InputEnergyComparison {
        &self.input_energy_comparison
    }

    pub fn segments(&self) -> &[TypedServiceComparisonSegment] {
        &self.segments
    }
}

/// Compare two topologies at one exact service boundary and frozen profile.
///
/// If the input-energy bases differ (e.g. fuel chemical LHV vs stored
/// electricity), service is still compared but no efficiency/input-energy
/// ratio is emitted. Upstream primary-energy and lifecycle comparison belong
/// to a separate well-to-service boundary.
pub fn compare_delivered_service(
    contract: &TypedServiceDemandContract,
    reference: &DeliveredServiceTrace,
    candidate: &DeliveredServiceTrace,
    tolerance: ServiceDeliveryTolerance,
) -> Result<TypedServiceComparison, ServiceContractError> {
    for trace in [reference, candidate] {
        if trace.profile_id() != contract.profile_id()
            || trace.profile_version() != contract.version()
        {
            return Err(ServiceContractError::ProfileIdentityMismatch {
                system: trace.system_name().to_string(),
            });
        }
        if trace.boundary() != contract.boundary() {
            return Err(ServiceContractError::BoundaryMismatch {
                system: trace.system_name().to_string(),
            });
        }
        if trace.segments().len() != contract.segments().len() {
            return Err(ServiceContractError::SegmentCountMismatch {
                system: trace.system_name().to_string(),
                expected: contract.segments().len(),
                actual: trace.segments().len(),
            });
        }
    }

    let mut segments = Vec::with_capacity(contract.segments().len());
    let mut common_service_energy_j = 0.0;
    let mut reference_surplus_energy_j = 0.0;
    let mut candidate_surplus_energy_j = 0.0;
    let mut reference_unserved_energy_j = 0.0;
    let mut candidate_unserved_energy_j = 0.0;

    for index in 0..contract.segments().len() {
        let demand = contract.segments()[index];
        let reference_segment = reference.segments()[index];
        let candidate_segment = candidate.segments()[index];

        if demand.time_window() != reference_segment.time_window()
            || demand.time_window() != candidate_segment.time_window()
        {
            return Err(ServiceContractError::TimeWindowMismatch { index });
        }

        let required = demand.required_energy_j();
        let reference_delivered = reference_segment.delivered_energy_j();
        let candidate_delivered = candidate_segment.delivered_energy_j();

        for (system, delivered) in [
            (reference.system_name(), reference_delivered),
            (candidate.system_name(), candidate_delivered),
        ] {
            let deficit = (required - delivered).max(0.0);
            let allowed = tolerance.allowed_shortfall_j(required);

            if required > 0.0 && (!allowed.is_finite() || allowed >= required) {
                return Err(ServiceContractError::ToleranceExceedsDemand {
                    index,
                    required_energy_j: required,
                    allowed_shortfall_j: allowed,
                });
            }
            if deficit > allowed {
                return Err(ServiceContractError::ServiceDeficit {
                    system: system.to_string(),
                    index,
                    required_energy_j: required,
                    delivered_energy_j: delivered,
                    deficit_energy_j: deficit,
                    allowed_shortfall_j: allowed,
                });
            }
        }

        let common = required.min(reference_delivered).min(candidate_delivered);
        let reference_surplus = (reference_delivered - required).max(0.0);
        let candidate_surplus = (candidate_delivered - required).max(0.0);
        let reference_unserved = (required - reference_delivered).max(0.0);
        let candidate_unserved = (required - candidate_delivered).max(0.0);

        common_service_energy_j += common;
        reference_surplus_energy_j += reference_surplus;
        candidate_surplus_energy_j += candidate_surplus;
        reference_unserved_energy_j += reference_unserved;
        candidate_unserved_energy_j += candidate_unserved;

        segments.push(TypedServiceComparisonSegment {
            time_window: demand.time_window(),
            required_energy_j: required,
            reference_delivered_energy_j: reference_delivered,
            candidate_delivered_energy_j: candidate_delivered,
            common_service_energy_j: common,
            reference_surplus_energy_j: reference_surplus,
            candidate_surplus_energy_j: candidate_surplus,
            reference_unserved_energy_j: reference_unserved,
            candidate_unserved_energy_j: candidate_unserved,
        });
    }

    if !common_service_energy_j.is_finite() || common_service_energy_j <= 0.0 {
        return Err(ServiceContractError::ZeroCommonService);
    }

    let input_energy_comparison = match (reference.input_energy(), candidate.input_energy()) {
        (None, _) => InputEnergyComparison::MissingReferenceInput,
        (_, None) => InputEnergyComparison::MissingCandidateInput,
        (Some(reference_input), Some(candidate_input)) => {
            if reference_input.basis() != candidate_input.basis() {
                InputEnergyComparison::DifferentInputBases {
                    reference_basis: reference_input.basis().clone(),
                    candidate_basis: candidate_input.basis().clone(),
                }
            } else {
                let ratio = reference_input.energy_j() / candidate_input.energy_j();
                if !ratio.is_finite() || ratio <= 0.0 {
                    return Err(ServiceContractError::InvalidInputEnergyRatio { value: ratio });
                }
                InputEnergyComparison::Comparable {
                    basis: reference_input.basis().clone(),
                    reference_input_energy_j: reference_input.energy_j(),
                    candidate_input_energy_j: candidate_input.energy_j(),
                    candidate_service_per_input_ratio: ratio,
                    reference_evidence_id: reference_input.evidence_id().map(str::to_string),
                    candidate_evidence_id: candidate_input.evidence_id().map(str::to_string),
                }
            }
        }
    };

    Ok(TypedServiceComparison {
        profile_id: contract.profile_id().to_string(),
        profile_version: contract.version().to_string(),
        boundary: contract.boundary().clone(),
        reference_name: reference.system_name().to_string(),
        reference_topology_id: reference.topology_id().to_string(),
        candidate_name: candidate.system_name().to_string(),
        candidate_topology_id: candidate.topology_id().to_string(),
        required_service_energy_j: contract.total_required_energy_j(),
        common_service_energy_j,
        reference_surplus_energy_j,
        candidate_surplus_energy_j,
        reference_unserved_energy_j,
        candidate_unserved_energy_j,
        input_energy_comparison,
        segments,
    })
}

#[derive(Debug, Clone, PartialEq)]
pub enum ServiceContractError {
    MissingBoundaryId,
    MissingServiceSpecification,
    MissingProfileId,
    MissingProfileVersion,
    MissingEvidenceId,
    MissingInputEnergyBasisLabel,
    MissingIdentity {
        field: &'static str,
    },
    EmptyProfile,
    EmptyTrace,
    ZeroTotalDemand,
    InvalidEnergy {
        quantity: &'static str,
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
    NonContiguousTrace {
        previous_end: i64,
        next_start: i64,
    },
    ProfileIdentityMismatch {
        system: String,
    },
    BoundaryMismatch {
        system: String,
    },
    SegmentCountMismatch {
        system: String,
        expected: usize,
        actual: usize,
    },
    TimeWindowMismatch {
        index: usize,
    },
    ToleranceExceedsDemand {
        index: usize,
        required_energy_j: f64,
        allowed_shortfall_j: f64,
    },
    ServiceDeficit {
        system: String,
        index: usize,
        required_energy_j: f64,
        delivered_energy_j: f64,
        deficit_energy_j: f64,
        allowed_shortfall_j: f64,
    },
    InvalidInputEnergyRatio {
        value: f64,
    },
    ZeroCommonService,
}

impl fmt::Display for ServiceContractError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingBoundaryId => write!(f, "service boundary ID must not be empty"),
            Self::MissingServiceSpecification => {
                write!(f, "service specification ID must not be empty")
            }
            Self::MissingProfileId => write!(f, "service profile ID must not be empty"),
            Self::MissingProfileVersion => write!(f, "service profile version must not be empty"),
            Self::MissingEvidenceId => write!(f, "input evidence ID must not be empty"),
            Self::MissingInputEnergyBasisLabel => {
                write!(f, "other input-energy basis label must not be empty")
            }
            Self::MissingIdentity { field } => write!(f, "{field} must not be empty"),
            Self::EmptyProfile => write!(f, "service profile must not be empty"),
            Self::EmptyTrace => write!(f, "delivered service trace must not be empty"),
            Self::ZeroTotalDemand => write!(f, "total service demand must be positive"),
            Self::InvalidEnergy { quantity, value } => {
                write!(f, "{quantity} must be finite and non-negative; got {value}")
            }
            Self::InvalidTolerance {
                absolute_energy_j,
                relative_fraction,
            } => write!(
                f,
                "service tolerance invalid: absolute={absolute_energy_j} J, relative={relative_fraction}"
            ),
            Self::NonContiguousProfile {
                previous_end,
                next_start,
            } => write!(
                f,
                "service profile not contiguous: previous end {previous_end}, next start {next_start}"
            ),
            Self::NonContiguousTrace {
                previous_end,
                next_start,
            } => write!(
                f,
                "service trace not contiguous: previous end {previous_end}, next start {next_start}"
            ),
            Self::ProfileIdentityMismatch { system } => {
                write!(f, "{system} uses a different service profile ID/version")
            }
            Self::BoundaryMismatch { system } => {
                write!(f, "{system} uses a different delivered-service boundary")
            }
            Self::SegmentCountMismatch {
                system,
                expected,
                actual,
            } => write!(
                f,
                "{system} has {actual} service segments; expected {expected}"
            ),
            Self::TimeWindowMismatch { index } => {
                write!(f, "service trace time window mismatch at segment {index}")
            }
            Self::ToleranceExceedsDemand {
                index,
                required_energy_j,
                allowed_shortfall_j,
            } => write!(
                f,
                "service tolerance consumes requested energy at segment {index}: required={required_energy_j} J, allowed={allowed_shortfall_j} J"
            ),
            Self::ServiceDeficit {
                system,
                index,
                required_energy_j,
                delivered_energy_j,
                deficit_energy_j,
                allowed_shortfall_j,
            } => write!(
                f,
                "{system} under-delivers segment {index}: required={required_energy_j} J, delivered={delivered_energy_j} J, deficit={deficit_energy_j} J, allowed={allowed_shortfall_j} J"
            ),
            Self::InvalidInputEnergyRatio { value } => {
                write!(f, "candidate/reference input-energy ratio invalid: {value}")
            }
            Self::ZeroCommonService => write!(f, "common delivered service must be positive"),
        }
    }
}

impl std::error::Error for ServiceContractError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn boundary(interface: ServiceInterface, id: &str) -> ServiceBoundary {
        ServiceBoundary::new(interface, id, "service-spec-v1").unwrap()
    }

    fn window(start: i64, end: i64) -> TimeWindow {
        TimeWindow::new(start, end).unwrap()
    }

    fn contract(boundary: ServiceBoundary) -> TypedServiceDemandContract {
        TypedServiceDemandContract::new(
            "DATA-CENTER-DUTY",
            "v1",
            boundary,
            vec![
                TypedServiceDemandSegment::new(window(0, 60), 6_000_000.0).unwrap(),
                TypedServiceDemandSegment::new(window(60, 120), 6_000_000.0).unwrap(),
            ],
        )
        .unwrap()
    }

    fn trace(
        name: &str,
        topology: &str,
        profile: &str,
        boundary: ServiceBoundary,
        energies: [f64; 2],
        input: Option<InputEnergyRecord>,
    ) -> DeliveredServiceTrace {
        DeliveredServiceTrace::new(
            name,
            topology,
            format!("sha256:{topology}-config"),
            profile,
            "v1",
            boundary,
            vec![
                DeliveredServiceSegment::new(window(0, 60), energies[0]).unwrap(),
                DeliveredServiceSegment::new(window(60, 120), energies[1]).unwrap(),
            ],
            input,
        )
        .unwrap()
    }

    #[test]
    fn compares_different_topologies_at_same_ac_customer_boundary() {
        let b = boundary(ServiceInterface::NetAc, "facility-ac-main");
        let demand = contract(b.clone());
        let reference = trace(
            "engine-gen",
            "reciprocating-engine-generator",
            "DATA-CENTER-DUTY",
            b.clone(),
            [6_000_000.0, 6_000_000.0],
            Some(
                InputEnergyRecord::new(
                    InputEnergyBasis::FuelChemicalLhv,
                    30_000_000.0,
                    Some("FUEL-RECEIPT-01".to_string()),
                )
                .unwrap(),
            ),
        );
        let candidate = trace(
            "linear-gen",
            "free-piston-linear-generator",
            "DATA-CENTER-DUTY",
            b,
            [6_000_000.0, 6_000_000.0],
            Some(
                InputEnergyRecord::new(
                    InputEnergyBasis::FuelChemicalLhv,
                    27_000_000.0,
                    Some("FUEL-RECEIPT-02".to_string()),
                )
                .unwrap(),
            ),
        );

        let comparison = compare_delivered_service(
            &demand,
            &reference,
            &candidate,
            ServiceDeliveryTolerance::new(0.0, 0.0).unwrap(),
        )
        .unwrap();

        assert_eq!(comparison.common_service_energy_j(), 12_000_000.0);
        match comparison.input_energy_comparison() {
            InputEnergyComparison::Comparable {
                candidate_service_per_input_ratio,
                basis,
                ..
            } => {
                assert_eq!(basis, &InputEnergyBasis::FuelChemicalLhv);
                assert!((candidate_service_per_input_ratio - 30.0 / 27.0).abs() < 1e-12);
            }
            other => panic!("expected comparable input bases, got {other:?}"),
        }
    }

    #[test]
    fn rejects_ac_and_dc_comparison_as_different_service_boundaries() {
        let ac = boundary(ServiceInterface::NetAc, "facility-ac-main");
        let dc = boundary(ServiceInterface::RegulatedDc, "facility-800vdc-bus");
        let demand = contract(ac.clone());
        let reference = trace("ac-system", "microturbine", "DATA-CENTER-DUTY", ac, [6e6, 6e6], None);
        let candidate = trace("dc-system", "dc-source", "DATA-CENTER-DUTY", dc, [6e6, 6e6], None);

        assert!(matches!(
            compare_delivered_service(
                &demand,
                &reference,
                &candidate,
                ServiceDeliveryTolerance::new(0.0, 0.0).unwrap()
            ),
            Err(ServiceContractError::BoundaryMismatch { system }) if system == "dc-system"
        ));
    }

    #[test]
    fn different_energy_input_bases_do_not_emit_an_efficiency_ratio() {
        let b = boundary(ServiceInterface::RegulatedDc, "data-center-800vdc");
        let demand = contract(b.clone());
        let reference = trace(
            "microturbine-rectifier",
            "microturbine-plus-rectifier",
            "DATA-CENTER-DUTY",
            b.clone(),
            [6e6, 6e6],
            Some(
                InputEnergyRecord::new(
                    InputEnergyBasis::FuelChemicalLhv,
                    30e6,
                    None,
                )
                .unwrap(),
            ),
        );
        let candidate = trace(
            "battery-storage",
            "battery-inverter",
            "DATA-CENTER-DUTY",
            b,
            [6e6, 6e6],
            Some(
                InputEnergyRecord::new(
                    InputEnergyBasis::StoredElectricalEnergyAtInitialSoc,
                    28e6,
                    None,
                )
                .unwrap(),
            ),
        );

        let comparison = compare_delivered_service(
            &demand,
            &reference,
            &candidate,
            ServiceDeliveryTolerance::new(0.0, 0.0).unwrap(),
        )
        .unwrap();

        assert_eq!(comparison.common_service_energy_j(), 12e6);
        assert!(matches!(
            comparison.input_energy_comparison(),
            InputEnergyComparison::DifferentInputBases { .. }
        ));
    }

    #[test]
    fn propulsion_work_is_a_distinct_typed_boundary() {
        let b = boundary(
            ServiceInterface::PropulsionWork(PropulsionWorkStage::Propulsor),
            "vessel-propulsor-shaft",
        );
        let demand = contract(b.clone());
        let reference = trace("diesel-drive", "diesel-engine-propulsor", "DATA-CENTER-DUTY", b.clone(), [6e6, 6e6], None);
        let candidate = trace("battery-drive", "battery-electric-propulsor", "DATA-CENTER-DUTY", b, [6e6, 6e6], None);
        let comparison = compare_delivered_service(
            &demand,
            &reference,
            &candidate,
            ServiceDeliveryTolerance::new(0.0, 0.0).unwrap(),
        )
        .unwrap();
        assert_eq!(
            comparison.boundary().interface(),
            ServiceInterface::PropulsionWork(PropulsionWorkStage::Propulsor)
        );
    }

    #[test]
    fn rejects_under_delivery_and_does_not_credit_surplus() {
        let b = boundary(ServiceInterface::NetAc, "site-ac-pcc");
        let demand = contract(b.clone());
        let reference = trace("reference", "engine-gen", "DATA-CENTER-DUTY", b.clone(), [7e6, 6e6], None);
        let deficit = trace("deficit", "small-engine-gen", "DATA-CENTER-DUTY", b.clone(), [5e6, 6e6], None);
        assert!(matches!(
            compare_delivered_service(
                &demand,
                &reference,
                &deficit,
                ServiceDeliveryTolerance::new(0.0, 0.0).unwrap()
            ),
            Err(ServiceContractError::ServiceDeficit { .. })
        ));

        let sufficient = trace("candidate", "engine-gen", "DATA-CENTER-DUTY", b, [6e6, 6e6], None);
        let comparison = compare_delivered_service(
            &demand,
            &reference,
            &sufficient,
            ServiceDeliveryTolerance::new(0.0, 0.0).unwrap(),
        )
        .unwrap();
        assert_eq!(comparison.common_service_energy_j(), 12e6);
        assert_eq!(comparison.reference_surplus_energy_j(), 1e6);
        assert_eq!(comparison.candidate_surplus_energy_j(), 0.0);
    }

    #[test]
    fn rejects_overflowing_input_energy_ratio() {
        let b = boundary(ServiceInterface::NetAc, "site-ac-pcc");
        let demand = contract(b.clone());
        let reference = trace(
            "reference",
            "engine-gen",
            "DATA-CENTER-DUTY",
            b.clone(),
            [6e6, 6e6],
            Some(
                InputEnergyRecord::new(InputEnergyBasis::FuelChemicalLhv, 1.0e308, None)
                    .unwrap(),
            ),
        );
        let candidate = trace(
            "candidate",
            "engine-gen",
            "DATA-CENTER-DUTY",
            b,
            [6e6, 6e6],
            Some(
                InputEnergyRecord::new(InputEnergyBasis::FuelChemicalLhv, 1.0e-308, None)
                    .unwrap(),
            ),
        );

        assert!(matches!(
            compare_delivered_service(
                &demand,
                &reference,
                &candidate,
                ServiceDeliveryTolerance::new(0.0, 0.0).unwrap()
            ),
            Err(ServiceContractError::InvalidInputEnergyRatio { .. })
        ));
    }

    #[test]
    fn rejects_profile_identity_and_window_mismatch() {
        let b = boundary(ServiceInterface::NetAc, "site-ac-pcc");
        let demand = contract(b.clone());
        let wrong_profile = trace("wrong-profile", "engine-gen", "other-profile", b.clone(), [6e6, 6e6], None);
        let candidate = trace("candidate", "engine-gen", "DATA-CENTER-DUTY", b.clone(), [6e6, 6e6], None);
        let reference = trace("reference", "engine-gen", "DATA-CENTER-DUTY", b.clone(), [6e6, 6e6], None);

        assert!(matches!(
            compare_delivered_service(
                &demand,
                &wrong_profile,
                &candidate,
                ServiceDeliveryTolerance::new(0.0, 0.0).unwrap()
            ),
            Err(ServiceContractError::ProfileIdentityMismatch { .. })
        ));

        let shifted = DeliveredServiceTrace::new(
            "shifted",
            "engine-gen",
            "sha256:shifted",
            "DATA-CENTER-DUTY",
            "v1",
            b,
            vec![
                DeliveredServiceSegment::new(window(1, 61), 6e6).unwrap(),
                DeliveredServiceSegment::new(window(61, 121), 6e6).unwrap(),
            ],
            None,
        )
        .unwrap();
        assert!(matches!(
            compare_delivered_service(
                &demand,
                &reference,
                &shifted,
                ServiceDeliveryTolerance::new(0.0, 0.0).unwrap()
            ),
            Err(ServiceContractError::TimeWindowMismatch { .. })
        ));
    }

    #[test]
    fn rejects_invalid_identity_and_input_energy() {
        assert!(ServiceBoundary::new(ServiceInterface::NetAc, " ", "spec").is_err());
        assert!(ServiceBoundary::new(ServiceInterface::NetAc, "bus", " ").is_err());
        assert!(InputEnergyRecord::new(InputEnergyBasis::FuelChemicalLhv, 0.0, None).is_err());
        assert!(InputEnergyRecord::new(
            InputEnergyBasis::FuelChemicalLhv,
            1.0,
            Some(" ".to_string())
        )
        .is_err());
        assert!(
            InputEnergyRecord::new(InputEnergyBasis::Other(" ".to_string()), 1.0, None).is_err()
        );
        assert!(ServiceDeliveryTolerance::new(0.0, 1.01).is_err());
    }
}
