//! Evidence-bound Linux cgroup v2 resource snapshots for external solver runs.
//!
//! This module parses already-captured cgroup files. A runtime adapter must create and own a
//! per-operation cgroup, place the complete solver process tree in it, and capture these files
//! before launch and after the cgroup becomes empty. Parent-process-only accounting is not
//! equivalent to cgroup process-tree evidence.
//!
//! CPU and memory are measured at cgroup scope. Host/device energy and per-process GPU time are
//! deliberately not inferred here.

use std::collections::BTreeMap;

use crate::integrity::sha256_hex;

pub const CGROUP_RESOURCE_SCHEMA: &str = "linux-cgroup-v2-resource-evidence/0.1.0";
pub const CGROUP_RESOURCE_EVIDENCE_LEVEL: &str =
    "raw_cgroup_v2_counter_snapshots_bound_to_process_tree_measure_cpu_and_peak_memory_not_gpu_or_energy";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CgroupV2Snapshot {
    pub cgroup_id: String,
    pub monotonic_time_ns: u64,
    pub cpu_usage_usec: u64,
    pub cpu_user_usec: u64,
    pub cpu_system_usec: u64,
    pub memory_peak_bytes: u64,
    /// Stable identity of the one open memory.peak FD used for both observations.
    pub memory_peak_reader_id: String,
    /// Exact cgroup2 mount-options bytes; local-only memory.events semantics are rejected.
    pub cgroup2_mount_options_sha256: String,
    pub oom_events: u64,
    pub oom_kill_events: u64,
    pub oom_group_kill_events: Option<u64>,
    pub populated: bool,
    pub frozen: bool,
    pub cpu_stat_sha256: String,
    pub memory_peak_sha256: String,
    pub memory_events_sha256: String,
    pub cgroup_events_sha256: String,
    pub snapshot_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CgroupV2ResourceReport {
    pub schema: String,
    pub cgroup_id: String,
    pub start_monotonic_time_ns: u64,
    pub end_monotonic_time_ns: u64,
    pub elapsed_wall_time_ns: u64,
    pub cpu_usage_usec: u64,
    pub cpu_user_usec: u64,
    pub cpu_system_usec: u64,
    /// Exact rational CPU core-hours = numerator / denominator, both in microseconds.
    pub cpu_core_hours_numerator_usec: u64,
    pub cpu_core_hours_denominator_usec: u64,
    pub peak_memory_bytes: u64,
    pub oom_event_delta: u64,
    pub oom_kill_event_delta: u64,
    pub oom_group_kill_event_delta: Option<u64>,
    pub start_snapshot_sha256: String,
    pub end_snapshot_sha256: String,
    pub evidence_level: String,
    pub report_sha256: String,
}

fn parse_key_value_file(input: &str, label: &str) -> Result<BTreeMap<String, u64>, String> {
    let mut result = BTreeMap::new();
    for (line_number, line) in input.lines().enumerate() {
        let mut parts = line.split_whitespace();
        let Some(key) = parts.next() else {
            continue;
        };
        let Some(raw_value) = parts.next() else {
            return Err(format!("{label} line {} is missing a value", line_number + 1));
        };
        if parts.next().is_some() {
            return Err(format!("{label} line {} contains trailing fields", line_number + 1));
        }
        if key.is_empty() || !key.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'_') {
            return Err(format!("{label} line {} has an invalid key", line_number + 1));
        }
        let value = raw_value
            .parse::<u64>()
            .map_err(|error| format!("{label} key {key} is not an unsigned integer: {error}"))?;
        if result.insert(key.to_owned(), value).is_some() {
            return Err(format!("{label} contains duplicate key {key}"));
        }
    }
    Ok(result)
}

fn required_counter(
    values: &BTreeMap<String, u64>,
    key: &str,
    label: &str,
) -> Result<u64, String> {
    values
        .get(key)
        .copied()
        .ok_or_else(|| format!("{label} is missing required counter {key}"))
}

fn bool_counter(values: &BTreeMap<String, u64>, key: &str) -> Result<bool, String> {
    match required_counter(values, key, "cgroup.events")? {
        0 => Ok(false),
        1 => Ok(true),
        value => Err(format!("cgroup.events {key} must be 0 or 1, observed {value}")),
    }
}

fn append_u64(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn append_string(bytes: &mut Vec<u8>, value: &str) {
    append_u64(bytes, value.len() as u64);
    bytes.extend_from_slice(value.as_bytes());
}

fn snapshot_canonical_bytes(snapshot: &CgroupV2Snapshot) -> Vec<u8> {
    let mut bytes = Vec::new();
    for value in [
        CGROUP_RESOURCE_SCHEMA,
        snapshot.cgroup_id.as_str(),
        snapshot.cpu_stat_sha256.as_str(),
        snapshot.memory_peak_sha256.as_str(),
        snapshot.memory_events_sha256.as_str(),
        snapshot.cgroup_events_sha256.as_str(),
        snapshot.memory_peak_reader_id.as_str(),
        snapshot.cgroup2_mount_options_sha256.as_str(),
    ] {
        append_string(&mut bytes, value);
    }
    for value in [
        snapshot.monotonic_time_ns,
        snapshot.cpu_usage_usec,
        snapshot.cpu_user_usec,
        snapshot.cpu_system_usec,
        snapshot.memory_peak_bytes,
        snapshot.oom_events,
        snapshot.oom_kill_events,
    ] {
        append_u64(&mut bytes, value);
    }
    match snapshot.oom_group_kill_events {
        Some(value) => {
            bytes.push(1);
            append_u64(&mut bytes, value);
        }
        None => bytes.push(0),
    }
    bytes.push(u8::from(snapshot.populated));
    bytes.push(u8::from(snapshot.frozen));
    bytes
}

fn report_canonical_bytes(report: &CgroupV2ResourceReport) -> Vec<u8> {
    let mut bytes = Vec::new();
    for value in [
        report.schema.as_str(),
        report.cgroup_id.as_str(),
        report.start_snapshot_sha256.as_str(),
        report.end_snapshot_sha256.as_str(),
        report.evidence_level.as_str(),
    ] {
        append_string(&mut bytes, value);
    }
    for value in [
        report.start_monotonic_time_ns,
        report.end_monotonic_time_ns,
        report.elapsed_wall_time_ns,
        report.cpu_usage_usec,
        report.cpu_user_usec,
        report.cpu_system_usec,
        report.peak_memory_bytes,
        report.oom_event_delta,
        report.oom_kill_event_delta,
    ] {
        append_u64(&mut bytes, value);
    }
    append_u64(bytes, report.cpu_core_hours_numerator_usec);
    append_u64(bytes, report.cpu_core_hours_denominator_usec);
    match report.oom_group_kill_event_delta {
        Some(value) => {
            bytes.push(1);
            append_u64(&mut bytes, value);
        }
        None => bytes.push(0),
    }
    bytes
}

/// Parse one coherent cgroup v2 snapshot from the raw contents of its measurement files.
///
/// cgroup_id must be a stable per-operation identity, not merely a reusable directory path.
/// monotonic_time_ns must come from a monotonic clock, not wall-clock time.
pub fn parse_cgroup_v2_snapshot(
    cgroup_id: &str,
    memory_peak_reader_id: &str,
    cgroup2_mount_options: &str,
    monotonic_time_ns: u64,
    cpu_stat: &str,
    memory_peak: &str,
    memory_events: &str,
    cgroup_events: &str,
) -> Result<CgroupV2Snapshot, String> {
    if cgroup_id.trim().is_empty() {
        return Err("cgroup identity must not be empty".to_owned());
    }
    if memory_peak_reader_id.trim().is_empty() {
        return Err("memory.peak reader identity must not be empty".to_owned());
    }
    if cgroup2_mount_options.trim().is_empty() {
        return Err("cgroup2 mount options must not be empty".to_owned());
    }
    if cgroup2_mount_options
        .split(',')
        .any(|option| option.trim() == "memory_localevents")
    {
        return Err(
            "memory_localevents mount option disables hierarchical memory.events evidence"
                .to_owned(),
        );
    }

    let cpu = parse_key_value_file(cpu_stat, "cpu.stat")?;
    let memory = memory_peak
        .trim()
        .parse::<u64>()
        .map_err(|error| format!("memory.peak is not an unsigned integer: {error}"))?;
    let events = parse_key_value_file(memory_events, "memory.events")?;
    let cgroup = parse_key_value_file(cgroup_events, "cgroup.events")?;

    let mut snapshot = CgroupV2Snapshot {
        cgroup_id: cgroup_id.to_owned(),
        monotonic_time_ns,
        cpu_usage_usec: required_counter(&cpu, "usage_usec", "cpu.stat")?,
        cpu_user_usec: required_counter(&cpu, "user_usec", "cpu.stat")?,
        cpu_system_usec: required_counter(&cpu, "system_usec", "cpu.stat")?,
        memory_peak_bytes: memory,
        memory_peak_reader_id: memory_peak_reader_id.to_owned(),
        cgroup2_mount_options_sha256: sha256_hex(cgroup2_mount_options.as_bytes())?,
        oom_events: required_counter(&events, "oom", "memory.events")?,
        oom_kill_events: required_counter(&events, "oom_kill", "memory.events")?,
        oom_group_kill_events: events.get("oom_group_kill").copied(),
        populated: bool_counter(&cgroup, "populated")?,
        frozen: bool_counter(&cgroup, "frozen")?,
        cpu_stat_sha256: sha256_hex(cpu_stat.as_bytes())?,
        memory_peak_sha256: sha256_hex(memory_peak.as_bytes())?,
        memory_events_sha256: sha256_hex(memory_events.as_bytes())?,
        cgroup_events_sha256: sha256_hex(cgroup_events.as_bytes())?,
        snapshot_sha256: String::new(),
    };
    snapshot.snapshot_sha256 = sha256_hex(&snapshot_canonical_bytes(&snapshot))?;
    Ok(snapshot)
}

fn validate_snapshot_integrity(snapshot: &CgroupV2Snapshot) -> Result<(), String> {
    if sha256_hex(&snapshot_canonical_bytes(snapshot))? != snapshot.snapshot_sha256 {
        return Err("cgroup snapshot canonical digest mismatch".to_owned());
    }
    for digest in [
        snapshot.cpu_stat_sha256.as_str(),
        snapshot.memory_peak_sha256.as_str(),
        snapshot.memory_events_sha256.as_str(),
        snapshot.cgroup_events_sha256.as_str(),
    ] {
        if digest.len() != 64
            || digest.bytes().any(|byte| !byte.is_ascii_hexdigit())
            || digest != digest.to_ascii_lowercase()
        {
            return Err("cgroup raw-file digest is malformed".to_owned());
        }
    }
    Ok(())
}

fn monotonic_delta(start: u64, end: u64, name: &str) -> Result<u64, String> {
    end.checked_sub(start)
        .ok_or_else(|| format!("cgroup {name} counter regressed or reset"))
}

/// Derive operation-scoped cgroup CPU/memory evidence from two snapshots.
///
/// The starting cgroup must be empty before the solver is launched; the final cgroup must be
/// empty after all descendants exit. OOM counters are retained as observations, not treated as
/// proof that the solver process itself returned a particular exit code.
pub fn derive_cgroup_v2_resource_report(
    start: &CgroupV2Snapshot,
    end: &CgroupV2Snapshot,
) -> Result<CgroupV2ResourceReport, String> {
    validate_snapshot_integrity(start)?;
    validate_snapshot_integrity(end)?;
    if start.cgroup_id != end.cgroup_id {
        return Err("cgroup resource snapshots have different cgroup identities".to_owned());
    }
    if start.memory_peak_reader_id != end.memory_peak_reader_id {
        return Err("memory.peak samples were read through different file descriptors".to_owned());
    }
    if start.cgroup2_mount_options_sha256 != end.cgroup2_mount_options_sha256 {
        return Err("cgroup2 mount options changed between resource snapshots".to_owned());
    }
    if start.populated || end.populated {
        return Err("cgroup must be empty at both baseline and finalization".to_owned());
    }
    if start.frozen || end.frozen {
        return Err("cgroup is frozen at a resource snapshot boundary".to_owned());
    }
    if start.oom_group_kill_events.is_some() != end.oom_group_kill_events.is_some() {
        return Err("memory.events oom_group_kill capability changed between snapshots".to_owned());
    }

    let elapsed_wall_time_ns = end
        .monotonic_time_ns
        .checked_sub(start.monotonic_time_ns)
        .filter(|value| *value > 0)
        .ok_or_else(|| "cgroup monotonic sample interval is zero or regressed".to_owned())?;
    let cpu_usage_usec = monotonic_delta(start.cpu_usage_usec, end.cpu_usage_usec, "usage_usec")?;
    let cpu_user_usec = monotonic_delta(start.cpu_user_usec, end.cpu_user_usec, "user_usec")?;
    let cpu_system_usec = monotonic_delta(start.cpu_system_usec, end.cpu_system_usec, "system_usec")?;
    if end.memory_peak_bytes < start.memory_peak_bytes {
        return Err("memory.peak regressed or was reset during the operation".to_owned());
    }
    let oom_event_delta = monotonic_delta(start.oom_events, end.oom_events, "oom")?;
    let oom_kill_event_delta =
        monotonic_delta(start.oom_kill_events, end.oom_kill_events, "oom_kill")?;
    let oom_group_kill_event_delta = match (
        start.oom_group_kill_events,
        end.oom_group_kill_events,
    ) {
        (Some(start), Some(end)) => Some(monotonic_delta(start, end, "oom_group_kill")?),
        (None, None) => None,
        _ => unreachable!("capability change was rejected above"),
    };

    let mut report = CgroupV2ResourceReport {
        schema: CGROUP_RESOURCE_SCHEMA.to_owned(),
        cgroup_id: start.cgroup_id.clone(),
        start_monotonic_time_ns: start.monotonic_time_ns,
        end_monotonic_time_ns: end.monotonic_time_ns,
        elapsed_wall_time_ns,
        cpu_usage_usec,
        cpu_user_usec,
        cpu_system_usec,
        // One core-hour is exactly 3,600,000,000 CPU microseconds.
        cpu_core_hours_numerator_usec: cpu_usage_usec,
        cpu_core_hours_denominator_usec: 3_600_000_000,
        peak_memory_bytes: end.memory_peak_bytes,
        oom_event_delta,
        oom_kill_event_delta,
        oom_group_kill_event_delta,
        start_snapshot_sha256: start.snapshot_sha256.clone(),
        end_snapshot_sha256: end.snapshot_sha256.clone(),
        evidence_level: CGROUP_RESOURCE_EVIDENCE_LEVEL.to_owned(),
        report_sha256: String::new(),
    };
    report.report_sha256 = sha256_hex(&report_canonical_bytes(&report))?;
    Ok(report)
}

pub fn verify_cgroup_v2_resource_report(
    start: &CgroupV2Snapshot,
    end: &CgroupV2Snapshot,
    report: &CgroupV2ResourceReport,
) -> Result<(), String> {
    if report.schema != CGROUP_RESOURCE_SCHEMA
        || report.evidence_level != CGROUP_RESOURCE_EVIDENCE_LEVEL
    {
        return Err("unsupported cgroup resource schema or evidence ceiling".to_owned());
    }
    validate_snapshot_integrity(start)?;
    validate_snapshot_integrity(end)?;
    let replayed = derive_cgroup_v2_resource_report(start, end)?;
    if replayed != *report {
        return Err("replayed cgroup resource report differs from supplied report".to_owned());
    }
    if sha256_hex(&report_canonical_bytes(report))? != report.report_sha256 {
        return Err("cgroup resource report canonical digest mismatch".to_owned());
    }
    Ok(())
}

fn csv_escape(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

fn csv_row(values: Vec<String>) -> String {
    values
        .iter()
        .map(|value| csv_escape(value))
        .collect::<Vec<_>>()
        .join(",")
}

/// Export both raw-snapshot identities and the derived operation report for an independent oracle.
pub fn cgroup_v2_resource_evidence_to_csv(
    start: &CgroupV2Snapshot,
    end: &CgroupV2Snapshot,
    report: &CgroupV2ResourceReport,
) -> String {
    let header = "record_type,cgroup_id,memory_peak_reader_id,cgroup2_mount_options_sha256,monotonic_time_ns,cpu_usage_usec,cpu_user_usec,cpu_system_usec,memory_peak_bytes,oom_events,oom_kill_events,oom_group_kill_events,populated,frozen,cpu_stat_sha256,memory_peak_sha256,memory_events_sha256,cgroup_events_sha256,snapshot_sha256,schema,start_monotonic_time_ns,end_monotonic_time_ns,elapsed_wall_time_ns,cpu_usage_delta_usec,cpu_user_delta_usec,cpu_system_delta_usec,cpu_core_hours_numerator_usec,cpu_core_hours_denominator_usec,peak_memory_bytes,oom_event_delta,oom_kill_event_delta,oom_group_kill_event_delta,start_snapshot_sha256,end_snapshot_sha256,evidence_level,report_sha256";
    let mut output = format!("{header}\n");
    for (kind, snapshot) in [("start", start), ("end", end)] {
        let mut row = vec![String::new(); 35];
        row[0] = kind.to_owned();
        row[1] = snapshot.cgroup_id.clone();
        row[2] = snapshot.memory_peak_reader_id.clone();
        row[3] = snapshot.cgroup2_mount_options_sha256.clone();
        row[4] = snapshot.monotonic_time_ns.to_string();
        row[5] = snapshot.cpu_usage_usec.to_string();
        row[6] = snapshot.cpu_user_usec.to_string();
        row[7] = snapshot.cpu_system_usec.to_string();
        row[8] = snapshot.memory_peak_bytes.to_string();
        row[9] = snapshot.oom_events.to_string();
        row[10] = snapshot.oom_kill_events.to_string();
        row[11] = snapshot.oom_group_kill_events.map(|value| value.to_string()).unwrap_or_default();
        row[12] = snapshot.populated.to_string();
        row[13] = snapshot.frozen.to_string();
        row[14] = snapshot.cpu_stat_sha256.clone();
        row[15] = snapshot.memory_peak_sha256.clone();
        row[16] = snapshot.memory_events_sha256.clone();
        row[17] = snapshot.cgroup_events_sha256.clone();
        row[18] = snapshot.snapshot_sha256.clone();
        output.push_str(&csv_row(row));
        output.push('\n');
    }
    let mut row = vec![String::new(); 35];
    row[0] = "report".to_owned();
    row[1] = report.cgroup_id.clone();
    row[19] = report.schema.clone();
    row[20] = report.start_monotonic_time_ns.to_string();
    row[21] = report.end_monotonic_time_ns.to_string();
    row[22] = report.elapsed_wall_time_ns.to_string();
    row[23] = report.cpu_usage_usec.to_string();
    row[24] = report.cpu_user_usec.to_string();
    row[25] = report.cpu_system_usec.to_string();
    row[26] = report.cpu_core_hours_numerator_usec.to_string();
    row[27] = report.cpu_core_hours_denominator_usec.to_string();
    row[28] = report.peak_memory_bytes.to_string();
    row[29] = report.oom_event_delta.to_string();
    row[30] = report.oom_kill_event_delta.to_string();
    row[31] = report.oom_group_kill_event_delta.map(|value| value.to_string()).unwrap_or_default();
    row[32] = report.start_snapshot_sha256.clone();
    row[33] = report.end_snapshot_sha256.clone();
    row[34] = report.evidence_level.clone();
    row[35-1] = report.report_sha256.clone();
    output.push_str(&csv_row(row));
    output.push('\n');
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot(time_ns: u64, cpu: u64, user: u64, system: u64, peak: u64, oom: u64, kill: u64) -> CgroupV2Snapshot {
        parse_cgroup_v2_snapshot(
            "solver-operation-42",
            "memory-peak-reader-42",
            "rw,nosuid,nodev,relatime",
            time_ns,
            &format!("usage_usec {cpu}\nuser_usec {user}\nsystem_usec {system}\nnr_periods 10\n"),
            &format!("{peak}\n"),
            &format!("low 0\nhigh 0\nmax 0\noom {oom}\noom_kill {kill}\noom_group_kill 0\n"),
            "populated 0\nfrozen 0\n",
        ).expect("parse cgroup snapshot fixture")
    }

    #[test]
    fn derives_cpu_wall_peak_memory_and_oom_deltas() {
        let start = snapshot(1_000_000_000, 1_000_000, 800_000, 200_000, 2_000_000, 0, 0);
        let end = snapshot(11_000_000_000, 6_000_000, 4_800_000, 1_200_000, 48_000_000, 1, 1);
        let report = derive_cgroup_v2_resource_report(&start, &end).expect("derive resource report");
        assert_eq!(report.elapsed_wall_time_ns, 10_000_000_000);
        assert_eq!(report.cpu_usage_usec, 5_000_000);
        assert_eq!(report.cpu_user_usec, 4_000_000);
        assert_eq!(report.cpu_system_usec, 1_000_000);
        assert_eq!(report.peak_memory_bytes, 48_000_000);
        assert_eq!(report.oom_event_delta, 1);
        assert_eq!(report.oom_kill_event_delta, 1);
        verify_cgroup_v2_resource_report(&start, &end, &report).unwrap();
    }

    #[test]
    fn rejects_active_cgroup_at_finalization_and_nonempty_start() {
        let start = snapshot(1_000, 0, 0, 0, 0, 0, 0);
        let mut active = snapshot(2_000, 1, 1, 0, 1, 0, 0);
        active.populated = true;
        assert!(derive_cgroup_v2_resource_report(&start, &active)
            .expect_err("active cgroup must reject")
            .contains("empty"));
        let mut busy_start = snapshot(1_000, 0, 0, 0, 0, 0, 0);
        busy_start.populated = true;
        assert!(derive_cgroup_v2_resource_report(&busy_start, &start).is_err());
    }

    #[test]
    fn rejects_counter_regression_identity_drift_and_memory_peak_reset() {
        let start = snapshot(1_000, 100, 80, 20, 1000, 0, 0);
        let cpu_regressed = snapshot(2_000, 99, 79, 20, 1100, 0, 0);
        assert!(derive_cgroup_v2_resource_report(&start, &cpu_regressed)
            .expect_err("CPU counter reset must reject")
            .contains("regressed"));

        let mut wrong_id = snapshot(2_000, 200, 160, 40, 1200, 0, 0);
        wrong_id.cgroup_id = "other-operation".to_owned();
        assert!(derive_cgroup_v2_resource_report(&start, &wrong_id)
            .expect_err("cross-cgroup report must reject")
            .contains("different cgroup"));

        let memory_reset = snapshot(2_000, 200, 160, 40, 900, 0, 0);
        assert!(derive_cgroup_v2_resource_report(&start, &memory_reset)
            .expect_err("memory peak reset must reject")
            .contains("memory.peak"));
    }

    #[test]
    fn rejects_malformed_and_duplicate_kernel_counter_fields() {
        assert!(parse_cgroup_v2_snapshot(
            "x",
            1,
            "usage_usec 1\nuser_usec 1\n",
            "10\n",
            "oom 0\noom_kill 0\n",
            "populated 0\nfrozen 0\n",
        ).expect_err("missing system_usec").contains("system_usec"));
        assert!(parse_cgroup_v2_snapshot(
            "x",
            1,
            "usage_usec 1\nusage_usec 2\nuser_usec 1\nsystem_usec 0\n",
            "10\n",
            "oom 0\noom_kill 0\n",
            "populated 0\nfrozen 0\n",
        ).expect_err("duplicate CPU stat").contains("duplicate"));
    }

    #[test]
    fn independent_python_oracle_replays_cgroup_resource_evidence() {
        let start = snapshot(1_000_000_000, 1_000_000, 800_000, 200_000, 2_000_000, 0, 0);
        let end = snapshot(11_000_000_000, 6_000_000, 4_800_000, 1_200_000, 48_000_000, 1, 1);
        let report = derive_cgroup_v2_resource_report(&start, &end).unwrap();
        let root = std::env::temp_dir().join(format!(
            "cgroup-v2-resource-oracle-{}-{}",
            std::process::id(),
            start.monotonic_time_ns
        ));
        std::fs::create_dir_all(&root).unwrap();
        let csv = root.join("resource.csv");
        std::fs::write(&csv, cgroup_v2_resource_evidence_to_csv(&start, &end, &report)).unwrap();
        let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("scripts/verify-cgroup-v2-resource-independent.py");
        let output = std::process::Command::new("python3")
            .arg(script)
            .arg(&csv)
            .output()
            .expect("run independent cgroup resource oracle");
        assert!(
            output.status.success(),
            "independent cgroup resource oracle failed: {}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}
