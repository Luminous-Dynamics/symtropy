// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Carrier-neutral transport-path metadata and matched-useful-service gating.
//!
//! This module is a bounded composition contract, not a second topology graph,
//! energy ledger, physical solver, dispatch optimizer, or safety authority.
//! It answers only whether declared candidate paths describe the same useful
//! service under a matched scenario and measurement boundary. A COMPARABLE
//! result is metadata admissibility, not physical-performance evidence.

use super::{EvidenceClaim, EvidencePolicy, ModuleEvidenceClass};
use serde::Serialize;
use std::collections::BTreeSet;

/// Physical energy form carried across one declared transport primitive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum EnergyCarrier {
    Electrical,
    ElectromagneticField,
    Thermal,
    Mechanical,
    Acoustic,
    Chemical,
    Electrochemical,
    GravitationalPotential,
}

/// Physical mechanism by which energy crosses or traverses a path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum TransportMechanism {
    Conduction,
    /// Transient conduction through an explicitly ionized gas medium.
    /// Classification does not establish a useful-power advantage or safety.
    PlasmaChannelConduction,
    SuperconductingConduction,
    MagneticInduction,
    CapacitiveCoupling,
    RadiativePropagation,
    GuidedElectromagnetic,
    ThermalConduction,
    ThermalAdvection,
    PhaseChangeCapillary,
    ShaftDrive,
    HydraulicFlow,
    PneumaticFlow,
    AcousticWave,
    EnergyBearingLogistics,
}

/// Declared physical medium/path. This is not a carrier or mechanism.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum TransportMedium {
    Conductor,
    /// Ionized gas used as a transient conductive path; research-only by default.
    IonizedGas,
    SuperconductingCable,
    Soil,
    Coil,
    Electrodes,
    FreeSpace,
    Waveguide,
    Solid,
    Liquid,
    Gas,
    Pipe,
    Shaft,
    Road,
    Rail,
    Vessel,
}

/// Explicit transduction/conversion stage attached to a primitive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ConversionStage {
    Inverter,
    Rectifier,
    ElectroOptic,
    OpticalToElectric,
    MotorGenerator,
    HeatPump,
    Electrolysis,
    FuelCell,
    Combustion,
    CompressionExpansion,
    AcousticToElectric,
    OtherDeclaredTransduction,
}

/// Storage state is orthogonal to the transport mechanism.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum StorageState {
    None,
    Field,
    Battery,
    Thermal,
    Mechanical,
    Chemical,
    Gravitational,
}

/// An immutable, metadata-only description of one transport primitive.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TransportPrimitive {
    primitive_id: String,
    source_port_id: String,
    destination_port_id: String,
    carrier: EnergyCarrier,
    mechanism: TransportMechanism,
    medium: TransportMedium,
    conversion_stages: Vec<ConversionStage>,
    storage_state: StorageState,
    operating_regime_profile_digest: String,
    evidence: EvidenceClaim,
}

/// Complete constructor input for one primitive. Unknown evidence remains
/// explicit through the existing EvidenceClaim::unknown() representation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TransportPrimitiveSpec {
    pub primitive_id: String,
    pub source_port_id: String,
    pub destination_port_id: String,
    pub carrier: EnergyCarrier,
    pub mechanism: TransportMechanism,
    pub medium: TransportMedium,
    pub conversion_stages: Vec<ConversionStage>,
    pub storage_state: StorageState,
    pub operating_regime_profile_digest: String,
    pub evidence: EvidenceClaim,
}

impl TransportPrimitive {
    pub fn new(spec: TransportPrimitiveSpec) -> Result<Self, TransportContractError> {
        for (field, value) in [
            ("primitive_id", spec.primitive_id.as_str()),
            ("source_port_id", spec.source_port_id.as_str()),
            ("destination_port_id", spec.destination_port_id.as_str()),
            (
                "operating_regime_profile_digest",
                spec.operating_regime_profile_digest.as_str(),
            ),
        ] {
            if value.trim().is_empty() {
                return Err(TransportContractError::MissingField { field });
            }
        }

        if spec.source_port_id == spec.destination_port_id {
            return Err(TransportContractError::SelfReferentialPorts {
                port_id: spec.source_port_id,
            });
        }

        if !valid_carrier_mechanism_medium(spec.carrier, spec.mechanism, spec.medium) {
            return Err(TransportContractError::InvalidPhysicalCombination {
                carrier: spec.carrier,
                mechanism: spec.mechanism,
                medium: spec.medium,
            });
        }

        Ok(Self {
            primitive_id: spec.primitive_id,
            source_port_id: spec.source_port_id,
            destination_port_id: spec.destination_port_id,
            carrier: spec.carrier,
            mechanism: spec.mechanism,
            medium: spec.medium,
            conversion_stages: spec.conversion_stages,
            storage_state: spec.storage_state,
            operating_regime_profile_digest: spec.operating_regime_profile_digest,
            evidence: spec.evidence,
        })
    }

    pub fn primitive_id(&self) -> &str {
        &self.primitive_id
    }

    pub fn source_port_id(&self) -> &str {
        &self.source_port_id
    }

    pub fn destination_port_id(&self) -> &str {
        &self.destination_port_id
    }

    pub const fn carrier(&self) -> EnergyCarrier {
        self.carrier
    }

    pub const fn mechanism(&self) -> TransportMechanism {
        self.mechanism
    }

    pub const fn medium(&self) -> TransportMedium {
        self.medium
    }

    pub fn conversion_stages(&self) -> &[ConversionStage] {
        &self.conversion_stages
    }

    pub const fn storage_state(&self) -> StorageState {
        self.storage_state
    }

    pub fn operating_regime_profile_digest(&self) -> &str {
        &self.operating_regime_profile_digest
    }

    pub const fn evidence(&self) -> &EvidenceClaim {
        &self.evidence
    }
}

/// A linear composition view over primitive identities already identified by
/// an owning graph or experiment. It does not perform path search.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TransportPathContract {
    path_id: String,
    source_port_id: String,
    useful_service_port_id: String,
    primitives: Vec<TransportPrimitive>,
}

impl TransportPathContract {
    pub fn new(
        path_id: impl Into<String>,
        source_port_id: impl Into<String>,
        useful_service_port_id: impl Into<String>,
        primitives: Vec<TransportPrimitive>,
    ) -> Result<Self, TransportContractError> {
        let path_id = path_id.into();
        let source_port_id = source_port_id.into();
        let useful_service_port_id = useful_service_port_id.into();

        for (field, value) in [
            ("path_id", path_id.as_str()),
            ("source_port_id", source_port_id.as_str()),
            ("useful_service_port_id", useful_service_port_id.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(TransportContractError::MissingField { field });
            }
        }

        if primitives.is_empty() {
            return Err(TransportContractError::EmptyPath { path_id });
        }

        let mut primitive_ids = BTreeSet::new();
        for primitive in &primitives {
            if !primitive_ids.insert(primitive.primitive_id().to_owned()) {
                return Err(TransportContractError::DuplicatePrimitiveId {
                    primitive_id: primitive.primitive_id().to_owned(),
                });
            }
        }

        let actual_source = primitives[0].source_port_id();
        if actual_source != source_port_id {
            return Err(TransportContractError::SourcePortMismatch {
                expected: source_port_id,
                actual: actual_source.to_owned(),
            });
        }

        for pair in primitives.windows(2) {
            let previous = &pair[0];
            let next = &pair[1];
            if previous.destination_port_id() != next.source_port_id() {
                return Err(TransportContractError::DiscontinuousPath {
                    previous_primitive_id: previous.primitive_id().to_owned(),
                    next_primitive_id: next.primitive_id().to_owned(),
                    expected_port: previous.destination_port_id().to_owned(),
                    actual_port: next.source_port_id().to_owned(),
                });
            }
        }

        let actual_destination = primitives
            .last()
            .expect("non-empty path checked above")
            .destination_port_id();
        if actual_destination != useful_service_port_id {
            return Err(TransportContractError::DestinationPortMismatch {
                expected: useful_service_port_id,
                actual: actual_destination.to_owned(),
            });
        }

        Ok(Self {
            path_id,
            source_port_id,
            useful_service_port_id,
            primitives,
        })
    }

    pub fn path_id(&self) -> &str {
        &self.path_id
    }

    pub fn source_port_id(&self) -> &str {
        &self.source_port_id
    }

    pub fn useful_service_port_id(&self) -> &str {
        &self.useful_service_port_id
    }

    pub fn primitives(&self) -> &[TransportPrimitive] {
        &self.primitives
    }
}

/// Useful service required at the receiving boundary.
///
/// Quality and boundary profiles are digests of separately versioned
/// contracts. For example, a heat service profile must bind temperature grade;
/// a shaft-work profile must bind torque/speed; an optical profile must bind
/// spectrum; and a chemical-feedstock profile must bind product specification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UsefulServiceContract {
    service_id: String,
    kind: UsefulServiceKind,
    quality_profile_digest: Option<String>,
    duty_cycle_digest: Option<String>,
    scenario_digest: Option<String>,
    location_environment_digest: Option<String>,
    source_boundary_profile_digest: Option<String>,
    useful_service_boundary_profile_digest: Option<String>,
}

/// Useful service classes. These are endpoints, not transport carriers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum UsefulServiceKind {
    ElectricalLoad,
    ShaftWork,
    Traction,
    Heat,
    Cooling,
    OpticalRadiantOutput,
    ChemicalFeedstock,
    AcousticOutput,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UsefulServiceSpec {
    pub service_id: String,
    pub kind: UsefulServiceKind,
    pub quality_profile_digest: Option<String>,
    pub duty_cycle_digest: Option<String>,
    pub scenario_digest: Option<String>,
    pub location_environment_digest: Option<String>,
    pub source_boundary_profile_digest: Option<String>,
    pub useful_service_boundary_profile_digest: Option<String>,
}

impl UsefulServiceContract {
    pub fn new(spec: UsefulServiceSpec) -> Result<Self, TransportContractError> {
        if spec.service_id.trim().is_empty() {
            return Err(TransportContractError::MissingField {
                field: "service_id",
            });
        }

        for (field, value) in [
            (
                "quality_profile_digest",
                spec.quality_profile_digest.as_deref(),
            ),
            ("duty_cycle_digest", spec.duty_cycle_digest.as_deref()),
            ("scenario_digest", spec.scenario_digest.as_deref()),
            (
                "location_environment_digest",
                spec.location_environment_digest.as_deref(),
            ),
            (
                "source_boundary_profile_digest",
                spec.source_boundary_profile_digest.as_deref(),
            ),
            (
                "useful_service_boundary_profile_digest",
                spec.useful_service_boundary_profile_digest.as_deref(),
            ),
        ] {
            if value.is_some_and(|value| value.trim().is_empty()) {
                return Err(TransportContractError::MissingField { field });
            }
        }

        Ok(Self {
            service_id: spec.service_id,
            kind: spec.kind,
            quality_profile_digest: spec.quality_profile_digest,
            duty_cycle_digest: spec.duty_cycle_digest,
            scenario_digest: spec.scenario_digest,
            location_environment_digest: spec.location_environment_digest,
            source_boundary_profile_digest: spec.source_boundary_profile_digest,
            useful_service_boundary_profile_digest: spec.useful_service_boundary_profile_digest,
        })
    }

    pub fn service_id(&self) -> &str {
        &self.service_id
    }

    pub const fn kind(&self) -> UsefulServiceKind {
        self.kind
    }
}

/// A candidate path and the service it claims to provide.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TransportPathCandidate {
    candidate_id: String,
    path: TransportPathContract,
    service: UsefulServiceContract,
}

impl TransportPathCandidate {
    pub fn new(
        candidate_id: impl Into<String>,
        path: TransportPathContract,
        service: UsefulServiceContract,
    ) -> Result<Self, TransportContractError> {
        let candidate_id = candidate_id.into();
        if candidate_id.trim().is_empty() {
            return Err(TransportContractError::MissingField {
                field: "candidate_id",
            });
        }
        Ok(Self {
            candidate_id,
            path,
            service,
        })
    }

    pub fn candidate_id(&self) -> &str {
        &self.candidate_id
    }

    pub fn path(&self) -> &TransportPathContract {
        &self.path
    }

    pub fn service(&self) -> &UsefulServiceContract {
        &self.service
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ComparisonField {
    ServiceKind,
    QualityProfile,
    DutyCycle,
    Scenario,
    LocationEnvironment,
    SourceBoundary,
    UsefulServiceBoundary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum MatchedServiceReason {
    MissingField {
        candidate_id: String,
        field: ComparisonField,
    },
    MismatchedField {
        field: ComparisonField,
    },
    EvidenceNotAdmissible {
        candidate_id: String,
        primitive_id: String,
        evidence_class: ModuleEvidenceClass,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum MatchedServiceComparisonStatus {
    Comparable,
    NotComparable,
    Unresolved,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MatchedServiceAssessment {
    status: MatchedServiceComparisonStatus,
    reasons: Vec<MatchedServiceReason>,
    ordered_candidate_ids: Vec<String>,
}

impl MatchedServiceAssessment {
    pub const fn status(&self) -> MatchedServiceComparisonStatus {
        self.status
    }

    pub fn reasons(&self) -> &[MatchedServiceReason] {
        &self.reasons
    }

    pub fn ordered_candidate_ids(&self) -> &[String] {
        &self.ordered_candidate_ids
    }
}

/// Assess metadata-level comparability for candidate paths.
///
/// The evaluator does not compute efficiency, exergy, cost, service quality or
/// safety. Those values remain with the relevant measurement, energy-accounting
/// and domain-specific owners. Candidate order is canonicalized by candidate
/// ID so reason ordering is deterministic.
pub fn assess_matched_service(
    candidates: &[TransportPathCandidate],
    evidence_policy: &EvidencePolicy,
) -> Result<MatchedServiceAssessment, TransportContractError> {
    if candidates.len() < 2 {
        return Err(TransportContractError::InsufficientCandidates {
            count: candidates.len(),
        });
    }

    let mut ordered: Vec<&TransportPathCandidate> = candidates.iter().collect();
    ordered.sort_by(|left, right| left.candidate_id.cmp(&right.candidate_id));

    let mut candidate_ids = BTreeSet::new();
    for candidate in &ordered {
        if !candidate_ids.insert(candidate.candidate_id.clone()) {
            return Err(TransportContractError::DuplicateCandidateId {
                candidate_id: candidate.candidate_id.clone(),
            });
        }
    }

    let mut reasons = Vec::new();

    let baseline_kind = ordered[0].service.kind;
    if ordered
        .iter()
        .any(|candidate| candidate.service.kind != baseline_kind)
    {
        reasons.push(MatchedServiceReason::MismatchedField {
            field: ComparisonField::ServiceKind,
        });
    }

    macro_rules! compare_optional_field {
        ($field:expr, $getter:expr) => {{
            let mut known_values: Vec<String> = Vec::new();
            for candidate in &ordered {
                match ($getter)(&candidate.service) {
                    Some(value) => known_values.push(value),
                    None => reasons.push(MatchedServiceReason::MissingField {
                        candidate_id: candidate.candidate_id.clone(),
                        field: $field,
                    }),
                }
            }

            if let Some(first) = known_values.first() {
                if known_values.iter().any(|value| value != first) {
                    reasons.push(MatchedServiceReason::MismatchedField { field: $field });
                }
            }
        }};
    }

    compare_optional_field!(
        ComparisonField::QualityProfile,
        |service: &UsefulServiceContract| service.quality_profile_digest.clone()
    );
    compare_optional_field!(
        ComparisonField::DutyCycle,
        |service: &UsefulServiceContract| service.duty_cycle_digest.clone()
    );
    compare_optional_field!(
        ComparisonField::Scenario,
        |service: &UsefulServiceContract| service.scenario_digest.clone()
    );
    compare_optional_field!(
        ComparisonField::LocationEnvironment,
        |service: &UsefulServiceContract| service.location_environment_digest.clone()
    );
    compare_optional_field!(
        ComparisonField::SourceBoundary,
        |service: &UsefulServiceContract| service.source_boundary_profile_digest.clone()
    );
    compare_optional_field!(
        ComparisonField::UsefulServiceBoundary,
        |service: &UsefulServiceContract| service.useful_service_boundary_profile_digest.clone()
    );

    for candidate in &ordered {
        for primitive in candidate.path.primitives() {
            if !evidence_policy.accepts(primitive.evidence()) {
                reasons.push(MatchedServiceReason::EvidenceNotAdmissible {
                    candidate_id: candidate.candidate_id.clone(),
                    primitive_id: primitive.primitive_id().to_owned(),
                    evidence_class: primitive.evidence().class(),
                });
            }
        }
    }

    let has_known_mismatch = reasons
        .iter()
        .any(|reason| matches!(reason, MatchedServiceReason::MismatchedField { .. }));
    let status = if has_known_mismatch {
        MatchedServiceComparisonStatus::NotComparable
    } else if !reasons.is_empty() {
        MatchedServiceComparisonStatus::Unresolved
    } else {
        MatchedServiceComparisonStatus::Comparable
    };

    Ok(MatchedServiceAssessment {
        status,
        reasons,
        ordered_candidate_ids: ordered
            .iter()
            .map(|candidate| candidate.candidate_id.clone())
            .collect(),
    })
}

fn valid_carrier_mechanism_medium(
    carrier: EnergyCarrier,
    mechanism: TransportMechanism,
    medium: TransportMedium,
) -> bool {
    match (carrier, mechanism) {
        (EnergyCarrier::Electrical, TransportMechanism::Conduction) => {
            matches!(medium, TransportMedium::Conductor | TransportMedium::Soil)
        }
        (
            EnergyCarrier::Electrical,
            TransportMechanism::PlasmaChannelConduction,
        ) => medium == TransportMedium::IonizedGas,
        (EnergyCarrier::Electrical, TransportMechanism::SuperconductingConduction) => {
            medium == TransportMedium::SuperconductingCable
        }
        (EnergyCarrier::ElectromagneticField, TransportMechanism::MagneticInduction) => {
            medium == TransportMedium::Coil
        }
        (EnergyCarrier::ElectromagneticField, TransportMechanism::CapacitiveCoupling) => {
            medium == TransportMedium::Electrodes
        }
        (EnergyCarrier::ElectromagneticField, TransportMechanism::RadiativePropagation) => {
            medium == TransportMedium::FreeSpace
        }
        (EnergyCarrier::ElectromagneticField, TransportMechanism::GuidedElectromagnetic) => {
            medium == TransportMedium::Waveguide
        }
        (EnergyCarrier::Thermal, TransportMechanism::ThermalConduction) => {
            medium == TransportMedium::Solid
        }
        (EnergyCarrier::Thermal, TransportMechanism::ThermalAdvection) => matches!(
            medium,
            TransportMedium::Liquid | TransportMedium::Gas | TransportMedium::Pipe
        ),
        (EnergyCarrier::Thermal, TransportMechanism::PhaseChangeCapillary) => {
            medium == TransportMedium::Pipe
        }
        (EnergyCarrier::Mechanical, TransportMechanism::ShaftDrive) => {
            medium == TransportMedium::Shaft
        }
        (EnergyCarrier::Mechanical, TransportMechanism::HydraulicFlow) => {
            matches!(medium, TransportMedium::Liquid | TransportMedium::Pipe)
        }
        (EnergyCarrier::Mechanical, TransportMechanism::PneumaticFlow) => {
            matches!(medium, TransportMedium::Gas | TransportMedium::Pipe)
        }
        (EnergyCarrier::Acoustic, TransportMechanism::AcousticWave) => matches!(
            medium,
            TransportMedium::Solid | TransportMedium::Liquid | TransportMedium::Gas
        ),
        (
            EnergyCarrier::Chemical
            | EnergyCarrier::Electrochemical
            | EnergyCarrier::GravitationalPotential,
            TransportMechanism::EnergyBearingLogistics,
        ) => matches!(
            medium,
            TransportMedium::Pipe
                | TransportMedium::Road
                | TransportMedium::Rail
                | TransportMedium::Vessel
        ),
        _ => false,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransportContractError {
    MissingField {
        field: &'static str,
    },
    SelfReferentialPorts {
        port_id: String,
    },
    InvalidPhysicalCombination {
        carrier: EnergyCarrier,
        mechanism: TransportMechanism,
        medium: TransportMedium,
    },
    EmptyPath {
        path_id: String,
    },
    DuplicatePrimitiveId {
        primitive_id: String,
    },
    SourcePortMismatch {
        expected: String,
        actual: String,
    },
    DiscontinuousPath {
        previous_primitive_id: String,
        next_primitive_id: String,
        expected_port: String,
        actual_port: String,
    },
    DestinationPortMismatch {
        expected: String,
        actual: String,
    },
    InsufficientCandidates {
        count: usize,
    },
    DuplicateCandidateId {
        candidate_id: String,
    },
}

impl core::fmt::Display for TransportContractError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::MissingField { field } => write!(f, "{field} must not be empty"),
            Self::SelfReferentialPorts { port_id } => {
                write!(
                    f,
                    "transport primitive cannot connect port {port_id} to itself"
                )
            }
            Self::InvalidPhysicalCombination {
                carrier,
                mechanism,
                medium,
            } => write!(
                f,
                "unsupported carrier/mechanism/medium combination: {carrier:?}/{mechanism:?}/{medium:?}"
            ),
            Self::EmptyPath { path_id } => {
                write!(
                    f,
                    "transport path {path_id} must contain at least one primitive"
                )
            }
            Self::DuplicatePrimitiveId { primitive_id } => {
                write!(f, "duplicate transport primitive id {primitive_id}")
            }
            Self::SourcePortMismatch { expected, actual } => write!(
                f,
                "transport path source port mismatch: expected {expected}, got {actual}"
            ),
            Self::DiscontinuousPath {
                previous_primitive_id,
                next_primitive_id,
                expected_port,
                actual_port,
            } => write!(
                f,
                "path discontinuity between {previous_primitive_id} and {next_primitive_id}: expected port {expected_port}, got {actual_port}"
            ),
            Self::DestinationPortMismatch { expected, actual } => write!(
                f,
                "transport path destination mismatch: expected {expected}, got {actual}"
            ),
            Self::InsufficientCandidates { count } => {
                write!(
                    f,
                    "matched-service comparison requires at least two candidates, got {count}"
                )
            }
            Self::DuplicateCandidateId { candidate_id } => {
                write!(f, "duplicate transport candidate id {candidate_id}")
            }
        }
    }
}

impl std::error::Error for TransportContractError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn verified_evidence(id: &str) -> EvidenceClaim {
        EvidenceClaim::new(
            ModuleEvidenceClass::IndependentlyVerified,
            Some(id.to_owned()),
        )
        .unwrap()
    }

    fn primitive(
        id: &str,
        source: &str,
        destination: &str,
        carrier: EnergyCarrier,
        mechanism: TransportMechanism,
        medium: TransportMedium,
    ) -> TransportPrimitive {
        TransportPrimitive::new(TransportPrimitiveSpec {
            primitive_id: id.to_owned(),
            source_port_id: source.to_owned(),
            destination_port_id: destination.to_owned(),
            carrier,
            mechanism,
            medium,
            conversion_stages: Vec::new(),
            storage_state: StorageState::None,
            operating_regime_profile_digest: format!("regime-{id}"),
            evidence: verified_evidence(&format!("evidence-{id}")),
        })
        .unwrap()
    }

    fn service(service_id: &str) -> UsefulServiceContract {
        UsefulServiceContract::new(UsefulServiceSpec {
            service_id: service_id.to_owned(),
            kind: UsefulServiceKind::ElectricalLoad,
            quality_profile_digest: Some("quality-regulated-dc-48v".to_owned()),
            duty_cycle_digest: Some("duty-10min-on-5min-off".to_owned()),
            scenario_digest: Some("scenario-remote-sensor-node-v1".to_owned()),
            location_environment_digest: Some("environment-indoor-dry-ambient-v1".to_owned()),
            source_boundary_profile_digest: Some("source-site-ac-input-v1".to_owned()),
            useful_service_boundary_profile_digest: Some(
                "receiver-regulated-dc-output-v1".to_owned(),
            ),
        })
        .unwrap()
    }

    fn candidate(
        candidate_id: &str,
        primitive: TransportPrimitive,
        service: UsefulServiceContract,
    ) -> TransportPathCandidate {
        let source = primitive.source_port_id().to_owned();
        let destination = primitive.destination_port_id().to_owned();
        let path = TransportPathContract::new(
            format!("path-{candidate_id}"),
            source,
            destination,
            vec![primitive],
        )
        .unwrap();
        TransportPathCandidate::new(candidate_id, path, service).unwrap()
    }

    fn policy() -> EvidencePolicy {
        EvidencePolicy::new(vec![ModuleEvidenceClass::IndependentlyVerified])
    }

    #[test]
    fn wired_inductive_and_photonic_paths_can_match_one_electrical_service() {
        let wired = candidate(
            "wired",
            primitive(
                "wire",
                "site-source",
                "load",
                EnergyCarrier::Electrical,
                TransportMechanism::Conduction,
                TransportMedium::Conductor,
            ),
            service("service-wired"),
        );
        let inductive = candidate(
            "inductive",
            primitive(
                "ipt",
                "site-source",
                "load",
                EnergyCarrier::ElectromagneticField,
                TransportMechanism::MagneticInduction,
                TransportMedium::Coil,
            ),
            service("service-inductive"),
        );
        let photonic = candidate(
            "photonic",
            primitive(
                "laser",
                "site-source",
                "load",
                EnergyCarrier::ElectromagneticField,
                TransportMechanism::RadiativePropagation,
                TransportMedium::FreeSpace,
            ),
            service("service-photonic"),
        );

        let assessment = assess_matched_service(&[photonic, wired, inductive], &policy()).unwrap();

        assert_eq!(
            assessment.status(),
            MatchedServiceComparisonStatus::Comparable
        );
        assert!(assessment.reasons().is_empty());
        assert_eq!(
            assessment.ordered_candidate_ids(),
            &[
                "inductive".to_owned(),
                "photonic".to_owned(),
                "wired".to_owned()
            ]
        );
    }

    #[test]
    fn optical_and_rf_share_radiative_mechanism_but_keep_distinct_regime_profiles() {
        let optical = primitive(
            "optical",
            "source",
            "receiver",
            EnergyCarrier::ElectromagneticField,
            TransportMechanism::RadiativePropagation,
            TransportMedium::FreeSpace,
        );
        let rf = primitive(
            "rf",
            "source",
            "receiver",
            EnergyCarrier::ElectromagneticField,
            TransportMechanism::RadiativePropagation,
            TransportMedium::FreeSpace,
        );

        assert_ne!(
            optical.operating_regime_profile_digest(),
            rf.operating_regime_profile_digest()
        );
    }

    #[test]
    fn plasma_channel_is_electrical_conduction_in_ionized_gas_not_a_new_carrier() {
        let plasma = TransportPrimitive::new(TransportPrimitiveSpec {
            primitive_id: "plasma-channel".to_owned(),
            source_port_id: "pulse-source".to_owned(),
            destination_port_id: "load".to_owned(),
            carrier: EnergyCarrier::Electrical,
            mechanism: TransportMechanism::PlasmaChannelConduction,
            medium: TransportMedium::IonizedGas,
            conversion_stages: Vec::new(),
            storage_state: StorageState::None,
            operating_regime_profile_digest: "transient-plasma-regime-v1".to_owned(),
            evidence: verified_evidence("plasma-literature-case"),
        })
        .unwrap();

        assert_eq!(plasma.carrier(), EnergyCarrier::Electrical);
        assert_eq!(
            plasma.mechanism(),
            TransportMechanism::PlasmaChannelConduction
        );
        assert_eq!(plasma.medium(), TransportMedium::IonizedGas);
        // A valid physical classification is not a deployability or safety pass.
        let unqualified = EvidencePolicy::new(vec![ModuleEvidenceClass::FieldQualified]);
        assert!(!unqualified.accepts(plasma.evidence()));
    }

    #[test]
    fn rejects_invalid_carrier_mechanism_medium_combinations() {
        let result = TransportPrimitive::new(TransportPrimitiveSpec {
            primitive_id: "invalid-medium-pair".to_owned(),
            source_port_id: "source".to_owned(),
            destination_port_id: "receiver".to_owned(),
            carrier: EnergyCarrier::ElectromagneticField,
            mechanism: TransportMechanism::AcousticWave,
            medium: TransportMedium::FreeSpace,
            conversion_stages: Vec::new(),
            storage_state: StorageState::None,
            operating_regime_profile_digest: "regime".to_owned(),
            evidence: verified_evidence("evidence"),
        });

        assert!(matches!(
            result,
            Err(TransportContractError::InvalidPhysicalCombination { .. })
        ));
    }

    #[test]
    fn rejects_empty_and_self_referential_primitive_ports() {
        let mut spec = TransportPrimitiveSpec {
            primitive_id: "p".to_owned(),
            source_port_id: "same".to_owned(),
            destination_port_id: "same".to_owned(),
            carrier: EnergyCarrier::Electrical,
            mechanism: TransportMechanism::Conduction,
            medium: TransportMedium::Conductor,
            conversion_stages: Vec::new(),
            storage_state: StorageState::None,
            operating_regime_profile_digest: "regime".to_owned(),
            evidence: verified_evidence("e"),
        };
        assert!(matches!(
            TransportPrimitive::new(spec.clone()),
            Err(TransportContractError::SelfReferentialPorts { .. })
        ));
        spec.primitive_id.clear();
        spec.destination_port_id = "destination".to_owned();
        assert!(matches!(
            TransportPrimitive::new(spec),
            Err(TransportContractError::MissingField { .. })
        ));
    }

    #[test]
    fn validates_multi_stage_port_continuity() {
        let first = primitive(
            "first",
            "source",
            "intermediate",
            EnergyCarrier::Electrical,
            TransportMechanism::Conduction,
            TransportMedium::Conductor,
        );
        let second = primitive(
            "second",
            "intermediate",
            "load",
            EnergyCarrier::Electrical,
            TransportMechanism::Conduction,
            TransportMedium::Conductor,
        );
        let path =
            TransportPathContract::new("two-stage", "source", "load", vec![first, second]).unwrap();

        assert_eq!(path.primitives().len(), 2);
        assert_eq!(path.source_port_id(), "source");
        assert_eq!(path.useful_service_port_id(), "load");
    }

    #[test]
    fn rejects_discontinuous_multi_stage_path() {
        let first = primitive(
            "first",
            "source",
            "port-a",
            EnergyCarrier::Electrical,
            TransportMechanism::Conduction,
            TransportMedium::Conductor,
        );
        let second = primitive(
            "second",
            "port-b",
            "load",
            EnergyCarrier::Electrical,
            TransportMechanism::Conduction,
            TransportMedium::Conductor,
        );

        assert!(matches!(
            TransportPathContract::new("broken", "source", "load", vec![first, second]),
            Err(TransportContractError::DiscontinuousPath { .. })
        ));
    }

    #[test]
    fn rejects_duplicate_primitive_ids_and_wrong_terminal_port() {
        let first = primitive(
            "same",
            "source",
            "middle",
            EnergyCarrier::Electrical,
            TransportMechanism::Conduction,
            TransportMedium::Conductor,
        );
        let second = primitive(
            "same",
            "middle",
            "load",
            EnergyCarrier::Electrical,
            TransportMechanism::Conduction,
            TransportMedium::Conductor,
        );

        assert!(matches!(
            TransportPathContract::new("duplicate", "source", "load", vec![first.clone(), second]),
            Err(TransportContractError::DuplicatePrimitiveId { .. })
        ));
        assert!(matches!(
            TransportPathContract::new("wrong-end", "source", "other-load", vec![first]),
            Err(TransportContractError::DestinationPortMismatch { .. })
        ));
    }

    #[test]
    fn rejects_empty_path() {
        assert!(matches!(
            TransportPathContract::new("empty", "source", "load", Vec::new()),
            Err(TransportContractError::EmptyPath { .. })
        ));
    }

    #[test]
    fn missing_service_profile_is_unresolved_not_imputed() {
        let mut incomplete = service("incomplete");
        incomplete.quality_profile_digest = None;

        let first = candidate(
            "a",
            primitive(
                "a-wire",
                "source",
                "load",
                EnergyCarrier::Electrical,
                TransportMechanism::Conduction,
                TransportMedium::Conductor,
            ),
            incomplete,
        );
        let second = candidate(
            "b",
            primitive(
                "b-ipt",
                "source",
                "load",
                EnergyCarrier::ElectromagneticField,
                TransportMechanism::MagneticInduction,
                TransportMedium::Coil,
            ),
            service("complete"),
        );

        let assessment = assess_matched_service(&[first, second], &policy()).unwrap();
        assert_eq!(
            assessment.status(),
            MatchedServiceComparisonStatus::Unresolved
        );
        assert!(assessment.reasons().iter().any(|reason| matches!(
            reason,
            MatchedServiceReason::MissingField {
                field: ComparisonField::QualityProfile,
                ..
            }
        )));
    }

    #[test]
    fn any_known_service_or_boundary_mismatch_is_not_comparable() {
        let first = candidate(
            "a",
            primitive(
                "a-wire",
                "source",
                "load",
                EnergyCarrier::Electrical,
                TransportMechanism::Conduction,
                TransportMedium::Conductor,
            ),
            service("a"),
        );
        let mut different = service("b");
        different.quality_profile_digest = Some("quality-heat-90c".to_owned());
        different.useful_service_boundary_profile_digest =
            Some("boundary-thermal-output".to_owned());
        let second = candidate(
            "b",
            primitive(
                "b-pipe",
                "source",
                "load",
                EnergyCarrier::Thermal,
                TransportMechanism::ThermalAdvection,
                TransportMedium::Pipe,
            ),
            different,
        );

        let assessment = assess_matched_service(&[first, second], &policy()).unwrap();
        assert_eq!(
            assessment.status(),
            MatchedServiceComparisonStatus::NotComparable
        );
        assert!(assessment.reasons().iter().any(|reason| matches!(
            reason,
            MatchedServiceReason::MismatchedField {
                field: ComparisonField::QualityProfile
            }
        )));
        assert!(assessment.reasons().iter().any(|reason| matches!(
            reason,
            MatchedServiceReason::MismatchedField {
                field: ComparisonField::UsefulServiceBoundary
            }
        )));
    }

    #[test]
    fn mismatch_in_each_service_and_boundary_dimension_is_not_comparable() {
        type ServiceMutation = (ComparisonField, fn(&mut UsefulServiceContract));
        let mutations: [ServiceMutation; 7] = [
            (ComparisonField::ServiceKind, |service| {
                service.kind = UsefulServiceKind::Heat;
            }),
            (ComparisonField::QualityProfile, |service| {
                service.quality_profile_digest = Some("different-quality".to_owned());
            }),
            (ComparisonField::DutyCycle, |service| {
                service.duty_cycle_digest = Some("different-duty".to_owned());
            }),
            (ComparisonField::Scenario, |service| {
                service.scenario_digest = Some("different-scenario".to_owned());
            }),
            (ComparisonField::LocationEnvironment, |service| {
                service.location_environment_digest = Some("different-environment".to_owned());
            }),
            (ComparisonField::SourceBoundary, |service| {
                service.source_boundary_profile_digest =
                    Some("different-source-boundary".to_owned());
            }),
            (ComparisonField::UsefulServiceBoundary, |service| {
                service.useful_service_boundary_profile_digest =
                    Some("different-useful-boundary".to_owned());
            }),
        ];

        for (field, mutate) in mutations {
            let first = candidate(
                "a",
                primitive(
                    "wire-a",
                    "source",
                    "load",
                    EnergyCarrier::Electrical,
                    TransportMechanism::Conduction,
                    TransportMedium::Conductor,
                ),
                service("service-a"),
            );
            let mut changed_service = service("service-b");
            mutate(&mut changed_service);
            let second = candidate(
                "b",
                primitive(
                    "wire-b",
                    "source",
                    "load",
                    EnergyCarrier::Electrical,
                    TransportMechanism::Conduction,
                    TransportMedium::Conductor,
                ),
                changed_service,
            );

            let assessment = assess_matched_service(&[first, second], &policy()).unwrap();
            assert_eq!(
                assessment.status(),
                MatchedServiceComparisonStatus::NotComparable,
                "expected mismatch in {field:?}"
            );
            assert!(assessment.reasons().iter().any(|reason| matches!(
                reason,
                MatchedServiceReason::MismatchedField { field: actual } if *actual == field
            )));
        }
    }

    #[test]
    fn evidence_outside_caller_policy_is_unresolved() {
        let mut untrusted = primitive(
            "wire",
            "source",
            "load",
            EnergyCarrier::Electrical,
            TransportMechanism::Conduction,
            TransportMedium::Conductor,
        );
        untrusted.evidence = EvidenceClaim::new(
            ModuleEvidenceClass::ManufacturerDeclared,
            Some("vendor".to_owned()),
        )
        .unwrap();

        let first = candidate("a", untrusted, service("a"));
        let second = candidate(
            "b",
            primitive(
                "ipt",
                "source",
                "load",
                EnergyCarrier::ElectromagneticField,
                TransportMechanism::MagneticInduction,
                TransportMedium::Coil,
            ),
            service("b"),
        );

        let assessment = assess_matched_service(&[first, second], &policy()).unwrap();
        assert_eq!(
            assessment.status(),
            MatchedServiceComparisonStatus::Unresolved
        );
        assert!(assessment.reasons().iter().any(|reason| matches!(
            reason,
            MatchedServiceReason::EvidenceNotAdmissible {
                candidate_id,
                primitive_id,
                evidence_class: ModuleEvidenceClass::ManufacturerDeclared
            } if candidate_id == "a" && primitive_id == "wire"
        )));
    }

    #[test]
    fn no_global_efficiency_score_is_emitted() {
        let first = candidate(
            "a",
            primitive(
                "wire",
                "source",
                "load",
                EnergyCarrier::Electrical,
                TransportMechanism::Conduction,
                TransportMedium::Conductor,
            ),
            service("a"),
        );
        let second = candidate(
            "b",
            primitive(
                "ipt",
                "source",
                "load",
                EnergyCarrier::ElectromagneticField,
                TransportMechanism::MagneticInduction,
                TransportMedium::Coil,
            ),
            service("b"),
        );

        let assessment = assess_matched_service(&[first, second], &policy()).unwrap();
        assert_eq!(
            assessment.status(),
            MatchedServiceComparisonStatus::Comparable
        );
    }

    #[test]
    fn comparison_requires_two_unique_candidates() {
        let only = candidate(
            "only",
            primitive(
                "wire",
                "source",
                "load",
                EnergyCarrier::Electrical,
                TransportMechanism::Conduction,
                TransportMedium::Conductor,
            ),
            service("only"),
        );

        assert!(matches!(
            assess_matched_service(std::slice::from_ref(&only), &policy()),
            Err(TransportContractError::InsufficientCandidates { count: 1 })
        ));
        assert!(matches!(
            assess_matched_service(&[only.clone(), only], &policy()),
            Err(TransportContractError::DuplicateCandidateId { .. })
        ));
    }

    #[test]
    fn service_contract_rejects_blank_known_digest() {
        let result = UsefulServiceContract::new(UsefulServiceSpec {
            service_id: "blank".to_owned(),
            kind: UsefulServiceKind::ElectricalLoad,
            quality_profile_digest: Some("quality".to_owned()),
            duty_cycle_digest: Some("  ".to_owned()),
            scenario_digest: Some("scenario".to_owned()),
            location_environment_digest: Some("environment".to_owned()),
            source_boundary_profile_digest: Some("source-boundary".to_owned()),
            useful_service_boundary_profile_digest: Some("load-boundary".to_owned()),
        });

        assert!(matches!(
            result,
            Err(TransportContractError::MissingField {
                field: "duty_cycle_digest"
            })
        ));
    }

    #[test]
    fn conversion_and_storage_metadata_remain_orthogonal_to_transport() {
        let primitive = TransportPrimitive::new(TransportPrimitiveSpec {
            primitive_id: "stored-ipt".to_owned(),
            source_port_id: "tx".to_owned(),
            destination_port_id: "rx".to_owned(),
            carrier: EnergyCarrier::ElectromagneticField,
            mechanism: TransportMechanism::MagneticInduction,
            medium: TransportMedium::Coil,
            conversion_stages: vec![ConversionStage::Rectifier],
            storage_state: StorageState::Battery,
            operating_regime_profile_digest: "ipt-profile".to_owned(),
            evidence: verified_evidence("ipt-evidence"),
        })
        .unwrap();

        assert_eq!(primitive.conversion_stages(), &[ConversionStage::Rectifier]);
        assert_eq!(primitive.storage_state(), StorageState::Battery);
        assert_eq!(primitive.mechanism(), TransportMechanism::MagneticInduction);
    }

    #[test]
    fn all_admitted_transport_families_have_explicit_physical_pairs() {
        let cases = [
            (
                EnergyCarrier::Electrical,
                TransportMechanism::Conduction,
                TransportMedium::Conductor,
            ),
            (
                EnergyCarrier::Electrical,
                TransportMechanism::SuperconductingConduction,
                TransportMedium::SuperconductingCable,
            ),
            (
                EnergyCarrier::ElectromagneticField,
                TransportMechanism::MagneticInduction,
                TransportMedium::Coil,
            ),
            (
                EnergyCarrier::ElectromagneticField,
                TransportMechanism::CapacitiveCoupling,
                TransportMedium::Electrodes,
            ),
            (
                EnergyCarrier::ElectromagneticField,
                TransportMechanism::RadiativePropagation,
                TransportMedium::FreeSpace,
            ),
            (
                EnergyCarrier::ElectromagneticField,
                TransportMechanism::GuidedElectromagnetic,
                TransportMedium::Waveguide,
            ),
            (
                EnergyCarrier::Thermal,
                TransportMechanism::ThermalConduction,
                TransportMedium::Solid,
            ),
            (
                EnergyCarrier::Thermal,
                TransportMechanism::ThermalAdvection,
                TransportMedium::Liquid,
            ),
            (
                EnergyCarrier::Thermal,
                TransportMechanism::PhaseChangeCapillary,
                TransportMedium::Pipe,
            ),
            (
                EnergyCarrier::Mechanical,
                TransportMechanism::ShaftDrive,
                TransportMedium::Shaft,
            ),
            (
                EnergyCarrier::Mechanical,
                TransportMechanism::HydraulicFlow,
                TransportMedium::Pipe,
            ),
            (
                EnergyCarrier::Mechanical,
                TransportMechanism::PneumaticFlow,
                TransportMedium::Gas,
            ),
            (
                EnergyCarrier::Acoustic,
                TransportMechanism::AcousticWave,
                TransportMedium::Liquid,
            ),
            (
                EnergyCarrier::Chemical,
                TransportMechanism::EnergyBearingLogistics,
                TransportMedium::Vessel,
            ),
            (
                EnergyCarrier::Electrochemical,
                TransportMechanism::EnergyBearingLogistics,
                TransportMedium::Road,
            ),
            (
                EnergyCarrier::GravitationalPotential,
                TransportMechanism::EnergyBearingLogistics,
                TransportMedium::Rail,
            ),
        ];

        for (carrier, mechanism, medium) in cases {
            assert!(
                valid_carrier_mechanism_medium(carrier, mechanism, medium),
                "expected admitted combination {carrier:?}/{mechanism:?}/{medium:?}"
            );
        }
    }
}
