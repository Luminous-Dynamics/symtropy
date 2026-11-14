// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Typed, evidence-aware interfaces for composing power-system modules.
//!
//! This module deliberately implements a narrow connection contract, not
//! electrical protection, installation design, cable sizing, or device
//! physics. A compatible result means only that the declared ports overlap
//! under the supplied policy. It is not permission to energize or work on
//! equipment.

use serde::Serialize;

/// Broad role of a Power Fabric module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ModuleClass {
    Source,
    Storage,
    Conversion,
    Protection,
    Distribution,
    Load,
    Thermal,
    Measurement,
    Control,
}

/// Evidence class for one module-port capability claim.
///
/// These classes are descriptive rather than a universal quality ranking.
/// A caller must explicitly allow the classes suitable for the claim being
/// assessed. Unknown is never admissible.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ModuleEvidenceClass {
    Unknown,
    ManufacturerDeclared,
    Modelled,
    Measured,
    IndependentlyVerified,
    FieldQualified,
}

/// Evidence supporting a port capability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EvidenceClaim {
    class: ModuleEvidenceClass,
    evidence_id: Option<String>,
}

impl EvidenceClaim {
    /// Create a claim. Known claims require a non-empty evidence identifier.
    pub fn new(
        class: ModuleEvidenceClass,
        evidence_id: Option<String>,
    ) -> Result<Self, PortContractError> {
        match class {
            ModuleEvidenceClass::Unknown => {
                if evidence_id.is_some() {
                    return Err(PortContractError::UnexpectedEvidenceForUnknownClaim);
                }
                Ok(Self {
                    class,
                    evidence_id: None,
                })
            }
            _ => {
                let Some(id) = evidence_id else {
                    return Err(PortContractError::MissingEvidenceId);
                };
                if id.trim().is_empty() {
                    return Err(PortContractError::MissingEvidenceId);
                }
                Ok(Self {
                    class,
                    evidence_id: Some(id),
                })
            }
        }
    }

    /// Explicit unknown claim; it cannot satisfy any compatibility policy.
    pub const fn unknown() -> Self {
        Self {
            class: ModuleEvidenceClass::Unknown,
            evidence_id: None,
        }
    }

    pub const fn class(&self) -> ModuleEvidenceClass {
        self.class
    }

    pub fn evidence_id(&self) -> Option<&str> {
        self.evidence_id.as_deref()
    }
}

/// Caller-supplied evidence admissibility policy.
///
/// The evaluator does not infer that one evidence class is universally better
/// than another. Each compatibility claim must declare which classes are
/// admissible for that specific evaluation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EvidencePolicy {
    accepted_classes: Vec<ModuleEvidenceClass>,
}

impl EvidencePolicy {
    pub fn new(accepted_classes: Vec<ModuleEvidenceClass>) -> Self {
        Self {
            accepted_classes: accepted_classes
                .into_iter()
                .filter(|class| *class != ModuleEvidenceClass::Unknown)
                .collect(),
        }
    }

    pub fn accepts(&self, claim: &EvidenceClaim) -> bool {
        claim.class != ModuleEvidenceClass::Unknown
            && claim.evidence_id.as_deref().is_some_and(|id| !id.trim().is_empty())
            && self.accepted_classes.contains(&claim.class)
    }

    pub fn accepted_classes(&self) -> &[ModuleEvidenceClass] {
        &self.accepted_classes
    }
}

/// Stable identity and configuration state for an installed or proposed module.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ModuleIdentity {
    module_id: String,
    hardware_revision: String,
    configuration_digest: String,
    configuration_current: bool,
}

impl ModuleIdentity {
    pub fn new(
        module_id: impl Into<String>,
        hardware_revision: impl Into<String>,
        configuration_digest: impl Into<String>,
        configuration_current: bool,
    ) -> Result<Self, PortContractError> {
        let module_id = module_id.into();
        let hardware_revision = hardware_revision.into();
        let configuration_digest = configuration_digest.into();

        for (field, value) in [
            ("module_id", module_id.as_str()),
            ("hardware_revision", hardware_revision.as_str()),
            ("configuration_digest", configuration_digest.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(PortContractError::MissingIdentityField { field });
            }
        }

        Ok(Self {
            module_id,
            hardware_revision,
            configuration_digest,
            configuration_current,
        })
    }

    pub fn module_id(&self) -> &str {
        &self.module_id
    }

    pub fn hardware_revision(&self) -> &str {
        &self.hardware_revision
    }

    pub fn configuration_digest(&self) -> &str {
        &self.configuration_digest
    }

    pub const fn configuration_current(&self) -> bool {
        self.configuration_current
    }
}

/// Direction in which power may flow through one electrical port.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum PortDirection {
    Input,
    Output,
    Bidirectional,
}

/// Closed interval with optional bounds.
///
/// None means unknown, not zero and not unbounded. Compatibility requires
/// both bounds to be present on both sides.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct QuantityRange {
    minimum: Option<f64>,
    maximum: Option<f64>,
}

impl QuantityRange {
    pub fn new(
        minimum: Option<f64>,
        maximum: Option<f64>,
    ) -> Result<Self, PortContractError> {
        for value in [minimum, maximum].into_iter().flatten() {
            if !value.is_finite() {
                return Err(PortContractError::NonFiniteRangeBound { value });
            }
        }

        if let (Some(minimum), Some(maximum)) = (minimum, maximum)
            && minimum > maximum
        {
            return Err(PortContractError::ReversedRange { minimum, maximum });
        }

        Ok(Self { minimum, maximum })
    }

    pub const fn unknown() -> Self {
        Self {
            minimum: None,
            maximum: None,
        }
    }

    pub const fn minimum(self) -> Option<f64> {
        self.minimum
    }

    pub const fn maximum(self) -> Option<f64> {
        self.maximum
    }

    pub const fn is_complete(self) -> bool {
        self.minimum.is_some() && self.maximum.is_some()
    }

    fn intersection(self, other: Self) -> Option<Self> {
        let (Some(left_min), Some(left_max), Some(right_min), Some(right_max)) = (
            self.minimum,
            self.maximum,
            other.minimum,
            other.maximum,
        ) else {
            return None;
        };

        let minimum = left_min.max(right_min);
        let maximum = left_max.min(right_max);
        if minimum > maximum {
            return None;
        }

        Some(Self {
            minimum: Some(minimum),
            maximum: Some(maximum),
        })
    }
}

/// Fixed or reversible DC polarity reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum DcPolarity {
    PositiveToNegative,
    NegativeToPositive,
    Reversible,
}

/// AC port capability declaration.
///
/// All numeric quantities use SI units: volts, hertz, amperes and watts.
/// Unknown values remain None and block the affected compatibility claim.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AcPort {
    voltage_v: QuantityRange,
    frequency_hz: QuantityRange,
    phase_count: Option<u8>,
    maximum_current_a: Option<f64>,
    maximum_power_w: Option<f64>,
    evidence: EvidenceClaim,
}

impl AcPort {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        voltage_v: QuantityRange,
        frequency_hz: QuantityRange,
        phase_count: Option<u8>,
        maximum_current_a: Option<f64>,
        maximum_power_w: Option<f64>,
        evidence: EvidenceClaim,
    ) -> Result<Self, PortContractError> {
        if phase_count == Some(0) {
            return Err(PortContractError::InvalidPhaseCount);
        }
        validate_positive_limit("maximum_current_a", maximum_current_a)?;
        validate_positive_limit("maximum_power_w", maximum_power_w)?;
        validate_positive_range(voltage_v)?;
        validate_positive_range(frequency_hz)?;

        Ok(Self {
            voltage_v,
            frequency_hz,
            phase_count,
            maximum_current_a,
            maximum_power_w,
            evidence,
        })
    }

    pub const fn voltage_v(&self) -> QuantityRange {
        self.voltage_v
    }

    pub const fn frequency_hz(&self) -> QuantityRange {
        self.frequency_hz
    }

    pub const fn phase_count(&self) -> Option<u8> {
        self.phase_count
    }

    pub const fn maximum_current_a(&self) -> Option<f64> {
        self.maximum_current_a
    }

    pub const fn maximum_power_w(&self) -> Option<f64> {
        self.maximum_power_w
    }

    pub const fn evidence(&self) -> &EvidenceClaim {
        &self.evidence
    }
}

/// DC port capability declaration.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DcPort {
    voltage_v: QuantityRange,
    polarity: Option<DcPolarity>,
    maximum_current_a: Option<f64>,
    maximum_power_w: Option<f64>,
    evidence: EvidenceClaim,
}

impl DcPort {
    pub fn new(
        voltage_v: QuantityRange,
        polarity: Option<DcPolarity>,
        maximum_current_a: Option<f64>,
        maximum_power_w: Option<f64>,
        evidence: EvidenceClaim,
    ) -> Result<Self, PortContractError> {
        validate_positive_limit("maximum_current_a", maximum_current_a)?;
        validate_positive_limit("maximum_power_w", maximum_power_w)?;
        validate_positive_range(voltage_v)?;

        Ok(Self {
            voltage_v,
            polarity,
            maximum_current_a,
            maximum_power_w,
            evidence,
        })
    }

    pub const fn voltage_v(&self) -> QuantityRange {
        self.voltage_v
    }

    pub const fn polarity(&self) -> Option<DcPolarity> {
        self.polarity
    }

    pub const fn maximum_current_a(&self) -> Option<f64> {
        self.maximum_current_a
    }

    pub const fn maximum_power_w(&self) -> Option<f64> {
        self.maximum_power_w
    }

    pub const fn evidence(&self) -> &EvidenceClaim {
        &self.evidence
    }
}

/// Electrical interface type. AC and DC remain distinct even at equal voltage.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum ElectricalInterface {
    Ac(AcPort),
    Dc(DcPort),
}

/// One explicitly identified electrical port on a module.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ModulePort {
    port_id: String,
    direction: PortDirection,
    interface: ElectricalInterface,
}

impl ModulePort {
    pub fn new(
        port_id: impl Into<String>,
        direction: PortDirection,
        interface: ElectricalInterface,
    ) -> Result<Self, PortContractError> {
        let port_id = port_id.into();
        if port_id.trim().is_empty() {
            return Err(PortContractError::MissingPortId);
        }
        Ok(Self {
            port_id,
            direction,
            interface,
        })
    }

    pub fn port_id(&self) -> &str {
        &self.port_id
    }

    pub const fn direction(&self) -> PortDirection {
        self.direction
    }

    pub const fn interface(&self) -> &ElectricalInterface {
        &self.interface
    }
}

/// A module contract containing identity, class and typed electrical ports.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PowerModule {
    identity: ModuleIdentity,
    module_class: ModuleClass,
    ports: Vec<ModulePort>,
}

impl PowerModule {
    pub fn new(
        identity: ModuleIdentity,
        module_class: ModuleClass,
        ports: Vec<ModulePort>,
    ) -> Result<Self, PortContractError> {
        if ports.is_empty() {
            return Err(PortContractError::ModuleHasNoPorts);
        }

        for (index, port) in ports.iter().enumerate() {
            if ports[..index]
                .iter()
                .any(|prior| prior.port_id == port.port_id)
            {
                return Err(PortContractError::DuplicatePortId);
            }
        }

        Ok(Self {
            identity,
            module_class,
            ports,
        })
    }

    pub const fn identity(&self) -> &ModuleIdentity {
        &self.identity
    }

    pub const fn module_class(&self) -> ModuleClass {
        self.module_class
    }

    pub fn ports(&self) -> &[ModulePort] {
        &self.ports
    }

    pub fn port(&self, port_id: &str) -> Option<&ModulePort> {
        self.ports.iter().find(|port| port.port_id == port_id)
    }
}

/// Fail-closed connection outcomes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum CompatibilityStatus {
    CompatibleWithinDeclaredEnvelope,
    Incompatible,
    UnknownInsufficientEvidence,
    StaleConfigurationRequiresReconciliation,
}

/// Stable machine-readable reason codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum CompatibilityReason {
    LeftConfigurationStale,
    RightConfigurationStale,
    LeftPortNotFound,
    RightPortNotFound,
    PortTypeMismatch,
    FlowDirectionMismatch,
    LeftEvidenceNotAdmissible,
    RightEvidenceNotAdmissible,
    MissingVoltageRange,
    DisjointVoltageRange,
    MissingFrequencyRange,
    DisjointFrequencyRange,
    MissingPhaseCount,
    PhaseMismatch,
    MissingCurrentLimit,
    MissingPowerLimit,
    MissingDcPolarity,
    DcPolarityMismatch,
}

/// Common declared port intersection returned only after all required checks pass.
///
/// The current and power limits are independent maxima taken from the two
/// declarations. They are not a coupled voltage-current operating curve or a
/// substitute for device-specific power-factor, thermal, protection or
/// transient envelopes.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ConnectionEnvelope {
    interface: ElectricalInterfaceKind,
    voltage_v: QuantityRange,
    frequency_hz: Option<QuantityRange>,
    phase_count: Option<u8>,
    dc_polarity: Option<DcPolarity>,
    maximum_current_a: f64,
    maximum_power_w: f64,
}

impl ConnectionEnvelope {
    pub const fn interface(&self) -> ElectricalInterfaceKind {
        self.interface
    }

    pub const fn voltage_v(&self) -> QuantityRange {
        self.voltage_v
    }

    pub const fn frequency_hz(&self) -> Option<QuantityRange> {
        self.frequency_hz
    }

    pub const fn phase_count(&self) -> Option<u8> {
        self.phase_count
    }

    pub const fn dc_polarity(&self) -> Option<DcPolarity> {
        self.dc_polarity
    }

    pub const fn maximum_current_a(&self) -> f64 {
        self.maximum_current_a
    }

    pub const fn maximum_power_w(&self) -> f64 {
        self.maximum_power_w
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ElectricalInterfaceKind {
    Ac,
    Dc,
}

/// Result of a deterministic module-port compatibility assessment.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CompatibilityAssessment {
    status: CompatibilityStatus,
    reasons: Vec<CompatibilityReason>,
    envelope: Option<ConnectionEnvelope>,
}

impl CompatibilityAssessment {
    pub const fn status(&self) -> CompatibilityStatus {
        self.status
    }

    pub fn reasons(&self) -> &[CompatibilityReason] {
        &self.reasons
    }

    pub fn envelope(&self) -> Option<&ConnectionEnvelope> {
        self.envelope.as_ref()
    }
}

/// Errors from creating malformed module/port contracts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PortContractError {
    MissingIdentityField { field: &'static str },
    MissingPortId,
    MissingEvidenceId,
    UnexpectedEvidenceForUnknownClaim,
    InvalidPhaseCount,
    InvalidPositiveLimit { field: &'static str, value: f64 },
    InvalidRangeBound { value: f64 },
    NonFiniteRangeBound { value: f64 },
    ReversedRange { minimum: f64, maximum: f64 },
    ModuleHasNoPorts,
    DuplicatePortId,
}

impl core::fmt::Display for PortContractError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::MissingIdentityField { field } => {
                write!(f, "module identity field {field} must not be empty")
            }
            Self::MissingPortId => write!(f, "port_id must not be empty"),
            Self::MissingEvidenceId => write!(f, "known evidence claim requires evidence_id"),
            Self::UnexpectedEvidenceForUnknownClaim => {
                write!(f, "unknown evidence claim must not carry an evidence_id")
            }
            Self::InvalidPhaseCount => write!(f, "phase_count must be greater than zero"),
            Self::InvalidPositiveLimit { field, value } => {
                write!(f, "{field} must be finite and positive; got {value}")
            }
            Self::InvalidRangeBound { value } => {
                write!(f, "range bound must be positive; got {value}")
            }
            Self::NonFiniteRangeBound { value } => {
                write!(f, "range bound must be finite; got {value}")
            }
            Self::ReversedRange { minimum, maximum } => {
                write!(f, "range minimum {minimum} exceeds maximum {maximum}")
            }
            Self::ModuleHasNoPorts => write!(f, "module must declare at least one port"),
            Self::DuplicatePortId => write!(f, "module contains a duplicate port id"),
        }
    }
}

impl std::error::Error for PortContractError {}

fn validate_positive_limit(
    field: &'static str,
    value: Option<f64>,
) -> Result<(), PortContractError> {
    if let Some(value) = value
        && (!value.is_finite() || value <= 0.0)
    {
        return Err(PortContractError::InvalidPositiveLimit { field, value });
    }
    Ok(())
}

fn validate_positive_range(range: QuantityRange) -> Result<(), PortContractError> {
    for value in [range.minimum, range.maximum].into_iter().flatten() {
        if value <= 0.0 {
            return Err(PortContractError::InvalidRangeBound { value });
        }
    }
    Ok(())
}

fn direction_is_compatible(left: PortDirection, right: PortDirection) -> bool {
    matches!(
        (left, right),
        (PortDirection::Output, PortDirection::Input)
            | (PortDirection::Input, PortDirection::Output)
            | (PortDirection::Bidirectional, PortDirection::Input)
            | (PortDirection::Input, PortDirection::Bidirectional)
            | (PortDirection::Output, PortDirection::Bidirectional)
            | (PortDirection::Bidirectional, PortDirection::Output)
            | (PortDirection::Bidirectional, PortDirection::Bidirectional)
    )
}

fn status_for_reasons(reasons: &[CompatibilityReason]) -> CompatibilityStatus {
    if reasons.iter().any(|reason| {
        matches!(
            reason,
            CompatibilityReason::LeftConfigurationStale
                | CompatibilityReason::RightConfigurationStale
        )
    }) {
        CompatibilityStatus::StaleConfigurationRequiresReconciliation
    } else if reasons.iter().any(|reason| {
        matches!(
            reason,
            CompatibilityReason::PortTypeMismatch
                | CompatibilityReason::FlowDirectionMismatch
                | CompatibilityReason::DisjointVoltageRange
                | CompatibilityReason::DisjointFrequencyRange
                | CompatibilityReason::PhaseMismatch
                | CompatibilityReason::DcPolarityMismatch
        )
    }) {
        CompatibilityStatus::Incompatible
    } else if reasons.is_empty() {
        CompatibilityStatus::CompatibleWithinDeclaredEnvelope
    } else {
        CompatibilityStatus::UnknownInsufficientEvidence
    }
}

/// Assess one connection between two module ports.
///
/// The evaluator is deterministic and read-only. It does not select protective
/// devices, validate a complete installation, authorize energization, or
/// establish that an installation is safe to touch.
pub fn assess_connection(
    left_module: &PowerModule,
    left_port_id: &str,
    right_module: &PowerModule,
    right_port_id: &str,
    evidence_policy: &EvidencePolicy,
) -> CompatibilityAssessment {
    let mut reasons = Vec::new();

    if !left_module.identity.configuration_current {
        reasons.push(CompatibilityReason::LeftConfigurationStale);
    }
    if !right_module.identity.configuration_current {
        reasons.push(CompatibilityReason::RightConfigurationStale);
    }
    if !reasons.is_empty() {
        return CompatibilityAssessment {
            status: status_for_reasons(&reasons),
            reasons,
            envelope: None,
        };
    }

    let Some(left_port) = left_module.port(left_port_id) else {
        reasons.push(CompatibilityReason::LeftPortNotFound);
        return CompatibilityAssessment {
            status: CompatibilityStatus::UnknownInsufficientEvidence,
            reasons,
            envelope: None,
        };
    };
    let Some(right_port) = right_module.port(right_port_id) else {
        reasons.push(CompatibilityReason::RightPortNotFound);
        return CompatibilityAssessment {
            status: CompatibilityStatus::UnknownInsufficientEvidence,
            reasons,
            envelope: None,
        };
    };

    if !direction_is_compatible(left_port.direction, right_port.direction) {
        reasons.push(CompatibilityReason::FlowDirectionMismatch);
    }

    let envelope = match (&left_port.interface, &right_port.interface) {
        (ElectricalInterface::Ac(left), ElectricalInterface::Ac(right)) => {
            assess_ac_ports(left, right, evidence_policy, &mut reasons)
        }
        (ElectricalInterface::Dc(left), ElectricalInterface::Dc(right)) => {
            assess_dc_ports(left, right, evidence_policy, &mut reasons)
        }
        _ => {
            reasons.push(CompatibilityReason::PortTypeMismatch);
            None
        }
    };

    let status = status_for_reasons(&reasons);
    CompatibilityAssessment {
        status,
        reasons,
        envelope: if status == CompatibilityStatus::CompatibleWithinDeclaredEnvelope {
            envelope
        } else {
            None
        },
    }
}

fn assess_ac_ports(
    left: &AcPort,
    right: &AcPort,
    evidence_policy: &EvidencePolicy,
    reasons: &mut Vec<CompatibilityReason>,
) -> Option<ConnectionEnvelope> {
    if !evidence_policy.accepts(&left.evidence) {
        reasons.push(CompatibilityReason::LeftEvidenceNotAdmissible);
    }
    if !evidence_policy.accepts(&right.evidence) {
        reasons.push(CompatibilityReason::RightEvidenceNotAdmissible);
    }

    let voltage = match left.voltage_v.intersection(right.voltage_v) {
        Some(range) => Some(range),
        None if !left.voltage_v.is_complete() || !right.voltage_v.is_complete() => {
            reasons.push(CompatibilityReason::MissingVoltageRange);
            None
        }
        None => {
            reasons.push(CompatibilityReason::DisjointVoltageRange);
            None
        }
    };

    let frequency = match left.frequency_hz.intersection(right.frequency_hz) {
        Some(range) => Some(range),
        None if !left.frequency_hz.is_complete() || !right.frequency_hz.is_complete() => {
            reasons.push(CompatibilityReason::MissingFrequencyRange);
            None
        }
        None => {
            reasons.push(CompatibilityReason::DisjointFrequencyRange);
            None
        }
    };

    let phase_count = match (left.phase_count, right.phase_count) {
        (Some(left), Some(right)) if left == right => Some(left),
        (Some(_), Some(_)) => {
            reasons.push(CompatibilityReason::PhaseMismatch);
            None
        }
        _ => {
            reasons.push(CompatibilityReason::MissingPhaseCount);
            None
        }
    };

    let current = match (left.maximum_current_a, right.maximum_current_a) {
        (Some(left), Some(right)) => Some(left.min(right)),
        _ => {
            reasons.push(CompatibilityReason::MissingCurrentLimit);
            None
        }
    };
    let power = match (left.maximum_power_w, right.maximum_power_w) {
        (Some(left), Some(right)) => Some(left.min(right)),
        _ => {
            reasons.push(CompatibilityReason::MissingPowerLimit);
            None
        }
    };

    match (voltage, frequency, phase_count, current, power) {
        (Some(voltage_v), Some(frequency_hz), Some(phase_count), Some(current), Some(power))
            if reasons.is_empty() =>
        {
            Some(ConnectionEnvelope {
                interface: ElectricalInterfaceKind::Ac,
                voltage_v,
                frequency_hz: Some(frequency_hz),
                phase_count: Some(phase_count),
                dc_polarity: None,
                maximum_current_a: current,
                maximum_power_w: power,
            })
        }
        _ => None,
    }
}

fn assess_dc_ports(
    left: &DcPort,
    right: &DcPort,
    evidence_policy: &EvidencePolicy,
    reasons: &mut Vec<CompatibilityReason>,
) -> Option<ConnectionEnvelope> {
    if !evidence_policy.accepts(&left.evidence) {
        reasons.push(CompatibilityReason::LeftEvidenceNotAdmissible);
    }
    if !evidence_policy.accepts(&right.evidence) {
        reasons.push(CompatibilityReason::RightEvidenceNotAdmissible);
    }

    let voltage = match left.voltage_v.intersection(right.voltage_v) {
        Some(range) => Some(range),
        None if !left.voltage_v.is_complete() || !right.voltage_v.is_complete() => {
            reasons.push(CompatibilityReason::MissingVoltageRange);
            None
        }
        None => {
            reasons.push(CompatibilityReason::DisjointVoltageRange);
            None
        }
    };

    let polarity = match (left.polarity, right.polarity) {
        (Some(left), Some(right)) if left == right => Some(left),
        (Some(DcPolarity::Reversible), Some(other))
        | (Some(other), Some(DcPolarity::Reversible)) => Some(other),
        (Some(_), Some(_)) => {
            reasons.push(CompatibilityReason::DcPolarityMismatch);
            None
        }
        _ => {
            reasons.push(CompatibilityReason::MissingDcPolarity);
            None
        }
    };

    let current = match (left.maximum_current_a, right.maximum_current_a) {
        (Some(left), Some(right)) => Some(left.min(right)),
        _ => {
            reasons.push(CompatibilityReason::MissingCurrentLimit);
            None
        }
    };
    let power = match (left.maximum_power_w, right.maximum_power_w) {
        (Some(left), Some(right)) => Some(left.min(right)),
        _ => {
            reasons.push(CompatibilityReason::MissingPowerLimit);
            None
        }
    };

    match (voltage, polarity, current, power) {
        (Some(voltage_v), Some(dc_polarity), Some(current), Some(power)) if reasons.is_empty() => {
            Some(ConnectionEnvelope {
                interface: ElectricalInterfaceKind::Dc,
                voltage_v,
                frequency_hz: None,
                phase_count: None,
                dc_polarity: Some(dc_polarity),
                maximum_current_a: current,
                maximum_power_w: power,
            })
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn evidence(class: ModuleEvidenceClass, id: &str) -> EvidenceClaim {
        EvidenceClaim::new(class, Some(id.to_owned())).unwrap()
    }

    fn policy() -> EvidencePolicy {
        EvidencePolicy::new(vec![
            ModuleEvidenceClass::Measured,
            ModuleEvidenceClass::IndependentlyVerified,
            ModuleEvidenceClass::FieldQualified,
        ])
    }

    fn identity(id: &str, current: bool) -> ModuleIdentity {
        ModuleIdentity::new(id, "rev-a", format!("sha256:{id}"), current).unwrap()
    }

    #[allow(clippy::too_many_arguments)]
    fn ac_port(
        id: &str,
        direction: PortDirection,
        voltage: (f64, f64),
        frequency: (f64, f64),
        phase_count: Option<u8>,
        current: Option<f64>,
        power: Option<f64>,
        claim: EvidenceClaim,
    ) -> ModulePort {
        ModulePort::new(
            id,
            direction,
            ElectricalInterface::Ac(
                AcPort::new(
                    QuantityRange::new(Some(voltage.0), Some(voltage.1)).unwrap(),
                    QuantityRange::new(Some(frequency.0), Some(frequency.1)).unwrap(),
                    phase_count,
                    current,
                    power,
                    claim,
                )
                .unwrap(),
            ),
        )
        .unwrap()
    }

    fn dc_port(
        id: &str,
        direction: PortDirection,
        voltage: (f64, f64),
        polarity: Option<DcPolarity>,
        current: Option<f64>,
        power: Option<f64>,
        claim: EvidenceClaim,
    ) -> ModulePort {
        ModulePort::new(
            id,
            direction,
            ElectricalInterface::Dc(
                DcPort::new(
                    QuantityRange::new(Some(voltage.0), Some(voltage.1)).unwrap(),
                    polarity,
                    current,
                    power,
                    claim,
                )
                .unwrap(),
            ),
        )
        .unwrap()
    }

    fn module(id: &str, current: bool, ports: Vec<ModulePort>) -> PowerModule {
        PowerModule::new(identity(id, current), ModuleClass::Conversion, ports).unwrap()
    }

    #[test]
    fn unknown_evidence_rejects_even_an_empty_supplied_identifier() {
        assert!(matches!(
            EvidenceClaim::new(ModuleEvidenceClass::Unknown, Some(String::new())),
            Err(PortContractError::UnexpectedEvidenceForUnknownClaim)
        ));
    }

    #[test]
    fn ac_ports_match_and_return_intersection_envelope() {
        let source = module(
            "source",
            true,
            vec![ac_port(
                "out",
                PortDirection::Output,
                (210.0, 250.0),
                (49.0, 51.0),
                Some(1),
                Some(32.0),
                Some(7_000.0),
                evidence(ModuleEvidenceClass::Measured, "meter-source-01"),
            )],
        );
        let load = module(
            "load",
            true,
            vec![ac_port(
                "in",
                PortDirection::Input,
                (220.0, 240.0),
                (49.5, 50.5),
                Some(1),
                Some(20.0),
                Some(4_000.0),
                evidence(ModuleEvidenceClass::IndependentlyVerified, "test-load-01"),
            )],
        );

        let result = assess_connection(&source, "out", &load, "in", &policy());
        assert_eq!(
            result.status(),
            CompatibilityStatus::CompatibleWithinDeclaredEnvelope
        );
        let envelope = result.envelope().unwrap();
        assert_eq!(envelope.interface(), ElectricalInterfaceKind::Ac);
        assert_eq!(envelope.voltage_v().minimum(), Some(220.0));
        assert_eq!(envelope.voltage_v().maximum(), Some(240.0));
        assert_eq!(envelope.frequency_hz().unwrap().minimum(), Some(49.5));
        assert_eq!(envelope.maximum_current_a(), 20.0);
        assert_eq!(envelope.maximum_power_w(), 4_000.0);
    }

    #[test]
    fn equal_numeric_voltage_does_not_make_ac_and_dc_compatible() {
        let ac = module(
            "ac-source",
            true,
            vec![ac_port(
                "out",
                PortDirection::Output,
                (48.0, 48.0),
                (50.0, 50.0),
                Some(1),
                Some(10.0),
                Some(480.0),
                evidence(ModuleEvidenceClass::Measured, "ac-test"),
            )],
        );
        let dc = module(
            "dc-load",
            true,
            vec![dc_port(
                "in",
                PortDirection::Input,
                (48.0, 48.0),
                Some(DcPolarity::PositiveToNegative),
                Some(10.0),
                Some(480.0),
                evidence(ModuleEvidenceClass::Measured, "dc-test"),
            )],
        );

        let result = assess_connection(&ac, "out", &dc, "in", &policy());
        assert_eq!(result.status(), CompatibilityStatus::Incompatible);
        assert!(result.reasons().contains(&CompatibilityReason::PortTypeMismatch));
        assert!(result.envelope().is_none());
    }

    #[test]
    fn disjoint_voltage_ranges_are_incompatible() {
        let source = module(
            "source",
            true,
            vec![dc_port(
                "out",
                PortDirection::Output,
                (42.0, 54.0),
                Some(DcPolarity::PositiveToNegative),
                Some(30.0),
                Some(1_500.0),
                evidence(ModuleEvidenceClass::Measured, "source-test"),
            )],
        );
        let load = module(
            "load",
            true,
            vec![dc_port(
                "in",
                PortDirection::Input,
                (200.0, 240.0),
                Some(DcPolarity::PositiveToNegative),
                Some(10.0),
                Some(2_000.0),
                evidence(ModuleEvidenceClass::Measured, "load-test"),
            )],
        );

        let result = assess_connection(&source, "out", &load, "in", &policy());
        assert_eq!(result.status(), CompatibilityStatus::Incompatible);
        assert!(result.reasons().contains(&CompatibilityReason::DisjointVoltageRange));
    }

    #[test]
    fn mismatched_dc_polarity_is_incompatible() {
        let source = module(
            "source",
            true,
            vec![dc_port(
                "out",
                PortDirection::Output,
                (40.0, 60.0),
                Some(DcPolarity::PositiveToNegative),
                Some(30.0),
                Some(1_500.0),
                evidence(ModuleEvidenceClass::Measured, "source-test"),
            )],
        );
        let load = module(
            "load",
            true,
            vec![dc_port(
                "in",
                PortDirection::Input,
                (48.0, 52.0),
                Some(DcPolarity::NegativeToPositive),
                Some(10.0),
                Some(500.0),
                evidence(ModuleEvidenceClass::Measured, "load-test"),
            )],
        );

        let result = assess_connection(&source, "out", &load, "in", &policy());
        assert_eq!(result.status(), CompatibilityStatus::Incompatible);
        assert!(result.reasons().contains(&CompatibilityReason::DcPolarityMismatch));
    }

    #[test]
    fn unknown_polarity_is_unknown_not_compatible() {
        let source = module(
            "source",
            true,
            vec![dc_port(
                "out",
                PortDirection::Output,
                (40.0, 60.0),
                None,
                Some(30.0),
                Some(1_500.0),
                evidence(ModuleEvidenceClass::Measured, "source-test"),
            )],
        );
        let load = module(
            "load",
            true,
            vec![dc_port(
                "in",
                PortDirection::Input,
                (48.0, 52.0),
                Some(DcPolarity::PositiveToNegative),
                Some(10.0),
                Some(500.0),
                evidence(ModuleEvidenceClass::Measured, "load-test"),
            )],
        );

        let result = assess_connection(&source, "out", &load, "in", &policy());
        assert_eq!(
            result.status(),
            CompatibilityStatus::UnknownInsufficientEvidence
        );
        assert!(result.reasons().contains(&CompatibilityReason::MissingDcPolarity));
    }

    #[test]
    fn direction_mismatch_blocks_two_outputs() {
        let left = module(
            "left",
            true,
            vec![dc_port(
                "port",
                PortDirection::Output,
                (40.0, 60.0),
                Some(DcPolarity::PositiveToNegative),
                Some(30.0),
                Some(1_500.0),
                evidence(ModuleEvidenceClass::Measured, "left-test"),
            )],
        );
        let right = module(
            "right",
            true,
            vec![dc_port(
                "port",
                PortDirection::Output,
                (40.0, 60.0),
                Some(DcPolarity::PositiveToNegative),
                Some(30.0),
                Some(1_500.0),
                evidence(ModuleEvidenceClass::Measured, "right-test"),
            )],
        );

        let result = assess_connection(&left, "port", &right, "port", &policy());
        assert_eq!(result.status(), CompatibilityStatus::Incompatible);
        assert!(result.reasons().contains(&CompatibilityReason::FlowDirectionMismatch));
    }

    #[test]
    fn reverse_argument_order_is_supported() {
        let source = module(
            "source",
            true,
            vec![dc_port(
                "out",
                PortDirection::Output,
                (40.0, 60.0),
                Some(DcPolarity::Reversible),
                Some(30.0),
                Some(1_500.0),
                evidence(ModuleEvidenceClass::Measured, "source-test"),
            )],
        );
        let load = module(
            "load",
            true,
            vec![dc_port(
                "in",
                PortDirection::Input,
                (48.0, 52.0),
                Some(DcPolarity::PositiveToNegative),
                Some(10.0),
                Some(500.0),
                evidence(ModuleEvidenceClass::Measured, "load-test"),
            )],
        );

        let result = assess_connection(&load, "in", &source, "out", &policy());
        assert_eq!(
            result.status(),
            CompatibilityStatus::CompatibleWithinDeclaredEnvelope
        );
        assert_eq!(
            result.envelope().unwrap().dc_polarity(),
            Some(DcPolarity::PositiveToNegative)
        );
    }

    #[test]
    fn caller_must_explicitly_admit_evidence_classes() {
        let source = module(
            "source",
            true,
            vec![dc_port(
                "out",
                PortDirection::Output,
                (40.0, 60.0),
                Some(DcPolarity::PositiveToNegative),
                Some(30.0),
                Some(1_500.0),
                evidence(ModuleEvidenceClass::ManufacturerDeclared, "datasheet-1"),
            )],
        );
        let load = module(
            "load",
            true,
            vec![dc_port(
                "in",
                PortDirection::Input,
                (48.0, 52.0),
                Some(DcPolarity::PositiveToNegative),
                Some(10.0),
                Some(500.0),
                evidence(ModuleEvidenceClass::Measured, "load-test"),
            )],
        );

        let result = assess_connection(
            &source,
            "out",
            &load,
            "in",
            &EvidencePolicy::new(vec![ModuleEvidenceClass::Measured]),
        );
        assert_eq!(
            result.status(),
            CompatibilityStatus::UnknownInsufficientEvidence
        );
        assert!(result.reasons().contains(&CompatibilityReason::LeftEvidenceNotAdmissible));
    }

    #[test]
    fn stale_configuration_requires_reconciliation_before_compatibility() {
        let source = module(
            "source",
            false,
            vec![dc_port(
                "out",
                PortDirection::Output,
                (40.0, 60.0),
                Some(DcPolarity::PositiveToNegative),
                Some(30.0),
                Some(1_500.0),
                evidence(ModuleEvidenceClass::Measured, "source-test"),
            )],
        );
        let load = module(
            "load",
            true,
            vec![dc_port(
                "in",
                PortDirection::Input,
                (48.0, 52.0),
                Some(DcPolarity::PositiveToNegative),
                Some(10.0),
                Some(500.0),
                evidence(ModuleEvidenceClass::Measured, "load-test"),
            )],
        );

        let result = assess_connection(&source, "out", &load, "in", &policy());
        assert_eq!(
            result.status(),
            CompatibilityStatus::StaleConfigurationRequiresReconciliation
        );
        assert!(result.envelope().is_none());
    }

    #[test]
    fn missing_limits_or_voltage_remain_unknown() {
        let source = PowerModule::new(
            identity("source", true),
            ModuleClass::Source,
            vec![ModulePort::new(
                "out",
                PortDirection::Output,
                ElectricalInterface::Dc(
                    DcPort::new(
                        QuantityRange::unknown(),
                        Some(DcPolarity::PositiveToNegative),
                        None,
                        Some(1_500.0),
                        evidence(ModuleEvidenceClass::Measured, "source-test"),
                    )
                    .unwrap(),
                ),
            )
            .unwrap()],
        )
        .unwrap();
        let load = module(
            "load",
            true,
            vec![dc_port(
                "in",
                PortDirection::Input,
                (48.0, 52.0),
                Some(DcPolarity::PositiveToNegative),
                Some(10.0),
                Some(500.0),
                evidence(ModuleEvidenceClass::Measured, "load-test"),
            )],
        );

        let result = assess_connection(&source, "out", &load, "in", &policy());
        assert_eq!(
            result.status(),
            CompatibilityStatus::UnknownInsufficientEvidence
        );
        assert!(result.reasons().contains(&CompatibilityReason::MissingVoltageRange));
        assert!(result.reasons().contains(&CompatibilityReason::MissingCurrentLimit));
    }

    #[test]
    fn malformed_identity_and_duplicate_ports_are_rejected() {
        assert!(matches!(
            ModuleIdentity::new(" ", "rev-a", "sha256:test", true),
            Err(PortContractError::MissingIdentityField { field: "module_id" })
        ));

        let first = dc_port(
            "same",
            PortDirection::Output,
            (40.0, 60.0),
            Some(DcPolarity::PositiveToNegative),
            Some(10.0),
            Some(500.0),
            evidence(ModuleEvidenceClass::Measured, "first"),
        );
        let second = first.clone();
        assert!(matches!(
            PowerModule::new(identity("dup", true), ModuleClass::Source, vec![first, second]),
            Err(PortContractError::DuplicatePortId)
        ));
    }

    #[test]
    fn ranges_reject_nan_and_reversed_bounds() {
        assert!(matches!(
            QuantityRange::new(Some(f64::NAN), Some(2.0)),
            Err(PortContractError::NonFiniteRangeBound { .. })
        ));
        assert!(matches!(
            QuantityRange::new(Some(2.0), Some(1.0)),
            Err(PortContractError::ReversedRange { .. })
        ));
    }

    #[test]
    fn ac_frequency_mismatch_and_phase_mismatch_are_rejected() {
        let source = module(
            "source",
            true,
            vec![ac_port(
                "out",
                PortDirection::Output,
                (200.0, 250.0),
                (49.0, 51.0),
                Some(3),
                Some(40.0),
                Some(20_000.0),
                evidence(ModuleEvidenceClass::Measured, "source"),
            )],
        );
        let load = module(
            "load",
            true,
            vec![ac_port(
                "in",
                PortDirection::Input,
                (210.0, 240.0),
                (55.0, 60.0),
                Some(1),
                Some(20.0),
                Some(10_000.0),
                evidence(ModuleEvidenceClass::Measured, "load"),
            )],
        );

        let result = assess_connection(&source, "out", &load, "in", &policy());
        assert_eq!(result.status(), CompatibilityStatus::Incompatible);
        assert!(result.reasons().contains(&CompatibilityReason::DisjointFrequencyRange));
        assert!(result.reasons().contains(&CompatibilityReason::PhaseMismatch));
    }
}
