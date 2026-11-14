// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
#![forbid(unsafe_code)]

use serde::de::{self, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Number;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fmt;
use std::fs;
use std::path::Path;

const DEFAULT_SCENARIO_PATH: &str = "docs/tech/MARITIME_FLOATING_SITE_SCENARIO_V0_1.json";
const MAX_INPUT_BYTES: usize = 1_000_000;
const EXPECTED_SCHEMA_ID: &str = "luminous.symtropy.maritime-floating-site-scenario";
const EXPECTED_SCHEMA_VERSION: &str = "0.1.0";
const EXPECTED_SCENARIO_ID: &str = "MARITIME-FLOATING-SITE-SYNTH-001";

const CASE_IDS: &[&str] = &[
    "baseline-declared-envelope-and-energy",
    "vessel-exceeds-berth-length",
    "charging-interruption-energy-shortfall",
    "stale-configuration-currentness",
    "unsupported-nuclear-thermal-hydraulics-request",
    "floating-site-backup-deficit",
];

#[derive(Debug, Clone, PartialEq)]
enum JsonValue {
    Null,
    Bool(bool),
    Number(Number),
    String(String),
    Array(Vec<JsonValue>),
    Object(BTreeMap<String, JsonValue>),
}

struct StrictJsonVisitor;

impl<'de> Visitor<'de> for StrictJsonVisitor {
    type Value = JsonValue;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a JSON value with no duplicate object keys")
    }

    fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
        Ok(JsonValue::Null)
    }

    fn visit_bool<E: de::Error>(self, value: bool) -> Result<Self::Value, E> {
        Ok(JsonValue::Bool(value))
    }

    fn visit_i64<E: de::Error>(self, value: i64) -> Result<Self::Value, E> {
        Ok(JsonValue::Number(Number::from(value)))
    }

    fn visit_u64<E: de::Error>(self, value: u64) -> Result<Self::Value, E> {
        Ok(JsonValue::Number(Number::from(value)))
    }

    fn visit_f64<E: de::Error>(self, value: f64) -> Result<Self::Value, E> {
        Number::from_f64(value)
            .map(JsonValue::Number)
            .ok_or_else(|| E::custom("non-finite JSON number"))
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
        Ok(JsonValue::String(value.to_owned()))
    }

    fn visit_string<E: de::Error>(self, value: String) -> Result<Self::Value, E> {
        Ok(JsonValue::String(value))
    }

    fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
        Ok(JsonValue::Null)
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let mut values = Vec::new();
        while let Some(value) = seq.next_element::<JsonValue>()? {
            values.push(value);
        }
        Ok(JsonValue::Array(values))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut values = BTreeMap::new();
        while let Some(key) = map.next_key::<String>()? {
            let value = map.next_value::<JsonValue>()?;
            match values.entry(key) {
                std::collections::btree_map::Entry::Vacant(entry) => {
                    entry.insert(value);
                }
                std::collections::btree_map::Entry::Occupied(entry) => {
                    return Err(<A::Error as de::Error>::custom(format!(
                        "duplicate object key: {}",
                        entry.key()
                    )));
                }
            }
        }
        Ok(JsonValue::Object(values))
    }
}

impl<'de> Deserialize<'de> for JsonValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(StrictJsonVisitor)
    }
}

#[derive(Debug, Serialize)]
struct CaseResult {
    case_id: String,
    derived_disposition: String,
    expected_disposition: String,
    expectation_matches: bool,
}

#[derive(Debug, Serialize)]
struct VerificationReport {
    status: &'static str,
    schema_id: String,
    schema_version: String,
    scenario_id: String,
    source_path: String,
    source_sha256: String,
    source_commit_binding: String,
    verifier_version: &'static str,
    execution_performed: bool,
    physical_validation: &'static str,
    external_approval: &'static str,
    fixtures_checked: usize,
    cases: Vec<CaseResult>,
    claim_ceiling: Vec<&'static str>,
}

#[derive(Debug)]
struct Contract {
    route_energy_mwh: f64,
    reserve_mwh: f64,
    minimum_departure_energy_mwh: f64,
    baseline_stored_energy_mwh: f64,
    essential_load_mw: f64,
    independent_backup_mw: f64,
}

fn object<'a>(value: &'a JsonValue, path: &str) -> Result<&'a BTreeMap<String, JsonValue>, String> {
    match value {
        JsonValue::Object(value) => Ok(value),
        _ => Err(format!("{path}: expected object")),
    }
}

fn array<'a>(value: &'a JsonValue, path: &str) -> Result<&'a [JsonValue], String> {
    match value {
        JsonValue::Array(value) => Ok(value),
        _ => Err(format!("{path}: expected array")),
    }
}

fn field<'a>(value: &'a JsonValue, key: &str, path: &str) -> Result<&'a JsonValue, String> {
    let values = object(value, path)?;
    values
        .get(key)
        .ok_or_else(|| format!("{path}: missing required field {key}"))
}

fn text<'a>(value: &'a JsonValue, path: &str) -> Result<&'a str, String> {
    match value {
        JsonValue::String(value) => Ok(value),
        _ => Err(format!("{path}: expected string")),
    }
}

fn boolean(value: &JsonValue, path: &str) -> Result<bool, String> {
    match value {
        JsonValue::Bool(value) => Ok(*value),
        _ => Err(format!("{path}: expected boolean")),
    }
}

fn number(value: &JsonValue, path: &str) -> Result<f64, String> {
    match value {
        JsonValue::Number(value) => {
            let value = value
                .as_f64()
                .ok_or_else(|| format!("{path}: number is outside finite f64 range"))?;
            if !value.is_finite() {
                return Err(format!("{path}: number must be finite"));
            }
            Ok(value)
        }
        _ => Err(format!("{path}: expected number")),
    }
}

fn exact_keys(value: &JsonValue, expected: &[&str], path: &str) -> Result<(), String> {
    let actual_map = object(value, path)?;
    let actual: BTreeSet<&str> = actual_map.keys().map(String::as_str).collect();
    let expected: BTreeSet<&str> = expected.iter().copied().collect();
    if actual == expected {
        return Ok(());
    }
    let missing = expected.difference(&actual).copied().collect::<Vec<_>>();
    let unknown = actual.difference(&expected).copied().collect::<Vec<_>>();
    Err(format!(
        "{path}: field-set mismatch; missing={missing:?}, unknown={unknown:?}"
    ))
}

fn expect_text(value: &JsonValue, path: &str, expected: &str) -> Result<(), String> {
    let actual = text(value, path)?;
    if actual == expected {
        Ok(())
    } else {
        Err(format!("{path}: expected {expected:?}, got {actual:?}"))
    }
}

fn expect_bool(value: &JsonValue, path: &str, expected: bool) -> Result<(), String> {
    let actual = boolean(value, path)?;
    if actual == expected {
        Ok(())
    } else {
        Err(format!("{path}: expected {expected}, got {actual}"))
    }
}

fn expect_number(value: &JsonValue, path: &str, expected: f64) -> Result<(), String> {
    let actual = number(value, path)?;
    if actual == expected {
        Ok(())
    } else {
        Err(format!("{path}: expected {expected:?}, got {actual:?}"))
    }
}

fn expect_string_set(value: &JsonValue, expected: &[&str], path: &str) -> Result<(), String> {
    let values = array(value, path)?;
    let mut actual = BTreeSet::new();
    for (index, value) in values.iter().enumerate() {
        let item = text(value, &format!("{path}[{index}]"))?;
        if !actual.insert(item) {
            return Err(format!("{path}: duplicate array value {item:?}"));
        }
    }
    let expected: BTreeSet<&str> = expected.iter().copied().collect();
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "{path}: set mismatch; missing={:?}, unknown={:?}",
            expected.difference(&actual).copied().collect::<Vec<_>>(),
            actual.difference(&expected).copied().collect::<Vec<_>>()
        ))
    }
}

fn quantity(value: &JsonValue, expected_unit: &str, path: &str) -> Result<f64, String> {
    exact_keys(value, &["unit", "value"], path)?;
    expect_text(
        field(value, "unit", path)?,
        &format!("{path}.unit"),
        expected_unit,
    )?;
    let value = number(field(value, "value", path)?, &format!("{path}.value"))?;
    if value < 0.0 {
        return Err(format!(
            "{path}.value: negative quantity is outside this fixture's domain"
        ));
    }
    Ok(value)
}

fn frozen_quantity(
    value: &JsonValue,
    unit: &str,
    expected: f64,
    path: &str,
) -> Result<f64, String> {
    let actual = quantity(value, unit, path)?;
    if actual != expected {
        return Err(format!(
            "{path}: frozen v0.1.0 value changed; expected {expected}, got {actual}"
        ));
    }
    Ok(actual)
}

fn envelope_fits(
    vessel_length_m: f64,
    vessel_width_m: f64,
    vessel_draft_m: f64,
    berth_length_m: f64,
    berth_width_m: f64,
    berth_max_draft_m: f64,
) -> bool {
    vessel_length_m <= berth_length_m
        && vessel_width_m <= berth_width_m
        && vessel_draft_m <= berth_max_draft_m
}

fn lookup_entity<'a>(entities: &'a [JsonValue], wanted_id: &str) -> Result<&'a JsonValue, String> {
    entities
        .iter()
        .find(|entity| {
            field(entity, "id", "entity")
                .and_then(|v| text(v, "entity.id"))
                .ok()
                == Some(wanted_id)
        })
        .ok_or_else(|| format!("missing entity reference {wanted_id:?}"))
}

fn validate_status(root: &JsonValue) -> Result<(), String> {
    let status = field(root, "status", "root")?;
    exact_keys(
        status,
        &[
            "kind",
            "execution_performed",
            "engine_capability_admission",
            "physical_validation",
            "external_approval",
        ],
        "status",
    )?;
    expect_text(
        field(status, "kind", "status")?,
        "status.kind",
        "SpecificationOnly",
    )?;
    expect_bool(
        field(status, "execution_performed", "status")?,
        "status.execution_performed",
        false,
    )?;
    expect_text(
        field(status, "engine_capability_admission", "status")?,
        "status.engine_capability_admission",
        "NotAssessed",
    )?;
    expect_text(
        field(status, "physical_validation", "status")?,
        "status.physical_validation",
        "None",
    )?;
    expect_text(
        field(status, "external_approval", "status")?,
        "status.external_approval",
        "None",
    )?;
    Ok(())
}

fn validate_provenance(root: &JsonValue) -> Result<(), String> {
    let provenance = field(root, "provenance", "root")?;
    exact_keys(
        provenance,
        &[
            "repository",
            "parent_issue",
            "specification_path",
            "source_data_class",
            "real_site_data_used",
            "note",
        ],
        "provenance",
    )?;
    expect_text(
        field(provenance, "repository", "provenance")?,
        "provenance.repository",
        "Luminous-Dynamics/symtropy",
    )?;
    expect_text(
        field(provenance, "parent_issue", "provenance")?,
        "provenance.parent_issue",
        "https://github.com/Luminous-Dynamics/symtropy/issues/1612",
    )?;
    expect_text(
        field(provenance, "specification_path", "provenance")?,
        "provenance.specification_path",
        "docs/tech/MARITIME_FLOATING_SITE_SCENARIO_V0_1.md",
    )?;
    expect_text(
        field(provenance, "source_data_class", "provenance")?,
        "provenance.source_data_class",
        "SyntheticIllustrative",
    )?;
    expect_bool(
        field(provenance, "real_site_data_used", "provenance")?,
        "provenance.real_site_data_used",
        false,
    )?;
    expect_text(
        field(provenance, "note", "provenance")?,
        "provenance.note",
        "All values are invented for contract and arithmetic checks; they are not a real vessel, port, grid, settlement, or nuclear supplier profile.",
    )?;
    Ok(())
}

fn validate_execution_contract(root: &JsonValue) -> Result<(), String> {
    let contract = field(root, "execution_contract", "root")?;
    exact_keys(
        contract,
        &[
            "requested_profile",
            "headless",
            "fixed_timestep_required",
            "seed_policy",
            "stable_entity_ids_required",
            "independent_oracle_required",
            "canonical_output_required",
            "postflight_immutability_required",
            "profile_admission",
            "unsupported_model_semantics",
        ],
        "execution_contract",
    )?;
    for (key, value) in [
        ("requested_profile", "EngineeringDeterministic"),
        ("seed_policy", "ExplicitAndFrozenBeforeExecution"),
        (
            "profile_admission",
            "Must be checked against the exact current engine capability census and ENG-SYNTH-CRED gate.",
        ),
        (
            "unsupported_model_semantics",
            "UnsupportedOrUnknown; never silently approximate.",
        ),
    ] {
        expect_text(
            field(contract, key, "execution_contract")?,
            &format!("execution_contract.{key}"),
            value,
        )?;
    }
    for key in [
        "headless",
        "fixed_timestep_required",
        "stable_entity_ids_required",
        "independent_oracle_required",
        "canonical_output_required",
        "postflight_immutability_required",
    ] {
        expect_bool(
            field(contract, key, "execution_contract")?,
            &format!("execution_contract.{key}"),
            true,
        )?;
    }
    Ok(())
}

fn validate_entities(root: &JsonValue) -> Result<Vec<JsonValue>, String> {
    let entities = array(field(root, "entities", "root")?, "entities")?;
    if entities.len() != 4 {
        return Err(format!(
            "entities: expected 4 frozen entities, got {}",
            entities.len()
        ));
    }

    let mut ids = BTreeSet::new();
    for (index, entity) in entities.iter().enumerate() {
        let path = format!("entities[{index}]");
        let id = text(field(entity, "id", &path)?, &format!("{path}.id"))?;
        if !ids.insert(id.to_owned()) {
            return Err(format!("{path}: duplicate entity id {id:?}"));
        }
        match id {
            "vessel.synthetic-cargo-01" => {
                exact_keys(
                    entity,
                    &["id", "kind", "dimensions", "evidence_class"],
                    &path,
                )?;
                expect_text(
                    field(entity, "kind", &path)?,
                    &format!("{path}.kind"),
                    "AbstractVesselEnvelope",
                )?;
                expect_text(
                    field(entity, "evidence_class", &path)?,
                    &format!("{path}.evidence_class"),
                    "SyntheticAssumption",
                )?;
                let dimensions = field(entity, "dimensions", &path)?;
                exact_keys(
                    dimensions,
                    &["length", "width", "declared_draft"],
                    &format!("{path}.dimensions"),
                )?;
                frozen_quantity(
                    field(dimensions, "length", &path)?,
                    "m",
                    80.0,
                    "vessel.length",
                )?;
                frozen_quantity(
                    field(dimensions, "width", &path)?,
                    "m",
                    12.0,
                    "vessel.width",
                )?;
                frozen_quantity(
                    field(dimensions, "declared_draft", &path)?,
                    "m",
                    2.5,
                    "vessel.declared_draft",
                )?;
            }
            "berth.synthetic-01" => {
                exact_keys(
                    entity,
                    &["id", "kind", "constraints", "evidence_class"],
                    &path,
                )?;
                expect_text(
                    field(entity, "kind", &path)?,
                    &format!("{path}.kind"),
                    "AbstractBerthEnvelope",
                )?;
                expect_text(
                    field(entity, "evidence_class", &path)?,
                    &format!("{path}.evidence_class"),
                    "SyntheticAssumption",
                )?;
                let constraints = field(entity, "constraints", &path)?;
                exact_keys(
                    constraints,
                    &["maximum_length", "maximum_width", "maximum_declared_draft"],
                    &format!("{path}.constraints"),
                )?;
                frozen_quantity(
                    field(constraints, "maximum_length", &path)?,
                    "m",
                    100.0,
                    "berth.maximum_length",
                )?;
                frozen_quantity(
                    field(constraints, "maximum_width", &path)?,
                    "m",
                    18.0,
                    "berth.maximum_width",
                )?;
                frozen_quantity(
                    field(constraints, "maximum_declared_draft", &path)?,
                    "m",
                    3.0,
                    "berth.maximum_declared_draft",
                )?;
            }
            "energy.shore-power-01" => {
                exact_keys(entity, &["id", "kind", "evidence_class"], &path)?;
                expect_text(
                    field(entity, "kind", &path)?,
                    &format!("{path}.kind"),
                    "AbstractChargingResource",
                )?;
                expect_text(
                    field(entity, "evidence_class", &path)?,
                    &format!("{path}.evidence_class"),
                    "SyntheticAssumption",
                )?;
            }
            "site.floating-essential-loads-01" => {
                exact_keys(entity, &["id", "kind", "evidence_class"], &path)?;
                expect_text(
                    field(entity, "kind", &path)?,
                    &format!("{path}.kind"),
                    "AbstractEssentialLoadGroup",
                )?;
                expect_text(
                    field(entity, "evidence_class", &path)?,
                    &format!("{path}.evidence_class"),
                    "SyntheticAssumption",
                )?;
            }
            _ => {
                return Err(format!(
                    "{path}: unknown entity id {id:?} in frozen v0.1.0 fixture"
                ));
            }
        }
    }

    let expected: BTreeSet<&str> = [
        "vessel.synthetic-cargo-01",
        "berth.synthetic-01",
        "energy.shore-power-01",
        "site.floating-essential-loads-01",
    ]
    .into_iter()
    .collect();
    let actual: BTreeSet<&str> = ids.iter().map(String::as_str).collect();
    if actual != expected {
        return Err("entities: frozen identifier set mismatch".to_owned());
    }
    Ok(entities.to_vec())
}

fn validate_contracts(root: &JsonValue) -> Result<Contract, String> {
    let contracts = field(root, "contracts", "root")?;
    exact_keys(
        contracts,
        &[
            "berth_envelope",
            "battery_departure_energy",
            "floating_site_essential_load",
        ],
        "contracts",
    )?;

    let berth = field(contracts, "berth_envelope", "contracts")?;
    exact_keys(
        berth,
        &[
            "model_scope",
            "required_vessel_dimensions",
            "excluded_semantics",
        ],
        "contracts.berth_envelope",
    )?;
    expect_text(
        field(berth, "model_scope", "contracts.berth_envelope")?,
        "contracts.berth_envelope.model_scope",
        "ComponentWiseDeclaredEnvelopeComparisonOnly",
    )?;
    expect_string_set(
        field(
            berth,
            "required_vessel_dimensions",
            "contracts.berth_envelope",
        )?,
        &["length", "width", "declared_draft"],
        "contracts.berth_envelope.required_vessel_dimensions",
    )?;
    expect_string_set(
        field(berth, "excluded_semantics", "contracts.berth_envelope")?,
        &[
            "UnderKeelClearance",
            "TidesAndWaterLevels",
            "Maneuvering",
            "MooringLoads",
            "FenderLoads",
            "Hydrodynamics",
            "Seakeeping",
            "StructuralQualification",
            "LocalPortApproval",
        ],
        "contracts.berth_envelope.excluded_semantics",
    )?;

    let energy = field(contracts, "battery_departure_energy", "contracts")?;
    exact_keys(
        energy,
        &[
            "route_energy",
            "protected_reserve",
            "minimum_departure_energy",
            "baseline_stored_energy",
            "arithmetic",
            "excluded_semantics",
        ],
        "contracts.battery_departure_energy",
    )?;
    let route_energy_mwh = frozen_quantity(
        field(energy, "route_energy", "contracts.battery_departure_energy")?,
        "MWh",
        4.0,
        "contracts.route_energy",
    )?;
    let reserve_mwh = frozen_quantity(
        field(
            energy,
            "protected_reserve",
            "contracts.battery_departure_energy",
        )?,
        "MWh",
        1.5,
        "contracts.protected_reserve",
    )?;
    let minimum_departure_energy_mwh = frozen_quantity(
        field(
            energy,
            "minimum_departure_energy",
            "contracts.battery_departure_energy",
        )?,
        "MWh",
        5.5,
        "contracts.minimum_departure_energy",
    )?;
    let baseline_stored_energy_mwh = frozen_quantity(
        field(
            energy,
            "baseline_stored_energy",
            "contracts.battery_departure_energy",
        )?,
        "MWh",
        6.0,
        "contracts.baseline_stored_energy",
    )?;
    if route_energy_mwh + reserve_mwh != minimum_departure_energy_mwh {
        return Err("route energy plus reserve does not equal the frozen minimum".to_owned());
    }
    expect_text(
        field(energy, "arithmetic", "contracts.battery_departure_energy")?,
        "contracts.battery_departure_energy.arithmetic",
        "available_departure_energy >= route_energy + protected_reserve",
    )?;
    expect_string_set(
        field(
            energy,
            "excluded_semantics",
            "contracts.battery_departure_energy",
        )?,
        &[
            "RealVesselPropulsionPower",
            "HydrodynamicallyDerivedEnergyDemand",
            "BatteryThermalBehavior",
            "BatteryDegradationModel",
            "PowerQuality",
            "BatterySafetyQualification",
        ],
        "contracts.battery_departure_energy.excluded_semantics",
    )?;

    let site = field(contracts, "floating_site_essential_load", "contracts")?;
    exact_keys(
        site,
        &[
            "synthetic_essential_load",
            "synthetic_independent_backup",
            "arithmetic",
            "expected_instantaneous_deficit",
            "excluded_semantics",
        ],
        "contracts.floating_site_essential_load",
    )?;
    let essential_load_mw = frozen_quantity(
        field(
            site,
            "synthetic_essential_load",
            "contracts.floating_site_essential_load",
        )?,
        "MW",
        1.2,
        "contracts.synthetic_essential_load",
    )?;
    let independent_backup_mw = frozen_quantity(
        field(
            site,
            "synthetic_independent_backup",
            "contracts.floating_site_essential_load",
        )?,
        "MW",
        0.5,
        "contracts.synthetic_independent_backup",
    )?;
    frozen_quantity(
        field(
            site,
            "expected_instantaneous_deficit",
            "contracts.floating_site_essential_load",
        )?,
        "MW",
        0.7,
        "contracts.expected_instantaneous_deficit",
    )?;
    if (essential_load_mw - independent_backup_mw).max(0.0) != 0.7 {
        return Err(
            "derived instantaneous site deficit does not equal the frozen value".to_owned(),
        );
    }
    expect_text(
        field(site, "arithmetic", "contracts.floating_site_essential_load")?,
        "contracts.floating_site_essential_load.arithmetic",
        "reported_deficit = max(0, essential_load - independent_backup)",
    )?;
    expect_string_set(
        field(
            site,
            "excluded_semantics",
            "contracts.floating_site_essential_load",
        )?,
        &[
            "EnergyDuration",
            "PowerSystemTransientStability",
            "ProtectionCoordination",
            "GridFormingControls",
            "NuclearSafetyFunctions",
            "EmergencyLifeSafetyQualification",
        ],
        "contracts.floating_site_essential_load.excluded_semantics",
    )?;

    Ok(Contract {
        route_energy_mwh,
        reserve_mwh,
        minimum_departure_energy_mwh,
        baseline_stored_energy_mwh,
        essential_load_mw,
        independent_backup_mw,
    })
}

fn validate_claim_ceiling(root: &JsonValue) -> Result<(), String> {
    let invariants = field(root, "global_invariants", "root")?;
    expect_string_set(
        invariants,
        &[
            "SyntheticScenarioResult != PhysicalObservation",
            "DeterministicReplay != ModelValidity",
            "RunCompleted != RequirementSatisfied",
            "EngineeringFeasibility != ExternalApproval",
            "PlatformServiceRestored != PhysicalPlantOrVesselRestored",
            "AuthenticatedEvent != IndependentlyVerifiedPhysicalFact",
            "UnavailableOrStaleCriticalEvidence != Healthy",
            "UnsupportedPhysics != ApproximationByDefault",
        ],
        "global_invariants",
    )?;

    let security = field(root, "security_and_privacy", "root")?;
    exact_keys(
        security,
        &[
            "uses_real_facility_layout",
            "contains_tactical_security_details",
            "contains_credentials_or_personal_data",
            "public_artifact_class",
            "site_specific_security_plan_required_in_public_artifact",
        ],
        "security_and_privacy",
    )?;
    expect_bool(
        field(
            security,
            "uses_real_facility_layout",
            "security_and_privacy",
        )?,
        "security_and_privacy.uses_real_facility_layout",
        false,
    )?;
    expect_bool(
        field(
            security,
            "contains_tactical_security_details",
            "security_and_privacy",
        )?,
        "security_and_privacy.contains_tactical_security_details",
        false,
    )?;
    expect_bool(
        field(
            security,
            "contains_credentials_or_personal_data",
            "security_and_privacy",
        )?,
        "security_and_privacy.contains_credentials_or_personal_data",
        false,
    )?;
    expect_text(
        field(security, "public_artifact_class", "security_and_privacy")?,
        "security_and_privacy.public_artifact_class",
        "SanitizedSyntheticSpecification",
    )?;
    expect_bool(
        field(
            security,
            "site_specific_security_plan_required_in_public_artifact",
            "security_and_privacy",
        )?,
        "security_and_privacy.site_specific_security_plan_required_in_public_artifact",
        false,
    )?;

    expect_string_set(
        field(root, "claim_ceiling", "root")?,
        &[
            "Contract and arithmetic behavior only, for exact fixture inputs.",
            "No seaworthiness, dock safety, nuclear safety/security, cyber-resilience, grid stability, emergency-egress, commercial-performance, regulatory-approval, or physical-operation claim.",
        ],
        "claim_ceiling",
    )?;
    Ok(())
}

fn fixture_map(root: &JsonValue) -> Result<BTreeMap<String, &JsonValue>, String> {
    let values = array(field(root, "fixtures", "root")?, "fixtures")?;
    if values.len() != CASE_IDS.len() {
        return Err(format!(
            "fixtures: expected {} frozen cases, got {}",
            CASE_IDS.len(),
            values.len()
        ));
    }
    let mut result = BTreeMap::new();
    let mut observed_order = Vec::new();
    for (index, fixture) in values.iter().enumerate() {
        let path = format!("fixtures[{index}]");
        exact_keys(
            fixture,
            &["case_id", "revision", "inputs", "expected"],
            &path,
        )?;
        let case_id = text(
            field(fixture, "case_id", &path)?,
            &format!("{path}.case_id"),
        )?;
        if result.insert(case_id.to_owned(), fixture).is_some() {
            return Err(format!("{path}: duplicate case_id {case_id:?}"));
        }
        expect_number(
            field(fixture, "revision", &path)?,
            &format!("{path}.revision"),
            1.0,
        )?;
        observed_order.push(case_id);
    }
    if observed_order.as_slice() != CASE_IDS {
        return Err(format!(
            "fixtures: ordered case-id list does not match frozen v0.1.0 corpus: {observed_order:?}"
        ));
    }
    Ok(result)
}

fn expected_disposition<'a>(expected: &'a JsonValue, path: &str) -> Result<&'a str, String> {
    text(
        field(expected, "disposition", path)?,
        &format!("{path}.disposition"),
    )
}

fn verify_case(
    case_id: &str,
    fixture: &JsonValue,
    entities: &[JsonValue],
    contract: &Contract,
) -> Result<CaseResult, String> {
    let path = format!("fixture.{case_id}");
    let inputs = field(fixture, "inputs", &path)?;
    let expected = field(fixture, "expected", &path)?;
    let disposition = match case_id {
        "baseline-declared-envelope-and-energy" => {
            exact_keys(
                inputs,
                &["vessel_entity", "berth_entity", "stored_departure_energy"],
                &format!("{path}.inputs"),
            )?;
            exact_keys(
                expected,
                &[
                    "disposition",
                    "berth_envelope",
                    "departure_energy",
                    "physical_qualification",
                    "regulatory_approval",
                ],
                &format!("{path}.expected"),
            )?;
            expect_text(
                field(inputs, "vessel_entity", &path)?,
                "baseline.inputs.vessel_entity",
                "vessel.synthetic-cargo-01",
            )?;
            expect_text(
                field(inputs, "berth_entity", &path)?,
                "baseline.inputs.berth_entity",
                "berth.synthetic-01",
            )?;
            let stored = frozen_quantity(
                field(inputs, "stored_departure_energy", &path)?,
                "MWh",
                contract.baseline_stored_energy_mwh,
                "baseline.stored_departure_energy",
            )?;
            let vessel = lookup_entity(entities, "vessel.synthetic-cargo-01")?;
            let berth = lookup_entity(entities, "berth.synthetic-01")?;
            let dimensions = field(vessel, "dimensions", "vessel")?;
            let constraints = field(berth, "constraints", "berth")?;
            let fits = envelope_fits(
                quantity(
                    field(dimensions, "length", "vessel.dimensions")?,
                    "m",
                    "vessel.length",
                )?,
                quantity(
                    field(dimensions, "width", "vessel.dimensions")?,
                    "m",
                    "vessel.width",
                )?,
                quantity(
                    field(dimensions, "declared_draft", "vessel.dimensions")?,
                    "m",
                    "vessel.declared_draft",
                )?,
                quantity(
                    field(constraints, "maximum_length", "berth.constraints")?,
                    "m",
                    "berth.maximum_length",
                )?,
                quantity(
                    field(constraints, "maximum_width", "berth.constraints")?,
                    "m",
                    "berth.maximum_width",
                )?,
                quantity(
                    field(constraints, "maximum_declared_draft", "berth.constraints")?,
                    "m",
                    "berth.maximum_declared_draft",
                )?,
            );
            let energy_ok = stored >= contract.minimum_departure_energy_mwh;
            let envelope_result = if fits {
                "FitsDeclaredSyntheticEnvelope"
            } else {
                "OutsideDeclaredSyntheticEnvelope"
            };
            let energy_result = if energy_ok {
                "SufficientByDeclaredArithmetic"
            } else {
                "InsufficientByDeclaredArithmetic"
            };
            expect_text(
                field(expected, "berth_envelope", &path)?,
                "baseline.expected.berth_envelope",
                envelope_result,
            )?;
            expect_text(
                field(expected, "departure_energy", &path)?,
                "baseline.expected.departure_energy",
                energy_result,
            )?;
            expect_bool(
                field(expected, "physical_qualification", &path)?,
                "baseline.expected.physical_qualification",
                false,
            )?;
            expect_bool(
                field(expected, "regulatory_approval", &path)?,
                "baseline.expected.regulatory_approval",
                false,
            )?;
            if fits && energy_ok {
                "PassDeclaredChecksOnly"
            } else {
                "Infeasible"
            }
        }
        "vessel-exceeds-berth-length" => {
            exact_keys(
                inputs,
                &["vessel_dimensions", "berth_entity"],
                &format!("{path}.inputs"),
            )?;
            exact_keys(
                expected,
                &[
                    "disposition",
                    "failed_constraint",
                    "silent_fallback_allowed",
                ],
                &format!("{path}.expected"),
            )?;
            expect_text(
                field(inputs, "berth_entity", &path)?,
                "oversize.inputs.berth_entity",
                "berth.synthetic-01",
            )?;
            let vessel_dims = field(inputs, "vessel_dimensions", &path)?;
            exact_keys(
                vessel_dims,
                &["length", "width", "declared_draft"],
                "oversize.inputs.vessel_dimensions",
            )?;
            frozen_quantity(
                field(vessel_dims, "length", &path)?,
                "m",
                105.0,
                "oversize.vessel.length",
            )?;
            frozen_quantity(
                field(vessel_dims, "width", &path)?,
                "m",
                12.0,
                "oversize.vessel.width",
            )?;
            frozen_quantity(
                field(vessel_dims, "declared_draft", &path)?,
                "m",
                2.5,
                "oversize.vessel.declared_draft",
            )?;
            let berth = lookup_entity(entities, "berth.synthetic-01")?;
            let constraints = field(berth, "constraints", "berth")?;
            let length = quantity(
                field(vessel_dims, "length", &path)?,
                "m",
                "oversize.vessel.length",
            )?;
            let max_length = quantity(
                field(constraints, "maximum_length", "berth.constraints")?,
                "m",
                "berth.maximum_length",
            )?;
            if length <= max_length {
                return Err(
                    "oversize fixture no longer exceeds its declared berth length".to_owned(),
                );
            }
            expect_text(
                field(expected, "failed_constraint", &path)?,
                "oversize.expected.failed_constraint",
                "VesselLengthExceedsDeclaredBerthLength",
            )?;
            expect_bool(
                field(expected, "silent_fallback_allowed", &path)?,
                "oversize.expected.silent_fallback_allowed",
                false,
            )?;
            "Infeasible"
        }
        "charging-interruption-energy-shortfall" => {
            exact_keys(
                inputs,
                &[
                    "starting_stored_energy",
                    "charging_power",
                    "charging_duration",
                    "charging_efficiency",
                    "route_energy",
                    "protected_reserve",
                ],
                &format!("{path}.inputs"),
            )?;
            exact_keys(
                expected,
                &[
                    "charge_added_energy",
                    "available_departure_energy",
                    "minimum_departure_energy",
                    "disposition",
                    "failed_constraint",
                ],
                &format!("{path}.expected"),
            )?;
            let start = frozen_quantity(
                field(inputs, "starting_stored_energy", &path)?,
                "MWh",
                3.0,
                "charging.start",
            )?;
            let power = frozen_quantity(
                field(inputs, "charging_power", &path)?,
                "MW",
                2.0,
                "charging.power",
            )?;
            let duration = frozen_quantity(
                field(inputs, "charging_duration", &path)?,
                "h",
                1.0,
                "charging.duration",
            )?;
            let efficiency = number(
                field(inputs, "charging_efficiency", &path)?,
                "charging.efficiency",
            )?;
            if !(0.0..=1.0).contains(&efficiency) {
                return Err(
                    "charging.efficiency must be in the inclusive [0, 1] interval".to_owned(),
                );
            }
            if efficiency != 0.9 {
                return Err("charging.efficiency: frozen v0.1.0 value changed".to_owned());
            }
            let route = frozen_quantity(
                field(inputs, "route_energy", &path)?,
                "MWh",
                contract.route_energy_mwh,
                "charging.route_energy",
            )?;
            let reserve = frozen_quantity(
                field(inputs, "protected_reserve", &path)?,
                "MWh",
                contract.reserve_mwh,
                "charging.reserve",
            )?;
            let added = power * duration * efficiency;
            let available = start + added;
            let required = route + reserve;
            expect_quantity_value(
                field(expected, "charge_added_energy", &path)?,
                "MWh",
                added,
                "charging.expected.charge_added_energy",
            )?;
            expect_quantity_value(
                field(expected, "available_departure_energy", &path)?,
                "MWh",
                available,
                "charging.expected.available_departure_energy",
            )?;
            expect_quantity_value(
                field(expected, "minimum_departure_energy", &path)?,
                "MWh",
                required,
                "charging.expected.minimum_departure_energy",
            )?;
            expect_text(
                field(expected, "failed_constraint", &path)?,
                "charging.expected.failed_constraint",
                "DepartureEnergyBelowRoutePlusReserve",
            )?;
            if available >= required {
                return Err(
                    "charging fixture no longer demonstrates a reserve-inclusive energy shortfall"
                        .to_owned(),
                );
            }
            "Infeasible"
        }
        "stale-configuration-currentness" => {
            exact_keys(
                inputs,
                &[
                    "run_configuration_revision",
                    "current_asset_configuration_revision",
                ],
                &format!("{path}.inputs"),
            )?;
            exact_keys(
                expected,
                &[
                    "disposition",
                    "reason",
                    "prior_result_may_be_carried_forward",
                ],
                &format!("{path}.expected"),
            )?;
            let run = text(
                field(inputs, "run_configuration_revision", &path)?,
                "stale.run_configuration_revision",
            )?;
            let current = text(
                field(inputs, "current_asset_configuration_revision", &path)?,
                "stale.current_asset_configuration_revision",
            )?;
            if run != "config-001" || current != "config-002" || run == current {
                return Err(
                    "stale configuration fixture identity changed or mismatch vanished".to_owned(),
                );
            }
            expect_text(
                field(expected, "reason", &path)?,
                "stale.expected.reason",
                "ConfigurationIdentityMismatch",
            )?;
            expect_bool(
                field(expected, "prior_result_may_be_carried_forward", &path)?,
                "stale.expected.prior_result_may_be_carried_forward",
                false,
            )?;
            "Indeterminate"
        }
        "unsupported-nuclear-thermal-hydraulics-request" => {
            exact_keys(
                inputs,
                &["requested_model", "available_model_evidence"],
                &format!("{path}.inputs"),
            )?;
            exact_keys(
                expected,
                &[
                    "disposition",
                    "silent_rigid_body_approximation_allowed",
                    "nuclear_safety_claim_allowed",
                ],
                &format!("{path}.expected"),
            )?;
            expect_text(
                field(inputs, "requested_model", &path)?,
                "unsupported.requested_model",
                "NuclearThermalHydraulics",
            )?;
            expect_text(
                field(inputs, "available_model_evidence", &path)?,
                "unsupported.available_model_evidence",
                "None",
            )?;
            expect_bool(
                field(expected, "silent_rigid_body_approximation_allowed", &path)?,
                "unsupported.expected.silent_rigid_body_approximation_allowed",
                false,
            )?;
            expect_bool(
                field(expected, "nuclear_safety_claim_allowed", &path)?,
                "unsupported.expected.nuclear_safety_claim_allowed",
                false,
            )?;
            "Unsupported"
        }
        "floating-site-backup-deficit" => {
            exact_keys(
                inputs,
                &["essential_load", "independent_backup"],
                &format!("{path}.inputs"),
            )?;
            exact_keys(
                expected,
                &[
                    "disposition",
                    "instantaneous_unserved_load",
                    "duration_claim_allowed",
                    "power_quality_claim_allowed",
                ],
                &format!("{path}.expected"),
            )?;
            let load = frozen_quantity(
                field(inputs, "essential_load", &path)?,
                "MW",
                contract.essential_load_mw,
                "site.essential_load",
            )?;
            let backup = frozen_quantity(
                field(inputs, "independent_backup", &path)?,
                "MW",
                contract.independent_backup_mw,
                "site.independent_backup",
            )?;
            let deficit = (load - backup).max(0.0);
            expect_quantity_value(
                field(expected, "instantaneous_unserved_load", &path)?,
                "MW",
                deficit,
                "site.expected.instantaneous_unserved_load",
            )?;
            expect_bool(
                field(expected, "duration_claim_allowed", &path)?,
                "site.expected.duration_claim_allowed",
                false,
            )?;
            expect_bool(
                field(expected, "power_quality_claim_allowed", &path)?,
                "site.expected.power_quality_claim_allowed",
                false,
            )?;
            if deficit <= 0.0 {
                return Err(
                    "floating-site fixture no longer represents an instantaneous service deficit"
                        .to_owned(),
                );
            }
            "ServiceDeficit"
        }
        _ => return Err(format!("{path}: unknown case id")),
    };

    let expected_value = expected_disposition(expected, &format!("{path}.expected"))?;
    if expected_value != disposition {
        return Err(format!(
            "{path}: expected disposition {:?} disagrees with independently derived {:?}",
            expected_value, disposition
        ));
    }
    Ok(CaseResult {
        case_id: case_id.to_owned(),
        derived_disposition: disposition.to_owned(),
        expected_disposition: expected_value.to_owned(),
        expectation_matches: true,
    })
}

fn expect_quantity_value(
    value: &JsonValue,
    unit: &str,
    expected: f64,
    path: &str,
) -> Result<(), String> {
    let actual = quantity(value, unit, path)?;
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "{path}: expected {expected} {unit}, got {actual} {unit}"
        ))
    }
}

fn parse_strict(raw: &[u8]) -> Result<JsonValue, String> {
    if raw.len() > MAX_INPUT_BYTES {
        return Err(format!(
            "input exceeds the {}-byte safety bound",
            MAX_INPUT_BYTES
        ));
    }
    let mut deserializer = serde_json::Deserializer::from_slice(raw);
    let value = JsonValue::deserialize(&mut deserializer)
        .map_err(|error| format!("invalid JSON: {error}"))?;
    deserializer
        .end()
        .map_err(|error| format!("trailing JSON data: {error}"))?;
    Ok(value)
}

fn validate_document(root: &JsonValue) -> Result<(String, String, Vec<CaseResult>), String> {
    exact_keys(
        root,
        &[
            "schema_id",
            "schema_version",
            "scenario_id",
            "scenario_revision",
            "status",
            "provenance",
            "execution_contract",
            "entities",
            "contracts",
            "fixtures",
            "global_invariants",
            "security_and_privacy",
            "claim_ceiling",
        ],
        "root",
    )?;
    expect_text(
        field(root, "schema_id", "root")?,
        "schema_id",
        EXPECTED_SCHEMA_ID,
    )?;
    expect_text(
        field(root, "schema_version", "root")?,
        "schema_version",
        EXPECTED_SCHEMA_VERSION,
    )?;
    expect_text(
        field(root, "scenario_id", "root")?,
        "scenario_id",
        EXPECTED_SCENARIO_ID,
    )?;
    expect_number(
        field(root, "scenario_revision", "root")?,
        "scenario_revision",
        1.0,
    )?;
    validate_status(root)?;
    validate_provenance(root)?;
    validate_execution_contract(root)?;
    let entities = validate_entities(root)?;
    let contract = validate_contracts(root)?;
    validate_claim_ceiling(root)?;
    let fixtures = fixture_map(root)?;
    let mut cases = Vec::with_capacity(CASE_IDS.len());
    for case_id in CASE_IDS {
        let fixture = fixtures
            .get(*case_id)
            .ok_or_else(|| format!("missing frozen case {case_id:?}"))?;
        cases.push(verify_case(case_id, fixture, &entities, &contract)?);
    }
    Ok((
        EXPECTED_SCHEMA_ID.to_owned(),
        EXPECTED_SCHEMA_VERSION.to_owned(),
        cases,
    ))
}

fn verify_bytes(
    raw: &[u8],
    source_path: &str,
    source_commit: &str,
) -> Result<VerificationReport, String> {
    let root = parse_strict(raw)?;
    let (schema_id, schema_version, cases) = validate_document(&root)?;
    let digest = Sha256::digest(raw);
    let source_sha256 = digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let scenario_id = text(field(&root, "scenario_id", "root")?, "scenario_id")?.to_owned();
    Ok(VerificationReport {
        status: "VERIFIED_SYNTHETIC_CONTRACT_ONLY",
        schema_id,
        schema_version,
        scenario_id,
        source_path: source_path.to_owned(),
        source_sha256,
        source_commit_binding: source_commit.to_owned(),
        verifier_version: env!("CARGO_PKG_VERSION"),
        execution_performed: false,
        physical_validation: "None",
        external_approval: "None",
        fixtures_checked: cases.len(),
        cases,
        claim_ceiling: vec![
            "Contract and arithmetic consistency for the exact synthetic fixture bytes only.",
            "No physical fidelity, engineering suitability, safety, certification, external approval, or physical-operation authority.",
        ],
    })
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let path = args
        .next()
        .unwrap_or_else(|| DEFAULT_SCENARIO_PATH.to_owned());
    if args.next().is_some() {
        return Err("usage: maritime-fixture-verifier [scenario-json-path]".to_owned());
    }
    let file_path = Path::new(&path);
    let metadata = fs::metadata(file_path)
        .map_err(|error| format!("cannot stat {}: {error}", file_path.display()))?;
    if metadata.len() > MAX_INPUT_BYTES as u64 {
        return Err(format!(
            "input exceeds the {}-byte safety bound",
            MAX_INPUT_BYTES
        ));
    }
    let raw = fs::read(file_path)
        .map_err(|error| format!("cannot read {}: {error}", file_path.display()))?;
    let source_commit =
        env::var("MARITIME_SOURCE_COMMIT").unwrap_or_else(|_| "UNBOUND_LOCAL_SOURCE".to_owned());
    let report = verify_bytes(&raw, &path, &source_commit)?;
    let output = serde_json::to_string(&report)
        .map_err(|error| format!("cannot serialize verification report: {error}"))?;
    println!("{output}");
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("FAIL_MARITIME_FIXTURE_VERIFIER: {error}");
        std::process::exit(2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_bytes() -> Vec<u8> {
        include_bytes!("../../../../docs/tech/MARITIME_FLOATING_SITE_SCENARIO_V0_1.json").to_vec()
    }

    fn expect_rejected(mutator: impl FnOnce(String) -> String, name: &str) {
        let raw = String::from_utf8(fixture_bytes()).expect("fixture is UTF-8");
        let changed = mutator(raw);
        let result = verify_bytes(changed.as_bytes(), "mutated-fixture.json", "unit-test");
        assert!(
            result.is_err(),
            "mutated fixture should be rejected: {name}"
        );
    }

    #[test]
    fn frozen_synthetic_fixture_corpus_matches_independent_derivations() {
        let raw = fixture_bytes();
        let report = verify_bytes(&raw, "frozen-fixture.json", "unit-test")
            .expect("frozen fixture should verify");
        assert_eq!(report.fixtures_checked, 6);
        assert!(report.cases.iter().all(|case| case.expectation_matches));
        assert!(
            report
                .cases
                .iter()
                .all(|case| { case.derived_disposition == case.expected_disposition })
        );
        assert!(!report.execution_performed);
        assert_eq!(report.physical_validation, "None");
        assert_eq!(report.external_approval, "None");
        assert_eq!(report.status, "VERIFIED_SYNTHETIC_CONTRACT_ONLY");
    }

    #[test]
    fn duplicate_json_object_keys_are_rejected() {
        let result = parse_strict(br#"{"schema_id":"one","schema_id":"two"}"#);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("duplicate object key"));
    }

    #[test]
    fn corrupted_expected_disposition_is_rejected() {
        expect_rejected(
            |raw| raw.replacen("\"PassDeclaredChecksOnly\"", "\"Infeasible\"", 1),
            "corrupted expected disposition",
        );
    }

    #[test]
    fn duplicate_fixture_id_is_rejected() {
        expect_rejected(
            |raw| {
                raw.replacen(
                    "vessel-exceeds-berth-length",
                    "baseline-declared-envelope-and-energy",
                    1,
                )
            },
            "duplicate fixture identifier",
        );
    }

    #[test]
    fn unknown_unit_is_rejected() {
        expect_rejected(
            |raw| raw.replacen("\"unit\": \"MWh\"", "\"unit\": \"XYZ\"", 1),
            "unknown unit",
        );
    }

    #[test]
    fn impossible_charging_efficiency_is_rejected() {
        expect_rejected(
            |raw| {
                raw.replacen(
                    "\"charging_efficiency\": 0.9",
                    "\"charging_efficiency\": 1.5",
                    1,
                )
            },
            "efficiency outside [0, 1]",
        );
    }

    #[test]
    fn stale_configuration_cannot_claim_previous_result_current() {
        expect_rejected(
            |raw| {
                raw.replacen(
                    "\"prior_result_may_be_carried_forward\": false",
                    "\"prior_result_may_be_carried_forward\": true",
                    1,
                )
            },
            "stale result improperly carried forward",
        );
    }

    #[test]
    fn unsupported_physics_cannot_claim_rigid_body_fallback() {
        expect_rejected(
            |raw| {
                raw.replacen(
                    "\"silent_rigid_body_approximation_allowed\": false",
                    "\"silent_rigid_body_approximation_allowed\": true",
                    1,
                )
            },
            "unsupported physics fallback",
        );
    }

    #[test]
    fn source_digest_is_bound_to_exact_raw_bytes() {
        let raw = fixture_bytes();
        let report = verify_bytes(&raw, "frozen-fixture.json", "0123456789abcdef")
            .expect("fixture should verify");
        let digest = Sha256::digest(&raw);
        let expected = digest
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        assert_eq!(report.source_sha256, expected);
        assert_eq!(report.source_commit_binding, "0123456789abcdef");
    }

    #[test]
    fn envelope_boundary_is_inclusive_and_does_not_use_tolerance() {
        assert!(envelope_fits(100.0, 12.0, 3.0, 100.0, 18.0, 3.0));
        assert!(!envelope_fits(100.000_001, 12.0, 3.0, 100.0, 18.0, 3.0));
    }

    #[test]
    fn input_size_limit_fails_closed() {
        let raw = vec![b' '; MAX_INPUT_BYTES + 1];
        assert!(parse_strict(&raw).is_err());
    }

    #[test]
    fn unknown_schema_fields_are_rejected() {
        expect_rejected(
            |raw| {
                raw.replacen(
                    "\"schema_version\": \"0.1.0\",",
                    "\"schema_version\": \"0.1.0\",\n  \"unknown_field\": true,",
                    1,
                )
            },
            "unknown top-level field",
        );
    }

    #[test]
    fn non_finite_json_numbers_are_rejected() {
        expect_rejected(
            |raw| {
                raw.replacen(
                    "\"charging_efficiency\": 0.9",
                    "\"charging_efficiency\": NaN",
                    1,
                )
            },
            "non-finite numeric input",
        );
    }

    #[test]
    fn trailing_json_payload_is_rejected() {
        let mut raw = fixture_bytes();
        raw.extend_from_slice(b"\n{}");
        assert!(parse_strict(&raw).is_err());
    }
}
