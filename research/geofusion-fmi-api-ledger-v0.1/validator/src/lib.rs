#![forbid(unsafe_code)]

use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    fs,
    path::{Component, Path, PathBuf},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditReport {
    pub federates: usize,
    pub events: usize,
    pub do_step_attempts: usize,
    pub exchanged_values: usize,
    pub barriers: usize,
    pub artifacts_verified: usize,
    pub synthetic: bool,
    pub state_checkpointing: &'static str,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Ledger {
    schema_version: String,
    ledger_id: String,
    evidence_class: String,
    created_utc: String,
    fmi_standard_release: String,
    master: Master,
    artifacts: Vec<Artifact>,
    federates: Vec<Federate>,
    events: Vec<Event>,
    commit_barriers: Vec<Barrier>,
    state_checkpointing: Checkpoint,
    normalized_trace_artifact_id: String,
    energy_contract_artifact_id: String,
    claim_ceiling: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Master {
    master_id: String,
    name: String,
    version: String,
    algorithm_id: String,
    platform: String,
    artifact_id: String,
    collector_mode: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Artifact {
    artifact_id: String,
    role: String,
    path: String,
    byte_length: u64,
    sha256: String,
    executable: bool,
    synthetic_stub: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Federate {
    federate_id: String,
    model_name: String,
    fmi_model_description_version: String,
    interface_type: String,
    model_identifier: String,
    platform: String,
    artifacts: FederateArtifacts,
    clock: Clock,
    capabilities: Capabilities,
    variables: Vec<Variable>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FederateArtifacts {
    package: String,
    model_description: String,
    platform_binary: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Clock {
    clock_id: String,
    master_clock_id: String,
    model_time_unit: String,
    mapping_digest: String,
    nanoseconds_per_model_time_numerator: i64,
    nanoseconds_per_model_time_denominator: i64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Capabilities {
    can_handle_variable_communication_step_size: bool,
    can_interpolate_inputs: bool,
    can_return_early_after_intermediate_update: bool,
    has_event_mode: bool,
    can_get_and_set_fmu_state: bool,
    can_serialize_fmu_state: bool,
    can_run_asynchronously: bool,
    provides_intermediate_update: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Variable {
    value_reference: u64,
    name: String,
    fmi_type: String,
    causality: String,
    variability: String,
    unit: String,
    port_id: String,
    carrier: String,
    boundary_id: String,
    port_role: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Event {
    sequence: usize,
    event_id: String,
    federate_id: String,
    kind: String,
    call_symbol: String,
    status: String,
    payload: Payload,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct Payload {
    interface_type: Option<String>,
    early_return_allowed: Option<bool>,
    event_mode_used: Option<bool>,
    instance_name: Option<String>,
    visible: Option<bool>,
    logging_on: Option<bool>,
    resource_path: Option<String>,
    required_intermediate_variables: Option<Vec<u64>>,
    transition: Option<String>,
    tolerance_defined: Option<bool>,
    tolerance_bits: Option<String>,
    start_time_bits: Option<String>,
    stop_time_defined: Option<bool>,
    stop_time_bits: Option<String>,
    interval: Option<Interval>,
    value_references: Option<Vec<u64>>,
    n_value_references: Option<usize>,
    values: Option<Vec<ExchangeValue>>,
    attempt_id: Option<String>,
    current_communication_point_bits: Option<String>,
    communication_step_size_bits: Option<String>,
    last_successful_time_bits: Option<String>,
    early_return: Option<bool>,
    event_handling_needed: Option<bool>,
    terminate_simulation: Option<bool>,
    no_set_fmu_state_prior_to_current_point: Option<bool>,
    master_start_ns: Option<i64>,
    master_end_ns: Option<i64>,
    state_artifact_id: Option<String>,
    checkpoint_id: Option<String>,
    reason_code: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Interval {
    source_interval_id: String,
    start_master_ns: i64,
    end_master_ns: i64,
    sampling_semantics: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExchangeValue {
    value_reference: u64,
    value: f64,
    raw_f64_bits: String,
    port_id: String,
    source_interval_id: String,
    transfer_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Barrier {
    barrier_id: String,
    sequence: usize,
    master_time_ns: i64,
    federate_ids: Vec<String>,
    attempt_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Checkpoint {
    status: String,
    reason_code: String,
    snapshot_artifact_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Trace {
    fixture: String,
    attempts: Vec<TraceAttempt>,
    claim: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct TraceAttempt {
    attempt_id: String,
    federate_id: String,
    start_master_ns: i64,
    end_master_ns: i64,
    status: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct EnergyContract {
    fixture: String,
    interval_id: String,
    unit: String,
    expected: BTreeMap<String, f64>,
    residual_w: f64,
    claim: String,
}

#[derive(Debug, Clone)]
struct StepRecord {
    federate_id: String,
    start_master_ns: i64,
    end_master_ns: i64,
    status: String,
}

#[derive(Debug, Clone)]
struct ValueRecord {
    kind: String,
    federate_id: String,
    variable: VariableRecord,
    value: ExchangeValue,
    interval: Interval,
}

#[derive(Debug, Clone)]
struct VariableRecord {
    causality: String,
    unit: String,
    port_id: String,
    carrier: String,
    boundary_id: String,
    port_role: String,
}

#[derive(Debug, Default)]
struct Lifecycle {
    created: bool,
    init_entered: bool,
    initialized: bool,
    do_step_seen: bool,
    terminated: bool,
    freed: bool,
}

pub fn audit_file(ledger_path: &Path, artifact_root: &Path) -> Result<AuditReport, String> {
    let bytes = fs::read(ledger_path)
        .map_err(|e| format!("cannot read ledger {}: {e}", ledger_path.display()))?;
    let ledger: Ledger = serde_json::from_slice(&bytes)
        .map_err(|e| format!("ledger JSON/schema decoding failed: {e}"))?;
    audit(ledger, artifact_root)
}

pub fn audit_json(contents: &str, artifact_root: &Path) -> Result<AuditReport, String> {
    let ledger: Ledger = serde_json::from_str(contents)
        .map_err(|e| format!("ledger JSON/schema decoding failed: {e}"))?;
    audit(ledger, artifact_root)
}

fn nonempty(value: &str, label: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        Err(format!("{label} must be non-empty"))
    } else {
        Ok(())
    }
}

fn parse_bits(value: &str, label: &str) -> Result<f64, String> {
    if value.len() != 16 || !value.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(format!("{label}: expected 16 hexadecimal IEEE-754 bits"));
    }
    let bits = u64::from_str_radix(value, 16)
        .map_err(|_| format!("{label}: invalid hexadecimal bits"))?;
    let decoded = f64::from_bits(bits);
    if !decoded.is_finite() {
        return Err(format!("{label}: non-finite Float64 is not admissible"));
    }
    Ok(decoded)
}

fn gcd_i128(left: i128, right: i128) -> i128 {
    let mut a = left.unsigned_abs();
    let mut b = right.unsigned_abs();
    while b != 0 {
        let remainder = a % b;
        a = b;
        b = remainder;
    }
    i128::try_from(a).expect("gcd is bounded by a positive i128 denominator")
}

/// Convert the exact IEEE-754 value into an integer nanosecond count using
/// rational arithmetic. No floating-point multiplication or rounding occurs.
fn to_ns(model_time: f64, clock: &Clock, label: &str) -> Result<i64, String> {
    if !model_time.is_finite() {
        return Err(format!("{label}: non-finite model time"));
    }
    if clock.nanoseconds_per_model_time_numerator <= 0
        || clock.nanoseconds_per_model_time_denominator <= 0
    {
        return Err(format!("{label}: invalid rational clock scale"));
    }

    let bits = model_time.to_bits();
    let negative = (bits >> 63) != 0;
    let exponent_bits = ((bits >> 52) & 0x7ff) as i32;
    let fraction_bits = bits & ((1_u64 << 52) - 1);
    if exponent_bits == 0 && fraction_bits == 0 {
        return Ok(0);
    }

    let (significand, binary_exponent) = if exponent_bits == 0 {
        (i128::from(fraction_bits), -1074)
    } else {
        (
            i128::from((1_u64 << 52) | fraction_bits),
            exponent_bits - 1023 - 52,
        )
    };
    let signed_significand = if negative { -significand } else { significand };

    let (mut value_numerator, mut value_denominator) = if binary_exponent >= 0 {
        let shift = u32::try_from(binary_exponent)
            .map_err(|_| format!("{label}: binary exponent overflow"))?;
        let factor = 1_i128
            .checked_shl(shift)
            .ok_or_else(|| format!("{label}: binary exponent overflow"))?;
        (
            signed_significand.checked_mul(factor)
                .ok_or_else(|| format!("{label}: binary time numerator overflow"))?,
            1_i128,
        )
    } else {
        let shift = u32::try_from(-binary_exponent)
            .map_err(|_| format!("{label}: binary exponent overflow"))?;
        let denominator = 1_i128
            .checked_shl(shift)
            .ok_or_else(|| format!("{label}: binary time denominator overflow"))?;
        (signed_significand, denominator)
    };

    let value_gcd = gcd_i128(value_numerator, value_denominator);
    value_numerator /= value_gcd;
    value_denominator /= value_gcd;

    let mut scale_numerator = i128::from(clock.nanoseconds_per_model_time_numerator);
    let mut scale_denominator = i128::from(clock.nanoseconds_per_model_time_denominator);

    let gcd_left = gcd_i128(value_numerator, scale_denominator);
    value_numerator /= gcd_left;
    scale_denominator /= gcd_left;

    let gcd_right = gcd_i128(scale_numerator, value_denominator);
    scale_numerator /= gcd_right;
    value_denominator /= gcd_right;

    let numerator = value_numerator.checked_mul(scale_numerator)
        .ok_or_else(|| format!("{label}: scaled time numerator overflow"))?;
    let denominator = value_denominator.checked_mul(scale_denominator)
        .ok_or_else(|| format!("{label}: scaled time denominator overflow"))?;

    if denominator == 0 || numerator % denominator != 0 {
        return Err(format!("{label}: exact value is not an integral master nanosecond"));
    }
    i64::try_from(numerator / denominator)
        .map_err(|_| format!("{label}: master nanosecond value exceeds i64"))
}

fn checked_artifact_path(root: &Path, relative: &str) -> Result<PathBuf, String> {
    let rel = Path::new(relative);
    if rel.components().any(|c| matches!(c, Component::ParentDir | Component::RootDir | Component::Prefix(_))) {
        return Err(format!("artifact path is not root-relative: {relative}"));
    }
    let root = root.canonicalize()
        .map_err(|e| format!("cannot canonicalize artifact root {}: {e}", root.display()))?;
    let joined = root.join(rel);
    let mut cursor = root.clone();
    for component in rel.components() {
        if let Component::Normal(part) = component {
            cursor.push(part);
            let meta = fs::symlink_metadata(&cursor)
                .map_err(|e| format!("cannot inspect artifact path {}: {e}", cursor.display()))?;
            if meta.file_type().is_symlink() {
                return Err(format!("symlink artifact path is rejected: {relative}"));
            }
        }
    }
    let full = joined.canonicalize()
        .map_err(|e| format!("cannot resolve artifact {relative}: {e}"))?;
    if !full.starts_with(&root) || !full.is_file() {
        return Err(format!("artifact escapes root or is not a regular file: {relative}"));
    }
    Ok(full)
}

fn digest_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

fn touch_payload(p: &Payload) {
    let _ = (
        &p.resource_path, &p.state_artifact_id, &p.checkpoint_id, &p.reason_code,
    );
}

fn audit(ledger: Ledger, artifact_root: &Path) -> Result<AuditReport, String> {
    for (value, name) in [
        (&ledger.schema_version, "schema_version"),
        (&ledger.ledger_id, "ledger_id"),
        (&ledger.created_utc, "created_utc"),
        (&ledger.fmi_standard_release, "fmi_standard_release"),
        (&ledger.claim_ceiling, "claim_ceiling"),
        (&ledger.master.master_id, "master.master_id"),
        (&ledger.master.name, "master.name"),
        (&ledger.master.version, "master.version"),
        (&ledger.master.algorithm_id, "master.algorithm_id"),
        (&ledger.master.platform, "master.platform"),
    ] { nonempty(value, name)?; }

    if ledger.schema_version != "0.1.0" || ledger.fmi_standard_release != "3.0.2" {
        return Err("unsupported ledger/FMI schema release".into());
    }
    if !["synthetic_fixture", "instrumented_runtime"].contains(&ledger.evidence_class.as_str()) {
        return Err("unknown evidence_class".into());
    }
    if ledger.master.collector_mode != "instrumented_c_api_boundary"
        && ledger.master.collector_mode != "fixture_authored_not_runtime_instrumented" {
        return Err("unknown collector_mode".into());
    }
    if ledger.artifacts.is_empty() || ledger.federates.is_empty() || ledger.events.is_empty() {
        return Err("artifacts, federates and events must be non-empty".into());
    }

    let root = artifact_root.canonicalize()
        .map_err(|e| format!("cannot canonicalize artifact root: {e}"))?;
    let mut artifacts = HashMap::new();
    for artifact in &ledger.artifacts {
        nonempty(&artifact.artifact_id, "artifact_id")?;
        nonempty(&artifact.sha256, "artifact.sha256")?;
        if artifact.sha256.len() != 64 || !artifact.sha256.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(format!("invalid SHA-256 syntax for {}", artifact.artifact_id));
        }
        if artifacts.insert(artifact.artifact_id.as_str(), artifact).is_some() {
            return Err(format!("duplicate artifact id {}", artifact.artifact_id));
        }
        let path = checked_artifact_path(&root, &artifact.path)?;
        let bytes = fs::read(&path)
            .map_err(|e| format!("cannot read artifact {}: {e}", artifact.path))?;
        if bytes.len() as u64 != artifact.byte_length {
            return Err(format!("artifact byte length mismatch: {}", artifact.artifact_id));
        }
        if digest_hex(&bytes) != artifact.sha256.to_ascii_lowercase() {
            return Err(format!("artifact SHA-256 mismatch: {}", artifact.artifact_id));
        }
        if ledger.evidence_class == "synthetic_fixture" && (artifact.executable || !artifact.synthetic_stub) {
            return Err(format!("synthetic artifact must be an explicitly non-executable stub: {}", artifact.artifact_id));
        }
    }
    let master_artifact = artifacts.get(ledger.master.artifact_id.as_str())
        .ok_or("master executable artifact reference is missing")?;
    if master_artifact.role != "master_executable" {
        return Err("master artifact has the wrong role".into());
    }
    let trace_artifact = artifacts.get(ledger.normalized_trace_artifact_id.as_str())
        .ok_or("normalized trace artifact reference is missing")?;
    let energy_artifact = artifacts.get(ledger.energy_contract_artifact_id.as_str())
        .ok_or("energy contract artifact reference is missing")?;
    if trace_artifact.role != "normalized_trace" || energy_artifact.role != "energy_contract" {
        return Err("trace or energy artifact has the wrong role".into());
    }

    let mut federates = HashMap::new();
    let mut variable_maps: HashMap<&str, HashMap<u64, VariableRecord>> = HashMap::new();
    for f in &ledger.federates {
        for (value, label) in [
            (&f.federate_id, "federate_id"),
            (&f.model_name, "model_name"),
            (&f.model_identifier, "model_identifier"),
            (&f.platform, "platform"),
            (&f.clock.clock_id, "clock_id"),
            (&f.clock.master_clock_id, "master_clock_id"),
            (&f.clock.mapping_digest, "clock.mapping_digest"),
        ] { nonempty(value, label)?; }
        if f.fmi_model_description_version != "3.0" || f.interface_type != "CoSimulation" {
            return Err(format!("{} is outside the FMI 3.0 CoSimulation profile", f.federate_id));
        }
        if f.clock.model_time_unit != "s" || f.clock.nanoseconds_per_model_time_numerator <= 0
            || f.clock.nanoseconds_per_model_time_denominator <= 0 {
            return Err(format!("{} has an invalid clock mapping", f.federate_id));
        }
        if f.capabilities.can_serialize_fmu_state && !f.capabilities.can_get_and_set_fmu_state {
            return Err(format!("{} advertises canSerializeFMUState without canGetAndSetFMUState", f.federate_id));
        }
        let _capability_profile = (
            f.capabilities.can_handle_variable_communication_step_size,
            f.capabilities.can_interpolate_inputs,
            f.capabilities.can_return_early_after_intermediate_update,
            f.capabilities.has_event_mode,
            f.capabilities.can_run_asynchronously,
            f.capabilities.provides_intermediate_update,
        );
        if federates.insert(f.federate_id.as_str(), f).is_some() {
            return Err(format!("duplicate federate {}", f.federate_id));
        }
        let mut map = HashMap::new();
        for v in &f.variables {
            if v.fmi_type != "Float64" || !["input", "output"].contains(&v.causality.as_str())
                || !["continuous", "discrete", "tunable", "fixed", "constant"].contains(&v.variability.as_str())
                || !["coupling_source", "coupling_sink", "terminal_sink"].contains(&v.port_role.as_str()) {
                return Err(format!("{} variable {} outside the declared Float64 physical-port profile", f.federate_id, v.name));
            }
            for (value, label) in [
                (&v.name, "variable.name"), (&v.unit, "variable.unit"),
                (&v.port_id, "variable.port_id"), (&v.carrier, "variable.carrier"),
                (&v.boundary_id, "variable.boundary_id"),
            ] { nonempty(value, label)?; }
            if map.insert(v.value_reference, VariableRecord {
                causality: v.causality.clone(), unit: v.unit.clone(), port_id: v.port_id.clone(),
                carrier: v.carrier.clone(), boundary_id: v.boundary_id.clone(), port_role: v.port_role.clone(),
            }).is_some() {
                return Err(format!("duplicate value reference in {}", f.federate_id));
            }
        }
        variable_maps.insert(&f.federate_id, map);
        for (role, id, expected_role) in [
            ("package", f.artifacts.package.as_str(), "fmu_package"),
            ("model_description", f.artifacts.model_description.as_str(), "model_description"),
            ("platform_binary", f.artifacts.platform_binary.as_str(), "platform_binary"),
        ] {
            let a = artifacts.get(id).ok_or_else(|| format!("{} {role} artifact is missing", f.federate_id))?;
            if a.role != expected_role {
                return Err(format!("{} {role} artifact has role {}", f.federate_id, a.role));
            }
        }
    }

    let mut life: HashMap<&str, Lifecycle> = federates.keys().map(|id| (*id, Lifecycle::default())).collect();
    let mut event_ids = BTreeSet::new();
    let mut steps: HashMap<String, StepRecord> = HashMap::new();
    let mut successful = BTreeSet::new();
    let mut value_records = Vec::new();
    for (index, event) in ledger.events.iter().enumerate() {
        if event.sequence != index || !event_ids.insert(event.event_id.as_str()) {
            return Err(format!("event sequence gap or duplicate event id at index {index}"));
        }
        if event.status != "ok" {
            return Err(format!("positive ledger contains non-ok FMI status at {}", event.event_id));
        }
        let f = federates.get(event.federate_id.as_str())
            .ok_or_else(|| format!("unknown federate in event {}", event.event_id))?;
        let state = life.get_mut(event.federate_id.as_str()).expect("federate lifecycle exists");
        touch_payload(&event.payload);
        let expected_symbol = match event.kind.as_str() {
            "instance_created" => "fmi3InstantiateCoSimulation",
            "initialization_mode" => if event.payload.transition.as_deref() == Some("enter") { "fmi3EnterInitializationMode" } else { "fmi3ExitInitializationMode" },
            "set_float64" => "fmi3SetFloat64",
            "get_float64" => "fmi3GetFloat64",
            "do_step" => "fmi3DoStep",
            "terminate" => "fmi3Terminate",
            "free_instance" => "fmi3FreeInstance",
            "state_snapshot" => "fmi3GetFMUState",
            "state_restore" => "fmi3SetFMUState",
            other => return Err(format!("unsupported event kind {other}")),
        };
        if event.call_symbol != expected_symbol {
            return Err(format!("call-symbol mismatch at {}", event.event_id));
        }

        match event.kind.as_str() {
            "instance_created" => {
                if state.created || state.terminated || event.payload.interface_type.as_deref() != Some("CoSimulation")
                    || event.payload.instance_name.as_deref().unwrap_or("").is_empty()
                    || event.payload.visible != Some(false) || event.payload.logging_on != Some(false)
                    || event.payload.required_intermediate_variables.is_none()
                    || event.payload.early_return_allowed != Some(false)
                    || event.payload.event_mode_used != Some(false) {
                    return Err(format!("invalid instantiate event {}", event.event_id));
                }
                state.created = true;
            }
            "initialization_mode" => {
                if !state.created || state.terminated { return Err(format!("initialization outside instance lifetime at {}", event.event_id)); }
                match event.payload.transition.as_deref() {
                    Some("enter") => {
                        if state.init_entered || state.initialized { return Err(format!("duplicate initialization entry for {}", event.federate_id)); }
                        let start = parse_bits(event.payload.start_time_bits.as_deref().ok_or("missing initialization start time")?, "start_time")?;
                        let stop = parse_bits(event.payload.stop_time_bits.as_deref().ok_or("missing initialization stop time")?, "stop_time")?;
                        if start >= stop || event.payload.tolerance_defined != Some(false)
                            || event.payload.tolerance_bits.is_some() || event.payload.stop_time_defined != Some(true) {
                            return Err(format!("invalid initialization arguments at {}", event.event_id));
                        }
                        state.init_entered = true;
                    }
                    Some("exit") => {
                        if !state.init_entered || state.initialized { return Err(format!("invalid initialization exit for {}", event.federate_id)); }
                        state.initialized = true;
                    }
                    _ => return Err(format!("unknown initialization transition at {}", event.event_id)),
                }
            }
            "set_float64" | "get_float64" => {
                if !state.initialized || state.terminated { return Err(format!("I/O outside initialized lifetime at {}", event.event_id)); }
                if event.kind == "get_float64" && !successful.contains(event.federate_id.as_str()) {
                    return Err(format!("output read before successful step at {}", event.event_id));
                }
                if event.kind == "set_float64" && state.do_step_seen {
                    return Err(format!("input set after step at {}", event.event_id));
                }
                let interval = event.payload.interval.as_ref().ok_or("missing exchange interval")?;
                if interval.start_master_ns < 0 || interval.start_master_ns >= interval.end_master_ns
                    || !["constant_over_declared_half_open_interval", "sample_at_interval_end",
                        "sample_at_interval_start", "interval_average"].contains(&interval.sampling_semantics.as_str()) {
                    return Err(format!("invalid exchange interval at {}", event.event_id));
                }
                let refs = event.payload.value_references.as_ref().ok_or("missing value references")?;
                let values = event.payload.values.as_ref().ok_or("missing exchanged values")?;
                if refs.len() != values.len() || event.payload.n_value_references != Some(values.len())
                    || refs.iter().zip(values).any(|(r,v)| *r != v.value_reference) || values.is_empty() {
                    return Err(format!("value reference/count mismatch at {}", event.event_id));
                }
                for value in values {
                    let variable = variable_maps.get(event.federate_id.as_str()).and_then(|m| m.get(&value.value_reference))
                        .ok_or_else(|| format!("undeclared value reference at {}", event.event_id))?;
                    let expected_causality = if event.kind == "set_float64" { "input" } else { "output" };
                    if variable.causality != expected_causality || variable.port_id != value.port_id
                        || value.source_interval_id != interval.source_interval_id {
                        return Err(format!("variable causality/port/interval mismatch at {}", event.event_id));
                    }
                    let decoded = parse_bits(&value.raw_f64_bits, "raw_f64_bits")?;
                    if decoded.to_bits() != value.value.to_bits() {
                        return Err(format!("Float64 bit-pattern mismatch at {}", event.event_id));
                    }
                    value_records.push(ValueRecord {
                        kind: event.kind.clone(), federate_id: event.federate_id.clone(),
                        variable: variable.clone(), value: value.clone(), interval: interval.clone(),
                    });
                }
            }
            "do_step" => {
                if !state.initialized || state.terminated { return Err(format!("doStep outside initialized lifetime at {}", event.event_id)); }
                let p = &event.payload;
                let aid = p.attempt_id.as_deref().ok_or("missing normalized attempt id")?;
                if steps.contains_key(aid) { return Err(format!("duplicate attempt binding {aid}")); }
                let cp = parse_bits(p.current_communication_point_bits.as_deref().ok_or("missing current communication point")?, "currentCommunicationPoint")?;
                let dt = parse_bits(p.communication_step_size_bits.as_deref().ok_or("missing communication step size")?, "communicationStepSize")?;
                let last = parse_bits(p.last_successful_time_bits.as_deref().ok_or("missing lastSuccessfulTime")?, "lastSuccessfulTime")?;
                if dt <= 0.0 || last <= cp || last > cp + dt
                    || p.early_return != Some(false) || p.event_handling_needed != Some(false)
                    || p.terminate_simulation != Some(false) || p.no_set_fmu_state_prior_to_current_point != Some(true) {
                    return Err(format!("invalid or unsupported doStep result at {}", event.event_id));
                }
                let clock = &f.clock;
                let mapped_start = to_ns(cp, clock, "currentCommunicationPoint")?;
                let mapped_end = to_ns(last, clock, "lastSuccessfulTime")?;
                if p.master_start_ns != Some(mapped_start) || p.master_end_ns != Some(mapped_end) {
                    return Err(format!("FMI time does not match master time at {}", event.event_id));
                }
                steps.insert(aid.to_owned(), StepRecord {
                    federate_id: event.federate_id.clone(),
                    start_master_ns: mapped_start, end_master_ns: mapped_end, status: "accepted".to_owned(),
                });
                successful.insert(event.federate_id.as_str());
                state.do_step_seen = true;
            }
            "state_snapshot" | "state_restore" => {
                if !(f.capabilities.can_get_and_set_fmu_state && f.capabilities.can_serialize_fmu_state) {
                    return Err(format!("state API used without both advertised capabilities at {}", event.event_id));
                }
                return Err("state snapshot/restore event requires v0.2 state-artifact semantics".into());
            }
            "terminate" => {
                if !state.initialized || state.terminated { return Err(format!("invalid terminate at {}", event.event_id)); }
                state.terminated = true;
            }
            "free_instance" => {
                if !state.terminated || state.freed { return Err(format!("freeInstance without exactly one prior terminate at {}", event.event_id)); }
                state.freed = true;
            }
            _ => unreachable!(),
        }
    }
    for (id, state) in &life {
        if !(state.created && state.init_entered && state.initialized && state.do_step_seen && state.terminated && state.freed) {
            return Err(format!("incomplete FMI lifecycle for {id}"));
        }
    }
    if steps.is_empty() { return Err("ledger contains no doStep attempts".into()); }

    for value in &value_records {
        let step = steps.values().find(|s| s.federate_id == value.federate_id
            && s.start_master_ns == value.interval.start_master_ns
            && s.end_master_ns == value.interval.end_master_ns);
        if step.is_none() {
            return Err(format!("exchanged value interval has no matching successful doStep for port {}", value.variable.port_id));
        }
    }

    let mut transfer_reads: HashMap<&str, &ValueRecord> = HashMap::new();
    let mut transfer_writes: HashMap<&str, &ValueRecord> = HashMap::new();
    let mut observed_ports = BTreeMap::new();
    for value in &value_records {
        observed_ports.insert(value.variable.port_id.clone(), value.value.value);
        if let Some(id) = value.value.transfer_id.as_deref() {
            let map = if value.kind == "get_float64" { &mut transfer_reads } else { &mut transfer_writes };
            if map.insert(id, value).is_some() { return Err(format!("duplicate transfer leg for {id}")); }
        }
    }
    if transfer_reads.keys().collect::<BTreeSet<_>>() != transfer_writes.keys().collect::<BTreeSet<_>>() {
        return Err("coupling transfer read/write identities do not match".into());
    }
    for (id, read) in transfer_reads {
        let write = transfer_writes.get(id).ok_or("missing receiver transfer leg")?;
        let a = &read.value;
        let b = &write.value;
        if (a.raw_f64_bits.as_str(), a.source_interval_id.as_str(), read.interval.start_master_ns, read.interval.end_master_ns)
            != (b.raw_f64_bits.as_str(), b.source_interval_id.as_str(), write.interval.start_master_ns, write.interval.end_master_ns)
            || read.variable.port_role != "coupling_source" || write.variable.port_role != "coupling_sink"
            || read.variable.carrier != write.variable.carrier || read.variable.unit != write.variable.unit
            || read.variable.boundary_id != write.variable.boundary_id {
            return Err(format!("producer/consumer transfer mismatch for {id}"));
        }
    }

    let trace_bytes = fs::read(checked_artifact_path(&root, &trace_artifact.path)?)
        .map_err(|e| format!("cannot read normalized trace: {e}"))?;
    let trace: Trace = serde_json::from_slice(&trace_bytes)
        .map_err(|e| format!("normalized trace JSON invalid: {e}"))?;
    let _ = (&trace.fixture, &trace.claim);
    let trace_map: HashMap<&str, &TraceAttempt> = trace.attempts.iter().map(|t| (t.attempt_id.as_str(), t)).collect();
    if trace_map.len() != trace.attempts.len() || trace_map.len() != steps.len() {
        return Err("normalized trace and API doStep attempts are not one-to-one".into());
    }
    for (id, step) in &steps {
        let attempt = trace_map.get(id.as_str()).ok_or_else(|| format!("trace missing attempt {id}"))?;
        if attempt.federate_id != step.federate_id || attempt.start_master_ns != step.start_master_ns
            || attempt.end_master_ns != step.end_master_ns || attempt.status != step.status {
            return Err(format!("normalized trace mismatch for attempt {id}"));
        }
    }

    let mut used = BTreeSet::new();
    let mut barrier_ids = BTreeSet::new();
    for (index, barrier) in ledger.commit_barriers.iter().enumerate() {
        if barrier.sequence != index || !barrier_ids.insert(barrier.barrier_id.as_str()) {
            return Err("barrier sequence gap or duplicate barrier id".into());
        }
        if barrier.federate_ids.len() != barrier.attempt_ids.len() {
            return Err(format!("barrier participant count mismatch at {}", barrier.barrier_id));
        }
        let mut participants = BTreeSet::new();
        for id in &barrier.attempt_ids {
            if !used.insert(id.as_str()) { return Err(format!("attempt appears in multiple barriers: {id}")); }
            let step = steps.get(id).ok_or_else(|| format!("barrier references unknown attempt {id}"))?;
            if !participants.insert(step.federate_id.as_str()) || step.end_master_ns != barrier.master_time_ns {
                return Err(format!("barrier membership/time mismatch at {}", barrier.barrier_id));
            }
        }
        if participants != barrier.federate_ids.iter().map(String::as_str).collect::<BTreeSet<_>>() {
            return Err(format!("barrier federate identity mismatch at {}", barrier.barrier_id));
        }
    }
    if used.len() != steps.len() { return Err("one or more doStep attempts have no commit barrier".into()); }

    let energy_bytes = fs::read(checked_artifact_path(&root, &energy_artifact.path)?)
        .map_err(|e| format!("cannot read energy contract: {e}"))?;
    let energy: EnergyContract = serde_json::from_slice(&energy_bytes)
        .map_err(|e| format!("energy contract JSON invalid: {e}"))?;
    let _ = (&energy.fixture, &energy.unit, &energy.claim);
    for (port, expected) in &energy.expected {
        let got = observed_ports.get(port).ok_or_else(|| format!("energy contract port missing from API exchange: {port}"))?;
        if got.to_bits() != expected.to_bits() {
            return Err(format!("energy/API exchanged value mismatch at port {port}"));
        }
    }
    let service = *energy.expected.get("hx.thermal_service").ok_or("missing service output")?;
    let loss = *energy.expected.get("hx.heat_loss").ok_or("missing heat-loss output")?;
    let storage = *energy.expected.get("hx.storage_charge").ok_or("missing storage output")?;
    let input = *energy.expected.get("hx.thermal_in").ok_or("missing thermal input")?;
    let residual = input - service - loss - storage;
    if !residual.is_finite() || residual.to_bits() != energy.residual_w.to_bits() || residual != 0.0 {
        return Err("synthetic thermal first-law residual does not independently recompute to zero".into());
    }
    if energy.interval_id != value_records.first().map(|v| v.value.source_interval_id.as_str()).unwrap_or_default() {
        return Err("energy-contract interval does not match exchanged values".into());
    }

    let has_serialized_state_capability = ledger.federates.iter().any(|f|
        f.capabilities.can_get_and_set_fmu_state && f.capabilities.can_serialize_fmu_state
    );
    if ledger.state_checkpointing.status == "unavailable" {
        if !ledger.state_checkpointing.snapshot_artifact_ids.is_empty() || has_serialized_state_capability
            || ledger.state_checkpointing.reason_code != "SERIALIZATION_CAPABILITY_NOT_ADVERTISED" {
            return Err("state-checkpoint unavailable disposition conflicts with capabilities/artifacts".into());
        }
    } else if ledger.state_checkpointing.status == "available" {
        return Err("available checkpointing requires v0.2 state snapshot/restore semantics".into());
    } else if ledger.state_checkpointing.status != "not_used" {
        return Err("unknown state-checkpoint disposition".into());
    }

    if ledger.evidence_class == "instrumented_runtime"
        && ledger.master.collector_mode != "instrumented_c_api_boundary" {
        return Err("runtime evidence requires an instrumented C-API-boundary collector".into());
    }
    if ledger.evidence_class == "synthetic_fixture"
        && ledger.master.collector_mode != "fixture_authored_not_runtime_instrumented" {
        return Err("synthetic fixture must not claim runtime collection".into());
    }

    Ok(AuditReport {
        federates: ledger.federates.len(),
        events: ledger.events.len(),
        do_step_attempts: steps.len(),
        exchanged_values: value_records.len(),
        barriers: ledger.commit_barriers.len(),
        artifacts_verified: ledger.artifacts.len(),
        synthetic: ledger.evidence_class == "synthetic_fixture",
        state_checkpointing: if ledger.state_checkpointing.status == "unavailable" { "unavailable" } else { "not_used" },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    const FIXTURE: &str = include_str!("../../fixtures/positive-two-fmu-ledger.json");

    fn root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
    }

    fn value() -> Value {
        serde_json::from_str(FIXTURE).expect("fixture is valid JSON")
    }

    fn run(d: Value) -> Result<AuditReport, String> {
        let ledger: Ledger = serde_json::from_value(d).map_err(|e| e.to_string())?;
        audit(ledger, &root())
    }

    #[test]
    fn exact_binary64_clock_mapping_refuses_fractional_nanoseconds() {
        let clock = Clock {
            clock_id: "clock-test".to_owned(),
            master_clock_id: "master-test".to_owned(),
            model_time_unit: "s".to_owned(),
            mapping_digest: "mapping-test".to_owned(),
            nanoseconds_per_model_time_numerator: 1_000_000_000,
            nanoseconds_per_model_time_denominator: 1,
        };
        assert_eq!(to_ns(0.0, &clock, "zero"), Ok(0));
        assert_eq!(to_ns(0.25, &clock, "quarter-second"), Ok(250_000_000));
        assert_eq!(to_ns(2.0, &clock, "two-seconds"), Ok(2_000_000_000));
        assert!(to_ns(0.1, &clock, "binary64-tenth").is_err());
        assert!(to_ns(f64::MAX, &clock, "overflow").is_err());
    }

    #[test]
    fn accepts_the_frozen_positive_two_fmu_fixture() {
        let report = audit_json(FIXTURE, &root()).expect("positive fixture must pass");
        assert_eq!(report.federates, 2);
        assert_eq!(report.do_step_attempts, 2);
        assert_eq!(report.exchanged_values, 5);
        assert_eq!(report.barriers, 1);
        assert!(report.synthetic);
    }

    #[test]
    fn rejects_duplicate_event_identity() {
        let mut d=value(); let first=d["events"][0]["event_id"].clone(); d["events"][1]["event_id"]=first;
        assert!(run(d).unwrap_err().contains("duplicate event id"));
    }

    #[test]
    fn rejects_undeclared_value_reference() {
        let mut d=value(); d["events"][7]["payload"]["values"][0]["value_reference"]=999.into();
        assert!(run(d).unwrap_err().contains("undeclared value reference"));
    }

    #[test]
    fn rejects_exact_float64_bit_mismatch() {
        let mut d=value(); d["events"][7]["payload"]["values"][0]["raw_f64_bits"]="0000000000000000".into();
        assert!(run(d).unwrap_err().contains("Float64 bit-pattern mismatch"));
    }

    #[test]
    fn rejects_wrong_boundary_across_transfer() {
        let mut d=value(); d["federates"][1]["variables"][0]["boundary_id"]="wrong-boundary".into();
        assert!(run(d).unwrap_err().contains("producer/consumer transfer mismatch"));
    }

    #[test]
    fn rejects_mismatched_commit_barrier_time() {
        let mut d=value(); d["commit_barriers"][0]["master_time_ns"]=999.into();
        assert!(run(d).unwrap_err().contains("barrier membership/time mismatch"));
    }

    #[test]
    fn rejects_state_serialization_without_state_api_capability() {
        let mut d=value(); d["federates"][0]["capabilities"]["can_serialize_fmu_state"]=true.into();
        assert!(run(d).unwrap_err().contains("canSerializeFMUState without"));
    }

    #[test]
    fn rejects_trace_attempt_without_api_call() {
        let mut d=value(); d["events"][6]["payload"]["attempt_id"]="unlinked-attempt".into();
        assert!(run(d).unwrap_err().contains("normalized trace and API doStep attempts"));
    }

    #[test]
    fn rejects_root_traversal_artifact() {
        let mut d=value(); d["artifacts"][0]["path"]="../../etc/passwd".into();
        assert!(run(d).unwrap_err().contains("not root-relative"));
    }

    #[test]
    fn rejects_unknown_fields_through_strict_deserialization() {
        let mut d=value(); d["unexpected"]="must fail".into();
        let result:Result<Ledger,_>=serde_json::from_value(d);
        assert!(result.is_err());
    }

    #[test]
    fn rejects_lifecycle_gap() {
        let mut d=value(); d["events"][14]["kind"]="terminate".into();
        assert!(run(d).is_err());
    }
}
