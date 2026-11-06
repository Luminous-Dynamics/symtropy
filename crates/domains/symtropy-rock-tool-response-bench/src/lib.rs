//! Semantic conformance checks for mechanism-resolved rock/tool response bundles.
//!
//! This crate validates identity, references, method discriminators, signal
//! metadata and reproducible kinematic fields. It does not authenticate raw
//! artifact bytes, validate physical sensor calibration, infer fracture
//! mechanisms, predict excavation performance, rank methods or control hardware.

#![forbid(unsafe_code)]

use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum ValidationCode {
    RootMustBeObject,
    UnsupportedSchemaVersion,
    MissingBundleId,
    EmptyRecords,
    MissingRecordField,
    DuplicateRecordId,
    DuplicateSpecimenId,
    UnsupportedMechanism,
    MechanismConfigurationMismatch,
    UnknownConfigurationField,
    InvalidNumber,
    InvalidDigest,
    InvalidEvidenceRole,
    DuplicateChannelId,
    DuplicateArtifactId,
    MissingCalibration,
    InvalidUncertainty,
    InvalidSignalRate,
    BandwidthExceedsNyquist,
    MissingClockDomain,
    MissingSynchronizationUncertainty,
    MissingAntiAliasFilter,
    MissingArtifactReference,
    ArtifactChannelMismatch,
    ArtifactClockMismatch,
    InvalidArtifactRole,
    MissingDiagnosticChannel,
    NonIndependentRequiredChannel,
    MissingObservationSource,
    MissingDerivedLineage,
    ArtifactParentMissing,
    ArtifactLineageCycle,
    OdcSpeedRatioMismatch,
    LaserEnergyBoundaryMismatch,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct ValidationIssue {
    pub record_id: Option<String>,
    pub code: ValidationCode,
    pub detail: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ValidationReport {
    pub issues: Vec<ValidationIssue>,
}

impl ValidationReport {
    pub fn is_valid(&self) -> bool {
        self.issues.is_empty()
    }

    pub fn issue_count(&self, code: ValidationCode) -> usize {
        self.issues.iter().filter(|issue| issue.code == code).count()
    }
}

/// Parse a JSON bundle and run semantic validation.
pub fn validate_bundle_json(source: &str) -> Result<ValidationReport, serde_json::Error> {
    let value: Value = serde_json::from_str(source)?;
    Ok(validate_bundle(&value))
}

/// Validate cross-field and cross-reference semantics after JSON parsing.
///
/// Full JSON Schema 2020-12 validation remains a separate B0 gate. SHA-256
/// digests are format-checked here; verifying a digest against artifact bytes
/// belongs to the artifact-ingestion boundary, where those bytes are present.
pub fn validate_bundle(bundle: &Value) -> ValidationReport {
    let mut issues = Vec::new();
    if !bundle.is_object() {
        push(&mut issues, None, ValidationCode::RootMustBeObject, "bundle root must be an object");
        return finish(issues);
    }

    if bundle.get("schema_version").and_then(Value::as_u64) != Some(1) {
        push(&mut issues, None, ValidationCode::UnsupportedSchemaVersion, "schema_version must be 1");
    }
    if non_empty_string(bundle, "bundle_id").is_none() {
        push(&mut issues, None, ValidationCode::MissingBundleId, "bundle_id must be a non-empty string");
    }
    let Some(records) = bundle.get("records").and_then(Value::as_array) else {
        push(&mut issues, None, ValidationCode::EmptyRecords, "records must be a non-empty array");
        return finish(issues);
    };
    if records.is_empty() {
        push(&mut issues, None, ValidationCode::EmptyRecords, "records must contain at least one record");
        return finish(issues);
    }

    let mut record_ids = BTreeSet::new();
    let mut specimen_ids = BTreeSet::new();
    for record in records {
        let record_id = non_empty_string(record, "record_id").map(ToOwned::to_owned);
        if let Some(id) = &record_id {
            if !record_ids.insert(id.clone()) {
                push(&mut issues, record_id.clone(), ValidationCode::DuplicateRecordId, "record_id is not unique within the bundle");
            }
        } else {
            push(&mut issues, None, ValidationCode::MissingRecordField, "record_id must be a non-empty string");
        }

        validate_record(record, record_id.clone(), &mut specimen_ids, &mut issues);
    }

    finish(issues)
}

fn validate_record(
    record: &Value,
    record_id: Option<String>,
    specimen_ids: &mut BTreeSet<String>,
    issues: &mut Vec<ValidationIssue>,
) {
    for field in ["mechanism", "protocol", "material", "machine", "method_configuration", "environment", "channels", "artifacts", "observations", "quality"] {
        if record.get(field).is_none() {
            push(issues, record_id.clone(), ValidationCode::MissingRecordField, format!("missing required record field: {field}"));
        }
    }

    if let Some(material) = record.get("material") {
        for field in ["cohort_ref", "specimen_ref", "lithology"] {
            if non_empty_string(material, field).is_none() {
                push(issues, record_id.clone(), ValidationCode::MissingRecordField, format!("material.{field} must be non-empty"));
            }
        }
        if let Some(specimen) = non_empty_string(material, "specimen_ref") {
            if !specimen_ids.insert(specimen.to_owned()) {
                push(issues, record_id.clone(), ValidationCode::DuplicateSpecimenId, "physical specimen_ref is reused within this bundle");
            }
        }
    }

    if let Some(protocol) = record.get("protocol") {
        for field in ["protocol_id", "revision", "run_id"] {
            if non_empty_string(protocol, field).is_none() {
                push(issues, record_id.clone(), ValidationCode::MissingRecordField, format!("protocol.{field} must be non-empty"));
            }
        }
        validate_digest(protocol, "digest_sha256", record_id.clone(), "protocol.digest_sha256", issues);
    }
    if let Some(machine) = record.get("machine") {
        for field in ["machine_profile_ref", "tool_profile_ref"] {
            if non_empty_string(machine, field).is_none() {
                push(issues, record_id.clone(), ValidationCode::MissingRecordField, format!("machine.{field} must be non-empty"));
            }
        }
        validate_digest(machine, "configuration_digest_sha256", record_id.clone(), "machine.configuration_digest_sha256", issues);
    }
    if let Some(environment) = record.get("environment") {
        require_positive(environment, "temperature_k", record_id.clone(), "environment.temperature_k", issues);
        require_non_negative(environment, "confining_pressure_pa", record_id.clone(), "environment.confining_pressure_pa", issues);
    }

    let mechanism = non_empty_string(record, "mechanism");
    let config = record.get("method_configuration");
    if let (Some(mechanism), Some(config)) = (mechanism, config) {
        validate_method_config(mechanism, config, record_id.clone(), issues);
    } else {
        push(issues, record_id.clone(), ValidationCode::UnsupportedMechanism, "mechanism or method_configuration is missing");
    }

    let channels = record.get("channels").and_then(Value::as_array);
    let artifacts = record.get("artifacts").and_then(Value::as_array);
    let observations = record.get("observations").and_then(Value::as_array);

    let mut channel_ids = BTreeSet::new();
    let mut artifact_map: BTreeMap<String, &Value> = BTreeMap::new();

    if let Some(artifacts) = artifacts {
        for artifact in artifacts {
            let Some(id) = non_empty_string(artifact, "artifact_id") else {
                push(issues, record_id.clone(), ValidationCode::MissingRecordField, "artifact_id must be non-empty");
                continue;
            };
            if artifact_map.insert(id.to_owned(), artifact).is_some() {
                push(issues, record_id.clone(), ValidationCode::DuplicateArtifactId, format!("duplicate artifact_id: {id}"));
            }
            validate_digest(artifact, "sha256", record_id.clone(), &format!("artifact {id}.sha256"), issues);
            if non_empty_string(artifact, "artifact_role").is_none() {
                push(issues, record_id.clone(), ValidationCode::InvalidArtifactRole, format!("artifact {id} has no artifact_role"));
            }
            if !artifact.get("channel_refs").and_then(Value::as_array).is_some() {
                push(issues, record_id.clone(), ValidationCode::ArtifactChannelMismatch, format!("artifact {id} must declare channel_refs"));
            }
        }
    } else {
        push(issues, record_id.clone(), ValidationCode::MissingArtifactReference, "artifacts must be an array");
    }

    if let Some(channels) = channels {
        for channel in channels {
            let Some(id) = non_empty_string(channel, "channel_id") else {
                push(issues, record_id.clone(), ValidationCode::MissingRecordField, "channel_id must be non-empty");
                continue;
            };
            if !channel_ids.insert(id.to_owned()) {
                push(issues, record_id.clone(), ValidationCode::DuplicateChannelId, format!("duplicate channel_id: {id}"));
            }

            for field in ["quantity", "unit", "sensor_ref", "measurement_location"] {
                if non_empty_string(channel, field).is_none() {
                    push(issues, record_id.clone(), ValidationCode::MissingRecordField, format!("channel {id}.{field} must be non-empty"));
                }
            }
            if non_empty_string(channel, "calibration_ref").is_none() {
                push(issues, record_id.clone(), ValidationCode::MissingCalibration, format!("channel {id} lacks calibration_ref"));
            }
            require_non_negative(channel, "standard_uncertainty", record_id.clone(), &format!("channel {id}.standard_uncertainty"), issues);

            let role = non_empty_string(channel, "measurement_role");
            if !valid_role(role) {
                push(issues, record_id.clone(), ValidationCode::InvalidEvidenceRole, format!("channel {id} has an unsupported measurement_role"));
            } else if role == Some("DerivedFromIndependentMeasurements") && non_empty_string(channel, "lineage_ref").is_none() {
                push(issues, record_id.clone(), ValidationCode::MissingDerivedLineage, format!("derived channel {id} requires lineage_ref"));
            }

            let sample_rate = require_positive(channel, "sample_rate_hz", record_id.clone(), &format!("channel {id}.sample_rate_hz"), issues);
            let bandwidth = require_positive(channel, "bandwidth_hz", record_id.clone(), &format!("channel {id}.bandwidth_hz"), issues);
            if let (Some(sample_rate), Some(bandwidth)) = (sample_rate, bandwidth) {
                if bandwidth > sample_rate / 2.0 {
                    push(issues, record_id.clone(), ValidationCode::BandwidthExceedsNyquist, format!("channel {id} bandwidth exceeds sample_rate_hz / 2"));
                }
            }
            if non_empty_string(channel, "clock_domain_ref").is_none() {
                push(issues, record_id.clone(), ValidationCode::MissingClockDomain, format!("channel {id} lacks clock_domain_ref"));
            }
            if number(channel, "synchronization_uncertainty_ns").is_none_or(|n| n < 0.0) {
                push(issues, record_id.clone(), ValidationCode::MissingSynchronizationUncertainty, format!("channel {id} requires finite non-negative synchronization_uncertainty_ns"));
            }
            if non_empty_string(channel, "anti_alias_filter_ref").is_none() {
                push(issues, record_id.clone(), ValidationCode::MissingAntiAliasFilter, format!("channel {id} requires anti_alias_filter_ref"));
            }

            let Some(raw_ref) = non_empty_string(channel, "raw_artifact_ref") else {
                push(issues, record_id.clone(), ValidationCode::MissingArtifactReference, format!("channel {id} lacks raw_artifact_ref"));
                continue;
            };
            let Some(artifact) = artifact_map.get(raw_ref) else {
                push(issues, record_id.clone(), ValidationCode::MissingArtifactReference, format!("channel {id} references missing artifact {raw_ref}"));
                continue;
            };
            if !string_array_contains(artifact, "channel_refs", id) {
                push(issues, record_id.clone(), ValidationCode::ArtifactChannelMismatch, format!("artifact {raw_ref} does not reference channel {id}"));
            }
            if non_empty_string(channel, "clock_domain_ref") != non_empty_string(artifact, "clock_domain_ref") {
                push(issues, record_id.clone(), ValidationCode::ArtifactClockMismatch, format!("channel {id} and raw artifact {raw_ref} have different clock domains"));
            }
            let role = non_empty_string(artifact, "artifact_role");
            if !matches!(role, Some("RawSignal" | "ThermalImage" | "HighSpeedImage")) {
                push(issues, record_id.clone(), ValidationCode::InvalidArtifactRole, format!("artifact {raw_ref} is not a raw time-series/image artifact"));
            }
        }
    } else {
        push(issues, record_id.clone(), ValidationCode::MissingRecordField, "channels must be an array");
    }

    if let Some(artifacts) = artifacts {
        for artifact in artifacts {
            let Some(id) = non_empty_string(artifact, "artifact_id") else { continue };
            if let Some(parents) = artifact.get("parent_artifact_refs").and_then(Value::as_array) {
                for parent in parents.iter().filter_map(Value::as_str) {
                    if !artifact_map.contains_key(parent) {
                        push(issues, record_id.clone(), ValidationCode::ArtifactParentMissing, format!("artifact {id} references missing parent {parent}"));
                    }
                    if parent == id {
                        push(issues, record_id.clone(), ValidationCode::ArtifactLineageCycle, format!("artifact {id} references itself as parent"));
                    }
                }
            }
            if let Some(refs) = artifact.get("channel_refs").and_then(Value::as_array) {
                for ref_id in refs.iter().filter_map(Value::as_str) {
                    if !channel_ids.contains(ref_id) {
                        push(issues, record_id.clone(), ValidationCode::ArtifactChannelMismatch, format!("artifact {id} references unknown channel {ref_id}"));
                    }
                }
            }
        }
        if has_artifact_cycle(&artifact_map) {
            push(issues, record_id.clone(), ValidationCode::ArtifactLineageCycle, "artifact parent graph contains a cycle");
        }
    }

    if mechanism == Some("ATCP") {
        if let Some(config) = config {
            for field in ["axial_waveform_artifact_ref", "torsional_waveform_artifact_ref"] {
                if let Some(reference) = non_empty_string(config, field) {
                    if !artifact_map.contains_key(reference) {
                        push(issues, record_id.clone(), ValidationCode::MissingArtifactReference, format!("method_configuration.{field} references missing artifact {reference}"));
                    }
                }
            }
        }
    }

    if let Some(observations) = observations {
        for observation in observations {
            let Some(metric_id) = non_empty_string(observation, "metric_id") else {
                push(issues, record_id.clone(), ValidationCode::MissingRecordField, "observation metric_id must be non-empty");
                continue;
            };
            if number(observation, "value").is_none() {
                push(issues, record_id.clone(), ValidationCode::InvalidNumber, format!("observation {metric_id} value must be finite numeric"));
            }
            require_non_negative(observation, "standard_uncertainty", record_id.clone(), &format!("observation {metric_id}.standard_uncertainty"), issues);
            if non_empty_string(observation, "unit").is_none() || non_empty_string(observation, "calibration_ref").is_none() {
                push(issues, record_id.clone(), ValidationCode::MissingRecordField, format!("observation {metric_id} needs unit and calibration_ref"));
            }
            let role = non_empty_string(observation, "measurement_role");
            if !valid_role(role) {
                push(issues, record_id.clone(), ValidationCode::InvalidEvidenceRole, format!("observation {metric_id} has an unsupported role"));
            }
            if role == Some("DerivedFromIndependentMeasurements")
                && (non_empty_string(observation, "lineage_ref").is_none() || non_empty_string(observation, "method_ref").is_none())
            {
                push(issues, record_id.clone(), ValidationCode::MissingDerivedLineage, format!("derived observation {metric_id} needs lineage_ref and method_ref"));
            }
            let sources = observation.get("source_artifact_refs").and_then(Value::as_array);
            if sources.is_none_or(|array| array.is_empty()) {
                push(issues, record_id.clone(), ValidationCode::MissingObservationSource, format!("observation {metric_id} needs source_artifact_refs"));
            } else if let Some(sources) = sources {
                for source in sources.iter().filter_map(Value::as_str) {
                    if !artifact_map.contains_key(source) {
                        push(issues, record_id.clone(), ValidationCode::MissingObservationSource, format!("observation {metric_id} references missing artifact {source}"));
                    }
                }
            }
        }
    } else {
        push(issues, record_id.clone(), ValidationCode::MissingObservationSource, "observations must be an array");
    }

    if let (Some(mechanism), Some(channels)) = (mechanism, channels) {
        validate_diagnostic_channels(mechanism, channels, record_id.clone(), issues);
    }
}

fn validate_method_config(
    mechanism: &str,
    config: &Value,
    record_id: Option<String>,
    issues: &mut Vec<ValidationIssue>,
) {
    let kind = non_empty_string(config, "kind");
    if kind != Some(mechanism) {
        push(issues, record_id.clone(), ValidationCode::MechanismConfigurationMismatch, "mechanism and method_configuration.kind must match");
    }

    let (required, positive, allowed): (&[&str], &[&str], &[&str]) = match mechanism {
        "ATCP" => (
            &["axial_waveform_artifact_ref", "torsional_waveform_artifact_ref", "phase_offset_rad", "axial_excitation_frequency_hz", "torsional_excitation_frequency_hz", "rotation_speed_rad_s", "weight_on_bit_n"],
            &["axial_excitation_frequency_hz", "torsional_excitation_frequency_hz", "rotation_speed_rad_s", "weight_on_bit_n"],
            &["kind", "axial_waveform_artifact_ref", "torsional_waveform_artifact_ref", "phase_offset_rad", "axial_excitation_frequency_hz", "torsional_excitation_frequency_hz", "axial_impulse_energy_j", "torsional_impulse_energy_j", "rotation_speed_rad_s", "weight_on_bit_n", "fluid_pressure_pa", "flow_rate_m3_s"],
        ),
        "LASER_ASSISTED_PDC" => (
            &["wavelength_m", "source_electrical_input_j", "workpiece_incident_optical_energy_j", "spot_major_axis_m", "spot_minor_axis_m", "incidence_angle_rad", "beam_to_cutter_offset_m", "preheat_to_cut_delay_s", "back_rake_rad", "side_rake_rad", "cutter_spacing_m", "depth_of_cut_m", "cutting_speed_m_s", "laser_state"],
            &["wavelength_m", "spot_major_axis_m", "spot_minor_axis_m", "cutter_spacing_m", "depth_of_cut_m", "cutting_speed_m_s"],
            &["kind", "wavelength_m", "source_electrical_input_j", "optical_output_energy_j", "workpiece_incident_optical_energy_j", "absorbed_energy_estimate_j", "spot_major_axis_m", "spot_minor_axis_m", "incidence_angle_rad", "standoff_m", "beam_to_cutter_offset_m", "preheat_to_cut_delay_s", "back_rake_rad", "side_rake_rad", "cutter_spacing_m", "depth_of_cut_m", "cutting_speed_m_s", "laser_state"],
        ),
        "ODC" => (
            &["disc_diameter_m", "eccentricity_m", "angular_speed_rad_s", "traction_speed_m_s", "depth_of_cut_m", "speed_ratio", "speed_ratio_definition"],
            &["disc_diameter_m", "eccentricity_m", "angular_speed_rad_s", "traction_speed_m_s", "depth_of_cut_m", "speed_ratio"],
            &["kind", "disc_diameter_m", "eccentricity_m", "angular_speed_rad_s", "traction_speed_m_s", "depth_of_cut_m", "speed_ratio", "speed_ratio_definition", "cutter_edge_profile_ref"],
        ),
        _ => {
            push(issues, record_id, ValidationCode::UnsupportedMechanism, format!("unsupported mechanism: {mechanism}"));
            return;
        }
    };

    for field in required {
        let valid = if matches!(*field, "laser_state" | "speed_ratio_definition" | "axial_waveform_artifact_ref" | "torsional_waveform_artifact_ref") {
            non_empty_string(config, field).is_some()
        } else {
            number(config, field).is_some()
        };
        if !valid {
            push(issues, record_id.clone(), ValidationCode::InvalidNumber, format!("method_configuration.{field} is missing or invalid"));
        }
    }
    for field in positive {
        require_positive(config, field, record_id.clone(), &format!("method_configuration.{field}"), issues);
    }
    let allowed: BTreeSet<&str> = allowed.iter().copied().collect();
    if let Some(object) = config.as_object() {
        for key in object.keys() {
            if !allowed.contains(key.as_str()) {
                push(issues, record_id.clone(), ValidationCode::UnknownConfigurationField, format!("unknown {mechanism} configuration field: {key}"));
            }
        }
    }

    if mechanism == "LASER_ASSISTED_PDC" {
        for field in ["source_electrical_input_j", "optical_output_energy_j", "workpiece_incident_optical_energy_j", "absorbed_energy_estimate_j", "standoff_m", "preheat_to_cut_delay_s"] {
            if config.get(field).is_some() {
                require_non_negative(config, field, record_id.clone(), &format!("method_configuration.{field}"), issues);
            }
        }
        let source = number(config, "source_electrical_input_j");
        let optical = number(config, "optical_output_energy_j");
        let incident = number(config, "workpiece_incident_optical_energy_j");
        let absorbed = number(config, "absorbed_energy_estimate_j");
        let arm_valid = match non_empty_string(config, "laser_state") {
            Some("ON") => source.is_some_and(|v| v > 0.0) && incident.is_some_and(|v| v > 0.0),
            Some("OFF_SHAM") => source.is_some_and(|v| v > 0.0)
                && incident == Some(0.0)
                && absorbed.is_none_or(|v| v == 0.0),
            Some("OFF_BASELINE") => source == Some(0.0)
                && optical.is_none_or(|v| v == 0.0)
                && incident == Some(0.0)
                && absorbed.is_none_or(|v| v == 0.0),
            _ => false,
        };
        if !arm_valid {
            push(issues, record_id.clone(), ValidationCode::LaserEnergyBoundaryMismatch, "laser ON, OFF_SHAM and OFF_BASELINE energy fields do not match the declared arm");
        }
    }

    if mechanism == "ODC" {
        if non_empty_string(config, "speed_ratio_definition") != Some("v/(omega*e)") {
            push(issues, record_id.clone(), ValidationCode::OdcSpeedRatioMismatch, "ODC speed_ratio_definition must be v/(omega*e)");
        }
        let v = number(config, "traction_speed_m_s");
        let omega = number(config, "angular_speed_rad_s");
        let e = number(config, "eccentricity_m");
        let stated = number(config, "speed_ratio");
        if let (Some(v), Some(omega), Some(e), Some(stated)) = (v, omega, e, stated) {
            let recomputed = v / (omega * e);
            if !recomputed.is_finite() || (recomputed - stated).abs() > 1.0e-9 * recomputed.abs().max(1.0) {
                push(issues, record_id, ValidationCode::OdcSpeedRatioMismatch, format!("stored ODC speed_ratio {stated} does not match recomputed value {recomputed}"));
            }
        }
    }
}

fn validate_diagnostic_channels(
    mechanism: &str,
    channels: &[Value],
    record_id: Option<String>,
    issues: &mut Vec<ValidationIssue>,
) {
    let required: &[&str] = match mechanism {
        "ATCP" => &["axial_force", "torsional_torque"],
        "LASER_ASSISTED_PDC" => &["optical_power", "surface_temperature"],
        "ODC" => &["thrust_force", "cutting_torque"],
        _ => return,
    };
    let mut found = Vec::new();
    for quantity in required {
        let channel = channels.iter().find(|channel| non_empty_string(channel, "quantity") == Some(*quantity));
        let Some(channel) = channel else {
            push(issues, record_id.clone(), ValidationCode::MissingDiagnosticChannel, format!("{mechanism} requires diagnostic channel {quantity}"));
            continue;
        };
        if non_empty_string(channel, "measurement_role") != Some("IndependentMeasurement") {
            push(issues, record_id.clone(), ValidationCode::NonIndependentRequiredChannel, format!("{mechanism} diagnostic channel {quantity} must be an IndependentMeasurement"));
        }
        found.push(non_empty_string(channel, "clock_domain_ref").unwrap_or_default());
    }
    if found.len() == required.len() && found.windows(2).any(|window| window[0] != window[1]) {
        push(issues, record_id, ValidationCode::MissingClockDomain, format!("{mechanism} coupled diagnostic channels must share a clock domain in v0.1"));
    }
}

fn has_artifact_cycle(artifacts: &BTreeMap<String, &Value>) -> bool {
    fn visit(
        id: &str,
        artifacts: &BTreeMap<String, &Value>,
        visiting: &mut BTreeSet<String>,
        visited: &mut BTreeSet<String>,
    ) -> bool {
        if visited.contains(id) {
            return false;
        }
        if !visiting.insert(id.to_owned()) {
            return true;
        }
        if let Some(parents) = artifacts.get(id).and_then(|value| value.get("parent_artifact_refs")).and_then(Value::as_array) {
            for parent in parents.iter().filter_map(Value::as_str) {
                if artifacts.contains_key(parent) && visit(parent, artifacts, visiting, visited) {
                    return true;
                }
            }
        }
        visiting.remove(id);
        visited.insert(id.to_owned());
        false
    }

    let mut visiting = BTreeSet::new();
    let mut visited = BTreeSet::new();
    artifacts.keys().any(|id| visit(id, artifacts, &mut visiting, &mut visited))
}

fn valid_role(role: Option<&str>) -> bool {
    matches!(role, Some("IndependentMeasurement" | "DerivedFromIndependentMeasurements" | "ProcessSensor" | "OperatorReported" | "ModelEstimate" | "Simulation"))
}

fn validate_digest(
    object: &Value,
    field: &str,
    record_id: Option<String>,
    label: &str,
    issues: &mut Vec<ValidationIssue>,
) {
    let valid = non_empty_string(object, field)
        .is_some_and(|value| value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit()));
    if !valid {
        push(issues, record_id, ValidationCode::InvalidDigest, format!("{label} must be 64 hexadecimal characters"));
    }
}

fn require_positive(
    object: &Value,
    field: &str,
    record_id: Option<String>,
    label: &str,
    issues: &mut Vec<ValidationIssue>,
) -> Option<f64> {
    match number(object, field) {
        Some(value) if value > 0.0 => Some(value),
        _ => {
            push(issues, record_id, ValidationCode::InvalidNumber, format!("{label} must be finite and > 0"));
            None
        }
    }
}

fn require_non_negative(
    object: &Value,
    field: &str,
    record_id: Option<String>,
    label: &str,
    issues: &mut Vec<ValidationIssue>,
) {
    match number(object, field) {
        Some(value) if value >= 0.0 => {}
        _ => push(issues, record_id, ValidationCode::InvalidUncertainty, format!("{label} must be finite and >= 0")),
    }
}

fn number(object: &Value, field: &str) -> Option<f64> {
    object.get(field).and_then(Value::as_f64).filter(|value| value.is_finite())
}

fn non_empty_string<'a>(object: &'a Value, field: &str) -> Option<&'a str> {
    object.get(field).and_then(Value::as_str).map(str::trim).filter(|value| !value.is_empty())
}

fn string_array_contains(object: &Value, field: &str, value: &str) -> bool {
    object.get(field).and_then(Value::as_array).is_some_and(|array| array.iter().any(|item| item.as_str() == Some(value)))
}

fn push(
    issues: &mut Vec<ValidationIssue>,
    record_id: Option<String>,
    code: ValidationCode,
    detail: impl Into<String>,
) {
    issues.push(ValidationIssue {
        record_id,
        code,
        detail: detail.into(),
    });
}

fn finish(mut issues: Vec<ValidationIssue>) -> ValidationReport {
    issues.sort();
    issues.dedup();
    ValidationReport { issues }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn fixture() -> Value {
        serde_json::from_str(include_str!("../../../../docs/research/rock-extraction-portfolio/fixtures/synthetic-rock-tool-response.v0_1.json"))
            .expect("frozen synthetic fixture must parse")
    }

    fn has(report: &ValidationReport, code: ValidationCode) -> bool {
        report.issues.iter().any(|issue| issue.code == code)
    }

    fn record_by_mechanism<'a>(bundle: &'a mut Value, mechanism: &str) -> &'a mut Value {
        bundle["records"]
            .as_array_mut()
            .expect("records array")
            .iter_mut()
            .find(|record| record["mechanism"] == mechanism)
            .expect("fixture mechanism")
    }

    #[test]
    fn frozen_three_method_fixture_is_valid() {
        let report = validate_bundle(&fixture());
        assert!(report.is_valid(), "{:#?}", report.issues);
    }

    #[test]
    fn unknown_schema_version_is_rejected() {
        let mut data = fixture();
        data["schema_version"] = json!(2);
        assert!(has(&validate_bundle(&data), ValidationCode::UnsupportedSchemaVersion));
    }

    #[test]
    fn duplicate_record_identity_is_rejected() {
        let mut data = fixture();
        let duplicate_id = data["records"][0]["record_id"].clone();
        data["records"][1]["record_id"] = duplicate_id;
        assert!(has(&validate_bundle(&data), ValidationCode::DuplicateRecordId));
    }

    #[test]
    fn reused_physical_specimen_is_rejected() {
        let mut data = fixture();
        let reused_specimen = data["records"][0]["material"]["specimen_ref"].clone();
        data["records"][1]["material"]["specimen_ref"] = reused_specimen;
        assert!(has(&validate_bundle(&data), ValidationCode::DuplicateSpecimenId));
    }

    #[test]
    fn mechanism_configuration_mismatch_is_rejected() {
        let mut data = fixture();
        data["records"][0]["method_configuration"]["kind"] = json!("ODC");
        assert!(has(&validate_bundle(&data), ValidationCode::MechanismConfigurationMismatch));
    }

    #[test]
    fn unknown_method_configuration_field_is_rejected() {
        let mut data = fixture();
        data["records"][0]["method_configuration"]["unreviewed_knob"] = json!(1.0);
        assert!(has(&validate_bundle(&data), ValidationCode::UnknownConfigurationField));
    }

    #[test]
    fn invalid_digest_is_rejected() {
        let mut data = fixture();
        data["records"][0]["artifacts"][0]["sha256"] = json!("not-a-digest");
        assert!(has(&validate_bundle(&data), ValidationCode::InvalidDigest));
    }

    #[test]
    fn dangling_channel_artifact_reference_is_rejected() {
        let mut data = fixture();
        data["records"][0]["channels"][0]["raw_artifact_ref"] = json!("missing-artifact");
        assert!(has(&validate_bundle(&data), ValidationCode::MissingArtifactReference));
    }

    #[test]
    fn atcp_waveform_artifacts_must_resolve() {
        let mut data = fixture();
        data["records"][0]["method_configuration"]["torsional_waveform_artifact_ref"] = json!("MISSING-TORSIONAL-WAVEFORM");
        assert!(has(&validate_bundle(&data), ValidationCode::MissingArtifactReference));
    }

    #[test]
    fn artifact_must_reference_its_channels() {
        let mut data = fixture();
        data["records"][0]["artifacts"][0]["channel_refs"] = json!([]);
        assert!(has(&validate_bundle(&data), ValidationCode::ArtifactChannelMismatch));
    }

    #[test]
    fn mismatched_clock_domains_are_rejected() {
        let mut data = fixture();
        data["records"][0]["channels"][0]["clock_domain_ref"] = json!("OTHER-CLOCK");
        assert!(has(&validate_bundle(&data), ValidationCode::ArtifactClockMismatch));
    }

    #[test]
    fn missing_synchronization_uncertainty_is_rejected() {
        let mut data = fixture();
        data["records"][0]["channels"][0].as_object_mut().unwrap().remove("synchronization_uncertainty_ns");
        assert!(has(&validate_bundle(&data), ValidationCode::MissingSynchronizationUncertainty));
    }

    #[test]
    fn missing_anti_alias_filter_is_rejected() {
        let mut data = fixture();
        data["records"][0]["channels"][0].as_object_mut().unwrap().remove("anti_alias_filter_ref");
        assert!(has(&validate_bundle(&data), ValidationCode::MissingAntiAliasFilter));
    }

    #[test]
    fn bandwidth_above_nyquist_is_rejected() {
        let mut data = fixture();
        data["records"][0]["channels"][0]["bandwidth_hz"] = json!(11000.0);
        assert!(has(&validate_bundle(&data), ValidationCode::BandwidthExceedsNyquist));
    }

    #[test]
    fn missing_calibration_is_rejected() {
        let mut data = fixture();
        data["records"][0]["channels"][0].as_object_mut().unwrap().remove("calibration_ref");
        assert!(has(&validate_bundle(&data), ValidationCode::MissingCalibration));
    }

    #[test]
    fn simulated_diagnostic_channel_is_not_independent_measurement() {
        let mut data = fixture();
        data["records"][0]["channels"][0]["measurement_role"] = json!("Simulation");
        assert!(has(&validate_bundle(&data), ValidationCode::NonIndependentRequiredChannel));
    }

    #[test]
    fn missing_required_mechanism_channel_is_rejected() {
        let mut data = fixture();
        let channels = data["records"][0]["channels"].as_array_mut().unwrap();
        channels.retain(|channel| channel["quantity"] != "torsional_torque");
        assert!(has(&validate_bundle(&data), ValidationCode::MissingDiagnosticChannel));
    }

    #[test]
    fn derived_observation_requires_lineage_and_method() {
        let mut data = fixture();
        data["records"][2]["observations"][0].as_object_mut().unwrap().remove("lineage_ref");
        assert!(has(&validate_bundle(&data), ValidationCode::MissingDerivedLineage));
    }

    #[test]
    fn observation_source_must_resolve() {
        let mut data = fixture();
        data["records"][0]["observations"][0]["source_artifact_refs"] = json!(["unknown"]);
        assert!(has(&validate_bundle(&data), ValidationCode::MissingObservationSource));
    }

    #[test]
    fn artifact_parent_must_resolve() {
        let mut data = fixture();
        data["records"][0]["artifacts"][0]["parent_artifact_refs"] = json!(["unknown"]);
        assert!(has(&validate_bundle(&data), ValidationCode::ArtifactParentMissing));
    }

    #[test]
    fn artifact_lineage_cycle_is_rejected() {
        let mut data = fixture();
        data["records"][0]["artifacts"][0]["parent_artifact_refs"] = json!(["SYN-ART-ATCP-RAW"]);
        assert!(has(&validate_bundle(&data), ValidationCode::ArtifactLineageCycle));
    }

    #[test]
    fn laser_off_baseline_is_a_valid_mechanical_control() {
        let mut data = fixture();
        let config = &mut data["records"][1]["method_configuration"];
        config["laser_state"] = json!("OFF_BASELINE");
        config["source_electrical_input_j"] = json!(0.0);
        config["optical_output_energy_j"] = json!(0.0);
        config["workpiece_incident_optical_energy_j"] = json!(0.0);
        config["absorbed_energy_estimate_j"] = json!(0.0);
        assert!(validate_bundle(&data).is_valid());
    }

    #[test]
    fn laser_off_sham_requires_zero_workpiece_exposure() {
        let mut data = fixture();
        let config = &mut data["records"][1]["method_configuration"];
        config["laser_state"] = json!("OFF_SHAM");
        config["source_electrical_input_j"] = json!(900.0);
        config["optical_output_energy_j"] = json!(600.0);
        config["workpiece_incident_optical_energy_j"] = json!(0.0);
        config["absorbed_energy_estimate_j"] = json!(0.0);
        assert!(validate_bundle(&data).is_valid());
    }

    #[test]
    fn laser_baseline_cannot_claim_laser_energy() {
        let mut data = fixture();
        let config = &mut data["records"][1]["method_configuration"];
        config["laser_state"] = json!("OFF_BASELINE");
        config["source_electrical_input_j"] = json!(900.0);
        config["workpiece_incident_optical_energy_j"] = json!(0.0);
        assert!(has(&validate_bundle(&data), ValidationCode::LaserEnergyBoundaryMismatch));
    }

    #[test]
    fn odc_speed_ratio_is_recomputed() {
        let mut data = fixture();
        data["records"][2]["method_configuration"]["speed_ratio"] = json!(0.5);
        assert!(has(&validate_bundle(&data), ValidationCode::OdcSpeedRatioMismatch));
    }

    #[test]
    fn zero_odc_eccentricity_is_rejected() {
        let mut data = fixture();
        data["records"][2]["method_configuration"]["eccentricity_m"] = json!(0.0);
        let report = validate_bundle(&data);
        assert!(has(&report, ValidationCode::InvalidNumber));
        assert!(has(&report, ValidationCode::OdcSpeedRatioMismatch));
    }

    #[test]
    fn diagnostic_channels_must_share_clock_domain() {
        let mut data = fixture();
        data["records"][0]["channels"][1]["clock_domain_ref"] = json!("CLOCK-OTHER");
        assert!(has(&validate_bundle(&data), ValidationCode::MissingClockDomain));
    }

    #[test]
    fn duplicate_channel_identity_is_rejected() {
        let mut data = fixture();
        let duplicate = data["records"][0]["channels"][0].clone();
        data["records"][0]["channels"].as_array_mut().unwrap().push(duplicate);
        assert!(has(&validate_bundle(&data), ValidationCode::DuplicateChannelId));
    }

    #[test]
    fn duplicate_artifact_identity_is_rejected() {
        let mut data = fixture();
        let duplicate = data["records"][0]["artifacts"][0].clone();
        data["records"][0]["artifacts"].as_array_mut().unwrap().push(duplicate);
        assert!(has(&validate_bundle(&data), ValidationCode::DuplicateArtifactId));
    }

    #[test]
    fn invalid_sample_rate_is_rejected() {
        let mut data = fixture();
        data["records"][0]["channels"][0]["sample_rate_hz"] = json!(0.0);
        assert!(has(&validate_bundle(&data), ValidationCode::InvalidNumber));
    }

    #[test]
    fn unsupported_mechanism_is_rejected() {
        let mut data = fixture();
        data["records"][0]["mechanism"] = json!("ULTRASONIC_ASSISTED");
        assert!(has(&validate_bundle(&data), ValidationCode::UnsupportedMechanism));
    }

    #[test]
    fn empty_record_set_is_rejected() {
        let mut data = fixture();
        data["records"] = json!([]);
        assert!(has(&validate_bundle(&data), ValidationCode::EmptyRecords));
    }

    #[test]
    fn validation_issue_order_is_deterministic() {
        let mut data = fixture();
        data["records"][0]["channels"][0]["sample_rate_hz"] = json!(0.0);
        data["records"][0]["channels"][0].as_object_mut().unwrap().remove("calibration_ref");
        let first = validate_bundle(&data);
        let second = validate_bundle(&data);
        assert_eq!(first, second);
    }

    #[test]
    fn json_entrypoint_reports_parse_errors() {
        let error = validate_bundle_json("{not-json");
        assert!(error.is_err());
    }

    #[test]
    fn helper_finds_fixture_mechanisms() {
        let mut data = fixture();
        let odc = record_by_mechanism(&mut data, "ODC");
        assert_eq!(odc["record_id"], "SYN-ODC-001");
    }
}
