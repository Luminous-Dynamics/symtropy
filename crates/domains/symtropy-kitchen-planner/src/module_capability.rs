// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Vendor-neutral descriptive contracts for interoperable kitchen modules.
//!
//! This schema records declared or evidenced capabilities. It does not issue
//! device commands, certify pressure vessels, override appliance-local safety
//! controllers, or establish food-contact compliance by itself.

use crate::action_ontology::ActionGroupV1;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const KITCHEN_MODULE_CAPABILITY_SCHEMA_VERSION: u32 = 1;
pub const KITCHEN_MODULE_CLAIM_CEILING: &str = "Descriptive module capability only; not controller authorization, safety certification, food-contact approval, or physical qualification.";

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ModuleEvidenceClassV1 {
    Declared,
    Modelled,
    Simulated,
    Measured,
    IndependentlyAssayed,
    IndependentlyQualified,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ModuleEvidenceRefV1 {
    pub source_ref: String,
    pub method_ref: Option<String>,
    pub calibration_ref: Option<String>,
    pub uncertainty_ref: Option<String>,
    pub evidence_class: ModuleEvidenceClassV1,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum KitchenModuleKindV1 {
    PrepDrive,
    FoodProcessor,
    Cutter,
    Mixer,
    ThermalVessel,
    PressureCooker,
    InductionHeater,
    Sensor,
    Scale,
    CleaningStation,
    Storage,
    PortioningStation,
    PackagingHandler,
    Other,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ModuleCapabilityClaimStatusV1 {
    Declared,
    Evaluated,
    IndependentlyVerified,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ModuleParameterRangeV1 {
    pub parameter_name: String,
    /// Exact fixed-point lower/upper bounds.
    pub minimum_value: i64,
    pub maximum_value: i64,
    pub decimal_places: u8,
    pub unit: String,
    pub evidence: ModuleEvidenceRefV1,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct KitchenActionCapabilityV1 {
    pub capability_id: String,
    pub action_group: ActionGroupV1,
    pub input_form_refs: Vec<String>,
    pub output_form_refs: Vec<String>,
    /// Unit-bearing quantities are explicit. Do not infer volume from mass.
    pub minimum_batch_mass_g: Option<u64>,
    pub maximum_batch_mass_g: Option<u64>,
    pub nominal_throughput_g_per_min: Option<u64>,
    pub maximum_continuous_run_ms: Option<u64>,
    pub parameter_ranges: Vec<ModuleParameterRangeV1>,
    pub claim_status: ModuleCapabilityClaimStatusV1,
    pub evidence: ModuleEvidenceRefV1,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ModuleElectricalEnvelopeV1 {
    pub nominal_voltage_min_mv: Option<u32>,
    pub nominal_voltage_max_mv: Option<u32>,
    pub mains_frequency_min_millihz: Option<u32>,
    pub mains_frequency_max_millihz: Option<u32>,
    pub continuous_real_power_limit_w: Option<u64>,
    pub peak_real_power_limit_w: Option<u64>,
    pub maximum_surge_duration_ms: Option<u64>,
    pub power_profile_ref: Option<String>,
    pub electrical_evidence: Option<ModuleEvidenceRefV1>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct KitchenModuleInterfacesV1 {
    /// Mechanical mount/adapter record. A shared name does not prove fitment.
    pub mechanical_interface_ref: Option<String>,
    /// Wiring/connector and electrical compatibility record.
    pub electrical_interface_ref: Option<String>,
    /// Data/protocol adapter record; no command API is implied.
    pub data_protocol_ref: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct KitchenModuleServiceRefsV1 {
    pub user_manual_ref: Option<String>,
    pub service_manual_ref: Option<String>,
    pub spare_parts_ref: Option<String>,
    pub repair_procedure_ref: Option<String>,
    pub warranty_ref: Option<String>,
    pub cleaning_sanitation_ref: Option<String>,
    pub food_contact_material_refs: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PressureSafetyQualificationV1 {
    pub pressure_vessel_qualification_ref: String,
    pub pressure_relief_test_ref: String,
    pub independent_test_report_ref: String,
    pub qualification_evidence: ModuleEvidenceRefV1,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct KitchenModuleCapabilityV1 {
    pub schema_version: u32,
    pub module_id: String,
    pub module_revision: String,
    pub module_kind: KitchenModuleKindV1,
    pub manufacturer: String,
    pub model: String,
    pub sku: Option<String>,
    pub market_region: String,
    pub hardware_revision: Option<String>,
    pub firmware_revision: Option<String>,
    pub capabilities: Vec<KitchenActionCapabilityV1>,
    pub electrical: ModuleElectricalEnvelopeV1,
    pub interfaces: KitchenModuleInterfacesV1,
    pub service: KitchenModuleServiceRefsV1,
    /// Reference to the appliance-local safety authority/interlock system.
    /// This field never gives Symtropy control of that authority.
    pub local_safety_authority_ref: String,
    /// Additional qualification references (e.g. pressure vessel/type testing).
    pub qualification_refs: Vec<String>,
    /// Required for pressure-cooker modules. Generic qualification_refs are
    /// not accepted as a substitute for vessel and relief-system evidence.
    pub pressure_safety: Option<PressureSafetyQualificationV1>,
    pub source_evidence: ModuleEvidenceRefV1,
    pub claim_ceiling: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ModuleCapabilityViolationCodeV1 {
    UnsupportedSchemaVersion,
    EmptyRequiredValue,
    InvalidClaimCeiling,
    EmptyCapabilityList,
    DuplicateCapabilityId,
    InvalidBatchRange,
    InvalidParameterRange,
    InvalidParameterScale,
    EmptyEvidenceReference,
    MeasuredEvidenceMissingCalibration,
    QualifiedEvidenceMissingMethod,
    InvalidElectricalRange,
    InvalidPowerEnvelope,
    EmptyPowerProfileReference,
    PressureModuleMissingQualification,
    IncompleteElectricalRange,
    InvalidCapabilityClaimEvidence,
    PressureCapabilityKindMismatch,
    InvalidOperatingRange,
    MissingElectricalEvidence,
    EmptyInterfaceReference,
    DuplicateReference,
    EmptySafetyAuthorityReference,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ModuleCapabilityViolationV1 {
    pub code: ModuleCapabilityViolationCodeV1,
    pub capability_id: Option<String>,
    pub detail: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ModuleCapabilityValidationReportV1 {
    pub module_id: String,
    pub violations: Vec<ModuleCapabilityViolationV1>,
}

impl ModuleCapabilityValidationReportV1 {
    pub fn is_valid(&self) -> bool {
        self.violations.is_empty()
    }
}

fn add_violation(
    violations: &mut Vec<ModuleCapabilityViolationV1>,
    code: ModuleCapabilityViolationCodeV1,
    capability_id: Option<&str>,
    detail: impl Into<String>,
) {
    violations.push(ModuleCapabilityViolationV1 {
        code,
        capability_id: capability_id.map(str::to_owned),
        detail: detail.into(),
    });
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn validate_evidence(
    evidence: &ModuleEvidenceRefV1,
    capability_id: Option<&str>,
    violations: &mut Vec<ModuleCapabilityViolationV1>,
) {
    if !nonempty(&evidence.source_ref) {
        add_violation(
            violations,
            ModuleCapabilityViolationCodeV1::EmptyEvidenceReference,
            capability_id,
            "source_ref must be non-empty",
        );
    }
    if evidence
        .method_ref
        .as_deref()
        .is_some_and(|value| !nonempty(value))
        || evidence
            .calibration_ref
            .as_deref()
            .is_some_and(|value| !nonempty(value))
        || evidence
            .uncertainty_ref
            .as_deref()
            .is_some_and(|value| !nonempty(value))
    {
        add_violation(
            violations,
            ModuleCapabilityViolationCodeV1::EmptyEvidenceReference,
            capability_id,
            "optional evidence references must be non-empty when supplied",
        );
    }
    if evidence.evidence_class == ModuleEvidenceClassV1::Measured
        && evidence
            .calibration_ref
            .as_deref()
            .is_none_or(|value| !nonempty(value))
    {
        add_violation(
            violations,
            ModuleCapabilityViolationCodeV1::MeasuredEvidenceMissingCalibration,
            capability_id,
            "measured evidence requires an explicit calibration reference",
        );
    }
    if matches!(
        evidence.evidence_class,
        ModuleEvidenceClassV1::IndependentlyAssayed | ModuleEvidenceClassV1::IndependentlyQualified
    ) && evidence
        .method_ref
        .as_deref()
        .is_none_or(|value| !nonempty(value))
    {
        add_violation(
            violations,
            ModuleCapabilityViolationCodeV1::QualifiedEvidenceMissingMethod,
            capability_id,
            "independent assay/qualification requires a method or protocol reference",
        );
    }
}

/// Validate module metadata and source evidence. This is schema/evidence
/// validation only; it does not establish physical safety or interoperability.
pub fn validate_kitchen_module_capability(
    module: &KitchenModuleCapabilityV1,
) -> ModuleCapabilityValidationReportV1 {
    let mut violations = Vec::new();
    if module.schema_version != KITCHEN_MODULE_CAPABILITY_SCHEMA_VERSION {
        add_violation(
            &mut violations,
            ModuleCapabilityViolationCodeV1::UnsupportedSchemaVersion,
            None,
            format!(
                "unsupported module capability schema version {}",
                module.schema_version
            ),
        );
    }

    for (name, value) in [
        ("module_id", module.module_id.as_str()),
        ("module_revision", module.module_revision.as_str()),
        ("manufacturer", module.manufacturer.as_str()),
        ("model", module.model.as_str()),
        ("market_region", module.market_region.as_str()),
        (
            "local_safety_authority_ref",
            module.local_safety_authority_ref.as_str(),
        ),
    ] {
        if !nonempty(value) {
            add_violation(
                &mut violations,
                ModuleCapabilityViolationCodeV1::EmptyRequiredValue,
                None,
                format!("{name} must be non-empty"),
            );
        }
    }
    if module.claim_ceiling != KITCHEN_MODULE_CLAIM_CEILING {
        add_violation(
            &mut violations,
            ModuleCapabilityViolationCodeV1::InvalidClaimCeiling,
            None,
            "module claim ceiling must match the fixed descriptive-only statement",
        );
    }
    validate_evidence(&module.source_evidence, None, &mut violations);

    if module.capabilities.is_empty() {
        add_violation(
            &mut violations,
            ModuleCapabilityViolationCodeV1::EmptyCapabilityList,
            None,
            "at least one action capability is required",
        );
    }
    let mut capability_ids = BTreeSet::new();
    for capability in &module.capabilities {
        let capability_id = capability.capability_id.as_str();
        if !nonempty(capability_id) {
            add_violation(
                &mut violations,
                ModuleCapabilityViolationCodeV1::EmptyRequiredValue,
                Some(capability_id),
                "capability_id must be non-empty",
            );
        }
        if !capability_ids.insert(capability_id) {
            add_violation(
                &mut violations,
                ModuleCapabilityViolationCodeV1::DuplicateCapabilityId,
                Some(capability_id),
                "capability_id must be unique within a module revision",
            );
        }
        if capability.minimum_batch_mass_g == Some(0)
            || capability.maximum_batch_mass_g == Some(0)
            || capability.nominal_throughput_g_per_min == Some(0)
            || capability.maximum_continuous_run_ms == Some(0)
        {
            add_violation(
                &mut violations,
                ModuleCapabilityViolationCodeV1::InvalidOperatingRange,
                Some(capability_id),
                "batch, throughput and run-time limits must be positive when supplied",
            );
        }
        if capability.minimum_batch_mass_g.is_some_and(|minimum| {
            capability
                .maximum_batch_mass_g
                .is_some_and(|maximum| minimum > maximum)
        }) {
            add_violation(
                &mut violations,
                ModuleCapabilityViolationCodeV1::InvalidBatchRange,
                Some(capability_id),
                "minimum batch mass must not exceed maximum batch mass",
            );
        }
        if capability.minimum_batch_mass_g.is_some() && capability.maximum_batch_mass_g.is_none() {
            add_violation(
                &mut violations,
                ModuleCapabilityViolationCodeV1::InvalidBatchRange,
                Some(capability_id),
                "a minimum batch mass requires an explicit maximum",
            );
        }
        for parameter in &capability.parameter_ranges {
            if !nonempty(&parameter.parameter_name) || !nonempty(&parameter.unit) {
                add_violation(
                    &mut violations,
                    ModuleCapabilityViolationCodeV1::InvalidParameterRange,
                    Some(capability_id),
                    "parameter name and unit must be non-empty",
                );
            }
            if parameter.minimum_value > parameter.maximum_value {
                add_violation(
                    &mut violations,
                    ModuleCapabilityViolationCodeV1::InvalidParameterRange,
                    Some(capability_id),
                    format!(
                        "parameter {} has minimum greater than maximum",
                        parameter.parameter_name
                    ),
                );
            }
            if parameter.decimal_places > 6 {
                add_violation(
                    &mut violations,
                    ModuleCapabilityViolationCodeV1::InvalidParameterScale,
                    Some(capability_id),
                    format!(
                        "parameter {} exceeds six decimal places",
                        parameter.parameter_name
                    ),
                );
            }
            validate_evidence(&parameter.evidence, Some(capability_id), &mut violations);
        }
        if capability.input_form_refs.is_empty() || capability.output_form_refs.is_empty() {
            add_violation(
                &mut violations,
                ModuleCapabilityViolationCodeV1::EmptyRequiredValue,
                Some(capability_id),
                "each action capability needs at least one input form and output form reference",
            );
        }
        for reference in capability
            .input_form_refs
            .iter()
            .chain(capability.output_form_refs.iter())
        {
            if !nonempty(reference) {
                add_violation(
                    &mut violations,
                    ModuleCapabilityViolationCodeV1::EmptyRequiredValue,
                    Some(capability_id),
                    "input/output form references must be non-empty",
                );
            }
        }
        validate_evidence(&capability.evidence, Some(capability_id), &mut violations);
    }

    let electrical = &module.electrical;
    if electrical.nominal_voltage_min_mv.is_some() != electrical.nominal_voltage_max_mv.is_some()
        || electrical.mains_frequency_min_millihz.is_some()
            != electrical.mains_frequency_max_millihz.is_some()
        || electrical.continuous_real_power_limit_w.is_some()
            != electrical.peak_real_power_limit_w.is_some()
    {
        add_violation(
            &mut violations,
            ModuleCapabilityViolationCodeV1::IncompleteElectricalRange,
            None,
            "voltage, frequency and continuous/peak power limits must be supplied as complete pairs",
        );
    }
    if electrical.nominal_voltage_min_mv.is_some_and(|minimum| {
        electrical
            .nominal_voltage_max_mv
            .is_some_and(|maximum| minimum > maximum)
    }) || electrical
        .mains_frequency_min_millihz
        .is_some_and(|minimum| {
            electrical
                .mains_frequency_max_millihz
                .is_some_and(|maximum| minimum > maximum)
        })
    {
        add_violation(
            &mut violations,
            ModuleCapabilityViolationCodeV1::InvalidElectricalRange,
            None,
            "electrical minimum must not exceed maximum",
        );
    }
    if electrical.maximum_surge_duration_ms.is_some()
        && (electrical.continuous_real_power_limit_w.is_none()
            || electrical.peak_real_power_limit_w.is_none())
    {
        add_violation(
            &mut violations,
            ModuleCapabilityViolationCodeV1::IncompleteElectricalRange,
            None,
            "a surge duration requires continuous and peak real-power limits",
        );
    }
    if electrical
        .continuous_real_power_limit_w
        .is_some_and(|continuous| {
            electrical
                .peak_real_power_limit_w
                .is_some_and(|peak| peak < continuous)
        })
        || electrical.peak_real_power_limit_w.is_some_and(|peak| {
            electrical
                .continuous_real_power_limit_w
                .is_some_and(|continuous| peak > continuous)
                && electrical
                    .maximum_surge_duration_ms
                    .is_none_or(|duration| duration == 0)
        })
    {
        add_violation(
            &mut violations,
            ModuleCapabilityViolationCodeV1::InvalidPowerEnvelope,
            None,
            "peak real-power limit must be at least the continuous real-power limit",
        );
    }
    if electrical
        .power_profile_ref
        .as_deref()
        .is_some_and(|value| !nonempty(value))
    {
        add_violation(
            &mut violations,
            ModuleCapabilityViolationCodeV1::EmptyPowerProfileReference,
            None,
            "power_profile_ref must be non-empty when supplied",
        );
    }
    let has_electrical_ratings = electrical.nominal_voltage_min_mv.is_some()
        || electrical.mains_frequency_min_millihz.is_some()
        || electrical.continuous_real_power_limit_w.is_some()
        || electrical.power_profile_ref.is_some();
    if has_electrical_ratings && electrical.electrical_evidence.is_none() {
        add_violation(
            &mut violations,
            ModuleCapabilityViolationCodeV1::MissingElectricalEvidence,
            None,
            "electrical ratings/profiles require an explicit evidence source",
        );
    }
    if let Some(evidence) = electrical.electrical_evidence.as_ref() {
        validate_evidence(evidence, None, &mut violations);
    }
    if electrical.maximum_surge_duration_ms == Some(0) {
        add_violation(
            &mut violations,
            ModuleCapabilityViolationCodeV1::InvalidPowerEnvelope,
            None,
            "maximum surge duration must be positive when supplied",
        );
    }
    for capability in &module.capabilities {
        let evidence_class = capability.evidence.evidence_class;
        let claim_mismatch = (capability.claim_status == ModuleCapabilityClaimStatusV1::Evaluated
            && !matches!(
                evidence_class,
                ModuleEvidenceClassV1::Measured
                    | ModuleEvidenceClassV1::IndependentlyAssayed
                    | ModuleEvidenceClassV1::IndependentlyQualified
            ))
            || (capability.claim_status == ModuleCapabilityClaimStatusV1::IndependentlyVerified
                && evidence_class != ModuleEvidenceClassV1::IndependentlyQualified);
        if claim_mismatch {
            add_violation(
                &mut violations,
                ModuleCapabilityViolationCodeV1::InvalidCapabilityClaimEvidence,
                Some(&capability.capability_id),
                "capability claim status exceeds its evidence class",
            );
        }
    }
    let declares_pressure_cook = module
        .capabilities
        .iter()
        .any(|capability| capability.action_group == ActionGroupV1::PressureCook);
    if (module.module_kind == KitchenModuleKindV1::PressureCooker) != declares_pressure_cook {
        add_violation(
            &mut violations,
            ModuleCapabilityViolationCodeV1::PressureCapabilityKindMismatch,
            None,
            "PressureCook action capability and pressure-cooker module kind must be declared together",
        );
    }
    if module.module_kind == KitchenModuleKindV1::PressureCooker {
        match module.pressure_safety.as_ref() {
            Some(pressure) => {
                for (name, reference) in [
                    (
                        "pressure_vessel_qualification_ref",
                        pressure.pressure_vessel_qualification_ref.as_str(),
                    ),
                    (
                        "pressure_relief_test_ref",
                        pressure.pressure_relief_test_ref.as_str(),
                    ),
                    (
                        "independent_test_report_ref",
                        pressure.independent_test_report_ref.as_str(),
                    ),
                ] {
                    if !nonempty(reference) {
                        add_violation(
                            &mut violations,
                            ModuleCapabilityViolationCodeV1::PressureModuleMissingQualification,
                            None,
                            format!("{name} must be non-empty"),
                        );
                    }
                }
                if pressure.qualification_evidence.evidence_class
                    != ModuleEvidenceClassV1::IndependentlyQualified
                {
                    add_violation(
                        &mut violations,
                        ModuleCapabilityViolationCodeV1::PressureModuleMissingQualification,
                        None,
                        "pressure module qualification evidence must be independently_qualified",
                    );
                }
                validate_evidence(&pressure.qualification_evidence, None, &mut violations);
            }
            None => add_violation(
                &mut violations,
                ModuleCapabilityViolationCodeV1::PressureModuleMissingQualification,
                None,
                "pressure-cooker modules require vessel, pressure-relief and independent test evidence",
            ),
        }
    } else if module.pressure_safety.is_some() {
        add_violation(
            &mut violations,
            ModuleCapabilityViolationCodeV1::PressureModuleMissingQualification,
            None,
            "pressure_safety record is only valid for a pressure-cooker module",
        );
    }

    for reference in [
        module.interfaces.mechanical_interface_ref.as_deref(),
        module.interfaces.electrical_interface_ref.as_deref(),
        module.interfaces.data_protocol_ref.as_deref(),
        module.service.user_manual_ref.as_deref(),
        module.service.service_manual_ref.as_deref(),
        module.service.spare_parts_ref.as_deref(),
        module.service.repair_procedure_ref.as_deref(),
        module.service.warranty_ref.as_deref(),
        module.service.cleaning_sanitation_ref.as_deref(),
    ]
    .into_iter()
    .flatten()
    {
        if !nonempty(reference) {
            add_violation(
                &mut violations,
                ModuleCapabilityViolationCodeV1::EmptyInterfaceReference,
                None,
                "optional interface/service references must be non-empty when supplied",
            );
        }
    }

    let mut references = BTreeSet::new();
    for reference in module
        .qualification_refs
        .iter()
        .chain(module.service.food_contact_material_refs.iter())
    {
        if !nonempty(reference) {
            add_violation(
                &mut violations,
                ModuleCapabilityViolationCodeV1::EmptyRequiredValue,
                None,
                "qualification and food-contact material references must be non-empty",
            );
        }
        if !references.insert(reference.as_str()) {
            add_violation(
                &mut violations,
                ModuleCapabilityViolationCodeV1::DuplicateReference,
                None,
                format!("duplicate evidence/reference {reference}"),
            );
        }
    }

    violations.sort_by(|a, b| {
        (
            a.code,
            a.capability_id.as_deref().unwrap_or(""),
            a.detail.as_str(),
        )
            .cmp(&(
                b.code,
                b.capability_id.as_deref().unwrap_or(""),
                b.detail.as_str(),
            ))
    });
    ModuleCapabilityValidationReportV1 {
        module_id: module.module_id.clone(),
        violations,
    }
}

/// Return the capabilities that declare support for an action group after the
/// module has passed structural/evidence validation. This is compatibility
/// discovery only, not proof of fitment, certification or controller authority.
pub fn declared_action_group_capabilities(
    module: &KitchenModuleCapabilityV1,
    action_group: ActionGroupV1,
) -> Result<Vec<&KitchenActionCapabilityV1>, ModuleCapabilityValidationReportV1> {
    let report = validate_kitchen_module_capability(module);
    if !report.is_valid() {
        return Err(report);
    }
    Ok(module
        .capabilities
        .iter()
        .filter(|capability| capability.action_group == action_group)
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    fn evidence(source: &str) -> ModuleEvidenceRefV1 {
        ModuleEvidenceRefV1 {
            source_ref: source.to_owned(),
            method_ref: None,
            calibration_ref: None,
            uncertainty_ref: None,
            evidence_class: ModuleEvidenceClassV1::Declared,
        }
    }

    fn valid_module() -> KitchenModuleCapabilityV1 {
        KitchenModuleCapabilityV1 {
            schema_version: KITCHEN_MODULE_CAPABILITY_SCHEMA_VERSION,
            module_id: "fixture:prep-drive-01".to_owned(),
            module_revision: "rev-a".to_owned(),
            module_kind: KitchenModuleKindV1::PrepDrive,
            manufacturer: "Synthetic Manufacturer".to_owned(),
            model: "Modular Prep Drive".to_owned(),
            sku: Some("FIXTURE-001".to_owned()),
            market_region: "ZA".to_owned(),
            hardware_revision: Some("hw-a".to_owned()),
            firmware_revision: None,
            capabilities: vec![KitchenActionCapabilityV1 {
                capability_id: "capability:chop".to_owned(),
                action_group: ActionGroupV1::Chop,
                input_form_refs: vec!["form:whole-produce".to_owned()],
                output_form_refs: vec!["form:chopped-produce".to_owned()],
                minimum_batch_mass_g: Some(50),
                maximum_batch_mass_g: Some(1_000),
                nominal_throughput_g_per_min: Some(500),
                maximum_continuous_run_ms: Some(120_000),
                parameter_ranges: vec![],
                claim_status: ModuleCapabilityClaimStatusV1::Declared,
                evidence: evidence("fixture:manufacturer-datasheet"),
            }],
            electrical: ModuleElectricalEnvelopeV1 {
                nominal_voltage_min_mv: Some(220_000),
                nominal_voltage_max_mv: Some(240_000),
                mains_frequency_min_millihz: Some(50_000),
                mains_frequency_max_millihz: Some(50_000),
                continuous_real_power_limit_w: Some(1_000),
                peak_real_power_limit_w: Some(1_200),
                maximum_surge_duration_ms: Some(100),
                power_profile_ref: Some("fixture:simulated-power-profile".to_owned()),
                electrical_evidence: Some(evidence("fixture:manufacturer-rating")),
            },
            interfaces: KitchenModuleInterfacesV1 {
                mechanical_interface_ref: Some("adapter:fixture-mount".to_owned()),
                electrical_interface_ref: Some("adapter:fixture-electrical".to_owned()),
                data_protocol_ref: None,
            },
            service: KitchenModuleServiceRefsV1 {
                user_manual_ref: Some("fixture:user-manual".to_owned()),
                service_manual_ref: None,
                spare_parts_ref: None,
                repair_procedure_ref: None,
                warranty_ref: None,
                cleaning_sanitation_ref: Some("fixture:cleaning-protocol".to_owned()),
                food_contact_material_refs: vec![
                    "fixture:food-contact-polymer-datasheet".to_owned(),
                ],
            },
            local_safety_authority_ref: "controller:appliance-local-safety".to_owned(),
            qualification_refs: vec![],
            pressure_safety: None,
            source_evidence: evidence("fixture:module-datasheet"),
            claim_ceiling: KITCHEN_MODULE_CLAIM_CEILING.to_owned(),
        }
    }

    #[derive(Deserialize)]
    struct ModuleFixtureBundle {
        cases: Vec<ModuleFixtureCase>,
    }

    #[derive(Deserialize)]
    struct ModuleFixtureCase {
        fixture_id: String,
        expected_valid: bool,
        module: KitchenModuleCapabilityV1,
    }

    #[test]
    fn synthetic_module_json_fixtures_match_rust_validator() {
        let bundle: ModuleFixtureBundle = serde_json::from_str(include_str!(
            "../schemas/fixtures/kitchen-module-capability-v1.json"
        ))
        .expect("module capability fixture bundle must deserialize");
        for fixture in bundle.cases {
            assert_eq!(
                validate_kitchen_module_capability(&fixture.module).is_valid(),
                fixture.expected_valid,
                "fixture {} did not match expected validity",
                fixture.fixture_id
            );
        }
    }

    #[test]
    fn module_capability_discovery_is_not_actuation_authority() {
        let module = valid_module();
        let matches = declared_action_group_capabilities(&module, ActionGroupV1::Chop).unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].capability_id, "capability:chop");
    }

    #[test]
    fn pressure_cooker_requires_separate_vessel_qualification_reference() {
        let mut module = valid_module();
        module.module_kind = KitchenModuleKindV1::PressureCooker;
        module.qualification_refs.clear();
        let report = validate_kitchen_module_capability(&module);
        assert!(report.violations.iter().any(|v| {
            v.code == ModuleCapabilityViolationCodeV1::PressureModuleMissingQualification
        }));
    }

    #[test]
    fn measured_evidence_without_calibration_is_rejected() {
        let mut module = valid_module();
        module.source_evidence.evidence_class = ModuleEvidenceClassV1::Measured;
        let report = validate_kitchen_module_capability(&module);
        assert!(report.violations.iter().any(|v| {
            v.code == ModuleCapabilityViolationCodeV1::MeasuredEvidenceMissingCalibration
        }));
    }

    #[test]
    fn min_max_batch_reversal_is_rejected() {
        let mut module = valid_module();
        module.capabilities[0].minimum_batch_mass_g = Some(1_000);
        module.capabilities[0].maximum_batch_mass_g = Some(50);
        let report = validate_kitchen_module_capability(&module);
        assert!(
            report
                .violations
                .iter()
                .any(|v| v.code == ModuleCapabilityViolationCodeV1::InvalidBatchRange)
        );
    }
}
