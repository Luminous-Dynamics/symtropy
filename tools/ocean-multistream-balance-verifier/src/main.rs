use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::{env, fs, path::Path};

const DEFAULT_PATH: &str =
    "docs/research/port-energy-compute/OCEAN_MULTISTREAM_BALANCE_FIXTURE_V0_1.json";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Bundle {
    schema_version: String,
    fixture_id: String,
    as_of: String,
    synthetic_only: bool,
    interval_start_utc: String,
    interval_end_utc: String,
    interval_duration_seconds: u64,
    maximum_clock_offset_ms: u64,
    maximum_absolute_residual_z_score: f64,
    claim_ceiling: String,
    evidence_states: Vec<String>,
    control_volumes: Vec<ControlVolume>,
    streams: Vec<Stream>,
    covariance_contract: CovarianceContract,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ControlVolume {
    control_volume_id: String,
    name: String,
    boundary_completeness_state: String,
    ports: Vec<Port>,
    storage_inventory: StorageInventory,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Port {
    port_id: String,
    direction: String,
    status: String,
    not_applicable_reason: Option<String>,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct StorageInventory {
    measurement_id: String,
    quantity_origin: String,
    evidence_state: String,
    interval_start_utc: String,
    interval_end_utc: String,
    signed_delta_storage_mass_kg: f64,
    independent_standard_uncertainty_kg: f64,
    common_mode_standard_uncertainty_kg: f64,
    correlation_group_id: Option<String>,
    sensor_id: Option<String>,
    calibration_id: Option<String>,
    calibration_valid_from_utc: Option<String>,
    calibration_valid_until_utc: Option<String>,
    clock_offset_ms: i64,
    quality_state: String,
    source_rights_state: String,
    raw_data_digest: String,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Endpoint {
    endpoint_type: String,
    node_id: String,
    port_id: Option<String>,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Stream {
    stream_id: String,
    stream_type: String,
    from: Endpoint,
    to: Endpoint,
    quantity_origin: String,
    mass_flow_kg_s: f64,
    independent_standard_uncertainty_kg_s: f64,
    common_mode_standard_uncertainty_kg_s: f64,
    correlation_group_id: Option<String>,
    evidence: Evidence,
    model: Option<ModelEvidence>,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Evidence {
    record_id: String,
    interval_start_utc: String,
    interval_end_utc: String,
    evidence_state: String,
    quality_state: String,
    source_rights_state: String,
    clock_offset_ms: i64,
    sensor_id: Option<String>,
    calibration_id: Option<String>,
    calibration_valid_from_utc: Option<String>,
    calibration_valid_until_utc: Option<String>,
    raw_data_digest: String,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ModelEvidence {
    method_id: String,
    model_version: String,
    input_digests: Vec<String>,
    validation_state: String,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CovarianceContract {
    rule: String,
    correlation_groups: Vec<String>,
    unknown_correlation_state: String,
}
#[derive(Debug, Serialize)]
struct ControlVolumeResult {
    control_volume_id: String,
    incoming_mass_flow_kg_s: f64,
    outgoing_mass_flow_kg_s: f64,
    storage_accumulation_kg_s: f64,
    residual_kg_s: f64,
    independent_standard_uncertainty_kg_s: f64,
    common_mode_standard_uncertainty_kg_s: f64,
    combined_standard_uncertainty_kg_s: f64,
    absolute_residual_z_score: f64,
    status: String,
}
#[derive(Debug, Serialize)]
struct GlobalBoundaryResult {
    external_incoming_mass_flow_kg_s: f64,
    external_outgoing_mass_flow_kg_s: f64,
    total_storage_accumulation_kg_s: f64,
    residual_kg_s: f64,
    independent_standard_uncertainty_kg_s: f64,
    common_mode_standard_uncertainty_kg_s: f64,
    combined_standard_uncertainty_kg_s: f64,
    absolute_residual_z_score: f64,
    internal_transfer_count: usize,
    status: String,
}
#[derive(Debug, Serialize)]
struct Receipt {
    schema_version: String,
    tool_version: String,
    fixture_id: String,
    input_sha256: String,
    input_byte_length: u64,
    input_hash_scope: String,
    synthetic_only: bool,
    status: String,
    control_volumes: Vec<ControlVolumeResult>,
    global_boundary: GlobalBoundaryResult,
    environmental_compliance_status: String,
    ecological_impact_status: String,
    site_clearance_status: String,
    claim_ceiling: String,
}

fn finite(value: f64, name: &str) -> Result<(), String> {
    if value.is_finite() { Ok(()) }
    else { Err(format!("{name} must be finite")) }
}
fn nonnegative(value: f64, name: &str) -> Result<(), String> {
    finite(value, name)?;
    if value < 0.0 { Err(format!("{name} must be nonnegative")) }
    else { Ok(()) }
}
fn positive(value: f64, name: &str) -> Result<(), String> {
    finite(value, name)?;
    if value <= 0.0 { Err(format!("{name} must be positive")) }
    else { Ok(()) }
}
fn valid_timestamp(value: &str) -> bool {
    let b = value.as_bytes();
    b.len() == 20
        && b[4] == b'-' && b[7] == b'-'
        && b[10] == b'T' && b[13] == b':'
        && b[16] == b':' && b[19] == b'Z'
        && [0usize,1,2,3,5,6,8,9,11,12,14,15,17,18]
            .iter().all(|i| b[*i].is_ascii_digit())
}
fn check_window(start: &str, end: &str, bundle: &Bundle, context: &str) -> Result<(), String> {
    if !valid_timestamp(start) || !valid_timestamp(end) || start >= end {
        return Err(format!("{context} has an invalid UTC interval"));
    }
    if start != bundle.interval_start_utc || end != bundle.interval_end_utc {
        return Err(format!("{context} does not match the bundle observation window"));
    }
    Ok(())
}
fn valid_digest(value: &str) -> bool {
    value.starts_with("sha256:") && value.len() > 7
}
fn calibration_covers(
    valid_from: &Option<String>,
    valid_until: &Option<String>,
    bundle: &Bundle,
    context: &str,
) -> Result<(), String> {
    let from = valid_from.as_deref().ok_or_else(|| format!("{context} lacks calibration start"))?;
    let until = valid_until.as_deref().ok_or_else(|| format!("{context} lacks calibration end"))?;
    if !valid_timestamp(from) || !valid_timestamp(until)
        || from > bundle.interval_start_utc.as_str()
        || until < bundle.interval_end_utc.as_str()
        || from >= until {
        return Err(format!("{context} calibration validity does not cover the entire interval"));
    }
    Ok(())
}
fn validate_evidence(
    evidence: &Evidence,
    bundle: &Bundle,
    context: &str,
) -> Result<(), String> {
    if evidence.record_id.trim().is_empty() {
        return Err(format!("{context} has empty record identity"));
    }
    check_window(&evidence.interval_start_utc, &evidence.interval_end_utc, bundle, context)?;
    if !bundle.evidence_states.iter().any(|s| s == &evidence.evidence_state)
        || evidence.evidence_state != "synthetic" {
        return Err(format!("{context} violates synthetic evidence claim boundary"));
    }
    if evidence.quality_state != "valid"
        || evidence.source_rights_state != "synthetic_test_data_only" {
        return Err(format!("{context} has invalid quality or source rights"));
    }
    if evidence.clock_offset_ms.unsigned_abs() > bundle.maximum_clock_offset_ms {
        return Err(format!("{context} signed clock offset exceeds tolerance"));
    }
    if !valid_digest(&evidence.raw_data_digest) {
        return Err(format!("{context} raw-data digest is missing or invalid"));
    }
    Ok(())
}
fn validate_storage(
    storage: &StorageInventory,
    bundle: &Bundle,
    ids: &mut HashSet<String>,
) -> Result<f64, String> {
    if storage.measurement_id.trim().is_empty() || !ids.insert(storage.measurement_id.clone()) {
        return Err("empty or duplicate storage-inventory identity".into());
    }
    check_window(
        &storage.interval_start_utc,
        &storage.interval_end_utc,
        bundle,
        &format!("storage inventory {}", storage.measurement_id),
    )?;
    if storage.quantity_origin != "metered" || storage.evidence_state != "synthetic" {
        return Err(format!("storage inventory {} must be an explicitly metered synthetic observation", storage.measurement_id));
    }
    if storage.quality_state != "valid"
        || storage.source_rights_state != "synthetic_test_data_only"
        || storage.clock_offset_ms.unsigned_abs() > bundle.maximum_clock_offset_ms
        || !valid_digest(&storage.raw_data_digest) {
        return Err(format!("storage inventory {} has unqualified evidence", storage.measurement_id));
    }
    for (name, value) in [
        ("sensor_id", storage.sensor_id.as_deref()),
        ("calibration_id", storage.calibration_id.as_deref()),
    ] {
        if value.map(str::trim).filter(|s| !s.is_empty()).is_none() {
            return Err(format!("storage inventory {} missing {name}", storage.measurement_id));
        }
    }
    calibration_covers(
        &storage.calibration_valid_from_utc,
        &storage.calibration_valid_until_utc,
        bundle,
        &format!("storage inventory {}", storage.measurement_id),
    )?;
    nonnegative(storage.independent_standard_uncertainty_kg, "storage inventory uncertainty")?;
    nonnegative(storage.common_mode_standard_uncertainty_kg, "storage common-mode uncertainty")?;
    finite(storage.signed_delta_storage_mass_kg, "signed storage mass change")?;
    let storage_rate = storage.signed_delta_storage_mass_kg / bundle.interval_duration_seconds as f64;
    if storage.common_mode_standard_uncertainty_kg > 0.0
        && storage.correlation_group_id.as_deref().unwrap_or("").trim().is_empty() {
        return Err("nonzero storage common-mode uncertainty requires correlation group".into());
    }
    if storage.common_mode_standard_uncertainty_kg == 0.0
        && storage.correlation_group_id.is_some() {
        return Err("storage correlation group is present with zero common-mode uncertainty".into());
    }
    Ok(storage_rate)
}
fn validate_stream_evidence(stream: &Stream, bundle: &Bundle) -> Result<(), String> {
    let context = format!("stream {}", stream.stream_id);
    validate_evidence(&stream.evidence, bundle, &context)?;
    match stream.quantity_origin.as_str() {
        "metered" => {
            if stream.model.is_some() {
                return Err(format!("{context} declares a model for a metered quantity"));
            }
            if stream.evidence.sensor_id.as_deref().unwrap_or("").trim().is_empty()
                || stream.evidence.calibration_id.as_deref().unwrap_or("").trim().is_empty() {
                return Err(format!("{context} metered quantity lacks sensor/calibration identity"));
            }
            calibration_covers(
                &stream.evidence.calibration_valid_from_utc,
                &stream.evidence.calibration_valid_until_utc,
                bundle,
                &context,
            )?;
        }
        "modelled" => {
            let model = stream.model.as_ref().ok_or_else(|| format!("{context} model evidence missing"))?;
            if model.method_id.trim().is_empty() || model.model_version.trim().is_empty()
                || model.input_digests.is_empty()
                || model.input_digests.iter().any(|d| !valid_digest(d))
                || model.validation_state != "synthetic_test_assumption" {
                return Err(format!("{context} model method, version, input digests or validation state is incomplete"));
            }
            if stream.evidence.sensor_id.is_some()
                || stream.evidence.calibration_id.is_some() {
                return Err(format!("{context} modelled quantity must not masquerade as a metered sensor result"));
            }
        }
        _ => return Err(format!("{context} quantity origin must be metered or modelled")),
    }
    Ok(())
}
fn validate(bundle: &Bundle) -> Result<Receipt, String> {
    if !bundle.synthetic_only || bundle.schema_version.trim().is_empty()
        || bundle.fixture_id.trim().is_empty() || bundle.as_of.trim().is_empty()
        || bundle.claim_ceiling.trim().is_empty() {
        return Err("bundle must be explicitly synthetic and fully identified".into());
    }
    check_window(&bundle.interval_start_utc, &bundle.interval_end_utc, bundle, "bundle")?;
    positive(bundle.interval_duration_seconds as f64, "interval_duration_seconds")?;
    positive(bundle.maximum_absolute_residual_z_score, "maximum_absolute_residual_z_score")?;
    if !bundle.claim_ceiling.contains("Not a field measurement")
        || bundle.interval_start_utc[11..13].parse::<u64>().unwrap_or(99) > 23 {
        return Err("bundle claim ceiling or interval metadata invalid".into());
    }
    if bundle.covariance_contract.unknown_correlation_state != "reject"
        || !bundle.covariance_contract.rule.contains("summed algebraically")
        || bundle.covariance_contract.correlation_groups.is_empty() {
        return Err("covariance contract must fail closed on unknown correlation".into());
    }
    let declared_groups: HashSet<&str> =
        bundle.covariance_contract.correlation_groups.iter().map(String::as_str).collect();
    if declared_groups.len() != bundle.covariance_contract.correlation_groups.len() {
        return Err("duplicate covariance correlation group".into());
    }
    if bundle.control_volumes.is_empty() || bundle.streams.is_empty() {
        return Err("bundle must contain at least one control volume and stream".into());
    }

    let mut cv_ids = HashSet::new();
    let mut ports: HashMap<(String, String), String> = HashMap::new();
    let mut storage_ids = HashSet::new();
    let mut storage_by_cv = HashMap::new();
    for cv in &bundle.control_volumes {
        if cv.control_volume_id.trim().is_empty() || !cv_ids.insert(cv.control_volume_id.clone()) {
            return Err(format!("empty or duplicate control-volume ID {}", cv.control_volume_id));
        }
        if cv.name.trim().is_empty() || cv.boundary_completeness_state != "declared_complete_synthetic" {
            return Err(format!("control volume {} boundary completeness is not qualified", cv.control_volume_id));
        }
        if cv.ports.is_empty() {
            return Err(format!("control volume {} has no boundary ports", cv.control_volume_id));
        }
        let mut local_ports = HashSet::new();
        for port in &cv.ports {
            if port.port_id.trim().is_empty() || !local_ports.insert(port.port_id.clone()) {
                return Err(format!("control volume {} has empty or duplicate port", cv.control_volume_id));
            }
            if !["in", "out"].contains(&port.direction.as_str()) {
                return Err(format!("port {} has invalid direction", port.port_id));
            }
            match port.status.as_str() {
                "connected" => {
                    if port.not_applicable_reason.as_ref().is_some_and(|r| !r.trim().is_empty()) {
                        return Err(format!("connected port {} carries a not-applicable reason", port.port_id));
                    }
                }
                "not_applicable" => {
                    if port.not_applicable_reason.as_deref().unwrap_or("").trim().is_empty() {
                        return Err(format!("not-applicable port {} lacks a justification", port.port_id));
                    }
                }
                _ => return Err(format!("port {} has unsupported status", port.port_id)),
            }
            ports.insert(
                (cv.control_volume_id.clone(), port.port_id.clone()),
                port.status.clone(),
            );
        }
        let storage_rate = validate_storage(&cv.storage_inventory, bundle, &mut storage_ids)?;
        storage_by_cv.insert(cv.control_volume_id.clone(), (storage_rate, &cv.storage_inventory));
    }

    let mut used_ports: HashMap<(String, String), usize> = HashMap::new();
    let mut stream_ids = HashSet::new();
    let mut per_cv_in: HashMap<String, f64> = HashMap::new();
    let mut per_cv_out: HashMap<String, f64> = HashMap::new();
    let mut per_cv_independent_var: HashMap<String, f64> = HashMap::new();
    let mut per_cv_common: HashMap<String, HashMap<String, f64>> = HashMap::new();
    let mut global_external_in = 0.0;
    let mut global_external_out = 0.0;
    let mut global_independent_var = 0.0;
    let mut global_common: HashMap<String, f64> = HashMap::new();
    let mut internal_transfer_count = 0usize;
    let mut record_ids = HashSet::new();

    for stream in &bundle.streams {
        if stream.stream_id.trim().is_empty() || !stream_ids.insert(stream.stream_id.clone()) {
            return Err(format!("empty or duplicate stream ID {}", stream.stream_id));
        }
        if stream.stream_type.trim().is_empty() {
            return Err(format!("stream {} type is empty", stream.stream_id));
        }
        if stream.evidence.record_id.trim().is_empty()
            || !record_ids.insert(stream.evidence.record_id.clone()) {
            return Err(format!("stream {} has empty or duplicate evidence record ID", stream.stream_id));
        }
        positive(stream.mass_flow_kg_s, "stream mass flow")?;
        nonnegative(stream.independent_standard_uncertainty_kg_s, "stream independent uncertainty")?;
        nonnegative(stream.common_mode_standard_uncertainty_kg_s, "stream common-mode uncertainty")?;
        if stream.common_mode_standard_uncertainty_kg_s > 0.0 {
            let group = stream.correlation_group_id.as_deref().unwrap_or("");
            if group.trim().is_empty() || !declared_groups.contains(group) {
                return Err(format!("stream {} has an unknown or undeclared correlation group", stream.stream_id));
            }
        } else if stream.correlation_group_id.is_some() {
            return Err(format!("stream {} declares correlation group with zero common-mode uncertainty", stream.stream_id));
        }
        validate_stream_evidence(stream, bundle)?;

        let mut source_cv: Option<String> = None;
        let mut destination_cv: Option<String> = None;
        let mut source_external = false;
        let mut destination_external = false;

        for (endpoint, expected_direction, label) in [
            (&stream.from, "out", "source"),
            (&stream.to, "in", "destination"),
        ] {
            match endpoint.endpoint_type.as_str() {
                "external" => {
                    if endpoint.node_id.trim().is_empty() || endpoint.port_id.is_some() {
                        return Err(format!("stream {} {label} external endpoint has invalid node/port", stream.stream_id));
                    }
                    if label == "source" { source_external = true; }
                    else { destination_external = true; }
                }
                "control_volume" => {
                    if !cv_ids.contains(&endpoint.node_id) {
                        return Err(format!("stream {} references unknown control volume {}", stream.stream_id, endpoint.node_id));
                    }
                    let port_id = endpoint.port_id.as_deref().ok_or_else(|| format!("stream {} {label} missing port ID", stream.stream_id))?;
                    let key = (endpoint.node_id.clone(), port_id.to_string());
                    let port_status = ports.get(&key).ok_or_else(|| format!("stream {} references unknown port", stream.stream_id))?;
                    if port_status != "connected" {
                        return Err(format!("stream {} uses a port marked not-applicable", stream.stream_id));
                    }
                    let cv = bundle.control_volumes.iter()
                        .find(|c| c.control_volume_id == endpoint.node_id)
                        .ok_or_else(|| format!("stream {} control volume disappeared", stream.stream_id))?;
                    let port = cv.ports.iter().find(|p| p.port_id == port_id)
                        .ok_or_else(|| format!("stream {} port disappeared", stream.stream_id))?;
                    if port.direction != expected_direction {
                        return Err(format!("stream {} {label} direction does not match port {}", stream.stream_id, port_id));
                    }
                    *used_ports.entry(key).or_insert(0) += 1;
                    if label == "source" { source_cv = Some(endpoint.node_id.clone()); }
                    else { destination_cv = Some(endpoint.node_id.clone()); }
                }
                _ => return Err(format!("stream {} has unsupported endpoint type", stream.stream_id)),
            }
        }
        if source_external && destination_external {
            return Err(format!("stream {} has no control-volume endpoint", stream.stream_id));
        }
        if let (Some(a), Some(b)) = (&source_cv, &destination_cv) {
            if a == b {
                return Err(format!("stream {} is a self-loop; model recirculation explicitly or split the control volume", stream.stream_id));
            }
            internal_transfer_count += 1;
            *per_cv_out.entry(a.clone()).or_insert(0.0) -= stream.mass_flow_kg_s;
            *per_cv_in.entry(b.clone()).or_insert(0.0) += stream.mass_flow_kg_s;
            *per_cv_independent_var.entry(a.clone()).or_insert(0.0) += stream.independent_standard_uncertainty_kg_s.powi(2);
            *per_cv_independent_var.entry(b.clone()).or_insert(0.0) += stream.independent_standard_uncertainty_kg_s.powi(2);
            if let Some(group) = &stream.correlation_group_id {
                *per_cv_common.entry(a.clone()).or_default().entry(group.clone()).or_insert(0.0) -= stream.common_mode_standard_uncertainty_kg_s;
                *per_cv_common.entry(b.clone()).or_default().entry(group.clone()).or_insert(0.0) += stream.common_mode_standard_uncertainty_kg_s;
            }
            // Internal transfer contributes equal and opposite terms to the global system boundary.
        } else if source_external {
            let cv = destination_cv.as_ref()
                .ok_or_else(|| format!("stream {} external source has no control-volume destination", stream.stream_id))?;
            global_external_in += stream.mass_flow_kg_s;
            per_cv_in.entry(cv.clone()).and_modify(|v| *v += stream.mass_flow_kg_s).or_insert(stream.mass_flow_kg_s);
            global_independent_var += stream.independent_standard_uncertainty_kg_s.powi(2);
            *per_cv_independent_var.entry(cv.clone()).or_insert(0.0) += stream.independent_standard_uncertainty_kg_s.powi(2);
            if let Some(group) = &stream.correlation_group_id {
                *global_common.entry(group.clone()).or_insert(0.0) += stream.common_mode_standard_uncertainty_kg_s;
                *per_cv_common.entry(cv.clone()).or_default().entry(group.clone()).or_insert(0.0) += stream.common_mode_standard_uncertainty_kg_s;
            }
        } else if destination_external {
            let cv = source_cv.as_ref()
                .ok_or_else(|| format!("stream {} external destination has no control-volume source", stream.stream_id))?;
            global_external_out += stream.mass_flow_kg_s;
            per_cv_out.entry(cv.clone()).and_modify(|v| *v -= stream.mass_flow_kg_s).or_insert(-stream.mass_flow_kg_s);
            global_independent_var += stream.independent_standard_uncertainty_kg_s.powi(2);
            *per_cv_independent_var.entry(cv.clone()).or_insert(0.0) += stream.independent_standard_uncertainty_kg_s.powi(2);
            if let Some(group) = &stream.correlation_group_id {
                *global_common.entry(group.clone()).or_insert(0.0) -= stream.common_mode_standard_uncertainty_kg_s;
                *per_cv_common.entry(cv.clone()).or_default().entry(group.clone()).or_insert(0.0) -= stream.common_mode_standard_uncertainty_kg_s;
            }
        }
    }

    for ((cv_id, port_id), status) in &ports {
        let count = used_ports.get(&(cv_id.clone(), port_id.clone())).copied().unwrap_or(0);
        match status.as_str() {
            "connected" if count != 1 => return Err(format!("connected port {cv_id}/{port_id} used {count} times; expected exactly once")),
            "not_applicable" if count != 0 => return Err(format!("not-applicable port {cv_id}/{port_id} is unexpectedly connected")),
            _ => {}
        }
    }

    let mut per_cv_results = Vec::new();
    let mut total_storage_rate = 0.0;
    let mut total_storage_independent_var = 0.0;

    for cv in &bundle.control_volumes {
        let (storage_rate, storage) = storage_by_cv.get(&cv.control_volume_id)
            .ok_or_else(|| format!("control volume {} lacks explicit storage inventory", cv.control_volume_id))?;
        total_storage_rate += storage_rate;
        let storage_u = storage.independent_standard_uncertainty_kg / bundle.interval_duration_seconds as f64;
        total_storage_independent_var += storage_u.powi(2);
        *per_cv_independent_var.entry(cv.control_volume_id.clone()).or_insert(0.0) += storage_u.powi(2);
        if storage.common_mode_standard_uncertainty_kg > 0.0 {
            let group = storage.correlation_group_id.as_ref()
                .ok_or_else(|| format!("control volume {} storage correlation group missing", cv.control_volume_id))?;
            if !declared_groups.contains(group.as_str()) {
                return Err(format!("control volume {} storage uses undeclared correlation group", cv.control_volume_id));
            }
            let common_rate = storage.common_mode_standard_uncertainty_kg / bundle.interval_duration_seconds as f64;
            *global_common.entry(group.clone()).or_insert(0.0) -= common_rate;
            *per_cv_common.entry(cv.control_volume_id.clone()).or_default().entry(group.clone()).or_insert(0.0) -= common_rate;
        }
        let incoming = per_cv_in.get(&cv.control_volume_id).copied().unwrap_or(0.0);
        let outgoing = -per_cv_out.get(&cv.control_volume_id).copied().unwrap_or(0.0);
        let residual = incoming - outgoing - storage_rate;
        finite(residual, "control volume residual")?;
        let independent_var = per_cv_independent_var.get(&cv.control_volume_id).copied().unwrap_or(0.0);
        let common_component = per_cv_common.get(&cv.control_volume_id)
            .map(|m| m.values().map(|v| v.powi(2)).sum::<f64>().sqrt()).unwrap_or(0.0);
        let independent = independent_var.sqrt();
        let combined = (independent.powi(2) + common_component.powi(2)).sqrt();
        positive(combined, "control-volume combined uncertainty")?;
        let z = residual.abs() / combined;
        if z > bundle.maximum_absolute_residual_z_score {
            return Err(format!("control volume {} residual exceeds declared uncertainty screen", cv.control_volume_id));
        }
        per_cv_results.push(ControlVolumeResult {
            control_volume_id:cv.control_volume_id.clone(),
            incoming_mass_flow_kg_s:incoming,
            outgoing_mass_flow_kg_s:outgoing,
            storage_accumulation_kg_s:*storage_rate,
            residual_kg_s:residual,
            independent_standard_uncertainty_kg_s:independent,
            common_mode_standard_uncertainty_kg_s:common_component,
            combined_standard_uncertainty_kg_s:combined,
            absolute_residual_z_score:z,
            status:"WITHIN_DECLARED_CONTROL_VOLUME_UNCERTAINTY_SCREEN_NOT_PERMIT_COMPLIANCE".into(),
        });
    }

    let global_residual = global_external_in - global_external_out - total_storage_rate;
    finite(global_residual, "global boundary residual")?;
    let global_independent_var = global_independent_var + total_storage_independent_var;
    let global_common_combined = global_common.values().map(|v| v.powi(2)).sum::<f64>().sqrt();
    let global_independent = global_independent_var.sqrt();
    let global_combined = (global_independent.powi(2) + global_common_combined.powi(2)).sqrt();
    positive(global_combined, "global boundary combined uncertainty")?;
    let global_z = global_residual.abs() / global_combined;
    if global_z > bundle.maximum_absolute_residual_z_score {
        return Err("global external boundary residual exceeds declared uncertainty screen".into());
    }

    Ok(Receipt {
        schema_version:bundle.schema_version.clone(),
        tool_version:env!("CARGO_PKG_VERSION").to_string(),
        fixture_id:bundle.fixture_id.clone(),
        input_sha256:String::new(),
        input_byte_length:0,
        input_hash_scope:"unset_until_cli_hashes_exact_input_bytes".into(),
        synthetic_only:true,
        status:"SYNTHETIC_MULTI_STREAM_BALANCE_VALID_NO_SITE_OR_PERMIT_CLAIM".into(),
        control_volumes:per_cv_results,
        global_boundary:GlobalBoundaryResult {
            external_incoming_mass_flow_kg_s:global_external_in,
            external_outgoing_mass_flow_kg_s:global_external_out,
            total_storage_accumulation_kg_s:total_storage_rate,
            residual_kg_s:global_residual,
            independent_standard_uncertainty_kg_s:global_independent,
            common_mode_standard_uncertainty_kg_s:global_common_combined,
            combined_standard_uncertainty_kg_s:global_combined,
            absolute_residual_z_score:global_z,
            internal_transfer_count,
            status:"WITHIN_DECLARED_GLOBAL_BOUNDARY_UNCERTAINTY_SCREEN_NOT_PERMIT_COMPLIANCE".into(),
        },
        environmental_compliance_status:"NOT_ASSESSED_NO_PERMIT_LIMIT_OR_FIELD_BASELINE".into(),
        ecological_impact_status:"NOT_ASSESSED_NO_FIELD_ECOLOGICAL_EVIDENCE".into(),
        site_clearance_status:"UNKNOWN_BLOCKED".into(),
        claim_ceiling:bundle.claim_ceiling.clone(),
    })
}

fn run(path: &Path) -> Result<Receipt, String> {
    let bytes = fs::read(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let digest = Sha256::digest(&bytes);
    let mut bundle: Bundle =
        serde_json::from_slice(&bytes).map_err(|e| format!("invalid bundle JSON: {e}"))?;
    if bundle.interval_duration_seconds == 0 {
        return Err("interval duration cannot be zero".into());
    }
    let mut receipt = validate(&bundle)?;
    receipt.input_sha256 = format!("{digest:x}");
    receipt.input_byte_length = bytes.len() as u64;
    receipt.input_hash_scope =
        "SHA-256 over exact input-file bytes; proves byte identity, not meter authenticity or acquisition-chain integrity".into();
    Ok(receipt)
}
fn main() {
    let path = env::args().nth(1).unwrap_or_else(|| DEFAULT_PATH.into());
    match run(Path::new(&path)) {
        Ok(receipt) => match serde_json::to_string_pretty(&receipt) {
            Ok(value) => println!("{value}"),
            Err(error) => { eprintln!("cannot serialize receipt: {error}"); std::process::exit(2); }
        },
        Err(error) => { eprintln!("OCEAN_MULTISTREAM_BALANCE_REJECTED: {error}"); std::process::exit(1); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Bundle {
        serde_json::from_str(include_str!(
            "../../../docs/research/port-energy-compute/OCEAN_MULTISTREAM_BALANCE_FIXTURE_V0_1.json"
        )).expect("multi-stream fixture parses")
    }
    #[test]
    fn complete_multi_stream_graph_closes_each_control_volume_and_global_boundary() {
        let report = validate(&fixture()).expect("synthetic graph should validate");
        assert_eq!(report.control_volumes.len(), 2);
        assert_eq!(report.global_boundary.internal_transfer_count, 1);
        assert_eq!(report.global_boundary.external_incoming_mass_flow_kg_s, 10.0);
        assert!((report.global_boundary.external_outgoing_mass_flow_kg_s - 10.0).abs() < 1e-12);
        assert_eq!(report.global_boundary.residual_kg_s, 0.0);
        assert_eq!(report.site_clearance_status, "UNKNOWN_BLOCKED");
    }
    #[test]
    fn missing_purge_port_stream_is_rejected() {
        let mut b = fixture();
        b.streams.retain(|s| s.stream_id != "S-PURGE");
        assert!(validate(&b).unwrap_err().contains("used 0 times; expected exactly once"));
    }
    #[test]
    fn internal_transfer_direction_mismatch_is_rejected() {
        let mut b = fixture();
        b.streams[1].from.port_id = Some("p-intake".into());
        assert!(validate(&b).unwrap_err().contains("source direction does not match"));
    }
    #[test]
    fn modelled_evaporation_requires_method_version_and_input_receipts() {
        let mut b = fixture();
        b.streams[2].model.as_mut().unwrap().input_digests.clear();
        assert!(validate(&b).unwrap_err().contains("model method, version, input digests"));
    }
    #[test]
    fn common_mode_uncertainty_requires_a_declared_group() {
        let mut b = fixture();
        b.streams[0].correlation_group_id = Some("UNDECLARED".into());
        assert!(validate(&b).unwrap_err().contains("unknown or undeclared correlation group"));
    }
    #[test]
    fn zero_fill_for_a_connected_stream_is_rejected() {
        let mut b = fixture();
        b.streams[0].mass_flow_kg_s = 0.0;
        assert!(validate(&b).unwrap_err().contains("stream mass flow must be positive"));
    }
    #[test]
    fn stale_sample_calibration_is_rejected() {
        let mut b = fixture();
        b.streams[0].evidence.calibration_valid_until_utc = Some("2026-10-12T09:59:59Z".into());
        assert!(validate(&b).unwrap_err().contains("calibration validity does not cover"));
    }
    #[test]
    fn unknown_not_applicable_port_without_reason_is_rejected() {
        let mut b = fixture();
        let port = b.control_volumes[0].ports.iter_mut()
            .find(|p| p.port_id == "p-chemical-cleaning").unwrap();
        port.not_applicable_reason = None;
        assert!(validate(&b).unwrap_err().contains("lacks a justification"));
    }
}
