// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! EV-CARRIER-ICD-001 exact-revision interface compatibility screen.
//! A compatible result requires independent, scoped qualification evidence on
//! both sides of every required interface. This module never grants authority.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const ICD_SCHEMA_VERSION: u16 = 1;
pub const MAX_MODULES: usize = 32;
pub const MAX_PORTS_PER_CONFIGURATION: usize = 64;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum InterfaceFamily {
    MechanicalStructure,
    HighVoltageDc,
    LowVoltageDc,
    DataControl,
    ThermalFluid,
    CargoRetention,
    UavDocking,
    AmrDocking,
    Environmental,
    Service,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PortDirection {
    Provider,
    Consumer,
    Bidirectional,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceClass {
    IndependentTest,
    Measured,
    Synthetic,
    Planned,
    Unknown,
    Stale,
    Revoked,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CompatibilityStatus {
    Compatible,
    Incompatible,
    Indeterminate,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct QuantityRange {
    pub unit: String,
    pub minimum: f64,
    pub maximum: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Quantity {
    pub unit: String,
    pub value: f64,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum QualificationTestResult {
    Passed,
    Failed,
    Inconclusive,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct QualificationEvidence {
    pub evidence_id: String,
    pub evidence_class: EvidenceClass,
    pub source_reference: String,
    pub issuer_id: String,
    pub report_digest_sha256: String,
    pub measurement_dataset_sha256: String,
    pub calibration_bundle_sha256: String,
    pub tested_vehicle_configuration_id: String,
    pub test_article_vehicle_serial: String,
    pub tested_module_configuration_id: String,
    pub test_article_module_serial: String,
    pub interface_revision: String,
    pub test_procedure_id: String,
    pub test_procedure_revision: String,
    pub acceptance_criteria_id: String,
    pub uncertainty_budget_reference: String,
    pub test_completed_at_utc: String,
    pub valid_until_utc: Option<String>,
    pub test_result: QualificationTestResult,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct InterfacePort {
    pub port_id: String,
    pub mate_port_id: String,
    pub family: InterfaceFamily,
    pub interface_revision: String,
    /// Exact connector, mount pattern, protocol, fluid, latch or service profile.
    pub profile_id: String,
    pub direction: PortDirection,
    pub polarity_profile: Option<String>,
    /// Provider's operating range must fit inside the consumer's accepted range.
    pub operating_range: Option<QuantityRange>,
    /// Provider's capacity and consumer's required capacity use the same unit.
    pub capacity_limit: Option<Quantity>,
    pub capacity_required: Option<Quantity>,
    pub required: bool,
    pub evidence: Vec<QualificationEvidence>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct VehicleInterfaceContract {
    pub vehicle_configuration_id: String,
    pub vehicle_family_id: String,
    pub vehicle_revision: String,
    pub interface_inventory_complete: bool,
    pub ports: Vec<InterfacePort>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ModuleInterfaceContract {
    pub module_configuration_id: String,
    pub module_family_id: String,
    pub module_revision: String,
    pub target_vehicle_family_id: String,
    pub target_vehicle_revision: String,
    pub interface_inventory_complete: bool,
    pub ports: Vec<InterfacePort>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CompatibilityInput {
    pub schema_version: u16,
    pub benchmark_id: String,
    pub vehicle: VehicleInterfaceContract,
    pub modules: Vec<ModuleInterfaceContract>,
    pub evaluation_time_utc: String,
    pub claim_ceiling: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct InterfaceFinding {
    pub module_port_id: String,
    pub vehicle_port_id: Option<String>,
    pub family: InterfaceFamily,
    pub status: CompatibilityStatus,
    pub findings: Vec<String>,
    pub vehicle_evidence_id: Option<String>,
    pub vehicle_evidence_source_reference: Option<String>,
    pub vehicle_evidence_report_digest_sha256: Option<String>,
    pub vehicle_evidence_receipt: Option<QualificationEvidence>,
    pub module_evidence_receipt: Option<QualificationEvidence>,
    pub module_evidence_id: String,
    pub module_evidence_source_reference: String,
    pub module_evidence_report_digest_sha256: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ModuleCompatibilityResult {
    pub module_configuration_id: String,
    pub module_family_id: String,
    pub status: CompatibilityStatus,
    pub required_interface_count: usize,
    pub compatible_interface_count: usize,
    pub incompatible_interface_count: usize,
    pub indeterminate_interface_count: usize,
    pub findings: Vec<String>,
    pub interfaces: Vec<InterfaceFinding>,
    pub production_release_ready: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CompatibilityReport {
    pub schema_version: u16,
    pub benchmark_id: String,
    pub input_digest_sha256: String,
    pub vehicle_configuration_id: String,
    pub evaluated_at_utc: String,
    pub module_result_count: usize,
    pub results: Vec<ModuleCompatibilityResult>,
    pub physical_authority_granted: bool,
    pub actuators_commanded: bool,
    pub production_release_granted: bool,
    pub evidence_digest_content_verified: bool,
    pub source_signatures_authenticated: bool,
    pub claim_ceiling: String,
}

fn valid_digest(digest: &str) -> bool {
    digest.len() == 64
        && digest
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn valid_utc_timestamp(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 20
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16] != b':'
        || bytes[19] != b'Z'
    {
        return false;
    }
    for index in [0, 1, 2, 3, 5, 6, 8, 9, 11, 12, 14, 15, 17, 18] {
        if !bytes[index].is_ascii_digit() {
            return false;
        }
    }
    let parse = |start: usize, end: usize| {
        value
            .get(start..end)
            .and_then(|part| part.parse::<u32>().ok())
    };
    let Some(month) = parse(5, 7) else {
        return false;
    };
    let Some(day) = parse(8, 10) else {
        return false;
    };
    let Some(hour) = parse(11, 13) else {
        return false;
    };
    let Some(minute) = parse(14, 16) else {
        return false;
    };
    let Some(second) = parse(17, 19) else {
        return false;
    };
    (1..=12).contains(&month)
        && (1..=31).contains(&day)
        && hour <= 23
        && minute <= 59
        && second <= 60
}

fn valid_optional_quantity(quantity: &Option<Quantity>) -> bool {
    quantity.as_ref().is_none_or(|value| {
        !value.unit.trim().is_empty() && value.value.is_finite() && value.value >= 0.0
    })
}

fn valid_optional_range(range: &Option<QuantityRange>) -> bool {
    range.as_ref().is_none_or(|value| {
        !value.unit.trim().is_empty()
            && value.minimum.is_finite()
            && value.maximum.is_finite()
            && value.minimum <= value.maximum
    })
}

fn validate_ports(
    label: &str,
    ports: &[InterfacePort],
    evaluation_time_utc: &str,
    errors: &mut Vec<String>,
) {
    if ports.len() > MAX_PORTS_PER_CONFIGURATION {
        errors.push(format!(
            "{label}.ports: exceeds {MAX_PORTS_PER_CONFIGURATION} ports"
        ));
    }
    let mut ids = BTreeSet::new();
    for (index, port) in ports.iter().enumerate() {
        if port.port_id.trim().is_empty()
            || port.mate_port_id.trim().is_empty()
            || !ids.insert(port.port_id.as_str())
        {
            errors.push(format!(
                "{label}.ports[{index}]: port and mate identities must be nonempty; port IDs unique"
            ));
        }
        if port.interface_revision.trim().is_empty() || port.profile_id.trim().is_empty() {
            errors.push(format!(
                "{label}.ports[{index}]: interface revision and profile ID are required"
            ));
        }
        if port
            .polarity_profile
            .as_ref()
            .is_some_and(|profile| profile.trim().is_empty())
        {
            errors.push(format!(
                "{label}.ports[{index}].polarity_profile: blank values are not permitted"
            ));
        }
        if !valid_optional_range(&port.operating_range) {
            errors.push(format!(
                "{label}.ports[{index}].operating_range: invalid finite unit range"
            ));
        }
        if !valid_optional_quantity(&port.capacity_limit)
            || !valid_optional_quantity(&port.capacity_required)
        {
            errors.push(format!(
                "{label}.ports[{index}]: capacity quantities must be finite and nonnegative"
            ));
        }
        if port.evidence.is_empty() {
            errors.push(format!(
                "{label}.ports[{index}].evidence: at least one scoped evidence receipt is required"
            ));
        }
        let mut evidence_ids = BTreeSet::new();
        for (evidence_index, evidence) in port.evidence.iter().enumerate() {
            if evidence.evidence_id.trim().is_empty()
                || !evidence_ids.insert(evidence.evidence_id.as_str())
                || evidence.source_reference.trim().is_empty()
                || evidence.issuer_id.trim().is_empty()
                || evidence.tested_vehicle_configuration_id.trim().is_empty()
                || evidence.test_article_vehicle_serial.trim().is_empty()
                || evidence.tested_module_configuration_id.trim().is_empty()
                || evidence.test_article_module_serial.trim().is_empty()
                || evidence.interface_revision.trim().is_empty()
                || evidence.test_procedure_id.trim().is_empty()
                || evidence.test_procedure_revision.trim().is_empty()
                || evidence.acceptance_criteria_id.trim().is_empty()
                || evidence.uncertainty_budget_reference.trim().is_empty()
                || !valid_utc_timestamp(&evidence.test_completed_at_utc)
                || evidence.test_completed_at_utc.as_str() > evaluation_time_utc
                || evidence.valid_until_utc.as_ref().is_some_and(|expiry| {
                    !valid_utc_timestamp(expiry)
                        || expiry.as_str() < evidence.test_completed_at_utc.as_str()
                })
                || (evidence.evidence_class == EvidenceClass::IndependentTest
                    && evidence.valid_until_utc.is_none())
                || evidence
                    .limitations
                    .iter()
                    .any(|limitation| limitation.trim().is_empty())
                || !valid_digest(&evidence.report_digest_sha256)
                || !valid_digest(&evidence.measurement_dataset_sha256)
                || !valid_digest(&evidence.calibration_bundle_sha256)
            {
                errors.push(format!(
                    "{label}.ports[{index}].evidence[{evidence_index}]: complete source, issuer, exact test-article scope, procedure, acceptance, uncertainty, time and lowercase SHA-256 digests are required"
                ));
            }
        }
    }
}

pub fn validate_input(input: &CompatibilityInput) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    if input.schema_version != ICD_SCHEMA_VERSION {
        errors.push(format!(
            "schema_version: expected {ICD_SCHEMA_VERSION}, got {}",
            input.schema_version
        ));
    }
    if input.benchmark_id != "EV-CARRIER-ICD-001" {
        errors.push("benchmark_id: expected EV-CARRIER-ICD-001".into());
    }
    if input.claim_ceiling.trim().is_empty() {
        errors.push("claim_ceiling: must be nonempty".into());
    }
    if !valid_utc_timestamp(&input.evaluation_time_utc) {
        errors.push("evaluation_time_utc: expected normalized YYYY-MM-DDTHH:MM:SSZ".into());
    }
    if input.vehicle.vehicle_configuration_id.trim().is_empty()
        || input.vehicle.vehicle_family_id.trim().is_empty()
        || input.vehicle.vehicle_revision.trim().is_empty()
    {
        errors.push("vehicle: configuration, family and revision IDs are required".into());
    }
    if input.modules.is_empty() || input.modules.len() > MAX_MODULES {
        errors.push(format!(
            "modules: expected 1..={MAX_MODULES} configurations"
        ));
    }
    validate_ports(
        "vehicle",
        &input.vehicle.ports,
        &input.evaluation_time_utc,
        &mut errors,
    );
    let mut vehicle_mate_ids = BTreeSet::new();
    for port in &input.vehicle.ports {
        if !vehicle_mate_ids.insert(port.mate_port_id.as_str()) {
            errors.push(format!(
                "vehicle.ports: multiple vehicle ports claim module mate ID {}",
                port.mate_port_id
            ));
        }
    }
    let mut module_ids = BTreeSet::new();
    for (index, module) in input.modules.iter().enumerate() {
        if module.module_configuration_id.trim().is_empty()
            || module.module_family_id.trim().is_empty()
            || module.module_revision.trim().is_empty()
            || module.target_vehicle_family_id.trim().is_empty()
            || module.target_vehicle_revision.trim().is_empty()
            || !module_ids.insert(module.module_configuration_id.as_str())
        {
            errors.push(format!(
                "modules[{index}]: exact unique module and target vehicle identities are required"
            ));
        }
        validate_ports(
            &format!("modules[{index}]"),
            &module.ports,
            &input.evaluation_time_utc,
            &mut errors,
        );
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn qualified_evidence<'a>(
    evidence: &'a [QualificationEvidence],
    vehicle: &VehicleInterfaceContract,
    module: &ModuleInterfaceContract,
    interface_revision: &str,
    evaluation_time_utc: &str,
) -> Option<&'a QualificationEvidence> {
    evidence.iter().find(|receipt| {
        receipt.evidence_class == EvidenceClass::IndependentTest
            && receipt.test_result == QualificationTestResult::Passed
            && receipt.tested_vehicle_configuration_id == vehicle.vehicle_configuration_id
            && receipt.tested_module_configuration_id == module.module_configuration_id
            && receipt.interface_revision == interface_revision
            && receipt
                .valid_until_utc
                .as_deref()
                .is_some_and(|valid_until| valid_until > evaluation_time_utc)
            && valid_digest(&receipt.report_digest_sha256)
            && valid_digest(&receipt.measurement_dataset_sha256)
            && valid_digest(&receipt.calibration_bundle_sha256)
    })
}

fn failed_exact_evidence(
    evidence: &[QualificationEvidence],
    vehicle: &VehicleInterfaceContract,
    module: &ModuleInterfaceContract,
    interface_revision: &str,
) -> bool {
    evidence.iter().any(|receipt| {
        receipt.evidence_class == EvidenceClass::IndependentTest
            && receipt.test_result == QualificationTestResult::Failed
            && receipt.tested_vehicle_configuration_id == vehicle.vehicle_configuration_id
            && receipt.tested_module_configuration_id == module.module_configuration_id
            && receipt.interface_revision == interface_revision
    })
}

fn expired_exact_evidence(
    evidence: &[QualificationEvidence],
    vehicle: &VehicleInterfaceContract,
    module: &ModuleInterfaceContract,
    interface_revision: &str,
    evaluation_time_utc: &str,
) -> bool {
    evidence.iter().any(|receipt| {
        receipt.evidence_class == EvidenceClass::IndependentTest
            && receipt.tested_vehicle_configuration_id == vehicle.vehicle_configuration_id
            && receipt.tested_module_configuration_id == module.module_configuration_id
            && receipt.interface_revision == interface_revision
            && receipt
                .valid_until_utc
                .as_deref()
                .is_some_and(|valid_until| valid_until <= evaluation_time_utc)
    })
}

fn first_evidence_id(evidence: &[QualificationEvidence]) -> Option<String> {
    evidence.first().map(|receipt| receipt.evidence_id.clone())
}

fn directions_mate(vehicle: PortDirection, module: PortDirection) -> bool {
    matches!(
        (vehicle, module),
        (PortDirection::Provider, PortDirection::Consumer)
            | (PortDirection::Consumer, PortDirection::Provider)
            | (PortDirection::Bidirectional, PortDirection::Bidirectional)
    )
}

fn find_vehicle_port<'a>(
    vehicle: &'a VehicleInterfaceContract,
    port_id: &str,
) -> Option<&'a InterfacePort> {
    vehicle.ports.iter().find(|port| port.port_id == port_id)
}

fn add_interface_finding(
    findings: &mut Vec<String>,
    status: &mut CompatibilityStatus,
    result: CompatibilityStatus,
    finding: &str,
) {
    findings.push(finding.to_string());
    if result == CompatibilityStatus::Incompatible || *status != CompatibilityStatus::Incompatible {
        *status = result;
    }
}

fn evaluate_interface(
    vehicle: &VehicleInterfaceContract,
    module: &ModuleInterfaceContract,
    module_port: &InterfacePort,
    evaluation_time_utc: &str,
) -> InterfaceFinding {
    let vehicle_port = find_vehicle_port(vehicle, &module_port.mate_port_id);
    let mut findings = Vec::new();
    let mut status = CompatibilityStatus::Compatible;

    let Some(vehicle_port) = vehicle_port else {
        let missing_status = if vehicle.interface_inventory_complete {
            CompatibilityStatus::Incompatible
        } else {
            CompatibilityStatus::Indeterminate
        };
        findings.push(if vehicle.interface_inventory_complete {
            "required_vehicle_port_not_declared".into()
        } else {
            "vehicle_interface_inventory_incomplete".into()
        });
        return InterfaceFinding {
            module_port_id: module_port.port_id.clone(),
            vehicle_port_id: None,
            family: module_port.family,
            status: missing_status,
            findings,
            vehicle_evidence_id: None,
            vehicle_evidence_source_reference: None,
            vehicle_evidence_report_digest_sha256: None,
            vehicle_evidence_receipt: None,
            module_evidence_receipt: module_port.evidence.first().cloned(),
            module_evidence_id: first_evidence_id(&module_port.evidence).unwrap_or_default(),
            module_evidence_source_reference: module_port
                .evidence
                .first()
                .map(|receipt| receipt.source_reference.clone())
                .unwrap_or_default(),
            module_evidence_report_digest_sha256: module_port
                .evidence
                .first()
                .map(|receipt| receipt.report_digest_sha256.clone())
                .unwrap_or_default(),
        };
    };

    if vehicle_port.mate_port_id != module_port.port_id {
        add_interface_finding(
            &mut findings,
            &mut status,
            CompatibilityStatus::Incompatible,
            "mate_port_identity_not_reciprocal",
        );
    }
    if vehicle_port.family != module_port.family {
        add_interface_finding(
            &mut findings,
            &mut status,
            CompatibilityStatus::Incompatible,
            "interface_family_mismatch",
        );
    }
    if vehicle_port.interface_revision != module_port.interface_revision {
        add_interface_finding(
            &mut findings,
            &mut status,
            CompatibilityStatus::Incompatible,
            "interface_revision_mismatch",
        );
    }
    if vehicle_port.profile_id != module_port.profile_id {
        add_interface_finding(
            &mut findings,
            &mut status,
            CompatibilityStatus::Incompatible,
            "interface_profile_mismatch",
        );
    }
    if !directions_mate(vehicle_port.direction, module_port.direction) {
        add_interface_finding(
            &mut findings,
            &mut status,
            CompatibilityStatus::Incompatible,
            "port_direction_mismatch",
        );
    }

    match (
        &vehicle_port.polarity_profile,
        &module_port.polarity_profile,
    ) {
        (Some(left), Some(right)) if left != right => {
            add_interface_finding(
                &mut findings,
                &mut status,
                CompatibilityStatus::Incompatible,
                "polarity_profile_mismatch",
            );
        }
        (None, None) => {}
        (Some(_), None) | (None, Some(_))
            if matches!(
                module_port.family,
                InterfaceFamily::HighVoltageDc | InterfaceFamily::LowVoltageDc
            ) =>
        {
            add_interface_finding(
                &mut findings,
                &mut status,
                CompatibilityStatus::Indeterminate,
                "required_polarity_profile_missing",
            );
        }
        _ => {}
    }

    match (&vehicle_port.operating_range, &module_port.operating_range) {
        (Some(provider), Some(consumer)) => {
            if provider.unit != consumer.unit {
                add_interface_finding(
                    &mut findings,
                    &mut status,
                    CompatibilityStatus::Incompatible,
                    "operating_range_unit_mismatch",
                );
            } else if provider.minimum < consumer.minimum || provider.maximum > consumer.maximum {
                add_interface_finding(
                    &mut findings,
                    &mut status,
                    CompatibilityStatus::Incompatible,
                    "provider_operating_range_outside_consumer_acceptance",
                );
            }
        }
        (None, None) => {
            if matches!(
                module_port.family,
                InterfaceFamily::HighVoltageDc
                    | InterfaceFamily::LowVoltageDc
                    | InterfaceFamily::ThermalFluid
            ) {
                add_interface_finding(
                    &mut findings,
                    &mut status,
                    CompatibilityStatus::Indeterminate,
                    "required_operating_range_missing",
                );
            }
        }
        _ => add_interface_finding(
            &mut findings,
            &mut status,
            CompatibilityStatus::Indeterminate,
            "operating_range_incomplete",
        ),
    }

    match (&vehicle_port.capacity_limit, &module_port.capacity_required) {
        (Some(limit), Some(required)) => {
            if limit.unit != required.unit {
                add_interface_finding(
                    &mut findings,
                    &mut status,
                    CompatibilityStatus::Incompatible,
                    "capacity_unit_mismatch",
                );
            } else if limit.value + f64::EPSILON < required.value {
                add_interface_finding(
                    &mut findings,
                    &mut status,
                    CompatibilityStatus::Incompatible,
                    "provider_capacity_below_module_requirement",
                );
            }
        }
        (None, None) => {
            if matches!(
                module_port.family,
                InterfaceFamily::MechanicalStructure
                    | InterfaceFamily::HighVoltageDc
                    | InterfaceFamily::LowVoltageDc
                    | InterfaceFamily::ThermalFluid
                    | InterfaceFamily::CargoRetention
                    | InterfaceFamily::Service
            ) {
                add_interface_finding(
                    &mut findings,
                    &mut status,
                    CompatibilityStatus::Indeterminate,
                    "required_capacity_limit_or_demand_missing",
                );
            }
        }
        _ => add_interface_finding(
            &mut findings,
            &mut status,
            CompatibilityStatus::Indeterminate,
            "capacity_limit_or_demand_incomplete",
        ),
    }

    if failed_exact_evidence(
        &vehicle_port.evidence,
        vehicle,
        module,
        &vehicle_port.interface_revision,
    ) || failed_exact_evidence(
        &module_port.evidence,
        vehicle,
        module,
        &module_port.interface_revision,
    ) {
        add_interface_finding(
            &mut findings,
            &mut status,
            CompatibilityStatus::Incompatible,
            "exact_configuration_independent_test_failed",
        );
    }
    if expired_exact_evidence(
        &vehicle_port.evidence,
        vehicle,
        module,
        &vehicle_port.interface_revision,
        evaluation_time_utc,
    ) || expired_exact_evidence(
        &module_port.evidence,
        vehicle,
        module,
        &module_port.interface_revision,
        evaluation_time_utc,
    ) {
        add_interface_finding(
            &mut findings,
            &mut status,
            CompatibilityStatus::Indeterminate,
            "exact_configuration_qualification_evidence_expired",
        );
    }
    let vehicle_qualified_evidence = qualified_evidence(
        &vehicle_port.evidence,
        vehicle,
        module,
        &vehicle_port.interface_revision,
        evaluation_time_utc,
    );
    let module_qualified_evidence = qualified_evidence(
        &module_port.evidence,
        vehicle,
        module,
        &module_port.interface_revision,
        evaluation_time_utc,
    );
    if vehicle_qualified_evidence.is_none() || module_qualified_evidence.is_none() {
        add_interface_finding(
            &mut findings,
            &mut status,
            CompatibilityStatus::Indeterminate,
            "independent_exact_configuration_qualification_not_present_on_both_ports",
        );
    }
    let vehicle_receipt = vehicle_qualified_evidence.or_else(|| vehicle_port.evidence.first());
    let module_receipt = module_qualified_evidence.or_else(|| module_port.evidence.first());

    findings.sort();
    findings.dedup();
    InterfaceFinding {
        module_port_id: module_port.port_id.clone(),
        vehicle_port_id: Some(vehicle_port.port_id.clone()),
        family: module_port.family,
        status,
        findings,
        vehicle_evidence_id: vehicle_receipt.map(|receipt| receipt.evidence_id.clone()),
        vehicle_evidence_source_reference: vehicle_receipt
            .map(|receipt| receipt.source_reference.clone()),
        vehicle_evidence_report_digest_sha256: vehicle_receipt
            .map(|receipt| receipt.report_digest_sha256.clone()),
        vehicle_evidence_receipt: vehicle_receipt.cloned(),
        module_evidence_receipt: module_receipt.cloned(),
        module_evidence_id: module_receipt
            .map(|receipt| receipt.evidence_id.clone())
            .unwrap_or_default(),
        module_evidence_source_reference: module_receipt
            .map(|receipt| receipt.source_reference.clone())
            .unwrap_or_default(),
        module_evidence_report_digest_sha256: module_receipt
            .map(|receipt| receipt.report_digest_sha256.clone())
            .unwrap_or_default(),
    }
}

fn canonical_input(input: &CompatibilityInput) -> CompatibilityInput {
    let mut canonical = input.clone();
    canonical
        .vehicle
        .ports
        .sort_by(|left, right| left.port_id.cmp(&right.port_id));
    for port in &mut canonical.vehicle.ports {
        port.evidence
            .sort_by(|left, right| left.evidence_id.cmp(&right.evidence_id));
    }
    canonical.modules.sort_by(|left, right| {
        left.module_configuration_id
            .cmp(&right.module_configuration_id)
    });
    for module in &mut canonical.modules {
        module
            .ports
            .sort_by(|left, right| left.port_id.cmp(&right.port_id));
        for port in &mut module.ports {
            port.evidence
                .sort_by(|left, right| left.evidence_id.cmp(&right.evidence_id));
        }
    }
    canonical
}

fn digest_input(input: &CompatibilityInput) -> Result<String, String> {
    let canonical = canonical_input(input);
    let bytes = serde_json::to_vec(&canonical)
        .map_err(|error| format!("cannot serialize canonical ICD input: {error}"))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

pub fn evaluate_compatibility(
    input: &CompatibilityInput,
) -> Result<CompatibilityReport, Vec<String>> {
    validate_input(input)?;
    let canonical = canonical_input(input);
    let input_digest_sha256 = digest_input(&canonical).map_err(|error| vec![error])?;

    let mut results = Vec::new();
    for module in &canonical.modules {
        let mut interfaces = Vec::new();
        let mut findings = Vec::new();
        if module.target_vehicle_family_id != canonical.vehicle.vehicle_family_id {
            findings.push("target_vehicle_family_mismatch".into());
        }
        if module.target_vehicle_revision != canonical.vehicle.vehicle_revision {
            findings.push("target_vehicle_revision_mismatch".into());
        }
        if !module.interface_inventory_complete {
            findings.push("module_interface_inventory_incomplete".into());
        }

        let mut seen_mate_ports = BTreeMap::<String, String>::new();
        let mut required_ports = 0_usize;
        for port in &module.ports {
            if !port.required {
                continue;
            }
            required_ports += 1;
            if let Some(previous) =
                seen_mate_ports.insert(port.mate_port_id.clone(), port.port_id.clone())
            {
                let mut duplicate = InterfaceFinding {
                    module_port_id: port.port_id.clone(),
                    vehicle_port_id: Some(port.mate_port_id.clone()),
                    family: port.family,
                    status: CompatibilityStatus::Incompatible,
                    findings: vec![
                        "multiple_module_ports_claim_same_vehicle_port".into(),
                        format!("conflicts_with_module_port_{previous}"),
                    ],
                    vehicle_evidence_id: find_vehicle_port(&canonical.vehicle, &port.mate_port_id)
                        .and_then(|vehicle_port| first_evidence_id(&vehicle_port.evidence)),
                    vehicle_evidence_source_reference: find_vehicle_port(
                        &canonical.vehicle,
                        &port.mate_port_id,
                    )
                    .and_then(|vehicle_port| vehicle_port.evidence.first())
                    .map(|receipt| receipt.source_reference.clone()),
                    vehicle_evidence_report_digest_sha256: find_vehicle_port(
                        &canonical.vehicle,
                        &port.mate_port_id,
                    )
                    .and_then(|vehicle_port| vehicle_port.evidence.first())
                    .map(|receipt| receipt.report_digest_sha256.clone()),
                    vehicle_evidence_receipt: find_vehicle_port(
                        &canonical.vehicle,
                        &port.mate_port_id,
                    )
                    .and_then(|vehicle_port| vehicle_port.evidence.first())
                    .cloned(),
                    module_evidence_receipt: port.evidence.first().cloned(),
                    module_evidence_id: first_evidence_id(&port.evidence).unwrap_or_default(),
                    module_evidence_source_reference: port
                        .evidence
                        .first()
                        .map(|receipt| receipt.source_reference.clone())
                        .unwrap_or_default(),
                    module_evidence_report_digest_sha256: port
                        .evidence
                        .first()
                        .map(|receipt| receipt.report_digest_sha256.clone())
                        .unwrap_or_default(),
                };
                duplicate.findings.sort();
                interfaces.push(duplicate);
                continue;
            }
            interfaces.push(evaluate_interface(
                &canonical.vehicle,
                module,
                port,
                &canonical.evaluation_time_utc,
            ));
        }

        let target_vehicle_mismatch = module.target_vehicle_family_id
            != canonical.vehicle.vehicle_family_id
            || module.target_vehicle_revision != canonical.vehicle.vehicle_revision;
        if target_vehicle_mismatch {
            for finding in &mut interfaces {
                if finding.status == CompatibilityStatus::Compatible {
                    finding.status = CompatibilityStatus::Incompatible;
                    finding
                        .findings
                        .push("target_vehicle_configuration_mismatch".into());
                }
            }
        }

        interfaces.sort_by(|left, right| left.module_port_id.cmp(&right.module_port_id));
        let compatible = interfaces
            .iter()
            .filter(|item| item.status == CompatibilityStatus::Compatible)
            .count();
        let incompatible = interfaces
            .iter()
            .filter(|item| item.status == CompatibilityStatus::Incompatible)
            .count();
        let indeterminate = interfaces
            .iter()
            .filter(|item| item.status == CompatibilityStatus::Indeterminate)
            .count();

        let status = if target_vehicle_mismatch || incompatible > 0 {
            CompatibilityStatus::Incompatible
        } else if indeterminate > 0
            || !module.interface_inventory_complete
            || !canonical.vehicle.interface_inventory_complete
            || required_ports == 0
        {
            CompatibilityStatus::Indeterminate
        } else if compatible == required_ports {
            CompatibilityStatus::Compatible
        } else {
            CompatibilityStatus::Indeterminate
        };

        if !canonical.vehicle.interface_inventory_complete {
            findings.push("vehicle_interface_inventory_incomplete".into());
        }
        if required_ports == 0 {
            findings.push("module_declares_no_required_interfaces".into());
        }
        if status == CompatibilityStatus::Incompatible {
            findings.push("one_or_more_declared_interfaces_are_incompatible".into());
        }
        if status == CompatibilityStatus::Indeterminate {
            findings.push("compatibility_not_fully_qualified".into());
        }
        findings.sort();
        findings.dedup();
        results.push(ModuleCompatibilityResult {
            module_configuration_id: module.module_configuration_id.clone(),
            module_family_id: module.module_family_id.clone(),
            status,
            required_interface_count: required_ports,
            compatible_interface_count: compatible,
            incompatible_interface_count: incompatible,
            indeterminate_interface_count: indeterminate,
            findings,
            interfaces,
            production_release_ready: false,
        });
    }

    results.sort_by(|left, right| {
        left.module_configuration_id
            .cmp(&right.module_configuration_id)
    });
    Ok(CompatibilityReport {
        schema_version: ICD_SCHEMA_VERSION,
        benchmark_id: "EV-CARRIER-ICD-001".into(),
        input_digest_sha256,
        vehicle_configuration_id: canonical.vehicle.vehicle_configuration_id.clone(),
        evaluated_at_utc: canonical.evaluation_time_utc.clone(),
        module_result_count: results.len(),
        results,
        physical_authority_granted: false,
        actuators_commanded: false,
        production_release_granted: false,
        evidence_digest_content_verified: false,
        source_signatures_authenticated: false,
        claim_ceiling: "Read-only exact-revision interface compatibility screen. A compatible result requires independent test evidence bound to both exact configurations and interface revisions. It is not a vehicle/module release, regulatory approval, physical actuation or operational authority.".into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn evidence(
        class: EvidenceClass,
        vehicle_id: &str,
        module_id: &str,
        revision: &str,
    ) -> QualificationEvidence {
        QualificationEvidence {
            evidence_id: format!("EV-{vehicle_id}-{module_id}-{revision}"),
            evidence_class: class,
            source_reference: "synthetic://icd/test-record".into(),
            issuer_id: "synthetic://icd-test-issuer".into(),
            report_digest_sha256: "a".repeat(64),
            measurement_dataset_sha256: "b".repeat(64),
            calibration_bundle_sha256: "c".repeat(64),
            tested_vehicle_configuration_id: vehicle_id.into(),
            test_article_vehicle_serial: format!("TEST-VEH-{vehicle_id}"),
            tested_module_configuration_id: module_id.into(),
            test_article_module_serial: format!("TEST-MOD-{module_id}"),
            interface_revision: revision.into(),
            test_procedure_id: "ICD-TP-001".into(),
            test_procedure_revision: "REV-A".into(),
            acceptance_criteria_id: "ICD-AC-001".into(),
            uncertainty_budget_reference: "synthetic://icd/uncertainty-budget".into(),
            test_completed_at_utc: "2026-11-21T06:00:00Z".into(),
            valid_until_utc: Some("2027-11-21T06:00:00Z".into()),
            test_result: QualificationTestResult::Passed,
            limitations: vec!["test-only in-memory receipt".into()],
        }
    }

    fn paired_port(
        port_id: &str,
        mate_port_id: &str,
        family: InterfaceFamily,
        profile_id: &str,
        direction: PortDirection,
        revision: &str,
        mut evidence: QualificationEvidence,
    ) -> InterfacePort {
        evidence.evidence_id = format!("{port_id}-{}", evidence.evidence_id);
        InterfacePort {
            port_id: port_id.into(),
            mate_port_id: mate_port_id.into(),
            family,
            interface_revision: revision.into(),
            profile_id: profile_id.into(),
            direction,
            polarity_profile: None,
            operating_range: None,
            capacity_limit: None,
            capacity_required: None,
            required: true,
            evidence: vec![evidence],
        }
    }

    fn qualified_input() -> CompatibilityInput {
        let vehicle_id = "carrier-ref-01-r1";
        let module_id = "detachable-pod-r1";
        let revision = "ICD-001-0.1";
        let vehicle_ports = vec![
            InterfacePort {
                operating_range: Some(QuantityRange {
                    unit: "V_DC".into(),
                    minimum: 350.0,
                    maximum: 450.0,
                }),
                capacity_limit: Some(Quantity {
                    unit: "kW".into(),
                    value: 120.0,
                }),
                polarity_profile: Some("DC_POS_NEG".into()),
                ..paired_port(
                    "vehicle-hv",
                    "module-hv",
                    InterfaceFamily::HighVoltageDc,
                    "HV-400V-DC-KEY-A",
                    PortDirection::Provider,
                    revision,
                    evidence(
                        EvidenceClass::IndependentTest,
                        vehicle_id,
                        module_id,
                        revision,
                    ),
                )
            },
            InterfacePort {
                capacity_limit: Some(Quantity {
                    unit: "kg".into(),
                    value: 1_200.0,
                }),
                ..paired_port(
                    "vehicle-mount",
                    "module-mount",
                    InterfaceFamily::MechanicalStructure,
                    "MOUNT-DATUM-A",
                    PortDirection::Provider,
                    revision,
                    evidence(
                        EvidenceClass::IndependentTest,
                        vehicle_id,
                        module_id,
                        revision,
                    ),
                )
            },
            InterfacePort {
                ..paired_port(
                    "vehicle-data",
                    "module-data",
                    InterfaceFamily::DataControl,
                    "CAN-FD-ICD-001",
                    PortDirection::Bidirectional,
                    revision,
                    evidence(
                        EvidenceClass::IndependentTest,
                        vehicle_id,
                        module_id,
                        revision,
                    ),
                )
            },
        ];
        let module_ports = vec![
            InterfacePort {
                operating_range: Some(QuantityRange {
                    unit: "V_DC".into(),
                    minimum: 300.0,
                    maximum: 500.0,
                }),
                capacity_required: Some(Quantity {
                    unit: "kW".into(),
                    value: 60.0,
                }),
                polarity_profile: Some("DC_POS_NEG".into()),
                ..paired_port(
                    "module-hv",
                    "vehicle-hv",
                    InterfaceFamily::HighVoltageDc,
                    "HV-400V-DC-KEY-A",
                    PortDirection::Consumer,
                    revision,
                    evidence(
                        EvidenceClass::IndependentTest,
                        vehicle_id,
                        module_id,
                        revision,
                    ),
                )
            },
            InterfacePort {
                capacity_required: Some(Quantity {
                    unit: "kg".into(),
                    value: 700.0,
                }),
                ..paired_port(
                    "module-mount",
                    "vehicle-mount",
                    InterfaceFamily::MechanicalStructure,
                    "MOUNT-DATUM-A",
                    PortDirection::Consumer,
                    revision,
                    evidence(
                        EvidenceClass::IndependentTest,
                        vehicle_id,
                        module_id,
                        revision,
                    ),
                )
            },
            InterfacePort {
                ..paired_port(
                    "module-data",
                    "vehicle-data",
                    InterfaceFamily::DataControl,
                    "CAN-FD-ICD-001",
                    PortDirection::Bidirectional,
                    revision,
                    evidence(
                        EvidenceClass::IndependentTest,
                        vehicle_id,
                        module_id,
                        revision,
                    ),
                )
            },
        ];
        CompatibilityInput {
            schema_version: 1,
            benchmark_id: "EV-CARRIER-ICD-001".into(),
            vehicle: VehicleInterfaceContract {
                vehicle_configuration_id: vehicle_id.into(),
                vehicle_family_id: "EV-CARRIER-REF-01".into(),
                vehicle_revision: "R1".into(),
                interface_inventory_complete: true,
                ports: vehicle_ports,
            },
            evaluation_time_utc: "2026-11-21T06:30:00Z".into(),
            modules: vec![ModuleInterfaceContract {
                module_configuration_id: module_id.into(),
                module_family_id: "UAV-POD".into(),
                module_revision: "R1".into(),
                target_vehicle_family_id: "EV-CARRIER-REF-01".into(),
                target_vehicle_revision: "R1".into(),
                interface_inventory_complete: true,
                ports: module_ports,
            }],
            claim_ceiling: "Test fixture only".into(),
        }
    }

    #[test]
    fn committed_synthetic_corpus_preserves_indeterminate_and_incompatible_cases() {
        let input: CompatibilityInput = serde_json::from_str(include_str!(
            "../fixtures/ev_carrier_interface_contract_v0_1.json"
        ))
        .expect("ICD fixture must deserialize");
        let report = evaluate_compatibility(&input).expect("fixture must evaluate");
        assert_eq!(report.module_result_count, 4);
        assert_eq!(
            report
                .results
                .iter()
                .find(|result| result.module_configuration_id == "detachable-uav-pod-synthetic")
                .unwrap()
                .status,
            CompatibilityStatus::Indeterminate
        );
        assert_eq!(
            report
                .results
                .iter()
                .find(|result| result.module_configuration_id
                    == "integrated-drone-bay-polarity-negative-control")
                .unwrap()
                .status,
            CompatibilityStatus::Incompatible
        );
        assert_eq!(
            report
                .results
                .iter()
                .find(|result| result.module_configuration_id
                    == "mobile-drone-trailer-revision-negative-control")
                .unwrap()
                .status,
            CompatibilityStatus::Incompatible
        );
        assert_eq!(
            report
                .results
                .iter()
                .find(|result| result.module_configuration_id == "amr-handoff-unqualified")
                .unwrap()
                .status,
            CompatibilityStatus::Indeterminate
        );
        assert!(!report.production_release_granted);
        assert!(!report.physical_authority_granted);
    }

    #[test]
    fn exact_revision_with_independent_evidence_is_compatible() {
        let report = evaluate_compatibility(&qualified_input()).expect("valid input");
        assert_eq!(report.results[0].status, CompatibilityStatus::Compatible);
        assert_eq!(report.results[0].compatible_interface_count, 3);
        assert!(!report.production_release_granted);
        assert!(!report.physical_authority_granted);
    }

    #[test]
    fn compatible_interface_findings_carry_both_exact_evidence_sources() {
        let report = evaluate_compatibility(&qualified_input()).expect("valid input");
        let hv = report.results[0]
            .interfaces
            .iter()
            .find(|interface| interface.module_port_id == "module-hv")
            .expect("HV interface result");
        assert!(
            hv.vehicle_evidence_id
                .as_deref()
                .unwrap()
                .starts_with("vehicle-hv-")
        );
        assert!(hv.module_evidence_id.starts_with("module-hv-"));
        assert!(
            hv.vehicle_evidence_source_reference
                .as_deref()
                .unwrap()
                .starts_with("synthetic://")
        );
        assert!(
            hv.module_evidence_source_reference
                .starts_with("synthetic://")
        );
        let expected_digest = "a".repeat(64);
        assert_eq!(
            hv.vehicle_evidence_report_digest_sha256.as_deref(),
            Some(expected_digest.as_str())
        );
        assert_eq!(hv.module_evidence_report_digest_sha256, expected_digest);
        let vehicle_receipt = hv
            .vehicle_evidence_receipt
            .as_ref()
            .expect("vehicle receipt");
        let module_receipt = hv.module_evidence_receipt.as_ref().expect("module receipt");
        assert_eq!(
            vehicle_receipt.test_article_vehicle_serial,
            "TEST-VEH-carrier-ref-01-r1"
        );
        assert_eq!(
            module_receipt.test_article_module_serial,
            "TEST-MOD-detachable-pod-r1"
        );
        assert_eq!(vehicle_receipt.test_procedure_revision, "REV-A");
        assert_eq!(module_receipt.test_result, QualificationTestResult::Passed);
    }

    #[test]
    fn exact_scope_failed_independent_test_is_incompatible() {
        let mut input = qualified_input();
        input.modules[0].ports[0].evidence[0].test_result = QualificationTestResult::Failed;
        let report = evaluate_compatibility(&input).expect("valid input with failed evidence");
        let hv = report.results[0]
            .interfaces
            .iter()
            .find(|interface| interface.module_port_id == "module-hv")
            .expect("HV result");
        assert_eq!(hv.status, CompatibilityStatus::Incompatible);
        assert!(
            hv.findings
                .contains(&"exact_configuration_independent_test_failed".into())
        );
    }

    #[test]
    fn expired_independent_test_receipt_is_indeterminate() {
        let mut input = qualified_input();
        input.vehicle.ports[0].evidence[0].valid_until_utc = Some("2026-11-21T06:30:00Z".into());
        let report = evaluate_compatibility(&input).expect("valid but expired evidence");
        let hv = report.results[0]
            .interfaces
            .iter()
            .find(|interface| interface.module_port_id == "module-hv")
            .expect("HV result");
        assert_eq!(hv.status, CompatibilityStatus::Indeterminate);
        assert!(
            hv.findings
                .contains(&"exact_configuration_qualification_evidence_expired".into())
        );
    }

    #[test]
    fn missing_expiry_on_independent_test_receipt_is_rejected() {
        let mut input = qualified_input();
        input.modules[0].ports[0].evidence[0].valid_until_utc = None;
        assert!(validate_input(&input).is_err());
    }

    #[test]
    fn inconclusive_independent_test_does_not_qualify_interface() {
        let mut input = qualified_input();
        input.modules[0].ports[0].evidence[0].test_result = QualificationTestResult::Inconclusive;
        let report = evaluate_compatibility(&input).expect("valid inconclusive evidence");
        let hv = report.results[0]
            .interfaces
            .iter()
            .find(|interface| interface.module_port_id == "module-hv")
            .expect("HV result");
        assert_eq!(hv.status, CompatibilityStatus::Indeterminate);
    }

    #[test]
    fn synthetic_evidence_never_produces_compatible_result() {
        let mut input = qualified_input();
        input.modules[0].ports[0].evidence[0].evidence_class = EvidenceClass::Synthetic;
        let report = evaluate_compatibility(&input).expect("valid input");
        assert_eq!(report.results[0].status, CompatibilityStatus::Indeterminate);
    }

    #[test]
    fn known_voltage_polarity_or_profile_mismatch_is_incompatible() {
        let mut input = qualified_input();
        input.modules[0].ports[0].polarity_profile = Some("DC_NEG_POS".into());
        let report = evaluate_compatibility(&input).expect("valid input");
        assert_eq!(report.results[0].status, CompatibilityStatus::Incompatible);
        assert!(
            report.results[0]
                .interfaces
                .iter()
                .find(|interface| interface.module_port_id == "module-hv")
                .expect("HV finding")
                .findings
                .contains(&"polarity_profile_mismatch".into())
        );
    }

    #[test]
    fn module_demand_above_vehicle_capacity_is_incompatible() {
        let mut input = qualified_input();
        input.modules[0].ports[1].capacity_required = Some(Quantity {
            unit: "kg".into(),
            value: 1_300.0,
        });
        let report = evaluate_compatibility(&input).expect("valid input");
        assert_eq!(report.results[0].status, CompatibilityStatus::Incompatible);
    }

    #[test]
    fn provider_operating_range_must_fit_inside_consumer_acceptance() {
        let mut input = qualified_input();
        input.vehicle.ports[0].operating_range = Some(QuantityRange {
            unit: "V_DC".into(),
            minimum: 250.0,
            maximum: 480.0,
        });
        let report = evaluate_compatibility(&input).expect("valid input");
        assert_eq!(report.results[0].status, CompatibilityStatus::Incompatible);
        assert!(
            report.results[0]
                .interfaces
                .iter()
                .find(|interface| interface.module_port_id == "module-hv")
                .expect("HV finding")
                .findings
                .contains(&"provider_operating_range_outside_consumer_acceptance".into())
        );
    }

    #[test]
    fn new_interface_revision_invalidates_old_qualification_receipts() {
        let mut input = qualified_input();
        input.vehicle.ports[0].interface_revision = "ICD-001-0.2".into();
        input.modules[0].ports[0].interface_revision = "ICD-001-0.2".into();
        let report = evaluate_compatibility(&input).expect("valid updated revision");
        assert_eq!(report.results[0].status, CompatibilityStatus::Indeterminate);
        assert!(report.results[0].interfaces.iter().any(|interface| {
            interface.module_port_id == "module-hv"
                && interface.status == CompatibilityStatus::Indeterminate
        }));
    }

    #[test]
    fn ambiguous_vehicle_mating_port_inventory_is_rejected() {
        let mut input = qualified_input();
        let mut duplicate = input.vehicle.ports[1].clone();
        duplicate.port_id = "vehicle-mount-copy".into();
        input.vehicle.ports.push(duplicate);
        let errors = validate_input(&input).expect_err("ambiguous host mate identity");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("multiple vehicle ports claim module mate ID"))
        );
    }

    #[test]
    fn wrong_vehicle_revision_is_incompatible() {
        let mut input = qualified_input();
        input.modules[0].target_vehicle_revision = "R2".into();
        let report = evaluate_compatibility(&input).expect("valid input");
        assert_eq!(report.results[0].status, CompatibilityStatus::Incompatible);
    }

    #[test]
    fn stale_or_wrong_scope_evidence_is_indeterminate() {
        let mut input = qualified_input();
        input.modules[0].ports[0].evidence[0].evidence_class = EvidenceClass::Stale;
        let report = evaluate_compatibility(&input).expect("valid input");
        assert_eq!(report.results[0].status, CompatibilityStatus::Indeterminate);
        let mut wrong_scope = qualified_input();
        wrong_scope.vehicle.ports[0].evidence[0].tested_module_configuration_id =
            "another-module".into();
        let report = evaluate_compatibility(&wrong_scope).expect("valid input");
        assert_eq!(report.results[0].status, CompatibilityStatus::Indeterminate);
    }

    #[test]
    fn missing_declared_vehicle_port_is_incompatible_only_for_complete_inventory() {
        let mut input = qualified_input();
        input
            .vehicle
            .ports
            .retain(|port| port.port_id != "vehicle-hv");
        let report = evaluate_compatibility(&input).expect("valid input");
        assert_eq!(report.results[0].status, CompatibilityStatus::Incompatible);
        let mut incomplete = qualified_input();
        incomplete.vehicle.interface_inventory_complete = false;
        incomplete
            .vehicle
            .ports
            .retain(|port| port.port_id != "vehicle-hv");
        let report = evaluate_compatibility(&incomplete).expect("valid input");
        assert_eq!(report.results[0].status, CompatibilityStatus::Indeterminate);
    }

    #[test]
    fn nonreciprocal_and_duplicate_mate_ports_are_incompatible() {
        let mut input = qualified_input();
        input.modules[0].ports[0].mate_port_id = "missing-port".into();
        let report = evaluate_compatibility(&input).expect("valid input");
        assert_eq!(report.results[0].status, CompatibilityStatus::Incompatible);

        let mut duplicate = qualified_input();
        let mut another = duplicate.modules[0].ports[1].clone();
        another.port_id = "module-mount-copy".into();
        duplicate.modules[0].ports.push(another);
        let report = evaluate_compatibility(&duplicate).expect("valid input");
        assert_eq!(report.results[0].status, CompatibilityStatus::Incompatible);
    }

    #[test]
    fn reordered_ports_and_modules_produce_identical_report_digest() {
        let mut input = qualified_input();
        let first = evaluate_compatibility(&input).expect("first report");
        input.vehicle.ports.reverse();
        input.modules[0].ports.reverse();
        for port in &mut input.vehicle.ports {
            port.evidence.reverse();
        }
        for port in &mut input.modules[0].ports {
            port.evidence.reverse();
        }
        let second = evaluate_compatibility(&input).expect("permuted report");
        assert_eq!(first, second);
    }

    #[test]
    fn malformed_test_timestamp_and_calibration_digest_are_rejected() {
        let mut bad_time = qualified_input();
        bad_time.modules[0].ports[0].evidence[0].test_completed_at_utc = "not-a-time".into();
        assert!(validate_input(&bad_time).is_err());

        let mut bad_calibration = qualified_input();
        bad_calibration.vehicle.ports[0].evidence[0].calibration_bundle_sha256 = "invalid".into();
        assert!(validate_input(&bad_calibration).is_err());
    }

    #[test]
    fn malformed_digests_and_nonfinite_quantities_are_rejected() {
        let mut input = qualified_input();
        input.vehicle.ports[0].evidence[0].report_digest_sha256 = "not-a-digest".into();
        assert!(validate_input(&input).is_err());

        let mut invalid = qualified_input();
        invalid.modules[0].ports[1]
            .capacity_required
            .as_mut()
            .unwrap()
            .value = f64::NAN;
        assert!(validate_input(&invalid).is_err());
    }
}
